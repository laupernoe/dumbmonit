//! Tests d'intégration du domaine public d'une page de statut.
//!
//! La propriété qui compte : une requête qui arrive avec le domaine d'une page
//! dans `Host` n'atteint que cette page — jamais la connexion, l'API
//! d'administration, les agents, le MCP, les métriques ni une autre page, même
//! avec un cookie de session valide.

mod common;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::TestApp;

const DOMAIN: &str = "status.acme.example";

struct Raw {
    status: StatusCode,
    headers: HeaderMap,
    text: String,
}

impl Raw {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).unwrap_or(Value::Null)
    }
}

/// Requête sous un nom d'hôte donné, avec en option un cookie de session et un
/// `X-Forwarded-Host`.
async fn on_host(
    app: &TestApp,
    method: &str,
    uri: &str,
    host: Option<&str>,
    cookie: Option<&str>,
    body: Option<Value>,
    forwarded_host: Option<&str>,
) -> Raw {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(host) = host {
        builder = builder.header(header::HOST, host);
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie).header("x-requested-with", "DumbMonit");
    }
    if let Some(forwarded) = forwarded_host {
        builder = builder.header("x-forwarded-host", forwarded);
    }
    let request = match body {
        Some(value) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    Raw { status, headers, text: String::from_utf8_lossy(&bytes).into_owned() }
}

async fn get(app: &TestApp, uri: &str, host: &str) -> Raw {
    on_host(app, "GET", uri, Some(host), None, None, None).await
}

