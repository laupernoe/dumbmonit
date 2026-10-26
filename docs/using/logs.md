# Logs: agents, syslog and SNMP traps

DumbMonit can keep the log lines of your machines and equipment next to their
metrics, search them, and alert on patterns. Everything lands in the embedded
VictoriaLogs and is searched on the admin-only **Logs** page with
[LogsQL](https://docs.victoriametrics.com/victorialogs/logsql/).

!!! note "Turn the log store on first"
    **Settings → System → Log store**. While it is off, received lines are
    dropped (and counted), and the Logs page says so.

## What can feed it

| Source | Stream (`service`) | How |
|---|---|---|
| DumbMonit itself | `dumbmonit` | Always, once the store is on. |
| Agents | `agent` | Per machine, from its device page. |
| Syslog | `syslog` | The built-in server (UDP and TCP). |
| SNMP traps | `trap` | The built-in receiver (v1, v2c). |

Every line has `level` (`error`, `warn`, `info`, `debug`), `host`, `origin`
(journald unit, file, event log channel or sender address) and, when the
sender is one of your devices, `target` (its id) and `target_name`.

## Agent log collection

Open an agent device and use **Log collection**. It is off until you switch it
on; the agent applies the setting at its next report, with no restart.

- **systemd journal** (Linux): new entries from now on, filtered by priority.
- **Followed files**: absolute paths with `*` and `?` in each component, for
  example `/var/log/nginx/*.log`. Rotation and truncation are followed; lines
  already in a file when collection starts are not replayed. Levels are guessed
  from the words `error`, `warn`, `debug`.
- **Windows event log channels**: `System`, `Application`… read through
  PowerShell every 15 seconds.
- **Minimum level**: lines below it are never sent.

Lines travel over the agent's usual channel (same server, same port, same
token or certificate) in batches. If the server cannot be reached the agent
keeps up to 8 MiB on disk (next to its binding secret) and sends them when it
comes back; beyond that, the oldest are dropped. Set `DUMBMONIT_AGENT_LOGS=off`
in the agent's environment to forbid collection on a machine whatever the
server asks.

!!! warning
    The paths come from the server, and the agent usually runs as root. Only
    administrators can set them, but treat the setting like the agent's command
    channel.

## Syslog server

Off by default. Set `DUMBMONIT_SYSLOG_ENABLED=true`, publish the port in your
Compose file (`5514/udp` and `5514/tcp`, unprivileged because the container
does not run as root), and point your equipment at it. RFC 3164 and RFC 5424
are understood, with newline-delimited or octet-counted TCP framing. The
sender is attached to the device whose address is the source IP (or, failing
that, whose name is the hostname in the message). Port 514 can be redirected to
5514 on the host or the router. Variables: see
[Configuration](../reference/configuration.md#central-logs-syslog-server-and-snmp-traps).

## SNMP traps

Off by default (`DUMBMONIT_TRAPS_ENABLED=true`, port `1162/udp`). SNMP v1 and
v2c traps and v2c informs (acknowledged) are received; **v3 is not supported**
and such messages are counted as refused. Set `DUMBMONIT_TRAPS_COMMUNITIES` to
accept only your communities. Each trap becomes a line named after the trap
(`linkDown`, `coldStart`…, or the OID) followed by its variable bindings, which
are named after the built-in SNMP profiles when they know the OID.

## Searching

The **Logs** page adds a **Source** (server, agents, syslog, SNMP traps, all)
and a **Device** filter to the period, minimum level and LogsQL query. A device
page has a **Logs** button that opens the page already filtered. The
**Receivers** block shows what listens and what arrived.

## Alerting on a pattern

Open **Alert on this search** below the search form: it creates a *log pattern*
rule from the current source, level and query. The rule counts matching lines
over a sliding window (1 minute to 24 hours) and fires when the count reaches
your threshold, **per device** (one alert per `target`) or overall. It is a
normal alert rule: severity, hold time, channels, silences, dependency
suppression, history and backups all apply, and it can be edited on the
**Alerts** page.

Examples (the query part):

- `service:=syslog "Failed password"` — failed logins reported through syslog;
- `service:=trap level:in("error", "warn")` — link-down and authentication
  traps, which is how you turn a trap into an alert;
- `service:=agent origin:"file:/var/log/nginx/error.log"` — nginx errors.

Rules only accept filters (no `|` pipes). A rule watching a log source that
stays silent never fires: a count of zero is "no problem", so use `>=` with a
threshold of at least 1.
