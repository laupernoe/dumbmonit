//! Synology DSM : version majeure et sauvegardes Hyper Backup.
//!
//! Le collecteur lit le système, le stockage et Hyper Backup ; il ne lit
//! aujourd'hui ni le 2FA, ni l'état du compte `admin`, ni le blocage
//! automatique, ni le pare-feu, ni la redirection HTTPS. Ces contrôles
//! demandent de nouvelles lectures (`SYNO.Core.Security.*`) et sont listés
//! dans la documentation plutôt que devinés ici.

use super::super::facts::list;
use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};

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

pub const DEFS: &[&CheckDef] = &[&DSM_VERSION, &BACKUP_CONFIGURED, &BACKUP_RESULT];

pub struct Synology;

impl SecurityProvider for Synology {
    fn kinds(&self) -> &'static [&'static str] {
        &["synology"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["synology_system_info", "synology_backup_tasks", "synology_backup_last_result"]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![dsm_version(facts), backup_configured(facts), backup_result(facts)]
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
}
