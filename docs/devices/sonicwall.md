# SonicWall SonicOS

A SonicWall firewall (TZ, NSa, NSsp, NSv) through the SonicOS 7 API: the link
of every interface bound to a zone. Kept minimal on purpose.

!!! warning "Beta"

    This integration was built from what SonicWall publishes about the SonicOS
    API, not against a running appliance. The exact fields of the interface
    status answer are not documented, so DumbMonit reads it tolerantly: any
    object carrying a `name` and a status string (`status` or `link_status`)
    is an interface. Tunnel, HA and licence states are not read, because no
    API path for them could be confirmed. Tell us what breaks.

DumbMonit signs in with HTTP Basic (`Authorization: Basic`, never in the URL):
`POST /api/sonicos/auth` opens the session, then `GET
/api/sonicos/version` (optional) and `GET
/api/sonicos/reporting/interfaces/ipv4/status` are read, and `DELETE
/api/sonicos/auth` closes the session, even when a read failed. SonicOS allows
a single API session at a time, so closing it matters.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Management interface (HTTPS, default) |

## What it watches

All metrics are prefixed `dumbmonit_sonicwall_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version`, `model` | Value 1, when the version page answers. |
| `interface_link_up` | `interface`, `zone` | Link, read from the status string (`1 Gbps Full Duplex` is up, `No link` is down). |
| `interface_down` | `interface`, `zone` | 1 when the interface is bound to a zone and has no link. Ports without a zone, or unassigned, are left alone. |
| `interfaces`, `interfaces_down` | | Counts. |

The [built-in rule](../alerting/rules.md#sonicwall-sonicos) that applies:

- **SonicWall interface without link**: for five minutes (Advisory).

## The monitor page

The Firewall panel says which interfaces have no link and shows the SonicOS
version.

## Create a read-only user for the SonicOS API

1. Device → Settings → Administration → Audit/SonicOS API: make sure HTTP Basic Access (RFC-2617) is on, then click Accept.

2. Device → Users → Local Users & Groups → Add User: name it as follows, give it a long random password and add it to the group SonicWall Read-Only Admins.

    ```
    dumbmonit
    ```

3. Allow HTTPS management on the interface DumbMonit reaches the firewall through (Device → Network → Interfaces → Configure → Management).

4. In DumbMonit, enter the firewall address (sonicwall.lan) with that user name and password.

### Required permissions

- **Group SonicWall Read-Only Admins** — The account reads and changes nothing

!!! warning
    The management interface ships a self-signed certificate: tick "Accept an unverifiable certificate" unless you installed your own.

Vendor documentation: https://www.sonicwall.com/support/knowledge-base/introduction-to-sonicos-api/kA1VN0000000FGr0AM

### Notes

- SonicWall's own article states that the API is reserved to accounts with
  full privileges. If the read-only account is refused (403) while the
  password is right, SonicOS requires a higher privilege on your version:
  weigh that before granting it.
- HTTP Basic must be allowed under Audit/SonicOS API. The menu names above
  follow SonicOS 7; they differ slightly between 7.0 and 7.1.
- SonicOS 6.5 answers 406 on the status path: this integration targets
  SonicOS 7.
- Tunnel, HA and licence states are not read; use SNMP for them.

## Credentials

| Credential | Fields |
|---|---|
| User name and password | The account created above, sent as HTTP Basic. |

Address: the host name or IP of the firewall, `host:port`, or a URL. Options:

- **Port** (443): the HTTPS port of the management interface.
- **Accept an unverifiable certificate**: the firewall's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"SonicWall refused the account (401)": wrong user name or password, or HTTP
Basic is not allowed under Audit/SonicOS API.

"SonicWall refused … (403)": the SonicOS API is disabled, or the account
cannot use it.

No interface figure and a "no /api/sonicos/…" error: the firmware does not
offer the status path (SonicOS 6.5).
