//! Routes des astreintes : plannings, remplacements, politiques d'escalade,
//! journal des escalades, et, pour chacun, ses contacts, ses heures calmes et
//! son lien de calendrier.
//!
//! Les lectures sont ouvertes à tous les comptes ; écrire un planning ou une
//! politique est réservé à l'administrateur (garde de `require_session`), un
//! remplacement est ouvert aussi aux opérateurs (`OPERATOR_ROUTES`), et tout ce
//! qui vit sous `/account` est fait par chacun pour lui-même, en session.

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::alerting::model::Severity;
use crate::alerting::silence::Schedule;
use crate::api::channels::ChannelPayload;
use crate::api::{ApiError, ApiResult};
use crate::auth::clients;
use crate::auth::middleware::{Authenticated, OperatorIdentity};
use crate::auth::teams::{Access, TeamAccess};
use crate::auth::users::{self, User};
use crate::db;
use crate::notify::{self, ChannelConfig};
use crate::oncall::escalation::{PolicyDef, Step, Target};
use crate::oncall::runner::{self, Ctx, PERSONAL_KINDS, recipient_key};
use crate::oncall::schedule::{Definition, Layer, Shift, parse_tz};
use crate::oncall::store::{self, ContactRow, OverrideRow};
use crate::oncall::{CHANNEL_KIND, ical};
use crate::state::AppState;

/// Période maximale d'une lecture de gardes : trois mois, de quoi afficher un
/// mois entier avec ses débords.
const MAX_SHIFT_RANGE_DAYS: i64 = 92;
/// Durée maximale d'un remplacement.
const MAX_OVERRIDE_DAYS: i64 = 366;
/// Contacts au plus par compte.
const MAX_CONTACTS: usize = 20;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/oncall/people", get(people))
        .route("/oncall/now", get(now_on_call))
        .route("/oncall/schedules", get(list_schedules).post(create_schedule))
        .route("/oncall/schedules/{id}", put(update_schedule).delete(delete_schedule))
        .route("/oncall/schedules/{id}/shifts", get(shifts))
        .route("/oncall/schedules/{id}/overrides", get(list_overrides).post(create_override))
        .route("/oncall/schedules/{id}/overrides/{oid}", delete(delete_override))
        .route("/oncall/policies", get(list_policies).post(create_policy))
        .route("/oncall/policies/{id}", put(update_policy).delete(delete_policy))
        .route("/oncall/escalations", get(escalations))
        .route("/account/contacts/options", get(contact_options))
        .route("/account/contacts", get(list_contacts).post(create_contact))
        .route("/account/contacts/{id}", put(update_contact).delete(delete_contact))
        .route("/account/contacts/{id}/test", post(test_contact))
        .route("/account/oncall", get(profile).put(update_profile))
        .route("/account/oncall/calendar", post(create_calendar).delete(revoke_calendar))
}

/// Le lien de calendrier : le jeton est l'autorisation, comme une URL secrète
/// de calendrier partout ailleurs.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/public/oncall/calendar/{token}", get(calendar))
}

fn to_wire(at: DateTime<Utc>) -> String {
    store::to_sql(at)
}

// --------------------------------------------------------------------------
// Personnes
// --------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct Person {
    pub id: i64,
    pub display_name: String,
    pub disabled: bool,
    /// Cette personne est d'astreinte à cet instant, sur au moins un planning.
    pub on_call: bool,
    /// Réservés aux administrateurs et à la personne elle-même.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<&'static str>,
    /// Moyens de contact actifs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<usize>,
}

/// `GET /api/oncall/people`
///
/// Tous les rôles voient le nom et l'état d'astreinte (de quoi composer un
/// planning) ; l'identifiant de connexion, le rôle et les moyens de contact ne
/// sont rendus qu'aux administrateurs et à la personne concernée.
pub async fn people(
    State(state): State<AppState>,
    Authenticated(caller): Authenticated,
) -> ApiResult<Json<Vec<Person>>> {
    let accounts = users::list(&state.pool).await?;
    let now = Utc::now();
    let schedules = runner::load_schedules(&state.pool, now).await?;
    let on_call: HashSet<i64> = entries(&schedules, now).iter().filter_map(|e| e.user_id).collect();
    let contacts = if caller.role.is_admin() {
        store::list_contacts(&state.pool, &state.cipher, None).await?
    } else {
        store::list_contacts(&state.pool, &state.cipher, Some(caller.id)).await?
    };
    Ok(Json(
        accounts
            .into_iter()
            .map(|user| {
                let full = caller.role.is_admin() || caller.id == user.id;
                Person {
                    id: user.id,
                    display_name: user.display_name,
                    disabled: user.disabled,
                    on_call: on_call.contains(&user.id),
                    contacts: full.then(|| {
                        contacts.iter().filter(|c| c.user_id == user.id && c.enabled).count()
                    }),
                    role: full.then(|| user.role.as_str()),
                    username: full.then_some(user.username),
                }
            })
            .collect(),
    ))
}

async fn known_users(state: &AppState) -> ApiResult<HashSet<i64>> {
    Ok(users::list(&state.pool).await?.into_iter().map(|user| user.id).collect())
}