/// Une page publiée `acme` servie sur [`DOMAIN`], et une autre page `other`.
async fn page_with_domain(app: &TestApp, admin: &str) -> i64 {
    let reply = app
        .post(
            "/api/status-pages",
            json!({ "title": "Acme", "slug": "acme", "published": true, "domain": DOMAIN }),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["domain"], DOMAIN);
    let reply = app
        .post(
            "/api/status-pages",
            json!({ "title": "Other", "slug": "other", "published": true }),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert!(reply.body["domain"].is_null());
    let pages = app.get("/api/status-pages", Some(admin)).await.body;
    pages
        .as_array()
        .unwrap()
        .iter()
        .find(|page| page["slug"] == "acme")
        .and_then(|page| page["id"].as_i64())
        .unwrap()
}

#[tokio::test]
async fn the_domain_is_normalised_unique_and_clearable() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply = app
        .post(
            "/api/status-pages",
            json!({ "title": "Acme", "slug": "acme", "domain": "HTTPS://Status.Acme.Example/" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["domain"], DOMAIN);
    let id = reply.body["id"].as_i64().unwrap();

    // Un port, un chemin, une adresse IP ou un nom sans point sont refusés.
    for bad in ["status.acme.example:8443", "status.acme.example/x", "10.0.0.1", "status", "a_b.c"]
    {
        let reply = app
            .put(
                &format!("/api/status-pages/{id}"),
                json!({ "title": "Acme", "slug": "acme", "domain": bad }),
                Some(&admin),
            )
            .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}: {}", reply.body);
    }

    // Une seconde page ne peut pas prendre le même nom.
    let reply = app
        .post(
            "/api/status-pages",
            json!({ "title": "Twin", "slug": "twin", "domain": "status.ACME.example" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body["error"].as_str().unwrap().contains("domain"), "{}", reply.body);

    // Omis : gardé. `null` : retiré.
    let reply = app
        .put(
            &format!("/api/status-pages/{id}"),
            json!({ "title": "Acme 2", "slug": "acme" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["domain"], DOMAIN);
    let reply = app
        .put(
            &format!("/api/status-pages/{id}"),
            json!({ "title": "Acme", "slug": "acme", "domain": null }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body["domain"].is_null());
    let reply = app.get(&format!("/api/status-pages/{id}"), Some(&admin)).await;
    assert!(reply.body["domain"].is_null());
}

#[tokio::test]
async fn the_admin_host_cannot_become_a_page_domain() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = on_host(
        &app,
        "POST",
        "/api/status-pages",
        Some("monit.acme.example:8080"),
        Some(&admin),
        Some(json!({ "title": "Acme", "slug": "acme", "domain": "Monit.Acme.Example" })),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.text);
    assert!(reply.text.contains("DumbMonit itself"), "{}", reply.text);
}

#[tokio::test]
async fn on_the_domain_only_the_page_is_reachable() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    page_with_domain(&app, &admin).await;

    // La racine sert l'interface, qui sait quelle page afficher.
    for host in [DOMAIN, "STATUS.acme.example:443", "status.acme.example."] {
        let root = get(&app, "/", host).await;
        assert_eq!(root.status, StatusCode::OK, "{host}");
        assert!(
            root.text.contains("<meta name=\"dumbmonit-status-page\" content=\"acme\">"),
            "{host}: the page must name its status page"
        );
        assert!(root.headers.get(header::SET_COOKIE).is_none());
        // La page est encadrable comme `/s/<slug>`, sans rien de l'interface.
        let csp = root.headers["content-security-policy"].to_str().unwrap();
        assert!(!csp.contains("frame-ancestors"), "{csp}");
        assert!(!csp.contains("spotify"), "{csp}");
    }

    // Ce qui appartient à la page.
    for uri in [
        "/s/acme",
        "/s/acme/embed",
        "/s/acme/confirm?token=x",
        "/s/acme/unsubscribe?token=x",
        "/api/public/status/acme",
        "/api/public/status/acme/badge.svg",
        "/api/public/status/acme/uptime.svg",
        "/api/public/status/acme/rss",
        "/favicon.svg",
    ] {
        let reply = get(&app, uri, DOMAIN).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri}: {}", reply.text);
    }
    // Les écritures publiques de la page répondent comme sur l'adresse habituelle.
    for uri in [
        "/api/public/status/acme/unsubscribe?token=nope",
        "/api/public/status/acme/confirm?token=nope",
    ] {
        let there = on_host(&app, "POST", uri, Some(DOMAIN), None, None, None).await;
        let here = on_host(&app, "POST", uri, None, None, None, None).await;
        assert_eq!(there.status, here.status, "{uri}");
        assert_eq!(there.text, here.text, "{uri}");
    }

    // Le flux pointe vers le domaine, jamais vers l'hôte brut de la requête.
    let rss = get(&app, "/api/public/status/acme/rss", DOMAIN).await;
    assert!(rss.text.contains("<link>https://status.acme.example/</link>"), "{}", rss.text);

    // Tout le reste n'existe pas — avec ou sans session.
    let refused: Vec<(&str, &str, Option<Value>)> = vec![
        ("GET", "/login", None),
        ("GET", "/setup", None),
        ("GET", "/settings", None),
        ("GET", "/targets", None),
        ("GET", "/status/1", None),
        ("GET", "/wall", None),
        ("GET", "/index.html", None),
        ("GET", "/s/other", None),
        ("GET", "/api/public/status/other", None),
        ("GET", "/api/public/status/other/badge.svg", None),
        ("GET", "/api/health", None),
        ("GET", "/api/openapi.json", None),
        ("GET", "/api/auth/status", None),
        ("GET", "/api/auth/me", None),
        ("POST", "/api/auth/login", Some(json!({ "password": common::PASSWORD }))),
        ("POST", "/api/auth/setup", Some(json!({ "password": "x" }))),
        ("GET", "/api/auth/oidc/start", None),
        ("GET", "/api/targets", None),
        ("GET", "/api/status-pages", None),
        ("POST", "/api/status-pages", Some(json!({ "title": "Pwned" }))),
        ("POST", "/api/ingest", Some(json!({}))),
        ("POST", "/api/mcp", Some(json!({}))),
        ("GET", "/api/push/abcdef", None),
        ("GET", "/metrics", None),
        ("GET", "/federate", None),
        ("GET", "/install.sh", None),
        ("GET", "/download/dumbmonit-agent", None),
        ("GET", "/api/public/status/acme/../../auth/status", None),
    ];
    for (method, uri, body) in refused {
        for cookie in [None, Some(admin.as_str())] {
            let reply = on_host(&app, method, uri, Some(DOMAIN), cookie, body.clone(), None).await;
            assert_eq!(
                reply.status,
                StatusCode::NOT_FOUND,
                "{method} {uri} (session: {}) must not exist on the page domain: {}",
                cookie.is_some(),
                reply.text
            );
            assert!(reply.headers.get(header::SET_COOKIE).is_none(), "{method} {uri}");
            assert_eq!(reply.headers["x-frame-options"], "DENY", "{method} {uri}");
        }
    }

    // Aucune page n'a été créée par la requête refusée.
    let pages = app.get("/api/status-pages", Some(&admin)).await;
    assert_eq!(pages.body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn other_hosts_keep_the_whole_instance() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    page_with_domain(&app, &admin).await;

    for host in [None, Some("monit.acme.example"), Some("192.168.1.10:8080")] {
        let reply = on_host(&app, "GET", "/api/auth/status", host, None, None, None).await;
        assert_eq!(reply.status, StatusCode::OK, "{host:?}");
        let root = on_host(&app, "GET", "/", host, None, None, None).await;
        assert!(!root.text.contains("dumbmonit-status-page"), "{host:?}");
    }

    // `X-Forwarded-Host` d'un client qui n'est pas un mandataire de confiance :
    // ignoré, seul `Host` compte.
    let reply = on_host(
        &app,
        "GET",
        "/api/auth/status",
        Some("monit.acme.example"),
        None,
        None,
        Some(DOMAIN),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["configured"], true, "{}", reply.text);
}

#[tokio::test]
async fn changing_or_removing_the_domain_takes_effect_at_once() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let id = page_with_domain(&app, &admin).await;
    assert_eq!(get(&app, "/api/auth/status", DOMAIN).await.status, StatusCode::NOT_FOUND);

    let reply = app
        .put(
            &format!("/api/status-pages/{id}"),
            json!({
                "title": "Acme", "slug": "acme", "published": true,
                "domain": "status2.acme.example"
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(get(&app, "/api/auth/status", DOMAIN).await.status, StatusCode::OK);
    assert_eq!(
        get(&app, "/api/auth/status", "status2.acme.example").await.status,
        StatusCode::NOT_FOUND
    );

    // Le slug change : le domaine suit la page.
    let reply = app
        .put(
            &format!("/api/status-pages/{id}"),
            json!({ "title": "Acme", "slug": "acme-new", "published": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let root = get(&app, "/", "status2.acme.example").await;
    assert!(root.text.contains("content=\"acme-new\""));
    assert_eq!(
        get(&app, "/api/public/status/acme-new", "status2.acme.example").await.status,
        StatusCode::OK
    );

    let reply = app.delete(&format!("/api/status-pages/{id}"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(get(&app, "/api/auth/status", "status2.acme.example").await.status, StatusCode::OK);
}
