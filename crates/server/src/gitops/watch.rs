//! Dossier surveillé (`DUMBMONIT_CONFIG_DIR`).
//!
//! Tous les fichiers `.yaml` / `.yml` du dossier forment un seul document
//! (sections concaténées), appliqué avec `prune` au démarrage puis à chaque
//! changement de contenu. Une relecture toutes les cinq secondes plutôt qu'une
//! notification du système de fichiers : elle fonctionne aussi sur un volume
//! réseau, un montage Kubernetes (liens symboliques qui basculent) ou un dépôt
//! synchronisé, là où les événements se perdent.
//!
//! Un dossier sans aucun fichier n'applique rien : un montage manquant ne doit
//! pas faire supprimer tout ce que le fichier gérait.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::{ConfigDoc, apply, refs};
use crate::state::AppState;

const POLL: Duration = Duration::from_secs(5);
/// Un échec identique n'est retenté qu'après ce délai (ou si les fichiers changent).
const RETRY: Duration = Duration::from_secs(600);

/// Dernier passage du dossier surveillé.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WatchStatus {
    pub directory: Option<String>,
    pub files: Vec<String>,
    /// Date (UTC) de la dernière application réussie.
    pub last_applied_at: Option<String>,
    pub last_error: Option<String>,
    pub last_changes: Option<(usize, usize, usize)>,
}

static STATUS: LazyLock<Mutex<WatchStatus>> = LazyLock::new(|| Mutex::new(WatchStatus::default()));

pub fn status() -> WatchStatus {
    STATUS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

fn update(f: impl FnOnce(&mut WatchStatus)) {
    f(&mut STATUS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
}

/// Lance la surveillance, si un dossier est configuré.
pub fn spawn(state: AppState) {
    let Some(dir) = state.config.config_dir.clone() else { return };
    update(|status| status.directory = Some(dir.display().to_string()));
    tracing::info!(dir = %dir.display(), "configuration directory watched");
    tokio::spawn(async move {
        let mut last: Option<u64> = None;
        let mut failed_at: Option<Instant> = None;
        loop {
            let (files, signature) = match read_dir(&dir).await {
                Ok(read) => read,
                Err(error) => {
                    update(|status| status.last_error = Some(error));
                    tokio::time::sleep(POLL).await;
                    continue;
                }
            };
            let retry = failed_at.is_some_and(|at| at.elapsed() >= RETRY);
            if !files.is_empty() && (last != Some(signature) || retry) {
                last = Some(signature);
                match run(&state, &dir).await {
                    Ok(counts) => {
                        failed_at = None;
                        update(|status| {
                            status.files = files.clone();
                            status.last_applied_at =
                                Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
                            status.last_error = None;
                            status.last_changes = Some(counts);
                        });
                    }
                    Err(error) => {
                        tracing::warn!(%error, "configuration directory not applied");
                        failed_at = Some(Instant::now());
                        update(|status| {
                            status.files = files.clone();
                            status.last_error = Some(error);
                        });
                    }
                }
            } else if files.is_empty() {
                update(|status| {
                    status.files.clear();
                    status.last_error =
                        Some("No .yaml file in the configuration directory.".into());
                });
            }
            tokio::time::sleep(POLL).await;
        }
    });
}

/// Les fichiers YAML du dossier (triés) et une empreinte de leur contenu.
async fn read_dir(dir: &Path) -> Result<(Vec<String>, u64), String> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|error| format!("The configuration directory cannot be read: {error}"))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"));
        // `metadata` suit les liens symboliques (montages Kubernetes).
        let is_file = tokio::fs::metadata(&path).await.is_ok_and(|meta| meta.is_file());
        if yaml && is_file {
            paths.push(path);
        }
    }
    paths.sort();
    let mut hasher = DefaultHasher::new();
    let mut names = Vec::new();
    for path in &paths {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let content = tokio::fs::read(path)
            .await
            .map_err(|error| format!("{name} cannot be read: {error}"))?;
        name.hash(&mut hasher);
        content.hash(&mut hasher);
        names.push(name);
    }
    Ok((names, hasher.finish()))
}

/// Lit, résout et applique le dossier.
async fn run(state: &AppState, dir: &Path) -> Result<(usize, usize, usize), String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| e.to_string())?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"));
        if yaml {
            paths.push(path);
        }
    }
    paths.sort();

    let mut docs = Vec::new();
    for path in &paths {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let text = tokio::fs::read_to_string(path)
            .await
            .map_err(|error| format!("{name} cannot be read: {error}"))?;
        docs.push(ConfigDoc::parse_yaml(&text).map_err(|error| format!("{name}: {error}"))?);
    }
    let mut doc = ConfigDoc::merge(docs)?;
    refs::resolve_doc(&mut doc, Some(dir))?;

    match apply::apply(state, doc, true, "config-file", None).await {
        Ok(plan) => Ok((plan.create, plan.update, plan.delete)),
        Err(apply::ApplyError::Invalid(message)) => Err(message),
        Err(apply::ApplyError::Internal(error)) => Err(format!("internal error: {error}")),
    }
}
