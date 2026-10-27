//! Configuration déclarative en fichiers YAML (GitOps).
//!
//! Un fichier décrit ce que l'instance doit contenir — équipements, règles
//! d'alerte et surcharges, canaux de notification, fenêtres de maintenance,
//! pages de statut — et `plan` / `apply` font converger l'instance vers lui.
//!
//! Le format n'invente rien : chaque section est celle du lot de sauvegarde
//! ([`crate::backup::bundle`]), avec les mêmes clés naturelles (type + adresse
//! pour un équipement, `uid` pour une règle, nom pour un canal, `slug` pour une
//! page) et les mêmes références (`parent`, `via_agent`, `selector.refs`… sont
//! des `type|adresse`). La restauration sait déjà créer sans dupliquer et
//! mettre à jour sans rien supprimer ; ce module ajoute ce qu'un fichier
//! déclaratif demande en plus :
//!
//! * un **plan** : la liste de ce qui serait créé, modifié ou supprimé, sans
//!   rien écrire, et sans jamais afficher une valeur secrète ;
//! * des **objets « gérés par fichier »** : marqués à l'application, refusés à
//!   la modification à la main, et seuls candidats à la suppression (`prune`) ;
//! * des **secrets par référence** (`${VARIABLE}` ou `${file:/chemin}`), jamais
//!   en clair dans un export.
//!
//! Les identifiants des équipements restent hors du fichier : tout se retrouve
//! par clé naturelle, ce qui rend le fichier portable d'une instance à l'autre.

pub mod apply;
pub mod guard;
pub mod plan;
pub mod refs;
pub mod watch;

use serde::{Deserialize, Serialize};

use crate::backup::bundle::{
    BundleChannel, BundleOverride, BundleRule, BundleSilence, BundleStatusPage, BundleTarget,
    target_ref,
};

/// Version du schéma du fichier. Un fichier d'une version supérieure est refusé.
pub const SCHEMA_VERSION: u32 = 1;

/// Valeur de `targets.managed_by` pour un équipement déclaré par un fichier.
pub const MANAGED_BY_FILE: &str = "config-file";

/// Le document déclaratif.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDoc {
    /// Version du schéma (`1`).
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<BundleTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<BundleRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_overrides: Vec<BundleOverride>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<BundleChannel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub silences: Vec<BundleSilence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_pages: Vec<BundleStatusPage>,
}

/// Les sections du document, dans l'ordre où le plan les présente.
pub const SECTIONS: [&str; 6] =
    ["targets", "channels", "rules", "rule_overrides", "silences", "status_pages"];

/// Clé naturelle d'une fenêtre de maintenance : son nom, suivi de son
/// équipement quand elle en vise un.
pub fn silence_key(name: &str, target: Option<&str>) -> String {
    match target {
        Some(target) => format!("{name}|{target}"),
        None => name.to_string(),
    }
}

impl ConfigDoc {
    /// Lit un document YAML. Les erreurs ne recopient pas le texte du fichier :
    /// il peut porter des références de secrets.
    pub fn parse_yaml(text: &str) -> Result<Self, String> {
        dumbmonit_collectors::yaml::from_str::<Self>(text)
            .map_err(|error| format!("The configuration file could not be read: {error}"))
    }

    /// Lit un document déjà décodé (JSON envoyé par l'outil en ligne de commande).
    pub fn from_json(value: serde_json::Value) -> Result<Self, String> {
        serde_json::from_value(value)
            .map_err(|error| format!("The configuration could not be read: {error}"))
    }

    pub fn to_yaml(&self) -> Result<String, String> {
        serde_saphyr::to_string(self).map_err(|error| format!("YAML could not be written: {error}"))
    }

    /// Fusionne plusieurs fichiers d'un dossier en un seul document.
    pub fn merge(docs: Vec<Self>) -> Result<Self, String> {
        let mut merged = Self { version: SCHEMA_VERSION, ..Self::default() };
        for doc in docs {
            if doc.version != SCHEMA_VERSION {
                return Err(version_error(doc.version));
            }
            merged.targets.extend(doc.targets);
            merged.rules.extend(doc.rules);
            merged.rule_overrides.extend(doc.rule_overrides);
            merged.channels.extend(doc.channels);
            merged.silences.extend(doc.silences);
            merged.status_pages.extend(doc.status_pages);
        }
        Ok(merged)
    }

