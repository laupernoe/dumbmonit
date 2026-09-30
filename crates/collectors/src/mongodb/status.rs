//! Traduction de `serverStatus` et `replSetGetStatus` en mesures.

use dumbmonit_proto::{MetricKind, Sample};

use super::bson::{Bson, Document};

/// Nombre maximal de membres de jeu de réplicas décrits un par un (MongoDB en
/// accepte cinquante au plus).
const MAX_MEMBERS: usize = 50;

/// Compteurs d'opérations publiés, dans l'ordre de `serverStatus.opcounters`.
const OPCOUNTERS: [&str; 6] = ["insert", "query", "update", "delete", "getmore", "command"];

/// Familles d'assertions. `regular` est une erreur interne du serveur ; `user`
/// une erreur renvoyée à un client (doublon de clé, requête invalide).
const ASSERTS: [&str; 6] = ["regular", "warning", "msg", "user", "tripwire", "rollovers"];

pub fn server_samples(status: &Document, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let counter = |name: &str, value: f64| Sample::new(name, value, MetricKind::Counter, ts_ms);

    out.push(
        gauge("mongodb_version_info", 1.0)
            .with_label("version", status.str("version").unwrap_or(""))
            .with_label("process", status.str("process").unwrap_or("mongod"))
            .with_label("storage_engine", status.str("storageEngine/name").unwrap_or("")),
    );
    out.push(gauge("mongodb_role_info", 1.0).with_label("role", role(status)));

    let mut push = |name: &str, path: &str, kind: MetricKind| {
        if let Some(value) = status.number(path) {
            out.push(Sample::new(name, value, kind, ts_ms));
        }
    };
    use MetricKind::{Counter, Gauge};
    push("mongodb_uptime_seconds", "uptime", Gauge);
    push("mongodb_connections_current", "connections/current", Gauge);
    push("mongodb_connections_available", "connections/available", Gauge);
    push("mongodb_connections_active", "connections/active", Gauge);
    push("mongodb_connections_created_total", "connections/totalCreated", Counter);
    push("mongodb_connections_rejected_total", "connections/rejected", Counter);
    push("mongodb_queued_operations", "globalLock/currentQueue/total", Gauge);
    push("mongodb_active_clients", "globalLock/activeClients/total", Gauge);
    push("mongodb_network_bytes_in_total", "network/bytesIn", Counter);
    push("mongodb_network_bytes_out_total", "network/bytesOut", Counter);
    push("mongodb_cursors_open", "metrics/cursor/open/total", Gauge);
    push("mongodb_cursors_timed_out_total", "metrics/cursor/timedOut", Counter);
    push("mongodb_documents_deleted_total", "metrics/document/deleted", Counter);

    if let (Some(current), Some(available)) =
        (status.number("connections/current"), status.number("connections/available"))
        && current + available > 0.0
    {
        out.push(gauge(
            "mongodb_connections_used_percent",
            current / (current + available) * 100.0,
        ));
    }

    // `mem.resident` est en Mio.
    if let Some(resident) = status.number("mem/resident") {
        out.push(gauge("mongodb_memory_resident_bytes", resident * 1024.0 * 1024.0));
    }

    for op in OPCOUNTERS {
        if let Some(value) = status.number(&format!("opcounters/{op}")) {
            out.push(counter("mongodb_operations_total", value).with_label("op", op));
        }
    }
    for kind in ASSERTS {
        if let Some(value) = status.number(&format!("asserts/{kind}")) {
            out.push(counter("mongodb_asserts_total", value).with_label("type", kind));
        }
    }

    // Cache WiredTiger : taille, occupation, part de pages modifiées. Au-delà
    // de 95 % d'occupation ou de 20 % de pages modifiées, WiredTiger fait
    // évincer les threads applicatifs eux-mêmes, et les requêtes ralentissent.
    let cache = |key: &str| status.number(&format!("wiredTiger/cache/{key}"));
    let max = cache("maximum bytes configured").filter(|max| *max > 0.0);
    let used = cache("bytes currently in the cache");
    let dirty = cache("tracked dirty bytes in the cache");
    if let Some(max) = max {
        out.push(gauge("mongodb_wiredtiger_cache_max_bytes", max));
        if let Some(used) = used {
            out.push(gauge("mongodb_wiredtiger_cache_used_bytes", used));
            out.push(gauge("mongodb_wiredtiger_cache_used_percent", used / max * 100.0));
        }
        if let Some(dirty) = dirty {
            out.push(gauge("mongodb_wiredtiger_cache_dirty_bytes", dirty));
            out.push(gauge("mongodb_wiredtiger_cache_dirty_percent", dirty / max * 100.0));
        }
    }
    if let Some(read) = cache("bytes read into cache") {
        out.push(counter("mongodb_wiredtiger_cache_read_bytes_total", read));
    }
    out
}

