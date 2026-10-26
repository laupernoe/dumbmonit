//! Elasticsearch et OpenSearch, par leur API HTTP.
//!
//! Les deux moteurs parlent le même dialecte pour ce qui nous intéresse : un
//! seul type d'équipement, `elasticsearch`, les couvre. Le compte recommandé ne
//! porte que le privilège de cluster `monitor` : il lit la santé et les
//! statistiques des nœuds, aucun document d'aucun index.
//!
//! Ce que ce module existe pour voir :
//!
//! * **un cluster `red`** : au moins un shard primaire n'est pas alloué, des
//!   données sont inaccessibles ; **`yellow`** : des réplicas manquent, la
//!   prochaine panne d'un nœud perdra des données ;
//! * **des shards non alloués** ou qui n'en finissent pas de s'initialiser ;
//! * **un disque qui approche les seuils d'allocation** : à 95 % (flood stage),
//!   les index passent en lecture seule et les écritures sont refusées ;
//! * **un tas JVM saturé**, avant que le nœud ne ralentisse puis ne quitte le
//!   cluster.
//!
//! # Appels
//!
//! `GET /` (version), `GET /_cluster/health`, `GET /_nodes/stats/fs,jvm,os`.
//! Ce dernier est facultatif : un compte qui n'a pas le droit de le lire garde
//! la santé du cluster, sans les métriques par nœud.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | `https` si l'API est servie en TLS. |
//! | `port` | `9200` | Port de l'API. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use serde_json::Value;

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;

pub const DEFAULT_PORT: u16 = 9200;

#[derive(Default)]
pub struct ElasticsearchCollector;

impl ElasticsearchCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<HttpClient, ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    let auth = Auth::from_credential(&target.credential)?;
    Ok(HttpClient::new(
        crate::http::client(options.insecure_tls)?,
        options.base_url,
        auth,
        options.request_timeout,
        "Elasticsearch",
    ))
}

#[async_trait]
impl Collector for ElasticsearchCollector {
    fn kind(&self) -> &'static str {
        "elasticsearch"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let replies = read(&client(target)?).await?;
        Ok(samples(&replies, chrono::Utc::now().timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let info: Value = client(target)?.get_json("/").await?;
        check_info(&info)?;
        Ok(Some("elasticsearch".to_string()))
    }
}

/// Ce que renvoient les trois appels.
pub struct Replies {
    pub info: Value,
    pub health: Value,
    /// `Null` quand le compte n'a pas le droit de lire les statistiques.
    pub nodes: Value,
}

fn check_info(info: &Value) -> Result<(), ProbeError> {
    if info.pointer("/version/number").and_then(Value::as_str).is_none() {
        return Err(ProbeError::Protocol(
            "/ answered without a version number: this does not look like Elasticsearch or \
             OpenSearch. Check the device type and the port."
                .to_string(),
        ));
    }
    Ok(())
}

pub async fn read(client: &HttpClient) -> Result<Replies, ProbeError> {
    let info: Value = client.get_json("/").await?;
    check_info(&info)?;
    let health: Value = client.get_json("/_cluster/health").await?;
    let path = "/_nodes/stats/fs,jvm,os";
    let reply = client.get_json_raw(path).await?;
    let nodes = if reply.status.is_success() {
        serde_json::from_str(&reply.body).map_err(|error| {
            ProbeError::Protocol(format!("Unexpected response from {path}: {error}"))
        })?
    } else if reply.status.as_u16() == 403 {
        Value::Null
    } else {
        return Err(client.status_error(reply.status, &reply.body, path));
    };
    Ok(Replies { info, health, nodes })
}

fn number(value: &Value, pointer: &str) -> Option<f64> {
    value.pointer(pointer)?.as_f64().filter(|v| v.is_finite())
}

/// `green` 0, `yellow` 1, `red` 2 : une valeur qui se compare à un seuil.
pub fn status_level(status: &str) -> Option<f64> {
    match status {
        "green" => Some(0.0),
        "yellow" => Some(1.0),
        "red" => Some(2.0),
        _ => None,
    }
}

pub fn samples(replies: &Replies, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);

