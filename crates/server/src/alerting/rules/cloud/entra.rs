//! Règles livrées de entra (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_entra_credential_expiry_seconds",
    "dumbmonit_entra_ad_sync_age_seconds",
    "dumbmonit_entra_admin_without_mfa",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "A secret or certificate of an Entra application expires in less than 30 days: the application stops signing in on that day.".to_string(),
            operator: Operator::Lt,
            threshold: 2592000.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("entra_credential_expiring", "Entra application secret expiring", RuleKind::Threshold, "dumbmonit_entra_credential_expiry_seconds > 0")
        },
        Rule {
            description: "A secret or certificate of an Entra application expired in the last 30 days: remove it, or renew it if the application still needs it.".to_string(),
            operator: Operator::Lt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("entra_credential_expired", "Entra application secret expired", RuleKind::Threshold, "dumbmonit_entra_credential_expiry_seconds")
        },
        Rule {
            description: "The last synchronization with the on-premises Active Directory is more than 3 hours old: check Entra Connect.".to_string(),
            operator: Operator::Gt,
            threshold: 10800.0,
            for_duration: Duration::from_secs(600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("entra_ad_sync_late", "Entra directory sync late", RuleKind::Threshold, "dumbmonit_entra_ad_sync_age_seconds")
        },
        Rule {
            description: "At least one administrator has no multifactor authentication method registered.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("entra_admin_without_mfa", "Entra administrator without MFA", RuleKind::Threshold, "dumbmonit_entra_admin_without_mfa")
        },
    ]
}
