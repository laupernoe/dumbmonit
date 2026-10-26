//! Stockage : rétentions réglables, historique long terme et VictoriaLogs
//! embarqué, espace disque.
//!
//! Trois processus enfants au plus, tous facultatifs sauf le premier quand
//! aucune instance externe n'est désignée :
//!
//! - VictoriaMetrics des mesures brutes (`/data/vm`) ;
//! - VictoriaMetrics de l'historique long terme (`/data/vm-long`), alimenté par
//!   l'[`aggregator`] avec des agrégats cinq minutes ;
//! - VictoriaLogs (`/data/vlogs`), coupé par défaut, qui reçoit le journal du
//!   serveur.
//!
//! Un changement de rétention fait depuis l'interface relance le processus
//! concerné (arrêt propre, relance, attente de `/health`) : quelques secondes
//! sans écriture, que le tampon d'écriture rattrape.

pub mod aggregator;
pub mod settings;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::Serialize;

use self::settings::{Source, StorageSettings};
use crate::state::AppState;
use crate::tsdb::{EmbeddedConfig, EmbeddedVm, Flavor, LongTerm, Victoria};

/// Processus embarqués et état de l'agrégateur.
#[derive(Default)]
pub struct Storage {
    processes: tokio::sync::Mutex<Processes>,
    /// Une seule modification à la fois : deux relances croisées du même
    /// processus ne mèneraient à rien de bon.
    applying: tokio::sync::Mutex<()>,
    status: std::sync::Mutex<Status>,
}

#[derive(Default)]
struct Processes {
    metrics: Option<EmbeddedVm>,
    long: Option<EmbeddedVm>,
    logs: Option<EmbeddedVm>,
}

/// Ce que l'agrégateur et VictoriaLogs ont fait en dernier.
#[derive(Debug, Default, Clone)]
pub struct Status {
    pub last_window: Option<i64>,
    pub backfilled_until: Option<i64>,
    pub backfill_done: bool,
    pub series: Option<u64>,
    pub rollup_error: Option<String>,
    pub logs_error: Option<String>,
}

impl Storage {
    /// Pose le processus des mesures brutes, lancé par `main`.
    pub async fn set_metrics(&self, vm: Option<EmbeddedVm>) {
        self.processes.lock().await.metrics = vm;
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    pub fn update_status(&self, change: impl FnOnce(&mut Status)) {
        change(&mut self.status.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
    }

    /// Arrêt ordonné de tous les processus, VictoriaMetrics brut en dernier.
    pub async fn stop_all(&self) {
        let mut processes = self.processes.lock().await;
        if let Some(logs) = processes.logs.take() {
            logs.stop().await;
        }
        if let Some(long) = processes.long.take() {
            long.stop().await;
        }
        if let Some(metrics) = processes.metrics.take() {
            metrics.stop().await;
        }
    }
}

fn admin_key() -> String {
    crate::crypto::generate_secret()
}

fn long_config(state: &AppState, retention: &str, key: &str) -> EmbeddedConfig {
    let config = &state.config;
    EmbeddedConfig {
        flavor: Flavor::LongTerm,
        binary: config.vm_binary.clone(),
        data_path: config.vm_long_data_path(),
        listen: config.vm_long_listen.clone(),
        retention: retention.to_string(),
        memory: config.vm_long_memory.clone(),
        admin_key: key.to_string(),
        health_url: format!("{}/health", config.effective_long_url().unwrap_or_default()),
    }
}

fn logs_config(state: &AppState, retention: &str) -> EmbeddedConfig {
    let config = &state.config;
    EmbeddedConfig {
        flavor: Flavor::Logs,
        binary: config.vlogs_binary.clone(),
        data_path: config.vlogs_data_path(),
        listen: config.vlogs_listen.clone(),
        retention: retention.to_string(),
        memory: config.vlogs_memory.clone(),
        admin_key: String::new(),
        health_url: format!("{}/health", config.vlogs_url()),
    }
}

/// Lance l'historique long terme (si activé) et VictoriaLogs (si activé),
/// puis branche l'aiguillage des lectures. Rien ici n'empêche le serveur de
/// démarrer : un échec est journalisé et montré dans l'interface.
pub async fn start(state: &AppState) {
    let settings = match settings::load(&state.pool, &state.config).await {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), "storage settings unreadable");
            settings::resolve(&state.config, |_| None)
        }
    };
    if settings.rollup_enabled
        && let Err(error) = enable_rollup(state, &settings).await
    {
        tracing::warn!(error = %format!("{error:#}"), "long-term history not started");
        state.storage().update_status(|s| s.rollup_error = Some(format!("{error:#}")));
    }
    if settings.logs_enabled
        && let Err(error) = enable_logs(state, &settings.logs.flag).await
    {
        tracing::warn!(error = %format!("{error:#}"), "embedded VictoriaLogs not started");
        state.storage().update_status(|s| s.logs_error = Some(format!("{error:#}")));
    }
    aggregator::spawn(state.clone());
}

