//! Tests d'intégration des moniteurs en poussée (heartbeat) : l'URL publique
//! qu'un cron appelle, et ce que l'interface lit du moniteur d'une cible.
//!
//! VictoriaMetrics est injoignable : on vérifie le contrat HTTP et l'effet sur
//! le verdict du collecteur, pas l'écriture des séries.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dumbmonit_proto::Collector;
use dumbmonit_server::collectors::push::{self, PushCollector};
use dumbmonit_server::{collectors, db};
use serde_json::json;
use tower::ServiceExt;

use common::TestApp;

const SECRET: &str = "secret-de-test-suffisamment-long";

/// Instance configurée avec le collecteur `push` enregistré, comme en production.
async fn setup() -> (TestApp, sqlx::SqlitePool, String) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let cipher = db::init_cipher(&pool, SECRET).await.expect("chiffrement");
    let victoria = dumbmonit_server::tsdb::Victoria::new(common::UNREACHABLE_VICTORIA).unwrap();
    let sink =
        dumbmonit_server::tsdb::spawn_writer(victoria.clone(), std::time::Duration::from_secs(60));
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(PushCollector::new(pool.clone())));
    let state = dumbmonit_server::state::AppState::new(dumbmonit_server::state::Inner {
        config,
        pool: pool.clone(),
        cipher,
        victoria,
        sink,
        collectors: registry,
    });
    let app = TestApp { router: dumbmonit_server::api::router(state), _dir: dir };
    app.create_admin().await;
    let cookie = app.admin_cookie().await;
    (app, pool, cookie)
}

