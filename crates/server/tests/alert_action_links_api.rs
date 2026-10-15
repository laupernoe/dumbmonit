//! Tests d'intégration des liens d'action des notifications : un lien signé
//! acquitte ou met en pause, une fois, sans session — et rien d'autre.
//!
//! Le côté sécurité est l'essentiel : jeton forgé, altéré, expiré, d'un autre
//! secret, rejoué, appel d'un autre site, fonction coupée — tous refusés, et
//! refusés de la même façon.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::{TimeDelta, Utc};
use dumbmonit_server::alerting::group::AlertOutcome;
use dumbmonit_server::alerting::machine::{AlertState, Phase};
use dumbmonit_server::alerting::model::Severity;
use dumbmonit_server::config::Config;
use dumbmonit_server::crypto::Cipher;
use dumbmonit_server::notify::action_link::{Action, ActionKey, AlertRef, Claims};
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{api, collectors, db, tsdb};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

const UNREACHABLE_VICTORIA: &str = "http://127.0.0.1:1";
const SECRET: &str = "secret-de-test-suffisamment-long";
const PASSWORD: &str = "mot-de-passe-du-homelab";

struct TestApp {
    router: Router,
    pool: SqlitePool,
    key: ActionKey,
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
    let key = ActionKey::from_cipher(&cipher);
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

