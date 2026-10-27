//! Tests d'intégration de la connexion SAML 2.0.
//!
//! Les réponses de l'IdP sont des documents signés une fois pour toutes
//! (`tests/fixtures/saml/generate.py`), façon Entra ID, Okta et Keycloak, et
//! leurs variantes hostiles. L'horloge de validation et l'identifiant de la
//! requête sont figés par `Config::saml_hooks` pour que ces documents restent
//! valides : aucun test ne signe quoi que ce soit.

mod common;

use std::time::{Duration, SystemTime};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use common::{PASSWORD, Reply, TestApp, setup_with};
use dumbmonit_server::auth::saml::TestHooks;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

/// 2026-10-26T10:01:00Z : une minute après l'émission des réponses.
const NOW: u64 = 1_793_008_860;
/// 2026-10-26T11:00:00Z : bien après leur expiration.
const LATER: u64 = 1_793_012_400;
const REQUEST_ID: &str = "_dm-test-request-0001";
const SP_ENTITY: &str = "https://monit.example.test/saml";
const ACS: &str = "http://localhost:8080/api/auth/saml/acs";

const ENTRA: &str = "https://sts.windows.net/9188040d-6c67-4c5b-b112-36a304b66dad/";
const OKTA: &str = "http://www.okta.com/exk8dumbmonittest0h7";
const KEYCLOAK: &str = "https://keycloak.example.test/realms/lab";

fn fixture(name: &str) -> String {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saml").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture {}", path.display()))
}

/// Instance configurée (un administrateur), horloge SAML posée à `secs`.
async fn app_at(secs: u64) -> TestApp {
    let app = setup_with(|config| {
        config.saml_hooks = TestHooks {
            now: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
            request_id: Some(REQUEST_ID.to_string()),
        };
    })
    .await;
    app.create_admin().await;
    app
}

/// Enregistre la configuration d'un IdP ; `extra` complète ou remplace.
async fn configure(app: &TestApp, admin: &str, idp: &str, cert: &str, extra: Value) -> Reply {
    let mut body = json!({
        "enabled": true,
        "idp_entity_id": idp,
        "idp_sso_url": "https://idp.example.test/sso",
        "idp_certificates": [fixture(cert)],
        "sp_entity_id": SP_ENTITY,
        "auto_create": true,
        "button_label": "Company SSO",
    });
    for (key, value) in extra.as_object().expect("objet").iter() {
        body[key] = value.clone();
    }
    app.put("/api/auth/saml/config", body, Some(admin)).await
}

async fn configure_ok(app: &TestApp, idp: &str, cert: &str, extra: Value) -> String {
    let admin = app.admin_cookie().await;
    let reply = configure(app, &admin, idp, cert, extra).await;
    assert_eq!(reply.status, StatusCode::OK, "configuration refusée : {}", reply.body);
    admin
}

/// Une requête brute : statut, en-têtes utiles et corps texte.
async fn raw(
    app: &TestApp,
    request: Request<Body>,
) -> (StatusCode, Option<String>, Vec<String>, String) {
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_string)
        .collect();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    (status, location, cookies, String::from_utf8_lossy(&bytes).to_string())
}

/// Ce que le navigateur garde du départ vers l'IdP.
struct Started {
    relay_state: String,
    /// `nom=valeur` du cookie de liaison.
    binding: String,
    location: String,
}

async fn start(app: &TestApp) -> Started {
    let request = Request::get("/api/auth/saml/start").body(Body::empty()).unwrap();
    let (status, location, cookies, body) = raw(app, request).await;
    assert_eq!(status, StatusCode::SEE_OTHER, "départ refusé : {body}");
    let location = location.expect("redirection vers l'IdP");
    let url = reqwest::Url::parse(&location).expect("URL absolue");
    let relay_state = url
        .query_pairs()
        .find(|(name, _)| name == "RelayState")
        .map(|(_, value)| value.into_owned())
        .expect("RelayState");
    let binding = cookies
        .iter()
        .find(|cookie| cookie.starts_with("dumbmonit_saml="))
        .and_then(|cookie| cookie.split(';').next())
        .expect("cookie de liaison")
        .to_string();
    Started { relay_state, binding, location }
}

