//! Réglages de la sonde RADIUS.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError, Target};

use super::packet::{MAX_PASSWORD, MAX_VALUE};
use crate::uptime::{guard, tags};

/// Port RADIUS d'authentification (UDP 1812).
pub const DEFAULT_PORT: u16 = 1812;

/// Identifiant de NAS annoncé par défaut. Un serveur RADIUS exige un
/// `NAS-Identifier` ou un `NAS-IP-Address` dans toute requête d'accès.
pub const DEFAULT_NAS_IDENTIFIER: &str = "dumbmonit";

/// Les réglages lus sur la cible. Pas de `Debug` : la structure porte le secret
/// partagé et un mot de passe.
#[derive(Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    pub shared_secret: String,
    pub username: String,
    pub password: String,
    pub nas_identifier: String,
    pub allow_private: bool,
    pub timeout: Duration,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (host, port_in_address) = tags::split_host_port(&target.address, DEFAULT_PORT)?;
        let port = tags::parse_u32(target, "port", u32::from(port_in_address), 1..=65_535)? as u16;

        let Credential::Radius { shared_secret, username, password } = &target.credential else {
            return Err(ProbeError::Config(
                "this check needs a RADIUS credential: the shared secret of the server and \
                 the user name and password of a test account"
                    .to_string(),
            ));
        };
        if shared_secret.is_empty() {
            return Err(ProbeError::Config("the RADIUS shared secret is empty".to_string()));
        }
        if username.is_empty() || username.len() > MAX_VALUE {
            return Err(ProbeError::Config(format!(
                "the test account name must hold 1 to {MAX_VALUE} bytes"
            )));
        }
        if password.len() > MAX_PASSWORD {
            return Err(ProbeError::Config(format!(
                "RADIUS limits the password to {MAX_PASSWORD} bytes"
            )));
        }

        let nas_identifier = tags::tag(target, "nas_identifier").unwrap_or(DEFAULT_NAS_IDENTIFIER);
        if nas_identifier.len() > MAX_VALUE {
            return Err(ProbeError::Config(format!(
                "\"nas_identifier\" is limited to {MAX_VALUE} bytes"
            )));
        }

        Ok(Self {
            host,
            port,
            shared_secret: shared_secret.clone(),
            username: username.clone(),
            password: password.clone(),
            nas_identifier: nas_identifier.to_string(),
            allow_private: guard::allowed(target)?,
            timeout: tags::parse_timeout(target, "timeout_seconds")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn credential() -> Credential {
        Credential::Radius {
            shared_secret: "testing123".into(),
            username: "probe".into(),
            password: "pw".into(),
        }
    }

    fn target(address: &str, tags: &[(&str, &str)]) -> Target {
        let mut target = cible("radius", address, tags);
        target.credential = credential();
        target
    }

    #[test]
    fn les_defauts_couvrent_un_serveur_sans_etiquette() {
        let options = Options::from_target(&target("radius.lan", &[])).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("radius.lan", DEFAULT_PORT));
        assert_eq!(options.nas_identifier, DEFAULT_NAS_IDENTIFIER);
        assert_eq!(options.username, "probe");
        assert!(!options.allow_private);
    }

    #[test]
    fn le_port_se_lit_dans_ladresse_ou_letiquette() {
        assert_eq!(Options::from_target(&target("radius.lan:11812", &[])).unwrap().port, 11812);
        let options = Options::from_target(&target("radius.lan", &[("port", "1645")])).unwrap();
        assert_eq!(options.port, 1645);
    }

    #[test]
    fn lidentifiant_de_nas_se_regle() {
        let options = Options::from_target(&target("r", &[("nas_identifier", "lab-1")])).unwrap();
        assert_eq!(options.nas_identifier, "lab-1");
        let trop_long = "n".repeat(300);
        let target = target("r", &[("nas_identifier", &trop_long)]);
        assert!(Options::from_target(&target).is_err());
    }

    #[test]
    fn sans_identifiant_radius_la_configuration_est_refusee() {
        for credential in [
            Credential::None,
            Credential::UsernamePassword { username: "u".into(), password: "p".into() },
        ] {
            let mut target = cible("radius", "radius.lan", &[]);
            target.credential = credential;
            let error = Options::from_target(&target).err().unwrap();
            assert!(matches!(error, ProbeError::Config(_)));
            assert!(!error.means_down());
            assert!(error.to_string().contains("shared secret"), "{error}");
        }
    }

    #[test]
    fn un_compte_ou_un_secret_inutilisable_est_une_erreur_de_configuration() {
        let invalides = [
            Credential::Radius {
                shared_secret: "".into(),
                username: "u".into(),
                password: "p".into(),
            },
            Credential::Radius {
                shared_secret: "s".into(),
                username: "".into(),
                password: "p".into(),
            },
            Credential::Radius {
                shared_secret: "s".into(),
                username: "u".into(),
                password: "p".repeat(129),
            },
        ];
        for credential in invalides {
            let mut target = cible("radius", "radius.lan", &[]);
            target.credential = credential;
            let error = Options::from_target(&target).err().unwrap();
            assert!(matches!(error, ProbeError::Config(_)), "{error}");
        }
    }
}
