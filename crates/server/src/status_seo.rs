//! Référencement d'une page de statut publique (bloc « SEO » des réglages).
//!
//! Trois choses tiennent ici :
//! - les réglages enregistrés ([`Seo`]) et leur validation ;
//! - les valeurs effectives ([`resolve`]) : chaque champ laissé vide reçoit une
//!   valeur par défaut tirée du nom de la page ;
//! - le rendu serveur ([`head_for`], [`apply`]) : les robots d'indexation et les
//!   aperçus de liens n'exécutent pas le JavaScript, les balises sont donc
//!   injectées dans le `<head>` de l'index de l'interface.
//!
//! Règle de fond : une page n'est indexable que si elle est publiée *et* que
//! l'administrateur a coché la case. Le reste (page inconnue, non publiée,
//! variantes `/embed`, `/confirm`…) répond `noindex, nofollow`, sans rien
//! révéler du titre d'une page non publiée.
//!
//! Les valeurs par défaut sont aussi calculées côté interface
//! (`web/src/lib/components/status/seo.ts`) pour l'aperçu en direct : garder les
//! deux alignés.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::db::status_pages::StatusPage;

/// Longueurs visées par les moteurs (affichage tronqué au-delà).
pub const TITLE_TARGET: usize = 60;
pub const DESCRIPTION_TARGET: usize = 155;

const MAX_TITLE: usize = 120;
const MAX_DESCRIPTION: usize = 320;
const MAX_KEYWORDS: usize = 300;
const MAX_SITE_NAME: usize = 100;
const MAX_URL: usize = 2_048;

pub const ROBOTS_INDEX: &str = "index, follow";
pub const ROBOTS_NOINDEX: &str = "noindex, nofollow";

/// Réglages enregistrés. Champ vide : valeur par défaut générée.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Seo {
    /// Autorise l'indexation (`index, follow`) ; sinon `noindex, nofollow`.
    pub index: bool,
    pub title: String,
    pub description: String,
    pub keywords: String,
    /// URL canonique absolue.
    pub canonical: String,
    /// Image de partage (Open Graph / Twitter), URL absolue.
    pub image: String,
    /// Langue du contenu (`en`, `fr`, `pt-BR`).
    pub lang: String,
    /// Nom du site ou de l'organisation.
    pub site_name: String,
}

impl Seo {
    /// Relit la colonne ; une valeur illisible vaut « tout par défaut ».
    pub fn parse(json: &str) -> Seo {
        serde_json::from_str(json).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    /// Nettoie et valide la saisie. L'erreur est un message pour l'utilisateur.
    pub fn validate(mut self) -> Result<Seo, String> {
        fn plain(label: &str, value: &mut String, max: usize) -> Result<(), String> {
            *value = value.trim().to_string();
            if value.chars().count() > max {
                return Err(format!("The SEO {label} is limited to {max} characters."));
            }
            if value.chars().any(char::is_control) {
                return Err(format!("The SEO {label} must be plain text on a single line."));
            }
            Ok(())
        }
        plain("title", &mut self.title, MAX_TITLE)?;
        plain("description", &mut self.description, MAX_DESCRIPTION)?;
        plain("keywords", &mut self.keywords, MAX_KEYWORDS)?;
        plain("site name", &mut self.site_name, MAX_SITE_NAME)?;
        plain("language", &mut self.lang, 35)?;
        self.canonical = absolute_url("canonical URL", &self.canonical)?;
        self.image = absolute_url("share image URL", &self.image)?;
        if !self.lang.is_empty() && !is_valid_lang(&self.lang) {
            return Err("The SEO language must be a language tag like en, fr or pt-BR.".into());
        }
        Ok(self)
    }
}

/// Vide, ou une URL http(s) absolue sans identifiants.
fn absolute_url(label: &str, raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    let invalid = || format!("The SEO {label} must be an absolute http:// or https:// address.");
    if raw.len() > MAX_URL || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(invalid());
    }
    let url = reqwest::Url::parse(raw).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none_or(str::is_empty)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid());
    }
    Ok(url.to_string())
}