    /// Contrôles qui ne demandent pas la base : version, clés présentes et
    /// uniques, ce qu'un fichier n'a pas le droit de déclarer.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != SCHEMA_VERSION {
            return Err(version_error(self.version));
        }
        let mut problems: Vec<String> = Vec::new();

        let mut seen = std::collections::HashSet::new();
        for target in &self.targets {
            if target.kind.trim().is_empty() || target.address.trim().is_empty() {
                problems.push(format!("targets: \"{}\" needs a kind and an address.", target.name));
            } else if !seen.insert(target_ref(&target.kind, &target.address)) {
                problems.push(format!(
                    "targets: {}|{} is declared twice.",
                    target.kind, target.address
                ));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for rule in &self.rules {
            if rule.uid.trim().is_empty() {
                problems.push(format!("rules: \"{}\" needs a uid.", rule.name));
            } else if !seen.insert(rule.uid.clone()) {
                problems.push(format!("rules: uid {} is declared twice.", rule.uid));
            }
            if rule.builtin {
                problems.push(format!(
                    "rules: \"{}\" is a built-in rule; adjust it with rule_overrides instead.",
                    rule.name
                ));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for channel in &self.channels {
            if channel.name.trim().is_empty() || !seen.insert(channel.name.clone()) {
                problems.push(format!(
                    "channels: \"{}\" has no name or is declared twice.",
                    channel.name
                ));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for silence in &self.silences {
            if !seen.insert(silence_key(&silence.name, silence.target.as_deref())) {
                problems.push(format!("silences: \"{}\" is declared twice.", silence.name));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for page in &self.status_pages {
            if page.slug.trim().is_empty() || !seen.insert(page.slug.clone()) {
                problems.push(format!(
                    "status_pages: \"{}\" has no slug or is declared twice.",
                    page.slug
                ));
            }
            if page.access_mode.as_deref() == Some("password") {
                problems.push(format!(
                    "status_pages: the password of \"{}\" is set in the interface (it is kept \
                     when a file is applied); a file cannot declare it.",
                    page.slug
                ));
            }
        }
        let mut seen = std::collections::HashSet::new();
        for over in &self.rule_overrides {
            if !seen.insert(format!("{}|{}", over.rule_uid, over.target)) {
                problems.push(format!(
                    "rule_overrides: {} on {} is declared twice.",
                    over.rule_uid, over.target
                ));
            }
        }

        if problems.is_empty() { Ok(()) } else { Err(problems.join("\n")) }
    }
}

fn version_error(found: u32) -> String {
    format!(
        "Unsupported configuration version {found}: this server reads version {SCHEMA_VERSION}. \
         Start the file with \"version: {SCHEMA_VERSION}\"."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minimal_file_parses_and_round_trips() {
        let text = "version: 1\ntargets:\n  - kind: http\n    address: https://example.org\n    \
                    name: Example\n    interval_secs: 60\n    enabled: true\n";
        let doc = ConfigDoc::parse_yaml(text).expect("parse");
        doc.validate().expect("valid");
        assert_eq!(doc.targets.len(), 1);
        let yaml = doc.to_yaml().expect("write");
        let again = ConfigDoc::parse_yaml(&yaml).expect("reparse");
        assert_eq!(again.targets[0].address, "https://example.org");
    }

    #[test]
    fn unknown_sections_and_versions_are_refused() {
        assert!(ConfigDoc::parse_yaml("version: 1\nusers: []\n").is_err());
        let doc = ConfigDoc::parse_yaml("version: 2\n").expect("parse");
        assert!(doc.validate().is_err());
    }

    #[test]
    fn duplicates_are_refused() {
        let text = "version: 1\nchannels:\n  - {name: a, kind: ntfy, enabled: true}\n  - {name: a, \
                    kind: ntfy, enabled: true}\n";
        let doc = ConfigDoc::parse_yaml(text).expect("parse");
        assert!(doc.validate().unwrap_err().contains("declared twice"));
    }
}
