//! Abonnés SMS, webhook et Slack d'une page de statut : confirmation, envoi
//! signé, choix des services, désabonnement, vue d'administration masquée.
//!
//! Un seul test dans ce binaire : les limiteurs du point d'inscription sont
//! propres au processus.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tower::ServiceExt;

use common::TestApp;

/// Une requête reçue par le faux serveur.
#[derive(Debug, Clone)]
struct Received {
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Received {
    fn header(&self, name: &str) -> String {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    }
}

/// Faux serveur HTTP : répond 200 à tout, garde ce qu'il reçoit.
async fn fake_http() -> (u16, Arc<Mutex<Vec<Received>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("écoute");
    let port = listener.local_addr().unwrap().port();
    let store = Arc::new(Mutex::new(Vec::new()));
    let seen = store.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { return };
            let seen = seen.clone();
            tokio::spawn(async move {
                let mut raw = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let Ok(n) = stream.read(&mut chunk).await else { return };
                    if n == 0 {
                        return;
                    }
                    raw.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&raw).to_string();
                    let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };
                    let length = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                        })
                        .unwrap_or(0);
                    if body.len() < length {
                        continue;
                    }
                    let mut lines = head.lines();
                    let path = lines
                        .next()
                        .unwrap_or("")
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    let headers = lines
                        .filter_map(|l| l.split_once(':'))
                        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                        .collect();
                    seen.lock().unwrap().push(Received {
                        path,
                        headers,
                        body: body[..length].to_string(),
                    });
                    let _ = stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok",
                        )
                        .await;
                    return;
                }
            });
        }
    });
    (port, store)
}

