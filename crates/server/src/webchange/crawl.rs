//! Lecture des pages : la page de départ seule, ou tout un site.
//!
//! En mode site, les pages viennent du plan de site (`sitemap.xml`, ou ceux
//! qu'annonce `robots.txt`) quand il en liste dans le périmètre, sinon d'un
//! parcours en largeur des liens de même origine. Le parcours est poli : deux
//! requêtes à la fois au plus, une pause entre deux lots, `robots.txt`
//! respecté, et un plafond de pages ([`Options::max_pages`]) comme de durée.
//!
//! Le client HTTP est celui des sondes de disponibilité, garde-fou compris :
//! la boucle locale et le lien local (le service de métadonnées d'un cloud, le
//! VictoriaMetrics embarqué) ne sont joignables qu'avec `allow_private_targets`.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dumbmonit_collectors::uptime::guard;
use dumbmonit_proto::ProbeError;
use reqwest::Url;
use reqwest::header::CONTENT_TYPE;

use super::html;
use super::options::{Mode, Options};
use super::robots::Robots;
use super::sitemap::{self, Sitemap};

pub const USER_AGENT: &str = concat!("DumbMonit/", env!("CARGO_PKG_VERSION"));

/// Délai d'une requête, corps compris.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Corps lu au plus pour une page : au-delà, la fin est ignorée.
const MAX_PAGE_BYTES: usize = 5 * 1024 * 1024;
/// Corps lu au plus pour un plan de site ou un `robots.txt`.
const MAX_SITEMAP_BYTES: usize = 10 * 1024 * 1024;
/// Requêtes simultanées vers le site.
const CONCURRENCY: usize = 2;
/// Pause entre deux lots de requêtes.
const POLITE_DELAY: Duration = Duration::from_millis(500);
/// Plans de site lus au plus derrière un index.
const MAX_CHILD_SITEMAPS: usize = 10;
/// Redirections suivies au plus.
const MAX_REDIRECTS: usize = 5;
/// Durée maximale d'un parcours. Au-delà, ce qui est lu est gardé, et le
/// parcours compte comme incomplet : aucune page n'est déclarée disparue.
const CRAWL_BUDGET: Duration = Duration::from_secs(10 * 60);

/// Ce qu'une page a donné.
#[derive(Debug, Clone)]
pub struct PageResult {
    pub url: Url,
    /// Code HTTP, absent si la requête n'a pas abouti.
    pub status: Option<u16>,
    /// Ce qui a empêché de lire la page.
    pub error: Option<String>,
    pub title: Option<String>,
    /// Texte de la page, une ligne par bloc ; `None` en cas d'erreur.
    pub lines: Option<Vec<String>>,
    /// 404 ou 410 : la page n'existe plus.
    pub gone: bool,
}

/// Le résultat d'un parcours.
#[derive(Debug, Clone)]
pub struct Crawl {
    /// La page de départ d'abord, puis les autres dans l'ordre de lecture.
    pub pages: Vec<PageResult>,
    /// Vrai si le parcours a tout vu : ni plafond atteint, ni délai dépassé,
    /// ni page de départ en échec. Seul un parcours complet peut conclure
    /// qu'une page a disparu du site.
    pub complete: bool,
}

