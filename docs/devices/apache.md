# Apache httpd

The web server: busy and idle workers, the worker scoreboard by state, and
request/byte rates when ExtendedStatus is on.

DumbMonit reads Apache's built-in `mod_status`, queried with `?auto` so it
answers in plain `Key: Value` lines instead of the HTML page meant for a
browser.

## What it watches

All metrics are prefixed `dumbmonit_apache_`. The `_total` metrics are
counters: charts and rules turn them into rates. Everything below
`workers_busy`/`workers_idle` and `scoreboard` needs `ExtendedStatus On`.

| Metric | What | Labels |
|---|---|---|
| `workers_busy`, `workers_idle` | worker processes or threads, by state | |
| `scoreboard` | worker slots by scoreboard state | `state` |
| `requests_total` | requests served since Apache started | |
| `sent_bytes_total` | bytes sent since Apache started | |
| `uptime_seconds` | time since Apache started | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | Apache's own running averages | |
| `cpu_load_ratio` | Apache's own `CPULoad` value | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | async connection counts (event and worker MPMs) | |
| `version_info` | presence; the `Server` response header | `version` |

`scoreboard` always adds up to the configured number of worker slots: states
are waiting, open_slot, starting, reading, sending, keepalive, dns_lookup,
closing, logging, finishing and idle_cleanup.

## The monitor page

The panel above the charts reads what the probe stored; opening the page
never queries Apache. It shows busy and idle workers, the scoreboard
breakdown, and, with ExtendedStatus, the request and byte rates over 24
hours.

## Enable Apache's mod_status

1. Enable mod_status (it ships with Apache and is usually on already).

    ```
    a2enmod status
    ```

2. Add this location to the Apache configuration, restricted to the DumbMonit host (adjust the address).

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Optional: turn on the request and byte counters.

    ```
    ExtendedStatus On
    ```

4. Reload Apache, then open http://apache.lan/server-status?auto from the DumbMonit host.

    ```
    apachectl graceful
    ```

5. In DumbMonit, enter the Apache host, for example apache.lan, and the path if it is not /server-status.

!!! warning
    Without ExtendedStatus, requests and bytes are not reported at all: the chart stays empty rather than flat.

### Good to know

On Debian and Ubuntu the location goes in a file of its own, elsewhere in the main configuration. DumbMonit adds ?auto itself. Without ExtendedStatus only busy and idle workers and the scoreboard are reported. Apache does not publish its own version in mod_status; DumbMonit reads it from the Server response header instead, which some distributions trim down on purpose.

Vendor documentation: https://httpd.apache.org/docs/2.4/mod/mod_status.html

## Credentials

| Credential | Fields |
|---|---|
| No password | Nothing: the status page is filtered so that only DumbMonit reaches it. |
| Basic auth | The user and password of the block protecting the status page, if any. |

Address: a host name or IP, with the port if it is not 80. Options:
protocol (HTTP by default), port, status path (`/server-status` by default,
`?auto` added automatically), certificate check and request timeout (10 s).
