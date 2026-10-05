//! Tests d'intégration de la liaison d'un agent à sa machine.
//!
//! Le scénario que ces tests ferment : un parc partage un jeton
//! d'enregistrement, une machine est compromise, et son occupant se sert de la
//! clé d'identité d'une voisine — un `/etc/machine-id` qui se lit en une
//! seconde — pour pousser de fausses mesures en son nom et surtout venir
//! chercher les commandes Docker qui lui étaient destinées.
//!
//! Tout passe par l'API réelle : c'est ce que ferait l'attaquant.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::routing::get;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_HEADER: &str = "x-dumbmonit-agent-secret";

/// Un VictoriaMetrics factice qui déclare un conteneur `web` sur toute cible.
///
/// L'API refuse une commande pour un conteneur absent de l'inventaire ; il faut
/// donc un inventaire pour pouvoir en déposer une et vérifier qui a le droit de
/// venir la chercher.
async fn fake_victoria() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new().route(
        "/api/v1/query",
        get(|| async {
            axum::Json(json!({
                "status": "success",
                "data": {
                    "resultType": "vector",
                    "result": [
                        { "metric": { "__name__": "dumbmonit_container_up", "container": "web",
                                      "image": "nginx:1.27" }, "value": [1.0, "1"] }
                    ]
                }
            }))
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("faux VictoriaMetrics");
    });
    format!("http://{address}")
}

/// Instance branchée sur le faux inventaire, avec un administrateur.
async fn setup_with_inventory() -> TestApp {
    let victoria = fake_victoria().await;
    let app = common::setup_with(move |config| config.victoria_url = Some(victoria)).await;
    app.create_admin().await;
    app
}

/// Crée un jeton d'enregistrement et rend son secret en clair.
async fn token(app: &TestApp, admin: &str, scope: Value) -> String {
    let mut body = json!({ "name": "parc" });
    if let Value::Object(fields) = scope {
        for (key, value) in fields {
            body[key] = value;
        }
    }
    let reply = app.post("/api/agent/tokens", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "jeton : {}", reply.body);
    reply.body["secret"].as_str().expect("secret").to_string()
}

/// Un lot vide poussé au nom d'une machine.
///
/// `binding` est le secret de liaison présenté, s'il y en a un ; `supported`
/// dit si le binaire sait en recevoir un — c'est ce qui distingue un agent
/// courant d'un agent antérieur à la liaison.
async fn push(
    app: &TestApp,
    enrolment: &str,
    machine_id: &str,
    hostname: &str,
    binding: Option<&str>,
    supported: bool,
) -> Reply {
    let batch = json!({
        "protocol": 1,
        "identity": {
            "hostname": hostname,
            "os": "linux",
            "agent_version": "0.9.0",
            "machine_id": machine_id,
            "commands_enabled": true,
            "binding_supported": supported,
        },
        "sent_at_ms": 0,
        "samples": [],
    });
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = binding {
        builder = builder.header(SECRET_HEADER, secret);
    }
    send(app, builder.body(Body::from(batch.to_string())).unwrap()).await
}

/// Demande les commandes en attente pour une clé d'identité.
async fn fetch_commands(app: &TestApp, enrolment: &str, key: &str, binding: Option<&str>) -> Reply {
    let mut builder = Request::builder()
        .method("GET")
        .uri(format!("/api/agent/commands?key={key}"))
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"));
    if let Some(secret) = binding {
        builder = builder.header(SECRET_HEADER, secret);
    }
    send(app, builder.body(Body::empty()).unwrap()).await
}

