//! Qui surveille le surveillant ?
//!
//! Deux réponses, volontairement simples.
//!
//! **Le battement de cœur sortant.** Si l'hôte de DumbMonit tombe, rien ici ne
//! peut le dire : il faut quelqu'un dehors. Un service de « dead man's switch »
//! (Healthchecks.io, Uptime Kuma en mode push, Better Stack, Cronitor…) attend un
//! appel HTTP régulier et alerte quand il cesse. Le serveur l'envoie toutes les
//! `interval_seconds`, **seulement si le moteur va bien** — planificateur qui
//! tourne, base SQLite qui répond, VictoriaMetrics joignable. Sinon il se tait,
//! et c'est le silence qui alerte. Sur demande, il envoie plutôt un signal
//! d'échec explicite (`/fail`, ou `status=down` pour Uptime Kuma), ce qui fait
//! gagner le délai de grâce du service.
//!
//! **L'auto-surveillance.** Toutes les minutes, quelques mesures de l'instance
//! partent dans VictoriaMetrics sous `dumbmonit_instance_*` : place libre du
//! volume de données, taille de la base, relances du VictoriaMetrics embarqué,
//! retard du planificateur, écritures ratées. Des règles livrées
//! (`alerting::rules`, bloc « DumbMonit lui-même ») les lisent : les alertes, les
//! silences et les canaux sont ceux de tout le reste, pas un système parallèle.
//! Ce qu'aucune règle ne peut voir — VictoriaMetrics lui-même arrêté, qui porte
//! l'évaluation des règles — c'est le battement de cœur qui le couvre.
//!
//! L'URL du battement contient souvent son propre secret (l'UUID de
//! Healthchecks.io, le jeton d'Uptime Kuma) : elle est rangée chiffrée, ne
//! revient jamais par l'API (seul son hôte est montré) et n'apparaît dans aucun
//! journal — les erreurs passent par l'expurgation de `notify::http`. L'envoi
//! suit la garde des canaux de notification (`notify::guard`) : jamais vers
//! l'interface de ce serveur ni vers son VictoriaMetrics.

use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use anyhow::Result;
use reqwest::{Method, Url};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tracing::{debug, warn};

use crate::auth::settings;
use crate::notify::{self, http};
use crate::state::AppState;
use crate::stats::stats;

/// Réglage du battement (JSON en clair dans `settings`).
const CONFIG_KEY: &str = "self_monitoring.heartbeat";
/// URL du battement, chiffrée avec le secret d'instance.
const URL_KEY: &str = "self_monitoring.heartbeat_url";

/// Période par défaut, bornes comprises.
pub const DEFAULT_INTERVAL: u64 = 60;
pub const MIN_INTERVAL: u64 = 30;
pub const MAX_INTERVAL: u64 = 3600;

/// Délai d'un appel : court, un service de battement répond en quelques
/// dizaines de millisecondes, et un appel en vol ne doit pas chevaucher le suivant.
const PING_TIMEOUT: Duration = Duration::from_secs(10);

/// Délai de chaque contrôle de composant.
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// La boucle du planificateur tourne chaque seconde ; un tour peut attendre
/// un jeton d'exécution le temps d'une sonde. Deux minutes sans tour, elle est
/// bloquée ou morte.
pub const SCHEDULER_STALE: Duration = Duration::from_secs(120);

/// Période de publication des mesures `dumbmonit_instance_*`.
const PUBLISH_EVERY: Duration = Duration::from_secs(60);

/// Relecture du réglage quand le battement est coupé.
const IDLE_RECHECK: Duration = Duration::from_secs(60);

/// Noms des mesures publiées (préfixe `dumbmonit_` ajouté à l'écriture).
pub const METRIC_DATA_FREE: &str = "instance_data_free_bytes";
pub const METRIC_DATA_SIZE: &str = "instance_data_size_bytes";
pub const METRIC_DATABASE: &str = "instance_database_bytes";
pub const METRIC_SCHEDULER_LAG: &str = "instance_scheduler_lag_seconds";
pub const METRIC_SCHEDULER_BACKLOG: &str = "instance_scheduler_backlog";
pub const METRIC_WRITES_FAILED: &str = "instance_sample_writes_failed_total";
pub const METRIC_SAMPLES_DROPPED: &str = "instance_samples_dropped_total";
pub const METRIC_VM_RESTARTS: &str = "instance_vm_restarts_total";

