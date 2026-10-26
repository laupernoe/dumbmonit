//! Réglages de la sonde SNMP OID.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError, Target};

use crate::snmp::oid::ObjectId;
use crate::snmp::session::{CommunityVersion, DEFAULT_PORT};
use crate::uptime::{guard, tags};

#[derive(Debug, Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    pub oid: ObjectId,
    pub version: CommunityVersion,
    /// Borne basse tolérée pour une valeur numérique (incluse).
    pub min: Option<f64>,
    /// Borne haute tolérée pour une valeur numérique (incluse).
    pub max: Option<f64>,
    /// Texte que la valeur doit contenir, sans tenir compte de la casse.
    pub expect: Option<String>,
    pub allow_private: bool,
    pub timeout: Duration,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (host, port_in_address) = tags::split_host_port(&target.address, DEFAULT_PORT)?;
        let port = tags::parse_u32(target, "port", u32::from(port_in_address), 1..=65_535)? as u16;

        let raw_oid = tags::tag(target, "oid").ok_or_else(|| {
            ProbeError::Config(
                "\"oid\" is required: the object to read, for example \"1.3.6.1.2.1.1.3.0\""
                    .to_string(),
            )
        })?;
        let oid: ObjectId =
            raw_oid.parse().map_err(|error| ProbeError::Config(format!("\"oid\": {error}")))?;

        let min = parse_number(target, "min")?;
        let max = parse_number(target, "max")?;
        let inverted = matches!((min, max), (Some(low), Some(high)) if low > high);
        if inverted {
            return Err(ProbeError::Config("\"min\" cannot be greater than \"max\"".to_string()));
        }

        let snmp = matches!(
            target.credential,
            Credential::SnmpCommunity { .. } | Credential::SnmpV3 { .. }
        );
        if !snmp {
            return Err(ProbeError::Config(
                "this check needs an SNMP community (v1/v2c) or an SNMP v3 user".to_string(),
            ));
        }

        Ok(Self {
            host,
            port,
            oid,
            version: CommunityVersion::from_tags(&target.tags),
            min,
            max,
            expect: tags::tag(target, "expect").map(str::to_string),
            allow_private: guard::allowed(target)?,
            timeout: tags::parse_timeout(target, "timeout_seconds")?,
        })
    }
}

/// Nombre décimal, la virgule étant tolérée comme séparateur.
fn parse_number(target: &Target, key: &str) -> Result<Option<f64>, ProbeError> {
    let Some(raw) = tags::tag(target, key) else { return Ok(None) };
    let value: f64 = raw
        .replace(',', ".")
        .parse()
        .map_err(|_| ProbeError::Config(format!("\"{key}\" expects a number, got \"{raw}\"")))?;
    if !value.is_finite() {
        return Err(ProbeError::Config(format!("\"{key}\" must be a finite number")));
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    const OID: &str = "1.3.6.1.2.1.1.3.0";

    fn target(address: &str, tags: &[(&str, &str)]) -> Target {
        let mut target = cible("snmp_oid", address, tags);
        target.credential = Credential::SnmpCommunity { community: "public".into() };
        target
    }

    fn options(tags: &[(&str, &str)]) -> Result<Options, ProbeError> {
        Options::from_target(&target("switch.lan", tags))
    }

    #[test]
    fn loid_est_obligatoire_et_valide() {
        let error = options(&[]).unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.means_down());
        assert!(error.to_string().contains("oid"), "{error}");

        for invalide in ["pas-un-oid", "1", "1.3.x"] {
            let error = options(&[("oid", invalide)]).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{invalide}");
        }
        let lues = options(&[("oid", ".1.3.6.1.2.1.1.3.0")]).unwrap();
        assert_eq!(lues.oid.to_string(), OID);
    }

    #[test]
    fn les_defauts_couvrent_une_lecture_simple() {
        let lues = options(&[("oid", OID)]).unwrap();
        assert_eq!((lues.host.as_str(), lues.port), ("switch.lan", 161));
        assert_eq!(lues.version, CommunityVersion::V2c);
        assert!(lues.min.is_none() && lues.max.is_none() && lues.expect.is_none());
        assert!(!lues.allow_private);
    }

    #[test]
    fn le_port_se_lit_dans_ladresse_ou_letiquette() {
        let target = target("switch.lan:1161", &[("oid", OID)]);
        assert_eq!(Options::from_target(&target).unwrap().port, 1161);
        let lues = options(&[("oid", OID), ("port", "1162")]).unwrap();
        assert_eq!(lues.port, 1162);
    }

    #[test]
    fn les_bornes_se_lisent_et_se_controlent() {
        let lues = options(&[("oid", OID), ("min", "10"), ("max", "85,5")]).unwrap();
        assert_eq!((lues.min, lues.max), (Some(10.0), Some(85.5)));

        let error = options(&[("oid", OID), ("min", "x")]).unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        let error = options(&[("oid", OID), ("min", "9"), ("max", "3")]).unwrap_err();
        assert!(error.to_string().contains("greater"), "{error}");
    }

    #[test]
    fn la_version_v1_se_demande_par_etiquette() {
        let lues = options(&[("oid", OID), ("snmp_version", "1")]).unwrap();
        assert_eq!(lues.version, CommunityVersion::V1);
    }

    #[test]
    fn sans_identifiant_snmp_la_configuration_est_refusee() {
        let target = cible("snmp_oid", "switch.lan", &[("oid", OID)]);
        let error = Options::from_target(&target).unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(error.to_string().contains("SNMP"), "{error}");
    }
}
