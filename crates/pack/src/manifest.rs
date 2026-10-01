//! Le format d'un paquet tel qu'il s'écrit en YAML (`schema: 1`).
//!
//! Ces structures ne font que lire : aucune vérification de sens ici, c'est le
//! rôle de [`crate::Pack::parse`], qui dit *où* est l'erreur dans des termes que
//! l'auteur du paquet comprend. `deny_unknown_fields` partout : une faute de
//! frappe dans une clé (`treshold`) doit être une erreur, pas un réglage ignoré.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer};

/// Seule version du format connue de ce serveur.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    /// `[a-z][a-z0-9-]*` : le type de cible sera `pack.<id>`, le préfixe des
    /// métriques `dumbmonit_<id avec des _>_`.
    pub id: String,
    /// Version sémantique du paquet (`1.2.0`).
    pub version: String,
    /// Versions de DumbMonit acceptées, en exigence semver (`>=0.1`).
    #[serde(default)]
    pub requires: Option<String>,
    pub label: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub address_hint: String,
    #[serde(default)]
    pub default_port: u16,
    /// Protocole par défaut ; la cible peut le changer (option `scheme`).
    #[serde(default)]
    pub scheme: Scheme,
    #[serde(default)]
    pub setup: Setup,
    /// Formes d'identifiant acceptées, dans l'ordre de préférence. Vide : `none`.
    #[serde(default)]
    pub credentials: Vec<CredentialSpec>,
    #[serde(default)]
    pub options: Vec<OptionSpec>,
    #[serde(default)]
    pub sources: Vec<SourceSpec>,
    #[serde(default)]
    pub metrics: Vec<MetricSpec>,
    #[serde(default)]
    pub discover: Vec<DiscoverSpec>,
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
    /// Profils SNMP, repris tels quels du format de `profiles/*.yaml`.
    #[serde(default)]
    pub snmp_profiles: Vec<serde_yaml_ng::Value>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    #[default]
    Http,
    Https,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub warning: String,
    #[serde(default)]
    pub doc_url: String,
}

/// `api_token`, ou `{kind: api_token, label: …, help: …}` pour reformuler.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CredentialSpec {
    Kind(String),
    Detailed(CredentialDetail),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialDetail {
    pub kind: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub help: Option<String>,
}

impl CredentialSpec {
    pub fn kind(&self) -> &str {
        match self {
            Self::Kind(kind) => kind,
            Self::Detailed(detail) => &detail.kind,
        }
    }
}

/// Un réglage lu dans `Target::tags[key]`, même forme que ceux des types compilés.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionSpec {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub help: String,
    #[serde(default)]
    pub placeholder: String,
    #[serde(default)]
    pub default: Scalar,
    #[serde(default)]
    pub required: bool,
    /// `text`, `number`, `boolean` ou `select`.
    #[serde(default = "text_input")]
    pub input: String,
    #[serde(default)]
    pub choices: Vec<String>,
}

fn text_input() -> String {
    "text".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    /// Une requête HTTP dont la réponse est lue en JSON ou en texte.
    Http,
    /// Une page au format d'exposition Prometheus.
    Prometheus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    #[default]
    Get,
    Post,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    /// D'après l'identifiant de la cible : jeton → `Bearer`, identifiants → basic.
    #[default]
    Auto,
    /// Rien d'implicite ; les en-têtes peuvent toujours citer l'identifiant.
    None,
    Bearer,
    Basic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Json,
    Text,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpec {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: SourceType,
    /// Chemin relatif à la racine de la cible, toujours sur son hôte.
    pub path: String,
    #[serde(default)]
    pub method: Method,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub auth: Auth,
    /// `http` seulement ; `json` par défaut.
    #[serde(default)]
    pub format: Option<Format>,
    /// `prometheus` seulement : familles retenues telles quelles.
    #[serde(default)]
    pub keep: Vec<String>,
    /// `prometheus` seulement : nouveau nom d'une famille retenue.
    #[serde(default)]
    pub rename: BTreeMap<String, String>,
    /// `prometheus` seulement : un échantillon dont l'étiquette correspond à
    /// l'expression est écarté (pseudo-systèmes de fichiers, interfaces virtuelles).
    #[serde(default)]
    pub drop: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Gauge,
    Counter,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSpec {
    /// Nom sans préfixe : `power_watts` devient `dumbmonit_<paquet>_power_watts`.
    pub name: String,
    #[serde(default)]
    pub help: String,
    pub source: String,
    #[serde(default)]
    pub kind: Kind,
    /// Chemin JSONPath dans la réponse (source `http` au format `json`).
    #[serde(default)]
    pub json: Option<String>,
    /// Expression régulière (source `http` au format `text`).
    #[serde(default)]
    pub regex: Option<String>,
    /// Nom de famille (source `prometheus`).
    #[serde(default)]
    pub prom: Option<String>,
    #[serde(default)]
    pub scale: Option<f64>,
    /// Valeur textuelle → nombre (`on: 1`, `off: 0`).
    #[serde(default)]
    pub map: BTreeMap<Scalar, f64>,
    /// Étiquette → d'où la lire (voir la documentation du format).
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverSpec {
    pub name: String,
    pub source: String,
    /// JSONPath qui désigne les lignes (`$.sensors[*]`).
    pub rows: String,
    #[serde(default = "default_max_rows")]
    pub max_rows: usize,
    /// Étiquette → JSONPath relatif à la ligne (`$.name`).
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    pub metrics: Vec<RowMetricSpec>,
}

pub const DEFAULT_MAX_ROWS: usize = 100;

fn default_max_rows() -> usize {
    DEFAULT_MAX_ROWS
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowMetricSpec {
    pub name: String,
    #[serde(default)]
    pub help: String,
    #[serde(default)]
    pub kind: Kind,
    /// JSONPath relatif à la ligne.
    pub json: String,
    #[serde(default)]
    pub scale: Option<f64>,
    #[serde(default)]
    pub map: BTreeMap<Scalar, f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSpec {
    /// Identifiant : la règle installée porte l'uid `pack:<paquet>:<name>`.
    pub name: String,
    /// Libellé affiché ; le nom à défaut.
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// Expression MetricsQL, qui cite les métriques du paquet par leur nom complet.
    pub expr: String,
    #[serde(default = "default_op")]
    pub op: String,
    /// Un nombre, ou `{{option.x}}` : la valeur par défaut de cette option.
    pub threshold: Scalar,
    /// Durée pendant laquelle la condition doit tenir : `30s`, `5m`, `1h`.
    #[serde(default, rename = "for")]
    pub for_duration: Option<String>,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub unit: String,
}

fn default_op() -> String {
    ">".into()
}

fn default_severity() -> String {
    "warning".into()
}

/// Un scalaire YAML lu comme texte : `0`, `true`, `1.5` et `"on"` sont tous
/// acceptés là où l'auteur pense « une valeur », sans guillemets imposés.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scalar(pub String);

impl<'de> Deserialize<'de> for Scalar {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Text(String),
            Bool(bool),
            Int(i64),
            Float(f64),
        }
        Ok(Self(match Raw::deserialize(deserializer)? {
            Raw::Text(text) => text,
            Raw::Bool(value) => value.to_string(),
            Raw::Int(value) => value.to_string(),
            Raw::Float(value) => value.to_string(),
        }))
    }
}