// --------------------------------------------------------------------------
// Plannings
// --------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct NowView {
    pub user_id: i64,
    pub since: Option<String>,
    pub until: Option<String>,
    pub layer: Option<usize>,
    pub override_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleView {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub timezone: String,
    pub handoff_notifications: bool,
    pub layers: Vec<Layer>,
    pub now: Option<NowView>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SchedulePayload {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub timezone: String,
    #[serde(default)]
    pub handoff_notifications: bool,
    #[serde(default)]
    pub layers: Vec<Layer>,
}

fn now_view(
    definition: &Definition,
    overrides: &[crate::oncall::schedule::Override],
    now: DateTime<Utc>,
) -> Option<NowView> {
    definition.current(overrides, now).map(|(slot, since, until)| NowView {
        user_id: slot.user_id,
        since: since.map(to_wire),
        until: until.map(to_wire),
        layer: slot.layer,
        override_id: slot.override_id,
    })
}

fn schedule_view(
    row: store::ScheduleRow,
    overrides: &[crate::oncall::schedule::Override],
    now: DateTime<Utc>,
) -> ScheduleView {
    let current = now_view(&row.definition, overrides, now);
    ScheduleView {
        id: row.id,
        name: row.name,
        description: row.description,
        timezone: row.definition.timezone,
        handoff_notifications: row.definition.handoff_notifications,
        layers: row.definition.layers,
        now: current,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

/// `GET /api/oncall/schedules`
pub async fn list_schedules(
    State(state): State<AppState>,
    TeamAccess(access): TeamAccess,
) -> ApiResult<Json<Vec<ScheduleView>>> {
    let now = Utc::now();
    let schedules = visible_schedules(&state, &access, now).await?;
    Ok(Json(
        schedules.into_iter().map(|(row, overrides)| schedule_view(row, &overrides, now)).collect(),
    ))
}

impl SchedulePayload {
    async fn validate(self, state: &AppState) -> ApiResult<(String, String, Definition)> {
        let name = self.name.trim().to_string();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(ApiError::BadRequest(
                "The schedule needs a name (80 characters at most).".into(),
            ));
        }
        let definition = Definition {
            timezone: self.timezone.trim().to_string(),
            handoff_notifications: self.handoff_notifications,
            layers: self
                .layers
                .into_iter()
                .map(|mut layer| {
                    layer.name = layer.name.trim().to_string();
                    layer
                })
                .collect(),
        };
        definition.validate().map_err(ApiError::BadRequest)?;
        let known = known_users(state).await?;
        if let Some(unknown) =
            definition.layers.iter().flat_map(|l| &l.participants).find(|id| !known.contains(id))
        {
            return Err(ApiError::BadRequest(format!("Account {unknown} does not exist.")));
        }
        Ok((name, self.description.trim().to_string(), definition))
    }
}

fn duplicate_name(error: anyhow::Error, what: &str) -> ApiError {
    if format!("{error:#}").contains("UNIQUE constraint failed") {
        ApiError::Conflict(format!("A {what} with this name already exists."))
    } else {
        ApiError::Internal(error)
    }
}

async fn one_schedule(state: &AppState, id: i64) -> ApiResult<ScheduleView> {
    let now = Utc::now();
    let row = store::get_schedule(&state.pool, id).await?.ok_or_else(|| schedule_not_found(id))?;
    let overrides: Vec<_> =
        store::list_overrides(&state.pool, Some(id), Some(now - TimeDelta::days(1)))
            .await?
            .iter()
            .map(OverrideRow::pure)
            .collect();
    Ok(schedule_view(row, &overrides, now))
}

fn schedule_not_found(id: i64) -> ApiError {
    ApiError::NotFound(format!("On-call schedule {id} not found."))
}

/// `POST /api/oncall/schedules`
pub async fn create_schedule(
    State(state): State<AppState>,
    TeamAccess(access): TeamAccess,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<(StatusCode, Json<ScheduleView>)> {
    let (name, description, definition) = payload.validate(&state).await?;
    let id = store::save_schedule(&state.pool, None, &name, &description, &definition)
        .await
        .map_err(|e| duplicate_name(e, "schedule"))?;
    clients::adopt(&state.pool, &access, clients::Kind::Schedule, id).await?;
    Ok((StatusCode::CREATED, Json(one_schedule(&state, id).await?)))
}

/// `PUT /api/oncall/schedules/{id}`
pub async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<Json<ScheduleView>> {
    store::get_schedule(&state.pool, id).await?.ok_or_else(|| schedule_not_found(id))?;
    let (name, description, definition) = payload.validate(&state).await?;
    store::save_schedule(&state.pool, Some(id), &name, &description, &definition)
        .await
        .map_err(|e| duplicate_name(e, "schedule"))?;
    Ok(Json(one_schedule(&state, id).await?))
}

/// `DELETE /api/oncall/schedules/{id}` — refusé tant qu'une politique l'appelle.
pub async fn delete_schedule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let users: Vec<String> = store::list_policies(&state.pool)
        .await?
        .into_iter()
        .filter(|policy| {
            policy
                .definition
                .steps
                .iter()
                .flat_map(|s| &s.targets)
                .any(|t| *t == Target::Schedule { id })
        })
        .map(|policy| policy.name)
        .collect();
    if !users.is_empty() {
        return Err(ApiError::Conflict(format!(
            "This schedule is still called by {}. Remove it from those escalation policies first.",
            users.join(", ")
        )));
    }
    if store::delete_schedule(&state.pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(schedule_not_found(id))
    }
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ShiftView {
    pub start: String,
    pub end: String,
    pub user_id: i64,
    pub layer: Option<usize>,
    pub override_id: Option<i64>,
}

impl From<Shift> for ShiftView {
    fn from(shift: Shift) -> Self {
        Self {
            start: to_wire(shift.start),
            end: to_wire(shift.end),
            user_id: shift.user_id,
            layer: shift.layer,
            override_id: shift.override_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ShiftsView {
    #[serde(rename = "final")]
    pub final_shifts: Vec<ShiftView>,
    pub layers: Vec<Vec<ShiftView>>,
    pub overrides: Vec<OverrideRow>,
}

/// `GET /api/oncall/schedules/{id}/shifts?from=&to=`
pub async fn shifts(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(range): Query<RangeQuery>,
) -> ApiResult<Json<ShiftsView>> {
    let now = Utc::now();
    let from = range.from.unwrap_or(now - TimeDelta::days(1));
    let to = range.to.unwrap_or(from + TimeDelta::days(7));
    if to <= from || to - from > TimeDelta::days(MAX_SHIFT_RANGE_DAYS) {
        return Err(ApiError::BadRequest(format!(
            "\"to\" must come after \"from\", at most {MAX_SHIFT_RANGE_DAYS} days later."
        )));
    }
    let row = store::get_schedule(&state.pool, id).await?.ok_or_else(|| schedule_not_found(id))?;
    let overrides: Vec<OverrideRow> = store::list_overrides(&state.pool, Some(id), Some(from))
        .await?
        .into_iter()
        .filter(|o| o.starts_at < to)
        .collect();
    let pure: Vec<_> = overrides.iter().map(OverrideRow::pure).collect();
    let final_shifts =
        row.definition.shifts(&pure, from, to).into_iter().map(ShiftView::from).collect();
    let layers = (0..row.definition.layers.len())
        .map(|index| {
            row.definition.layer_shifts(index, from, to).into_iter().map(ShiftView::from).collect()
        })
        .collect();
    Ok(Json(ShiftsView { final_shifts, layers, overrides }))
}

// --------------------------------------------------------------------------
// Remplacements
// --------------------------------------------------------------------------

/// `GET /api/oncall/schedules/{id}/overrides` — ceux qui ne sont pas finis
/// depuis plus de trente jours.
pub async fn list_overrides(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Vec<OverrideRow>>> {
    store::get_schedule(&state.pool, id).await?.ok_or_else(|| schedule_not_found(id))?;
    let now = Utc::now();
    let mut rows =
        store::list_overrides(&state.pool, Some(id), Some(now - TimeDelta::days(30))).await?;
    // À venir et en cours d'abord, puis le passé récent, le plus proche en tête.
    rows.sort_by_key(|o| {
        (
            o.ends_at <= now,
            if o.ends_at > now { o.starts_at.timestamp() } else { -o.ends_at.timestamp() },
        )
    });
    rows.truncate(200);
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct OverridePayload {
    pub user_id: i64,
    #[serde(default)]
    pub replaces: Option<i64>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    #[serde(default)]
    pub note: String,
}

/// `POST /api/oncall/schedules/{id}/overrides` — administrateurs et opérateurs.
pub async fn create_override(
    State(state): State<AppState>,
    OperatorIdentity(principal): OperatorIdentity,
    Path(id): Path<i64>,
    Json(payload): Json<OverridePayload>,
) -> ApiResult<(StatusCode, Json<OverrideRow>)> {
    store::get_schedule(&state.pool, id).await?.ok_or_else(|| schedule_not_found(id))?;
    if payload.ends_at <= payload.starts_at {
        return Err(ApiError::BadRequest("The replacement must end after it starts.".into()));
    }
    if payload.ends_at - payload.starts_at > TimeDelta::days(MAX_OVERRIDE_DAYS) {
        return Err(ApiError::BadRequest("A replacement lasts one year at most.".into()));
    }
    if payload.replaces == Some(payload.user_id) {
        return Err(ApiError::BadRequest("Someone cannot replace themself.".into()));
    }
    let known = known_users(&state).await?;
    for user in std::iter::once(payload.user_id).chain(payload.replaces) {
        if !known.contains(&user) {
            return Err(ApiError::BadRequest(format!("Account {user} does not exist.")));
        }
    }
    let note: String = payload.note.trim().chars().take(200).collect();
    let created_by = principal.label();
    let oid = store::insert_override(
        &state.pool,
        &store::NewOverride {
            schedule_id: id,
            user_id: payload.user_id,
            replaces: payload.replaces,
            starts_at: payload.starts_at,
            ends_at: payload.ends_at,
            note: &note,
            created_by: &created_by,
        },
    )
    .await?;
    let row = store::list_overrides(&state.pool, Some(id), None)
        .await?
        .into_iter()
        .find(|o| o.id == oid)
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("replacement {oid} vanished")))?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// `DELETE /api/oncall/schedules/{id}/overrides/{oid}`
pub async fn delete_override(
    State(state): State<AppState>,
    _: OperatorIdentity,
    Path((id, oid)): Path<(i64, i64)>,
) -> ApiResult<StatusCode> {
    if store::delete_override(&state.pool, id, oid).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Replacement {oid} not found in schedule {id}.")))
    }
}

#[derive(Debug, Serialize)]
pub struct OnCallEntry {
    pub schedule_id: i64,
    pub schedule_name: String,
    pub user_id: Option<i64>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub override_id: Option<i64>,
}

fn entries(
    schedules: &[(store::ScheduleRow, Vec<crate::oncall::schedule::Override>)],
    now: DateTime<Utc>,
) -> Vec<OnCallEntry> {
    schedules
        .iter()
        .map(|(row, overrides)| {
            let current = now_view(&row.definition, overrides, now);
            OnCallEntry {
                schedule_id: row.id,
                schedule_name: row.name.clone(),
                user_id: current.as_ref().map(|c| c.user_id),
                since: current.as_ref().and_then(|c| c.since.clone()),
                until: current.as_ref().and_then(|c| c.until.clone()),
                override_id: current.as_ref().and_then(|c| c.override_id),
            }
        })
        .collect()
}

/// `GET /api/oncall/now`
pub async fn now_on_call(
    State(state): State<AppState>,
    TeamAccess(access): TeamAccess,
) -> ApiResult<Json<Vec<OnCallEntry>>> {
    let now = Utc::now();
    let schedules = visible_schedules(&state, &access, now).await?;
    Ok(Json(entries(&schedules, now)))
}

/// Mode MSP : les plannings que ces droits laissent voir (ceux du client de la
/// vue et ceux de l'instance ; tous sans restriction de client).
async fn visible_schedules(
    state: &AppState,
    access: &Access,
    now: DateTime<Utc>,
) -> ApiResult<Vec<(store::ScheduleRow, Vec<crate::oncall::schedule::Override>)>> {
    let mut schedules = runner::load_schedules(&state.pool, now).await?;
    clients::retain_visible(
        &state.pool,
        access,
        clients::Kind::Schedule,
        &mut schedules,
        |(row, _)| row.id,
        true,
    )
    .await?;
    Ok(schedules)
}

// --------------------------------------------------------------------------
// Politiques d'escalade
// --------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PolicyView {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub repeat: u32,
    pub steps: Vec<Step>,
    /// Canaux d'astreinte qui déclenchent cette politique.
    pub channels: Vec<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PolicyPayload {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub repeat: u32,
    #[serde(default)]
    pub steps: Vec<Step>,
}

async fn policy_views(state: &AppState) -> ApiResult<Vec<PolicyView>> {
    let channels = store::oncall_channels(&state.pool).await?;
    Ok(store::list_policies(&state.pool)
        .await?
        .into_iter()
        .map(|row| PolicyView {
            channels: channels
                .iter()
                .filter(|(_, _, p)| *p == Some(row.id))
                .map(|(id, _, _)| *id)
                .collect(),
            id: row.id,
            name: row.name,
            description: row.description,
            repeat: row.definition.repeat,
            steps: row.definition.steps,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

async fn one_policy(state: &AppState, id: i64) -> ApiResult<PolicyView> {
    policy_views(state).await?.into_iter().find(|p| p.id == id).ok_or_else(|| policy_not_found(id))
}

fn policy_not_found(id: i64) -> ApiError {
    ApiError::NotFound(format!("Escalation policy {id} not found."))
}

impl PolicyPayload {
    async fn validate(self, state: &AppState) -> ApiResult<(String, String, PolicyDef)> {
        let name = self.name.trim().to_string();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(ApiError::BadRequest(
                "The policy needs a name (80 characters at most).".into(),
            ));
        }
        let definition = PolicyDef { repeat: self.repeat, steps: self.steps };
        definition.validate().map_err(ApiError::BadRequest)?;
        let users = known_users(state).await?;
        let schedules: HashSet<i64> =
            store::list_schedules(&state.pool).await?.into_iter().map(|s| s.id).collect();
        let channels: HashMap<i64, String> =
            sqlx::query_as::<_, (i64, String)>("SELECT id, kind FROM notification_channels")
                .fetch_all(&state.pool)
                .await?
                .into_iter()
                .collect();
        for (index, step) in definition.steps.iter().enumerate() {
            for target in &step.targets {
                let problem = match *target {
                    Target::Schedule { id } => {
                        (!schedules.contains(&id)).then(|| format!("schedule {id} does not exist"))
                    }
                    Target::User { id } => {
                        (!users.contains(&id)).then(|| format!("account {id} does not exist"))
                    }
                    Target::Channel { id } => match channels.get(&id) {
                        None => Some(format!("channel {id} does not exist")),
                        Some(kind) if kind == CHANNEL_KIND => {
                            Some("an on-call channel cannot be a step target".to_string())
                        }
                        Some(_) => None,
                    },
                };
                if let Some(problem) = problem {
                    return Err(ApiError::BadRequest(format!("Step {}: {problem}.", index + 1)));
                }
            }
        }
        Ok((name, self.description.trim().to_string(), definition))
    }
}

/// `GET /api/oncall/policies`
pub async fn list_policies(
    State(state): State<AppState>,
    TeamAccess(access): TeamAccess,
) -> ApiResult<Json<Vec<PolicyView>>> {
    let mut views = policy_views(&state).await?;
    clients::retain_visible(
        &state.pool,
        &access,
        clients::Kind::EscalationPolicy,
        &mut views,
        |view| view.id,
        true,
    )
    .await?;
    Ok(Json(views))
}

/// `POST /api/oncall/policies`
pub async fn create_policy(
    State(state): State<AppState>,
    TeamAccess(access): TeamAccess,
    Json(payload): Json<PolicyPayload>,
) -> ApiResult<(StatusCode, Json<PolicyView>)> {
    let (name, description, definition) = payload.validate(&state).await?;
    let id = store::save_policy(&state.pool, None, &name, &description, &definition)
        .await
        .map_err(|e| duplicate_name(e, "policy"))?;
    clients::adopt(&state.pool, &access, clients::Kind::EscalationPolicy, id).await?;
    Ok((StatusCode::CREATED, Json(one_policy(&state, id).await?)))
}

/// `PUT /api/oncall/policies/{id}`
pub async fn update_policy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<PolicyPayload>,
) -> ApiResult<Json<PolicyView>> {
    store::get_policy(&state.pool, id).await?.ok_or_else(|| policy_not_found(id))?;
    let (name, description, definition) = payload.validate(&state).await?;
    store::save_policy(&state.pool, Some(id), &name, &description, &definition)
        .await
        .map_err(|e| duplicate_name(e, "policy"))?;
    Ok(Json(one_policy(&state, id).await?))
}

/// `DELETE /api/oncall/policies/{id}` — refusé tant qu'un canal la déclenche.
pub async fn delete_policy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let users: Vec<String> = store::oncall_channels(&state.pool)
        .await?
        .into_iter()
        .filter(|(_, _, policy)| *policy == Some(id))
        .map(|(_, name, _)| name)
        .collect();
    if !users.is_empty() {
        return Err(ApiError::Conflict(format!(
            "This policy is still run by the channel(s) {}. Point them to another policy first.",
            users.join(", ")
        )));
    }
    if store::delete_policy(&state.pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(policy_not_found(id))
    }
}

/// Vérifie que la politique d'un canal d'astreinte existe (création et
/// modification d'un canal).
pub(crate) async fn ensure_channel_policy(state: &AppState, settings: &Value) -> ApiResult<()> {
    let Some(id) = store::policy_id_of(settings) else {
        return Err(ApiError::BadRequest("Choose an escalation policy for this channel.".into()));
    };
    store::get_policy(&state.pool, id)
        .await?
        .map(|_| ())
        .ok_or_else(|| ApiError::BadRequest(format!("Escalation policy {id} does not exist.")))
}

// --------------------------------------------------------------------------
// Journal des escalades
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct EscalationQuery {
    pub status: Option<String>,
    pub fingerprint: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct EscalationView {
    pub id: i64,
    pub policy_id: i64,
    pub policy_name: String,
    pub channel_id: i64,
    pub fingerprint: String,
    pub target_id: Option<i64>,
    pub target_name: String,
    pub rule_name: String,
    pub severity: Severity,
    pub status: String,
    pub step: i64,
    pub round: i64,
    pub next_at: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub acked_by: Option<String>,
    pub events: Vec<store::Event>,
}

/// `GET /api/oncall/escalations?status=active|ended&fingerprint=&limit=`
pub async fn escalations(
    State(state): State<AppState>,
    Query(query): Query<EscalationQuery>,
) -> ApiResult<Json<Vec<EscalationView>>> {
    let active = match query.status.as_deref() {
        None | Some("") | Some("all") => None,
        Some("active") => Some(true),
        Some("ended") => Some(false),
        Some(other) => {
            return Err(ApiError::BadRequest(format!(
                "Unknown status \"{other}\" (active, ended or all)."
            )));
        }
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let names: HashMap<i64, String> =
        store::list_policies(&state.pool).await?.into_iter().map(|p| (p.id, p.name)).collect();
    let rows =
        store::list_escalations(&state.pool, active, query.fingerprint.as_deref(), limit).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let events = store::events_of(&state.pool, row.id).await?;
        out.push(EscalationView {
            policy_name: names.get(&row.policy_id).cloned().unwrap_or_default(),
            id: row.id,
            policy_id: row.policy_id,
            channel_id: row.channel_id,
            fingerprint: row.fingerprint,
            target_id: row.target_id,
            target_name: row.target_name,
            rule_name: row.rule_name,
            severity: row.severity,
            status: row.status,
            step: row.next_step,
            round: row.round,
            next_at: row.next_at.map(to_wire),
            started_at: row.started_at.map(to_wire),
            ended_at: row.ended_at.map(to_wire),
            acked_by: row.acked_by,
            events,
        });
    }
    Ok(Json(out))
}

// --------------------------------------------------------------------------
// Mon compte : contacts
// --------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TeamChannel {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub recipient: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ContactOptions {
    pub kinds: Vec<&'static str>,
    pub team_channels: Vec<TeamChannel>,
}

/// `GET /api/account/contacts/options`
pub async fn contact_options(
    State(state): State<AppState>,
    Authenticated(_): Authenticated,
) -> ApiResult<Json<ContactOptions>> {
    let rows = sqlx::query_as::<_, (i64, String, String, i64)>(
        "SELECT id, name, kind, enabled FROM notification_channels ORDER BY name",
    )
    .fetch_all(&state.pool)
    .await?;
    let team_channels = rows
        .into_iter()
        .filter(|(_, _, _, enabled)| *enabled != 0)
        .filter_map(|(id, name, kind, _)| {
            recipient_key(&kind).map(|(_, _, recipient)| TeamChannel { id, name, kind, recipient })
        })
        .collect();
    Ok(Json(ContactOptions { kinds: PERSONAL_KINDS.to_vec(), team_channels }))
}

#[derive(Debug, Serialize)]
pub struct ContactView {
    pub id: i64,
    pub label: String,
    pub kind: String,
    pub via_channel_id: Option<i64>,
    pub via_channel_name: Option<String>,
    pub address: Option<String>,
    pub settings: Value,
    pub has_secret: bool,
    pub enabled: bool,
    pub min_severity: Severity,
    pub last_error: Option<String>,
    pub last_used_at: Option<String>,
}

async fn contact_views(state: &AppState, user_id: i64) -> ApiResult<Vec<ContactView>> {
    let names: HashMap<i64, String> =
        sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM notification_channels")
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();
    Ok(store::list_contacts(&state.pool, &state.cipher, Some(user_id))
        .await?
        .into_iter()
        .map(|c| ContactView {
            via_channel_name: c.via_channel_id.and_then(|id| names.get(&id).cloned()),
            id: c.id,
            label: c.label,
            kind: c.kind,
            via_channel_id: c.via_channel_id,
            address: c.address,
            settings: c.settings,
            has_secret: c.has_secret,
            enabled: c.enabled,
            min_severity: c.min_severity,
            last_error: c.last_error,
            last_used_at: c.last_used_at.map(to_wire),
        })
        .collect())
}

async fn my_contact(state: &AppState, user: &User, id: i64) -> ApiResult<ContactRow> {
    store::list_contacts(&state.pool, &state.cipher, Some(user.id))
        .await?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Contact {id} not found.")))
}

#[derive(Debug, Deserialize)]
pub struct ContactPayload {
    pub label: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub via_channel_id: Option<i64>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub settings: Option<Value>,
    #[serde(default)]
    pub secrets: Option<Value>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub min_severity: Option<String>,
}

/// Vérifie une adresse selon ce que le canal d'équipe attend.
fn check_address(recipient: &str, address: &str) -> Result<(), String> {
    let ok = match recipient {
        "email" => {
            address.contains('@') && !address.contains(char::is_whitespace) && address.len() <= 254
        }
        "phone" => {
            address.starts_with('+')
                && address.len() <= 20
                && address[1..].chars().all(|c| c.is_ascii_digit())
                && address.len() > 6
        }
        _ => {
            let digits = address.strip_prefix('-').unwrap_or(address);
            !digits.is_empty() && digits.len() <= 20 && digits.chars().all(|c| c.is_ascii_digit())
        }
    };
    if ok {
        Ok(())
    } else {
        Err(match recipient {
            "email" => format!("\"{address}\" is not an email address."),
            "phone" => format!(
                "\"{address}\" is not a phone number in international format (+33612345678)."
            ),
            _ => format!("\"{address}\" is not a Telegram chat ID (a number)."),
        })
    }
}

impl ContactPayload {
    async fn validate(
        self,
        state: &AppState,
        user: &User,
        current: Option<&ContactRow>,
    ) -> ApiResult<store::ContactDraft> {
        let label = self.label.trim().to_string();
        if label.is_empty() || label.chars().count() > 80 {
            return Err(ApiError::BadRequest(
                "The contact needs a label (80 characters at most).".into(),
            ));
        }
        let min_severity = match self.min_severity.as_deref().map(str::trim) {
            None | Some("") => current.map_or(Severity::Info, |c| c.min_severity),
            Some(raw @ ("info" | "warning" | "critical")) => Severity::parse(raw),
            Some(other) => {
                return Err(ApiError::BadRequest(format!(
                    "Unknown severity \"{other}\" (info, warning or critical)."
                )));
            }
        };
        let enabled = self.enabled.or(current.map(|c| c.enabled)).unwrap_or(true);

        if let Some(channel_id) = self.via_channel_id {
            let channels = db::alerts::list_channels(&state.pool, &state.cipher).await?;
            let channel =
                channels.iter().find(|c| c.id == channel_id && c.enabled).ok_or_else(|| {
                    ApiError::BadRequest(format!(
                        "Team channel {channel_id} does not exist or is disabled."
                    ))
                })?;
            let (_, _, recipient) = recipient_key(&channel.kind).ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "The channel \"{}\" cannot reach a single person: pick an email, SMS, Signal or Telegram channel.",
                    channel.name
                ))
            })?;
            let address = self.address.as_deref().map(str::trim).unwrap_or_default().to_string();
            check_address(recipient, &address).map_err(ApiError::BadRequest)?;
            return Ok(store::ContactDraft {
                id: current.map(|c| c.id),
                user_id: user.id,
                label,
                kind: channel.kind.clone(),
                via_channel_id: Some(channel_id),
                address: Some(address),
                settings: json!({}),
                secrets: Some(json!({})),
                enabled,
                min_severity,
            });
        }

        let kind = self
            .kind
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_string)
            .or_else(|| current.filter(|c| c.via_channel_id.is_none()).map(|c| c.kind.clone()))
            .ok_or_else(|| ApiError::BadRequest("Choose a service, or a team channel.".into()))?;
        if !PERSONAL_KINDS.contains(&kind.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "\"{kind}\" cannot be a personal contact (available: {}).",
                PERSONAL_KINDS.join(", ")
            )));
        }
        // Même validation qu'un canal : secrets fusionnés, refusés dans les
        // réglages, jamais renvoyés vers une destination qu'ils n'ont pas connue,
        // et configuration éprouvée par le constructeur du notificateur.
        let existing = current.filter(|c| c.via_channel_id.is_none()).map(|c| ChannelConfig {
            id: c.id,
            name: c.label.clone(),
            kind: c.kind.clone(),
            enabled: c.enabled,
            settings: c.settings.clone(),
            secrets: c.secrets.clone(),
        });
        let draft = ChannelPayload {
            name: label.clone(),
            kind: kind.clone(),
            enabled: Some(enabled),
            settings: self.settings,
            secrets: self.secrets,
            policy: None,
        }
        .validate(existing.as_ref())?;
        Ok(store::ContactDraft {
            id: current.map(|c| c.id),
            user_id: user.id,
            label,
            kind,
            via_channel_id: None,
            address: None,
            settings: draft.settings,
            secrets: draft.secrets,
            enabled,
            min_severity,
        })
    }
}

