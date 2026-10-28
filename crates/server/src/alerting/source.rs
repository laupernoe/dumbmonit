//! Accès aux séries temporelles, derrière un trait.
//!
//! Le moteur ne connaît que [`MetricSource`] : c'est ce qui permet de tester tout le
//! cycle d'évaluation — machine à états, suppression, silences, regroupement — sans
//! jamais ouvrir de socket vers VictoriaMetrics.

use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::tsdb::{InstantSeries, RangeSeries, Victoria};

/// Un point instantané : ses étiquettes, sa valeur.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesPoint {
    pub labels: BTreeMap<String, String>,
    pub value: f64,
    pub ts_ms: i64,
}

/// Une série sur une plage : ses étiquettes, ses points `(secondes, valeur)`.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesRange {
    pub labels: BTreeMap<String, String>,
    pub points: Vec<(f64, f64)>,
}

#[async_trait]
pub trait MetricSource: Send + Sync {
    /// Exécute une requête MetricsQL instantanée.
    async fn instant(&self, query: &str) -> Result<Vec<SeriesPoint>>;

    /// Compte les lignes de journal qui répondent à un filtre LogsQL sur la
    /// fenêtre écoulée : un point par cible (`target`), ou un seul si `per_target`
    /// est faux. Les sources sans magasin de journaux répondent par une erreur.
    async fn log_matches(
        &self,
        _query: &str,
        _window_secs: u64,
        _per_target: bool,
    ) -> Result<Vec<SeriesPoint>> {
        anyhow::bail!("the log store is not available")
    }

    /// Exécute une requête MetricsQL sur une plage (secondes Unix), restreinte
    /// aux séries qui correspondent à `filter` quand il est donné. Sert aux
    /// règles de prévision. Les sources sans historique répondent par une erreur,
    /// ce qui gèle les alertes de la règle au lieu de les résoudre.
    async fn range(
        &self,
        _query: &str,
        _start_s: i64,
        _end_s: i64,
        _step_secs: u64,
        _filter: Option<&str>,
    ) -> Result<Vec<SeriesRange>> {
        anyhow::bail!("range queries are not available")
    }
}

#[async_trait]
impl MetricSource for Victoria {
    async fn instant(&self, query: &str) -> Result<Vec<SeriesPoint>> {
        Ok(self.query(query).await?.into_iter().filter_map(convert).collect())
    }

    async fn range(
        &self,
        query: &str,
        start_s: i64,
        end_s: i64,
        step_secs: u64,
        filter: Option<&str>,
    ) -> Result<Vec<SeriesRange>> {
        let series = self
            .query_range_filtered(query, start_s * 1000, end_s * 1000, step_secs, filter)
            .await?;
        Ok(series.into_iter().map(convert_range).collect())
    }
}

/// Convertit une série de plage ; les points illisibles ou non finis sont
/// écartés, comme pour les requêtes instantanées.
fn convert_range(series: RangeSeries) -> SeriesRange {
    let points = series
        .values
        .into_iter()
        .filter_map(|(t, raw)| raw.parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| (t, v)))
        .collect();
    SeriesRange { labels: series.metric, points }
}

/// Convertit une série renvoyée par l'API Prometheus.
///
/// Les valeurs illisibles ou non finies sont écartées plutôt que remontées en
/// erreur : un `NaN` isolé dans une réponse ne doit pas priver d'évaluation les
/// centaines d'autres séries du même lot.
fn convert(series: InstantSeries) -> Option<SeriesPoint> {
    let (timestamp, raw) = series.value;
    let value: f64 = raw.parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(SeriesPoint { labels: series.metric, value, ts_ms: (timestamp * 1000.0).round() as i64 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(value: &str) -> InstantSeries {
        InstantSeries {
            metric: [("__name__".to_string(), "dumbmonit_up".to_string())].into_iter().collect(),
            value: (1_700_000_000.0, value.to_string()),
        }
    }

    #[test]
    fn une_valeur_numerique_est_convertie_avec_son_horodatage() {
        let point = convert(series("42.5")).expect("readable value");
        assert_eq!(point.value, 42.5);
        assert_eq!(point.ts_ms, 1_700_000_000_000);
        assert_eq!(point.labels.get("__name__").map(String::as_str), Some("dumbmonit_up"));
    }

    #[test]
    fn une_serie_de_plage_garde_ses_points_lisibles() {
        let series = RangeSeries {
            metric: [("pool".to_string(), "tank".to_string())].into_iter().collect(),
            values: vec![
                (1.0, "10".to_string()),
                (2.0, "NaN".to_string()),
                (3.0, "12.5".to_string()),
            ],
        };
        let range = convert_range(series);
        assert_eq!(range.points, vec![(1.0, 10.0), (3.0, 12.5)]);
        assert_eq!(range.labels.get("pool").map(String::as_str), Some("tank"));
    }

    #[test]
    fn les_valeurs_illisibles_ou_non_finies_sont_ecartees() {
        assert!(convert(series("NaN")).is_none());
        assert!(convert(series("+Inf")).is_none());
        assert!(convert(series("not a number")).is_none());
    }
}
