//! Tests d'intégration des notifications Web Push : clé publique VAPID,
//! abonnements par compte, garde SSRF sur l'adresse fournie par le navigateur,
//! et canal `webpush`.
//!
//! Aucun service de push réel n'est joint : les adresses acceptées sont sous
//! `.invalid`, un domaine qui ne se résout jamais (RFC 2606).

mod common;

use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use serde_json::{Value, json};

use common::TestApp;

/// Abonnement plausible : une vraie clé P-256 et un secret de 16 octets.
fn subscription(endpoint: &str) -> Value {
    let secret = p256::SecretKey::from_slice(&[7u8; 32]).expect("valid scalar");
    let public = secret.public_key().to_encoded_point(false);
    json!({
        "endpoint": endpoint,
        "expirationTime": null,
        "keys": {
            "p256dh": URL_SAFE_NO_PAD.encode(public.as_bytes()),
            "auth": URL_SAFE_NO_PAD.encode([3u8; 16]),
        }
    })
}

#[tokio::test]
async fn la_cle_publique_est_servie_la_cle_privee_jamais() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let first = app.get("/api/webpush", Some(&admin)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let key = first.body["public_key"].as_str().expect("public key");
    assert_eq!(URL_SAFE_NO_PAD.decode(key).expect("base64url").len(), 65);
    assert_eq!(first.body["devices"], json!([]));
    let keys: Vec<&String> = first.body.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["devices", "public_key"], "nothing else, no private key");

    let again = app.get("/api/webpush", Some(&admin)).await;
    assert_eq!(again.body["public_key"], first.body["public_key"], "stable key pair");

    assert_eq!(app.get("/api/webpush", None).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn une_adresse_privee_ou_non_https_est_refusee() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    for endpoint in [
        "http://push.example.invalid/sub/1",
        "https://127.0.0.1:8428/api/v1/admin/tsdb/delete_series",
        "https://[::1]/x",
        "https://10.0.0.1/x",
        "https://169.254.169.254/latest/meta-data/",
        "https://localhost/x",
        "https://nas.lan/x",
    ] {
        let reply =
            app.post("/api/webpush/subscriptions", subscription(endpoint), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{endpoint}: {}", reply.body);
    }

    let mut bad_key = subscription("https://push.example.invalid/sub/1");
    bad_key["keys"]["p256dh"] = json!(URL_SAFE_NO_PAD.encode([4u8; 65]));
    let reply = app.post("/api/webpush/subscriptions", bad_key, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "not a curve point");

    let mut short_auth = subscription("https://push.example.invalid/sub/1");
    short_auth["keys"]["auth"] = json!("AAAA");
    let reply = app.post("/api/webpush/subscriptions", short_auth, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    assert_eq!(app.get("/api/webpush", Some(&admin)).await.body["devices"], json!([]));
}

#[tokio::test]
async fn chacun_gere_ses_propres_appareils() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;

    let endpoint = "https://push.example.invalid/sub/admin-phone";
    let mine = app.post("/api/webpush/subscriptions", subscription(endpoint), Some(&admin)).await;
    assert_eq!(mine.status, StatusCode::CREATED, "{}", mine.body);
    assert_eq!(mine.body["push_service"], "push.example.invalid");
    assert_eq!(mine.body["fingerprint"].as_str().unwrap().len(), 16);
    let text = mine.body.to_string();
    assert!(!text.contains("admin-phone"), "the full endpoint is never returned: {text}");
    assert!(!text.contains("p256dh") && !text.contains("auth\""), "keys are never returned");

    // Le même navigateur qui se réabonne ne crée pas de doublon.
    let again = app.post("/api/webpush/subscriptions", subscription(endpoint), Some(&admin)).await;
    assert_eq!(again.body["id"], mine.body["id"]);

    // Un lecteur abonne son propre appareil : ce n'est pas une écriture d'admin.
    let theirs = app
        .post(
            "/api/webpush/subscriptions",
            subscription("https://push.example.invalid/sub/viewer-laptop"),
            Some(&viewer),
        )
        .await;
    assert_eq!(theirs.status, StatusCode::CREATED, "{}", theirs.body);

    let listed = app.get("/api/webpush", Some(&viewer)).await;
    let devices = listed.body["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 1, "a viewer only sees their own devices");
    assert_eq!(devices[0]["id"], theirs.body["id"]);

    // Ni l'un ni l'autre ne peut retirer l'appareil de l'autre.
    let admin_id = mine.body["id"].as_i64().unwrap();
    let viewer_id = theirs.body["id"].as_i64().unwrap();
    let reply = app.delete(&format!("/api/webpush/subscriptions/{admin_id}"), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = app.delete(&format!("/api/webpush/subscriptions/{viewer_id}"), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(app.get("/api/webpush", Some(&viewer)).await.body["devices"], json!([]));
    assert_eq!(
        app.get("/api/webpush", Some(&admin)).await.body["devices"].as_array().unwrap().len(),
        1
    );
}

#[tokio::test]
async fn le_test_sans_appareil_le_dit_et_un_echec_est_consigne() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply = app.post("/api/webpush/test", json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["ok"], false);
    assert!(reply.body["message"].as_str().unwrap().contains("No device"));

    let endpoint = "https://push.example.invalid/sub/secret-token-xyz";
    let created =
        app.post("/api/webpush/subscriptions", subscription(endpoint), Some(&admin)).await;
    assert_eq!(created.status, StatusCode::CREATED);
    let reply = app.post("/api/webpush/test", json!({}), Some(&admin)).await;
    assert_eq!(reply.body["ok"], false);
    assert_eq!(reply.body["failed"], 1);

    let devices = app.get("/api/webpush", Some(&admin)).await.body["devices"].clone();
    let error = devices[0]["last_error"].as_str().expect("error recorded");
    assert!(!error.contains("secret-token-xyz"), "the endpoint path never leaks: {error}");
}

#[tokio::test]
async fn le_canal_webpush_se_cree_sans_secret_et_explique_l_absence_d_appareil() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let kinds = app.get("/api/notify/kinds", Some(&admin)).await;
    let webpush = kinds.body.as_array().unwrap().iter().find(|k| k["kind"] == "webpush");
    let webpush = webpush.expect("webpush in the catalogue");
    assert_eq!(webpush["secrets"], json!([]));

    let created = app
        .post(
            "/api/notify/channels",
            json!({ "name": "Phones", "kind": "webpush", "settings": { "urgency": "high" } }),
            Some(&admin),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_i64().unwrap();

    let refused = app
        .post(
            "/api/notify/channels",
            json!({ "name": "Bad", "kind": "webpush", "settings": { "urgency": "asap" } }),
            Some(&admin),
        )
        .await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);

    let test = app.post(&format!("/api/notify/channels/{id}/test"), json!({}), Some(&admin)).await;
    assert_eq!(test.status, StatusCode::BAD_REQUEST);
    assert!(
        test.body["error"].as_str().unwrap().contains("no device is subscribed"),
        "{}",
        test.body
    );
}
