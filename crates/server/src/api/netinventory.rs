//! Scans de découverte planifiés et inventaire du réseau.
//!
//! - `GET|POST /api/discovery/schedules`, `PUT|DELETE /api/discovery/schedules/{id}` :
//!   les scans planifiés ;
//! - `POST /api/discovery/schedules/{id}/run` : en lancer un sans attendre ;
//! - `GET /api/discovery/inventory` : appareils et ports connus, inconnus d'abord ;
//! - `POST /api/discovery/inventory/devices/{id}/known`,
//!   `/inventory/ports/{id}/known`, `/inventory/known` : marquer connu ;
//! - `POST /api/discovery/inventory/devices/{id}/ignore` : ignorer un appareil
//!   (plus aucune alerte) ; `…/known` le remet dans l'inventaire.
//!
//! Tout est réservé aux administrateurs : c'est une carte du réseau.

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use dumbmonit_proto::{ScanDepth, TargetId};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::netinventory::store::{self, Counts, DeviceView, Schedule, ScheduleInput};
use crate::netinventory::{MAX_INTERVAL_SECS, MAX_SCHEDULES, MIN_INTERVAL_SECS, canonical_cidr};
use crate::state::AppState;

const MAX_NAME_CHARS: usize = 80;

fn default_interval() -> i64 {
    3600
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SchedulePayload {
    #[serde(default)]
    pub name: String,
    /// Sous-réseau en notation CIDR (1024 adresses au plus).
    pub cidr: String,
    /// Agent relais qui balaie ; absent : le serveur.
    #[serde(default)]
    pub agent_id: Option<TargetId>,
    #[serde(default = "default_interval")]
    pub interval_secs: i64,
    /// Liste de ports de la découverte : `standard` ou `extended`.
    #[serde(default)]
    pub depth: ScanDepth,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct ScheduleView {
    pub id: i64,
    pub name: String,
    pub cidr: String,
    pub agent_id: Option<TargetId>,
    pub agent_name: Option<String>,
    pub interval_secs: i64,
    pub depth: ScanDepth,
    pub enabled: bool,
    /// Le premier balayage a initialisé l'inventaire.
    pub initialized: bool,
    pub running: bool,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub devices: i64,
    pub unknown_devices: i64,
    pub ignored_devices: i64,
    /// Changements de MAC pour une IP, en attente d'être marqués connus ou ignorés.
    pub mac_changes: i64,
    pub unknown_ports: i64,
}

#[derive(Debug, Serialize)]
pub struct ScheduleList {
    pub schedules: Vec<ScheduleView>,
}

#[derive(Debug, Serialize)]
pub struct InventoryView {
    pub devices: Vec<DeviceView>,
}

#[derive(Debug, Serialize)]
pub struct Marked {
    pub marked: u64,
}

#[derive(Debug, Deserialize)]
pub struct InventoryQuery {
    #[serde(default)]
    pub schedule_id: Option<i64>,
}

fn view(schedule: Schedule, counts: Counts, agents: &HashMap<TargetId, String>) -> ScheduleView {
    ScheduleView {
        agent_name: schedule.agent_id.and_then(|id| agents.get(&id).cloned()),
        running: schedule.last_status.as_deref() == Some("running"),
        id: schedule.id,
        name: schedule.name,
        cidr: schedule.cidr,
        agent_id: schedule.agent_id,
        interval_secs: schedule.interval_secs,
        depth: schedule.depth,
        enabled: schedule.enabled,
        initialized: schedule.initialized,
        last_run_at: schedule.last_run_at,
        last_status: schedule.last_status,
        last_error: schedule.last_error,
        devices: counts.devices,
        unknown_devices: counts.unknown_devices,
        ignored_devices: counts.ignored_devices,
        mac_changes: counts.mac_changes,
        unknown_ports: counts.unknown_ports,
    }
}

async fn agent_names(state: &AppState) -> ApiResult<HashMap<TargetId, String>> {
    let hosts = crate::collectors::agent::list_hosts(&state.pool).await?;
    Ok(hosts.into_iter().map(|(id, name, _)| (id, name)).collect())
}

/// Valide une saisie et la ramène à sa forme stockée.
async fn validate(state: &AppState, payload: SchedulePayload) -> ApiResult<ScheduleInput> {
    let name = payload.name.trim().to_string();
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(ApiError::BadRequest(format!(
            "The name must be at most {MAX_NAME_CHARS} characters."
        )));
    }
    let cidr = canonical_cidr(&payload.cidr)?;
    if !(MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&payload.interval_secs) {
        return Err(ApiError::BadRequest(format!(
            "The interval must be between {} minutes and {} days.",
            MIN_INTERVAL_SECS / 60,
            MAX_INTERVAL_SECS / 86_400
        )));
    }
    if let Some(agent_id) = payload.agent_id
        && !agent_names(state).await?.contains_key(&agent_id)
    {
        return Err(ApiError::NotFound("Unknown relay agent.".to_string()));
    }
    Ok(ScheduleInput {
        name,
        cidr,
        agent_id: payload.agent_id,
        interval_secs: payload.interval_secs,
        depth: payload.depth,
        enabled: payload.enabled,
    })
}

fn conflict(error: sqlx::Error) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::Conflict("This network is already scheduled for this scanner.".to_string())
    } else {
        ApiError::Internal(error.into())
    }
}

