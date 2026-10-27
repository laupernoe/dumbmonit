//! Règles « motif de journal » : le nombre de lignes qui répondent à un filtre
//! LogsQL, compté par VictoriaLogs sur une fenêtre glissante.
//!
//! [`EngineSource`] réunit les deux sources du moteur : VictoriaMetrics pour les
//! requêtes de métriques, VictoriaLogs (quand le magasin embarqué tourne) pour
//! les motifs. Le résultat prend la forme d'une série ordinaire, `target`
//! portant l'identifiant de la cible : la machine à états, la suppression par
//! dépendance, les silences et les notifications s'appliquent sans changement.

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;

use crate::alerting::model::{MAX_LOG_WINDOW_SECS, MIN_LOG_WINDOW_SECS};
use crate::alerting::source::{MetricSource, SeriesPoint};
use crate::tsdb::Victoria;

/// Nom de la série produite.
pub const SERIES_NAME: &str = "dumbmonit_log_matches";

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder().timeout(Duration::from_secs(30)).build().unwrap_or_default()
});

pub struct EngineSource<'a> {
    metrics: &'a Victoria,
    logs_url: Option<String>,
}

impl<'a> EngineSource<'a> {
    pub fn new(metrics: &'a Victoria, logs_url: Option<String>) -> Self {
        Self { metrics, logs_url }
    }
}

#[async_trait]
impl MetricSource for EngineSource<'_> {
    async fn instant(&self, query: &str) -> Result<Vec<SeriesPoint>> {
        self.metrics.instant(query).await
    }

    async fn log_matches(
        &self,
        query: &str,
        window_secs: u64,
        per_target: bool,
    ) -> Result<Vec<SeriesPoint>> {
        let Some(base) = &self.logs_url else {
            bail!("the log store is off: turn it on in Settings > System");
        };
        let logsql = match_query(query, window_secs, per_target).map_err(anyhow::Error::msg)?;
        let response = CLIENT
            .post(format!("{base}/select/logsql/query"))
            .form(&[("query", logsql.as_str())])
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            bail!("the log store answered {status}: {}", text.trim());
        }
        Ok(parse_counts(&text, per_target, chrono::Utc::now().timestamp_millis()))
    }
}

/// La requête LogsQL qui compte les lignes d'un filtre sur la fenêtre écoulée.
pub fn match_query(user: &str, window_secs: u64, per_target: bool) -> Result<String, String> {
    let user = user.trim();
    if user.is_empty() {
        return Err("The log filter is required: without it, the rule watches nothing.".into());
    }
    if crate::logs::central::has_pipe(user) {
        return Err("Only filters are accepted: remove the `|` pipes from the query.".into());
    }
    let window = window_secs.clamp(MIN_LOG_WINDOW_SECS, MAX_LOG_WINDOW_SECS);
    let stats = if per_target {
        "stats by (target) count() as matches"
    } else {
        "stats count() as matches"
    };
    Ok(format!("_time:{window}s ({user}) | {stats}"))
}

/// Points issus de la réponse de VictoriaLogs (une ligne JSON par groupe).
pub fn parse_counts(text: &str, per_target: bool, now_ms: i64) -> Vec<SeriesPoint> {
    let mut points = Vec::new();
    let mut total = 0.0;
    for line in text.lines() {
        let Ok(object) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(line)
        else {
            continue;
        };
        let count = object
            .get("matches")
            .and_then(|value| {
                value.as_str().map(str::to_string).or_else(|| Some(value.to_string()))
            })
            .and_then(|raw| raw.parse::<f64>().ok())
            .unwrap_or(0.0);
        total += count;
        if !per_target {
            continue;
        }
        let target = object.get("target").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let mut labels = BTreeMap::new();
        labels.insert("__name__".to_string(), SERIES_NAME.to_string());
        if !target.is_empty() {
            labels.insert("target".to_string(), target);
        }
        points.push(SeriesPoint { labels, value: count, ts_ms: now_ms });
    }
    if !per_target {
        let labels = BTreeMap::from([("__name__".to_string(), SERIES_NAME.to_string())]);
        points.push(SeriesPoint { labels, value: total, ts_ms: now_ms });
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_counts_the_filter_over_the_window() {
        let per_target = match_query("service:=syslog \"Failed password\"", 300, true).unwrap();
        assert_eq!(
            per_target,
            "_time:300s (service:=syslog \"Failed password\") | stats by (target) count() as matches"
        );
        assert!(match_query("x", 5, false).unwrap().starts_with("_time:60s "));
        assert!(match_query("x", 10_000_000, false).unwrap().starts_with("_time:86400s "));
        assert!(match_query("  ", 300, true).is_err());
        assert!(match_query("error | stats count()", 300, true).is_err());
    }

    #[test]
    fn counts_become_one_series_per_target() {
        let text = "{\"target\":\"12\",\"matches\":\"5\"}\n{\"target\":\"\",\"matches\":\"2\"}\n";
        let points = parse_counts(text, true, 7);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].labels["target"], "12");
        assert_eq!(points[0].value, 5.0);
        assert!(!points[1].labels.contains_key("target"));
        assert_eq!(points[0].labels["__name__"], SERIES_NAME);
    }

    #[test]
    fn a_global_rule_always_yields_one_point() {
        let points = parse_counts("{\"matches\":\"3\"}\n", false, 1);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].value, 3.0);
        let empty = parse_counts("", false, 1);
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].value, 0.0);
        assert!(parse_counts("", true, 1).is_empty());
    }
}
