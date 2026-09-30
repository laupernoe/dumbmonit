//! Le panneau des services d'arrière-plan : Redis / Valkey, MongoDB, RabbitMQ
//! et CrowdSec (`collectors/{redis,mongodb,rabbitmq,crowdsec}`).
//!
//! `GET /targets/{id}/backend` reconstruit la vue à partir des séries de la
//! cible dans VictoriaMetrics, sans jamais réinterroger le service : dernières
//! valeurs, débits sur dix minutes, accroissements sur vingt-quatre heures, et
//! pour CrowdSec l'âge du dernier changement du compteur de chaque bouncer. La
//! vue a la forme de celle des serveurs de journaux (`observability.rs`) :
//! l'interface affiche les deux avec le même panneau.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_proto::TargetId;

use crate::api::observability::{Breakdown, Check, Figure, ObservabilityView, Row};
use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

pub const KINDS: [&str; 4] = ["redis", "mongodb", "rabbitmq", "crowdsec"];

const LOOKBACK: &str = "1h";
/// Au-delà, un bouncer est tenu pour arrêté (même seuil que la règle livrée).
const BOUNCER_STALE_SECONDS: f64 = 30.0 * 60.0;
/// Files montrées dans le panneau, les plus remplies d'abord.
const QUEUES_SHOWN: usize = 10;

pub fn routes() -> Router<AppState> {
    Router::new().route("/targets/{id}/backend", get(overview))
}

