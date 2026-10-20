# Nginx

The web server and reverse proxy: active and waiting connections, the
accept/handled/request rates, and upstream health when the commercial NGINX
Plus API is available.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 80 | TCP | Status page (HTTP, default) |
| 443 | TCP | Only if the status page is served over HTTPS |

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

## The monitor page

The panel above the charts reads what the probe stored; opening the page
never queries Nginx. It shows active, reading, writing and waiting
connections, the accept/handled/request rates over 24 hours, and, with NGINX
Plus, upstream health and per-zone traffic.

## Enable Nginx's stub_status module

1. Add this location to the server block Nginx already serves, restricted to the DumbMonit host (adjust the address).

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    ```

2. Reload Nginx.

    ```
    nginx -s reload
    ```

3. Open http://nginx.lan/basic_status from the DumbMonit host: it must show Active connections.

4. In DumbMonit, enter the Nginx host, for example nginx.lan, and the path if it is not /basic_status.

5. If NGINX Plus is installed, tick Read the NGINX Plus API to add upstream health and per-zone counters.

!!! warning
    stub_status only counts connections and requests: a configuration error in a server block or an unreachable backend does not show there.

### Good to know

Nginx ships stub_status built in: it only needs a location that turns it on, in the server block Nginx already serves or in a server block of its own on another port. The allow and deny lines restrict it to the DumbMonit host, so no one else can read it. The open-source build has no notion of upstream: that only exists in NGINX Plus, read through its own API when enabled.

Vendor documentation: https://nginx.org/en/docs/http/ngx_http_stub_status_module.html

## Credentials

| Credential | Fields |
|---|---|
| No password | Nothing: the status page is filtered so that only DumbMonit reaches it. |
| Basic auth | The user and password of the block protecting the status page, if any. |

Address: a host name or IP, with the port if it is not 80. Options:
protocol (HTTP by default), port, status path (`/basic_status` by default),
certificate check, request timeout (10 s), and the NGINX Plus API switch
and version.
