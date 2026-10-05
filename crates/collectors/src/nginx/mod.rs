//! Nginx : le module `stub_status` livré avec le serveur, et l'API NGINX Plus
//! quand elle est disponible.
//!
//! `stub_status` ne sert pas de format d'exposition structuré — ni JSON, ni le
//! texte Prometheus que lisent les autres serveurs de ce dossier — mais six
//! lignes de texte fixes :
//!
//! ```text
//! Active connections: 291
//! server accepts handled requests
//!  16630948 16630946 31070465
//! Reading: 6 Writing: 179 Waiting: 106
//! ```
//!
//! `parse_stub_status` lit ce format ligne à ligne. L'API NGINX Plus
//! (`/api/<version>/http/upstreams` et `/api/<version>/http/server_zones`),
//! elle, répond en JSON ; elle n'existe que sur la version commerciale et son
//! absence (404) n'est pas une erreur — seul `stub_status` est garanti.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de la page de statut. |
//! | `port` | `80` | Port du serveur. |
//! | `status_path` | `/basic_status` | Chemin de `stub_status`. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |
//! | `plus_api` | `false` | Lit en plus l'API NGINX Plus (zones et upstreams). |
//! | `plus_api_version` | `9` | Version de l'API NGINX Plus à appeler. |

mod plus;
mod status;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;

pub use plus::{ServerZone, Upstream, UpstreamPeer};
pub use status::StubStatus;

pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_STATUS_PATH: &str = "/basic_status";
pub const DEFAULT_PLUS_API_VERSION: u32 = 9;

#[derive(Default)]
pub struct NginxCollector;

impl NginxCollector {
    pub fn new() -> Self {
        Self
    }
}

struct NginxOptions {
    http: Options,
    status_path: String,
    plus_api: bool,
    plus_api_version: u32,
}

impl NginxOptions {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let http = Options::from_target(target, DEFAULT_PORT)?;
        let status_path = match tag(target, "status_path") {
            None => DEFAULT_STATUS_PATH.to_string(),
            Some(raw) if raw.starts_with('/') => raw.to_string(),
            Some(raw) => {
                return Err(ProbeError::Config(format!(
                    "\"status_path\" must start with \"/\", got \"{raw}\""
                )));
            }
        };
        let plus_api = parse_bool(tag(target, "plus_api"))?;
        let plus_api_version = match tag(target, "plus_api_version") {
            None => DEFAULT_PLUS_API_VERSION,
            Some(raw) => raw.parse().ok().filter(|v| *v > 0).ok_or_else(|| {
                ProbeError::Config(format!("Invalid \"plus_api_version\": \"{raw}\""))
            })?,
        };
        Ok(Self { http, status_path, plus_api, plus_api_version })
    }
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn parse_bool(value: Option<&str>) -> Result<bool, ProbeError> {
    match value {
        None => Ok(false),
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "oui" | "on" => Ok(true),
            "false" | "0" | "no" | "non" | "off" => Ok(false),
            other => Err(ProbeError::Config(format!(
                "Expected a boolean value (true/false), got \"{other}\""
            ))),
        },
    }
}

fn client(target: &Target, options: &Options) -> Result<HttpClient, ProbeError> {
    let auth = Auth::from_credential(&target.credential)?;
    let http = crate::http::client(options.insecure_tls)?;
    Ok(HttpClient::new(http, options.base_url.clone(), auth, options.request_timeout, "Nginx"))
}

#[async_trait]
impl Collector for NginxCollector {
    fn kind(&self) -> &'static str {
        "nginx"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = NginxOptions::from_target(target)?;
        let client = client(target, &options.http)?;

        let reply = client.get_raw(&options.status_path).await?;
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let mut samples = Vec::new();
        if let Some(server) = reply.server_header.as_deref() {
            samples.push(status::server_header_sample(server, ts_ms));
        }
        if !reply.status.is_success() {
            return Err(not_found_hint(&client, reply.status, &reply.body, &options.status_path));
        }
        let stub = status::parse_stub_status(&reply.body).map_err(|detail| {
            ProbeError::Protocol(format!(
                "{} does not look like Nginx's stub_status: {detail}",
                client.url(&options.status_path)
            ))
        })?;
        samples.extend(status::samples(&stub, ts_ms));

