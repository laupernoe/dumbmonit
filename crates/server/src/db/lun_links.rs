//! Liens manuels entre un LUN d'un NAS et l'équipement qui l'utilise.
//!
//! Table `lun_links` (migration 0213), lue par `GET /targets/{id}/luns`
//! ([`crate::api::luns`]) et voyageant dans la sauvegarde
//! ([`crate::backup`]).

use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::SqlitePool;

/// Un lien : le LUN `lun` du NAS `target_id` est l'entité `entity` de la
/// famille de capacité `family` sur l'équipement `consumer_target_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct LunLink {
    pub target_id: i64,
    pub lun: String,
    pub consumer_target_id: i64,
    pub family: String,
    pub entity: String,
    pub author: String,
    pub created_at: String,
}

/// Les liens d'un NAS.
pub async fn list_for(pool: &SqlitePool, target_id: i64) -> Result<Vec<LunLink>> {
    sqlx::query_as(
        "SELECT target_id, lun, consumer_target_id, family, entity, author, created_at
         FROM lun_links WHERE target_id = ? ORDER BY lun",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
    .context("lecture des liens de LUN")
}

/// Tous les liens, pour la sauvegarde.
pub async fn list_all(pool: &SqlitePool) -> Result<Vec<LunLink>> {
    sqlx::query_as(
        "SELECT target_id, lun, consumer_target_id, family, entity, author, created_at
         FROM lun_links ORDER BY target_id, lun",
    )
    .fetch_all(pool)
    .await
    .context("lecture des liens de LUN")
}

/// Crée ou remplace le lien d'un LUN.
pub async fn upsert(
    pool: &SqlitePool,
    target_id: i64,
    lun: &str,
    consumer_target_id: i64,
    family: &str,
    entity: &str,
    author: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO lun_links (target_id, lun, consumer_target_id, family, entity, author)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (target_id, lun) DO UPDATE SET
            consumer_target_id = excluded.consumer_target_id,
            family = excluded.family,
            entity = excluded.entity,
            author = excluded.author,
            created_at = datetime('now')",
    )
    .bind(target_id)
    .bind(lun)
    .bind(consumer_target_id)
    .bind(family)
    .bind(entity)
    .bind(author)
    .execute(pool)
    .await
    .context("enregistrement du lien de LUN")?;
    Ok(())
}

/// Retire le lien d'un LUN ; faux s'il n'y en avait pas.
pub async fn delete(pool: &SqlitePool, target_id: i64, lun: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM lun_links WHERE target_id = ? AND lun = ?")
        .bind(target_id)
        .bind(lun)
        .execute(pool)
        .await
        .context("suppression du lien de LUN")?;
    Ok(result.rows_affected() > 0)
}
