//! Mise à jour automatique de l'agent lui-même.
//!
//! À ne pas confondre avec la mise à jour des conteneurs Docker
//! (`CMD_CONTAINER_UPDATE`) : il s'agit ici du binaire `dumbmonit-agent`.
//!
//! Le dialogue tient en deux champs, ajoutés de façon à ce qu'aucun des deux
//! côtés n'ait à connaître la version de l'autre :
//!
//! - l'agent décrit, dans son identité ([`crate::AgentIdentity::self_update`]),
//!   l'empreinte du binaire qui tourne, sa plateforme, ce qui l'empêcherait de se
//!   remplacer (conteneur, emplacement en lecture seule…) et l'issue de sa
//!   dernière tentative ;
//! - le serveur, dans l'accusé de réception d'un lot ([`crate::PushAck::agent_update`]),
//!   propose le binaire de **son propre build** pour cette plateforme — version,
//!   empreinte SHA-256 et signature Ed25519 — quand ses réglages l'autorisent
//!   pour cette machine (activé, fenêtre, déploiement progressif).
//!
//! Un agent antérieur ignore l'offre ; un serveur antérieur n'en fait pas, et
//! ignore la description. L'empreinte, et non la seule version, décide : un
//! serveur `:edge` distribue l'agent de son build, à version égale.

use serde::{Deserialize, Serialize};

/// Téléchargement du binaire proposé : `GET {UPDATE_DOWNLOAD_PATH}/{file}`,
/// sous le jeton d'enrôlement, le secret de liaison et la clé de la machine,
/// comme les autres routes d'agent — servi sur le canal chiffré comme sur
/// l'ancien.
pub const UPDATE_DOWNLOAD_PATH: &str = "/api/agent/update/download";

/// Suffixe de la signature détachée d'un binaire (`/download/<nom>.sig`) :
/// 64 octets bruts, Ed25519 sur le binaire entier.
pub const SIGNATURE_SUFFIX: &str = ".sig";

/// Délai laissé au nouveau binaire pour faire accepter un lot par le serveur,
/// faute de quoi l'ancien est rétabli.
pub const UPDATE_TRIAL_SECS: u64 = 120;

/// Préfixe commun des binaires de l'agent : `dumbmonit-agent-linux-x86_64`.
pub const AGENT_FILE_PREFIX: &str = "dumbmonit-agent-";

/// Nom du binaire d'une plateforme (`linux-x86_64`, `windows-x86_64`…), tel
/// que le serveur le livre sur `/download/…`.
pub fn agent_file_name(platform: &str) -> String {
    if platform.starts_with("windows-") {
        format!("{AGENT_FILE_PREFIX}{platform}.exe")
    } else {
        format!("{AGENT_FILE_PREFIX}{platform}")
    }
}

/// Ce qui permet, ou empêche, un agent de se remplacer lui-même.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfUpdateAbility {
    /// L'agent peut se remplacer et redémarrer.
    #[default]
    Ready,
    /// Agent en conteneur : c'est l'image qu'il faut mettre à jour.
    Container,
    /// Le dossier du binaire n'est pas inscriptible par l'agent (unité
    /// systemd d'avant la mise à jour automatique) : relancer une fois la
    /// commande d'installation.
    ReadOnly,
    /// `self_update: false` dans la configuration de l'agent.
    Disabled,
    /// Plateforme que ce binaire ne sait pas mettre à jour.
    Unsupported,
}

impl SelfUpdateAbility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Container => "container",
            Self::ReadOnly => "read_only",
            Self::Disabled => "disabled",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Issue de la dernière tentative de mise à jour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfUpdateOutcome {
    /// Le nouveau binaire tourne et a fait accepter un lot.
    Succeeded,
    /// La tentative a échoué avant le remplacement (téléchargement, empreinte,
    /// signature, binaire qui ne démarre pas) : l'ancien n'a jamais cessé de
    /// tourner.
    Failed,
    /// Le nouveau binaire n'a rien fait accepter à temps, ou s'arrêtait au
    /// démarrage : l'ancien a été rétabli.
    RolledBack,
    /// Refusée sans rien télécharger : binaire non signé, ou plus ancien que
    /// celui qui tourne sans que le serveur l'ait demandé.
    Refused,
}

