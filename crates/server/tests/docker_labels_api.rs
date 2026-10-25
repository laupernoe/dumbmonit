//! Moniteurs créés par les étiquettes Docker d'un agent : création, mise à jour,
//! archivage, adoption, verrou d'édition et libération, par l'API réelle.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

fn monitor(key: &str, url: &str) -> Value {
    json!({
        "key": key, "kind": "http", "address": url, "name": "Gitea",
        "folder": "Apps", "tags": { "env": "prod" }, "interval_secs": 30
    })
}

fn batch(declared: Option<Value>) -> Value {
    let mut batch = json!({
        "protocol": 1,
        "identity": {
            "hostname": "nas", "os": "linux", "agent_version": "0.9.0",
            "machine_id": "id-nas", "binding_supported": true
        },
        "sent_at_ms": 0,
        "samples": [{ "metric": "cpu_usage_percent", "value": 1.0, "kind": "gauge",
                      "ts_ms": chrono::Utc::now().timestamp_millis(), "labels": {} }],
    });
    if let Some(declared) = declared {
        batch["declared_monitors"] = declared;
    }
    batch
}

async fn push(app: &TestApp, enrolment: &str, secret: Option<&str>, body: &Value) -> Reply {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        builder = builder.header("x-dumbmonit-agent-secret", secret);
    }
    let response =
        app.router.clone().oneshot(builder.body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie: None,
        location: None,
    }
}

struct Lab {
    app: TestApp,
    admin: String,
    enrolment: String,
    secret: String,
    host: i64,
}

impl Lab {
    async fn new() -> Self {
        let app = TestApp::configured().await;
        let admin = app.admin_cookie().await;
        let reply = app.post("/api/agent/tokens", json!({ "name": "parc" }), Some(&admin)).await;
        let enrolment = reply.body["secret"].as_str().unwrap().to_string();
        let first = push(&app, &enrolment, None, &batch(Some(json!([])))).await;
        assert_eq!(first.status, StatusCode::OK, "{}", first.body);
        let secret = first.body["agent_secret"].as_str().unwrap().to_string();
        let host = first.body["target_id"].as_i64().unwrap();
        Self { app, admin, enrolment, secret, host }
    }

    async fn declare(&self, declared: Value) {
        let reply = push(&self.app, &self.enrolment, Some(&self.secret), &batch(Some(declared))).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    }

    async fn http_targets(&self) -> Vec<Value> {
        let reply = self.app.get("/api/targets", Some(&self.admin)).await;
        reply.body.as_array().unwrap().iter().filter(|t| t["kind"] == "http").cloned().collect()
    }
}

#[tokio::test]
async fn un_conteneur_etiquete_cree_un_moniteur_sous_sa_machine() {
    let lab = Lab::new().await;
    lab.declare(json!([monitor("gitea/http", "http://{host}:3000/healthz")])).await;
    let list = lab.http_targets().await;
    assert_eq!(list.len(), 1);
    let m = &list[0];
    assert_eq!(m["address"], "http://nas:3000/healthz");
    assert_eq!(m["parent_id"], lab.host);
    assert_eq!(m["group_name"], "Apps");
    assert_eq!(m["interval_secs"], 30);
    assert_eq!(m["managed"]["by"], "docker-labels");
    assert_eq!(m["managed"]["key"], "gitea/http");
    assert!(m["managed"]["missing_since"].is_null());
}

