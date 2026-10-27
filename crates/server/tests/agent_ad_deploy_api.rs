//! Tests d'intégration du déploiement par Active Directory : le jeton de
//! flotte limité à un domaine, son script, et la façon dont les serveurs
//! s'inscrivent avec.
//!
//! Tout passe par l'API réelle, comme le ferait un serveur du domaine.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_HEADER: &str = "x-dumbmonit-agent-secret";

/// Crée un kit pour `corp.example.com` et rend la réponse.
async fn kit(app: &TestApp, admin: &str, extra: Value) -> Reply {
    let mut body = json!({ "domain": "Corp.Example.com", "base_url": "https://dm.example.com/" });
    if let Value::Object(fields) = extra {
        for (key, value) in fields {
            body[key] = value;
        }
    }
    app.post("/api/agent/ad-kit", body, Some(admin)).await
}

/// Un lot vide poussé au nom d'un serveur Windows.
async fn push(app: &TestApp, token: &str, hostname: &str, binding: Option<&str>) -> Reply {
    let batch = json!({
        "protocol": 1,
        "identity": {
            "hostname": hostname,
            "os": "windows",
            "agent_version": "0.13.0",
            "binding_supported": true,
            "tags": { "ad_ou": "OU=Servers,DC=corp,DC=example,DC=com" },
        },
        "sent_at_ms": 0,
        "samples": [],
    });
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = binding {
        builder = builder.header(SECRET_HEADER, secret);
    }
    let request = builder.body(Body::from(batch.to_string())).unwrap();
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

#[tokio::test]
async fn the_kit_is_a_domain_fleet_token_and_a_script_that_carries_it() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply =
        kit(&app, &admin, json!({ "target_ou": "OU=Servers,DC=corp,DC=example,DC=com" })).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let body = &reply.body;
    let secret = body["secret"].as_str().expect("secret");
    assert!(secret.starts_with("dmon_"));
    assert_eq!(body["filename"], "Deploy-DumbMonit.ps1");
    assert_eq!(body["folder"], "Active Directory corp.example.com");
    assert_eq!(body["token"]["ad_domain"], "corp.example.com");
    assert_eq!(body["token"]["max_uses"], Value::Null, "un jeton de flotte");
    assert!(body["token"]["expires_at"].is_string(), "30 jours par défaut");

    let script = body["script"].as_str().expect("script");
    assert!(script.starts_with('\u{feff}'));
    assert!(script.contains("$Domain         = 'corp.example.com'"));
    assert!(script.contains("$Url            = 'https://dm.example.com'"));
    assert!(!script.contains("__DM_"));
    assert!(!script.contains(secret), "le jeton n'est jamais en clair dans le script");
    // La commande d'installation est celle que génère le serveur.
    assert!(body["install_windows"].as_str().unwrap().contains(secret));

    // Le jeton est listé, désigné par son domaine, jamais par son secret.
    let tokens = app.get("/api/agent/tokens", Some(&admin)).await;
    assert_eq!(tokens.body[0]["ad_domain"], "corp.example.com");
    assert!(!tokens.body.to_string().contains(secret));
}

#[tokio::test]
async fn only_the_servers_of_the_domain_enrol_and_they_are_filed_together() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = kit(&app, &admin, json!({})).await;
    let token = reply.body["secret"].as_str().unwrap().to_string();

    // Nom court ou autre domaine : refusé, et le refus dit quoi faire.
    for outsider in ["srv01", "srv01.other.example.com"] {
        let refused = push(&app, &token, outsider, None).await;
        assert_eq!(refused.status, StatusCode::FORBIDDEN, "{outsider}: {}", refused.body);
        assert!(refused.body["error"].as_str().unwrap().contains("Active Directory domain"));
    }

    // Un serveur du domaine s'inscrit et reçoit sa propre identité.
    let enrolled = push(&app, &token, "SRV01.corp.example.com", None).await;
    assert_eq!(enrolled.status, StatusCode::OK, "{}", enrolled.body);
    let own = enrolled.body["agent_secret"].as_str().expect("secret de liaison");
    assert!(own.starts_with("dmab_"));
    let target = enrolled.body["target_id"].as_i64().unwrap();

    let view = app.get(&format!("/api/targets/{target}"), Some(&admin)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    assert_eq!(view.body["group_name"], "Active Directory corp.example.com");
    assert_eq!(view.body["tags"]["ad"], "corp.example.com");
    assert_eq!(view.body["tags"]["ad_ou"], "OU=Servers,DC=corp,DC=example,DC=com");

    // Le jeton de flotte seul ne suffit pas à se faire passer pour lui.
    let usurper = push(&app, &token, "SRV01.corp.example.com", None).await;
    assert_eq!(usurper.status, StatusCode::FORBIDDEN, "{}", usurper.body);

    let tokens = app.get("/api/agent/tokens", Some(&admin)).await;
    assert_eq!(tokens.body[0]["uses"], 1);
}

#[tokio::test]
async fn the_number_of_servers_can_be_capped() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = kit(&app, &admin, json!({ "max_uses": 1 })).await;
    assert_eq!(reply.body["token"]["max_uses"], 1);
    let token = reply.body["secret"].as_str().unwrap().to_string();

    assert_eq!(push(&app, &token, "srv01.corp.example.com", None).await.status, StatusCode::OK);
    let second = push(&app, &token, "srv02.corp.example.com", None).await;
    assert_eq!(second.status, StatusCode::FORBIDDEN, "{}", second.body);
}

