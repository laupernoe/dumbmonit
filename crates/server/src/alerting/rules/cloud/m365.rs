//! Règles livrées de m365 (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_m365_services_degraded",
    "dumbmonit_m365_license_used_percent",
    "dumbmonit_m365_risky_users",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "A Microsoft 365 service is degraded, interrupted or under investigation according to the service health dashboard.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(600),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("m365_service_degraded", "Microsoft 365 service degraded", RuleKind::Threshold, "dumbmonit_m365_services_degraded")
        },
        Rule {
            description: "Every unit of a paid license is assigned: the next user cannot be given it.".to_string(),
            operator: Operator::Gt,
            threshold: 99.99,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("m365_license_exhausted", "Microsoft 365 licenses exhausted", RuleKind::Threshold, "dumbmonit_m365_license_used_percent")
        },
        Rule {
            description: "More than 95 % of a paid license is assigned.".to_string(),
            operator: Operator::Gt,
            threshold: 95.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(604800)),
            ..base("m365_license_nearly_full", "Microsoft 365 licenses nearly full", RuleKind::Threshold, "dumbmonit_m365_license_used_percent < 99.99")
        },
        Rule {
            description: "Entra ID Protection flags at least one user as at risk.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(900),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("m365_risky_users", "Microsoft 365 users at risk", RuleKind::Threshold, "dumbmonit_m365_risky_users")
        },
    ]
}