/// Réveille la boucle du battement après un changement de réglage.
static WAKE: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Dernier battement tenté, pour l'interface. En mémoire : un redémarrage
/// l'oublie, et le suivant arrive dans la minute.
static LAST_PING: Mutex<Option<PingRecord>> = Mutex::new(None);

// --------------------------------------------------------------------------
// Réglage
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    /// Envoyer un signal d'échec plutôt que se taire quand un composant est KO.
    pub send_fail: bool,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self { enabled: false, interval_seconds: DEFAULT_INTERVAL, send_fail: false }
    }
}

pub async fn load_config(state: &AppState) -> Result<HeartbeatConfig> {
    Ok(settings::get(&state.pool, CONFIG_KEY).await?.unwrap_or_default())
}

pub async fn save_config(state: &AppState, config: &HeartbeatConfig) -> Result<()> {
    settings::set(&state.pool, CONFIG_KEY, config).await?;
    WAKE.notify_one();
    Ok(())
}

pub async fn load_url(state: &AppState) -> Result<Option<String>> {
    let url = settings::get_secret(&state.pool, &state.cipher, URL_KEY).await?;
    Ok(url.filter(|url| !url.trim().is_empty()))
}

/// Range l'URL, ou l'efface quand elle est vide.
pub async fn save_url(state: &AppState, url: &str) -> Result<()> {
    let url = url.trim();
    if url.is_empty() {
        settings::delete(&state.pool, URL_KEY).await?;
    } else {
        settings::set_secret(&state.pool, &state.cipher, URL_KEY, url).await?;
    }
    WAKE.notify_one();
    Ok(())
}

/// Vérifie une URL saisie. Le message ne la cite jamais : elle est le secret.
pub fn check_url(raw: &str) -> Result<(), String> {
    http::check_url(raw, "Heartbeat URL").map_err(|error| match error {
        notify::error::NotifyError::Config(why) => why,
        other => other.to_string(),
    })?;
    match Url::parse(raw.trim()) {
        Ok(url) if url.host_str().is_some() => Ok(()),
        _ => Err("\"Heartbeat URL\" is not a valid address".to_string()),
    }
}

/// Hôte seul, ce que l'interface peut montrer sans rien divulguer.
pub fn url_host(raw: &str) -> Option<String> {
    Url::parse(raw.trim()).ok()?.host_str().map(str::to_string)
}

/// Adresse du signal d'échec.
///
/// Uptime Kuma lit `status=up` dans la requête : il devient `status=down`. Les
/// autres (Healthchecks.io, Better Stack, Cronitor…) prennent un `/fail` au bout
/// du chemin.
pub fn fail_url(raw: &str) -> Option<String> {
    let mut url = Url::parse(raw.trim()).ok()?;
    if url.query_pairs().any(|(name, _)| name == "status") {
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(name, value)| {
                let value = if name == "status" { "down".to_string() } else { value.into_owned() };
                (name.into_owned(), value)
            })
            .collect();
        url.query_pairs_mut().clear().extend_pairs(pairs);
    } else {
        let path = format!("{}/fail", url.path().trim_end_matches('/'));
        url.set_path(&path);
    }
    Some(url.to_string())
}

// --------------------------------------------------------------------------
// Santé du moteur
// --------------------------------------------------------------------------

/// Un composant du moteur et son état.
#[derive(Debug, Clone, Serialize)]
pub struct Component {
    /// `scheduler`, `database` ou `victoria`.
    pub name: &'static str,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Component {
    fn from(name: &'static str, result: Result<(), String>) -> Self {
        match result {
            Ok(()) => Self { name, ok: true, error: None },
            Err(error) => Self { name, ok: false, error: Some(error) },
        }
    }
}

/// Les trois conditions d'un battement : planificateur, base, VictoriaMetrics.
pub async fn components(state: &AppState) -> Vec<Component> {
    let idle = stats().scheduler_idle();
    let scheduler = scheduler_verdict(idle);

    let query = sqlx::query("SELECT 1").execute(&state.pool);
    let database = match tokio::time::timeout(CHECK_TIMEOUT, query).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err(format!("no answer within {} s", CHECK_TIMEOUT.as_secs())),
    };

    let victoria = match tokio::time::timeout(CHECK_TIMEOUT, state.victoria.health()).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!("{error:#}")),
        Err(_) => Err(format!("no answer within {} s", CHECK_TIMEOUT.as_secs())),
    };

    vec![
        Component::from("scheduler", scheduler),
        Component::from("database", database),
        Component::from("victoria", victoria),
    ]
}

