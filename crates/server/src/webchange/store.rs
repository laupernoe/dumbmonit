//! Ce que la surveillance garde : état, pages, instantanés, changements.
//!
//! Le texte des instantanés vit dans SQLite ; les captures d'écran, elles,
//! sont des fichiers sous `<data>/webchange/<cible>/<instantané>.jpg`. Une
//! capture pèse de 100 à 800 Ko : cent changements sur quelques sites, c'est
//! déjà une centaine de mégaoctets, qui gonfleraient la base, son journal WAL
//! et chaque sauvegarde planifiée (`backup::local` copie la base entière, tous
//! les jours) pour des images que l'on peut perdre sans dommage. Le texte, lui,
//! est petit, et c'est lui qui fait le diff : il reste dans la base.
//!
//! Rétention : par cible, les cent derniers changements ; un instantané est
//! gardé tant qu'il est le dernier d'une page ou qu'un changement gardé le
//! cite. Le reste est effacé, fichiers compris, à la fin de chaque vérification.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dumbmonit_proto::TargetId;
use sqlx::{FromRow, SqlitePool};

/// Changements gardés par cible.
pub const KEEP_CHANGES: i64 = 100;
/// Texte gardé au plus par instantané, en octets.
const MAX_CONTENT_BYTES: usize = 1024 * 1024;
/// Un verrou plus vieux que ceci est celui d'une vérification interrompue.
const STALE_LOCK_MINUTES: i64 = 30;

/// Répertoire des captures d'une cible.
pub fn target_dir(data_dir: &Path, target: TargetId) -> PathBuf {
    data_dir.join("webchange").join(target.to_string())
}

/// Fichier de la capture d'un instantané.
pub fn screenshot_path(data_dir: &Path, target: TargetId, snapshot: i64) -> PathBuf {
    target_dir(data_dir, target).join(format!("{snapshot}.jpg"))
}

/// Efface les captures d'une cible supprimée. Les lignes, elles, partent avec
/// la cible (`ON DELETE CASCADE`).
pub async fn forget_files(data_dir: &Path, target: TargetId) {
    let dir = target_dir(data_dir, target);
    match tokio::fs::remove_dir_all(&dir).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(target = target, %error, "screenshots not removed"),
    }
}

#[derive(Debug, Clone, Default, FromRow)]
pub struct State {
    pub fingerprint: Option<String>,
    pub running_since: Option<String>,
    pub last_check_at: Option<String>,
    pub last_check_changes: i64,
    pub last_error: Option<String>,
    pub last_error_kind: Option<String>,
}

pub async fn state(pool: &SqlitePool, target: TargetId) -> Result<Option<State>> {
    sqlx::query_as(
        "SELECT fingerprint, running_since, last_check_at, last_check_changes, last_error,
                last_error_kind
         FROM webchange_state WHERE target_id = ?",
    )
    .bind(target)
    .fetch_optional(pool)
    .await
    .context("lecture de l'état de la surveillance")
}

/// Âge de la dernière vérification, en secondes ; `None` s'il n'y en a pas eu.
pub async fn last_check_age(pool: &SqlitePool, target: TargetId) -> Result<Option<i64>> {
    let age: Option<Option<i64>> = sqlx::query_scalar(
        "SELECT CAST(strftime('%s', 'now') - strftime('%s', last_check_at) AS INTEGER)
         FROM webchange_state WHERE target_id = ? AND last_check_at IS NOT NULL",
    )
    .bind(target)
    .fetch_optional(pool)
    .await
    .context("âge de la dernière vérification")?;
    Ok(age.flatten())
}

/// Prend le verrou de vérification ; faux si une autre la tient déjà.
pub async fn try_lock(pool: &SqlitePool, target: TargetId) -> Result<bool> {
    sqlx::query("INSERT INTO webchange_state (target_id) VALUES (?) ON CONFLICT DO NOTHING")
        .bind(target)
        .execute(pool)
        .await
        .context("création de l'état de la surveillance")?;
    let taken = sqlx::query(
        "UPDATE webchange_state SET running_since = datetime('now')
         WHERE target_id = ?
           AND (running_since IS NULL OR running_since < datetime('now', ?))",
    )
    .bind(target)
    .bind(format!("-{STALE_LOCK_MINUTES} minutes"))
    .execute(pool)
    .await
    .context("verrou de vérification")?;
    Ok(taken.rows_affected() == 1)
}

