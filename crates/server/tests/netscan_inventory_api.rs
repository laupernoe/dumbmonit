//! Scans de découverte planifiés et inventaire du réseau : diff d'inventaire
//! (premier balayage silencieux, nouvel appareil, nouveau port, changement d'IP)
//! et routes `/api/discovery/schedules` et `/api/discovery/inventory`.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use dumbmonit_proto::{Confidence, DiscoveredHost, DiscoveredService, Transport};
use dumbmonit_server::db;
use dumbmonit_server::netinventory::store::{self, Unknown};
use serde_json::{Value, json};
use sqlx::SqlitePool;

/// Instance configurée, avec un accès direct à la base.
async fn setup() -> (TestApp, SqlitePool) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("ouverture de la base");
    let app = common::build(dir, config, pool.clone()).await;
    app.create_admin().await;
    (app, pool)
}

fn service(port: u16) -> DiscoveredService {
    DiscoveredService {
        port,
        protocol: Transport::Tcp,
        product: "svc".to_string(),
        kind: None,
        version: None,
        confidence: Confidence::High,
        tls: false,
        detail: None,
        monitored_target_id: None,
        prefill: None,
    }
}

fn host(ip: &str, mac: Option<&str>, ports: &[u16]) -> DiscoveredHost {
    DiscoveredHost {
        ip: ip.to_string(),
        hostname: None,
        mac: mac.map(str::to_string),
        vendor: None,
        services: ports.iter().map(|p| service(*p)).collect(),
    }
}

async fn schedule(app: &TestApp, admin: &str, cidr: &str) -> i64 {
    let reply = app.post("/api/discovery/schedules", json!({ "cidr": cidr }), Some(admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    reply.body["id"].as_i64().expect("id")
}

fn find<'a>(devices: &'a [Value], ip: &str) -> &'a Value {
    devices.iter().find(|d| d["ip"] == ip).expect("appareil listé")
}

fn kinds(items: &[Unknown]) -> (usize, usize) {
    let devices = items.iter().filter(|i| matches!(i, Unknown::Device { .. })).count();
    let changes = items.iter().filter(|i| matches!(i, Unknown::MacChange { .. })).count();
    (devices, items.len() - devices - changes)
}

fn mac_changes(items: &[Unknown]) -> Vec<&Unknown> {
    items.iter().filter(|i| matches!(i, Unknown::MacChange { .. })).collect()
}

#[tokio::test]
async fn le_premier_balayage_initialise_l_inventaire_sans_rien_signaler() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;

    let hosts =
        [host("10.0.0.2", Some("aa:aa:aa:aa:aa:aa"), &[22, 80]), host("10.0.0.3", None, &[])];
    let applied = store::apply_scan(&pool, id, &hosts).await.unwrap();
    assert!(applied.initialized);
    assert_eq!((applied.new_devices, applied.new_ports), (0, 0));
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());

    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let devices = inventory.body["devices"].as_array().expect("devices");
    assert_eq!(devices.len(), 2);
    assert!(devices.iter().all(|d| d["known"] == true));
    let with_ports = devices.iter().find(|d| d["ip"] == "10.0.0.2").unwrap();
    assert_eq!(with_ports["ports"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn un_balayage_vide_n_initialise_rien() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;

    let applied = store::apply_scan(&pool, id, &[]).await.unwrap();
    assert!(!applied.initialized);
    // Le balayage suivant est donc encore un premier balayage : silencieux.
    let applied = store::apply_scan(&pool, id, &[host("10.0.0.2", None, &[22])]).await.unwrap();
    assert!(applied.initialized);
    assert_eq!(applied.new_devices, 0);
}

#[tokio::test]
async fn un_nouvel_appareil_et_un_nouveau_port_sont_signales_une_seule_fois() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    let mac = "aa:aa:aa:aa:aa:aa";
    store::apply_scan(&pool, id, &[host("10.0.0.2", Some(mac), &[22])]).await.unwrap();

    let second = [
        host("10.0.0.2", Some(mac), &[22, 3389]),
        host("10.0.0.9", Some("bb:bb:bb:bb:bb:bb"), &[80]),
    ];
    let applied = store::apply_scan(&pool, id, &second).await.unwrap();
    assert_eq!((applied.new_devices, applied.new_ports), (1, 1));
    let items = store::unknown_items(&pool).await.unwrap();
    // Le port du nouvel appareil n'ajoute pas d'alerte : celle de l'appareil le couvre.
    assert_eq!(kinds(&items), (1, 1));

    // Même résultat au balayage suivant : rien de plus, rien de moins.
    let applied = store::apply_scan(&pool, id, &second).await.unwrap();
    assert_eq!((applied.new_devices, applied.new_ports), (0, 0));
    assert_eq!(kinds(&store::unknown_items(&pool).await.unwrap()), (1, 1));

    let counts = app.get("/api/discovery/schedules", Some(&admin)).await;
    let row = &counts.body["schedules"][0];
    assert_eq!(row["devices"], 2);
    assert_eq!(row["unknown_devices"], 1);
    assert_eq!(row["unknown_ports"], 1);
}

