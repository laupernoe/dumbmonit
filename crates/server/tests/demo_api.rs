//! Mode démonstration : la connexion `demo` / `demo` fonctionne, les lectures
//! aussi, et toute écriture est refusée par le garde unique — quel que soit
//! l'appelant, session ouverte ou non.

mod common;

use axum::http::StatusCode;
use dumbmonit_server::{db, demo, notify, tsdb};
use serde_json::{Value, json};

use common::{TestApp, UNREACHABLE_VICTORIA};

/// Instance en mode démonstration, parc synthétique semé (sans les équipements
/// simulés, qui ont leur propre test : `demo_estate.rs`).
async fn demo_app() -> TestApp {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = common::base_config(dir.path());
    config.demo = true;
    let pool = db::open(&config.database_path()).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    let victoria = tsdb::Victoria::new(UNREACHABLE_VICTORIA).expect("client");
    let sink = tsdb::spawn_writer(victoria, std::time::Duration::from_secs(60));
    demo::seed::run(&pool, &cipher, &sink, &[]).await.expect("parc de démonstration");
    common::build(dir, config, pool).await
}

async fn demo_cookie(app: &TestApp) -> String {
    let reply = app.login_as(demo::DEMO_USERNAME, demo::DEMO_PASSWORD).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{:?}", reply.body);
    reply.cookie()
}

fn assert_refused(reply: &common::Reply, what: &str) {
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{what} : {:?}", reply.body);
    assert_eq!(reply.body["demo"], Value::Bool(true), "{what}");
    assert_eq!(reply.body["error"], demo::READ_ONLY_MESSAGE, "{what}");
}

#[tokio::test]
async fn le_statut_annonce_la_demonstration_sans_ouvrir_de_session() {
    let app = demo_app().await;
    let status = app.get("/api/auth/status", None).await;
    assert_eq!(status.body["demo"], true);
    assert_eq!(status.body["authenticated"], false, "l'écran de connexion reste affiché");
    let health = app.get("/api/health", None).await;
    assert_eq!(health.body["demo"], true);
}

#[tokio::test]
async fn le_compte_demo_se_connecte_et_lit_le_parc() {
    let app = demo_app().await;
    let cookie = demo_cookie(&app).await;

    let me = app.get("/api/auth/me", Some(&cookie)).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["username"], "demo");

    let targets = app.get("/api/targets", Some(&cookie)).await;
    assert_eq!(targets.status, StatusCode::OK);
    let names: Vec<&str> =
        targets.body.as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"nextcloud.home.arpa"), "{names:?}");
    assert!(names.contains(&"docker01.home.arpa"), "l'agent simulé s'est enregistré : {names:?}");
    // Tout est fictif : des noms en `.home.arpa`, les domaines surveillés, la
    // zone Cloudflare et les domaines publics des deux clients.
    assert!(
        names.iter().all(|name| {
            name.contains("home.arpa")
                || name.contains("dupont.ch")
                || name.contains("muller-archi.ch")
                || *name == demo::extras::CLOUDFLARE_HOST
                || demo::extras::DOMAINS.iter().any(|(domain, _, _)| domain == name)
        }),
        "{names:?}"
    );
    for host in demo::synthetic::AGENT_HOSTS {
        assert!(names.contains(&host.hostname), "agent {} : {names:?}", host.hostname);
    }

    for uri in
        ["/api/alerts/rules", "/api/alerts/history", "/api/notify/channels", "/api/status-pages"]
    {
        let reply = app.get(uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri} : {:?}", reply.body);
    }
    let public = app.get("/api/public/status/home", None).await;
    assert_eq!(public.status, StatusCode::OK, "la page de statut publique est servie");

    // Aucun secret ne revient : les canaux semés n'en ont pas, et l'API ne
    // renverrait de toute façon que `has_secret`.
    let channels = app.get("/api/notify/channels", Some(&cookie)).await;
    assert!(!channels.body.to_string().contains("secret_enc"));
}

