//! Catalogue des destinations de journaux : ce que l'interface doit demander
//! pour chacune, et comment la construire.
//!
//! Même principe que `notify::catalog` : l'interface ne connaît aucun service,
//! elle rend les champs annoncés ici. Ajouter une destination, c'est écrire son
//! module dans `sinks/` et ajouter une ligne à [`SPECS`] — rien d'autre ne
//! connaît les types concrets.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};

use super::sinks::{self, Sink};
use crate::notify::catalog::Field;

/// Page de la documentation qui décrit chaque destination.
pub const LOGS_DOC: &str = "https://dumbmonit.readthedocs.io/en/latest/install/logs/";

/// Un type de destination, prêt à être présenté.
#[derive(Debug, Clone, Serialize)]
pub struct KindInfo {
    pub kind: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    pub doc_url: String,
    pub settings: Vec<Field>,
    pub secrets: Vec<Field>,
}

/// Réglages et secrets d'une destination, tels que la construction les lit.
#[derive(Debug, Clone, Default)]
pub struct DestinationConfig {
    pub settings: Map<String, Value>,
    pub secrets: HashMap<String, String>,
}

impl DestinationConfig {
    /// Valeur texte non vide d'un réglage (un nombre est accepté et converti).
    pub fn text(&self, key: &str) -> Option<String> {
        match self.settings.get(key)? {
            Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        }
    }

    pub fn flag(&self, key: &str) -> bool {
        match self.settings.get(key) {
            Some(Value::Bool(value)) => *value,
            Some(Value::String(text)) => matches!(text.as_str(), "true" | "1" | "on" | "yes"),
            _ => false,
        }
    }

    pub fn secret(&self, key: &str) -> Option<&str> {
        self.secrets.get(key).map(String::as_str).filter(|value| !value.is_empty())
    }

    /// Valeur d'un réglage, ou son défaut annoncé au catalogue.
    pub fn text_or(&self, key: &str, default: &str) -> String {
        self.text(key).unwrap_or_else(|| default.to_string())
    }
}

/// Une entrée du catalogue : sa description et son constructeur.
pub struct KindSpec {
    pub kind: &'static str,
    pub describe: fn() -> KindInfo,
    pub build: fn(&DestinationConfig) -> Result<Arc<dyn Sink>, String>,
}

/// Toutes les destinations, dans l'ordre du menu.
pub const SPECS: &[KindSpec] = &[
    KindSpec { kind: "gelf", describe: sinks::gelf::describe, build: sinks::gelf::build },
    KindSpec {
        kind: "victorialogs",
        describe: sinks::victorialogs::describe,
        build: sinks::victorialogs::build,
    },
    KindSpec { kind: "loki", describe: sinks::loki::describe, build: sinks::loki::build },
    KindSpec {
        kind: "elasticsearch",
        describe: sinks::elasticsearch::describe,
        build: sinks::elasticsearch::build,
    },
    KindSpec { kind: "syslog", describe: sinks::syslog::describe, build: sinks::syslog::build },
];

pub fn all() -> Vec<KindInfo> {
    SPECS.iter().map(|spec| (spec.describe)()).collect()
}

pub fn spec(kind: &str) -> Option<&'static KindSpec> {
    SPECS.iter().find(|spec| spec.kind == kind)
}

/// Construit l'expéditeur d'une destination.
///
/// Ses erreurs sont expurgées des secrets de la destination : un service qui
/// recopie l'en-tête reçu dans sa réponse d'erreur ne fait pas fuir le jeton
/// jusqu'à l'interface ou au journal.
pub fn build(kind: &str, config: &DestinationConfig) -> Result<Arc<dyn Sink>, String> {
    let spec = spec(kind).ok_or_else(|| format!("unknown log destination type `{kind}`"))?;
    let inner = (spec.build)(config).map_err(|error| redact(&error, config))?;
    let secrets: Vec<String> =
        config.secrets.values().filter(|value| value.len() >= 4).cloned().collect();
    if secrets.is_empty() {
        return Ok(inner);
    }
    Ok(Arc::new(Redacting { inner, secrets }))
}

fn redact(error: &str, config: &DestinationConfig) -> String {
    let mut out = error.to_string();
    for secret in config.secrets.values().filter(|value| value.len() >= 4) {
        out = out.replace(secret.as_str(), super::scrub::MASK);
    }
    out
}

struct Redacting {
    inner: Arc<dyn Sink>,
    secrets: Vec<String>,
}

