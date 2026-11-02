//! Mode sentinelle d'une machine : réglage (adresses à sonder, période, seuil,
//! canal de notification).
//!
//! Réservé à un administrateur connecté à l'interface : le canal est un secret,
//! il ne se confie pas à un jeton d'API et ne ressort jamais (`has_secret`,
//! `channel_kind`). Le réglage part vers l'agent dans l'accusé de réception de
//! ses lots (`ingest.rs`), sur le canal chiffré seulement.

use axum::extract::{Extension, Path, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::Serialize;

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::{CurrentPrincipal, Principal};
use crate::collectors::agent::sentinel::{self, Input, Stored};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/targets/{id}/agent/sentinel", get(get_setting).put(put_setting))
}

/// Le réglage tel que l'interface l'affiche : jamais le canal lui-même.
#[derive(Debug, Serialize)]
pub struct SentinelView {
    pub enabled: bool,
    pub urls: Vec<String>,
    pub interval_secs: u64,
    pub threshold: u32,
    /// Un canal de notification est enregistré.
    pub has_secret: bool,
    /// `webhook` ou `ntfy`, quand un canal est enregistré.
    pub channel_kind: Option<&'static str>,
}

impl From<&Stored> for SentinelView {
    fn from(stored: &Stored) -> Self {
        Self {
            enabled: stored.enabled,
            urls: stored.urls.clone(),
            interval_secs: stored.interval_secs,
            threshold: stored.threshold,
            has_secret: stored.channel.is_some(),
            channel_kind: stored.channel.as_ref().map(|channel| channel.kind()),
        }
    }
}

fn admin(principal: &Principal) -> ApiResult<()> {
    match principal {
        Principal::User(user) if user.role.is_admin() => Ok(()),
        _ => Err(ApiError::Forbidden(
            "Only an administrator signed in to the web interface can set the sentinel.".into(),
        )),
    }
}

async fn agent_target(state: &AppState, id: TargetId) -> ApiResult<()> {
    let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM targets WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;
    match kind.as_deref() {
        Some("agent") => Ok(()),
        Some(_) => Err(ApiError::NotFound(format!("Monitor {id} has no agent."))),
        None => Err(ApiError::NotFound(format!("Monitor {id} not found."))),
    }
}

/// `GET /api/targets/{id}/agent/sentinel`
async fn get_setting(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<SentinelView>> {
    admin(&principal)?;
    agent_target(&state, id).await?;
    let stored = sentinel::load(&state.pool, &state.cipher, id).await?.unwrap_or_default();
    Ok(Json(SentinelView::from(&stored)))
}

/// `PUT /api/targets/{id}/agent/sentinel` : règle la sentinelle ; l'agent
/// l'applique à son prochain lot. `channel` omis garde le canal en place,
/// `clear_channel` l'efface.
async fn put_setting(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Path(id): Path<TargetId>,
    Json(input): Json<Input>,
) -> ApiResult<Json<SentinelView>> {
    admin(&principal)?;
    agent_target(&state, id).await?;
    let current = sentinel::load(&state.pool, &state.cipher, id).await?;
    let stored = sentinel::merge(current.as_ref(), input).map_err(ApiError::BadRequest)?;
    sentinel::save(&state.pool, &state.cipher, id, &stored).await?;
    tracing::info!(
        cible = id,
        sentinelle = stored.enabled,
        par = principal.label(),
        "sentinelle de l'agent réglée depuis l'interface"
    );
    Ok(Json(SentinelView::from(&stored)))
}
