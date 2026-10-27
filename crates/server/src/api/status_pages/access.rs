//! Pages de statut privées : qui peut lire une page.
//!
//! Trois modes, par page (`status_pages.access_mode`) :
//!
//! - `public` : tout le monde (comportement historique) ;
//! - `password` : un mot de passe propre à la page. Le visiteur le saisit une
//!   fois, le serveur le compare à l'empreinte Argon2id et pose un cookie
//!   d'accès signé (HMAC-SHA256, sous-clé dérivée du secret d'instance). Le
//!   cookie porte la page, une échéance et une empreinte du mot de passe : en
//!   changer invalide tous les cookies déjà posés ;
//! - `users` : un compte DumbMonit connecté (session locale, LDAP ou SSO,
//!   toutes valent). Le cookie de session est propre à l'hôte principal : ce
//!   mode ne s'applique donc pas sur le domaine public d'une page.
//!
//! Le contrôle s'applique à tout ce qui sort d'une page par son slug : document
//! JSON, badges, flux RSS, calendrier iCal, logo, détail d'un jour, abonnement.
//! Les essais de mot de passe sont comptés par adresse et par page
//! ([`crate::auth::rate_limit`]), en mémoire comme pour la connexion.

use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::{invalidate_cache, page_not_found, public_not_found};
use crate::api::{ApiError, ApiResult};
use crate::auth::client_ip::ClientIp;
use crate::auth::rate_limit::{Buckets, Key};
use crate::auth::{middleware, password};
use crate::db;
use crate::db::status_pages::StatusPage;
use crate::state::AppState;

pub const MODES: [&str; 3] = ["public", "password", "users"];

/// Durée de validité du cookie d'accès, en secondes : une journée de travail.
const COOKIE_TTL_SECS: i64 = 12 * 3600;
const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 200;
/// Étiquette de la sous-clé de signature (voir `Cipher::subkey`).
const KEY_LABEL: &[u8] = b"status-page-access/v1";
const PASSWORD_NEEDED: &str = "This page is protected by a password.";
const SIGN_IN_NEEDED: &str = "Sign in to DumbMonit to see this page.";

/// Seaux des essais de mot de passe, partagés par toutes les pages.
static LIMITER: LazyLock<Mutex<Buckets>> = LazyLock::new(|| Mutex::new(Buckets::new()));

fn cookie_name(page_id: i64) -> String {
    format!("dm_page_{page_id}")
}

/// Empreinte courte de l'empreinte Argon2id : suffit à lier le cookie au mot de
/// passe courant sans exposer l'empreinte elle-même.
fn fingerprint(hash: &str) -> String {
    hex::encode(Sha256::digest(hash.as_bytes()))[..16].to_string()
}

fn sign(key: &[u8; 32], page_id: i64, expires: i64, fp: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(format!("page|{page_id}|{expires}|{fp}").as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Valeur du cookie d'accès : `<échéance>.<signature>`.
fn cookie_value(key: &[u8; 32], page_id: i64, expires: i64, fp: &str) -> String {
    format!("{expires}.{}", sign(key, page_id, expires, fp))
}

/// Vérifie la valeur d'un cookie d'accès : forme, signature en temps constant,
/// puis échéance.
fn cookie_is_valid(key: &[u8; 32], page_id: i64, fp: &str, value: &str, now: i64) -> bool {
    let Some((expires, tag)) = value.split_once('.') else { return false };
    let Ok(expires) = expires.parse::<i64>() else { return false };
    let expected = sign(key, page_id, expires, fp);
    bool::from(expected.as_bytes().ct_eq(tag.as_bytes())) && expires > now
}

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|line| line.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.to_string())
}

