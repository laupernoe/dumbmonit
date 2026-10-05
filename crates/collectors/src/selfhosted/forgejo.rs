//! Forgejo et Gitea : même API, lue par un jeton d'accès (`Authorization:
//! token …`).
//!
//! Deux appels doivent réussir :
//!
//! * `/api/v1/version` — public, la preuve de vie ;
//! * `/api/v1/user` — le compte propriétaire du jeton (`is_admin` dit s'il a
//!   accès au reste).
//!
//! Le reste est au mieux, compté dans `forgejo_scrape_errors` sans faire
//! échouer la sonde :
//!
//! * `/api/healthz` — base de données et cache, au format
//!   [IETF health check](https://datatracker.ietf.org/doc/html/draft-inadarei-api-health-check) ;
//!   en échec, l'instance répond 424 avec le même corps, pas seulement le code ;
//! * `/api/v1/repos/search?limit=1` — le nombre de dépôts visibles du compte,
//!   dans l'en-tête `X-Total-Count` ;
//! * `/api/v1/admin/users?limit=1`, `/api/v1/admin/orgs?limit=1` — comptes et
//!   organisations de l'instance, même en-tête, administrateur uniquement ;
//! * `/api/v1/admin/cron` — les tâches planifiées (vérification de dépôts,
//!   nettoyage…) et leur prochaine exécution : en retard, c'est qu'elles ne
//!   tournent plus ;
//! * `/api/v1/admin/actions/runners` — les exécuteurs Actions, en ligne ou non.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dumbmonit_proto::{Collector, ProbeError, Sample, Target, TargetId};
use serde::Deserialize;
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::{MAX_NAMED, flag, gauge, http_client, now_ms, settle, token};

pub const DEFAULT_PORT: u16 = 3000;

#[derive(Debug, Deserialize)]
pub struct Version {
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    #[serde(default)]
    is_admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct Cron {
    #[serde(default)]
    name: String,
    #[serde(default)]
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActionRunnersResponse {
    #[serde(default)]
    entries: Vec<ActionRunner>,
}

#[derive(Debug, Deserialize)]
pub struct ActionRunner {
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    client: &HttpClient,
    target_id: TargetId,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let version: Version = client.get_json("/api/v1/version").await?;
    let user: User = client.get_json("/api/v1/user").await?;
    let mut errors = 0u32;
    let mut out = version_samples(&version, ts_ms);
    out.push(flag("forgejo_token_is_admin", user.is_admin, ts_ms));

    let (healthz, repos, users, orgs, cron, runners) = futures::join!(
        client.get_json_even_on_error::<Value>("/api/healthz"),
        client.get_total_count("/api/v1/repos/search?limit=1"),
        client.get_total_count("/api/v1/admin/users?limit=1"),
        client.get_total_count("/api/v1/admin/orgs?limit=1"),
        client.get_json::<Vec<Cron>>("/api/v1/admin/cron"),
        client.get_json::<ActionRunnersResponse>("/api/v1/admin/actions/runners"),
    );
    if let Some(v) = settle(healthz, &mut errors, target_id, "/api/healthz") {
        out.extend(healthz_samples(&v, ts_ms));
    }
    if let Some(Some(total)) = settle(repos, &mut errors, target_id, "/api/v1/repos/search") {
        out.push(gauge("forgejo_repos", total, ts_ms));
    }
    if let Some(Some(total)) = settle(users, &mut errors, target_id, "/api/v1/admin/users") {
        out.push(gauge("forgejo_users", total, ts_ms));
    }
    if let Some(Some(total)) = settle(orgs, &mut errors, target_id, "/api/v1/admin/orgs") {
        out.push(gauge("forgejo_orgs", total, ts_ms));
    }
    if let Some(v) = settle(cron, &mut errors, target_id, "/api/v1/admin/cron") {
        out.extend(cron_samples(&v, Utc::now(), ts_ms));
    }
    if let Some(v) = settle(runners, &mut errors, target_id, "/api/v1/admin/actions/runners") {
        out.extend(runner_samples(&v, ts_ms));
    }

    out.push(gauge("forgejo_scrape_errors", f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ traduction

pub fn version_samples(version: &Version, ts_ms: i64) -> Vec<Sample> {
    vec![gauge("forgejo_version_info", 1.0, ts_ms).with_label("version", &version.version)]
}

/// Format IETF : `status` vaut `pass`, `warn` ou `fail` ; chaque entrée de
/// `checks` est une liste dont seul le premier élément est regardé.
pub fn healthz_samples(healthz: &Value, ts_ms: i64) -> Vec<Sample> {
    let healthy = healthz.get("status").and_then(Value::as_str) != Some("fail");
    let mut out = vec![flag("forgejo_healthz_ok", healthy, ts_ms)];
    if let Some(checks) = healthz.get("checks").and_then(Value::as_object) {
        for (name, value) in checks {
            let status = value.get(0).and_then(|first| first.get("status")).and_then(Value::as_str);
            let component = name.split(':').next().unwrap_or(name);
            out.push(
                flag("forgejo_healthz_check", status == Some("fail"), ts_ms)
                    .with_label("check", component),
            );
        }
    }
    out
}

/// En retard : la prochaine exécution annoncée est déjà passée.
pub fn cron_samples(tasks: &[Cron], now: DateTime<Utc>, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge("forgejo_cron_tasks", tasks.len() as f64, ts_ms)];
    // Un drapeau par tâche, qu'elle soit en retard ou non : un contrôle de
    // sécurité qui ne lirait que les tâches en retard ne verrait jamais le cas
    // sain, faute de série à zéro pour les autres.
    for task in tasks.iter().take(MAX_NAMED) {
        let overdue = task
            .next
            .as_deref()
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .is_some_and(|next| next.with_timezone(&Utc) < now);
        out.push(flag("forgejo_cron_task_overdue", overdue, ts_ms).with_label("task", &task.name));
    }
    out
}

pub fn runner_samples(runners: &ActionRunnersResponse, ts_ms: i64) -> Vec<Sample> {
    let online = runners.entries.iter().filter(|r| r.status == "online").count();
    let offline: Vec<&ActionRunner> =
        runners.entries.iter().filter(|r| r.status != "online").collect();
    let mut out = vec![
        gauge("forgejo_runners_total", runners.entries.len() as f64, ts_ms),
        gauge("forgejo_runners_online", online as f64, ts_ms),
        gauge("forgejo_runners_offline", offline.len() as f64, ts_ms),
    ];
    for runner in offline.iter().take(MAX_NAMED) {
        let name = if runner.name.is_empty() { "runner" } else { &runner.name };
        out.push(flag("forgejo_runner_offline", true, ts_ms).with_label("runner", name));
    }
    out
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct ForgejoCollector;

impl ForgejoCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(token) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "Forgejo and Gitea expect an access token (Authorization: token …), configured: {}",
                target.credential
            )));
        };
        let auth = Auth::header("Authorization", format!("token {token}"));
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/api/v1").unwrap_or(trimmed).to_string();
        http_client(&target, "http", DEFAULT_PORT, auth, "Forgejo")
    }
}

