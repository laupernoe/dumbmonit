//! Reconnaître la plateforme d'un site (WordPress, Shopify, Wix…), pour
//! ranger les sites surveillés.
//!
//! Trois sources, de la plus sûre à la moins sûre : la balise
//! `<meta name="generator">`, les en-têtes propres à un hébergeur
//! (`x-shopify-stage`, `x-wix-request-id`…), puis les traces dans le balisage
//! (`/wp-content/`, `cdn.shopify.com`, `__NEXT_DATA__`). La première qui parle
//! l'emporte ; sans indice, c'est [`Platform::Other`].
//!
//! [`detect_platform`] ne fait aucun appel réseau : l'import CSV
//! (`POST /api/targets/import`) lit la page une fois avec [`identify`] et le
//! collecteur `website` réutilise la page qu'il vient de lire.

use std::time::Duration;

use dumbmonit_proto::ProbeError;
use reqwest::Url;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::webchange::html;

/// Plateforme d'un site. Sérialisée par son identifiant (`wordpress`,
/// `nextjs`…), celui de l'étiquette `platform` de `dumbmonit_website_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    WordPress,
    Shopify,
    Wix,
    Squarespace,
    Webflow,
    Ghost,
    Drupal,
    Joomla,
    PrestaShop,
    Magento,
    NextJs,
    Nuxt,
    Gatsby,
    Hugo,
    Jekyll,
    Other,
}

impl Platform {
    pub const ALL: [Platform; 16] = [
        Self::WordPress,
        Self::Shopify,
        Self::Wix,
        Self::Squarespace,
        Self::Webflow,
        Self::Ghost,
        Self::Drupal,
        Self::Joomla,
        Self::PrestaShop,
        Self::Magento,
        Self::NextJs,
        Self::Nuxt,
        Self::Gatsby,
        Self::Hugo,
        Self::Jekyll,
        Self::Other,
    ];

    /// Identifiant stable (métrique, API).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WordPress => "wordpress",
            Self::Shopify => "shopify",
            Self::Wix => "wix",
            Self::Squarespace => "squarespace",
            Self::Webflow => "webflow",
            Self::Ghost => "ghost",
            Self::Drupal => "drupal",
            Self::Joomla => "joomla",
            Self::PrestaShop => "prestashop",
            Self::Magento => "magento",
            Self::NextJs => "nextjs",
            Self::Nuxt => "nuxt",
            Self::Gatsby => "gatsby",
            Self::Hugo => "hugo",
            Self::Jekyll => "jekyll",
            Self::Other => "other",
        }
    }

    /// Nom affiché, celui du dossier `Websites/<label>` de l'import.
    pub fn label(self) -> &'static str {
        match self {
            Self::WordPress => "WordPress",
            Self::Shopify => "Shopify",
            Self::Wix => "Wix",
            Self::Squarespace => "Squarespace",
            Self::Webflow => "Webflow",
            Self::Ghost => "Ghost",
            Self::Drupal => "Drupal",
            Self::Joomla => "Joomla",
            Self::PrestaShop => "PrestaShop",
            Self::Magento => "Magento",
            Self::NextJs => "Next.js",
            Self::Nuxt => "Nuxt",
            Self::Gatsby => "Gatsby",
            Self::Hugo => "Hugo",
            Self::Jekyll => "Jekyll",
            Self::Other => "Other",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == slug)
    }
}

/// Mots du `generator`, dans l'ordre où on les cherche.
const GENERATORS: &[(&str, Platform)] = &[
    ("wordpress", Platform::WordPress),
    ("woocommerce", Platform::WordPress),
    ("shopify", Platform::Shopify),
    ("wix.com", Platform::Wix),
    ("squarespace", Platform::Squarespace),
    ("webflow", Platform::Webflow),
    ("ghost", Platform::Ghost),
    ("drupal", Platform::Drupal),
    ("joomla", Platform::Joomla),
    ("prestashop", Platform::PrestaShop),
    ("magento", Platform::Magento),
    ("next.js", Platform::NextJs),
    ("nuxt", Platform::Nuxt),
    ("gatsby", Platform::Gatsby),
    ("hugo", Platform::Hugo),
    ("jekyll", Platform::Jekyll),
];

