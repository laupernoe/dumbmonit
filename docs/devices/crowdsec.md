# CrowdSec

The security engine: active decisions and where they come from, alerts,
bouncers that stopped pulling, log lines read and those no parser recognised,
and whether the Local API answers.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 6060 | TCP | Prometheus metrics (default) |
| 8080 | TCP | Only with the Local API check option (on by default) |

DumbMonit reads CrowdSec's Prometheus metrics (port 6060) and checks `/health`
of the Local API (port 8080). With an optional bouncer key, it also asks the
Local API for the decisions on 192.0.2.1, a documentation address: the answer
is always empty, and proves that authentication and the database respond. It
never creates or deletes a decision and never downloads the ban list.

The failure worth watching is a bouncer that stopped pulling. The firewall or
reverse proxy keeps running with the list it had, new attackers get through,
and nothing complains. CrowdSec does not publish when a bouncer last pulled;
DumbMonit reads it from the bouncer's request counter, which stops moving.
The second one is an acquisition that broke (a log file renamed, a container
recreated): CrowdSec reads nothing, sees no attack, and stays green.

## What it watches

All metrics are prefixed `dumbmonit_crowdsec_`. Totals are counters: charts
and rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `lapi_up` | 1 when the Local API answers `/health` | |
| `lapi_decisions_ok` | with a bouncer key: 1 when the Local API answered the decision query | |
| `decisions` | active decisions, community blocklists included | |
| `decisions_by_origin` | | `origin`: `crowdsec`, `cscli`, `CAPI`, `lists`… |
| `alerts` | alerts kept by the Local API, community ones excluded | |
| `bouncers_seen` | bouncers that called the Local API since CrowdSec started | |
| `bouncer_requests_total` | requests of each bouncer; when it stops moving, the bouncer stopped pulling | `bouncer` |
| `machine_requests_total` | requests of each agent (machine) | `machine` |
| `lapi_requests_total` | all requests to the Local API | |
| `scenario_overflows_total` | scenarios triggered by the local agent: each one is an alert | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | log lines read by the agent, and those parsed or not | |
| `source_lines_total`, `source_lines_unparsed_total` | the same per log source | `source` |
| `buckets` | scenario buckets currently open | |
| `alerts_recent` | with a machine login: local alerts of the last 24 hours read from the Local API | |
| `alerts_by_country` | with a machine login: those alerts by country of origin (`source.cn`) | `country` |

The Local API families appear where the Local API runs, the log families where
an agent reads logs; the official image and a standard install run both.

The [built-in rules](../alerting/rules.md#databases-message-broker-and-security-engine)
that apply: CrowdSec Local API down, CrowdSec bouncer stopped pulling,
CrowdSec reads no logs, plus Monitor unreachable.

## The monitor page

The panel above the charts reads what the probe stored; opening the page never
queries CrowdSec. It says what is wrong in a sentence (Local API, bouncers,
log reading and the share of lines no parser recognised), then the active
decisions, alerts kept, scenarios triggered in the last 24 hours and lines
read per second, then the time since each bouncer last pulled, the decisions
by origin and the unparsed share of each log source.

With a machine login, a threat map follows: the alerts of the last 24 hours,
shaded by country. The same counts feed the threat map of the Security page,
next to the firewalls' blocks and Cloudflare's threats.

## Let DumbMonit read CrowdSec's metrics

1. Make the metrics reachable: set listen_addr in the prometheus section of /etc/crowdsec/config.yaml to an address DumbMonit can reach, then restart CrowdSec.

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. Check from the DumbMonit host that the metrics answer on port 6060, and filter that port so only the DumbMonit host reaches it.

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. DumbMonit also checks the Local API on port 8080 of the same host: untick Check the Local API for a host that only runs an agent.
4. Optional: create a bouncer key and paste it as Bouncer API key below.

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. Or, for the threat map of the Security page: create a machine login and choose Machine login below. Only the country of each alert is kept.

    ```
    sudo cscli machines add dumbmonit --password 'a-long-password'
    ```

6. In DumbMonit, enter the CrowdSec host, for example "crowdsec.lan".

### Required permissions

- **Bouncer API key (optional)** — Read-only: it can only read decisions
- **Machine login (optional)** — Reads the alerts; a machine can also send alerts, so keep it for DumbMonit only

!!! warning
    A bouncer that stops pulling keeps enforcing a frozen list: DumbMonit warns when a bouncer's requests stop for 30 minutes.

Vendor documentation: [CrowdSec Prometheus metrics](https://docs.crowdsec.net/docs/next/observability/prometheus/).

### Good to know

The metrics need no credential: they hold counts, bouncer and machine names and
log file paths, never an IP address. The official Docker image already listens
on every interface. `listen_uri` in the `api.server` section must be reachable
for the Local API check. A bouncer key can only read decisions: DumbMonit asks
for the decisions on 192.0.2.1, a documentation address, so the answer is
always empty and the ban list is never downloaded. Where the Local API and the
agents run on different hosts, add each host: decisions, alerts and bouncers
are counted where the Local API runs, log lines where each agent reads them.
DumbMonit only knows bouncers that pulled at least once since CrowdSec last
started.

A bouncer deleted with `cscli bouncers delete` keeps its counter until
CrowdSec restarts, and is reported as stopped until then. A bouncer that only
calls the Local API when it serves a request (a reverse proxy bouncer in live
mode) can stay quiet for half an hour on a calm night; switch it to stream
mode, or raise the rule's window.

## Credentials

| Credential | Fields |
|---|---|
| Metrics only | Nothing: reads `/metrics`, and `/health` of the Local API. |
| Bouncer API key | The key printed by `cscli bouncers add dumbmonit`, sent as `X-Api-Key` to the Local API. |
| Machine login | The name and password given to `cscli machines add`. DumbMonit logs in (`/v1/watchers/login`) and reads the local alerts of the last 24 hours (`/v1/alerts`, community ones excluded), keeping only the country of each one: no address is stored. |

Address: a host name or IP (`crowdsec.lan`), `host:port` for the metrics, or
a full URL. Options: protocol (HTTP by default), metrics port (6060), Check
the Local API (on), Local API port (8080), certificate check and request
timeout (10 s). Hub items (parsers, scenarios, collections) that are tainted
or out of date are not visible through the metrics, only with `cscli hub
list` on the host.
