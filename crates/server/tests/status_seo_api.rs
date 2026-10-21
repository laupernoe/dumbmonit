//! Référencement des pages de statut : `noindex` par défaut, balises rendues
//! côté serveur dans le HTML, `robots.txt` et `sitemap.xml`.

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

async fn raw(app: &TestApp, uri: &str) -> Raw {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    Raw { status, headers, text: String::from_utf8_lossy(&bytes).into_owned() }
}

async fn page(app: &TestApp, admin: &str, body: Value) -> Value {
    let reply = app.post("/api/status-pages", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "page: {}", reply.body);
    reply.body
}

async fn target(app: &TestApp, admin: &str, name: &str, address: &str, group: &str) -> i64 {
    let body = json!({ "name": name, "address": address, "kind": "dummy", "group_name": group });
    let reply = app.post("/api/targets", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "target: {}", reply.body);
    reply.body["id"].as_i64().unwrap()
}

fn robots_header(raw: &Raw) -> &str {
    raw.headers.get("x-robots-tag").and_then(|v| v.to_str().ok()).unwrap_or_default()
}

#[tokio::test]
async fn a_new_page_is_noindex_and_closed_to_robots() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let body = json!({ "title": "Acme", "slug": "acme", "published": true });
    let created = page(&app, &admin, body).await;
    assert_eq!(created["seo"]["index"], false);

    let html = raw(&app, "/s/acme").await;
    assert_eq!(html.status, StatusCode::OK);
    assert_eq!(robots_header(&html), "noindex, nofollow");
    assert!(html.text.contains("name=\"robots\" content=\"noindex, nofollow\""));
    assert!(!html.text.contains("rel=\"canonical\""));

    let robots = raw(&app, "/robots.txt").await;
    assert_eq!(robots.status, StatusCode::OK);
    assert_eq!(robots.text, "User-agent: *\nDisallow: /\n");
    let sitemap = raw(&app, "/sitemap.xml").await;
    assert!(!sitemap.text.contains("<url>"));
}

#[tokio::test]
async fn an_indexable_page_renders_its_tags_on_the_server() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    page(
        &app,
        &admin,
        json!({
            "title": "Acme", "slug": "acme", "published": true,
            "seo": {
                "index": true,
                "title": "Acme \"status\" <b>",
                "description": "Is Acme up?",
                "lang": "fr",
                "image": "https://cdn.example/og.png"
            }
        }),
    )
    .await;

    let html = raw(&app, "/s/acme").await;
    assert_eq!(robots_header(&html), "index, follow");
    assert!(html.text.contains("<title>Acme &quot;status&quot; &lt;b&gt;</title>"));
    assert!(html.text.contains("content=\"Is Acme up?\""));
    assert!(html.text.contains("<html lang=\"fr\""));
    assert!(html.text.contains("property=\"og:image\" content=\"https://cdn.example/og.png\""));
    assert!(html.text.contains("name=\"twitter:card\" content=\"summary_large_image\""));
    assert!(html.text.contains("application/ld+json"));
    assert!(!html.text.contains("<b>"), "rien n'est injecté brut");

    // Les variantes de la page ne sont jamais indexées.
    let embed = raw(&app, "/s/acme/embed").await;
    assert_eq!(robots_header(&embed), "noindex, nofollow");

    let robots = raw(&app, "/robots.txt").await;
    assert!(robots.text.contains("Allow: /s/acme$"));
    assert!(robots.text.contains("Disallow: /\n"));
    let sitemap = raw(&app, "/sitemap.xml").await;
    assert_eq!(sitemap.headers[header::CONTENT_TYPE], "application/xml; charset=utf-8");
    assert!(sitemap.text.contains("/s/acme</loc>"));
}

#[tokio::test]
async fn unpublished_or_unknown_pages_stay_noindex_and_leak_nothing() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    page(
        &app,
        &admin,
        json!({
            "title": "Secret roadmap", "slug": "hidden", "published": false,
            "seo": { "index": true, "title": "Secret roadmap" }
        }),
    )
    .await;
    for uri in ["/s/hidden", "/s/nothing-here"] {
        let html = raw(&app, uri).await;
        assert_eq!(robots_header(&html), "noindex, nofollow", "{uri}");
        assert!(!html.text.contains("Secret roadmap"), "{uri}");
    }
    assert!(!raw(&app, "/sitemap.xml").await.text.contains("hidden"));
    assert!(!raw(&app, "/robots.txt").await.text.contains("hidden"));
}

#[tokio::test]
async fn seo_settings_are_validated_and_kept_when_omitted() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = page(
        &app,
        &admin,
        json!({ "title": "Acme", "slug": "acme", "seo": { "index": true, "keywords": "a, b" } }),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    for bad in [
        json!({ "canonical": "javascript:alert(1)" }),
        json!({ "image": "/relative.png" }),
        json!({ "lang": "not a tag" }),
        json!({ "title": "x".repeat(121) }),
    ] {
        let body = json!({ "title": "Acme", "seo": bad });
        let reply = app.put(&format!("/api/status-pages/{id}"), body, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}: {}", reply.body);
    }

    let reply =
        app.put(&format!("/api/status-pages/{id}"), json!({ "title": "Acme" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["seo"]["index"], true);
    assert_eq!(reply.body["seo"]["keywords"], "a, b");
}

#[tokio::test]
async fn only_public_names_reach_visitors_never_private_names_addresses_or_ids() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let crm = target(&app, &admin, "SRV-CRM01", "10.66.1.2", "").await;
    let db = target(&app, &admin, "SRV-DB02", "10.66.1.3", "Rack").await;

    let created = page(
        &app,
        &admin,
        json!({
            "title": "Acme", "slug": "acme", "published": true,
            "seo": { "index": true },
            "groups": ["Rack"],
            "group_labels": [{ "target_id": db, "label": "Database" }]
        }),
    )
    .await;
    assert_eq!(created["group_labels"][0]["label"], "Database");
    let id = created["id"].as_i64().unwrap();
    let items = json!([{ "target_id": crm, "label": "Customer portal" }]);
    let reply = app.put(&format!("/api/status-pages/{id}/items"), items, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "items: {}", reply.body);

    let mut surfaces = Vec::new();
    for uri in [
        "/api/public/status/acme",
        "/api/public/status/acme/rss",
        "/api/public/status/acme/badge.svg",
        "/s/acme",
        "/s/acme/embed",
        "/robots.txt",
        "/sitemap.xml",
    ] {
        let reply = raw(&app, uri).await;
        assert_ne!(reply.status, StatusCode::INTERNAL_SERVER_ERROR, "{uri}");
        surfaces.push((uri, reply.text));
    }
    for (uri, text) in &surfaces {
        for secret in ["SRV-CRM01", "SRV-DB02", "10.66.1.", "target_id", "\"address\""] {
            assert!(!text.contains(secret), "{uri} exposes {secret}");
        }
    }
    let public = &surfaces[0].1;
    assert!(public.contains("Customer portal"), "the chosen public name is shown");
    assert!(public.contains("Database"), "a group member shows its public name");
}
