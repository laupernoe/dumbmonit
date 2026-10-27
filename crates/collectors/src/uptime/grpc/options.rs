//! Réglages de la sonde gRPC.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError, Target};

use crate::uptime::{guard, tags};

/// Port par défaut d'un service gRPC en clair : celui des exemples de la
/// documentation gRPC. Il n'existe pas de port officiel.
pub const DEFAULT_PORT: u16 = 50051;
/// Port par défaut quand la connexion est chiffrée.
pub const DEFAULT_TLS_PORT: u16 = 443;

/// Longueur maximale du nom de service, qui part dans un message protobuf.
const MAX_SERVICE_LEN: usize = 253;

#[derive(Debug, Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    /// Nom envoyé en SNI, vérifié dans le certificat et annoncé en `:authority`.
    pub server_name: String,
    pub allow_untrusted: bool,
    pub allow_private: bool,
    /// Service dont on demande la santé. Vide : le serveur dans son ensemble.
    pub service: String,
    pub timeout: Duration,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let tls = tags::parse_bool(target, "tls", false)?;
        let default_port = if tls { DEFAULT_TLS_PORT } else { DEFAULT_PORT };
        // Port sentinelle : l'adresse peut en porter un, sinon c'est le
        // chiffrement qui décide.
        let (host, port_in_address) = tags::split_host_port(&target.address, 0)?;
        let port = match tags::parse_u32(target, "port", u32::from(port_in_address), 0..=65_535)? {
            0 => default_port,
            port => port as u16,
        };

        if !matches!(target.credential, Credential::None) {
            return Err(ProbeError::Config(
                "this check accepts no credential: the gRPC health protocol is unauthenticated"
                    .to_string(),
            ));
        }

        let service = tags::tag(target, "service").unwrap_or_default().to_string();
        if service.len() > MAX_SERVICE_LEN || service.chars().any(char::is_control) {
            return Err(ProbeError::Config(format!(
                "\"service\" must be a service name of at most {MAX_SERVICE_LEN} characters, \
                 for example \"my.package.MyService\""
            )));
        }

        Ok(Self {
            server_name: tags::tag(target, "server_name")
                .map(str::to_string)
                .unwrap_or_else(|| host.clone()),
            host,
            port,
            tls,
            allow_untrusted: tags::parse_bool(target, "insecure_tls", false)?,
            allow_private: guard::allowed(target)?,
            service,
            timeout: tags::parse_timeout(target, "timeout_seconds")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn options(address: &str, tags: &[(&str, &str)]) -> Result<Options, ProbeError> {
        Options::from_target(&cible("grpc", address, tags))
    }

    #[test]
    fn le_port_par_defaut_suit_le_chiffrement() {
        assert_eq!(options("api.lan", &[]).unwrap().port, 50051);
        assert_eq!(options("api.lan", &[("tls", "true")]).unwrap().port, 443);
        assert_eq!(options("api.lan:9000", &[("tls", "true")]).unwrap().port, 9000);
        assert_eq!(options("api.lan", &[("port", "8443")]).unwrap().port, 8443);
    }

    #[test]
    fn sans_service_on_interroge_le_serveur_entier() {
        let lues = options("api.lan", &[]).unwrap();
        assert_eq!(lues.service, "");
        assert!(!lues.tls && !lues.allow_untrusted && !lues.allow_private);
        assert_eq!(lues.server_name, "api.lan");
    }

    #[test]
    fn le_service_et_le_nom_de_serveur_se_lisent() {
        let tags = [("service", " pkg.Echo "), ("server_name", "grpc.example.com")];
        let lues = options("10.0.0.5", &tags).unwrap();
        assert_eq!(lues.service, "pkg.Echo");
        assert_eq!(lues.server_name, "grpc.example.com");
        assert_eq!(lues.host, "10.0.0.5");
    }

    #[test]
    fn un_nom_de_service_inutilisable_est_une_erreur_de_configuration() {
        let trop_long = "s".repeat(300);
        for invalide in [trop_long.as_str(), "a\nb"] {
            let error = options("api.lan", &[("service", invalide)]).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)));
            assert!(!error.means_down());
        }
    }

    #[test]
    fn un_identifiant_est_refuse() {
        let mut target = cible("grpc", "api.lan", &[]);
        target.credential = Credential::ApiToken { token: "t".into() };
        let error = Options::from_target(&target).unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
