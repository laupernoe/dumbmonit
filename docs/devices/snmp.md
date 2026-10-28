# SNMP device

The most universal type: almost every piece of network hardware speaks SNMP.
Switches, routers, NAS, UPS and printers all qualify. SNMP v1, v2c and v3 are
supported.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 161 | UDP | SNMP (default) |

## What it watches

The collection profile is detected automatically from the device's
`sysObjectID` (and, for UPS, its `sysDescr`). Ten profiles ship with the
product; a device can match several, and the profiles include each other.

| Profile | Applies to | Metrics (prefix `dumbmonit_`) |
|---|---|---|
| System (SNMPv2-MIB) | Every device | `system_uptime_seconds`, `system_info` (name, description, location, contact as labels) |
| Network interfaces (IF-MIB) | Every device, as a fallback | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out`, `if_discards_in/out`, `if_admin_status`, `if_oper_status`, `if_speed_bps`, per interface |
| Host resources (HOST-RESOURCES-MIB) | Net-SNMP (Linux, BSD, macOS), the Windows SNMP agent, Synology, QNAP, Solaris | `host_uptime_seconds`, `host_processes`, `host_users`, `cpu_load_percent`, `storage_bytes_total/used` per filesystem, `memory_bytes_total/used`, `load_average` (1, 5 and 15 minutes, Net-SNMP only) |
| UPS (UPS-MIB) | APC, Eaton, MGE, CyberPower, Riello, Socomec, Tripp Lite, Vertiv, Salicru, and any agent announcing UPS-MIB | `ups_battery_status`, `ups_battery_seconds_on`, `ups_battery_minutes_remaining`, `ups_battery_charge_percent`, battery volts/amperes/temperature, input volts/hertz/line bads, `ups_output_source`, output volts/hertz/watts, `ups_output_load_percent`, `ups_alarms_present` |
| Printer (PRINTER-MIB) | HP, Kyocera, Epson, Canon, Brother, Lexmark, Ricoh, Xerox, Samsung, OKI, Konica Minolta | `printer_supply_level/max` per supply, `printer_input_level/max` per tray, `printer_pages_printed`, `printer_status` |
| Ethernet switch (EtherLike-MIB, LLDP) | Chosen by hand for any switch; included by the vendor switch profiles | Everything from IF-MIB, plus per port `ether_fcs_errors`, `ether_alignment_errors`, `ether_symbol_errors`, `ether_late_collisions`, `ether_excessive_collisions`, `ether_mac_transmit_errors`, `ether_mac_receive_errors`, `ether_duplex_status` (1 unknown, 2 half, 3 full), `if_broadcast_packets_in`, `if_multicast_packets_in` (64-bit), and `lldp_neighbor_info` (who is plugged where: `remote_sysname`, `remote_port`, `remote_port_desc`; the local port number is the second part of the `index` label). For the switch panel of the device page: `switch_port_status` (every physical Ethernet port, disabled and never-used ones included; `admin` label 1 enabled, 2 disabled), `port_pvid` (native VLAN, Q-BRIDGE-MIB), `poe_port_status` (POWER-ETHERNET-MIB: 1 disabled, 2 searching, 3 delivering, 4 fault, 6 other fault; `class` label), `poe_power_budget_watts`, `poe_power_consumed_watts` and `poe_main_status` |
| Zyxel switch | Every Zyxel `sysObjectID` (`1.3.6.1.4.1.890`) | Everything from Ethernet switch, plus `cpu_load_percent` (one-minute average), `zyxel_cpu_5min_percent`, `memory_usage_percent`, `zyxel_firmware_info` (firmware and model as labels), and on switches that publish the hardware monitor, `zyxel_temperature_celsius` and `zyxel_temperature_high_threshold_celsius` per sensor, `zyxel_fan_rpm` and `zyxel_fan_low_threshold_rpm` per fan |
| WatchGuard Firebox | Every WatchGuard `sysObjectID` (`1.3.6.1.4.1.3097`) | Everything from IF-MIB, plus `cpu_load_percent` (one-minute average), `watchguard_cpu_5min_percent`, `watchguard_cpu_15min_percent`, `watchguard_active_connections`, `watchguard_bytes_in/out`, `watchguard_packets_in/out`, `watchguard_stream_requests_dropped`, `watchguard_ipsec_tunnels`, `watchguard_ipsec_tunnel_kbytes_in/out` per peer address, and on a cluster `watchguard_ha_status`, `watchguard_ha_peer_status`, `watchguard_ha_error`, `watchguard_ha_peer_error` |
| Stormshield SNS | Every Stormshield `sysObjectID` (`1.3.6.1.4.1.11256`) | Everything from IF-MIB, plus `stormshield_ha_nodes`, `stormshield_ha_dead_nodes`, `stormshield_ha_active_nodes`, `stormshield_ha_links`, `stormshield_ha_faulty_links`, `stormshield_ha_sync_status`, `stormshield_ha_node_quality` per node serial, `stormshield_cpu_temperature_celsius` per processor and `stormshield_fan_rpm` per fan |
| HPE / Aruba switch | `1.3.6.1.4.1.11.2.3.7` (HPE ProCurve family) and `1.3.6.1.4.1.47196.4` (Aruba CX) | Everything from Ethernet switch, nothing proprietary |

Counters (interface octets, pages printed) are stored raw: use `rate()` in a
query to get a throughput. A device reboot shows as a counter reset, never as a
fake spike.

The [built-in rules](../alerting/rules.md) that apply to SNMP devices: Device
unreachable, High CPU, Disk almost full, Filesystem almost full (forecast), UPS
on battery, UPS battery low, Port accumulating errors, Port flapping, Unusual
CPU (baseline).

### Zyxel switches

The standard part of the Zyxel profile (interfaces, Ethernet errors, LLDP) is
common to every Zyxel switch. The CPU, memory and firmware figures come from
the private branch that the current managed switches share under
`1.3.6.1.4.1.890.1.15` (the XS1930 and GS1900 among them); older models and
Zyxel firewalls lay theirs out differently, and those series simply stay empty
there. The profile was checked against a real XS1930-10; its temperature and
fan series follow Zyxel's published hardware-monitor MIB but have not yet been
seen on real hardware.

`dot3StatsFrameTooLongs` is deliberately not read: on the XS1930 it counts
billions of perfectly healthy jumbo frames, and a series that always climbs
on a healthy network only teaches people to ignore it.

### WatchGuard Firebox

The profile reads the three MIBs WatchGuard publishes under its enterprise
number: system statistics (CPU, connections, traffic), IPsec tunnels and
high availability. The CPU figures are reported in hundredths of a percent
and are scaled to a percentage. The OIDs come from WatchGuard's MIBs and
documentation; they have not yet been seen on a real Firebox, so a missing
series means the model or Fireware version does not publish it. The
high-availability values are the MIB's raw codes. SNMP has to be enabled
in the Firebox's configuration; the 64-bit counters need SNMPv2c or v3.

`watchguard_ipsec_tunnels` counts the security associations established at
this moment, site-to-site tunnels and mobile users together: it is shown, but
no built-in rule alerts on it. The generic rules (device unreachable, high
CPU, port errors, port flapping) apply. No REST API is used: this integration is SNMP only.

### Stormshield SNS

The profile reads STORMSHIELD-HA-MIB (cluster nodes, dead nodes, faulty HA
links, node quality) and STORMSHIELD-SYSTEM-MONITOR-MIB (processor
temperature, fan speed) under `1.3.6.1.4.1.11256`. A firewall outside a
cluster reports a single node; a virtual appliance has no fan or sensor, and
those series stay empty. The OIDs come from Stormshield's MIBs; they have not
yet been seen on a real appliance. The health columns of
STORMSHIELD-HEALTH-MONITOR-MIB are text whose values the MIB does not list, so
they are not read.

Two [built-in rules](../alerting/rules.md#stormshield-sns) apply on top of the
generic ones: a dead HA node and a faulty HA link. Enable the SNMP agent
(Configuration → Notifications → SNMP agent) and add a filter rule that lets
only the DumbMonit server query it, as Stormshield recommends. No web API is
used: this integration is SNMP only.

### HPE / Aruba switches and Aruba Instant On

Aruba Instant On is a cloud-managed range. HPE publishes no public API for
its portal that this integration could rely on, and no SNMP agent is
documented for its access points, so DumbMonit cannot read the access points
nor the Instant On portal. The Instant On 1930 switches do offer
SNMP; when they announce an HPE `sysObjectID` of the ProCurve family
(`1.3.6.1.4.1.11.2.3.7`), the HPE / Aruba switch profile applies the standard
switch series (interfaces, Ethernet errors, LLDP neighbours) without any
manual choice. If a switch announces another identifier, choose the Ethernet
switch profile by hand: the result is the same. For Aruba access points,
gateways and switches managed from the cloud, use
[Aruba Central](arubacentral.md).

### Server management controllers (BMC)

A BMC's own SNMP agent describes the BMC, not the server: its small ARM
Linux, its memory, its network links. A Supermicro BMC, for instance, answers
as Net-SNMP and gets the Host resources profile, which reads everything that
agent publishes about itself. The server's fans, temperatures, power supplies
and drives are read over [Redfish](redfish.md) instead.

## What to prepare on the device

1. Open the device's web interface and find its SNMP settings, often under Network, Services or Administration.

2. Enable SNMP v2c and note the read-only community (public on many devices).

3. If the device filters by address, allow this server's address.

4. In DumbMonit, enter the address and the community: the rest is detected automatically.

!!! warning
    A community is not encrypted on the network: on a shared network, prefer SNMP v3, which authenticates and encrypts.

## Credentials

| Credential | Fields |
|---|---|
| SNMP v1 / v2c (community) | The read-only community. |
| SNMP v3 | User name; optional authentication (MD5, SHA-1, SHA-224, SHA-256, SHA-384 or SHA-512 and a passphrase); optional privacy (DES, AES-128, AES-192 or AES-256 and a passphrase); optional context name. |

Address: an IP or host name, for example `192.168.1.10`. The default port is
161; write `host:port` to use another one.

## Options

This type reads no options. Free tags are still available and are copied on
every series as `tag_<key>`.

## Common errors

| Symptom | Likely cause |
|---|---|
| *Unreachable* right after adding | Wrong community, SNMP not enabled, or the device only accepts requests from allowed addresses. The device never answers a wrong community: it looks exactly like a device that is off. |
| Only `system_*` and `if_*` series | The `sysObjectID` matched no specific profile. IF-MIB and System apply to everything; HOST-RESOURCES, UPS, PRINTER and Zyxel need a known vendor OID. On a switch of another brand, choose the Ethernet switch profile by hand. |
| No interface traffic | The device does not expose the 64-bit `ifXTable` counters. |
| Empty scan | The scan tries one community on every address; devices with another community or SNMP v3 only do not answer. |

Use **Probe now** on the monitor page: it reports the number of samples and the
series names produced by one probe.
