//! Exporter Prometheus quelconque : une page `/metrics` lue telle quelle.
//!
//! node_exporter, windows_exporter, cAdvisor, blackbox_exporter, un exporter
//! maison : tout ce qui sert le format texte d'exposition Prometheus devient une
//! cible. Chaque ligne de la page devient une série `dumbmonit_<nom>` avec ses
//! étiquettes d'origine, plus `target=<id>` posée par le pipeline. Une étiquette
//! `target` ou `host` venue de l'exporter est renommée `exported_…` par
//! `apply_identity`, jamais écrasée.
//!
//! Le nombre de séries est borné (`max_series`) : un exporter qui publie des
//! étiquettes à cardinalité folle ne doit pas remplir VictoriaMetrics.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `preset` | `custom` | Exporter courant : fixe le port et le chemin par défaut. |
//! | `scheme` | `http` | Protocole. |
//! | `port` | celui du préréglage | Port de l'exporter. |
//! | `path` | `/metrics` | Chemin (et requête) lus, ex. `/probe?target=…&module=…`. |
//! | `keep` | vide | Expression régulière : seules les métriques dont le nom la satisfait sont gardées. |
//! | `drop` | vide | Expression régulière : les métriques dont le nom la satisfait sont écartées. |
//! | `max_series` | `5000` | Plafond de séries écrites par lecture. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai de la requête. |

use std::collections::HashMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use regex::{Regex, RegexBuilder};

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::observability::prom::Exposition;

pub const DEFAULT_PORT: u16 = 9100;
pub const DEFAULT_PATH: &str = "/metrics";
pub const DEFAULT_MAX_SERIES: usize = 5_000;
/// Plafond que l'utilisateur ne peut pas dépasser.
pub const HARD_MAX_SERIES: usize = 50_000;
/// Une page plus lourde n'est pas un exporter : on refuse de l'analyser.
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;
/// Taille compilée maximale d'une expression de filtre.
const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// Préréglages : clé, port par défaut, chemin par défaut.
pub const PRESETS: &[(&str, u16, &str)] = &[
    ("custom", DEFAULT_PORT, DEFAULT_PATH),
    ("node_exporter", 9100, "/metrics"),
    ("windows_exporter", 9182, "/metrics"),
    ("cadvisor", 8080, "/metrics"),
    ("blackbox_exporter", 9115, "/metrics"),
    ("snmp_exporter", 9116, "/metrics"),
    ("smartctl_exporter", 9633, "/metrics"),
    ("mysqld_exporter", 9104, "/metrics"),
    ("postgres_exporter", 9187, "/metrics"),
    ("redis_exporter", 9121, "/metrics"),
    ("nginx_exporter", 9113, "/metrics"),
    ("prometheus", 9090, "/metrics"),
];

#[derive(Default)]
pub struct PrometheusCollector;

