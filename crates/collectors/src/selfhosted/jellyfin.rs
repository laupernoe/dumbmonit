//! Jellyfin.
//!
//! Tout passe par l'API, avec une clé d'API créée dans le tableau de bord.
//! Jellyfin ne sait pas restreindre une clé : chacune porte tous les droits
//! sur le serveur. C'est pourtant la seule façon de lire les tâches
//! planifiées et les lectures de tous les utilisateurs ; un compte ordinaire
//! ne voit que ses propres sessions et aucune tâche.
//!
//! La panne que ce module existe pour voir : **le scan de bibliothèque qui
//! échoue chaque nuit** (un partage réseau démonté, un disque plein). Les
//! nouveaux films n'apparaissent plus, et rien ne le dit sinon une ligne dans
//! les tâches planifiées.
//!
//! Jellyfin ne vérifie pas ses propres mises à jour (c'est l'affaire du
//! paquet ou de l'image) : seul un redémarrage en attente, après une mise à
//! jour d'extension, est signalé.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target, TargetId};
use serde::Deserialize;

use super::client::{Auth, HttpClient};
use super::{MAX_NAMED, flag, gauge, http_client, now_ms, settle, token};

pub const DEFAULT_PORT: u16 = 8096;

/// La tâche du scan de bibliothèque, par sa clé (stable, non traduite).
const LIBRARY_SCAN: &str = "RefreshLibrary";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub has_pending_restart: bool,
    #[serde(default)]
    pub is_shutting_down: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ScheduledTask {
    pub name: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub last_execution_result: Option<TaskResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaskResult {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub end_time_utc: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Session {
    #[serde(default)]
    pub now_playing_item: Option<serde_json::Value>,
    #[serde(default)]
    pub play_state: Option<PlayState>,
    #[serde(default)]
    pub transcoding_info: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayState {
    #[serde(default)]
    pub play_method: Option<String>,
    #[serde(default)]
    pub is_paused: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Plugin {
    pub name: String,
    #[serde(default)]
    pub status: Option<String>,
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    client: &HttpClient,
    target_id: TargetId,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let info: SystemInfo = client.get_json("/System/Info").await?;
    let mut out = system_samples(&info, ts_ms);
    let mut errors = 0u32;
    let (tasks, sessions, plugins) = futures::join!(
        client.get_json::<Vec<ScheduledTask>>("/ScheduledTasks"),
        client.get_json::<Vec<Session>>("/Sessions"),
        client.get_json::<Vec<Plugin>>("/Plugins"),
    );
    if let Some(tasks) = settle(tasks, &mut errors, target_id, "/ScheduledTasks") {
        out.extend(task_samples(&tasks, chrono::Utc::now(), ts_ms));
    }
    if let Some(sessions) = settle(sessions, &mut errors, target_id, "/Sessions") {
        out.extend(session_samples(&sessions, ts_ms));
    }
    if let Some(plugins) = settle(plugins, &mut errors, target_id, "/Plugins") {
        out.extend(plugin_samples(&plugins, ts_ms));
    }
    out.push(gauge("jellyfin_scrape_errors", f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ traduction

pub fn system_samples(info: &SystemInfo, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        flag("jellyfin_pending_restart", info.has_pending_restart, ts_ms),
        flag("jellyfin_shutting_down", info.is_shutting_down, ts_ms),
    ];
    if let Some(version) = &info.version {
        out.push(gauge("jellyfin_version_info", 1.0, ts_ms).with_label("version", version));
    }
    out
}

/// `Failed` : une exception ; `Aborted` : le serveur s'est arrêté pendant la
/// tâche. `Cancelled` est un geste de l'utilisateur, pas une panne.
fn failed(result: &TaskResult) -> bool {
    matches!(result.status.as_deref(), Some("Failed" | "Aborted"))
}

pub fn task_samples(
    tasks: &[ScheduledTask],
    now: chrono::DateTime<chrono::Utc>,
    ts_ms: i64,
) -> Vec<Sample> {
    let failures: Vec<&ScheduledTask> = tasks
        .iter()
        .filter(|task| task.last_execution_result.as_ref().is_some_and(failed))
        .collect();
    let running = tasks.iter().filter(|t| t.state.as_deref() == Some("Running")).count();
    let mut out = vec![
        gauge("jellyfin_scheduled_tasks_failed", failures.len() as f64, ts_ms),
        gauge("jellyfin_scheduled_tasks_running", running as f64, ts_ms),
    ];
    for task in failures.iter().take(MAX_NAMED) {
        let message = task
            .last_execution_result
            .as_ref()
            .and_then(|r| r.error_message.as_deref())
            .unwrap_or("");
        // Le message complet est une trace .NET : sa première ligne suffit.
        let message: String = message.lines().next().unwrap_or("").chars().take(160).collect();
        out.push(
            gauge("jellyfin_scheduled_task_failed", 1.0, ts_ms)
                .with_label("task", task.name.as_str())
                .with_label("error", message),
        );
    }
    if let Some(scan) = tasks.iter().find(|t| t.key.as_deref() == Some(LIBRARY_SCAN))
        && let Some(result) = &scan.last_execution_result
    {
        out.push(flag("jellyfin_library_scan_last_ok", !failed(result), ts_ms));
        let ended = result
            .end_time_utc
            .as_deref()
            .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok());
        if let Some(ended) = ended {
            let age = (now - ended.with_timezone(&chrono::Utc)).num_seconds().max(0);
            out.push(gauge("jellyfin_library_scan_age_seconds", age as f64, ts_ms));
        }
    }
    out
}

pub fn session_samples(sessions: &[Session], ts_ms: i64) -> Vec<Sample> {
    let playing: Vec<&Session> = sessions.iter().filter(|s| s.now_playing_item.is_some()).collect();
    let transcoding = playing
        .iter()
        .filter(|s| {
            s.play_state.as_ref().and_then(|p| p.play_method.as_deref()) == Some("Transcode")
                || s.transcoding_info.as_ref().is_some_and(|t| {
                    t.get("IsVideoDirect").and_then(serde_json::Value::as_bool) == Some(false)
                })
        })
        .count();
    let paused =
        playing.iter().filter(|s| s.play_state.as_ref().is_some_and(|p| p.is_paused)).count();
    vec![
        gauge("jellyfin_sessions", sessions.len() as f64, ts_ms),
        gauge("jellyfin_streams", playing.len() as f64, ts_ms),
        gauge("jellyfin_streams_paused", paused as f64, ts_ms),
        gauge("jellyfin_transcodes", transcoding as f64, ts_ms),
    ]
}

/// Une extension qui ne charge plus (`Malfunctioned`), ou plus compatible
/// avec cette version (`NotSupported`) : ce qu'elle fournissait a disparu.
pub fn plugin_samples(plugins: &[Plugin], ts_ms: i64) -> Vec<Sample> {
    let broken: Vec<&Plugin> = plugins
        .iter()
        .filter(|p| matches!(p.status.as_deref(), Some("Malfunctioned" | "NotSupported")))
        .collect();
    let mut out = vec![
        gauge("jellyfin_plugins", plugins.len() as f64, ts_ms),
        gauge("jellyfin_plugins_broken", broken.len() as f64, ts_ms),
    ];
    for plugin in broken.iter().take(MAX_NAMED) {
        out.push(
            gauge("jellyfin_plugin_broken", 1.0, ts_ms)
                .with_label("plugin", plugin.name.as_str())
                .with_label("status", plugin.status.as_deref().unwrap_or("")),
        );
    }
    out
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct JellyfinCollector;

impl JellyfinCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(key) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "Jellyfin expects an API key, configured: {}",
                target.credential
            )));
        };
        // L'en-tête `Authorization: MediaBrowser`, le seul que Jellyfin 10.11+
        // accepte par défaut (`X-Emby-Token` et `api_key` sont des héritages
        // désactivés).
        let auth = Auth::header(
            "Authorization",
            format!(
                "MediaBrowser Client=\"DumbMonit\", Device=\"DumbMonit\", DeviceId=\"dumbmonit\", \
                 Version=\"{}\", Token=\"{key}\"",
                env!("CARGO_PKG_VERSION")
            ),
        );
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/web").unwrap_or(trimmed).to_string();
        http_client(&target, "http", DEFAULT_PORT, auth, "Jellyfin")
    }
}

