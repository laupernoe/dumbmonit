//! Prévisions de remplissage d'un équipement : `GET /targets/{id}/forecasts`.
//!
//! Pour chaque entité de capacité de l'équipement (point de montage, datastore,
//! pool, volume, licence…), le taux d'occupation, les grandeurs brutes quand
//! l'intégration les publie, la croissance par jour et « plein dans N jours »,
//! avec la confiance du calcul. C'est le même moteur que la règle livrée
//! `capacity_forecast` ([`crate::alerting::derived`]) et ses réglages
//! (profondeur, paliers de sévérité) : ce que la page affiche est ce qui
//! alerterait.

use std::collections::{BTreeMap, HashMap};

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{TimeDelta, Utc};
use serde::Serialize;

use crate::alerting::capacity::{self, Selection};
use crate::alerting::derived::{self, SeriesForecast};
use crate::alerting::forecast::{self, Status};
use crate::alerting::model::{AnomalyParams, Severity};
use crate::alerting::rules::RULE_CAPACITY_FORECAST;
use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;

/// Profondeur de recherche de la dernière valeur brute (`used`, `size`) : un
/// NAS interrogé toutes les dix minutes a des séries plus vieilles que la
/// fenêtre par défaut d'une requête instantanée.
const LOOKBACK: &str = "1h";

pub fn routes() -> Router<AppState> {
    Router::new().route("/targets/{id}/forecasts", get(list))
}

/// Prévision d'une entité.
#[derive(Debug, Serialize)]
pub struct ForecastView {
    /// Famille de capacité (`filesystem`, `pbs_datastore`…).
    pub family: String,
    /// Nom affiché de la famille.
    pub family_label: String,
    /// Nom de la métrique du pourcentage, ou la famille pour une expression.
    pub metric: String,
    /// Entité lisible : `/srv`, `tank`, `volume1`… Vide pour l'équipement entier.
    pub entity: String,
    /// Étiquettes de la série, sans le nom de métrique.
    pub labels: BTreeMap<String, String>,
    /// Taux d'occupation actuel, en pourcentage.
    pub used_percent: Option<f64>,
    /// Grandeur utilisée, dans `unit`, si l'intégration la publie.
    pub used: Option<f64>,
    /// Grandeur totale, dans `unit`.
    pub size: Option<f64>,
    /// `bytes`, `seats`… Vide sans grandeur brute.
    pub unit: String,
    /// Croissance, en points de pourcentage par jour.
    pub growth_per_day: Option<f64>,
    /// Jours avant d'être plein ; `null` sans croissance ou sans historique.
    pub days_to_full: Option<f64>,
    /// Date estimée du remplissage, UTC sans suffixe.
    pub full_at: Option<String>,
    /// Confiance de 0 à 1.
    pub confidence: f64,
    pub status: Status,
    /// Sévérité que l'alerte de prévision aurait aujourd'hui ; `null` quand la
    /// règle ne se déclencherait pas (trop loin, trop incertain, règle éteinte).
    pub severity: Option<Severity>,
    /// Profondeur d'historique régressée, en jours.
    pub window_days: u32,
}

