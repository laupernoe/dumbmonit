//! Rapports périodiques par courriel : calendriers (CRUD) et aperçu immédiat.
//!
//! Réservé aux administrateurs, lecture comprise : la liste des destinataires
//! est une liste d'adresses de l'équipe.

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::db;
use crate::reports::schedule::{self, Frequency, Period, parse_timezone};
use crate::reports::store::{self, ScheduleInput, StoredSchedule};
use crate::reports::{self, MAX_SCHEDULES};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/reports/schedules", get(list).post(create))
        .route("/reports/schedules/{id}", get(get_one).put(update).delete(remove))
        .route("/reports/schedules/{id}/send-test", post(send_test))
        .route("/reports/sla", get(download_sla))
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
    /// `digest` ou `sla`.
    pub kind: String,
    pub attach_pdf: bool,
    pub attach_csv: bool,
    pub page_id: Option<i64>,
    pub objective: Option<f64>,
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
        kind: stored.kind.clone(),
        attach_pdf: stored.attach_pdf,
        attach_csv: stored.attach_csv,
        page_id: stored.page_id,
        objective: stored.objective,
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
    /// `digest` (résumé glissant) ou `sla` (mois civil précédent, mensuel).
    pub kind: String,
    pub attach_pdf: bool,
    pub attach_csv: bool,
    pub page_id: Option<i64>,
    pub objective: Option<f64>,
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
            kind: "digest".to_string(),
            attach_pdf: true,
            attach_csv: true,
            page_id: None,
            objective: None,
        }
    }
}

/// Objectif de disponibilité : un pourcentage entre 0 (exclu) et 100.
fn valid_objective(objective: Option<f64>) -> ApiResult<Option<f64>> {
    match objective {
        None => Ok(None),
        Some(value) if value.is_finite() && value > 0.0 && value <= 100.0 => Ok(Some(value)),
        Some(_) => Err(ApiError::BadRequest(
            "The availability objective must be a percentage above 0 and up to 100.".into(),
        )),
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
        let kind = self.kind.trim().to_string();
        if kind != "digest" && kind != "sla" {
            return Err(ApiError::BadRequest("The kind must be \"digest\" or \"sla\".".into()));
        }
        if kind == "sla" && frequency != Frequency::Monthly {
            return Err(ApiError::BadRequest(
                "An SLA report covers a calendar month: choose the monthly frequency.".into(),
            ));
        }
        if let Some(id) = self.page_id
            && db::status_pages::get_page(&state.pool, id).await?.is_none()
        {
            return Err(ApiError::BadRequest("The status page does not exist.".into()));
        }
        let objective = valid_objective(self.objective)?;
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
            kind,
            attach_pdf: self.attach_pdf,
            attach_csv: self.attach_csv,
            page_id: self.page_id,
            objective,
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

/// Paramètres de `GET /api/reports/sla`.
#[derive(Debug, Deserialize)]
pub struct SlaQuery {
    /// Premier jour inclus, `AAAA-MM-JJ`. Absent : le mois civil précédent.
    pub from: Option<String>,
    /// Dernier jour inclus. Absent avec `from` : aujourd'hui.
    pub to: Option<String>,
    /// Fuseau IANA de la période et de l'affichage. Défaut : UTC.
    pub tz: Option<String>,
    /// `pdf` (défaut), `csv` (moniteurs) ou `incidents_csv`.
    pub format: Option<String>,
    /// Objectif de disponibilité, en pour cent.
    pub objective: Option<f64>,
    /// Page de statut dont le nom habille le rapport.
    pub page: Option<i64>,
    /// Titre du rapport (nom affiché sous l'en-tête).
    pub title: Option<String>,
}

/// Plus longue période demandable : une année (bissextile comprise).
const MAX_SLA_DAYS: i64 = 366;

fn parse_day(label: &str, raw: &str) -> ApiResult<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        .map_err(|_| ApiError::BadRequest(format!("\"{label}\" must be a date like 2026-10-31.")))
}

/// `GET /api/reports/sla` : le rapport de disponibilité d'une période choisie,
/// à la demande, en PDF ou en CSV.
pub async fn download_sla(
    State(state): State<AppState>,
    _admin: AdminIdentity,
    Query(query): Query<SlaQuery>,
) -> ApiResult<Response> {
    use crate::reports::{collect, sla};
    let tz_name = query.tz.as_deref().map(str::trim).filter(|t| !t.is_empty()).unwrap_or("UTC");
    let tz = parse_timezone(tz_name).ok_or_else(|| {
        ApiError::BadRequest("The timezone must be an IANA name such as \"Europe/Paris\".".into())
    })?;
    let now = Utc::now();
    let period = match (query.from.as_deref(), query.to.as_deref()) {
        (None, None) => Period::previous_calendar_month(now, tz)
            .ok_or_else(|| ApiError::BadRequest("Cannot work out the month.".into()))?,
        (from, to) => {
            let today = now.with_timezone(&tz).date_naive();
            let first = match from {
                Some(raw) => parse_day("from", raw)?,
                None => {
                    return Err(ApiError::BadRequest("Give \"from\" together with \"to\".".into()));
                }
            };
            let last = match to {
                Some(raw) => parse_day("to", raw)?,
                None => today,
            };
            if last < first {
                return Err(ApiError::BadRequest("\"to\" must not be before \"from\".".into()));
            }
            if (last - first).num_days() + 1 > MAX_SLA_DAYS {
                return Err(ApiError::BadRequest(format!(
                    "A report covers at most {MAX_SLA_DAYS} days."
                )));
            }
            let start = schedule::local_midnight(first, tz)
                .ok_or_else(|| ApiError::BadRequest("\"from\" is not a valid date.".into()))?;
            let end = schedule::local_midnight(last.succ_opt().unwrap_or(last), tz)
                .ok_or_else(|| ApiError::BadRequest("\"to\" is not a valid date.".into()))?;
            if start >= now {
                return Err(ApiError::BadRequest("The period starts in the future.".into()));
            }
            // Une période en cours s'arrête à maintenant.
            Period::custom(start, end.min(now))
        }
    };

    let title = query.title.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let name = title.unwrap_or("Availability report");
    if name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "The title must be 120 characters at most, without control characters.".into(),
        ));
    }
    let objective = valid_objective(query.objective)?;
    if let Some(id) = query.page
        && db::status_pages::get_page(&state.pool, id).await?.is_none()
    {
        return Err(ApiError::BadRequest("The status page does not exist.".into()));
    }

    let data = collect::collect_period(&state, name, Frequency::Monthly, period, now)
        .await
        .map_err(|error| ApiError::Internal(error.context("collecte du rapport SLA")))?;
    let options = reports::sla_options(&state, query.page, objective, now).await;
    let stem = sla::file_stem(&data, tz);
    let (body, content_type, filename) = match query.format.as_deref().unwrap_or("pdf") {
        "pdf" => (sla::pdf(&data, tz, &options), "application/pdf", format!("{stem}.pdf")),
        "csv" => (
            sla::csv_monitors(&data, objective).into_bytes(),
            reports::CSV_TYPE,
            format!("{stem}-monitors.csv"),
        ),
        "incidents_csv" => (
            sla::csv_incidents(&data).into_bytes(),
            reports::CSV_TYPE,
            format!("{stem}-incidents.csv"),
        ),
        other => {
            return Err(ApiError::BadRequest(format!(
                "Unknown format \"{other}\" (expected: pdf, csv, incidents_csv)."
            )));
        }
    };
    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
            (header::CACHE_CONTROL, "private, no-store".to_string()),
        ],
        body,
    )
        .into_response())
}
