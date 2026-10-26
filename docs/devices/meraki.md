# Cisco Meraki

A Cisco Meraki organization through the Dashboard API: devices that are
offline, Internet uplinks of the MX routers that failed (and the failover to
the other uplink), a license out of compliance or about to expire, and the
number of clients per network.

!!! warning "Beta"

    This integration is written from Cisco's Dashboard API documentation and
    has not been tested on real Meraki equipment. Every call is a read. Tell
    us what breaks.

DumbMonit uses the API key of an administrator who has read-only access to the
organization. The key is sent as `Authorization: Bearer`, never in the URL.
The monitor's address is the API itself (`api.meraki.com`), not a device, so
DumbMonit does not need to reach your sites.

Every minute DumbMonit reads two required pages — `/api/v1/organizations` and
`/api/v1/organizations/{id}/devices/statuses` — and four optional ones that a
narrower key, or another licensing model, may refuse without failing the rest:
network names, `uplinks/statuses`, the license (`licenses/overview`, or the
per-device `licenses` list for organizations on per-device licensing) and the
clients seen in the last hour for each network.

Two behaviours of the API are handled for you. The dashboard answers some
requests with a redirect (307) to the server that hosts the organization
(`n123.meraki.com`): DumbMonit follows it, but only inside the Meraki domain,
so the key never goes anywhere else. And the API allows 10 requests per second
per organization: DumbMonit reads one page after the other and waits for the
`Retry-After` of a 429 answer before trying again.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Meraki Dashboard API (HTTPS, api.meraki.com) |

## What it watches

All metrics are prefixed `dumbmonit_meraki_`.

| Metric | Labels | Meaning |
|---|---|---|
| `organization_info` | `organization`, `organization_id` | Value 1. |
| `devices` | `status` | Devices by status: `online`, `alerting`, `offline`, `dormant`. Exact counts. |
| `device_online` | `device`, `serial`, `model`, `product_type`, `network` | 1 when the device is `online` or `alerting`. Dormant devices have no per-device series. |
| `device_offline` | same | 1 when the status is `offline`. |
| `device_alerting` | same | 1 when the status is `alerting` (reachable, with a problem the dashboard flags). |
| `device_last_report_age_seconds` | same | Since the device last reported to the cloud. |
| `uplink_up` | `device`, `serial`, `interface`, `network` | 1 when the uplink is `active` or `ready`. |
| `uplink_failed` | same | 1 when the status is `failed`: a configured uplink that lost Internet. `not connected` (a socket without a cable, an empty cellular slot) is not a failure. |
| `uplink_failover` | `device`, `serial`, `network` | 1 when the `wan1` uplink failed and another one carries the traffic. |
| `uplinks_failed` | | Count. |
| `license_ok` | | 1 when the license overview says `OK`. Co-termination licensing only. |
| `license_status_info` | `status` | Value 1. |
| `license_expiry_seconds` | | Until the license expires; for per-device licensing, until the nearest expiry of the active licenses. |
| `network_clients` | `network`, `network_id` | Clients seen in the last hour (up to 1000 per network, 25 networks). |
| `clients` | | Their sum. |

At most 200 devices and 200 uplink entries are named per organization; the
counts stay exact up to the 1000 devices one page returns.

The [built-in rules](../alerting/rules.md#cisco-meraki) that apply:

- **Meraki device offline**: offline for ten minutes (Advisory). Dormant
  devices are never reported.
- **Meraki uplink failed**: five minutes (Advisory).
- **Meraki license problem**: the license is not in compliance for an hour
  (Advisory, reminded daily).
- **Meraki license expiring**: less than 30 days left (Advisory, reminded
  daily).

## The monitor page

The Cloud network panel says what is wrong in a sentence, shows the devices
online, alerting and dormant, the clients and the license, then lists the
devices offline or alerting and the uplinks that failed (with a note when the
router already runs on its backup uplink).

## Create a read-only API key for DumbMonit

1. In the Meraki dashboard, open Organization → Settings and tick Enable access to the Cisco Meraki Dashboard API.

2. Open Organization → Administrators → Add admin and create an administrator dedicated to DumbMonit, with Organization access set to Read-only.

    ```
    dumbmonit@example.com
    ```

3. Sign in as that administrator, open My profile → API access and click Generate new API key. Copy it: the dashboard shows it only once.

4. In DumbMonit, enter api.meraki.com as the address and paste the key as API key. Leave Organization empty to read the first organization the key sees.

### Required permissions

- **Organization access: Read-only** — Lists devices, uplinks, licensing and clients, changes nothing

!!! warning
    A dormant device (not seen for a long time) is counted but never reported as down, and a WAN socket without a cable is not a failed uplink: only an uplink that lost its Internet access is.

Vendor documentation: https://developer.cisco.com/meraki/api-v1/

### Notes

- A key is tied to the administrator who generated it: if that account is
  removed or loses access, the monitor shows an authentication error.
- Other regions and the US government cloud use their own API host
  (`api.meraki.cn`, `api.gov-meraki.com`): enter it as the address.

## Credentials

| Credential | Fields |
|---|---|
| API key | The key generated under My profile → API access, sent as `Authorization: Bearer`. |

Address: `api.meraki.com` (or the API host of your region). Options:

- **Organization**: the ID or the name of the organization to read; empty for
  the first one the key sees.
- **Networks**: comma-separated network names or IDs to watch; empty for every
  network. It restricts devices, uplinks and clients.
- **Timeout per request** (15 s).

## Troubleshooting

"Meraki refused the API key (401)": the key is wrong, was regenerated, or its
administrator was removed.

"Meraki refused … (403)": API access is not enabled for the organization
(Organization → Settings), or the administrator has no access to it.

"The API key does not see an organization …": the Organization option matches
neither an ID nor a name the key can see.

No license figure: the key cannot read it, or the organization uses
subscription licensing, which this version does not read.
