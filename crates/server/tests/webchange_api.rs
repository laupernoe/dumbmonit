//! Surveillance des changements d'un site, de bout en bout : un faux site
//! servi en local, une référence, des pages modifiées, ajoutées, disparues, et
//! ce que l'API en montre (pages, changements, diff, captures).

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use dumbmonit_server::webchange::{self, WebchangeCollector};
use dumbmonit_server::{collectors, db};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

use common::TestApp;

/// Le contenu du faux site : chemin → (statut, type, corps).
type Pages = Arc<Mutex<HashMap<String, (u16, &'static str, String)>>>;

async fn serve(State(pages): State<Pages>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    let found = pages.lock().unwrap().get(&path).cloned();
    match found {
        Some((status, content_type, body)) => {
            (StatusCode::from_u16(status).unwrap(), [(header::CONTENT_TYPE, content_type)], body)
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "absent").into_response(),
    }
}

async fn fake_site() -> (String, Pages) {
    let pages: Pages = Arc::new(Mutex::new(HashMap::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new().fallback(serve).with_state(pages.clone());
    tokio::spawn(async move { axum::serve(listener, router).await.expect("faux site") });
    (format!("http://{address}"), pages)
}

fn set(pages: &Pages, path: &str, status: u16, body: String) {
    let content_type = if path.ends_with(".txt") { "text/plain" } else { "text/html" };
    pages.lock().unwrap().insert(path.to_string(), (status, content_type, body));
}

fn html(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html><head><title>{title}</title><script>var t = Date.now();</script>\
         </head><body>{body}</body></html>"
    )
}

fn index(links: &[&str]) -> String {
    let links: String =
        links.iter().map(|l| format!("<li><a href=\"{l}\">Lien {l}</a></li>")).collect();
    html(
        "Accueil",
        &format!(
            "<h1>Documentation</h1><ul>{links}</ul>\
             <a href=\"/ailleurs.html\">hors périmètre</a>\
             <a href=\"https://example.org/\">externe</a>\
             <a href=\"logo.png\">image</a><a href=\"#haut\">ancre</a>"
        ),
    )
}

/// Page A : dix lignes, dont une qui change en son milieu, et une ligne
/// d'horodatage que l'option `ignore` doit écarter.
fn page_a(middle: &str, stamp: u32) -> String {
    let mut body = String::new();
    for i in 1..=10 {
        if i == 5 {
            body.push_str(&format!("<p>{middle}</p>"));
        } else {
            body.push_str(&format!("<p>Ligne {i}</p>"));
        }
    }
    body.push_str(&format!("<footer>Generated at {stamp}</footer>"));
    html("Page A", &body)
}

struct Setup {
    app: TestApp,
    pool: SqlitePool,
    data_dir: std::path::PathBuf,
    cookie: String,
}

async fn setup() -> Setup {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let data_dir = config.data_dir.clone();
    let pool = db::open(&config.database_path()).await.expect("base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(WebchangeCollector::new(webchange::Context {
        pool: pool.clone(),
        data_dir: data_dir.clone(),
        browser_url: None,
    })));
    let app = common::build_with_registry(dir, config, pool.clone(), registry).await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;
    Setup { app, pool, data_dir, cookie }
}

impl Setup {
    async fn create(&self, address: &str, tags: Value) -> i64 {
        let reply = self
            .app
            .post(
                "/api/targets",
                json!({ "name": "Site", "address": address, "kind": "webchange", "tags": tags }),
                Some(&self.cookie),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "création : {}", reply.body);
        reply.body["id"].as_i64().unwrap()
    }

