//! Filtres SCIM (RFC 7644 §3.4.2.2), réduits à ce qu'envoient Entra ID et Okta.
//!
//! Sont compris : l'égalité `attribut eq "valeur"` (ou `true`/`false`), les
//! chemins à filtre de valeur `emails[type eq "work"].value eq "…"`, la forme
//! `members[value eq "…"]` (un membre dont la valeur est `…`), et la conjonction
//! `and`. Noms d'attributs, opérateurs et mots-clés sont insensibles à la casse ;
//! un préfixe de schéma (`urn:ietf:params:scim:schemas:core:2.0:User:userName`)
//! est retiré. Tout le reste — `or`, `not`, parenthèses, autres opérateurs — est
//! refusé : mieux vaut un `400 invalidFilter` qu'un filtre mal compris qui
//! rendrait le mauvais compte.

/// Longueur maximale d'un filtre.
const MAX_LEN: usize = 1024;
/// Nombre maximal de comparaisons jointes par `and`.
const MAX_TERMS: usize = 8;

/// Valeur comparée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Str(String),
    Bool(bool),
}

impl Literal {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(value) => Some(value),
            Self::Bool(_) => None,
        }
    }
}

/// Une égalité.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// Chemin en minuscules, sans préfixe de schéma : `username`,
    /// `emails.value`, `members.value`.
    pub attr: String,
    /// Filtre de valeur entre crochets : sous-attribut en minuscules et valeur
    /// (`emails[type eq "work"]` → `("type", "work")`).
    pub selector: Option<(String, Literal)>,
    pub value: Literal,
}

/// Lit un filtre : une ou plusieurs égalités jointes par `and`.
pub fn parse(input: &str) -> Result<Vec<Comparison>, String> {
    if input.len() > MAX_LEN {
        return Err(format!("The filter is longer than {MAX_LEN} characters."));
    }
    let mut parser = Parser { chars: input.chars().collect(), at: 0 };
    let mut out = vec![parser.comparison()?];
    loop {
        parser.skip_spaces();
        if parser.done() {
            break;
        }
        let word = parser.word();
        if !word.eq_ignore_ascii_case("and") {
            return Err(if word.eq_ignore_ascii_case("or") || word.eq_ignore_ascii_case("not") {
                format!("The \"{word}\" operator is not supported: only \"and\" joins comparisons.")
            } else {
                "Unexpected text in the filter.".to_string()
            });
        }
        out.push(parser.comparison()?);
        if out.len() > MAX_TERMS {
            return Err(format!("A filter joins at most {MAX_TERMS} comparisons."));
        }
    }
    Ok(out)
}

/// Lit un chemin de `PATCH` à filtre de valeur seul, comme
/// `members[value eq "…"]` : `("members.value", "…")`.
pub fn parse_value_path(input: &str) -> Option<Comparison> {
    let terms = parse(input).ok()?;
    let [term] = terms.as_slice() else { return None };
    term.selector.is_none().then(|| term.clone())
}

