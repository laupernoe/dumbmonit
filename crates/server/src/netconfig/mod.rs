//! Sauvegarde et comparaison des configurations réseau (type `netconfig`).
//!
//! Le relevé lui-même (SSH ou API, profils déclaratifs) vit dans
//! `dumbmonit_collectors::netconfig`, partagé avec l'agent relais. Ici :
//!
//! - **le rythme** : une tâche de fond relève chaque équipement toutes les
//!   `fetch_interval_hours` (24 h par défaut ; un échec est retenté au bout
//!   d'un quart d'heure). Le collecteur, interrogé à la période habituelle
//!   par le planificateur, ne fait que relire l'état en base : les règles
//!   d'alerte gardent leur point toutes les minutes sans qu'un équipement
//!   soit sollicité plus d'une fois par jour ;
//! - **les versions** : chaque relevé est réduit à sa forme comparable
//!   (lignes volatiles du profil retirées) puis comparé à la dernière
//!   version par une empreinte à clé ([`crate::crypto::Cipher::keyed_digest`]).
//!   Seul un texte différent fait une nouvelle version, compressée puis
//!   chiffrée avec la clé d'instance. Rétention : `keep_versions` versions au
//!   plus, `keep_days` jours ; la dernière reste toujours ;
//! - **la clé d'hôte SSH** : confiance au premier usage, empreinte mémorisée,
//!   connexion refusée avant authentification si elle change, alerte, et
//!   seul un administrateur accepte la nouvelle clé ;
//! - **le masquage** des secrets à l'affichage ([`mask`]), que seul un
//!   administrateur connecté peut lever, comme le téléchargement d'une
//!   version brute (journal d'audit dans les deux cas).
//!
//! # Métriques produites
//!
//! | Métrique | Sens |
//! |---|---|
//! | `netconfig_versions` | Versions gardées. |
//! | `netconfig_config_changed` | 1 si le dernier relevé réussi a produit une nouvelle version. |
//! | `netconfig_host_key_changed` | 1 tant qu'une clé d'hôte différente attend d'être acceptée. |
//! | `netconfig_last_success_age_seconds` | Âge du dernier relevé réussi. |

pub mod store;

use std::io::{Read, Write};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::Context as _;
use async_trait::async_trait;
use dumbmonit_collectors::netconfig::{self as fetcher, Capture, Failure, FetchOptions, text};
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target, TargetId};
use regex::Regex;
use sqlx::SqlitePool;
use tokio::sync::Semaphore;
use tokio::time::{Instant, MissedTickBehavior, interval};
use tracing::{debug, info, warn};

use crate::collectors::relay::{self, JobResult};
use crate::crypto::Cipher;
use crate::db;
use crate::state::AppState;
use crate::webchange::diff;

/// Type de cible, tel qu'enregistré dans `Target::kind`.
pub const KIND: &str = fetcher::KIND;

/// `uid` de la règle livrée « Network configuration changed ».
pub const RULE_CHANGED: &str = "netconfig_changed";
/// `uid` de la règle livrée « SSH host key changed ».
pub const RULE_HOST_KEY: &str = "netconfig_host_key_changed";

/// Libellés des sous-clés d'instance : empreinte de comparaison, étiquette
/// des secrets masqués.
const DIGEST_LABEL: &[u8] = b"dumbmonit/netconfig/digest/v1";
const MASK_LABEL: &[u8] = b"dumbmonit/netconfig/mask/v1";

/// Passage de la tâche de fond.
const TICK: Duration = Duration::from_secs(60);
/// Relevés simultanés au plus, toutes origines confondues.
const MAX_PARALLEL: usize = 4;
/// Un relevé en échec est retenté au bout de ce délai.
const RETRY_AFTER_SECS: i64 = 15 * 60;

/// Réglages du serveur lus dans les options de la cible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    pub interval_secs: i64,
    pub keep_versions: i64,
    pub keep_days: i64,
}

impl Default for Schedule {
    fn default() -> Self {
        Self { interval_secs: 24 * 3600, keep_versions: 100, keep_days: 365 }
    }
}

fn bounded(target: &Target, key: &str, min: i64, max: i64, default: i64) -> Result<i64, String> {
    match target.tags.get(key).map(|value| value.trim()) {
        None | Some("") => Ok(default),
        Some(raw) => match raw.parse::<i64>() {
            Ok(value) if (min..=max).contains(&value) => Ok(value),
            _ => Err(format!("\"{key}\" must be a whole number from {min} to {max}")),
        },
    }
}

