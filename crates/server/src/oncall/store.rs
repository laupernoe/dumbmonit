//! Persistance des astreintes : plannings, remplacements, politiques, contacts,
//! préférences et journal des escalades.
//!
//! Le module ne raisonne pas : il lit et écrit. Les définitions de plannings et
//! de politiques sont des documents JSON ; un document illisible (version
//! future, édition à la main) retombe sur une définition vide plutôt que de
//! faire échouer toute la lecture.

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::alerting::model::Severity;
use crate::alerting::silence::Schedule;
use crate::crypto::Cipher;
use crate::oncall::escalation::PolicyDef;
use crate::oncall::schedule::{Definition, Override};

pub fn to_sql(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn from_sql(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw.trim()).ok().map(|at| at.with_timezone(&Utc))
}

fn opt_time(row: &SqliteRow, column: &str) -> Result<Option<DateTime<Utc>>> {
    let raw: Option<String> = row.try_get(column)?;
    Ok(raw.as_deref().and_then(from_sql))
}

// --------------------------------------------------------------------------
// Plannings
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ScheduleRow {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub definition: Definition,
    pub created_at: String,
    pub updated_at: String,
}

fn schedule_from_row(row: &SqliteRow) -> Result<ScheduleRow> {
    let raw: String = row.try_get("definition")?;
    Ok(ScheduleRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        definition: serde_json::from_str(&raw).unwrap_or_default(),
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const SCHEDULE_COLUMNS: &str = "id, name, description, definition, created_at, updated_at";

pub async fn list_schedules(pool: &SqlitePool) -> Result<Vec<ScheduleRow>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {SCHEDULE_COLUMNS} FROM oncall_schedules ORDER BY name COLLATE NOCASE"
    )))
    .fetch_all(pool)
    .await
    .context("lecture des plannings d'astreinte")?;
    rows.iter().map(schedule_from_row).collect()
}

pub async fn get_schedule(pool: &SqlitePool, id: i64) -> Result<Option<ScheduleRow>> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {SCHEDULE_COLUMNS} FROM oncall_schedules WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("lecture d'un planning d'astreinte")?;
    row.as_ref().map(schedule_from_row).transpose()
}

/// Crée (`id` à `None`) ou remplace un planning ; rend son identifiant.
pub async fn save_schedule(
    pool: &SqlitePool,
    id: Option<i64>,
    name: &str,
    description: &str,
    definition: &Definition,
) -> Result<i64> {
    let raw = serde_json::to_string(definition)?;
    match id {
        None => {
            let result = sqlx::query(
                "INSERT INTO oncall_schedules (name, description, definition) VALUES (?, ?, ?)",
            )
            .bind(name)
            .bind(description)
            .bind(&raw)
            .execute(pool)
            .await
            .context("création d'un planning d'astreinte")?;
            Ok(result.last_insert_rowid())
        }
        Some(id) => {
            sqlx::query(
                "UPDATE oncall_schedules SET name = ?, description = ?, definition = ?,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?",
            )
            .bind(name)
            .bind(description)
            .bind(&raw)
            .bind(id)
            .execute(pool)
            .await
            .context("modification d'un planning d'astreinte")?;
            Ok(id)
        }
    }
}

pub async fn delete_schedule(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM oncall_schedules WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("suppression d'un planning d'astreinte")?;
    Ok(result.rows_affected() > 0)
}

// --------------------------------------------------------------------------
// Remplacements
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct OverrideRow {
    pub id: i64,
    pub schedule_id: i64,
    pub user_id: i64,
    pub replaces: Option<i64>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub note: String,
    pub created_by: String,
    pub created_at: String,
}

impl OverrideRow {
    pub fn pure(&self) -> Override {
        Override {
            id: self.id,
            user_id: self.user_id,
            replaces: self.replaces,
            starts_at: self.starts_at,
            ends_at: self.ends_at,
        }
    }
}

