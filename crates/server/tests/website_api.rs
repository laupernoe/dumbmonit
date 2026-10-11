//! Santé et référencement d'un site, de bout en bout : un faux site servi en
//! local, des redirections, une boucle, une page blanche, `noindex`, un
//! changement de titre confirmé à la deuxième sonde, et ce que l'API en montre.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use dumbmonit_server::website::{self, WebsiteCollector};
use dumbmonit_server::{collectors, db};
use serde_json::{Value, json};

use common::TestApp;

/// Une réponse du faux site : statut, en-têtes, corps.
#[derive(Clone)]
struct Page {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

type Site = Arc<Mutex<HashMap<String, Page>>>;

async fn serve(State(site): State<Site>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    let Some(page) = site.lock().unwrap().get(&path).cloned() else {
        return (StatusCode::NOT_FOUND, "absent").into_response();
    };
    let mut response =
        (StatusCode::from_u16(page.status).unwrap(), page.body.clone()).into_response();
    for (name, value) in page.headers {
        response
            .headers_mut()
            .append(HeaderName::from_static(name), HeaderValue::from_str(&value).unwrap());
    }
    response
}

async fn fake_site() -> (String, Site) {
    let site: Site = Arc::new(Mutex::new(HashMap::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse");
    let router = Router::new().fallback(serve).with_state(site.clone());
    tokio::spawn(async move { axum::serve(listener, router).await.expect("faux site") });
    (format!("http://{address}"), site)
}

fn html_page(site: &Site, path: &str, body: String) {
    set(site, path, 200, vec![("content-type", "text/html; charset=utf-8".into())], body);
}

fn set(site: &Site, path: &str, status: u16, headers: Vec<(&'static str, String)>, body: String) {
    site.lock().unwrap().insert(path.to_string(), Page { status, headers, body });
}

fn redirect(site: &Site, path: &str, status: u16, to: &str) {
    set(site, path, status, vec![("location", to.to_string())], String::new());
}

/// Une page complète : assez de texte pour ne pas être blanche.
fn article(title: &str, description: &str, extra_head: &str) -> String {
    format!(
        "<!doctype html><html lang=\"fr\"><head><title>{title}</title>\
         <meta name=\"description\" content=\"{description}\">\
         <meta name=\"generator\" content=\"WordPress 6.6.2\">\
         <link rel=\"canonical\" href=\"/accueil\">{extra_head}</head>\
         <body><h1>Bienvenue</h1><p>{}</p></body></html>",
        "Du contenu bien réel pour les visiteurs. ".repeat(10)
    )
}

struct Setup {
    app: TestApp,
    cookie: String,
    collector: WebsiteCollector,
}

async fn setup() -> Setup {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.expect("base");
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(WebsiteCollector::new(pool.clone())));
    let collector = WebsiteCollector::new(pool.clone());
    let app = common::build_with_registry(dir, config, pool, registry).await;
    app.create_admin().await;
    let cookie = app.admin_cookie().await;
    Setup { app, cookie, collector }
}

impl Setup {
    /// Crée le site par l'API et rend la cible telle que le planificateur la
    /// passerait au collecteur.
    async fn create(&self, address: &str, extra: &[(&str, &str)]) -> Target {
        let mut tags: HashMap<String, String> =
            [("allow_private_targets".to_string(), "true".to_string())].into();
        tags.extend(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        let reply = self
            .app
            .post(
                "/api/targets",
                json!({ "name": "Site", "address": address, "kind": "website", "tags": tags }),
                Some(&self.cookie),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "création : {}", reply.body);
        Target {
            id: reply.body["id"].as_i64().unwrap(),
            name: "Site".into(),
            address: address.into(),
            kind: website::KIND.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags.into_iter().collect(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    async fn probe(&self, target: &Target) -> Vec<Sample> {
        self.collector.probe(target).await.expect("sonde réussie")
    }

    async fn get(&self, uri: &str) -> Value {
        let reply = self.app.get(uri, Some(&self.cookie)).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri} : {}", reply.body);
        reply.body
    }
}

fn value(samples: &[Sample], metric: &str) -> Option<f64> {
    samples.iter().find(|s| s.metric == metric).map(|s| s.value)
}

fn field<'a>(overview: &'a Value, name: &str) -> &'a Value {
    overview["fields"].as_array().unwrap().iter().find(|f| f["field"] == name).unwrap()
}

#[tokio::test]
async fn website_redirections_suivies_sans_erreur_et_chaine_notee() {
    let (base, site) = fake_site().await;
    redirect(&site, "/", 301, "/fr/");
    redirect(&site, "/fr/", 302, &format!("{base}/fr/accueil"));
    html_page(&site, "/fr/accueil", article("Accueil", "Le site", ""));
    set(&site, "/robots.txt", 200, vec![], "User-agent: *\nDisallow: /admin\n".into());
    set(
        &site,
        "/sitemap.xml",
        200,
        vec![("content-type", "application/xml".into())],
        "<urlset><url><loc>https://ex.fr/</loc></url></urlset>".into(),
    );
    let s = setup().await;
    let target = s.create(&format!("{base}/"), &[]).await;

    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_up"), Some(1.0));
    assert_eq!(value(&samples, "website_status_code"), Some(200.0));
    assert_eq!(value(&samples, "website_redirect_hops"), Some(2.0));
    assert_eq!(value(&samples, "website_blank"), Some(0.0));
    assert_eq!(value(&samples, "website_noindex"), Some(0.0));
    assert_eq!(value(&samples, "website_h1_count"), Some(1.0));
    assert_eq!(value(&samples, "website_robots_blocks_all"), Some(0.0));
    assert_eq!(value(&samples, "website_sitemap_ok"), Some(1.0));
    assert_eq!(value(&samples, "website_final_url_changed"), Some(0.0));
    let info = samples.iter().find(|s| s.metric == "website_info").unwrap();
    assert_eq!(info.labels["platform"], "wordpress");

    let overview = s.get(&format!("/api/targets/{}/website", target.id)).await;
    assert_eq!(overview["probe"]["final_url"], format!("{base}/fr/accueil"));
    assert_eq!(
        overview["probe"]["redirects"],
        json!([{ "url": format!("{base}/"), "status": 301 }, { "url": format!("{base}/fr/"), "status": 302 }])
    );
    assert_eq!(overview["probe"]["platform"], "wordpress");
    assert_eq!(overview["platform_label"], "WordPress");
    assert_eq!(field(&overview, "title")["value"], "Accueil");
    assert_eq!(field(&overview, "canonical")["value"], format!("{base}/accueil"));
    assert_eq!(overview["sitemap_ok"], true);
    assert!(overview["error"].is_null());

    // Deuxième sonde identique : rien ne change.
    s.probe(&target).await;
    let history = s.get(&format!("/api/targets/{}/website/changes", target.id)).await;
    assert_eq!(history, json!([]));
}

#[tokio::test]
async fn website_un_changement_de_titre_n_est_confirme_qu_a_la_deuxieme_sonde() {
    let (base, site) = fake_site().await;
    html_page(&site, "/", article("Accueil", "Le site", ""));
    let s = setup().await;
    let target = s.create(&base, &[]).await;
    s.probe(&target).await;

    // Une variante vue une seule fois (test A/B) : en attente, rien n'alerte.
    html_page(&site, "/", article("Soldes d'été", "Le site", ""));
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_title_changed"), Some(0.0));
    let overview = s.get(&format!("/api/targets/{}/website", target.id)).await;
    assert_eq!(field(&overview, "title")["value"], "Accueil");
    assert_eq!(field(&overview, "title")["pending"], "Soldes d'été");
    html_page(&site, "/", article("Accueil", "Le site", ""));
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_title_changed"), Some(0.0));
    let overview = s.get(&format!("/api/targets/{}/website", target.id)).await;
    assert!(field(&overview, "title")["pending"].is_null(), "l'attente est effacée");

    // Le même titre nouveau deux sondes de suite : confirmé.
    html_page(&site, "/", article("Nouveau  titre", "Le site", ""));
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_title_changed"), Some(0.0));
    html_page(&site, "/", article("nouveau titre", "Le site", ""));
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_title_changed"), Some(1.0));
    assert_eq!(value(&samples, "website_description_changed"), Some(0.0));

    let history = s.get(&format!("/api/targets/{}/website/changes", target.id)).await;
    let history = history.as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["field"], "title");
    assert_eq!(history[0]["before"], "Accueil");
    assert_eq!(history[0]["after"], "nouveau titre");
    let overview = s.get(&format!("/api/targets/{}/website", target.id)).await;
    assert_eq!(field(&overview, "title")["recently_changed"], true);
    assert!(field(&overview, "title")["changed_at"].is_string());

    // La jauge reste levée aux sondes suivantes (fenêtre de 24 h).
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_title_changed"), Some(1.0));
}

#[tokio::test]
async fn website_parametres_d_url_ignores_et_demenagement_detecte() {
    let (base, site) = fake_site().await;
    redirect(&site, "/", 302, "/accueil?utm_source=1");
    html_page(&site, "/accueil", article("Accueil", "Le site", ""));
    html_page(&site, "/ailleurs", article("Accueil", "Le site", ""));
    let s = setup().await;
    let target = s.create(&base, &[]).await;
    s.probe(&target).await;

    for n in 2..5 {
        redirect(&site, "/", 302, &format!("/accueil/?utm_source={n}&session={n}"));
        let samples = s.probe(&target).await;
        assert_eq!(value(&samples, "website_final_url_changed"), Some(0.0), "sonde {n}");
    }
    let overview = s.get(&format!("/api/targets/{}/website", target.id)).await;
    assert!(field(&overview, "final_url")["pending"].is_null());

    // Un vrai déménagement : autre chemin, confirmé à la deuxième sonde.
    redirect(&site, "/", 301, "/ailleurs");
    assert_eq!(value(&s.probe(&target).await, "website_final_url_changed"), Some(0.0));
    assert_eq!(value(&s.probe(&target).await, "website_final_url_changed"), Some(1.0));
}

#[tokio::test]
async fn website_boucle_et_exces_de_redirections_sont_des_pannes() {
    let (base, site) = fake_site().await;
    redirect(&site, "/a", 302, "/b");
    redirect(&site, "/b", 302, "/a");
    for i in 0..4 {
        redirect(&site, &format!("/n{i}"), 301, &format!("/n{}", i + 1));
    }
    html_page(&site, "/n4", article("Fin", "Le site", ""));
    let s = setup().await;

    let target = s.create(&format!("{base}/a"), &[]).await;
    match s.collector.probe(&target).await {
        Err(ProbeError::Unreachable(message)) => {
            assert!(message.starts_with("redirect loop:"), "{message}");
            assert!(message.contains("/b"), "{message}");
        }
        other => panic!("boucle attendue, obtenu {other:?}"),
    }

    let target = s.create(&format!("{base}/n0"), &[("max_redirects", "3")]).await;
    match s.collector.probe(&target).await {
        Err(error @ ProbeError::Unreachable(_)) => {
            assert!(error.means_down());
            assert!(error.to_string().contains("more than 3 redirects"), "{error}");
        }
        other => panic!("trop de redirections attendu, obtenu {other:?}"),
    }
    let target = s.create(&format!("{base}/n0?x=1"), &[("max_redirects", "4")]).await;
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_redirect_hops"), Some(4.0));
}

#[tokio::test]
async fn website_page_blanche_erreur_serveur_et_texte_attendu() {
    let (base, site) = fake_site().await;
    html_page(
        &site,
        "/",
        "<html><head><title>Boutique</title><script>boot()</script></head>\
         <body><div id=\"app\"></div></body></html>"
            .into(),
    );
    let s = setup().await;
    let target = s.create(&base, &[("expect_text", "Ajouter au panier")]).await;
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_up"), Some(1.0));
    assert_eq!(value(&samples, "website_blank"), Some(1.0));
    assert_eq!(value(&samples, "website_text_chars"), Some(0.0));
    assert_eq!(value(&samples, "website_expect_text_missing"), Some(1.0));

    set(&site, "/", 503, vec![], "Service Unavailable".into());
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_up"), Some(0.0));
    assert_eq!(value(&samples, "website_status_code"), Some(503.0));
    assert_eq!(value(&samples, "website_blank"), None, "une erreur n'est pas une page blanche");
}

#[tokio::test]
async fn website_noindex_par_balise_et_par_en_tete_et_robots_qui_bloque_tout() {
    let (base, site) = fake_site().await;
    html_page(
        &site,
        "/",
        article("Accueil", "Le site", "<meta name=\"robots\" content=\"noindex\">"),
    );
    set(&site, "/robots.txt", 200, vec![], "User-agent: *\nDisallow: /\n".into());
    let s = setup().await;
    let target = s.create(&base, &[]).await;
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_noindex"), Some(1.0));
    assert_eq!(value(&samples, "website_robots_blocks_all"), Some(1.0));
    assert_eq!(value(&samples, "website_sitemap_ok"), Some(0.0));

    set(
        &site,
        "/",
        200,
        vec![("content-type", "text/html".into()), ("x-robots-tag", "noindex, nofollow".into())],
        article("Accueil", "Le site", ""),
    );
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_noindex"), Some(1.0));
    assert_eq!(value(&samples, "website_nofollow"), Some(1.0));

    html_page(&site, "/", article("Accueil", "Le site", ""));
    let samples = s.probe(&target).await;
    assert_eq!(value(&samples, "website_noindex"), Some(0.0));
}

#[tokio::test]
async fn website_api_refuse_une_autre_cible_et_un_relais() {
    let s = setup().await;
    let reply = s
        .app
        .post(
            "/api/targets",
            json!({ "name": "Démo", "address": "demo", "kind": "dummy" }),
            Some(&s.cookie),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
    let id = reply.body["id"].as_i64().unwrap();
    let reply = s.app.get(&format!("/api/targets/{id}/website"), Some(&s.cookie)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = s.app.get(&format!("/api/targets/{id}/website/changes"), Some(&s.cookie)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    // Avant toute sonde, la vue est vide mais répond.
    let target = s.create("https://example.com", &[]).await;
    let overview = s.get(&format!("/api/targets/{}/website", target.id)).await;
    assert!(overview["probe"].is_null());
    assert_eq!(overview["fields"], json!([]));
}

#[tokio::test]
async fn website_le_type_est_decrit_avec_ses_options() {
    let s = setup().await;
    let kinds = s.get("/api/collectors").await;
    let website = kinds
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["kind"] == "website")
        .unwrap_or_else(|| panic!("type website absent : {kinds}"));
    assert_eq!(website["label"], "Website health & SEO");
    let keys: Vec<&str> =
        website["options"].as_array().unwrap().iter().map(|o| o["key"].as_str().unwrap()).collect();
    for key in ["expect_text", "min_text_chars", "max_redirects", "user_agent", "ignore_query"] {
        assert!(keys.contains(&key), "{key} absent : {keys:?}");
    }
}

#[tokio::test]
async fn website_identify_pour_l_import() {
    let (base, site) = fake_site().await;
    redirect(&site, "/", 301, "/shop");
    set(
        &site,
        "/shop",
        200,
        vec![("x-shopify-stage", "production".into())],
        "<html></html>".into(),
    );
    let identity = website::identify(&base, Duration::from_secs(5), true).await.unwrap();
    assert_eq!(identity.platform, website::Platform::Shopify);
    assert_eq!(identity.final_url.as_str(), format!("{base}/shop"));
    // Sans autorisation, la boucle locale est refusée, comme pour les sondes.
    assert!(matches!(
        website::identify(&base, Duration::from_secs(5), false).await,
        Err(ProbeError::Config(_))
    ));
}
