//! Tests d'intégration des dossiers (`group_name`) et de l'ordre manuel
//! (`position`) des cibles — migration `0038_target_groups`, `PUT /api/targets/{id}`
//! et `POST /api/targets/reorder`.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dumbmonit_server::config::Config;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{api, collectors, db, tsdb};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

const UNREACHABLE_VICTORIA: &str = "http://127.0.0.1:1";
const SECRET: &str = "secret-de-test-suffisamment-long";
const PASSWORD: &str = "mot-de-passe-du-homelab";

struct TestApp {
    router: Router,
    _pool: SqlitePool,
    admin: String,
    _dir: tempfile::TempDir,
}

async fn setup() -> TestApp {
    let dir = tempfile::tempdir().expect("temporary directory");
    let mut config = Config::from_env().expect("default configuration");
    config.data_dir = dir.path().to_path_buf();
    config.victoria_url = Some(UNREACHABLE_VICTORIA.to_string());

    let pool = db::open(&config.database_path()).await.expect("database opened");
    let cipher = db::init_cipher(&pool, SECRET).await.expect("cipher initialised");

    let victoria = tsdb::Victoria::new(UNREACHABLE_VICTORIA).expect("client");
    let sink = tsdb::spawn_writer(victoria.clone(), Duration::from_secs(60));
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    let state = AppState::new(Inner {
        config,
        pool: pool.clone(),
        cipher,
        victoria,
        sink,
        collectors: registry,
    });

    let mut app =
        TestApp { router: api::router(state), _pool: pool, admin: String::new(), _dir: dir };
    let (status, body) = app
        .request(
            "POST",
            "/api/auth/setup",
            Some(json!({ "username": "admin", "password": PASSWORD })),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "admin creation: {body}");
    app.admin = app.login("admin", PASSWORD).await;
    app
}

impl TestApp {
    async fn login(&self, username: &str, password: &str) -> String {
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "username": username, "password": password }).to_string()))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let raw = response.headers().get(header::SET_COOKIE).expect("session cookie");
        raw.to_str().unwrap().split(';').next().unwrap().to_string()
    }

    async fn viewer(&self) -> String {
        let (status, body) = self
            .request(
                "POST",
                "/api/users",
                Some(json!({ "username": "viewer", "role": "viewer", "password": "mot-de-passe-du-lecteur" })),
                Some(&self.admin),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "viewer creation: {body}");
        self.login("viewer", "mot-de-passe-du-lecteur").await
    }

    async fn request(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
        cookie: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder =
                builder.header(header::COOKIE, cookie).header("x-requested-with", "DumbMonit");
        }
        let request = match body {
            Some(value) => builder
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let response = self.router.clone().oneshot(request).await.expect("response");
        let status = response.status();
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn create_target(&self, name: &str, address: &str) -> (i64, Value) {
        let (status, body) = self
            .request(
                "POST",
                "/api/targets",
                Some(json!({ "name": name, "address": address, "kind": "dummy" })),
                Some(&self.admin),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "target refused: {body}");
        (body["id"].as_i64().expect("target id"), body)
    }

    async fn get_target(&self, id: i64) -> Value {
        let (status, body) =
            self.request("GET", &format!("/api/targets/{id}"), None, Some(&self.admin)).await;
        assert_eq!(status, StatusCode::OK, "target read: {body}");
        body
    }
}

/// Une cible neuve n'a pas de dossier, et un rang cohérent avec l'insertion :
/// la colonne existe et la migration l'a bien appliquée.
#[tokio::test]
async fn une_cible_neuve_na_pas_de_dossier() {
    let app = setup().await;
    let (_, created) = app.create_target("switch", "10.0.0.1").await;
    assert_eq!(created["group_name"], json!(""));
    assert!(created["position"].is_i64(), "position manque : {created}");
}

/// `PUT` avec `group_name` range la cible dans un dossier, et le champ se relit
/// tel quel — sans échapper par une re-saisie du reste du formulaire.
#[tokio::test]
async fn ranger_une_cible_dans_un_dossier() {
    let app = setup().await;
    let (id, _) = app.create_target("switch", "10.0.0.1").await;

    let (status, body) = app
        .request(
            "PUT",
            &format!("/api/targets/{id}"),
            Some(json!({
                "name": "switch",
                "address": "10.0.0.1",
                "kind": "dummy",
                "group_name": "Rack A"
            })),
            Some(&app.admin),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "mise à jour refusée : {body}");
    assert_eq!(body["group_name"], json!("Rack A"));

    let reread = app.get_target(id).await;
    assert_eq!(reread["group_name"], json!("Rack A"));
}

/// Un nom de dossier trop long est une erreur de saisie, pas une panne.
#[tokio::test]
async fn un_nom_de_dossier_trop_long_est_refuse() {
    let app = setup().await;
    let (id, _) = app.create_target("switch", "10.0.0.1").await;
    let too_long = "x".repeat(61);

    let (status, body) = app
        .request(
            "PUT",
            &format!("/api/targets/{id}"),
            Some(json!({
                "name": "switch",
                "address": "10.0.0.1",
                "kind": "dummy",
                "group_name": too_long
            })),
            Some(&app.admin),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "devrait être refusé : {body}");
}

/// `POST /api/targets/reorder` réécrit le rang de chaque cible listée d'après
/// son indice dans `order` — et seulement celles-là.
#[tokio::test]
async fn reordonner_un_lot_de_cibles() {
    let app = setup().await;
    let (a, _) = app.create_target("a", "10.0.0.1").await;
    let (b, _) = app.create_target("b", "10.0.0.2").await;
    let (c, _) = app.create_target("c", "10.0.0.3").await;

    let (status, body) = app
        .request(
            "POST",
            "/api/targets/reorder",
            Some(json!({ "order": [c, a, b] })),
            Some(&app.admin),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "réordonnancement refusé : {body}");

    let a = app.get_target(a).await;
    let b = app.get_target(b).await;
    let c = app.get_target(c).await;
    assert_eq!(c["position"], json!(0));
    assert_eq!(a["position"], json!(1));
    assert_eq!(b["position"], json!(2));
}

/// Une liste vide ne veut rien dire : refusée plutôt que silencieusement sans effet.
#[tokio::test]
async fn reordonner_une_liste_vide_est_refuse() {
    let app = setup().await;
    let (status, body) = app
        .request("POST", "/api/targets/reorder", Some(json!({ "order": [] })), Some(&app.admin))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "devrait être refusé : {body}");
}

/// Un lecteur ne réordonne pas la rangée des autres.
#[tokio::test]
async fn un_lecteur_ne_peut_pas_reordonner() {
    let app = setup().await;
    let (a, _) = app.create_target("a", "10.0.0.1").await;
    let viewer = app.viewer().await;

    let (status, body) = app
        .request("POST", "/api/targets/reorder", Some(json!({ "order": [a] })), Some(&viewer))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "devrait être refusé : {body}");
}