    let text = |pointer: &str| replies.info.pointer(pointer).and_then(Value::as_str).unwrap_or("");
    let distribution = match text("/version/distribution") {
        "" => "elasticsearch",
        other => other,
    };
    out.push(
        gauge("elasticsearch_version_info", 1.0)
            .with_label("version", text("/version/number"))
            .with_label("cluster", text("/cluster_name"))
            .with_label("distribution", distribution),
    );

    let health = &replies.health;
    if let Some(level) = health.get("status").and_then(Value::as_str).and_then(status_level) {
        out.push(gauge("elasticsearch_cluster_status", level));
    }
    for (name, key) in [
        ("elasticsearch_nodes", "number_of_nodes"),
        ("elasticsearch_data_nodes", "number_of_data_nodes"),
        ("elasticsearch_active_primary_shards", "active_primary_shards"),
        ("elasticsearch_active_shards", "active_shards"),
        ("elasticsearch_relocating_shards", "relocating_shards"),
        ("elasticsearch_initializing_shards", "initializing_shards"),
        ("elasticsearch_unassigned_shards", "unassigned_shards"),
        ("elasticsearch_delayed_unassigned_shards", "delayed_unassigned_shards"),
        ("elasticsearch_pending_tasks", "number_of_pending_tasks"),
        ("elasticsearch_active_shards_percent", "active_shards_percent_as_number"),
    ] {
        if let Some(value) = number(health, &format!("/{key}")) {
            out.push(gauge(name, value));
        }
    }

    if let Some(nodes) = replies.nodes.pointer("/nodes").and_then(Value::as_object) {
        for (id, node) in nodes {
            let name = node.get("name").and_then(Value::as_str).unwrap_or(id.as_str());
            let with = |sample: Sample| sample.with_label("node", name);
            if let (Some(total), Some(free)) = (
                number(node, "/fs/total/total_in_bytes"),
                number(node, "/fs/total/available_in_bytes"),
            ) && total > 0.0
            {
                out.push(with(gauge("elasticsearch_node_disk_free_bytes", free)));
                out.push(with(gauge("elasticsearch_node_disk_total_bytes", total)));
                out.push(with(gauge(
                    "elasticsearch_node_disk_used_percent",
                    ((total - free) / total * 100.0).clamp(0.0, 100.0),
                )));
            }
            if let Some(heap) = number(node, "/jvm/mem/heap_used_percent") {
                out.push(with(gauge("elasticsearch_node_heap_used_percent", heap)));
            }
            if let Some(cpu) = number(node, "/os/cpu/percent") {
                out.push(with(gauge("elasticsearch_node_cpu_percent", cpu)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    const INFO: &str = r#"{"name":"es1","cluster_name":"prod","version":{"number":"8.15.2"}}"#;
    const OPENSEARCH: &str = r#"{"name":"os1","cluster_name":"logs","version":{"distribution":"opensearch","number":"2.17.0"}}"#;
    const HEALTH_YELLOW: &str = r#"{"cluster_name":"prod","status":"yellow","number_of_nodes":3,
        "number_of_data_nodes":3,"active_primary_shards":40,"active_shards":70,
        "relocating_shards":0,"initializing_shards":1,"unassigned_shards":9,
        "delayed_unassigned_shards":0,"number_of_pending_tasks":0,
        "active_shards_percent_as_number":87.5}"#;
    const HEALTH_RED: &str = r#"{"cluster_name":"prod","status":"red","number_of_nodes":2,
        "unassigned_shards":12,"active_shards_percent_as_number":60.0}"#;
    const NODES: &str = r#"{"nodes":{"abc":{"name":"data-1",
        "fs":{"total":{"total_in_bytes":1000,"available_in_bytes":100}},
        "jvm":{"mem":{"heap_used_percent":71}},"os":{"cpu":{"percent":12}}},
        "def":{"name":"data-2","fs":{"total":{"total_in_bytes":2000,"available_in_bytes":1500}}}}}"#;

