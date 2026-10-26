//! InfluxDB 2.x et 3.x (Core, Enterprise), par leur API HTTP.
//!
//! Un seul type d'équipement, `influxdb` : la version est reconnue à la forme de
//! la réponse de `/health`.
//!
//! * **v2** : `/health` répond un objet JSON (`status` à `pass` ou `fail`, et la
//!   `version`), `/ready` un objet avec `status: ready` et le temps de marche
//!   (`up`). Aucune des deux ne demande de jeton.
//! * **v3** : `/health` répond `OK` en texte et demande un jeton (porteur) tant
//!   que le serveur n'a pas été lancé avec `--disable-authz health`. La version
//!   vient de `/ping`.
//!
//! Les métriques internes viennent de `/metrics` (format Prometheus), lues au
//! mieux : une instance qui les refuse (jeton insuffisant) garde la santé et
//! l'état de disponibilité, sans les compteurs.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | `https` si l'API est servie en TLS. |
//! | `port` | `8086` | Port de l'API (`8181` pour la v3 par défaut). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use serde_json::Value;

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::observability::prom::Exposition;

pub const DEFAULT_PORT: u16 = 8086;

#[derive(Default)]
pub struct InfluxdbCollector;

impl InfluxdbCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<HttpClient, ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    // Un jeton d'API, porteur ; ou rien, pour une v2 dont `/health` est ouvert.
    let auth = match &target.credential {
        Credential::None => Auth::None,
        Credential::ApiToken { token } if !token.trim().is_empty() => {
            Auth::Bearer(token.trim().to_string())
        }
        other => {
            return Err(ProbeError::Config(format!(
                "InfluxDB expects no credential or an API token, configured: {other}"
            )));
        }
    };
    Ok(HttpClient::new(
        crate::http::client(options.insecure_tls)?,
        options.base_url,
        auth,
        options.request_timeout,
        "InfluxDB",
    ))
}

#[async_trait]
impl Collector for InfluxdbCollector {
    fn kind(&self) -> &'static str {
        "influxdb"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let replies = read(&client(target)?).await?;
        Ok(samples(&replies, chrono::Utc::now().timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read_health(&client(target)?).await?;
        Ok(Some("influxdb".to_string()))
    }
}

/// La génération du serveur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    V2,
    V3,
}

impl Generation {
    fn label(self) -> &'static str {
        match self {
            Self::V2 => "v2",
            Self::V3 => "v3",
        }
    }
}

/// Ce que `/health` (et `/ready`, `/ping`) ont appris.
#[derive(Debug)]
pub struct Health {
    pub generation: Generation,
    pub healthy: bool,
    pub version: String,
    /// Vrai si le serveur se dit prêt à répondre aux requêtes.
    pub ready: bool,
    pub uptime_seconds: Option<f64>,
}

pub struct Replies {
    pub health: Health,
    /// `None` quand `/metrics` n'est pas lisible.
    pub metrics: Option<Exposition>,
}

/// Lit `/health` : une réponse JSON annonce une v2, du texte une v3. Un 503
/// de la v2 (`status: fail`) est une réponse, pas une panne de transport.
pub fn parse_health(status: u16, body: &str) -> Result<Health, ProbeError> {
    if let Ok(json) = serde_json::from_str::<Value>(body)
        && let Some(state) = json.get("status").and_then(Value::as_str)
    {
        let version = json
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim_start_matches('v')
            .to_string();
        let healthy = matches!(state, "pass" | "ok" | "OK");
        return Ok(Health {
            generation: Generation::V2,
            healthy,
            version,
            ready: healthy,
            uptime_seconds: None,
        });
    }
    if (200..300).contains(&status) {
        return Ok(Health {
            generation: Generation::V3,
            healthy: true,
            version: String::new(),
            ready: true,
            uptime_seconds: None,
        });
    }
    Err(ProbeError::Protocol(format!(
        "/health answered {status} without a status: this does not look like InfluxDB. Check the \
         device type and the port."
    )))
}

/// `1h2m3.5s`, `45s`, `2d3h` : la durée que `/ready` de la v2 annonce.
pub fn parse_go_duration(text: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut number = String::new();
    let mut rest = text.trim().chars().peekable();
    let mut seen = false;
    while let Some(c) = rest.next() {
        if c.is_ascii_digit() || c == '.' {
            number.push(c);
            continue;
        }
        let value: f64 = number.parse().ok()?;
        number.clear();
        let unit = match c {
            'h' => 3600.0,
            's' => 1.0,
            'd' => 86_400.0,
            'm' => {
                if rest.peek() == Some(&'s') {
                    rest.next();
                    total += value / 1000.0;
                    seen = true;
                    continue;
                }
                60.0
            }
            _ => return None,
        };
        total += value * unit;
        seen = true;
    }
    (seen && number.is_empty()).then_some(total)
}

