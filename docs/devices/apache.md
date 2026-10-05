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

## The device page

The panel above the charts reads what the probe stored; opening the page
never queries Apache. It shows busy and idle workers, the scoreboard
breakdown, and, with ExtendedStatus, the request and byte rates over 24
hours.

## Enable Apache's mod_status

1. Make sure mod_status is loaded (it ships with Apache and is usually enabled by default).

    ```
    a2enmod status
    ```

2. Add a status location, restricted to the DumbMonit host; adjust the address to match. On Debian and Ubuntu this goes in a file of its own, elsewhere in the main configuration.

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Turn on the request and byte counters (optional, but worth it): without it, only busy/idle workers and the scoreboard are reported.

    ```
    ExtendedStatus On
    ```

4. Reload Apache and check that it answers before configuring DumbMonit.

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. In DumbMonit, enter the Apache host, for example "apache.lan", and the path if it is not "/server-status" (DumbMonit adds "?auto" itself).

!!! warning
    Without ExtendedStatus, requests and bytes are not reported at all, not
    as zero: the chart stays empty rather than lying flat. Apache does not
    publish its own version in mod_status; DumbMonit reads it from the
    Server response header instead, which some distributions trim down on
    purpose.

On Debian and Ubuntu, the status location usually already exists in
`/etc/apache2/mods-available/status.conf`; editing the `Require` line there
is enough, no new file needed. Where the configuration is a single file
(most other distributions), add the block to the main virtual host instead.

## Credentials

| Credential | Fields |
|---|---|
| No password | Nothing: the status page is filtered so that only DumbMonit reaches it. |
| Basic auth | The user and password of the block protecting the status page, if any. |

Address: a host name or IP, with the port if it is not 80. Options:
protocol (HTTP by default), port, status path (`/server-status` by default,
`?auto` added automatically), certificate check and request timeout (10 s).
