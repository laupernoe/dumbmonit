//! Lecture des séries temporelles.
//!
//! Ces routes relaient les requêtes vers VictoriaMetrics plutôt que de l'exposer
//! directement : l'interface n'a ainsi qu'une seule origine à contacter, et le jour
//! où l'authentification sera en place, elle protégera aussi l'accès aux données.

use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::teams::{self, TeamAccess};
use crate::state::AppState;

/// Nombre maximal de points par série renvoyés à l'interface.
///
/// Au-delà, le navigateur peine autant que le réseau, et l'œil ne distingue plus
/// rien : on élargit le pas plutôt que de tout transmettre.
const MAX_POINTS: i64 = 2_000;

/// Longueur maximale d'une expression MetricsQL, en octets.
///
/// Les requêtes de l'interface tiennent en quelques centaines d'octets ; au-delà
/// de quelques kilo-octets, c'est une expression construite pour occuper
/// VictoriaMetrics, pas pour lire un graphe.
const MAX_QUERY_LEN: usize = 4 * 1024;

/// Plage maximale d'une requête sur intervalle : un peu plus d'un an, la
/// rétention par défaut. Plus large, la requête parcourrait des séries qui
/// n'existent pas et le pas serait de toute façon élargi jusqu'à l'absurde.
const MAX_RANGE_MS: i64 = 400 * 24 * 3_600_000;

