//! Active Directory : les constats façon PingCastle deviennent des contrôles.
//!
//! Le collecteur `activedirectory` (écrit à part) produit des constats —
//! comptes kerberoastables, délégation non contrainte, âge du mot de passe
//! `krbtgt`… Ils se branchent ici sans code supplémentaire par l'un des deux
//! chemins suivants.
//!
//! # 1. Par les métriques (chemin par défaut)
//!
//! Une série par constat, toujours publiée (0 quand rien n'est trouvé, pour
//! qu'un constat corrigé passe au vert plutôt que de disparaître) :
//!
//! ```text
//! dumbmonit_ad_finding_count{finding="kerberoastable_users", severity="high", title="Kerberoastable user accounts"} 3
//! ```
//!
//! * `finding` : identifiant stable du constat (voir [`CATALOGUE`] pour ceux
//!   qui ont déjà une référence et une remédiation rédigées) ;
//! * `severity` : `critical` | `high` | `medium` | `low` (`info` compte
//!   comme `low`) ;
//! * `title` : facultatif, repris quand le constat est hors catalogue ;
//! * valeur : nombre d'objets concernés ; 0 = contrôle réussi.
//!
//! # 2. Par la liste des constats
//!
//! [`checks_from_findings`] convertit directement la réponse de
//! `GET /api/targets/{id}/ad/findings` (identifiant, sévérité, titre, compte,
//! exemples d'objets) — les exemples passent alors dans la preuve.

use serde::Deserialize;

use super::super::facts::list;
use super::super::{Category, Check, CheckDef, Facts, Outcome, SecurityProvider, Severity};

pub const METRIC: &str = "ad_finding_count";

const fn finding(
    id: &'static str,
    title: &'static str,
    category: Category,
    severity: Severity,
    reference: &'static str,
    remediation: &'static str,
) -> CheckDef {
    CheckDef { id, title, category, severity, reference, remediation }
}

const PINGCASTLE_RULES: &str = "https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html";

/// Constats connus : leur identifiant, sans le préfixe `ad.`, est celui du
/// label `finding`.
pub const CATALOGUE: &[CheckDef] = &[
    finding(
        "ad.kerberoastable_users",
        "No Kerberoastable user accounts",
        Category::Authentication,
        Severity::High,
        PINGCASTLE_RULES,
        "Remove service principal names from user accounts, or move the services to group \
         managed service accounts (gMSA) with random 120-character passwords.",
    ),
    finding(
        "ad.asrep_roastable_users",
        "Kerberos pre-authentication required for every account",
        Category::Authentication,
        Severity::High,
        PINGCASTLE_RULES,
        "Clear \"Do not require Kerberos preauthentication\" on the listed accounts.",
    ),
    finding(
        "ad.unconstrained_delegation",
        "No unconstrained Kerberos delegation",
        Category::Authentication,
        Severity::Critical,
        "https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos",
        "Replace unconstrained delegation with constrained or resource-based constrained \
         delegation on the listed computers and accounts (domain controllers excepted).",
    ),
    finding(
        "ad.krbtgt_password_age",
        "krbtgt password changed in the last 180 days",
        Category::Authentication,
        Severity::High,
        "https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password",
        "Reset the krbtgt password twice, waiting for replication (and the ticket lifetime) \
         between the two resets.",
    ),
    finding(
        "ad.privileged_group_size",
        "Privileged groups kept small",
        Category::Authentication,
        Severity::Medium,
        "https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/plan/security-best-practices/implementing-least-privilege-administrative-models",
        "Remove day-to-day accounts from Domain Admins, Enterprise Admins and Administrators; \
         use dedicated admin accounts and delegation instead.",
    ),
    finding(
        "ad.laps_coverage",
        "Local administrator passwords managed by LAPS",
        Category::Authentication,
        Severity::High,
        "https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview",
        "Deploy Windows LAPS to every workstation and member server so no two machines share a \
         local administrator password.",
    ),
    finding(
        "ad.password_policy",
        "Domain password policy meets the baseline",
        Category::Authentication,
        Severity::Medium,
        "https://learn.microsoft.com/en-us/microsoft-365/admin/misc/password-policy-recommendations",
        "Require at least 14 characters, enable lockout, and use fine-grained password policies \
         for privileged accounts.",
    ),
    finding(
        "ad.stale_accounts",
        "No stale enabled accounts",
        Category::Authentication,
        Severity::Medium,
        PINGCASTLE_RULES,
        "Disable, then delete, user and computer accounts that have not logged on for months.",
    ),
];

pub const DEFS: &[&CheckDef] = &[
    &CATALOGUE[0],
    &CATALOGUE[1],
    &CATALOGUE[2],
    &CATALOGUE[3],
    &CATALOGUE[4],
    &CATALOGUE[5],
    &CATALOGUE[6],
    &CATALOGUE[7],
];

/// Un constat tel que le sert `GET /api/targets/{id}/ad/findings`.
#[derive(Debug, Clone, Deserialize)]
pub struct AdFinding {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub count: f64,
    #[serde(default, alias = "sample_objects")]
    pub samples: Vec<String>,
}

pub struct ActiveDirectory;

