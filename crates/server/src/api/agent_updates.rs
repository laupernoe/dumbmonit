//! Mise à jour automatique des agents (Settings → Agents).
//!
//! À ne pas confondre avec la mise à jour des conteneurs Docker
//! (`agent_commands.rs`) : il s'agit du binaire `dumbmonit-agent` lui-même.
//!
//! Administrateur et session seulement (`/agent/updates` est fermé aux jetons
//! d'API) : un réglage qui remplace le binaire de tout un parc ne se pose pas
//! par script. Côté agent, le téléchargement du binaire proposé passe par le
//! canal des agents (jeton d'enrôlement, secret de liaison, clé de la
//! machine), chiffré quand l'agent l'a.

use axum::body::Bytes;
use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;

use crate::agent_tls::AgentCert;
use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminIdentity;
use crate::collectors::agent::selfupdate::{self, Overview, SettingsChange, Window};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agent/updates", get(overview).put(update_settings))
        .route("/agent/updates/all", post(update_all))
        .route("/agent/updates/{id}", put(set_agent))
        .route("/agent/updates/{id}/now", post(update_now))
}

/// Route des agents : le binaire proposé, sous les garanties du canal.
pub fn agent_routes() -> Router<AppState> {
    Router::new().route("/agent/update/download/{name}", get(download))
}

#[derive(Debug, Deserialize)]
pub struct KeyQuery {
    #[serde(default)]
    key: String,
}

/// `GET /api/agent/update/download/{name}` : le binaire, pour une machine liée.
async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    cert: Option<Extension<AgentCert>>,
    Path(name): Path<String>,
    Query(query): Query<KeyQuery>,
) -> Response {
    if let Err(rejection) =
        super::agent_commands::authenticate(&state, &headers, &query.key, cert.as_deref()).await
    {
        return rejection.into_response();
    }
    super::agent_files::serve_binary(&state, &name).await
}

/// `GET /api/agent/updates`
async fn overview(_: AdminIdentity, State(state): State<AppState>) -> ApiResult<Json<Overview>> {
    Ok(Json(selfupdate::overview(&state).await?))
}

#[derive(Debug, Default, Deserialize)]
pub struct SettingsPayload {
    #[serde(default)]
    pub enabled: Option<bool>,
    /// `any`, `maintenance` ou `manual`.
    #[serde(default)]
    pub window: Option<String>,
    #[serde(default)]
    pub canary_minutes: Option<u32>,
    #[serde(default)]
    pub allow_unsigned: Option<bool>,
}

/// `PUT /api/agent/updates` : réglages globaux (champs absents inchangés).
async fn update_settings(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    Json(payload): Json<SettingsPayload>,
) -> ApiResult<Json<Overview>> {
    let window = match payload.window.as_deref() {
        None => None,
        Some(text) => Some(Window::parse(text).ok_or_else(|| {
            ApiError::BadRequest("The window must be 'any', 'maintenance' or 'manual'.".into())
        })?),
    };
    if payload.canary_minutes.is_some_and(|minutes| minutes > selfupdate::MAX_CANARY_MINUTES) {
        return Err(ApiError::BadRequest(format!(
            "The canary delay is limited to {} minutes.",
            selfupdate::MAX_CANARY_MINUTES
        )));
    }
    let change = SettingsChange {
        enabled: payload.enabled,
        window,
        canary_minutes: payload.canary_minutes,
        allow_unsigned: payload.allow_unsigned,
    };
    selfupdate::change_settings(&state.pool, &change).await?;
    let action = match payload.allow_unsigned {
        Some(true) => "agent_update.unsigned_allowed",
        _ => "agent_update.settings",
    };
    audit::record(&state.pool, Some(&me.label()), action, None, ip).await;
    Ok(Json(selfupdate::overview(&state).await?))
}

#[derive(Debug, Default, Deserialize)]
pub struct AgentPayload {
    /// `true` / `false` : forcé pour cette machine ; `null` : le réglage
    /// global ; absent : inchangé.
    #[serde(default)]
    pub enabled: Option<Value>,
    #[serde(default)]
    pub canary: Option<bool>,
}

/// `PUT /api/agent/updates/{id}` : réglage propre à une machine.
async fn set_agent(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<AgentPayload>,
) -> ApiResult<Json<Overview>> {
    let mode = match payload.enabled {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::Bool(on)) => Some(Some(on)),
        Some(_) => {
            return Err(ApiError::BadRequest("'enabled' must be true, false or null.".into()));
        }
    };
    if !selfupdate::set_host(&state.pool, id, mode, payload.canary).await? {
        return Err(ApiError::NotFound(format!("Agent {id} not found.")));
    }
    let subject = id.to_string();
    audit::record(&state.pool, Some(&me.label()), "agent_update.agent", Some(&subject), ip).await;
    Ok(Json(selfupdate::overview(&state).await?))
}

#[derive(Debug, Default, Deserialize)]
pub struct NowPayload {
    /// Installer le binaire du serveur même s'il est plus ancien que celui
    /// de la machine.
    #[serde(default)]
    pub allow_downgrade: bool,
}

/// `POST /api/agent/updates/{id}/now` : « Update now » pour une machine.
async fn update_now(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    body: Bytes,
) -> ApiResult<Json<Overview>> {
    // Corps facultatif : `{}` ou rien du tout.
    let payload: NowPayload = if body.iter().all(u8::is_ascii_whitespace) {
        NowPayload::default()
    } else {
        serde_json::from_slice(&body)
            .map_err(|error| ApiError::BadRequest(format!("Unreadable request: {error}")))?
    };
    if !selfupdate::request(&state.pool, id, payload.allow_downgrade).await? {
        return Err(ApiError::NotFound(format!("Agent {id} not found.")));
    }
    let subject = id.to_string();
    audit::record(&state.pool, Some(&me.label()), "agent_update.requested", Some(&subject), ip)
        .await;
    Ok(Json(selfupdate::overview(&state).await?))
}

/// `POST /api/agent/updates/all` : « Update all now ».
async fn update_all(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
) -> ApiResult<Json<Overview>> {
    let count = selfupdate::request_all(&state).await?;
    let subject = count.to_string();
    audit::record(&state.pool, Some(&me.label()), "agent_update.requested_all", Some(&subject), ip)
        .await;
    Ok(Json(selfupdate::overview(&state).await?))
}
