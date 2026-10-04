//! Home Assistant, par son API REST et, pour les réparations, son API WebSocket.
//!
//! Un jeton d'accès longue durée d'un compte **non administrateur** suffit :
//! `/api/config` et `/api/states` sont lisibles par tout compte, et la liste
//! des réparations aussi. Rien n'est jamais appelé qui change un état : aucun
//! service, aucun événement, aucune écriture.
//!
//! La panne que ce module existe pour voir n'est pas « Home Assistant ne répond
//! plus » (la sonde HTTP le voit déjà) mais « Home Assistant répond et quelque
//! chose, derrière, ne marche plus » : une intégration dont toutes les entités
//! passent `unavailable`, une pile de capteur à 5 %, une réparation qui attend.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | `https` derrière un proxy inverse ou avec TLS activé. |
//! | `port` | `8123` | Port de Home Assistant. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par appel. |
//! | `battery_threshold` | `20` | Pourcentage sous lequel une pile est faible. |
//! | `exclude_domains` | — | Domaines hors des totaux d'entités indisponibles. |
//! | `repairs` | `true` | Lit les réparations par l'API WebSocket. |
//! | `device_stale_days` | `3` | Jours sans activité au-delà desquels un appareil de l'app compagnon est signalé périmé. |
//!
//! # Appareils de l'app compagnon (`mobile_app`)
//!
//! Chaque téléphone ou tablette qui a installé l'app Home Assistant crée un
//! `device_tracker` ; sa dernière activité nourrit le tableau générique des
//! appareils clients (`client_devices.rs`). L'identification se fait sans le
//! registre d'entités, que l'API REST ne sert pas : voir le commentaire de
//! [`metrics::mobile_app_devices`]. Facultatif de nature — une installation
//! sans l'app compagnon ne publie simplement aucun appareil, sans erreur.

pub mod metrics;
pub mod websocket;

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::api_options::{Connection, parse_bool, parse_list, parse_u64_in, tag};
use crate::client_devices;
use metrics::{Config, State, Tuning};

pub const DEFAULT_PORT: u16 = 8123;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_BATTERY_THRESHOLD: u64 = 20;

#[derive(Debug, Clone)]
struct Settings {
    connection: Connection,
    token: String,
    tuning: Tuning,
    repairs: bool,
}

impl Settings {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let token = match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => token.trim().to_string(),
            other => {
                return Err(ProbeError::Config(format!(
                    "Home Assistant expects a long-lived access token, configured: {other}"
                )));
            }
        };
        let mut target = target.clone();
        // L'adresse copiée depuis le navigateur finit souvent par `/api` ou `/lovelace/…`.
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/api").unwrap_or(trimmed).to_string();
        Ok(Self {
            connection: Connection::from_target(
                &target,
                "http",
                DEFAULT_PORT,
                DEFAULT_PORT,
                DEFAULT_REQUEST_TIMEOUT,
            )?,
            token,
            tuning: Tuning {
                battery_threshold: parse_u64_in(
                    tag(&target, "battery_threshold"),
                    DEFAULT_BATTERY_THRESHOLD,
                    1..=100,
                    "Low battery threshold (%)",
                )? as f64,
                exclude_domains: parse_list(tag(&target, "exclude_domains")),
            },
            repairs: parse_bool(tag(&target, "repairs"), true)?,
        })
    }
}

struct Client {
    http: reqwest::Client,
    settings: Settings,
}