async fn overview(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<ObservabilityView>> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device {id} not found.")))?;
    let kind = target.kind.as_str();
    if !KINDS.contains(&kind) {
        return Err(ApiError::NotFound(format!(
            "Device {id} is not a Redis, MongoDB, RabbitMQ or CrowdSec server."
        )));
    }
    let selector = format!(r#"{{__name__=~"dumbmonit_{kind}_.*", target="{id}"}}"#);
    let counters = format!(r#"{{__name__=~"dumbmonit_{kind}_.*_total", target="{id}"}}"#);
    let last = format!("last_over_time({selector}[{LOOKBACK}]) keep_metric_names");
    let rates = format!("rate({counters}[10m]) keep_metric_names");
    let day = format!("increase_prometheus({counters}[24h]) keep_metric_names");
    // L'âge du dernier tirage de chaque bouncer, déduit de son compteur.
    let idle = format!(
        r#"(time() - tlast_change_over_time(dumbmonit_crowdsec_bouncer_requests_total{{target="{id}"}}[1d])) keep_metric_names"#
    );
    let (last, rates, day) = tokio::try_join!(
        state.victoria.query(&last),
        state.victoria.query(&rates),
        state.victoria.query(&day),
    )?;
    let idle = if kind == "crowdsec" { state.victoria.query(&idle).await? } else { Vec::new() };
    Ok(Json(build_view(kind, &Series::new(kind, &last, &rates, &day, &idle))))
}

// ------------------------------------------------------------- assemblage

struct Entry {
    name: String,
    labels: BTreeMap<String, String>,
    value: f64,
}

/// Les séries de la cible, rangées par nom court (sans `dumbmonit_<kind>_`).
#[derive(Default)]
pub struct Series {
    last: Vec<Entry>,
    rate: Vec<Entry>,
    day: Vec<Entry>,
    idle: Vec<Entry>,
    sampled_at: Option<f64>,
}

impl Series {
    pub fn new(
        kind: &str,
        last: &[InstantSeries],
        rate: &[InstantSeries],
        day: &[InstantSeries],
        idle: &[InstantSeries],
    ) -> Self {
        let prefix = format!("dumbmonit_{kind}_");
        let convert = |list: &[InstantSeries]| -> Vec<Entry> {
            list.iter()
                .filter_map(|s| {
                    let name = s.metric.get("__name__")?.strip_prefix(&prefix)?.to_string();
                    let value = s.value.1.parse::<f64>().ok().filter(|v| v.is_finite())?;
                    Some(Entry { name, labels: s.metric.clone(), value })
                })
                .collect()
        };
        let sampled_at = last.iter().map(|s| s.value.0).reduce(f64::max);
        Self {
            last: convert(last),
            rate: convert(rate),
            day: convert(day),
            idle: convert(idle),
            sampled_at,
        }
    }

    fn get(&self, name: &str) -> Option<f64> {
        pick(&self.last, name)
    }
    fn rate(&self, name: &str) -> Option<f64> {
        pick(&self.rate, name)
    }
    fn day(&self, name: &str) -> Option<f64> {
        pick(&self.day, name)
    }
    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Entry> + 'a {
        self.last.iter().filter(move |e| e.name == name)
    }
    fn max(&self, name: &str) -> Option<f64> {
        self.all(name).map(|e| e.value).reduce(f64::max)
    }
    fn label(&self, name: &str, label: &str) -> Option<String> {
        self.all(name).next().and_then(|e| e.labels.get(label).cloned())
    }
    /// Valeur d'une série de la même famille portant les mêmes étiquettes `keys`.
    fn sibling(&self, name: &str, entry: &Entry, keys: &[&str]) -> Option<f64> {
        self.all(name)
            .find(|e| keys.iter().all(|k| e.labels.get(*k) == entry.labels.get(*k)))
            .map(|e| e.value)
    }
}

fn pick(entries: &[Entry], name: &str) -> Option<f64> {
    let mut found = false;
    let mut total = 0.0;
    for entry in entries.iter().filter(|e| e.name == name) {
        found = true;
        total += entry.value;
    }
    found.then_some(total)
}

fn check(label: &str, state: &'static str, detail: impl Into<String>) -> Check {
    Check { label: label.to_string(), state, detail: detail.into() }
}

fn figure(label: &str, value: Option<f64>, unit: &'static str) -> Figure {
    Figure { label: label.to_string(), value, unit }
}

fn row(label: impl Into<String>, value: Option<f64>, unit: &'static str) -> Row {
    Row { label: label.into(), value, unit, state: None }
}

fn plural(count: f64, one: &str, many: &str) -> String {
    let n = count.round();
    format!("{n} {}", if n == 1.0 { one } else { many })
}

fn label_of(entry: &Entry, key: &str) -> String {
    entry.labels.get(key).cloned().unwrap_or_default()
}

/// Les trois premiers noms d'une liste, puis « and N more ».
fn names(list: &[String]) -> String {
    let shown: Vec<&str> = list.iter().take(3).map(String::as_str).collect();
    let rest = list.len().saturating_sub(3);
    if rest > 0 { format!("{} and {rest} more", shown.join(", ")) } else { shown.join(", ") }
}

pub fn build_view(kind: &str, s: &Series) -> ObservabilityView {
    let mut view = ObservabilityView {
        kind: kind.to_string(),
        version: s.label("version_info", "version"),
        sampled_at: s.sampled_at,
        ..Default::default()
    };
    if s.sampled_at.is_none() {
        view.state = "ok".to_string();
        return view;
    }
    match kind {
        "redis" => redis(&mut view, s),
        "mongodb" => mongodb(&mut view, s),
        "rabbitmq" => rabbitmq(&mut view, s),
        _ => crowdsec(&mut view, s),
    }
    view.state = if view.checks.iter().any(|c| c.state == "warning") {
        "warning"
    } else if view.checks.iter().any(|c| c.state == "advisory") {
        "advisory"
    } else {
        "ok"
    }
    .to_string();
    view
}

// ------------------------------------------------------------------ Redis

fn redis(view: &mut ObservabilityView, s: &Series) {
    if let Some(product) = s.label("version_info", "product").filter(|p| p != "redis") {
        // Valkey garde `redis_version` figé : le nom du produit lève le doute.
        let mut chars = product.chars();
        let name = chars.next().map(|c| c.to_uppercase().chain(chars).collect::<String>());
        view.version = Some(format!(
            "{} {}",
            name.unwrap_or_default(),
            view.version.take().unwrap_or_default()
        ));
    }
    let role = s.label("role_info", "role").unwrap_or_default();

    // Réplication, vue de la réplique puis du primaire.
    if role == "replica" {
        match s.get("master_link_up") {
            Some(up) if up >= 1.0 => {
                view.checks.push(check("Replication", "ok", "Replica, linked to its primary."))
            }
            Some(_) => view.checks.push(check(
                "Replication",
                "warning",
                "Replica whose link to the primary is down: it serves stale data.",
            )),
            None => {}
        }
    } else if let Some(replicas) = s.get("connected_replicas").filter(|n| *n > 0.0) {
        let offline: Vec<String> = s
            .all("replica_online")
            .filter(|e| e.value < 1.0)
            .map(|e| label_of(e, "replica"))
            .collect();
        let lag = s.max("replica_lag_seconds").unwrap_or(0.0);
        let (state, detail) = if !offline.is_empty() {
            ("warning", format!("Replicas not online: {}.", names(&offline)))
        } else if lag > 30.0 {
            ("advisory", format!("A replica has not acknowledged for {lag:.0} seconds."))
        } else {
            ("ok", format!("Primary with {} in sync.", plural(replicas, "replica", "replicas")))
        };
        view.checks.push(check("Replication", state, detail));
    }

    // Mémoire contre `maxmemory`.
    match s.get("memory_used_percent") {
        Some(percent) => {
            let policy = s.label("maxmemory_policy_info", "policy").unwrap_or_default();
            let state = if percent > 90.0 { "warning" } else { "ok" };
            view.checks.push(check(
                "Memory",
                state,
                format!(
                    "{percent:.0}% of maxmemory used; at the limit the policy {policy} applies."
                ),
            ));
        }
        None if s.get("memory_used_bytes").is_some() => view.checks.push(check(
            "Memory",
            "ok",
            "No maxmemory set: Redis grows until the host runs out of memory.",
        )),
        None => {}
    }

    let mut failures = Vec::new();
    if s.get("rdb_last_save_ok") == Some(0.0) {
        failures.push("the last snapshot (BGSAVE) failed");
    }
    if s.get("aof_last_write_ok") == Some(0.0) {
        failures.push("the last append-only file write failed");
    }
    if failures.is_empty() {
        if s.get("rdb_last_save_ok").is_some() {
            view.checks.push(check("Persistence", "ok", "Last snapshot and writes succeeded."));
        }
    } else {
        view.checks.push(check(
            "Persistence",
            "warning",
            format!("{}: check the disk of the server.", capitalise(&failures.join(", and "))),
        ));
    }

    if let Some(rejected) = s.day("rejected_connections_total").filter(|n| *n >= 0.5) {
        view.checks.push(check(
            "Connections",
            "warning",
            format!(
                "{} refused in the last 24 hours: maxclients is reached.",
                plural(rejected, "connection", "connections")
            ),
        ));
    }

    let hits = s.rate("keyspace_hits_total");
    let misses = s.rate("keyspace_misses_total");
    let hit_ratio = match (hits, misses) {
        (Some(h), Some(m)) if h + m > 0.0 => Some(h / (h + m) * 100.0),
        _ => None,
    };
    view.figures.push(figure("Keys", s.get("keys"), "count"));
    view.figures.push(figure("Memory used", s.get("memory_used_bytes"), "bytes"));
    view.figures.push(figure("Clients", s.get("connected_clients"), "count"));
    view.figures.push(figure("Commands", s.rate("commands_processed_total"), "per_second"));
    view.figures.push(figure("Hit ratio", hit_ratio, "percent"));
    view.figures.push(figure("Evicted in 24 h", s.day("evicted_keys_total"), "count"));
    view.figures.push(figure("Up for", s.get("uptime_seconds"), "seconds"));

    let mut dbs: Vec<Row> =
        s.all("db_keys").map(|e| row(label_of(e, "db"), Some(e.value), "count")).collect();
    dbs.sort_by(|a, b| a.label.cmp(&b.label));
    if !dbs.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Keyspace".to_string(),
            note: "Keys per database.".to_string(),
            rows: dbs,
        });
    }
    let replicas: Vec<Row> = s
        .all("replica_online")
        .map(|e| Row {
            label: format!("{} lag", label_of(e, "replica")),
            value: s.sibling("replica_lag_seconds", e, &["replica"]),
            unit: "seconds",
            state: Some(if e.value >= 1.0 { "ok" } else { "warning" }),
        })
        .collect();
    if !replicas.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Replicas".to_string(),
            note: "Seconds since each replica last acknowledged.".to_string(),
            rows: replicas,
        });
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

