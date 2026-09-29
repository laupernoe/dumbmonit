# VictoriaMetrics and VictoriaLogs

The time series database and the log database themselves: whether they accept
what they are sent, how much disk is left before they stop accepting anything,
and what they refuse.

Two device types share this page, because the two products share their
foundations: `victoriametrics` for a single-node VictoriaMetrics, and
`victorialogs` for a single-node VictoriaLogs. Both publish their own health on
`/health` and `/metrics`, on the port of their API; DumbMonit reads those two
and nothing else. It never runs a query against your data and never writes.

These servers rarely go down outright. They go down *by half*: they keep
answering queries and the dashboards still open, while every new sample or log
line is refused, because the disk crossed the read-only limit or the data came
with a timestamp outside retention. Whatever pushes to them (vmagent,
Prometheus remote write, Vector, Fluent Bit, Promtail) retries, then drops.
This page is about seeing that coming.

## What it watches

Metrics are prefixed `dumbmonit_victoriametrics_` or
`dumbmonit_victorialogs_`. Totals are counters: charts and rules turn them into
rates.

| Metric | What | Labels |
|---|---|---|
| `healthy` | 1 when `/health` answers OK, 0 when it answers anything else | |
| `version_info` | value 1 | `version` |
| `rows_ingested_total` | samples (VictoriaMetrics) or log lines (VictoriaLogs) accepted | |
| `bytes_ingested_total` | log volume accepted (VictoriaLogs only) | |
| `rows_rejected_total` | everything refused, always present, even at zero | |
| `rows_rejected_reason_total` | the same, by reason: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | free and total space of the storage volume | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: below it, the storage turns read-only | `path` |
| `disk_headroom_percent` | free space left above that limit, as a share of the disk | `path` |
| `read_only` | 1 once the storage has switched to read-only | |
| `data_size_bytes` | data on disk | |
| `slow_inserts_total`, `rows_added_total` | new samples that took the slow path, and all samples added (VictoriaMetrics only) | |
| `active_series` | series that received a sample in the last hour (VictoriaMetrics only) | |
| `http_errors_total` | requests answered with an error | |
| `log_errors_total` | error messages the server logged | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**A refused sample is lost.** VictoriaMetrics refuses a sample older than its
retention or more than two days in the future, a label value longer than
`-maxLabelValueLen`, a series over `-maxLabelsPerTimeseries`, and every sample
of a new series once `-storage.maxHourlySeries` or `-storage.maxDailySeries` is
reached. VictoriaLogs refuses lines outside its retention and lines with too
many fields. The client usually gets no error for these: only this counter
knows.

**Slow inserts** are samples whose series VictoriaMetrics had to look up on
disk instead of in its cache. Every brand-new series is one, so a deployment
causes a burst; a share that stays above 5% means the server lacks memory for
its active series. That is the threshold the VictoriaMetrics documentation
gives.

The [built-in rules](../alerting/rules.md#log-and-metrics-servers) that apply:
Metrics or log storage read-only, Metrics or log storage almost read-only,
Metrics or log server unhealthy, Metrics or log data refused, VictoriaMetrics
slow inserts, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries the server. It says, in this order:

1. **What is wrong, in a sentence**: health, storage (*Read-only*, or how
   much of the disk is left above the limit), data refused in the last 24
   hours, slow inserts, errors logged. Each line carries a word as well as a
   colour.
2. **What the server does**: samples or lines ingested per second, active
   series or ingested volume, data on disk, uptime.
3. **What was refused in the last 24 hours, by reason**, and each storage
   path's free space against its read-only limit.

## Let DumbMonit read VictoriaMetrics' own health

1. Nothing to install: VictoriaMetrics publishes its own health on /health and /metrics, on the port of its API (8428 by default). Check from the DumbMonit host that both answer.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. If VictoriaMetrics runs with -httpAuth.username and -httpAuth.password, pick Username / password below and enter those. Behind vmauth or a reverse proxy that expects a bearer token, pick Bearer token. If -metricsAuthKey is set, /metrics only answers with that key in the URL, which DumbMonit does not send: protect /metrics with the basic authentication above instead.

3. In DumbMonit, enter the server address, for example "victoriametrics.lan" or "http://10.0.0.5:8428". Behind a reverse proxy, enter the full URL with its path prefix. This covers the single-node server; the components of a VictoriaMetrics cluster expose other metrics and are not covered.

4. DumbMonit only reads /health and /metrics. It never runs a query against your data and never writes anything.

!!! warning
    Free disk space is what to watch: below -storage.minFreeDiskSpaceBytes (100 MB by default) VictoriaMetrics switches to read-only and refuses every new sample, while it keeps answering queries as if nothing happened. DumbMonit shows the headroom left above that limit and warns under 10%.

## Let DumbMonit read VictoriaLogs' own health

1. Nothing to install: VictoriaLogs publishes its own health on /health and /metrics, on the port of its API (9428 by default). Check from the DumbMonit host that both answer.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_rows_ingested_total
    ```

2. If VictoriaLogs runs with -httpAuth.username and -httpAuth.password, pick Username / password below and enter those. Behind vmauth or a reverse proxy that expects a bearer token, pick Bearer token. If -metricsAuthKey is set, /metrics only answers with that key in the URL, which DumbMonit does not send: protect /metrics with the basic authentication above instead.

3. In DumbMonit, enter the server address, for example "victorialogs.lan" or "http://10.0.0.6:9428". Behind a reverse proxy, enter the full URL with its path prefix.

4. DumbMonit only reads /health and /metrics. It never runs a LogsQL query and never reads a log line.

!!! warning
    Free disk space is what to watch: below -storage.minFreeDiskSpaceBytes (10 MB by default) VictoriaLogs switches to read-only and refuses every new log line. DumbMonit shows the headroom left above that limit and warns under 10%.

## Credentials

| Credential | Fields |
|---|---|
| No authentication | The default: `/health` and `/metrics` are open unless you protected them. |
| Username / password | The `-httpAuth.username` and `-httpAuth.password` of the server, or of the reverse proxy in front of it. |
| Bearer token | A token that vmauth or your reverse proxy expects in `Authorization: Bearer`. |

Address: a host name or IP (`victoriametrics.lan`, `10.0.0.5`), `host:port`,
or a full URL with a path prefix behind a reverse proxy
(`https://metrics.lan/vm`). Options: protocol (HTTP by default), port (8428 or
9428), certificate check and request timeout (10 s).
