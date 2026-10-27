//! Profils déclaratifs de sauvegarde : comment parler à un modèle d'équipement.
//!
//! Un profil dit par où passer (SSH interactif, commande SSH unique, API
//! HTTP), comment reconnaître l'invite, couper la pagination, passer en mode
//! privilégié, et quelles commandes envoyer. Il porte aussi les règles qui
//! servent après coup : lignes volatiles (ignorées pour décider qu'une
//! configuration a changé) et secrets à masquer à l'affichage.
//!
//! Les profils sont livrés avec le produit (`profiles/netconfig/*.yaml`) et
//! incorporés au binaire : aucune commande ne vient jamais de l'interface ni
//! d'un champ saisi par l'utilisateur.

use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

/// Façon de récupérer la configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    /// Session interactive avec terminal : invite, pagination, `enable`.
    SshShell,
    /// Une commande SSH par élément de `commands`, sans terminal.
    SshExec,
    /// Une requête HTTPS `GET`.
    Http,
}

/// Une réponse automatique : quand la dernière ligne reçue correspond à
/// `expect`, `send` est envoyé tel quel.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub expect: String,
    pub send: String,
}

/// Passage en mode privilégié.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enable {
    pub command: String,
    /// Invite qui signale le mode non privilégié (`>` en fin de ligne).
    pub unprivileged: String,
    /// Demande du mot de passe `enable`.
    pub password_prompt: String,
}

/// Authentification d'une requête HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpAuth {
    /// Authentification HTTP basique (identifiant / mot de passe, ou clé et
    /// secret d'API).
    Basic,
    /// Jeton d'API dans l'en-tête `header`.
    Header,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpSpec {
    /// Chemin et requête, fixes : aucun champ saisi n'y est interpolé.
    pub path: String,
    pub auth: HttpAuth,
    #[serde(default)]
    pub header: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub label: String,
    pub transport: Transport,
    /// Port par défaut ; sinon 22 en SSH, 443 en HTTP.
    #[serde(default)]
    pub port: Option<u16>,
    /// Invite du terminal (dernière ligne reçue), en mode interactif.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Réponses aux questions posées avant la première invite.
    #[serde(default)]
    pub login_replies: Vec<Reply>,
    #[serde(default)]
    pub enable: Option<Enable>,
    /// Commandes dont la sortie est jetée (pagination, largeur du terminal).
    #[serde(default)]
    pub setup: Vec<String>,
    /// Invite de pagination et la touche qui fait avancer.
    #[serde(default)]
    pub more: Option<Reply>,
    /// Commandes dont la sortie forme la configuration.
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub exit: Option<String>,
    #[serde(default)]
    pub http: Option<HttpSpec>,
    /// Lignes retirées de la sortie avant enregistrement (bavardage).
    #[serde(default)]
    pub strip: Vec<String>,
    /// Lignes ignorées pour décider qu'une configuration a changé.
    #[serde(default)]
    pub volatile: Vec<String>,
    /// Expressions dont le groupe nommé `secret` est masqué à l'affichage.
    #[serde(default)]
    pub secrets: Vec<String>,
}

impl Profile {
    pub fn default_port(&self) -> u16 {
        self.port.unwrap_or(match self.transport {
            Transport::Http => 443,
            Transport::SshShell | Transport::SshExec => 22,
        })
    }

    pub fn uses_ssh(&self) -> bool {
        self.transport != Transport::Http
    }

    /// Vérifie tout ce qu'un profil promet : expressions valides, groupe
    /// `secret` présent, commandes et réglages propres au transport.
    pub fn validate(&self) -> Result<(), String> {
        let id = &self.id;
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("profile id \"{id}\" must be made of [a-z0-9_]"));
        }
        let mut patterns: Vec<&str> = Vec::new();
        patterns.extend(self.prompt.as_deref());
        patterns.extend(self.login_replies.iter().map(|r| r.expect.as_str()));
        patterns.extend(self.more.iter().map(|r| r.expect.as_str()));
        if let Some(enable) = &self.enable {
            patterns.push(&enable.unprivileged);
            patterns.push(&enable.password_prompt);
        }
        patterns.extend(self.strip.iter().map(String::as_str));
        patterns.extend(self.volatile.iter().map(String::as_str));
        for pattern in &patterns {
            Regex::new(pattern).map_err(|e| format!("{id}: invalid expression {pattern}: {e}"))?;
        }
        for pattern in &self.secrets {
            let re = Regex::new(pattern)
                .map_err(|e| format!("{id}: invalid expression {pattern}: {e}"))?;
            if !re.capture_names().any(|name| name == Some("secret")) {
                return Err(format!("{id}: secret rule {pattern} has no (?P<secret>…) group"));
            }
        }
        match self.transport {
            Transport::SshShell => {
                if self.prompt.is_none() {
                    return Err(format!("{id}: an interactive profile needs a prompt"));
                }
                if self.commands.is_empty() {
                    return Err(format!("{id}: no command"));
                }
            }
            Transport::SshExec => {
                if self.commands.is_empty() {
                    return Err(format!("{id}: no command"));
                }
            }
            Transport::Http => {
                let Some(http) = &self.http else {
                    return Err(format!("{id}: an HTTP profile needs an http section"));
                };
                if !http.path.starts_with('/') {
                    return Err(format!("{id}: the HTTP path must start with /"));
                }
                if http.auth == HttpAuth::Header && http.header.is_none() {
                    return Err(format!("{id}: header authentication needs a header name"));
                }
            }
        }
        Ok(())
    }

    /// Règles de masquage compilées.
    pub fn secret_rules(&self) -> Vec<Regex> {
        compile(&self.secrets)
    }

    /// Règles des lignes volatiles compilées.
    pub fn volatile_rules(&self) -> Vec<Regex> {
        compile(&self.volatile)
    }

    /// Règles des lignes retirées compilées.
    pub fn strip_rules(&self) -> Vec<Regex> {
        compile(&self.strip)
    }
}

