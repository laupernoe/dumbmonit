//! Socle commun des sondes de boîte aux lettres : IMAP et POP3.
//!
//! Les deux protocoles se déroulent de la même façon — connexion, TLS implicite
//! ou `STARTTLS`, bannière, identification facultative, adieux — et partagent
//! donc leurs réglages et leur plomberie. Seules les commandes diffèrent.
//!
//! La règle sur les identifiants est celle de la sonde SMTP : ils ne partent
//! qu'une fois le certificat jugé par [`session::record_tls`], ou en clair si
//! l'utilisateur a écrit `security = none` en toutes lettres. Le mode par défaut
//! est chiffré : on ne peut pas envoyer un mot de passe en clair par oubli.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError, Target};
use tokio::net::TcpStream;

use super::guard;
use super::outcome::Report;
use super::session::{self, Deadline, LineReader, SessionError, Stream};
use super::tags;

/// Taille maximale d'une ligne de réponse.
pub const MAX_LINE_BYTES: usize = 2048;

/// Mode de chiffrement de la connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    /// TLS dès la connexion (993, 995).
    Implicit,
    /// Connexion en clair puis `STARTTLS` / `STLS` (143, 110).
    StartTls,
    /// Aucun chiffrement. Les identifiants partent en clair.
    None,
}

