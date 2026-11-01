//! HTTPS de l'interface et de l'API, sur le port même de l'interface.
//!
//! Le port de l'interface (`DUMBMONIT_BIND`, 8080) sert à la fois HTTP et
//! HTTPS ([`listener`]) : rien à publier de plus, et un mandataire inverse qui
//! parle HTTP au conteneur continue de fonctionner tel quel.
//!
//! - **Protocoles** : TLS 1.3, et TLS 1.2 avec les seules suites ECDHE + AEAD
//!   (AES-GCM, ChaCha20-Poly1305) ; échange de clés hybride post-quantique
//!   X25519MLKEM768 proposé en premier, X25519 et P-256 ensuite. Ni 0-RTT, ni
//!   suite CBC, ni RSA en échange de clés : rustls ne les implémente pas.
//! - **Certificat** : fourni (`DUMBMONIT_TLS_CERT` / `DUMBMONIT_TLS_KEY`,
//!   relu à chaque changement des fichiers ou sur `SIGHUP`), sinon autosigné
//!   et gardé dans la base ([`cert`]). Son empreinte est écrite dans le
//!   journal et montrée dans les réglages.
//! - **Redirection** : une navigation en clair, arrivée directement (sans
//!   mandataire), est renvoyée vers `https://` sur le même hôte et le même
//!   port. Active d'office sur une installation neuve ; sur une instance mise
//!   à jour, elle attend qu'un administrateur la pose (l'interface le lui
//!   propose), pour ne surprendre personne. Les API, scripts d'installation,
//!   binaires, mesures et pages de statut ne sont jamais redirigés : des
//!   scripts et des agents les lisent en HTTP.
//! - **Mandataires** : une requête qui porte `X-Forwarded-*` ou `Forwarded`
//!   n'est jamais redirigée — le mandataire choisit le schéma. Son
//!   `X-Forwarded-Proto: https` ne compte pour les cookies `Secure` et HSTS
//!   que s'il vient d'un mandataire de confiance (`DUMBMONIT_TRUSTED_PROXIES`).

pub mod audit;
pub mod cert;
mod listener;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use rustls::ServerConfig;
use rustls::crypto::CryptoProvider;
use serde::Serialize;

pub use listener::{WebIo, WebListener, WebPeer, listen};

use crate::state::AppState;

/// Réglage « rediriger HTTP vers HTTPS » (table `settings`, booléen JSON).
pub const SETTING_REDIRECT: &str = "web_https_redirect";
/// Certificat autosigné (table `settings`, [`cert::StoredCert`] en JSON).
const SETTING_CERT: &str = "web_tls_cert";
/// Clé du certificat autosigné, chiffrée par la clé de l'instance.
const SETTING_KEY: &str = "web_tls_key";

/// Durée de `Strict-Transport-Security` : un an, sans `includeSubDomains` —
/// le domaine porte souvent d'autres services qui ne sont pas les nôtres.
pub const HSTS: &str = "max-age=31536000";

/// Ce que la couche [`transport`] a établi de la requête.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Transport {
    /// Chiffrée jusqu'à ce serveur (HTTPS direct).
    pub tls: bool,
    /// Le navigateur est en HTTPS : direct, ou derrière un mandataire de
    /// confiance qui le dit (`X-Forwarded-Proto: https`).
    pub https: bool,
}

/// Provenance du certificat présenté.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Provided,
    SelfSigned,
}

/// Le certificat en service, tel que les réglages le montrent.
#[derive(Debug, Clone, Serialize)]
pub struct CertSummary {
    pub source: Source,
    /// Empreinte SHA-256 du certificat, comme l'affichent les navigateurs.
    pub fingerprint_sha256: String,
    /// Épingle de la clé publique (`sha256//…`), celle de `curl --pinnedpubkey`.
    pub public_key_pin: Option<String>,
    pub subject: String,
    pub issuer: String,
    pub not_after: Option<String>,
    /// Noms du certificat autosigné (vide pour un certificat fourni).
    pub names: Vec<String>,
    #[serde(skip)]
    pub der: Vec<u8>,
}

