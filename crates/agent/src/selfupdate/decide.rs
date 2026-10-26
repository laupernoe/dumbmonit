//! Ce que l'agent fait d'une offre de mise à jour : fonction pure.
//!
//! Le serveur décide *quand* proposer (réglages, fenêtre, déploiement
//! progressif) ; l'agent décide s'il *accepte* : jamais un binaire qu'il ne
//! peut pas authentifier, jamais un retour en arrière que le serveur n'a pas
//! demandé, jamais deux fois de suite une version qui a échoué ici.

use dumbmonit_proto::{AgentUpdateOffer, SelfUpdateAbility};

/// Ce que l'agent sait de lui-même au moment de décider.
#[derive(Debug, Clone)]
pub struct Current<'a> {
    pub version: &'a str,
    pub sha256: Option<&'a str>,
    pub ability: SelfUpdateAbility,
    /// Empreintes qui ont déjà échoué sur cette machine.
    pub failed: &'a [String],
    /// Vrai si l'offre est arrivée par le canal chiffré, serveur authentifié
    /// par son autorité épinglée.
    pub encrypted: bool,
    /// Vrai si ce binaire connaît au moins une clé de publication.
    pub has_keys: bool,
}

/// Décision sur une offre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Télécharger, vérifier, installer.
    Install,
    /// Rien à faire, rien à dire au serveur (déjà à jour, ou incapable de se
    /// remplacer — ce que l'identité dit déjà).
    Skip(&'static str),
    /// Refus à rapporter au serveur, pour l'interface.
    Refuse(String),
}