/// Compile une liste d'expressions déjà validées ; une expression invalide
/// (impossible pour un profil livré, vérifié par les tests) est écartée.
fn compile(patterns: &[String]) -> Vec<Regex> {
    patterns.iter().filter_map(|pattern| Regex::new(pattern).ok()).collect()
}

/// Les profils livrés, incorporés à la compilation : un fichier renommé casse
/// la construction plutôt que de vider le catalogue en production.
const EMBEDDED: &[(&str, &str)] = &[
    ("cisco-ios.yaml", include_str!("../../../../profiles/netconfig/cisco-ios.yaml")),
    ("cisco-nxos.yaml", include_str!("../../../../profiles/netconfig/cisco-nxos.yaml")),
    ("aruba-hpe.yaml", include_str!("../../../../profiles/netconfig/aruba-hpe.yaml")),
    (
        "mikrotik-routeros.yaml",
        include_str!("../../../../profiles/netconfig/mikrotik-routeros.yaml"),
    ),
    ("juniper-junos.yaml", include_str!("../../../../profiles/netconfig/juniper-junos.yaml")),
    ("fortigate.yaml", include_str!("../../../../profiles/netconfig/fortigate.yaml")),
    ("pfsense.yaml", include_str!("../../../../profiles/netconfig/pfsense.yaml")),
    ("opnsense.yaml", include_str!("../../../../profiles/netconfig/opnsense.yaml")),
    ("opnsense-api.yaml", include_str!("../../../../profiles/netconfig/opnsense-api.yaml")),
    ("ubiquiti-edgeos.yaml", include_str!("../../../../profiles/netconfig/ubiquiti-edgeos.yaml")),
    ("unifi-device.yaml", include_str!("../../../../profiles/netconfig/unifi-device.yaml")),
    ("stormshield.yaml", include_str!("../../../../profiles/netconfig/stormshield.yaml")),
    ("paloalto-api.yaml", include_str!("../../../../profiles/netconfig/paloalto-api.yaml")),
];

/// Identifiants des profils livrés, dans l'ordre du catalogue : la liste de
/// choix du formulaire (`api/collectors.rs`) doit la reprendre.
pub const PROFILE_IDS: &[&str] = &[
    "cisco_ios",
    "cisco_nxos",
    "aruba_hpe",
    "mikrotik_routeros",
    "juniper_junos",
    "fortigate",
    "pfsense",
    "opnsense",
    "opnsense_api",
    "ubiquiti_edgeos",
    "unifi_device",
    "stormshield",
    "paloalto_api",
];

/// Le catalogue, lu une seule fois. Un profil illisible ou invalide est
/// écarté avec un avertissement (les tests garantissent qu'aucun ne l'est).
pub fn catalog() -> &'static [Profile] {
    static CATALOG: OnceLock<Vec<Profile>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut profiles = Vec::new();
        for (file, source) in EMBEDDED {
            match crate::yaml::from_str::<Profile>(source) {
                Ok(profile) => match profile.validate() {
                    Ok(()) => profiles.push(profile),
                    Err(error) => tracing::warn!(file, %error, "netconfig profile refused"),
                },
                Err(error) => tracing::warn!(file, %error, "netconfig profile unreadable"),
            }
        }
        profiles
    })
}

/// Le profil `id`, s'il existe.
pub fn find(id: &str) -> Option<&'static Profile> {
    catalog().iter().find(|profile| profile.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_profile_is_valid_and_listed() {
        assert_eq!(catalog().len(), EMBEDDED.len(), "a profile was refused");
        let ids: Vec<&str> = catalog().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, PROFILE_IDS);
        for profile in catalog() {
            assert!(!profile.secrets.is_empty(), "{} masks nothing", profile.id);
        }
    }

    #[test]
    fn a_secret_rule_without_its_group_is_refused() {
        let profile: Profile = crate::yaml::from_str(
            "id: x\nlabel: X\ntransport: ssh_exec\ncommands: [show]\nsecrets: ['pw (\\S+)']\n",
        )
        .unwrap();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let parsed = crate::yaml::from_str::<Profile>(
            "id: x\nlabel: X\ntransport: ssh_exec\ncommands: [show]\nrun_anything: true\n",
        );
        assert!(parsed.is_err());
    }
}
