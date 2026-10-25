# Heartbeat (push)

The silent failure nobody notices: the nightly backup that stopped running
after an update, the certificate renewal cron that lost its permissions, the
Home Assistant automation that broke on a rename. Nothing complains, and you
find out the day you need the backup.

A **heartbeat** monitor (also called a *dead man's switch*, or a *push* monitor
in Uptime Kuma) turns the logic around: DumbMonit contacts nothing. The job
calls a secret URL each time it runs, and it is the **absence** of a call that
raises the alert.

## Network ports to open

From the agent to the DumbMonit server:

| Port | Protocol | Purpose |
|---|---|---|
| 8080 | TCP | Heartbeat call (default port, 443 behind a proxy) |

## How it works

Each heartbeat monitor gets a URL of the form
`https://<your DumbMonit>/api/push/<token>`. The token is a 128-bit random
value, stored hashed for lookup and encrypted for display: it is shown on the
monitor page, with a copy button, for as long as the monitor exists. It is
deliberately open — no session, no API token, no `X-Requested-With` header —
because a `curl` line in a crontab has none of those. Holding the token only
lets you say "the job ran".

On every scheduler pass (the monitor's interval, 60 s by default) the collector
reads the time of the last call and writes:

| Situation | `probe_success` | Effect |
|---|---|---|
| No call received yet | *(nothing written)* | The monitor shows **Waiting**. No alert: the job is not in place yet. |
| Last call within the expected interval + grace | `1` | **Reporting**. |
| Last call older than the expected interval + grace | `0`, `reason="missed"` | **Down**; the "Heartbeat missed" rule fires after two minutes. |
| Last call said `status=down` | `0`, `reason="reported_down"` | **Down** at once, whatever the clock says. |

A call also re-evaluates the monitor immediately, so a job that resumes clears
its alert without waiting for the next scheduler pass.

The endpoint answers `204` with an empty body when the call is recorded,
`404` for an unknown or regenerated token, `400` for a `status` that is neither
`up` nor `down`, and `429` (with `Retry-After`) beyond sixty calls per minute
for one token. Both `GET` and `POST` are accepted; `HEAD` works too.

### Query parameters

| Parameter | Meaning |
|---|---|
| `status` | `up` (default) or `down`: the job reports a failure itself. |
| `msg` | A free word of explanation, kept (truncated to 500 characters) and shown on the monitor page. |
| `exit_code` | The job's exit code (see below). Anything other than `0` is a failure. |

`status` and `msg` are the parameters of Uptime Kuma's push monitor: a script
written for it works unchanged.

## Start, success, failure and exit code

The plain URL is enough to know that a job ran. To also know **how long** it
took, **whether it failed** and **what it printed**, call the same URL with a
suffix:

| URL | Meaning |
|---|---|
| `/api/push/<token>` | A run that succeeded, of unknown duration (the original call, unchanged). |
| `/api/push/<token>/start` | The job starts. Opens a run; the next end call gives its duration. |
| `/api/push/<token>/success` | The job succeeded. Same as the plain call, but closes the open run. |
| `/api/push/<token>/fail` | The job failed. |
| `/api/push/<token>/<code>` | The job ended with this exit code, for example `/2`. `0` is a success, anything else a failure. |

The exit code can also be given as `?exit_code=2` on `/success` or `/fail`; a
`/success` that carries a non-zero code counts as a failure.

With `POST`, the **request body is the output of the job**. It is kept
truncated to its **last 10 KB** (the error is almost always at the end; the
text is cut on a character boundary, invalid bytes are replaced). A body larger
than 256 KB is refused with `413`: pipe the output through `tail -c 10000`.
Without a body, `msg` is kept as the output.

The duration is the time between `/start` and the end call. A call without a
preceding `/start` has an unknown duration. A new `/start` while a run is still
open marks that run as *interrupted*. The monitor page lists the last
runs (200 per monitor, 90 days at most) with their status, duration, exit code
and output.

The same guards apply to every entry point: the token, sixty calls per minute
per token, and a bounded body.

## Setup

The steps below are the ones the notice next to the form shows.

1. In the address, write a short label for the job, unique among your heartbeats: "nightly-backup".

2. Set the expected interval to the job's schedule ("24h" for a nightly job), or give a cron expression and a time zone, then save the monitor.

3. Copy the URL shown on the monitor page and call it at the end of the job.

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. To time the job and report how it went, call .../start before it, then .../success or .../fail after it, with the output of the job as the request body. An exit code can be given as ?exit_code=2 or as the last path segment: .../2.

5. To report a failure at once, append ?status=down&msg=... to the URL.

!!! warning
    Call the URL at the end of the job, after the part that matters: a call at the top would report a success even when the job fails.

Nothing is contacted: the label only has to be unique. Until the first call arrives the monitor shows "Waiting" and nothing is alerted. Lost or leaked URL? "Regenerate" on the monitor page issues a new one; the old one stops answering immediately.

Credentials: none.

### Examples

A crontab line, nightly at 03:00, that only calls in when the backup succeeded:

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

A script that reports its own failure, with the exit code as the message:

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
if /usr/local/bin/backup.sh; then
    curl -fsS -m 10 --retry 3 "$URL" > /dev/null
else
    curl -fsS -m 10 --retry 3 "$URL?status=down&msg=backup+exit+$?" > /dev/null
fi
```

A wrapper that reports the start, the exit code and the output of any command
(only the last 10 KB of the output are sent). Replace `/usr/local/bin/backup.sh`
with the job:

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
curl -fsS -m 10 --retry 3 "$URL/start" > /dev/null
OUT=$(/usr/local/bin/backup.sh 2>&1); CODE=$?
printf '%s' "$OUT" | tail -c 10000 \
    | curl -fsS -m 10 --retry 3 --data-binary @- "$URL/$CODE" > /dev/null
exit "$CODE"
```

`$URL/$CODE` is `/0` after a success and `/2`, `/23`... after a failure: one
call carries the result, the exit code and the output.

Home Assistant, a `rest_command` called at the end of an automation:

```yaml
rest_command:
  dumbmonit_heartbeat:
    url: "https://monit.example.com/api/push/<token>"
    method: post
```

Windows Task Scheduler (PowerShell):

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "https://monit.example.com/api/push/<token>" | Out-Null
```

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `expected_interval` | Expected interval | `24h` | How often the job is supposed to call in: 30m, 1h, 6h, 24h, 7d (or a number of seconds). A missed call is declared once this interval plus the grace period has passed. |
| `grace` | Grace period | `10%` | Extra time tolerated after the expected interval before the heartbeat counts as missed: a percentage of the interval (10%) or a fixed duration (15m). Never less than one minute. |
| `schedule` | Cron schedule | *(empty)* | Instead of the interval: when the job is supposed to start, as a 5-field cron expression (minute hour day-of-month month day-of-week) or a shortcut such as `@daily`. Empty: use the interval. |
| `timezone` | Time zone | `UTC` | IANA time zone in which the cron schedule is read, for example `Europe/Paris`. It follows daylight saving time. |
| `max_duration` | Maximum duration | *(empty)* | A run that lasts longer than this raises the "Job too long" alert: `30m`, `2h`. Needs a `/start` at the beginning of the job. Empty: a run still open after one period plus the grace period counts as stuck. |

Durations accept `s`, `m`, `h`, `d`, `w` suffixes and can be combined
(`1h30m`); a bare number is a number of seconds. The expected interval must be
between 10 seconds and a year.

### Cron schedule

A job that runs "every day at 02:30" or "on weekdays at 08:00" is better
described by a cron expression than by an interval, because the expected time
does not drift after a late run. Give `schedule` (and `timezone`, UTC by
default); the interval is then ignored.

- Five fields: minute (0-59), hour (0-23), day of month (1-31), month (1-12 or
  `jan`-`dec`), day of week (0-7 or `sun`-`sat`, 0 and 7 are Sunday).
- Each field takes `*`, a value, a range (`8-17`), a list (`1,15`), and a step
  (`*/15`, `10-50/10`). Names are case-insensitive.
- Shortcuts: `@yearly`, `@annually`, `@monthly`, `@weekly`, `@daily`,
  `@midnight`, `@hourly`.
- As in cron, when both the day of month and the day of week are restricted, a
  date matches if **either** does.
- Daylight saving time: when a local time does not exist (spring forward), the
  run is expected right after the gap; when it happens twice (fall back), it is
  expected once, at the first occurrence. A schedule on every hour (`0 * * * *`)
  follows real time.

The job is **late** when the last scheduled time plus the grace period has
passed with no call (a `/start`, `/success` or plain call, even half a minute
early, counts). A percentage grace is a percentage of the gap between two
scheduled times. A schedule or a time zone that cannot be read is refused when
the monitor is saved, with the reason.

## Metrics

All labelled `probe="push"` in addition to `target`, `host` and `tag_*`, all
prefixed `dumbmonit_`.

| Metric | Meaning |
|---|---|
| `probe_success` | 1 if the last call is on time and did not report a failure, 0 otherwise. Absent until the first call. |
| `probe_failure_info` | Presence (1) with a `reason` label: `missed` or `reported_down`. |
| `push_last_seen_seconds` | Age of the last call. |
| `push_received_total` | Calls received since the token was created (counter). |
| `push_run_failed` | 1 if the last finished run failed (`/fail` or a non-zero exit code), 0 otherwise. Absent until a run has finished. |
| `push_run_too_long` | 1 if a run is longer than the maximum duration (open or finished), 0 otherwise. Absent until a run is known. |
| `push_last_run_duration_seconds` | Duration of the last finished run, when a `/start` preceded it. |

Availability over thirty days, as for any service monitor:
`avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Built-in rules that apply (all at warning level):

| Rule | Fires when |
|---|---|
| **Heartbeat missed** (`push_missed`) | The job did not call in before its expected interval or cron schedule plus grace period, or reported `status=down`. Two minutes after the deadline. |
| **Job failed** (`push_failed`) | The job did call, but its last run failed: `/fail`, or a non-zero exit code. The message carries the exit code and the end of the output. It resolves with the next successful run, and speaks again for each new failure. |
| **Job too long** (`push_too_long`) | A run lasts longer than `max_duration`, or, without it, a `/start` is still open after one period plus the grace period. |

A silent job (nothing arrives) and a failing job (something arrives, and it is
bad) are therefore two different alerts. A misconfigured schedule is shown on
the monitor and refused at save time, never alerted as an outage. The "Service
down" and "Service flapping" rules skip heartbeats, and "Monitor unreachable"
never fires for them: the collector always runs, only its verdict changes.

## API

| Route | Session | Purpose |
|---|---|---|
| `GET` or `POST /api/push/<token>` | none | The call itself. `204`, `404`, `400`, `413` or `429`. |
| `GET` or `POST /api/push/<token>/start`, `/success`, `/fail`, `/<code>` | none | Start or end of a run (see above). Same answers; an unknown suffix is a `404`. |
| `GET /api/targets/<id>/push` | required | The monitor: `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`, `schedule`, `timezone`, `next_expected_at`, `max_duration_secs`, `last_run`. Created on first read if missing. |
| `GET /api/targets/<id>/push/runs?limit=&before=` | required | The run history, newest first: `runs` (`id`, `started_at`, `finished_at`, `duration_secs`, `status` among `running`, `success`, `failed`, `lost`, `exit_code`, `output`) and `next_before` for the next page. |
| `POST /api/targets/<id>/push/regenerate` | admin | New token; the previous URL answers `404` from then on. The counter and last call are kept. |
