//! Actualités du mode mur : titres des flux RSS/Atom, en texte seul.
//!
//! - `GET /api/wall/news` : ce que le mur interroge toutes les cinq minutes — les
//!   titres des flux réglés. Le navigateur
//!   ne peut pas lire un flux tiers (CORS) : c'est le serveur qui le fait, avec le
//!   garde-fou des adresses internes, un délai, une taille maximale et un cache.
//! - `GET|PUT /api/wall/news/settings` : les réglages (administrateurs).
//! - `GET /api/wall/news/presets` : les sources proposées par pays
//!   (`wall_news_presets.rs`).
//! - `POST /api/wall/news/test` : lit un flux saisi et montre son premier titre.
//!
//! Les images des articles ne sont plus montrées : les anciens réglages
//! (`gallery`, la vidéo en direct) sont ignorés à la lecture.
//!
//! Tout est éteint par défaut. Les réglages vivent dans la table `settings`
//! (clé `wall.news`).

use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use dumbmonit_collectors::uptime::guard;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::wall_news_presets::{COUNTRIES, Country};
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::{AdminIdentity, Identity};
use crate::auth::settings;
use crate::state::AppState;

const SETTINGS_KEY: &str = "wall.news";
const MAX_FEEDS: usize = 8;
const MAX_URL_LEN: usize = 500;
const MAX_LABEL_LEN: usize = 40;
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const FEED_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_REDIRECTS: usize = 3;
const PER_FEED: usize = 15;
const MAX_ITEMS: usize = 40;
const MAX_TITLE_CHARS: usize = 220;
/// Durée de vie du cache ; plus courte quand tous les flux ont échoué.
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_TTL_FAILED: Duration = Duration::from_secs(60);
const USER_AGENT: &str = concat!("DumbMonit/", env!("CARGO_PKG_VERSION"), " (wall news ticker)");

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/wall/news", get(news))
        .route("/wall/news/settings", get(get_settings).put(put_settings))
        .route("/wall/news/presets", get(presets))
        .route("/wall/news/test", post(test_feed))
}

// ------------------------------------------------------------------ réglages

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Feed {
    pub url: String,
    /// Nom affiché devant les titres ; à défaut, le domaine du flux.
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewsSettings {
    pub ticker: bool,
    pub feeds: Vec<Feed>,
    // Les anciens champs des images (`gallery`) et de la vidéo en direct
    // (`video`, `video_url`…), retirées, sont ignorés à la lecture : serde laisse
    // passer les champs inconnus.
}

async fn load(state: &AppState) -> anyhow::Result<NewsSettings> {
    Ok(settings::get(&state.pool, SETTINGS_KEY).await?.unwrap_or_default())
}

async fn get_settings(
    State(state): State<AppState>,
    AdminIdentity(_): AdminIdentity,
) -> ApiResult<Json<NewsSettings>> {
    Ok(Json(load(&state).await?))
}

async fn put_settings(
    State(state): State<AppState>,
    AdminIdentity(_): AdminIdentity,
    Json(mut payload): Json<NewsSettings>,
) -> ApiResult<Json<NewsSettings>> {
    validate(&mut payload).map_err(ApiError::BadRequest)?;
    settings::set(&state.pool, SETTINGS_KEY, &payload).await?;
    *cache().lock().await = None;
    Ok(Json(payload))
}

async fn presets(Identity(_): Identity) -> Json<&'static [Country]> {
    Json(COUNTRIES)
}

/// Nettoie et valide les réglages reçus.
fn validate(settings: &mut NewsSettings) -> Result<(), String> {
    settings.feeds.retain(|feed| !feed.url.trim().is_empty());
    if settings.feeds.len() > MAX_FEEDS {
        return Err(format!("At most {MAX_FEEDS} feeds."));
    }
    for feed in &mut settings.feeds {
        feed.url = feed.url.trim().to_string();
        feed.label = feed.label.trim().chars().take(MAX_LABEL_LEN).collect();
        if feed.url.len() > MAX_URL_LEN {
            return Err("A feed address is too long.".into());
        }
        feed_url(&feed.url)
            .ok_or_else(|| format!("\"{}\" is not an http(s) address.", truncate(&feed.url, 60)))?;
    }
    Ok(())
}

pub(super) fn feed_url(text: &str) -> Option<Url> {
    let url = Url::parse(text).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then_some(url)
}

