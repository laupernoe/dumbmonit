//! Actualités du mode mur : bandeau de titres (flux RSS/Atom) et vignette vidéo.
//!
//! - `GET /api/wall/news` : ce que le mur interroge toutes les cinq minutes — les
//!   titres des flux réglés et l'adresse d'intégration de la vidéo. Le navigateur
//!   ne peut pas lire un flux tiers (CORS) : c'est le serveur qui le fait, avec le
//!   garde-fou des adresses internes, un délai, une taille maximale et un cache.
//! - `GET|PUT /api/wall/news/settings` : les réglages (administrateurs).
//! - `GET /api/wall/news/presets` : les sources proposées par pays
//!   (`wall_news_presets.rs`).
//! - `POST /api/wall/news/test` : lit un flux saisi et montre son premier titre.
//!
//! - `GET /api/wall/news/image/{id}` : l'image d'un article, relayée par le
//!   serveur (`wall_news_images.rs`) — un écran ne parle jamais au site d'actualité.
//!
//! Le carrousel d'images (`gallery`) lit les mêmes flux que le bandeau ; l'image
//! d'un article vient du flux (`media:content`, `media:thumbnail`, `enclosure`,
//! première `<img>` du contenu) et, à défaut, de la balise `og:image` de sa page,
//! lue avec les mêmes garde-fous et retenue longtemps.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewsSettings {
    pub ticker: bool,
    /// Carrousel des articles avec leur image, sur les mêmes flux.
    #[serde(default)]
    pub gallery: bool,
    pub feeds: Vec<Feed>,
    pub video: bool,
    /// Lien saisi (YouTube, Dailymotion, Twitch), tel que l'administrateur l'a collé.
    pub video_url: String,
    /// `right` (colonne de droite) ou `center` (au milieu, au-dessus du bandeau).
    pub video_position: String,
    /// `small`, `medium` ou `large`.
    pub video_size: String,
}

impl Default for NewsSettings {
    fn default() -> Self {
        Self {
            ticker: false,
            gallery: false,
            feeds: Vec::new(),
            video: false,
            video_url: String::new(),
            video_position: "right".into(),
            video_size: "medium".into(),
        }
    }
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
    settings.video_url = settings.video_url.trim().to_string();
    if settings.video_url.len() > MAX_URL_LEN {
        return Err("The video address is too long.".into());
    }
    if !settings.video_url.is_empty() && embed_url(&settings.video_url).is_none() {
        return Err("Use a YouTube, Dailymotion or Twitch link (video, live or channel).".into());
    }
    if settings.video && settings.video_url.is_empty() {
        return Err("Paste the video's link, or turn the video off.".into());
    }
    if !matches!(settings.video_position.as_str(), "right" | "center") {
        return Err("Unknown video position.".into());
    }
    if !matches!(settings.video_size.as_str(), "small" | "medium" | "large") {
        return Err("Unknown video size.".into());
    }
    Ok(())
}

pub(super) fn feed_url(text: &str) -> Option<Url> {
    let url = Url::parse(text).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then_some(url)
}

// ------------------------------------------------------------- vidéo intégrée

