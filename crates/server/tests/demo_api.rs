//! Mode démonstration : la connexion `demo` / `demo` fonctionne, les lectures
//! aussi, et toute écriture est refusée par le garde unique — quel que soit
//! l'appelant, session ouverte ou non.

mod common;

use axum::http::StatusCode;
use dumbmonit_server::{db, demo, notify, tsdb};
use serde_json::{Value, json};

use common::{TestApp, UNREACHABLE_VICTORIA};

/// Instance en mode démonstration, parc synthétique semé (sans les équipements
/// simulés, qui ont leur propre test : `demo_estate.rs`).
async fn demo_app() -> TestApp {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = common::base_config(dir.path());
    config.demo = true;
    let pool = db::open(&config.database_path()).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    let victoria = tsdb::Victoria::new(UNREACHABLE_VICTORIA).expect("client");
    let sink = tsdb::spawn_writer(victoria, std::time::Duration::from_secs(60));
    demo::seed::run(&pool, &cipher, &sink, &[]).await.expect("parc de démonstration");
    common::build(dir, config, pool).await
}

async fn demo_cookie(app: &TestApp) -> String {
    let reply = app.login_as(demo::DEMO_USERNAME, demo::DEMO_PASSWORD).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{:?}", reply.body);
    reply.cookie()
}

fn assert_refused(reply: &common::Reply, what: &str) {
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{what} : {:?}", reply.body);
    assert_eq!(reply.body["demo"], Value::Bool(true), "{what}");
    assert_eq!(reply.body["error"], demo::READ_ONLY_MESSAGE, "{what}");
}

#[tokio::test]
async fn le_statut_annonce_la_demonstration_sans_ouvrir_de_session() {
    let app = demo_app().await;
    let status = app.get("/api/auth/status", None).await;
    assert_eq!(status.body["demo"], true);
    assert_eq!(status.body["authenticated"], false, "l'écran de connexion reste affiché");
    let health = app.get("/api/health", None).await;
    assert_eq!(health.body["demo"], true);
}

#[tokio::test]
async fn le_compte_demo_se_connecte_et_lit_le_parc() {
    let app = demo_app().await;
    let cookie = demo_cookie(&app).await;

    let me = app.get("/api/auth/me", Some(&cookie)).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["username"], "demo");

    let targets = app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(targets.status, StatusCode::OK);
    let names: Vec<&str> =
        targets.body.as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"nextcloud.home.arpa"), "{names:?}");
    assert!(names.contains(&"docker01.home.arpa"), "l'agent simulé s'est enregistré : {names:?}");
    assert!(names.iter().all(|name| name.contains("home.arpa")), "{names:?}");

    for uri in
        ["/api/alerts/rules", "/api/alerts/history", "/api/notify/channels", "/api/status-pages"]
    {
        let reply = app.get(uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri} : {:?}", reply.body);
    }
    let public = app.get("/api/public/status/home", None).await;
    assert_eq!(public.status, StatusCode::OK, "la page de statut publique est servie");

    // Aucun secret ne revient : les canaux semés n'en ont pas, et l'API ne
    // renverrait de toute façon que `has_secret`.
    let channels = app.get("/api/notify/channels", Some(&cookie)).await;
    assert!(!channels.body.to_string().contains("secret_enc"));
}

