# Server logs

DumbMonit writes its own logs to disk, lets an administrator download the last
days, and can forward them to a log server as they are written. Everything is in
**Settings → System → Logs** (administrators only).

## Where the files are

Logs live in the data volume, under `/data/logs/`:

```
/data/logs/dumbmonit-2026-10-20.log      today, plain text
/data/logs/dumbmonit-2026-10-19.log.gz   past days, compressed
```

Each line is one JSON object (JSON lines):

```json
{"ts":"2026-10-20T12:34:56.123Z","level":"INFO","target":"dumbmonit_server::scheduler","msg":"cycle done","fields":{"targets":42},"kind":"app"}
```

| Field | Meaning |
| --- | --- |
| `ts` | UTC time, RFC 3339 |
| `level` | `ERROR`, `WARN`, `INFO` or `DEBUG` |
| `target` | the module that wrote it |
| `msg` | the message |
| `fields` | structured values attached to the message |
| `kind` | `app` for the application, `audit` for audit-log entries (sign-ins, tokens, users…) |

The global filter `DUMBMONIT_LOG` (for example `info` or `dumbmonit_server=debug`)
decides what is logged at all. Everything below only sees what passes it.

No secret is written to the logs: passwords, tokens, API keys and similar values
are masked before a line is written, on disk and on every destination.

## Rotation, retention and size cap

- A new file starts every day (UTC). The day's file stays in plain text; past days
  are compressed with gzip.
- **Retention**: how many days are kept, 14 by default (1 to 365). Older files are
  deleted.
- **Size cap**: the total size of the folder, 1024 MB by default (16 to 102400).
  When it is reached, the oldest days are deleted first.

Both are editable in Settings. If the disk writer cannot keep up, or the cap is
hit, some records are not written; the section shows how many.

## Export

Pick the last 1, 7, 14 or 30 days, or a custom number of days (never more than the
retention). The estimated size of the download is shown before the **Download
logs** button. The archive is a `.tar.gz` with the daily files of the period and
`audit.jsonl`, the audit log filtered to the same period.

The same download is available to scripts with a signed-in session:
`GET /api/logs/export?days=7`.

## Destinations

A destination forwards the logs to a log server. You can add several. Common
settings:

- **Name** and **Enabled**.
- **Minimum level**: `error`, `warn`, `info` (default) or `debug`. Records below it
  are not sent. The global `DUMBMONIT_LOG` filter applies first, so a destination
  can never receive more than the filter lets through.
- Each destination has its own in-memory queue of 10 000 records. Records are sent
  in batches of up to 500, or every second.

**Loss is accepted.** If a batch cannot be sent (server down, refused, timeout) it is
dropped and counted, not retried, and a full queue drops the oldest records. A busy or
unreachable log server therefore never slows DumbMonit down. The list shows, for each
destination, the records sent, the records lost, and the last error. The disk copy is
never affected.

**Send test** sends one record and shows whether the server accepted it.

Secrets (passwords, tokens, API keys) are stored encrypted and never shown again. When
you edit a destination, leave them empty to keep the saved ones. If you change the
address, type them again.

### Graylog (GELF) { #graylog-gelf }

Create a GELF input in Graylog, then:

| Field | Meaning |
| --- | --- |
| `transport` | `udp` (default), `tcp` or `http` |
| `address` | `host:port` for UDP and TCP (port 12201 by default); a URL for HTTP, for example `http://graylog:12201/gelf` |
| `tls` | TLS on TCP |
| `insecure_tls` | accept a certificate that does not verify |
| `username`, `password` (secret) | HTTP basic authentication |

UDP splits messages larger than 8192 bytes into GELF chunks. TCP sends one JSON
document per record, ended by a null byte.

Example: `transport` = `udp`, `address` = `graylog:12201`.

### VictoriaLogs { #victorialogs }

| Field | Meaning |
| --- | --- |
| `url` | base URL, for example `http://victorialogs:9428` |
| `stream_fields` | fields that identify a stream, `service,host` by default |
| `username`, `password` (secret) | basic authentication |
| `token` (secret) | bearer token, instead of a password |

Records are posted to
`/insert/jsonline?_msg_field=_msg&_time_field=_time&_stream_fields=service,host`.

Example: `url` = `http://victorialogs:9428`.

### Grafana Loki { #grafana-loki }

| Field | Meaning |
| --- | --- |
| `url` | base URL, for example `http://loki:3100` |
| `tenant` | optional, sent as `X-Scope-OrgID` |
| `username`, `password` (secret) | basic authentication (Grafana Cloud) |
| `token` (secret) | bearer token, instead of a password |

Records are posted to `/loki/api/v1/push` with the stream labels
`service_name="dumbmonit"`, `host` and `level`.

Example: `url` = `http://loki:3100`. In Grafana, query `{service_name="dumbmonit"}`.

### Elasticsearch / OpenSearch { #elasticsearch-opensearch }

| Field | Meaning |
| --- | --- |
| `url` | for example `https://es:9200` |
| `index` | index or data stream, `dumbmonit-logs` by default |
| `insecure_tls` | accept a certificate that does not verify |
| `username`, `password` (secret) | basic authentication |
| `api_key` (secret) | sent as `Authorization: ApiKey …`, instead of a password |

Records are sent to `/_bulk` as NDJSON with the `create` action, which also works with a
data stream.

Example: `url` = `https://es:9200`, `index` = `dumbmonit-logs`.

### Syslog { #syslog }

Syslog messages follow RFC 5424.

| Field | Meaning |
| --- | --- |
| `transport` | `udp` (default), `tcp` or `tls` |
| `address` | `host:port`; 514 by default for UDP and TCP, 6514 for TLS |
| `framing` | TCP and TLS only: `octet-counting` (default, RFC 6587 and 5425) or `newline` |
| `facility` | `local0` (default) to `local7`, `user` or `daemon` |
| `app_name` | `dumbmonit` by default |
| `insecure_tls` | accept a certificate that does not verify |
| `ca_cert` | optional PEM certificate authority to trust for TLS |

Example: `transport` = `tls`, `address` = `syslog.example.org:6514`.
