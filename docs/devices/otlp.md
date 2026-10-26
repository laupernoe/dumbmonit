# OpenTelemetry source

Receives metrics pushed with OTLP over HTTP, from an OpenTelemetry Collector or
directly from an instrumented application. Gauges, sums and histograms are stored
as series, graphed in the explorer and usable in threshold rules. Logs sent to
the same endpoint are stored in VictoriaLogs when it is enabled.

DumbMonit does not poll anything here: the data comes in.

## Endpoints

| Route | Purpose |
|---|---|
| `POST /api/otlp/v1/metrics` | Metrics, `application/x-protobuf` or `application/json`, plain or `Content-Encoding: gzip`. |
| `POST /api/otlp/v1/logs` | Logs, relayed unchanged to VictoriaLogs (`503` while it is disabled). |

OTLP over gRPC is not supported: the HTTP exporter of the OpenTelemetry Collector
(`otlphttp`) and of every SDK does the same job on the same port as the web
interface, with no extra listener.

## Setup

1. In **Settings, API & assistants**, create a token with the **write** scope.
   Restrict it to the network of your collector and give it an expiry date.
2. Add an **OpenTelemetry source** monitor and note its number: it ends the
   address of its page, `/devices/<number>`.
3. Point the exporter at DumbMonit with the token and the number.

OpenTelemetry Collector:

```yaml
receivers:
  otlp:
    protocols:
      http:
      grpc:

processors:
  # Counters and histograms must be cumulative; this turns delta ones into
  # cumulative ones. Not needed with the defaults of most SDKs.
  deltatocumulative:
  batch:

exporters:
  otlphttp/dumbmonit:
    endpoint: https://monit.example.com/api/otlp
    headers:
      Authorization: Bearer dmt_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
      X-DumbMonit-Target: "12"

service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [deltatocumulative, batch]
      exporters: [otlphttp/dumbmonit]
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlphttp/dumbmonit]
```

An SDK reads the same settings from its environment:

```
OTEL_EXPORTER_OTLP_ENDPOINT=https://monit.example.com/api/otlp
OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
OTEL_EXPORTER_OTLP_HEADERS=Authorization=Bearer dmt_xxx,X-DumbMonit-Target=12
```

The target can also be given as `?target=<number>` on the URL.

## How points become series

Names are written with underscores (`http.server.duration` becomes
`dumbmonit_http_server_duration`). Point attributes become labels, as do the
resource attributes `service.name`, `service.namespace` and
`service.instance.id` (`service_name`, …). The labels `target` and `host` are
always the monitor's own; the sender's are kept as `exported_target` and
`exported_host`.

| OTLP | Series |
|---|---|
| Gauge | Gauge. |
| Sum, monotonic, cumulative | Counter, named `…_total`. |
| Sum, not monotonic | Gauge. |
| Histogram, cumulative | Counters `…_bucket{le}` (cumulated), `…_sum`, `…_count`. |
| Summary | Counters `…_sum`, `…_count` and a gauge with `quantile`. |

Not stored, and counted as rejected in the response (`partialSuccess`): delta
temporality, exponential histograms, and points dated more than 30 days back or
one hour ahead. Traces are not accepted.

## Limits

* 8 MiB per request (32 MiB once decompressed), 50 000 series per request.
* 120 requests per minute per token (the limit of every API token); the response
  carries `RateLimit-*` headers and a `429` tells the exporter to retry.
* Labels are limited to 32 per series and 256 characters per value.

## Activity and alerts

The monitor publishes `dumbmonit_otlp_last_receive_seconds` (age of the last
accepted request), `dumbmonit_otlp_points_received_total` and
`dumbmonit_otlp_points_rejected_total`. They live in memory: after a restart of
the server nothing is published until the next request. Create a threshold rule
on `dumbmonit_otlp_last_receive_seconds` to be told when an exporter goes quiet,
and others on the metrics you send, for example
`dumbmonit_queue_depth > 1000`.

## Security

The `write` scope also opens the write half of the REST API: keep the token in
the secrets of the collector, restrict it by network and set an expiry date.
