//! Tests d'intégration : règles du journal d'événements Windows d'une machine.
//!
//! De la création d'une règle dans l'interface à l'événement reçu : la règle
//! part dans l'accusé de réception d'un agent Windows (jamais d'un agent
//! Linux), l'événement qu'il renvoie est rangé et compté, ce qui vise une
//! règle inconnue est ignoré.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_HEADER: &str = "x-dumbmonit-agent-secret";

fn batch(os: &str, machine: &str, event_log: Option<Value>) -> Value {
    let mut batch = json!({
        "protocol": 1,
        "identity": {
            "hostname": machine,
            "os": os,
            "agent_version": "0.9.0",
            "machine_id": format!("id-{machine}"),
            "binding_supported": true,
        },
        "sent_at_ms": 0,
        "samples": [{ "metric": "cpu_usage_percent", "value": 1.0, "kind": "gauge",
                      "ts_ms": chrono::Utc::now().timestamp_millis(), "labels": {} }],
    });
    if let Some(event_log) = event_log {
        batch["event_log"] = event_log;
    }
    batch
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

struct Machine {
    enrolment: String,
    secret: String,
    host: i64,
}

async fn enrol(app: &TestApp, admin: &str, os: &str, name: &str) -> Machine {
    let reply = app.post("/api/agent/tokens", json!({ "name": name }), Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let enrolment = reply.body["secret"].as_str().expect("jeton").to_string();
    let first = push(app, &enrolment, None, &batch(os, name, None)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    Machine {
        secret: first.body["agent_secret"].as_str().expect("secret").to_string(),
        host: first.body["target_id"].as_i64().expect("hôte"),
        enrolment,
    }
}

fn failed_logons() -> Value {
    json!({
        "name": "Failed logons",
        "channel": "Security",
        "provider": "Microsoft-Windows-Security-Auditing",
        "event_ids": "4625,4740",
        "severity": "warning",
    })
}

fn event(rule_id: i64, record_id: u64) -> Value {
    json!({
        "rule_id": rule_id,
        "channel": "Security",
        "provider": "Microsoft-Windows-Security-Auditing",
        "event_id": 4625,
        "level": 0,
        "record_id": record_id,
        "time_created_ms": chrono::Utc::now().timestamp_millis() - 1_000,
        "computer": "srv-ad.example.lan",
        "message": "An account failed to log on.\r\n\r\nAccount Name: admin",
    })
}

#[tokio::test]
async fn a_rule_reaches_a_windows_agent_and_its_events_come_back() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let machine = enrol(&app, &admin, "windows", "srv-ad").await;
    let base = format!("/api/targets/{}/agent/event-rules", machine.host);

    // Aucune règle par défaut, des exemples proposés.
    let empty = app.get(&base, Some(&admin)).await;
    assert_eq!(empty.status, StatusCode::OK, "{}", empty.body);
    assert_eq!(empty.body["supported"], true);
    assert_eq!(empty.body["rules"], json!([]));
    assert_eq!(empty.body["max_per_minute"], 10);
    let presets = empty.body["presets"].as_array().expect("exemples");
    assert!(presets.iter().any(|p| p["id"] == "failed_logon" && p["event_ids"] == "4625"));
    assert!(presets.iter().any(|p| p["id"] == "unexpected_reboot" && p["severity"] == "critical"));

    // Un agent Windows apprend qu'il n'a aucune règle.
    let none =
        push(&app, &machine.enrolment, Some(&machine.secret), &batch("windows", "srv-ad", None))
            .await;
    assert_eq!(none.status, StatusCode::OK, "{}", none.body);
    assert_eq!(none.body["event_rules"], json!([]));

    let created = app.post(&base, failed_logons(), Some(&admin)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let rule = created.body["id"].as_i64().expect("identifiant");
    assert_eq!(created.body["event_ids"], "4625, 4740");
    assert_eq!(created.body["enabled"], true);
    assert_eq!(created.body["matches_total"], 0);

    let ack =
        push(&app, &machine.enrolment, Some(&machine.secret), &batch("windows", "srv-ad", None))
            .await;
    assert_eq!(ack.status, StatusCode::OK, "{}", ack.body);
    let sent = ack.body["event_rules"].as_array().expect("règles");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["id"], rule);
    assert_eq!(sent[0]["channel"], "Security");
    assert_eq!(sent[0]["provider"], "Microsoft-Windows-Security-Auditing");
    assert_eq!(
        sent[0]["event_ids"],
        json!([{ "from": 4625, "to": 4625 }, { "from": 4740, "to": 4740 }])
    );

    // Deux événements de la règle, un d'une règle inconnue, des écartés.
    let report = json!({
        "events": [event(rule, 100), event(rule, 101), event(rule + 999, 102)],
        "dropped": [{ "rule_id": rule, "count": 7 }],
    });
    let reply = push(
        &app,
        &machine.enrolment,
        Some(&machine.secret),
        &batch("windows", "srv-ad", Some(report)),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    let events =
        app.get(&format!("/api/targets/{}/agent/events", machine.host), Some(&admin)).await;
    assert_eq!(events.status, StatusCode::OK, "{}", events.body);
    let events = events.body.as_array().expect("événements").clone();
    assert_eq!(events.len(), 2, "l'événement d'une règle inconnue est ignoré");
    assert_eq!(events[0]["record_id"], 101, "du plus récent au plus ancien");
    assert_eq!(events[0]["rule_name"], "Failed logons");
    assert_eq!(events[0]["severity"], "warning");
    assert_eq!(events[0]["event_id"], 4625);
    assert_eq!(events[0]["level"], "information", "le niveau 0 des audits est une information");
    assert_eq!(events[0]["computer"], "srv-ad.example.lan");
    assert!(events[0]["message"].as_str().unwrap().starts_with("An account failed to log on."));
    assert!(events[0]["occurred_at"].is_string());

    let listed = app.get(&base, Some(&admin)).await;
    let counted = &listed.body["rules"][0];
    assert_eq!(counted["matches_total"], 2);
    assert_eq!(counted["dropped_total"], 7);
    assert!(counted["last_match_at"].is_string());

    let limited =
        app.get(&format!("/api/targets/{}/agent/events?limit=1", machine.host), Some(&admin)).await;
    assert_eq!(limited.body.as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn a_linux_agent_is_never_sent_event_rules() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let machine = enrol(&app, &admin, "linux", "nas").await;
    let base = format!("/api/targets/{}/agent/event-rules", machine.host);

    let view = app.get(&base, Some(&admin)).await;
    assert_eq!(view.body["supported"], false);
    let created = app.post(&base, failed_logons(), Some(&admin)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);

    let ack =
        push(&app, &machine.enrolment, Some(&machine.secret), &batch("linux", "nas", None)).await;
    assert_eq!(ack.status, StatusCode::OK, "{}", ack.body);
    assert!(ack.body.get("event_rules").is_none(), "{}", ack.body);
}

#[tokio::test]
async fn rules_are_validated_edited_disabled_and_deleted() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let machine = enrol(&app, &admin, "windows", "srv-files").await;
    let base = format!("/api/targets/{}/agent/event-rules", machine.host);

    for (body, why) in [
        (json!({ "name": "", "channel": "System", "event_ids": "41" }), "nom vide"),
        (json!({ "name": "x", "channel": "", "event_ids": "41" }), "journal vide"),
        (json!({ "name": "x", "channel": "System" }), "aucun filtre"),
        (json!({ "name": "x", "channel": "System", "event_ids": "4x" }), "identifiant illisible"),
        (
            json!({ "name": "x", "channel": "System", "provider": "a']", "event_ids": "41" }),
            "fournisseur qui casserait la requête",
        ),
        (
            json!({ "name": "x", "channel": "System", "event_ids": "41",
                    "pattern": "(", "pattern_regex": true }),
            "expression invalide",
        ),
    ] {
        let reply = app.post(&base, body, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{why} : {}", reply.body);
    }

    let created = app
        .post(
            &base,
            json!({ "name": "Unexpected reboot", "channel": "System",
                    "provider": "Microsoft-Windows-Kernel-Power", "event_ids": "41",
                    "severity": "critical" }),
            Some(&admin),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_i64().unwrap();
    let one = format!("{base}/{id}");

    let edited = app
        .put(
            &one,
            json!({ "name": "Services that crashed", "channel": "System",
                    "event_ids": "7031, 7034", "levels": ["error", "error", "critical"],
                    "pattern": "spooler", "severity": "warning", "enabled": false }),
            Some(&admin),
        )
        .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.body);
    assert_eq!(edited.body["name"], "Services that crashed");
    assert_eq!(edited.body["provider"], Value::Null);
    assert_eq!(edited.body["levels"], json!(["critical", "error"]));
    assert_eq!(edited.body["pattern"], "spooler");
    assert_eq!(edited.body["enabled"], false);

    // Une règle coupée ne part plus vers l'agent.
    let ack =
        push(&app, &machine.enrolment, Some(&machine.secret), &batch("windows", "srv-files", None))
            .await;
    assert_eq!(ack.body["event_rules"], json!([]));

    // Un lecteur voit les règles mais ne les change pas.
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get(&base, Some(&viewer)).await.status, StatusCode::OK);
    assert_eq!(app.delete(&one, Some(&viewer)).await.status, StatusCode::FORBIDDEN);

    assert_eq!(app.delete(&one, Some(&admin)).await.status, StatusCode::NO_CONTENT);
    assert_eq!(app.delete(&one, Some(&admin)).await.status, StatusCode::NOT_FOUND);
    let missing = app.put(&one, failed_logons(), Some(&admin)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    // Une règle d'une autre machine ne se modifie pas par celle-ci.
    let other = enrol(&app, &admin, "windows", "srv-print").await;
    let foreign = app
        .post(
            &format!("/api/targets/{}/agent/event-rules", other.host),
            failed_logons(),
            Some(&admin),
        )
        .await;
    let foreign_id = foreign.body["id"].as_i64().unwrap();
    let crossed = app.delete(&format!("{base}/{foreign_id}"), Some(&admin)).await;
    assert_eq!(crossed.status, StatusCode::NOT_FOUND);

    let unknown = app.get("/api/targets/999999/agent/event-rules", Some(&admin)).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}
