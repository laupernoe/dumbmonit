//! Règles livrées de profondeur pour OPNsense, en plus de celles de `rules.rs`
//! (passerelles, table d'états, tunnels, services, CARP, mises à jour).

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Métriques que ces règles visent : le test
/// `les_regles_livrees_ne_visent_que_des_metriques_produites` les accepte.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_opnsense_certificate_expiry_seconds",
    "dumbmonit_opnsense_disk_used_percent",
    "dumbmonit_opnsense_cpu_percent",
    "dumbmonit_opnsense_swap_used_percent",
    "dumbmonit_opnsense_pf_enabled",
    "dumbmonit_opnsense_interface_errors_in",
    "dumbmonit_opnsense_interface_errors_out",
    "dumbmonit_opnsense_firmware_connection_ok",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Un certificat d'interface d'administration, de portail captif ou de
        // VPN qui expire coupe l'accès sans prévenir. Les certificats marqués
        // « inutilisés » par OPNsense ne sont pas collectés.
        Rule {
            description: "A certificate used by the firewall expires in less than 14 days (web interface, captive portal, VPN, ...).".to_string(),
            operator: Operator::Lt,
            threshold: 14.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "opnsense_certificate_expiring",
                "OPNsense certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_opnsense_certificate_expiry_seconds",
            )
        },
        Rule {
            description: "A certificate used by the firewall has expired: clients now refuse it.".to_string(),
            operator: Operator::Lt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "opnsense_certificate_expired",
                "OPNsense certificate expired",
                RuleKind::Threshold,
                "dumbmonit_opnsense_certificate_expiry_seconds",
            )
        },
        Rule {
            description: "The firewall's disk is more than 90% full: logs, captures and updates will fail.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "opnsense_disk_almost_full",
                "OPNsense disk almost full",
                RuleKind::Threshold,
                "dumbmonit_opnsense_disk_used_percent",
            )
        },
        Rule {
            description: "The firewall's processor has been above 90% for fifteen minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "opnsense_cpu_high",
                "OPNsense CPU high",
                RuleKind::Threshold,
                "dumbmonit_opnsense_cpu_percent",
            )
        },
        // Un pare-feu qui swappe ne tient plus sa charge : débit effondré, puis
        // processus tués. Une heure pour ne pas réagir à un pic de mise à jour.
        Rule {
            description: "The firewall uses more than half of its swap for an hour: it is short of memory.".to_string(),
            operator: Operator::Gt,
            threshold: 50.0,
            clear_threshold: Some(40.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "opnsense_swap_high",
                "OPNsense swap in use",
                RuleKind::Threshold,
                "dumbmonit_opnsense_swap_used_percent",
            )
        },
        Rule {
            description: "The packet filter (pf) is disabled: the firewall no longer filters or translates anything.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "opnsense_pf_disabled",
                "OPNsense packet filter disabled",
                RuleKind::Threshold,
                "dumbmonit_opnsense_pf_enabled",
            )
        },
        // Cent erreurs en un quart d'heure : un câble, un SFP ou un duplex
        // mal négocié, pas le bruit de fond d'un lien sain.
        Rule {
            description: "An interface logged more than 100 receive or transmit errors in 15 minutes: check the cable, the SFP and the duplex.".to_string(),
            operator: Operator::Gt,
            threshold: 100.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "opnsense_interface_errors",
                "OPNsense interface errors",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_opnsense_interface_errors_in[15m]) + increase_prometheus(dumbmonit_opnsense_interface_errors_out[15m])",
            )
        },
        Rule {
            description: "The firewall cannot reach its update mirror: it will not learn about security updates.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(12 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "opnsense_firmware_mirror_unreachable",
                "OPNsense update mirror unreachable",
                RuleKind::Threshold,
                "dumbmonit_opnsense_firmware_connection_ok",
            )
        },
    ]
}
