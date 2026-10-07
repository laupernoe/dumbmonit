//! Le rôle `operator`, entre `viewer` et `admin` : une matrice rôle × route.
//!
//! Un opérateur traite les alertes (acquitter, ignorer, mettre en sourdine,
//! effacer l'historique résolu) et ne touche à rien de la configuration ; un
//! lecteur ne fait ni l'un ni l'autre ; un administrateur fait tout.

mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use common::{PASSWORD, TestApp};
use dumbmonit_server::alerting::group::AlertOutcome;
use dumbmonit_server::alerting::machine::{AlertState, Phase};
use dumbmonit_server::alerting::model::Severity;
use dumbmonit_server::db;
use serde_json::{Value, json};
use tower::ServiceExt;

const OPERATOR_PASSWORD: &str = "mot-de-passe-de-l-operateur";
const VIEWER_PASSWORD: &str = "mot-de-passe-du-lecteur";

struct Lab {
    app: TestApp,
    admin: String,
    operator: String,
    viewer: String,
    target: i64,
    rule: i64,
}

/// Instance avec un administrateur, un opérateur, un lecteur, un équipement et
/// une alerte `cpu_high@<équipement>` en cours.
async fn lab() -> Lab {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    db::alerts::seed_builtin_rules(&pool).await.expect("règles intégrées");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;

    for (name, role, password) in
        [("ops", "operator", OPERATOR_PASSWORD), ("watcher", "viewer", VIEWER_PASSWORD)]
    {
        let reply = app
            .post(
                "/api/users",
                json!({ "username": name, "role": role, "password": password }),
                Some(&admin),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{name}: {}", reply.body);
        assert_eq!(reply.body["role"], json!(role));
    }
    let operator = app.login_as("ops", OPERATOR_PASSWORD).await.cookie();
    let viewer = app.login_as("watcher", VIEWER_PASSWORD).await.cookie();

    let reply = app
        .post(
            "/api/targets",
            json!({ "name": "nas", "address": "10.0.0.7", "kind": "dummy" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let target = reply.body["id"].as_i64().expect("id");

    let rules = app.get("/api/alerts/rules", Some(&admin)).await;
    let rule = rules.body.as_array().expect("règles")[0]["id"].as_i64().expect("id de règle");

    db::alerts::save_states(&pool, &[firing_alert(&format!("cpu_high@{target}"), target)])
        .await
        .expect("état enregistré");

    Lab { app, admin, operator, viewer, target, rule }
}

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
    }
}

fn silence() -> Value {
    json!({
        "name": "Fan swap",
        "matchers": { "host": "nas" },
        "schedule": {
            "kind": "once",
            "starts_at": "2026-01-01T00:00:00Z",
            "ends_at": "2099-01-01T02:00:00Z"
        }
    })
}

/// Les écritures de configuration, avec un corps plausible : aucune ne doit
/// passer pour un opérateur ou un lecteur.
fn configuration_writes(lab: &Lab) -> Vec<(&'static str, String, Option<Value>)> {
    let t = lab.target;
    let r = lab.rule;
    vec![
        (
            "POST",
            "/api/targets".into(),
            Some(json!({ "name": "x", "address": "10.0.0.9", "kind": "dummy" })),
        ),
        (
            "PUT",
            format!("/api/targets/{t}"),
            Some(json!({ "name": "renamed", "address": "10.0.0.7", "kind": "dummy" })),
        ),
        ("DELETE", format!("/api/targets/{t}"), None),
        ("POST", format!("/api/targets/{t}/probe"), None),
        ("POST", "/api/targets/reorder".into(), Some(json!({ "ids": [t] }))),
        ("POST", "/api/alerts/rules".into(), Some(json!({ "name": "r", "kind": "threshold" }))),
        ("PUT", format!("/api/alerts/rules/{r}"), Some(json!({ "name": "r" }))),
        ("DELETE", format!("/api/alerts/rules/{r}"), None),
        ("POST", format!("/api/alerts/rules/{r}/enable"), Some(json!({ "enabled": false }))),
        ("PUT", "/api/notify/policy".into(), Some(json!({ "max_per_hour": 1 }))),
        ("POST", "/api/notify/channels".into(), Some(json!({ "name": "c", "kind": "ntfy" }))),
        (
            "POST",
            "/api/users".into(),
            Some(json!({ "username": "eve", "role": "admin", "password": "mot-de-passe-pirate" })),
        ),
        ("PUT", "/api/users/1".into(), Some(json!({ "role": "viewer" }))),
        ("DELETE", "/api/users/1".into(), None),
        ("POST", "/api/tokens".into(), Some(json!({ "name": "t", "scope": "write" }))),
        ("POST", "/api/agent/tokens".into(), Some(json!({ "name": "fleet" }))),
        ("PUT", "/api/onboarding".into(), Some(json!({ "dismissed": true }))),
        ("POST", "/api/backup".into(), Some(json!({}))),
        ("POST", "/api/status-pages".into(), Some(json!({ "title": "Status", "slug": "status" }))),
        ("PUT", "/api/auth/oidc/config".into(), Some(json!({ "issuer": "", "client_id": "" }))),
        ("POST", "/api/discovery".into(), Some(json!({ "cidr": "10.0.0.0/30" }))),
    ]
}

#[tokio::test]
async fn an_operator_handles_alerts() {
    let lab = lab().await;
    let app = &lab.app;
    let fingerprint = format!("cpu_high@{}", lab.target);

    let me = app.get("/api/auth/me", Some(&lab.operator)).await;
    assert_eq!(me.body["role"], json!("operator"), "{}", me.body);

    // Acquitter, puis lever.
    let ack =
        app.post(&format!("/api/alerts/{fingerprint}/ack"), json!({}), Some(&lab.operator)).await;
    assert_eq!(ack.status, StatusCode::OK, "{}", ack.body);
    assert_eq!(ack.body["acked_by"], json!("ops"));
    let unack = app.delete(&format!("/api/alerts/{fingerprint}/ack"), Some(&lab.operator)).await;
    assert_eq!(unack.status, StatusCode::OK, "{}", unack.body);

    // Mettre en sourdine, puis lever la sourdine.
    let created = app.post("/api/alerts/silences", silence(), Some(&lab.operator)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_i64().expect("id");
    let deleted = app.delete(&format!("/api/alerts/silences/{id}"), Some(&lab.operator)).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);

    // Effacer l'historique résolu.
    let cleared =
        app.post("/api/alerts/history/dismiss-resolved", json!({}), Some(&lab.operator)).await;
    assert_eq!(cleared.status, StatusCode::OK, "{}", cleared.body);

    // Ignorer la règle pour cet équipement, puis revenir dessus.
    let path = format!("/api/alerts/rules/{}/overrides/{}", lab.rule, lab.target);
    let ignored = app.put(&path, json!({ "enabled": false }), Some(&lab.operator)).await;
    assert_eq!(ignored.status, StatusCode::OK, "{}", ignored.body);
    assert_eq!(ignored.body["enabled"], json!(false));
    let undone = app.delete(&path, Some(&lab.operator)).await;
    assert_eq!(undone.status, StatusCode::NO_CONTENT, "{}", undone.body);

    // Il voit tout ce qu'un lecteur voit.
    for uri in ["/api/targets", "/api/alerts", "/api/alerts/rules", "/api/notify/channels"] {
        let reply = app.get(uri, Some(&lab.operator)).await;
        assert_eq!(reply.status, StatusCode::OK, "GET {uri}: {}", reply.body);
    }
}

#[tokio::test]
async fn an_operator_cannot_change_a_threshold_through_an_override() {
    let lab = lab().await;
    let app = &lab.app;
    let path = format!("/api/alerts/rules/{}/overrides/{}", lab.rule, lab.target);

    for body in [
        json!({ "threshold": 99.0 }),
        json!({ "enabled": false, "threshold": 99.0 }),
        json!({ "enabled": true }),
        json!({}),
    ] {
        let reply = app.put(&path, body.clone(), Some(&lab.operator)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{body}: {}", reply.body);
    }

    // Un seuil posé par l'administrateur survit à « ignorer » puis « annuler »
    // de la part d'un opérateur.
    let set = app.put(&path, json!({ "threshold": 80.0 }), Some(&lab.admin)).await;
    assert_eq!(set.status, StatusCode::OK, "{}", set.body);
    let ignored = app.put(&path, json!({ "enabled": false }), Some(&lab.operator)).await;
    assert_eq!(ignored.status, StatusCode::OK, "{}", ignored.body);
    assert_eq!(ignored.body["threshold"], json!(80.0), "{}", ignored.body);
    let undone = app.delete(&path, Some(&lab.operator)).await;
    assert_eq!(undone.status, StatusCode::NO_CONTENT, "{}", undone.body);
    let list =
        app.get(&format!("/api/alerts/rules/{}/overrides", lab.rule), Some(&lab.admin)).await;
    let kept = list.body.as_array().expect("liste");
    assert_eq!(kept.len(), 1, "{}", list.body);
    assert_eq!(kept[0]["threshold"], json!(80.0));
    assert!(kept[0]["enabled"].is_null(), "{}", list.body);
}

#[tokio::test]
async fn an_operator_cannot_touch_the_configuration() {
    let lab = lab().await;
    for (method, uri, body) in configuration_writes(&lab) {
        let reply = lab.app.request(method, &uri, body, Some(&lab.operator)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {uri}: {}", reply.body);
    }
    // Ni la liste des comptes, ni le journal d'audit.
    for uri in ["/api/users", "/api/auth/audit"] {
        let reply = lab.app.get(uri, Some(&lab.operator)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "GET {uri}");
    }
    // Rien n'a bougé : l'opérateur est toujours opérateur, l'équipement est là.
    let me = lab.app.get("/api/auth/me", Some(&lab.operator)).await;
    assert_eq!(me.body["role"], json!("operator"));
    let target = lab.app.get(&format!("/api/targets/{}", lab.target), Some(&lab.admin)).await;
    assert_eq!(target.status, StatusCode::OK);
    assert_eq!(target.body["name"], json!("nas"));
}

#[tokio::test]
async fn a_viewer_handles_nothing() {
    let lab = lab().await;
    let app = &lab.app;
    let fingerprint = format!("cpu_high@{}", lab.target);
    let path = format!("/api/alerts/rules/{}/overrides/{}", lab.rule, lab.target);
    let attempts: Vec<(&str, String, Option<Value>)> = vec![
        ("POST", format!("/api/alerts/{fingerprint}/ack"), Some(json!({}))),
        ("DELETE", format!("/api/alerts/{fingerprint}/ack"), None),
        ("POST", "/api/alerts/silences".into(), Some(silence())),
        ("POST", "/api/alerts/history/dismiss-resolved".into(), Some(json!({}))),
        ("PUT", path.clone(), Some(json!({ "enabled": false }))),
        ("DELETE", path, None),
    ];
    for (method, uri, body) in attempts.into_iter().chain(configuration_writes(&lab)) {
        let reply = app.request(method, &uri, body, Some(&lab.viewer)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {uri}: {}", reply.body);
    }
}

#[tokio::test]
async fn an_admin_does_both() {
    let lab = lab().await;
    let app = &lab.app;
    let fingerprint = format!("cpu_high@{}", lab.target);
    let ack =
        app.post(&format!("/api/alerts/{fingerprint}/ack"), json!({}), Some(&lab.admin)).await;
    assert_eq!(ack.status, StatusCode::OK, "{}", ack.body);
    let created = app.post("/api/alerts/silences", silence(), Some(&lab.admin)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let target = app
        .post(
            "/api/targets",
            json!({ "name": "router", "address": "10.0.0.1", "kind": "dummy" }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(target.status, StatusCode::CREATED, "{}", target.body);

    // L'administrateur promeut et rétrograde.
    let users = app.get("/api/users", Some(&lab.admin)).await;
    let ops =
        users.body.as_array().unwrap().iter().find(|u| u["username"] == json!("ops")).cloned();
    let ops_id = ops.expect("opérateur listé")["id"].as_i64().unwrap();
    let demoted = app
        .put(&format!("/api/users/{ops_id}"), json!({ "role": "viewer" }), Some(&lab.admin))
        .await;
    assert_eq!(demoted.status, StatusCode::OK, "{}", demoted.body);
    assert_eq!(demoted.body["role"], json!("viewer"));
    // La session existante suit le nouveau rôle dès la requête suivante.
    let refused =
        app.post(&format!("/api/alerts/{fingerprint}/ack"), json!({}), Some(&lab.operator)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);

    let reply = app
        .post(
            "/api/users",
            json!({ "username": "x", "role": "superuser", "password": PASSWORD }),
            Some(&lab.admin),
        )
        .await;
    assert!(reply.status.is_client_error(), "unknown role accepted: {}", reply.body);
}

#[tokio::test]
async fn a_write_token_owned_by_an_operator_is_read_only() {
    let lab = lab().await;
    let app = &lab.app;

    // Un second administrateur crée un jeton `write`, puis il est rétrogradé
    // opérateur : le jeton ne vaut plus qu'un jeton `read`.
    let reply = app
        .post(
            "/api/users",
            json!({ "username": "boss", "role": "admin", "password": PASSWORD }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let boss_id = reply.body["id"].as_i64().unwrap();
    let boss = app.login_as("boss", PASSWORD).await.cookie();
    let token =
        app.post("/api/tokens", json!({ "name": "ci", "scope": "write" }), Some(&boss)).await;
    assert_eq!(token.status, StatusCode::CREATED, "{}", token.body);
    let secret = token.body["secret"].as_str().unwrap().to_string();
    let demoted = app
        .put(&format!("/api/users/{boss_id}"), json!({ "role": "operator" }), Some(&lab.admin))
        .await;
    assert_eq!(demoted.status, StatusCode::OK, "{}", demoted.body);

    let fingerprint = format!("cpu_high@{}", lab.target);
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/alerts/{fingerprint}/ack"))
        .header(header::AUTHORIZATION, format!("Bearer {secret}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_users_table_accepts_the_operator_role_after_migration() {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let path = dir.path().join("dumbmonit.db");
    let old = db::open_up_to(&path, 41).await.expect("base ancienne");
    sqlx::query("INSERT INTO users (username, role) VALUES ('legacy', 'viewer')")
        .execute(&old)
        .await
        .expect("compte ancien");
    let refused = sqlx::query("INSERT INTO users (username, role) VALUES ('early', 'operator')")
        .execute(&old)
        .await;
    assert!(refused.is_err(), "avant la migration, le rôle est inconnu");
    old.close().await;

    let pool = db::open(&path).await.expect("migrations appliquées");
    sqlx::query("INSERT INTO users (username, role) VALUES ('ops', 'operator')")
        .execute(&pool)
        .await
        .expect("le rôle operator est admis");
    let refused = sqlx::query("INSERT INTO users (username, role) VALUES ('root', 'root')")
        .execute(&pool)
        .await;
    assert!(refused.is_err(), "la contrainte reste en place pour le reste");
    let legacy: (String,) = sqlx::query_as("SELECT role FROM users WHERE username = 'legacy'")
        .fetch_one(&pool)
        .await
        .expect("compte conservé");
    assert_eq!(legacy.0, "viewer");
    let check: Vec<(String,)> =
        sqlx::query_as("PRAGMA integrity_check").fetch_all(&pool).await.expect("intégrité");
    assert_eq!(check, vec![("ok".to_string(),)]);
}
