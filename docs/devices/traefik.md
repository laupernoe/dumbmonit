# Traefik

The reverse proxy: routers disabled by a configuration error, backend servers
failing their health check, certificates that a resolver never obtained or
that are not renewed, and the share of 5xx answers.

DumbMonit reads Traefik's API, which only reads: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (served by
Traefik 3.7) and `/api/version`. When Traefik's Prometheus metrics are served on the
same port, `/metrics` adds the request rate and the share of 5xx answers. No
method of the API changes anything, and DumbMonit sends none.

The failures worth watching are the silent ones. A router whose service or
middleware does not exist, or whose rule does not parse, is disabled: its sites
answer 404, and Traefik only says so in its log. A service whose servers all
fail their health check answers 503 "no available server". A router that asks
an ACME resolver for a certificate the resolver cannot obtain gets Traefik's
default self-signed certificate, which browsers refuse.

## What it watches

All metrics are prefixed `dumbmonit_traefik_`. Totals are counters: charts and
rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | routers of every protocol, and those in error or warning | |
| `services`, `services_errors`, `middlewares_errors` | the same for services and middlewares | |
| `router_status` | per HTTP router: 0 enabled, 1 warning, 2 disabled | `router`, `provider` |
| `router_error_info` | value 1, for a router that is not enabled: Traefik's first error | `router`, `error` |
| `service_status` | per service: 0 enabled, 1 warning, 2 disabled | `service` |
| `service_servers`, `service_servers_up` | servers of a load-balanced service, and those passing their health check | `service` |
| `server_up` | 1 when the server passes its health check (or has none) | `service`, `server` |
| `resolver_routers` | routers asking a certificate resolver for their certificate | `resolver` |
| `resolver_routers_uncovered` | those whose host names no certificate held by Traefik covers yet (needs `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificates Traefik holds, and the days left before each expires | `cert` |
| `requests_total`, `requests_5xx_total` | with metrics: requests of every entry point, and those answered 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | with metrics and router labels: the same per router | `router` |
| `config_reloads_total` | with metrics: configuration reloads | |

Traefik's own routers and services (`@internal`) are left out. Routers,
services and servers are described one by one up to 500 each; the totals always
cover everything.

The [built-in rules](../alerting/rules.md#reverse-proxies-and-domains) that
apply: Traefik router disabled, Traefik backend server down, Traefik service
without a server, Traefik certificate not obtained, Proxy certificate not
renewed, Reverse proxy answering 5xx, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries Traefik. It says what is wrong in a sentence (disabled routers and
Traefik's reason, services without a server, resolvers that obtained nothing,
certificates close to expiry, a high share of 5xx), then the routers,
services, request rate and 5xx share, then the servers up per service, the
certificates soonest to expire first, and the 5xx answers per router over 24
hours.

## Let DumbMonit read Traefik's API

1. Traefik's API only reads: routers, services and certificates, never a change. Enable it with the Prometheus metrics, which add the request rate and the share of 5xx answers: add these two arguments to Traefik's command (or the same keys to its static configuration) and restart it. Both are then served on the traefik entry point, port 8080.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. Port 8080 answers without a password: publish it only on an address the DumbMonit host reaches, filter it so that nothing else does, and check that it answers.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. To require a password instead, route /api and /metrics through an entry point of your own with a basicAuth middleware, as the documentation shows, and pick basicAuth user below. Create the user dumbmonit with htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. In DumbMonit, enter the Traefik host, for example "traefik.lan", with the port of that entry point if it is not 8080. Untick Read the metrics if /metrics is served elsewhere.

!!! warning
    A router disabled by a configuration error takes its sites down, and Traefik only says so in its log. The check that a certificate resolver obtained its certificates needs /api/certificates, which Traefik 3.7 serves; without it, certificate expiry comes from the metrics only.

### An API entry point with a password

Instead of `--api.insecure=true`, give the API and the metrics an entry point
of their own and put a basicAuth middleware in front of both. In the static
configuration:

```yaml
entryPoints:
  monitor:
    address: ":8082"
api: {}
metrics:
  prometheus:
    manualRouting: true
    addRoutersLabels: true
```

and in the dynamic configuration (file provider shown; Docker labels work the
same way), with the line printed by `htpasswd`:

```yaml
http:
  middlewares:
    dumbmonit-auth:
      basicAuth:
        users:
          - "dumbmonit:$2y$05$…"
  routers:
    monitor-api:
      entryPoints: [monitor]
      rule: "PathPrefix(`/api`)"
      service: api@internal
      middlewares: [dumbmonit-auth]
    monitor-metrics:
      entryPoints: [monitor]
      rule: "Path(`/metrics`)"
      service: prometheus@internal
      middlewares: [dumbmonit-auth]
```

Then enter `traefik.lan:8082` in DumbMonit with the user `dumbmonit`.
`addRoutersLabels` is what gives the 5xx answers per router; without it only
the totals per entry point are counted.

## Credentials

| Credential | Fields |
|---|---|
| No password | Nothing: the API port is filtered so that only DumbMonit reaches it. |
| basicAuth user | The user and password of the basicAuth middleware in front of `/api` and `/metrics`. |

Address: a host name or IP (`traefik.lan`), `host:port`, or a full URL.
Options: protocol (HTTP by default), API port (8080), Read the metrics (on),
certificate check and request timeout (10 s).

A service without health check reports every server as up: Traefik only marks
a server down when a `healthCheck` fails. Weighted and mirroring services have
no servers of their own and are not counted.
