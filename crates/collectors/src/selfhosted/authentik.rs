//! authentik.
//!
//! Un jeton d'API (`Authorization: Bearer …`) d'un compte de service en
//! lecture seule, et les routes d'administration de l'API v3 :
//! `/api/v3/admin/version/`, `/admin/workers/`, `/admin/system_tasks/`,
//! `/outposts/instances/` et `/core/users/`. Les sondes de santé
//! (`/-/health/live/` et `/-/health/ready/`) sont publiques.
//!
//! Les pannes que ce module existe pour voir : **un travailleur arrêté** (plus
//! de synchronisation LDAP, plus de courriels, plus de nettoyage des sessions,
//! sans rien d'autre qu'un compteur dans l'administration), **une tâche
//! planifiée en échec** (synchronisation d'une source, blueprint non appliqué)
//! et **un avant-poste déconnecté** (les applications qu'il protège ne
//! répondent plus). Les trois sont facultatives : une page que le jeton ne
//! peut pas lire, ou qu'une autre version de authentik n'a pas, est sautée.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::{
    MAX_NAMED, flag, gauge, http_client, instant, metric, now_ms, number, optional_json, settle,
    token,
};

pub const DEFAULT_PORT: u16 = 9000;
const PRODUCT: &str = "authentik";
const P: &str = "authentik";

/// Un avant-poste est connecté s'il a donné signe de vie dans ce délai.
pub const OUTPOST_FRESH_SECONDS: i64 = 300;

/// Au plus autant d'avant-postes interrogés par sonde (un appel chacun).
pub const MAX_OUTPOSTS: usize = 10;

// ------------------------------------------------------------ traduction

/// `GET /api/v3/admin/version/`.
pub fn version_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let current = body.get("version_current").and_then(Value::as_str).unwrap_or("unknown");
    let mut out =
        vec![gauge(&metric(P, "version_info"), 1.0, ts_ms).with_label("version", current)];
    // `outdated` n'a de sens que si authentik a pu lire la dernière version
    // publiée (`version_latest_valid`) : sans accès sortant, il se tait.
    if body.get("version_latest_valid").and_then(Value::as_bool) == Some(true) {
        let latest = body.get("version_latest").and_then(Value::as_str).unwrap_or("");
        let outdated = body.get("outdated").and_then(Value::as_bool).unwrap_or(false);
        out.push(
            flag(&metric(P, "update_available"), outdated, ts_ms)
                .with_label("latest_version", latest),
        );
    }
    out
}

/// Les deux sondes de santé : un code 2xx est « prêt ».
pub fn health_samples(live: bool, ready: bool, ts_ms: i64) -> Vec<Sample> {
    vec![flag(&metric(P, "live"), live, ts_ms), flag(&metric(P, "ready"), ready, ts_ms)]
}

/// `GET /api/v3/admin/workers/` : `[{"worker_id", "version", "version_matches"}]`.
pub fn worker_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(workers) = body.as_array() else { return Vec::new() };
    let mismatched =
        workers.iter().filter(|w| w.get("version_matches").and_then(Value::as_bool) == Some(false));
    vec![
        gauge(&metric(P, "workers"), workers.len() as f64, ts_ms),
        gauge(&metric(P, "workers_version_mismatch"), mismatched.count() as f64, ts_ms),
    ]
}

/// `GET /api/v3/admin/system_tasks/` : la dernière exécution de chaque tâche,
/// `SUCCESSFUL`, `WARNING`, `ERROR` ou `UNKNOWN`.
pub fn task_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(tasks) = body.as_array() else { return Vec::new() };
    let status =
        |task: &Value| task.get("status").and_then(Value::as_str).unwrap_or("").to_string();
    let failed: Vec<&Value> = tasks.iter().filter(|t| status(t) == "ERROR").collect();
    let warned = tasks.iter().filter(|t| status(t) == "WARNING").count();
    let mut out = vec![
        gauge(&metric(P, "tasks_failed"), failed.len() as f64, ts_ms),
        gauge(&metric(P, "tasks_warning"), warned as f64, ts_ms),
    ];
    for task in failed.into_iter().take(MAX_NAMED) {
        let name = task.get("task_name").and_then(Value::as_str).unwrap_or("unknown");
        out.push(gauge(&metric(P, "task_failed"), 1.0, ts_ms).with_label("task", name));
    }
    out
}

