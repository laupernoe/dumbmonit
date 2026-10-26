//! Règles livrées de aws (générées à partir de la spécification du produit).

use std::time::Duration;

use super::super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_aws_alarm_in_alarm",
    "dumbmonit_aws_ec2_status_check_failed",
    "dumbmonit_aws_ec2_cpu_percent",
    "dumbmonit_aws_rds_cpu_percent",
    "dumbmonit_aws_rds_free_storage_bytes",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "A CloudWatch alarm has been in the ALARM state for 5 minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(300),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base(
                "aws_alarm_in_alarm",
                "AWS CloudWatch alarm in ALARM",
                RuleKind::Threshold,
                "dumbmonit_aws_alarm_in_alarm",
            )
        },
        Rule {
            description:
                "An EC2 instance has failed an instance or system status check for 10 minutes."
                    .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(600),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base(
                "aws_ec2_status_check_failed",
                "AWS EC2 status check failed",
                RuleKind::Threshold,
                "dumbmonit_aws_ec2_status_check_failed",
            )
        },
        Rule {
            description: "An EC2 instance has had a CPU peak above 90 % for 30 minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(1800),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base(
                "aws_ec2_cpu_high",
                "AWS EC2 CPU high",
                RuleKind::Threshold,
                "dumbmonit_aws_ec2_cpu_percent",
            )
        },
        Rule {
            description: "An RDS database has had a CPU peak above 90 % for 30 minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(1800),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base(
                "aws_rds_cpu_high",
                "AWS RDS CPU high",
                RuleKind::Threshold,
                "dumbmonit_aws_rds_cpu_percent",
            )
        },
        Rule {
            description: "An RDS database has less than 5 GiB of free storage for 15 minutes."
                .to_string(),
            operator: Operator::Lt,
            threshold: 5368709120.0,
            for_duration: Duration::from_secs(900),
            severity: Severity::Warning,
            unit: "".to_string(),
            repeat_interval: Some(Duration::from_secs(21600)),
            ..base(
                "aws_rds_storage_low",
                "AWS RDS free storage low",
                RuleKind::Threshold,
                "dumbmonit_aws_rds_free_storage_bytes",
            )
        },
    ]
}
