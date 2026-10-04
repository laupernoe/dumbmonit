//! Machines sous agent (Linux, Windows) et leurs conteneurs Docker.
//!
//! Ce que l'agent remonte aujourd'hui : mises à jour en attente (dont
//! sécurité, selon la distribution), redémarrage requis, mode SELinux, et
//! pour Docker l'âge des images et la disponibilité d'une image plus récente.
//! Ni l'état du pare-feu de l'hôte, ni les conteneurs privilégiés ou lancés
//! en root ne sont remontés : voir la documentation.

use super::super::facts::list;
use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};

pub const SECURITY_UPDATES: CheckDef = CheckDef {
    id: "host.updates.security",
    title: "Security updates installed",
    category: Category::Patching,
    severity: Severity::High,
    reference: "https://www.cisecurity.org/controls/continuous-vulnerability-management",
    remediation: "Install pending security updates (apt/dnf/zypper) and consider automatic \
                  security updates (unattended-upgrades, dnf-automatic).",
};

pub const UPDATES: CheckDef = CheckDef {
    id: "host.updates",
    title: "No pending updates",
    category: Category::Patching,
    severity: Severity::Low,
    reference: "https://www.cisecurity.org/controls/continuous-vulnerability-management",
    remediation: "Apply pending updates during the next maintenance window.",
};

pub const REBOOT: CheckDef = CheckDef {
    id: "host.reboot",
    title: "No reboot pending",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://www.cisecurity.org/controls/continuous-vulnerability-management",
    remediation: "Reboot the machine: an updated kernel or library is installed but the old one is \
                  still running.",
};

pub const SELINUX: CheckDef = CheckDef {
    id: "host.selinux",
    title: "SELinux enforcing",
    category: Category::Configuration,
    severity: Severity::Medium,
    reference: "https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index",
    remediation: "Set SELINUX=enforcing in /etc/selinux/config and fix the denials in the audit \
                  log rather than disabling the policy.",
};

pub const CONTAINER_UPDATES: CheckDef = CheckDef {
    id: "docker.image.updates",
    title: "Container images up to date",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often",
    remediation: "Pull the newer images and recreate the containers (`docker compose pull && \
                  docker compose up -d`).",
};

pub const CONTAINER_AGE: CheckDef = CheckDef {
    id: "docker.image.age",
    title: "No container image older than 180 days",
    category: Category::Patching,
    severity: Severity::Low,
    reference: "https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often",
    remediation: "Rebuild or re-pull old images: even a pinned tag needs its base layers \
                  refreshed to receive security fixes.",
};

pub const DEFS: &[&CheckDef] =
    &[&SECURITY_UPDATES, &UPDATES, &REBOOT, &SELINUX, &CONTAINER_UPDATES, &CONTAINER_AGE];

/// Au-delà, une image n'a vraisemblablement pas reçu les correctifs de sa base.
const MAX_IMAGE_AGE_DAYS: f64 = 180.0;

pub struct Agent;

impl SecurityProvider for Agent {
    fn kinds(&self) -> &'static [&'static str] {
        &["agent"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[
            "agent_security_updates_pending",
            "agent_updates_pending",
            "agent_reboot_required",
            "agent_selinux_mode",
            "container_update_available",
            "container_image_age_seconds",
        ]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        let count =
            |def: &CheckDef, metric: &str, noun: &str, missing: &str| match facts.max(metric) {
                None => def.unknown(missing),
                Some(n) if n <= 0.0 => def.pass("None"),
                Some(n) => def.fail(super::plural(n, noun)),
            };
        let mut checks = vec![
            count(
                &SECURITY_UPDATES,
                "agent_security_updates_pending",
                "security update",
                "Not reported: this package manager does not tell security updates apart.",
            ),
            count(&UPDATES, "agent_updates_pending", "update", "Not reported by the agent."),
            match facts.max("agent_reboot_required") {
                None => REBOOT.unknown("Not reported by the agent."),
                Some(r) => {
                    REBOOT.verdict(r == 0.0, if r == 0.0 { "No" } else { "Reboot required" })
                }
            },
            match facts.max("agent_selinux_mode") {
                None => SELINUX.unknown("SELinux not present (or not reported) on this machine."),
                Some(mode) => SELINUX.verdict(
                    mode >= 2.0,
                    match mode as i64 {
                        2 => "Enforcing",
                        1 => "Permissive: denials are logged, not enforced",
                        _ => "Disabled",
                    },
                ),
            },
        ];
        // Les contrôles Docker n'existent que sur une machine qui a des
        // conteneurs : ailleurs ils ne sont pas « inconnus », ils sont sans objet.
        if facts.has("container_update_available") {
            checks.push(container_updates(facts));
        }
        if facts.has("container_image_age_seconds") {
            let old = facts.names_where("container_image_age_seconds", "container", |age| {
                age > MAX_IMAGE_AGE_DAYS * 86_400.0
            });
            checks.push(if old.is_empty() {
                CONTAINER_AGE.pass("Every image is younger than 180 days")
            } else {
                CONTAINER_AGE.fail(list(&old))
            });
        }
        checks
    }
}