impl PrometheusCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Réglages d'une lecture, tels que la cible les porte.
struct Settings {
    options: Options,
    path: String,
    keep: Option<Regex>,
    drop: Option<Regex>,
    max_series: usize,
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn preset(target: &Target) -> Result<(u16, &'static str), ProbeError> {
    let Some(name) = tag(target, "preset") else { return Ok((DEFAULT_PORT, DEFAULT_PATH)) };
    PRESETS
        .iter()
        .find(|(key, _, _)| *key == name)
        .map(|(_, port, path)| (*port, *path))
        .ok_or_else(|| ProbeError::Config(format!("Unknown exporter preset \"{name}\"")))
}

fn filter(target: &Target, key: &str) -> Result<Option<Regex>, ProbeError> {
    let Some(pattern) = tag(target, key) else { return Ok(None) };
    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map(Some)
        .map_err(|error| ProbeError::Config(format!("Invalid \"{key}\" expression: {error}")))
}

/// Sépare une adresse saisie sous forme d'URL complète (`http://hôte:9100/metrics`)
/// en racine et en chemin ; une adresse nue n'a pas de chemin.
fn split_address(address: &str) -> (String, Option<String>) {
    let address = address.trim();
    let Some(rest) = address.strip_prefix("https://").or_else(|| address.strip_prefix("http://"))
    else {
        return (address.to_string(), None);
    };
    let scheme_len = address.len() - rest.len();
    match rest.find(['/', '?']) {
        Some(at) if rest[at..].len() > 1 => {
            let split = scheme_len + at;
            let tail = &address[split..];
            let tail = if tail.starts_with('?') { format!("/{tail}") } else { tail.to_string() };
            (address[..split].to_string(), Some(tail))
        }
        Some(at) => (address[..scheme_len + at].to_string(), None),
        None => (address.to_string(), None),
    }
}

impl Settings {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (default_port, default_path) = preset(target)?;
        let (base, url_path) = split_address(&target.address);
        let mut for_options = target.clone();
        for_options.address = base;
        let options = Options::from_target(&for_options, default_port)?;
        let path = tag(target, "path").map(str::to_string).or(url_path);
        let mut path = path.unwrap_or_else(|| default_path.to_string());
        if !path.starts_with('/') {
            path.insert(0, '/');
        }
        let max_series = match tag(target, "max_series") {
            None => DEFAULT_MAX_SERIES,
            Some(raw) => {
                raw.parse::<usize>().ok().filter(|n| (1..=HARD_MAX_SERIES).contains(n)).ok_or_else(
                    || {
                        ProbeError::Config(format!(
                            "Series limit must be between 1 and {HARD_MAX_SERIES}, got \"{raw}\""
                        ))
                    },
                )?
            }
        };
        Ok(Self {
            options,
            path,
            keep: filter(target, "keep")?,
            drop: filter(target, "drop")?,
            max_series,
        })
    }
}

/// Lit la page et la convertit en échantillons.
async fn scrape(target: &Target) -> Result<Vec<Sample>, ProbeError> {
    let settings = Settings::from_target(target)?;
    let auth = Auth::from_credential(&target.credential)?;
    let http = crate::http::client(settings.options.insecure_tls)?;
    let client = HttpClient::new(
        http,
        settings.options.base_url.clone(),
        auth,
        settings.options.request_timeout,
        "The exporter",
    );
    let reply = client.get_raw(&settings.path).await?;
    if !reply.status.is_success() {
        return Err(client.status_error(reply.status, &reply.body, &settings.path));
    }
    if reply.body.len() > MAX_BODY_BYTES {
        return Err(ProbeError::Protocol(format!(
            "{} is larger than {} MiB: this is not a metrics page.",
            client.url(&settings.path),
            MAX_BODY_BYTES / (1024 * 1024)
        )));
    }
    let ts_ms = chrono::Utc::now().timestamp_millis();
    convert(&reply.body, &settings, ts_ms).ok_or_else(|| {
        ProbeError::Protocol(format!(
            "{} does not look like Prometheus metrics. Check the port and the path (/metrics \
             by default).",
            client.url(&settings.path)
        ))
    })
}

/// Types déclarés par les lignes `# TYPE nom type`.
fn declared_types(text: &str) -> HashMap<&str, &str> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.strip_prefix("# TYPE ")?.split_whitespace();
            Some((words.next()?, words.next()?))
        })
        .collect()
}

/// Nature d'une série : un compteur monotone ou une valeur instantanée.
///
/// Un histogramme et un résumé publient des compteurs (`_bucket`, `_count`,
/// `_sum`) ; sans déclaration, le suffixe `_total` fait foi, comme pour Prometheus.
fn kind_of(name: &str, types: &HashMap<&str, &str>) -> MetricKind {
    if let Some(declared) = types.get(name) {
        return if *declared == "counter" { MetricKind::Counter } else { MetricKind::Gauge };
    }
    for suffix in ["_bucket", "_count", "_sum"] {
        if let Some(base) = name.strip_suffix(suffix)
            && matches!(types.get(base), Some(&"histogram") | Some(&"summary"))
        {
            return MetricKind::Counter;
        }
    }
    if let Some(base) = name.strip_suffix("_total")
        && types.get(base).is_some_and(|t| *t == "counter")
    {
        return MetricKind::Counter;
    }
    if name.ends_with("_total") { MetricKind::Counter } else { MetricKind::Gauge }
}

