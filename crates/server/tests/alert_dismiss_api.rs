//! Tests d'intégration de l'écartement d'une alerte (« j'ai compris ») : la
//! route, ses droits, la liste qui ne la montre plus, l'annulation, et ce que
//! le moteur en fait (un cycle qui réécrit l'état ne doit pas le perdre, la
//! résolution doit l'effacer, et il survit à un redémarrage).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use dumbmonit_server::alerting::group::AlertOutcome;
use dumbmonit_server::alerting::machine::{AlertState, Phase};
use dumbmonit_server::alerting::model::Severity;
use dumbmonit_server::config::Config;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{api, collectors, db, tsdb};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

const UNREACHABLE_VICTORIA: &str = "http://127.0.0.1:1";
const SECRET: &str = "secret-de-test-suffisamment-long";
const PASSWORD: &str = "mot-de-passe-du-homelab";
const VIEWER_PASSWORD: &str = "mot-de-passe-du-lecteur";

struct TestApp {
    router: Router,
    pool: SqlitePool,
    admin: String,
    _dir: tempfile::TempDir,
}

async fn setup() -> TestApp {
    let dir = tempfile::tempdir().expect("temporary directory");
    let mut config = Config::from_env().expect("default configuration");
    config.data_dir = dir.path().to_path_buf();
    config.setup_code = Some("test-setup-code".into());
    config.victoria_url = Some(UNREACHABLE_VICTORIA.to_string());

    let pool = db::open(&config.database_path()).await.expect("database opened");
    let cipher = db::init_cipher(&pool, SECRET).await.expect("cipher initialised");
    db::alerts::seed_builtin_rules(&pool).await.expect("built-in rules seeded");

    let victoria = tsdb::Victoria::new(UNREACHABLE_VICTORIA).expect("client");
    let sink = tsdb::spawn_writer(victoria.clone(), Duration::from_secs(60));
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    let state = AppState::new(Inner {
        config,
        pool: pool.clone(),
        cipher,
        victoria,
        sink,
        collectors: registry,
    });

    let mut app = TestApp { router: api::router(state), pool, admin: String::new(), _dir: dir };
    let (status, body) = app
        .request(
            "POST",
            "/api/auth/setup",
            Some(json!({ "setup_code": "test-setup-code", "username": "admin", "password": PASSWORD })),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "admin creation: {body}");
    app.admin = app.login("admin", PASSWORD).await;
    app
}

