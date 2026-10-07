//! Vérification de nouvelle version : le serveur interroge au plus une fois par
//! jour l'API publique de GitHub pour connaître la dernière release.
//!
//! Garanties, chacune tenue ici :
//!
//! * l'URL est écrite en dur ([`RELEASES_URL`]) — aucune adresse fournie par un
//!   utilisateur n'est jamais jointe, donc pas de SSRF possible ;
//! * la requête est anonyme et ne transporte que l'identité du produit
//!   (`User-Agent: DumbMonit/<version>`) : ni identifiant d'instance, ni donnée ;
//! * le résultat est gardé en mémoire (24 h, 1 h après un échec) ; « Check now »
//!   est limité à une fois par minute ;
//! * `DUMBMONIT_UPDATE_CHECK=off` ou le réglage de l'interface coupe tout appel
//!   réseau (parcs sans accès à Internet) ;
//! * rien n'est jamais installé automatiquement.

use std::cmp::Ordering;
use std::future::Future;
use std::sync::OnceLock;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::config::{env_flag_or, env_var};

/// Seule adresse jamais contactée.
pub const RELEASES_URL: &str =
    "https://api.github.com/repos/laupernoe/dumbmonit/releases?per_page=15";
/// Page des releases, repli quand GitHub ne donne pas de lien.
pub const RELEASES_PAGE: &str = "https://github.com/laupernoe/dumbmonit/releases";

