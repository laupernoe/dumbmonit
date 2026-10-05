//! Routes de la connexion OpenID Connect.
//!
//! `start` et `callback` sont des navigations, pas des appels d'API : le
//! navigateur est envoyé chez le fournisseur, puis revient ici avec un code. Les
//! erreurs se terminent donc par une redirection vers l'écran de connexion, avec
//! une raison courte dans l'URL, et jamais par un JSON que personne ne lirait.

use std::net::SocketAddr;

use axum::Json;
use axum::extract::{ConnectInfo, Extension, FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::auth::middleware::AdminUser;
use crate::auth::oidc::flow::{self, CallbackParams, FlowError};
use crate::auth::oidc::{self, OidcConfig, Source, discovery};
use crate::auth::{AuthResult, AuthState, cookie, session};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct StartParams {
    /// Page interne à rouvrir après la connexion.
    redirect: Option<String>,
}

/// Cookie qui lie une tentative de connexion au navigateur qui l'a lancée.
///
/// Sans lui, le `state` ne prouve rien sur *qui* revient : un attaquant
/// lancerait une connexion avec son propre compte chez le fournisseur, puis
/// enverrait l'URL de retour à sa victime, qui se retrouverait connectée sous
/// l'identité de l'attaquant. Le cookie porte l'empreinte du `state`, pas le
/// `state` lui-même ; il ne vit que le temps d'une connexion et n'est envoyé
/// qu'aux routes OIDC.
const BINDING_COOKIE: &str = "dumbmonit_oidc";
const BINDING_MAX_AGE_SECS: u64 = 600;

fn state_fingerprint(state: &str) -> String {
    hex::encode(Sha256::digest(state.as_bytes()))
}

fn binding_cookie(state: &str, secure: bool) -> HeaderValue {
    let mut value = format!(
        "{BINDING_COOKIE}={}; Path=/api/auth/oidc; HttpOnly; SameSite=Lax; Max-Age={BINDING_MAX_AGE_SECS}",
        state_fingerprint(state)
    );
    if secure {
        value.push_str("; Secure");
    }
    header_value(&value)
}

fn clear_binding_cookie(secure: bool) -> HeaderValue {
    let mut value =
        format!("{BINDING_COOKIE}=; Path=/api/auth/oidc; HttpOnly; SameSite=Lax; Max-Age=0");
    if secure {
        value.push_str("; Secure");
    }
    header_value(&value)
}

/// Le navigateur qui revient est-il celui qui est parti avec ce `state` ?
fn binding_matches(headers: &HeaderMap, state: Option<&str>) -> bool {
    let Some(state) = state else { return false };
    let expected = state_fingerprint(state);
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .filter(|(name, _)| *name == BINDING_COOKIE)
        .any(|(_, value)| bool::from(value.as_bytes().ct_eq(expected.as_bytes())))
}

/// `GET /api/auth/oidc/start` — envoie le navigateur chez le fournisseur.
pub async fn start(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    origin: Origin,
    Query(params): Query<StartParams>,
) -> Response {
    let resolved = match oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await {
        Ok(resolved) => resolved,
        Err(error) => return failure(FlowError::Internal(error)),
    };
    let redirect = params.redirect.filter(|path| is_internal_path(path));
    match flow::start(&auth, &resolved.config, &origin.0, redirect).await {
        Ok((url, login_state)) => (
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, binding_cookie(&login_state, auth.cookie_secure())),
                (header::LOCATION, header_value(&url)),
            ],
        )
            .into_response(),
        Err(error) => failure(error),
    }
}

/// `GET /api/auth/oidc/callback` — retour du fournisseur avec un code.
pub async fn callback(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    headers: HeaderMap,
    Query(params): Query<CallbackParams>,
) -> Response {
    if !binding_matches(&headers, params.state.as_deref()) {
        return failure(FlowError::State);
    }
    let resolved = match oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await {
        Ok(resolved) => resolved,
        Err(error) => return failure(FlowError::Internal(error)),
    };
    let (user, redirect) = match flow::finish(&state.pool, &auth, &resolved.config, params).await {
        Ok(found) => found,
        Err(error) => return failure(error),
    };
    let token = match session::create(&state.pool, user.id).await {
        Ok(token) => token,
        Err(error) => return failure(FlowError::Internal(error)),
    };
    let destination = redirect.unwrap_or_else(|| "/".to_string());
    // `AppendHeaders` : deux `Set-Cookie` dans un même tableau se remplaceraient.
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, header_value(&destination))],
        axum::response::AppendHeaders([
            (header::SET_COOKIE, cookie::set(&token, auth.cookie_secure())),
            (header::SET_COOKIE, clear_binding_cookie(auth.cookie_secure())),
        ]),
    )
        .into_response()
}

