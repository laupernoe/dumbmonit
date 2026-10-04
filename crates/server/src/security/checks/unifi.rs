//! UniFi : micrologiciel des équipements adoptés.
//!
//! Le collecteur lit les équipements et leur drapeau `upgradable` ; les
//! réglages du site (SSH des équipements, mot de passe par défaut, 2FA du
//! compte UI) ne sont pas lus aujourd'hui.

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};
use super::zero_everywhere;

pub const FIRMWARE: CheckDef = CheckDef {
    id: "unifi.firmware",
    title: "Device firmware up to date",
    category: Category::Patching,
    severity: Severity::High,
    reference: "https://community.ui.com/releases",
    remediation: "Upgrade the listed devices from the UniFi Network application (Devices > \
                  Upgrade), or enable scheduled automatic firmware updates.",
};

pub const DEFS: &[&CheckDef] = &[&FIRMWARE];

pub struct Unifi;

impl SecurityProvider for Unifi {
    fn kinds(&self) -> &'static [&'static str] {
        &["unifi"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["unifi_device_upgradable"]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![zero_everywhere(
            &FIRMWARE,
            facts,
            "unifi_device_upgradable",
            "device",
            "",
            "Device list not collected.",
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn les_equipements_a_mettre_a_jour_sont_nommes() {
        let facts = Facts::from_pairs(&[
            ("unifi_device_upgradable", &[("device", "Gateway")], 1.0),
            ("unifi_device_upgradable", &[("device", "Living room AP")], 0.0),
        ]);
        let check = &Unifi.evaluate("", &facts)[0];
        assert_eq!((check.result, check.evidence.as_str()), (Outcome::Fail, "Gateway"));

        let ok = Facts::from_pairs(&[
            ("unifi_device_upgradable", &[("device", "Gateway")], 0.0),
            ("unifi_device_upgradable", &[("device", "AP")], 0.0),
        ]);
        let check = &Unifi.evaluate("", &ok)[0];
        assert_eq!(
            (check.result, check.evidence.as_str()),
            (Outcome::Pass, "None on any of 2 devices")
        );
    }
}
