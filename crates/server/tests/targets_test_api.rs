//! `POST /api/targets/test` (sonde sans enregistrement) et
//! `GET /api/discovery/subnets` (réseau par défaut du balayage).

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use dumbmonit_server::{collectors, db};
use serde_json::json;

async fn app() -> (common::TestApp, String) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = common::base_config(dir.path());
    config.probe_timeout = Duration::from_secs(2);
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(collectors::SnmpCollector::new()));
    let app = common::build_with_registry(dir, config, pool, registry).await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;
    (app, cookie)
}

fn snmp(address: &str) -> serde_json::Value {
    json!({
        "name": "switch", "address": address, "kind": "snmp",
        "credential": { "type": "snmp_community", "community": "public" }
    })
}

#[tokio::test]
async fn unreachable_device_reports_the_probe_error_and_saves_nothing() {
    let (app, cookie) = app().await;
    let reply = app.post("/api/targets/test", snmp("127.0.0.1:1"), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["ok"], false);
    assert!(reply.body["error"].as_str().is_some_and(|e| !e.is_empty()), "{}", reply.body);
    let list = app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(list.body.as_array().map(Vec::len), Some(0), "rien n'est enregistré");
}

#[tokio::test]
async fn non_snmp_kind_runs_a_full_probe() {
    let (app, cookie) = app().await;
    let body = json!({ "name": "d", "address": "demo", "kind": "dummy" });
    let reply = app.post("/api/targets/test", body, Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["ok"], true, "{}", reply.body);
    assert!(reply.body["sample_count"].as_u64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn validation_errors_are_400_and_viewers_are_refused() {
    let (app, cookie) = app().await;
    let reply = app.post("/api/targets/test", snmp("  "), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let viewer = app.viewer_cookie(&cookie).await;
    let reply = app.post("/api/targets/test", snmp("10.0.0.1"), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = app.post("/api/targets/test", snmp("10.0.0.1"), None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stored_secret_is_not_sent_to_a_new_address() {
    let (app, cookie) = app().await;
    let created = app.post("/api/targets", snmp("127.0.0.1:1"), Some(&cookie)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_u64().unwrap();
    let body = json!({ "id": id, "name": "switch", "address": "10.9.9.9", "kind": "snmp" });
    let reply = app.post("/api/targets/test", body, Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
}

#[tokio::test]
async fn subnets_come_from_monitored_devices_and_never_from_docker_bridges() {
    let (app, cookie) = app().await;
    for address in ["192.168.77.10", "192.168.77.11:161", "10.5.5.5"] {
        let created = app.post("/api/targets", snmp(address), Some(&cookie)).await;
        assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    }
    let reply = app.get("/api/discovery/subnets", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = reply.body["subnets"].as_array().expect("liste");
    assert_eq!(list[0]["cidr"], "192.168.77.0/24");
    assert_eq!(list[0]["source"], "devices");
    assert_eq!(list[1]["cidr"], "10.5.5.0/24");
    for subnet in list {
        let cidr = subnet["cidr"].as_str().expect("cidr");
        assert!(cidr.parse::<ipnet::Ipv4Net>().is_ok(), "{cidr}");
        assert!(["config", "devices", "server"].contains(&subnet["source"].as_str().unwrap()));
        assert!(!cidr.starts_with("172.17."), "pont Docker proposé : {cidr}");
    }
    let viewer = app.viewer_cookie(&cookie).await;
    let reply = app.get("/api/discovery/subnets", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}
