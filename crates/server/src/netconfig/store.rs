//! Tables `netconfig_state` et `netconfig_versions` (migration 0175).

use anyhow::{Context, Result};
use dumbmonit_proto::TargetId;
use sqlx::{Row, SqlitePool};

/// Au-delà, un verrou de relevé est tenu pour abandonné (serveur arrêté au
/// milieu) : un relevé dure au plus quelques minutes.
const STALE_LOCK: &str = "-15 minutes";

/// État d'un équipement.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub host_key: Option<String>,
    pub host_key_pinned_at: Option<String>,
    pub host_key_seen: Option<String>,
    pub host_key_seen_at: Option<String>,
    pub running_since: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub last_error_kind: Option<String>,
    pub last_changed: bool,
}

/// Une version, sans son contenu.
#[derive(Debug, Clone)]
pub struct Version {
    pub id: i64,
    pub fetched_at: String,
    pub last_seen_at: String,
    pub profile: String,
    pub digest: String,
    pub size_bytes: i64,
    pub lines: i64,
    pub added: i64,
    pub removed: i64,
}

/// Ce qu'il faut pour enregistrer une version.
pub struct NewVersion<'a> {
    pub profile: &'a str,
    pub digest: &'a str,
    pub size_bytes: i64,
    pub lines: i64,
    pub added: i64,
    pub removed: i64,
    pub content_enc: &'a [u8],
}

async fn ensure(pool: &SqlitePool, id: TargetId) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO netconfig_state (target_id) VALUES (?)")
        .bind(id)
        .execute(pool)
        .await
        .context("création de l'état netconfig")?;
    Ok(())
}

