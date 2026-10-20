# MikroTik RouterOS

Routers and switches running RouterOS 7: whether RouterOS and the RouterBOOT
firmware have fallen behind, CPU, memory and storage, temperatures, fans and
power supplies, and the state, traffic, errors and link losses of every
interface.

DumbMonit reads the REST API that RouterOS 7 serves under `/rest`, with a user
in a group that can only read. It sends `GET` requests and nothing else.

A MikroTik rarely breaks; it ages. A RouterOS release no longer the latest of
its channel, a RouterBOOT firmware left behind after an upgrade (it only
follows when asked, at the next reboot), a redundant power supply that died
without anyone noticing, a port that keeps logging errors or losing its link.
That is what this page watches, along with the load.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | REST API (HTTPS, default) |
| 80 | TCP | Only with the HTTP protocol option |

## What it watches

All metrics are prefixed `dumbmonit_mikrotik_`. Totals are counters: charts
and rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `info` | value 1 | `version`, `release` (`long-term`, `stable`…), `board`, `architecture`, `identity` |
| `uptime_seconds` | time since the last boot | |
| `cpu_load_percent`, `cpu_count` | CPU load, all cores | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | memory | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | the flash or disk RouterOS lives on | |
| `update_checked` | 1 when the router has checked for a newer RouterOS and got an answer | |
| `update_available` | 1 when the latest version of the channel differs from the installed one | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 1 on RouterBOARD hardware, 0 on a Cloud Hosted Router or an x86 machine | |
| `firmware_upgrade_pending` | 1 when RouterBOOT is older than the firmware bundled with RouterOS | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | each sensor the board has | `sensor` (`cpu-temperature`, `fan1-speed`…) |
| `health_ok` | 1 when a `psu*-state` or `fan*-state` sensor reads `ok`, 0 otherwise | `sensor` |
| `interface_running` | 1 when the interface has a link | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | traffic | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | errors and drops | `interface`, `type` |
| `interface_link_downs_total` | times the link was lost | `interface`, `type` |
| `interfaces_skipped` | interfaces left out beyond the limit set in the options | |
| `scrape_errors` | secondary reads that failed during the last probe | |

Only enabled interfaces are read. Dynamic ones (a PPPoE or L2TP client, whose
name changes at each connection) and the loopback are left out.

Sensors depend on the board: a hAP has a voltage and a temperature, a CCR
adds fans and power supplies, a Cloud Hosted Router has none. A sensor the
board does not have produces no series.

The newest RouterOS version is only known when the router itself checks for
updates: a read-only user is not allowed to start that check. The optional
step below schedules it on the router. Without it, `update_checked` stays at 0
and the update rule never fires.

The [built-in rules](../alerting/rules.md#mikrotik-routeros) that apply:
MikroTik CPU high, memory high, storage almost full, temperature high, power
supply or fan failed, interface accumulating errors, link flapping, RouterOS
update available and firmware upgrade pending, plus Monitor unreachable.

## The monitor page

The panel above the charts reads what the probe stored; opening the page
never connects to the router. It shows the RouterOS version and whether a
newer one is available, the RouterBOOT firmware, the uptime, CPU, memory and
storage, each sensor with its state, and a table of the interfaces with their
link, traffic, errors and link losses.

## Create a read-only user on the router

1. WinBox or WebFig → System → Users → Groups → Add: name the group as follows and tick only the policies listed below.

    ```
    dumbmonit-read
    ```

2. System → Users → Add: create the user dumbmonit in that group, with a long random password.

    ```
    dumbmonit
    ```

3. IP → Services: enable www-ssl and pick a certificate (create one under System → Certificates).

4. Optional, to hear about new RouterOS versions: System → Scheduler → Add, interval 1d, start time startup, with this On Event script.

    ```
    /system package update check-for-updates once
    ```

5. In DumbMonit, enter the router's address (192.168.88.1) with that user name and password.

### Required permissions

- **read** — Read the router's state
- **api** — Open the API
- **rest-api** — Open the REST API under /rest

!!! warning
    RouterOS 7.1 or later is required (RouterOS 6 has no REST API); with a self-signed certificate, tick "Accept an unverifiable certificate" in the options.

Vendor documentation: https://help.mikrotik.com/docs/spaces/ROS/pages/47579162/REST+API

### Why these permissions

RouterOS needs all three policies for a REST read: `rest-api` opens the API,
`api` and `read` allow the read itself. Without `rest-api` the router answers
401, without `api` "not allowed", without `read` "not enough permissions";
DumbMonit reports each case as a credential problem naming the missing policy.
With the three, a change, a reboot or an update check is refused. Never use the
default full-access account. To accept the account only from DumbMonit, set the
user's Allowed Address to the DumbMonit host.

RouterOS ships the HTTPS service disabled and without a certificate. Prefer the
terminal? These commands make a certificate on the router itself (then tick
"Accept an unverifiable certificate" in the options):

```
/certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
/certificate sign local-ca
/certificate add name=https-cert common-name=router.lan days-valid=825
/certificate sign https-cert ca=local-ca
/ip service set www-ssl certificate=https-cert disabled=no
```

The `www` service (plain HTTP) also answers the REST API, but it sends the
password in clear with every request. A read-only user may not start the
update check, so DumbMonit only reads its result.

The three policies were checked one by one on RouterOS 7.23.7, each with a
fresh user: without `rest-api` the router answers 401, without `api` it
answers "not allowed", without `read` "not enough permissions". DumbMonit
reports each case as a credential problem naming the missing policy, not as
a router down. With the three, a change, a reboot or an update check is
refused with "not enough permissions".

RouterOS may keep applying a group's former policies to a user for a while
after the group is edited. If you change the policies of an existing group and
the error persists, remove and re-create the user.

## Credentials

| Credential | Fields |
|---|---|
| Read-only user | The `dumbmonit` user and its password, sent as HTTP basic authentication with every request. The REST API has no session. |

Address: a host name or IP (`192.168.88.1`), `host:port`, or a full URL
(`https://router.lan:8443`). Options: protocol (HTTPS by default; HTTP uses the
`www` service), port (443 for HTTPS, 80 for HTTP), certificate check, request
timeout (10 s), whether to read the interfaces (on), and how many at most
(64).