/// Le nombre d'instances d'un avant-poste vues récemment, d'après
/// `GET /outposts/instances/{pk}/health/`.
pub fn fresh_instances(health: &Value, now_s: i64) -> usize {
    health
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter(|h| {
            h.get("last_seen")
                .and_then(Value::as_str)
                .and_then(instant)
                .is_some_and(|seen| now_s - seen <= OUTPOST_FRESH_SECONDS)
        })
        .count()
}

/// Les avant-postes qui servent au moins un fournisseur : `(pk, nom)`. Un
/// avant-poste sans fournisseur n'est pas censé tourner.
pub fn used_outposts(body: &Value) -> Vec<(String, String)> {
    body.get("results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter(|o| o.get("providers").and_then(Value::as_array).is_some_and(|p| !p.is_empty()))
        .filter_map(|o| {
            Some((o.get("pk")?.as_str()?.to_string(), o.get("name")?.as_str()?.to_string()))
        })
        .take(MAX_OUTPOSTS)
        .collect()
}

pub fn outpost_samples(connected: &[(String, usize)], ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        gauge(&metric(P, "outposts_total"), connected.len() as f64, ts_ms),
        gauge(
            &metric(P, "outposts_disconnected"),
            connected.iter().filter(|(_, n)| *n == 0).count() as f64,
            ts_ms,
        ),
    ];
    for (name, instances) in connected {
        out.push(
            gauge(&metric(P, "outpost_instances"), *instances as f64, ts_ms)
                .with_label("outpost", name),
        );
    }
    out
}

// ------------------------------------------------------------------ sonde

/// Un code HTTP de sonde de santé : vrai pour un 2xx, faux pour le reste,
/// sans erreur (un 503 est la réponse).
async fn health(client: &HttpClient, path: &str) -> Result<bool, ProbeError> {
    Ok(client.get(path).await?.0.is_success())
}

pub async fn probe(
    client: &HttpClient,
    target: &Target,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let version: Value = client.get_json("/api/v3/admin/version/").await?;
    let mut out = version_samples(&version, ts_ms);
    let mut errors = 0u32;
    let (live, ready, workers, tasks, outposts, users) = futures::join!(
        health(client, "/-/health/live/"),
        health(client, "/-/health/ready/"),
        optional_json(client, "/api/v3/admin/workers/"),
        optional_json(client, "/api/v3/admin/system_tasks/"),
        optional_json(client, "/api/v3/outposts/instances/?page_size=100"),
        optional_json(client, "/api/v3/core/users/?page_size=1"),
    );
    let live = settle(live, &mut errors, target.id, "health/live");
    let ready = settle(ready, &mut errors, target.id, "health/ready");
    if let (Some(live), Some(ready)) = (live, ready) {
        out.extend(health_samples(live, ready, ts_ms));
    }
    if let Some(Some(body)) = settle(workers, &mut errors, target.id, "admin/workers") {
        out.extend(worker_samples(&body, ts_ms));
    }
    if let Some(Some(body)) = settle(tasks, &mut errors, target.id, "admin/system_tasks") {
        out.extend(task_samples(&body, ts_ms));
    }
    if let Some(Some(body)) = settle(users, &mut errors, target.id, "core/users")
        && let Some(count) = number(body.pointer("/pagination/count"))
    {
        out.push(gauge(&metric(P, "users"), count, ts_ms));
    }
    if let Some(Some(body)) = settle(outposts, &mut errors, target.id, "outposts/instances") {
        let now_s = ts_ms / 1000;
        let paths: Vec<(String, String)> = used_outposts(&body)
            .into_iter()
            .map(|(pk, name)| (format!("/api/v3/outposts/instances/{pk}/health/"), name))
            .collect();
        let healths =
            futures::future::join_all(paths.iter().map(|(path, _)| optional_json(client, path)))
                .await;
        let mut connected = Vec::new();
        for ((_, name), health) in paths.iter().zip(healths) {
            if let Some(Some(body)) = settle(health, &mut errors, target.id, "outposts/health") {
                connected.push((name.clone(), fresh_instances(&body, now_s)));
            }
        }
        out.extend(outpost_samples(&connected, ts_ms));
    }
    out.push(gauge(&metric(P, "scrape_errors"), f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct AuthentikCollector;

impl AuthentikCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(key) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "authentik expects an API token, configured: {}",
                target.credential
            )));
        };
        http_client(
            target,
            "http",
            DEFAULT_PORT,
            Auth::header("Authorization", format!("Bearer {key}")),
            PRODUCT,
        )
    }
}