impl Schedule {
    pub fn from_target(target: &Target) -> Result<Self, String> {
        let defaults = Self::default();
        Ok(Self {
            interval_secs: bounded(target, "fetch_interval_hours", 1, 168, 24)? * 3600,
            keep_versions: bounded(target, "keep_versions", 1, 1000, defaults.keep_versions)?,
            keep_days: bounded(target, "keep_days", 0, 3650, defaults.keep_days)?,
        })
    }
}

/// Le collecteur : relit l'état en base, ne contacte jamais l'équipement.
pub struct NetconfigCollector {
    pool: SqlitePool,
}

impl NetconfigCollector {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Collector for NetconfigCollector {
    fn kind(&self) -> &str {
        KIND
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        FetchOptions::from_target(target)?;
        Schedule::from_target(target).map_err(ProbeError::Config)?;
        let state = store::state(&self.pool, target.id).await.map_err(ProbeError::Other)?;
        let Some(state) = state else { return Ok(Vec::new()) };
        let now = chrono::Utc::now().timestamp_millis();
        let host_key_changed = state.host_key_seen.is_some();
        let mut samples = vec![Sample::new(
            "netconfig_host_key_changed",
            if host_key_changed { 1.0 } else { 0.0 },
            MetricKind::Gauge,
            now,
        )];
        // Une clé changée n'est pas une panne : l'alerte dédiée la porte, et la
        // série `up` continue.
        if !host_key_changed
            && let (Some(message), Some(kind)) =
                (state.last_error.as_deref(), state.last_error_kind.as_deref())
        {
            return Err(match kind {
                "down" => ProbeError::Unreachable(message.to_string()),
                "auth" => ProbeError::Auth(message.to_string()),
                _ => ProbeError::Config(message.to_string()),
            });
        }
        let versions =
            store::version_count(&self.pool, target.id).await.map_err(ProbeError::Other)?;
        if versions > 0 {
            let count = versions as f64;
            samples.push(Sample::new("netconfig_versions", count, MetricKind::Gauge, now));
            samples.push(Sample::new(
                "netconfig_config_changed",
                if state.last_changed { 1.0 } else { 0.0 },
                MetricKind::Gauge,
                now,
            ));
        }
        if let Some(success) = state.last_success_at.as_deref().and_then(parse_utc) {
            let age = (chrono::Utc::now() - success).num_seconds().max(0);
            samples.push(Sample::new(
                "netconfig_last_success_age_seconds",
                age as f64,
                MetricKind::Gauge,
                now,
            ));
        }
        Ok(samples)
    }
}

/// Horodatage SQLite (`YYYY-MM-DD HH:MM:SS`, UTC sans suffixe).
fn parse_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").ok().map(|t| t.and_utc())
}

fn permits() -> Arc<Semaphore> {
    static PERMITS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    PERMITS.get_or_init(|| Arc::new(Semaphore::new(MAX_PARALLEL))).clone()
}

/// Lance la tâche de fond des relevés.
pub fn spawn(state: AppState) {
    tokio::spawn(run(state));
}

async fn run(state: AppState) {
    let mut ticker = interval(TICK);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        if let Err(error) = tick(&state).await {
            warn!(?error, "configuration backups: cycle failed");
        }
    }
}

async fn tick(state: &AppState) -> anyhow::Result<()> {
    let targets = db::targets::list_enabled(&state.pool, &state.cipher).await?;
    for target in targets.into_iter().filter(|target| target.kind == KIND) {
        let Ok(schedule) = Schedule::from_target(&target) else { continue };
        let current = store::state(&state.pool, target.id).await?;
        let failed = current.as_ref().is_some_and(|s| s.last_error.is_some());
        let due = match store::attempt_age(&state.pool, target.id).await? {
            None => true,
            Some(age) => age >= schedule.interval_secs || (failed && age >= RETRY_AFTER_SECS),
        };
        if due && start(state.clone(), target).await? {
            debug!("configuration backup started");
        }
    }
    Ok(())
}

/// Lance un relevé en tâche de fond, sauf si un autre est en cours pour cet
/// équipement. Vrai s'il a été lancé.
pub async fn start(state: AppState, target: Target) -> anyhow::Result<bool> {
    if !store::try_lock(&state.pool, target.id).await? {
        return Ok(false);
    }
    tokio::spawn(async move {
        let Ok(_permit) = permits().acquire_owned().await else { return };
        let started = std::time::Instant::now();
        match run_fetch(&state, &target).await {
            Ok(changed) => info!(
                target = target.id,
                changed,
                elapsed = ?started.elapsed(),
                "configuration backup done"
            ),
            Err(error) => warn!(target = target.id, ?error, "configuration backup failed"),
        }
        if let Err(error) = store::unlock(&state.pool, target.id).await {
            warn!(target = target.id, ?error, "configuration backup lock not released");
        }
    });
    Ok(true)
}