impl Client {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ProbeError> {
        let timeout = self.settings.connection.request_timeout;
        let url = format!("{}{path}", self.settings.connection.base_url);
        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.settings.token)
            .timeout(timeout)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ProbeError::Timeout(timeout)
                } else {
                    ProbeError::Unreachable(format!("{path}: {error}"))
                }
            })?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| ProbeError::Unreachable(format!("{path}: {error}")))?;
        match status {
            status if status.is_success() => serde_json::from_str(&body).map_err(|error| {
                ProbeError::Protocol(format!("Unexpected answer from {path}: {error}"))
            }),
            StatusCode::UNAUTHORIZED => Err(ProbeError::Auth(
                "Home Assistant refused the access token (401). Create a new long-lived token \
                 for the dumbmonit user."
                    .to_string(),
            )),
            StatusCode::FORBIDDEN => Err(ProbeError::Auth(format!(
                "Home Assistant refused {path} (403): the address may be banned after failed \
                 logins (ip_bans.yaml)."
            ))),
            StatusCode::NOT_FOUND => Err(ProbeError::Protocol(format!(
                "No Home Assistant API at {url}: check the address and the port (8123)."
            ))),
            status if status.is_server_error() => {
                Err(ProbeError::Unreachable(format!("Home Assistant answered {status} on {path}")))
            }
            status => {
                Err(ProbeError::Protocol(format!("Home Assistant answered {status} on {path}")))
            }
        }
    }
}

#[derive(Default)]
pub struct HomeAssistantCollector;

impl HomeAssistantCollector {
    pub fn new() -> Self {
        Self
    }

    async fn collect(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = Settings::from_target(target)?;
        let stale_days = client_devices::stale_days(target)?;
        let client =
            Client { http: crate::http::client(settings.connection.insecure_tls)?, settings };
        let ts_ms = chrono::Utc::now().timestamp_millis();

        // La preuve de vie et d'authentification : le seul appel qui condamne.
        let config: Config = client.get("/api/config").await?;
        let mut out = metrics::config_samples(&config, ts_ms);
        let mut errors = 0u32;

        let settings = &client.settings;
        let repairs = async {
            if !settings.repairs {
                return None;
            }
            let endpoint = match websocket::Endpoint::from_base_url(&settings.connection.base_url) {
                Ok(endpoint) => endpoint,
                Err(error) => return Some(Err(error)),
            };
            Some(
                websocket::list_issues(
                    &endpoint,
                    &settings.token,
                    settings.connection.insecure_tls,
                    settings.connection.request_timeout,
                )
                .await,
            )
        };
        let (states, repairs) = futures::join!(client.get::<Vec<State>>("/api/states"), repairs);
        match states {
            Ok(states) => {
                out.extend(metrics::state_samples(&states, &settings.tuning, ts_ms));
                let devices = metrics::mobile_app_devices(&states);
                out.extend(client_devices::samples("homeassistant", &devices, stale_days, ts_ms));
            }
            Err(error) => {
                errors += 1;
                warn!(target_id = target.id, %error, "états Home Assistant illisibles");
            }
        }
        match repairs {
            Some(Ok(issues)) => out.extend(metrics::repair_samples(&issues, ts_ms)),
            Some(Err(error)) => {
                errors += 1;
                warn!(target_id = target.id, %error, "réparations Home Assistant illisibles");
            }
            None => {}
        }
        out.push(Sample::new(
            "homeassistant_scrape_errors",
            f64::from(errors),
            MetricKind::Gauge,
            ts_ms,
        ));
        Ok(out)
    }
}