/// État du HTTPS en mémoire, partagé par l'écoute et l'API.
pub struct WebTls {
    config: RwLock<Option<Arc<ServerConfig>>>,
    summary: RwLock<Option<CertSummary>>,
    /// Dernière erreur de chargement du certificat fourni (gardé l'ancien).
    error: RwLock<Option<String>>,
    redirect: AtomicBool,
}

impl Default for WebTls {
    fn default() -> Self {
        Self::new()
    }
}

impl WebTls {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(None),
            summary: RwLock::new(None),
            error: RwLock::new(None),
            redirect: AtomicBool::new(false),
        }
    }

    /// Configuration TLS à servir ; `None` : le port ne sert que HTTP.
    pub fn server_config(&self) -> Option<Arc<ServerConfig>> {
        self.config.read().ok().and_then(|config| config.clone())
    }

    pub fn summary(&self) -> Option<CertSummary> {
        self.summary.read().ok().and_then(|summary| summary.clone())
    }

    pub fn error(&self) -> Option<String> {
        self.error.read().ok().and_then(|error| error.clone())
    }

    pub fn redirect_enabled(&self) -> bool {
        self.redirect.load(Ordering::Relaxed)
    }

    pub fn set_redirect(&self, on: bool) {
        self.redirect.store(on, Ordering::Relaxed);
    }

    fn install(&self, config: Arc<ServerConfig>, summary: CertSummary) {
        if let Ok(mut slot) = self.config.write() {
            *slot = Some(config);
        }
        if let Ok(mut slot) = self.summary.write() {
            *slot = Some(summary);
        }
        self.set_error(None);
    }

    fn set_error(&self, error: Option<String>) {
        if let Ok(mut slot) = self.error.write() {
            *slot = error;
        }
    }
}

/// Fournisseur cryptographique de l'interface : aws-lc-rs, suites AEAD
/// seulement, échange de clés hybride post-quantique d'abord.
pub fn provider() -> Arc<CryptoProvider> {
    use rustls::crypto::aws_lc_rs::{self, cipher_suite as suite, kx_group};
    Arc::new(CryptoProvider {
        cipher_suites: vec![
            suite::TLS13_AES_256_GCM_SHA384,
            suite::TLS13_AES_128_GCM_SHA256,
            suite::TLS13_CHACHA20_POLY1305_SHA256,
            suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
            suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
            suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
            suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
            suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
            suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        ],
        kx_groups: vec![
            kx_group::X25519MLKEM768,
            kx_group::X25519,
            kx_group::SECP256R1,
            kx_group::SECP384R1,
        ],
        ..aws_lc_rs::default_provider()
    })
}

/// Configuration rustls d'une identité : TLS 1.3 et 1.2, ordre du serveur,
/// HTTP/1.1.
pub fn server_config(identity: cert::Identity) -> Result<ServerConfig> {
    let mut config = ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .context("TLS versions")?
        .with_no_client_auth()
        .with_single_cert(identity.chain, identity.key)
        .context("HTTPS certificate and key do not match or are not supported")?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config.ignore_client_order = true;
    Ok(config)
}

