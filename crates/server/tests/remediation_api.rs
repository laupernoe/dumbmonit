//! Procédures et remédiation : droits, validation, rejeu, armement en deux
//! étapes, automate et sauvegarde.

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::{TimeDelta, Utc};
use common::TestApp;
use dumbmonit_proto::{CommandReport, CommandStatus};
use dumbmonit_server::alerting::group::AlertOutcome;
use dumbmonit_server::alerting::machine::{AlertState, Phase};
use dumbmonit_server::alerting::model::Severity;
use dumbmonit_server::backup::{ExportOptions, collect, restore};
use dumbmonit_server::collectors::agent::commands;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{collectors, db, remediation, tsdb};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

const OPERATOR_PASSWORD: &str = "mot-de-passe-de-l-operateur";

struct Lab {
    app: TestApp,
    pool: SqlitePool,
    admin: String,
    operator: String,
    viewer: String,
    /// Machine équipée de l'agent.
    host: i64,
    fingerprint: String,
}

async fn lab() -> Lab {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    db::alerts::seed_builtin_rules(&pool).await.expect("règles intégrées");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    let reply = app
        .post(
            "/api/users",
            json!({ "username": "ops", "role": "operator", "password": OPERATOR_PASSWORD }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let operator = app.login_as("ops", OPERATOR_PASSWORD).await.cookie();
    let viewer = app.viewer_cookie(&admin).await;

    let host: i64 = sqlx::query_scalar(
        "INSERT INTO targets (name, address, kind, interval_secs, enabled, tags, group_name)
         VALUES ('web-1', 'agent:web-1', 'agent', 60, 1, '{}', 'Production') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agent_hosts (target_id, agent_key, hostname, commands_enabled, remediation)
         VALUES (?, 'key-web-1', 'web-1', 1, ?)",
    )
    .bind(host)
    .bind(
        json!({
            "enabled": true,
            "services": ["nginx"],
            "scripts": [{ "name": "flush", "params": { "level": ["low", "high"] } }]
        })
        .to_string(),
    )
    .execute(&pool)
    .await
    .unwrap();
    let fingerprint = format!("host_down@{host}");
    db::alerts::save_states(&pool, &[firing(&fingerprint, host, 30, |_| {})]).await.unwrap();
    Lab { app, pool, admin, operator, viewer, host, fingerprint }
}

fn firing(
    fingerprint: &str,
    target: i64,
    minutes: i64,
    tweak: impl FnOnce(&mut AlertState),
) -> AlertOutcome {
    let since = Utc::now() - TimeDelta::minutes(minutes);
    let mut state = AlertState {
        phase: Phase::Firing,
        condition_since: Some(since),
        firing_since: Some(since),
        last_eval_at: Some(Utc::now()),
        ..AlertState::default()
    };
    tweak(&mut state);
    AlertOutcome {
        fingerprint: fingerprint.to_string(),
        rule_uid: "host_down".to_string(),
        rule_name: "Host down".to_string(),
        target_id: Some(target),
        target_name: "web-1".to_string(),
        series_key: format!("up{{target=\"{target}\"}}"),
        labels: BTreeMap::from([("target".to_string(), target.to_string())]),
        state,
        severity: Severity::Critical,
        value: Some(0.0),
        score: None,
        unit: String::new(),
        operator: "<".to_string(),
        threshold: 1.0,
        channels: Vec::new(),
        repeat_interval: None,
        escalate_after: None,
        unacked_after: None,
        just_transitioned: false,
        recurred: false,
        detail: None,
    }
}

async fn create_action(lab: &Lab, body: Value) -> Value {
    let reply = lab.app.post("/api/remediation/actions", body, Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body
}

/// Requête brute, pour choisir les en-têtes (CSRF, jeton).
async fn raw(lab: &Lab, method: &str, uri: &str, headers: &[(&str, String)]) -> StatusCode {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, value);
    }
    let request =
        builder.header(header::CONTENT_TYPE, "application/json").body(Body::from("{}")).unwrap();
    let response = lab.app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let _ = response.into_body().collect().await;
    status
}

#[tokio::test]
async fn runbooks_are_admin_written_and_matched_by_rule_folder_and_kind() {
    let lab = lab().await;
    let body = json!({
        "title": "Web server down",
        "body": "<script>alert(1)</script>\n1. Check nginx",
        "links": [
            { "scope": "rule", "value": "host_down" },
            { "scope": "group", "value": "Production" }
        ]
    });
    let reply = lab.app.post("/api/runbooks", body.clone(), Some(&lab.viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let reply = lab.app.post("/api/runbooks", body.clone(), Some(&lab.operator)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "operators do not write runbooks");
    let reply = lab.app.post("/api/runbooks", body, Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    // Stocké tel quel : c'est l'interface qui échappe le HTML.
    assert!(reply.body["body"].as_str().unwrap().starts_with("<script>"));
    let other = json!({ "title": "Agents", "links": [{ "scope": "kind", "value": "agent" }] });
    assert_eq!(
        lab.app.post("/api/runbooks", other, Some(&lab.admin)).await.status,
        StatusCode::CREATED
    );
    let bad = json!({ "title": "x", "links": [{ "scope": "shell", "value": "x" }] });
    assert_eq!(
        lab.app.post("/api/runbooks", bad, Some(&lab.admin)).await.status,
        StatusCode::BAD_REQUEST
    );

    let uri = format!("/api/runbooks/match?target_id={}", lab.host);
    let found = lab.app.get(&uri, Some(&lab.viewer)).await;
    assert_eq!(found.body.as_array().unwrap().len(), 2, "folder + kind: {}", found.body);
    let found = lab.app.get("/api/runbooks/match?rule_uid=host_down", Some(&lab.viewer)).await;
    assert_eq!(found.body.as_array().unwrap().len(), 1);

    let context = lab
        .app
        .get(&format!("/api/remediation/alert/{}", lab.fingerprint), Some(&lab.viewer))
        .await;
    assert_eq!(context.status, StatusCode::OK, "{}", context.body);
    assert_eq!(context.body["runbooks"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn the_catalogue_is_closed_and_names_cannot_traverse_paths() {
    let lab = lab().await;
    for body in [
        json!({ "name": "x", "kind": "shell", "params": { "command": "rm -rf /" } }),
        json!({ "name": "x", "kind": "script.run", "params": { "script": "../../bin/sh" } }),
        json!({ "name": "x", "kind": "script.run", "params": { "script": "flush", "command": "id" } }),
        json!({ "name": "x", "kind": "script.run",
                "params": { "script": "flush", "params": { "level": "high; reboot" } } }),
        json!({ "name": "x", "kind": "service.restart", "params": { "service": "--all" } }),
        json!({ "name": "x", "kind": "webhook", "params": { "url": "https://u:p@hooks.lan/x" } }),
        json!({ "name": "x", "kind": "webhook", "params": { "url": "file:///etc/passwd" } }),
        json!({ "name": "x", "kind": "service.restart", "params": { "service": "nginx" },
                "secrets": { "token": "t" } }),
    ] {
        let reply = lab.app.post("/api/remediation/actions", body.clone(), Some(&lab.admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{body} -> {}", reply.body);
    }
}

#[tokio::test]
async fn secrets_are_never_returned() {
    let lab = lab().await;
    let action = create_action(
        &lab,
        json!({ "name": "hook", "kind": "webhook", "params": { "url": "http://127.0.0.1:9/x" },
                "secrets": { "token": "very-secret-token" } }),
    )
    .await;
    assert_eq!(action["has_secret"], true);
    let listed = lab.app.get("/api/remediation/actions", Some(&lab.admin)).await;
    assert!(!listed.body.to_string().contains("very-secret-token"));
    let backup =
        collect(&lab.pool, &cipher(&lab.pool).await, ExportOptions::default()).await.unwrap();
    assert_eq!(backup.remediation_actions[0].secrets["token"], "very-secret-token");
}

async fn cipher(pool: &SqlitePool) -> dumbmonit_server::crypto::Cipher {
    db::init_cipher(pool, "secret-de-test-suffisamment-long").await.unwrap()
}

#[tokio::test]
async fn operators_run_from_an_alert_viewers_and_tokens_cannot() {
    let lab = lab().await;
    let action = create_action(
        &lab,
        json!({ "name": "restart nginx", "kind": "service.restart", "params": { "service": "nginx" } }),
    )
    .await;
    let uri = format!("/api/remediation/actions/{}/run", action["id"]);
    let from_alert = json!({ "fingerprint": lab.fingerprint });

    let reply = lab.app.post(&uri, from_alert.clone(), Some(&lab.viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "viewer");
    // Sans alerte, un opérateur ne choisit pas la machine.
    let reply = lab.app.post(&uri, json!({ "target_id": lab.host }), Some(&lab.operator)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "operator without alert: {}", reply.body);
    // Un champ inconnu est refusé.
    let reply = lab.app.post(&uri, json!({ "command": "id" }), Some(&lab.operator)).await;
    assert!(reply.status.is_client_error());

    // CSRF : une écriture par cookie sans l'en-tête de l'interface est refusée.
    let status = raw(&lab, "POST", &uri, &[("cookie", lab.operator.clone())]).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "missing X-Requested-With");

    // Un jeton d'API, même `write`, n'agit pas sur les machines.
    let token = lab
        .app
        .post("/api/tokens", json!({ "name": "automation", "scope": "write" }), Some(&lab.admin))
        .await;
    let secret = token.body["secret"].as_str().unwrap().to_string();
    let status = raw(&lab, "POST", &uri, &[("authorization", format!("Bearer {secret}"))]).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "API token");
    let status = raw(
        &lab,
        "GET",
        "/api/remediation/actions",
        &[("authorization", format!("Bearer {secret}"))],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "API token read");

    let reply = lab.app.post(&uri, from_alert.clone(), Some(&lab.operator)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["status"], "queued");
    assert_eq!(reply.body["requested_by"], "ops");
    assert_eq!(reply.body["target_id"], lab.host);

    // La demande remise à l'agent porte un nonce et une échéance bornée.
    let pending = commands::pending_for_target(&lab.pool, lab.host).await.unwrap();
    assert_eq!(pending.len(), 1);
    let args = &pending[0].args;
    assert_eq!(pending[0].kind, "service.restart");
    assert_eq!(args["name"], "nginx");
    assert_eq!(args["nonce"].as_str().unwrap().len(), 32);
    let expires = args["expires_at_ms"].as_i64().unwrap();
    let now = Utc::now().timestamp_millis();
    assert!(expires > now && expires <= now + 600_000 + 1000);

    // Pas deux demandes identiques en file.
    let again = lab.app.post(&uri, from_alert, Some(&lab.admin)).await;
    assert_eq!(again.status, StatusCode::CONFLICT, "{}", again.body);

    // Le compte rendu de l'agent se reporte sur le journal.
    let report = CommandReport { status: CommandStatus::Done, result: "state: active".into() };
    commands::report(&lab.pool, lab.host, pending[0].id, &report).await.unwrap();
    let runs = lab.app.get("/api/remediation/runs", Some(&lab.viewer)).await;
    assert_eq!(runs.body[0]["status"], "done", "{}", runs.body);
    assert_eq!(runs.body[0]["output"], "state: active");
}

#[tokio::test]
async fn the_server_refuses_what_the_agent_does_not_declare() {
    let lab = lab().await;
    let ssh = create_action(
        &lab,
        json!({ "name": "restart ssh", "kind": "service.restart", "params": { "service": "sshd" } }),
    )
    .await;
    let script = create_action(
        &lab,
        json!({ "name": "flush max", "kind": "script.run",
                "params": { "script": "flush", "params": { "level": "max" } } }),
    )
    .await;
    for action in [ssh, script] {
        let uri = format!("/api/remediation/actions/{}/run", action["id"]);
        let reply =
            lab.app.post(&uri, json!({ "fingerprint": lab.fingerprint }), Some(&lab.admin)).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
        assert_eq!(reply.body["status"], "failed");
        assert!(reply.body["output"].as_str().unwrap().starts_with("Refused before sending"));
    }
    assert!(commands::pending_for_target(&lab.pool, lab.host).await.unwrap().is_empty());

    // Actions désactivées sur la machine : rien ne part.
    sqlx::query("UPDATE agent_hosts SET remediation = '{\"enabled\":false}'")
        .execute(&lab.pool)
        .await
        .unwrap();
    let nginx = create_action(
        &lab,
        json!({ "name": "restart nginx", "kind": "service.restart", "params": { "service": "nginx" } }),
    )
    .await;
    let uri = format!("/api/remediation/actions/{}/run", nginx["id"]);
    let reply =
        lab.app.post(&uri, json!({ "fingerprint": lab.fingerprint }), Some(&lab.admin)).await;
    assert_eq!(reply.body["status"], "failed", "{}", reply.body);
    assert!(reply.body["output"].as_str().unwrap().contains("actions: false"));
}

#[tokio::test]
async fn runs_are_bounded_per_minute() {
    let lab = lab().await;
    let action = create_action(
        &lab,
        json!({ "name": "hook", "kind": "webhook", "params": { "url": "http://127.0.0.1:9/x" } }),
    )
    .await;
    for _ in 0..remediation::exec::MAX_RUNS_PER_MINUTE {
        sqlx::query(
            "INSERT INTO remediation_runs (action_name, kind, origin, requested_by, status)
             VALUES ('x', 'webhook', 'manual', 'admin', 'done')",
        )
        .execute(&lab.pool)
        .await
        .unwrap();
    }
    let uri = format!("/api/remediation/actions/{}/run", action["id"]);
    let reply = lab.app.post(&uri, json!({ "target_id": lab.host }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS, "{}", reply.body);
}

#[tokio::test]
async fn arming_a_policy_takes_two_steps_by_the_same_admin() {
    let lab = lab().await;
    let action = create_action(
        &lab,
        json!({ "name": "restart nginx", "kind": "service.restart", "params": { "service": "nginx" } }),
    )
    .await;
    let reply = lab
        .app
        .post(
            "/api/remediation/policies",
            json!({ "action_id": action["id"], "rule_uid": "host_down", "after_minutes": 5 }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["enabled"], false, "born disarmed");
    let id = reply.body["id"].as_i64().unwrap();
    let arm = format!("/api/remediation/policies/{id}/arm");
    let confirm = format!("/api/remediation/policies/{id}/confirm");

    assert_eq!(
        lab.app.post(&arm, json!({}), Some(&lab.operator)).await.status,
        StatusCode::FORBIDDEN
    );

    // Jeton faux : refusé, et le vrai est consommé avec lui.
    let challenge = lab.app.post(&arm, json!({}), Some(&lab.admin)).await;
    assert_eq!(challenge.status, StatusCode::OK, "{}", challenge.body);
    let token = challenge.body["token"].as_str().unwrap().to_string();
    assert!(challenge.body["summary"].as_str().unwrap().contains("restart nginx"));
    let wrong = lab.app.post(&confirm, json!({ "token": "0000" }), Some(&lab.admin)).await;
    assert_eq!(wrong.status, StatusCode::FORBIDDEN);
    let late = lab.app.post(&confirm, json!({ "token": token }), Some(&lab.admin)).await;
    assert_eq!(late.status, StatusCode::FORBIDDEN, "single use");

    // Un autre administrateur ne confirme pas à la place du premier.
    lab.app
        .post(
            "/api/users",
            json!({ "username": "root2", "role": "admin", "password": "un-autre-mot-de-passe" }),
            Some(&lab.admin),
        )
        .await;
    let other = lab.app.login_as("root2", "un-autre-mot-de-passe").await.cookie();
    let challenge = lab.app.post(&arm, json!({}), Some(&lab.admin)).await;
    let token = challenge.body["token"].as_str().unwrap().to_string();
    let stolen = lab.app.post(&confirm, json!({ "token": token }), Some(&other)).await;
    assert_eq!(stolen.status, StatusCode::FORBIDDEN);

    let challenge = lab.app.post(&arm, json!({}), Some(&lab.admin)).await;
    let token = challenge.body["token"].as_str().unwrap().to_string();
    let armed = lab.app.post(&confirm, json!({ "token": token }), Some(&lab.admin)).await;
    assert_eq!(armed.status, StatusCode::OK, "{}", armed.body);
    assert_eq!(armed.body["enabled"], true);
    assert_eq!(armed.body["armed_by"], "admin");

    // Modifier l'action désarme la politique.
    let reply = lab
        .app
        .put(
            &format!("/api/remediation/actions/{}", action["id"]),
            json!({ "name": "restart nginx", "kind": "service.restart",
                    "params": { "service": "php-fpm" } }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let policies = lab.app.get("/api/remediation/policies", Some(&lab.admin)).await;
    assert_eq!(policies.body[0]["enabled"], false, "{}", policies.body);
}

async fn state_for(pool: &SqlitePool) -> AppState {
    let dir = tempfile::tempdir().unwrap();
    let config = common::base_config(dir.path());
    let victoria = tsdb::Victoria::new(common::UNREACHABLE_VICTORIA.to_string()).unwrap();
    let sink = tsdb::spawn_writer(victoria.clone(), Duration::from_secs(60));
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    AppState::new(Inner {
        config,
        pool: pool.clone(),
        cipher: cipher(pool).await,
        victoria,
        sink,
        collectors: registry,
    })
}

async fn armed_policy(lab: &Lab, action: i64, extra: Value) -> i64 {
    let mut body = json!({ "action_id": action, "rule_uid": "host_down", "after_minutes": 10,
                           "max_attempts": 3, "window_minutes": 60 });
    body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    let reply = lab.app.post("/api/remediation/policies", body, Some(&lab.admin)).await;
    let id = reply.body["id"].as_i64().unwrap();
    let challenge = lab
        .app
        .post(&format!("/api/remediation/policies/{id}/arm"), json!({}), Some(&lab.admin))
        .await;
    let token = challenge.body["token"].as_str().unwrap().to_string();
    let armed = lab
        .app
        .post(
            &format!("/api/remediation/policies/{id}/confirm"),
            json!({ "token": token }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(armed.status, StatusCode::OK);
    id
}

#[tokio::test]
async fn the_automaton_respects_its_guards_and_stops_after_two_failures() {
    let lab = lab().await;
    let state = state_for(&lab.pool).await;
    let http = reqwest::Client::new();
    // Un webhook vers un port fermé : chaque tentative échoue.
    let action = create_action(
        &lab,
        json!({ "name": "hook", "kind": "webhook", "params": { "url": "http://127.0.0.1:9/x" } }),
    )
    .await;
    let policy = armed_policy(&lab, action["id"].as_i64().unwrap(), json!({})).await;

    // En maintenance : rien.
    let fp = lab.fingerprint.clone();
    db::alerts::save_states(&lab.pool, &[firing(&fp, lab.host, 30, |s| s.silenced = true)])
        .await
        .unwrap();
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 0, "silenced");
    // Parent en panne : rien.
    db::alerts::save_states(&lab.pool, &[firing(&fp, lab.host, 30, |s| s.suppressed = true)])
        .await
        .unwrap();
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 0, "suppressed");
    // Pas encore assez longtemps : rien.
    db::alerts::save_states(&lab.pool, &[firing(&fp, lab.host, 2, |_| {})]).await.unwrap();
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 0, "too early");

    db::alerts::save_states(&lab.pool, &[firing(&fp, lab.host, 30, |_| {})]).await.unwrap();
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 1, "first attempt");
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 1, "second attempt");
    // Deux échecs de suite : la politique s'arrête.
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 0);
    let policies = lab.app.get("/api/remediation/policies", Some(&lab.admin)).await;
    let record = &policies.body[0];
    assert_eq!(record["id"], policy);
    assert!(record["halted_at"].is_string(), "{record}");
    assert_eq!(record["enabled"], false);
    let runs =
        lab.app.get(&format!("/api/remediation/runs?policy_id={policy}"), Some(&lab.admin)).await;
    assert_eq!(runs.body.as_array().unwrap().len(), 2);
    assert_eq!(runs.body[0]["trigger"], "auto");
    assert_eq!(runs.body[0]["requested_by"], format!("policy:{policy}"));
}

#[tokio::test]
async fn the_automaton_caps_attempts_per_window() {
    let lab = lab().await;
    let state = state_for(&lab.pool).await;
    let http = reqwest::Client::new();
    let action = create_action(
        &lab,
        json!({ "name": "restart nginx", "kind": "service.restart", "params": { "service": "nginx" } }),
    )
    .await;
    armed_policy(&lab, action["id"].as_i64().unwrap(), json!({ "max_attempts": 1 })).await;
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 1);
    // L'agent a réussi ; l'alerte sonne toujours, mais le quota est atteint.
    let pending = commands::pending_for_target(&lab.pool, lab.host).await.unwrap();
    let done = CommandReport { status: CommandStatus::Done, result: String::new() };
    commands::report(&lab.pool, lab.host, pending[0].id, &done).await.unwrap();
    assert_eq!(remediation::auto::tick(&state, &http).await.unwrap(), 0, "max attempts reached");
}

#[tokio::test]
async fn runbooks_actions_and_policies_travel_with_the_backup_disarmed() {
    let lab = lab().await;
    lab.app
        .post(
            "/api/runbooks",
            json!({ "title": "Disk", "body": "Clean /var", "links": [{ "scope": "rule", "value": "disk_full" }] }),
            Some(&lab.admin),
        )
        .await;
    let device = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "nas", "address": "10.0.0.7", "kind": "dummy" }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(device.status, StatusCode::CREATED, "{}", device.body);
    let action = create_action(
        &lab,
        json!({ "name": "hook", "kind": "webhook", "params": { "url": "https://hooks.lan/x" },
                "secrets": { "token": "s3cret" }, "target_id": device.body["id"] }),
    )
    .await;
    armed_policy(&lab, action["id"].as_i64().unwrap(), json!({ "target_id": device.body["id"] }))
        .await;
    remediation::store::insert_run(
        &lab.pool,
        &remediation::store::NewRun {
            action_name: "x".into(),
            kind: "webhook".into(),
            trigger: "manual",
            requested_by: "admin".into(),
            status: "done",
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let lot = collect(&lab.pool, &cipher(&lab.pool).await, ExportOptions::default()).await.unwrap();
    assert_eq!(lot.runbooks.len(), 1);
    assert_eq!(lot.remediation_actions.len(), 1);
    assert_eq!(lot.remediation_policies.len(), 1);

    let dir = tempfile::tempdir().unwrap();
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.unwrap();
    let target_cipher = cipher(&pool).await;
    let report = restore(&pool, &target_cipher, &lot, true).await.unwrap();
    let section = |name: &str| report.sections.iter().find(|s| s.section == name).unwrap().created;
    assert_eq!(section("runbooks"), 1, "{report:#?}");
    assert_eq!(section("remediation_actions"), 1, "{report:#?}");
    assert_eq!(section("remediation_policies"), 1, "{report:#?}");
    let enabled: i64 = sqlx::query_scalar("SELECT enabled FROM remediation_policies")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(enabled, 0, "restored disarmed");
    let restored = remediation::store::list_actions(&pool).await.unwrap();
    let secrets =
        remediation::store::action_secrets(&pool, &target_cipher, restored[0].id).await.unwrap();
    assert_eq!(secrets["token"], "s3cret", "secrets travel encrypted in the bundle");
    let runs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM remediation_runs").fetch_one(&pool).await.unwrap();
    assert_eq!(runs, 0, "the log does not travel");
    // Relire le même lot ne change rien.
    let again = restore(&pool, &target_cipher, &lot, true).await.unwrap();
    for name in ["runbooks", "remediation_actions", "remediation_policies"] {
        let s = again.sections.iter().find(|s| s.section == name).unwrap();
        assert_eq!((s.created, s.updated), (0, 0), "{name}");
    }
}
