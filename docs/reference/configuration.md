# Configuration

Everything goes through environment variables; none is required. There is no
configuration file to mount.

## Server

| Variable | Default | Role |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Listen address of the API and the web UI, inside the container. |
| `DUMBMONIT_DATA_DIR` | `/data` | Persistent directory: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (embedded VictoriaMetrics) and `webchange/` (screenshots of website changes). |
| `DUMBMONIT_VM_URL` | *(unset)* | Base URL of an external VictoriaMetrics (`http://host:8428`). When set, the embedded one is not started; see [below](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(generated)* | Instance secret. If empty, read from `<data dir>/secret.key`, generated on first start. Encrypts device credentials, channel secrets and agent tokens with AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Maximum simultaneous probes, all collectors combined. |
| `DUMBMONIT_WEBSITE_MAX_CONCURRENT` | `10` | Maximum simultaneous [website health](../devices/website.md#pacing-hundreds-of-sites) checks, 1 to 256, within the limit above. |
| `DUMBMONIT_WEBSITE_MAX_PER_HOST` | `1` | Maximum simultaneous website checks towards one domain (`www.example.com` and `shop.example.com` count as one), 1 to 16. |
| `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` | `2` | Minimum seconds between two website checks towards one domain, 0 to 3600. A site's `host_spacing_seconds` option overrides it after that site's check. |
| `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` | `360` | Default period, in minutes, of the website metadata comparison (title, description, canonical, final address, `robots.txt`), 0 to 10080; 0 compares at every check. Up/down is still checked at every polling interval. A site's `change_check_minutes` option overrides it. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Hard timeout of one probe. Service monitors have their own, shorter `timeout_seconds` option. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Period of the batched writes towards VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Number of buffered samples that triggers a write before the period elapses. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Threads of the async runtime. The server spends its time waiting on the network: four cover a homelab, raise it only for hundreds of devices. |
| `DUMBMONIT_DB_POOL` | `4` | Maximum open SQLite connections. Each one costs a thread and a 2 MB page cache; idle connections close after five minutes. |
| `DUMBMONIT_LOG` | `info` | Log filter, `tracing` syntax: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Directory of the agent binaries served under `/download/`. Empty or missing: the install command fails with `404`, the server still runs. |
| `DUMBMONIT_RESET_PASSWORD` | *(off)* | `1`, `true`, `yes` or `on`: clear the password and every session at startup. The UI then shows `/setup` again. Remove it afterwards. |
| `DUMBMONIT_COOKIE_SECURE` | *(off)* | `1` to always set the `Secure` attribute on the session cookie. Only behind HTTPS: over plain HTTP the browser would never send the cookie back. Without it, the attribute is still added whenever the browser is on an `https://` page (its `Origin` says so, or a trusted proxy sends `X-Forwarded-Proto: https`), so plain HTTP on the LAN keeps working. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(empty)* | Reverse proxies whose `X-Forwarded-For`, `X-Forwarded-Proto` and `X-Forwarded-Host` are believed, as addresses or CIDR ranges separated by commas (`10.0.0.5, 172.16.0.0/12`). The login rate limiter and the security log then see the real client address instead of the proxy's; without it, every visitor behind the proxy shares one bucket. Status page links (RSS feed, subscription emails) and the single sign-on return address also use the forwarded host only from these proxies. The headers are ignored from any other address, so a client cannot pick its own bucket or the address of a link. |
| `DUMBMONIT_SCAN_SUBNETS` | *(empty)* | Networks offered by the device form's network scan, as IPv4 CIDR ranges separated by commas (`192.168.1.0/24, 10.0.5.0/24`). In Docker bridge mode the server only sees the Docker network, not your LAN, so set this to pre-fill the scan with the right one. Unset: the scan suggests the /24 networks of the devices you already monitor, then the server's own networks (Docker bridges excluded), and otherwise leaves the field empty. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(off)* | `true` to create an account for an unknown identity at its first single sign-on. Off, only existing accounts (already linked, or matched by a provider-verified email) can sign in. Like the other `DUMBMONIT_OIDC_*` variables, read only while no single sign-on settings are saved in the interface. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(off)* | `1` to accept an OpenID Connect issuer in plain `http://`. Off, the settings screen refuses anything but `https://`: a clear-text issuer hands the authorization code and client secret to whoever listens on the network. Only for a test provider on the loopback. |
| `DUMBMONIT_SETUP_CODE` | *(generated)* | Code `/setup` asks for while no administrator exists. Unset, a random one is printed in the server log at each start; set it for an automated deployment. See [Install with Docker](../install/docker.md). |
| `DUMBMONIT_PUBLIC_URL` | *(unset)* | Address browsers reach DumbMonit at (`https://monit.example.org`). Used for links in notifications and status-page emails, the single sign-on redirect URI and the contact given to push services (HTTPS only). A value saved in the interface (notification policy, SSO settings) wins. |
| `DUMBMONIT_API_CORS_ORIGINS` | *(any origin)* | Origins allowed to call the API cross-origin with a bearer token, and to reach the MCP endpoint: a comma-separated list, or `off` for none. See [HTTP API](api.md#cross-origin-requests-cors). |
| `DUMBMONIT_OIDC_ISSUER` | *(unset)* | Issuer URL of the OpenID Connect provider; discovery is read from `{issuer}/.well-known/openid-configuration`. Must be `https://` unless `DUMBMONIT_OIDC_ALLOW_HTTP` is set. |
| `DUMBMONIT_OIDC_CLIENT_ID` | *(unset)* | Client id registered with the provider. |
| `DUMBMONIT_OIDC_CLIENT_SECRET` | *(unset)* | Client secret. |
| `DUMBMONIT_OIDC_PROVIDER_NAME` | `SSO` | Label of the "Continue with …" button. |
| `DUMBMONIT_OIDC_SCOPES` | `openid profile email` | Scopes requested, separated by spaces. |
| `DUMBMONIT_OIDC_GROUPS_CLAIM` | `groups` | Token claim listing the user's groups. |
| `DUMBMONIT_OIDC_ADMIN_GROUPS` | *(empty)* | Comma-separated groups whose members become administrators. |
| `DUMBMONIT_OIDC_OPERATOR_GROUPS` | *(empty)* | Comma-separated groups whose other members become operators. See [Single sign-on](../using/settings.md#single-sign-on). |
| `DUMBMONIT_METRICS_PUBLIC` | *(off)* | `1` to serve `/metrics`, `/federate` and `/prometheus/api/v1/…` without an API token. Anyone who can reach the port then reads every measurement of every device and the state of the instance; only set it where something else already restricts access to the port. See [Metrics](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Alert evaluation period. Values below 10 are raised to 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Retention of alert history, in days. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Scheduled local backups of the database. `0`, `false`, `no` or `off` to stop writing them. See [Backup and restore](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` stops the daily look at the latest GitHub release (air-gapped instances). See [Update](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Where the scheduled backups are written. Point it at a second volume to survive losing the first. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Hours between two backups, 1 to 8760. The first one happens one interval after startup, not at startup. |
| `DUMBMONIT_DEMO` | *(off)* | `1` for a public, read-only demo with a fixed fictional estate, sign-in `demo` / `demo`. **Recreates the database at every start**: never on a real data volume. See [Run a demo instance](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | How many backups are kept, 1 to 365. The oldest are removed with their `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(unset)* | Headless Chromium used for the screenshots of [website changes](../devices/webchange.md#screenshots), as `http://host:9222` (its DevTools port). Add a `chromedp/headless-shell` service to your Compose file, as shown in [Screenshots](../devices/webchange.md#screenshots). Unset: text comparison only. |

Baseline retention (60 days) and the 14-day learning period are not
configurable.

### Former `EZYMONIT_*` names

DumbMonit was called EzyMonit until September 2026. Every `DUMBMONIT_*`
variable is also read under its old `EZYMONIT_*` name, as a fallback, with one
warning at startup per variable still set that way (`EZYMONIT_X is deprecated,
use DUMBMONIT_X`). The old database file `/data/ezymonit.db` is renamed to
`dumbmonit.db` at startup, and agent tokens `ezym_…` remain valid (new ones are
`dmon_…`). The session cookie is now `dumbmonit_session`: the first start after
the rename logs everyone out once.

### Embedded VictoriaMetrics

The image ships the VictoriaMetrics binary. Unless `DUMBMONIT_VM_URL` is set,
the server starts it as a child process at startup, forwards its log lines into
the server log, restarts it with a backoff if it dies, and stops it at shutdown
(`SIGTERM`, 20 s of grace, then kill; the Compose file's `stop_grace_period`
leaves room for that).

| Variable | Default | Role |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Path of the VictoriaMetrics binary to start. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Listen address, inside the container. Loopback by default: only the server talks to it. The development overlay sets `0.0.0.0:8428` and publishes the port. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retention, in the syntax of `-retentionPeriod`: a number of months, or a duration such as `30d` or `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget for VictoriaMetrics' caches (`-memory.allowedBytes`). Without a budget VM sizes them for 60 % of the host's RAM and its resident memory grows for days. A few dozen devices fit in 256 MB; raise it for hundreds of hosts. |

Its data lives in `<data dir>/vm`, on the same volume as the database. Three
flags are fixed by the server and not configurable:
`-search.maxUniqueTimeseries=100000` (a runaway query cannot exhaust the
budget), `-search.maxConcurrentRequests=4` (one UI plus the alert engine) and
`-dedup.minScrapeInterval=10s` (duplicate points closer than ten seconds are
collapsed; nothing is lost at the 30 s cadence of the collectors).

With `DUMBMONIT_VM_URL` set, none of this applies: the server uses the
instance you name and the embedded one is never started. `GET /api/health`
says which case you are in (`victoria.embedded`).

## Compose-level settings

The shipped `docker-compose.yml` reads no variable and needs no `.env` file.
Everything is set under `environment:` of the service or by editing the file;
see [Advanced options](../install/docker.md#advanced-options) (port, secret,
retention and memory, external VictoriaMetrics, password reset, screenshots).
The development overlay `docker-compose.dev.yml` reads `DUMBMONIT_VM_PORT`
(default `8428`) for the host port of the embedded VictoriaMetrics.

## Ports

| Port | Container | Purpose |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Web UI, API, agent installers and downloads, agent ingest. The only port to publish. |
| `8428/tcp` | `dumbmonit` | Embedded VictoriaMetrics HTTP API, on the container's loopback. Published only by the development overlay. |

Outbound: SNMP (UDP 161 by default) towards devices, HTTPS towards Proxmox,
PBS, Synology and notification services, plus whatever your service monitors
target. ICMP ping needs the `net.ipv4.ping_group_range` sysctl set in the
Compose file (no capability).

## Volumes and files

One volume, `dumbmonit-data`, mounted on `/data`. Its name is fixed in the
Compose file, whatever the project is called.

| Path (in the container) | Volume | Contents |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Devices, credentials (encrypted), rules, alert state and history, baselines, silences, channels, agent tokens, password hash, sessions. |
| `/data/secret.key` | `dumbmonit-data` | The instance secret. **Back it up.** Without it, encrypted credentials are unrecoverable and the server refuses to start. |
| `/data/backups/` | `dumbmonit-data` | Scheduled local backups: `dumbmonit-<timestamp>.db` (an online SQLite copy) and the matching `.key`. Daily by default, seven kept. |
| `/data/vm/` | `dumbmonit-data` | Embedded VictoriaMetrics time series, 12 months by default. Empty when `DUMBMONIT_VM_URL` is set. |
| `/agents/` | image | Agent binaries. Mount another directory and set `DUMBMONIT_AGENT_DIR` to ship your own. |
| `/etc/ssl/certs/ca-certificates.crt` | image | TLS roots for outbound HTTPS. Mount your own bundle here to trust a private authority. |

## Resetting the password

```bash
# add DUMBMONIT_RESET_PASSWORD: "1" under environment: in docker-compose.yml
docker compose up -d
# open the UI: /setup asks for a new password
# remove the line again, then:
docker compose up -d
```

The reset clears the password and every session; devices, rules and channels
are untouched.

## Agent

The agent has its own variables and configuration file: see
[Linux, macOS, FreeBSD and Windows agent](../devices/agent.md#configuration-file).

### Collection scope

What the agent measures, and therefore how many series one machine produces.
The defaults keep a host with dozens of containers to a few hundred series;
each key is a `agent.yaml` entry, overridable by its environment variable.

| Key | Variable | Default | Role |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Network interfaces left out: container and VM interfaces, the loopback. A list of exact names or regular expressions (unanchored; anything with a metacharacter is a regex). The value replaces the default; an empty list keeps every interface. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(empty)* | Allow list, same syntax. When set, only these interfaces are reported and `interfaces_ignore` is not consulted. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Mount points left out of filesystem usage and disk I/O, same syntax. Removable drives under `/run/media/` stay. Pseudo filesystems (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) are always skipped, whatever the path. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | One `cpu_core_usage_percent` series per core, on top of the global usage. `cpu_count` is reported either way. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Containers detailed per cycle, running ones first. Beyond, containers are only counted (`container_count`, `container_running_count` stay exact) and `container_series_skipped` says how many. `0` counts without detailing. |

In the environment, lists are comma-separated (`^veth, ^br-, docker0`); in
YAML, either a list or one comma-separated string. Disk I/O counters
(`disk_read_bytes`, `disk_written_bytes`) carry the `device` label only: one
series per block device, however many times it is mounted.
