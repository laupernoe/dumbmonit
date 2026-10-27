//! Application d'un fichier de configuration.
//!
//! Tout se passe dans **une** transaction : la restauration des sections
//! déclarées (le code de la sauvegarde, qui crée sans dupliquer), puis la
//! suppression de ce qui est géré par fichier et n'est plus déclaré (`prune`),
//! puis le marquage « géré par fichier ». La moindre ligne refusée — une
//! référence introuvable, une valeur que la base rejette — annule le tout :
//! un fichier à moitié appliqué serait pire qu'un fichier refusé.

use std::net::IpAddr;

use anyhow::Context;
use sqlx::{Row, Sqlite, Transaction};

use super::plan::{self, Plan};
use super::{ConfigDoc, MANAGED_BY_FILE, refs};
use crate::auth::audit;
use crate::backup::bundle::{Bundle, VERSION, target_ref};
use crate::backup::import;
use crate::state::AppState;

/// Un seul `apply` à la fois : l'API et le dossier surveillé peuvent se croiser.
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug)]
pub enum ApplyError {
    /// Le fichier est refusé : le message est destiné à son auteur.
    Invalid(String),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for ApplyError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl From<sqlx::Error> for ApplyError {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(error.into())
    }
}

/// Valide le document et le rapproche de l'instance.
async fn prepare(
    state: &AppState,
    mut desired: ConfigDoc,
) -> Result<(plan::Current, ConfigDoc), ApplyError> {
    desired.validate().map_err(ApplyError::Invalid)?;
    let current = plan::load_current(&state.pool, &state.cipher).await?;
    plan::check_ownership(&current, &desired).map_err(ApplyError::Invalid)?;
    plan::normalize(&mut desired, &current.doc);
    Ok((current, desired))
}

/// Plan d'un document, sans rien écrire. Les références de secrets non résolues
/// sont admises : on suppose le secret inchangé.
pub async fn plan(state: &AppState, desired: ConfigDoc, prune: bool) -> Result<Plan, ApplyError> {
    let (current, desired) = prepare(state, desired).await?;
    Ok(plan::diff(&current, &desired, prune))
}

/// Applique un document. `actor` et `ip` vont au journal d'audit.
pub async fn apply(
    state: &AppState,
    mut desired: ConfigDoc,
    prune: bool,
    actor: &str,
    ip: Option<IpAddr>,
) -> Result<Plan, ApplyError> {
    let open = refs::unresolved(&mut desired);
    if !open.is_empty() {
        return Err(ApplyError::Invalid(format!(
            "Secret references are still unresolved in: {}. Apply the file with dumbmonit-cli \
             (which reads them from its own environment) or from DUMBMONIT_CONFIG_DIR.",
            open.join(", ")
        )));
    }
    let _guard = LOCK.lock().await;
    let (current, desired) = prepare(state, desired).await?;
    let plan = plan::diff(&current, &desired, prune);

    let bundle = Bundle {
        version: VERSION,
        created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        source_version: env!("CARGO_PKG_VERSION").to_string(),
        targets: desired.targets.clone(),
        rules: desired.rules.clone(),
        rule_overrides: desired.rule_overrides.clone(),
        channels: desired.channels.clone(),
        silences: desired.silences.clone(),
        status_pages: desired.status_pages.clone(),
        ..Bundle::default()
    };

    let mut tx = state.pool.begin().await.context("ouverture de la transaction")?;
    let sections = import::restore_config(&mut tx, &state.cipher, &bundle).await?;
    let notes: Vec<&str> =
        sections.iter().flat_map(|s| s.notes.iter().map(String::as_str)).collect();
    if !notes.is_empty() {
        let message = notes.join("\n");
        tx.rollback().await.context("annulation")?;
        return Err(ApplyError::Invalid(message));
    }

    let gone = match prune_managed(&mut tx, &plan).await {
        Ok(gone) => gone,
        Err(error) => {
            let _ = tx.rollback().await;
            return Err(error);
        }
    };
    mark_managed(&mut tx, &desired).await?;
    tx.commit().await.context("validation de la configuration")?;

    // Les domaines publics des pages ont pu changer.
    crate::status_host::invalidate();
    for id in gone {
        crate::api::targets::forget(state, id).await;
    }
    tracing::info!(
        actor,
        create = plan.create,
        update = plan.update,
        delete = plan.delete,
        "configuration applied"
    );
    if !plan.is_empty() {
        let summary =
            format!("create {}, update {}, delete {}", plan.create, plan.update, plan.delete);
        audit::record(&state.pool, Some(actor), "config.applied", Some(&summary), ip).await;
    }
    Ok(plan)
}

