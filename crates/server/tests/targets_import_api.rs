//! Import CSV de sites (`POST /api/targets/import`) de bout en bout : droits,
//! aperçu, erreurs par ligne, doublons, dossiers explicites ou déduits de la
//! plateforme (lue sur un faux site servi en local).

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Html;
use axum::routing::get;
use dumbmonit_server::website::WebsiteCollector;
use dumbmonit_server::{collectors, db};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{Reply, TestApp};

const ENTETE: &str = "url,name,folder,tags,interval,expect_text";

struct Lab {
    app: TestApp,
    admin: String,
}

async fn lab() -> Lab {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(WebsiteCollector::new(pool.clone())));
    let app = common::build_with_registry(dir, config, pool, registry).await;
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    Lab { app, admin }
}

/// Faux sites : `/wp` annonce WordPress par `generator`, `/wc` par `wp-content`,
/// `/plain` n'annonce rien.
async fn fake_sites() -> String {
    let router = Router::new()
        .route(
            "/wp",
            get(|| async {
                Html(
                    "<html><head><meta name=\"generator\" content=\"WordPress 6.6.2\">\
                     <title>Blog</title></head><body>Bonjour</body></html>",
                )
            }),
        )
        .route(
            "/wc",
            get(|| async {
                Html(
                    "<html><head><link rel=\"stylesheet\" href=\"/wp-content/themes/x/style.css\">\
                     <title>Boutique</title></head><body>Bonjour</body></html>",
                )
            }),
        )
        .route(
            "/shop",
            get(|| async {
                Html(
                    "<html><head><meta property=\"og:type\" content=\"product\">\
                     <title>Boutique</title></head><body>Bonjour</body></html>",
                )
            }),
        )
        .route(
            "/plain",
            get(|| async {
                Html("<html><head><title>Simple</title></head><body>Salut</body></html>")
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    tokio::spawn(async move { axum::serve(listener, router).await.expect("faux sites") });
    format!("http://{address}")
}

/// Envoie un corps brut (`text/csv` ou autre type) à l'import.
async fn post_raw(
    app: &TestApp,
    uri: &str,
    content_type: &str,
    body: String,
    cookie: Option<&str>,
) -> Reply {
    let mut builder = Request::builder().method("POST").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie).header("x-requested-with", "DumbMonit");
    }
    let request =
        builder.header(header::CONTENT_TYPE, content_type).body(Body::from(body)).unwrap();
    let response = app.router.clone().oneshot(request).await.expect("réponse");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Reply { status, body, set_cookie: None, location: None }
}

async fn import_csv(lab: &Lab, csv: &str) -> Reply {
    post_raw(&lab.app, "/api/targets/import", "text/csv", csv.to_string(), Some(&lab.admin)).await
}

async fn targets(lab: &Lab) -> Vec<Value> {
    let reply = lab.app.get("/api/targets", Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    reply.body.as_array().expect("liste").clone()
}

#[tokio::test]
async fn seul_un_administrateur_peut_importer() {
    let lab = lab().await;
    let csv = format!("{ENTETE}\nhttps://a.example.com,A,Lab,,,\n");

    let anonyme = post_raw(&lab.app, "/api/targets/import", "text/csv", csv.clone(), None).await;
    assert_eq!(anonyme.status, StatusCode::UNAUTHORIZED, "{}", anonyme.body);

    for (nom, role, mot_de_passe) in [
        ("ops", "operator", "mot-de-passe-operateur"),
        ("watcher", "viewer", common::VIEWER_PASSWORD),
    ] {
        let cree = lab
            .app
            .post(
                "/api/users",
                json!({ "username": nom, "role": role, "password": mot_de_passe }),
                Some(&lab.admin),
            )
            .await;
        assert_eq!(cree.status, StatusCode::CREATED, "{nom}: {}", cree.body);
        let cookie = lab.app.login_as(nom, mot_de_passe).await.cookie();
        let reply =
            post_raw(&lab.app, "/api/targets/import", "text/csv", csv.clone(), Some(&cookie)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{role}: {}", reply.body);
    }
    assert!(targets(&lab).await.is_empty(), "aucune cible créée par un refus");

    let admin = import_csv(&lab, &csv).await;
    assert_eq!(admin.status, StatusCode::CREATED, "{}", admin.body);
}

#[tokio::test]
async fn l_apercu_ne_cree_rien() {
    let lab = lab().await;
    let csv = format!("{ENTETE}\nhttps://a.example.com,A,Lab,,,\nhttps://b.example.com,B,Lab,,,\n");
    let reply = post_raw(
        &lab.app,
        "/api/targets/import?dry_run=true",
        "text/csv",
        csv.clone(),
        Some(&lab.admin),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["dry_run"], json!(true));
    assert_eq!(reply.body["total"], json!(2));
    assert_eq!(reply.body["created"], json!(2));
    assert_eq!(reply.body["errors"], json!(0));
    for row in reply.body["rows"].as_array().unwrap() {
        assert_eq!(row["status"], json!("ok"), "{row}");
        assert!(row["id"].is_null(), "{row}");
    }
    assert!(targets(&lab).await.is_empty());

    // Le même fichier, pour de bon, crée les deux cibles.
    let reel = import_csv(&lab, &csv).await;
    assert_eq!(reel.status, StatusCode::CREATED, "{}", reel.body);
    assert_eq!(reel.body["dry_run"], json!(false));
    assert_eq!(reel.body["created"], json!(2));
    assert_eq!(targets(&lab).await.len(), 2);
}

#[tokio::test]
async fn les_colonnes_deviennent_les_options_du_site() {
    let lab = lab().await;
    let csv = format!(
        "{ENTETE}\nhttps://a.example.com,Alpha,Lab/Sites,env=prod; team=ops,5m,Bienvenue\n\
         https://b.example.com,,Lab/Sites,,90,\n"
    );
    let reply = import_csv(&lab, &csv).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["created"], json!(2), "{}", reply.body);
    let rows = reply.body["rows"].as_array().unwrap();
    assert_eq!(rows[0]["row"], json!(2));
    assert_eq!(rows[0]["status"], json!("created"));
    assert_eq!(rows[0]["folder"], json!("Lab/Sites"), "dossier explicite conservé");

    let id = rows[0]["id"].as_i64().expect("id");
    let cible = lab.app.get(&format!("/api/targets/{id}"), Some(&lab.admin)).await;
    assert_eq!(cible.status, StatusCode::OK, "{}", cible.body);
    assert_eq!(cible.body["kind"], json!("website"));
    assert_eq!(cible.body["name"], json!("Alpha"));
    assert_eq!(cible.body["group_name"], json!("Lab/Sites"));
    assert_eq!(cible.body["interval_secs"], json!(300));
    assert_eq!(cible.body["tags"]["env"], json!("prod"));
    assert_eq!(cible.body["tags"]["team"], json!("ops"));
    assert_eq!(cible.body["tags"]["expect_text"], json!("Bienvenue"));

    // Sans nom, c'est l'hôte ; l'intervalle en secondes nues est accepté.
    let id = rows[1]["id"].as_i64().expect("id");
    let cible = lab.app.get(&format!("/api/targets/{id}"), Some(&lab.admin)).await;
    assert_eq!(cible.body["name"], json!("b.example.com"));
    assert_eq!(cible.body["interval_secs"], json!(90));
    assert_eq!(cible.body["kind"], json!("website"));
}

#[tokio::test]
async fn chaque_ligne_en_erreur_est_signalee_sans_bloquer_les_autres() {
    let lab = lab().await;
    let csv = format!(
        "{ENTETE}\n\
         https://ok.example.com,Bon,Lab,,,\n\
         ,Sans adresse,Lab,,,\n\
         ftp://fichiers.example.com,Ftp,Lab,,,\n\
         not a url,Cassée,Lab,,,\n\
         https://tags.example.com,Tags,Lab,oops,,\n\
         https://delai.example.com,Délai,Lab,,bientôt,\n\
         https://court.example.com,Court,Lab,,5,\n"
    );
    let reply = import_csv(&lab, &csv).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["total"], json!(7));
    assert_eq!(reply.body["created"], json!(1));
    assert_eq!(reply.body["errors"], json!(6), "{}", reply.body);
    let rows = reply.body["rows"].as_array().unwrap();
    assert_eq!(rows[0]["status"], json!("created"));
    for (i, row) in rows.iter().enumerate().skip(1) {
        assert_eq!(row["status"], json!("error"), "ligne {i}: {row}");
        assert_eq!(row["row"], json!(i + 2), "numéro de ligne du fichier");
        assert!(row["message"].as_str().is_some_and(|m| !m.is_empty()), "{row}");
    }
    assert!(rows[1]["message"].as_str().unwrap().contains("empty"), "{}", rows[1]);
    assert!(rows[4]["message"].as_str().unwrap().contains("key=value"), "{}", rows[4]);
    assert!(rows[5]["message"].as_str().unwrap().contains("interval"), "{}", rows[5]);
    assert!(rows[6]["message"].as_str().unwrap().contains("at least"), "{}", rows[6]);
    assert_eq!(targets(&lab).await.len(), 1, "seule la ligne valide est créée");
}

#[tokio::test]
async fn l_en_tete_avec_une_colonne_url_est_obligatoire() {
    let lab = lab().await;
    for csv in ["", "a.example.com\nb.example.com\n", "name,folder\nA,Lab\n"] {
        let reply = import_csv(&lab, csv).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{csv:?}: {}", reply.body);
    }
    assert!(targets(&lab).await.is_empty());
}

#[tokio::test]
async fn au_dela_de_mille_lignes_tout_est_refuse() {
    let lab = lab().await;
    let mut csv = String::from("url,folder\n");
    for i in 0..1001 {
        csv.push_str(&format!("https://site{i}.example.com,Lab\n"));
    }
    let reply = import_csv(&lab, &csv).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{}", reply.body);
    assert!(targets(&lab).await.is_empty());
}

#[tokio::test]
async fn les_doublons_sont_ignores_et_signales() {
    let lab = lab().await;
    // Un site déjà surveillé (même adresse à « www. », schéma et barre près).
    let existant = lab
        .app
        .post(
            "/api/targets",
            json!({
                "name": "Déjà là",
                "address": "https://www.deja.example.com/",
                "kind": "website"
            }),
            Some(&lab.admin),
        )
        .await;
    assert_eq!(existant.status, StatusCode::CREATED, "{}", existant.body);

    let csv = format!(
        "{ENTETE}\n\
         http://deja.example.com,Doublon,Lab,,,\n\
         https://neuf.example.com,Neuf,Lab,,,\n\
         https://neuf.example.com/,Neuf bis,Lab,,,\n"
    );
    let reply = import_csv(&lab, &csv).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["created"], json!(1), "{}", reply.body);
    assert_eq!(reply.body["skipped"], json!(2), "{}", reply.body);
    assert_eq!(reply.body["errors"], json!(0));
    let rows = reply.body["rows"].as_array().unwrap();
    assert_eq!(rows[0]["status"], json!("skipped"));
    assert!(rows[0]["message"].as_str().unwrap().contains("Already monitored"));
    assert_eq!(rows[1]["status"], json!("created"));
    assert_eq!(rows[2]["status"], json!("skipped"), "doublon au sein du fichier");
    assert_eq!(targets(&lab).await.len(), 2);
}

