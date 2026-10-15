//! Tests d'intégration : les services qu'un agent détecte deviennent des
//! équipements rangés sous leur machine.
//!
//! Tout passe par l'API réelle, de l'ingestion à la liste des modules, et le
//! dernier test part d'une base d'avant la séparation pour vérifier que les
//! réglages d'une installation existante suivent le service sur son appareil.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dumbmonit_server::db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_HEADER: &str = "x-dumbmonit-agent-secret";

fn sample(metric: &str, labels: Value) -> Value {
    json!({ "metric": metric, "value": 1.0, "kind": "gauge",
            "ts_ms": chrono::Utc::now().timestamp_millis(), "labels": labels })
}

/// Un lot d'une machine : processeur, deux conteneurs et un pool ZFS par défaut.
fn batch(samples: Vec<Value>) -> Value {
    json!({
        "protocol": 1,
        "identity": {
            "hostname": "nas",
            "os": "linux",
            "agent_version": "0.9.0",
            "machine_id": "id-nas",
            "commands_enabled": true,
            "binding_supported": true,
        },
        "sent_at_ms": 0,
        "samples": samples,
    })
}

fn full_batch() -> Value {
    batch(vec![
        sample("cpu_usage_percent", json!({})),
        sample("container_up", json!({ "container": "web", "image": "nginx" })),
        sample("container_up", json!({ "container": "db", "image": "postgres" })),
        sample("agent_zfs_pool_health", json!({ "pool": "tank" })),
    ])
}

async fn push(app: &TestApp, enrolment: &str, secret: Option<&str>, body: &Value) -> Reply {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        builder = builder.header(SECRET_HEADER, secret);
    }
    let response = app
        .router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie: None,
        location: None,
    }
}