/// Prépare le HTTPS au démarrage : certificat, puis réglage de redirection.
pub async fn init(state: &AppState) -> Result<()> {
    if state.config.tls_enabled {
        if let Err(error) = load(state).await {
            // Un certificat fourni et illisible est une erreur de
            // configuration : mieux vaut s'arrêter que servir autre chose.
            if state.config.tls_cert.is_some() || state.config.tls_key.is_some() {
                return Err(error);
            }
            let message = format!("{error:#}");
            tracing::warn!(error = %message, "HTTPS unavailable: the web port serves HTTP only");
            state.web_tls().set_error(Some(message));
        }
        if let Some(summary) = state.web_tls().summary() {
            tracing::info!(
                source = ?summary.source,
                fingerprint = %summary.fingerprint_sha256,
                not_after = summary.not_after.as_deref().unwrap_or("?"),
                "HTTPS available on the web port (TLS 1.3/1.2, X25519MLKEM768)"
            );
        }
    } else {
        tracing::warn!("DUMBMONIT_TLS=off: the web port serves plain HTTP only");
    }
    let redirect = match state.config.https_redirect {
        Some(forced) => forced,
        None if state.config.demo => false,
        None => match crate::auth::settings::get::<bool>(&state.pool, SETTING_REDIRECT).await? {
            Some(on) => on,
            None => {
                // Installation neuve : rien à ménager, le HTTPS d'emblée.
                // Instance mise à jour : on n'impose rien, l'interface propose.
                let fresh = crate::auth::users::count(&state.pool).await? == 0;
                crate::auth::settings::set(&state.pool, SETTING_REDIRECT, &fresh).await?;
                fresh
            }
        },
    };
    state.web_tls().set_redirect(redirect && state.config.tls_enabled);
    Ok(())
}

/// Charge (ou fabrique) le certificat et installe la configuration TLS.
pub async fn load(state: &AppState) -> Result<()> {
    let (identity, summary) = match (&state.config.tls_cert, &state.config.tls_key) {
        (Some(cert_path), Some(key_path)) => provided(cert_path, key_path)?,
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("DUMBMONIT_TLS_CERT and DUMBMONIT_TLS_KEY go together: set both or none")
        }
        (None, None) => self_signed(state).await?,
    };
    let config = server_config(identity)?;
    state.web_tls().install(Arc::new(config), summary);
    Ok(())
}

fn provided(cert_path: &Path, key_path: &Path) -> Result<(cert::Identity, CertSummary)> {
    let identity = cert::read_provided(cert_path, key_path)?;
    let leaf = identity.chain[0].as_ref().to_vec();
    let info = dumbmonit_collectors::uptime::parse_certificate(&leaf).ok();
    let summary = CertSummary {
        source: Source::Provided,
        fingerprint_sha256: cert::display_fingerprint(&leaf),
        public_key_pin: None,
        subject: info.as_ref().map(|i| i.subject.clone()).unwrap_or_default(),
        issuer: info.as_ref().map(|i| i.issuer.clone()).unwrap_or_default(),
        not_after: info.as_ref().and_then(|i| iso(i.not_after_s * 1000)),
        names: Vec::new(),
        der: leaf,
    };
    Ok((identity, summary))
}

