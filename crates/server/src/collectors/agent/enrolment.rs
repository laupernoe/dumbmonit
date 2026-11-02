//! Garde de l'enrôlement des agents.
//!
//! Un jeton d'enrôlement ne sert qu'une fois par machine : ensuite l'agent
//! parle sous son propre certificat client. Le risque est donc l'enrôlement
//! lui-même — un jeton de flotte qui fuit, surtout s'il n'expire jamais, permet
//! d'inscrire de faux agents. Trois protections, toutes ici :
//!
//! 1. **validation** des nouvelles machines ([`ApprovalMode`]) : selon le
//!    réglage, une machine inconnue entre d'office, ou attend qu'un
//!    administrateur l'accepte. Tant qu'elle attend, rien n'existe pour elle :
//!    ni cible, ni mesure, ni certificat. Au plus [`MAX_PENDING`] en attente ;
//! 2. **débit** par jeton (une heure glissante) et **alerte** groupée à chaque
//!    enrôlement, accepté ou en attente : un message par minute, pas un par
//!    machine ;
//! 3. **mise en veille** d'un jeton de flotte resté sans enrôlement pendant N
//!    jours : il cesse d'enrôler, une notification part, un bouton le remet en
//!    service. Les machines déjà enrôlées ne sont jamais touchées.
//!
//! Les décisions sont des fonctions pures ([`verdict`], [`dormant_due`],
//! [`over_rate`], [`compose`]) ; le reste va chercher l'état en base.

use std::net::IpAddr;

use anyhow::{Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use dumbmonit_proto::AgentIdentity;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tracing::{info, warn};

use super::store::hostname_in_domain;
use crate::alerting::model::Severity;
use crate::state::AppState;

/// Clé du réglage dans la table `settings`.
pub const SETTINGS_KEY: &str = "agent.enrolment";

/// Plafond des machines en attente de validation. Au-delà, une nouvelle
/// demande est refusée (429) : un jeton qui fuit ne doit pas noyer la liste.
pub const MAX_PENDING: i64 = 200;

/// Durée pendant laquelle une machine refusée n'est plus montrée, en heures.
pub const REJECT_HOLD_HOURS: i64 = 24;

/// Débit par défaut : enrôlements par jeton et par heure glissante.
pub const DEFAULT_RATE_PER_HOUR: u32 = 60;

/// Débit maximal réglable : au-delà, autant couper la limite (0).
pub const MAX_RATE_PER_HOUR: u32 = 10_000;

/// Inactivité par défaut avant la mise en veille d'un jeton de flotte, en jours.
pub const DEFAULT_DORMANT_DAYS: u32 = 30;

/// Inactivité maximale réglable, en jours (dix ans).
pub const MAX_DORMANT_DAYS: u32 = 3_650;

/// Une demande en attente que la machine ne renouvelle plus est oubliée après
/// ce délai : elle ne bloque pas le plafond indéfiniment.
const STALE_REQUEST_DAYS: i64 = 7;

/// Les événements servent au débit (une heure) et au message groupé : deux
/// jours suffisent largement.
const EVENT_RETENTION_HOURS: i64 = 48;

/// Nombre de noms cités dans le message groupé.
pub const NOTICE_NAMES: usize = 5;

/// Longueur maximale d'un nom d'hôte conservé : il vient d'une machine que le
/// serveur ne connaît pas encore.
const MAX_HOSTNAME_LEN: usize = 253;

/// Ce que le serveur fait d'une machine inconnue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Elle entre d'office (comportement d'avant cette version).
    Auto,
    /// Elle attend qu'un administrateur l'accepte.
    Manual,
    /// Elle entre d'office si son nom complet est dans le domaine Active
    /// Directory du jeton ; sinon, elle attend.
    AdDomain,
}

/// Réglages de la garde, tels que rangés sous [`SETTINGS_KEY`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnrolmentSettings {
    /// Jetons de flotte (réutilisables).
    pub fleet_approval: ApprovalMode,
    /// Jetons à usage unique : l'administrateur l'a créé pour cette machine.
    pub single_use_approval: ApprovalMode,
    /// Enrôlements par jeton et par heure ; 0 : sans limite.
    pub rate_per_hour: u32,
    /// Jours sans enrôlement avant la mise en veille d'un jeton de flotte ;
    /// 0 : jamais.
    pub dormant_days: u32,
}

impl Default for EnrolmentSettings {
    /// Valeurs d'une installation neuve. Une instance qui avait déjà des jetons
    /// avant cette version a reçu `auto` partout (migration 0233).
    fn default() -> Self {
        Self {
            fleet_approval: ApprovalMode::Manual,
            single_use_approval: ApprovalMode::Auto,
            rate_per_hour: DEFAULT_RATE_PER_HOUR,
            dormant_days: DEFAULT_DORMANT_DAYS,
        }
    }
}

impl EnrolmentSettings {
    /// Comportement d'avant la garde : acceptation automatique partout. C'est
    /// ce que reçoit une instance qui avait déjà des jetons (migration 0233),
    /// et une restauration d'une sauvegarde de cette époque.
    pub fn legacy() -> Self {
        Self {
            fleet_approval: ApprovalMode::Auto,
            single_use_approval: ApprovalMode::Auto,
            ..Self::default()
        }
    }

    /// Mode applicable à un jeton.
    pub fn mode_for(&self, fleet: bool) -> ApprovalMode {
        if fleet { self.fleet_approval } else { self.single_use_approval }
    }

    /// Bornes des valeurs saisies.
    pub fn validate(&self) -> Result<(), String> {
        if self.rate_per_hour > MAX_RATE_PER_HOUR {
            return Err(format!(
                "The enrolment rate is limited to {MAX_RATE_PER_HOUR} per hour (0 for no limit)."
            ));
        }
        if self.dormant_days > MAX_DORMANT_DAYS {
            return Err(format!(
                "The inactivity delay is limited to {MAX_DORMANT_DAYS} days (0 for never)."
            ));
        }
        Ok(())
    }
}

/// Vrai pour un jeton de flotte : tout jeton qui n'est pas à usage unique.
pub fn is_fleet(max_uses: Option<i64>) -> bool {
    max_uses != Some(1)
}

/// Verdict de la validation pour une machine inconnue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accept,
    Ask,
}

/// Décide si une machine inconnue entre d'office ou attend.
///
/// `token_domain` : domaine Active Directory du jeton, déjà normalisé.
pub fn verdict(mode: ApprovalMode, token_domain: Option<&str>, hostname: &str) -> Verdict {
    match mode {
        ApprovalMode::Auto => Verdict::Accept,
        ApprovalMode::Manual => Verdict::Ask,
        ApprovalMode::AdDomain => match token_domain {
            Some(domain) if hostname_in_domain(hostname, domain) => Verdict::Accept,
            _ => Verdict::Ask,
        },
    }
}

