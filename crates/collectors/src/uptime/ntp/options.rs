//! Réglages de la sonde NTP, lus sur les étiquettes de la cible.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

use crate::uptime::guard;
use crate::uptime::tags;

/// Port NTP standard (UDP 123).
pub const DEFAULT_PORT: u16 = 123;

/// Écart par défaut, en millisecondes, au-delà duquel la sonde échoue.
///
/// 100 ms est large pour un serveur de référence (souvent inférieur à 10 ms),
/// mais raisonnable pour un équipement du réseau local qui ne corrige que par
/// SNTP : le but est de repérer une horloge qui dérive franchement, pas de
/// remplacer `chronyd`.
pub const DEFAULT_OFFSET_THRESHOLD_MS: f64 = 100.0;

#[derive(Debug, Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
    pub offset_threshold_s: f64,
    /// Lève le garde-fou sur la boucle locale et le lien local (voir `guard`).
    pub allow_private: bool,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (host, port_in_address) = tags::split_host_port(&target.address, DEFAULT_PORT)?;
        let port = tags::parse_u32(target, "port", u32::from(port_in_address), 1..=65_535)? as u16;
        let offset_threshold_ms = parse_offset_threshold(target)?;

        Ok(Self {
            host,
            port,
            timeout: tags::parse_timeout(target, "timeout_seconds")?,
            offset_threshold_s: offset_threshold_ms / 1_000.0,
            allow_private: guard::allowed(target)?,
        })
    }
}

/// L'écart tolère une valeur fractionnaire (`"12.5"`) : une borne à la
/// milliseconde entière serait arbitraire, l'horloge ne s'arrête pas là.
fn parse_offset_threshold(target: &Target) -> Result<f64, ProbeError> {
    let Some(raw) = tags::tag(target, "offset_threshold_ms") else {
        return Ok(DEFAULT_OFFSET_THRESHOLD_MS);
    };
    let value: f64 = raw.parse().map_err(|_| {
        ProbeError::Config(format!("\"offset_threshold_ms\" expects a number, got \"{raw}\""))
    })?;
    if !(value.is_finite() && value > 0.0) {
        return Err(ProbeError::Config(
            "\"offset_threshold_ms\" must be a positive number".to_string(),
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    #[test]
    fn les_defauts_couvrent_un_serveur_sans_etiquette() {
        let options = Options::from_target(&cible("ntp", "pool.ntp.org", &[])).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("pool.ntp.org", DEFAULT_PORT));
        assert_eq!(options.offset_threshold_s, DEFAULT_OFFSET_THRESHOLD_MS / 1_000.0);
        assert!(!options.allow_private);
    }

    #[test]
    fn le_seuil_decart_se_lit_en_millisecondes_fractionnaires() {
        let target = cible("ntp", "ntp.lan", &[("offset_threshold_ms", "12.5")]);
        let options = Options::from_target(&target).unwrap();
        assert!((options.offset_threshold_s - 0.0125).abs() < 1e-9);
    }

    #[test]
    fn un_seuil_invalide_est_une_erreur_de_configuration() {
        for valeur in ["0", "-5", "pas-un-nombre"] {
            let target = cible("ntp", "ntp.lan", &[("offset_threshold_ms", valeur)]);
            let error = Options::from_target(&target).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{valeur}");
        }
    }

    #[test]
    fn le_port_se_lit_dans_ladresse_ou_letiquette() {
        let options = Options::from_target(&cible("ntp", "ntp.lan:1230", &[])).unwrap();
        assert_eq!(options.port, 1230);
        let options = Options::from_target(&cible("ntp", "ntp.lan", &[("port", "1231")])).unwrap();
        assert_eq!(options.port, 1231);
    }
}
