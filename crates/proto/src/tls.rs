//! Canal chiffré des agents : TLS 1.3, certificats clients (mTLS) et
//! épinglage de l'autorité du serveur.
//!
//! Le serveur tient sa propre autorité de certification, créée au premier
//! démarrage. Chaque agent génère sa paire de clés sur sa machine — la clé
//! privée ne la quitte jamais —, présente la clé publique une fois avec son
//! jeton d'enrôlement et reçoit un certificat client court, renouvelé de
//! lui-même avant échéance. Le serveur, lui, n'est reconnu que par l'empreinte
//! SHA-256 de son autorité : un certificat public valide présenté par un
//! intermédiaire ne trompe pas l'agent.
//!
//! Tout ce que les deux côtés doivent écrire de la même façon est défini ici.

use serde::{Deserialize, Serialize};

use crate::AgentIdentity;

/// Port d'écoute par défaut du canal des agents.
pub const DEFAULT_AGENT_TLS_PORT: u16 = 8443;

/// Nom porté par le certificat du serveur sur le canal des agents.
///
/// L'agent joint le serveur par l'adresse qu'on lui a donnée (IP, nom local,
/// nom public derrière une redirection de port) : aucune ne peut figurer
/// d'avance dans le certificat. Il vérifie donc ce nom fixe, sous une autorité
/// épinglée que personne d'autre ne détient — c'est l'épinglage qui fait la
/// sécurité, pas le nom.
pub const AGENT_TLS_SERVER_NAME: &str = "agents.dumbmonit.internal";

/// Enrôlement : clé publique de l'agent contre un certificat client.
pub const TLS_ENROL_PATH: &str = "/api/agent/tls/enroll";

/// Renouvellement, sous le certificat en cours.
pub const TLS_RENEW_PATH: &str = "/api/agent/tls/renew";

/// En-tête par lequel le serveur annonce son canal chiffré :
/// `port=8443; pins=<sha256>[,<sha256>]`.
///
/// Sur le canal chiffré, il tient à jour les empreintes épinglées par l'agent
/// (rotation de l'autorité). Sur l'ancien canal en clair, il invite un agent à
/// jour à migrer — c'est alors une confiance au premier usage, documentée
/// comme telle.
pub const AGENT_TLS_HEADER: &str = "x-dumbmonit-agent-tls";

/// Refus du canal chiffré à une requête sans certificat client valable
/// (absent, expiré, révoqué, ou machine supprimée). L'agent qui le reçoit
/// abandonne son certificat et se réenrôle avec son jeton.
pub const CERT_REFUSED: &str = "No valid agent certificate: this agent must enrol again \
     (its machine may have been deleted from DumbMonit).";

/// Refus de l'ancien canal en clair quand le serveur exige mTLS.
///
/// Un agent à jour lit l'en-tête [`AGENT_TLS_HEADER`] de cette réponse et
/// bascule de lui-même ; un binaire plus ancien doit être mis à jour.
pub const MTLS_REQUIRED: &str = "This server only accepts agents over its encrypted agent \
     channel (TLS 1.3 with client certificates). Update the agent by re-running the install \
     command shown in DumbMonit, and make sure the agent port of the server is reachable.";

/// Durée de vie d'un certificat client, en jours.
pub const CLIENT_CERT_DAYS: i64 = 30;

/// Marge de renouvellement : l'agent renouvelle quand il reste moins que cela.
pub const RENEW_BEFORE_DAYS: i64 = 10;

/// Demande d'enrôlement sur le canal chiffré.
///
/// Authentifiée par le jeton d'enrôlement (`Authorization: Bearer`) et, pour
/// une machine déjà liée, par son secret de liaison — exactement comme un lot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TlsEnrolRequest {
    pub identity: AgentIdentity,
    /// Clé publique ECDSA P-256 de l'agent, point non compressé (65 octets), en
    /// base64 standard.
    pub public_key: String,
}

/// Demande de renouvellement, sous le certificat en cours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TlsRenewRequest {
    /// Nouvelle clé publique : chaque renouvellement change aussi de clé.
    pub public_key: String,
}

/// Certificat remis à l'agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TlsCertificate {
    /// Certificat X.509 DER, en base64 standard.
    pub certificate: String,
    /// Fin de validité, en millisecondes depuis l'époque Unix.
    pub not_after_ms: i64,
    /// Empreintes SHA-256 (hexadécimal) des autorités à épingler.
    pub pins: Vec<String>,
    /// Secret de liaison attribué par cet enrôlement, la seule et unique fois.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_secret: Option<String>,
}

/// Annonce du canal chiffré, telle que l'en-tête [`AGENT_TLS_HEADER`] la porte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsOffer {
    pub port: u16,
    pub pins: Vec<String>,
}