fn validate_query(query: &str) -> ApiResult<()> {
    if query.trim().is_empty() {
        return Err(ApiError::BadRequest("The query is empty.".into()));
    }
    if query.len() > MAX_QUERY_LEN {
        return Err(ApiError::BadRequest(format!(
            "The query is too long ({} bytes, maximum {MAX_QUERY_LEN}).",
            query.len()
        )));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    /// Expression MetricsQL.
    pub query: String,
    /// Bornes en millisecondes depuis l'époque Unix.
    pub start: i64,
    pub end: i64,
    /// Pas d'échantillonnage en secondes. Ajusté si la plage est trop large.
    #[serde(default)]
    pub step: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SeriesView {
    pub metric: std::collections::BTreeMap<String, String>,
    /// Couples `[horodatage_secondes, "valeur"]`, au format de l'API Prometheus.
    pub values: Vec<(f64, String)>,
}

/// Un refus de VictoriaMetrics est une requête invalide (400) ; un backend
/// injoignable, trop lent ou illisible est une panne côté serveur (502).
fn backend_error(error: anyhow::Error) -> ApiError {
    if error.downcast_ref::<crate::tsdb::QueryRejected>().is_some() {
        ApiError::BadRequest(format!("{error:#}"))
    } else {
        tracing::warn!(%error, "VictoriaMetrics query failed");
        ApiError::BadGateway("The metrics backend is not answering.".into())
    }
}

pub async fn query_range(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Query(params): Query<RangeQuery>,
) -> ApiResult<Json<Vec<SeriesView>>> {
    validate_query(&params.query)?;
    if params.end <= params.start {
        return Err(ApiError::BadRequest("The end of the range must be after its start.".into()));
    }
    if params.end - params.start > MAX_RANGE_MS {
        return Err(ApiError::BadRequest(
            "The range is too wide: query at most 400 days at a time.".into(),
        ));
    }

    let step = resolve_step(params.start, params.end, params.step);
    // Un compte restreint par ses équipes ne lit que les séries de ses
    // équipements : le filtre est ajouté par VictoriaMetrics à chaque sélecteur.
    let filter = teams::series_filter(access.visible_targets(&state.pool).await?.as_ref());

    let series = state
        .victoria
        .query_range_filtered(&params.query, params.start, params.end, step, filter.as_deref())
        .await
        .map_err(backend_error)?;

    Ok(Json(
        series.into_iter().map(|s| SeriesView { metric: s.metric, values: s.values }).collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct InstantQuery {
    pub query: String,
}

pub async fn query(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Query(params): Query<InstantQuery>,
) -> ApiResult<Json<Vec<SeriesView>>> {
    validate_query(&params.query)?;

    let filter = teams::series_filter(access.visible_targets(&state.pool).await?.as_ref());
    let series = state
        .victoria
        .query_fresh_filtered(&params.query, filter.as_deref())
        .await
        .map_err(backend_error)?;

    Ok(Json(
        series
            .into_iter()
            .map(|s| SeriesView { metric: s.metric, values: vec![s.value] })
            .collect(),
    ))
}

/// Séries lues pour dresser le catalogue : au-delà, un parc exceptionnel serait
/// décrit par ses premières séries seulement (`truncated`).
const CATALOG_SERIES_LIMIT: usize = 50_000;
/// Valeurs d'étiquette montrées par métrique : de quoi reconnaître l'étiquette.
const CATALOG_VALUES_PER_LABEL: usize = 12;
/// Fenêtre où une série compte comme vivante.
const CATALOG_WINDOW_SECS: i64 = 3600;
const CATALOG_MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Serialize, PartialEq)]
pub struct LabelInfo {
    pub name: String,
    /// Quelques valeurs vues, triées.
    pub values: Vec<String>,
    /// Il y en a d'autres que celles listées.
    pub more: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MetricInfo {
    pub name: String,
    pub series: usize,
    pub labels: Vec<LabelInfo>,
}

#[derive(Debug, Serialize)]
pub struct Catalog {
    pub metrics: Vec<MetricInfo>,
    /// La liste des séries a été coupée à [`CATALOG_SERIES_LIMIT`].
    pub truncated: bool,
}

/// Regroupe des séries (étiquettes, nom sous `__name__`) par métrique.
fn summarize(series: &[serde_json::Map<String, serde_json::Value>]) -> Vec<MetricInfo> {
    use std::collections::{BTreeMap, BTreeSet};
    type Seen = (usize, BTreeMap<String, BTreeSet<String>>);
    let mut by_name: BTreeMap<String, Seen> = BTreeMap::new();
    for entry in series {
        let Some(name) = entry.get("__name__").and_then(|v| v.as_str()) else { continue };
        if !name.starts_with("dumbmonit_") {
            continue;
        }
        let seen = by_name.entry(name.to_string()).or_default();
        seen.0 += 1;
        for (label, value) in entry {
            if label == "__name__" {
                continue;
            }
            if let Some(value) = value.as_str() {
                seen.1.entry(label.clone()).or_default().insert(value.to_string());
            }
        }
    }
    by_name
        .into_iter()
        .map(|(name, (series, labels))| MetricInfo {
            name,
            series,
            labels: labels
                .into_iter()
                .map(|(name, values)| LabelInfo {
                    more: values.len() > CATALOG_VALUES_PER_LABEL,
                    values: values.into_iter().take(CATALOG_VALUES_PER_LABEL).collect(),
                    name,
                })
                .collect(),
        })
        .collect()
}

/// `GET /api/metrics/catalog` : les métriques `dumbmonit_*` vivantes et leurs
/// étiquettes, pour l'explorateur de métriques. Une seule lecture de
/// `/api/v1/series` côté VictoriaMetrics, résumée ici.
pub async fn catalog(State(state): State<AppState>) -> ApiResult<Json<Catalog>> {
    let since = chrono::Utc::now().timestamp() - CATALOG_WINDOW_SECS;
    let query = reqwest::Url::parse_with_params(
        "http://catalog.invalid/",
        [
            ("match[]", "{__name__=~\"dumbmonit_.*\"}".to_string()),
            ("start", since.to_string()),
            ("limit", (CATALOG_SERIES_LIMIT + 1).to_string()),
        ],
    )
    .map_err(anyhow::Error::from)?
    .query()
    .map(str::to_string);
    let (status, body) = state
        .victoria
        .proxy_read(&["series"], query.as_deref(), None, CATALOG_MAX_BYTES)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "VictoriaMetrics catalog failed");
            ApiError::BadGateway("The metrics backend is not answering.".into())
        })?;
    if status != 200 {
        return Err(ApiError::BadGateway("The metrics backend refused the listing.".into()));
    }
    #[derive(Deserialize)]
    struct Series {
        data: Vec<serde_json::Map<String, serde_json::Value>>,
    }
    let parsed: Series = serde_json::from_str(&body).map_err(|_| {
        ApiError::BadGateway("The metrics backend sent an unreadable answer.".into())
    })?;
    let truncated = parsed.data.len() > CATALOG_SERIES_LIMIT;
    let slice = &parsed.data[..parsed.data.len().min(CATALOG_SERIES_LIMIT)];
    Ok(Json(Catalog { metrics: summarize(slice), truncated }))
}

/// Choisit un pas qui garde le nombre de points sous [`MAX_POINTS`].
///
/// Le pas demandé par l'interface est respecté tant qu'il reste raisonnable ; sur
/// une plage d'un an, il est élargi silencieusement plutôt que de renvoyer une
/// erreur que l'utilisateur ne saurait pas corriger.
fn resolve_step(start_ms: i64, end_ms: i64, requested: Option<u64>) -> u64 {
    let span_secs = ((end_ms - start_ms) / 1000).max(1);
    let minimum = (span_secs / MAX_POINTS).max(1) as u64;
    match requested {
        Some(step) if step >= minimum => step,
        _ => minimum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3_600_000;

    #[test]
    fn the_catalog_groups_series_by_metric_and_keeps_only_ours() {
        let series: Vec<_> = [
            serde_json::json!({"__name__": "dumbmonit_cpu", "target": "1", "core": "0"}),
            serde_json::json!({"__name__": "dumbmonit_cpu", "target": "2", "core": "0"}),
            serde_json::json!({"__name__": "other_metric", "target": "2"}),
        ]
        .into_iter()
        .map(|v| v.as_object().unwrap().clone())
        .collect();
        let found = summarize(&series);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "dumbmonit_cpu");
        assert_eq!(found[0].series, 2);
        assert_eq!(found[0].labels[0].name, "core");
        assert_eq!(found[0].labels[1].values, vec!["1", "2"]);
    }

    #[test]
    fn honours_a_reasonable_requested_step() {
        assert_eq!(resolve_step(0, HOUR, Some(60)), 60);
    }

    #[test]
    fn widens_a_step_that_would_return_too_many_points() {
        // Un an à un pas de 10 secondes représenterait plus de trois millions de
        // points ; le pas est élargi pour rester sous la limite.
        let year = 365 * 24 * HOUR;
        let step = resolve_step(0, year, Some(10));
        assert!(step > 10);
        assert!((year / 1000) / step as i64 <= MAX_POINTS);
    }

    #[test]
    fn picks_a_step_on_its_own_when_none_is_given() {
        assert_eq!(resolve_step(0, HOUR, None), 1);
    }

    #[test]
    fn an_oversized_query_is_refused() {
        assert!(validate_query("up").is_ok());
        assert!(validate_query("   ").is_err());
        assert!(validate_query(&"a".repeat(MAX_QUERY_LEN + 1)).is_err());
    }

    #[test]
    fn never_returns_a_zero_step() {
        assert_eq!(resolve_step(0, 1, None), 1);
    }
}
