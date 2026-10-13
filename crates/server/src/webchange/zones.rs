//! Les zones d'une page que la comparaison lit.
//!
//! Le menu, l'en-tête et le pied de page sont les mêmes sur toutes les pages
//! d'un site : un lien ajouté au menu ferait signaler chaque page comme
//! modifiée, en une seule vérification. Ils sont donc écartés par défaut, avec
//! les mêmes règles que les rôles implicites du HTML : `nav` partout, `header`
//! et `footer` hors d'un contenu de section (`article`, `aside`, `main`,
//! `nav`, `section`), `aside` hors d'un `article` ou d'un `main`, et tout
//! élément de rôle `navigation`, `banner`, `contentinfo` ou `search`.
//!
//! L'utilisateur peut aussi désigner la zone à lire (`content_selector`, par
//! exemple `main` ou `#content`) et d'autres zones à écarter
//! (`exclude_selector`, par exemple `.cookie-banner`). Les sélecteurs sont
//! volontairement simples, sans combinateur : un nom de balise, un `#id`, des
//! `.classes` et des `[attributs]` ou `[attributs=valeur]`, combinés dans un
//! même sélecteur (`div.notice[role=alert]`) et séparés par des virgules.

/// Un sélecteur simple : tous ses éléments doivent correspondre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    /// `(nom, valeur attendue)` ; sans valeur, la seule présence suffit.
    attributes: Vec<(String, Option<String>)>,
}

impl Selector {
    /// Lit une liste de sélecteurs séparés par des virgules ; vide si `input`
    /// l'est.
    pub fn parse_list(input: &str) -> Result<Vec<Selector>, String> {
        input
            .split([',', '\n'])
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(Selector::parse)
            .collect()
    }

    fn parse(input: &str) -> Result<Selector, String> {
        let invalid =
            || format!("\"{input}\" is not a simple selector (tag, #id, .class, [attr=value])");
        if input.chars().any(|c| c.is_whitespace() || matches!(c, '>' | '+' | '~' | ':' | '*')) {
            return Err(invalid());
        }
        let mut selector =
            Selector { tag: None, id: None, classes: Vec::new(), attributes: Vec::new() };
        let mut rest = input;
        let name_end = rest.find(['#', '.', '[']).unwrap_or(rest.len());
        if name_end > 0 {
            selector.tag = Some(name(&rest[..name_end]).ok_or_else(invalid)?);
        }
        rest = &rest[name_end..];
        while let Some(first) = rest.chars().next() {
            rest = &rest[1..];
            match first {
                '#' | '.' => {
                    let end = rest.find(['#', '.', '[']).unwrap_or(rest.len());
                    let value = name(&rest[..end]).ok_or_else(invalid)?;
                    if first == '#' {
                        selector.id = Some(value);
                    } else {
                        selector.classes.push(value);
                    }
                    rest = &rest[end..];
                }
                '[' => {
                    let end = rest.find(']').ok_or_else(invalid)?;
                    let inner = &rest[..end];
                    let (key, value) = match inner.split_once('=') {
                        Some((key, value)) => {
                            let value = value.trim_matches(|c| c == '"' || c == '\'');
                            (key, Some(value.to_string()))
                        }
                        None => (inner, None),
                    };
                    selector.attributes.push((name(key).ok_or_else(invalid)?, value));
                    rest = &rest[end + 1..];
                }
                _ => return Err(invalid()),
            }
        }
        Ok(selector)
    }

    /// L'élément `tag`, d'attributs `attributes` (noms en minuscules), en
    /// fait-il partie ?
    pub fn matches(&self, tag: &str, attributes: &[(String, String)]) -> bool {
        let attribute =
            |key: &str| attributes.iter().find(|(k, _)| k == key).map(|(_, value)| value.trim());
        if self.tag.as_deref().is_some_and(|wanted| wanted != tag) {
            return false;
        }
        if let Some(id) = &self.id
            && attribute("id") != Some(id.as_str())
        {
            return false;
        }
        if !self.classes.is_empty() {
            let classes: Vec<&str> = attribute("class").unwrap_or("").split_whitespace().collect();
            if !self.classes.iter().all(|class| classes.contains(&class.as_str())) {
                return false;
            }
        }
        self.attributes.iter().all(|(key, wanted)| match (attribute(key), wanted) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(value), Some(wanted)) => value == wanted,
        })
    }
}

