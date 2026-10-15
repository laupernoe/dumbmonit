//! Auto-surveillance : état du moteur et réglage du battement de cœur sortant.
//!
//! L'URL du battement porte souvent son propre secret : aucune réponse ne doit
//! la rendre, ni en clair ni dans un message d'erreur.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::http::StatusCode;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use common::TestApp;

const SECRET_URL: &str = "https://hc-ping.com/0d3c1a2b-secret-uuid";

/// Serveur HTTP minimal qui répond toujours `status` et compte les appels.
async fn service(status: &'static str) -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buffer = [0u8; 4096];
                let _ = socket.read(&mut buffer).await;
                let reply =
                    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                let _ = socket.write_all(reply.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    (address, calls)
}

#[tokio::test]
async fn letat_du_moteur_se_lit_et_le_battement_est_coupe_par_defaut() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply = app.get("/api/self-monitoring", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let names: Vec<&str> = reply.body["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["scheduler", "database", "victoria"]);
    // Le VictoriaMetrics des tests est injoignable : l'état le dit.
    assert_eq!(reply.body["status"], "degraded");
    assert_eq!(reply.body["components"][1]["ok"], true);
    assert_eq!(reply.body["components"][2]["ok"], false);
    assert!(reply.body["database_bytes"].as_u64().unwrap() > 0);

    let heartbeat = &reply.body["heartbeat"];
    assert_eq!(heartbeat["enabled"], false);
    assert_eq!(heartbeat["interval_seconds"], 60);
    assert_eq!(heartbeat["has_url"], false);
}

#[tokio::test]
async fn lurl_du_battement_ne_revient_jamais() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    // Pas d'URL : on ne peut pas l'allumer.
    let refused =
        app.put("/api/self-monitoring/heartbeat", json!({ "enabled": true }), Some(&admin)).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);

    let saved = app
        .put(
            "/api/self-monitoring/heartbeat",
            json!({
                "enabled": true,
                "interval_seconds": 120,
                "send_fail": true,
                "url": SECRET_URL
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    assert_eq!(saved.body["enabled"], true);
    assert_eq!(saved.body["interval_seconds"], 120);
    assert_eq!(saved.body["send_fail"], true);
    assert_eq!(saved.body["has_url"], true);
    assert_eq!(saved.body["url_host"], "hc-ping.com");
    assert!(!saved.body.to_string().contains("secret-uuid"), "{}", saved.body);

    let read = app.get("/api/self-monitoring", Some(&admin)).await;
    assert!(!read.body.to_string().contains("secret-uuid"), "{}", read.body);
    assert_eq!(read.body["heartbeat"]["has_url"], true);

    // Omise, l'URL est gardée ; vide, elle est effacée.
    let kept =
        app.put("/api/self-monitoring/heartbeat", json!({ "enabled": false }), Some(&admin)).await;
    assert_eq!(kept.status, StatusCode::OK, "{}", kept.body);
    assert_eq!(kept.body["has_url"], true);
    assert_eq!(kept.body["interval_seconds"], 120, "réglage précédent gardé");
    let cleared = app
        .put("/api/self-monitoring/heartbeat", json!({ "enabled": false, "url": "" }), Some(&admin))
        .await;
    assert_eq!(cleared.body["has_url"], false);
}

#[tokio::test]
async fn un_reglage_invalide_est_refuse() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    for body in [
        json!({ "enabled": true, "url": SECRET_URL, "interval_seconds": 5 }),
        json!({ "enabled": true, "url": SECRET_URL, "interval_seconds": 7200 }),
        json!({ "enabled": true, "url": "hc-ping.com/secret-uuid" }),
    ] {
        let reply = app.put("/api/self-monitoring/heartbeat", body.clone(), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{body}");
        assert!(!reply.body.to_string().contains("secret-uuid"), "{}", reply.body);
    }
}

#[tokio::test]
async fn seul_un_administrateur_regle_le_battement() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    let read = app.get("/api/self-monitoring", Some(&viewer)).await;
    assert_eq!(read.status, StatusCode::OK);
    let reply =
        app.put("/api/self-monitoring/heartbeat", json!({ "enabled": false }), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let test = app.post("/api/self-monitoring/heartbeat/test", json!({}), Some(&viewer)).await;
    assert_eq!(test.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn le_bouton_tester_appelle_le_service() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let (address, calls) = service("200 OK").await;

    let reply = app
        .post(
            "/api/self-monitoring/heartbeat/test",
            json!({ "url": format!("http://{address}/ping/abc-secret") }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["ok"], true, "{}", reply.body);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    // Un second essai trop rapproché attend son tour.
    let again = app.post("/api/self-monitoring/heartbeat/test", json!({}), Some(&admin)).await;
    assert_eq!(again.status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn un_refus_du_service_est_rapporte_sans_lurl() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let (address, _) = service("404 Not Found").await;

    let reply = app
        .post(
            "/api/self-monitoring/heartbeat/test",
            json!({ "url": format!("http://{address}/ping/abc-secret") }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["ok"], false);
    let error = reply.body["error"].as_str().unwrap();
    assert!(error.contains("404"), "{error}");
    assert!(!error.contains("abc-secret"), "{error}");
}

#[tokio::test]
async fn tester_sans_url_demande_den_saisir_une() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = app.post("/api/self-monitoring/heartbeat/test", json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
}
