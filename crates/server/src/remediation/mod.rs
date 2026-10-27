//! Procédures liées aux alertes et remédiation.
//!
//! - [`runbooks`] : textes Markdown rattachés à une règle, un dossier ou un type
//!   d'équipement, affichés sur l'alerte, la fiche de l'équipement et en lien
//!   dans les notifications.
//! - [`store`] : actions typées, politiques automatiques et journal d'audit.
//! - [`exec`] : exécution d'une action, manuelle ou automatique.
//! - [`auto`] : l'automate qui déclenche les politiques armées.
//!
//! Le catalogue des actions est **fermé** : quatre types, aucun champ qui porte
//! une commande. Redémarrer un service ou lancer un script passe par l'agent de
//! la machine, qui a le dernier mot (`actions: false` par défaut, liste de
//! services autorisés, scripts déclarés dans son propre dossier). Le serveur ne
//! fait que demander un nom et des paramètres validés des deux côtés.

pub mod auto;
pub mod exec;
pub mod runbooks;
pub mod store;

use std::collections::BTreeMap;

use dumbmonit_proto::remediation as rules;
use dumbmonit_proto::{CMD_CONTAINER_RESTART, CMD_SCRIPT_RUN, CMD_SERVICE_RESTART};
use serde::Serialize;
use serde_json::{Value, json};

/// Appel d'un webhook sortant, sous la garde anti-SSRF des notifications.
pub const KIND_WEBHOOK: &str = "webhook";

/// Les types d'action connus, dans l'ordre où l'interface les propose.
pub const KINDS: [&str; 4] =
    [CMD_SERVICE_RESTART, CMD_CONTAINER_RESTART, CMD_SCRIPT_RUN, KIND_WEBHOOK];

/// Longueur maximale d'une URL de webhook.
const MAX_URL_LEN: usize = 2048;

/// Longueur maximale d'un jeton de webhook.
const MAX_TOKEN_LEN: usize = 1024;

/// Un champ d'un type d'action, tel que l'interface le rend.
#[derive(Debug, Clone, Serialize)]
pub struct KindField {
    pub key: &'static str,
    pub label: &'static str,
    /// `text`, `url`, `secret` ou `params` (paires clé = valeur).
    pub input: &'static str,
    pub required: bool,
    pub help: &'static str,
}

/// Un type d'action du catalogue.
#[derive(Debug, Clone, Serialize)]
pub struct KindInfo {
    pub kind: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Vrai si l'action passe par l'agent de la machine.
    pub needs_agent: bool,
    pub fields: Vec<KindField>,
}

/// Le catalogue, décrit pour l'interface (`GET /api/remediation/kinds`).
pub fn catalog() -> Vec<KindInfo> {
    vec![
        KindInfo {
            kind: CMD_SERVICE_RESTART,
            label: "Restart a service",
            description: "Restarts a systemd unit or a Windows service through the agent. The \
                          service must be listed in action_services on the machine.",
            needs_agent: true,
            fields: vec![KindField {
                key: "service",
                label: "Service",
                input: "text",
                required: true,
                help: "Unit or service name, e.g. nginx or W32Time.",
            }],
        },
        KindInfo {
            kind: CMD_CONTAINER_RESTART,
            label: "Restart a Docker container",
            description: "Restarts a container through the agent's Docker channel.",
            needs_agent: true,
            fields: vec![KindField {
                key: "container",
                label: "Container",
                input: "text",
                required: true,
                help: "Container name.",
            }],
        },
        KindInfo {
            kind: CMD_SCRIPT_RUN,
            label: "Run a declared script",
            description: "Runs a script declared on the machine itself, in the agent's \
                          actions.d folder. The server only sends its name and parameters.",
            needs_agent: true,
            fields: vec![
                KindField {
                    key: "script",
                    label: "Script name",
                    input: "text",
                    required: true,
                    help: "Name of a manifest in actions.d, without .yaml.",
                },
                KindField {
                    key: "params",
                    label: "Parameters",
                    input: "params",
                    required: false,
                    help: "Values allowed by the script's manifest.",
                },
            ],
        },
        KindInfo {
            kind: KIND_WEBHOOK,
            label: "Call a webhook",
            description: "Sends a JSON POST describing the action, the device and the alert.",
            needs_agent: false,
            fields: vec![
                KindField {
                    key: "url",
                    label: "URL",
                    input: "url",
                    required: true,
                    help: "http:// or https:// address, without credentials.",
                },
                KindField {
                    key: "token",
                    label: "Bearer token",
                    input: "secret",
                    required: false,
                    help: "Sent as Authorization: Bearer. Never shown again.",
                },
            ],
        },
    ]
}

