//! VictoriaMetrics et VictoriaLogs (version à nœud unique).
//!
//! Les deux produits partagent leur socle : `/health` qui répond `OK`, une page
//! `/metrics` où l'on retrouve `vm_app_version`, les drapeaux de ligne de
//! commande (`flag{name=…}`) et le même mécanisme de protection du disque —
//! sous `-storage.minFreeDiskSpaceBytes` d'espace libre, le stockage passe en
//! lecture seule et **refuse toute nouvelle donnée**. C'est la panne que ce
//! module existe pour voir venir : l'ingestion s'arrête net, et les clients qui
//! poussent (vmagent, Promtail, Vector…) accumulent puis perdent.
//!
//! Ce qui est lu, pour les deux : l'ingestion (lignes acceptées), les lignes
//! rejetées par raison (horodatage hors rétention, étiquettes trop longues,
//! limites de séries), la marge de disque au-dessus du seuil de lecture seule,
//! la taille des données, les erreurs HTTP et les messages d'erreur du journal.
//! Pour VictoriaMetrics en plus : les insertions lentes (le signe que la mémoire
//! ne suffit plus au nombre de séries actives) et les séries actives.

use dumbmonit_proto::{MetricKind, ProbeError, Sample};

use super::client::HttpClient;
use super::prom::Exposition;
use super::{counter, gauge, ready_sample};

/// Le produit interrogé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavour {
    Metrics,
    Logs,
}

impl Flavour {
    pub fn kind(self) -> &'static str {
        match self {
            Self::Metrics => "victoriametrics",
            Self::Logs => "victorialogs",
        }
    }

    pub fn product(self) -> &'static str {
        match self {
            Self::Metrics => "VictoriaMetrics",
            Self::Logs => "VictoriaLogs",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::Metrics => 8428,
            Self::Logs => 9428,
        }
    }

    /// Préfixe des métriques propres au produit dans `/metrics`.
    fn own(self) -> &'static str {
        match self {
            Self::Metrics => "vm",
            Self::Logs => "vl",
        }
    }

    /// La famille qui prouve qu'on parle au bon produit : un VictoriaLogs
    /// déclaré comme VictoriaMetrics (ou l'inverse) donnerait une page vide.
    fn signature(self) -> &'static str {
        match self {
            Self::Metrics => "vm_rows_inserted_total",
            Self::Logs => "vl_rows_ingested_total",
        }
    }
}

/// Les raisons de rejet que VictoriaMetrics compte sous d'autres familles que
/// `vm_rows_ignored_total`, et le nom qu'elles prennent ici.
const VM_OTHER_REJECTIONS: [(&str, &str); 3] = [
    ("vm_rows_invalid_total", "invalid"),
    ("vm_hourly_series_limit_rows_dropped_total", "hourly_series_limit"),
    ("vm_daily_series_limit_rows_dropped_total", "daily_series_limit"),
];

/// Interroge `/health` puis `/metrics`.
pub async fn probe(
    client: &HttpClient,
    flavour: Flavour,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    // `/health` répond 200 `OK` ; tout autre code avec une phrase est un serveur
    // vivant qui se dit malade, pas une panne de transport.
    let health = client.get_raw("/health").await?;
    if matches!(health.status.as_u16(), 401 | 403) {
        return Err(client.status_error(health.status, &health.body, "/health"));
    }
    let healthy = health.status.is_success() && health.body.trim().eq_ignore_ascii_case("ok");

    let page = Exposition::parse(&client.get_text("/metrics").await?);
    if !page.has(flavour.signature()) {
        return Err(ProbeError::Protocol(format!(
            "{} does not look like {}: its /metrics has no {}. Check the device type and the port.",
            client.url("/metrics"),
            flavour.product(),
            flavour.signature()
        )));
    }
    let mut samples = samples(&page, flavour, ts_ms);
    samples.push(ready_sample(&format!("{}_healthy", flavour.kind()), healthy, ts_ms));
    Ok(samples)
}