fn form_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// L'IdP fait poster sa réponse par le navigateur. Rend la redirection.
async fn post_acs(app: &TestApp, xml: &str, relay_state: Option<&str>) -> String {
    let mut body = format!("SAMLResponse={}", form_encode(&STANDARD.encode(xml)));
    if let Some(relay_state) = relay_state {
        body.push_str(&format!("&RelayState={}", form_encode(relay_state)));
    }
    let request = Request::post("/api/auth/saml/acs")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    let (status, location, _, text) = raw(app, request).await;
    assert_eq!(status, StatusCode::SEE_OTHER, "ACS : {text}");
    location.expect("redirection de l'ACS")
}

/// La raison d'un renvoi vers l'écran de connexion, ou `None` si ce n'en est pas un.
fn refusal(location: &str) -> Option<String> {
    location.strip_prefix("/login?error=saml&reason=").map(str::to_string)
}

/// Fin de la connexion : rend le cookie de session, ou la raison du refus.
async fn complete(app: &TestApp, location: &str, binding: Option<&str>) -> Result<String, String> {
    if let Some(reason) = refusal(location) {
        return Err(reason);
    }
    assert!(location.starts_with("/api/auth/saml/complete?ticket="), "{location}");
    let mut request = Request::get(location);
    if let Some(binding) = binding {
        request = request.header(header::COOKIE, binding);
    }
    let (status, next, cookies, body) = raw(app, request.body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::SEE_OTHER, "complete : {body}");
    let next = next.expect("redirection finale");
    if let Some(reason) = refusal(&next) {
        return Err(reason);
    }
    assert_eq!(next, "/");
    let session = cookies
        .iter()
        .find(|cookie| cookie.starts_with("dumbmonit_session="))
        .and_then(|cookie| cookie.split(';').next())
        .expect("cookie de session")
        .to_string();
    assert!(
        cookies.iter().any(|cookie| cookie.starts_with("dumbmonit_saml=;")),
        "le cookie de liaison est effacé : {cookies:?}"
    );
    Ok(session)
}

/// Connexion initiée par DumbMonit, du départ à la session.
async fn sign_in(app: &TestApp, response: &str) -> Result<String, String> {
    let started = start(app).await;
    let location = post_acs(app, &fixture(response), Some(&started.relay_state)).await;
    complete(app, &location, Some(&started.binding)).await
}

