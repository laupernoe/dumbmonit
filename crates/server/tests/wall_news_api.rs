//! Actualités du mode mur : réglages validés, vidéo intégrée, titres lus par le
//! serveur (jamais vers une adresse interne), lecture d'essai d'un flux.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

fn settings(feeds: serde_json::Value) -> serde_json::Value {
    json!({
        "ticker": true, "feeds": feeds, "video": false,
        "video_url": "", "video_position": "right", "video_size": "medium"
    })
}

#[tokio::test]
async fn les_reglages_sont_reserves_aux_administrateurs_et_valides() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    // Éteint par défaut.
    let reply = app.get("/api/wall/news/settings", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["ticker"], false);
    assert_eq!(reply.body["video_position"], "right");

    let reply = app.get("/api/wall/news/settings", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let reply = app.put("/api/wall/news/settings", settings(json!([])), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);

    // Les adresses vides sont écartées, les autres nettoyées.
    let reply = app
        .put(
            "/api/wall/news/settings",
            settings(json!([
                { "url": "  https://www.bfmtv.com/rss/news-24-7/  ", "label": "  BFM  " },
                { "url": "   " }
            ])),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["feeds"].as_array().unwrap().len(), 1);
    assert_eq!(reply.body["feeds"][0]["url"], "https://www.bfmtv.com/rss/news-24-7/");
    assert_eq!(reply.body["feeds"][0]["label"], "BFM");
    let reply = app.get("/api/wall/news/settings", Some(&admin)).await;
    assert_eq!(reply.body["feeds"][0]["label"], "BFM");

    let feeds: Vec<_> = (0..9).map(|n| json!({ "url": format!("https://ex{n}.fr/rss") })).collect();
    let too_long = format!("https://ex.fr/{}", "a".repeat(600));
    for (body, expected) in [
        (settings(json!(feeds)), "At most 8 feeds"),
        (settings(json!([{ "url": "ftp://ex.fr/rss" }])), "not an http(s) address"),
        (settings(json!([{ "url": too_long }])), "too long"),
        (
            json!({ "ticker": false, "feeds": [], "video": true, "video_url": "",
                    "video_position": "right", "video_size": "medium" }),
            "Paste the video's link",
        ),
        (
            json!({ "ticker": false, "feeds": [], "video": false,
                    "video_url": "https://example.org/live",
                    "video_position": "right", "video_size": "medium" }),
            "YouTube, Dailymotion or Twitch",
        ),
        (
            json!({ "ticker": false, "feeds": [], "video": false, "video_url": "",
                    "video_position": "bottom", "video_size": "medium" }),
            "Unknown video position",
        ),
        (
            json!({ "ticker": false, "feeds": [], "video": false, "video_url": "",
                    "video_position": "right", "video_size": "huge" }),
            "Unknown video size",
        ),
    ] {
        let reply = app.put("/api/wall/news/settings", body, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{expected}: {}", reply.body);
        assert!(reply.body["error"].as_str().unwrap().contains(expected), "{}", reply.body);
    }
}

#[tokio::test]
async fn le_mur_recoit_la_video_integree_et_les_flux_en_panne_sont_nommes() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    // Rien de réglé : ni bandeau ni vidéo.
    let reply = app.get("/api/wall/news", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["ticker"], false);
    assert!(reply.body["video"].is_null());
    assert!(reply.body["items"].as_array().unwrap().is_empty());

    let body = json!({
        "ticker": true,
        "feeds": [{ "url": "http://127.0.0.1:1/rss", "label": "Interne" }],
        "video": true, "video_url": "https://youtu.be/dQw4w9WgXcQ",
        "video_position": "center", "video_size": "large"
    });
    let reply = app.put("/api/wall/news/settings", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    let reply = app.get("/api/wall/news", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["video"]["position"], "center");
    assert_eq!(reply.body["video"]["size"], "large");
    assert!(
        reply.body["video"]["embed_url"]
            .as_str()
            .unwrap()
            .starts_with("https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"),
        "{}",
        reply.body
    );
    // Le serveur ne va pas lire une adresse interne : le flux est signalé en panne.
    assert!(reply.body["items"].as_array().unwrap().is_empty());
    let problems = reply.body["problems"].as_array().unwrap();
    assert_eq!(problems.len(), 1, "{}", reply.body);
    assert_eq!(problems[0]["feed"], "http://127.0.0.1:1/rss");
    assert!(!problems[0]["message"].as_str().unwrap().is_empty());

    // Le résultat est mis en cache, sans changer ce qui est annoncé.
    let again = app.get("/api/wall/news", Some(&viewer)).await;
    assert_eq!(again.body["problems"], reply.body["problems"]);
}

#[tokio::test]
async fn les_sources_proposees_et_la_lecture_d_essai() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let reply = app.get("/api/wall/news/presets", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let countries = reply.body.as_array().unwrap();
    assert!(countries.iter().any(|c| c["code"] == "FR"));
    assert!(countries.iter().all(|c| c["code"].is_string() && c["feeds"].is_array()));

    // Essai : adresse invalide refusée, adresse interne signalée sans lecture.
    let reply =
        app.post("/api/wall/news/test", json!({ "url": "pas une adresse" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app
        .post("/api/wall/news/test", json!({ "url": "http://127.0.0.1:1/rss" }), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["ok"], false);
    assert_eq!(reply.body["count"], 0);
    assert!(reply.body["error"].is_string());
    let reply = app
        .post("/api/wall/news/test", json!({ "url": "http://127.0.0.1:1/rss" }), Some(&viewer))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}
