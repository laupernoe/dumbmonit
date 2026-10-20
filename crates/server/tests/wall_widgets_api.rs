//! Widgets du mode mur : réglages réservés aux administrateurs et validés, clés
//! d'API qui ne reviennent jamais, sections éteintes par défaut, et disposition
//! des widgets.

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
    assert_eq!(reply.body["layout"]["right"], json!(["weather", "news", "cve"]));
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
async fn la_disposition_est_enregistree_et_rendue_au_mur() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    // Un widget inconnu (l'ancienne vidéo) et un doublon sont écartés en
    // silence ; les widgets absents retrouvent leur zone par défaut.
    let body = json!({ "layout": {
        "top": ["fx", "video", "crypto"],
        "bottom": ["fx"],
        "hidden": ["cve"]
    } });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["layout"]["top"], json!(["fx", "crypto"]));
    assert_eq!(reply.body["layout"]["hidden"], json!(["cve"]));
    assert_eq!(reply.body["layout"]["right"], json!(["weather", "news", "text"]));
    assert_eq!(reply.body["layout"]["bottom"], json!(["stocks", "claude", "openai"]));

    let reply = app.get("/api/wall/widgets", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["layout"]["top"], json!(["fx", "crypto"]));
    assert!(reply.body.get("rotate_seconds").is_none(), "{}", reply.body);
}

#[tokio::test]
async fn la_consommation_d_ia_garde_ses_cles_admin_secretes_et_ses_noms() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let reply = app.get("/api/wall/widgets/settings", Some(&admin)).await;
    assert_eq!(reply.body["ai"]["period"], "week");
    assert_eq!(reply.body["ai"]["by_person"], false);
    assert_eq!(reply.body["ai"]["claude"]["enabled"], false);
    assert_eq!(reply.body["has_anthropic_key"], false);
    assert_eq!(reply.body["has_openai_key"], false);

    // Une clé ordinaire n'est pas une clé admin Anthropic.
    let body = json!({ "secrets": { "anthropic": "sk-ant-api03-ordinary" } });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    assert!(reply.body["error"].as_str().unwrap().contains("sk-ant-admin"));

    let body = json!({
        "ai": {
            "period": "month",
            "by_person": true,
            "claude": {
                "enabled": true,
                "people": [{ "id": " user_01ABC ", "name": " Alice " }]
            },
            "openai": { "enabled": true, "people": [] }
        },
        "secrets": { "anthropic": "sk-ant-admin01-secret", "openai": "sk-admin-secret" }
    });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["has_anthropic_key"], true);
    assert_eq!(reply.body["has_openai_key"], true);
    assert_eq!(
        reply.body["ai"]["claude"]["people"],
        json!([{ "id": "user_01ABC", "name": "Alice" }])
    );
    let text = reply.body.to_string();
    assert!(!text.contains("sk-ant-admin01-secret") && !text.contains("sk-admin-secret"), "{text}");

    // Sans `secrets`, les clés restent ; chaîne vide, effacée.
    let reply = app.put("/api/wall/widgets/settings", json!({}), Some(&admin)).await;
    assert_eq!(reply.body["has_anthropic_key"], true);
    let body = json!({ "secrets": { "openai": "" } });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.body["has_openai_key"], false);
    assert_eq!(reply.body["has_anthropic_key"], true);
    // Les réglages omis reprennent leur défaut : éteint.
    assert_eq!(reply.body["ai"]["claude"]["enabled"], false);

    for (body, expected) in [
        (json!({ "ai": { "period": "year" } }), "period"),
        (json!({ "ai": { "claude": { "people": [{ "id": "x", "name": "" }] } } }), "name"),
    ] {
        let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
        assert!(reply.body["error"].as_str().unwrap().contains(expected), "{}", reply.body);
    }

    // OpenAI allumé sans clé : la carte le dit, sans appeler personne.
    let body = json!({
        "ai": { "openai": { "enabled": true, "people": [] } },
        "secrets": { "openai": "" }
    });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = app.get("/api/wall/widgets", Some(&viewer)).await;
    assert_eq!(reply.body["openai"]["needs_key"], true, "{}", reply.body);
    assert!(reply.body["openai"]["data"].is_null());
    assert_eq!(reply.body["openai"]["source"], "OpenAI");
    assert!(reply.body["claude"].is_null(), "éteint par l'enregistrement précédent");
}

#[tokio::test]
async fn le_message_libre_est_court_et_n_arrive_au_mur_que_s_il_est_allume() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let reply = app.get("/api/wall/widgets", Some(&viewer)).await;
    assert!(reply.body["text"].is_null(), "{}", reply.body);

    let body = json!({ "text": { "enabled": true, "content": "  " } });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let body = json!({ "text": { "enabled": true, "content": "x".repeat(201) } });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    let body = json!({ "text": { "enabled": true, "content": "Stand-up  at\n10:00" } });
    let reply = app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["text"]["content"], "Stand-up at 10:00");
    let reply = app.get("/api/wall/widgets", Some(&viewer)).await;
    assert_eq!(reply.body["text"], "Stand-up at 10:00", "{}", reply.body);

    let body = json!({ "text": { "enabled": false, "content": "Stand-up at 10:00" } });
    app.put("/api/wall/widgets/settings", body, Some(&admin)).await;
    let reply = app.get("/api/wall/widgets", Some(&viewer)).await;
    assert!(reply.body["text"].is_null(), "{}", reply.body);
}
