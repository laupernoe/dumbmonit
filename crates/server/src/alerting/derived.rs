//! Règles dérivées : prévision de remplissage et dérive de baseline.
//!
//! Ces deux types de règle ne se résument pas à une requête instantanée : ils
//! lisent de l'historique dans VictoriaMetrics, le passent aux calculs purs de
//! [`crate::alerting::forecast`] et [`crate::alerting::drift`], puis rendent des
//! séries ordinaires — un nombre de jours, un rapport à la normale — que la
//! machine à états compare au seuil de la règle comme n'importe quelle autre.
//! Phase, `for`, suppression par dépendance, silences, surcharges par
//! équipement et notifications s'appliquent donc sans exception.
//!
//! Ce qui accompagne chaque série (le score d'écart, le texte « usual 120 ms »,
//! l'autorisation de s'ouvrir) voyage à part, dans des [`Annotation`]s indexées
//! par empreinte.
//!
//! L'historique ne change pas d'un cycle à l'autre : il est relu au plus toutes
//! les dix minutes, ce qui garde la charge sur VictoriaMetrics à quelques
//! requêtes par règle et par quart d'heure.

use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, Mutex};

use anyhow::Result;
use chrono::{DateTime, TimeDelta, Utc};

use crate::alerting::capacity::{self, Family, Selection};
use crate::alerting::cycle::Annotation;
use crate::alerting::drift::{self, DriftInput};
use crate::alerting::forecast::{self, Forecast, Status};
use crate::alerting::model::{self, AnomalyParams, Rule};
use crate::alerting::source::{MetricSource, SeriesPoint, SeriesRange};

/// Durée de vie de l'historique relu.
const REFRESH: TimeDelta = TimeDelta::minutes(10);
/// Points visés par série sur la fenêtre d'une prévision : un par heure sur
/// sept jours.
const TARGET_POINTS: i64 = 168;
const SECS_PER_DAY: i64 = 86_400;
/// Fenêtre et pas de la médiane mobile d'une règle de dérive.
const ROLLING: &str = "7d:5m";

/// Séries rendues pour une règle, et ce qui les accompagne.
#[derive(Debug, Clone, Default)]
pub struct Derived {
    pub series: Vec<SeriesPoint>,
    pub annotations: HashMap<String, Annotation>,
}

/// La prévision d'une série.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesForecast {
    /// Identifiant de la famille, ou [`capacity::CUSTOM`].
    pub family: &'static str,
    pub labels: BTreeMap<String, String>,
    pub limit: f64,
    pub window_days: u32,
    pub forecast: Forecast,
}

type Cache<T> = LazyLock<Mutex<HashMap<String, (DateTime<Utc>, T)>>>;

static FORECASTS: Cache<Vec<SeriesForecast>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static BASELINES: Cache<HashMap<String, DriftInput>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn cached<T: Clone>(cache: &Cache<T>, key: &str, now: DateTime<Utc>) -> Option<T> {
    let guard = cache.lock().ok()?;
    let (at, value) = guard.get(key)?;
    let age = now.signed_duration_since(*at);
    (age >= TimeDelta::zero() && age < REFRESH).then(|| value.clone())
}

fn remember<T>(cache: &Cache<T>, key: String, now: DateTime<Utc>, value: T) {
    if let Ok(mut guard) = cache.lock() {
        // Les règles supprimées ou modifiées laissent des entrées : on fait le
        // ménage de ce qui a expiré à chaque écriture.
        guard.retain(|_, (at, _)| now.signed_duration_since(*at) < REFRESH);
        guard.insert(key, (now, value));
    }
}

fn cache_key(rule: &Rule) -> String {
    let params = serde_json::to_string(&rule.params).unwrap_or_default();
    format!("{}\u{1f}{}\u{1f}{params}", rule.uid, rule.query)
}

// --------------------------------------------------------------- prévision

/// Profondeur effective d'une famille, en jours.
pub fn window_days(params: &AnomalyParams, family: Option<&Family>) -> u32 {
    let asked = params
        .forecast_window_days
        .clamp(model::MIN_FORECAST_WINDOW_DAYS, model::MAX_FORECAST_WINDOW_DAYS);
    asked.max(family.map_or(0, |f| f.min_window_days))
}

