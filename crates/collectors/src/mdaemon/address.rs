//! Adresse de l'équipement et réglages lus sur la cible (`Target::tags`).
//!
//! Un serveur de messagerie se surveille sur plusieurs ports à la fois : l'adresse
//! saisie est donc un **hôte**, pas une URL. Une URL ou un `hôte:port` collés par
//! habitude sont acceptés — l'hôte en est extrait, et le port donné devient celui
//! de l'API.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

/// Délai par défaut d'une connexion, d'une bannière ou d'un appel d'API.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Un hôte, avec le port éventuellement écrit dans l'adresse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    /// Nom ou adresse IP, sans crochets.
    pub name: String,
    /// Port écrit dans l'adresse (`mail.lan:444`), le cas échéant.
    pub port: Option<u16>,
}

impl Host {
    /// Forme utilisable dans une URL : les adresses IPv6 prennent leurs crochets.
    pub fn for_url(&self) -> String {
        if self.name.contains(':') { format!("[{}]", self.name) } else { self.name.clone() }
    }
}

/// Extrait l'hôte d'une adresse saisie : `mail.lan`, `mail.lan:444`,
/// `https://mail.lan:444/MdMgmtWS/`, `fd00::25`, `[fd00::25]:444`.
pub fn parse_host(address: &str) -> Result<Host, ProbeError> {
    let mut rest = address.trim();
    for scheme in ["https://", "http://"] {
        if let Some(stripped) = rest.strip_prefix(scheme) {
            rest = stripped;
        }
    }
    let rest = rest.split('/').next().unwrap_or_default();
    if rest.is_empty() {
        return Err(ProbeError::Config("Device address is empty".to_string()));
    }
    if let Some(inner) = rest.strip_prefix('[') {
        let (name, after) = inner
            .split_once(']')
            .ok_or_else(|| ProbeError::Config(format!("Invalid address: \"{address}\"")))?;
        let port = match after.strip_prefix(':') {
            Some(raw) => Some(parse_port(raw)?),
            None => None,
        };
        return Ok(Host { name: name.to_string(), port });
    }
    // Plus d'un deux-points : une IPv6 nue, sans port possible.
    if rest.matches(':').count() > 1 {
        return Ok(Host { name: rest.to_string(), port: None });
    }
    match rest.split_once(':') {
        Some((name, raw)) if !name.is_empty() => {
            Ok(Host { name: name.to_string(), port: Some(parse_port(raw)?) })
        }
        Some(_) => Err(ProbeError::Config(format!("Invalid address: \"{address}\""))),
        None => Ok(Host { name: rest.to_string(), port: None }),
    }
}

pub fn parse_port(raw: &str) -> Result<u16, ProbeError> {
    match raw.trim().parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(ProbeError::Config(format!("Invalid port: \"{raw}\""))),
    }
}

pub fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

pub fn parse_bool_or(value: Option<&str>, default: bool) -> Result<bool, ProbeError> {
    match value {
        None => Ok(default),
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "oui" | "on" => Ok(true),
            "false" | "0" | "no" | "non" | "off" => Ok(false),
            other => Err(ProbeError::Config(format!(
                "Expected a boolean value (true/false), got \"{other}\""
            ))),
        },
    }
}

/// Délai par requête, de 1 à 60 s : au-delà, le délai global du planificateur
/// couperait l'interrogation avant qu'elle ait pu enregistrer quoi que ce soit.
pub fn parse_timeout(value: Option<&str>) -> Result<Duration, ProbeError> {
    match value {
        None => Ok(DEFAULT_REQUEST_TIMEOUT),
        Some(raw) => {
            let seconds: u64 = raw
                .parse()
                .map_err(|_| ProbeError::Config(format!("Invalid timeout: \"{raw}\"")))?;
            if !(1..=60).contains(&seconds) {
                return Err(ProbeError::Config(
                    "The request timeout must be between 1 and 60 seconds".to_string(),
                ));
            }
            Ok(Duration::from_secs(seconds))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(name: &str, port: Option<u16>) -> Host {
        Host { name: name.to_string(), port }
    }

    #[test]
    fn l_adresse_accepte_hote_port_url_et_ipv6() {
        assert_eq!(parse_host("mail.lan").unwrap(), host("mail.lan", None));
        assert_eq!(parse_host(" mail.lan:444 ").unwrap(), host("mail.lan", Some(444)));
        assert_eq!(
            parse_host("https://mail.lan:444/MdMgmtWS/").unwrap(),
            host("mail.lan", Some(444))
        );
        assert_eq!(parse_host("fd00::25").unwrap(), host("fd00::25", None));
        assert_eq!(parse_host("[fd00::25]:4443").unwrap(), host("fd00::25", Some(4443)));
        assert_eq!(parse_host("fd00::25").unwrap().for_url(), "[fd00::25]");
    }

    #[test]
    fn une_adresse_invalide_est_une_erreur_de_configuration() {
        for bad in ["", "   ", "https://", ":444", "mail.lan:0", "mail.lan:http", "[fd00::25"] {
            assert!(
                matches!(parse_host(bad), Err(ProbeError::Config(_))),
                "« {bad} » devrait être refusée"
            );
        }
    }

    #[test]
    fn le_delai_est_borne() {
        assert_eq!(parse_timeout(None).unwrap(), Duration::from_secs(10));
        assert_eq!(parse_timeout(Some("30")).unwrap(), Duration::from_secs(30));
        assert!(parse_timeout(Some("0")).is_err());
        assert!(parse_timeout(Some("61")).is_err());
        assert!(parse_timeout(Some("vite")).is_err());
    }
}
