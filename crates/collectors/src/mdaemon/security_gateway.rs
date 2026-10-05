//! SecurityGateway for Email Servers : les services de la passerelle, et l'API
//! REST de la version 12.5 et suivantes quand une clé d'API est fournie.
//!
//! L'API REST est documentée ainsi, et rien de plus n'est supposé ici :
//!
//! * racine `https://<serveur>:<port>/api/v1`, port HTTPS 4443 par défaut ;
//! * authentification `Authorization: Bearer <clé>` ;
//! * réponses `{"success": true, "data": …}`, erreurs avec un statut HTTP et un
//!   corps `error` / `errorCode` / `statusCode` ;
//! * spécification OpenAPI servie à `/api/v1/openapi` ;
//! * « des compteurs de performance en lecture seule ».
//!
//! Le chemin des compteurs n'est pas publié en ligne : il est lu dans la
//! spécification (le seul chemin `GET` sans paramètre qui parle de compteurs).
//! Leurs noms non plus : chaque valeur numérique de `data` devient une série
//! `securitygateway_counter{counter="…"}`, le nom étant la clé JSON mise en
//! `snake_case`. Rien n'est renommé ni deviné.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `services` | `smtp,web` | Services vérifiés, `nom` ou `nom:port`. |
//! | `request_timeout_seconds` | `10` | Délai par connexion et par appel. |
//! | `api_port` | `4443` | Port HTTPS de l'interface web, qui sert l'API REST. |
//! | `api_tls` | `true` | HTTPS vers l'API (sinon HTTP, sur le port 4000 en général). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `counters` | `true` | Lit les compteurs de performance. |

use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::Value;
use tracing::{debug, warn};

use super::address::{self, Host};
use super::email_server::{banner_version, nothing_answered, service_samples};
use super::ports::{self, ServiceSpec};
use super::xmlapi::map_transport;

/// Port HTTPS de l'interface web de SecurityGateway.
const DEFAULT_API_PORT: u16 = 4443;

/// Racine de l'API REST.
const API_ROOT: &str = "/api/v1";

/// Nombre maximal de compteurs repris : au-delà, c'est que `data` contient
/// autre chose que des compteurs, et la cardinalité doit rester bornée.
const MAX_COUNTERS: usize = 64;

/// Taille maximale d'une réponse.
const MAX_BODY: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(super) struct Options {
    pub host: Host,
    pub services: Vec<ServiceSpec>,
    pub request_timeout: Duration,
    pub insecure_tls: bool,
    /// `https://sg.lan:4443`, sans la racine de l'API.
    pub origin: String,
    pub counters: bool,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let host = address::parse_host(&target.address)?;
        let api_port = match address::tag(target, "api_port") {
            Some(raw) => address::parse_port(raw)?,
            None => host.port.unwrap_or(DEFAULT_API_PORT),
        };
        let api_tls = address::parse_bool_or(address::tag(target, "api_tls"), true)?;
        let scheme = if api_tls { "https" } else { "http" };
        Ok(Self {
            origin: format!("{scheme}://{}:{api_port}", host.for_url()),
            services: ports::parse_services(
                address::tag(target, "services"),
                ports::SECURITY_GATEWAY_SERVICES,
                ports::SECURITY_GATEWAY_DEFAULT,
            )?,
            request_timeout: address::parse_timeout(address::tag(
                target,
                "request_timeout_seconds",
            ))?,
            insecure_tls: address::parse_bool_or(address::tag(target, "insecure_tls"), false)?,
            counters: address::parse_bool_or(address::tag(target, "counters"), true)?,
            host,
        })
    }
}

/// Client de l'API REST. Ne dérive pas `Debug` : il porte la clé.
struct RestClient {
    http: reqwest::Client,
    origin: String,
    key: String,
    timeout: Duration,
}

impl RestClient {
    /// `GET` d'un chemin absolu (`/api/v1/…`). `Ok(None)` sur un 403 quand
    /// `forbidden_is_none` : un droit qui manque n'est pas une panne.
    async fn get(&self, path: &str, forbidden_is_none: bool) -> Result<Option<Value>, ProbeError> {
        let response = self
            .http
            .get(format!("{}{path}", self.origin))
            .timeout(self.timeout)
            .bearer_auth(&self.key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| map_transport(&error, "the SecurityGateway API", self.timeout))?;
        let status = response.status();
        if status == StatusCode::FORBIDDEN && forbidden_is_none {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(status_error(status, path));
        }
        let body = response
            .bytes()
            .await
            .map_err(|error| map_transport(&error, "the SecurityGateway API", self.timeout))?;
        if body.len() > MAX_BODY {
            return Err(ProbeError::Protocol(format!("{path}: answer unexpectedly large")));
        }
        serde_json::from_slice(&body).map(Some).map_err(|_| {
            ProbeError::Protocol(format!(
                "{path} did not answer JSON: is this the SecurityGateway web port?"
            ))
        })
    }
}

