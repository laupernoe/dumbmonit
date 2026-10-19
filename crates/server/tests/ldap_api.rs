//! Réglages et connexion LDAP, sans annuaire réel : le mot de passe de service
//! ne fait jamais l'aller-retour, le texte clair est refusé, l'administrateur
//! local entre toujours quand l'annuaire est injoignable, et le bouton
//! « Tester » n'envoie le mot de passe enregistré qu'au serveur pour lequel il
//! a été enregistré.
//!
//! Le test contre un vrai OpenLDAP est dans `ldap_directory.rs`.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp, setup_with};
use dumbmonit_server::auth::ldap::{LdapConfig, LdapEnv};
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

const SERVICE_PASSWORD: &str = "mot-de-passe-du-compte-de-service";

async fn configured(adjust: impl FnOnce(&mut dumbmonit_server::config::Config)) -> TestApp {
    let app = setup_with(adjust).await;
    app.create_admin().await;
    app
}

fn form(url: &str) -> Value {
    json!({
        "url": url,
        "bind_dn": "cn=svc-dumbmonit,ou=service,dc=example,dc=org",
        "bind_password": SERVICE_PASSWORD,
        "base_dn": "dc=example,dc=org",
        "admin_groups": ["cn=DumbMonit Admins,ou=groups,dc=example,dc=org"],
        "viewer_groups": ["staff"],
    })
}

#[tokio::test]
async fn the_service_password_never_comes_back() {
    let app = configured(|_| {}).await;
    let admin = app.admin_cookie().await;

    let saved =
        app.put("/api/auth/ldap/config", form("ldaps://dc.example.org"), Some(&admin)).await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    assert_eq!(saved.body["source"], "settings");
    assert_eq!(saved.body["enabled"], true);
    assert_eq!(saved.body["has_bind_password"], true);
    assert_eq!(
        saved.body["tls_verify"], true,
        "la vérification du certificat est active par défaut"
    );
    assert!(!saved.body.to_string().contains(SERVICE_PASSWORD));

    let read = app.get("/api/auth/ldap/config", Some(&admin)).await;
    assert!(!read.body.to_string().contains(SERVICE_PASSWORD));
    assert_eq!(read.body["has_bind_password"], true);
    assert_eq!(read.body["viewer_groups"], json!(["staff"]));

    // Sans mot de passe, même serveur et même compte : il est conservé.
    let mut again = form("ldaps://dc.example.org");
    again.as_object_mut().unwrap().remove("bind_password");
    let kept = app.put("/api/auth/ldap/config", again.clone(), Some(&admin)).await;
    assert_eq!(kept.status, StatusCode::OK, "{}", kept.body);
    assert_eq!(kept.body["has_bind_password"], true);

    // Un autre serveur n'hérite pas du mot de passe enregistré.
    again["url"] = json!("ldaps://ailleurs.example.org");
    let other = app.put("/api/auth/ldap/config", again.clone(), Some(&admin)).await;
    assert_eq!(other.status, StatusCode::BAD_REQUEST, "{}", other.body);

    // Recherche anonyme : le mot de passe est effacé.
    again["url"] = json!("ldaps://dc.example.org");
    again["bind_dn"] = json!("");
    let anonymous = app.put("/api/auth/ldap/config", again, Some(&admin)).await;
    assert_eq!(anonymous.status, StatusCode::OK, "{}", anonymous.body);
    assert_eq!(anonymous.body["has_bind_password"], false);

    let status = app.get("/api/auth/status", None).await;
    assert_eq!(status.body["ldap"]["enabled"], true);

    let forgotten = app.delete("/api/auth/ldap/config", Some(&admin)).await;
    assert_eq!(forgotten.status, StatusCode::OK);
    assert_eq!(forgotten.body["source"], "none");
    assert_eq!(forgotten.body["enabled"], false);
}

