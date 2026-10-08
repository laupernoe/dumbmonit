//! Bandeau d'état : le document public est lisible depuis un autre site, le
//! reste de l'API ne l'est pas, et le script est servi.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::TestApp;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

async fn get(app: &TestApp, uri: &str) -> axum::http::Response<Body> {
    let request = Request::builder()
        .uri(uri)
        .header(header::ORIGIN, "https://blog.example")
        .body(Body::empty())
        .unwrap();
    app.router.clone().oneshot(request).await.expect("réponse")
}

#[tokio::test]
async fn the_public_document_is_cross_origin_readable_and_the_banner_script_is_served() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let page = app
        .post(
            "/api/status-pages",
            json!({ "title": "Home", "slug": "home", "published": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(page.status, StatusCode::CREATED, "{:?}", page.body);

    let response = get(&app, "/api/public/status/home").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert!(response.headers().get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS).is_none());

    // Le reste de l'API ne s'ouvre pas : pas d'en-tête CORS sans jeton.
    let response = get(&app, "/api/auth/status").await;
    assert!(response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());

    let response = get(&app, "/api/public/status/home/banner.js").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/javascript")
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&bytes).contains("attachShadow"));
}