/// `GET /api/account/contacts`
pub async fn list_contacts(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> ApiResult<Json<Vec<ContactView>>> {
    Ok(Json(contact_views(&state, user.id).await?))
}

async fn one_contact(state: &AppState, user: &User, id: i64) -> ApiResult<ContactView> {
    contact_views(state, user.id)
        .await?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Contact {id} not found.")))
}

/// `POST /api/account/contacts`
pub async fn create_contact(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Json(payload): Json<ContactPayload>,
) -> ApiResult<(StatusCode, Json<ContactView>)> {
    let existing = store::list_contacts(&state.pool, &state.cipher, Some(user.id)).await?;
    if existing.len() >= MAX_CONTACTS {
        return Err(ApiError::BadRequest(format!("{MAX_CONTACTS} contacts at most per account.")));
    }
    let draft = payload.validate(&state, &user, None).await?;
    let id = store::save_contact(&state.pool, &state.cipher, &draft).await?;
    Ok((StatusCode::CREATED, Json(one_contact(&state, &user, id).await?)))
}

/// `PUT /api/account/contacts/{id}`
pub async fn update_contact(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<ContactPayload>,
) -> ApiResult<Json<ContactView>> {
    let current = my_contact(&state, &user, id).await?;
    let draft = payload.validate(&state, &user, Some(&current)).await?;
    store::save_contact(&state.pool, &state.cipher, &draft).await?;
    Ok(Json(one_contact(&state, &user, id).await?))
}

/// `DELETE /api/account/contacts/{id}`
pub async fn delete_contact(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::delete_contact(&state.pool, user.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Contact {id} not found.")))
    }
}

#[derive(Debug, Serialize)]
pub struct TestReport {
    pub ok: bool,
    pub message: String,
}

async fn public_url(state: &AppState) -> ApiResult<Option<String>> {
    let global = notify::policy_store::load_global(&state.pool).await?;
    let env_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
    Ok(global.public_url(env_url.as_deref()))
}

/// `POST /api/account/contacts/{id}/test`
pub async fn test_contact(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
) -> ApiResult<Json<TestReport>> {
    let contact = my_contact(&state, &user, id).await?;
    let http = notify::http_client();
    let ctx = Ctx {
        pool: &state.pool,
        cipher: &state.cipher,
        http: &http,
        public_url: None,
        action_key: None,
    };
    match runner::test_contact(&ctx, &user, &contact).await {
        Ok(()) => Ok(Json(TestReport {
            ok: true,
            message: format!("Test message sent to \"{}\".", contact.label),
        })),
        Err(error) => {
            Err(ApiError::BadRequest(format!("Sending to \"{}\" failed: {error}", contact.label)))
        }
    }
}

// --------------------------------------------------------------------------
// Mon compte : heures et calendrier
// --------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct NextShift {
    pub schedule_id: i64,
    pub schedule_name: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct Profile {
    pub timezone: String,
    pub quiet_hours: Option<Schedule>,
    pub calendar: bool,
    pub on_call: Vec<OnCallEntry>,
    pub next_shift: Option<NextShift>,
}

async fn profile_of(state: &AppState, user: &User) -> ApiResult<Profile> {
    let now = Utc::now();
    let prefs = store::get_prefs(&state.pool, user.id).await?;
    let schedules = runner::load_schedules(&state.pool, now).await?;
    let on_call =
        entries(&schedules, now).into_iter().filter(|e| e.user_id == Some(user.id)).collect();
    let horizon = now + TimeDelta::days(60);
    let next_shift = schedules
        .iter()
        .flat_map(|(row, overrides)| {
            row.definition
                .shifts(overrides, now, horizon)
                .into_iter()
                .filter(|s| s.user_id == user.id && s.start > now)
                .map(move |s| (row, s))
        })
        .min_by_key(|(_, s)| s.start)
        .map(|(row, s)| NextShift {
            schedule_id: row.id,
            schedule_name: row.name.clone(),
            start: to_wire(s.start),
            end: to_wire(s.end),
        });
    Ok(Profile {
        timezone: prefs.timezone,
        quiet_hours: prefs.quiet_hours,
        calendar: prefs.calendar,
        on_call,
        next_shift,
    })
}

/// `GET /api/account/oncall`
pub async fn profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> ApiResult<Json<Profile>> {
    Ok(Json(profile_of(&state, &user).await?))
}

#[derive(Debug, Deserialize)]
pub struct ProfilePayload {
    #[serde(default)]
    pub timezone: Option<String>,
    /// Absent : conservées ; `null` : effacées.
    #[serde(default, deserialize_with = "present")]
    pub quiet_hours: Option<Value>,
}

fn present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// `PUT /api/account/oncall`
pub async fn update_profile(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Json(payload): Json<ProfilePayload>,
) -> ApiResult<Json<Profile>> {
    let current = store::get_prefs(&state.pool, user.id).await?;
    let timezone = match payload.timezone.as_deref().map(str::trim) {
        None => current.timezone,
        Some("") => String::new(),
        Some(name) => {
            parse_tz(name)
                .ok_or_else(|| ApiError::BadRequest(format!("Unknown time zone \"{name}\".")))?;
            name.to_string()
        }
    };
    let quiet_hours = match payload.quiet_hours {
        None => current.quiet_hours,
        Some(Value::Null) => None,
        Some(value) => {
            let schedule: Schedule = serde_json::from_value(value)
                .map_err(|e| ApiError::BadRequest(format!("Quiet hours not understood: {e}")))?;
            if !matches!(schedule, Schedule::Weekly { .. }) {
                return Err(ApiError::BadRequest("Quiet hours are a weekly window.".into()));
            }
            Some(schedule)
        }
    };
    store::save_prefs(&state.pool, user.id, &timezone, quiet_hours.as_ref()).await?;
    Ok(Json(profile_of(&state, &user).await?))
}

fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[derive(Debug, Serialize)]
pub struct CalendarLink {
    pub url: String,
}

/// `POST /api/account/oncall/calendar` — un nouveau lien ; l'ancien cesse de
/// fonctionner.
pub async fn create_calendar(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> ApiResult<Json<CalendarLink>> {
    let token = crate::crypto::generate_secret();
    store::set_calendar_hash(&state.pool, user.id, Some(&token_hash(&token))).await?;
    let path = format!("/api/public/oncall/calendar/{token}.ics");
    let url = match public_url(&state).await? {
        Some(base) => format!("{base}{path}"),
        None => path,
    };
    Ok(Json(CalendarLink { url }))
}

/// `DELETE /api/account/oncall/calendar`
pub async fn revoke_calendar(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> ApiResult<StatusCode> {
    store::set_calendar_hash(&state.pool, user.id, None).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/public/oncall/calendar/{token}.ics` — les gardes de la personne,
/// de trente jours en arrière à quatre-vingt-dix jours devant.
pub async fn calendar(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Response> {
    let token = token.strip_suffix(".ics").unwrap_or(&token);
    let not_found = || ApiError::NotFound("Unknown calendar link.".into());
    if token.is_empty() || token.len() > 128 {
        return Err(not_found());
    }
    let user_id = store::user_by_calendar_hash(&state.pool, &token_hash(token))
        .await?
        .ok_or_else(not_found)?;
    let user =
        users::get(&state.pool, user_id).await?.filter(|u| !u.disabled).ok_or_else(not_found)?;
    let now = Utc::now();
    let from = now - TimeDelta::days(30);
    let to = now + TimeDelta::days(90);
    let schedules = store::list_schedules(&state.pool).await?;
    let overrides = store::list_overrides(&state.pool, None, Some(from)).await?;
    let mut collected = Vec::new();
    for row in &schedules {
        let mine: Vec<_> =
            overrides.iter().filter(|o| o.schedule_id == row.id).map(OverrideRow::pure).collect();
        for shift in row.definition.shifts(&mine, from, to) {
            if shift.user_id == user.id {
                collected.push(ical::Entry {
                    schedule_id: row.id,
                    schedule_name: &row.name,
                    shift,
                });
            }
        }
    }
    collected.sort_by_key(|entry| entry.shift.start);
    let body = ical::render(&runner::display_name(&user), &collected, now);
    Ok((
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (header::CACHE_CONTROL, "private, max-age=300"),
        ],
        body,
    )
        .into_response())
}
