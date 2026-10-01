//! Paquets d'intégration installés sur cette instance.
//!
//! Le format, la vérification et la collecte vivent dans `dumbmonit-pack` ; ici,
//! seulement ce qui tient au serveur : la table `packs`, l'enregistrement des
//! collecteurs dans le registre (sans redémarrage), les règles d'alerte livrées
//! par un paquet et les seuils réglés par équipement.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use dumbmonit_collectors::snmp::Catalog;
use dumbmonit_pack::{KIND_PREFIX, Pack, PackCollector, PackRule};
use dumbmonit_proto::Target;
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use tracing::{info, warn};

use crate::alerting::model::{AnomalyParams, TargetSelector};
use crate::collectors::Registry;

/// Rappel d'une alerte de paquet tant qu'elle dure : celui des règles livrées.
const REPEAT: Duration = Duration::from_secs(6 * 3600);

/// Un paquet tel qu'il est enregistré.
#[derive(Debug, Clone, Serialize)]
pub struct Installed {
    pub id: String,
    pub version: String,
    pub sha256: String,
    pub enabled: bool,
    /// UTC, sans suffixe, comme tous les horodatages du serveur.
    pub installed_at: String,
    #[serde(skip)]
    pub yaml: String,
}

fn installed(row: &sqlx::sqlite::SqliteRow) -> Result<Installed> {
    Ok(Installed {
        id: row.try_get("id")?,
        version: row.try_get("version")?,
        sha256: row.try_get("sha256")?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        installed_at: row.try_get("installed_at")?,
        yaml: row.try_get("yaml")?,
    })
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Installed>> {
    let rows = sqlx::query(
        "SELECT id, version, yaml, sha256, enabled, installed_at FROM packs ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .context("lecture des paquets")?;
    rows.iter().map(installed).collect()
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Installed>> {
    let row = sqlx::query(
        "SELECT id, version, yaml, sha256, enabled, installed_at FROM packs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("lecture du paquet")?;
    row.as_ref().map(installed).transpose()
}

/// Ce qu'une installation a changé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Saved {
    Created,
    Updated,
    Unchanged,
}

/// Enregistre un paquet vérifié. Une mise à jour garde l'état activé ou non.
pub async fn save(pool: &SqlitePool, pack: &Pack) -> Result<Saved> {
    let existing = get(pool, pack.id()).await?;
    if existing.as_ref().is_some_and(|existing| existing.sha256 == pack.sha256()) {
        return Ok(Saved::Unchanged);
    }
    sqlx::query(
        "INSERT INTO packs (id, version, yaml, sha256, enabled) VALUES (?, ?, ?, ?, 1)
         ON CONFLICT(id) DO UPDATE SET version = excluded.version, yaml = excluded.yaml,
             sha256 = excluded.sha256, installed_at = datetime('now')",
    )
    .bind(pack.id())
    .bind(pack.version())
    .bind(pack.yaml())
    .bind(pack.sha256())
    .execute(pool)
    .await
    .context("enregistrement du paquet")?;
    Ok(if existing.is_some() { Saved::Updated } else { Saved::Created })
}

/// Préfixe des uid de règles d'un paquet.
pub fn rule_prefix(id: &str) -> String {
    format!("pack:{id}:")
}

/// Supprime un paquet et ses règles ; faux s'il n'existait pas.
pub async fn remove(pool: &SqlitePool, id: &str) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let prefix = rule_prefix(id);
    sqlx::query("DELETE FROM alert_rules WHERE substr(uid, 1, ?) = ?")
        .bind(prefix.len() as i64)
        .bind(&prefix)
        .execute(&mut *tx)
        .await
        .context("suppression des règles du paquet")?;
    sqlx::query("DELETE FROM rule_overrides WHERE substr(rule_uid, 1, ?) = ?")
        .bind(prefix.len() as i64)
        .bind(&prefix)
        .execute(&mut *tx)
        .await
        .context("suppression des seuils du paquet")?;
    let removed = sqlx::query("DELETE FROM packs WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .context("suppression du paquet")?
        .rows_affected()
        > 0;
    tx.commit().await?;
    Ok(removed)
}

pub async fn set_enabled(pool: &SqlitePool, id: &str, enabled: bool) -> Result<bool> {
    Ok(sqlx::query("UPDATE packs SET enabled = ? WHERE id = ?")
        .bind(i64::from(enabled))
        .bind(id)
        .execute(pool)
        .await
        .context("activation du paquet")?
        .rows_affected()
        > 0)
}

/// Nombre d'équipements qui utilisent le type d'un paquet.
pub async fn targets_using(pool: &SqlitePool, id: &str) -> Result<i64> {
    Ok(sqlx::query("SELECT COUNT(*) AS n FROM targets WHERE kind = ?")
        .bind(format!("{KIND_PREFIX}{id}"))
        .fetch_one(pool)
        .await
        .context("décompte des équipements du paquet")?
        .try_get("n")?)
}

/// Installe les règles d'un paquet qui n'existent pas encore.
///
/// `INSERT OR IGNORE` sur `uid`, comme les règles livrées : une règle que
/// l'utilisateur a modifiée n'est jamais réécrite par une mise à jour du paquet.
pub async fn install_rules(pool: &SqlitePool, pack: &Pack) -> Result<usize> {
    let selector = serde_json::to_string(&TargetSelector::All)?;
    let params = serde_json::to_string(&AnomalyParams::default())?;
    let mut inserted = 0;
    for rule in pack.rules() {
        inserted += insert_rule(pool, rule, &selector, &params).await?;
    }
    Ok(inserted)
}

async fn insert_rule(
    pool: &SqlitePool,
    rule: &PackRule,
    selector: &str,
    params: &str,
) -> Result<usize> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO alert_rules
             (uid, name, description, kind, query, operator, threshold, clear_threshold,
              for_secs, severity, selector, channels, params, unit, repeat_secs,
              escalate_after_secs, enabled, builtin)
         VALUES (?, ?, ?, 'threshold', ?, ?, ?, NULL, ?, ?, ?, '[]', ?, ?, ?, NULL, 1, 0)",
    )
    .bind(&rule.uid)
    .bind(&rule.title)
    .bind(&rule.description)
    .bind(&rule.expr)
    .bind(&rule.op)
    .bind(rule.threshold)
    .bind(rule.for_secs as i64)
    .bind(&rule.severity)
    .bind(selector)
    .bind(params)
    .bind(&rule.unit)
    .bind(REPEAT.as_secs() as i64)
    .execute(pool)
    .await
    .with_context(|| format!("insertion de la règle « {} »", rule.uid))?;
    Ok(result.rows_affected() as usize)
}

/// Les paquets activés, relus et vérifiés. Un paquet devenu illisible (format
/// resserré par une nouvelle version, par exemple) est signalé et ignoré :
/// les autres restent en service.
pub async fn load_enabled(pool: &SqlitePool) -> Result<Vec<Arc<Pack>>> {
    let mut packs = Vec::new();
    for row in list(pool).await? {
        if !row.enabled {
            continue;
        }
        match Pack::parse(&row.yaml) {
            Ok(pack) => packs.push(Arc::new(pack)),
            Err(error) => {
                warn!(pack = %row.id, %error, "paquet d'intégration ignoré : il ne passe plus la vérification")
            }
        }
    }
    Ok(packs)
}

/// Enregistre le collecteur d'un paquet ; sans effet pour un paquet SNMP seul.
pub fn activate(registry: &Registry, pack: &Arc<Pack>) -> Result<(), String> {
    if !pack.has_collector() {
        return Ok(());
    }
    registry.register_runtime(Arc::new(PackCollector::new(pack.clone())))
}

pub fn deactivate(registry: &Registry, id: &str) {
    registry.unregister_runtime(&format!("{KIND_PREFIX}{id}"));
}

/// Aligne le registre sur la table : chaque paquet activé est enregistré, tout
/// type `pack.*` qui n'y correspond plus est retiré. Sert au démarrage et après
/// une restauration.
pub async fn sync_registry(pool: &SqlitePool, registry: &Registry) -> Result<()> {
    let packs = load_enabled(pool).await?;
    let wanted: HashSet<String> = packs.iter().map(|pack| pack.kind().to_string()).collect();
    for kind in registry.runtime_kinds() {
        if kind.starts_with(KIND_PREFIX) && !wanted.contains(&*kind) {
            registry.unregister_runtime(&kind);
        }
    }
    for pack in &packs {
        if let Err(error) = activate(registry, pack) {
            warn!(pack = pack.id(), %error, "paquet d'intégration non enregistré");
        }
    }
    if !packs.is_empty() {
        info!(packs = ?packs.iter().map(|p| p.id()).collect::<Vec<_>>(), "integration packs loaded");
    }
    Ok(())
}

/// Le catalogue SNMP livré, complété des profils des paquets activés.
///
/// Lu une fois au démarrage : un profil SNMP installé ensuite attend le
/// prochain redémarrage (l'API le dit).
pub fn snmp_catalog(base: &Catalog, packs: &[Arc<Pack>]) -> Option<Catalog> {
    let profiles: Vec<_> = packs.iter().flat_map(|pack| pack.snmp_profiles()).collect();
    if profiles.is_empty() {
        return None;
    }
    let mut catalog = base.clone();
    for profile in profiles {
        if let Err(error) = catalog.add_source(&profile.id, &profile.source) {
            warn!(profile = %profile.id, %error, "profil SNMP de paquet ignoré");
        }
    }
    Some(catalog)
}

/// Seuils réglés sur l'équipement : une règle de paquet dont le seuil vient
/// d'une option (`threshold: "{{option.x}}"`) prend, pour chaque équipement, la
/// valeur de cette option. Écrit comme une surcharge de seuil ; l'option de
/// l'équipement a le dernier mot, une option vidée rend le seuil de la règle.
pub async fn apply_thresholds(pool: &SqlitePool, target: &Target) {
    let Some(id) = dumbmonit_pack::id_from_kind(&target.kind) else { return };
    let Ok(Some(row)) = get(pool, id).await else { return };
    let Ok(pack) = Pack::parse(&row.yaml) else { return };
    for rule in pack.rules() {
        let Some(option) = &rule.threshold_option else { continue };
        let value = target.tags.get(option).and_then(|raw| raw.trim().parse::<f64>().ok());
        if let Err(error) = write_threshold(pool, &rule.uid, target.id, value).await {
            warn!(rule = %rule.uid, target_id = target.id, %error, "seuil de paquet non enregistré");
        }
    }
}

async fn write_threshold(
    pool: &SqlitePool,
    rule_uid: &str,
    target_id: i64,
    threshold: Option<f64>,
) -> Result<()> {
    match threshold {
        Some(value) => {
            sqlx::query(
                "INSERT INTO rule_overrides (rule_uid, target_id, threshold) VALUES (?, ?, ?)
                 ON CONFLICT(rule_uid, target_id) DO UPDATE SET threshold = excluded.threshold,
                     updated_at = datetime('now')",
            )
            .bind(rule_uid)
            .bind(target_id)
            .bind(value)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query(
                "UPDATE rule_overrides SET threshold = NULL, updated_at = datetime('now')
                 WHERE rule_uid = ? AND target_id = ?",
            )
            .bind(rule_uid)
            .bind(target_id)
            .execute(pool)
            .await?;
            sqlx::query(
                "DELETE FROM rule_overrides WHERE rule_uid = ? AND target_id = ?
                     AND threshold IS NULL AND clear_threshold IS NULL AND enabled IS NULL",
            )
            .bind(rule_uid)
            .bind(target_id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