/// Une action validée, prête à être enregistrée ou exécutée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionSpec {
    Service { service: String },
    Container { container: String },
    Script { script: String, params: BTreeMap<String, String> },
    Webhook { url: String },
}

impl ActionSpec {
    /// Valide les paramètres (non secrets) d'un type d'action. Tout champ
    /// inconnu est refusé : rien ne doit pouvoir se glisser dans l'action sans
    /// que le serveur sache ce que c'est.
    pub fn parse(kind: &str, params: &Value) -> Result<Self, String> {
        let object = match params {
            Value::Object(map) => map.clone(),
            Value::Null => serde_json::Map::new(),
            _ => return Err("'params' must be an object.".to_string()),
        };
        let text = |key: &str| -> Result<String, String> {
            object
                .get(key)
                .and_then(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("'{key}' is required."))
        };
        let only = |allowed: &[&str]| -> Result<(), String> {
            match object.keys().find(|key| !allowed.contains(&key.as_str())) {
                Some(key) => Err(format!("Unknown field '{key}' for a {kind} action.")),
                None => Ok(()),
            }
        };
        match kind {
            CMD_SERVICE_RESTART => {
                only(&["service"])?;
                let service = text("service")?;
                rules::validate_service_name(&service)?;
                Ok(Self::Service { service })
            }
            CMD_CONTAINER_RESTART => {
                only(&["container"])?;
                let container = text("container")?;
                crate::api::agent_commands::validate_name(&container)?;
                Ok(Self::Container { container })
            }
            CMD_SCRIPT_RUN => {
                only(&["script", "params"])?;
                let script = text("script")?;
                rules::validate_action_name(&script)?;
                let mut values = BTreeMap::new();
                match object.get("params") {
                    None | Some(Value::Null) => {}
                    Some(Value::Object(map)) => {
                        for (key, value) in map {
                            let value = value
                                .as_str()
                                .ok_or_else(|| format!("Parameter '{key}' must be text."))?;
                            values.insert(key.clone(), value.trim().to_string());
                        }
                    }
                    Some(_) => return Err("'params' must be an object of texts.".to_string()),
                }
                rules::validate_params(&values)?;
                Ok(Self::Script { script, params: values })
            }
            KIND_WEBHOOK => {
                only(&["url"])?;
                let url = text("url")?;
                validate_webhook_url(&url)?;
                Ok(Self::Webhook { url })
            }
            other => Err(format!("Unknown action kind '{other}'.")),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Service { .. } => CMD_SERVICE_RESTART,
            Self::Container { .. } => CMD_CONTAINER_RESTART,
            Self::Script { .. } => CMD_SCRIPT_RUN,
            Self::Webhook { .. } => KIND_WEBHOOK,
        }
    }

    /// Paramètres tels qu'enregistrés et renvoyés par l'API (jamais de secret).
    pub fn to_params(&self) -> Value {
        match self {
            Self::Service { service } => json!({ "service": service }),
            Self::Container { container } => json!({ "container": container }),
            Self::Script { script, params } => json!({ "script": script, "params": params }),
            Self::Webhook { url } => json!({ "url": url }),
        }
    }

