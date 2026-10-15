//! Machines sous agent (Linux, Windows) et leurs conteneurs Docker.
//!
//! Linux : mises à jour en attente (dont sécurité, selon la distribution),
//! redémarrage requis, connexion SSH de root et par mot de passe (`sshd -T`),
//! pare-feu entrant actif, mode SELinux. Windows : pare-feu par profil,
//! redémarrage en attente, Bureau à distance sans NLA, SMBv1. Docker : âge
//! des images et image plus récente disponible. Les conteneurs privilégiés ou
//! lancés en root ne sont pas remontés : voir la documentation.
//!
//! Le type `agent` couvre les deux systèmes : une machine est traitée comme
//! Windows dès qu'elle remonte une série propre à Windows. Les contrôles de
//! l'autre système sont alors sans objet — absents, pas « inconnus ».

use super::super::facts::list;
use super::super::{Category, Check, CheckDef, Facts, Guide, SecurityProvider, Severity};

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

pub const SSH_ROOT: CheckDef = CheckDef {
    id: "host.ssh.root_login",
    title: "SSH root login with a password refused",
    category: Category::Authentication,
    severity: Severity::High,
    reference: "https://man.openbsd.org/sshd_config#PermitRootLogin",
    remediation: "Set `PermitRootLogin no` (or at least `prohibit-password`) in \
                  /etc/ssh/sshd_config, sign in as a normal user and use sudo, then reload sshd.",
};

pub const SSH_PASSWORD: CheckDef = CheckDef {
    id: "host.ssh.password",
    title: "SSH accepts keys only",
    category: Category::Authentication,
    severity: Severity::Medium,
    reference: "https://man.openbsd.org/sshd_config#PasswordAuthentication",
    remediation: "Once your key works, set `PasswordAuthentication no` and \
                  `KbdInteractiveAuthentication no` in /etc/ssh/sshd_config and reload sshd: \
                  password guessing then gets nowhere.",
};

pub const FIREWALL: CheckDef = CheckDef {
    id: "host.firewall",
    title: "Host firewall filters inbound traffic",
    category: Category::Exposure,
    severity: Severity::High,
    reference: "https://www.cisecurity.org/controls/secure-configuration-of-enterprise-assets-and-software",
    remediation: "Enable a firewall that drops unsolicited inbound traffic and opens only the \
                  services you need: `ufw enable` on Ubuntu, firewalld on Fedora and RHEL, an \
                  nftables ruleset elsewhere.",
};

pub const WIN_FIREWALL: CheckDef = CheckDef {
    id: "windows.firewall",
    title: "Windows Firewall on for every profile",
    category: Category::Exposure,
    severity: Severity::High,
    reference: "https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/",
    remediation: "Turn Windows Defender Firewall on for the domain, private and public profiles \
                  (wf.msc, or `Set-NetFirewallProfile -All -Enabled True`), and open only the \
                  ports you need.",
};

pub const WIN_RDP_NLA: CheckDef = CheckDef {
    id: "windows.rdp.nla",
    title: "Remote Desktop requires Network Level Authentication",
    category: Category::Authentication,
    severity: Severity::High,
    reference: "https://learn.microsoft.com/en-us/windows-server/remote/remote-desktop-services/clients/remote-desktop-allow-access",
    remediation: "In System > Remote Desktop, keep \"Require devices to use Network Level \
                  Authentication\" checked, and never expose RDP to the internet: use a VPN.",
};

pub const WIN_SMB1: CheckDef = CheckDef {
    id: "windows.smb1",
    title: "SMBv1 disabled",
    category: Category::Configuration,
    severity: Severity::High,
    reference: "https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3",
    remediation: "Remove SMBv1, the protocol WannaCry spread through: \
                  `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`, then reboot.",
};

pub const DEFS: &[&CheckDef] = &[
    &SECURITY_UPDATES,
    &UPDATES,
    &REBOOT,
    &SELINUX,
    &SSH_ROOT,
    &SSH_PASSWORD,
    &FIREWALL,
    &WIN_FIREWALL,
    &WIN_RDP_NLA,
    &WIN_SMB1,
    &CONTAINER_UPDATES,
    &CONTAINER_AGE,
];

