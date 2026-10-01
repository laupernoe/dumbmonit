//! Paquets d'intégration par l'API : installer, lister, activer, désinstaller.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const SHELLY: &str = include_str!("../../../packs/shelly-plug/pack.yaml");
const KIND: &str = "pack.shelly-plug";
const RULE: &str = "pack:shelly-plug:overheating";

async fn install(app: &common::TestApp, yaml: &str, cookie: &str) -> common::Reply {
    app.post("/api/packs", json!({ "yaml": yaml }), Some(cookie)).await
}

async fn rule(app: &common::TestApp, cookie: &str, uid: &str) -> Option<Value> {
    let rules = app.get("/api/alerts/rules", Some(cookie)).await.body;
    rules.as_array().expect("liste des règles").iter().find(|r| r["uid"] == uid).cloned()
}

async fn kinds(app: &common::TestApp, cookie: &str) -> Vec<String> {
    let collectors = app.get("/api/collectors", Some(cookie)).await.body;
    collectors
        .as_array()
        .expect("catalogue")
        .iter()
        .map(|kind| kind["kind"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[tokio::test]
async fn un_paquet_sinstalle_sert_de_type_puis_se_desinstalle() {
    let app = common::setup().await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;

    let reply = install(&app, SHELLY, &cookie).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["outcome"], "created");
    assert_eq!(reply.body["rules_added"], 1);
    assert_eq!(reply.body["restart_required"], false);
    assert_eq!(reply.body["pack"]["kind"], KIND);
    assert_eq!(reply.body["pack"]["version"], "1.0.0");
    assert!(
        reply.body["pack"]["metrics"]
            .as_array()
            .unwrap()
            .contains(&json!("dumbmonit_shelly_plug_power_watts"))
    );

    let list = app.get("/api/packs", Some(&cookie)).await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(list.body.as_array().unwrap().len(), 1);
    assert_eq!(list.body[0]["enabled"], true);
    assert_eq!(list.body[0]["targets"], 0);

    // Le type apparaît dans le catalogue, décrit par le manifeste.
    let catalog = app.get("/api/collectors", Some(&cookie)).await.body;
    let shelly = catalog.as_array().unwrap().iter().find(|k| k["kind"] == KIND).expect("type");
    assert_eq!(shelly["label"], "Shelly plug (Gen2+)");
    assert_eq!(shelly["default_port"], 80);
    assert!(shelly["options"].as_array().unwrap().iter().any(|o| o["key"] == "switch_id"));

    // La règle livrée est installée, seuil tiré de la valeur par défaut.
    let installed = rule(&app, &cookie, RULE).await.expect("règle du paquet");
    assert_eq!(installed["threshold"], 70.0);
    assert_eq!(installed["builtin"], false);
    assert_eq!(installed["for_secs"], 300);

    // Un équipement de ce type : son option devient son seuil.
    let target = app
        .post(
            "/api/targets",
            json!({"name": "Desk plug", "address": "192.168.1.50", "kind": KIND,
                   "tags": {"max_temperature": "55"}}),
            Some(&cookie),
        )
        .await;
    assert_eq!(target.status, StatusCode::CREATED, "{}", target.body);
    let target_id = target.body["id"].as_i64().unwrap();
    let overrides = rule(&app, &cookie, RULE).await.unwrap()["overrides"].clone();
    assert_eq!(overrides[0]["target_id"], target_id);
    assert_eq!(overrides[0]["threshold"], 55.0);

    // Un relais ne sait pas interroger un paquet.
    let relayed = app
        .post(
            "/api/targets",
            json!({"name": "x", "address": "192.168.1.51", "kind": KIND, "via_agent": target_id}),
            Some(&cookie),
        )
        .await;
    assert_eq!(relayed.status, StatusCode::BAD_REQUEST);

    // Tant qu'un équipement l'utilise, le paquet reste.
    let refused = app.delete("/api/packs/shelly-plug", Some(&cookie)).await;
    assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);

    // Désactivé, le type disparaît du catalogue ; réactivé, il revient.
    let disabled = app.put("/api/packs/shelly-plug/disable", json!({}), Some(&cookie)).await;
    assert_eq!(disabled.status, StatusCode::OK, "{}", disabled.body);
    assert_eq!(disabled.body["enabled"], false);
    assert!(!kinds(&app, &cookie).await.contains(&KIND.to_string()));
    let enabled = app.put("/api/packs/shelly-plug/enable", json!({}), Some(&cookie)).await;
    assert_eq!(enabled.body["enabled"], true);
    assert!(kinds(&app, &cookie).await.contains(&KIND.to_string()));

    let deleted = app.delete(&format!("/api/targets/{target_id}"), Some(&cookie)).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    let removed = app.delete("/api/packs/shelly-plug", Some(&cookie)).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT, "{}", removed.body);
    assert!(app.get("/api/packs", Some(&cookie)).await.body.as_array().unwrap().is_empty());
    assert!(rule(&app, &cookie, RULE).await.is_none(), "les règles partent avec le paquet");
    assert!(!kinds(&app, &cookie).await.contains(&KIND.to_string()));
    let missing = app.get("/api/packs/shelly-plug", Some(&cookie)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn une_mise_a_jour_ne_reecrit_jamais_une_regle_modifiee() {
    let app = common::setup().await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;
    assert_eq!(install(&app, SHELLY, &cookie).await.status, StatusCode::CREATED);

    let id = rule(&app, &cookie, RULE).await.unwrap()["id"].as_i64().unwrap();
    let off = app
        .post(&format!("/api/alerts/rules/{id}/enable"), json!({"enabled": false}), Some(&cookie))
        .await;
    assert_eq!(off.status, StatusCode::OK, "{}", off.body);

    let same = install(&app, SHELLY, &cookie).await;
    assert_eq!(same.status, StatusCode::OK);
    assert_eq!(same.body["outcome"], "unchanged");

    let newer = SHELLY.replace("version: 1.0.0", "version: 1.1.0").replace("for: 5m", "for: 10m");
    let updated = install(&app, &newer, &cookie).await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
    assert_eq!(updated.body["outcome"], "updated");
    assert_eq!(updated.body["rules_added"], 0);
    assert_eq!(updated.body["pack"]["version"], "1.1.0");
    let kept = rule(&app, &cookie, RULE).await.unwrap();
    assert_eq!(kept["enabled"], false, "le réglage de l'utilisateur est conservé");
    assert_eq!(kept["for_secs"], 300);
}

#[tokio::test]
async fn un_paquet_invalide_est_refuse_avec_toutes_ses_erreurs() {
    let app = common::setup().await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;

    let broken = SHELLY
        .replace("id: shelly-plug", "id: Shelly")
        .replace("path: /rpc", "path: http://evil.lan/rpc");
    let reply = install(&app, &broken, &cookie).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let message = reply.body.to_string();
    assert!(message.contains("id:"), "{message}");
    assert!(message.contains("sources[status].path"), "{message}");
    assert!(app.get("/api/packs", Some(&cookie)).await.body.as_array().unwrap().is_empty());

    let reply = install(&app, "not: [valid", &cookie).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn le_yaml_brut_est_accepte_et_lanti_csrf_sapplique() {
    let app = common::setup().await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;

    let post = |with_csrf: bool| {
        let mut builder = Request::post("/api/packs")
            .header(header::COOKIE, &cookie)
            .header(header::CONTENT_TYPE, "application/yaml");
        if with_csrf {
            builder = builder.header("x-requested-with", "DumbMonit");
        }
        builder.body(Body::from(SHELLY)).unwrap()
    };

    let response = app.router.clone().oneshot(post(false)).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN, "écriture sans en-tête anti-CSRF");

    let response = app.router.clone().oneshot(post(true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["pack"]["id"], "shelly-plug");
}

#[tokio::test]
async fn un_lecteur_ne_gere_pas_les_paquets() {
    let app = common::setup().await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    assert_eq!(app.get("/api/packs", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(install(&app, SHELLY, &viewer).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/packs", None).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn les_paquets_figurent_dans_la_sauvegarde() {
    let app = common::setup().await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;
    assert_eq!(install(&app, SHELLY, &cookie).await.status, StatusCode::CREATED);

    let status = app.get("/api/backup", Some(&cookie)).await.body;
    let section = status["contents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["section"] == "packs")
        .expect("section packs")
        .clone();
    assert_eq!(section["count"], 1);
}

#[tokio::test]
async fn un_paquet_restaure_redevient_un_type_sans_redemarrage() {
    const PHRASE: &str = "une phrase de passe assez longue";
    let source = common::setup().await;
    source.create_admin().await;
    let cookie = source.admin_cookie().await;
    assert_eq!(install(&source, SHELLY, &cookie).await.status, StatusCode::CREATED);
    let target = source
        .post(
            "/api/targets",
            json!({"name": "Desk plug", "address": "192.168.1.50", "kind": KIND}),
            Some(&cookie),
        )
        .await;
    assert_eq!(target.status, StatusCode::CREATED, "{}", target.body);
    let bundle = source.post("/api/backup", json!({ "passphrase": PHRASE }), Some(&cookie)).await;
    assert_eq!(bundle.status, StatusCode::OK, "{}", bundle.body);
    assert_eq!(bundle.body["summary"]["packs"], 1);

    let fresh = common::setup().await;
    fresh.create_admin().await;
    let cookie = fresh.admin_cookie().await;
    assert!(!kinds(&fresh, &cookie).await.contains(&KIND.to_string()));
    let restored = fresh
        .post(
            "/api/backup/restore",
            json!({ "bundle": bundle.body, "passphrase": PHRASE, "apply": true }),
            Some(&cookie),
        )
        .await;
    assert_eq!(restored.status, StatusCode::OK, "{}", restored.body);
    let packs = restored.body["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["section"] == "packs")
        .expect("section packs")
        .clone();
    assert_eq!(packs["created"], 1, "{packs}");

    assert!(kinds(&fresh, &cookie).await.contains(&KIND.to_string()));
    let list = fresh.get("/api/packs", Some(&cookie)).await.body;
    assert_eq!(list[0]["targets"], 1);
    assert!(rule(&fresh, &cookie, RULE).await.is_some(), "la règle suit la section rules");
}
