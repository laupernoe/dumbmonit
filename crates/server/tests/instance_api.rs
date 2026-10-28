//! Réglages d'instance et assistant de démarrage : lecture publique minimale,
//! écriture réservée aux administrateurs, valeurs refusées, une instance neuve
//! qui attend l'assistant, et l'ancien `product_mode` ignoré sans erreur.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn une_instance_neuve_attend_l_assistant() {
    let app = TestApp::configured().await;

    // Sans session : seulement l'état de l'assistant.
    let reply = app.get("/api/instance", None).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["setup_completed"], false);
    assert!(reply.body.get("product_mode").is_none(), "plus de mode : {}", reply.body);
    assert!(reply.body.get("name").is_none(), "le nom ne sort pas sans session : {}", reply.body);

    // Le détail demande une session.
    let reply = app.get("/api/instance/settings", None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn l_administrateur_nomme_l_instance_et_termine_l_assistant() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;

    let reply = app
        .put(
            "/api/instance/settings",
            json!({
                "name": "  Acme NOC  ",
                "language": "fr",
                "timezone": "Europe/Paris"
            }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["name"], "Acme NOC");
    assert_eq!(reply.body["setup_completed"], false);
    assert!(reply.body.get("product_mode").is_none(), "{}", reply.body);

    // Un champ absent garde sa valeur.
    let reply =
        app.put("/api/instance/settings", json!({ "setup_completed": true }), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["timezone"], "Europe/Paris");
    assert!(reply.body["setup_completed_at"].is_string());

    let reply = app.get("/api/instance", None).await;
    assert_eq!(reply.body["setup_completed"], true);

    let reply = app.get("/api/instance/settings", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["language"], "fr");
}

#[tokio::test]
async fn un_ancien_client_qui_envoie_le_mode_n_est_pas_refuse() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;

    // Un onglet resté ouvert sur une ancienne version : le champ est ignoré.
    let reply = app
        .put(
            "/api/instance/settings",
            json!({ "product_mode": "homelab", "name": "Lab" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["name"], "Lab");
    assert!(reply.body.get("product_mode").is_none(), "{}", reply.body);
}

#[tokio::test]
async fn les_valeurs_inconnues_sont_refusees_et_rien_n_est_ecrit() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;

    for body in [
        json!({ "language": "klingon" }),
        json!({ "timezone": "Mars/Olympus" }),
        json!({ "name": "x".repeat(81) }),
    ] {
        let reply = app.put("/api/instance/settings", body.clone(), Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{body} : {}", reply.body);
    }
    let reply = app.get("/api/instance/settings", Some(&cookie)).await;
    assert_eq!(reply.body["name"], "");
    assert_eq!(reply.body["language"], "");
}

#[tokio::test]
async fn un_lecteur_ne_change_pas_les_reglages() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let reply = app.get("/api/instance/settings", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let reply = app.put("/api/instance/settings", json!({ "name": "Pirate" }), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
}
