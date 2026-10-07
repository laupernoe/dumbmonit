//! Routes des notifications Web Push d'un compte : clé publique VAPID,
//! abonnement de l'appareil courant, liste et suppression des appareils, envoi
//! de test.
//!
//! Tout se fait au nom du compte connecté, y compris pour un lecteur (`viewer`) :
//! recevoir les alertes sur son téléphone ne modifie rien de la supervision. Un
//! jeton d'API n'a pas accès à ces routes (l'extracteur [`Authenticated`] exige
//! une session) — un abonnement appartient à un navigateur, pas à un script.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::Authenticated;
use crate::notify::webpush::{self, Audience, Store, endpoint, store, vapid};
use crate::notify::{self, message};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/webpush", get(overview))
        .route("/webpush/subscriptions", post(subscribe))
        .route("/webpush/subscriptions/{id}", delete(unsubscribe))
        .route("/webpush/test", post(test))
}

/// Ce que la section « Push notifications » affiche.
#[derive(Debug, Serialize)]
pub struct Overview {
    /// Clé publique VAPID, en base64url : `applicationServerKey` de
    /// `pushManager.subscribe`. La clé privée ne sort jamais.
    pub public_key: String,
    pub devices: Vec<store::Device>,
}

/// `GET /api/webpush`
pub async fn overview(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> ApiResult<Json<Overview>> {
    let keys = vapid::load_or_create(&state.pool, &state.cipher).await?;
    let devices = store::list_for_user(&state.pool, user.id).await?;
    Ok(Json(Overview { public_key: keys.public_key().to_string(), devices }))
}

/// `PushSubscription.toJSON()` tel que le navigateur le produit.
#[derive(Debug, Deserialize)]
pub struct SubscriptionPayload {
    pub endpoint: String,
    pub keys: SubscriptionKeys,
}

#[derive(Debug, Deserialize)]
pub struct SubscriptionKeys {
    pub p256dh: String,
    pub auth: String,
}

/// Les navigateurs encodent en base64url sans remplissage ; certains anciens
/// le laissent. Les deux sont acceptés.
fn decode_key(value: &str, name: &str, len: usize) -> ApiResult<Vec<u8>> {
    let value = value.trim();
    let bytes = URL_SAFE_NO_PAD
        .decode(value.trim_end_matches('='))
        .or_else(|_| URL_SAFE.decode(value))
        .map_err(|_| ApiError::BadRequest(format!("\"keys.{name}\" is not base64url.")))?;
    if bytes.len() != len {
        return Err(ApiError::BadRequest(format!(
            "\"keys.{name}\" must be {len} bytes, got {}.",
            bytes.len()
        )));
    }
    Ok(bytes)
}

/// `POST /api/webpush/subscriptions` — abonne l'appareil courant.
pub async fn subscribe(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    headers: HeaderMap,
    Json(payload): Json<SubscriptionPayload>,
) -> ApiResult<(StatusCode, Json<store::Device>)> {
    let url = endpoint::validate(payload.endpoint.trim()).map_err(ApiError::BadRequest)?;
    endpoint::vet_resolution(&url).await.map_err(ApiError::BadRequest)?;

    let p256dh = decode_key(&payload.keys.p256dh, "p256dh", 65)?;
    if web_push_native::p256::PublicKey::from_sec1_bytes(&p256dh).is_err() {
        return Err(ApiError::BadRequest("\"keys.p256dh\" is not a P-256 public key.".into()));
    }
    let auth = decode_key(&payload.keys.auth, "auth", 16)?;

    let user_agent =
        headers.get(header::USER_AGENT).and_then(|value| value.to_str().ok()).unwrap_or("");
    let device = store::describe_user_agent(user_agent);
    let saved = store::upsert(
        &state.pool,
        &state.cipher,
        user.id,
        payload.endpoint.trim(),
        &p256dh,
        &auth,
        &device,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(saved)))
}

/// `DELETE /api/webpush/subscriptions/{id}` — oublie un appareil du compte.
pub async fn unsubscribe(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::delete_for_user(&state.pool, user.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Push subscription {id} not found.")))
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct TestPayload {
    /// Un seul appareil du compte ; absent : tous.
    #[serde(default)]
    pub id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TestReport {
    pub ok: bool,
    pub delivered: usize,
    pub failed: usize,
    pub removed: usize,
    pub message: String,
}

/// `POST /api/webpush/test` — envoie une notification de test aux appareils du
/// compte connecté.
pub async fn test(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    payload: Option<Json<TestPayload>>,
) -> ApiResult<Json<TestReport>> {
    let only = payload.and_then(|Json(payload)| payload.id);
    if notify::sending_disabled() {
        return Err(ApiError::Forbidden(notify::DISABLED_MESSAGE.into()));
    }
    let store = Store { pool: &state.pool, cipher: &state.cipher };
    let audience = Audience::User { user_id: user.id, subscription: only };
    let mut message = message::test_message("Web Push");
    message.title = "DumbMonit — test notification".into();
    message.text = "Push notifications reach this device. Tap to open the alerts.".into();
    let outcome = webpush::send(
        store,
        &webpush::client(),
        &audience,
        &webpush::Settings::for_test(),
        &message,
    )
    .await
    .map_err(|error| ApiError::Internal(error.into()))?;

    let message = match (outcome.delivered, outcome.failed, outcome.removed) {
        (0, 0, 0) => "No device of this account is subscribed yet.".to_string(),
        (0, 0, removed) => format!(
            "The subscription had expired and was removed ({removed}). Enable push again on \
             this device."
        ),
        (0, _, _) => format!(
            "The push service refused the message: {}",
            outcome.last_error.as_deref().unwrap_or("unknown error")
        ),
        (delivered, failed, _) if failed > 0 => {
            format!("Sent to {delivered} device(s), {failed} failed.")
        }
        (delivered, _, _) => format!("Sent to {delivered} device(s)."),
    };
    Ok(Json(TestReport {
        ok: outcome.delivered > 0,
        delivered: outcome.delivered,
        failed: outcome.failed,
        removed: outcome.removed,
        message,
    }))
}