// --------------------------------------------------------------------- titres

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NewsItem {
    pub title: String,
    pub link: String,
    pub source: String,
    /// UTC sans suffixe ; absent quand le flux ne date pas l'article.
    pub published: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeedProblem {
    pub feed: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct WallNews {
    pub ticker: bool,
    pub items: Vec<NewsItem>,
    pub problems: Vec<FeedProblem>,
}

struct Cached {
    feeds: Vec<Feed>,
    at: Instant,
    ttl: Duration,
    items: Vec<NewsItem>,
    problems: Vec<(String, String)>,
}

fn cache() -> &'static Mutex<Option<Cached>> {
    static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

async fn news(State(state): State<AppState>, Identity(_): Identity) -> ApiResult<Json<WallNews>> {
    let settings = load(&state).await?;
    let (items, problems) = if settings.ticker && !settings.feeds.is_empty() {
        headlines(&settings.feeds).await
    } else {
        (Vec::new(), Vec::new())
    };
    let problems =
        problems.into_iter().map(|(feed, message)| FeedProblem { feed, message }).collect();
    Ok(Json(WallNews { ticker: settings.ticker, items, problems }))
}

type Headlines = (Vec<NewsItem>, Vec<(String, String)>);

/// Les titres des flux, depuis le cache s'il est frais. Le verrou reste pris
/// pendant la lecture : cent murs qui se réveillent ensemble ne font qu'un tour.
async fn headlines(feeds: &[Feed]) -> Headlines {
    let mut slot = cache().lock().await;
    if let Some(hit) = slot.as_ref()
        && hit.feeds == feeds
        && hit.at.elapsed() < hit.ttl
    {
        return (hit.items.clone(), hit.problems.clone());
    }
    let client = match client() {
        Ok(client) => client,
        Err(error) => return (Vec::new(), vec![(String::new(), error)]),
    };
    let mut tasks = tokio::task::JoinSet::new();
    for (index, feed) in feeds.iter().enumerate() {
        let (client, feed) = (client.clone(), feed.clone());
        tasks.spawn(async move { (index, feed.url.clone(), fetch(&client, &feed).await) });
    }
    let mut results = Vec::new();
    while let Some(done) = tasks.join_next().await {
        if let Ok(done) = done {
            results.push(done);
        }
    }
    results.sort_by_key(|(index, ..)| *index);
    let mut items = Vec::new();
    let mut problems = Vec::new();
    for (_, url, result) in results {
        match result {
            Ok(found) => items.extend(found),
            Err(message) => problems.push((url, message)),
        }
    }
    let items = merge(items);
    let ttl = if items.is_empty() { CACHE_TTL_FAILED } else { CACHE_TTL };
    *slot = Some(Cached {
        feeds: feeds.to_vec(),
        at: Instant::now(),
        ttl,
        items: items.clone(),
        problems: problems.clone(),
    });
    (items, problems)
}

/// Les plus récents d'abord (les articles sans date à la fin), sans doublon.
fn merge(mut items: Vec<NewsItem>) -> Vec<NewsItem> {
    items.sort_by(|a, b| b.published.cmp(&a.published));
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| seen.insert(item.link.clone()));
    items.truncate(MAX_ITEMS);
    items
}

#[derive(Debug, Deserialize)]
struct TestPayload {
    url: String,
}

#[derive(Debug, Serialize)]
struct TestResult {
    ok: bool,
    /// Pourquoi le flux n'a pas pu être lu.
    error: Option<String>,
    count: usize,
    first: Option<NewsItem>,
}

/// `POST /api/wall/news/test` — une lecture pour de vrai, sans cache, qui
/// montre le premier titre : l'administrateur voit que son flux répond avant de
/// l'enregistrer.
async fn test_feed(
    AdminIdentity(_): AdminIdentity,
    Json(payload): Json<TestPayload>,
) -> ApiResult<Json<TestResult>> {
    let url = payload.url.trim();
    if url.len() > MAX_URL_LEN || feed_url(url).is_none() {
        return Err(ApiError::BadRequest("Enter an http(s) address.".into()));
    }
    let outcome = match client() {
        Ok(client) => fetch(&client, &Feed { url: url.into(), label: String::new() }).await,
        Err(error) => Err(error),
    };
    Ok(Json(match outcome {
        Ok(items) if items.is_empty() => TestResult {
            ok: false,
            error: Some("The feed is readable but has no articles.".into()),
            count: 0,
            first: None,
        },
        Ok(items) => {
            TestResult { ok: true, error: None, count: items.len(), first: items.first().cloned() }
        }
        Err(error) => TestResult { ok: false, error: Some(error), count: 0, first: None },
    }))
}

pub(super) fn client() -> Result<reqwest::Client, String> {
    // Les noms sont vérifiés par le résolveur au moment de la connexion ; une
    // redirection vers une adresse littérale ne passe pas par lui.
    let redirects = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        let forbidden = attempt
            .url()
            .host_str()
            .map(|host| host.trim_matches(|c| c == '[' || c == ']'))
            .and_then(|host| host.parse().ok())
            .is_some_and(guard::is_forbidden);
        if forbidden {
            attempt.error("redirect to an internal address refused")
        } else {
            attempt.follow()
        }
    });
    reqwest::Client::builder()
        .dns_resolver(Arc::new(guard::GuardedResolver { allow_private: false }))
        .redirect(redirects)
        .connect_timeout(Duration::from_secs(5))
        .timeout(FEED_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| format!("HTTP client unavailable: {error}"))
}

