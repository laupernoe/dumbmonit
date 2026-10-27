# AWS (CloudWatch)

The state of an AWS account through CloudWatch: alarms that are firing, EC2
instances failing their status checks or running hot, and RDS databases with
a high CPU or little free storage.

DumbMonit calls the CloudWatch API of one region (`monitoring.<region>.amazonaws.com`)
with a read-only IAM access key, once a minute: `DescribeAlarms` for the
alarms in the `ALARM` state, `ListMetrics` for the EC2 instances and RDS
databases that publish metrics, and one `GetMetricData` request for their
last 10 minutes. Requests are signed with AWS Signature Version 4 by DumbMonit
itself; no AWS software is needed. Optionally, DumbMonit assumes an IAM role
through STS (`sts:AssumeRole`) and signs with its temporary credentials, kept
for an hour.

This is an Internet API: the address of the device is the CloudWatch endpoint
of the region, not a machine. One monitor covers one region of one account;
add a monitor per region.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the AWS documentation (CloudWatch API
    reference, Signature Version 4, STS), and the signature is checked
    against the official AWS test vector, not against a live account. It only
    reads. Tell us what breaks.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | CloudWatch API (HTTPS, monitoring.&lt;region&gt;.amazonaws.com) |
| 443 | TCP | STS AssumeRole, only with a role ARN (HTTPS, sts.&lt;region&gt;.amazonaws.com) |

## What it watches

All metrics are prefixed `dumbmonit_aws_`.

| Metric | Labels | Meaning |
|---|---|---|
| `alarms_in_alarm` | | Number of CloudWatch alarms (metric and composite) in the `ALARM` state. |
| `alarm_in_alarm` | `alarm` | 1 for each alarm in the `ALARM` state. |
| `ec2_instances` | | EC2 instances that published CPU in the last 3 hours. |
| `ec2_cpu_percent` | `instance` | Highest `CPUUtilization` over the last 10 minutes. |
| `ec2_status_check_failed` | `instance` | 1 when `StatusCheckFailed` (instance or system) was above 0 in the last 10 minutes. |
| `rds_instances` | | RDS database instances that published CPU in the last 3 hours. |
| `rds_cpu_percent` | `db` | Highest `CPUUtilization` over the last 10 minutes. |
| `rds_free_storage_bytes` | `db` | Latest `FreeStorageSpace`. |
| `rds_connections` | `db` | Latest `DatabaseConnections`. |
| `ec2_readable`, `rds_readable` | | 0 when the IAM permissions do not allow listing or reading the metrics of that service (the monitor keeps working with the alarms). |

At most 100 EC2 instances and 100 RDS databases are read (one `GetMetricData`
request is limited to 500 queries); the counts stay exact. An instance with
no datapoint in the last 10 minutes (stopped, or just launched) has no series.
Aggregated metrics (by Auto Scaling group, image or instance type) are
ignored, and so are Aurora clusters: only database instances are read.

The [built-in rules](../alerting/rules.md#aws-cloudwatch) that apply:

- **AWS CloudWatch alarm in ALARM**: an alarm has been firing for 5 minutes
  (Advisory, reminded every 6 hours).
- **AWS EC2 status check failed**: an instance or system status check has
  failed for 10 minutes (Advisory).
- **AWS EC2 CPU high**: a CPU peak above 90 % for 30 minutes (Advisory).
- **AWS RDS CPU high**: a CPU peak above 90 % for 30 minutes (Advisory).
- **AWS RDS free storage low**: under 5 GiB free for 15 minutes (Advisory).

## The monitor page

The page says in a sentence what is wrong, shows the alarms in the ALARM
state, the number of EC2 instances and RDS databases and the failed status
checks, and lists the alarms firing and the instances with their status check.

## Create a read-only IAM user for DumbMonit

1. In the AWS console, open IAM, Policies, Create policy, JSON tab, paste this read-only policy and name it DumbMonitReadOnly.

    `{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":["cloudwatch:DescribeAlarms","cloudwatch:ListMetrics","cloudwatch:GetMetricData"],"Resource":"*"}]}`

2. Open IAM, Users, Create user: name it dumbmonit, leave console access off, and attach the DumbMonitReadOnly policy.

3. Open the user, Security credentials, Create access key, choose Application running outside AWS. Copy the access key ID and the secret access key: the secret is shown once.

4. Optional, to use a role instead: attach the policy to a role that trusts the user, give the user only sts:AssumeRole on that role, and copy the role ARN into the Role ARN option.

5. In DumbMonit, enter monitoring.&lt;region&gt;.amazonaws.com as the address, paste the key and its secret, and set the same region in the Region option.

### Required permissions

- **cloudwatch:DescribeAlarms** — Lists the alarms in the ALARM state; changes nothing.
- **cloudwatch:ListMetrics** — Finds the EC2 instances and RDS databases that publish metrics.
- **cloudwatch:GetMetricData** — Reads CPU, status checks, free storage and connections over the last 10 minutes.
- **sts:AssumeRole (only with a role ARN)** — Lets the user assume the read-only role; allow it on that role only.

!!! warning
    Use a dedicated IAM user with only these read permissions, never a key that can change resources. AWS bills GetMetricData per metric requested (USD 0.01 per 1,000): a long check interval keeps the cost low on large accounts.

Vendor documentation: https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/Welcome.html

## Credentials

| Credential | Fields |
|---|---|
| Access key | Access key ID (`AKIA…`) and secret access key of the IAM user. |

Address: `monitoring.<region>.amazonaws.com`, for example
`monitoring.eu-west-3.amazonaws.com`. Options:

- **Region** (required): the region of the address, for example `eu-west-3`.
  It is part of the request signature: a mismatch is refused by AWS.
- **Role ARN**: a role to assume through STS (`arn:aws:iam::123456789012:role/dumbmonit-read`).
  Empty: the user's own permissions are used.
- **STS endpoint URL**: only with a role; replaces
  `https://sts.<region>.amazonaws.com` (VPC endpoint, China or GovCloud regions).
- **Timeout per request** (15 s).

## Troubleshooting

"AWS refused the request (UnrecognizedClientException / InvalidClientTokenId)":
the access key ID is wrong, or the key was deactivated or deleted.

"AWS refused the request (SignatureDoesNotMatch)": the secret access key is
wrong, or the Region option does not match the address.

"AWS refused the request (AccessDeniedException)": the policy lacks
`cloudwatch:DescribeAlarms`. Missing `ListMetrics` or `GetMetricData` does not
fail the monitor: the EC2 and RDS series are simply absent and `ec2_readable`
/ `rds_readable` are 0.

"AWS rejected the request date": the clock of the DumbMonit server is more than
a few minutes off; synchronise it.

"AWS refused to assume the role": the user is not allowed `sts:AssumeRole` on
the role, or the trust policy of the role does not name the user.
