# Nginx

The web server and reverse proxy: active and waiting connections, the
accept/handled/request rates, and upstream health when the commercial NGINX
Plus API is available.

DumbMonit reads Nginx's built-in `stub_status` module, a handful of plain-text
counters, with no API and no account needed on a correctly filtered network.
When the commercial NGINX Plus is installed, DumbMonit can also read its JSON
API for upstream health and per-zone counters.

## What it watches

All metrics are prefixed `dumbmonit_nginx_`. The `_total` metrics are
counters: charts and rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `connections_active` | connections currently open | |
| `connections_reading`, `connections_writing`, `connections_waiting` | connections by what they are doing | |
| `accepts_total`, `handled_total` | connections accepted, and actually handled | |
| `requests_total` | requests served since Nginx started | |
| `version_info` | presence; the `Server` response header, when Nginx sends one | `server` |
| `plus_upstream_server_up` | 1 when NGINX Plus marks the server up | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | NGINX Plus per-server counters | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | NGINX Plus per-zone counters | `zone` |

A gap between `accepts_total` and `handled_total` means connections Nginx
dropped before it could serve them (usually `worker_connections` reached).
The `plus_*` metrics only appear when "Read the NGINX Plus API" is ticked and
NGINX Plus answers; the open-source build does not have an "upstream" to
report on `stub_status` alone.

## The device page

The panel above the charts reads what the probe stored; opening the page
never queries Nginx. It shows active, reading, writing and waiting
connections, the accept/handled/request rates over 24 hours, and, with NGINX
Plus, upstream health and per-zone traffic.

## Enable Nginx's stub_status module

1. Nginx ships stub_status built in: it only needs a location that turns it on. Add this to the server block Nginx already serves (or to its own server block on a port of its own), then reload Nginx.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Restrict that location to the DumbMonit host, so no one else can read it; adjust the address to match.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Check that it answers before configuring DumbMonit.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. In DumbMonit, enter the Nginx host, for example "nginx.lan", and the path if it is not "/basic_status". If NGINX Plus is installed, tick "Read the NGINX Plus API" to add upstream health and per-zone counters.

!!! warning
    stub_status only counts connections and requests: a configuration error
    in a server block, or a backend nginx cannot reach, does not show here.
    The open-source build has no notion of "upstream": that only exists in
    NGINX Plus, read through its own API when enabled.

The example above restricts the location by IP, the simplest setup for a
homelab; a `basic_auth` block works just as well if the status page cannot
sit behind a firewall rule of its own. `stub_status` itself cannot be
password-protected by Nginx directly — the password, if any, belongs to the
surrounding `location` block.

## Credentials

| Credential | Fields |
|---|---|
| No password | Nothing: the status page is filtered so that only DumbMonit reaches it. |
| Basic auth | The user and password of the block protecting the status page, if any. |

Address: a host name or IP, with the port if it is not 80. Options:
protocol (HTTP by default), port, status path (`/basic_status` by default),
certificate check, request timeout (10 s), and the NGINX Plus API switch
and version.
