//! Journal d'événements Windows : règles par machine, événements reçus, alertes.
//!
//! Les règles d'une machine partent dans l'accusé de réception de chacun de
//! ses lots ([`agent_rules`]) ; l'agent renvoie les événements qui y répondent
//! dans son lot suivant ([`record`]). Le serveur en garde les derniers pour la
//! page de la machine, et en tire une jauge par sévérité ([`gauges`]) :
//! `dumbmonit_agent_eventlog_matches_<sévérité>` vaut le nombre d'événements
//! reçus dans les quinze dernières minutes. Les règles livrées
//! `windows_event_<sévérité>` déclenchent dès qu'elle passe au-dessus de zéro,
//! et se résolvent quinze minutes après le dernier événement.
//!
//! Une seule alerte par machine et par sévérité : une rafale ne fait pas une
//! alerte par événement. Le fait le plus récent est cité dans le message (voir
//! `alerting::cycle::Occurrence`) ; une alerte déjà partie reparle au plus
//! toutes les [`ANNOUNCE_EVERY_MS`] quand de nouveaux événements arrivent.

use anyhow::{Context, Result};
use dumbmonit_proto::{
    EventLevel, EventLogReport, EventLogRule, MAX_EVENT_LOG_ENTRIES, MAX_EVENT_MESSAGE_CHARS,
    MetricKind, Sample, Target, TargetId, format_event_ids, parse_event_ids, truncate_message,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::alerting::model::Severity;

/// Fenêtre de la jauge : une alerte se résout quinze minutes après le dernier
/// événement.
pub const WINDOW_MS: i64 = 15 * 60 * 1000;

/// Délai minimal entre deux annonces d'une même alerte active.
pub const ANNOUNCE_EVERY_MS: i64 = 10 * 60 * 1000;

/// Événements gardés par machine pour la page de l'équipement.
pub const KEEP_PER_HOST: i64 = 500;

/// Règles par machine, au plus.
pub const MAX_RULES_PER_HOST: i64 = 50;

/// Longueur maximale d'un nom, d'un journal, d'un fournisseur ou d'un motif.
const MAX_FIELD: usize = 256;

/// Longueur du résumé cité dans les notifications.
const DETAIL_CHARS: usize = 300;

/// Les sévérités, chacune avec sa règle livrée et sa jauge.
pub const SEVERITIES: [Severity; 3] = [Severity::Critical, Severity::Warning, Severity::Info];

/// `uid` de la règle livrée d'une sévérité.
pub fn alert_rule(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "windows_event_critical",
        Severity::Warning => "windows_event_warning",
        Severity::Info => "windows_event_info",
    }
}

/// Jauge d'une sévérité, sans le préfixe `dumbmonit_`.
pub fn gauge_metric(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "agent_eventlog_matches_critical",
        Severity::Warning => "agent_eventlog_matches_warning",
        Severity::Info => "agent_eventlog_matches_info",
    }
}

/// Une règle telle que l'API la rend.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventRule {
    pub id: i64,
    pub target_id: TargetId,
    pub name: String,
    pub channel: String,
    pub provider: Option<String>,
    /// Forme canonique, vide pour « tous ».
    pub event_ids: String,
    pub levels: Vec<EventLevel>,
    pub pattern: Option<String>,
    pub pattern_regex: bool,
    pub severity: Severity,
    pub enabled: bool,
    pub matches_total: i64,
    pub dropped_total: i64,
    /// Dernier événement reçu, UTC sans suffixe.
    pub last_match_at: Option<String>,
}

