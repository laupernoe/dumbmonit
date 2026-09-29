//! Le panneau des serveurs de journaux et de métriques : VictoriaMetrics,
//! VictoriaLogs, Loki et Graylog (`collectors/observability`).
//!
//! `GET /targets/{id}/observability` reconstruit la vue à partir des séries de
//! la cible dans VictoriaMetrics, sans jamais réinterroger le serveur : les
//! dernières valeurs, les débits sur dix minutes et ce qui a été refusé sur
//! vingt-quatre heures. Le jugement (qu'est-ce qui va, qu'est-ce qui ne va pas)
//! est rendu ici, en phrases ; l'interface ne fait que l'afficher, ce qui lui
//! permet d'être la même pour les quatre produits.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::Serialize;

use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

/// Les types servis par ce panneau, et le préfixe de leurs séries.
pub const KINDS: [&str; 4] = ["victoriametrics", "victorialogs", "loki", "graylog"];

/// Profondeur de recherche de la dernière valeur.
const LOOKBACK: &str = "1h";

/// Au-delà, la part d'insertions lentes dit que la mémoire ne suffit plus aux
/// séries actives (seuil de la documentation de VictoriaMetrics).
const SLOW_INSERTS_PERCENT: f64 = 5.0;

pub fn routes() -> Router<AppState> {
    Router::new().route("/targets/{id}/observability", get(overview))
}

// --------------------------------------------------------------------- vues

