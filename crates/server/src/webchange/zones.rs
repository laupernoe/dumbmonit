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
        split_outside_quotes(input)?
            .into_iter()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(Selector::parse)
            .collect()
    }

    fn parse(input: &str) -> Result<Selector, String> {
        let invalid =
            || format!("\"{input}\" is not a simple selector (tag, #id, .class, [attr=value])");
        let mut selector =
            Selector { tag: None, id: None, classes: Vec::new(), attributes: Vec::new() };
        let mut rest = input;
        let name_end = rest.find(['#', '.', '[']).unwrap_or(rest.len());
        if name_end > 0 {
            // Le lecteur rend les noms de balises en minuscules : `DIV` doit
            // trouver `<div>`.
            selector.tag = Some(name(&rest[..name_end]).ok_or_else(invalid)?.to_ascii_lowercase());
        }
        rest = &rest[name_end..];
        while let Some(first) = rest.chars().next() {
            // Avancer d'un caractère, pas d'un octet : `é` en prend deux.
            rest = &rest[first.len_utf8()..];
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
                    let (key, value, after) = attribute(rest).ok_or_else(invalid)?;
                    let key = name(key).ok_or_else(invalid)?.to_ascii_lowercase();
                    selector.attributes.push((key, value));
                    rest = after;
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

/// Coupe `input` aux virgules et aux fins de ligne, sauf entre guillemets :
/// `[title="a, b"]` reste un seul sélecteur.
fn split_outside_quotes(input: &str) -> Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let mut quote = None;
    let mut start = 0;
    for (at, c) in input.char_indices() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, ',' | '\n') => {
                parts.push(&input[start..at]);
                start = at + 1;
            }
            (None, _) => {}
        }
    }
    if quote.is_some() {
        return Err(format!("\"{}\" has an unclosed quote", input.trim()));
    }
    parts.push(&input[start..]);
    Ok(parts)
}

/// Lit `nom]`, `nom=valeur]` ou `nom="valeur"]` (le `[` déjà lu) : rend le
/// nom, la valeur attendue et ce qui suit le `]`.
fn attribute(input: &str) -> Option<(&str, Option<String>, &str)> {
    let key_end = input.find(['=', ']'])?;
    let key = &input[..key_end];
    let rest = &input[key_end..];
    if let Some(after) = rest.strip_prefix(']') {
        return Some((key, None, after));
    }
    let rest = &rest[1..];
    let (value, rest) = match rest.chars().next()? {
        quote @ ('"' | '\'') => {
            let inner = &rest[1..];
            let end = inner.find(quote)?;
            (&inner[..end], &inner[end + 1..])
        }
        _ => {
            let end = rest.find(']')?;
            let value = &rest[..end];
            let plain = !value.is_empty()
                && !value.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '['));
            if !plain {
                return None;
            }
            (value, &rest[end..])
        }
    };
    let after = rest.strip_prefix(']')?;
    Some((key, Some(value.to_string()), after))
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
        for input in [
            "main p",
            "ul > li",
            "a:hover",
            "*",
            "div.",
            "#",
            "[x",
            "di$v",
            "[lang=fr]é",
            "[a=b c]",
            "[a=\"x]",
            "[a=\"x\"y]",
            "[=x]",
            "div.x:hover",
            "[a=\"x, y]",
        ] {
            assert!(Selector::parse_list(input).is_err(), "{input}");
        }
    }

    #[test]
    fn les_caracteres_non_ascii_ne_font_pas_paniquer() {
        let list = Selector::parse_list("#été, .café[lang=fr]").unwrap();
        assert!(list[0].matches("div", &attrs(&[("id", "été")])));
        assert!(list[1].matches("p", &attrs(&[("class", "café"), ("lang", "fr")])));
        assert!(Selector::parse_list("[lang=fr]é").is_err());
        assert!(Selector::parse_list("é[").is_err());
    }

    #[test]
    fn les_noms_de_balises_et_dattributs_ignorent_la_casse() {
        let list = Selector::parse_list("DIV[Data-Ad], Main").unwrap();
        assert!(list[0].matches("div", &attrs(&[("data-ad", "")])));
        assert!(list[1].matches("main", &[]));
        // L'id et les classes gardent leur casse.
        assert!(!Selector::parse_list("#Top").unwrap()[0].matches("div", &attrs(&[("id", "top")])));
    }

    #[test]
    fn une_valeur_entre_guillemets_peut_contenir_virgules_et_espaces() {
        let list =
            Selector::parse_list("[aria-label=\"Cookies, consent\"], [title='a ] b'], .x").unwrap();
        assert_eq!(list.len(), 3);
        assert!(list[0].matches("div", &attrs(&[("aria-label", "Cookies, consent")])));
        assert!(list[1].matches("div", &attrs(&[("title", "a ] b")])));
        assert!(list[2].matches("div", &attrs(&[("class", "x")])));
        assert!(
            Selector::parse_list("[data-x=\"\"]").unwrap()[0]
                .matches("div", &attrs(&[("data-x", "")]))
        );
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
