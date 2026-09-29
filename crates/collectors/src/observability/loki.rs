//! Grafana Loki.
//!
//! `/ready` dit si Loki accepte d'écrire et de lire ; `/metrics` dit ce qu'il
//! fait de ce qu'on lui envoie. Les pannes que ce module existe pour voir :
//!
//! * **des lignes refusées** (`loki_discarded_samples_total`) : trop anciennes,
//!   limite de débit du locataire, trop de flux, ligne trop longue. Le client
//!   reçoit un 4xx, Promtail ou Alloy abandonnent le lot, et le journal manque
//!   à l'appel le jour où on le cherche ;
//! * **des blocs qui ne partent pas vers le stockage** (`flush failures`) : ils
//!   restent en mémoire, et disparaissent au prochain redémarrage ;
//! * **un WAL sur un disque plein**, **des requêtes en erreur 5xx**, un anneau
//!   avec des membres `Unhealthy`.
//!
//! Loki découpé en microservices expose ces familles réparties entre ses
//! composants : on surveille alors l'adresse du composant qui porte la famille
//! voulue (distributeur, ingester) ; ce qui manque ne produit pas de série.

use dumbmonit_proto::{MetricKind, ProbeError, Sample};

use super::client::HttpClient;
use super::prom::Exposition;
use super::{counter, gauge, ready_sample};

pub const DEFAULT_PORT: u16 = 3100;

pub async fn probe(client: &HttpClient, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    let ready = client.get_raw("/ready").await?;
    if matches!(ready.status.as_u16(), 401 | 403) {
        return Err(client.status_error(ready.status, &ready.body, "/ready"));
    }
    // `ready` en 200 ; en 503, une phrase : « Ingester not ready: waiting for
    // 15s after being ready ». Un 404 veut dire que l'adresse n'est pas Loki.
    if ready.status.as_u16() == 404 {
        return Err(client.status_error(ready.status, &ready.body, "/ready"));
    }
    let is_ready = ready.status.is_success() && ready.body.trim().eq_ignore_ascii_case("ready");

    let page = Exposition::parse(&client.get_text("/metrics").await?);
    if !page.has("loki_build_info") {
        return Err(ProbeError::Protocol(format!(
            "{} does not look like Loki: its /metrics has no loki_build_info. Check the device \
             type and the port.",
            client.url("/metrics")
        )));
    }
    let mut samples = samples(&page, ts_ms);
    samples.push(ready_sample("loki_ready", is_ready, ts_ms));
    Ok(samples)
}