/// Démarre (au besoin) le stockage long terme et branche l'aiguillage.
async fn enable_rollup(state: &AppState, settings: &StorageSettings) -> Result<()> {
    let Some(url) = state.config.effective_long_url() else {
        anyhow::bail!(
            "no long-term store: the metrics store is external and DUMBMONIT_VM_LONG_URL is not set"
        );
    };
    let mut victoria = Victoria::new(&url)?;
    if state.config.vm_long_embedded() {
        let mut processes = state.storage().processes.lock().await;
        if processes.long.is_none() {
            let key = admin_key();
            let vm = EmbeddedVm::start(long_config(state, &settings.rollup.flag, &key))
                .await
                .context("starting the long-term VictoriaMetrics")?;
            processes.long = Some(vm);
            victoria = victoria.with_admin_key(key);
        } else if let Some(existing) = state.victoria.long_term() {
            victoria = existing.victoria.clone();
        }
    }
    let raw_secs = i64::from(settings.metrics.days) * 86_400;
    let long = Arc::new(LongTerm::new(victoria, raw_secs));
    // La couverture déjà acquise (lancements précédents) compte tout de suite.
    if let Some(oldest) = aggregator::stored_backfill(&state.pool).await {
        long.extend_coverage(oldest);
    }
    state.victoria.set_long_term(Some(long));
    state.storage().update_status(|s| s.rollup_error = None);
    Ok(())
}

async fn disable_rollup(state: &AppState) {
    if let Some(long) = state.victoria.long_term() {
        long.set_enabled(false);
    }
    if let Some(vm) = state.storage().processes.lock().await.long.take() {
        vm.stop().await;
    }
}

async fn enable_logs(state: &AppState, retention: &str) -> Result<()> {
    {
        let mut processes = state.storage().processes.lock().await;
        if processes.logs.is_none() {
            let vm = EmbeddedVm::start(logs_config(state, retention))
                .await
                .context("starting the embedded VictoriaLogs")?;
            processes.logs = Some(vm);
        }
    }
    crate::logs::ship::set_embedded(Some(state.config.vlogs_url()));
    crate::logs::ship::reload(&state.pool, &state.cipher).await?;
    state.storage().update_status(|s| s.logs_error = None);
    Ok(())
}

async fn disable_logs(state: &AppState) -> Result<()> {
    crate::logs::ship::set_embedded(None);
    crate::logs::ship::reload(&state.pool, &state.cipher).await?;
    if let Some(vm) = state.storage().processes.lock().await.logs.take() {
        vm.stop().await;
    }
    Ok(())
}

/// Le VictoriaLogs embarqué tourne-t-il ? (Adresse, pour la recherche.)
pub async fn logs_url(state: &AppState) -> Option<String> {
    state.storage().processes.lock().await.logs.as_ref().map(|_| state.config.vlogs_url())
}

// --------------------------------------------------------------------------
// Vue et modification
// --------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MetricsView {
    pub embedded: bool,
    pub retention_days: u32,
    pub retention_source: Source,
    pub running: bool,
}