    /// Lance une vérification et attend qu'elle finisse.
    async fn check(&self, id: i64) {
        let reply = self
            .app
            .post(&format!("/api/targets/{id}/webchange/check"), json!({}), Some(&self.cookie))
            .await;
        assert_eq!(reply.status, StatusCode::ACCEPTED, "{}", reply.body);
        assert_eq!(reply.body["started"], true);
        for _ in 0..300 {
            let running: Option<String> =
                sqlx::query_scalar("SELECT running_since FROM webchange_state WHERE target_id = ?")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
                    .unwrap();
            if running.is_none() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("la vérification ne s'est pas terminée");
    }

    async fn get(&self, uri: &str) -> Value {
        let reply = self.app.get(uri, Some(&self.cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri} : {}", reply.body);
        reply.body
    }

    async fn raw(&self, uri: &str) -> (StatusCode, header::HeaderMap, Vec<u8>) {
        let request = axum::http::Request::get(uri)
            .header(header::COOKIE, &self.cookie)
            .body(Body::empty())
            .unwrap();
        let response = self.app.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes().to_vec();
        (status, headers, body)
    }
}

fn urls(pages: &Value) -> Vec<String> {
    let mut urls: Vec<String> = pages["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["url"].as_str().unwrap().to_string())
        .collect();
    urls.sort();
    urls
}

fn kinds(changes: &Value) -> Vec<(String, String)> {
    let mut kinds: Vec<(String, String)> = changes
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let url = c["url"].as_str().unwrap();
            let path = url.split_once("/docs/").map_or(url, |(_, p)| p).to_string();
            (path, c["kind"].as_str().unwrap().to_string())
        })
        .collect();
    kinds.sort();
    kinds
}

#[tokio::test]
async fn reference_puis_changements_pages_nouvelles_et_disparues() {
    let (base, site) = fake_site().await;
    set(&site, "/robots.txt", 200, "User-agent: *\nDisallow: /docs/prive".into());
    set(&site, "/docs/", 200, index(&["a.html", "b.html", "prive.html"]));
    set(&site, "/docs/a.html", 200, page_a("Tarif : 10 €", 1));
    set(&site, "/docs/b.html", 200, html("Page B", "<p>Bonjour</p>"));
    set(&site, "/docs/prive.html", 200, html("Privé", "<p>secret</p>"));
    set(&site, "/ailleurs.html", 200, html("Ailleurs", "<p>hors</p>"));

    let s = setup().await;
    let id = s
        .create(
            &format!("{base}/docs/"),
            json!({ "scope": "site", "allow_private_targets": "true", "ignore": "^Generated at \\d+$" }),
        )
        .await;

    // Référence : trois pages, aucune nouvelle.
    s.check(id).await;
    let pages = s.get(&format!("/api/targets/{id}/webchange/pages")).await;
    assert_eq!(pages["screenshots_available"], false);
    assert_eq!(
        urls(&pages),
        [format!("{base}/docs/"), format!("{base}/docs/a.html"), format!("{base}/docs/b.html")],
        "robots.txt, périmètre, liens externes et fichiers écartés"
    );
    for page in pages["pages"].as_array().unwrap() {
        assert_eq!(page["status"], 200);
        assert!(page["error"].is_null());
        assert!(page["last_checked"].is_string());
        assert!(page["last_changed"].is_null(), "la référence n'est pas un changement");
        assert_eq!(page["changes"], 0);
        assert!(page["latest_snapshot"].is_i64());
    }
    let titles: Vec<&str> =
        pages["pages"].as_array().unwrap().iter().filter_map(|p| p["title"].as_str()).collect();
    assert!(titles.contains(&"Page A"));
    let changes = s.get(&format!("/api/targets/{id}/webchange/changes")).await;
    assert_eq!(changes, json!([]));

    // La sonde rend les mesures de la dernière vérification.
    let probe = s.app.post(&format!("/api/targets/{id}/probe"), json!({}), Some(&s.cookie)).await;
    assert_eq!(probe.status, StatusCode::OK, "{}", probe.body);
    let series = probe.body["series"].to_string();
    assert!(series.contains("webchange_last_check_changes"), "{series}");

    // Une vérification sans changement réel : l'horodatage seul a bougé.
    set(&site, "/docs/a.html", 200, page_a("Tarif : 10 €", 2));
    s.check(id).await;
    let changes = s.get(&format!("/api/targets/{id}/webchange/changes")).await;
    assert_eq!(changes, json!([]), "la ligne exclue ne compte pas");

    // A change, B disparaît (404), C apparaît (et l'index change avec).
    set(&site, "/docs/", 200, index(&["a.html", "b.html", "c.html", "prive.html"]));
    set(&site, "/docs/a.html", 200, page_a("Tarif : 12 €", 3));
    set(&site, "/docs/b.html", 404, "absent".into());
    set(&site, "/docs/c.html", 200, html("Page C", "<p>Nouveauté</p><p>Deux</p>"));
    s.check(id).await;

    let changes = s.get(&format!("/api/targets/{id}/webchange/changes")).await;
    assert_eq!(
        kinds(&changes),
        [
            ("".to_string(), "changed".to_string()),
            ("a.html".to_string(), "changed".to_string()),
            ("b.html".to_string(), "removed_page".to_string()),
            ("c.html".to_string(), "new_page".to_string()),
        ]
    );
    let ids: Vec<i64> =
        changes.as_array().unwrap().iter().map(|c| c["id"].as_i64().unwrap()).collect();
    assert!(ids.windows(2).all(|w| w[0] > w[1]), "du plus récent au plus ancien : {ids:?}");
    let by_path = |path: &str| {
        changes
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["url"].as_str().unwrap().ends_with(path))
            .cloned()
            .unwrap()
    };
    let new_page = by_path("/c.html");
    assert_eq!(new_page["added"], 2);
    assert!(new_page["before"].is_null() && new_page["after"].is_i64());
    let removed = by_path("/b.html");
    assert_eq!(removed["removed"], 1);
    assert!(removed["before"].is_i64() && removed["after"].is_null());
    let changed = by_path("/a.html");
    assert_eq!((changed["added"].as_i64(), changed["removed"].as_i64()), (Some(1), Some(1)));
    assert!(changed["detected_at"].as_str().unwrap().len() == 19, "UTC sans suffixe");