/// Vrai si un jeton de flotte inactif depuis `activity` doit être mis en
/// veille. `days` à 0 : jamais.
pub fn dormant_due(activity: DateTime<Utc>, days: u32, now: DateTime<Utc>) -> bool {
    days > 0 && now - activity >= TimeDelta::days(i64::from(days))
}

/// Vrai si `recent` enrôlements dans l'heure atteignent la limite. `rate` à 0 :
/// sans limite.
pub fn over_rate(recent: i64, rate: u32) -> bool {
    rate > 0 && recent >= i64::from(rate)
}

/// Vrai si un refus pèse encore sur une machine qui revient : moins de
/// [`REJECT_HOLD_HOURS`] après la décision, et avec le même jeton. Un autre
/// jeton, c'est une nouvelle action de l'administrateur.
pub fn rejection_holds(
    decided_at: Option<DateTime<Utc>>,
    rejected_token: Option<i64>,
    token_id: i64,
    now: DateTime<Utc>,
) -> bool {
    rejected_token == Some(token_id)
        && decided_at.is_some_and(|at| now - at < TimeDelta::hours(REJECT_HOLD_HOURS))
}

/// D'où vient une demande d'enrôlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrolOrigin {
    /// Un agent, par le réseau : la garde s'applique.
    Network { source_ip: Option<IpAddr> },
    /// Le serveur lui-même (données de démonstration) : rien ne vient du
    /// réseau, il n'y a rien à garder. Jamais pour une requête HTTP.
    Internal,
}

impl EnrolOrigin {
    /// Demande reçue d'une adresse.
    pub fn network(source_ip: Option<IpAddr>) -> Self {
        Self::Network { source_ip }
    }
}

/// Pourquoi une machine inconnue n'entre pas (encore).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Elle attend la validation d'un administrateur.
    Pending,
    /// Un administrateur l'a refusée il y a moins de 24 h.
    Rejected,
    /// Trop de machines attendent déjà.
    TooManyPending,
    /// Le jeton a dépassé son débit horaire.
    RateLimited,
    /// Le jeton de flotte est en veille.
    Dormant,
}

/// Format des horodatages des tables d'agent.
fn stamp(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Relit un horodatage des tables d'agent (avec ou sans `Z`).
fn parse_stamp(raw: &str) -> Option<DateTime<Utc>> {
    let raw = raw.trim().trim_end_matches('Z').replace(' ', "T");
    chrono::NaiveDateTime::parse_from_str(&raw, "%Y-%m-%dT%H:%M:%S").ok().map(|at| at.and_utc())
}

/// Coupe une chaîne venue d'une machine inconnue à une longueur raisonnable.
fn bounded(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

/// Réglages en vigueur (valeurs par défaut s'ils n'ont jamais été écrits).
pub async fn load_settings(pool: &SqlitePool) -> Result<EnrolmentSettings> {
    Ok(crate::auth::settings::get::<EnrolmentSettings>(pool, SETTINGS_KEY)
        .await?
        .unwrap_or_default())
}

/// Range les réglages (déjà validés).
pub async fn save_settings(pool: &SqlitePool, settings: &EnrolmentSettings) -> Result<()> {
    crate::auth::settings::set(pool, SETTINGS_KEY, settings).await
}

/// Type d'un événement du journal court.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Enrolled,
    Pending,
    Burst,
    Dormant,
}

impl EventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Enrolled => "enrolled",
            Self::Pending => "pending",
            Self::Burst => "burst",
            Self::Dormant => "dormant",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "enrolled" => Some(Self::Enrolled),
            "pending" => Some(Self::Pending),
            "burst" => Some(Self::Burst),
            "dormant" => Some(Self::Dormant),
            _ => None,
        }
    }
}

async fn record_event(
    pool: &SqlitePool,
    token_id: Option<i64>,
    hostname: &str,
    kind: EventKind,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO agent_enrolment_events (token_id, hostname, kind, at_ms) VALUES (?, ?, ?, ?)",
    )
    .bind(token_id)
    .bind(bounded(hostname, MAX_HOSTNAME_LEN))
    .bind(kind.as_str())
    .bind(now.timestamp_millis())
    .execute(pool)
    .await
    .context("recording an enrolment event")?;
    Ok(())
}

/// Ce que la garde doit savoir d'un jeton.
struct TokenInfo {
    max_uses: Option<i64>,
    ad_domain: Option<String>,
    dormant: bool,
    activity: Option<DateTime<Utc>>,
}

async fn token_info(pool: &SqlitePool, token_id: i64) -> Result<Option<TokenInfo>> {
    let row = sqlx::query(
        "SELECT max_uses, ad_domain, dormant_at,
                COALESCE(enrol_activity_at, created_at) AS activity
         FROM agent_tokens WHERE id = ?",
    )
    .bind(token_id)
    .fetch_optional(pool)
    .await
    .context("reading the token for the enrolment guard")?;
    row.map(|row| {
        let activity: Option<String> = row.try_get("activity")?;
        Ok(TokenInfo {
            max_uses: row.try_get("max_uses")?,
            ad_domain: row.try_get("ad_domain")?,
            dormant: row.try_get::<Option<String>, _>("dormant_at")?.is_some(),
            activity: activity.as_deref().and_then(parse_stamp),
        })
    })
    .transpose()
}

/// Met un jeton en veille. Vrai s'il ne l'était pas : l'événement (donc la
/// notification) n'est écrit qu'une fois.
async fn put_to_sleep(pool: &SqlitePool, token_id: i64, now: DateTime<Utc>) -> Result<bool> {
    let done = sqlx::query(
        "UPDATE agent_tokens SET dormant_at = ?
         WHERE id = ? AND dormant_at IS NULL AND revoked_at IS NULL",
    )
    .bind(stamp(now))
    .bind(token_id)
    .execute(pool)
    .await
    .context("putting the token to sleep")?;
    let slept = done.rows_affected() > 0;
    if slept {
        record_event(pool, Some(token_id), "", EventKind::Dormant, now).await?;
        info!(jeton = token_id, "jeton de flotte mis en veille : aucun enrôlement récent");
    }
    Ok(slept)
}

/// Met en veille les jetons de flotte restés sans enrôlement `days` jours.
/// Seuls comptent ceux qui pourraient encore enrôler : un jeton révoqué,
/// échu ou épuisé n'a pas à faire parler de lui. Renvoie ceux qui viennent
/// d'être mis en veille.
pub async fn sweep_dormant(pool: &SqlitePool, days: u32, now: DateTime<Utc>) -> Result<Vec<i64>> {
    if days == 0 {
        return Ok(Vec::new());
    }
    let cutoff = stamp(now - TimeDelta::days(i64::from(days)));
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM agent_tokens
         WHERE dormant_at IS NULL AND revoked_at IS NULL
           AND (max_uses IS NULL OR max_uses > 1)
           AND (max_uses IS NULL OR uses < max_uses)
           AND (expires_at IS NULL OR expires_at > ?)
           AND COALESCE(enrol_activity_at, created_at) <= ?",
    )
    .bind(stamp(now))
    .bind(cutoff)
    .fetch_all(pool)
    .await
    .context("looking for idle fleet tokens")?;
    let mut slept = Vec::new();
    for id in ids {
        if put_to_sleep(pool, id, now).await? {
            slept.push(id);
        }
    }
    Ok(slept)
}