/// Ce que l'API accepte pour créer ou remplacer une règle.
#[derive(Debug, Clone, Deserialize)]
pub struct EventRuleInput {
    pub name: String,
    pub channel: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub event_ids: String,
    #[serde(default)]
    pub levels: Vec<EventLevel>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub pattern_regex: bool,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

/// Une règle vérifiée, prête à ranger.
#[derive(Debug, Clone, PartialEq)]
struct Clean {
    name: String,
    channel: String,
    provider: Option<String>,
    event_ids: String,
    levels: String,
    pattern: Option<String>,
    pattern_regex: bool,
    severity: Severity,
    enabled: bool,
}

/// Vérifie une saisie ; le message d'erreur s'adresse à l'utilisateur.
fn validate(input: &EventRuleInput) -> Result<Clean, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > MAX_FIELD {
        return Err(format!("The rule name is required ({MAX_FIELD} characters at most)."));
    }
    let channel = input.channel.trim().to_string();
    if channel.is_empty() || channel.chars().count() > MAX_FIELD {
        return Err(format!("The event log is required ({MAX_FIELD} characters at most)."));
    }
    if channel.chars().any(char::is_control) {
        return Err("The event log name contains control characters.".to_string());
    }
    let provider = input.provider.as_deref().map(str::trim).filter(|p| !p.is_empty());
    if let Some(provider) = provider {
        // Le fournisseur entre tel quel dans la requête XPath de l'agent.
        let forbidden = |c: char| matches!(c, '\'' | '"' | '[' | ']' | '<' | '>' | '&');
        if provider.chars().count() > MAX_FIELD
            || provider.chars().any(|c| forbidden(c) || c.is_control())
        {
            return Err(
                "The source must be a provider name, without quotes or brackets.".to_string()
            );
        }
    }
    let ranges = parse_event_ids(&input.event_ids)?;
    let mut levels = input.levels.clone();
    levels.sort_unstable();
    levels.dedup();
    if ranges.is_empty() && provider.is_none() && levels.is_empty() {
        return Err("Pick at least an event ID, a source or a level: a rule on a whole log \
                    would report every event."
            .to_string());
    }
    let pattern = input.pattern.as_deref().map(str::trim).filter(|p| !p.is_empty());
    if let Some(pattern) = pattern {
        if pattern.chars().count() > MAX_FIELD {
            return Err(format!("The pattern is limited to {MAX_FIELD} characters."));
        }
        if input.pattern_regex
            && let Err(error) = regex::Regex::new(pattern)
        {
            return Err(format!("The pattern is not a valid regular expression: {error}"));
        }
    }
    Ok(Clean {
        name,
        channel,
        provider: provider.map(str::to_string),
        event_ids: format_event_ids(&ranges),
        levels: levels.iter().map(|l| l.as_str()).collect::<Vec<_>>().join(","),
        pattern: pattern.map(str::to_string),
        pattern_regex: input.pattern_regex && pattern.is_some(),
        severity: input.severity,
        enabled: input.enabled,
    })
}

fn parse_levels(raw: &str) -> Vec<EventLevel> {
    raw.split(',').filter_map(EventLevel::parse).collect()
}

fn utc(ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ms).map(|at| at.format("%Y-%m-%dT%H:%M:%S").to_string())
}

const COLUMNS: &str = "id, target_id, name, channel, provider, event_ids, levels, pattern, \
     pattern_regex, severity, enabled, matches_total, dropped_total, last_match_ms";

fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<EventRule> {
    let severity: String = row.try_get("severity")?;
    let levels: String = row.try_get("levels")?;
    Ok(EventRule {
        id: row.try_get("id")?,
        target_id: row.try_get("target_id")?,
        name: row.try_get("name")?,
        channel: row.try_get("channel")?,
        provider: row.try_get("provider")?,
        event_ids: row.try_get("event_ids")?,
        levels: parse_levels(&levels),
        pattern: row.try_get("pattern")?,
        pattern_regex: row.try_get::<i64, _>("pattern_regex")? != 0,
        severity: Severity::parse(&severity),
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        matches_total: row.try_get("matches_total")?,
        dropped_total: row.try_get("dropped_total")?,
        last_match_at: row.try_get::<Option<i64>, _>("last_match_ms")?.and_then(utc),
    })
}

