//! Clés d'accès (WebAuthn) : enregistrement, connexion, gestion dans « Mon compte ».
//!
//! - `POST /api/auth/passkey/login/begin` et `…/finish` (publiques) : connexion
//!   sans nom d'utilisateur ni mot de passe, par une clé découvrable. La
//!   vérification de l'utilisateur (code, biométrie) est exigée : la clé vaut à
//!   elle seule second facteur, le TOTP n'est pas redemandé ;
//! - `GET /api/auth/passkeys`, `POST …/register/begin` et `…/register/finish`,
//!   `PUT|DELETE …/{id}` : à la charge d'un compte connecté. Ajouter une clé
//!   redemande le mot de passe, comme l'enrôlement du TOTP — une session volée ne
//!   doit pas pouvoir installer sa propre clé.
//!
//! Réservées aux comptes locaux (avec mot de passe) : un compte d'annuaire ou de
//! SSO perdrait, avec une clé d'accès, la révocation que son fournisseur applique.

use std::time::Instant;

use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::auth::{guard_attempt, limiter_keys, open_session};
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::Authenticated;
use crate::auth::passkeys::{self, Ceremony, MAX_NAME_CHARS, MAX_PER_USER, NewPasskey};
use crate::auth::rate_limit::Key;
use crate::auth::users::{self, User};
use crate::auth::webauthn::{self, RelyingParty, base64url_decode, base64url_encode};
use crate::auth::{AuthError, AuthResult, AuthState, audit, password};
use crate::state::AppState;

/// Routes réservées à un compte connecté.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/passkeys", get(list))
        .route("/auth/passkeys/register/begin", post(register_begin))
        .route("/auth/passkeys/register/finish", post(register_finish))
        .route("/auth/passkeys/{id}", put(rename).delete(remove))
}

/// Routes publiques : la connexion elle-même.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/passkey/login/begin", post(login_begin))
        .route("/auth/passkey/login/finish", post(login_finish))
}

/// Délai annoncé au navigateur pour la cérémonie, en millisecondes.
const TIMEOUT_MS: u64 = 300_000;

async fn public_url(state: &AppState) -> Option<String> {
    let global = crate::notify::policy_store::load_global(&state.pool).await.ok()?;
    let env_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
    global.public_url(env_url.as_deref())
}

async fn relying_party(state: &AppState, headers: &HeaderMap) -> AuthResult<RelyingParty> {
    passkeys::relying_party(headers, public_url(state).await.as_deref())
}

/// Seaux du compteur de tentatives d'une connexion sans nom : l'adresse seule.
fn anonymous_keys(ip: Option<std::net::IpAddr>) -> Vec<Key> {
    match ip {
        Some(ip) => vec![Key::Ip(ip)],
        None => vec![Key::User("\u{0}passkey".to_string(), None)],
    }
}

fn clean_name(raw: &str) -> String {
    let name: String = raw.trim().chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect();
    if name.is_empty() { "Passkey".to_string() } else { name }
}

fn local_only(user: &User) -> AuthResult<()> {
    if user.password_hash.is_none() {
        return Err(AuthError::Conflict(
            "Passkeys are for local accounts: this account signs in through its identity \
             provider."
                .into(),
        ));
    }
    Ok(())
}

/// `GET /api/auth/passkeys`
pub async fn list(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> AuthResult<Json<Vec<passkeys::PasskeyView>>> {
    Ok(Json(passkeys::list(&state.pool, user.id).await?))
}

#[derive(Deserialize)]
pub struct RegisterBegin {
    password: String,
}

impl std::fmt::Debug for RegisterBegin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegisterBegin { password: <redacted> }")
    }
}