/// Nature d'une erreur de relevé, comme `probe` la rendra.
fn classify(error: &ProbeError) -> &'static str {
    match error {
        ProbeError::Timeout(_) | ProbeError::Unreachable(_) => "down",
        ProbeError::Auth(_) => "auth",
        ProbeError::Protocol(_) | ProbeError::Config(_) | ProbeError::Other(_) => "config",
    }
}

/// La même chose à partir du texte d'une erreur rapportée par un relais.
fn classify_text(message: &str) -> &'static str {
    if message.starts_with("Timed out") || message.starts_with("Monitor unreachable") {
        "down"
    } else if message.starts_with("Authentication refused") {
        "auth"
    } else {
        "config"
    }
}

/// Le texte d'une erreur relayée sans le préfixe de sa nature, que
/// `ProbeError` remettra.
fn without_prefix(message: &str) -> &str {
    for prefix in [
        "Monitor unreachable: ",
        "Authentication refused: ",
        "Invalid response: ",
        "Invalid configuration: ",
    ] {
        if let Some(rest) = message.strip_prefix(prefix) {
            return rest;
        }
    }
    message
}

/// Un relevé complet ; vrai s'il a produit une nouvelle version.
async fn run_fetch(state: &AppState, target: &Target) -> anyhow::Result<bool> {
    let pool = &state.pool;
    let id = target.id;
    let pinned = store::state(pool, id).await?.and_then(|s| s.host_key);
    let outcome = match db::targets::relay_of(pool, id).await? {
        None => fetcher::fetch(target, pinned.as_deref()).await,
        Some(agent) => via_relay(state, target, agent, pinned.as_deref()).await,
    };
    let capture = match outcome {
        Ok(capture) => capture,
        Err(Failure::HostKeyMismatch { seen }) => {
            warn!(target = id, %seen, "SSH host key changed: configuration backup refused");
            store::host_key_mismatch(pool, id, &seen).await?;
            return Ok(false);
        }
        Err(Failure::Probe(error)) => {
            store::finish_failed(pool, id, classify(&error), &error.to_string()).await?;
            return Ok(false);
        }
    };
    record(state, target, capture).await
}

/// Relevé confié à l'agent relais de l'équipement.
async fn via_relay(
    state: &AppState,
    target: &Target,
    agent: TargetId,
    pinned: Option<&str>,
) -> Result<Capture, Failure> {
    let mut copy = target.clone();
    // L'empreinte mémorisée voyage avec la sonde ; une valeur posée sur la
    // cible elle-même n'est jamais prise en compte.
    match pinned {
        Some(pinned) => {
            copy.tags.insert(fetcher::RELAY_HOST_KEY_TAG.to_string(), pinned.to_string());
        }
        None => {
            copy.tags.remove(fetcher::RELAY_HOST_KEY_TAG);
        }
    }
    let timeout = FetchOptions::from_target(target)?
        .timeout
        .saturating_mul(2)
        .min(Duration::from_secs(fetcher::MAX_TIMEOUT_SECS));
    let receiver = state
        .relay()
        .enqueue(agent, copy, timeout, false, Instant::now() + relay::deadline_for(timeout))
        .map_err(|_| ProbeError::Unreachable("the relay agent is still busy".into()))?;
    let outcome = match receiver.await {
        Ok(JobResult::Done(outcome)) => outcome,
        Ok(JobResult::Expired) | Err(_) => {
            return Err(ProbeError::Unreachable(format!(
                "relay agent {agent} did not report the configuration backup"
            ))
            .into());
        }
    };
    let capture = outcome.netconfig.unwrap_or_default();
    if capture.host_key_mismatch {
        return Err(Failure::HostKeyMismatch { seen: capture.host_key.unwrap_or_default() });
    }
    if let Some(error) = outcome.error {
        let message = without_prefix(&error).to_string();
        return Err(match classify_text(&error) {
            "down" => ProbeError::Unreachable(message),
            "auth" => ProbeError::Auth(message),
            _ => ProbeError::Protocol(message),
        }
        .into());
    }
    let Some(config) = capture.config.filter(|config| !config.trim().is_empty()) else {
        return Err(ProbeError::Protocol(
            "the relay agent sent no configuration (is it up to date?)".into(),
        )
        .into());
    };
    if config.len() > fetcher::MAX_CONFIG_BYTES {
        return Err(ProbeError::Protocol("the configuration is too large".into()).into());
    }
    Ok(Capture { config, host_key: capture.host_key })
}