pub fn samples(page: &Exposition, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(version) = page.first("loki_build_info").and_then(|l| l.label("version")) {
        out.push(gauge("loki_version_info", 1.0, ts_ms).with_label("version", version));
    }
    let mut push = |name: &str, value: Option<f64>, kind: MetricKind| {
        if let Some(value) = value.filter(|v| v.is_finite()) {
            out.push(Sample::new(name, value, kind, ts_ms));
        }
    };
    use MetricKind::{Counter, Gauge};

    push("loki_received_bytes_total", page.sum("loki_distributor_bytes_received_total"), Counter);
    push("loki_received_lines_total", page.sum("loki_distributor_lines_received_total"), Counter);
    push("loki_memory_streams", page.sum("loki_ingester_memory_streams"), Gauge);
    push(
        "loki_flush_failures_total",
        page.sum("loki_ingester_chunks_flush_failures_total"),
        Counter,
    );
    push("loki_flush_queue_length", page.sum("loki_ingester_flush_queue_length"), Gauge);
    push(
        "loki_wal_disk_full_failures_total",
        page.sum("loki_ingester_wal_disk_full_failures_total"),
        Counter,
    );
    // Malgré son nom, la jauge est une fraction (0,87 sur un disque occupé à
    // 87 %) : elle est rendue en pourcentage.
    push(
        "loki_wal_disk_used_percent",
        page.sum("loki_ingester_wal_disk_usage_percent").map(|ratio| ratio * 100.0),
        Gauge,
    );
    push("loki_panics_total", page.sum("loki_panic_total"), Counter);
    push(
        "loki_log_errors_total",
        page.sum_where("loki_log_messages_total", |l| l.label("level") == Some("error")),
        Counter,
    );
    push(
        "loki_ring_unhealthy_members",
        page.sum_where("loki_ring_members", |l| l.label("state") == Some("Unhealthy")),
        Gauge,
    );
    push(
        "loki_retention_last_run_timestamp_seconds",
        page.sum("loki_compactor_apply_retention_last_successful_run_timestamp_seconds")
            .filter(|ts| *ts > 0.0),
        Gauge,
    );
    push("loki_memory_resident_bytes", page.sum("process_resident_memory_bytes"), Gauge);

    // Requêtes : HTTP et gRPC confondus. Une erreur est un 5xx côté HTTP, un
    // `error` côté gRPC ; les 4xx sont la faute du client, et les lignes
    // refusées sont déjà comptées plus bas.
    let requests = page.sum("loki_request_duration_seconds_count");
    let errors = page.sum_where("loki_request_duration_seconds_count", |l| {
        l.label("status_code").is_some_and(|code| code.starts_with('5') || code == "error")
    });
    push("loki_requests_total", requests, Counter);
    push("loki_request_errors_total", errors, Counter);

    // Lignes refusées : le total est toujours émis, y compris à zéro, parce que
    // Loki ne crée la série d'une raison qu'au premier refus.
    let reasons = page.sum_by("loki_discarded_samples_total", "reason");
    out.push(counter("loki_discarded_lines_total", reasons.values().sum(), ts_ms));
    for (reason, value) in reasons.into_iter().filter(|(reason, _)| !reason.is_empty()) {
        out.push(
            counter("loki_discarded_lines_reason_total", value, ts_ms).with_label("reason", reason),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `/metrics` d'un Loki 3.7.8 en binaire unique (configuration fournie
    /// avec l'image), qui a accepté quelques lignes et en a refusé deux, trop
    /// anciennes d'un mois. Les lignes `_bucket` des histogrammes ont été
    /// retirées pour alléger le fichier ; rien d'autre n'a été touché.
    const LOKI: &str = include_str!("testdata/loki_3.7.8.prom");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn loki_reel() {
        let samples = samples(&Exposition::parse(LOKI), 0);
        let version = samples.iter().find(|s| s.metric == "loki_version_info").unwrap();
        assert_eq!(version.labels["version"], "3.7.8");
        assert_eq!(value(&samples, "loki_discarded_lines_total"), Some(2.0));
        let reason =
            samples.iter().find(|s| s.metric == "loki_discarded_lines_reason_total").unwrap();
        assert_eq!(reason.labels["reason"], "greater_than_max_sample_age");
        assert!(value(&samples, "loki_received_lines_total").unwrap() >= 2.0);
        assert!(value(&samples, "loki_received_bytes_total").unwrap() > 0.0);
        assert_eq!(value(&samples, "loki_flush_failures_total"), Some(0.0));
        assert_eq!(value(&samples, "loki_ring_unhealthy_members"), Some(0.0));
        assert_eq!(value(&samples, "loki_panics_total"), Some(0.0));
        assert_eq!(value(&samples, "loki_memory_streams"), Some(1.0));
        let wal = value(&samples, "loki_wal_disk_used_percent").unwrap();
        assert!((1.0..=100.0).contains(&wal), "pourcentage {wal}");
        // Les 400 des poussées refusées ne sont pas des erreurs du serveur.
        assert_eq!(value(&samples, "loki_request_errors_total"), Some(0.0));
        assert!(value(&samples, "loki_requests_total").unwrap() > 0.0);
        // La rétention n'a jamais tourné : pas de date à zéro.
        assert!(value(&samples, "loki_retention_last_run_timestamp_seconds").is_none());
    }

    #[test]
    fn les_erreurs_serveur_et_grpc_sont_comptees() {
        let page = Exposition::parse(
            "loki_build_info{version=\"3.7.8\"} 1\n\
             loki_request_duration_seconds_count{route=\"push\",status_code=\"204\"} 10\n\
             loki_request_duration_seconds_count{route=\"push\",status_code=\"500\"} 2\n\
             loki_request_duration_seconds_count{route=\"q\",status_code=\"503\"} 1\n\
             loki_request_duration_seconds_count{route=\"/logproto.Pusher/Push\",status_code=\"error\"} 4\n\
             loki_request_duration_seconds_count{route=\"q\",status_code=\"429\"} 7\n",
        );
        let samples = samples(&page, 0);
        assert_eq!(value(&samples, "loki_request_errors_total"), Some(7.0));
        assert_eq!(value(&samples, "loki_requests_total"), Some(24.0));
        // Aucun refus : le total existe quand même, à zéro.
        assert_eq!(value(&samples, "loki_discarded_lines_total"), Some(0.0));
    }
}