fn status_error(status: StatusCode, path: &str) -> ProbeError {
    match status {
        StatusCode::UNAUTHORIZED => ProbeError::Auth(
            "SecurityGateway refused the API key: it is wrong, disabled, expired or deleted"
                .to_string(),
        ),
        StatusCode::FORBIDDEN => ProbeError::Auth(format!(
            "SecurityGateway refused {path}: the account that owns the key lacks the right, \
             or the administrator IP restrictions do not allow the address DumbMonit \
             connects from"
        )),
        StatusCode::NOT_FOUND => ProbeError::Config(format!(
            "No REST API at {path}: it needs SecurityGateway 12.5 or later, served on the \
             web interface port (api_port). For an older version, remove the API key and \
             watch the services only"
        )),
        s if s.is_server_error() => {
            ProbeError::Unreachable(format!("SecurityGateway API {path}: HTTP {}", s.as_u16()))
        }
        s => ProbeError::Protocol(format!("SecurityGateway API {path}: HTTP {}", s.as_u16())),
    }
}

/// Ce que la spécification OpenAPI apprend.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Spec {
    /// `info.version`.
    pub version: Option<String>,
    /// Chemin absolu des compteurs, s'il y en a un.
    pub counters_path: Option<String>,
}

/// Lit la spécification : version, et chemin des compteurs de performance —
/// le plus court des chemins `GET` sans paramètre dont le nom parle de
/// compteurs.
pub(super) fn read_spec(spec: &Value) -> Spec {
    let version = spec.pointer("/info/version").and_then(Value::as_str).map(str::to_string);
    // Les chemins sont relatifs au premier serveur déclaré, s'il est relatif.
    let server = spec
        .pointer("/servers/0/url")
        .and_then(Value::as_str)
        .filter(|url| url.starts_with('/'))
        .unwrap_or(API_ROOT)
        .trim_end_matches('/');
    let counters_path = spec
        .get("paths")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(path, item)| {
            let lower = path.to_ascii_lowercase();
            lower.contains("counter") && !path.contains('{') && item.get("get").is_some()
        })
        .map(|(path, _)| path.as_str())
        .min_by_key(|path| (path.len(), *path))
        .map(|path| {
            if path.starts_with(API_ROOT) || path.starts_with(server) {
                path.to_string()
            } else {
                format!("{server}{path}")
            }
        });
    Spec { version, counters_path }
}

/// `deliveryQueue` → `delivery_queue`, `Queued for Delivery` → `queued_for_delivery`.
pub(super) fn snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut previous_lower = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && previous_lower && !out.ends_with('_') {
                out.push('_');
            }
            previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
            out.push(c.to_ascii_lowercase());
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            previous_lower = false;
        }
    }
    out.trim_end_matches('_').to_string()
}

/// Aplatit `data` en couples (nom, valeur). Un nombre, un booléen ou une chaîne
/// qui est un nombre deviennent une valeur ; un tableau d'objets `name`/`value`
/// donne un compteur par objet ; le reste est ignoré.
pub(super) fn flatten_counters(data: &Value) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    walk(data, String::new(), &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|a, b| a.0 == b.0);
    out.truncate(MAX_COUNTERS);
    out
}

fn join(prefix: &str, key: &str) -> String {
    let key = snake(key);
    match (prefix.is_empty(), key.is_empty()) {
        (_, true) => prefix.to_string(),
        (true, false) => key,
        (false, false) => format!("{prefix}_{key}"),
    }
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn walk(value: &Value, prefix: String, out: &mut Vec<(String, f64)>) {
    if out.len() >= MAX_COUNTERS * 2 {
        return;
    }
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                walk(child, join(&prefix, key), out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let named = item.as_object().and_then(|object| {
                    let name = ["name", "counter", "key", "id"]
                        .iter()
                        .find_map(|k| object.get(*k).and_then(Value::as_str))?;
                    let value = object.get("value").and_then(number)?;
                    Some((name, value))
                });
                match named {
                    Some((name, value)) => {
                        let name = join(&prefix, name);
                        if !name.is_empty() {
                            out.push((name, value));
                        }
                    }
                    None => walk(item, join(&prefix, &index.to_string()), out),
                }
            }
        }
        other => {
            if let Some(v) = number(other).filter(|_| !prefix.is_empty()) {
                out.push((prefix, v));
            }
        }
    }
}

