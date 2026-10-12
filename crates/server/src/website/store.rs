//! Ce que le collecteur `website` garde en base : la référence des
//! métadonnées, le dernier résultat, le cache `robots.txt` / plan de site et
//! l'historique des changements confirmés.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use dumbmonit_proto::TargetId;
use sqlx::{FromRow, SqlitePool};

use super::changes::{Change, Tracker};

/// Changements gardés par cible.
pub const KEEP_CHANGES: i64 = 200;

#[derive(Debug, Clone, FromRow)]
pub struct StateRow {
    pub fingerprint: String,
    pub tracker: String,
    pub changed_at: String,
    pub last_probe: Option<String>,
    pub last_probe_at: Option<String>,
    pub last_error: Option<String>,
    pub robots_checked_at: Option<String>,
    /// Âge de la lecture de `robots.txt`, en secondes.
    pub robots_age: Option<i64>,
    pub robots_blocks_all: Option<bool>,
    pub sitemap_ok: Option<bool>,
    /// Dernière comparaison des métadonnées.
    pub changes_checked_at: Option<String>,
    /// Son âge, en secondes.
    pub check_age: Option<i64>,
}

impl StateRow {
    pub fn tracker(&self) -> Tracker {
        serde_json::from_str(&self.tracker).unwrap_or_default()
    }

    /// Champ → date du dernier changement confirmé.
    pub fn changed_at(&self) -> BTreeMap<String, String> {
        serde_json::from_str(&self.changed_at).unwrap_or_default()
    }
}

pub async fn state(pool: &SqlitePool, target: TargetId) -> Result<Option<StateRow>> {
    sqlx::query_as(
        "SELECT fingerprint, tracker, changed_at, last_probe, last_probe_at, last_error,
                robots_checked_at,
                CAST(strftime('%s', 'now') - strftime('%s', robots_checked_at) AS INTEGER)
                    AS robots_age,
                robots_blocks_all, sitemap_ok, changes_checked_at,
                CAST(strftime('%s', 'now') - strftime('%s', changes_checked_at) AS INTEGER)
                    AS check_age
         FROM website_state WHERE target_id = ?",
    )
    .bind(target)
    .fetch_optional(pool)
    .await
    .context("lecture de l'état du site")
}

/// Ce qu'une lecture réussie enregistre.
pub struct Saved<'a> {
    pub fingerprint: &'a str,
    pub tracker: &'a Tracker,
    pub changes: &'a [Change],
    pub last_probe: &'a str,
    /// `Some` quand `robots.txt` et le plan de site viennent d'être relus.
    pub robots: Option<(bool, bool)>,
    /// Les métadonnées ont été comparées à la référence (sonde de
    /// changement) ; sinon `tracker` est rendu tel quel.
    pub checked: bool,
}

/// Enregistre une lecture et ses changements, d'un bloc.
pub async fn save(pool: &SqlitePool, target: TargetId, saved: Saved<'_>) -> Result<()> {
    let mut tx = pool.begin().await?;
    let mut changed_at: BTreeMap<String, String> =
        sqlx::query_scalar::<_, String>("SELECT changed_at FROM website_state WHERE target_id = ?")
            .bind(target)
            .fetch_optional(&mut *tx)
            .await?
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
    if !saved.changes.is_empty() {
        let now: String = sqlx::query_scalar("SELECT datetime('now')").fetch_one(&mut *tx).await?;
        for change in saved.changes {
            sqlx::query(
                "INSERT INTO website_changes (target_id, detected_at, field, before, after)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(target)
            .bind(&now)
            .bind(&change.field)
            .bind(&change.before)
            .bind(&change.after)
            .execute(&mut *tx)
            .await?;
            changed_at.insert(change.field.clone(), now.clone());
        }
        sqlx::query(
            "DELETE FROM website_changes WHERE target_id = ? AND id NOT IN
               (SELECT id FROM website_changes WHERE target_id = ? ORDER BY id DESC LIMIT ?)",
        )
        .bind(target)
        .bind(target)
        .bind(KEEP_CHANGES)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO website_state (target_id, fingerprint, tracker, changed_at, last_probe,
                                    last_probe_at, last_error)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), NULL)
         ON CONFLICT (target_id) DO UPDATE SET
            fingerprint = excluded.fingerprint, tracker = excluded.tracker,
            changed_at = excluded.changed_at, last_probe = excluded.last_probe,
            last_probe_at = excluded.last_probe_at, last_error = NULL",
    )
    .bind(target)
    .bind(saved.fingerprint)
    .bind(serde_json::to_string(saved.tracker)?)
    .bind(serde_json::to_string(&changed_at)?)
    .bind(saved.last_probe)
    .execute(&mut *tx)
    .await?;
    if saved.checked {
        sqlx::query(
            "UPDATE website_state SET changes_checked_at = datetime('now') WHERE target_id = ?",
        )
        .bind(target)
        .execute(&mut *tx)
        .await?;
    }
    if let Some((blocks_all, sitemap_ok)) = saved.robots {
        sqlx::query(
            "UPDATE website_state SET robots_checked_at = datetime('now'),
                    robots_blocks_all = ?, sitemap_ok = ? WHERE target_id = ?",
        )
        .bind(blocks_all)
        .bind(sitemap_ok)
        .bind(target)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await.context("enregistrement de la lecture du site")
}

/// Repart d'une référence neuve : réglages changés, ou numéro de cible repris.
pub async fn reset(pool: &SqlitePool, target: TargetId) -> Result<()> {
    sqlx::query("DELETE FROM website_state WHERE target_id = ?").bind(target).execute(pool).await?;
    Ok(())
}

/// Note l'échec d'une lecture, sans toucher à la référence.
pub async fn save_error(pool: &SqlitePool, target: TargetId, message: &str) -> Result<()> {
    sqlx::query(
        "UPDATE website_state SET last_error = ?, last_probe_at = datetime('now')
         WHERE target_id = ?",
    )
    .bind(message)
    .bind(target)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ChangeRow {
    pub id: i64,
    pub detected_at: String,
    pub field: String,
    pub before: String,
    pub after: String,
}

/// Les derniers changements, du plus récent au plus ancien.
pub async fn changes(pool: &SqlitePool, target: TargetId, limit: i64) -> Result<Vec<ChangeRow>> {
    sqlx::query_as(
        "SELECT id, detected_at, field, before, after FROM website_changes
         WHERE target_id = ? ORDER BY id DESC LIMIT ?",
    )
    .bind(target)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("lecture des changements du site")
}