/// Compresse puis chiffre un texte de configuration.
pub fn seal(cipher: &Cipher, config: &str) -> anyhow::Result<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(config.as_bytes())?;
    let compressed = encoder.finish()?;
    cipher.encrypt(&compressed)
}

/// Déchiffre puis décompresse, borné à deux fois la taille admise.
pub fn open(cipher: &Cipher, sealed: &[u8]) -> anyhow::Result<String> {
    let compressed = cipher.decrypt(sealed)?;
    let limit = (fetcher::MAX_CONFIG_BYTES * 2) as u64;
    let mut out = String::new();
    flate2::read::GzDecoder::new(compressed.as_slice())
        .take(limit)
        .read_to_string(&mut out)
        .context("version illisible")?;
    Ok(out)
}

/// Empreinte de comparaison d'une forme comparable.
fn digest(cipher: &Cipher, normalized: &str) -> String {
    hex::encode(cipher.keyed_digest(DIGEST_LABEL, normalized.as_bytes()))
}

/// Empreinte de comparaison d'un texte relevé avec le profil `profile_id`
/// (restauration d'une sauvegarde, où l'empreinte d'origine ne vaut rien sous
/// une autre clé d'instance).
pub fn digest_of(cipher: &Cipher, profile_id: &str, config: &str) -> String {
    let volatile = fetcher::profile::find(profile_id)
        .map(|profile| profile.volatile_rules())
        .unwrap_or_default();
    digest(cipher, &text::normalize(config, &volatile))
}

/// Range un relevé : nouvelle version si la configuration a changé.
async fn record(state: &AppState, target: &Target, capture: Capture) -> anyhow::Result<bool> {
    let pool = &state.pool;
    let id = target.id;
    let options = FetchOptions::from_target(target)?;
    let schedule = Schedule::from_target(target).unwrap_or_default();
    let profile = options.profile;
    if let Some(host_key) = &capture.host_key {
        store::pin_if_absent(pool, id, host_key).await?;
    }
    let volatile = profile.volatile_rules();
    let normalized = text::normalize(&capture.config, &volatile);
    let digest = digest(&state.cipher, &normalized);
    let latest = store::latest(pool, id).await?;
    let changed = match &latest {
        Some(version) if version.digest == digest => {
            store::touch(pool, version.id).await?;
            false
        }
        _ => {
            let after: Vec<&str> = normalized.lines().collect();
            let (added, removed) = match &latest {
                Some(version) => {
                    let sealed = store::content(pool, id, version.id).await?.unwrap_or_default();
                    let before_text = open(&state.cipher, &sealed)
                        .map(|old| text::normalize(&old, &volatile))
                        .unwrap_or_default();
                    let before: Vec<&str> = before_text.lines().collect();
                    diff::counts(&diff::diff(&before, &after))
                }
                None => (after.len(), 0),
            };
            let sealed = seal(&state.cipher, &capture.config)?;
            store::insert(
                pool,
                id,
                &store::NewVersion {
                    profile: &profile.id,
                    digest: &digest,
                    size_bytes: capture.config.len() as i64,
                    lines: capture.config.lines().count() as i64,
                    added: added as i64,
                    removed: removed as i64,
                    content_enc: &sealed,
                },
            )
            .await?;
            let pruned = store::prune(pool, id, schedule.keep_versions, schedule.keep_days).await?;
            if pruned > 0 {
                debug!(target = id, pruned, "old configuration versions removed");
            }
            // La toute première version est une référence, pas un changement.
            latest.is_some()
        }
    };
    store::finish_ok(pool, id, changed).await?;
    Ok(changed)
}

/// Règles de secours, appliquées en plus de celles du profil : un mot de passe
/// qu'un profil aurait oublié reste masqué.
fn fallback_rules() -> &'static [Regex] {
    static RULES: OnceLock<Vec<Regex>> = OnceLock::new();
    RULES.get_or_init(|| {
        [
            concat!(
                r#"(?i)\b(?:password|passwd|secret|passphrase|psk)\b["']?\s*[=:]?\s*["']?"#,
                r#"(?P<secret>[^"'\s<>,;]+)"#,
            ),
            r"(?i)<(?:password|secret|psk|key)>(?P<secret>[^<]+)</",
        ]
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect()
    })
}

