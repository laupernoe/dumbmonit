//! Import d'une sauvegarde Uptime Kuma (`POST /api/targets/import/kuma`) de bout en
//! bout : droits, aperçu, création, doublons, fichier invalide.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dumbmonit_server::collectors::{
    self, DnsCollector, HttpCollector, PingCollector, PushCollector, TcpCollector,
};
use dumbmonit_server::db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const BACKUP: &str = include_str!("fixtures/uptime-kuma-backup.json");

struct Lab {
    app: TestApp,
    admin: String,
}

async fn lab() -> Lab {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(HttpCollector::new()));
    registry.register(Arc::new(PingCollector::new()));
    registry.register(Arc::new(TcpCollector::new()));
    registry.register(Arc::new(DnsCollector::new()));
    registry.register(Arc::new(PushCollector::new(pool.clone())));
    let app = common::build_with_registry(dir, config, pool, registry).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    Lab { app, admin }
}

async fn post(app: &TestApp, uri: &str, body: String, cookie: Option<&str>) -> Reply {
    let mut builder = Request::builder().method("POST").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie).header("x-requested-with", "DumbMonit");
    }
    let request =
        builder.header(header::CONTENT_TYPE, "application/json").body(Body::from(body)).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Reply { status, body, set_cookie: None, location: None }
}

async fn targets(lab: &Lab) -> Vec<Value> {
    let reply = lab.app.get("/api/targets", Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body.as_array().expect("liste").clone()
}

#[tokio::test]
async fn l_apercu_ne_cree_rien_puis_l_import_cree_les_equivalents() {
    let lab = lab().await;
    let url = "/api/targets/import/kuma";

    let anonyme = post(&lab.app, url, BACKUP.into(), None).await;
    assert_eq!(anonyme.status, StatusCode::UNAUTHORIZED, "{}", anonyme.body);

    let apercu =
        post(&lab.app, &format!("{url}?dry_run=true"), BACKUP.into(), Some(&lab.admin)).await;
    assert_eq!(apercu.status, StatusCode::OK, "{}", apercu.body);
    assert_eq!(apercu.body["total"], json!(13));
    assert_eq!(apercu.body["created"], json!(8), "{}", apercu.body);
    assert_eq!(apercu.body["skipped"], json!(5));
    assert_eq!(apercu.body["errors"], json!(0));
    assert_eq!(apercu.body["notifications_ignored"], json!(2));
    assert!(targets(&lab).await.is_empty(), "l'aperçu ne crée rien");

    let reel = post(&lab.app, url, BACKUP.into(), Some(&lab.admin)).await;
    assert_eq!(reel.status, StatusCode::CREATED, "{}", reel.body);
    assert_eq!(reel.body["created"], json!(8));
    let cibles = targets(&lab).await;
    assert_eq!(cibles.len(), 8);

    let blog = cibles.iter().find(|t| t["name"] == "Blog").expect("Blog");
    assert_eq!(blog["kind"], json!("http"));
    assert_eq!(blog["group_name"], json!("Clients/Acme"));
    assert_eq!(blog["tags"]["env"], json!("prod"));
    // Aucun identifiant d'Uptime Kuma n'est recopié.
    let tout = serde_json::to_string(&cibles).unwrap();
    for secret in ["hunter2", "s3cr3t-token", "Bearer abc123", "pushtok-0123"] {
        assert!(!tout.contains(secret), "{secret} ne doit pas être importé");
    }
    let cron = cibles.iter().find(|t| t["name"] == "Nightly backup").expect("heartbeat");
    assert_eq!(cron["kind"], json!("push"));
    assert_eq!(cron["enabled"], json!(false));

    // Le même fichier une seconde fois : tout est déjà suivi.
    let encore = post(&lab.app, url, BACKUP.into(), Some(&lab.admin)).await;
    assert_eq!(encore.body["created"], json!(0), "{}", encore.body);
    assert_eq!(targets(&lab).await.len(), 8);
}

#[tokio::test]
async fn un_fichier_qui_n_est_pas_une_sauvegarde_est_refuse() {
    let lab = lab().await;
    let reply = post(
        &lab.app,
        "/api/targets/import/kuma",
        json!({ "hello": "world" }).to_string(),
        Some(&lab.admin),
    )
    .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
}
