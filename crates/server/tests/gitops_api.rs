//! Configuration en fichiers (GitOps) par l'API réelle : plan sans effet,
//! application transactionnelle, objets gérés verrouillés, `prune`, export sans
//! secret et refus des références non résolues.

mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::TestApp;

const SECRET: &str = "jeton-ntfy-tres-secret";

fn file(extra_channel: bool) -> String {
    let mut text = String::from(
        "version: 1\n\
         targets:\n\
         \x20 - kind: http\n\
         \x20   address: https://example.org\n\
         \x20   name: Example\n\
         \x20   interval_secs: 60\n\
         \x20   enabled: true\n\
         \x20   group_name: Web\n\
         channels:\n\
         \x20 - name: Phone\n\
         \x20   kind: ntfy\n\
         \x20   enabled: true\n\
         \x20   settings: {server: 'https://ntfy.example.org', topic: alerts}\n",
    );
    text.push_str(&format!("    secrets: {{token: {SECRET}}}\n"));
    if extra_channel {
        text.push_str("  - name: Backup\n    kind: ntfy\n    enabled: false\n");
        text.push_str("    settings: {server: 'https://ntfy.example.org', topic: backup}\n");
    }
    text
}

async fn post(app: &TestApp, cookie: &str, path: &str, yaml: &str, prune: bool) -> common::Reply {
    app.post(path, json!({ "yaml": yaml, "prune": prune }), Some(cookie)).await
}

#[tokio::test]
async fn a_plan_changes_nothing_and_never_shows_a_secret() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;

    let plan = post(&app, &cookie, "/api/config/plan", &file(false), false).await;
    assert_eq!(plan.status, StatusCode::OK, "{}", plan.body);
    assert_eq!(plan.body["create"], 2);
    assert_eq!(plan.body["applied"], false);
    assert!(!plan.body.to_string().contains(SECRET));

    let targets = app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(targets.body.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn apply_creates_marks_managed_and_is_idempotent() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;

    let first = post(&app, &cookie, "/api/config/apply", &file(false), false).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    assert_eq!(first.body["create"], 2);

    // Deuxième passage : plus rien à faire.
    let second = post(&app, &cookie, "/api/config/plan", &file(false), true).await;
    assert_eq!(second.body["changes"].as_array().map(Vec::len), Some(0), "{}", second.body);

    // L'équipement est « géré » : modification à la main refusée.
    let targets = app.get("/api/targets", Some(&cookie)).await;
    let target = &targets.body[0];
    assert_eq!(target["managed"]["by"], "config-file");
    let id = target["id"].as_i64().expect("id");
    let edit = app
        .put(
            &format!("/api/targets/{id}"),
            json!({ "name": "Renamed", "kind": "http", "address": "https://example.org" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(edit.status, StatusCode::CONFLICT, "{}", edit.body);

    // Le canal aussi.
    let channels = app.get("/api/notify/channels", Some(&cookie)).await;
    let channel_id = channels.body[0]["id"].as_i64().expect("id");
    let delete = app.delete(&format!("/api/notify/channels/{channel_id}"), Some(&cookie)).await;
    assert_eq!(delete.status, StatusCode::CONFLICT, "{}", delete.body);

    // Un export ne contient jamais le secret, seulement une référence.
    let export = app.get("/api/config/export?format=json", Some(&cookie)).await;
    assert_eq!(export.status, StatusCode::OK);
    let text = export.body.to_string();
    assert!(!text.contains(SECRET), "{text}");
    assert!(text.contains("${DUMBMONIT_CHANNEL_PHONE_TOKEN}"), "{text}");
}

#[tokio::test]
async fn prune_removes_only_what_the_file_manages() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;

    // Un canal saisi à la main : jamais touché par prune.
    let by_hand = app
        .post(
            "/api/notify/channels",
            json!({ "name": "Hand", "kind": "ntfy", "enabled": true,
                    "settings": { "server": "https://ntfy.example.org", "topic": "h" },
                    "secrets": {} }),
            Some(&cookie),
        )
        .await;
    assert!(by_hand.status.is_success(), "{}", by_hand.body);

    let applied = post(&app, &cookie, "/api/config/apply", &file(true), false).await;
    assert_eq!(applied.status, StatusCode::OK, "{}", applied.body);

    // Sans le canal « Backup » et avec prune : seul lui disparaît.
    let plan = post(&app, &cookie, "/api/config/plan", &file(false), true).await;
    assert_eq!(plan.body["delete"], 1, "{}", plan.body);
    let pruned = post(&app, &cookie, "/api/config/apply", &file(false), true).await;
    assert_eq!(pruned.status, StatusCode::OK, "{}", pruned.body);

    let channels = app.get("/api/notify/channels", Some(&cookie)).await;
    let names: Vec<&str> =
        channels.body.as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
    assert!(names.contains(&"Hand") && names.contains(&"Phone"), "{names:?}");
    assert!(!names.contains(&"Backup"), "{names:?}");

    // Libérer rend la main sur tout.
    let released = app.post("/api/config/release", json!({}), Some(&cookie)).await;
    assert_eq!(released.status, StatusCode::OK, "{}", released.body);
    let status = app.get("/api/config/status", Some(&cookie)).await;
    assert_eq!(status.body["managed"], json!({}), "{}", status.body);
}

#[tokio::test]
async fn unresolved_references_are_refused_on_apply_but_planned() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;
    // Entre guillemets : dans une table en ligne, `{` ouvrirait une table imbriquée.
    let yaml = file(false).replace(SECRET, "'${NTFY_TOKEN}'");

    let plan = post(&app, &cookie, "/api/config/plan", &yaml, false).await;
    assert_eq!(plan.status, StatusCode::OK, "{}", plan.body);
    assert_eq!(plan.body["unresolved"], json!(["channels Phone"]));

    let apply = post(&app, &cookie, "/api/config/apply", &yaml, false).await;
    assert_eq!(apply.status, StatusCode::BAD_REQUEST, "{}", apply.body);
}

#[tokio::test]
async fn a_broken_file_is_refused_whole_and_viewers_are_refused() {
    let app = TestApp::configured().await;
    let cookie = app.admin_cookie().await;

    // Un parent qui n'existe nulle part : rien n'est appliqué, pas même le reste.
    let broken = json!({
        "version": 1,
        "targets": [{ "kind": "http", "address": "https://b.example.org", "name": "B",
                      "interval_secs": 60, "enabled": true,
                      "parent": "http|https://nowhere.example.org" }],
        "channels": [{ "name": "Phone", "kind": "ntfy", "enabled": true,
                       "settings": { "topic": "x" } }]
    });
    let reply = app.post("/api/config/apply", json!({ "config": broken }), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let channels = app.get("/api/notify/channels", Some(&cookie)).await;
    assert_eq!(channels.body.as_array().map(Vec::len), Some(0), "{}", channels.body);

    let viewer = app.viewer_cookie(&cookie).await;
    let refused = app.get("/api/config/export", Some(&viewer)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);
}