/// `fr`, `pt-BR`, `zh-Hans` : 2 ou 3 lettres, puis des sous-étiquettes.
pub fn is_valid_lang(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let primary = parts.next().unwrap_or_default();
    (2..=3).contains(&primary.len())
        && primary.bytes().all(|b| b.is_ascii_alphabetic())
        && parts.all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

// --------------------------------------------------------------------------
// Valeurs par défaut et valeurs effectives
// --------------------------------------------------------------------------

/// Coupe à `max` caractères, avec `…` si le texte a été raccourci.
pub fn truncate(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

pub fn default_title(name: &str) -> String {
    truncate(&format!("{name} – Status and uptime"), TITLE_TARGET)
}

pub fn default_description(name: &str, page_description: &str) -> String {
    let given = page_description.trim();
    if given.is_empty() {
        truncate(
            &format!("Live status, uptime history and incident updates for {name} services."),
            DESCRIPTION_TARGET,
        )
    } else {
        truncate(given, DESCRIPTION_TARGET)
    }
}

pub fn default_keywords(name: &str) -> String {
    format!("{name}, status page, uptime, incidents, maintenance")
}

/// Adresse publique de base (sans barre finale) : le domaine de la page, sinon
/// l'origine vue par l'administrateur à l'enregistrement.
fn base_url(page: &StatusPage) -> Option<String> {
    if let Some(domain) = &page.domain {
        return Some(format!("https://{domain}"));
    }
    let origin = page.link_origin.trim().trim_end_matches('/');
    (!origin.is_empty()).then(|| origin.to_string())
}

/// Adresse sous laquelle la page est servie, si on la connaît.
pub fn page_url(page: &StatusPage) -> Option<String> {
    let base = base_url(page)?;
    Some(if page.domain.is_some() { format!("{base}/") } else { format!("{base}/s/{}", page.slug) })
}

/// Valeurs effectives, telles que publiées.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Resolved {
    /// Publiée et indexation cochée.
    pub indexable: bool,
    pub title: String,
    pub description: String,
    pub keywords: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub lang: String,
    pub site_name: String,
}

pub fn resolve(page: &StatusPage) -> Resolved {
    let seo = Seo::parse(&page.seo);
    let name = page.title.trim();
    let pick = |own: &str, fallback: String| {
        if own.trim().is_empty() { fallback } else { own.trim().to_string() }
    };
    let lang = if is_valid_lang(&seo.lang) { seo.lang.clone() } else { "en".to_string() };
    let canonical = if seo.canonical.is_empty() { page_url(page) } else { Some(seo.canonical) };
    let image = if !seo.image.is_empty() {
        Some(seo.image)
    } else if page.logo_type.is_some() {
        base_url(page).map(|base| format!("{base}/api/public/status/{}/logo", page.slug))
    } else {
        None
    };
    Resolved {
        indexable: page.published && seo.index,
        title: pick(&seo.title, default_title(name)),
        description: pick(&seo.description, default_description(name, &page.description)),
        keywords: pick(&seo.keywords, default_keywords(name)),
        canonical,
        image,
        lang,
        site_name: pick(&seo.site_name, name.to_string()),
    }
}

// --------------------------------------------------------------------------
// Rendu dans le <head>
// --------------------------------------------------------------------------

/// Échappe pour un texte ou un attribut HTML entre guillemets.
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// JSON sûr dans un `<script type="application/ld+json">` : rien ne peut y
/// refermer la balise.
fn json_for_script(value: &Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "{}".into())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

/// Ce qui s'ajoute à l'index de l'interface pour une route de page de statut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    /// Remplace `<title>` ; `None` : le titre de l'interface reste.
    pub title: Option<String>,
    /// Remplace `lang` de `<html>`.
    pub lang: Option<String>,
    /// Balises ajoutées avant `</head>`.
    pub fragment: String,
    /// Valeur de `X-Robots-Tag` (et de la balise `robots`).
    pub robots: &'static str,
    /// Retire la description générique de l'interface.
    pub replaces_description: bool,
}

