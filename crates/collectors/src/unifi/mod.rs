//! UniFi Network : l'application auto-hébergée (UniFi Network Server) ou
//! celle d'une console UniFi OS (Dream Machine, Cloud Gateway, Cloud Key).
//!
//! Deux façons d'entrer, choisies par l'identifiant de la cible :
//!
//! * une **clé d'API** (`X-API-KEY`) : l'API d'intégration officielle
//!   (`/proxy/network/integration/v1` sur une console, `/integration/v1` sur
//!   un serveur auto-hébergé) donne version, équipements, leurs statistiques
//!   et le nombre de clients. L'état du WAN n'y figure pas : il est demandé à
//!   l'API classique avec la même clé, et sauté en silence si elle la refuse ;
//! * un **compte local en lecture seule** (rôle *View Only*) : l'API classique
//!   (`/api/s/<site>/stat/*`) donne tout cela, plus le WAN, Internet, les
//!   clients par type et les alarmes. La session est gardée d'une collecte à
//!   l'autre et rouverte quand elle expire.
//!
//! Aucune commande n'est jamais envoyée : ni redémarrage, ni adoption, ni
//! mise à jour. Le seul `POST` est la lecture du journal système, que l'API
//! n'expose pas autrement.
//!
//! # Clients suivis dans le tableau des appareils
//!
//! `/rest/user` (API classique, acceptée avec les deux formes d'accès) donne
//! les clients que le contrôleur a un jour vus, pas seulement ceux connectés
//! maintenant. Un réseau domestique en compte facilement des centaines —
//! téléphones de passage, Wi-Fi invité, objets connectés — et la plupart ne
//! reviendront jamais : les suivre tous noierait l'alerte de péremption sous
//! des appareils qui ne sont simplement jamais revenus. Par défaut, seuls les
//! clients que l'utilisateur a marqués d'une façon ou d'une autre comptent :
//! nommés dans UniFi, réservés en IP fixe, ou annotés (« Notes »). L'option
//! `watched_clients` ajoute des adresses MAC ou des noms précis à la liste, ou
//! vaut `all` pour tout suivre (déconseillé : un routeur domestique voit
//! défiler des dizaines de clients par jour).
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `site` | `default` | Nom court du site (celui de l'URL `/manage/<site>/…`). |
//! | `port` | `443` | 8443 pour un serveur auto-hébergé. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `15` | Délai par appel. |
//! | `watched_clients` | vide | Clients à suivre en plus des nommés/IP fixe/annotés : MAC ou noms, ou `all`. |
//! | `device_stale_days` | `3` | Jours sans connexion au-delà desquels un client suivi est signalé périmé. |

pub mod metrics;
pub mod model;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target, TargetId};
use futures::StreamExt;
use reqwest::header::{COOKIE, SET_COOKIE};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use tracing::warn;

use crate::api_options::{Connection, tag};
use crate::client_devices::{self, Device as ClientDevice};
use model::{
    Classic, ClientRecord, Device, DeviceState, Info, IntegrationDevice, Paged, Site, Statistics,
};

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const DEFAULT_SITE: &str = "default";

/// Au plus autant d'équipements dont on demande les statistiques, en parallèle
/// par huit, avec une clé d'API.
const MAX_STATISTICS: usize = 100;

/// Chemins de l'application Network derrière UniFi OS.
const CONSOLE_PREFIX: &str = "/proxy/network";

#[derive(Debug, Clone)]
enum Auth {
    ApiKey(String),
    Login { username: String, password: String },
}

#[derive(Debug, Clone)]
struct Settings {
    connection: Connection,
    auth: Auth,
    site: String,
}

