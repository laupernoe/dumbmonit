//! Domaine public d'une page de statut.
//!
//! Une page peut porter un nom d'hôte (`status.example.com`). Le mandataire
//! inverse de l'utilisateur fait pointer ce nom vers l'interface, sans réécrire
//! le chemin ; quand une requête arrive avec ce nom dans `Host`, le serveur sert
//! la page à la racine `/` — et rien d'autre de l'instance.
//!
//! C'est la propriété qui compte ici : sur ce domaine, seules les routes
//! publiques de *cette* page répondent (son document, ses badges, son flux, son
//! abonnement, les ressources du build). Connexion, installation, API
//! d'administration, réception des agents, MCP, métriques et pages des autres
//! slugs répondent 404, comme s'ils n'existaient pas. Aucun cookie n'entre ni ne
//! sort sur ce nom d'hôte.
//!
//! L'hôte est lu dans `Host` (ou l'autorité de l'URI en HTTP/2). `X-Forwarded-Host`
//! n'est considéré que s'il vient d'un mandataire déclaré de confiance
//! (`DUMBMONIT_TRUSTED_PROXIES`). Reconnaître un domaine ne fait jamais que
//! *restreindre* ce qui est servi : un client qui écrirait lui-même ces en-têtes
//! ne peut que s'enfermer dans la page de statut, pas en sortir.
//!
//! La correspondance domaine → slug est gardée en mémoire et rechargée après
//! toute écriture sur les pages ([`invalidate`]).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, RwLock};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ipnet::IpNet;

use crate::state::AppState;

/// Longueur maximale d'un nom de domaine (RFC 1035).
const MAX_DOMAIN_LEN: usize = 253;
const MAX_LABEL_LEN: usize = 63;

/// Requête (et réponse) servie sur le domaine public d'une page : le slug de
/// cette page. Posée par [`guard`], lue par `api::spa` (amorce de l'interface)
/// et par les en-têtes de sécurité.
#[derive(Debug, Clone)]
pub struct StatusHost {
    pub slug: String,
}

// --------------------------------------------------------------------------
// Validation
// --------------------------------------------------------------------------

/// Normalise et valide un domaine saisi : minuscules, sans schéma ni barre
/// finale ni point final. Vide : pas de domaine. L'erreur est un message
/// destiné à l'utilisateur.
pub fn normalise_domain(raw: &str) -> Result<Option<String>, String> {
    let mut value = raw.trim().to_ascii_lowercase();
    for scheme in ["https://", "http://"] {
        if let Some(rest) = value.strip_prefix(scheme) {
            value = rest.to_string();
        }
    }
    let value = value.strip_suffix('/').unwrap_or(&value);
    let value = value.strip_suffix('.').unwrap_or(value);
    if value.is_empty() {
        return Ok(None);
    }
    if !value.is_ascii() {
        return Err(
            "Use the ASCII (punycode) form of the name, for example xn--bcher-kva.example.".into(),
        );
    }
    if value.contains(['/', ':', '@', '?', '#', '[', ']']) || value.contains(char::is_whitespace) {
        return Err(
            "Enter the host name only, like status.example.com: no path, port or user.".into()
        );
    }
    let invalid = || {
        Err("Not a valid host name: use letters, digits, hyphens and dots, like \
             status.example.com."
            .to_string())
    };
    if value.len() > MAX_DOMAIN_LEN {
        return invalid();
    }
    let labels: Vec<&str> = value.split('.').collect();
    if labels.len() < 2 {
        return Err("Use a full domain name with a dot, like status.example.com.".into());
    }
    for label in &labels {
        if label.is_empty()
            || label.len() > MAX_LABEL_LEN
            || label.starts_with('-')
            || label.ends_with('-')
            || !label.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return invalid();
        }
    }
    // Un dernier label tout en chiffres est une adresse IPv4, pas un domaine.
    if labels.last().is_some_and(|tld| tld.bytes().all(|b| b.is_ascii_digit())) {
        return Err("Use a domain name, not an IP address.".into());
    }
    Ok(Some(value.to_string()))
}

