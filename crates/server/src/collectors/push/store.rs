//! Persistance des moniteurs en poussée (table `push_monitors`).

use anyhow::{Context, Result};
use dumbmonit_proto::TargetId;
use sqlx::{Row, SqlitePool};

use crate::crypto::Cipher;

use super::token;

/// Longueur maximale conservée pour le message libre d'un appel. Au-delà, c'est
/// un journal qu'on nous envoie, pas un mot d'explication.
pub const MAX_MESSAGE_CHARS: usize = 500;

/// Ce que le dernier appel a déclaré de lui-même.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Up,
    Down,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }

    fn parse(raw: &str) -> Self {
        if raw == "down" { Self::Down } else { Self::Up }
    }
}

/// Un moniteur tel qu'il est en base, jeton déchiffré.
#[derive(Debug, Clone)]
pub struct Monitor {
    pub target_id: TargetId,
    pub token: String,
    pub last_seen_ms: Option<i64>,
    pub last_seen_at: Option<String>,
    pub last_status: Status,
    pub last_message: String,
    pub received_total: i64,
    pub created_at: String,
}

/// Le moniteur d'une cible, créé avec un jeton neuf s'il n'existe pas encore.
///
/// Créer à la demande plutôt qu'à l'insertion de la cible : une cible peut
/// changer de type, ou avoir été créée par un client qui ignore ce module. La
/// page de l'équipement obtient ainsi toujours une URL.
pub async fn ensure(pool: &SqlitePool, cipher: &Cipher, target_id: TargetId) -> Result<Monitor> {
    if let Some(monitor) = get(pool, cipher, target_id).await? {
        return Ok(monitor);
    }
    let token = token::generate();
    sqlx::query(
        "INSERT OR IGNORE INTO push_monitors (target_id, token_hash, token_enc) VALUES (?, ?, ?)",
    )
    .bind(target_id)
    .bind(token::fingerprint(&token))
    .bind(cipher.encrypt(token.as_bytes())?)
    .execute(pool)
    .await
    .context("creating the push monitor")?;
    get(pool, cipher, target_id).await?.context("push monitor missing after its creation")
}

/// Remplace le jeton : l'ancienne URL cesse de répondre immédiatement. Le
/// compteur et le dernier appel sont conservés — c'est le même travail qu'on
/// surveille, seule la clé change.
pub async fn regenerate(
    pool: &SqlitePool,
    cipher: &Cipher,
    target_id: TargetId,
) -> Result<Monitor> {
    ensure(pool, cipher, target_id).await?;
    let token = token::generate();
    sqlx::query("UPDATE push_monitors SET token_hash = ?, token_enc = ? WHERE target_id = ?")
        .bind(token::fingerprint(&token))
        .bind(cipher.encrypt(token.as_bytes())?)
        .bind(target_id)
        .execute(pool)
        .await
        .context("regenerating the push token")?;
    get(pool, cipher, target_id).await?.context("push monitor missing after its regeneration")
}

