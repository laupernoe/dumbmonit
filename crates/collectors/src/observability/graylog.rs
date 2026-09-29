//! Graylog (Open, et les éditions payantes qui partagent la même API).
//!
//! Tout passe par l'API REST (`/api`), avec un compte au rôle intégré
//! **Reader** : il porte exactement les permissions de lecture utilisées ici
//! (`system:read`, `journal:read`, `indexercluster:read`, `inputs:read`,
//! `metrics:read`, `throughput:read`) et ne peut lire aucun message tant
//! qu'aucun flux ne lui est partagé. Seules les notifications de Graylog
//! demandent en plus `notifications:read` ; sans elle, elles sont sautées en
//! silence.
//!
//! La panne que ce module existe pour voir : **OpenSearch qui ne suit plus**.
//! Graylog continue d'accepter les messages, les range dans son journal disque,
//! et rien ne se voit dans l'interface de recherche sinon des messages qui
//! n'arrivent plus. Le journal non validé qui grossit, la santé du cluster
//! d'indexation et les échecs d'écriture le disent.
//!
//! Le journal, les tampons et le débit sont ceux **du nœud interrogé** : dans
//! une grappe, chaque nœud s'ajoute comme un équipement.

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, ProbeError, Sample, TargetId};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::warn;

use super::client::HttpClient;
use super::{counter, gauge};

pub const DEFAULT_PORT: u16 = 9000;

/// Au plus autant de notifications, et d'entrées en échec, portent une série
/// nommée : au-delà, seul leur nombre est compté.
const MAX_NAMED: usize = 32;

/// Les métriques internes lues en un seul appel groupé.
const INTERNAL_METRICS: [&str; 11] = [
    "org.graylog2.throughput.input",
    "org.graylog2.throughput.output",
    "org.graylog2.buffers.input.usage",
    "org.graylog2.buffers.input.size",
    "org.graylog2.buffers.process.usage",
    "org.graylog2.buffers.process.size",
    "org.graylog2.buffers.output.usage",
    "org.graylog2.buffers.output.size",
    "org.graylog2.outputs.BatchedMessageFilterOutput.outputWriteFailures",
    "org.graylog.failure.FailureSubmissionQueue.submittedFailures",
    "org.graylog2.indexer.messages.Messages.invalid-timestamps",
];

// ------------------------------------------------------------------ modèle

