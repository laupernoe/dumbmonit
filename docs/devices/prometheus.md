# Prometheus exporter

Any exporter that serves the Prometheus text format: node_exporter,
windows_exporter, cAdvisor, blackbox_exporter, smartctl_exporter, a database
exporter, or one you wrote. DumbMonit reads its `/metrics` page on every check
and stores each series, so it shows up in the explorer and the graphs and can be
used in threshold rules.

## Network ports to open

From the DumbMonit server (or its relay agent) to the exporter:

| Port | Protocol | Purpose |
|---|---|---|
| 9100 | TCP | Metrics page (HTTP, default for node_exporter) |

Other exporters use their own port; the **Exporter** list fills in the usual one:
windows_exporter 9182, cAdvisor 8080, blackbox_exporter 9115, snmp_exporter 9116,
smartctl_exporter 9633, mysqld_exporter 9104, postgres_exporter 9187,
redis_exporter 9121, nginx exporter 9113, Prometheus 9090.

## Setup

1. Check that the exporter answers from the DumbMonit host.
   ```
   curl http://server.lan:9100/metrics
   ```
2. In DumbMonit, add a **Prometheus exporter** monitor, enter the host and pick
   the exporter (or leave `custom` and give the port).
3. Optional: restrict what is stored with **Keep only** or **Ignore**.
4. Optional: if the exporter is on another site, choose a relay agent to read it
   from there.

You can also paste the whole URL as the address, for example
`http://server.lan:9100/metrics` or, for the blackbox exporter,
`http://bb.lan:9115/probe?target=example.org&module=http_2xx`.

## What it stores

Every series of the page is stored as `dumbmonit_<name>` with its own labels,
plus `target` (the monitor) and `host` (its name). A label named `target` or
`host` coming from the exporter is kept as `exported_target` or `exported_host`.
Series declared `counter` (and the `_bucket`, `_count`, `_sum` of histograms and
summaries) are stored as counters: graphs and rules turn them into rates with
`rate()`. Everything else is a gauge. `NaN` and infinite values are not stored.

Two extra series tell how the read went:

| Metric | What |
|---|---|
| `scrape_series` | Series stored by the last read. |
| `scrape_series_dropped` | Series left out because the limit was reached. |

## Options

| Option | Default | Purpose |
|---|---|---|
| Exporter | `custom` | Preset for the port. |
| Protocol, Port | `http`, preset port | Where the exporter listens. |
| Path | `/metrics` | Page to read, with its query if any. |
| Keep only | empty | Regular expression: only the metrics whose name matches are stored. |
| Ignore | empty | Regular expression: the metrics whose name matches are not stored. |
| Series limit | 5000 | At most this many series per read (1 to 50000); the surplus is counted. |
| Accept an unverifiable certificate | off | For an HTTPS exporter with a private certificate. |

Expressions are RE2-style (the Rust `regex` syntax), unanchored: write
`^node_` to match a prefix. They apply to the metric name, after `Keep only`.

## Authentication

None, a user name and password (HTTP basic) or a bearer token. Secrets are
stored encrypted and never shown again.

## Failures

An exporter that does not answer (connection refused, timeout, 5xx) marks the
monitor down. A refused login (401, 403), a page that is not Prometheus metrics
or a wrong path is a configuration problem: it is shown on the monitor but does
not notify anyone.

## Alerts

No rule is shipped, since the metrics depend on the exporter. Create a threshold
rule on any series, for example `100 - avg by (target) (rate(dumbmonit_node_cpu_seconds_total{mode="idle"}[5m])) * 100 > 90`
(see [Alert rules](../alerting/rules.md#creating-a-threshold-rule)).