/// Convertit la page ; `None` si ce n'est pas du format d'exposition.
fn convert(body: &str, settings: &Settings, ts_ms: i64) -> Option<Vec<Sample>> {
    let trimmed = body.trim_start();
    if trimmed.starts_with('<') {
        return None;
    }
    let page = Exposition::parse(body);
    if page.is_empty() {
        return None;
    }
    let types = declared_types(body);
    let mut out = Vec::new();
    let mut matched = 0usize;
    for line in page.lines() {
        if !line.value.is_finite() {
            continue;
        }
        if settings.keep.as_ref().is_some_and(|re| !re.is_match(&line.name)) {
            continue;
        }
        if settings.drop.as_ref().is_some_and(|re| re.is_match(&line.name)) {
            continue;
        }
        matched += 1;
        if out.len() >= settings.max_series {
            continue;
        }
        let mut sample = Sample::new(&line.name, line.value, kind_of(&line.name, &types), ts_ms);
        sample.labels = line.labels.clone();
        out.push(sample);
    }
    let dropped = matched - out.len();
    out.push(Sample::new("scrape_series", out.len() as f64, MetricKind::Gauge, ts_ms));
    out.push(Sample::new("scrape_series_dropped", dropped as f64, MetricKind::Gauge, ts_ms));
    Some(out)
}