/// Traces dans le balisage (en minuscules).
const MARKUP: &[(&str, Platform)] = &[
    ("/wp-content/", Platform::WordPress),
    ("/wp-includes/", Platform::WordPress),
    ("cdn.shopify.com", Platform::Shopify),
    ("shopify.theme", Platform::Shopify),
    ("static.wixstatic.com", Platform::Wix),
    ("static1.squarespace.com", Platform::Squarespace),
    ("assets.squarespace.com", Platform::Squarespace),
    ("data-wf-site", Platform::Webflow),
    ("assets.website-files.com", Platform::Webflow),
    ("cdn.prod.website-files.com", Platform::Webflow),
    ("ghost-portal", Platform::Ghost),
    ("/sites/default/files/", Platform::Drupal),
    ("drupal-settings-json", Platform::Drupal),
    ("/media/jui/", Platform::Joomla),
    ("/media/system/js/", Platform::Joomla),
    ("prestashop", Platform::PrestaShop),
    ("mage/cookies", Platform::Magento),
    ("__next_data__", Platform::NextJs),
    ("/_next/static/", Platform::NextJs),
    ("__nuxt", Platform::Nuxt),
    ("/_nuxt/", Platform::Nuxt),
    ("___gatsby", Platform::Gatsby),
];

/// Devine la plateforme d'un site d'après sa page d'accueil telle que servie :
/// adresse finale (après redirections), en-têtes et HTML. Ne fait aucun appel
/// réseau et n'échoue jamais : sans indice, [`Platform::Other`].
pub fn detect_platform(final_url: &Url, headers: &HeaderMap, html: &str) -> Platform {
    let header = |name: &str| {
        headers
            .get_all(name)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>()
            .join(" ")
    };

    // 1. Le générateur déclaré.
    for element in html::elements(html) {
        if element.name != "meta" {
            continue;
        }
        let is_generator =
            element.attribute("name").is_some_and(|n| n.eq_ignore_ascii_case("generator"));
        if !is_generator {
            continue;
        }
        let content = element.attribute("content").unwrap_or_default().to_ascii_lowercase();
        if let Some((_, platform)) = GENERATORS.iter().find(|(word, _)| content.contains(word)) {
            return *platform;
        }
    }

    // 2. Les en-têtes d'hébergeurs et de moteurs.
    let names: Vec<String> = headers.keys().map(|k| k.as_str().to_ascii_lowercase()).collect();
    let has_prefix = |prefix: &str| names.iter().any(|n| n.starts_with(prefix));
    if has_prefix("x-shopify-") || header("powered-by").contains("shopify") {
        return Platform::Shopify;
    }
    if has_prefix("x-wix-") {
        return Platform::Wix;
    }
    if has_prefix("x-ghost-") {
        return Platform::Ghost;
    }
    if names.iter().any(|n| n == "x-drupal-cache" || n == "x-drupal-dynamic-cache")
        || header("x-generator").contains("drupal")
    {
        return Platform::Drupal;
    }
    if header("server").contains("squarespace") {
        return Platform::Squarespace;
    }
    let powered = header("x-powered-by");
    for (word, platform) in [
        ("next.js", Platform::NextJs),
        ("nuxt", Platform::Nuxt),
        ("wp engine", Platform::WordPress),
        ("wordpress", Platform::WordPress),
        ("prestashop", Platform::PrestaShop),
        ("ghost", Platform::Ghost),
    ] {
        if powered.contains(word) {
            return platform;
        }
    }
    if header("link").contains("api.w.org") {
        return Platform::WordPress;
    }

    // 3. Les traces dans la page. On se limite au début : les marqueurs sont
    // dans l'en-tête ou les premiers scripts, et une page de plusieurs
    // mégaoctets n'a pas à être recopiée en minuscules.
    let mut end = html.len().min(512 * 1024);
    while !html.is_char_boundary(end) {
        end -= 1;
    }
    let lower = html[..end].to_ascii_lowercase();
    if let Some((_, platform)) = MARKUP.iter().find(|(mark, _)| lower.contains(mark)) {
        return *platform;
    }

    // 4. L'hôte lui-même : un site encore sur le sous-domaine de sa plateforme.
    let host = final_url.host_str().unwrap_or_default().to_ascii_lowercase();
    for (suffix, platform) in [
        (".myshopify.com", Platform::Shopify),
        (".wixsite.com", Platform::Wix),
        (".squarespace.com", Platform::Squarespace),
        (".webflow.io", Platform::Webflow),
        (".ghost.io", Platform::Ghost),
        (".wordpress.com", Platform::WordPress),
    ] {
        if host.ends_with(suffix) {
            return platform;
        }
    }
    Platform::Other
}

