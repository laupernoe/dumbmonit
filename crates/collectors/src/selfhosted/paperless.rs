//! Paperless-ngx.
//!
//! Trois lectures, avec un compte ordinaire (ni équipe, ni super-utilisateur)
//! qui ne porte que trois permissions de lecture :
//!
//! * `/api/remote_version/` — la version installée et celle publiée ; c'est la
//!   preuve de vie et d'authentification ;
//! * `/api/status/` — l'état de la base, de Redis, de Celery, de l'index de
//!   recherche, du classifieur et du contrôle d'intégrité, et le décompte des
//!   tâches des trente derniers jours. Paperless 2.x le réserve aux comptes
//!   d'équipe ; depuis la 3.0, la permission « System Monitoring » suffit ;
//! * `/api/tasks/` — les tâches en échec récentes, nommées.
//!
//! La panne que ce module existe pour voir : **Redis ou Celery tombé**. La
//! page s'ouvre, les documents se lisent, et plus rien de ce qui arrive dans
//! le dossier de consommation ou par courriel n'est importé.
//!
//! Paperless garde les documents et les tâches de chacun privés : ce compte
//! ne voit que les tâches sans propriétaire (les tâches planifiées) et les
//! documents qu'on lui a partagés. Le décompte des échecs de `/api/status/`,
//! lui, couvre toutes les tâches.

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target, TargetId};
use serde::Deserialize;
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::options::tag;
use super::{MAX_NAMED, flag, gauge, http_client, now_ms, number, settle, token};

pub const DEFAULT_PORT: u16 = 8000;

/// Fenêtre des tâches en échec comptées et nommées.
pub const DEFAULT_TASK_LOOKBACK_HOURS: u64 = 24;

#[derive(Debug, Deserialize)]
pub struct RemoteVersion {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub update_available: Option<bool>,
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    client: &HttpClient,
    target_id: TargetId,
    lookback: Duration,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let remote: RemoteVersion = client.get_json("/api/remote_version/").await?;
    let mut errors = 0u32;
    let (status, tasks, statistics) = futures::join!(
        client.get("/api/status/"),
        client.get_json::<Value>("/api/tasks/?page_size=100"),
        client.get_json::<Value>("/api/statistics/"),
    );