impl Settings {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let auth = match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => {
                Auth::ApiKey(token.trim().to_string())
            }
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                Auth::Login { username: username.trim().to_string(), password: password.clone() }
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "UniFi Network expects an API key or the user name and password of a View \
                     Only administrator, configured: {other}"
                )));
            }
        };
        let mut target = target.clone();
        // Une adresse copiée depuis le navigateur finit souvent par `/manage…` ou `/network…`.
        let address = target.address.trim().to_string();
        for marker in ["/manage", "/network", "/proxy/network"] {
            if let Some(index) = address.find(marker)
                && address[..index].contains("://")
            {
                target.address = address[..index].to_string();
            }
        }
        let site = tag(&target, "site").unwrap_or(DEFAULT_SITE).to_string();
        if site.contains(['/', '?', '#', ' ']) {
            return Err(ProbeError::Config(format!(
                "The site \"{site}\" is not a site short name: use the name shown in the address \
                 bar after /manage/ or /network/, such as \"default\"."
            )));
        }
        Ok(Self {
            connection: Connection::from_target(
                &target,
                "https",
                80,
                DEFAULT_PORT,
                DEFAULT_REQUEST_TIMEOUT,
            )?,
            auth,
            site,
        })
    }

    fn cache_key(&self, id: TargetId) -> String {
        let who = match &self.auth {
            Auth::ApiKey(key) => format!("key:{}", key.len()),
            Auth::Login { username, .. } => format!("user:{username}"),
        };
        format!("{id}\u{0}{}\u{0}{who}", self.connection.base_url)
    }
}

/// Ce qui se garde d'une collecte à l'autre.
#[derive(Debug, Clone, Default)]
struct Session {
    /// `""` pour un serveur auto-hébergé, `/proxy/network` derrière UniFi OS ;
    /// `None` tant que ce n'est pas établi.
    prefix: Option<String>,
    /// Cookies de session (`unifises` ou `TOKEN`) : vide avec une clé d'API.
    cookies: String,
    csrf: Option<String>,
}

struct Reply {
    status: StatusCode,
    body: String,
    cookies: Vec<String>,
    csrf: Option<String>,
    final_path: String,
}

struct Client {
    http: reqwest::Client,
    settings: Settings,
    session: Session,
}

impl Session {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("")
    }
}

impl Client {
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.settings.connection.base_url)
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Reply, ProbeError> {
        let timeout = self.settings.connection.request_timeout;
        let mut request = self
            .http
            .request(method, self.url(path))
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(timeout);
        if let Auth::ApiKey(key) = &self.settings.auth {
            request = request.header("X-API-KEY", key);
        }
        if !self.session.cookies.is_empty() {
            request = request.header(COOKIE, &self.session.cookies);
        }
        if let Some(csrf) = &self.session.csrf {
            request = request.header("X-CSRF-Token", csrf);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                ProbeError::Timeout(timeout)
            } else {
                ProbeError::Unreachable(format!("{path}: {error}"))
            }
        })?;
        let status = response.status();
        let final_path = response.url().path().to_string();
        let headers = response.headers();
        let cookies = headers
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .filter_map(|v| v.split(';').next())
            .map(|pair| pair.trim().to_string())
            .filter(|pair| !pair.ends_with('='))
            .collect();
        let csrf = ["x-updated-csrf-token", "x-csrf-token"]
            .iter()
            .find_map(|name| headers.get(*name).and_then(|v| v.to_str().ok()))
            .map(str::to_string);
        let body = response
            .text()
            .await
            .map_err(|error| ProbeError::Unreachable(format!("{path}: {error}")))?;
        Ok(Reply { status, body, cookies, csrf, final_path })
    }

    fn error(&self, reply: &Reply, path: &str) -> ProbeError {
        let excerpt: String = reply.body.chars().take(160).collect();
        match reply.status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => match &self.settings.auth {
                Auth::ApiKey(_) => ProbeError::Auth(format!(
                    "UniFi refused the API key on {path} ({}). Create a new key in Settings → \
                     Control Plane → Integrations.",
                    reply.status
                )),
                Auth::Login { .. } => ProbeError::Auth(format!(
                    "UniFi refused the account on {path} ({}): {excerpt}",
                    reply.status
                )),
            },
            StatusCode::NOT_FOUND => ProbeError::Protocol(format!(
                "UniFi Network has no {path} at {}: check the address and the port (443 on a \
                 console, 8443 on a self-hosted server).",
                self.settings.connection.base_url
            )),
            StatusCode::TOO_MANY_REQUESTS => ProbeError::Protocol(
                "UniFi is rate-limiting logins after failed attempts: wait a few minutes.".into(),
            ),
            status if status.is_server_error() => {
                ProbeError::Unreachable(format!("UniFi answered {status} on {path}"))
            }
            status => ProbeError::Protocol(format!("UniFi answered {status} on {path}: {excerpt}")),
        }
    }

    async fn json(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, ProbeError> {
        let reply = self.send(method, path, body).await?;
        if !reply.status.is_success() {
            return Err(self.error(&reply, path));
        }
        serde_json::from_str(&reply.body).map_err(|error| {
            ProbeError::Protocol(format!("Unexpected answer from {path}: {error}"))
        })
    }

    /// Un `GET` de l'API classique, enveloppe `meta`/`data` vérifiée.
    async fn classic(&self, path: &str) -> Result<Vec<Value>, ProbeError> {
        let full = format!("{}{path}", self.session.prefix());
        let reply = self.send(Method::GET, &full, None).await?;
        let parsed: Option<Classic<Value>> = serde_json::from_str(&reply.body).ok();
        match parsed {
            Some(classic) if classic.meta.rc == "ok" && reply.status.is_success() => {
                Ok(classic.data)
            }
            Some(classic) => {
                let msg = classic.meta.msg.unwrap_or_default();
                Err(match msg.as_str() {
                    "api.err.LoginRequired" => {
                        ProbeError::Auth(format!("UniFi session expired on {full}"))
                    }
                    "api.err.NoSiteContext" | "api.err.NotFound" | "api.err.InvalidObject"
                        if path.contains("/api/s/") =>
                    {
                        ProbeError::Config(format!(
                            "UniFi knows no site \"{}\" ({msg}): set the Site option to the short \
                             name shown after /manage/ or /network/ in the address bar.",
                            self.settings.site
                        ))
                    }
                    "api.err.NoPermission" => ProbeError::Auth(format!(
                        "The UniFi account may not read {full} ({msg}): give it the View Only role on this site."
                    )),
                    _ => self.error(&reply, &full),
                })
            }
            None => Err(if reply.status.is_success() {
                ProbeError::Protocol(format!("Unexpected answer from {full}"))
            } else {
                self.error(&reply, &full)
            }),
        }
    }
}

