//! Règles livrées de profondeur pour UniFi Network, en plus de celles de
//! `rules.rs` (équipement hors ligne, Internet, WAN, firmware, alarmes).

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Métriques que ces règles visent : le test
/// `les_regles_livrees_ne_visent_que_des_metriques_produites` les accepte.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_unifi_subsystem_status",
    "dumbmonit_unifi_device_overheating",
    "dumbmonit_unifi_device_poe_budget_watts",
    "dumbmonit_unifi_device_poe_used_watts",
    "dumbmonit_unifi_radio_channel_utilization_percent",
    "dumbmonit_unifi_wifi_clients",
    "dumbmonit_unifi_wifi_clients_weak_signal",
    "dumbmonit_unifi_port_rx_errors_total",
    "dumbmonit_unifi_port_tx_errors_total",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // 2 = erreur. `www` est déjà couvert par la règle « Internet down » et
        // `wan` par celle des liens WAN : ils ne sont pas répétés ici.
        Rule {
            description: "A UniFi subsystem (Wi-Fi, LAN, VPN) reports an error in the controller's health view.".to_string(),
            operator: Operator::Gt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "unifi_subsystem_error",
                "UniFi subsystem in error",
                RuleKind::Threshold,
                "dumbmonit_unifi_subsystem_status{subsystem=~\"wlan|lan|vpn\"} < 3",
            )
        },
        Rule {
            description: "A UniFi device reports that it is overheating.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "unifi_device_overheating",
                "UniFi device overheating",
                RuleKind::Threshold,
                "dumbmonit_unifi_device_overheating",
            )
        },
        // Un commutateur qui dépasse son budget coupe des ports au hasard : une
        // caméra ou un point d'accès disparaît sans raison apparente.
        Rule {
            description: "A PoE switch draws more than 90% of its PoE budget: the next device plugged in may not be powered.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "unifi_poe_budget_high",
                "UniFi PoE budget nearly used",
                RuleKind::Threshold,
                "100 * dumbmonit_unifi_device_poe_used_watts / dumbmonit_unifi_device_poe_budget_watts",
            )
        },
        // Un canal occupé à plus de 80 % pendant une demi-heure ralentit tout
        // le monde ; ponctuellement, c'est normal (voisinage, sauvegarde).
        Rule {
            description: "The channel of an access point radio is busy more than 80% of the time for 30 minutes: Wi-Fi is slow for everyone on it.".to_string(),
            operator: Operator::Gt,
            threshold: 80.0,
            clear_threshold: Some(65.0),
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "unifi_radio_channel_busy",
                "UniFi Wi-Fi channel saturated",
                RuleKind::Threshold,
                "dumbmonit_unifi_radio_channel_utilization_percent",
            )
        },
        // Au moins cinq clients : avec deux téléphones, un seul au fond du jardin
        // fait déjà 50 %.
        Rule {
            description: "More than 40% of the Wi-Fi clients have a signal weaker than -75 dBm (at least five clients): access points are badly placed or too few.".to_string(),
            operator: Operator::Gt,
            threshold: 40.0,
            clear_threshold: Some(30.0),
            for_duration: Duration::from_secs(60 * 60),
            severity: Severity::Info,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "unifi_wifi_weak_signal",
                "UniFi Wi-Fi clients with a weak signal",
                RuleKind::Threshold,
                "100 * dumbmonit_unifi_wifi_clients_weak_signal / dumbmonit_unifi_wifi_clients and dumbmonit_unifi_wifi_clients >= 5",
            )
        },
        // 100 erreurs en quinze minutes : un câble ou un connecteur qui lâche,
        // pas le bruit de fond d'un lien sain.
        Rule {
            description: "A switch port logged more than 100 receive or transmit errors in 15 minutes: check the cable and the connectors.".to_string(),
            operator: Operator::Gt,
            threshold: 100.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "unifi_port_errors",
                "UniFi port errors",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_unifi_port_rx_errors_total[15m]) + increase_prometheus(dumbmonit_unifi_port_tx_errors_total[15m])",
            )
        },
    ]
}
