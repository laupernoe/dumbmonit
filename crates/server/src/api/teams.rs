//! Administration des équipes : `/api/teams`.
//!
//! Réservée aux administrateurs connectés (extracteur [`AdminUser`]) ; les
//! jetons d'API n'y ont pas accès (`TOKEN_DENIED`) : décider de qui voit quoi
//! est un geste que l'on fait soi-même, dans l'interface.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminUser;
use crate::auth::teams::{self, Team, TeamError, TeamInput, TeamSettings, UnassignedAccess};
use crate::auth::{AuthError, AuthResult, audit};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/teams", get(list).post(create))
        .route("/teams/settings", put(put_settings))
        .route("/teams/{id}", put(update).delete(remove))
}

#[derive(Debug, Serialize)]
pub struct TeamsView {
    pub teams: Vec<Team>,
    /// Dossiers existants (sous-dossiers et leurs parents), pour la matrice.
    pub folders: Vec<String>,
    pub settings: TeamSettings,
}

fn team_error(error: TeamError) -> AuthError {
    match error {
        TeamError::Invalid(why) => AuthError::Invalid(why),
        TeamError::Conflict(why) => AuthError::Conflict(why),
        TeamError::NotFound => AuthError::NotFound("Team not found.".into()),
        TeamError::Internal(error) => AuthError::Internal(error),
    }
}

/// `GET /api/teams`
pub async fn list(_: AdminUser, State(state): State<AppState>) -> AuthResult<Json<TeamsView>> {
    Ok(Json(TeamsView {
        teams: teams::list(&state.pool).await?,
        folders: teams::folders(&state.pool).await?,
        settings: teams::settings(&state.pool).await?,
    }))
}

async fn load(state: &AppState, id: i64) -> AuthResult<Team> {
    teams::get(&state.pool, id).await?.ok_or_else(|| AuthError::NotFound("Team not found.".into()))
}

/// `POST /api/teams`
pub async fn create(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<TeamInput>,
) -> AuthResult<(StatusCode, Json<Team>)> {
    let id = teams::create(&state.pool, &input).await.map_err(team_error)?;
    let team = load(&state, id).await?;
    audit::record(&state.pool, Some(&me.username), "team.created", Some(&team.name), ip).await;
    Ok((StatusCode::CREATED, Json(team)))
}

/// `PUT /api/teams/{id}`
pub async fn update(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(input): Json<TeamInput>,
) -> AuthResult<Json<Team>> {
    teams::update(&state.pool, id, &input).await.map_err(team_error)?;
    let team = load(&state, id).await?;
    audit::record(&state.pool, Some(&me.username), "team.updated", Some(&team.name), ip).await;
    Ok(Json(team))
}

/// `DELETE /api/teams/{id}`
pub async fn remove(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AuthResult<StatusCode> {
    let team = load(&state, id).await?;
    if !teams::delete(&state.pool, id).await? {
        return Err(AuthError::NotFound("Team not found.".into()));
    }
    audit::record(&state.pool, Some(&me.username), "team.deleted", Some(&team.name), ip).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SettingsInput {
    pub unassigned_access: UnassignedAccess,
}

/// `PUT /api/teams/settings`
pub async fn put_settings(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<SettingsInput>,
) -> AuthResult<Json<TeamSettings>> {
    let settings = TeamSettings { unassigned_access: input.unassigned_access };
    teams::save_settings(&state.pool, &settings).await?;
    let value = match settings.unassigned_access {
        UnassignedAccess::All => "all",
        UnassignedAccess::None => "none",
    };
    audit::record(&state.pool, Some(&me.username), "team.settings", Some(value), ip).await;
    Ok(Json(settings))
}