impl TlsOffer {
    /// Valeur de l'en-tête.
    pub fn to_header(&self) -> String {
        format!("port={}; pins={}", self.port, self.pins.join(","))
    }

    /// Lit l'en-tête ; `None` s'il est illisible, sans port ou sans empreinte
    /// valable. Une empreinte doit être un SHA-256 hexadécimal : tout le reste
    /// est écarté plutôt qu'épinglé.
    pub fn parse(value: &str) -> Option<Self> {
        let mut port = None;
        let mut pins = Vec::new();
        for part in value.split(';') {
            let Some((key, raw)) = part.split_once('=') else { continue };
            match key.trim() {
                "port" => port = raw.trim().parse::<u16>().ok().filter(|port| *port != 0),
                "pins" => {
                    pins = raw
                        .split(',')
                        .map(|pin| pin.trim().to_ascii_lowercase())
                        .filter(|pin| is_pin(pin))
                        .collect();
                }
                _ => {}
            }
        }
        let port = port?;
        if pins.is_empty() {
            return None;
        }
        Some(Self { port, pins })
    }
}

/// Vrai pour une empreinte SHA-256 en hexadécimal minuscule.
pub fn is_pin(pin: &str) -> bool {
    pin.len() == 64
        && pin.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Normalise une empreinte saisie (`AB:CD…`, majuscules, espaces) ; `None`
/// si ce n'en est pas une.
pub fn normalise_pin(raw: &str) -> Option<String> {
    let pin: String = raw
        .chars()
        .filter(|c| !matches!(c, ':' | ' ' | '-'))
        .collect::<String>()
        .to_ascii_lowercase();
    is_pin(&pin).then_some(pin)
}

/// URL du canal chiffré pour une URL de serveur donnée : le même hôte, le
/// port des agents. `None` pour une adresse d'écoute qui n'en est pas une
/// (`0.0.0.0`).
pub fn agent_channel_url(server_url: &str, port: u16) -> Option<String> {
    let rest = server_url.split_once("://").map_or(server_url, |(_, rest)| rest);
    let authority = rest.split('/').next().unwrap_or(rest);
    let authority = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    // `[::1]:8080` → `[::1]` ; `hôte:8080` → `hôte` ; `hôte` → `hôte`.
    let host = match authority.find(']') {
        Some(end) => &authority[..=end],
        None => authority.split(':').next().unwrap_or(authority),
    };
    let host = host.trim();
    if host.is_empty() || host == "0.0.0.0" || host == "[::]" {
        return None;
    }
    Some(format!("https://{host}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const PIN_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    #[test]
    fn the_offer_round_trips_through_its_header() {
        let offer = TlsOffer { port: 8443, pins: vec![PIN_A.into(), PIN_B.into()] };
        assert_eq!(TlsOffer::parse(&offer.to_header()), Some(offer));
    }

    #[test]
    fn an_offer_without_a_valid_pin_or_port_is_ignored() {
        assert_eq!(TlsOffer::parse("port=8443; pins="), None);
        assert_eq!(TlsOffer::parse("port=8443; pins=pas-une-empreinte"), None);
        assert_eq!(TlsOffer::parse(&format!("pins={PIN_A}")), None);
        assert_eq!(TlsOffer::parse(&format!("port=0; pins={PIN_A}")), None);
        // Une empreinte douteuse est écartée, la bonne gardée.
        let offer = TlsOffer::parse(&format!("port=9443; pins=zz,{}", PIN_A.to_uppercase()));
        assert_eq!(offer, Some(TlsOffer { port: 9443, pins: vec![PIN_A.into()] }));
    }

    #[test]
    fn the_agent_channel_url_keeps_the_host_and_takes_the_agent_port() {
        assert_eq!(
            agent_channel_url("https://monit.example.com", 8443).as_deref(),
            Some("https://monit.example.com:8443")
        );
        assert_eq!(
            agent_channel_url("http://192.168.1.10:8080/", 9443).as_deref(),
            Some("https://192.168.1.10:9443")
        );
        assert_eq!(
            agent_channel_url("http://[fd00::1]:8080", 8443).as_deref(),
            Some("https://[fd00::1]:8443")
        );
        assert_eq!(agent_channel_url("http://0.0.0.0:8080", 8443), None);
    }

    #[test]
    fn a_pin_typed_with_colons_is_normalised() {
        let colons: String = PIN_A
            .as_bytes()
            .chunks(2)
            .map(|pair| std::str::from_utf8(pair).unwrap().to_uppercase())
            .collect::<Vec<_>>()
            .join(":");
        assert_eq!(normalise_pin(&colons).as_deref(), Some(PIN_A));
        assert_eq!(normalise_pin("abc"), None);
    }
}