/// Masque les secrets d'un texte de version, selon le profil `profile_id`
/// (celui qui l'a relevé) et les règles de secours. Chaque valeur devient
/// `<masked:xxxxxx>`, une empreinte à clé courte : un secret changé reste
/// visible dans un diff sans être révélé, et l'empreinte ne se vérifie pas
/// hors de cette instance. Rend le texte et le nombre de valeurs masquées.
pub fn mask(cipher: &Cipher, profile_id: &str, config: &str) -> (String, usize) {
    let mut rules: Vec<Regex> = fetcher::profile::find(profile_id)
        .map(|profile| profile.secret_rules())
        .unwrap_or_default();
    rules.extend(fallback_rules().iter().cloned());
    let tag = |secret: &str| {
        let digest = cipher.keyed_digest(MASK_LABEL, secret.as_bytes());
        format!("<masked:{}>", hex::encode(&digest[..3]))
    };
    text::mask(config, &rules, &tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> Cipher {
        Cipher::from_key([7u8; 32])
    }

    fn target(tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "sw1".into(),
            address: "192.0.2.1".into(),
            kind: KIND.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn versions_round_trip_sealed() {
        let cipher = cipher();
        let config = "hostname sw1\nenable secret 5 $1$abc\n";
        let sealed = seal(&cipher, config).unwrap();
        assert!(!sealed.windows(6).any(|w| w == b"$1$abc"), "stored in clear");
        assert_eq!(open(&cipher, &sealed).unwrap(), config);
        // Une autre clé ne lit rien.
        assert!(open(&Cipher::from_key([8u8; 32]), &sealed).is_err());
    }

    #[test]
    fn masking_hides_secrets_but_shows_that_they_changed() {
        let cipher = cipher();
        let before = "hostname sw1\nsnmp-server community public RO\nenable secret 5 $1$old\n";
        let after = "hostname sw1\nsnmp-server community s3cr3t RO\nenable secret 5 $1$old\n";
        let (masked_before, count) = mask(&cipher, "cisco_ios", before);
        let (masked_after, _) = mask(&cipher, "cisco_ios", after);
        assert!(count >= 2);
        for secret in ["public", "s3cr3t", "$1$old"] {
            assert!(!masked_before.contains(secret) && !masked_after.contains(secret), "{secret}");
        }
        assert!(masked_before.contains("hostname sw1"));
        // Le secret changé change d'étiquette, celui qui n'a pas bougé la garde.
        let lines_before: Vec<&str> = masked_before.lines().collect();
        let lines_after: Vec<&str> = masked_after.lines().collect();
        assert_ne!(lines_before[1], lines_after[1]);
        assert_eq!(lines_before[2], lines_after[2]);
    }

    #[test]
    fn an_unknown_profile_still_masks_obvious_secrets() {
        let (masked, count) = mask(&cipher(), "gone", "user admin password hunter2\n");
        assert_eq!(count, 1);
        assert!(!masked.contains("hunter2"));
    }

    #[test]
    fn the_digest_ignores_volatile_lines_and_needs_the_key() {
        let cipher = cipher();
        let profile = fetcher::profile::find("cisco_ios").unwrap();
        let volatile = profile.volatile_rules();
        let a = text::normalize("! Last configuration change at 10:00\nhostname sw1\n", &volatile);
        let b = text::normalize("! Last configuration change at 12:00\nhostname sw1\n", &volatile);
        assert_eq!(digest(&cipher, &a), digest(&cipher, &b));
        assert_ne!(digest(&cipher, &a), digest(&Cipher::from_key([9u8; 32]), &a));
    }

    #[test]
    fn schedule_options_are_bounded() {
        assert_eq!(Schedule::from_target(&target(&[])).unwrap(), Schedule::default());
        assert!(Schedule::from_target(&target(&[("fetch_interval_hours", "0")])).is_err());
        assert!(Schedule::from_target(&target(&[("keep_versions", "5000")])).is_err());
        let custom = Schedule::from_target(&target(&[("fetch_interval_hours", "6")])).unwrap();
        assert_eq!(custom.interval_secs, 6 * 3600);
    }

    #[test]
    fn relay_errors_keep_their_nature() {
        assert_eq!(classify_text("Timed out after 60s"), "down");
        assert_eq!(classify_text("Monitor unreachable: refused"), "down");
        assert_eq!(classify_text("Authentication refused: bad password"), "auth");
        assert_eq!(classify_text("Invalid response: no prompt"), "config");
    }
}
