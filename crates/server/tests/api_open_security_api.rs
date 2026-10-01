//! L'API ouverte et sa sécurité : spécification OpenAPI publique, version
//! annoncée, CORS réservé aux jetons, et ce qui borne un jeton d'API —
//! expiration, réseaux autorisés, dernier usage (date et adresse), plafond
//! d'appels, compte propriétaire, journalisation des écritures.

mod common;

use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode, header};
use common::{PASSWORD, TestApp};
use dumbmonit_server::db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Réponse complète, en-têtes compris.
struct Full {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

/// Requête construite à la main : jeton, cookie, adresse du client, en-têtes.
#[derive(Default)]
struct Call<'a> {
    method: &'a str,
    uri: &'a str,
    token: Option<&'a str>,
    cookie: Option<&'a str>,
    peer: Option<&'a str>,
    headers: Vec<(&'a str, &'a str)>,
    body: Option<Value>,
}

async fn send(app: &TestApp, call: Call<'_>) -> Full {
    let mut builder = Request::builder().method(call.method).uri(call.uri);
    if let Some(token) = call.token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(cookie) = call.cookie {
        builder = builder.header(header::COOKIE, cookie).header("x-requested-with", "DumbMonit");
    }
    for (name, value) in &call.headers {
        builder = builder.header(*name, *value);
    }
    let mut request = match call.body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    if let Some(peer) = call.peer {
        let addr: SocketAddr = peer.parse().expect("adresse");
        request.extensions_mut().insert(ConnectInfo(addr));
    }
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    Full { status, headers, body: serde_json::from_slice(&bytes).unwrap_or(Value::Null) }
}

fn get<'a>(uri: &'a str, token: &'a str) -> Call<'a> {
    Call { method: "GET", uri, token: Some(token), ..Default::default() }
}

/// Crée un jeton par la session ; rend le corps de la réponse.
async fn create_token(app: &TestApp, cookie: &str, payload: Value) -> Value {
    let reply = app.post("/api/tokens", payload, Some(cookie)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "création du jeton : {}", reply.body);
    reply.body
}

fn secret(created: &Value) -> String {
    created["secret"].as_str().expect("secret").to_string()
}

/// Seconde connexion à la base de l'instance de test, pour vieillir un jeton.
async fn pool(app: &TestApp) -> sqlx::SqlitePool {
    let config = common::base_config(app._dir.path());
    db::open(&config.database_path()).await.expect("base")
}

// --------------------------------------------------------------------------
// Spécification, version, CORS
// --------------------------------------------------------------------------

#[tokio::test]
async fn the_openapi_specification_is_public_and_versioned() {
    let app = TestApp::configured().await;
    let reply =
        send(&app, Call { method: "GET", uri: "/api/openapi.json", ..Default::default() }).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.headers[header::CONTENT_TYPE].to_str().unwrap().starts_with("application/json"),
        "{:?}",
        reply.headers
    );
    let spec = &reply.body;
    assert!(spec["openapi"].as_str().unwrap().starts_with("3.1"));
    assert_eq!(spec["info"]["version"], dumbmonit_server::api::API_VERSION);
    assert_eq!(spec["info"]["x-server-version"], env!("CARGO_PKG_VERSION"));
    assert!(spec["paths"]["/api/targets"]["get"].is_object());
    assert!(spec["paths"]["/api/tokens"]["post"].is_object());
    assert_eq!(spec["paths"]["/api/targets"]["get"]["x-token-scope"], "read");
    assert_eq!(spec["components"]["securitySchemes"]["bearerAuth"]["scheme"], "bearer");
    assert_eq!(
        reply.headers["x-dumbmonit-api-version"],
        dumbmonit_server::api::API_VERSION,
        "la version est aussi annoncée en en-tête"
    );
}