async fn me(app: &TestApp, session: &str) -> Value {
    let reply = app.get("/api/auth/me", Some(session)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body
}

async fn usernames(app: &TestApp, admin: &str) -> Vec<String> {
    let reply = app.get("/api/users", Some(admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply
        .body
        .as_array()
        .expect("liste")
        .iter()
        .map(|u| u["username"].as_str().unwrap().to_string())
        .collect()
}

async fn pool(app: &TestApp) -> sqlx::SqlitePool {
    dumbmonit_server::db::open(&app._dir.path().join(dumbmonit_server::db::DATABASE_FILE))
        .await
        .expect("base")
}

// --- Configuration, métadonnées, statut --------------------------------------

#[tokio::test]
async fn the_settings_are_admin_only_and_checked() {
    let app = app_at(NOW).await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get("/api/auth/saml/config", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/auth/saml/config", None).await.status, StatusCode::UNAUTHORIZED);

    let initial = app.get("/api/auth/saml/config", Some(&admin)).await;
    assert_eq!(initial.status, StatusCode::OK, "{}", initial.body);
    assert_eq!(initial.body["enabled"], false);
    assert_eq!(initial.body["allow_idp_initiated"], false);
    assert_eq!(initial.body["auto_create"], false);
    assert_eq!(initial.body["acs_url"], ACS);
    assert_eq!(initial.body["sp_metadata_url"], "http://localhost:8080/api/auth/saml/metadata");

    // Activée sans certificat : refusée, des assertions non vérifiables ne
    // doivent jamais être acceptées.
    let reply =
        configure(&app, &admin, ENTRA, "entra-cert.pem", json!({ "idp_certificates": [] })).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let reply =
        configure(&app, &admin, ENTRA, "entra-cert.pem", json!({ "idp_certificates": ["xyz"] }))
            .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let reply =
        configure(&app, &admin, ENTRA, "entra-cert.pem", json!({ "idp_sso_url": "javascript:x" }))
            .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    let reply = configure(&app, &admin, ENTRA, "entra-cert.pem", json!({})).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["idp_entity_id"], ENTRA);
    assert!(
        reply.body["idp_certificates"][0]
            .as_str()
            .unwrap()
            .starts_with("-----BEGIN CERTIFICATE-----")
    );
    assert_eq!(reply.body["acs_url"], ACS);

    let viewer_put = configure(&app, &viewer, ENTRA, "entra-cert.pem", json!({})).await;
    assert_eq!(viewer_put.status, StatusCode::FORBIDDEN);

    let status = app.get("/api/auth/status", None).await;
    assert_eq!(status.body["saml"]["enabled"], true);
    assert_eq!(status.body["saml"]["label"], "Company SSO");
}

#[tokio::test]
async fn the_service_provider_metadata_is_public_xml() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    let request = Request::get("/api/auth/saml/metadata").body(Body::empty()).unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let kind = response.headers().get(header::CONTENT_TYPE).unwrap().to_str().unwrap().to_string();
    assert!(kind.starts_with("application/samlmetadata+xml"), "{kind}");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8_lossy(&bytes);
    assert!(xml.contains(SP_ENTITY), "{xml}");
    assert!(xml.contains(ACS), "{xml}");
    assert!(xml.contains("WantAssertionsSigned=\"true\""), "{xml}");
}

#[tokio::test]
async fn idp_metadata_is_read_without_being_stored() {
    let app = app_at(NOW).await;
    let admin = app.admin_cookie().await;
    let reply = app
        .post(
            "/api/auth/saml/metadata/import",
            json!({ "xml": fixture("okta-metadata.xml") }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["idp_entity_id"], OKTA);
    assert_eq!(
        reply.body["idp_sso_url"],
        "https://dumbmonit-test.okta.example/app/dumbmonit/exk8dumbmonittest0h7/sso/saml"
    );
    let imported = reply.body["idp_certificates"][0].as_str().unwrap().to_string();
    let expected: String = fixture("okta-cert.pem").split_whitespace().collect();
    assert_eq!(imported.split_whitespace().collect::<String>(), expected);

    let stored = app.get("/api/auth/saml/config", Some(&admin)).await;
    assert_eq!(stored.body["idp_entity_id"], "", "rien n'est enregistré");

    let bad = app
        .post("/api/auth/saml/metadata/import", json!({ "xml": "<not-metadata/>" }), Some(&admin))
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sign_in_is_refused_while_saml_is_off() {
    let app = app_at(NOW).await;
    let request = Request::get("/api/auth/saml/start").body(Body::empty()).unwrap();
    let (status, location, _, _) = raw(&app, request).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(refusal(&location.unwrap()).as_deref(), Some("not_configured"));
    assert_eq!(app.get("/api/auth/status", None).await.body["saml"]["enabled"], false);
}

// --- Connexions réussies, par IdP ---------------------------------------------

#[tokio::test]
async fn entra_id_sign_in_creates_the_account_with_its_group_role() {
    let app = app_at(NOW).await;
    configure_ok(
        &app,
        ENTRA,
        "entra-cert.pem",
        json!({ "operator_groups": ["0b6a1f3e-2c4d-4e5f-8a9b-operators000"] }),
    )
    .await;
    let started = start(&app).await;
    assert!(started.location.starts_with("https://idp.example.test/sso?"), "{}", started.location);
    assert!(started.location.contains("SAMLRequest="), "{}", started.location);
    let location = post_acs(&app, &fixture("entra-response.xml"), Some(&started.relay_state)).await;
    let session = complete(&app, &location, Some(&started.binding)).await.expect("connexion");
    let user = me(&app, &session).await;
    assert_eq!(user["username"], "alice@contoso.example");
    assert_eq!(user["display_name"], "Alice Martin");
    assert_eq!(user["role"], "operator");
    assert_eq!(user["auth"], "saml");
}

#[tokio::test]
async fn okta_sign_in_with_signed_response_and_assertion() {
    let app = app_at(NOW).await;
    configure_ok(
        &app,
        OKTA,
        "okta-cert.pem",
        json!({ "operator_groups": ["dumbmonit-operators"] }),
    )
    .await;
    let session = sign_in(&app, "okta-response.xml").await.expect("connexion");
    let user = me(&app, &session).await;
    assert_eq!(user["username"], "bob@okta.example");
    assert_eq!(user["display_name"], "Bob Durand");
    assert_eq!(user["role"], "operator");
}

#[tokio::test]
async fn keycloak_groups_give_the_role_and_the_teams() {
    let app = app_at(NOW).await;
    let db = pool(&app).await;
    sqlx::query(
        "INSERT INTO teams (name, idp_groups) VALUES ('ops', '[\"/ops\"]'), ('dev', '[\"/dev\"]')",
    )
    .execute(&db)
    .await
    .unwrap();
    configure_ok(
        &app,
        KEYCLOAK,
        "keycloak-cert.pem",
        json!({ "admin_groups": ["/dumbmonit-admins"] }),
    )
    .await;
    let session = sign_in(&app, "keycloak-response.xml").await.expect("connexion");
    let user = me(&app, &session).await;
    assert_eq!(user["username"], "carol@kc.example");
    assert_eq!(user["role"], "admin");

    let teams: Vec<(String, String)> = sqlx::query_as(
        "SELECT t.name, m.source FROM team_members m JOIN teams t ON t.id = m.team_id
         JOIN users u ON u.id = m.user_id WHERE u.username = 'carol@kc.example'",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(teams, vec![("ops".to_string(), "idp".to_string())]);

    // L'identité persistante est enregistrée : c'est elle qui retrouvera le compte.
    let (subject, issuer): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT saml_subject, saml_issuer FROM users WHERE username = 'carol@kc.example'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(subject.as_deref(), Some("G-1c2d3e4f-5a6b-7c8d-9e0f-carolkeycloak"));
    assert_eq!(issuer.as_deref(), Some(KEYCLOAK));
}

/// Keycloak, réglages par défaut : seule la réponse est signée. Sa signature,
/// posée sur la racine, couvre l'assertion.
#[tokio::test]
async fn a_response_signed_only_at_the_root_is_accepted() {
    let app = app_at(NOW).await;
    configure_ok(&app, KEYCLOAK, "keycloak-cert.pem", json!({})).await;
    let session = sign_in(&app, "keycloak-document-signed-response.xml").await.expect("connexion");
    let user = me(&app, &session).await;
    assert_eq!(user["username"], "carol@kc.example");
    assert_eq!(user["role"], "viewer");
}

// --- Réponses refusées ----------------------------------------------------------

/// Chaque document hostile est refusé, et personne n'est connecté ni créé.
#[tokio::test]
async fn forged_or_mismatched_responses_are_refused() {
    let cases = [
        ("unsigned-response.xml", "assertion non signée"),
        ("tampered-response.xml", "assertion modifiée après signature"),
        ("rogue-key-response.xml", "clé non déclarée"),
        ("xsw-sibling-response.xml", "XSW : assertion non signée à côté"),
        ("xsw-advice-response.xml", "XSW : assertion signée enveloppée"),
        ("wrong-audience-response.xml", "autre audience"),
        ("wrong-destination-response.xml", "autre destination"),
        ("unknown-request-response.xml", "InResponseTo inconnu"),
        ("sha1-response.xml", "RSA-SHA1"),
    ];
    for (file, what) in cases {
        let app = app_at(NOW).await;
        let admin = configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
        let refused = sign_in(&app, file).await;
        assert_eq!(refused, Err("invalid_response".to_string()), "{what} ({file})");
        assert_eq!(usernames(&app, &admin).await, vec!["admin".to_string()], "{what}");
    }
}

/// XSW sur la réponse : la réponse signée de Carol, intacte, est enveloppée
/// dans une réponse non signée qui porte l'assertion de Mallory.
#[tokio::test]
async fn a_signed_response_wrapped_in_another_is_refused() {
    let app = app_at(NOW).await;
    let admin = configure_ok(&app, KEYCLOAK, "keycloak-cert.pem", json!({})).await;
    let refused = sign_in(&app, "xsw-response-wrap-response.xml").await;
    assert_eq!(refused, Err("invalid_response".to_string()));
    assert_eq!(usernames(&app, &admin).await, vec!["admin".to_string()]);
}

/// Avant `NotBefore` (09:55), tolérance de deux minutes comprise : refusée.
#[tokio::test]
async fn an_assertion_not_yet_valid_is_refused() {
    // 09:50:00Z.
    let app = app_at(NOW - 11 * 60).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    assert_eq!(sign_in(&app, "entra-response.xml").await, Err("expired".to_string()));
}

/// Quelques secondes d'écart d'horloge avec l'IdP restent dans la tolérance.
#[tokio::test]
async fn a_small_clock_skew_is_tolerated() {
    // 09:59:45Z : la réponse semble émise quinze secondes dans le futur.
    let app = app_at(NOW - 75).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    assert!(sign_in(&app, "entra-response.xml").await.is_ok());
}

#[tokio::test]
async fn a_doctype_is_refused() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    let body = fixture("entra-response.xml");
    let body = body.split_once("?>").map(|(_, rest)| rest).unwrap();
    let hostile = format!("<?xml version=\"1.0\"?><!DOCTYPE r [<!ENTITY x \"y\">]>{body}");
    let started = start(&app).await;
    let location = post_acs(&app, &hostile, Some(&started.relay_state)).await;
    assert_eq!(refusal(&location).as_deref(), Some("invalid_response"));
}

#[tokio::test]
async fn an_expired_assertion_is_refused() {
    let app = app_at(LATER).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    assert_eq!(sign_in(&app, "entra-response.xml").await, Err("expired".to_string()));
}

#[tokio::test]
async fn an_assertion_is_accepted_only_once() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    assert!(sign_in(&app, "entra-response.xml").await.is_ok());
    assert_eq!(sign_in(&app, "entra-response.xml").await, Err("replay".to_string()));
}

#[tokio::test]
async fn a_relay_state_serves_once() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    let started = start(&app).await;
    let first = post_acs(&app, &fixture("entra-response.xml"), Some(&started.relay_state)).await;
    assert!(refusal(&first).is_none());
    let again = post_acs(&app, &fixture("okta-response.xml"), Some(&started.relay_state)).await;
    assert_eq!(refusal(&again).as_deref(), Some("state"));
    let unknown = post_acs(&app, &fixture("entra-response.xml"), Some("not-a-relay-state")).await;
    assert_eq!(refusal(&unknown).as_deref(), Some("state"));
}

/// Sans le cookie posé au départ, la réponse ne connecte personne : c'est ce
/// qui empêche de faire poster à une victime une réponse obtenue ailleurs.
#[tokio::test]
async fn the_session_opens_only_in_the_browser_that_started() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    let started = start(&app).await;
    let location = post_acs(&app, &fixture("entra-response.xml"), Some(&started.relay_state)).await;
    assert_eq!(complete(&app, &location, None).await, Err("state".to_string()));
    // Le ticket a servi : le bon cookie n'y change plus rien.
    assert_eq!(complete(&app, &location, Some(&started.binding)).await, Err("state".to_string()));

    let other = start(&app).await;
    let location = post_acs(&app, &fixture("okta-response.xml"), Some(&other.relay_state)).await;
    // (L'IdP configuré est Entra : la réponse Okta est refusée dès l'ACS.)
    assert_eq!(refusal(&location).as_deref(), Some("invalid_response"));
}

// --- Initiée par l'IdP --------------------------------------------------------

#[tokio::test]
async fn idp_initiated_sign_in_is_off_by_default() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    let location = post_acs(&app, &fixture("idp-initiated-response.xml"), None).await;
    assert_eq!(refusal(&location).as_deref(), Some("unsolicited"));
}

