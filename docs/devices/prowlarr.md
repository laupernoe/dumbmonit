# Prowlarr (beta)

Whether Prowlarr still serves its indexers: health errors, indexers disabled after failures, disk space and updates.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 9696 | TCP | API (HTTP, default) |
| 443 | TCP | Only with the https scheme behind a reverse proxy |

The failure worth watching: **an indexer that is disabled and forgotten**. After a few failures Prowlarr disables an indexer for hours, then days; Sonarr, Radarr and Lidarr keep searching through the others and nothing says that releases are missing.

## What it watches

All metrics are prefixed `dumbmonit_prowlarr_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `update_available` | 1 when a newer release exists (absent when the update list is unavailable) | `latest_version` |
| `health_errors`, `health_warnings` | checks of the System → Status page that report an error or a warning |  |
| `health_issue` | value 1 for each error or warning, at most 32 | `source`, `level` |
| `indexers_disabled` | indexers still disabled after repeated failures |  |
| `disk_used_percent`, `disk_free_bytes` | each disk Prowlarr reports (root folders, download folder), at most 32 | `path` |
| `scrape_errors` | calls that failed on the last probe |  |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: Prowlarr health error, Prowlarr indexers disabled, Prowlarr disk almost full, Prowlarr update available, plus Monitor unreachable.

## Read the API key in Prowlarr

1. In Prowlarr, open Settings → General → Security and copy the API Key.

2. In DumbMonit, enter Prowlarr's address and paste the key.

!!! warning
    A Prowlarr API key cannot be limited to reading: keep it only in DumbMonit, which sends GET requests and nothing else.

## Credentials

| Credential | Fields |
|---|---|
| API key | Sent in the `X-Api-Key` header. |

Address: a host name (`prowlarr.lan`), `host:port`, or a full URL with a path prefix (`https://home.lan/prowlarr`, matching Prowlarr's URL Base setting). Options: protocol (HTTP by default), port (9696), certificate check and request timeout (15 s).