const TTL: TimeDelta = TimeDelta::hours(24);
const RETRY_AFTER_FAILURE: TimeDelta = TimeDelta::hours(1);
const MIN_FORCE_INTERVAL: TimeDelta = TimeDelta::seconds(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const NOTES_MAX_CHARS: usize = 700;
const SETTING_KEY: &str = "update_check";

// ----------------------------------------------------------------- versions

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ident {
    Num(u64),
    Text(String),
}

impl Ord for Ident {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a.cmp(b),
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
            // Semver : un identifiant numérique est plus petit qu'un alphanumérique.
            (Self::Num(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Num(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Version semver (`1.2.3`, `0.1.0-alpha.6`), préfixe `v` et métadonnées `+…` tolérés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    pre: Vec<Ident>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches('v');
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut parts = core.split('.');
        let mut numbers = [0u64; 3];
        for slot in &mut numbers {
            *slot = parts.next()?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(pre) => pre
                .split('.')
                .map(|id| match id.parse::<u64>() {
                    Ok(n) => Some(Ident::Num(n)),
                    Err(_) if !id.is_empty() => Some(Ident::Text(id.to_string())),
                    Err(_) => None,
                })
                .collect::<Option<Vec<_>>>()?,
        };
        Some(Self { core: numbers, pre })
    }

    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            // Une version sans pré-version est plus grande que `-alpha.N`.
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.pre.cmp(&other.pre),
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ----------------------------------------------------------------- releases

/// Ce que l'on lit d'une release GitHub.
#[derive(Debug, Deserialize)]
pub struct Release {
    tag_name: String,
    html_url: Option<String>,
    body: Option<String>,
    #[serde(default)]
    draft: bool,
}

/// La plus récente des releases publiées que `current` doit considérer.
///
/// Les brouillons sont ignorés ; une pré-version n'est proposée qu'à qui en
/// exécute déjà une (un utilisateur de version stable n'est pas invité à passer
/// en alpha).
pub fn newest(releases: Vec<Release>, current: &Version) -> Option<Latest> {
    releases
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| Version::parse(&r.tag_name).map(|v| (v, r)))
        .filter(|(v, _)| current.is_prerelease() || !v.is_prerelease())
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, r)| Latest {
            version: r.tag_name.trim_start_matches('v').to_string(),
            url: r.html_url.filter(|u| u.starts_with("https://github.com/")),
            notes: r.body.map(|b| short_notes(&b)).filter(|n| !n.is_empty()),
        })
}

fn short_notes(body: &str) -> String {
    let body = body.replace('\r', "");
    let body = body.trim();
    if body.chars().count() <= NOTES_MAX_CHARS {
        return body.to_string();
    }
    let cut: String = body.chars().take(NOTES_MAX_CHARS).collect();
    format!("{}…", cut.trim_end())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Latest {
    pub version: String,
    pub url: Option<String>,
    pub notes: Option<String>,
}

// -------------------------------------------------------------------- cache

/// Ce que `GET /api/update` répond.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    /// Dernière version connue ; absente tant qu'aucune vérification n'a abouti.
    pub latest: Option<String>,
    pub release_url: Option<String>,
    pub notes: Option<String>,
    pub update_available: bool,
    /// Dernière vérification réussie, UTC sans suffixe (`2026-10-07 09:00:00`).
    pub checked_at: Option<String>,
    /// La vérification est active (ni réglage coupé, ni variable d'environnement).
    pub enabled: bool,
    /// Coupée par `DUMBMONIT_UPDATE_CHECK=off` : l'interface ne peut pas la rallumer.
    pub locked_by_env: bool,
    /// Dernier échec de vérification, en clair.
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum CheckError {
    /// « Check now » trop rapproché ; secondes à attendre.
    TooSoon(i64),
}

#[derive(Default)]
struct Cache {
    latest: Option<Latest>,
    checked_at: Option<DateTime<Utc>>,
    last_attempt: Option<DateTime<Utc>>,
    error: Option<String>,
}

pub struct Checker {
    current: String,
    cache: Mutex<Cache>,
}

impl Checker {
    pub fn new(current: &str) -> Self {
        Self { current: current.to_string(), cache: Mutex::new(Cache::default()) }
    }

    /// Instance du processus.
    pub fn global() -> &'static Checker {
        static CHECKER: OnceLock<Checker> = OnceLock::new();
        CHECKER.get_or_init(|| Checker::new(env!("CARGO_PKG_VERSION")))
    }

    fn info(&self, cache: &Cache, enabled: bool, locked_by_env: bool) -> UpdateInfo {
        let newer = match (&cache.latest, Version::parse(&self.current)) {
            (Some(latest), Some(current)) => {
                Version::parse(&latest.version).is_some_and(|l| l > current)
            }
            _ => false,
        };
        UpdateInfo {
            current: self.current.clone(),
            latest: cache.latest.as_ref().map(|l| l.version.clone()),
            release_url: cache
                .latest
                .as_ref()
                .map(|l| l.url.clone().unwrap_or_else(|| RELEASES_PAGE.to_string())),
            notes: cache.latest.as_ref().and_then(|l| l.notes.clone()),
            update_available: enabled && newer,
            checked_at: cache.checked_at.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            enabled,
            locked_by_env,
            error: cache.error.clone(),
        }
    }

    /// État courant ; interroge GitHub si la vérification est active et que le
    /// cache est périmé, ou si `force`. `fetch` n'est appelé que dans ces cas.
    pub async fn get<F, Fut>(
        &self,
        now: DateTime<Utc>,
        enabled: bool,
        locked_by_env: bool,
        force: bool,
        fetch: F,
    ) -> Result<UpdateInfo, CheckError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<Release>, String>>,
    {
        let mut cache = self.cache.lock().await;
        if !enabled {
            return Ok(self.info(&cache, false, locked_by_env));
        }
        if force && let Some(last) = cache.last_attempt {
            let wait = MIN_FORCE_INTERVAL - (now - last);
            if wait > TimeDelta::zero() {
                return Err(CheckError::TooSoon(wait.num_seconds().max(1)));
            }
        }
        let ttl = if cache.error.is_some() { RETRY_AFTER_FAILURE } else { TTL };
        let stale = cache.last_attempt.is_none_or(|last| now - last >= ttl);
        if force || stale {
            cache.last_attempt = Some(now);
            match fetch().await {
                Ok(releases) => {
                    let current = Version::parse(&self.current);
                    cache.latest = current.and_then(|c| newest(releases, &c));
                    cache.checked_at = Some(now);
                    cache.error = None;
                }
                Err(error) => {
                    tracing::debug!(%error, "update check failed");
                    cache.error = Some(error);
                }
            }
        }
        Ok(self.info(&cache, true, locked_by_env))
    }
}

/// Interroge GitHub. Aucune donnée de l'instance n'est envoyée.
pub async fn fetch_releases() -> Result<Vec<Release>, String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION"), " (update check)"))
        .build()
        .map_err(|e| format!("HTTP client unavailable: {e}"))?;
    let response = client
        .get(RELEASES_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Could not reach api.github.com.".to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub answered {}.", response.status()));
    }
    // Réponse bornée : quinze releases pèsent quelques dizaines de Kio.
    let bytes = response.bytes().await.map_err(|_| "GitHub's answer was cut short.".to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("GitHub's answer was unexpectedly large.".to_string());
    }
    serde_json::from_slice(&bytes).map_err(|_| "GitHub's answer was not understood.".to_string())
}

// ----------------------------------------------------------------- réglage

/// `DUMBMONIT_UPDATE_CHECK=off` : coupure imposée par l'environnement.
pub fn env_disabled() -> bool {
    env_var("DUMBMONIT_UPDATE_CHECK").is_some() && !env_flag_or("DUMBMONIT_UPDATE_CHECK", true)
}