/// Les règles d'une machine, dans l'ordre de création.
pub async fn list(pool: &SqlitePool, host: TargetId) -> Result<Vec<EventRule>> {
    let rows = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM agent_event_rules WHERE target_id = ? ORDER BY id"
    ))
    .bind(host)
    .fetch_all(pool)
    .await
    .context("reading the event log rules")?;
    rows.iter().map(from_row).collect()
}

/// Une règle de cette machine.
pub async fn get(pool: &SqlitePool, host: TargetId, id: i64) -> Result<Option<EventRule>> {
    let row = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM agent_event_rules WHERE target_id = ? AND id = ?"
    ))
    .bind(host)
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("reading an event log rule")?;
    row.as_ref().map(from_row).transpose()
}

/// Ce qui peut mal tourner à l'enregistrement d'une règle.
#[derive(Debug)]
pub enum SaveError {
    Invalid(String),
    TooMany,
    NotFound,
    Internal(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for SaveError {
    fn from(error: E) -> Self {
        Self::Internal(error.into())
    }
}

/// Crée une règle pour cette machine.
pub async fn create(
    pool: &SqlitePool,
    host: TargetId,
    input: &EventRuleInput,
) -> Result<EventRule, SaveError> {
    let clean = validate(input).map_err(SaveError::Invalid)?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_event_rules WHERE target_id = ?")
            .bind(host)
            .fetch_one(pool)
            .await?;
    if count >= MAX_RULES_PER_HOST {
        return Err(SaveError::TooMany);
    }
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO agent_event_rules
             (target_id, name, channel, provider, event_ids, levels, pattern, pattern_regex,
              severity, enabled)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(host)
    .bind(&clean.name)
    .bind(&clean.channel)
    .bind(&clean.provider)
    .bind(&clean.event_ids)
    .bind(&clean.levels)
    .bind(&clean.pattern)
    .bind(i64::from(clean.pattern_regex))
    .bind(clean.severity.as_str())
    .bind(i64::from(clean.enabled))
    .fetch_one(pool)
    .await?;
    get(pool, host, id).await?.ok_or(SaveError::NotFound)
}

/// Remplace une règle de cette machine. Ses compteurs sont gardés.
pub async fn update(
    pool: &SqlitePool,
    host: TargetId,
    id: i64,
    input: &EventRuleInput,
) -> Result<EventRule, SaveError> {
    let clean = validate(input).map_err(SaveError::Invalid)?;
    let changed = sqlx::query(
        "UPDATE agent_event_rules
         SET name = ?, channel = ?, provider = ?, event_ids = ?, levels = ?, pattern = ?,
             pattern_regex = ?, severity = ?, enabled = ?, updated_at = datetime('now')
         WHERE target_id = ? AND id = ?",
    )
    .bind(&clean.name)
    .bind(&clean.channel)
    .bind(&clean.provider)
    .bind(&clean.event_ids)
    .bind(&clean.levels)
    .bind(&clean.pattern)
    .bind(i64::from(clean.pattern_regex))
    .bind(clean.severity.as_str())
    .bind(i64::from(clean.enabled))
    .bind(host)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(SaveError::NotFound);
    }
    get(pool, host, id).await?.ok_or(SaveError::NotFound)
}

/// Supprime une règle ; faux si elle n'existait pas. Les événements reçus
/// restent dans l'historique.
pub async fn delete(pool: &SqlitePool, host: TargetId, id: i64) -> Result<bool> {
    let removed = sqlx::query("DELETE FROM agent_event_rules WHERE target_id = ? AND id = ?")
        .bind(host)
        .bind(id)
        .execute(pool)
        .await
        .context("deleting an event log rule")?
        .rows_affected();
    Ok(removed > 0)
}

/// Les règles actives d'une machine, telles que l'agent les applique.
pub async fn agent_rules(pool: &SqlitePool, host: TargetId) -> Result<Vec<EventLogRule>> {
    Ok(list(pool, host)
        .await?
        .into_iter()
        .filter(|rule| rule.enabled)
        .map(|rule| EventLogRule {
            id: rule.id,
            channel: rule.channel,
            provider: rule.provider,
            // Rangée sous forme canonique : se relit toujours.
            event_ids: parse_event_ids(&rule.event_ids).unwrap_or_default(),
            levels: rule.levels,
            pattern: rule.pattern,
            regex: rule.pattern_regex,
        })
        .collect())
}

/// Un événement reçu, tel que la page de la machine l'affiche.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventView {
    pub id: i64,
    pub rule_id: Option<i64>,
    pub rule_name: String,
    pub severity: Severity,
    pub channel: String,
    pub provider: String,
    pub event_id: i64,
    /// Niveau lisible ; `None` pour « Verbose ».
    pub level: Option<EventLevel>,
    pub record_id: i64,
    pub computer: String,
    pub message: String,
    /// `TimeCreated` de l'événement, UTC sans suffixe.
    pub occurred_at: Option<String>,
    pub received_at: Option<String>,
}

