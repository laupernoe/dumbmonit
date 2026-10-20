//! Persistance : destinations (`log_destinations`) et réglages de rétention
//! (`app_settings`).
//!
//! Comme pour les canaux de notification, rien de chiffré ne ressort d'une
//! lecture ordinaire : [`list`] dit seulement s'il existe un secret.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};
use sqlx::{Row, SqlitePool};

use super::catalog::DestinationConfig;
use crate::crypto::Cipher;

const RETENTION_KEY: &str = "logs_retention_days";
const MAX_TOTAL_KEY: &str = "logs_max_total_mb";

/// Une destination, telle que l'API la renvoie (sans ses secrets).
#[derive(Debug, Clone, Serialize)]
pub struct Destination {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub min_level: String,
    pub settings: Map<String, Value>,
    pub has_secret: bool,
}

/// Ce qu'une création ou une modification enregistre.
pub struct Input<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub enabled: bool,
    pub min_level: &'a str,
    pub settings: &'a Map<String, Value>,
}

fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Destination> {
    let settings: String = row.try_get("settings")?;
    let secrets: Option<Vec<u8>> = row.try_get("secrets")?;
    Ok(Destination {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        kind: row.try_get("kind")?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        min_level: row.try_get("min_level")?,
        settings: serde_json::from_str(&settings).unwrap_or_default(),
        has_secret: secrets.is_some_and(|bytes| !bytes.is_empty()),
    })
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Destination>> {
    let rows = sqlx::query(
        "SELECT id, name, kind, enabled, min_level, settings, secrets FROM log_destinations \
         ORDER BY name, id",
    )
    .fetch_all(pool)
    .await
    .context("reading log destinations")?;
    rows.iter().map(from_row).collect()
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Destination>> {
    let row = sqlx::query(
        "SELECT id, name, kind, enabled, min_level, settings, secrets FROM log_destinations \
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("reading a log destination")?;
    row.as_ref().map(from_row).transpose()
}

/// Secrets déchiffrés d'une destination ; vides s'il n'y en a pas.
pub async fn secrets(
    pool: &SqlitePool,
    cipher: &Cipher,
    id: i64,
) -> Result<HashMap<String, String>> {
    let blob: Option<Option<Vec<u8>>> =
        sqlx::query_scalar("SELECT secrets FROM log_destinations WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    match blob.flatten() {
        Some(bytes) if !bytes.is_empty() => {
            let plain = cipher.decrypt(&bytes).context("log destination secrets unreadable")?;
            Ok(serde_json::from_slice(&plain).unwrap_or_default())
        }
        _ => Ok(HashMap::new()),
    }
}

/// Configuration complète, secrets compris, pour construire l'expéditeur.
pub async fn config(
    pool: &SqlitePool,
    cipher: &Cipher,
    destination: &Destination,
) -> Result<DestinationConfig> {
    Ok(DestinationConfig {
        settings: destination.settings.clone(),
        secrets: secrets(pool, cipher, destination.id).await?,
    })
}

fn seal(cipher: &Cipher, secrets: &HashMap<String, String>) -> Result<Option<Vec<u8>>> {
    if secrets.is_empty() {
        return Ok(None);
    }
    Ok(Some(cipher.encrypt(serde_json::to_string(secrets)?.as_bytes())?))
}

pub async fn insert(
    pool: &SqlitePool,
    cipher: &Cipher,
    input: &Input<'_>,
    secrets: &HashMap<String, String>,
) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO log_destinations (name, kind, enabled, min_level, settings, secrets) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(input.name)
    .bind(input.kind)
    .bind(input.enabled)
    .bind(input.min_level)
    .bind(Value::Object(input.settings.clone()).to_string())
    .bind(seal(cipher, secrets)?)
    .execute(pool)
    .await
    .context("saving a log destination")?;
    Ok(result.last_insert_rowid())
}

/// Modification ; `secrets` à `None` garde ceux en place.
pub async fn update(
    pool: &SqlitePool,
    cipher: &Cipher,
    id: i64,
    input: &Input<'_>,
    secrets: Option<&HashMap<String, String>>,
) -> Result<bool> {
    let settings = Value::Object(input.settings.clone()).to_string();
    let result = match secrets {
        Some(secrets) => {
            sqlx::query(
                "UPDATE log_destinations SET name = ?, kind = ?, enabled = ?, min_level = ?, \
             settings = ?, secrets = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(input.name)
            .bind(input.kind)
            .bind(input.enabled)
            .bind(input.min_level)
            .bind(settings)
            .bind(seal(cipher, secrets)?)
            .bind(id)
            .execute(pool)
            .await
        }
        None => {
            sqlx::query(
                "UPDATE log_destinations SET name = ?, kind = ?, enabled = ?, min_level = ?, \
             settings = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(input.name)
            .bind(input.kind)
            .bind(input.enabled)
            .bind(input.min_level)
            .bind(settings)
            .bind(id)
            .execute(pool)
            .await
        }
    }
    .context("updating a log destination")?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM log_destinations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("deleting a log destination")?;
    Ok(result.rows_affected() > 0)
}

// --------------------------------------------------------------------------
// Rétention
// --------------------------------------------------------------------------

async fn setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?)
}

async fn put_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rétention (jours) et plafond (Mo) enregistrés, ou les défauts.
pub async fn limits(pool: &SqlitePool) -> Result<(u32, u64)> {
    let retention = setting(pool, RETENTION_KEY)
        .await?
        .and_then(|value| value.parse().ok())
        .unwrap_or(super::DEFAULT_RETENTION_DAYS)
        .clamp(super::MIN_RETENTION_DAYS, super::MAX_RETENTION_DAYS);
    let max_total = setting(pool, MAX_TOTAL_KEY)
        .await?
        .and_then(|value| value.parse().ok())
        .unwrap_or(super::DEFAULT_MAX_TOTAL_MB)
        .clamp(super::MIN_TOTAL_MB, super::MAX_TOTAL_MB);
    Ok((retention, max_total))
}

pub async fn save_limits(pool: &SqlitePool, retention_days: u32, max_total_mb: u64) -> Result<()> {
    put_setting(pool, RETENTION_KEY, &retention_days.to_string()).await?;
    put_setting(pool, MAX_TOTAL_KEY, &max_total_mb.to_string()).await
}