pub async fn unlock(pool: &SqlitePool, target: TargetId) -> Result<()> {
    sqlx::query("UPDATE webchange_state SET running_since = NULL WHERE target_id = ?")
        .bind(target)
        .execute(pool)
        .await
        .context("libération du verrou de vérification")?;
    Ok(())
}

/// Au démarrage : aucune vérification ne peut être en cours.
pub async fn release_all(pool: &SqlitePool) -> Result<()> {
    sqlx::query("UPDATE webchange_state SET running_since = NULL")
        .execute(pool)
        .await
        .context("libération des verrous de vérification")?;
    Ok(())
}

/// Fin d'une vérification qui a abouti : la référence est prise, les erreurs
/// effacées.
pub async fn finish_ok(
    pool: &SqlitePool,
    target: TargetId,
    fingerprint: &str,
    changes: i64,
    error: Option<(&str, &str)>,
) -> Result<()> {
    sqlx::query(
        "UPDATE webchange_state
         SET fingerprint = ?, last_check_at = datetime('now'), last_check_changes = ?,
             last_error = ?, last_error_kind = ?
         WHERE target_id = ?",
    )
    .bind(fingerprint)
    .bind(changes)
    .bind(error.map(|(_, message)| message))
    .bind(error.map(|(kind, _)| kind))
    .bind(target)
    .execute(pool)
    .await
    .context("fin de vérification")?;
    Ok(())
}

/// Fin d'une vérification qui n'a pas pu lire la page de départ : rien n'est
/// comparé, la référence ne bouge pas.
pub async fn finish_failed(
    pool: &SqlitePool,
    target: TargetId,
    kind: &str,
    message: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE webchange_state
         SET last_check_at = datetime('now'), last_check_changes = 0,
             last_error = ?, last_error_kind = ?
         WHERE target_id = ?",
    )
    .bind(message)
    .bind(kind)
    .bind(target)
    .execute(pool)
    .await
    .context("fin de vérification en échec")?;
    Ok(())
}

/// Une page telle que gardée.
#[derive(Debug, Clone, FromRow)]
pub struct PageRow {
    pub url: String,
    pub title: Option<String>,
    pub status: Option<i64>,
    pub error: Option<String>,
    pub hash: Option<String>,
    pub last_checked: Option<String>,
    pub last_changed: Option<String>,
    pub removed: bool,
    pub latest_snapshot: Option<i64>,
    /// Changements gardés pour cette adresse.
    pub changes: i64,
}

pub async fn pages(pool: &SqlitePool, target: TargetId) -> Result<Vec<PageRow>> {
    sqlx::query_as(
        "SELECT p.url, p.title, p.status, p.error, p.hash, p.last_checked, p.last_changed,
                p.removed, p.latest_snapshot,
                (SELECT COUNT(*) FROM webchange_changes c
                 WHERE c.target_id = p.target_id AND c.url = p.url) AS changes
         FROM webchange_pages p
         WHERE p.target_id = ?
         ORDER BY p.removed, p.id",
    )
    .bind(target)
    .fetch_all(pool)
    .await
    .context("lecture des pages surveillées")
}

/// Pages suivies (non disparues) et, parmi elles, celles en erreur.
pub async fn page_counts(pool: &SqlitePool, target: TargetId) -> Result<(i64, i64)> {
    sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(error IS NOT NULL), 0)
         FROM webchange_pages WHERE target_id = ? AND removed = 0",
    )
    .bind(target)
    .fetch_one(pool)
    .await
    .context("décompte des pages surveillées")
}

/// Ce que la lecture d'une page a donné, à enregistrer sur sa ligne.
pub struct PageUpdate<'a> {
    pub url: &'a str,
    pub title: Option<&'a str>,
    pub status: Option<i64>,
    pub error: Option<&'a str>,
    /// Nouvelle empreinte du texte ; `None` garde la précédente.
    pub hash: Option<&'a str>,
    pub removed: bool,
    /// Nouvel instantané ; `None` garde le précédent.
    pub snapshot: Option<i64>,
    /// Le texte a changé à cette lecture.
    pub changed: bool,
}

