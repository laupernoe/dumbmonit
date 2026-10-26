//! Mode multi-clients (MSP) : `/api/clients`.
//!
//! Administration réservée à l'administrateur MSP connecté (extracteur
//! [`AdminUser`]) ; les jetons d'API n'y ont pas accès (`TOKEN_DENIED`). Seul
//! le logo d'un client se lit aussi par ses membres (route ouverte aux comptes
//! restreints, voir `teams::SCOPED_ROUTES`), pour l'en-tête de l'interface.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::status_pages::branding;
use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::clients::{
    self, Client, ClientError, ClientHealth, ClientInput, Kind, MspSettings,
};
use crate::auth::middleware::{AdminUser, Identity};
use crate::auth::teams::TeamAccess;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/clients", get(list).post(create))
        .route("/clients/settings", put(put_settings))
        .route("/clients/overview", get(overview))
        .route("/clients/assignments", get(assignments))
        .route("/clients/assign", put(assign))
        .route("/clients/{id}", put(update).delete(remove))
        .route("/clients/{id}/logo", get(logo).put(upload_logo).delete(delete_logo))
}

/// Erreur du module des clients, pour l'API.
pub(crate) fn api_error(error: ClientError) -> ApiError {
    match error {
        ClientError::Invalid(why) => ApiError::BadRequest(why),
        ClientError::Conflict(why) => ApiError::Conflict(why),
        ClientError::NotFound => ApiError::NotFound("Not found.".into()),
        ClientError::Internal(error) => ApiError::Internal(error),
    }
}

#[derive(Debug, Serialize)]
pub struct ClientsView {
    pub settings: MspSettings,
    pub clients: Vec<Client>,
    /// Comptes non administrateurs membres d'aucun client : mode activé, ils
    /// ne voient rien.
    pub unassigned_users: Vec<i64>,
}

/// `GET /api/clients`
pub async fn list(_: AdminUser, State(state): State<AppState>) -> ApiResult<Json<ClientsView>> {
    Ok(Json(ClientsView {
        settings: clients::settings(&state.pool).await?,
        clients: clients::list(&state.pool).await?,
        unassigned_users: clients::unassigned_users(&state.pool).await?,
    }))
}