/// Remet en service un jeton en veille : son compteur d'inactivité repart de
/// maintenant. Faux s'il est inconnu, révoqué ou pas en veille.
pub async fn reactivate(pool: &SqlitePool, token_id: i64, now: DateTime<Utc>) -> Result<bool> {
    let done = sqlx::query(
        "UPDATE agent_tokens SET dormant_at = NULL, enrol_activity_at = ?
         WHERE id = ? AND revoked_at IS NULL AND dormant_at IS NOT NULL",
    )
    .bind(stamp(now))
    .bind(token_id)
    .execute(pool)
    .await
    .context("reactivating the token")?;
    Ok(done.rows_affected() > 0)
}

/// Jetons en veille, avec le moment de leur mise en veille.
pub async fn dormant_tokens(pool: &SqlitePool) -> Result<std::collections::HashMap<i64, String>> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, dormant_at FROM agent_tokens WHERE dormant_at IS NOT NULL")
            .fetch_all(pool)
            .await
            .context("listing dormant tokens")?;
    Ok(rows.into_iter().collect())
}

/// État d'une demande d'enrôlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestStatus {
    Pending,
    Approved,
    Rejected,
}

struct RequestRow {
    id: i64,
    status: RequestStatus,
    token_id: Option<i64>,
    decided_at: Option<DateTime<Utc>>,
}

async fn request_by_key(pool: &SqlitePool, key: &str) -> Result<Option<RequestRow>> {
    let row = sqlx::query(
        "SELECT id, status, token_id, decided_at FROM agent_enrolment_requests WHERE agent_key = ?",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .context("reading the enrolment request")?;
    let Some(row) = row else { return Ok(None) };
    let status = match row.try_get::<String, _>("status")?.as_str() {
        "approved" => RequestStatus::Approved,
        "rejected" => RequestStatus::Rejected,
        _ => RequestStatus::Pending,
    };
    let decided_at: Option<String> = row.try_get("decided_at")?;
    Ok(Some(RequestRow {
        id: row.try_get("id")?,
        status,
        token_id: row.try_get("token_id")?,
        decided_at: decided_at.as_deref().and_then(parse_stamp),
    }))
}

/// Nombre de machines en attente de validation.
pub async fn pending_count(pool: &SqlitePool) -> Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM agent_enrolment_requests WHERE status = 'pending'")
        .fetch_one(pool)
        .await
        .context("counting pending enrolments")
}

/// Enrôlements (acceptés ou mis en attente) servis par un jeton dans l'heure.
async fn recent_enrolments(pool: &SqlitePool, token_id: i64, now: DateTime<Utc>) -> Result<i64> {
    let since = (now - TimeDelta::hours(1)).timestamp_millis();
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_enrolment_events
         WHERE token_id = ? AND kind IN ('enrolled', 'pending') AND at_ms >= ?",
    )
    .bind(token_id)
    .bind(since)
    .fetch_one(pool)
    .await
    .context("counting the recent enrolments of a token")
}

/// Note un dépassement de débit, une fois par jeton et par heure : c'est ce
/// qui déclenche l'alerte « enrolment burst ».
async fn note_burst(pool: &SqlitePool, token_id: i64, now: DateTime<Utc>) -> Result<()> {
    let since = (now - TimeDelta::hours(1)).timestamp_millis();
    let already: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM agent_enrolment_events
         WHERE token_id = ? AND kind = 'burst' AND at_ms >= ? LIMIT 1",
    )
    .bind(token_id)
    .bind(since)
    .fetch_optional(pool)
    .await
    .context("looking for a recent enrolment burst")?;
    if already.is_none() {
        warn!(jeton = token_id, "rafale d'enrôlements : débit du jeton dépassé");
        record_event(pool, Some(token_id), "", EventKind::Burst, now).await?;
    }
    Ok(())
}

/// Range (ou rouvre) la demande d'une machine qui attend sa validation.
async fn upsert_pending(
    pool: &SqlitePool,
    key: &str,
    identity: &AgentIdentity,
    source_ip: Option<String>,
    token_id: i64,
    now: DateTime<Utc>,
) -> Result<()> {
    let at = stamp(now);
    sqlx::query(
        "INSERT INTO agent_enrolment_requests
             (agent_key, hostname, os, source_ip, token_id, status, requested_at, last_seen_at)
         VALUES (?, ?, ?, ?, ?, 'pending', ?, ?)
         ON CONFLICT(agent_key) DO UPDATE SET
             hostname = excluded.hostname, os = excluded.os, source_ip = excluded.source_ip,
             token_id = excluded.token_id, status = 'pending',
             requested_at = excluded.requested_at, last_seen_at = excluded.last_seen_at,
             decided_at = NULL, decided_by = NULL",
    )
    .bind(key)
    .bind(bounded(&identity.hostname, MAX_HOSTNAME_LEN))
    .bind(bounded(&identity.os, 64))
    .bind(source_ip)
    .bind(token_id)
    .bind(&at)
    .bind(&at)
    .execute(pool)
    .await
    .context("recording the pending enrolment")?;
    Ok(())
}

/// Une machine en attente qui réessaie : on garde trace de sa dernière
/// présentation, pour l'interface et pour la purge des demandes oubliées.
async fn touch_pending(
    pool: &SqlitePool,
    id: i64,
    identity: &AgentIdentity,
    source_ip: Option<String>,
    token_id: i64,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "UPDATE agent_enrolment_requests
         SET hostname = ?, os = ?, source_ip = COALESCE(?, source_ip), token_id = ?,
             last_seen_at = ?
         WHERE id = ?",
    )
    .bind(bounded(&identity.hostname, MAX_HOSTNAME_LEN))
    .bind(bounded(&identity.os, 64))
    .bind(source_ip)
    .bind(token_id)
    .bind(stamp(now))
    .bind(id)
    .execute(pool)
    .await
    .context("refreshing the pending enrolment")?;
    Ok(())
}