/// Page ou variante qui ne doit jamais être indexée (inconnue, non publiée,
/// `/embed`, `/confirm`, `/unsubscribe`) : une seule balise, rien du contenu.
pub fn noindex_head() -> Head {
    Head {
        title: None,
        lang: None,
        fragment: format!("<meta name=\"robots\" content=\"{ROBOTS_NOINDEX}\" data-dm-seo>"),
        robots: ROBOTS_NOINDEX,
        replaces_description: false,
    }
}

/// `<head>` de la route principale d'une page (`/s/<slug>` ou la racine de son
/// domaine). `None` : page inconnue.
pub fn head_for(page: Option<&StatusPage>) -> Head {
    let Some(page) = page.filter(|page| page.published) else {
        return noindex_head();
    };
    let r = resolve(page);
    let robots = if r.indexable { ROBOTS_INDEX } else { ROBOTS_NOINDEX };
    let mut tags = Vec::new();
    let mut meta = |name: &str, content: &str| {
        tags.push(format!("<meta name=\"{name}\" content=\"{}\" data-dm-seo>", esc(content)));
    };
    meta("description", &r.description);
    if !r.keywords.is_empty() {
        meta("keywords", &r.keywords);
    }
    meta("robots", robots);
    meta("twitter:card", if r.image.is_some() { "summary_large_image" } else { "summary" });
    meta("twitter:title", &r.title);
    meta("twitter:description", &r.description);
    if let Some(image) = &r.image {
        meta("twitter:image", image);
    }
    let mut prop = |name: &str, content: &str| {
        tags.push(format!("<meta property=\"{name}\" content=\"{}\" data-dm-seo>", esc(content)));
    };
    prop("og:type", "website");
    prop("og:title", &r.title);
    prop("og:description", &r.description);
    prop("og:site_name", &r.site_name);
    if let Some(image) = &r.image {
        prop("og:image", image);
    }
    if let Some(url) = &r.canonical {
        prop("og:url", url);
    }
    // Canonique, langues alternatives et données structurées : seulement pour
    // une page qu'on invite à indexer.
    if r.indexable {
        if let Some(url) = &r.canonical {
            let url = esc(url);
            tags.push(format!("<link rel=\"canonical\" href=\"{url}\" data-dm-seo>"));
            tags.push(format!(
                "<link rel=\"alternate\" hreflang=\"{}\" href=\"{url}\" data-dm-seo>",
                esc(&r.lang)
            ));
            tags.push(format!(
                "<link rel=\"alternate\" hreflang=\"x-default\" href=\"{url}\" data-dm-seo>"
            ));
        }
        tags.push(format!(
            "<script type=\"application/ld+json\">{}</script>",
            json_for_script(&json_ld(&r, page))
        ));
    }
    Head {
        title: Some(r.title),
        lang: Some(r.lang),
        fragment: tags.join("\n"),
        robots,
        replaces_description: true,
    }
}

/// Données structurées : la page web, et l'organisation qui la publie.
fn json_ld(r: &Resolved, page: &StatusPage) -> Value {
    let mut organisation = Map::new();
    organisation.insert("@type".into(), json!("Organization"));
    organisation.insert("name".into(), json!(r.site_name));
    if !page.homepage_url.is_empty() {
        organisation.insert("url".into(), json!(page.homepage_url));
    }
    if let Some(image) = &r.image {
        organisation.insert("logo".into(), json!(image));
    }
    let mut web = Map::new();
    web.insert("@context".into(), json!("https://schema.org"));
    web.insert("@type".into(), json!("WebPage"));
    web.insert("name".into(), json!(r.title));
    web.insert("description".into(), json!(r.description));
    web.insert("inLanguage".into(), json!(r.lang));
    if let Some(url) = &r.canonical {
        web.insert("url".into(), json!(url));
    }
    if let Some(image) = &r.image {
        web.insert("image".into(), json!(image));
    }
    web.insert("publisher".into(), Value::Object(organisation));
    Value::Object(web)
}

