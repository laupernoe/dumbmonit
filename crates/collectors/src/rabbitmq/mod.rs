//! RabbitMQ, par son API de gestion (greffon `rabbitmq_management`).
//!
//! Le compte recommandé porte la seule étiquette `monitoring` et, sur chaque
//! vhost à surveiller, des permissions vides (`^$` pour configurer, écrire et
//! lire) : il voit l'état des nœuds, des alarmes et des files, et ne peut ni
//! publier, ni consommer, ni lire un message.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **une alarme de ressource** (mémoire ou disque) : RabbitMQ bloque alors
//!   *tous* les éditeurs du cluster, sans erreur côté client — les
//!   applications restent suspendues à leur `publish` ;
//! * **une partition réseau** entre nœuds, **un nœud arrêté** ;
//! * **une file qui se remplit sans consommateur** : le service qui la vidait
//!   est tombé, et personne ne le voit tant que la file ne déborde pas ;
//! * **une file qui n'est plus en service** (`state` autre que `running`) :
//!   file quorum sans majorité, file classique sur un nœud arrêté.
//!
//! # Appels
//!
//! `GET /api/overview`, `GET /api/nodes` (colonnes choisies),
//! `GET /api/health/checks/alarms` (200 sans alarme, 503 avec la liste),
//! `GET /api/queues` paginé, trié par nombre de messages décroissant et limité
//! à `max_queues` files, colonnes choisies.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | `https` si l'API de gestion est servie en TLS. |
//! | `port` | `15672` | Port de l'API de gestion. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |
//! | `max_queues` | `100` | Files décrites une par une, les plus remplies d'abord (0 à 500). |

use std::collections::BTreeMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use serde::Deserialize;
use serde_json::Value;

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::uptime::tags;

pub const DEFAULT_PORT: u16 = 15672;
pub const DEFAULT_MAX_QUEUES: u32 = 100;
/// Plafond de pagination de l'API de gestion.
const MAX_PAGE_SIZE: u32 = 500;

const NODE_COLUMNS: &str = "name,running,mem_used,mem_limit,mem_alarm,disk_free,disk_free_limit,\
                            disk_free_alarm,fd_used,fd_total,partitions,uptime,being_drained";
const QUEUE_COLUMNS: &str =
    "name,vhost,type,state,messages,messages_ready,messages_unacknowledged,consumers";

#[derive(Default)]
pub struct RabbitmqCollector;

impl RabbitmqCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<HttpClient, ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    let auth = match Auth::from_credential(&target.credential)? {
        Auth::Basic { username, password } => Auth::Basic { username, password },
        _ => {
            return Err(ProbeError::Config(
                "RabbitMQ's management API expects a user name and password.".to_string(),
            ));
        }
    };
    Ok(HttpClient::new(
        crate::http::client(options.insecure_tls)?,
        options.base_url,
        auth,
        options.request_timeout,
        "RabbitMQ",
    ))
}

fn max_queues(target: &Target) -> Result<u32, ProbeError> {
    tags::parse_u32(target, "max_queues", DEFAULT_MAX_QUEUES, 0..=MAX_PAGE_SIZE)
}