// ---------------------------------------------------------------- MongoDB

fn mongodb(view: &mut ObservabilityView, s: &Series) {
    if let Some(present) = s.get("replset_primary_present") {
        let set = s.label("replset_info", "set").unwrap_or_default();
        let down: Vec<String> = s
            .all("replset_member_health")
            .filter(|e| e.value < 1.0)
            .map(|e| label_of(e, "member"))
            .collect();
        let lag = s.max("replset_member_lag_seconds").unwrap_or(0.0);
        let (state, detail) = if present < 1.0 {
            ("warning", format!("Replica set {set} has no primary: no write is accepted."))
        } else if !down.is_empty() {
            ("warning", format!("Members unreachable: {}.", names(&down)))
        } else if lag > 60.0 {
            ("advisory", format!("A secondary is {lag:.0} seconds behind the primary."))
        } else {
            let role = s.label("role_info", "role").unwrap_or_default();
            ("ok", format!("Replica set {set}, this member is {role}, every member healthy."))
        };
        view.checks.push(check("Replica set", state, detail));
    }
    if let Some(percent) = s.get("connections_used_percent") {
        let state = if percent > 80.0 { "warning" } else { "ok" };
        view.checks.push(check(
            "Connections",
            state,
            format!("{percent:.1}% of the available connections in use."),
        ));
    }
    if let Some(dirty) = s.get("wiredtiger_cache_dirty_percent") {
        let used = s.get("wiredtiger_cache_used_percent").unwrap_or(0.0);
        let state = if dirty > 20.0 {
            "warning"
        } else if used > 95.0 {
            "advisory"
        } else {
            "ok"
        };
        view.checks.push(check(
            "Cache",
            state,
            format!(
                "WiredTiger cache {used:.0}% full, {dirty:.1}% dirty. Above 95% full or 20% dirty, \
                 queries evict pages themselves and slow down."
            ),
        ));
    }
    let regular = s
        .day
        .iter()
        .find(|e| {
            e.name == "asserts_total" && e.labels.get("type").map(String::as_str) == Some("regular")
        })
        .map(|e| e.value);
    if let Some(n) = regular.filter(|n| *n >= 0.5) {
        view.checks.push(check(
            "Internal errors",
            "advisory",
            format!(
                "{} in the last 24 hours.",
                plural(n, "regular assertion", "regular assertions")
            ),
        ));
    }

    view.figures.push(figure("Connections", s.get("connections_current"), "count"));
    view.figures.push(figure("Operations", s.rate("operations_total"), "per_second"));
    view.figures.push(figure("Cache used", s.get("wiredtiger_cache_used_bytes"), "bytes"));
    view.figures.push(figure("Resident memory", s.get("memory_resident_bytes"), "bytes"));
    view.figures.push(figure("Queued operations", s.get("queued_operations"), "count"));
    view.figures.push(figure("Up for", s.get("uptime_seconds"), "seconds"));

    let mut members: Vec<Row> = s
        .all("replset_member_state")
        .map(|e| {
            let state_name = label_of(e, "state");
            let healthy = s.sibling("replset_member_health", e, &["member"]).unwrap_or(1.0) >= 1.0;
            Row {
                label: format!("{} ({state_name})", label_of(e, "member")),
                value: s.sibling("replset_member_lag_seconds", e, &["member"]),
                unit: "seconds",
                state: Some(if !healthy || state_name == "down" { "warning" } else { "ok" }),
            }
        })
        .collect();
    members.sort_by(|a, b| a.label.cmp(&b.label));
    if !members.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Replica set members".to_string(),
            note: "State of each member, and how far each secondary is behind.".to_string(),
            rows: members,
        });
    }
    let ops: Vec<Row> = s
        .rate
        .iter()
        .filter(|e| e.name == "operations_total")
        .map(|e| row(label_of(e, "op"), Some(e.value), "per_second"))
        .collect();
    if !ops.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Operations".to_string(),
            note: "Per second over the last ten minutes.".to_string(),
            rows: ops,
        });
    }
}