#[tokio::test]
async fn le_semis_montre_les_nouveautes() {
    let app = demo_app().await;
    let cookie = demo_cookie(&app).await;

    // Les types simulés en mémoire : commutateur, menaces, échéances, licence.
    let targets = app.get("/api/targets", Some(&cookie)).await;
    let kinds: Vec<&str> =
        targets.body.as_array().unwrap().iter().filter_map(|t| t["kind"].as_str()).collect();
    for kind in demo::extras::KINDS {
        assert!(kinds.contains(kind), "{kind} : {kinds:?}");
    }
    let domains = kinds.iter().filter(|kind| **kind == "domain").count();
    assert_eq!(domains, demo::extras::DOMAINS.len());

    // Deux pages de statut, chacune sous sa scène de nuit.
    let pages = app.get("/api/status-pages", Some(&cookie)).await;
    let scenes: Vec<(String, String)> = pages
        .body
        .as_array()
        .unwrap()
        .iter()
        .map(|page| {
            let scenes = page["scenes"].as_array().unwrap();
            (page["slug"].as_str().unwrap().to_string(), scenes[0].as_str().unwrap().to_string())
        })
        .collect();
    let expected = [("home", "paris"), ("lab", "tokyo"), ("photos", "newyork"), ("muller", "dubai")];
    assert_eq!(
        scenes,
        expected.map(|(slug, scene)| (slug.to_string(), scene.to_string())),
        "les quatre scènes de nuit"
    );
    let lab = app.get("/api/public/status/lab", None).await;
    assert_eq!(lab.status, StatusCode::OK, "{:?}", lab.body);

    // Le réseau : un balayage terminé, un appareil nouveau, un port inattendu.
    let schedules = app.get("/api/discovery/schedules", Some(&cookie)).await;
    let schedule = &schedules.body["schedules"][0];
    assert_eq!(schedule["last_status"], "done", "{schedule}");
    assert_eq!(schedule["initialized"], true);
    assert_eq!(schedule["unknown_devices"], 1, "{schedule}");
    assert_eq!(schedule["unknown_ports"], 1, "{schedule}");
    assert!(schedule["devices"].as_i64().unwrap() >= 8, "{schedule}");
}

/// Longueur d'un tableau JSON, à la racine ou sous `key`.
fn count(body: &Value, key: Option<&str>) -> usize {
    let list = match key {
        Some(key) => &body[key],
        None => body,
    };
    list.as_array().map_or(0, Vec::len)
}

#[tokio::test]
async fn chaque_page_cle_a_des_donnees() {
    let app = demo_app().await;
    let cookie = demo_cookie(&app).await;
    // (route, clé du tableau, minimum attendu).
    let pages: &[(&str, Option<&str>, usize)] = &[
        ("/api/targets", None, 25),
        ("/api/alerts/rules", None, 10),
        ("/api/alerts/history", None, 2),
        ("/api/alerts/silences", None, 2),
        ("/api/notify/channels", None, 7),
        ("/api/status-pages", None, 4),
        ("/api/incidents", None, 3),
        ("/api/oncall/schedules", None, 2),
        ("/api/oncall/policies", None, 2),
        ("/api/oncall/people", None, 4),
        ("/api/runbooks", None, 4),
        ("/api/dashboards", None, 2),
        ("/api/reports/schedules", None, 2),
        ("/api/teams", Some("teams"), 1),
        ("/api/clients", Some("clients"), 2),
        ("/api/clients/overview", None, 2),
        ("/api/locations", Some("locations"), 3),
        ("/api/relays", None, 3),
        ("/api/security/cves", Some("findings"), 4),
        ("/api/discovery/inventory", Some("devices"), 8),
        ("/api/agent/tokens", None, 3),
    ];
    for (uri, key, minimum) in pages {
        let reply = app.get(uri, Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri} : {:?}", reply.body);
        let found = count(&reply.body, *key);
        assert!(found >= *minimum, "{uri} : {found} < {minimum} : {}", reply.body);
    }

    // Le mode MSP est allumé, avec le bureau de Genève comme second client.
    let clients = app.get("/api/clients", Some(&cookie)).await;
    assert_eq!(clients.body["settings"]["enabled"], true, "{}", clients.body);
    assert!(clients.body.to_string().contains(demo::workspace::OFFICE_CLIENT));

    // Le relais du bureau sert les sondes du site, qui passent par lui.
    let relays = app.get("/api/relays", Some(&cookie)).await;
    let relay = relays
        .body
        .as_array()
        .and_then(|list| list.iter().find(|r| r["relay"] == true))
        .cloned()
        .unwrap_or_default();
    assert_eq!(relay["name"], demo::synthetic::RELAY_HOST, "{}", relays.body);
    assert_eq!(relay["site"], demo::synthetic::OFFICE_SITE);
    let relayed = relay["relayed"].as_u64().unwrap_or(0) as usize;
    assert!(relayed >= demo::workspace::OFFICE_CHECKS.len(), "{relay}");

    // Les failles : activées, une exploitée, rien de téléchargé.
    let cves = app.get("/api/security/cves", Some(&cookie)).await;
    assert_eq!(cves.body["enabled"], true, "{}", cves.body);
    assert!(cves.body.to_string().contains("CVE-2025-32463"), "{}", cves.body);

    // Les tableaux de bord semés passent la validation de l'API.
    let dashboards = app.get("/api/dashboards", Some(&cookie)).await;
    for dashboard in dashboards.body.as_array().unwrap() {
        assert!(count(&dashboard["widgets"], None) >= 5, "{dashboard}");
    }

    // La page publique du bureau a son incident en cours.
    let office = app.get("/api/public/status/muller", None).await;
    assert_eq!(office.status, StatusCode::OK, "{:?}", office.body);
    assert!(office.body.to_string().contains("Delays on incoming email"), "{}", office.body);
}

