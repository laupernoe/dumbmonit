//! Tests d'intégration des rapports périodiques : droits, validation, cycle de
//! vie du calendrier et aperçu sans canal courriel.

mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::TestApp;

async fn smtp_channel(app: &TestApp, cookie: &str) -> i64 {
    let reply = app
        .post(
            "/api/notify/channels",
            json!({
                "name": "Mail", "kind": "smtp",
                "settings": {
                    "host": "127.0.0.1", "port": 1, "security": "none",
                    "from": "dumbmonit@example.org", "to": ["ops@example.org"]
                }
            }),
            Some(cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "channel: {}", reply.body);
    reply.body["id"].as_i64().expect("channel id")
}

#[tokio::test]
async fn un_rapport_a_des_valeurs_par_defaut_hebdomadaires() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply = app
        .post("/api/reports/schedules", json!({ "recipients": ["Ops@Example.org"] }), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["frequency"], "weekly");
    assert_eq!(reply.body["weekday"], 0);
    assert_eq!(reply.body["hour"], 8);
    assert_eq!(reply.body["timezone"], "UTC");
    assert_eq!(reply.body["enabled"], true);
    assert_eq!(reply.body["recipients"], json!(["ops@example.org"]), "normalised");
    assert!(reply.body["last_sent_at"].is_null(), "never sent yet");
    assert!(reply.body["next_run_at"].is_string(), "next run announced");

    let id = reply.body["id"].as_i64().unwrap();
    let list = app.get("/api/reports/schedules", Some(&admin)).await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(list.body.as_array().unwrap().len(), 1);

    let updated = app
        .put(
            &format!("/api/reports/schedules/{id}"),
            json!({
                "frequency": "monthly", "day_of_month": 15, "hour": 6,
                "timezone": "Europe/Paris", "recipients": ["a@example.org", "b@example.org"]
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
    assert_eq!(updated.body["frequency"], "monthly");
    assert_eq!(updated.body["day_of_month"], 15);
    assert_eq!(updated.body["timezone"], "Europe/Paris");

    let removed = app.delete(&format!("/api/reports/schedules/{id}"), Some(&admin)).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let gone = app.get(&format!("/api/reports/schedules/{id}"), Some(&admin)).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn un_lecteur_ne_voit_ni_ne_modifie_rien() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let created = app
        .post("/api/reports/schedules", json!({ "recipients": ["ops@example.org"] }), Some(&admin))
        .await;
    let id = created.body["id"].as_i64().unwrap();

    // La liste des destinataires est réservée aux administrateurs, lecture comprise.
    let list = app.get("/api/reports/schedules", Some(&viewer)).await;
    assert_eq!(list.status, StatusCode::FORBIDDEN);
    let one = app.get(&format!("/api/reports/schedules/{id}"), Some(&viewer)).await;
    assert_eq!(one.status, StatusCode::FORBIDDEN);
    let create = app
        .post("/api/reports/schedules", json!({ "recipients": ["x@example.org"] }), Some(&viewer))
        .await;
    assert_eq!(create.status, StatusCode::FORBIDDEN);
    let test =
        app.post(&format!("/api/reports/schedules/{id}/send-test"), json!({}), Some(&viewer)).await;
    assert_eq!(test.status, StatusCode::FORBIDDEN);

    let anonymous = app.get("/api/reports/schedules", None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn les_valeurs_invalides_sont_refusees() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let cases = [
        json!({ "frequency": "hourly", "recipients": ["a@example.org"] }),
        json!({ "weekday": 7, "recipients": ["a@example.org"] }),
        json!({ "day_of_month": 29, "recipients": ["a@example.org"] }),
        json!({ "hour": 24, "recipients": ["a@example.org"] }),
        json!({ "timezone": "Mars/Olympus", "recipients": ["a@example.org"] }),
        json!({ "name": "  ", "recipients": ["a@example.org"] }),
        json!({ "recipients": ["pas une adresse"] }),
        json!({ "recipients": ["a@example.org\r\nBcc: x@example.org"] }),
        json!({ "recipients": [] }),
        json!({ "recipients": ["a@example.org"], "channel_id": 9999 }),
    ];
    for body in cases {
        let reply = app.post("/api/reports/schedules", body.clone(), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{body} -> {}", reply.body);
    }

    // Trop de destinataires.
    let many: Vec<String> = (0..21).map(|i| format!("user{i}@example.org")).collect();
    let reply =
        app.post("/api/reports/schedules", json!({ "recipients": many }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    // Un rapport désactivé peut ne pas encore avoir de destinataire.
    let draft = app
        .post("/api/reports/schedules", json!({ "enabled": false, "recipients": [] }), Some(&admin))
        .await;
    assert_eq!(draft.status, StatusCode::CREATED, "{}", draft.body);
    assert!(draft.body["next_run_at"].is_null(), "a disabled report has no next run");
}

#[tokio::test]
async fn le_canal_doit_etre_un_canal_courriel() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let channel = smtp_channel(&app, &admin).await;

    let ok = app
        .post(
            "/api/reports/schedules",
            json!({ "recipients": ["a@example.org"], "channel_id": channel }),
            Some(&admin),
        )
        .await;
    assert_eq!(ok.status, StatusCode::CREATED, "{}", ok.body);
    assert_eq!(ok.body["channel_id"], channel);
}

#[tokio::test]
async fn l_apercu_echoue_proprement_sans_canal_courriel() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = app
        .post("/api/reports/schedules", json!({ "recipients": ["a@example.org"] }), Some(&admin))
        .await;
    let id = created.body["id"].as_i64().unwrap();

    let reply =
        app.post(&format!("/api/reports/schedules/{id}/send-test"), json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let message = reply.body["error"].as_str().unwrap_or_default();
    assert!(message.contains("email"), "message explicite : {message}");

    let unknown = app.post("/api/reports/schedules/99999/send-test", json!({}), Some(&admin)).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}
