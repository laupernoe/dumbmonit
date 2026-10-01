//! CORS de l'API ouverte : un tableau de bord ou un outil servi depuis une autre
//! origine peut appeler `/api/*` avec un jeton, jamais avec la session.
//!
//! La règle tient en trois points :
//!
//! - une réponse ne porte `Access-Control-Allow-Origin` que si la requête
//!   présentait `Authorization: Bearer dmt_…` (ou pour la spécification
//!   publique `/api/openapi.json`). Une requête portée par le cookie de session
//!   n'est jamais lisible depuis une autre origine ;
//! - `Access-Control-Allow-Credentials` n'est jamais envoyé : un navigateur
//!   refuse alors toute requête inter-origines qui voudrait joindre le cookie ;
//! - le préflight n'est accepté que s'il annonce l'en-tête `Authorization`, et
//!   `X-Requested-With` — la preuve d'origine qu'exigent les écritures par
//!   cookie — ne figure pas dans les en-têtes autorisés. Une page tierce ne peut
//!   donc ni lire ni écrire au nom d'une session, même en contournant le reste.
//!
//! Les origines admises viennent de `DUMBMONIT_API_CORS_ORIGINS` : absente,
//! toute origine (le jeton est la seule autorisation qui compte) ; `off`, aucune ;
//! sinon une liste d'origines séparées par des virgules. La même politique sert
//! au point d'entrée MCP pour valider l'en-tête `Origin` (protection contre le
//! « DNS rebinding » qu'exige le protocole).

use std::sync::Arc;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::auth::token;

/// Variable d'environnement des origines admises.
pub const ENV: &str = "DUMBMONIT_API_CORS_ORIGINS";

/// En-têtes qu'une page d'une autre origine peut envoyer. `X-Requested-With`
/// en est absent à dessein (voir la documentation du module).
pub const ALLOWED_HEADERS: &str = "authorization, content-type, accept, mcp-protocol-version, \
     mcp-method, mcp-name, mcp-session-id";

/// En-têtes de réponse lisibles par la page appelante.
pub const EXPOSED_HEADERS: &str = "retry-after, www-authenticate, ratelimit-limit, \
     ratelimit-remaining, ratelimit-reset, x-dumbmonit-api-version";

const ALLOWED_METHODS: &str = "GET, POST, PUT, DELETE";

/// Durée de mise en cache d'un préflight par le navigateur.
const MAX_AGE_SECS: u32 = 600;

/// Chemin de la spécification, lisible de partout sans jeton.
const PUBLIC_SPEC: &str = "/api/openapi.json";

/// Origines admises pour les requêtes inter-origines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorsPolicy {
    /// Toute origine (`*`).
    Any,
    /// Aucune : l'API ne répond qu'à sa propre origine et aux clients hors
    /// navigateur.
    Off,
    /// Ces origines seulement, normalisées (`https://grafana.example.com`).
    List(Vec<String>),
}

impl CorsPolicy {
    pub fn from_env() -> Self {
        Self::parse(crate::config::env_var(ENV).as_deref())
    }

    pub fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else { return Self::Any };
        match raw.to_ascii_lowercase().as_str() {
            "*" | "any" => return Self::Any,
            "off" | "none" | "false" | "0" => return Self::Off,
            _ => {}
        }
        let origins: Vec<String> =
            raw.split(',').map(normalise_origin).filter(|o| !o.is_empty()).collect();
        if origins.is_empty() { Self::Off } else { Self::List(origins) }
    }

    /// Valeur d'`Access-Control-Allow-Origin` pour cette origine, ou `None` si
    /// elle n'est pas admise.
    pub fn allow_origin(&self, origin: &str) -> Option<HeaderValue> {
        match self {
            Self::Any => Some(HeaderValue::from_static("*")),
            Self::Off => None,
            Self::List(origins) => {
                if origins.contains(&normalise_origin(origin)) {
                    HeaderValue::from_str(origin).ok()
                } else {
                    None
                }
            }
        }
    }

    /// L'origine d'une requête est-elle acceptable ? Oui si la politique
    /// l'admet, ou si c'est la nôtre (même hôte que l'en-tête `Host`).
    pub fn accepts(&self, origin: &str, headers: &HeaderMap) -> bool {
        same_host(origin, headers) || self.allow_origin(origin).is_some()
    }
}

/// `HTTPS://Example.com:443/` → `https://example.com:443` : casse et barre
/// finale ne doivent pas faire refuser une origine écrite à la main.
fn normalise_origin(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_ascii_lowercase()
}

/// Vrai si l'origine désigne l'hôte même auquel la requête est adressée.
pub fn same_host(origin: &str, headers: &HeaderMap) -> bool {
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).map(str::trim);
    let origin_host = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    let origin_host = origin_host.split('/').next().unwrap_or(origin_host);
    host.is_some_and(|host| host.eq_ignore_ascii_case(origin_host))
}

fn is_api(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/")
}