fn container_updates(facts: &Facts) -> Check {
    // -1 : la vérification n'a pas pu se faire (registre privé, limite de
    // requêtes, contrôle désactivé).
    if facts.all("container_update_available").all(|f| f.value < 0.0) {
        return CONTAINER_UPDATES.unknown("Image update check disabled or unavailable.");
    }
    let outdated = facts.names_where("container_update_available", "container", |v| v > 0.0);
    if outdated.is_empty() {
        CONTAINER_UPDATES.pass("No newer image found")
    } else {
        CONTAINER_UPDATES.fail(format!("Newer image available: {}", list(&outdated)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    fn by_id<'a>(checks: &'a [Check], def: &CheckDef) -> Option<&'a Check> {
        checks.iter().find(|c| c.id == def.id)
    }

    #[test]
    fn une_machine_a_jour() {
        let facts = Facts::from_pairs(&[
            ("agent_security_updates_pending", &[], 0.0),
            ("agent_updates_pending", &[], 0.0),
            ("agent_reboot_required", &[], 0.0),
            ("agent_selinux_mode", &[], 2.0),
        ]);
        let checks = Agent.evaluate("", &facts);
        assert_eq!(checks.len(), 4, "pas de contrôle Docker sans conteneur");
        assert!(checks.iter().all(|c| c.result == Outcome::Pass));
    }

    #[test]
    fn mises_a_jour_redemarrage_et_selinux_permissif() {
        let facts = Facts::from_pairs(&[
            ("agent_security_updates_pending", &[], 3.0),
            ("agent_updates_pending", &[], 12.0),
            ("agent_reboot_required", &[], 1.0),
            ("agent_selinux_mode", &[], 1.0),
        ]);
        let checks = Agent.evaluate("", &facts);
        assert!(checks.iter().all(|c| c.result == Outcome::Fail));
        assert_eq!(by_id(&checks, &SECURITY_UPDATES).unwrap().evidence, "3 security updates");
        assert!(by_id(&checks, &SELINUX).unwrap().evidence.starts_with("Permissive"));
    }

    #[test]
    fn conteneurs_en_retard_et_images_anciennes() {
        let facts = Facts::from_pairs(&[
            ("container_update_available", &[("container", "nextcloud")], 1.0),
            ("container_update_available", &[("container", "db")], 0.0),
            ("container_update_available", &[("container", "private")], -1.0),
            ("container_image_age_seconds", &[("container", "db")], 400.0 * 86_400.0),
            ("container_image_age_seconds", &[("container", "nextcloud")], 3.0 * 86_400.0),
        ]);
        let checks = Agent.evaluate("", &facts);
        let updates = by_id(&checks, &CONTAINER_UPDATES).unwrap();
        assert_eq!(updates.evidence, "Newer image available: nextcloud");
        let age = by_id(&checks, &CONTAINER_AGE).unwrap();
        assert_eq!((age.result, age.evidence.as_str()), (Outcome::Fail, "db"));
    }

    #[test]
    fn verification_d_image_desactivee() {
        let facts =
            Facts::from_pairs(&[("container_update_available", &[("container", "a")], -1.0)]);
        let checks = Agent.evaluate("", &facts);
        assert_eq!(by_id(&checks, &CONTAINER_UPDATES).unwrap().result, Outcome::Unknown);
    }
}
