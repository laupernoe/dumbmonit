//! Widgets du mode mur : réglages réservés aux administrateurs et validés, clés
//! d'API qui ne reviennent jamais, sections éteintes par défaut, et relais
//! d'images des actualités fermé à toute adresse non publiée.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn les_reglages_sont_valides_et_les_cles_ne_reviennent_jamais() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    // Éteint par défaut, sans clé.
    let reply = app.get("/api/wall/widgets/settings", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["crypto"]["enabled"], false);
    assert_eq!(reply.body["rotate_seconds"], 20);
    assert_eq!(reply.body["has_finnhub_key"], false);

    let reply = app.get("/api/wall/widgets/settings", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let reply = app.put("/api/wall/widgets/settings", json!({}), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);

    // Une clé enregistrée : seule sa présence revient.
    let body = json!({
        "stocks": { "enabled": false, "symbols": ["aapl", "MSFT"] },
        "fx": { "enabled": false, "base": "chf", "currencies": ["usd", "eur"] },
        "secrets": { "finnhub": "  d1secretkey  " }
    });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["has_finnhub_key"], true);
    assert_eq!(reply.body["stocks"]["symbols"], json!(["AAPL", "MSFT"]));
    assert_eq!(reply.body["fx"]["base"], "CHF");
    assert!(!reply.body.to_string().contains("d1secretkey"), "{}", reply.body);

    // Sans `secrets`, la clé reste ; une chaîne vide l'efface.
    let reply = app.put("/api/wall/widgets/settings", json!({}), Some(&admin)).await;
    assert_eq!(reply.body["has_finnhub_key"], true);
    let reply = app
        .put("/api/wall/widgets/settings", json!({ "secrets": { "finnhub": "" } }), Some(&admin))
        .await;
    assert_eq!(reply.body["has_finnhub_key"], false);

    for (body, expected) in [
        (json!({ "rotate_seconds": 1 }), "seconds"),
        (json!({ "crypto": { "enabled": true, "coins": [] } }), "at least one coin"),
        (json!({ "crypto": { "coins": ["bit coin"] } }), "CoinGecko"),
        (json!({ "weather": { "enabled": true, "places": [] } }), "at least one place"),
        (json!({ "stocks": { "symbols": ["A&B"] } }), "ticker"),
        (json!({ "fx": { "base": "XYZ" } }), "base currency"),
        (json!({ "cve": { "source": "twitter" } }), "source"),
    ] {
        let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{expected}: {}", reply.body);
        assert!(reply.body["error"].as_str().unwrap().contains(expected), "{}", reply.body);
    }
}

#[tokio::test]
async fn le_mur_ne_recoit_que_les_widgets_allumes() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let reply = app.get("/api/wall/widgets", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    for widget in ["crypto", "weather", "stocks", "fx", "cve"] {
        assert!(reply.body[widget].is_null(), "{widget}: {}", reply.body);
    }

    // Les actions sans clé : le widget le dit, sans appeler personne.
    let body = json!({ "stocks": { "enabled": true, "symbols": ["AAPL"] } });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = app.get("/api/wall/widgets", Some(&viewer)).await;
    assert_eq!(reply.body["stocks"]["needs_key"], true, "{}", reply.body);
    assert!(reply.body["stocks"]["data"].is_null());
    assert_eq!(reply.body["stocks"]["source"], "Finnhub");

    let reply = app.get("/api/wall/widgets/catalog", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body["coins"].as_array().unwrap().iter().any(|c| c["symbol"] == "BTC"));
    assert!(reply.body["fx_currencies"].as_array().unwrap().iter().any(|c| c == "JPY"));

    let reply = app.get("/api/wall/widgets/places?q=a", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app.get("/api/wall/widgets/places?q=Lausanne", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn le_relais_d_images_ne_sert_que_les_images_publiees() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let reply = app.get("/api/wall/news/image/not-an-id", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let unknown = "0123456789abcdef0123456789abcdef";
    let reply = app.get(&format!("/api/wall/news/image/{unknown}"), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = app.get(&format!("/api/wall/news/image/{unknown}"), None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);

    // Le carrousel s'allume avec les actualités, éteint par défaut.
    let reply = app.get("/api/wall/news/settings", Some(&admin)).await;
    assert_eq!(reply.body["gallery"], false);
    let reply = app.get("/api/wall/news", Some(&viewer)).await;
    assert_eq!(reply.body["gallery"], false);
}
