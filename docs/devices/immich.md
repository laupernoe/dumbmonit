# Immich

Whether Immich keeps up with what is uploaded: job queues that stall or sit
paused, updates, the disk that holds the library, how many photos and videos
it holds, and — for every phone, tablet and desktop signed in to the account —
its last connection and last backup.

DumbMonit reads Immich's API with an **API key limited to eight read
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

## Devices

Every device signed in to the account that owns the key, with its last
connection and last backup, is published under a generic family shared by
every DumbMonit integration that tracks client devices — `dumbmonit_client_device_*`,
prefixed without `immich`, documented in full under
[Client devices](../alerting/rules.md#client-devices):

| Metric | What | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Last connection (the session's last activity) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | Last backup; see the limit below | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positive once a device has gone past `device_stale_days` (3 by default) without connecting or backing up; drives the [Client device stale](../alerting/rules.md#client-devices) rule | `device`, `type`, `os`, `user`, `kind`, `signal` (`connection` or `backup`) |

**Limit worth knowing**: Immich's API does not record which device uploaded a
given photo, so `last_backup_timestamp_seconds` is the whole library's most
recent upload — the same value for every device, published under a device
named `library` rather than a real one. It catches every device having
stopped at once (storage full, the account logged out everywhere); it will
not catch a single device going silent while another keeps uploading. Only
`last_seen_timestamp_seconds` — the last time each device's session was
active — is genuinely per device.

The device page's table shows these figures for every device, with a status
of OK or Stale.

## The device page

The panel above the charts reads what the probe stored; opening the page never
calls Immich. It says what is wrong first (queues stuck, queues paused,
maintenance), then updates, then photos, videos, library size, disk used and
jobs waiting and running, then the queues that have work, by name, then the
table of devices (name, type, OS, last connection, last backup, status).

## Create a read-only API key in Immich

1. Log in to Immich with an account that can open Administration: Immich only lets those accounts read photo counts and job queues. Open Account Settings → API Keys → New API Key, name it as follows, and tick these eight permissions: the first five for job queues, counts and version, the last three to list the account's devices and their last connection and backup.

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read session.read asset.read user.read
    ```

2. Copy the key now: Immich shows it only once. It reads those eight pages and nothing else: no photo, no album, no setting. If your Immich does not offer queue.read, tick job.read instead.

3. A key made by an ordinary account works too: version, updates, disk space and devices are read, photo counts and job queues are skipped.

4. In DumbMonit, enter Immich's address, for example "immich.lan" (port 2283) or "https://photos.example.com" behind a reverse proxy, and paste the key.

!!! warning
    A paused queue is easy to forget: new photos get no thumbnail, no metadata and no face, and Immich only says so on its Jobs page. DumbMonit warns when jobs wait in a queue that is not paused and runs nothing. session.read, asset.read and user.read list every device of the account that owns the key — its phones, tablets and browsers — with each one's last connection. Immich's API does not record which device uploaded a given photo, so "last backup" is the whole library's most recent upload, shown the same for every device: if only one device stops backing up while the others keep going, its connection will show as stale but the shared backup figure will still look fresh.

What each permission opens: `server.about` the version (`/api/server/about`),
`server.versionCheck` the latest release Immich knows of, `server.storage` the
disk, `server.statistics` photo and video counts, `queue.read` the job queues
(`/api/queues`; older versions answer on `/api/jobs` with `job.read`),
`session.read` the account's devices (`/api/sessions`), `asset.read` the
library's most recent upload (`/api/search/metadata`), `user.read` the
account's display name (`/api/users/me`).

## Credentials

| Credential | Fields |
|---|---|
| API key | Sent in the `x-api-key` header. |

Address: a host name (`immich.lan`), `host:port`, or a full URL with a path
prefix; a trailing `/api` is accepted. Options: protocol (HTTP by default),
port (2283), certificate check, request timeout (15 s), and device staleness
(`device_stale_days`, 3 days by default).

Validated against Immich v3.2.4 (official images, machine learning off); the
device endpoints were checked against the `v3.2.4` tag of Immich's own
OpenAPI specification (`immich-app/immich`), which confirmed that
`/api/search/metadata` no longer accepts a `deviceId` filter — the reason
"last backup" is library-wide rather than per device.