async fn send(app: &TestApp, request: Request<Body>) -> Reply {
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

/// Enrôle une machine liée et rend (jeton, secret de liaison, cible, session).
async fn bound_machine(app: &TestApp) -> (String, String, i64, String) {
    let admin = app.admin_cookie().await;
    let enrolment = token(app, &admin, json!({ "reusable": true })).await;
    let reply = push(app, &enrolment, "id-nas", "nas", None, true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let secret = reply.body["agent_secret"].as_str().expect("secret de liaison").to_string();
    let target = reply.body["target_id"].as_i64().expect("cible");
    (enrolment, secret, target, admin)
}

#[tokio::test]
async fn a_first_batch_binds_the_machine_and_hands_the_secret_once() {
    let app = TestApp::configured().await;
    let (enrolment, secret, target, admin) = bound_machine(&app).await;

    assert!(secret.starts_with("dmab_"), "préfixe inattendu : {secret}");

    // Le lot suivant présente le secret : accepté, et le secret n'est pas rejoué.
    let again = push(&app, &enrolment, "id-nas", "nas", Some(&secret), true).await;
    assert_eq!(again.status, StatusCode::OK, "{}", again.body);
    assert!(again.body.get("agent_secret").is_none(), "le secret ne se redonne pas");
    assert_eq!(again.body["bound"], true);

    // L'interface le dit.
    let view = app.get(&format!("/api/targets/{target}/agent"), Some(&admin)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    assert_eq!(view.body["binding"], "bound");
    assert_eq!(view.body["bound"], true);
    assert!(view.body["bound_at"].is_string());
}

#[tokio::test]
async fn a_second_machine_cannot_take_over_an_existing_registration() {
    let app = TestApp::configured().await;
    let (enrolment, secret, target, admin) = bound_machine(&app).await;

    // La machine compromise a le jeton de parc, et connaît la clé de sa voisine.
    let stolen = push(&app, &enrolment, "id-nas", "usurpateur", None, true).await;
    assert_eq!(stolen.status, StatusCode::FORBIDDEN, "{}", stolen.body);
    assert!(
        stolen.body["error"].as_str().unwrap_or("").contains("Allow re-enrolment"),
        "le refus doit dire quoi faire : {}",
        stolen.body
    );

    // Avec un secret inventé, pas mieux.
    let forged = push(&app, &enrolment, "id-nas", "usurpateur", Some("dmab_0000"), true).await;
    assert_eq!(forged.status, StatusCode::FORBIDDEN, "{}", forged.body);

    // Rien n'a bougé : ni le nom d'hôte de la victime, ni rien d'autre.
    let view = app.get(&format!("/api/targets/{target}/agent"), Some(&admin)).await;
    assert_eq!(view.body["hostname"], "nas");

    // Et la vraie machine continue de passer.
    let honest = push(&app, &enrolment, "id-nas", "nas", Some(&secret), true).await;
    assert_eq!(honest.status, StatusCode::OK, "{}", honest.body);
}

#[tokio::test]
async fn a_machine_cannot_fetch_the_container_commands_of_another() {
    let app = setup_with_inventory().await;
    let (enrolment, secret, target, admin) = bound_machine(&app).await;

    // Une commande est déposée pour la victime, par le chemin normal.
    let queued = app
        .post(&format!("/api/targets/{target}/containers/web/restart"), json!({}), Some(&admin))
        .await;
    assert_eq!(queued.status, StatusCode::CREATED, "{}", queued.body);

    // La machine compromise vient la chercher avec la clé de sa voisine.
    let stolen = fetch_commands(&app, &enrolment, "id-nas", None).await;
    assert_eq!(stolen.status, StatusCode::FORBIDDEN, "{}", stolen.body);
    let forged = fetch_commands(&app, &enrolment, "id-nas", Some("dmab_0000")).await;
    assert_eq!(forged.status, StatusCode::FORBIDDEN, "{}", forged.body);

    // Et la commande est toujours là, intacte, pour son vrai destinataire.
    let mine = fetch_commands(&app, &enrolment, "id-nas", Some(&secret)).await;
    assert_eq!(mine.status, StatusCode::OK, "{}", mine.body);
    assert_eq!(mine.body.as_array().expect("liste").len(), 1);
    assert_eq!(mine.body[0]["args"]["name"], "web");

    // Le compte rendu obéit à la même règle.
    let id = mine.body[0]["id"].as_i64().expect("id");
    let report = Request::builder()
        .method("POST")
        .uri(format!("/api/agent/commands/{id}?key=id-nas"))
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "status": "done", "result": "volé" }).to_string()))
        .unwrap();
    assert_eq!(send(&app, report).await.status, StatusCode::FORBIDDEN);
}

