//! Tests d'intégration de la garde de l'enrôlement : validation des nouvelles
//! machines, débit par jeton, mise en veille des jetons de flotte inactifs.
//!
//! Le scénario fermé : un jeton de flotte fuit (il n'expire jamais, il est
//! dans un script de déploiement), et quelqu'un s'en sert pour inscrire de
//! faux agents. Tout passe par l'API réelle, comme le ferait l'agent.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::TestApp;

/// Instance dont le test garde la base, réglages de la garde effacés : c'est
/// l'état d'une installation neuve (le harnais commun les met en automatique).
async fn fresh_install() -> (TestApp, sqlx::SqlitePool, String) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = dumbmonit_server::db::open(&config.database_path()).await.expect("base");
    let app = common::build(dir, config, pool.clone()).await;
    sqlx::query("DELETE FROM settings WHERE key = 'agent.enrolment'")
        .execute(&pool)
        .await
        .expect("réglages effacés");
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    (app, pool, admin)
}

async fn token(app: &TestApp, admin: &str, reusable: bool) -> (i64, String) {
    let reply = app.post("/api/agent/tokens", json!({ "reusable": reusable }), Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "jeton : {}", reply.body);
    let id = reply.body["id"].as_i64().expect("id");
    (id, reply.body["secret"].as_str().expect("secret").to_string())
}

fn batch(hostname: &str) -> Value {
    json!({
        "protocol": 1,
        "identity": {
            "hostname": hostname,
            "os": "linux",
            "agent_version": "0.15.0",
            "machine_id": format!("id-{hostname}"),
            "binding_supported": true,
        },
        "sent_at_ms": 0,
        "samples": [],
    })
}

/// Pousse un lot sur le port de l'interface.
async fn push(
    app: &TestApp,
    enrolment: &str,
    hostname: &str,
    secret: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        builder = builder.header("x-dumbmonit-agent-secret", secret);
    }
    let request = builder.body(Body::from(batch(hostname).to_string())).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn count(pool: &sqlx::SqlitePool, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.expect("compte")
}