#[async_trait::async_trait]
impl Sink for Redacting {
    async fn send(&self, batch: &[Arc<super::Record>]) -> Result<(), String> {
        self.inner.send(batch).await.map_err(|error| {
            self.secrets
                .iter()
                .fold(error, |error, secret| error.replace(secret.as_str(), super::scrub::MASK))
        })
    }
}

// --------------------------------------------------------------------------
// Construction des champs, pour les modules de `sinks`
// --------------------------------------------------------------------------

pub(crate) fn doc(anchor: &str) -> String {
    format!("{LOGS_DOC}#{anchor}")
}

fn field(key: &'static str, label: &'static str, input: &'static str, help: &'static str) -> Field {
    Field {
        key,
        label,
        required: false,
        input,
        help,
        placeholder: "",
        options: &[],
        shape: "scalar",
        default: "",
        destination: false,
    }
}

pub(crate) fn text(key: &'static str, label: &'static str, help: &'static str) -> Field {
    field(key, label, "text", help)
}

pub(crate) fn url(key: &'static str, label: &'static str, help: &'static str) -> Field {
    Field { destination: true, required: true, ..field(key, label, "url", help) }
}

pub(crate) fn address(help: &'static str, placeholder: &'static str) -> Field {
    Field {
        destination: true,
        required: true,
        placeholder,
        ..field("address", "Address", "text", help)
    }
}

pub(crate) fn boolean(key: &'static str, label: &'static str, help: &'static str) -> Field {
    field(key, label, "boolean", help)
}

pub(crate) fn password(key: &'static str, label: &'static str, help: &'static str) -> Field {
    field(key, label, "password", help)
}

pub(crate) fn textarea(key: &'static str, label: &'static str, help: &'static str) -> Field {
    field(key, label, "textarea", help)
}

/// Liste fermée ; le premier choix est le défaut.
pub(crate) fn select(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    options: &'static [&'static str],
) -> Field {
    Field { options, default: options[0], ..field(key, label, "select", help) }
}

pub(crate) fn placeholder(field: Field, placeholder: &'static str) -> Field {
    Field { placeholder, ..field }
}

pub(crate) fn with_default(field: Field, default: &'static str) -> Field {
    Field { default, ..field }
}

pub(crate) fn destination(field: Field) -> Field {
    Field { destination: true, ..field }
}

/// Identifiant HTTP Basic, commun aux destinations HTTP.
pub(crate) fn username() -> Field {
    text("username", "Username", "HTTP Basic authentication, with the password below.")
}

pub(crate) fn insecure_tls() -> Field {
    boolean(
        "insecure_tls",
        "Skip certificate verification",
        "Accept a self-signed or private certificate. The connection stays encrypted but is not authenticated.",
    )
}

/// Clés des secrets annoncés pour un type.
pub fn secret_keys(kind: &str) -> Vec<&'static str> {
    spec(kind)
        .map(|spec| (spec.describe)().secrets.iter().map(|f| f.key).collect())
        .unwrap_or_default()
}

/// Clés des réglages annoncés pour un type.
pub fn setting_keys(kind: &str) -> Vec<&'static str> {
    spec(kind)
        .map(|spec| (spec.describe)().settings.iter().map(|f| f.key).collect())
        .unwrap_or_default()
}

/// Clés des réglages qui disent où partent les secrets.
pub fn destination_keys(kind: &str) -> Vec<&'static str> {
    spec(kind)
        .map(|spec| {
            (spec.describe)().settings.iter().filter(|f| f.destination).map(|f| f.key).collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_type_se_construit_avec_ses_seuls_champs_obligatoires_et_defauts() {
        for spec in SPECS {
            let info = (spec.describe)();
            assert_eq!(info.kind, spec.kind);
            assert!(info.doc_url.starts_with(LOGS_DOC));
            let mut config = DestinationConfig::default();
            for field in &info.settings {
                let value = if !field.default.is_empty() {
                    field.default.to_string()
                } else if field.required {
                    field.placeholder.to_string()
                } else {
                    continue;
                };
                assert!(!value.is_empty(), "{}: {} sans exemple", spec.kind, field.key);
                config.settings.insert(field.key.into(), Value::String(value));
            }
            if let Err(error) = (spec.build)(&config) {
                panic!("{} ne se construit pas : {error}", spec.kind);
            }
            for field in info.settings.iter().chain(&info.secrets) {
                if field.input == "select" {
                    assert!(field.options.contains(&field.default));
                }
            }
        }
    }

    #[test]
    fn aucun_secret_n_est_un_reglage() {
        for spec in SPECS {
            let info = (spec.describe)();
            for field in &info.settings {
                assert_ne!(
                    field.input, "password",
                    "{}.{} doit être un secret",
                    spec.kind, field.key
                );
            }
        }
    }
}
