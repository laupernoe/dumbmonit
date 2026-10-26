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

fn edge_of<'a>(body: &'a Value, from: &str, to: &str) -> Option<&'a Value> {
    body["edges"].as_array().expect("edges").iter().find(|e| e["from"] == from && e["to"] == to)
}

async fn parent_id(app: &TestApp, admin: &str, id: i64) -> Value {
    let reply = app.get(&format!("/api/targets/{id}"), Some(admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body["parent_id"].clone()
}

#[tokio::test]
async fn un_rattachement_corrige_la_carte_et_le_parent_dalerte() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let fw =
        create(&app, &admin, json!({ "name": "fw", "address": "10.0.0.1", "kind": "dummy" })).await;
    let sw =
        create(&app, &admin, json!({ "name": "sw", "address": "10.0.0.2", "kind": "dummy" })).await;
    let nas = create(
        &app,
        &admin,
        json!({ "name": "nas", "address": "10.0.0.3", "kind": "dummy", "parent_id": fw }),
    )
    .await;

    // Un lecteur ne corrige rien.
    let viewer = app.viewer_cookie(&admin).await;
    let move_nas = json!({ "kind": "parent", "from": format!("t{sw}"), "to": format!("t{nas}") });
    let refused = app.post("/api/topology/corrections", move_nas.clone(), Some(&viewer)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);

    let reply = app.post("/api/topology/corrections", move_nas, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let first = reply.body["correction"]["id"].as_i64().expect("id");
    assert_eq!(reply.body["correction"]["use_for_alerts"], true, "coché par défaut");
    assert_eq!(reply.body["correction"]["author"], "admin");
    assert_eq!(parent_id(&app, &admin, nas).await, sw, "le parent d'alerte suit");

    let map = app.get("/api/topology", Some(&viewer)).await.body;
    let moved = edge_of(&map, &format!("t{sw}"), &format!("t{nas}")).expect("nouveau lien");
    assert_eq!(moved["source"], "parent");
    assert_eq!(moved["origin"], "manual");
    assert_eq!(moved["correction_id"], first);
    assert!(edge_of(&map, &format!("t{fw}"), &format!("t{nas}")).is_none(), "l'ancien parti");
    assert_eq!(map["corrections"].as_array().unwrap().len(), 1);

    // Une boucle est refusée : le commutateur sous le NAS.
    let looped = json!({ "kind": "parent", "from": format!("t{nas}"), "to": format!("t{sw}") });
    let reply = app.post("/api/topology/corrections", looped, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    // Un second rattachement, pour la carte seule, remplace le premier et
    // rend le parent d'avant.
    let map_only = json!({ "kind": "parent", "from": format!("t{fw}"), "to": format!("t{nas}"),
                           "use_for_alerts": false });
    let reply = app.post("/api/topology/corrections", map_only, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["replaced"][0]["id"], first);
    let second = reply.body["correction"]["id"].as_i64().expect("id");
    assert_eq!(parent_id(&app, &admin, nas).await, fw);

    let reply = app.delete(&format!("/api/topology/corrections/{second}"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let again = app.delete(&format!("/api/topology/corrections/{second}"), Some(&admin)).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    let list = app.get("/api/topology/corrections", Some(&admin)).await;
    assert_eq!(list.body, json!([]));
}

#[tokio::test]
async fn un_lien_faux_disparait_et_son_parent_dalerte_aussi() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let up =
        create(&app, &admin, json!({ "name": "up", "address": "10.0.0.1", "kind": "dummy" })).await;
    let down = create(
        &app,
        &admin,
        json!({ "name": "down", "address": "10.0.0.2", "kind": "dummy", "parent_id": up }),
    )
    .await;
    let wrong = json!({ "kind": "forbid", "from": format!("t{up}"), "to": format!("t{down}"),
                        "source": "parent" });
    let reply = app.post("/api/topology/corrections", wrong, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let id = reply.body["correction"]["id"].as_i64().expect("id");
    assert_eq!(parent_id(&app, &admin, down).await, Value::Null);
    let map = app.get("/api/topology", Some(&admin)).await.body;
    assert!(edges(&map).is_empty(), "{map}");

    // Retirée, la correction rend le lien et le parent.
    app.delete(&format!("/api/topology/corrections/{id}"), Some(&admin)).await;
    assert_eq!(parent_id(&app, &admin, down).await, up);

    // Un nœud inconnu, un type de lien inconnu : refusés.
    for body in [
        json!({ "kind": "forbid", "from": "t999", "to": format!("t{down}"), "source": "lldp" }),
        json!({ "kind": "forbid", "from": format!("t{up}"), "to": format!("t{down}"),
                "source": "telepathy" }),
        json!({ "kind": "link", "from": format!("t{up}"), "to": format!("t{up}"),
                "source": "lldp" }),
        json!({ "kind": "site", "to": "nimporte" }),
    ] {
        let reply = app.post("/api/topology/corrections", body.clone(), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{body} : {}", reply.body);
    }
}

#[tokio::test]
async fn deux_rattachements_dun_reseau_deviennent_une_regle_acceptee() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let sw =
        create(&app, &admin, json!({ "name": "sw", "address": "10.0.5.2", "kind": "dummy" })).await;
    let a =
        create(&app, &admin, json!({ "name": "a", "address": "10.0.5.10", "kind": "dummy" })).await;
    let b =
        create(&app, &admin, json!({ "name": "b", "address": "10.0.5.11", "kind": "dummy" })).await;
    for child in [a, b] {
        let body = json!({ "kind": "parent", "from": format!("t{sw}"), "to": format!("t{child}"),
                           "use_for_alerts": false });
        let reply = app.post("/api/topology/corrections", body, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    }
    let found = app.get("/api/topology/suggestions", Some(&admin)).await;
    assert_eq!(found.status, StatusCode::OK, "{}", found.body);
    let suggestion = found.body[0].clone();
    assert_eq!(suggestion["kind"], "parent");
    assert_eq!(suggestion["subnet"], "10.0.5.0/24");
    assert_eq!(suggestion["parent"], format!("t{sw}"));
    assert_eq!(suggestion["evidence"], 2);

    // Un nouvel équipement du réseau, puis la règle acceptée : il suit.
    let c =
        create(&app, &admin, json!({ "name": "c", "address": "10.0.5.12", "kind": "dummy" })).await;
    let mut accept = suggestion.clone();
    accept["status"] = json!("active");
    let reply = app.post("/api/topology/rules", accept, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let rule = reply.body["id"].as_i64().expect("id");

    let map = app.get("/api/topology", Some(&admin)).await.body;
    let learned = edge_of(&map, &format!("t{sw}"), &format!("t{c}")).expect("règle appliquée");
    assert_eq!(learned["origin"], "rule");
    assert_eq!(learned["rule_id"], rule);
    assert_eq!(learned["source"], "manual");
    assert_eq!(map["rules"][0]["matches"], 1);
    assert_eq!(map["suggestions"], json!([]), "acceptée, elle n'est plus proposée");
    let ids: Vec<&str> =
        map["nodes"].as_array().unwrap().iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), 4, "aucun nœud en double : {ids:?}");

    // Retirée, la règle cesse de s'appliquer et peut revenir en suggestion.
    let reply = app.delete(&format!("/api/topology/rules/{rule}"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let map = app.get("/api/topology", Some(&admin)).await.body;
    assert!(edge_of(&map, &format!("t{sw}"), &format!("t{c}")).is_none());
    assert_eq!(map["suggestions"].as_array().unwrap().len(), 1);
}
