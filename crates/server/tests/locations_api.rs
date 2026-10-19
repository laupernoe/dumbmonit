//! Emplacements de la carte du monde : création, affectation explicite ou par
//! site, site principal, et fond de carte (avec son effet sur la politique de
//! sécurité de l'interface).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

use common::TestApp;

async fn create_target(app: &TestApp, admin: &str, body: Value) -> i64 {
    let reply = app.post("/api/targets", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "création : {}", reply.body);
    reply.body["id"].as_i64().expect("id")
}

async fn create_location(app: &TestApp, admin: &str, body: Value) -> i64 {
    let reply = app.post("/api/locations", body, Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "emplacement : {}", reply.body);
    reply.body["id"].as_i64().expect("id")
}

/// Membres d'un emplacement : (cible, source).
fn members(body: &Value, location: i64) -> Vec<(i64, String)> {
    let found = body["locations"]
        .as_array()
        .expect("locations")
        .iter()
        .find(|l| l["id"] == location)
        .expect("emplacement listé");
    found["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| (m["target_id"].as_i64().unwrap(), m["source"].as_str().unwrap().to_string()))
        .collect()
}

#[tokio::test]
async fn les_cibles_trouvent_leur_emplacement() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let routeur = create_target(
        &app,
        &admin,
        json!({ "name": "routeur", "address": "10.0.0.1", "kind": "dummy" }),
    )
    .await;
    let nas = create_target(
        &app,
        &admin,
        json!({ "name": "nas", "address": "10.1.0.2", "kind": "dummy", "tags": { "site": "Annexe" } }),
    )
    .await;
    let camera = create_target(
        &app,
        &admin,
        json!({ "name": "camera", "address": "10.2.0.3", "kind": "dummy", "tags": { "site": "Cave" } }),
    )
    .await;

    let siege = create_location(
        &app,
        &admin,
        json!({ "name": "Siège", "address": "Lausanne", "latitude": 46.52, "longitude": 6.63,
                "main_site": true }),
    )
    .await;
    let annexe = create_location(
        &app,
        &admin,
        json!({ "name": "Annexe", "latitude": 48.85, "longitude": 2.35, "site": "annexe" }),
    )
    .await;

    let viewer = app.viewer_cookie(&admin).await;
    let reply = app.get("/api/locations", Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(members(&reply.body, siege), vec![(routeur, "main".to_string())]);
    assert_eq!(members(&reply.body, annexe), vec![(nas, "site".to_string())]);
    assert_eq!(reply.body["unplaced"], json!([camera]));
    assert_eq!(reply.body["sites"], json!(["Annexe", "Cave"]));

    // En masse depuis la liste, puis par le formulaire du moniteur.
    let reply = app
        .post(
            "/api/locations/assign",
            json!({ "location_id": annexe, "target_ids": [camera, 999] }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["updated"], 1, "une cible inconnue est ignorée");
    let reply = app.get(&format!("/api/targets/{camera}"), Some(&admin)).await;
    assert_eq!(reply.body["location_id"], annexe);

    let mut edit = reply.body.clone();
    edit["location_id"] = json!(siege);
    let reply = app.put(&format!("/api/targets/{camera}"), edit, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["location_id"], siege);

    // Un PUT qui ne connaît pas le champ garde l'affectation.
    let reply = app
        .put(
            &format!("/api/targets/{camera}"),
            json!({ "name": "camera", "address": "10.2.0.3", "kind": "dummy",
                    "tags": { "site": "Cave" } }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["location_id"], siege);

    let reply = app.get("/api/locations", Some(&admin)).await;
    assert_eq!(
        members(&reply.body, siege),
        vec![(routeur, "main".to_string()), (camera, "assigned".to_string())]
    );
    assert_eq!(reply.body["unplaced"], json!([]));

    // Un emplacement inconnu est une erreur de saisie.
    let reply = app
        .post(
            "/api/locations/assign",
            json!({ "location_id": 77, "target_ids": [nas] }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app
        .post(
            "/api/targets",
            json!({ "name": "x", "address": "10.9.9.9", "kind": "dummy", "location_id": 77 }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    // Supprimer l'emplacement laisse ses cibles, sans affectation.
    assert_eq!(
        app.delete(&format!("/api/locations/{siege}"), Some(&admin)).await.status,
        StatusCode::NO_CONTENT
    );
    let reply = app.get(&format!("/api/targets/{camera}"), Some(&admin)).await;
    assert_eq!(reply.body["location_id"], Value::Null);
    let reply = app.get("/api/locations", Some(&admin)).await;
    assert_eq!(reply.body["unplaced"], json!([routeur, camera]));
}

#[tokio::test]
async fn un_seul_site_principal_et_des_noms_uniques() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let a = create_location(
        &app,
        &admin,
        json!({ "name": "A", "latitude": 0, "longitude": 0, "main_site": true }),
    )
    .await;
    let b = create_location(
        &app,
        &admin,
        json!({ "name": "B", "latitude": 1, "longitude": 1, "main_site": true }),
    )
    .await;
    let reply = app.get("/api/locations", Some(&admin)).await;
    let main: Vec<i64> = reply.body["locations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["main_site"] == true)
        .map(|l| l["id"].as_i64().unwrap())
        .collect();
    assert_eq!(main, vec![b], "le dernier marqué l'emporte");

    let reply = app
        .post("/api/locations", json!({ "name": "a", "latitude": 2, "longitude": 2 }), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "noms comparés sans casse");
    let reply = app
        .put(
            &format!("/api/locations/{a}"),
            json!({ "name": "A", "latitude": 95, "longitude": 0 }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = app
        .put(
            "/api/locations/404",
            json!({ "name": "Z", "latitude": 0, "longitude": 0 }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    // Un lecteur lit, n'écrit pas.
    let viewer = app.viewer_cookie(&admin).await;
    let reply = app
        .post(
            "/api/locations",
            json!({ "name": "C", "latitude": 0, "longitude": 0 }),
            Some(&viewer),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/locations", None).await.status, StatusCode::UNAUTHORIZED);
}

async fn csp(app: &TestApp, cookie: &str) -> String {
    let request = Request::builder()
        .uri("/api/locations/basemap")
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    response
        .headers()
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

#[tokio::test]
async fn le_fond_de_carte_ouvre_seulement_son_origine() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;

    let reply = app.get("/api/locations/basemap", Some(&admin)).await;
    assert_eq!(reply.body["mode"], "offline", "hors ligne par défaut");
    assert!(!csp(&app, &admin).await.contains("openstreetmap"));

    let reply = app.put("/api/locations/basemap", json!({ "mode": "osm" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["attribution"], "© OpenStreetMap contributors");
    let policy = csp(&app, &admin).await;
    let img = policy.split(';').find(|d| d.trim().starts_with("img-src")).expect("img-src");
    assert!(img.contains("https://tile.openstreetmap.org"), "{img}");

    let reply = app
        .put(
            "/api/locations/basemap",
            json!({ "mode": "custom", "url": "https://{s}.tiles.example/{z}/{x}/{y}.png",
                    "attribution": "Moi" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "un joker de sous-domaine est refusé");

    let reply = app
        .put(
            "/api/locations/basemap",
            json!({ "mode": "custom", "url": "http://tiles.lan:8080/{z}/{x}/{y}.png",
                    "attribution": "Mes tuiles" }),
            Some(&admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let policy = csp(&app, &admin).await;
    assert!(policy.contains("http://tiles.lan:8080"), "{policy}");
    assert!(!policy.contains("openstreetmap"), "l'ancienne origine disparaît");

    let reply = app.put("/api/locations/basemap", json!({ "mode": "offline" }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!csp(&app, &admin).await.contains("tiles.lan"));
}
