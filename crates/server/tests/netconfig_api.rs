//! Sauvegarde des configurations réseau, côté API : droits par rôle,
//! masquage des secrets, téléchargement réservé, journal d'audit, clé d'hôte
//! changée et acceptée, identifiants jamais renvoyés. Les versions sont posées
//! directement en base, comme un relevé les aurait rangées.

mod common;

use std::sync::Arc;

use axum::http::StatusCode;
use dumbmonit_server::netconfig::{self, store};
use dumbmonit_server::{collectors, db};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use common::TestApp;

const OPERATOR_PASSWORD: &str = "mot-de-passe-operateur";
const VIEWER_PASSWORD: &str = "mot-de-passe-du-lecteur";

/// Secrets de l'équipement : aucun ne doit sortir masqué ou non par un chemin
/// qui ne l'autorise pas.
const COMMUNITY: &str = "c0mmun1ty-s3cr3t";
const ENABLE_HASH: &str = "$1$mERr$hx5rVt7rPNoS4wqbXKX7m0";
const LOGIN_PASSWORD: &str = "ssh-login-p4ss";
const ENABLE_SECRET: &str = "en4ble-s3cret";

struct Lab {
    app: TestApp,
    pool: SqlitePool,
    admin: String,
    operator: String,
    viewer: String,
    target: i64,
    first: i64,
    second: i64,
}

fn config(community: &str) -> String {
    format!(
        "!\n! Last configuration change at 10:00:00 UTC\nhostname sw1\n\
         enable secret 5 {ENABLE_HASH}\nsnmp-server community {community} RO\ninterface Gi0/1\n \
         description uplink\n!\nend\n"
    )
}