/// Pas de la requête de plage : une heure, ou plus pour une longue fenêtre.
pub fn step_secs(window_days: u32) -> u64 {
    let window = i64::from(window_days) * SECS_PER_DAY;
    (window / TARGET_POINTS).max(3600) as u64
}

/// Prévisions de toutes les séries que vise `selection`.
///
/// `filter` restreint les séries (`{target="12"}` pour la page d'un
/// équipement). Une requête qui échoue fait échouer l'ensemble : une règle ne
/// doit pas résoudre les alertes d'une famille qu'elle n'a pas pu lire.
pub async fn collect_forecasts(
    source: &dyn MetricSource,
    selection: &Selection,
    params: &AnomalyParams,
    now: DateTime<Utc>,
    filter: Option<&str>,
) -> Result<Vec<SeriesForecast>> {
    let mut out = Vec::new();
    match selection {
        Selection::Custom(query) => {
            let days = window_days(params, None);
            for series in fetch(source, query, days, now, filter).await? {
                out.push(predict(capacity::CUSTOM, series, params.forecast_limit, days, now));
            }
        }
        Selection::Families(families) => {
            // Les noms nus d'une même profondeur partagent une seule requête ; on
            // les retrouve ensuite par `__name__`.
            let mut bare: BTreeMap<u32, Vec<&'static Family>> = BTreeMap::new();
            for family in families.iter().copied() {
                let days = window_days(params, Some(family));
                if capacity::is_bare_metric(family.percent) {
                    bare.entry(days).or_default().push(family);
                } else {
                    for series in fetch(source, family.percent, days, now, filter).await? {
                        out.push(predict(family.id, series, 100.0, days, now));
                    }
                }
            }
            for (days, group) in bare {
                let names: Vec<&str> = group.iter().map(|f| f.percent).collect();
                let query = format!("{{__name__=~\"{}\"}}", names.join("|"));
                for series in fetch(source, &query, days, now, filter).await? {
                    let name = series.labels.get("__name__").map(String::as_str);
                    let Some(family) = group.iter().find(|f| Some(f.percent) == name) else {
                        continue;
                    };
                    out.push(predict(family.id, series, 100.0, days, now));
                }
            }
        }
    }
    Ok(out)
}

/// Historique d'une expression sur `days` jours, jusqu'à `now`.
async fn fetch(
    source: &dyn MetricSource,
    query: &str,
    days: u32,
    now: DateTime<Utc>,
    filter: Option<&str>,
) -> Result<Vec<SeriesRange>> {
    let end = now.timestamp();
    let start = end - i64::from(days) * SECS_PER_DAY;
    source.range(query, start, end, step_secs(days), filter).await
}

fn predict(
    family: &'static str,
    series: SeriesRange,
    limit: f64,
    days: u32,
    now: DateTime<Utc>,
) -> SeriesForecast {
    let window = (i64::from(days) * SECS_PER_DAY) as f64;
    let forecast = forecast::forecast(&series.points, limit, now.timestamp() as f64, window);
    let mut labels = series.labels;
    labels.entry("family".to_string()).or_insert_with(|| family.to_string());
    SeriesForecast { family, labels, limit, window_days: days, forecast }
}

/// Évalue une règle de prévision : une série par entité dont la prévision est
/// assez sûre, de valeur « jours avant d'être plein ».
pub async fn forecast_rule(
    source: &dyn MetricSource,
    rule: &Rule,
    now: DateTime<Utc>,
) -> Result<Derived> {
    let key = cache_key(rule);
    let forecasts = match cached(&FORECASTS, &key, now) {
        Some(forecasts) => forecasts,
        None => {
            let selection = capacity::select(&rule.query);
            let fresh = collect_forecasts(source, &selection, &rule.params, now, None).await?;
            remember(&FORECASTS, key, now, fresh.clone());
            fresh
        }
    };
    Ok(forecast_series(rule, &forecasts, now))
}

