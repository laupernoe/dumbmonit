//! Images des articles du mur : trouvées dans le flux (ou, à défaut, dans la
//! page de l'article), puis relayées par le serveur.
//!
//! - [`item_image`] lit l'image d'un article dans son flux : `media:content`
//!   (la plus large), `media:thumbnail`, `enclosure` d'un type image, lien Atom
//!   `rel="enclosure"`, et en dernier lieu la première `<img>` du contenu.
//! - [`complete_from_pages`] complète les articles qui n'en ont pas avec
//!   l'`og:image` (ou `twitter:image`) de leur page : quelques pages par tour,
//!   lues avec le garde-fou des adresses internes, 512 Kio au plus, et le
//!   résultat retenu par lien — une page n'est lue qu'une fois.
//! - [`publish`] remplace l'adresse d'origine par `/api/wall/news/image/<id>` :
//!   l'écran ne parle jamais au site d'actualité (ni son adresse IP ni ses
//!   cookies), et la politique de sécurité du mur reste `img-src 'self'`.
//! - `GET /api/wall/news/image/{id}` ne sert que les images des articles
//!   publiés (liste d'autorisation), lues avec le même client gardé, 6 Mo au
//!   plus, un type `image/*` annoncé **et** une signature PNG, JPEG, GIF, WebP
//!   ou AVIF reconnue — jamais de SVG, qui porterait des scripts. Un petit cache
//!   en mémoire évite de relire la même image pour chaque écran.

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

use axum::extract::Path;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, body::Bytes};
use dumbmonit_collectors::uptime::guard;
use regex::Regex;
use reqwest::Url;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::wall_news::{NewsItem, client, describe, feed_url};
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::Identity;
use crate::state::AppState;

const MEDIA_NS: &str = "http://search.yahoo.com/mrss/";
/// Pages d'articles lues par tour de lecture des flux, au plus.
const PAGES_PER_ROUND: usize = 8;
const MAX_PAGE_BYTES: usize = 512 * 1024;
const PAGE_TIMEOUT: Duration = Duration::from_secs(5);
/// Liens dont on retient l'`og:image` (trouvée ou non).
const MAX_REMEMBERED_PAGES: usize = 500;
/// Images que le relais accepte de servir (les articles publiés récemment).
const MAX_ALLOWED: usize = 600;
const MAX_IMAGE_BYTES: usize = 6 * 1024 * 1024;
/// Octets d'images gardés en mémoire, toutes confondues.
const MAX_CACHED_BYTES: usize = 64 * 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new().route("/wall/news/image/{id}", get(image))
}

// ------------------------------------------------------------ dans le flux

fn looks_like_image(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".gif", ".webp", ".avif"].iter().any(|ext| path.ends_with(ext))
}

/// Les entités qu'on trouve dans un attribut HTML déjà extrait du XML.
fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&").replace("&#38;", "&").replace("&#x26;", "&").replace("&quot;", "\"")
}

/// Adresse absolue `http(s)`, relative à la page de l'article au besoin.
fn resolve(raw: &str, base: &str) -> Option<String> {
    let raw = decode_entities(raw.trim());
    if raw.is_empty() || raw.starts_with("data:") {
        return None;
    }
    let url = match Url::parse(base) {
        Ok(base) => base.join(&raw).ok()?,
        Err(_) => Url::parse(&raw).ok()?,
    };
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then(|| url.to_string())
}

