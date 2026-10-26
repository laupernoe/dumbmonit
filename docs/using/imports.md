# Import from other tools

**Add a monitor → Import** brings your monitors over from another tool. All the
importers work the same way: you give the source, DumbMonit shows a **preview**
(what will be created, what is skipped and why), you tick the ones you want, then
confirm. A monitor that already exists (same type and address) is never duplicated.
Administrators only.

What is **never** imported: notification channels, passwords and tokens, custom
headers and request bodies. Set those up again in DumbMonit.

| From | Source | Becomes |
|---|---|---|
| Uptime Kuma | `kuma.db`, or the JSON backup of 1.x | see [Monitors page](devices.md) |
| UptimeRobot | read-only API key, or CSV export | `http`, `ping`, `tcp`, heartbeat |
| Gatus | `config.yaml` | `http`, `tcp`, `ping`, `dns`, heartbeat |
| Healthchecks.io | API key (healthchecks.io or self-hosted) | heartbeat (push) |

## UptimeRobot

1. In UptimeRobot: **Integrations → API → Read-only API key**.
2. In DumbMonit: **Import from UptimeRobot**, paste the key, **Preview**. The key is used
   once to list your monitors and is never stored.

Alternative: load the CSV exported from the dashboard (columns *Friendly Name*,
*URL / IP*, *Type*, *Port*, *Interval*, *Keyword*; the order does not matter).

| UptimeRobot | DumbMonit |
|---|---|
| HTTP(s) | `http` (method kept) |
| Keyword | `http` with the keyword (and "must not appear" when set) |
| Ping | `ping` |
| Port | `tcp` |
| Heartbeat | [heartbeat](../devices/push.md) (new URL) |

Paused monitors stay paused.

## Gatus

Paste or load your `config.yaml`.

- `endpoints` with an `http(s)://` URL become `http` monitors. `[STATUS] == 200`
  and `[BODY].path == value` conditions are kept; the other conditions
  (`[RESPONSE_TIME]`, …) are listed in the row's message.
- `tcp://host:port` becomes `tcp`, `icmp://host` becomes `ping`, an endpoint with
  a `dns:` block becomes `dns`.
- `external-endpoints` become heartbeats (new URL).
- `group` becomes the folder; `interval` and `enabled` are kept. Other schemes
  (`starttls://`, `ssh://`, …) are skipped with a reason.

## Healthchecks.io

1. In Healthchecks: **Project Settings → API Access**. A *read-write* key also
   lists each check's current ping URL; a read-only key works but the old URLs are
   not shown.
2. In DumbMonit: **Import from Healthchecks.io**, paste the key (and the address of
   your server if it is self-hosted), **Preview**.

Each check becomes a [heartbeat monitor](../devices/push.md): the expected interval
is the check's *period* (or an approximation of its cron schedule, flagged in the
row), the *grace time* is kept, tags become tags, a paused check stays paused.

### Redirecting the old ping URLs

A heartbeat monitor has a **new** secret URL. The job that used to call
`https://hc-ping.com/<uuid>` must now call the DumbMonit URL (shown on the monitor
page). Two ways:

- edit the job (crontab, script) — the clean way;
- or, if you self-host Healthchecks on a domain you control, keep the old
  address answering with a redirect to the new one, for example with nginx:

  ```nginx
  location = /ping/<old-uuid> {
      return 307 https://monit.example.org/api/push/<new-token>;
  }
  ```

  `307` keeps the method and body. `hc-ping.com` itself is not yours to redirect:
  there, the jobs have to be edited.

The row's message lists the old ping URL for each check, so you can search your
scripts for it.
