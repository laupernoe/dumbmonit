//! Texte visible d'une page HTML, son titre et ses liens.
//!
//! Pas d'arbre DOM : un lecteur de balises suffit à ce qu'on compare. On veut
//! le texte qu'un visiteur lit, un bloc par ligne, débarrassé de ce qu'il ne
//! voit pas (`script`, `style`, `noscript`, `svg`, `template`), et les `href`
//! des liens pour parcourir un site. Un HTML mal formé ne fait jamais échouer :
//! au pire une ligne est coupée ailleurs, ce qui reste stable d'une lecture à
//! l'autre — et c'est la stabilité qui compte pour détecter un changement.

/// Éléments dont le contenu n'est jamais affiché.
const HIDDEN: &[&str] = &["script", "style", "noscript", "svg", "template", "iframe", "object"];

/// Éléments qui commencent et terminent une ligne.
const BLOCKS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "br",
    "caption",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "legend",
    "li",
    "main",
    "nav",
    "ol",
    "option",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "thead",
    "tfoot",
    "tr",
    "ul",
];

/// Cellules de tableau : séparées par une espace, la ligne du tableau restant
/// une ligne de texte.
const CELLS: &[&str] = &["td", "th"];

/// Ce qu'une page contient pour nous.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Page {
    /// Contenu de `<title>`, espaces réduites ; `None` s'il est absent ou vide.
    pub title: Option<String>,
    /// Texte visible, une ligne par bloc, sans ligne vide.
    pub lines: Vec<String>,
    /// Valeurs des `href` de `<a>` et `<area>`, entités décodées, dans l'ordre
    /// du document.
    pub links: Vec<String>,
    /// `<base href>`, s'il est déclaré.
    pub base: Option<String>,
}

/// Lit une page HTML.
pub fn parse(html: &str) -> Page {
    let mut page = Page::default();
    let mut text = TextBuilder::default();
    let mut rest = html;
    let mut pre_depth = 0usize;

    while !rest.is_empty() {
        let Some(open) = rest.find('<') else {
            text.push(&decode_entities(rest), pre_depth > 0);
            break;
        };
        if open > 0 {
            text.push(&decode_entities(&rest[..open]), pre_depth > 0);
        }
        rest = &rest[open..];

        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |end| &after[end + 3..]);
            continue;
        }
        if rest.starts_with("<!") || rest.starts_with("<?") {
            rest = rest.find('>').map_or("", |end| &rest[end + 1..]);
            continue;
        }

        let Some(tag) = Tag::read(rest) else {
            // Un `<` qui n'ouvre aucune balise : du texte, comme le ferait un
            // navigateur.
            text.push("<", pre_depth > 0);
            rest = &rest[1..];
            continue;
        };
        rest = &rest[tag.len..];
        let name = tag.name.as_str();

        if tag.closing {
            if BLOCKS.contains(&name) {
                text.break_line();
            } else if CELLS.contains(&name) {
                text.space();
            }
            if name == "pre" {
                pre_depth = pre_depth.saturating_sub(1);
            }
            continue;
        }

        if HIDDEN.contains(&name) && !tag.self_closing {
            rest = raw_content(rest, name).1;
            continue;
        }
        match name {
            "title" => {
                let (inner, after) = raw_content(rest, "title");
                if page.title.is_none() {
                    page.title = Some(collapse(&decode_entities(inner))).filter(|t| !t.is_empty());
                }
                rest = after;
                continue;
            }
            "a" | "area" => {
                if let Some(href) = tag.attribute("href") {
                    page.links.push(decode_entities(href));
                }
            }
            "base" => {
                if page.base.is_none() {
                    page.base = tag.attribute("href").map(decode_entities);
                }
            }
            "pre" => pre_depth += 1,
            _ => {}
        }
        if BLOCKS.contains(&name) {
            text.break_line();
        } else if CELLS.contains(&name) {
            text.space();
        }
    }

    page.lines = text.finish();
    page
}

/// Une balise ouvrante ou fermante, telle que lue.
struct Tag {
    name: String,
    closing: bool,
    self_closing: bool,
    attributes: Vec<(String, String)>,
    /// Octets consommés, `<` et `>` compris.
    len: usize,
}