#[derive(Debug, Serialize)]
pub struct RollupView {
    pub available: bool,
    pub embedded: bool,
    pub enabled: bool,
    pub running: bool,
    pub retention_days: u32,
    pub step_seconds: i64,
    pub last_window: Option<String>,
    pub backfilled_until: Option<String>,
    pub backfill_done: bool,
    pub series: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogsStoreView {
    pub available: bool,
    pub enabled: bool,
    pub running: bool,
    pub retention_days: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UsageView {
    pub key: &'static str,
    pub path: String,
    pub bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct DiskView {
    pub usage: Vec<UsageView>,
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct GrowthView {
    pub bytes_per_day: Option<f64>,
    pub days_until_full: Option<f64>,
    pub sampled_days: u32,
}

#[derive(Debug, Serialize)]
pub struct LimitsView {
    pub min_retention_days: u32,
    pub max_retention_days: u32,
    pub min_rollup_retention_days: u32,
    pub max_rollup_retention_days: u32,
    pub min_logs_retention_days: u32,
    pub max_logs_retention_days: u32,
}

#[derive(Debug, Serialize)]
pub struct StorageView {
    pub metrics: MetricsView,
    pub rollup: RollupView,
    pub logs_store: LogsStoreView,
    pub disk: DiskView,
    pub growth: GrowthView,
    pub limits: LimitsView,
    pub restarted: Vec<&'static str>,
}

/// Horodatage à la manière du reste de l'API : UTC, sans suffixe.
fn server_time(secs: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(secs, 0).map(|at| at.format("%Y-%m-%d %H:%M:%S").to_string())
}

pub async fn view(state: &AppState) -> Result<StorageView> {
    let settings = settings::load(&state.pool, &state.config).await?;
    let config = &state.config;
    let (metrics_vm, long_vm, logs_vm) = {
        let processes = state.storage().processes.lock().await;
        (processes.metrics.clone(), processes.long.clone(), processes.logs.clone())
    };
    let metrics_running = match &metrics_vm {
        Some(vm) => vm.healthy().await,
        None => state.victoria.health().await.is_ok(),
    };
    let long = state.victoria.long_term().filter(|long| long.enabled());
    let long_running = match (&long_vm, &long) {
        (Some(vm), _) => vm.healthy().await,
        (None, Some(long)) => long.victoria.health().await.is_ok(),
        (None, None) => false,
    };
    let logs_running = match &logs_vm {
        Some(vm) => vm.healthy().await,
        None => false,
    };
    let status = state.storage().status();
    let disk = disk_view(state).await;
    let growth = aggregator::growth(&state.pool, disk.free_bytes).await;
    Ok(StorageView {
        metrics: MetricsView {
            embedded: config.vm_embedded(),
            retention_days: settings.metrics.days,
            retention_source: settings.metrics.source,
            running: metrics_running,
        },
        rollup: RollupView {
            available: config.effective_long_url().is_some(),
            embedded: config.vm_long_embedded(),
            enabled: settings.rollup_enabled,
            running: long_running,
            retention_days: settings.rollup.days,
            step_seconds: crate::tsdb::rollup::STEP_SECS,
            last_window: status.last_window.and_then(server_time),
            backfilled_until: status.backfilled_until.and_then(server_time),
            backfill_done: status.backfill_done,
            series: status.series,
            last_error: status.rollup_error,
        },
        logs_store: LogsStoreView {
            available: config.vlogs_binary.is_file(),
            enabled: settings.logs_enabled,
            running: logs_running,
            retention_days: settings.logs.days,
            last_error: status.logs_error,
        },
        disk,
        growth,
        limits: LimitsView {
            min_retention_days: settings::MIN_RETENTION_DAYS,
            max_retention_days: settings::MAX_RETENTION_DAYS,
            min_rollup_retention_days: settings::MIN_ROLLUP_RETENTION_DAYS,
            max_rollup_retention_days: settings::MAX_ROLLUP_RETENTION_DAYS,
            min_logs_retention_days: settings::MIN_LOGS_RETENTION_DAYS,
            max_logs_retention_days: settings::MAX_LOGS_RETENTION_DAYS,
        },
        restarted: Vec::new(),
    })
}

/// Ce qu'une modification demande ; un champ absent n'est pas touché.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub metrics_retention_days: Option<u32>,
    pub rollup_enabled: Option<bool>,
    pub rollup_retention_days: Option<u32>,
    pub logs_store_enabled: Option<bool>,
    pub logs_store_retention_days: Option<u32>,
}

/// Échec d'une modification : la faute de la demande, ou un processus qui ne
/// repart pas.
#[derive(Debug)]
pub enum UpdateError {
    Invalid(String),
    Failed(anyhow::Error),
}

impl From<anyhow::Error> for UpdateError {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}

/// Applique une modification et rend ce qui a été relancé, démarré ou arrêté.
pub async fn apply(state: &AppState, update: &Update) -> Result<Vec<&'static str>, UpdateError> {
    use settings::*;
    let _guard = state.storage().applying.lock().await;

    let current = load(&state.pool, &state.config).await?;
    let config = &state.config;
    let mut restarted = Vec::new();

    // Tout est vérifié avant d'écrire quoi que ce soit.
    if let Some(days) = update.metrics_retention_days {
        if !config.vm_embedded() && days != current.metrics.days {
            return Err(UpdateError::Invalid(
                "The metrics store is external (DUMBMONIT_VM_URL): set its retention there \
                 (-retentionPeriod)."
                    .into(),
            ));
        }
        check_days("raw metrics", days, MIN_RETENTION_DAYS, MAX_RETENTION_DAYS)
            .map_err(UpdateError::Invalid)?;
    }
    if let Some(days) = update.rollup_retention_days {
        check_days("long-term", days, MIN_ROLLUP_RETENTION_DAYS, MAX_ROLLUP_RETENTION_DAYS)
            .map_err(UpdateError::Invalid)?;
        if !config.vm_long_embedded() && days != current.rollup.days {
            return Err(UpdateError::Invalid(
                "The long-term store is external (DUMBMONIT_VM_LONG_URL): set its retention \
                 there."
                    .into(),
            ));
        }
    }
    if update.rollup_enabled == Some(true) && config.effective_long_url().is_none() {
        return Err(UpdateError::Invalid(
            "No long-term store is available: the metrics store is external. Set \
             DUMBMONIT_VM_LONG_URL to a second VictoriaMetrics."
                .into(),
        ));
    }
    if let Some(days) = update.logs_store_retention_days {
        check_days("log store", days, MIN_LOGS_RETENTION_DAYS, MAX_LOGS_RETENTION_DAYS)
            .map_err(UpdateError::Invalid)?;
    }
    if update.logs_store_enabled == Some(true) && !config.vlogs_binary.is_file() {
        return Err(UpdateError::Invalid(format!(
            "VictoriaLogs is not in this image ({} is missing).",
            config.vlogs_binary.display()
        )));
    }

    // Mesures brutes.
    if let Some(days) = update.metrics_retention_days.filter(|d| *d != current.metrics.days) {
        write(&state.pool, METRICS_RETENTION_KEY, &days.to_string()).await?;
        let vm = state.storage().processes.lock().await.metrics.clone();
        if let Some(vm) = vm {
            vm.restart_with(&format!("{days}d")).await?;
            restarted.push("metrics");
        }
        if let Some(long) = state.victoria.long_term() {
            long.set_raw_retention_secs(i64::from(days) * 86_400);
        }
    }

    // Historique long terme.
    if let Some(days) = update.rollup_retention_days.filter(|d| *d != current.rollup.days) {
        write(&state.pool, ROLLUP_RETENTION_KEY, &days.to_string()).await?;
        let vm = state.storage().processes.lock().await.long.clone();
        if let Some(vm) = vm {
            vm.restart_with(&format!("{days}d")).await?;
            restarted.push("rollup");
        }
    }
    match update.rollup_enabled {
        Some(enabled) if enabled != current.rollup_enabled => {
            write(&state.pool, ROLLUP_ENABLED_KEY, if enabled { "on" } else { "off" }).await?;
            if enabled {
                let fresh = load(&state.pool, &state.config).await?;
                enable_rollup(state, &fresh).await?;
            } else {
                disable_rollup(state).await;
            }
            if !restarted.contains(&"rollup") {
                restarted.push("rollup");
            }
        }
        _ => {}
    }

    // VictoriaLogs.
    let logs_enabled = update.logs_store_enabled.unwrap_or(current.logs_enabled);
    if let Some(days) = update.logs_store_retention_days.filter(|d| *d != current.logs.days) {
        write(&state.pool, LOGS_RETENTION_KEY, &days.to_string()).await?;
        let vm = state.storage().processes.lock().await.logs.clone();
        if let Some(vm) = vm.filter(|_| logs_enabled) {
            vm.restart_with(&format!("{days}d")).await?;
            restarted.push("logs_store");
        }
    }
    match update.logs_store_enabled {
        Some(enabled) if enabled != current.logs_enabled => {
            write(&state.pool, LOGS_ENABLED_KEY, if enabled { "on" } else { "off" }).await?;
            if enabled {
                let fresh = load(&state.pool, &state.config).await?;
                if let Err(error) = enable_logs(state, &fresh.logs.flag).await {
                    state.storage().update_status(|s| s.logs_error = Some(format!("{error:#}")));
                    return Err(error.into());
                }
            } else {
                disable_logs(state).await?;
            }
            if !restarted.contains(&"logs_store") {
                restarted.push("logs_store");
            }
        }
        _ => {}
    }
    Ok(restarted)
}

// --------------------------------------------------------------------------
// Disque
// --------------------------------------------------------------------------

/// Taille d'un répertoire (fichiers seulement, liens non suivis). `None` s'il
/// n'existe pas.
pub fn dir_size(path: &Path) -> Option<u64> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.is_file() {
        return Some(metadata.len());
    }
    if !metadata.is_dir() {
        return Some(0);
    }
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else { continue };
            if metadata.is_dir() {
                stack.push(entry.path());
            } else if metadata.is_file() {
                total += metadata.len();
            }
        }
    }
    Some(total)
}