/// Instance configurée dont le test garde la base, pour y reproduire l'état
/// d'une machine enrôlée avant la liaison.
async fn configured_with_pool() -> (TestApp, sqlx::SqlitePool) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = dumbmonit_server::db::open(&config.database_path()).await.expect("base");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    (app, pool)
}

/// Ramène une machine à l'état d'un enregistrement d'avant la liaison : aucun
/// secret, binaire qui ne sait pas en recevoir.
async fn make_legacy(pool: &sqlx::SqlitePool, target: i64) {
    sqlx::query(
        "UPDATE agent_hosts SET secret_hash = NULL, bound_at = NULL, binding_supported = 0
         WHERE target_id = ?",
    )
    .bind(target)
    .execute(pool)
    .await
    .expect("machine d'avant la liaison");
}

/// Demande les sondes relayées pour une clé d'identité.
async fn fetch_relay(app: &TestApp, enrolment: &str, key: &str) -> Reply {
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/agent/relay?key={key}"))
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .body(Body::empty())
        .unwrap();
    send(app, request).await
}

#[tokio::test]
async fn an_agent_too_old_to_be_bound_is_not_enrolled() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let enrolment = token(&app, &admin, json!({ "reusable": true })).await;

    let refused = push(&app, &enrolment, "id-vieux", "vieux-nas", None, false).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);
    assert!(
        refused.body["error"].as_str().unwrap_or("").contains("install command"),
        "le refus doit dire quoi faire : {}",
        refused.body
    );
    // Aucune machine n'est apparue.
    let targets = app.get("/api/targets", Some(&admin)).await;
    assert_eq!(targets.body.as_array().map(Vec::len), Some(0), "{}", targets.body);
}

/// Une machine enrôlée avant la liaison et jamais liée est refusée partout —
/// lots, commandes, relais — même avec le jeton qui l'a enrôlée, et même une
/// fois son agent à jour : seule la fenêtre de reliaison la fait rentrer.
#[tokio::test]
async fn a_legacy_unbound_machine_is_refused_until_it_is_re_enrolled() {
    let (app, pool) = configured_with_pool().await;
    let (enrolment, _, target, admin) = bound_machine(&app).await;
    make_legacy(&pool, target).await;

    for supported in [false, true] {
        let refused = push(&app, &enrolment, "id-nas", "nas", None, supported).await;
        assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);
        assert!(
            refused.body["error"].as_str().unwrap_or("").contains("Re-enrol this host"),
            "le refus doit dire quoi faire : {}",
            refused.body
        );
        assert!(refused.body.get("agent_secret").is_none());
    }
    let commands = fetch_commands(&app, &enrolment, "id-nas", None).await;
    assert_eq!(commands.status, StatusCode::FORBIDDEN, "{}", commands.body);
    assert!(commands.body["error"].as_str().unwrap_or("").contains("Allow re-enrolment"));
    assert_eq!(fetch_relay(&app, &enrolment, "id-nas").await.status, StatusCode::FORBIDDEN);

    // L'interface le montre, avec de quoi agir.
    let view = app.get(&format!("/api/targets/{target}/agent"), Some(&admin)).await;
    assert_eq!(view.body["binding"], "unbound");
    assert_eq!(view.body["bound"], false);

    // Un administrateur ouvre la fenêtre ; l'agent à jour s'y lie.
    let opened =
        app.post(&format!("/api/targets/{target}/agent/rebind"), json!({}), Some(&admin)).await;
    assert_eq!(opened.status, StatusCode::OK, "{}", opened.body);
    let rebound = push(&app, &enrolment, "id-nas", "nas", None, true).await;
    assert_eq!(rebound.status, StatusCode::OK, "{}", rebound.body);
    let secret = rebound.body["agent_secret"].as_str().expect("secret de liaison").to_string();
    assert_eq!(
        fetch_commands(&app, &enrolment, "id-nas", Some(&secret)).await.status,
        StatusCode::OK
    );
    let view = app.get(&format!("/api/targets/{target}/agent"), Some(&admin)).await;
    assert_eq!(view.body["binding"], "bound");
}