/// Un nom de balise, d'id, de classe ou d'attribut. Les noms de balises et
/// d'attributs sont comparés en minuscules par le lecteur ; ceux des ids et
/// des classes gardent leur casse, comme dans un navigateur.
fn name(raw: &str) -> Option<String> {
    let valid =
        !raw.is_empty() && raw.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'));
    valid.then(|| raw.to_string())
}

/// Ce que la lecture du texte garde d'une page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zones {
    /// Zones à lire ; vide : toute la page. Si aucune n'est trouvée sur une
    /// page, toute la page est lue, plutôt qu'un texte vide qui passerait pour
    /// un contenu effacé.
    pub content: Vec<Selector>,
    /// Zones à écarter, en plus du menu, de l'en-tête et du pied de page.
    pub exclude: Vec<Selector>,
    /// Écarter le menu, l'en-tête et le pied de page du site.
    pub drop_chrome: bool,
}

impl Zones {
    /// Toute la page, rien d'écarté : la lecture telle qu'avant les zones, que
    /// gardent les autres lecteurs (santé et référencement d'un site).
    pub fn everything() -> Self {
        Self { content: Vec::new(), exclude: Vec::new(), drop_chrome: false }
    }

    /// L'élément ouvert ici est-il de ceux que l'on saute ? `sectioning` :
    /// nombre de contenus de section ouverts autour de lui ; `in_article` :
    /// s'il est dans un `article` ou un `main`.
    pub fn excludes(
        &self,
        tag: &str,
        attributes: &[(String, String)],
        sectioning: usize,
        in_article: bool,
    ) -> bool {
        if self.drop_chrome && is_chrome(tag, attributes, sectioning, in_article) {
            return true;
        }
        self.exclude.iter().any(|selector| selector.matches(tag, attributes))
    }

    /// L'élément ouvert ici commence-t-il une zone à lire ?
    pub fn starts_content(&self, tag: &str, attributes: &[(String, String)]) -> bool {
        self.content.iter().any(|selector| selector.matches(tag, attributes))
    }
}

/// Rôles ARIA de l'habillage d'un site.
const CHROME_ROLES: &[&str] = &["navigation", "banner", "contentinfo", "search"];

fn is_chrome(
    tag: &str,
    attributes: &[(String, String)],
    sectioning: usize,
    in_article: bool,
) -> bool {
    let role = attributes.iter().find(|(k, _)| k == "role").map(|(_, v)| v.trim());
    if role.is_some_and(|role| CHROME_ROLES.contains(&role)) {
        return true;
    }
    match tag {
        "nav" => true,
        "header" | "footer" => sectioning == 0,
        "aside" => !in_article,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn les_selecteurs_simples_se_lisent_et_se_combinent() {
        let list =
            Selector::parse_list("main, #content,.cookie-banner\n div.notice[role=alert]").unwrap();
        assert_eq!(list.len(), 4);
        assert!(list[0].matches("main", &[]));
        assert!(!list[0].matches("div", &[]));
        assert!(list[1].matches("div", &attrs(&[("id", "content")])));
        assert!(list[2].matches("section", &attrs(&[("class", "x cookie-banner y")])));
        assert!(!list[2].matches("section", &attrs(&[("class", "cookie-banner-x")])));
        assert!(list[3].matches("div", &attrs(&[("class", "notice"), ("role", "alert")])));
        assert!(!list[3].matches("div", &attrs(&[("class", "notice"), ("role", "status")])));
        assert!(
            Selector::parse_list("[data-ad]").unwrap()[0]
                .matches("div", &attrs(&[("data-ad", "")]))
        );
        assert!(Selector::parse_list("  ").unwrap().is_empty());
    }

    #[test]
    fn les_selecteurs_composes_sont_refuses() {
        for input in ["main p", "ul > li", "a:hover", "*", "div.", "#", "[x", "di$v"] {
            assert!(Selector::parse_list(input).is_err(), "{input}");
        }
    }

    #[test]
    fn lhabillage_suit_les_roles_implicites_du_html() {
        let zones = Zones { drop_chrome: true, ..Zones::everything() };
        assert!(zones.excludes("nav", &[], 2, true));
        assert!(zones.excludes("header", &[], 0, false));
        assert!(!zones.excludes("header", &[], 1, true), "en-tête d'un article");
        assert!(zones.excludes("footer", &[], 0, false));
        assert!(zones.excludes("aside", &[], 0, false));
        assert!(!zones.excludes("aside", &[], 1, true), "encadré d'un article");
        assert!(zones.excludes("div", &attrs(&[("role", "banner")]), 0, false));
        assert!(!zones.excludes("div", &[], 0, false));
        assert!(!Zones::everything().excludes("nav", &[], 0, false));
    }
}
