//! Règles livrées de hetzner (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_hetzner_server_running",
    "dumbmonit_hetzner_server_cpu_percent",
    "dumbmonit_hetzner_server_backup_age_seconds",
    "dumbmonit_hetzner_volume_attached",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "A Hetzner Cloud server is not in the running state (off, migrating, stuck starting).".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(600),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("hetzner_server_not_running", "Hetzner server not running", RuleKind::Threshold, "dumbmonit_hetzner_server_running")
        },
        Rule {
            description: "A Hetzner server averages more than 90% CPU over ten minutes, for 30 minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(1800),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("hetzner_cpu_high", "Hetzner server CPU sustained high", RuleKind::Threshold, "dumbmonit_hetzner_server_cpu_percent")
        },
        Rule {
            description: "The newest backup of a server with backups enabled is more than 3 days old.".to_string(),
            operator: Operator::Gt,
            threshold: 259200.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("hetzner_backup_old", "Hetzner backup too old", RuleKind::Threshold, "dumbmonit_hetzner_server_backup_age_seconds")
        },
        Rule {
            description: "A volume is attached to no server, and is billed anyway.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(604800)),
            ..base("hetzner_volume_unattached", "Hetzner volume not attached", RuleKind::Threshold, "dumbmonit_hetzner_volume_attached")
        },
    ]
}
