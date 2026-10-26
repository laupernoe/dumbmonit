//! Équipes et droits par dossier : ce qu'un compte restreint voit et fait.
//!
//! Le scénario : deux dossiers, `Paris` (et son sous-dossier `Paris/Rack`) et
//! `Lyon`, un équipement sans dossier, et deux équipes. Un lecteur de l'équipe
//! Paris ne doit rien voir de Lyon — ni par la liste, ni par le détail, ni par
//! les métriques, les alertes, la carte, la recherche ou une route oubliée.

mod common;

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dumbmonit_server::db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{PASSWORD, TestApp};

const SECRET_NAME: &str = "lyon-secret-nas";
const SECRET_ADDRESS: &str = "10.66.66.66";

/// Faux VictoriaMetrics : répond une série vide et garde chaque chaîne de
/// requête reçue, pour vérifier le filtre d'équipements.
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
    paris: i64,
    rack: i64,
    lyon: i64,
    unfiled: i64,
    paris_team: i64,
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

async fn create_target(app: &TestApp, admin: &str, name: &str, address: &str, folder: &str) -> i64 {
    let reply = app
        .post(
            "/api/targets",
            json!({ "name": name, "address": address, "kind": "dummy", "group_name": folder }),
            Some(admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
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

    let paris = create_target(&app, &admin, "paris-router", "10.1.0.1", "Paris").await;
    let rack = create_target(&app, &admin, "paris-switch", "10.1.0.2", "Paris/Rack").await;
    let lyon = create_target(&app, &admin, SECRET_NAME, SECRET_ADDRESS, "Lyon").await;
    let unfiled = create_target(&app, &admin, "loose-box", "10.9.0.1", "").await;

    let reply = app
        .post(
            "/api/teams",
            json!({ "name": "Paris", "grants": [{ "folder": "Paris", "level": "read" }] }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let paris_team = reply.body["id"].as_i64().unwrap();
    let reply = app
        .post(
            "/api/teams",
            json!({ "name": "Lyon", "grants": [{ "folder": "Lyon", "level": "manage" }] }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);

    // Une alerte active par équipement, Paris et Lyon.
    for (fingerprint, target) in [("fp-paris", paris), ("fp-lyon", lyon)] {
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

    Lab { app, pool, admin, seen, paris, rack, lyon, unfiled, paris_team }
}

/// Inscrit `user` dans l'équipe, en plus de ses membres actuels.
async fn join(lab: &Lab, team: i64, user: i64) {
    let reply = lab.app.get("/api/teams", Some(&lab.admin)).await;
    let mut members: Vec<i64> = reply.body["teams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == team)
        .unwrap()["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["user_id"].as_i64().unwrap())
        .collect();
    members.push(user);
    let reply = lab
        .app
        .put(&format!("/api/teams/{team}"), json!({ "members": members }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

fn ids(body: &Value) -> Vec<i64> {
    let mut ids: Vec<i64> =
        body.as_array().unwrap().iter().map(|t| t["id"].as_i64().unwrap()).collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn a_team_reader_sees_its_folder_and_subfolders_only() {
    let lab = lab().await;
    let (viewer, cookie) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    join(&lab, lab.paris_team, viewer).await;

    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let mut expected = vec![lab.paris, lab.rack];
    expected.sort_unstable();
    assert_eq!(ids(&reply.body), expected);

    // Le détail et les sous-routes d'un équipement de Lyon : introuvables.
    for uri in [
        format!("/api/targets/{}", lab.lyon),
        format!("/api/targets/{}/rules", lab.lyon),
        format!("/api/targets/{}/security", lab.lyon),
        format!("/api/targets/{}", lab.unfiled),
    ] {
        let reply = lab.app.get(&uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "GET {uri}: {}", reply.body);
    }
    let reply = lab.app.get(&format!("/api/targets/{}", lab.rack), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    // Alertes et historique : celles de Paris seulement.
    let reply = lab.app.get("/api/alerts", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
    let targets: Vec<i64> =
        reply.body.as_array().unwrap().iter().map(|a| a["target_id"].as_i64().unwrap()).collect();
    assert_eq!(targets, vec![lab.paris], "{}", reply.body);
    let reply = lab.app.get("/api/alerts/history", Some(&cookie)).await;
    let targets: Vec<i64> =
        reply.body.as_array().unwrap().iter().map(|a| a["target_id"].as_i64().unwrap()).collect();
    assert_eq!(targets, vec![lab.paris], "{}", reply.body);

    // L'administrateur, lui, voit tout.
    let reply = lab.app.get("/api/targets", Some(&lab.admin)).await;
    assert_eq!(reply.body.as_array().unwrap().len(), 4);

    // Ce que l'interface lit pour masquer la navigation.
    let reply = lab.app.get("/api/auth/me", Some(&cookie)).await;
    assert_eq!(reply.body["access"]["scoped"], true, "{}", reply.body);
    assert_eq!(reply.body["access"]["teams"], json!(["Paris"]));
    assert_eq!(reply.body["access"]["ceiling"], "read");
    let reply = lab.app.get("/api/auth/me", Some(&lab.admin)).await;
    assert_eq!(reply.body["access"]["scoped"], false);
}

#[tokio::test]
async fn metric_queries_are_filtered_to_the_visible_targets() {
    let lab = lab().await;
    let (viewer, cookie) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    join(&lab, lab.paris_team, viewer).await;

    let query = format!("dumbmonit_up{{target=\"{}\"}}", lab.lyon);
    for uri in [
        format!("/api/metrics/query?query={}", urlencode(&query)),
        format!("/api/metrics/query_range?query={}&start=0&end=600000", urlencode(&query)),
    ] {
        lab.seen.lock().unwrap().clear();
        let reply = lab.app.get(&uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri}: {}", reply.body);
        let seen = lab.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "{seen:?}");
        let decoded = urldecode(&seen[0]);
        let mut ids = [lab.paris, lab.rack];
        ids.sort_unstable();
        let filter = format!("extra_filters[]={{target=~\"{}|{}\"}}", ids[0], ids[1]);
        assert!(decoded.contains(&filter), "{decoded}");
    }

    // Sans équipe d'accueil (réglage « rien »), le filtre ne laisse rien passer.
    let (_, lonely) = create_user(&lab.app, &lab.admin, "bob", "viewer").await;
    let reply = lab
        .app
        .put("/api/teams/settings", json!({ "unassigned_access": "none" }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    lab.seen.lock().unwrap().clear();
    let reply = lab.app.get("/api/metrics/query?query=up", Some(&lonely)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let decoded = urldecode(&lab.seen.lock().unwrap()[0]);
    assert!(decoded.contains("extra_filters[]={target=\"-1\"}"), "{decoded}");
    let reply = lab.app.get("/api/targets", Some(&lonely)).await;
    assert_eq!(reply.body, json!([]));

    // L'administrateur interroge sans filtre.
    lab.seen.lock().unwrap().clear();
    let reply = lab.app.get("/api/metrics/query?query=up", Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!urldecode(&lab.seen.lock().unwrap()[0]).contains("extra_filters"));
}

fn urlencode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
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

/// Toutes les routes GET de la spécification, appelées par un lecteur de
/// l'équipe Paris : aucune réponse ne doit nommer l'équipement de Lyon.
#[tokio::test]
async fn no_read_route_leaks_another_teams_target() {
    let lab = lab().await;
    let (viewer, cookie) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    join(&lab, lab.paris_team, viewer).await;

    let spec = lab.app.get("/api/openapi.json", None).await.body;
    let mut checked = 0;
    for (path, item) in spec["paths"].as_object().unwrap() {
        if item.get("get").is_none() || !path.starts_with("/api/") {
            continue;
        }
        // Les paramètres de chemin désignent l'équipement de Lyon quand ils le
        // peuvent : c'est lui qu'on essaie d'atteindre.
        let mut uri = String::new();
        for segment in path.split('/').skip(1) {
            uri.push('/');
            if segment.starts_with('{') {
                uri.push_str(&lab.lyon.to_string());
            } else {
                uri.push_str(segment);
            }
        }
        let reply = lab.app.get(&uri, Some(&cookie)).await;
        let text = reply.body.to_string();
        assert!(
            !text.contains(SECRET_NAME) && !text.contains(SECRET_ADDRESS),
            "GET {uri} ({}) révèle l'équipement de Lyon : {text}",
            reply.status
        );
        checked += 1;
    }
    assert!(checked > 50, "seulement {checked} routes lues");
}

#[tokio::test]
async fn routes_outside_the_team_list_are_refused() {
    let lab = lab().await;
    let (viewer, cookie) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    join(&lab, lab.paris_team, viewer).await;
    for uri in [
        "/api/logs",
        "/api/relays",
        "/api/self-monitoring",
        "/api/backup",
        "/api/alerts/rules/1/overrides",
        "/api/teams",
        "/api/incidents",
        "/api/security/summary",
        "/api/wall/widgets",
        "/api/notify/channels",
        "/api/alerts/overrides",
    ] {
        let reply = lab.app.get(uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "GET {uri}: {}", reply.body);
    }
    // Les séries brutes et la fédération Prometheus non plus.
    let reply = lab.app.get("/federate?match%5B%5D=up", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    // Les routes de son compte restent ouvertes.
    let reply = lab.app.get("/api/auth/me", Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::OK);
}

#[tokio::test]
async fn operating_and_managing_follow_the_folder_level_and_the_role() {
    let lab = lab().await;
    let (operator, cookie) = create_user(&lab.app, &lab.admin, "olga", "operator").await;
    let reply = lab
        .app
        .put(
            &format!("/api/teams/{}", lab.paris_team),
            json!({
                "members": [operator],
                "grants": [
                    { "folder": "Paris", "level": "operate" },
                    { "folder": "Paris/Rack", "level": "manage" }
                ]
            }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    // Traiter une alerte : la sienne oui, celle de Lyon reste introuvable.
    let reply = lab.app.post("/api/alerts/fp-lyon/ack", json!({}), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    let reply = lab.app.post("/api/alerts/fp-paris/ack", json!({}), Some(&cookie)).await;
    assert_ne!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert_ne!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);

    // Maintenance : sur un équipement qu'il traite, pas sur toute l'instance.
    let schedule = json!({
        "kind": "weekly", "days": [0], "start_minute": 0, "end_minute": 60, "utc_offset_minutes": 0
    });
    let reply = lab
        .app
        .post("/api/alerts/silences", json!({ "name": "all", "schedule": schedule }), Some(&cookie))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab
        .app
        .post(
            "/api/alerts/silences",
            json!({ "name": "lyon", "target_id": lab.lyon, "schedule": schedule }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    let reply = lab
        .app
        .post(
            "/api/alerts/silences",
            json!({ "name": "paris", "target_id": lab.paris, "schedule": schedule }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);

    // Modifier : seulement dans le dossier géré.
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{}", lab.rack),
            json!({ "name": "renamed", "address": "10.1.0.2", "kind": "dummy" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{}", lab.paris),
            json!({ "name": "renamed", "address": "10.1.0.1", "kind": "dummy" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{}", lab.lyon),
            json!({ "name": "renamed", "address": SECRET_ADDRESS, "kind": "dummy" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    // Déplacer hors du dossier géré : refusé.
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{}", lab.rack),
            json!({ "name": "s", "address": "10.1.0.2", "kind": "dummy", "group_name": "Lyon" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    // Créer : dans le dossier géré oui, ailleurs non.
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "new", "address": "10.1.0.9", "kind": "dummy", "group_name": "Paris/Rack" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let reply = lab
        .app
        .post(
            "/api/targets",
            json!({ "name": "new2", "address": "10.1.0.10", "kind": "dummy", "group_name": "Lyon" }),
            Some(&cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    // Ce que le rôle ne permet pas reste fermé, équipe ou non.
    let reply = lab.app.post("/api/notify/channels", json!({}), Some(&cookie)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);

    // Un lecteur dans une équipe « gestion » reste plafonné à la lecture.
    let (viewer, reader) = create_user(&lab.app, &lab.admin, "vera", "viewer").await;
    let reply = lab
        .app
        .post(
            "/api/teams",
            json!({ "name": "Rack", "members": [viewer], "grants": [{ "folder": "Paris/Rack", "level": "manage" }] }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{}", lab.rack),
            json!({ "name": "nope", "address": "10.1.0.2", "kind": "dummy" }),
            Some(&reader),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);

    // Un opérateur sans équipe garde l'ancien comportement : pas d'édition.
    let (_, free) = create_user(&lab.app, &lab.admin, "omar", "operator").await;
    let reply = lab
        .app
        .put(
            &format!("/api/targets/{}", lab.rack),
            json!({ "name": "nope", "address": "10.1.0.2", "kind": "dummy" }),
            Some(&free),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let reply = lab.app.get("/api/targets", Some(&free)).await;
    assert_eq!(reply.body.as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn a_token_inherits_the_teams_of_its_owner() {
    let lab = lab().await;
    let (viewer, _) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    join(&lab, lab.paris_team, viewer).await;
    let reply = lab
        .app
        .post("/api/tokens", json!({ "name": "script", "scope": "read" }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let token = reply.body["secret"].as_str().unwrap().to_string();
    let token_id = reply.body["id"].as_i64().unwrap();
    sqlx::query("UPDATE api_tokens SET user_id = ? WHERE id = ?")
        .bind(viewer)
        .bind(token_id)
        .execute(&lab.pool)
        .await
        .unwrap();

    let bearer = |uri: &str| {
        Request::builder()
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    };
    let response = lab.app.router.clone().oneshot(bearer("/api/targets")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let mut expected = vec![lab.paris, lab.rack];
    expected.sort_unstable();
    assert_eq!(ids(&body), expected);
    let uri = format!("/api/targets/{}", lab.lyon);
    let response = lab.app.router.clone().oneshot(bearer(&uri)).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = lab.app.router.clone().oneshot(bearer("/api/logs")).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    // Les outils de l'assistant lisent toute l'instance : refusés.
    let mcp = Request::builder()
        .method("POST")
        .uri("/api/mcp")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }).to_string()))
        .unwrap();
    let response = lab.app.router.clone().oneshot(mcp).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn identity_provider_groups_drive_team_membership() {
    let lab = lab().await;
    let (viewer, cookie) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    let reply = lab
        .app
        .put(
            &format!("/api/teams/{}", lab.paris_team),
            json!({ "idp_groups": ["noc-paris"] }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);

    use dumbmonit_server::auth::teams::sync_idp_groups;
    sync_idp_groups(&lab.pool, viewer, &["cn=NOC-Paris,ou=Groups,dc=example,dc=org".into()])
        .await
        .unwrap();
    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(reply.body.as_array().unwrap().len(), 2, "{}", reply.body);

    // Le groupe perdu, le compte sort de l'équipe… et revoit tout (réglage
    // par défaut pour un compte sans équipe).
    sync_idp_groups(&lab.pool, viewer, &["other".into()]).await.unwrap();
    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(reply.body.as_array().unwrap().len(), 4, "{}", reply.body);

    // Un membre inscrit à la main n'est jamais retiré par la synchronisation.
    join(&lab, lab.paris_team, viewer).await;
    sync_idp_groups(&lab.pool, viewer, &[]).await.unwrap();
    let reply = lab.app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(reply.body.as_array().unwrap().len(), 2, "{}", reply.body);
}

#[tokio::test]
async fn teams_are_administered_by_admins_only_and_validated() {
    let lab = lab().await;
    let (_, viewer) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    let reply = lab.app.get("/api/teams", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let reply = lab.app.post("/api/teams", json!({ "name": "X" }), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);

    let reply = lab.app.post("/api/teams", json!({ "name": "paris" }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "nom déjà pris, sans casse : {}", reply.body);
    let reply = lab.app.post("/api/teams", json!({ "name": "  " }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = lab
        .app
        .post("/api/teams", json!({ "name": "Ghost", "members": [9999] }), Some(&lab.admin))
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    let reply = lab.app.get("/api/teams", Some(&lab.admin)).await;
    assert_eq!(reply.body["folders"], json!(["Lyon", "Paris", "Paris/Rack"]));
    assert_eq!(reply.body["settings"]["unassigned_access"], "all");

    let reply = lab.app.delete(&format!("/api/teams/{}", lab.paris_team), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let reply = lab.app.delete(&format!("/api/teams/{}", lab.paris_team), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let _ = PASSWORD;
}

#[tokio::test]
async fn teams_travel_in_the_backup() {
    let lab = lab().await;
    let (viewer, _) = create_user(&lab.app, &lab.admin, "anna", "viewer").await;
    join(&lab, lab.paris_team, viewer).await;
    let bundle = dumbmonit_server::backup::export::collect(
        &lab.pool,
        &db::init_cipher(&lab.pool, "secret-de-test-suffisamment-long").await.unwrap(),
        Default::default(),
    )
    .await
    .unwrap();
    let paris = bundle.teams.iter().find(|t| t.name == "Paris").unwrap();
    assert_eq!(paris.members[0].username, "anna");
    assert_eq!(paris.grants[0].folder, "Paris");

    sqlx::query("DELETE FROM teams").execute(&lab.pool).await.unwrap();
    let cipher = db::init_cipher(&lab.pool, "secret-de-test-suffisamment-long").await.unwrap();
    dumbmonit_server::backup::import::restore(&lab.pool, &cipher, &bundle, true).await.unwrap();
    let reply = lab.app.get("/api/teams", Some(&lab.admin)).await;
    let names: Vec<&str> = reply.body["teams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Lyon", "Paris"]);
}
