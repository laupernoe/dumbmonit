//! Calendriers iCal des maintenances.
//!
//! Deux flux :
//!
//! - `GET /api/public/status/{slug}/maintenance.ics` : les maintenances
//!   annoncées sur une page de statut, ouvert comme la page (le mode d'accès de
//!   la page s'applique, voir `status_pages::access`) ;
//! - `GET /api/calendar/maintenance.ics?token=…` : toutes les maintenances de
//!   l'instance, protégé par un jeton propre au flux. Le jeton est un secret
//!   d'URL, comme celui d'un heartbeat : l'administrateur le lit et le fait
//!   tourner depuis l'interface (`/api/calendar/maintenance`).
//!
//! Seules les annonces de type `maintenance` y figurent, de 90 jours dans le
//! passé à l'avenir lointain.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::api::status_pages::{access, parse_stored};
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::db;
use crate::db::status_pages::Incident;
use crate::ical::{self, Event};
use crate::state::AppState;

/// Clé du jeton dans `app_settings`.
const TOKEN_KEY: &str = "ical_token";
/// Profondeur du passé repris dans les flux, en jours.
const HISTORY_DAYS: i64 = 90;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/calendar/maintenance", get(show_token))
        .route("/calendar/maintenance/rotate", post(rotate_token))
}

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/public/status/{slug}/maintenance.ics", get(page_feed))
        .route("/calendar/maintenance.ics", get(private_feed))
}

#[derive(Debug, Serialize)]
pub struct TokenView {
    pub token: String,
    /// Chemin du flux, jeton compris : l'interface y ajoute l'origine.
    pub path: String,
}

fn view(token: String) -> TokenView {
    let path = format!("/api/calendar/maintenance.ics?token={token}");
    TokenView { token, path }
}

async fn read_token(state: &AppState) -> ApiResult<Option<String>> {
    Ok(sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(TOKEN_KEY)
        .fetch_optional(&state.pool)
        .await?)
}

async fn write_token(state: &AppState, token: &str) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(TOKEN_KEY)
    .bind(token)
    .execute(&state.pool)
    .await?;
    Ok(())
}

fn new_token() -> String {
    hex::encode(rand::random::<[u8; 24]>())
}

/// `GET /api/calendar/maintenance` : le jeton du flux privé, créé au premier appel.
pub async fn show_token(
    State(state): State<AppState>,
    _admin: AdminIdentity,
) -> ApiResult<Json<TokenView>> {
    let token = match read_token(&state).await? {
        Some(token) => token,
        None => {
            let token = new_token();
            write_token(&state, &token).await?;
            token
        }
    };
    Ok(Json(view(token)))
}

/// `POST /api/calendar/maintenance/rotate` : un nouveau jeton ; l'ancien cesse de valoir.
pub async fn rotate_token(
    State(state): State<AppState>,
    _admin: AdminIdentity,
) -> ApiResult<Json<TokenView>> {
    let token = new_token();
    write_token(&state, &token).await?;
    Ok(Json(view(token)))
}

/// Les événements d'une liste d'annonces : seules les maintenances, avec leur
/// dernier message en description.
async fn events(state: &AppState, incidents: Vec<Incident>) -> ApiResult<Vec<Event>> {
    let mut out = Vec::new();
    for incident in incidents.into_iter().filter(|incident| incident.kind == "maintenance") {
        let Some(start) = parse_stored(&incident.starts_at) else { continue };
        let updates = db::status_pages::list_updates(&state.pool, incident.id).await?;
        out.push(Event {
            uid: format!("maintenance-{}@dumbmonit", incident.id),
            summary: incident.title.clone(),
            description: updates.last().map(|update| update.body.clone()).unwrap_or_default(),
            start,
            end: incident.ends_at.as_deref().and_then(parse_stored),
            modified: parse_stored(&incident.updated_at).unwrap_or(start),
            status: incident.status.clone(),
        });
    }
    out.sort_by_key(|event| event.start);
    Ok(out)
}

fn since() -> String {
    (Utc::now() - TimeDelta::days(HISTORY_DAYS)).format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn calendar_response(body: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("text/calendar; charset=utf-8")),
            (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=300")),
        ],
        body,
    )
        .into_response()
}

/// `GET /api/public/status/{slug}/maintenance.ics`.
pub async fn page_feed(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let page = access::open_page(&state, &slug, &headers).await?;
    let incidents = db::status_pages::incidents_for_page(&state.pool, page.id, &since()).await?;
    let name = format!("{} - maintenance", page.title);
    let body = ical::calendar(&name, &events(&state, incidents).await?);
    Ok(access::seal(calendar_response(body), !page.is_public()))
}

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    #[serde(default)]
    pub token: Option<String>,
}

/// `GET /api/calendar/maintenance.ics?token=…`.
pub async fn private_feed(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
) -> ApiResult<Response> {
    let not_found = || ApiError::NotFound("This calendar does not exist.".into());
    let (Some(given), Some(stored)) = (query.token, read_token(&state).await?) else {
        return Err(not_found());
    };
    if !bool::from(given.as_bytes().ct_eq(stored.as_bytes())) {
        return Err(not_found());
    }
    let since = since();
    let incidents: Vec<Incident> = db::status_pages::list_incidents(&state.pool)
        .await?
        .into_iter()
        .filter(|incident| {
            let last = incident.ends_at.as_deref().unwrap_or(incident.starts_at.as_str());
            last >= since.as_str()
        })
        .collect();
    let body = ical::calendar("DumbMonit - maintenance", &events(&state, incidents).await?);
    let mut response = calendar_response(body);
    let private = HeaderValue::from_static("private, no-store");
    response.headers_mut().insert(header::CACHE_CONTROL, private);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_long_and_distinct() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 48);
        assert_ne!(a, b);
        assert!(view(a.clone()).path.ends_with(&a));
    }
}