async fn read_health(client: &HttpClient) -> Result<Health, ProbeError> {
    let reply = client.get_raw("/health").await?;
    match reply.status.as_u16() {
        401 | 403 | 404 => return Err(client.status_error(reply.status, &reply.body, "/health")),
        _ => {}
    }
    let mut health = parse_health(reply.status.as_u16(), &reply.body)?;
    match health.generation {
        Generation::V2 => {
            // `/ready` donne le temps de marche ; son absence n'est pas une erreur.
            if let Ok(reply) = client.get_raw("/ready").await
                && let Ok(json) = serde_json::from_str::<Value>(&reply.body)
            {
                health.ready = json.get("status").and_then(Value::as_str) == Some("ready");
                health.uptime_seconds =
                    json.get("up").and_then(Value::as_str).and_then(parse_go_duration);
            }
        }
        Generation::V3 => {
            if let Ok(reply) = client.get_raw("/ping").await
                && reply.status.is_success()
                && let Ok(json) = serde_json::from_str::<Value>(&reply.body)
            {
                health.version =
                    json.get("version").and_then(Value::as_str).unwrap_or("").to_string();
            }
        }
    }
    Ok(health)
}

pub async fn read(client: &HttpClient) -> Result<Replies, ProbeError> {
    let health = read_health(client).await?;
    let metrics = match client.get_raw("/metrics").await {
        Ok(reply) if reply.status.is_success() => Some(Exposition::parse(&reply.body)),
        // Jeton sans droit sur `/metrics`, ou métriques désactivées : au mieux.
        Ok(_) => None,
        Err(error) if error.means_down() => return Err(error),
        Err(_) => None,
    };
    Ok(Replies { health, metrics })
}

