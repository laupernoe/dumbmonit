//! Règles livrées de infomaniak (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_infomaniak_product_expiry_seconds",
    "dumbmonit_infomaniak_product_locked",
    "dumbmonit_infomaniak_product_maintenance",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "A product (domain, hosting, mail) expires in less than 14 days."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1209600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base(
                "infomaniak_product_expiring",
                "Infomaniak product expiring",
                RuleKind::Threshold,
                "dumbmonit_infomaniak_product_expiry_seconds",
            )
        },
        Rule {
            description: "The expiry date of a product has passed.".to_string(),
            operator: Operator::Lt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Critical,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base(
                "infomaniak_product_expired",
                "Infomaniak product expired",
                RuleKind::Threshold,
                "dumbmonit_infomaniak_product_expiry_seconds",
            )
        },
        Rule {
            description: "Infomaniak flags a product as locked for an hour.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base(
                "infomaniak_product_locked",
                "Infomaniak product locked",
                RuleKind::Threshold,
                "dumbmonit_infomaniak_product_locked",
            )
        },
        Rule {
            description: "Infomaniak flags a product as under maintenance for an hour.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base(
                "infomaniak_product_maintenance",
                "Infomaniak product in maintenance",
                RuleKind::Threshold,
                "dumbmonit_infomaniak_product_maintenance",
            )
        },
    ]
}
