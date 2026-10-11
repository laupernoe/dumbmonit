//! Régressions de l'audit de sécurité d'octobre 2026 : ce qu'un lecteur (ou un
//! jeton `read`) ne doit plus lire, et les limites posées aux gestes coûteux.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::TestApp;
use dumbmonit_server::db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const HEADER_SECRET: &str = "cle-api-tres-secrete";

async fn admin_and_viewer() -> (TestApp, String, String) {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    (app, admin, viewer)
}

// --- M1 : en-têtes du webhook personnalisé ---------------------------------

#[tokio::test]
async fn les_en_tetes_du_webhook_sont_des_secrets_jamais_relus() {
    let (app, admin, viewer) = admin_and_viewer().await;

    // Dans les réglages, renvoyés en clair : refusés.
    let refused = app
        .post(
            "/api/notify/channels",
            json!({
                "name": "Jira",
                "kind": "webhook",
                "settings": { "headers": { "X-Api-Key": HEADER_SECRET } },
                "secrets": { "url": "https://jira.example.test/hook" }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);
    assert!(!refused.body.to_string().contains(HEADER_SECRET));

    // Parmi les secrets, en texte JSON comme le formulaire l'envoie : acceptés.
    let created = app
        .post(
            "/api/notify/channels",
            json!({
                "name": "Jira",
                "kind": "webhook",
                "settings": { "method": "POST" },
                "secrets": {
                    "url": "https://jira.example.test/hook",
                    "headers": format!("{{\"X-Api-Key\": \"{HEADER_SECRET}\"}}")
                }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["has_secret"], true);
    let id = created.body["id"].as_i64().unwrap();

    // Un JSON invalide est refusé avant d'être enregistré.
    let invalid = app
        .post(
            "/api/notify/channels",
            json!({
                "name": "Bad",
                "kind": "webhook",
                "secrets": { "url": "https://x.example.test/", "headers": "X-Api-Key: oops" }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST, "{}", invalid.body);

    for cookie in [&admin, &viewer] {
        let listed = app.get("/api/notify/channels", Some(cookie)).await;
        assert_eq!(listed.status, StatusCode::OK);
        assert!(!listed.body.to_string().contains(HEADER_SECRET), "{}", listed.body);
    }

    // Renommer sans renvoyer les secrets les conserve.
    let renamed = app
        .put(
            &format!("/api/notify/channels/{id}"),
            json!({ "name": "Jira (prod)", "kind": "webhook", "settings": { "method": "POST" } }),
            Some(&admin),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
    assert_eq!(renamed.body["has_secret"], true);

    // Le catalogue range le champ parmi les secrets.
    let kinds = app.get("/api/notify/kinds", Some(&admin)).await;
    let webhook = kinds.body.as_array().unwrap().iter().find(|k| k["kind"] == "webhook").unwrap();
    let in_secrets =
        webhook["secrets"].as_array().unwrap().iter().any(|field| field["key"] == "headers");
    let in_settings =
        webhook["settings"].as_array().unwrap().iter().any(|field| field["key"] == "headers");
    assert!(in_secrets && !in_settings, "{webhook}");
}

#[tokio::test]
async fn les_anciens_en_tetes_en_clair_migrent_vers_les_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.unwrap();
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.unwrap();

    sqlx::query(
        "INSERT INTO notification_channels (name, kind, enabled, settings) VALUES (?, ?, 1, ?)",
    )
    .bind("Ancien")
    .bind("webhook")
    .bind(
        json!({ "url": "https://ancien.example.test/hook", "headers": { "X-Api-Key": HEADER_SECRET } })
            .to_string(),
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(db::alerts::lift_legacy_webhook_headers(&pool, &cipher).await.unwrap(), 1);
    assert_eq!(db::alerts::lift_legacy_webhook_headers(&pool, &cipher).await.unwrap(), 0);

    let channel = db::alerts::list_channels(&pool, &cipher).await.unwrap().remove(0);
    assert!(channel.settings.get("headers").is_none(), "{}", channel.settings);
    assert_eq!(channel.secrets["headers"]["X-Api-Key"], HEADER_SECRET);
    assert_eq!(channel.headers().unwrap(), vec![("X-Api-Key".into(), HEADER_SECRET.into())]);

    let raw: String = sqlx::query_scalar("SELECT settings FROM notification_channels")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!raw.contains(HEADER_SECRET), "nothing left in clear text: {raw}");

    let app = common::build(dir, config, pool).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    let listed = app.get("/api/notify/channels", Some(&admin)).await;
    assert!(!listed.body.to_string().contains(HEADER_SECRET), "{}", listed.body);
    assert_eq!(listed.body[0]["has_secret"], true);
}

// --- M2 et L8 : listes réservées aux administrateurs -----------------------

#[tokio::test]
async fn les_abonnes_et_les_jetons_ne_se_listent_qu_en_administrateur() {
    let (app, admin, viewer) = admin_and_viewer().await;
    let page = app
        .post("/api/status-pages", json!({ "title": "Public", "slug": "public" }), Some(&admin))
        .await;
    assert_eq!(page.status, StatusCode::CREATED, "{}", page.body);
    let id = page.body["id"].as_i64().unwrap();
    let subscribers = format!("/api/status-pages/{id}/subscribers");

    for uri in [subscribers.as_str(), "/api/tokens", "/api/agent/tokens"] {
        let reply = app.get(uri, Some(&viewer)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{uri}: {}", reply.body);
        let reply = app.get(uri, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri}: {}", reply.body);
    }
}

// --- L2 : Web Push ----------------------------------------------------------

/// Abonnement plausible : une vraie clé P-256 et un secret de 16 octets.
fn subscription(endpoint: &str) -> Value {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use p256::elliptic_curve::sec1::ToEncodedPoint;
    let secret = p256::SecretKey::from_slice(&[7u8; 32]).expect("valid scalar");
    let public = secret.public_key().to_encoded_point(false);
    json!({
        "endpoint": endpoint,
        "keys": {
            "p256dh": URL_SAFE_NO_PAD.encode(public.as_bytes()),
            "auth": URL_SAFE_NO_PAD.encode([3u8; 16]),
        }
    })
}

#[tokio::test]
async fn un_compte_a_un_nombre_borne_d_appareils_et_de_tests() {
    let (app, _admin, viewer) = admin_and_viewer().await;
    for n in 0..10 {
        let reply = app
            .post(
                "/api/webpush/subscriptions",
                subscription(&format!("https://push.example.invalid/sub/{n}")),
                Some(&viewer),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{n}: {}", reply.body);
    }
    let over = app
        .post(
            "/api/webpush/subscriptions",
            subscription("https://push.example.invalid/sub/eleventh"),
            Some(&viewer),
        )
        .await;
    assert_eq!(over.status, StatusCode::CONFLICT, "{}", over.body);
    // Le même navigateur qui se réabonne reste admis.
    let again = app
        .post(
            "/api/webpush/subscriptions",
            subscription("https://push.example.invalid/sub/3"),
            Some(&viewer),
        )
        .await;
    assert_eq!(again.status, StatusCode::CREATED, "{}", again.body);

    let first = app.post("/api/webpush/test", json!({}), Some(&viewer)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let second = app.post("/api/webpush/test", json!({}), Some(&viewer)).await;
    assert_eq!(second.status, StatusCode::TOO_MANY_REQUESTS, "{}", second.body);
}

// --- L3 : options et adresses des cibles ------------------------------------

#[tokio::test]
async fn un_lecteur_ne_lit_ni_les_options_secretes_ni_le_mot_de_passe_d_une_url() {
    let (app, admin, viewer) = admin_and_viewer().await;
    let created = app
        .post(
            "/api/targets",
            json!({
                "name": "API",
                "kind": "dummy",
                "address": "https://bob:hunter2-secret@api.example.test/health",
                "interval_secs": 60,
                "tags": {
                    "api_key": "valeur-tres-secrete",
                    "headers": "Authorization: Bearer jeton-tres-secret",
                    "mirror": "https://alice:autre-secret@miroir.example.test/",
                    "site": "cellar"
                }
            }),
            Some(&admin),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_i64().unwrap();

    for uri in ["/api/targets".to_string(), format!("/api/targets/{id}")] {
        let seen = app.get(&uri, Some(&viewer)).await;
        let text = seen.body.to_string();
        for secret in ["hunter2-secret", "valeur-tres-secrete", "jeton-tres-secret", "autre-secret"]
        {
            assert!(!text.contains(secret), "{uri} leaks {secret}: {text}");
        }
        assert!(text.contains("cellar"), "ordinary tags stay readable: {text}");
        assert!(text.contains("bob:***@api.example.test"), "{text}");

        // L'administrateur, qui peut modifier la cible, voit les vraies valeurs.
        let full = app.get(&uri, Some(&admin)).await.body.to_string();
        assert!(full.contains("hunter2-secret") && full.contains("valeur-tres-secrete"));
    }
}

// --- L4 : origine des liens publics -----------------------------------------

#[tokio::test]
async fn un_client_ne_choisit_pas_le_lien_du_flux_rss() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let page = app
        .post(
            "/api/status-pages",
            json!({ "title": "Public", "slug": "flux", "published": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(page.status, StatusCode::CREATED, "{}", page.body);

    let request = Request::builder()
        .uri("/api/public/status/flux/rss")
        .header(header::HOST, "status.example.test")
        .header("x-forwarded-host", "evil.example")
        .header("x-forwarded-proto", "https")
        .body(Body::empty())
        .unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let feed = String::from_utf8_lossy(&body);
    assert!(!feed.contains("evil.example"), "{feed}");
    assert!(feed.contains("http://status.example.test/s/flux"), "{feed}");
}

// --- L5 : cookie `Secure` en HTTPS -------------------------------------------

async fn login_cookie(app: &TestApp, origin: Option<&str>) -> String {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    let body = json!({ "username": "admin", "password": common::PASSWORD }).to_string();
    let response = app.router.clone().oneshot(builder.body(Body::from(body)).unwrap()).await;
    let response = response.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    response.headers()[header::SET_COOKIE].to_str().unwrap().to_string()
}

#[tokio::test]
async fn le_cookie_de_session_est_secure_quand_le_navigateur_est_en_https() {
    let app = TestApp::configured().await;
    let https = login_cookie(&app, Some("https://dumbmonit.example.test")).await;
    assert!(https.contains("; Secure"), "{https}");
    let plain = login_cookie(&app, Some("http://192.168.1.5:8080")).await;
    assert!(!plain.contains("Secure"), "plain HTTP on a LAN must keep working: {plain}");
    let none = login_cookie(&app, None).await;
    assert!(!none.contains("Secure"), "{none}");
}

// --- L6 : santé publique sans détail ------------------------------------------

#[tokio::test]
async fn la_sante_publique_ne_recopie_pas_les_erreurs() {
    let app = TestApp::configured().await;
    let anonymous = app.get("/api/health", None).await;
    assert_eq!(anonymous.body["victoria"]["ok"], false);
    let error = anonymous.body["victoria"]["error"].as_str().unwrap();
    assert!(!error.contains("127.0.0.1"), "no raw dependency error in public: {error}");

    let admin = app.admin_cookie().await;
    let signed_in = app.get("/api/health", Some(&admin)).await;
    let detail = signed_in.body["victoria"]["error"].as_str().unwrap();
    assert_ne!(detail, error, "a session gets the actual cause");
}