fn override_from_row(row: &SqliteRow) -> Result<OverrideRow> {
    let starts: String = row.try_get("starts_at")?;
    let ends: String = row.try_get("ends_at")?;
    Ok(OverrideRow {
        id: row.try_get("id")?,
        schedule_id: row.try_get("schedule_id")?,
        user_id: row.try_get("user_id")?,
        replaces: row.try_get("replaces_user_id")?,
        starts_at: from_sql(&starts).unwrap_or_default(),
        ends_at: from_sql(&ends).unwrap_or_default(),
        note: row.try_get("note")?,
        created_by: row.try_get("created_by")?,
        created_at: row.try_get("created_at")?,
    })
}

/// Remplacements d'un planning (ou de tous), qui finissent après `since`.
pub async fn list_overrides(
    pool: &SqlitePool,
    schedule_id: Option<i64>,
    since: Option<DateTime<Utc>>,
) -> Result<Vec<OverrideRow>> {
    let since = since.map(to_sql);
    let rows = sqlx::query(
        "SELECT id, schedule_id, user_id, replaces_user_id, starts_at, ends_at, note, created_by,
                created_at
         FROM oncall_overrides
         WHERE (? IS NULL OR schedule_id = ?) AND (? IS NULL OR ends_at > ?)
         ORDER BY starts_at, id
         LIMIT 2000",
    )
    .bind(schedule_id)
    .bind(schedule_id)
    .bind(&since)
    .bind(&since)
    .fetch_all(pool)
    .await
    .context("lecture des remplacements d'astreinte")?;
    rows.iter().map(override_from_row).collect()
}

pub struct NewOverride<'a> {
    pub schedule_id: i64,
    pub user_id: i64,
    pub replaces: Option<i64>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub note: &'a str,
    pub created_by: &'a str,
}

pub async fn insert_override(pool: &SqlitePool, new: &NewOverride<'_>) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO oncall_overrides
             (schedule_id, user_id, replaces_user_id, starts_at, ends_at, note, created_by)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new.schedule_id)
    .bind(new.user_id)
    .bind(new.replaces)
    .bind(to_sql(new.starts_at))
    .bind(to_sql(new.ends_at))
    .bind(new.note)
    .bind(new.created_by)
    .execute(pool)
    .await
    .context("création d'un remplacement d'astreinte")?;
    Ok(result.last_insert_rowid())
}

pub async fn delete_override(pool: &SqlitePool, schedule_id: i64, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM oncall_overrides WHERE id = ? AND schedule_id = ?")
        .bind(id)
        .bind(schedule_id)
        .execute(pool)
        .await
        .context("suppression d'un remplacement d'astreinte")?;
    Ok(result.rows_affected() > 0)
}

// --------------------------------------------------------------------------
// Relèves annoncées
// --------------------------------------------------------------------------

/// `None` : jamais vu ; `Some(None)` : personne d'astreinte à la dernière lecture.
pub async fn shift_state(pool: &SqlitePool, schedule_id: i64) -> Result<Option<Option<i64>>> {
    let row = sqlx::query("SELECT user_id FROM oncall_shift_state WHERE schedule_id = ?")
        .bind(schedule_id)
        .fetch_optional(pool)
        .await
        .context("lecture de la dernière relève")?;
    row.map(|row| row.try_get::<Option<i64>, _>("user_id").map_err(Into::into)).transpose()
}

pub async fn set_shift_state(
    pool: &SqlitePool,
    schedule_id: i64,
    user_id: Option<i64>,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO oncall_shift_state (schedule_id, user_id, since) VALUES (?, ?, ?)
         ON CONFLICT(schedule_id) DO UPDATE SET user_id = excluded.user_id, since = excluded.since",
    )
    .bind(schedule_id)
    .bind(user_id)
    .bind(to_sql(now))
    .execute(pool)
    .await
    .context("enregistrement de la relève")?;
    Ok(())
}

// --------------------------------------------------------------------------
// Politiques d'escalade
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PolicyRow {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub definition: PolicyDef,
    pub created_at: String,
    pub updated_at: String,
}