/// `primary`, `secondary`, `arbiter`, `standalone` ou `mongos`.
pub fn role(status: &Document) -> &'static str {
    if status.str("process") == Some("mongos") {
        return "mongos";
    }
    let Some(repl) = status.doc("repl") else { return "standalone" };
    if repl.get("setName").is_none() {
        return "standalone";
    }
    if repl.path("isWritablePrimary") == Some(&Bson::Bool(true))
        || repl.path("ismaster") == Some(&Bson::Bool(true))
    {
        "primary"
    } else if repl.path("arbiterOnly") == Some(&Bson::Bool(true)) {
        "arbiter"
    } else if repl.path("secondary") == Some(&Bson::Bool(true)) {
        "secondary"
    } else {
        "other"
    }
}

/// Vrai si le serveur fait partie d'un jeu de réplicas.
pub fn in_replica_set(status: &Document) -> bool {
    status.doc("repl").is_some_and(|repl| repl.get("setName").is_some())
}

/// Nom lisible d'un état de membre (`replSetGetStatus.members[].state`).
fn state_name(state: f64) -> &'static str {
    match state as i64 {
        0 => "startup",
        1 => "primary",
        2 => "secondary",
        3 => "recovering",
        5 => "startup2",
        6 => "unknown",
        7 => "arbiter",
        8 => "down",
        9 => "rollback",
        10 => "removed",
        _ => "other",
    }
}

fn date_ms(value: Option<&Bson>) -> Option<i64> {
    match value? {
        Bson::DateTime(ms) => Some(*ms),
        _ => None,
    }
}