impl SelfUpdateOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
            Self::Refused => "refused",
        }
    }

    /// Vrai si la même version ne doit pas être retentée d'elle-même.
    pub fn is_failure(self) -> bool {
        !matches!(self, Self::Succeeded)
    }
}

/// Compte rendu de la dernière tentative, répété dans chaque lot jusqu'à la
/// suivante : un serveur redémarré entre-temps l'apprend quand même.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfUpdateReport {
    pub outcome: SelfUpdateOutcome,
    pub from_version: String,
    pub to_version: String,
    /// Empreinte du binaire visé : c'est elle que le serveur ne repropose plus
    /// après un échec.
    pub to_sha256: String,
    /// Cause d'un échec, en anglais, pour l'interface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Instant de l'issue, en millisecondes depuis l'époque Unix.
    pub at_ms: i64,
}

/// Ce que l'agent dit de lui-même pour sa mise à jour.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SelfUpdateInfo {
    /// SHA-256 (hexadécimal) du binaire qui tourne.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Plateforme du binaire : `linux-x86_64`, `windows-x86_64`…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default)]
    pub ability: SelfUpdateAbility,
    /// Vrai si ce binaire connaît au moins une clé de signature des versions.
    #[serde(default)]
    pub signing_keys: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<SelfUpdateReport>,
}

/// Binaire que le serveur propose à l'agent, dans l'accusé de réception.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentUpdateOffer {
    /// Version du binaire proposé (celle du serveur).
    pub version: String,
    /// Commit du build, quand il est connu (`DUMBMONIT_BUILD`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    /// Nom du fichier à télécharger sous [`UPDATE_DOWNLOAD_PATH`].
    pub file: String,
    /// SHA-256 (hexadécimal) attendu.
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Signature Ed25519 détachée du binaire, en base64. Absente : le build
    /// n'a pas été signé (CI sans clé, build local).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Réglage « accepter les builds non signés (développement) ». L'agent
    /// n'en tient compte que sur le canal chiffré, serveur authentifié.
    #[serde(default)]
    pub allow_unsigned: bool,
    /// Le serveur demande explicitement un retour en arrière.
    #[serde(default)]
    pub allow_downgrade: bool,
    /// Demande explicite (« Update now ») : une version qui a déjà échoué sur
    /// cette machine est retentée une fois.
    #[serde(default)]
    pub force: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_names_are_those_the_server_ships() {
        assert_eq!(agent_file_name("linux-x86_64"), "dumbmonit-agent-linux-x86_64");
        assert_eq!(agent_file_name("windows-x86_64"), "dumbmonit-agent-windows-x86_64.exe");
    }

    #[test]
    fn an_offer_from_an_older_server_shape_is_still_readable() {
        let offer: AgentUpdateOffer = serde_json::from_str(
            r#"{"version":"0.14.0","file":"dumbmonit-agent-linux-x86_64","sha256":"ab"}"#,
        )
        .unwrap();
        assert!(!offer.allow_unsigned && !offer.allow_downgrade && !offer.force);
        assert!(offer.signature.is_none());
    }

    #[test]
    fn the_description_of_an_agent_round_trips() {
        let info = SelfUpdateInfo {
            sha256: Some("cd".into()),
            platform: Some("linux-aarch64".into()),
            ability: SelfUpdateAbility::Container,
            signing_keys: true,
            last: Some(SelfUpdateReport {
                outcome: SelfUpdateOutcome::RolledBack,
                from_version: "0.13.5".into(),
                to_version: "0.14.0".into(),
                to_sha256: "ef".into(),
                reason: Some("no batch accepted within 120 s".into()),
                at_ms: 1,
            }),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains(r#""ability":"container""#));
        assert!(json.contains(r#""outcome":"rolled_back""#));
        assert_eq!(serde_json::from_str::<SelfUpdateInfo>(&json).unwrap(), info);
        assert!(SelfUpdateOutcome::Refused.is_failure());
        assert!(!SelfUpdateOutcome::Succeeded.is_failure());
    }
}