/// Les derniers événements reçus d'une machine, du plus récent au plus ancien.
pub async fn recent(pool: &SqlitePool, host: TargetId, limit: i64) -> Result<Vec<EventView>> {
    let rows = sqlx::query(
        "SELECT id, rule_id, rule_name, severity, channel, provider, event_id, level, record_id,
                computer, message, occurred_ms, received_ms
         FROM agent_event_log WHERE target_id = ? ORDER BY id DESC LIMIT ?",
    )
    .bind(host)
    .bind(limit.clamp(1, KEEP_PER_HOST))
    .fetch_all(pool)
    .await
    .context("reading the received events")?;
    rows.iter()
        .map(|row| {
            let severity: String = row.try_get("severity")?;
            let level: i64 = row.try_get("level")?;
            Ok(EventView {
                id: row.try_get("id")?,
                rule_id: row.try_get("rule_id")?,
                rule_name: row.try_get("rule_name")?,
                severity: Severity::parse(&severity),
                channel: row.try_get("channel")?,
                provider: row.try_get("provider")?,
                event_id: row.try_get("event_id")?,
                level: u8::try_from(level).ok().and_then(EventLevel::from_windows),
                record_id: row.try_get("record_id")?,
                computer: row.try_get("computer")?,
                message: row.try_get("message")?,
                occurred_at: utc(row.try_get("occurred_ms")?),
                received_at: utc(row.try_get("received_ms")?),
            })
        })
        .collect()
}

/// Coupe un champ court reçu de l'agent.
fn short(text: &str) -> String {
    truncate_message(text.trim(), MAX_FIELD)
}

/// Ce qu'une notification dit d'un événement.
fn detail(rule_name: &str, event_id: u32, provider: &str, message: &str) -> String {
    let first_line = message.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let mut text = format!("{rule_name} · {event_id}");
    if !provider.is_empty() {
        text.push(' ');
        text.push_str(provider);
    }
    if !first_line.is_empty() {
        text.push_str(": ");
        text.push_str(first_line);
    }
    truncate_message(&text, DETAIL_CHARS)
}