/// Les clients à suivre dans le tableau générique des appareils ; voir le
/// commentaire de module.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ClientWatch {
    /// Seuls les clients marqués ([`ClientRecord::cared_about`]).
    Default,
    All,
    /// Marqués, plus ces adresses MAC ou ces noms précis.
    Listed(Vec<String>),
}

impl ClientWatch {
    fn parse(raw: Option<&str>) -> Self {
        let items: Vec<String> = raw
            .unwrap_or_default()
            .split(',')
            .map(|item| item.trim().to_ascii_lowercase())
            .filter(|item| !item.is_empty())
            .collect();
        match items.as_slice() {
            [] => Self::Default,
            [all] if all == "all" => Self::All,
            _ => Self::Listed(items),
        }
    }

    fn covers(&self, record: &ClientRecord) -> bool {
        if record.cared_about() {
            return true;
        }
        match self {
            Self::Default => false,
            Self::All => true,
            Self::Listed(items) => {
                let mac = record.mac.to_ascii_lowercase();
                items.iter().any(|item| {
                    *item == mac
                        || record.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(item))
                        || record.hostname.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(item))
                })
            }
        }
    }
}

/// Les clients retenus par `watch`, traduits vers le modèle générique
/// `client_devices::Device`.
fn watched_client_devices(records: &[ClientRecord], watch: &ClientWatch) -> Vec<ClientDevice> {
    records
        .iter()
        .filter(|record| watch.covers(record))
        .map(|record| ClientDevice {
            name: record.display_name(),
            device_type: match record.is_wired {
                Some(true) => "Wired".to_string(),
                Some(false) => "Wireless".to_string(),
                None => "Unknown".to_string(),
            },
            os: String::new(),
            user: String::new(),
            last_seen: record.last_seen,
            last_backup: None,
        })
        .collect()
}

/// `/rest/user` : les clients connus du site, nommés ou pas ; voir le
/// commentaire de module. Acceptée avec les deux formes d'authentification
/// (comme la santé classique lue avec une clé d'API, plus bas).
async fn known_clients(client: &Client, site: &str) -> Result<Vec<ClientRecord>, ProbeError> {
    let raw = client.classic(&format!("/api/s/{site}/rest/user")).await?;
    Ok(raw.iter().filter_map(model::classic_client).collect())
}

#[derive(Default)]
pub struct UnifiCollector {
    sessions: Mutex<HashMap<String, Session>>,
}

impl UnifiCollector {
    pub fn new() -> Self {
        Self::default()
    }

    fn cached(&self, key: &str) -> Option<Session> {
        self.sessions.lock().ok()?.get(key).cloned()
    }