/// Genre d'un site, déduit de signaux simples. Moins sûr que la plateforme :
/// sans indice net, on ne range pas (`None`) et l'appelant retombe sur la
/// plateforme, voire le domaine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Ecommerce,
    Blog,
    App,
    Showcase,
}

impl Category {
    pub const ALL: [Category; 4] = [Self::Ecommerce, Self::Blog, Self::App, Self::Showcase];

    /// Identifiant stable (métrique, étiquette `category`, API).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ecommerce => "ecommerce",
            Self::Blog => "blog",
            Self::App => "app",
            Self::Showcase => "showcase",
        }
    }

    /// Nom affiché, celui du dossier `Websites/<label>` de l'import.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ecommerce => "E-commerce",
            Self::Blog => "Blog",
            Self::App => "Web app",
            Self::Showcase => "Showcase",
        }
    }

    /// Lit un identifiant ou un nom affiché, sans tenir compte de la casse
    /// (colonne `category` de l'import).
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|c| {
            c.as_str() == text
                || c.label().to_ascii_lowercase() == text
                || (*c == Self::Ecommerce && matches!(text.as_str(), "e-commerce" | "shop"))
        })
    }
}

/// Devine le genre d'un site d'après sa plateforme et sa page. `og_type` est
/// la valeur de `og:type` de la page, en minuscules. Seuls les signaux nets
/// comptent : une plateforme généraliste (WordPress, Drupal, Hugo…) sans autre
/// indice rend `None`.
pub fn categorize(
    platform: Platform,
    final_url: &Url,
    html: &str,
    og_type: Option<&str>,
) -> Option<Category> {
    let mut end = html.len().min(512 * 1024);
    while !html.is_char_boundary(end) {
        end -= 1;
    }
    let lower = html[..end].to_ascii_lowercase();
    let host = final_url.host_str().unwrap_or_default().to_ascii_lowercase();

    if matches!(platform, Platform::Shopify | Platform::PrestaShop | Platform::Magento)
        || og_type == Some("product")
        || ["woocommerce", "add_to_cart", "add-to-cart", "cdn.shopify.com"]
            .iter()
            .any(|mark| lower.contains(mark))
    {
        return Some(Category::Ecommerce);
    }
    if platform == Platform::Ghost
        || og_type == Some("article")
        || [".blogspot.com", ".substack.com", ".wordpress.com", ".medium.com"]
            .iter()
            .any(|suffix| host.ends_with(suffix))
        || host.starts_with("blog.")
    {
        return Some(Category::Blog);
    }
    if matches!(platform, Platform::NextJs | Platform::Nuxt)
        || lower.contains("<div id=\"root\"></div>")
        || lower.contains("<div id=\"app\"></div>")
    {
        return Some(Category::App);
    }
    if matches!(platform, Platform::Wix | Platform::Squarespace | Platform::Webflow) {
        return Some(Category::Showcase);
    }
    None
}

/// Ce que l'import apprend d'un site en une requête.
#[derive(Debug, Clone)]
pub struct Identity {
    /// Adresse finale, redirections suivies.
    pub final_url: Url,
    pub status: u16,
    pub platform: Platform,
    pub category: Option<Category>,
}