/// Taille des bases SQLite de `dir` (journal WAL compris).
pub fn databases_size(dir: &Path) -> Option<u64> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut total = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if (name.ends_with(".db") || name.ends_with(".db-wal") || name.ends_with(".db-shm"))
            && let Ok(metadata) = entry.metadata()
            && metadata.is_file()
        {
            total += metadata.len();
        }
    }
    Some(total)
}

/// Espace libre et total du système de fichiers qui porte `path`.
#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // les types de `statvfs` varient d'une plateforme à l'autre
pub fn filesystem_space(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let raw = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY : `statvfs` remplit la structure fournie ; le chemin est une
    // chaîne C valide pour la durée de l'appel.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(raw.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    let block = stats.f_frsize as u64;
    Some((stats.f_bavail as u64 * block, stats.f_blocks as u64 * block))
}

#[cfg(not(unix))]
pub fn filesystem_space(_path: &Path) -> Option<(u64, u64)> {
    None
}

/// Les emplacements mesurés, dans l'ordre affiché.
fn usage_paths(state: &AppState) -> Vec<(&'static str, PathBuf, bool)> {
    let config = &state.config;
    let data = &config.data_dir;
    vec![
        ("metrics", config.vm_data_path(), false),
        ("rollup", config.vm_long_data_path(), false),
        ("logs_store", config.vlogs_data_path(), false),
        ("database", data.join("*.db"), true),
        ("logs", crate::logs::dir(data), false),
        ("backups", config.backup_path(), false),
    ]
}