fn policy_from_row(row: &SqliteRow) -> Result<PolicyRow> {
    let raw: String = row.try_get("definition")?;
    Ok(PolicyRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        definition: serde_json::from_str(&raw).unwrap_or_default(),
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn list_policies(pool: &SqlitePool) -> Result<Vec<PolicyRow>> {
    let rows = sqlx::query(
        "SELECT id, name, description, definition, created_at, updated_at
         FROM oncall_policies ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await
    .context("lecture des politiques d'escalade")?;
    rows.iter().map(policy_from_row).collect()
}

pub async fn get_policy(pool: &SqlitePool, id: i64) -> Result<Option<PolicyRow>> {
    let row = sqlx::query(
        "SELECT id, name, description, definition, created_at, updated_at
         FROM oncall_policies WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("lecture d'une politique d'escalade")?;
    row.as_ref().map(policy_from_row).transpose()
}

pub async fn save_policy(
    pool: &SqlitePool,
    id: Option<i64>,
    name: &str,
    description: &str,
    definition: &PolicyDef,
) -> Result<i64> {
    let raw = serde_json::to_string(definition)?;
    match id {
        None => {
            let result = sqlx::query(
                "INSERT INTO oncall_policies (name, description, definition) VALUES (?, ?, ?)",
            )
            .bind(name)
            .bind(description)
            .bind(&raw)
            .execute(pool)
            .await
            .context("création d'une politique d'escalade")?;
            Ok(result.last_insert_rowid())
        }
        Some(id) => {
            sqlx::query(
                "UPDATE oncall_policies SET name = ?, description = ?, definition = ?,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?",
            )
            .bind(name)
            .bind(description)
            .bind(&raw)
            .bind(id)
            .execute(pool)
            .await
            .context("modification d'une politique d'escalade")?;
            Ok(id)
        }
    }
}

pub async fn delete_policy(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM oncall_policies WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("suppression d'une politique d'escalade")?;
    Ok(result.rows_affected() > 0)
}

/// Lit l'identifiant de politique d'un canal d'astreinte : un nombre, ou une
/// chaîne de chiffres (ce qu'un formulaire envoie parfois).
pub fn policy_id_of(settings: &Value) -> Option<i64> {
    match settings.get(crate::oncall::POLICY_SETTING)? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
    .filter(|id| *id > 0)
}

/// Canaux d'astreinte et la politique que chacun déclenche.
pub async fn oncall_channels(pool: &SqlitePool) -> Result<Vec<(i64, String, Option<i64>)>> {
    let rows = sqlx::query("SELECT id, name, settings FROM notification_channels WHERE kind = ?")
        .bind(crate::oncall::CHANNEL_KIND)
        .fetch_all(pool)
        .await
        .context("lecture des canaux d'astreinte")?;
    rows.iter()
        .map(|row| {
            let raw: String = row.try_get("settings")?;
            let settings: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
            Ok((row.try_get("id")?, row.try_get("name")?, policy_id_of(&settings)))
        })
        .collect()
}

// --------------------------------------------------------------------------
// Contacts
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ContactRow {
    pub id: i64,
    pub user_id: i64,
    pub label: String,
    pub kind: String,
    pub via_channel_id: Option<i64>,
    pub address: Option<String>,
    pub settings: Value,
    /// Déchiffrés ; `Null` sans secret ou si le secret d'instance a changé.
    pub secrets: Value,
    pub has_secret: bool,
    pub enabled: bool,
    pub min_severity: Severity,
    pub position: i64,
    pub last_error: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub async fn list_contacts(
    pool: &SqlitePool,
    cipher: &Cipher,
    user_id: Option<i64>,
) -> Result<Vec<ContactRow>> {
    let rows = sqlx::query(
        "SELECT id, user_id, label, kind, via_channel_id, address, settings, secret_enc, enabled,
                min_severity, position, last_error, last_used_at
         FROM oncall_contacts
         WHERE (? IS NULL OR user_id = ?)
         ORDER BY user_id, position, id",
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("lecture des contacts d'astreinte")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let settings: String = row.try_get("settings")?;
        let encrypted: Option<Vec<u8>> = row.try_get("secret_enc")?;
        let secrets = match &encrypted {
            Some(bytes) => match cipher.decrypt(bytes) {
                Ok(plain) => serde_json::from_slice(&plain).unwrap_or(Value::Null),
                Err(error) => {
                    tracing::warn!(%error, "secrets d'un contact d'astreinte illisibles");
                    Value::Null
                }
            },
            None => Value::Null,
        };
        let severity: String = row.try_get("min_severity")?;
        out.push(ContactRow {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            label: row.try_get("label")?,
            kind: row.try_get("kind")?,
            via_channel_id: row.try_get("via_channel_id")?,
            address: row.try_get("address")?,
            settings: serde_json::from_str(&settings).unwrap_or(json!({})),
            has_secret: secrets.as_object().is_some_and(|map| map.values().any(|v| !v.is_null())),
            secrets,
            enabled: row.try_get::<i64, _>("enabled")? != 0,
            min_severity: Severity::parse(&severity),
            position: row.try_get("position")?,
            last_error: row.try_get("last_error")?,
            last_used_at: opt_time(row, "last_used_at")?,
        });
    }
    Ok(out)
}

pub struct ContactDraft {
    pub id: Option<i64>,
    pub user_id: i64,
    pub label: String,
    pub kind: String,
    pub via_channel_id: Option<i64>,
    pub address: Option<String>,
    pub settings: Value,
    /// `None` : conserver les secrets en place (modification) ou aucun (création).
    pub secrets: Option<Value>,
    pub enabled: bool,
    pub min_severity: Severity,
}

pub async fn save_contact(pool: &SqlitePool, cipher: &Cipher, draft: &ContactDraft) -> Result<i64> {
    let settings = serde_json::to_string(&draft.settings)?;
    let encrypted = match &draft.secrets {
        Some(value) if value.as_object().is_some_and(|map| !map.is_empty()) => {
            Some(cipher.encrypt(&serde_json::to_vec(value)?)?)
        }
        _ => None,
    };
    match draft.id {
        None => {
            let result = sqlx::query(
                "INSERT INTO oncall_contacts
                     (user_id, label, kind, via_channel_id, address, settings, secret_enc, enabled,
                      min_severity, position)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?,
                         (SELECT COALESCE(MAX(position), 0) + 1 FROM oncall_contacts WHERE user_id = ?))",
            )
            .bind(draft.user_id)
            .bind(&draft.label)
            .bind(&draft.kind)
            .bind(draft.via_channel_id)
            .bind(&draft.address)
            .bind(&settings)
            .bind(&encrypted)
            .bind(i64::from(draft.enabled))
            .bind(draft.min_severity.as_str())
            .bind(draft.user_id)
            .execute(pool)
            .await
            .context("création d'un contact d'astreinte")?;
            Ok(result.last_insert_rowid())
        }
        Some(id) => {
            let keep = draft.secrets.is_none();
            sqlx::query(
                "UPDATE oncall_contacts SET label = ?, kind = ?, via_channel_id = ?, address = ?,
                     settings = ?, secret_enc = CASE WHEN ? THEN secret_enc ELSE ? END,
                     enabled = ?, min_severity = ?, last_error = NULL
                 WHERE id = ? AND user_id = ?",
            )
            .bind(&draft.label)
            .bind(&draft.kind)
            .bind(draft.via_channel_id)
            .bind(&draft.address)
            .bind(&settings)
            .bind(keep)
            .bind(&encrypted)
            .bind(i64::from(draft.enabled))
            .bind(draft.min_severity.as_str())
            .bind(id)
            .bind(draft.user_id)
            .execute(pool)
            .await
            .context("modification d'un contact d'astreinte")?;
            Ok(id)
        }
    }
}

pub async fn delete_contact(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM oncall_contacts WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("suppression d'un contact d'astreinte")?;
    Ok(result.rows_affected() > 0)
}

pub async fn note_contact_use(
    pool: &SqlitePool,
    id: i64,
    error: Option<&str>,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query("UPDATE oncall_contacts SET last_error = ?, last_used_at = ? WHERE id = ?")
        .bind(error)
        .bind(to_sql(now))
        .bind(id)
        .execute(pool)
        .await
        .context("compte rendu d'un contact d'astreinte")?;
    Ok(())
}

// --------------------------------------------------------------------------
// Préférences
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Prefs {
    pub timezone: String,
    pub quiet_hours: Option<Schedule>,
    pub calendar: bool,
}

pub async fn get_prefs(pool: &SqlitePool, user_id: i64) -> Result<Prefs> {
    let row = sqlx::query(
        "SELECT timezone, quiet_hours, ical_token_hash FROM oncall_user_prefs WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("lecture des préférences d'astreinte")?;
    let Some(row) = row else { return Ok(Prefs::default()) };
    let quiet: Option<String> = row.try_get("quiet_hours")?;
    let hash: Option<String> = row.try_get("ical_token_hash")?;
    Ok(Prefs {
        timezone: row.try_get("timezone")?,
        quiet_hours: quiet.as_deref().and_then(|raw| serde_json::from_str(raw).ok()),
        calendar: hash.is_some(),
    })
}

/// Préférences de tous les comptes, pour un passage du moteur.
pub async fn all_prefs(pool: &SqlitePool) -> Result<std::collections::HashMap<i64, Prefs>> {
    let rows = sqlx::query(
        "SELECT user_id, timezone, quiet_hours, ical_token_hash FROM oncall_user_prefs",
    )
    .fetch_all(pool)
    .await
    .context("lecture des préférences d'astreinte")?;
    rows.iter()
        .map(|row| {
            let quiet: Option<String> = row.try_get("quiet_hours")?;
            let hash: Option<String> = row.try_get("ical_token_hash")?;
            Ok((
                row.try_get("user_id")?,
                Prefs {
                    timezone: row.try_get("timezone")?,
                    quiet_hours: quiet.as_deref().and_then(|raw| serde_json::from_str(raw).ok()),
                    calendar: hash.is_some(),
                },
            ))
        })
        .collect()
}

pub async fn save_prefs(
    pool: &SqlitePool,
    user_id: i64,
    timezone: &str,
    quiet_hours: Option<&Schedule>,
) -> Result<()> {
    let quiet = quiet_hours.map(serde_json::to_string).transpose()?;
    sqlx::query(
        "INSERT INTO oncall_user_prefs (user_id, timezone, quiet_hours) VALUES (?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET timezone = excluded.timezone,
             quiet_hours = excluded.quiet_hours,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(user_id)
    .bind(timezone)
    .bind(quiet)
    .execute(pool)
    .await
    .context("enregistrement des préférences d'astreinte")?;
    Ok(())
}

pub async fn set_calendar_hash(pool: &SqlitePool, user_id: i64, hash: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO oncall_user_prefs (user_id, ical_token_hash) VALUES (?, ?)
         ON CONFLICT(user_id) DO UPDATE SET ical_token_hash = excluded.ical_token_hash,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(user_id)
    .bind(hash)
    .execute(pool)
    .await
    .context("enregistrement du lien de calendrier")?;
    Ok(())
}

pub async fn user_by_calendar_hash(pool: &SqlitePool, hash: &str) -> Result<Option<i64>> {
    let row = sqlx::query("SELECT user_id FROM oncall_user_prefs WHERE ical_token_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await
        .context("lecture du lien de calendrier")?;
    row.map(|row| row.try_get("user_id").map_err(Into::into)).transpose()
}

// --------------------------------------------------------------------------
// Escalades et journal
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EscalationRow {
    pub id: i64,
    pub policy_id: i64,
    pub channel_id: i64,
    pub fingerprint: String,
    pub episode: String,
    pub target_id: Option<i64>,
    pub target_name: String,
    pub rule_name: String,
    pub severity: Severity,
    pub item: String,
    pub status: String,
    pub next_step: i64,
    pub round: i64,
    pub next_at: Option<DateTime<Utc>>,
    pub acked_by: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

const ESCALATION_COLUMNS: &str = "id, policy_id, channel_id, fingerprint, episode, target_id, \
     target_name, rule_name, severity, item, status, next_step, round, next_at, acked_by, \
     started_at, ended_at";

fn escalation_from_row(row: &SqliteRow) -> Result<EscalationRow> {
    let severity: String = row.try_get("severity")?;
    Ok(EscalationRow {
        id: row.try_get("id")?,
        policy_id: row.try_get("policy_id")?,
        channel_id: row.try_get("channel_id")?,
        fingerprint: row.try_get("fingerprint")?,
        episode: row.try_get("episode")?,
        target_id: row.try_get("target_id")?,
        target_name: row.try_get("target_name")?,
        rule_name: row.try_get("rule_name")?,
        severity: Severity::parse(&severity),
        item: row.try_get("item")?,
        status: row.try_get("status")?,
        next_step: row.try_get("next_step")?,
        round: row.try_get("round")?,
        next_at: opt_time(row, "next_at")?,
        acked_by: row.try_get("acked_by")?,
        started_at: opt_time(row, "started_at")?,
        ended_at: opt_time(row, "ended_at")?,
    })
}

/// Vrai si cette alerte (empreinte + épisode) a déjà une escalade sur ce canal.
pub async fn escalation_exists(
    pool: &SqlitePool,
    channel_id: i64,
    fingerprint: &str,
    episode: &str,
) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM oncall_escalations WHERE channel_id = ? AND fingerprint = ? AND episode = ?
         LIMIT 1",
    )
    .bind(channel_id)
    .bind(fingerprint)
    .bind(episode)
    .fetch_optional(pool)
    .await
    .context("recherche d'une escalade")?;
    Ok(row.is_some())
}

pub struct NewEscalation<'a> {
    pub policy_id: i64,
    pub channel_id: i64,
    pub fingerprint: &'a str,
    pub episode: &'a str,
    pub target_id: Option<i64>,
    pub target_name: &'a str,
    pub rule_name: &'a str,
    pub severity: Severity,
    pub item: &'a str,
    pub now: DateTime<Utc>,
}

pub async fn insert_escalation(pool: &SqlitePool, new: &NewEscalation<'_>) -> Result<i64> {
    let at = to_sql(new.now);
    let result = sqlx::query(
        "INSERT INTO oncall_escalations
             (policy_id, channel_id, fingerprint, episode, target_id, target_name, rule_name,
              severity, item, status, next_step, round, next_at, started_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', 0, 0, ?, ?)",
    )
    .bind(new.policy_id)
    .bind(new.channel_id)
    .bind(new.fingerprint)
    .bind(new.episode)
    .bind(new.target_id)
    .bind(new.target_name)
    .bind(new.rule_name)
    .bind(new.severity.as_str())
    .bind(new.item)
    .bind(&at)
    .bind(&at)
    .execute(pool)
    .await
    .context("création d'une escalade")?;
    Ok(result.last_insert_rowid())
}

pub async fn active_escalations(pool: &SqlitePool) -> Result<Vec<EscalationRow>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ESCALATION_COLUMNS} FROM oncall_escalations WHERE status = 'active' ORDER BY id"
    )))
    .fetch_all(pool)
    .await
    .context("lecture des escalades en cours")?;
    rows.iter().map(escalation_from_row).collect()
}

/// Escalades en cours d'une alerte sur un canal.
pub async fn active_for(
    pool: &SqlitePool,
    channel_id: i64,
    fingerprint: &str,
) -> Result<Vec<EscalationRow>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ESCALATION_COLUMNS} FROM oncall_escalations
         WHERE status = 'active' AND channel_id = ? AND fingerprint = ?"
    )))
    .bind(channel_id)
    .bind(fingerprint)
    .fetch_all(pool)
    .await
    .context("lecture des escalades d'une alerte")?;
    rows.iter().map(escalation_from_row).collect()
}