/// `POST /api/auth/passkeys/register/begin` — options pour `navigator.credentials.create()`.
pub async fn register_begin(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    Authenticated(user): Authenticated,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Json(payload): Json<RegisterBegin>,
) -> AuthResult<Json<Value>> {
    local_only(&user)?;
    let rp = relying_party(&state, &headers).await?;
    if passkeys::count(&state.pool, Some(user.id)).await? >= MAX_PER_USER {
        return Err(AuthError::Conflict(format!(
            "At most {MAX_PER_USER} passkeys per account: remove one first."
        )));
    }
    // Même garde que l'enrôlement du TOTP.
    let stored = user.password_hash.clone().unwrap_or_default();
    let keys = limiter_keys(ip, Some(&user.username));
    guard_attempt(&auth, &keys).await?;
    if !password::verify(payload.password, stored).await? {
        auth.limiter().lock().await.record_failure(&keys, Instant::now());
        return Err(AuthError::Unauthorized("Wrong password.".into()));
    }
    auth.limiter().lock().await.record_success(&keys);

    let existing: Vec<Value> = passkeys::credential_ids(&state.pool, user.id)
        .await?
        .iter()
        .map(|id| json!({ "type": "public-key", "id": base64url_encode(id) }))
        .collect();
    let (challenge_id, challenge) = passkeys::start(
        Ceremony::Register { user_id: user.id },
        rp.clone(),
    );
    Ok(Json(json!({
        "challenge_id": challenge_id,
        "options": {
            "rp": { "id": rp.id, "name": "DumbMonit" },
            "user": {
                "id": base64url_encode(user.id.to_string().as_bytes()),
                "name": user.username,
                "displayName": user.label(),
            },
            "challenge": base64url_encode(&challenge),
            // ES256 seulement : voir `auth::webauthn`.
            "pubKeyCredParams": [{ "type": "public-key", "alg": -7 }],
            "timeout": TIMEOUT_MS,
            "attestation": "none",
            "excludeCredentials": existing,
            "authenticatorSelection": {
                "residentKey": "required",
                "requireResidentKey": true,
                "userVerification": "required",
            },
        },
    })))
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinish {
    challenge_id: String,
    #[serde(default)]
    name: String,
    credential: RegistrationCredential,
}

#[derive(Debug, Deserialize)]
pub struct RegistrationCredential {
    id: String,
    response: RegistrationResponse,
    #[serde(default)]
    transports: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationResponse {
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    attestation_object: String,
}

fn refused(reason: impl std::fmt::Display) -> AuthError {
    tracing::warn!(%reason, "passkey ceremony refused");
    AuthError::Unauthorized("The passkey could not be verified. Start again.".into())
}

fn decode(text: &str) -> AuthResult<Vec<u8>> {
    base64url_decode(text).ok_or_else(|| AuthError::Invalid("Unreadable passkey response.".into()))
}

/// `POST /api/auth/passkeys/register/finish` — vérifie la réponse et range la clé.
pub async fn register_finish(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ClientIp(ip): ClientIp,
    Json(payload): Json<RegisterFinish>,
) -> AuthResult<Json<passkeys::PasskeyView>> {
    local_only(&user)?;
    let Some(pending) = passkeys::take(&payload.challenge_id) else {
        return Err(AuthError::Unauthorized("This request has expired. Start again.".into()));
    };
    match &pending.ceremony {
        Ceremony::Register { user_id } if *user_id == user.id => {}
        _ => return Err(AuthError::Unauthorized("This request has expired. Start again.".into())),
    }
    let client_data = decode(&payload.credential.response.client_data_json)?;
    let attestation = decode(&payload.credential.response.attestation_object)?;
    let verified =
        webauthn::verify_registration(&pending.rp, &pending.challenge, &client_data, &attestation)
            .map_err(refused)?;
    // L'identifiant annoncé par le navigateur est celui de l'attestation.
    if base64url_decode(&payload.credential.id).as_deref() != Some(verified.credential_id.as_slice())
    {
        return Err(refused("credential id mismatch"));
    }

    let name = clean_name(&payload.name);
    let transports: Vec<String> = payload
        .credential
        .transports
        .into_iter()
        .filter(|t| t.len() <= 20 && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .take(8)
        .collect();
    let stored = passkeys::insert(
        &state.pool,
        &NewPasskey {
            user_id: user.id,
            name: &name,
            credential_id: &verified.credential_id,
            public_key: &verified.public_key,
            sign_count: verified.sign_count,
            backup_eligible: verified.backup_eligible,
            backup_state: verified.backup_state,
            transports: &transports,
        },
    )
    .await?;
    if !stored {
        return Err(AuthError::Conflict("This passkey is already registered.".into()));
    }
    audit::record(&state.pool, Some(&user.username), "passkey.added", Some(name.as_str()), ip).await;
    tracing::info!(user = %user.username, "passkey registered");
    let added = passkeys::list(&state.pool, user.id)
        .await?
        .into_iter()
        .next_back()
        .ok_or_else(|| AuthError::Internal(anyhow::anyhow!("passkey not found after insert")))?;
    Ok(Json(added))
}

#[derive(Debug, Deserialize)]
pub struct RenamePayload {
    name: String,
}

/// `PUT /api/auth/passkeys/{id}` — renomme une de ses clés.
pub async fn rename(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<RenamePayload>,
) -> AuthResult<StatusCode> {
    if !passkeys::rename(&state.pool, user.id, id, &clean_name(&payload.name)).await? {
        return Err(AuthError::NotFound("No such passkey.".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/auth/passkeys/{id}` — retire une de ses clés.
pub async fn remove(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AuthResult<StatusCode> {
    if !passkeys::delete(&state.pool, user.id, id).await? {
        return Err(AuthError::NotFound("No such passkey.".into()));
    }
    audit::record(&state.pool, Some(&user.username), "passkey.removed", None, ip).await;
    tracing::info!(user = %user.username, "passkey removed");
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/auth/passkey/login/begin` — options pour `navigator.credentials.get()`.
pub async fn login_begin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AuthResult<Json<Value>> {
    let rp = relying_party(&state, &headers).await?;
    let (challenge_id, challenge) = passkeys::start(Ceremony::Login, rp.clone());
    Ok(Json(json!({
        "challenge_id": challenge_id,
        "options": {
            "challenge": base64url_encode(&challenge),
            "rpId": rp.id,
            "timeout": TIMEOUT_MS,
            "userVerification": "required",
            // Vide : clé découvrable, le navigateur propose celles qu'il connaît.
            "allowCredentials": [],
        },
    })))
}

#[derive(Debug, Deserialize)]
pub struct LoginFinish {
    challenge_id: String,
    credential: AssertionCredential,
}

#[derive(Debug, Deserialize)]
pub struct AssertionCredential {
    id: String,
    response: AssertionResponse,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionResponse {
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    authenticator_data: String,
    signature: String,
    #[serde(default)]
    user_handle: Option<String>,
}

/// `POST /api/auth/passkey/login/finish` — ouvre la session si l'assertion est valide.
pub async fn login_finish(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    ClientIp(ip): ClientIp,
    Json(payload): Json<LoginFinish>,
) -> AuthResult<Response> {
    let keys = anonymous_keys(ip);
    guard_attempt(&auth, &keys).await?;

    let fail = |reason: &str| {
        tracing::warn!(reason, "passkey sign-in refused");
        AuthError::Unauthorized("Passkey sign-in failed.".into())
    };
    let outcome = verify_login(&state, &payload).await;
    let (user, stored_id, assertion) = match outcome {
        Ok(found) => found,
        Err(reason) => {
            auth.limiter().lock().await.record_failure(&keys, Instant::now());
            audit::record(&state.pool, None, "passkey.failed", None, ip).await;
            return Err(fail(reason));
        }
    };
    auth.limiter().lock().await.record_success(&keys);
    passkeys::record_use(&state.pool, stored_id, assertion.sign_count, assertion.backup_state)
        .await?;
    audit::record(&state.pool, Some(&user.username), "login.passkey", None, ip).await;
    open_session(&state, &auth, &user, ip).await
}

/// Retrouve la clé, le compte, et vérifie l'assertion. L'erreur est la raison
/// du refus, destinée au journal seulement.
async fn verify_login(
    state: &AppState,
    payload: &LoginFinish,
) -> Result<(User, i64, webauthn::Assertion), &'static str> {
    let pending = passkeys::take(&payload.challenge_id).ok_or("unknown or expired challenge")?;
    if !matches!(pending.ceremony, Ceremony::Login) {
        return Err("challenge of another ceremony");
    }
    let credential_id = base64url_decode(&payload.credential.id).ok_or("credential id")?;
    let stored = passkeys::find(&state.pool, &credential_id)
        .await
        .map_err(|_| "database")?
        .ok_or("unknown passkey")?;
    let user = users::get(&state.pool, stored.user_id)
        .await
        .map_err(|_| "database")?
        .filter(|user| !user.disabled && user.password_hash.is_some())
        .ok_or("account unavailable")?;

    let response = &payload.credential.response;
    // Le navigateur renvoie l'identifiant d'utilisateur enregistré avec la clé.
    if let Some(handle) = response.user_handle.as_deref().filter(|h| !h.is_empty()) {
        let handle = base64url_decode(handle).ok_or("user handle")?;
        if handle != user.id.to_string().as_bytes() {
            return Err("user handle mismatch");
        }
    }
    let client_data = base64url_decode(&response.client_data_json).ok_or("client data")?;
    let authenticator_data = base64url_decode(&response.authenticator_data).ok_or("auth data")?;
    let signature = base64url_decode(&response.signature).ok_or("signature")?;
    let assertion = webauthn::verify_assertion(
        &pending.rp,
        &pending.challenge,
        &client_data,
        &authenticator_data,
        &signature,
        &stored.public_key,
        stored.sign_count,
    )
    .map_err(|error| match error {
        webauthn::WebAuthnError::Malformed(why) | webauthn::WebAuthnError::Rejected(why) => why,
    })?;
    Ok((user, stored.id, assertion))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_nom_est_nettoye_et_borne() {
        assert_eq!(clean_name("  YubiKey  "), "YubiKey");
        assert_eq!(clean_name(""), "Passkey");
        assert_eq!(clean_name("a\nb"), "ab");
        assert_eq!(clean_name(&"x".repeat(200)).chars().count(), MAX_NAME_CHARS);
    }

    #[test]
    fn le_mot_de_passe_ne_fuit_pas_dans_les_traces() {
        let rendered = format!("{:?}", RegisterBegin { password: "s3cr3t".into() });
        assert!(!rendered.contains("s3cr3t"), "{rendered}");
    }
}
