//! Mode multi-clients (MSP) : l'isolation des données entre clients.
//!
//! Le scénario : deux clients, Acme (A) et Beta (B), un équipement chacun, une
//! alerte chacun. Un compte membre d'Acme ne doit rien lire ni modifier de
//! Beta — ni par la liste, ni par le détail, ni par les métriques, les
//! alertes, les règles, les rapports, les logos ou une route oubliée — et un
//! jeton d'agent d'Acme ne doit rien faire entrer chez Beta.

mod common;

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dumbmonit_server::{backup, db};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const SECRET_NAME: &str = "beta-secret-nas";
const SECRET_ADDRESS: &str = "10.77.77.77";
const VIEW_HEADER: &str = "x-dumbmonit-client";

/// Faux VictoriaMetrics : séries vides, et la chaîne de chaque requête reçue.
async fn fake_victoria() -> (String, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    let handler = move |uri: axum::http::Uri| {
        let record = record.clone();
        async move {
            record.lock().unwrap().push(uri.query().unwrap_or_default().to_string());
            let kind = if uri.path().ends_with("query_range") { "matrix" } else { "vector" };
            axum::Json(json!({ "status": "success", "data": { "resultType": kind, "result": [] } }))
        }
    };
    let router = axum::Router::new().fallback(handler);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    (format!("http://{address}"), seen)
}

struct Lab {
    app: TestApp,
    pool: sqlx::SqlitePool,
    admin: String,
    seen: Arc<Mutex<Vec<String>>>,
    acme: i64,
    beta: i64,
    a1: i64,
    b1: i64,
}

