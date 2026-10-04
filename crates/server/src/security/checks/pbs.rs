//! Proxmox Backup Server : mises à jour, vérification, élagage, copie hors
//! site, intégrité des blocs, certificat.
//!
//! Le chiffrement côté client des sauvegardes n'est pas lu par le
//! collecteur aujourd'hui (il faudrait parcourir les manifestes) : voir la
//! documentation.

use super::super::facts::{list, plural};
use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};
use super::zero_everywhere;

pub const UPDATES: CheckDef = CheckDef {
    id: "pbs.updates",
    title: "System updates installed",
    category: Category::Patching,
    severity: Severity::High,
    reference: "https://pbs.proxmox.com/docs/package-repositories.html",
    remediation: "Install pending updates (Administration > Updates, or `apt update && apt \
                  full-upgrade`).",
};

pub const RESTART: CheckDef = CheckDef {
    id: "pbs.restart",
    title: "Running the installed Backup Server version",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://pbs.proxmox.com/docs/package-repositories.html",
    remediation: "The proxmox-backup package was upgraded but the daemons still run the old \
                  version: restart them (`systemctl restart proxmox-backup-proxy proxmox-backup`) \
                  or reboot.",
};

pub const VERIFY: CheckDef = CheckDef {
    id: "pbs.verify",
    title: "Verify job scheduled",
    category: Category::Backup,
    severity: Severity::High,
    reference: "https://pbs.proxmox.com/docs/maintenance.html#verification",
    remediation: "Create a verify job on each datastore (Datastore > Verify Jobs) so silent \
                  corruption is found before you need the backup.",
};

pub const PRUNE: CheckDef = CheckDef {
    id: "pbs.prune",
    title: "Prune job scheduled",
    category: Category::Backup,
    severity: Severity::Medium,
    reference: "https://pbs.proxmox.com/docs/maintenance.html#pruning",
    remediation: "Create a prune job (Datastore > Prune & GC Jobs) with a retention policy, or the \
                  datastore grows until backups fail.",
};

pub const OFFSITE: CheckDef = CheckDef {
    id: "pbs.offsite",
    title: "Backups copied elsewhere (sync or tape)",
    category: Category::Backup,
    severity: Severity::Medium,
    reference: "https://pbs.proxmox.com/docs/managing-remotes.html",
    remediation: "Follow the 3-2-1 rule: sync the datastore to a second Backup Server (remote + \
                  sync job) or write it to tape.",
};

pub const INTEGRITY: CheckDef = CheckDef {
    id: "pbs.integrity",
    title: "Garbage collection clean, no bad chunks",
    category: Category::Backup,
    severity: Severity::High,
    reference: "https://pbs.proxmox.com/docs/maintenance.html#garbage-collection",
    remediation: "Read the last garbage collection task log; bad chunks mean damaged backups — \
                  run a verify job and re-run the affected backups.",
};

pub const CERTIFICATE: CheckDef = CheckDef {
    id: "pbs.certificate",
    title: "Certificate valid for 14+ days",
    category: Category::Encryption,
    severity: Severity::Medium,
    reference: "https://pbs.proxmox.com/docs/certificate-management.html",
    remediation: "Renew the certificate, ideally with the built-in ACME client (Certificates > \
                  ACME). Clients pinning the fingerprint must be updated afterwards.",
};

pub const DEFS: &[&CheckDef] =
    &[&UPDATES, &RESTART, &VERIFY, &PRUNE, &OFFSITE, &INTEGRITY, &CERTIFICATE];

pub struct Pbs;

impl SecurityProvider for Pbs {
    fn kinds(&self) -> &'static [&'static str] {
        &["pbs"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[
            "pbs_node_updates_pending",
            "pbs_node_running_version_stale",
            "pbs_verify_jobs_total",
            "pbs_prune_jobs_total",
            "pbs_sync_jobs_total",
            "pbs_tape_backup_jobs_total",
            "pbs_gc_last_run_ok",
            "pbs_gc_bad_chunks",
            "pbs_node_certificate_expires_seconds",
        ]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![
            zero_everywhere(
                &UPDATES,
                facts,
                "pbs_node_updates_pending",
                "",
                "update",
                "Not collected: listing updates needs the Sys.Modify privilege on /system.",
            ),
            restart(facts),
            jobs(&VERIFY, facts, "pbs_verify_jobs_total", "verify"),
            jobs(&PRUNE, facts, "pbs_prune_jobs_total", "prune"),
            offsite(facts),
            integrity(facts),
            certificate(facts),
        ]
    }
}

fn jobs(def: &CheckDef, facts: &Facts, metric: &str, what: &str) -> Check {
    match facts.max(metric) {
        None => def.unknown("Job list not collected."),
        Some(count) => def.verdict(count > 0.0, plural(count, &format!("{what} job"))),
    }
}

fn restart(facts: &Facts) -> Check {
    match facts.max("pbs_node_running_version_stale") {
        None => RESTART.unknown("Not collected yet."),
        Some(stale) if stale > 0.0 => {
            RESTART.fail("The daemons still run the version before the last upgrade")
        }
        Some(_) => RESTART.pass("The daemons run the installed version"),
    }
}

