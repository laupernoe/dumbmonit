//! Proxmox VE : mises à jour, dépôts, certificat, couverture des sauvegardes.
//!
//! Sources : `collectors/proxmox/apt.rs` (mises à jour, dépôts, abonnement,
//! redémarrage), `metrics.rs` (certificat), `backup.rs` (travaux et
//! couverture). Le pare-feu du datacenter et le 2FA de `root@pam` ne sont pas
//! lus par le collecteur aujourd'hui : voir la documentation.

use super::super::facts::list;
use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};
use super::zero_everywhere;

pub const SECURITY_UPDATES: CheckDef = CheckDef {
    id: "pve.updates.security",
    title: "Security updates installed",
    category: Category::Patching,
    severity: Severity::High,
    reference: "https://pve.proxmox.com/wiki/System_Software_Updates",
    remediation: "Install pending updates on every node (Node > Updates > Upgrade, or \
                  `apt update && apt full-upgrade`). Never use `apt upgrade` alone on Proxmox VE.",
};

pub const PACKAGES: CheckDef = CheckDef {
    id: "pve.updates.packages",
    title: "Proxmox VE packages up to date",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://pve.proxmox.com/wiki/System_Software_Updates",
    remediation: "Upgrade the Proxmox VE packages on each node with `apt full-upgrade`, one node \
                  at a time in a cluster.",
};

pub const REBOOT: CheckDef = CheckDef {
    id: "pve.reboot",
    title: "Running the installed kernel",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://pve.proxmox.com/wiki/System_Software_Updates",
    remediation: "Reboot the node (migrate guests first) so the newer installed kernel and its \
                  security fixes are actually running.",
};

pub const REPO_TEST: CheckDef = CheckDef {
    id: "pve.repo.test",
    title: "No test repository in use",
    category: Category::Configuration,
    severity: Severity::Medium,
    reference: "https://pve.proxmox.com/wiki/Package_Repositories",
    remediation: "Disable the pvetest (and Ceph test) repository under Node > Updates > \
                  Repositories; it ships packages that have not been released.",
};

pub const REPO_HEALTH: CheckDef = CheckDef {
    id: "pve.repo.health",
    title: "Package repositories correctly configured",
    category: Category::Configuration,
    severity: Severity::Medium,
    reference: "https://pve.proxmox.com/wiki/Package_Repositories",
    remediation: "Open Node > Updates > Repositories and fix what Proxmox VE flags — typically the \
                  enterprise repository enabled without a subscription, which silently stops all \
                  updates.",
};

pub const SUBSCRIPTION: CheckDef = CheckDef {
    id: "pve.subscription",
    title: "Enterprise repository subscription",
    category: Category::Patching,
    severity: Severity::Low,
    reference: "https://pve.proxmox.com/wiki/Package_Repositories",
    remediation: "Proxmox recommends the enterprise repository (subscription) for production. In a \
                  homelab the no-subscription repository is acceptable: keep it updated.",
};

pub const CERTIFICATE: CheckDef = CheckDef {
    id: "pve.certificate",
    title: "Web interface certificate valid for 14+ days",
    category: Category::Encryption,
    severity: Severity::Medium,
    reference: "https://pve.proxmox.com/wiki/Certificate_Management",
    remediation: "Renew the node certificate, ideally with the built-in ACME (Let's Encrypt) \
                  client under Node > System > Certificates.",
};

pub const BACKUP_COVERAGE: CheckDef = CheckDef {
    id: "pve.backup.coverage",
    title: "Every guest is in a backup job",
    category: Category::Backup,
    severity: Severity::High,
    reference: "https://pve.proxmox.com/wiki/Backup_and_Restore",
    remediation: "Add the uncovered guests to a backup job (Datacenter > Backup), or select \
                  \"all\" with explicit exclusions.",
};

pub const BACKUP_JOBS: CheckDef = CheckDef {
    id: "pve.backup.jobs",
    title: "Backup jobs succeed",
    category: Category::Backup,
    severity: Severity::High,
    reference: "https://pve.proxmox.com/wiki/Backup_and_Restore",
    remediation: "Open the failed job's task log (Datacenter > Backup) and fix the cause: storage \
                  full or unreachable, locked guest, snapshot failure.",
};

pub const DEFS: &[&CheckDef] = &[
    &SECURITY_UPDATES,
    &PACKAGES,
    &REBOOT,
    &REPO_TEST,
    &REPO_HEALTH,
    &SUBSCRIPTION,
    &CERTIFICATE,
    &BACKUP_COVERAGE,
    &BACKUP_JOBS,
];

pub struct Proxmox;

