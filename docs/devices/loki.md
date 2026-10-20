# Grafana Loki

The log server itself: whether it is ready, which log lines it refuses and
why, whether its chunks reach storage, and whether its write-ahead log still
has disk.

Loki publishes `/ready` and `/metrics` on its HTTP port; DumbMonit reads those
two and nothing else. It never runs a LogQL query and never reads a log line.

The failure worth watching is quiet. When Loki refuses a line (older than
`reject_old_samples_max_age`, over the tenant's ingestion rate or stream
limit, longer than `max_line_size`) the sender receives a 4xx, and Promtail,
Alloy or Fluent Bit usually drop the batch. Grafana shows nothing but a gap,
found the day someone looks for the log of an incident. The second one is
chunks that cannot be written to storage: Loki keeps them in memory, and loses
them on the next restart.

## What it watches

All metrics are prefixed `dumbmonit_loki_`. Totals are counters: charts and
rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `ready` | 1 when `/ready` answers `ready`, 0 while Loki starts or when a component cannot write or read | |
| `version_info` | value 1 | `version` |
| `received_lines_total`, `received_bytes_total` | what the distributor received, all tenants | |
| `discarded_lines_total` | lines refused, always present, even at zero | |
| `discarded_lines_reason_total` | the same by reason: `greater_than_max_sample_age`, `rate_limited`, `stream_limit`, `line_too_long`, `per_stream_rate_limit`… | `reason` |
| `flush_failures_total`, `flush_queue_length` | chunks that failed to reach storage, chunks waiting to | |
| `memory_streams` | streams held in memory by the ingester | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | write-ahead log writes lost to a full disk, and how full that disk is | |
| `requests_total`, `request_errors_total` | HTTP and gRPC requests, and those answered with a 5xx or a gRPC error. A 4xx is the client's fault and is not counted as an error. | |
| `ring_unhealthy_members` | members of the hash rings marked unhealthy | |
| `panics_total`, `log_errors_total` | panics recovered, error messages logged | |
| `retention_last_run_timestamp_seconds` | last successful retention run of the compactor, when retention is on | |
| `memory_resident_bytes` | | |

Loki split into components (simple scalable, microservices) exposes these
families where they belong: refused lines on the distributor, flush and WAL on
the ingester. Add the components you want to watch; a family a component does
not have simply produces no series.

The [built-in rules](../alerting/rules.md#log-and-metrics-servers) that apply:
Loki not ready, Loki refusing log lines, Loki cannot flush to storage, Loki
write-ahead log disk full, Loki request errors, plus Monitor unreachable.

## The monitor page

The panel above the charts reads what the probe stored; opening the page never
queries Loki. It says what is wrong in a sentence (readiness, lines refused in
the last 24 hours, chunks that failed to flush, a full WAL disk, unhealthy
ring members, server errors), then the lines and volume received per second,
the streams in memory and the WAL disk, then the lines refused in the last 24
hours by reason.

## Let DumbMonit read Loki's own health

1. Nothing to install: check from the DumbMonit host that Loki answers on its HTTP port (3100 by default).

    ```
    curl http://loki.lan:3100/ready
    ```

2. Loki has no login of its own: only if a reverse proxy in front asks for one, pick Username / password (or Bearer token) below.
3. In DumbMonit, enter Loki's address, for example "loki.lan" or "https://logs.lan/loki" behind a reverse proxy.
4. With Loki split into components (simple scalable or microservices), add at least the write path.

!!! warning
    Refused lines are the failure to watch: when Loki refuses a line (too old, over a rate or stream limit, too long), the sender usually drops the batch and Grafana only shows a gap in the logs.

Vendor documentation: [Loki meta-monitoring](https://grafana.com/docs/loki/latest/operations/meta-monitoring/).

### Good to know

No tenant is needed: `/ready` and `/metrics` are not per tenant. The write path
is where refused lines and flush failures are counted. DumbMonit only reads
`/ready` and `/metrics`: it never runs a LogQL query and never reads a log line.

## Credentials

| Credential | Fields |
|---|---|
| No authentication | The default: Loki itself has no login. |
| Username / password | The basic authentication of the reverse proxy in front of Loki. |
| Bearer token | A token your reverse proxy expects in `Authorization: Bearer`. |

Address: a host name or IP (`loki.lan`), `host:port`, or a full URL with a path
prefix (`https://logs.lan/loki`). Options: protocol (HTTP by default), port
(3100), certificate check and request timeout (10 s).
