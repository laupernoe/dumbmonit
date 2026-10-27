//! Réglage de la collecte de journaux d'une machine (migration 0130).
//!
//! Le réglage se pose dans l'interface et voyage dans l'accusé de réception des
//! lots de mesures ; l'agent l'applique à chaud. Les lignes reçues partent dans
//! VictoriaLogs (`logs::central`) : ici, seul le suivi « dernière réception ».

use anyhow::{Context, Result};
use dumbmonit_proto::{LogsSetting, TargetId};
use sqlx::{Row, SqlitePool};

/// Fichiers suivis au plus, par machine.
pub const MAX_FILES: usize = 20;
/// Canaux du Journal d'événements au plus, par machine.
pub const MAX_EVENTLOG: usize = 10;
/// Longueur maximale d'un motif de fichier.
pub const MAX_PATTERN: usize = 300;

/// Réglage de la machine ; désactivé si elle n'en a pas.
pub async fn setting(pool: &SqlitePool, target_id: TargetId) -> Result<LogsSetting> {
    let row = sqlx::query(
        "SELECT enabled, min_level, journald, files, eventlog FROM agent_log_settings \
         WHERE target_id = ?",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .context("reading the log collection setting")?;
    let Some(row) = row else { return Ok(LogsSetting::default()) };
    let files: String = row.try_get("files")?;
    let eventlog: String = row.try_get("eventlog")?;
    Ok(LogsSetting {
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        min_level: row.try_get("min_level")?,
        journald: row.try_get::<i64, _>("journald")? != 0,
        files: serde_json::from_str(&files).unwrap_or_default(),
        eventlog: serde_json::from_str(&eventlog).unwrap_or_default(),
    })
}

/// Vérifie et nettoie une saisie : niveau connu, chemins absolus, canaux sûrs.
pub fn clean(mut setting: LogsSetting) -> Result<LogsSetting, String> {
    if !["error", "warn", "info", "debug"].contains(&setting.min_level.as_str()) {
        return Err("The minimum level must be error, warn, info or debug.".into());
    }
    let mut files: Vec<String> = Vec::new();
    for raw in &setting.files {
        let pattern = raw.trim();
        if pattern.is_empty() || files.iter().any(|f| f == pattern) {
            continue;
        }
        if pattern.len() > MAX_PATTERN || pattern.contains('\0') || pattern.contains('\n') {
            return Err(format!("A file pattern is limited to {MAX_PATTERN} characters."));
        }
        let absolute = pattern.starts_with('/')
            || pattern.starts_with('\\')
            || pattern.chars().nth(1) == Some(':');
        if !absolute {
            return Err(format!("\"{pattern}\" is not an absolute path."));
        }
        if pattern.contains("..") {
            return Err(format!("\"{pattern}\" must not contain \"..\"."));
        }
        files.push(pattern.to_string());
    }
    if files.len() > MAX_FILES {
        return Err(format!("At most {MAX_FILES} file patterns."));
    }
    let mut channels: Vec<String> = Vec::new();
    for raw in &setting.eventlog {
        let channel = raw.trim();
        if channel.is_empty() || channels.iter().any(|c| c == channel) {
            continue;
        }
        let safe = channel.len() <= 128
            && channel.chars().all(|c| c.is_ascii_alphanumeric() || " _-./".contains(c));
        if !safe {
            return Err(format!("\"{channel}\" is not a valid event log channel name."));
        }
        channels.push(channel.to_string());
    }
    if channels.len() > MAX_EVENTLOG {
        return Err(format!("At most {MAX_EVENTLOG} event log channels."));
    }
    setting.files = files;
    setting.eventlog = channels;
    Ok(setting)
}

pub async fn save(pool: &SqlitePool, target_id: TargetId, setting: &LogsSetting) -> Result<()> {
    sqlx::query(
        "INSERT INTO agent_log_settings (target_id, enabled, min_level, journald, files, eventlog) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(target_id) DO UPDATE SET enabled = excluded.enabled, \
             min_level = excluded.min_level, journald = excluded.journald, \
             files = excluded.files, eventlog = excluded.eventlog, updated_at = datetime('now')",
    )
    .bind(target_id)
    .bind(i64::from(setting.enabled))
    .bind(&setting.min_level)
    .bind(i64::from(setting.journald))
    .bind(serde_json::to_string(&setting.files)?)
    .bind(serde_json::to_string(&setting.eventlog)?)
    .execute(pool)
    .await
    .context("saving the log collection setting")?;
    Ok(())
}

/// Dernière réception et total de lignes reçues.
pub async fn status(pool: &SqlitePool, target_id: TargetId) -> Result<(Option<String>, i64)> {
    let row = sqlx::query(
        "SELECT last_received_at, received_total FROM agent_log_settings WHERE target_id = ?",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .context("reading the log collection status")?;
    match row {
        Some(row) => Ok((row.try_get("last_received_at")?, row.try_get("received_total")?)),
        None => Ok((None, 0)),
    }
}

/// Note une réception de `count` lignes.
pub async fn note_received(pool: &SqlitePool, target_id: TargetId, count: i64) -> Result<()> {
    sqlx::query(
        "INSERT INTO agent_log_settings (target_id, last_received_at, received_total) \
         VALUES (?, datetime('now'), ?) \
         ON CONFLICT(target_id) DO UPDATE SET last_received_at = datetime('now'), \
             received_total = received_total + excluded.received_total",
    )
    .bind(target_id)
    .bind(count)
    .execute(pool)
    .await
    .context("noting received log lines")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(files: &[&str], eventlog: &[&str]) -> LogsSetting {
        LogsSetting {
            enabled: true,
            files: files.iter().map(|f| f.to_string()).collect(),
            eventlog: eventlog.iter().map(|c| c.to_string()).collect(),
            ..LogsSetting::default()
        }
    }

    #[test]
    fn a_clean_setting_keeps_absolute_patterns_and_drops_duplicates() {
        let cleaned =
            clean(setting(&[" /var/log/*.log ", "/var/log/*.log", "C:\\logs\\*.txt"], &["System"]))
                .unwrap();
        assert_eq!(cleaned.files, vec!["/var/log/*.log", "C:\\logs\\*.txt"]);
        assert_eq!(cleaned.eventlog, vec!["System"]);
    }

    #[test]
    fn relative_traversing_or_unsafe_entries_are_refused() {
        assert!(clean(setting(&["var/log/x.log"], &[])).is_err());
        assert!(clean(setting(&["/var/log/../shadow"], &[])).is_err());
        assert!(clean(setting(&[], &["System'; calc; '"])).is_err());
        let mut bad = setting(&[], &[]);
        bad.min_level = "loud".into();
        assert!(clean(bad).is_err());
    }
}