async fn create_heartbeat(app: &TestApp, cookie: &str, expected: &str) -> i64 {
    let reply = app
        .post(
            "/api/targets",
            json!({
                "name": "Nightly backup",
                "address": "nightly-backup",
                "kind": "push",
                "tags": { "expected_interval": expected, "grace": "1m" }
            }),
            Some(cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body["id"].as_i64().expect("identifiant")
}

fn target(id: i64, expected: &str) -> dumbmonit_proto::Target {
    dumbmonit_proto::Target {
        id,
        name: "Nightly backup".into(),
        address: "nightly-backup".into(),
        kind: "push".into(),
        profile_id: None,
        parent_id: None,
        interval: std::time::Duration::from_secs(60),
        enabled: true,
        tags: [("expected_interval".to_string(), expected.to_string())].into_iter().collect(),
        credential: dumbmonit_proto::Credential::None,
        group_name: String::new(),
        position: 0,
    }
}

#[tokio::test]
async fn le_type_push_est_decrit_a_linterface_sans_identifiant_ni_port() {
    let (app, _pool, cookie) = setup().await;
    let reply = app.get("/api/collectors", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let push = reply.body.as_array().unwrap().iter().find(|c| c["kind"] == "push").expect("push");
    assert_eq!(push["label"], "Heartbeat (push)");
    assert_eq!(push["credential_types"], json!(["none"]));
    let keys: Vec<&str> =
        push["options"].as_array().unwrap().iter().map(|o| o["key"].as_str().unwrap()).collect();
    assert_eq!(keys, ["expected_interval", "grace", "schedule", "timezone", "max_duration"]);
}

#[tokio::test]
async fn la_page_de_lequipement_obtient_une_url_et_attend_le_premier_appel() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;

    let reply = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let token = reply.body["token"].as_str().expect("jeton");
    assert_eq!(token.len(), 32);
    assert_eq!(reply.body["path"], format!("/api/push/{token}"));
    assert_eq!(reply.body["verdict"], "waiting");
    assert!(reply.body["last_seen_at"].is_null());
    assert_eq!(reply.body["received_total"], 0);
    assert_eq!(reply.body["expected_interval_secs"], 3600);
    assert_eq!(reply.body["grace_secs"], 60);

    // Relire ne fabrique pas un second jeton.
    let again = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await;
    assert_eq!(again.body["token"], token);

    // Sans session : rien.
    let anonymous = app.get(&format!("/api/targets/{id}/push"), None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn un_appel_public_sans_session_ni_en_tete_est_accepte_et_change_le_verdict() {
    let (app, pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Tel que `curl` l'appelle depuis une crontab : GET, pas de cookie, pas
    // d'en-tête `X-Requested-With`.
    let reply = app.get(&format!("/api/push/{token}"), None).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    // POST aussi, pour les clients qui préfèrent.
    let reply = app.request("POST", &format!("/api/push/{token}"), None, None).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    let view = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body;
    assert_eq!(view["verdict"], "on_time");
    assert_eq!(view["received_total"], 2);
    assert!(view["last_seen_at"].is_string());
    assert_eq!(view["last_status"], "up");

    // Le collecteur écrit un succès.
    let samples = PushCollector::new(pool.clone()).probe(&target(id, "1h")).await.expect("ok");
    assert_eq!(samples.iter().find(|s| s.metric == "probe_success").unwrap().value, 1.0);

    // L'appel a rejoué le contrôle tout de suite : la cible porte un résultat.
    let status = db::targets::statuses(&pool).await.unwrap().remove(&id).unwrap();
    assert!(status.last_probe_at.is_some(), "verdict enregistré");
    assert!(status.last_error.is_none());
}

#[tokio::test]
async fn un_appel_qui_signale_un_echec_est_un_echec_immediat() {
    let (app, pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "24h").await;
    let token = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body["token"]
        .as_str()
        .unwrap()
        .to_string();

    let reply =
        app.get(&format!("/api/push/{token}?status=down&msg=rsync%20exit%2023"), None).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    let view = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body;
    assert_eq!(view["verdict"], "reported_down");
    assert_eq!(view["last_status"], "down");
    assert_eq!(view["last_message"], "rsync exit 23");

    let samples = PushCollector::new(pool.clone()).probe(&target(id, "24h")).await.expect("ok");
    assert_eq!(samples.iter().find(|s| s.metric == "probe_success").unwrap().value, 0.0);
    let info = samples.iter().find(|s| s.metric == "probe_failure_info").unwrap();
    assert_eq!(info.labels.get("reason").map(String::as_str), Some("reported_down"));

    // Un statut inconnu est refusé, sans être enregistré.
    let reply = app.get(&format!("/api/push/{token}?status=maybe"), None).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let view = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body;
    assert_eq!(view["received_total"], 1);

    // Le travail reprend : un appel `up` efface le verdict.
    assert_eq!(
        app.get(&format!("/api/push/{token}?status=up"), None).await.status,
        StatusCode::NO_CONTENT
    );
    let view = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body;
    assert_eq!(view["verdict"], "on_time");
}

#[tokio::test]
async fn un_jeton_inconnu_ou_mal_forme_repond_404() {
    let (app, _pool, _cookie) = setup().await;
    for path in [
        "/api/push/0123456789abcdef0123456789abcdef",
        "/api/push/pas-un-jeton",
        "/api/push/0123456789ABCDEF0123456789ABCDEF",
    ] {
        let reply = app.get(path, None).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn regenerer_change_lurl_et_coupe_lancienne() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let old = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body["token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(app.get(&format!("/api/push/{old}"), None).await.status, StatusCode::NO_CONTENT);

    let reply =
        app.post(&format!("/api/targets/{id}/push/regenerate"), json!({}), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let new = reply.body["token"].as_str().unwrap().to_string();
    assert_ne!(new, old);
    assert_eq!(reply.body["received_total"], 1, "le compteur survit à la régénération");

    assert_eq!(app.get(&format!("/api/push/{old}"), None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get(&format!("/api/push/{new}"), None).await.status, StatusCode::NO_CONTENT);

    // La régénération est une écriture : lecteur refusé, session obligatoire.
    let viewer = app.viewer_cookie(&cookie).await;
    let reply =
        app.post(&format!("/api/targets/{id}/push/regenerate"), json!({}), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}

/// Le jeton permet de déclarer « le travail a tourné » : un lecteur voit l'état
/// du moniteur, pas l'URL secrète, et sa lecture ne crée rien.
#[tokio::test]
async fn un_lecteur_ne_voit_pas_le_jeton_et_ne_cree_rien() {
    let (app, pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    sqlx::query("DELETE FROM push_monitors WHERE target_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let viewer = app.viewer_cookie(&cookie).await;

    let reply = app.get(&format!("/api/targets/{id}/push"), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body["token"].is_null(), "{}", reply.body);
    assert!(reply.body["path"].is_null(), "{}", reply.body);
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM push_monitors WHERE target_id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0, "une lecture de lecteur ne crée pas de jeton");

    // L'administrateur, lui, l'obtient (et le crée au besoin).
    let reply = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await;
    assert!(reply.body["token"].is_string(), "{}", reply.body);
    let reply = app.get(&format!("/api/targets/{id}/push"), Some(&viewer)).await;
    assert!(reply.body["token"].is_null(), "{}", reply.body);
}

#[tokio::test]
async fn le_moniteur_ne_se_lit_que_sur_une_cible_push() {
    let (app, _pool, cookie) = setup().await;
    let reply = app
        .post(
            "/api/targets",
            json!({ "name": "demo", "address": "demo", "kind": "dummy" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let id = reply.body["id"].as_i64().unwrap();
    let reply = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app.get("/api/targets/9999/push", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn supprimer_la_cible_supprime_son_moniteur() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body["token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        app.delete(&format!("/api/targets/{id}"), Some(&cookie)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(app.get(&format!("/api/push/{token}"), None).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn le_jeton_est_stocke_chiffre_et_hache_jamais_en_clair() {
    let (app, pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body["token"]
        .as_str()
        .unwrap()
        .to_string();
    let row: (String, Vec<u8>) =
        sqlx::query_as("SELECT token_hash, token_enc FROM push_monitors WHERE target_id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, push::token::fingerprint(&token));
    assert_ne!(row.0, token);
    assert!(!String::from_utf8_lossy(&row.1).contains(&token), "le clair n'est pas en base");
}

/// POST avec un corps brut (la sortie du travail), comme `curl --data-binary`.
async fn raw_post(app: &TestApp, uri: &str, body: Vec<u8>) -> StatusCode {
    let request = Request::builder().method("POST").uri(uri).body(Body::from(body)).unwrap();
    app.router.clone().oneshot(request).await.expect("réponse").status()
}

/// Appel public sans session, comme `curl` dans une crontab.
async fn hit(app: &TestApp, path: &str) -> StatusCode {
    app.get(path, None).await.status
}

async fn token_of(app: &TestApp, cookie: &str, id: i64) -> String {
    app.get(&format!("/api/targets/{id}/push"), Some(cookie)).await.body["token"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn start_puis_succes_mesure_la_duree_et_laisse_une_execution() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = token_of(&app, &cookie, id).await;

    assert_eq!(hit(&app, &format!("/api/push/{token}/start")).await, StatusCode::NO_CONTENT);
    let view = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body;
    assert_eq!(view["last_run"]["status"], "running");
    assert!(view["last_run"]["started_at"].is_string());

    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let uri = format!("/api/push/{token}/success");
    assert_eq!(raw_post(&app, &uri, b"all good\n".to_vec()).await, StatusCode::NO_CONTENT);

    let page = app.get(&format!("/api/targets/{id}/push/runs"), Some(&cookie)).await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    let runs = page.body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1, "{}", page.body);
    assert_eq!(runs[0]["status"], "success");
    assert_eq!(runs[0]["output"], "all good\n");
    assert!(runs[0]["duration_secs"].as_f64().unwrap() > 0.0);
    assert!(runs[0]["started_at"].is_string() && runs[0]["finished_at"].is_string());
    assert!(page.body["next_before"].is_null());
}

#[tokio::test]
async fn le_ping_simple_est_un_succes_sans_duree() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = token_of(&app, &cookie, id).await;

    assert_eq!(app.get(&format!("/api/push/{token}"), None).await.status, StatusCode::NO_CONTENT);
    let runs = app.get(&format!("/api/targets/{id}/push/runs"), Some(&cookie)).await.body;
    assert_eq!(runs["runs"][0]["status"], "success");
    assert!(runs["runs"][0]["started_at"].is_null());
    assert!(runs["runs"][0]["duration_secs"].is_null());
}

#[tokio::test]
async fn fail_et_code_de_sortie_non_nul_sont_des_echecs_avec_leur_sortie() {
    let (app, pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = token_of(&app, &cookie, id).await;

    // `/fail` avec un code en paramètre et la sortie en corps.
    let uri = format!("/api/push/{token}/fail?exit_code=23");
    assert_eq!(raw_post(&app, &uri, b"rsync: error".to_vec()).await, StatusCode::NO_CONTENT);
    // Le code dans le chemin.
    assert_eq!(app.get(&format!("/api/push/{token}/2"), None).await.status, StatusCode::NO_CONTENT);
    // Le code 0 dans le chemin, ou un `exit_code=0` : un succès.
    assert_eq!(app.get(&format!("/api/push/{token}/0"), None).await.status, StatusCode::NO_CONTENT);
    // Un succès qui porte un code non nul est un échec.
    assert_eq!(
        app.get(&format!("/api/push/{token}/success?exit_code=1"), None).await.status,
        StatusCode::NO_CONTENT
    );

    let runs = app.get(&format!("/api/targets/{id}/push/runs"), Some(&cookie)).await.body;
    let runs = runs["runs"].as_array().unwrap();
    let summary: Vec<(String, Option<i64>)> = runs
        .iter()
        .map(|r| (r["status"].as_str().unwrap().to_string(), r["exit_code"].as_i64()))
        .collect();
    // Les plus récentes d'abord.
    assert_eq!(
        summary,
        [
            ("failed".to_string(), Some(1)),
            ("success".to_string(), Some(0)),
            ("failed".to_string(), Some(2)),
            ("failed".to_string(), Some(23)),
        ]
    );
    assert_eq!(runs[3]["output"], "rsync: error");

    // Dernière exécution réussie : plus d'échec annoncé par le collecteur.
    assert_eq!(app.get(&format!("/api/push/{token}/0"), None).await.status, StatusCode::NO_CONTENT);
    let samples = PushCollector::new(pool.clone()).probe(&target(id, "1h")).await.expect("ok");
    assert_eq!(samples.iter().find(|s| s.metric == "push_run_failed").unwrap().value, 0.0);

    // Un échec : le collecteur le dit, et le heartbeat reste « à l'heure ».
    assert_eq!(hit(&app, &format!("/api/push/{token}/fail")).await, StatusCode::NO_CONTENT);
    let samples = PushCollector::new(pool.clone()).probe(&target(id, "1h")).await.expect("ok");
    assert_eq!(samples.iter().find(|s| s.metric == "push_run_failed").unwrap().value, 1.0);
    assert_eq!(samples.iter().find(|s| s.metric == "probe_success").unwrap().value, 1.0);
    let view = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body;
    assert_eq!(view["verdict"], "on_time");
}

#[tokio::test]
async fn les_entrees_invalides_sont_refusees_sans_rien_enregistrer() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = token_of(&app, &cookie, id).await;

    for bad in ["?exit_code=abc", "?exit_code=-1", "/success?exit_code=1.5"] {
        let reply = app.get(&format!("/api/push/{token}{bad}"), None).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}");
    }
    // Un segment inconnu est un 404, comme un jeton inconnu.
    for path in ["/nope", "/12a"] {
        let reply = app.get(&format!("/api/push/{token}{path}"), None).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
    }
    let unknown = "0123456789abcdef0123456789abcdef";
    assert_eq!(hit(&app, &format!("/api/push/{unknown}/start")).await, StatusCode::NOT_FOUND);

    let runs = app.get(&format!("/api/targets/{id}/push/runs"), Some(&cookie)).await.body;
    assert!(runs["runs"].as_array().unwrap().is_empty(), "{runs}");
}

#[tokio::test]
async fn le_corps_est_borne_et_la_sortie_tronquee_a_10_ko() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = token_of(&app, &cookie, id).await;

    // 100 Ko : accepté, conservé tronqué (la fin), UTF-8 intact.
    let body = format!("{}\u{20ac}FIN", "é".repeat(50_000)).into_bytes();
    let uri = format!("/api/push/{token}/fail");
    assert_eq!(raw_post(&app, &uri, body).await, StatusCode::NO_CONTENT);
    let runs = app.get(&format!("/api/targets/{id}/push/runs"), Some(&cookie)).await.body;
    let output = runs["runs"][0]["output"].as_str().unwrap();
    assert!(output.len() <= 10 * 1024 && output.len() > 10 * 1024 - 4, "{}", output.len());
    assert!(output.ends_with("\u{20ac}FIN"));

    // Au-delà de 256 Ko : refusé.
    let uri = format!("/api/push/{token}/fail");
    assert_eq!(raw_post(&app, &uri, vec![b'x'; 300 * 1024]).await, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn lhistorique_se_pagine_et_se_lit_sans_le_jeton() {
    let (app, _pool, cookie) = setup().await;
    let id = create_heartbeat(&app, &cookie, "1h").await;
    let token = token_of(&app, &cookie, id).await;
    for n in 0..5 {
        assert_eq!(
            app.get(&format!("/api/push/{token}/{n}"), None).await.status,
            StatusCode::NO_CONTENT
        );
    }
    let viewer = app.viewer_cookie(&cookie).await;
    let first = app.get(&format!("/api/targets/{id}/push/runs?limit=2"), Some(&viewer)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let runs = first.body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["exit_code"], 4);
    let cursor = first.body["next_before"].as_i64().expect("page suivante");
    let second = app
        .get(&format!("/api/targets/{id}/push/runs?limit=2&before={cursor}"), Some(&viewer))
        .await;
    assert_eq!(second.body["runs"][0]["exit_code"], 2);
    let last = app
        .get(&format!("/api/targets/{id}/push/runs?limit=100&before={cursor}"), Some(&viewer))
        .await;
    assert_eq!(last.body["runs"].as_array().unwrap().len(), 3);
    assert!(last.body["next_before"].is_null());

    // Sans session : refusé.
    let anonymous = app.get(&format!("/api/targets/{id}/push/runs"), None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn un_planning_cron_se_valide_a_lenregistrement_et_se_relit() {
    let (app, _pool, cookie) = setup().await;
    let create = |schedule: &str, timezone: &str| {
        json!({
            "name": "Cron job",
            "address": format!("cron-{}", schedule.len()),
            "kind": "push",
            "tags": {
                "schedule": schedule,
                "timezone": timezone,
                "grace": "15m",
                "max_duration": "2h"
            }
        })
    };
    let invalid = [("* * * *", "UTC"), ("0 3 * * *", "Mars/Olympus"), ("99 * * * *", "UTC")];
    for (schedule, timezone) in invalid {
        let reply = app.post("/api/targets", create(schedule, timezone), Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{schedule} {timezone}: {}", reply.body);
        assert!(reply.body["error"].as_str().unwrap().contains("schedule"), "{}", reply.body);
    }
    let valid = create("30 2 * * mon-fri", "Europe/Paris");
    let reply = app.post("/api/targets", valid, Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let id = reply.body["id"].as_i64().unwrap();

    let view = app.get(&format!("/api/targets/{id}/push"), Some(&cookie)).await.body;
    assert_eq!(view["schedule"], "30 2 * * mon-fri");
    assert_eq!(view["timezone"], "Europe/Paris");
    assert_eq!(view["grace_secs"], 900);
    assert_eq!(view["max_duration_secs"], 7200);
    assert!(view["next_expected_at"].is_string(), "{view}");
    assert!(view["settings_error"].is_null());
}
