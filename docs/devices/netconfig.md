# Network configuration backup

Keep every version of the configuration of a switch, a router or a firewall,
see what changed line by line, and be told when it changes, the way Oxidized
or RANCID do. DumbMonit reads the configuration over SSH (or through the
device's API for OPNsense and PAN-OS), keeps a new version only when the
configuration really changed, and stores each version compressed and
encrypted.

## Supported devices

Each device model is a **profile** shipped with DumbMonit. A profile says how
to recognise the prompt, how to turn paging off, how to enter privileged mode
and which commands to send. Nothing typed in the web interface is ever sent to
the device: only the commands of the chosen profile, as they are written.

| Device model | Profile | How | Commands |
|---|---|---|---|
| Cisco IOS / IOS-XE | `cisco_ios` | SSH terminal, `enable` when needed | `terminal length 0`, `terminal width 0`, `show running-config` |
| Cisco NX-OS | `cisco_nxos` | SSH terminal | `terminal length 0`, `terminal width 511`, `show running-config` |
| Aruba / HPE switches (ArubaOS-Switch, ProCurve, AOS-CX) | `aruba_hpe` | SSH terminal, `enable` when needed | `no page`, `show running-config` |
| MikroTik RouterOS | `mikrotik_routeros` | SSH command | `/export terse` |
| Juniper Junos | `juniper_junos` | SSH command | `show configuration \| display omit \| no-more` |
| Fortinet FortiGate (FortiOS) | `fortigate` | SSH terminal, paging answered | `show` |
| pfSense | `pfsense` | SSH command | `cat /cf/conf/config.xml` |
| OPNsense | `opnsense` | SSH command | `cat /conf/config.xml` |
| OPNsense | `opnsense_api` | HTTPS API | `GET /api/core/backup/download/this` |
| Ubiquiti EdgeOS / UniFi USG | `ubiquiti_edgeos` | SSH terminal | `terminal length 0`, `show configuration commands` |
| UniFi switch / access point | `unifi_device` | SSH command | `mca-ctrl -t dump-cfg` |
| Stormshield SNS | `stormshield` | SSH command | the files of `/usr/Firewall/ConfigFiles`, read one after the other |
| Palo Alto PAN-OS | `paloalto_api` | HTTPS XML API | `show config running` |

The profiles live in `profiles/netconfig/*.yaml` in the source tree. Besides
the commands, each one lists the **volatile lines** of its model (a "last
changed" timestamp, a clock period, values re-encrypted at every read) and the
**secrets** it masks (passwords, SNMP communities, pre-shared keys, private
keys).

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 22 | TCP | SSH (default port), for every profile but the API ones |
| 443 | TCP | Only with an API profile (OPNsense API, PAN-OS) |

A device that the server cannot reach can be backed up through a **relay
agent** on its network, like any other device: the agent connects, sends the
configuration back to the server and keeps nothing.

## Back up a device configuration

1. Choose the device model in "Device model": it decides the commands sent, and nothing else is ever run on the device.
2. Create a read-only account on the device that may log in over SSH and show the running configuration.
3. For OPNsense through its API, create an API key for a user allowed to download backups; for PAN-OS, generate the API key of a read-only role.
4. The first SSH connection remembers the device's host key: if it ever changes, backups stop and an alert asks you to check it.

What the account needs:

- A read-only account — Allowed to log in over SSH and to show the configuration

The SSH login has an optional **enable password**, for a profile that enters
privileged mode (Cisco IOS, Aruba) with another password than the login's;
empty, the login password is used. An account that lands directly in
privileged mode (privilege 15) needs none. Password and keyboard-interactive
authentication are supported; keys are not.

!!! warning
    Configurations hold secrets: they are stored encrypted, shown with their secrets masked, and only an administrator signed in can reveal or download them.

## How a backup works

Every **Back up every (hours)** (24 by default), the server reads the
configuration. A failed read is retried after 15 minutes. **Back up now** on
the device page reads it at once.

The configuration is then reduced to its comparable form: the volatile lines of
the profile are left out, and so are trailing spaces. If that form is the same
as the latest version's, nothing new is stored: the latest version is only
marked as seen again. Otherwise a new version is kept, with the number of lines
added and removed. The first version is a reference, never a change.

Limits: every step (connection, prompt, command) has **Timeout per step
(seconds)** (60 by default, from 10 to 110), and the whole read twice that. A
configuration larger than 4 MiB is refused rather than stored truncated.

## SSH host key

The first SSH connection trusts the key the device presents and remembers its
SHA-256 fingerprint, shown on the device page. From then on, a different key
**cuts the connection before any credential is sent**: no password ever leaves
for a device that could be an impostor. The device page shows both
fingerprints, the **SSH host key changed** alert fires, and backups stop until
an administrator compares the new fingerprint with the one the device shows
(`show ssh` / `show crypto key mypubkey`, the console of the appliance) and
accepts it. A replaced or reinstalled device changes its key; so does an
interception.

## Viewing versions and changes

The device page lists the versions. Picking one shows the line diff with the
previous version (or with any other one): each added line starts with `+` and
each removed line with `−`, so the change does not read by colour alone.

Secrets are **masked** in what is shown: each one becomes `<masked:xxxxxx>`,
where `xxxxxx` is a short fingerprint keyed with this instance's secret. A
changed password therefore shows as a changed line without being revealed, and
the fingerprint cannot be checked against a guess anywhere else. Masking uses
the rules of the profile, plus generic rules for anything called password,
secret or pre-shared key.

Who sees what:

| | Viewer | Operator | Administrator |
|---|---|---|---|
| Versions list, dates, lines changed | yes | yes | yes |
| Content and diffs, secrets masked | no | yes | yes |
| Secrets in clear, download | no | no | yes, signed in, recorded in the audit log |

API tokens never get a configuration in clear, nor a download.

## Storage and retention

Versions are kept in the DumbMonit database, compressed (gzip) and encrypted
with the instance key (AES-256-GCM, the same as device credentials). The
comparison fingerprint is an HMAC under a key derived from the instance
secret, never a plain hash of a text that holds secrets.

**Versions kept** (100 by default) and **Keep versions for (days)** (365 by
default; 0 for no age limit, by the date a version was last seen) decide what
is deleted. The latest version is always kept.

The DumbMonit backup carries the devices, their options and credentials and the
remembered host keys. The configuration versions themselves are included only
when you tick **Include network configuration versions** when exporting; it is
off by default.

## Alerts

| Rule | Fires when |
|---|---|
| Network configuration changed | The last backup found a configuration different from the previous version (`dumbmonit_netconfig_config_changed`). It resolves at the next backup without change. |
| SSH host key changed | The device presented another host key; backups are stopped (`dumbmonit_netconfig_host_key_changed`). |

A device that cannot be reached raises **Monitor unreachable**, a refused
account shows on the device page. See [Rules](../alerting/rules.md).

## Options

| Option | Default | Effect |
|---|---|---|
| Device model | `cisco_ios` | The profile: how to talk to the device and which commands to send. |
| Back up every (hours) | 24 | From 1 to 168. |
| Versions kept | 100 | From 1 to 1000. |
| Keep versions for (days) | 365 | From 0 (no age limit) to 3650. |
| Timeout per step (seconds) | 60 | From 10 to 110. |
| Accept an unverifiable certificate | off | API profiles only, for a device of your own behind a self-signed certificate. |