    fn remember(&self, key: &str, session: Option<Session>) {
        if let Ok(mut sessions) = self.sessions.lock() {
            match session {
                Some(session) => sessions.insert(key.to_string(), session),
                None => sessions.remove(key),
            };
        }
    }

    async fn collect(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = Settings::from_target(target)?;
        let stale_days = client_devices::stale_days(target)?;
        let watch = ClientWatch::parse(tag(target, "watched_clients"));
        let key = settings.cache_key(target.id);
        let mut client = Client {
            http: crate::http::client(settings.connection.insecure_tls)?,
            session: self.cached(&key).unwrap_or_default(),
            settings,
        };
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let outcome = match client.settings.auth.clone() {
            Auth::ApiKey(_) => integration(&mut client, target.id, &watch, stale_days, ts_ms).await,
            Auth::Login { username, password } => {
                classic(&mut client, &username, &password, target.id, &watch, stale_days, ts_ms)
                    .await
            }
        };
        match &outcome {
            Ok(_) => self.remember(&key, Some(client.session.clone())),
            Err(_) => self.remember(&key, None),
        }
        outcome
    }
}

// ------------------------------------------------------------ compte local

/// Ouvre une session : UniFi OS (`/api/auth/login`) ou serveur auto-hébergé
/// (`/api/login`), reconnus à ce que répond la racine.
async fn login(client: &mut Client, username: &str, password: &str) -> Result<(), ProbeError> {
    client.session = Session::default();
    let root = client.send(Method::GET, "/", None).await?;
    // Le serveur auto-hébergé renvoie la racine vers `/manage` ; UniFi OS sert
    // sa page directement.
    let console = !root.final_path.starts_with("/manage");
    let (path, body) = if console {
        ("/api/auth/login", json!({"username": username, "password": password, "rememberMe": true}))
    } else {
        ("/api/login", json!({"username": username, "password": password, "remember": true}))
    };
    let reply = client.send(Method::POST, path, Some(&body)).await?;
    if !reply.status.is_success() {
        let detail = serde_json::from_str::<Value>(&reply.body).ok();
        let msg = detail
            .as_ref()
            .and_then(|v| v.pointer("/meta/msg").or_else(|| v.get("message")))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        return Err(match reply.status {
            StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                if msg.contains("2fa") || msg.contains("MFA") || msg.contains("Ubic2fa") {
                    ProbeError::Auth(
                        "UniFi asks for a second factor: use a local account without two-factor \
                         authentication, or an API key."
                            .into(),
                    )
                } else {
                    ProbeError::Auth(format!("UniFi refused the user name or password ({msg})."))
                }
            }
            _ => client.error(&reply, path),
        });
    }
    client.session = Session {
        prefix: Some(if console { CONSOLE_PREFIX.to_string() } else { String::new() }),
        cookies: reply.cookies.join("; "),
        csrf: reply.csrf,
    };
    if client.session.cookies.is_empty() {
        return Err(ProbeError::Protocol(
            "UniFi accepted the login but set no session cookie".into(),
        ));
    }
    Ok(())
}

async fn classic(
    client: &mut Client,
    username: &str,
    password: &str,
    target_id: TargetId,
    watch: &ClientWatch,
    stale_days: f64,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    if client.session.cookies.is_empty() {
        login(client, username, password).await?;
    }
    let site = client.settings.site.clone();
    let sysinfo = match client.classic(&format!("/api/s/{site}/stat/sysinfo")).await {
        Err(ProbeError::Auth(_)) => {
            // Session expirée : une seule nouvelle tentative.
            login(client, username, password).await?;
            client.classic(&format!("/api/s/{site}/stat/sysinfo")).await?
        }
        other => other?,
    };
    let mut out = Vec::new();
    if let Some(version) = sysinfo.first().and_then(|s| s.get("version")).and_then(Value::as_str) {
        out.push(metrics::version_sample(version, "classic", ts_ms));
    }
    let mut errors = 0u32;
    let (health_path, device_path) =
        (format!("/api/s/{site}/stat/health"), format!("/api/s/{site}/stat/device"));
    let (health, devices, alarms, clients) = futures::join!(
        client.classic(&health_path),
        client.classic(&device_path),
        alarm_count(client, &site, ts_ms),
        known_clients(client, &site),
    );
    match clients {
        Ok(records) => {
            let tracked = watched_client_devices(&records, watch);
            out.extend(client_devices::samples("unifi", &tracked, stale_days, ts_ms));
        }
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "clients connus UniFi illisibles");
        }
    }
    match health {
        Ok(entries) => {
            let health = model::health(&entries);
            let total = [health.clients_wireless, health.clients_wired, health.clients_guest]
                .iter()
                .flatten()
                .sum();
            out.extend(metrics::health_samples(&health, ts_ms));
            out.push(metrics::clients_sample(total, ts_ms));
        }
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "santé UniFi illisible");
        }
    }
    match devices {
        Ok(raw) => {
            let devices: Vec<Device> = raw.iter().filter_map(model::classic_device).collect();
            out.extend(metrics::device_samples(&devices, ts_ms));
        }
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "équipements UniFi illisibles");
        }
    }
    match alarms {
        Ok(count) => out.push(metrics::alarms_sample(count, ts_ms)),
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "alarmes UniFi illisibles");
        }
    }
    out.push(Sample::new("unifi_scrape_errors", f64::from(errors), MetricKind::Gauge, ts_ms));
    Ok(out)
}