async fn create_user(app: &TestApp, admin: &str, username: &str, role: &str) -> (i64, String) {
    let password = format!("{username}-mot-de-passe-solide");
    let reply = app
        .post(
            "/api/users",
            json!({ "username": username, "role": role, "password": password }),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let id = reply.body["id"].as_i64().unwrap();
    (id, app.login_as(username, &password).await.cookie())
}

async fn create_client(app: &TestApp, admin: &str, name: &str) -> i64 {
    let reply = app.post("/api/clients", json!({ "name": name }), Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body["id"].as_i64().unwrap()
}

async fn create_target(app: &TestApp, admin: &str, name: &str, address: &str, client: i64) -> i64 {
    let reply = app
        .post(
            "/api/targets",
            json!({ "name": name, "address": address, "kind": "dummy", "client_id": client }),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["client_id"], client, "{}", reply.body);
    reply.body["id"].as_i64().unwrap()
}

async fn lab() -> Lab {
    let (victoria, seen) = fake_victoria().await;
    let dir = tempfile::tempdir().unwrap();
    let mut config = common::base_config(dir.path());
    config.victoria_url = Some(victoria);
    let pool = db::open(&config.database_path()).await.unwrap();
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;

    let acme = create_client(&app, &admin, "Acme").await;
    let beta = create_client(&app, &admin, "Beta").await;
    let reply = app.put("/api/clients/settings", json!({ "enabled": true }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    let a1 = create_target(&app, &admin, "acme-router", "10.1.0.1", acme).await;
    let b1 = create_target(&app, &admin, SECRET_NAME, SECRET_ADDRESS, beta).await;

    // Une alerte active par équipement, et son historique.
    for (fingerprint, target) in [("fp-acme", a1), ("fp-beta", b1)] {
        sqlx::query(
            "INSERT INTO alert_state (fingerprint, rule_uid, target_id, series_key, labels, phase,
                                      severity, firing_since, last_eval_at)
             VALUES (?, 'host_down', ?, ?, ?, 'firing', 'critical',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .bind(fingerprint)
        .bind(target)
        .bind(format!("{{target=\"{target}\"}}"))
        .bind(format!("{{\"target\":\"{target}\"}}"))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO alert_history (fingerprint, rule_uid, target_id, from_phase, to_phase,
                                        severity, at)
             VALUES (?, 'host_down', ?, 'pending', 'firing', 'critical', ?)",
        )
        .bind(fingerprint)
        .bind(target)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    }

    Lab { app, pool, admin, seen, acme, beta, a1, b1 }
}

/// Rend `user` membre du client, en plus de ses membres actuels.
async fn join(lab: &Lab, client: i64, user: i64) {
    let reply = lab.app.get("/api/clients", Some(&lab.admin)).await;
    let mut members: Vec<i64> = reply.body["clients"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == client)
        .unwrap()["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["user_id"].as_i64().unwrap())
        .collect();
    members.push(user);
    let reply = lab
        .app
        .put(&format!("/api/clients/{client}"), json!({ "members": members }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

fn ids(body: &Value) -> Vec<i64> {
    let mut ids: Vec<i64> =
        body.as_array().unwrap().iter().map(|t| t["id"].as_i64().unwrap()).collect();
    ids.sort_unstable();
    ids
}

/// Requête brute : le corps tel quel (PDF, CSV), avec un en-tête au choix.
async fn raw(
    app: &TestApp,
    uri: &str,
    cookie: &str,
    extra: Option<(&str, &str)>,
) -> (StatusCode, String) {
    let mut builder = Request::builder()
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header("x-requested-with", "DumbMonit");
    if let Some((name, value)) = extra {
        builder = builder.header(name, value);
    }
    let response =
        app.router.clone().oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn get_with_view(app: &TestApp, uri: &str, cookie: &str, client: i64) -> Reply {
    let (status, text) = raw(app, uri, cookie, Some((VIEW_HEADER, &client.to_string()))).await;
    Reply {
        status,
        body: serde_json::from_str(&text).unwrap_or(Value::Null),
        set_cookie: None,
        location: None,
    }
}

#[tokio::test]
async fn a_single_client_instance_sees_no_change_until_msp_mode_is_on() {
    let lab = lab().await;
    // Mode coupé : un compte sans équipe voit tout, comme avant.
    let reply = lab.app.put("/api/clients/settings", json!({ "enabled": false }), Some(&lab.admin));
    assert_eq!(reply.await.status, StatusCode::OK);
    let (_, cookie) = create_user(&lab.app, &lab.admin, "vera", "viewer").await;
    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(ids(&reply.body).len(), 2, "{}", reply.body);
    let reply = lab.app.get("/api/auth/me", Some(&cookie)).await;
    assert_eq!(reply.body["msp"]["enabled"], false, "{}", reply.body);
    assert_eq!(reply.body["access"]["scoped"], false);

    // Mode activé : un compte créé mode coupé est au client par défaut, qui
    // n'a rien ici.
    let reply = lab.app.put("/api/clients/settings", json!({ "enabled": true }), Some(&lab.admin));
    assert_eq!(reply.await.status, StatusCode::OK);
    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, json!([]));
    // Un compte créé mode activé n'appartient à aucun client : il ne voit rien.
    let (_, fresh) = create_user(&lab.app, &lab.admin, "nina", "viewer").await;
    let reply = lab.app.get("/api/targets", Some(&fresh)).await;
    assert_eq!(reply.body, json!([]));
    let reply = lab.app.get("/api/clients", Some(&lab.admin)).await;
    assert_eq!(reply.body["unassigned_users"].as_array().unwrap().len(), 1, "{}", reply.body);
}

#[tokio::test]
async fn a_client_member_sees_its_client_only() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;

    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(ids(&reply.body), vec![lab.a1]);

    // Détail et sous-routes d'un équipement de Beta : introuvables.
    for uri in [
        format!("/api/targets/{}", lab.b1),
        format!("/api/targets/{}/rules", lab.b1),
        format!("/api/targets/{}/security", lab.b1),
        format!("/api/targets/{}/probe-locations", lab.b1),
    ] {
        let reply = lab.app.get(&uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "GET {uri}: {}", reply.body);
    }

    // Alertes et historique : celles d'Acme seulement.
    for uri in ["/api/alerts", "/api/alerts/history"] {
        let reply = lab.app.get(uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::OK);
        let targets: Vec<i64> = reply
            .body
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["target_id"].as_i64().unwrap())
            .collect();
        assert_eq!(targets, vec![lab.a1], "GET {uri}: {}", reply.body);
    }

    // Ce que l'interface lit pour le sélecteur.
    let reply = lab.app.get("/api/auth/me", Some(&cookie)).await;
    assert_eq!(reply.body["access"]["scoped"], true, "{}", reply.body);
    assert_eq!(reply.body["msp"]["admin"], false);
    let names: Vec<&str> = reply.body["msp"]["clients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Acme"]);

    // L'administrateur MSP voit tout, et la liste de tous les clients.
    let reply = lab.app.get("/api/targets", Some(&lab.admin)).await;
    assert_eq!(reply.body.as_array().unwrap().len(), 2);
    let reply = lab.app.get("/api/auth/me", Some(&lab.admin)).await;
    assert_eq!(reply.body["msp"]["admin"], true);
    assert_eq!(reply.body["msp"]["clients"].as_array().unwrap().len(), 3, "{}", reply.body);
}

#[tokio::test]
async fn metric_queries_are_filtered_to_the_client() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;

    let query = format!("dumbmonit_up%7Btarget%3D%22{}%22%7D", lab.b1);
    for uri in [
        format!("/api/metrics/query?query={query}"),
        format!("/api/metrics/query_range?query={query}&start=0&end=600000"),
    ] {
        lab.seen.lock().unwrap().clear();
        let reply = lab.app.get(&uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri}: {}", reply.body);
        let seen = lab.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "{seen:?}");
        let decoded = urldecode(&seen[0]);
        let filter = format!("extra_filters[]={{target=~\"{}\"}}", lab.a1);
        assert!(decoded.contains(&filter), "{decoded}");
    }

    // Les séries brutes (fédération) restent fermées.
    let reply = lab.app.get("/federate?match%5B%5D=up", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
}

fn urldecode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
                out.push(u8::from_str_radix(hex, 16).unwrap());
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap()
}

/// Toutes les routes GET de la spécification, appelées par un membre d'Acme
/// avec l'identifiant de l'équipement — puis du client — de Beta : aucune
/// réponse ne doit nommer Beta.
#[tokio::test]
async fn no_read_route_leaks_another_client() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "operator").await;
    join(&lab, lab.acme, alice).await;

    let spec = lab.app.get("/api/openapi.json", None).await.body;
    let mut checked = 0;
    for (path, item) in spec["paths"].as_object().unwrap() {
        if item.get("get").is_none() || !path.starts_with("/api/") {
            continue;
        }
        for id in [lab.b1, lab.beta] {
            let mut uri = String::new();
            for segment in path.split('/').skip(1) {
                uri.push('/');
                if segment.starts_with('{') {
                    uri.push_str(&id.to_string());
                } else {
                    uri.push_str(segment);
                }
            }
            for uri in [uri.clone(), format!("{uri}?client={}", lab.beta)] {
                let reply = lab.app.get(&uri, Some(&cookie)).await;
                let text = reply.body.to_string();
                assert!(
                    !text.contains(SECRET_NAME)
                        && !text.contains(SECRET_ADDRESS)
                        && !text.contains("\"Beta\""),
                    "GET {uri} ({}) révèle le client Beta : {text}",
                    reply.status
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 100, "seulement {checked} routes lues");
}

#[tokio::test]
async fn routes_outside_the_closed_list_are_refused_to_client_members() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;
    for uri in [
        "/api/logs",
        "/api/backup",
        "/api/notify/channels",
        "/api/reports/schedules",
        "/api/agent/tokens",
        "/api/clients",
        "/api/clients/overview",
        "/api/clients/assignments",
        "/api/summary",
        "/api/teams",
        "/api/users",
    ] {
        let reply = lab.app.get(uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "GET {uri}: {}", reply.body);
    }
}

#[tokio::test]
async fn a_client_operator_cannot_touch_another_client() {
    let lab = lab().await;
    let (olga, cookie) = create_user(&lab.app, &lab.admin, "olga", "operator").await;
    join(&lab, lab.acme, olga).await;
    // Une équipe qui gère tous les dossiers… de ses clients seulement : le
    // niveau client passe avant le niveau dossier.
    let reply = lab
        .app
        .post(
            "/api/teams",
            json!({ "name": "Ops", "members": [olga],
                    "grants": [{ "folder": "*", "level": "manage" }] }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);

    // Traiter une alerte : la sienne oui, celle de Beta reste introuvable.
    let reply = lab.app.post("/api/alerts/fp-beta/ack", json!({}), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    let reply = lab.app.post("/api/alerts/fp-acme/ack", json!({}), Some(&cookie)).await;
    assert_ne!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert_ne!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);

    // Maintenance : sur son équipement, jamais sur celui de Beta ni partout.
    let schedule = json!({
        "kind": "weekly", "days": [0], "start_minute": 0, "end_minute": 60, "utc_offset_minutes": 0
    });
    let reply = lab
        .app
        .post(
            "/api/alerts/silences",
            json!({ "name": "beta", "target_id": lab.b1, "schedule": schedule }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    let reply = lab
        .app
        .post("/api/alerts/silences", json!({ "name": "all", "schedule": schedule }), Some(&cookie))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab
        .app
        .post(
            "/api/alerts/silences",
            json!({ "name": "acme", "target_id": lab.a1, "schedule": schedule }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let silence = reply.body["id"].as_i64().unwrap();
    // Le silence suit le client de l'équipement ; celui de Beta reste caché.
    let reply = lab.app.get("/api/clients/assignments", Some(&lab.admin)).await;
    assert_eq!(reply.body["silences"][silence.to_string()], lab.acme, "{}", reply.body);

    // Modifier ou supprimer l'équipement de Beta : introuvable.
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{}", lab.b1),
            json!({ "name": "pwned", "address": SECRET_ADDRESS, "kind": "dummy" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    let reply = lab.app.delete(&format!("/api/targets/{}", lab.b1), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    let reply = lab.app.post(&format!("/api/targets/{}/probe", lab.b1), json!({}), Some(&cookie));
    assert_eq!(reply.await.status, StatusCode::NOT_FOUND);

    // Créer chez Beta : refusé sans dire si Beta existe ; sans client : chez Acme.
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "x", "address": "10.1.0.9", "kind": "dummy", "client_id": lab.beta }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "y", "address": "10.1.0.10", "kind": "dummy" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["client_id"], lab.acme);
    let mine = reply.body["id"].as_i64().unwrap();
    // Un parent chez Beta : « n'existe pas ».
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{mine}"),
            json!({ "name": "y", "address": "10.1.0.10", "kind": "dummy", "parent_id": lab.b1 }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    // Se déplacer chez Beta : réservé à l'administrateur MSP.
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{mine}"),
            json!({ "name": "y", "address": "10.1.0.10", "kind": "dummy", "client_id": lab.beta }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let owner: i64 = sqlx::query_scalar("SELECT client_id FROM targets WHERE id = ?")
        .bind(mine)
        .fetch_one(&lab.pool)
        .await
        .unwrap();
    assert_eq!(owner, lab.acme);
}

#[tokio::test]
async fn rules_and_status_pages_of_another_client_stay_hidden() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;

    // Une règle de Beta.
    db::alerts::seed_builtin_rules(&lab.pool).await.unwrap();
    let reply = lab.app.get("/api/alerts/rules", Some(&lab.admin)).await;
    let rule = reply.body.as_array().unwrap()[0]["id"].as_i64().unwrap();
    let reply = lab
        .app
        .put(
            "/api/clients/assign",
            json!({ "kind": "rule", "ids": [rule], "client_id": lab.beta }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    let reply = lab.app.get("/api/alerts/rules", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!ids(&reply.body).contains(&rule), "{}", reply.body);

    // Une page de statut créée dans la vue de Beta lui appartient, et refuse
    // un équipement d'Acme.
    let (status, text) = {
        let request = Request::builder()
            .method("POST")
            .uri("/api/status-pages")
            .header(header::COOKIE, &lab.admin)
            .header("x-requested-with", "DumbMonit")
            .header(VIEW_HEADER, lab.beta.to_string())
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "slug": "beta", "title": "Beta status" }).to_string()))
            .unwrap();
        let response = lab.app.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    };
    assert_eq!(status, StatusCode::CREATED, "{text}");
    let page: Value = serde_json::from_str(&text).unwrap();
    let page = page["id"].as_i64().unwrap();
    let reply = lab
        .app
        .put(
            &format!("/api/status-pages/{page}/items"),
            json!([{ "target_id": lab.a1 }]),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let reply = lab
        .app
        .put(
            &format!("/api/status-pages/{page}/items"),
            json!([{ "target_id": lab.b1 }]),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    // Le membre d'Acme ne la voit pas dans la liste.
    let reply = lab.app.get("/api/status-pages", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!reply.body.to_string().contains("Beta status"), "{}", reply.body);
}

#[tokio::test]
async fn the_admin_view_filters_lists_without_restricting() {
    let lab = lab().await;
    let reply = get_with_view(&lab.app, "/api/targets", &lab.admin, lab.acme).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(ids(&reply.body), vec![lab.a1]);
    let reply = get_with_view(&lab.app, "/api/auth/me", &lab.admin, lab.acme).await;
    assert_eq!(reply.body["msp"]["current"], lab.acme, "{}", reply.body);
    assert_eq!(reply.body["access"]["scoped"], false);
    // Pas de barrière : les routes d'administration restent ouvertes.
    let reply = get_with_view(&lab.app, "/api/clients", &lab.admin, lab.acme).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    // L'en-tête ne donne rien à un membre d'Acme qui viserait Beta.
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;
    let reply = get_with_view(&lab.app, "/api/targets", &cookie, lab.beta).await;
    assert_eq!(ids(&reply.body), vec![lab.a1], "{}", reply.body);
}

#[tokio::test]
async fn the_sla_report_of_a_client_names_nothing_of_another() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Le rapport de Beta : introuvable pour un membre d'Acme.
    let uri = format!("/api/reports/sla?from={today}&to={today}&format=csv&client={}", lab.beta);
    let (status, _) = raw(&lab.app, &uri, &cookie, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Le sien : ses équipements et ses incidents seulement.
    for format in ["csv", "incidents_csv", "pdf"] {
        let uri = format!("/api/reports/sla?from={today}&to={today}&format={format}");
        let (status, text) = raw(&lab.app, &uri, &cookie, None).await;
        assert_eq!(status, StatusCode::OK, "{format}: {text}");
        assert!(!text.contains(SECRET_NAME), "{format}: {text}");
        assert!(!text.contains("Removed device"), "{format}: {text}");
        if format == "incidents_csv" {
            assert!(text.contains("acme-router"), "{text}");
            assert_eq!(text.lines().count(), 2, "one header, one incident: {text}");
        }
    }

    // L'administrateur, pour Acme : la même chose.
    let uri = format!("/api/reports/sla?from={today}&to={today}&format=csv&client={}", lab.acme);
    let (status, text) = raw(&lab.app, &uri, &lab.admin, None).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("acme-router") && !text.contains(SECRET_NAME), "{text}");
}

#[tokio::test]
async fn logos_are_shown_to_the_client_members_only() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;
    for client in [lab.acme, lab.beta] {
        let reply = lab
            .app
            .put(
                &format!("/api/clients/{client}/logo"),
                json!({ "data": "data:image/png;base64,iVBORw0KGgo=" }),
                Some(&lab.admin),
            )
            .await;
        assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    }
    let acme_logo = format!("/api/clients/{}/logo", lab.acme);
    let (status, _) = raw(&lab.app, &acme_logo, &cookie, None).await;
    assert_eq!(status, StatusCode::OK);
    let beta_logo = format!("/api/clients/{}/logo", lab.beta);
    let (status, _) = raw(&lab.app, &beta_logo, &cookie, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Pas de SVG : il peut porter du script.
    let reply = lab
        .app
        .put(
            &format!("/api/clients/{}/logo", lab.acme),
            json!({ "data": "PHN2Zz48c2NyaXB0Lz48L3N2Zz4=" }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
}

/// Un lot vide poussé au nom d'une machine, avec un jeton d'enrôlement.
async fn push(app: &TestApp, enrolment: &str, machine_id: &str, hostname: &str) -> Reply {
    let batch = json!({
        "protocol": 1,
        "identity": {
            "hostname": hostname,
            "os": "linux",
            "agent_version": "0.9.0",
            "machine_id": machine_id,
            "commands_enabled": false,
            "binding_supported": true,
        },
        "sent_at_ms": 0,
        "samples": [],
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/ingest")
        .header(header::AUTHORIZATION, format!("Bearer {enrolment}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(batch.to_string()))
        .unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie: None,
        location: None,
    }
}

#[tokio::test]
async fn an_agent_token_of_a_client_only_enrols_into_that_client() {
    let lab = lab().await;
    let reply = lab
        .app
        .post(
            "/api/agent/tokens",
            json!({ "name": "acme fleet", "reusable": true, "client_id": lab.acme }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["client_id"], lab.acme);
    let acme_token = reply.body["secret"].as_str().unwrap().to_string();

    // Une machine nouvelle entre chez Acme.
    let reply = push(&lab.app, &acme_token, "id-acme-1", "acme-1").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let target = reply.body["target_id"].as_i64().unwrap();
    let owner: i64 = sqlx::query_scalar("SELECT client_id FROM targets WHERE id = ?")
        .bind(target)
        .fetch_one(&lab.pool)
        .await
        .unwrap();
    assert_eq!(owner, lab.acme);

    // Une machine déclarée chez Beta (cible « agent » créée à la main) : le
    // jeton d'Acme ne l'adopte pas et rien n'entre chez Beta.
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "beta-host", "address": "id-beta-1", "kind": "agent",
                    "client_id": lab.beta }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let beta_host = reply.body["id"].as_i64().unwrap();
    let reply = push(&lab.app, &acme_token, "id-beta-1", "beta-host").await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let bound: Option<i64> =
        sqlx::query_scalar("SELECT target_id FROM agent_hosts WHERE target_id = ?")
            .bind(beta_host)
            .fetch_optional(&lab.pool)
            .await
            .unwrap();
    assert_eq!(bound, None, "no agent bound to Beta's target");
    let beta_targets: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM targets WHERE client_id = ?")
            .bind(lab.beta)
            .fetch_one(&lab.pool)
            .await
            .unwrap();
    assert_eq!(beta_targets, 2, "Beta has its NAS and its declared host, nothing more");

    // Le quota d'Acme atteint : plus d'enrôlement.
    let reply = lab
        .app
        .put(&format!("/api/clients/{}", lab.acme), json!({ "monitor_quota": 2 }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = push(&lab.app, &acme_token, "id-acme-2", "acme-2").await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "z", "address": "10.1.0.99", "kind": "dummy", "client_id": lab.acme }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
}

#[tokio::test]
async fn moving_a_target_takes_its_dependents_and_cuts_cross_client_links() {
    let lab = lab().await;
    let child = {
        let reply = lab
            .app
            .post(
                "/api/targets",
                json!({ "name": "child", "address": "10.1.0.5", "kind": "dummy",
                        "parent_id": lab.a1, "client_id": lab.acme }),
                Some(&lab.admin),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
        reply.body["id"].as_i64().unwrap()
    };
    // Un parent d'un autre client : refusé à la création.
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "cross", "address": "10.1.0.6", "kind": "dummy",
                    "parent_id": lab.b1, "client_id": lab.acme }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);

    let reply = lab
        .app
        .put(
            "/api/clients/assign",
            json!({ "kind": "target", "ids": [lab.a1], "client_id": lab.beta }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    let reply = lab.app.get("/api/clients/assignments", Some(&lab.admin)).await;
    assert_eq!(reply.body["targets"][lab.a1.to_string()], lab.beta, "{}", reply.body);
    assert_eq!(reply.body["targets"][child.to_string()], lab.beta, "{}", reply.body);

    // Un client qui a encore des équipements ne se supprime pas.
    let reply = lab.app.delete(&format!("/api/clients/{}", lab.beta), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    let reply = lab.app.delete(&format!("/api/clients/{}", lab.acme), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    let reply = lab.app.delete("/api/clients/1", Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
}

#[tokio::test]
async fn clients_are_administered_by_the_msp_admin_only() {
    let lab = lab().await;
    let (alice, cookie) = create_user(&lab.app, &lab.admin, "alice", "operator").await;
    join(&lab, lab.acme, alice).await;
    let reply = lab.app.post("/api/clients", json!({ "name": "X" }), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab
        .app
        .put(&format!("/api/clients/{}", lab.acme), json!({ "members": [alice] }), Some(&cookie))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab
        .app
        .put(
            "/api/clients/assign",
            json!({ "kind": "target", "ids": [lab.b1], "client_id": lab.acme }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab.app.put("/api/clients/settings", json!({ "enabled": false }), Some(&cookie));
    assert_eq!(reply.await.status, StatusCode::FORBIDDEN);
    // Validation : nom en double, quota négatif, contact invalide.
    let reply = lab.app.post("/api/clients", json!({ "name": "acme" }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    let uri = format!("/api/clients/{}", lab.acme);
    let reply = lab.app.put(&uri, json!({ "monitor_quota": -1 }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let reply = lab
        .app
        .put(
            &format!("/api/clients/{}", lab.acme),
            json!({ "contacts": ["not an address"] }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
}

#[tokio::test]
async fn identity_provider_groups_drive_client_membership() {
    let lab = lab().await;
    let (viewer, cookie) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    let reply = lab
        .app
        .put(
            &format!("/api/clients/{}", lab.acme),
            json!({ "idp_groups": ["customers-acme"] }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    use dumbmonit_server::auth::clients::sync_idp_groups;
    sync_idp_groups(&lab.pool, viewer, &["cn=Customers-Acme,ou=Groups,dc=example,dc=org".into()])
        .await
        .unwrap();
    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(ids(&reply.body), vec![lab.a1], "{}", reply.body);
    sync_idp_groups(&lab.pool, viewer, &["other".into()]).await.unwrap();
    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(reply.body, json!([]), "{}", reply.body);
}

#[tokio::test]
async fn a_backup_carries_clients_and_what_belongs_to_them() {
    let lab = lab().await;
    let (alice, _) = create_user(&lab.app, &lab.admin, "alice", "viewer").await;
    join(&lab, lab.acme, alice).await;
    let reply = lab
        .app
        .put(
            &format!("/api/clients/{}", lab.beta),
            json!({ "contacts": ["ops@beta.example"], "monitor_quota": 10, "sla_report": true }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    let cipher = db::init_cipher(&lab.pool, "secret-de-test-suffisamment-long").await.unwrap();
    let bundle = backup::collect(&lab.pool, &cipher, backup::ExportOptions::default())
        .await
        .unwrap();
    assert_eq!(bundle.summary()["clients"], 3);

    let dir = tempfile::tempdir().unwrap();
    let pool = db::open(&dir.path().join("restored.db")).await.unwrap();
    let cipher = db::init_cipher(&pool, "un-autre-secret-de-test-assez-long").await.unwrap();
    let report = backup::restore(&pool, &cipher, &bundle, true).await.unwrap();
    let section = report.sections.iter().find(|s| s.section == "clients").unwrap();
    assert!(section.created >= 2, "{section:?}");

    let restored: Vec<(String, i64, Option<i64>, String)> = sqlx::query_as(
        "SELECT c.name, t.client_id, c.monitor_quota, c.contacts
         FROM targets t JOIN clients c ON c.id = t.client_id ORDER BY t.name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let acme = restored.iter().find(|r| r.0 == "Acme").expect("Acme's target");
    let beta = restored.iter().find(|r| r.0 == "Beta").expect("Beta's target");
    assert_ne!(acme.1, beta.1);
    assert_eq!(beta.2, Some(10));
    assert!(beta.3.contains("ops@beta.example"));
    let member: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM client_members m JOIN users u ON u.id = m.user_id
         JOIN clients c ON c.id = m.client_id
         WHERE u.username = 'alice' AND c.name = 'Acme'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(member, 1);
    let enabled: Vec<u8> = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'msp'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&enabled).contains("true"));
}