/// Lit la page de départ, et le reste du site en mode site.
///
/// Une page de départ injoignable (réseau, délai, erreur 5xx) rend une
/// erreur qui signifie « hors service » ; une page de départ en 4xx est
/// rendue comme résultat, à l'appelant d'en juger.
pub async fn crawl(options: &Options) -> Result<Crawl, ProbeError> {
    let started = Instant::now();
    let client = Client::new(options.insecure_tls, options.allow_private)?;
    let start = options.scope.start().clone();
    guard::vet_url(&start, options.allow_private, Duration::from_secs(5)).await?;

    let first = match client.get(&start, MAX_PAGE_BYTES, true).await {
        Ok(response) => response,
        Err(FetchError::Timeout) => return Err(ProbeError::Timeout(REQUEST_TIMEOUT)),
        Err(FetchError::Network(message)) => return Err(ProbeError::Unreachable(message)),
    };
    if first.status >= 500 {
        return Err(ProbeError::Unreachable(format!(
            "the start page answers HTTP {}",
            first.status
        )));
    }
    let (start_result, start_links) = read_page(&start, first, options.mode == Mode::Page);
    let start_ok = start_result.error.is_none();
    let mut pages = vec![start_result];
    if options.mode == Mode::Page {
        return Ok(Crawl { pages, complete: start_ok });
    }
    if !start_ok {
        return Ok(Crawl { pages, complete: false });
    }

    let robots = client.robots(&start).await;
    let mut seen: HashSet<Url> = HashSet::from([start.clone()]);
    let mut queue: VecDeque<Url> = VecDeque::new();
    let from_sitemap = client.sitemap_urls(options, &robots).await;
    let follow_links = from_sitemap.is_empty();
    let candidates = if follow_links { start_links } else { from_sitemap };
    enqueue(&mut queue, &mut seen, options, &robots, candidates);

    let mut complete = true;
    while !queue.is_empty() {
        if pages.len() >= options.max_pages || started.elapsed() > CRAWL_BUDGET {
            complete = false;
            break;
        }
        let room = (options.max_pages - pages.len()).min(CONCURRENCY);
        let batch: Vec<Url> = (0..room).filter_map(|_| queue.pop_front()).collect();
        tokio::time::sleep(POLITE_DELAY).await;
        let fetched = futures::future::join_all(batch.into_iter().map(|url| {
            let client = &client;
            async move {
                match client.get(&url, MAX_PAGE_BYTES, false).await {
                    Ok(response) if response.skipped => None,
                    Ok(response) => Some(read_page(&url, response, false)),
                    Err(error) => Some((failed(&url, error), Vec::new())),
                }
            }
        }))
        .await;
        for (result, links) in fetched.into_iter().flatten() {
            if follow_links {
                enqueue(&mut queue, &mut seen, options, &robots, links);
            }
            pages.push(result);
        }
    }
    Ok(Crawl { pages, complete })
}

/// Ajoute à la file ce qui est dans le périmètre, permis par `robots.txt` et
/// pas encore vu.
fn enqueue(
    queue: &mut VecDeque<Url>,
    seen: &mut HashSet<Url>,
    options: &Options,
    robots: &Robots,
    urls: Vec<Url>,
) {
    for url in urls.into_iter().filter_map(|url| options.scope.contains(url)) {
        if seen.contains(&url) || !robots.allows(&path_and_query(&url)) {
            continue;
        }
        seen.insert(url.clone());
        queue.push_back(url);
    }
}

fn path_and_query(url: &Url) -> String {
    match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_string(),
    }
}

/// Une page dont la requête n'a pas abouti.
fn failed(url: &Url, error: FetchError) -> PageResult {
    PageResult {
        url: url.clone(),
        status: None,
        error: Some(match error {
            FetchError::Timeout => format!("no answer within {}s", REQUEST_TIMEOUT.as_secs()),
            FetchError::Network(message) => message,
        }),
        title: None,
        lines: None,
        gone: false,
    }
}

/// Interprète une réponse : texte et titre d'une page HTML, liens à suivre.
///
/// `any_text` : une page seule peut être un fichier texte ou JSON, qu'on
/// compare ligne à ligne ; dans un parcours, on ne garde que le HTML.
fn read_page(url: &Url, response: Response, any_text: bool) -> (PageResult, Vec<Url>) {
    let mut result = PageResult {
        url: url.clone(),
        status: Some(response.status),
        error: None,
        title: None,
        lines: None,
        gone: matches!(response.status, 404 | 410),
    };
    if response.status >= 400 {
        result.error = Some(format!("HTTP {}", response.status));
        return (result, Vec::new());
    }
    if is_html(&response.content_type) {
        let page = html::parse(&response.body);
        let base = page
            .base
            .as_deref()
            .and_then(|base| response.final_url.join(base).ok())
            .unwrap_or_else(|| response.final_url.clone());
        let links = page.links.iter().filter_map(|href| base.join(href.trim()).ok()).collect();
        result.title = page.title;
        result.lines = Some(page.lines);
        return (result, links);
    }
    if any_text && is_text(&response.content_type) {
        result.lines = Some(
            response.body.lines().map(html::collapse).filter(|line| !line.is_empty()).collect(),
        );
        return (result, Vec::new());
    }
    result.error = Some(format!("not a web page (content type {})", response.content_type));
    (result, Vec::new())
}

