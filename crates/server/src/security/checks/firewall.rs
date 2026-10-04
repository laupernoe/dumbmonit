//! Pare-feu OPNsense et pfSense.
//!
//! OPNsense dit par son API si des mises à jour attendent, si une nouvelle
//! série majeure existe, si un redémarrage est requis et si `pf` filtre. La
//! REST API de pfSense ne dit rien des mises à jour du système lui-même —
//! seulement du paquet REST API. L'exposition de l'interface
//! d'administration sur le WAN n'est observable sur aucun des deux
//! aujourd'hui : voir la documentation.

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};

pub const OPN_UPDATES: CheckDef = CheckDef {
    id: "opnsense.firmware.updates",
    title: "Firmware updates installed",
    category: Category::Patching,
    severity: Severity::High,
    reference: "https://docs.opnsense.org/manual/updates.html",
    remediation: "Install the pending updates (System > Firmware > Status > Check for updates, \
                  then Update). OPNsense publishes security fixes in minor releases.",
};

pub const OPN_UPGRADE: CheckDef = CheckDef {
    id: "opnsense.firmware.series",
    title: "On the current major release series",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://docs.opnsense.org/manual/updates.html",
    remediation: "A new major series is available: plan the upgrade (System > Firmware > Status). \
                  The previous series stops receiving updates a few months after a new one ships.",
};

pub const OPN_REBOOT: CheckDef = CheckDef {
    id: "opnsense.reboot",
    title: "No reboot pending after an update",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://docs.opnsense.org/manual/updates.html",
    remediation: "Reboot the firewall in a maintenance window: the installed kernel or base fixes \
                  are not active until then.",
};

pub const OPN_PF: CheckDef = CheckDef {
    id: "opnsense.pf",
    title: "Packet filter enabled",
    category: Category::Exposure,
    severity: Severity::Critical,
    reference: "https://docs.opnsense.org/manual/firewall.html",
    remediation: "Re-enable filtering (Firewall > Settings > Advanced: untick \"Disable \
                  Firewall\"). With pf off, the firewall routes everything without filtering.",
};

pub const PFS_RESTAPI: CheckDef = CheckDef {
    id: "pfsense.restapi",
    title: "REST API package up to date",
    category: Category::Patching,
    severity: Severity::Low,
    reference: "https://pfrest.org/",
    remediation: "Update the REST API package (`pfsense-restapi update`, or System > Package \
                  Manager). It runs with administrator rights on the firewall.",
};

pub const DEFS: &[&CheckDef] = &[&OPN_UPDATES, &OPN_UPGRADE, &OPN_REBOOT, &OPN_PF, &PFS_RESTAPI];

pub struct Opnsense;

impl SecurityProvider for Opnsense {
    fn kinds(&self) -> &'static [&'static str] {
        &["opnsense"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[
            "opnsense_firmware_checked",
            "opnsense_firmware_updates_pending",
            "opnsense_firmware_upgrade_available",
            "opnsense_firmware_reboot_required",
            "opnsense_pf_enabled",
        ]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        // Sans contrôle de mise à jour côté OPNsense, rien n'est su : les
        // séries de mises à jour n'existent alors pas (voir
        // `collectors/opnsense/metrics.rs`).
        let not_checked = facts.max("opnsense_firmware_checked") == Some(0.0);
        let firmware = |def: &CheckDef, metric: &str, ok: &str, ko: &dyn Fn(f64) -> String| {
            if not_checked {
                return def
                    .unknown("OPNsense has not checked for updates yet (System > Firmware).");
            }
            match facts.max(metric) {
                None => def.unknown("Firmware status not collected."),
                Some(value) if value > 0.0 => def.fail(ko(value)),
                Some(_) => def.pass(ok),
            }
        };
        vec![
            firmware(&OPN_UPDATES, "opnsense_firmware_updates_pending", "Up to date", &|n| {
                super::plural(n, "package update")
            }),
            firmware(
                &OPN_UPGRADE,
                "opnsense_firmware_upgrade_available",
                "Current series",
                &|_| "A new major release is available".to_string(),
            ),
            firmware(
                &OPN_REBOOT,
                "opnsense_firmware_reboot_required",
                "No reboot pending",
                &|_| "Reboot required".to_string(),
            ),
            match facts.max("opnsense_pf_enabled") {
                None => OPN_PF.unknown("Packet filter status not collected."),
                Some(on) => OPN_PF.verdict(
                    on >= 1.0,
                    if on >= 1.0 { "pf is filtering" } else { "pf is disabled" },
                ),
            },
        ]
    }
}

pub struct Pfsense;

impl SecurityProvider for Pfsense {
    fn kinds(&self) -> &'static [&'static str] {
        &["pfsense"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["pfsense_restapi_update_available"]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![match facts.max("pfsense_restapi_update_available") {
            None => PFS_RESTAPI.unknown("REST API version not collected."),
            Some(available) => PFS_RESTAPI.verdict(
                available == 0.0,
                if available == 0.0 { "Up to date" } else { "An update is available" },
            ),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn opnsense_a_jour_et_filtrant() {
        let facts = Facts::from_pairs(&[
            ("opnsense_firmware_checked", &[], 1.0),
            ("opnsense_firmware_updates_pending", &[], 0.0),
            ("opnsense_firmware_upgrade_available", &[], 0.0),
            ("opnsense_firmware_reboot_required", &[], 0.0),
            ("opnsense_pf_enabled", &[], 1.0),
        ]);
        assert!(Opnsense.evaluate("", &facts).iter().all(|c| c.result == Outcome::Pass));
    }

    #[test]
    fn opnsense_en_retard_et_pf_coupe() {
        let facts = Facts::from_pairs(&[
            ("opnsense_firmware_checked", &[], 1.0),
            ("opnsense_firmware_updates_pending", &[], 12.0),
            ("opnsense_firmware_upgrade_available", &[], 1.0),
            ("opnsense_firmware_reboot_required", &[], 1.0),
            ("opnsense_pf_enabled", &[], 0.0),
        ]);
        let checks = Opnsense.evaluate("", &facts);
        assert!(checks.iter().all(|c| c.result == Outcome::Fail));
        assert_eq!(checks[0].evidence, "12 package updates");
        assert_eq!(checks[3].evidence, "pf is disabled");
    }

    #[test]
    fn sans_controle_de_mise_a_jour_tout_est_inconnu() {
        let facts = Facts::from_pairs(&[("opnsense_firmware_checked", &[], 0.0)]);
        let checks = Opnsense.evaluate("", &facts);
        assert!(checks[..3].iter().all(|c| c.result == Outcome::Unknown));
        assert!(checks[0].evidence.contains("not checked"));
    }

    #[test]
    fn pfsense_paquet_rest_api() {
        let facts = Facts::from_pairs(&[("pfsense_restapi_update_available", &[], 1.0)]);
        assert_eq!(Pfsense.evaluate("", &facts)[0].result, Outcome::Fail);
        let facts = Facts::from_pairs(&[("pfsense_restapi_update_available", &[], 0.0)]);
        assert_eq!(Pfsense.evaluate("", &facts)[0].result, Outcome::Pass);
    }
}
