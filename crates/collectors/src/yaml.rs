//! Lecture YAML commune à tout le produit (profils SNMP, paquets, `agent.yaml`,
//! spécification OpenAPI).
//!
//! `serde-saphyr` est en Rust pur et borne d'office ce qu'un document hostile
//! peut coûter (budget d'événements, alias). Seul réglage : les booléens
//! stricts. Par défaut il lit `yes`/`no`/`on`/`off` comme des booléens
//! (YAML 1.1) ; DumbMonit a toujours suivi YAML 1.2, où ce sont des chaînes —
//! une table `map: { yes: 1, no: 0 }` d'un paquet doit garder ses clés.
//! Sans extrait de source dans les erreurs : `agent.yaml` porte le jeton de
//! l'agent, qu'une erreur de syntaxe recopierait sinon dans le journal.

use serde::de::DeserializeOwned;

pub use serde_saphyr::Error;

/// Lit un document YAML (YAML 1.2 : seuls `true` et `false` sont booléens).
pub fn from_str<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    let mut options = serde_saphyr::Options::default();
    options.strict_booleans = true;
    options.with_snippet = false;
    serde_saphyr::from_str_with_options(text, options)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    #[test]
    fn yes_and_no_stay_strings() {
        let map: BTreeMap<String, serde_json::Value> =
            super::from_str("yes: on\nno: off\nflag: true\n").unwrap();
        assert_eq!(map["yes"], "on");
        assert_eq!(map["no"], "off");
        assert_eq!(map["flag"], true);
    }

    #[test]
    fn json_is_yaml() {
        let value: serde_json::Value = super::from_str(r#"{"a": [1, "b", null]}"#).unwrap();
        assert_eq!(value, serde_json::json!({"a": [1, "b", null]}));
    }

    #[test]
    fn errors_do_not_quote_the_source() {
        let err = super::from_str::<serde_json::Value>("token: dmon_secret
url: [
")
            .unwrap_err()
            .to_string();
        assert!(!err.contains("dmon_secret"), "{err}");
    }
}
