//! `GET /api/summary` : compteurs plats pour Homepage, lisibles avec un jeton `read`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::TestApp;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn with_token(app: &TestApp, uri: &str, token: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn le_resume_est_plat_et_lisible_avec_un_jeton_de_lecture() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let anonymous = app.get("/api/summary", None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);

    let device = app
        .post(
            "/api/targets",
            json!({ "name": "nas", "address": "nas.lan", "kind": "dummy" }),
            Some(&admin),
        )
        .await;
    assert_eq!(device.status, StatusCode::CREATED, "{}", device.body);
    let token =
        app.post("/api/tokens", json!({ "name": "homepage", "scope": "read" }), Some(&admin)).await;
    let secret = token.body["secret"].as_str().expect("secret").to_string();

    let (status, body) = with_token(&app, "/api/summary", &secret).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["devices"]["total"], json!(1), "{body}");
    for path in ["/devices/up", "/devices/down", "/alerts/firing", "/alerts/critical"] {
        assert!(body.pointer(path).is_some_and(Value::is_u64), "{path} manquant : {body}");
    }
    assert!(["ok", "warning", "critical"].contains(&body["status"].as_str().unwrap()), "{body}");
}