/// Vrai si la liste `Access-Control-Request-Headers` annonce `name`.
fn requests_header(headers: &HeaderMap, name: &str) -> bool {
    headers
        .get_all(header::ACCESS_CONTROL_REQUEST_HEADERS)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|h| h.trim().eq_ignore_ascii_case(name))
}

/// Middleware CORS, posé sur tout le routeur ; il ne s'occupe que de `/api/*`.
pub async fn layer(
    State(policy): State<Arc<CorsPolicy>>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if !is_api(&path) {
        return next.run(request).await;
    }
    // Le point d'entrée MCP valide `Origin` avec la même politique.
    request.extensions_mut().insert(policy.clone());

    let origin =
        request.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok()).map(str::to_string);

    let is_preflight = request.method() == Method::OPTIONS
        && origin.is_some()
        && request.headers().contains_key(header::ACCESS_CONTROL_REQUEST_METHOD);
    if is_preflight {
        let origin = origin.as_deref().unwrap_or_default();
        return preflight(&policy, origin, request.headers(), &path);
    }

    let bearer = token::extract_bearer(request.headers()).is_some();
    let mut response = next.run(request).await;
    if let Some(origin) = origin
        && (bearer || path == PUBLIC_SPEC)
        && let Some(allowed) = policy.allow_origin(&origin)
    {
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, allowed);
        headers.insert(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            HeaderValue::from_static(EXPOSED_HEADERS),
        );
        if matches!(*policy, CorsPolicy::List(_)) {
            headers.append(header::VARY, HeaderValue::from_static("origin"));
        }
    }
    response
}

fn preflight(policy: &CorsPolicy, origin: &str, headers: &HeaderMap, path: &str) -> Response {
    let Some(allowed) = policy.allow_origin(origin) else {
        return refuse(
            "Cross-origin requests are not allowed from this origin (DUMBMONIT_API_CORS_ORIGINS).",
        );
    };
    if !requests_header(headers, "authorization") && path != PUBLIC_SPEC {
        return refuse(
            "Cross-origin requests must carry an API token (Authorization: Bearer dmt_…); the \
             session cookie only works from the DumbMonit interface itself.",
        );
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    let out = response.headers_mut();
    out.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, allowed);
    out.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static(ALLOWED_METHODS));
    out.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static(ALLOWED_HEADERS));
    out.insert(header::ACCESS_CONTROL_MAX_AGE, HeaderValue::from(MAX_AGE_SECS));
    if matches!(policy, CorsPolicy::List(_)) {
        out.append(header::VARY, HeaderValue::from_static("origin"));
    }
    response
}

/// Préflight refusé : pas d'en-tête CORS, donc le navigateur bloque ; le corps
/// explique pourquoi à qui ouvre l'onglet réseau.
fn refuse(message: &str) -> Response {
    (StatusCode::FORBIDDEN, Json(json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_policy_is_read_from_a_short_list() {
        assert_eq!(CorsPolicy::parse(None), CorsPolicy::Any);
        assert_eq!(CorsPolicy::parse(Some("  ")), CorsPolicy::Any);
        assert_eq!(CorsPolicy::parse(Some("*")), CorsPolicy::Any);
        assert_eq!(CorsPolicy::parse(Some("off")), CorsPolicy::Off);
        assert_eq!(
            CorsPolicy::parse(Some("https://Grafana.example.com/, http://dash.lan:3000")),
            CorsPolicy::List(vec![
                "https://grafana.example.com".into(),
                "http://dash.lan:3000".into()
            ])
        );
        assert_eq!(CorsPolicy::parse(Some(" , ")), CorsPolicy::Off);
    }

    #[test]
    fn only_listed_origins_are_echoed() {
        let any = CorsPolicy::Any;
        assert_eq!(any.allow_origin("https://evil.example").unwrap(), "*");
        assert!(CorsPolicy::Off.allow_origin("https://dash.lan").is_none());

        let list = CorsPolicy::parse(Some("https://dash.lan"));
        assert_eq!(list.allow_origin("https://dash.lan").unwrap(), "https://dash.lan");
        assert_eq!(list.allow_origin("https://DASH.lan/").unwrap(), "https://DASH.lan/");
        assert!(list.allow_origin("https://dash.lan.evil.example").is_none());
        assert!(list.allow_origin("http://dash.lan").is_none(), "le schéma compte");
    }

    #[test]
    fn our_own_origin_is_always_accepted() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "monit.lan:8080".parse().unwrap());
        assert!(CorsPolicy::Off.accepts("http://monit.lan:8080", &headers));
        assert!(!CorsPolicy::Off.accepts("http://evil.example", &headers));
        assert!(CorsPolicy::Any.accepts("http://evil.example", &headers));
    }

    #[test]
    fn x_requested_with_is_never_allowed_cross_origin() {
        assert!(!ALLOWED_HEADERS.contains("x-requested-with"));
        assert!(ALLOWED_HEADERS.contains("authorization"));
    }
}
