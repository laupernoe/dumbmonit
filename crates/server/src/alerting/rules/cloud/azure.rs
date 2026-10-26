//! Règles livrées de azure (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_azure_resources_unavailable",
    "dumbmonit_azure_resources_degraded",
    "dumbmonit_azure_alerts_high_active",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "Resource Health reports a resource of the subscription as unavailable for 10 minutes (not counting resources stopped by their owner).".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(600),
            severity: Severity::Critical,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("azure_resource_unavailable", "Azure resource unavailable", RuleKind::Threshold, "dumbmonit_azure_resources_unavailable")
        },
        Rule {
            description: "Resource Health reports a resource of the subscription as degraded for 15 minutes.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(900),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("azure_resource_degraded", "Azure resource degraded", RuleKind::Threshold, "dumbmonit_azure_resources_degraded")
        },
        Rule {
            description: "An Azure Monitor alert of severity Sev0 (critical) or Sev1 (error) is fired and not closed.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(300),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("azure_alert_severe", "Azure Monitor alert Sev0 or Sev1", RuleKind::Threshold, "dumbmonit_azure_alerts_high_active")
        },
    ]
}
