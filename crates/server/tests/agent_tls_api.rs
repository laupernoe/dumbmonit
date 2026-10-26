//! Canal chiffré des agents, de bout en bout : un vrai port TLS, de vraies
//! poignées de main, comme le ferait un agent — ou un attaquant.
//!
//! Le serveur est monté sur une base temporaire ; l'ancien canal (le port de
//! l'interface) est appelé sans socket, le canal chiffré par un client
//! rustls réel avec ou sans certificat client.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{agent_tls, api, collectors, db, tsdb};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::{Value, json};

use common::TestApp;

const SERVER_NAME: &str = dumbmonit_proto::AGENT_TLS_SERVER_NAME;

struct Lab {
    app: TestApp,
    state: AppState,
    addr: SocketAddr,
}

/// Instance avec un administrateur. `before_init` s'exécute avant la
/// préparation du canal chiffré (pour simuler une instance qui avait déjà des
/// agents au moment de la mise à jour).
async fn lab_with<T>(before_init: impl AsyncFnOnce(&TestApp, &str) -> T) -> (Lab, String, T) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("chiffre");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::AgentCollector::new(pool.clone())));
    let victoria = tsdb::Victoria::new(common::UNREACHABLE_VICTORIA.to_string()).expect("client");
    let sink = tsdb::spawn_writer(victoria.clone(), std::time::Duration::from_secs(60));
    let state = AppState::new(Inner { config, pool, cipher, victoria, sink, collectors: registry });
    let app = TestApp { router: api::router(state.clone()), _dir: dir };
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    let prepared = before_init(&app, &admin).await;

    agent_tls::init(&state).await.expect("canal des agents");
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port");
    let addr = tcp.local_addr().expect("adresse");
    let listener = agent_tls::listen(state.clone(), tcp).expect("écoute");
    let router = agent_tls::router(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<agent_tls::AgentPeer>())
            .await
            .expect("canal des agents");
    });
    (Lab { app, state, addr }, admin, prepared)
}

async fn lab() -> (Lab, String) {
    let (lab, admin, ()) = lab_with(async |_, _| {}).await;
    (lab, admin)
}

async fn token(app: &TestApp, admin: &str) -> String {
    let reply = app
        .post("/api/agent/tokens", json!({ "name": "parc", "reusable": true }), Some(admin))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "jeton : {}", reply.body);
    reply.body["secret"].as_str().expect("secret").to_string()
}

fn identity(machine_id: &str) -> Value {
    json!({
        "hostname": machine_id,
        "os": "linux",
        "agent_version": "0.14.0",
        "machine_id": machine_id,
        "commands_enabled": true,
        "binding_supported": true,
    })
}

fn batch(machine_id: &str) -> Value {
    json!({ "protocol": 1, "identity": identity(machine_id), "sent_at_ms": 0, "samples": [] })
}

/// Autorité active, telle que la base la garde.
async fn authority(lab: &Lab) -> Vec<u8> {
    sqlx::query_scalar("SELECT cert_der FROM agent_ca WHERE state = 'active'")
        .fetch_one(&lab.state.pool)
        .await
        .expect("autorité")
}

/// Client du canal chiffré : il fait confiance à l'autorité donnée, et
/// présente un certificat client s'il en a un.
fn client(lab: &Lab, ca: &[u8], identity: Option<(&[u8], &[u8])>) -> reqwest::Client {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(CertificateDer::from(ca.to_vec())).expect("racine");
    let builder = rustls::ClientConfig::builder_with_provider(agent_tls::provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("TLS 1.3")
        .with_root_certificates(roots);
    let config = match identity {
        Some((certificate, key)) => builder
            .with_client_auth_cert(
                vec![CertificateDer::from(certificate.to_vec())],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.to_vec())),
            )
            .expect("certificat client"),
        None => builder.with_no_client_auth(),
    };
    reqwest::Client::builder()
        .tls_backend_preconfigured(config)
        .resolve(SERVER_NAME, lab.addr)
        .build()
        .expect("client")
}

fn url(lab: &Lab, path: &str) -> String {
    format!("https://{SERVER_NAME}:{}{path}", lab.addr.port())
}

struct Enrolled {
    certificate: Vec<u8>,
    key: Vec<u8>,
    secret: Option<String>,
    pins: Vec<String>,
}