/// Renvoie à l'écran de connexion avec la raison de l'échec. Le détail, lui, va
/// dans le journal : c'est là que l'administrateur le cherchera.
fn failure(error: FlowError) -> Response {
    match &error {
        FlowError::Denied(why) => {
            tracing::warn!(reason = %why, "OIDC login refused by the provider")
        }
        FlowError::NotConfigured
        | FlowError::State
        | FlowError::NoAccount
        | FlowError::Disabled => {
            tracing::warn!(reason = error.reason(), "OIDC login failed")
        }
        FlowError::ProviderUnreachable(detail)
        | FlowError::Exchange(detail)
        | FlowError::Internal(detail) => {
            tracing::error!(reason = error.reason(), detail = %format!("{detail:#}"), "OIDC login failed")
        }
        FlowError::InvalidToken(detail) => {
            tracing::warn!(reason = error.reason(), detail = %detail, "OIDC login failed")
        }
    }
    Redirect::to(&format!("/login?error=oidc&reason={}", error.reason())).into_response()
}

fn header_value(value: &str) -> header::HeaderValue {
    header::HeaderValue::from_str(value).unwrap_or_else(|_| header::HeaderValue::from_static("/"))
}

/// Un chemin interne, et rien d'autre : ni URL absolue, ni `//` ou `/\` que les
/// navigateurs lisent comme une adresse externe, ni caractère de contrôle.
///
/// La règle de forme est doublée d'une résolution contre une origine factice :
/// si le résultat en sort, ce n'était pas un chemin interne, quelle que soit
/// l'astuce d'écriture.
fn is_internal_path(path: &str) -> bool {
    let mut chars = path.chars();
    if chars.next() != Some('/') || matches!(chars.next(), Some('/' | '\\')) {
        return false;
    }
    if path.chars().any(|c| c.is_control() || c == '\\') {
        return false;
    }
    reqwest::Url::parse("http://internal.invalid/")
        .and_then(|base| base.join(path))
        .is_ok_and(|url| url.host_str() == Some("internal.invalid"))
}

/// Origine de la requête, établie comme le fait [`request_origin`].
pub struct Origin(String);

impl<S: Send + Sync> FromRequestParts<S> for Origin {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let trusted = parts
            .extensions
            .get::<AuthState>()
            .map(|auth| auth.trusted_proxies().to_vec())
            .unwrap_or_default();
        let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|info| info.0.ip());
        let behind_proxy = peer.is_some_and(|peer| trusted.iter().any(|net| net.contains(&peer)));
        Ok(Self(request_origin(&parts.headers, behind_proxy)))
    }
}

/// Origine par laquelle le navigateur nous joint, pour construire l'URL de
/// retour quand aucune URL publique n'est configurée.
///
/// Les en-têtes `X-Forwarded-*` ne sont lus que si la connexion vient d'un
/// mandataire déclaré de confiance (`DUMBMONIT_TRUSTED_PROXIES`), comme pour
/// l'adresse du client : sinon n'importe quel client choisirait l'adresse vers
/// laquelle le fournisseur renverra le code.
fn request_origin(headers: &HeaderMap, behind_trusted_proxy: bool) -> String {
    let text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok()).map(str::trim);
    let forwarded = |name: &str| {
        behind_trusted_proxy
            .then(|| text(name))
            .flatten()
            .and_then(|value| value.split(',').next())
            .map(str::trim)
    };
    let scheme = forwarded("x-forwarded-proto")
        .filter(|scheme| *scheme == "https" || *scheme == "http")
        .unwrap_or("http");
    let host = forwarded("x-forwarded-host")
        .filter(|host| !host.is_empty())
        .or_else(|| text("host"))
        .unwrap_or("localhost:8080");
    format!("{scheme}://{host}")
}