/// Les alarmes : le journal système « critique » des dernières 24 heures
/// (UniFi Network 8 et suivants), ou les alarmes non archivées des versions
/// antérieures.
async fn alarm_count(client: &Client, site: &str, ts_ms: i64) -> Result<f64, ProbeError> {
    let path = format!("{}/v2/api/site/{site}/system-log/critical", client.session.prefix());
    let body = json!({
        "timestampFrom": ts_ms - 24 * 3600 * 1000,
        "timestampTo": ts_ms,
        "pageSize": 100,
        "pageNumber": 0,
    });
    let reply = client.send(Method::POST, &path, Some(&body)).await?;
    if reply.status == StatusCode::NOT_FOUND {
        let legacy = client.classic(&format!("/api/s/{site}/stat/alarm?archived=false")).await?;
        return Ok(legacy.len() as f64);
    }
    if !reply.status.is_success() {
        return Err(client.error(&reply, &path));
    }
    let value: Value = serde_json::from_str(&reply.body)
        .map_err(|error| ProbeError::Protocol(format!("Unexpected answer from {path}: {error}")))?;
    Ok(match &value {
        Value::Array(entries) => entries.len() as f64,
        other => other
            .get("total_element_count")
            .and_then(Value::as_f64)
            .or_else(|| other.get("data").and_then(Value::as_array).map(|d| d.len() as f64))
            .unwrap_or(0.0),
    })
}

// ------------------------------------------------------------ clé d'API

