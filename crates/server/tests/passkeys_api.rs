//! Clés d'accès (WebAuthn) de bout en bout : enregistrement, connexion sans mot
//! de passe, rejeu, signature fausse, gestion. L'authentificateur est simulé par
//! une clé P-256 de test qui signe comme le ferait un trousseau.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use common::{PASSWORD, TestApp};
use http_body_util::BodyExt;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost";
const RP_ID: &str = "localhost";

struct Reply {
    status: StatusCode,
    body: Value,
    cookie: Option<String>,
}

async fn send(
    app: &TestApp,
    method: &str,
    uri: &str,
    body: Option<Value>,
    cookie: Option<&str>,
) -> Reply {
    let mut builder = Request::builder().method(method).uri(uri).header(header::ORIGIN, ORIGIN);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie).header("x-requested-with", "DumbMonit");
    }
    let request = match body {
        Some(value) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .map(str::to_string);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply { status, body: serde_json::from_slice(&bytes).unwrap_or(Value::Null), cookie }
}

fn key() -> SigningKey {
    SigningKey::from_slice(&[11u8; 32]).unwrap()
}

fn client_data(kind: &str, challenge: &str) -> Vec<u8> {
    json!({ "type": kind, "challenge": challenge, "origin": ORIGIN }).to_string().into_bytes()
}

fn auth_data(flags: u8, count: u32, credential: Option<(&[u8], Vec<u8>)>) -> Vec<u8> {
    let mut out = Sha256::digest(RP_ID.as_bytes()).to_vec();
    out.push(flags);
    out.extend_from_slice(&count.to_be_bytes());
    if let Some((id, cose)) = credential {
        out.extend_from_slice(&[0u8; 16]);
        out.extend_from_slice(&(id.len() as u16).to_be_bytes());
        out.extend_from_slice(id);
        out.extend_from_slice(&cose);
    }
    out
}

fn cose(key: &SigningKey) -> Vec<u8> {
    let point = key.verifying_key().to_encoded_point(false);
    let mut out = vec![0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20];
    out.extend_from_slice(point.x().unwrap());
    out.extend_from_slice(&[0x22, 0x58, 0x20]);
    out.extend_from_slice(point.y().unwrap());
    out
}

fn attestation(auth: &[u8]) -> Vec<u8> {
    let mut out = vec![0xa3, 0x63, b'f', b'm', b't', 0x64, b'n', b'o', b'n', b'e'];
    out.extend_from_slice(&[0x67, b'a', b't', b't', b'S', b't', b'm', b't', 0xa0]);
    out.extend_from_slice(&[0x68, b'a', b'u', b't', b'h', b'D', b'a', b't', b'a', 0x59]);
    out.extend_from_slice(&(auth.len() as u16).to_be_bytes());
    out.extend_from_slice(auth);
    out
}

/// `r || s` en DER.
fn der(signature: &Signature) -> Vec<u8> {
    let raw = signature.to_bytes();
    let integer = |bytes: &[u8]| {
        let mut value: Vec<u8> = bytes.iter().copied().skip_while(|b| *b == 0).collect();
        if value.first().is_none_or(|b| b & 0x80 != 0) {
            value.insert(0, 0);
        }
        let mut out = vec![0x02, value.len() as u8];
        out.extend(value);
        out
    };
    let mut body = integer(&raw[..32]);
    body.extend(integer(&raw[32..]));
    let mut out = vec![0x30, body.len() as u8];
    out.extend(body);
    out
}

const CRED_ID: &[u8] = b"test-credential-0001";

/// Enregistre la clé de test pour le compte `admin` ; rend la session.
async fn register(app: &TestApp) -> String {
    let admin = app.admin_cookie().await;
    let wrong = send(
        app,
        "POST",
        "/api/auth/passkeys/register/begin",
        Some(json!({"password": "nope"})),
        Some(&admin),
    )
    .await;
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED, "{}", wrong.body);

    let begin = send(
        app,
        "POST",
        "/api/auth/passkeys/register/begin",
        Some(json!({"password": PASSWORD})),
        Some(&admin),
    )
    .await;
    assert_eq!(begin.status, StatusCode::OK, "{}", begin.body);
    let challenge = begin.body["options"]["challenge"].as_str().unwrap().to_string();
    assert_eq!(begin.body["options"]["rp"]["id"], RP_ID);
    assert_eq!(begin.body["options"]["pubKeyCredParams"][0]["alg"], -7);

    let auth = auth_data(0x01 | 0x04 | 0x40, 0, Some((CRED_ID, cose(&key()))));
    let finish = send(
        app,
        "POST",
        "/api/auth/passkeys/register/finish",
        Some(json!({
            "challenge_id": begin.body["challenge_id"],
            "name": "  Test key ",
            "credential": {
                "id": B64.encode(CRED_ID),
                "response": {
                    "clientDataJSON": B64.encode(client_data("webauthn.create", &challenge)),
                    "attestationObject": B64.encode(attestation(&auth)),
                },
                "transports": ["internal"],
            },
        })),
        Some(&admin),
    )
    .await;
    assert_eq!(finish.status, StatusCode::OK, "{}", finish.body);
    assert_eq!(finish.body["name"], "Test key");
    admin
}