fn offsite(facts: &Facts) -> Check {
    let sync = facts.max("pbs_sync_jobs_total");
    let tape = facts.max("pbs_tape_backup_jobs_total");
    if sync.is_none() && tape.is_none() {
        return OFFSITE.unknown("Sync and tape jobs not collected.");
    }
    let sync = sync.unwrap_or(0.0);
    let tape = tape.unwrap_or(0.0);
    OFFSITE.verdict(
        sync + tape > 0.0,
        format!("{}, {}", plural(sync, "sync job"), plural(tape, "tape backup job")),
    )
}

fn integrity(facts: &Facts) -> Check {
    if !facts.has("pbs_gc_last_run_ok") && !facts.has("pbs_gc_bad_chunks") {
        return INTEGRITY.unknown("Garbage collection status not collected.");
    }
    let failed = facts.names_where("pbs_gc_last_run_ok", "datastore", |v| v < 1.0);
    let bad: Vec<String> = facts
        .all("pbs_gc_bad_chunks")
        .filter(|f| f.value > 0.0)
        .map(|f| format!("{}: {}", f.label("datastore"), plural(f.value, "bad chunk")))
        .collect();
    let mut problems = Vec::new();
    if !failed.is_empty() {
        problems.push(format!("last GC failed on {}", list(&failed)));
    }
    if !bad.is_empty() {
        problems.push(list(&bad));
    }
    if problems.is_empty() {
        INTEGRITY.pass("Last garbage collection succeeded, no bad chunk")
    } else {
        INTEGRITY.fail(problems.join("; "))
    }
}

fn certificate(facts: &Facts) -> Check {
    let Some(seconds) = facts.min("pbs_node_certificate_expires_seconds") else {
        return CERTIFICATE.unknown("Certificate not collected.");
    };
    if seconds <= 0.0 {
        return CERTIFICATE.fail("The certificate has expired");
    }
    let days = (seconds / 86_400.0).floor();
    CERTIFICATE.verdict(days >= 14.0, format!("Expires in {days} days"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    fn result(checks: &[Check], def: &CheckDef) -> (Outcome, String) {
        let check = checks.iter().find(|c| c.id == def.id).unwrap();
        (check.result, check.evidence.clone())
    }

    #[test]
    fn un_serveur_bien_tenu() {
        let facts = Facts::from_pairs(&[
            ("pbs_node_updates_pending", &[], 0.0),
            ("pbs_node_running_version_stale", &[], 0.0),
            ("pbs_verify_jobs_total", &[], 1.0),
            ("pbs_prune_jobs_total", &[], 2.0),
            ("pbs_sync_jobs_total", &[], 1.0),
            ("pbs_gc_last_run_ok", &[("datastore", "main")], 1.0),
            ("pbs_gc_bad_chunks", &[("datastore", "main")], 0.0),
            ("pbs_node_certificate_expires_seconds", &[], 90.0 * 86_400.0),
        ]);
        let checks = Pbs.evaluate("", &facts);
        assert!(checks.iter().all(|c| c.result == Outcome::Pass), "{checks:?}");
        assert_eq!(result(&checks, &PRUNE).1, "2 prune jobs");
        assert_eq!(result(&checks, &OFFSITE).1, "1 sync job, 0 tape backup jobs");
    }

    #[test]
    fn sans_verification_ni_copie_hors_site() {
        let facts = Facts::from_pairs(&[
            ("pbs_verify_jobs_total", &[], 0.0),
            ("pbs_sync_jobs_total", &[], 0.0),
            ("pbs_tape_backup_jobs_total", &[], 0.0),
            ("pbs_node_updates_pending", &[], 4.0),
        ]);
        let checks = Pbs.evaluate("", &facts);
        assert_eq!(result(&checks, &VERIFY), (Outcome::Fail, "0 verify jobs".into()));
        assert_eq!(result(&checks, &OFFSITE).0, Outcome::Fail);
        assert_eq!(result(&checks, &UPDATES), (Outcome::Fail, "4 updates".into()));
        assert_eq!(result(&checks, &PRUNE).0, Outcome::Unknown);
    }

    #[test]
    fn blocs_abimes_et_ramasse_miettes_en_echec() {
        let facts = Facts::from_pairs(&[
            ("pbs_gc_last_run_ok", &[("datastore", "main")], 0.0),
            ("pbs_gc_bad_chunks", &[("datastore", "main")], 3.0),
        ]);
        let (outcome, evidence) = result(&Pbs.evaluate("", &facts), &INTEGRITY);
        assert_eq!(outcome, Outcome::Fail);
        assert_eq!(evidence, "last GC failed on main; main: 3 bad chunks");
    }

    #[test]
    fn certificat_proche_ou_expire() {
        let soon =
            Facts::from_pairs(&[("pbs_node_certificate_expires_seconds", &[], 3.0 * 86_400.0)]);
        assert_eq!(
            result(&Pbs.evaluate("", &soon), &CERTIFICATE),
            (Outcome::Fail, "Expires in 3 days".into())
        );
        let gone = Facts::from_pairs(&[("pbs_node_certificate_expires_seconds", &[], -1.0)]);
        assert_eq!(result(&Pbs.evaluate("", &gone), &CERTIFICATE).1, "The certificate has expired");
    }
}
