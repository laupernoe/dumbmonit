//! Lecture de `mod_status` (`?auto`), et répartition du tableau de bord
//! (« scoreboard »).
//!
//! Module purement fonctionnel : la mise en forme texte se teste sur des
//! chaînes figées, sans serveur Apache en face.
//!
//! <https://httpd.apache.org/docs/2.4/mod/mod_status.html>

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};

/// Ce que `mod_status?auto` peut publier. Seuls `busy_workers` et
/// `idle_workers` sont garantis : le reste dépend de `ExtendedStatus` et de la
/// version d'Apache.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModStatus {
    pub busy_workers: f64,
    pub idle_workers: f64,
    /// Présents seulement si `ExtendedStatus On`.
    pub total_accesses: Option<f64>,
    pub total_kbytes: Option<f64>,
    pub cpu_load: Option<f64>,
    pub uptime_seconds: Option<f64>,
    pub req_per_sec: Option<f64>,
    pub bytes_per_sec: Option<f64>,
    pub bytes_per_req: Option<f64>,
    /// Présents seulement avec un MPM asynchrone (`event`, `worker`).
    pub conns_total: Option<f64>,
    pub conns_async_writing: Option<f64>,
    pub conns_async_keep_alive: Option<f64>,
    pub conns_async_closing: Option<f64>,
    /// Un caractère par emplacement de worker.
    pub scoreboard: Option<String>,
    /// `Apache/2.4.58 (Unix)`, si la page le publie (pas garanti : la plupart
    /// des distributions le retirent par prudence).
    pub server_version: Option<String>,
}

/// Lit la page `?auto` : des lignes `Clé: valeur`, dans un ordre qui varie
/// d'une version à l'autre — contrairement à `stub_status` de Nginx, rien ici
/// n'a de position fixe, la lecture passe donc par une table.
pub fn parse_mod_status(text: &str) -> Result<ModStatus, String> {
    let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        fields.insert(key.trim(), value.trim());
    }

    let busy_workers = required_number(&fields, "BusyWorkers")?;
    let idle_workers = required_number(&fields, "IdleWorkers")?;

    Ok(ModStatus {
        busy_workers,
        idle_workers,
        total_accesses: optional_number(&fields, "Total Accesses"),
        total_kbytes: optional_number(&fields, "Total kBytes"),
        cpu_load: optional_number(&fields, "CPULoad"),
        uptime_seconds: optional_number(&fields, "Uptime"),
        req_per_sec: optional_number(&fields, "ReqPerSec"),
        bytes_per_sec: optional_number(&fields, "BytesPerSec"),
        bytes_per_req: optional_number(&fields, "BytesPerReq"),
        conns_total: optional_number(&fields, "ConnsTotal"),
        conns_async_writing: optional_number(&fields, "ConnsAsyncWriting"),
        conns_async_keep_alive: optional_number(&fields, "ConnsAsyncKeepAlive"),
        conns_async_closing: optional_number(&fields, "ConnsAsyncClosing"),
        scoreboard: fields.get("Scoreboard").map(|s| s.to_string()),
        server_version: fields.get("ServerVersion").map(|s| s.to_string()),
    })
}

fn required_number(fields: &BTreeMap<&str, &str>, key: &str) -> Result<f64, String> {
    let raw = fields.get(key).ok_or_else(|| {
        format!(
            "missing \"{key}\": this does not look like mod_status's \"?auto\" output \
             (check that the \"auto\" query string was requested)"
        )
    })?;
    raw.parse().map_err(|_| format!("invalid \"{key}\" value: \"{raw}\""))
}

fn optional_number(fields: &BTreeMap<&str, &str>, key: &str) -> Option<f64> {
    fields.get(key).and_then(|raw| raw.parse().ok())
}

/// Les onze états d'un emplacement de worker, dans l'ordre documenté par
/// `mod_status` — « waiting » et « open_slot » sont de loin les plus fréquents,
/// ils ouvrent la liste pour que `apache_scoreboard` s'y retrouve vite.
pub const SCOREBOARD_STATES: &[(char, &str)] = &[
    ('_', "waiting"),
    ('.', "open_slot"),
    ('S', "starting"),
    ('R', "reading"),
    ('W', "sending"),
    ('K', "keepalive"),
    ('D', "dns_lookup"),
    ('C', "closing"),
    ('L', "logging"),
    ('G', "finishing"),
    ('I', "idle_cleanup"),
];

/// Compte chaque état du tableau de bord ; un caractère inconnu (une future
/// version d'Apache en ajouterait un) tombe dans `"other"` plutôt que de
/// disparaître.
pub fn scoreboard_counts(scoreboard: &str) -> BTreeMap<&'static str, u64> {
    let mut counts: BTreeMap<&'static str, u64> =
        SCOREBOARD_STATES.iter().map(|(_, label)| (*label, 0)).collect();
    for ch in scoreboard.chars() {
        let label = SCOREBOARD_STATES
            .iter()
            .find(|(candidate, _)| *candidate == ch)
            .map(|(_, label)| *label)
            .unwrap_or("other");
        *counts.entry(label).or_insert(0) += 1;
    }
    counts
}

fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Gauge, ts_ms)
}

fn counter(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Counter, ts_ms)
}

