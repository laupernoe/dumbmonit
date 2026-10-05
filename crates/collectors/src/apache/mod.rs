//! Apache httpd : `mod_status`, interrogé avec `?auto` pour obtenir des lignes
//! `Clé: valeur` plutôt que la page HTML destinée à un navigateur.
//!
//! Deux niveaux de détail, selon `ExtendedStatus` dans la configuration
//! d'Apache : `BusyWorkers`/`IdleWorkers` et le tableau de bord sont toujours
//! là, les compteurs cumulatifs (requêtes, octets, débits) n'apparaissent
//! qu'avec `ExtendedStatus On`. Ce collecteur accepte les deux : voir
//! [`status`] pour le détail du format et des champs optionnels.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de la page de statut. |
//! | `port` | `80` | Port du serveur. |
//! | `status_path` | `/server-status` | Chemin de `mod_status` (sans `?auto`, ajouté automatiquement). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

mod status;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;

pub use status::ModStatus;

pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_STATUS_PATH: &str = "/server-status";

#[derive(Default)]
pub struct ApacheCollector;

impl ApacheCollector {
    pub fn new() -> Self {
        Self
    }
}

struct ApacheOptions {
    http: Options,
    status_path: String,
}

impl ApacheOptions {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let http = Options::from_target(target, DEFAULT_PORT)?;
        let status_path = match tag(target, "status_path") {
            None => DEFAULT_STATUS_PATH.to_string(),
            Some(raw) if raw.starts_with('/') => raw.trim_end_matches('/').to_string(),
            Some(raw) => {
                return Err(ProbeError::Config(format!(
                    "\"status_path\" must start with \"/\", got \"{raw}\""
                )));
            }
        };
        Ok(Self { http, status_path })
    }

    /// Le chemin interrogé, requête `auto` ajoutée : c'est elle qui fait
    /// répondre `mod_status` en texte plutôt qu'en HTML.
    fn auto_path(&self) -> String {
        format!("{}?auto", self.status_path)
    }
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn client(target: &Target, options: &Options) -> Result<HttpClient, ProbeError> {
    let auth = Auth::from_credential(&target.credential)?;
    let http = crate::http::client(options.insecure_tls)?;
    Ok(HttpClient::new(http, options.base_url.clone(), auth, options.request_timeout, "Apache"))
}

#[async_trait]
impl Collector for ApacheCollector {
    fn kind(&self) -> &'static str {
        "apache"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = ApacheOptions::from_target(target)?;
        let client = client(target, &options.http)?;

        let reply = client.get_raw(&options.auto_path()).await?;
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let mut samples = Vec::new();
        if let Some(server) = reply.server_header.as_deref() {
            samples.push(status::server_header_sample(server, ts_ms));
        }
        if !reply.status.is_success() {
            return Err(not_found_hint(&client, reply.status, &reply.body, &options.status_path));
        }
        let parsed = status::parse_mod_status(&reply.body).map_err(|detail| {
            ProbeError::Protocol(format!(
                "{} does not look like Apache's mod_status: {detail}",
                client.url(&options.status_path)
            ))
        })?;
        samples.extend(status::samples(&parsed, ts_ms));
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let options = ApacheOptions::from_target(target)?;
        let client = client(target, &options.http)?;
        let reply = client.get_raw(&options.auto_path()).await?;
        if !reply.status.is_success() {
            return Err(not_found_hint(&client, reply.status, &reply.body, &options.status_path));
        }
        status::parse_mod_status(&reply.body).map_err(ProbeError::Protocol)?;
        Ok(Some("apache".to_string()))
    }
}

/// `mod_status` est le seul point d'accès que ce collecteur appelle : un 404
/// dit presque toujours qu'il n'a pas été activé, pas qu'il a été déplacé.
fn not_found_hint(
    client: &HttpClient,
    status: reqwest::StatusCode,
    body: &str,
    path: &str,
) -> ProbeError {
    if status == reqwest::StatusCode::NOT_FOUND {
        return ProbeError::Protocol(format!(
            "Apache has no {path} at {}: enable mod_status and add a \"SetHandler \
             server-status\" location, or check the \"status_path\" option.",
            client.url(path)
        ));
    }
    client.status_error(status, body, path)
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::Query;
    use axum::http::{HeaderMap, StatusCode, header};
    use axum::response::IntoResponse;
    use axum::routing::get as route_get;
    use dumbmonit_proto::Credential;
    use std::collections::HashMap;

    use super::*;

    const EXTENDED: &str = "Total Accesses: 46499\nTotal kBytes: 332255\nCPULoad: .1\n\
                             Uptime: 1575804\nReqPerSec: .0295\nBytesPerSec: 215.906\n\
                             BytesPerReq: 7317.43\nBusyWorkers: 2\nIdleWorkers: 8\n\
                             Scoreboard: ____W_________________..........\n";

    fn cible(address: &str, tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "apache".into(),
            address: address.into(),
            kind: "apache".into(),
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
    async fn une_interrogation_complete_exige_la_requete_auto() {
        let app = Router::new().route(
            DEFAULT_STATUS_PATH,
            route_get(|Query(params): Query<HashMap<String, String>>| async move {
                if params.contains_key("auto") {
                    let mut headers = HeaderMap::new();
                    headers.insert(header::SERVER, "Apache/2.4.58 (Unix)".parse().unwrap());
                    (headers, EXTENDED).into_response()
                } else {
                    StatusCode::OK.into_response()
                }
            }),
        );
        let address = serve(app).await;
        let samples = ApacheCollector::new().probe(&cible(&address, &[])).await.unwrap();

        assert_eq!(find(&samples, "apache_workers_busy").unwrap().value, 2.0);
        assert_eq!(find(&samples, "apache_workers_idle").unwrap().value, 8.0);
        assert_eq!(find(&samples, "apache_requests_total").unwrap().value, 46_499.0);
        assert_eq!(find(&samples, "apache_sent_bytes_total").unwrap().value, 332_255.0 * 1024.0);
        let busy = find(&samples, "apache_scoreboard")
            .map(|_| samples.iter().filter(|s| s.metric == "apache_scoreboard").count());
        assert_eq!(busy, Some(status::SCOREBOARD_STATES.len()));
        let version = samples.iter().find(|s| s.metric == "apache_version_info").unwrap();
        assert_eq!(version.labels.get("version").map(String::as_str), Some("Apache/2.4.58 (Unix)"));
    }

    #[tokio::test]
    async fn un_statut_absent_explique_comment_lactiver() {
        let app =
            Router::new().route(DEFAULT_STATUS_PATH, route_get(|| async { StatusCode::NOT_FOUND }));
        let address = serve(app).await;
        let error = ApacheCollector::new().probe(&cible(&address, &[])).await.unwrap_err();
        assert!(error.to_string().contains("mod_status"), "{error}");
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(ApacheCollector::new().kind(), "apache");
    }

    #[test]
    fn le_chemin_de_statut_se_personnalise() {
        let target = cible("apache.lan", &[("status_path", "/status")]);
        let options = ApacheOptions::from_target(&target).unwrap();
        assert_eq!(options.auto_path(), "/status?auto");
    }
}
