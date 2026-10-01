//! Persistance de la musique du mur (migration `0035_music.sql`) : le compte
//! Spotify relié et le lien partagé que jouent les murs.

use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::crypto::Cipher;

/// Le compte Spotify relié, tel qu'il est stocké. Le jeton de rafraîchissement
/// reste chiffré tant qu'on ne le présente pas à Spotify.
#[derive(Debug, Clone)]
pub struct Account {
    pub client_id: String,
    pub refresh_token: Option<Vec<u8>>,
    pub scopes: String,
    pub account_name: Option<String>,
    pub authorized_at: String,
    pub last_error: Option<String>,
}

impl Account {
    pub fn refresh_token(&self, cipher: &Cipher) -> Result<Option<String>> {
        self.refresh_token
            .as_deref()
            .map(|sealed| {
                let plain = cipher.decrypt(sealed).context("Spotify refresh token")?;
                String::from_utf8(plain).context("Spotify refresh token is not text")
            })
            .transpose()
    }
}

pub async fn account(pool: &SqlitePool) -> Result<Option<Account>> {
    let row = sqlx::query(
        "SELECT client_id, refresh_token, scopes, account_name, authorized_at, last_error
         FROM music_spotify WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .context("lecture du compte Spotify")?;
    row.map(|row| {
        Ok(Account {
            client_id: row.try_get("client_id")?,
            refresh_token: row.try_get("refresh_token")?,
            scopes: row.try_get("scopes")?,
            account_name: row.try_get("account_name")?,
            authorized_at: row.try_get("authorized_at")?,
            last_error: row.try_get("last_error")?,
        })
    })
    .transpose()
}

/// Enregistre une connexion toute neuve : elle remplace la précédente, et son
/// autorisation fait repartir les six mois du jeton de rafraîchissement.
pub async fn save_connection(
    pool: &SqlitePool,
    cipher: &Cipher,
    client_id: &str,
    refresh_token: &str,
    scopes: &str,
    account_id: &str,
    account_name: Option<&str>,
) -> Result<()> {
    let sealed = cipher.encrypt(refresh_token.as_bytes())?;
    sqlx::query(
        "INSERT INTO music_spotify
             (id, client_id, refresh_token, scopes, account_id, account_name, authorized_at,
              last_error, updated_at)
         VALUES (1, ?, ?, ?, ?, ?, datetime('now'), NULL, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             client_id = excluded.client_id, refresh_token = excluded.refresh_token,
             scopes = excluded.scopes, account_id = excluded.account_id,
             account_name = excluded.account_name, authorized_at = excluded.authorized_at,
             last_error = NULL, updated_at = excluded.updated_at",
    )
    .bind(client_id)
    .bind(sealed)
    .bind(scopes)
    .bind(account_id)
    .bind(account_name)
    .execute(pool)
    .await
    .context("enregistrement du compte Spotify")?;
    Ok(())
}

/// Spotify a remis un nouveau jeton de rafraîchissement : il remplace l'ancien.
pub async fn replace_refresh_token(pool: &SqlitePool, cipher: &Cipher, token: &str) -> Result<()> {
    let sealed = cipher.encrypt(token.as_bytes())?;
    sqlx::query(
        "UPDATE music_spotify SET refresh_token = ?, updated_at = datetime('now') WHERE id = 1",
    )
    .bind(sealed)
    .execute(pool)
    .await
    .context("mise à jour du jeton Spotify")?;
    Ok(())
}

/// Spotify ne reconnaît plus le jeton : on l'oublie, et on garde la raison
/// pour l'écran de réglages. Le client ID reste, pour se reconnecter en un geste.
pub async fn mark_expired(pool: &SqlitePool, reason: &str) -> Result<()> {
    sqlx::query(
        "UPDATE music_spotify SET refresh_token = NULL, last_error = ?, updated_at = datetime('now')
         WHERE id = 1",
    )
    .bind(reason)
    .execute(pool)
    .await
    .context("révocation du jeton Spotify")?;
    Ok(())
}

pub async fn delete_account(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM music_spotify WHERE id = 1")
        .execute(pool)
        .await
        .context("suppression du compte Spotify")?;
    Ok(())
}

/// Le lien que jouent les murs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WallLink {
    pub link: String,
    /// Compte (ou jeton) qui l'a envoyé.
    pub set_by: String,
    /// UTC sans suffixe, comme toutes les dates du serveur.
    pub set_at: String,
}

pub async fn link(pool: &SqlitePool) -> Result<Option<WallLink>> {
    let row = sqlx::query("SELECT link, set_by, set_at FROM music_wall_link WHERE id = 1")
        .fetch_optional(pool)
        .await
        .context("lecture du lien du mur")?;
    row.map(|row| {
        Ok(WallLink {
            link: row.try_get("link")?,
            set_by: row.try_get("set_by")?,
            set_at: row.try_get("set_at")?,
        })
    })
    .transpose()
}

pub async fn set_link(pool: &SqlitePool, link: &str, set_by: &str) -> Result<WallLink> {
    sqlx::query(
        "INSERT INTO music_wall_link (id, link, set_by, set_at) VALUES (1, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET link = excluded.link, set_by = excluded.set_by,
             set_at = excluded.set_at",
    )
    .bind(link)
    .bind(set_by)
    .execute(pool)
    .await
    .context("enregistrement du lien du mur")?;
    self::link(pool).await?.context("lien du mur introuvable après écriture")
}

pub async fn clear_link(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM music_wall_link WHERE id = 1")
        .execute(pool)
        .await
        .context("suppression du lien du mur")?;
    Ok(())
}