#[derive(Debug, Default, Serialize)]
pub struct ObservabilityView {
    pub kind: String,
    pub version: Option<String>,
    /// Horodatage (secondes Unix) de la mesure la plus récente ; `null` sans mesure.
    pub sampled_at: Option<f64>,
    /// Pire état des vérifications : `ok`, `advisory` ou `warning`.
    pub state: String,
    /// Ce qui va et ce qui ne va pas, dans l'ordre où le lire.
    pub checks: Vec<Check>,
    /// Les chiffres qui disent ce que le serveur fait.
    pub figures: Vec<Figure>,
    /// Des détails par raison, par disque, par tampon…
    pub breakdowns: Vec<Breakdown>,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub label: String,
    /// `ok`, `advisory` ou `warning`.
    pub state: &'static str,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct Figure {
    pub label: String,
    pub value: Option<f64>,
    /// `bytes`, `bytes_per_second`, `per_second`, `percent`, `count`, `seconds`.
    pub unit: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Breakdown {
    pub title: String,
    /// Une phrase sous le titre ; vide si inutile.
    pub note: String,
    pub rows: Vec<Row>,
}

#[derive(Debug, Serialize)]
pub struct Row {
    pub label: String,
    pub value: Option<f64>,
    pub unit: &'static str,
    /// `ok`, `advisory`, `warning` ; `null` pour une ligne sans verdict.
    pub state: Option<&'static str>,
}

// ------------------------------------------------------------- gestionnaire

async fn overview(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<ObservabilityView>> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device {id} not found.")))?;
    let kind = target.kind.as_str();
    if !KINDS.contains(&kind) {
        return Err(ApiError::NotFound(format!("Device {id} is not a log or metrics server.")));
    }
    let selector = format!(r#"{{__name__=~"dumbmonit_{kind}_.*", target="{id}"}}"#);
    let counters = format!(r#"{{__name__=~"dumbmonit_{kind}_.*_total", target="{id}"}}"#);
    let last = format!("last_over_time({selector}[{LOOKBACK}]) keep_metric_names");
    let rates = format!("rate({counters}[10m]) keep_metric_names");
    let day = format!("increase_prometheus({counters}[24h]) keep_metric_names");
    let (last, rates, day) = tokio::try_join!(
        state.victoria.query(&last),
        state.victoria.query(&rates),
        state.victoria.query(&day),
    )?;
    Ok(Json(build_view(kind, &Series::new(kind, &last, &rates, &day))))
}

// ------------------------------------------------------------- assemblage

/// Les séries de la cible, rangées par nom court (sans `dumbmonit_<kind>_`).
#[derive(Default)]
pub struct Series {
    last: Vec<Entry>,
    rate: Vec<Entry>,
    day: Vec<Entry>,
    sampled_at: Option<f64>,
}

struct Entry {
    name: String,
    labels: BTreeMap<String, String>,
    value: f64,
}

impl Series {
    pub fn new(
        kind: &str,
        last: &[InstantSeries],
        rate: &[InstantSeries],
        day: &[InstantSeries],
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
        Self { last: convert(last), rate: convert(rate), day: convert(day), sampled_at }
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
    fn day_by(&self, name: &str, label: &str) -> Vec<(String, f64)> {
        self.day
            .iter()
            .filter(|e| e.name == name)
            .map(|e| (e.labels.get(label).cloned().unwrap_or_default(), e.value))
            .collect()
    }
    fn label(&self, name: &str, label: &str) -> Option<String> {
        self.all(name).next().and_then(|e| e.labels.get(label).cloned())
    }
}

/// La somme des séries d'un nom (une seule, le plus souvent).
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

/// `too_small_timestamp` devient « too small timestamp ».
fn words(reason: &str) -> String {
    reason.replace('_', " ")
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
        "victoriametrics" | "victorialogs" => victoria(&mut view, s, kind == "victoriametrics"),
        "loki" => loki(&mut view, s),
        _ => graylog(&mut view, s),
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

/// Les lignes refusées sur vingt-quatre heures, par raison, et leur verdict.
fn rejected(
    view: &mut ObservabilityView,
    s: &Series,
    total: &str,
    by_reason: &str,
    label: &str,
    what: &str,
) {
    let refused = s.day(total);
    let (state, detail) = match refused {
        None => ("ok", format!("Needs two measurements a day apart before it can count {what}.")),
        Some(n) if n >= 0.5 => {
            ("advisory", format!("{n:.0} {what} refused in the last 24 hours: that data is lost."))
        }
        Some(_) => ("ok", "Nothing refused in the last 24 hours.".to_string()),
    };
    view.checks.push(check(label, state, detail));
    let mut reasons: Vec<(String, f64)> =
        s.day_by(by_reason, "reason").into_iter().filter(|(_, n)| *n >= 0.5).collect();
    reasons.sort_by(|a, b| b.1.total_cmp(&a.1));
    if !reasons.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Refused in the last 24 hours".to_string(),
            note: "By the reason the server gives.".to_string(),
            rows: reasons
                .into_iter()
                .map(|(reason, n)| Row {
                    label: words(&reason),
                    value: Some(n),
                    unit: "count",
                    state: Some("advisory"),
                })
                .collect(),
        });
    }
}

fn victoria(view: &mut ObservabilityView, s: &Series, metrics: bool) {
    match s.get("healthy") {
        Some(h) if h >= 1.0 => view.checks.push(check("Health", "ok", "Answers OK on /health.")),
        Some(_) => view.checks.push(check(
            "Health",
            "warning",
            "Reachable, but /health does not answer OK.",
        )),
        None => {}
    }

    // Le disque : lecture seule, ou marge au-dessus du seuil.
    let read_only = s.get("read_only").unwrap_or(0.0) >= 1.0;
    let headroom = s.all("disk_headroom_percent").map(|e| e.value).reduce(f64::min);
    if read_only {
        view.checks.push(check(
            "Storage",
            "warning",
            "Read-only: free disk space fell below -storage.minFreeDiskSpaceBytes, and every new \
             write is refused until space is freed.",
        ));
    } else if let Some(headroom) = headroom {
        let state = if headroom < 10.0 { "advisory" } else { "ok" };
        view.checks.push(check(
            "Storage",
            state,
            format!(
                "{headroom:.1}% of the disk left before the server switches to read-only and \
                 refuses new data."
            ),
        ));
    }

    let plural_what = if metrics { "samples" } else { "log lines" };
    rejected(
        view,
        s,
        "rows_rejected_total",
        "rows_rejected_reason_total",
        "Refused data",
        plural_what,
    );

    if metrics
        && let (Some(slow), Some(added)) =
            (s.rate("slow_inserts_total"), s.rate("rows_added_total"))
        && added > 0.0
    {
        let share = slow / added * 100.0;
        let state = if share > SLOW_INSERTS_PERCENT { "advisory" } else { "ok" };
        view.checks.push(check(
            "Slow inserts",
            state,
            format!(
                "{share:.1}% of new samples took the slow path over the last ten minutes. \
                     Above {SLOW_INSERTS_PERCENT}% for long, the server lacks memory for its \
                     active series."
            ),
        ));
    }
    if let Some(errors) = s.day("log_errors_total").filter(|n| *n >= 0.5) {
        view.checks.push(check(
            "Error log",
            "advisory",
            format!("{} logged in the last 24 hours.", plural(errors, "error", "errors")),
        ));
    }

    view.figures.push(figure(
        if metrics { "Samples ingested" } else { "Log lines ingested" },
        s.rate("rows_ingested_total"),
        "per_second",
    ));
    if metrics {
        view.figures.push(figure("Active series", s.get("active_series"), "count"));
    } else {
        view.figures.push(figure(
            "Ingested volume",
            s.rate("bytes_ingested_total"),
            "bytes_per_second",
        ));
    }
    view.figures.push(figure("Data on disk", s.get("data_size_bytes"), "bytes"));
    view.figures.push(figure("Up for", s.get("uptime_seconds"), "seconds"));

    let mut disks = Vec::new();
    for entry in s.all("disk_free_bytes") {
        let path = entry.labels.get("path").cloned().unwrap_or_default();
        let limit = s
            .all("disk_free_limit_bytes")
            .find(|e| e.labels.get("path") == Some(&path))
            .map(|e| e.value);
        let state = match limit {
            Some(limit) if entry.value <= limit => Some("warning"),
            _ => Some("ok"),
        };
        disks.push(Row {
            label: format!("{path} free"),
            value: Some(entry.value),
            unit: "bytes",
            state,
        });
        if let Some(limit) = limit {
            disks.push(row(format!("{path} read-only below"), Some(limit), "bytes"));
        }
    }
    if !disks.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Disk".to_string(),
            note: "Free space against the -storage.minFreeDiskSpaceBytes limit.".to_string(),
            rows: disks,
        });
    }
}

