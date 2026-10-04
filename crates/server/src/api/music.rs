//! Routes de la musique du mode mur (`crate::music`).
//!
//! - `GET /api/music/now` : ce que les murs interrogent toutes les cinq
//!   secondes — la lecture en cours sur le compte Spotify relié (quel que soit
//!   l'appareil qui joue) et le lien partagé à jouer.
//! - `PUT|DELETE /api/music/link` : « Play on the wall » depuis n'importe quel
//!   écran, pour tous les murs.
//! - `GET|DELETE /api/music/spotify`, `POST …/authorize`, `POST …/complete`,
//!   `GET …/callback` : relier et délier le compte (administrateurs).
//! - `GET /api/music/spotify/token` : un jeton d'accès court pour le Web
//!   Playback SDK du mur. Réservé à une session de navigateur — jamais à un
//!   jeton d'API — car c'est la seule chose qui sorte du serveur vers Spotify.
//! - `GET /api/music/speaker`, `PUT /api/music/speaker` : l'enceinte vue des
//!   réglages (compte, Premium, appareils listés par Spotify, rapports des
//!   murs) et son nom.
//! - `POST /api/music/speaker/report` : un mur dit où en est son enceinte, et
//!   apprend si Spotify la liste bien.
//! - `POST /api/music/speaker/play` : « Play here » sur le mur, « Test sound »
//!   dans les réglages — le compte se met à jouer sur l'enceinte. Session de
//!   navigateur seulement, lecteurs compris : c'est ce que permet déjà le jeton
//!   que reçoit le mur.
//!
//! Le jeton de rafraîchissement, lui, ne quitte jamais le serveur.

use axum::extract::{Extension, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{AdminIdentity, AdminUser, Authenticated, Identity};
use crate::music::spotify::{self as spotify_api, SpotifyError};
use crate::music::store::{self, WallLink};
use crate::music::{
    self, CallbackParams, MusicError, MusicHub, SpeakerReport, SpeakerStatus, SpotifyNow,
    SpotifyView,
};
use crate::state::AppState;

/// Routes sous le garde de session (les écritures y exigent un administrateur).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/music/now", get(now))
        .route("/music/link", put(set_link).delete(clear_link))
        .route("/music/spotify", get(spotify_status).delete(disconnect))
        .route("/music/spotify/authorize", post(authorize))
        .route("/music/spotify/complete", post(complete))
        .route("/music/spotify/token", get(sdk_token))
        .route("/music/speaker", get(speaker_status).put(set_speaker_name))
        .route("/music/speaker/report", post(speaker_report))
        .route("/music/speaker/play", post(speaker_play))
}

/// Le retour direct de Spotify : une navigation du navigateur, sans l'en-tête
/// anti-CSRF qu'aucune redirection ne porte. Protégée par le `state` à usage
/// unique et par la session du compte qui a commencé l'autorisation.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/music/spotify/callback", get(callback))
}

// ------------------------------------------------------------------- le mur

#[derive(Debug, Serialize)]
pub struct WallMusic {
    pub spotify: SpotifyNow,
    pub link: Option<WallLink>,
    /// Nom de l'appareil Spotify Connect que le mur annonce.
    pub speaker_name: String,
}

/// `GET /api/music/now`.
async fn now(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    Identity(_): Identity,
) -> ApiResult<Response> {
    let spotify = hub.now(&state.pool, &state.cipher).await;
    let link = store::link(&state.pool).await?;
    let speaker_name = store::speaker_name(&state.pool).await?;
    Ok(no_store(Json(WallMusic { spotify, link, speaker_name })))
}

#[derive(Debug, Deserialize)]
struct LinkPayload {
    link: String,
}

/// `PUT /api/music/link` — tous les murs joueront ce lien.
async fn set_link(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    Json(payload): Json<LinkPayload>,
) -> ApiResult<Json<WallLink>> {
    let link =
        music::link::validate(&payload.link).map_err(|why| ApiError::BadRequest(why.into()))?;
    Ok(Json(store::set_link(&state.pool, &link, &principal.label()).await?))
}