/// Garde d'une machine inconnue, appelée juste avant sa création.
///
/// `Ok(Ok(request))` : elle peut entrer (`request` : sa demande acceptée, à
/// effacer une fois la machine créée). `Ok(Err(refus))` : elle n'entre pas, et
/// rien n'a été créé pour elle.
pub async fn admit(
    pool: &SqlitePool,
    token_id: i64,
    identity: &AgentIdentity,
    key: &str,
    origin: &EnrolOrigin,
    now: DateTime<Utc>,
) -> Result<Result<Option<i64>, Refusal>> {
    let source_ip = match origin {
        EnrolOrigin::Internal => return Ok(Ok(None)),
        EnrolOrigin::Network { source_ip } => source_ip.map(|ip| ip.to_string()),
    };
    let settings = load_settings(pool).await?;
    let Some(token) = token_info(pool, token_id).await? else {
        return Ok(Err(Refusal::Dormant));
    };
    let fleet = is_fleet(token.max_uses);

    // Veille : posée par la tâche de fond, ou constatée ici si elle n'est pas
    // encore passée.
    if token.dormant {
        return Ok(Err(Refusal::Dormant));
    }
    if fleet && token.activity.is_some_and(|at| dormant_due(at, settings.dormant_days, now)) {
        put_to_sleep(pool, token_id, now).await?;
        return Ok(Err(Refusal::Dormant));
    }

    if let Some(request) = request_by_key(pool, key).await? {
        match request.status {
            RequestStatus::Approved => return Ok(Ok(Some(request.id))),
            RequestStatus::Pending => {
                touch_pending(pool, request.id, identity, source_ip, token_id, now).await?;
                return Ok(Err(Refusal::Pending));
            }
            RequestStatus::Rejected
                if rejection_holds(request.decided_at, request.token_id, token_id, now) =>
            {
                return Ok(Err(Refusal::Rejected));
            }
            // Délai passé, ou autre jeton : une nouvelle demande.
            RequestStatus::Rejected => {}
        }
    }

    let recent = recent_enrolments(pool, token_id, now).await?;
    if over_rate(recent, settings.rate_per_hour) {
        note_burst(pool, token_id, now).await?;
        return Ok(Err(Refusal::RateLimited));
    }

    match verdict(settings.mode_for(fleet), token.ad_domain.as_deref(), &identity.hostname) {
        Verdict::Accept => Ok(Ok(None)),
        Verdict::Ask => {
            if pending_count(pool).await? >= MAX_PENDING {
                warn!(hote = identity.hostname, "enrôlement refusé : trop de machines en attente");
                return Ok(Err(Refusal::TooManyPending));
            }
            upsert_pending(pool, key, identity, source_ip, token_id, now).await?;
            record_event(pool, Some(token_id), &identity.hostname, EventKind::Pending, now)
                .await?;
            info!(hote = identity.hostname, jeton = token_id, "enrôlement en attente");
            Ok(Err(Refusal::Pending))
        }
    }
}

/// Note un enrôlement servi : il compte pour le débit et l'alerte, remet à zéro
/// l'inactivité du jeton, et efface la demande de la machine s'il y en avait une.
pub async fn note_enrolled(
    pool: &SqlitePool,
    token_id: i64,
    key: &str,
    hostname: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    record_event(pool, Some(token_id), hostname, EventKind::Enrolled, now).await?;
    sqlx::query("UPDATE agent_tokens SET enrol_activity_at = ? WHERE id = ?")
        .bind(stamp(now))
        .bind(token_id)
        .execute(pool)
        .await
        .context("recording the token activity")?;
    sqlx::query("DELETE FROM agent_enrolment_requests WHERE agent_key = ?")
        .bind(key)
        .execute(pool)
        .await
        .context("closing the enrolment request")?;
    Ok(())
}

/// Une machine en attente, telle que l'interface la montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingRequest {
    pub id: i64,
    pub hostname: String,
    pub os: String,
    /// Adresse d'où la demande est venue (la dernière fois).
    pub source_ip: Option<String>,
    pub token_id: Option<i64>,
    /// Identifiant (nom) du jeton présenté, jamais le secret.
    pub token_name: Option<String>,
    pub requested_at: String,
    pub last_seen_at: String,
}

/// Machines en attente, la plus récente d'abord.
pub async fn list_pending(pool: &SqlitePool) -> Result<Vec<PendingRequest>> {
    let rows = sqlx::query(
        "SELECT r.id, r.hostname, r.os, r.source_ip, r.token_id, t.name AS token_name,
                r.requested_at, r.last_seen_at
         FROM agent_enrolment_requests r LEFT JOIN agent_tokens t ON t.id = r.token_id
         WHERE r.status = 'pending'
         ORDER BY r.requested_at DESC, r.id DESC
         LIMIT ?",
    )
    .bind(MAX_PENDING)
    .fetch_all(pool)
    .await
    .context("listing pending enrolments")?;
    rows.iter()
        .map(|row| {
            Ok(PendingRequest {
                id: row.try_get("id")?,
                hostname: row.try_get("hostname")?,
                os: row.try_get("os")?,
                source_ip: row.try_get("source_ip")?,
                token_id: row.try_get("token_id")?,
                token_name: row.try_get("token_name")?,
                requested_at: row.try_get("requested_at")?,
                last_seen_at: row.try_get("last_seen_at")?,
            })
        })
        .collect()
}

/// Accepte ou refuse des machines en attente. Renvoie le nombre de demandes
/// tranchées (une demande déjà tranchée ou inconnue est ignorée).
pub async fn decide(
    pool: &SqlitePool,
    ids: &[i64],
    approve: bool,
    by: &str,
    now: DateTime<Utc>,
) -> Result<u64> {
    let status = if approve { "approved" } else { "rejected" };
    let at = stamp(now);
    let mut tx = pool.begin().await.context("opening the decision transaction")?;
    let mut decided = 0;
    for &id in ids {
        let done = sqlx::query(
            "UPDATE agent_enrolment_requests SET status = ?, decided_at = ?, decided_by = ?
             WHERE id = ? AND status = 'pending'",
        )
        .bind(status)
        .bind(&at)
        .bind(by)
        .bind(id)
        .execute(&mut *tx)
        .await
        .context("recording the enrolment decision")?;
        decided += done.rows_affected();
    }
    tx.commit().await.context("saving the enrolment decisions")?;
    Ok(decided)
}

/// Purge : événements anciens, refus échus (la machine pourra redemander),
/// demandes que leur machine ne renouvelle plus, acceptations jamais servies.
pub async fn purge(pool: &SqlitePool, now: DateTime<Utc>) -> Result<()> {
    let events = (now - TimeDelta::hours(EVENT_RETENTION_HOURS)).timestamp_millis();
    sqlx::query("DELETE FROM agent_enrolment_events WHERE at_ms < ? AND notified = 1")
        .bind(events)
        .execute(pool)
        .await
        .context("purging enrolment events")?;
    sqlx::query(
        "DELETE FROM agent_enrolment_requests WHERE status = 'rejected' AND decided_at < ?",
    )
    .bind(stamp(now - TimeDelta::hours(REJECT_HOLD_HOURS)))
    .execute(pool)
    .await
    .context("purging expired rejections")?;
    let stale = stamp(now - TimeDelta::days(STALE_REQUEST_DAYS));
    sqlx::query(
        "DELETE FROM agent_enrolment_requests
         WHERE (status = 'pending' AND last_seen_at < ?)
            OR (status = 'approved' AND decided_at < ?)",
    )
    .bind(&stale)
    .bind(&stale)
    .execute(pool)
    .await
    .context("purging stale enrolment requests")?;
    Ok(())
}

