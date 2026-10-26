//! Tests d'intégration des astreintes : plannings, remplacements, politiques,
//! canal `oncall`, contacts et heures de chacun, lien de calendrier.

mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};

use common::TestApp;

async fn user_id(app: &TestApp, admin: &str, name: &str) -> i64 {
    let people = app.get("/api/oncall/people", Some(admin)).await;
    assert_eq!(people.status, StatusCode::OK, "{}", people.body);
    people.body.as_array().unwrap().iter().find(|p| p["username"] == name).unwrap()["id"]
        .as_i64()
        .unwrap()
}

fn schedule(admin_id: i64, viewer_id: i64) -> Value {
    json!({
        "name": "Ops",
        "timezone": "Europe/Paris",
        "handoff_notifications": true,
        "layers": [{
            "name": "Base",
            "rotation": { "kind": "weekly", "start": "2026-01-05T09:00" },
            "participants": [admin_id, viewer_id]
        }]
    })
}

#[tokio::test]
async fn un_planning_une_politique_et_un_canal_d_astreinte_se_tiennent() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    let admin_id = user_id(&app, &admin, "admin").await;
    let viewer_id = user_id(&app, &admin, "viewer").await;

    // Un lecteur lit, n'écrit pas.
    let refused =
        app.post("/api/oncall/schedules", schedule(admin_id, viewer_id), Some(&viewer)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);

    let bad = app
        .post(
            "/api/oncall/schedules",
            json!({"name": "X", "timezone": "Mars/Base", "layers": []}),
            Some(&admin),
        )
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.body);

    let created =
        app.post("/api/oncall/schedules", schedule(admin_id, viewer_id), Some(&admin)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let schedule_id = created.body["id"].as_i64().unwrap();
    assert!(created.body["now"]["user_id"].is_i64(), "someone is on call: {}", created.body);
    assert_eq!(created.body["layers"][0]["rotation"]["start"], "2026-01-05T09:00");

    let duplicate =
        app.post("/api/oncall/schedules", schedule(admin_id, viewer_id), Some(&admin)).await;
    assert_eq!(duplicate.status, StatusCode::CONFLICT);

    let shifts = app
        .get(
            &format!(
                "/api/oncall/schedules/{schedule_id}/shifts?from=2026-01-05T00:00:00Z&to=2026-01-19T12:00:00Z"
            ),
            Some(&viewer),
        )
        .await;
    assert_eq!(shifts.status, StatusCode::OK, "{}", shifts.body);
    let users: Vec<i64> = shifts.body["final"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["user_id"].as_i64().unwrap())
        .collect();
    assert_eq!(users, vec![admin_id, viewer_id, admin_id], "weekly turns");
    assert_eq!(shifts.body["final"][0]["start"], "2026-01-05T08:00:00Z", "09:00 CET");
    assert_eq!(shifts.body["layers"].as_array().unwrap().len(), 1);

    // Remplacement : un lecteur ne peut pas, l'administrateur oui.
    let cover = json!({
        "user_id": viewer_id,
        "replaces": admin_id,
        "starts_at": "2026-01-06T00:00:00Z",
        "ends_at": "2026-01-07T00:00:00Z",
        "note": "dentist"
    });
    let path = format!("/api/oncall/schedules/{schedule_id}/overrides");
    assert_eq!(app.post(&path, cover.clone(), Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    let made = app.post(&path, cover, Some(&admin)).await;
    assert_eq!(made.status, StatusCode::CREATED, "{}", made.body);
    assert_eq!(made.body["created_by"], "admin");
    let shifts = app
        .get(
            &format!(
                "/api/oncall/schedules/{schedule_id}/shifts?from=2026-01-05T00:00:00Z&to=2026-01-08T00:00:00Z"
            ),
            Some(&admin),
        )
        .await;
    let covered: Vec<(i64, bool)> = shifts.body["final"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["user_id"].as_i64().unwrap(), s["override_id"].is_i64()))
        .collect();
    assert_eq!(covered, vec![(admin_id, false), (viewer_id, true), (admin_id, false)]);

    // Politique : une cible inconnue est refusée, puis la vraie.
    let policy = json!({
        "name": "Default",
        "repeat": 1,
        "steps": [
            { "delay_minutes": 10, "targets": [{ "type": "schedule", "id": schedule_id }] },
            { "delay_minutes": 15, "targets": [{ "type": "user", "id": 999 }] }
        ]
    });
    let bad = app.post("/api/oncall/policies", policy.clone(), Some(&admin)).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.body);
    let mut policy = policy;
    policy["steps"][1]["targets"][0]["id"] = json!(admin_id);
    let created = app.post("/api/oncall/policies", policy, Some(&admin)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let policy_id = created.body["id"].as_i64().unwrap();

    // Le canal d'astreinte : sans politique valide, refusé.
    let kinds = app.get("/api/notify/kinds", Some(&admin)).await;
    let oncall = kinds.body.as_array().unwrap().iter().find(|k| k["kind"] == "oncall").unwrap();
    assert_eq!(oncall["settings"][0]["lookup"], "oncall_policies");
    assert_eq!(oncall["actions"], "none");
    let missing = app
        .post(
            "/api/notify/channels",
            json!({"name": "Pager", "kind": "oncall", "settings": {"policy_id": 4242}}),
            Some(&admin),
        )
        .await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST, "{}", missing.body);
    let channel = app
        .post(
            "/api/notify/channels",
            json!({"name": "Pager", "kind": "oncall", "settings": {"policy_id": policy_id}}),
            Some(&admin),
        )
        .await;
    assert_eq!(channel.status, StatusCode::CREATED, "{}", channel.body);
    let channel_id = channel.body["id"].as_i64().unwrap();

    let policies = app.get("/api/oncall/policies", Some(&viewer)).await;
    assert_eq!(policies.body[0]["channels"], json!([channel_id]));

    // Les liens se protègent : ni politique utilisée, ni planning appelé.
    let busy = app.delete(&format!("/api/oncall/policies/{policy_id}"), Some(&admin)).await;
    assert_eq!(busy.status, StatusCode::CONFLICT, "{}", busy.body);
    let busy = app.delete(&format!("/api/oncall/schedules/{schedule_id}"), Some(&admin)).await;
    assert_eq!(busy.status, StatusCode::CONFLICT, "{}", busy.body);

    let now = app.get("/api/oncall/now", Some(&viewer)).await;
    assert_eq!(now.status, StatusCode::OK);
    assert_eq!(now.body[0]["schedule_name"], "Ops");

    let log = app.get("/api/oncall/escalations?status=active", Some(&viewer)).await;
    assert_eq!(log.status, StatusCode::OK, "{}", log.body);
    assert_eq!(log.body, json!([]));
}

