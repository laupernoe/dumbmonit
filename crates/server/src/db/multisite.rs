//! Réglages des sondes multi-emplacements (voir [`crate::multisite`]).

use std::collections::HashMap;

use anyhow::{Context, Result};
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// Emplacements d'où un moniteur est sondé, et seuil de la panne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultisiteConfig {
    /// Le serveur sonde aussi, en direct.
    pub server: bool,
    /// Agents relais qui sondent aussi, chacun depuis son réseau.
    #[serde(default)]
    pub agents: Vec<TargetId>,
    /// Emplacements en échec qu'il faut pour déclarer la panne ; `None` : la
    /// majorité des emplacements.
    #[serde(default)]
    pub quorum: Option<u32>,
}

impl MultisiteConfig {
    /// Nombre d'emplacements configurés, serveur compris.
    pub fn locations(&self) -> usize {
        self.agents.len() + usize::from(self.server)
    }
}

/// Réglages de toutes les cibles qui en ont, indexés par cible.
pub async fn all(pool: &SqlitePool) -> Result<HashMap<TargetId, MultisiteConfig>> {
    let rows = sqlx::query("SELECT target_id, include_server, quorum FROM target_multisite")
        .fetch_all(pool)
        .await
        .context("lecture des sondes multi-emplacements")?;
    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        let quorum: Option<i64> = row.try_get("quorum")?;
        map.insert(
            row.try_get::<TargetId, _>("target_id")?,
            MultisiteConfig {
                server: row.try_get::<i64, _>("include_server")? != 0,
                agents: Vec::new(),
                quorum: quorum.and_then(|q| u32::try_from(q).ok()),
            },
        );
    }
    let agents = sqlx::query(
        "SELECT target_id, agent_id FROM target_multisite_agents ORDER BY target_id, agent_id",
    )
    .fetch_all(pool)
    .await
    .context("lecture des agents des sondes multi-emplacements")?;
    for row in agents {
        let target: TargetId = row.try_get("target_id")?;
        if let Some(config) = map.get_mut(&target) {
            config.agents.push(row.try_get("agent_id")?);
        }
    }
    Ok(map)
}

/// Réglages d'une cible, s'il y en a.
pub async fn get(pool: &SqlitePool, id: TargetId) -> Result<Option<MultisiteConfig>> {
    let Some(row) =
        sqlx::query("SELECT include_server, quorum FROM target_multisite WHERE target_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .context("lecture de la sonde multi-emplacements")?
    else {
        return Ok(None);
    };
    let quorum: Option<i64> = row.try_get("quorum")?;
    let agents = sqlx::query_scalar::<_, TargetId>(
        "SELECT agent_id FROM target_multisite_agents WHERE target_id = ? ORDER BY agent_id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("lecture des agents de la sonde multi-emplacements")?;
    Ok(Some(MultisiteConfig {
        server: row.try_get::<i64, _>("include_server")? != 0,
        agents,
        quorum: quorum.and_then(|q| u32::try_from(q).ok()),
    }))
}

/// Enregistre (ou retire, `None`) les réglages d'une cible, d'un bloc.
pub async fn set(pool: &SqlitePool, id: TargetId, config: Option<&MultisiteConfig>) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM target_multisite WHERE target_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .context("retrait de la sonde multi-emplacements")?;
    if let Some(config) = config {
        sqlx::query(
            "INSERT INTO target_multisite (target_id, include_server, quorum) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(i64::from(config.server))
        .bind(config.quorum.map(i64::from))
        .execute(&mut *tx)
        .await
        .context("enregistrement de la sonde multi-emplacements")?;
        for agent in &config.agents {
            sqlx::query(
                "INSERT OR IGNORE INTO target_multisite_agents (target_id, agent_id) VALUES (?, ?)",
            )
            .bind(id)
            .bind(agent)
            .execute(&mut *tx)
            .await
            .context("enregistrement d'un agent de la sonde multi-emplacements")?;
        }
    }
    tx.commit().await?;
    Ok(())
}
