# authentik (beta)

Whether authentik can still sign people in and keep its background work running: readiness, workers, failed system tasks, outposts that stopped reporting, version updates and the number of users.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 9000 | TCP | API (HTTP, default) |
| 9443 | TCP | Only with the https scheme on authentik's own HTTPS port |
| 443 | TCP | Only with the https scheme behind a reverse proxy |

The failure worth watching: **work that silently stops**. A worker that is gone means no source synchronisation, no e-mail, no clean-up, with nothing but a counter in the administration. An outpost that stopped reporting takes every application it protects down with it.

## What it watches

All metrics are prefixed `dumbmonit_authentik_`.

| Metric | What | Labels |
|---|---|---|
| `live`, `ready` | 1 while `/-/health/live/` / `/-/health/ready/` answer 2xx (ready covers the database and cache) |  |
| `version_info` | value 1 | `version` |
| `update_available` | 1 when a newer release exists (absent when authentik cannot read the latest version) | `latest_version` |
| `workers`, `workers_version_mismatch` | workers reporting, and those on another version than the server |  |
| `tasks_failed`, `tasks_warning` | system tasks whose last run failed / ended with a warning |  |
| `task_failed` | value 1 for each failed task, at most 32 | `task` |
| `outposts_total`, `outposts_disconnected` | outposts that serve at least one provider, and those with no instance seen in five minutes |  |
| `outpost_instances` | instances seen in the last five minutes (at most 10 outposts) | `outpost` |
| `users` | user accounts |  |
| `scrape_errors` | calls that failed on the last probe |  |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: authentik not ready, authentik has no worker, authentik task failed, authentik outpost disconnected, authentik update available, plus Monitor unreachable.

## Create an API token in authentik

1. In authentik, open Directory → Users → Create a service account and name it as follows, without a random password.

    ```
    dumbmonit
    ```

2. Put that user in a group that can open the authentik interface, so it can read the system pages.

3. Open Directory → Tokens and App passwords, create a token for that user with the intent API Token and no expiry, and copy its key.

4. In DumbMonit, enter authentik's address and paste the key.

!!! warning
    authentik has no read-only role for its system pages: the token's user must be allowed into the interface, so keep the key only in DumbMonit. Pages the token cannot read are skipped, not failed.

## Credentials

| Credential | Fields |
|---|---|
| API token | Sent as `Authorization: Bearer …`. |

Address: a host name (`auth.lan`), `host:port`, or a full URL (`https://auth.example.com`). Options: protocol (HTTP by default), port (9000), certificate check and request timeout (15 s).