fn is_id(text: &str, len: std::ops::RangeInclusive<usize>) -> bool {
    len.contains(&text.len())
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn youtube_video(id: &str) -> Option<String> {
    is_id(id, 11..=11).then(|| {
        format!("https://www.youtube-nocookie.com/embed/{id}?autoplay=1&mute=1&rel=0&playsinline=1")
    })
}

fn youtube_channel(id: &str) -> Option<String> {
    (id.starts_with("UC") && is_id(id, 24..=24)).then(|| {
        format!(
            "https://www.youtube-nocookie.com/embed/live_stream?channel={id}\
             &autoplay=1&mute=1&rel=0&playsinline=1"
        )
    })
}

fn dailymotion(id: &str) -> Option<String> {
    let valid = (4..=12).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    valid.then(|| format!("https://www.dailymotion.com/embed/video/{id}?autoplay=1&mute=1"))
}

/// Twitch exige le domaine qui encadre le lecteur (`parent`) : c'est l'interface
/// qui l'ajoute, elle seule le connaît.
fn twitch(channel: &str) -> Option<String> {
    let valid = (3..=25).contains(&channel.len())
        && channel.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !matches!(channel, "videos" | "directory" | "downloads" | "settings");
    valid.then(|| {
        format!(
            "https://player.twitch.tv/?channel={}&muted=true&autoplay=true",
            channel.to_ascii_lowercase()
        )
    })
}

/// Transforme un lien de page YouTube, Dailymotion ou Twitch en adresse
/// d'intégration (`https`, origine fixe, identifiant validé), ou `None`. La
/// politique de sécurité du mur n'encadre que ces origines (`api/mod.rs`).
pub fn embed_url(input: &str) -> Option<String> {
    let url = Url::parse(input.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").or_else(|| host.strip_prefix("m.")).unwrap_or(&host);
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let query =
        |name: &str| url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned());
    match host {
        "youtu.be" => youtube_video(segments.first()?),
        "youtube.com" | "youtube-nocookie.com" => match segments.as_slice() {
            ["watch"] => youtube_video(&query("v")?),
            ["embed", "live_stream"] => youtube_channel(&query("channel")?),
            ["live" | "embed" | "shorts" | "v", id] => youtube_video(id),
            ["channel", id] | ["channel", id, "live"] => youtube_channel(id),
            _ => None,
        },
        "dai.ly" => dailymotion(segments.first()?),
        "dailymotion.com" | "geo.dailymotion.com" => match segments.as_slice() {
            ["video", id] | ["embed", "video", id] => dailymotion(id.split('_').next()?),
            ["player.html"] => dailymotion(&query("video")?),
            _ => None,
        },
        "twitch.tv" => twitch(segments.first()?),
        "player.twitch.tv" => twitch(&query("channel")?),
        _ => None,
    }
}

// --------------------------------------------------------------------- titres

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NewsItem {
    pub title: String,
    pub link: String,
    pub source: String,
    /// UTC sans suffixe ; absent quand le flux ne date pas l'article.
    pub published: Option<String>,
    /// Adresse relayée par le serveur (`/api/wall/news/image/<id>`), jamais
    /// celle du site : l'écran ne charge rien d'ailleurs.
    pub image: Option<String>,
    /// L'adresse d'origine de l'image, connue du serveur seul.
    #[serde(skip)]
    pub image_src: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeedProblem {
    pub feed: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct VideoView {
    pub embed_url: String,
    pub position: String,
    pub size: String,
}

#[derive(Debug, Serialize)]
pub struct WallNews {
    pub ticker: bool,
    pub gallery: bool,
    pub items: Vec<NewsItem>,
    pub problems: Vec<FeedProblem>,
    pub video: Option<VideoView>,
}

struct Cached {
    feeds: Vec<Feed>,
    pages: bool,
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
    let video = settings.video.then(|| embed_url(&settings.video_url)).flatten().map(|embed_url| {
        VideoView {
            embed_url,
            position: settings.video_position.clone(),
            size: settings.video_size.clone(),
        }
    });
    let wanted = settings.ticker || settings.gallery;
    let (items, problems) = if wanted && !settings.feeds.is_empty() {
        headlines(&settings.feeds, settings.gallery).await
    } else {
        (Vec::new(), Vec::new())
    };
    let problems =
        problems.into_iter().map(|(feed, message)| FeedProblem { feed, message }).collect();
    Ok(Json(WallNews {
        ticker: settings.ticker,
        gallery: settings.gallery,
        items,
        problems,
        video,
    }))
}

type Headlines = (Vec<NewsItem>, Vec<(String, String)>);

/// Les titres des flux, depuis le cache s'il est frais. Le verrou reste pris
/// pendant la lecture : cent murs qui se réveillent ensemble ne font qu'un tour.
///
/// `pages` : le carrousel est allumé, les articles sans image dans le flux
/// sont complétés par l'`og:image` de leur page (quelques-uns par tour).
async fn headlines(feeds: &[Feed], pages: bool) -> Headlines {
    let mut slot = cache().lock().await;
    if let Some(hit) = slot.as_ref()
        && hit.feeds == feeds
        && hit.pages == pages
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
    let mut items = merge(items);
    if pages {
        super::wall_news_images::complete_from_pages(&client, &mut items).await;
    }
    super::wall_news_images::publish(&mut items).await;
    let ttl = if items.is_empty() { CACHE_TTL_FAILED } else { CACHE_TTL };
    *slot = Some(Cached {
        feeds: feeds.to_vec(),
        pages,
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
        let image_src = super::wall_news_images::item_image(node, &link);
        items.push(NewsItem {
            title,
            link,
            source: source.to_string(),
            published,
            image: None,
            image_src,
        });
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
    fn refuses_html_and_unsafe_links() {
        assert!(parse("<html><body/></html>", "x").is_err());
        assert!(parse("pas du xml", "x").is_err());
        let xml = r#"<rss><channel><item><title>T</title><link>javascript:alert(1)</link></item></channel></rss>"#;
        assert!(parse(xml, "x").unwrap().is_empty());
    }

    #[test]
    fn embeds_only_known_players() {
        let yt = "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?autoplay=1&mute=1&rel=0&playsinline=1";
        for link in [
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://youtu.be/dQw4w9WgXcQ?t=3",
            "https://www.youtube.com/live/dQw4w9WgXcQ",
            "https://www.youtube.com/embed/dQw4w9WgXcQ",
        ] {
            assert_eq!(embed_url(link).as_deref(), Some(yt), "{link}");
        }
        let channel = format!("UC{}", "a".repeat(22));
        let live = embed_url(&format!("https://www.youtube.com/channel/{channel}/live")).unwrap();
        assert!(live.starts_with("https://www.youtube-nocookie.com/embed/live_stream?channel=UC"));
        let live2 =
            embed_url(&format!("https://www.youtube.com/embed/live_stream?channel={channel}"))
                .unwrap();
        assert_eq!(live, live2);
        assert!(
            embed_url("https://www.dailymotion.com/video/x8abcd")
                .unwrap()
                .starts_with("https://www.dailymotion.com/embed/video/x8abcd?")
        );
        assert_eq!(
            embed_url("https://www.twitch.tv/Some_Channel").as_deref(),
            Some("https://player.twitch.tv/?channel=some_channel&muted=true&autoplay=true")
        );
        for bad in [
            "https://evil.example/watch?v=dQw4w9WgXcQ",
            "https://www.youtube.com/watch?v=short",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ%22%3E",
            "javascript:alert(1)",
            "https://www.youtube.com.evil.example/watch?v=dQw4w9WgXcQ",
            "https://www.twitch.tv/videos",
            "https://www.twitch.tv/a&parent=evil",
            "",
        ] {
            assert_eq!(embed_url(bad), None, "{bad}");
        }
    }

    #[test]
    fn settings_are_checked() {
        let mut ok = NewsSettings {
            ticker: true,
            feeds: vec![Feed {
                url: " https://www.bfmtv.com/rss/news-24-7/ ".into(),
                label: "".into(),
            }],
            ..Default::default()
        };
        assert!(validate(&mut ok).is_ok());
        assert_eq!(ok.feeds[0].url, "https://www.bfmtv.com/rss/news-24-7/");
        let mut bad = ok.clone();
        bad.feeds[0].url = "ftp://x".into();
        assert!(validate(&mut bad).is_err());
        let mut video = ok.clone();
        video.video = true;
        assert!(validate(&mut video).is_err());
        video.video_url = "https://example.org/live".into();
        assert!(validate(&mut video).is_err());
    }

    #[test]
    fn merge_sorts_dedupes_and_caps() {
        let item = |link: &str, published: Option<&str>| NewsItem {
            title: "t".into(),
            link: link.into(),
            source: "s".into(),
            published: published.map(str::to_string),
            image: None,
            image_src: None,
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
