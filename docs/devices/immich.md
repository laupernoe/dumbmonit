# Immich

Whether Immich keeps up with what is uploaded: job queues that stall or sit
paused, updates, the disk that holds the library, and how many photos and
videos it holds.

DumbMonit reads Immich's API with an **API key limited to five read
permissions**. The key cannot see a photo, an album or a person, and cannot
change anything.

The failure worth watching: **jobs that no longer run**. A queue paused during
a big import and forgotten, or a job worker that stopped, and new photos get no
thumbnail, no metadata, no face and no smart search, while uploads keep
succeeding. Immich only shows it on its Jobs page.

## What it watches

All metrics are prefixed `dumbmonit_immich_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `update_available` | 1 when Immich's own version check knows a newer release (absent when the check is turned off) | `latest_version` |
| `maintenance_mode` | 1 while Immich is in maintenance mode | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | the disk that holds the library | |
| `photos`, `videos`, `usage_bytes`, `users` | library size, all users together | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | all queues together | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | per queue: waiting (including delayed and held by a pause), running, failed, paused | `queue` |
| `statistics_readable` | 0 when the key belongs to an ordinary account and counts and queues are skipped | |
| `scrape_errors` | calls that failed on the last probe | |

Recent Immich versions drop failed jobs from the queue once they are logged, so
`queue_failed` usually reads 0; a job that failed shows in Immich's own log.

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply:
Immich jobs stalled, Immich queue paused, Immich storage almost full, Immich
update available, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
calls Immich. It says what is wrong first (queues stuck, queues paused,
maintenance), then updates, then photos, videos, library size, disk used and
jobs waiting and running, then the queues that have work, by name.

## Create a read-only API key in Immich

1. Log in to Immich with an account that can open Administration: Immich only lets those accounts read photo counts and job queues. Open Account Settings → API Keys → New API Key, name it as follows, and tick only these five permissions.

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read
    ```

2. Copy the key now: Immich shows it only once. It reads those five pages and nothing else: no photo, no album, no setting. If your Immich does not offer queue.read, tick job.read instead.

3. A key made by an ordinary account works too: version, updates and disk space are read, photo counts and job queues are skipped.

4. In DumbMonit, enter Immich's address, for example "immich.lan" (port 2283) or "https://photos.example.com" behind a reverse proxy, and paste the key.

!!! warning
    A paused queue is easy to forget: new photos get no thumbnail, no metadata and no face, and Immich only says so on its Jobs page. DumbMonit warns when jobs wait in a queue that is not paused and runs nothing.

What each permission opens: `server.about` the version (`/api/server/about`),
`server.versionCheck` the latest release Immich knows of, `server.storage` the
disk, `server.statistics` photo and video counts, `queue.read` the job queues
(`/api/queues`; older versions answer on `/api/jobs` with `job.read`).

## Credentials

| Credential | Fields |
|---|---|
| API key | Sent in the `x-api-key` header. |

Address: a host name (`immich.lan`), `host:port`, or a full URL with a path
prefix; a trailing `/api` is accepted. Options: protocol (HTTP by default),
port (2283), certificate check and request timeout (15 s).

Validated against Immich v3.2.4 (official images, machine learning off).