#[tokio::test]
async fn every_api_response_announces_the_api_version() {
    let app = TestApp::configured().await;
    for uri in ["/api/health", "/api/targets", "/api/no-such-route"] {
        let reply = send(&app, Call { method: "GET", uri, ..Default::default() }).await;
        assert_eq!(
            reply.headers.get("x-dumbmonit-api-version").and_then(|v| v.to_str().ok()),
            Some(dumbmonit_server::api::API_VERSION),
            "{uri} ({})",
            reply.status
        );
    }
}

#[tokio::test]
async fn cors_opens_the_api_to_tokens_and_never_to_the_session() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let token = secret(&create_token(&app, &admin, json!({ "name": "Dashboard" })).await);
    let origin = ("origin", "https://dash.example");

    // Préflight d'une requête à jeton : accepté, sans credentials.
    let reply = send(
        &app,
        Call {
            method: "OPTIONS",
            uri: "/api/targets",
            headers: vec![
                origin,
                ("access-control-request-method", "GET"),
                ("access-control-request-headers", "authorization, content-type"),
            ],
            ..Default::default()
        },
    )
    .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(reply.headers["access-control-allow-origin"], "*");
    let allowed = reply.headers["access-control-allow-headers"].to_str().unwrap();
    assert!(allowed.contains("authorization"), "{allowed}");
    assert!(
        !allowed.contains("x-requested-with"),
        "la preuve anti-CSRF ne traverse pas : {allowed}"
    );
    assert!(reply.headers.get("access-control-allow-credentials").is_none());

    // Préflight sans jeton annoncé : c'est une session qui voudrait passer.
    let reply = send(
        &app,
        Call {
            method: "OPTIONS",
            uri: "/api/targets",
            headers: vec![
                origin,
                ("access-control-request-method", "POST"),
                ("access-control-request-headers", "content-type, x-requested-with"),
            ],
            ..Default::default()
        },
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert!(reply.headers.get("access-control-allow-origin").is_none());

    // Requête à jeton depuis une autre origine : lisible, en-têtes exposés.
    let mut call = get("/api/targets", &token);
    call.headers.push(origin);
    let reply = send(&app, call).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers["access-control-allow-origin"], "*");
    let exposed = reply.headers["access-control-expose-headers"].to_str().unwrap();
    assert!(exposed.contains("ratelimit-remaining") && exposed.contains("retry-after"));
    assert!(reply.headers.get("access-control-allow-credentials").is_none());

    // La même lecture par cookie : aucun en-tête CORS, le navigateur bloque.
    let reply = send(
        &app,
        Call {
            method: "GET",
            uri: "/api/targets",
            cookie: Some(&admin),
            headers: vec![origin],
            ..Default::default()
        },
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.headers.get("access-control-allow-origin").is_none());

    // La spécification, publique, se lit de partout.
    let reply = send(
        &app,
        Call {
            method: "GET",
            uri: "/api/openapi.json",
            headers: vec![origin],
            ..Default::default()
        },
    )
    .await;
    assert_eq!(reply.headers["access-control-allow-origin"], "*");
}

#[tokio::test]
async fn a_token_needs_no_csrf_header_but_a_cookie_does() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let token =
        secret(&create_token(&app, &admin, json!({ "name": "CI", "scope": "write" })).await);
    let device = json!({ "name": "nas", "address": "nas.lan", "kind": "dummy" });

    let reply = send(
        &app,
        Call {
            method: "POST",
            uri: "/api/targets",
            token: Some(&token),
            body: Some(device.clone()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);

    // Un cookie sans l'en-tête, depuis une autre origine : refusé.
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/targets")
        .header(header::COOKIE, &admin)
        .header("sec-fetch-site", "cross-site")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "name": "nas2", "address": "nas2.lan", "kind": "dummy" }).to_string(),
        ))
        .unwrap();
    request.extensions_mut().insert(ConnectInfo("192.0.2.1:5000".parse::<SocketAddr>().unwrap()));
    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// --------------------------------------------------------------------------
// Création et liste
// --------------------------------------------------------------------------