fn first_img(html: &str) -> Option<String> {
    static IMG: OnceLock<Regex> = OnceLock::new();
    let re = IMG.get_or_init(|| {
        Regex::new(r#"(?is)<img\b[^>]*?\bsrc\s*=\s*["']([^"']+)["']"#).expect("expression valide")
    });
    re.captures_iter(html)
        .map(|c| c[1].to_string())
        // Les pixels de suivi et les émoticônes ne font pas une illustration.
        .find(|src| !src.contains("feeds.feedburner.com") && !src.contains("/emoji/"))
}

/// L'image d'un article de flux RSS ou Atom, adresse d'origine absolue.
pub fn item_image(node: roxmltree::Node, link: &str) -> Option<String> {
    // 1. Media RSS : la plus large des `media:content` images, sinon une vignette.
    let mut best: Option<(u32, String)> = None;
    for media in node.descendants().filter(|n| n.is_element()) {
        if media.tag_name().namespace() != Some(MEDIA_NS) {
            continue;
        }
        let Some(url) = media.attribute("url") else { continue };
        let rank = match media.tag_name().name() {
            "content" => {
                let medium = media.attribute("medium");
                let kind = media.attribute("type");
                let image = medium == Some("image")
                    || kind.is_some_and(|t| t.starts_with("image/"))
                    || (medium.is_none() && kind.is_none() && looks_like_image(url));
                if !image {
                    continue;
                }
                // Une largeur inconnue passe après une largeur annoncée.
                media.attribute("width").and_then(|w| w.parse().ok()).unwrap_or(2).max(2)
            }
            "thumbnail" => 1,
            _ => continue,
        };
        if let Some(url) = resolve(url, link)
            && best.as_ref().is_none_or(|(r, _)| rank > *r)
        {
            best = Some((rank, url));
        }
    }
    if let Some((_, url)) = best {
        return Some(url);
    }
    // 2. Pièce jointe image (RSS `enclosure`, Atom `link rel="enclosure"`).
    for child in node.children().filter(|n| n.is_element()) {
        let name = child.tag_name().name();
        let image_type = child.attribute("type").is_some_and(|t| t.starts_with("image/"));
        let url = match name {
            "enclosure" if image_type => child.attribute("url"),
            "link" if child.attribute("rel") == Some("enclosure") && image_type => {
                child.attribute("href")
            }
            _ => None,
        };
        if let Some(url) = url.and_then(|u| resolve(u, link)) {
            return Some(url);
        }
    }
    // 3. La première image du contenu HTML.
    for wanted in ["encoded", "content", "description", "summary"] {
        for child in node.children().filter(|n| n.is_element() && n.tag_name().name() == wanted) {
            let html: String = child.descendants().filter_map(|n| n.text()).collect();
            if let Some(url) = first_img(&html).and_then(|src| resolve(&src, link)) {
                return Some(url);
            }
        }
    }
    None
}

// ----------------------------------------------------- dans la page (og:image)

/// L'`og:image` (ou `twitter:image`) d'une page HTML.
pub fn page_image(html: &str, page: &str) -> Option<String> {
    static META: OnceLock<Regex> = OnceLock::new();
    static ATTR: OnceLock<Regex> = OnceLock::new();
    let meta = META.get_or_init(|| Regex::new(r"(?is)<meta\b[^>]*>").expect("expression valide"));
    let attr = ATTR.get_or_init(|| {
        Regex::new(r#"(?is)\b([a-z:_-]+)\s*=\s*(?:"([^"]*)"|'([^']*)')"#)
            .expect("expression valide")
    });
    let mut found: Option<(u8, String)> = None;
    for tag in meta.find_iter(html) {
        let mut name = None;
        let mut content = None;
        for c in attr.captures_iter(tag.as_str()) {
            let value = c.get(2).or_else(|| c.get(3)).map(|m| m.as_str().to_string());
            match c[1].to_ascii_lowercase().as_str() {
                "property" | "name" => name = value.map(|v| v.to_ascii_lowercase()),
                "content" => content = value,
                _ => {}
            }
        }
        let rank = match name.as_deref() {
            Some("og:image:secure_url") => 3,
            Some("og:image" | "og:image:url") => 2,
            Some("twitter:image" | "twitter:image:src") => 1,
            _ => continue,
        };
        if let Some(url) = content.and_then(|c| resolve(&c, page))
            && found.as_ref().is_none_or(|(r, _)| rank > *r)
        {
            found = Some((rank, url));
        }
    }
    found.map(|(_, url)| url)
}

/// `og:image` déjà cherchée, par lien d'article (`None` : la page n'en a pas).
fn remembered() -> &'static Mutex<HashMap<String, Option<String>>> {
    static PAGES: LazyLock<Mutex<HashMap<String, Option<String>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    &PAGES
}

async fn read_page(client: &reqwest::Client, link: &str) -> Result<Option<String>, String> {
    let url = feed_url(link).ok_or("not an http(s) address")?;
    guard::vet_url(&url, false, PAGE_TIMEOUT).await.map_err(|e| e.to_string())?;
    let mut response = client
        .get(url.clone())
        .header(header::ACCEPT, "text/html,application/xhtml+xml;q=0.9")
        .timeout(PAGE_TIMEOUT)
        .send()
        .await
        .map_err(describe)?;
    if !response.status().is_success() {
        return Err(format!("answered {}", response.status().as_u16()));
    }
    // Les balises `<meta>` sont dans l'en-tête : le début de la page suffit.
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
        body.extend_from_slice(&chunk);
        if body.len() >= MAX_PAGE_BYTES {
            body.truncate(MAX_PAGE_BYTES);
            break;
        }
    }
    let final_url = response.url().to_string();
    Ok(page_image(&String::from_utf8_lossy(&body), &final_url))
}

