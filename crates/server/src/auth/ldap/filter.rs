//! Filtres de recherche LDAP : échappement des valeurs et gabarits.
//!
//! Le nom d'utilisateur vient de l'écran de connexion, donc de n'importe qui.
//! Glissé tel quel dans `(sAMAccountName={username})`, un `*` ferait trouver
//! n'importe quel compte et un `)(` réécrirait le filtre (injection LDAP).
//! Toute valeur insérée passe donc par [`escape`], qui suit la RFC 4515 § 3 :
//! `\`, `*`, `(`, `)` et NUL deviennent `\5c`, `\2a`, `\28`, `\29` et `\00`.

/// Échappe une valeur pour l'insérer dans un filtre (RFC 4515 § 3).
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\5c"),
            '*' => out.push_str("\\2a"),
            '(' => out.push_str("\\28"),
            ')' => out.push_str("\\29"),
            '\0' => out.push_str("\\00"),
            other => out.push(other),
        }
    }
    out
}

/// Remplace chaque `{nom}` du gabarit par sa valeur échappée.
pub fn render(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), &escape(value));
    }
    out
}

/// Vérifie la forme d'un gabarit : entre parenthèses, équilibré, et portant au
/// moins l'un des marqueurs attendus.
pub fn check_template(template: &str, any_of: &[&str]) -> Result<(), String> {
    let template = template.trim();
    if !(template.starts_with('(') && template.ends_with(')')) {
        return Err("it must be enclosed in parentheses, like (uid={username}).".into());
    }
    let mut depth = 0i32;
    let mut escaped = false;
    for c in template.chars() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err("its parentheses are not balanced.".into());
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err("its parentheses are not balanced.".into());
    }
    if !any_of.iter().any(|marker| template.contains(marker)) {
        return Err(format!("it must contain {}.", any_of.join(" or ")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rfc_4515_specials_are_escaped() {
        assert_eq!(escape("jdoe"), "jdoe");
        assert_eq!(escape("*"), "\\2a");
        assert_eq!(escape("a(b)c"), "a\\28b\\29c");
        assert_eq!(escape("back\\slash"), "back\\5cslash");
        assert_eq!(escape("nul\0byte"), "nul\\00byte");
        // L'UTF-8 passe tel quel : la RFC l'admet dans une valeur.
        assert_eq!(escape("éloïse"), "éloïse");
    }

    #[test]
    fn an_injection_attempt_stays_a_literal_value() {
        let filter = render(
            "(&(objectClass=user)(sAMAccountName={username}))",
            &[("username", "*)(objectClass=*")],
        );
        assert_eq!(filter, "(&(objectClass=user)(sAMAccountName=\\2a\\29\\28objectClass=\\2a))");
        // Le filtre obtenu reste un seul test d'égalité : pas une parenthèse de plus.
        assert!(check_template(&filter, &["sAMAccountName"]).is_ok());

        let wildcard = render("(uid={username})", &[("username", "*")]);
        assert_eq!(wildcard, "(uid=\\2a)");
    }

    #[test]
    fn every_marker_is_replaced() {
        let filter = render(
            "(|(member={dn})(memberUid={username}))",
            &[("dn", "cn=J (Doe),dc=x"), ("username", "jdoe")],
        );
        assert_eq!(filter, "(|(member=cn=J \\28Doe\\29,dc=x)(memberUid=jdoe))");
    }

    #[test]
    fn templates_are_checked_for_shape_and_markers() {
        assert!(check_template("(uid={username})", &["{username}"]).is_ok());
        assert!(check_template("uid={username}", &["{username}"]).is_err());
        assert!(check_template("(uid={username}", &["{username}"]).is_err());
        assert!(check_template("(uid={username}))(", &["{username}"]).is_err());
        assert!(check_template("(objectClass=user)", &["{username}"]).is_err());
        assert!(check_template("(member={dn})", &["{dn}", "{username}"]).is_ok());
        // Une parenthèse échappée ne compte pas.
        assert!(check_template("(cn=a\\29{username})", &["{username}"]).is_ok());
    }
}