/// Une machine jamais liée ne se lie pas davantage avec un autre jeton : il
/// faut la fenêtre, et un jeton qui puisse encore enrôler.
#[tokio::test]
async fn an_unbound_machine_cannot_be_claimed_with_another_token() {
    let (app, pool) = configured_with_pool().await;
    let (_, _, target, admin) = bound_machine(&app).await;
    make_legacy(&pool, target).await;
    let other = token(&app, &admin, json!({ "reusable": true })).await;

    let stolen = push(&app, &other, "id-nas", "pirate", None, true).await;
    assert_eq!(stolen.status, StatusCode::FORBIDDEN, "{}", stolen.body);
    assert!(stolen.body.get("agent_secret").is_none());
    assert_eq!(fetch_commands(&app, &other, "id-nas", None).await.status, StatusCode::FORBIDDEN);

    let opened =
        app.post(&format!("/api/targets/{target}/agent/rebind"), json!({}), Some(&admin)).await;
    assert_eq!(opened.status, StatusCode::OK, "{}", opened.body);
    let rebound = push(&app, &other, "id-nas", "nas", None, true).await;
    assert_eq!(rebound.status, StatusCode::OK, "{}", rebound.body);
    assert!(rebound.body["agent_secret"].is_string(), "{}", rebound.body);
}

/// Sans jeton valide, le corps n'est même pas lu : 401, pas une erreur de
/// désérialisation.
#[tokio::test]
async fn agent_routes_refuse_unknown_tokens_before_reading_the_body() {
    let app = TestApp::configured().await;
    for (method, uri) in [
        ("POST", "/api/ingest"),
        ("POST", "/api/agent/commands/1?key=x"),
        ("POST", "/api/agent/relay/1?key=x"),
    ] {
        for bearer in [None, Some("Bearer dmon_inconnu")] {
            let mut builder = Request::builder()
                .method(method)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json");
            if let Some(bearer) = bearer {
                builder = builder.header(header::AUTHORIZATION, bearer);
            }
            let reply = send(&app, builder.body(Body::from("{ pas du json")).unwrap()).await;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{method} {uri}: {}", reply.body);
        }
    }
}