async fn load(state: &AppState, id: i64) -> ApiResult<Client> {
    clients::get(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Client not found.".into()))
}

/// `POST /api/clients`
pub async fn create(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<ClientInput>,
) -> ApiResult<(StatusCode, Json<Client>)> {
    let id = clients::create(&state.pool, &input).await.map_err(api_error)?;
    let client = load(&state, id).await?;
    audit::record(&state.pool, Some(&me.username), "client.created", Some(&client.name), ip).await;
    Ok((StatusCode::CREATED, Json(client)))
}

/// `PUT /api/clients/{id}`
pub async fn update(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(input): Json<ClientInput>,
) -> ApiResult<Json<Client>> {
    clients::update(&state.pool, id, &input).await.map_err(|error| match error {
        ClientError::NotFound => ApiError::NotFound("Client not found.".into()),
        other => api_error(other),
    })?;
    let client = load(&state, id).await?;
    audit::record(&state.pool, Some(&me.username), "client.updated", Some(&client.name), ip).await;
    Ok(Json(client))
}

/// `DELETE /api/clients/{id}`
pub async fn remove(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let client = load(&state, id).await?;
    clients::delete(&state.pool, id).await.map_err(api_error)?;
    audit::record(&state.pool, Some(&me.username), "client.deleted", Some(&client.name), ip).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SettingsInput {
    pub enabled: bool,
}

/// `PUT /api/clients/settings` — active ou coupe le mode MSP.
pub async fn put_settings(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<SettingsInput>,
) -> ApiResult<Json<MspSettings>> {
    let settings = MspSettings { enabled: input.enabled };
    clients::save_settings(&state.pool, &settings).await?;
    let value = if settings.enabled { "enabled" } else { "disabled" };
    audit::record(&state.pool, Some(&me.username), "client.settings", Some(value), ip).await;
    Ok(Json(settings))
}

/// `GET /api/clients/overview` — l'état de chaque client, pour la vue
/// d'ensemble de l'administrateur MSP.
pub async fn overview(
    _: AdminUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ClientHealth>>> {
    Ok(Json(clients::overview(&state.pool).await?))
}

/// `GET /api/clients/assignments` — le client de chaque objet qui en a un, par
/// sorte (`targets`, `status_pages`, `agent_tokens`, `channels`, `rules`,
/// `silences`, `reports`). Absent : à l'instance.
pub async fn assignments(
    _: AdminUser,
    State(state): State<AppState>,
) -> ApiResult<Json<BTreeMap<&'static str, BTreeMap<i64, i64>>>> {
    let mut out = BTreeMap::new();
    for kind in Kind::ALL {
        let owners = clients::owners(&state.pool, kind).await?;
        out.insert(kind.key(), owners.into_iter().collect());
    }
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
pub struct AssignInput {
    pub kind: Kind,
    pub ids: Vec<i64>,
    /// `null` : à l'instance (canaux, règles, silences, rapports, pages).
    pub client_id: Option<i64>,
}

/// `PUT /api/clients/assign` — rattache des objets à un client. Déplacer un
/// équipement emporte ce qui en dépend et coupe les liens vers les autres
/// clients (voir [`clients::move_targets`]).
pub async fn assign(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<AssignInput>,
) -> ApiResult<StatusCode> {
    clients::assign(&state.pool, input.kind, &input.ids, input.client_id)
        .await
        .map_err(api_error)?;
    let subject = format!(
        "{} {:?} -> {}",
        input.kind.key(),
        input.ids,
        input.client_id.map_or_else(|| "instance".to_string(), |id| id.to_string())
    );
    audit::record(&state.pool, Some(&me.username), "client.assigned", Some(&subject), ip).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/clients/{id}/logo` — pour l'administrateur MSP et les membres du
/// client. Un autre compte reçoit 404, comme pour un client inexistant.
pub async fn logo(
    Identity(who): Identity,
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Response> {
    let allowed = who.is_admin() || access.clients().is_some_and(|mine| mine.contains(&id));
    let not_found = || ApiError::NotFound("This client has no logo.".into());
    if !allowed {
        return Err(not_found());
    }
    let (kind, bytes) = clients::logo(&state.pool, id).await?.ok_or_else(not_found)?;
    branding::serve_bytes(bytes, &kind, "private, no-cache")
}

#[derive(Debug, Deserialize)]
pub struct LogoPayload {
    /// Contenu du fichier en base64 (avec ou sans préfixe `data:`).
    pub data: String,
}

/// `PUT /api/clients/{id}/logo` — PNG, JPEG ou WebP reconnu à sa signature
/// (jamais de SVG, qui peut porter du script).
pub async fn upload_logo(
    AdminUser(me): AdminUser,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<LogoPayload>,
) -> ApiResult<StatusCode> {
    let client = load(&state, id).await?;
    let bytes = branding::decode(&payload.data)?;
    if bytes.is_empty() {
        return Err(ApiError::BadRequest("The logo file is empty.".into()));
    }
    if bytes.len() > clients::MAX_LOGO_BYTES {
        return Err(ApiError::BadRequest(format!(
            "The logo is limited to {} KiB.",
            clients::MAX_LOGO_BYTES / 1024
        )));
    }
    let Some(kind) = branding::sniff(&bytes) else {
        return Err(ApiError::BadRequest("The logo must be a PNG, JPEG or WebP image.".into()));
    };
    clients::set_logo(&state.pool, id, Some((kind, &bytes))).await?;
    audit::record(&state.pool, Some(&me.username), "client.logo", Some(&client.name), ip).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/clients/{id}/logo`
pub async fn delete_logo(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if !clients::set_logo(&state.pool, id, None).await? {
        return Err(ApiError::NotFound("Client not found.".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
