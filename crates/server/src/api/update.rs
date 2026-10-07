//! Mise à jour guidée : la dernière version publiée, lue sur GitHub au plus une
//! fois par jour (`crate::update`).
//!
//! * `GET /update` — version courante, dernière version, notes, `update_available`.
//! * `POST /update/check` — vérifie maintenant (au plus une fois par minute).
//! * `PUT /update/settings` — active ou coupe la vérification (administrateur).

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::{get, post, put};
use chrono::Utc;
use serde::Deserialize;

use crate::api::{ApiError, ApiResult};
use crate::state::AppState;
use crate::update::{self, CheckError, Checker, UpdateInfo};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/update", get(status))
        .route("/update/check", post(check_now))
        .route("/update/settings", put(settings))
}

async fn run(state: &AppState, force: bool) -> ApiResult<UpdateInfo> {
    let locked = update::env_disabled();
    // Le mode démonstration n'a pas à joindre GitHub depuis un serveur public.
    let enabled = !locked
        && !state.config.demo
        && update::setting_enabled(&state.pool).await.map_err(|e| ApiError::Internal(e.into()))?;
    Checker::global().get(Utc::now(), enabled, locked, force, update::fetch_releases).await.map_err(
        |CheckError::TooSoon(seconds)| {
            ApiError::Conflict(format!("Checked a moment ago. Try again in {seconds} s."))
        },
    )
}

async fn status(State(state): State<AppState>) -> ApiResult<Json<UpdateInfo>> {
    run(&state, false).await.map(Json)
}

async fn check_now(State(state): State<AppState>) -> ApiResult<Json<UpdateInfo>> {
    run(&state, true).await.map(Json)
}

#[derive(Deserialize)]
struct SettingsPayload {
    enabled: bool,
}

async fn settings(
    State(state): State<AppState>,
    Json(payload): Json<SettingsPayload>,
) -> ApiResult<Json<UpdateInfo>> {
    update::set_setting_enabled(&state.pool, payload.enabled)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    run(&state, false).await.map(Json)
}