#[tokio::test]
async fn closing_stops_enrolments_and_revoking_stops_everything() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = kit(&app, &admin, json!({})).await;
    let token = reply.body["secret"].as_str().unwrap().to_string();
    let id = reply.body["token"]["id"].as_i64().unwrap();

    let enrolled = push(&app, &token, "srv01.corp.example.com", None).await;
    let own = enrolled.body["agent_secret"].as_str().unwrap().to_string();

    // Fin du déploiement : plus d'inscription, mais le parc continue.
    let closed = app.post(&format!("/api/agent/tokens/{id}/close"), json!({}), Some(&admin)).await;
    assert_eq!(closed.status, StatusCode::NO_CONTENT, "{}", closed.body);
    let late = push(&app, &token, "srv02.corp.example.com", None).await;
    assert_eq!(late.status, StatusCode::FORBIDDEN, "{}", late.body);
    let again = push(&app, &token, "srv01.corp.example.com", Some(&own)).await;
    assert_eq!(again.status, StatusCode::OK, "{}", again.body);

    // La révocation, elle, coupe tout.
    let revoked = app.delete(&format!("/api/agent/tokens/{id}"), Some(&admin)).await;
    assert_eq!(revoked.status, StatusCode::NO_CONTENT);
    let cut = push(&app, &token, "srv01.corp.example.com", Some(&own)).await;
    assert_eq!(cut.status, StatusCode::UNAUTHORIZED, "{}", cut.body);
    let gone = app.post(&format!("/api/agent/tokens/{id}/close"), json!({}), Some(&admin)).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_kit_cannot_be_created_already_expired_or_capped_at_nothing() {
    // Le terme lui-même est couvert par les tests du magasin ; ici, l'API
    // refuse une portée qui n'inscrirait rien.
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let expired = kit(&app, &admin, json!({ "expires_in_days": 0 })).await;
    assert_eq!(expired.status, StatusCode::BAD_REQUEST, "{}", expired.body);
    let empty = kit(&app, &admin, json!({ "max_uses": 0 })).await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST, "{}", empty.body);
    let tokens = app.get("/api/agent/tokens", Some(&admin)).await;
    assert_eq!(tokens.body.as_array().map(Vec::len), Some(0), "aucun jeton à moitié créé");
}

#[tokio::test]
async fn bad_input_and_non_administrators_are_refused() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    for domain in ["", "corp", "corp..lan", "corp.lan'; Remove-GPO *"] {
        let reply = app.post("/api/agent/ad-kit", json!({ "domain": domain }), Some(&admin)).await;
        // `corp` seul est un nom DNS valide d'une étiquette : accepté.
        if domain == "corp" {
            assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
        } else {
            assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{domain:?}: {}", reply.body);
        }
    }
    let quote = kit(&app, &admin, json!({ "base_url": "https://dm'.example.com" })).await;
    assert_eq!(quote.status, StatusCode::BAD_REQUEST, "{}", quote.body);
    let newline = kit(&app, &admin, json!({ "target_ou": "OU=A\nB" })).await;
    assert_eq!(newline.status, StatusCode::BAD_REQUEST, "{}", newline.body);

    let viewer = app.viewer_cookie(&admin).await;
    let reply = kit(&app, &viewer, json!({})).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let anonymous = app.post("/api/agent/ad-kit", json!({ "domain": "corp.lan" }), None).await;
    assert!(anonymous.status.is_client_error(), "{}", anonymous.status);
    assert_ne!(anonymous.status, StatusCode::CREATED);
}

#[tokio::test]
async fn the_server_name_and_dns_server_reach_the_kit() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply = kit(
        &app,
        &admin,
        json!({ "server_host": "DumbMonit.Corp.Example.com", "dns_server": "10.0.0.2" }),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["server_url"], json!("https://dumbmonit.corp.example.com"));
    assert_eq!(reply.body["dns_server"], json!("10.0.0.2"));
    let script = reply.body["script"].as_str().expect("script");
    assert!(script.contains("$Url            = 'https://dumbmonit.corp.example.com'"));
    assert!(script.contains("$DnsServer      = '10.0.0.2'"));
    assert!(
        reply.body["install_windows"]
            .as_str()
            .unwrap()
            .contains("https://dumbmonit.corp.example.com")
    );

    // Sans saisie : l'URL publique, aucun serveur DNS.
    let plain = kit(&app, &admin, json!({})).await;
    assert_eq!(plain.status, StatusCode::CREATED, "{}", plain.body);
    assert_eq!(plain.body["server_url"], json!("https://dm.example.com"));
    assert_eq!(plain.body["dns_server"], Value::Null);
    assert!(plain.body["script"].as_str().unwrap().contains("$DnsServer      = ''"));
}

#[tokio::test]
async fn a_malformed_server_name_or_dns_server_is_refused() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    for extra in [
        json!({ "server_host": "https://dm.example.com" }),
        json!({ "server_host": "dm'; calc" }),
        json!({ "server_host": "dm:99999" }),
        json!({ "dns_server": "dns.corp.lan" }),
        json!({ "dns_server": "10.0.0.2'; calc" }),
    ] {
        let reply = kit(&app, &admin, extra.clone()).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{extra}: {}", reply.body);
    }
}