    let mut out = Vec::new();
    let mut version = None;
    match status {
        Ok((code, body)) if code.is_success() => {
            match client.decode::<Value>(&body, "/api/status/") {
                Ok(status) => {
                    version =
                        status.get("pngx_version").and_then(Value::as_str).map(str::to_string);
                    out.extend(status_samples(&status, ts_ms));
                }
                Err(error) => {
                    errors += 1;
                    tracing::warn!(target_id, %error, "état de Paperless illisible");
                }
            }
        }
        // Sans la permission « System Monitoring » (ou avant la 3.0, sans le
        // statut d'équipe) : la page d'état reste fermée, le reste se lit.
        Ok((code, _)) if code == reqwest::StatusCode::FORBIDDEN => {
            out.push(flag("paperless_status_readable", false, ts_ms));
        }
        Ok((code, body)) => {
            errors += 1;
            let error = client.status_error(code, &body, "/api/status/");
            tracing::warn!(target_id, %error, "état de Paperless indisponible");
        }
        Err(error) => {
            errors += 1;
            tracing::warn!(target_id, %error, "état de Paperless indisponible");
        }
    }
    out.extend(version_samples(&remote, version.as_deref(), ts_ms));
    if let Some(tasks) = settle(tasks, &mut errors, target_id, "/api/tasks/") {
        let since = chrono::Utc::now() - chrono::Duration::from_std(lookback).unwrap_or_default();
        out.extend(task_samples(&tasks, since, ts_ms));
    }
    if let Some(statistics) = settle(statistics, &mut errors, target_id, "/api/statistics/") {
        out.extend(statistics_samples(&statistics, ts_ms));
    }
    out.push(gauge("paperless_scrape_errors", f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ traduction

/// La version installée vient de `/api/status/` quand il est lisible
/// (`3.2.1`), sinon de `/api/remote_version/`, qui donne la dernière publiée.
pub fn version_samples(remote: &RemoteVersion, installed: Option<&str>, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(version) = installed {
        out.push(gauge("paperless_version_info", 1.0, ts_ms).with_label("version", version));
    }
    // `update_available` est calculé par Paperless lui-même ; la version
    // publiée vaut `0.0.0` quand la vérification est coupée ou a échoué.
    let latest =
        remote.version.as_deref().filter(|v| !v.trim_start_matches('v').starts_with("0.0"));
    if let (Some(available), Some(latest)) = (remote.update_available, latest) {
        out.push(
            flag("paperless_update_available", available, ts_ms)
                .with_label("latest_version", latest.trim_start_matches('v')),
        );
    }
    out
}

/// `OK` 0, `WARNING` 1, `ERROR` 2. `DISABLED` (l'index LLM coupé) : pas de série.
fn component_status(value: Option<&Value>) -> Option<f64> {
    match value?.as_str()?.to_ascii_uppercase().as_str() {
        "OK" => Some(0.0),
        "WARNING" => Some(1.0),
        "ERROR" => Some(2.0),
        _ => None,
    }
}

pub fn status_samples(status: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![flag("paperless_status_readable", true, ts_ms)];
    let components = [
        ("database", "/database/status"),
        ("redis", "/tasks/redis_status"),
        ("celery", "/tasks/celery_status"),
        ("index", "/tasks/index_status"),
        ("classifier", "/tasks/classifier_status"),
        ("sanity_check", "/tasks/sanity_check_status"),
        ("llm_index", "/tasks/llmindex_status"),
    ];
    for (component, pointer) in components {
        if let Some(value) = component_status(status.pointer(pointer)) {
            out.push(
                gauge("paperless_component_status", value, ts_ms)
                    .with_label("component", component),
            );
        }
    }
    if let Some(Value::Array(unapplied)) =
        status.pointer("/database/migration_status/unapplied_migrations")
    {
        out.push(gauge("paperless_unapplied_migrations", unapplied.len() as f64, ts_ms));
    }
    let total = number(status.pointer("/storage/total"));
    let available = number(status.pointer("/storage/available"));
    if let Some(available) = available {
        out.push(gauge("paperless_storage_available_bytes", available, ts_ms));
    }
    if let (Some(total), Some(available)) = (total, available)
        && total > 0.0
    {
        out.push(gauge("paperless_storage_total_bytes", total, ts_ms));
        out.push(gauge(
            "paperless_storage_used_percent",
            (total - available) / total * 100.0,
            ts_ms,
        ));
    }
    // Décompte sur la fenêtre de Paperless (trente jours), toutes tâches confondues.
    for (name, key) in [
        ("paperless_tasks_failed_window", "failure_count"),
        ("paperless_tasks_pending", "pending_count"),
        ("paperless_tasks_window", "total_count"),
    ] {
        if let Some(value) = number(status.pointer(&format!("/tasks/summary/{key}"))) {
            out.push(gauge(name, value, ts_ms));
        }
    }
    out
}

/// Une tâche, telle que 3.x (`task_type`, `status: "failure"`, `input_data`)
/// ou 2.x (`task_name`, `status: "FAILURE"`, `task_file_name`) la décrit.
struct Task<'a> {
    failed: bool,
    acknowledged: bool,
    done: Option<chrono::DateTime<chrono::Utc>>,
    kind: &'a str,
    file: Option<&'a str>,
}

fn read_task(task: &Value) -> Task<'_> {
    let text = |key: &str| task.get(key).and_then(Value::as_str);
    let failed = text("status").is_some_and(|s| s.eq_ignore_ascii_case("failure"));
    let done = text("date_done")
        .or_else(|| text("date_created"))
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
        .map(|date| date.with_timezone(&chrono::Utc));
    let kind = text("task_type").or_else(|| text("task_name")).or_else(|| text("type"));
    let file = task
        .pointer("/input_data/filename")
        .and_then(Value::as_str)
        .or_else(|| text("task_file_name"));
    Task {
        failed,
        acknowledged: task.get("acknowledged").and_then(Value::as_bool).unwrap_or(false),
        done,
        kind: kind.unwrap_or("task"),
        file,
    }
}

/// Tâches en échec depuis `since`, non acquittées dans Paperless.
pub fn task_samples(
    tasks: &Value,
    since: chrono::DateTime<chrono::Utc>,
    ts_ms: i64,
) -> Vec<Sample> {
    // 3.x pagine (`results`), 2.x rend une liste.
    let list = tasks.get("results").unwrap_or(tasks).as_array().map(Vec::as_slice).unwrap_or(&[]);
    let failed: Vec<Task> = list
        .iter()
        .map(read_task)
        .filter(|task| task.failed && !task.acknowledged)
        .filter(|task| task.done.is_none_or(|done| done >= since))
        .collect();
    let mut out = vec![gauge("paperless_tasks_failed_recent", failed.len() as f64, ts_ms)];
    for task in failed.iter().take(MAX_NAMED) {
        out.push(
            gauge("paperless_task_failed", 1.0, ts_ms)
                .with_label("task", task.kind)
                .with_label("file", task.file.unwrap_or("")),
        );
    }
    out
}

pub fn statistics_samples(statistics: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(total) = number(statistics.get("documents_total")) {
        out.push(gauge("paperless_documents", total, ts_ms));
    }
    if let Some(inbox) = number(statistics.get("documents_inbox")) {
        out.push(gauge("paperless_documents_inbox", inbox, ts_ms));
    }
    out
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct PaperlessCollector;

impl PaperlessCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let auth = match (&target.credential, token(&target.credential)) {
            (_, Some(token)) => Auth::header("Authorization", format!("Token {token}")),
            (Credential::UsernamePassword { username, password }, _)
                if !username.trim().is_empty() =>
            {
                Auth::basic(username.trim(), password)
            }
            (other, _) => {
                return Err(ProbeError::Config(format!(
                    "Paperless-ngx expects an API token or a user name and password, configured: {other}"
                )));
            }
        };
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/api").unwrap_or(trimmed).to_string();
        http_client(&target, "http", DEFAULT_PORT, auth, "Paperless-ngx")
    }

