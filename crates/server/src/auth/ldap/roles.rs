//! Des groupes de l'annuaire au rôle DumbMonit.
//!
//! La règle : membre d'un groupe administrateur → `admin`, sinon d'un groupe
//! opérateur → `operator`, sinon d'un groupe lecteur → `viewer`. Dès qu'une
//! liste est remplie, il faut appartenir à l'un des groupes listés pour entrer :
//! un utilisateur qui perd ses groupes perd l'accès. Sans aucune liste, tout
//! utilisateur trouvé par le filtre entre, et son rôle se règle à la main.
//!
//! Un groupe configuré se compare au DN complet du groupe ou à son nom court
//! (la valeur du premier RDN, `cn=Admins,ou=…` → `Admins`), sans tenir compte
//! de la casse — l'annuaire ne la respecte pas non plus.

use crate::auth::users::Role;

/// Ce que les groupes décident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Role(Role),
    /// Les groupes ne disent rien : le compte garde son rôle (lecteur s'il est créé).
    Keep,
    /// Membre d'aucun groupe autorisé.
    Deny,
}

/// Décide pour un utilisateur membre de `groups` (DN), qui a aujourd'hui
/// `current` (`None` pour un compte à créer).
pub fn decide(
    groups: &[String],
    admin_groups: &[String],
    operator_groups: &[String],
    viewer_groups: &[String],
    current: Option<Role>,
) -> Decision {
    if admin_groups.is_empty() && operator_groups.is_empty() && viewer_groups.is_empty() {
        return Decision::Keep;
    }
    let member = |listed: &[String]| {
        groups.iter().any(|group| listed.iter().any(|wanted| matches(wanted, group)))
    };
    if member(admin_groups) {
        return Decision::Role(Role::Admin);
    }
    let role = if member(operator_groups) {
        Role::Operator
    } else if member(viewer_groups) {
        Role::Viewer
    } else {
        return Decision::Deny;
    };
    // Sans liste d'administrateurs, les groupes ne disent rien du rôle `admin` :
    // un administrateur promu à la main le reste.
    if admin_groups.is_empty() && current == Some(Role::Admin) {
        return Decision::Keep;
    }
    Decision::Role(role)
}

/// Un groupe configuré désigne-t-il ce groupe (DN) ?
fn matches(wanted: &str, group_dn: &str) -> bool {
    let wanted = wanted.trim().to_lowercase();
    if wanted.is_empty() {
        return false;
    }
    let dn = group_dn.trim().to_lowercase();
    wanted == dn || short_name(&dn).is_some_and(|name| name == wanted)
}

/// La valeur du premier RDN d'un DN : `cn=Admins,ou=Groups` → `Admins`. Les
/// séquences d'échappement (`\,`, `\2C`) sont décodées pour la comparaison.
pub fn short_name(dn: &str) -> Option<String> {
    let mut first = String::new();
    let mut chars = dn.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                first.push(c);
                if let Some(next) = chars.next() {
                    first.push(next);
                }
            }
            ',' | '+' => break,
            other => first.push(other),
        }
    }
    let (_, value) = first.split_once('=')?;
    let value = unescape(value.trim());
    (!value.is_empty()).then_some(value)
}

fn unescape(value: &str) -> String {
    let mut out = Vec::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // `from_str_radix` admettrait un signe (`\+1`) : deux chiffres hexadécimaux exigés.
            let hex = value
                .get(i + 1..i + 3)
                .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(byte) = hex {
                out.push(byte);
                i += 3;
            } else {
                out.push(bytes[i + 1]);
                i += 2;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    const ADMINS: &str = "CN=DumbMonit Admins,OU=Groups,DC=corp,DC=example";
    const NOC: &str = "cn=noc,ou=groups,dc=example,dc=org";
    const STAFF: &str = "cn=staff,ou=groups,dc=example,dc=org";

    #[test]
    fn groups_map_to_the_highest_role() {
        let admin = strings(&["dumbmonit admins"]);
        let operator = strings(&[NOC]);
        let viewer = strings(&["staff"]);
        let decide = |groups: &[&str]| decide(&strings(groups), &admin, &operator, &viewer, None);
        assert_eq!(decide(&[ADMINS, NOC]), Decision::Role(Role::Admin));
        assert_eq!(decide(&[NOC, STAFF]), Decision::Role(Role::Operator));
        assert_eq!(decide(&[STAFF]), Decision::Role(Role::Viewer));
    }

    #[test]
    fn losing_every_listed_group_denies_access() {
        let viewer = strings(&["staff"]);
        assert_eq!(decide(&[], &[], &[], &viewer, Some(Role::Viewer)), Decision::Deny);
        assert_eq!(
            decide(&strings(&["cn=other,dc=x"]), &strings(&["admins"]), &[], &[], None),
            Decision::Deny
        );
    }

    #[test]
    fn without_any_list_the_groups_say_nothing() {
        assert_eq!(decide(&strings(&[STAFF]), &[], &[], &[], None), Decision::Keep);
        assert_eq!(decide(&[], &[], &[], &[], Some(Role::Admin)), Decision::Keep);
    }

    #[test]
    fn without_an_admin_list_an_admin_is_never_demoted() {
        let viewer = strings(&["staff"]);
        assert_eq!(
            decide(&strings(&[STAFF]), &[], &[], &viewer, Some(Role::Admin)),
            Decision::Keep
        );
        assert_eq!(
            decide(&strings(&[STAFF]), &[], &[], &viewer, Some(Role::Operator)),
            Decision::Role(Role::Viewer)
        );
    }

    #[test]
    fn a_group_matches_by_full_dn_or_short_name_whatever_the_case() {
        assert!(matches(ADMINS, &ADMINS.to_lowercase()));
        assert!(matches("DumbMonit Admins", ADMINS));
        assert!(matches("dumbmonit admins", ADMINS));
        assert!(!matches("Admins", ADMINS));
        assert!(!matches("", ADMINS));
        assert!(!matches("ou=groups", ADMINS));
    }

    #[test]
    fn the_short_name_reads_escaped_values() {
        assert_eq!(short_name("cn=Doe\\, John,ou=x").as_deref(), Some("Doe, John"));
        assert_eq!(short_name("cn=Ops\\2C Team,dc=x").as_deref(), Some("Ops, Team"));
        assert_eq!(short_name("cn=Admins").as_deref(), Some("Admins"));
        assert_eq!(short_name("not a dn"), None);
    }
}