#[tokio::test]
async fn toute_ecriture_est_refusee_avec_ou_sans_session() {
    let app = demo_app().await;
    let cookie = demo_cookie(&app).await;
    let target = json!({"name": "x", "address": "169.254.169.254", "kind": "http"});
    let writes: &[(&str, &str, Option<Value>)] = &[
        ("POST", "/api/targets", Some(target.clone())),
        ("PUT", "/api/targets/1", Some(target)),
        ("DELETE", "/api/targets/1", None),
        ("POST", "/api/targets/1/probe", Some(json!({}))),
        ("POST", "/api/discovery", Some(json!({"cidr": "10.0.0.0/24"}))),
        (
            "POST",
            "/api/auth/password",
            Some(json!({"current_password": "demo", "new_password": "x"})),
        ),
        ("POST", "/api/auth/totp/enroll", Some(json!({}))),
        ("POST", "/api/users", Some(json!({"username": "eve", "password": "p", "role": "admin"}))),
        ("DELETE", "/api/users/1", None),
        ("POST", "/api/tokens", Some(json!({"name": "t", "scope": "write"}))),
        ("POST", "/api/agent/tokens", Some(json!({"name": "t"}))),
        ("POST", "/api/notify/channels", Some(json!({"name": "c", "kind": "ntfy"}))),
        ("POST", "/api/notify/channels/1/test", Some(json!({}))),
        ("PUT", "/api/notify/policy", Some(json!({}))),
        ("POST", "/api/alerts/rules", Some(json!({}))),
        ("POST", "/api/alerts/rules/1/enable", Some(json!({"enabled": false}))),
        ("POST", "/api/alerts/silences", Some(json!({}))),
        ("POST", "/api/alerts/abc/ack", Some(json!({}))),
        ("PUT", "/api/onboarding", Some(json!({}))),
        ("POST", "/api/backup", Some(json!({}))),
        ("POST", "/api/backup/restore", Some(json!({}))),
        ("POST", "/api/status-pages", Some(json!({}))),
        ("POST", "/api/public/status/home/subscribe", Some(json!({"email": "a@b.c"}))),
        ("POST", "/api/mcp", Some(json!({"jsonrpc": "2.0", "method": "tools/list", "id": 1}))),
        ("POST", "/api/ingest", Some(json!({}))),
        ("PATCH", "/api/targets/1", Some(json!({}))),
    ];
    for (method, uri, body) in writes {
        for session in [Some(cookie.as_str()), None] {
            let reply = app.request(method, uri, body.clone(), session).await;
            assert_refused(&reply, &format!("{method} {uri}"));
        }
    }
    for uri in ["/api/auth/audit", "/api/auth/oidc/start", "/api/push/abc", "/api/agent/commands"] {
        assert_refused(&app.get(uri, Some(&cookie)).await, uri);
    }

    // Rien n'a bougé : pas de cible en plus, pas de compte en plus.
    let targets = app.get("/api/targets", Some(&cookie)).await;
    assert!(!targets.body.to_string().contains("169.254.169.254"));
    let users = app.get("/api/users", Some(&cookie)).await;
    assert_eq!(users.body.as_array().map(Vec::len), Some(1), "{:?}", users.body);

    // Se déconnecter reste possible.
    let logout = app.post("/api/auth/logout", json!({}), Some(&cookie)).await;
    assert!(logout.status.is_success(), "{:?}", logout.body);
}

#[tokio::test]
async fn un_visiteur_ne_peut_pas_verrouiller_le_compte_partage() {
    let app = demo_app().await;
    for _ in 0..12 {
        let reply = app.login_as(demo::DEMO_USERNAME, "mauvais").await;
        assert_ne!(reply.status, StatusCode::NO_CONTENT);
    }
    // Sans adresse cliente (requêtes de test), aucun seau ne subsiste : le
    // compte `demo` n'est jamais bloqué par les échecs des autres.
    demo_cookie(&app).await;
}

#[tokio::test]
async fn aucune_notification_ne_part() {
    notify::disable_sending();
    let channel = notify::channel::ChannelConfig {
        id: 1,
        name: "webhook".into(),
        kind: "webhook".into(),
        enabled: true,
        // Adresse injoignable : si l'envoi partait, l'erreur serait une erreur réseau.
        settings: json!({"url": "http://127.0.0.1:1/hook"}),
        secrets: json!({}),
    };
    let http = reqwest::Client::new();
    let message = notify::message::test_message("webhook");
    let report = notify::deliver(&http, &channel, &message).await;
    assert_eq!(report.error.as_deref(), Some(notify::DISABLED_MESSAGE));
}