pub async fn set_progress(
    pool: &SqlitePool,
    id: i64,
    next_step: usize,
    round: u32,
    next_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query("UPDATE oncall_escalations SET next_step = ?, round = ?, next_at = ? WHERE id = ?")
        .bind(next_step as i64)
        .bind(i64::from(round))
        .bind(to_sql(next_at))
        .bind(id)
        .execute(pool)
        .await
        .context("avancement d'une escalade")?;
    Ok(())
}

pub async fn finish_escalation(
    pool: &SqlitePool,
    id: i64,
    status: &str,
    acked_by: Option<&str>,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "UPDATE oncall_escalations SET status = ?, acked_by = COALESCE(?, acked_by), ended_at = ?,
             next_at = NULL
         WHERE id = ? AND status = 'active'",
    )
    .bind(status)
    .bind(acked_by)
    .bind(to_sql(now))
    .bind(id)
    .execute(pool)
    .await
    .context("fin d'une escalade")?;
    Ok(())
}

/// Ligne du journal d'une escalade.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Event {
    pub at: String,
    pub step: Option<i64>,
    pub round: Option<i64>,
    pub event: String,
    pub user_id: Option<i64>,
    pub user_name: Option<String>,
    pub via: String,
    pub kind: String,
    pub detail: String,
    #[serde(skip)]
    pub contact_id: Option<i64>,
    #[serde(skip)]
    pub channel_id: Option<i64>,
}

