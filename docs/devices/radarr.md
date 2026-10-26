# Radarr (beta)

Whether Radarr still finds and imports movies: health errors, failing download clients and indexers, errors in the download queue, disk space and updates.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 7878 | TCP | API (HTTP, default) |
| 443 | TCP | Only with the https scheme behind a reverse proxy |

The failure worth watching: **a download client or an indexer that quietly stopped**. Radarr keeps running and its page opens, but nothing is grabbed or imported any more; the only trace is the red banner of its System → Status page, which DumbMonit reads as it is.

## What it watches

All metrics are prefixed `dumbmonit_radarr_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `update_available` | 1 when a newer release exists (absent when the update list is unavailable) | `latest_version` |
| `health_errors`, `health_warnings` | checks of the System → Status page that report an error or a warning |  |
| `health_issue` | value 1 for each error or warning, at most 32 | `source`, `level` |
| `queue_total`, `queue_unknown` | items in the download queue, and those Radarr cannot match to its library |  |
| `queue_has_errors`, `queue_has_warnings` | 1 when at least one queue item has an error (failed download, import blocked) or a warning |  |
| `disk_used_percent`, `disk_free_bytes` | each disk Radarr reports (root folders, download folder), at most 32 | `path` |
| `scrape_errors` | calls that failed on the last probe |  |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: Radarr health error, Radarr queue has errors, Radarr disk almost full, Radarr update available, plus Monitor unreachable.

## Read the API key in Radarr

1. In Radarr, open Settings → General → Security and copy the API Key.

2. In DumbMonit, enter Radarr's address and paste the key.

!!! warning
    A Radarr API key cannot be limited to reading: keep it only in DumbMonit, which sends GET requests and nothing else.

## Credentials

| Credential | Fields |
|---|---|
| API key | Sent in the `X-Api-Key` header. |

Address: a host name (`radarr.lan`), `host:port`, or a full URL with a path prefix (`https://home.lan/radarr`, matching Radarr's URL Base setting). Options: protocol (HTTP by default), port (7878), certificate check and request timeout (15 s).
