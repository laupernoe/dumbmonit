//! Serveurs de journaux et de métriques : VictoriaMetrics, VictoriaLogs, Grafana
//! Loki et Graylog.
//!
//! Surveiller la surveillance. Ces serveurs tombent rarement tout à fait ; ils
//! tombent *à moitié* : ils répondent, l'interface s'ouvre, et ils refusent en
//! silence une partie de ce qu'on leur envoie — disque sous le seuil de lecture
//! seule, lignes trop anciennes, limite de débit, OpenSearch qui ne suit plus.
//! Le jour où l'on cherche le journal de l'incident, il n'y est pas. Chaque
//! module lit ce que le serveur dit de lui-même, sans jamais interroger les
//! données qu'il garde : aucune requête LogQL, LogsQL ou MetricsQL, aucune
//! recherche Graylog, aucun message lu.
//!
//! * [`victoria`] — `/health` et `/metrics` de VictoriaMetrics et VictoriaLogs ;
//! * [`loki`] — `/ready` et `/metrics` de Loki ;
//! * [`graylog`] — l'API REST de Graylog, avec un compte au rôle Reader, qui
//!   couvre aussi la santé du cluster OpenSearch ou Elasticsearch derrière lui.
//!
//! # Principes
//!
//! * **Les erreurs sont classées pour l'alerting.** Un identifiant refusé donne
//!   `ProbeError::Auth` ; un serveur qui répond « pas prêt » est une mesure
//!   (`*_ready = 0`, `*_healthy = 0`), pas une panne de transport.
//! * **Les compteurs restent des compteurs.** Les totaux sont écrits tels quels
//!   (`MetricKind::Counter`) ; les règles et les graphes en tirent un débit.
//!   Un total de rejets est toujours émis, même à zéro, parce que ces serveurs
//!   ne créent la série d'une raison qu'au premier rejet.
//! * **La cardinalité est bornée** : des sommes, découpées au plus par raison de
//!   rejet, par chemin de stockage ou par état.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole ; `https` derrière un proxy inverse. |
//! | `port` | celui du produit | 8428, 9428, 3100 ou 9000. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

pub(crate) mod client;
pub mod graylog;
pub mod loki;
pub(crate) mod options;
pub mod prom;
pub mod victoria;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};

use client::{Auth, HttpClient};
use options::Options;
use victoria::Flavour;

pub use options::DEFAULT_REQUEST_TIMEOUT;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn counter(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Counter, ts_ms)
}

fn ready_sample(name: &str, ready: bool, ts_ms: i64) -> Sample {
    gauge(name, if ready { 1.0 } else { 0.0 }, ts_ms)
}

fn http_client(
    target: &Target,
    default_port: u16,
    auth: Auth,
    product: &'static str,
) -> Result<HttpClient, ProbeError> {
    let options = Options::from_target(target, default_port)?;
    Ok(HttpClient::new(
        crate::http::client(options.insecure_tls)?,
        options.base_url,
        auth,
        options.request_timeout,
        product,
    ))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// VictoriaMetrics ou VictoriaLogs, selon la saveur.
pub struct VictoriaCollector {
    flavour: Flavour,
}

impl VictoriaCollector {
    pub fn metrics() -> Self {
        Self { flavour: Flavour::Metrics }
    }

    pub fn logs() -> Self {
        Self { flavour: Flavour::Logs }
    }

    fn client(&self, target: &Target) -> Result<HttpClient, ProbeError> {
        let flavour = self.flavour;
        let auth = Auth::from_credential(&target.credential)?;
        http_client(target, flavour.default_port(), auth, flavour.product())
    }
}

#[async_trait]
impl Collector for VictoriaCollector {
    fn kind(&self) -> &'static str {
        self.flavour.kind()
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        victoria::probe(&self.client(target)?, self.flavour, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        victoria::probe(&self.client(target)?, self.flavour, now_ms()).await?;
        Ok(Some(self.flavour.kind().to_string()))
    }
}

#[derive(Default)]
pub struct LokiCollector;

impl LokiCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let auth = Auth::from_credential(&target.credential)?;
        http_client(target, loki::DEFAULT_PORT, auth, "Loki")
    }
}

#[async_trait]
impl Collector for LokiCollector {
    fn kind(&self) -> &'static str {
        "loki"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        loki::probe(&Self::client(target)?, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        loki::probe(&Self::client(target)?, now_ms()).await?;
        Ok(Some("loki".to_string()))
    }
}

#[derive(Default)]
pub struct GraylogCollector;

impl GraylogCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        // Une adresse copiée depuis le navigateur de l'API se termine souvent
        // par `/api` : les chemins l'ajoutent déjà.
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/api").unwrap_or(trimmed).to_string();
        let auth = Auth::graylog(&target.credential)?;
        http_client(&target, graylog::DEFAULT_PORT, auth, "Graylog")
    }
}

