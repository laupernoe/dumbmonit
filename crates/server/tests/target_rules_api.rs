//! Tests d'intégration de `GET /api/targets/{id}/rules` : les règles qui
//! surveillent un équipement, d'après les métriques qu'il a écrites.
//!
//! Le faux VictoriaMetrics répond aux séries d'une cible : la cible 1 est un
//! agent avec Docker et une sonde HTTP, la cible 2 n'a encore rien écrit.

mod common;

use std::collections::HashMap;

use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use dumbmonit_server::db;
use serde_json::{Value, json};

use common::TestApp;

const SECRET: &str = "secret-de-test-suffisamment-long";

async fn fake_victoria() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new().route(
        "/api/v1/series",
        get(|Query(params): Query<HashMap<String, String>>| async move {
            let selector = params.get("match[]").cloned().unwrap_or_default();
            assert!(params.contains_key("start"), "fenêtre bornée");
            let series: Vec<Value> = if selector == "{target=\"1\"}" {
                vec![
                    json!({ "__name__": "dumbmonit_up", "target": "1" }),
                    json!({ "__name__": "dumbmonit_container_up", "target": "1", "container": "web" }),
                    json!({ "__name__": "dumbmonit_container_update_available", "target": "1", "container": "web" }),
                    json!({ "__name__": "dumbmonit_probe_success", "target": "1", "probe": "http" }),
                ]
            } else {
                Vec::new()
            };
            axum::Json(json!({ "status": "success", "data": series }))
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("faux VictoriaMetrics");
    });
    format!("http://{address}")
}

async fn create(pool: &sqlx::SqlitePool, name: &str) -> i64 {
    let cipher = db::init_cipher(pool, SECRET).await.expect("chiffrement");
    db::targets::create(
        pool,
        &cipher,
        &db::targets::TargetInput {
            name: name.into(),
            address: format!("{name}.lan"),
            kind: "agent".into(),
            profile_id: None,
            parent_id: None,
            via_agent: None,
            interval: std::time::Duration::from_secs(60),
            enabled: true,
            tags: Default::default(),
            credential: None,
            group_name: String::new(),
        },
    )
    .await
    .expect("cible")
}

async fn setup() -> (TestApp, i64, i64) {
    let victoria = fake_victoria().await;
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = common::base_config(dir.path());
    config.victoria_url = Some(victoria);
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    db::alerts::seed_builtin_rules(&pool).await.expect("règles intégrées");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    let docker = create(&pool, "docker").await;
    let fresh = create(&pool, "fresh").await;
    assert_eq!((docker, fresh), (1, 2), "le faux suppose ces identifiants");
    (app, docker, fresh)
}

fn rule<'a>(body: &'a Value, uid: &str) -> Option<&'a Value> {
    body["rules"].as_array().unwrap().iter().find(|r| r["uid"] == uid)
}

#[tokio::test]
async fn seules_les_regles_qui_lisent_les_metriques_de_l_equipement_sont_listees() {
    let (app, docker, _) = setup().await;
    let admin = app.admin_cookie().await;

    let reply = app.get(&format!("/api/targets/{docker}/rules"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["status"], "ok");
    let update = rule(&reply.body, "container_update_available").expect("règle Docker listée");
    assert_eq!(update["enabled"], true);
    assert_eq!(update["ignored"], false);
    assert_eq!(update["firing"], false);
    assert_eq!(update["severity"], "info");
    assert!(rule(&reply.body, "container_stopped").is_some());
    // Une sonde HTTP, pas de battement de cœur : la règle des services en
    // panne la surveille, celle des battements manqués (`probe="push"`) non.
    assert!(rule(&reply.body, "service_down").is_some());
    assert!(rule(&reply.body, "push_missed").is_none());
}

#[tokio::test]
async fn une_regle_coupee_pour_l_equipement_est_signalee() {
    let (app, docker, fresh) = setup().await;
    let admin = app.admin_cookie().await;
    let listed = app.get(&format!("/api/targets/{docker}/rules"), Some(&admin)).await;
    let id = rule(&listed.body, "container_update_available").unwrap()["id"].as_i64().unwrap();

    let put = app
        .put(
            &format!("/api/alerts/rules/{id}/overrides/{docker}"),
            json!({ "enabled": false }),
            Some(&admin),
        )
        .await;
    assert_eq!(put.status, StatusCode::OK, "{}", put.body);

    let reply = app.get(&format!("/api/targets/{docker}/rules"), Some(&admin)).await;
    let update = rule(&reply.body, "container_update_available").unwrap();
    assert_eq!(update["ignored"], true);
    assert_eq!(update["custom_threshold"], false);
    assert_eq!(rule(&reply.body, "container_stopped").unwrap()["ignored"], false);

    // La surcharge ne vaut que pour cet équipement.
    let other = app.get(&format!("/api/targets/{fresh}/rules"), Some(&admin)).await;
    assert_eq!(other.body["status"], "no_data");
    assert!(other.body["rules"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn un_equipement_inconnu_est_introuvable() {
    let (app, _, _) = setup().await;
    let admin = app.admin_cookie().await;
    let reply = app.get("/api/targets/999/rules", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sans_victoriametrics_la_liste_est_indisponible() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = app
        .request(
            "POST",
            "/api/targets",
            Some(json!({ "name": "box", "address": "box.lan", "kind": "dummy" })),
            Some(&admin),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_i64().unwrap();
    let reply = app.get(&format!("/api/targets/{id}/rules"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["status"], "unavailable");
}