pub async fn state(pool: &SqlitePool, id: TargetId) -> Result<Option<State>> {
    let row = sqlx::query(
        "SELECT host_key, host_key_pinned_at, host_key_seen, host_key_seen_at, running_since,
                last_attempt_at, last_success_at, last_error, last_error_kind, last_changed
         FROM netconfig_state WHERE target_id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("lecture de l'état netconfig")?;
    let Some(row) = row else { return Ok(None) };
    Ok(Some(State {
        host_key: row.try_get("host_key")?,
        host_key_pinned_at: row.try_get("host_key_pinned_at")?,
        host_key_seen: row.try_get("host_key_seen")?,
        host_key_seen_at: row.try_get("host_key_seen_at")?,
        running_since: row.try_get("running_since")?,
        last_attempt_at: row.try_get("last_attempt_at")?,
        last_success_at: row.try_get("last_success_at")?,
        last_error: row.try_get("last_error")?,
        last_error_kind: row.try_get("last_error_kind")?,
        last_changed: row.try_get::<i64, _>("last_changed")? != 0,
    }))
}

/// Prend le verrou de relevé. Faux si un relevé récent le tient déjà.
pub async fn try_lock(pool: &SqlitePool, id: TargetId) -> Result<bool> {
    ensure(pool, id).await?;
    let done = sqlx::query(
        "UPDATE netconfig_state SET running_since = datetime('now')
         WHERE target_id = ?
           AND (running_since IS NULL OR running_since < datetime('now', ?))",
    )
    .bind(id)
    .bind(STALE_LOCK)
    .execute(pool)
    .await
    .context("verrou de relevé")?;
    Ok(done.rows_affected() == 1)
}

pub async fn unlock(pool: &SqlitePool, id: TargetId) -> Result<()> {
    sqlx::query("UPDATE netconfig_state SET running_since = NULL WHERE target_id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("libération du verrou de relevé")?;
    Ok(())
}

/// Au démarrage : aucun relevé n'est plus en cours.
pub async fn release_all(pool: &SqlitePool) -> Result<()> {
    sqlx::query("UPDATE netconfig_state SET running_since = NULL WHERE running_since IS NOT NULL")
        .execute(pool)
        .await
        .context("libération des verrous de relevé")?;
    Ok(())
}

/// Secondes écoulées depuis la dernière tentative, `None` sans tentative.
pub async fn attempt_age(pool: &SqlitePool, id: TargetId) -> Result<Option<i64>> {
    let age = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT CAST(strftime('%s', 'now') - strftime('%s', last_attempt_at) AS INTEGER)
         FROM netconfig_state WHERE target_id = ? AND last_attempt_at IS NOT NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("âge du dernier relevé")?
    .flatten();
    Ok(age)
}

/// Relevé réussi. `changed` : il a produit une nouvelle version (autre que
/// la toute première).
pub async fn finish_ok(pool: &SqlitePool, id: TargetId, changed: bool) -> Result<()> {
    sqlx::query(
        "UPDATE netconfig_state
         SET last_attempt_at = datetime('now'), last_success_at = datetime('now'),
             last_error = NULL, last_error_kind = NULL, last_changed = ?,
             host_key_seen = NULL, host_key_seen_at = NULL
         WHERE target_id = ?",
    )
    .bind(i64::from(changed))
    .bind(id)
    .execute(pool)
    .await
    .context("fin de relevé")?;
    Ok(())
}

/// Relevé en échec : `kind` vaut `down`, `auth` ou `config`.
pub async fn finish_failed(
    pool: &SqlitePool,
    id: TargetId,
    kind: &str,
    message: &str,
) -> Result<()> {
    ensure(pool, id).await?;
    sqlx::query(
        "UPDATE netconfig_state
         SET last_attempt_at = datetime('now'), last_error = ?, last_error_kind = ?
         WHERE target_id = ?",
    )
    .bind(message)
    .bind(kind)
    .bind(id)
    .execute(pool)
    .await
    .context("échec de relevé")?;
    Ok(())
}

/// Première clé vue : elle devient la clé mémorisée. Sans effet si une clé
/// l'est déjà (une clé différente n'arrive jamais jusqu'ici : la connexion
/// est refusée avant).
pub async fn pin_if_absent(pool: &SqlitePool, id: TargetId, host_key: &str) -> Result<()> {
    ensure(pool, id).await?;
    sqlx::query(
        "UPDATE netconfig_state SET host_key = ?, host_key_pinned_at = datetime('now')
         WHERE target_id = ? AND host_key IS NULL",
    )
    .bind(host_key)
    .bind(id)
    .execute(pool)
    .await
    .context("mémorisation de la clé d'hôte")?;
    Ok(())
}

/// Une clé différente de la clé mémorisée a été présentée.
pub async fn host_key_mismatch(pool: &SqlitePool, id: TargetId, seen: &str) -> Result<()> {
    ensure(pool, id).await?;
    sqlx::query(
        "UPDATE netconfig_state
         SET last_attempt_at = datetime('now'),
             host_key_seen_at = CASE WHEN host_key_seen IS ? THEN host_key_seen_at
                                     ELSE datetime('now') END,
             host_key_seen = ?,
             last_error = 'The SSH host key changed: the connection was refused before \
                           sending any credential.',
             last_error_kind = 'host_key'
         WHERE target_id = ?",
    )
    .bind(seen)
    .bind(seen)
    .bind(id)
    .execute(pool)
    .await
    .context("clé d'hôte changée")?;
    Ok(())
}

/// Un administrateur accepte la clé `seen` présentée en dernier : elle
/// devient la clé mémorisée. Faux si ce n'est plus celle-là (une autre a été
/// vue entre-temps) : rien n'est changé.
pub async fn accept_host_key(pool: &SqlitePool, id: TargetId, seen: &str) -> Result<bool> {
    let done = sqlx::query(
        "UPDATE netconfig_state
         SET host_key = host_key_seen, host_key_pinned_at = datetime('now'),
             host_key_seen = NULL, host_key_seen_at = NULL,
             last_error = NULL, last_error_kind = NULL
         WHERE target_id = ? AND host_key_seen = ?",
    )
    .bind(id)
    .bind(seen)
    .execute(pool)
    .await
    .context("acceptation de la clé d'hôte")?;
    Ok(done.rows_affected() == 1)
}

fn version_row(row: &sqlx::sqlite::SqliteRow) -> Result<Version> {
    Ok(Version {
        id: row.try_get("id")?,
        fetched_at: row.try_get("fetched_at")?,
        last_seen_at: row.try_get("last_seen_at")?,
        profile: row.try_get("profile")?,
        digest: row.try_get("digest")?,
        size_bytes: row.try_get("size_bytes")?,
        lines: row.try_get("lines")?,
        added: row.try_get("added")?,
        removed: row.try_get("removed")?,
    })
}

/// Colonnes d'une [`Version`], en littéral : `sqlx` n'accepte que du SQL
/// statique, et `concat!` en garde la garantie.
macro_rules! version_columns {
    () => {
        "id, fetched_at, last_seen_at, profile, digest, size_bytes, lines, added, removed"
    };
}

/// Versions d'un équipement, la plus récente d'abord.
pub async fn versions(pool: &SqlitePool, id: TargetId, limit: i64) -> Result<Vec<Version>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        version_columns!(),
        " FROM netconfig_versions WHERE target_id = ? ORDER BY id DESC LIMIT ?"
    ))
    .bind(id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("lecture des versions")?;
    rows.iter().map(version_row).collect()
}

