//! Nginx Proxy Manager : les hôtes qu'il sert, et les certificats qu'ils
//! présentent.
//!
//! L'API REST (`/api`, port 81) est lue avec un compte dédié dont les droits se
//! limitent à *voir* les hôtes : jeton obtenu par `POST /api/tokens`, puis
//! `GET` des hôtes proxy, des redirections, des hôtes 404 (« dead hosts » dans
//! l'API) et des flux. Chaque hôte dit s'il est activé et si nginx a accepté sa
//! configuration (`meta.nginx_online`) : un hôte refusé par `nginx -t` n'est
//! plus servi, et l'interface de NPM ne le montre qu'en passant la souris sur
//! un point rouge.
//!
//! Les certificats ne sont **pas** lus dans l'API : `/api/nginx/certificates`
//! rend la clé privée des certificats importés et les identifiants du défi DNS
//! à tout compte qui a le droit de les voir. Ils sont lus là où ils servent :
//! une poignée de main TLS sur le port 443 de NPM, avec le nom de l'hôte, pour
//! chaque certificat attaché à un hôte activé. C'est aussi plus juste : on
//! mesure le certificat réellement présenté, pas celui que NPM croit servir.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de l'interface d'administration. |
//! | `port` | `81` | Port de l'interface d'administration. |
//! | `certificates` | `true` | Lit les certificats présentés sur `tls_port`. |
//! | `tls_port` | `443` | Port HTTPS des hôtes. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (API). |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use futures::StreamExt;
use serde::Deserialize;
use tracing::debug;

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::uptime::tags;
use crate::uptime::tls::{cert, handshake};

pub const DEFAULT_PORT: u16 = 81;
pub const DEFAULT_TLS_PORT: u16 = 443;
/// Hôtes décrits un par un, au plus.
const MAX_HOSTS: usize = 500;
/// Certificats vérifiés par passage, au plus.
const MAX_CERTIFICATES: usize = 100;
/// Poignées de main menées de front.
const PARALLEL_HANDSHAKES: usize = 16;
/// Délai d'une poignée de main : sur le réseau local, elle prend quelques
/// millisecondes.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);
/// Au-delà, les certificats pas encore lus attendent le passage suivant.
const CERTIFICATES_DEADLINE: Duration = Duration::from_secs(4);
/// Longueur maximale d'un message d'erreur de nginx repris en étiquette.
const MAX_ERROR_LABEL: usize = 200;
/// Un jeton est renouvelé quand il lui reste moins d'une heure.
const TOKEN_MARGIN_SECONDS: i64 = 3600;

/// Les quatre sortes d'hôtes, leur chemin dans l'API et leur nom en étiquette.
const HOST_TYPES: [(&str, &str); 4] = [
    ("proxy", "/api/nginx/proxy-hosts"),
    ("redirection", "/api/nginx/redirection-hosts"),
    ("dead", "/api/nginx/dead-hosts"),
    ("stream", "/api/nginx/streams"),
];

#[derive(Default)]
pub struct NpmCollector {
    /// Jetons par (adresse, compte) : NPM les donne pour vingt-quatre heures,
    /// inutile de se reconnecter à chaque passage.
    tokens: Mutex<HashMap<(String, String), (String, i64)>>,
}

impl NpmCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

struct Settings {
    api: HttpClient,
    base_url: String,
    http: reqwest::Client,
    timeout: Duration,
    username: String,
    password: String,
    certificates: bool,
    tls_port: u16,
}

fn settings(target: &Target) -> Result<Settings, ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    let (username, password) = match &target.credential {
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            (username.trim().to_string(), password.clone())
        }
        other => {
            return Err(ProbeError::Config(format!(
                "Nginx Proxy Manager expects the email and password of a user, configured: {other}"
            )));
        }
    };
    let http = crate::http::client(options.insecure_tls)?;
    let api = HttpClient::new(
        http.clone(),
        options.base_url.clone(),
        Auth::None,
        options.request_timeout,
        "Nginx Proxy Manager",
    );
    let tls_port = tags::parse_u32(target, "tls_port", u32::from(DEFAULT_TLS_PORT), 1..=65535)?;
    Ok(Settings {
        api,
        base_url: options.base_url,
        http,
        timeout: options.request_timeout,
        username,
        password,
        certificates: tags::parse_bool(target, "certificates", true)?,
        tls_port: tls_port as u16,
    })
}

