//! Abonnements Web Push en base : un par navigateur et par compte.
//!
//! Les clés de chiffrement du navigateur (`p256dh`, `auth`) sont chiffrées par
//! le secret d'instance et ne sortent jamais par l'API ; l'adresse complète de
//! l'abonnement non plus — c'est un jeton de fait, qui suffit à envoyer des
//! messages au navigateur à qui possède aussi les clés. L'interface reçoit le
//! nom du service de push et une empreinte courte, de quoi reconnaître « cet
//! appareil » sans rien pouvoir en faire.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

use crate::crypto::Cipher;

/// Longueur maximale conservée d'une erreur d'envoi.
const MAX_ERROR_LEN: usize = 300;

/// Abonnement tel que l'interface le voit.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Device {
    pub id: i64,
    /// Libellé lisible : « Firefox on Android ».
    pub device: String,
    /// Hôte du service de push (`fcm.googleapis.com`, `web.push.apple.com`…).
    pub push_service: String,
    /// 16 premiers caractères hexadécimaux du SHA-256 de l'adresse : le
    /// navigateur calcule la même chose pour se reconnaître dans la liste.
    pub fingerprint: String,
    pub created_at: String,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
}

/// Abonnement prêt à recevoir un message, clés déchiffrées.
pub struct Subscription {
    pub id: i64,
    pub endpoint: String,
    pub p256dh: Vec<u8>,
    pub auth: Vec<u8>,
}

/// Empreinte publique d'une adresse d'abonnement.
pub fn fingerprint(endpoint: &str) -> String {
    hex::encode(Sha256::digest(endpoint.as_bytes()))[..16].to_string()
}

fn push_service(endpoint: &str) -> String {
    reqwest::Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_default()
}

/// Enregistre un abonnement, ou remplace celui de la même adresse.
///
/// Un navigateur n'a qu'un abonnement par site : s'il se réabonne — clés
/// renouvelées, ou un autre compte connecté sur le même appareil —, la ligne
/// existante change de propriétaire et de clés au lieu d'en créer une seconde,
/// qui recevrait chaque alerte en double.
pub async fn upsert(
    pool: &SqlitePool,
    cipher: &Cipher,
    user_id: i64,
    endpoint: &str,
    p256dh: &[u8],
    auth: &[u8],
    device: &str,
) -> Result<Device> {
    let p256dh_enc = cipher.encrypt(p256dh)?;
    let auth_enc = cipher.encrypt(auth)?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO webpush_subscriptions (user_id, endpoint, p256dh_enc, auth_enc, device)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(endpoint) DO UPDATE SET
             user_id = excluded.user_id,
             p256dh_enc = excluded.p256dh_enc,
             auth_enc = excluded.auth_enc,
             device = excluded.device,
             last_error = NULL
         RETURNING id",
    )
    .bind(user_id)
    .bind(endpoint)
    .bind(&p256dh_enc)
    .bind(&auth_enc)
    .bind(device)
    .fetch_one(pool)
    .await
    .context("storing the push subscription")?;
    get(pool, id).await?.context("push subscription missing after insert")
}

fn device_from(row: &sqlx::sqlite::SqliteRow) -> Result<Device> {
    let endpoint: String = row.try_get("endpoint")?;
    Ok(Device {
        id: row.try_get("id")?,
        device: row.try_get("device")?,
        push_service: push_service(&endpoint),
        fingerprint: fingerprint(&endpoint),
        created_at: row.try_get("created_at")?,
        last_success_at: row.try_get("last_success_at")?,
        last_error: row.try_get("last_error")?,
    })
}