pub fn samples(replies: &Replies, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let counter = |name: &str, value: f64| Sample::new(name, value, MetricKind::Counter, ts_ms);
    let health = &replies.health;

    out.push(
        gauge("influxdb_version_info", 1.0)
            .with_label("version", health.version.clone())
            .with_label("generation", health.generation.label()),
    );
    out.push(gauge("influxdb_healthy", if health.healthy { 1.0 } else { 0.0 }));
    out.push(gauge("influxdb_ready", if health.ready { 1.0 } else { 0.0 }));
    if let Some(up) = health.uptime_seconds {
        out.push(gauge("influxdb_uptime_seconds", up));
    }

    let Some(metrics) = &replies.metrics else { return out };
    for (name, source) in [
        ("influxdb_goroutines", "go_goroutines"),
        ("influxdb_memory_resident_bytes", "process_resident_memory_bytes"),
        ("influxdb_open_fds", "process_open_fds"),
    ] {
        if let Some(value) = metrics.sum(source) {
            out.push(gauge(name, value));
        }
    }
    if let (Some(open), Some(max)) = (metrics.sum("process_open_fds"), metrics.sum("process_max_fds"))
        && max > 0.0
    {
        out.push(gauge("influxdb_open_fds_percent", open / max * 100.0));
    }
    if let Some(start) = metrics.sum("process_start_time_seconds")
        && health.uptime_seconds.is_none()
    {
        let now = ts_ms as f64 / 1000.0;
        if now > start {
            out.push(gauge("influxdb_uptime_seconds", now - start));
        }
    }
    // Requêtes HTTP : `http_api_requests_total` (v2) ou `http_requests_total` (v3),
    // le code de réponse dans `status` ou `code`.
    let family = ["http_api_requests_total", "http_requests_total"]
        .into_iter()
        .find(|family| metrics.has(family));
    if let Some(family) = family {
        let status = |line: &crate::observability::prom::Line| {
            line.label("status").or_else(|| line.label("code")).unwrap_or("").to_string()
        };
        if let Some(total) = metrics.sum(family) {
            out.push(counter("influxdb_http_requests_total", total));
        }
        let errors = metrics.sum_where(family, |line| status(line).starts_with('5')).unwrap_or(0.0);
        out.push(counter("influxdb_http_requests_5xx_total", errors));
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    const HEALTH_V2: &str = r#"{"name":"influxdb","message":"ready for queries and writes",
        "status":"pass","checks":[],"version":"v2.7.5","commit":"abc"}"#;
    const HEALTH_V2_FAIL: &str = r#"{"name":"influxdb","message":"disk full","status":"fail"}"#;
    const READY_V2: &str = r#"{"status":"ready","started":"2026-01-01T00:00:00Z","up":"1h2m3.5s"}"#;
    const METRICS: &str = "go_goroutines 42\n\
        process_resident_memory_bytes 1.048576e+08\n\
        process_open_fds 100\n\
        process_max_fds 1000\n\
        http_api_requests_total{handler=\"platform\",status=\"200\"} 90\n\
        http_api_requests_total{handler=\"platform\",status=\"500\"} 10\n";

    fn find<'a>(samples: &'a [Sample], name: &str) -> Option<&'a Sample> {
        samples.iter().find(|s| s.metric == name)
    }

    #[test]
    fn la_duree_go_est_lue() {
        assert_eq!(parse_go_duration("1h2m3.5s"), Some(3723.5));
        assert_eq!(parse_go_duration("45s"), Some(45.0));
        assert_eq!(parse_go_duration("250ms"), Some(0.25));
        assert_eq!(parse_go_duration("2d3h"), Some(183_600.0));
        assert_eq!(parse_go_duration("bientôt"), None);
        assert_eq!(parse_go_duration(""), None);
    }

    #[test]
    fn v2_en_bonne_sante_puis_en_echec() {
        let health = parse_health(200, HEALTH_V2).unwrap();
        assert_eq!((health.generation, health.healthy, health.version.as_str()), (Generation::V2, true, "2.7.5"));
        let health = parse_health(503, HEALTH_V2_FAIL).unwrap();
        assert!(!health.healthy && !health.ready);
    }

    #[test]
    fn v3_repond_du_texte() {
        let health = parse_health(200, "OK").unwrap();
        assert_eq!((health.generation, health.healthy), (Generation::V3, true));
        let error = parse_health(500, "boom").unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)));
    }

    #[test]
    fn les_metriques_internes_sont_reprises() {
        let mut health = parse_health(200, HEALTH_V2).unwrap();
        health.uptime_seconds = Some(3723.5);
        let replies = Replies { health, metrics: Some(Exposition::parse(METRICS)) };
        let samples = samples(&replies, 0);
        assert_eq!(find(&samples, "influxdb_healthy").unwrap().value, 1.0);
        assert_eq!(find(&samples, "influxdb_uptime_seconds").unwrap().value, 3723.5);
        assert_eq!(find(&samples, "influxdb_open_fds_percent").unwrap().value, 10.0);
        assert_eq!(find(&samples, "influxdb_http_requests_total").unwrap().value, 100.0);
        let errors = find(&samples, "influxdb_http_requests_5xx_total").unwrap();
        assert_eq!((errors.value, errors.kind), (10.0, MetricKind::Counter));
    }

    #[test]
    fn sans_metriques_la_sante_reste() {
        let replies = Replies { health: parse_health(200, "OK").unwrap(), metrics: None };
        let samples = samples(&replies, 0);
        assert_eq!(samples.len(), 3);
        assert_eq!(find(&samples, "influxdb_ready").unwrap().value, 1.0);
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    #[tokio::test]
    async fn interrogation_v2() {
        let app = Router::new()
            .route("/health", get(|| async { HEALTH_V2 }))
            .route("/ready", get(|| async { READY_V2 }))
            .route("/metrics", get(|| async { METRICS }));
        let target = cible("influxdb", &serve(app).await, &[]);
        let samples = InfluxdbCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "influxdb_uptime_seconds").unwrap().value, 3723.5);
        assert_eq!(find(&samples, "influxdb_goroutines").unwrap().value, 42.0);
    }

    #[tokio::test]
    async fn v3_demande_un_jeton() {
        let app = Router::new().route(
            "/health",
            get(|headers: HeaderMap| async move {
                if headers.get("authorization").and_then(|v| v.to_str().ok())
                    == Some("Bearer apiv3_secret")
                {
                    (StatusCode::OK, "OK")
                } else {
                    (StatusCode::UNAUTHORIZED, "")
                }
            }),
        );
        let base = serve(app).await;
        let mut target = cible("influxdb", &base, &[]);
        let error = InfluxdbCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)) && !error.means_down(), "{error}");
        target.credential = Credential::ApiToken { token: "apiv3_secret".into() };
        let samples = InfluxdbCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "influxdb_healthy").unwrap().value, 1.0);
    }

    #[tokio::test]
    async fn un_identifiant_de_mauvais_type_est_une_erreur_de_configuration() {
        let mut target = cible("influxdb", "127.0.0.1:1", &[]);
        target.credential =
            Credential::UsernamePassword { username: "u".into(), password: "p".into() };
        let error = InfluxdbCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn un_port_ferme_est_une_panne() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let target = cible("influxdb", &format!("127.0.0.1:{port}"), &[]);
        let error = InfluxdbCollector::new().probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }
}
