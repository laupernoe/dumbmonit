//! Garde de l'enrôlement des agents (Settings → Agents) : réglages, machines
//! en attente de validation, remise en service d'un jeton en veille.
//!
//! Administrateur et session seulement (`/agent/enrolment` et `/agent/tokens`
//! sont fermés aux jetons d'API) : accepter une machine, c'est lui ouvrir le
//! serveur.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminIdentity;
use crate::collectors::agent::enrolment::{self, EnrolmentSettings, PendingRequest};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agent/enrolment", get(overview).put(update))
        .route("/agent/enrolment/decide", post(decide))
        .route("/agent/tokens/{id}/reactivate", post(reactivate))
}

/// Ce que la section « Enrolment approval » affiche.
#[derive(Debug, Serialize)]
pub struct Overview {
    pub settings: EnrolmentSettings,
    /// Machines en attente, la plus récente d'abord.
    pub pending: Vec<PendingRequest>,
    /// Plafond des machines en attente : au-delà, les demandes sont refusées.
    pub max_pending: i64,
    /// Heures pendant lesquelles une machine refusée n'est plus montrée.
    pub reject_hold_hours: i64,
}

/// `GET /api/agent/enrolment`.
async fn overview(_: AdminIdentity, State(state): State<AppState>) -> ApiResult<Json<Overview>> {
    Ok(Json(Overview {
        settings: enrolment::load_settings(&state.pool).await?,
        pending: enrolment::list_pending(&state.pool).await?,
        max_pending: enrolment::MAX_PENDING,
        reject_hold_hours: enrolment::REJECT_HOLD_HOURS,
    }))
}

/// `PUT /api/agent/enrolment` : les réglages, en entier.
async fn update(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    Json(settings): Json<EnrolmentSettings>,
) -> ApiResult<Json<Overview>> {
    settings.validate().map_err(ApiError::BadRequest)?;
    enrolment::save_settings(&state.pool, &settings).await?;
    audit::record(&state.pool, Some(&me.label()), "agent_enrolment.settings", None, ip).await;
    overview(AdminIdentity(me), State(state)).await
}

#[derive(Debug, Deserialize)]
pub struct DecidePayload {
    /// Demandes à trancher (cases cochées, ou toutes).
    pub ids: Vec<i64>,
    /// Vrai : accepter ; faux : refuser.
    pub approve: bool,
}

#[derive(Debug, Serialize)]
pub struct Decided {
    /// Demandes effectivement tranchées (une demande déjà tranchée est ignorée).
    pub decided: u64,
}

/// `POST /api/agent/enrolment/decide` : accepte ou refuse, une machine ou
/// plusieurs. Une machine acceptée entre à sa prochaine tentative ; refusée,
/// elle n'est plus montrée pendant 24 h, sauf si elle revient avec un autre
/// jeton.
async fn decide(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    Json(payload): Json<DecidePayload>,
) -> ApiResult<Json<Decided>> {
    if payload.ids.is_empty() {
        return Err(ApiError::BadRequest("Select at least one machine.".into()));
    }
    let max = usize::try_from(enrolment::MAX_PENDING).unwrap_or(usize::MAX);
    if payload.ids.len() > max {
        return Err(ApiError::BadRequest(format!("At most {max} machines at once.")));
    }
    let who = me.label();
    let now = chrono::Utc::now();
    let decided = enrolment::decide(&state.pool, &payload.ids, payload.approve, &who, now).await?;
    let action =
        if payload.approve { "agent_enrolment.approved" } else { "agent_enrolment.rejected" };
    let subject = format!("{decided} machine(s)");
    audit::record(&state.pool, Some(&who), action, Some(&subject), ip).await;
    Ok(Json(Decided { decided }))
}

/// `POST /api/agent/tokens/{id}/reactivate` : remet en service un jeton de
/// flotte en veille. Son délai d'inactivité repart de maintenant.
async fn reactivate(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if !enrolment::reactivate(&state.pool, id, chrono::Utc::now()).await? {
        return Err(ApiError::NotFound(format!("Token {id} not found, revoked or not asleep.")));
    }
    let subject = id.to_string();
    audit::record(&state.pool, Some(&me.label()), "agent_token.reactivated", Some(&subject), ip)
        .await;
    Ok(StatusCode::NO_CONTENT)
}