fn scheduler_verdict(idle: Duration) -> Result<(), String> {
    if idle < SCHEDULER_STALE {
        Ok(())
    } else {
        Err(format!("no scheduling round for {} s", idle.as_secs()))
    }
}

// --------------------------------------------------------------------------
// Battement
// --------------------------------------------------------------------------

/// Ce qu'a donné le dernier battement.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PingRecord {
    /// UTC sans suffixe, comme partout dans l'API.
    pub at: String,
    /// `up` (tout va bien, appel envoyé), `fail` (signal d'échec envoyé) ou
    /// `withheld` (un composant est KO, rien n'est parti).
    pub outcome: &'static str,
    /// Vrai quand l'appel est parti et que le service l'a accepté.
    pub ok: bool,
    /// Erreur d'envoi, déjà expurgée.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Composants KO au moment du battement.
    pub failed: Vec<&'static str>,
}

pub fn last_ping() -> Option<PingRecord> {
    LAST_PING.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn record(ping: PingRecord) {
    *LAST_PING.lock().unwrap_or_else(|e| e.into_inner()) = Some(ping);
}

fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Un appel vers le service. L'erreur rendue est expurgée de l'URL.
pub async fn ping(client: &reqwest::Client, url: &str, fail: bool) -> Result<(), String> {
    if notify::sending_disabled() {
        return Err(notify::DISABLED_MESSAGE.to_string());
    }
    let target = match fail {
        true => fail_url(url).ok_or_else(|| "the heartbeat URL is not valid".to_string())?,
        false => url.trim().to_string(),
    };
    let request = http::Request::new(Method::GET, &target)
        .secret_text(url.trim())
        .secret_text(target.clone())
        .timeout(PING_TIMEOUT);
    http::send(client, request).await.map_err(|error| error.to_string())
}

/// Un battement complet : contrôle, puis appel ou silence.
async fn beat(state: &AppState, client: &reqwest::Client, config: &HeartbeatConfig, url: &str) {
    let failed: Vec<&'static str> =
        components(state).await.into_iter().filter(|c| !c.ok).map(|c| c.name).collect();
    let host = url_host(url).unwrap_or_default();

    if failed.is_empty() {
        let result = ping(client, url, false).await;
        match &result {
            Ok(()) => debug!(%host, "heartbeat sent"),
            Err(error) => warn!(%host, %error, "heartbeat not accepted"),
        }
        let ok = result.is_ok();
        record(PingRecord { at: now(), outcome: "up", ok, error: result.err(), failed });
        return;
    }

    if config.send_fail {
        let result = ping(client, url, true).await;
        let ok = result.is_ok();
        warn!(%host, components = ?failed, sent = ok, "heartbeat: failure signal");
        record(PingRecord { at: now(), outcome: "fail", ok, error: result.err(), failed });
    } else {
        warn!(components = ?failed, "heartbeat withheld: a component of DumbMonit is down");
        record(PingRecord { at: now(), outcome: "withheld", ok: false, error: None, failed });
    }
}

async fn heartbeat_loop(state: AppState) {
    let client = notify::http_client();
    // Dernier réglage lu : une base qui ne répond plus ne doit pas empêcher le
    // signal d'échec, qui est fait pour ce cas-là.
    let mut cached: Option<(HeartbeatConfig, Option<String>)> = None;
    loop {
        match (load_config(&state).await, load_url(&state).await) {
            (Ok(config), Ok(url)) => cached = Some((config, url)),
            (Err(error), _) | (_, Err(error)) => {
                warn!(%error, "heartbeat settings unreadable, last known ones kept")
            }
        }

        let wait = match &cached {
            Some((config, Some(url))) if config.enabled => {
                beat(&state, &client, config, url).await;
                Duration::from_secs(config.interval_seconds.clamp(MIN_INTERVAL, MAX_INTERVAL))
            }
            _ => IDLE_RECHECK,
        };
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = WAKE.notified() => {}
        }
    }
}

// --------------------------------------------------------------------------
// Mesures de l'instance
// --------------------------------------------------------------------------

/// Place libre et taille du volume qui porte `path`, en octets.
#[cfg(unix)]
#[allow(clippy::unnecessary_cast, clippy::useless_conversion)]
pub fn volume(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: `statvfs` remplit la structure qu'on lui passe, initialisée à zéro,
    // et ne garde aucun pointeur au-delà de l'appel.
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    let unit = match stat.f_frsize as u64 {
        0 => stat.f_bsize as u64,
        frsize => frsize,
    };
    Some((stat.f_bavail as u64 * unit, stat.f_blocks as u64 * unit))
}