/// Enrôle une machine sur le canal chiffré, comme le ferait l'agent : la
/// réponse, et la clé privée générée pour l'occasion.
async fn enrol(
    lab: &Lab,
    enrolment: &str,
    machine_id: &str,
    secret: Option<&str>,
) -> (reqwest::Response, Vec<u8>) {
    let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).expect("clé");
    let ca = authority(lab).await;
    let mut request = client(lab, &ca, None)
        .post(url(lab, "/api/agent/tls/enroll"))
        .bearer_auth(enrolment)
        .json(&json!({ "identity": identity(machine_id), "public_key": BASE64.encode(key.public_key_raw()) }));
    if let Some(secret) = secret {
        request = request.header("x-dumbmonit-agent-secret", secret);
    }
    (request.send().await.expect("enrôlement"), key.serialize_der())
}

async fn enrolled(lab: &Lab, enrolment: &str, machine_id: &str, secret: Option<&str>) -> Enrolled {
    let (response, key) = enrol(lab, enrolment, machine_id, secret).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("corps");
    Enrolled {
        certificate: BASE64
            .decode(body["certificate"].as_str().expect("certificat"))
            .expect("base64"),
        key,
        secret: body["agent_secret"].as_str().map(str::to_string),
        pins: body["pins"]
            .as_array()
            .expect("empreintes")
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect(),
    }
}

/// Pousse un lot sur le canal chiffré.
async fn push_tls(
    lab: &Lab,
    enrolled: &Enrolled,
    enrolment: &str,
    machine_id: &str,
) -> reqwest::Response {
    let ca = authority(lab).await;
    let mut request =
        client(lab, &ca, Some((enrolled.certificate.as_slice(), enrolled.key.as_slice())))
            .post(url(lab, "/api/ingest"))
            .bearer_auth(enrolment)
            .json(&batch(machine_id));
    if let Some(secret) = &enrolled.secret {
        request = request.header("x-dumbmonit-agent-secret", secret);
    }
    request.send().await.expect("lot")
}

/// Pousse un lot sur l'ancien canal (port de l'interface, sans socket).
async fn push_plain(
    app: &TestApp,
    enrolment: &str,
    machine_id: &str,
    secret: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        builder = builder.header("x-dumbmonit-agent-secret", secret);
    }
    let request = builder.body(Body::from(batch(machine_id).to_string())).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let offer = response
        .headers()
        .get(dumbmonit_proto::AGENT_TLS_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null), offer)
}