#[async_trait]
impl Collector for NpmCollector {
    fn kind(&self) -> &'static str {
        "npm"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = settings(target)?;
        let version = read_root(&settings.api).await?;
        let mut hosts = BTreeMap::new();
        for (kind, path) in HOST_TYPES {
            if let Some(list) = self.read_hosts(&settings, path).await? {
                hosts.insert(kind, list);
            }
        }
        if !hosts.contains_key("proxy") {
            return Err(ProbeError::Auth(
                "The Nginx Proxy Manager user may not view proxy hosts: give it View on Proxy \
                 Hosts, and All Items."
                    .to_string(),
            ));
        }
        let now = chrono::Utc::now();
        let mut samples = samples(version.as_deref(), &hosts, now.timestamp_millis());
        if settings.certificates {
            samples.extend(
                certificates(&settings, &hosts, now.timestamp(), now.timestamp_millis()).await,
            );
        }
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read_root(&settings(target)?.api).await?;
        Ok(Some("npm".to_string()))
    }
}

// ------------------------------------------------------------------- API

#[derive(Debug, Deserialize)]
struct Root {
    /// `"OK"` : sans lui, ce n'est pas NPM qui a répondu.
    #[allow(dead_code)]
    status: String,
    version: Option<RootVersion>,
}

#[derive(Debug, Deserialize)]
struct RootVersion {
    major: u32,
    minor: u32,
    revision: u32,
}

/// `GET /api/`, sans jeton : prouve que c'est bien NPM et donne sa version.
async fn read_root(api: &HttpClient) -> Result<Option<String>, ProbeError> {
    let reply = api.get_json_raw("/api/").await?;
    if !reply.status.is_success() {
        return Err(api.status_error(reply.status, &reply.body, "/api/"));
    }
    let root: Root = serde_json::from_str(&reply.body).map_err(|_| {
        ProbeError::Protocol(format!(
            "{} does not look like Nginx Proxy Manager. Check the device type and the port of \
             its web interface (81 by default).",
            api.url("/api/")
        ))
    })?;
    Ok(root.version.map(|v| format!("{}.{}.{}", v.major, v.minor, v.revision)))
}

#[derive(Debug, Deserialize)]
struct Token {
    token: String,
    #[serde(default)]
    expires: String,
}

#[derive(serde::Serialize)]
struct Login<'a> {
    identity: &'a str,
    secret: &'a str,
}

impl NpmCollector {
    fn cache_key(settings: &Settings) -> (String, String) {
        (settings.base_url.clone(), settings.username.clone())
    }