// --- Réglages, réservés aux administrateurs ---------------------------------

/// La configuration telle que l'écran des réglages la montre : sans le secret,
/// avec sa provenance et l'URL de retour calculée.
#[derive(Debug, Serialize)]
pub struct ConfigView {
    source: Source,
    enabled: bool,
    /// L'environnement décrit-il un fournisseur ? Permet de proposer d'y revenir.
    env_available: bool,
    issuer: String,
    client_id: String,
    has_client_secret: bool,
    provider_name: String,
    scopes: String,
    auto_create: bool,
    /// `auto_create` n'a jamais été choisi : il vaut « non » par défaut, et
    /// l'interface l'explique (les versions antérieures l'activaient).
    auto_create_defaulted: bool,
    admin_groups: Vec<String>,
    groups_claim: String,
    public_url: String,
    redirect_uri: String,
}

fn view(state: &AppState, resolved: oidc::Resolved, origin: &str) -> ConfigView {
    let config = resolved.config;
    ConfigView {
        source: resolved.source,
        enabled: config.enabled(),
        env_available: state.config.oidc.is_set(),
        redirect_uri: config.redirect_uri(origin),
        has_client_secret: !config.client_secret.is_empty(),
        issuer: config.issuer,
        client_id: config.client_id,
        provider_name: config.provider_name,
        scopes: config.scopes,
        auto_create: config.auto_create,
        auto_create_defaulted: resolved.auto_create_defaulted,
        admin_groups: config.admin_groups,
        groups_claim: config.groups_claim,
        public_url: config.public_url,
    }
}

/// `GET /api/auth/oidc/config`
pub async fn get_config(
    State(state): State<AppState>,
    _: AdminUser,
    origin: Origin,
) -> AuthResult<Json<ConfigView>> {
    let resolved = oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await?;
    Ok(Json(view(&state, resolved, &origin.0)))
}

/// Corps de `PUT`. Un secret absent ou vide conserve celui déjà enregistré.
#[derive(Deserialize)]
pub struct ConfigPayload {
    issuer: String,
    client_id: String,
    #[serde(default)]
    client_secret: Option<String>,
    #[serde(default)]
    provider_name: String,
    #[serde(default)]
    scopes: String,
    /// Absent : désactivé. Créer des comptes à la volée doit être un choix.
    #[serde(default)]
    auto_create: bool,
    #[serde(default)]
    admin_groups: Vec<String>,
    #[serde(default)]
    groups_claim: String,
    #[serde(default)]
    public_url: String,
}

impl std::fmt::Debug for ConfigPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConfigPayload {{ issuer: {:?}, client_id: {:?}, client_secret: <redacted> }}",
            self.issuer, self.client_id
        )
    }
}

/// `PUT /api/auth/oidc/config`
pub async fn put_config(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    _: AdminUser,
    origin: Origin,
    Json(payload): Json<ConfigPayload>,
) -> AuthResult<Json<ConfigView>> {
    let config = OidcConfig {
        issuer: payload.issuer,
        client_id: payload.client_id,
        client_secret: payload.client_secret.unwrap_or_default(),
        provider_name: payload.provider_name,
        scopes: payload.scopes,
        auto_create: payload.auto_create,
        admin_groups: payload.admin_groups,
        groups_claim: payload.groups_claim,
        public_url: payload.public_url,
    }
    .normalized();

    if !config.issuer.is_empty() && !config.issuer.starts_with("https://") {
        // Un émetteur en clair livre le code d'autorisation et le secret client
        // au réseau ; seul un fournisseur de test justifie de l'accepter, et il
        // faut le dire à l'environnement (`DUMBMONIT_OIDC_ALLOW_HTTP=1`).
        let tolerated = state.config.oidc_allow_http && config.issuer.starts_with("http://");
        if !tolerated {
            return Err(crate::auth::AuthError::Invalid(
                "The issuer must be a URL starting with https:// (set DUMBMONIT_OIDC_ALLOW_HTTP=1 \
                 to allow a plain-HTTP provider for testing)."
                    .into(),
            ));
        }
    }
    if !config.public_url.is_empty()
        && !(config.public_url.starts_with("https://") || config.public_url.starts_with("http://"))
    {
        return Err(crate::auth::AuthError::Invalid(
            "The public URL must start with http:// or https://.".into(),
        ));
    }

    oidc::save(&state.pool, &state.cipher, config).await?;
    // Le fournisseur a pu changer : le document mis en cache ne vaut plus rien.
    *auth.discovery_cache().lock().await = None;
    tracing::info!("OIDC settings saved");

    let resolved = oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await?;
    Ok(Json(view(&state, resolved, &origin.0)))
}