/// Enrôle la machine avec un premier lot complet ; rend (jeton, secret, hôte, session).
async fn enrolled(app: &TestApp) -> (String, String, i64, String) {
    let admin = app.admin_cookie().await;
    let reply = app.post("/api/agent/tokens", json!({ "name": "parc" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let enrolment = reply.body["secret"].as_str().expect("jeton").to_string();
    let first = push(app, &enrolment, None, &full_batch()).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let secret = first.body["agent_secret"].as_str().expect("secret").to_string();
    let host = first.body["target_id"].as_i64().expect("hôte");
    (enrolment, secret, host, admin)
}

async fn targets(app: &TestApp, admin: &str) -> Vec<Value> {
    let reply = app.get("/api/targets", Some(admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body.as_array().expect("liste").clone()
}

fn service<'a>(targets: &'a [Value], module: &str) -> Option<&'a Value> {
    targets.iter().find(|t| t["kind"] == "agent_service" && t["profile_id"] == module)
}

async fn modules(app: &TestApp, admin: &str, host: i64) -> Vec<Value> {
    let reply = app.get(&format!("/api/targets/{host}/agent/modules"), Some(admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body.as_array().expect("modules").clone()
}

fn module<'a>(list: &'a [Value], id: &str) -> &'a Value {
    list.iter().find(|m| m["id"] == id).unwrap_or_else(|| panic!("module {id} absent"))
}

#[tokio::test]
async fn each_detected_service_becomes_a_device_under_its_machine() {
    let app = TestApp::configured().await;
    let (_, _, host, admin) = enrolled(&app).await;

    let list = targets(&app, &admin).await;
    let docker = service(&list, "docker").expect("appareil Docker");
    let zfs = service(&list, "zfs").expect("appareil ZFS");
    for child in [docker, zfs] {
        assert_eq!(child["parent_id"], host, "rangé sous la machine : {child}");
    }
    assert_eq!(docker["name"], "nas · Docker");
    // Rien d'autre n'a été créé : pas de WireGuard sur cette machine.
    assert_eq!(list.iter().filter(|t| t["kind"] == "agent_service").count(), 2);

    let found = modules(&app, &admin, host).await;
    assert_eq!(found.len(), 8, "tous les modules connus sont listés");
    let docker_module = module(&found, "docker");
    assert_eq!(docker_module["detected"], true);
    assert_eq!(docker_module["enabled"], true);
    assert_eq!(docker_module["automatic"], true);
    assert_eq!(docker_module["target_id"], docker["id"]);
    assert!(docker_module["last_seen_at"].is_string());
    let wireguard = module(&found, "wireguard");
    assert_eq!(wireguard["detected"], false);
    assert_eq!(wireguard["enabled"], false);
    assert!(wireguard["target_id"].is_null());

    // Les conteneurs se commandent aussi depuis l'appareil Docker ; un autre
    // service n'a pas de conteneurs.
    let docker_id = docker["id"].as_i64().unwrap();
    let zfs_id = zfs["id"].as_i64().unwrap();
    let commands = app.get(&format!("/api/targets/{docker_id}/commands"), Some(&admin)).await;
    assert_eq!(commands.status, StatusCode::OK, "{}", commands.body);
    let none = app.get(&format!("/api/targets/{zfs_id}/containers"), Some(&admin)).await;
    assert_eq!(none.status, StatusCode::NOT_FOUND);
    // La liste des modules n'existe que sur la machine.
    let wrong = app.get(&format!("/api/targets/{docker_id}/agent/modules"), Some(&admin)).await;
    assert_eq!(wrong.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unticking_a_module_drops_its_device_and_tells_the_agent() {
    let app = TestApp::configured().await;
    let (enrolment, secret, host, admin) = enrolled(&app).await;
    let docker = service(&targets(&app, &admin).await, "docker").unwrap()["id"].as_i64().unwrap();

    let off = app
        .put(
            &format!("/api/targets/{host}/agent/modules/docker"),
            json!({ "enabled": false }),
            Some(&admin),
        )
        .await;
    assert_eq!(off.status, StatusCode::OK, "{}", off.body);
    assert_eq!(off.body["enabled"], false);
    assert_eq!(off.body["automatic"], false);
    assert!(off.body["target_id"].is_null());
    let gone = app.get(&format!("/api/targets/{docker}"), Some(&admin)).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND, "l'appareil est supprimé");

    // Le lot suivant ne le recrée pas, et l'accusé dit à l'agent d'arrêter.
    let next = push(&app, &enrolment, Some(&secret), &full_batch()).await;
    assert_eq!(next.status, StatusCode::OK, "{}", next.body);
    assert_eq!(next.body["disabled_modules"], json!(["docker"]));
    // Les deux conteneurs sont écartés, pas rangés sous la machine.
    assert_eq!(next.body["accepted"], 2, "processeur et pool ZFS seulement : {}", next.body);
    let list = targets(&app, &admin).await;
    assert!(service(&list, "docker").is_none());
    assert_eq!(module(&modules(&app, &admin, host).await, "docker")["detected"], true);

    // Recoché : l'appareil revient, l'accusé ne nomme plus rien.
    let on = app
        .put(
            &format!("/api/targets/{host}/agent/modules/docker"),
            json!({ "enabled": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(on.status, StatusCode::OK, "{}", on.body);
    assert!(on.body["target_id"].is_i64());
    let again = push(&app, &enrolment, Some(&secret), &full_batch()).await;
    assert!(again.body.get("disabled_modules").is_none(), "{}", again.body);
    assert_eq!(again.body["accepted"], 4);

    // Supprimer l'appareil d'un service revient à le décocher.
    let zfs = service(&targets(&app, &admin).await, "zfs").unwrap()["id"].as_i64().unwrap();
    assert_eq!(
        app.delete(&format!("/api/targets/{zfs}"), Some(&admin)).await.status,
        StatusCode::NO_CONTENT
    );
    let after = push(&app, &enrolment, Some(&secret), &full_batch()).await;
    assert_eq!(after.body["disabled_modules"], json!(["zfs"]));
    assert!(service(&targets(&app, &admin).await, "zfs").is_none());
}

#[tokio::test]
async fn a_module_can_be_ticked_before_it_is_found_and_goes_with_its_host() {
    let app = TestApp::configured().await;
    let (_, _, host, admin) = enrolled(&app).await;

    let on = app
        .put(
            &format!("/api/targets/{host}/agent/modules/wireguard"),
            json!({ "enabled": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(on.status, StatusCode::OK, "{}", on.body);
    assert_eq!(on.body["detected"], false);
    assert_eq!(on.body["enabled"], true);
    let list = targets(&app, &admin).await;
    assert_eq!(service(&list, "wireguard").expect("créé à la main")["parent_id"], host);

    let unknown = app
        .put(
            &format!("/api/targets/{host}/agent/modules/cpu"),
            json!({ "enabled": true }),
            Some(&admin),
        )
        .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);

    // Un lecteur voit la liste mais ne coche rien.
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(modules(&app, &viewer, host).await.len(), 8);
    let refused = app
        .put(
            &format!("/api/targets/{host}/agent/modules/zfs"),
            json!({ "enabled": false }),
            Some(&viewer),
        )
        .await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);

    // Supprimer la machine emporte ses services.
    assert_eq!(
        app.delete(&format!("/api/targets/{host}"), Some(&admin)).await.status,
        StatusCode::NO_CONTENT
    );
    assert!(targets(&app, &admin).await.iter().all(|t| t["kind"] != "agent_service"));
}

#[tokio::test]
async fn an_existing_install_moves_its_settings_and_alerts_to_the_new_service_device() {
    // Une base d'avant la séparation : une machine liée, une règle et une
    // surcharge qui la visent pour ses conteneurs, un silence, une alerte en cours.
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let path = config.database_path();
    let old = db::open_up_to(&path, 53).await.expect("base ancienne");
    let enrolment = "dmon_ancien-jeton-de-parc";
    let secret = "dmab_ancien-secret-de-liaison";
    let hash = |text: &str| hex::encode(Sha256::digest(text.as_bytes()));
    // Les empreintes sont liées en paramètres : seules des requêtes statiques
    // passent le contrôle d'injection de sqlx.
    sqlx::query(
        "INSERT INTO agent_tokens (id, name, token_hash, prefix) VALUES (1, 'parc', ?, 'dmon_anc')",
    )
    .bind(hash(enrolment))
    .execute(&old)
    .await
    .expect("jeton ancien");
    sqlx::query(
        "INSERT INTO targets (id, name, address, kind, tags) \
         VALUES (5, 'nas', 'id-nas', 'agent', '{\"site\":\"cave\"}')",
    )
    .execute(&old)
    .await
    .expect("machine ancienne");
    sqlx::query(
        "INSERT INTO agent_hosts (target_id, agent_key, hostname, token_id, secret_hash, \
         binding_supported, bound_at) VALUES (5, 'id-nas', 'nas', 1, ?, 1, '2026-01-01T00:00:00Z')",
    )
    .bind(hash(secret))
    .execute(&old)
    .await
    .expect("liaison ancienne");
    let statements: [&'static str; 6] = [
        "INSERT INTO alert_rules (uid, name, kind, query, operator, threshold, severity, selector) \
         VALUES ('my_docker', 'Mine', 'threshold', 'dumbmonit_container_up == bool 0', '>', 0, \
                 'warning', '{\"kind\":\"ids\",\"ids\":[5]}')",
        "INSERT INTO alert_rules (uid, name, kind, query, operator, threshold, severity, selector) \
         VALUES ('my_cpu', 'CPU', 'threshold', 'dumbmonit_cpu_usage_percent', '>', 90, \
                 'warning', '{\"kind\":\"ids\",\"ids\":[5]}')",
        "INSERT INTO rule_overrides (rule_uid, target_id, enabled) VALUES ('my_docker', 5, 0)",
        "INSERT INTO rule_overrides (rule_uid, target_id, threshold) VALUES ('my_cpu', 5, 95)",
        "INSERT INTO silences (name, target_id, schedule) \
         VALUES ('Sunday backups', 5, '{\"kind\":\"weekly\",\"days\":[0],\"start_minute\":0,\"end_minute\":60,\"utc_offset_minutes\":0}')",
        "INSERT INTO alert_state (fingerprint, rule_uid, target_id, series_key, labels, phase, \
         severity, firing_since, last_eval_at) VALUES ('my_docker@old', 'my_docker', 5, \
         '{container=\"web\",host=\"nas\",target=\"5\"}', \
         '{\"container\":\"web\",\"host\":\"nas\",\"target\":\"5\"}', 'firing', 'warning', \
         '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    ];
    for statement in statements {
        sqlx::query(statement).execute(&old).await.unwrap_or_else(|e| panic!("{statement}: {e}"));
    }
    old.close().await;

    // La mise à jour : les migrations passent, puis la machine pousse son lot.
    let pool = db::open(&path).await.expect("migrations appliquées");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    let reply = push(&app, enrolment, Some(secret), &full_batch()).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["target_id"], 5, "même machine, même appareil");

    let list = targets(&app, &admin).await;
    let docker = service(&list, "docker").expect("appareil Docker créé")["id"].as_i64().unwrap();
    let docker_view = app.get(&format!("/api/targets/{docker}"), Some(&admin)).await;
    assert_eq!(docker_view.body["tags"]["site"], "cave", "les étiquettes de l'hôte suivent");

    // La règle qui visait la machine pour ses conteneurs vise aussi le service ;
    // celle du processeur n'a pas bougé.
    let selector = |uid: &'static str| {
        let pool = pool.clone();
        async move {
            let row = sqlx::query("SELECT selector FROM alert_rules WHERE uid = ?")
                .bind(uid)
                .fetch_one(&pool)
                .await
                .expect("règle");
            serde_json::from_str::<Value>(&row.get::<String, _>("selector")).unwrap()
        }
    };
    assert_eq!(selector("my_docker").await["ids"], json!([5, docker]));
    assert_eq!(selector("my_cpu").await["ids"], json!([5]));

    let overrides: Vec<(String, i64)> = sqlx::query_as(
        "SELECT rule_uid, target_id FROM rule_overrides ORDER BY rule_uid, target_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        overrides,
        [
            ("my_cpu".to_string(), 5),
            ("my_docker".to_string(), 5),
            ("my_docker".to_string(), docker)
        ]
    );

    let silenced: Vec<(i64,)> = sqlx::query_as(
        "SELECT target_id FROM silences WHERE name = 'Sunday backups' ORDER BY target_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    // Chaque service de la machine (ici Docker et ZFS) reçoit sa copie.
    let zfs = service(&list, "zfs").expect("appareil ZFS créé")["id"].as_i64().unwrap();
    let mut expected = vec![(5,), (docker,), (zfs,)];
    expected.sort();
    assert_eq!(silenced, expected, "la fenêtre de maintenance vaut aussi pour les services");

    // L'alerte en cours est reprise sous le service, avec l'empreinte que le
    // moteur lui calculera : elle ne se résout pas pour se redéclencher.
    let alert = sqlx::query("SELECT fingerprint, target_id, labels, phase FROM alert_state")
        .fetch_one(&pool)
        .await
        .expect("alerte");
    assert_eq!(alert.get::<i64, _>("target_id"), docker);
    assert_eq!(alert.get::<String, _>("phase"), "firing");
    let labels: Value = serde_json::from_str(&alert.get::<String, _>("labels")).unwrap();
    assert_eq!(labels["target"], docker.to_string());
    assert_eq!(labels["container"], "web");
    let expected = format!("container=\"web\",target=\"{docker}\"");
    let fingerprint: String = alert.get("fingerprint");
    assert!(fingerprint.starts_with("my_docker@") && fingerprint != "my_docker@old");
    assert_eq!(
        fingerprint,
        dumbmonit_server::alerting::model::fingerprint("my_docker", &format!("{{{expected}}}"))
    );

    // Un second lot ne recopie rien une deuxième fois.
    let again = push(&app, enrolment, Some(secret), &full_batch()).await;
    assert_eq!(again.status, StatusCode::OK);
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM silences").fetch_one(&pool).await.unwrap();
    assert_eq!(count.0, 3);
}
