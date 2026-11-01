//! Changements de configuration à faire après une mise à jour : réservés à
//! l'administrateur (session), annoncés une fois, fermés pour de bon, et
//! la version précédente suit les démarrages.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use dumbmonit_server::db;
use dumbmonit_server::upgrade_notes::{self, BASELINE, CATALOG};
use serde_json::{Value, json};
use sqlx::SqlitePool;

const CURRENT: &str = env!("CARGO_PKG_VERSION");
const PORT_NOTE: &str = "agent-channel-port";

/// Instance montée sur une base dont le test garde la main, sans compte.
async fn lab() -> (TestApp, SqlitePool) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let app = common::build(dir, config, pool.clone()).await;
    (app, pool)
}

async fn setting(pool: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .unwrap()
}

async fn put_setting(pool: &SqlitePool, key: &str, value: &str) {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .unwrap();
}

/// Instance administrée qui vient de passer de 0.13.0 à la version courante.
async fn upgraded() -> (TestApp, SqlitePool, String) {
    let (app, pool) = lab().await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    put_setting(&pool, "upgrade_notes_seen", "0.13.0").await;
    upgrade_notes::record_version(&pool, CURRENT).await.unwrap();
    (app, pool, admin)
}

fn note<'a>(body: &'a Value, id: &str) -> &'a Value {
    body["notes"].as_array().expect("notes").iter().find(|n| n["id"] == id).expect(id)
}