    // Le détail : les deux instantanés, et le diff avec son contexte.
    let detail = s
        .get(&format!("/api/targets/{id}/webchange/changes/{}", changed["id"].as_i64().unwrap()))
        .await;
    assert_eq!(detail["kind"], "changed");
    assert_eq!(detail["before_snapshot"]["id"], changed["before"]);
    assert_eq!(detail["after_snapshot"]["id"], changed["after"]);
    assert_eq!(detail["after_snapshot"]["title"], "Page A");
    assert_eq!(detail["after_snapshot"]["has_screenshot"], false);
    let diff: Vec<(String, String, Value)> = detail["diff"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["op"].as_str().unwrap().into(),
                l["text"].as_str().unwrap().into(),
                l["count"].clone(),
            )
        })
        .collect();
    let line = |op: &str, text: &str| (op.to_string(), text.to_string(), Value::Null);
    assert_eq!(
        diff,
        [
            // Quatre lignes avant : une seule serait cachée, elles restent.
            line("equal", "Ligne 1"),
            line("equal", "Ligne 2"),
            line("equal", "Ligne 3"),
            line("equal", "Ligne 4"),
            line("delete", "Tarif : 10 €"),
            line("insert", "Tarif : 12 €"),
            line("equal", "Ligne 6"),
            line("equal", "Ligne 7"),
            line("equal", "Ligne 8"),
            ("skip".to_string(), String::new(), json!(2)),
        ]
    );

    // Le filtre par adresse et la limite.
    let only_a = s
        .get(&format!(
            "/api/targets/{id}/webchange/changes?url={}&limit=1",
            urlencode(&format!("{base}/docs/a.html"))
        ))
        .await;
    assert_eq!(only_a.as_array().unwrap().len(), 1);
    assert_eq!(only_a[0]["id"], changed["id"]);

    // Les pages : B disparue, A modifiée, C suivie.
    let pages = s.get(&format!("/api/targets/{id}/webchange/pages")).await;
    let page = |path: &str| {
        pages["pages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["url"].as_str().unwrap().ends_with(path))
            .cloned()
            .unwrap()
    };
    assert_eq!(page("/b.html")["status"], 404);
    assert_eq!(page("/b.html")["error"], "HTTP 404");
    assert_eq!(page("/a.html")["changes"], 1);
    assert!(page("/a.html")["last_changed"].is_string());
    assert_eq!(page("/a.html")["latest_snapshot"], changed["after"]);

    // C n'est plus liée (sans 404) : un parcours complet constate sa
    // disparition.
    set(&site, "/docs/", 200, index(&["a.html", "b.html", "prive.html"]));
    s.check(id).await;
    let changes = s.get(&format!("/api/targets/{id}/webchange/changes?limit=2")).await;
    assert_eq!(
        kinds(&changes),
        [
            ("".to_string(), "changed".to_string()),
            ("c.html".to_string(), "removed_page".to_string())
        ]
    );
    let pages = s.get(&format!("/api/targets/{id}/webchange/pages")).await;
    let c = pages["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["url"].as_str().unwrap().ends_with("/c.html"))
        .cloned()
        .unwrap();
    assert_eq!(c["status"], 404, "une page disparue s'affiche « Removed »");
    assert_eq!(c["error"], "No longer linked");

    // Pas de navigateur : aucune capture.
    let snapshot = changed["after"].as_i64().unwrap();
    let (status, _, _) =
        s.raw(&format!("/api/targets/{id}/webchange/snapshots/{snapshot}/screenshot")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Une capture présente est servie telle quelle, en cache immuable ; elle
    // n'est servie qu'à travers sa propre cible.
    sqlx::query("UPDATE webchange_snapshots SET has_screenshot = 1 WHERE id = ?")
        .bind(snapshot)
        .execute(&s.pool)
        .await
        .unwrap();
    let path = webchange::store::screenshot_path(&s.data_dir, id, snapshot);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"\xFF\xD8\xFFjpeg").unwrap();
    let (status, headers, body) =
        s.raw(&format!("/api/targets/{id}/webchange/snapshots/{snapshot}/screenshot")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "image/jpeg");
    assert!(headers[header::CACHE_CONTROL].to_str().unwrap().contains("immutable"));
    assert_eq!(body, b"\xFF\xD8\xFFjpeg");
    let other =
        s.create(&format!("{base}/autre/"), json!({ "allow_private_targets": "true" })).await;
    let (status, _, _) =
        s.raw(&format!("/api/targets/{other}/webchange/snapshots/{snapshot}/screenshot")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "l'instantané d'une autre cible");

    // Supprimer la cible efface tout, fichiers compris.
    let reply = s.app.delete(&format!("/api/targets/{id}"), Some(&s.cookie)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(!path.exists(), "capture effacée");
    let left: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM webchange_snapshots WHERE target_id = ?")
            .bind(id)
            .fetch_one(&s.pool)
            .await
            .unwrap();
    assert_eq!(left, 0);
}

fn urlencode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[tokio::test]
async fn une_page_de_depart_injoignable_rend_lequipement_hors_service() {
    let s = setup().await;
    // Un port fermé : connexion refusée.
    let id = s.create("http://127.0.0.1:1/", json!({ "allow_private_targets": "true" })).await;
    s.check(id).await;
    let probe = s.app.post(&format!("/api/targets/{id}/probe"), json!({}), Some(&s.cookie)).await;
    assert_eq!(probe.status, StatusCode::BAD_REQUEST);
    let message = probe.body["error"].as_str().unwrap();
    assert!(message.starts_with("Device unreachable"), "{message}");
    let target = s.get(&format!("/api/targets/{id}")).await;
    assert_eq!(target["error_kind"], "down");
}

#[tokio::test]
async fn la_boucle_locale_est_refusee_sans_autorisation_explicite() {
    let (base, site) = fake_site().await;
    set(&site, "/", 200, html("Accueil", "<p>x</p>"));
    let s = setup().await;
    let id = s.create(&format!("{base}/"), json!({})).await;
    s.check(id).await;
    let probe = s.app.post(&format!("/api/targets/{id}/probe"), json!({}), Some(&s.cookie)).await;
    assert_eq!(probe.status, StatusCode::BAD_REQUEST);
    assert!(probe.body["error"].as_str().unwrap().contains("allow_private_targets"));
    let target = s.get(&format!("/api/targets/{id}")).await;
    assert_eq!(target["error_kind"], "config", "un refus n'est pas une panne");
}

#[tokio::test]
async fn les_routes_refusent_un_autre_type_et_le_relais() {
    let s = setup().await;
    let reply = s
        .app
        .post(
            "/api/targets",
            json!({ "name": "Démo", "address": "demo", "kind": "dummy" }),
            Some(&s.cookie),
        )
        .await;
    let dummy = reply.body["id"].as_i64().unwrap();
    for uri in [
        format!("/api/targets/{dummy}/webchange/pages"),
        format!("/api/targets/{dummy}/webchange/changes"),
        format!("/api/targets/{dummy}/webchange/changes/1"),
        "/api/targets/9999/webchange/pages".to_string(),
    ] {
        let reply = s.app.get(&uri, Some(&s.cookie)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{uri}");
    }
    let reply = s
        .app
        .post(&format!("/api/targets/{dummy}/webchange/check"), json!({}), Some(&s.cookie))
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    let reply = s
        .app
        .post(
            "/api/targets",
            json!({
                "name": "Site",
                "address": "https://example.com/",
                "kind": "webchange",
                "via_agent": dummy
            }),
            Some(&s.cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(reply.body["error"].as_str().unwrap().contains("relay"), "{}", reply.body);
}
