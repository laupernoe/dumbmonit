//! Provisionnement SCIM 2.0 : jeton dédié, cycles Entra ID et Okta, équipes et
//! membres, et les garde-fous — jamais un administrateur, jamais un membre
//! inscrit à la main, jamais un droit sur les dossiers.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{PASSWORD, TestApp};
use dumbmonit_server::db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

const USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
const GROUP_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
const PATCH_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
const ERROR_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:Error";
const LOCAL_PASSWORD: &str = "mot-de-passe-local-solide";

/// Réponse SCIM, en-têtes utiles compris.
struct Scim {
    status: StatusCode,
    content_type: Option<String>,
    www_authenticate: Option<String>,
    retry_after: Option<String>,
    location: Option<String>,
    body: Value,
}

async fn call(
    app: &TestApp,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Scim {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/scim+json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let text = |name: header::HeaderName| {
        response.headers().get(name).and_then(|value| value.to_str().ok()).map(str::to_string)
    };
    let content_type = text(header::CONTENT_TYPE);
    let www_authenticate = text(header::WWW_AUTHENTICATE);
    let retry_after = text(header::RETRY_AFTER);
    let location = text(header::LOCATION);
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Scim { status, content_type, www_authenticate, retry_after, location, body }
}

/// Une instance configurée, SCIM activé et son jeton.
struct Lab {
    app: TestApp,
    admin: String,
    token: String,
    pool: SqlitePool,
}

impl Lab {
    async fn new() -> Self {
        let app = TestApp::configured().await;
        let admin = app.admin_cookie().await;
        let reply = app.put("/api/auth/scim", json!({ "enabled": true }), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let reply = app.post("/api/auth/scim/token", json!({}), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let token = reply.body["token"].as_str().expect("jeton").to_string();
        let config = common::base_config(app._dir.path());
        let pool = db::open(&config.database_path()).await.expect("base");
        Self { app, admin, token, pool }
    }

    async fn get(&self, uri: &str) -> Scim {
        call(&self.app, "GET", uri, Some(&self.token), None).await
    }

    async fn post(&self, uri: &str, body: Value) -> Scim {
        call(&self.app, "POST", uri, Some(&self.token), Some(body)).await
    }

    async fn put(&self, uri: &str, body: Value) -> Scim {
        call(&self.app, "PUT", uri, Some(&self.token), Some(body)).await
    }

    async fn patch(&self, uri: &str, operations: Value) -> Scim {
        let body = json!({ "schemas": [PATCH_SCHEMA], "Operations": operations });
        call(&self.app, "PATCH", uri, Some(&self.token), Some(body)).await
    }

    async fn delete(&self, uri: &str) -> Scim {
        call(&self.app, "DELETE", uri, Some(&self.token), None).await
    }

    /// Crée un compte SCIM et rend son identifiant.
    async fn user(&self, username: &str) -> String {
        let reply = self
            .post("/api/scim/v2/Users", json!({ "schemas": [USER_SCHEMA], "userName": username }))
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
        reply.body["id"].as_str().expect("id").to_string()
    }

    /// Le compte tel que l'administration le voit.
    async fn account(&self, username: &str) -> Option<Value> {
        let reply = self.app.get("/api/users", Some(&self.admin)).await;
        assert_eq!(reply.status, StatusCode::OK);
        reply.body.as_array().expect("comptes").iter().find(|u| u["username"] == username).cloned()
    }

    /// Crée un compte local avec mot de passe.
    async fn local_user(&self, username: &str, role: &str) -> i64 {
        let body = json!({ "username": username, "role": role, "password": LOCAL_PASSWORD });
        let reply = self.app.post("/api/users", body, Some(&self.admin)).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
        reply.body["id"].as_i64().expect("id")
    }

    async fn scalar(&self, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await.expect(sql)
    }

    async fn audit_actions(&self) -> Vec<String> {
        let reply = self.app.get("/api/auth/audit", Some(&self.admin)).await;
        assert_eq!(reply.status, StatusCode::OK);
        reply
            .body
            .as_array()
            .expect("journal")
            .iter()
            .filter_map(|entry| entry["action"].as_str().map(str::to_string))
            .collect()
    }
}

fn names(list: &Value) -> Vec<String> {
    list["Resources"]
        .as_array()
        .expect("Resources")
        .iter()
        .map(|r| r["userName"].as_str().or(r["displayName"].as_str()).unwrap_or("").to_string())
        .collect()
}

fn member_ids(group: &Value) -> Vec<String> {
    group["members"]
        .as_array()
        .map(|members| {
            members.iter().filter_map(|m| m["value"].as_str().map(str::to_string)).collect()
        })
        .unwrap_or_default()
}

fn assert_scim_error(reply: &Scim, status: StatusCode, scim_type: Option<&str>) {
    assert_eq!(reply.status, status, "{}", reply.body);
    assert_eq!(reply.content_type.as_deref(), Some("application/scim+json"));
    assert_eq!(reply.body["schemas"], json!([ERROR_SCHEMA]), "{}", reply.body);
    assert_eq!(reply.body["status"], json!(status.as_u16().to_string()));
    if let Some(scim_type) = scim_type {
        assert_eq!(reply.body["scimType"], scim_type, "{}", reply.body);
    }
}

// ---------------------------------------------------------------------------
// Réglages et jeton
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_admin_settings_follow_the_contract() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply = app.get("/api/auth/scim", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(
        reply.body,
        json!({
            "enabled": false,
            "base_url": "http://localhost:8080/api/scim/v2",
            "token_set": false,
            "token_created_at": null,
            "default_role": "viewer",
        })
    );

    let body = json!({ "enabled": true, "default_role": "operator" });
    let reply = app.put("/api/auth/scim", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["enabled"], true);
    assert_eq!(reply.body["default_role"], "operator");

    // Jamais administrateur.
    let body = json!({ "enabled": true, "default_role": "admin" });
    let reply = app.put("/api/auth/scim", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    let reply = app.post("/api/auth/scim/token", json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let token = reply.body["token"].as_str().expect("jeton");
    assert!(token.starts_with("dmscim_") && token.len() > 60, "{token}");
    assert_eq!(reply.body["token_set"], true);
    assert!(reply.body["token_created_at"].is_string());

    // Le jeton n'est montré qu'une fois, et seule son empreinte est gardée.
    let reply = app.get("/api/auth/scim", Some(&admin)).await;
    assert!(reply.body.get("token").is_none(), "{}", reply.body);
    assert_eq!(reply.body["token_set"], true);
    let config = common::base_config(app._dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let stored: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT value FROM settings").fetch_all(&pool).await.unwrap();
    assert!(
        stored.iter().all(|value| !String::from_utf8_lossy(value).contains(token)),
        "le jeton en clair ne doit être nulle part en base"
    );

    let reply = app.delete("/api/auth/scim/token", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let reply = app.get("/api/auth/scim", Some(&admin)).await;
    assert_eq!(reply.body["token_set"], false);
    assert_eq!(reply.body["token_created_at"], Value::Null);

    let actions: Vec<String> = app
        .get("/api/auth/audit", Some(&admin))
        .await
        .body
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["action"].as_str().map(str::to_string))
        .collect();
    for action in ["scim.enabled", "scim.token.created", "scim.token.revoked"] {
        assert!(actions.iter().any(|a| a == action), "{action} absent : {actions:?}");
    }
}

#[tokio::test]
async fn only_an_admin_session_manages_scim() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    for (method, uri) in [
        ("GET", "/api/auth/scim"),
        ("PUT", "/api/auth/scim"),
        ("POST", "/api/auth/scim/token"),
        ("DELETE", "/api/auth/scim/token"),
    ] {
        let reply = app.request(method, uri, Some(json!({ "enabled": true })), Some(&viewer)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {uri} : {}", reply.body);
        let reply = app.request(method, uri, Some(json!({ "enabled": true })), None).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }

    // Un jeton d'API, même `write` et d'un administrateur, n'y touche pas.
    let created =
        app.post("/api/tokens", json!({ "name": "ci", "scope": "write" }), Some(&admin)).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let api_token = created.body["secret"].as_str().expect("secret").to_string();
    let reply = call(&app, "POST", "/api/auth/scim/token", Some(&api_token), None).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
}

#[tokio::test]
async fn scim_needs_its_own_token() {
    let lab = Lab::new().await;
    let app = &lab.app;
    let uri = "/api/scim/v2/Users";

    let reply = call(app, "GET", uri, None, None).await;
    assert_scim_error(&reply, StatusCode::UNAUTHORIZED, None);
    assert!(reply.www_authenticate.as_deref().is_some_and(|v| v.starts_with("Bearer")));

    let wrong = format!("dmscim_{}", "0".repeat(64));
    let reply = call(app, "GET", uri, Some(&wrong), None).await;
    assert_scim_error(&reply, StatusCode::UNAUTHORIZED, None);

    // Un jeton d'API n'est pas un jeton SCIM…
    let created =
        app.post("/api/tokens", json!({ "name": "ci", "scope": "write" }), Some(&lab.admin)).await;
    let api_token = created.body["secret"].as_str().expect("secret").to_string();
    let reply = call(app, "GET", uri, Some(&api_token), None).await;
    assert_scim_error(&reply, StatusCode::UNAUTHORIZED, None);

    // … ni une session…
    let reply = lab.app.get(uri, Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);

    // … et le jeton SCIM n'ouvre rien d'autre.
    let reply = call(app, "GET", "/api/targets", Some(&lab.token), None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    let reply = call(app, "GET", "/api/users", Some(&lab.token), None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);

    let reply = lab.get(uri).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.content_type.as_deref(), Some("application/scim+json"));

    // Une rotation invalide l'ancien jeton.
    let rotated = app.post("/api/auth/scim/token", json!({}), Some(&lab.admin)).await;
    let new_token = rotated.body["token"].as_str().expect("jeton").to_string();
    assert_ne!(new_token, lab.token);
    let reply = lab.get(uri).await;
    assert_scim_error(&reply, StatusCode::UNAUTHORIZED, None);
    let reply = call(app, "GET", uri, Some(&new_token), None).await;
    assert_eq!(reply.status, StatusCode::OK);

    // Révoqué : plus rien.
    let reply = app.delete("/api/auth/scim/token", Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let reply = call(app, "GET", uri, Some(&new_token), None).await;
    assert_scim_error(&reply, StatusCode::UNAUTHORIZED, None);
}

#[tokio::test]
async fn disabled_scim_answers_404_even_with_the_token() {
    let lab = Lab::new().await;
    let reply = lab.app.put("/api/auth/scim", json!({ "enabled": false }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    for uri in ["/api/scim/v2/Users", "/api/scim/v2/Groups", "/api/scim/v2/ServiceProviderConfig"] {
        let reply = lab.get(uri).await;
        assert_scim_error(&reply, StatusCode::NOT_FOUND, None);
    }
    let reply = lab.post("/api/scim/v2/Users", json!({ "userName": "x" })).await;
    assert_scim_error(&reply, StatusCode::NOT_FOUND, None);

    // Jamais activé : pareil.
    let fresh = TestApp::configured().await;
    let reply = call(&fresh, "GET", "/api/scim/v2/Users", Some("dmscim_x"), None).await;
    assert_scim_error(&reply, StatusCode::NOT_FOUND, None);
}

#[tokio::test]
async fn repeated_failures_are_rate_limited() {
    let lab = Lab::new().await;
    let wrong = format!("dmscim_{}", "f".repeat(64));
    for _ in 0..6 {
        let reply = call(&lab.app, "GET", "/api/scim/v2/Users", Some(&wrong), None).await;
        assert_scim_error(&reply, StatusCode::UNAUTHORIZED, None);
    }
    let reply = lab.get("/api/scim/v2/Users").await;
    assert_scim_error(&reply, StatusCode::TOO_MANY_REQUESTS, None);
    assert!(reply.retry_after.is_some());
}

#[tokio::test]
async fn the_discovery_documents_are_served() {
    let lab = Lab::new().await;
    let config = lab.get("/api/scim/v2/ServiceProviderConfig").await;
    assert_eq!(config.status, StatusCode::OK);
    assert_eq!(config.body["patch"]["supported"], true);
    assert_eq!(config.body["filter"]["supported"], true);
    assert_eq!(config.body["changePassword"]["supported"], false);
    assert_eq!(config.body["authenticationSchemes"][0]["type"], "oauthbearertoken");

    let types = lab.get("/api/scim/v2/ResourceTypes").await;
    assert_eq!(types.status, StatusCode::OK);
    assert_eq!(types.body["totalResults"], 2);
    let schemas = lab.get("/api/scim/v2/Schemas").await;
    assert_eq!(schemas.status, StatusCode::OK);
    let ids: Vec<&str> = schemas.body["Resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["id"].as_str())
        .collect();
    assert_eq!(ids, [USER_SCHEMA, GROUP_SCHEMA]);
}

// ---------------------------------------------------------------------------
// Comptes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_entra_id_user_lifecycle() {
    let lab = Lab::new().await;

    // Entra ID cherche d'abord le compte.
    let reply = lab.get("/api/scim/v2/Users?filter=userName+eq+%22jane%40contoso.com%22").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["totalResults"], 0);
    assert_eq!(reply.body["Resources"], json!([]));

    let reply = lab
        .post(
            "/api/scim/v2/Users",
            json!({
                "schemas": [USER_SCHEMA, "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"],
                "externalId": "jane",
                "userName": "jane@contoso.com",
                "active": true,
                "displayName": "Jane Doe",
                "emails": [{ "primary": true, "type": "work", "value": "jane@contoso.com" }],
                "name": { "formatted": "Jane Doe", "familyName": "Doe", "givenName": "Jane" },
                "roles": [],
            }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.content_type.as_deref(), Some("application/scim+json"));
    let id = reply.body["id"].as_str().expect("id").to_string();
    assert_eq!(id.len(), 36, "un UUID : {id}");
    assert_eq!(reply.body["userName"], "jane@contoso.com");
    assert_eq!(reply.body["displayName"], "Jane Doe");
    assert_eq!(reply.body["active"], true);
    assert_eq!(reply.body["externalId"], "jane");
    assert_eq!(reply.body["emails"][0]["value"], "jane@contoso.com");
    assert_eq!(reply.body["meta"]["resourceType"], "User");
    let location = format!("http://localhost:8080/api/scim/v2/Users/{id}");
    assert_eq!(reply.body["meta"]["location"], location);
    assert_eq!(reply.location.as_deref(), Some(location.as_str()));

    let account = lab.account("jane@contoso.com").await.expect("compte créé");
    assert_eq!(account["role"], "viewer");
    assert_eq!(account["display_name"], "Jane Doe");
    assert_ne!(account["auth"], "password", "SCIM ne pose jamais de mot de passe");

    // Les filtres d'Entra ID, insensibles à la casse de l'attribut et de `eq`.
    for filter in [
        r#"userName eq "JANE@contoso.com""#,
        r#"USERNAME EQ "jane@contoso.com""#,
        r#"externalId eq "jane""#,
        r#"emails[type eq "work"].value eq "jane@contoso.com""#,
    ] {
        let uri = format!("/api/scim/v2/Users?filter={}", urlencode(filter));
        let reply = lab.get(&uri).await;
        assert_eq!(reply.status, StatusCode::OK, "{filter} : {}", reply.body);
        assert_eq!(reply.body["totalResults"], 1, "{filter}");
        assert_eq!(reply.body["Resources"][0]["id"], id.as_str());
    }
    let uri = format!("/api/scim/v2/Users?filter={}", urlencode(r#"externalId eq "JANE""#));
    assert_eq!(lab.get(&uri).await.body["totalResults"], 0, "externalId est sensible à la casse");

    // Mise à jour à la manière d'Entra ID : `Replace`, chemins variés,
    // attributs inconnus tolérés.
    let uri = format!("/api/scim/v2/Users/{id}");
    let reply = lab
        .patch(
            &uri,
            json!([
                { "op": "Replace", "path": "displayName", "value": "Jane D." },
                { "op": "Add", "path": "name.givenName", "value": "J" },
                { "op": "Replace", "path": "emails[type eq \"work\"].value", "value": "jd@contoso.com" },
                {
                    "op": "Add",
                    "path": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User:department",
                    "value": "IT",
                },
                { "op": "Replace", "path": "title", "value": "Engineer" },
            ]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["displayName"], "Jane D.");
    assert_eq!(reply.body["emails"][0]["value"], "jd@contoso.com");

    // Désactivation : `active` en chaîne `"False"`.
    let reply =
        lab.patch(&uri, json!([{ "op": "Replace", "path": "active", "value": "False" }])).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["active"], false);
    assert_eq!(lab.account("jane@contoso.com").await.unwrap()["disabled"], true);

    let uri_active = format!("/api/scim/v2/Users?filter={}", urlencode("active eq false"));
    assert_eq!(lab.get(&uri_active).await.body["totalResults"], 1);

    // Réactivation : sans chemin, objet de valeurs.
    let reply = lab.patch(&uri, json!([{ "op": "replace", "value": { "active": "True" } }])).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["active"], true);

    let reply = lab.delete(&uri).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(lab.account("jane@contoso.com").await.is_none());
    let reply = lab.get(&uri).await;
    assert_scim_error(&reply, StatusCode::NOT_FOUND, None);
    let reply = lab.delete(&uri).await;
    assert_scim_error(&reply, StatusCode::NOT_FOUND, None);

    let actions = lab.audit_actions().await;
    for action in
        ["scim.user.created", "scim.user.updated", "scim.user.disabled", "scim.user.deleted"]
    {
        assert!(actions.iter().any(|a| a == action), "{action} absent : {actions:?}");
    }
}

#[tokio::test]
async fn an_okta_user_lifecycle() {
    let lab = Lab::new().await;
    let user = json!({
        "schemas": [USER_SCHEMA],
        "userName": "bob@okta.example",
        "name": { "givenName": "Bob", "familyName": "Smith" },
        "emails": [{ "primary": true, "value": "bob@okta.example", "type": "work" }],
        "locale": "en-US",
        "externalId": "00u1",
        "groups": [],
        "password": "N0t-Stored-Anywhere!",
        "active": true,
    });
    let reply = lab.post("/api/scim/v2/Users", user.clone()).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let id = reply.body["id"].as_str().unwrap().to_string();
    // Sans `displayName`, le nom vient de `name`.
    assert_eq!(reply.body["displayName"], "Bob Smith");
    let account = lab.account("bob@okta.example").await.unwrap();
    assert_ne!(account["auth"], "password", "le mot de passe d'Okta n'est jamais lu");

    // Okta remplace le compte entier.
    let mut replaced = user.clone();
    replaced["id"] = json!(id);
    replaced["displayName"] = json!("Robert Smith");
    replaced["emails"] =
        json!([{ "primary": true, "value": "robert@okta.example", "type": "work" }]);
    let uri = format!("/api/scim/v2/Users/{id}");
    let reply = lab.put(&uri, replaced).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["displayName"], "Robert Smith");
    assert_eq!(reply.body["emails"][0]["value"], "robert@okta.example");

    // Désactivation à la manière d'Okta : booléen, sans chemin.
    let reply = lab.patch(&uri, json!([{ "op": "replace", "value": { "active": false } }])).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["active"], false);

    let filter = urlencode(r#"userName eq "bob@okta.example""#);
    let reply =
        lab.get(&format!("/api/scim/v2/Users?filter={filter}&startIndex=1&count=100")).await;
    assert_eq!(reply.body["totalResults"], 1);
    assert_eq!(reply.body["Resources"][0]["active"], false);
    assert_eq!(lab.account("bob@okta.example").await.unwrap()["disabled"], true);

    let mut active = user.clone();
    active["active"] = json!(true);
    let reply = lab.put(&uri, active).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["active"], true);
}

#[tokio::test]
async fn the_default_role_applies_to_created_accounts() {
    let lab = Lab::new().await;
    let body = json!({ "enabled": true, "default_role": "operator" });
    assert_eq!(lab.app.put("/api/auth/scim", body, Some(&lab.admin)).await.status, StatusCode::OK);
    lab.user("olga").await;
    assert_eq!(lab.account("olga").await.unwrap()["role"], "operator");

    // Une requête ne choisit pas son rôle.
    let body = json!({ "userName": "mallory", "roles": [{ "value": "admin" }], "role": "admin" });
    let reply = lab.post("/api/scim/v2/Users", body).await;
    assert_eq!(reply.status, StatusCode::CREATED);
    assert_eq!(lab.account("mallory").await.unwrap()["role"], "operator");
}

#[tokio::test]
async fn an_administrator_is_never_touched() {
    let lab = Lab::new().await;

    // L'administrateur existant n'est pas rattaché.
    let reply = lab.post("/api/scim/v2/Users", json!({ "userName": "admin" })).await;
    assert_scim_error(&reply, StatusCode::CONFLICT, Some("uniqueness"));
    let reply =
        lab.post("/api/scim/v2/Users", json!({ "userName": "ADMIN", "active": false })).await;
    assert_scim_error(&reply, StatusCode::CONFLICT, Some("uniqueness"));
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM users WHERE scim_id IS NOT NULL").await, 0);
    assert!(lab.app.login_as("admin", PASSWORD).await.status.is_success());
    let reply = lab.get("/api/scim/v2/Users?filter=userName%20eq%20%22admin%22").await;
    assert_eq!(reply.body["totalResults"], 0, "un compte non provisionné reste invisible");

    // Un compte provisionné puis promu administrateur devient intouchable.
    let id = lab.user("eve").await;
    let local_id = lab.account("eve").await.unwrap()["id"].as_i64().unwrap();
    let reply = lab
        .app
        .put(&format!("/api/users/{local_id}"), json!({ "role": "admin" }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    let uri = format!("/api/scim/v2/Users/{id}");
    let reply =
        lab.patch(&uri, json!([{ "op": "replace", "path": "active", "value": false }])).await;
    assert_scim_error(&reply, StatusCode::FORBIDDEN, None);
    let reply = lab.put(&uri, json!({ "userName": "eve", "active": false })).await;
    assert_scim_error(&reply, StatusCode::FORBIDDEN, None);
    let reply = lab.put(&uri, json!({ "userName": "eve2" })).await;
    assert_scim_error(&reply, StatusCode::FORBIDDEN, None);
    let reply = lab.delete(&uri).await;
    assert_scim_error(&reply, StatusCode::FORBIDDEN, None);
    let account = lab.account("eve").await.expect("toujours là");
    assert_eq!(account["role"], "admin");
    assert_eq!(account["disabled"], false);

    // Ni inscrit dans une équipe.
    let reply = lab
        .post(
            "/api/scim/v2/Groups",
            json!({ "displayName": "Admins", "members": [{ "value": id }] }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(member_ids(&reply.body), Vec::<String>::new());
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_members").await, 0);
}

#[tokio::test]
async fn an_existing_local_account_is_linked_but_never_an_admin() {
    let lab = Lab::new().await;
    lab.local_user("carol", "viewer").await;
    let carol = lab.app.login_as("carol", LOCAL_PASSWORD).await.cookie();
    assert_eq!(lab.app.get("/api/auth/me", Some(&carol)).await.status, StatusCode::OK);

    let reply = lab
        .post("/api/scim/v2/Users", json!({ "userName": "Carol", "displayName": "Carol Smith" }))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["userName"], "carol");
    let id = reply.body["id"].as_str().unwrap().to_string();
    let account = lab.account("carol").await.unwrap();
    assert_eq!(account["role"], "viewer", "le rôle du compte rattaché reste le sien");
    assert_eq!(account["display_name"], "Carol Smith");

    // Rattaché une seule fois.
    let reply = lab.post("/api/scim/v2/Users", json!({ "userName": "carol" })).await;
    assert_scim_error(&reply, StatusCode::CONFLICT, Some("uniqueness"));

    // Désactivé par SCIM : ses sessions sont fermées.
    let uri = format!("/api/scim/v2/Users/{id}");
    let reply =
        lab.patch(&uri, json!([{ "op": "Replace", "path": "active", "value": "False" }])).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(lab.app.get("/api/auth/me", Some(&carol)).await.status, StatusCode::UNAUTHORIZED);
    assert!(!lab.app.login_as("carol", LOCAL_PASSWORD).await.status.is_success());

    // Un administrateur local, même désactivé, n'est jamais rattaché.
    lab.local_user("root2", "admin").await;
    let reply = lab.post("/api/scim/v2/Users", json!({ "userName": "root2" })).await;
    assert_scim_error(&reply, StatusCode::CONFLICT, Some("uniqueness"));
    assert_eq!(
        lab.scalar("SELECT COUNT(*) FROM users WHERE username = 'root2' AND scim_id IS NULL").await,
        1
    );

    // Un compte désactivé par un administrateur le reste, quoi que dise SCIM.
    let dave = lab.local_user("dave", "viewer").await;
    let reply = lab
        .app
        .put(&format!("/api/users/{dave}"), json!({ "disabled": true }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let reply = lab.post("/api/scim/v2/Users", json!({ "userName": "dave", "active": true })).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["active"], false);
    let uri = format!("/api/scim/v2/Users/{}", reply.body["id"].as_str().unwrap());
    let reply =
        lab.patch(&uri, json!([{ "op": "replace", "path": "active", "value": true }])).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["active"], false);
    assert_eq!(lab.account("dave").await.unwrap()["disabled"], true);
}

#[tokio::test]
async fn usernames_stay_unique_and_valid() {
    let lab = Lab::new().await;
    let first = lab.user("anna").await;
    lab.user("bert").await;
    let reply = lab.post("/api/scim/v2/Users", json!({ "userName": "ANNA" })).await;
    assert_scim_error(&reply, StatusCode::CONFLICT, Some("uniqueness"));
    let reply = lab
        .patch(
            &format!("/api/scim/v2/Users/{first}"),
            json!([{ "op": "replace", "path": "userName", "value": "bert" }]),
        )
        .await;
    assert_scim_error(&reply, StatusCode::CONFLICT, Some("uniqueness"));

    for body in [
        json!({ "displayName": "No user name" }),
        json!({ "userName": "has space" }),
        json!({ "userName": "a".repeat(65) }),
        json!({ "userName": "ok", "active": "maybe" }),
        json!({ "userName": "ok", "emails": [{ "value": "not an email" }] }),
    ] {
        let reply = lab.post("/api/scim/v2/Users", body.clone()).await;
        assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("invalidValue"));
    }

    let reply = call(
        &lab.app,
        "POST",
        "/api/scim/v2/Users",
        Some(&lab.token),
        Some(Value::String("{not json".into())),
    )
    .await;
    assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("invalidSyntax"));

    let uri = format!("/api/scim/v2/Users/{first}");
    let reply = lab.patch(&uri, json!([{ "op": "remove" }])).await;
    assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("noTarget"));
    let reply = lab.patch(&uri, json!([{ "op": "move", "path": "active" }])).await;
    assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("invalidSyntax"));
    let reply = lab.patch(&uri, json!([{ "op": "remove", "path": "userName" }])).await;
    assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("mutability"));
}

#[tokio::test]
async fn lists_are_paginated() {
    let lab = Lab::new().await;
    for name in ["u1", "u2", "u3", "u4", "u5"] {
        lab.user(name).await;
    }
    let reply = lab.get("/api/scim/v2/Users?startIndex=2&count=2").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["totalResults"], 5);
    assert_eq!(reply.body["startIndex"], 2);
    assert_eq!(reply.body["itemsPerPage"], 2);
    assert_eq!(names(&reply.body), ["u2", "u3"]);

    let reply = lab.get("/api/scim/v2/Users?count=0").await;
    assert_eq!(reply.body["totalResults"], 5);
    assert_eq!(reply.body["itemsPerPage"], 0);

    let reply = lab.get("/api/scim/v2/Users?startIndex=0&count=-3").await;
    assert_eq!(reply.body["startIndex"], 1);
    assert_eq!(reply.body["itemsPerPage"], 0);

    let reply = lab.get("/api/scim/v2/Users?startIndex=5").await;
    assert_eq!(names(&reply.body), ["u5"]);
    let reply = lab.get("/api/scim/v2/Users?startIndex=9").await;
    assert_eq!(reply.body["totalResults"], 5);
    assert_eq!(reply.body["Resources"], json!([]));

    let reply = lab.get("/api/scim/v2/Users?count=many").await;
    assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("invalidValue"));
}

#[tokio::test]
async fn unsupported_filters_are_invalid_filters() {
    let lab = Lab::new().await;
    for filter in [
        r#"userName co "a""#,
        r#"userName sw "a""#,
        r#"userName eq "a" or userName eq "b""#,
        r#"not (userName eq "a")"#,
        r#"title eq "x""#,
        "userName eq",
        r#"userName eq "unterminated"#,
        "active eq \"yes\"",
    ] {
        let reply = lab.get(&format!("/api/scim/v2/Users?filter={}", urlencode(filter))).await;
        assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("invalidFilter"));
    }
    for filter in [r#"displayName sw "N""#, r#"members[value eq "x"].display eq "y""#, "id pr"] {
        let reply = lab.get(&format!("/api/scim/v2/Groups?filter={}", urlencode(filter))).await;
        assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("invalidFilter"));
    }
}

// ---------------------------------------------------------------------------
// Équipes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn groups_and_their_members() {
    let lab = Lab::new().await;
    let u1 = lab.user("u1").await;
    let u2 = lab.user("u2").await;

    let reply = lab
        .post(
            "/api/scim/v2/Groups",
            json!({
                "schemas": [GROUP_SCHEMA],
                "displayName": "NOC",
                "externalId": "g1",
                "members": [{ "value": u1 }],
            }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let gid = reply.body["id"].as_str().unwrap().to_string();
    assert_eq!(reply.body["displayName"], "NOC");
    assert_eq!(member_ids(&reply.body), [u1.clone()]);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_members WHERE source = 'scim'").await, 1);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_folder_grants").await, 0, "aucun droit");

    // Filtres et exclusion des membres, comme Entra ID.
    let filter = urlencode(r#"displayName eq "noc""#);
    let reply =
        lab.get(&format!("/api/scim/v2/Groups?filter={filter}&excludedAttributes=members")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["totalResults"], 1);
    assert!(reply.body["Resources"][0].get("members").is_none(), "{}", reply.body);
    let reply = lab
        .get(&format!("/api/scim/v2/Groups?filter={}", urlencode(r#"externalId eq "g1""#)))
        .await;
    assert_eq!(reply.body["totalResults"], 1);
    let with_u1 = format!(r#"id eq "{gid}" and members eq "{u1}""#);
    let reply = lab.get(&format!("/api/scim/v2/Groups?filter={}", urlencode(&with_u1))).await;
    assert_eq!(reply.body["totalResults"], 1);
    let with_u2 = format!(r#"id eq "{gid}" and members[value eq "{u2}"]"#);
    let reply = lab.get(&format!("/api/scim/v2/Groups?filter={}", urlencode(&with_u2))).await;
    assert_eq!(reply.body["totalResults"], 0);

    let uri = format!("/api/scim/v2/Groups/{gid}");
    let reply = lab.get(&format!("{uri}?excludedAttributes=members")).await;
    assert!(reply.body.get("members").is_none());

    // Ajout, retrait par filtre de valeur, retrait par liste.
    let reply = lab
        .patch(&uri, json!([{ "op": "Add", "path": "members", "value": [{ "value": u2 }] }]))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(member_ids(&lab.get(&uri).await.body), [u1.clone(), u2.clone()]);

    let path = format!(r#"members[value eq "{u1}"]"#);
    let reply = lab.patch(&uri, json!([{ "op": "Remove", "path": path }])).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(member_ids(&lab.get(&uri).await.body), [u2.clone()]);

    let reply = lab
        .patch(&uri, json!([{ "op": "Remove", "path": "members", "value": [{ "value": u2 }] }]))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(member_ids(&lab.get(&uri).await.body), Vec::<String>::new());

    // Renommage sans chemin (Okta) et remplacement complet.
    let reply = lab
        .patch(
            &uri,
            json!([{ "op": "replace", "value": { "id": gid, "displayName": "NOC Team" } }]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let reply =
        lab.put(&uri, json!({ "displayName": "NOC Team", "members": [{ "value": u1 }] })).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["displayName"], "NOC Team");
    assert_eq!(member_ids(&reply.body), [u1.clone()]);

    // Un membre inconnu est refusé.
    let reply = lab
        .patch(&uri, json!([{ "op": "add", "path": "members", "value": [{ "value": "nope" }] }]))
        .await;
    assert_scim_error(&reply, StatusCode::BAD_REQUEST, Some("invalidValue"));

    // Deux équipes ne portent pas le même nom.
    let reply = lab.post("/api/scim/v2/Groups", json!({ "displayName": "noc team" })).await;
    assert_scim_error(&reply, StatusCode::CONFLICT, Some("uniqueness"));

    // Suppression : l'équipe créée par SCIM disparaît.
    let reply = lab.delete(&uri).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM teams").await, 0);
    assert_scim_error(&lab.get(&uri).await, StatusCode::NOT_FOUND, None);

    let actions = lab.audit_actions().await;
    for action in ["scim.group.created", "scim.group.updated", "scim.group.deleted"] {
        assert!(actions.iter().any(|a| a == action), "{action} absent : {actions:?}");
    }
}

#[tokio::test]
async fn a_linked_team_keeps_its_name_rights_and_manual_members() {
    let lab = Lab::new().await;
    let local = lab.local_user("frank", "viewer").await;
    sqlx::query("INSERT INTO teams (name) VALUES ('Ops')").execute(&lab.pool).await.unwrap();
    let team: i64 = sqlx::query_scalar("SELECT id FROM teams WHERE name = 'Ops'")
        .fetch_one(&lab.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO team_members (team_id, user_id, source) VALUES (?, ?, 'manual')")
        .bind(team)
        .bind(local)
        .execute(&lab.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO team_folder_grants (team_id, folder, level) VALUES (?, 'Paris', 'read')",
    )
    .bind(team)
    .execute(&lab.pool)
    .await
    .unwrap();

    // Le fournisseur rattache l'équipe existante (nom sans casse) et y inscrit
    // un compte.
    let user = lab.user("grace").await;
    let reply = lab
        .post(
            "/api/scim/v2/Groups",
            json!({ "displayName": "ops", "members": [{ "value": user }] }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let uri = format!("/api/scim/v2/Groups/{}", reply.body["id"].as_str().unwrap());
    assert_eq!(reply.body["displayName"], "Ops");

    // Le membre manuel n'est jamais retiré, même par un remplacement complet.
    let reply = lab.patch(&uri, json!([{ "op": "replace", "path": "members", "value": [] }])).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let reply = lab.patch(&uri, json!([{ "op": "remove", "path": "members" }])).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_members WHERE source = 'manual'").await, 1);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_members WHERE source = 'scim'").await, 0);

    // Le nom donné par l'administrateur reste.
    let reply =
        lab.put(&uri, json!({ "displayName": "Renamed", "members": [{ "value": user }] })).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["displayName"], "Ops");

    // Supprimée côté fournisseur : détachée seulement.
    let reply = lab.delete(&uri).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(
        lab.scalar("SELECT COUNT(*) FROM teams WHERE name = 'Ops' AND scim_id IS NULL").await,
        1
    );
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_folder_grants").await, 1);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_members WHERE source = 'manual'").await, 1);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_members WHERE source = 'scim'").await, 0);
    assert_scim_error(&lab.get(&uri).await, StatusCode::NOT_FOUND, None);
    assert!(lab.audit_actions().await.iter().any(|a| a == "scim.group.unlinked"));
}

#[tokio::test]
async fn deleting_a_user_removes_it_from_its_teams() {
    let lab = Lab::new().await;
    let user = lab.user("henry").await;
    let reply = lab
        .post(
            "/api/scim/v2/Groups",
            json!({ "displayName": "Night", "members": [{ "value": user }] }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let group = format!("/api/scim/v2/Groups/{}", reply.body["id"].as_str().unwrap());
    let reply = lab.delete(&format!("/api/scim/v2/Users/{user}")).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(lab.scalar("SELECT COUNT(*) FROM team_members").await, 0);
    let reply = lab.get(&group).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(member_ids(&reply.body), Vec::<String>::new());
}

/// Encodage d'URL minimal pour les filtres des tests.
fn urlencode(raw: &str) -> String {
    let mut out = String::new();
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