/// Supprime ce que le plan retire. Rend les identifiants des équipements
/// supprimés, dont il faut encore oublier alertes, captures et séries.
async fn prune_managed(
    tx: &mut Transaction<'_, Sqlite>,
    plan: &Plan,
) -> Result<Vec<i64>, ApplyError> {
    fn keys<'a>(plan: &'a Plan, section: &str) -> Vec<&'a str> {
        plan.changes
            .iter()
            .filter(|c| c.action == "delete" && c.section == section)
            .map(|c| c.key.as_str())
            .collect()
    }
    let mut targets_gone = Vec::new();

    for slug in keys(plan, "status_pages") {
        sqlx::query("DELETE FROM status_pages WHERE slug = ?")
            .bind(slug)
            .execute(&mut **tx)
            .await?;
        forget_managed(tx, "status_page", slug).await?;
    }
    for key in keys(plan, "silences") {
        sqlx::query(
            "DELETE FROM silences WHERE id IN (
                 SELECT s.id FROM silences s WHERE CASE WHEN s.target_id IS NULL THEN s.name
                 ELSE s.name || '|' || (SELECT t.kind || '|' || t.address FROM targets t
                                        WHERE t.id = s.target_id) END = ?)",
        )
        .bind(key)
        .execute(&mut **tx)
        .await?;
        forget_managed(tx, "silence", key).await?;
    }
    for key in keys(plan, "rule_overrides") {
        if let Some((uid, reference)) = key.split_once('|') {
            sqlx::query(
                "DELETE FROM rule_overrides WHERE rule_uid = ?
                 AND target_id = (SELECT id FROM targets WHERE kind || '|' || address = ?)",
            )
            .bind(uid)
            .bind(reference)
            .execute(&mut **tx)
            .await?;
        }
        forget_managed(tx, "override", key).await?;
    }
    for uid in keys(plan, "rules") {
        sqlx::query("DELETE FROM alert_rules WHERE uid = ? AND builtin = 0")
            .bind(uid)
            .execute(&mut **tx)
            .await?;
        crate::db::alerts::forget_rule_states(tx, uid, None).await?;
        sqlx::query("DELETE FROM rule_overrides WHERE rule_uid = ?")
            .bind(uid)
            .execute(&mut **tx)
            .await?;
        forget_managed(tx, "rule", uid).await?;
    }
    for name in keys(plan, "channels") {
        let id: Option<i64> = sqlx::query("SELECT id FROM notification_channels WHERE name = ?")
            .bind(name)
            .fetch_optional(&mut **tx)
            .await?
            .map(|row| row.try_get("id"))
            .transpose()?;
        let Some(id) = id else { continue };
        let users = sqlx::query("SELECT name, channels FROM alert_rules")
            .fetch_all(&mut **tx)
            .await?
            .iter()
            .filter(|row| {
                let ids: Vec<i64> = row
                    .try_get::<String, _>("channels")
                    .ok()
                    .and_then(|raw| serde_json::from_str(&raw).ok())
                    .unwrap_or_default();
                ids.contains(&id)
            })
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect::<Vec<_>>();
        if !users.is_empty() {
            return Err(ApplyError::Invalid(format!(
                "The channel \"{name}\" cannot be deleted: still used by {}.",
                users.join(", ")
            )));
        }
        sqlx::query("DELETE FROM notification_channels WHERE id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        forget_managed(tx, "channel", name).await?;
    }
    for reference in keys(plan, "targets") {
        let id: Option<i64> = sqlx::query(
            "SELECT id FROM targets WHERE kind || '|' || address = ? AND managed_by = ?",
        )
        .bind(reference)
        .bind(MANAGED_BY_FILE)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| row.try_get("id"))
        .transpose()?;
        let Some(id) = id else { continue };
        sqlx::query("DELETE FROM targets WHERE id = ?").bind(id).execute(&mut **tx).await?;
        targets_gone.push(id);
    }
    Ok(targets_gone)
}