/// Range ce qu'un lot rapporte du journal d'événements : les événements des
/// règles de cette machine encore actives, et les compteurs de chaque règle.
/// Ce qui vise une règle inconnue (supprimée entre-temps) est ignoré.
pub async fn record(
    pool: &SqlitePool,
    host: TargetId,
    report: &EventLogReport,
    received_ms: i64,
) -> Result<usize> {
    let rules = list(pool, host).await?;
    let find = |id: i64| rules.iter().find(|rule| rule.id == id && rule.enabled);
    let mut stored = 0;
    let mut tx = pool.begin().await?;
    // Le plus récent de chaque sévérité, pour le résumé de l'alerte.
    let mut latest: Vec<(Severity, String)> = Vec::new();
    for event in report.events.iter().take(MAX_EVENT_LOG_ENTRIES) {
        let Some(rule) = find(event.rule_id) else { continue };
        let message = truncate_message(event.message.trim(), MAX_EVENT_MESSAGE_CHARS);
        let provider = short(&event.provider);
        // Une horloge déréglée ne doit pas fausser l'ordre : on garde ce que
        // l'événement dit, borné à l'instant de réception.
        let occurred_ms = if event.time_created_ms > 0 {
            event.time_created_ms.min(received_ms)
        } else {
            received_ms
        };
        sqlx::query(
            "INSERT INTO agent_event_log
                 (target_id, rule_id, rule_name, severity, channel, provider, event_id, level,
                  record_id, computer, message, occurred_ms, received_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(host)
        .bind(rule.id)
        .bind(&rule.name)
        .bind(rule.severity.as_str())
        .bind(short(&event.channel))
        .bind(&provider)
        .bind(i64::from(event.event_id))
        .bind(i64::from(event.level))
        .bind(i64::try_from(event.record_id).unwrap_or(i64::MAX))
        .bind(short(&event.computer))
        .bind(&message)
        .bind(occurred_ms)
        .bind(received_ms)
        .execute(&mut *tx)
        .await
        .context("storing a received event")?;
        sqlx::query(
            "UPDATE agent_event_rules
             SET matches_total = matches_total + 1, last_match_ms = ? WHERE id = ?",
        )
        .bind(received_ms)
        .bind(rule.id)
        .execute(&mut *tx)
        .await?;
        let summary = detail(&rule.name, event.event_id, &provider, &message);
        match latest.iter_mut().find(|(severity, _)| *severity == rule.severity) {
            Some(entry) => entry.1 = summary,
            None => latest.push((rule.severity, summary)),
        }
        stored += 1;
    }
    for dropped in &report.dropped {
        if find(dropped.rule_id).is_none() {
            continue;
        }
        sqlx::query("UPDATE agent_event_rules SET dropped_total = dropped_total + ? WHERE id = ?")
            .bind(i64::try_from(dropped.count).unwrap_or(i64::MAX))
            .bind(dropped.rule_id)
            .execute(&mut *tx)
            .await?;
    }
    for (severity, summary) in latest {
        // Le fait n'est annoncé de nouveau qu'au-delà du délai : une rafale
        // donne une notification, le résumé suit le dernier événement.
        sqlx::query(
            "INSERT INTO agent_event_alerts (target_id, severity, announced_ms, detail)
             VALUES (?, ?, ?, ?)
             ON CONFLICT (target_id, severity) DO UPDATE SET
                 detail = excluded.detail,
                 announced_ms = CASE
                     WHEN agent_event_alerts.announced_ms IS NULL
                       OR agent_event_alerts.matches = 0
                       OR excluded.announced_ms - agent_event_alerts.announced_ms >= ?
                     THEN excluded.announced_ms
                     ELSE agent_event_alerts.announced_ms END",
        )
        .bind(host)
        .bind(severity.as_str())
        .bind(received_ms)
        .bind(&summary)
        .bind(ANNOUNCE_EVERY_MS)
        .execute(&mut *tx)
        .await
        .context("updating the event alert")?;
    }
    sqlx::query(
        "DELETE FROM agent_event_log WHERE target_id = ? AND id NOT IN (
             SELECT id FROM agent_event_log WHERE target_id = ? ORDER BY id DESC LIMIT ?)",
    )
    .bind(host)
    .bind(host)
    .bind(KEEP_PER_HOST)
    .execute(&mut *tx)
    .await
    .context("pruning the received events")?;
    tx.commit().await?;
    Ok(stored)
}

/// Les jauges d'une machine qui a des règles : le nombre d'événements de
/// chaque sévérité des quinze dernières minutes, à l'heure du lot. Rien pour
/// une machine sans règle. La même valeur est gardée pour l'alerte, qui ne
/// reparle d'un fait que si la jauge l'a déjà enregistré.
pub async fn gauges(pool: &SqlitePool, target: &Target, now_ms: i64) -> Result<Vec<Sample>> {
    let rules: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_event_rules WHERE target_id = ?")
            .bind(target.id)
            .fetch_one(pool)
            .await?;
    let alerts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_event_alerts WHERE target_id = ?")
            .bind(target.id)
            .fetch_one(pool)
            .await?;
    if rules == 0 && alerts == 0 {
        return Ok(Vec::new());
    }
    let labels = target.base_labels();
    let mut samples = Vec::with_capacity(SEVERITIES.len());
    for severity in SEVERITIES {
        let matches: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_event_log
             WHERE target_id = ? AND severity = ? AND received_ms > ?",
        )
        .bind(target.id)
        .bind(severity.as_str())
        .bind(now_ms - WINDOW_MS)
        .fetch_one(pool)
        .await?;
        sqlx::query(
            "UPDATE agent_event_alerts SET matches = ? WHERE target_id = ? AND severity = ?",
        )
        .bind(matches)
        .bind(target.id)
        .bind(severity.as_str())
        .execute(pool)
        .await?;
        let mut sample =
            Sample::new(gauge_metric(severity), matches as f64, MetricKind::Gauge, now_ms);
        sample.labels = labels.clone();
        samples.push(sample);
    }
    // Plus aucune règle et plus rien de récent : la machine sort du calcul.
    if rules == 0 {
        sqlx::query("DELETE FROM agent_event_alerts WHERE target_id = ? AND matches = 0")
            .bind(target.id)
            .execute(pool)
            .await?;
    }
    Ok(samples)
}

