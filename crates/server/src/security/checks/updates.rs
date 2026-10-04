//! « Une mise à jour de l'éditeur attend » pour les produits dont le
//! collecteur ne sait rien d'autre de la sécurité : une table, une ligne par
//! contrôle. C'est le contrôle le plus rentable d'un homelab — la plupart des
//! compromissions passent par une faille corrigée depuis des semaines.

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};

const fn update(
    id: &'static str,
    title: &'static str,
    severity: Severity,
    reference: &'static str,
    remediation: &'static str,
) -> CheckDef {
    CheckDef { id, title, category: Category::Patching, severity, reference, remediation }
}

pub const FORTIGATE_FIRMWARE: CheckDef = update(
    "fortigate.firmware",
    "FortiOS patch installed",
    Severity::Critical,
    "https://docs.fortinet.com/upgrade-tool",
    "Install the latest patch of your FortiOS branch (System > Firmware), following Fortinet's \
     upgrade path. FortiOS flaws are actively exploited within days of disclosure.",
);

pub const FORTIGATE_LICENSE: CheckDef = CheckDef {
    id: "fortigate.license",
    title: "FortiGuard licences valid",
    category: Category::Configuration,
    severity: Severity::High,
    reference: "https://docs.fortinet.com/product/fortigate",
    remediation: "Renew the expired FortiGuard services: without them IPS, antivirus and web \
                  filtering signatures stop updating.",
};

pub const MIKROTIK_ROUTEROS: CheckDef = update(
    "mikrotik.routeros",
    "RouterOS up to date",
    Severity::High,
    "https://help.mikrotik.com/docs/spaces/ROS/pages/328142/Upgrading+and+installation",
    "Install the latest RouterOS of your channel (System > Packages > Check For Updates).",
);

pub const MIKROTIK_ROUTERBOOT: CheckDef = update(
    "mikrotik.routerboot",
    "RouterBOOT firmware matches RouterOS",
    Severity::Low,
    "https://help.mikrotik.com/docs/spaces/ROS/pages/328142/Upgrading+and+installation",
    "Run System > RouterBOARD > Upgrade, then reboot, so the bootloader matches RouterOS.",
);

pub const HOMEASSISTANT: CheckDef = update(
    "homeassistant.updates",
    "Home Assistant components up to date",
    Severity::Medium,
    "https://www.home-assistant.io/docs/configuration/securing/",
    "Install the pending updates (Settings > Updates) after reading the release notes.",
);

pub const NEXTCLOUD: CheckDef = update(
    "nextcloud.server",
    "Nextcloud server up to date",
    Severity::High,
    "https://docs.nextcloud.com/server/latest/admin_manual/maintenance/upgrade.html",
    "Upgrade Nextcloud with the updater (or your container image) to the latest maintenance \
     release.",
);

pub const NEXTCLOUD_APPS: CheckDef = update(
    "nextcloud.apps",
    "Nextcloud apps up to date",
    Severity::Medium,
    "https://docs.nextcloud.com/server/latest/admin_manual/installation/harden_server.html",
    "Update the apps (Apps > Updates, or `occ app:update --all`).",
);

pub const ADGUARD: CheckDef = update(
    "adguard.version",
    "AdGuard Home up to date",
    Severity::Medium,
    "https://github.com/AdguardTeam/AdGuardHome/wiki/Getting-Started#update",
    "Update AdGuard Home (the Update button in the web interface, or a newer image).",
);

pub const PIHOLE: CheckDef = update(
    "pihole.version",
    "Pi-hole up to date",
    Severity::Medium,
    "https://docs.pi-hole.net/main/update/",
    "Run `pihole -up` (or pull the newer container image).",
);

pub const PLEX: CheckDef = update(
    "plex.version",
    "Plex Media Server up to date",
    Severity::Medium,
    "https://support.plex.tv/articles/200289506-updating-plex-media-server/",
    "Update Plex Media Server (Settings > General, or a newer image).",
);

pub const PAPERLESS: CheckDef = update(
    "paperless.version",
    "Paperless-ngx up to date",
    Severity::Medium,
    "https://docs.paperless-ngx.com/setup/#updating",
    "Pull the newer Paperless-ngx image and restart the stack.",
);

pub const TAILSCALE: CheckDef = update(
    "tailscale.clients",
    "Tailscale clients up to date",
    Severity::Medium,
    "https://tailscale.com/kb/1067/update",
    "Update the listed devices, or enable auto-updates (`tailscale set --auto-update`).",
);

pub const DEFS: &[&CheckDef] = &[
    &FORTIGATE_FIRMWARE,
    &FORTIGATE_LICENSE,
    &MIKROTIK_ROUTEROS,
    &MIKROTIK_ROUTERBOOT,
    &HOMEASSISTANT,
    &NEXTCLOUD,
    &NEXTCLOUD_APPS,
    &ADGUARD,
    &PIHOLE,
    &PLEX,
    &PAPERLESS,
    &TAILSCALE,
];

/// Une ligne de la table : le type d'équipement, le contrôle, la métrique
/// (drapeau 0/1 ou compte — dans les deux cas, 0 signifie « à jour »).
struct Row {
    kind: &'static str,
    def: &'static CheckDef,
    metric: &'static str,
    /// Nom à donner au compte dans la preuve ; vide pour un drapeau.
    noun: &'static str,
}