#[derive(Debug, Deserialize)]
pub struct SystemInfo {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub lifecycle: Option<String>,
    #[serde(default)]
    pub lb_status: Option<String>,
    #[serde(default)]
    pub is_processing: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Journal {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub uncommitted_journal_entries: Option<f64>,
    #[serde(default)]
    pub journal_size: Option<f64>,
    #[serde(default)]
    pub journal_size_limit: Option<f64>,
    #[serde(default)]
    pub append_events_per_second: Option<f64>,
    #[serde(default)]
    pub read_events_per_second: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct IndexerHealth {
    pub status: String,
    #[serde(default)]
    pub shards: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize)]
pub struct Notifications {
    #[serde(default)]
    pub notifications: Vec<Notification>,
}

#[derive(Debug, Deserialize)]
pub struct Notification {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub severity: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InputStates {
    #[serde(default)]
    pub states: Vec<InputState>,
}

#[derive(Debug, Deserialize)]
pub struct InputState {
    pub state: String,
    #[serde(default)]
    pub message_input: Option<InputSummary>,
}

#[derive(Debug, Deserialize)]
pub struct InputSummary {
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClusterNodes {
    #[serde(default)]
    pub nodes: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct MetricList {
    #[serde(default)]
    pub metrics: Vec<InternalMetric>,
}

#[derive(Debug, Deserialize)]
pub struct InternalMetric {
    pub full_name: String,
    #[serde(default)]
    pub metric: Value,
}

impl InternalMetric {
    /// La valeur d'une jauge, d'un compteur, ou le total d'un mètre.
    fn number(&self) -> Option<f64> {
        self.metric
            .get("value")
            .or_else(|| self.metric.get("count"))
            .or_else(|| self.metric.get("rate").and_then(|rate| rate.get("total")))
            .and_then(Value::as_f64)
    }
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    client: &HttpClient,
    target_id: TargetId,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let system: SystemInfo = client.get_json("/api/system").await?;
    let mut out = system_samples(&system, ts_ms);
    let mut errors = 0u32;

    let wanted = json!({ "metrics": INTERNAL_METRICS });
    let (journal, health, notifications, inputs, nodes, internal) = futures::join!(
        client.get_json::<Journal>("/api/system/journal"),
        client.get_raw("/api/system/indexer/cluster/health"),
        client.get_json::<Notifications>("/api/system/notifications"),
        client.get_json::<InputStates>("/api/system/inputstates"),
        client.get_json::<ClusterNodes>("/api/system/cluster/nodes"),
        client.post_json::<MetricList, _>("/api/system/metrics/multiple", &wanted),
    );

    if let Some(journal) = settle(journal, &mut errors, target_id, "journal") {
        out.extend(journal_samples(&journal, ts_ms));
    }
    match health {
        Ok(reply) if reply.status.is_success() => {
            match serde_json::from_str::<IndexerHealth>(&reply.body) {
                Ok(health) => out.extend(indexer_samples(Some(&health), ts_ms)),
                Err(error) => {
                    errors += 1;
                    warn!(target_id, %error, "santé d'OpenSearch illisible");
                }
            }
        }
        // Graylog répond 500 « Couldn't read Elasticsearch cluster health »
        // quand OpenSearch ne répond plus : c'est une mesure, pas un échec.
        Ok(reply) if reply.status.is_server_error() => out.extend(indexer_samples(None, ts_ms)),
        Ok(reply) => {
            errors += 1;
            let error = client.status_error(reply.status, &reply.body, "indexer/cluster/health");
            warn!(target_id, %error, "santé d'OpenSearch indisponible");
        }
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "santé d'OpenSearch indisponible");
        }
    }
    match notifications {
        Ok(list) => out.extend(notification_samples(&list, ts_ms)),
        // Permission facultative : sans `notifications:read`, rien à dire.
        Err(ProbeError::Auth(_)) => {}
        Err(error) => {
            errors += 1;
            warn!(target_id, %error, "notifications Graylog indisponibles");
        }
    }
    if let Some(inputs) = settle(inputs, &mut errors, target_id, "inputstates") {
        out.extend(input_samples(&inputs, ts_ms));
    }
    if let Some(nodes) = settle(nodes, &mut errors, target_id, "cluster/nodes") {
        out.push(gauge("graylog_cluster_nodes", nodes.nodes.len() as f64, ts_ms));
    }
    if let Some(internal) = settle(internal, &mut errors, target_id, "metrics/multiple") {
        out.extend(internal_samples(&internal, ts_ms));
    }
    out.push(gauge("graylog_scrape_errors", f64::from(errors), ts_ms));
    Ok(out)
}

fn settle<T>(
    outcome: Result<T, ProbeError>,
    errors: &mut u32,
    target_id: TargetId,
    path: &str,
) -> Option<T> {
    match outcome {
        Ok(value) => Some(value),
        Err(error) => {
            *errors += 1;
            warn!(target_id, path, %error, "appel Graylog indisponible");
            None
        }
    }
}

// ------------------------------------------------------------ traduction

/// `7.1.9+002c047` devient `7.1.9` : le suffixe est un commit, pas une version.
fn short_version(version: &str) -> &str {
    version.split(['+', ' ']).next().unwrap_or(version)
}

fn flag(name: &str, value: bool, ts_ms: i64) -> Sample {
    gauge(name, if value { 1.0 } else { 0.0 }, ts_ms)
}

pub fn system_samples(system: &SystemInfo, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(version) = system.version.as_deref().map(short_version) {
        out.push(gauge("graylog_version_info", 1.0, ts_ms).with_label("version", version));
    }
    if let Some(processing) = system.is_processing {
        out.push(flag("graylog_processing", processing, ts_ms));
    }
    if let Some(lb) = &system.lb_status {
        out.push(flag("graylog_lb_alive", lb.eq_ignore_ascii_case("alive"), ts_ms));
    }
    if let Some(lifecycle) = &system.lifecycle {
        out.push(flag("graylog_running", lifecycle.eq_ignore_ascii_case("running"), ts_ms));
    }
    out
}

pub fn journal_samples(journal: &Journal, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if journal.enabled == Some(false) {
        return out;
    }
    let pairs = [
        ("graylog_journal_uncommitted_entries", journal.uncommitted_journal_entries),
        ("graylog_journal_size_bytes", journal.journal_size),
        ("graylog_journal_size_limit_bytes", journal.journal_size_limit),
        ("graylog_journal_append_per_second", journal.append_events_per_second),
        ("graylog_journal_read_per_second", journal.read_events_per_second),
    ];
    for (name, value) in pairs {
        if let Some(value) = value {
            out.push(gauge(name, value, ts_ms));
        }
    }
    if let (Some(size), Some(limit)) = (journal.journal_size, journal.journal_size_limit)
        && limit > 0.0
    {
        out.push(gauge("graylog_journal_used_percent", size / limit * 100.0, ts_ms));
    }
    out
}

/// 0 vert, 1 jaune, 2 rouge, 3 injoignable depuis Graylog.
pub fn indexer_samples(health: Option<&IndexerHealth>, ts_ms: i64) -> Vec<Sample> {
    let Some(health) = health else {
        return vec![gauge("graylog_indexer_status", 3.0, ts_ms)];
    };
    let status = match health.status.to_ascii_lowercase().as_str() {
        "green" => 0.0,
        "yellow" => 1.0,
        _ => 2.0,
    };
    let mut out = vec![gauge("graylog_indexer_status", status, ts_ms)];
    for (state, count) in &health.shards {
        out.push(gauge("graylog_indexer_shards", *count, ts_ms).with_label("state", state));
    }
    out
}

pub fn notification_samples(list: &Notifications, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge("graylog_notifications", list.notifications.len() as f64, ts_ms)];
    let urgent =
        list.notifications.iter().filter(|n| n.severity.as_deref() == Some("urgent")).count();
    out.push(gauge("graylog_notifications_urgent", urgent as f64, ts_ms));
    for notification in list.notifications.iter().take(MAX_NAMED) {
        out.push(
            gauge("graylog_notification", 1.0, ts_ms)
                .with_label("type", notification.kind.as_str())
                .with_label("severity", notification.severity.as_deref().unwrap_or("normal")),
        );
    }
    out
}

pub fn input_samples(inputs: &InputStates, ts_ms: i64) -> Vec<Sample> {
    let failed: Vec<&InputState> = inputs
        .states
        .iter()
        .filter(|input| matches!(input.state.as_str(), "FAILED" | "FAILING"))
        .collect();
    let running = inputs.states.iter().filter(|input| input.state == "RUNNING").count();
    let mut out = vec![
        gauge("graylog_inputs_running", running as f64, ts_ms),
        gauge("graylog_inputs_failed", failed.len() as f64, ts_ms),
    ];
    for input in failed.into_iter().take(MAX_NAMED) {
        let title = input
            .message_input
            .as_ref()
            .and_then(|summary| summary.title.as_deref())
            .unwrap_or("unnamed input");
        out.push(gauge("graylog_input_failed", 1.0, ts_ms).with_label("input", title));
    }
    out
}

pub fn internal_samples(list: &MetricList, ts_ms: i64) -> Vec<Sample> {
    let get = |name: &str| {
        list.metrics.iter().find(|metric| metric.full_name == name).and_then(InternalMetric::number)
    };
    let mut out = Vec::new();
    let counters = [
        ("graylog_messages_in_total", "org.graylog2.throughput.input"),
        ("graylog_messages_out_total", "org.graylog2.throughput.output"),
        (
            "graylog_output_failures_total",
            "org.graylog2.outputs.BatchedMessageFilterOutput.outputWriteFailures",
        ),
        (
            "graylog_processing_failures_total",
            "org.graylog.failure.FailureSubmissionQueue.submittedFailures",
        ),
        (
            "graylog_invalid_timestamps_total",
            "org.graylog2.indexer.messages.Messages.invalid-timestamps",
        ),
    ];
    for (name, source) in counters {
        if let Some(value) = get(source) {
            out.push(counter(name, value, ts_ms));
        }
    }
    for buffer in ["input", "process", "output"] {
        let usage = get(&format!("org.graylog2.buffers.{buffer}.usage"));
        let size = get(&format!("org.graylog2.buffers.{buffer}.size"));
        if let (Some(usage), Some(size)) = (usage, size)
            && size > 0.0
        {
            out.push(
                Sample::new(
                    "graylog_buffer_used_percent",
                    usage / size * 100.0,
                    MetricKind::Gauge,
                    ts_ms,
                )
                .with_label("buffer", buffer),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Réponses réelles d'un Graylog 7.1.9 (OpenSearch 2.19.6) interrogé avec
    // un compte au rôle Reader, d'abord en bonne santé puis OpenSearch arrêté.
    // Les identifiants de nœud et de grappe, le nom d'hôte et l'adresse de
    // transport ont été remplacés ; le reste est intact.
    const SYSTEM: &str = include_str!("testdata/graylog_7.1.9/system.json");
    const JOURNAL: &str = include_str!("testdata/graylog_7.1.9/journal.json");
    const JOURNAL_STALLED: &str =
        include_str!("testdata/graylog_7.1.9/journal_opensearch_down.json");
    const HEALTH: &str = include_str!("testdata/graylog_7.1.9/indexer_cluster_health.json");
    const HEALTH_DOWN: &str =
        include_str!("testdata/graylog_7.1.9/indexer_cluster_health_opensearch_down.json");
    const NOTIFICATIONS: &str = include_str!("testdata/graylog_7.1.9/notifications.json");
    const INPUTS: &str = include_str!("testdata/graylog_7.1.9/inputstates.json");
    const NODES: &str = include_str!("testdata/graylog_7.1.9/cluster_nodes.json");
    const METRICS: &str = include_str!("testdata/graylog_7.1.9/metrics_multiple.json");
    const METRICS_STALLED: &str =
        include_str!("testdata/graylog_7.1.9/metrics_multiple_opensearch_down.json");

    fn parse<T: serde::de::DeserializeOwned>(body: &str) -> T {
        serde_json::from_str(body).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn le_systeme_dit_version_et_traitement() {
        let samples = system_samples(&parse(SYSTEM), 0);
        let version = samples.iter().find(|s| s.metric == "graylog_version_info").unwrap();
        assert_eq!(version.labels["version"], "7.1.9");
        assert_eq!(value(&samples, "graylog_processing"), Some(1.0));
        assert_eq!(value(&samples, "graylog_lb_alive"), Some(1.0));
        assert_eq!(value(&samples, "graylog_running"), Some(1.0));
    }

    #[test]
    fn le_journal_qui_grossit_se_voit_quand_opensearch_tombe() {
        let healthy = journal_samples(&parse(JOURNAL), 0);
        assert_eq!(value(&healthy, "graylog_journal_uncommitted_entries"), Some(0.0));
        assert_eq!(value(&healthy, "graylog_journal_size_limit_bytes"), Some(5_368_709_120.0));
        let stalled = journal_samples(&parse(JOURNAL_STALLED), 0);
        assert!(value(&stalled, "graylog_journal_uncommitted_entries").unwrap() > 0.0);
        let used = value(&stalled, "graylog_journal_used_percent").unwrap();
        assert!(used > 0.0 && used < 1.0, "{used}");
    }

    #[test]
    fn la_sante_d_opensearch_et_son_absence() {
        let health: IndexerHealth = parse(HEALTH);
        let samples = indexer_samples(Some(&health), 0);
        assert_eq!(value(&samples, "graylog_indexer_status"), Some(0.0));
        assert!(samples.iter().any(|s| s.metric == "graylog_indexer_shards"
            && s.labels["state"] == "unassigned"
            && s.value == 0.0));
        // Le corps d'erreur réel que Graylog rend avec son 500 n'est pas une
        // santé : il ne se désérialise pas, et la sonde le traite comme
        // « injoignable ».
        assert!(serde_json::from_str::<IndexerHealth>(HEALTH_DOWN).is_err());
        assert_eq!(value(&indexer_samples(None, 0), "graylog_indexer_status"), Some(3.0));
    }

    #[test]
    fn les_notifications_et_les_entrees_en_echec_sont_nommees() {
        let samples = notification_samples(&parse(NOTIFICATIONS), 0);
        assert_eq!(value(&samples, "graylog_notifications"), Some(2.0));
        assert_eq!(value(&samples, "graylog_notifications_urgent"), Some(1.0));
        assert!(
            samples.iter().any(|s| s.metric == "graylog_notification"
                && s.labels["type"] == "input_failed_to_start")
        );

        let samples = input_samples(&parse(INPUTS), 0);
        assert_eq!(value(&samples, "graylog_inputs_running"), Some(1.0));
        assert_eq!(value(&samples, "graylog_inputs_failed"), Some(1.0));
        let failed = samples.iter().find(|s| s.metric == "graylog_input_failed").unwrap();
        assert_eq!(failed.labels["input"], "syslog-tcp");
    }

    #[test]
    fn les_metriques_internes_donnent_debit_tampons_et_echecs() {
        let samples = internal_samples(&parse(METRICS), 0);
        assert!(value(&samples, "graylog_messages_in_total").unwrap() > 0.0);
        assert!(value(&samples, "graylog_messages_out_total").unwrap() > 0.0);
        assert_eq!(value(&samples, "graylog_output_failures_total"), Some(0.0));
        assert_eq!(value(&samples, "graylog_processing_failures_total"), Some(0.0));
        let buffers: Vec<_> =
            samples.iter().filter(|s| s.metric == "graylog_buffer_used_percent").collect();
        assert_eq!(buffers.len(), 3);
        let stalled = internal_samples(&parse(METRICS_STALLED), 0);
        let out_ = value(&stalled, "graylog_messages_out_total").unwrap();
        let in_ = value(&stalled, "graylog_messages_in_total").unwrap();
        assert!(in_ > out_, "OpenSearch arrêté : plus d'entrées que de sorties");
        let nodes: ClusterNodes = parse(NODES);
        assert_eq!(nodes.nodes.len(), 1);
    }

    #[test]
    fn la_version_perd_son_suffixe_de_commit() {
        assert_eq!(short_version("7.1.9+002c047"), "7.1.9");
        assert_eq!(short_version("6.3.5"), "6.3.5");
    }
}
