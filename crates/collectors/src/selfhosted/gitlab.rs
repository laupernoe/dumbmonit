//! GitLab (self-managed).
//!
//! Un jeton d'accès personnel (`PRIVATE-TOKEN`) lit, dans l'ordre :
//!
//! * `/api/v4/version` — la preuve de vie et d'authentification : tout jeton
//!   valide la lit, et c'est le seul appel qui doit réussir ;
//! * `/-/readiness?all=1` — la santé détaillée (base, cache, files d'attente,
//!   état partagé, Gitaly). GitLab répond 404 sans l'adresse de DumbMonit dans
//!   la liste blanche de supervision (`monitoring_whitelist`) ni de compte
//!   administrateur, et 503 — avec le détail dans le corps — quand un
//!   composant est en échec : seul le premier cas compte comme une erreur ;
//! * `/api/v4/sidekiq/queue_metrics` — le retard de chaque file Sidekiq.
//!   Sans Sidekiq, plus aucune notification, aucun webhook, aucun import ne
//!   part ;
//! * `/api/v4/runners/all` — les exécuteurs CI, en ligne ou non ;
//! * `/api/v4/admin/migrations/pending` — les migrations de base de données
//!   posées par une mise à jour, pas encore exécutées ;
//! * `/api/v4/application/statistics` — les décomptes de l'instance ;
//! * `/api/v4/application/settings` — deux réglages de sécurité : l'auto-
//!   inscription et l'obligation du second facteur ;
//! * `/api/v4/license` — plan et expiration de la licence (EE uniquement) ;
//! * `/api/v4/projects/<id>/pipelines`, pour les projets listés dans
//!   l'étiquette `watched_projects` : pipelines en échec ou en cours.
//!
//! Toutes ces lectures, hors la première, demandent un compte administrateur :
//! GitLab ne connaît pas de délégation plus fine pour elles. Un jeton
//! ordinaire ne voit que la version ; le reste est compté dans
//! `gitlab_scrape_errors` sans faire échouer la sonde.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dumbmonit_proto::{Collector, ProbeError, Sample, Target, TargetId};
use serde::Deserialize;
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::options::tag;
use super::{MAX_NAMED, flag, gauge, http_client, now_ms, number, settle, token};

pub const DEFAULT_PORT: u16 = 443;

/// Fenêtre par défaut des pipelines « récents » comptés en échec.
pub const DEFAULT_PIPELINE_LOOKBACK_HOURS: u64 = 24;