/// Laisse passer la requête si la page est ouverte à son porteur, sinon renvoie
/// 401 avec le mode d'accès demandé (l'interface affiche alors le formulaire ou
/// l'invitation à se connecter).
pub async fn authorize(state: &AppState, page: &StatusPage, headers: &HeaderMap) -> ApiResult<()> {
    match page.access_mode.as_str() {
        "password" => {
            let hash = db::status_pages::access_password_hash(&state.pool, page.id).await?;
            let valid = match (hash, read_cookie(headers, &cookie_name(page.id))) {
                (Some(hash), Some(value)) => cookie_is_valid(
                    &state.cipher.subkey(KEY_LABEL),
                    page.id,
                    &fingerprint(&hash),
                    &value,
                    Utc::now().timestamp(),
                ),
                _ => false,
            };
            match valid {
                true => Ok(()),
                false => Err(ApiError::AccessRequired("password", PASSWORD_NEEDED.into())),
            }
        }
        "users" => {
            let session = middleware::current_session(&state.pool, headers)
                .await
                .map_err(|_| ApiError::Internal(anyhow::anyhow!("session illisible")))?;
            match session {
                Some(_) => Ok(()),
                None => Err(ApiError::AccessRequired("users", SIGN_IN_NEEDED.into())),
            }
        }
        _ => Ok(()),
    }
}