impl Security {
    pub fn parse(raw: &str) -> Result<Self, ProbeError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "tls" | "implicit" | "ssl" => Ok(Self::Implicit),
            "starttls" | "stls" => Ok(Self::StartTls),
            "none" | "plain" => Ok(Self::None),
            other => Err(ProbeError::Config(format!(
                "\"security\" expects \"tls\", \"starttls\" or \"none\", got \"{other}\""
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implicit => "tls",
            Self::StartTls => "starttls",
            Self::None => "none",
        }
    }
}

/// Ports usuels d'un protocole, selon le mode de chiffrement.
#[derive(Debug, Clone, Copy)]
pub struct Ports {
    pub implicit: u16,
    pub starttls: u16,
    pub none: u16,
}

/// Réglages communs aux sondes IMAP et POP3.
#[derive(Debug, Clone)]
pub struct Common {
    pub host: String,
    pub port: u16,
    pub security: Security,
    /// Nom envoyé en SNI et vérifié dans le certificat.
    pub server_name: String,
    pub allow_untrusted: bool,
    pub allow_private: bool,
    /// Identifiants, quand la cible en porte.
    pub login: Option<(String, String)>,
    pub timeout: Duration,
}

impl Common {
    pub fn from_target(target: &Target, ports: Ports) -> Result<Self, ProbeError> {
        let security = Security::parse(tags::tag(target, "security").unwrap_or("tls"))?;
        // Port sentinelle : l'adresse peut en porter un, sinon c'est le mode de
        // chiffrement qui décide.
        let (host, port_in_address) = tags::split_host_port(&target.address, 0)?;
        if host.is_empty() {
            return Err(ProbeError::Config(
                "the address must be the mail server (for example \"imap.example.com\")"
                    .to_string(),
            ));
        }
        let port = match tags::parse_u32(target, "port", u32::from(port_in_address), 0..=65_535)? {
            0 => match security {
                Security::Implicit => ports.implicit,
                Security::StartTls => ports.starttls,
                Security::None => ports.none,
            },
            port => port as u16,
        };

        let login = match &target.credential {
            Credential::None => None,
            Credential::UsernamePassword { username, password } => {
                // Un saut de ligne dans un identifiant permettrait d'injecter une
                // commande dans le dialogue.
                let forbidden = |text: &str| text.contains(['\r', '\n', '\0']);
                if forbidden(username) || forbidden(password) {
                    return Err(ProbeError::Config(
                        "the user name and password cannot contain line breaks or NUL bytes"
                            .to_string(),
                    ));
                }
                Some((username.clone(), password.clone()))
            }
            _ => {
                return Err(ProbeError::Config(
                    "this check accepts no credential, or a user name and password".to_string(),
                ));
            }
        };

        Ok(Self {
            server_name: tags::tag(target, "server_name")
                .map(str::to_string)
                .unwrap_or_else(|| host.clone()),
            host,
            port,
            security,
            allow_untrusted: tags::parse_bool(target, "insecure_tls", false)?,
            allow_private: guard::allowed(target)?,
            login,
            timeout: tags::parse_timeout(target, "timeout_seconds")?,
        })
    }
}

/// Erreur interne : le détail est déjà dans le rapport.
pub struct Aborted;

/// Enregistre l'échec d'une étape de session dans le rapport.
pub fn fail(report: &mut Report, error: &SessionError) -> Aborted {
    let (reason, detail) = error.failure();
    report.fail(reason, detail);
    Aborted
}

/// Ouvre la connexion et, en TLS implicite, la chiffre.
///
/// `Err` seulement quand le garde-fou refuse l'adresse. `Ok(None)` : la sonde a
/// échoué, la raison est dans le rapport.
pub async fn open(
    report: &mut Report,
    common: &Common,
    deadline: Deadline,
) -> Result<Option<Stream>, ProbeError> {
    let connected =
        match session::connect(&common.host, common.port, common.allow_private, deadline).await {
            Ok(connected) => connected,
            Err(SessionError::Refused(error)) => return Err(error),
            Err(error) => {
                fail(report, &error);
                return Ok(None);
            }
        };
    report.gauge("connect_seconds", connected.connect.as_secs_f64());

    if common.security == Security::Implicit {
        return Ok(upgrade(report, connected.stream, common, deadline).await);
    }
    Ok(Some(Stream::Plain(connected.stream)))
}

/// Chiffre la connexion et publie les métriques du certificat.
///
/// `None` quand la sonde doit s'arrêter : rien n'a été envoyé, et surtout pas un
/// identifiant, tant que le certificat n'a pas été jugé.
pub async fn upgrade(
    report: &mut Report,
    plain: TcpStream,
    common: &Common,
    deadline: Deadline,
) -> Option<Stream> {
    let secured = match session::upgrade(plain, &common.server_name, deadline).await {
        Ok(secured) => secured,
        Err(error) => {
            fail(report, &error);
            return None;
        }
    };
    if !session::record_tls(report, &secured, common.allow_untrusted) {
        return None;
    }
    Some(Stream::Tls(Box::new(secured.stream)))
}

/// Envoie une ligne.
pub async fn send(
    report: &mut Report,
    stream: &mut Stream,
    line: &str,
    deadline: Deadline,
) -> Result<(), Aborted> {
    stream.write_line(line, deadline).await.map_err(|error| fail(report, &error))
}

/// Lit une ligne.
pub async fn read_line(
    report: &mut Report,
    stream: &mut Stream,
    reader: &mut LineReader,
    deadline: Deadline,
) -> Result<String, Aborted> {
    reader.line(stream, deadline).await.map_err(|error| fail(report, &error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    const PORTS: Ports = Ports { implicit: 993, starttls: 143, none: 143 };

    #[test]
    fn le_port_decoule_du_mode_de_chiffrement() {
        let common = Common::from_target(&cible("imap", "mail.lan", &[]), PORTS).unwrap();
        assert_eq!((common.security, common.port), (Security::Implicit, 993));
        let target = cible("imap", "mail.lan", &[("security", "starttls")]);
        assert_eq!(Common::from_target(&target, PORTS).unwrap().port, 143);
        let target = cible("imap", "mail.lan:1143", &[("security", "none")]);
        assert_eq!(Common::from_target(&target, PORTS).unwrap().port, 1143);
        let target = cible("imap", "mail.lan", &[("port", "10993")]);
        assert_eq!(Common::from_target(&target, PORTS).unwrap().port, 10993);
    }

    #[test]
    fn un_mode_inconnu_est_une_erreur_de_configuration() {
        let target = cible("imap", "mail.lan", &[("security", "ssl3")]);
        let error = Common::from_target(&target, PORTS).unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.means_down());
    }

    #[test]
    fn des_identifiants_avec_saut_de_ligne_sont_refuses() {
        let mut target = cible("imap", "mail.lan", &[]);
        target.credential =
            Credential::UsernamePassword { username: "a\r\nb".into(), password: "x".into() };
        assert!(Common::from_target(&target, PORTS).is_err());
        target.credential =
            Credential::UsernamePassword { username: "a".into(), password: "x".into() };
        assert!(Common::from_target(&target, PORTS).unwrap().login.is_some());
    }

    #[test]
    fn un_autre_type_didentifiant_est_refuse() {
        let mut target = cible("imap", "mail.lan", &[]);
        target.credential = Credential::ApiToken { token: "t".into() };
        assert!(Common::from_target(&target, PORTS).is_err());
    }
}
