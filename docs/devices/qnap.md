# QNAP QTS / QuTS hero

!!! warning "Beta"

    This integration was built from the public references (the pages read by
    the Home Assistant `qnap` integration and the `python-qnapstats` library)
    and tested against recorded-style responses only, not against a real QNAP
    NAS yet. It only reads. Tell us what breaks.

A QNAP NAS through the CGI pages of its web interface: volumes that fill up,
disks whose SMART health is no longer good or that run too hot, processor
temperature, memory, and the QTS or QuTS hero updates QNAP offers.

QNAP publishes no documented REST API for the NAS status, so DumbMonit reads
the same XML pages as the web interface and as the reference clients. Each
minute it opens a session (`POST /cgi-bin/authLogin.cgi`, the password
base64-encoded in the request body, never in the URL), reads, and closes the
session (`/cgi-bin/authLogout.cgi`) even when a read failed: QTS limits the
number of open sessions. The session identifier travels in the URL of the
reads, so DumbMonit removes it from every error message.

One page is required — the system information (`manaRequest.cgi?subfunc=sysinfo`:
processor, memory, temperatures, uptime) — and three are optional, left out
without failing the rest when the account or the QTS version does not serve
them: volumes (`chartReq.cgi`), disks and their SMART health
(`disk_manage.cgi`), and the firmware update check (`sysRequest.cgi`).

The state of RAID groups (degraded, rebuilding) is not read: no reference
client documents it.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Web interface (HTTPS, default) |
| 8080 | TCP | Only with the HTTP protocol option |

## What it watches

All metrics are prefixed `dumbmonit_qnap_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Value 1. |
| `cpu_usage_percent` | | Processor usage. |
| `cpu_temperature_celsius`, `system_temperature_celsius` | | Processor and enclosure temperature. |
| `memory_total_bytes`, `memory_used_percent` | | Memory. |
| `uptime_seconds` | | Since the last boot. |
| `volumes` | | Number of volumes. |
| `volume_size_bytes`, `volume_free_bytes`, `volume_used_percent` | `volume` | Per volume. |
| `disks`, `disks_unhealthy` | | Disks present (empty bays are left out) and those whose SMART health is not good. |
| `disk_ok` | `disk`, `model` | 1 when the SMART health is good. |
| `disk_warning` | `disk`, `model` | 1 when it is a warning rather than a failure. |
| `disk_temperature_celsius` | `disk`, `model` | Disk temperature. |
| `firmware_update_available` | | 1 when QNAP offers a newer release. |
| `firmware_latest_info` | `version` | Value 1: that release. |

The [built-in rules](../alerting/rules.md#qnap-qts) that apply:

- **QNAP volume nearly full**: above 90 % for 30 minutes (Advisory).
- **QNAP disk health not good**: for 15 minutes (Advisory).
- **QNAP disk too hot**: above 55 °C for 15 minutes (Advisory).
- **QNAP processor too hot**: above 90 °C for 15 minutes (Advisory).
- **QNAP firmware update**: a release is offered (Info, reminded weekly).

## The monitor page

The NAS panel says what is wrong in a sentence, shows processor, memory,
temperature and uptime, then lists the volumes (the fullest first) and the
disks with their SMART health and temperature.

## Create a limited account for DumbMonit

1. Open Control Panel → Privilege → Users → Create → Create a User and create the account as follows, with a long random password. Tick no share and no application privilege.

    ```
    dumbmonit
    ```

2. Add the account to the administrators group: QTS serves the system status, volumes and disks to that group only.

3. Optional: under Control Panel → System → Security → Allow/Deny List, allow the web interface only from the DumbMonit server's address.

4. In DumbMonit, enter the NAS address (for example nas.lan) with that user name and password. Choose http and port 8080 only if HTTPS is off.

### Required permissions

- **administrators group** — The only group QTS lets read the system status
- **No share, no application privilege** — Nothing else is needed

!!! warning
    QTS has no read-only role for its system status, so the account sits in the administrators group: give it a unique password, and an account without 2-step verification. Over HTTP the password travels base64-encoded, not encrypted: prefer HTTPS.

### Notes

- DumbMonit never writes: every call is a read, and the account has no share
  or application privilege.
- An account with 2-step verification cannot log in for a monitor: create a
  dedicated one without it.

## Credentials

| Credential | Fields |
|---|---|
| User name and password | The account created above. |

Address: the host name or IP of the NAS, `host:port`, or a URL. Options:

- **Protocol** (https): `http` or `https`.
- **Port** (443): used if the address does not give a port; enter 8080 for
  HTTP, QTS does not use 80.
- **Accept an unverifiable certificate**: the NAS's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"QNAP refused the login": wrong user name or password, or the account is
disabled.

"The account uses 2-step verification": create a dedicated account without it.

"QNAP refused the session … the account may lack the right to read it": the
account is not in the administrators group.

No disk or volume: those pages are optional and a QTS version may serve them
differently; the rest is still read. Tell us your QTS version.