    let mut app =
        TestApp { router: api::router(state), pool, key, admin: String::new(), _dir: dir };
    let (status, body) = app
        .request(
            "POST",
            "/api/auth/setup",
            Some(json!({ "setup_code": "test-setup-code", "username": "admin", "password": PASSWORD })),
            Some(""),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "admin creation: {body}");
    app.admin = app.login().await;
    let (status, body) = app
        .request(
            "PUT",
            "/api/notify/policy",
            Some(json!({ "public_url": "https://monit.example.org/" })),
            Some(&app.admin.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["action_links"], json!(true), "on by default");
    app
}

impl TestApp {
    async fn login(&self) -> String {
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "username": "admin", "password": PASSWORD }).to_string()))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let raw = response.headers().get(header::SET_COOKIE).expect("session cookie");
        raw.to_str().unwrap().split(';').next().unwrap().to_string()
    }

    /// `cookie` : `None` = aucun en-tête (pas même l'anti-CSRF), `Some("")` =
    /// l'en-tête anti-CSRF sans session (la page publique), sinon la session.
    async fn request(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
        cookie: Option<&str>,
    ) -> (StatusCode, Value) {
        self.request_with(method, uri, body, cookie, &[]).await
    }

    async fn request_with(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
        cookie: Option<&str>,
        extra: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder = builder.header("x-requested-with", "DumbMonit");
            if !cookie.is_empty() {
                builder = builder.header(header::COOKIE, cookie);
            }
        }
        for (name, value) in extra {
            builder = builder.header(*name, *value);
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
        let cache = response.headers().get(header::CACHE_CONTROL).cloned();
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        if status == StatusCode::OK && uri.starts_with("/api/public/alert-action") {
            assert_eq!(cache.as_ref().and_then(|v| v.to_str().ok()), Some("no-store"));
        }
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn inspect(&self, token: &str) -> (StatusCode, Value) {
        self.request(
            "POST",
            "/api/public/alert-action/inspect",
            Some(json!({ "token": token })),
            Some(""),
        )
        .await
    }

    async fn execute(&self, token: &str) -> (StatusCode, Value) {
        self.request("POST", "/api/public/alert-action", Some(json!({ "token": token })), Some(""))
            .await
    }

    /// Lien tel que le moteur l'aurait signé : chaque alerte avec l'épisode
    /// en cours (un lien pour une alerte absente n'a pas d'épisode).
    async fn token(&self, action: Action, fingerprints: &[&str]) -> String {
        let mut alerts = Vec::new();
        for fingerprint in fingerprints {
            let since = db::alerts::get_active(&self.pool, fingerprint)
                .await
                .expect("read")
                .and_then(|alert| alert.state.firing_since);
            alerts.push(AlertRef::new(*fingerprint, since));
        }
        self.key.sign(&Claims::new(action, alerts, Utc::now()))
    }

    async fn create_target(&self, name: &str) -> i64 {
        let (status, body) = self
            .request(
                "POST",
                "/api/targets",
                Some(json!({ "name": name, "address": "10.0.0.7", "kind": "dummy" })),
                Some(&self.admin),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "target refused: {body}");
        body["id"].as_i64().expect("target id")
    }

    async fn fire(&self, fingerprint: &str, target_id: i64) {
        let now = Utc::now();
        let outcome = AlertOutcome {
            fingerprint: fingerprint.to_string(),
            rule_uid: "cpu_high".to_string(),
            rule_name: "CPU".to_string(),
            target_id: Some(target_id),
            target_name: "nas".to_string(),
            series_key: format!("dumbmonit_cpu_usage_percent{{host=\"{fingerprint}\"}}"),
            labels: BTreeMap::from([("target".to_string(), target_id.to_string())]),
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
        };
        db::alerts::save_states(&self.pool, &[outcome]).await.expect("state saved");
    }

    async fn alert(&self, fingerprint: &str) -> Value {
        let (status, body) = self.request("GET", "/api/alerts", None, Some(&self.admin)).await;
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
async fn an_ack_link_shows_then_acknowledges_once_without_a_session() {
    let app = setup().await;
    let nas = app.create_target("nas").await;
    app.fire("cpu_high@1", nas).await;
    let token = app.token(Action::Ack, &["cpu_high@1"]).await;

    // Afficher ne fait rien.
    let (status, view) = app.inspect(&token).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["action"], json!("ack"));
    assert_eq!(view["used"], json!(false));
    assert_eq!(view["alerts"][0]["target_name"], json!("nas"));
    assert_eq!(view["alerts"][0]["acked"], json!(false));
    assert!(view["alerts"][0].get("fingerprint").is_none(), "nothing more than needed");
    assert_eq!(app.alert("cpu_high@1").await["acked"], json!(false));

    // Exécuter acquitte, au nom du lien, jusqu'à la résolution (plafond 30 j).
    let (status, done) = app.execute(&token).await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(done["acked"], json!(1));
    let alert = app.alert("cpu_high@1").await;
    assert_eq!(alert["acked"], json!(true));
    assert_eq!(alert["acked_by"], json!("via notification link"));
    let until: chrono::DateTime<Utc> = alert["acked_until"].as_str().unwrap().parse().unwrap();
    assert!(until > Utc::now() + TimeDelta::days(29), "until resolved: {until}");

    // Journalisé.
    let (_, audit) = app.request("GET", "/api/auth/audit", None, Some(&app.admin)).await;
    assert!(
        audit.as_array().unwrap().iter().any(|entry| entry["action"] == json!("alert.acked")
            && entry["subject"] == json!("cpu_high@1")
            && entry["actor"] == json!("via notification link")),
        "{audit}"
    );

    // Une seule fois.
    let (status, again) = app.execute(&token).await;
    assert_eq!(status, StatusCode::CONFLICT, "{again}");
    let (status, view) = app.inspect(&token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["used"], json!(true));
}

#[tokio::test]
async fn a_snooze_link_pauses_for_an_hour_and_never_shortens_a_longer_ack() {
    let app = setup().await;
    let nas = app.create_target("nas").await;
    app.fire("cpu_high@1", nas).await;
    app.fire("cpu_high@2", nas).await;

    let (status, done) =
        app.execute(&app.token(Action::Snooze, &["cpu_high@1", "cpu_high@2"]).await).await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(done["acked"], json!(2), "one link covers the whole message");
    let until: chrono::DateTime<Utc> =
        app.alert("cpu_high@1").await["acked_until"].as_str().unwrap().parse().unwrap();
    assert!((until - (Utc::now() + TimeDelta::hours(1))).abs() < TimeDelta::minutes(1));

    // Acquittée jusqu'à la résolution, puis « snooze » : l'acquittement long reste.
    app.execute(&app.token(Action::Ack, &["cpu_high@2"]).await).await;
    let (status, done) = app.execute(&app.token(Action::Snooze, &["cpu_high@2"]).await).await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(done["acked"], json!(0));
    let until: chrono::DateTime<Utc> =
        app.alert("cpu_high@2").await["acked_until"].as_str().unwrap().parse().unwrap();
    assert!(until > Utc::now() + TimeDelta::days(29));
}

#[tokio::test]
async fn refused_tokens_all_look_the_same() {
    let app = setup().await;
    let nas = app.create_target("nas").await;
    app.fire("cpu_high@1", nas).await;

    let valid = app.token(Action::Ack, &["cpu_high@1"]).await;
    let other_secret = {
        let cipher =
            Cipher::derive("un-tout-autre-secret-assez-long", b"un-sel-quelconque").unwrap();
        ActionKey::from_cipher(&cipher).sign(&Claims::new(
            Action::Ack,
            vec![AlertRef::from("cpu_high@1")],
            Utc::now(),
        ))
    };
    let expired = {
        let mut claims =
            Claims::new(Action::Ack, vec![AlertRef::from("cpu_high@1")], Utc::now());
        claims.expires_at = Utc::now() - TimeDelta::seconds(1);
        app.key.sign(&claims)
    };
    let (payload, tag) = valid.split_once('.').unwrap();
    let flipped_tag = format!("{payload}.{}{}", &tag[..tag.len() - 2], "AA");
    // Un lien vers une autre alerte, forgé en recollant l'étiquette d'un vrai.
    let other = app.token(Action::Ack, &["cpu_high@9"]).await;
    let spliced = format!("{}.{tag}", other.split_once('.').unwrap().0);

    let mut messages = Vec::new();
    for token in [other_secret, expired, flipped_tag, spliced, "x".into(), String::new()] {
        for (status, body) in [app.inspect(&token).await, app.execute(&token).await] {
            assert_eq!(status, StatusCode::NOT_FOUND, "{token}: {body}");
            messages.push(body["error"].as_str().unwrap_or_default().to_string());
        }
    }
    messages.dedup();
    assert_eq!(messages, vec!["This link is invalid or has expired."], "no oracle");
    assert_eq!(app.alert("cpu_high@1").await["acked"], json!(false), "nothing happened");
}

#[tokio::test]
async fn only_the_instance_page_may_call_and_never_with_a_cross_site_request() {
    let app = setup().await;
    let nas = app.create_target("nas").await;
    app.fire("cpu_high@1", nas).await;
    let token = app.token(Action::Ack, &["cpu_high@1"]).await;
    let body = json!({ "token": token });

    // Sans l'en-tête anti-CSRF : refusé (un formulaire tiers ne peut pas le poser).
    let (status, _) =
        app.request("POST", "/api/public/alert-action", Some(body.clone()), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Avec, mais déclaré inter-sites par le navigateur : refusé aussi.
    let (status, _) = app
        .request_with(
            "POST",
            "/api/public/alert-action",
            Some(body.clone()),
            Some(""),
            &[("sec-fetch-site", "cross-site")],
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Un GET (préchargeur de liens) n'existe pas.
    let (status, _) = app.request("GET", "/api/public/alert-action", None, Some("")).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(app.alert("cpu_high@1").await["acked"], json!(false));

    let (status, _) = app
        .request_with(
            "POST",
            "/api/public/alert-action",
            Some(body),
            Some(""),
            &[("sec-fetch-site", "same-origin")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn switching_links_off_revokes_those_already_sent() {
    let app = setup().await;
    let nas = app.create_target("nas").await;
    app.fire("cpu_high@1", nas).await;
    let token = app.token(Action::Ack, &["cpu_high@1"]).await;

    let (status, policy) = app
        .request(
            "PUT",
            "/api/notify/policy",
            Some(json!({ "action_links": false })),
            Some(&app.admin),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{policy}");
    assert_eq!(policy["action_links"], json!(false));
    let (status, _) = app.execute(&token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Rallumées, mais sans URL publique : toujours rien.
    app.request(
        "PUT",
        "/api/notify/policy",
        Some(json!({ "action_links": true, "public_url": "" })),
        Some(&app.admin),
    )
    .await;
    let (status, _) = app.execute(&token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(app.alert("cpu_high@1").await["acked"], json!(false));
}

#[tokio::test]
async fn a_link_never_acknowledges_a_later_episode_of_the_same_alert() {
    let app = setup().await;
    let nas = app.create_target("nas").await;
    app.fire("cpu_high@1", nas).await;
    let token = app.token(Action::Ack, &["cpu_high@1"]).await;

    // Résolue puis redéclenchée : même empreinte, autre épisode.
    let later =
        (Utc::now() + TimeDelta::minutes(10)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    sqlx::query("UPDATE alert_state SET firing_since = ? WHERE fingerprint = 'cpu_high@1'")
        .bind(later)
        .execute(&app.pool)
        .await
        .expect("re-fired");

    let (status, view) = app.inspect(&token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["alerts"], json!([]), "the new episode is not the link's");
    let (status, body) = app.execute(&token).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(app.alert("cpu_high@1").await["acked"], json!(false));

    // Le lien de l'épisode en cours, lui, fonctionne.
    let (status, _) = app.execute(&app.token(Action::Ack, &["cpu_high@1"]).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(app.alert("cpu_high@1").await["acked"], json!(true));
}

#[tokio::test]
async fn a_link_to_resolved_alerts_does_nothing_and_stays_usable() {
    let app = setup().await;
    let token = app.token(Action::Ack, &["cpu_high@404"]).await;
    let (status, view) = app.inspect(&token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["alerts"], json!([]));
    let (status, body) = app.execute(&token).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(body["error"].as_str().unwrap().contains("no longer active"));
    let (_, view) = app.inspect(&token).await;
    assert_eq!(view["used"], json!(false), "not consumed by a no-op");
}

#[tokio::test]
async fn the_channel_catalogue_says_how_each_kind_shows_the_actions() {
    let app = setup().await;
    let (status, kinds) = app.request("GET", "/api/notify/kinds", None, Some(&app.admin)).await;
    assert_eq!(status, StatusCode::OK);
    let style = |kind: &str| {
        kinds.as_array().unwrap().iter().find(|k| k["kind"] == json!(kind)).unwrap()["actions"]
            .clone()
    };
    assert_eq!(style("ntfy"), json!("buttons"));
    assert_eq!(style("telegram"), json!("buttons"));
    assert_eq!(style("smtp"), json!("links"));
    assert_eq!(style("pagerduty"), json!("none"));
}