/// Retire de `html` la première balise qui commence par `start`, jusqu'à `end`.
fn remove_span(html: &mut String, start: &str, end: &str) {
    if let Some(from) = html.find(start)
        && let Some(len) = html[from..].find(end)
    {
        html.replace_range(from..from + len + end.len(), "");
    }
}

/// Applique `head` à l'index de l'interface. Une substitution de texte : le
/// fichier est celui de notre build, et les valeurs insérées sont échappées.
pub fn apply(html: &str, head: &Head) -> String {
    let mut out = html.to_string();
    if let Some(title) = &head.title
        && let Some(from) = out.find("<title>")
        && let Some(len) = out[from..].find("</title>")
    {
        let end = from + len + "</title>".len();
        out.replace_range(from..end, &format!("<title>{}</title>", esc(title)));
    }
    if head.replaces_description {
        remove_span(&mut out, "<meta name=\"description\"", ">");
    }
    if let Some(lang) = head.lang.as_deref().filter(|lang| is_valid_lang(lang)) {
        out = out.replacen("<html lang=\"en\"", &format!("<html lang=\"{}\"", esc(lang)), 1);
    }
    out.replacen("</head>", &format!("{}</head>", head.fragment), 1)
}

// --------------------------------------------------------------------------
// robots.txt et sitemap.xml
// --------------------------------------------------------------------------

/// `robots.txt` de l'hôte principal : tout est fermé, sauf les pages indexables
/// qui n'ont pas de domaine à elles (leurs ressources comprises, pour que les
/// robots puissent exécuter l'interface).
pub fn robots_main(pages: &[StatusPage]) -> String {
    let open: Vec<&StatusPage> = pages
        .iter()
        .filter(|page| page.domain.is_none() && resolve(page).indexable)
        .collect();
    let mut out = String::from("User-agent: *\n");
    if open.is_empty() {
        out.push_str("Disallow: /\n");
        return out;
    }
    for page in &open {
        out.push_str(&format!("Allow: /s/{}$\n", page.slug));
        out.push_str(&format!("Allow: /api/public/status/{}\n", page.slug));
    }
    out.push_str("Allow: /_app/\nAllow: /favicon.svg\nDisallow: /\n");
    if let Some(base) = open.iter().find_map(|page| base_url(page)) {
        out.push_str(&format!("Sitemap: {base}/sitemap.xml\n"));
    }
    out
}

/// `robots.txt` du domaine d'une page : ouvert si elle est indexable.
pub fn robots_domain(page: Option<&StatusPage>) -> String {
    match page.filter(|page| resolve(page).indexable) {
        Some(page) => {
            let mut out = String::from("User-agent: *\nAllow: /\nDisallow: /s/\n");
            if let Some(base) = base_url(page) {
                out.push_str(&format!("Sitemap: {base}/sitemap.xml\n"));
            }
            out
        }
        None => "User-agent: *\nDisallow: /\n".to_string(),
    }
}

