# Forgejo / Gitea

Forgejo and Gitea share the same REST API, so DumbMonit reads both with one
collector: whether the instance answers, its health check, how many
repositories, accounts and organisations it holds, whether its scheduled
tasks (repository checks, cleanups, the update checker) still run on their
own interval, and whether its Actions runners are online.

Two calls must succeed: `/api/v1/version`, public, and `/api/v1/user`, the
proof that the **access token** is valid. Everything else — the health
check, the repository/account/organisation counts, the scheduled tasks and
the Actions runners — is read from endpoints Forgejo and Gitea reserve to
site administrator accounts, and is skipped without failing the probe when
the token belongs to an ordinary one.

The failure worth watching here: **a scheduled task that stopped running**.
Forgejo and Gitea keep serving pages and git operations while their internal
scheduler is stuck; nobody notices until the update checker or a repository
consistency check has not run in weeks.

## What it watches

All metrics are prefixed `dumbmonit_forgejo_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `token_is_admin` | 1 when the token's account is a site administrator | |
| `healthz_ok` | 1 unless the health check reports `fail` | |
| `healthz_check` | 1 per failing component (database, cache) | `check` |
| `repos` | repositories visible to the token's account | |
| `users`, `orgs` | accounts and organisations on the instance | |
| `cron_tasks` | scheduled tasks defined on the instance | |
| `cron_task_overdue` | 1 per task whose next run is already due, 0 otherwise | `task` |
| `runners_total`, `runners_online`, `runners_offline` | Actions runners known to the instance | |
| `runner_offline` | value 1 per offline runner (at most 32) | `runner` |
| `scrape_errors` | administrator-only calls that failed on the last probe (0 with an ordinary token's ones never attempted) | |

The [built-in rules](../alerting/rules.md#self-hosted-forges) that apply:
Forgejo/Gitea runners all offline, Forgejo/Gitea scheduled task overdue, plus
Device unreachable. A security score checks the scheduled tasks: see
[Vendor updates and other checks](../using/security-score.md).

## What it does not watch

Forgejo and Gitea have no read-only administrator role: every call reserved
to "admin" needs an account with full rights, not a scope-limited delegate.
Their API also has no equivalent of GitLab's sign-up or two-factor settings,
nor a self-reported "update available" flag — compare the version above
against the project's [release notes](https://forgejo.org/releases/) by hand.

## The device page

The panel above the charts reads what the probe stored; opening the page
never calls Forgejo or Gitea. It says what is wrong first (the health check
failing, runners all offline, a scheduled task overdue), then the version,
then the repository, account and organisation counts.

## Create an access token on a site administrator account

1. Create an access token on a site administrator account: avatar → Settings → Applications → Generate new token. Name it as follows and tick the read:admin, read:repository and read:user scopes.

    ```
    dumbmonit
    ```

2. read:admin opens the administration counts, the scheduled tasks and the Actions runners; without it, only the version and the token's own account are read, and the rest is skipped without failing the probe. Forgejo and Gitea have no read-only administrator role either.

3. In DumbMonit, enter the address, for example "forgejo.lan" (port 3000) or "https://git.example.com" behind a reverse proxy, and paste the token.

!!! warning
    An access token on a site administrator account can read and change everything the account can, whatever scopes are ticked: Forgejo and Gitea only gate which API routes a scope opens, not how much an administrator account itself can do. Dedicate a bot account to it rather than reusing your own.

Checking the token by hand, from the DumbMonit host:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credentials

| Credential | Fields |
|---|---|
| Access token | Sent as `Authorization: token …`. |

Address: a host name (`forgejo.lan`) or `host:port`. Options: protocol
(HTTP by default — put a reverse proxy with TLS in front for anything but a
LAN), port (3000), certificate check and request timeout (15 s).

Validated against Forgejo 9.0 (official API source and Swagger definitions,
shared with Gitea).