#[async_trait]
impl Collector for GraylogCollector {
    fn kind(&self) -> &'static str {
        "graylog"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        graylog::probe(&Self::client(target)?, target.id, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = Self::client(target)?;
        let _: graylog::SystemInfo = client.get_json("/api/system").await?;
        Ok(Some("graylog".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::StatusCode;
    use axum::routing::{get, post};
    use dumbmonit_proto::Credential;

    use super::*;

    fn target(kind: &str, address: String, credential: Credential) -> Target {
        Target {
            id: 7,
            name: kind.into(),
            address,
            kind: kind.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential,
        }
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn les_types_sont_annonces() {
        assert_eq!(VictoriaCollector::metrics().kind(), "victoriametrics");
        assert_eq!(VictoriaCollector::logs().kind(), "victorialogs");
        assert_eq!(LokiCollector::new().kind(), "loki");
        assert_eq!(GraylogCollector::new().kind(), "graylog");
    }

    #[tokio::test]
    async fn un_loki_pas_pret_reste_une_mesure() {
        let app = Router::new()
            .route(
                "/ready",
                get(|| async {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Ingester not ready: waiting for 15s after being ready",
                    )
                }),
            )
            .route("/metrics", get(|| async { include_str!("testdata/loki_3.7.8.prom") }));
        let base = serve(app).await;
        let samples =
            LokiCollector::new().probe(&target("loki", base, Credential::None)).await.unwrap();
        assert_eq!(value(&samples, "loki_ready"), Some(0.0));
        assert!(value(&samples, "loki_discarded_lines_total").is_some());
    }

    #[tokio::test]
    async fn un_victorialogs_declare_comme_victoriametrics_est_signale() {
        let app = Router::new()
            .route("/health", get(|| async { "OK" }))
            .route("/metrics", get(|| async { include_str!("testdata/victorialogs_1.52.0.prom") }));
        let base = serve(app).await;
        let error = VictoriaCollector::metrics()
            .probe(&target("victoriametrics", base.clone(), Credential::None))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(ref m) if m.contains("VictoriaMetrics")));
        let samples = VictoriaCollector::logs()
            .probe(&target("victorialogs", base, Credential::None))
            .await
            .unwrap();
        assert_eq!(value(&samples, "victorialogs_healthy"), Some(1.0));
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_n_est_pas_une_panne() {
        let app = Router::new().route("/health", get(|| async { StatusCode::UNAUTHORIZED }));
        let base = serve(app).await;
        let error = VictoriaCollector::metrics()
            .probe(&target("victoriametrics", base, Credential::None))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn graylog_avec_opensearch_arrete() {
        use axum::http::HeaderMap;
        let app = Router::new()
            .route(
                "/api/system",
                get(|headers: HeaderMap| async move {
                    // `jeton:token` en basic, rien d'autre.
                    let expected = "Basic dG9rOnRva2Vu";
                    if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some(expected)
                    {
                        return (StatusCode::UNAUTHORIZED, "");
                    }
                    (StatusCode::OK, include_str!("testdata/graylog_7.1.9/system.json"))
                }),
            )
            .route(
                "/api/system/journal",
                get(|| async {
                    include_str!("testdata/graylog_7.1.9/journal_opensearch_down.json")
                }),
            )
            .route(
                "/api/system/indexer/cluster/health",
                get(|| async {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        include_str!(
                            "testdata/graylog_7.1.9/indexer_cluster_health_opensearch_down.json"
                        ),
                    )
                }),
            )
            .route(
                "/api/system/notifications",
                get(|| async { (StatusCode::FORBIDDEN, r#"{"message":"Not authorized"}"#) }),
            )
            .route(
                "/api/system/inputstates",
                get(|| async { include_str!("testdata/graylog_7.1.9/inputstates.json") }),
            )
            .route(
                "/api/system/cluster/nodes",
                get(|| async { include_str!("testdata/graylog_7.1.9/cluster_nodes.json") }),
            )
            .route(
                "/api/system/metrics/multiple",
                post(|headers: HeaderMap| async move {
                    // Sans cet en-tête, Graylog refuse tout POST.
                    assert!(headers.contains_key("x-requested-by"));
                    include_str!("testdata/graylog_7.1.9/metrics_multiple_opensearch_down.json")
                }),
            );
        let base = serve(app).await;
        let token = Credential::ApiToken { token: "tok".into() };
        let samples = GraylogCollector::new()
            .probe(&target("graylog", format!("{base}/api/"), token))
            .await
            .unwrap();
        assert_eq!(value(&samples, "graylog_indexer_status"), Some(3.0));
        assert!(value(&samples, "graylog_journal_uncommitted_entries").unwrap() > 0.0);
        // La permission facultative manque : ni série, ni erreur de collecte.
        assert!(value(&samples, "graylog_notifications").is_none());
        assert_eq!(value(&samples, "graylog_scrape_errors"), Some(0.0));
        assert_eq!(value(&samples, "graylog_inputs_failed"), Some(1.0));

        let error = GraylogCollector::new()
            .probe(&target("graylog", base, Credential::ApiToken { token: "faux".into() }))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }
}