/// `sitemap.xml` : une entrée par page indexable donnée.
pub fn sitemap(pages: &[&StatusPage]) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for page in pages {
        if !resolve(page).indexable {
            continue;
        }
        let Some(url) = page_url(page) else { continue };
        out.push_str(&format!("  <url><loc>{}</loc>", esc(&url)));
        let day: String = page.updated_at.chars().take(10).collect();
        if day.len() == 10 && day.as_bytes()[4] == b'-' && day.as_bytes()[7] == b'-' {
            out.push_str(&format!("<lastmod>{}</lastmod>", esc(&day)));
        }
        out.push_str("</url>\n");
    }
    out.push_str("</urlset>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(seo: Seo, published: bool) -> StatusPage {
        StatusPage {
            id: 1,
            slug: "acme".into(),
            title: "Acme <Cloud>".into(),
            description: String::new(),
            published,
            theme: "auto".into(),
            show_uptime_days: 90,
            created_at: String::new(),
            updated_at: "2026-10-21T08:00:00Z".into(),
            accent: "default".into(),
            scenes: String::new(),
            scene_rotation: "visit".into(),
            simple: false,
            show_calendar: false,
            show_latency: false,
            footer_text: String::new(),
            homepage_url: "https://acme.example/".into(),
            logo_type: None,
            subscribe_channel_id: None,
            link_origin: "https://monit.example".into(),
            domain: None,
            seo: seo.to_json(),
        }
    }

    const INDEX: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\" />\
        <meta name=\"description\" content=\"DumbMonit generic\" /><title>DumbMonit</title>\
        </head><body></body></html>";

    #[test]
    fn a_page_is_noindex_by_default() {
        let head = head_for(Some(&page(Seo::default(), true)));
        assert_eq!(head.robots, ROBOTS_NOINDEX);
        assert!(head.fragment.contains("content=\"noindex, nofollow\""));
        assert!(!head.fragment.contains("rel=\"canonical\""));
        assert!(!head.fragment.contains("ld+json"));
        assert!(robots_main(&[page(Seo::default(), true)]).contains("Disallow: /\n"));
        assert!(!robots_main(&[page(Seo::default(), true)]).contains("Allow:"));
    }

    #[test]
    fn an_unpublished_page_stays_noindex_and_leaks_no_title() {
        let seo = Seo { index: true, title: "Secret roadmap".into(), ..Seo::default() };
        let head = head_for(Some(&page(seo, false)));
        assert_eq!(head.robots, ROBOTS_NOINDEX);
        assert!(head.title.is_none());
        let html = apply(INDEX, &head);
        assert!(!html.contains("Secret roadmap"));
        assert!(html.contains("<title>DumbMonit</title>"));
        assert_eq!(head_for(None), noindex_head());
    }

    #[test]
    fn an_indexable_page_gets_the_full_head() {
        let seo = Seo { index: true, ..Seo::default() };
        let html = apply(INDEX, &head_for(Some(&page(seo, true))));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("content=\"index, follow\""));
        assert!(html.contains("<link rel=\"canonical\" href=\"https://monit.example/s/acme\""));
        assert!(html.contains("hreflang=\"x-default\""));
        assert!(html.contains("property=\"og:title\""));
        assert!(html.contains("name=\"twitter:card\" content=\"summary\""));
        assert!(html.contains("application/ld+json"));
        assert!(!html.contains("DumbMonit generic"), "la description générique doit partir");
        assert_eq!(html.matches("<title>").count(), 1);
        assert_eq!(html.matches("name=\"description\"").count(), 1);
    }

    #[test]
    fn everything_is_escaped() {
        let evil = "\"><script>alert(1)</script>";
        let seo = Seo {
            index: true,
            title: evil.into(),
            description: evil.into(),
            keywords: evil.into(),
            site_name: evil.into(),
            canonical: "https://x.example/?a=\"b".into(),
            ..Seo::default()
        };
        // Même un contenu qui aurait échappé à la validation reste inerte.
        let html = apply(INDEX, &head_for(Some(&page(seo, true))));
        assert!(!html.contains("<script>alert"));
        assert!(!html.contains("\"><script"));
        assert!(html.contains("&quot;&gt;&lt;script&gt;"));
        // Le JSON-LD ne peut pas refermer sa balise.
        let ld_start = html.find("application/ld+json").unwrap();
        let ld = &html[ld_start..];
        let end = ld.find("</script>").unwrap();
        assert!(!ld[..end].contains('<'));
        assert!(ld[..end].contains("\\u003c"));
        // Le nom de la page lui-même (`<Cloud>`) est échappé dans le titre par défaut.
        let plain = page(Seo { index: true, ..Seo::default() }, true);
        let html = apply(INDEX, &head_for(Some(&plain)));
        assert!(html.contains("Acme &lt;Cloud&gt; – Status and uptime"));
    }

    #[test]
    fn own_values_win_and_defaults_fill_the_rest() {
        let seo = Seo {
            index: true,
            title: "Custom title".into(),
            lang: "fr".into(),
            image: "https://cdn.example/og.png".into(),
            ..Seo::default()
        };
        let r = resolve(&page(seo);
        assert_eq!(r.title, "Custom title");
        assert_eq!(r.lang, "fr");
        assert_eq!(r.image.as_deref(), Some("https://cdn.example/og.png"));
        assert!(r.description.starts_with("Live status, uptime history"));
        assert_eq!(r.site_name, "Acme <Cloud>");
        assert!(r.description.chars().count() <= DESCRIPTION_TARGET);
        assert!(r.title.chars().count() <= TITLE_TARGET);
    }

    #[test]
    fn a_domain_is_the_canonical_address() {
        let mut p = page(Seo { index: true, ..Seo::default() }, true);
        p.domain = Some("status.acme.example".into());
        assert_eq!(page_url(&p).as_deref(), Some("https://status.acme.example/"));
        let robots = robots_domain(Some(&p));
        assert!(robots.contains("Allow: /\n"));
        assert!(robots.contains("Sitemap: https://status.acme.example/sitemap.xml"));
        assert!(sitemap(&[&p]).contains("<loc>https://status.acme.example/</loc>"));
        // Le robots de l'hôte principal n'ouvre pas une page qui a son domaine.
        assert!(!robots_main(&[p]).contains("Allow:"));
    }

    #[test]
    fn robots_and_sitemap_follow_the_checkbox() {
        let on = page(Seo { index: true, ..Seo::default() }, true);
        let off = page(Seo::default(), true);
        let robots = robots_main(&[on.clone(), off.clone()]);
        assert!(robots.contains("Allow: /s/acme$"));
        assert!(robots.contains("Sitemap: https://monit.example/sitemap.xml"));
        let xml = sitemap(&[&on, &off]);
        assert_eq!(xml.matches("<url>").count(), 1);
        assert!(xml.contains("<lastmod>2026-10-21</lastmod>"));
        assert_eq!(robots_domain(Some(&off)), "User-agent: *\nDisallow: /\n");
        assert_eq!(robots_domain(None), "User-agent: *\nDisallow: /\n");
    }

    #[test]
    fn input_is_validated() {
        let ok = Seo { title: "  Hello ".into(), lang: "pt-BR".into(), ..Seo::default() };
        assert_eq!(ok.validate().unwrap().title, "Hello");
        let bad = |seo: Seo| seo.validate().is_err();
        assert!(bad(Seo { canonical: "javascript:alert(1)".into(), ..Seo::default() }));
        assert!(bad(Seo { image: "https://u:p@x.example/a.png".into(), ..Seo::default() }));
        assert!(bad(Seo { canonical: "/relative".into(), ..Seo::default() }));
        assert!(bad(Seo { lang: "english please".into(), ..Seo::default() }));
        assert!(bad(Seo { title: "a\nb".into(), ..Seo::default() }));
        assert!(bad(Seo { title: "x".repeat(MAX_TITLE + 1), ..Seo::default() }));
    }

    #[test]
    fn truncation_adds_an_ellipsis() {
        assert_eq!(truncate("short", 10), "short");
        let cut = truncate(&"a".repeat(80), 60);
        assert_eq!(cut.chars().count(), 60);
        assert!(cut.ends_with('…'));
    }
}