/// Traduit des prévisions en séries pour la machine à états.
pub fn forecast_series(rule: &Rule, forecasts: &[SeriesForecast], now: DateTime<Utc>) -> Derived {
    let mut derived = Derived::default();
    for item in forecasts {
        let f = &item.forecast;
        let Some(days) = f.days_to_full else { continue };
        let sure = f.status == Status::Full || f.confidence >= rule.params.forecast_min_confidence;
        if !sure {
            continue;
        }
        let fingerprint = model::fingerprint(&rule.uid, &model::identity_key(&item.labels));
        let unit = if item.family == capacity::CUSTOM { "" } else { "%" };
        let detail = match f.growth_per_day {
            Some(growth) => format!(
                "at {}{unit}, {}{unit}/day, confidence {:.0}%",
                short(f.current),
                ShortSigned(growth),
                f.confidence * 100.0
            ),
            None => format!("at {}{unit}", short(f.current)),
        };
        derived
            .annotations
            .insert(fingerprint, Annotation { score: None, detail: Some(detail), armed: true });
        derived.series.push(SeriesPoint {
            labels: item.labels.clone(),
            value: (days * 10.0).round() / 10.0,
            ts_ms: now.timestamp_millis(),
        });
    }
    derived
}

// ------------------------------------------------------------------ dérive

/// Pas de la sous-requête de la fenêtre courante : quinze points par fenêtre,
/// entre quinze secondes et cinq minutes.
fn drift_step(window_secs: u64) -> u64 {
    (window_secs / 15).clamp(15, 300)
}

/// Requêtes d'une règle de dérive, exposées pour les tests.
pub fn drift_queries(query: &str, params: &AnomalyParams) -> (String, String, Vec<String>) {
    let window = params.drift_window_secs.clamp(60, 86_400);
    let step = drift_step(window);
    let range = format!("({query})[{window}s:{step}s]");
    let current = format!("avg_over_time({range})");
    let samples = format!("count_over_time({range})");
    let weeks = params.drift_weeks.min(model::MAX_DRIFT_WEEKS);
    let seasonal = (1..=weeks).map(|w| format!("avg_over_time({range} offset {w}w)")).collect();
    (current, samples, seasonal)
}

/// Évalue une règle de dérive : une série par mesure assez fournie, de valeur
/// « combien de fois la normale ».
pub async fn drift_rule(
    source: &dyn MetricSource,
    rule: &Rule,
    now: DateTime<Utc>,
) -> Result<Derived> {
    let (current_q, samples_q, seasonal_q) = drift_queries(&rule.query, &rule.params);
    let current = by_key(source.instant(&current_q).await?);
    let samples = by_key(source.instant(&samples_q).await?);

    let key = cache_key(rule);
    let baselines = match cached(&BASELINES, &key, now) {
        Some(baselines) => baselines,
        None => {
            let fresh = drift_baselines(source, &rule.query, &seasonal_q).await?;
            remember(&BASELINES, key, now, fresh.clone());
            fresh
        }
    };

    let mut derived = Derived::default();
    for (identity, (labels, value)) in current {
        let mut input = baselines.get(&identity).cloned().unwrap_or_default();
        input.current = value;
        input.samples = samples.get(&identity).map_or(0.0, |(_, count)| *count);
        let Some(verdict) = drift::evaluate(&input, &rule.params) else { continue };

        let fingerprint = model::fingerprint(&rule.uid, &model::identity_key(&labels));
        let reference = if verdict.seasonal {
            format!("same hour over the last {} weeks", input.seasonal.len())
        } else {
            "7-day median".to_string()
        };
        let detail =
            format!("now {}, usual {} ({reference})", short(value), short(verdict.baseline));
        derived.annotations.insert(
            fingerprint,
            Annotation { score: Some(verdict.score), detail: Some(detail), armed: verdict.armed },
        );
        derived.series.push(SeriesPoint {
            labels,
            value: (verdict.ratio * 100.0).round() / 100.0,
            ts_ms: now.timestamp_millis(),
        });
    }
    Ok(derived)
}

