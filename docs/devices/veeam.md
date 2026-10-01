# Veeam Backup & Replication

A Veeam Backup & Replication server through its REST API: jobs whose last run
failed or ended with a warning, failed sessions of the last 24 hours,
repositories filling up, and the license.

DumbMonit logs in with a Windows account that holds only the Veeam Backup
Viewer role (OAuth2 password grant on `/api/oauth2/token`, port 9419) and
keeps the token for its fifteen-minute lifetime. Every call carries the
mandatory `x-api-version` header, `1.1-rev0` by default: the revision that
came with version 12.0.

The job states (`/api/v1/jobs/states`) are the one required read. The others —
`/api/v1/serverInfo`, `/api/v1/sessions` (created in the last 24 hours),
`/api/v1/backupInfrastructure/repositories/states` and `/api/v1/license` — are
read when the role allows them. Veeam keeps some routes for the Backup
Administrator role, the license among them on current versions: a refusal
there is shown as "not readable", never as an error.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from Veeam's REST API reference and help
    center (authentication, versioning, the job, session, repository and
    license models), not against a running backup server. Which routes the
    Viewer role may read varies with the version: tell us what your server
    refuses.

## What it watches

All metrics are prefixed `dumbmonit_veeam_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version` | Value 1: the build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Per job. |
| `job_failed`, `job_warning` | `job`, `type` | 1 when the last run of an enabled job failed, or ended with a warning. A disabled job keeps its last result, which no longer says anything: it counts as neither. |
| `job_result_info` | `job`, `type`, `result` | Value 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Since the last run; absent for a job that never ran. |
| `job_progress_percent` | `job`, `type` | While the job runs. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Counts; at most 200 jobs are described one by one. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sessions created in the last 24 hours, by result. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Per repository. An object storage repository has no capacity: only its used space is published. |
| `license_valid` | | 1 when the license status is `Valid`. |
| `license_info` | `status`, `type`, `edition` | Value 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Until the license and the support contract expire; absent for a perpetual license. |
| `license_instances_licensed`, `license_instances_used` | | Instance license use. |
| `section_readable` | `section`: `sessions`, `repositories`, `license` | 1 when the account may read it, 0 when Veeam refused. |

The [built-in rules](../alerting/rules.md#veeam-backup-replication) that apply:

- **Veeam job failed**: the last run of an enabled job failed (Warning,
  reminded daily).
- **Veeam job ended with a warning** (Advisory, reminded daily).
- **Veeam repository nearly full**: more than 90 % for thirty minutes
  (Advisory).
- **Veeam license expiring**: less than 30 days left (Advisory, reminded
  daily). Only when the account can read the license.

## The device page

The Backups panel says what is wrong in a sentence, shows the failed jobs, the
failed sessions of the last 24 hours, the running jobs and the license, then
lists the enabled jobs — failures first — with their type and last run, and
the repositories with their fill and free space.

## Give DumbMonit a Veeam Backup Viewer account

1. On the backup server, create a local Windows account for DumbMonit with a long random password and no other right on the machine (a domain account works too).

    ```
    net user dumbmonit * /add
    ```

2. In the Veeam console, open the main menu > Users and Roles > Security, click Add, enter the account (for example VBR01\dumbmonit) and give it the Veeam Backup Viewer role only.

3. In DumbMonit, enter the address of the backup server, for example "vbr.lan", and the account as HOST\dumbmonit or DOMAIN\dumbmonit with its password. The REST API listens on port 9419 with a self-signed certificate: tick Accept an unverifiable certificate unless you installed your own.

!!! warning
    Veeam keeps some REST API routes for the Backup Administrator role, the license among them on current versions. With the Viewer role DumbMonit reads the jobs, sessions and repositories, and shows the license as not readable: it never asks for more.

## Credentials

| Credential | Fields |
|---|---|
| User name and password | The account with the Veeam Backup Viewer role, as `HOST\dumbmonit` or `DOMAIN\dumbmonit`. |

Address: the host name or IP of the backup server, `host:port`, or a URL.
Options:

- **Protocol** (`https`) and **Port** (9419): the REST API service's.
- **API version** (`1.1-rev0`): sent as `x-api-version`. Change it only if
  the server refuses it; a v13 server documents `1.3-rev1`.
- **Accept an unverifiable certificate**: the REST API's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"Veeam refused the login": the user name needs its host or domain
(`VBR01\dumbmonit`), or the account has no Veeam role.

"Veeam refused to list the jobs (403)": the account has a role that cannot
read jobs; give it Veeam Backup Viewer.

"Veeam refused the API version": set the API version option to one the server
lists on `https://<server>:9419/swagger`.

The license shows "Not readable": expected with the Viewer role on current
versions; DumbMonit does not ask for more.