/// Une réponse de page privée ne se met en cache nulle part hors du navigateur
/// de celui qui y a droit.
pub fn seal(mut response: Response, private: bool) -> Response {
    if private {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
        response.headers_mut().remove(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        let noindex = HeaderValue::from_static("noindex, nofollow");
        response.headers_mut().insert("x-robots-tag", noindex);
    }
    response
}

/// Page publiée d'après son slug, puis contrôle d'accès : le point d'entrée
/// commun des routes publiques.
pub async fn open_page(state: &AppState, slug: &str, headers: &HeaderMap) -> ApiResult<StatusPage> {
    let page = match db::status_pages::get_page_by_slug(&state.pool, slug).await? {
        Some(page) if page.published => page,
        _ => return Err(public_not_found()),
    };
    authorize(state, &page, headers).await?;
    Ok(page)
}

#[derive(Debug, Deserialize)]
pub struct UnlockPayload {
    pub password: String,
}

/// `POST /api/public/status/{slug}/access` : échange le mot de passe de la page
/// contre un cookie d'accès.
pub async fn unlock(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(slug): Path<String>,
    Json(payload): Json<UnlockPayload>,
) -> ApiResult<Response> {
    let page = match db::status_pages::get_page_by_slug(&state.pool, &slug).await? {
        Some(page) if page.published && page.access_mode == "password" => page,
        _ => return Err(public_not_found()),
    };
    let Some(hash) = db::status_pages::access_password_hash(&state.pool, page.id).await? else {
        return Err(public_not_found());
    };

    let mut keys = vec![Key::user(&format!("page:{}", page.id), ip)];
    if let Some(ip) = ip {
        keys.push(Key::Ip(ip));
    }
    let now = Instant::now();
    if let Err(wait) = LIMITER.lock().unwrap_or_else(|p| p.into_inner()).check(&keys, now) {
        return Err(ApiError::TooManyRequests(format!(
            "Too many attempts. Try again in {wait} seconds."
        )));
    }

    // Une saisie démesurée n'a pas à passer par Argon2.
    let attempt = payload.password;
    let ok = attempt.chars().count() <= MAX_PASSWORD_LEN
        && password::verify(attempt, hash.clone()).await?;
    {
        let mut limiter = LIMITER.lock().unwrap_or_else(|p| p.into_inner());
        if ok {
            limiter.record_success(&keys);
        } else {
            limiter.record_failure(&keys, now);
        }
    }
    if !ok {
        return Err(ApiError::Forbidden("Wrong password.".into()));
    }

    let expires = Utc::now().timestamp() + COOKIE_TTL_SECS;
    let key = state.cipher.subkey(KEY_LABEL);
    let value = cookie_value(&key, page.id, expires, &fingerprint(&hash));
    let set = format!(
        "{}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={COOKIE_TTL_SECS}",
        cookie_name(page.id)
    );
    let mut response = Json(json!({ "status": "unlocked" })).into_response();
    if let Ok(value) = HeaderValue::from_str(&set) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[derive(Debug, Deserialize)]
pub struct AccessPayload {
    pub mode: String,
    /// Nouveau mot de passe d'accès ; absent, celui qui existe est gardé.
    #[serde(default)]
    pub password: Option<String>,
}

/// `PUT /api/status-pages/{id}/access` : règle le mode d'accès d'une page.
pub async fn set_access(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<AccessPayload>,
) -> ApiResult<Json<super::StatusPageView>> {
    let page =
        db::status_pages::get_page(&state.pool, id).await?.ok_or_else(|| page_not_found(id))?;
    let mode = payload.mode.trim().to_string();
    if !MODES.contains(&mode.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unknown access mode \"{mode}\" (expected: {}).",
            MODES.join(", ")
        )));
    }
    let given = payload.password.as_deref().map(str::trim).filter(|p| !p.is_empty());
    if let Some(given) = given {
        let length = given.chars().count();
        if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&length) {
            return Err(ApiError::BadRequest(format!(
                "The page password must be {MIN_PASSWORD_LEN} to {MAX_PASSWORD_LEN} characters long."
            )));
        }
    }
    let hash = if mode == "password" {
        match given {
            Some(given) => Some(Some(password::hash(given.to_string()).await?)),
            None if page.has_password => None,
            None => {
                return Err(ApiError::BadRequest(
                    "Set a password for the page before protecting it with one.".into(),
                ));
            }
        }
    } else {
        // Un autre mode n'a que faire d'un mot de passe oublié dans la base.
        Some(None)
    };
    db::status_pages::set_access(&state.pool, id, &mode, hash.as_ref().map(|h| h.as_deref()))
        .await?;
    invalidate_cache();
    let page =
        db::status_pages::get_page(&state.pool, id).await?.ok_or_else(|| page_not_found(id))?;
    Ok(Json(super::page_view(&state, page).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7; 32];

    #[test]
    fn a_fresh_cookie_is_accepted_until_it_expires() {
        let fp = fingerprint("$argon2id$hash");
        let value = cookie_value(&KEY, 3, 1_000, &fp);
        assert!(cookie_is_valid(&KEY, 3, &fp, &value, 999));
        assert!(!cookie_is_valid(&KEY, 3, &fp, &value, 1_000), "expired");
    }

    #[test]
    fn a_cookie_is_bound_to_its_page_and_its_password() {
        let fp = fingerprint("$argon2id$hash");
        let value = cookie_value(&KEY, 3, 1_000, &fp);
        assert!(!cookie_is_valid(&KEY, 4, &fp, &value, 0), "other page");
        assert!(!cookie_is_valid(&KEY, 3, &fingerprint("$argon2id$other"), &value, 0));
        assert!(!cookie_is_valid(&[8; 32], 3, &fp, &value, 0), "other instance key");
    }

    #[test]
    fn a_forged_or_mangled_cookie_is_refused() {
        let fp = fingerprint("h");
        assert!(!cookie_is_valid(&KEY, 1, &fp, "", 0));
        assert!(!cookie_is_valid(&KEY, 1, &fp, "abc.def", 0));
        assert!(!cookie_is_valid(&KEY, 1, &fp, "99999999999.00", 0));
        // Prolonger l'échéance sans refaire la signature ne passe pas.
        let value = cookie_value(&KEY, 1, 10, &fp);
        let tag = value.split_once('.').unwrap().1;
        assert!(!cookie_is_valid(&KEY, 1, &fp, &format!("99999999999.{tag}"), 0));
    }

    #[test]
    fn the_cookie_is_found_among_others() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("a=1; dm_page_3=123.abc; dumbmonit_session=x.y"),
        );
        assert_eq!(read_cookie(&headers, "dm_page_3").as_deref(), Some("123.abc"));
        assert_eq!(read_cookie(&headers, "dm_page_4"), None);
    }
}
