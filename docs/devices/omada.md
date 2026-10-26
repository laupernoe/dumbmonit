# TP-Link Omada

A TP-Link Omada controller through its OpenAPI: access points, switches and
gateways that lost the controller, devices waiting to be adopted, devices
whose CPU or memory is saturated, firmware updates the controller offers, and
the number of clients per site.

!!! warning "Beta"

    This integration is written from TP-Link's Omada OpenAPI documentation
    and has not been tested on a real controller. Every call is a read
    (apart from the login that obtains the access token). Tell us what
    breaks.

DumbMonit uses an OpenAPI app created in the controller in Client mode with the
Viewer role. Its client ID and secret are exchanged for an access token valid
for two hours (`grant_type=client_credentials`); the token is kept until it is
about to expire, then requested again the same way. It is sent as
`Authorization: AccessToken=…`, never in the URL.

The controller ID (Omada ID) is the one shown under Open API. Left empty, it is
read from `/api/info`, which the controller publishes without authentication.

Every minute DumbMonit lists the sites (`/openapi/v1/{omadacId}/sites`), then for
each of them reads the devices (`…/sites/{siteId}/devices`, required) and the
number of clients (`…/clients`, optional: a narrower role may refuse it).

The network scan recognises a controller from `/api/info` on ports 8043 and 443
and offers to add it.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8043 | TCP | Controller HTTPS and OpenAPI (software controller, default) |
| 443 | TCP | Hardware controllers (OC200, OC300) and installs behind a reverse proxy |

Port 443 is only needed when the controller listens there instead of 8043.

## What it watches

All metrics are prefixed `dumbmonit_omada_`.

| Metric | Labels | Meaning |
|---|---|---|
| `controller_info` | `version`, `api_version`, `omadac_id` | Value 1. |
| `sites` | | Sites read. |
| `devices` | `status` | `online`, `offline` (disconnected, heartbeat missed or isolated) and `pending` (waiting for adoption). |
| `device_online`, `device_offline`, `device_pending` | `device`, `mac`, `model`, `device_type`, `site` | 1 or 0, per device. |
| `device_info` | same, plus `firmware`, `ip` | Value 1. |
| `device_cpu_usage_percent`, `device_memory_used_percent` | same | Only while the device is online: an offline device reports zeros. |
| `device_uptime_seconds`, `device_clients` | same | Only while the device is online. |
| `device_upgrade_available` | same | 1 when the controller offers a firmware update. |
| `site_clients` | `site` | Clients of the site. |
| `clients` | | Their sum. |

At most 200 devices are named per passage; the status counts are exact.

The [built-in rules](../alerting/rules.md#tp-link-omada) that apply:

- **Omada device offline**: disconnected for ten minutes (Advisory).
- **Omada device CPU high**: above 90 % for fifteen minutes (Advisory).
- **Omada device memory high**: above 90 % for fifteen minutes (Advisory).
- **Omada firmware update**: the controller offers an update (Info, reminded
  weekly).

## The monitor page

The Wi-Fi and switching panel says what is wrong in a sentence, shows the
devices online, the sites and the clients, then lists the devices that need a
look: disconnected, awaiting adoption, saturated or with a firmware update.

## Create a read-only OpenAPI client for DumbMonit

1. In the Omada controller, open Global View → Settings → Platform Integration → Open API and click Add New App.

2. Name the app as follows, choose the Client access mode and give it the Viewer role on the sites to read.

    ```
    dumbmonit
    ```

3. Click Apply: the controller shows the Omada ID, the Client ID and the Client Secret. Copy them: the secret is shown once.

4. In DumbMonit, enter the controller address (omada.lan), set the port if it is not 8043 (443 for a hardware controller), then paste the Client ID and the Client secret. Enter the Omada ID in the Controller ID option, or leave it empty to read it from the controller.

### Required permissions

- **Viewer role on the sites** — Reads sites, devices and client counts, changes nothing

!!! warning
    The Omada controller ships a self-signed certificate: tick "Accept an unverifiable certificate" unless you installed your own. The OpenAPI exists from controller version 5.9.

### Notes

- The menu names above follow controller 5.14; older versions place Open API
  under Settings → Platform Integration too.
- Alerts of the controller are not read by this version.

## Credentials

| Credential | Fields |
|---|---|
| OpenAPI client | The Client ID (user name field) and the Client Secret (password field) of the app. |

Address: the host name or IP of the controller, `host:port`, or a URL. Options:

- **Port** (8043): the HTTPS port of the controller.
- **Controller ID**: the Omada ID; empty to read it from the controller.
- **Site**: the name or ID of the only site to read; empty for every site.
- **Accept an unverifiable certificate**: the controller's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"Omada refused the OpenAPI client": the Client ID or secret is wrong, the app
was deleted, or it is not in Client mode, or the Controller ID does not belong
to this controller.

"… did not answer like an Omada controller": wrong address or port (8043 for
the software controller, 443 for a hardware one).

"Omada refused … (error …): … permission": give the app the Viewer role on the
sites to read.

"The Omada controller has no site …": the Site option matches neither a name
nor an ID.