#[async_trait]
impl Collector for RabbitmqCollector {
    fn kind(&self) -> &'static str {
        "rabbitmq"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let client = client(target)?;
        let replies = read(&client, max_queues(target)?).await?;
        Ok(samples(&replies, chrono::Utc::now().timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let overview: Value = client(target)?.get_json("/api/overview").await?;
        check_overview(&overview)?;
        Ok(Some("rabbitmq".to_string()))
    }
}

pub struct Replies {
    pub overview: Value,
    pub nodes: Vec<Value>,
    pub alarms: Alarms,
    pub queues: Vec<Queue>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Alarms {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub alarms: Vec<Alarm>,
}

#[derive(Debug, Deserialize)]
pub struct Alarm {
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub resource: String,
}

#[derive(Debug, Deserialize)]
pub struct Queue {
    pub name: String,
    #[serde(default)]
    pub vhost: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub messages_ready: Option<f64>,
    #[serde(default)]
    pub messages_unacknowledged: Option<f64>,
    #[serde(default)]
    pub consumers: Option<f64>,
}

#[derive(Deserialize)]
struct QueuePage {
    #[serde(default)]
    items: Vec<Queue>,
}

fn check_overview(overview: &Value) -> Result<(), ProbeError> {
    if overview.get("rabbitmq_version").is_none() && overview.get("management_version").is_none() {
        return Err(ProbeError::Protocol(
            "/api/overview answered without a RabbitMQ version: this does not look like \
             RabbitMQ's management API. Check the device type and the port."
                .to_string(),
        ));
    }
    Ok(())
}

pub async fn read(client: &HttpClient, max_queues: u32) -> Result<Replies, ProbeError> {
    let overview: Value = client.get_json("/api/overview").await?;
    check_overview(&overview)?;
    let nodes: Vec<Value> = client.get_json(&format!("/api/nodes?columns={NODE_COLUMNS}")).await?;

    // 200 sans alarme, 503 avec la liste : les deux sont des réponses.
    let reply = client.get_raw("/api/health/checks/alarms").await?;
    let alarms = match reply.status.as_u16() {
        200 | 503 => serde_json::from_str(&reply.body).unwrap_or_default(),
        _ => return Err(client.status_error(reply.status, &reply.body, "/api/health/checks/alarms")),
    };

    let queues = if max_queues == 0 {
        Vec::new()
    } else {
        let path = format!(
            "/api/queues?page=1&page_size={max_queues}&sort=messages&sort_reverse=true&columns={QUEUE_COLUMNS}"
        );
        let reply = client.get_raw(&path).await?;
        match reply.status.as_u16() {
            // Aucune file visible : l'API répond « page hors limites ».
            400 if reply.body.contains("page") => Vec::new(),
            200 => serde_json::from_str::<QueuePage>(&reply.body)
                .map(|page| page.items)
                .map_err(|error| {
                    ProbeError::Protocol(format!("Unexpected response from /api/queues: {error}"))
                })?,
            _ => return Err(client.status_error(reply.status, &reply.body, "/api/queues")),
        }
    };
    Ok(Replies { overview, nodes, alarms, queues })
}

fn number(value: &Value, path: &str) -> Option<f64> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    current.as_f64().filter(|v| v.is_finite())
}

fn flag(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_bool().map(|b| if b { 1.0 } else { 0.0 })
}

pub fn samples(replies: &Replies, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let counter = |name: &str, value: f64| Sample::new(name, value, MetricKind::Counter, ts_ms);
    let overview = &replies.overview;
    let text = |key: &str| overview.get(key).and_then(Value::as_str).unwrap_or("");

    out.push(
        gauge("rabbitmq_version_info", 1.0)
            .with_label("version", text("rabbitmq_version"))
            .with_label("erlang", text("erlang_version"))
            .with_label("cluster", text("cluster_name")),
    );
    for (name, path) in [
        ("rabbitmq_connections", "object_totals.connections"),
        ("rabbitmq_channels", "object_totals.channels"),
        ("rabbitmq_consumers", "object_totals.consumers"),
        ("rabbitmq_queues", "object_totals.queues"),
        ("rabbitmq_messages_ready", "queue_totals.messages_ready"),
        ("rabbitmq_messages_unacked", "queue_totals.messages_unacknowledged"),
    ] {
        if let Some(value) = number(overview, path) {
            out.push(gauge(name, value));
        }
    }
    // Les compteurs de messages n'existent qu'après le premier message : zéro
    // tant qu'aucun n'a circulé, pour que les débits partent d'une valeur.
    let stat = |key: &str| number(overview, &format!("message_stats.{key}")).unwrap_or(0.0);
    out.push(counter("rabbitmq_messages_published_total", stat("publish")));
    out.push(counter("rabbitmq_messages_delivered_total", stat("deliver_get")));
    out.push(counter("rabbitmq_messages_redelivered_total", stat("redeliver")));
    out.push(counter(
        "rabbitmq_messages_unroutable_total",
        stat("drop_unroutable") + stat("return_unroutable"),
    ));

    // Nœuds.
    let mut running = 0.0;
    let mut partitions = 0.0;
    for node in &replies.nodes {
        let Some(name) = node.get("name").and_then(Value::as_str) else { continue };
        let with = |sample: Sample| sample.with_label("node", name);
        let is_running = node.get("running").and_then(Value::as_bool).unwrap_or(false);
        if is_running {
            running += 1.0;
        }
        out.push(with(gauge("rabbitmq_node_running", if is_running { 1.0 } else { 0.0 })));
        let node_partitions =
            node.get("partitions").and_then(Value::as_array).map_or(0.0, |p| p.len() as f64);
        partitions += node_partitions;
        out.push(with(gauge("rabbitmq_node_partitions", node_partitions)));
        if !is_running {
            // Un nœud arrêté ne publie plus que son nom : rien d'autre à lire.
            continue;
        }
        let mut push = |metric: &str, value: Option<f64>| {
            if let Some(value) = value {
                out.push(with(gauge(metric, value)));
            }
        };
        push("rabbitmq_node_memory_used_bytes", number(node, "mem_used"));
        push("rabbitmq_node_memory_limit_bytes", number(node, "mem_limit"));
        push("rabbitmq_node_disk_free_bytes", number(node, "disk_free"));
        push("rabbitmq_node_disk_free_limit_bytes", number(node, "disk_free_limit"));
        push("rabbitmq_node_fd_used", number(node, "fd_used"));
        push("rabbitmq_node_uptime_seconds", number(node, "uptime").map(|ms| ms / 1000.0));
        push("rabbitmq_node_being_drained", flag(node, "being_drained"));
        if let (Some(used), Some(limit)) = (number(node, "mem_used"), number(node, "mem_limit"))
            && limit > 0.0
        {
            push("rabbitmq_node_memory_used_percent", Some(used / limit * 100.0));
        }
        if let (Some(used), Some(total)) = (number(node, "fd_used"), number(node, "fd_total"))
            && total > 0.0
        {
            push("rabbitmq_node_fd_used_percent", Some(used / total * 100.0));
        }
    }
    out.push(gauge("rabbitmq_nodes", replies.nodes.len() as f64));
    out.push(gauge("rabbitmq_nodes_running", running));
    out.push(gauge("rabbitmq_partitions", partitions));

    // Alarmes : lues sur le contrôle de santé, qui les voit tout de suite
    // (les statistiques des nœuds ont quelques secondes de retard). Mémoire et
    // disque sont toujours écrits, à zéro sans alarme.
    let mut by_node: BTreeMap<(&str, &str), f64> = BTreeMap::new();
    for node in &replies.nodes {
        if let Some(name) = node.get("name").and_then(Value::as_str) {
            by_node.insert((name, "memory"), 0.0);
            by_node.insert((name, "disk"), 0.0);
        }
    }
    for alarm in &replies.alarms.alarms {
        let resource = if alarm.resource.contains("disk") { "disk" } else { alarm.resource.as_str() };
        by_node.insert((alarm.node.as_str(), resource), 1.0);
    }
    for ((node, resource), value) in &by_node {
        out.push(gauge("rabbitmq_alarm", *value).with_label("node", *node).with_label("resource", *resource));
    }
    out.push(gauge("rabbitmq_alarms", replies.alarms.alarms.len() as f64));

    // Files : les plus remplies, une par une ; les décomptes sur la même liste.
    let mut idle_with_messages = 0.0;
    let mut not_running = 0.0;
    for queue in &replies.queues {
        let with = |sample: Sample| {
            sample.with_label("vhost", queue.vhost.clone()).with_label("queue", queue.name.clone())
        };
        let ready = queue.messages_ready.unwrap_or(0.0);
        let consumers = queue.consumers.unwrap_or(0.0);
        out.push(with(gauge("rabbitmq_queue_messages_ready", ready)));
        out.push(with(gauge(
            "rabbitmq_queue_messages_unacked",
            queue.messages_unacknowledged.unwrap_or(0.0),
        )));
        out.push(with(gauge("rabbitmq_queue_consumers", consumers)));
        let state = queue.state.as_deref().unwrap_or("running");
        let is_running = state == "running" || state == "idle";
        out.push(with(gauge("rabbitmq_queue_running", if is_running { 1.0 } else { 0.0 })));
        if consumers == 0.0 && ready > 0.0 {
            idle_with_messages += 1.0;
        }
        if !is_running {
            not_running += 1.0;
        }
    }
    out.push(gauge("rabbitmq_queues_without_consumers", idle_with_messages));
    out.push(gauge("rabbitmq_queues_not_running", not_running));
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

    const OVERVIEW: &str = include_str!("testdata/overview.json");
    const NODES: &str = include_str!("testdata/nodes.json");
    const QUEUES: &str = include_str!("testdata/queues.json");
    const ALARMS_OK: &str = include_str!("testdata/alarms_ok.json");
    const ALARMS_MEMORY: &str = include_str!("testdata/alarms_memory.json");

    fn replies(alarms: &str) -> Replies {
        let page: QueuePage = serde_json::from_str(QUEUES).unwrap();
        Replies {
            overview: serde_json::from_str(OVERVIEW).unwrap(),
            nodes: serde_json::from_str(NODES).unwrap(),
            alarms: serde_json::from_str(alarms).unwrap(),
            queues: page.items,
        }
    }

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    /// RabbitMQ 4.1.4, un nœud, trois files remplies sans consommateur dans
    /// deux vhosts, lu par un compte `monitoring` aux permissions vides.
    #[test]
    fn rabbitmq_reel() {
        let samples = samples(&replies(ALARMS_OK), 0);
        let version = find(&samples, "rabbitmq_version_info", &[]).unwrap();
        assert_eq!(version.labels["version"], "4.1.4");
        assert_eq!(find(&samples, "rabbitmq_messages_ready", &[]).unwrap().value, 11.0);
        assert_eq!(find(&samples, "rabbitmq_queues", &[]).unwrap().value, 3.0);
        assert_eq!(find(&samples, "rabbitmq_nodes_running", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "rabbitmq_partitions", &[]).unwrap().value, 0.0);
        let memory = find(&samples, "rabbitmq_node_memory_used_percent", &[]).unwrap().value;
        assert!(memory > 0.0 && memory < 5.0, "{memory}");
        let alarm = find(&samples, "rabbitmq_alarm", &[("resource", "memory")]).unwrap();
        assert_eq!((alarm.labels["node"].as_str(), alarm.value), ("rabbit@rabbit1", 0.0));
        assert_eq!(find(&samples, "rabbitmq_alarms", &[]).unwrap().value, 0.0);
        let orders =
            find(&samples, "rabbitmq_queue_messages_ready", &[("vhost", "/"), ("queue", "orders")]);
        assert_eq!(orders.unwrap().value, 7.0);
        assert!(find(&samples, "rabbitmq_queue_consumers", &[("vhost", "shop"), ("queue", "emails")]).is_some());
        assert_eq!(find(&samples, "rabbitmq_queues_without_consumers", &[]).unwrap().value, 3.0);
        assert_eq!(find(&samples, "rabbitmq_queues_not_running", &[]).unwrap().value, 0.0);
        let published = find(&samples, "rabbitmq_messages_published_total", &[]).unwrap();
        assert_eq!(published.kind, MetricKind::Counter);
    }

    #[test]
    fn une_alarme_memoire_reelle() {
        let samples = samples(&replies(ALARMS_MEMORY), 0);
        assert_eq!(find(&samples, "rabbitmq_alarm", &[("resource", "memory")]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "rabbitmq_alarm", &[("resource", "disk")]).unwrap().value, 0.0);
        assert_eq!(find(&samples, "rabbitmq_alarms", &[]).unwrap().value, 1.0);
    }

    #[test]
    fn un_noeud_arrete_et_une_file_quorum_sans_majorite() {
        let mut replies = replies(ALARMS_OK);
        replies.nodes.push(serde_json::json!({"name": "rabbit@rabbit2", "running": false, "partitions": []}));
        replies.queues.push(Queue {
            name: "payments".into(),
            vhost: "/".into(),
            state: Some("minority".into()),
            messages_ready: None,
            messages_unacknowledged: None,
            consumers: Some(2.0),
        });
        let samples = samples(&replies, 0);
        assert_eq!(find(&samples, "rabbitmq_node_running", &[("node", "rabbit@rabbit2")]).unwrap().value, 0.0);
        assert!(find(&samples, "rabbitmq_node_memory_used_bytes", &[("node", "rabbit@rabbit2")]).is_none());
        assert_eq!(find(&samples, "rabbitmq_nodes_running", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "rabbitmq_queues_not_running", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "rabbitmq_queue_running", &[("queue", "payments")]).unwrap().value, 0.0);
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
    async fn interrogation_complete() {
        let app = Router::new()
            .route(
                "/api/overview",
                get(|headers: HeaderMap| async move {
                    if authorized(&headers) { (StatusCode::OK, OVERVIEW) } else { (StatusCode::UNAUTHORIZED, "") }
                }),
            )
            .route("/api/nodes", get(|| async { NODES }))
            .route(
                "/api/health/checks/alarms",
                get(|| async { (StatusCode::SERVICE_UNAVAILABLE, ALARMS_MEMORY) }),
            )
            .route("/api/queues", get(|| async { QUEUES }));
        let base = serve(app).await;
        let mut target = cible("rabbitmq", &base, &[]);
        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "secret".into() };
        let samples = RabbitmqCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "rabbitmq_alarms", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "rabbitmq_queues_without_consumers", &[]).unwrap().value, 3.0);

        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "faux".into() };
        let error = RabbitmqCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");

        target.credential = Credential::None;
        let error = RabbitmqCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_RABBITMQ=hôte` et
    /// `DUMBMONIT_TEST_RABBITMQ_PASSWORD` (utilisateur `dumbmonit`).
    #[tokio::test]
    #[ignore = "demande un RabbitMQ joignable"]
    async fn rabbitmq_reel() {
        let address = std::env::var("DUMBMONIT_TEST_RABBITMQ").unwrap();
        let password = std::env::var("DUMBMONIT_TEST_RABBITMQ_PASSWORD").unwrap();
        let mut target = cible("rabbitmq", &address, &[]);
        target.credential = Credential::UsernamePassword { username: "dumbmonit".into(), password };
        for sample in RabbitmqCollector::new().probe(&target).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
