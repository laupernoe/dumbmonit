# FortiGate

A Fortinet FortiGate through the FortiOS REST API: site-to-site IPsec tunnels
that are down, enabled interfaces that lost their link, an HA cluster out of
sync or short of a member, memory approaching conserve mode, FortiGuard and
FortiCare licences expiring, and patch releases offered for the running
FortiOS branch.

DumbMonit uses a REST API user whose access profile only reads, and which
only accepts calls from the DumbMonit server's address (trusted host). Its
token is sent as `Authorization: Bearer`, never in the URL: FortiOS 7.4.5 and
7.6.1 refuse it there anyway.

Every minute DumbMonit reads three required pages — `/api/v2/monitor/system/status`,
`/api/v2/monitor/system/resource/usage` and `/api/v2/monitor/system/interface/select`
— and six optional ones that a narrower profile, or a FortiGate without a
cluster or tunnels, may refuse or leave empty without failing the rest: the
administrative state of interfaces (`/api/v2/cmdb/system/interface`), HA
statistics and checksums, IPsec tunnels, licence status and available
firmware.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from Fortinet's FortiOS documentation and the
    response shapes recorded by the community Prometheus exporter for
    FortiGate, not against a running FortiGate. Every call is a read. The
    licence and firmware pages are the least documented: tell us what
    breaks.

## What it watches

All metrics are prefixed `dumbmonit_fortigate_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Value 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Current usage. |
| `sessions`, `session_setup_rate` | | Active sessions, new sessions per second. |
| `log_disk_ok` | | 0 when the log disk needs formatting. |
| `interface_link_up` | `interface`, `alias` | Physical link. |
| `interface_enabled` | `interface`, `alias` | Administrative state; every interface counts as enabled when the profile cannot read the configuration. |
| `interface_down` | `interface`, `alias` | 1 when the interface is enabled, has an address and has no link. Free ports without an address are left alone. |
| `interface_speed_mbps` | `interface`, `alias` | While the link is up. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Counters. |
| `interfaces_down` | | Count. |
| `ha_members` | | Members in the cluster; 0 for a standalone FortiGate. |
| `ha_in_sync` | | With two members or more: 1 when every member has the same configuration checksum. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | Per member. |
| `ipsec_tunnel_up` | `tunnel` | 1 when at least one phase 2 of the tunnel is up. Dial-up tunnels (remote access) are left out. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | Per phase 2 selector. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Counters. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Value 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Counts. |
| `license_status_info` | `license`, `status` | Value 1, for every licence the FortiGate holds (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…); `no_license` entries are left out. |
| `license_expired` | `license` | 1 when the status is `expired`. |
| `license_expiry_seconds` | `license` | Until expiry; negative once expired. |
| `firmware_update_available` | | 1 when FortiGuard offers a newer patch release of the running branch (7.4.4 → 7.4.5). A new branch (7.6) is a project, not an alert. |
| `firmware_latest_info` | `version` | Value 1: that patch release. |

The [built-in rules](../alerting/rules.md#fortigate) that apply:

- **FortiGate IPsec tunnel down**: no phase 2 up for five minutes (Warning).
- **FortiGate interface without link**: for five minutes (Advisory).
- **FortiGate HA out of sync**: for fifteen minutes (Advisory).
- **FortiGate HA member lost**: fewer members than at any time in the last
  24 hours (Warning). The expected count is published nowhere, so a member
  removed for good stops alerting the next day.
- **FortiGate memory near conserve mode**: above 85 % for ten minutes
  (Advisory). At 88 % by default FortiOS enters conserve mode and stops
  inspecting new sessions.
- **FortiGate CPU high**: above 90 % for fifteen minutes (Advisory).
- **FortiGate licence expiring**: less than 30 days left, or expired
  (Advisory, reminded daily).
- **FortiGate firmware update**: a patch release is offered (Info, reminded
  weekly).

## The device page

The Firewall panel says what is wrong in a sentence, shows CPU, memory,
sessions and the HA state, then lists the IPsec tunnels (down first), the
interfaces without link, the licences expiring within 60 days or expired, and
an available patch release.

## Create a read-only REST API user for DumbMonit

1. From the FortiGate CLI, create an access profile that can only read the system, network and VPN state, and nothing else.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Create the REST API user dumbmonit with that profile, trusting only the address of the DumbMonit server (replace 192.0.2.10), then generate its token: it is printed once.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. In DumbMonit, enter the address of the FortiGate, for example "fw.lan" (add the port if the administration interface does not listen on 443), and paste the token as API token. FortiGate ships a self-signed certificate: tick Accept an unverifiable certificate unless you installed your own.

!!! warning
    An interface counts as down only when it is enabled, has an address and has no link: free ports are left alone. Firmware updates are those FortiGuard offers the FortiGate itself: without access to FortiGuard, none is reported.

## Credentials

| Credential | Fields |
|---|---|
| API token | The token printed by `execute api-user generate-key dumbmonit`, sent as `Authorization: Bearer`. |

Address: the host name or IP of the FortiGate, `host:port`, or a URL. Options:

- **Port** (443): the HTTPS port of the administration interface (`set
  admin-sport` under `config system global`).
- **VDOM**: the virtual domain to read; empty for the one the REST API user
  belongs to.
- **Accept an unverifiable certificate**: the FortiGate's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"FortiGate refused the API token (401)": the token is wrong or was
regenerated. Generate a new one with `execute api-user generate-key
dumbmonit`.

"FortiGate refused … (403)": the DumbMonit address is not among the trusted
hosts of the user, or its profile cannot read that page. An optional page
refused this way is simply left out.

No IPsec, licence or firmware figure: the profile cannot read them, or the
FortiGate has no tunnel; the rest is still read.
