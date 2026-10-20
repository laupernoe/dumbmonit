//! Journaux du serveur : stockage local, export, destinations — et le format
//! de chaque destination, vérifié contre un faux serveur local.

mod common;

use std::io::Read;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Request, StatusCode, Uri, header};
use common::TestApp;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tower::ServiceExt;

/// Requête reçue par le faux serveur HTTP.
#[derive(Debug)]
struct Received {
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
}

/// Faux service HTTP : note chaque requête et répond `status` / `reply`.
async fn fake_http(
    status: StatusCode,
    reply: &'static str,
) -> (String, mpsc::UnboundedReceiver<Received>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let handler = move |uri: Uri, headers: HeaderMap, body: Bytes| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(Received { uri, headers, body });
            (status, reply)
        }
    };
    let app = axum::Router::new().fallback(handler);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{address}"), rx)
}

async fn admin_app() -> (TestApp, String) {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;
    (app, cookie)
}

/// Crée une destination et rend son identifiant.
async fn create(app: &TestApp, cookie: &str, body: Value) -> i64 {
    let reply = app.post("/api/logs/destinations", body, Some(cookie)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body["id"].as_i64().unwrap()
}

async fn send_test(app: &TestApp, cookie: &str, id: i64) -> Value {
    let reply =
        app.post(&format!("/api/logs/destinations/{id}/test"), json!({}), Some(cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

async fn raw_get(app: &TestApp, uri: &str, cookie: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let request =
        Request::builder().uri(uri).header(header::COOKIE, cookie).body(Body::empty()).unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, headers, bytes)
}

// --------------------------------------------------------------------------
// Accès
// --------------------------------------------------------------------------

#[tokio::test]
async fn seul_un_administrateur_lit_les_journaux() {
    let (app, admin) = admin_app().await;
    assert_eq!(app.get("/api/logs", None).await.status, StatusCode::UNAUTHORIZED);
    let viewer = app.viewer_cookie(&admin).await;
    for uri in ["/api/logs", "/api/logs/destinations", "/api/logs/export/estimate?days=1"] {
        assert_eq!(app.get(uri, Some(&viewer)).await.status, StatusCode::FORBIDDEN, "{uri}");
    }
    let (status, _, _) = raw_get(&app, "/api/logs/export?days=1", &viewer).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/logs", Some(&admin)).await.status, StatusCode::OK);
}

// --------------------------------------------------------------------------
// Stockage local et export
// --------------------------------------------------------------------------

#[tokio::test]
async fn retention_reglable_et_bornee() {
    let (app, cookie) = admin_app().await;
    let reply = app.get("/api/logs", Some(&cookie)).await;
    assert_eq!(reply.body["retention_days"], 14);
    assert_eq!(reply.body["max_total_mb"], 1024);

    let reply = app
        .put(
            "/api/logs/settings",
            json!({ "retention_days": 30, "max_total_mb": 2048 }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["retention_days"], 30);
    assert_eq!(reply.body["max_total_mb"], 2048);

    for bad in [
        json!({ "retention_days": 0 }),
        json!({ "retention_days": 400 }),
        json!({ "max_total_mb": 1 }),
    ] {
        let reply = app.put("/api/logs/settings", bad.clone(), Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}");
    }
    // L'export ne va pas au-delà de la rétention.
    let reply = app.get("/api/logs/export/estimate?days=31", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
}

fn write_logs(dir: &std::path::Path) {
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let today = chrono::Utc::now().date_naive();
    let mut lines = String::new();
    for i in 0..500 {
        lines.push_str(&format!("{{\"level\":\"INFO\",\"msg\":\"line {i}\"}}\n"));
    }
    for back in [0u64, 1, 3, 20] {
        let day = today - chrono::Days::new(back);
        let name = format!("dumbmonit-{}.log", day.format("%Y-%m-%d"));
        if back == 0 {
            std::fs::write(logs.join(name), &lines).unwrap();
        } else {
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            std::io::Write::write_all(&mut encoder, lines.as_bytes()).unwrap();
            std::fs::write(logs.join(format!("{name}.gz")), encoder.finish().unwrap()).unwrap();
        }
    }
}

/// Noms des pièces d'une archive tar.gz.
fn tar_names(archive: &[u8]) -> Vec<String> {
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(archive).read_to_end(&mut raw).unwrap();
    let mut names = Vec::new();
    let mut offset = 0;
    while offset + 512 <= raw.len() && raw[offset] != 0 {
        let head = &raw[offset..offset + 512];
        names.push(String::from_utf8_lossy(&head[..100]).trim_end_matches('\0').to_string());
        let size =
            usize::from_str_radix(String::from_utf8_lossy(&head[124..135]).trim_matches('\0'), 8)
                .unwrap();
        offset += 512 + size.div_ceil(512) * 512;
    }
    names
}

#[tokio::test]
async fn estimation_puis_telechargement_en_flux() {
    let (app, cookie) = admin_app().await;
    write_logs(app._dir.path());

    let overview = app.get("/api/logs", Some(&cookie)).await.body;
    assert_eq!(overview["days_available"], 4);

    let estimate = app.get("/api/logs/export/estimate?days=7", Some(&cookie)).await;
    assert_eq!(estimate.status, StatusCode::OK, "{}", estimate.body);
    assert_eq!(estimate.body["files"], 3);
    let estimated = estimate.body["bytes"].as_u64().unwrap();

    let (status, headers, archive) = raw_get(&app, "/api/logs/export?days=7", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/gzip");
    assert!(headers[header::CONTENT_DISPOSITION].to_str().unwrap().contains("-7d.tar.gz"));
    let gap = estimated.abs_diff(archive.len() as u64) as f64 / archive.len() as f64;
    assert!(
        gap < 0.15 || estimated.abs_diff(archive.len() as u64) < 2048,
        "estimé {estimated}, réel {}",
        archive.len()
    );

    let names = tar_names(&archive);
    assert_eq!(names.len(), 4, "{names:?}");
    assert!(names[0].ends_with(".log.gz"));
    assert!(names[2].ends_with(".log"));
    // La connexion de l'administrateur est passée par le journal d'audit.
    assert_eq!(names[3], "audit.jsonl");
}

// --------------------------------------------------------------------------
// Destinations : secrets
// --------------------------------------------------------------------------

#[tokio::test]
async fn les_secrets_ne_ressortent_jamais() {
    let (app, cookie) = admin_app().await;
    let kinds = app.get("/api/logs/kinds", Some(&cookie)).await.body;
    let kinds: Vec<&str> =
        kinds.as_array().unwrap().iter().map(|k| k["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["gelf", "victorialogs", "loki", "elasticsearch", "syslog"]);

    let id = create(
        &app,
        &cookie,
        json!({
            "name": "Loki", "kind": "loki", "min_level": "warn",
            "settings": { "url": "http://127.0.0.1:9/" },
            "secrets": { "token": "tres-secret-jeton-loki" }
        }),
    )
    .await;
    let list = app.get("/api/logs/destinations", Some(&cookie)).await;
    assert!(!list.body.to_string().contains("tres-secret-jeton-loki"));
    assert_eq!(list.body[0]["has_secret"], true);
    assert_eq!(list.body[0]["min_level"], "warn");

    // Renommer sans ressaisir le jeton : il reste.
    let reply = app
        .put(
            &format!("/api/logs/destinations/{id}"),
            json!({ "name": "Loki prod", "kind": "loki", "settings": { "url": "http://127.0.0.1:9/" } }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["has_secret"], true);

    // Changer d'adresse sans le ressaisir : refusé.
    let reply = app
        .put(
            &format!("/api/logs/destinations/{id}"),
            json!({ "name": "Loki", "kind": "loki", "settings": { "url": "http://ailleurs:3100" } }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    // Un secret glissé dans les réglages est refusé ; `{}` efface.
    let reply = app
        .post(
            "/api/logs/destinations",
            json!({ "name": "x", "kind": "loki", "settings": { "url": "http://x", "token": "abc" } }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app
        .put(
            &format!("/api/logs/destinations/{id}"),
            json!({ "name": "Loki", "kind": "loki", "settings": { "url": "http://127.0.0.1:9/" }, "secrets": {} }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.body["has_secret"], false);

    assert_eq!(
        app.delete(&format!("/api/logs/destinations/{id}"), Some(&cookie)).await.status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn un_service_qui_recopie_le_jeton_ne_le_fait_pas_fuir() {
    let (app, cookie) = admin_app().await;
    let (url, _rx) =
        fake_http(StatusCode::UNAUTHORIZED, "invalid token jeton-qui-fuit-123 for tenant").await;
    let id = create(
        &app,
        &cookie,
        json!({ "name": "vl", "kind": "victorialogs", "settings": { "url": url }, "secrets": { "token": "jeton-qui-fuit-123" } }),
    )
    .await;
    let report = send_test(&app, &cookie, id).await;
    assert_eq!(report["ok"], false);
    let message = report["message"].as_str().unwrap();
    assert!(message.contains("401"), "{message}");
    assert!(!message.contains("jeton-qui-fuit-123"), "{message}");
}

// --------------------------------------------------------------------------
// Format de chaque destination
// --------------------------------------------------------------------------

#[tokio::test]
async fn victorialogs_jsonline() {
    let (app, cookie) = admin_app().await;
    let (url, mut rx) = fake_http(StatusCode::OK, "").await;
    let id = create(
        &app,
        &cookie,
        json!({ "name": "vl", "kind": "victorialogs", "settings": { "url": url, "username": "bob" }, "secrets": { "password": "pw-victoria" } }),
    )
    .await;
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let got = rx.recv().await.unwrap();
    assert_eq!(got.uri.path(), "/insert/jsonline");
    let query = got.uri.query().unwrap();
    assert!(query.contains("_msg_field=_msg") && query.contains("_time_field=_time"), "{query}");
    assert!(got.headers[header::AUTHORIZATION].to_str().unwrap().starts_with("Basic "));
    let text = String::from_utf8(got.body.to_vec()).unwrap();
    let line: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert!(line["_msg"].as_str().unwrap().contains("test message"));
    assert!(line["_time"].as_str().unwrap().ends_with('Z'));
    assert_eq!(line["service"], "dumbmonit");
    assert_eq!(line["level"], "info");
}

#[tokio::test]
async fn loki_push() {
    let (app, cookie) = admin_app().await;
    let (url, mut rx) = fake_http(StatusCode::NO_CONTENT, "").await;
    let id = create(
        &app,
        &cookie,
        json!({ "name": "loki", "kind": "loki", "settings": { "url": url, "tenant": "ops" }, "secrets": { "token": "loki-token-1" } }),
    )
    .await;
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let got = rx.recv().await.unwrap();
    assert_eq!(got.uri.path(), "/loki/api/v1/push");
    assert_eq!(got.headers["x-scope-orgid"], "ops");
    assert_eq!(got.headers[header::AUTHORIZATION], "Bearer loki-token-1");
    let body: Value = serde_json::from_slice(&got.body).unwrap();
    let stream = &body["streams"][0];
    assert_eq!(stream["stream"]["service_name"], "dumbmonit");
    let value = &stream["values"][0];
    assert!(value[0].as_str().unwrap().parse::<i128>().is_ok(), "horodatage en ns : {value}");
    let line: Value = serde_json::from_str(value[1].as_str().unwrap()).unwrap();
    assert!(line["msg"].as_str().unwrap().contains("test message"));
}

#[tokio::test]
async fn elasticsearch_bulk() {
    let (app, cookie) = admin_app().await;
    let (url, mut rx) = fake_http(StatusCode::OK, r#"{"took":1,"errors":false,"items":[]}"#).await;
    let id = create(
        &app,
        &cookie,
        json!({ "name": "es", "kind": "elasticsearch", "settings": { "url": url, "index": "logs-dumbmonit" }, "secrets": { "api_key": "ZXM6a2V5" } }),
    )
    .await;
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let got = rx.recv().await.unwrap();
    assert_eq!(got.uri.path(), "/_bulk");
    assert_eq!(got.headers[header::CONTENT_TYPE], "application/x-ndjson");
    assert_eq!(got.headers[header::AUTHORIZATION], "ApiKey ZXM6a2V5");
    let text = String::from_utf8(got.body.to_vec()).unwrap();
    assert!(text.ends_with('\n'));
    let lines: Vec<Value> = text.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(lines[0]["create"]["_index"], "logs-dumbmonit");
    assert!(lines[1]["@timestamp"].is_string());
    assert_eq!(lines[1]["log"]["level"], "info");
}

#[tokio::test]
async fn elasticsearch_refus_partiel_signale() {
    let (app, cookie) = admin_app().await;
    let (url, _rx) = fake_http(
        StatusCode::OK,
        r#"{"errors":true,"items":[{"create":{"status":400,"error":{"type":"x","reason":"mapping"}}}]}"#,
    )
    .await;
    let id = create(
        &app,
        &cookie,
        json!({ "name": "es", "kind": "elasticsearch", "settings": { "url": url } }),
    )
    .await;
    let report = send_test(&app, &cookie, id).await;
    assert_eq!(report["ok"], false);
    assert!(report["message"].as_str().unwrap().contains("mapping"));
}

#[tokio::test]
async fn gelf_http() {
    let (app, cookie) = admin_app().await;
    let (url, mut rx) = fake_http(StatusCode::ACCEPTED, "").await;
    let id = create(
        &app,
        &cookie,
        json!({ "name": "graylog", "kind": "gelf", "settings": { "transport": "http", "address": format!("{url}/gelf") } }),
    )
    .await;
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let got = rx.recv().await.unwrap();
    assert_eq!(got.uri.path(), "/gelf");
    let message: Value = serde_json::from_slice(&got.body).unwrap();
    assert_eq!(message["version"], "1.1");
    assert_eq!(message["level"], 6);
    assert!(message["short_message"].as_str().unwrap().contains("test message"));
    assert_eq!(message["_test"], "true");
}

#[tokio::test]
async fn gelf_udp() {
    let (app, cookie) = admin_app().await;
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    let id = create(
        &app,
        &cookie,
        json!({ "name": "graylog", "kind": "gelf", "settings": { "transport": "udp", "address": address.to_string() } }),
    )
    .await;
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let mut buffer = vec![0; 65_536];
    let (len, _) = socket.recv_from(&mut buffer).await.unwrap();
    let message: Value = serde_json::from_slice(&buffer[..len]).unwrap();
    assert_eq!(message["version"], "1.1");
    assert!(message["timestamp"].as_f64().unwrap() > 1.7e9);
}

#[tokio::test]
async fn gelf_tcp_termine_par_un_octet_nul() {
    let (app, cookie) = admin_app().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let id = create(
        &app,
        &cookie,
        json!({ "name": "graylog", "kind": "gelf", "settings": { "transport": "tcp", "address": address.to_string() } }),
    )
    .await;
    let reader = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut data = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte).await.unwrap();
            if byte[0] == 0 {
                break data;
            }
            data.push(byte[0]);
        }
    });
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let frame = reader.await.unwrap();
    let message: Value = serde_json::from_slice(&frame).unwrap();
    assert_eq!(message["version"], "1.1");
}

#[tokio::test]
async fn syslog_udp_rfc5424() {
    let (app, cookie) = admin_app().await;
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    let id = create(
        &app,
        &cookie,
        json!({ "name": "siem", "kind": "syslog", "settings": { "transport": "udp", "address": address.to_string(), "facility": "local3" } }),
    )
    .await;
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let mut buffer = vec![0; 65_536];
    let (len, _) = socket.recv_from(&mut buffer).await.unwrap();
    let line = String::from_utf8(buffer[..len].to_vec()).unwrap();
    // local3 (19) × 8 + informational (6) = 158.
    assert!(line.starts_with("<158>1 "), "{line}");
    assert!(line.contains(" dumbmonit "), "{line}");
    assert!(line.contains("[dumbmonit@32473 "), "{line}");
    assert!(line.ends_with("log shipping works"), "{line}");
}

#[tokio::test]
async fn syslog_tcp_octet_counting() {
    let (app, cookie) = admin_app().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let id = create(
        &app,
        &cookie,
        json!({ "name": "siem", "kind": "syslog", "settings": { "transport": "tcp", "address": address.to_string() } }),
    )
    .await;
    let reader = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut length = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte).await.unwrap();
            if byte[0] == b' ' {
                break;
            }
            length.push(byte[0]);
        }
        let length: usize = String::from_utf8(length).unwrap().parse().unwrap();
        let mut message = vec![0; length];
        stream.read_exact(&mut message).await.unwrap();
        String::from_utf8(message).unwrap()
    });
    assert_eq!(send_test(&app, &cookie, id).await["ok"], true);
    let message = reader.await.unwrap();
    assert!(message.starts_with("<134>1 "), "local0 + info : {message}");
    assert!(message.ends_with("log shipping works"));
}

#[tokio::test]
async fn une_destination_injoignable_est_signalee() {
    let (app, cookie) = admin_app().await;
    // Port fermé : la connexion est refusée tout de suite.
    let id = create(
        &app,
        &cookie,
        json!({ "name": "siem", "kind": "syslog", "settings": { "transport": "tcp", "address": "127.0.0.1:1" } }),
    )
    .await;
    let report = send_test(&app, &cookie, id).await;
    assert_eq!(report["ok"], false);
}