#[tokio::test]
async fn plain_ldap_is_refused_unless_explicitly_allowed() {
    let app = configured(|_| {}).await;
    let admin = app.admin_cookie().await;
    let plain = app.put("/api/auth/ldap/config", form("ldap://dc.example.org"), Some(&admin)).await;
    assert_eq!(plain.status, StatusCode::BAD_REQUEST, "{}", plain.body);

    let mut starttls = form("ldap://dc.example.org");
    starttls["starttls"] = json!(true);
    let reply = app.put("/api/auth/ldap/config", starttls, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    let mut injected = form("ldaps://dc.example.org");
    injected["user_filter"] = json!("(objectClass=user)");
    let reply = app.put("/api/auth/ldap/config", injected, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "un filtre sans {{username}}");

    let allowed = configured(|config| config.ldap_allow_plain = true).await;
    let admin = allowed.admin_cookie().await;
    let reply =
        allowed.put("/api/auth/ldap/config", form("ldap://dc.example.org"), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["allow_plain"], true);
}

#[tokio::test]
async fn only_admins_see_the_directory_settings() {
    let app = configured(|_| {}).await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get("/api/auth/ldap/config", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
    let reply = app.post("/api/auth/ldap/test", json!({}), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/auth/ldap/config", None).await.status, StatusCode::UNAUTHORIZED);
}

/// L'annuaire est en panne : l'administrateur local entre quand même, et un
/// utilisateur d'annuaire reçoit la réponse générique, sans 500.
#[tokio::test]
async fn the_local_admin_still_signs_in_when_the_directory_is_down() {
    let app = configured(|config| {
        config.ldap = LdapEnv {
            config: LdapConfig {
                url: "ldap://127.0.0.1:1".into(),
                base_dn: "dc=example,dc=org".into(),
                timeout_seconds: 2,
                ..LdapConfig::default()
            }
            .normalized(),
        };
        config.ldap_allow_plain = true;
    })
    .await;

    let admin = app.login_as("admin", PASSWORD).await;
    assert_eq!(admin.status, StatusCode::NO_CONTENT, "{}", admin.body);

    for (username, password) in [("jdoe", "un-mot-de-passe"), ("jdoe", ""), ("*", "x")] {
        let reply = app.login_as(username, password).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{username}: {}", reply.body);
        assert_eq!(reply.body["error"], "Wrong username or password.");
    }
    // Un mauvais mot de passe pour l'administrateur local ne part pas à l'annuaire.
    let wrong = app.login_as("admin", "pas-le-bon").await;
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);

    let status = app.get("/api/auth/status", None).await;
    assert_eq!(status.body["ldap"]["enabled"], true);

    let cookie = admin.cookie();
    let report =
        app.post("/api/auth/ldap/test", json!({ "username": "jdoe" }), Some(&cookie)).await;
    assert_eq!(report.status, StatusCode::OK, "{}", report.body);
    assert_eq!(report.body["ok"], false);
    assert_eq!(report.body["steps"][0]["step"], "connect");
    assert_eq!(report.body["steps"][0]["ok"], false);
}

/// Un faux serveur qui ne répond rien et garde ce qu'il reçoit.
async fn spy() -> (String, Arc<Mutex<Vec<u8>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ldap://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let sink = sink.clone();
            tokio::spawn(async move {
                let mut buffer = [0u8; 4096];
                while let Ok(Ok(n)) =
                    tokio::time::timeout(Duration::from_secs(5), socket.read(&mut buffer)).await
                {
                    if n == 0 {
                        break;
                    }
                    sink.lock().unwrap().extend_from_slice(&buffer[..n]);
                }
            });
        }
    });
    (url, seen)
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle.as_bytes())
}

/// Le bouton « Tester » reçoit le formulaire en cours : changer l'adresse ne
/// doit pas suffire à faire partir le mot de passe enregistré ailleurs.
#[tokio::test]
async fn the_stored_password_only_goes_to_its_own_server() {
    let (saved_url, saved_seen) = spy().await;
    let (other_url, other_seen) = spy().await;
    let app = configured(|config| config.ldap_allow_plain = true).await;
    let admin = app.admin_cookie().await;

    let mut saved = form(&saved_url);
    saved["timeout_seconds"] = json!(1);
    let reply = app.put("/api/auth/ldap/config", saved.clone(), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    saved.as_object_mut().unwrap().remove("bind_password");
    let mut elsewhere = saved.clone();
    elsewhere["url"] = json!(other_url);

    let report =
        app.post("/api/auth/ldap/test", json!({ "config": elsewhere }), Some(&admin)).await;
    assert_eq!(report.body["ok"], false);
    let report = app.post("/api/auth/ldap/test", json!({ "config": saved }), Some(&admin)).await;
    assert_eq!(report.body["ok"], false);
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(
        contains(&saved_seen.lock().unwrap(), SERVICE_PASSWORD),
        "témoin : le serveur enregistré reçoit bien la liaison du compte de service"
    );
    assert!(
        !contains(&other_seen.lock().unwrap(), SERVICE_PASSWORD),
        "le mot de passe enregistré est parti vers un autre serveur"
    );
}
