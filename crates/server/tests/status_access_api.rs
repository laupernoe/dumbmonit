//! Pages de statut privées : mot de passe, comptes connectés, et tout ce qui
//! sort d'une page par son slug (document, badge, flux, sitemap).

mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::TestApp;

/// Corps texte d'une route publique (sitemap, robots, page HTML).
async fn raw(app: &TestApp, uri: &str) -> String {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

async fn page(app: &TestApp, admin: &str) -> i64 {
    let reply = app
        .post(
            "/api/status-pages",
            json!({ "title": "Board", "slug": "board", "published": true,
                    "seo": { "index": true } }),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body["id"].as_i64().unwrap()
}

async fn set_access(app: &TestApp, admin: &str, id: i64, body: Value) -> Value {
    let reply = app.put(&format!("/api/status-pages/{id}/access"), body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

#[tokio::test]
async fn a_public_page_stays_open_by_default() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page(&app, &admin).await;
    let reply = app.get(&format!("/api/status-pages/{id}"), Some(&admin)).await;
    assert_eq!(reply.body["access_mode"], "public");
    assert_eq!(reply.body["has_password"], false);
    assert_eq!(app.get("/api/public/status/board", None).await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_password_page_needs_the_cookie_and_hides_the_hash() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page(&app, &admin).await;

    // Sans mot de passe enregistré, le mode « password » est refusé.
    let refused = app
        .put(&format!("/api/status-pages/{id}/access"), json!({ "mode": "password" }), Some(&admin))
        .await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
    let short = app
        .put(
            &format!("/api/status-pages/{id}/access"),
            json!({ "mode": "password", "password": "court" }),
            Some(&admin),
        )
        .await;
    assert_eq!(short.status, StatusCode::BAD_REQUEST);

    let view =
        set_access(&app, &admin, id, json!({ "mode": "password", "password": "board-secret-1" }))
            .await;
    assert_eq!(view["access_mode"], "password");
    assert_eq!(view["has_password"], true);
    assert!(!view.to_string().contains("argon2"), "the hash never leaves the server");

    // Document, badge, flux et jour : tous demandent le mot de passe.
    for path in [
        "/api/public/status/board",
        "/api/public/status/board/badge.svg",
        "/api/public/status/board/rss",
        "/api/public/status/board/uptime.svg",
        "/api/public/status/board/day/2026-01-01",
    ] {
        let reply = app.get(path, None).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(reply.body["access"], "password", "{path}");
    }

    // Mauvais mot de passe : refusé, aucun cookie.
    let wrong = app
        .post("/api/public/status/board/access", json!({ "password": "nope-nope-nope" }), None)
        .await;
    assert_eq!(wrong.status, StatusCode::FORBIDDEN);
    assert!(wrong.set_cookie.is_none());

    // Bon mot de passe : cookie d'accès, puis lecture.
    let right = app
        .post("/api/public/status/board/access", json!({ "password": "board-secret-1" }), None)
        .await;
    assert_eq!(right.status, StatusCode::OK, "{}", right.body);
    let cookie = right.cookie();
    let doc = app.get("/api/public/status/board", Some(&cookie)).await;
    assert_eq!(doc.status, StatusCode::OK, "{}", doc.body);
    assert_eq!(
        app.get("/api/public/status/board/badge.svg", Some(&cookie)).await.status,
        StatusCode::OK
    );

    // Un cookie d'une autre page, ou forgé, ne vaut rien.
    let forged = app.get("/api/public/status/board", Some("dm_page_1=9999999999.deadbeef")).await;
    assert_eq!(forged.status, StatusCode::UNAUTHORIZED);

    // Changer le mot de passe invalide les cookies déjà posés.
    set_access(&app, &admin, id, json!({ "mode": "password", "password": "board-secret-2" })).await;
    assert_eq!(
        app.get("/api/public/status/board", Some(&cookie)).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_users_page_needs_a_dumbmonit_session() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page(&app, &admin).await;
    set_access(&app, &admin, id, json!({ "mode": "users" })).await;

    let anonymous = app.get("/api/public/status/board", None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous.body["access"], "users");

    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get("/api/public/status/board", Some(&viewer)).await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_private_page_is_never_indexed() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page(&app, &admin).await;

    // Publique et indexable : dans le sitemap.
    let open = raw(&app, "/sitemap.xml").await;
    assert!(open.contains("/s/board</loc>"));

    for mode in ["users", "password"] {
        let mut body = json!({ "mode": mode });
        if mode == "password" {
            body["password"] = json!("board-secret-1");
        }
        set_access(&app, &admin, id, body).await;
        let sitemap = raw(&app, "/sitemap.xml").await;
        assert!(!sitemap.contains("board"), "{mode}: {sitemap}");
        let robots = raw(&app, "/robots.txt").await;
        assert!(!robots.contains("board"), "{mode}: {robots}");
        let html = raw(&app, "/s/board").await;
        assert!(html.contains("noindex, nofollow"), "{mode}");
    }
}

#[tokio::test]
async fn leaving_password_mode_forgets_the_password() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page(&app, &admin).await;
    set_access(&app, &admin, id, json!({ "mode": "password", "password": "board-secret-1" })).await;
    let view = set_access(&app, &admin, id, json!({ "mode": "public" })).await;
    assert_eq!(view["has_password"], false);
    assert_eq!(app.get("/api/public/status/board", None).await.status, StatusCode::OK);
    let bad = app
        .put(&format!("/api/status-pages/{id}/access"), json!({ "mode": "secret" }), Some(&admin))
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}
