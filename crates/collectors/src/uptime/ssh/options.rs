//! Réglages de la sonde SSH.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

use super::wire::KeyType;
use crate::uptime::guard;
use crate::uptime::tags;

/// Port SSH standard.
pub const DEFAULT_PORT: u16 = 22;

/// Longueur d'un SHA-256 en base64 sans remplissage.
const FINGERPRINT_LEN: usize = 43;

#[derive(Debug, Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    /// Empreinte attendue, normalisée (`SHA256:…` sans remplissage).
    pub expected_host_key: Option<String>,
    pub host_key_type: KeyType,
    /// Publie l'empreinte même sans empreinte attendue.
    pub alert_on_change: bool,
    pub timeout: Duration,
    pub allow_private: bool,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (host, port_in_address) = tags::split_host_port(&target.address, DEFAULT_PORT)?;
        let port = tags::parse_u32(target, "port", u32::from(port_in_address), 1..=65_535)? as u16;
        let expected_host_key =
            tags::tag(target, "expected_host_key").map(normalize_fingerprint).transpose()?;

        Ok(Self {
            host,
            port,
            expected_host_key,
            host_key_type: KeyType::parse(tags::tag(target, "host_key_type").unwrap_or("auto"))?,
            alert_on_change: tags::parse_bool(target, "alert_on_change", false)?,
            timeout: tags::parse_timeout(target, "timeout_seconds")?,
            allow_private: guard::allowed(target)?,
        })
    }

    /// Faut-il pousser l'échange jusqu'à la clé d'hôte ?
    pub fn wants_host_key(&self) -> bool {
        self.expected_host_key.is_some() || self.alert_on_change
    }
}

/// Ramène l'empreinte au format d'`ssh-keygen -l` : `SHA256:` puis 43 caractères
/// base64 sans remplissage. Le préfixe est facultatif, les `=` finaux tolérés.
pub fn normalize_fingerprint(raw: &str) -> Result<String, ProbeError> {
    let raw = raw.trim();
    let body = match raw.get(..7) {
        Some(prefix) if prefix.eq_ignore_ascii_case("SHA256:") => &raw[7..],
        _ => raw,
    };
    let body = body.trim_end_matches('=');
    let valid = body.len() == FINGERPRINT_LEN
        && body.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/');
    if !valid {
        return Err(ProbeError::Config(
            "\"expected_host_key\" expects an OpenSSH SHA256 fingerprint such as \
             \"SHA256:nThbg6kXUpJWGl7E1IGOCspRomTxdCARLviKw6E5SY8\" (see `ssh-keygen -lf`)"
                .to_string(),
        ));
    }
    Ok(format!("SHA256:{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    const EMPREINTE: &str = "SHA256:nThbg6kXUpJWGl7E1IGOCspRomTxdCARLviKw6E5SY8";

    #[test]
    fn les_defauts_couvrent_un_serveur_sans_etiquette() {
        let options = Options::from_target(&cible("ssh", "nas.lan", &[])).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("nas.lan", DEFAULT_PORT));
        assert!(!options.wants_host_key());
        assert_eq!(options.host_key_type, KeyType::Auto);
    }

    #[test]
    fn le_port_se_lit_dans_ladresse_ou_letiquette() {
        assert_eq!(Options::from_target(&cible("ssh", "nas.lan:2222", &[])).unwrap().port, 2222);
        let options = Options::from_target(&cible("ssh", "nas.lan", &[("port", "2200")])).unwrap();
        assert_eq!(options.port, 2200);
    }

    #[test]
    fn lempreinte_est_normalisee() {
        let corps = &EMPREINTE[7..];
        let minuscules = format!("sha256:{corps}=");
        let espaces = format!(" {EMPREINTE} ");
        for saisie in [EMPREINTE, corps, minuscules.as_str(), espaces.as_str()] {
            assert_eq!(normalize_fingerprint(saisie).unwrap(), EMPREINTE, "{saisie}");
        }
    }

    #[test]
    fn une_empreinte_invalide_est_une_erreur_de_configuration() {
        let mauvais = "SHA256:nThbg6kXUpJWGl7E1IGOCspRomTxdCARLviKw6E5SY!";
        for saisie in ["SHA256:court", "MD5:00:11:22", mauvais, "é"] {
            let error = normalize_fingerprint(saisie).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{saisie}");
            assert!(!error.means_down());
        }
    }

    #[test]
    fn une_empreinte_attendue_ou_lalerte_demandent_la_cle() {
        let target = cible("ssh", "x", &[("expected_host_key", EMPREINTE)]);
        assert!(Options::from_target(&target).unwrap().wants_host_key());
        let target = cible("ssh", "x", &[("alert_on_change", "true")]);
        assert!(Options::from_target(&target).unwrap().wants_host_key());
    }

    #[test]
    fn un_type_de_cle_inconnu_est_refuse() {
        let target = cible("ssh", "x", &[("host_key_type", "dsa")]);
        assert!(Options::from_target(&target).is_err());
    }
}