/// Événement à annoncer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub hostname: String,
    /// Identifiant (nom) du jeton, s'il existe encore.
    pub token_name: Option<String>,
}

/// Une notification prête à partir.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub title: String,
    pub text: String,
    pub severity: Severity,
    /// Clé stable : un canal d'astreinte ou de tickets s'en servirait pour
    /// rapprocher deux envois.
    pub fingerprint: String,
}

/// Liste lisible des premiers noms, puis « and N more ».
fn names_line(names: &[&str]) -> String {
    let shown: Vec<&str> = names.iter().copied().take(NOTICE_NAMES).collect();
    let mut line = shown.join(", ");
    if names.len() > shown.len() {
        line.push_str(&format!(", and {} more", names.len() - shown.len()));
    }
    line
}

/// Met en forme les événements d'une minute : un message pour tous les
/// enrôlements (acceptés et en attente), un par rafale, un par mise en veille.
pub fn compose(events: &[Event], dormant_days: u32) -> Vec<Notice> {
    let mut notices = Vec::new();
    let pending: Vec<&str> = events
        .iter()
        .filter(|event| event.kind == EventKind::Pending)
        .map(|event| event.hostname.as_str())
        .collect();
    let enrolled: Vec<&str> = events
        .iter()
        .filter(|event| event.kind == EventKind::Enrolled)
        .map(|event| event.hostname.as_str())
        .collect();
    let total = pending.len() + enrolled.len();
    if total > 0 {
        // Les machines en attente d'abord : ce sont elles qui demandent un geste.
        let names: Vec<&str> = pending.iter().chain(enrolled.iter()).copied().collect();
        let plural = if total == 1 { "" } else { "s" };
        let title = if pending.is_empty() {
            format!("DumbMonit — {total} new agent enrolment{plural}")
        } else {
            format!(
                "DumbMonit — {total} new agent enrolment{plural} ({} waiting for approval)",
                pending.len()
            )
        };
        let mut text = format!("{total} machine{plural}: {}.", names_line(&names));
        if !enrolled.is_empty() {
            text.push_str(&format!("\nAccepted: {}.", enrolled.len()));
        }
        if !pending.is_empty() {
            text.push_str(&format!(
                "\nWaiting for approval: {}. Accept or reject them in Settings > Agents.",
                pending.len()
            ));
        }
        text.push_str("\nIf you did not install these agents, revoke the enrolment token.");
        notices.push(Notice {
            title,
            text,
            severity: if pending.is_empty() { Severity::Info } else { Severity::Warning },
            fingerprint: "dumbmonit-agent-enrolment".to_string(),
        });
    }
    let mut bursts: Vec<&str> = Vec::new();
    let mut sleeping: Vec<&str> = Vec::new();
    for event in events {
        let name = event.token_name.as_deref().unwrap_or("(deleted token)");
        match event.kind {
            EventKind::Burst if !bursts.contains(&name) => bursts.push(name),
            EventKind::Dormant if !sleeping.contains(&name) => sleeping.push(name),
            _ => {}
        }
    }
    for name in bursts {
        notices.push(Notice {
            title: format!("DumbMonit — enrolment burst on token {name}"),
            text: format!(
                "More machines tried to enrol with token {name} than its hourly limit allows; \
                 further attempts are refused (HTTP 429) until the rate drops. If you did not \
                 start a deployment, revoke the token in Settings > Agents."
            ),
            severity: Severity::Critical,
            fingerprint: format!("dumbmonit-enrolment-burst-{name}"),
        });
    }
    for name in sleeping {
        notices.push(Notice {
            title: format!("DumbMonit — enrolment token {name} put to sleep"),
            text: format!(
                "Fleet token {name} enrolled no machine for {dormant_days} days: it no longer \
                 enrols new ones. Machines already enrolled keep reporting. Reactivate it in \
                 Settings > Agents if you still need it."
            ),
            severity: Severity::Info,
            fingerprint: format!("dumbmonit-enrolment-dormant-{name}"),
        });
    }
    notices
}

/// Événements pas encore annoncés, marqués annoncés dans la foulée : un canal
/// en panne ne doit pas faire repartir le même message à chaque minute.
pub async fn take_unnotified(pool: &SqlitePool) -> Result<Vec<Event>> {
    let mut tx = pool.begin().await.context("opening the notice transaction")?;
    let rows = sqlx::query(
        "SELECT e.id, e.kind, e.hostname, t.name AS token_name
         FROM agent_enrolment_events e LEFT JOIN agent_tokens t ON t.id = e.token_id
         WHERE e.notified = 0 ORDER BY e.id",
    )
    .fetch_all(&mut *tx)
    .await
    .context("reading enrolment events")?;
    let mut events = Vec::new();
    let mut last = None;
    for row in &rows {
        last = Some(row.try_get::<i64, _>("id")?);
        let Some(kind) = EventKind::parse(&row.try_get::<String, _>("kind")?) else { continue };
        events.push(Event {
            kind,
            hostname: row.try_get("hostname")?,
            token_name: row.try_get("token_name")?,
        });
    }
    if let Some(last) = last {
        sqlx::query("UPDATE agent_enrolment_events SET notified = 1 WHERE notified = 0 AND id <= ?")
            .bind(last)
            .execute(&mut *tx)
            .await
            .context("marking enrolment events as notified")?;
    }
    tx.commit().await.context("saving the notice state")?;
    Ok(events)
}

/// Fabrique le message d'une notification, au format des canaux.
fn message(
    notice: &Notice,
    public_url: Option<&str>,
    now: DateTime<Utc>,
) -> crate::notify::Message {
    let link = public_url.map(|base| format!("{base}/settings/monitoring#agents"));
    let mut text = notice.text.clone();
    let mut markdown = format!("**{}**\n{}", notice.title, notice.text);
    if let Some(link) = &link {
        text.push_str(&format!("\n{link}"));
        markdown.push_str(&format!("\n[Settings > Agents]({link})"));
    }
    crate::notify::Message {
        title: notice.title.clone(),
        text,
        markdown,
        severity: notice.severity,
        resolved: false,
        target_name: "DumbMonit".to_string(),
        target_id: None,
        at: now,
        rule_name: "Agent enrolment".to_string(),
        fingerprint: notice.fingerprint.clone(),
        value: None,
        threshold: None,
        unit: String::new(),
        operator: String::new(),
        count: 1,
        link,
        actions: Vec::new(),
    }
}