#[tokio::test]
async fn toute_ecriture_est_refusee_avec_ou_sans_session() {
    let app = demo_app().await;
    let cookie = demo_cookie(&app).await;
    let target = json!({"name": "x", "address": "169.254.169.254", "kind": "http"});
    let writes: &[(&str, &str, Option<Value>)] = &[
        ("POST", "/api/targets", Some(target.clone())),
        ("PUT", "/api/targets/1", Some(target)),
        ("DELETE", "/api/targets/1", None),
        ("POST", "/api/targets/1/probe", Some(json!({}))),
        ("POST", "/api/discovery", Some(json!({"cidr": "10.0.0.0/24"}))),
        (
            "POST",
            "/api/auth/password",
            Some(json!({"current_password": "demo", "new_password": "x"})),
        ),
        ("POST", "/api/auth/totp/enroll", Some(json!({}))),
        ("POST", "/api/users", Some(json!({"username": "eve", "password": "p", "role": "admin"}))),
        ("DELETE", "/api/users/1", None),
        ("POST", "/api/tokens", Some(json!({"name": "t", "scope": "write"}))),
        ("POST", "/api/agent/tokens", Some(json!({"name": "t"}))),
        ("POST", "/api/notify/channels", Some(json!({"name": "c", "kind": "ntfy"}))),
        ("POST", "/api/notify/channels/1/test", Some(json!({}))),
        ("PUT", "/api/notify/policy", Some(json!({}))),
        ("POST", "/api/alerts/rules", Some(json!({}))),
        ("POST", "/api/alerts/rules/1/enable", Some(json!({"enabled": false}))),
        ("POST", "/api/alerts/silences", Some(json!({}))),
        ("POST", "/api/alerts/abc/ack", Some(json!({}))),
        ("PUT", "/api/onboarding", Some(json!({}))),
        ("POST", "/api/backup", Some(json!({}))),
        ("POST", "/api/backup/restore", Some(json!({}))),
        ("POST", "/api/status-pages", Some(json!({}))),
        ("POST", "/api/public/status/home/subscribe", Some(json!({"email": "a@b.c"}))),
        ("POST", "/api/mcp", Some(json!({"jsonrpc": "2.0", "method": "tools/list", "id": 1}))),
        ("POST", "/api/ingest", Some(json!({}))),
        ("PATCH", "/api/targets/1", Some(json!({}))),
    ];
    for (method, uri, body) in writes {
        for session in [Some(cookie.as_str()), None] {
            let reply = app.request(method, uri, body.clone(), session).await;
            assert_refused(&reply, &format!("{method} {uri}"));
        }
    }
    for uri in ["/api/auth/audit", "/api/auth/oidc/start", "/api/push/abc", "/api/agent/commands"] {
        assert_refused(&app.get(uri, Some(&cookie)).await, uri);
    }

    // Rien n'a bougé : pas de cible en plus, pas de compte en plus.
    let targets = app.get("/api/targets", Some(&cookie)).await;
    assert!(!targets.body.to_string().contains("169.254.169.254"));
    let users = app.get("/api/users", Some(&cookie)).await;
    let accounts = users.body.as_array().map(Vec::len).unwrap_or(0);
    assert_eq!(accounts, 4, "le visiteur et les trois comptes du semis : {:?}", users.body);
    assert!(!users.body.to_string().contains("\"eve\""));

    // Se déconnecter reste possible.
    let logout = app.post("/api/auth/logout", json!({}), Some(&cookie)).await;
    assert!(logout.status.is_success(), "{:?}", logout.body);
}

#[tokio::test]
async fn un_visiteur_ne_peut_pas_verrouiller_le_compte_partage() {
    let app = demo_app().await;
    for _ in 0..12 {
        let reply = app.login_as(demo::DEMO_USERNAME, "mauvais").await;
        assert_ne!(reply.status, StatusCode::NO_CONTENT);
    }
    // Sans adresse cliente (requêtes de test), aucun seau ne subsiste : le
    // compte `demo` n'est jamais bloqué par les échecs des autres.
    demo_cookie(&app).await;
}

#[tokio::test]
async fn aucune_notification_ne_part() {
    notify::disable_sending();
    let channel = notify::channel::ChannelConfig {
        id: 1,
        name: "webhook".into(),
        kind: "webhook".into(),
        enabled: true,
        // Adresse injoignable : si l'envoi partait, l'erreur serait une erreur réseau.
        settings: json!({"url": "http://127.0.0.1:1/hook"}),
        secrets: json!({}),
    };
    let http = reqwest::Client::new();
    let message = notify::message::test_message("webhook");
    let report = notify::deliver(&http, &channel, &message).await;
    assert_eq!(report.error.as_deref(), Some(notify::DISABLED_MESSAGE));
}
