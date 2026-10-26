//! Stockage : vue des rétentions et de l'espace disque, modifications
//! refusées quand elles ne peuvent pas s'appliquer, recherche dans les
//! journaux quand VictoriaLogs est coupé.

mod common;

use axum::http::StatusCode;
use serde_json::json;

async fn app() -> (common::TestApp, String) {
    let app = common::setup_with(|config| {
        config.vlogs_binary = "/nonexistent/victoria-logs-prod".into();
        config.vm_long_url = None;
    })
    .await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;
    (app, cookie)
}

#[tokio::test]
async fn the_storage_view_describes_an_external_metrics_store() {
    let (app, cookie) = app().await;
    let reply = app.get("/api/system/storage", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = reply.body;
    // Les tests désignent un VictoriaMetrics externe (injoignable).
    assert_eq!(body["metrics"]["embedded"], false);
    assert!(body["metrics"]["retention_days"].as_u64().unwrap() >= 7);
    // Sans instance long terme désignée, pas d'historique possible.
    assert_eq!(body["rollup"]["available"], false);
    assert_eq!(body["rollup"]["step_seconds"], 300);
    assert_eq!(body["logs_store"]["available"], false);
    assert_eq!(body["logs_store"]["enabled"], false);
    assert_eq!(body["logs_store"]["retention_days"], 30);
    let keys: Vec<&str> = body["disk"]["usage"]
        .as_array()
        .unwrap()
        .iter()
        .map(|usage| usage["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["metrics", "rollup", "logs_store", "database", "logs", "backups"]);
    // La base de test existe : elle a une taille.
    let database =
        body["disk"]["usage"].as_array().unwrap().iter().find(|u| u["key"] == "database");
    assert!(database.unwrap()["bytes"].as_u64().unwrap() > 0);
    assert!(body["disk"]["free_bytes"].as_u64().is_some());
    assert_eq!(body["growth"]["bytes_per_day"], serde_json::Value::Null);
    assert_eq!(body["limits"]["min_retention_days"], 7);
    assert_eq!(body["restarted"], json!([]));
}

#[tokio::test]
async fn changes_that_cannot_apply_are_refused_with_a_reason() {
    let (app, cookie) = app().await;
    let current =
        app.get("/api/system/storage", Some(&cookie)).await.body["metrics"]["retention_days"]
            .as_u64()
            .unwrap();

    // VictoriaMetrics externe : sa rétention se règle chez lui.
    let reply = app
        .put(
            "/api/system/storage",
            json!({ "metrics_retention_days": current + 10 }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    assert!(reply.body["error"].as_str().unwrap_or_default().contains("DUMBMONIT_VM_URL"));

    // Hors bornes.
    let reply = app
        .put("/api/system/storage", json!({ "logs_store_retention_days": 0 }), Some(&cookie))
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    // Pas d'instance long terme possible.
    let reply =
        app.put("/api/system/storage", json!({ "rollup_enabled": true }), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    // Binaire VictoriaLogs absent.
    let reply =
        app.put("/api/system/storage", json!({ "logs_store_enabled": true }), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    // Champ inconnu.
    let reply = app.put("/api/system/storage", json!({ "retention": 3 }), Some(&cookie)).await;
    assert!(reply.status.is_client_error(), "{}", reply.status);

    // Une rétention de journaux valide, VictoriaLogs coupé : enregistrée, rien
    // n'est relancé.
    let reply = app
        .put("/api/system/storage", json!({ "logs_store_retention_days": 14 }), Some(&cookie))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["logs_store"]["retention_days"], 14);
    assert_eq!(reply.body["restarted"], json!([]));
}

#[tokio::test]
async fn storage_and_log_search_are_for_administrators_only() {
    let (app, admin) = app().await;
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get("/api/system/storage", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/logs/search", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/system/storage", None).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn log_search_says_when_the_log_store_is_off() {
    let (app, cookie) = app().await;
    let reply = app.get("/api/logs/search?query=error", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    let reply = app.get("/api/logs/search?source=agents", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
}
