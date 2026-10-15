//! Synology DSM : version majeure, mise à jour disponible, compte `admin`,
//! blocage automatique, pare-feu et sauvegardes Hyper Backup.
//!
//! Les quatre réglages de sécurité (`synology_update_available`,
//! `synology_admin_account_enabled`, `synology_autoblock_enabled`,
//! `synology_firewall_enabled`) ne sont lus qu'avec un compte administrateur,
//! une fois par heure (`collectors/synology/security.rs`) : avec un compte
//! ordinaire ils sont inconnus, donc non comptés. Le 2FA et la redirection
//! HTTPS ne sont pas lus : voir la documentation.

use super::super::facts::list;
use super::super::{Category, Check, CheckDef, Facts, Guide, SecurityProvider, Severity};

pub const DSM_VERSION: CheckDef = CheckDef {
    id: "dsm.version",
    title: "DSM 7 or later",
    category: Category::Patching,
    severity: Severity::High,
    reference: "https://www.synology.com/en-global/products/status",
    remediation: "Upgrade to the latest DSM 7 release your model supports (Control Panel > Update \
                  & Restore). Older major versions no longer receive regular security fixes.",
};

pub const BACKUP_CONFIGURED: CheckDef = CheckDef {
    id: "dsm.backup.configured",
    title: "Hyper Backup task configured",
    category: Category::Backup,
    severity: Severity::Medium,
    reference: "https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS",
    remediation: "Create a Hyper Backup task to another device or a cloud destination: RAID and \
                  snapshots are not a backup.",
};

pub const BACKUP_RESULT: CheckDef = CheckDef {
    id: "dsm.backup.result",
    title: "Hyper Backup tasks succeed",
    category: Category::Backup,
    severity: Severity::High,
    reference: "https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS",
    remediation: "Open Hyper Backup, read the failed task's log and fix the destination \
                  (reachability, credentials, free space) before running it again.",
};

pub const DSM_UPDATE: CheckDef = CheckDef {
    id: "dsm.update",
    title: "DSM up to date",
    category: Category::Patching,
    severity: Severity::High,
    reference: "https://www.synology.com/en-global/security/advisory",
    remediation: "Install the pending DSM update (Control Panel > Update & Restore) and let DSM \
                  install important updates automatically.",
};

pub const ADMIN_ACCOUNT: CheckDef = CheckDef {
    id: "dsm.admin_disabled",
    title: "Built-in admin account disabled",
    category: Category::Authentication,
    severity: Severity::Medium,
    reference: "https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS",
    remediation: "Create a personal administrator, then disable the built-in `admin` account \
                  (Control Panel > User & Group): it is the first name every brute-force tries.",
};

pub const AUTOBLOCK: CheckDef = CheckDef {
    id: "dsm.autoblock",
    title: "Auto Block enabled",
    category: Category::Authentication,
    severity: Severity::Medium,
    reference: "https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/connection_security_autoblock",
    remediation: "Turn on Auto Block (Control Panel > Security > Protection) so repeated failed \
                  sign-ins block the source address.",
};

pub const FIREWALL: CheckDef = CheckDef {
    id: "dsm.firewall",
    title: "DSM firewall enabled",
    category: Category::Exposure,
    severity: Severity::Medium,
    reference: "https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/connection_security_firewall",
    remediation: "Turn on the firewall (Control Panel > Security > Firewall) and allow DSM \
                  services only from your local network or VPN.",
};

pub const DEFS: &[&CheckDef] = &[
    &DSM_VERSION,
    &BACKUP_CONFIGURED,
    &BACKUP_RESULT,
    &DSM_UPDATE,
    &ADMIN_ACCOUNT,
    &AUTOBLOCK,
    &FIREWALL,
];

/// Réglage lu seulement avec un compte administrateur.
const ADMIN_ONLY: &str = "Not readable with this account (needs an administrator).";

/// Par où commencer pour améliorer la note.
pub const SYNOLOGY_GUIDE: Guide = Guide {
    product: "Synology DSM",
    anchor: "synology",
    steps: &[
        "Run the latest DSM 7 release for your model and let DSM install important updates automatically (Control Panel > Update & Restore).",
        "Disable the built-in admin account; sign in with a personal administrator protected by 2-step verification (Control Panel > User & Group, Security > Account).",
        "Turn on Auto Block and the firewall (Control Panel > Security), and do not forward DSM ports to the internet: use a VPN.",
        "Back up to another device or a cloud destination with Hyper Backup, and fix any task that fails: RAID and snapshots are not backups.",
    ],
};

pub struct Synology;

impl SecurityProvider for Synology {
    fn kinds(&self) -> &'static [&'static str] {
        &["synology"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[
            "synology_system_info",
            "synology_backup_tasks",
            "synology_backup_last_result",
            "synology_update_available",
            "synology_admin_account_enabled",
            "synology_autoblock_enabled",
            "synology_firewall_enabled",
        ]
    }

    fn guide(&self, _kind: &str, _facts: &Facts) -> Guide {
        SYNOLOGY_GUIDE
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        let flag = |def: &CheckDef, metric: &str, ok_when: f64, ok: &str, ko: &str| match facts
            .max(metric)
        {
            None => def.unknown(ADMIN_ONLY),
            Some(v) if v == ok_when => def.pass(ok),
            Some(_) => def.fail(ko),
        };
        vec![
            dsm_version(facts),
            backup_configured(facts),
            backup_result(facts),
            flag(
                &DSM_UPDATE,
                "synology_update_available",
                0.0,
                "No update pending",
                "A DSM update is available",
            ),
            flag(&ADMIN_ACCOUNT, "synology_admin_account_enabled", 0.0, "Disabled", "Enabled"),
            flag(&AUTOBLOCK, "synology_autoblock_enabled", 1.0, "Enabled", "Disabled"),
            flag(&FIREWALL, "synology_firewall_enabled", 1.0, "Enabled", "Disabled"),
        ]
    }
}

