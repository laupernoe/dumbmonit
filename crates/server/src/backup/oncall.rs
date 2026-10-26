//! Section `oncall` du lot : plannings, remplacements, politiques d'escalade,
//! contacts et préférences de chaque compte.
//!
//! Les identifiants numériques ne voyagent pas : un participant est désigné
//! par son nom de compte, une cible d'étape par le nom du planning, du compte
//! ou du canal. Les secrets des contacts partent en clair dans la charge utile,
//! que la phrase de passe chiffre tout entière. Les liens de calendrier, eux,
//! ne partent pas : ce sont des jetons, à régénérer sur l'instance restaurée.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use crate::backup::import::{RestoreOutcome, SectionReport};
use crate::crypto::Cipher;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundleOncall {
    #[serde(default)]
    pub schedules: Vec<BundleSchedule>,
    #[serde(default)]
    pub overrides: Vec<BundleCover>,
    #[serde(default)]
    pub policies: Vec<BundlePolicy>,
    #[serde(default)]
    pub contacts: Vec<BundleContact>,
    #[serde(default)]
    pub prefs: Vec<BundlePrefs>,
}

impl BundleOncall {
    pub fn count(&self) -> usize {
        self.schedules.len() + self.policies.len() + self.contacts.len()
    }
}

/// Un planning ; dans `definition`, les participants sont des noms de compte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSchedule {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub definition: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleCover {
    pub schedule: String,
    pub user: String,
    #[serde(default)]
    pub replaces: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub created_by: String,
}

/// Une politique ; dans `definition`, chaque cible porte `name` au lieu d'`id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePolicy {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub definition: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleContact {
    pub user: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub via_channel: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub settings: Value,
    #[serde(default)]
    pub secrets: Value,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub min_severity: String,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePrefs {
    pub user: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub quiet_hours: Option<Value>,
}

// --------------------------------------------------------------------------
// Export
// --------------------------------------------------------------------------

async fn names(pool: &SqlitePool, sql: &'static str) -> Result<HashMap<i64, String>> {
    let rows = sqlx::query(sql).fetch_all(pool).await.context("lecture des noms")?;
    rows.iter().map(|row| Ok((row.try_get("id")?, row.try_get("name")?))).collect()
}

/// Remplace, dans un document JSON, les identifiants par des noms.
fn participants_to_names(definition: &mut Value, users: &HashMap<i64, String>) {
    for layer in definition.get_mut("layers").and_then(Value::as_array_mut).into_iter().flatten() {
        if let Some(list) = layer.get_mut("participants").and_then(Value::as_array_mut) {
            *list = list
                .iter()
                .filter_map(|id| id.as_i64().and_then(|id| users.get(&id)).map(|n| json!(n)))
                .collect();
        }
    }
}

fn targets_to_names(
    definition: &mut Value,
    users: &HashMap<i64, String>,
    schedules: &HashMap<i64, String>,
    channels: &HashMap<i64, String>,
) {
    for step in definition.get_mut("steps").and_then(Value::as_array_mut).into_iter().flatten() {
        if let Some(targets) = step.get_mut("targets").and_then(Value::as_array_mut) {
            *targets = targets
                .iter()
                .filter_map(|target| {
                    let kind = target.get("type")?.as_str()?;
                    let id = target.get("id")?.as_i64()?;
                    let name = match kind {
                        "schedule" => schedules.get(&id),
                        "user" => users.get(&id),
                        "channel" => channels.get(&id),
                        _ => None,
                    }?;
                    Some(json!({"type": kind, "name": name}))
                })
                .collect();
        }
    }
}