/// Dépouille l'enveloppe `{"success": …, "data": …}`.
fn data_of(body: Value, path: &str) -> Result<Value, ProbeError> {
    match body.get("success").and_then(Value::as_bool) {
        Some(false) => {
            let message = body.get("error").and_then(Value::as_str).unwrap_or("no message");
            let shown: String = message.chars().take(160).collect();
            Err(ProbeError::Protocol(format!("{path}: SecurityGateway answered an error: {shown}")))
        }
        _ => Ok(body.get("data").cloned().unwrap_or(body)),
    }
}

/// Ce que l'API a donné.
#[derive(Default)]
struct ApiReading {
    seconds: f64,
    version: Option<String>,
    /// `None` quand les compteurs n'ont pas été lus (option, droit, chemin absent).
    counters: Option<Vec<(String, f64)>>,
}

async fn read_api(client: &RestClient, counters: bool) -> Result<ApiReading, ProbeError> {
    let started = Instant::now();
    let openapi = format!("{API_ROOT}/openapi");
    let spec = client.get(&openapi, false).await?.map(|v| read_spec(&v)).unwrap_or_default();
    let mut reading = ApiReading { version: spec.version, ..ApiReading::default() };
    if counters {
        match &spec.counters_path {
            Some(path) => match client.get(path, true).await? {
                Some(body) => reading.counters = Some(flatten_counters(&data_of(body, path)?)),
                None => debug!(path, "compteurs refusés au compte de la clé : ignorés"),
            },
            None => debug!("aucun chemin de compteurs dans la spécification OpenAPI"),
        }
    }
    reading.seconds = started.elapsed().as_secs_f64();
    Ok(reading)
}

#[derive(Default)]
pub struct SecurityGatewayCollector;

impl SecurityGatewayCollector {
    pub fn new() -> Self {
        Self
    }

    fn api_client(
        &self,
        target: &Target,
        options: &Options,
    ) -> Result<Option<RestClient>, ProbeError> {
        match &target.credential {
            Credential::None => Ok(None),
            Credential::ApiToken { token } => Ok(Some(RestClient {
                http: crate::http::client(options.insecure_tls)?,
                origin: options.origin.clone(),
                key: token.trim().to_string(),
                timeout: options.request_timeout,
            })),
            other => Err(ProbeError::Config(format!(
                "SecurityGateway expects no credential (services only) or an API key \
                 (SecurityGateway 12.5 and later). Configured credential: {other}"
            ))),
        }
    }
}

#[async_trait]
impl Collector for SecurityGatewayCollector {
    fn kind(&self) -> &'static str {
        "securitygateway"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let client = self.api_client(target, &options)?;
        if options.services.is_empty() && client.is_none() {
            return Err(ProbeError::Config(
                "Nothing to watch: list at least one service, or enter an API key".to_string(),
            ));
        }
        let started = Instant::now();
        let ts_ms = chrono::Utc::now().timestamp_millis();

        let api_call = async {
            match &client {
                Some(client) => Some(read_api(client, options.counters).await),
                None => None,
            }
        };
        let (checks, api) = futures::join!(
            ports::check_all(&options.host.name, &options.services, options.request_timeout),
            api_call
        );

