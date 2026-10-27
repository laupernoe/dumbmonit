# UniFi Protect (beta)

Whether the cameras and their recorder are doing their job: cameras offline, cameras connected but not recording, NVR disks in failure, storage in use and recording switched off. Not to be confused with the UniFi Network monitor (access points, switches, gateway).

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | UniFi OS (HTTPS, default) |

The failure worth watching: **a camera that dropped off and nobody noticed**. A cable, a PoE port or a firmware update, and a camera is gone from the NVR; the footage you wanted is the footage that was never recorded. A camera that is connected but not recording is the same loss, one level deeper.

## What it watches

All metrics are prefixed `dumbmonit_unifiprotect_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `cameras_total`, `cameras_offline`, `cameras_not_recording` | adopted cameras; those not connected; those connected, set to record, but not recording |  |
| `camera_online`, `camera_recording` | 1 per camera while connected, while recording (at most 32) | `camera` |
| `storage_used_percent`, `storage_size_bytes` | NVR storage in use and its capacity |  |
| `disks_total`, `disks_unhealthy` | NVR disks, and those reported unhealthy |  |
| `recording_disabled` | 1 when recording is switched off on the NVR |  |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: Protect camera offline, Protect camera not recording, Protect NVR disk unhealthy, Protect recording disabled, plus Monitor unreachable.

## Create a local view-only user for Protect

1. In UniFi OS, open Admins & Users → Create New and create a local user named as follows, with no cloud account.

    ```
    dumbmonit
    ```

2. Give it access to the Protect application only, with the View Only role, and none to Network or other applications.

3. In DumbMonit, enter the console's address and that user name and password.

!!! warning
    Use a local account: UniFi OS refuses Ubiquiti cloud accounts here. The console's certificate is usually self-signed: turn on the certificate option.

## Credentials

| Credential | Fields |
|---|---|
| User name / password | A local UniFi OS user. DumbMonit logs in on `/api/auth/login`, keeps the session cookie between probes and logs in again when it expires. |

Address: the console's host name (`unvr.lan`) or `host:port`. Storage in use stays high on purpose: Protect overwrites its oldest footage, so no rule fires on a full disk. Options: protocol (HTTPS by default), port (443), certificate check and request timeout (15 s).