pub async fn setting_enabled(pool: &SqlitePool) -> sqlx::Result<bool> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(SETTING_KEY)
        .fetch_optional(pool)
        .await?;
    Ok(value.as_deref() != Some("off"))
}

pub async fn set_setting_enabled(pool: &SqlitePool, enabled: bool) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(SETTING_KEY)
    .bind(if enabled { "on" } else { "off" })
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn release(tag: &str, draft: bool) -> Release {
        Release { tag_name: tag.into(), html_url: None, body: None, draft }
    }

    fn at(minutes: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_790_000_000, 0).unwrap() + TimeDelta::minutes(minutes)
    }

    #[test]
    fn prereleases_compare_like_semver() {
        assert!(v("0.1.0-alpha.7") > v("0.1.0-alpha.6"));
        assert!(v("0.1.0-alpha.10") > v("0.1.0-alpha.9"));
        assert!(v("0.1.0") > v("0.1.0-rc.1"));
        assert!(v("0.1.0-beta.1") > v("0.1.0-alpha.9"));
        assert!(v("0.2.0-alpha.1") > v("0.1.9"));
        assert!(v("v1.0.0+build5") == v("1.0.0"));
        assert!(Version::parse("nightly").is_none());
        assert!(Version::parse("1.2").is_none());
    }

    #[test]
    fn newest_skips_drafts_and_unparseable_tags() {
        let current = v("0.1.0-alpha.6");
        let latest = newest(
            vec![
                release("v0.1.0-alpha.5", false),
                release("v0.1.0-alpha.8", true),
                release("v0.1.0-alpha.7", false),
                release("weekly", false),
            ],
            &current,
        )
        .unwrap();
        assert_eq!(latest.version, "0.1.0-alpha.7");
    }

    #[test]
    fn stable_installs_are_not_offered_prereleases() {
        let latest =
            newest(vec![release("v1.1.0-alpha.1", false), release("v1.0.1", false)], &v("1.0.0"))
                .unwrap();
        assert_eq!(latest.version, "1.0.1");
    }

    #[tokio::test]
    async fn caches_for_a_day_and_rate_limits_forced_checks() {
        let checker = Checker::new("0.1.0-alpha.6");
        let calls = AtomicUsize::new(0);
        let fetch = || async {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(vec![release("v0.1.0-alpha.7", false)])
        };
        let info = checker.get(at(0), true, false, false, fetch).await.unwrap();
        assert!(info.update_available);
        assert_eq!(info.latest.as_deref(), Some("0.1.0-alpha.7"));
        // Dans la journée : servi par le cache.
        checker.get(at(600), true, false, false, fetch).await.unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
        // « Check now » après plus d'une minute : autorisé.
        checker.get(at(600), true, false, true, fetch).await.unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 2);
        // Aussitôt après : refusé, sans appel.
        let too_soon = checker.get(at(600), true, false, true, fetch).await;
        assert!(matches!(too_soon, Err(CheckError::TooSoon(_))));
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 2);
        // Plus de 24 h après la dernière tentative : nouvelle interrogation.
        checker.get(at(600 + 24 * 60), true, false, false, fetch).await.unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 3);
    }

    #[tokio::test]
    async fn a_failure_keeps_the_last_answer_and_retries_after_an_hour() {
        let checker = Checker::new("0.1.0-alpha.6");
        checker
            .get(at(0), true, false, false, || async { Ok(vec![release("v0.1.0-alpha.7", false)]) })
            .await
            .unwrap();
        let failing = checker
            .get(at(24 * 60), true, false, false, || async { Err("down".to_string()) })
            .await
            .unwrap();
        assert_eq!(failing.error.as_deref(), Some("down"));
        assert_eq!(failing.latest.as_deref(), Some("0.1.0-alpha.7"));
        let calls = AtomicUsize::new(0);
        let fetch = || async {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            Err::<Vec<Release>, _>("down".to_string())
        };
        checker.get(at(24 * 60 + 30), true, false, false, fetch).await.unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 0);
        checker.get(at(24 * 60 + 61), true, false, false, fetch).await.unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn disabled_never_fetches() {
        let checker = Checker::new("0.1.0-alpha.6");
        let calls = AtomicUsize::new(0);
        let fetch = || async {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(vec![release("v9.0.0", false)])
        };
        let info = checker.get(at(0), false, true, true, fetch).await.unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 0);
        assert!(!info.enabled && info.locked_by_env && !info.update_available);
    }

    #[test]
    fn notes_are_shortened() {
        let long = "x".repeat(2000);
        assert!(short_notes(&long).chars().count() <= NOTES_MAX_CHARS + 1);
        assert_eq!(short_notes("  hi \r\n"), "hi");
    }
}