pub fn samples(status: &ModStatus, ts_ms: i64) -> Vec<Sample> {
    let mut samples = vec![
        gauge("apache_workers_busy", status.busy_workers, ts_ms),
        gauge("apache_workers_idle", status.idle_workers, ts_ms),
    ];
    if let Some(total) = status.total_accesses {
        samples.push(counter("apache_requests_total", total, ts_ms));
    }
    if let Some(kbytes) = status.total_kbytes {
        samples.push(counter("apache_sent_bytes_total", kbytes * 1_024.0, ts_ms));
    }
    if let Some(uptime) = status.uptime_seconds {
        samples.push(gauge("apache_uptime_seconds", uptime, ts_ms));
    }
    if let Some(value) = status.req_per_sec {
        samples.push(gauge("apache_requests_per_second", value, ts_ms));
    }
    if let Some(value) = status.bytes_per_sec {
        samples.push(gauge("apache_bytes_per_second", value, ts_ms));
    }
    if let Some(value) = status.bytes_per_req {
        samples.push(gauge("apache_bytes_per_request", value, ts_ms));
    }
    if let Some(value) = status.cpu_load {
        samples.push(gauge("apache_cpu_load_ratio", value, ts_ms));
    }
    if let Some(value) = status.conns_total {
        samples.push(gauge("apache_connections_total", value, ts_ms));
    }
    if let Some(value) = status.conns_async_writing {
        samples.push(gauge("apache_connections_async_writing", value, ts_ms));
    }
    if let Some(value) = status.conns_async_keep_alive {
        samples.push(gauge("apache_connections_async_keepalive", value, ts_ms));
    }
    if let Some(value) = status.conns_async_closing {
        samples.push(gauge("apache_connections_async_closing", value, ts_ms));
    }
    if let Some(scoreboard) = &status.scoreboard {
        for (state, count) in scoreboard_counts(scoreboard) {
            samples
                .push(gauge("apache_scoreboard", count as f64, ts_ms).with_label("state", state));
        }
    }
    if let Some(version) = &status.server_version {
        samples.push(gauge("apache_version_info", 1.0, ts_ms).with_label("version", version));
    }
    samples
}

/// Présence de l'en-tête `Server` HTTP, étiquette unique : la source la plus
/// sûre de la version, qu'`ExtendedStatus` soit activé ou non.
pub fn server_header_sample(server: &str, ts_ms: i64) -> Sample {
    gauge("apache_version_info", 1.0, ts_ms).with_label("version", server)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'exemple de la documentation officielle de `mod_status`.
    const EXEMPLE: &str = "Total Accesses: 46499\n\
                            Total kBytes: 332255\n\
                            CPULoad: .1\n\
                            Uptime: 1575804\n\
                            ReqPerSec: .0295\n\
                            BytesPerSec: 215.906\n\
                            BytesPerReq: 7317.43\n\
                            BusyWorkers: 2\n\
                            IdleWorkers: 8\n\
                            ConnsTotal: 3\n\
                            ConnsAsyncWriting: 0\n\
                            ConnsAsyncKeepAlive: 1\n\
                            ConnsAsyncClosing: 0\n\
                            Scoreboard: ____W_________________..........\n";

    #[test]
    fn lexemple_de_la_documentation_se_lit_entierement() {
        let status = parse_mod_status(EXEMPLE).unwrap();
        assert_eq!(status.busy_workers, 2.0);
        assert_eq!(status.idle_workers, 8.0);
        assert_eq!(status.total_accesses, Some(46_499.0));
        assert_eq!(status.total_kbytes, Some(332_255.0));
        assert_eq!(status.cpu_load, Some(0.1));
        assert_eq!(status.uptime_seconds, Some(1_575_804.0));
        assert_eq!(status.conns_total, Some(3.0));
        assert_eq!(status.scoreboard.as_deref(), Some("____W_________________.........."));
    }

    #[test]
    fn un_statut_minimal_sans_extended_status_reste_lisible() {
        let status = parse_mod_status("BusyWorkers: 1\nIdleWorkers: 4\n").unwrap();
        assert_eq!(status.busy_workers, 1.0);
        assert_eq!(status.idle_workers, 4.0);
        assert_eq!(status.total_accesses, None);
        let samples = samples(&status, 0);
        assert!(!samples.iter().any(|s| s.metric == "apache_requests_total"));
    }

    #[test]
    fn labsence_de_busyworkers_est_signalee() {
        let error = parse_mod_status("IdleWorkers: 4\n").unwrap_err();
        assert!(error.contains("BusyWorkers"), "{error}");
    }

    #[test]
    fn le_tableau_de_bord_se_decompose_par_etat() {
        let counts = scoreboard_counts("____W_________________..........");
        assert_eq!(counts["waiting"], 21);
        assert_eq!(counts["sending"], 1);
        assert_eq!(counts["open_slot"], 10);
        assert_eq!(counts["starting"], 0);
    }

    #[test]
    fn un_caractere_inconnu_tombe_dans_other() {
        let counts = scoreboard_counts("_?.");
        assert_eq!(counts.get("other"), Some(&1), "le « ? » inconnu est compté à part");
        assert_eq!(counts["waiting"], 1);
        assert_eq!(counts["open_slot"], 1);
        let total: u64 = counts.values().sum();
        assert_eq!(total, 3, "les trois caractères sont tous comptés");
    }

    #[test]
    fn les_accumulateurs_sont_des_compteurs_le_reste_des_jauges() {
        let status = parse_mod_status(EXEMPLE).unwrap();
        let samples = samples(&status, 0);
        for metric in ["apache_requests_total", "apache_sent_bytes_total"] {
            assert_eq!(
                samples.iter().find(|s| s.metric == metric).unwrap().kind,
                MetricKind::Counter,
                "{metric}"
            );
        }
        assert_eq!(
            samples.iter().find(|s| s.metric == "apache_workers_busy").unwrap().kind,
            MetricKind::Gauge
        );
    }
}