pub fn replset_samples(status: &Document, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let members: Vec<&Document> = status
        .array("members")
        .unwrap_or_default()
        .iter()
        .filter_map(Bson::as_document)
        .take(MAX_MEMBERS)
        .collect();

    let primary_optime = members
        .iter()
        .find(|m| m.number("state") == Some(1.0))
        .and_then(|m| date_ms(m.get("optimeDate")));
    out.push(gauge(
        "mongodb_replset_primary_present",
        f64::from(u8::from(primary_optime.is_some())),
    ));
    if let Some(set) = status.str("set") {
        out.push(
            gauge("mongodb_replset_info", 1.0)
                .with_label("set", set)
                .with_label("members", members.len().to_string()),
        );
    }

    for member in members {
        let Some(name) = member.str("name") else { continue };
        let state = member.number("state").unwrap_or(6.0);
        out.push(
            gauge("mongodb_replset_member_state", state)
                .with_label("member", name)
                .with_label("state", state_name(state)),
        );
        if let Some(health) = member.number("health") {
            out.push(gauge("mongodb_replset_member_health", health).with_label("member", name));
        }
        // Le retard n'a de sens que pour une réplique qui porte des données :
        // un arbitre n'a pas d'oplog.
        if state == 2.0
            && let (Some(primary), Some(own)) = (primary_optime, date_ms(member.get("optimeDate")))
        {
            let lag = ((primary - own) as f64 / 1000.0).max(0.0);
            out.push(gauge("mongodb_replset_member_lag_seconds", lag).with_label("member", name));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn doc(bytes: &[u8]) -> Document {
        Document::decode(bytes).unwrap()
    }

    /// `serverStatus` d'un MongoDB 8.0.13, membre unique (primaire) d'un jeu
    /// de réplicas, lu par un compte `clusterMonitor`.
    const SERVER_STATUS: &[u8] = include_bytes!("testdata/mongodb_8.0.13_server_status.bson");
    /// `replSetGetStatus` du même serveur.
    const REPLSET: &[u8] = include_bytes!("testdata/mongodb_8.0.13_replset_status.bson");

    #[test]
    fn server_status_reel() {
        let status = doc(SERVER_STATUS);
        let samples = server_samples(&status, 0);
        let version = samples.iter().find(|s| s.metric == "mongodb_version_info").unwrap();
        assert_eq!(version.labels["version"], "8.0.13");
        assert_eq!(version.labels["storage_engine"], "wiredTiger");
        assert_eq!(role(&status), "primary");
        assert!(in_replica_set(&status));
        assert!(value(&samples, "mongodb_connections_current").unwrap() >= 1.0);
        let percent = value(&samples, "mongodb_connections_used_percent").unwrap();
        assert!(percent > 0.0 && percent < 5.0, "{percent}");
        let inserts = samples
            .iter()
            .find(|s| s.metric == "mongodb_operations_total" && s.labels["op"] == "insert")
            .unwrap();
        assert!(inserts.value >= 500.0, "les 500 insertions du jeu d'essai");
        assert_eq!(inserts.kind, MetricKind::Counter);
        assert_eq!(samples.iter().filter(|s| s.metric == "mongodb_asserts_total").count(), 6);
        let cache = value(&samples, "mongodb_wiredtiger_cache_used_percent").unwrap();
        assert!(cache > 0.0 && cache < 100.0, "{cache}");
        assert!(value(&samples, "mongodb_wiredtiger_cache_dirty_percent").is_some());
        assert!(value(&samples, "mongodb_memory_resident_bytes").unwrap() > 1e6);
    }

    #[test]
    fn replset_reel() {
        let samples = replset_samples(&doc(REPLSET), 0);
        assert_eq!(value(&samples, "mongodb_replset_primary_present"), Some(1.0));
        let info = samples.iter().find(|s| s.metric == "mongodb_replset_info").unwrap();
        assert_eq!(info.labels["set"], "rs0");
        let state = samples.iter().find(|s| s.metric == "mongodb_replset_member_state").unwrap();
        assert_eq!(state.value, 1.0);
        assert_eq!(state.labels["state"], "primary");
        assert_eq!(value(&samples, "mongodb_replset_member_health"), Some(1.0));
        // Le primaire n'a pas de retard sur lui-même.
        assert!(value(&samples, "mongodb_replset_member_lag_seconds").is_none());
    }

    #[test]
    fn une_replique_en_retard_et_un_membre_injoignable() {
        let member = |name: &str, state: i32, health: f64, optime: i64| {
            Bson::Document(
                Document::new()
                    .with("name", Bson::String(name.into()))
                    .with("state", Bson::Int32(state))
                    .with("health", Bson::Double(health))
                    .with("optimeDate", Bson::DateTime(optime)),
            )
        };
        let status = Document::new().with("set", Bson::String("rs0".into())).with(
            "members",
            Bson::Array(vec![
                member("db1:27017", 1, 1.0, 1_000_000),
                member("db2:27017", 2, 1.0, 880_000),
                member("db3:27017", 8, 0.0, 0),
            ]),
        );
        let samples = replset_samples(&status, 0);
        let lag =
            samples.iter().find(|s| s.metric == "mongodb_replset_member_lag_seconds").unwrap();
        assert_eq!((lag.labels["member"].as_str(), lag.value), ("db2:27017", 120.0));
        let down = samples
            .iter()
            .find(|s| {
                s.metric == "mongodb_replset_member_health" && s.labels["member"] == "db3:27017"
            })
            .unwrap();
        assert_eq!(down.value, 0.0);
        let state = samples
            .iter()
            .find(|s| {
                s.metric == "mongodb_replset_member_state" && s.labels["member"] == "db3:27017"
            })
            .unwrap();
        assert_eq!(state.labels["state"], "down");

        // Plus de primaire : l'élection a échoué, plus aucune écriture.
        let status = Document::new()
            .with("members", Bson::Array(vec![member("db2:27017", 2, 1.0, 880_000)]));
        assert_eq!(
            value(&replset_samples(&status, 0), "mongodb_replset_primary_present"),
            Some(0.0)
        );
    }

    #[test]
    fn un_serveur_isole() {
        let status = Document::new()
            .with("version", Bson::String("7.0.2".into()))
            .with("process", Bson::String("mongod".into()));
        assert_eq!(role(&status), "standalone");
        assert!(!in_replica_set(&status));
    }
}