        let mut samples = service_samples("securitygateway", &checks, ts_ms);
        let mut api_answered = false;
        let mut version = None;
        match api {
            None => {}
            Some(Ok(reading)) => {
                api_answered = true;
                samples.push(Sample::new("securitygateway_api_up", 1.0, MetricKind::Gauge, ts_ms));
                samples.push(Sample::new(
                    "securitygateway_api_response_seconds",
                    reading.seconds,
                    MetricKind::Gauge,
                    ts_ms,
                ));
                version = reading.version;
                if options.counters {
                    samples.push(Sample::new(
                        "securitygateway_counters_available",
                        if reading.counters.is_some() { 1.0 } else { 0.0 },
                        MetricKind::Gauge,
                        ts_ms,
                    ));
                }
                for (name, value) in reading.counters.unwrap_or_default() {
                    samples.push(
                        Sample::new("securitygateway_counter", value, MetricKind::Gauge, ts_ms)
                            .with_label("counter", name),
                    );
                }
            }
            Some(Err(error)) if error.means_down() => {
                warn!(target_id = target.id, %error, "API SecurityGateway injoignable");
                samples.push(Sample::new("securitygateway_api_up", 0.0, MetricKind::Gauge, ts_ms));
            }
            Some(Err(error)) => return Err(error),
        }

        if !api_answered && !checks.is_empty() && checks.iter().all(|check| !check.up) {
            return Err(nothing_answered(&checks));
        }
        if !api_answered && checks.is_empty() {
            return Err(ProbeError::Unreachable("the SecurityGateway API does not answer".into()));
        }

        let (version, source) = match version {
            Some(version) => (Some(version), "openapi"),
            None => (banner_version(&checks, "SecurityGateway"), "smtp_banner"),
        };
        if let Some(version) = version {
            samples.push(
                Sample::new("securitygateway_info", 1.0, MetricKind::Gauge, ts_ms)
                    .with_label("version", version)
                    .with_label("source", source),
            );
        }
        samples.push(Sample::new(
            "securitygateway_scrape_duration_seconds",
            started.elapsed().as_secs_f64(),
            MetricKind::Gauge,
            ts_ms,
        ));
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    const OPENAPI: &str = include_str!("testdata/sg-openapi.json");
    const COUNTERS: &str = include_str!("testdata/sg-counters.json");

    fn cible(address: &str, tags: &[(&str, &str)], credential: Credential) -> Target {
        Target {
            id: 11,
            name: "sg".into(),
            address: address.into(),
            kind: "securitygateway".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(SecurityGatewayCollector::new().kind(), "securitygateway");
    }

    #[test]
    fn les_defauts_visent_l_interface_web_en_https() {
        let options = Options::from_target(&cible("sg.lan", &[], Credential::None)).unwrap();
        assert_eq!(options.origin, "https://sg.lan:4443");
        assert!(options.counters);
        let names: Vec<&str> = options.services.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["smtp", "web"]);
    }

    #[test]
    fn la_specification_donne_la_version_et_le_chemin_des_compteurs() {
        let spec = read_spec(&serde_json::from_str(OPENAPI).unwrap());
        assert_eq!(spec.version.as_deref(), Some("12.5.1"));
        assert_eq!(spec.counters_path.as_deref(), Some("/api/v1/performance/counters"));

        // Des chemins déjà absolus, sans `servers` : repris tels quels.
        let absolute = serde_json::json!({
            "paths": { "/api/v1/counters": { "get": {} }, "/api/v1/counters/{id}": { "get": {} } }
        });
        assert_eq!(read_spec(&absolute).counters_path.as_deref(), Some("/api/v1/counters"));
        // Pas de compteurs : rien n'est deviné.
        let none = serde_json::json!({ "paths": { "/users": { "get": {} } } });
        assert_eq!(read_spec(&none), Spec::default());
        // Un chemin de compteurs en écriture seule n'est pas lu.
        let post = serde_json::json!({ "paths": { "/counters/reset": { "post": {} } } });
        assert!(read_spec(&post).counters_path.is_none());
    }

    #[test]
    fn les_noms_passent_en_snake_case() {
        assert_eq!(snake("deliveryQueue"), "delivery_queue");
        assert_eq!(snake("Queued for Delivery"), "queued_for_delivery");
        assert_eq!(snake("activeInboundSMTPSessions"), "active_inbound_smtpsessions");
        assert_eq!(snake("DB pool / max"), "db_pool_max");
        assert_eq!(snake("  "), "");
    }

    #[test]
    fn les_compteurs_sont_repris_sans_supposer_leurs_noms() {
        let body: Value = serde_json::from_str(COUNTERS).unwrap();
        let counters = flatten_counters(&data_of(body, "/api/v1/performance/counters").unwrap());
        let get = |name: &str| counters.iter().find(|(n, _)| n == name).map(|(_, v)| *v);
        assert_eq!(get("delivery_queue"), Some(42.0));
        assert_eq!(get("admin_quarantine"), Some(7.0));
        assert_eq!(get("active_inbound_smtp_sessions"), Some(3.0));
        assert_eq!(get("database_pool_max_size"), Some(50.0));
        assert_eq!(get("counters_bad_messages"), Some(0.0));
        assert_eq!(get("counters_queued_for_delivery"), Some(42.0));
        assert!(get("server_name").is_none(), "le texte n'est pas une mesure");
        assert_eq!(counters.len(), 12);
    }