#[tokio::test]
async fn sans_dossier_la_plateforme_decide_du_dossier() {
    let lab = lab().await;
    let base = fake_sites().await;
    let csv = format!("url,name\n{base}/wp,Blog\n{base}/wc,Boutique\n{base}/plain,Simple\n");
    let reply = import_csv(&lab, &csv).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["created"], json!(3), "{}", reply.body);
    let rows = reply.body["rows"].as_array().unwrap();
    assert_eq!(rows[0]["folder"], json!("Websites/WordPress"), "{}", rows[0]);
    assert_eq!(rows[0]["platform"], json!("WordPress"));
    assert_eq!(rows[1]["folder"], json!("Websites/WordPress"), "{}", rows[1]);
    assert_eq!(rows[2]["folder"], json!("Websites/Other"), "{}", rows[2]);

    let id = rows[0]["id"].as_i64().expect("id");
    let cible = lab.app.get(&format!("/api/targets/{id}"), Some(&lab.admin)).await;
    assert_eq!(cible.body["group_name"], json!("Websites/WordPress"));
    assert_eq!(cible.body["kind"], json!("website"));

    // Un site injoignable retombe aussi dans « Other ».
    let mort = import_csv(&lab, "url\nhttp://127.0.0.1:1/\n").await;
    assert_eq!(mort.body["rows"][0]["folder"], json!("Websites/Other"), "{}", mort.body);
}