async fn list(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Vec<ForecastView>>> {
    let exists = sqlx::query("SELECT 1 FROM targets WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(anyhow::Error::from)?
        .is_some();
    if !exists {
        return Err(ApiError::NotFound(format!("Monitor {id} not found.")));
    }

    // Les réglages de la règle livrée, si elle existe encore : la page dit alors
    // exactement ce que la règle ferait.
    let rule = db::alerts::list_rules(&state.pool)
        .await?
        .into_iter()
        .find(|rule| rule.uid == RULE_CAPACITY_FORECAST);
    let params = rule.as_ref().map_or_else(AnomalyParams::default, |rule| rule.params);

    let now = Utc::now();
    let filter = format!("{{target=\"{id}\"}}");
    let selection = Selection::Families(capacity::FAMILIES.iter().collect());
    let forecasts =
        derived::collect_forecasts(&state.victoria, &selection, &params, now, Some(&filter))
            .await
            .map_err(|error| {
                tracing::warn!(target_id = id, ?error, "forecasts not computed");
                ApiError::BadGateway("The time-series database did not answer.".into())
            })?;

    let raw = raw_sizes(&state, &forecasts, &filter).await;
    let verdict = |item: &SeriesForecast| -> Option<Severity> {
        let rule = rule.as_ref().filter(|rule| rule.enabled)?;
        let f = &item.forecast;
        let days = f.days_to_full?;
        let sure = f.status == Status::Full || f.confidence >= params.forecast_min_confidence;
        let fires = sure && rule.operator.test(days, rule.threshold);
        fires.then(|| forecast::tier(rule.severity, Some(days), &params))
    };

    let mut views: Vec<ForecastView> = forecasts
        .iter()
        .map(|item| {
            let family = capacity::family(item.family);
            let key = capacity::match_key(&item.labels);
            let (used, size) = raw.get(&key).copied().unwrap_or((None, None));
            let f = &item.forecast;
            let mut labels = item.labels.clone();
            let metric = labels.remove("__name__").unwrap_or_else(|| item.family.to_string());
            labels.remove("family");
            ForecastView {
                family: item.family.to_string(),
                family_label: family.map_or_else(|| item.family.to_string(), |f| f.label.into()),
                metric,
                entity: capacity::entity(&item.labels),
                labels,
                used_percent: Some(f.current).filter(|v| v.is_finite()),
                used,
                size,
                unit: family.map_or("", |f| f.unit).to_string(),
                growth_per_day: f.growth_per_day,
                days_to_full: f.days_to_full,
                full_at: f.days_to_full.map(|days| {
                    let at = now + TimeDelta::seconds((days * 86_400.0) as i64);
                    at.format("%Y-%m-%dT%H:%M:%S").to_string()
                }),
                confidence: f.confidence,
                status: f.status,
                severity: verdict(item),
                window_days: item.window_days,
            }
        })
        .collect();

    // Ce qui se remplit le plus tôt d'abord, puis le plus plein.
    views.sort_by(|a, b| {
        let soon = |v: &ForecastView| v.days_to_full.unwrap_or(f64::INFINITY);
        soon(a)
            .total_cmp(&soon(b))
            .then_with(|| b.used_percent.unwrap_or(0.0).total_cmp(&a.used_percent.unwrap_or(0.0)))
            .then_with(|| a.entity.cmp(&b.entity))
    });
    Ok(Json(views))
}

/// Dernières grandeurs brutes (`used`, `size`) des familles qui les publient,
/// par clé de rapprochement. Une lecture qui échoue laisse les champs à `null` :
/// le pourcentage et la prévision suffisent à la page.
async fn raw_sizes(
    state: &AppState,
    forecasts: &[SeriesForecast],
    filter: &str,
) -> HashMap<String, (Option<f64>, Option<f64>)> {
    let mut out: HashMap<String, (Option<f64>, Option<f64>)> = HashMap::new();
    let mut families: Vec<&str> = forecasts.iter().map(|f| f.family).collect();
    families.sort_unstable();
    families.dedup();
    for family in families.into_iter().filter_map(capacity::family) {
        for (expr, is_used) in [(family.used, true), (family.size, false)] {
            let Some(expr) = expr else { continue };
            let query = format!("last_over_time(({expr})[{LOOKBACK}:])");
            let series = match state.victoria.query_fresh_filtered(&query, Some(filter)).await {
                Ok(series) => series,
                Err(error) => {
                    tracing::debug!(family = family.id, ?error, "raw capacity not read");
                    continue;
                }
            };
            for item in series {
                let parsed = item.value.1.parse::<f64>().ok().filter(|v| v.is_finite());
                let Some(value) = parsed else { continue };
                let entry = out.entry(capacity::match_key(&item.metric)).or_default();
                if is_used {
                    entry.0 = Some(value);
                } else {
                    entry.1 = Some(value);
                }
            }
        }
    }
    out
}