#[tokio::test]
async fn idp_initiated_sign_in_when_allowed_refuses_in_response_to() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({ "allow_idp_initiated": true })).await;
    // Une réponse qui prétend répondre à une requête n'est pas une réponse
    // spontanée.
    let location = post_acs(&app, &fixture("unknown-request-response.xml"), None).await;
    assert_eq!(refusal(&location).as_deref(), Some("invalid_response"));

    let location = post_acs(&app, &fixture("idp-initiated-response.xml"), None).await;
    let session = complete(&app, &location, None).await.expect("connexion initiée par l'IdP");
    assert_eq!(me(&app, &session).await["username"], "alice@contoso.example");
}

// --- Comptes --------------------------------------------------------------------

#[tokio::test]
async fn without_auto_create_an_unknown_person_is_refused() {
    let app = app_at(NOW).await;
    configure_ok(&app, ENTRA, "entra-cert.pem", json!({ "auto_create": false })).await;
    assert_eq!(sign_in(&app, "entra-response.xml").await, Err("no_account".to_string()));
}

#[tokio::test]
async fn an_existing_account_is_linked_by_email() {
    let app = app_at(NOW).await;
    let admin = configure_ok(&app, ENTRA, "entra-cert.pem", json!({ "auto_create": false })).await;
    let reply = app
        .post(
            "/api/users",
            json!({ "username": "alice@contoso.example", "role": "viewer", "password": PASSWORD }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let session = sign_in(&app, "entra-response.xml").await.expect("connexion");
    let user = me(&app, &session).await;
    assert_eq!(user["username"], "alice@contoso.example");
    assert_eq!(user["id"], reply.body["id"]);
}

#[tokio::test]
async fn a_disabled_account_is_refused() {
    let app = app_at(NOW).await;
    let admin = configure_ok(&app, ENTRA, "entra-cert.pem", json!({ "auto_create": false })).await;
    let reply = app
        .post(
            "/api/users",
            json!({ "username": "alice@contoso.example", "role": "viewer", "password": PASSWORD }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let db = pool(&app).await;
    sqlx::query("UPDATE users SET disabled = 1 WHERE username = 'alice@contoso.example'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(sign_in(&app, "entra-response.xml").await, Err("disabled".to_string()));
}

#[tokio::test]
async fn a_local_admin_with_a_password_is_never_taken_over() {
    let app = app_at(NOW).await;
    let admin = configure_ok(&app, ENTRA, "entra-cert.pem", json!({})).await;
    let reply = app
        .post(
            "/api/users",
            json!({ "username": "alice@contoso.example", "role": "admin", "password": PASSWORD }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let session = sign_in(&app, "entra-response.xml").await.expect("connexion");
    let user = me(&app, &session).await;
    assert_eq!(user["username"], "alice@contoso.example-1");
    assert_eq!(user["role"], "viewer");
}
