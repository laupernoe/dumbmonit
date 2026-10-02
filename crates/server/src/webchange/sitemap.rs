//! Lecture d'un plan de site (`sitemap.xml`, protocole sitemaps.org).
//!
//! Deux formes : une liste de pages (`<urlset><url><loc>`) ou un index qui
//! renvoie vers d'autres plans (`<sitemapindex><sitemap><loc>`). Les espaces de
//! noms sont ignorés : on ne regarde que le nom local des balises, comme le
//! font les moteurs de recherche face aux plans approximatifs.

/// Ce qu'un plan de site annonce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sitemap {
    /// Des pages.
    Urls(Vec<String>),
    /// D'autres plans de site.
    Index(Vec<String>),
}

/// Analyse un plan de site ; `None` si ce n'en est pas un.
pub fn parse(xml: &str) -> Option<Sitemap> {
    let document = roxmltree::Document::parse(xml).ok()?;
    let root = document.root_element();
    let (entry, wrap): (&str, fn(Vec<String>) -> Sitemap) = match root.tag_name().name() {
        "urlset" => ("url", Sitemap::Urls),
        "sitemapindex" => ("sitemap", Sitemap::Index),
        _ => return None,
    };
    let locations = root
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == entry)
        .filter_map(|node| {
            node.children()
                .find(|child| child.is_element() && child.tag_name().name() == "loc")
                .and_then(|loc| loc.text())
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_string)
        })
        .collect();
    Some(wrap(locations))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_liste_de_pages() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
              <url><loc> https://ex.fr/ </loc><lastmod>2026-01-01</lastmod></url>
              <url><loc>https://ex.fr/a?x=1&amp;y=2</loc></url>
              <url><lastmod>sans adresse</lastmod></url>
            </urlset>"#;
        assert_eq!(
            parse(xml),
            Some(Sitemap::Urls(vec!["https://ex.fr/".into(), "https://ex.fr/a?x=1&y=2".into()]))
        );
    }

    #[test]
    fn un_index_de_plans() {
        let xml = r#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
              <sitemap><loc>https://ex.fr/sitemap-1.xml</loc></sitemap>
              <sitemap><loc>https://ex.fr/sitemap-2.xml</loc></sitemap>
            </sitemapindex>"#;
        assert_eq!(
            parse(xml),
            Some(Sitemap::Index(vec![
                "https://ex.fr/sitemap-1.xml".into(),
                "https://ex.fr/sitemap-2.xml".into()
            ]))
        );
    }

    #[test]
    fn autre_chose_quun_plan_est_ignore() {
        assert_eq!(parse("<html><body>404</body></html>"), None);
        assert_eq!(parse("pas du XML"), None);
        assert_eq!(parse("<urlset></urlset>"), Some(Sitemap::Urls(Vec::new())));
    }
}
