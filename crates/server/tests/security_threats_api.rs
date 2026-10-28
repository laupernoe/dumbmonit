//! Tests d'intégration de la carte des menaces (`GET /api/security/threats`),
//! contre un faux VictoriaMetrics : pays seulement, sources réunies, filtre par
//! équipement, et réponse partielle quand VictoriaMetrics se tait.
//!
//! Le faux rend les alertes CrowdSec par pays de la cible 1 et les menaces
//! Cloudflare de la cible 2. Il filtre sur `target="…"` quand la requête en
//! porte un, comme le ferait VictoriaMetrics.

mod common;

use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use dumbmonit_server::db;
use serde_json::{Value, json};

use common::TestApp;

const SECRET: &str = "secret-de-test-suffisamment-long";

fn series() -> Vec<Value> {
    vec![
        json!({ "metric": { "__name__": "dumbmonit_crowdsec_alerts_recent", "target": "1" }, "value": [1.0, "9"] }),
        json!({ "metric": { "__name__": "dumbmonit_crowdsec_alerts_by_country", "target": "1", "country": "CN" }, "value": [1.0, "6"] }),
        json!({ "metric": { "__name__": "dumbmonit_crowdsec_alerts_by_country", "target": "1", "country": "FR" }, "value": [1.0, "2"] }),
        json!({ "metric": { "__name__": "dumbmonit_cloudflare_analytics_readable", "target": "2" }, "value": [1.0, "1"] }),
        json!({ "metric": { "__name__": "dumbmonit_cloudflare_country_threats", "target": "2", "country": "CN" }, "value": [1.0, "40"] }),
        json!({ "metric": { "__name__": "dumbmonit_cloudflare_country_threats", "target": "2", "country": "US" }, "value": [1.0, "3"] }),
    ]
}

async fn fake_victoria() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new().route(
        "/api/v1/query",
        get(|Query(params): Query<std::collections::HashMap<String, String>>| async move {
            let query = params.get("query").cloned().unwrap_or_default();
            assert!(query.starts_with("last_over_time("), "{query}");
            assert!(query.contains("crowdsec_alerts_by_country"), "{query}");
            assert!(query.contains("cloudflare_country_threats"), "{query}");
            let result: Vec<Value> = series()
                .into_iter()
                .filter(|one| {
                    let target = one["metric"]["target"].as_str().unwrap_or_default();
                    !query.contains("target=\"") || query.contains(&format!("target=\"{target}\""))
                })
                .collect();
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

async fn create(
    pool: &sqlx::SqlitePool,
    cipher: &dumbmonit_server::crypto::Cipher,
    name: &str,
) -> i64 {
    db::targets::create(
        pool,
        cipher,
        &db::targets::TargetInput {
            name: name.into(),
            address: format!("{name}.lan"),
            kind: "dummy".into(),
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

/// Instance avec deux équipements (1 : CrowdSec, 2 : Cloudflare).
async fn setup(victoria: Option<String>) -> TestApp {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = common::base_config(dir.path());
    if let Some(victoria) = victoria {
        config.victoria_url = Some(victoria);
    }
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    let cipher = db::init_cipher(&pool, SECRET).await.expect("chiffrement");
    let crowdsec = create(&pool, &cipher, "crowdsec").await;
    let cloudflare = create(&pool, &cipher, "cloudflare").await;
    let ids = [crowdsec, cloudflare];
    assert_eq!(ids, [1, 2], "les séries du faux VictoriaMetrics visent les cibles 1 et 2");
    app
}

#[tokio::test]
async fn les_pays_de_toutes_les_sources_sans_aucune_adresse() {
    let app = setup(Some(fake_victoria().await)).await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    let reply = app.get("/api/security/threats", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert_eq!(body["partial"], false);
    let codes: Vec<&str> =
        body["countries"].as_array().unwrap().iter().map(|c| c["code"].as_str().unwrap()).collect();
    assert_eq!(codes, ["CN", "US", "FR"], "du plus actif au plus calme");
    assert_eq!(
        body["countries"][0],
        json!({ "code": "CN", "total": 46, "crowdsec": 6, "firewall": 0, "cloudflare": 40 })
    );
    let sources = body["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2, "{body}");
    assert_eq!(sources[0]["source"], "crowdsec");
    assert_eq!(sources[0]["targets"], json!([1]));
    assert_eq!(sources[0]["total"], 8);
    assert_eq!(sources[0]["window_secs"], 86_400);
    assert_eq!(sources[1]["source"], "cloudflare");
    assert_eq!(sources[1]["total"], 43);
    // Aucune adresse IP ne sort, sous aucune forme.
    let text = body.to_string();
    assert!(!text.contains("203.0.113") && !text.contains("\"ip\""), "{text}");
}

#[tokio::test]
async fn le_panneau_d_un_equipement_ne_voit_que_ses_sources() {
    let app = setup(Some(fake_victoria().await)).await;
    let admin = app.admin_cookie().await;
    let reply = app.get("/api/security/threats?target=1", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let codes: Vec<&str> = reply.body["countries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["CN", "FR"]);
    assert_eq!(reply.body["countries"][0]["cloudflare"], 0);
    assert_eq!(reply.body["sources"].as_array().unwrap().len(), 1);

    let reply = app.get("/api/security/threats?target=999", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
}

#[tokio::test]
async fn sans_victoriametrics_la_reponse_est_partielle() {
    let app = setup(None).await;
    let admin = app.admin_cookie().await;
    let reply = app.get("/api/security/threats", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["partial"], true);
    assert_eq!(reply.body["countries"], json!([]));
    assert!(reply.body["evaluated_at"].as_str().is_some_and(|t| !t.is_empty()));
}

#[tokio::test]
async fn il_faut_etre_connecte() {
    let app = setup(None).await;
    let reply = app.get("/api/security/threats", None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{}", reply.body);
}