fn loki(view: &mut ObservabilityView, s: &Series) {
    match s.get("ready") {
        Some(r) if r >= 1.0 => view.checks.push(check("Ready", "ok", "Answers ready on /ready.")),
        Some(_) => view.checks.push(check(
            "Ready",
            "warning",
            "Reachable, but /ready does not answer ready: Loki is starting, or a component is \
             not able to write or read.",
        )),
        None => {}
    }
    rejected(
        view,
        s,
        "discarded_lines_total",
        "discarded_lines_reason_total",
        "Refused lines",
        "log lines",
    );

    if let Some(failures) = s.day("flush_failures_total") {
        view.checks.push(if failures >= 0.5 {
            check(
                "Flush to storage",
                "warning",
                format!(
                    "{} to write to storage in the last 24 hours. Chunks that cannot be written \
                     stay in memory and are lost if Loki restarts.",
                    plural(failures, "chunk failed", "chunks failed")
                ),
            )
        } else {
            check("Flush to storage", "ok", "Every chunk reached storage in the last 24 hours.")
        });
    }
    if let Some(full) = s.day("wal_disk_full_failures_total").filter(|n| *n >= 0.5) {
        view.checks.push(check(
            "Write-ahead log",
            "warning",
            format!(
                "{} could not be logged because the disk was full.",
                plural(full, "write", "writes")
            ),
        ));
    }
    if let Some(unhealthy) = s.get("ring_unhealthy_members").filter(|n| *n >= 1.0) {
        view.checks.push(check(
            "Ring",
            "warning",
            format!(
                "{} in the hash ring.",
                plural(unhealthy, "unhealthy member", "unhealthy members")
            ),
        ));
    }
    if let (Some(errors), Some(requests)) =
        (s.rate("request_errors_total"), s.rate("requests_total"))
        && errors > 0.0
    {
        let share = if requests > 0.0 { errors / requests * 100.0 } else { 100.0 };
        view.checks.push(check(
            "Requests",
            "advisory",
            format!(
                "{share:.1}% of requests failed with a server error over the last ten minutes."
            ),
        ));
    }

    view.figures.push(figure("Lines received", s.rate("received_lines_total"), "per_second"));
    view.figures.push(figure(
        "Volume received",
        s.rate("received_bytes_total"),
        "bytes_per_second",
    ));
    view.figures.push(figure("Streams in memory", s.get("memory_streams"), "count"));
    view.figures.push(figure("WAL disk used", s.get("wal_disk_used_percent"), "percent"));
}