#[tokio::test]
async fn a_token_is_created_with_an_expiry_and_networks() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    for (payload, needle) in [
        (json!({ "name": "x", "expires_in_days": 0 }), "expires_in_days"),
        (json!({ "name": "x", "expires_in_days": 5000 }), "expires_in_days"),
        (json!({ "name": "x", "allowed_networks": ["nas.lan"] }), "Invalid network \"nas.lan\""),
        (json!({ "name": "x", "allowed_networks": ["10.0.0.0/40"] }), "Invalid network"),
        (json!({ "name": "x", "scope": "admin" }), "read, write"),
    ] {
        let reply = app.post("/api/tokens", payload.clone(), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{payload}: {}", reply.body);
        assert!(reply.body["error"].as_str().unwrap().contains(needle), "{}", reply.body);
    }

    let created = create_token(
        &app,
        &admin,
        json!({
            "name": "Grafana",
            "scope": "read",
            "expires_in_days": 90,
            "allowed_networks": ["192.168.1.7/24", "10.0.0.5", " ", "192.168.1.0/24"],
        }),
    )
    .await;
    assert!(secret(&created).starts_with("dmt_"));
    assert_eq!(created["allowed_networks"], json!(["192.168.1.0/24", "10.0.0.5/32"]));
    assert_eq!(created["created_by"], "admin");
    assert_eq!(created["expired"], false);
    let expires = chrono::DateTime::parse_from_rfc3339(created["expires_at"].as_str().unwrap())
        .expect("RFC 3339");
    let days = (expires.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days();
    assert!((89..=90).contains(&days), "{days}");

    // Sans expiration ni réseau : comme avant.
    let plain = create_token(&app, &admin, json!({ "name": "Forever" })).await;
    assert!(plain["expires_at"].is_null());
    assert_eq!(plain["allowed_networks"], json!([]));

    // La liste ne montre jamais le secret, ni son empreinte.
    let list = app.get("/api/tokens", Some(&admin)).await;
    let text = list.body.to_string();
    assert!(!text.contains(&secret(&created)) && !text.contains("token_hash"), "{text}");
    assert_eq!(list.body.as_array().unwrap().len(), 2);
}

// --------------------------------------------------------------------------
// Ce qui borne un jeton
// --------------------------------------------------------------------------

#[tokio::test]
async fn an_expired_token_is_refused_and_says_so() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created =
        create_token(&app, &admin, json!({ "name": "Short-lived", "expires_in_days": 30 })).await;
    let token = secret(&created);

    assert_eq!(send(&app, get("/api/targets", &token)).await.status, StatusCode::OK);

    // On vieillit le jeton plutôt que d'attendre trente jours.
    let pool = pool(&app).await;
    sqlx::query("UPDATE api_tokens SET expires_at = '2020-01-01T00:00:00Z' WHERE id = ?")
        .bind(created["id"].as_i64().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let reply = send(&app, get("/api/targets", &token)).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(
        reply.body["error"].as_str().unwrap().contains("expired on 2020-01-01"),
        "{}",
        reply.body
    );
    let challenge = reply.headers[header::WWW_AUTHENTICATE].to_str().unwrap();
    assert!(challenge.contains("invalid_token"), "{challenge}");

    // Le point d'entrée MCP applique la même règle.
    let reply = send(
        &app,
        Call {
            method: "POST",
            uri: "/api/mcp",
            token: Some(&token),
            body: Some(json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" })),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);

    let list = app.get("/api/tokens", Some(&admin)).await;
    assert_eq!(list.body[0]["expired"], true);
}

#[tokio::test]
async fn a_network_restricted_token_only_works_from_its_networks() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = create_token(
        &app,
        &admin,
        json!({ "name": "LAN only", "allowed_networks": ["192.168.1.0/24"] }),
    )
    .await;
    let token = secret(&created);

    let mut call = get("/api/targets", &token);
    call.peer = Some("10.0.0.5:40000");
    let reply = send(&app, call).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let message = reply.body["error"].as_str().unwrap();
    assert!(message.contains("cannot be used from 10.0.0.5"), "{message}");
    assert!(!message.contains("192.168.1.0"), "le refus ne livre pas la liste : {message}");

    // Sans adresse connue, le doute profite à la sécurité.
    assert_eq!(send(&app, get("/api/targets", &token)).await.status, StatusCode::FORBIDDEN);

    let mut call = get("/api/targets", &token);
    call.peer = Some("192.168.1.20:40000");
    assert_eq!(send(&app, call).await.status, StatusCode::OK);

    // IPv4 vue à travers une socket double pile.
    let mut call = get("/api/targets", &token);
    call.peer = Some("[::ffff:192.168.1.21]:40000");
    assert_eq!(send(&app, call).await.status, StatusCode::OK);

    // Le dernier usage retient la date et l'adresse.
    let list = app.get("/api/tokens", Some(&admin)).await;
    let row = &list.body[0];
    assert!(row["last_used_at"].is_string(), "{row}");
    assert_eq!(row["last_used_ip"], "192.168.1.21", "l'adresse change : mise à jour immédiate");
}

#[tokio::test]
async fn behind_a_trusted_proxy_the_forwarded_address_counts() {
    let app = common::setup_with(|config| {
        config.trusted_proxies = vec!["127.0.0.1/32".parse().unwrap()];
    })
    .await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    let token = secret(
        &create_token(
            &app,
            &admin,
            json!({ "name": "Proxied", "allowed_networks": ["203.0.113.0/24"] }),
        )
        .await,
    );

    let mut call = get("/api/targets", &token);
    call.peer = Some("127.0.0.1:50000");
    call.headers.push(("x-forwarded-for", "203.0.113.9"));
    assert_eq!(send(&app, call).await.status, StatusCode::OK);

    // Le même en-tête venant d'un client direct n'est pas cru.
    let mut call = get("/api/targets", &token);
    call.peer = Some("198.51.100.4:50000");
    call.headers.push(("x-forwarded-for", "203.0.113.9"));
    assert_eq!(send(&app, call).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn each_token_has_a_rate_limit_with_headers() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    // Le compteur est global au processus et indexé par identifiant de jeton :
    // on s'éloigne des identifiants qu'utilisent les autres tests de ce fichier.
    for index in 0..9 {
        create_token(&app, &admin, json!({ "name": format!("filler {index}") })).await;
    }
    let created = create_token(&app, &admin, json!({ "name": "Loop" })).await;
    assert_eq!(created["id"], 10);
    let token = secret(&created);

    let reply = send(&app, get("/api/alerts", &token)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers["ratelimit-limit"], "120");
    assert_eq!(reply.headers["ratelimit-remaining"], "119");
    let reset: u64 = reply.headers["ratelimit-reset"].to_str().unwrap().parse().unwrap();
    assert!((1..=61).contains(&reset), "{reset}");

    for _ in 1..120 {
        assert_eq!(send(&app, get("/api/alerts", &token)).await.status, StatusCode::OK);
    }
    let reply = send(&app, get("/api/alerts", &token)).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    let retry: u64 = reply.headers[header::RETRY_AFTER].to_str().unwrap().parse().unwrap();
    assert!((1..=61).contains(&retry), "{retry}");
    assert_eq!(reply.headers["ratelimit-remaining"], "0");
    assert!(reply.body["error"].as_str().unwrap().contains("120 calls per minute"));

    // Le plafond est par jeton : un autre passe.
    let other = secret(&create_token(&app, &admin, json!({ "name": "Other" })).await);
    assert_eq!(send(&app, get("/api/alerts", &other)).await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_token_is_worth_what_its_owner_is() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = app
        .post(
            "/api/users",
            json!({ "username": "alice", "role": "admin", "password": "mot-de-passe-d-alice" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let alice_id = reply.body["id"].as_i64().unwrap();
    let alice = app.login_as("alice", "mot-de-passe-d-alice").await.cookie();
    let created = create_token(&app, &alice, json!({ "name": "Alice CI", "scope": "write" })).await;
    assert_eq!(created["created_by"], "alice");
    let token = secret(&created);
    let device =
        || json!({ "name": format!("d{}", rand_suffix()), "address": "x.lan", "kind": "dummy" });

    let write = |token: String, body: Value| {
        let app = &app;
        async move {
            send(
                app,
                Call {
                    method: "POST",
                    uri: "/api/targets",
                    token: Some(&token),
                    body: Some(body),
                    ..Default::default()
                },
            )
            .await
        }
    };
    assert_eq!(write(token.clone(), device()).await.status, StatusCode::CREATED);

    // Alice redevient lectrice : son jeton `write` ne fait plus que lire.
    let reply =
        app.put(&format!("/api/users/{alice_id}"), json!({ "role": "viewer" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = write(token.clone(), device()).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert!(
        reply.body["error"].as_str().unwrap().contains("no longer an administrator"),
        "{}",
        reply.body
    );
    assert_eq!(send(&app, get("/api/targets", &token)).await.status, StatusCode::OK);

    // Désactivée : plus rien.
    let reply =
        app.put(&format!("/api/users/{alice_id}"), json!({ "disabled": true }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = send(&app, get("/api/targets", &token)).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(reply.body["error"].as_str().unwrap().contains("disabled account"), "{}", reply.body);

    // Supprimée : ses jetons sont révoqués, pas laissés orphelins.
    let reply = app.delete(&format!("/api/users/{alice_id}"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    let list = app.get("/api/tokens", Some(&admin)).await;
    let row = list.body.as_array().unwrap().iter().find(|t| t["id"] == created["id"]).unwrap();
    assert!(row["revoked_at"].is_string(), "{row}");
    assert_eq!(send(&app, get("/api/targets", &token)).await.status, StatusCode::UNAUTHORIZED);
}

fn rand_suffix() -> u32 {
    rand::random::<u32>()
}

#[tokio::test]
async fn no_token_can_mint_a_token_or_touch_accounts() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let token =
        secret(&create_token(&app, &admin, json!({ "name": "Powerful", "scope": "write" })).await);
    for (method, uri, body) in [
        ("POST", "/api/tokens", Some(json!({ "name": "escalation", "scope": "write" }))),
        ("GET", "/api/tokens", None),
        ("POST", "/api/agent/tokens", Some(json!({ "name": "fleet" }))),
        (
            "POST",
            "/api/users",
            Some(json!({ "username": "x", "role": "admin", "password": PASSWORD })),
        ),
        ("GET", "/api/backup", None),
    ] {
        let reply =
            send(&app, Call { method, uri, token: Some(&token), body, ..Default::default() }).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {uri}: {}", reply.body);
    }
    let list = app.get("/api/tokens", Some(&admin)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 1, "aucun jeton fabriqué");
}

// --------------------------------------------------------------------------
// Journalisation
// --------------------------------------------------------------------------

#[derive(Clone)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn writes_by_token_are_logged_without_the_secret() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let token = secret(
        &create_token(&app, &admin, json!({ "name": "Provisioning", "scope": "write" })).await,
    );

    let buffer = Captured(Arc::new(Mutex::new(Vec::new())));
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let reply = send(
        &app,
        Call {
            method: "POST",
            uri: "/api/targets?note=hidden",
            token: Some(&token),
            body: Some(json!({ "name": "nas", "address": "nas.lan", "kind": "dummy",
                               "credential": { "type": "snmp_community", "community": "s3cr3t-community" } })),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    // Une lecture n'est pas une écriture : pas de ligne pour elle.
    send(&app, get("/api/targets", &token)).await;

    let logs = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    let lines: Vec<&str> = logs.lines().filter(|l| l.contains("API write by token")).collect();
    assert_eq!(lines.len(), 1, "{logs}");
    let line = lines[0];
    assert!(
        line.contains("Provisioning") && line.contains("POST") && line.contains("/api/targets"),
        "{line}"
    );
    assert!(line.contains("201"), "{line}");
    assert!(!logs.contains(&token), "le secret ne doit jamais être journalisé");
    assert!(!logs.contains("s3cr3t-community"), "ni le corps de la requête");
    assert!(!line.contains("note=hidden"), "ni la chaîne de requête : {line}");
}
