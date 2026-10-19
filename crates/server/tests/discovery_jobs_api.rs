//! Tests d'intégration de l'API de découverte réseau v2 (`/api/discovery/jobs`).
//!
//! Le registre du harnais connaît les collecteurs réseau partagés, donc les
//! empreintes aussi : un balayage réel part, borné à un réseau de documentation
//! (RFC 5737) où personne ne répond, et se termine seul.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

async fn setup() -> TestApp {
    let app = common::setup().await;
    app.create_admin().await;
    app
}

/// Attend qu'un balayage serveur se termine (au plus ~15 s).
async fn wait_done(app: &TestApp, admin: &str, id: &str) -> serde_json::Value {
    for _ in 0..150 {
        let reply = app.get(&format!("/api/discovery/jobs/{id}"), Some(admin)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        if reply.body["status"] != "running" {
            return reply.body;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("le balayage ne s'est pas terminé");
}

#[tokio::test]
async fn a_server_scan_runs_and_finishes_without_leaking_the_community() {
    let app = setup().await;
    let admin = app.admin_cookie().await;

    let reply = app
        .post(
            "/api/discovery/jobs",
            json!({ "cidr": "192.0.2.0/30", "snmp_community": "s3cr3t-community" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::ACCEPTED, "{}", reply.body);
    let id = reply.body["id"].as_str().expect("id").to_string();

    let job = wait_done(&app, &admin, &id).await;
    assert_eq!(job["status"], "done", "{job}");
    assert_eq!(job["hosts_found"], 0);
    assert_eq!(job["progress"]["phase"], "done");

    // La community ne doit apparaître nulle part dans ce qui est renvoyé.
    assert!(!job.to_string().contains("s3cr3t"), "la community a fuité: {job}");
    let list = app.get("/api/discovery/jobs", Some(&admin)).await;
    assert_eq!(list.status, StatusCode::OK);
    assert!(!list.body.to_string().contains("s3cr3t"));
    assert_eq!(list.body["jobs"][0]["id"], id);
}

#[tokio::test]
async fn a_scan_can_be_cancelled() {
    let app = setup().await;
    let admin = app.admin_cookie().await;
    let reply =
        app.post("/api/discovery/jobs", json!({ "cidr": "192.0.2.0/24" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::ACCEPTED, "{}", reply.body);
    let id = reply.body["id"].as_str().expect("id").to_string();

    let cancel = app.delete(&format!("/api/discovery/jobs/{id}"), Some(&admin)).await;
    assert_eq!(cancel.status, StatusCode::NO_CONTENT, "{}", cancel.body);
    let job = app.get(&format!("/api/discovery/jobs/{id}"), Some(&admin)).await;
    assert!(job.body["status"] == "cancelled" || job.body["status"] == "done", "{}", job.body);
}

#[tokio::test]
async fn a_bad_or_too_wide_network_is_refused() {
    let app = setup().await;
    let admin = app.admin_cookie().await;

    let bad = app.post("/api/discovery/jobs", json!({ "cidr": "not-a-cidr" }), Some(&admin)).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.body);

    // Un /8 dépasse la borne de 1024 adresses (/22).
    let wide = app.post("/api/discovery/jobs", json!({ "cidr": "10.0.0.0/8" }), Some(&admin)).await;
    assert_eq!(wide.status, StatusCode::BAD_REQUEST, "{}", wide.body);
}

#[tokio::test]
async fn an_unknown_relay_agent_is_rejected() {
    let app = setup().await;
    let admin = app.admin_cookie().await;
    let reply = app
        .post(
            "/api/discovery/jobs",
            json!({ "cidr": "192.0.2.0/30", "agent_id": 9999 }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
}

#[tokio::test]
async fn discovery_jobs_are_admin_only() {
    let app = setup().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    // Sans session : refusé.
    assert_eq!(app.get("/api/discovery/jobs", None).await.status, StatusCode::UNAUTHORIZED);
    // Lecteur : interdit (admin requis).
    assert_eq!(app.get("/api/discovery/jobs", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        app.post("/api/discovery/jobs", json!({ "cidr": "192.0.2.0/30" }), Some(&viewer))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}