/// Un fait à annoncer, pour le moteur d'alerte.
#[derive(Debug, Clone, PartialEq)]
pub struct EventOccurrence {
    /// `uid` de la règle livrée de la sévérité.
    pub rule_uid: &'static str,
    pub target_id: TargetId,
    /// Instant de la dernière annonce (ms depuis l'époque).
    pub announced_ms: i64,
    /// Valeur de la jauge écrite au même moment.
    pub matches: i64,
    pub detail: String,
}

/// Les faits à annoncer : une ligne par machine et par sévérité active.
pub async fn occurrences(pool: &SqlitePool) -> Result<Vec<EventOccurrence>> {
    let rows = sqlx::query(
        "SELECT target_id, severity, matches, announced_ms, detail FROM agent_event_alerts
         WHERE announced_ms IS NOT NULL AND matches > 0",
    )
    .fetch_all(pool)
    .await
    .context("reading the event alerts")?;
    rows.iter()
        .map(|row| {
            let severity: String = row.try_get("severity")?;
            Ok(EventOccurrence {
                rule_uid: alert_rule(Severity::parse(&severity)),
                target_id: row.try_get("target_id")?,
                announced_ms: row.try_get("announced_ms")?,
                matches: row.try_get("matches")?,
                detail: row.try_get::<Option<String>, _>("detail")?.unwrap_or_default(),
            })
        })
        .collect()
}

/// Un exemple proposé par l'interface, jamais créé d'office.
#[derive(Debug, Clone, Serialize)]
pub struct EventRulePreset {
    /// Clé stable, pour les textes de l'interface.
    pub id: &'static str,
    pub name: &'static str,
    pub channel: &'static str,
    pub provider: Option<&'static str>,
    pub event_ids: &'static str,
    pub levels: &'static [EventLevel],
    pub severity: Severity,
}

