//! Règles livrées de profondeur pour Tailscale, en plus de celles de `rules.rs`
//! (appareil hors ligne, clé qui expire, appareil à approuver, mise à jour).

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Métriques que ces règles visent : le test
/// `les_regles_livrees_ne_visent_que_des_metriques_produites` les accepte.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_tailscale_device_key_expiry_seconds",
    "dumbmonit_tailscale_device_watched",
    "dumbmonit_tailscale_device_online",
    "dumbmonit_tailscale_device_routes_advertised",
    "dumbmonit_tailscale_device_routes_unapproved",
    "dumbmonit_tailscale_device_tailnet_lock_error",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // La règle d'expiration à 14 jours existe déjà ; celle-ci dit que c'est
        // fait : l'appareil a quitté le tailnet et ne reviendra pas seul.
        Rule {
            description: "The node key of a monitored Tailscale device has expired: it left the tailnet and needs to sign in again.".to_string(),
            operator: Operator::Lt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "tailscale_key_expired",
                "Tailscale node key expired",
                RuleKind::Threshold,
                "dumbmonit_tailscale_device_key_expiry_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0",
            )
        },
        // Un routeur de sous-réseau ou un nœud de sortie hors ligne coupe tout
        // ce qu'il dessert, même si l'appareil n'est pas dans la liste surveillée.
        Rule {
            description: "A Tailscale device that advertises routes (subnet router or exit node) has been offline for 10 minutes: the networks behind it are unreachable.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "tailscale_router_offline",
                "Tailscale subnet router offline",
                RuleKind::Threshold,
                "dumbmonit_tailscale_device_online and on (target, device) dumbmonit_tailscale_device_routes_advertised > 0",
            )
        },
        // Les routes annoncées doivent être approuvées dans la console ; sinon
        // le routeur est en ligne et ne sert à rien.
        Rule {
            description: "A Tailscale device advertises routes that nobody approved in the admin console: the router is online but the routes do not work.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "tailscale_routes_unapproved",
                "Tailscale routes awaiting approval",
                RuleKind::Threshold,
                "dumbmonit_tailscale_device_routes_unapproved",
            )
        },
        Rule {
            description: "A Tailscale device reports a tailnet lock error: it is not signed by a trusted node and cannot talk to the tailnet.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "tailscale_tailnet_lock_error",
                "Tailscale tailnet lock error",
                RuleKind::Threshold,
                "dumbmonit_tailscale_device_tailnet_lock_error",
            )
        },
    ]
}
