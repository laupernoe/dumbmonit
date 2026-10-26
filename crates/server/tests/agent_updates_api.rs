//! Mise à jour automatique des agents, par l'API réelle : l'offre jointe à
//! l'accusé de réception, ses conditions (empreinte, réglages, fenêtre,
//! demande explicite, échec rapporté), le téléchargement par le canal des
//! agents et la vue de l'interface.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::Engine as _;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_HEADER: &str = "x-dumbmonit-agent-secret";
const KEY_HEADER: &str = "x-dumbmonit-agent-key";
const BINARY: &[u8] = b"un faux binaire d'agent, version de ce serveur";
const FILE: &str = "dumbmonit-agent-linux-x86_64";

struct Fleet {
    app: TestApp,
    _agents: tempfile::TempDir,
    admin: String,
    enrolment: String,
    sha256: String,
}

/// Instance qui livre un binaire Linux signé, un administrateur et un jeton.
async fn fleet() -> Fleet {
    let agents = tempfile::tempdir().expect("répertoire des agents");
    std::fs::write(agents.path().join(FILE), BINARY).unwrap();
    std::fs::write(agents.path().join(format!("{FILE}.sig")), [7u8; 64]).unwrap();
    let dir = agents.path().to_path_buf();
    let app = common::setup_with(move |config| config.agent_dir = dir).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    let token = json!({ "name": "parc", "reusable": true });
    let reply = app.post("/api/agent/tokens", token, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let enrolment = reply.body["secret"].as_str().unwrap().to_string();
    let sha256 = hex::encode(Sha256::digest(BINARY));
    Fleet { app, _agents: agents, admin, enrolment, sha256 }
}

async fn send(app: &TestApp, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    (status, bytes.to_vec())
}

/// Un lot vide ; `self_update` : ce que l'agent dit de son binaire.
async fn push(fleet: &Fleet, secret: Option<&str>, self_update: Value) -> Reply {
    let mut identity = json!({
        "hostname": "nas",
        "os": "linux",
        "arch": "x86_64",
        "agent_version": "0.0.1",
        "machine_id": "id-nas",
        "binding_supported": true,
    });
    if !self_update.is_null() {
        identity["self_update"] = self_update;
    }
    let batch = json!({ "protocol": 1, "identity": identity, "sent_at_ms": 0, "samples": [] });
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {}", fleet.enrolment))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        builder = builder.header(SECRET_HEADER, secret);
    }
    let request = builder.body(Body::from(batch.to_string())).unwrap();
    let (status, bytes) = send(&fleet.app, request).await;
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie: None,
        location: None,
    }
}

fn running(sha: &str) -> Value {
    json!({ "sha256": sha, "platform": "linux-x86_64", "ability": "ready", "signing_keys": true })
}

/// Enrôle la machine et rend (secret de liaison, cible).
async fn enrolled(fleet: &Fleet) -> (String, i64) {
    let reply = push(fleet, None, Value::Null).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    (
        reply.body["agent_secret"].as_str().unwrap().to_string(),
        reply.body["target_id"].as_i64().unwrap(),
    )
}

fn agent_status(view: &Reply, id: i64) -> Value {
    view.body["agents"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["id"] == id)
        .cloned()
        .expect("agent listé")
}