#[tokio::test]
async fn a_reinstalled_machine_gets_back_in_through_the_window_an_admin_opens() {
    let app = TestApp::configured().await;
    let (enrolment, old_secret, target, admin) = bound_machine(&app).await;

    // Disque refait : l'agent revient sans secret.
    assert_eq!(
        push(&app, &enrolment, "id-nas", "nas", None, true).await.status,
        StatusCode::FORBIDDEN
    );

    // Un lecteur ne peut pas ouvrir la fenêtre.
    let viewer = app.viewer_cookie(&admin).await;
    let refused =
        app.post(&format!("/api/targets/{target}/agent/rebind"), json!({}), Some(&viewer)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);

    let opened =
        app.post(&format!("/api/targets/{target}/agent/rebind"), json!({}), Some(&admin)).await;
    assert_eq!(opened.status, StatusCode::OK, "{}", opened.body);
    assert!(opened.body["rebind_until"].is_string());
    assert_eq!(opened.body["minutes"], 60);

    let rebound = push(&app, &enrolment, "id-nas", "nas", None, true).await;
    assert_eq!(rebound.status, StatusCode::OK, "{}", rebound.body);
    let new_secret = rebound.body["agent_secret"].as_str().expect("nouveau secret");
    assert_ne!(new_secret, old_secret);

    // La fenêtre s'est refermée derrière elle, et l'ancien secret ne vaut plus.
    assert_eq!(
        push(&app, &enrolment, "id-nas", "nas", Some(&old_secret), true).await.status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn a_single_use_token_enrols_one_machine_and_no_more() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    // Sans rien préciser : usage unique, c'est le défaut.
    let enrolment = token(&app, &admin, json!({})).await;

    let first = push(&app, &enrolment, "id-a", "a", None, true).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let secret = first.body["agent_secret"].as_str().expect("secret").to_string();

    let second = push(&app, &enrolment, "id-b", "b", None, true).await;
    assert_eq!(second.status, StatusCode::FORBIDDEN, "{}", second.body);
    assert!(
        second.body["error"].as_str().unwrap_or("").contains("reusable"),
        "le refus doit dire comment faire autrement : {}",
        second.body
    );

    // La machine déjà entrée n'est pas coupée pour autant.
    assert_eq!(
        push(&app, &enrolment, "id-a", "a", Some(&secret), true).await.status,
        StatusCode::OK
    );

    // Le compteur est visible dans l'interface.
    let listed = app.get("/api/agent/tokens", Some(&admin)).await;
    let token = listed.body.as_array().expect("liste").first().expect("jeton");
    assert_eq!(token["max_uses"], 1);
    assert_eq!(token["uses"], 1);
}

#[tokio::test]
async fn a_fleet_token_enrols_up_to_the_count_it_was_given() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let enrolment = token(&app, &admin, json!({ "reusable": true, "max_uses": 2 })).await;

    assert_eq!(push(&app, &enrolment, "id-a", "a", None, true).await.status, StatusCode::OK);
    assert_eq!(push(&app, &enrolment, "id-b", "b", None, true).await.status, StatusCode::OK);
    assert_eq!(push(&app, &enrolment, "id-c", "c", None, true).await.status, StatusCode::FORBIDDEN);

    // Une portée qui n'enrôlerait rien est refusée à la création, pas plus tard.
    let bad = app
        .post(
            "/api/agent/tokens",
            json!({ "name": "x", "reusable": true, "max_uses": 0 }),
            Some(&admin),
        )
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.body);
}

#[tokio::test]
async fn an_oversized_command_report_is_cut_down_by_the_server() {
    let app = setup_with_inventory().await;
    let (enrolment, secret, target, admin) = bound_machine(&app).await;

    let queued = app
        .post(&format!("/api/targets/{target}/containers/web/restart"), json!({}), Some(&admin))
        .await;
    assert_eq!(queued.status, StatusCode::CREATED, "{}", queued.body);
    let pending = fetch_commands(&app, &enrolment, "id-nas", Some(&secret)).await;
    let id = pending.body[0]["id"].as_i64().expect("id");

    // Un agent modifié envoie un journal que personne ne lira.
    let huge = format!("{}the last line explains", "x".repeat(300_000));
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/agent/commands/{id}?key=id-nas"))
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(SECRET_HEADER, &secret)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "status": "done", "result": huge }).to_string()))
        .unwrap();
    assert_eq!(send(&app, request).await.status, StatusCode::NO_CONTENT);

    let listed = app.get(&format!("/api/targets/{target}/commands"), Some(&admin)).await;
    let stored = listed.body[0]["result"].as_str().expect("compte rendu");
    assert!(stored.len() < 5_000, "le compte rendu doit être borné : {} octets", stored.len());
    assert!(
        stored.starts_with("[truncated by the server, beginning dropped]"),
        "la coupe doit être annoncée : {stored}"
    );
    assert!(stored.ends_with("the last line explains"), "la fin est ce qui explique");
}
