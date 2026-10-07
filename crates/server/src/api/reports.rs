//! Rapports périodiques par courriel : calendriers (CRUD) et aperçu immédiat.
//!
//! Réservé aux administrateurs, lecture comprise : la liste des destinataires
//! est une liste d'adresses de l'équipe.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::db;
use crate::reports::schedule::{Frequency, parse_timezone};
use crate::reports::store::{self, ScheduleInput, StoredSchedule};
use crate::reports::{self, MAX_SCHEDULES};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/reports/schedules", get(list).post(create))
        .route("/reports/schedules/{id}", get(get_one).put(update).delete(remove))
        .route("/reports/schedules/{id}/send-test", post(send_test))
}

#[derive(Debug, Serialize)]
pub struct ScheduleView {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub frequency: String,
    /// 0 = lundi.
    pub weekday: u32,
    pub day_of_month: u32,
    pub hour: u32,
    pub timezone: String,
    pub recipients: Vec<String>,
    pub channel_id: Option<i64>,
    pub last_sent_at: Option<String>,
    pub last_error: Option<String>,
    pub next_run_at: Option<String>,
}

fn iso(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn view(stored: &StoredSchedule, now: DateTime<Utc>) -> ScheduleView {
    ScheduleView {
        id: stored.id,
        name: stored.name.clone(),
        enabled: stored.enabled,
        frequency: stored.frequency.clone(),
        weekday: stored.weekday,
        day_of_month: stored.day_of_month,
        hour: stored.hour,
        timezone: stored.timezone.clone(),
        recipients: stored.recipients.clone(),
        channel_id: stored.channel_id,
        last_sent_at: stored.last_sent_at.map(iso),
        last_error: stored.last_error.clone(),
        next_run_at: reports::next_run(stored, now).map(iso),
    }
}

/// Calendrier soumis. Les champs absents prennent la valeur par défaut :
/// hebdomadaire, le lundi à 8 h, en UTC.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SchedulePayload {
    pub name: String,
    pub enabled: bool,
    pub frequency: String,
    pub weekday: u32,
    pub day_of_month: u32,
    pub hour: u32,
    pub timezone: String,
    pub recipients: Vec<String>,
    pub channel_id: Option<i64>,
}

impl Default for SchedulePayload {
    fn default() -> Self {
        Self {
            name: "Team report".to_string(),
            enabled: true,
            frequency: "weekly".to_string(),
            weekday: 0,
            day_of_month: 1,
            hour: 8,
            timezone: "UTC".to_string(),
            recipients: Vec::new(),
            channel_id: None,
        }
    }
}

impl SchedulePayload {
    async fn validate(self, state: &AppState) -> ApiResult<ScheduleInput> {
        let name = self.name.trim().to_string();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(ApiError::BadRequest(
                "The name must be 1 to 80 characters, without control characters.".into(),
            ));
        }
        let frequency = Frequency::parse(&self.frequency).ok_or_else(|| {
            ApiError::BadRequest(
                "The frequency must be \"daily\", \"weekly\" or \"monthly\".".into(),
            )
        })?;
        if self.weekday > 6 {
            return Err(ApiError::BadRequest(
                "The weekday must be between 0 (Monday) and 6 (Sunday).".into(),
            ));
        }
        if !(1..=28).contains(&self.day_of_month) {
            return Err(ApiError::BadRequest("The day of the month must be 1 to 28.".into()));
        }
        if self.hour > 23 {
            return Err(ApiError::BadRequest("The hour must be between 0 and 23.".into()));
        }
        let timezone = self.timezone.trim().to_string();
        if parse_timezone(&timezone).is_none() {
            return Err(ApiError::BadRequest(
                "The timezone must be an IANA name such as \"Europe/Paris\" or \"UTC\".".into(),
            ));
        }
        let recipients =
            reports::normalise_recipients(&self.recipients).map_err(ApiError::BadRequest)?;
        if self.enabled && recipients.is_empty() {
            return Err(ApiError::BadRequest(
                "Add at least one recipient before enabling the report.".into(),
            ));
        }
        if let Some(id) = self.channel_id {
            let channel = db::alerts::get_channel(&state.pool, &state.cipher, id).await?;
            if !channel.is_some_and(|channel| channel.kind == "smtp") {
                return Err(ApiError::BadRequest(
                    "The channel must be an existing email (SMTP) channel.".into(),
                ));
            }
        }
        Ok(ScheduleInput {
            name,
            enabled: self.enabled,
            frequency: frequency.as_str().to_string(),
            weekday: self.weekday,
            day_of_month: self.day_of_month,
            hour: self.hour,
            timezone,
            recipients,
            channel_id: self.channel_id,
        })
    }
}

fn not_found() -> ApiError {
    ApiError::NotFound("No such report schedule.".into())
}

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminIdentity,
) -> ApiResult<Json<Vec<ScheduleView>>> {
    let now = Utc::now();
    let schedules = store::list(&state.pool).await?;
    Ok(Json(schedules.iter().map(|stored| view(stored, now)).collect()))
}

pub async fn get_one(
    State(state): State<AppState>,
    _admin: AdminIdentity,
    Path(id): Path<i64>,
) -> ApiResult<Json<ScheduleView>> {
    let stored = store::get(&state.pool, id).await?.ok_or_else(not_found)?;
    Ok(Json(view(&stored, Utc::now())))
}

pub async fn create(
    State(state): State<AppState>,
    _admin: AdminIdentity,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<(StatusCode, Json<ScheduleView>)> {
    let input = payload.validate(&state).await?;
    if store::count(&state.pool).await? >= MAX_SCHEDULES {
        return Err(ApiError::Conflict(format!(
            "At most {MAX_SCHEDULES} report schedules are allowed."
        )));
    }
    let now = Utc::now();
    let id = store::insert(&state.pool, &input, now).await?;
    let stored = store::get(&state.pool, id).await?.ok_or_else(not_found)?;
    Ok((StatusCode::CREATED, Json(view(&stored, now))))
}

pub async fn update(
    State(state): State<AppState>,
    _admin: AdminIdentity,
    Path(id): Path<i64>,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<Json<ScheduleView>> {
    let current = store::get(&state.pool, id).await?.ok_or_else(not_found)?;
    let input = payload.validate(&state).await?;
    let now = Utc::now();
    store::update(&state.pool, &current, &input, now).await?;
    let stored = store::get(&state.pool, id).await?.ok_or_else(not_found)?;
    Ok(Json(view(&stored, now)))
}

pub async fn remove(
    State(state): State<AppState>,
    _admin: AdminIdentity,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::delete(&state.pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

#[derive(Debug, Serialize)]
pub struct SendTestReply {
    pub sent: usize,
    pub failed: usize,
}

/// Envoie tout de suite un aperçu du rapport aux destinataires enregistrés.
/// N'affecte pas le calendrier : ni dernier envoi, ni créneau consommé.
pub async fn send_test(
    State(state): State<AppState>,
    _admin: AdminIdentity,
    Path(id): Path<i64>,
) -> ApiResult<Json<SendTestReply>> {
    let stored = store::get(&state.pool, id).await?.ok_or_else(not_found)?;
    if !reports::try_reserve_preview(id) {
        return Err(ApiError::Conflict(
            "A preview was just sent. Wait half a minute before sending another.".into(),
        ));
    }
    match reports::deliver(&state, &stored, true, Utc::now()).await {
        Ok(delivery) => Ok(Json(SendTestReply { sent: delivery.sent, failed: delivery.failed })),
        Err(message) => Err(ApiError::BadRequest(message)),
    }
}