fn graylog(view: &mut ObservabilityView, s: &Series) {
    match s.get("processing") {
        Some(p) if p >= 1.0 => view.checks.push(check("Processing", "ok", "Processing messages.")),
        Some(_) => view.checks.push(check(
            "Processing",
            "warning",
            "Message processing is paused on this node: messages pile up in the journal.",
        )),
        None => {}
    }
    if s.get("lb_alive").is_some_and(|alive| alive < 1.0) {
        view.checks.push(check(
            "Load balancer status",
            "advisory",
            "The node reports itself dead or throttled to load balancers.",
        ));
    }
    match s.get("indexer_status").map(|v| v.round() as i64) {
        Some(0) => view.checks.push(check("Search cluster", "ok", "OpenSearch reports green.")),
        Some(1) => view.checks.push(check(
            "Search cluster",
            "advisory",
            "OpenSearch reports yellow: some replica shards are not assigned.",
        )),
        Some(2) => view.checks.push(check(
            "Search cluster",
            "warning",
            "OpenSearch reports red: some primary shards are missing, messages for them cannot \
             be written or searched.",
        )),
        Some(_) => view.checks.push(check(
            "Search cluster",
            "warning",
            "Graylog cannot read the health of its OpenSearch or Elasticsearch cluster: it is \
             down or unreachable from Graylog.",
        )),
        None => {}
    }
    if let Some(uncommitted) = s.get("journal_uncommitted_entries") {
        let used = s.get("journal_used_percent").unwrap_or(0.0);
        let state = if used >= 50.0 {
            "warning"
        } else if uncommitted >= 10_000.0 {
            "advisory"
        } else {
            "ok"
        };
        view.checks.push(check(
            "Journal",
            state,
            format!(
                "{} waiting to be processed; the journal is {used:.1}% full. A journal that \
                 keeps growing means the output does not keep up.",
                plural(uncommitted, "message", "messages")
            ),
        ));
    }
    if let Some(failed) = s.get("inputs_failed") {
        let names: Vec<String> =
            s.all("input_failed").filter_map(|e| e.labels.get("input").cloned()).collect();
        view.checks.push(if failed >= 1.0 {
            check(
                "Inputs",
                "warning",
                format!(
                    "{} failed to start{}.",
                    plural(failed, "input", "inputs"),
                    if names.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", names.join(", "))
                    }
                ),
            )
        } else {
            check("Inputs", "ok", "Every input on this node is running.")
        });
    }
    let failures = s.day("output_failures_total").unwrap_or(0.0)
        + s.day("processing_failures_total").unwrap_or(0.0);
    if failures >= 0.5 {
        view.checks.push(check(
            "Indexing failures",
            "advisory",
            format!(
                "{} in the last 24 hours: messages that did not reach the search cluster.",
                plural(failures, "failure", "failures")
            ),
        ));
    }
    if let Some(urgent) = s.get("notifications_urgent").filter(|n| *n >= 1.0) {
        view.checks.push(check(
            "Notifications",
            "advisory",
            format!(
                "Graylog raised {}.",
                plural(urgent, "urgent notification", "urgent notifications")
            ),
        ));
    }

    view.figures.push(figure("Messages in", s.rate("messages_in_total"), "per_second"));
    view.figures.push(figure("Messages out", s.rate("messages_out_total"), "per_second"));
    view.figures.push(figure("Journal backlog", s.get("journal_uncommitted_entries"), "count"));
    view.figures.push(figure("Nodes", s.get("cluster_nodes"), "count"));

    let buffers: Vec<Row> = ["input", "process", "output"]
        .iter()
        .filter_map(|buffer| {
            let used = s
                .all("buffer_used_percent")
                .find(|e| e.labels.get("buffer").map(String::as_str) == Some(buffer))?
                .value;
            Some(Row {
                label: format!("{buffer} buffer"),
                value: Some(used),
                unit: "percent",
                state: Some(if used >= 90.0 { "advisory" } else { "ok" }),
            })
        })
        .collect();
    if !buffers.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Buffers".to_string(),
            note: "How full each of this node's ring buffers is.".to_string(),
            rows: buffers,
        });
    }
    let shards: Vec<Row> = s
        .all("indexer_shards")
        .map(|e| {
            let state = e.labels.get("state").cloned().unwrap_or_default();
            let verdict = if state == "unassigned" && e.value >= 1.0 { "advisory" } else { "ok" };
            Row {
                label: format!("{state} shards"),
                value: Some(e.value),
                unit: "count",
                state: Some(verdict),
            }
        })
        .collect();
    if !shards.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Search cluster shards".to_string(),
            note: String::new(),
            rows: shards,
        });
    }
    let notes: Vec<Row> = s
        .all("notification")
        .map(|e| {
            let kind = e.labels.get("type").cloned().unwrap_or_default();
            let urgent = e.labels.get("severity").map(String::as_str) == Some("urgent");
            Row {
                label: words(&kind),
                value: None,
                unit: "count",
                state: Some(if urgent { "advisory" } else { "ok" }),
            }
        })
        .collect();
    if !notes.is_empty() {
        view.breakdowns.push(Breakdown {
            title: "Graylog notifications".to_string(),
            note: "What Graylog itself flags on its System → Overview page.".to_string(),
            rows: notes,
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
        let view = build_view("loki", &Series::new("loki", &[], &[], &[]));
        assert!(view.sampled_at.is_none());
        assert!(view.checks.is_empty());
    }

    #[test]
    fn un_victoriametrics_en_lecture_seule_est_en_alerte() {
        let k = "victoriametrics";
        let last = [
            serie(k, "version_info", 1.0, &[("version", "v1.152.0")]),
            serie(k, "healthy", 1.0, &[]),
            serie(k, "read_only", 1.0, &[]),
            serie(k, "disk_free_bytes", 9e7, &[("path", "/storage")]),
            serie(k, "disk_free_limit_bytes", 1e8, &[("path", "/storage")]),
            serie(k, "disk_headroom_percent", 0.0, &[("path", "/storage")]),
        ];
        let day = [
            serie(k, "rows_rejected_total", 12.0, &[]),
            serie(k, "rows_rejected_reason_total", 12.0, &[("reason", "big_timestamp")]),
            serie(k, "rows_rejected_reason_total", 0.0, &[("reason", "too_many_labels")]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &day));
        assert_eq!(view.version.as_deref(), Some("v1.152.0"));
        assert_eq!(view.state, "warning");
        let storage = view.checks.iter().find(|c| c.label == "Storage").unwrap();
        assert!(storage.detail.contains("Read-only"));
        let refused = view.checks.iter().find(|c| c.label == "Refused data").unwrap();
        assert_eq!(refused.state, "advisory");
        let reasons = view.breakdowns.iter().find(|b| b.title.starts_with("Refused")).unwrap();
        // Les raisons à zéro ne s'affichent pas.
        assert_eq!(reasons.rows.len(), 1);
        assert_eq!(reasons.rows[0].label, "big timestamp");
        let disk = view.breakdowns.iter().find(|b| b.title == "Disk").unwrap();
        assert_eq!(disk.rows[0].state, Some("warning"));
    }

    #[test]
    fn un_graylog_sans_opensearch_le_dit() {
        let k = "graylog";
        let last = [
            serie(k, "processing", 1.0, &[]),
            serie(k, "indexer_status", 3.0, &[]),
            serie(k, "journal_uncommitted_entries", 1190.0, &[]),
            serie(k, "journal_used_percent", 0.01, &[]),
            serie(k, "inputs_failed", 1.0, &[]),
            serie(k, "input_failed", 1.0, &[("input", "syslog-tcp")]),
            serie(k, "buffer_used_percent", 95.0, &[("buffer", "output")]),
        ];
        let view = build_view(k, &Series::new(k, &last, &[], &[]));
        assert_eq!(view.state, "warning");
        let cluster = view.checks.iter().find(|c| c.label == "Search cluster").unwrap();
        assert!(cluster.detail.contains("cannot read"));
        let inputs = view.checks.iter().find(|c| c.label == "Inputs").unwrap();
        assert!(inputs.detail.contains("syslog-tcp"));
        let buffers = view.breakdowns.iter().find(|b| b.title == "Buffers").unwrap();
        assert_eq!(buffers.rows[0].state, Some("advisory"));
    }

    #[test]
    fn un_loki_sain_est_au_vert() {
        let k = "loki";
        let last =
            [serie(k, "ready", 1.0, &[]), serie(k, "version_info", 1.0, &[("version", "3.7.8")])];
        let day = [
            serie(k, "discarded_lines_total", 0.0, &[]),
            serie(k, "flush_failures_total", 0.0, &[]),
        ];
        let rate = [serie(k, "received_lines_total", 12.5, &[])];
        let view = build_view(k, &Series::new(k, &last, &rate, &day));
        assert_eq!(view.state, "ok");
        assert_eq!(view.figures[0].value, Some(12.5));
    }
}
