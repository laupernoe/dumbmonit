//! Réglages de la sonde UDP, lus sur les étiquettes de la cible.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

use crate::uptime::guard;
use crate::uptime::tags;

/// Taille maximale du datagramme envoyé, en octets : de quoi tenir dans un
/// paquet sans fragmentation sur un réseau ordinaire.
pub const MAX_PAYLOAD_BYTES: usize = 1_400;

/// Codage de l'étiquette `payload`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// Texte ; `\n`, `\r`, `\t`, `\0`, `\\` et `\xNN` y sont interprétés.
    Text,
    /// Octets en hexadécimal (`"ff00 1a"`), espaces tolérés.
    Hex,
}

impl Encoding {
    pub fn parse(raw: &str) -> Result<Self, ProbeError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "text" | "" => Ok(Self::Text),
            "hex" => Ok(Self::Hex),
            other => Err(ProbeError::Config(format!(
                "\"payload_encoding\" expects \"text\" or \"hex\", got \"{other}\""
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    pub payload: Vec<u8>,
    /// Sous-chaîne que la réponse doit contenir. `None` : aucune attente.
    pub expect_response: Option<String>,
    pub timeout: Duration,
    pub allow_private: bool,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        // Pas de port par défaut : il n'existe pas de « port UDP » générique, et
        // sonder le mauvais ne se verrait pas (UDP ne répond pas).
        let (host, port_in_address) = tags::split_host_port(&target.address, 0)?;
        let port = tags::parse_u32(target, "port", u32::from(port_in_address), 0..=65_535)? as u16;
        if port == 0 {
            return Err(ProbeError::Config(
                "the UDP port is required: write it in the address (\"host:5060\") or in the \
                 \"port\" tag"
                    .to_string(),
            ));
        }

        let encoding = Encoding::parse(tags::tag(target, "payload_encoding").unwrap_or("text"))?;
        let payload = match tags::tag(target, "payload") {
            None => Vec::new(),
            Some(raw) => match encoding {
                Encoding::Text => unescape(raw)?,
                Encoding::Hex => decode_hex(raw)?,
            },
        };
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(ProbeError::Config(format!(
                "\"payload\" is {} bytes long, the limit is {MAX_PAYLOAD_BYTES}",
                payload.len()
            )));
        }

        Ok(Self {
            host,
            port,
            payload,
            expect_response: tags::tag(target, "expect_response").map(str::to_string),
            timeout: tags::parse_timeout(target, "timeout_seconds")?,
            allow_private: guard::allowed(target)?,
        })
    }
}

/// Interprète les échappements d'un texte saisi sur une seule ligne.
fn unescape(raw: &str) -> Result<Vec<u8>, ProbeError> {
    let mut out = Vec::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match chars.next() {
            Some('n') => out.push(b'\n'),
            Some('r') => out.push(b'\r'),
            Some('t') => out.push(b'\t'),
            Some('0') => out.push(0),
            Some('\\') => out.push(b'\\'),
            Some('x') => {
                let hi = chars.next().and_then(|c| c.to_digit(16));
                let lo = chars.next().and_then(|c| c.to_digit(16));
                match (hi, lo) {
                    (Some(hi), Some(lo)) => out.push((hi * 16 + lo) as u8),
                    _ => {
                        return Err(ProbeError::Config(
                            "\"payload\": \"\\x\" must be followed by two hexadecimal digits"
                                .to_string(),
                        ));
                    }
                }
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "\"payload\": unknown escape sequence \"\\{}\"",
                    other.map(String::from).unwrap_or_default()
                )));
            }
        }
    }
    Ok(out)
}

fn decode_hex(raw: &str) -> Result<Vec<u8>, ProbeError> {
    let digits: Vec<char> = raw.chars().filter(|c| !c.is_whitespace() && *c != ':').collect();
    if !digits.len().is_multiple_of(2) {
        return Err(ProbeError::Config(
            "\"payload\" in hex needs an even number of digits".to_string(),
        ));
    }
    digits
        .chunks(2)
        .map(|pair| {
            let hi = pair[0].to_digit(16);
            let lo = pair[1].to_digit(16);
            match (hi, lo) {
                (Some(hi), Some(lo)) => Ok((hi * 16 + lo) as u8),
                _ => Err(ProbeError::Config(format!(
                    "\"payload\" is not hexadecimal: \"{}{}\"",
                    pair[0], pair[1]
                ))),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    #[test]
    fn le_port_est_obligatoire() {
        let error = Options::from_target(&cible("udp", "10.0.0.1", &[])).unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.means_down());
        assert!(error.to_string().contains("port"), "{error}");
    }

    #[test]
    fn le_port_se_lit_dans_ladresse_ou_letiquette() {
        assert_eq!(Options::from_target(&cible("udp", "nas.lan:5060", &[])).unwrap().port, 5060);
        let options = Options::from_target(&cible("udp", "nas.lan", &[("port", "161")])).unwrap();
        assert_eq!(options.port, 161);
    }

    #[test]
    fn le_texte_interprete_les_echappements() {
        let target = cible("udp", "x:1", &[("payload", "PING\\r\\n\\x01\\\\")]);
        let options = Options::from_target(&target).unwrap();
        assert_eq!(options.payload, b"PING\r\n\x01\\");
    }

    #[test]
    fn lhexadecimal_tolere_les_espaces() {
        let target = cible("udp", "x:1", &[("payload", "ff 00:1A"), ("payload_encoding", "hex")]);
        assert_eq!(Options::from_target(&target).unwrap().payload, vec![0xff, 0x00, 0x1a]);
    }

    #[test]
    fn un_payload_invalide_est_une_erreur_de_configuration() {
        for (payload, encoding) in
            [("abc", "hex"), ("zz", "hex"), ("\\q", "text"), ("\\x1", "text"), ("a", "base64")]
        {
            let tags = [("payload", payload), ("payload_encoding", encoding)];
            let target = cible("udp", "x:1", &tags);
            let error = Options::from_target(&target).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{payload}/{encoding}");
            assert!(!error.means_down());
        }
    }

    #[test]
    fn un_payload_trop_long_est_refuse() {
        let long = "a".repeat(MAX_PAYLOAD_BYTES + 1);
        let target = cible("udp", "x:1", &[("payload", &long)]);
        assert!(Options::from_target(&target).is_err());
    }

    #[test]
    fn la_reponse_attendue_est_facultative() {
        let options = Options::from_target(&cible("udp", "x:1", &[])).unwrap();
        assert!(options.expect_response.is_none());
        assert!(options.payload.is_empty());
        let target = cible("udp", "x:1", &[("expect_response", "pong")]);
        let options = Options::from_target(&target).unwrap();
        assert_eq!(options.expect_response.as_deref(), Some("pong"));
    }
}
