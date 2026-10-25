//! Limitation des essais de mot de passe d'une page privée. Dans son propre
//! binaire de test : les compteurs sont en mémoire et partagés par le
//! processus, un autre test échouant exprès sur la même page les fausserait.

mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::TestApp;

#[tokio::test]
async fn password_attempts_are_rate_limited() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = app
        .post(
            "/api/status-pages",
            json!({ "title": "Board", "slug": "board", "published": true }),
            Some(&admin),
        )
        .await;
    let id = created.body["id"].as_i64().unwrap();
    let access = json!({ "mode": "password", "password": "board-secret-1" });
    let set = app.put(&format!("/api/status-pages/{id}/access"), access, Some(&admin)).await;
    assert_eq!(set.status, StatusCode::OK, "{}", set.body);

    let mut last = StatusCode::OK;
    for _ in 0..8 {
        last = app
            .post("/api/public/status/board/access", json!({ "password": "nope-nope-nope" }), None)
            .await
            .status;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
    // Même le bon mot de passe attend la fin du blocage.
    let blocked = app
        .post("/api/public/status/board/access", json!({ "password": "board-secret-1" }), None)
        .await;
    assert_eq!(blocked.status, StatusCode::TOO_MANY_REQUESTS);
}
