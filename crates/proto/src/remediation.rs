//! Actions de remédiation : ce que le serveur peut demander à un agent au-delà
//! des conteneurs, et ce que l'agent déclare savoir faire.
//!
//! Le catalogue est fermé et typé. Le serveur ne transmet jamais une ligne de
//! commande : il demande un *nom* (un service, un script déclaré localement sur
//! la machine) et des paramètres dont chaque valeur est validée ici, des deux
//! côtés du canal. C'est l'agent, et lui seul, qui sait ce qu'un nom de script
//! exécute ; un serveur détourné ne peut donc rien lancer que l'administrateur
//! de la machine n'ait déposé lui-même.
//!
//! Les règles de validation vivent dans ce module partagé pour que le serveur
//! refuse à l'enregistrement exactement ce que l'agent refuserait à l'exécution.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Redémarre un service systemd ou Windows. Arguments : `{"name": "…"}` plus
/// l'enveloppe anti-rejeu ([`ARG_NONCE`], [`ARG_EXPIRES_AT_MS`]).
pub const CMD_SERVICE_RESTART: &str = "service.restart";

/// Lance un script déclaré dans le dossier d'actions de l'agent. Arguments :
/// `{"name": "…", "params": {"clé": "valeur"}}` plus l'enveloppe anti-rejeu.
pub const CMD_SCRIPT_RUN: &str = "script.run";

/// Jeton à usage unique d'une demande : l'agent refuse un nonce déjà vu.
pub const ARG_NONCE: &str = "nonce";

/// Échéance absolue de la demande, en millisecondes Unix : passée cette heure,
/// l'agent refuse de l'exécuter, même si elle lui parvient intacte.
pub const ARG_EXPIRES_AT_MS: &str = "expires_at_ms";

/// Durée de validité d'une demande de remédiation, en secondes. La même que
/// celle de la file de commandes : au-delà, la panne a changé de nature.
pub const ACTION_TTL_SECS: i64 = 600;

/// Nombre maximal de paramètres d'un script.
pub const MAX_SCRIPT_PARAMS: usize = 8;

/// Longueur maximale d'une valeur de paramètre, en octets.
pub const MAX_PARAM_VALUE_LEN: usize = 128;

/// Nombre maximal de valeurs permises par paramètre dans un manifeste.
pub const MAX_PARAM_CHOICES: usize = 32;

/// Nombre maximal de scripts déclarés par un agent.
pub const MAX_DECLARED_SCRIPTS: usize = 64;

/// Nombre maximal de services autorisés déclarés par un agent.
pub const MAX_DECLARED_SERVICES: usize = 64;

/// Délai d'exécution par défaut d'un script, en secondes.
pub const SCRIPT_DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Délai d'exécution maximal d'un script, quel que soit son manifeste.
pub const SCRIPT_MAX_TIMEOUT_SECS: u64 = 300;

/// Nom d'une action locale (script) : minuscules, chiffres, `-` et `_`, 64
/// caractères au plus, jamais en tête un `-` ou un `.`.
///
/// La règle est volontairement plus stricte qu'un nom de fichier : le nom ne
/// contient ni `/`, ni `\`, ni `.`, ce qui ferme toute traversée de chemin
/// (`../`, chemin absolu, lecteur Windows) avant même que l'agent ne compare
/// le nom à la liste de son dossier.
pub fn validate_action_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_ok =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'));
    if !first_ok || !rest_ok || name.len() > 64 {
        return Err(format!(
            "invalid action name '{}': lowercase letters, digits, '-' and '_' only (64 max)",
            printable(name)
        ));
    }
    Ok(())
}

/// Nom d'un service systemd (`nginx`, `nginx.service`, `getty@tty1.service`)
/// ou Windows (`W32Time`, `MSSQL$SQLEXPRESS`).
///
/// Jamais de `-` ni de `/` en tête : le nom est passé en argument à
/// `systemctl` ou `net`, qui le prendraient pour une option. Ni espace ni
/// guillemet : aucun service légitime n'en a besoin, et c'est autant de
/// surprises en moins pour les outils qui le recevront.
pub fn validate_service_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_alphanumeric());
    let rest_ok =
        chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@' | ':' | '$'));
    if !first_ok || !rest_ok || name.len() > 128 || name.contains("..") {
        return Err(format!("invalid service name '{}'", printable(name)));
    }
    Ok(())
}

/// Clé d'un paramètre de script : `[a-z][a-z0-9_]{0,31}`.
pub fn validate_param_key(key: &str) -> Result<(), String> {
    let mut chars = key.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase());
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !first_ok || !rest_ok || key.len() > 32 {
        return Err(format!("invalid parameter name '{}'", printable(key)));
    }
    Ok(())
}