async fn wait_for(store: &Arc<Mutex<Vec<Received>>>, path: &str, count: usize) -> Vec<Received> {
    for _ in 0..100 {
        let found: Vec<Received> =
            store.lock().unwrap().iter().filter(|r| r.path == path).cloned().collect();
        if found.len() >= count {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no {count} request(s) on {path} within 10 s");
}

fn count(store: &Arc<Mutex<Vec<Received>>>, path: &str) -> usize {
    store.lock().unwrap().iter().filter(|r| r.path == path).count()
}

fn signature_ok(secret: &str, request: &Received) -> bool {
    let timestamp = request.header("X-DumbMonit-Timestamp");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{timestamp}.{}", request.body).as_bytes());
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
        == request.header("X-DumbMonit-Signature")
}

async fn anonymous(
    app: &TestApp,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(value) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn token_of(body: &str) -> String {
    let value: Value = serde_json::from_str(body).unwrap();
    value["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn sms_webhook_and_slack_subscribers() {
    let (port, received) = fake_http().await;
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    // Base de l'instance, pour y poser deux équipements suivis.
    let db_file = std::fs::read_dir(app._dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "db"))
        .expect("base");
    let pool = dumbmonit_server::db::open(&db_file).await.expect("base");

    let reply = app
        .post(
            "/api/status-pages",
            json!({ "title": "Chan lab", "slug": "chan-lab", "published": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let page = reply.body["id"].as_i64().unwrap();
    for (name, address) in [("nas", "10.0.0.2"), ("gw", "10.0.0.1")] {
        sqlx::query(
            "INSERT INTO targets (name, address, kind, interval_secs, enabled, tags, group_name)
             VALUES (?, ?, 'dummy', 60, 1, '{}', 'Rack')",
        )
        .bind(name)
        .bind(address)
        .execute(&pool)
        .await
        .unwrap();
    }
    let nas: i64 = sqlx::query_scalar("SELECT id FROM targets WHERE name = 'nas'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let gw: i64 = sqlx::query_scalar("SELECT id FROM targets WHERE name = 'gw'")
        .fetch_one(&pool)
        .await
        .unwrap();
    for (target, label) in [(nas, "NAS"), (gw, "Gateway")] {
        sqlx::query("INSERT INTO status_page_items (page_id, target_id, label) VALUES (?, ?, ?)")
            .bind(page)
            .bind(target)
            .bind(label)
            .execute(&pool)
            .await
            .unwrap();
    }

    // Aucun canal ouvert : la route n'existe pas.
    let hook = format!("http://127.0.0.1:{port}/hook-a");
    let (status, _) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "kind": "webhook", "url": hook })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Canal SMS : un canal SMTP ne convient pas, un canal Vonage oui.
    let reply = app
        .post(
            "/api/notify/channels",
            json!({ "name": "Mail", "kind": "smtp", "settings": {
                "host": "127.0.0.1", "port": 2525, "security": "none",
                "from": "s@example.org", "to": ["o@example.org"] } }),
            Some(&admin),
        )
        .await;
    let smtp = reply.body["id"].as_i64().unwrap();
    let reply = app
        .post(
            "/api/notify/channels",
            json!({ "name": "Texto", "kind": "vonage",
                    "settings": { "from": "DumbMonit", "to": ["+33600000000"] },
                    "secrets": { "api_key": "k", "api_secret": "s" } }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let vonage = reply.body["id"].as_i64().unwrap();
    let path = format!("/api/status-pages/{page}/subscription");
    let reply = app.put(&path, json!({ "sms_channel_id": smtp }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app
        .put(
            &path,
            json!({ "sms_channel_id": vonage, "webhook": true, "slack": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = app.get(&path, Some(&admin)).await;
    assert_eq!(reply.body, json!({ "sms_channel_id": vonage, "webhook": true, "slack": true }));
    let (_, doc) = anonymous(&app, "GET", "/api/public/status/chan-lab", None).await;
    assert_eq!(doc["page"]["subscribe_kinds"], json!(["sms", "webhook", "slack"]));

    // Courriel non proposé ; Slack n'accepte que ses webhooks entrants.
    let (status, _) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "email": "a@example.net" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "kind": "slack", "url": hook })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Webhook A (toute la page) : requête signée de confirmation, secret montré une fois.
    let (status, reply) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "kind": "webhook", "url": hook })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{reply}");
    assert_eq!(reply["confirm"], "signed_request");
    let secret_a = reply["secret"].as_str().unwrap().to_string();
    assert!(secret_a.starts_with("whsec_"));
    let confirmation = wait_for(&received, "/hook-a", 1).await.remove(0);
    assert!(signature_ok(&secret_a, &confirmation), "confirmation signature");
    assert_eq!(confirmation.header("X-DumbMonit-Event"), "subscription.confirm");
    let token_a = token_of(&confirmation.body);
    let (status, _) = anonymous(
        &app,
        "POST",
        &format!("/api/public/status/chan-lab/confirm?token={token_a}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Webhook B : ne suit que le NAS.
    let hook_b = format!("http://127.0.0.1:{port}/hook-b");
    let (status, _) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "kind": "webhook", "url": hook_b, "components": ["nope"] })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, reply) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "kind": "webhook", "url": hook_b, "components": ["nas"] })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{reply}");
    let secret_b = reply["secret"].as_str().unwrap().to_string();
    let token_b = token_of(&wait_for(&received, "/hook-b", 1).await[0].body);
    let (status, _) = anonymous(
        &app,
        "POST",
        &format!("/api/public/status/chan-lab/confirm?token={token_b}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Une annonce de toute la page part vers les deux, signée.
    let reply = app
        .post(
            "/api/incidents",
            json!({ "title": "Power cut", "page_id": page, "body": "Looking into it." }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let a = wait_for(&received, "/hook-a", 2).await.remove(1);
    assert!(signature_ok(&secret_a, &a));
    let payload: Value = serde_json::from_str(&a.body).unwrap();
    assert_eq!(payload["incident"]["title"], "Power cut");
    assert_eq!(payload["message"], "Looking into it.");
    assert!(payload["unsubscribe_url"].as_str().unwrap().contains(&token_a));
    let b = wait_for(&received, "/hook-b", 2).await.remove(1);
    assert!(signature_ok(&secret_b, &b));

    // Une annonce qui ne touche que la passerelle ne va pas à B.
    let reply = app
        .post(
            "/api/incidents",
            json!({ "title": "Gateway reboot", "page_id": page, "components": [gw] }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["components"], json!([gw]));
    wait_for(&received, "/hook-a", 3).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(count(&received, "/hook-b"), 2, "B only follows the NAS");

    // Un abonné Slack déjà confirmé (l'hôte est celui de Slack en vrai) reçoit
    // un message `text` avec le lien de désabonnement.
    sqlx::query(
        "INSERT INTO status_subscribers (page_id, email, token, kind, confirmed_at)
         VALUES (?, ?, 'slacktoken', 'slack', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
    )
    .bind(page)
    .bind(format!("http://127.0.0.1:{port}/slack"))
    .execute(&pool)
    .await
    .unwrap();
    let reply = app
        .post(
            "/api/incidents",
            json!({ "title": "Slack check", "page_id": page, "components": [nas] }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let slack = wait_for(&received, "/slack", 1).await.remove(0);
    let text: Value = serde_json::from_str(&slack.body).unwrap();
    let text = text["text"].as_str().unwrap();
    assert!(
        text.contains("Slack check") && text.contains("unsubscribe?token=slacktoken"),
        "{text}"
    );

    // SMS : code par message, mauvais code refusé, bon code accepté.
    let (status, reply) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "kind": "sms", "phone": "0612345678" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{reply}");
    let (status, reply) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/subscribe",
        Some(json!({ "kind": "sms", "phone": "+33 6 12 34 56 78" })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{reply}");
    assert_eq!(reply["confirm"], "code");
    assert!(reply.get("secret").is_none());
    let (status, _) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/confirm-code",
        Some(json!({ "phone": "+33612345678", "code": "000000" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let token: String =
        sqlx::query_scalar("SELECT token FROM status_subscribers WHERE kind = 'sms'")
            .fetch_one(&pool)
            .await
            .unwrap();
    // Le code réel est parti par SMS : on pose ici une empreinte connue.
    let known = hex::encode(Sha256::digest(format!("{token}:123456").as_bytes()));
    sqlx::query("UPDATE status_subscribers SET code_hash = ? WHERE kind = 'sms'")
        .bind(known)
        .execute(&pool)
        .await
        .unwrap();
    let (status, _) = anonymous(
        &app,
        "POST",
        "/api/public/status/chan-lab/confirm-code",
        Some(json!({ "phone": "+33612345678", "code": "123456" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Vue d'administration : destinations masquées, ni jeton ni secret.
    let reply = app.get(&format!("/api/status-pages/{page}/subscribers"), Some(&admin)).await;
    let rows = reply.body.as_array().unwrap();
    let sms = rows.iter().find(|r| r["kind"] == "sms").expect("sms row");
    assert_eq!(sms["destination"], "+33•••••••78");
    assert!(sms["email"].is_null());
    assert!(!sms["confirmed_at"].is_null());
    let web_b =
        rows.iter().find(|r| r["components"] == json!([nas]) && r["kind"] == "webhook").unwrap();
    assert_eq!(web_b["destination"], "http://127.0.0.1/…");
    let dump = reply.body.to_string();
    for forbidden in ["token", "secret", "whsec_", "code_hash", "/hook-a"] {
        assert!(!dump.contains(forbidden), "{forbidden} leaked: {dump}");
    }

    // Export CSV.
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/status-pages/{page}/subscribers.csv"))
                .header(header::COOKIE, &admin)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/csv"));
    let csv = String::from_utf8(response.into_body().collect().await.unwrap().to_bytes().to_vec())
        .unwrap();
    assert!(csv.starts_with("type,destination,status,subscribed_at,services\r\n"), "{csv}");
    assert!(csv.contains("\"sms\",\"+33612345678\",confirmed"), "{csv}");
    assert!(!csv.contains("/hook-a"), "URLs stay masked in the export");

    // Désabonnement en un clic par le jeton.
    let (status, _) = anonymous(
        &app,
        "POST",
        &format!("/api/public/status/chan-lab/unsubscribe?token={token_a}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reply = app.get(&format!("/api/status-pages/{page}/subscribers"), Some(&admin)).await;
    assert_eq!(reply.body.as_array().unwrap().len(), 3);
}
