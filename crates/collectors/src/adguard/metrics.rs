//! Réponses de l'API `/control` et échantillons qui en sont tirés.
//!
//! Tout ici est pur : les structures arrivent déjà désérialisées, les tests
//! partent des réponses réelles capturées dans `testdata/`.
//!
//! Les chiffres de requêtes d'AdGuard Home ne sont **pas** des compteurs : ce
//! sont des sommes sur sa fenêtre de statistiques glissante (24 h par défaut,
//! réglable de 1 h à 90 jours). Ils sont écrits en jauges, accompagnés de la
//! longueur de la fenêtre, et ne doivent jamais passer par `increase`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dumbmonit_proto::{MetricKind, Sample};
use serde::Deserialize;

/// Au plus autant de listes de filtrage portent une série nommée.
pub const MAX_FILTERS: usize = 32;

/// Au plus autant de serveurs amont sont testés ou nommés.
pub const MAX_UPSTREAMS: usize = 8;

/// Longueur maximale d'une étiquette tirée d'un nom saisi par l'utilisateur.
const MAX_LABEL: usize = 100;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

pub fn label(text: &str) -> String {
    text.trim().chars().take(MAX_LABEL).collect()
}

// ------------------------------------------------------------------ modèle

/// `GET /control/status`.
#[derive(Debug, Deserialize)]
pub struct Status {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub protection_enabled: bool,
    /// Millisecondes restantes d'une pause de la protection, 0 sans pause.
    #[serde(default)]
    pub protection_disabled_duration: Option<f64>,
}

/// `GET /control/stats`.
#[derive(Debug, Deserialize)]
pub struct Stats {
    #[serde(default)]
    pub num_dns_queries: f64,
    #[serde(default)]
    pub num_blocked_filtering: f64,
    #[serde(default)]
    pub num_replaced_safebrowsing: f64,
    #[serde(default)]
    pub num_replaced_safesearch: f64,
    #[serde(default)]
    pub num_replaced_parental: f64,
    /// Secondes.
    #[serde(default)]
    pub avg_processing_time: f64,
    /// Réponses par serveur amont, sur la fenêtre (0.107.24 et suivantes).
    #[serde(default)]
    pub top_upstreams_responses: Vec<BTreeMap<String, f64>>,
    /// Temps de réponse moyen par serveur amont, en secondes.
    #[serde(default)]
    pub top_upstreams_avg_time: Vec<BTreeMap<String, f64>>,
}

/// `GET /control/stats/config`.
#[derive(Debug, Deserialize)]
pub struct StatsConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Millisecondes.
    #[serde(default)]
    pub interval: Option<f64>,
}

/// `GET /control/filtering/status`.
#[derive(Debug, Deserialize)]
pub struct FilteringStatus {
    #[serde(default)]
    pub enabled: bool,
    /// Heures entre deux mises à jour des listes ; 0 les désactive.
    #[serde(default)]
    pub interval: Option<f64>,
    #[serde(default)]
    pub filters: Option<Vec<Filter>>,
}

#[derive(Debug, Deserialize)]
pub struct Filter {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules_count: f64,
    /// Absent tant que la liste n'a jamais été téléchargée.
    #[serde(default)]
    pub last_updated: Option<String>,
}

/// `POST /control/version.json` avec `recheck_now: false`.
#[derive(Debug, Deserialize)]
pub struct VersionCheck {
    /// Vrai quand le contrôle est désactivé (`--no-check-update`, l'image
    /// Docker officielle, les paquets des distributions).
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub new_version: Option<String>,
}

/// `GET /control/dns_info` : seuls les serveurs amont nous intéressent.
#[derive(Debug, Deserialize)]
pub struct DnsInfo {
    #[serde(default)]
    pub upstream_dns: Option<Vec<String>>,
    #[serde(default)]
    pub bootstrap_dns: Option<Vec<String>>,
}

impl DnsInfo {
    /// Les lignes d'amont à tester : sans commentaire ni ligne vide, bornées.
    pub fn upstreams(&self) -> Vec<String> {
        self.upstream_dns
            .iter()
            .flatten()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .take(MAX_UPSTREAMS)
            .map(str::to_string)
            .collect()
    }
}

// ------------------------------------------------------------------ échantillons

pub fn status_samples(status: &Status, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        gauge("adguard_running", flag(status.running), ts_ms),
        gauge("adguard_protection_enabled", flag(status.protection_enabled), ts_ms),
        gauge(
            "adguard_protection_paused_seconds",
            status
                .protection_disabled_duration
                .filter(|ms| *ms > 0.0)
                .map_or(0.0, |ms| ms / 1000.0),
            ts_ms,
        ),
    ];
    if !status.version.is_empty() {
        out.push(gauge("adguard_version_info", 1.0, ts_ms).with_label("version", &status.version));
    }
    out
}