    fn replies(info: &str, health: &str, nodes: &str) -> Replies {
        Replies {
            info: serde_json::from_str(info).unwrap(),
            health: serde_json::from_str(health).unwrap(),
            nodes: serde_json::from_str(nodes).unwrap(),
        }
    }

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    #[test]
    fn cluster_jaune_avec_shards_non_alloues() {
        let samples = samples(&replies(INFO, HEALTH_YELLOW, NODES), 0);
        assert_eq!(find(&samples, "elasticsearch_cluster_status", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "elasticsearch_unassigned_shards", &[]).unwrap().value, 9.0);
        assert_eq!(find(&samples, "elasticsearch_nodes", &[]).unwrap().value, 3.0);
        let version = find(&samples, "elasticsearch_version_info", &[]).unwrap();
        assert_eq!(version.labels["version"], "8.15.2");
        assert_eq!(version.labels["distribution"], "elasticsearch");
    }

    #[test]
    fn cluster_rouge() {
        let samples = samples(&replies(INFO, HEALTH_RED, "null"), 0);
        assert_eq!(find(&samples, "elasticsearch_cluster_status", &[]).unwrap().value, 2.0);
        // Sans droit sur les statistiques : pas de série par nœud.
        assert!(find(&samples, "elasticsearch_node_disk_used_percent", &[]).is_none());
    }

    #[test]
    fn disque_et_tas_par_noeud() {
        let samples = samples(&replies(INFO, HEALTH_YELLOW, NODES), 0);
        let disk = find(&samples, "elasticsearch_node_disk_used_percent", &[("node", "data-1")]);
        assert_eq!(disk.unwrap().value, 90.0);
        let disk = find(&samples, "elasticsearch_node_disk_used_percent", &[("node", "data-2")]);
        assert_eq!(disk.unwrap().value, 25.0);
        let heap = find(&samples, "elasticsearch_node_heap_used_percent", &[("node", "data-1")]);
        assert_eq!(heap.unwrap().value, 71.0);
        // data-2 n'annonce pas son tas : rien d'inventé.
        assert!(
            find(&samples, "elasticsearch_node_heap_used_percent", &[("node", "data-2")]).is_none()
        );
    }

    #[test]
    fn opensearch_est_reconnu() {
        let samples = samples(&replies(OPENSEARCH, HEALTH_RED, "null"), 0);
        let version = find(&samples, "elasticsearch_version_info", &[]).unwrap();
        assert_eq!(version.labels["distribution"], "opensearch");
        assert_eq!(version.labels["cluster"], "logs");
    }

    #[test]
    fn un_statut_inconnu_ne_produit_pas_de_serie() {
        let samples = samples(&replies(INFO, r#"{"status":"purple"}"#, "null"), 0);
        assert!(find(&samples, "elasticsearch_cluster_status", &[]).is_none());
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn authorized(headers: &HeaderMap) -> bool {
        // dumbmonit:secret
        headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some("Basic ZHVtYm1vbml0OnNlY3JldA==")
    }

    #[tokio::test]
    async fn interrogation_complete_et_identifiants_refuses() {
        let app = Router::new()
            .route(
                "/",
                get(|headers: HeaderMap| async move {
                    if authorized(&headers) {
                        (StatusCode::OK, INFO)
                    } else {
                        (StatusCode::UNAUTHORIZED, "")
                    }
                }),
            )
            .route("/_cluster/health", get(|| async { HEALTH_YELLOW }))
            .route("/_nodes/stats/fs,jvm,os", get(|| async { (StatusCode::FORBIDDEN, "{}") }));
        let base = serve(app).await;
        let mut target = cible("elasticsearch", &base, &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "secret".into(),
        };
        let samples = ElasticsearchCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "elasticsearch_cluster_status", &[]).unwrap().value, 1.0);
        assert!(find(&samples, "elasticsearch_node_heap_used_percent", &[]).is_none());

        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "faux".into() };
        let error = ElasticsearchCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)) && !error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn un_port_ferme_est_une_panne() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let target = cible("elasticsearch", &format!("127.0.0.1:{port}"), &[]);
        let error = ElasticsearchCollector::new().probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn un_autre_service_est_refuse_comme_erreur_de_protocole() {
        let app = Router::new().route("/", get(|| async { r#"{"hello":"world"}"# }));
        let base = serve(app).await;
        let target = cible("elasticsearch", &base, &[]);
        let error = ElasticsearchCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }
}