async fn one(state: &AppState, id: i64) -> ApiResult<ScheduleView> {
    let schedule = store::get_schedule(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Unknown scheduled scan.".to_string()))?;
    let counts = store::counts(&state.pool).await?.remove(&id).unwrap_or_default();
    Ok(view(schedule, counts, &agent_names(state).await?))
}

/// `GET /api/discovery/schedules`
pub async fn list(
    _: AdminIdentity,
    State(state): State<AppState>,
) -> ApiResult<Json<ScheduleList>> {
    let mut counts = store::counts(&state.pool).await?;
    let agents = agent_names(&state).await?;
    let schedules = store::list_schedules(&state.pool)
        .await?
        .into_iter()
        .map(|s| {
            let counts = counts.remove(&s.id).unwrap_or_default();
            view(s, counts, &agents)
        })
        .collect();
    Ok(Json(ScheduleList { schedules }))
}

/// `POST /api/discovery/schedules`
pub async fn create(
    _: AdminIdentity,
    State(state): State<AppState>,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<(StatusCode, Json<ScheduleView>)> {
    let input = validate(&state, payload).await?;
    if store::count_schedules(&state.pool).await? >= MAX_SCHEDULES {
        return Err(ApiError::BadRequest(format!("At most {MAX_SCHEDULES} scheduled scans.")));
    }
    let id = store::insert_schedule(&state.pool, &input).await.map_err(conflict)?;
    Ok((StatusCode::CREATED, Json(one(&state, id).await?)))
}

/// `PUT /api/discovery/schedules/{id}`
pub async fn update(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<Json<ScheduleView>> {
    let input = validate(&state, payload).await?;
    if !store::update_schedule(&state.pool, id, &input).await.map_err(conflict)? {
        return Err(ApiError::NotFound("Unknown scheduled scan.".to_string()));
    }
    Ok(Json(one(&state, id).await?))
}

/// `DELETE /api/discovery/schedules/{id}` — retire aussi son inventaire.
pub async fn remove(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::delete_schedule(&state.pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("Unknown scheduled scan.".to_string()))
    }
}

/// `POST /api/discovery/schedules/{id}/run` — balaie au prochain tour de la
/// boucle (quelques secondes), sans attendre l'intervalle.
pub async fn run_now(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::request_run(&state.pool, id).await? {
        Ok(StatusCode::ACCEPTED)
    } else {
        Err(ApiError::NotFound("Unknown scheduled scan.".to_string()))
    }
}

/// `GET /api/discovery/inventory`
pub async fn inventory(
    _: AdminIdentity,
    State(state): State<AppState>,
    Query(query): Query<InventoryQuery>,
) -> ApiResult<Json<InventoryView>> {
    Ok(Json(InventoryView { devices: store::inventory(&state.pool, query.schedule_id).await? }))
}

/// `POST /api/discovery/inventory/devices/{id}/known`
pub async fn device_known(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::mark_device_known(&state.pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("Unknown device.".to_string()))
    }
}

/// `POST /api/discovery/inventory/devices/{id}/ignore`
pub async fn device_ignore(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::mark_device_ignored(&state.pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("Unknown device.".to_string()))
    }
}

/// `POST /api/discovery/inventory/ports/{id}/known`
pub async fn port_known(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::mark_port_known(&state.pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("Unknown port.".to_string()))
    }
}

/// `POST /api/discovery/inventory/known` — tout marquer connu (`?schedule_id=`
/// pour un seul scan).
pub async fn all_known(
    _: AdminIdentity,
    State(state): State<AppState>,
    Query(query): Query<InventoryQuery>,
) -> ApiResult<Json<Marked>> {
    let marked = store::mark_all_known(&state.pool, query.schedule_id).await?;
    Ok(Json(Marked { marked }))
}
