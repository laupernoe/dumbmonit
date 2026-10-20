//! Identifiants d'une cible en modification : les parties publiques sont
//! renvoyées pour pré-remplir le formulaire, un secret laissé vide est gardé
//! quand c'est cohérent, et aucune réponse ne contient jamais un secret.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dumbmonit_proto::{Credential, SnmpV3PrivacyProtocol};
use dumbmonit_server::config::Config;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{api, collectors, db, tsdb};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const UNREACHABLE_VICTORIA: &str = "http://127.0.0.1:1";
const PASSWORD: &str = "mot-de-passe-du-homelab";

/// Secrets reconnaissables : aucun ne doit apparaître dans une réponse.
const SECRETS: &[&str] = &[
    "s3cr3t-community",
    "s3cr3t-auth-phrase",
    "s3cr3t-priv-phrase",
    "s3cr3t-token-uuid",
    "s3cr3t-bearer",
    "s3cr3t-password",
];

struct TestApp {
    router: Router,
    state: AppState,
    cookie: String,
    _dir: tempfile::TempDir,
}

async fn setup() -> TestApp {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = Config::from_env().expect("configuration par défaut");
    config.data_dir = dir.path().to_path_buf();
    config.setup_code = Some("test-setup-code".into());
    config.victoria_url = Some(UNREACHABLE_VICTORIA.to_string());

    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long")
        .await
        .expect("initialisation du chiffrement");
    let victoria = tsdb::Victoria::new(UNREACHABLE_VICTORIA).expect("client");
    let sink = tsdb::spawn_writer(victoria.clone(), std::time::Duration::from_secs(60));
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    let state = AppState::new(Inner { config, pool, cipher, victoria, sink, collectors: registry });

    let mut app =
        TestApp { router: api::router(state.clone()), state, cookie: String::new(), _dir: dir };
    let (status, body) = app
        .request(
            "POST",
            "/api/auth/setup",
            Some(json!({ "setup_code": "test-setup-code", "password": PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "création de l'admin : {body}");
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "password": PASSWORD }).to_string()))
        .unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let raw = response.headers().get(header::SET_COOKIE).expect("cookie de session");
    app.cookie = raw.to_str().unwrap().split(';').next().unwrap().to_string();
    app
}

