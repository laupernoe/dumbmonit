//! Réglages lus sur les étiquettes de la cible.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError, Target};

use crate::uptime::tags::{parse_bool, parse_u32, split_host_port, tag};

/// Protection de la connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    /// TLS dès la connexion, port 636.
    Ldaps,
    /// Connexion en clair sur 389, chiffrée par StartTLS avant la liaison.
    StartTls,
    /// En clair de bout en bout : le mot de passe circule lisible. Refusé sans
    /// `allow_plaintext`.
    Plain,
}

impl Security {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ldaps => "ldaps",
            Self::StartTls => "starttls",
            Self::Plain => "plain",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::Ldaps => 636,
            Self::StartTls | Self::Plain => 389,
        }
    }
}

pub const DEFAULT_STALE_DAYS: u32 = 90;
pub const DEFAULT_INVENTORY_MINUTES: u32 = 15;
pub const DEFAULT_REQUEST_TIMEOUT_SECONDS: u32 = 8;

#[derive(Debug, Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub insecure_tls: bool,
    /// Autorité de certification à laquelle se fier (PEM ou base64).
    pub ca_cert: Option<String>,
    pub request_timeout: Duration,
    pub check_all_dcs: bool,
    pub stale_days_users: u32,
    pub stale_days_computers: u32,
    pub inventory_interval: Duration,
    pub bind_user: String,
    pub bind_password: String,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let security = match tag(target, "security").map(str::to_ascii_lowercase).as_deref() {
            None | Some("ldaps") => Security::Ldaps,
            Some("starttls") => Security::StartTls,
            Some("plain") => Security::Plain,
            Some(other) => {
                return Err(ProbeError::Config(format!(
                    "\"security\" expects ldaps, starttls or plain, got \"{other}\""
                )));
            }
        };
        if security == Security::Plain && !parse_bool(target, "allow_plaintext", false)? {
            return Err(ProbeError::Config(
                "Plain LDAP sends the service account's password in clear text. Use LDAPS or \
                 StartTLS, or tick \"Allow plain LDAP\" to accept it."
                    .to_string(),
            ));
        }
        let default_port = match tag(target, "port") {
            None => security.default_port(),
            Some(raw) => raw
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| ProbeError::Config(format!("Invalid port: \"{raw}\"")))?,
        };
        let address = strip_scheme(&target.address);
        let (host, port) = split_host_port(address, default_port)?;

        let (bind_user, bind_password) = match &target.credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                // Un mot de passe vide ferait une liaison « non authentifiée » qu'AD
                // accepte sans rien vérifier : on la refuse ici plutôt que de lire
                // l'annuaire en anonyme sans le savoir.
                if password.is_empty() {
                    return Err(ProbeError::Config(
                        "The service account's password is empty: Active Directory would \
                         accept an unauthenticated bind without checking anything."
                            .to_string(),
                    ));
                }
                (username.trim().to_string(), password.clone())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "Active Directory expects the user name and password of a service \
                     account, configured: {other}"
                )));
            }
        };

        Ok(Self {
            host,
            port,
            security,
            insecure_tls: parse_bool(target, "insecure_tls", false)?,
            ca_cert: tag(target, "ca_cert").map(str::to_string),
            request_timeout: Duration::from_secs(u64::from(parse_u32(
                target,
                "request_timeout_seconds",
                DEFAULT_REQUEST_TIMEOUT_SECONDS,
                1..=60,
            )?)),
            check_all_dcs: parse_bool(target, "check_all_dcs", true)?,
            stale_days_users: parse_u32(target, "stale_days_users", DEFAULT_STALE_DAYS, 1..=3650)?,
            stale_days_computers: parse_u32(
                target,
                "stale_days_computers",
                DEFAULT_STALE_DAYS,
                1..=3650,
            )?,
            inventory_interval: Duration::from_secs(
                60 * u64::from(parse_u32(
                    target,
                    "inventory_minutes",
                    DEFAULT_INVENTORY_MINUTES,
                    1..=1440,
                )?),
            ),
            bind_user,
            bind_password,
        })
    }

    /// URL donnée au client LDAP. Une IPv6 prend ses crochets.
    pub fn url(&self) -> String {
        let scheme = if self.security == Security::Ldaps { "ldaps" } else { "ldap" };
        if self.host.contains(':') {
            format!("{scheme}://[{}]:{}", self.host, self.port)
        } else {
            format!("{scheme}://{}:{}", self.host, self.port)
        }
    }

    /// Le certificat est vérifié : TLS actif et pas d'exception demandée.
    pub fn verifies_certificate(&self) -> bool {
        self.security != Security::Plain && !self.insecure_tls
    }
}

/// `ldaps://dc1.corp.lan:636/` garde son hôte et son port.
fn strip_scheme(address: &str) -> &str {
    let address = address.trim();
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    rest.split(['/', '?']).next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn with_login(mut target: Target) -> Target {
        target.credential = Credential::UsernamePassword {
            username: "svc-dumbmonit@corp.lan".into(),
            password: "s3cret".into(),
        };
        target
    }

    #[test]
    fn ldaps_par_defaut_sur_636() {
        let options =
            Options::from_target(&with_login(cible("activedirectory", "dc1.corp.lan", &[])))
                .unwrap();
        assert_eq!(options.security, Security::Ldaps);
        assert_eq!(options.port, 636);
        assert_eq!(options.url(), "ldaps://dc1.corp.lan:636");
        assert!(options.verifies_certificate());
        assert_eq!(options.inventory_interval, Duration::from_secs(15 * 60));
    }

    #[test]
    fn starttls_sur_389_et_adresse_collee_depuis_une_url() {
        let target =
            with_login(cible("activedirectory", "ldap://10.0.0.5/", &[("security", "starttls")]));
        let options = Options::from_target(&target).unwrap();
        assert_eq!(options.url(), "ldap://10.0.0.5:389");
        let v6 = Options::from_target(&with_login(cible("activedirectory", "[fd00::5]:3269", &[])))
            .unwrap();
        assert_eq!(v6.url(), "ldaps://[fd00::5]:3269");
    }

    #[test]
    fn le_clair_exige_un_accord_explicite() {
        let target = with_login(cible("activedirectory", "dc1", &[("security", "plain")]));
        assert!(matches!(Options::from_target(&target), Err(ProbeError::Config(_))));
        let target = with_login(cible(
            "activedirectory",
            "dc1",
            &[("security", "plain"), ("allow_plaintext", "true")],
        ));
        let options = Options::from_target(&target).unwrap();
        assert_eq!(options.url(), "ldap://dc1:389");
        assert!(!options.verifies_certificate());
    }

    #[test]
    fn un_mot_de_passe_vide_est_refuse() {
        let mut target = cible("activedirectory", "dc1", &[]);
        target.credential =
            Credential::UsernamePassword { username: "svc".into(), password: String::new() };
        assert!(matches!(Options::from_target(&target), Err(ProbeError::Config(_))));
        let target = cible("activedirectory", "dc1", &[]);
        assert!(matches!(Options::from_target(&target), Err(ProbeError::Config(_))));
    }
}
