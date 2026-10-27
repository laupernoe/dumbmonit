# qBittorrent (beta)

Whether qBittorrent is downloading: torrents in error or stalled, lost connectivity (closed port, VPN down), speeds, the free space of the download folder and the number of torrents by state.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8080 | TCP | WebUI (HTTP, default) |
| 443 | TCP | Only with the https scheme behind a reverse proxy |

The failure worth watching: **torrents in error that never resume**. A full disk or a moved folder puts torrents in error or missing-files state; qBittorrent keeps running and shows them only in its list. A closed port or a dropped VPN ("firewalled" or "disconnected") stops everything just as quietly.

## What it watches

All metrics are prefixed `dumbmonit_qbittorrent_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `connected` | 1 while qBittorrent is connected to the network (0 when firewalled or disconnected) | `status` |
| `download_speed_bytes`, `upload_speed_bytes` | current speeds, bytes per second |  |
| `free_space_bytes` | free space where qBittorrent saves its downloads |  |
| `dht_nodes` | DHT nodes known |  |
| `torrents_total` | torrents in the list |  |
| `torrents` | torrents per group | `state` (`downloading`, `stalled_download`, `seeding`, `paused`, `queued`, `checking`, `error`, `other`) |
| `torrents_errored` | torrents in error or with missing files |  |
| `torrent_error` | value 1 for each torrent in error, at most 32 | `torrent`, `state` |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: qBittorrent torrents in error, qBittorrent disconnected, qBittorrent free space low, plus Monitor unreachable.

## Give DumbMonit access to qBittorrent's WebUI

1. In qBittorrent, open Tools → Options → Web UI and tick Web User Interface (Remote control).

2. Under Authentication, set a user name and password named as follows, or allow DumbMonit's address in the whitelisted IP subnets bypass.

    ```
    dumbmonit
    ```

3. In DumbMonit, enter qBittorrent's address and that user name and password, or choose No authentication for a bypassed address.

!!! warning
    qBittorrent bans an address for an hour after five failed logins (HTTP 403): fix the password before the next probe, or the probe stays refused.

## Credentials

| Credential | Fields |
|---|---|
| User name / password | The WebUI account. DumbMonit opens a session with `POST /api/v2/auth/login` and reuses the cookie for the probe. |
| No authentication | For an address qBittorrent lets in without a login (the whitelisted subnets option). |

Address: a host name (`qbittorrent.lan`), `host:port`, or a full URL with a path prefix. Options: protocol (HTTP by default), port (8080), certificate check and request timeout (15 s).