#[async_trait]
impl Collector for HomeAssistantCollector {
    fn kind(&self) -> &'static str {
        "homeassistant"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        self.collect(target).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let settings = Settings::from_target(target)?;
        let client =
            Client { http: crate::http::client(settings.connection.insecure_tls)?, settings };
        let _: Config = client.get("/api/config").await?;
        Ok(Some("homeassistant".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::get;

    use super::*;
    use crate::api_options::test_support::target;

    const TOKEN: &str = "llat-for-tests";

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some(&format!("Bearer {TOKEN}"))
    }

    async fn repairs_socket(mut socket: WebSocket) {
        let _ = socket
            .send(Message::Text(r#"{"type":"auth_required","ha_version":"2026.9.4"}"#.into()))
            .await;
        let Some(Ok(Message::Text(auth))) = socket.recv().await else { return };
        if !auth.contains(TOKEN) {
            let _ = socket.send(Message::Text(r#"{"type":"auth_invalid"}"#.into())).await;
            return;
        }
        let _ = socket
            .send(Message::Text(r#"{"type":"auth_ok","ha_version":"2026.9.4"}"#.into()))
            .await;
        let Some(Ok(Message::Text(command))) = socket.recv().await else { return };
        assert!(command.contains("repairs/list_issues"));
        let result = include_str!("testdata/ha_2026.9.4/repairs_list_issues.json");
        let reply = format!(r#"{{"id":1,"type":"result","success":true,"result":{result}}}"#);
        let _ = socket.send(Message::Text(reply.into())).await;
    }

    async fn serve() -> String {
        let app = Router::new()
            .route(
                "/api/config",
                get(|headers: HeaderMap| async move {
                    if !authorized(&headers) {
                        return (StatusCode::UNAUTHORIZED, "401: Unauthorized").into_response();
                    }
                    include_str!("testdata/ha_2026.9.4/api_config.json").into_response()
                }),
            )
            .route(
                "/api/states",
                get(|headers: HeaderMap| async move {
                    if !authorized(&headers) {
                        return (StatusCode::UNAUTHORIZED, "401: Unauthorized").into_response();
                    }
                    include_str!("testdata/ha_2026.9.4/api_states.json").into_response()
                }),
            )
            .route(
                "/api/websocket",
                get(|ws: WebSocketUpgrade| async move { ws.on_upgrade(repairs_socket) }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn value(samples: &[Sample], metric: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == metric).map(|s| s.value)
    }

    #[tokio::test]
    async fn la_collecte_complete_lit_config_etats_et_reparations() {
        let base = serve().await;
        let samples = HomeAssistantCollector::new()
            .probe(&target(
                "homeassistant",
                &format!("{base}/api/"),
                &[],
                Credential::ApiToken { token: TOKEN.into() },
            ))
            .await
            .unwrap();
        assert_eq!(value(&samples, "homeassistant_running"), Some(1.0));
        assert_eq!(value(&samples, "homeassistant_batteries_low"), Some(3.0));
        assert_eq!(value(&samples, "homeassistant_updates_available"), Some(5.0));
        assert!(samples.iter().any(|s| s.metric == "homeassistant_repair"));
        assert_eq!(value(&samples, "homeassistant_scrape_errors"), Some(0.0));
        assert!(
            samples.iter().any(|s| s.metric == "client_device_last_seen_timestamp_seconds"
                && s.labels.get("device").map(String::as_str) == Some("Pixel 8")
                && s.labels.get("kind").map(String::as_str) == Some("homeassistant")),
            "l'app compagnon nourrit le tableau générique des appareils"
        );
    }

    #[tokio::test]
    async fn les_reparations_se_desactivent() {
        let base = serve().await;
        let samples = HomeAssistantCollector::new()
            .probe(&target(
                "homeassistant",
                &base,
                &[("repairs", "false")],
                Credential::ApiToken { token: TOKEN.into() },
            ))
            .await
            .unwrap();
        assert!(!samples.iter().any(|s| s.metric == "homeassistant_repairs"));
        assert_eq!(value(&samples, "homeassistant_scrape_errors"), Some(0.0));
    }

    #[tokio::test]
    async fn un_jeton_refuse_n_est_pas_une_panne() {
        let base = serve().await;
        let error = HomeAssistantCollector::new()
            .probe(&target(
                "homeassistant",
                &base,
                &[],
                Credential::ApiToken { token: "faux".into() },
            ))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn une_option_invalide_est_une_erreur_de_configuration() {
        for tags in [
            &[("battery_threshold", "0")][..],
            &[("repairs", "peut-être")],
            &[("port", "x")],
            &[("device_stale_days", "0")],
        ] {
            let error = HomeAssistantCollector::new()
                .probe(&target(
                    "homeassistant",
                    "ha.lan",
                    tags,
                    Credential::ApiToken { token: "t".into() },
                ))
                .await
                .unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{tags:?}");
        }
        let error = HomeAssistantCollector::new()
            .probe(&target("homeassistant", "ha.lan", &[], Credential::None))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