#[derive(Debug, Deserialize)]
pub struct Version {
    pub version: String,
    #[serde(default)]
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QueueMetrics {
    #[serde(default)]
    queues: HashMap<String, QueueMetric>,
}

#[derive(Debug, Deserialize)]
pub struct QueueMetric {
    #[serde(default)]
    backlog: f64,
    #[serde(default)]
    latency: f64,
}

#[derive(Debug, Deserialize)]
pub struct Runner {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct PendingMigrations {
    #[serde(default)]
    pending_migrations: Vec<PendingMigration>,
}

#[derive(Debug, Deserialize)]
pub struct PendingMigration {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct License {
    #[serde(default)]
    plan: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(default)]
    expired: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Pipeline {
    #[serde(default)]
    status: String,
    #[serde(default)]
    updated_at: Option<String>,
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    client: &HttpClient,
    target_id: TargetId,
    projects: &[String],
    pipeline_lookback: Duration,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let version: Version = client.get_json("/api/v4/version").await?;
    let mut errors = 0u32;
    let mut out = version_samples(&version, ts_ms);

    let (readiness, queues, runners, migrations, statistics, settings, license) = futures::join!(
        client.get_json_even_on_error::<Value>("/-/readiness?all=1"),
        client.get_json::<QueueMetrics>("/api/v4/sidekiq/queue_metrics"),
        client.get_json::<Vec<Runner>>("/api/v4/runners/all?per_page=100"),
        client.get_json::<PendingMigrations>("/api/v4/admin/migrations/pending"),
        client.get_json::<Value>("/api/v4/application/statistics"),
        client.get_json::<Value>("/api/v4/application/settings"),
        client.get_json::<License>("/api/v4/license"),
    );
    if let Some(v) = settle(readiness, &mut errors, target_id, "/-/readiness") {
        out.extend(readiness_samples(&v, ts_ms));
    }
    if let Some(v) = settle(queues, &mut errors, target_id, "/api/v4/sidekiq/queue_metrics") {
        out.extend(queue_samples(&v, ts_ms));
    }
    if let Some(v) = settle(runners, &mut errors, target_id, "/api/v4/runners/all") {
        out.extend(runner_samples(&v, ts_ms));
    }
    if let Some(v) = settle(migrations, &mut errors, target_id, "/api/v4/admin/migrations/pending")
    {
        out.extend(migration_samples(&v, ts_ms));
    }
    if let Some(v) = settle(statistics, &mut errors, target_id, "/api/v4/application/statistics") {
        out.extend(statistics_samples(&v, ts_ms));
    }
    if let Some(v) = settle(settings, &mut errors, target_id, "/api/v4/application/settings") {
        out.extend(settings_samples(&v, ts_ms));
    }
    if let Some(v) = settle(license, &mut errors, target_id, "/api/v4/license") {
        out.extend(license_samples(&v, ts_ms));
    }

    let since =
        chrono::Utc::now() - chrono::Duration::from_std(pipeline_lookback).unwrap_or_default();
    for project in projects.iter().take(MAX_NAMED) {
        let path = format!("/api/v4/projects/{project}/pipelines?per_page=20");
        match client.get_json::<Vec<Pipeline>>(&path).await {
            Ok(pipelines) => out.extend(pipeline_samples(project, &pipelines, since, ts_ms)),
            Err(error) => {
                errors += 1;
                tracing::warn!(target_id, project, %error, "pipelines GitLab indisponibles");
            }
        }
    }

    out.push(gauge("gitlab_scrape_errors", f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ traduction

pub fn version_samples(version: &Version, ts_ms: i64) -> Vec<Sample> {
    let mut sample =
        gauge("gitlab_version_info", 1.0, ts_ms).with_label("version", &version.version);
    if let Some(revision) = version.revision.as_deref() {
        sample = sample.with_label("revision", revision);
    }
    vec![sample]
}

/// Chaque clé de premier niveau est un composant (`db_check`, `gitaly_check`…),
/// sa valeur une liste d'au moins un élément ; seul le premier est regardé.
/// `ok` vaut 0, tout le reste (`failed`…) vaut 1.
pub fn readiness_samples(readiness: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(checks) = readiness.as_object() else { return Vec::new() };
    let mut out = Vec::new();
    for (name, value) in checks {
        let status = value.get(0).and_then(|first| first.get("status")).and_then(Value::as_str);
        let failed = !matches!(status, Some("ok"));
        let check = name.strip_suffix("_check").unwrap_or(name);
        out.push(flag("gitlab_readiness_check", failed, ts_ms).with_label("check", check));
    }
    out
}

pub fn queue_samples(metrics: &QueueMetrics, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for (queue, metric) in metrics.queues.iter().take(MAX_NAMED) {
        out.push(
            gauge("gitlab_sidekiq_queue_backlog", metric.backlog, ts_ms).with_label("queue", queue),
        );
        out.push(
            gauge("gitlab_sidekiq_queue_latency_seconds", metric.latency, ts_ms)
                .with_label("queue", queue),
        );
    }
    out
}

pub fn runner_samples(runners: &[Runner], ts_ms: i64) -> Vec<Sample> {
    let online = runners.iter().filter(|r| r.status == "online").count();
    let offline: Vec<&Runner> = runners.iter().filter(|r| r.status != "online").collect();
    let mut out = vec![
        gauge("gitlab_runners_total", runners.len() as f64, ts_ms),
        gauge("gitlab_runners_online", online as f64, ts_ms),
        gauge("gitlab_runners_offline", offline.len() as f64, ts_ms),
    ];
    for runner in offline.iter().take(MAX_NAMED) {
        let name = runner.description.as_deref().unwrap_or("runner");
        out.push(flag("gitlab_runner_offline", true, ts_ms).with_label("runner", name));
    }
    out
}

/// Migrations de base de données posées par une mise à jour, pas encore
/// exécutées (`/api/v4/admin/migrations/pending`). GitLab ne dit pas, par
/// cette voie, si l'une d'elles a déjà échoué.
pub fn migration_samples(pending: &PendingMigrations, ts_ms: i64) -> Vec<Sample> {
    let mut out =
        vec![gauge("gitlab_migrations_pending", pending.pending_migrations.len() as f64, ts_ms)];
    for migration in pending.pending_migrations.iter().take(MAX_NAMED) {
        out.push(
            flag("gitlab_migration_pending", true, ts_ms).with_label("migration", &migration.name),
        );
    }
    out
}

pub fn statistics_samples(statistics: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for (name, key) in [
        ("gitlab_projects", "projects"),
        ("gitlab_users", "users"),
        ("gitlab_groups", "groups"),
        ("gitlab_forks", "forks"),
        ("gitlab_issues", "issues"),
        ("gitlab_merge_requests", "merge_requests"),
    ] {
        if let Some(value) = number(statistics.get(key)) {
            out.push(gauge(name, value, ts_ms));
        }
    }
    out
}

pub fn settings_samples(settings: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(signup) = settings.get("signup_enabled").and_then(Value::as_bool) {
        out.push(flag("gitlab_signup_enabled", signup, ts_ms));
    }
    if let Some(two_factor) =
        settings.get("require_two_factor_authentication").and_then(Value::as_bool)
    {
        out.push(flag("gitlab_two_factor_required", two_factor, ts_ms));
    }
    out
}

pub fn license_samples(license: &License, ts_ms: i64) -> Vec<Sample> {
    let Some(plan) = license.plan.as_deref() else { return Vec::new() };
    let mut out = vec![gauge("gitlab_license_info", 1.0, ts_ms).with_label("plan", plan)];
    if let Some(expired) = license.expired {
        out.push(flag("gitlab_license_expired", expired, ts_ms));
    }
    if let Some(expiry) = license
        .expires_at
        .as_deref()
        .and_then(|raw| chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok())
    {
        let expires_at = expiry.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc();
        let seconds = (expires_at - chrono::Utc::now()).num_seconds();
        out.push(gauge("gitlab_license_expiry_seconds", seconds as f64, ts_ms));
    }
    out
}

/// Pipelines en échec depuis `since`, et en cours (sans fenêtre : l'état actuel).
pub fn pipeline_samples(
    project: &str,
    pipelines: &[Pipeline],
    since: DateTime<Utc>,
    ts_ms: i64,
) -> Vec<Sample> {
    let recent = |pipeline: &&Pipeline| {
        pipeline
            .updated_at
            .as_deref()
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .is_none_or(|updated| updated.with_timezone(&Utc) >= since)
    };
    let failed = pipelines.iter().filter(|p| p.status == "failed").filter(recent).count();
    let pending =
        pipelines.iter().filter(|p| matches!(p.status.as_str(), "pending" | "running")).count();
    vec![
        gauge("gitlab_pipelines_failed_recent", failed as f64, ts_ms)
            .with_label("project", project),
        gauge("gitlab_pipelines_pending", pending as f64, ts_ms).with_label("project", project),
    ]
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct GitlabCollector;

impl GitlabCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(token) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "GitLab expects a personal access token (PRIVATE-TOKEN), configured: {}",
                target.credential
            )));
        };
        let auth = Auth::header("PRIVATE-TOKEN", token);
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/api/v4").unwrap_or(trimmed).to_string();
        http_client(&target, "https", DEFAULT_PORT, auth, "GitLab")
    }

