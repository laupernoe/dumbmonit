//! Règles livrées de ovh (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] =
    &["dumbmonit_ovh_vps_up", "dumbmonit_ovh_dedicated_up", "dumbmonit_ovh_tasks_active"];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "An OVHcloud VPS is not in the running state for 10 minutes.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(600),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("ovh_vps_down", "OVHcloud VPS not running", RuleKind::Threshold, "dumbmonit_ovh_vps_up")
        },
        Rule {
            description: "OVHcloud monitoring reports a dedicated server in error, or powered off, for 5 minutes.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(300),
            severity: Severity::Critical,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("ovh_dedicated_down", "OVHcloud dedicated server in error", RuleKind::Threshold, "dumbmonit_ovh_dedicated_up")
        },
        Rule {
            description: "One or more tasks (reboot, reinstall, maintenance) have been pending or running on the servers for an hour.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("ovh_tasks_active", "OVHcloud task in progress", RuleKind::Threshold, "dumbmonit_ovh_tasks_active")
        },
    ]
}
