//! Règles livrées de cloudflare (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_cloudflare_tunnel_down",
    "dumbmonit_cloudflare_tunnel_degraded",
    "dumbmonit_cloudflare_certificate_expiry_seconds",
    "dumbmonit_cloudflare_zone_active",
    "dumbmonit_cloudflare_zone_http_5xx_percent",
    "dumbmonit_cloudflare_lb_pool_healthy",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "A Cloudflare tunnel has no connection left and cannot carry traffic.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(300),
            severity: Severity::Critical,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("cloudflare_tunnel_down", "Cloudflare tunnel down", RuleKind::Threshold, "dumbmonit_cloudflare_tunnel_down")
        },
        Rule {
            description: "A Cloudflare tunnel has lost some of its connections to the edge.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(900),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("cloudflare_tunnel_degraded", "Cloudflare tunnel degraded", RuleKind::Threshold, "dumbmonit_cloudflare_tunnel_degraded")
        },
        Rule {
            description: "An active certificate of a zone expires in less than 14 days; Cloudflare should have renewed it by now.".to_string(),
            operator: Operator::Lt,
            threshold: 1209600.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("cloudflare_certificate_expiring", "Cloudflare certificate expiring", RuleKind::Threshold, "dumbmonit_cloudflare_certificate_expiry_seconds")
        },
        Rule {
            description: "A zone is not active (pending name server change, moved or deactivated).".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(86400)),
            ..base("cloudflare_zone_inactive", "Cloudflare zone not active", RuleKind::Threshold, "dumbmonit_cloudflare_zone_active")
        },
        Rule {
            description: "More than 5% of the responses of a zone are 5xx over the last one to two hours (zones with at least 100 requests).".to_string(),
            operator: Operator::Gt,
            threshold: 5.0,
            for_duration: Duration::from_secs(900),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("cloudflare_5xx_rate", "Cloudflare 5xx rate high", RuleKind::Threshold, "dumbmonit_cloudflare_zone_http_5xx_percent")
        },
        Rule {
            description: "An enabled load balancer pool is unhealthy.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(300),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base("cloudflare_pool_down", "Cloudflare load balancer pool unhealthy", RuleKind::Threshold, "dumbmonit_cloudflare_lb_pool_healthy")
        },
    ]
}