async fn assertion_body(app: &TestApp, count: u32, tamper: bool) -> Value {
    let begin = send(app, "POST", "/api/auth/passkey/login/begin", Some(json!({})), None).await;
    assert_eq!(begin.status, StatusCode::OK, "{}", begin.body);
    let challenge = begin.body["options"]["challenge"].as_str().unwrap().to_string();
    let client = client_data("webauthn.get", &challenge);
    let auth = auth_data(0x01 | 0x04, count, None);
    let mut signed = auth.clone();
    signed.extend_from_slice(&Sha256::digest(&client));
    let signature: Signature = key().sign(&signed);
    let mut sig = der(&signature);
    if tamper {
        let last = sig.len() - 1;
        sig[last] ^= 0x01;
    }
    json!({
        "challenge_id": begin.body["challenge_id"],
        "credential": {
            "id": B64.encode(CRED_ID),
            "response": {
                "clientDataJSON": B64.encode(client),
                "authenticatorData": B64.encode(auth),
                "signature": B64.encode(sig),
                "userHandle": B64.encode(b"1"),
            },
        },
    })
}

#[tokio::test]
async fn une_cle_enregistree_ouvre_une_session_sans_mot_de_passe() {
    let app = TestApp::configured().await;
    let admin = register(&app).await;

    let listed = send(&app, "GET", "/api/auth/passkeys", None, Some(&admin)).await;
    assert_eq!(listed.body.as_array().unwrap().len(), 1);
    assert_eq!(listed.body[0]["name"], "Test key");

    let status = send(&app, "GET", "/api/auth/status", None, None).await;
    assert_eq!(status.body["passkeys"], true);

    let body = assertion_body(&app, 1, false).await;
    let login =
        send(&app, "POST", "/api/auth/passkey/login/finish", Some(body.clone()), None).await;
    assert_eq!(login.status, StatusCode::NO_CONTENT, "{}", login.body);
    let cookie = login.cookie.expect("cookie de session");
    let me = send(&app, "GET", "/api/auth/me", None, Some(&cookie)).await;
    assert_eq!(me.body["username"], "admin");

    // Le même corps rejoué : le défi est consommé.
    let replay = send(&app, "POST", "/api/auth/passkey/login/finish", Some(body), None).await;
    assert_eq!(replay.status, StatusCode::UNAUTHORIZED);

    // Compteur qui ne progresse pas (1 après 1) : clé clonée.
    let stale = assertion_body(&app, 1, false).await;
    let refused = send(&app, "POST", "/api/auth/passkey/login/finish", Some(stale), None).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn une_signature_fausse_ou_une_cle_inconnue_est_refusee() {
    let app = TestApp::configured().await;
    register(&app).await;

    let tampered = assertion_body(&app, 1, true).await;
    let refused = send(&app, "POST", "/api/auth/passkey/login/finish", Some(tampered), None).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
    assert!(refused.cookie.is_none());

    let mut unknown = assertion_body(&app, 2, false).await;
    unknown["credential"]["id"] = json!(B64.encode(b"another-credential"));
    let refused = send(&app, "POST", "/api/auth/passkey/login/finish", Some(unknown), None).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn l_origine_doit_etre_https_ou_localhost() {
    let app = TestApp::configured().await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/passkey/login/begin")
        .header(header::ORIGIN, "http://monit.lan")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn renommer_et_supprimer_une_cle() {
    let app = TestApp::configured().await;
    let admin = register(&app).await;
    let id = send(&app, "GET", "/api/auth/passkeys", None, Some(&admin)).await.body[0]["id"]
        .as_i64()
        .unwrap();
    let uri = format!("/api/auth/passkeys/{id}");

    let renamed = send(&app, "PUT", &uri, Some(json!({"name": "Phone"})), Some(&admin)).await;
    assert_eq!(renamed.status, StatusCode::NO_CONTENT);
    let listed = send(&app, "GET", "/api/auth/passkeys", None, Some(&admin)).await;
    assert_eq!(listed.body[0]["name"], "Phone");

    let removed = send(&app, "DELETE", &uri, None, Some(&admin)).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let again = send(&app, "DELETE", &uri, None, Some(&admin)).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    let status = send(&app, "GET", "/api/auth/status", None, None).await;
    assert_eq!(status.body["passkeys"], false);
}