#[tokio::test]
async fn only_an_admin_session_reaches_the_changes() {
    let (app, _pool, admin) = upgraded().await;
    let viewer = app.viewer_cookie(&admin).await;

    assert_eq!(app.get("/api/upgrade-notes", None).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(app.get("/api/upgrade-notes", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    let close = app.post("/api/upgrade-notes/close", json!({}), Some(&viewer)).await;
    assert_eq!(close.status, StatusCode::FORBIDDEN);
    let announce = app.post("/api/upgrade-notes/announce", json!({}), Some(&viewer)).await;
    assert_eq!(announce.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/upgrade-notes", Some(&admin)).await.status, StatusCode::OK);
}

#[tokio::test]
async fn an_upgrade_lists_the_agent_port_change_and_announces_it_once() {
    let (app, _pool, admin) = upgraded().await;

    let reply = app.get("/api/upgrade-notes", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["current"], CURRENT);
    assert_eq!(reply.body["upgraded_from"], "0.13.0");
    let port = note(&reply.body, PORT_NOTE);
    assert_eq!(port["status"], "pending", "{port}");
    assert_eq!(port["version"], "0.14.0");
    assert!(port["compose"].as_str().unwrap().contains("8443:8443"));
    assert_eq!(port["apply"], "docker compose up -d");
    // Sans en-tête `Host` exploitable, la détection ne conclut pas : la version décide.
    assert_eq!(port["detection"]["state"], "unknown", "{port}");
    assert!(reply.body["pending"].as_u64().unwrap() >= 1);
    assert_eq!(reply.body["announce"], true);
    // À traiter d'abord.
    assert_eq!(reply.body["notes"][0]["status"], "pending");

    // La fenêtre a été montrée : elle ne se rouvre pas d'elle-même, la note reste.
    let done = app.post("/api/upgrade-notes/announce", json!({}), Some(&admin)).await;
    assert_eq!(done.status, StatusCode::NO_CONTENT);
    let again = app.get("/api/upgrade-notes", Some(&admin)).await;
    assert_eq!(again.body["announce"], false);
    assert_eq!(note(&again.body, PORT_NOTE)["status"], "pending");
    assert_eq!(again.body["pending"], reply.body["pending"]);
}

#[tokio::test]
async fn a_closed_change_never_comes_back() {
    let (app, pool, admin) = upgraded().await;

    let unknown = app
        .post("/api/upgrade-notes/close", json!({ "ids": ["no-such-change"] }), Some(&admin))
        .await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST, "{}", unknown.body);

    let closed =
        app.post("/api/upgrade-notes/close", json!({ "ids": [PORT_NOTE] }), Some(&admin)).await;
    assert_eq!(closed.status, StatusCode::OK, "{}", closed.body);
    let port = note(&closed.body, PORT_NOTE);
    assert_eq!(port["status"], "closed");
    assert!(port["closed_at"].is_string());
    assert!(port["detection"].is_null(), "rien à sonder une fois fermée");

    // Fermer deux fois ne journalise qu'une fois.
    let twice =
        app.post("/api/upgrade-notes/close", json!({ "ids": [PORT_NOTE] }), Some(&admin)).await;
    assert_eq!(twice.status, StatusCode::OK);
    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_log WHERE action = 'upgrade_notes.closed' AND subject = ?",
    )
    .bind(PORT_NOTE)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, 1);

    // Un nouveau démarrage, une nouvelle lecture : elle reste fermée.
    upgrade_notes::record_version(&pool, CURRENT).await.unwrap();
    let after = app.get("/api/upgrade-notes", Some(&admin)).await;
    assert_eq!(note(&after.body, PORT_NOTE)["status"], "closed");

    // Fermer sans identifiant ferme tout ce qui reste à traiter.
    let all = app.post("/api/upgrade-notes/close", json!({}), Some(&admin)).await;
    assert_eq!(all.status, StatusCode::OK);
    assert_eq!(all.body["pending"], 0, "{}", all.body);
    assert_eq!(all.body["announce"], false, "plus rien à annoncer");
}

#[tokio::test]
async fn a_new_instance_has_nothing_to_change() {
    let (app, pool) = lab().await;
    // Premier démarrage : aucun compte.
    upgrade_notes::record_version(&pool, CURRENT).await.unwrap();
    assert_eq!(setting(&pool, "upgrade_notes_from").await.as_deref(), Some(CURRENT));
    app.create_admin().await;
    let admin = app.admin_cookie().await;

    let reply = app.get("/api/upgrade-notes", Some(&admin)).await;
    assert_eq!(reply.body["pending"], 0, "{}", reply.body);
    assert_eq!(reply.body["announce"], false);
    assert_eq!(note(&reply.body, PORT_NOTE)["status"], "clear");
}

#[tokio::test]
async fn an_old_instance_without_history_comes_from_the_baseline() {
    let (app, pool) = lab().await;
    app.create_admin().await;
    assert_eq!(setting(&pool, "upgrade_notes_seen").await, None);

    upgrade_notes::record_version(&pool, "0.15.0").await.unwrap();
    assert_eq!(setting(&pool, "upgrade_notes_from").await.as_deref(), Some(BASELINE));
    assert_eq!(setting(&pool, "upgrade_notes_seen").await.as_deref(), Some("0.15.0"));

    // Sans changement de version, rien ne bouge.
    put_setting(&pool, "upgrade_notes_from", "0.14.5").await;
    upgrade_notes::record_version(&pool, "0.15.0").await.unwrap();
    assert_eq!(setting(&pool, "upgrade_notes_from").await.as_deref(), Some("0.14.5"));
}

#[tokio::test]
async fn the_window_stays_open_until_the_changes_are_dealt_with() {
    let (app, pool) = lab().await;
    app.create_admin().await;
    put_setting(&pool, "upgrade_notes_seen", "0.14.2").await;
    put_setting(&pool, "upgrade_notes_from", "0.13.0").await;

    // Deux mises à jour de suite, la note du port jamais traitée : on garde l'origine.
    upgrade_notes::record_version(&pool, "0.14.3").await.unwrap();
    assert_eq!(setting(&pool, "upgrade_notes_from").await.as_deref(), Some("0.13.0"));
    assert_eq!(setting(&pool, "upgrade_notes_seen").await.as_deref(), Some("0.14.3"));

    // Fermée, elle ne retient plus la fenêtre, qui repart de la version quittée.
    let port = CATALOG.iter().find(|n| n.id == PORT_NOTE).unwrap();
    upgrade_notes::close_note(&pool, port, "admin").await.unwrap();
    upgrade_notes::record_version(&pool, "0.14.4").await.unwrap();
    assert_eq!(setting(&pool, "upgrade_notes_from").await.as_deref(), Some("0.14.3"));
}
