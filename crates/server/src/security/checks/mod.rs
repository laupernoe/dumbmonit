//! Les contrôles, un module par famille d'équipements.
//!
//! Chaque module expose ses définitions (`DEFS`) et un fournisseur. Les
//! contrôles ne s'appuient que sur des métriques réellement produites par les
//! collecteurs ; ce qui manquerait (2FA Synology, conteneurs privilégiés…)
//! est listé dans `docs/using/security-score.md`, pas simulé ici.

pub mod activedirectory;
pub mod agent;
pub mod firewall;
pub mod pbs;
pub mod proxmox;
pub mod synology;
pub mod tls;
pub mod unifi;
pub mod updates;

use super::facts::{list, plural};
use super::{Check, CheckDef, Facts};

/// Toutes les définitions statiques (les constats Active Directory, ouverts,
/// n'en font partie que pour ceux du catalogue).
pub fn all_defs() -> Vec<&'static CheckDef> {
    [
        proxmox::DEFS,
        pbs::DEFS,
        synology::DEFS,
        firewall::DEFS,
        unifi::DEFS,
        tls::DEFS,
        agent::DEFS,
        updates::DEFS,
        activedirectory::DEFS,
    ]
    .concat()
}

/// Contrôle « ce compteur vaut zéro partout » : une série par instance
/// (nœud, équipement, conteneur…), nommée par l'étiquette `label`.
///
/// Sans aucune série, le contrôle est inconnu avec le motif `missing`.
pub(crate) fn zero_everywhere(
    def: &CheckDef,
    facts: &Facts,
    metric: &str,
    label: &str,
    noun: &str,
    missing: &str,
) -> Check {
    if !facts.has(metric) {
        return def.unknown(missing);
    }
    let offenders: Vec<String> = facts
        .all(metric)
        .filter(|f| f.value > 0.0)
        .map(|f| {
            let who = f.label(label);
            if noun.is_empty() {
                if who.is_empty() { "Detected".to_string() } else { who.to_string() }
            } else if who.is_empty() {
                plural(f.value, noun)
            } else {
                format!("{who}: {}", plural(f.value, noun))
            }
        })
        .collect();
    if offenders.is_empty() {
        let instances = facts.all(metric).count();
        if instances > 1 && !label.is_empty() {
            def.pass(format!("None on any of {instances} {label}s"))
        } else {
            def.pass("None")
        }
    } else {
        def.fail(list(&offenders))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{Category, Outcome, Severity};

    const DEF: CheckDef = CheckDef {
        id: "t",
        title: "t",
        category: Category::Patching,
        severity: Severity::High,
        reference: "https://example.org",
        remediation: "r",
    };

    #[test]
    fn zero_partout_nomme_les_coupables() {
        let facts =
            Facts::from_pairs(&[("u", &[("node", "pve1")], 0.0), ("u", &[("node", "pve2")], 3.0)]);
        let check = zero_everywhere(&DEF, &facts, "u", "node", "update", "absent");
        assert_eq!(check.result, Outcome::Fail);
        assert_eq!(check.evidence, "pve2: 3 updates");

        let ok = Facts::from_pairs(&[("u", &[("node", "a")], 0.0), ("u", &[("node", "b")], 0.0)]);
        let check = zero_everywhere(&DEF, &ok, "u", "node", "update", "absent");
        assert_eq!(check.result, Outcome::Pass);
        assert_eq!(check.evidence, "None on any of 2 nodes");

        let check = zero_everywhere(&DEF, &Facts::default(), "u", "node", "update", "absent");
        assert_eq!(check.result, Outcome::Unknown);
        assert_eq!(check.evidence, "absent");
    }

    #[test]
    fn sans_nom_l_echec_ne_donne_que_l_instance() {
        let facts = Facts::from_pairs(&[("u", &[("device", "AP")], 1.0)]);
        let check = zero_everywhere(&DEF, &facts, "u", "device", "", "absent");
        assert_eq!(check.evidence, "AP");
    }
}