async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Device>> {
    let row = sqlx::query(
        "SELECT id, endpoint, device, created_at, last_success_at, last_error
         FROM webpush_subscriptions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(device_from).transpose()
}

/// Appareils abonnés d'un compte, du plus récent au plus ancien.
pub async fn list_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Device>> {
    let rows = sqlx::query(
        "SELECT id, endpoint, device, created_at, last_success_at, last_error
             FROM webpush_subscriptions WHERE user_id = ? ORDER BY id DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("listing push subscriptions")?;
    rows.iter().map(device_from).collect()
}

/// Supprime un abonnement du compte. Faux si l'abonnement n'existe pas ou
/// appartient à quelqu'un d'autre — les deux cas se confondent volontairement.
pub async fn delete_for_user(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM webpush_subscriptions WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Supprime un abonnement que le service de push a déclaré expiré (404/410).
pub async fn delete(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM webpush_subscriptions WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn mark_success(pool: &SqlitePool, id: i64, at: DateTime<Utc>) -> Result<()> {
    sqlx::query(
        "UPDATE webpush_subscriptions SET last_success_at = ?, last_error = NULL WHERE id = ?",
    )
    .bind(at.format("%Y-%m-%d %H:%M:%S").to_string())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_error(pool: &SqlitePool, id: i64, error: &str) -> Result<()> {
    let error: String = error.chars().take(MAX_ERROR_LEN).collect();
    sqlx::query("UPDATE webpush_subscriptions SET last_error = ? WHERE id = ?")
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Qui doit recevoir un message.
pub enum Audience<'a> {
    /// Les comptes nommés (vide : tous les comptes actifs).
    Users(&'a [String]),
    /// Un compte, éventuellement réduit à un seul de ses appareils.
    User { user_id: i64, subscription: Option<i64> },
}

/// Abonnements destinataires, clés déchiffrées. Les comptes désactivés n'en
/// reçoivent pas. Un abonnement illisible (secret d'instance changé, ligne
/// abîmée) est ignoré : il ne doit pas bloquer les autres.
pub async fn recipients(
    pool: &SqlitePool,
    cipher: &Cipher,
    audience: &Audience<'_>,
) -> Result<Vec<Subscription>> {
    let rows = sqlx::query(
        "SELECT s.id, s.user_id, s.endpoint, s.p256dh_enc, s.auth_enc, u.username
         FROM webpush_subscriptions s JOIN users u ON u.id = s.user_id
         WHERE u.disabled = 0 ORDER BY s.id",
    )
    .fetch_all(pool)
    .await
    .context("listing push recipients")?;

    let mut out = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let user_id: i64 = row.try_get("user_id")?;
        let username: String = row.try_get("username")?;
        let wanted = match audience {
            Audience::Users(names) => {
                names.is_empty() || names.iter().any(|name| name.eq_ignore_ascii_case(&username))
            }
            Audience::User { user_id: wanted, subscription } => {
                *wanted == user_id && subscription.is_none_or(|only| only == id)
            }
        };
        if !wanted {
            continue;
        }
        let p256dh_enc: Vec<u8> = row.try_get("p256dh_enc")?;
        let auth_enc: Vec<u8> = row.try_get("auth_enc")?;
        let (Ok(p256dh), Ok(auth)) = (cipher.decrypt(&p256dh_enc), cipher.decrypt(&auth_enc))
        else {
            tracing::warn!(subscription = id, "push subscription keys unreadable, skipped");
            continue;
        };
        out.push(Subscription { id, endpoint: row.try_get("endpoint")?, p256dh, auth });
    }
    Ok(out)
}

/// Libellé lisible d'un navigateur, tiré de son en-tête `User-Agent`.
///
/// Volontairement grossier — navigateur et système, sans version : c'est ce
/// qu'il faut pour reconnaître son téléphone dans une liste, et rien de plus
/// n'est conservé.
pub fn describe_user_agent(user_agent: &str) -> String {
    let ua = user_agent;
    let browser = if ua.contains("Edg/") || ua.contains("EdgA/") || ua.contains("EdgiOS/") {
        "Edge"
    } else if ua.contains("OPR/") || ua.contains("Opera") {
        "Opera"
    } else if ua.contains("SamsungBrowser/") {
        "Samsung Internet"
    } else if ua.contains("Firefox/") || ua.contains("FxiOS/") {
        "Firefox"
    } else if ua.contains("Chrome/") || ua.contains("CriOS/") || ua.contains("Chromium/") {
        "Chrome"
    } else if ua.contains("Safari/") {
        "Safari"
    } else {
        "Browser"
    };
    let system = if ua.contains("iPhone") {
        "iPhone"
    } else if ua.contains("iPad") {
        "iPad"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
        "macOS"
    } else if ua.contains("CrOS") {
        "ChromeOS"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        ""
    };
    if system.is_empty() { browser.to_string() } else { format!("{browser} on {system}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_navigateurs_courants_sont_reconnus() {
        for (ua, expected) in [
            (
                "Mozilla/5.0 (Android 14; Mobile; rv:131.0) Gecko/131.0 Firefox/131.0",
                "Firefox on Android",
            ),
            (
                "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 \
                 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
                "Safari on iPhone",
            ),
            (
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/129.0.0.0 Safari/537.36 Edg/129.0.0.0",
                "Edge on Windows",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/129.0.0.0 Safari/537.36",
                "Chrome on Linux",
            ),
            ("curl/8.0", "Browser"),
        ] {
            assert_eq!(describe_user_agent(ua), expected, "{ua}");
        }
    }

    #[test]
    fn l_empreinte_est_courte_et_stable() {
        let a = fingerprint("https://fcm.googleapis.com/fcm/send/abc");
        assert_eq!(a.len(), 16);
        assert_eq!(a, fingerprint("https://fcm.googleapis.com/fcm/send/abc"));
        assert_ne!(a, fingerprint("https://fcm.googleapis.com/fcm/send/abd"));
    }
}
