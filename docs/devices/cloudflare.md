# Cloudflare

The zones of a Cloudflare account through the Cloudflare API: zones that are
no longer active, tunnels that went down or degraded, certificates about to
expire, a rise of 5xx responses and unhealthy load balancer pools.

DumbMonit reads the API at `api.cloudflare.com/client/v4` once a minute with
a read-only API token sent as a bearer token. Only the list of zones is
required; every other part (tunnels, certificates, traffic, load balancers)
needs its own read permission and is skipped, without making the monitor
fail, when the token lacks it or the plan does not include the feature. The
calls made for each zone are limited to the first 50 active zones and sent
five at a time, far below Cloudflare's limit of 1,200 requests per five
minutes.

This is an Internet API: the address of the device is the API's, not a
machine's. DumbMonit needs outbound HTTPS to `api.cloudflare.com`; nothing
else is contacted.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the Cloudflare API documentation, not
    against a live account. It only reads. Tell us what breaks.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Cloudflare API (HTTPS, api.cloudflare.com) |

## What it watches

All metrics are prefixed `dumbmonit_cloudflare_`. At most 200 named items are
listed per family; the counts stay exact.

| Metric | Labels | Meaning |
|---|---|---|
| `zone_active` | `zone` | 1 when the zone status is `active`; 0 for `pending`, `moved`, `deactivated`… |
| `zone_paused` | `zone` | 1 when the zone is paused (Cloudflare only does DNS for it). |
| `zone_info` | `zone`, `status`, `plan` | Value 1. |
| `zones`, `zones_active`, `zones_paused` | | Counts. |
| `tunnel_healthy`, `tunnel_degraded`, `tunnel_down` | `tunnel` | 1 for the tunnel status (`healthy`, `degraded`, `down`). A tunnel that was never run (`inactive`) is none of them. |
| `tunnel_connections` | `tunnel` | Connections from the tunnel to Cloudflare. |
| `tunnels`, `tunnels_healthy`, `tunnels_degraded`, `tunnels_down` | | Counts. |
| `tunnels_readable` | | 0 when the tunnels cannot be read (see Troubleshooting). |
| `certificate_expiry_seconds` | `zone`, `host` | Time until the nearest expiry among the active certificates of the host. |
| `certificates_readable` | | 0 when the certificates cannot be read. |
| `zone_http_requests`, `zone_http_5xx` | `zone` | Requests and 5xx responses of the last complete hour and the current one (1 to 2 hours). |
| `zone_http_5xx_percent` | `zone` | Share of 5xx responses; only for zones with at least 100 requests in the window. |
| `analytics_readable` | | 0 when the traffic cannot be read; absent when the **Read traffic** option is off. |
| `lb_pool_healthy` | `pool` | 1 when an enabled load balancer pool is healthy. Disabled pools are not reported. |
| `lb_pool_enabled`, `lb_pool_origins`, `lb_pool_origins_healthy` | `pool` | Pool state and its enabled and healthy origins. |
| `load_balancer_enabled` | `zone`, `load_balancer` | 1 when the load balancer is enabled. |
| `lb_pools`, `load_balancers` | | Counts. |
| `lb_readable` | | 0 when pools or load balancers cannot be read. |

Zones that are not active or are paused get no certificate, traffic or load
balancer call. The traffic is read from the hourly analytics: it lags by up
to an hour, and a burst of errors takes up to two hours to leave the window.

The [built-in rules](../alerting/rules.md#cloudflare) that apply:

- **Cloudflare tunnel down**: a tunnel with no connection for 5 minutes
  (Critical, reminded every 6 h).
- **Cloudflare tunnel degraded**: a tunnel that lost part of its connections
  for 15 minutes (Advisory).
- **Cloudflare certificate expiring**: an active certificate expires in less
  than 14 days; Cloudflare renews them itself, so this means a renewal failed
  (Advisory, reminded daily).
- **Cloudflare zone not active**: a zone not `active` for an hour (Advisory).
- **Cloudflare 5xx rate high**: more than 5% of the responses are 5xx for 15
  minutes (Advisory).
- **Cloudflare load balancer pool unhealthy**: an enabled pool unhealthy for
  5 minutes (Advisory).

## The monitor page

The Cloudflare panel says what is wrong in a sentence, shows the zones, the
tunnels, the requests and the 5xx responses of the window, then lists the
zones, the tunnels, the certificates and the load balancer pools with their
state.

## Create a read-only API token for DumbMonit

1. In the Cloudflare dashboard, open My Profile > API Tokens > Create Token > Create Custom Token, and name it DumbMonit.

2. Add the read-only permissions listed below, and nothing else. The first one is required; the others switch on one part each.

3. Under Account Resources and Zone Resources, choose the account and the zones to monitor (All zones is fine).

4. Click Continue to summary > Create Token, then copy the token: it is shown once.

5. In DumbMonit, use this address and paste the token as API token. Leave Account ID empty unless the token covers several accounts.
   `api.cloudflare.com`

### Required permissions

- **Zone > Zone > Read** — Lists the zones and their status; required.
- **Zone > SSL and Certificates > Read** — Reads the expiry date of the certificates of each zone.
- **Zone > Analytics > Read** — Reads the requests and 5xx responses of each zone.
- **Zone > Load Balancers > Read** — Lists the load balancers of each zone.
- **Account > Cloudflare Tunnel > Read** — Reads the status of the tunnels.
- **Account > Account Settings > Read** — Finds the account of the tunnels; not needed if the Account ID option is filled.
- **Account > Load Balancing: Monitors And Pools > Read** — Reads the health of the load balancer pools.

!!! warning
    A part whose permission is missing (or that the plan does not include) is skipped and its readable metric is 0; it does not make the monitor fail. Only the zones the token can see are watched.

Vendor documentation: https://developers.cloudflare.com/fundamentals/api/get-started/create-token/

## Credentials

| Credential | Fields |
|---|---|
| API token | The token created above, sent as a bearer token. |

Address: `api.cloudflare.com`. Options:

- **Account ID**: the account of the tunnels and the load balancer pools.
  Empty: read from the token (up to 5 accounts), or from the accounts that own
  the zones when the token cannot list accounts.
- **Read traffic**: on by default; turn it off to skip the GraphQL analytics
  calls (one per 10 zones).
- **Timeout per request** (15 s).

## Troubleshooting

"Cloudflare refused the API token (401)": the token is wrong, expired or
revoked.

"Cloudflare refused /client/v4/zones (403)": the token lacks **Zone > Zone >
Read**, does not cover any zone, or the client IP address filtering of the
token excludes the DumbMonit server.

"Cloudflare has no resource at … check the account ID": the Account ID option
is not an account the token can reach.

`tunnels_readable`, `certificates_readable`, `analytics_readable` or
`lb_readable` is 0: the token lacks the permission of that part (see the list
above), the plan does not include it (load balancing is a paid add-on; the
analytics of the free plan are limited), or Cloudflare refused one of the
per-zone calls.