pub async fn collect(pool: &SqlitePool, cipher: &Cipher) -> Result<BundleOncall> {
    let users = names(pool, "SELECT id, username AS name FROM users").await?;
    let schedule_names = names(pool, "SELECT id, name FROM oncall_schedules").await?;
    let channel_names = names(pool, "SELECT id, name FROM notification_channels").await?;

    let mut out = BundleOncall::default();
    let rows =
        sqlx::query("SELECT name, description, definition FROM oncall_schedules ORDER BY id")
            .fetch_all(pool)
            .await
            .context("export des plannings")?;
    for row in &rows {
        let raw: String = row.try_get("definition")?;
        let mut definition: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        participants_to_names(&mut definition, &users);
        out.schedules.push(BundleSchedule {
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            definition,
        });
    }

    let rows = sqlx::query(
        "SELECT schedule_id, user_id, replaces_user_id, starts_at, ends_at, note, created_by
         FROM oncall_overrides ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .context("export des remplacements")?;
    for row in &rows {
        let schedule: i64 = row.try_get("schedule_id")?;
        let user: i64 = row.try_get("user_id")?;
        let replaces: Option<i64> = row.try_get("replaces_user_id")?;
        let (Some(schedule), Some(user)) = (schedule_names.get(&schedule), users.get(&user)) else {
            continue;
        };
        out.overrides.push(BundleCover {
            schedule: schedule.clone(),
            user: user.clone(),
            replaces: replaces.and_then(|id| users.get(&id).cloned()),
            starts_at: row.try_get("starts_at")?,
            ends_at: row.try_get("ends_at")?,
            note: row.try_get("note")?,
            created_by: row.try_get("created_by")?,
        });
    }

    let rows = sqlx::query("SELECT name, description, definition FROM oncall_policies ORDER BY id")
        .fetch_all(pool)
        .await
        .context("export des politiques d'escalade")?;
    for row in &rows {
        let raw: String = row.try_get("definition")?;
        let mut definition: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        targets_to_names(&mut definition, &users, &schedule_names, &channel_names);
        out.policies.push(BundlePolicy {
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            definition,
        });
    }

    for contact in crate::oncall::store::list_contacts(pool, cipher, None).await? {
        let Some(user) = users.get(&contact.user_id) else { continue };
        out.contacts.push(BundleContact {
            user: user.clone(),
            label: contact.label,
            kind: contact.kind,
            via_channel: contact.via_channel_id.and_then(|id| channel_names.get(&id).cloned()),
            address: contact.address,
            settings: contact.settings,
            secrets: contact.secrets,
            enabled: contact.enabled,
            min_severity: contact.min_severity.as_str().to_string(),
        });
    }

    let rows = sqlx::query("SELECT user_id, timezone, quiet_hours FROM oncall_user_prefs")
        .fetch_all(pool)
        .await
        .context("export des préférences d'astreinte")?;
    for row in &rows {
        let user: i64 = row.try_get("user_id")?;
        let Some(user) = users.get(&user) else { continue };
        let quiet: Option<String> = row.try_get("quiet_hours")?;
        out.prefs.push(BundlePrefs {
            user: user.clone(),
            timezone: row.try_get("timezone")?,
            quiet_hours: quiet.as_deref().and_then(|raw| serde_json::from_str(raw).ok()),
        });
    }
    Ok(out)
}

// --------------------------------------------------------------------------
// Restauration
// --------------------------------------------------------------------------

async fn ids_by_name(
    tx: &mut Transaction<'_, Sqlite>,
    sql: &'static str,
) -> Result<HashMap<String, i64>> {
    let rows = sqlx::query(sql).fetch_all(&mut **tx).await.context("relecture des noms")?;
    rows.iter()
        .map(|row| {
            let name: String = row.try_get("name")?;
            Ok((name.to_lowercase(), row.try_get("id")?))
        })
        .collect()
}

/// Restaure la section : par nom pour les plannings, les politiques et les
/// contacts (compte + libellé), à l'identique pour les remplacements.
pub async fn restore(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &BundleOncall,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("oncall");
    let users = ids_by_name(tx, "SELECT id, username AS name FROM users").await?;
    let channels = ids_by_name(tx, "SELECT id, name FROM notification_channels").await?;
    let user_id = |name: &str| users.get(&name.to_lowercase()).copied();

    for schedule in &bundle.schedules {
        let mut definition = schedule.definition.clone();
        for layer in
            definition.get_mut("layers").and_then(Value::as_array_mut).into_iter().flatten()
        {
            if let Some(list) = layer.get_mut("participants").and_then(Value::as_array_mut) {
                let before = list.len();
                *list = list
                    .iter()
                    .filter_map(|name| name.as_str().and_then(user_id).map(|id| json!(id)))
                    .collect();
                if list.len() < before {
                    report.notes.push(format!(
                        "schedule \"{}\": {} participant(s) without a matching account were left out",
                        schedule.name,
                        before - list.len()
                    ));
                }
            }
        }
        let raw = serde_json::to_string(&definition)?;
        let existing: Option<(i64, String)> = sqlx::query_as(
            "SELECT id, definition FROM oncall_schedules WHERE name = ? COLLATE NOCASE",
        )
        .bind(&schedule.name)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche d'un planning")?;
        let result =
            match &existing {
                Some((_, current)) if *current == raw => {
                    report.count(RestoreOutcome::Skipped);
                    continue;
                }
                Some((id, _)) => {
                    sqlx::query(
                        "UPDATE oncall_schedules SET description = ?, definition = ? WHERE id = ?",
                    )
                    .bind(&schedule.description)
                    .bind(&raw)
                    .bind(id)
                    .execute(&mut **tx)
                    .await
                }
                None => sqlx::query(
                    "INSERT INTO oncall_schedules (name, description, definition) VALUES (?, ?, ?)",
                )
                .bind(&schedule.name)
                .bind(&schedule.description)
                .bind(&raw)
                .execute(&mut **tx)
                .await,
            };
        match result {
            Ok(_) if existing.is_some() => report.count(RestoreOutcome::Updated),
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused(&format!("schedule \"{}\"", schedule.name), &error.into()),
        }
    }
    let schedules = ids_by_name(tx, "SELECT id, name FROM oncall_schedules").await?;

    for cover in &bundle.overrides {
        let (Some(schedule), Some(user)) =
            (schedules.get(&cover.schedule.to_lowercase()).copied(), user_id(&cover.user))
        else {
            report.skipped += 1;
            report.notes.push(format!(
                "replacement by \"{}\" in \"{}\": unknown account or schedule",
                cover.user, cover.schedule
            ));
            continue;
        };
        let replaces = cover.replaces.as_deref().and_then(user_id);
        let exists: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM oncall_overrides
             WHERE schedule_id = ? AND user_id = ? AND starts_at = ? AND ends_at = ?",
        )
        .bind(schedule)
        .bind(user)
        .bind(&cover.starts_at)
        .bind(&cover.ends_at)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche d'un remplacement")?;
        if exists.is_some() {
            report.count(RestoreOutcome::Skipped);
            continue;
        }
        let result = sqlx::query(
            "INSERT INTO oncall_overrides
                 (schedule_id, user_id, replaces_user_id, starts_at, ends_at, note, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(schedule)
        .bind(user)
        .bind(replaces)
        .bind(&cover.starts_at)
        .bind(&cover.ends_at)
        .bind(&cover.note)
        .bind(&cover.created_by)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => {
                report.refused(&format!("replacement by \"{}\"", cover.user), &error.into())
            }
        }
    }

    for policy in &bundle.policies {
        let mut definition = policy.definition.clone();
        for step in definition.get_mut("steps").and_then(Value::as_array_mut).into_iter().flatten()
        {
            if let Some(targets) = step.get_mut("targets").and_then(Value::as_array_mut) {
                let before = targets.len();
                *targets = targets
                    .iter()
                    .filter_map(|target| {
                        let kind = target.get("type")?.as_str()?;
                        let name = target.get("name")?.as_str()?.to_lowercase();
                        let id = match kind {
                            "schedule" => schedules.get(&name),
                            "user" => users.get(&name),
                            "channel" => channels.get(&name),
                            _ => None,
                        }?;
                        Some(json!({"type": kind, "id": id}))
                    })
                    .collect();
                if targets.len() < before {
                    report.notes.push(format!(
                        "policy \"{}\": {} target(s) not found on this instance were left out",
                        policy.name,
                        before - targets.len()
                    ));
                }
            }
        }
        let raw = serde_json::to_string(&definition)?;
        let existing: Option<(i64, String)> = sqlx::query_as(
            "SELECT id, definition FROM oncall_policies WHERE name = ? COLLATE NOCASE",
        )
        .bind(&policy.name)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche d'une politique")?;
        let result =
            match &existing {
                Some((_, current)) if *current == raw => {
                    report.count(RestoreOutcome::Skipped);
                    continue;
                }
                Some((id, _)) => {
                    sqlx::query(
                        "UPDATE oncall_policies SET description = ?, definition = ? WHERE id = ?",
                    )
                    .bind(&policy.description)
                    .bind(&raw)
                    .bind(id)
                    .execute(&mut **tx)
                    .await
                }
                None => sqlx::query(
                    "INSERT INTO oncall_policies (name, description, definition) VALUES (?, ?, ?)",
                )
                .bind(&policy.name)
                .bind(&policy.description)
                .bind(&raw)
                .execute(&mut **tx)
                .await,
            };
        match result {
            Ok(_) if existing.is_some() => report.count(RestoreOutcome::Updated),
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused(&format!("policy \"{}\"", policy.name), &error.into()),
        }
    }

    for contact in &bundle.contacts {
        let Some(user) = user_id(&contact.user) else {
            report.skipped += 1;
            report.notes.push(format!("contact of \"{}\": unknown account", contact.user));
            continue;
        };
        let via = match contact.via_channel.as_deref() {
            None => None,
            Some(name) => match channels.get(&name.to_lowercase()) {
                Some(id) => Some(*id),
                None => {
                    report.skipped += 1;
                    report.notes.push(format!(
                        "contact \"{}\" of \"{}\": team channel \"{name}\" not found",
                        contact.label, contact.user
                    ));
                    continue;
                }
            },
        };
        let secrets = contact
            .secrets
            .as_object()
            .filter(|map| !map.is_empty())
            .map(|_| serde_json::to_vec(&contact.secrets))
            .transpose()?
            .map(|plain| cipher.encrypt(&plain))
            .transpose()?;
        let settings =
            if contact.settings.is_object() { contact.settings.clone() } else { json!({}) };
        let severity = crate::alerting::model::Severity::parse(&contact.min_severity);
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM oncall_contacts WHERE user_id = ? AND label = ?")
                .bind(user)
                .bind(&contact.label)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche d'un contact")?;
        let result = match existing {
            Some((id,)) => sqlx::query(
                "UPDATE oncall_contacts SET kind = ?, via_channel_id = ?, address = ?, settings = ?,
                     secret_enc = ?, enabled = ?, min_severity = ?
                 WHERE id = ?",
            )
            .bind(&contact.kind)
            .bind(via)
            .bind(&contact.address)
            .bind(settings.to_string())
            .bind(&secrets)
            .bind(i64::from(contact.enabled))
            .bind(severity.as_str())
            .bind(id)
            .execute(&mut **tx)
            .await,
            None => sqlx::query(
                "INSERT INTO oncall_contacts
                     (user_id, label, kind, via_channel_id, address, settings, secret_enc, enabled,
                      min_severity)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(user)
            .bind(&contact.label)
            .bind(&contact.kind)
            .bind(via)
            .bind(&contact.address)
            .bind(settings.to_string())
            .bind(&secrets)
            .bind(i64::from(contact.enabled))
            .bind(severity.as_str())
            .execute(&mut **tx)
            .await,
        };
        match result {
            Ok(_) if existing.is_some() => report.count(RestoreOutcome::Updated),
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused(&format!("contact \"{}\"", contact.label), &error.into()),
        }
    }

    for prefs in &bundle.prefs {
        let Some(user) = user_id(&prefs.user) else { continue };
        let quiet = prefs.quiet_hours.as_ref().map(Value::to_string);
        sqlx::query(
            "INSERT INTO oncall_user_prefs (user_id, timezone, quiet_hours) VALUES (?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET timezone = excluded.timezone,
                 quiet_hours = excluded.quiet_hours",
        )
        .bind(user)
        .bind(&prefs.timezone)
        .bind(quiet)
        .execute(&mut **tx)
        .await
        .context("préférences d'astreinte")?;
    }
    Ok(report)
}