async fn integration(
    client: &mut Client,
    target_id: TargetId,
    watch: &ClientWatch,
    stale_days: f64,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    // Où vit l'API : derrière UniFi OS ou à la racine d'un serveur auto-hébergé.
    let mut info = None;
    let candidates = match &client.session.prefix {
        Some(known) => vec![known.clone()],
        None => vec![CONSOLE_PREFIX.to_string(), String::new()],
    };
    let mut last_error = None;
    for prefix in candidates {
        let path = format!("{prefix}/integration/v1/info");
        let reply = client.send(Method::GET, &path, None).await?;
        if reply.status == StatusCode::NOT_FOUND {
            last_error = Some(client.error(&reply, &path));
            continue;
        }
        if !reply.status.is_success() {
            return Err(client.error(&reply, &path));
        }
        let parsed: Info = serde_json::from_str(&reply.body).map_err(|error| {
            ProbeError::Protocol(format!("Unexpected answer from {path}: {error}"))
        })?;
        client.session.prefix = Some(prefix);
        info = Some(parsed);
        break;
    }
    let info = info.ok_or_else(|| {
        last_error.unwrap_or_else(|| ProbeError::Protocol("No UniFi integration API".into()))
    })?;
    let base = format!("{}/integration/v1", client.session.prefix());
    let mut out = vec![metrics::version_sample(&info.application_version, "integration", ts_ms)];

    let sites: Paged<Site> = serde_json::from_value(
        client.json(Method::GET, &format!("{base}/sites?limit=200"), None).await?,
    )
    .map_err(|error| ProbeError::Protocol(format!("Unexpected list of sites: {error}")))?;
    let wanted = client.settings.site.clone();
    let site = sites
        .data
        .iter()
        .find(|s| s.internal_reference.as_deref() == Some(wanted.as_str()) || s.id == wanted)
        .or_else(|| {
            sites
                .data
                .iter()
                .find(|s| s.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&wanted)))
        })
        .cloned()
        .ok_or_else(|| {
            let known: Vec<String> = sites
                .data
                .iter()
                .map(|s| s.internal_reference.clone().unwrap_or_else(|| s.id.clone()))
                .collect();
            ProbeError::Config(format!(
                "UniFi knows no site \"{wanted}\"; this key sees: {}.",
                known.join(", ")
            ))
        })?;

    let mut errors = 0u32;
    let mut raw_devices: Vec<IntegrationDevice> = Vec::new();
    let mut offset = 0u64;
    loop {
        let path = format!("{base}/sites/{}/devices?offset={offset}&limit=200", site.id);
        let page: Paged<IntegrationDevice> = serde_json::from_value(
            client.json(Method::GET, &path, None).await?,
        )
        .map_err(|error| ProbeError::Protocol(format!("Unexpected list of devices: {error}")))?;
        let received = page.data.len() as u64;
        raw_devices.extend(page.data);
        offset = page.offset + received;
        if received == 0 || offset >= page.total_count || raw_devices.len() >= metrics::MAX_DEVICES
        {
            break;
        }
    }

    // Statistiques des équipements en ligne, huit à la fois.
    let client_ref = &*client;
    let site_id = site.id.clone();
    let wanted_stats: Vec<String> = raw_devices
        .iter()
        .filter(|d| DeviceState::from_integration(&d.state) == DeviceState::Online)
        .take(MAX_STATISTICS)
        .map(|d| d.id.clone())
        .collect();
    let stats: HashMap<String, Result<Statistics, ProbeError>> =
        futures::stream::iter(wanted_stats)
            .map(|id| {
                let path = format!("{base}/sites/{site_id}/devices/{id}/statistics/latest");
                async move {
                    let result =
                        client_ref.json(Method::GET, &path, None).await.and_then(|value| {
                            serde_json::from_value(value).map_err(|error| {
                                ProbeError::Protocol(format!("Unexpected statistics: {error}"))
                            })
                        });
                    (id, result)
                }
            })
            .buffer_unordered(8)
            .collect()
            .await;
    if stats.values().any(Result::is_err) {
        errors += 1;
        warn!(target_id, "statistiques d'équipements UniFi partiellement illisibles");
    }
    let devices: Vec<Device> = raw_devices
        .into_iter()
        .map(|device| {
            let stat = stats.get(&device.id).and_then(|r| r.as_ref().ok()).cloned();
            device.into_device(stat)
        })
        .collect();
    out.extend(metrics::device_samples(&devices, ts_ms));

    match client.json(Method::GET, &format!("{base}/sites/{}/clients?limit=1", site.id), None).await
    {
        Ok(value) => match serde_json::from_value::<Paged<Value>>(value) {
            Ok(page) => out.push(metrics::clients_sample(page.total_count as f64, ts_ms)),
            Err(_) => errors += 1,
        },
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "clients UniFi illisibles");
        }
    }

    // Le WAN n'existe que dans l'API classique : même clé, au mieux.
    let site_ref = site.internal_reference.clone().unwrap_or_else(|| wanted.clone());
    match client.classic(&format!("/api/s/{site_ref}/stat/health")).await {
        Ok(entries) => out.extend(metrics::health_samples(&model::health(&entries), ts_ms)),
        Err(ProbeError::Auth(_) | ProbeError::Protocol(_) | ProbeError::Config(_)) => {}
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "santé UniFi illisible avec la clé d'API");
        }
    }
    // Les clients connus (`/rest/user`) ne sont pas dans l'API d'intégration
    // non plus : même clé, même tolérance que la santé ci-dessus.
    match known_clients(client, &site_ref).await {
        Ok(records) => {
            let tracked = watched_client_devices(&records, watch);
            out.extend(client_devices::samples("unifi", &tracked, stale_days, ts_ms));
        }
        Err(ProbeError::Auth(_) | ProbeError::Protocol(_) | ProbeError::Config(_)) => {}
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "clients connus UniFi illisibles avec la clé d'API");
        }
    }
    out.push(Sample::new("unifi_scrape_errors", f64::from(errors), MetricKind::Gauge, ts_ms));
    Ok(out)
}

#[async_trait]
impl Collector for UnifiCollector {
    fn kind(&self) -> &'static str {
        "unifi"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        self.collect(target).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.collect(target).await?;
        Ok(Some("unifi".to_string()))
    }
}

#[cfg(test)]
mod tests;