/// Les exemples proposés à l'ajout d'une règle.
pub const PRESETS: &[EventRulePreset] = &[
    EventRulePreset {
        id: "failed_logon",
        name: "Failed logons",
        channel: "Security",
        provider: Some("Microsoft-Windows-Security-Auditing"),
        event_ids: "4625",
        levels: &[],
        severity: Severity::Warning,
    },
    EventRulePreset {
        id: "account_locked",
        name: "Account locked out",
        channel: "Security",
        provider: Some("Microsoft-Windows-Security-Auditing"),
        event_ids: "4740",
        levels: &[],
        severity: Severity::Warning,
    },
    EventRulePreset {
        id: "unexpected_reboot",
        name: "Unexpected reboot",
        channel: "System",
        provider: Some("Microsoft-Windows-Kernel-Power"),
        event_ids: "41",
        levels: &[],
        severity: Severity::Critical,
    },
    EventRulePreset {
        id: "service_crashed",
        name: "Service terminated unexpectedly",
        channel: "System",
        provider: Some("Service Control Manager"),
        event_ids: "7031, 7034",
        levels: &[],
        severity: Severity::Warning,
    },
    EventRulePreset {
        id: "disk_errors",
        name: "Disk errors",
        channel: "System",
        provider: None,
        event_ids: "7, 11, 51, 153",
        levels: &[EventLevel::Error, EventLevel::Warning],
        severity: Severity::Critical,
    },
    EventRulePreset {
        id: "log_cleared",
        name: "Security log cleared",
        channel: "Security",
        provider: None,
        event_ids: "1102",
        levels: &[],
        severity: Severity::Critical,
    },
    EventRulePreset {
        id: "application_errors",
        name: "Application errors",
        channel: "Application",
        provider: None,
        event_ids: "",
        levels: &[EventLevel::Critical, EventLevel::Error],
        severity: Severity::Info,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn input(event_ids: &str) -> EventRuleInput {
        EventRuleInput {
            name: " Failed logons ".into(),
            channel: "Security".into(),
            provider: Some("  ".into()),
            event_ids: event_ids.into(),
            levels: vec![EventLevel::Information, EventLevel::Information],
            pattern: Some(String::new()),
            pattern_regex: true,
            severity: Severity::Warning,
            enabled: true,
        }
    }

    #[test]
    fn une_saisie_est_nettoyee_avant_d_etre_rangee() {
        let clean = validate(&input("4625,4740")).unwrap();
        assert_eq!(clean.name, "Failed logons");
        assert_eq!(clean.provider, None, "un fournisseur vide veut dire tous");
        assert_eq!(clean.event_ids, "4625, 4740");
        assert_eq!(clean.levels, "information");
        assert_eq!(clean.pattern, None);
        assert!(!clean.pattern_regex, "pas d'expression sans motif");
    }

    #[test]
    fn une_regle_sans_aucun_filtre_est_refusee() {
        let mut empty = input("");
        empty.levels.clear();
        assert!(validate(&empty).is_err());
    }

    #[test]
    fn un_fournisseur_qui_casserait_la_requete_est_refuse() {
        let mut quoted = input("41");
        quoted.provider = Some("Kernel']] or *[(".into());
        assert!(validate(&quoted).is_err());
    }

    #[test]
    fn une_expression_reguliere_invalide_est_refusee() {
        let mut bad = input("4625");
        bad.pattern = Some("(unclosed".into());
        assert!(validate(&bad).is_err());
        bad.pattern_regex = false;
        assert!(validate(&bad).is_ok(), "en sous-chaîne, tout motif est valable");
    }

    #[test]
    fn le_resume_cite_la_regle_l_identifiant_et_la_premiere_ligne() {
        let text = detail(
            "Failed logons",
            4625,
            "Microsoft-Windows-Security-Auditing",
            "\r\nAn account failed to log on.\r\n\r\nSubject: …",
        );
        assert_eq!(
            text,
            "Failed logons · 4625 Microsoft-Windows-Security-Auditing: \
             An account failed to log on."
        );
        assert!(detail("r", 1, "", &"x".repeat(1000)).chars().count() <= DETAIL_CHARS + 1);
    }

    #[test]
    fn les_exemples_sont_des_regles_valables() {
        for preset in PRESETS {
            let input = EventRuleInput {
                name: preset.name.into(),
                channel: preset.channel.into(),
                provider: preset.provider.map(str::to_string),
                event_ids: preset.event_ids.into(),
                levels: preset.levels.to_vec(),
                pattern: None,
                pattern_regex: false,
                severity: preset.severity,
                enabled: true,
            };
            assert!(validate(&input).is_ok(), "{}", preset.id);
        }
    }

    #[test]
    fn chaque_severite_a_sa_regle_et_sa_jauge() {
        for severity in SEVERITIES {
            assert!(alert_rule(severity).ends_with(severity.as_str()));
            assert!(gauge_metric(severity).ends_with(severity.as_str()));
        }
    }
}
