//! Santé et référencement d'un site (`crate::website`) : la dernière lecture
//! (adresse finale, redirections, métadonnées et leur état de confirmation)
//! et l'historique des changements confirmés.
//!
//! Une cible qui n'est pas de type `website` répond 404, comme une cible
//! inexistante.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;
use crate::website::{self, LastProbe, changes, store};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/website", get(overview))
        .route("/targets/{id}/website/changes", get(history))
}

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Serialize)]
struct Overview {
    /// Dernière lecture (UTC sans suffixe), réussie ou non.
    checked_at: Option<String>,
    /// Erreur de la dernière lecture (site injoignable, boucle de
    /// redirections…) ; `probe` reste alors la dernière lecture réussie.
    error: Option<String>,
    probe: Option<LastProbe>,
    platform_label: Option<&'static str>,
    /// Champs suivis, dans l'ordre d'affichage.
    fields: Vec<FieldView>,
    robots_checked_at: Option<String>,
    robots_blocks_all: Option<bool>,
    sitemap_ok: Option<bool>,
}

#[derive(Serialize)]
struct FieldView {
    field: &'static str,
    /// Valeur de référence, confirmée (chaîne vide : absente).
    value: Option<String>,
    /// Valeur différente vue une fois, en attente de la sonde suivante.
    pending: Option<String>,
    /// Date du dernier changement confirmé.
    changed_at: Option<String>,
    /// Changé depuis moins de 24 heures : la jauge `…_changed` vaut 1.
    recently_changed: bool,
}

/// `GET /api/targets/{id}/website`
async fn overview(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Overview>> {
    load(&state, id).await?;
    let Some(row) = store::state(&state.pool, id).await? else {
        return Ok(Json(Overview {
            checked_at: None,
            error: None,
            probe: None,
            platform_label: None,
            fields: Vec::new(),
            robots_checked_at: None,
            robots_blocks_all: None,
            sitemap_ok: None,
        }));
    };
    let tracker = row.tracker();
    let changed_at = row.changed_at();
    let ages = website::change_ages(&changed_at);
    let fields = changes::FIELDS
        .iter()
        .map(|field| FieldView {
            field,
            value: tracker.confirmed.get(*field).cloned(),
            pending: tracker.pending.get(*field).cloned(),
            changed_at: changed_at.get(*field).cloned(),
            recently_changed: ages
                .get(*field)
                .is_some_and(|age| *age < website::CHANGE_WINDOW_SECS),
        })
        .collect();
    let probe: Option<LastProbe> =
        row.last_probe.as_deref().and_then(|raw| serde_json::from_str(raw).ok());
    Ok(Json(Overview {
        checked_at: row.last_probe_at,
        error: row.last_error,
        platform_label: probe.as_ref().map(|p| p.platform.label()),
        probe,
        fields,
        robots_checked_at: row.robots_checked_at,
        robots_blocks_all: row.robots_blocks_all,
        sitemap_ok: row.sitemap_ok,
    }))
}

#[derive(Deserialize)]
struct HistoryQuery {
    #[serde(default)]
    limit: Option<i64>,
}

/// `GET /api/targets/{id}/website/changes?limit=…`
async fn history(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Query(query): Query<HistoryQuery>,
) -> ApiResult<Json<Vec<store::ChangeRow>>> {
    load(&state, id).await?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    Ok(Json(store::changes(&state.pool, id, limit).await?))
}

async fn load(state: &AppState, id: TargetId) -> ApiResult<()> {
    let not_found = || ApiError::NotFound(format!("Website {id} not found."));
    let target = db::targets::get(&state.pool, &state.cipher, id).await?.ok_or_else(not_found)?;
    if target.kind != website::KIND {
        return Err(not_found());
    }
    Ok(())
}