/// Valeur d'un paramètre : 128 octets au plus, lettres, chiffres et `. _ - : @ / +`
/// seulement, jamais de `-` en tête.
///
/// La valeur est passée au script sous la forme d'un seul argument
/// `--clé=valeur`, jamais par un interpréteur : elle ne peut ni ouvrir une
/// option, ni s'échapper dans une seconde commande. La liste de caractères
/// fermée est une ceinture en plus des bretelles — un script mal écrit qui la
/// réinjecterait dans un shell n'y trouverait ni espace, ni guillemet, ni `;`.
pub fn validate_param_value(value: &str) -> Result<(), String> {
    let ok_chars = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '@' | '/' | '+'));
    if value.is_empty() || value.len() > MAX_PARAM_VALUE_LEN || !ok_chars || value.starts_with('-')
    {
        return Err(format!("invalid parameter value '{}'", printable(value)));
    }
    Ok(())
}

/// Valide un jeu de paramètres complet (nombre, clés, valeurs).
pub fn validate_params(params: &BTreeMap<String, String>) -> Result<(), String> {
    if params.len() > MAX_SCRIPT_PARAMS {
        return Err(format!("too many parameters (max {MAX_SCRIPT_PARAMS})"));
    }
    for (key, value) in params {
        validate_param_key(key)?;
        validate_param_value(value)?;
    }
    Ok(())
}

/// Un nonce tel que le serveur les forge : 32 caractères hexadécimaux.
pub fn validate_nonce(nonce: &str) -> Result<(), String> {
    if nonce.len() != 32 || !nonce.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("missing or malformed nonce".to_string());
    }
    Ok(())
}

/// Une chaîne reçue, rendue sûre à afficher dans un message d'erreur : bornée
/// et débarrassée des caractères de contrôle.
fn printable(text: &str) -> String {
    text.chars().take(64).map(|c| if c.is_control() { '?' } else { c }).collect()
}

/// Ce qu'un agent déclare de ses actions de remédiation, à chaque lot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationInfo {
    /// Faux tant que `actions: true` n'est pas posé dans agent.yaml : l'agent
    /// refuse alors tout redémarrage de service et tout script.
    #[serde(default)]
    pub enabled: bool,
    /// Services que l'agent accepte de redémarrer (`action_services`).
    #[serde(default)]
    pub services: Vec<String>,
    /// Scripts déclarés dans le dossier d'actions de la machine.
    #[serde(default)]
    pub scripts: Vec<DeclaredScript>,
}

/// Un script déclaré localement, tel que l'agent le décrit au serveur : son
/// nom, une description, et les valeurs permises de chacun de ses paramètres.
/// Jamais sa commande.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredScript {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Paramètre → valeurs permises. Un paramètre absent d'ici est refusé.
    #[serde(default)]
    pub params: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub timeout_secs: u64,
}

impl DeclaredScript {
    /// Vérifie des paramètres contre le schéma déclaré par l'agent : chaque clé
    /// doit être déclarée, chaque valeur figurer dans la liste permise, et
    /// chaque paramètre déclaré être fourni.
    pub fn check_params(&self, params: &BTreeMap<String, String>) -> Result<(), String> {
        validate_params(params)?;
        for (key, value) in params {
            let Some(allowed) = self.params.get(key) else {
                return Err(format!("parameter '{key}' is not declared by script '{}'", self.name));
            };
            if !allowed.iter().any(|choice| choice == value) {
                return Err(format!(
                    "value '{value}' is not allowed for parameter '{key}' of script '{}'",
                    self.name
                ));
            }
        }
        if let Some(missing) = self.params.keys().find(|key| !params.contains_key(*key)) {
            return Err(format!("parameter '{missing}' of script '{}' is missing", self.name));
        }
        Ok(())
    }
}

impl RemediationInfo {
    /// Borne et assainit une déclaration reçue d'un agent : un binaire modifié
    /// ne doit pas pouvoir faire ranger des mégaoctets en base.
    pub fn sanitized(mut self) -> Self {
        self.services.retain(|name| validate_service_name(name).is_ok());
        self.services.truncate(MAX_DECLARED_SERVICES);
        self.scripts.retain(|script| validate_action_name(&script.name).is_ok());
        self.scripts.truncate(MAX_DECLARED_SCRIPTS);
        for script in &mut self.scripts {
            script.description = script.description.chars().take(200).collect();
            script.params.retain(|key, _| validate_param_key(key).is_ok());
            while script.params.len() > MAX_SCRIPT_PARAMS {
                let last = script.params.keys().next_back().cloned().unwrap_or_default();
                script.params.remove(&last);
            }
            for choices in script.params.values_mut() {
                choices.retain(|value| validate_param_value(value).is_ok());
                choices.truncate(MAX_PARAM_CHOICES);
            }
            script.timeout_secs = script.timeout_secs.min(SCRIPT_MAX_TIMEOUT_SECS);
        }
        self
    }