async fn fetch(client: &reqwest::Client, feed: &Feed) -> Result<Vec<NewsItem>, String> {
    let url = feed_url(&feed.url).ok_or("not an http(s) address")?;
    guard::vet_url(&url, false, Duration::from_secs(5)).await.map_err(|error| error.to_string())?;
    let mut response = client
        .get(url.clone())
        .header(
            reqwest::header::ACCEPT,
            "application/rss+xml, application/atom+xml, application/xml, text/xml;q=0.9, */*;q=0.5",
        )
        .send()
        .await
        .map_err(describe)?;
    if !response.status().is_success() {
        return Err(format!("answered {}", response.status().as_u16()));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return Err("the feed is larger than 2 MB".into());
        }
        body.extend_from_slice(&chunk);
    }
    let text = String::from_utf8_lossy(&body);
    let source = if feed.label.is_empty() {
        url.host_str().unwrap_or_default().trim_start_matches("www.").to_string()
    } else {
        feed.label.clone()
    };
    parse(&text, &source)
}

pub(super) fn describe(error: reqwest::Error) -> String {
    if error.is_timeout() {
        return "no answer in time".into();
    }
    let error = error.without_url();
    let mut text = error.to_string();
    let mut cause = std::error::Error::source(&error);
    while let Some(inner) = cause {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        cause = inner.source();
    }
    truncate(&text, 160)
}

pub(super) fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

fn clean(text: &str) -> String {
    truncate(&text.split_whitespace().collect::<Vec<_>>().join(" "), MAX_TITLE_CHARS)
}

fn child_text(node: roxmltree::Node, name: &str) -> Option<String> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
        .map(clean)
        .filter(|text| !text.is_empty())
}

fn when(text: &str) -> Option<String> {
    let text = text.trim();
    let parsed =
        DateTime::parse_from_rfc2822(text).or_else(|_| DateTime::parse_from_rfc3339(text)).ok()?;
    Some(parsed.with_timezone(&Utc).format("%Y-%m-%dT%H:%M:%S").to_string())
}