pub fn stats_samples(stats: &Stats, config: Option<&StatsConfig>, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        gauge("adguard_dns_queries", stats.num_dns_queries, ts_ms),
        gauge("adguard_blocked_filtering", stats.num_blocked_filtering, ts_ms),
        gauge("adguard_replaced_safebrowsing", stats.num_replaced_safebrowsing, ts_ms),
        gauge("adguard_replaced_parental", stats.num_replaced_parental, ts_ms),
        gauge("adguard_replaced_safesearch", stats.num_replaced_safesearch, ts_ms),
        gauge("adguard_avg_processing_seconds", stats.avg_processing_time, ts_ms),
    ];
    if stats.num_dns_queries > 0.0 {
        let blocked = stats.num_blocked_filtering
            + stats.num_replaced_safebrowsing
            + stats.num_replaced_parental;
        out.push(gauge("adguard_blocked_percent", 100.0 * blocked / stats.num_dns_queries, ts_ms));
    }
    if let Some(config) = config {
        if let Some(enabled) = config.enabled {
            out.push(gauge("adguard_stats_enabled", flag(enabled), ts_ms));
        }
        if let Some(interval) = config.interval.filter(|ms| *ms > 0.0) {
            out.push(gauge("adguard_stats_window_seconds", interval / 1000.0, ts_ms));
        }
    }
    let upstreams = |list: &[BTreeMap<String, f64>]| -> Vec<(String, f64)> {
        list.iter().flatten().take(MAX_UPSTREAMS).map(|(k, v)| (label(k), *v)).collect()
    };
    for (upstream, value) in upstreams(&stats.top_upstreams_responses) {
        out.push(
            gauge("adguard_upstream_responses", value, ts_ms).with_label("upstream", upstream),
        );
    }
    for (upstream, value) in upstreams(&stats.top_upstreams_avg_time) {
        out.push(
            gauge("adguard_upstream_avg_seconds", value, ts_ms).with_label("upstream", upstream),
        );
    }
    out
}

pub fn filtering_samples(status: &FilteringStatus, now: DateTime<Utc>, ts_ms: i64) -> Vec<Sample> {
    let filters = status.filters.as_deref().unwrap_or_default();
    let enabled: Vec<&Filter> = filters.iter().filter(|f| f.enabled).collect();
    let mut out = vec![
        gauge("adguard_filtering_enabled", flag(status.enabled), ts_ms),
        gauge("adguard_filters", filters.len() as f64, ts_ms),
        gauge("adguard_filters_enabled", enabled.len() as f64, ts_ms),
        gauge("adguard_filter_rules", enabled.iter().map(|f| f.rules_count).sum(), ts_ms),
    ];
    if let Some(hours) = status.interval {
        out.push(gauge("adguard_filter_update_interval_hours", hours, ts_ms));
    }
    let age = |filter: &Filter| -> Option<f64> {
        let updated = DateTime::parse_from_rfc3339(filter.last_updated.as_deref()?).ok()?;
        Some((now - updated.with_timezone(&Utc)).num_seconds().max(0) as f64)
    };
    // Une liste activée qui n'a jamais été téléchargée ne filtre rien.
    let never = enabled.iter().filter(|f| f.last_updated.is_none()).count();
    out.push(gauge("adguard_filters_never_updated", never as f64, ts_ms));
    if let Some(oldest) = enabled.iter().filter_map(|f| age(f)).reduce(f64::max) {
        out.push(gauge("adguard_filter_oldest_update_age_seconds", oldest, ts_ms));
    }
    for filter in enabled.iter().take(MAX_FILTERS) {
        let name = if filter.name.trim().is_empty() { &filter.url } else { &filter.name };
        let name = label(name);
        out.push(
            gauge("adguard_filter_rules_count", filter.rules_count, ts_ms)
                .with_label("filter", name.clone()),
        );
        if let Some(age) = age(filter) {
            out.push(
                gauge("adguard_filter_update_age_seconds", age, ts_ms).with_label("filter", name),
            );
        }
    }
    out
}

pub fn version_samples(check: &VersionCheck, current: &str, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge("adguard_update_check_enabled", flag(!check.disabled), ts_ms)];
    if check.disabled {
        return out;
    }
    let latest = check.new_version.as_deref().map(str::trim).filter(|v| !v.is_empty());
    let available = latest.is_some_and(|latest| is_newer(latest, current));
    let mut sample = gauge("adguard_update_available", flag(available), ts_ms);
    if let Some(latest) = latest {
        sample = sample.with_label("latest", latest);
    }
    out.push(sample);
    out
}