#[tokio::test]
async fn an_agent_enrols_then_reports_over_mtls_with_post_quantum_key_exchange() {
    let (lab, admin) = lab().await;
    let enrolment = token(&lab.app, &admin).await;
    let agent = enrolled(&lab, &enrolment, "id-nas", None).await;
    assert!(agent.secret.is_some(), "première présentation : secret de liaison attribué");
    assert_eq!(agent.pins, lab.state.agent_tls().pins());

    let response = push_tls(&lab, &agent, &enrolment, "id-nas").await;
    assert_eq!(response.status(), StatusCode::OK);
    let offer = response.headers().get(dumbmonit_proto::AGENT_TLS_HEADER).expect("annonce");
    let offer = dumbmonit_proto::TlsOffer::parse(offer.to_str().unwrap()).expect("annonce lisible");
    assert_eq!(offer.pins, agent.pins);

    let group: Option<String> = sqlx::query_scalar("SELECT tls_group FROM agent_hosts")
        .fetch_one(&lab.state.pool)
        .await
        .unwrap();
    assert_eq!(group.as_deref(), Some(agent_tls::POST_QUANTUM_GROUP));

    // La vue de l'interface le dit.
    let view = lab.app.get("/api/agent/tls", Some(&admin)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    assert_eq!(view.body["agents"][0]["channel"], "mtls");
    assert_eq!(view.body["required"], true, "installation neuve : mTLS exigé d'office");
}

#[tokio::test]
async fn without_a_client_certificate_only_enrolment_answers() {
    let (lab, admin) = lab().await;
    let enrolment = token(&lab.app, &admin).await;
    let ca = authority(&lab).await;
    let anonymous = client(&lab, &ca, None);

    let response = anonymous
        .post(url(&lab, "/api/ingest"))
        .bearer_auth(&enrolment)
        .json(&batch("id-x"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"], dumbmonit_proto::CERT_REFUSED);

    // Rien d'autre n'est servi : ni interface, ni santé, ni spécification.
    for path in ["/", "/api/health", "/api/openapi.json", "/api/agent/tls"] {
        let response = anonymous.get(url(&lab, path)).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn a_bad_token_is_refused_and_repeated_attempts_are_throttled() {
    let (lab, _) = lab().await;
    let mut last = StatusCode::OK;
    for _ in 0..8 {
        last = enrol(&lab, "dmon_faux", "id-x", None).await.0.status();
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn a_client_certificate_from_another_authority_fails_the_handshake() {
    let (lab, admin) = lab().await;
    let enrolment = token(&lab.app, &admin).await;
    let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
    let mut params = rcgen::CertificateParams::default();
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ClientAuth];
    let forged = params.self_signed(&key).unwrap();
    let ca = authority(&lab).await;
    let result = client(&lab, &ca, Some((forged.der().as_ref(), key.serialize_der().as_slice())))
        .post(url(&lab, "/api/ingest"))
        .bearer_auth(&enrolment)
        .json(&batch("id-x"))
        .send()
        .await;
    assert!(result.is_err(), "poignée de main acceptée : {:?}", result.map(|r| r.status()));
}

#[tokio::test]
async fn deleting_the_machine_revokes_its_certificate() {
    let (lab, admin) = lab().await;
    let enrolment = token(&lab.app, &admin).await;
    let agent = enrolled(&lab, &enrolment, "id-nas", None).await;
    let response = push_tls(&lab, &agent, &enrolment, "id-nas").await;
    let ack: Value = response.json().await.unwrap();
    let target = ack["target_id"].as_i64().expect("cible");

    let deleted = lab.app.delete(&format!("/api/targets/{target}"), Some(&admin)).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);

    let response = push_tls(&lab, &agent, &enrolment, "id-nas").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"], dumbmonit_proto::CERT_REFUSED);
}

#[tokio::test]
async fn a_certificate_cannot_speak_for_another_machine() {
    let (lab, admin) = lab().await;
    let enrolment = token(&lab.app, &admin).await;
    let nas = enrolled(&lab, &enrolment, "id-nas", None).await;
    let _web = enrolled(&lab, &enrolment, "id-web", None).await;
    let response = push_tls(&lab, &nas, &enrolment, "id-web").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn renewal_issues_a_new_certificate_under_the_current_one() {
    let (lab, admin) = lab().await;
    let enrolment = token(&lab.app, &admin).await;
    let agent = enrolled(&lab, &enrolment, "id-nas", None).await;
    let ca = authority(&lab).await;
    let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
    let response = client(&lab, &ca, Some((agent.certificate.as_slice(), agent.key.as_slice())))
        .post(url(&lab, "/api/agent/tls/renew"))
        .json(&json!({ "public_key": BASE64.encode(key.public_key_raw()) }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let renewed = Enrolled {
        certificate: BASE64.decode(body["certificate"].as_str().unwrap()).unwrap(),
        key: key.serialize_der(),
        secret: agent.secret.clone(),
        pins: agent.pins.clone(),
    };
    assert_ne!(renewed.certificate, agent.certificate);
    assert!(body["not_after_ms"].as_i64().unwrap() > chrono::Utc::now().timestamp_millis());
    assert_eq!(push_tls(&lab, &renewed, &enrolment, "id-nas").await.status(), StatusCode::OK);

    // Sans certificat, pas de renouvellement : il faut repasser par le jeton.
    let anonymous = client(&lab, &ca, None)
        .post(url(&lab, "/api/agent/tls/renew"))
        .json(&json!({ "public_key": BASE64.encode(key.public_key_raw()) }))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_existing_agent_migrates_from_the_plain_channel_then_it_can_be_closed() {
    // Une instance mise à jour : la machine rapportait déjà en clair.
    let (lab, admin, (enrolment, secret)) = lab_with(async |app, admin| {
        let enrolment = token(app, admin).await;
        let (status, ack, _) = push_plain(app, &enrolment, "id-nas", None).await;
        assert_eq!(status, StatusCode::OK, "{ack}");
        let secret = ack["agent_secret"].as_str().unwrap().to_string();
        (enrolment, secret)
    })
    .await;

    // L'ancien canal reste ouvert, et annonce le canal chiffré.
    let (status, _, offer) = push_plain(&lab.app, &enrolment, "id-nas", Some(&secret)).await;
    assert_eq!(status, StatusCode::OK);
    let offer = dumbmonit_proto::TlsOffer::parse(&offer.expect("annonce")).expect("lisible");
    assert_eq!(offer.pins, lab.state.agent_tls().pins());

    // L'agent à jour s'enrôle avec son jeton et son secret de liaison : même
    // machine, pas de nouveau secret.
    let agent = enrolled(&lab, &enrolment, "id-nas", Some(&secret)).await;
    assert_eq!(agent.secret, None);
    let agent = Enrolled { secret: Some(secret.clone()), ..agent };
    assert_eq!(push_tls(&lab, &agent, &enrolment, "id-nas").await.status(), StatusCode::OK);

    // Un voisin qui connaît la clé d'identité mais pas le secret n'obtient rien.
    assert_eq!(enrol(&lab, &enrolment, "id-nas", None).await.0.status(), StatusCode::FORBIDDEN);

    // Exiger mTLS ferme l'ancien canal ; le refus porte encore l'annonce.
    let view = lab.app.get("/api/agent/tls", Some(&admin)).await;
    assert_eq!(view.body["required"], false, "mise à jour : l'ancien canal reste ouvert");
    let set = lab.app.put("/api/agent/tls", json!({ "required": true }), Some(&admin)).await;
    assert_eq!(set.status, StatusCode::OK, "{}", set.body);
    let (status, body, offer) = push_plain(&lab.app, &enrolment, "id-nas", Some(&secret)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], dumbmonit_proto::MTLS_REQUIRED);
    assert!(offer.is_some());
    // Le canal chiffré, lui, passe toujours.
    assert_eq!(push_tls(&lab, &agent, &enrolment, "id-nas").await.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_rotation_is_announced_before_the_new_authority_signs_anything() {
    let (lab, admin) = lab().await;
    let enrolment = token(&lab.app, &admin).await;
    let agent = enrolled(&lab, &enrolment, "id-nas", None).await;
    let old_ca = authority(&lab).await;

    let started = lab.app.post("/api/agent/tls/rotation", json!({}), Some(&admin)).await;
    assert_eq!(started.status, StatusCode::OK, "{}", started.body);
    let response = push_tls(&lab, &agent, &enrolment, "id-nas").await;
    let offer = response.headers().get(dumbmonit_proto::AGENT_TLS_HEADER).unwrap();
    let offer = dumbmonit_proto::TlsOffer::parse(offer.to_str().unwrap()).unwrap();
    assert_eq!(offer.pins.len(), 2, "l'autorité suivante est annoncée");
    assert_eq!(offer.pins[0], agent.pins[0], "l'active d'abord");

    let done = lab.app.post("/api/agent/tls/rotation/complete", json!({}), Some(&admin)).await;
    assert_eq!(done.status, StatusCode::OK, "{}", done.body);
    let new_ca = authority(&lab).await;
    assert_ne!(new_ca, old_ca);
    // Le serveur se présente sous la nouvelle autorité ; le certificat client
    // signé par l'ancienne reste accepté jusqu'à son renouvellement.
    let response =
        client(&lab, &new_ca, Some((agent.certificate.as_slice(), agent.key.as_slice())))
            .post(url(&lab, "/api/ingest"))
            .bearer_auth(&enrolment)
            .header("x-dumbmonit-agent-secret", agent.secret.clone().unwrap())
            .json(&batch("id-nas"))
            .send()
            .await
            .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let stale = client(&lab, &old_ca, Some((agent.certificate.as_slice(), agent.key.as_slice())))
        .post(url(&lab, "/api/ingest"))
        .send()
        .await;
    assert!(stale.is_err(), "l'ancienne autorité ne signe plus le serveur");
}

#[tokio::test]
async fn the_install_parameters_carry_the_agent_channel_and_its_pin() {
    let (lab, admin) = lab().await;
    let reply =
        lab.app.get("/api/agent/install?base_url=https://monit.example.com", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let pin = lab.state.agent_tls().pins()[0].clone();
    assert_eq!(reply.body["ca_pin"], pin.as_str());
    assert_eq!(reply.body["agent_url"], "https://monit.example.com:8443");
    let linux = reply.body["linux_template"].as_str().unwrap();
    assert!(linux.contains("--token={TOKEN}"), "{linux}");
    assert!(linux.contains(&format!("--ca-pin={pin}")), "{linux}");
    let windows = reply.body["windows_template"].as_str().unwrap();
    assert!(windows.contains(&format!("-CaPin {pin}")), "{windows}");
}