impl TestApp {
    async fn login(&self, username: &str, password: &str) -> String {
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "username": username, "password": password }).to_string()))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let raw = response.headers().get(header::SET_COOKIE).expect("session cookie");
        raw.to_str().unwrap().split(';').next().unwrap().to_string()
    }

    async fn viewer(&self) -> String {
        let (status, body) = self
            .request(
                "POST",
                "/api/users",
                Some(
                    json!({ "username": "viewer", "role": "viewer", "password": VIEWER_PASSWORD }),
                ),
                Some(&self.admin),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "viewer creation: {body}");
        self.login("viewer", VIEWER_PASSWORD).await
    }

    async fn request(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
        cookie: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder =
                builder.header(header::COOKIE, cookie).header("x-requested-with", "DumbMonit");
        }
        let request = match body {
            Some(value) => builder
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let response = self.router.clone().oneshot(request).await.expect("response");
        let status = response.status();
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn create_target(&self, name: &str, address: &str) -> i64 {
        let (status, body) = self
            .request(
                "POST",
                "/api/targets",
                Some(json!({ "name": name, "address": address, "kind": "dummy" })),
                Some(&self.admin),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "target refused: {body}");
        body["id"].as_i64().expect("target id")
    }

    /// Une alerte `firing`, déjà notifiée, telle que le moteur l'aurait écrite.
    fn firing_alert(fingerprint: &str, target_id: i64) -> AlertOutcome {
        let now = Utc::now();
        AlertOutcome {
            fingerprint: fingerprint.to_string(),
            rule_uid: "cpu_high".to_string(),
            rule_name: "CPU".to_string(),
            target_id: Some(target_id),
            target_name: "nas".to_string(),
            series_key: "dumbmonit_cpu_usage_percent{host=\"nas\"}".to_string(),
            labels: BTreeMap::from([
                ("host".to_string(), "nas".to_string()),
                ("target".to_string(), target_id.to_string()),
            ]),
            state: AlertState {
                phase: Phase::Firing,
                condition_since: Some(now),
                firing_since: Some(now),
                last_eval_at: Some(now),
                last_notified_at: Some(now),
                notify_count: 1,
                value: Some(97.5),
                ..AlertState::default()
            },
            severity: Severity::Warning,
            value: Some(97.5),
            score: None,
            unit: "%".to_string(),
            operator: ">".to_string(),
            threshold: 90.0,
            channels: Vec::new(),
            repeat_interval: Some(Duration::from_secs(3600)),
            escalate_after: None,
            unacked_after: None,
            just_transitioned: false,
            recurred: false,
            detail: None,
        }
    }

    async fn alert(&self, fingerprint: &str) -> Value {
        self.listed("/api/alerts", fingerprint).await
    }

    async fn listed(&self, uri: &str, fingerprint: &str) -> Value {
        let (status, body) = self.request("GET", uri, None, Some(&self.admin)).await;
        assert_eq!(status, StatusCode::OK);
        body.as_array()
            .expect("list")
            .iter()
            .find(|alert| alert["fingerprint"] == json!(fingerprint))
            .cloned()
            .unwrap_or(Value::Null)
    }
}

#[tokio::test]
async fn a_dismissed_alert_leaves_the_list_and_comes_back_on_undo() {
    let app = setup().await;
    let nas = app.create_target("nas", "10.0.0.7").await;
    db::alerts::save_states(
        &app.pool,
        &[TestApp::firing_alert("cpu_high@1", nas), TestApp::firing_alert("disk_full@1", nas)],
    )
    .await
    .expect("state saved");

    let before = app.alert("cpu_high@1").await;
    assert_eq!(before["dismissed"], json!(false));
    assert!(before["dismissed_by"].is_null());

    let (status, body) =
        app.request("POST", "/api/alerts/cpu_high@1/dismiss", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["dismissed"], json!(true));
    assert_eq!(body["dismissed_by"], json!("admin"));
    assert!(body["dismissed_at"].is_string());
    assert_eq!(body["phase"], json!("firing"), "the state machine does not move");

    // Hors de la liste active (vue d'ensemble, mur, compteurs la lisent)…
    assert!(app.alert("cpu_high@1").await.is_null(), "hidden from the active alerts");
    assert!(!app.alert("disk_full@1").await.is_null(), "the others stay");
    // … mais retrouvable dans la vue des alertes écartées, pour l'annuler.
    let dismissed = app.listed("/api/alerts?dismissed=true", "cpu_high@1").await;
    assert_eq!(dismissed["dismissed_by"], json!("admin"));
    assert!(app.listed("/api/alerts?dismissed=true", "disk_full@1").await.is_null());
    let filtered =
        app.listed(&format!("/api/alerts?dismissed=true&target_id={nas}"), "cpu_high@1").await;
    assert!(!filtered.is_null(), "the device filter still applies");

    let (status, audit) = app.request("GET", "/api/auth/audit", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        audit.as_array().unwrap().iter().any(|entry| entry["action"] == json!("alert.dismissed")
            && entry["subject"] == json!("cpu_high@1")
            && entry["actor"] == json!("admin")),
        "{audit}"
    );

    // Annulation (« Undo »).
    let (status, body) =
        app.request("DELETE", "/api/alerts/cpu_high@1/dismiss", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["dismissed"], json!(false));
    assert!(body["dismissed_by"].is_null());
    assert_eq!(app.alert("cpu_high@1").await["dismissed"], json!(false), "listed again");
    assert!(app.listed("/api/alerts?dismissed=true", "cpu_high@1").await.is_null());
}

