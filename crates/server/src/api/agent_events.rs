//! Règles du journal d'événements Windows d'une machine, et les derniers
//! événements reçus (voir `collectors::agent::eventlog`).
//!
//! Les règles partent vers l'agent dans l'accusé de réception de son lot
//! suivant : une modification s'applique en une période d'échantillonnage,
//! sans toucher à la machine.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use dumbmonit_proto::{EVENT_RULE_MAX_PER_MINUTE, TargetId};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::collectors::agent::eventlog::{
    self, EventRule, EventRuleInput, EventRulePreset, EventView, SaveError,
};
use crate::collectors::agent::{self as agent};
use crate::db;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/agent/event-rules", get(list).post(create))
        .route("/targets/{id}/agent/event-rules/{rule_id}", put(update).delete(remove))
        .route("/targets/{id}/agent/events", get(events))
}

/// Ce que la section « Event log » de la page d'une machine affiche.
#[derive(Debug, Serialize)]
pub struct EventRulesView {
    /// Vrai pour une machine sous Windows : ailleurs, les règles seraient
    /// gardées sans jamais partir vers l'agent.
    pub supported: bool,
    /// Événements retenus par règle et par minute, au-delà comptés comme
    /// écartés (`dropped_total`).
    pub max_per_minute: u32,
    pub rules: Vec<EventRule>,
    /// Exemples proposés à l'ajout, jamais créés d'office.
    pub presets: &'static [EventRulePreset],
}

/// La machine, si elle existe et porte un agent.
async fn host(state: &AppState, id: TargetId) -> ApiResult<dumbmonit_proto::Target> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    if target.kind != "agent" {
        return Err(ApiError::NotFound(format!("Monitor {id} has no agent.")));
    }
    Ok(target)
}

fn save_error(error: SaveError) -> ApiError {
    match error {
        SaveError::Invalid(why) => ApiError::BadRequest(why),
        SaveError::TooMany => ApiError::Conflict(format!(
            "A machine has at most {} event log rules.",
            eventlog::MAX_RULES_PER_HOST
        )),
        SaveError::NotFound => ApiError::NotFound("Event log rule not found.".to_string()),
        SaveError::Internal(error) => ApiError::Internal(error),
    }
}

/// `GET /api/targets/{id}/agent/event-rules`
async fn list(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<EventRulesView>> {
    host(&state, id).await?;
    let supported = agent::host(&state.pool, id)
        .await?
        .is_some_and(|info| info.os.eq_ignore_ascii_case("windows"));
    Ok(Json(EventRulesView {
        supported,
        max_per_minute: EVENT_RULE_MAX_PER_MINUTE,
        rules: eventlog::list(&state.pool, id).await?,
        presets: eventlog::PRESETS,
    }))
}

/// `POST /api/targets/{id}/agent/event-rules`
async fn create(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Json(input): Json<EventRuleInput>,
) -> ApiResult<(StatusCode, Json<EventRule>)> {
    host(&state, id).await?;
    let rule = eventlog::create(&state.pool, id, &input).await.map_err(save_error)?;
    tracing::info!(cible = id, regle = rule.id, nom = %rule.name, "règle d'événements créée");
    Ok((StatusCode::CREATED, Json(rule)))
}

/// `PUT /api/targets/{id}/agent/event-rules/{rule_id}`
async fn update(
    State(state): State<AppState>,
    Path((id, rule_id)): Path<(TargetId, i64)>,
    Json(input): Json<EventRuleInput>,
) -> ApiResult<Json<EventRule>> {
    host(&state, id).await?;
    let rule = eventlog::update(&state.pool, id, rule_id, &input).await.map_err(save_error)?;
    tracing::info!(cible = id, regle = rule.id, "règle d'événements modifiée");
    Ok(Json(rule))
}

/// `DELETE /api/targets/{id}/agent/event-rules/{rule_id}`
async fn remove(
    State(state): State<AppState>,
    Path((id, rule_id)): Path<(TargetId, i64)>,
) -> ApiResult<StatusCode> {
    host(&state, id).await?;
    if !eventlog::delete(&state.pool, id, rule_id).await? {
        return Err(ApiError::NotFound("Event log rule not found.".to_string()));
    }
    tracing::info!(cible = id, regle = rule_id, "règle d'événements supprimée");
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/targets/{id}/agent/events` : les derniers événements reçus, du
/// plus récent au plus ancien (50 par défaut).
async fn events(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Query(query): Query<EventsQuery>,
) -> ApiResult<Json<Vec<EventView>>> {
    host(&state, id).await?;
    Ok(Json(eventlog::recent(&state.pool, id, query.limit.unwrap_or(50)).await?))
}