impl SecurityProvider for Proxmox {
    fn kinds(&self) -> &'static [&'static str] {
        &["proxmox"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[
            "proxmox_node_updates_security_pending",
            "proxmox_node_pve_packages_upgradable",
            "proxmox_node_reboot_required",
            "proxmox_node_repository_enabled",
            "proxmox_node_repository_errors",
            "proxmox_node_repository_warnings",
            "proxmox_node_subscription_active",
            "proxmox_node_certificate_expiry_days",
            "proxmox_backup_guests_not_covered",
            "proxmox_backup_jobs_total",
            "proxmox_backup_job_last_ok",
        ]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![
            zero_everywhere(
                &SECURITY_UPDATES,
                facts,
                "proxmox_node_updates_security_pending",
                "node",
                "security update",
                "Not collected: listing pending updates needs the Sys.Modify privilege on the \
                 monitoring token.",
            ),
            zero_everywhere(
                &PACKAGES,
                facts,
                "proxmox_node_pve_packages_upgradable",
                "node",
                "package",
                "Not collected yet.",
            ),
            zero_everywhere(
                &REBOOT,
                facts,
                "proxmox_node_reboot_required",
                "node",
                "",
                "Not collected yet.",
            ),
            repo_test(facts),
            repo_health(facts),
            subscription(facts),
            certificate(facts),
            backup_coverage(facts),
            backup_jobs(facts),
        ]
    }
}

fn repo_test(facts: &Facts) -> Check {
    if !facts.has("proxmox_node_repository_enabled") {
        return REPO_TEST.unknown("Repository list not collected.");
    }
    let enabled: Vec<String> = facts
        .all("proxmox_node_repository_enabled")
        .filter(|f| f.value > 0.0 && f.label("repo").contains("test"))
        .map(|f| format!("{}: {}", f.label("node"), f.label("repo")))
        .collect();
    if enabled.is_empty() {
        REPO_TEST.pass("No test repository enabled")
    } else {
        REPO_TEST.fail(format!("Enabled: {}", list(&enabled)))
    }
}

fn repo_health(facts: &Facts) -> Check {
    let errors = facts.sum("proxmox_node_repository_errors");
    let warnings = facts.sum("proxmox_node_repository_warnings");
    match (errors, warnings) {
        (None, None) => REPO_HEALTH.unknown("Repository status not collected."),
        (errors, warnings) => {
            let errors = errors.unwrap_or(0.0);
            let warnings = warnings.unwrap_or(0.0);
            REPO_HEALTH.verdict(
                errors == 0.0 && warnings == 0.0,
                format!("{errors} error(s), {warnings} warning(s) reported by Proxmox VE"),
            )
        }
    }
}

fn subscription(facts: &Facts) -> Check {
    if !facts.has("proxmox_node_subscription_active") {
        return SUBSCRIPTION.unknown("Subscription status not collected.");
    }
    let without = facts.names_where("proxmox_node_subscription_active", "node", |v| v < 1.0);
    if without.is_empty() {
        SUBSCRIPTION.pass("Active subscription on every node")
    } else {
        SUBSCRIPTION.fail(format!("No active subscription: {}", list(&without)))
    }
}

fn certificate(facts: &Facts) -> Check {
    let Some(days) = facts.min("proxmox_node_certificate_expiry_days") else {
        return CERTIFICATE.unknown("Certificate not collected.");
    };
    if days <= 0.0 {
        CERTIFICATE.fail("A node certificate has expired")
    } else {
        CERTIFICATE.verdict(days > 14.0, format!("Earliest expiry in {} days", days.floor()))
    }
}

fn backup_coverage(facts: &Facts) -> Check {
    let Some(uncovered) = facts.max("proxmox_backup_guests_not_covered") else {
        return BACKUP_COVERAGE.unknown("Backup job coverage not collected.");
    };
    BACKUP_COVERAGE.verdict(
        uncovered == 0.0,
        if uncovered == 0.0 {
            "Every guest is selected by a backup job".to_string()
        } else {
            format!("{} not in any backup job", super::plural(uncovered, "guest"))
        },
    )
}