    #[test]
    fn la_cardinalite_est_bornee() {
        let many: serde_json::Map<String, Value> =
            (0..500).map(|i| (format!("c{i:03}"), Value::from(i))).collect();
        assert_eq!(flatten_counters(&Value::Object(many)).len(), MAX_COUNTERS);
    }

    #[test]
    fn une_enveloppe_en_echec_est_une_erreur_de_protocole() {
        let body = serde_json::json!({ "success": false, "error": "Not allowed", "errorCode": 7 });
        assert!(matches!(data_of(body, "/x"), Err(ProbeError::Protocol(_))));
    }

    #[test]
    fn une_cle_refusee_est_une_erreur_d_authentification_et_pas_une_panne() {
        let error = status_error(StatusCode::UNAUTHORIZED, "/api/v1/openapi");
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
        assert!(matches!(
            status_error(StatusCode::NOT_FOUND, "/api/v1/openapi"),
            ProbeError::Config(_)
        ));
        assert!(status_error(StatusCode::BAD_GATEWAY, "/api/v1/openapi").means_down());
    }

    #[test]
    fn un_identifiant_inadapte_est_refuse_sans_divulguer_le_secret() {
        let target = cible(
            "sg.lan",
            &[],
            Credential::UsernamePassword {
                username: "dumbmonit".into(),
                password: "SECRET".into(),
            },
        );
        let options = Options::from_target(&target).unwrap();
        let error = SecurityGatewayCollector::new().api_client(&target, &options).err().unwrap();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.to_string().contains("SECRET"));
    }

    /// Une fausse API : la spécification, puis les compteurs, derrière la clé
    /// `sg-KEY`.
    async fn fausse_api() -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut chunk = [0u8; 2048];
                    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                        match socket.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => request.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let text = String::from_utf8_lossy(&request).to_string();
                    let authorized = text.to_lowercase().contains("authorization: bearer sg-key");
                    let (status, body) = if !authorized {
                        (
                            "401 Unauthorized",
                            r#"{"error":"Unauthorized","errorCode":401,"statusCode":401}"#,
                        )
                    } else if text.starts_with("GET /api/v1/openapi ") {
                        ("200 OK", OPENAPI)
                    } else if text.starts_with("GET /api/v1/performance/counters ") {
                        ("200 OK", COUNTERS)
                    } else {
                        ("404 Not Found", "{}")
                    };
                    let reply = format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(reply.as_bytes()).await;
                });
            }
        });
        port
    }

    #[tokio::test]
    async fn une_interrogation_complete_lit_la_specification_et_les_compteurs() {
        let port = fausse_api().await.to_string();
        let target = cible(
            "127.0.0.1",
            &[("services", "none"), ("api_port", &port), ("api_tls", "false")],
            Credential::ApiToken { token: "sg-KEY".into() },
        );
        let samples = SecurityGatewayCollector::new().probe(&target).await.unwrap();
        let valeur = |metric: &str| samples.iter().find(|s| s.metric == metric).map(|s| s.value);
        assert_eq!(valeur("securitygateway_api_up"), Some(1.0));
        assert_eq!(valeur("securitygateway_counters_available"), Some(1.0));
        let queue = samples
            .iter()
            .find(|s| {
                s.metric == "securitygateway_counter" && s.labels["counter"] == "delivery_queue"
            })
            .unwrap();
        assert_eq!(queue.value, 42.0);
        let info = samples.iter().find(|s| s.metric == "securitygateway_info").unwrap();
        assert_eq!(info.labels["version"], "12.5.1");
    }

    #[tokio::test]
    async fn une_cle_refusee_ne_passe_pas_pour_une_panne() {
        let port = fausse_api().await.to_string();
        let target = cible(
            "127.0.0.1",
            &[("services", "none"), ("api_port", &port), ("api_tls", "false")],
            Credential::ApiToken { token: "mauvaise".into() },
        );
        let error = SecurityGatewayCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }
}