    async fn token(&self, settings: &Settings, fresh: bool) -> Result<String, ProbeError> {
        let key = Self::cache_key(settings);
        let now = chrono::Utc::now().timestamp();
        if !fresh
            && let Some((token, expires)) =
                self.tokens.lock().ok().and_then(|t| t.get(&key).cloned())
            && expires - now > TOKEN_MARGIN_SECONDS
        {
            return Ok(token);
        }
        let body = Login { identity: &settings.username, secret: &settings.password };
        let reply = settings.api.post_raw("/api/tokens", &body).await?;
        if matches!(reply.status.as_u16(), 400 | 401 | 403) {
            return Err(ProbeError::Auth(
                "Nginx Proxy Manager refused the email or the password of the user.".to_string(),
            ));
        }
        if !reply.status.is_success() {
            return Err(settings.api.status_error(reply.status, &reply.body, "/api/tokens"));
        }
        let token: Token = serde_json::from_str(&reply.body).map_err(|error| {
            ProbeError::Protocol(format!("Unexpected response from /api/tokens: {error}"))
        })?;
        let expires = chrono::DateTime::parse_from_rfc3339(&token.expires)
            .map(|t| t.timestamp())
            .unwrap_or(now + TOKEN_MARGIN_SECONDS + 60);
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.insert(key, (token.token.clone(), expires));
        }
        Ok(token.token)
    }

    /// Une liste d'hôtes, ou `None` si le compte n'a pas le droit de la voir.
    async fn read_hosts(
        &self,
        settings: &Settings,
        path: &str,
    ) -> Result<Option<Vec<Host>>, ProbeError> {
        let mut fresh = false;
        loop {
            let token = self.token(settings, fresh).await?;
            let client = HttpClient::new(
                settings.http.clone(),
                settings.base_url.clone(),
                Auth::Bearer(token),
                settings.timeout,
                "Nginx Proxy Manager",
            );
            let reply = client.get_json_raw(path).await?;
            match reply.status.as_u16() {
                200..=299 => {
                    return serde_json::from_str(&reply.body).map(Some).map_err(|error| {
                        ProbeError::Protocol(format!("Unexpected response from {path}: {error}"))
                    });
                }
                403 => return Ok(None),
                // Un jeton expiré ou révoqué : NPM répond 401, parfois 500.
                401 | 500 if !fresh => {
                    if let Ok(mut tokens) = self.tokens.lock() {
                        tokens.remove(&Self::cache_key(settings));
                    }
                    fresh = true;
                }
                _ => return Err(client.status_error(reply.status, &reply.body, path)),
            }
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub nginx_online: Option<bool>,
    #[serde(default)]
    pub nginx_err: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Host {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub domain_names: Vec<String>,
    /// Booléen depuis la 2.10, entier avant.
    #[serde(default)]
    pub enabled: serde_json::Value,
    #[serde(default)]
    pub meta: Meta,
    #[serde(default)]
    pub certificate_id: serde_json::Value,
    #[serde(default)]
    pub incoming_port: Option<u64>,
}

impl Host {
    pub fn is_enabled(&self) -> bool {
        match &self.enabled {
            serde_json::Value::Bool(value) => *value,
            serde_json::Value::Number(value) => value.as_u64() != Some(0),
            _ => true,
        }
    }

    /// Faux seulement si nginx a refusé la configuration de l'hôte.
    pub fn is_online(&self) -> bool {
        self.meta.nginx_online != Some(false)
    }

    /// Le nom sous lequel l'hôte est montré : son premier domaine, ou le port
    /// d'écoute d'un flux.
    pub fn label(&self) -> String {
        match (self.domain_names.first(), self.incoming_port) {
            (Some(name), _) => name.to_ascii_lowercase(),
            (None, Some(port)) => format!("port {port}"),
            (None, None) => format!("#{}", self.id),
        }
    }

    /// L'identifiant du certificat attaché, s'il y en a un (`0` : aucun ;
    /// `"new"` n'existe que le temps d'une création).
    pub fn certificate(&self) -> Option<u64> {
        self.certificate_id.as_u64().filter(|id| *id > 0)
    }
}

fn first_error_line(error: &str) -> String {
    let line = error.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    let line = line.strip_prefix("nginx: ").unwrap_or(line);
    line.chars().take(MAX_ERROR_LABEL).collect()
}

pub fn samples(
    version: Option<&str>,
    hosts: &BTreeMap<&str, Vec<Host>>,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    if let Some(version) = version {
        out.push(gauge("npm_version_info", 1.0).with_label("version", version));
    }
    let mut described = 0usize;
    for (kind, list) in hosts {
        let enabled = list.iter().filter(|h| h.is_enabled()).count();
        let offline = list.iter().filter(|h| h.is_enabled() && !h.is_online()).count();
        out.push(gauge("npm_hosts", list.len() as f64).with_label("type", *kind));
        out.push(gauge("npm_hosts_enabled", enabled as f64).with_label("type", *kind));
        out.push(
            gauge("npm_hosts_disabled", (list.len() - enabled) as f64).with_label("type", *kind),
        );
        out.push(gauge("npm_hosts_offline", offline as f64).with_label("type", *kind));
        // Un hôte désactivé l'a été à dessein : il n'est pas décrit, et son
        // alerte éventuelle se résout d'elle-même.
        for host in list.iter().filter(|h| h.is_enabled()) {
            if described >= MAX_HOSTS {
                break;
            }
            described += 1;
            let name = host.label();
            out.push(
                gauge("npm_host_online", if host.is_online() { 1.0 } else { 0.0 })
                    .with_label("type", *kind)
                    .with_label("host", name.clone()),
            );
            if !host.is_online()
                && let Some(error) = host.meta.nginx_err.as_deref().map(first_error_line)
                && !error.is_empty()
            {
                out.push(
                    gauge("npm_host_error_info", 1.0)
                        .with_label("type", *kind)
                        .with_label("host", name)
                        .with_label("error", error),
                );
            }
        }
    }
    out
}

/// Un nom par certificat attaché à un hôte activé et servi : celui du premier
/// hôte qui le porte. Les flux n'ont pas de nom à présenter en SNI.
pub fn certificate_hosts(hosts: &BTreeMap<&str, Vec<Host>>) -> Vec<String> {
    let mut seen = BTreeMap::new();
    for (kind, list) in hosts {
        if *kind == "stream" {
            continue;
        }
        for host in list.iter().filter(|h| h.is_enabled() && h.is_online()) {
            // Un joker (`*.example.com`) ne se présente pas en SNI : le premier
            // nom exact de l'hôte fait l'affaire.
            let name = host.domain_names.iter().find(|name| !name.starts_with("*."));
            if let (Some(id), Some(name)) = (host.certificate(), name) {
                seen.entry(id).or_insert_with(|| name.to_ascii_lowercase());
            }
        }
    }
    seen.into_values().take(MAX_CERTIFICATES).collect()
}

/// Les certificats présentés sur le port HTTPS de NPM, lus par une poignée de
/// main par nom. Un nom qui ne répond pas est passé sous silence : l'hôte a
/// son propre moniteur, ici seule compte la date d'expiration.
///
/// Le tout doit tenir dans le délai global d'une interrogation (dix secondes
/// par défaut) : poignées de main brèves, menées de front, et arrêt net si la
/// première échoue faute de connexion — le port est alors filtré, inutile
/// d'attendre cent fois le même délai.
async fn certificates(
    settings: &Settings,
    hosts: &BTreeMap<&str, Vec<Host>>,
    now_s: i64,
    ts_ms: i64,
) -> Vec<Sample> {
    let names = certificate_hosts(hosts);
    let checked = names.len();
    let address = reqwest::Url::parse(&settings.base_url).ok().and_then(|u| {
        u.host_str().map(|h| h.trim_start_matches('[').trim_end_matches(']').to_string())
    });
    let timeout = settings.timeout.min(HANDSHAKE_TIMEOUT);
    let read_one = |name: String| {
        let address = address.clone();
        async move {
            let address = address?;
            let done = handshake::inspect(&address, settings.tls_port, &name, true, timeout).await;
            let done = match done {
                Ok(done) => done,
                Err(error) => {
                    debug!(host = %name, ?error, "poignée de main TLS impossible");
                    let unreachable = matches!(
                        error,
                        handshake::HandshakeError::Connect(_)
                            | handshake::HandshakeError::Timeout
                            | handshake::HandshakeError::Resolve(_)
                    );
                    return Some(Err(unreachable));
                }
            };
            match cert::parse_der(&done.leaf_der) {
                Ok(info) => Some(Ok((name, info.days_until_expiry(now_s)))),
                Err(error) => {
                    debug!(host = %name, %error, "certificat illisible");
                    Some(Err(false))
                }
            }
        }
    };
    let mut results = Vec::new();
    let mut names = names.into_iter();
    if let Some(first) = names.next() {
        let outcome = read_one(first).await;
        let port_closed = matches!(outcome, None | Some(Err(true)));
        results.push(outcome);
        if !port_closed {
            let rest: Vec<_> = futures::stream::iter(names)
                .map(read_one)
                .buffer_unordered(PARALLEL_HANDSHAKES)
                .take_until(tokio::time::sleep(CERTIFICATES_DEADLINE))
                .collect()
                .await;
            results.extend(rest);
        }
    }
    let read: Vec<(String, f64)> = results.into_iter().flatten().flatten().collect();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let mut out = vec![
        gauge("npm_certificates_checked", checked as f64),
        gauge("npm_certificates_read", read.len() as f64),
    ];
    for (name, days) in read {
        out.push(gauge("npm_cert_expiry_days", days).with_label("host", name));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Json;
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};

    use super::*;
    use crate::uptime::tags::test_support::cible;

    // Captures d'un Nginx Proxy Manager 2.16.0, lues avec un compte « View »
    // sur les hôtes et « Hidden » sur les certificats : trois hôtes proxy dont
    // un désactivé et un refusé par nginx (directive inconnue), une
    // redirection, un hôte 404 et un flux TCP.
    const ROOT: &str = include_str!("testdata/root.json");
    const PROXY: &str = include_str!("testdata/proxy-hosts.json");
    const REDIRECTION: &str = include_str!("testdata/redirection-hosts.json");
    const DEAD: &str = include_str!("testdata/dead-hosts.json");
    const STREAMS: &str = include_str!("testdata/streams.json");

    fn hosts() -> BTreeMap<&'static str, Vec<Host>> {
        BTreeMap::from([
            ("proxy", serde_json::from_str(PROXY).unwrap()),
            ("redirection", serde_json::from_str(REDIRECTION).unwrap()),
            ("dead", serde_json::from_str(DEAD).unwrap()),
            ("stream", serde_json::from_str(STREAMS).unwrap()),
        ])
    }

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    #[test]
    fn npm_reel() {
        let samples = samples(Some("2.16.0"), &hosts(), 0);
        let proxy = [("type", "proxy")];
        assert_eq!(find(&samples, "npm_hosts", &proxy).unwrap().value, 3.0);
        assert_eq!(find(&samples, "npm_hosts_disabled", &proxy).unwrap().value, 1.0);
        assert_eq!(find(&samples, "npm_hosts_offline", &proxy).unwrap().value, 1.0);
        let broken = find(&samples, "npm_host_online", &[("host", "broken.example.test")]);
        assert_eq!(broken.unwrap().value, 0.0);
        let error = find(&samples, "npm_host_error_info", &[("host", "broken.example.test")]);
        assert!(
            error.unwrap().labels["error"].starts_with("[emerg] unknown directive"),
            "{:?}",
            error.unwrap().labels
        );
        assert_eq!(
            find(&samples, "npm_host_online", &[("host", "app.example.test")]).unwrap().value,
            1.0
        );
        assert!(
            find(&samples, "npm_host_online", &[("host", "photos.example.test")]).is_none(),
            "un hôte désactivé n'est pas décrit"
        );
        assert_eq!(find(&samples, "npm_hosts", &[("type", "dead")]).unwrap().value, 1.0);
        assert_eq!(
            find(&samples, "npm_host_online", &[("type", "stream"), ("host", "port 9000")])
                .unwrap()
                .value,
            1.0
        );
        assert_eq!(find(&samples, "npm_version_info", &[]).unwrap().labels["version"], "2.16.0");
        // Seul app.example.test porte un certificat.
        assert_eq!(certificate_hosts(&hosts()), vec!["app.example.test"]);
    }

    #[test]
    fn un_hote_d_avant_la_2_10_a_un_entier_pour_etat() {
        let host: Host = serde_json::from_str(
            r#"{"id":4,"domain_names":["a.test"],"enabled":0,"certificate_id":"new","meta":{}}"#,
        )
        .unwrap();
        assert!(!host.is_enabled());
        assert!(host.is_online(), "sans verdict de nginx, l'hôte est tenu pour servi");
        assert_eq!(host.certificate(), None);
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        address.to_string()
    }

    /// Un faux NPM : le jeton `t1` est révoqué après la première lecture, le
    /// suivant (`t2`) reste valable ; les flux sont cachés au compte.
    #[tokio::test]
    async fn interrogation_complete_avec_un_jeton_revoque() {
        let logins = Arc::new(AtomicUsize::new(0));
        let counter = logins.clone();
        fn bearer(headers: &HeaderMap) -> String {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string()
        }
        let reads = Arc::new(AtomicUsize::new(0));
        let reads_proxy = reads.clone();
        let app = Router::new()
            .route("/api/", get(|| async { ROOT }))
            .route(
                "/api/tokens",
                post(move |Json(body): Json<serde_json::Value>| {
                    let counter = counter.clone();
                    async move {
                        if body["identity"] != "dumbmonit@example.test" || body["secret"] != "pw" {
                            return (StatusCode::BAD_REQUEST, String::new());
                        }
                        let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
                        let body =
                            format!(r#"{{"token":"t{n}","expires":"2999-01-01T00:00:00Z"}}"#);
                        (StatusCode::OK, body)
                    }
                }),
            )
            .route(
                "/api/nginx/proxy-hosts",
                get(move |h: HeaderMap| {
                    let reads = reads_proxy.clone();
                    async move {
                        let first = reads.fetch_add(1, Ordering::SeqCst) == 0;
                        match bearer(&h).as_str() {
                            "Bearer t1" if first => (StatusCode::OK, PROXY),
                            "Bearer t2" => (StatusCode::OK, PROXY),
                            _ => (StatusCode::INTERNAL_SERVER_ERROR, r#"{"error":{}}"#),
                        }
                    }
                }),
            )
            .route("/api/nginx/redirection-hosts", get(|| async { REDIRECTION }))
            .route("/api/nginx/dead-hosts", get(|| async { DEAD }))
            .route("/api/nginx/streams", get(|| async { (StatusCode::FORBIDDEN, "") }));
        let address = serve(app).await;
        let mut target = cible("npm", &address, &[("certificates", "false")]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit@example.test".into(),
            password: "pw".into(),
        };
        let collector = NpmCollector::new();
        let samples = collector.probe(&target).await.unwrap();
        assert_eq!(find(&samples, "npm_hosts_offline", &[("type", "proxy")]).unwrap().value, 1.0);
        assert!(find(&samples, "npm_hosts", &[("type", "stream")]).is_none());
        assert_eq!(logins.load(Ordering::SeqCst), 1);
        // Deuxième passage : le jeton en cache est révoqué, un seul nouveau suffit.
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 2);
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 2, "le jeton valable est réutilisé");

        target.credential = Credential::UsernamePassword {
            username: "dumbmonit@example.test".into(),
            password: "faux".into(),
        };
        let error = NpmCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    /// Un serveur TLS local qui présente un certificat de test (valable
    /// jusqu'en 2046) : le certificat d'app.example.test est lu par poignée
    /// de main, sans jamais passer par l'API des certificats.
    async fn tls_server() -> u16 {
        use rustls::pki_types::pem::PemObject;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};

        let certificate =
            CertificateDer::from_pem_slice(include_bytes!("testdata/test-only.crt")).unwrap();
        let key = PrivateKeyDer::from_pem_slice(include_bytes!("testdata/test-only.key")).unwrap();
        let config = rustls::ServerConfig::builder_with_provider(handshake::provider())
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![certificate], key)
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let _ = acceptor.accept(stream).await;
                });
            }
        });
        port
    }

    #[tokio::test]
    async fn les_certificats_sont_lus_par_poignee_de_main() {
        let app = Router::new()
            .route("/api/", get(|| async { ROOT }))
            .route(
                "/api/tokens",
                post(|| async { r#"{"token":"t","expires":"2999-01-01T00:00:00Z"}"# }),
            )
            .route("/api/nginx/proxy-hosts", get(|| async { PROXY }))
            .route("/api/nginx/redirection-hosts", get(|| async { REDIRECTION }))
            .route("/api/nginx/dead-hosts", get(|| async { DEAD }))
            .route("/api/nginx/streams", get(|| async { STREAMS }));
        let address = serve(app).await;
        let tls_port = tls_server().await.to_string();
        let mut target = cible("npm", &address, &[("tls_port", &tls_port)]);
        target.credential =
            Credential::UsernamePassword { username: "a@b.test".into(), password: "x".into() };
        let samples = NpmCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "npm_certificates_checked", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "npm_certificates_read", &[]).unwrap().value, 1.0);
        let days = find(&samples, "npm_cert_expiry_days", &[("host", "app.example.test")]);
        assert!(days.unwrap().value > 3000.0, "{:?}", days.unwrap().value);

        // Port HTTPS fermé : rien n'est lu, et l'interrogation aboutit vite.
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_port = closed.local_addr().unwrap().port().to_string();
        drop(closed);
        target.tags.insert("tls_port".into(), closed_port);
        let started = std::time::Instant::now();
        let samples = NpmCollector::new().probe(&target).await.unwrap();
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(find(&samples, "npm_certificates_read", &[]).unwrap().value, 0.0);
        assert!(find(&samples, "npm_cert_expiry_days", &[]).is_none());
    }

    #[tokio::test]
    async fn autre_chose_que_npm() {
        let app = Router::new().route("/api/", get(|| async { "<html></html>" }));
        let address = serve(app).await;
        let mut target = cible("npm", &address, &[]);
        target.credential =
            Credential::UsernamePassword { username: "a@b.test".into(), password: "x".into() };
        let error = NpmCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_NPM=hôte:81`,
    /// `DUMBMONIT_TEST_NPM_USER` et `DUMBMONIT_TEST_NPM_PASSWORD` ; le port
    /// HTTPS dans `DUMBMONIT_TEST_NPM_TLS_PORT` s'il n'est pas 443.
    #[tokio::test]
    #[ignore = "demande un Nginx Proxy Manager joignable"]
    async fn npm_reel_joignable() {
        let address = std::env::var("DUMBMONIT_TEST_NPM").unwrap();
        let port = std::env::var("DUMBMONIT_TEST_NPM_TLS_PORT").unwrap_or_else(|_| "443".into());
        let mut target = cible("npm", &address, &[("tls_port", &port)]);
        target.credential = Credential::UsernamePassword {
            username: std::env::var("DUMBMONIT_TEST_NPM_USER").unwrap(),
            password: std::env::var("DUMBMONIT_TEST_NPM_PASSWORD").unwrap(),
        };
        for sample in NpmCollector::new().probe(&target).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
