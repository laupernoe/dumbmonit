# Duplicati

!!! warning "Beta"

    Built from Duplicati's source code and documentation, not yet tested
    against a real server. Tell us what your server answers differently.

A Duplicati server through its API: the age of each task's last successful
backup, its last result, duration, size, warnings and errors, and the task
running now.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8200 | TCP | Web interface and API (HTTP, default) |

Duplicati has no user accounts, only the password of its web interface. From
Duplicati 2.1, DumbMonit exchanges that password for a short-lived access
token (`POST /api/v1/auth/login`, then `Authorization: Bearer`), keeps it for
ten minutes and signs in again if the server refuses it.

!!! note "Duplicati 2.0"
    Version 2.0 signs in through `/login.cgi` (a nonce, a salt and a
    `salted-sha256` hash, then session cookies). That mode is not supported:
    against a 2.0 server DumbMonit reports that the sign-in route is missing
    and asks for 2.1 or later.

The task list (`GET /api/v1/backups`) is the one required read. The others
are read when the server answers and never fail the probe:

- `GET /api/v1/serverstate`: the task running now, and whether the server is
  paused.
- `GET /api/v1/progressstate`: the progress of that task.
- `GET /api/v1/backup/{id}/log`: the result of each task's last run
  (`Success`, `Warning`, `Error`, `Fatal`) with its warning and error counts,
  for the first 50 tasks. Without it, a failure is deduced from the task's
  `LastErrorDate` and `LastErrorMessage`.

The task list already carries each task's metadata (`LastBackupDate`,
`LastBackupFinished`, `LastBackupDuration`, `LastErrorMessage`, sizes), so
`GET /api/v1/backup/{id}` is not called.

## What it watches

All metrics are prefixed `dumbmonit_duplicati_`.

| Metric | Labels | Meaning |
|---|---|---|
| `backup_failed` | `backup`, `backup_id` | 1 when the last run failed: its result is `Error` or `Fatal`, or, without the log, an error is more recent than the last run. A warning is not a failure. |
| `backup_running` | `backup`, `backup_id` | 1 for the task running now. |
| `backup_scheduled` | `backup`, `backup_id` | 1 when the task has a schedule. |
| `backup_last_success_age_seconds` | `backup`, `backup_id` | Since the last successful backup; absent for a task that never succeeded. |
| `backup_last_duration_seconds` | `backup`, `backup_id` | Duration of the last run. |
| `backup_source_bytes`, `backup_source_files` | `backup`, `backup_id` | Size and file count of the source. |
| `backup_target_bytes`, `backup_versions` | `backup`, `backup_id` | Size at the destination and number of versions kept. |
| `backup_last_result_info` | `backup`, `backup_id`, `result` | Value 1: `success`, `warning`, `error`, `fatal`. |
| `backup_last_warnings`, `backup_last_errors` | `backup`, `backup_id` | Counts of the last run. |
| `backup_last_error_info` | `backup`, `backup_id`, `message` | Value 1 with the last error message (200 characters at most), only while the task is failed. |
| `backup_progress_percent` | `backup`, `backup_id` | Progress of the running task. |
| `server_paused` | | 1 when the server is paused. |
| `backups`, `backups_failed`, `backups_running` | | Counts; at most 200 tasks are described one by one. |

The [built-in rules](../alerting/rules.md#duplicati) that apply:

- **Duplicati backup failed** (Warning, reminded daily).
- **Duplicati backup too old**: a *scheduled* task has not succeeded for more
  than two days (Advisory, reminded daily).

## The monitor page

The Backups panel says what is wrong in a sentence, shows the failed and
running counts and whether the server is paused, then lists the tasks —
failed first — with the age of their last success, their size and, for a
failed one, the last error message.

## Let DumbMonit read the Duplicati API

1. In the Duplicati web interface, open Settings and set a long random password for the interface: Duplicati 2.1 requires one.
2. If the server only listens on the machine itself, start it with these options so that DumbMonit can reach it.

    ```
    --webservice-interface=any --webservice-allowed-hostnames=*
    ```

3. In DumbMonit, enter the Duplicati address (for example nas.lan) and paste that password.

### Required permissions

- **Interface password** — Duplicati has no accounts or read-only mode

!!! warning
    Duplicati 2.1 or later is required: version 2.0 signs in through a different mechanism that DumbMonit does not use. The password gives full access, so keep port 8200 on your local network.

### Why these permissions

Duplicati knows a single secret, the interface password; it can neither be
limited to reading nor attached to a named account. DumbMonit only issues
`GET` requests apart from the sign-in.

## Credentials

| Credential | Fields |
|---|---|
| Web interface password | The password of the Duplicati web interface (stored encrypted). |

Address: the host name or IP of the server, `host:port`, or a URL. Options:

- **Protocol** (`http`) and **Port** (8200).
- **Accept an unverifiable certificate**: if the server runs over HTTPS with a
  self-signed certificate.
- **Timeout per request** (15 s).

## Troubleshooting

"Duplicati refused the password": the password differs from the interface's.

"this collector needs Duplicati 2.1 or later": the server has no
`/api/v1/auth/login`; update it.

The server answers "Invalid host header" or a similar refusal: add the name
you use in DumbMonit to `--webservice-allowed-hostnames`.

A task shows no age: it never completed a backup successfully.
