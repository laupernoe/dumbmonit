# Vaultwarden (beta)

Whether the password vault answers, which version runs, whether anyone can still create an account, and, with the admin token, how many accounts exist, are disabled or have no two-step login.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | API (HTTPS, default) |
| 80 | TCP | Only with the http scheme option, inside a trusted network |

A vault that does not answer is the failure that matters, and it is the plain `/alive` check that sees it: apps keep their offline copy for a while, so nobody notices until a new device needs to sync. Open sign-ups are the quieter issue worth a glance on a vault exposed to the internet.

## What it watches

All metrics are prefixed `dumbmonit_vaultwarden_`.

| Metric | What | Labels |
|---|---|---|
| `up` | value 1 while `/alive` answers |  |
| `version_info` | value 1 | `version` |
| `signups_open` | 1 when anyone can create an account |  |
| `admin_readable` | 1 when the admin page was read (absent without the admin token) |  |
| `users`, `users_disabled`, `users_without_2fa` | accounts; disabled ones; active ones with no two-step login |  |
| `items` | vault items, all accounts together |  |
| `scrape_errors` | calls that failed on the last probe |  |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: Vaultwarden sign-ups open, plus Monitor unreachable.

## Check Vaultwarden, and optionally read its admin page

1. Nothing to install: check from the DumbMonit host that Vaultwarden answers.

    ```
    curl https://vault.example.com/alive
    ```

2. Optional, for account counts: enable the admin page with an ADMIN_TOKEN on the server, and keep the plain token you type on the /admin page.

3. In DumbMonit, enter Vaultwarden's address; paste the admin token, or choose No admin token for the public checks only.

!!! warning
    The admin token opens Vaultwarden's admin page and can change every setting: store it only in DumbMonit, or leave it out if you do not need the counts. DumbMonit reads counters only, never a name, e-mail address or vault item.

## Credentials

| Credential | Fields |
|---|---|
| No admin token | Public checks only: `/alive` and `/api/config`. |
| Admin token | The plain ADMIN_TOKEN. Vaultwarden has no admin API key: the token opens a session on `/admin` (cookie valid 20 minutes), which DumbMonit keeps for 15 minutes between probes, because the login is rate-limited. |

Address: a host name (`vault.example.com`), `host:port`, or a full URL with a path prefix. Options: protocol (HTTPS by default), port (443), certificate check and request timeout (15 s).
