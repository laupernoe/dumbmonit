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
//! - `GET /api/discovery/review` : combien d'appareils sont « à examiner », et si
//!   le dernier lot de nouveautés est à annoncer ; `POST …/review/seen` l'acquitte ;
//! - `POST /api/discovery/inventory/bulk` : ajouter, ignorer ou remettre à
//!   examiner plusieurs appareils d'un coup (200 au plus), un résultat par appareil.
//!
//! Tout est réservé aux administrateurs : c'est une carte du réseau.

use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use dumbmonit_proto::{ScanDepth, TargetId};
use serde::{Deserialize, Serialize};

use crate::api::targets::{self, TargetPayload};
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::auth::teams::{Access, TeamAccess};
use crate::netinventory::review::{self, ItemResult, MAX_BULK, Outcome};
use crate::netinventory::store::{self, Counts, DeviceView, Schedule, ScheduleInput};
use crate::netinventory::{MAX_INTERVAL_SECS, MAX_SCHEDULES, MIN_INTERVAL_SECS, canonical_cidr};
use crate::state::AppState;

const MAX_NAME_CHARS: usize = 80;

/// Un passage par jour par défaut : de quoi voir arriver un appareil sans charger le réseau.
fn default_interval() -> i64 {
    86_400
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

// ------------------------------------------------------------ à examiner

#[derive(Debug, Serialize)]
pub struct ReviewView {
    /// Appareils nouveaux, ni surveillés ni ignorés.
    pub to_review: usize,
    /// Lot de nouveautés pas encore vu : l'interface l'annonce une fois.
    pub announce: bool,
    /// Appareils nouveaux de ce lot.
    pub batch_count: i64,
    /// Numéro du lot, pour que l'interface n'annonce jamais deux fois le même.
    pub batch: i64,
}

async fn monitored(state: &AppState) -> ApiResult<HashSet<String>> {
    let targets = crate::db::targets::list(&state.pool, &state.cipher).await?;
    Ok(review::monitored_hosts(targets.iter().map(|t| t.address.as_str())))
}

/// `GET /api/discovery/review`
pub async fn review_state(
    _: AdminIdentity,
    State(state): State<AppState>,
) -> ApiResult<Json<ReviewView>> {
    let devices = store::inventory(&state.pool, None).await?;
    let to_review = review::to_review_count(&devices, &monitored(&state).await?);
    let batch = store::review_batch(&state.pool).await?;
    Ok(Json(ReviewView {
        to_review,
        // Rien à annoncer si la file est vide : tout a déjà été traité.
        announce: batch.unseen && to_review > 0,
        batch_count: batch.count,
        batch: batch.batch,
    }))
}

/// `POST /api/discovery/review/seen`
pub async fn review_seen(_: AdminIdentity, State(state): State<AppState>) -> ApiResult<StatusCode> {
    store::mark_batch_seen(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BulkAction {
    /// Créer un moniteur par appareil, avec le type que la détection propose.
    Add,
    /// Ignorer : l'appareil ne revient plus dans la file.
    Ignore,
    /// Défaire : remettre l'appareil dans la file « à examiner ».
    Restore,
}

#[derive(Debug, Deserialize)]
pub struct BulkPayload {
    pub action: BulkAction,
    pub device_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct BulkReport {
    pub results: Vec<ItemResult>,
    pub added: usize,
    pub pending_credentials: usize,
    pub already_present: usize,
    pub errors: usize,
}

impl BulkReport {
    fn new(results: Vec<ItemResult>) -> Self {
        let count = |o: Outcome| results.iter().filter(|r| r.outcome == o).count();
        Self {
            added: count(Outcome::Added),
            pending_credentials: count(Outcome::PendingCredentials),
            already_present: count(Outcome::AlreadyPresent),
            errors: count(Outcome::Error),
            results,
        }
    }
}

/// Crée le moniteur d'un appareil. Chaque appareil est sa propre transaction
/// (la création ordinaire d'une cible) : un échec n'annule pas les autres.
async fn add_one(
    state: &AppState,
    access: &Access,
    device: &DeviceView,
    schedules: &[Schedule],
    existing: &mut HashSet<String>,
) -> ItemResult {
    let id = device.id;
    if review::classify(device, existing) == review::Review::Monitored {
        return ItemResult::new(id, Outcome::AlreadyPresent);
    }
    let Some(proposal) = review::proposal(device) else {
        return ItemResult::new(id, Outcome::Error)
            .with_message("No monitor type was detected for this device: add it by hand.");
    };
    let pending = crate::api::collectors::needs_credential(&proposal.kind);
    let mut tags = proposal.tags;
    if pending {
        tags.insert(review::PENDING_TAG.to_string(), review::PENDING_CREDENTIALS.to_string());
    }
    // Un scan passé par un agent relais : le moniteur l'est aussi.
    let via_agent = schedules.iter().find(|s| s.id == device.schedule_id).and_then(|s| s.agent_id);
    let address = proposal.address.to_lowercase();
    let payload = TargetPayload {
        name: proposal.name,
        address: proposal.address,
        kind: proposal.kind,
        profile_id: proposal.profile_id,
        parent_id: None,
        via_agent: via_agent.map(Some),
        interval_secs: None,
        // Sans identifiants, un moniteur actif échouerait en boucle : en pause.
        enabled: if pending { Some(false) } else { None },
        tags,
        group_name: None,
        location_id: None,
        multisite: None,
        credential: None,
        client_id: None,
    };
    let created =
        targets::create(TeamAccess(access.clone()), State(state.clone()), Json(payload)).await;
    match created {
        Ok((_, Json(view))) => {
            existing.insert(address);
            if let Err(error) = store::mark_device_known(&state.pool, id).await {
                tracing::warn!(?error, device = id, "cannot mark an added device as known");
            }
            let outcome = if pending { Outcome::PendingCredentials } else { Outcome::Added };
            ItemResult::new(id, outcome).with_target(view.id)
        }
        Err(ApiError::Conflict(message)) => {
            // Même type et même adresse : déjà là, ce n'est pas une erreur.
            if let Err(error) = store::mark_device_known(&state.pool, id).await {
                tracing::warn!(?error, device = id, "cannot mark a present device as known");
            }
            ItemResult::new(id, Outcome::AlreadyPresent).with_message(message)
        }
        Err(error) => ItemResult::new(id, Outcome::Error).with_message(error.into_parts().1),
    }
}

/// `POST /api/discovery/inventory/bulk`
pub async fn bulk(
    _: AdminIdentity,
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Json(payload): Json<BulkPayload>,
) -> ApiResult<Json<BulkReport>> {
    if payload.device_ids.is_empty() {
        return Err(ApiError::BadRequest("Select at least one device.".to_string()));
    }
    if payload.device_ids.len() > MAX_BULK {
        return Err(ApiError::BadRequest(format!(
            "At most {MAX_BULK} devices can be handled at once."
        )));
    }
    let inventory = store::inventory(&state.pool, None).await?;
    let schedules = store::list_schedules(&state.pool).await?;
    let mut existing = monitored(&state).await?;
    let mut seen = HashSet::new();
    let mut results = Vec::with_capacity(payload.device_ids.len());
    for id in payload.device_ids {
        // Un identifiant répété n'est traité qu'une fois.
        if !seen.insert(id) {
            continue;
        }
        let Some(device) = inventory.iter().find(|d| d.id == id) else {
            results.push(ItemResult::new(id, Outcome::Error).with_message("Unknown device."));
            continue;
        };
        let result = match payload.action {
            BulkAction::Add => add_one(&state, &access, device, &schedules, &mut existing).await,
            BulkAction::Ignore => match store::mark_device_ignored(&state.pool, id).await? {
                true => ItemResult::new(id, Outcome::Ignored),
                false => ItemResult::new(id, Outcome::Error).with_message("Unknown device."),
            },
            BulkAction::Restore => match store::mark_device_review(&state.pool, id).await? {
                true => ItemResult::new(id, Outcome::Restored),
                false => ItemResult::new(id, Outcome::Error).with_message("Unknown device."),
            },
        };
        results.push(result);
    }
    Ok(Json(BulkReport::new(results)))
}