/// Version majeure dans « DSM 7.2.1-69057 Update 5 ».
pub fn dsm_major(raw: &str) -> Option<u32> {
    let rest = raw.trim().trim_start_matches("DSM").trim();
    rest.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn dsm_version(facts: &Facts) -> Check {
    let Some(raw) =
        facts.all("synology_system_info").map(|f| f.label("dsm_version")).find(|v| !v.is_empty())
    else {
        return DSM_VERSION.unknown("DSM version not collected.");
    };
    match dsm_major(raw) {
        Some(major) => DSM_VERSION.verdict(major >= 7, raw.to_string()),
        None => DSM_VERSION.unknown(format!("Unrecognised version \"{raw}\"")),
    }
}

fn backup_configured(facts: &Facts) -> Check {
    match facts.max("synology_backup_tasks") {
        None => {
            BACKUP_CONFIGURED.unknown("Hyper Backup not collected (package absent or no access).")
        }
        Some(tasks) => BACKUP_CONFIGURED.verdict(tasks > 0.0, super::plural(tasks, "task")),
    }
}

fn backup_result(facts: &Facts) -> Check {
    if !facts.has("synology_backup_last_result") {
        return BACKUP_RESULT.unknown("No Hyper Backup result collected.");
    }
    // 2 : échec franc (`failed`, `error`, `broken`…), voir
    // `collectors/synology/backup.rs::result_severity`.
    let failed = facts.names_where("synology_backup_last_result", "name", |v| v >= 2.0);
    if failed.is_empty() {
        BACKUP_RESULT.pass("No task in error")
    } else {
        BACKUP_RESULT.fail(format!("Failed: {}", list(&failed)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn la_version_majeure_se_lit_dans_le_libelle() {
        assert_eq!(dsm_major("DSM 7.2.1-69057 Update 5"), Some(7));
        assert_eq!(dsm_major("DSM 6.2.4-25556"), Some(6));
        assert_eq!(dsm_major("7.1"), Some(7));
        assert_eq!(dsm_major("unknown"), None);
    }

    #[test]
    fn dsm_6_echoue_dsm_7_reussit() {
        let old = Facts::from_pairs(&[(
            "synology_system_info",
            &[("dsm_version", "DSM 6.2.4-25556")],
            1.0,
        )]);
        let check = &Synology.evaluate("", &old)[0];
        assert_eq!((check.result, check.evidence.as_str()), (Outcome::Fail, "DSM 6.2.4-25556"));
        let new =
            Facts::from_pairs(&[("synology_system_info", &[("dsm_version", "DSM 7.2.2")], 1.0)]);
        assert_eq!(Synology.evaluate("", &new)[0].result, Outcome::Pass);
    }

    #[test]
    fn sauvegardes_absentes_ou_en_echec() {
        let facts = Facts::from_pairs(&[
            ("synology_backup_tasks", &[], 2.0),
            ("synology_backup_last_result", &[("name", "Offsite"), ("result", "failed")], 2.0),
            ("synology_backup_last_result", &[("name", "Local"), ("result", "done")], 0.0),
        ]);
        let checks = Synology.evaluate("", &facts);
        assert_eq!(checks[1].result, Outcome::Pass);
        assert_eq!(
            (checks[2].result, checks[2].evidence.as_str()),
            (Outcome::Fail, "Failed: Offsite")
        );

        let none = Facts::from_pairs(&[("synology_backup_tasks", &[], 0.0)]);
        let checks = Synology.evaluate("", &none);
        assert_eq!(checks[1].result, Outcome::Fail);
        assert_eq!(checks[2].result, Outcome::Unknown);
    }

    #[test]
    fn reglages_de_securite_dsm() {
        let facts = Facts::from_pairs(&[
            ("synology_update_available", &[], 1.0),
            ("synology_admin_account_enabled", &[], 1.0),
            ("synology_autoblock_enabled", &[], 1.0),
            ("synology_firewall_enabled", &[], 0.0),
        ]);
        let checks = Synology.evaluate("", &facts);
        let result = |def: &CheckDef| checks.iter().find(|c| c.id == def.id).unwrap().result;
        assert_eq!(result(&DSM_UPDATE), Outcome::Fail);
        assert_eq!(result(&ADMIN_ACCOUNT), Outcome::Fail);
        assert_eq!(result(&AUTOBLOCK), Outcome::Pass);
        assert_eq!(result(&FIREWALL), Outcome::Fail);
        // Compte non administrateur : inconnus, pas comptés.
        let checks = Synology.evaluate("", &Facts::default());
        assert!(checks[3..].iter().all(|c| c.result == Outcome::Unknown));
    }
}
