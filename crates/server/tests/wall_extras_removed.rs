//! La musique, les actualités et les widgets du mur ont disparu avec
//! l'interface Homelab. Une instance qui les avait réglés garde ses lignes en
//! base (réglages `wall.*`, tables `music_*`) : rien ne les lit plus, rien ne
//! casse, leurs routes répondent 404, et une sauvegarde — y compris un lot qui
//! porterait des sections inconnues — s'exporte et se restaure.

mod common;

use axum::http::StatusCode;
use dumbmonit_server::backup::{Bundle, ExportOptions, collect, restore};
use dumbmonit_server::db;
use dumbmonit_server::instance::{self, InstanceSettings};

/// Ce qu'une 0.14 laissait derrière elle quand le mur était réglé.
async fn leave_old_wall_settings(pool: &sqlx::SqlitePool) {
    for (key, value) in [
        ("wall.widgets", r#"{"coins":{"enabled":true,"ids":["bitcoin"]}}"#),
        ("wall.news", r#"{"enabled":true,"feeds":["https://example.org/rss"]}"#),
    ] {
        sqlx::query("INSERT INTO settings (key, value) VALUES (?, CAST(? AS BLOB))")
            .bind(key)
            .bind(value)
            .execute(pool)
            .await
            .expect("ancien réglage");
    }
    sqlx::query("INSERT INTO music_wall_link (id, link, set_by) VALUES (1, ?, 'admin')")
        .bind("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M")
        .execute(pool)
        .await
        .expect("ancien lien");
}

#[tokio::test]
async fn les_anciens_reglages_du_mur_ne_genent_rien_et_leurs_routes_ont_disparu() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    leave_old_wall_settings(&pool).await;
    let app = common::build(dir, config, pool).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;

    for uri in [
        "/api/wall/widgets",
        "/api/wall/widgets/settings",
        "/api/wall/news",
        "/api/wall/news/settings",
        "/api/music/now",
        "/api/music/spotify",
    ] {
        let reply = app.get(uri, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{uri} : {}", reply.body);
    }
    // Le reste de l'instance se charge comme avant.
    let reply = app.get("/api/instance/settings", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    // Le géocodage des lieux, qui vivait avec la météo du mur, reste en place.
    let reply = app.get("/api/locations/geocode?q=a", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
}

#[tokio::test]
async fn une_sauvegarde_avec_d_anciennes_sections_du_mur_se_restaure() {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    leave_old_wall_settings(&pool).await;
    let chosen = InstanceSettings { name: "Lab".into(), ..InstanceSettings::default() };
    instance::save(&pool, &chosen).await.unwrap();

    // L'export ignore les lignes que plus rien ne lit.
    let lot = collect(&pool, &cipher, ExportOptions::default()).await.expect("export");

    // Un lot qui porterait des sections du mur (`music`, `wall_widgets`,
    // `wall_news`) se lit quand même : les champs inconnus sont ignorés.
    let mut json = serde_json::to_value(&lot).expect("lot en JSON");
    json["music"] = serde_json::json!({ "link": "https://open.spotify.com/album/x" });
    json["wall_widgets"] = serde_json::json!({ "coins": { "enabled": true } });
    json["wall_news"] = serde_json::json!({ "enabled": true });
    let old: Bundle = serde_json::from_value(json).expect("lot avec sections inconnues");

    let target_dir = tempfile::tempdir().unwrap();
    let target = db::open(&target_dir.path().join("dumbmonit.db")).await.expect("base cible");
    let target_cipher =
        db::init_cipher(&target, "secret-de-test-suffisamment-long").await.expect("clé");
    restore(&target, &target_cipher, &old, true).await.expect("restauration");
    assert_eq!(instance::load(&target).await.name, "Lab");
}
