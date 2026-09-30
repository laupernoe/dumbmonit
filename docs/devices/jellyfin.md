# Jellyfin

Whether Jellyfin keeps its libraries current and its viewers served: scheduled
tasks that failed on their last run (the library scan first), plugins that no
longer load, a restart waiting, and what is playing right now, transcoded or
not.

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
| `sessions`, `streams`, `streams_paused`, `transcodes` | client sessions; those playing something, paused, and transcoding | |
| `scrape_errors` | calls that failed on the last probe | |

Jellyfin does not check for its own updates (the package or the image does), so
there is no update metric.

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply:
Jellyfin scheduled task failed, Jellyfin plugin broken, Jellyfin restart
pending, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
calls Jellyfin. It says what is wrong first (failed tasks, broken plugins, a
restart waiting), then streams, transcodes, sessions and the age of the last
library scan, then the failed tasks and broken plugins by name with Jellyfin's
own error.

## Create an API key in Jellyfin

1. In Jellyfin: Dashboard → API Keys (under Advanced) → +, and name the key as follows. Copy it.

    ```
    DumbMonit
    ```

2. Jellyfin cannot limit what a key may do: every key has full rights on the server. It is still the only way to read scheduled tasks and everyone's playback; an ordinary user sees neither.

3. In DumbMonit, enter Jellyfin's address, for example "jellyfin.lan" (port 8096) or "https://media.example.com" behind a reverse proxy, and paste the key.

4. DumbMonit only reads /System/Info, /ScheduledTasks, /Sessions and /Plugins. It never starts a task, never stops a playback and never changes a setting.

!!! warning
    Because the key carries every right, keep it to a network you trust and put Jellyfin behind HTTPS across the internet. If it ever leaks, revoke it on the same page: nothing else uses it.

## Credentials

| Credential | Fields |
|---|---|
| API key | Sent in the `Authorization: MediaBrowser … Token="…"` header, the form Jellyfin 10.11 and later accept by default. |

Address: a host name (`jellyfin.lan`), `host:port`, or a full URL with a path
prefix (`https://example.com/jellyfin`); a trailing `/web` is accepted.
Options: protocol (HTTP by default), port (8096), certificate check and request
timeout (15 s).

Validated against Jellyfin 12.1.0 (official image).
