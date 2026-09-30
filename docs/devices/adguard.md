# AdGuard Home

The DNS filter itself: whether protection is on, whether its DNS server runs,
what it answered and blocked, whether its filter lists still update, whether
its upstream servers answer, and whether a newer version is out.

A broken DNS filter does not look broken. Pages keep opening, with their ads.
The usual failures are quiet: protection paused "for an hour" from the phone
app and never turned back on, filter lists that stopped downloading weeks ago,
an upstream DNS-over-HTTPS server that no longer answers so that every lookup
waits for the next one.

DumbMonit reads AdGuard Home's REST API (`/control`) with HTTP basic
authentication. Tested against AdGuard Home v0.107.79 (and the v0.107.78
version check).

## What it watches

All metrics are prefixed `dumbmonit_adguard_`. Query and block figures are
**sums over AdGuard Home's statistics window** (24 hours unless changed in
Settings → General settings), not counters: they go up and down as the window
slides, and are stored as they are.

| Metric | What | Labels |
|---|---|---|
| `running` | 1 when the DNS server runs | |
| `protection_enabled` | 1 when protection is on | |
| `protection_paused_seconds` | time left before a paused protection comes back on, 0 without a pause | |
| `version_info` | value 1 | `version` |
| `dns_queries` | queries in the statistics window | |
| `blocked_filtering` | queries blocked by the filter lists and custom rules | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | answers replaced by Browsing security, Parental control, Safe search | |
| `blocked_percent` | share of queries blocked (filters, Browsing security, Parental control); absent without any query | |
| `avg_processing_seconds` | average time to answer | |
| `stats_window_seconds`, `stats_enabled` | the statistics window and whether statistics are kept (0.107.30 and later) | |
| `upstream_responses`, `upstream_avg_seconds` | answers and average answer time per upstream in the window, as the dashboard's Top upstreams shows them (at most 8) | `upstream` |
| `upstream_up` | 1 when AdGuard Home's own test of that upstream passes; 0 when it fails or gives no answer within 4 seconds (at most 8). The label is the upstream as AdGuard Home writes it in its statistics (`https://dns10.quad9.net:443/dns-query`, `192.0.2.1:53`); for an upstream that stayed silent, the line as configured. | `upstream` |
| `filtering_enabled` | 1 when filtering is on in Settings → General settings | |
| `filters`, `filters_enabled` | blocklists configured, and enabled | |
| `filter_rules` | rules loaded from the enabled lists | |
| `filter_rules_count`, `filter_update_age_seconds` | per enabled list (at most 32) | `filter` |
| `filter_oldest_update_age_seconds` | age of the oldest update among enabled lists | |
| `filters_never_updated` | enabled lists that were never downloaded | |
| `filter_update_interval_hours` | how often lists are updated; 0 turns updates off | |
| `update_check_enabled` | 0 when AdGuard Home runs with its version check off | |
| `update_available` | 1 when the last version check announced a newer version; absent when the check is off | `latest` |
| `scrape_errors` | secondary calls that failed on the last check | |

The version check is off in the official Docker image and in most
distribution packages, which update AdGuard Home themselves: there,
`update_available` is never written and its rule stays quiet.

Upstream statistics only count answers: an upstream that stops answering
leaves no trace in them. That is why DumbMonit asks AdGuard Home to test each
upstream at every check, the same test as the Test upstreams button in
Settings → DNS settings: one DNS query from AdGuard Home to that upstream. A
dead upstream would keep AdGuard Home waiting twice its upstream timeout (20
seconds by default); DumbMonit stops waiting after 4 seconds and reports it
down. Untick the Test upstream servers option to turn this off.

The [built-in rules](../alerting/rules.md#adguard-home) that apply:
AdGuard Home protection off, AdGuard Home DNS not running, AdGuard Home
upstream failing, AdGuard Home filter lists stale, AdGuard Home update
available, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
connects to AdGuard Home. It says whether protection and the DNS server are on
(and when a pause ends), then the queries, blocks and answer time over the
statistics window, the upstream servers with their test result and answer
time, and the filter lists with their rules and last update.

## Add a user for DumbMonit to AdGuard Home

1. AdGuard Home has no read-only account: every user listed in AdGuardHome.yaml can change every setting. Give DumbMonit a user of its own, so it can be removed without touching yours. First make a bcrypt hash of a long random password (htpasswd comes with the apache2-utils or httpd-tools package):

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. Stop AdGuard Home, open AdGuardHome.yaml (next to the AdGuardHome binary, or in the conf folder of the Docker volume) and add this line under "users:", at the same indentation as the "- name:" line already there. Replace HASH with what htpasswd printed after "dumbmonit:". Start AdGuard Home again.

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    The section then reads:

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. Check from the DumbMonit host that the user is accepted:

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. In DumbMonit, enter the address of the AdGuard Home web interface, for example "adguard.lan" (port 80) or "http://adguard.lan:3000", then the user dumbmonit and its password.

5. DumbMonit only reads the status, the statistics, the filter lists and the last version check. It never reads the query log. At every check it asks AdGuard Home to test each upstream DNS server, as the Test upstreams button does; untick Test upstream servers below to stop that.

!!! warning
    The dumbmonit user can change every setting, like any AdGuard Home user: keep its password to DumbMonit alone. It travels with every request, so across an untrusted network turn on HTTPS in Settings → Encryption settings. After five wrong passwords, AdGuard Home refuses the address for 15 minutes.

AdGuard Home must be stopped while you edit the file: it rewrites
AdGuardHome.yaml whenever a setting changes, and would drop a line added while
it runs.

## Credentials

| Credential | Fields |
|---|---|
| User name / password | The dedicated user added to AdGuardHome.yaml, sent as HTTP basic authentication. |
| No authentication | Only for an AdGuard Home with no user at all in AdGuardHome.yaml. |

Address: a host name or IP (`adguard.lan`), `host:port`, or a full URL with a
path prefix behind a reverse proxy (`https://dns.lan/adguard`); a trailing
`/control` is ignored. Options: protocol (HTTP by default), port (80),
certificate check, request timeout (5 s) and the upstream test (on).

## Calls

| Call | Why |
|---|---|
| `GET /control/status` | running, protection, version: the only call whose failure fails the check |
| `GET /control/stats`, `GET /control/stats/config` | queries, blocks, upstream answers, statistics window |
| `GET /control/filtering/status` | filter lists |
| `POST /control/version.json` with `{"recheck_now": false}` | the result of the last version check, as the web interface reads it on every page load; it never forces a new check |
| `GET /control/dns_info`, then `POST /control/test_upstream_dns` once per upstream | the configured upstreams and bootstrap servers, then the upstream test; changes no setting |
