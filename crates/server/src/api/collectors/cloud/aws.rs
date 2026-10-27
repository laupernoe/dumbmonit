//! Notice de aws (générée à partir de la spécification du produit).

use super::super::*;

pub(super) fn view() -> CollectorView {
    CollectorView {
        kind: "aws",
        label: "AWS (CloudWatch)",
        summary: "CloudWatch alarms in the ALARM state, EC2 status checks and CPU, and RDS CPU and free storage.",
        examples: &["Amazon EC2", "Amazon RDS", "CloudWatch alarms"],
        credential_types: &["username_password"],
        credentials: &[CredentialView {
            kind: "username_password",
            label: "Access key",
            help: "Access key ID and secret access key of a read-only IAM user (or of a user allowed to assume a read-only role).",
            fields: &[
                CredentialField {
                    key: "username",
                    label: "Access key ID",
                    help: "Starts with AKIA for a long-term key.",
                    placeholder: "AKIAIOSFODNN7EXAMPLE",
                    input: "text",
                    choices: &[],
                    required: true,
                },
                CredentialField {
                    key: "password",
                    label: "Secret access key",
                    help: "Shown once when the access key is created.",
                    placeholder: "",
                    input: "password",
                    choices: &[],
                    required: true,
                },
            ],
        }],
        address_hint: "monitoring.eu-west-3.amazonaws.com",
        default_port: 443,
        setup: Setup {
            title: "Create a read-only IAM user for DumbMonit",
            steps: &[
                "In the AWS console, open IAM, Policies, Create policy, JSON tab, paste this read-only policy and name it DumbMonitReadOnly.\n{\"Version\":\"2012-10-17\",\"Statement\":[{\"Effect\":\"Allow\",\"Action\":[\"cloudwatch:DescribeAlarms\",\"cloudwatch:ListMetrics\",\"cloudwatch:GetMetricData\"],\"Resource\":\"*\"}]}",
                "Open IAM, Users, Create user: name it dumbmonit, leave console access off, and attach the DumbMonitReadOnly policy.",
                "Open the user, Security credentials, Create access key, choose Application running outside AWS. Copy the access key ID and the secret access key: the secret is shown once.",
                "Optional, to use a role instead: attach the policy to a role that trusts the user, give the user only sts:AssumeRole on that role, and copy the role ARN into the Role ARN option.",
                "In DumbMonit, enter monitoring.<region>.amazonaws.com as the address, paste the key and its secret, and set the same region in the Region option.",
            ],
            ports: &[
                to_device!(443, "tcp", "CloudWatch API (HTTPS, monitoring.<region>.amazonaws.com)"),
                to_device_optional!(
                    443,
                    "tcp",
                    "Only with a role ARN: STS AssumeRole (HTTPS, sts.<region>.amazonaws.com)"
                ),
            ],
            permissions: &[
                "cloudwatch:DescribeAlarms\nLists the alarms in the ALARM state; changes nothing.",
                "cloudwatch:ListMetrics\nFinds the EC2 instances and RDS databases that publish metrics.",
                "cloudwatch:GetMetricData\nReads CPU, status checks, free storage and connections over the last 10 minutes.",
                "sts:AssumeRole (only with a role ARN)\nLets the user assume the read-only role; allow it on that role only.",
            ],
            warning: "Use a dedicated IAM user with only these read permissions, never a key that can change resources. AWS bills GetMetricData per metric requested (USD 0.01 per 1,000): a long check interval keeps the cost low on large accounts.",
        },
        options: &[
            OptionView {
                key: "region",
                label: "Region",
                help: "AWS region of the account resources, also used to sign requests. It must match the region in the address.",
                placeholder: "eu-west-3",
                default: "",
                required: true,
                input: "text",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
            OptionView {
                key: "role_arn",
                label: "Role ARN",
                help: "Optional. Role to assume through STS, instead of reading with the user's own permissions.",
                placeholder: "arn:aws:iam::123456789012:role/dumbmonit-read",
                default: "",
                required: false,
                input: "text",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
            OptionView {
                key: "endpoint_url",
                label: "STS endpoint URL",
                help: "Optional, only with a role. Replaces https://sts.<region>.amazonaws.com (VPC endpoint, China or GovCloud regions).",
                placeholder: "https://sts.eu-west-3.amazonaws.com",
                default: "",
                required: false,
                input: "text",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
            OptionView {
                key: "request_timeout_seconds",
                label: "Timeout per request (seconds)",
                help: "Time allowed for each call, from 1 to 120.",
                placeholder: "15",
                default: "15",
                required: false,
                input: "number",
                choices: &[],
                group: "",
                items: no_items,
                columns: no_columns,
            },
        ],
    }
}
