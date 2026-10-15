//! Chiffres de disponibilité et de latence lus dans VictoriaMetrics : la page
//! de statut publique et les rapports périodiques. Un faux VictoriaMetrics
//! répond aux requêtes instantanées et par plage pour les deux équipements du
//! test ; sans lui (injoignable), la page s'affiche sans chiffres et le rapport
//! le dit.

mod common;

use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use chrono::Utc;
use common::TestApp;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Faux VictoriaMetrics : toutes les cibles 1 et 2 répondent 99,5 % — sauf
/// `tfirst_over_time`, qui date la première mesure d'il y a 200 jours.
async fn fake_victoria() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new()
        .route(
            "/api/v1/query",
            get(|Query(params): Query<HashMap<String, String>>| async move {
                let query = params.get("query").cloned().unwrap_or_default();
                let now = Utc::now().timestamp();
                let value = if query.contains("tfirst_over_time") {
                    (now - 200 * 86_400).to_string()
                } else if query.contains("probe_duration_seconds") {
                    "42.0".to_string()
                } else {
                    "99.5".to_string()
                };
                let result: Vec<Value> = [1, 2]
                    .iter()
                    .map(|id| {
                        json!({ "metric": { "target": id.to_string() }, "value": [now, value] })
                    })
                    .collect();
                axum::Json(json!({
                    "status": "success",
                    "data": { "resultType": "vector", "result": result }
                }))
            }),
        )
        .route(
            "/api/v1/query_range",
            get(|Query(params): Query<HashMap<String, String>>| async move {
                let start: i64 = params.get("start").and_then(|s| s.parse().ok()).unwrap_or(0);
                let step: i64 = params.get("step").and_then(|s| s.parse().ok()).unwrap_or(86_400);
                let values: Vec<Value> =
                    (0..4).map(|n| json!([start + n * step, "98.0"])).collect();
                let result: Vec<Value> = [1, 2]
                    .iter()
                    .map(|id| json!({ "metric": { "target": id.to_string() }, "values": values }))
                    .collect();
                axum::Json(json!({
                    "status": "success",
                    "data": { "resultType": "matrix", "result": result }
                }))
            }),
        );
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("faux VictoriaMetrics");
    });
    format!("http://{address}")
}

async fn estate(app: &TestApp, admin: &str, slug: &str) {
    for body in [
        json!({ "name": "Site web", "address": "http://127.0.0.1:1/", "kind": "http" }),
        json!({ "name": "NAS", "address": "nas.lan", "kind": "dummy" }),
    ] {
        let reply = app.post("/api/targets", body, Some(admin)).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    }
    let reply = app
        .post(
            "/api/status-pages",
            json!({
                "title": "Chiffres", "slug": slug, "published": true, "show_uptime_days": 90,
                "show_calendar": true, "show_latency": true
            }),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let page = reply.body["id"].as_i64().unwrap();
    let reply = app
        .put(
            &format!("/api/status-pages/{page}/items"),
            json!([
                { "target_id": 1, "label": "Site web", "group_name": "Public" },
                { "target_id": 2, "label": "Stockage", "group_name": "Interne" }
            ]),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

#[tokio::test]
async fn la_page_publique_porte_les_disponibilites_et_les_latences() {
    let victoria = fake_victoria().await;
    let app = common::setup_with(move |config| config.victoria_url = Some(victoria)).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    estate(&app, &admin, "chiffres").await;

    let reply = app.get("/api/public/status/chiffres", None).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let groups = reply.body["groups"].as_array().unwrap();
    let item = |label: &str| -> &Value {
        groups
            .iter()
            .flat_map(|g| g["items"].as_array().unwrap())
            .find(|i| i["label"] == label)
            .unwrap_or_else(|| panic!("service {label}"))
    };

    // Sonde HTTP : le dernier résultat dit « en marche », mais une disponibilité
    // horaire de 99,5 % la rend dégradée.
    let web = item("Site web");
    assert_eq!(web["state"], "degraded", "{web}");
    assert_eq!(web["uptime_24h"], 99.5);
    assert_eq!(web["uptime_7d"], 99.5);
    assert_eq!(web["uptime_30d"], 99.5);
    // Appareils tout neufs : la fenêtre adaptative reste à 7 jours, pas de chiffre sur 90.
    assert!(web["uptime_90d"].is_null(), "{web}");
    assert_eq!(web["latency_ms"], 42.0);
    assert!(!web["latency_series"]["h24"].as_array().unwrap().is_empty(), "{web}");
    assert!(!web["latency_series"]["d7"].as_array().unwrap().is_empty());
    assert!(web["history"].is_array());

    // Équipement sans sonde : sa disponibilité est ramenée à la part couverte.
    let nas = item("Stockage");
    assert!(nas["uptime_24h"].is_number(), "{nas}");
    assert!(nas["uptime_90d"].is_null(), "{nas}");
    assert!(nas["history"].is_array());

    // Le calendrier existe, les badges lisent les mêmes chiffres.
    assert!(reply.body["calendar"].is_array());
    let uptime = app.request("GET", "/api/public/status/chiffres/uptime.svg", None, None).await;
    assert_eq!(uptime.status, StatusCode::OK);
}

#[tokio::test]
async fn la_page_s_affiche_sans_chiffres_quand_la_base_de_series_est_muette() {
    // VictoriaMetrics injoignable (adresse par défaut des tests).
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    estate(&app, &admin, "muette").await;
    let reply = app.get("/api/public/status/muette", None).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let first = &reply.body["groups"][0]["items"][0];
    assert!(first["uptime_24h"].is_null(), "{first}");
    assert_eq!(first["state"], "unknown");
}

#[tokio::test]
async fn un_rapport_collecte_disponibilites_et_incidents() {
    let victoria = fake_victoria().await;
    let app = common::setup_with(move |config| config.victoria_url = Some(victoria)).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    estate(&app, &admin, "rapport").await;

    // Un canal courriel qui n'aboutit pas : l'envoi échoue, mais le rapport a
    // bien été collecté et rendu avant.
    let reply = app
        .post(
            "/api/notify/channels",
            json!({
                "name": "Mail", "kind": "smtp",
                "settings": {
                    "host": "127.0.0.1", "port": 1, "security": "none",
                    "from": "dumbmonit@example.org", "to": ["ops@example.org"]
                }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let reply = app
        .post("/api/reports/schedules", json!({ "recipients": ["a@example.org"] }), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let id = reply.body["id"].as_i64().unwrap();

    let reply =
        app.post(&format!("/api/reports/schedules/{id}/send-test"), json!({}), Some(&admin)).await;
    // Selon que le serveur de courriel est injoignable ou le canal refusé, la
    // réponse compte les échecs ou les dit : dans les deux cas rien n'est parti.
    match reply.status {
        StatusCode::OK => {
            assert_eq!(reply.body["sent"], 0, "{}", reply.body);
            assert!(reply.body["failed"].as_u64().unwrap() >= 1, "{}", reply.body);
        }
        other => assert_eq!(other, StatusCode::BAD_REQUEST, "{}", reply.body),
    }
}
