//! Changements de configuration à faire après une mise à jour
//! (`crate::upgrade_notes`).
//!
//! * `GET /upgrade-notes` — les notes du catalogue et leur état pour cette
//!   instance ; `?fresh=true` refait les vérifications sans attendre.
//! * `POST /upgrade-notes/close` — ferme des notes (toutes celles à traiter
//!   si `ids` est vide) : elles ne reviennent plus pour leur version.
//! * `POST /upgrade-notes/announce` — note que la fenêtre a été montrée pour
//!   la version courante.
//!
//! Administrateur et session seulement : un jeton d'API n'a pas à fermer le
//! rappel d'un changement de déploiement.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminUser;
use crate::state::AppState;
use crate::upgrade_notes::{self, CATALOG, Overview};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/upgrade-notes", get(list))
        .route("/upgrade-notes/close", post(close))
        .route("/upgrade-notes/announce", post(announce))
}

/// Version exécutée.
const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// Adresse par laquelle l'administrateur joint le serveur, d'après `Host`.
fn request_host(headers: &HeaderMap) -> Option<&str> {
    let host = headers.get(header::HOST)?.to_str().ok()?;
    // Le port de l'interface ne dit rien de celui des agents.
    match host.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => Some(name),
        _ => Some(host),
    }
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    /// Refait les vérifications maintenant plutôt que d'attendre la mémoire d'une minute.
    #[serde(default)]
    fresh: bool,
}

/// `GET /api/upgrade-notes`
async fn list(
    _: AdminUser,
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<Overview>> {
    Ok(Json(upgrade_notes::overview(&state, CURRENT, request_host(&headers), query.fresh).await?))
}

#[derive(Debug, Default, Deserialize)]
struct ClosePayload {
    /// Notes à fermer. Vide : toutes celles qui sont à traiter.
    #[serde(default)]
    ids: Vec<String>,
}

/// `POST /api/upgrade-notes/close`
async fn close(
    AdminUser(me): AdminUser,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ClosePayload>,
) -> ApiResult<Json<Overview>> {
    let host = request_host(&headers);
    let mut notes = Vec::new();
    if payload.ids.is_empty() {
        let current = upgrade_notes::overview(&state, CURRENT, host, false).await?;
        let pending = current.pending_ids();
        notes.extend(CATALOG.iter().filter(|note| pending.contains(&note.id)));
    } else {
        for id in &payload.ids {
            let unknown = || ApiError::BadRequest(format!("Unknown change \"{id}\"."));
            notes.push(CATALOG.iter().find(|note| note.id == id).ok_or_else(unknown)?);
        }
    }
    for note in notes {
        if upgrade_notes::close_note(&state.pool, note, &me.username).await? {
            let actor = Some(me.username.as_str());
            audit::record(&state.pool, actor, "upgrade_notes.closed", Some(note.id), ip).await;
        }
    }
    Ok(Json(upgrade_notes::overview(&state, CURRENT, host, false).await?))
}

/// `POST /api/upgrade-notes/announce`
async fn announce(_: AdminUser, State(state): State<AppState>) -> ApiResult<StatusCode> {
    upgrade_notes::mark_announced(&state.pool, CURRENT).await?;
    Ok(StatusCode::NO_CONTENT)
}
