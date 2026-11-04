//! Règles livrées de Jellyfin au-delà des trois premières (tâche en échec,
//! extension cassée, redémarrage en attente), qui restent dans `rules.rs`.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Métriques visées par ces règles et absentes de `PRODUITES`.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_jellyfin_library_scan_age_seconds",
    "dumbmonit_jellyfin_library_scan_scheduled",
    "dumbmonit_jellyfin_storage_used_percent",
    "dumbmonit_jellyfin_transcodes_software",
    "dumbmonit_jellyfin_failed_logins_recent",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Le scan n'a pas tourné depuis trois jours alors qu'il est planifié
        // (par défaut toutes les douze heures) : serveur à l'arrêt, tâche
        // coincée. Un scan sans déclencheur périodique est écarté.
        Rule {
            description: "The scheduled Jellyfin library scan has not finished for three days: \
                          new films and episodes are not picked up."
                .to_string(),
            operator: Operator::Gt,
            threshold: 3.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "jellyfin_library_scan_stale",
                "Jellyfin library scan stale",
                RuleKind::Threshold,
                "dumbmonit_jellyfin_library_scan_age_seconds \
                 and on (target) dumbmonit_jellyfin_library_scan_scheduled == 1",
            )
        },
        // Un volume par disque (bibliothèques, données, cache, transcodages).
        Rule {
            description: "A disk used by Jellyfin (a library, its data, cache or transcode \
                          folder) is more than 90% full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(88.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "jellyfin_storage_almost_full",
                "Jellyfin storage almost full",
                RuleKind::Threshold,
                "dumbmonit_jellyfin_storage_used_percent",
            )
        },
        // Un ou deux transcodages logiciels sont normaux ; trois qui durent
        // saturent un petit processeur et font saccader la lecture.
        Rule {
            description: "Three or more Jellyfin streams have been transcoded on the CPU, with \
                          no hardware acceleration, for 15 minutes: playback may stutter."
                .to_string(),
            operator: Operator::Ge,
            threshold: 3.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "jellyfin_cpu_transcoding_heavy",
                "Jellyfin heavy CPU transcoding",
                RuleKind::Threshold,
                "dumbmonit_jellyfin_transcodes_software",
            )
        },
        // Dix échecs en une heure dépassent la faute de frappe : un mot de passe
        // deviné, ou un client resté configuré avec l'ancien.
        Rule {
            description: "Ten or more failed Jellyfin sign-ins in the last hour: a password \
                          being guessed, or a client still using an old one."
                .to_string(),
            operator: Operator::Ge,
            threshold: 10.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "jellyfin_failed_logins",
                "Jellyfin failed sign-ins",
                RuleKind::Threshold,
                "dumbmonit_jellyfin_failed_logins_recent",
            )
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_regles_jellyfin_visent_des_metriques_declarees() {
        let rules = rules();
        let uids: Vec<&str> = rules.iter().map(|r| r.uid.as_str()).collect();
        assert_eq!(
            uids,
            [
                "jellyfin_library_scan_stale",
                "jellyfin_storage_almost_full",
                "jellyfin_cpu_transcoding_heavy",
                "jellyfin_failed_logins",
            ]
        );
        for rule in &rules {
            assert!(rule.builtin && rule.enabled);
            for mot in rule.query.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if mot.starts_with("dumbmonit_") {
                    assert!(PRODUCED.contains(&mot), "{}: {mot}", rule.uid);
                }
            }
        }
    }
}