#[async_trait]
impl Collector for JellyfinCollector {
    fn kind(&self) -> &'static str {
        "jellyfin"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, target.id, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: SystemInfo = Self::client(target)?.get_json("/System/Info").await?;
        Ok(Some("jellyfin".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;

    // Réponses réelles d'un Jellyfin 12.1.0 (image officielle), lues avec une
    // clé d'API. Un film de test a été lu en transcodage depuis un client web
    // simulé ; la tâche « Clean Transcode Directory » a échoué pour de vrai
    // (son dossier remplacé par un fichier) ; l'extension Bookshelf venait
    // d'être installée, d'où le redémarrage en attente. Identifiants tronqués,
    // adresses et noms remplacés ; la liste des versions de l'extension
    // installée a été raccourcie.
    const INFO: &str = include_str!("testdata/jellyfin_12.1.0/system_info.json");
    const TASKS: &str = include_str!("testdata/jellyfin_12.1.0/scheduled_tasks.json");
    const SESSIONS: &str = include_str!("testdata/jellyfin_12.1.0/sessions.json");
    const PLUGINS: &str = include_str!("testdata/jellyfin_12.1.0/plugins.json");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn le_redemarrage_en_attente_se_voit() {
        let samples = system_samples(&serde_json::from_str(INFO).unwrap(), 0);
        assert_eq!(value(&samples, "jellyfin_pending_restart"), Some(1.0));
        assert_eq!(value(&samples, "jellyfin_shutting_down"), Some(0.0));
        let version = samples.iter().find(|s| s.metric == "jellyfin_version_info").unwrap();
        assert_eq!(version.labels["version"], "12.1.0");
    }

    #[test]
    fn une_tache_en_echec_est_nommee_et_le_scan_date() {
        let tasks: Vec<ScheduledTask> = serde_json::from_str(TASKS).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-09-30T08:03:28Z").unwrap().into();
        let samples = task_samples(&tasks, now, 0);
        assert_eq!(value(&samples, "jellyfin_scheduled_tasks_failed"), Some(1.0));
        let failed = samples.iter().find(|s| s.metric == "jellyfin_scheduled_task_failed").unwrap();
        assert_eq!(failed.labels["task"], "Clean Transcode Directory");
        assert_eq!(failed.labels["error"], "The file '/cache/transcodes' already exists.");
        assert_eq!(value(&samples, "jellyfin_library_scan_last_ok"), Some(1.0));
        let age = value(&samples, "jellyfin_library_scan_age_seconds").unwrap();
        assert!((3500.0..3700.0).contains(&age), "{age}");
    }

    #[test]
    fn une_lecture_transcodee_est_comptee() {
        let sessions: Vec<Session> = serde_json::from_str(SESSIONS).unwrap();
        let samples = session_samples(&sessions, 0);
        assert_eq!(value(&samples, "jellyfin_sessions"), Some(3.0));
        assert_eq!(value(&samples, "jellyfin_streams"), Some(1.0));
        assert_eq!(value(&samples, "jellyfin_transcodes"), Some(1.0));
        assert_eq!(value(&samples, "jellyfin_streams_paused"), Some(0.0));
    }

    #[test]
    fn les_extensions_cassees_sont_nommees() {
        let plugins: Vec<Plugin> = serde_json::from_str(PLUGINS).unwrap();
        let samples = plugin_samples(&plugins, 0);
        assert_eq!(value(&samples, "jellyfin_plugins_broken"), Some(0.0));
        assert!(value(&samples, "jellyfin_plugins").unwrap() >= 7.0);
        let broken: Vec<Plugin> = serde_json::from_str(
            r#"[{"Name":"Trakt","Version":"26.0.0.0","Status":"Malfunctioned"}]"#,
        )
        .unwrap();
        let samples = plugin_samples(&broken, 0);
        let plugin = samples.iter().find(|s| s.metric == "jellyfin_plugin_broken").unwrap();
        assert_eq!(plugin.labels["plugin"], "Trakt");
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, key: &str) -> Target {
        Target {
            id: 11,
            name: "media".into(),
            address,
            kind: "jellyfin".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Credential::ApiToken { token: key.into() },
            group_name: String::new(),
            position: 0,
        }
    }

    #[tokio::test]
    async fn la_sonde_complete_presente_la_cle_en_mediabrowser() {
        let app = Router::new()
            .route(
                "/System/Info",
                get(|headers: HeaderMap| async move {
                    let auth = headers.get("authorization").and_then(|v| v.to_str().ok());
                    if !auth.is_some_and(|a| {
                        a.starts_with("MediaBrowser ") && a.contains("Token=\"k3y\"")
                    }) {
                        return (StatusCode::UNAUTHORIZED, "");
                    }
                    (StatusCode::OK, INFO)
                }),
            )
            .route("/ScheduledTasks", get(|| async { TASKS }))
            .route("/Sessions", get(|| async { SESSIONS }))
            .route("/Plugins", get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "boom") }));
        let base = serve(app).await;
        let samples =
            JellyfinCollector::new().probe(&target(format!("{base}/web/"), "k3y")).await.unwrap();
        assert_eq!(value(&samples, "jellyfin_transcodes"), Some(1.0));
        assert_eq!(value(&samples, "jellyfin_scheduled_tasks_failed"), Some(1.0));
        assert_eq!(value(&samples, "jellyfin_scrape_errors"), Some(1.0), "/Plugins en erreur");

        let error = JellyfinCollector::new().probe(&target(base, "autre")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }
}