// --------------------------------------------------------------- RabbitMQ

fn rabbitmq(view: &mut ObservabilityView, s: &Series) {
    let alarms: Vec<String> = s
        .all("alarm")
        .filter(|e| e.value >= 1.0)
        .map(|e| format!("{} on {}", label_of(e, "resource"), label_of(e, "node")))
        .collect();
    if alarms.is_empty() {
        view.checks.push(check("Alarms", "ok", "No memory or disk alarm."));
    } else {
        view.checks.push(check(
            "Alarms",
            "warning",
            format!(
                "Resource alarm ({}): every publisher in the cluster is blocked.",
                names(&alarms)
            ),
        ));
    }
    if let (Some(total), Some(running)) = (s.get("nodes"), s.get("nodes_running")) {
        let partitions = s.get("partitions").unwrap_or(0.0);
        let (state, detail) = if running < total {
            ("warning", format!("{running:.0} of {total:.0} nodes running."))
        } else if partitions > 0.0 {
            ("warning", "Nodes see a network partition: the cluster is split.".to_string())
        } else {
            ("ok", format!("{} running, no partition.", plural(total, "node", "nodes")))
        };
        view.checks.push(check("Cluster", state, detail));
    }
    let idle: Vec<String> = s
        .all("queue_messages_ready")
        .filter(|e| {
            e.value > 0.0 && s.sibling("queue_consumers", e, &["vhost", "queue"]) == Some(0.0)
        })
        .map(queue_name)
        .collect();
    if !idle.is_empty() {
        view.checks.push(check(
            "Consumers",
            "advisory",
            format!("Queues holding messages with no consumer: {}.", names(&idle)),
        ));
    }
    let stopped: Vec<String> =
        s.all("queue_running").filter(|e| e.value < 1.0).map(queue_name).collect();
    if !stopped.is_empty() {
        view.checks.push(check(
            "Queues",
            "warning",
            format!("Queues not running: {}.", names(&stopped)),
        ));
    }

    view.figures.push(figure("Messages ready", s.get("messages_ready"), "count"));
    view.figures.push(figure("Unacknowledged", s.get("messages_unacked"), "count"));
    view.figures.push(figure("Published", s.rate("messages_published_total"), "per_second"));
    view.figures.push(figure("Delivered", s.rate("messages_delivered_total"), "per_second"));
    view.figures.push(figure("Connections", s.get("connections"), "count"));
    view.figures.push(figure("Consumers", s.get("consumers"), "count"));
    view.figures.push(figure("Queues", s.get("queues"), "count"));

    let mut queues: Vec<&Entry> = s.all("queue_messages_ready").collect();
    queues.sort_by(|a, b| b.value.total_cmp(&a.value));
    let rows: Vec<Row> = queues
        .into_iter()
        .take(QUEUES_SHOWN)
        .map(|e| {
            let consumers = s.sibling("queue_consumers", e, &["vhost", "queue"]).unwrap_or(0.0);
            Row {
                label: format!(
                    "{} ({})",
                    queue_name(e),
                    plural(consumers, "consumer", "consumers")
                ),
                value: Some(e.value),
                unit: "count",
                state: Some(if consumers == 0.0 && e.value > 0.0 { "advisory" } else { "ok" }),
            }
        })
        .collect();
    if !rows.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Fullest queues".to_string(),
            note: "Messages ready, and who reads them.".to_string(),
            rows,
        });
    }
    let nodes: Vec<Row> = s
        .all("node_running")
        .flat_map(|e| {
            let node = label_of(e, "node");
            let memory = s.sibling("node_memory_used_percent", e, &["node"]);
            let disk = s.sibling("node_disk_free_bytes", e, &["node"]);
            let running = e.value >= 1.0;
            [
                Row {
                    label: format!("{node} memory"),
                    value: memory,
                    unit: "percent",
                    state: Some(if running { "ok" } else { "warning" }),
                },
                row(format!("{node} free disk"), disk, "bytes"),
            ]
        })
        .collect();
    if !nodes.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Nodes".to_string(),
            note: "Memory against the high watermark, and free disk.".to_string(),
            rows: nodes,
        });
    }
}

