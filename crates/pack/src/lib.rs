//! Paquets d'intégration : des types d'équipement décrits en YAML plutôt que
//! compilés dans le serveur.
//!
//! Un paquet déclare où lire (des `sources` HTTP ou Prometheus, toujours sur
//! l'hôte de la cible), quoi en extraire (des `metrics` par JSONPath, expression
//! régulière ou nom de famille Prometheus, et des lignes découvertes), et les
//! règles d'alerte qui vont avec. [`Pack::parse`] vérifie tout ce qui peut
//! l'être sans réseau ; [`PackCollector`] l'exécute comme n'importe quel
//! collecteur.
//!
//! Les garde-fous ne sont pas négociables par le paquet : chaque requête part
//! vers l'adresse de la cible et nulle part ailleurs, passe par le garde-fou
//! d'adresses des sondes, ne suit une redirection que vers la même origine, et
//! lit au plus [`MAX_BODY_BYTES`]. Les noms de métriques sont forcés sous
//! `dumbmonit_<paquet>_`, si bien qu'un paquet ne peut pas écrire dans les
//! séries d'un autre type.

pub mod cli;
mod collector;
mod compile;
mod describe;
mod extract;
pub mod fixture;
mod http;
pub mod manifest;
pub mod prom;
pub mod template;

use std::collections::BTreeSet;

pub use collector::PackCollector;
pub use compile::PackRule;
pub use extract::Body;

use compile::Source;
use manifest::Manifest;

/// Taille maximale d'un paquet, YAML compris.
pub const MAX_YAML_BYTES: usize = 256 * 1024;
/// Requêtes par interrogation, redirections comprises.
pub const MAX_REQUESTS: usize = 16;
/// Corps de réponse lu au plus.
pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
/// Séries par interrogation : au-delà, les suivantes sont abandonnées.
pub const MAX_SERIES: usize = 5_000;
/// Lignes découvertes au plus par bloc `discover`, quoi que dise le paquet.
pub const MAX_ROWS: usize = 1_000;
/// Métriques déclarées au plus (y compris celles des lignes découvertes).
pub const MAX_METRICS: usize = 256;
/// Règles déclarées au plus.
pub const MAX_RULES: usize = 64;

/// Préfixe du type de cible d'un paquet.
pub const KIND_PREFIX: &str = "pack.";
/// Préfixe que l'écriture ajoute à toutes les métriques.
pub const METRIC_PREFIX: &str = "dumbmonit_";

/// Un paquet lu et vérifié, prêt à collecter.
#[derive(Debug)]
pub struct Pack {
    manifest: Manifest,
    yaml: String,
    sha256: String,
    kind: String,
    /// Préfixe des noms d'échantillon (`shelly_plug_`), sans `dumbmonit_`.
    prefix: String,
    sources: Vec<Source>,
    rules: Vec<PackRule>,
    snmp_profiles: Vec<SnmpProfile>,
    /// Noms complets des séries produites (`dumbmonit_shelly_plug_power_watts`).
    produced: BTreeSet<String>,
    warnings: Vec<String>,
}

/// Un profil SNMP apporté par un paquet, à ajouter au catalogue.
#[derive(Debug, Clone)]
pub struct SnmpProfile {
    pub id: String,
    /// Le profil en YAML, tel que `Catalog::add_source` le lit.
    pub source: String,
}

/// Ce qui empêche un paquet d'être installé, une ligne par problème.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackError {
    pub errors: Vec<String>,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.errors.join("\n"))
    }
}

impl std::error::Error for PackError {}

impl Pack {
    /// Lit et vérifie un paquet. Toutes les erreurs sont rendues d'un coup :
    /// corriger un paquet ne doit pas se faire une ligne à la fois.
    pub fn parse(yaml: &str) -> Result<Self, PackError> {
        compile::compile(yaml)
    }

    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    pub fn version(&self) -> &str {
        &self.manifest.version
    }

    pub fn label(&self) -> &str {
        &self.manifest.label
    }

    pub fn summary(&self) -> &str {
        &self.manifest.summary
    }

    /// Type de cible : `pack.<id>`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Le YAML tel qu'il a été fourni.
    pub fn yaml(&self) -> &str {
        &self.yaml
    }

    /// Empreinte SHA-256 du YAML, en hexadécimal.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Vrai si le paquet interroge lui-même ses cibles (sources HTTP ou
    /// Prometheus) ; un paquet qui n'apporte que des profils SNMP n'a pas de
    /// type de cible à lui.
    pub fn has_collector(&self) -> bool {
        !self.sources.is_empty()
    }

    pub fn rules(&self) -> &[PackRule] {
        &self.rules
    }

    pub fn snmp_profiles(&self) -> &[SnmpProfile] {
        &self.snmp_profiles
    }

    /// Noms complets des métriques que le paquet peut produire.
    pub fn metric_names(&self) -> impl Iterator<Item = &str> {
        self.produced.iter().map(String::as_str)
    }

    /// Remarques qui n'empêchent pas l'installation (source inutilisée…).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Préfixe des noms d'échantillon, sans `dumbmonit_`.
    pub fn metric_prefix(&self) -> &str {
        &self.prefix
    }
}

/// Préfixe des échantillons d'un paquet : `shelly-plug` → `shelly_plug_`.
pub fn prefix_for(id: &str) -> String {
    format!("{}_", id.replace('-', "_"))
}

/// L'identifiant d'un paquet d'après son type de cible (`pack.x` → `x`).
pub fn id_from_kind(kind: &str) -> Option<&str> {
    kind.strip_prefix(KIND_PREFIX)
}