/// Retire un préfixe de schéma (`urn:…:User:userName` → `userName`) et passe
/// en minuscules.
pub fn normalize_attr(attr: &str) -> String {
    let attr = attr.trim();
    // Le nom suit le dernier `:` : il n'en contient jamais, contrairement au
    // schéma (`urn:…:2.0:User:name.givenName`).
    let attr = if attr.get(..4).is_some_and(|head| head.eq_ignore_ascii_case("urn:")) {
        attr.rfind(':').map_or(attr, |cut| &attr[cut + 1..])
    } else {
        attr
    };
    attr.to_ascii_lowercase()
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn done(&self) -> bool {
        self.at >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn skip_spaces(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }

    /// Un mot : nom d'attribut (chemin compris), opérateur ou mot-clé.
    fn word(&mut self) -> String {
        self.skip_spaces();
        let start = self.at;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '_' | '-' | '$'))
        {
            self.at += 1;
        }
        self.chars[start..self.at].iter().collect()
    }

    fn expect(&mut self, wanted: char) -> Result<(), String> {
        self.skip_spaces();
        if self.peek() == Some(wanted) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("Expected \"{wanted}\" in the filter."))
        }
    }

    fn operator(&mut self) -> Result<(), String> {
        let op = self.word();
        if op.is_empty() {
            return Err("Expected an operator in the filter.".into());
        }
        if !op.eq_ignore_ascii_case("eq") {
            return Err(format!("The \"{op}\" operator is not supported: use \"eq\"."));
        }
        Ok(())
    }

    fn literal(&mut self) -> Result<Literal, String> {
        self.skip_spaces();
        if self.peek() == Some('"') {
            let start = self.at;
            self.at += 1;
            let mut escaped = false;
            loop {
                let Some(c) = self.peek() else {
                    return Err("Unterminated string in the filter.".into());
                };
                self.at += 1;
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    break;
                }
            }
            let raw: String = self.chars[start..self.at].iter().collect();
            return serde_json::from_str::<String>(&raw)
                .map(Literal::Str)
                .map_err(|_| "Invalid string in the filter.".to_string());
        }
        let word = self.word();
        if word.eq_ignore_ascii_case("true") {
            Ok(Literal::Bool(true))
        } else if word.eq_ignore_ascii_case("false") {
            Ok(Literal::Bool(false))
        } else {
            Err("Expected a quoted string, true or false in the filter.".into())
        }
    }

    fn comparison(&mut self) -> Result<Comparison, String> {
        self.skip_spaces();
        if self.peek() == Some('(') {
            return Err("Parentheses are not supported in filters.".into());
        }
        let attr = self.word();
        if attr.is_empty() {
            return Err("Expected an attribute name in the filter.".into());
        }
        let attr = normalize_attr(&attr);
        self.skip_spaces();
        if self.peek() != Some('[') {
            self.operator()?;
            let value = self.literal()?;
            return Ok(Comparison { attr, selector: None, value });
        }

        // Filtre de valeur : `attr[sub eq "v"]`, suivi ou non de `.sub2 eq "w"`.
        self.at += 1;
        let sub = self.word().to_ascii_lowercase();
        if sub.is_empty() {
            return Err("Expected an attribute name inside the brackets.".into());
        }
        self.operator()?;
        let inner = self.literal()?;
        self.expect(']')?;
        if self.peek() == Some('.') {
            self.at += 1;
            let tail = self.word().to_ascii_lowercase();
            if tail.is_empty() {
                return Err("Expected a sub-attribute after the brackets.".into());
            }
            self.operator()?;
            let value = self.literal()?;
            return Ok(Comparison {
                attr: format!("{attr}.{tail}"),
                selector: Some((sub, inner)),
                value,
            });
        }
        // `members[value eq "x"]` : un élément dont `value` vaut `x`.
        Ok(Comparison { attr: format!("{attr}.{sub}"), selector: None, value: inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eq(attr: &str, value: &str) -> Comparison {
        Comparison { attr: attr.into(), selector: None, value: Literal::Str(value.into()) }
    }

    #[test]
    fn simple_equalities_are_read_whatever_the_case() {
        let got = parse(r#"userName eq "jane@example.org""#).unwrap();
        assert_eq!(got, [eq("username", "jane@example.org")]);
        let got = parse(r#"USERNAME EQ "Jane""#).unwrap();
        assert_eq!(got, [eq("username", "Jane")]);
        let got = parse(r#"externalId eq "a\"b""#).unwrap();
        assert_eq!(got, [eq("externalid", "a\"b")]);
        let got = parse("active eq False").unwrap();
        let expected =
            Comparison { attr: "active".into(), selector: None, value: Literal::Bool(false) };
        assert_eq!(got, [expected]);
    }

    #[test]
    fn schema_prefixes_are_dropped() {
        let got = parse(r#"urn:ietf:params:scim:schemas:core:2.0:User:userName eq "x""#).unwrap();
        assert_eq!(got, [eq("username", "x")]);
        let long = "urn:ietf:params:scim:schemas:core:2.0:User:name.givenName";
        assert_eq!(normalize_attr(long), "name.givenname");
        assert_eq!(normalize_attr("displayName"), "displayname");
    }

    #[test]
    fn value_filters_are_read() {
        let got = parse(r#"emails[type eq "work"].value eq "j@x.org""#).unwrap();
        let expected = Comparison {
            attr: "emails.value".into(),
            selector: Some(("type".into(), Literal::Str("work".into()))),
            value: Literal::Str("j@x.org".into()),
        };
        assert_eq!(got, [expected]);
        let member = parse_value_path(r#"members[value eq "abc"]"#);
        assert_eq!(member, Some(eq("members.value", "abc")));
        assert_eq!(parse_value_path(r#"members[value eq "abc"].display eq "x""#), None);
    }

    #[test]
    fn and_joins_comparisons() {
        let got = parse(r#"id eq "g1" and members eq "u1""#).unwrap();
        assert_eq!(got, [eq("id", "g1"), eq("members", "u1")]);
    }

    #[test]
    fn everything_else_is_refused() {
        let refused = [
            "",
            "userName",
            r#"userName co "j""#,
            r#"userName sw "j""#,
            r#"userName pr"#,
            r#"userName eq "a" or userName eq "b""#,
            r#"not (userName eq "a")"#,
            r#"(userName eq "a")"#,
            r#"userName eq "unterminated"#,
            r#"userName eq 42"#,
            r#"userName eq "a" garbage"#,
            r#"emails[type eq "work" .value eq "x""#,
        ];
        for filter in refused {
            assert!(parse(filter).is_err(), "accepté à tort : {filter}");
        }
        let long = format!(r#"userName eq "{}""#, "a".repeat(MAX_LEN));
        assert!(parse(&long).is_err());
        let many: Vec<&str> = std::iter::repeat_n(r#"id eq "x""#, MAX_TERMS + 1).collect();
        let many = many.join(" and ");
        assert!(parse(&many).is_err());
    }
}