#[tokio::test]
async fn chacun_gere_ses_contacts_et_ses_heures_sans_voir_ceux_des_autres() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    // Canal d'équipe courriel : un contact n'y donne que son adresse.
    let smtp = app
        .post(
            "/api/notify/channels",
            json!({
                "name": "Team mail",
                "kind": "smtp",
                "settings": {"host": "smtp.example.invalid", "from": "dm@example.org", "to": ["ops@example.org"]}
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(smtp.status, StatusCode::CREATED, "{}", smtp.body);
    let smtp_id = smtp.body["id"].as_i64().unwrap();

    let options = app.get("/api/account/contacts/options", Some(&viewer)).await;
    assert_eq!(options.status, StatusCode::OK, "{}", options.body);
    assert_eq!(options.body["team_channels"][0]["recipient"], "email");
    assert!(options.body["kinds"].as_array().unwrap().contains(&json!("ntfy")));
    assert!(!options.body["kinds"].as_array().unwrap().contains(&json!("webhook")));

    let bad = app
        .post(
            "/api/account/contacts",
            json!({"label": "Mail", "via_channel_id": smtp_id, "address": "not-an-address"}),
            Some(&viewer),
        )
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.body);
    let mail = app
        .post(
            "/api/account/contacts",
            json!({"label": "Mail", "via_channel_id": smtp_id, "address": "viewer@example.org", "min_severity": "critical"}),
            Some(&viewer),
        )
        .await;
    assert_eq!(mail.status, StatusCode::CREATED, "{}", mail.body);
    assert_eq!(mail.body["kind"], "smtp");
    assert_eq!(mail.body["via_channel_name"], "Team mail");
    assert_eq!(mail.body["min_severity"], "critical");

    let webhook = app
        .post(
            "/api/account/contacts",
            json!({"label": "Hook", "kind": "webhook", "settings": {}}),
            Some(&viewer),
        )
        .await;
    assert_eq!(webhook.status, StatusCode::BAD_REQUEST, "no arbitrary requests: {}", webhook.body);

    let phone = app
        .post(
            "/api/account/contacts",
            json!({
                "label": "Phone",
                "kind": "ntfy",
                "settings": {"topic": "viewer-oncall"},
                "secrets": {"token": "tk_secret"}
            }),
            Some(&viewer),
        )
        .await;
    assert_eq!(phone.status, StatusCode::CREATED, "{}", phone.body);
    assert_eq!(phone.body["has_secret"], true);
    assert!(!phone.body.to_string().contains("tk_secret"), "secrets never come back");
    let phone_id = phone.body["id"].as_i64().unwrap();

    // Renommer sans ressaisir le secret le garde.
    let renamed = app
        .put(
            &format!("/api/account/contacts/{phone_id}"),
            json!({"label": "Pixel", "kind": "ntfy", "settings": {"topic": "viewer-oncall"}}),
            Some(&viewer),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
    assert_eq!(renamed.body["has_secret"], true);

    // L'administrateur ne voit pas les contacts du lecteur par cette route.
    let mine = app.get("/api/account/contacts", Some(&admin)).await;
    assert_eq!(mine.body, json!([]));
    let foreign = app.delete(&format!("/api/account/contacts/{phone_id}"), Some(&admin)).await;
    assert_eq!(foreign.status, StatusCode::NOT_FOUND);

    let quiet = json!({
        "timezone": "Europe/Paris",
        "quiet_hours": {
            "kind": "weekly", "days": [0,1,2,3,4,5,6], "start_minute": 1320, "end_minute": 420,
            "timezone": "Europe/Paris"
        }
    });
    let profile = app.put("/api/account/oncall", quiet, Some(&viewer)).await;
    assert_eq!(profile.status, StatusCode::OK, "{}", profile.body);
    assert_eq!(profile.body["timezone"], "Europe/Paris");
    assert_eq!(profile.body["quiet_hours"]["start_minute"], 1320);
    assert_eq!(profile.body["calendar"], false);
    let cleared = app.put("/api/account/oncall", json!({"quiet_hours": null}), Some(&viewer)).await;
    assert!(cleared.body["quiet_hours"].is_null());
    assert_eq!(cleared.body["timezone"], "Europe/Paris", "kept when omitted");

    // Lien de calendrier : servi sans session, révocable.
    let link = app.post("/api/account/oncall/calendar", json!({}), Some(&viewer)).await;
    assert_eq!(link.status, StatusCode::OK, "{}", link.body);
    let url = link.body["url"].as_str().unwrap().to_string();
    assert!(url.starts_with("/api/public/oncall/calendar/") && url.ends_with(".ics"), "{url}");
    let feed = app.get(&url, None).await;
    assert_eq!(feed.status, StatusCode::OK);
    assert_eq!(
        app.delete("/api/account/oncall/calendar", Some(&viewer)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(app.get(&url, None).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn la_liste_des_personnes_cache_les_donnees_personnelles_aux_autres() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let seen_by_admin = app.get("/api/oncall/people", Some(&admin)).await;
    assert_eq!(seen_by_admin.status, StatusCode::OK, "{}", seen_by_admin.body);
    for person in seen_by_admin.body.as_array().unwrap() {
        assert!(person["username"].is_string() && person["role"].is_string());
        assert!(person["contacts"].is_u64());
    }

    let seen_by_viewer = app.get("/api/oncall/people", Some(&viewer)).await;
    assert_eq!(seen_by_viewer.status, StatusCode::OK, "{}", seen_by_viewer.body);
    let list = seen_by_viewer.body.as_array().unwrap();
    assert!(list.len() >= 2);
    let mut own = 0;
    for person in list {
        assert!(person["display_name"].is_string() && person["on_call"].is_boolean());
        if person.get("username").is_some() {
            // Seule la fiche de l'appelant est complète.
            own += 1;
            assert_eq!(person["username"], "viewer");
        } else {
            assert!(person.get("role").is_none() && person.get("contacts").is_none());
        }
    }
    assert_eq!(own, 1, "only the caller's own record is complete: {list:?}");
}