/// Nom d'hôte d'une valeur `Host` (`Status.Example.com:443` → `status.example.com`).
pub fn host_of(value: &str) -> Option<String> {
    let value = value.trim();
    let host = if let Some(rest) = value.strip_prefix('[') {
        // IPv6 littérale : jamais un domaine de page, mais pas une raison d'échouer.
        &value[..rest.find(']').map_or(value.len(), |end| end + 2)]
    } else {
        value.split(':').next().unwrap_or(value)
    };
    let host = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// Hôte d'une URL publique réglée (`https://monit.example.com/`), s'il y en a un.
pub fn host_of_url(url: &str) -> Option<String> {
    let url = reqwest::Url::parse(url.trim()).ok()?;
    url.host_str().and_then(host_of)
}

// --------------------------------------------------------------------------
// Cache domaine → slug
// --------------------------------------------------------------------------

type Domains = Arc<HashMap<String, String>>;

/// Une entrée par base de données — par répertoire de données, en pratique
/// une seule ; les tests d'intégration en font tourner plusieurs côte à côte
/// dans le même processus.
static DOMAINS: LazyLock<RwLock<HashMap<PathBuf, Domains>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// Incrémenté à chaque invalidation : un chargement commencé avant ne remplit
/// pas le cache avec une liste déjà périmée.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Oublie la correspondance : à appeler après toute écriture sur les pages.
pub fn invalidate() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    DOMAINS.write().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
}

async fn domains(state: &AppState) -> Domains {
    let key = &state.config.data_dir;
    if let Some(cached) = DOMAINS.read().unwrap_or_else(|poisoned| poisoned.into_inner()).get(key) {
        return Arc::clone(cached);
    }
    let generation = GENERATION.load(Ordering::SeqCst);
    let map: Domains = match crate::db::status_pages::list_domains(&state.pool).await {
        Ok(rows) => Arc::new(rows.into_iter().collect()),
        Err(error) => {
            // Sans la liste, aucun domaine n'est reconnu : l'instance reste
            // servie normalement, derrière son authentification. Rien en cache,
            // la requête suivante réessaie.
            tracing::warn!(error = %format!("{error:#}"), "status page domains unreadable");
            return Arc::new(HashMap::new());
        }
    };
    let mut cache = DOMAINS.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    if GENERATION.load(Ordering::SeqCst) == generation {
        cache.insert(key.clone(), Arc::clone(&map));
    }
    map
}

// --------------------------------------------------------------------------
// Routage
// --------------------------------------------------------------------------

/// Noms d'hôte sous lesquels la requête est arrivée : `Host`, l'autorité de
/// l'URI, et `X-Forwarded-Host` seulement depuis un mandataire de confiance.
fn candidate_hosts(request: &Request, trusted: &[IpNet]) -> Vec<String> {
    let headers = request.headers();
    let mut hosts = Vec::new();
    if let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()).and_then(host_of) {
        hosts.push(host);
    }
    if let Some(host) = request.uri().host().and_then(host_of) {
        hosts.push(host);
    }
    let peer = request.extensions().get::<ConnectInfo<SocketAddr>>().map(|info| info.0.ip());
    if peer.is_some_and(|peer| trusted.iter().any(|net| net.contains(&peer))) {
        let forwarded = headers
            .get("x-forwarded-host")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(host_of);
        hosts.extend(forwarded);
    }
    hosts
}

/// Ce que le domaine d'une page laisse passer.
#[derive(Debug, PartialEq, Eq)]
enum Allowed {
    /// L'application, qui affiche la page publique.
    Page,
    /// Une ressource du build (`/_app/…`, icônes, manifeste).
    Asset,
    /// L'API publique de cette page, et d'elle seule.
    Api,
    No,
}

fn allowed(path: &str, slug: &str) -> Allowed {
    // Aucun segment `.`/`..` ni double barre : le chemin est comparé tel quel.
    if path.split('/').skip(1).any(|segment| segment == "." || segment == "..")
        || path.contains("//")
    {
        return Allowed::No;
    }
    if path == "/" {
        return Allowed::Page;
    }
    if let Some(rest) = path.strip_prefix("/s/").and_then(|rest| rest.strip_prefix(slug)) {
        return match rest {
            "" | "/" | "/embed" | "/confirm" | "/unsubscribe" => Allowed::Page,
            _ => Allowed::No,
        };
    }
    if let Some(rest) = path.strip_prefix("/api/public/status/").and_then(|r| r.strip_prefix(slug))
        && (rest.is_empty() || rest.starts_with('/'))
    {
        return Allowed::Api;
    }
    if path.starts_with("/_app/") {
        return Allowed::Asset;
    }
    // Fichiers à la racine du build : `favicon.svg`, `manifest.webmanifest`…
    // Seulement ceux qui existent : `/install.sh` n'en est pas un.
    let file = &path[1..];
    if !file.contains('/') && crate::api::spa::is_build_asset(file) {
        return Allowed::Asset;
    }
    Allowed::No
}