#[tokio::test]
async fn a_viewer_cannot_dismiss() {
    let app = setup().await;
    let nas = app.create_target("nas", "10.0.0.7").await;
    db::alerts::save_states(&app.pool, &[TestApp::firing_alert("cpu_high@1", nas)])
        .await
        .expect("state saved");
    let viewer = app.viewer().await;

    let (status, body) =
        app.request("POST", "/api/alerts/cpu_high@1/dismiss", None, Some(&viewer)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, _) =
        app.request("DELETE", "/api/alerts/cpu_high@1/dismiss", None, Some(&viewer)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app.request("POST", "/api/alerts/cpu_high@1/dismiss", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(app.alert("cpu_high@1").await["dismissed"], json!(false));
}

#[tokio::test]
async fn an_unknown_or_resolved_alert_cannot_be_dismissed() {
    let app = setup().await;
    let nas = app.create_target("nas", "10.0.0.7").await;
    let mut resolved = TestApp::firing_alert("cpu_high@old", nas);
    resolved.state.phase = Phase::Resolved;
    resolved.state.resolved_at = Some(Utc::now());
    db::alerts::save_states(&app.pool, &[resolved]).await.expect("state saved");

    let (status, body) =
        app.request("POST", "/api/alerts/nope@0/dismiss", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, _) =
        app.request("POST", "/api/alerts/cpu_high@old/dismiss", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "a resolved alert has nothing to dismiss");
    let (status, _) =
        app.request("DELETE", "/api/alerts/nope@0/dismiss", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_engine_keeps_a_dismissal_until_resolution_and_it_survives_a_restart() {
    let app = setup().await;
    let nas = app.create_target("nas", "10.0.0.7").await;
    db::alerts::save_states(&app.pool, &[TestApp::firing_alert("cpu_high@1", nas)])
        .await
        .expect("state saved");
    let (status, _) =
        app.request("POST", "/api/alerts/cpu_high@1/dismiss", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::OK);

    // Un cycle qui a chargé l'état *avant* le clic réécrit la ligne sans
    // écartement : celui de la base doit survivre.
    let mut still_firing = TestApp::firing_alert("cpu_high@1", nas);
    still_firing.state.last_eval_at = Some(Utc::now());
    db::alerts::save_states(&app.pool, &[still_firing]).await.expect("state rewritten");
    assert!(app.alert("cpu_high@1").await.is_null(), "still hidden after an engine write");

    // Ce que le moteur relit au démarrage : l'écartement est là.
    let stored = db::alerts::load_states(&app.pool).await.expect("states loaded");
    let state = &stored.iter().find(|a| a.fingerprint == "cpu_high@1").unwrap().state;
    assert!(state.is_dismissed());
    assert_eq!(state.dismissed_by.as_deref(), Some("admin"));

    // Résolution : la ligne perd son écartement, la prochaine occurrence alertera.
    let mut resolved = TestApp::firing_alert("cpu_high@1", nas);
    resolved.state.phase = Phase::Resolved;
    resolved.state.resolved_at = Some(Utc::now());
    resolved.state.last_eval_at = Some(Utc::now());
    db::alerts::save_states(&app.pool, &[resolved]).await.expect("state resolved");
    let stored = db::alerts::load_states(&app.pool).await.expect("states loaded");
    let state = &stored.iter().find(|a| a.fingerprint == "cpu_high@1").unwrap().state;
    assert_eq!(state.dismissed_at, None, "resolved: cleared in the database too");
    assert_eq!(state.dismissed_by, None);

    let mut back = TestApp::firing_alert("cpu_high@1", nas);
    back.state.notify_count = 0;
    back.state.last_eval_at = Some(Utc::now());
    db::alerts::save_states(&app.pool, &[back]).await.expect("state fired again");
    assert_eq!(app.alert("cpu_high@1").await["dismissed"], json!(false), "a new occurrence shows");
}