async fn lab() -> Lab {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config_file = common::base_config(dir.path());
    let pool = db::open(&config_file.database_path()).await.expect("ouverture de la base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(netconfig::NetconfigCollector::new(pool.clone())));
    let app = common::build_with_registry(dir, config_file, pool.clone(), registry).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    for (name, role, password) in
        [("ops", "operator", OPERATOR_PASSWORD), ("watcher", "viewer", VIEWER_PASSWORD)]
    {
        let reply = app
            .post(
                "/api/users",
                json!({ "username": name, "role": role, "password": password }),
                Some(&admin),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{name}: {}", reply.body);
    }
    let operator = app.login_as("ops", OPERATOR_PASSWORD).await.cookie();
    let viewer = app.login_as("watcher", VIEWER_PASSWORD).await.cookie();

    let reply = app
        .post(
            "/api/targets",
            json!({
                "name": "sw1",
                "address": "192.0.2.10",
                "kind": "netconfig",
                "tags": { "profile": "cisco_ios" },
                "credential": {
                    "type": "ssh_login",
                    "username": "backup",
                    "password": LOGIN_PASSWORD,
                    "enable_secret": ENABLE_SECRET
                }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let target = reply.body["id"].as_i64().expect("id");

    // Même secret d'instance que l'application : le même chiffreur.
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    let mut ids = Vec::new();
    for community in ["public", COMMUNITY] {
        let text = config(community);
        let sealed = netconfig::seal(&cipher, &text).expect("chiffrement");
        let digest = netconfig::digest_of(&cipher, "cisco_ios", &text);
        let id = store::insert(
            &pool,
            target,
            &store::NewVersion {
                profile: "cisco_ios",
                digest: &digest,
                size_bytes: text.len() as i64,
                lines: text.lines().count() as i64,
                added: 1,
                removed: 1,
                content_enc: &sealed,
            },
        )
        .await
        .expect("version");
        ids.push(id);
    }
    Lab { app, pool, admin, operator, viewer, target, first: ids[0], second: ids[1] }
}

fn text_of(body: &Value) -> String {
    body.to_string()
}

#[tokio::test]
async fn versions_are_stored_encrypted() {
    let lab = lab().await;
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT content_enc FROM netconfig_versions WHERE id = ?")
            .bind(lab.second)
            .fetch_one(&lab.pool)
            .await
            .expect("contenu");
    let needle = COMMUNITY.as_bytes();
    assert!(!stored.windows(needle.len()).any(|w| w == needle), "stored in clear");
    let digest: String = sqlx::query_scalar("SELECT digest FROM netconfig_versions WHERE id = ?")
        .bind(lab.second)
        .fetch_one(&lab.pool)
        .await
        .expect("empreinte");
    assert!(!digest.contains(COMMUNITY));
}

#[tokio::test]
async fn each_role_sees_what_it_may() {
    let lab = lab().await;
    let app = &lab.app;
    let id = lab.target;
    let version = format!("/api/targets/{id}/netconfig/versions/{}", lab.second);
    let diff = format!("/api/targets/{id}/netconfig/diff?to={}", lab.second);
    let download = format!("/api/targets/{id}/netconfig/versions/{}/download", lab.second);

    // Le lecteur voit la liste, jamais le contenu.
    let overview = app.get(&format!("/api/targets/{id}/netconfig"), Some(&lab.viewer)).await;
    assert_eq!(overview.status, StatusCode::OK, "{}", overview.body);
    assert_eq!(overview.body["versions"].as_array().map(Vec::len), Some(2));
    assert_eq!(overview.body["can_read"], json!(false));
    assert!(!text_of(&overview.body).contains(COMMUNITY));
    for uri in [&version, &diff, &download] {
        let reply = app.get(uri, Some(&lab.viewer)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "viewer {uri}");
    }

    // L'opérateur lit, secrets masqués ; un secret changé reste visible comme
    // une ligne changée, sans être révélé.
    let reply = app.get(&version, Some(&lab.operator)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["masked"], json!(true));
    let content = reply.body["content"].as_str().expect("texte");
    assert!(content.contains("hostname sw1"));
    for secret in [COMMUNITY, ENABLE_HASH] {
        assert!(!content.contains(secret), "secret shown to an operator: {content}");
    }
    assert!(content.contains("<masked:"));
    let reply = app.get(&diff, Some(&lab.operator)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["added"], json!(1));
    assert_eq!(reply.body["removed"], json!(1));
    assert!(!text_of(&reply.body).contains(COMMUNITY));
    assert!(!text_of(&reply.body).contains("public RO"), "the old secret is masked too");
    let revealing = [format!("{version}?unmasked=true"), format!("{diff}&unmasked=true")];
    for uri in revealing.iter().chain([&download]) {
        let reply = app.get(uri, Some(&lab.operator)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "operator {uri}");
    }

    // L'administrateur peut lever le masquage et télécharger, chaque fois audité.
    let reply = app.get(&format!("{version}?unmasked=true"), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body["content"].as_str().unwrap_or("").contains(COMMUNITY));
    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_log
         WHERE action IN ('netconfig.revealed', 'netconfig.downloaded')",
    )
    .fetch_one(&lab.pool)
    .await
    .expect("journal");
    assert_eq!(audited, 1);
    let reply = app.request("GET", &download, None, Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let audited: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_log WHERE action = 'netconfig.downloaded'")
            .fetch_one(&lab.pool)
            .await
            .expect("journal");
    assert_eq!(audited, 1);

    // Sans `to` précédent connu : la toute première version n'a pas de départ.
    let first = app
        .get(&format!("/api/targets/{id}/netconfig/diff?to={}", lab.first), Some(&lab.operator))
        .await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    assert_eq!(first.body["from"], Value::Null);
}

#[tokio::test]
async fn the_ssh_login_is_never_returned() {
    let lab = lab().await;
    for cookie in [&lab.admin, &lab.operator, &lab.viewer] {
        let reply = lab.app.get(&format!("/api/targets/{}", lab.target), Some(cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let text = text_of(&reply.body);
        assert!(!text.contains(LOGIN_PASSWORD) && !text.contains(ENABLE_SECRET), "{text}");
    }
    let reply = lab.app.get(&format!("/api/targets/{}", lab.target), Some(&lab.admin)).await;
    assert_eq!(reply.body["credential_public"]["username"], json!("backup"), "{}", reply.body);
}

#[tokio::test]
async fn a_changed_host_key_waits_for_an_administrator() {
    let lab = lab().await;
    let app = &lab.app;
    let id = lab.target;
    let old = "SHA256:Nh0Me49Zh9fDw/VYUfq43IJmI1T+XrjiYONPND8GzaM";
    let new = "SHA256:p2QAMXNIC1TJYWeIOttrVc98/R1BUFWu3/LiyKgUfQM";
    store::pin_if_absent(&lab.pool, id, old).await.expect("première clé");
    // Une deuxième « première » clé ne remplace jamais la mémorisée.
    store::pin_if_absent(&lab.pool, id, new).await.expect("sans effet");
    store::host_key_mismatch(&lab.pool, id, new).await.expect("clé changée");

    let overview = app.get(&format!("/api/targets/{id}/netconfig"), Some(&lab.viewer)).await;
    assert_eq!(overview.body["host_key"]["pinned"], json!(old));
    assert_eq!(overview.body["host_key"]["changed_to"], json!(new));
    assert_eq!(overview.body["last_error_kind"], json!("host_key"));

    // La sonde le dit par la métrique de l'alerte, sans passer pour une panne.
    let cipher = db::init_cipher(&lab.pool, "secret-de-test-suffisamment-long").await.unwrap();
    let target = db::targets::get(&lab.pool, &cipher, id).await.expect("lecture").expect("cible");
    let collector = netconfig::NetconfigCollector::new(lab.pool.clone());
    let samples =
        dumbmonit_proto::Collector::probe(&collector, &target).await.expect("pas une panne");
    let changed = samples.iter().find(|s| s.metric == "netconfig_host_key_changed").expect("série");
    assert_eq!(changed.value, 1.0);

    let accept = format!("/api/targets/{id}/netconfig/host-key/accept");
    let reply = app.post(&accept, json!({ "fingerprint": new }), Some(&lab.operator)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "an operator cannot accept");
    let reply = app.post(&accept, json!({ "fingerprint": old }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "only the key presented can be accepted");
    let reply = app.post(&accept, json!({ "fingerprint": new }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);

    let state = store::state(&lab.pool, id).await.expect("état").expect("ligne");
    assert_eq!(state.host_key.as_deref(), Some(new));
    assert_eq!(state.host_key_seen, None);
    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_log WHERE action = 'netconfig.host_key_accepted'",
    )
    .fetch_one(&lab.pool)
    .await
    .expect("journal");
    assert_eq!(audited, 1);
}

#[tokio::test]
async fn another_kind_is_not_a_configuration_backup() {
    let lab = lab().await;
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "demo", "address": "198.51.100.1", "kind": "dummy" }),
            Some(&lab.admin),
        )
        .await;
    let other = reply.body["id"].as_i64().expect("id");
    let reply = lab.app.get(&format!("/api/targets/{other}/netconfig"), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    // Une version d'un autre équipement ne se lit pas par celui-ci.
    let reply = lab
        .app
        .get(&format!("/api/targets/{other}/netconfig/versions/{}", lab.second), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}
