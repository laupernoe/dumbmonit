//! Tests d'intégration de la note de sécurité : rapport d'un équipement et
//! vue d'ensemble du parc, contre un faux VictoriaMetrics.
//!
//! Le faux répond à toute requête instantanée avec deux machines sous agent :
//! la cible 1 en retard de mises à jour de sécurité, la cible 2 à jour. Il
//! filtre sur `target="…"` quand la requête en porte un, comme le ferait
//! VictoriaMetrics.

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
        json!({ "metric": { "__name__": "dumbmonit_agent_security_updates_pending", "target": "1" }, "value": [1.0, "4"] }),
        json!({ "metric": { "__name__": "dumbmonit_agent_updates_pending", "target": "1" }, "value": [1.0, "9"] }),
        json!({ "metric": { "__name__": "dumbmonit_agent_reboot_required", "target": "1" }, "value": [1.0, "1"] }),
        json!({ "metric": { "__name__": "dumbmonit_agent_security_updates_pending", "target": "2" }, "value": [1.0, "0"] }),
        json!({ "metric": { "__name__": "dumbmonit_agent_updates_pending", "target": "2" }, "value": [1.0, "0"] }),
        json!({ "metric": { "__name__": "dumbmonit_agent_reboot_required", "target": "2" }, "value": [1.0, "0"] }),
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
            assert!(query.contains("dumbmonit_(") && query.contains("agent_reboot_required"));
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
    kind: &str,
) -> i64 {
    db::targets::create(
        pool,
        cipher,
        &db::targets::TargetInput {
            name: name.into(),
            address: format!("{name}.lan"),
            kind: kind.into(),
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

async fn setup() -> (TestApp, Vec<i64>) {
    let victoria = fake_victoria().await;
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = common::base_config(dir.path());
    config.victoria_url = Some(victoria);
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    let cipher = db::init_cipher(&pool, SECRET).await.expect("chiffrement");
    let ids = vec![
        create(&pool, &cipher, "behind", "agent").await,
        create(&pool, &cipher, "patched", "agent").await,
        create(&pool, &cipher, "router", "ping").await,
    ];
    (app, ids)
}

#[tokio::test]
async fn le_rapport_d_un_equipement_note_et_trie_ses_controles() {
    let (app, ids) = setup().await;
    assert_eq!(ids[..2], [1, 2], "le faux suppose ces identifiants");
    let admin = app.admin_cookie().await;

    let reply = app.get(&format!("/api/targets/{}/security", ids[0]), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert_eq!(body["supported"], true);
    assert_eq!(body["kind"], "agent");
    assert_eq!(body["target_name"], "behind");
    // Sécurité (6) + redémarrage (3) + mises à jour (1) en échec, SELinux inconnu.
    assert_eq!(body["score"], 0);
    assert_eq!(body["grade"], "F");
    assert_eq!(body["counts"], json!({ "pass": 0, "fail": 3, "unknown": 1 }));
    let checks = body["checks"].as_array().unwrap();
    assert_eq!(checks[0]["id"], "host.updates.security");
    assert_eq!(checks[0]["result"], "fail");
    assert_eq!(checks[0]["severity"], "high");
    assert_eq!(checks[0]["weight"], 6);
    assert_eq!(checks[0]["evidence"], "4 security updates");
    assert!(checks[0]["reference"].as_str().unwrap().starts_with("https://"));
    assert!(!checks[0]["remediation"].as_str().unwrap().is_empty());
    assert_eq!(checks.last().unwrap()["result"], "unknown");

    let patched = app.get(&format!("/api/targets/{}/security", ids[1]), Some(&admin)).await;
    assert_eq!(patched.body["score"], 100);
    assert_eq!(patched.body["grade"], "A");
}

#[tokio::test]
async fn un_type_sans_controle_repond_non_pris_en_charge() {
    let (app, ids) = setup().await;
    let admin = app.admin_cookie().await;
    let reply = app.get(&format!("/api/targets/{}/security", ids[2]), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["supported"], false);
    assert!(reply.body["score"].is_null());
    assert_eq!(reply.body["checks"], json!([]));

    let missing = app.get("/api/targets/999/security", Some(&admin)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn la_vue_d_ensemble_va_du_pire_au_meilleur() {
    let (app, _ids) = setup().await;
    let admin = app.admin_cookie().await;
    let reply = app.get("/api/security/summary", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let devices = reply.body["devices"].as_array().unwrap();
    // Le « ping » n'a pas de contrôle : il n'apparaît pas.
    assert_eq!(devices.len(), 2, "{devices:?}");
    assert_eq!(devices[0]["target_name"], "behind");
    assert_eq!(devices[0]["grade"], "F");
    assert_eq!(devices[0]["fail"], 3);
    assert_eq!(devices[0]["top_failures"][0], "Security updates installed");
    assert_eq!(devices[1]["target_name"], "patched");
    assert_eq!(devices[1]["score"], 100);
}

#[tokio::test]
async fn la_note_exige_une_session() {
    let (app, ids) = setup().await;
    for uri in [format!("/api/targets/{}/security", ids[0]), "/api/security/summary".to_string()] {
        let reply = app.get(&uri, None).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
}