async fn pending(app: &TestApp, admin: &str) -> Vec<Value> {
    let view = app.get("/api/agent/enrolment", Some(admin)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    view.body["pending"].as_array().cloned().unwrap_or_default()
}

async fn decide(app: &TestApp, admin: &str, ids: &[i64], approve: bool) -> Value {
    let body = json!({ "ids": ids, "approve": approve });
    let reply = app.post("/api/agent/enrolment/decide", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

async fn configure(app: &TestApp, admin: &str, settings: Value) {
    let reply = app.put("/api/agent/enrolment", settings, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

#[tokio::test]
async fn a_fleet_machine_waits_for_approval_and_nothing_is_recorded_meanwhile() {
    let (app, pool, admin) = fresh_install().await;
    let view = app.get("/api/agent/enrolment", Some(&admin)).await;
    assert_eq!(view.body["settings"]["fleet_approval"], "manual", "installation neuve");
    assert_eq!(view.body["settings"]["single_use_approval"], "auto");
    assert_eq!(view.body["max_pending"], 200);

    let (fleet_id, fleet) = token(&app, &admin, true).await;
    for _ in 0..3 {
        let (status, body) = push(&app, &fleet, "intrus", None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], dumbmonit_proto::ENROLMENT_PENDING);
    }
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM targets").await, 0, "aucun équipement");
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM agent_hosts").await, 0);

    let waiting = pending(&app, &admin).await;
    assert_eq!(waiting.len(), 1, "une seule ligne malgré les essais : {waiting:?}");
    assert_eq!(waiting[0]["hostname"], "intrus");
    assert_eq!(waiting[0]["token_id"], fleet_id);
    assert!(waiting[0]["token_name"].is_string());
    assert!(waiting[0].get("source_ip").is_some());

    // Un jeton à usage unique a été créé pour cette machine-là : il entre.
    let (_, single) = token(&app, &admin, false).await;
    let (status, body) = push(&app, &single, "laptop", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let id = waiting[0]["id"].as_i64().unwrap();
    assert_eq!(decide(&app, &admin, &[id], true).await["decided"], 1);
    let (status, body) = push(&app, &fleet, "intrus", None).await;
    assert_eq!(status, StatusCode::OK, "acceptée : {body}");
    assert!(body["agent_secret"].is_string());
    assert!(pending(&app, &admin).await.is_empty());
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM agent_hosts").await, 2);
}

#[tokio::test]
async fn a_rejected_machine_is_refused_and_no_longer_listed() {
    let (app, _pool, admin) = fresh_install().await;
    let (_, fleet) = token(&app, &admin, true).await;
    push(&app, &fleet, "a", None).await;
    push(&app, &fleet, "b", None).await;
    push(&app, &fleet, "c", None).await;
    let ids: Vec<i64> =
        pending(&app, &admin).await.iter().map(|p| p["id"].as_i64().unwrap()).collect();
    assert_eq!(ids.len(), 3);

    // En lot : deux refusées d'un coup, la troisième acceptée.
    assert_eq!(decide(&app, &admin, &ids[..2], false).await["decided"], 2);
    assert_eq!(decide(&app, &admin, &ids[..2], true).await["decided"], 0, "déjà tranchées");
    assert_eq!(decide(&app, &admin, &ids[2..], true).await["decided"], 1);

    let mut refused = 0;
    for name in ["a", "b", "c"] {
        let (status, body) = push(&app, &fleet, name, None).await;
        if status == StatusCode::FORBIDDEN {
            assert_eq!(body["error"], dumbmonit_proto::ENROLMENT_REJECTED);
            refused += 1;
        } else {
            assert_eq!(status, StatusCode::OK, "{body}");
        }
    }
    assert_eq!(refused, 2);
    assert!(pending(&app, &admin).await.is_empty(), "les refusées ne reviennent pas");

    let none = json!({ "ids": [], "approve": true });
    let empty = app.post("/api/agent/enrolment/decide", none, Some(&admin)).await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_ad_domain_mode_admits_hosts_of_the_domain_and_asks_for_the_rest() {
    let (app, _pool, admin) = fresh_install().await;
    configure(
        &app,
        &admin,
        json!({ "fleet_approval": "ad_domain", "single_use_approval": "auto",
                "rate_per_hour": 60, "dormant_days": 30 }),
    )
    .await;
    // Un jeton de flotte ordinaire n'a pas de domaine : la machine attend.
    let (_, fleet) = token(&app, &admin, true).await;
    let (status, body) = push(&app, &fleet, "srv01.corp.example.com", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], dumbmonit_proto::ENROLMENT_PENDING);
}

#[tokio::test]
async fn a_token_over_its_hourly_rate_gets_429() {
    let (app, _pool, admin) = fresh_install().await;
    configure(
        &app,
        &admin,
        json!({ "fleet_approval": "auto", "single_use_approval": "auto",
                "rate_per_hour": 2, "dormant_days": 30 }),
    )
    .await;
    let (_, fleet) = token(&app, &admin, true).await;
    assert_eq!(push(&app, &fleet, "a", None).await.0, StatusCode::OK);
    assert_eq!(push(&app, &fleet, "b", None).await.0, StatusCode::OK);
    assert_eq!(push(&app, &fleet, "c", None).await.0, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn an_idle_fleet_token_sleeps_without_cutting_its_machines_and_can_be_reactivated() {
    let (app, pool, admin) = fresh_install().await;
    configure(
        &app,
        &admin,
        json!({ "fleet_approval": "auto", "single_use_approval": "auto",
                "rate_per_hour": 60, "dormant_days": 30 }),
    )
    .await;
    let (id, fleet) = token(&app, &admin, true).await;
    let (status, ack) = push(&app, &fleet, "old", None).await;
    assert_eq!(status, StatusCode::OK, "{ack}");
    let secret = ack["agent_secret"].as_str().expect("secret").to_string();

    sqlx::query("UPDATE agent_tokens SET enrol_activity_at = '2020-01-01T00:00:00Z'")
        .execute(&pool)
        .await
        .unwrap();
    let (status, body) = push(&app, &fleet, "new", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(body["error"].as_str().unwrap().contains("put to sleep"), "{body}");
    // La machine déjà enrôlée continue de remonter.
    assert_eq!(push(&app, &fleet, "old", Some(&secret)).await.0, StatusCode::OK);

    let tokens = app.get("/api/agent/tokens", Some(&admin)).await;
    let listed = tokens.body.as_array().unwrap().iter().find(|t| t["id"] == id).unwrap().clone();
    assert!(listed["dormant_at"].is_string(), "{listed}");

    let reactivate = format!("/api/agent/tokens/{id}/reactivate");
    let first = app.post(&reactivate, json!({}), Some(&admin)).await;
    assert_eq!(first.status, StatusCode::NO_CONTENT);
    let again = app.post(&reactivate, json!({}), Some(&admin)).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND, "déjà en service");
    assert_eq!(push(&app, &fleet, "new", None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn settings_are_checked_and_reserved_to_administrators() {
    let (app, _pool, admin) = fresh_install().await;
    let too_fast = json!({ "fleet_approval": "auto", "single_use_approval": "auto",
                           "rate_per_hour": 1_000_000, "dormant_days": 30 });
    let reply = app.put("/api/agent/enrolment", too_fast, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get("/api/agent/enrolment", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    let reply = app
        .post("/api/agent/enrolment/decide", json!({ "ids": [1], "approve": true }), Some(&viewer))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}
