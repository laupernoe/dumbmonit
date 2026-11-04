# Jellyfin

Whether Jellyfin keeps its libraries current and its viewers served: scheduled
tasks that failed on their last run (the library scan first), plugins that no
longer load, a restart waiting, and what is playing right now, transcoded or
not.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8096 | TCP | API (HTTP, default) |
| 8920 | TCP | Only with the https scheme on Jellyfin's own HTTPS port |

DumbMonit reads Jellyfin's API with an **API key** created in the dashboard. It
never starts a task, never stops a playback and never changes a setting.

The failure worth watching: **the library scan that fails every night**. A
network share unmounted, a disk full, and new films and episodes stop
appearing; Jellyfin itself keeps playing what it already knows, and the only
trace is a line in Scheduled Tasks.

## What it watches

All metrics are prefixed `dumbmonit_jellyfin_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `pending_restart` | 1 when Jellyfin waits for a restart (after a plugin install or update) | |
| `shutting_down` | 1 while Jellyfin shuts down | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | tasks whose last run failed or was aborted; tasks running now | |
| `scheduled_task_failed` | value 1 per failed task (at most 32) | `task`, `error` (first line of the message) |
| `library_scan_last_ok`, `library_scan_age_seconds` | last run of "Scan Media Library": succeeded or not, and how long ago it ended | |
| `plugins`, `plugins_broken` | installed plugins; those that failed to load or do not support this version | |
| `plugin_broken` | value 1 per broken plugin | `plugin`, `status` |
| `library_scan_scheduled` | 1 when "Scan Media Library" has a periodic trigger (not only at startup); keeps the stale-scan rule off a scan you disabled | |
| `libraries`, `libraries_scanning` | libraries; those Jellyfin is refreshing now | |
| `sessions`, `streams`, `streams_paused`, `transcodes` | client sessions; those playing something, paused, and transcoding | |
| `transcodes_software` | transcodes with no hardware acceleration, so on the CPU | |
| `streaming_users` | distinct users playing something | |
| `users`, `users_disabled` | accounts; those disabled | |
| `failed_logins_recent` | failed sign-ins in the last hour, from the activity log (at most 500 entries read) | |
| `storage_used_percent`, `storage_free_bytes` | one series per disk (libraries, data, metadata, cache, transcodes, logs; folders on the same disk count once); Jellyfin 10.11 and later | `volume` |
| `scrape_errors` | calls that failed on the last probe | |

Jellyfin does not check for its own updates (the package or the image does), so
there is no update metric.

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply:
Jellyfin scheduled task failed, Jellyfin plugin broken, Jellyfin restart
pending, Jellyfin library scan stale (no finished scan for three days, only
when the scan is scheduled), Jellyfin storage almost full (90 %), Jellyfin heavy
CPU transcoding (three or more software transcodes for 15 minutes), Jellyfin
failed sign-ins (ten in an hour), plus Monitor unreachable.

Everything past the first four calls (storage, libraries, users, activity log)
is optional: a Jellyfin that does not offer it (storage needs 10.11) or a key
that cannot read it is skipped without error.

## The monitor page

The panel above the charts reads what the probe stored; opening the page never
calls Jellyfin. It says what is wrong first (failed tasks, broken plugins, a
restart waiting), then streams, transcodes, sessions and the age of the last
library scan, then the failed tasks and broken plugins by name with Jellyfin's
own error.

## Create an API key in Jellyfin

1. In Jellyfin, open Dashboard → Advanced → API Keys → + and name the key as follows, then copy it.

    ```
    DumbMonit
    ```

2. In DumbMonit, enter Jellyfin's address and paste the key.

Vendor documentation: <https://api.jellyfin.org/>

### Why a full-rights key

Jellyfin cannot limit what a key may do: every key has full rights on the server. It is still the only way to read scheduled tasks and everyone's playback; an ordinary user sees neither.

DumbMonit only reads `/System/Info`, `/ScheduledTasks`, `/Sessions`, `/Plugins`, `/System/Info/Storage`, `/Library/VirtualFolders`, `/Users` and `/System/ActivityLog/Entries`. It never starts a task, never stops a playback and never changes a setting. Keep the key to a network you trust and put Jellyfin behind HTTPS across the internet.

!!! warning
    Jellyfin keys cannot be limited, so this one has every right on the server: keep it to DumbMonit and revoke it on the same page if it leaks.

## Troubleshooting

- **No storage, library or user figures**: storage needs Jellyfin 10.11 or later, and these calls are skipped (not an error) when the key cannot read them. A call that really fails raises `scrape_errors`.
- **"Library scan stale" never fires**: it only watches a scan that has a periodic trigger; check Dashboard → Scheduled Tasks → Scan Media Library.
- **A task keeps failing**: the `error` label of `scheduled_task_failed` carries the first line of Jellyfin's message; the full trace is in the Jellyfin log.
- **Failed sign-ins stay at 0**: they come from the activity log, which the key must be able to read; the figure covers the last hour only.
- **Unauthorized**: the key was revoked or mistyped; create a new one under Dashboard → Advanced → API Keys.

## Credentials

| Credential | Fields |
|---|---|
| API key | Sent in the `Authorization: MediaBrowser … Token="…"` header, the form Jellyfin 10.11 and later accept by default. |

Address: a host name (`jellyfin.lan`), `host:port`, or a full URL with a path
prefix (`https://example.com/jellyfin`); a trailing `/web` is accepted.
Options: protocol (HTTP by default), port (8096), certificate check and request
timeout (15 s).

Validated against Jellyfin 12.1.0 (official image).
