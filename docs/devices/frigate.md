# Frigate NVR (beta)

Whether Frigate still sees: cameras that stopped delivering frames, detector inference speed and hangs (Coral, GPU, CPU), frame rates, the disk holding the recordings and updates.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 5000 | TCP | API (HTTP, default, no login) |
| 8971 | TCP | Only with the authenticated port behind a bearer-token proxy |

The failure worth watching: **a camera that stopped sending frames**. Frigate shows it only in its interface; its recordings and detections simply stop. `camera_fps` at zero is the most reliable sign. A detector stuck on one inference, or a full recording disk, are the other two.

## What it watches

All metrics are prefixed `dumbmonit_frigate_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `update_available` | 1 when Frigate knows a newer release | `latest_version` |
| `uptime_seconds`, `detection_fps` | time since Frigate started; detections per second, all cameras |  |
| `cameras_total`, `cameras_offline` | cameras, and those delivering no frames |  |
| `camera_online`, `camera_fps` | 1 while frames arrive; frames per second received (at most 32 cameras) | `camera` |
| `camera_process_fps`, `camera_skipped_fps`, `camera_detection_fps` | frames processed, dropped for lack of time, and sent to detection | `camera` |
| `detector_inference_ms` | inference time of each detector | `detector` |
| `detector_busy_seconds` | how long a detector has been on the same inference (0 when idle) | `detector` |
| `storage_used_percent`, `storage_free_bytes` | recording disks (in-memory mounts are left out) | `path` |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: Frigate camera offline, Frigate detector stuck, Frigate storage almost full, Frigate update available, plus Monitor unreachable.

## Let DumbMonit read Frigate's statistics

1. Nothing to install: check from the DumbMonit host that Frigate's API answers on port 5000.

    ```
    curl http://frigate.lan:5000/api/version
    ```

2. If a proxy in front of Frigate expects a token (port 8971 requires a login), choose Bearer token below; otherwise choose No authentication.

3. In DumbMonit, enter Frigate's address, for example "frigate.lan".

!!! warning
    Port 5000 has no login and gives full control of Frigate: expose it only to DumbMonit, never to the internet. A camera you turned off in Frigate's settings also reads as offline.

## Credentials

| Credential | Fields |
|---|---|
| No authentication | Port 5000, the internal API. |
| Bearer token | Sent as `Authorization: Bearer …`, for a proxy that expects it. |

Address: a host name (`frigate.lan`), `host:port`, or a full URL with a path prefix. Options: protocol (HTTP by default), port (5000), certificate check and request timeout (15 s).
