//! Réglages du canal chiffré des agents (Settings → Agents).
//!
//! Administrateur et session seulement (`/agent/tls` est fermé aux jetons
//! d'API) : exiger mTLS ou faire tourner l'autorité peut couper tout un parc.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::agent_tls::{self, POST_QUANTUM_GROUP, pki};
use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminIdentity;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agent/tls", get(overview).put(update))
        .route("/agent/tls/rotation", post(start_rotation).delete(cancel_rotation))
        .route("/agent/tls/rotation/complete", post(complete_rotation))
}

/// Canal par lequel une machine a parlé en dernier.
#[derive(Debug, Serialize)]
pub struct AgentChannel {
    /// Identifiant de la cible « agent ».
    pub id: i64,
    /// `mtls` (canal chiffré), `legacy` (ancien canal) ou `none` (jamais vu).
    pub channel: &'static str,
    /// Groupe d'échange de clés négocié au dernier lot chiffré.
    pub group: Option<String>,
    pub tls_seen_at: Option<String>,
    pub legacy_seen_at: Option<String>,
}

/// Rotation de l'autorité en cours.
#[derive(Debug, Serialize)]
pub struct Rotation {
    pub started_at: Option<String>,
    /// Empreinte de l'autorité suivante.
    pub next_pin: String,
    /// Agents qui ont parlé par le canal chiffré depuis le début de la
    /// rotation, donc épinglé la nouvelle autorité.
    pub ready: usize,
    /// Agents qui utilisent le canal chiffré.
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    /// Vrai si le port des agents est ouvert (`DUMBMONIT_AGENT_LISTEN`).
    pub enabled: bool,
    /// Port que les agents composent.
    pub port: Option<u16>,
    /// L'ancien canal est-il refusé, en ce moment ?
    pub required: bool,
    /// Le réglage tel que l'administrateur l'a posé.
    pub required_setting: bool,
    /// Fin de la période de transition (`DUMBMONIT_AGENT_LEGACY_DAYS`).
    pub legacy_until: Option<String>,
    /// Version de TLS et échange de clés du canal, pour l'affichage.
    pub protocol: &'static str,
    pub key_exchange: &'static str,
    /// Empreinte SHA-256 de l'autorité active.
    pub ca_pin: Option<String>,
    pub rotation: Option<Rotation>,
    pub agents: Vec<AgentChannel>,
}

fn at(ms: Option<i64>) -> Option<String> {
    let ms = ms?;
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// `GET /api/agent/tls`
async fn overview(_: AdminIdentity, State(state): State<AppState>) -> ApiResult<Json<Overview>> {
    let authorities = pki::authorities(&state.pool).await?;
    let ca_pin =
        authorities.iter().find(|ca| ca.state == "active").map(|ca| ca.fingerprint.clone());
    let next = authorities.iter().find(|ca| ca.state == "next");

    let rows = sqlx::query(
        "SELECT target_id, tls_seen_ms, tls_group, legacy_seen_ms FROM agent_hosts ORDER BY target_id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;
    let mut agents = Vec::with_capacity(rows.len());
    let mut ready = 0;
    let mut total = 0;
    for row in &rows {
        let tls: Option<i64> = row.try_get("tls_seen_ms").map_err(anyhow::Error::from)?;
        let legacy: Option<i64> = row.try_get("legacy_seen_ms").map_err(anyhow::Error::from)?;
        // Le canal du dernier lot : une machine revenue à l'ancien canal (agent
        // réinstallé en ancienne version) n'est plus comptée comme chiffrée.
        let channel = match (tls, legacy) {
            (Some(tls), Some(legacy)) if legacy > tls => "legacy",
            (Some(_), _) => "mtls",
            (None, Some(_)) => "legacy",
            (None, None) => "none",
        };
        if channel == "mtls" {
            total += 1;
            if next.is_some_and(|next| tls.is_some_and(|tls| tls > next.created_ms)) {
                ready += 1;
            }
        }
        agents.push(AgentChannel {
            id: row.try_get("target_id").map_err(anyhow::Error::from)?,
            channel,
            group: row.try_get("tls_group").map_err(anyhow::Error::from)?,
            tls_seen_at: at(tls),
            legacy_seen_at: at(legacy),
        });
    }

    Ok(Json(Overview {
        enabled: state.config.agent_listen.is_some(),
        port: state.config.agent_port(),
        required: agent_tls::mtls_required(&state).await?,
        required_setting: agent_tls::required_setting(&state.pool).await?,
        legacy_until: at(agent_tls::legacy_deadline(&state).await?),
        protocol: "TLS 1.3",
        key_exchange: POST_QUANTUM_GROUP,
        ca_pin,
        rotation: next.map(|next| Rotation {
            started_at: at(Some(next.created_ms)),
            next_pin: next.fingerprint.clone(),
            ready,
            total,
        }),
        agents,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    /// Exiger le canal chiffré : l'ancien canal est alors refusé.
    pub required: bool,
}

/// `PUT /api/agent/tls`
async fn update(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    Json(payload): Json<UpdatePayload>,
) -> ApiResult<Json<Overview>> {
    if payload.required && state.config.agent_listen.is_none() {
        return Err(ApiError::BadRequest(
            "The agent port is closed (DUMBMONIT_AGENT_LISTEN=off): requiring it would cut \
             every agent off."
                .into(),
        ));
    }
    agent_tls::set_required(&state.pool, payload.required).await?;
    let action = if payload.required { "agent_tls.required" } else { "agent_tls.optional" };
    audit::record(&state.pool, Some(&me.label()), action, None, ip).await;
    overview(AdminIdentity(me), State(state)).await
}

/// `POST /api/agent/tls/rotation` : crée l'autorité suivante. Les agents
/// l'épinglent à leur prochain échange chiffré ; elle ne signe rien avant la
/// fin de la rotation.
async fn start_rotation(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
) -> ApiResult<Json<Overview>> {
    pki::start_rotation(&state.pool, &state.cipher).await?;
    agent_tls::reload(&state).await?;
    audit::record(&state.pool, Some(&me.label()), "agent_tls.rotation_started", None, ip).await;
    overview(AdminIdentity(me), State(state)).await
}

/// `POST /api/agent/tls/rotation/complete` : l'autorité suivante devient
/// active et signe le certificat du serveur.
async fn complete_rotation(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
) -> ApiResult<Json<Overview>> {
    if !pki::complete_rotation(&state.pool).await? {
        return Err(ApiError::NotFound("No certificate authority rotation in progress.".into()));
    }
    agent_tls::reload(&state).await?;
    audit::record(&state.pool, Some(&me.label()), "agent_tls.rotation_completed", None, ip).await;
    overview(AdminIdentity(me), State(state)).await
}

/// `DELETE /api/agent/tls/rotation` : abandonne une rotation commencée.
async fn cancel_rotation(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
) -> ApiResult<StatusCode> {
    if !pki::cancel_rotation(&state.pool).await? {
        return Err(ApiError::NotFound("No certificate authority rotation in progress.".into()));
    }
    agent_tls::reload(&state).await?;
    audit::record(&state.pool, Some(&me.label()), "agent_tls.rotation_cancelled", None, ip).await;
    Ok(StatusCode::NO_CONTENT)
}
