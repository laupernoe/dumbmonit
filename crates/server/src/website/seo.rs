//! Ce qu'une page dit d'elle-même aux moteurs de recherche : titre,
//! description, adresse canonique, consignes d'indexation, balises Open
//! Graph, langue, titres `h1` et variantes de langue (`hreflang`).
//!
//! La lecture reprend le lecteur de balises de `webchange` : pas d'arbre DOM,
//! jamais d'échec sur un HTML mal formé.

use reqwest::Url;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::webchange::html;

/// Les métadonnées d'une page, telles que lues (espaces réduites).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Metadata {
    pub title: Option<String>,
    pub description: Option<String>,
    /// Résolue contre l'adresse finale.
    pub canonical: Option<String>,
    /// Contenu de `<meta name="robots">` et des en-têtes `X-Robots-Tag`.
    pub robots: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub lang: Option<String>,
    pub h1_count: usize,
    pub hreflang_count: usize,
    /// Langues des variantes `hreflang`, triées et sans doublon.
    pub hreflang: Vec<String>,
    /// Jetons `google-site-verification` (Search Console), triés.
    pub google_verification: Vec<String>,
    /// `og:type` (`website`, `article`, `product`…), minuscules.
    pub og_type: Option<String>,
    pub noindex: bool,
    pub nofollow: bool,
}