        if options.plus_api {
            samples.extend(plus::read(&client, options.plus_api_version, ts_ms).await?);
        }
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let options = NginxOptions::from_target(target)?;
        let client = client(target, &options.http)?;
        let reply = client.get_raw(&options.status_path).await?;
        if !reply.status.is_success() {
            return Err(not_found_hint(&client, reply.status, &reply.body, &options.status_path));
        }
        status::parse_stub_status(&reply.body).map_err(ProbeError::Protocol)?;
        Ok(Some("nginx".to_string()))
    }
}

/// `stub_status` est le seul point d'accès que ce collecteur appelle : un 404
/// dit presque toujours qu'il n'a pas été activé, pas qu'il a été déplacé.
fn not_found_hint(
    client: &HttpClient,
    status: reqwest::StatusCode,
    body: &str,
    path: &str,
) -> ProbeError {
    if status == reqwest::StatusCode::NOT_FOUND {
        return ProbeError::Protocol(format!(
            "Nginx has no {path} at {}: enable \"stub_status;\" in a location of the server \
             block, or check the \"status_path\" option.",
            client.url(path)
        ));
    }
    client.status_error(status, body, path)
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, header};
    use axum::response::IntoResponse;
    use axum::routing::get as route_get;
    use dumbmonit_proto::Credential;

    use super::*;

    const STUB: &str = "Active connections: 291 \nserver accepts handled requests\n \
                         16630948 16630946 31070465 \nReading: 6 Writing: 179 Waiting: 106 \n";

    fn cible(address: &str, tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "nginx".into(),
            address: address.into(),
            kind: "nginx".into(),
            profile_id: None,
            parent_id: None,
            interval: std::time::Duration::from_secs(60),
            enabled: true,
            tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            credential: Credential::None,
        }
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        address.to_string()
    }

    fn find<'a>(samples: &'a [Sample], metric: &str) -> Option<&'a Sample> {
        samples.iter().find(|s| s.metric == metric)
    }

    #[tokio::test]
    async fn une_interrogation_complete_lit_les_connexions_et_le_serveur() {
        let app = Router::new().route(
            DEFAULT_STATUS_PATH,
            route_get(|| async {
                let mut headers = HeaderMap::new();
                headers.insert(header::SERVER, "nginx/1.25.3".parse().unwrap());
                (headers, STUB)
            }),
        );
        let address = serve(app).await;
        let samples = NginxCollector::new().probe(&cible(&address, &[])).await.unwrap();

        assert_eq!(find(&samples, "nginx_connections_active").unwrap().value, 291.0);
        assert_eq!(find(&samples, "nginx_connections_reading").unwrap().value, 6.0);
        assert_eq!(find(&samples, "nginx_connections_writing").unwrap().value, 179.0);
        assert_eq!(find(&samples, "nginx_connections_waiting").unwrap().value, 106.0);
        assert_eq!(find(&samples, "nginx_accepts_total").unwrap().value, 16_630_948.0);
        assert_eq!(find(&samples, "nginx_handled_total").unwrap().value, 16_630_946.0);
        assert_eq!(find(&samples, "nginx_requests_total").unwrap().value, 31_070_465.0);
        let version = find(&samples, "nginx_version_info").unwrap();
        assert_eq!(version.labels.get("server").map(String::as_str), Some("nginx/1.25.3"));
    }

    #[tokio::test]
    async fn un_statut_absent_explique_comment_lactiver() {
        let app = Router::new().route(
            DEFAULT_STATUS_PATH,
            route_get(|| async { StatusCode::NOT_FOUND.into_response() }),
        );
        let address = serve(app).await;
        let error = NginxCollector::new().probe(&cible(&address, &[])).await.unwrap_err();
        assert!(error.to_string().contains("stub_status"), "{error}");
    }

    #[tokio::test]
    async fn un_chemin_sans_barre_oblique_est_refuse() {
        let error = NginxCollector::new()
            .probe(&cible("nginx.lan", &[("status_path", "basic_status")]))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(NginxCollector::new().kind(), "nginx");
    }
}