/// Liste vide ou absente : on suppose du HTML, comme un navigateur.
fn is_html(content_type: &str) -> bool {
    let essence = essence(content_type);
    essence.is_empty() || essence == "text/html" || essence == "application/xhtml+xml"
}

fn is_text(content_type: &str) -> bool {
    let essence = essence(content_type);
    essence.starts_with("text/")
        || matches!(essence.as_str(), "application/json" | "application/xml")
        || essence.ends_with("+json")
        || essence.ends_with("+xml")
}

fn essence(content_type: &str) -> String {
    content_type.split(';').next().unwrap_or_default().trim().to_ascii_lowercase()
}

struct Client {
    http: reqwest::Client,
}

struct Response {
    status: u16,
    content_type: String,
    body: String,
    final_url: Url,
    /// Vrai si le corps n'a pas été lu : ni HTML ni texte, dans un parcours.
    skipped: bool,
}

enum FetchError {
    Timeout,
    Network(String),
}

impl Client {
    fn new(insecure_tls: bool, allow_private: bool) -> Result<Self, ProbeError> {
        // Les noms sont vérifiés par le résolveur au moment de la connexion ;
        // une redirection vers une adresse littérale ne passe pas par lui, elle
        // est vérifiée ici.
        let redirects = reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                return attempt.error(format!("more than {MAX_REDIRECTS} redirects"));
            }
            let forbidden = !allow_private
                && attempt
                    .url()
                    .host_str()
                    .map(|host| host.trim_matches(|c| c == '[' || c == ']'))
                    .and_then(|host| host.parse().ok())
                    .is_some_and(guard::is_forbidden);
            if forbidden {
                let to = attempt.url().to_string();
                attempt.error(format!("redirect to {to} refused (loopback or link-local)"))
            } else {
                attempt.follow()
            }
        });
        let http = reqwest::Client::builder()
            .dns_resolver(Arc::new(guard::GuardedResolver { allow_private }))
            .redirect(redirects)
            .danger_accept_invalid_certs(insecure_tls)
            .connect_timeout(dumbmonit_collectors::http::CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| ProbeError::Config(format!("HTTP client unavailable: {error}")))?;
        Ok(Self { http })
    }

    /// `any_type` : lire le corps quel que soit son type (page de départ).
    async fn get(&self, url: &Url, max: usize, any_type: bool) -> Result<Response, FetchError> {
        let request = self
            .http
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml;q=0.9,*/*;q=0.5");
        let mut response = request.send().await.map_err(classify)?;
        let status = response.status().as_u16();
        let final_url = response.url().clone();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        if !any_type && status < 400 && !is_html(&content_type) {
            return Ok(Response {
                status,
                content_type,
                body: String::new(),
                final_url,
                skipped: true,
            });
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(classify)? {
            body.extend_from_slice(&chunk);
            if body.len() >= max {
                body.truncate(max);
                break;
            }
        }
        Ok(Response {
            status,
            content_type,
            body: String::from_utf8_lossy(&body).into_owned(),
            final_url,
            skipped: false,
        })
    }

    /// `robots.txt` de l'origine ; tout est permis s'il manque ou ne se lit pas.
    async fn robots(&self, start: &Url) -> Robots {
        let Ok(url) = start.join("/robots.txt") else { return Robots::default() };
        match self.get(&url, MAX_SITEMAP_BYTES, true).await {
            Ok(response) if response.status == 200 => Robots::parse(&response.body),
            _ => Robots::default(),
        }
    }

    /// Les pages du périmètre que listent les plans de site, dans leur ordre.
    async fn sitemap_urls(&self, options: &Options, robots: &Robots) -> Vec<Url> {
        let start = options.scope.start();
        let mut candidates: Vec<Url> = robots
            .sitemaps
            .iter()
            .filter_map(|raw| Url::parse(raw).ok())
            .filter(|url| url.origin() == start.origin())
            .take(3)
            .collect();
        if candidates.is_empty() {
            candidates.extend(start.join("/sitemap.xml").ok());
        }

        let mut locations = Vec::new();
        for candidate in candidates {
            match self.sitemap(&candidate).await {
                Some(Sitemap::Urls(urls)) => locations.extend(urls),
                Some(Sitemap::Index(children)) => {
                    let children = children
                        .iter()
                        .filter_map(|raw| Url::parse(raw).ok())
                        .filter(|url| url.origin() == start.origin())
                        .take(MAX_CHILD_SITEMAPS);
                    for child in children {
                        // Un index d'index n'est pas suivi : un niveau suffit
                        // à tous les plans de site raisonnables.
                        if let Some(Sitemap::Urls(urls)) = self.sitemap(&child).await {
                            locations.extend(urls);
                        }
                    }
                }
                None => {}
            }
        }

        let mut seen = HashSet::new();
        locations
            .iter()
            .filter_map(|raw| Url::parse(raw).ok())
            .filter_map(|url| options.scope.contains(url))
            .filter(|url| url != start && seen.insert(url.clone()))
            .collect()
    }

    async fn sitemap(&self, url: &Url) -> Option<Sitemap> {
        let response = self.get(url, MAX_SITEMAP_BYTES, true).await.ok()?;
        if response.status != 200 {
            return None;
        }
        sitemap::parse(&response.body)
    }
}

/// Message lisible d'une erreur `reqwest`, causes comprises : « error sending
/// request » seul ne dit rien.
fn classify(error: reqwest::Error) -> FetchError {
    if error.is_timeout() {
        return FetchError::Timeout;
    }
    let mut message = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        let text = cause.to_string();
        if !message.contains(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        source = cause.source();
    }
    FetchError::Network(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_de_contenu() {
        assert!(is_html("text/html; charset=utf-8"));
        assert!(is_html("Application/XHTML+XML"));
        assert!(is_html(""));
        assert!(!is_html("application/pdf"));
        assert!(is_text("text/plain"));
        assert!(is_text("application/ld+json"));
        assert!(!is_text("image/png"));
    }

    fn response(status: u16, content_type: &str, body: &str) -> Response {
        Response {
            status,
            content_type: content_type.into(),
            body: body.into(),
            final_url: Url::parse("https://ex.fr/docs/page").unwrap(),
            skipped: false,
        }
    }

    #[test]
    fn une_page_html_donne_son_texte_et_ses_liens_resolus() {
        let url = Url::parse("https://ex.fr/docs/page").unwrap();
        let (result, links) = read_page(
            &url,
            response(200, "text/html", "<title>T</title><p>Un</p><a href='autre'>x</a>"),
            false,
        );
        assert_eq!(result.title.as_deref(), Some("T"));
        assert_eq!(result.lines.as_deref(), Some(&["Un".to_string(), "x".to_string()][..]));
        assert_eq!(links, [Url::parse("https://ex.fr/docs/autre").unwrap()]);
    }

    #[test]
    fn erreurs_http_et_contenus_non_textuels() {
        let url = Url::parse("https://ex.fr/docs/page").unwrap();
        let (gone, _) = read_page(&url, response(404, "text/html", "absent"), false);
        assert!(gone.gone);
        assert_eq!(gone.error.as_deref(), Some("HTTP 404"));
        let (forbidden, _) = read_page(&url, response(403, "text/html", ""), false);
        assert!(!forbidden.gone);
        let (json, _) = read_page(&url, response(200, "application/json", "{\n \"a\": 1\n}"), true);
        assert_eq!(json.lines.map(|l| l.len()), Some(3));
        let (pdf, _) = read_page(&url, response(200, "application/pdf", "%PDF"), true);
        assert!(pdf.error.unwrap().contains("application/pdf"));
    }
}