/// `DELETE /api/auth/oidc/config` — oublie le réglage enregistré ; l'environnement
/// reprend la main, ou le SSO s'éteint.
pub async fn delete_config(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    _: AdminUser,
    origin: Origin,
) -> AuthResult<Json<ConfigView>> {
    oidc::clear(&state.pool).await?;
    *auth.discovery_cache().lock().await = None;
    tracing::info!("OIDC settings cleared");
    let resolved = oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await?;
    Ok(Json(view(&state, resolved, &origin.0)))
}

#[derive(Debug, Deserialize, Default)]
pub struct TestPayload {
    /// Émetteur à interroger ; celui de la configuration effective par défaut.
    #[serde(default)]
    issuer: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestReport {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discovery: Option<discovery::Discovery>,
}

/// `POST /api/auth/oidc/test` — lit le document de découverte et le rapporte.
/// Rien d'autre : ni jeton, ni connexion.
pub async fn test(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    _: AdminUser,
    payload: Option<Json<TestPayload>>,
) -> AuthResult<Json<TestReport>> {
    let issuer =
        match payload.and_then(|Json(payload)| payload.issuer).map(|s| s.trim().to_string()) {
            Some(issuer) if !issuer.is_empty() => issuer,
            _ => oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await?.config.issuer,
        };
    if issuer.is_empty() {
        return Ok(Json(TestReport {
            ok: false,
            error: Some("Enter the issuer URL first.".into()),
            discovery: None,
        }));
    }
    Ok(Json(match discovery::fetch(auth.http(), &issuer).await {
        Ok(document) => TestReport { ok: true, error: None, discovery: Some(document) },
        Err(error) => TestReport { ok: false, error: Some(format!("{error:#}")), discovery: None },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_follows_the_proxy_headers_then_the_host() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "192.168.1.10:8080".parse().unwrap());
        assert_eq!(request_origin(&headers, true), "http://192.168.1.10:8080");

        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("x-forwarded-host", "monit.example.org".parse().unwrap());
        assert_eq!(request_origin(&headers, true), "https://monit.example.org");
        // Sans mandataire de confiance devant, les en-têtes ne comptent pas.
        assert_eq!(request_origin(&headers, false), "http://192.168.1.10:8080");
    }

    #[test]
    fn the_callback_must_come_back_to_the_browser_that_started() {
        let cookie = binding_cookie("etat-1", false);
        let pair = cookie.to_str().unwrap().split(';').next().unwrap().to_string();
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, format!("autre=1; {pair}").parse().unwrap());
        assert!(binding_matches(&headers, Some("etat-1")));
        assert!(!binding_matches(&headers, Some("etat-2")));
        assert!(!binding_matches(&headers, None));
        assert!(!binding_matches(&HeaderMap::new(), Some("etat-1")));
    }

    #[test]
    fn only_internal_paths_are_accepted_as_destinations() {
        assert!(is_internal_path("/"));
        assert!(is_internal_path("/targets/3"));
        assert!(is_internal_path("/alerts?severity=warning&x=%2F#top"));
        assert!(!is_internal_path(""));
        assert!(!is_internal_path("targets"));
        assert!(!is_internal_path("//evil.example.org"));
        assert!(!is_internal_path("/\\evil.example.org"));
        assert!(!is_internal_path("/\\/evil.example.org"));
        assert!(!is_internal_path("/targets\\@evil.example.org"));
        assert!(!is_internal_path("https://evil.example.org"));
        assert!(!is_internal_path("javascript:alert(1)"));
        assert!(!is_internal_path("/x\r\nSet-Cookie: a=b"));
        assert!(!is_internal_path("/x\tSet-Cookie: a=b"));
    }
}