#[tokio::test]
async fn an_outdated_agent_is_offered_the_signed_binary_of_this_build() {
    let fleet = fleet().await;
    let (secret, target) = enrolled(&fleet).await;
    let old = "0".repeat(64);

    let reply = push(&fleet, Some(&secret), running(&old)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let offer = &reply.body["agent_update"];
    assert_eq!(offer["file"], FILE);
    assert_eq!(offer["sha256"], fleet.sha256);
    assert_eq!(offer["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(offer["size"], BINARY.len());
    let signature = base64::engine::general_purpose::STANDARD.encode([7u8; 64]);
    assert_eq!(offer["signature"], signature);
    assert_eq!(offer["allow_unsigned"], false);
    assert_eq!(offer["force"], false);

    let view = fleet.app.get("/api/agent/updates", Some(&fleet.admin)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    assert_eq!(agent_status(&view, target)["status"], "updating");
    assert_eq!(view.body["binaries"][0]["signed"], true);

    // Une fois le binaire en place : plus d'offre, « à jour ».
    let reply = push(&fleet, Some(&secret), running(&fleet.sha256)).await;
    assert!(reply.body.get("agent_update").is_none(), "{}", reply.body);
    let view = fleet.app.get("/api/agent/updates", Some(&fleet.admin)).await;
    assert_eq!(agent_status(&view, target)["status"], "up_to_date");
}

#[tokio::test]
async fn older_agents_and_containers_are_never_offered_anything() {
    let fleet = fleet().await;
    let (secret, target) = enrolled(&fleet).await;

    // Agent antérieur : il ne dit rien de son binaire.
    let reply = push(&fleet, Some(&secret), Value::Null).await;
    assert!(reply.body.get("agent_update").is_none());
    let view = fleet.app.get("/api/agent/updates", Some(&fleet.admin)).await;
    assert_eq!(agent_status(&view, target)["status"], "legacy");

    let container = json!({ "sha256": "0".repeat(64), "platform": "linux-x86_64",
                            "ability": "container" });
    let reply = push(&fleet, Some(&secret), container).await;
    assert!(reply.body.get("agent_update").is_none());
    let view = fleet.app.get("/api/agent/updates", Some(&fleet.admin)).await;
    assert_eq!(agent_status(&view, target)["status"], "container");
}

#[tokio::test]
async fn the_manual_window_waits_for_update_now() {
    let fleet = fleet().await;
    let (secret, target) = enrolled(&fleet).await;
    let manual = json!({ "window": "manual" });
    let reply = fleet.app.put("/api/agent/updates", manual, Some(&fleet.admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["settings"]["window"], "manual");

    let old = "0".repeat(64);
    let reply = push(&fleet, Some(&secret), running(&old)).await;
    assert!(reply.body.get("agent_update").is_none(), "fenêtre manuelle : rien");

    let now = format!("/api/agent/updates/{target}/now");
    let reply = fleet.app.post(&now, json!({}), Some(&fleet.admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = push(&fleet, Some(&secret), running(&old)).await;
    assert_eq!(reply.body["agent_update"]["force"], true, "{}", reply.body);
}

#[tokio::test]
async fn a_rolled_back_version_is_not_offered_again_by_itself() {
    let fleet = fleet().await;
    let (secret, target) = enrolled(&fleet).await;
    let old = "0".repeat(64);
    let mut info = running(&old);
    info["last"] = json!({
        "outcome": "rolled_back",
        "from_version": "0.0.1",
        "to_version": env!("CARGO_PKG_VERSION"),
        "to_sha256": fleet.sha256,
        "reason": "the new agent had no batch accepted by the server within 120 s",
        "at_ms": chrono::Utc::now().timestamp_millis(),
    });

    let reply = push(&fleet, Some(&secret), info).await;
    assert!(reply.body.get("agent_update").is_none(), "{}", reply.body);
    let view = fleet.app.get("/api/agent/updates", Some(&fleet.admin)).await;
    let agent = agent_status(&view, target);
    assert_eq!(agent["status"], "failed");
    assert!(agent["reason"].as_str().unwrap().contains("120 s"));
    assert_eq!(agent["last"]["outcome"], "rolled_back");
}

#[tokio::test]
async fn the_per_agent_switch_and_the_global_one() {
    let fleet = fleet().await;
    let (secret, target) = enrolled(&fleet).await;
    let old = "0".repeat(64);
    let agent = format!("/api/agent/updates/{target}");
    assert!(push(&fleet, Some(&secret), running(&old)).await.body.get("agent_update").is_some());

    let reply = fleet.app.put(&agent, json!({ "enabled": false }), Some(&fleet.admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(agent_status(&reply, target)["status"], "disabled");
    assert!(push(&fleet, Some(&secret), running(&old)).await.body.get("agent_update").is_none());

    // De retour au réglage global, coupé lui aussi.
    fleet.app.put(&agent, json!({ "enabled": null }), Some(&fleet.admin)).await;
    fleet.app.put("/api/agent/updates", json!({ "enabled": false }), Some(&fleet.admin)).await;
    assert!(push(&fleet, Some(&secret), running(&old)).await.body.get("agent_update").is_none());
    // Forcée pour la machine : elle passe.
    fleet.app.put(&agent, json!({ "enabled": true, "canary": true }), Some(&fleet.admin)).await;
    let reply = push(&fleet, Some(&secret), running(&old)).await;
    assert_eq!(reply.body["agent_update"]["sha256"], fleet.sha256, "{}", reply.body);

    let view = fleet.app.get("/api/agent/updates", Some(&fleet.admin)).await;
    assert_eq!(agent_status(&view, target)["canary"], true);
    assert_eq!(view.body["rollout"]["canaries"], 1);
    let missing = fleet.app.put("/api/agent/updates/999999", json!({}), Some(&fleet.admin)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    let night = json!({ "window": "nuit" });
    let bad = fleet.app.put("/api/agent/updates", night, Some(&fleet.admin)).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_binary_is_downloaded_through_the_agent_channel_by_a_bound_machine_only() {
    let fleet = fleet().await;
    let (secret, _) = enrolled(&fleet).await;
    let download = |secret: Option<&str>| {
        let mut builder = Request::builder()
            .uri(format!("/api/agent/update/download/{FILE}"))
            .header(header::AUTHORIZATION, format!("Bearer {}", fleet.enrolment))
            .header(KEY_HEADER, "id-nas");
        if let Some(secret) = secret {
            builder = builder.header(SECRET_HEADER, secret);
        }
        builder.body(Body::empty()).unwrap()
    };

    let (status, bytes) = send(&fleet.app, download(Some(&secret))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, BINARY);
    // Sans le secret de liaison, une voisine porteuse du jeton n'obtient rien.
    let (status, _) = send(&fleet.app, download(None)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // La signature est aussi servie à côté du binaire, pour qui vérifie à la main.
    let request = Request::builder().uri(format!("/download/{FILE}.sig")).body(Body::empty());
    let (status, bytes) = send(&fleet.app, request.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, [7u8; 64]);
}

#[tokio::test]
async fn the_update_settings_are_for_administrators_in_a_session() {
    let fleet = fleet().await;
    let viewer = fleet.app.viewer_cookie(&fleet.admin).await;
    let off = json!({ "enabled": false });
    let reply = fleet.app.put("/api/agent/updates", off, Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = fleet.app.get("/api/agent/updates", None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}
