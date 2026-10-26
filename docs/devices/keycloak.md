# Keycloak (beta)

Whether Keycloak can really log people in: readiness with the database check, version, realms (a disabled one is a locked-out population), and, with a service account, active sessions and users per realm.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8080 | TCP | API (HTTP, default) |
| 9000 | TCP | Health checks on the management port (Keycloak 25 and later) |
| 443 | TCP | Only with the https scheme behind a reverse proxy |

The failure worth watching: **Keycloak up, database down**. The login page still renders, nobody can log in, and every application behind Keycloak looks broken at once. `/health/ready` is where Keycloak says so; a `503` there is a measurement, not a transport error.

## What it watches

All metrics are prefixed `dumbmonit_keycloak_`.

| Metric | What | Labels |
|---|---|---|
| `ready`, `live` | 1 while the readiness / liveness check reports UP |  |
| `health_check_up` | each readiness check (database…), at most 32 | `check` |
| `admin_readable` | 1 when the service account could read the administration API (absent without credentials) |  |
| `version_info` | value 1 | `version` |
| `uptime_seconds`, `memory_used_percent` | time since start; JVM memory in use |  |
| `realms_total`, `realms_disabled` | realms, and disabled ones |  |
| `realm_sessions`, `realm_offline_sessions`, `realm_users` | active sessions, offline sessions and users of each realm read | `realm` |
| `sessions_active` | active sessions, all realms read |  |
| `scrape_errors` | calls that failed on the last probe |  |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: Keycloak not ready, Keycloak realm disabled, plus Monitor unreachable.

## Let DumbMonit read Keycloak's health and realms

1. Run Keycloak with health checks on, with the option below. From Keycloak 25 they answer on the management port 9000.

    ```
    KC_HEALTH_ENABLED=true
    ```

2. Optional, for realms, sessions and users: in the master realm, create a client named as follows, with Client authentication and Service accounts roles on, and Standard flow off.

    ```
    dumbmonit
    ```

3. On that client's Service accounts roles tab, assign the client roles listed below, then copy the secret from its Credentials tab.

4. In DumbMonit, enter Keycloak's address and the client ID and secret, or choose None (health checks only).

### Required permissions

- **view-realm**: Master-realm client role: realms and their state
- **view-users**: Master-realm client role: user counts

!!! warning
    Keycloak 24 and earlier serve the health checks on the main port: set the health port option to same. Health checks must be enabled on the server, or DumbMonit reports a missing /health/ready.

## Credentials

| Credential | Fields |
|---|---|
| None (health checks only) | Nothing is sent; the health checks are public. |
| Client ID / secret | The service-account client. DumbMonit asks for a token (`client_credentials`) on `/realms/<realm>/protocol/openid-connect/token` at each probe. |

Address: the host name Keycloak answers on (`sso.lan`), `host:port`, or a full URL (`https://sso.example.com`, with `/auth` on legacy installs). The health checks are read on the same host, on the health port. Options: protocol, port, health port (`health_port`, 9000; `same` for Keycloak 24 and earlier), health protocol (`health_scheme`), realm of the client (`auth_realm`, master), realms to read (`realms`), certificate check and request timeout (15 s).