pub async fn save_page(pool: &SqlitePool, target: TargetId, page: &PageUpdate<'_>) -> Result<()> {
    sqlx::query(
        "INSERT INTO webchange_pages
            (target_id, url, title, status, error, hash, last_checked, last_changed, removed,
             latest_snapshot)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'), CASE WHEN ? THEN datetime('now') END, ?, ?)
         ON CONFLICT (target_id, url) DO UPDATE SET
            title = COALESCE(excluded.title, webchange_pages.title),
            status = excluded.status,
            error = excluded.error,
            hash = COALESCE(excluded.hash, webchange_pages.hash),
            last_checked = excluded.last_checked,
            last_changed = COALESCE(excluded.last_changed, webchange_pages.last_changed),
            removed = excluded.removed,
            latest_snapshot = COALESCE(excluded.latest_snapshot, webchange_pages.latest_snapshot)",
    )
    .bind(target)
    .bind(page.url)
    .bind(page.title)
    .bind(page.status)
    .bind(page.error)
    .bind(page.hash)
    .bind(page.changed)
    .bind(page.removed)
    .bind(page.snapshot)
    .execute(pool)
    .await
    .context("enregistrement d'une page surveillée")?;
    Ok(())
}

/// Oublie les pages qu'une nouvelle référence ne contient plus (réglages
/// changés) : elles ne relèvent plus de cette surveillance.
pub async fn forget_pages_except(
    pool: &SqlitePool,
    target: TargetId,
    keep: &[String],
) -> Result<()> {
    let known: Vec<String> =
        sqlx::query_scalar("SELECT url FROM webchange_pages WHERE target_id = ?")
            .bind(target)
            .fetch_all(pool)
            .await
            .context("lecture des pages surveillées")?;
    let to_forget: Vec<&String> = known.iter().filter(|url| !keep.contains(url)).collect();
    if to_forget.is_empty() {
        return Ok(());
    }
    // Une transaction plutôt qu'un autocommit par page : ce réglage ne change
    // jamais qu'une poignée de pages à la fois, mais autant suivre la même
    // convention que le reste du nettoyage (`db/alerts.rs`, `db/pbs.rs`, …).
    let mut tx = pool.begin().await.context("ouverture de la transaction d'oubli")?;
    for url in to_forget {
        sqlx::query("DELETE FROM webchange_pages WHERE target_id = ? AND url = ?")
            .bind(target)
            .bind(url)
            .execute(&mut *tx)
            .await
            .context("oubli d'une page")?;
    }
    tx.commit().await.context("validation de l'oubli des pages")?;
    Ok(())
}

pub async fn insert_snapshot(
    pool: &SqlitePool,
    target: TargetId,
    url: &str,
    title: Option<&str>,
    hash: &str,
    content: &str,
) -> Result<i64> {
    let content = truncate(content, MAX_CONTENT_BYTES);
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO webchange_snapshots (target_id, url, title, hash, content)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(target)
    .bind(url)
    .bind(title)
    .bind(hash)
    .bind(content)
    .fetch_one(pool)
    .await
    .context("enregistrement d'un instantané")?;
    Ok(id)
}

pub async fn mark_screenshot(pool: &SqlitePool, snapshot: i64) -> Result<()> {
    sqlx::query("UPDATE webchange_snapshots SET has_screenshot = 1 WHERE id = ?")
        .bind(snapshot)
        .execute(pool)
        .await
        .context("enregistrement d'une capture")?;
    Ok(())
}

/// Texte d'un instantané.
pub async fn snapshot_content(pool: &SqlitePool, snapshot: i64) -> Result<Option<String>> {
    sqlx::query_scalar("SELECT content FROM webchange_snapshots WHERE id = ?")
        .bind(snapshot)
        .fetch_optional(pool)
        .await
        .context("lecture d'un instantané")
}

/// Ce que l'API montre d'un instantané.
#[derive(Debug, Clone, FromRow)]
pub struct SnapshotRow {
    pub id: i64,
    pub fetched_at: String,
    pub title: Option<String>,
    pub has_screenshot: bool,
}

/// Un instantané de la cible ; `None` s'il n'existe pas ou appartient à une
/// autre cible.
pub async fn snapshot(
    pool: &SqlitePool,
    target: TargetId,
    snapshot: i64,
) -> Result<Option<SnapshotRow>> {
    sqlx::query_as(
        "SELECT id, fetched_at, title, has_screenshot FROM webchange_snapshots
         WHERE id = ? AND target_id = ?",
    )
    .bind(snapshot)
    .bind(target)
    .fetch_optional(pool)
    .await
    .context("lecture d'un instantané")
}

