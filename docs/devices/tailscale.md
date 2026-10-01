# Tailscale

The devices of a tailnet through the Tailscale API: servers that went
offline, node keys about to expire, devices waiting for approval, and clients
with an update available.

DumbMonit reads `GET /api/v2/tailnet/{tailnet}/devices` on
`api.tailscale.com`, once a minute. The recommended credential is an OAuth
client limited to the `devices:core:read` scope: DumbMonit exchanges its ID
and secret for a one-hour token (`/api/v2/oauth/token`) and keeps it until it
expires. It can list the devices and change nothing. An API access token
(`tskey-api-…`) works too, but it carries every right of the user who created
it and expires after 90 days at most.

This is an Internet API: the address of the device is the API's, not a
machine's. DumbMonit needs outbound HTTPS to `api.tailscale.com`; nothing in
the tailnet is contacted.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from Tailscale's API documentation and the
    device model of its official Go client, not against a live tailnet. It
    only reads. Tell us what breaks.

## What it watches

All metrics are prefixed `dumbmonit_tailscale_`. A device is named by the
first label of its MagicDNS name (`nas` for `nas.tail1234.ts.net`). Devices
shared into the tailnet from another one are counted, not described.

| Metric | Labels | Meaning |
|---|---|---|
| `device_online` | `device` | 1 while the device is connected to the coordination server. |
| `device_offline_seconds` | `device` | 0 while online, otherwise the time since it was last seen. |
| `device_watched` | `device` | 1 when the device must stay online: every tagged device by default, or those named in the option below. |
| `device_key_expiry_seconds` | `device` | Until the node key expires; absent when key expiry is disabled. |
| `device_authorized` | `device` | 0 while the device waits for approval. |
| `device_update_available` | `device` | 1 when Tailscale offers a newer client. |
| `device_info` | `device`, `os`, `client_version`, `tags` | Value 1. |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | Counts. |

Only watched devices raise an alert when they go offline: a phone or a laptop
that sleeps is not an outage. Tagged devices are, in practice, the servers,
subnet routers and exit nodes; list others in the **Devices to keep online**
option, or enter `all`.

The [built-in rules](../alerting/rules.md#tailscale) that apply:

- **Tailscale device offline**: a watched device disconnected for more than
  15 minutes (Advisory).
- **Tailscale node key expiring**: less than 14 days left (Advisory,
  reminded daily). When it expires, the device leaves the tailnet until
  someone signs it in again.
- **Tailscale device awaiting approval** (Advisory): approve it if it is
  expected, remove it otherwise.
- **Tailscale client update**: a watched device has a client update for six
  hours (Info, reminded weekly).

## The device page

The Tailnet panel says what is wrong in a sentence, shows the devices online,
the watched devices offline, the available updates and the shared devices,
then lists the watched devices and any device awaiting approval or whose key
expires soon.

## Create a read-only OAuth client for DumbMonit

1. In the Tailscale console (login.tailscale.com), open Settings > Trust credentials (formerly OAuth clients) and generate an OAuth client with only the devices:core:read scope. It lists the devices and can change nothing.

2. Copy the client ID and the client secret (tskey-client-…), shown once. In DumbMonit, enter api.tailscale.com as address and paste them as Client ID and Client secret. Leave the Tailnet option empty: the tailnet of the client is read.

!!! warning
    Only tagged devices (servers, subnet routers, exit nodes) are expected to stay online: a phone or a laptop that sleeps is not an outage. Name the devices to watch in the Devices to keep online option, or enter all.

## Credentials

| Credential | Fields |
|---|---|
| OAuth client | Client ID and client secret (`tskey-client-…`) of a client with the `devices:core:read` scope. |
| API access token | A `tskey-api-…` token, sent as a bearer token. |

Address: `api.tailscale.com`. Options:

- **Tailnet**: empty for the tailnet of the credentials, or the tailnet name
  shown in the Tailscale console.
- **Devices to keep online**: comma-separated device names or tags
  (`nas, tag:router`), or `all`. Empty: every tagged device.
- **Timeout per request** (15 s).

## Troubleshooting

"Tailscale refused the OAuth client": the ID or the secret is wrong, or the
client was deleted.

"Tailscale refused to list the devices (403)": the client lacks the
`devices:core:read` scope.

"Tailscale knows no tailnet …": leave the Tailnet option empty.