/// `DELETE /api/music/link` — les murs arrêtent le lecteur intégré.
async fn clear_link(
    State(state): State<AppState>,
    AdminIdentity(_): AdminIdentity,
) -> ApiResult<StatusCode> {
    store::clear_link(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/music/spotify/token` — pour le Web Playback SDK du mur.
///
/// `Authenticated` n'existe que pour une session : un jeton d'API reçoit 401.
/// Lecteurs compris — un écran mural tourne volontiers sous un compte lecteur.
async fn sdk_token(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    Authenticated(_): Authenticated,
) -> Response {
    match hub.access_token(&state.pool, &state.cipher).await {
        Ok(token) => no_store(Json(json!({
            "access_token": token.token,
            "expires_in": token.expires_in().as_secs(),
        }))),
        Err(error) => music_error(error),
    }
}

// ---------------------------------------------------------------- l'enceinte

/// `GET /api/music/speaker` — l'enceinte vue des réglages.
async fn speaker_status(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    Identity(_): Identity,
) -> ApiResult<Response> {
    let status: SpeakerStatus = hub.speaker_status(&state.pool, &state.cipher).await?;
    Ok(no_store(Json(status)))
}

#[derive(Debug, Deserialize)]
struct SpeakerNamePayload {
    /// `null` (ou vide) : revenir au nom par défaut.
    name: Option<String>,
}

/// `PUT /api/music/speaker` — renomme l'enceinte de tous les murs.
async fn set_speaker_name(
    State(state): State<AppState>,
    AdminIdentity(_): AdminIdentity,
    Json(payload): Json<SpeakerNamePayload>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = match payload.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        None => None,
        Some(name) => Some(
            spotify_api::clean_speaker_name(name)
                .map_err(|why| ApiError::BadRequest(why.into()))?,
        ),
    };
    let name = name.filter(|n| n != spotify_api::SPEAKER_NAME);
    store::set_speaker_name(&state.pool, name.as_deref()).await?;
    Ok(Json(json!({ "speaker_name": store::speaker_name(&state.pool).await? })))
}

/// `POST /api/music/speaker/report` — un mur dit où en est son enceinte.
///
/// Session de navigateur seulement : ce sont les murs qui parlent ici.
async fn speaker_report(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    Authenticated(_): Authenticated,
    Json(report): Json<SpeakerReport>,
) -> ApiResult<Response> {
    let report = report.check().map_err(|why| ApiError::BadRequest(why.into()))?;
    let listed = hub.report(&state.pool, &state.cipher, report).await;
    Ok(no_store(Json(json!({ "listed": listed }))))
}

#[derive(Debug, Default, Deserialize)]
struct PlayPayload {
    /// L'appareil du mur ; absent, celui qui porte le nom de l'enceinte.
    #[serde(default)]
    device_id: Option<String>,
}

/// `POST /api/music/speaker/play` — le compte se met à jouer sur l'enceinte.
async fn speaker_play(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    Authenticated(_): Authenticated,
    Json(payload): Json<PlayPayload>,
) -> Response {
    let name = match store::speaker_name(&state.pool).await {
        Ok(name) => name,
        Err(error) => return ApiError::Internal(error).into_response(),
    };
    let requested = payload.device_id.filter(|id| {
        (1..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric())
    });
    let device = match requested {
        Some(id) => id,
        None => match hub.speaker_device(&state.pool, &state.cipher, &name).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                let message = format!(
                    "Spotify does not list “{name}” right now. Open the wall on the display, \
                     check that it says the speaker is ready, then try again."
                );
                return no_store((StatusCode::CONFLICT, Json(json!({ "error": message }))));
            }
            Err(error) => return music_error(error),
        },
    };
    match hub.transfer(&state.pool, &state.cipher, &device).await {
        Ok(()) => no_store(Json(json!({ "device_id": device, "speaker_name": name }))),
        Err(error) => music_error(error),
    }
}

// ----------------------------------------------------------------- réglages

/// `GET /api/music/spotify` — l'état du compte relié, sans aucun jeton.
async fn spotify_status(
    State(state): State<AppState>,
    Identity(_): Identity,
) -> ApiResult<Json<SpotifyView>> {
    Ok(Json(SpotifyView::load(&state.pool).await?))
}

#[derive(Debug, Deserialize)]
struct AuthorizePayload {
    client_id: String,
    redirect_uri: String,
}

#[derive(Debug, Serialize)]
struct AuthorizeReply {
    /// Adresse d'approbation chez Spotify, à ouvrir dans le navigateur.
    authorize_url: String,
    redirect_uri: String,
    /// Secondes pendant lesquelles l'approbation peut revenir.
    expires_in: u64,
}

/// `POST /api/music/spotify/authorize` — commence une connexion.
///
/// Réservé à un administrateur connecté par session : l'autorisation est liée à
/// son compte, et seul ce compte pourra la terminer.
async fn authorize(
    Extension(hub): Extension<MusicHub>,
    AdminUser(user): AdminUser,
    headers: HeaderMap,
    Json(payload): Json<AuthorizePayload>,
) -> ApiResult<Json<AuthorizeReply>> {
    let client_id = payload.client_id.trim();
    let redirect_uri = payload.redirect_uri.trim();
    music::check_client_id(client_id).map_err(|why| ApiError::BadRequest(why.into()))?;
    let origin = headers.get(header::ORIGIN).and_then(|value| value.to_str().ok());
    music::check_redirect_uri(redirect_uri, origin)
        .map_err(|why| ApiError::BadRequest(why.into()))?;
    let authorize_url = hub.start(client_id, redirect_uri, user.id);
    Ok(Json(AuthorizeReply {
        authorize_url,
        redirect_uri: redirect_uri.to_string(),
        expires_in: 600,
    }))
}

#[derive(Debug, Deserialize)]
struct CompletePayload {
    /// L'adresse de la page sur laquelle Spotify a renvoyé le navigateur.
    url: String,
}

/// `POST /api/music/spotify/complete` — termine une connexion par
/// l'adresse de bouclage, recollée par l'administrateur.
async fn complete(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    AdminUser(user): AdminUser,
    ClientIp(ip): ClientIp,
    Json(payload): Json<CompletePayload>,
) -> ApiResult<Json<SpotifyView>> {
    let params = music::parse_pasted(&payload.url).ok_or_else(|| {
        ApiError::BadRequest(
            "Paste the whole address of the page Spotify sent you to: it starts with \
             http://127.0.0.1:8888/callback?code="
                .into(),
        )
    })?;
    match hub.finish(&state.pool, &state.cipher, params, user.id).await {
        Ok(()) => {
            audit::record(&state.pool, Some(&user.username), "spotify.connected", None, ip).await;
            Ok(Json(SpotifyView::load(&state.pool).await?))
        }
        Err(music::ConnectError::Internal(error)) => Err(ApiError::Internal(error)),
        Err(error) => Err(ApiError::BadRequest(error.message())),
    }
}

/// `GET /api/music/spotify/callback` — retour direct (DumbMonit en HTTPS).
async fn callback(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Query(params): Query<CallbackParams>,
) -> Response {
    // La session qui revient doit être celle qui est partie : le `state` est
    // lié à ce compte, et un administrateur seulement peut relier Spotify.
    let user = match crate::auth::middleware::current_session(&state.pool, &headers).await {
        Ok(Some((_, user))) if user.role.is_admin() => user,
        Ok(_) => return back_to_settings("session"),
        Err(_) => return back_to_settings("internal"),
    };
    match hub.finish(&state.pool, &state.cipher, params, user.id).await {
        Ok(()) => {
            audit::record(&state.pool, Some(&user.username), "spotify.connected", None, ip).await;
            back_to_settings("connected")
        }
        Err(error) => {
            match &error {
                music::ConnectError::Internal(detail) => {
                    tracing::error!(detail = %format!("{detail:#}"), "Spotify connection failed")
                }
                other => tracing::warn!(reason = other.reason(), "Spotify connection failed"),
            }
            back_to_settings(error.reason())
        }
    }
}

/// Retour à la section des réglages, avec l'issue dans l'URL (`?spotify=`).
fn back_to_settings(outcome: &str) -> Response {
    Redirect::to(&format!("/settings?spotify={outcome}#music")).into_response()
}

/// `DELETE /api/music/spotify` — délie le compte et oublie ses jetons.
async fn disconnect(
    State(state): State<AppState>,
    Extension(hub): Extension<MusicHub>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
) -> ApiResult<StatusCode> {
    store::delete_account(&state.pool).await?;
    store::set_product(&state.pool, None).await?;
    hub.forget().await;
    let actor = principal.label();
    audit::record(&state.pool, Some(&actor), "spotify.disconnected", None, ip).await;
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------------ outils

/// Une réponse qu'aucun cache ne garde : elle porte un jeton, ou un état qui
/// change toutes les secondes.
fn no_store(body: impl IntoResponse) -> Response {
    let mut response = body.into_response();
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Statut HTTP d'un échec Spotify : 409 quand c'est à l'administrateur d'agir
/// (relier ou relier à nouveau), 502/503 quand c'est Spotify qui ne suit pas.
fn music_error(error: MusicError) -> Response {
    let error = match error {
        MusicError::Spotify(error) => error,
        MusicError::Internal(error) => return ApiError::Internal(error).into_response(),
    };
    let status = match &error {
        SpotifyError::NotConnected | SpotifyError::Expired(_) => StatusCode::CONFLICT,
        SpotifyError::RateLimited(_) => StatusCode::SERVICE_UNAVAILABLE,
        SpotifyError::Rejected(_) | SpotifyError::Unreachable(_) => StatusCode::BAD_GATEWAY,
    };
    no_store((status, Json(json!({ "error": error.message() }))))
}
