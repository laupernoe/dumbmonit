//! Carte en direct d'un pare-feu OPNsense : éteinte sans l'option `live_map`,
//! réservée aux pare-feu, et la base IP → pays s'éteint sans rien télécharger.

mod common;

use std::sync::Arc;

use axum::http::StatusCode;
use dumbmonit_server::{collectors, db};
use serde_json::{Value, json};

use common::TestApp;

/// Une instance qui connaît le type `opnsense` (et `dummy`).
async fn app() -> TestApp {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(collectors::OpnsenseCollector::new()));
    let app = common::build_with_registry(dir, config, pool, registry).await;
    app.create_admin().await;
    app
}

async fn create(app: &TestApp, admin: &str, body: Value) -> i64 {
    let reply = app.post("/api/targets", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "création : {}", reply.body);
    reply.body["id"].as_i64().expect("id")
}

#[tokio::test]
async fn sans_l_option_la_carte_reste_eteinte() {
    let app = app().await;
    let admin = app.admin_cookie().await;
    let fw = create(
        &app,
        &admin,
        json!({ "name": "fw", "address": "fw.lan", "kind": "opnsense",
                "credential": { "type": "username_password", "username": "k", "password": "s" } }),
    )
    .await;
    let viewer = app.viewer_cookie(&admin).await;
    let reply = app.get(&format!("/api/targets/{fw}/opnsense/live"), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["enabled"], false);
    assert_eq!(reply.body["recent"], json!([]));
    assert_eq!(reply.body["window_secs"], 900);

    let other = create(&app, &admin, json!({ "name": "x", "address": "x", "kind": "dummy" })).await;
    let reply = app.get(&format!("/api/targets/{other}/opnsense/live"), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app.get("/api/targets/999/opnsense/live", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn la_geolocalisation_s_eteint_et_se_rallume() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = app.put("/api/geoip", json!({ "enabled": false }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["enabled"], false);
    assert_eq!(reply.body["state"], "disabled");
    assert_eq!(reply.body["source_url"], "https://db-ip.com");
    let reply = app.get("/api/geoip", Some(&admin)).await;
    assert_eq!(reply.body["state"], "disabled");

    let viewer = app.viewer_cookie(&admin).await;
    let reply = app.put("/api/geoip", json!({ "enabled": true }), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "un lecteur n'y touche pas");
}