/// Les références d'une règle de dérive : semaines précédentes et médiane mobile.
async fn drift_baselines(
    source: &dyn MetricSource,
    query: &str,
    seasonal_q: &[String],
) -> Result<HashMap<String, DriftInput>> {
    let mut inputs: HashMap<String, DriftInput> = HashMap::new();
    for seasonal in seasonal_q {
        for (identity, (_, value)) in by_key(source.instant(seasonal).await?) {
            inputs.entry(identity).or_default().seasonal.push(value);
        }
    }
    let rolling = format!("({query})[{ROLLING}]");
    let medians = source.instant(&format!("median_over_time({rolling})")).await?;
    for (identity, (_, value)) in by_key(medians) {
        inputs.entry(identity).or_default().rolling_median = Some(value);
    }
    let spreads = source.instant(&format!("mad_over_time({rolling})")).await?;
    for (identity, (_, value)) in by_key(spreads) {
        inputs.entry(identity).or_default().rolling_mad = Some(value);
    }
    let history = source.instant(&format!("count_over_time({rolling})")).await?;
    for (identity, (_, value)) in by_key(history) {
        inputs.entry(identity).or_default().history = value;
    }
    Ok(inputs)
}

/// Indexe des points par leurs étiquettes sans nom de métrique : les fonctions
/// `*_over_time` le retirent, la requête brute le garde.
fn by_key(points: Vec<SeriesPoint>) -> HashMap<String, (BTreeMap<String, String>, f64)> {
    points
        .into_iter()
        .map(|point| (capacity::match_key(&point.labels), (point.labels, point.value)))
        .collect()
}

// --------------------------------------------------------------- affichage

/// Nombre court pour un message : trois chiffres significatifs suffisent.
fn short(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude >= 100.0 {
        format!("{value:.0}")
    } else if magnitude >= 1.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.3}")
    }
}

/// [`short`] avec son signe, pour une croissance.
struct ShortSigned(f64);

