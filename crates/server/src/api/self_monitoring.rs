//! Auto-surveillance de l'instance (`crate::selfmon`).
//!
//! * `GET /self-monitoring` — état du moteur (planificateur, base,
//!   VictoriaMetrics), volume de données, taille de la base, relances du
//!   VictoriaMetrics embarqué, et réglage du battement de cœur sortant.
//! * `PUT /self-monitoring/heartbeat` — réglage du battement. L'URL ne revient
//!   jamais : `has_url` et `url_host` seulement. Omise, elle est gardée ; vide,
//!   elle est effacée.
//! * `POST /self-monitoring/heartbeat/test` — un appel « tout va bien » tout de
//!   suite, vers l'URL fournie ou celle rangée.

use std::time::Duration;

use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::selfmon::{self, Component, HeartbeatConfig, PingRecord};
use crate::state::AppState;
use crate::stats::stats;

/// Délai entre deux essais : un bouton martelé ne doit pas inonder le service.
const TEST_COOLDOWN: Duration = Duration::from_secs(3);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/self-monitoring", get(overview))
        .route("/self-monitoring/heartbeat", put(update_heartbeat))
        .route("/self-monitoring/heartbeat/test", post(test_heartbeat))
}

#[derive(Debug, Serialize)]
pub struct HeartbeatView {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub send_fail: bool,
    /// Une URL est rangée ; elle-même ne sort jamais.
    pub has_url: bool,
    /// Son hôte seul, pour reconnaître le service.
    pub url_host: Option<String>,
    pub last_ping: Option<PingRecord>,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    /// `ok` quand les trois composants vont bien, sinon `degraded`.
    pub status: &'static str,
    pub components: Vec<Component>,
    /// Place libre et taille du volume de données ; absentes si illisibles.
    pub data_free_bytes: Option<u64>,
    pub data_total_bytes: Option<u64>,
    pub database_bytes: u64,
    pub vm_embedded: bool,
    /// Relances du VictoriaMetrics embarqué depuis le démarrage du serveur.
    pub vm_restarts: u64,
    pub scheduler_idle_seconds: u64,
    pub scheduler_backlog: u64,
    pub heartbeat: HeartbeatView,
}

async fn heartbeat_view(state: &AppState) -> ApiResult<HeartbeatView> {
    let config = selfmon::load_config(state).await?;
    let url = selfmon::load_url(state).await?;
    Ok(HeartbeatView {
        enabled: config.enabled,
        interval_seconds: config.interval_seconds,
        send_fail: config.send_fail,
        has_url: url.is_some(),
        url_host: url.as_deref().and_then(selfmon::url_host),
        last_ping: selfmon::last_ping(),
    })
}

async fn overview(State(state): State<AppState>) -> ApiResult<Json<Overview>> {
    let components = selfmon::components(&state).await;
    let volume = selfmon::volume(&state.config.data_dir);
    Ok(Json(Overview {
        status: if components.iter().all(|c| c.ok) { "ok" } else { "degraded" },
        components,
        data_free_bytes: volume.map(|(free, _)| free),
        data_total_bytes: volume.map(|(_, size)| size),
        database_bytes: selfmon::database_bytes(&state.config.database_path()).await,
        vm_embedded: state.config.vm_embedded(),
        vm_restarts: stats().vm_restarts(),
        scheduler_idle_seconds: stats().scheduler_idle().as_secs(),
        scheduler_backlog: stats().snapshot().scheduler_backlog,
        heartbeat: heartbeat_view(&state).await?,
    }))
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatUpdate {
    pub enabled: bool,
    #[serde(default)]
    pub interval_seconds: Option<u64>,
    #[serde(default)]
    pub send_fail: Option<bool>,
    /// Omise : l'URL rangée est gardée. Vide : elle est effacée.
    #[serde(default)]
    pub url: Option<String>,
}

async fn update_heartbeat(
    State(state): State<AppState>,
    Json(update): Json<HeartbeatUpdate>,
) -> ApiResult<Json<HeartbeatView>> {
    let current = selfmon::load_config(&state).await?;
    let interval = update.interval_seconds.unwrap_or(current.interval_seconds);
    if !(selfmon::MIN_INTERVAL..=selfmon::MAX_INTERVAL).contains(&interval) {
        return Err(ApiError::BadRequest(format!(
            "The interval must be between {} and {} seconds.",
            selfmon::MIN_INTERVAL,
            selfmon::MAX_INTERVAL
        )));
    }
    let new_url = update.url.as_deref().map(str::trim);
    if let Some(url) = new_url.filter(|url| !url.is_empty()) {
        selfmon::check_url(url).map_err(ApiError::BadRequest)?;
    }
    let will_have_url = match new_url {
        Some(url) => !url.is_empty(),
        None => selfmon::load_url(&state).await?.is_some(),
    };
    if update.enabled && !will_have_url {
        return Err(ApiError::BadRequest(
            "Enter the URL to ping before turning the heartbeat on.".to_string(),
        ));
    }

    if let Some(url) = new_url {
        selfmon::save_url(&state, url).await?;
    }
    let config = HeartbeatConfig {
        enabled: update.enabled,
        interval_seconds: interval,
        send_fail: update.send_fail.unwrap_or(current.send_fail),
    };
    selfmon::save_config(&state, &config).await?;
    Ok(Json(heartbeat_view(&state).await?))
}

#[derive(Debug, Default, Deserialize)]
pub struct HeartbeatTest {
    /// URL à essayer avant de l'enregistrer ; à défaut, celle rangée.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatTestResult {
    pub ok: bool,
    /// Erreur d'envoi, expurgée de l'URL.
    pub error: Option<String>,
}

async fn test_heartbeat(
    State(state): State<AppState>,
    Json(test): Json<HeartbeatTest>,
) -> ApiResult<Json<HeartbeatTestResult>> {
    if let Err(wait) = state.cooldowns().try_take("heartbeat_test", 0, TEST_COOLDOWN) {
        return Err(ApiError::TooManyRequests(format!("Wait {wait} s before testing again.")));
    }
    let given = test.url.filter(|url| !url.trim().is_empty());
    let url = match given {
        Some(url) => {
            selfmon::check_url(&url).map_err(ApiError::BadRequest)?;
            url
        }
        None => match selfmon::load_url(&state).await? {
            Some(url) => url,
            None => return Err(ApiError::BadRequest("Enter the URL to ping first.".to_string())),
        },
    };
    let result = selfmon::ping(&crate::notify::http_client(), &url, false).await;
    Ok(Json(HeartbeatTestResult { ok: result.is_ok(), error: result.err() }))
}
