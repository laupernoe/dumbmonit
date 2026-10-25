//! Calendriers iCal des maintenances : par page de statut (avec son mode
//! d'accès) et privé avec jeton.

mod common;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::TestApp;

struct Raw {
    status: StatusCode,
    headers: HeaderMap,
    text: String,
}

async fn raw(app: &TestApp, uri: &str, cookie: Option<&str>) -> Raw {
    let mut builder = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let response = app.router.clone().oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    Raw { status, headers, text: String::from_utf8_lossy(&bytes).into_owned() }
}

async fn page(app: &TestApp, admin: &str) -> i64 {
    let body = json!({ "title": "Acme", "slug": "acme", "published": true });
    let reply = app.post("/api/status-pages", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body["id"].as_i64().unwrap()
}

async fn announce(app: &TestApp, admin: &str, body: Value) {
    let reply = app.post("/api/incidents", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
}

#[tokio::test]
async fn a_page_feed_lists_its_maintenance_only() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page(&app, &admin).await;
    announce(
        &app,
        &admin,
        json!({ "title": "Switch upgrade", "kind": "maintenance", "status": "scheduled",
                "page_id": id, "starts_at": "2099-01-05T22:00:00Z", "ends_at": "2099-01-05T23:30:00Z",
                "body": "Short outage, expected." }),
    )
    .await;
    announce(
        &app,
        &admin,
        json!({ "title": "Disk failure", "kind": "incident", "page_id": id }),
    )
    .await;

    let feed = raw(&app, "/api/public/status/acme/maintenance.ics", None).await;
    assert_eq!(feed.status, StatusCode::OK);
    assert_eq!(feed.headers[header::CONTENT_TYPE], "text/calendar; charset=utf-8");
    assert!(feed.text.starts_with("BEGIN:VCALENDAR\r\n"), "{}", feed.text);
    assert!(feed.text.contains("SUMMARY:Switch upgrade\r\n"));
    assert!(feed.text.contains("DTSTART:20990105T220000Z\r\n"));
    assert!(feed.text.contains("DTEND:20990105T233000Z\r\n"));
    assert!(feed.text.contains("DESCRIPTION:Short outage\\, expected.\r\n"));
    assert!(!feed.text.contains("Disk failure"), "incidents are not maintenance");

    // Une page absente ou non publiée n'a pas de calendrier.
    let missing = raw(&app, "/api/public/status/nope/maintenance.ics", None).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_private_page_feed_follows_the_page_access() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page(&app, &admin).await;
    let access = json!({ "mode": "password", "password": "board-secret-1" });
    let set = app.put(&format!("/api/status-pages/{id}/access"), access, Some(&admin)).await;
    assert_eq!(set.status, StatusCode::OK, "{}", set.body);

    let denied = raw(&app, "/api/public/status/acme/maintenance.ics", None).await;
    assert_eq!(denied.status, StatusCode::UNAUTHORIZED);

    let unlocked = app
        .post("/api/public/status/acme/access", json!({ "password": "board-secret-1" }), None)
        .await;
    let cookie = unlocked.cookie();
    let feed = raw(&app, "/api/public/status/acme/maintenance.ics", Some(&cookie)).await;
    assert_eq!(feed.status, StatusCode::OK);
    assert_eq!(feed.headers[header::CACHE_CONTROL], "private, no-store");
}

#[tokio::test]
async fn the_private_feed_needs_its_token() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    announce(
        &app,
        &admin,
        json!({ "title": "Power test", "kind": "maintenance", "status": "scheduled",
                "starts_at": "2099-03-01T08:00:00Z", "ends_at": "2099-03-01T09:00:00Z" }),
    )
    .await;

    // Le jeton se lit en administrateur seulement.
    assert_eq!(app.get("/api/calendar/maintenance", None).await.status, StatusCode::UNAUTHORIZED);
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get("/api/calendar/maintenance", Some(&viewer)).await.status, StatusCode::FORBIDDEN);

    let shown = app.get("/api/calendar/maintenance", Some(&admin)).await;
    assert_eq!(shown.status, StatusCode::OK, "{}", shown.body);
    let token = shown.body["token"].as_str().unwrap().to_string();
    assert_eq!(app.get("/api/calendar/maintenance", Some(&admin)).await.body["token"], token.as_str());

    assert_eq!(raw(&app, "/api/calendar/maintenance.ics", None).await.status, StatusCode::NOT_FOUND);
    let wrong = raw(&app, "/api/calendar/maintenance.ics?token=nope", None).await;
    assert_eq!(wrong.status, StatusCode::NOT_FOUND);
    let feed = raw(&app, &format!("/api/calendar/maintenance.ics?token={token}"), None).await;
    assert_eq!(feed.status, StatusCode::OK);
    assert!(feed.text.contains("SUMMARY:Power test\r\n"), "{}", feed.text);

    // Tourner le jeton invalide l'ancien lien.
    let rotated = app.post("/api/calendar/maintenance/rotate", json!({}), Some(&admin)).await;
    assert_eq!(rotated.status, StatusCode::OK);
    assert_ne!(rotated.body["token"], token.as_str());
    let old = raw(&app, &format!("/api/calendar/maintenance.ics?token={token}"), None).await;
    assert_eq!(old.status, StatusCode::NOT_FOUND);
}
