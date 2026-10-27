//! Journaux centralisés : réglage de la collecte d'un agent, réception de ses
//! lignes, état des récepteurs, et règles « motif de journal ».

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_HEADER: &str = "x-dumbmonit-agent-secret";
const KEY_HEADER: &str = "x-dumbmonit-agent-key";

fn batch() -> Value {
    json!({
        "protocol": 1,
        "identity": {
            "hostname": "nas",
            "os": "linux",
            "agent_version": "0.14.0",
            "machine_id": "id-nas",
            "commands_enabled": true,
            "binding_supported": true,
        },
        "sent_at_ms": 0,
        "samples": [],
    })
}

async fn agent_post(
    app: &TestApp,
    uri: &str,
    enrolment: &str,
    secret: &str,
    body: &Value,
) -> Reply {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(KEY_HEADER, "id-nas");
    // Une machine pas encore liée n'envoie pas l'en-tête du tout.
    if !secret.is_empty() {
        builder = builder.header(SECRET_HEADER, secret);
    }
    let request = builder.body(Body::from(body.to_string())).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie: None,
        location: None,
    }
}

/// Enrôle une machine ; rend (jeton, secret de liaison, cible, session admin).
async fn enrolled(app: &TestApp) -> (String, String, i64, String) {
    let admin = app.admin_cookie().await;
    let reply = app.post("/api/agent/tokens", json!({ "name": "parc" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let enrolment = reply.body["secret"].as_str().expect("jeton").to_string();
    let first = agent_post(app, "/api/ingest", &enrolment, "", &batch()).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let secret = first.body["agent_secret"].as_str().expect("secret").to_string();
    let target = first.body["target_id"].as_i64().expect("cible");
    (enrolment, secret, target, admin)
}

#[tokio::test]
async fn an_agent_collects_logs_only_once_an_administrator_turns_it_on() {
    let app = TestApp::configured().await;
    let (enrolment, secret, target, admin) = enrolled(&app).await;
    let uri = format!("/api/logs/agents/{target}");

    let reply = app.get(&uri, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["enabled"], false);
    assert_eq!(reply.body["received_total"], 0);
    assert_eq!(reply.body["store_running"], false);

    // Éteinte : les lignes sont acquittées puis jetées.
    let lines = json!({ "lines": [
        { "ts_ms": chrono::Utc::now().timestamp_millis(), "level": "error",
          "source": "journald:ssh.service", "message": "Failed password" },
        { "ts_ms": chrono::Utc::now().timestamp_millis(), "level": "info",
          "source": "journald:ssh.service", "message": "chatter" },
    ] });
    let sent = agent_post(&app, "/api/agent/logs", &enrolment, &secret, &lines).await;
    assert_eq!(sent.status, StatusCode::NO_CONTENT, "{}", sent.body);
    let reply = app.get(&uri, Some(&admin)).await;
    assert_eq!(reply.body["received_total"], 0);

    let setting = json!({ "enabled": true, "min_level": "warn", "files": ["/var/log/x.log"] });
    let reply = app.put(&uri, setting, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["enabled"], true);
    assert_eq!(reply.body["files"], json!(["/var/log/x.log"]));

    // Le réglage voyage dans l'accusé de réception du lot suivant.
    let ack = agent_post(&app, "/api/ingest", &enrolment, &secret, &batch()).await;
    assert_eq!(ack.status, StatusCode::OK, "{}", ack.body);
    assert_eq!(ack.body["logs"]["enabled"], true);
    assert_eq!(ack.body["logs"]["files"], json!(["/var/log/x.log"]));

    // Allumée : seule la ligne au-dessus du niveau minimal est gardée.
    let sent = agent_post(&app, "/api/agent/logs", &enrolment, &secret, &lines).await;
    assert_eq!(sent.status, StatusCode::NO_CONTENT, "{}", sent.body);
    let reply = app.get(&uri, Some(&admin)).await;
    assert_eq!(reply.body["received_total"], 1);
    assert!(reply.body["last_received_at"].is_string());
}

#[tokio::test]
async fn the_agent_log_route_checks_the_machine_and_the_setting_is_validated() {
    let app = TestApp::configured().await;
    let (enrolment, _, target, admin) = enrolled(&app).await;

    let lines = json!({ "lines": [] });
    let denied = agent_post(&app, "/api/agent/logs", &enrolment, "dmab_wrong", &lines).await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.body);

    let uri = format!("/api/logs/agents/{target}");
    for bad in [
        json!({ "enabled": true, "min_level": "loud" }),
        json!({ "enabled": true, "files": ["relative/path.log"] }),
        json!({ "enabled": true, "files": ["/var/log/../etc/shadow"] }),
        json!({ "enabled": true, "eventlog": ["System'; calc; '"] }),
    ] {
        let reply = app.put(&uri, bad.clone(), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}: {}", reply.body);
    }
    let missing = app.get("/api/logs/agents/99999", Some(&admin)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);

    // Un compte en lecture seule n'en lit rien.
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get(&uri, Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/logs/inputs", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_receivers_are_reported_off_by_default() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = app.get("/api/logs/inputs", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["syslog"]["enabled"], false);
    assert_eq!(reply.body["traps"]["enabled"], false);
    assert_eq!(reply.body["store_running"], false);
}

#[tokio::test]
async fn a_log_pattern_rule_keeps_its_window_and_refuses_pipes() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let rule = json!({
        "name": "SSH failures",
        "kind": "log_pattern",
        "query": "service:=syslog \"Failed password\"",
        "operator": ">=",
        "threshold": 3,
        "severity": "warning",
        "params": { "window_secs": 600, "per_target": false },
    });
    let created = app.post("/api/alerts/rules", rule.clone(), Some(&admin)).await;
    assert!(created.status.is_success(), "{} {}", created.status, created.body);
    assert_eq!(created.body["kind"], "log_pattern");
    assert_eq!(created.body["params"]["log_window_secs"], 600);
    assert_eq!(created.body["params"]["log_per_target"], false);

    // Un `PUT` qui renvoie la règle telle quelle ne remet pas la fenêtre à zéro.
    let id = created.body["id"].as_i64().expect("id");
    let mut again = created.body.clone();
    again["name"] = json!("SSH failures (renamed)");
    let saved = app.put(&format!("/api/alerts/rules/{id}"), again, Some(&admin)).await;
    assert!(saved.status.is_success(), "{} {}", saved.status, saved.body);
    assert_eq!(saved.body["params"]["log_window_secs"], 600);
    assert_eq!(saved.body["params"]["log_per_target"], false);

    let mut piped = rule.clone();
    piped["name"] = json!("Piped");
    piped["query"] = json!("error | stats count()");
    let refused = app.post("/api/alerts/rules", piped, Some(&admin)).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);

    let mut short = rule;
    short["name"] = json!("Too short");
    short["params"] = json!({ "window_secs": 10 });
    let refused = app.post("/api/alerts/rules", short, Some(&admin)).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);
}