pub struct NewChange<'a> {
    pub url: &'a str,
    pub kind: &'a str,
    pub added: i64,
    pub removed: i64,
    pub before: Option<i64>,
    pub after: Option<i64>,
}

pub async fn insert_change(
    pool: &SqlitePool,
    target: TargetId,
    change: &NewChange<'_>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO webchange_changes
            (target_id, url, kind, added, removed, before_snapshot, after_snapshot)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(target)
    .bind(change.url)
    .bind(change.kind)
    .bind(change.added)
    .bind(change.removed)
    .bind(change.before)
    .bind(change.after)
    .execute(pool)
    .await
    .context("enregistrement d'un changement")?;
    Ok(())
}

#[derive(Debug, Clone, FromRow)]
pub struct ChangeRow {
    pub id: i64,
    pub url: String,
    pub kind: String,
    pub detected_at: String,
    pub added: i64,
    pub removed: i64,
    pub before_snapshot: Option<i64>,
    pub after_snapshot: Option<i64>,
}

/// Colonnes d'un changement. Une macro plutôt qu'une constante : `sqlx` 0.9
/// n'accepte que des requêtes littérales, et `concat!` n'assemble que des
/// littéraux.
macro_rules! change_columns {
    () => {
        "id, url, kind, detected_at, added, removed, before_snapshot, after_snapshot"
    };
}

/// Changements de la cible, du plus récent au plus ancien.
pub async fn changes(
    pool: &SqlitePool,
    target: TargetId,
    url: Option<&str>,
    limit: i64,
) -> Result<Vec<ChangeRow>> {
    sqlx::query_as(concat!(
        "SELECT ",
        change_columns!(),
        " FROM webchange_changes
         WHERE target_id = ? AND (? IS NULL OR url = ?)
         ORDER BY id DESC LIMIT ?"
    ))
    .bind(target)
    .bind(url)
    .bind(url)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("lecture des changements")
}

pub async fn change(pool: &SqlitePool, target: TargetId, id: i64) -> Result<Option<ChangeRow>> {
    sqlx::query_as(concat!(
        "SELECT ",
        change_columns!(),
        " FROM webchange_changes WHERE id = ? AND target_id = ?"
    ))
    .bind(id)
    .bind(target)
    .fetch_optional(pool)
    .await
    .context("lecture d'un changement")
}

/// Applique la rétention de la cible, fichiers compris.
pub async fn prune(pool: &SqlitePool, data_dir: &Path, target: TargetId) -> Result<()> {
    sqlx::query(
        "DELETE FROM webchange_changes
         WHERE target_id = ?
           AND id NOT IN (SELECT id FROM webchange_changes WHERE target_id = ?
                          ORDER BY id DESC LIMIT ?)",
    )
    .bind(target)
    .bind(target)
    .bind(KEEP_CHANGES)
    .execute(pool)
    .await
    .context("purge des changements anciens")?;

    let orphans: Vec<(i64, bool)> = sqlx::query_as(
        "SELECT s.id, s.has_screenshot FROM webchange_snapshots s
         WHERE s.target_id = ?
           AND NOT EXISTS (SELECT 1 FROM webchange_pages p
                           WHERE p.target_id = s.target_id AND p.latest_snapshot = s.id)
           AND NOT EXISTS (SELECT 1 FROM webchange_changes c
                           WHERE c.target_id = s.target_id
                             AND (c.before_snapshot = s.id OR c.after_snapshot = s.id))",
    )
    .bind(target)
    .fetch_all(pool)
    .await
    .context("recherche des instantanés à purger")?;
    for (id, has_screenshot) in orphans {
        sqlx::query("DELETE FROM webchange_snapshots WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .context("purge d'un instantané")?;
        if has_screenshot {
            let path = screenshot_path(data_dir, target, id);
            if let Err(error) = tokio::fs::remove_file(&path).await
                && error.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(path = %path.display(), %error, "screenshot not removed");
            }
        }
    }
    Ok(())
}

/// Coupe à `max` octets au plus, sur une frontière de caractère.
fn truncate(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_coupe_respecte_les_caracteres() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("aé", 2), "a");
        assert_eq!(truncate("aéb", 3), "aé");
    }
}