const ROWS: &[Row] = &[
    Row {
        kind: "fortigate",
        def: &FORTIGATE_FIRMWARE,
        metric: "fortigate_firmware_update_available",
        noun: "",
    },
    Row {
        kind: "fortigate",
        def: &FORTIGATE_LICENSE,
        metric: "fortigate_license_expired",
        noun: "expired licence",
    },
    Row {
        kind: "mikrotik",
        def: &MIKROTIK_ROUTEROS,
        metric: "mikrotik_update_available",
        noun: "",
    },
    Row {
        kind: "mikrotik",
        def: &MIKROTIK_ROUTERBOOT,
        metric: "mikrotik_firmware_upgrade_pending",
        noun: "",
    },
    Row {
        kind: "homeassistant",
        def: &HOMEASSISTANT,
        metric: "homeassistant_updates_available",
        noun: "update",
    },
    Row { kind: "nextcloud", def: &NEXTCLOUD, metric: "nextcloud_update_available", noun: "" },
    Row {
        kind: "nextcloud",
        def: &NEXTCLOUD_APPS,
        metric: "nextcloud_app_updates_available",
        noun: "app update",
    },
    Row { kind: "adguard", def: &ADGUARD, metric: "adguard_update_available", noun: "" },
    Row {
        kind: "pihole",
        def: &PIHOLE,
        metric: "pihole_updates_available",
        noun: "component update",
    },
    Row { kind: "plex", def: &PLEX, metric: "plex_update_available", noun: "" },
    Row { kind: "paperless", def: &PAPERLESS, metric: "paperless_update_available", noun: "" },
    Row {
        kind: "tailscale",
        def: &TAILSCALE,
        metric: "tailscale_devices_update_available",
        noun: "device",
    },
];

pub struct VendorUpdates;

impl SecurityProvider for VendorUpdates {
    fn kinds(&self) -> &'static [&'static str] {
        &[
            "fortigate",
            "mikrotik",
            "homeassistant",
            "nextcloud",
            "adguard",
            "pihole",
            "plex",
            "paperless",
            "tailscale",
        ]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[
            "fortigate_firmware_update_available",
            "fortigate_license_expired",
            "mikrotik_update_available",
            "mikrotik_firmware_upgrade_pending",
            "homeassistant_updates_available",
            "nextcloud_update_available",
            "nextcloud_app_updates_available",
            "adguard_update_available",
            "pihole_updates_available",
            "plex_update_available",
            "paperless_update_available",
            "tailscale_devices_update_available",
        ]
    }

    fn evaluate(&self, kind: &str, facts: &Facts) -> Vec<Check> {
        evaluate_kind(kind, facts)
    }
}

/// Les contrôles d'un type précis — ce que l'API sert, pour qu'un FortiGate
/// sans donnée ne liste pas les contrôles de Pi-hole.
pub fn evaluate_kind(kind: &str, facts: &Facts) -> Vec<Check> {
    ROWS.iter().filter(|row| row.kind == kind).map(|row| evaluate_row(row, facts)).collect()
}

fn evaluate_row(row: &Row, facts: &Facts) -> Check {
    match facts.max(row.metric) {
        None => row.def.unknown("Not collected yet."),
        Some(value) if value <= 0.0 => row.def.pass("Up to date"),
        Some(_) if row.noun.is_empty() => row.def.fail("An update is available"),
        Some(value) => row.def.fail(super::plural(value, row.noun)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn chaque_ligne_vise_une_metrique_declaree_et_un_type_declare() {
        for row in ROWS {
            assert!(VendorUpdates.metrics().contains(&row.metric), "{}", row.metric);
            assert!(VendorUpdates.kinds().contains(&row.kind), "{}", row.kind);
        }
        for kind in VendorUpdates.kinds() {
            assert!(!evaluate_kind(kind, &Facts::default()).is_empty(), "{kind}");
        }
    }

    #[test]
    fn drapeau_et_compte() {
        let facts = Facts::from_pairs(&[
            ("mikrotik_update_available", &[], 1.0),
            ("mikrotik_firmware_upgrade_pending", &[], 0.0),
        ]);
        let checks = evaluate_kind("mikrotik", &facts);
        assert_eq!(checks.len(), 2);
        assert_eq!(
            (checks[0].result, checks[0].evidence.as_str()),
            (Outcome::Fail, "An update is available")
        );
        assert_eq!(checks[1].result, Outcome::Pass);

        let ha = Facts::from_pairs(&[("homeassistant_updates_available", &[], 2.0)]);
        assert_eq!(evaluate_kind("homeassistant", &ha)[0].evidence, "2 updates");
    }

    #[test]
    fn evaluate_ne_garde_que_le_type_demande() {
        let facts = Facts::from_pairs(&[("plex_update_available", &[], 0.0)]);
        let checks = VendorUpdates.evaluate("plex", &facts);
        assert_eq!(checks.len(), 1);
        assert_eq!((checks[0].id.as_str(), checks[0].result), ("plex.version", Outcome::Pass));
    }

    #[test]
    fn le_rapport_d_un_type_ne_montre_que_ses_controles() {
        let report = crate::security::build_report(3, "fw", "fortigate", &Facts::default());
        assert_eq!(report.checks.len(), 2);
        assert!(report.checks.iter().all(|c| c.id.starts_with("fortigate.")));
    }
}
