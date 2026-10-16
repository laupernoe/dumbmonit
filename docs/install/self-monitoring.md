# Watching DumbMonit itself

Who watches the watcher? DumbMonit answers in two parts:

- an **outgoing heartbeat**, for what it cannot see from the inside: the host
  down, the container stopped, the engine stuck;
- **built-in rules** on its own health, for what it can see: the data volume
  filling up, the metrics database restarting in a loop, polls stalling,
  writes failing, the database growing abnormally.

Both live in **Settings → System → DumbMonit itself**, which also shows the
state of the engine right now.

## The outgoing heartbeat

DumbMonit calls a URL of yours at a regular interval (60 seconds by default),
**only while the engine is healthy**:

- the scheduler is running (it completed a round in the last two minutes);
- the SQLite database answers;
- VictoriaMetrics answers.

When one of them fails, nothing is sent. A "dead man's switch" service waits
for these calls and alerts you when they stop, whether the cause is the whole
host, the container or one component.

### Setting it up

1. Create a check on the service of your choice and copy its ping URL:
   - **Healthchecks.io** (or a self-hosted Healthchecks): a check with a period
     equal to the interval, and a grace time a little longer
     (`https://hc-ping.com/<uuid>`);
   - **Uptime Kuma**: a monitor of type *Push*
     (`https://kuma.example.com/api/push/<token>?status=up&msg=OK&ping=`);
   - **Better Stack**: a heartbeat (`https://uptime.betterstack.com/api/v1/heartbeat/<token>`);
   - any service that accepts a plain `GET` and alerts when it stops coming
     (Cronitor, Dead Man's Snitch…).
2. In **Settings → System → DumbMonit itself**, paste the URL, choose the
   interval (30 to 3600 seconds), turn **Send a heartbeat** on and save.
3. Press **Test**: one call leaves right away and the result is shown. The
   service should mark the check as up.

The call is a `GET` with a 10-second timeout. It follows the same guard as the
notification channels: it is never sent to DumbMonit's own web interface or to
its VictoriaMetrics.

!!! note "ntfy"
    ntfy delivers every message it receives, so pointing the heartbeat at an
    ntfy topic sends you a notification every minute rather than when the
    calls stop. Use a dead man's switch service, and keep ntfy as a channel
    for the alerts themselves.

### Failure signal

By default, a sick engine simply stops calling, and the service alerts once
its grace time has passed. Turn **Signal failures** on to call the failure URL
instead, which the service reports at once:

| Service | Failure URL |
|---|---|
| Healthchecks.io, Better Stack | `/fail` added to the path: `https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | `status=up` replaced by `status=down` in the query |

Leave it off for any other service: an unknown URL with `/fail` at the end
would most likely answer 404, which looks like nothing being sent anyway.

### The URL is a secret

A ping URL usually holds its own token: whoever knows it can make your check
look healthy. DumbMonit stores it encrypted with the instance secret, never
returns it through the API (only its host is shown), and leaves it out of the
logs and of error messages. To change it, type the new one; to keep it, leave
the field empty. **Remove the URL** forgets it and turns the heartbeat off.

The setting is not part of the encrypted configuration bundle: set it again
after restoring onto a new instance.

## Built-in rules

Every minute DumbMonit writes a few measurements about itself to
VictoriaMetrics, and the built-in rules of the *DumbMonit itself* group read
them. They behave like every other rule: same channels, same silences, same
per-rule thresholds in **Alerts → Rules**.

| Rule | Fires when | Default |
|---|---|---|
| DumbMonit data volume almost full | Less than 10% free on the volume that holds `/data` | 15 min, warning |
| DumbMonit metrics database restarting in a loop | The embedded VictoriaMetrics was restarted 3 times or more in 30 minutes | 1 min, critical |
| DumbMonit scheduler stalled | No scheduling round for more than two minutes | 5 min, critical |
| DumbMonit polls running late | Monitors stay overdue after every round for 15 minutes | 15 min, warning |
| DumbMonit metric writes failing | Batches refused by VictoriaMetrics, or samples dropped, in the last 15 minutes | 5 min, warning |
| DumbMonit database growing fast | `dumbmonit.db` more than doubled in a day and is over 100 MB | 1 h, warning |
| DumbMonit backup did not run | No successful local backup for two days ([details](backup.md#scheduled-local-backups)) | 1 h, warning |

These rules need VictoriaMetrics to be evaluated. When VictoriaMetrics itself
is down for good, or the whole host is, no rule can fire: that is the
heartbeat's job.

### Measurements

| Metric | Meaning |
|---|---|
| `dumbmonit_instance_data_free_bytes` | Free space on the volume that holds `/data`, for an unprivileged user. |
| `dumbmonit_instance_data_size_bytes` | Size of that volume. |
| `dumbmonit_instance_database_bytes` | Size of `dumbmonit.db` and its write-ahead log. |
| `dumbmonit_instance_scheduler_lag_seconds` | Seconds since the scheduler last completed a round (normally under 2). |
| `dumbmonit_instance_scheduler_backlog` | Monitors still due for a probe after the last round. |
| `dumbmonit_instance_sample_writes_failed_total` | Batches VictoriaMetrics refused or did not answer, since the server started. |
| `dumbmonit_instance_samples_dropped_total` | Samples dropped before reaching the write buffer, since the server started. |
| `dumbmonit_instance_vm_restarts_total` | Restarts of the embedded VictoriaMetrics since the server started (embedded only). |

They carry no `target` label: the alerts they raise are not attached to a
monitor. For a Prometheus of your own, `GET /metrics` exposes the same kind of
figures, see [Scraping DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## API

| Method | Route | |
|---|---|---|
| `GET` | `/api/self-monitoring` | Engine state, data volume, database size, heartbeat setting and last heartbeat. |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`. `url` omitted keeps it, `""` clears it. |
| `POST` | `/api/self-monitoring/heartbeat/test` | One call now, to `{ "url" }` or to the stored URL. |

The two writes are for administrators (or a token with the `write` scope).
