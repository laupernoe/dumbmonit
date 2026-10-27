# Hetzner Cloud

The servers of a Hetzner Cloud project through the Hetzner Cloud API: servers
that are not running, a processor that stays saturated, backups that stopped
and volumes attached to nothing.

DumbMonit reads the API at `api.hetzner.cloud/v1` once a minute with a
read-only project token sent as a bearer token. The list of servers is
required; the activity of the servers, the volumes and the backups are
optional parts that, if refused, are skipped without making the monitor fail.
One monitor reads one project (a token belongs to a single project).

This is an Internet API: the address of the device is the API's, not a
machine's. DumbMonit needs outbound HTTPS to `api.hetzner.cloud`; nothing in
your servers is contacted.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the Hetzner Cloud API documentation, not
    against a live project. It only reads. Tell us what breaks.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Hetzner Cloud API (HTTPS, api.hetzner.cloud) |

## What it watches

All metrics are prefixed `dumbmonit_hetzner_`. At most 200 named items are
listed per family; the counts stay exact.

| Metric | Labels | Meaning |
|---|---|---|
| `server_running` | `server` | 1 when the server status is `running`. |
| `server_info` | `server`, `status`, `type`, `location` | Value 1. |
| `server_backups_enabled` | `server` | 1 when automatic backups are enabled. |
| `servers`, `servers_running`, `servers_not_running` | | Counts. |
| `server_cpu_percent` | `server` | CPU usage averaged over the last ten minutes (running servers only). |
| `server_disk_read_bytes_per_second`, `server_disk_write_bytes_per_second` | `server` | Disk bandwidth, same window. |
| `server_network_in_bytes_per_second`, `server_network_out_bytes_per_second` | `server` | Network bandwidth, same window. |
| `metrics_readable` | | 0 when the server metrics cannot be read; absent when the **Servers with metrics** option is 0. |
| `volume_attached` | `volume` | 1 when the volume is attached to a server. |
| `volume_size_gigabytes` | `volume` | Size of the volume. |
| `volumes`, `volumes_unattached` | | Counts. |
| `volumes_readable` | | 0 when the volumes cannot be read. |
| `server_backup_age_seconds` | `server` | Age of the newest backup, for servers with backups enabled and at least one backup. |
| `backups_readable` | | 0 when the backups cannot be read; absent when no server has backups enabled. |

The metrics are read for the first running servers only, as many as the
**Servers with metrics** option says (10 by default, at most 100): each one
costs an API call, and Hetzner allows 3,600 requests per hour and project.
Backups are read from the newest images (`type=backup`), over 5 pages at most.

The [built-in rules](../alerting/rules.md#hetzner-cloud) that apply:

- **Hetzner server not running**: a server that is not in the `running` state
  for 10 minutes (Advisory). A server powered off on purpose raises it too.
- **Hetzner server CPU sustained high**: more than 90% CPU over ten minutes,
  for 30 minutes (Advisory).
- **Hetzner backup too old**: the newest backup of a server with backups
  enabled is more than 3 days old (Advisory, reminded daily).
- **Hetzner volume not attached**: a volume attached to no server for an hour
  (Info, reminded weekly).

## The monitor page

The Hetzner Cloud panel says what is wrong in a sentence, shows the servers
and the servers running, then lists the servers with their state, the
volumes and the backups of the servers that have them.

## Create a read-only API token for DumbMonit

1. In the Hetzner Cloud Console, open the project to monitor, then Security > API tokens > Generate API token.

2. Name it DumbMonit and choose the Read permission (not Read & Write).

3. Click Generate API token, then copy the token: it is shown once.

4. In DumbMonit, use this address and paste the token as API token. One monitor reads one project.
   `api.hetzner.cloud`

### Required permissions

- **Read (project token)** — Lists servers, volumes and backups and reads server metrics; changes nothing.

!!! warning
    A server that is powered off is reported as not running, on purpose or not: mute the rule for it if that is expected. The API allows 3600 requests per hour and project, so server metrics are read for 10 servers by default.

Vendor documentation: https://docs.hetzner.cloud/

## Credentials

| Credential | Fields |
|---|---|
| API token | The project token created above, sent as a bearer token. |

Address: `api.hetzner.cloud`. Options:

- **Servers with metrics**: how many running servers get their CPU, disk and
  network read each minute, from 0 (none) to 100 (10 by default).
- **Timeout per request** (15 s).

## Troubleshooting

"Hetzner Cloud refused the API token (401)": the token is wrong, deleted, or
belongs to another project.

"Hetzner Cloud refused … (403)": the token cannot read that resource.

`metrics_readable`, `volumes_readable` or `backups_readable` is 0: Hetzner
refused or failed one of those calls; the servers are still reported. A
server with backups enabled that has no backup yet raises no alert until its
first one exists.