pub async fn get(
    pool: &SqlitePool,
    cipher: &Cipher,
    target_id: TargetId,
) -> Result<Option<Monitor>> {
    let row = sqlx::query(
        "SELECT target_id, token_enc, last_seen_ms, last_seen_at, last_status, last_message,
                received_total, created_at
         FROM push_monitors WHERE target_id = ?",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .context("reading the push monitor")?;
    row.map(|row| row_to_monitor(&row, cipher)).transpose()
}

/// État de fraîcheur d'une cible, sans déchiffrer le jeton : c'est ce que le
/// planificateur relit à chaque cycle.
#[derive(Debug, Clone)]
pub struct Freshness {
    pub last_seen_ms: Option<i64>,
    pub last_status: Status,
    pub received_total: i64,
    /// Création du moniteur, en millisecondes depuis l'époque Unix.
    pub created_ms: i64,
}

pub async fn freshness(pool: &SqlitePool, target_id: TargetId) -> Result<Option<Freshness>> {
    let row = sqlx::query(
        "SELECT last_seen_ms, last_status, received_total,
                CAST(strftime('%s', created_at) AS INTEGER) * 1000 AS created_ms
         FROM push_monitors WHERE target_id = ?",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .context("reading the push monitor freshness")?;
    row.map(|row| {
        let last_status: String = row.try_get("last_status")?;
        Ok(Freshness {
            last_seen_ms: row.try_get("last_seen_ms")?,
            last_status: Status::parse(&last_status),
            received_total: row.try_get("received_total")?,
            created_ms: row.try_get::<Option<i64>, _>("created_ms")?.unwrap_or(0),
        })
    })
    .transpose()
}

/// Enregistre un appel. Renvoie la cible concernée, ou `None` si aucun moniteur
/// ne porte cette empreinte — un jeton inconnu ou régénéré.
///
/// Une seule requête, atomique : l'empreinte est cherchée et la ligne mise à
/// jour d'un coup, sans fenêtre entre la vérification et l'écriture.
pub async fn record_ping(
    pool: &SqlitePool,
    token_hash: &str,
    received_at_ms: i64,
    status: Status,
    message: &str,
) -> Result<Option<TargetId>> {
    let message: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
    let row = sqlx::query(
        "UPDATE push_monitors
         SET last_seen_ms = ?,
             last_seen_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
             last_status = ?,
             last_message = ?,
             received_total = received_total + 1
         WHERE token_hash = ?
         RETURNING target_id",
    )
    .bind(received_at_ms)
    .bind(status.as_str())
    .bind(message)
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .context("recording the push")?;
    row.map(|row| row.try_get("target_id").context("target of the push monitor")).transpose()
}

fn row_to_monitor(row: &sqlx::sqlite::SqliteRow, cipher: &Cipher) -> Result<Monitor> {
    let token_enc: Vec<u8> = row.try_get("token_enc")?;
    let token = String::from_utf8(cipher.decrypt(&token_enc)?).context("push token is not text")?;
    let last_status: String = row.try_get("last_status")?;
    Ok(Monitor {
        target_id: row.try_get("target_id")?,
        token,
        last_seen_ms: row.try_get("last_seen_ms")?,
        last_seen_at: row.try_get("last_seen_at")?,
        last_status: Status::parse(&last_status),
        last_message: row.try_get("last_message")?,
        received_total: row.try_get("received_total")?,
        created_at: row.try_get("created_at")?,
    })
}

// ------------------------------------------------------------- exécutions

/// Taille maximale conservée pour la sortie d'une exécution, en octets.
pub const MAX_OUTPUT_BYTES: usize = 10 * 1024;
/// Exécutions gardées par cible, les plus récentes.
pub const KEEP_RUNS: i64 = 200;
/// Âge au-delà duquel une exécution est purgée, en jours.
pub const KEEP_DAYS: i64 = 90;

/// Tronque une sortie à [`MAX_OUTPUT_BYTES`] sans casser l'UTF-8.
///
/// Les octets invalides sont remplacés (`U+FFFD`). Quand la sortie est trop
/// longue, c'est la fin qu'on garde : l'erreur est presque toujours dans les
/// dernières lignes.
pub fn truncate_output(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    if text.len() <= MAX_OUTPUT_BYTES {
        return text.into_owned();
    }
    let mut start = text.len() - MAX_OUTPUT_BYTES;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    /// Commencée puis remplacée par un nouveau `/start` sans jamais finir.
    Lost,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Lost => "lost",
        }
    }

    fn parse(raw: &str) -> Self {
        match raw {
            "running" => Self::Running,
            "failed" => Self::Failed,
            "lost" => Self::Lost,
            _ => Self::Success,
        }
    }
}

/// Une exécution du travail surveillé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: i64,
    pub target_id: TargetId,
    pub started_ms: Option<i64>,
    pub finished_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub status: RunStatus,
    pub exit_code: Option<i64>,
    pub output: String,
}

/// Colonnes lues d'une exécution (littéral : le SQL est une chaîne statique).
macro_rules! run_columns {
    () => {
        "id, target_id, started_ms, finished_ms, duration_ms, status, exit_code, output"
    };
}

fn row_to_run(row: &sqlx::sqlite::SqliteRow) -> Result<Run> {
    let status: String = row.try_get("status")?;
    Ok(Run {
        id: row.try_get("id")?,
        target_id: row.try_get("target_id")?,
        started_ms: row.try_get("started_ms")?,
        finished_ms: row.try_get("finished_ms")?,
        duration_ms: row.try_get("duration_ms")?,
        status: RunStatus::parse(&status),
        exit_code: row.try_get("exit_code")?,
        output: row.try_get("output")?,
    })
}

/// Ouvre une exécution. Celle qui tournait encore est déclarée perdue : un
/// travail qui recommence n'a pas fini le précédent.
pub async fn start_run(pool: &SqlitePool, target_id: TargetId, now_ms: i64) -> Result<()> {
    sqlx::query("UPDATE push_runs SET status = 'lost' WHERE target_id = ? AND status = 'running'")
        .bind(target_id)
        .execute(pool)
        .await
        .context("closing the previous run")?;
    sqlx::query("INSERT INTO push_runs (target_id, started_ms, status) VALUES (?, ?, 'running')")
        .bind(target_id)
        .bind(now_ms)
        .execute(pool)
        .await
        .context("opening a run")?;
    purge_runs(pool, target_id, now_ms).await
}