/// Envoie les notifications sur les canaux actifs de l'instance (pas ceux
/// d'un client en mode MSP, ni les canaux d'astreinte ou de tickets, qui
/// attendent des alertes). Renvoie le nombre d'envois réussis.
async fn deliver(state: &AppState, notices: &[Notice], http: &reqwest::Client) -> Result<usize> {
    use crate::auth::clients;
    let pool = &state.pool;
    let channels = crate::db::alerts::list_channels(pool, &state.cipher).await?;
    let owners = clients::owners(pool, clients::Kind::Channel).await?;
    let global = crate::notify::policy_store::load_global(pool).await?;
    let env_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
    let public_url = global.public_url(env_url.as_deref());
    let now = Utc::now();
    let store = crate::notify::webpush::Store { pool, cipher: &state.cipher };
    let mut sent = 0;
    for notice in notices {
        let message = message(notice, public_url.as_deref(), now);
        for config in channels.iter().filter(|config| {
            config.enabled
                && config.kind != crate::oncall::CHANNEL_KIND
                && !crate::notify::tickets::is_ticket_kind(&config.kind)
                && owners.get(&config.id).is_none_or(|client| *client == clients::DEFAULT_CLIENT)
        }) {
            let report = crate::notify::deliver_stored(store, http, config, &message).await;
            if report.is_success() {
                sent += 1;
            }
            if let Err(error) = crate::db::alerts::record_delivery(pool, &report, now).await {
                warn!(?error, "enrolment notice delivery not recorded");
            }
        }
    }
    Ok(sent)
}

/// Un passage de la tâche de fond : annonce la minute écoulée.
pub async fn notify_once(state: &AppState, http: &reqwest::Client) -> Result<usize> {
    let events = take_unnotified(&state.pool).await?;
    if events.is_empty() {
        return Ok(0);
    }
    let settings = load_settings(&state.pool).await?;
    let notices = compose(&events, settings.dormant_days);
    deliver(state, &notices, http).await
}