/// `v0.107.79` contre `v0.107.78` ; une préversion (`-b.2`) passe avant la
/// version finale. Une forme illisible compte comme nouvelle si elle diffère.
pub fn is_newer(latest: &str, current: &str) -> bool {
    fn parse(version: &str) -> Option<(Vec<u64>, bool)> {
        let version = version.trim().trim_start_matches('v');
        let (core, pre) = match version.split_once('-') {
            Some((core, _)) => (core, true),
            None => (version, false),
        };
        let parts = core.split('.').map(|p| p.parse().ok()).collect::<Option<Vec<u64>>>()?;
        Some((parts, pre))
    }
    match (parse(latest), parse(current)) {
        (Some((l, l_pre)), Some((c, c_pre))) => l > c || (l == c && c_pre && !l_pre),
        _ => latest.trim_start_matches('v') != current.trim_start_matches('v'),
    }
}

/// Résultat du test d'un serveur amont : `OK`, ou la phrase d'erreur
/// d'AdGuard Home ; `None` quand il n'a pas répondu dans le délai.
pub fn upstream_sample(upstream: &str, verdict: Option<&str>, ts_ms: i64) -> Sample {
    let ok = verdict.is_some_and(|v| v.trim().eq_ignore_ascii_case("ok"));
    gauge("adguard_upstream_up", flag(ok), ts_ms).with_label("upstream", label(upstream))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = include_str!("testdata/v0.107.79/status.json");
    const STATUS_PAUSED: &str = include_str!("testdata/v0.107.79/status_paused.json");
    const STATS: &str = include_str!("testdata/v0.107.79/stats.json");
    const STATS_CONFIG: &str = include_str!("testdata/v0.107.79/stats_config.json");
    const FILTERING: &str = include_str!("testdata/v0.107.79/filtering_status.json");
    const VERSION_DISABLED: &str = include_str!("testdata/v0.107.79/version_disabled.json");
    const VERSION_UPDATE: &str =
        include_str!("testdata/v0.107.79/version_update_from_v0.107.78.json");
    const VERSION_CURRENT: &str = include_str!("testdata/v0.107.79/version_up_to_date.json");
    const DNS_INFO: &str = include_str!("testdata/v0.107.79/dns_info.json");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn statut_reel() {
        let samples = status_samples(&serde_json::from_str(STATUS).unwrap(), 0);
        assert_eq!(value(&samples, "adguard_running"), Some(1.0));
        assert_eq!(value(&samples, "adguard_protection_enabled"), Some(1.0));
        assert_eq!(value(&samples, "adguard_protection_paused_seconds"), Some(0.0));
        let version = samples.iter().find(|s| s.metric == "adguard_version_info").unwrap();
        assert_eq!(version.labels["version"], "v0.107.79");
    }

    #[test]
    fn protection_en_pause() {
        let samples = status_samples(&serde_json::from_str(STATUS_PAUSED).unwrap(), 0);
        assert_eq!(value(&samples, "adguard_protection_enabled"), Some(0.0));
        let paused = value(&samples, "adguard_protection_paused_seconds").unwrap();
        assert!((583.0..=584.0).contains(&paused), "{paused}");
    }

    #[test]
    fn statistiques_reelles() {
        let stats: Stats = serde_json::from_str(STATS).unwrap();
        let config: StatsConfig = serde_json::from_str(STATS_CONFIG).unwrap();
        let samples = stats_samples(&stats, Some(&config), 0);
        assert_eq!(value(&samples, "adguard_dns_queries"), Some(18.0));
        assert_eq!(value(&samples, "adguard_blocked_filtering"), Some(6.0));
        let percent = value(&samples, "adguard_blocked_percent").unwrap();
        assert!((percent - 100.0 / 3.0).abs() < 1e-9);
        assert_eq!(value(&samples, "adguard_stats_window_seconds"), Some(86_400.0));
        assert_eq!(value(&samples, "adguard_stats_enabled"), Some(1.0));
        let upstream = samples.iter().find(|s| s.metric == "adguard_upstream_responses").unwrap();
        assert_eq!(upstream.labels["upstream"], "https://dns10.quad9.net:443/dns-query");
        assert!(value(&samples, "adguard_upstream_avg_seconds").unwrap() < 1.0);
        assert!(samples.iter().all(|s| s.kind == MetricKind::Gauge), "sommes sur une fenêtre");
    }

    #[test]
    fn sans_requete_pas_de_pourcentage() {
        let stats: Stats = serde_json::from_str("{}").unwrap();
        let samples = stats_samples(&stats, None, 0);
        assert!(value(&samples, "adguard_blocked_percent").is_none());
        assert_eq!(value(&samples, "adguard_dns_queries"), Some(0.0));
    }

    #[test]
    fn listes_de_filtrage_reelles() {
        let status: FilteringStatus = serde_json::from_str(FILTERING).unwrap();
        let now = DateTime::parse_from_rfc3339("2026-10-02T06:59:13Z").unwrap().to_utc();
        let samples = filtering_samples(&status, now, 0);
        assert_eq!(value(&samples, "adguard_filtering_enabled"), Some(1.0));
        assert_eq!(value(&samples, "adguard_filters"), Some(2.0));
        assert_eq!(value(&samples, "adguard_filters_enabled"), Some(1.0));
        assert!(value(&samples, "adguard_filter_rules").unwrap() > 100_000.0);
        assert_eq!(value(&samples, "adguard_filter_update_interval_hours"), Some(24.0));
        assert_eq!(value(&samples, "adguard_filters_never_updated"), Some(0.0));
        assert_eq!(value(&samples, "adguard_filter_oldest_update_age_seconds"), Some(172_800.0));
        let rules = samples.iter().find(|s| s.metric == "adguard_filter_rules_count").unwrap();
        assert_eq!(rules.labels["filter"], "AdGuard DNS filter");
        // La liste désactivée n'a pas de série.
        assert_eq!(samples.iter().filter(|s| s.metric == "adguard_filter_rules_count").count(), 1);
    }

    #[test]
    fn une_liste_jamais_telechargee_est_comptee() {
        let status: FilteringStatus = serde_json::from_str(
            r#"{"enabled":true,"interval":24,"filters":[{"name":"Neuve","url":"u","enabled":true,"rules_count":0}]}"#,
        )
        .unwrap();
        let samples = filtering_samples(&status, Utc::now(), 0);
        assert_eq!(value(&samples, "adguard_filters_never_updated"), Some(1.0));
        assert!(value(&samples, "adguard_filter_oldest_update_age_seconds").is_none());
        // `filters: null` sur une installation sans liste.
        let empty: FilteringStatus =
            serde_json::from_str(r#"{"enabled":false,"filters":null}"#).unwrap();
        let samples = filtering_samples(&empty, Utc::now(), 0);
        assert_eq!(value(&samples, "adguard_filters"), Some(0.0));
    }

    #[test]
    fn controle_de_version() {
        let disabled: VersionCheck = serde_json::from_str(VERSION_DISABLED).unwrap();
        let samples = version_samples(&disabled, "v0.107.79", 0);
        assert_eq!(value(&samples, "adguard_update_check_enabled"), Some(0.0));
        assert!(value(&samples, "adguard_update_available").is_none());

        let update: VersionCheck = serde_json::from_str(VERSION_UPDATE).unwrap();
        let samples = version_samples(&update, "v0.107.78", 0);
        assert_eq!(value(&samples, "adguard_update_available"), Some(1.0));
        let sample = samples.iter().find(|s| s.metric == "adguard_update_available").unwrap();
        assert_eq!(sample.labels["latest"], "v0.107.79");

        // À jour : AdGuard Home renvoie sa propre version comme « nouvelle ».
        let current: VersionCheck = serde_json::from_str(VERSION_CURRENT).unwrap();
        let samples = version_samples(&current, "v0.107.79", 0);
        assert_eq!(value(&samples, "adguard_update_available"), Some(0.0));
    }

    #[test]
    fn comparaison_de_versions() {
        assert!(is_newer("v0.107.80", "v0.107.79"));
        assert!(is_newer("v0.108.0", "v0.107.79"));
        assert!(!is_newer("v0.107.79", "v0.107.79"));
        assert!(!is_newer("v0.107.78", "v0.107.79"));
        assert!(is_newer("v0.108.0", "v0.108.0-b.91"));
        assert!(!is_newer("v0.108.0-b.91", "v0.108.0"));
        assert!(is_newer("edge", "v0.107.79"));
    }

    #[test]
    fn amonts_a_tester() {
        let info: DnsInfo = serde_json::from_str(DNS_INFO).unwrap();
        assert_eq!(info.upstreams(), ["https://dns10.quad9.net/dns-query"]);
        let info: DnsInfo = serde_json::from_str(
            r##"{"upstream_dns":["# commentaire","","1.1.1.1","[/lan/]192.168.1.1"]}"##,
        )
        .unwrap();
        assert_eq!(info.upstreams(), ["1.1.1.1", "[/lan/]192.168.1.1"]);
        assert_eq!(upstream_sample("1.1.1.1", Some("OK"), 0).value, 1.0);
        assert_eq!(upstream_sample("1.1.1.1", Some("couldn't communicate"), 0).value, 0.0);
        assert_eq!(upstream_sample("1.1.1.1", None, 0).value, 0.0);
    }
}
