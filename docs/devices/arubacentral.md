# Aruba Central

The Wi-Fi access points, switches and gateways of an HPE Aruba Networking
Central account, read through Central's cloud API: devices that went offline,
active critical alerts, devices in poor health, and the number of connected
clients.

DumbMonit reads the **new Central** API (the one served from the
`*.api.central.arubanetworks.com` gateways), once a minute. The recommended
credential is a personal API client created in HPE GreenLake: DumbMonit
exchanges its ID and secret for a two-hour access token
(`https://sso.common.cloud.hpe.com/as/token.oauth2`, `client_credentials`) and
keeps it until it expires. An access token pasted by hand works too, but
DumbMonit cannot renew it: it stops working after two hours.

This is an Internet API: the address of the monitor is the API gateway of your
region, not a machine on your network, and nothing on your sites is contacted.
Every call is a read.

!!! warning "Beta"

    Written from HPE's published API reference (`getDevicesV1`,
    `getAlertListV1`, `tenantClientHealthV1`, `tenantDeviceHealthV1`), not yet
    tested on a real Central account. Tell us what breaks. Credentials scoped
    to the HPE GreenLake platform (a different token endpoint) and classic
    Central (`/monitoring/v1/…`) are not supported.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Central API (HTTPS, the base URL of your region) |
| 443 | TCP | HPE GreenLake sign-in (HTTPS, sso.common.cloud.hpe.com) |

## What it reads

One required page and three optional ones that a client without the matching
right may refuse without failing the rest:

| Page | Use |
|---|---|
| `GET /network-monitoring/v1/devices` (required) | Every access point, switch and gateway: name, model, firmware, address, site, `ONLINE` or `OFFLINE`, uptime. Read a thousand devices at a time, up to ten pages. |
| `GET /network-notifications/v1/alerts` | The active alerts (the hundred most recent are classified by severity). |
| `GET /network-monitoring/v1/tenant-client-health` | Clients by type (wireless, wired) and health. |
| `GET /network-monitoring/v1/tenant-device-health` | Devices by type and health. |

## What it watches

All metrics are prefixed `dumbmonit_arubacentral_`.

| Metric | Labels | Meaning |
|---|---|---|
| `device_online` | `device`, `type`, `serial`, `site` | 1 when Central reports the device `ONLINE`. `type` is `access_point`, `switch` or `gateway`. |
| `device_uptime_seconds` | `device`, `type`, `serial` | Uptime, while online. |
| `device_info` | `device`, `type`, `serial`, `model`, `firmware`, `ip`, `site` | Value 1. |
| `devices`, `devices_offline` | | Counts of every device and of those offline. |
| `devices_by_type`, `devices_offline_by_type` | `type` | The same counts per type. |
| `alerts_active` | | Active alerts in Central. |
| `alerts_active_by_severity` | `severity` | Among the hundred most recent active alerts; `critical` is always published, at 0 when there is none. |
| `clients` | `type` | Connected clients per type. |
| `clients_by_health` | `type`, `health` | Clients per health group (`good`, `fair`, `poor`). |
| `devices_by_health` | `type`, `health` | Devices per health group. |
| `devices_poor_health` | | Devices in the `poor` group. |

The [built-in rules](../alerting/rules.md#aruba-central) that apply:

- **Aruba Central device offline**: Central sees a device offline for ten
  minutes (Advisory).
- **Aruba Central critical alert**: at least one active critical alert for ten
  minutes (Advisory).
- **Aruba Central devices in poor health**: at least one device in the `poor`
  health group for an hour (Info).

## Create a personal API client for DumbMonit

1. Sign in to HPE GreenLake (common.cloud.hpe.com) and open Manage Workspace → Personal API clients.

2. Click Create personal API client, name it as follows and choose your Central instance as the service.

    ```
    dumbmonit
    ```

3. Copy the client ID and the client secret: the secret is shown once, and HPE keeps no copy.

4. In Central, open the API gateway page (Menu → API Gateway → REST API) and note the base URL of your region.

5. In DumbMonit, enter that base URL as the address (for example us1.api.central.arubanetworks.com) and paste the client ID and the client secret.

### Required permissions

- **Read access to Central monitoring** — Devices, alerts and health overviews: DumbMonit never changes anything

!!! warning
    Beta: written from HPE's published API reference, not yet tested on a real Central account. It reads the new Central API with Central credentials; credentials scoped to the GreenLake platform and classic Central are not supported.

Vendor documentation: https://developer.arubanetworks.com/new-central/docs/generating-and-managing-access-tokens

### Base URLs by region

The address is the API gateway of the cluster that hosts your account, as HPE
lists them: `us1.api.central.arubanetworks.com` (US-1),
`us2.api…` (US-2), `us4.api…` (US-WEST-4), `us5.api…` (US-WEST-5),
`us6.api…` (US-East1), `de1.api…` (EU-1), `de2.api…` and `de3.api…`
(EU-Central2 and 3), `gb1.api…` (UK), `ca1.api…` (Canada), `in1.api…` (APAC-1),
`jp1.api…` (APAC-EAST1), `au1.api…` (APAC-SOUTH1), `ae1.api…` (UAE), all under
`central.arubanetworks.com`, and `cn1.api.central.arubanetworks.com.cn` (China).

## Credentials

| Credential | Fields |
|---|---|
| API client | Client ID and client secret of the personal API client. |
| Access token | An access token of your Central instance (valid two hours, not renewed). |

Options:

- **Sign-in host** (`sso.common.cloud.hpe.com`): the host that issues the
  tokens; change it only if HPE gives you another.
- **Timeout per request** (15 s).

## Troubleshooting

"Aruba Central refused the credentials (401)": the token expired, or the API
client was reset or deleted. With an API client DumbMonit asks for a new
token at the next minute.

"HPE GreenLake refused the API client": the client ID or secret is wrong.
The secret cannot be shown again; reset the client in HPE GreenLake.

"Aruba Central refused … (403)": the client has no access to Central
monitoring, or the base URL is not the one of your region. An optional page
refused this way is simply left out.

"did not answer like Aruba Central (no items)": the address is not a Central
API gateway.
