//! Prévisions de remplissage d'un équipement (`GET /api/targets/{id}/forecasts`).
//!
//! Un faux VictoriaMetrics sert une semaine d'historique pour un point de
//! montage qui gagne cinq points par jour (85 % aujourd'hui, plein dans trois
//! jours) et ses grandeurs brutes ; toutes les autres familles sont vides. La
//! règle livrée `capacity_forecast` est installée : la page dit ce qu'elle
//! déclencherait.

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

fn answer(result_type: &str, result: Vec<Value>) -> axum::Json<Value> {
    axum::Json(json!({
        "status": "success",
        "data": { "resultType": result_type, "result": result }
    }))
}

async fn fake_victoria(target: i64) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new()
        .route(
            "/api/v1/query_range",
            get(move |Query(params): Query<HashMap<String, String>>| async move {
                let query = params.get("query").cloned().unwrap_or_default();
                // Seule la requête groupée des noms nus porte `__name__=~` ; les
                // familles en expression (SNMP, marge disque…) restent vides.
                if !query.contains("__name__=~") || !query.contains("filesystem_used_percent") {
                    return answer("matrix", Vec::new());
                }
                let start: i64 = params.get("start").and_then(|s| s.parse().ok()).unwrap_or(0);
                let end: i64 = params.get("end").and_then(|s| s.parse().ok()).unwrap_or(0);
                let step: i64 = params.get("step").and_then(|s| s.parse().ok()).unwrap_or(3600);
                let values: Vec<Value> = (0..)
                    .map(|n| start + n * step)
                    .take_while(|t| *t <= end)
                    .map(|t| {
                        let days = (t - start) as f64 / 86_400.0;
                        json!([t, format!("{}", 50.0 + 5.0 * days)])
                    })
                    .collect();
                answer(
                    "matrix",
                    vec![json!({
                        "metric": {
                            "__name__": "dumbmonit_filesystem_used_percent",
                            "target": target.to_string(),
                            "host": "nas",
                            "mountpoint": "/srv"
                        },
                        "values": values
                    })],
                )
            }),
        )
        .route(
            "/api/v1/query",
            get(move |Query(params): Query<HashMap<String, String>>| async move {
                let query = params.get("query").cloned().unwrap_or_default();
                let value = if query.contains("dumbmonit_filesystem_used_bytes") {
                    "850"
                } else if query.contains("dumbmonit_filesystem_total_bytes") {
                    "1000"
                } else {
                    return answer("vector", Vec::new());
                };
                answer(
                    "vector",
                    vec![json!({
                        "metric": { "target": target.to_string(), "host": "nas", "mountpoint": "/srv" },
                        "value": [chrono::Utc::now().timestamp(), value]
                    })],
                )
            }),
        );
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("faux VictoriaMetrics");
    });
    format!("http://{address}")
}

/// Instance configurée, règles livrées installées, un équipement en base.
async fn setup(victoria: Option<String>) -> (TestApp, i64) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let cipher = db::init_cipher(&pool, SECRET).await.expect("chiffrement");
    db::alerts::seed_builtin_rules(&pool).await.expect("règles livrées");
    let id = db::targets::create(
        &pool,
        &cipher,
        &db::targets::TargetInput {
            name: "nas".into(),
            address: "nas.lan".into(),
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
    .expect("cible");
    // Le faux VictoriaMetrics doit connaître l'identifiant : il démarre après la
    // création de la cible, avant le montage du routeur.
    config.victoria_url = match victoria {
        Some(url) => Some(url),
        None => Some(fake_victoria(id).await),
    };
    let app = common::build(dir, config, pool).await;
    app.create_admin().await;
    (app, id)
}

#[tokio::test]
async fn un_point_de_montage_qui_se_remplit_annonce_sa_date() {
    let (app, id) = setup(None).await;
    let admin = app.admin_cookie().await;

    let reply = app.get(&format!("/api/targets/{id}/forecasts"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let list = reply.body.as_array().expect("liste");
    assert_eq!(list.len(), 1, "{}", reply.body);
    let fs = &list[0];
    assert_eq!(fs["family"], "filesystem");
    assert_eq!(fs["metric"], "dumbmonit_filesystem_used_percent");
    assert_eq!(fs["entity"], "/srv");
    assert_eq!(fs["status"], "growing");
    assert_eq!(fs["unit"], "bytes");
    assert_eq!(fs["used"], 850.0);
    assert_eq!(fs["size"], 1000.0);
    assert_eq!(fs["window_days"], 7);
    assert!(fs["labels"].get("__name__").is_none() && fs["labels"].get("family").is_none());

    let used = fs["used_percent"].as_f64().expect("pourcentage");
    assert!((used - 85.0).abs() < 0.5, "used = {used}");
    let growth = fs["growth_per_day"].as_f64().expect("croissance");
    assert!((growth - 5.0).abs() < 0.01, "growth = {growth}");
    let days = fs["days_to_full"].as_f64().expect("échéance");
    assert!((days - 3.0).abs() < 0.1, "days = {days}");
    assert!(fs["confidence"].as_f64().unwrap() > 0.9, "{fs}");
    assert!(fs["full_at"].as_str().is_some_and(|at| !at.ends_with('Z')), "UTC without suffix");
    // Moins de sept jours : la règle livrée serait critique.
    assert_eq!(fs["severity"], "critical");
}

#[tokio::test]
async fn un_equipement_inconnu_est_introuvable() {
    let (app, id) = setup(None).await;
    let admin = app.admin_cookie().await;
    let reply = app.get(&format!("/api/targets/{}/forecasts", id + 1000), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
}

#[tokio::test]
async fn sans_base_de_series_la_page_le_dit() {
    let (app, id) = setup(Some(common::UNREACHABLE_VICTORIA.to_string())).await;
    let admin = app.admin_cookie().await;
    let reply = app.get(&format!("/api/targets/{id}/forecasts"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY, "{}", reply.body);
}

#[tokio::test]
async fn la_regle_de_prevision_s_enregistre_avec_ses_paliers() {
    let (app, _) = setup(None).await;
    let admin = app.admin_cookie().await;

    let reply = app
        .post(
            "/api/alerts/rules",
            json!({
                "name": "Datastores", "kind": "forecast", "query": "pbs_datastore",
                "operator": "<", "threshold": 21.0, "for_secs": 3600, "unit": "d",
                "params": { "forecast_warning_days": 10, "forecast_critical_days": 4 }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["kind"], "forecast");
    assert_eq!(reply.body["params"]["forecast_critical_days"], 4.0);

    // Paliers inversés : refusé, avec une phrase qui dit pourquoi.
    let reply = app
        .post(
            "/api/alerts/rules",
            json!({
                "name": "Inverted", "kind": "forecast", "query": "*",
                "operator": "<", "threshold": 21.0,
                "params": { "forecast_warning_days": 2, "forecast_critical_days": 4 }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    // Les règles livrées de prévision et de dérive sont là.
    let reply = app.get("/api/alerts/rules", Some(&admin)).await;
    let rules = reply.body.as_array().expect("règles");
    let kinds: Vec<(&str, &str)> = rules
        .iter()
        .filter_map(|r| Some((r["uid"].as_str()?, r["kind"].as_str()?)))
        .collect();
    assert!(kinds.contains(&("capacity_forecast", "forecast")), "{kinds:?}");
    assert!(kinds.contains(&("latency_drift", "drift")), "{kinds:?}");
    assert!(kinds.contains(&("http_5xx_drift", "drift")), "{kinds:?}");
}
