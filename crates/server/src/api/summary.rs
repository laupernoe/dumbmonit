//! Résumé de l'état de l'instance pour les tableaux de bord tiers.
//!
//! `GET /api/summary` rend des compteurs plats (équipements joignables ou non,
//! alertes par gravité) et un verdict global, dans une forme que le widget
//! `customapi` de Homepage lit par chemin (`devices.down`, `alerts.firing`).
//! Un jeton d'API de portée `read` suffit : il se crée dans Réglages → Jetons
//! d'API, et ne peut rien faire d'autre que lire. Même source que l'outil
//! `get_status` du serveur MCP, donc mêmes chiffres que l'interface.

use axum::Json;
use axum::extract::State;
use serde_json::Value;

use crate::api::mcp::{ToolError, dashboard_summary};
use crate::api::{ApiError, ApiResult};
use crate::state::AppState;

pub async fn get(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    match dashboard_summary(&state).await {
        Ok(summary) => Ok(Json(summary)),
        Err(ToolError::Internal(error)) => Err(ApiError::Internal(error)),
        Err(ToolError::Failed(why)) => Err(ApiError::BadGateway(why)),
    }
}