    /// Paramètres tels qu'inscrits au journal, que tout compte connecté lit :
    /// l'URL d'un webhook y perd sa chaîne de requête et son fragment, où un
    /// jeton pourrait se cacher malgré la consigne.
    pub fn log_params(&self) -> Value {
        match self {
            Self::Webhook { url } => {
                let shown = reqwest::Url::parse(url)
                    .map(|mut parsed| {
                        parsed.set_query(None);
                        parsed.set_fragment(None);
                        parsed.to_string()
                    })
                    .unwrap_or_default();
                json!({ "url": shown })
            }
            other => other.to_params(),
        }
    }

    pub fn needs_agent(&self) -> bool {
        !matches!(self, Self::Webhook { .. })
    }
}

/// Une URL de webhook : http(s), sans identifiants (ils seraient renvoyés en
/// clair par l'API — le jeton secret est fait pour cela), bornée.
pub fn validate_webhook_url(url: &str) -> Result<(), String> {
    if url.len() > MAX_URL_LEN {
        return Err("The URL is too long.".to_string());
    }
    let parsed = reqwest::Url::parse(url).map_err(|_| "The URL is not valid.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("The URL must start with http:// or https://.".to_string());
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        return Err("The URL has no host.".to_string());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Put credentials in the token field, not in the URL.".to_string());
    }
    Ok(())
}

/// Valide les secrets d'un type d'action. Seul le webhook en a (`token`).
pub fn validate_secrets(kind: &str, secrets: &BTreeMap<String, String>) -> Result<(), String> {
    for (key, value) in secrets {
        match (kind, key.as_str()) {
            (KIND_WEBHOOK, "token") => {
                if value.len() > MAX_TOKEN_LEN || value.chars().any(char::is_control) {
                    return Err("The token is not valid.".to_string());
                }
            }
            _ => return Err(format!("Unknown secret '{key}' for a {kind} action.")),
        }
    }
    Ok(())
}

/// Démarre l'automate des politiques.
pub fn spawn(state: crate::state::AppState) {
    tokio::spawn(auto::run(state));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_is_closed() {
        assert!(ActionSpec::parse("shell", &json!({"command": "rm -rf /"})).is_err());
        assert!(ActionSpec::parse("script.run", &json!({"script": "x", "command": "id"})).is_err());
        assert_eq!(catalog().len(), KINDS.len());
    }

    #[test]
    fn names_and_parameters_are_validated_on_save() {
        assert!(ActionSpec::parse("service.restart", &json!({"service": "-H"})).is_err());
        assert!(ActionSpec::parse("script.run", &json!({"script": "../x"})).is_err());
        assert!(
            ActionSpec::parse("script.run", &json!({"script": "x", "params": {"a": "b c"}}))
                .is_err()
        );
        assert!(
            ActionSpec::parse("container.restart", &json!({"container": "dumbmonit"})).is_err()
        );
        let ok = ActionSpec::parse(
            "script.run",
            &json!({"script": "flush", "params": {"level": "high"}}),
        )
        .unwrap();
        assert_eq!(ok.to_params()["params"]["level"], "high");
    }

    #[test]
    fn the_log_never_shows_a_webhook_query_string() {
        let spec = ActionSpec::Webhook { url: "https://hooks.lan/x?token=s3cret#frag".into() };
        assert_eq!(spec.log_params()["url"], "https://hooks.lan/x");
    }

    #[test]
    fn webhook_urls_carry_no_credentials() {
        assert!(validate_webhook_url("https://hooks.example.org/x").is_ok());
        assert!(validate_webhook_url("https://user:pw@hooks.example.org/x").is_err());
        assert!(validate_webhook_url("file:///etc/passwd").is_err());
        assert!(validate_webhook_url("gopher://x").is_err());
        assert!(
            validate_secrets("webhook", &BTreeMap::from([("token".into(), "t".into())])).is_ok()
        );
        assert!(
            validate_secrets("webhook", &BTreeMap::from([("other".into(), "t".into())])).is_err()
        );
        assert!(
            validate_secrets("service.restart", &BTreeMap::from([("token".into(), "t".into())]))
                .is_err()
        );
    }
}