#[tokio::test]
async fn la_categorie_donnee_ou_devinee_range_le_site() {
    let lab = lab().await;
    let base = fake_sites().await;
    let csv = format!("url,category\n{base}/shop,\n{base}/wp,Clients\n{base}/plain,\n");
    let reply = import_csv(&lab, &csv).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let rows = reply.body["rows"].as_array().unwrap();
    assert_eq!(rows[0]["folder"], json!("Websites/E-commerce"), "{}", rows[0]);
    assert_eq!(rows[0]["category"], json!("E-commerce"));
    assert_eq!(rows[1]["folder"], json!("Websites/Clients"), "catégorie donnée : elle prime");
    assert_eq!(rows[2]["folder"], json!("Websites/Other"), "trop incertain : plateforme");
    assert!(rows[2]["category"].is_null());

    let id = rows[0]["id"].as_i64().expect("id");
    let cible = lab.app.get(&format!("/api/targets/{id}"), Some(&lab.admin)).await;
    assert_eq!(cible.body["tags"]["category"], json!("ecommerce"), "{}", cible.body);
    let id = rows[1]["id"].as_i64().expect("id");
    let cible = lab.app.get(&format!("/api/targets/{id}"), Some(&lab.admin)).await;
    assert_eq!(cible.body["tags"]["category"], json!("Clients"));

    let invalide = import_csv(&lab, &format!("url,category\n{base}/wp,a/b\n")).await;
    assert_eq!(invalide.body["errors"], json!(1), "{}", invalide.body);
}

#[tokio::test]
async fn un_dossier_explicite_evite_la_detection() {
    let lab = lab().await;
    let base = fake_sites().await;
    let csv = format!("url,folder\n{base}/wp,Clients\n");
    let reply = import_csv(&lab, &csv).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["rows"][0]["folder"], json!("Clients"));
    assert!(reply.body["rows"][0]["platform"].is_null(), "{}", reply.body);
}

#[tokio::test]
async fn le_corps_json_csv_vaut_le_corps_text_csv() {
    let lab = lab().await;
    let csv = format!("{ENTETE}\nhttps://json.example.com,Json,Lab,,,\n");
    let reply = lab.app.post("/api/targets/import", json!({ "csv": csv }), Some(&lab.admin)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    assert_eq!(reply.body["created"], json!(1));
    assert_eq!(targets(&lab).await[0]["name"], json!("Json"));

    let mauvais = post_raw(
        &lab.app,
        "/api/targets/import",
        "application/json",
        "{\"texte\": 1}".into(),
        Some(&lab.admin),
    )
    .await;
    assert_eq!(mauvais.status, StatusCode::BAD_REQUEST, "{}", mauvais.body);
}