    fn lookback(target: &Target) -> Result<Duration, ProbeError> {
        let Some(raw) = tag(target, "task_lookback_hours") else {
            return Ok(Duration::from_secs(DEFAULT_TASK_LOOKBACK_HOURS * 3600));
        };
        match raw.parse::<u64>() {
            Ok(hours) if (1..=720).contains(&hours) => Ok(Duration::from_secs(hours * 3600)),
            _ => Err(ProbeError::Config(format!(
                "Failed task window must be between 1 and 720 hours, got \"{raw}\""
            ))),
        }
    }
}

#[async_trait]
impl Collector for PaperlessCollector {
    fn kind(&self) -> &'static str {
        "paperless"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let lookback = Self::lookback(target)?;
        probe(&Self::client(target)?, target.id, lookback, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: RemoteVersion = Self::client(target)?.get_json("/api/remote_version/").await?;
        Ok(Some("paperless".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;

    use super::*;

    // Réponses réelles d'un Paperless-ngx 3.2.1 (image officielle, SQLite,
    // Valkey), lues avec un compte ordinaire portant `view_document`,
    // `view_paperlesstask` et `view_system_monitoring`. La liste des tâches
    // est celle du propriétaire des documents, pour qu'une importation en
    // échec y figure (un PDF volontairement corrompu) ; « Redis arrêté » a été
    // capturé en arrêtant le conteneur Redis.
    const STATUS: &str = include_str!("testdata/paperless_3.2.1/status.json");
    const STATUS_REDIS_DOWN: &str = include_str!("testdata/paperless_3.2.1/status_redis_down.json");
    const STATUS_FORBIDDEN: &str = include_str!("testdata/paperless_3.2.1/status_forbidden.txt");
    const REMOTE: &str = include_str!("testdata/paperless_3.2.1/remote_version.json");
    const TASKS: &str = include_str!("testdata/paperless_3.2.1/tasks.json");
    const STATISTICS: &str = include_str!("testdata/paperless_3.2.1/statistics.json");

    fn json(body: &str) -> Value {
        serde_json::from_str(body).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn component(samples: &[Sample], name: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == "paperless_component_status" && s.labels["component"] == name)
            .map(|s| s.value)
    }

    #[test]
    fn l_etat_dit_chaque_composant() {
        let samples = status_samples(&json(STATUS), 0);
        assert_eq!(component(&samples, "database"), Some(0.0));
        assert_eq!(component(&samples, "redis"), Some(0.0));
        assert_eq!(component(&samples, "celery"), Some(0.0));
        assert_eq!(component(&samples, "sanity_check"), Some(1.0));
        assert_eq!(component(&samples, "llm_index"), None, "coupé : pas de série");
        assert_eq!(value(&samples, "paperless_unapplied_migrations"), Some(0.0));
        assert_eq!(value(&samples, "paperless_tasks_failed_window"), Some(1.0));
        let used = value(&samples, "paperless_storage_used_percent").unwrap();
        assert!(used > 0.0 && used < 100.0);

        let down = status_samples(&json(STATUS_REDIS_DOWN), 0);
        assert_eq!(component(&down, "redis"), Some(2.0));
        assert_eq!(component(&down, "celery"), Some(2.0));
    }

    #[test]
    fn la_version_et_la_mise_a_jour() {
        let remote: RemoteVersion = serde_json::from_str(REMOTE).unwrap();
        let samples = version_samples(&remote, Some("3.2.1"), 0);
        assert_eq!(value(&samples, "paperless_update_available"), Some(0.0));
        let info = samples.iter().find(|s| s.metric == "paperless_version_info").unwrap();
        assert_eq!(info.labels["version"], "3.2.1");
        // Vérification coupée ou en échec : Paperless répond `0.0.0`.
        let off = RemoteVersion { version: Some("0.0.0".into()), update_available: Some(false) };
        assert!(value(&version_samples(&off, None, 0), "paperless_update_available").is_none());
    }

    #[test]
    fn les_taches_en_echec_recentes_sont_nommees() {
        let tasks = json(TASKS);
        let since = chrono::DateTime::parse_from_rfc3339("2026-09-30T00:00:00Z").unwrap().into();
        let samples = task_samples(&tasks, since, 0);
        assert_eq!(value(&samples, "paperless_tasks_failed_recent"), Some(1.0));
        let failed = samples.iter().find(|s| s.metric == "paperless_task_failed").unwrap();
        assert_eq!(failed.labels["task"], "consume_file");
        assert_eq!(failed.labels["file"], "bad.pdf");
        let later = chrono::DateTime::parse_from_rfc3339("2026-10-02T00:00:00Z").unwrap().into();
        assert_eq!(
            value(&task_samples(&tasks, later, 0), "paperless_tasks_failed_recent"),
            Some(0.0)
        );
    }

    #[test]
    fn les_taches_au_format_2x_se_lisent_aussi() {
        // Forme documentée de `/api/tasks/` en 2.x : une liste, statut Celery en capitales.
        let tasks = serde_json::json!([
            {"id": 3, "task_id": "x", "task_file_name": "scan.pdf", "date_created": "2026-09-30T06:00:00Z",
             "date_done": "2026-09-30T06:00:05Z", "type": "file", "status": "FAILURE",
             "result": "scan.pdf: Not consuming scan.pdf: It is a duplicate", "acknowledged": false,
             "related_document": null, "owner": null, "task_name": "consume_file"},
            {"id": 4, "status": "FAILURE", "acknowledged": true, "task_name": "consume_file",
             "date_done": "2026-09-30T06:00:05Z"}
        ]);
        let since = chrono::DateTime::parse_from_rfc3339("2026-09-29T00:00:00Z").unwrap().into();
        let samples = task_samples(&tasks, since, 0);
        assert_eq!(value(&samples, "paperless_tasks_failed_recent"), Some(1.0));
        let failed = samples.iter().find(|s| s.metric == "paperless_task_failed").unwrap();
        assert_eq!(failed.labels["file"], "scan.pdf");
    }

    #[test]
    fn les_statistiques_comptent_les_documents_visibles() {
        let samples = statistics_samples(&json(STATISTICS), 0);
        assert!(value(&samples, "paperless_documents").unwrap() > 0.0);
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 9,
            name: "paperless".into(),
            address,
            kind: "paperless".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential,
        }
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Token abc123")
    }

    #[tokio::test]
    async fn la_sonde_complete_avec_redis_arrete() {
        let app = Router::new()
            .route(
                "/api/remote_version/",
                get(|headers: HeaderMap| async move {
                    if !authorized(&headers) {
                        return (StatusCode::UNAUTHORIZED, r#"{"detail":"Invalid token."}"#);
                    }
                    (StatusCode::OK, REMOTE)
                }),
            )
            .route("/api/status/", get(|| async { STATUS_REDIS_DOWN }))
            .route("/api/tasks/", get(|| async { TASKS }))
            .route("/api/statistics/", get(|| async { STATISTICS }));
        let base = serve(app).await;
        let token = Credential::ApiToken { token: "abc123".into() };
        let samples =
            PaperlessCollector::new().probe(&target(format!("{base}/api/"), token)).await.unwrap();
        assert_eq!(component(&samples, "redis"), Some(2.0));
        assert_eq!(value(&samples, "paperless_scrape_errors"), Some(0.0));
        let info = samples.iter().find(|s| s.metric == "paperless_version_info").unwrap();
        assert_eq!(info.labels["version"], "3.2.1");

        let wrong = Credential::ApiToken { token: "faux".into() };
        let error = PaperlessCollector::new().probe(&target(base, wrong)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn sans_permission_de_supervision_le_reste_se_lit() {
        let app = Router::new()
            .route("/api/remote_version/", get(|| async { REMOTE }))
            .route("/api/status/", get(|| async { (StatusCode::FORBIDDEN, STATUS_FORBIDDEN) }))
            .route("/api/tasks/", get(|| async { TASKS }))
            .route("/api/statistics/", get(|| async { STATISTICS }));
        let base = serve(app).await;
        let login =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "p".into() };
        let samples = PaperlessCollector::new().probe(&target(base, login)).await.unwrap();
        assert_eq!(value(&samples, "paperless_status_readable"), Some(0.0));
        assert_eq!(value(&samples, "paperless_scrape_errors"), Some(0.0));
        assert!(value(&samples, "paperless_documents").is_some());
    }

    #[test]
    fn la_fenetre_des_taches_est_bornee() {
        let mut t = target("paperless.lan".into(), Credential::None);
        assert_eq!(
            PaperlessCollector::lookback(&t).unwrap(),
            Duration::from_secs(DEFAULT_TASK_LOOKBACK_HOURS * 3600)
        );
        t.tags.insert("task_lookback_hours".into(), "0".into());
        assert!(PaperlessCollector::lookback(&t).is_err());
        assert!(matches!(PaperlessCollector::client(&t), Err(ProbeError::Config(_))));
    }
}