/// Décide du sort d'une offre.
pub fn decide(offer: &AgentUpdateOffer, current: &Current<'_>) -> Plan {
    if current.ability != SelfUpdateAbility::Ready {
        return Plan::Skip("this agent cannot replace itself");
    }
    if current.sha256.is_some_and(|sha| sha.eq_ignore_ascii_case(&offer.sha256)) {
        return Plan::Skip("already running this build");
    }
    if offer.sha256.len() != 64 || !offer.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Plan::Refuse("the server offered an update without a valid SHA-256".to_string());
    }
    if dumbmonit_proto::is_older(&offer.version, current.version) && !offer.allow_downgrade {
        return Plan::Refuse(format!(
            "refusing to downgrade from {} to {} (not requested by the server)",
            current.version, offer.version
        ));
    }
    if !offer.force && current.failed.iter().any(|sha| sha.eq_ignore_ascii_case(&offer.sha256)) {
        return Plan::Skip("this build already failed on this machine");
    }
    let signed = offer.signature.as_deref().is_some_and(|s| !s.trim().is_empty());
    if !(signed && current.has_keys) {
        // Sans signature vérifiable, seule l'authentification du serveur par
        // le canal chiffré épinglé rend le réglage « builds non signés » digne
        // de foi : en clair, n'importe quel intermédiaire pourrait le poser.
        if !offer.allow_unsigned {
            let why = if signed {
                "this agent has no release signing key to check the signature"
            } else {
                "the build is not signed"
            };
            return Plan::Refuse(format!(
                "{why}; enable 'Accept unsigned builds (development)' on the server to allow it"
            ));
        }
        if !current.encrypted {
            return Plan::Refuse(
                "unsigned builds are only accepted over the encrypted agent channel".to_string(),
            );
        }
    }
    Plan::Install
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(version: &str, sha: char) -> AgentUpdateOffer {
        AgentUpdateOffer {
            version: version.to_string(),
            build: None,
            file: "dumbmonit-agent-linux-x86_64".to_string(),
            sha256: sha.to_string().repeat(64),
            size: None,
            signature: Some("c2lnbmF0dXJl".to_string()),
            allow_unsigned: false,
            allow_downgrade: false,
            force: false,
        }
    }

    fn current(failed: &[String]) -> Current<'_> {
        Current {
            version: "0.13.5",
            sha256: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            ability: SelfUpdateAbility::Ready,
            failed,
            encrypted: true,
            has_keys: true,
        }
    }

    #[test]
    fn a_newer_signed_build_is_installed() {
        assert_eq!(decide(&offer("0.14.0", 'b'), &current(&[])), Plan::Install);
    }

    #[test]
    fn the_build_decides_not_only_the_version() {
        // Serveur :edge : même version, autre build — l'agent se met au niveau.
        assert_eq!(decide(&offer("0.13.5", 'b'), &current(&[])), Plan::Install);
        // Même empreinte : rien à faire, quelle que soit la version annoncée.
        assert!(matches!(decide(&offer("0.14.0", 'a'), &current(&[])), Plan::Skip(_)));
        let upper = AgentUpdateOffer { sha256: "A".repeat(64), ..offer("0.14.0", 'a') };
        assert!(matches!(decide(&upper, &current(&[])), Plan::Skip(_)), "casse indifférente");
    }

    #[test]
    fn no_downgrade_unless_the_server_asks_for_it() {
        let older = offer("0.13.4", 'b');
        let refused = decide(&older, &current(&[]));
        assert!(matches!(refused, Plan::Refuse(why) if why.contains("downgrade")));
        let asked = AgentUpdateOffer { allow_downgrade: true, ..older };
        assert_eq!(decide(&asked, &current(&[])), Plan::Install);
    }

    #[test]
    fn a_build_that_failed_here_is_not_retried_unless_forced() {
        let failed = vec!["b".repeat(64)];
        assert!(matches!(decide(&offer("0.14.0", 'b'), &current(&failed)), Plan::Skip(_)));
        let forced = AgentUpdateOffer { force: true, ..offer("0.14.0", 'b') };
        assert_eq!(decide(&forced, &current(&failed)), Plan::Install);
        // Une autre version reste proposable.
        assert_eq!(decide(&offer("0.14.1", 'c'), &current(&failed)), Plan::Install);
    }

    #[test]
    fn an_unsigned_build_needs_the_server_setting_and_the_encrypted_channel() {
        let unsigned = AgentUpdateOffer { signature: None, ..offer("0.14.0", 'b') };
        assert!(matches!(decide(&unsigned, &current(&[])), Plan::Refuse(_)));

        let allowed = AgentUpdateOffer { allow_unsigned: true, ..unsigned };
        assert_eq!(decide(&allowed, &current(&[])), Plan::Install);
        // En clair, le réglage pourrait venir de n'importe qui : refus.
        let plain = Current { encrypted: false, ..current(&[]) };
        assert!(matches!(decide(&allowed, &plain), Plan::Refuse(why) if why.contains("encrypted")));
        // Signé mais sans clé pour vérifier : traité comme non signé.
        let no_key = Current { has_keys: false, ..current(&[]) };
        assert!(matches!(decide(&offer("0.14.0", 'b'), &no_key), Plan::Refuse(_)));
        // Un build signé passe aussi en clair : la signature suffit.
        assert_eq!(decide(&offer("0.14.0", 'b'), &plain), Plan::Install);
    }

    #[test]
    fn an_agent_that_cannot_replace_itself_does_nothing() {
        for ability in [
            SelfUpdateAbility::Container,
            SelfUpdateAbility::ReadOnly,
            SelfUpdateAbility::Disabled,
            SelfUpdateAbility::Unsupported,
        ] {
            let stuck = Current { ability, ..current(&[]) };
            assert!(matches!(decide(&offer("0.14.0", 'b'), &stuck), Plan::Skip(_)), "{ability:?}");
        }
    }

    #[test]
    fn a_malformed_checksum_is_refused() {
        let bad = AgentUpdateOffer { sha256: "zz".to_string(), ..offer("0.14.0", 'b') };
        assert!(matches!(decide(&bad, &current(&[])), Plan::Refuse(_)));
    }
}