fn queue_name(entry: &Entry) -> String {
    let vhost = label_of(entry, "vhost");
    let queue = label_of(entry, "queue");
    if vhost == "/" { queue } else { format!("{vhost}/{queue}") }
}

// --------------------------------------------------------------- CrowdSec

fn crowdsec(view: &mut ObservabilityView, s: &Series) {
    match s.get("lapi_up") {
        Some(up) if up >= 1.0 => {
            let detail = if s.get("lapi_decisions_ok") == Some(1.0) {
                "Answers /health, and decision queries with the bouncer key."
            } else {
                "Answers /health."
            };
            view.checks.push(check("Local API", "ok", detail));
        }
        Some(_) => view.checks.push(check(
            "Local API",
            "warning",
            "Does not answer: no decision is taken or handed to the bouncers.",
        )),
        None => {}
    }

    let mut stale = Vec::new();
    let mut bouncers: Vec<Row> = s
        .idle
        .iter()
        .filter(|e| e.name == "bouncer_requests_total")
        .map(|e| {
            let name = label_of(e, "bouncer");
            let is_stale = e.value > BOUNCER_STALE_SECONDS;
            if is_stale {
                stale.push(name.clone());
            }
            Row {
                label: format!("{name}, last pull"),
                value: Some(e.value),
                unit: "seconds",
                state: Some(if is_stale { "warning" } else { "ok" }),
            }
        })
        .collect();
    bouncers.sort_by(|a, b| a.label.cmp(&b.label));
    if !stale.is_empty() {
        view.checks.push(check(
            "Bouncers",
            "warning",
            format!("Stopped pulling for more than 30 minutes: {}.", names(&stale)),
        ));
    } else if !bouncers.is_empty() {
        view.checks.push(check(
            "Bouncers",
            "ok",
            format!("{} pulling.", plural(bouncers.len() as f64, "bouncer", "bouncers")),
        ));
    }

    if let Some(read) = s.day("lines_read_total") {
        if read < 0.5 {
            view.checks.push(check(
                "Log reading",
                "warning",
                "No log line read in the last 24 hours: the acquisition is broken.",
            ));
        } else {
            let unparsed = s.day("lines_unparsed_total").unwrap_or(0.0);
            let share = unparsed / read * 100.0;
            view.checks.push(check(
                "Log reading",
                "ok",
                format!(
                    "{} read in the last 24 hours, {share:.0}% not recognised by any parser.",
                    plural(read, "line", "lines")
                ),
            ));
        }
    }

    view.figures.push(figure("Active decisions", s.get("decisions"), "count"));
    view.figures.push(figure("Alerts kept", s.get("alerts"), "count"));
    view.figures.push(figure(
        "Scenarios triggered in 24 h",
        s.day("scenario_overflows_total"),
        "count",
    ));
    view.figures.push(figure("Lines read", s.rate("lines_read_total"), "per_second"));

    if !bouncers.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Bouncers".to_string(),
            note: "Time since each bouncer last called the Local API.".to_string(),
            rows: bouncers,
        });
    }
    let origins: Vec<Row> = s
        .all("decisions_by_origin")
        .map(|e| row(label_of(e, "origin"), Some(e.value), "count"))
        .collect();
    if !origins.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Decisions by origin".to_string(),
            note: "crowdsec: this installation; CAPI and lists: the community blocklists."
                .to_string(),
            rows: origins,
        });
    }
    let sources: Vec<Row> = s
        .day
        .iter()
        .filter(|e| e.name == "source_lines_unparsed_total")
        .map(|e| {
            let source = label_of(e, "source");
            let total = s
                .day
                .iter()
                .find(|t| {
                    t.name == "source_lines_total"
                        && t.labels.get("source") == e.labels.get("source")
                })
                .map(|t| t.value);
            let share = total.filter(|t| *t > 0.0).map(|t| e.value / t * 100.0);
            row(format!("{source} not parsed"), share, "percent")
        })
        .collect();
    if !sources.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Log sources".to_string(),
            note: "Share of the lines of the last 24 hours no parser recognised.".to_string(),
            rows: sources,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serie(kind: &str, name: &str, value: f64, labels: &[(&str, &str)]) -> InstantSeries {
        let mut metric = BTreeMap::new();
        metric.insert("__name__".to_string(), format!("dumbmonit_{kind}_{name}"));
        metric.insert("target".to_string(), "4".to_string());
        for (k, v) in labels {
            metric.insert((*k).to_string(), (*v).to_string());
        }
        InstantSeries { metric, value: (1_700_000_000.0, value.to_string()) }
    }

    #[test]
    fn sans_mesure_la_vue_est_vide() {
        let view = build_view("redis", &Series::new("redis", &[], &[], &[], &[]));
        assert!(view.sampled_at.is_none());
        assert!(view.checks.is_empty());
    }

    #[test]
    fn une_replique_redis_coupee_de_son_primaire() {
        let k = "redis";
        let last = [
            serie(k, "version_info", 1.0, &[("version", "8.1.3"), ("product", "valkey")]),
            serie(k, "role_info", 1.0, &[("role", "replica")]),
            serie(k, "master_link_up", 0.0, &[]),
            serie(k, "memory_used_bytes", 1e6, &[]),
            serie(k, "rdb_last_save_ok", 0.0, &[]),
            serie(k, "db_keys", 12.0, &[("db", "db0")]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &[], &[]));
        assert_eq!(view.version.as_deref(), Some("Valkey 8.1.3"));
        assert_eq!(view.state, "warning");
        let replication = view.checks.iter().find(|c| c.label == "Replication").unwrap();
        assert_eq!(replication.state, "warning");
        let persistence = view.checks.iter().find(|c| c.label == "Persistence").unwrap();
        assert!(persistence.detail.starts_with("The last snapshot"), "{}", persistence.detail);
        assert_eq!(view.breakdowns[0].title, "Keyspace");
    }

    #[test]
    fn un_jeu_de_replicas_mongodb_sans_primaire() {
        let k = "mongodb";
        let last = [
            serie(k, "replset_primary_present", 0.0, &[]),
            serie(k, "replset_info", 1.0, &[("set", "rs0")]),
            serie(
                k,
                "replset_member_state",
                2.0,
                &[("member", "db1:27017"), ("state", "secondary")],
            ),
            serie(k, "replset_member_health", 1.0, &[("member", "db1:27017")]),
            serie(k, "replset_member_state", 8.0, &[("member", "db2:27017"), ("state", "down")]),
            serie(k, "replset_member_health", 0.0, &[("member", "db2:27017")]),
            serie(k, "connections_used_percent", 1.0, &[]),
        ];
        let day = [
            serie(k, "asserts_total", 3.0, &[("type", "regular")]),
            serie(k, "asserts_total", 40.0, &[("type", "user")]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &day, &[]));
        assert_eq!(view.state, "warning");
        assert!(view.checks[0].detail.contains("no primary"));
        let errors = view.checks.iter().find(|c| c.label == "Internal errors").unwrap();
        assert!(errors.detail.starts_with("3 regular"), "{}", errors.detail);
        let members = view.breakdowns.iter().find(|b| b.title.starts_with("Replica")).unwrap();
        assert_eq!(members.rows[1].state, Some("warning"));
    }

    #[test]
    fn rabbitmq_en_alarme_avec_une_file_sans_consommateur() {
        let k = "rabbitmq";
        let last = [
            serie(k, "alarm", 1.0, &[("node", "rabbit@a"), ("resource", "memory")]),
            serie(k, "alarm", 0.0, &[("node", "rabbit@a"), ("resource", "disk")]),
            serie(k, "nodes", 1.0, &[]),
            serie(k, "nodes_running", 1.0, &[]),
            serie(k, "queue_messages_ready", 7.0, &[("vhost", "/"), ("queue", "orders")]),
            serie(k, "queue_consumers", 0.0, &[("vhost", "/"), ("queue", "orders")]),
            serie(k, "queue_messages_ready", 0.0, &[("vhost", "shop"), ("queue", "mail")]),
            serie(k, "queue_consumers", 2.0, &[("vhost", "shop"), ("queue", "mail")]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &[], &[]));
        assert_eq!(view.state, "warning");
        assert!(view.checks[0].detail.contains("memory on rabbit@a"));
        let consumers = view.checks.iter().find(|c| c.label == "Consumers").unwrap();
        assert!(consumers.detail.contains("orders") && !consumers.detail.contains("mail"));
        let queues = view.breakdowns.iter().find(|b| b.title == "Fullest queues").unwrap();
        assert_eq!(queues.rows[0].label, "orders (0 consumers)");
        assert_eq!(queues.rows[1].label, "shop/mail (2 consumers)");
    }

    #[test]
    fn un_bouncer_crowdsec_arrete() {
        let k = "crowdsec";
        let last = [serie(k, "lapi_up", 1.0, &[]), serie(k, "decisions", 4.0, &[])];
        let idle = [
            serie(k, "bouncer_requests_total", 12.0, &[("bouncer", "firewall")]),
            serie(k, "bouncer_requests_total", 7200.0, &[("bouncer", "nginx")]),
        ];
        let day = [serie(k, "lines_read_total", 0.0, &[])];
        let view = build_view(k, &Series::new(k, &last, &[], &day, &idle));
        assert_eq!(view.state, "warning");
        let bouncers = view.checks.iter().find(|c| c.label == "Bouncers").unwrap();
        assert!(bouncers.detail.contains("nginx") && !bouncers.detail.contains("firewall"));
        let reading = view.checks.iter().find(|c| c.label == "Log reading").unwrap();
        assert_eq!(reading.state, "warning");
    }
}