#[tokio::test]
async fn un_appareil_qui_change_d_ip_garde_son_identite() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    let mac = "aa:aa:aa:aa:aa:aa";
    store::apply_scan(&pool, id, &[host("10.0.0.2", Some(mac), &[22])]).await.unwrap();

    let applied =
        store::apply_scan(&pool, id, &[host("10.0.0.77", Some("AA-AA-AA-AA-AA-AA"), &[22])])
            .await
            .unwrap();
    assert_eq!((applied.new_devices, applied.new_ports), (0, 0));
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());

    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let devices = inventory.body["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0]["ip"], "10.0.0.77");
}

#[tokio::test]
async fn marquer_connu_arrete_le_signalement() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    let mac = "aa:aa:aa:aa:aa:aa";
    store::apply_scan(&pool, id, &[host("10.0.0.2", Some(mac), &[22])]).await.unwrap();
    let second = [
        host("10.0.0.2", Some(mac), &[22, 3389]),
        host("10.0.0.9", Some("bb:bb:bb:bb:bb:bb"), &[80]),
        host("10.0.0.10", Some("cc:cc:cc:cc:cc:cc"), &[]),
    ];
    store::apply_scan(&pool, id, &second).await.unwrap();
    assert_eq!(kinds(&store::unknown_items(&pool).await.unwrap()), (2, 1));

    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let devices = inventory.body["devices"].as_array().unwrap();
    // Les inconnus d'abord.
    assert_eq!(devices[0]["known"], false);

    let port = find(devices, "10.0.0.2")["ports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["port"] == 3389)
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let reply = app
        .post(&format!("/api/discovery/inventory/ports/{port}/known"), json!({}), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    assert_eq!(kinds(&store::unknown_items(&pool).await.unwrap()), (2, 0));

    let device = find(devices, "10.0.0.9")["id"].as_i64().unwrap();
    let reply = app
        .post(&format!("/api/discovery/inventory/devices/{device}/known"), json!({}), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    // Les ports de l'appareil sont connus avec lui, sans alerte de port à l'ouverture.
    assert_eq!(kinds(&store::unknown_items(&pool).await.unwrap()), (1, 0));

    let reply = app.post("/api/discovery/inventory/known", json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["marked"], 1);
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());

    for uri in
        ["/api/discovery/inventory/devices/9999/known", "/api/discovery/inventory/ports/9999/known"]
    {
        let reply = app.post(uri, json!({}), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn les_scans_planifies_se_gerent_par_l_api() {
    let (app, _pool) = setup().await;
    let admin = app.admin_cookie().await;

    let created = app
        .post(
            "/api/discovery/schedules",
            json!({ "name": "LAN", "cidr": "192.168.1.77/24", "interval_secs": 1800, "depth": "extended" }),
            Some(&admin),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let id = created.body["id"].as_i64().unwrap();
    // Le sous-réseau est ramené à son adresse de réseau.
    assert_eq!(created.body["cidr"], "192.168.1.0/24");
    assert_eq!(created.body["depth"], "extended");
    assert_eq!(created.body["enabled"], true);
    assert_eq!(created.body["initialized"], false);
    assert_eq!(created.body["agent_id"], Value::Null);

    // Même sous-réseau, même exécutant : refusé.
    let twice = app
        .post("/api/discovery/schedules", json!({ "cidr": "192.168.1.0/24" }), Some(&admin))
        .await;
    assert_eq!(twice.status, StatusCode::CONFLICT, "{}", twice.body);

    let list = app.get("/api/discovery/schedules", Some(&admin)).await;
    assert_eq!(list.body["schedules"].as_array().unwrap().len(), 1);

    let updated = app
        .put(
            &format!("/api/discovery/schedules/{id}"),
            json!({ "name": "LAN", "cidr": "192.168.1.0/24", "interval_secs": 600, "enabled": false }),
            Some(&admin),
        )
        .await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
    assert_eq!(updated.body["interval_secs"], 600);
    assert_eq!(updated.body["enabled"], false);
    assert_eq!(updated.body["depth"], "standard");

    let run =
        app.post(&format!("/api/discovery/schedules/{id}/run"), json!({}), Some(&admin)).await;
    assert_eq!(run.status, StatusCode::ACCEPTED, "{}", run.body);

    let gone = app.delete(&format!("/api/discovery/schedules/{id}"), Some(&admin)).await;
    assert_eq!(gone.status, StatusCode::NO_CONTENT);
    let again = app.delete(&format!("/api/discovery/schedules/{id}"), Some(&admin)).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    let put = app
        .put(
            &format!("/api/discovery/schedules/{id}"),
            json!({ "cidr": "10.0.0.0/24" }),
            Some(&admin),
        )
        .await;
    assert_eq!(put.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn les_saisies_invalides_sont_refusees() {
    let (app, _pool) = setup().await;
    let admin = app.admin_cookie().await;
    let bad = [
        json!({ "cidr": "pas-un-reseau" }),
        // Plus de 1024 adresses.
        json!({ "cidr": "10.0.0.0/16" }),
        json!({ "cidr": "10.0.0.0/24", "interval_secs": 10 }),
        json!({ "cidr": "10.0.0.0/24", "interval_secs": 999999999 }),
        json!({ "cidr": "10.0.0.0/24", "name": "x".repeat(200) }),
    ];
    for body in bad {
        let reply = app.post("/api/discovery/schedules", body.clone(), Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{body}: {}", reply.body);
    }
    let unknown_agent = app
        .post(
            "/api/discovery/schedules",
            json!({ "cidr": "10.0.0.0/24", "agent_id": 424242 }),
            Some(&admin),
        )
        .await;
    assert_eq!(unknown_agent.status, StatusCode::NOT_FOUND, "{}", unknown_agent.body);
}

#[tokio::test]
async fn l_inventaire_est_reserve_aux_administrateurs() {
    let (app, _pool) = setup().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    for uri in ["/api/discovery/schedules", "/api/discovery/inventory"] {
        let reply = app.get(uri, Some(&viewer)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{uri}");
        let anonymous = app.get(uri, None).await;
        assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let reply =
        app.post("/api/discovery/schedules", json!({ "cidr": "10.0.0.0/24" }), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn les_regles_d_alerte_livrees_sont_eteintes_et_en_information() {
    let (_app, pool) = setup().await;
    db::alerts::seed_builtin_rules(&pool).await.expect("règles livrées");
    for uid in ["netscan_new_device", "netscan_new_port", "netscan_mac_change"] {
        let (enabled, severity): (i64, String) =
            sqlx::query_as("SELECT enabled, severity FROM alert_rules WHERE uid = ?")
                .bind(uid)
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("règle {uid} absente"));
        assert_eq!(enabled, 0, "{uid} doit être désactivée par défaut");
        let expected = if uid == "netscan_mac_change" { "warning" } else { "info" };
        assert_eq!(severity, expected, "{uid}");
    }
}

#[tokio::test]
async fn une_autre_mac_sur_une_ip_connue_leve_un_changement_de_mac() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    let old = "aa:aa:aa:aa:aa:aa";
    store::apply_scan(&pool, id, &[host("10.0.0.2", Some(old), &[22])]).await.unwrap();

    let applied = store::apply_scan(
        &pool,
        id,
        &[
            host("10.0.0.2", Some("BB:BB:BB:BB:BB:BB"), &[]),
            host("10.0.0.3", Some("cc:cc:cc:cc:cc:cc"), &[]),
        ],
    )
    .await
    .unwrap();
    assert_eq!((applied.new_devices, applied.mac_changes), (2, 1));
    let items = store::unknown_items(&pool).await.unwrap();
    // Deux appareils nouveaux, dont un à la MAC changée : un seul changement.
    assert_eq!(kinds(&items), (2, 0));
    match mac_changes(&items).as_slice() {
        [Unknown::MacChange { ident, previous_mac, .. }] => {
            assert_eq!(ident, "mac:bb:bb:bb:bb:bb:bb");
            assert_eq!(previous_mac, old);
        }
        other => panic!("un changement attendu : {other:?}"),
    }

    let list = app.get("/api/discovery/schedules", Some(&admin)).await;
    assert_eq!(list.body["schedules"][0]["mac_changes"], 1);
    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let devices = inventory.body["devices"].as_array().unwrap();
    let spoof = devices.iter().find(|d| d["mac"] == "bb:bb:bb:bb:bb:bb").expect("appareil");
    assert_eq!(spoof["mac_change_from"], old);
    assert!(find(devices, "10.0.0.3")["mac_change_from"].is_null());

    // Le même balayage ne le relève pas une seconde fois.
    let again = store::apply_scan(&pool, id, &[host("10.0.0.2", Some("bb:bb:bb:bb:bb:bb"), &[])])
        .await
        .unwrap();
    assert_eq!(again.mac_changes, 0);

    // Marqué connu : le changement est acquitté.
    let device = spoof["id"].as_i64().unwrap();
    let reply = app
        .post(&format!("/api/discovery/inventory/devices/{device}/known"), json!({}), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    assert!(mac_changes(&store::unknown_items(&pool).await.unwrap()).is_empty());
}

#[tokio::test]
async fn le_premier_balayage_ne_signale_aucun_changement_de_mac() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    // Deux MAC pour une IP dans le tout premier balayage : inventaire de départ.
    let applied = store::apply_scan(
        &pool,
        id,
        &[
            host("10.0.0.2", Some("aa:aa:aa:aa:aa:aa"), &[]),
            host("10.0.0.2", Some("bb:bb:bb:bb:bb:bb"), &[]),
        ],
    )
    .await
    .unwrap();
    assert_eq!(applied.mac_changes, 0);
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn ignorer_un_appareil_coupe_toutes_ses_alertes_et_connu_le_reprend() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    store::apply_scan(&pool, id, &[host("10.0.0.2", Some("aa:aa:aa:aa:aa:aa"), &[22])])
        .await
        .unwrap();
    store::apply_scan(
        &pool,
        id,
        &[host("10.0.0.2", Some("bb:bb:bb:bb:bb:bb"), &[22]), host("10.0.0.2", None, &[])],
    )
    .await
    .unwrap();
    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let guest = inventory.body["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["mac"] == "bb:bb:bb:bb:bb:bb")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    assert!(!store::unknown_items(&pool).await.unwrap().is_empty());

    let uri = format!("/api/discovery/inventory/devices/{guest}/ignore");
    let reply = app.post(&uri, json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());

    // Un port qui apparait sur l'appareil ignoré ne déclenche rien non plus.
    store::apply_scan(&pool, id, &[host("10.0.0.2", Some("bb:bb:bb:bb:bb:bb"), &[22, 3389])])
        .await
        .unwrap();
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());
    let list = app.get("/api/discovery/schedules", Some(&admin)).await;
    let row = &list.body["schedules"][0];
    assert_eq!(row["ignored_devices"], 1);
    assert_eq!(row["unknown_ports"], 0);

    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let device =
        inventory.body["devices"].as_array().unwrap().iter().find(|d| d["id"] == guest).unwrap();
    assert_eq!((device["known"].as_bool(), device["ignored"].as_bool()), (Some(true), Some(true)));

    // « Connu » le sort de l'état ignoré : ses ports nouveaux comptent de nouveau.
    let reply = app
        .post(&format!("/api/discovery/inventory/devices/{guest}/known"), json!({}), Some(&admin))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
    store::apply_scan(&pool, id, &[host("10.0.0.2", Some("bb:bb:bb:bb:bb:bb"), &[22, 3389, 23])])
        .await
        .unwrap();
    assert_eq!(kinds(&store::unknown_items(&pool).await.unwrap()), (0, 1));

    let missing = "/api/discovery/inventory/devices/9999/ignore";
    let reply = app.post(missing, json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let viewer = app.viewer_cookie(&admin).await;
    let reply = app.post(&uri, json!({}), Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}

/// Hôte avec un service dont la détection propose un type et un pré-remplissage.
fn detected_host(ip: &str, mac: &str, kind: Option<&str>, address: &str) -> DiscoveredHost {
    let mut svc = service(8080);
    svc.kind = kind.map(str::to_string);
    svc.prefill = kind.map(|_| json!({ "name": format!("Device {ip}"), "address": address }));
    DiscoveredHost {
        ip: ip.to_string(),
        hostname: None,
        mac: Some(mac.to_string()),
        vendor: None,
        services: vec![svc],
    }
}

fn device_id(devices: &[Value], ip: &str) -> i64 {
    find(devices, ip)["id"].as_i64().expect("id")
}

#[tokio::test]
async fn l_ajout_en_bloc_rend_un_resultat_par_appareil_meme_mixte() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    // Inventaire de départ, puis trois appareils nouveaux.
    let base = host("10.0.0.2", Some("aa:aa:aa:aa:aa:aa"), &[22]);
    store::apply_scan(&pool, id, std::slice::from_ref(&base)).await.unwrap();
    store::apply_scan(
        &pool,
        id,
        &[
            base,
            detected_host("10.0.0.10", "00:00:00:00:00:10", Some("http"), "http://10.0.0.10:8080"),
            detected_host("10.0.0.11", "00:00:00:00:00:11", Some("http"), "http://10.0.0.11:8080"),
            // Rien de reconnu : une erreur claire, sans bloquer les autres.
            detected_host("10.0.0.12", "00:00:00:00:00:12", None, ""),
        ],
    )
    .await
    .unwrap();
    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let devices = inventory.body["devices"].as_array().unwrap().clone();
    let (a, b, none) = (
        device_id(&devices, "10.0.0.10"),
        device_id(&devices, "10.0.0.11"),
        device_id(&devices, "10.0.0.12"),
    );

    // Le deuxième est déjà surveillé avant l'appel.
    let existing = json!({ "name": "Déjà là", "address": "http://10.0.0.11:8080", "kind": "http" });
    let reply = app.post("/api/targets", existing, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);

    let review = app.get("/api/discovery/review", Some(&admin)).await;
    assert_eq!(review.body["to_review"], 2, "{}", review.body);

    let body = json!({ "action": "add", "device_ids": [a, b, none, 9999, a] });
    let reply = app.post("/api/discovery/inventory/bulk", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let results = reply.body["results"].as_array().unwrap();
    // Le doublon `a` n'est traité qu'une fois.
    assert_eq!(results.len(), 4, "{}", reply.body);
    let outcome =
        |device: i64| results.iter().find(|r| r["device_id"] == device).unwrap()["outcome"].clone();
    assert_eq!(outcome(a), "added");
    assert_eq!(outcome(b), "already_present");
    assert_eq!(outcome(none), "error");
    assert_eq!(outcome(9999), "error");
    assert_eq!(reply.body["added"], 1);
    assert_eq!(reply.body["already_present"], 1);
    assert_eq!(reply.body["errors"], 2);

    // Le moniteur existe, et l'appareil ajouté a quitté la file.
    let targets = app.get("/api/targets", Some(&admin)).await;
    let created =
        targets.body.as_array().unwrap().iter().any(|t| t["address"] == "http://10.0.0.10:8080");
    assert!(created, "{}", targets.body);
    let review = app.get("/api/discovery/review", Some(&admin)).await;
    assert_eq!(review.body["to_review"], 1, "{}", review.body);

    // Rejouer l'appel ne crée pas de doublon.
    let body = json!({ "action": "add", "device_ids": [a] });
    let again = app.post("/api/discovery/inventory/bulk", body, Some(&admin)).await;
    assert_eq!(again.body["results"][0]["outcome"], "already_present", "{}", again.body);
}

#[tokio::test]
async fn ignorer_en_bloc_sort_de_la_file_et_se_defait() {
    let (app, pool) = setup().await;
    let admin = app.admin_cookie().await;
    let id = schedule(&app, &admin, "10.0.0.0/24").await;
    let base = host("10.0.0.2", Some("aa:aa:aa:aa:aa:aa"), &[22]);
    store::apply_scan(&pool, id, std::slice::from_ref(&base)).await.unwrap();
    let next = [
        base,
        host("10.0.0.20", Some("00:00:00:00:00:20"), &[80]),
        host("10.0.0.21", Some("00:00:00:00:00:21"), &[80]),
    ];
    let applied = store::apply_scan(&pool, id, &next).await.unwrap();
    assert_eq!(applied.new_devices, 2);
    store::open_batch(&pool, applied.new_devices).await.unwrap();

    // Un seul avis pour tout le lot, et il ne revient pas une fois vu.
    let review = app.get("/api/discovery/review", Some(&admin)).await;
    assert_eq!(review.body["to_review"], 2);
    assert_eq!(review.body["announce"], true);
    assert_eq!(review.body["batch_count"], 2);
    let reply = app.post("/api/discovery/review/seen", json!({}), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let review = app.get("/api/discovery/review", Some(&admin)).await;
    assert_eq!(review.body["announce"], false);

    let inventory = app.get("/api/discovery/inventory", Some(&admin)).await;
    let devices = inventory.body["devices"].as_array().unwrap().clone();
    let ids = [device_id(&devices, "10.0.0.20"), device_id(&devices, "10.0.0.21")];

    let body = json!({ "action": "ignore", "device_ids": ids });
    let reply = app.post("/api/discovery/inventory/bulk", body, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body["results"].as_array().unwrap().iter().all(|r| r["outcome"] == "ignored"));
    let review = app.get("/api/discovery/review", Some(&admin)).await;
    assert_eq!(review.body["to_review"], 0);
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());

    // Défaire : les deux reviennent dans la file.
    let body = json!({ "action": "restore", "device_ids": ids });
    let reply = app.post("/api/discovery/inventory/bulk", body, Some(&admin)).await;
    assert!(reply.body["results"].as_array().unwrap().iter().all(|r| r["outcome"] == "restored"));
    let review = app.get("/api/discovery/review", Some(&admin)).await;
    assert_eq!(review.body["to_review"], 2);

    // Un passage sans nouveauté n'ouvre aucun lot.
    store::mark_batch_seen(&pool).await.unwrap();
    store::open_batch(&pool, 0).await.unwrap();
    assert!(!store::review_batch(&pool).await.unwrap().unseen);
}

#[tokio::test]
async fn l_ajout_en_bloc_est_plafonne_et_reserve_aux_administrateurs() {
    let (app, _pool) = setup().await;
    let admin = app.admin_cookie().await;
    let uri = "/api/discovery/inventory/bulk";
    let too_many: Vec<i64> = (1..=201).collect();
    let reply =
        app.post(uri, json!({ "action": "ignore", "device_ids": too_many }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    let reply = app.post(uri, json!({ "action": "ignore", "device_ids": [] }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    let viewer = app.viewer_cookie(&admin).await;
    let body = json!({ "action": "ignore", "device_ids": [1] });
    let reply = app.post(uri, body, Some(&viewer)).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}
