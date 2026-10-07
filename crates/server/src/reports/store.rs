//! Persistance des calendriers de rapports (`report_schedules`).

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};

fn to_sql(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn from_sql(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw).map(|at| at.with_timezone(&Utc)).unwrap_or_default()
}

/// Un calendrier tel que stocké.
#[derive(Debug, Clone)]
pub struct StoredSchedule {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub frequency: String,
    pub weekday: u32,
    pub day_of_month: u32,
    pub hour: u32,
    pub timezone: String,
    pub recipients: Vec<String>,
    pub channel_id: Option<i64>,
    pub armed_at: DateTime<Utc>,
    pub last_sent_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

/// Calendrier validé, prêt à être écrit.
#[derive(Debug, Clone)]
pub struct ScheduleInput {
    pub name: String,
    pub enabled: bool,
    pub frequency: String,
    pub weekday: u32,
    pub day_of_month: u32,
    pub hour: u32,
    pub timezone: String,
    pub recipients: Vec<String>,
    pub channel_id: Option<i64>,
}

const COLUMNS: &str = "id, name, enabled, frequency, weekday, day_of_month, hour, timezone, \
                       recipients, channel_id, armed_at, last_sent_at, last_error";

fn from_row(row: &SqliteRow) -> Result<StoredSchedule> {
    let recipients: String = row.try_get("recipients")?;
    let last_sent: Option<String> = row.try_get("last_sent_at")?;
    let armed: String = row.try_get("armed_at")?;
    Ok(StoredSchedule {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        frequency: row.try_get("frequency")?,
        weekday: row.try_get::<i64, _>("weekday")? as u32,
        day_of_month: row.try_get::<i64, _>("day_of_month")? as u32,
        hour: row.try_get::<i64, _>("hour")? as u32,
        timezone: row.try_get("timezone")?,
        recipients: serde_json::from_str(&recipients).unwrap_or_default(),
        channel_id: row.try_get("channel_id")?,
        armed_at: from_sql(&armed),
        last_sent_at: last_sent.as_deref().map(from_sql),
        last_error: row.try_get("last_error")?,
    })
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<StoredSchedule>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM report_schedules ORDER BY id"
    )))
    .fetch_all(pool)
    .await
    .context("lecture des rapports planifiés")?;
    rows.iter().map(from_row).collect()
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<StoredSchedule>> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM report_schedules WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("lecture du rapport planifié")?;
    row.as_ref().map(from_row).transpose()
}

pub async fn count(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM report_schedules")
        .fetch_one(pool)
        .await
        .context("comptage des rapports planifiés")?;
    Ok(row.try_get("n")?)
}

pub async fn insert(pool: &SqlitePool, input: &ScheduleInput, now: DateTime<Utc>) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO report_schedules
             (name, enabled, frequency, weekday, day_of_month, hour, timezone, recipients,
              channel_id, armed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.name)
    .bind(i64::from(input.enabled))
    .bind(&input.frequency)
    .bind(i64::from(input.weekday))
    .bind(i64::from(input.day_of_month))
    .bind(i64::from(input.hour))
    .bind(&input.timezone)
    .bind(serde_json::to_string(&input.recipients)?)
    .bind(input.channel_id)
    .bind(to_sql(now))
    .execute(pool)
    .await
    .context("création du rapport planifié")?;
    Ok(result.last_insert_rowid())
}

/// Met à jour un calendrier. Un changement d'horaire ou une réactivation
/// réarme : le créneau qui précède la modification ne partira pas après coup.
pub async fn update(
    pool: &SqlitePool,
    current: &StoredSchedule,
    input: &ScheduleInput,
    now: DateTime<Utc>,
) -> Result<()> {
    let retimed = current.frequency != input.frequency
        || current.weekday != input.weekday
        || current.day_of_month != input.day_of_month
        || current.hour != input.hour
        || current.timezone != input.timezone
        || (!current.enabled && input.enabled);
    let armed_at = if retimed { now } else { current.armed_at };
    sqlx::query(
        "UPDATE report_schedules
         SET name = ?, enabled = ?, frequency = ?, weekday = ?, day_of_month = ?, hour = ?,
             timezone = ?, recipients = ?, channel_id = ?, armed_at = ?
         WHERE id = ?",
    )
    .bind(&input.name)
    .bind(i64::from(input.enabled))
    .bind(&input.frequency)
    .bind(i64::from(input.weekday))
    .bind(i64::from(input.day_of_month))
    .bind(i64::from(input.hour))
    .bind(&input.timezone)
    .bind(serde_json::to_string(&input.recipients)?)
    .bind(input.channel_id)
    .bind(to_sql(armed_at))
    .bind(current.id)
    .execute(pool)
    .await
    .context("mise à jour du rapport planifié")?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM report_schedules WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("suppression du rapport planifié")?;
    Ok(result.rows_affected() > 0)
}

/// Réclame un créneau avant de l'envoyer. Vrai pour un seul appelant : le
/// `UPDATE` conditionnel est ce qui rend l'envoi unique, même si deux passes de
/// la boucle (ou deux instances sur la même base) se croisent.
pub async fn claim(
    pool: &SqlitePool,
    id: i64,
    slot: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE report_schedules SET last_sent_at = ?, last_error = NULL
         WHERE id = ? AND enabled = 1 AND (last_sent_at IS NULL OR last_sent_at < ?)",
    )
    .bind(to_sql(now))
    .bind(id)
    .bind(to_sql(slot))
    .execute(pool)
    .await
    .context("réservation du créneau de rapport")?;
    Ok(result.rows_affected() == 1)
}

pub async fn set_error(pool: &SqlitePool, id: i64, error: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE report_schedules SET last_error = ? WHERE id = ?")
        .bind(error)
        .bind(id)
        .execute(pool)
        .await
        .context("enregistrement de l'erreur d'envoi du rapport")?;
    Ok(())
}
