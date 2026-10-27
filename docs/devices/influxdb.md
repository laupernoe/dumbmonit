# InfluxDB

InfluxDB 2 and 3: health and readiness, uptime, memory, file descriptors and server errors.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8086 | TCP | InfluxDB 2 API (default port) |
| 8181 | TCP | Only for InfluxDB 3, whose default port is 8181 |

One device type covers both generations; the version is recognized from the answer of `/health`. InfluxDB 2 answers a JSON object (`status` pass or fail) and needs no token. InfluxDB 3 (Core and Enterprise) answers `OK` and asks for a bearer token unless it was started with `--disable-authz health,ping,metrics`.

The internal metrics come from `/metrics`, read on a best-effort basis: an instance that refuses them keeps the health and readiness series.

## What it watches

All metrics are prefixed `dumbmonit_influxdb_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version`, `generation` |
| `healthy`, `ready` | 1 when `/health` passes, and when the server says it is ready |  |
| `uptime_seconds` | since the server started |  |
| `goroutines`, `memory_resident_bytes` | process internals |  |
| `open_fds`, `open_fds_percent` | file descriptors, and their share of the limit |  |
| `http_requests_total`, `http_requests_5xx_total` | API requests, and those answered with a 5xx status (counters) |  |

The [built-in rules](../alerting/rules.md#more-databases-and-kafka)
that apply: InfluxDB unhealthy, InfluxDB not ready, InfluxDB server errors, plus Monitor unreachable.

## Let DumbMonit read InfluxDB's health

1. InfluxDB 2: nothing to create, /health, /ready and /metrics are open. Leave the credential on No authentication.

2. InfluxDB 3 asks for a token on /health and /metrics by default: paste a token as API token, or start the server with --disable-authz health,ping,metrics.

3. In DumbMonit, enter the server address, for example influx.lan, with the port 8086 for InfluxDB 2 or 8181 for InfluxDB 3.

4. Tick HTTPS if the server, or the reverse proxy in front of it, serves TLS.

!!! warning
    InfluxDB 3 answers 401 on /health without a token unless it runs with --disable-authz. The token is only used to read health and metrics.

### Why these permissions

InfluxDB 2 leaves its health and metrics endpoints open. For InfluxDB 3 any valid token is enough to read them; DumbMonit never queries or writes data.

## Credentials

| Credential | Fields |
|---|---|
| No authentication | InfluxDB 2, or InfluxDB 3 started with `--disable-authz`. |
| API token | A token, sent as a bearer token. Needed by InfluxDB 3 on `/health`. |

Address: a host name or IP (`influx.lan`), `host:port`, or a full URL. Options: protocol (HTTP by default), port (8086, or 8181 for InfluxDB 3), certificate check, request timeout (10 s).
