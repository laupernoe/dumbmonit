# MinIO

!!! warning "Beta"

    This integration was built from the MinIO metrics documentation, not
    against a running server yet. It only reads. Tell us what breaks.

A MinIO object store through its Prometheus metrics: nodes and drives that
went offline, raw and usable capacity, objects and buckets, the self-healing
that fails, replication failures and S3 requests that end in a server error.

DumbMonit reads `/minio/v2/metrics/cluster` with the JWT that
`mc admin prometheus generate` makes, sent as `Authorization: Bearer`, never
in the URL. The token opens the metrics pages and nothing else: no S3 access,
no console, no other right. If MinIO runs with
`MINIO_PROMETHEUS_AUTH_TYPE=public`, the pages need no token. The cluster page
is required; `/minio/v2/metrics/node` (uptime of the node asked) is optional.
Metric names changed across MinIO releases (`disk` became `drive`): both are
read, and a family the server does not publish simply produces no figure. The
console port (9001) is not used.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 9000 | TCP | S3 API and metrics (HTTP or HTTPS, default) |

## What it watches

All metrics are prefixed `dumbmonit_minio_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version`, `commit` | Value 1. |
| `nodes_online`, `nodes_offline` | | Nodes of the cluster. |
| `drives_online`, `drives_offline`, `drives` | | Drives of the cluster. |
| `capacity_raw_bytes`, `capacity_raw_free_bytes`, `capacity_raw_used_percent` | | Raw capacity. |
| `capacity_usable_bytes`, `capacity_usable_free_bytes`, `capacity_usable_used_percent` | | Usable capacity, after parity. |
| `usage_bytes`, `objects`, `buckets` | | Data stored, objects, buckets. |
| `heal_objects_failed` | | Objects the current self-healing run could not repair. |
| `heal_idle_seconds` | | Time since the last healing activity; not published until the first run ends. |
| `heal_active` | | 1 when healing was active in the last five minutes. |
| `replication_failed_objects` | | Objects that failed to replicate (published when the server exposes it). |
| `s3_requests_total`, `s3_requests_4xx_total`, `s3_requests_5xx_total` | | S3 request counters. 5xx are server errors; 4xx are client errors. |
| `uptime_seconds` | | Uptime of the node answering. |

The [built-in rules](../alerting/rules.md#minio) that apply:

- **MinIO node offline**: for five minutes (Warning).
- **MinIO drive offline**: for five minutes (Advisory).
- **MinIO capacity high**: usable capacity above 85 % for 30 minutes (Advisory).
- **MinIO heal failures**: for 30 minutes (Advisory).
- **MinIO replication failures**: for 30 minutes (Advisory).
- **MinIO S3 server errors**: more than 5 % of requests in 5xx over ten minutes
  (Advisory).

## The monitor page

The Object store panel says what is wrong in a sentence and shows nodes and
drives online, capacity used and the number of objects.

## Generate a Prometheus token for DumbMonit

1. With the mc client configured for your MinIO (alias myminio), generate the token of the cluster metrics.

    ```
    mc admin prometheus generate myminio cluster
    ```

2. Copy the value after bearer_token in the output: that is the token.

3. In DumbMonit, enter the MinIO address (for example minio.lan), paste the token as Prometheus token and choose https if MinIO serves TLS.

### Required permissions

- **Prometheus token** — Opens the metrics pages only, no S3 or console access

!!! warning
    The token only opens the metrics pages, but anyone holding it can read them. If MinIO runs with MINIO_PROMETHEUS_AUTH_TYPE=public, the metrics need no token: choose No authentication.

### Notes

- Replication failures and 5xx counters come from the families recent MinIO
  releases publish; an older release may not have them.

## Credentials

| Credential | Fields |
|---|---|
| Prometheus token | The `bearer_token` printed by `mc admin prometheus generate`. |
| No authentication | For a server with `MINIO_PROMETHEUS_AUTH_TYPE=public`. |

Address: the host name or IP of any node, `host:port`, or a URL. Options:

- **Protocol** (http): `http` or `https`.
- **Port** (9000): the S3 API port, not the console (9001).
- **Accept an unverifiable certificate**: for a server with TLS and a
  self-signed certificate.
- **Timeout per request** (15 s).

## Troubleshooting

"MinIO accepted the credentials but refused … (403)": the token is wrong or
was made for another server's key. Generate a new one.

"does not look like MinIO cluster metrics": the address or port is not the S3
API of a MinIO server (9000, not the console).

A node of a distributed cluster that does not answer is reported by the
others: point DumbMonit at a node that stays up, and add a ping or port
monitor on each node to see which one.