#[async_trait]
impl Collector for AuthentikCollector {
    fn kind(&self) -> &str {
        "authentik"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, target, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: Value = Self::client(target)?.get_json("/api/v3/admin/version/").await?;
        Ok(Some("authentik".to_string()))
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

    // Réponses d'un authentik 2025.8.4 : un travailleur en retard d'une version,
    // une synchronisation LDAP en échec, trois avant-postes dont un sans
    // fournisseur et un muet depuis la veille.
    const VERSION: &str = include_str!("testdata/authentik_2025.8.4/version.json");
    const VERSION_CURRENT: &str = include_str!("testdata/authentik_2025.8.4/version_current.json");
    const WORKERS: &str = include_str!("testdata/authentik_2025.8.4/workers.json");
    const TASKS: &str = include_str!("testdata/authentik_2025.8.4/system_tasks.json");
    const OUTPOSTS: &str = include_str!("testdata/authentik_2025.8.4/outposts.json");
    const HEALTH_EMBEDDED: &str =
        include_str!("testdata/authentik_2025.8.4/outpost_health_embedded.json");
    const HEALTH_LDAP: &str = include_str!("testdata/authentik_2025.8.4/outpost_health_ldap.json");
    const USERS: &str = include_str!("testdata/authentik_2025.8.4/users.json");

    // 2026-10-26T09:12:00Z
    const NOW: i64 = 1_793_005_920;

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn la_version_dit_si_une_mise_a_jour_existe() {
        let samples = version_samples(&json(VERSION), 0);
        assert_eq!(value(&samples, "authentik_update_available"), Some(1.0));
        let flag = samples.iter().find(|s| s.metric == "authentik_update_available").unwrap();
        assert_eq!(flag.labels["latest_version"], "2025.10.1");
        let info = samples.iter().find(|s| s.metric == "authentik_version_info").unwrap();
        assert_eq!(info.labels["version"], "2025.8.4");
        let samples = version_samples(&json(VERSION_CURRENT), 0);
        assert_eq!(value(&samples, "authentik_update_available"), Some(0.0));
        // Sans accès sortant, la dernière version est inconnue : aucun verdict.
        let blind = json(r#"{"version_current":"2025.8.4","version_latest_valid":false}"#);
        assert!(value(&version_samples(&blind, 0), "authentik_update_available").is_none());
    }

    #[test]
    fn un_travailleur_en_retard_se_compte() {
        let samples = worker_samples(&json(WORKERS), 0);
        assert_eq!(value(&samples, "authentik_workers"), Some(2.0));
        assert_eq!(value(&samples, "authentik_workers_version_mismatch"), Some(1.0));
        let samples = worker_samples(&json("[]"), 0);
        assert_eq!(value(&samples, "authentik_workers"), Some(0.0));
    }

    #[test]
    fn les_taches_en_echec_se_nomment() {
        let samples = task_samples(&json(TASKS), 0);
        assert_eq!(value(&samples, "authentik_tasks_failed"), Some(1.0));
        assert_eq!(value(&samples, "authentik_tasks_warning"), Some(1.0));
        let named = samples.iter().find(|s| s.metric == "authentik_task_failed").unwrap();
        assert!(named.labels["task"].ends_with("ldap_sync"));
    }

    #[test]
    fn un_avant_poste_muet_est_deconnecte_et_celui_sans_fournisseur_est_ignore() {
        let used = used_outposts(&json(OUTPOSTS));
        let names: Vec<&str> = used.iter().map(|(_, name)| name.as_str()).collect();
        assert_eq!(names, ["authentik Embedded Outpost", "ldap-outpost"]);
        assert_eq!(fresh_instances(&json(HEALTH_EMBEDDED), NOW), 1);
        assert_eq!(fresh_instances(&json(HEALTH_LDAP), NOW), 0);
        let samples =
            outpost_samples(&[("embedded".to_string(), 1), ("ldap-outpost".to_string(), 0)], 0);
        assert_eq!(value(&samples, "authentik_outposts_total"), Some(2.0));
        assert_eq!(value(&samples, "authentik_outposts_disconnected"), Some(1.0));
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, key: &str) -> Target {
        Target {
            id: 8,
            name: "sso".into(),
            address,
            kind: "authentik".into(),
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

    fn app() -> Router {
        Router::new()
            .route(
                "/api/v3/admin/version/",
                get(|headers: HeaderMap| async move {
                    let ok = headers.get("authorization").and_then(|v| v.to_str().ok())
                        == Some("Bearer cle");
                    if ok {
                        (StatusCode::OK, VERSION)
                    } else {
                        (StatusCode::FORBIDDEN, r#"{"detail":"Token invalid/expired"}"#)
                    }
                }),
            )
            .route("/-/health/live/", get(|| async { StatusCode::NO_CONTENT }))
            .route(
                "/-/health/ready/",
                get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "database") }),
            )
            .route("/api/v3/admin/workers/", get(|| async { WORKERS }))
            .route("/api/v3/admin/system_tasks/", get(|| async { TASKS }))
            .route("/api/v3/core/users/", get(|| async { USERS }))
            .route("/api/v3/outposts/instances/", get(|| async { OUTPOSTS }))
            .route(
                "/api/v3/outposts/instances/3f0f5a9e-1111-4a6e-9a0e-aaaaaaaaaaaa/health/",
                get(|| async { HEALTH_EMBEDDED }),
            )
            .route(
                "/api/v3/outposts/instances/3f0f5a9e-2222-4a6e-9a0e-bbbbbbbbbbbb/health/",
                get(|| async { HEALTH_LDAP }),
            )
    }

    #[tokio::test]
    async fn sonde_complete_avec_la_base_indisponible() {
        let base = serve(app()).await;
        let samples = AuthentikCollector::new().probe(&target(base, "cle")).await.unwrap();
        assert_eq!(value(&samples, "authentik_live"), Some(1.0));
        assert_eq!(value(&samples, "authentik_ready"), Some(0.0));
        assert_eq!(value(&samples, "authentik_workers"), Some(2.0));
        assert_eq!(value(&samples, "authentik_tasks_failed"), Some(1.0));
        assert_eq!(value(&samples, "authentik_users"), Some(37.0));
        assert_eq!(value(&samples, "authentik_outposts_total"), Some(2.0));
        // Les santés sont horodatées de 2026 : au moment du test, elles sont
        // toutes anciennes, quel que soit l'avant-poste.
        assert_eq!(value(&samples, "authentik_scrape_errors"), Some(0.0));
    }

    #[tokio::test]
    async fn un_jeton_refuse_est_une_erreur_d_authentification_et_un_hote_eteint_une_panne() {
        let base = serve(app()).await;
        let error = AuthentikCollector::new().probe(&target(base, "faux")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
        let error = AuthentikCollector::new()
            .probe(&target("http://127.0.0.1:1".into(), "cle"))
            .await
            .unwrap_err();
        assert!(error.means_down());
    }
}