/// Traduit la page `/metrics` en échantillons. Séparé de [`probe`] pour être
/// testé sur des pages réelles.
pub fn samples(page: &Exposition, flavour: Flavour, ts_ms: i64) -> Vec<Sample> {
    let p = flavour.kind();
    let own = flavour.own();
    let mut out = Vec::new();

    if let Some(version) = page.first("vm_app_version").and_then(|line| line.label("short_version"))
    {
        out.push(gauge(&format!("{p}_version_info"), 1.0, ts_ms).with_label("version", version));
    }
    push_gauge(&mut out, &format!("{p}_uptime_seconds"), page.sum("vm_app_uptime_seconds"), ts_ms);
    push_gauge(
        &mut out,
        &format!("{p}_memory_resident_bytes"),
        page.sum("process_resident_memory_bytes"),
        ts_ms,
    );

    // Ingestion.
    match flavour {
        Flavour::Metrics => {
            push_counter(
                &mut out,
                &format!("{p}_rows_ingested_total"),
                page.sum("vm_rows_inserted_total"),
                ts_ms,
            );
            push_counter(
                &mut out,
                &format!("{p}_rows_added_total"),
                page.sum("vm_rows_added_to_storage_total"),
                ts_ms,
            );
            push_counter(
                &mut out,
                &format!("{p}_slow_inserts_total"),
                page.sum("vm_slow_row_inserts_total"),
                ts_ms,
            );
            push_gauge(
                &mut out,
                &format!("{p}_active_series"),
                page.sum_where("vm_cache_entries", |l| {
                    l.label("type") == Some("storage/hour_metric_ids")
                }),
                ts_ms,
            );
        }
        Flavour::Logs => {
            push_counter(
                &mut out,
                &format!("{p}_rows_ingested_total"),
                page.sum("vl_rows_ingested_total"),
                ts_ms,
            );
            push_counter(
                &mut out,
                &format!("{p}_bytes_ingested_total"),
                page.sum("vl_bytes_ingested_total"),
                ts_ms,
            );
        }
    }

    // Rejets : le total est toujours émis, même à zéro, pour que la règle ait
    // deux points à comparer dès le premier rejet.
    let reasons = rejections(page, flavour);
    let total: f64 = reasons.iter().map(|(_, value)| value).sum();
    out.push(counter(&format!("{p}_rows_rejected_total"), total, ts_ms));
    for (reason, value) in reasons {
        out.push(
            counter(&format!("{p}_rows_rejected_reason_total"), value, ts_ms)
                .with_label("reason", reason),
        );
    }

    // Disque : l'espace libre, le seuil sous lequel le stockage passe en
    // lecture seule, et la marge entre les deux.
    let limit = page
        .sum(&format!("{own}_free_disk_space_limit_bytes"))
        .or_else(|| page.flag("storage.minFreeDiskSpaceBytes").and_then(parse_bytes));
    for line in page.family(&format!("{own}_free_disk_space_bytes")) {
        let path = line.label("path").unwrap_or_default();
        let free = line.value;
        out.push(gauge(&format!("{p}_disk_free_bytes"), free, ts_ms).with_label("path", path));
        let total = page
            .family(&format!("{own}_total_disk_space_bytes"))
            .find(|l| l.label("path") == Some(path))
            .map(|l| l.value);
        if let Some(total) = total {
            out.push(
                gauge(&format!("{p}_disk_total_bytes"), total, ts_ms).with_label("path", path),
            );
        }
        if let Some(limit) = limit {
            out.push(
                gauge(&format!("{p}_disk_free_limit_bytes"), limit, ts_ms).with_label("path", path),
            );
            if let Some(total) = total.filter(|total| *total > 0.0) {
                let headroom = ((free - limit) / total * 100.0).max(0.0);
                out.push(
                    gauge(&format!("{p}_disk_headroom_percent"), headroom, ts_ms)
                        .with_label("path", path),
                );
            }
        }
    }
    push_gauge(
        &mut out,
        &format!("{p}_read_only"),
        page.family(&format!("{own}_storage_is_read_only")).map(|l| l.value).reduce(f64::max),
        ts_ms,
    );
    push_gauge(
        &mut out,
        &format!("{p}_data_size_bytes"),
        page.sum(&format!("{own}_data_size_bytes")),
        ts_ms,
    );

    // Erreurs.
    let http_errors = match flavour {
        Flavour::Metrics => page.sum("vm_http_request_errors_total"),
        Flavour::Logs => page.sum("vl_http_errors_total"),
    };
    push_counter(&mut out, &format!("{p}_http_errors_total"), http_errors, ts_ms);
    push_counter(
        &mut out,
        &format!("{p}_log_errors_total"),
        page.sum_where("vm_log_messages_total", |l| {
            matches!(l.label("level"), Some("error" | "fatal" | "panic"))
        })
        // Sans message d'erreur depuis le démarrage, la famille ne compte que
        // des niveaux `info` ou `warn` ; zéro reste une mesure.
        .or(Some(0.0)),
        ts_ms,
    );
    out
}

