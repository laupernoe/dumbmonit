//! Du groupe au rôle.
//!
//! La règle : membre d'un groupe administrateur → `admin`, sinon d'un groupe
//! opérateur → `operator`, sinon `viewer`. Sans aucune liste, le fournisseur ne
//! dit rien du rôle et l'on garde celui que le compte a déjà. Sans liste
//! d'administrateurs, les groupes ne disent rien non plus du rôle `admin` : un
//! administrateur le reste.

use serde_json::Value;

use crate::auth::users::Role;

/// Rôle déduit des groupes annoncés pour un compte qui a aujourd'hui `current`
/// (`None` pour un compte à créer), ou `None` si les groupes configurés ne
/// permettent pas de décider.
pub fn role_for_groups(
    groups: &[String],
    admin_groups: &[String],
    operator_groups: &[String],
    current: Option<Role>,
) -> Option<Role> {
    if admin_groups.is_empty() && operator_groups.is_empty() {
        return None;
    }
    let member = |listed: &[String]| groups.iter().any(|group| listed.contains(group));
    if member(admin_groups) {
        return Some(Role::Admin);
    }
    if admin_groups.is_empty() && current == Some(Role::Admin) {
        return None;
    }
    Some(if member(operator_groups) { Role::Operator } else { Role::Viewer })
}

/// Lit la revendication de groupes, quelle que soit sa forme : tableau de chaînes
/// (le cas normal), chaîne unique, ou chaîne séparée par des virgules ou des
/// espaces comme certains fournisseurs la renvoient.
pub fn groups_from_claim(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => {
            items.iter().filter_map(Value::as_str).map(str::to_string).collect()
        }
        Some(Value::String(text)) => text
            .split([',', ' '])
            .map(str::trim)
            .filter(|group| !group.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_member_of_a_listed_group_is_admin_and_the_others_are_viewers() {
        let admin_groups = strings(&["dumbmonit-admins", "ops"]);
        let check = |groups: &[&str]| role_for_groups(&strings(groups), &admin_groups, &[], None);
        assert_eq!(check(&["dev", "ops"]), Some(Role::Admin));
        assert_eq!(check(&["dev"]), Some(Role::Viewer));
        assert_eq!(check(&[]), Some(Role::Viewer));
    }

    #[test]
    fn operator_groups_sit_between_admin_and_viewer() {
        let admins = strings(&["admins"]);
        let operators = strings(&["noc"]);
        let check = |groups: &[&str], current| {
            role_for_groups(&strings(groups), &admins, &operators, current)
        };
        assert_eq!(check(&["noc"], None), Some(Role::Operator));
        assert_eq!(check(&["noc", "admins"], None), Some(Role::Admin));
        assert_eq!(check(&["dev"], Some(Role::Operator)), Some(Role::Viewer));
        assert_eq!(check(&["noc"], Some(Role::Admin)), Some(Role::Operator));
    }

    #[test]
    fn without_an_admin_list_the_groups_never_demote_an_admin() {
        let operators = strings(&["noc"]);
        let check =
            |groups: &[&str], current| role_for_groups(&strings(groups), &[], &operators, current);
        assert_eq!(check(&["noc"], Some(Role::Admin)), None);
        assert_eq!(check(&["dev"], Some(Role::Admin)), None);
        assert_eq!(check(&["noc"], Some(Role::Viewer)), Some(Role::Operator));
        assert_eq!(check(&["dev"], Some(Role::Operator)), Some(Role::Viewer));
    }

    #[test]
    fn without_a_configured_list_the_provider_says_nothing() {
        assert_eq!(role_for_groups(&strings(&["ops"]), &[], &[], None), None);
    }

    #[test]
    fn the_groups_claim_is_read_in_its_common_shapes() {
        assert_eq!(groups_from_claim(Some(&json!(["a", "b", 3]))), strings(&["a", "b"]));
        assert_eq!(groups_from_claim(Some(&json!("a, b c"))), strings(&["a", "b", "c"]));
        assert!(groups_from_claim(None).is_empty());
        assert!(groups_from_claim(Some(&json!(42))).is_empty());
    }
}