/// Tâche de fond : chaque minute le message groupé, chaque heure la mise en
/// veille des jetons inactifs et la purge.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let http = crate::notify::http_client();
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut minutes: u64 = 0;
        loop {
            ticker.tick().await;
            if minutes.is_multiple_of(60) {
                let now = Utc::now();
                match load_settings(&state.pool).await {
                    Ok(settings) => {
                        if let Err(error) =
                            sweep_dormant(&state.pool, settings.dormant_days, now).await
                        {
                            warn!(error = %format!("{error:#}"), "idle token sweep failed");
                        }
                    }
                    Err(error) => warn!(error = %format!("{error:#}"), "enrolment settings unread"),
                }
                if let Err(error) = purge(&state.pool, now).await {
                    warn!(error = %format!("{error:#}"), "enrolment purge failed");
                }
            }
            minutes = minutes.wrapping_add(1);
            if let Err(error) = notify_once(&state, &http).await {
                warn!(error = %format!("{error:#}"), "enrolment notice not sent");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn at(raw: &str) -> DateTime<Utc> {
        parse_stamp(raw).expect("date")
    }

    #[test]
    fn a_new_install_asks_for_fleet_tokens_and_trusts_single_use_ones() {
        let settings = EnrolmentSettings::default();
        assert_eq!(settings.mode_for(true), ApprovalMode::Manual);
        assert_eq!(settings.mode_for(false), ApprovalMode::Auto);
        assert_eq!(settings.rate_per_hour, 60);
        assert_eq!(settings.dormant_days, 30);
        assert!(is_fleet(None));
        assert!(is_fleet(Some(5)));
        assert!(!is_fleet(Some(1)));
    }

    #[test]
    fn the_three_modes_decide_as_documented() {
        assert_eq!(verdict(ApprovalMode::Auto, None, "srv01"), Verdict::Accept);
        let manual = verdict(ApprovalMode::Manual, Some("corp.example.com"), "a.corp.example.com");
        assert_eq!(manual, Verdict::Ask);
        let ad = |host: &str, domain: Option<&str>| verdict(ApprovalMode::AdDomain, domain, host);
        assert_eq!(ad("srv01.corp.example.com", Some("corp.example.com")), Verdict::Accept);
        assert_eq!(ad("SRV02.Paris.Corp.Example.com.", Some("corp.example.com")), Verdict::Accept);
        assert_eq!(ad("srv01", Some("corp.example.com")), Verdict::Ask);
        assert_eq!(ad("srv01.evilcorp.example.com", Some("corp.example.com")), Verdict::Ask);
        // Un jeton sans domaine ne prouve rien : la machine attend.
        assert_eq!(ad("srv01.corp.example.com", None), Verdict::Ask);
    }

    #[test]
    fn dormancy_rate_and_rejection_follow_their_settings() {
        let now = at("2026-11-02T12:00:00Z");
        assert!(dormant_due(at("2026-10-03T12:00:00Z"), 30, now));
        assert!(!dormant_due(at("2026-10-03T12:00:01Z"), 30, now));
        assert!(!dormant_due(at("2020-01-01T00:00:00Z"), 0, now), "0 : jamais");

        assert!(!over_rate(59, 60));
        assert!(over_rate(60, 60));
        assert!(!over_rate(1_000_000, 0), "0 : sans limite");

        let decided = Some(at("2026-11-02T00:00:00Z"));
        assert!(rejection_holds(decided, Some(7), 7, now));
        assert!(!rejection_holds(decided, Some(7), 8, now), "un autre jeton est une autre action");
        assert!(!rejection_holds(decided, Some(7), 7, at("2026-11-03T00:00:00Z")));
    }

    #[test]
    fn settings_are_bounded_and_older_documents_still_read() {
        assert!(EnrolmentSettings::default().validate().is_ok());
        let too_fast =
            EnrolmentSettings { rate_per_hour: MAX_RATE_PER_HOUR + 1, ..Default::default() };
        assert!(too_fast.validate().is_err());
        let too_long =
            EnrolmentSettings { dormant_days: MAX_DORMANT_DAYS + 1, ..Default::default() };
        assert!(too_long.validate().is_err());
        // Un champ absent prend sa valeur par défaut.
        let partial: EnrolmentSettings =
            serde_json::from_str(r#"{"fleet_approval":"ad_domain"}"#).unwrap();
        assert_eq!(partial.fleet_approval, ApprovalMode::AdDomain);
        assert_eq!(partial.single_use_approval, ApprovalMode::Auto);
        assert_eq!(partial.dormant_days, DEFAULT_DORMANT_DAYS);
    }

    #[test]
    fn one_grouped_notice_names_the_first_five_machines() {
        let event = |kind, host: &str| Event {
            kind,
            hostname: host.to_string(),
            token_name: Some("tok_abcdefghij".to_string()),
        };
        let mut events: Vec<Event> =
            (1..=6).map(|n| event(EventKind::Enrolled, &format!("srv0{n}"))).collect();
        events.push(event(EventKind::Pending, "intrus"));
        let notices = compose(&events, 30);
        assert_eq!(notices.len(), 1, "un seul message pour la minute");
        let notice = &notices[0];
        assert!(notice.title.contains("7 new agent enrolments"), "{}", notice.title);
        assert!(notice.title.contains("1 waiting for approval"), "{}", notice.title);
        let expected = "7 machines: intrus, srv01, srv02, srv03, srv04, and 2 more.";
        assert!(notice.text.starts_with(expected), "{}", notice.text);
        assert_eq!(notice.severity, Severity::Warning);

        let quiet = compose(&[event(EventKind::Enrolled, "srv01")], 30);
        assert_eq!(quiet[0].severity, Severity::Info);
        assert!(quiet[0].title.ends_with("1 new agent enrolment"));
        assert!(compose(&[], 30).is_empty());
    }

    #[test]
    fn bursts_and_sleeps_get_their_own_single_notice_per_token() {
        let event = |kind, token: &str| Event {
            kind,
            hostname: String::new(),
            token_name: Some(token.to_string()),
        };
        let events = vec![
            event(EventKind::Burst, "tok_aaaaaaaaaa"),
            event(EventKind::Burst, "tok_aaaaaaaaaa"),
            event(EventKind::Dormant, "tok_bbbbbbbbbb"),
        ];
        let notices = compose(&events, 45);
        assert_eq!(notices.len(), 2);
        assert!(notices[0].title.contains("enrolment burst on token tok_aaaaaaaaaa"));
        assert_eq!(notices[0].severity, Severity::Critical);
        assert!(notices[1].title.contains("tok_bbbbbbbbbb put to sleep"));
        assert!(notices[1].text.contains("45 days"));
    }

    // --- Avec une base -----------------------------------------------------

    struct TestDb {
        pool: SqlitePool,
        _dir: tempfile::TempDir,
    }

    async fn setup() -> TestDb {
        let dir = tempfile::tempdir().expect("répertoire temporaire");
        let pool = crate::db::open(&dir.path().join("test.db")).await.expect("base");
        TestDb { pool, _dir: dir }
    }

    /// Jeton inséré directement : la garde ne lit que ses colonnes.
    async fn token(db: &TestDb, max_uses: Option<i64>, ad_domain: Option<&str>) -> i64 {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tokens")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        sqlx::query_scalar(
            "INSERT INTO agent_tokens (name, token_hash, prefix, max_uses, ad_domain)
             VALUES (?, ?, 'dmon_test', ?, ?) RETURNING id",
        )
        .bind(format!("tok_test{n:06}"))
        .bind(format!("hash-{n}"))
        .bind(max_uses)
        .bind(ad_domain)
        .fetch_one(&db.pool)
        .await
        .expect("jeton")
    }

    fn identity(hostname: &str) -> AgentIdentity {
        AgentIdentity {
            hostname: hostname.to_string(),
            os: "linux".into(),
            os_version: None,
            kernel_version: None,
            arch: None,
            agent_version: "0.1.0".into(),
            commands_enabled: None,
            relay: false,
            site: None,
            relay_control: false,
            relay_local: false,
            machine_id: Some(format!("id-{hostname}")),
            tags: BTreeMap::new(),
            binding_supported: true,
            self_update: None,
            relay_gateway: None,
            remediation: None,
        }
    }

    fn net() -> EnrolOrigin {
        EnrolOrigin::network(Some("192.0.2.10".parse().unwrap()))
    }

    async fn admit_now(db: &TestDb, token_id: i64, host: &str) -> Result<Option<i64>, Refusal> {
        let id = identity(host);
        admit(&db.pool, token_id, &id, &id.key(), &net(), Utc::now()).await.expect("garde")
    }

    #[tokio::test]
    async fn a_fleet_token_waits_for_approval_and_nothing_is_created_meanwhile() {
        let db = setup().await;
        let fleet = token(&db, None, None).await;

        assert_eq!(admit_now(&db, fleet, "srv01").await, Err(Refusal::Pending));
        // L'agent réessaie : toujours en attente, et toujours une seule ligne.
        assert_eq!(admit_now(&db, fleet, "srv01").await, Err(Refusal::Pending));
        let pending = list_pending(&db.pool).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].hostname, "srv01");
        assert_eq!(pending[0].source_ip.as_deref(), Some("192.0.2.10"));
        assert_eq!(pending[0].token_id, Some(fleet));
        assert!(pending[0].token_name.as_deref().is_some_and(|n| n.starts_with("tok_")));
        let targets: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM targets").fetch_one(&db.pool).await.unwrap();
        assert_eq!(targets, 0, "aucun équipement tant qu'elle attend");

        assert_eq!(decide(&db.pool, &[pending[0].id], true, "admin", Utc::now()).await.unwrap(), 1);
        let request = admit_now(&db, fleet, "srv01").await.expect("acceptée");
        assert_eq!(request, Some(pending[0].id));
        assert!(list_pending(&db.pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_single_use_token_is_trusted_by_default_but_not_in_manual_mode() {
        let db = setup().await;
        let single = token(&db, Some(1), None).await;
        assert_eq!(admit_now(&db, single, "laptop").await, Ok(None));

        let strict =
            EnrolmentSettings { single_use_approval: ApprovalMode::Manual, ..Default::default() };
        save_settings(&db.pool, &strict).await.unwrap();
        assert_eq!(admit_now(&db, single, "laptop2").await, Err(Refusal::Pending));

        let auto = EnrolmentSettings { fleet_approval: ApprovalMode::Auto, ..Default::default() };
        save_settings(&db.pool, &auto).await.unwrap();
        let fleet = token(&db, None, None).await;
        assert_eq!(admit_now(&db, fleet, "srv09").await, Ok(None));
        // L'origine interne (démonstration) ne passe pas par la garde.
        let id = identity("demo");
        let (key, origin) = (id.key(), EnrolOrigin::Internal);
        let internal = admit(&db.pool, fleet, &id, &key, &origin, Utc::now()).await.unwrap();
        assert_eq!(internal, Ok(None));
    }

    #[tokio::test]
    async fn the_ad_domain_mode_trusts_hosts_of_the_token_domain_only() {
        let db = setup().await;
        let settings =
            EnrolmentSettings { fleet_approval: ApprovalMode::AdDomain, ..Default::default() };
        save_settings(&db.pool, &settings).await.unwrap();
        let ad = token(&db, None, Some("corp.example.com")).await;
        let plain = token(&db, None, None).await;

        assert_eq!(admit_now(&db, ad, "srv01.corp.example.com").await, Ok(None));
        assert_eq!(admit_now(&db, plain, "srv02.corp.example.com").await, Err(Refusal::Pending));
    }

    #[tokio::test]
    async fn a_rejected_machine_stays_hidden_for_a_day_unless_another_token_is_used() {
        let db = setup().await;
        let fleet = token(&db, None, None).await;
        assert_eq!(admit_now(&db, fleet, "intrus").await, Err(Refusal::Pending));
        let id = list_pending(&db.pool).await.unwrap()[0].id;
        assert_eq!(decide(&db.pool, &[id], false, "admin", Utc::now()).await.unwrap(), 1);
        // Déjà tranchée : une seconde décision ne compte pas.
        assert_eq!(decide(&db.pool, &[id], true, "admin", Utc::now()).await.unwrap(), 0);

        assert_eq!(admit_now(&db, fleet, "intrus").await, Err(Refusal::Rejected));
        assert!(list_pending(&db.pool).await.unwrap().is_empty(), "pas ré-affichée");

        let other = token(&db, None, None).await;
        assert_eq!(admit_now(&db, other, "intrus").await, Err(Refusal::Pending));

        // Le refus échu, la machine peut redemander avec le premier jeton.
        let db = setup().await;
        let fleet = token(&db, None, None).await;
        assert_eq!(admit_now(&db, fleet, "intrus").await, Err(Refusal::Pending));
        let id = list_pending(&db.pool).await.unwrap()[0].id;
        let yesterday = Utc::now() - TimeDelta::hours(25);
        decide(&db.pool, &[id], false, "admin", yesterday).await.unwrap();
        assert_eq!(admit_now(&db, fleet, "intrus").await, Err(Refusal::Pending));
    }

    #[tokio::test]
    async fn no_more_than_two_hundred_machines_wait() {
        let db = setup().await;
        let fleet = token(&db, None, None).await;
        let unlimited = EnrolmentSettings { rate_per_hour: 0, ..Default::default() };
        save_settings(&db.pool, &unlimited).await.unwrap();
        for n in 0..MAX_PENDING {
            assert_eq!(admit_now(&db, fleet, &format!("m{n}")).await, Err(Refusal::Pending));
        }
        assert_eq!(pending_count(&db.pool).await.unwrap(), MAX_PENDING);
        assert_eq!(admit_now(&db, fleet, "one-more").await, Err(Refusal::TooManyPending));
        // Une machine déjà en attente qui réessaie n'est pas une nouvelle demande.
        assert_eq!(admit_now(&db, fleet, "m0").await, Err(Refusal::Pending));
    }

    #[tokio::test]
    async fn a_token_over_its_hourly_rate_is_refused_and_raises_one_burst() {
        let db = setup().await;
        let settings = EnrolmentSettings {
            fleet_approval: ApprovalMode::Auto,
            rate_per_hour: 3,
            ..Default::default()
        };
        save_settings(&db.pool, &settings).await.unwrap();
        let fleet = token(&db, None, None).await;
        let now = Utc::now();
        for host in ["a", "b", "c"] {
            assert_eq!(admit_now(&db, fleet, host).await, Ok(None));
            note_enrolled(&db.pool, fleet, &identity(host).key(), host, now).await.unwrap();
        }
        assert_eq!(admit_now(&db, fleet, "d").await, Err(Refusal::RateLimited));
        assert_eq!(admit_now(&db, fleet, "e").await, Err(Refusal::RateLimited));
        // Un autre jeton n'est pas concerné.
        let other = token(&db, None, None).await;
        assert_eq!(admit_now(&db, other, "f").await, Ok(None));

        let events = take_unnotified(&db.pool).await.unwrap();
        let bursts = events.iter().filter(|e| e.kind == EventKind::Burst).count();
        assert_eq!(bursts, 1, "une seule alerte de rafale par heure");
        assert_eq!(events.iter().filter(|e| e.kind == EventKind::Enrolled).count(), 3);
        assert!(take_unnotified(&db.pool).await.unwrap().is_empty(), "annoncés une fois");
    }

    #[tokio::test]
    async fn an_idle_fleet_token_falls_asleep_once_and_can_be_reactivated() {
        let db = setup().await;
        let auto = EnrolmentSettings { fleet_approval: ApprovalMode::Auto, ..Default::default() };
        save_settings(&db.pool, &auto).await.unwrap();
        let fleet = token(&db, None, None).await;
        let single = token(&db, Some(1), None).await;
        let old = stamp(Utc::now() - TimeDelta::days(31));
        sqlx::query("UPDATE agent_tokens SET enrol_activity_at = ?")
            .bind(&old)
            .execute(&db.pool)
            .await
            .unwrap();

        let now = Utc::now();
        assert_eq!(sweep_dormant(&db.pool, 30, now).await.unwrap(), vec![fleet]);
        assert!(sweep_dormant(&db.pool, 30, now).await.unwrap().is_empty(), "une seule fois");
        assert!(sweep_dormant(&db.pool, 0, now).await.unwrap().is_empty());
        assert_eq!(admit_now(&db, fleet, "late").await, Err(Refusal::Dormant));
        // Un jeton à usage unique ne s'endort pas.
        assert_eq!(admit_now(&db, single, "laptop").await, Ok(None));
        let dormant = dormant_tokens(&db.pool).await.unwrap();
        assert!(dormant.contains_key(&fleet) && !dormant.contains_key(&single));

        let events = take_unnotified(&db.pool).await.unwrap();
        assert_eq!(events.iter().filter(|e| e.kind == EventKind::Dormant).count(), 1);

        assert!(reactivate(&db.pool, fleet, now).await.unwrap());
        assert!(!reactivate(&db.pool, fleet, now).await.unwrap(), "déjà en service");
        assert_eq!(admit_now(&db, fleet, "late").await, Ok(None));
    }

    #[tokio::test]
    async fn the_guard_notices_an_idle_token_before_the_hourly_sweep() {
        let db = setup().await;
        let fleet = token(&db, None, None).await;
        sqlx::query("UPDATE agent_tokens SET enrol_activity_at = ? WHERE id = ?")
            .bind(stamp(Utc::now() - TimeDelta::days(40)))
            .bind(fleet)
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(admit_now(&db, fleet, "srv").await, Err(Refusal::Dormant));
        assert!(dormant_tokens(&db.pool).await.unwrap().contains_key(&fleet));
    }

    #[tokio::test]
    async fn purge_forgets_expired_rejections_and_old_events() {
        let db = setup().await;
        let fleet = token(&db, None, None).await;
        assert_eq!(admit_now(&db, fleet, "x").await, Err(Refusal::Pending));
        let id = list_pending(&db.pool).await.unwrap()[0].id;
        decide(&db.pool, &[id], false, "admin", Utc::now() - TimeDelta::hours(30)).await.unwrap();
        take_unnotified(&db.pool).await.unwrap();
        purge(&db.pool, Utc::now() + TimeDelta::hours(EVENT_RETENTION_HOURS + 1)).await.unwrap();
        let requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_enrolment_requests")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_enrolment_events")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!((requests, events), (0, 0));
    }
}