/// Par où commencer sur une machine Linux.
pub const LINUX_GUIDE: Guide = Guide {
    product: "Linux host",
    anchor: "linux-hosts",
    steps: &[
        "Install pending security updates, then turn on automatic security updates: unattended-upgrades on Debian and Ubuntu, dnf-automatic on Fedora and RHEL.",
        "Reboot after kernel or library updates, so the fixed code is what actually runs.",
        "Check that your SSH key works, then set `PermitRootLogin no`, `PasswordAuthentication no` and `KbdInteractiveAuthentication no` in /etc/ssh/sshd_config and reload sshd.",
        "Turn on a host firewall that drops unsolicited inbound traffic (ufw, firewalld or nftables) and open only the services you use.",
        "Where the distribution ships SELinux, keep it enforcing; pull newer container images regularly.",
    ],
};

/// Par où commencer sur l'appareil Docker d'une machine : ses contrôles sont
/// décrits avec ceux des machines Linux.
pub const DOCKER_GUIDE: Guide = Guide {
    product: "Docker",
    anchor: "linux-hosts",
    steps: &[
        "Pull newer images and recreate the containers that have one waiting in the registry, or turn on auto-update for them on the device page.",
        "Rebuild or replace images older than 180 days: their base layers no longer get security fixes.",
    ],
};

/// Par où commencer sur une machine Windows.
pub const WINDOWS_GUIDE: Guide = Guide {
    product: "Windows host",
    anchor: "windows-hosts",
    steps: &[
        "Install Windows updates (Settings > Windows Update) and restart when Windows asks.",
        "Keep Windows Defender Firewall on for the domain, private and public profiles.",
        "If Remote Desktop is on, require Network Level Authentication and reach it through a VPN, never straight from the internet.",
        "Remove SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.",
    ],
};

/// Séries que seul l'agent Windows publie.
const WINDOWS_ONLY: &[&str] =
    &["agent_firewall_profile_enabled", "agent_rdp_enabled", "agent_rdp_nla", "agent_smb1_enabled"];

fn is_windows(facts: &Facts) -> bool {
    WINDOWS_ONLY.iter().any(|metric| facts.has(metric))
}

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
            "agent_ssh_root_login",
            "agent_ssh_password_auth",
            "agent_firewall_active",
            "agent_firewall_profile_enabled",
            "agent_rdp_enabled",
            "agent_rdp_nla",
            "agent_smb1_enabled",
            "container_update_available",
            "container_image_age_seconds",
        ]
    }

    fn guide(&self, _kind: &str, facts: &Facts) -> Guide {
        if is_windows(facts) { WINDOWS_GUIDE } else { LINUX_GUIDE }
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        let mut checks = if is_windows(facts) { windows(facts) } else { linux(facts) };
        // Historique d'avant la séparation des modules : les séries des
        // conteneurs ont pu rester sous la machine.
        checks.extend(container_checks(facts));
        checks
    }
}

/// Les conteneurs d'une machine, notés sur son appareil Docker
/// (`agent_service`) depuis que les services de l'agent sont des équipements.
pub struct AgentDocker;

impl SecurityProvider for AgentDocker {
    fn kinds(&self) -> &'static [&'static str] {
        &["agent_service"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["container_update_available", "container_image_age_seconds"]
    }

    /// Seul l'appareil Docker a des conteneurs ; les autres services (ZFS,
    /// tunnels…) n'ont pas de carte sécurité.
    fn applies(&self, facts: &Facts) -> bool {
        facts.has("container_update_available") || facts.has("container_image_age_seconds")
    }

    fn guide(&self, _kind: &str, _facts: &Facts) -> Guide {
        DOCKER_GUIDE
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        container_checks(facts)
    }
}