/// Noms voulus pour le certificat autosigné. Le nom d'un conteneur Docker
/// laissé par défaut (les douze premiers caractères de son identifiant)
/// change à chaque recréation : il referait le certificat à chaque mise à
/// jour, il n'est donc pas retenu.
fn wanted_names(state: &AppState) -> Vec<String> {
    let hostname = std::fs::read_to_string("/etc/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .map(|name| name.trim().to_string())
        .filter(|name| !cert::is_container_id(name));
    let public_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
    cert::default_names(hostname.as_deref(), public_url.as_deref(), &state.config.tls_names)
}

async fn self_signed(state: &AppState) -> Result<(cert::Identity, CertSummary)> {
    let pool = &state.pool;
    let names = wanted_names(state);
    let stored: Option<cert::StoredCert> = crate::auth::settings::get(pool, SETTING_CERT).await?;
    let key_hex = crate::auth::settings::get_secret(pool, &state.cipher, SETTING_KEY).await?;
    let key = key_hex.as_deref().map(hex::decode).transpose().context("stored HTTPS key")?;
    let now = chrono::Utc::now().timestamp_millis();
    let (cert_der, key_der, stored) = match (stored, key) {
        (Some(stored), Some(key)) if !stored.needs_renewal(&names, now) => {
            (stored.der()?, key, stored)
        }
        (_, key) => {
            let renewing = key.is_some();
            let made = cert::self_signed(key.as_deref(), &names)?;
            let key_hex = hex::encode(&made.key_der);
            crate::auth::settings::set_secret(pool, &state.cipher, SETTING_KEY, &key_hex).await?;
            crate::auth::settings::set(pool, SETTING_CERT, &made.stored).await?;
            tracing::info!(
                renewed = renewing,
                names = ?made.stored.names,
                fingerprint = %cert::display_fingerprint(&made.cert_der),
                "self-signed HTTPS certificate created (DUMBMONIT_TLS_CERT/KEY to use your own)"
            );
            (made.cert_der, made.key_der, made.stored)
        }
    };
    let summary = CertSummary {
        source: Source::SelfSigned,
        fingerprint_sha256: cert::display_fingerprint(&cert_der),
        public_key_pin: cert::spki_pin(&key_der).ok(),
        subject: "DumbMonit (self-signed)".into(),
        issuer: "DumbMonit (self-signed)".into(),
        not_after: iso(stored.not_after_ms),
        names: stored.names.clone(),
        der: cert_der.clone(),
    };
    Ok((cert::identity_from_parts(cert_der, key_der), summary))
}

fn iso(ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Empreinte de modification des fichiers du certificat fourni.
fn stamp(paths: &[PathBuf]) -> Vec<Option<(SystemTime, u64)>> {
    paths
        .iter()
        .map(|path| {
            let meta = std::fs::metadata(path).ok()?;
            Some((meta.modified().ok()?, meta.len()))
        })
        .collect()
}

/// Recharge le certificat fourni quand ses fichiers changent (ou sur
/// `SIGHUP`), et renouvelle chaque jour le certificat autosigné s'il le faut.
/// Un certificat fourni illisible laisse l'ancien en service, et le dit.
pub fn spawn_refresh(state: AppState) {
    if !state.config.tls_enabled {
        return;
    }
    tokio::spawn(async move {
        let files: Vec<PathBuf> = [state.config.tls_cert.clone(), state.config.tls_key.clone()]
            .into_iter()
            .flatten()
            .collect();
        let provided = files.len() == 2;
        let mut last = stamp(&files);
        let every = if provided { Duration::from_secs(60) } else { Duration::from_secs(24 * 3600) };
        let mut tick = tokio::time::interval(every);
        tick.tick().await;
        #[cfg(unix)]
        let mut hangup =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).ok();
        loop {
            #[cfg(unix)]
            let forced = match hangup.as_mut() {
                Some(signal) => tokio::select! {
                    _ = tick.tick() => false,
                    _ = signal.recv() => true,
                },
                None => {
                    tick.tick().await;
                    false
                }
            };
            #[cfg(not(unix))]
            let forced = {
                tick.tick().await;
                false
            };
            if provided {
                let now = stamp(&files);
                if !forced && now == last {
                    continue;
                }
                last = now;
            }
            match load(&state).await {
                Ok(()) => {
                    if let Some(summary) = state.web_tls().summary() {
                        let fingerprint = summary.fingerprint_sha256;
                        tracing::info!(%fingerprint, "HTTPS certificate reloaded");
                    }
                }
                Err(error) => {
                    let message = format!("{error:#}");
                    tracing::warn!(
                        error = %message,
                        "HTTPS certificate not reloaded, the previous one stays"
                    );
                    state.web_tls().set_error(Some(message));
                }
            }
        }
    });
}

/// Mandataire de confiance qui dit `https`.
fn trusted_https(headers: &HeaderMap, peer: Option<SocketAddr>, trusted: &[ipnet::IpNet]) -> bool {
    peer.is_some_and(|peer| trusted.iter().any(|net| net.contains(&peer.ip())))
        && headers
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .is_some_and(|proto| proto.trim().eq_ignore_ascii_case("https"))
}

/// Couche la plus externe : pose `ConnectInfo<SocketAddr>` (que le reste du
/// serveur lit) à partir de ce que l'écoute a appris, et [`Transport`].
pub async fn transport(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let peer = request.extensions().get::<ConnectInfo<WebPeer>>().map(|info| info.0);
    if let Some(peer) = peer {
        request.extensions_mut().insert(ConnectInfo(peer.addr));
    }
    let addr = request.extensions().get::<ConnectInfo<SocketAddr>>().map(|info| info.0);
    let tls = peer.is_some_and(|peer| peer.tls);
    let https = tls || trusted_https(request.headers(), addr, &state.config.trusted_proxies);
    request.extensions_mut().insert(Transport { tls, https });
    let mut response = next.run(request).await;
    if https {
        let hsts = HeaderValue::from_static(HSTS);
        response.headers_mut().insert(header::STRICT_TRANSPORT_SECURITY, hsts);
    }
    response
}

/// Chemins jamais redirigés : lus par des scripts, des agents, des
/// collecteurs ou intégrés ailleurs.
fn never_redirected(path: &str) -> bool {
    const PREFIXES: &[&str] = &["/api/", "/s/", "/download/", "/prometheus/", "/.well-known/"];
    const EXACT: &[&str] =
        &["/install.sh", "/install.ps1", "/metrics", "/federate", "/robots.txt", "/sitemap.xml"];
    PREFIXES.iter().any(|prefix| path.starts_with(prefix)) || EXACT.contains(&path)
}

/// URL `https://` vers laquelle renvoyer cette requête, s'il le faut.
pub fn redirect_target(
    request: &Request,
    redirect_on: bool,
    trusted: &[ipnet::IpNet],
) -> Option<String> {
    let method = request.method();
    if !redirect_on || (*method != Method::GET && *method != Method::HEAD) {
        return None;
    }
    // Seule l'écoute réelle pose `WebPeer` : ni un test, ni une connexion déjà
    // chiffrée, ni la boucle locale (sonde de santé, tunnel SSH).
    let peer = request.extensions().get::<ConnectInfo<WebPeer>>()?.0;
    let ip = peer.addr.ip();
    if peer.tls || ip.is_loopback() || trusted.iter().any(|net| net.contains(&ip)) {
        return None;
    }
    if request.extensions().get::<crate::status_host::StatusHost>().is_some() {
        return None;
    }
    let headers = request.headers();
    let forwarded =
        ["forwarded", "x-forwarded-for", "x-forwarded-proto", "x-forwarded-host", "x-real-ip"];
    if forwarded.iter().any(|name| headers.contains_key(*name)) {
        return None;
    }
    // Une navigation de navigateur, pas un `curl` : `Accept: text/html`.
    let navigation = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("text/html"));
    let path = request.uri().path();
    if !navigation || never_redirected(path) {
        return None;
    }
    let host = headers.get(header::HOST).and_then(|value| value.to_str().ok())?.trim();
    let clean = host.bytes().all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b));
    if host.is_empty() || !clean {
        return None;
    }
    let query = request.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
    Some(format!("https://{host}{path}{query}"))
}