pub async fn version_count(pool: &SqlitePool, id: TargetId) -> Result<i64> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM netconfig_versions WHERE target_id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .context("décompte des versions")?;
    Ok(count)
}

pub async fn version(pool: &SqlitePool, id: TargetId, version_id: i64) -> Result<Option<Version>> {
    let row = sqlx::query(concat!(
        "SELECT ",
        version_columns!(),
        " FROM netconfig_versions WHERE target_id = ? AND id = ?"
    ))
    .bind(id)
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .context("lecture d'une version")?;
    row.as_ref().map(version_row).transpose()
}

/// La version qui précède `version_id`.
pub async fn previous(pool: &SqlitePool, id: TargetId, version_id: i64) -> Result<Option<Version>> {
    let row = sqlx::query(concat!(
        "SELECT ",
        version_columns!(),
        " FROM netconfig_versions WHERE target_id = ? AND id < ? ORDER BY id DESC LIMIT 1"
    ))
    .bind(id)
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .context("lecture de la version précédente")?;
    row.as_ref().map(version_row).transpose()
}

pub async fn latest(pool: &SqlitePool, id: TargetId) -> Result<Option<Version>> {
    Ok(versions(pool, id, 1).await?.into_iter().next())
}

/// Contenu chiffré d'une version.
pub async fn content(pool: &SqlitePool, id: TargetId, version_id: i64) -> Result<Option<Vec<u8>>> {
    let content: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT content_enc FROM netconfig_versions WHERE target_id = ? AND id = ?",
    )
    .bind(id)
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .context("lecture du contenu d'une version")?;
    Ok(content)
}

/// Relevé identique à la dernière version : elle est simplement revue.
pub async fn touch(pool: &SqlitePool, version_id: i64) -> Result<()> {
    sqlx::query("UPDATE netconfig_versions SET last_seen_at = datetime('now') WHERE id = ?")
        .bind(version_id)
        .execute(pool)
        .await
        .context("version revue")?;
    Ok(())
}

pub async fn insert(pool: &SqlitePool, id: TargetId, version: &NewVersion<'_>) -> Result<i64> {
    let done = sqlx::query(
        "INSERT INTO netconfig_versions
             (target_id, profile, digest, size_bytes, lines, added, removed, content_enc)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(version.profile)
    .bind(version.digest)
    .bind(version.size_bytes)
    .bind(version.lines)
    .bind(version.added)
    .bind(version.removed)
    .bind(version.content_enc)
    .execute(pool)
    .await
    .context("enregistrement d'une version")?;
    Ok(done.last_insert_rowid())
}

/// Rétention : au plus `keep` versions, et aucune qui n'a plus été vue depuis
/// `keep_days` jours (0 : pas de limite d'âge). La dernière version reste
/// toujours. Rend le nombre de versions supprimées.
pub async fn prune(pool: &SqlitePool, id: TargetId, keep: i64, keep_days: i64) -> Result<u64> {
    let mut removed = sqlx::query(
        "DELETE FROM netconfig_versions
         WHERE target_id = ?
           AND id NOT IN (SELECT id FROM netconfig_versions
                          WHERE target_id = ? ORDER BY id DESC LIMIT ?)",
    )
    .bind(id)
    .bind(id)
    .bind(keep.max(1))
    .execute(pool)
    .await
    .context("rétention des versions (nombre)")?
    .rows_affected();
    if keep_days > 0 {
        removed += sqlx::query(
            "DELETE FROM netconfig_versions
             WHERE target_id = ?
               AND last_seen_at < datetime('now', ?)
               AND id <> (SELECT MAX(id) FROM netconfig_versions WHERE target_id = ?)",
        )
        .bind(id)
        .bind(format!("-{keep_days} days"))
        .bind(id)
        .execute(pool)
        .await
        .context("rétention des versions (âge)")?
        .rows_affected();
    }
    Ok(removed)
}

/// Clés d'hôte mémorisées de tous les équipements, pour la sauvegarde.
pub async fn pinned_keys(pool: &SqlitePool) -> Result<Vec<(TargetId, String)>> {
    let rows = sqlx::query(
        "SELECT target_id, host_key FROM netconfig_state WHERE host_key IS NOT NULL
         ORDER BY target_id",
    )
    .fetch_all(pool)
    .await
    .context("lecture des clés d'hôte")?;
    rows.iter().map(|row| Ok((row.try_get("target_id")?, row.try_get("host_key")?))).collect()
}