    pub fn script(&self, name: &str) -> Option<&DeclaredScript> {
        self.scripts.iter().find(|script| script.name == name)
    }

    /// Vrai si ce service figure dans la liste autorisée (`*` : tous).
    pub fn allows_service(&self, name: &str) -> bool {
        service_allowed(&self.services, name)
    }
}

/// Vrai si `name` figure dans la liste `allowed` (`*` autorise tout service).
/// La comparaison ignore un suffixe `.service` des deux côtés : `nginx` et
/// `nginx.service` désignent la même unité.
pub fn service_allowed(allowed: &[String], name: &str) -> bool {
    let bare = |s: &str| s.strip_suffix(".service").unwrap_or(s).to_ascii_lowercase();
    let wanted = bare(name);
    allowed.iter().any(|entry| entry == "*" || bare(entry) == wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_names_cannot_traverse_paths() {
        for good in ["clear-cache", "restart_app", "a", "x1"] {
            assert!(validate_action_name(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "../etc/passwd",
            "..",
            "a/b",
            "a\\b",
            "/bin/sh",
            "C:evil",
            "-rf",
            ".hidden",
            "Clear",
            "name.yaml",
            "a b",
            "a\0b",
            &"x".repeat(65),
        ] {
            assert!(validate_action_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn service_names_cannot_become_options() {
        for good in ["nginx", "nginx.service", "getty@tty1.service", "MSSQL$SQLEXPRESS", "W32Time"]
        {
            assert!(validate_service_name(good).is_ok(), "{good}");
        }
        for bad in ["", "-H", "--all", "/stop", "a b", "a;b", "a\"b", "../x", "a\nb"] {
            assert!(validate_service_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn parameter_values_cannot_inject_arguments() {
        for good in ["high", "eth0", "10.0.0.1", "user@host", "/var/log", "v1+2"] {
            assert!(validate_param_value(good).is_ok(), "{good}");
        }
        for bad in ["", "--force", "-x", "a b", "a;rm", "$(id)", "`id`", "a|b", "a'b", "a\nb"] {
            assert!(validate_param_value(bad).is_err(), "{bad:?}");
        }
        assert!(validate_param_value(&"a".repeat(MAX_PARAM_VALUE_LEN + 1)).is_err());
        assert!(validate_param_key("Level").is_err());
        assert!(validate_param_key("level_2").is_ok());
    }

    #[test]
    fn script_parameters_are_checked_against_the_declared_schema() {
        let script = DeclaredScript {
            name: "flush".into(),
            params: BTreeMap::from([("level".to_string(), vec!["low".into(), "high".into()])]),
            ..DeclaredScript::default()
        };
        let ok = BTreeMap::from([("level".to_string(), "high".to_string())]);
        assert!(script.check_params(&ok).is_ok());
        let wrong = BTreeMap::from([("level".to_string(), "max".to_string())]);
        assert!(script.check_params(&wrong).is_err());
        let extra = BTreeMap::from([
            ("level".to_string(), "low".to_string()),
            ("other".to_string(), "x".to_string()),
        ]);
        assert!(script.check_params(&extra).is_err());
        assert!(script.check_params(&BTreeMap::new()).is_err(), "missing parameter");
    }

    #[test]
    fn a_declaration_is_bounded() {
        let info = RemediationInfo {
            enabled: true,
            services: (0..200).map(|i| format!("svc{i}")).chain(["-bad".to_string()]).collect(),
            scripts: vec![DeclaredScript {
                name: "ok".into(),
                description: "d".repeat(1000),
                params: BTreeMap::from([("k".to_string(), vec!["v".into(), "bad value".into()])]),
                timeout_secs: 99_999,
            }],
        }
        .sanitized();
        assert_eq!(info.services.len(), MAX_DECLARED_SERVICES);
        assert_eq!(info.scripts[0].description.len(), 200);
        assert_eq!(info.scripts[0].params["k"], vec!["v".to_string()]);
        assert_eq!(info.scripts[0].timeout_secs, SCRIPT_MAX_TIMEOUT_SECS);
    }

    #[test]
    fn service_allow_list_ignores_the_unit_suffix() {
        let list = vec!["nginx.service".to_string()];
        assert!(service_allowed(&list, "nginx"));
        assert!(service_allowed(&list, "NGINX.service"));
        assert!(!service_allowed(&list, "sshd"));
        assert!(service_allowed(&["*".to_string()], "anything"));
        assert!(!service_allowed(&[], "nginx"));
    }

    #[test]
    fn nonces_are_32_hex_characters() {
        assert!(validate_nonce("0123456789abcdef0123456789abcdef").is_ok());
        assert!(validate_nonce("short").is_err());
        assert!(validate_nonce(&"z".repeat(32)).is_err());
    }
}