/// Lit les métadonnées d'une page HTML servie depuis `final_url`.
pub fn extract(final_url: &Url, headers: &HeaderMap, body: &str) -> Metadata {
    let page = html::parse(body);
    let mut meta = Metadata { title: page.title, ..Metadata::default() };
    let mut robots: Vec<String> = Vec::new();
    let base = page
        .base
        .as_deref()
        .and_then(|base| final_url.join(base).ok())
        .unwrap_or_else(|| final_url.clone());

    for element in html::elements(body) {
        match element.name.as_str() {
            "html" if meta.lang.is_none() => {
                meta.lang = element.attribute("lang").filter(|v| !v.is_empty());
            }
            "h1" => meta.h1_count += 1,
            "meta" => {
                let content = element.attribute("content").map(|v| html::collapse(&v));
                let Some(content) = content.filter(|c| !c.is_empty()) else { continue };
                let name = element.attribute("name").map(|v| v.to_ascii_lowercase());
                let property = element.attribute("property").map(|v| v.to_ascii_lowercase());
                match (name.as_deref(), property.as_deref()) {
                    (Some("description"), _) if meta.description.is_none() => {
                        meta.description = Some(content);
                    }
                    // `googlebot` précise `robots` pour Google : les deux comptent.
                    (Some("robots" | "googlebot"), _) => robots.push(content),
                    (Some("google-site-verification"), _) => {
                        meta.google_verification.push(content);
                    }
                    (_, Some("og:type")) | (Some("og:type"), _) if meta.og_type.is_none() => {
                        meta.og_type = Some(content.to_lowercase());
                    }
                    (_, Some("og:title")) | (Some("og:title"), _) if meta.og_title.is_none() => {
                        meta.og_title = Some(content);
                    }
                    (_, Some("og:description")) | (Some("og:description"), _)
                        if meta.og_description.is_none() =>
                    {
                        meta.og_description = Some(content);
                    }
                    _ => {}
                }
            }
            "link" => {
                let rel = element.attribute("rel").unwrap_or_default().to_ascii_lowercase();
                let rels: Vec<&str> = rel.split_whitespace().collect();
                if rels.contains(&"canonical") && meta.canonical.is_none() {
                    meta.canonical = element
                        .attribute("href")
                        .filter(|href| !href.is_empty())
                        .and_then(|href| base.join(&href).ok())
                        .map(|mut url| {
                            url.set_fragment(None);
                            url.to_string()
                        });
                }
                if rels.contains(&"alternate")
                    && let Some(lang) = element.attribute("hreflang")
                {
                    meta.hreflang_count += 1;
                    meta.hreflang.push(lang.trim().to_ascii_lowercase());
                }
            }
            _ => {}
        }
    }

    meta.hreflang.sort();
    meta.hreflang.dedup();
    meta.google_verification.sort();
    meta.google_verification.dedup();

    // `X-Robots-Tag: noindex` ou `X-Robots-Tag: googlebot: noindex` ; une
    // consigne réservée à un autre robot (`otherbot: noindex`) ne compte pas.
    for value in headers.get_all("x-robots-tag").iter().filter_map(|v| v.to_str().ok()) {
        let value = html::collapse(value);
        let directives = match value.split_once(':') {
            Some((agent, rest))
                if !agent.contains(',') && !agent.trim().contains(' ') && !is_directive(agent) =>
            {
                let agent = agent.trim().to_ascii_lowercase();
                if agent != "googlebot" && agent != "*" {
                    continue;
                }
                rest.trim().to_string()
            }
            _ => value,
        };
        if !directives.is_empty() {
            robots.push(directives);
        }
    }

    let tokens: Vec<String> = robots
        .iter()
        .flat_map(|r| r.split(','))
        .map(|t| t.trim().to_ascii_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    meta.noindex = tokens.iter().any(|t| t == "noindex" || t == "none");
    meta.nofollow = tokens.iter().any(|t| t == "nofollow" || t == "none");
    if !robots.is_empty() {
        meta.robots = Some(robots.join(", "));
    }
    meta
}

/// Les directives qui peuvent porter un `:` (`max-snippet:20`,
/// `unavailable_after: …`) ne sont pas des noms de robots.
fn is_directive(word: &str) -> bool {
    let word = word.trim().to_ascii_lowercase();
    matches!(
        word.as_str(),
        "max-snippet" | "max-image-preview" | "max-video-preview" | "unavailable_after"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    fn url() -> Url {
        Url::parse("https://www.ex.fr/blog/article").unwrap()
    }

    const PAGE: &str = r#"<!doctype html>
<html lang="fr-FR"><head>
  <title>  Mon   article &amp; plus </title>
  <meta name="description" content="Une   description">
  <meta property="og:title" content="OG titre">
  <meta property="og:description" content="OG desc">
  <meta property="og:type" content="Article">
  <meta name="google-site-verification" content="tok_B">
  <meta name="google-site-verification" content="tok_A">
  <link rel="canonical" href="/blog/article?utm=1#x">
  <link rel="alternate" hreflang="en" href="/en/">
  <link rel="alternate" hreflang="de" href="/de/">
  <link rel="alternate" type="application/rss+xml" href="/feed">
  <!-- <meta name="robots" content="noindex"> -->
  <script>document.write('<h1>pas un titre</h1>')</script>
</head><body><h1>Titre</h1><h1>Second</h1></body></html>"#;

    #[test]
    fn lit_les_metadonnees() {
        let meta = extract(&url(), &HeaderMap::new(), PAGE);
        assert_eq!(meta.title.as_deref(), Some("Mon article & plus"));
        assert_eq!(meta.description.as_deref(), Some("Une description"));
        assert_eq!(meta.og_title.as_deref(), Some("OG titre"));
        assert_eq!(meta.og_description.as_deref(), Some("OG desc"));
        assert_eq!(meta.canonical.as_deref(), Some("https://www.ex.fr/blog/article?utm=1"));
        assert_eq!(meta.lang.as_deref(), Some("fr-FR"));
        assert_eq!(meta.h1_count, 2, "ni le commentaire ni le script ne comptent");
        assert_eq!(meta.hreflang_count, 2);
        assert_eq!(meta.hreflang, ["de", "en"]);
        assert_eq!(meta.google_verification, ["tok_A", "tok_B"]);
        assert_eq!(meta.og_type.as_deref(), Some("article"));
        assert!(!meta.noindex && !meta.nofollow && meta.robots.is_none());
    }

    #[test]
    fn noindex_par_la_balise_meta() {
        let page = r#"<html><head><meta name="ROBOTS" content="NoIndex, follow"></head></html>"#;
        let meta = extract(&url(), &HeaderMap::new(), page);
        assert!(meta.noindex && !meta.nofollow);
        let none = r#"<meta name="googlebot" content="none">"#;
        let meta = extract(&url(), &HeaderMap::new(), none);
        assert!(meta.noindex && meta.nofollow);
    }

    #[test]
    fn noindex_par_l_en_tete() {
        let mut headers = HeaderMap::new();
        headers.append("x-robots-tag", HeaderValue::from_static("noindex, nofollow"));
        let meta = extract(&url(), &headers, "<html></html>");
        assert!(meta.noindex && meta.nofollow);
        assert_eq!(meta.robots.as_deref(), Some("noindex, nofollow"));

        let mut headers = HeaderMap::new();
        headers.append("x-robots-tag", HeaderValue::from_static("googlebot: noindex"));
        assert!(extract(&url(), &headers, "").noindex);

        let mut headers = HeaderMap::new();
        headers.append("x-robots-tag", HeaderValue::from_static("otherbot: noindex"));
        headers.append("x-robots-tag", HeaderValue::from_static("max-snippet:20"));
        assert!(!extract(&url(), &headers, "").noindex);
    }

    #[test]
    fn page_vide() {
        let meta = extract(&url(), &HeaderMap::new(), "");
        assert_eq!(meta, Metadata::default());
    }
}