async fn forget_managed(
    tx: &mut Transaction<'_, Sqlite>,
    kind: &str,
    key: &str,
) -> Result<(), ApplyError> {
    sqlx::query("DELETE FROM config_managed WHERE kind = ? AND key = ?")
        .bind(kind)
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Inscrit tout ce que le document déclare comme géré par fichier.
async fn mark_managed(
    tx: &mut Transaction<'_, Sqlite>,
    desired: &ConfigDoc,
) -> Result<(), ApplyError> {
    for target in &desired.targets {
        let reference = target_ref(&target.kind, &target.address);
        sqlx::query(
            "UPDATE targets SET managed_by = ?, managed_source = NULL, managed_key = ?,
                 managed_missing_since = NULL
             WHERE kind = ? AND address = ? AND (managed_by IS NULL OR managed_by = ?)",
        )
        .bind(MANAGED_BY_FILE)
        .bind(&reference)
        .bind(&target.kind)
        .bind(&target.address)
        .bind(MANAGED_BY_FILE)
        .execute(&mut **tx)
        .await?;
    }
    let mut entries: Vec<(&str, String)> = Vec::new();
    entries.extend(desired.channels.iter().map(|c| ("channel", c.name.clone())));
    entries.extend(desired.rules.iter().map(|r| ("rule", r.uid.clone())));
    entries.extend(
        desired.rule_overrides.iter().map(|o| ("override", format!("{}|{}", o.rule_uid, o.target))),
    );
    entries.extend(
        desired
            .silences
            .iter()
            .map(|s| ("silence", super::silence_key(&s.name, s.target.as_deref()))),
    );
    entries.extend(desired.status_pages.iter().map(|p| ("status_page", p.slug.clone())));
    for (kind, key) in entries {
        sqlx::query("INSERT OR IGNORE INTO config_managed (kind, key) VALUES (?, ?)")
            .bind(kind)
            .bind(key)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// Rend tous les objets à la gestion manuelle. Ne supprime rien.
pub async fn release_all(state: &AppState) -> anyhow::Result<u64> {
    let _guard = LOCK.lock().await;
    let mut tx = state.pool.begin().await?;
    let tables = sqlx::query("DELETE FROM config_managed").execute(&mut *tx).await?.rows_affected();
    let targets = sqlx::query(
        "UPDATE targets SET managed_by = NULL, managed_key = NULL, managed_missing_since = NULL
         WHERE managed_by = ?",
    )
    .bind(MANAGED_BY_FILE)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(tables + targets)
}

/// Combien d'objets sont gérés par fichier, par section.
pub async fn managed_counts(
    state: &AppState,
) -> anyhow::Result<std::collections::BTreeMap<String, i64>> {
    let mut counts = std::collections::BTreeMap::new();
    for row in sqlx::query("SELECT kind, COUNT(*) AS n FROM config_managed GROUP BY kind")
        .fetch_all(&state.pool)
        .await?
    {
        counts.insert(row.try_get::<String, _>("kind")?, row.try_get::<i64, _>("n")?);
    }
    let targets: i64 = sqlx::query("SELECT COUNT(*) AS n FROM targets WHERE managed_by = ?")
        .bind(MANAGED_BY_FILE)
        .fetch_one(&state.pool)
        .await?
        .try_get("n")?;
    if targets > 0 {
        counts.insert("target".to_string(), targets);
    }
    Ok(counts)
}

/// Les clés des objets gérés par fichier, par nature (hors équipements, dont la
/// vue porte déjà `managed`) : de quoi afficher un badge dans les listes.
pub async fn managed_keys(
    state: &AppState,
) -> anyhow::Result<std::collections::BTreeMap<String, Vec<String>>> {
    let mut keys: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for row in sqlx::query("SELECT kind, key FROM config_managed ORDER BY kind, key")
        .fetch_all(&state.pool)
        .await?
    {
        keys.entry(row.try_get::<String, _>("kind")?)
            .or_default()
            .push(row.try_get::<String, _>("key")?);
    }
    Ok(keys)
}
