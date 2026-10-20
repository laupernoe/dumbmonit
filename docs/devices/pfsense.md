# pfSense

A pfSense firewall through the community REST API package
(`pfSense-pkg-RESTAPI`, version 2): gateways that are down, lose packets or
answer slowly, enabled interfaces without link, enabled services that stopped,
and CPU, memory, disk and temperature.

DumbMonit sends an API key in the `X-API-Key` header. A key carries exactly
the privileges of the pfSense user it belongs to: the `dumbmonit` user below
holds only the `GET` privileges of the five pages DumbMonit reads, so the key
can read those pages and change nothing. Every minute DumbMonit reads, in
parallel, `/api/v2/status/system`, `/api/v2/status/gateways`,
`/api/v2/status/interfaces`, `/api/v2/status/services` and
`/api/v2/system/version`, and, if the privilege was given,
`/api/v2/system/restapi/version`.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the REST API package's published
    documentation and the source of its models (field names, units, the
    response envelope and its error identifiers), not against a running
    pfSense. Every call is a read. Tell us what breaks.

## What it watches

All metrics are prefixed `dumbmonit_pfsense_`.

| Metric | Labels | Meaning |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | As the dashboard shows them, in percent. The CPU figure is pfSense's own estimate from the load average. |
| `temperature_celsius` | | When pfSense can read a sensor. |
| `load_average` | `period`: `1m`, `5m`, `15m` | Load average. |
| `cpu_count` | | Processor cores. |
| `version_info` | `version` | Value 1: the pfSense version, `2.7.2-RELEASE`. |
| `gateway_up` | `gateway` | 0 when pfSense marks the gateway down, 1 otherwise. |
| `gateway_status_info` | `gateway`, `status` | Value 1: pfSense's own word (`none` when all is well, `down`, `loss`, `delay`). |
| `gateway_loss_percent` | `gateway` | Packet loss to the monitor address. |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | Round trip to the monitor address and its deviation; absent while the gateway is down. |
| `gateways`, `gateways_down` | | Counts. |
| `interface_enabled`, `interface_up` | `interface`, `descr` | Enabled in the configuration; link up. |
| `interface_down` | `interface`, `descr` | 1 when the interface is enabled and has no link. A disabled interface is never down. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | Counters. |
| `interfaces_down` | | Count. |
| `service_running`, `service_enabled` | `service`, `description` | As Status > Services shows them. |
| `service_stopped` | `service`, `description` | 1 when the service is enabled and not running. |
| `services_stopped` | | Count. |
| `restapi_update_available` | | With the `/api/v2/system/restapi/version` privilege: 1 when the REST API package has a newer release. |
| `restapi_version_info` | `version`, `latest` | Value 1: installed and latest release of the package. |

pfSense does not say through this API whether a pfSense update is available:
the only route that touches system updates (`/api/v2/system/update`) starts
one. DumbMonit reports the package's own updates only.

The [built-in rules](../alerting/rules.md#pfsense) that apply:

- **pfSense gateway down**: a gateway is marked down for three minutes (Warning).
- **pfSense gateway losing packets**: more than 10 % loss for ten minutes, on
  a gateway that is not already down (Advisory).
- **pfSense gateway slow**: round trip above 500 ms for fifteen minutes
  (Advisory). Raise it for a satellite link.
- **pfSense interface without link**: an enabled interface without link for
  five minutes (Advisory).
- **pfSense service stopped**: an enabled service not running for five
  minutes (Advisory).
- **pfSense disk nearly full**: more than 90 % for thirty minutes (Advisory).

## The monitor page

The Firewall panel says what is wrong in a sentence, shows CPU, memory, disk
and temperature, then lists every gateway with its latency and loss, the
interfaces without link, the stopped services, and an available update of the
REST API package.

## Create a read-only REST API key for DumbMonit

1. Install the pfSense-pkg-RESTAPI package (version 2) from Diagnostics → Command Prompt, with the file matching your pfSense version (2.7.2 below).

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. System → REST API → Settings: tick both BasicAuth and Key among the authentication methods.

3. System → User Manager → Users → Add: create the user dumbmonit with a long random password.

4. Edit the user and, under Assigned Privileges, add the privileges listed below.

5. Create the key as dumbmonit, copy the key value of the answer, then remove the POST privilege again.

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

6. In DumbMonit, enter the pfSense address (pfsense.lan) and paste the key as API key.

### Required permissions

- **/api/v2/status/system GET** — CPU, memory, disk, temperature
- **/api/v2/status/gateways GET** — Gateway state, delay and loss
- **/api/v2/status/interfaces GET** — Interface link and counters
- **/api/v2/status/services GET** — Stopped services
- **/api/v2/system/version GET** — pfSense version
- **/api/v2/system/restapi/version GET** — Optional: updates of the REST API package
- **/api/v2/auth/key POST** — Only while creating the key, then remove it

!!! warning
    The REST API package is a community project, not part of pfSense: reinstall it after each pfSense upgrade. pfSense ships a self-signed certificate: tick "Accept an unverifiable certificate" unless you replaced it.

Vendor documentation: https://pfrest.org/AUTHENTICATION_AND_AUTHORIZATION/

### Why these permissions and this command

A key carries exactly the privileges of the pfSense user it belongs to, so the
`dumbmonit` user holds only the `GET` privileges of the pages DumbMonit reads:
the key can read those pages and change nothing. The package has no menu to
create a key for another user, which is why the key is created with one `POST`
call made as `dumbmonit` (the extra privilege is removed right after). The
package is installed from a shell command because pfSense's Package Manager
only lists official packages.

The REST API does not say whether a pfSense update is available, only whether
the package itself has one.

## Credentials

| Credential | Fields |
|---|---|
| REST API key | The key created for `dumbmonit` in step 3, sent as `X-API-Key`. |

Address: the host name or IP of pfSense, `host:port`, or a URL. Options:

- **Protocol** (`https`) and **Port** (443): the webConfigurator's, set under
  System > Advanced > Admin Access.
- **Accept an unverifiable certificate**: pfSense's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"pfSense refused the API key (401)": the key is wrong, or Key is not ticked
among the authentication methods under System > REST API > Settings.

"pfSense accepted the API key but refused … (403)": the `dumbmonit` user lacks
the privilege of that page; the message names it.

"… not found: is the REST API package installed?": the package is missing,
typically after a pfSense upgrade, which removes it. Reinstall it as in step 1.