/// Lit `address` une fois (redirections suivies, au plus 10, garde-fou des
/// adresses privées appliqué sauf `allow_private`) et devine sa plateforme.
/// Pensé pour l'import CSV : l'appelant borne la concurrence et le délai
/// global (`tokio::time::timeout`), `timeout` ne borne que chaque requête.
pub async fn identify(
    address: &str,
    timeout: Duration,
    allow_private: bool,
) -> Result<Identity, ProbeError> {
    let start = super::options::parse_url(address)?;
    let client = reqwest::Client::builder()
        .dns_resolver(std::sync::Arc::new(dumbmonit_collectors::uptime::guard::GuardedResolver {
            allow_private,
        }))
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(dumbmonit_collectors::http::CONNECT_TIMEOUT.min(timeout))
        .timeout(timeout)
        .user_agent(super::fetch::USER_AGENT)
        .build()
        .map_err(|error| ProbeError::Config(format!("HTTP client unavailable: {error}")))?;
    let followed = super::fetch::follow(start, super::options::DEFAULT_MAX_REDIRECTS as usize, {
        let client = &client;
        move |url| async move {
            super::fetch::get(client, &url, allow_private, super::fetch::MAX_BODY_BYTES).await
        }
    })
    .await?;
    let platform =
        detect_platform(&followed.final_url, &followed.answer.headers, &followed.answer.body);
    let og_type =
        super::seo::extract(&followed.final_url, &followed.answer.headers, &followed.answer.body)
            .og_type;
    let category =
        categorize(platform, &followed.final_url, &followed.answer.body, og_type.as_deref());
    Ok(Identity {
        final_url: followed.final_url,
        status: followed.answer.status,
        platform,
        category,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};

    fn detect(url: &str, headers: &[(&str, &str)], html: &str) -> Platform {
        let mut map = HeaderMap::new();
        for (name, value) in headers {
            map.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        detect_platform(&Url::parse(url).unwrap(), &map, html)
    }

    #[test]
    fn par_le_generateur() {
        let cases = [
            (r#"<meta name="generator" content="WordPress 6.6.2">"#, Platform::WordPress),
            (
                r#"<meta name="Generator" content="Drupal 10 (https://www.drupal.org)">"#,
                Platform::Drupal,
            ),
            (
                r#"<meta name="generator" content="Joomla! - Open Source Content Management">"#,
                Platform::Joomla,
            ),
            (r#"<meta name="generator" content="Hugo 0.134.0">"#, Platform::Hugo),
            (r#"<meta name="generator" content="Jekyll v4.3.3">"#, Platform::Jekyll),
            (r#"<meta name="generator" content="Ghost 5.96">"#, Platform::Ghost),
            (r#"<meta content="Webflow" name="generator">"#, Platform::Webflow),
            (r#"<meta name="generator" content="Wix.com Website Builder">"#, Platform::Wix),
            (r#"<meta name="generator" content="Gatsby 5.13.7">"#, Platform::Gatsby),
        ];
        for (html, expected) in cases {
            assert_eq!(detect("https://ex.fr/", &[], html), expected, "{html}");
        }
    }

    #[test]
    fn par_les_en_tetes() {
        assert_eq!(
            detect("https://ex.fr/", &[("x-shopify-stage", "production")], ""),
            Platform::Shopify
        );
        assert_eq!(detect("https://ex.fr/", &[("x-wix-request-id", "1")], ""), Platform::Wix);
        assert_eq!(detect("https://ex.fr/", &[("x-powered-by", "Next.js")], ""), Platform::NextJs);
        assert_eq!(detect("https://ex.fr/", &[("x-drupal-cache", "HIT")], ""), Platform::Drupal);
        assert_eq!(
            detect("https://ex.fr/", &[("server", "Squarespace")], ""),
            Platform::Squarespace
        );
        assert_eq!(
            detect(
                "https://ex.fr/",
                &[("link", "<https://ex.fr/wp-json/>; rel=\"https://api.w.org/\"")],
                ""
            ),
            Platform::WordPress
        );
        assert_eq!(
            detect("https://ex.fr/", &[("server", "nginx"), ("x-powered-by", "PHP/8.3")], ""),
            Platform::Other
        );
    }

    #[test]
    fn par_le_balisage() {
        let cases = [
            (
                r#"<link rel="stylesheet" href="/wp-content/themes/x/style.css">"#,
                Platform::WordPress,
            ),
            (r#"<script src="//cdn.shopify.com/s/files/1/theme.js"></script>"#, Platform::Shopify),
            (r#"<html data-wf-site="64a1"><body></body></html>"#, Platform::Webflow),
            (r#"<img src="https://static1.squarespace.com/static/a.jpg">"#, Platform::Squarespace),
            (r#"<script id="__NEXT_DATA__" type="application/json">{}</script>"#, Platform::NextJs),
            (r#"<div id="__nuxt"></div>"#, Platform::Nuxt),
            (r#"<div id="___gatsby"></div>"#, Platform::Gatsby),
            (r#"<img src="/sites/default/files/logo.png">"#, Platform::Drupal),
            ("<html><body><p>Bonjour</p></body></html>", Platform::Other),
        ];
        for (html, expected) in cases {
            assert_eq!(detect("https://ex.fr/", &[], html), expected, "{html}");
        }
    }

    #[test]
    fn par_l_hote_et_ordre_des_sources() {
        assert_eq!(detect("https://boutique.myshopify.com/", &[], ""), Platform::Shopify);
        assert_eq!(detect("https://moi.wixsite.com/site", &[], ""), Platform::Wix);
        // Le générateur prime sur le reste.
        assert_eq!(
            detect(
                "https://ex.fr/",
                &[("x-powered-by", "Next.js")],
                r#"<meta name="generator" content="WordPress 6.6"><div id="__NEXT_DATA__">"#
            ),
            Platform::WordPress
        );
    }

    #[test]
    fn identifiants_et_libelles() {
        for platform in Platform::ALL {
            assert_eq!(Platform::from_slug(platform.as_str()), Some(platform));
            assert_eq!(
                serde_json::to_value(platform).unwrap(),
                serde_json::json!(platform.as_str())
            );
        }
        assert_eq!(Platform::NextJs.label(), "Next.js");
        assert_eq!(Platform::Other.label(), "Other");
    }

    #[test]
    fn genres_nets_et_incertains() {
        let cat = |platform, url: &str, html: &str, og: Option<&str>| {
            categorize(platform, &Url::parse(url).unwrap(), html, og)
        };
        let ex = "https://ex.fr/";
        assert_eq!(cat(Platform::Shopify, ex, "", None), Some(Category::Ecommerce));
        assert_eq!(
            cat(Platform::WordPress, ex, "<body class=\"woocommerce\">", None),
            Some(Category::Ecommerce)
        );
        assert_eq!(cat(Platform::Other, ex, "", Some("product")), Some(Category::Ecommerce));
        assert_eq!(cat(Platform::Ghost, ex, "", None), Some(Category::Blog));
        assert_eq!(cat(Platform::WordPress, ex, "", Some("article")), Some(Category::Blog));
        assert_eq!(cat(Platform::Other, "https://blog.ex.fr/", "", None), Some(Category::Blog));
        assert_eq!(cat(Platform::NextJs, ex, "", None), Some(Category::App));
        assert_eq!(
            cat(Platform::Other, ex, "<body><div id=\"root\"></div>", None),
            Some(Category::App)
        );
        assert_eq!(cat(Platform::Wix, ex, "", None), Some(Category::Showcase));
        for platform in [Platform::WordPress, Platform::Drupal, Platform::Hugo, Platform::Other] {
            assert_eq!(
                cat(platform, ex, "<h1>Bonjour</h1>", Some("website")),
                None,
                "{platform:?}"
            );
        }
    }

    #[test]
    fn genre_lu_depuis_un_texte() {
        assert_eq!(Category::parse(" E-Commerce "), Some(Category::Ecommerce));
        assert_eq!(Category::parse("web app"), Some(Category::App));
        assert_eq!(Category::parse("blog"), Some(Category::Blog));
        assert_eq!(Category::parse("clients"), None);
    }
}