/// Les contrôles Docker n'existent que là où il y a des conteneurs : ailleurs
/// ils ne sont pas « inconnus », ils sont sans objet.
fn container_checks(facts: &Facts) -> Vec<Check> {
    let mut checks = Vec::new();
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

fn reboot(facts: &Facts) -> Check {
    match facts.max("agent_reboot_required") {
        None => REBOOT.unknown("Not reported by the agent."),
        Some(r) => REBOOT.verdict(r == 0.0, if r == 0.0 { "No" } else { "Reboot required" }),
    }
}

fn linux(facts: &Facts) -> Vec<Check> {
    let count = |def: &CheckDef, metric: &str, noun: &str, missing: &str| match facts.max(metric) {
        None => def.unknown(missing),
        Some(n) if n <= 0.0 => def.pass("None"),
        Some(n) => def.fail(super::plural(n, noun)),
    };
    vec![
        count(
            &SECURITY_UPDATES,
            "agent_security_updates_pending",
            "security update",
            "Not reported: this package manager does not tell security updates apart.",
        ),
        count(&UPDATES, "agent_updates_pending", "update", "Not reported by the agent."),
        reboot(facts),
        match facts.max("agent_ssh_root_login") {
            None => SSH_ROOT.unknown("No SSH server found, or agent too old to report it."),
            Some(v) if v >= 2.0 => SSH_ROOT.fail("PermitRootLogin yes"),
            Some(v) if v >= 1.0 => SSH_ROOT.pass("Root may sign in with a key only"),
            Some(_) => SSH_ROOT.pass("PermitRootLogin no"),
        },
        match facts.max("agent_ssh_password_auth") {
            None => SSH_PASSWORD.unknown("No SSH server found, or agent too old to report it."),
            Some(v) => SSH_PASSWORD.verdict(
                v == 0.0,
                if v == 0.0 { "Passwords refused" } else { "Passwords accepted" },
            ),
        },
        match facts.max("agent_firewall_active") {
            None => FIREWALL.unknown("No nftables or iptables ruleset could be read."),
            Some(v) => FIREWALL.verdict(
                v >= 1.0,
                if v >= 1.0 {
                    "Inbound traffic is filtered"
                } else {
                    "No inbound filtering rule found"
                },
            ),
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
    ]
}

fn windows(facts: &Facts) -> Vec<Check> {
    let firewall = if facts.has("agent_firewall_profile_enabled") {
        let off = facts.names_where("agent_firewall_profile_enabled", "profile", |v| v < 1.0);
        if off.is_empty() {
            WIN_FIREWALL.pass("On for every profile")
        } else {
            WIN_FIREWALL.fail(format!("Off for: {}", list(&off)))
        }
    } else {
        WIN_FIREWALL.unknown("Firewall profiles not reported by the agent.")
    };
    let rdp = match (facts.max("agent_rdp_enabled"), facts.max("agent_rdp_nla")) {
        (Some(on), _) if on < 1.0 => WIN_RDP_NLA.pass("Remote Desktop is off"),
        (_, Some(nla)) => WIN_RDP_NLA.verdict(
            nla >= 1.0,
            if nla >= 1.0 { "NLA required" } else { "Remote Desktop accepts sign-in without NLA" },
        ),
        _ => WIN_RDP_NLA.unknown("Remote Desktop settings not reported by the agent."),
    };
    let smb1 = match facts.max("agent_smb1_enabled") {
        None => WIN_SMB1.unknown("SMBv1 status not reported by the agent."),
        Some(on) => WIN_SMB1.verdict(on == 0.0, if on == 0.0 { "Disabled" } else { "Enabled" }),
    };
    vec![firewall, reboot(facts), rdp, smb1]
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
    fn l_appareil_docker_ne_porte_que_les_controles_des_conteneurs() {
        let facts = Facts::from_pairs(&[
            ("container_update_available", &[("container", "web")], 1.0),
            ("container_image_age_seconds", &[("container", "web")], 3.0 * 86_400.0),
        ]);
        assert!(AgentDocker.applies(&facts));
        let checks = AgentDocker.evaluate("agent_service", &facts);
        assert_eq!(checks.len(), 2);
        assert!(by_id(&checks, &CONTAINER_UPDATES).is_some());
        // Un service sans conteneur (ZFS, tunnel) n'a pas de carte sécurité.
        assert!(!AgentDocker.applies(&Facts::default()));
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
        assert_eq!(checks.len(), 7, "pas de contrôle Docker sans conteneur");
        let passed: Vec<&str> =
            checks.iter().filter(|c| c.result == Outcome::Pass).map(|c| c.id.as_str()).collect();
        assert_eq!(passed.len(), 4, "{passed:?}");
        // SSH et pare-feu non remontés (agent ancien) : inconnus, pas comptés.
        assert_eq!(by_id(&checks, &FIREWALL).unwrap().result, Outcome::Unknown);
        assert_eq!(Agent.guide("agent", &facts).anchor, "linux-hosts");
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
        assert!(checks.iter().all(|c| c.result != Outcome::Pass));
        assert_eq!(checks.iter().filter(|c| c.result == Outcome::Fail).count(), 4);
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

    #[test]
    fn ssh_et_pare_feu_linux() {
        let facts = Facts::from_pairs(&[
            ("agent_ssh_root_login", &[], 2.0),
            ("agent_ssh_password_auth", &[], 1.0),
            ("agent_firewall_active", &[], 0.0),
        ]);
        let checks = Agent.evaluate("", &facts);
        for def in [&SSH_ROOT, &SSH_PASSWORD, &FIREWALL] {
            assert_eq!(by_id(&checks, def).unwrap().result, Outcome::Fail, "{}", def.id);
        }
        let facts = Facts::from_pairs(&[
            ("agent_ssh_root_login", &[], 1.0),
            ("agent_ssh_password_auth", &[], 0.0),
            ("agent_firewall_active", &[], 1.0),
        ]);
        let checks = Agent.evaluate("", &facts);
        let root = by_id(&checks, &SSH_ROOT).unwrap();
        assert_eq!(
            (root.result, root.evidence.as_str()),
            (Outcome::Pass, "Root may sign in with a key only")
        );
        assert_eq!(by_id(&checks, &SSH_PASSWORD).unwrap().result, Outcome::Pass);
        assert_eq!(by_id(&checks, &FIREWALL).unwrap().result, Outcome::Pass);
    }

    #[test]
    fn une_machine_windows_n_a_que_ses_controles() {
        let facts = Facts::from_pairs(&[
            ("agent_firewall_profile_enabled", &[("profile", "domain")], 1.0),
            ("agent_firewall_profile_enabled", &[("profile", "private")], 1.0),
            ("agent_firewall_profile_enabled", &[("profile", "public")], 0.0),
            ("agent_reboot_required", &[], 0.0),
            ("agent_rdp_enabled", &[], 1.0),
            ("agent_rdp_nla", &[], 0.0),
            ("agent_smb1_enabled", &[], 0.0),
        ]);
        let checks = Agent.evaluate("agent", &facts);
        let ids: Vec<&str> = checks.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["windows.firewall", "host.reboot", "windows.rdp.nla", "windows.smb1"]);
        let firewall = by_id(&checks, &WIN_FIREWALL).unwrap();
        assert_eq!(
            (firewall.result, firewall.evidence.as_str()),
            (Outcome::Fail, "Off for: public")
        );
        assert_eq!(by_id(&checks, &WIN_RDP_NLA).unwrap().result, Outcome::Fail);
        assert_eq!(by_id(&checks, &WIN_SMB1).unwrap().result, Outcome::Pass);
        assert_eq!(Agent.guide("agent", &facts).anchor, "windows-hosts");

        // Bureau à distance coupé : NLA sans objet, le contrôle réussit.
        let off = Facts::from_pairs(&[("agent_rdp_enabled", &[], 0.0)]);
        assert_eq!(by_id(&Agent.evaluate("", &off), &WIN_RDP_NLA).unwrap().result, Outcome::Pass);
    }
}