/// Mesure de l'espace occupé : parcours de répertoires, hors du runtime.
pub async fn disk_view(state: &AppState) -> DiskView {
    let paths = usage_paths(state);
    let data_dir = state.config.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        let usage = paths
            .into_iter()
            .map(|(key, path, databases)| {
                let bytes = if databases {
                    path.parent().and_then(databases_size)
                } else {
                    dir_size(&path)
                };
                UsageView { key, path: path.display().to_string(), bytes }
            })
            .collect();
        let space = filesystem_space(&data_dir);
        DiskView {
            usage,
            free_bytes: space.map(|(free, _)| free),
            total_bytes: space.map(|(_, total)| total),
        }
    })
    .await
    .unwrap_or(DiskView { usage: Vec::new(), free_bytes: None, total_bytes: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_sizes_add_up_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), vec![0u8; 100]).unwrap();
        std::fs::create_dir_all(dir.path().join("x/y")).unwrap();
        std::fs::write(dir.path().join("x/y/b"), vec![0u8; 50]).unwrap();
        assert_eq!(dir_size(dir.path()), Some(150));
        assert_eq!(dir_size(&dir.path().join("absent")), None);
    }

    #[test]
    fn database_size_counts_sqlite_files_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("dumbmonit.db"), vec![0u8; 10]).unwrap();
        std::fs::write(dir.path().join("dumbmonit.db-wal"), vec![0u8; 5]).unwrap();
        std::fs::write(dir.path().join("secret.key"), vec![0u8; 32]).unwrap();
        assert_eq!(databases_size(dir.path()), Some(15));
    }

    #[cfg(unix)]
    #[test]
    fn the_filesystem_reports_its_free_space() {
        let (free, total) = filesystem_space(Path::new("/")).unwrap();
        assert!(total > 0 && free <= total);
    }
}