fn backup_jobs(facts: &Facts) -> Check {
    if facts.max("proxmox_backup_jobs_total") == Some(0.0) {
        return BACKUP_JOBS.fail("No backup job is configured");
    }
    if !facts.has("proxmox_backup_job_last_ok") {
        return BACKUP_JOBS.unknown("No backup job result collected yet.");
    }
    let failed = facts.names_where("proxmox_backup_job_last_ok", "job", |v| v < 1.0);
    if failed.is_empty() {
        BACKUP_JOBS.pass("Last run of every job succeeded")
    } else {
        BACKUP_JOBS.fail(format!("Last run failed: {}", list(&failed)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    fn find<'a>(checks: &'a [Check], def: &CheckDef) -> &'a Check {
        checks.iter().find(|c| c.id == def.id).expect("contrôle présent")
    }

    #[test]
    fn un_cluster_a_jour_et_sauvegarde_reussit_tout() {
        let facts = Facts::from_pairs(&[
            ("proxmox_node_updates_security_pending", &[("node", "pve1")], 0.0),
            ("proxmox_node_pve_packages_upgradable", &[("node", "pve1")], 0.0),
            ("proxmox_node_reboot_required", &[("node", "pve1")], 0.0),
            (
                "proxmox_node_repository_enabled",
                &[("node", "pve1"), ("repo", "no-subscription")],
                1.0,
            ),
            ("proxmox_node_repository_enabled", &[("node", "pve1"), ("repo", "test")], 0.0),
            ("proxmox_node_repository_errors", &[("node", "pve1")], 0.0),
            ("proxmox_node_repository_warnings", &[("node", "pve1")], 0.0),
            ("proxmox_node_subscription_active", &[("node", "pve1")], 1.0),
            ("proxmox_node_certificate_expiry_days", &[("node", "pve1")], 80.0),
            ("proxmox_backup_guests_not_covered", &[], 0.0),
            ("proxmox_backup_jobs_total", &[], 1.0),
            ("proxmox_backup_job_last_ok", &[("job", "daily")], 1.0),
        ]);
        let checks = Proxmox.evaluate("", &facts);
        assert_eq!(checks.len(), DEFS.len());
        for check in &checks {
            assert_eq!(check.result, Outcome::Pass, "{check:?}");
        }
    }

    #[test]
    fn mises_a_jour_et_redemarrage_en_attente() {
        let facts = Facts::from_pairs(&[
            ("proxmox_node_updates_security_pending", &[("node", "pve1")], 2.0),
            ("proxmox_node_updates_security_pending", &[("node", "pve2")], 0.0),
            ("proxmox_node_pve_packages_upgradable", &[("node", "pve2")], 5.0),
            ("proxmox_node_reboot_required", &[("node", "pve1")], 1.0),
        ]);
        let checks = Proxmox.evaluate("", &facts);
        let security = find(&checks, &SECURITY_UPDATES);
        assert_eq!(security.result, Outcome::Fail);
        assert_eq!(security.evidence, "pve1: 2 security updates");
        assert_eq!(find(&checks, &PACKAGES).evidence, "pve2: 5 packages");
        let reboot = find(&checks, &REBOOT);
        assert_eq!((reboot.result, reboot.evidence.as_str()), (Outcome::Fail, "pve1"));
    }

    #[test]
    fn sans_sys_modify_les_mises_a_jour_de_securite_sont_inconnues() {
        let checks = Proxmox.evaluate("", &Facts::default());
        let security = find(&checks, &SECURITY_UPDATES);
        assert_eq!(security.result, Outcome::Unknown);
        assert!(security.evidence.contains("Sys.Modify"));
    }

    #[test]
    fn depot_de_test_et_avertissements() {
        let facts = Facts::from_pairs(&[
            ("proxmox_node_repository_enabled", &[("node", "pve1"), ("repo", "test")], 1.0),
            ("proxmox_node_repository_warnings", &[("node", "pve1")], 1.0),
        ]);
        let checks = Proxmox.evaluate("", &facts);
        let test = find(&checks, &REPO_TEST);
        assert_eq!(test.result, Outcome::Fail);
        assert_eq!(test.evidence, "Enabled: pve1: test");
        assert_eq!(find(&checks, &REPO_HEALTH).result, Outcome::Fail);
    }

    #[test]
    fn abonnement_et_certificat() {
        let facts = Facts::from_pairs(&[
            ("proxmox_node_subscription_active", &[("node", "pve1")], 0.0),
            ("proxmox_node_certificate_expiry_days", &[("node", "pve1")], 9.5),
        ]);
        let checks = Proxmox.evaluate("", &facts);
        assert_eq!(find(&checks, &SUBSCRIPTION).evidence, "No active subscription: pve1");
        let cert = find(&checks, &CERTIFICATE);
        assert_eq!(
            (cert.result, cert.evidence.as_str()),
            (Outcome::Fail, "Earliest expiry in 9 days")
        );

        let expired =
            Facts::from_pairs(&[("proxmox_node_certificate_expiry_days", &[("node", "a")], -2.0)]);
        assert_eq!(
            find(&Proxmox.evaluate("", &expired), &CERTIFICATE).evidence,
            "A node certificate has expired"
        );
    }

    #[test]
    fn sauvegardes_absentes_ou_en_echec() {
        let facts = Facts::from_pairs(&[
            ("proxmox_backup_guests_not_covered", &[], 3.0),
            ("proxmox_backup_jobs_total", &[], 2.0),
            ("proxmox_backup_job_last_ok", &[("job", "nightly")], 0.0),
            ("proxmox_backup_job_last_ok", &[("job", "weekly")], 1.0),
        ]);
        let checks = Proxmox.evaluate("", &facts);
        assert_eq!(find(&checks, &BACKUP_COVERAGE).evidence, "3 guests not in any backup job");
        assert_eq!(find(&checks, &BACKUP_JOBS).evidence, "Last run failed: nightly");

        let none = Facts::from_pairs(&[("proxmox_backup_jobs_total", &[], 0.0)]);
        let jobs = Proxmox.evaluate("", &none);
        assert_eq!(find(&jobs, &BACKUP_JOBS).result, Outcome::Fail);
    }
}