/// Cherche l'image des articles qui n'en ont pas dans leur flux, dans leur page.
pub async fn complete_from_pages(client: &reqwest::Client, items: &mut [NewsItem]) {
    let mut wanted = Vec::new();
    {
        let known = remembered().lock().await;
        for (index, item) in items.iter_mut().enumerate() {
            if item.image_src.is_some() {
                continue;
            }
            match known.get(&item.link) {
                Some(found) => item.image_src = found.clone(),
                None if wanted.len() < PAGES_PER_ROUND => wanted.push(index),
                None => {}
            }
        }
    }
    if wanted.is_empty() {
        return;
    }
    let mut tasks = tokio::task::JoinSet::new();
    for index in wanted {
        let (client, link) = (client.clone(), items[index].link.clone());
        tasks.spawn(async move { (index, read_page(&client, &link).await) });
    }
    let mut known = remembered().lock().await;
    while let Some(done) = tasks.join_next().await {
        let Ok((index, result)) = done else { continue };
        // Une page qui ne répond pas sera retentée au prochain tour ; une page
        // lue sans image ne l'est plus.
        if let Ok(found) = result {
            if known.len() >= MAX_REMEMBERED_PAGES {
                known.clear();
            }
            known.insert(items[index].link.clone(), found.clone());
            items[index].image_src = found;
        }
    }
}

// ------------------------------------------------------------------- relais

fn image_id(url: &str) -> String {
    hex::encode(&Sha256::digest(url.as_bytes())[..16])
}

fn allowed() -> &'static Mutex<HashMap<String, String>> {
    static ALLOWED: LazyLock<Mutex<HashMap<String, String>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    &ALLOWED
}

/// Ouvre le relais aux images des articles et leur donne leur adresse locale.
pub async fn publish(items: &mut [NewsItem]) {
    let mut allowed = allowed().lock().await;
    if allowed.len() + items.len() > MAX_ALLOWED {
        allowed.clear();
    }
    for item in items.iter_mut() {
        item.image = item.image_src.as_ref().map(|src| {
            let id = image_id(src);
            allowed.insert(id.clone(), src.clone());
            format!("/api/wall/news/image/{id}")
        });
    }
}

/// Type d'image reconnu à sa signature ; SVG et le reste refusés.
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.len() >= 12
        && &bytes[4..8] == b"ftyp"
        && matches!(&bytes[8..12], b"avif" | b"avis")
    {
        Some("image/avif")
    } else {
        None
    }
}

struct Stored {
    id: String,
    bytes: Bytes,
    mime: &'static str,
}

/// Les images déjà relayées, les plus anciennes sortant les premières.
fn stored() -> &'static Mutex<VecDeque<Stored>> {
    static STORED: LazyLock<Mutex<VecDeque<Stored>>> =
        LazyLock::new(|| Mutex::new(VecDeque::new()));
    &STORED
}

