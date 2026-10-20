# Caddy

The web server and reverse proxy: upstreams failing their health checks, a
configuration reload that failed, handler errors and the share of 5xx answers.

DumbMonit reads three things from Caddy's admin API, all with `GET`:
`/config/apps/http/servers` (the HTTP configuration Caddy runs, never the
`tls` section where DNS provider tokens live), `/reverse_proxy/upstreams` (the
backends and the failures the passive health check remembers) and `/metrics`
(upstream health, reloads, requests and errors).

Caddy's admin API is not read-only: whoever reaches it can load a new
configuration or stop Caddy. It listens on `localhost:2019` for that reason.
The setup below keeps it there and lets Caddy itself publish a read-only view
of it, protected by a password, that answers `GET` on the three paths above
and refuses everything else.

## What it watches

All metrics are prefixed `dumbmonit_caddy_`. Totals are counters: charts and
rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `config_loaded` | 1 when Caddy runs at least one HTTP server | |
| `servers`, `routes` | HTTP servers and routes in the configuration | |
| `config_last_reload_ok` | 0 when the last configuration reload failed | |
| `config_last_reload_timestamp_seconds` | time of the last successful reload | |
| `upstreams`, `upstreams_unhealthy` | backends of every `reverse_proxy`, and those failing | |
| `upstream_healthy` | 1 when neither the active nor the passive health check rules the backend out | `upstream` |
| `upstream_fails` | failed requests the passive health check currently remembers | `upstream` |
| `upstream_requests` | requests in flight to the backend | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | with the `metrics` option: requests, 5xx answers and handler errors | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | the same per server (`srv0`, `srv1`… or the names you gave) | `server` |

`upstream_healthy` needs `/metrics`: the active health check is only visible
there. A read-only view that does not let `/metrics` through still reports the
backends and their remembered failures. Caddy does not publish its version
through the admin API.

The [built-in rules](../alerting/rules.md#reverse-proxies-and-domains) that
apply: Caddy upstream unhealthy, Caddy configuration reload failed, Reverse
proxy answering 5xx, plus Monitor unreachable.

## The monitor page

The panel above the charts reads what the probe stored; opening the page never
queries Caddy. It says what is wrong in a sentence (no configuration, a failed
reload, unhealthy upstreams, a high share of 5xx), then the servers, routes,
upstreams, request rate, 5xx share and handler errors over 24 hours, then each
upstream with its remembered failures.

## Publish a read-only view of Caddy's admin API

1. Hash a password for the user dumbmonit.

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

2. Add this site to the Caddyfile, put the hash in place of HASH, then reload Caddy.

    ```
    :2020 {
    basic_auth {
    dumbmonit HASH
    }
    @readonly {
    method GET
    path /config/apps/http/servers /reverse_proxy/upstreams /metrics
    }
    handle @readonly {
    reverse_proxy localhost:2019 {
    header_up Host localhost:2019
    }
    }
    respond 403
    }
    ```

3. Optional: add metrics to the global options block at the top of the Caddyfile to count requests and errors.

    ```
    metrics
    ```

4. Filter port 2020 so that only the DumbMonit host reaches it.

5. In DumbMonit, enter the Caddy host with that port, for example caddy.lan:2020, and the user dumbmonit with its password.

### Required permissions

- **GET on /config/apps/http/servers**: Servers and routes
- **GET on /reverse_proxy/upstreams**: Upstream health
- **GET on /metrics**: Request counters (optional)

!!! warning
    Never expose the admin API itself: whoever reaches port 2019 can replace the configuration or stop Caddy.

### Why these permissions

Caddy's admin API can load a new configuration and stop Caddy: it must stay out of reach of anything but DumbMonit. Leave it where Caddy puts it, on localhost:2019, and let Caddy publish a read-only view of it: GET only, on the three paths DumbMonit reads, behind a password. The site above answers on port 2020 and refuses everything else. If you do expose the admin API on a private network, filter the port so that only the DumbMonit host reaches it, and pick No password; the origins option does not stop a client that sends no Origin header. Without metrics, upstream health and reloads are still reported.

Vendor documentation: https://caddyserver.com/docs/api

## Credentials

| Credential | Fields |
|---|---|
| Read-only view | The user and password of the `basic_auth` block above (recommended). |
| No password | Nothing: the admin API itself, filtered so that only DumbMonit reaches it. |

Address: a host name or IP with the port of the read-only view
(`caddy.lan:2020`), or a full URL. Options: protocol (HTTP by default), port
(2019, the admin API itself), certificate check and request timeout (10 s).

The passive health check (`fail_duration`) only remembers failures for that
duration: a backend that fails once and recovers is reported unhealthy until
then. The rule waits five minutes before it fires.
