//! Imports Gatus et UptimeRobot (CSV) de bout en bout : droits, aperçu, création,
//! doublons. Les imports par clé d'API appellent un service tiers : leur conversion
//! est couverte par les tests unitaires de `api/monitor_imports.rs`.

mod common;

use std::sync::Arc;

use axum::http::StatusCode;
use dumbmonit_server::collectors::{
    self, DnsCollector, HttpCollector, PingCollector, PushCollector, TcpCollector,
};
use dumbmonit_server::db;
use serde_json::{Value, json};

use common::TestApp;

const GATUS: &str = "endpoints:\n  - name: Website\n    group: core\n    url: \"https://example.org/health\"\n    interval: 30s\n  - name: Database\n    url: \"tcp://db.lan:5432\"\n  - name: Mail\n    url: \"starttls://mail.example.org:587\"\n";

const CSV: &str = "Friendly Name,URL / IP,Type,Port,Interval\nBlog,https://blog.example.com,HTTP(s),,5 minutes\nNAS,nas.lan,Port,22,1 minute\n";

async fn lab() -> (TestApp, String) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(HttpCollector::new()));
    registry.register(Arc::new(PingCollector::new()));
    registry.register(Arc::new(TcpCollector::new()));
    registry.register(Arc::new(DnsCollector::new()));
    registry.register(Arc::new(PushCollector::new(pool.clone())));
    let app = common::build_with_registry(dir, config, pool, registry).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    (app, admin)
}

async fn count(app: &TestApp, admin: &str) -> usize {
    app.get("/api/targets", Some(admin)).await.body.as_array().expect("liste").len()
}

#[tokio::test]
async fn gatus_apercu_puis_import_puis_doublons() {
    let (app, admin) = lab().await;
    let url = "/api/targets/import/gatus";
    let body = json!({ "yaml": GATUS });

    let anonymous = app.post(url, body.clone(), None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);

    let preview = app.post(&format!("{url}?dry_run=true"), body.clone(), Some(&admin)).await;
    assert_eq!(preview.status, StatusCode::OK, "{}", preview.body);
    assert_eq!((preview.body["created"].clone(), preview.body["skipped"].clone()), (json!(2), json!(1)));
    assert_eq!(count(&app, &admin).await, 0, "a preview creates nothing");

    let done = app.post(url, body.clone(), Some(&admin)).await;
    assert_eq!(done.status, StatusCode::CREATED, "{}", done.body);
    assert_eq!(done.body["created"], json!(2));
    let rows: Vec<Value> = done.body["rows"].as_array().unwrap().clone();
    assert!(rows.iter().any(|r| r["name"] == "Website" && r["status"] == "created"));
    assert_eq!(count(&app, &admin).await, 2);

    let again = app.post(url, body, Some(&admin)).await;
    assert_eq!(again.body["created"], json!(0), "{}", again.body);

    let bad = app.post(url, json!({ "yaml": "hello: world" }), Some(&admin)).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn uptimerobot_csv_est_importe_et_les_lignes_cochees_seules() {
    let (app, admin) = lab().await;
    let url = "/api/targets/import/uptimerobot";
    let body = json!({ "csv": CSV });

    let preview = app.post(&format!("{url}?dry_run=true"), body.clone(), Some(&admin)).await;
    assert_eq!(preview.status, StatusCode::OK, "{}", preview.body);
    assert_eq!(preview.body["created"], json!(2));

    // Seule la première ligne de l'aperçu est cochée.
    let done = app.post(&format!("{url}?rows=1"), body, Some(&admin)).await;
    assert_eq!(done.status, StatusCode::CREATED, "{}", done.body);
    assert_eq!(done.body["created"], json!(1));
    assert_eq!(count(&app, &admin).await, 1);

    let empty = app.post(url, json!({}), Some(&admin)).await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
}
