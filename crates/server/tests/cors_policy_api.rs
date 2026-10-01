//! Politique CORS restreinte par `DUMBMONIT_API_CORS_ORIGINS`, et la même
//! politique appliquée à l'en-tête `Origin` du serveur MCP.
//!
//! Un seul test dans ce fichier : la variable d'environnement est globale au
//! processus, et chaque fichier de tests est un processus à part.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::TestApp;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn only_listed_origins_may_call_the_api_or_the_mcp_server() {
    // SAFETY : seul test de ce binaire, rien d'autre ne lit l'environnement en
    // même temps.
    unsafe { std::env::set_var("DUMBMONIT_API_CORS_ORIGINS", "https://dash.example/") };
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = app.post("/api/tokens", json!({ "name": "Dashboard" }), Some(&admin)).await;
    let token = reply.body["secret"].as_str().unwrap().to_string();

    let preflight = |origin: &'static str| {
        Request::builder()
            .method("OPTIONS")
            .uri("/api/targets")
            .header(header::ORIGIN, origin)
            .header("access-control-request-method", "GET")
            .header("access-control-request-headers", "authorization")
            .body(Body::empty())
            .unwrap()
    };
    let response = app.router.clone().oneshot(preflight("https://dash.example")).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(response.headers()["access-control-allow-origin"], "https://dash.example");
    assert!(response.headers().get_all(header::VARY).iter().any(|v| v == "origin"));

    let response = app.router.clone().oneshot(preflight("https://evil.example")).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(response.headers().get("access-control-allow-origin").is_none());

    let mcp = |origin: &'static str| {
        Request::builder()
            .method("POST")
            .uri("/api/mcp")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::HOST, "monit.lan:8080")
            .header(header::ORIGIN, origin)
            .body(Body::from(json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }).to_string()))
            .unwrap()
    };
    let response = app.router.clone().oneshot(mcp("https://evil.example")).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN, "protection contre le DNS rebinding");
    let response = app.router.clone().oneshot(mcp("https://dash.example")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["access-control-allow-origin"], "https://dash.example");
    let response = app.router.clone().oneshot(mcp("http://monit.lan:8080")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK, "notre propre origine est toujours admise");
}