impl Tag {
    /// Lit la balise au début de `input` (qui commence par `<`). `None` si ce
    /// n'est pas une balise : `a < b`, `<3`.
    fn read(input: &str) -> Option<Self> {
        let bytes = input.as_bytes();
        let mut i = 1;
        let closing = bytes.get(i) == Some(&b'/');
        if closing {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
            i += 1;
        }
        if i == start || !bytes[start].is_ascii_alphabetic() {
            return None;
        }
        let name = input[start..i].to_ascii_lowercase();
        let mut attributes = Vec::new();
        let mut self_closing = false;

        loop {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            match bytes.get(i) {
                None => return Some(Self { name, closing, self_closing, attributes, len: i }),
                Some(b'>') => {
                    i += 1;
                    break;
                }
                Some(b'/') => {
                    self_closing = true;
                    i += 1;
                    continue;
                }
                Some(_) => {}
            }
            let key_start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && !matches!(bytes[i], b'=' | b'>' | b'/')
            {
                i += 1;
            }
            if i == key_start {
                // Caractère inattendu (`=` isolé…) : on l'enjambe.
                i += 1;
                continue;
            }
            let key = input[key_start..i].to_ascii_lowercase();
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let mut value = String::new();
            if bytes.get(i) == Some(&b'=') {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                match bytes.get(i) {
                    Some(&quote @ (b'"' | b'\'')) => {
                        let from = i + 1;
                        let end =
                            input[from..].find(quote as char).map_or(input.len(), |e| from + e);
                        value = input[from..end].to_string();
                        i = (end + 1).min(input.len());
                    }
                    _ => {
                        let from = i;
                        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>'
                        {
                            i += 1;
                        }
                        value = input[from..i].to_string();
                    }
                }
            }
            attributes.push((key, value));
        }
        Some(Self { name, closing, self_closing, attributes, len: i })
    }

    fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes.iter().find(|(k, _)| k == key).map(|(_, v)| v.trim())
    }
}

/// Contenu brut jusqu'à `</name`, et ce qui suit la balise fermante. Sans
/// balise fermante, tout le reste du document est le contenu.
///
/// La recherche ne passe pas le reste du document en minuscules : une page
/// qui compte cent scripts le recopierait cent fois.
fn raw_content<'a>(input: &'a str, name: &str) -> (&'a str, &'a str) {
    let bytes = input.as_bytes();
    let mut from = 0;
    while let Some(at) = input[from..].find("</") {
        let at = from + at;
        let name_start = at + 2;
        let name_end = name_start + name.len();
        let matches = bytes
            .get(name_start..name_end)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name.as_bytes()))
            // `</scripts` n'est pas `</script`.
            && bytes.get(name_end).is_none_or(|b| !b.is_ascii_alphanumeric());
        if matches {
            let end = input[name_end..].find('>').map_or(input.len(), |e| name_end + e + 1);
            return (&input[..at], &input[end..]);
        }
        from = name_start;
    }
    (input, "")
}

/// Accumule le texte, une ligne par bloc.
#[derive(Default)]
struct TextBuilder {
    lines: Vec<String>,
    current: String,
}

impl TextBuilder {
    fn push(&mut self, text: &str, preformatted: bool) {
        for (index, segment) in text.split('\n').enumerate() {
            if index > 0 {
                if preformatted {
                    self.break_line();
                } else {
                    self.space();
                }
            }
            for c in segment.chars() {
                if c.is_whitespace() {
                    self.space();
                } else {
                    self.current.push(c);
                }
            }
        }
    }

    fn space(&mut self) {
        if !self.current.is_empty() && !self.current.ends_with(' ') {
            self.current.push(' ');
        }
    }

    fn break_line(&mut self) {
        let line = self.current.trim();
        if !line.is_empty() {
            self.lines.push(line.to_string());
        }
        self.current.clear();
    }

    fn finish(mut self) -> Vec<String> {
        self.break_line();
        self.lines
    }
}