/// Lit un document RSS 2.0 (ou RDF/RSS 1.0) ou Atom.
pub(super) fn parse(xml: &str, source: &str) -> Result<Vec<NewsItem>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|_| "not a readable RSS or Atom feed")?;
    let root = doc.root_element();
    let atom = root.tag_name().name() == "feed";
    if !atom && !matches!(root.tag_name().name(), "rss" | "RDF") {
        return Err("not an RSS or Atom feed".into());
    }
    let wanted = if atom { "entry" } else { "item" };
    let mut items = Vec::new();
    for node in root.descendants().filter(|n| n.is_element() && n.tag_name().name() == wanted) {
        let Some(title) = child_text(node, "title") else { continue };
        let link = if atom {
            let links =
                || node.children().filter(|c| c.is_element() && c.tag_name().name() == "link");
            links()
                .find(|l| matches!(l.attribute("rel"), None | Some("alternate")))
                .or_else(|| links().next())
                .and_then(|l| l.attribute("href"))
                .map(str::to_string)
        } else {
            child_text(node, "link")
        };
        let Some(link) = link.filter(|l| feed_url(l).is_some()) else { continue };
        let published = ["pubDate", "published", "updated", "date"]
            .iter()
            .find_map(|tag| child_text(node, tag))
            .and_then(|text| when(&text));
        items.push(NewsItem { title, link, source: source.to_string(), published });
    }
    items.sort_by(|a, b| b.published.cmp(&a.published));
    items.truncate(PER_FEED);
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rss_with_cdata_and_dates() {
        let xml = r#"<?xml version="1.0"?><rss version="2.0"><channel><title>X</title>
            <item><title><![CDATA[ Un  titre
            long ]]></title><link>https://ex.fr/a</link><pubDate>Wed, 14 Oct 2026 07:24:05 GMT</pubDate></item>
            <item><title>Sans lien</title></item></channel></rss>"#;
        let items = parse(xml, "ex.fr").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Un titre long");
        assert_eq!(items[0].published.as_deref(), Some("2026-10-14T07:24:05"));
        assert_eq!(items[0].source, "ex.fr");
    }

    #[test]
    fn reads_rdf_like_deutsche_welle() {
        let xml = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
            xmlns="http://purl.org/rss/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/">
            <item><title>T</title><link>https://dw.com/a</link><dc:date>2026-10-14T07:00:00Z</dc:date></item></rdf:RDF>"#;
        let items = parse(xml, "dw").unwrap();
        assert_eq!(items[0].published.as_deref(), Some("2026-10-14T07:00:00"));
    }

    #[test]
    fn reads_atom_and_prefers_the_alternate_link() {
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><title>T</title>
            <link rel="self" href="https://ex.fr/self"/><link rel="alternate" href="https://ex.fr/a"/>
            <updated>2026-10-14T09:00:00+02:00</updated></entry></feed>"#;
        let items = parse(xml, "ex").unwrap();
        assert_eq!(items[0].link, "https://ex.fr/a");
        assert_eq!(items[0].published.as_deref(), Some("2026-10-14T07:00:00"));
    }

    #[test]
    fn articles_without_a_picture_are_still_articles() {
        // Comme Hugging Face : titre, date et lien, rien d'autre.
        let xml = r#"<rss version="2.0"><channel><item><title>Scaling Indic Pretraining</title>
            <pubDate>Fri, 16 Oct 2026 01:44:47 GMT</pubDate><link>https://huggingface.co/blog/x</link>
            <guid isPermaLink="false">https://huggingface.co/blog/x</guid></item></channel></rss>"#;
        let items = parse(xml, "Hugging Face").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].published.as_deref(), Some("2026-10-16T01:44:47"));
        // Comme heise : Atom, un lien sans `rel`, du HTML dans le contenu.
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry>
            <title type="html"><![CDATA[PwnEscape]]></title><id>http://heise.de/-1</id>
            <updated>2026-10-19T11:03:00+02:00</updated>
            <link href="https://www.heise.de/news/a.html?wt_mc=rss"/>
            <content type="html"><![CDATA[<p><a href="https://www.heise.de/news/a.html"><img src="https://www.heise.de/scale/geometry/450/tux.png" class="webfeedsFeaturedVisual" alt="Tux" /></a></p>]]></content>
            </entry></feed>"#;
        let items = parse(atom, "heise").unwrap();
        assert_eq!(items[0].link, "https://www.heise.de/news/a.html?wt_mc=rss");
        assert_eq!(items[0].title, "PwnEscape");
    }

    #[test]
    fn refuses_html_and_unsafe_links() {
        assert!(parse("<html><body/></html>", "x").is_err());
        assert!(parse("pas du xml", "x").is_err());
        let xml = r#"<rss><channel><item><title>T</title><link>javascript:alert(1)</link></item></channel></rss>"#;
        assert!(parse(xml, "x").unwrap().is_empty());
    }

    #[test]
    fn settings_are_checked() {
        let mut ok = NewsSettings {
            ticker: true,
            feeds: vec![Feed {
                url: " https://www.bfmtv.com/rss/news-24-7/ ".into(),
                label: "".into(),
            }],
        };
        assert!(validate(&mut ok).is_ok());
        assert_eq!(ok.feeds[0].url, "https://www.bfmtv.com/rss/news-24-7/");
        let mut bad = ok.clone();
        bad.feeds[0].url = "ftp://x".into();
        assert!(validate(&mut bad).is_err());
    }

    #[test]
    fn old_video_and_picture_settings_are_ignored() {
        // La vidéo en direct et les images ont été retirées : un réglage
        // enregistré avant se lit toujours, ses champs sont simplement oubliés.
        let stored = r#"{"ticker":true,"gallery":true,"feeds":[],"video":true,
            "video_url":"https://youtu.be/dQw4w9WgXcQ","video_position":"right","video_size":"large"}"#;
        let settings: NewsSettings = serde_json::from_str(stored).unwrap();
        assert!(settings.ticker);
        let saved = serde_json::to_string(&settings).unwrap();
        assert!(!saved.contains("video") && !saved.contains("gallery"), "{saved}");
    }

    #[test]
    fn merge_sorts_dedupes_and_caps() {
        let item = |link: &str, published: Option<&str>| NewsItem {
            title: "t".into(),
            link: link.into(),
            source: "s".into(),
            published: published.map(str::to_string),
        };
        let merged = merge(vec![
            item("https://a/1", None),
            item("https://a/2", Some("2026-10-14T08:00:00")),
            item("https://a/2", Some("2026-10-14T08:00:00")),
            item("https://a/3", Some("2026-10-14T09:00:00")),
        ]);
        let links: Vec<&str> = merged.iter().map(|i| i.link.as_str()).collect();
        assert_eq!(links, ["https://a/3", "https://a/2", "https://a/1"]);
    }
}
