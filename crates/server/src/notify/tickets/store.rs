//! Liens entre alertes et tickets externes (table `alert_tickets`).

use std::collections::HashMap;

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use super::TicketRef;

fn to_sql(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn from_sql(raw: Option<String>) -> Option<DateTime<Utc>> {
    raw.and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc))
}

/// Un ticket suivi, tel que stocké.
#[derive(Debug, Clone)]
pub struct Linked {
    pub ticket: TicketRef,
    pub open: bool,
    pub closed_at: Option<DateTime<Utc>>,
}

pub async fn get(
    pool: &SqlitePool,
    channel_id: i64,
    fingerprint: &str,
) -> Result<Option<Linked>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT external_id, label, url, state, closed_at FROM alert_tickets
         WHERE channel_id = ? AND fingerprint = ?",
    )
    .bind(channel_id)
    .bind(fingerprint)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    Ok(Some(Linked {
        ticket: TicketRef {
            id: row.try_get("external_id")?,
            label: row.try_get("label")?,
            url: row.try_get("url")?,
        },
        open: row.try_get::<String, _>("state")? == "open",
        closed_at: from_sql(row.try_get("closed_at")?),
    }))
}

/// Enregistre un ticket ouvert pour l'alerte, en remplaçant un ancien lien clos.
pub async fn set_open(
    pool: &SqlitePool,
    channel_id: i64,
    fingerprint: &str,
    ticket: &TicketRef,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO alert_tickets
             (channel_id, fingerprint, external_id, label, url, state, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'open', ?, ?)
         ON CONFLICT (channel_id, fingerprint) DO UPDATE SET
             external_id = excluded.external_id, label = excluded.label, url = excluded.url,
             state = 'open', closed_at = NULL, updated_at = excluded.updated_at",
    )
    .bind(channel_id)
    .bind(fingerprint)
    .bind(&ticket.id)
    .bind(&ticket.label)
    .bind(&ticket.url)
    .bind(to_sql(now))
    .bind(to_sql(now))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_closed(
    pool: &SqlitePool,
    channel_id: i64,
    fingerprint: &str,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE alert_tickets SET state = 'closed', closed_at = ?, updated_at = ?
         WHERE channel_id = ? AND fingerprint = ?",
    )
    .bind(to_sql(now))
    .bind(to_sql(now))
    .bind(channel_id)
    .bind(fingerprint)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_reopened(
    pool: &SqlitePool,
    channel_id: i64,
    fingerprint: &str,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE alert_tickets SET state = 'open', closed_at = NULL, updated_at = ?
         WHERE channel_id = ? AND fingerprint = ?",
    )
    .bind(to_sql(now))
    .bind(channel_id)
    .bind(fingerprint)
    .execute(pool)
    .await?;
    Ok(())
}

/// Oublie les liens clos depuis longtemps : plus aucun délai de réouverture
/// raisonnable ne les atteint.
pub async fn purge_closed(pool: &SqlitePool, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM alert_tickets WHERE state = 'closed' AND closed_at < ?")
        .bind(to_sql(now - TimeDelta::days(30)))
        .execute(pool)
        .await?;
    Ok(())
}

/// Tickets encore ouverts d'une alerte, tous canaux confondus.
pub async fn open_for(
    pool: &SqlitePool,
    fingerprint: &str,
) -> Result<Vec<(i64, TicketRef)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT channel_id, external_id, label, url FROM alert_tickets
         WHERE fingerprint = ? AND state = 'open'",
    )
    .bind(fingerprint)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok::<_, sqlx::Error>((
                row.try_get("channel_id")?,
                TicketRef {
                    id: row.try_get("external_id")?,
                    label: row.try_get("label")?,
                    url: row.try_get("url")?,
                },
            ))
        })
        .collect()
}

/// Lien d'un ticket, tel que l'API le montre sur l'alerte.
#[derive(Debug, Clone, Serialize)]
pub struct TicketLink {
    pub channel_id: i64,
    pub channel_name: String,
    /// Type du canal (`jira`, `glpi`…).
    pub kind: String,
    /// Libellé du ticket (« PROJ-12 », « #34 »).
    pub label: String,
    pub url: String,
    /// `open` ou `closed`.
    pub state: String,
}

/// Les liens de toutes les alertes, rangés par empreinte.
pub async fn links_by_fingerprint(
    pool: &SqlitePool,
) -> Result<HashMap<String, Vec<TicketLink>>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT t.fingerprint, t.channel_id, c.name, c.kind, t.label, t.url, t.state
         FROM alert_tickets t JOIN notification_channels c ON c.id = t.channel_id
         ORDER BY t.channel_id",
    )
    .fetch_all(pool)
    .await?;
    let mut by_alert: HashMap<String, Vec<TicketLink>> = HashMap::new();
    for row in &rows {
        by_alert.entry(row.try_get("fingerprint")?).or_default().push(TicketLink {
            channel_id: row.try_get("channel_id")?,
            channel_name: row.try_get("name")?,
            kind: row.try_get("kind")?,
            label: row.try_get("label")?,
            url: row.try_get("url")?,
            state: row.try_get("state")?,
        });
    }
    Ok(by_alert)
}
