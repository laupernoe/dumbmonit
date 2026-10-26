# Kopia

!!! warning "Beta"

    Built from the Kopia server's source and API behaviour, not yet tested
    against a real server. Tell us what your server answers differently.

A Kopia server through its REST API: the last snapshot of each source (age,
errors, size, files), sources that missed their schedule, and the repository
connection.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 51515 | TCP | Server API (HTTPS, default) |

DumbMonit reads two routes of the server started with `kopia server start`,
with HTTP Basic authentication (the server user name and password):

- `GET /api/v1/repo/status`: whether the repository is connected, its storage
  and encryption. This read validates the credentials.
- `GET /api/v1/sources`: the sources the server backs up, each with its state,
  its schedule and its last snapshot (start and end time, size, files,
  errors). Read only when the repository is connected.

The snapshot history of each source (`/api/v1/snapshots`) is not read: the
last snapshot already comes with the source, and that is all the alerts need.
Sources whose state is `REMOTE` belong to other machines that use the same
repository: they are counted, not described.

## What it watches

All metrics are prefixed `dumbmonit_kopia_`.

| Metric | Labels | Meaning |
|---|---|---|
| `repo_connected` | | 1 when the server is connected to its repository. |
| `repo_readonly` | | 1 for a read-only repository. |
| `repo_info` | `storage`, `hash`, `encryption` | Value 1, only when connected. |
| `source_has_snapshot` | `source` | 1 once the source has a snapshot. `source` reads `user@host:/path`. |
| `source_scheduled` | `source` | 1 when Kopia triggers the source itself (an interval, times of day, a cron expression or a next snapshot time, and not "manual"). |
| `source_uploading` | `source` | 1 while a snapshot is running. |
| `source_status_info` | `source`, `status` | Value 1: `IDLE`, `PENDING`, `UPLOADING`, `PAUSED`, `UNKNOWN`. |
| `source_last_snapshot_age_seconds` | `source` | Since the end of the last snapshot; absent for a source without snapshot. |
| `source_last_snapshot_duration_seconds` | `source` | How long the last snapshot took. |
| `source_last_snapshot_errors` | `source` | Files the last snapshot could not read. |
| `source_last_snapshot_bytes`, `source_last_snapshot_files` | `source` | Size and file count of the last snapshot. |
| `sources`, `sources_without_snapshot`, `sources_with_errors`, `sources_uploading`, `sources_remote` | | Counts; at most 200 sources are described one by one. |

The [built-in rules](../alerting/rules.md#kopia) that apply:

- **Kopia snapshot with errors**: the last snapshot of a source finished with
  errors (Advisory, reminded daily).
- **Kopia snapshot too old**: a *scheduled* source has no snapshot for more
  than two days (Advisory, reminded daily). A manual source is never reported.
- **Kopia repository disconnected**: the server lost its repository
  (Warning).

## The monitor page

The Backups panel says what is wrong in a sentence, shows the repository
connection and the source counts, then lists the sources — those behind their
schedule or with errors first — with the age, the unreadable files and the
size of their last snapshot.

## Start the Kopia server with its own user name and password

1. On the machine that runs the backups, start the Kopia server with a generated certificate and its own user name and password.

    ```
    kopia server start --address=0.0.0.0:51515 --tls-generate-cert --server-username=dumbmonit --server-password=PASTE-A-LONG-RANDOM-PASSWORD
    ```

2. Check that the repository is connected to that server and that the DumbMonit server can reach port 51515.
3. In DumbMonit, enter the server address (for example nas.lan) with that user name and password.
4. Tick Accept an unverifiable certificate: the generated certificate is self-signed. If the server runs with --insecure, choose the http protocol instead.

### Required permissions

- **Server user name and password** — Kopia has no read-only API account

!!! warning
    Kopia's API has no read-only account: the server password also lets its holder start and delete snapshots. DumbMonit only reads, but keep port 51515 on your local network.

### Why these permissions

Kopia's server knows one set of credentials for its web interface and its API,
with no read-only variant. DumbMonit only issues `GET` requests. Running the
server without a password (`--without-password`, on a trusted network only) is
supported: choose "No authentication".

## Credentials

| Credential | Fields |
|---|---|
| Server user name and password | The `--server-username` and `--server-password` of the Kopia server. |
| No authentication | For a server started without a password. |

Address: the host name or IP of the server, `host:port`, or a URL. Options:

- **Protocol** (`https`) and **Port** (51515).
- **Accept an unverifiable certificate**: for the certificate made by
  `--tls-generate-cert`.
- **Timeout per request** (15 s).

## Troubleshooting

"Kopia refused the credentials (401)": the user name or password differs from
the `--server-username` / `--server-password` the server was started with.

`repo_connected` is 0: the server runs but its repository is not connected
(`kopia repository status` on the server).

A source never shows an age: it has no snapshot yet, or its only entry is a
`REMOTE` one from another machine.
