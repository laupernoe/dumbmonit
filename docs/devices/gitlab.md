# GitLab (self-managed)

Whether a self-managed GitLab instance is serving, and whether the parts that
quietly carry everything else are keeping up: Sidekiq's queues, the CI/CD
runners, database migrations left over from an upgrade, and — for GitLab
Enterprise Edition — the licence.

DumbMonit reads `/api/v4/version` with a **personal access token**, the only
call that must succeed: it is the proof that GitLab answers and that the
token is valid. Everything else — Sidekiq, runners, migrations, instance
statistics, the sign-up and two-factor settings, the licence, and the
readiness check in detail — is read from endpoints GitLab reserves to
administrator accounts, and is skipped without failing the probe when the
token belongs to an ordinary one.

The failure worth watching here: **Sidekiq falling behind**. GitLab keeps
serving pages while its background worker queue grows, and nothing on the
page says so until a notification, a webhook or a CI pipeline update is late
by minutes or hours.

## What it watches

All metrics are prefixed `dumbmonit_gitlab_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version`, `revision` |
| `readiness_check` | 1 when a readiness component (database, cache, queues, shared state, Gitaly) is not `ok` | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | jobs waiting and their age, per Sidekiq queue | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | CI/CD runners known to the instance | |
| `runner_offline` | value 1 per offline runner (at most 32) | `runner` |
| `migrations_pending` | database migrations left to apply | |
| `migration_pending` | value 1 per pending migration (at most 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | instance-wide counts | |
| `signup_enabled` | 1 while public sign-up is open | |
| `two_factor_required` | 1 while every user must set up two-factor authentication | |
| `license_info` | value 1 (Enterprise Edition only) | `plan` |
| `license_expired`, `license_expiry_seconds` | whether the licence has expired, and the time left | |
| `pipelines_failed_recent`, `pipelines_pending` | failed pipelines in the lookback window, and pipelines pending or running now — only for the projects listed in the "Projects watched for CI health" option | `project` |
| `scrape_errors` | administrator-only calls that failed on the last probe (0 with an ordinary token's ones never attempted) | |

The [built-in rules](../alerting/rules.md#self-hosted-forges) that apply:
GitLab runners all offline, GitLab Sidekiq backlog high, GitLab migrations
pending, GitLab license expiring, plus Device unreachable. A security score
checks the two-factor and sign-up settings, and pending migrations: see
[Vendor updates and other checks](../using/security-score.md).

## What it does not watch

GitLab's own REST API has no endpoint that says "a newer GitLab release is
available": unlike Nextcloud or Paperless-ngx, there is no self-reported
update flag to read, so DumbMonit does not claim one. Compare the version
above against GitLab's [release page](https://about.gitlab.com/releases/) by
hand, or watch GitLab's own update-check notice in the admin area.

CI pipeline health is read only for the projects you list: GitLab's API has
no instance-wide "all failing pipelines" endpoint, only a per-project one.

## The device page

The panel above the charts reads what the probe stored; opening the page
never calls GitLab. It says what is wrong first (a readiness component
failed, runners all offline, a large Sidekiq backlog), then the version, then
the instance counts and the settings read from the administration API.

## Create a personal access token on an administrator account

1. Create a personal access token on an administrator account: avatar → Edit profile → Access tokens → Add new token. Name it as follows, set an expiration date you are comfortable renewing, and tick the read_api scope only: it reads the whole API and writes nothing.

    ```
    dumbmonit
    ```

2. Sidekiq queues, CI runners, pending migrations, instance statistics, the sign-up and two-factor settings and the licence are each read from an administrator-only endpoint: GitLab has no finer delegation for them. A token from an ordinary account still works; only the version is then read, and the rest is skipped without failing the probe.

3. In DumbMonit, enter the address you open GitLab with, for example "gitlab.example.com" (HTTPS on 443) or "https://git.example.com" behind a reverse proxy, and paste the token.

4. To also watch CI pipelines, list the numeric project IDs (shown on each project's overview page, under its name) in the "Projects watched for CI health" option, comma-separated.

!!! warning
    A personal access token on an administrator account can read everything the account can read, even with the read_api scope: GitLab has no read-only administrator role. Dedicate a bot account to it rather than reusing your own, and revoke the token if it ever leaks.

Checking the token by hand, from the DumbMonit host:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credentials

| Credential | Fields |
|---|---|
| Personal access token | Sent as the `PRIVATE-TOKEN` header. |

Address: a host name (`gitlab.example.com`) or `host:port`. Options:
protocol (HTTPS by default), port (443), certificate check, request timeout
(15 s), the projects watched for CI health (none by default) and the failed
pipeline lookback window (24 h).

Validated against GitLab Enterprise Edition 17.5 (official API documentation).