#[async_trait]
impl Collector for PrometheusCollector {
    fn kind(&self) -> &'static str {
        "prometheus"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        scrape(target).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        scrape(target).await?;
        Ok(Some("prometheus".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get as route_get;
    use dumbmonit_proto::Credential;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    const PAGE: &str = "\
# HELP node_cpu_seconds_total CPU time.
# TYPE node_cpu_seconds_total counter
node_cpu_seconds_total{cpu=\"0\",mode=\"idle\"} 1234.5
node_cpu_seconds_total{cpu=\"0\",mode=\"user\"} 12.5
# TYPE node_load1 gauge
node_load1 0.42
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le=\"0.1\"} 3
http_request_duration_seconds_bucket{le=\"+Inf\"} 5
http_request_duration_seconds_sum 1.5
http_request_duration_seconds_count 5
go_goroutines NaN
";

    fn settings(tags: &[(&str, &str)]) -> Settings {
        Settings::from_target(&cible("prometheus", "exporter.lan", tags)).unwrap()
    }

    fn find<'a>(samples: &'a [Sample], name: &str) -> Vec<&'a Sample> {
        samples.iter().filter(|s| s.metric == name).collect()
    }

    #[test]
    fn la_page_est_convertie_avec_les_natures_declarees() {
        let samples = convert(PAGE, &settings(&[]), 7).unwrap();
        let cpu = find(&samples, "node_cpu_seconds_total");
        assert_eq!(cpu.len(), 2);
        assert_eq!(cpu[0].kind, MetricKind::Counter);
        assert_eq!(cpu[0].labels.get("mode").map(String::as_str), Some("idle"));
        assert_eq!(find(&samples, "node_load1")[0].kind, MetricKind::Gauge);
        for name in [
            "http_request_duration_seconds_bucket",
            "http_request_duration_seconds_sum",
            "http_request_duration_seconds_count",
        ] {
            assert_eq!(find(&samples, name)[0].kind, MetricKind::Counter, "{name}");
        }
        // Les NaN (marqueurs de péremption) ne sont pas écrits.
        assert!(find(&samples, "go_goroutines").is_empty());
        assert_eq!(find(&samples, "scrape_series")[0].value, 7.0);
    }

    #[test]
    fn keep_et_drop_filtrent_par_nom() {
        let samples = convert(PAGE, &settings(&[("keep", "^node_")]), 0).unwrap();
        assert!(find(&samples, "http_request_duration_seconds_sum").is_empty());
        assert_eq!(find(&samples, "node_load1").len(), 1);
        let samples = convert(PAGE, &settings(&[("drop", "_bucket$|^node_cpu")]), 0).unwrap();
        assert!(find(&samples, "node_cpu_seconds_total").is_empty());
        assert!(find(&samples, "http_request_duration_seconds_bucket").is_empty());
        assert_eq!(find(&samples, "node_load1").len(), 1);
    }

    #[test]
    fn la_limite_de_series_tronque_et_le_dit() {
        let samples = convert(PAGE, &settings(&[("max_series", "3")]), 0).unwrap();
        assert_eq!(find(&samples, "scrape_series")[0].value, 3.0);
        assert_eq!(find(&samples, "scrape_series_dropped")[0].value, 4.0);
    }

    #[test]
    fn une_page_html_ou_vide_n_est_pas_un_exporter() {
        assert!(convert("<html>hi</html>", &settings(&[]), 0).is_none());
        assert!(convert("", &settings(&[]), 0).is_none());
        assert!(convert("# HELP x y\n", &settings(&[]), 0).is_none());
    }

    #[test]
    fn les_reglages_invalides_sont_des_erreurs_de_configuration() {
        for tags in [&[("keep", "(")][..], &[("max_series", "0")], &[("preset", "inconnu")]] {
            let error = Settings::from_target(&cible("prometheus", "x", tags)).err().unwrap();
            assert!(!error.means_down(), "{error}");
        }
    }

    #[test]
    fn prereglage_adresse_url_et_chemin() {
        let s = settings(&[("preset", "windows_exporter")]);
        assert_eq!(s.options.base_url, "http://exporter.lan:9182");
        assert_eq!(s.path, "/metrics");
        let s = Settings::from_target(&cible(
            "prometheus",
            "https://bb.lan:9115/probe?target=x&module=http_2xx",
            &[],
        ))
        .unwrap();
        assert_eq!(s.options.base_url, "https://bb.lan:9115");
        assert_eq!(s.path, "/probe?target=x&module=http_2xx");
        let s = Settings::from_target(&cible("prometheus", "http://h:9100/", &[])).unwrap();
        assert_eq!(s.path, "/metrics");
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        address.to_string()
    }

    #[tokio::test]
    async fn lecture_avec_authentification_et_erreurs_distinguees() {
        let app = Router::new()
            .route(
                "/metrics",
                route_get(|headers: HeaderMap| async move {
                    let ok = headers.get("authorization").and_then(|v| v.to_str().ok())
                        == Some("Bearer secret");
                    if ok { (StatusCode::OK, PAGE) } else { (StatusCode::UNAUTHORIZED, "") }
                }),
            )
            .route("/html", route_get(|| async { "<html></html>" }));
        let address = serve(app).await;

        let mut target = cible("prometheus", &address, &[]);
        let denied = PrometheusCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(denied, ProbeError::Auth(_)) && !denied.means_down(), "{denied}");

        target.credential = Credential::ApiToken { token: "secret".into() };
        let samples = PrometheusCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "node_load1")[0].value, 0.42);

        let target = cible("prometheus", &address, &[("path", "/html")]);
        let wrong = PrometheusCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(wrong, ProbeError::Protocol(_)) && !wrong.means_down(), "{wrong}");
    }

    #[tokio::test]
    async fn un_exporter_injoignable_est_une_panne() {
        // Port fermé : on réserve un port puis on le libère.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let error = PrometheusCollector::new()
            .probe(&cible("prometheus", &address, &[]))
            .await
            .unwrap_err();
        assert!(error.means_down(), "{error}");
    }
}
