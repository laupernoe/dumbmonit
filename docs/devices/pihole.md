# Pi-hole

The DNS ad blocker: whether it is blocking, how many queries it answers and
blocks, how old its blocklists are, whether its upstream resolvers fail, whether
an update waits, and the messages of its own diagnosis page.

DumbMonit reads Pi-hole v6's REST API (`/api`). Pi-hole 5 and earlier have no
such API and are not supported.

What goes wrong with a Pi-hole rarely shows: DNS keeps answering. Blocking is
paused "for five minutes" and never comes back, the weekly blocklist update
(gravity) has failed for a month, an upstream resolver answers SERVFAIL, and the
diagnosis page nobody opens has been listing the reason all along.

## What it watches

All metrics are prefixed `dumbmonit_pihole_`. Pi-hole counts queries over the
last 24 hours, a sliding window that goes up and down: those figures are
gauges, suffixed `_24h`, not counters.

| Metric | What | Labels |
|---|---|---|
| `blocking_enabled` | 1 when blocking is on; 0 when it is off, paused or failed | |
| `blocking_timer_seconds` | time left before a paused blocking turns itself back on | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | queries answered and blocked in the last 24 hours | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | queries sent upstream, answered from the cache, distinct domains | |
| `queries_per_second` | average over the last minute | |
| `replies_servfail_24h` | SERVFAIL answers in the last 24 hours: an upstream resolver that fails shows here first | |
| `clients_active`, `clients_total` | clients seen | |
| `gravity_domains` | domains on the blocklists | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | last blocklist rebuild, and how long ago | |
| `version_info` | value 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 1 when a newer version of a component is published, 0 otherwise; absent until Pi-hole's daily check has run | `component` (`core`, `web`, `ftl`, `docker`) |
| `updates_available` | updates to install: the image in Docker (core, web and FTL come with it), otherwise each outdated component | |
| `messages`, `messages_by_type` | messages on the diagnosis page | `type` (`LOAD`, `RATE_LIMIT`, `LIST`…) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | the FTL daemon | |
| `privacy_level` | Pi-hole's privacy level, 0 to 3 | |
| `scrape_errors` | secondary API calls that failed during the last check | |

On a development branch, Pi-hole compares commits rather than version
numbers, and so does DumbMonit.

The [built-in rules](../alerting/rules.md#pi-hole) that apply: Pi-hole
blocking disabled, Pi-hole blocklists stale, Pi-hole update available, Pi-hole
diagnosis messages, plus Device unreachable.

## The device page

The panel above the charts reads what the check stored; opening the page never
connects to Pi-hole. It shows whether blocking is on (and when a pause ends),
the queries and share blocked in the last 24 hours, the blocklist size and age,
the clients, the versions with any update waiting, and the number of diagnosis
messages.

## Create an app password in Pi-hole

1. Pi-hole v6 or later is required: version 5 had no REST API. Pi-hole has no read-only account; the closest is an app password, which cannot change any setting.

2. In Pi-hole's web interface, open Settings → Web interface / API and switch the page from Basic to Expert. Click Configure app password, copy the password shown, then click Enable new app password. Pi-hole shows it only once, and a new one replaces the previous one.

3. An app password stays read-only for settings as long as webserver.api.app_sudo is false, the default. Check it on the Pi-hole machine (in Docker, prefix the command with docker exec and the container name):

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. In DumbMonit, enter the Pi-hole address, for example "pi.hole" or "192.168.1.53", and paste the app password. For a Pi-hole whose web interface has no password, pick No password instead.

5. DumbMonit only reads. It never pauses blocking and never changes a list, and it keeps one API session open between checks instead of opening one per check: Pi-hole allows 16 sessions at a time.

!!! warning
    The app password still allows pausing blocking and editing lists through the API: keep it for DumbMonit only. Pi-hole serves plain HTTP on port 80, and the password crosses the network each time a session opens: across an untrusted network, use HTTPS on port 443.

An app password also logs in without the second factor when two-factor
authentication is on: that is what it is for. To revoke it, empty
`webserver.api.app_pwhash` under Settings → All settings, or enable a new one:
the previous one stops working at once.

## Credentials

| Credential | Fields |
|---|---|
| App password | The password generated above. DumbMonit exchanges it for a session, sent in the `X-FTL-SID` header, and reuses that session while Pi-hole keeps it (30 minutes after its last use). |
| No password | Only for a Pi-hole whose web interface has no password. |

Address: a host name or IP (`pi.hole`), `host:port`, or a full URL
(`https://pi.hole`); the path of the web interface or of the API, copied from
the browser, is ignored. Options: protocol (HTTP by default), port (80), certificate check
(for HTTPS on 443, Pi-hole's own certificate is self-signed) and request
timeout (10 s).

If checks fail with "no free API session", Pi-hole's sessions are all taken,
usually by a script that logs in on every run without logging out. They expire
after 30 minutes; `webserver.api.max_sessions` raises the limit.
