//! Tests d'intégration : le mode relais se règle depuis l'interface.
//!
//! Un jeton créé par le parcours « site distant » fait de la machine qu'il
//! enrôle un relais dès son premier lot ; l'interrupteur de la page de l'agent
//! l'allume ou l'éteint ensuite, et l'accusé de réception le transmet à l'agent.
//! Un agent antérieur, qui ne sait pas suivre ce réglage, est signalé.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_HEADER: &str = "x-dumbmonit-agent-secret";

/// Un lot vide d'une machine ; `modern` : binaire qui suit le réglage du relais.
fn batch(modern: bool, relay: bool, site: Option<&str>) -> Value {
    json!({
        "protocol": 1,
        "identity": {
            "hostname": "relay-lyon", "os": "linux", "agent_version": "0.13.0",
            "machine_id": "id-relay-lyon", "commands_enabled": false,
            "binding_supported": true, "relay": relay, "site": site,
            "relay_control": modern,
        },
        "sent_at_ms": 0,
        "samples": [],
    })
}

async fn push(app: &TestApp, enrolment: &str, secret: Option<&str>, body: &Value) -> Reply {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        builder = builder.header(SECRET_HEADER, secret);
    }
    let response = app
        .router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie: None,
        location: None,
    }
}

async fn token(app: &TestApp, admin: &str, body: Value) -> Value {
    let reply = app.post("/api/agent/tokens", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body
}

async fn set_relay(app: &TestApp, admin: &str, host: i64, body: Value) -> Reply {
    app.put(&format!("/api/targets/{host}/agent/relay"), body, Some(admin)).await
}

#[tokio::test]
async fn a_remote_site_token_makes_the_enrolled_agent_a_relay_without_any_manual_step() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let body = json!({ "name": "Lyon", "relay": true, "site": "  Agence de Lyon " });
    let created = token(&app, &admin, body).await;
    assert_eq!(created["relay"], json!(true));
    assert_eq!(created["relay_site"], json!("Agence de Lyon"));
    let enrolment = created["secret"].as_str().expect("jeton").to_string();

    // Le premier lot vient d'un agent installé par la commande ordinaire.
    let first = push(&app, &enrolment, None, &batch(true, false, None)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    assert_eq!(first.body["relay"], json!({ "enabled": true, "site": "Agence de Lyon" }));
    let secret = first.body["agent_secret"].as_str().expect("secret").to_string();
    let host = first.body["target_id"].as_i64().expect("hôte");

    // L'agent adopte le réglage et le déclare au lot suivant.
    let second =
        push(&app, &enrolment, Some(&secret), &batch(true, true, Some("Agence de Lyon"))).await;
    assert_eq!(second.status, StatusCode::OK, "{}", second.body);
    let view = app.get(&format!("/api/targets/{host}/agent"), Some(&admin)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    assert_eq!(view.body["relay"], json!(true));
    assert_eq!(view.body["relay_wanted"], json!(true));
    assert_eq!(view.body["relay_control"], json!(true));
    assert_eq!(view.body["site"], json!("Agence de Lyon"));
}

#[tokio::test]
async fn the_relay_switch_reaches_the_agent_in_its_next_acknowledgement() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = token(&app, &admin, json!({ "name": "machine" })).await;
    assert_eq!(created["relay"], json!(false));
    let enrolment = created["secret"].as_str().expect("jeton").to_string();
    let first = push(&app, &enrolment, None, &batch(true, false, None)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    assert_eq!(first.body["relay"], json!({ "enabled": false }), "éteint par défaut");
    let secret = first.body["agent_secret"].as_str().expect("secret").to_string();
    let host = first.body["target_id"].as_i64().expect("hôte");

    let on = set_relay(&app, &admin, host, json!({ "enabled": true, "site": "Annexe" })).await;
    assert_eq!(on.status, StatusCode::OK, "{}", on.body);
    assert_eq!(on.body["relay_wanted"], json!(true));
    assert_eq!(on.body["relay_wanted_site"], json!("Annexe"));
    assert_eq!(on.body["relay"], json!(false), "l'agent ne l'a pas encore adopté");

    let next = push(&app, &enrolment, Some(&secret), &batch(true, false, None)).await;
    assert_eq!(next.body["relay"], json!({ "enabled": true, "site": "Annexe" }));

    let off = set_relay(&app, &admin, host, json!({ "enabled": false })).await;
    assert_eq!(off.status, StatusCode::OK, "{}", off.body);
    let next = push(&app, &enrolment, Some(&secret), &batch(true, true, Some("Annexe"))).await;
    assert_eq!(next.body["relay"], json!({ "enabled": false }));

    // Un site trop long est refusé, une cible qui n'est pas un agent aussi.
    let too_long = json!({ "enabled": true, "site": "x".repeat(81) });
    let long = set_relay(&app, &admin, host, too_long).await;
    assert_eq!(long.status, StatusCode::BAD_REQUEST, "{}", long.body);
    let missing = set_relay(&app, &admin, 9999, json!({ "enabled": true })).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);
}

#[tokio::test]
async fn an_agent_too_old_to_follow_the_switch_is_reported_as_such() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = token(&app, &admin, json!({ "name": "ancien" })).await;
    let enrolment = created["secret"].as_str().expect("jeton").to_string();
    // Un agent antérieur n'envoie pas `relay_control`.
    let mut old = batch(true, false, None);
    old["identity"].as_object_mut().unwrap().remove("relay_control");
    let first = push(&app, &enrolment, None, &old).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let host = first.body["target_id"].as_i64().expect("hôte");

    let on = set_relay(&app, &admin, host, json!({ "enabled": true })).await;
    assert_eq!(on.status, StatusCode::OK, "{}", on.body);
    assert_eq!(on.body["relay_control"], json!(false), "l'interface doit le dire trop ancien");
    assert_eq!(on.body["relay_wanted"], json!(true));
    assert_eq!(on.body["relay"], json!(false));
}
