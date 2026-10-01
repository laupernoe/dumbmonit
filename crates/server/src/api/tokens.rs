//! Gestion des jetons d'API depuis l'interface : liste, création, révocation.
//!
//! Ce sont des routes d'administration ordinaires, protégées par la session et
//! fermées à tout jeton (voir `middleware::TOKEN_DENIED`) : un jeton ne fabrique
//! jamais un autre jeton, et donc jamais un plus puissant que lui. Le jeton
//! lui-même n'est présenté qu'aux routes de l'API et au serveur MCP.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::Authenticated;
use crate::auth::token::{self as token, MAX_EXPIRY_DAYS, NewToken, Scope, TokenRecord};
use crate::state::AppState;

/// Longueur maximale d'un nom de jeton : il s'affiche dans une liste et dans les
/// journaux, il n'a pas à être un paragraphe.
const MAX_NAME_LEN: usize = 80;

/// Réponse à la création : le seul moment où le secret est visible.
#[derive(Debug, Serialize)]
pub struct CreatedToken {
    #[serde(flatten)]
    pub token: TokenRecord,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenPayload {
    pub name: String,
    /// `read` par défaut : c'est le choix sûr, et celui que l'interface présélectionne.
    #[serde(default)]
    pub scope: Option<String>,
    /// Durée de validité en jours ; absent ou `null` : le jeton n'expire pas.
    #[serde(default)]
    pub expires_in_days: Option<i64>,
    /// Réseaux d'où le jeton sera accepté (CIDR ou adresse seule) ; vide : partout.
    #[serde(default)]
    pub allowed_networks: Vec<String>,
}

pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vec<TokenRecord>>> {
    Ok(Json(token::list(&state.pool).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Authenticated(me): Authenticated,
    ClientIp(ip): ClientIp,
    Json(payload): Json<TokenPayload>,
) -> ApiResult<(StatusCode, Json<CreatedToken>)> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Token name is required.".into()));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(ApiError::BadRequest(format!(
            "Token name is limited to {MAX_NAME_LEN} characters."
        )));
    }
    let scope = match payload.scope.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Scope::Read,
        Some(raw) => Scope::parse(raw).ok_or_else(|| {
            ApiError::BadRequest(format!("Unknown scope \"{raw}\" (expected: read, write)."))
        })?,
    };
    let expires_at = match payload.expires_in_days {
        None => None,
        Some(days) if (1..=MAX_EXPIRY_DAYS).contains(&days) => {
            Some(Utc::now() + TimeDelta::days(days))
        }
        Some(_) => {
            return Err(ApiError::BadRequest(format!(
                "\"expires_in_days\" must be between 1 and {MAX_EXPIRY_DAYS}, or null for a token \
                 that never expires."
            )));
        }
    };
    let allowed_networks =
        token::parse_networks(&payload.allowed_networks).map_err(ApiError::BadRequest)?;

    let new = NewToken { name, scope, user_id: Some(me.id), expires_at, allowed_networks };
    let (record, secret) = token::create(&state.pool, &new).await?;
    tracing::info!(
        token = %record.name,
        token_id = record.id,
        scope = scope.as_str(),
        expires_at = record.expires_at.as_deref().unwrap_or("never"),
        networks = record.allowed_networks.len(),
        by = %me.username,
        "API token created"
    );
    audit::record(&state.pool, Some(&me.username), "token.created", Some(&record.name), ip).await;
    Ok((StatusCode::CREATED, Json(CreatedToken { token: record, secret })))
}

pub async fn revoke(
    State(state): State<AppState>,
    Authenticated(me): Authenticated,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if token::revoke(&state.pool, id).await? {
        tracing::info!(token_id = id, by = %me.username, "API token revoked");
        audit::record(&state.pool, Some(&me.username), "token.revoked", Some(&id.to_string()), ip)
            .await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Token {id} not found or already revoked.")))
    }
}