#[cfg(not(unix))]
pub fn volume(_path: &Path) -> Option<(u64, u64)> {
    None
}

/// Taille de la base, journal d'écriture compris : c'est ce qui occupe le volume.
pub async fn database_bytes(path: &Path) -> u64 {
    let mut total = 0;
    for suffix in ["", "-wal"] {
        let candidate = std::path::PathBuf::from(format!("{}{suffix}", path.display()));
        if let Ok(meta) = tokio::fs::metadata(&candidate).await {
            total += meta.len();
        }
    }
    total
}

/// Les mesures d'un tour de publication.
pub async fn samples(state: &AppState) -> Vec<dumbmonit_proto::Sample> {
    use dumbmonit_proto::{MetricKind, Sample};
    let now_ms = chrono::Utc::now().timestamp_millis();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, now_ms);
    let counter = |name: &str, value: f64| Sample::new(name, value, MetricKind::Counter, now_ms);

    let snapshot = stats().snapshot();
    let mut samples = vec![
        gauge(METRIC_DATABASE, database_bytes(&state.config.database_path()).await as f64),
        gauge(METRIC_SCHEDULER_LAG, stats().scheduler_idle().as_secs_f64()),
        gauge(METRIC_SCHEDULER_BACKLOG, snapshot.scheduler_backlog as f64),
        counter(METRIC_WRITES_FAILED, snapshot.sample_writes_failed as f64),
        counter(METRIC_SAMPLES_DROPPED, snapshot.samples_dropped as f64),
    ];
    if let Some((free, size)) = volume(&state.config.data_dir) {
        samples.push(gauge(METRIC_DATA_FREE, free as f64));
        samples.push(gauge(METRIC_DATA_SIZE, size as f64));
    }
    if state.config.vm_embedded() {
        samples.push(counter(METRIC_VM_RESTARTS, stats().vm_restarts() as f64));
    }
    samples
}

async fn publish_loop(state: AppState) {
    let mut ticker = tokio::time::interval(PUBLISH_EVERY);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        state.sink.send(samples(&state).await).await;
    }
}

/// Lance la publication des mesures et le battement de cœur.
pub fn spawn(state: AppState) {
    tokio::spawn(publish_loop(state.clone()));
    tokio::spawn(heartbeat_loop(state));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthchecks_prend_fail_au_bout_du_chemin() {
        assert_eq!(
            fail_url("https://hc-ping.com/0d3c1a2b-aaaa").as_deref(),
            Some("https://hc-ping.com/0d3c1a2b-aaaa/fail")
        );
        assert_eq!(
            fail_url("https://uptime.betterstack.com/api/v1/heartbeat/abc/").as_deref(),
            Some("https://uptime.betterstack.com/api/v1/heartbeat/abc/fail")
        );
    }

    #[test]
    fn uptime_kuma_passe_a_status_down() {
        let url = fail_url("https://kuma.lan/api/push/Tok3n?status=up&msg=OK&ping=").unwrap();
        assert!(url.starts_with("https://kuma.lan/api/push/Tok3n?"), "{url}");
        assert!(url.contains("status=down"), "{url}");
        assert!(url.contains("msg=OK"), "{url}");
        assert!(!url.contains("/fail"), "{url}");
    }

    #[test]
    fn une_url_saisie_est_verifiee_sans_etre_citee() {
        assert!(check_url("https://hc-ping.com/0d3c1a2b-aaaa").is_ok());
        let error = check_url("hc-ping.com/secret-uuid").unwrap_err();
        assert!(!error.contains("secret-uuid"), "{error}");
        assert!(check_url("ftp://hc-ping.com/secret-uuid").is_err());
    }

    #[test]
    fn seul_lhote_est_montre() {
        assert_eq!(url_host("https://hc-ping.com/secret").as_deref(), Some("hc-ping.com"));
        assert_eq!(url_host("pas une url"), None);
    }

    #[test]
    fn un_planificateur_muet_depuis_deux_minutes_est_ko() {
        assert!(scheduler_verdict(Duration::from_secs(2)).is_ok());
        assert!(scheduler_verdict(SCHEDULER_STALE).is_err());
    }

    #[test]
    fn le_reglage_par_defaut_est_coupe() {
        let config = HeartbeatConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.interval_seconds, DEFAULT_INTERVAL);
        assert!(!config.send_fail);
    }

    #[cfg(unix)]
    #[test]
    fn le_volume_courant_se_mesure() {
        let (free, size) = volume(Path::new(".")).expect("statvfs");
        assert!(size > 0 && free <= size);
    }
}