impl TestApp {
    async fn request(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if !self.cookie.is_empty() {
            builder = builder
                .header(header::COOKIE, &self.cookie)
                .header("x-requested-with", "DumbMonit");
        }
        let request = match body {
            Some(value) => builder
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let response = self.router.clone().oneshot(request).await.expect("réponse");
        let status = response.status();
        let bytes = response.into_body().collect().await.expect("corps").to_bytes();
        let text = String::from_utf8_lossy(&bytes);
        for secret in SECRETS {
            assert!(!text.contains(secret), "{method} {uri} a renvoyé le secret {secret} : {text}");
        }
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn create(&self, name: &str, address: &str, credential: Value) -> i64 {
        let (status, body) = self
            .request(
                "POST",
                "/api/targets",
                Some(json!({ "name": name, "address": address, "kind": "dummy",
                             "credential": credential })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_i64().unwrap()
    }

    async fn put(&self, id: i64, address: &str, credential: Value) -> (StatusCode, Value) {
        self.request(
            "PUT",
            &format!("/api/targets/{id}"),
            Some(json!({ "name": "Cible", "address": address, "kind": "dummy",
                         "credential": credential })),
        )
        .await
    }

    async fn stored(&self, id: i64) -> Credential {
        db::targets::get(&self.state.pool, &self.state.cipher, id)
            .await
            .unwrap()
            .expect("cible enregistrée")
            .credential
    }
}

fn proxmox_token() -> Credential {
    Credential::ApiToken { token: "dumbmonit@pve!monitor=s3cr3t-token-uuid".into() }
}

/// Cas signalé : ouvrir un moniteur Proxmox en modification montrait des champs
/// vides. L'identifiant du jeton est désormais renvoyé, et le renvoyer sans le
/// secret garde celui qui est enregistré.
#[tokio::test]
async fn un_jeton_proxmox_garde_son_secret_quand_seul_lidentifiant_est_renvoye() {
    let app = setup().await;
    let id = app
        .create(
            "PVE",
            "pve.lan",
            json!({ "type": "api_token", "token_id": "dumbmonit@pve!monitor",
                    "secret": "s3cr3t-token-uuid" }),
        )
        .await;

    let (_, view) = app.request("GET", &format!("/api/targets/{id}"), None).await;
    assert_eq!(view["credential_public"], json!({ "token_id": "dumbmonit@pve!monitor" }));
    assert_eq!(view["has_secret"], json!(true));

    let (status, body) = app
        .put(id, "pve.lan", json!({ "type": "api_token", "token_id": "dumbmonit@pve!monitor" }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(app.stored(id).await, proxmox_token(), "le secret aurait dû être gardé");

    // Un autre jeton n'hérite pas du secret : la réponse demande de le ressaisir.
    let (status, body) = app
        .put(id, "pve.lan", json!({ "type": "api_token", "token_id": "autre@pve!monitor" }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["error"].as_str().unwrap().contains("Enter the token secret again"), "{body}");
    assert_eq!(app.stored(id).await, proxmox_token(), "rien n'a été enregistré");
}

/// Un secret repris n'est jamais envoyé vers une nouvelle adresse.
#[tokio::test]
async fn un_secret_repris_ne_suit_pas_une_nouvelle_adresse() {
    let app = setup().await;
    let id = app
        .create(
            "PVE",
            "pve.lan",
            json!({ "type": "api_token", "token": "dumbmonit@pve!monitor=s3cr3t-token-uuid" }),
        )
        .await;
    let (status, body) = app
        .put(
            id,
            "attacker.example",
            json!({ "type": "api_token", "token_id": "dumbmonit@pve!monitor" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["error"].as_str().unwrap().contains("Re-enter the credential"), "{body}");
}

#[tokio::test]
async fn un_mot_de_passe_laisse_vide_est_garde_pour_le_meme_utilisateur() {
    let app = setup().await;
    let id = app
        .create(
            "PBS",
            "pbs.lan",
            json!({ "type": "username_password", "username": "root@pam",
                    "password": "s3cr3t-password" }),
        )
        .await;
    let (status, body) = app
        .put(id, "pbs.lan", json!({ "type": "username_password", "username": "root@pam" }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["credential_public"], json!({ "username": "root@pam" }));
    assert_eq!(
        app.stored(id).await,
        Credential::UsernamePassword {
            username: "root@pam".into(),
            password: "s3cr3t-password".into()
        }
    );

    let (status, body) = app
        .put(
            id,
            "pbs.lan",
            json!({ "type": "username_password", "username": "autre@pbs", "password": "" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["error"].as_str().unwrap().contains("Enter the password again"), "{body}");
}

/// Cause d'une vraie perte : modifier un SNMP v3 en ressaisissant seulement la
/// phrase d'authentification effaçait la phrase de chiffrement.
#[tokio::test]
async fn snmp_v3_garde_la_phrase_de_chiffrement_laissee_vide() {
    let app = setup().await;
    let id = app
        .create(
            "Switch",
            "10.0.0.2",
            json!({ "type": "snmp_v3", "username": "monitor",
                    "auth": { "protocol": "sha256", "passphrase": "s3cr3t-auth-phrase" },
                    "privacy": { "protocol": "aes128", "passphrase": "s3cr3t-priv-phrase" } }),
        )
        .await;
    let (status, body) = app
        .put(
            id,
            "10.0.0.2",
            json!({ "type": "snmp_v3", "username": "monitor",
                    "auth": { "protocol": "sha256" },
                    "privacy": { "protocol": "aes256" } }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["credential_public"],
        json!({ "username": "monitor", "auth_protocol": "sha256", "privacy_protocol": "aes256" })
    );
    let Credential::SnmpV3 { auth, privacy, .. } = app.stored(id).await else { panic!() };
    assert_eq!(auth.unwrap().passphrase, "s3cr3t-auth-phrase");
    let privacy = privacy.expect("le chiffrement aurait dû être gardé");
    assert_eq!(privacy.protocol, SnmpV3PrivacyProtocol::Aes256);
    assert_eq!(privacy.passphrase, "s3cr3t-priv-phrase");
}

/// Garde-fou : quel que soit le type, ni la liste, ni la fiche, ni la réponse
/// d'une modification ne contiennent un secret (vérifié dans `request`).
#[tokio::test]
async fn aucune_reponse_ne_contient_un_secret() {
    let app = setup().await;
    let credentials = [
        json!({ "type": "snmp_community", "community": "s3cr3t-community" }),
        json!({ "type": "snmp_v3", "username": "monitor",
                "auth": { "protocol": "sha256", "passphrase": "s3cr3t-auth-phrase" },
                "privacy": { "protocol": "aes128", "passphrase": "s3cr3t-priv-phrase" } }),
        json!({ "type": "api_token", "token_id": "dumbmonit@pve!monitor",
                "secret": "s3cr3t-token-uuid" }),
        json!({ "type": "api_token", "token": "s3cr3t-bearer==" }),
        json!({ "type": "username_password", "username": "root@pam",
                "password": "s3cr3t-password" }),
    ];
    let mut ids = Vec::new();
    for (i, credential) in credentials.iter().enumerate() {
        let address = format!("10.0.1.{i}");
        let id = app.create(&format!("Cible {i}"), &address, credential.clone()).await;
        let (status, view) = app.request("GET", &format!("/api/targets/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(view["has_secret"], json!(true), "{view}");
        // Une modification sans identifiant renvoie la cible, toujours sans secret.
        let (status, _) = app
            .request(
                "PUT",
                &format!("/api/targets/{id}"),
                Some(json!({ "name": "Renommée", "address": address, "kind": "dummy" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        ids.push(id);
    }
    let (status, list) = app.request("GET", "/api/targets", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), ids.len());
    // Le jeton porteur n'est jamais coupé en « identifiant » et secret.
    let bearer = list.as_array().unwrap().iter().find(|t| t["id"] == json!(ids[3])).unwrap();
    assert_eq!(bearer["credential_public"], json!({}));
}