/// Réduit toute suite d'espaces à une seule et rogne les bords.
pub fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Décode les entités : numériques, et les nommées que l'on rencontre
/// réellement dans du texte. Une entité inconnue reste telle quelle.
pub fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest[1..].find(|c: char| !c.is_ascii_alphanumeric() && c != '#').map(|e| e + 1);
        let (entity, consumed) = match end {
            Some(end) if rest.as_bytes().get(end) == Some(&b';') => (&rest[1..end], end + 1),
            // Sans point-virgule : les navigateurs tolèrent `&amp`, nous aussi.
            Some(end) => (&rest[1..end], end),
            None => (&rest[1..], rest.len()),
        };
        match decode_one(entity) {
            Some(c) => {
                out.push(c);
                rest = &rest[consumed..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_one(entity: &str) -> Option<char> {
    if let Some(number) = entity.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return char::from_u32(code).filter(|c| *c != '\0');
    }
    Some(match entity {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "laquo" => '«',
        "raquo" => '»',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "bull" => '•',
        "middot" => '·',
        "times" => '×',
        "deg" => '°',
        "euro" => '€',
        "pound" => '£',
        "yen" => '¥',
        "cent" => '¢',
        "sect" => '§',
        "para" => '¶',
        "eacute" => 'é',
        "Eacute" => 'É',
        "egrave" => 'è',
        "Egrave" => 'È',
        "ecirc" => 'ê',
        "euml" => 'ë',
        "agrave" => 'à',
        "Agrave" => 'À',
        "acirc" => 'â',
        "auml" => 'ä',
        "ccedil" => 'ç',
        "Ccedil" => 'Ç',
        "icirc" => 'î',
        "iuml" => 'ï',
        "ocirc" => 'ô',
        "ouml" => 'ö',
        "ucirc" => 'û',
        "ugrave" => 'ù',
        "uuml" => 'ü',
        "szlig" => 'ß',
        "ntilde" => 'ñ',
        "oelig" => 'œ',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_texte_visible_sort_un_bloc_par_ligne() {
        let page = parse(
            "<!doctype html><html><head><title> Mon   site </title>\
             <style>p { color: red }</style><script>var x = '<p>piège</p>';</script></head>\
             <body><h1>Bienvenue</h1><p>Premier   paragraphe,\n sur deux lignes.</p>\
             <ul><li>un</li><li>deux <b>gras</b></li></ul>\
             <noscript>Activez JavaScript</noscript><svg><text>logo</text></svg>\
             <!-- commentaire <p>caché</p> --><div>fin&nbsp;&amp;&#233;&#x41;</div></body></html>",
        );
        assert_eq!(page.title.as_deref(), Some("Mon site"));
        assert_eq!(
            page.lines,
            ["Bienvenue", "Premier paragraphe, sur deux lignes.", "un", "deux gras", "fin &éA",]
        );
    }

    #[test]
    fn les_liens_et_la_base_sont_releves_dans_lordre() {
        let page = parse(
            r#"<base href="/docs/"><a href="a.html">A</a> <A HREF='/b?x=1&amp;y=2'>B</A>
               <a name="ancre">sans lien</a><area href=c.html><a href = " d.html ">D</a>"#,
        );
        assert_eq!(page.base.as_deref(), Some("/docs/"));
        assert_eq!(page.links, ["a.html", "/b?x=1&y=2", "c.html", "d.html"]);
    }

    #[test]
    fn un_html_mal_forme_ne_fait_pas_echouer() {
        let page = parse("a < b et 3<4 <p>suite <div unterminated");
        assert_eq!(page.lines, ["a < b et 3<4", "suite"]);
        let page = parse("<script>jamais fermé <p>texte");
        assert!(page.lines.is_empty());
        let page = parse("<p>x</scripts><p>y");
        assert_eq!(page.lines, ["x", "y"]);
        let page = parse("<SCRIPT>a</ScRiPt><p>vu</p>");
        assert_eq!(page.lines, ["vu"]);
    }

    #[test]
    fn le_texte_preformate_garde_ses_lignes() {
        let page = parse("<pre>ligne 1\nligne   2\n\nligne 3</pre><p>a\nb</p>");
        assert_eq!(page.lines, ["ligne 1", "ligne 2", "ligne 3", "a b"]);
    }

    #[test]
    fn les_cellules_restent_sur_la_ligne_du_tableau() {
        let page = parse(
            "<table><tr><th>Nom</th><th>Prix</th></tr><tr><td>A</td><td>3 €</td></tr></table>",
        );
        assert_eq!(page.lines, ["Nom Prix", "A 3 €"]);
    }

    #[test]
    fn les_entites_inconnues_restent_telles_quelles() {
        assert_eq!(
            decode_entities("a &foo; b &amp c &#0; &#x110000;"),
            "a &foo; b & c &#0; &#x110000;"
        );
        assert_eq!(decode_entities("sans entité"), "sans entité");
        assert_eq!(decode_entities("fin &"), "fin &");
    }
}
