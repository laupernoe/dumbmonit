//! Carte de topologie : `GET /api/topology` assemble les liens déclarés et ceux
//! que les mesures révèlent, ouverte en lecture à tous les rôles.

mod common;

use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use serde_json::{Value, json};
use std::collections::HashMap;

use common::TestApp;

/// Un VictoriaMetrics factice : les invités de l'hyperviseur `hv` et un voisin
/// LLDP du commutateur `sw`, selon la requête reçue.
async fn fake_victoria(hv: i64, sw: i64) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new().route(
        "/api/v1/query",
        get(move |Query(params): Query<HashMap<String, String>>| async move {
            let query = params.get("query").cloned().unwrap_or_default();
            let result = if query.contains("lldp_neighbor_info") {
                json!([
                    { "metric": { "target": sw.to_string(), "remote_sysname": "hv.lan" },
                      "value": [1.0, "4"] },
                    { "metric": { "target": sw.to_string(), "remote_sysname": "phone-42" },
                      "value": [1.0, "4"] }
                ])
            } else {
                json!([
                    { "metric": { "__name__": "dumbmonit_proxmox_guest_running",
                                  "target": hv.to_string(), "name": "web01", "type": "qemu" },
                      "value": [1.0, "1"] },
                    { "metric": { "__name__": "dumbmonit_proxmox_guest_running",
                                  "target": hv.to_string(), "name": "scratch", "type": "lxc" },
                      "value": [1.0, "0"] }
                ])
            };
            axum::Json(json!({
                "status": "success",
                "data": { "resultType": "vector", "result": result }
            }))
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("faux VictoriaMetrics");
    });
    format!("http://{address}")
}

async fn create(app: &TestApp, admin: &str, body: Value) -> i64 {
    let reply = app.post("/api/targets", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "création : {}", reply.body);
    reply.body["id"].as_i64().expect("id")
}

fn edges(body: &Value) -> Vec<(String, String, String)> {
    body["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap().to_string(),
                e["to"].as_str().unwrap().to_string(),
                e["source"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[tokio::test]
async fn la_carte_reunit_parents_invites_et_voisins() {
    // Les identifiants sont attribués dans l'ordre : sw = 1, hv = 2, web = 3.
    let victoria = fake_victoria(2, 1).await;
    let app = common::setup_with(move |config| config.victoria_url = Some(victoria)).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;

    let sw = create(
        &app,
        &admin,
        json!({ "name": "sw", "address": "10.0.0.1", "kind": "dummy", "group_name": "Rack" }),
    )
    .await;
    let hv = create(
        &app,
        &admin,
        json!({ "name": "hv", "address": "10.0.0.2", "kind": "dummy", "group_name": "Rack" }),
    )
    .await;
    let web = create(
        &app,
        &admin,
        json!({ "name": "web01", "address": "web01.lan", "kind": "dummy", "parent_id": sw }),
    )
    .await;
    assert_eq!((sw, hv, web), (1, 2, 3));

    let viewer = app.viewer_cookie(&admin).await;
    let reply = app.get("/api/topology", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["metrics_available"], true);

    let mut got = edges(&reply.body);
    got.sort();
    let mut expected = vec![
        ("t1".to_string(), "t3".to_string(), "parent".to_string()),
        ("t2".to_string(), "t3".to_string(), "hypervisor".to_string()),
        ("t2".to_string(), "g2:scratch".to_string(), "hypervisor".to_string()),
        ("t1".to_string(), "t2".to_string(), "lldp".to_string()),
    ];
    expected.sort();
    assert_eq!(got, expected);

    let nodes = reply.body["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 4, "trois équipements et un invité, aucun voisin inventé");
    let guest = nodes.iter().find(|n| n["id"] == "g2:scratch").expect("invité");
    assert_eq!(guest["kind"], "container");
    assert_eq!(guest["running"], false);
    assert_eq!(guest["host_id"], 2);
    assert_eq!(guest["group_name"], "Rack");
    assert!(nodes.iter().all(|n| n.get("address").is_none()), "pas d'adresse sur la carte");
}

#[tokio::test]
async fn sans_mesures_la_carte_garde_les_liens_declares() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let parent =
        create(&app, &admin, json!({ "name": "box", "address": "10.0.0.1", "kind": "dummy" }))
            .await;
    create(
        &app,
        &admin,
        json!({ "name": "nas", "address": "10.0.0.5", "kind": "dummy", "parent_id": parent,
                "tags": { "site": "Cave" } }),
    )
    .await;

    let reply = app.get("/api/topology", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["metrics_available"], false);
    assert_eq!(edges(&reply.body), vec![("t1".into(), "t2".into(), "parent".into())]);
    let nodes = reply.body["nodes"].as_array().expect("nodes");
    assert_eq!(nodes[0]["site"], Value::Null, "sans indice, le site principal");
    assert_eq!(nodes[1]["site"], "Cave", "l'étiquette site place le nœud");
    assert_eq!(nodes[1]["site_source"], "tag");
}

#[tokio::test]
async fn la_carte_demande_une_session() {
    let app = TestApp::configured().await;
    assert_eq!(app.get("/api/topology", None).await.status, StatusCode::UNAUTHORIZED);
}