    /// Projets numériques listés dans l'étiquette `watched_projects`, séparés
    /// par des virgules : la santé CI ne se lit que pour ceux-là.
    fn projects(target: &Target) -> Vec<String> {
        let Some(raw) = tag(target, "watched_projects") else { return Vec::new() };
        raw.split(',').map(str::trim).filter(|p| !p.is_empty()).map(str::to_string).collect()
    }

    fn pipeline_lookback(target: &Target) -> Result<Duration, ProbeError> {
        let Some(raw) = tag(target, "pipeline_lookback_hours") else {
            return Ok(Duration::from_secs(DEFAULT_PIPELINE_LOOKBACK_HOURS * 3600));
        };
        match raw.parse::<u64>() {
            Ok(hours) if (1..=720).contains(&hours) => Ok(Duration::from_secs(hours * 3600)),
            _ => Err(ProbeError::Config(format!(
                "Pipeline lookback window must be between 1 and 720 hours, got \"{raw}\""
            ))),
        }
    }
}

#[async_trait]
impl Collector for GitlabCollector {
    fn kind(&self) -> &'static str {
        "gitlab"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let projects = Self::projects(target);
        let lookback = Self::pipeline_lookback(target)?;
        probe(&Self::client(target)?, target.id, &projects, lookback, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: Version = Self::client(target)?.get_json("/api/v4/version").await?;
        Ok(Some("gitlab".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;

    // Réponses d'un GitLab EE 17.5.2 self-managed, composées mot pour mot sur
    // la documentation officielle de l'API REST (version, readiness,
    // Sidekiq, exécuteurs, migrations, statistiques, réglages, licence,
    // pipelines).
    const VERSION: &str = include_str!("testdata/gitlab_17.5.2-ee/version.json");
    const READINESS: &str = include_str!("testdata/gitlab_17.5.2-ee/readiness.json");
    const READINESS_DEGRADED: &str =
        include_str!("testdata/gitlab_17.5.2-ee/readiness_degraded.json");
    const QUEUE_METRICS: &str =
        include_str!("testdata/gitlab_17.5.2-ee/sidekiq_queue_metrics.json");
    const RUNNERS: &str = include_str!("testdata/gitlab_17.5.2-ee/runners_all.json");
    const MIGRATIONS: &str = include_str!("testdata/gitlab_17.5.2-ee/migrations_pending.json");
    const MIGRATIONS_NONE: &str =
        include_str!("testdata/gitlab_17.5.2-ee/migrations_pending_none.json");
    const STATISTICS: &str = include_str!("testdata/gitlab_17.5.2-ee/application_statistics.json");
    const SETTINGS: &str = include_str!("testdata/gitlab_17.5.2-ee/application_settings.json");
    const LICENSE: &str = include_str!("testdata/gitlab_17.5.2-ee/license.json");
    const PIPELINES: &str = include_str!("testdata/gitlab_17.5.2-ee/pipelines.json");

    fn json(body: &str) -> Value {
        serde_json::from_str(body).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn la_version_porte_le_numero_et_la_revision() {
        let version: Version = serde_json::from_str(VERSION).unwrap();
        let samples = version_samples(&version, 0);
        let info = samples.iter().find(|s| s.metric == "gitlab_version_info").unwrap();
        assert_eq!(info.labels["version"], "17.5.2-ee");
        assert_eq!(info.labels["revision"], "a1b2c3d4e5f");
    }

    #[test]
    fn la_disponibilite_detaillee_nomme_le_composant_en_echec() {
        let samples = readiness_samples(&json(READINESS), 0);
        assert!(samples.iter().all(|s| s.value == 0.0));
        let degraded = readiness_samples(&json(READINESS_DEGRADED), 0);
        let db = degraded
            .iter()
            .find(|s| s.metric == "gitlab_readiness_check" && s.labels["check"] == "db")
            .unwrap();
        assert_eq!(db.value, 1.0);
    }

    #[test]
    fn les_files_sidekiq_donnent_le_retard_et_la_latence() {
        let metrics: QueueMetrics = serde_json::from_str(QUEUE_METRICS).unwrap();
        let samples = queue_samples(&metrics, 0);
        let backlog = samples
            .iter()
            .find(|s| {
                s.metric == "gitlab_sidekiq_queue_backlog" && s.labels["queue"] == "post_receive"
            })
            .unwrap();
        assert_eq!(backlog.value, 842.0);
    }

    #[test]
    fn les_executeurs_hors_ligne_sont_nommes() {
        let runners: Vec<Runner> = serde_json::from_str(RUNNERS).unwrap();
        let samples = runner_samples(&runners, 0);
        assert_eq!(value(&samples, "gitlab_runners_total"), Some(3.0));
        assert_eq!(value(&samples, "gitlab_runners_online"), Some(1.0));
        assert_eq!(value(&samples, "gitlab_runners_offline"), Some(2.0));
        let offline: Vec<_> =
            samples.iter().filter(|s| s.metric == "gitlab_runner_offline").collect();
        assert_eq!(offline.len(), 2);
        assert!(offline.iter().any(|s| s.labels["runner"] == "shared-runner-2"));
    }

    #[test]
    fn les_migrations_en_attente_sont_nommees() {
        let pending: PendingMigrations = serde_json::from_str(MIGRATIONS).unwrap();
        let samples = migration_samples(&pending, 0);
        assert_eq!(value(&samples, "gitlab_migrations_pending"), Some(1.0));
        let named = samples.iter().find(|s| s.metric == "gitlab_migration_pending").unwrap();
        assert_eq!(named.labels["migration"], "add_index_to_issues_on_namespace_id");

        let none: PendingMigrations = serde_json::from_str(MIGRATIONS_NONE).unwrap();
        let samples = migration_samples(&none, 0);
        assert_eq!(value(&samples, "gitlab_migrations_pending"), Some(0.0));
        assert!(!samples.iter().any(|s| s.metric == "gitlab_migration_pending"));
    }

    #[test]
    fn les_statistiques_se_lisent() {
        let samples = statistics_samples(&json(STATISTICS), 0);
        assert_eq!(value(&samples, "gitlab_projects"), Some(64.0));
        assert_eq!(value(&samples, "gitlab_users"), Some(57.0));
    }

    #[test]
    fn les_reglages_de_securite_se_lisent() {
        let samples = settings_samples(&json(SETTINGS), 0);
        assert_eq!(value(&samples, "gitlab_signup_enabled"), Some(0.0));
        assert_eq!(value(&samples, "gitlab_two_factor_required"), Some(1.0));
    }

    #[test]
    fn la_licence_donne_le_plan_et_l_expiration() {
        let license: License = serde_json::from_str(LICENSE).unwrap();
        let samples = license_samples(&license, 0);
        let info = samples.iter().find(|s| s.metric == "gitlab_license_info").unwrap();
        assert_eq!(info.labels["plan"], "ultimate");
        assert_eq!(value(&samples, "gitlab_license_expired"), Some(0.0));
        assert!(value(&samples, "gitlab_license_expiry_seconds").is_some());
    }

    #[test]
    fn les_pipelines_en_echec_recent_et_en_cours() {
        let pipelines: Vec<Pipeline> = serde_json::from_str(PIPELINES).unwrap();
        let since = chrono::DateTime::parse_from_rfc3339("2026-09-30T00:00:00Z").unwrap().into();
        let samples = pipeline_samples("64", &pipelines, since, 0);
        assert_eq!(value(&samples, "gitlab_pipelines_failed_recent"), Some(1.0));
        assert_eq!(value(&samples, "gitlab_pipelines_pending"), Some(1.0));
        let later = chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z").unwrap().into();
        assert_eq!(
            value(&pipeline_samples("64", &pipelines, later, 0), "gitlab_pipelines_failed_recent"),
            Some(0.0)
        );
    }

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 11,
            name: "gitlab".into(),
            address,
            kind: "gitlab".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential,
        }
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("private-token").and_then(|v| v.to_str().ok()) == Some("glpat-abc123")
    }

    #[tokio::test]
    async fn la_sonde_complete_lit_la_version_puis_le_reste_en_complement() {
        let app = Router::new()
            .route(
                "/api/v4/version",
                get(|headers: HeaderMap| async move {
                    if !authorized(&headers) {
                        return (StatusCode::UNAUTHORIZED, "{}".to_string());
                    }
                    (StatusCode::OK, VERSION.to_string())
                }),
            )
            .route("/-/readiness", get(|| async { READINESS }))
            .route("/api/v4/sidekiq/queue_metrics", get(|| async { QUEUE_METRICS }))
            .route("/api/v4/runners/all", get(|| async { RUNNERS }))
            .route("/api/v4/admin/migrations/pending", get(|| async { MIGRATIONS }))
            .route("/api/v4/application/statistics", get(|| async { STATISTICS }))
            .route("/api/v4/application/settings", get(|| async { SETTINGS }))
            .route("/api/v4/license", get(|| async { LICENSE }));
        let base = serve(app).await;
        let cred = Credential::ApiToken { token: "glpat-abc123".into() };
        let samples = GitlabCollector::new().probe(&target(base.clone(), cred)).await.unwrap();
        assert_eq!(value(&samples, "gitlab_runners_online"), Some(1.0));
        assert_eq!(value(&samples, "gitlab_scrape_errors"), Some(0.0));

        let wrong = Credential::ApiToken { token: "wrong".into() };
        let error = GitlabCollector::new().probe(&target(base, wrong)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn sans_compte_administrateur_le_reste_est_compte_en_erreur() {
        let app = Router::new().route("/api/v4/version", get(|| async { VERSION }));
        let base = serve(app).await;
        let cred = Credential::ApiToken { token: "glpat-abc123".into() };
        let samples = GitlabCollector::new().probe(&target(base, cred)).await.unwrap();
        assert!(value(&samples, "gitlab_scrape_errors").unwrap() > 0.0);
        let info = samples.iter().find(|s| s.metric == "gitlab_version_info").unwrap();
        assert_eq!(info.labels["version"], "17.5.2-ee");
    }

    #[test]
    fn les_projets_surveilles_viennent_de_l_etiquette() {
        let mut t = target("gitlab.lan".into(), Credential::ApiToken { token: "x".into() });
        assert!(GitlabCollector::projects(&t).is_empty());
        t.tags.insert("watched_projects".into(), " 1, 64 ,9 ".into());
        assert_eq!(GitlabCollector::projects(&t), vec!["1", "64", "9"]);
    }

    #[test]
    fn sans_jeton_c_est_une_erreur_de_configuration() {
        let t = target("gitlab.lan".into(), Credential::None);
        assert!(matches!(GitlabCollector::client(&t), Err(ProbeError::Config(_))));
    }
}