fn reply(bytes: Bytes, mime: &'static str) -> Response {
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=86400"));
    response
}

async fn fetch_image(url: &str) -> Result<(Bytes, &'static str), String> {
    let url = feed_url(url).ok_or("not an http(s) address")?;
    guard::vet_url(&url, false, Duration::from_secs(5)).await.map_err(|e| e.to_string())?;
    let client = client()?;
    let mut response = client
        .get(url)
        .header(header::ACCEPT, "image/avif,image/webp,image/png,image/jpeg,image/gif;q=0.9")
        .send()
        .await
        .map_err(describe)?;
    if !response.status().is_success() {
        return Err(format!("answered {}", response.status().as_u16()));
    }
    let announced = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !announced.starts_with("image/") || announced.contains("svg") {
        return Err("not an image".into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
        if body.len() + chunk.len() > MAX_IMAGE_BYTES {
            return Err("the image is larger than 6 MB".into());
        }
        body.extend_from_slice(&chunk);
    }
    let mime = sniff(&body).ok_or("not a PNG, JPEG, GIF, WebP or AVIF image")?;
    Ok((Bytes::from(body), mime))
}

/// `GET /api/wall/news/image/{id}` — l'image d'un article publié sur le mur.
async fn image(Identity(_): Identity, Path(id): Path<String>) -> ApiResult<Response> {
    let not_found = || ApiError::NotFound("No such image on the wall.".into());
    if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(not_found());
    }
    if let Some(hit) = stored().lock().await.iter().find(|s| s.id == id) {
        return Ok(reply(hit.bytes.clone(), hit.mime));
    }
    let Some(url) = allowed().lock().await.get(&id).cloned() else { return Err(not_found()) };
    let (bytes, mime) = fetch_image(&url).await.map_err(ApiError::BadGateway)?;
    let mut cache = stored().lock().await;
    if !cache.iter().any(|s| s.id == id) {
        cache.push_back(Stored { id, bytes: bytes.clone(), mime });
        let mut total: usize = cache.iter().map(|s| s.bytes.len()).sum();
        while total > MAX_CACHED_BYTES {
            let Some(old) = cache.pop_front() else { break };
            total -= old.bytes.len();
        }
    }
    Ok(reply(bytes, mime))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_item_image(xml: &str) -> Option<String> {
        let doc = roxmltree::Document::parse(xml).unwrap();
        let item = doc
            .descendants()
            .find(|n| n.is_element() && matches!(n.tag_name().name(), "item" | "entry"))
            .unwrap();
        item_image(item, "https://ex.fr/articles/a.html")
    }

    #[test]
    fn media_content_picks_the_widest_image() {
        // Comme Korben : `media:content medium="image"` avec largeur.
        let xml = r#"<rss xmlns:media="http://search.yahoo.com/mrss/"><channel><item>
            <title>T</title><link>https://ex.fr/a</link>
            <media:content url="https://ex.fr/small.jpg" medium="image" width="320"/>
            <media:content url="https://ex.fr/video.mp4" medium="video" width="4000"/>
            <media:content url="https://ex.fr/big.jpg" medium="image" width="1310"/>
            <media:thumbnail url="https://ex.fr/thumb.jpg"/>
            </item></channel></rss>"#;
        assert_eq!(first_item_image(xml).as_deref(), Some("https://ex.fr/big.jpg"));
    }

    #[test]
    fn media_group_and_thumbnail() {
        let xml = r#"<rss xmlns:media="http://search.yahoo.com/mrss/"><channel><item>
            <title>T</title><media:group><media:thumbnail url="/t.png"/></media:group>
            </item></channel></rss>"#;
        assert_eq!(first_item_image(xml).as_deref(), Some("https://ex.fr/t.png"));
    }

    #[test]
    fn enclosure_like_bfm() {
        // BFM annonce `image/jpg` (sic) dans une pièce jointe.
        let xml = r#"<rss><channel><item><title>T</title>
            <enclosure url="https://cdn.ex.fr/0x0:2048x1152/800x0/images/a.jpg" length="0" type="image/jpg"/>
            </item></channel></rss>"#;
        assert_eq!(
            first_item_image(xml).as_deref(),
            Some("https://cdn.ex.fr/0x0:2048x1152/800x0/images/a.jpg")
        );
        let audio = r#"<rss><channel><item><title>T</title>
            <enclosure url="https://ex.fr/a.mp3" type="audio/mpeg"/></item></channel></rss>"#;
        assert_eq!(first_item_image(audio), None);
    }

    #[test]
    fn atom_enclosure_link() {
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><title>T</title>
            <link rel="alternate" href="https://ex.fr/a"/>
            <link rel="enclosure" type="image/png" href="https://ex.fr/i.png"/></entry></feed>"#;
        assert_eq!(first_item_image(xml).as_deref(), Some("https://ex.fr/i.png"));
    }

    #[test]
    fn first_img_of_the_content_with_entities() {
        let xml = r#"<rss xmlns:content="http://purl.org/rss/1.0/modules/content/"><channel><item>
            <title>T</title>
            <description>Pas d'image ici</description>
            <content:encoded><![CDATA[<p>Texte</p><img class="x" src="/img/a.webp?w=800&amp;h=400" alt=""><img src="b.jpg">]]></content:encoded>
            </item></channel></rss>"#;
        assert_eq!(first_item_image(xml).as_deref(), Some("https://ex.fr/img/a.webp?w=800&h=400"));
        let escaped = r#"<rss><channel><item><title>T</title>
            <description>&lt;img src="https://ex.fr/c.png"&gt; texte</description></item></channel></rss>"#;
        assert_eq!(first_item_image(escaped).as_deref(), Some("https://ex.fr/c.png"));
    }

    #[test]
    fn no_image_and_unsafe_schemes() {
        let xml = r#"<rss><channel><item><title>T</title>
            <description>&lt;img src="javascript:alert(1)"&gt; &lt;img src="data:image/png;base64,AA"&gt;</description>
            </item></channel></rss>"#;
        assert_eq!(first_item_image(xml), None);
    }

    #[test]
    fn og_image_from_a_page() {
        let html = r#"<html><head>
            <meta name="twitter:image" content="https://ex.fr/tw.jpg">
            <meta content='/og.jpg' property='og:image' />
            </head><body><img src="/body.jpg"></body></html>"#;
        assert_eq!(page_image(html, "https://ex.fr/a").as_deref(), Some("https://ex.fr/og.jpg"));
        assert_eq!(page_image("<html><head></head></html>", "https://ex.fr/a"), None);
        let ftp = r#"<meta property="og:image" content="ftp://ex.fr/a.jpg">"#;
        assert_eq!(page_image(ftp, "https://ex.fr/a"), None);
    }

    #[test]
    fn sniffs_only_raster_images() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(sniff(b"GIF89a.."), Some("image/gif"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"\0\0\0\x1cftypavif\0\0"), Some("image/avif"));
        assert_eq!(sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script/></svg>"), None);
        assert_eq!(sniff(b"<html>"), None);
    }

    #[tokio::test]
    async fn published_images_get_a_local_address() {
        let mut items = vec![NewsItem {
            title: "t".into(),
            link: "https://ex.fr/a".into(),
            source: "s".into(),
            published: None,
            image: None,
            image_src: Some("https://cdn.ex.fr/a.jpg".into()),
        }];
        publish(&mut items).await;
        let local = items[0].image.clone().unwrap();
        let id = local.strip_prefix("/api/wall/news/image/").unwrap();
        assert_eq!(id.len(), 32);
        assert_eq!(
            allowed().lock().await.get(id).map(String::as_str),
            Some("https://cdn.ex.fr/a.jpg")
        );
        // Le JSON ne porte jamais l'adresse d'origine.
        let json = serde_json::to_string(&items[0]).unwrap();
        assert!(!json.contains("cdn.ex.fr"), "{json}");
    }
}