/// Redirection HTTP → HTTPS des navigations de l'interface.
pub async fn redirect(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let web = state.web_tls();
    let on = web.redirect_enabled() && web.server_config().is_some();
    match redirect_target(&request, on, &state.config.trusted_proxies) {
        Some(location) => match HeaderValue::from_str(&location) {
            // 307 et non 301/308 : une redirection permanente resterait en
            // cache si le HTTPS était coupé plus tard ; HSTS fait ce travail.
            Ok(location) => {
                (StatusCode::TEMPORARY_REDIRECT, [(header::LOCATION, location)]).into_response()
            }
            Err(_) => next.run(request).await,
        },
        None => next.run(request).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn request(path: &str, peer: Option<WebPeer>, headers: &[(&str, &str)]) -> Request {
        let mut builder =
            axum::http::Request::builder().uri(path).header(header::HOST, "nas.lan:8080");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let mut request = builder.body(Body::empty()).unwrap();
        if let Some(peer) = peer {
            request.extensions_mut().insert(ConnectInfo(peer));
        }
        request
    }

    fn lan(tls: bool) -> Option<WebPeer> {
        Some(WebPeer { addr: "192.168.1.20:50000".parse().unwrap(), tls })
    }

    const HTML: (&str, &str) = ("accept", "text/html,application/xhtml+xml");

    #[test]
    fn a_plain_navigation_goes_to_https_on_the_same_host_and_port() {
        let asked = request("/alerts?state=open", lan(false), &[HTML]);
        let target = redirect_target(&asked, true, &[]);
        assert_eq!(target.as_deref(), Some("https://nas.lan:8080/alerts?state=open"));
    }

    #[test]
    fn scripts_agents_proxies_and_the_loopback_are_never_redirected() {
        // Réglage coupé.
        assert!(redirect_target(&request("/", lan(false), &[HTML]), false, &[]).is_none());
        // Déjà en HTTPS.
        assert!(redirect_target(&request("/", lan(true), &[HTML]), true, &[]).is_none());
        // Pas une navigation (curl, agent).
        assert!(redirect_target(&request("/", lan(false), &[]), true, &[]).is_none());
        // Chemins lus par des scripts ou intégrés ailleurs.
        for path in ["/api/health", "/install.sh", "/download/x", "/metrics", "/s/home/embed"] {
            let asked = request(path, lan(false), &[HTML]);
            assert!(redirect_target(&asked, true, &[]).is_none(), "{path}");
        }
        // Derrière un mandataire, c'est lui qui choisit le schéma.
        let proxied = request("/", lan(false), &[HTML, ("x-forwarded-proto", "http")]);
        assert!(redirect_target(&proxied, true, &[]).is_none());
        // Un mandataire de confiance, même sans en-tête.
        let trusted: Vec<ipnet::IpNet> = vec!["192.168.1.0/24".parse().unwrap()];
        assert!(redirect_target(&request("/", lan(false), &[HTML]), true, &trusted).is_none());
        // La boucle locale.
        let local = Some(WebPeer { addr: "127.0.0.1:4000".parse().unwrap(), tls: false });
        assert!(redirect_target(&request("/", local, &[HTML]), true, &[]).is_none());
        // Sans l'écoute réelle (tests, autre service).
        assert!(redirect_target(&request("/", None, &[HTML]), true, &[]).is_none());
        // Une écriture n'est jamais redirigée.
        let mut post = request("/", lan(false), &[HTML]);
        *post.method_mut() = Method::POST;
        assert!(redirect_target(&post, true, &[]).is_none());
    }

    #[test]
    fn a_forged_host_cannot_inject_a_location() {
        let request = request("/", lan(false), &[HTML]);
        let mut forged = request;
        forged.headers_mut().insert(header::HOST, HeaderValue::from_static("evil.example/x"));
        assert!(redirect_target(&forged, true, &[]).is_none());
    }

    #[test]
    fn https_behind_a_proxy_counts_only_when_the_proxy_is_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        let peer: SocketAddr = "10.0.0.2:4000".parse().unwrap();
        let trusted: Vec<ipnet::IpNet> = vec!["10.0.0.0/8".parse().unwrap()];
        assert!(trusted_https(&headers, Some(peer), &trusted));
        assert!(!trusted_https(&headers, Some(peer), &[]));
        assert!(!trusted_https(&HeaderMap::new(), Some(peer), &trusted));
    }

    #[test]
    fn only_modern_suites_and_post_quantum_key_exchange_are_offered() {
        let provider = provider();
        assert_eq!(provider.cipher_suites.len(), 9);
        for suite in &provider.cipher_suites {
            let name = format!("{:?}", suite.suite());
            assert!(name.contains("GCM") || name.contains("CHACHA20"), "{name}");
            assert!(!name.contains("CBC") && !name.contains("SHA1"), "{name}");
        }
        assert_eq!(format!("{:?}", provider.kx_groups[0].name()), "X25519MLKEM768");
    }
}
