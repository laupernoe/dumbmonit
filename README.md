<div align="center">

<img src=".github/assets/logo.svg" alt="DumbMonit logo" width="96" height="96">

# DumbMonit

**Dumb-simple monitoring from the homelab to the small business.**

One container, one IP address to type in, useful graphs and alerts in under a minute.<br>
The UI reads like a weather bulletin for your network. The mascot is a pigeon.

[![CI](https://github.com/laupernoe/dumbmonit/actions/workflows/ci.yml/badge.svg)](https://github.com/laupernoe/dumbmonit/actions/workflows/ci.yml)
[![Documentation](https://readthedocs.org/projects/dumbmonit/badge/?version=latest)](https://dumbmonit.readthedocs.io/en/latest/)
[![Translation status](https://hosted.weblate.org/widget/dumbmonit/web-ui/svg-badge.svg)](https://hosted.weblate.org/engage/dumbmonit/)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![From homelab to small business](https://img.shields.io/badge/from%20homelab-to%20small%20business-6f83a3.svg)](#quick-start)
[![Status: work in progress](https://img.shields.io/badge/status-work%20in%20progress-orange.svg)](#status)

**[Live demo](https://demo.dumbmonit.app)** · **[Website](https://dumbmonit.app)** · **[Documentation](https://dumbmonit.readthedocs.io/en/latest/)**

<img src=".github/assets/screenshots/overview-dark.png" alt="DumbMonit overview: the bulletin sentence, counters, the weather window and the list of things that need you" width="900">

</div>

> **Work in progress.** DumbMonit is under active development; the current
> build is an alpha for early testers (`ghcr.io/laupernoe/dumbmonit:latest`). It
> runs daily on the author's homelab, but expect rough edges and breaking
> changes. Feedback and bug reports are very welcome; see [Status](#status)
> for what is known to be missing.

## Why

**The one-second answer.** The home screen states "Clear skies." or "1 unreachable,
4 building up." before anything else. You open it once a day, or from a
notification, and know whether everything is fine before you have finished
sitting down. Drilling down is a click away; it is never required.

**One way to add anything.** A switch, a Proxmox cluster, a NAS, a Windows box, a
website: pick a type from the list, a notice on the right explains what to
prepare on the device, and the form only shows the fields that matter for that
type. SNMP profiles are detected from the device's `sysObjectID`; defaults do the
rest.

**Quiet by default.** Useful rules are active right after install, and the tool
works hard to send fewer, better alerts: a switch going down produces one
notification instead of thirty, maintenance windows are a first-class object, and
anomaly detection learns your network's rhythm before it says anything.

DumbMonit is for people who watch ten machines and three switches and do not
want to operate Zabbix or Checkmk, nor assemble Prometheus + Grafana +
Alertmanager + exporters. It is 100 % open source under the Apache 2.0 license,
dependencies included (CI fails on a non-OSI dependency): no feature is held
back for a paid edition.

## Features

**Sources**

- **SNMP v1 / v2c / v3** — switches, routers, NAS, UPS, printers. Five profiles
  ship with the product (IF-MIB, HOST-RESOURCES-MIB, UPS-MIB, PRINTER-MIB, system)
  and are applied automatically from the device's `sysObjectID`.
- **Proxmox VE** — nodes, VMs and containers, storage, cluster quorum, and the age
  of the last successful backup per machine (the backup that has not run for
  weeks, discovered before restore day).
- **Proxmox Backup Server** — datastore usage and fill-up forecast, deduplication
  factor, per-machine snapshot age and verification result, failed tasks (backup,
  verify, GC, sync), age of the last garbage collection.
- **Proxmox Datacenter Manager** — the console that federates several PVE clusters
  and backup servers: which instances it still reaches and why one dropped off,
  the whole estate in one page (guests running, nodes online, CPU, memory and
  storage totals), failed tasks across every site, and the console's own health.
- **Proxmox Mail Gateway** — the postfix queues and how long the oldest message
  has been stuck there, the mail counted and filtered today (spam, viruses,
  bounces, greylisting), quarantine sizes, and the age of the antivirus and
  antispam signature databases: the silent failure where the gateway keeps
  filtering with last week's rules. Counts only, never message content.
- **OPNsense** — the open source firewall in front of the house: every gateway
  with its round-trip time and packet loss, so the backup WAN that took over
  three weeks ago stops being a surprise; the pf state table against its limit,
  interface counters and the public address of the moment, VPN tunnels up or
  down, DHCP leases, the resolver, CARP (including the maintenance mode everyone
  forgets to switch off) and pending updates. Read-only, through an API key.
- **Server hardware (Redfish)** — what the management controller (Supermicro,
  iDRAC, iLO, XClarity, ASRock Rack) knows about the server it sits in: fans,
  temperatures against the thresholds the hardware declares itself, power
  supplies and their redundancy, drives and their predicted failures, memory and
  processor health, and the event log counted by severity. An empty slot is
  never a failure. Read-only account, basic authentication by default.
- **Active Directory** — a Windows domain read over LDAP with a plain,
  unprivileged domain account: domain controllers and FSMO roles, privileged
  group membership, password policy, and the raw facts a domain audit looks
  for first — Kerberoastable and AS-REP roastable accounts, unconstrained
  delegation, an old krbtgt password, computers without LAPS, systems out of
  support. One device covers the whole domain.
- **MDaemon Email Server and SecurityGateway** — the Windows mail server and
  mail gateway from MDaemon Technologies: SMTP, IMAP, POP3 and webmail checked
  from the outside, greeting included, with no account at all; the version
  through MDaemon's XML API, and on SecurityGateway 12.5 and later the
  performance counters of its REST API (delivery queue, quarantine, sessions).
  Built from the published documentation, not yet tried on a live server.
- **Nextcloud, Immich, Paperless-ngx, Jellyfin and Plex** — the self-hosted
  applications that fail sideways: Nextcloud left in maintenance or waiting
  for `occ upgrade`, Immich queues paused or stalled while uploads still
  succeed, Paperless with Redis or Celery gone and nothing imported, the
  Jellyfin library scan failing every night; updates, disk space, streams and
  transcodes. Each through its own API with the narrowest access it allows (a
  serverinfo token, a five-permission Immich key, a Paperless user that reads
  no document).
- **GitLab (self-managed) and Forgejo/Gitea** — a Git forge's health: Sidekiq
  queue backlog and latency on GitLab, scheduled tasks and Actions runners on
  Forgejo/Gitea, CI/CD runners online, pending database migrations and, for
  GitLab Enterprise Edition, the licence. An ordinary access token proves the
  instance is up; administrator-only endpoints are skipped, not failed,
  without one.
- **Nginx and Apache** — the web server itself, through `stub_status` or
  `mod_status`: active/idle workers or connections, accept/handled/request
  rates, and, with the commercial NGINX Plus, upstream health and per-zone
  traffic. No account needed.
- **TrueNAS** — the ZFS pool that lost a disk and still serves its data (the
  failure nobody notices until the second disk), with the disk named; scrubs and
  resilvers, pool and dataset usage against quotas, snapshots and replication
  tasks, disk temperature and SMART self-tests, and the alerts TrueNAS raises
  itself. Nothing is ever started: no scrub, no test, no update check.
- **Synology DSM** — volumes, disks and SMART health, temperature, load and
  services, through the NAS web API; **Active Backup for Business** tasks, their
  last result and the age of the last success; whether **Synology Drive** and
  **Synology Photos** are installed and serving, with Drive's team folders
  and active connections.
- **Linux, macOS, FreeBSD and Windows agent** — CPU, memory, disks, network,
  services and uptime for machines that do not speak SNMP, plus **temperatures
  and fans**, **disk health (SMART)** and **ZFS pools** where the machine
  exposes them. One-line install, which registers the agent with systemd,
  OpenRC, launchd or rc.d. Binaries served by the server (Linux x86_64 /
  aarch64, FreeBSD x86_64, Windows x86_64); the macOS binaries are attached to
  each release, Apple's SDK not being redistributable.
- **Docker, through the agent** — container state, health, restarts, image age
  and available updates; opt-in per container: restart when down, update
  automatically (pull, recreate, health check, rollback) inside a maintenance
  window, prune the old image. **Plakar** backups: age and result of the last
  snapshot per kloset.
- **Service monitors**, Uptime Kuma style — HTTP(S) (status code, keyword, JSON
  path, certificate), TCP port, DNS resolution (asserting the record type, the
  expected values, and the ones that must never come back), ping, TLS
  certificate expiry and NTP (offset, stratum, leap indicator, for a router or
  a chrony/ntpd server acting as a local time source), each with its history
  bar, response time and availability percentage.
- **Application monitors** that begin a real session instead of knocking on a
  port — **SMTP** (greeting, EHLO, STARTTLS, AUTH: does your relay still accept
  you?), **PostgreSQL** and **MySQL/MariaDB** (connect, authenticate, run a
  query, with connection time and query time apart — the "up but slow" signal),
  **MQTT** (connect, subscribe, optionally read a retained message) and
  **WebSocket** (the upgrade handshake, which fails where a plain `GET` still
  answers 200). A refused password is reported as such, never as an outage.
- **Heartbeats** (dead man's switch) — a cron job, backup script or Home
  Assistant automation calls a secret URL each time it runs; if it stops
  calling, you are told. Uptime Kuma push-compatible (`?status=down&msg=`).
- **Website change detection** — watch a page, or a whole site crawled from its
  sitemap or its links, for changes in its visible text: a pricing page, a
  documentation site, a supplier's maintenance notice. Each change keeps the
  text before and after, a line diff and, when a browser is available, a
  screenshot of each version.
- **Client device tracking** — last-seen and last-backup per machine for
  products that serve many clients: Immich, Proxmox Backup Server and Veeam
  protected machines, Tailscale, UniFi and the Home Assistant companion app,
  each with a stale-device alert when one goes quiet.
- **Network discovery** — sweep a CIDR and add everything that answers in one go.
- **Integration packs** — a device kind declared in a single YAML file (an HTTP
  API or a Prometheus `/metrics` page on the device, extracted by JSONPath,
  regular expression or metric family), with its own alert rules, installed
  from *Settings → Integration packs* without a server release. Reference
  packs ship for Prometheus node_exporter, Shelly Gen2+ plugs and Speedtest
  Tracker. See [Integration packs](https://dumbmonit.readthedocs.io/en/latest/packs/).

**Alerting**

- Default rules: device unreachable, CPU saturated, disk almost full, disk full
  soon (extrapolation), UPS on battery, battery low, backup too old, service
  down / flapping / slow, certificate expiring soon or expired.
- **Dependency suppression** — declare a device as the parent of others; when the
  parent goes down, its descendants' alerts are suppressed instead of sent.
- **Grouping by host**, deduplication, periodic reminders and escalation.
- **Acknowledge, snooze, ignore or clear** — **Acknowledge** ("I know, stop
  reminding me") quiets reminders for four hours by default (up to thirty
  days) with an optional note, while the condition keeps being tracked;
  resolution is still announced, and clears the acknowledgement, so the same
  alert notifies again if it comes back. **Snooze** does the same without
  claiming you are working on it. **Ignore** turns a rule off for one device
  for good, shown as "Ignored on N devices" on the rule. **Clear** drops a
  resolved transition from History, one at a time or all at once; *Show
  cleared* brings it back. Public status pages ignore acknowledgements and
  snoozes — an acked or snoozed alert is still an alert for the outside world.
- **Maintenance windows**, one-off or weekly.
- **Notification policy** — hysteresis (trigger/clear thresholds), flap hold,
  per-channel cooldown, quiet hours, batching and an hourly cap, so a bad night
  does not turn into two hundred pushes.
- **Seasonal baseline** — anomaly detection with no threshold to tune: 168 buckets
  (hour × day of week) learn the usual behaviour; silent for its first 14 days,
  showing only what it *would* have fired.
- **Forecasts** — predictive rules (linear extrapolation computed by
  VictoriaMetrics: disk full soon, datastore full soon) are listed as forecasts on
  the overview, apart from what is broken right now.
- **22 notification channels** — Discord, Slack, Microsoft Teams, Telegram,
  Matrix, Mattermost, Rocket.Chat, Google Chat, ntfy, Gotify, Pushover, Pushbullet,
  Bark, Signal, Twilio (SMS), PagerDuty, Opsgenie, Home Assistant, Zulip, Apprise,
  email (SMTP) and a custom webhook. Each has a "Test" button; setup notes live in
  [the documentation](https://dumbmonit.readthedocs.io/en/latest/notifications/).

**Interface**

- Light and dark themes, following the system by default.
- **Wall mode** (`/wall`) — the bulletin alone, full screen, for a room monitor:
  a living Paris rooftop with the Eiffel Tower, lit windows and the DumbMonit
  pigeons going about their day, gentle and constant motion with nothing that
  flashes; an **OLED theme** (true black, dimmed text, a slow pixel shift
  against burn-in) next to Auto/Day/Night; and **wall music** — Spotify
  Connect turns the screen itself into a self-diagnosing, self-reconnecting
  speaker, or send any screen a Spotify, Deezer, YouTube or YouTube Music link
  to play and show what is on.
- **Security score** — every supported device gets a 0–100 score and an A–F
  grade from the vendor's own best-practice checks, PingCastle- and Secure
  Score-style: category, severity, pass/fail/unknown, evidence and
  remediation, a worst-first **Security** page, the score itself as a metric,
  and built-in rules for a score that drops or stays low.
- **⌘K / Ctrl K palette** — jump to any page, device or action.
- Every device is a 1U faceplate: LED, name, kind, address, last seen, sparkline;
  children stack under their parent and dim when it is unreachable. Devices can
  be grouped into **folders**, reordered by drag-and-drop (mouse, touch or
  keyboard), renamed and deleted in place.
- Mobile works for reading state, silencing an alert and scheduling maintenance.
- **Public status pages** (`/s/<slug>`) — groups of monitors with a 90-day
  daily history and 30/90-day uptime, incidents and maintenance, your logo and
  accent, email subscribers (double opt-in) and RSS, status, uptime and
  response-time badges for a README, and a compact embed for an intranet page.
- **Accounts** — admin and viewer roles, an optional **TOTP second factor**
  with recovery codes, an audit log of sign-ins and account changes, plus
  **OIDC / SSO** (Authentik, Authelia, Keycloak, Pocket ID…) with
  group-to-role mapping and account creation on first sign-in off by default.
- **Open HTTP API** — the same API behind the whole UI, described at
  `/api/openapi.json` (OpenAPI 3.1) for Swagger UI, Postman or a generated
  client. Tokens (`dmt_…`), created in *Settings → API & assistants*, are
  scoped `read` or `write`, can expire (30 days to never), be restricted to a
  list of networks, and are rate-limited; CORS opens them to a browser-based
  dashboard. Accounts, tokens and backups stay off limits to them. See
  [the API reference](https://dumbmonit.readthedocs.io/en/latest/reference/api/).
- **Install it on a phone** — a web manifest and home-screen icons; there is
  deliberately no offline mode, so the screen never shows yesterday's state.
- **Built-in MCP server** — connect Claude Code, Claude Desktop, ChatGPT, VS
  Code or Cursor with a scoped token and ask "is everything fine?" or "silence
  the NAS for an hour". 27 tools: a `read` token only looks (status, devices,
  alerts, metrics, agents, containers…); a `write` token can also act —
  silence or acknowledge, add a device, restart a container, post a
  status-page incident. Credentials and other secrets are never returned. See
  [Connect an assistant](https://dumbmonit.readthedocs.io/en/latest/using/assistant/).
- **Readable by the Prometheus or Grafana you already run** — `GET /metrics`
  exposes the instance's own health, `GET /federate` the measurements by
  selector, and `/prometheus` answers as a Prometheus data source, all behind a
  read-only API token.
- **Backup and restore** — the whole configuration, credentials included, as
  one file encrypted with a passphrase you choose, which restores onto a fresh
  instance with a dry run first; plus a daily online copy of the database and
  of `secret.key` in `/data/backups/`, and a built-in rule when it stops
  happening.

## Quick start

```bash
mkdir dumbmonit && cd dumbmonit
curl -fsSLO https://raw.githubusercontent.com/laupernoe/dumbmonit/main/docker-compose.yml
docker compose up -d
```

That pulls `ghcr.io/laupernoe/dumbmonit:latest` (amd64; arm64 images are paused for now). To run from
source instead, clone the repository and use `docker compose up -d --build`
(about ten minutes; Docker is the only requirement).

Then open http://localhost:8080. The first visit lands on `/setup`, where you
create the first admin account with the one-time setup code the server prints
in its logs (`docker compose logs dumbmonit`). The overview then walks you through three
steps — add a device, connect a notification channel, check that a message
arrives — and each one is a single click. Add a device with its IP address and
SNMP community: the collection profile is detected automatically.

- **Another port**: `DUMBMONIT_PORT=8099 docker compose up -d` (8080 is busy on
  most homelab machines).
- **Update**: `docker compose pull && docker compose up -d` (from source:
  `git pull && docker compose up -d --build`).
- **Lost password**: another admin can set a new one in *Settings → Users*. If
  no admin can sign in, `DUMBMONIT_RESET_PASSWORD=1 docker compose up -d`
  removes every account and session at startup — devices, rules and channels are
  untouched — and the UI asks you to create the first admin again at `/setup`,
  with the new setup code from the logs.
  Then run `docker compose up -d` again without the variable.
- **ICMP ping monitors** work without any capability: the container runs as a
  non-root user and `docker-compose.yml` sets the `net.ipv4.ping_group_range`
  sysctl that allows ICMP echo sockets. Keep those lines.
- **Backup**: *Settings → Backup* downloads the whole configuration as one
  encrypted file and restores it, and the server keeps a daily copy of its
  database in `/data/backups/`. `/data/secret.key` is what decrypts device
  credentials — a backup without it restores an instance that cannot talk to
  anything. See [Backup and restore](https://dumbmonit.readthedocs.io/en/latest/install/backup/).

### Installing the agent

Create a token in *Settings → Agents*; the UI shows the install command with the
token filled in:

```sh
curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
```

The same command installs the agent on Linux, macOS and FreeBSD: it detects the
system and registers the service with systemd, OpenRC, launchd or rc.d. On
macOS, download the binary from the
[releases page](https://github.com/laupernoe/dumbmonit/releases/latest) and add
`--bin=./dumbmonit-agent-macos-aarch64`. A PowerShell script is served at
`/install.ps1` for Windows. Every install command carries the expected
SHA-256 of the binary, and the script refuses a download that does not match
it, has no checksum, or cannot be hashed (`--insecure-skip-checksum` /
`-InsecureSkipChecksum` is the explicit escape hatch for air-gapped setups).

The agent registers itself as a device. A host that was enrolled before agent
binding existed, and never bound, is refused once its transition window
closes; the device page explains how to re-enrol it.

The agent is also published as an image, `ghcr.io/laupernoe/dumbmonit-agent`
(same tags as the server), for Docker hosts and **remote sites**: with
`DUMBMONIT_AGENT_RELAY=true` the agent runs, on the server's behalf, the
probes of the devices you assign to it (SNMP, Proxmox, HTTP…) from its own
network, so several sites show up in one DumbMonit — outbound only, no VPN.
See `docker-compose.agent.yml` and [Monitor a remote site](docs/install/remote-site.md).

## What it looks like

| | |
|:-:|:-:|
| <img src=".github/assets/screenshots/devices-light.png" alt="Devices page, light theme: every device is a rack faceplate with a status LED and a sparkline" width="440"> | <img src=".github/assets/screenshots/add-device-dark.png" alt="Add a device, dark theme: the SNMP form on the left and the setup notice on the right" width="440"> |
| Devices, stacked like a rack | Add a device: type, notice, relevant fields only |
| <img src=".github/assets/screenshots/alerts-light.png" alt="Alerts page, light theme: firing alerts grouped by host" width="440"> | <img src=".github/assets/screenshots/wall-dark.png" alt="Wall mode, dark theme: the bulletin full screen for a room monitor" width="440"> |
| Alerts, grouped by host | Wall mode for a room monitor |
| <img src=".github/assets/screenshots/device-proxmox-light.png" alt="A Proxmox VE device, light theme: the Guests panel lists every VM and container with status, CPU, memory, disk, network, uptime, last backup and HA state" width="440"> | <img src=".github/assets/screenshots/device-proxmox-dark.png" alt="The same Proxmox VE device page in the dark theme" width="440"> |
| Proxmox VE: every guest at a glance | The same page, dark theme |

More screenshots, in both themes, in [`.github/assets/screenshots/`](.github/assets/screenshots/).

## What runs

| Container | Role | Footprint |
|---|---|---|
| `dumbmonit` | Collection, API, alerting, web UI, and the embedded VictoriaMetrics for time series | ~40 MB RAM + the VictoriaMetrics budget (256 MB by default) |

One container, one volume. The image ships the VictoriaMetrics binary and the
server runs it as a child process; set `DUMBMONIT_VM_URL` to use an instance
you already have instead. Configuration and state live in an embedded SQLite
database: there is no database container. The published image is
`ghcr.io/laupernoe/dumbmonit` (`latest` = last tagged build, `edge` = last
commit on `main`, or a version such as `0.1.0-alpha.1`).

## Configuration

Everything goes through environment variables; none is required.

| Variable | Default | Role |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Listen address |
| `DUMBMONIT_DATA_DIR` | `/data` | SQLite database, instance secret, time series |
| `DUMBMONIT_VM_URL` | *(unset)* | External VictoriaMetrics; unset, the embedded one is started |
| `DUMBMONIT_VM_RETENTION` | `12` | Retention of the embedded VictoriaMetrics (months, or `30d`, `2y`) |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Memory budget of the embedded VictoriaMetrics |
| `DUMBMONIT_SECRET` | *(generated)* | Encrypts device credentials |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Concurrent probes |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Maximum duration of one probe |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Write period towards VictoriaMetrics |
| `DUMBMONIT_LOG` | `info` | Log filter (`tracing` syntax) |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Agent binaries served under `/download/…` |
| `DUMBMONIT_RESET_PASSWORD` | *(empty)* | Set to `1` to remove every account and session at startup |
| `DUMBMONIT_SETUP_CODE` | *(generated)* | Code asked by `/setup` while no admin exists; unset, a random one is printed in the logs at each start |

The `DUMBMONIT_PORT` variable is read by `docker-compose.yml` only and sets the
host port (default `8080`). The former `EZYMONIT_*` names are still accepted,
with a deprecation warning; see [CHANGELOG.md](CHANGELOG.md) for what the
rename changes.

### About the instance secret

SNMP communities, API tokens and passwords are encrypted with AES-256-GCM using a
key derived from the instance secret. On first start, DumbMonit generates this
secret in `/data/secret.key`.

**Back this file up together with the database.** Without it, device credentials
are unrecoverable. The server detects this at startup and refuses to continue
with an explicit message, rather than failing silently on every probe.

## Architecture

One Rust binary (which embeds the SvelteKit build) plus VictoriaMetrics for time
series, started by the server from the same image, and SQLite for configuration
and state.

```
crates/proto     shared types: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/collectors  snmp (profiles/*.yaml), proxmox, pbs, pdm, pmg, synology, opnsense, truenas, redfish, mdaemon, selfhosted, uptime — shared by the server and the relay agent
crates/pack      integration packs: declarative YAML device types (HTTP or Prometheus
                 sources), validated and run like any collector; the `pack lint` / `pack test` CLI
crates/server    the binary
  api/           axum routes; spa.rs serves the embedded web UI
  auth/          accounts and roles, HttpOnly session cookie, TOTP, OIDC, API tokens, rate limit
  collectors/    the agent collector (pushed metrics, commands, tokens) and the relay hub; re-exports crates/collectors
  scheduler.rs   runs every enabled target on its interval through the collector registry, or delegates it to its relay agent
  tsdb/          VictoriaMetrics writer (batched flush) + query proxy
  db/            SQLite + numbered migrations
  alerting/      rules, state machine, suppression by parent, silences, seasonal baseline
  notify/        22 notification channels, described to the UI by notify/catalog.rs
  crypto.rs      AES-256-GCM for credentials/tokens
crates/agent     Linux/macOS/FreeBSD/Windows agent + install scripts; relay mode runs the shared collectors remotely
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), static build embedded in the binary
profiles/        SNMP collection profiles, auto-applied by sysObjectID
packs/           reference integration packs (node-exporter, shelly-plug, speedtest-tracker),
                 each with its fixtures and expected output
```

Adding an integration means implementing the `Collector` trait
(`crates/proto/src/collector.rs`) and registering it: the scheduler and the API
know nothing about the concrete types. The UI is data-driven — the server
describes every device kind and every notification channel, so a new kind needs
no UI release.

## Documentation

The user guide lives at **[dumbmonit.readthedocs.io](https://dumbmonit.readthedocs.io)**:
installation, every source and notification channel, alerting, the agent, and
the HTTP API.

## Roadmap

What is planned, in progress and recently shipped is on the public
**[DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2)**. Priorities
can change; suggest an idea by opening an issue.

## Contributing

Bug reports, device profiles and new integrations are welcome. Read
[CONTRIBUTING.md](CONTRIBUTING.md) for the development setup (Docker only, no
Rust toolchain needed), the conventions, and how to add a collector or a
notification channel. Design rules for the UI are in [DESIGN.md](DESIGN.md) and
the product principles in [PRODUCT.md](PRODUCT.md).

The web UI can be **translated on [Weblate](https://hosted.weblate.org/engage/dumbmonit/)**,
no code needed: pick a language, translate, and Weblate opens the pull request.
English is the only shipped language so far.

Please report security issues privately: see [SECURITY.md](SECURITY.md).
This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## Status

**Work in progress — alpha.** DumbMonit is developed in the open and
used daily on the author's own homelab, but it is not ready for anyone who needs
it to be boring: the HTTP API is not frozen, the database schema still moves,
and some integrations have only been exercised against simulated devices, not
the real hardware. Known gaps and open bugs are tracked in the
[issues](https://github.com/laupernoe/dumbmonit/issues). Heartbeat monitors,
scoped API tokens covering the whole REST API and the TOTP second factor have
since shipped, as has backup and restore; there is still no upgrade guarantee
across schema changes. The project was called
EzyMonit until September 2026: `EZYMONIT_*` variables and the old agent
installation are still accepted and migrated.

## License

Apache 2.0 — see [LICENSE](LICENSE).