fn not_found(slug: &str) -> Response {
    let mut response = (StatusCode::NOT_FOUND, "Not found.").into_response();
    response.extensions_mut().insert(StatusHost { slug: slug.to_string() });
    response
}

/// Garde des domaines publics. Hors d'un domaine de page, la requête passe
/// sans changement.
pub async fn guard(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    let hosts = candidate_hosts(&request, &state.config.trusted_proxies);
    if hosts.is_empty() {
        return next.run(request).await;
    }
    let map = domains(&state).await;
    let Some(slug) = hosts.iter().find_map(|host| map.get(host)).cloned() else {
        return next.run(request).await;
    };

    if allowed(request.uri().path(), &slug) == Allowed::No {
        return not_found(&slug);
    }
    // Pas de session sur ce domaine : le cookie n'entre pas, aucun n'en sort.
    request.headers_mut().remove(header::COOKIE);
    request.extensions_mut().insert(StatusHost { slug: slug.clone() });
    let mut response = next.run(request).await;
    response.headers_mut().remove(header::SET_COOKIE);
    response.extensions_mut().insert(StatusHost { slug });
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_are_normalised() {
        assert_eq!(normalise_domain("  ").unwrap(), None);
        assert_eq!(normalise_domain("Status.Example.COM").unwrap().unwrap(), "status.example.com");
        assert_eq!(
            normalise_domain("https://status.example.com/").unwrap().unwrap(),
            "status.example.com"
        );
        assert_eq!(normalise_domain("status.example.com.").unwrap().unwrap(), "status.example.com");
        assert_eq!(normalise_domain("xn--bcher-kva.ch").unwrap().unwrap(), "xn--bcher-kva.ch");
    }

    #[test]
    fn invalid_domains_are_refused() {
        for bad in [
            "status",
            "status.example.com:8443",
            "status.example.com/path",
            "user@status.example.com",
            "-bad.example.com",
            "bad-.example.com",
            "bad..example.com",
            "under_score.example.com",
            "bücher.ch",
            "192.168.1.10",
            "[::1]",
            "a b.example.com",
        ] {
            assert!(normalise_domain(bad).is_err(), "{bad} should be refused");
        }
        let long = format!("{}.com", "a".repeat(64));
        assert!(normalise_domain(&long).is_err());
    }

    #[test]
    fn host_header_loses_port_and_case() {
        assert_eq!(host_of("Status.Example.com:443").unwrap(), "status.example.com");
        assert_eq!(host_of("status.example.com.").unwrap(), "status.example.com");
        assert_eq!(host_of("[::1]:8080").unwrap(), "[::1]");
        assert_eq!(host_of(""), None);
        assert_eq!(host_of_url("https://Monit.Example.com:8443/x").unwrap(), "monit.example.com");
    }

    #[test]
    fn only_the_page_its_api_and_the_build_are_allowed() {
        let slug = "acme";
        assert_eq!(allowed("/", slug), Allowed::Page);
        assert_eq!(allowed("/s/acme", slug), Allowed::Page);
        assert_eq!(allowed("/s/acme/embed", slug), Allowed::Page);
        assert_eq!(allowed("/s/acme/confirm", slug), Allowed::Page);
        assert_eq!(allowed("/api/public/status/acme", slug), Allowed::Api);
        assert_eq!(allowed("/api/public/status/acme/rss", slug), Allowed::Api);
        assert_eq!(allowed("/_app/immutable/x.js", slug), Allowed::Asset);
        assert_eq!(allowed("/favicon.svg", slug), Allowed::Asset);
        for refused in [
            "/login",
            "/setup",
            "/settings",
            "/status/1",
            "/index.html",
            "/s/other",
            "/s/acmex",
            "/s/acme/../../settings",
            "/api/public/status/other",
            "/api/public/status/acmex",
            "/api/public/status/acme/../../auth/status",
            "/api/auth/status",
            "/api/auth/login",
            "/api/health",
            "/api/ingest",
            "/api/mcp",
            "/api/push/abc",
            "/metrics",
            "/federate",
            "/install.sh",
            "/download/agent",
            "//api/targets",
        ] {
            assert_eq!(allowed(refused, slug), Allowed::No, "{refused}");
        }
    }
}