impl SecurityProvider for ActiveDirectory {
    fn kinds(&self) -> &'static [&'static str] {
        &["activedirectory"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[METRIC]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        let findings: Vec<AdFinding> = facts
            .all(METRIC)
            .filter(|f| !f.label("finding").is_empty())
            .map(|f| AdFinding {
                id: f.label("finding").to_string(),
                severity: f.label("severity").to_string(),
                title: f.label("title").to_string(),
                count: f.value,
                samples: Vec::new(),
            })
            .collect();
        checks_from_findings(&findings)
    }
}

fn parse_severity(raw: &str) -> Option<Severity> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "critical" => Some(Severity::Critical),
        "high" => Some(Severity::High),
        "medium" | "warning" => Some(Severity::Medium),
        "low" | "info" | "advisory" => Some(Severity::Low),
        _ => None,
    }
}

/// Convertit des constats en contrôles. Les constats du catalogue gardent
/// leur titre, leur référence et leur remédiation ; les autres prennent le
/// titre et la sévérité fournis. Un constat du catalogue absent des données
/// reste listé, `unknown`, pour montrer ce qui n'est pas encore examiné.
pub fn checks_from_findings(findings: &[AdFinding]) -> Vec<Check> {
    let mut checks: Vec<Check> = findings.iter().map(finding_check).collect();
    for def in CATALOGUE {
        if !findings.iter().any(|f| catalogue_id(def) == f.id) {
            checks.push(def.unknown("Not reported by the Active Directory collector yet."));
        }
    }
    checks
}

/// `ad.kerberoastable_users` → `kerberoastable_users`, l'identifiant du constat.
fn catalogue_id(def: &CheckDef) -> &str {
    def.id.strip_prefix("ad.").unwrap_or(def.id)
}

fn finding_check(finding: &AdFinding) -> Check {
    let evidence = if finding.count <= 0.0 {
        "None found".to_string()
    } else {
        let count = super::plural(finding.count, "object");
        if finding.samples.is_empty() {
            count
        } else {
            format!("{count}: {}", list(&finding.samples))
        }
    };
    let ok = finding.count <= 0.0;
    if let Some(def) = CATALOGUE.iter().find(|def| catalogue_id(def) == finding.id) {
        let mut check = def.verdict(ok, evidence);
        // La sévérité observée prime : le collecteur sait mieux que le
        // catalogue quand un constat est aggravé (compte privilégié…).
        if let Some(severity) = parse_severity(&finding.severity) {
            check.severity = severity;
            check.weight = severity.weight();
        }
        return check;
    }
    let severity = parse_severity(&finding.severity).unwrap_or(Severity::Medium);
    Check {
        id: format!("ad.{}", finding.id),
        title: if finding.title.is_empty() { finding.id.clone() } else { finding.title.clone() },
        category: Category::Authentication,
        severity,
        weight: severity.weight(),
        result: if ok { Outcome::Pass } else { Outcome::Fail },
        evidence,
        remediation: "See the Active Directory findings on this device for the affected objects."
            .to_string(),
        reference: PINGCASTLE_RULES.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_metriques_deviennent_des_controles() {
        let facts = Facts::from_pairs(&[
            (METRIC, &[("finding", "kerberoastable_users"), ("severity", "high")], 3.0),
            (METRIC, &[("finding", "unconstrained_delegation"), ("severity", "critical")], 0.0),
            (
                METRIC,
                &[
                    ("finding", "dns_admins_members"),
                    ("severity", "low"),
                    ("title", "DnsAdmins empty"),
                ],
                2.0,
            ),
        ]);
        let checks = ActiveDirectory.evaluate("activedirectory", &facts);
        let kerb = checks.iter().find(|c| c.id == "ad.kerberoastable_users").unwrap();
        assert_eq!((kerb.result, kerb.evidence.as_str()), (Outcome::Fail, "3 objects"));
        assert!(kerb.reference.starts_with("https://"));
        let deleg = checks.iter().find(|c| c.id == "ad.unconstrained_delegation").unwrap();
        assert_eq!(deleg.result, Outcome::Pass);
        let other = checks.iter().find(|c| c.id == "ad.dns_admins_members").unwrap();
        assert_eq!((other.title.as_str(), other.weight), ("DnsAdmins empty", 1));
        // Le reste du catalogue est montré comme non examiné.
        assert_eq!(
            checks.iter().filter(|c| c.result == Outcome::Unknown).count(),
            CATALOGUE.len() - 2
        );
    }

    #[test]
    fn la_liste_des_constats_porte_les_exemples() {
        let findings: Vec<AdFinding> = serde_json::from_value(serde_json::json!([
            { "id": "asrep_roastable_users", "severity": "info", "title": "x", "count": 2,
              "sample_objects": ["svc-backup", "olduser"] }
        ]))
        .unwrap();
        let checks = checks_from_findings(&findings);
        let check = &checks[0];
        assert_eq!(check.evidence, "2 objects: svc-backup, olduser");
        assert_eq!(check.severity, Severity::Low, "la sévérité du collecteur prime");
        assert_eq!(check.title, "Kerberos pre-authentication required for every account");
    }

    #[test]
    fn un_annuaire_sans_donnee_n_est_pas_note() {
        let report = crate::security::build_report(9, "dc1", "activedirectory", &Facts::default());
        assert!(report.supported);
        assert!(report.score.is_none());
        assert_eq!(report.counts.unknown, CATALOGUE.len());
    }
}
