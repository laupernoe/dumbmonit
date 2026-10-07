//! Mise à jour guidée : les routes `/api/update*`. Aucun test ne sort sur
//! Internet — la vérification est coupée avant la première lecture.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn the_update_routes_need_a_session() {
    let app = TestApp::configured().await;
    assert_eq!(app.get("/api/update", None).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn turning_the_check_off_is_persisted_and_makes_no_request() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let off = app.put("/api/update/settings", json!({ "enabled": false }), Some(&admin)).await;
    assert_eq!(off.status, StatusCode::OK, "{}", off.body);
    assert_eq!(off.body["enabled"], false);

    let info = app.get("/api/update", Some(&admin)).await;
    assert_eq!(info.status, StatusCode::OK);
    assert_eq!(info.body["enabled"], false);
    assert_eq!(info.body["update_available"], false);
    assert_eq!(info.body["current"], env!("CARGO_PKG_VERSION"));
    assert!(info.body["latest"].is_null());

    // « Check now » ne contacte rien non plus quand la vérification est coupée.
    let check = app.post("/api/update/check", json!({}), Some(&admin)).await;
    assert_eq!(check.status, StatusCode::OK);
    assert_eq!(check.body["enabled"], false);
}

#[tokio::test]
async fn only_an_administrator_changes_the_setting() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    app.put("/api/update/settings", json!({ "enabled": false }), Some(&admin)).await;
    let viewer = app.viewer_cookie(&admin).await;

    let denied = app.put("/api/update/settings", json!({ "enabled": true }), Some(&viewer)).await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    // Lire est permis au lecteur.
    assert_eq!(app.get("/api/update", Some(&viewer)).await.status, StatusCode::OK);
}