#[async_trait]
impl Collector for ForgejoCollector {
    fn kind(&self) -> &'static str {
        "forgejo"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, target.id, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: Version = Self::client(target)?.get_json("/api/v1/version").await?;
        Ok(Some("forgejo".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, header::HeaderValue};
    use axum::response::{IntoResponse, Response};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;

    // Réponses d'un Forgejo 9.0.2, composées mot pour mot sur la définition
    // Go publique de chaque structure (`modules/structs`) et le point de
    // contrôle de santé (`routers/web/healthcheck`), partagés avec Gitea.
    const VERSION: &str = include_str!("testdata/forgejo_9.0.2/version.json");
    const USER: &str = include_str!("testdata/forgejo_9.0.2/user.json");
    const USER_NOT_ADMIN: &str = include_str!("testdata/forgejo_9.0.2/user_not_admin.json");
    const HEALTHZ: &str = include_str!("testdata/forgejo_9.0.2/healthz.json");
    const HEALTHZ_FAILED: &str = include_str!("testdata/forgejo_9.0.2/healthz_failed.json");
    const CRON: &str = include_str!("testdata/forgejo_9.0.2/cron.json");
    const ACTIONS_RUNNERS: &str = include_str!("testdata/forgejo_9.0.2/actions_runners.json");

    fn json(body: &str) -> Value {
        serde_json::from_str(body).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn la_version_se_lit() {
        let version: Version = serde_json::from_str(VERSION).unwrap();
        let samples = version_samples(&version, 0);
        assert_eq!(samples[0].labels["version"], "9.0.2");
    }

    #[test]
    fn la_sante_nomme_le_composant_en_echec() {
        let samples = healthz_samples(&json(HEALTHZ), 0);
        assert_eq!(value(&samples, "forgejo_healthz_ok"), Some(1.0));
        let failed = healthz_samples(&json(HEALTHZ_FAILED), 0);
        assert_eq!(value(&failed, "forgejo_healthz_ok"), Some(0.0));
        let db = failed
            .iter()
            .find(|s| s.metric == "forgejo_healthz_check" && s.labels["check"] == "database")
            .unwrap();
        assert_eq!(db.value, 1.0);
    }

    #[test]
    fn les_taches_planifiees_en_retard_sont_nommees() {
        let tasks: Vec<Cron> = serde_json::from_str(CRON).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-03T12:00:00Z").unwrap().into();
        let samples = cron_samples(&tasks, now, 0);
        assert_eq!(value(&samples, "forgejo_cron_tasks"), Some(3.0));
        let flags: Vec<_> =
            samples.iter().filter(|s| s.metric == "forgejo_cron_task_overdue").collect();
        assert_eq!(flags.len(), 3);
        let overdue: Vec<_> = flags.iter().filter(|s| s.value == 1.0).collect();
        assert_eq!(overdue.len(), 1);
        assert_eq!(overdue[0].labels["task"], "cleanup_hook_task_table");
    }

    #[test]
    fn les_executeurs_hors_ligne_sont_nommes() {
        let runners: ActionRunnersResponse = serde_json::from_str(ACTIONS_RUNNERS).unwrap();
        let samples = runner_samples(&runners, 0);
        assert_eq!(value(&samples, "forgejo_runners_online"), Some(1.0));
        assert_eq!(value(&samples, "forgejo_runners_offline"), Some(1.0));
        let offline = samples.iter().find(|s| s.metric == "forgejo_runner_offline").unwrap();
        assert_eq!(offline.labels["runner"], "runner-amd64-2");
    }

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 12,
            name: "forgejo".into(),
            address,
            kind: "forgejo".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("token abc123")
    }

    fn with_total_count(body: &'static str, total: usize) -> Response {
        let mut response = body.into_response();
        response
            .headers_mut()
            .insert("x-total-count", HeaderValue::from_str(&total.to_string()).unwrap());
        response
    }

    #[tokio::test]
    async fn la_sonde_complete_lit_la_version_puis_l_utilisateur() {
        let app = Router::new()
            .route(
                "/api/v1/version",
                get(|headers: HeaderMap| async move {
                    if !authorized(&headers) {
                        return (StatusCode::UNAUTHORIZED, "{}".to_string());
                    }
                    (StatusCode::OK, VERSION.to_string())
                }),
            )
            .route("/api/v1/user", get(|| async { USER }))
            .route("/api/healthz", get(|| async { HEALTHZ }))
            .route(
                "/api/v1/repos/search",
                get(|| async { with_total_count("{\"ok\":true,\"data\":[]}", 12) }),
            )
            .route("/api/v1/admin/users", get(|| async { with_total_count("[]", 4) }))
            .route("/api/v1/admin/orgs", get(|| async { with_total_count("[]", 2) }))
            .route("/api/v1/admin/cron", get(|| async { CRON }))
            .route("/api/v1/admin/actions/runners", get(|| async { ACTIONS_RUNNERS }));
        let base = serve(app).await;
        let cred = Credential::ApiToken { token: "abc123".into() };
        let samples = ForgejoCollector::new().probe(&target(base.clone(), cred)).await.unwrap();
        assert_eq!(value(&samples, "forgejo_repos"), Some(12.0));
        assert_eq!(value(&samples, "forgejo_users"), Some(4.0));
        assert_eq!(value(&samples, "forgejo_orgs"), Some(2.0));
        assert_eq!(value(&samples, "forgejo_token_is_admin"), Some(1.0));
        assert_eq!(value(&samples, "forgejo_scrape_errors"), Some(0.0));

        let wrong = Credential::ApiToken { token: "wrong".into() };
        let error = ForgejoCollector::new().probe(&target(base, wrong)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn un_jeton_ordinaire_lit_la_version_et_l_identite_seules() {
        let app = Router::new()
            .route("/api/v1/version", get(|| async { VERSION }))
            .route("/api/v1/user", get(|| async { USER_NOT_ADMIN }));
        let base = serve(app).await;
        let cred = Credential::ApiToken { token: "abc123".into() };
        let samples = ForgejoCollector::new().probe(&target(base, cred)).await.unwrap();
        assert_eq!(value(&samples, "forgejo_token_is_admin"), Some(0.0));
        assert!(value(&samples, "forgejo_scrape_errors").unwrap() > 0.0);
    }

    #[test]
    fn sans_jeton_c_est_une_erreur_de_configuration() {
        let t = target("forgejo.lan".into(), Credential::None);
        assert!(matches!(ForgejoCollector::client(&t), Err(ProbeError::Config(_))));
    }
}