/// Les lignes rejetées, par raison. Toutes les raisons connues du serveur sont
/// rendues, y compris à zéro : c'est ce qui permet de voir la première.
fn rejections(page: &Exposition, flavour: Flavour) -> Vec<(String, f64)> {
    let mut reasons: Vec<(String, f64)> = Vec::new();
    match flavour {
        Flavour::Metrics => {
            reasons.extend(page.sum_by("vm_rows_ignored_total", "reason"));
            for (family, reason) in VM_OTHER_REJECTIONS {
                if let Some(value) = page.sum(family) {
                    reasons.push((reason.to_string(), value));
                }
            }
        }
        Flavour::Logs => {
            // `debug` : des lignes que le client a lui-même demandé de ne pas
            // enregistrer (paramètre `debug=1`), pas un rejet.
            reasons.extend(
                page.sum_by("vl_rows_dropped_total", "reason")
                    .into_iter()
                    .filter(|(reason, _)| reason != "debug"),
            );
            if let Some(value) = page.sum("vl_too_long_lines_skipped_total") {
                reasons.push(("too_long_line".to_string(), value));
            }
        }
    }
    reasons.retain(|(reason, _)| !reason.is_empty());
    reasons
}

/// `-storage.minFreeDiskSpaceBytes` accepte les suffixes `KB`, `MiB`, `GB`…
fn parse_bytes(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    let split = raw.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(raw.len());
    let (number, unit) = raw.split_at(split);
    let number: f64 = number.parse().ok()?;
    let factor = match unit.trim() {
        "" | "B" => 1.0,
        "KB" | "kB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some(number * factor)
}

fn push_gauge(out: &mut Vec<Sample>, name: &str, value: Option<f64>, ts_ms: i64) {
    if let Some(value) = value.filter(|v| v.is_finite()) {
        out.push(Sample::new(name, value, MetricKind::Gauge, ts_ms));
    }
}

fn push_counter(out: &mut Vec<Sample>, name: &str, value: Option<f64>, ts_ms: i64) {
    if let Some(value) = value.filter(|v| v.is_finite()) {
        out.push(Sample::new(name, value, MetricKind::Counter, ts_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `/metrics` d'un VictoriaMetrics v1.152.0 à nœud unique, après deux
    /// imports des mêmes cinquante séries, chacun suivi de deux lignes hors
    /// rétention (l'une trop ancienne, l'autre trop loin dans le futur).
    const VM: &str = include_str!("testdata/victoriametrics_1.152.0.prom");
    /// `/metrics` d'un VictoriaLogs v1.52.0 qui a reçu une centaine de lignes
    /// JSON.
    const VL: &str = include_str!("testdata/victorialogs_1.52.0.prom");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn labelled(samples: &[Sample], name: &str, key: &str, wanted: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == name && s.labels.get(key).map(String::as_str) == Some(wanted))
            .map(|s| s.value)
    }

    #[test]
    fn victoriametrics_reel() {
        let samples = samples(&Exposition::parse(VM), Flavour::Metrics, 0);
        let version = samples.iter().find(|s| s.metric == "victoriametrics_version_info").unwrap();
        assert_eq!(version.labels["version"], "v1.152.0");
        assert_eq!(value(&samples, "victoriametrics_rows_ingested_total"), Some(104.0));
        assert_eq!(value(&samples, "victoriametrics_rows_added_total"), Some(100.0));
        // Chaque série neuve est une insertion lente : les cinquante du premier
        // import, aucune du second, qui réutilise les mêmes séries.
        assert_eq!(value(&samples, "victoriametrics_slow_inserts_total"), Some(50.0));
        assert_eq!(value(&samples, "victoriametrics_active_series"), Some(50.0));
        assert_eq!(value(&samples, "victoriametrics_rows_rejected_total"), Some(4.0));
        assert_eq!(
            labelled(
                &samples,
                "victoriametrics_rows_rejected_reason_total",
                "reason",
                "big_timestamp"
            ),
            Some(2.0)
        );
        assert_eq!(
            labelled(&samples, "victoriametrics_rows_rejected_reason_total", "reason", "invalid"),
            Some(0.0)
        );
        assert_eq!(value(&samples, "victoriametrics_read_only"), Some(0.0));
        assert_eq!(
            labelled(&samples, "victoriametrics_disk_free_limit_bytes", "path", "/storage"),
            Some(100_000_000.0)
        );
        let headroom =
            labelled(&samples, "victoriametrics_disk_headroom_percent", "path", "/storage")
                .unwrap();
        assert!((12.0..15.0).contains(&headroom), "marge {headroom}");
        assert_eq!(value(&samples, "victoriametrics_log_errors_total"), Some(0.0));
        assert_eq!(value(&samples, "victoriametrics_http_errors_total"), Some(0.0));
        assert!(value(&samples, "victoriametrics_uptime_seconds").is_some());
        let counters =
            ["victoriametrics_rows_ingested_total", "victoriametrics_rows_rejected_total"];
        for name in counters {
            let sample = samples.iter().find(|s| s.metric == name).unwrap();
            assert_eq!(sample.kind, MetricKind::Counter, "{name}");
        }
    }

    #[test]
    fn victorialogs_reel() {
        let samples = samples(&Exposition::parse(VL), Flavour::Logs, 0);
        let version = samples.iter().find(|s| s.metric == "victorialogs_version_info").unwrap();
        assert_eq!(version.labels["version"], "v1.52.0");
        assert_eq!(value(&samples, "victorialogs_rows_ingested_total"), Some(101.0));
        assert!(value(&samples, "victorialogs_bytes_ingested_total").unwrap() > 0.0);
        // La ligne `debug=1` n'est pas un rejet.
        assert_eq!(value(&samples, "victorialogs_rows_rejected_total"), Some(0.0));
        assert!(
            labelled(&samples, "victorialogs_rows_rejected_reason_total", "reason", "debug")
                .is_none()
        );
        assert!(
            labelled(
                &samples,
                "victorialogs_rows_rejected_reason_total",
                "reason",
                "too_small_timestamp"
            )
            .is_some()
        );
        // Pas de famille `vl_free_disk_space_limit_bytes` : le seuil vient du drapeau.
        assert_eq!(
            labelled(&samples, "victorialogs_disk_free_limit_bytes", "path", "/vlogs"),
            Some(10_000_000.0)
        );
        assert!(value(&samples, "victorialogs_disk_headroom_percent").is_some());
        assert_eq!(value(&samples, "victorialogs_read_only"), Some(0.0));
        assert!(value(&samples, "victorialogs_slow_inserts_total").is_none());
    }

    #[test]
    fn un_stockage_en_lecture_seule_se_voit() {
        let page = Exposition::parse(
            "vm_rows_inserted_total{type=\"promremotewrite\"} 10\n\
             vm_free_disk_space_bytes{path=\"/storage\"} 90000000\n\
             vm_total_disk_space_bytes{path=\"/storage\"} 1000000000\n\
             vm_free_disk_space_limit_bytes{path=\"/storage\"} 100000000\n\
             vm_storage_is_read_only{path=\"/storage\"} 1\n",
        );
        let samples = samples(&page, Flavour::Metrics, 0);
        assert_eq!(value(&samples, "victoriametrics_read_only"), Some(1.0));
        // Sous le seuil, la marge est nulle et non négative.
        assert_eq!(value(&samples, "victoriametrics_disk_headroom_percent"), Some(0.0));
    }

    #[test]
    fn le_seuil_de_disque_accepte_les_suffixes() {
        assert_eq!(parse_bytes("10000000"), Some(1e7));
        assert_eq!(parse_bytes("1GB"), Some(1e9));
        assert_eq!(parse_bytes("512MiB"), Some(512.0 * 1024.0 * 1024.0));
        assert_eq!(parse_bytes("beaucoup"), None);
    }
}