#[tokio::test]
async fn les_etiquettes_mettent_a_jour_et_archivent() {
    let lab = Lab::new().await;
    lab.declare(json!([monitor("gitea/http", "http://nas:3000/a")])).await;
    let id = lab.http_targets().await[0]["id"].as_i64().unwrap();

    // Nouvelle adresse : même moniteur, mis à jour.
    let mut changed = monitor("gitea/http", "http://nas:3000/b");
    changed["interval_secs"] = json!(120);
    lab.declare(json!([changed])).await;
    let list = lab.http_targets().await;
    assert_eq!((list.len(), list[0]["id"].as_i64().unwrap()), (1, id));
    assert_eq!(list[0]["address"], "http://nas:3000/b");
    assert_eq!(list[0]["interval_secs"], 120);

    // Plus d'étiquette : archivé (désactivé, daté), pas supprimé.
    lab.declare(json!([])).await;
    let archived = &lab.http_targets().await[0];
    assert_eq!(archived["enabled"], false);
    assert!(archived["managed"]["missing_since"].is_string());

    // Un lot sans liste (Docker illisible) ne touche à rien.
    let before = archived.clone();
    let reply = push(&lab.app, &lab.enrolment, Some(&lab.secret), &batch(None)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(lab.http_targets().await[0]["managed"], before["managed"]);

    // Le conteneur revient : le moniteur se réactive.
    lab.declare(json!([monitor("gitea/http", "http://nas:3000/b")])).await;
    let back = &lab.http_targets().await[0];
    assert_eq!(back["enabled"], true);
    assert!(back["managed"]["missing_since"].is_null());
}

#[tokio::test]
async fn un_moniteur_gere_ne_se_modifie_ni_ne_se_supprime_a_la_main() {
    let lab = Lab::new().await;
    lab.declare(json!([monitor("gitea/http", "http://nas:3000/a")])).await;
    let target = lab.http_targets().await[0].clone();
    let id = target["id"].as_i64().unwrap();
    let uri = format!("/api/targets/{id}");

    let edit = json!({
        "name": "Renamed", "address": target["address"], "kind": "http",
        "interval_secs": 30, "tags": target["tags"], "group_name": "Apps",
        "parent_id": lab.host
    });
    let refused = lab.app.put(&uri, edit, Some(&lab.admin)).await;
    assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);

    // La pause, seule, reste permise.
    let pause = json!({
        "name": "Gitea", "address": target["address"], "kind": "http", "enabled": false,
        "interval_secs": 30, "tags": target["tags"], "group_name": "Apps",
        "parent_id": lab.host
    });
    let paused = lab.app.put(&uri, pause, Some(&lab.admin)).await;
    assert_eq!(paused.status, StatusCode::OK, "{}", paused.body);

    let refused = lab.app.delete(&uri, Some(&lab.admin)).await;
    assert_eq!(refused.status, StatusCode::CONFLICT);

    // Libéré, il redevient un moniteur ordinaire.
    let released = lab.app.post(&format!("{uri}/release"), json!({}), Some(&lab.admin)).await;
    assert_eq!(released.status, StatusCode::NO_CONTENT, "{}", released.body);
    assert!(lab.http_targets().await[0]["managed"].is_null());
    let gone = lab.app.delete(&uri, Some(&lab.admin)).await;
    assert_eq!(gone.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn l_adoption_reprend_un_moniteur_existant_seulement_si_demandee() {
    let lab = Lab::new().await;
    let manual = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "Mine", "address": "http://nas:3000/a", "kind": "http" }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(manual.status, StatusCode::CREATED, "{}", manual.body);
    let id = manual.body["id"].as_i64().unwrap();

    // Sans `adopt`, le doublon est refusé et le moniteur reste libre.
    lab.declare(json!([monitor("gitea/http", "http://nas:3000/a")])).await;
    let list = lab.http_targets().await;
    assert_eq!(list.len(), 1);
    assert!(list[0]["managed"].is_null());
    assert_eq!(list[0]["name"], "Mine");

    // Avec `adopt`, les étiquettes le prennent en charge.
    let mut adopt = monitor("gitea/http", "http://nas:3000/a");
    adopt["adopt"] = json!(true);
    // L'empreinte de la liste a changé (adopt), la réconciliation repart.
    lab.declare(json!([adopt])).await;
    let list = lab.http_targets().await;
    assert_eq!(list[0]["id"].as_i64().unwrap(), id);
    assert_eq!(list[0]["managed"]["key"], "gitea/http");
    assert_eq!(list[0]["name"], "Gitea");
}

#[tokio::test]
async fn un_type_non_pris_en_charge_est_ecarte() {
    let lab = Lab::new().await;
    let mut odd = monitor("x/ssh", "nas:22");
    odd["kind"] = json!("ssh");
    lab.declare(json!([odd])).await;
    assert!(lab.http_targets().await.is_empty());
}