pub async fn add_event(pool: &SqlitePool, escalation_id: i64, event: &Event) -> Result<()> {
    sqlx::query(
        "INSERT INTO oncall_escalation_events
             (escalation_id, at, step, round, event, user_id, user_name, via, kind, detail,
              contact_id, channel_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(escalation_id)
    .bind(&event.at)
    .bind(event.step)
    .bind(event.round)
    .bind(&event.event)
    .bind(event.user_id)
    .bind(&event.user_name)
    .bind(&event.via)
    .bind(&event.kind)
    .bind(&event.detail)
    .bind(event.contact_id)
    .bind(event.channel_id)
    .execute(pool)
    .await
    .context("journal d'une escalade")?;
    Ok(())
}

pub async fn events_of(pool: &SqlitePool, escalation_id: i64) -> Result<Vec<Event>> {
    let rows = sqlx::query(
        "SELECT at, step, round, event, user_id, user_name, via, kind, detail, contact_id,
                channel_id
         FROM oncall_escalation_events WHERE escalation_id = ? ORDER BY at, id",
    )
    .bind(escalation_id)
    .fetch_all(pool)
    .await
    .context("lecture du journal d'une escalade")?;
    rows.iter()
        .map(|row| {
            Ok(Event {
                at: row.try_get("at")?,
                step: row.try_get("step")?,
                round: row.try_get("round")?,
                event: row.try_get("event")?,
                user_id: row.try_get("user_id")?,
                user_name: row.try_get("user_name")?,
                via: row.try_get("via")?,
                kind: row.try_get("kind")?,
                detail: row.try_get("detail")?,
                contact_id: row.try_get("contact_id")?,
                channel_id: row.try_get("channel_id")?,
            })
        })
        .collect()
}

/// Escalades récentes, les plus récentes d'abord.
pub async fn list_escalations(
    pool: &SqlitePool,
    active: Option<bool>,
    fingerprint: Option<&str>,
    limit: i64,
) -> Result<Vec<EscalationRow>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ESCALATION_COLUMNS} FROM oncall_escalations
         WHERE (? IS NULL OR (status = 'active') = ?) AND (? IS NULL OR fingerprint = ?)
         ORDER BY id DESC LIMIT ?"
    )))
    .bind(active)
    .bind(active)
    .bind(fingerprint)
    .bind(fingerprint)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("lecture des escalades")?;
    rows.iter().map(escalation_from_row).collect()
}

/// Purge des escalades terminées depuis plus de `days` jours.
pub async fn purge_escalations(pool: &SqlitePool, before: DateTime<Utc>) -> Result<u64> {
    let result =
        sqlx::query("DELETE FROM oncall_escalations WHERE status <> 'active' AND ended_at < ?")
            .bind(to_sql(before))
            .execute(pool)
            .await
            .context("purge des escalades")?;
    Ok(result.rows_affected())
}