/// Termine l'exécution en cours, ou en crée une sans début (durée inconnue)
/// quand aucun `/start` ne l'a précédée.
pub async fn finish_run(
    pool: &SqlitePool,
    target_id: TargetId,
    now_ms: i64,
    failed: bool,
    exit_code: Option<i64>,
    output: &str,
) -> Result<()> {
    let status = if failed { RunStatus::Failed } else { RunStatus::Success };
    let open: Option<(i64, i64)> = sqlx::query_as(
        "SELECT id, started_ms FROM push_runs
         WHERE target_id = ? AND status = 'running' ORDER BY id DESC LIMIT 1",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .context("looking for the open run")?;
    match open {
        Some((id, started_ms)) => {
            sqlx::query(
                "UPDATE push_runs
                 SET finished_ms = ?, duration_ms = ?, status = ?, exit_code = ?, output = ?
                 WHERE id = ?",
            )
            .bind(now_ms)
            .bind((now_ms - started_ms).max(0))
            .bind(status.as_str())
            .bind(exit_code)
            .bind(output)
            .bind(id)
            .execute(pool)
            .await
            .context("closing the run")?;
        }
        None => {
            sqlx::query(
                "INSERT INTO push_runs (target_id, finished_ms, status, exit_code, output)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(target_id)
            .bind(now_ms)
            .bind(status.as_str())
            .bind(exit_code)
            .bind(output)
            .execute(pool)
            .await
            .context("recording a run")?;
        }
    }
    purge_runs(pool, target_id, now_ms).await
}

/// Garde les [`KEEP_RUNS`] dernières exécutions de la cible, et rien de plus
/// vieux que [`KEEP_DAYS`].
async fn purge_runs(pool: &SqlitePool, target_id: TargetId, now_ms: i64) -> Result<()> {
    let oldest_ms = now_ms - KEEP_DAYS * 86_400_000;
    sqlx::query(
        "DELETE FROM push_runs
         WHERE target_id = ?
           AND (id NOT IN (SELECT id FROM push_runs WHERE target_id = ? ORDER BY id DESC LIMIT ?)
                OR COALESCE(finished_ms, started_ms, 0) < ?)",
    )
    .bind(target_id)
    .bind(target_id)
    .bind(KEEP_RUNS)
    .bind(oldest_ms)
    .execute(pool)
    .await
    .context("purging old runs")?;
    Ok(())
}

/// Exécutions d'une cible, les plus récentes d'abord. `before` est l'identifiant
/// de la dernière exécution déjà vue (pagination par curseur).
pub async fn list_runs(
    pool: &SqlitePool,
    target_id: TargetId,
    before: Option<i64>,
    limit: i64,
) -> Result<Vec<Run>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        run_columns!(),
        " FROM push_runs WHERE target_id = ? AND id < ? ORDER BY id DESC LIMIT ?"
    ))
    .bind(target_id)
    .bind(before.unwrap_or(i64::MAX))
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("reading the runs")?;
    rows.iter().map(row_to_run).collect()
}

/// Ce que le contrôle des exécutions relit à chaque cycle.
#[derive(Debug, Clone, Default)]
pub struct RunState {
    /// Dernière exécution terminée (réussie ou échouée).
    pub last_finished: Option<Run>,
    /// Exécution en cours la plus récente.
    pub running: Option<Run>,
}

pub async fn run_state(pool: &SqlitePool, target_id: TargetId) -> Result<RunState> {
    let finished = sqlx::query(concat!(
        "SELECT ",
        run_columns!(),
        " FROM push_runs WHERE target_id = ? AND status IN ('success', 'failed')
         ORDER BY id DESC LIMIT 1"
    ))
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .context("reading the last finished run")?;
    let running = sqlx::query(concat!(
        "SELECT ",
        run_columns!(),
        " FROM push_runs WHERE target_id = ? AND status = 'running' ORDER BY id DESC LIMIT 1"
    ))
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .context("reading the running run")?;
    Ok(RunState {
        last_finished: finished.as_ref().map(row_to_run).transpose()?,
        running: running.as_ref().map(row_to_run).transpose()?,
    })
}

/// Dernière exécution terminée de chaque cible dont elle a échoué : la sortie
/// que l'alerte « échec » cite.
pub async fn failed_last_runs(pool: &SqlitePool) -> Result<Vec<Run>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        run_columns!(),
        " FROM push_runs r
         WHERE r.status = 'failed'
           AND r.id = (SELECT MAX(id) FROM push_runs
                       WHERE target_id = r.target_id AND status IN ('success', 'failed'))"
    ))
    .fetch_all(pool)
    .await
    .context("reading the failed runs")?;
    rows.iter().map(row_to_run).collect()
}
