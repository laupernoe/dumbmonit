//! Sonde NTP : horloge synchronisée, et écart contenu.
//!
//! Les deux contrôles lisent ce que la sonde a déjà mesuré
//! (`crates/collectors/src/uptime/ntp`) ; aucune nouvelle requête ne part
//! d'ici. Le seuil d'écart est fixé à 100 ms, indépendamment du réglage
//! `offset_threshold_ms` propre à chaque cible — comme la fenêtre de
//! quatorze jours de [`super::tls::EXPIRY`], c'est une référence de sécurité,
//! pas l'alerte que l'utilisateur a réglée pour sa cible.

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};

/// Écart, en secondes, au-delà duquel le contrôle échoue.
const OFFSET_THRESHOLD_S: f64 = 0.1;

pub const SYNCHRONIZED: CheckDef = CheckDef {
    id: "ntp.synchronized",
    title: "Server reports a synchronized clock",
    category: Category::Configuration,
    severity: Severity::Medium,
    reference: "https://www.rfc-editor.org/rfc/rfc5905",
    remediation: "Fix the server's own time source: point it at a reachable upstream, or give it \
                  time to finish its own synchronization. A stratum 16 (or a kiss-o'-death) \
                  server has nothing to offer the clients that query it.",
};

pub const OFFSET: CheckDef = CheckDef {
    id: "ntp.offset",
    title: "Clock offset within 100 ms",
    category: Category::Configuration,
    severity: Severity::Low,
    reference: "https://www.rfc-editor.org/rfc/rfc5905",
    remediation: "Check the server's own upstream sources and polling interval: a persistent \
                  drift usually means it lost its reference and is coasting on its local clock.",
};

pub const DEFS: &[&CheckDef] = &[&SYNCHRONIZED, &OFFSET];

pub struct Ntp;

impl SecurityProvider for Ntp {
    fn kinds(&self) -> &'static [&'static str] {
        &["ntp"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["probe_ntp_stratum", "probe_ntp_leap_indicator", "probe_ntp_offset_seconds"]
    }

    fn applies(&self, facts: &Facts) -> bool {
        !facts.is_empty()
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        let stratum = facts.max("probe_ntp_stratum");
        let leap = facts.max("probe_ntp_leap_indicator");
        let synchronized = match (stratum, leap) {
            (None, None) => SYNCHRONIZED.unknown("No reply observed."),
            _ => {
                let unsynchronized = stratum.is_some_and(|s| s <= 0.0 || s >= 16.0)
                    || leap.is_some_and(|l| l >= 3.0);
                let evidence = match stratum {
                    Some(value) => format!("Stratum {}", value as i64),
                    None => "Leap indicator unsynchronized, no stratum observed".to_string(),
                };
                SYNCHRONIZED.verdict(!unsynchronized, evidence)
            }
        };
        let offset = match facts.max("probe_ntp_offset_seconds") {
            None => OFFSET.unknown("No offset observed."),
            Some(value) => OFFSET.verdict(
                value.abs() <= OFFSET_THRESHOLD_S,
                format!("Offset {:.1} ms", value * 1_000.0),
            ),
        };
        vec![synchronized, offset]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn un_serveur_synchronise_dans_le_seuil_reussit_les_deux_controles() {
        let facts = Facts::from_pairs(&[
            ("probe_ntp_stratum", &[], 2.0),
            ("probe_ntp_leap_indicator", &[], 0.0),
            ("probe_ntp_offset_seconds", &[], 0.005),
        ]);
        let checks = Ntp.evaluate("ntp", &facts);
        assert!(checks.iter().all(|c| c.result == Outcome::Pass), "{checks:?}");
    }

    #[test]
    fn un_stratum_seize_echoue_la_synchronisation() {
        let facts = Facts::from_pairs(&[
            ("probe_ntp_stratum", &[], 16.0),
            ("probe_ntp_leap_indicator", &[], 0.0),
            ("probe_ntp_offset_seconds", &[], 0.0),
        ]);
        let checks = Ntp.evaluate("ntp", &facts);
        assert_eq!(checks[0].result, Outcome::Fail);
        assert!(checks[0].evidence.contains("16"));
    }

    #[test]
    fn un_indicateur_de_correction_desynchronise_echoue_meme_a_stratum_bas() {
        let facts = Facts::from_pairs(&[
            ("probe_ntp_stratum", &[], 2.0),
            ("probe_ntp_leap_indicator", &[], 3.0),
            ("probe_ntp_offset_seconds", &[], 0.0),
        ]);
        assert_eq!(Ntp.evaluate("ntp", &facts)[0].result, Outcome::Fail);
    }

    #[test]
    fn un_ecart_au_dela_de_cent_millisecondes_echoue_sans_affecter_la_synchronisation() {
        let facts = Facts::from_pairs(&[
            ("probe_ntp_stratum", &[], 2.0),
            ("probe_ntp_leap_indicator", &[], 0.0),
            ("probe_ntp_offset_seconds", &[], 0.5),
        ]);
        let checks = Ntp.evaluate("ntp", &facts);
        assert_eq!(checks[0].result, Outcome::Pass, "la synchronisation elle-même reste bonne");
        assert_eq!(checks[1].result, Outcome::Fail);
        assert!(checks[1].evidence.contains("500.0 ms"));
    }

    #[test]
    fn sans_aucune_mesure_les_deux_controles_sont_inconnus() {
        assert!(!Ntp.applies(&Facts::default()));
        let report = crate::security::build_report(1, "ntp.lan", "ntp", &Facts::default());
        assert!(!report.supported);
    }
}