impl std::fmt::Display for ShortSigned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.0 < 0.0 { "-" } else { "+" };
        let magnitude = self.0.abs();
        let digits = if magnitude >= 10.0 {
            format!("{magnitude:.0}")
        } else if magnitude >= 0.1 {
            format!("{magnitude:.1}")
        } else {
            format!("{magnitude:.2}")
        };
        write!(f, "{sign}{digits}")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_trait::async_trait;

    use super::*;
    use crate::alerting::model::{Operator, RuleKind, Severity, TargetSelector};

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + seconds, 0).expect("valid timestamp")
    }

    fn rule(kind: RuleKind, query: &str) -> Rule {
        Rule {
            id: 1,
            uid: format!("derived_{query}"),
            name: "derived".to_string(),
            description: String::new(),
            kind,
            query: query.to_string(),
            operator: Operator::Lt,
            threshold: 30.0,
            clear_threshold: None,
            for_duration: Duration::ZERO,
            severity: Severity::Info,
            selector: TargetSelector::All,
            channels: Vec::new(),
            params: AnomalyParams::default(),
            unit: "d".to_string(),
            repeat_interval: None,
            escalate_after: None,
            enabled: true,
            builtin: false,
        }
    }

    fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    /// Source factice : un pool qui gagne un point par jour, et une latence
    /// quatre fois plus haute que d'habitude.
    struct Fake;

    #[async_trait]
    impl MetricSource for Fake {
        async fn instant(&self, query: &str) -> Result<Vec<SeriesPoint>> {
            let value = if query.starts_with("avg_over_time") && query.contains("offset") {
                0.1
            } else if query.starts_with("avg_over_time") {
                0.4
            } else if query.starts_with("count_over_time") && query.contains("7d") {
                2016.0
            } else if query.starts_with("count_over_time") {
                15.0
            } else if query.starts_with("median_over_time") {
                0.1
            } else if query.starts_with("mad_over_time") {
                0.01
            } else {
                return Ok(Vec::new());
            };
            Ok(vec![SeriesPoint {
                labels: labels(&[("target", "4"), ("url", "https://nas.lan")]),
                value,
                ts_ms: 0,
            }])
        }

        async fn range(
            &self,
            query: &str,
            start_s: i64,
            end_s: i64,
            step_secs: u64,
            _filter: Option<&str>,
        ) -> Result<Vec<SeriesRange>> {
            if !query.contains("dumbmonit_truenas_pool_used_percent") {
                return Ok(Vec::new());
            }
            let points = (start_s..=end_s)
                .step_by(step_secs as usize)
                .map(|t| (t as f64, 60.0 + (t - start_s) as f64 / 86_400.0))
                .collect();
            Ok(vec![SeriesRange {
                labels: labels(&[
                    ("__name__", "dumbmonit_truenas_pool_used_percent"),
                    ("target", "4"),
                    ("pool", "tank"),
                ]),
                points,
            }])
        }
    }

    #[tokio::test]
    async fn la_regle_de_prevision_rend_des_jours_avant_plein() {
        let rule = rule(RuleKind::Forecast, "truenas_pool");
        let derived = forecast_rule(&Fake, &rule, at(0)).await.expect("evaluated");
        assert_eq!(derived.series.len(), 1);
        let point = &derived.series[0];
        // 60 % il y a sept jours, +1 point par jour : 67 %, plein dans 33 jours.
        assert!((point.value - 33.0).abs() < 0.2, "days = {}", point.value);
        assert_eq!(point.labels.get("family").map(String::as_str), Some("truenas_pool"));
        let fingerprint = model::fingerprint(&rule.uid, &model::identity_key(&point.labels));
        let annotation = derived.annotations.get(&fingerprint).expect("annotated");
        assert!(annotation.detail.as_deref().is_some_and(|d| d.contains("/day")), "{annotation:?}");
    }

    #[tokio::test]
    async fn une_prevision_peu_sure_ne_rend_pas_de_serie() {
        let mut rule = rule(RuleKind::Forecast, "truenas_pool");
        rule.params.forecast_min_confidence = 1.01;
        let derived = forecast_rule(&Fake, &rule, at(0)).await.expect("evaluated");
        assert!(derived.series.is_empty());
    }

    #[tokio::test]
    async fn la_regle_de_derive_rend_le_rapport_a_la_normale() {
        let rule = rule(RuleKind::Drift, "dumbmonit_probe_duration_seconds");
        let derived = drift_rule(&Fake, &rule, at(0)).await.expect("evaluated");
        assert_eq!(derived.series.len(), 1);
        assert!((derived.series[0].value - 4.0).abs() < 1e-9, "{}", derived.series[0].value);
        let annotation = derived.annotations.values().next().expect("annotated");
        assert!(annotation.armed);
        assert!(annotation.score.is_some_and(|s| s > 3.5));
        assert!(annotation.detail.as_deref().is_some_and(|d| d.contains("usual 0.100")));
    }

    #[test]
    fn les_requetes_de_derive_comparent_les_memes_heures() {
        let (current, samples, seasonal) =
            drift_queries("dumbmonit_probe_duration_seconds", &AnomalyParams::default());
        assert_eq!(current, "avg_over_time((dumbmonit_probe_duration_seconds)[900s:60s])");
        assert_eq!(samples, "count_over_time((dumbmonit_probe_duration_seconds)[900s:60s])");
        assert_eq!(seasonal.len(), 4);
        assert_eq!(
            seasonal[3],
            "avg_over_time((dumbmonit_probe_duration_seconds)[900s:60s] offset 4w)"
        );
    }

    #[test]
    fn la_fenetre_tient_compte_de_la_famille() {
        let params = AnomalyParams::default();
        assert_eq!(window_days(&params, None), 7);
        assert_eq!(window_days(&params, capacity::family("ups_battery")), 30);
        assert_eq!(step_secs(7), 3600);
        assert_eq!(step_secs(30), 30 * 86_400 / 168);
    }

    #[test]
    fn les_nombres_courts_restent_lisibles() {
        assert_eq!(short(92.44), "92.4");
        assert_eq!(short(0.1234), "0.123");
        assert_eq!(short(1234.5), "1234");
        assert_eq!(ShortSigned(0.84).to_string(), "+0.8");
        assert_eq!(ShortSigned(-0.042).to_string(), "-0.04");
    }
}
