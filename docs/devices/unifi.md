# UniFi Network

The UniFi network and what it manages: gateways, switches and access points
that are offline, isolated or waiting for adoption, firmware updates waiting,
the WAN links and the Internet connection as the gateway sees them, the number
of clients, and the alarms UniFi raised. It works with the UniFi Network
application of a UniFi OS console (Dream Machine, Cloud Gateway, Cloud Key)
and with a self-hosted UniFi Network Server.

DumbMonit reads UniFi with an API key or with a local account holding the
**View Only** role. It never restarts, adopts, upgrades or provisions a device.

The failure worth watching: **an access point or a switch drops off quietly**.
Clients roam to the next access point, the network keeps working, and the
coverage hole or the lost uplink is only noticed days later. The same goes for
a second WAN link that went down: nothing breaks until the first one fails too.

## Two ways in

| Credential | API used | What it reads |
|---|---|---|
| API key (UniFi OS console) | The official Integration API, at `/proxy/network/integration/v1` on a console and `/integration/v1` on a self-hosted server | Version, devices and their state, firmware updates, load, memory and uptime of each online device, number of clients |
| View Only account | The classic API (`/api/s/<site>/stat/*`), at `/proxy/network/api/…` on a console | All of the above, plus clients per device and by type, the WAN links, the WAN and Internet status, and alarms |

The WAN, Internet and alarm status only exist in the classic API. With a key,
DumbMonit also asks the classic API for the WAN and Internet status with the
same key; if the controller refuses it, those series are simply absent, without
an error.

Alarms are the entries of UniFi's "critical" system log over the last 24 hours
(UniFi Network 8 and later), or the unarchived alarms on older versions. That is
the only `POST` DumbMonit sends, and it only reads.

A session opened with the View Only account is kept from one probe to the next
and reopened when it expires: UniFi does not log a new login every minute.

## What it watches

All metrics are prefixed `dumbmonit_unifi_`. Device series carry `device` (its
name, or its MAC address when it has none), `mac`, `model` and `type`
(`gateway`, `switch`, `access_point`, `other`).

| Metric | What | Labels |
|---|---|---|
| `info` | value 1 | `version`, `api` (`integration` or `classic`) |
| `devices` | devices by state: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | adopted devices with a firmware update available | |
| `device_state` | 0 online, 1 offline, 2 pending adoption, 3 updating or provisioning, 4 adopting, 5 adoption failed, 6 isolated, 7 other | device labels |
| `device_up` | 1 online or updating, 0 offline, isolated or failed to adopt; absent for a device not adopted yet | device labels |
| `device_upgradable` | 1 when a firmware update is available | device labels, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | online devices only | device labels |
| `device_clients` | clients connected to the device (View Only account) | device labels |
| `wan_link_up` | 1 when a WAN port of the gateway is up; a disabled port has no series | device labels, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | the gateway's WAN and Internet status; absent without a UniFi gateway | |
| `internet_latency_seconds` | latency measured by the gateway | |
| `wan_availability_percent` | availability of each WAN over the last 24 hours | `wan` |
| `subsystem_status` | 0 ok, 1 warning, 2 error, 3 unknown, for `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | clients on the site | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (View Only account) | `type` |
| `alarms` | critical system log entries of the last 24 hours, or unarchived alarms | |
| `scrape_errors` | calls that failed on the last probe | |

The [built-in rules](../alerting/rules.md#unifi-network) that apply: UniFi
device offline, UniFi Internet down, UniFi WAN link down, UniFi gateway CPU
high, UniFi device waiting for adoption, UniFi firmware update available,
UniFi alarms, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries UniFi. It says what is wrong first (devices offline, Internet or a WAN
link down, alarms), then the devices by type with their state, firmware and
load, and the clients.

## How it was validated

Login, the View Only role, error answers (refused password, unknown site,
refused key, expired session) and the answers of a controller without devices
were checked against UniFi Network 10.6.106, self-hosted. No device could be
adopted there: the device list, the health of an equipped site and the
Integration API payloads are validated against the official documentation.

## Create a read-only access in UniFi Network

1. On a UniFi OS console (Dream Machine, Cloud Gateway, Cloud Key), you can create an API key: in UniFi Network, Settings → Control Plane → Integrations → Create API Key. Name it after DumbMonit and copy it now: it is shown only once. The key reads devices, their load and the number of clients through the official Integration API.

2. For the WAN, Internet and alarm status, and on a self-hosted UniFi Network Server, use a local account with the View Only role instead: Settings → Admins & Users → Create New. Name it as follows, give it a long random password, restrict it to local access, and set its UniFi Network role to View Only. View Only reads everything and can change nothing.

    ```
    dumbmonit
    ```

3. In DumbMonit, enter the address of the console or server, for example "unifi.lan" (port 443 on a console) or "unifi.lan:8443" for a self-hosted server, and paste the key or the account. If the network is not the default site, set the Site option to the short name shown in the address bar after /manage/ or /network/.

4. DumbMonit only reads. It never restarts, adopts, upgrades or provisions a device, and never acknowledges an alarm.

!!! warning
    Do not use the owner account or a UI.com cloud account: a leaked password would open every console linked to it. Keep two-factor authentication off for this local View Only account only, since a monitoring server cannot type a code. Consoles use a self-signed certificate by default: enable "Accept an unverifiable certificate" unless you installed your own.

## Credentials

| Credential | Fields |
|---|---|
| API key (UniFi OS console) | A key created in Settings → Control Plane → Integrations, sent as `X-API-KEY`. |
| View Only account | The user name and password of the local View Only account. |

Address: a host name or IP (`unifi.lan`), `host:port` (`unifi.lan:8443` for a
self-hosted server), or a full URL; an address copied from the browser, with
`/manage/…` or `/network/…`, is accepted. Options: site (`default`), port (443),
certificate check and request timeout (15 s).
