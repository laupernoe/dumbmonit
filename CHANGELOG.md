# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- **Widgets on the wall.** Weather for up to four places (now, the next hours
  and days, from Open-Meteo), crypto prices with their 24-hour change
  (CoinGecko), stock quotes with the change of the day (Finnhub, free key),
  exchange rates against a base currency (Frankfurter) and the latest critical
  vulnerabilities (NVD and the CISA catalogue of exploited ones). Each one is
  turned on and set up in **Settings → Wall display → Wall widgets**; they take
  turns in one card under the clock, fading from one to the next. Rises and
  falls carry an arrow and a sign, every card names its source. The server
  reads and caches every source (a minute for coins, an hour for rates and
  CVEs), so no display talks to them; a source that fails keeps its last
  figures for a while, then the card says *Unavailable*. API keys are stored
  encrypted and never shown again.
- **News pictures in rotation.** **Picture carousel** in **Wall news** shows
  the latest articles of the feeds with their picture (taken from the feed,
  or from the article's `og:image` when the feed has none), one at a time in
  the middle of the wall. Pictures are relayed by the server, only real
  PNG/JPEG/GIF/WebP/AVIF images up to 6 MB.

## 0.9.1 — 2026-10-16

### Changed

- **The map is a dependency mesh.** One node per monitor, with its icon and a
  status badge; declared parents are arrows that shape the hierarchy, agent
  services, hypervisor guests, relays and LLDP neighbours are lighter lines. A
  monitor that is down stands out with everything it hides (suppressed).
  Hovering a node lights up what it depends on and what depends on it; a click
  opens its panel.
- **Remote sites, found on their own.** Monitors behind a relay agent, on an
  agent that declares a site, or with a `site` tag are drawn as islands joined
  to the main site by a WAN link. No configuration needed.
- **Folders only on request.** The map ignores folders unless **Group by
  folder** is on (remembered in the browser); the folder and site filter is
  gone. Export to SVG/PNG and printing are unchanged.
- **Reproducible builds.** Two builds of the same commit now give bit-for-bit
  identical binaries (server, Linux, Windows and FreeBSD agents), web UI files
  and Docker images: the web UI version identifier is the commit instead of
  the build time, embedded files carry no modification date, the Windows agent
  gets a fixed link date, and the images are built with the commit date as
  `SOURCE_DATE_EPOCH` (`rewrite-timestamp`). A new workflow builds every
  change twice on two runners and fails on any difference. How to check a
  release yourself:
  [Reproducible builds](https://dumbmonit.readthedocs.io/en/latest/development/reproducible-builds/).
- **Phones: minimal and still.** Under 640 px the Overview fits one screen,
  lists scroll under a fixed toolbar, Tiles show two per row, decorative
  animations (cascades, blurred titles, breathing lights, the animated sky) are
  off, the bottom bar has four entries and field hints appear only while
  typing. Nothing changes on a computer.

### Fixed

- **Status page and report availability** no longer forgets the days before a
  monitor's labels changed (renamed, tags edited, or the upgrade to
  0.1.0-alpha.7 that stopped copying type options such as `port` into `tag_*`
  labels): every `dumbmonit_up` series of a device and every
  `dumbmonit_probe_success` series of an HTTP/uptime monitor is now merged, so
  those days show their real uptime instead of "no data" or a partial figure.
  The response-time graph of the status page merges them the same way.
- **Status page uptime bars** now come with a colour key (99.5% or more, 95 to
  99.5%, under 95%, no data), so a day with some downtime is no longer read as
  a day the service was down; the thresholds live in one place shared by the bars
  and the key.
- **Alert notifications.** An alert whose condition came back right after it
  resolved fired again without telling anyone; it is now announced as a new
  alert. A resolution is announced even when the alert has meanwhile been
  suppressed by a parent or silenced, as long as its firing was announced. The
  handoff to the escalation channel is no longer lost when the first
  announcement left late (after a maintenance window) or when the severity
  escalation fell due in the same cycle.
- **Last hardcoded strings.** API error messages and hints, the error page and
  the quick-silence name follow the interface language, and absolute dates use
  the locale's format.
- **Stale alerts.** Disabling a rule now clears its alerts at once (active ones
  leave a silent "rule disabled" entry in the history) instead of leaving them
  firing in the UI, or dropping them without a trace. An evaluation cycle
  running while a monitor or rule was removed no longer brings its alerts back
  (which then "resolved" with a notification). Deleting a rule also deletes its
  per-monitor overrides, so a new rule with the same name no longer inherits
  them.
- **Failed notifications.** A delivery refused for good (revoked webhook,
  deleted bot, broken channel settings) is no longer resent every cycle: the
  error is shown on the channel and the message dropped. Temporary failures
  (timeout, 429, 5xx) are retried with a growing delay, from 1 minute up to
  30 minutes.
- **Agent backlog.** An error page from a reverse proxy (400 or 404 without
  DumbMonit's error format, for instance during maintenance) no longer makes
  the agent throw away its whole buffer: it keeps the measurements and
  retries. A batch too large for a proxy (413) is split into smaller ones
  instead of being lost. When the server keeps none of a batch's
  measurements, the agent now warns that the machine's clock is probably off.
- **Shutdown.** Stopping the container no longer loses the last measurements
  still waiting to be written: they are flushed to VictoriaMetrics before it
  stops. Stopping while the embedded VictoriaMetrics waits to restart after a
  crash now returns at once instead of waiting for the full grace period.

- **SNMP survives a lost packet.** Each request now gets a few seconds with up
  to two resends instead of one request lasting the whole probe, so a single
  dropped UDP datagram no longer marks a device down. An SNMPv1 device that
  does not know one of the requested values no longer loses the other values
  of the batch.
- **Uptime checks with a long timeout.** A check whose own timeout is longer
  than `DUMBMONIT_PROBE_TIMEOUT_SECS` (up to 60 s) is no longer cut short and
  reported as unreachable; it records its failure like any other check.
- **Synology and certificate errors.** A Synology NAS whose system,
  utilization and storage calls all fail is reported down (timeout) or
  misconfigured (refused login) instead of up. On Proxmox VE, PBS, PDM, PMG,
  Redfish and Synology, only a rejected certificate counts as a configuration
  error: a host name containing `tls` or a handshake cut by a rebooting device
  now raises the down alert.
- **Relay agents with a wrong clock.** Values relayed with a timestamp more than
  30 days old or more than an hour ahead are dropped, as they already were for
  agents.
- **Metrics API when VictoriaMetrics is down.** `/api/metrics/query` and
  `/api/metrics/query_range` answer 502 instead of 400 when the metrics
  database is unreachable or failing; 400 is kept for queries it rejects.

- **Editing a user is all or nothing.** A password refused by
  `PUT /api/users/{id}` no longer leaves the role or display name changed.
- **Editing one secret of a channel keeps the others.** Secrets are merged by
  key (Pushover's user key survives a new token). `"secrets": {}` still clears
  them all, and a new kind or destination still asks for every secret again.
- **HTTP probe headers stay with their target.** Custom headers (API keys,
  tokens) are no longer sent to another origin on a redirect, like the
  credential.
- **Agents keep their binding on foreign 403s.** Only the server's own binding
  refusal makes an agent forget its secret, not a 403 from a reverse proxy or a
  firewall.

## 0.9.0 — 2026-10-16

### Upgrading from 0.1.0-alpha.8

- **Version numbers.** Releases are now plain `0.MINOR.PATCH`: this one is
  **0.9.0**, the next 0.10.0, and a fix-only release bumps the last number.
  Nothing to do; `:latest` still follows releases and `:edge` is built from the
  `dev` branch.
- **Existing Docker Compose file: nothing to do.** The image still runs as user
  65532 and your current `docker-compose.yml` keeps working as it is.
- **Bind mounts.** The new `docker-compose.yml` starts the container as root
  (`user: "0:0"` with four capabilities in `cap_add`) so that a host directory
  such as `./dumbmonit` or `/mnt/user/appdata/dumbmonit` works without
  preparation: the server hands `/data` to 65532 at startup, then drops root
  for good. Take the new file if you want that. On SELinux hosts (Fedora, RHEL)
  add `:Z` to the volume. A named volume needs none of this.
- **Reinstall the Linux agent (systemd) if its alpha.8 installation failed.** A
  bug in the installer could kill the shell while it wrote the service (the
  command ended on "Killed", the unit was empty or masked, the agent never
  started). On that machine, run
  `sudo rm /etc/systemd/system/dumbmonit-agent.service && sudo systemctl daemon-reload`,
  then run the installation command again from the agent page. Agents that were
  already running are not affected.
- **Monitors are now Monitors.** The interface and the documentation say
  *monitor*, and the built-in rule **Device unreachable** is now **Monitor
  unreachable** (same identifier: overrides and silences are kept). The API,
  URLs and metric names do not change.
- **Agent services become monitors.** On its first report after the update, each
  agent moves its rule overrides, machine-only rules, silences and open alerts
  to the new per-service monitors (Docker, Plakar, restic/Borg, ZFS, libvirt,
  WireGuard, IPsec, speed test). Older history stays under the machine, so a
  chart may show a break at the update. Review the machine's list of modules
  found, and untick the ones you do not want as monitors.
- **Security score is opt-in.** The Security page is gone and `/security` opens
  the setting. An installation that already monitored a rated monitor keeps the
  score on; a new one starts with it off. Turn it on in **Settings → Monitoring →
  Security score**. A few products (pfSense, UniFi, NTP, nginx/Apache, GitLab,
  Forgejo) are no longer rated.
- **Agent with Docker Compose.** `docker-compose.agent.yml` is shorter and now
  keeps the agent's binding in a volume. If you copied the old file, take the
  new one (the options moved to the agent page) so that recreating the
  container no longer unbinds it.
- **Notification links.** Acknowledge and snooze links in alert messages are
  active as soon as a public URL is configured; turn **Acknowledge and snooze
  from notifications** off in the notification policy if you do not want them.
- **Heartbeat to set again after a restore.** The outgoing heartbeat URL is not
  part of the encrypted configuration bundle: enter it again after restoring
  onto a new instance (Settings → System → DumbMonit itself).
- **New versions of the bundle.** VictoriaMetrics 1.154.0 and rustls 0.23.46
  (see Security); the bundled VictoriaMetrics is replaced when you pull the
  new image.

### Added

- **Acknowledge or snooze an alert from the notification.** Alert messages
  carry **Acknowledge** (until it resolves) and **Snooze 1 h**: buttons on
  ntfy, Telegram and Web Push (operators and admins only), links on Discord,
  Slack, email and the other channels, `{{ack_url}}` / `{{snooze_url}}` for
  custom webhooks; none on PagerDuty and Opsgenie. A link opens a confirmation
  page that works without signing in; nothing happens until its button is
  pressed, so link previews and mail scanners act on nothing. Each link is
  signed with a key derived from the instance secret, covers one gesture on
  the alerts of its message (that firing only: a later re-fire is not
  covered), works once and for 24 hours, and is recorded in the audit log as
  "via notification link". On by default when a public URL is set;
  **Acknowledge and snooze from notifications** in the notification policy
  turns it off and revokes the links already sent. New public endpoints
  `POST /api/public/alert-action/inspect` and `POST /api/public/alert-action`,
  `action_links` in the notification policy, `actions` in `GET /api/notify/kinds`.
  See [Acknowledge or snooze from the notification](docs/notifications.md#acknowledge-or-snooze-from-the-notification).
- **Who watches the watcher.** An optional **outgoing heartbeat**
  (Settings → System → DumbMonit itself): DumbMonit calls a URL of yours
  (Healthchecks.io, an Uptime Kuma push monitor, Better Stack…) every 60
  seconds by default, only while the scheduler runs, SQLite answers and
  VictoriaMetrics is reachable, so that an outside service alerts you when the
  host, the container or the engine goes quiet. A **Test** button, an optional
  failure signal (`/fail`, or `status=down` for Uptime Kuma), the URL stored
  encrypted and never shown again, the same outbound guard as the notification
  channels. Six new built-in rules on DumbMonit's own health, through the usual
  channels: data volume almost full, embedded VictoriaMetrics restarting in a
  loop, scheduler stalled, polls running late, metric writes failing, database
  growing fast, from new `dumbmonit_instance_*` series written every minute.
  New endpoints `GET /api/self-monitoring`, `PUT /api/self-monitoring/heartbeat`
  and `POST /api/self-monitoring/heartbeat/test`. See
  [Watching DumbMonit itself](docs/install/self-monitoring.md).
- **Docker health status.** The server image now carries a `HEALTHCHECK`
  (`dumbmonit healthcheck`, a local call to `/api/health`), so Docker, Portainer
  and Unraid show the container as healthy.
- **Each service an agent finds is its own monitor.** Docker, Plakar,
  restic/Borg, ZFS, libvirt, WireGuard, IPsec and the speed test no longer pile
  up on the machine's page: each one becomes an `agent_service` monitor under
  the machine, with its own status, alerts, rules and silences, hidden behind
  the machine when it goes down. The machine keeps CPU, memory, disks, network,
  SMART, sensors and OS health; every series is written once, under the monitor
  it belongs to. The machine's page lists the modules found and not found:
  untick one to drop its monitor (the agent stops collecting it), tick one to
  watch it before it shows up. Nothing to choose at install time. See
  [Services found by the agent](docs/devices/agent.md#services-found-by-the-agent).
- **Topology map.** A new **Map** page (`/map`, from the **⋯** menu of the Monitors page
  and the command palette) draws the whole infrastructure from the
  links DumbMonit already knows, nothing to type in: declared parents, relay
  agents, hypervisor guests (Proxmox, XCP-ng, Nutanix, vSphere, libvirt) and
  LLDP neighbours of SNMP switches, matched to a monitor only when the name is
  unambiguous. What is down and what sits behind it (impacted, alerts
  suppressed) stand out, in words as well as colour. Zoom and pan, a folder or
  relay-site filter, a list view on phones, and **Export** to SVG, PNG or a
  clean A4/A3 landscape print. New endpoint `GET /api/topology`. See
  [Topology map](docs/using/topology.md).
- **Import from Uptime Kuma.** On **Add a monitor**, drop the backup file of
  Uptime Kuma 1.x (Settings, Backup, Export; version 2 has no export) and get a
  preview of what will be created and what is skipped, and why: HTTP, keyword and
  JSON-query monitors become `http`, ping, TCP port and DNS keep their type, push
  monitors become heartbeats (with a new URL), groups become folders and tags
  become monitor tags. Notifications, credentials, headers and request bodies are
  never imported. New endpoint `POST /api/targets/import/kuma`. See
  [Coming from Uptime Kuma](docs/devices/services.md#coming-from-uptime-kuma).
- **XCP-ng / XenServer.** A new `xcpng` monitor reads a whole pool through XAPI
  (JSON-RPC, read-only role): hosts disabled or offline, VMs and their power
  state, storage repositories filling up, and high availability (protected VMs
  found halted, an overcommitted pool). A pool member's `HOST_IS_SLAVE` answer
  is followed to the coordinator. Four built-in rules. See
  [XCP-ng / XenServer](docs/devices/xcpng.md).
- **Nutanix AHV.** A new `nutanix` monitor reads Prism Central's v4 API
  (pc.2024.3 or later, Prism Viewer role): clusters, hosts degraded or in
  maintenance, AHV VMs, storage container fill level and unresolved alerts by
  severity. Three built-in rules. See [Nutanix AHV](docs/devices/nutanix.md).
- **OpenVPN server** monitor type: version, whether the server reports itself
  connected, and every client in session with its addresses and traffic, read
  through the management interface (read-only: `version`, `state`, `status 3`).
- **KVM virtual machines on the Linux agent.** A new `libvirt` module reads
  `virsh -c qemu:///system` (domains, their state, vCPUs and memory; storage
  pools and their fill level), silent where `virsh` is absent. Two built-in
  rules: a crashed domain, a pool almost full. See
  [the agent page](docs/devices/agent.md#kvm-virtual-machines-through-libvirt).
- **IPsec tunnels** in the agent (module `ipsec`): every strongSwan tunnel
  configured, up or down, read with `swanctl --list-conns` / `--list-sas`, with
  traffic and time to the next rekey, an IPsec panel on the monitor page, and
  the **IPsec tunnel down** rule.
- **Internet speed test** in the agent: download and upload rate, latency and
  jitter against an endpoint you name (LibreSpeed, Cloudflare, your own), at
  most once per period (6 h by default, 5 min minimum). Off by default since
  it uses bandwidth; the **Internet download rate low**, **upload rate low**
  and **latency high** rules stay silent until a threshold is set.
- **New security checks.** Linux hosts: SSH root login, SSH password
  authentication, host firewall. Windows hosts: firewall profiles, pending
  restart, Remote Desktop without Network Level Authentication, SMBv1.
  Websites (HTTP(S) monitors and Website health): HTTPS and security headers
  (HSTS, `nosniff`, clickjacking protection, CSP) besides the certificate.
  Synology: pending DSM update, built-in `admin` account, Auto Block, DSM
  firewall (administrator account only, read hourly). New series:
  `agent_ssh_root_login`, `agent_ssh_password_auth` and `agent_firewall_active`
  (Linux); `agent_firewall_profile_enabled`, `agent_reboot_required`,
  `agent_rdp_enabled`, `agent_rdp_nla` and `agent_smb1_enabled` (Windows);
  `probe_http_security_header` (HTTP(S) monitors); `website_https` and
  `website_security_header` (Website health); `synology_update_available`,
  `synology_admin_account_enabled`, `synology_autoblock_enabled` and
  `synology_firewall_enabled`.
- **Security report per monitor.** The grade badge opens a report: what fails
  and how to fix it, what passes, concrete steps to improve the score on that
  product, and a link to its documentation, under a visible "indicative, not a
  full audit" notice. New `GET`/`PUT /api/security/settings`; both security
  reads now carry `enabled`, and the monitor report a `guide`. See
  [Security score](docs/using/security-score.md).
- **Monitored alerts on the edit form.** Editing a monitor lists every alert
  rule that reads a metric it has written in the last 24 hours (for an agent
  with Docker: container stopped, unhealthy, restarting, update available, and
  so on), folded by default behind a summary. Turn a rule off to stop its
  alerts for that monitor only; the data is still collected, and the switches
  are saved with the form. Same per-monitor override as **Don't alert me
  again**. New endpoint `GET /api/targets/{id}/rules`.
- **Monitor form: Test connection, and a smarter network scan.** SNMP monitors get a
  "Test connection" button that probes the entered address and credential once
  (`POST /api/targets/test`, nothing is saved) and shows `sysName` / `sysDescr` or
  the probe's error. A disabled Save or Test button now says why. The network scan
  pre-fills a suggested network (`GET /api/discovery/subnets`), still editable: the
  `DUMBMONIT_SCAN_SUBNETS` setting, else the /24 networks of the monitors already
  monitored, else the server's own networks (outside a container only), else an
  empty field with a hint.
- **Command palette: recents, more actions, keyboard shortcuts.** Ctrl/⌘ K now
  lists the last eight opened pages and monitors first (kept per browser),
  offers more actions (silence alerts, open a settings section, keyboard
  shortcuts, sign out) limited to what your role may do, and shows each
  shortcut. New global shortcuts outside text fields: `g o` overview, `g d`
  monitors, `g a` alerts, `g s` status pages, `g n` add a monitor, `g t` theme,
  `?` for the help dialog.
- **Unsaved-changes warning in Settings.** The SSO, Wall news and Reports forms
  now show "Unsaved changes" next to Save and ask before you leave the page or
  close the tab.
- **Recoverable empty states and errors.** An empty page or a failed request
  now offers the next step (add a first monitor, sign in again, retry when
  offline or on a server error), and the Overview sky band is shorter on
  phones so content stays above the fold.

### Changed

- **Alerts → Now is a calm list.** One line per problem — what, which monitor,
  the cause in one line, since when — with **Ack** and **Open** on the line,
  grouped by severity or by monitor; groups fold, and suppressed, snoozed and
  acknowledged problems start folded. One rule on many series stays one line
  ("… and 7 more"); a click unfolds the details and Snooze / Never for this
  monitor. The Alerts tabs are now Now, Maintenance, History and Rules.
- **Selecting is a mode.** On the Alerts page and the Monitors list,
  checkboxes only appear after **Select**, with a floating action bar that
  moves nothing on the page; Escape or Done leaves it. Operators can
  acknowledge, unacknowledge or ignore several alerts in one go; admins can
  enable, disable, move to a folder or delete several monitors; a partial
  failure is listed and the failed ones stay selected.
- **What DumbMonit watches is called a monitor.** The interface and the
  documentation say monitor everywhere (navigation: Overview / Monitors /
  Alerts / Settings), and the built-in rule **Device unreachable** is now
  **Monitor unreachable**. Nothing changes under the hood: API routes, rule
  identifiers, `target` labels, URLs and metric names are the same.
- **A simpler Monitors page.** One toolbar: search, **Filter** (state, type,
  sort and tags fold behind it, with a count of those in use), Compact / Tiles,
  **Select** and **Add a monitor**. The map, **Create folder** and the CSV and
  Uptime Kuma imports moved to a **⋯** menu next to it. Folder headers are
  plain lines that show a state only when something inside is not reporting;
  security grades are quiet in the list, and monitors without a grade show none.
  The type filter only lists the types you use. Tags in use can be picked to
  keep the monitors that carry all of them; search, filters, sort and layout
  are kept in the URL (`?tag=env=prod`), and disabled monitors sit in a
  collapsed "Disabled (N)" section. The Rows layout is gone: Compact and
  Tiles remain, and a remembered Rows choice or a `?view=rows` link opens
  Compact.
- **Notifications and status pages are in Settings.** Notification channels
  and the policy are in **Settings → Notifications & reports**, the public
  status pages and their announcements in **Settings → Status pages**; the top
  bar is Overview, Monitors, Alerts, Settings. `/status`, `/status/<id>` and
  `/alerts#notifications…` forward to the new place; public `/s/<slug>` pages
  are unchanged.
- **Started as root, the server does not stay root.** With `user: "0:0"`
  (now in `docker-compose.yml`), before anything else (database,
  VictoriaMetrics, network) it gives `/data` to `65532:65532` — only the
  entries that belong to someone else, without following symbolic links — then
  drops its groups, group and user for good and checks that root cannot be
  regained. `PUID`/`PGID` (or `DUMBMONIT_PUID`/`DUMBMONIT_PGID`) choose another
  owner, for example your own user or Unraid's `99:100`. The image itself still
  runs as 65532, and the healthcheck drops root too when started as root.
- **The security score is opt-in and lives next to each monitor.** Turn it on in
  **Settings → Monitoring → Security score**; rated monitors then get a
  grade badge in the monitor list and on their page. Off, the score is not
  computed and `dumbmonit_security_score` is no longer written. New
  installations start with it off; an installation that already monitored a
  rated monitor keeps it on.
- **Security score coverage narrowed** to the products where it means
  something: hosts, websites, Synology, Proxmox VE, Proxmox Backup Server,
  OPNsense and Active Directory. pfSense, UniFi, NTP, nginx/Apache, GitLab,
  Forgejo and the vendor-update checks of other integrations are no longer
  rated: one or two checks gave a grade that meant little.
- **Shorter `docker-compose.agent.yml`.** Only what most hosts need; the
  options moved to the agent page. It now keeps the agent's binding in a
  volume, so recreating the container no longer unbinds it.
- **Adding a monitor starts from a short list.** Search first, then the common
  choices (website, ping, machine with the agent, SNMP monitor, hypervisor, NAS,
  network scan); every other type is grouped by category and folded. The agent
  appears once.
- **Alert rules page: grouped, filtered, counted.** The 150+ rules are grouped by
  monitor kind and folded by default (kinds you own first, with how many are off
  or firing); quick filters (Active, Disabled, Changed by me, Fired lately)
  join the search and the kind select, with a "N of M rules" counter. All the
  editing, toggling and deleting is unchanged.
- **Monitor page: flat charts fold into one line.** In "Everything else", the
  metric families that never moved over the period (the 0/1 checks of a
  website, for instance) are no longer drawn; they collapse into a "Steady
  checks (N)" line that lists their value. Only the series that change keep a chart.
- **Status page editor: compact scene choice.** The chosen scenes are shown as
  thumbnails with a "Change" button that opens a searchable, paged grid, and
  the list of services to add has a search box.
- **Wall: each problem once, readable.** A monitor written out as a problem is
  no longer repeated among the monitor tiles, long names and causes wrap instead
  of being cut, and "And N more" became a clear counter linking to Alerts.
- **Translations now go through pull requests.** The UI translations stay in
  `web/messages/<locale>.json`; see "Translating DumbMonit" in
  [CONTRIBUTING.md](CONTRIBUTING.md#translating-dumbmonit).
- **Clearer failures on notifications and status pages.** A failed channel
  delivery shows the service's message, HTTP status and a hint, with the raw
  text under "Details"; Remove and Delete moved into a row "More actions" menu,
  and each status page row shows its published or draft state and custom domain.

### Removed

- **The Security page.** `/security` now opens the security score setting; the
  report moved to each monitor.

### Fixed

- **Bind mounts.** `-v ./dumbmonit:/data`, `/mnt/user/appdata/dumbmonit` on
  Unraid or a directory Docker has just created as `root:root` left the
  container restarting on "not writable", with a hint that only fitted named
  volumes. With the new `docker-compose.yml` (or `--user 0:0`) the ownership is
  fixed at startup. Otherwise the message now suggests `--user 0:0` first, then
  the right command for a bind mount (`sudo chown -R` on the host path) or a
  named volume.
- **SELinux hosts.** A bind mount without a container label (Fedora, RHEL) is
  refused even to root; the server now says so and asks for `:Z` on the volume,
  documented with rootless Podman in the install guide.
- **Agent install on systemd hosts.** The Linux installer ran out of memory
  while writing the systemd unit (shell killed, empty unit, agent never
  started): a comment in the unit held `yes` between backticks, which the
  shell ran. Re-run the install command after upgrading the server.
- **Overflow menus on phones.** The "⋮" menu on a monitor's page (and every
  other overflow menu) now opens above the rest of the page, kept on screen
  and flipped upwards near the bottom, instead of hiding behind the next card
  where its actions could not be tapped.
- **Disabled monitor page.** No more false "Reporting" plate, "Probe now" or
  silence controls on a disabled monitor; an "Enable" button takes their place.
- **Public status page no longer claims "All systems operational" without
  data.** Services that are disabled or never measured are left out of the
  banner and the tally, which reads "No data yet".
- **Status page uptime.** The uptime next to each bar and in the summary line
  now covers the same days as the bar. On a page showing 8 to 29 days it was
  the 7-day uptime, under a heading naming the longer window.
- **Info alerts are no longer counted as problems.** The nav badge and the wall
  ("25 problems") count only warnings, criticals and unreachable monitors.
- **Unreachable agent message.** The duration is now readable ("no measurement
  for 10 d 21 h (limit 90 s)") instead of raw seconds.
- **Accessibility.** The Alerts tabs support Left/Right/Home/End with a roving
  tabindex, and the muted text colour (`ink-3`) now meets WCAG AA contrast in
  light and dark.

### Security

- **Bundled VictoriaMetrics 1.153.0 → 1.154.0.** Built with Go 1.27.2 and
  `golang.org/x/net` 0.60.0, which fix CVE-2026-97031, CVE-2026-78667 and
  CVE-2026-78669 (all high) in the binary shipped in the image. No change to the
  flags or query parameters the server uses.
- **rustls 0.23.46** (RUSTSEC-2026-0346 to RUSTSEC-2026-0349: TLS secrets in
  `Debug` output, ECH rejection data leak, Brotli decompression allocation,
  key-exchange downgrade).

## 0.1.0-alpha.8 — 2026-10-14

### Added

- **News on the wall.** Off by default, set in **Settings → Wall news**: a
  headline ticker along the bottom (smooth marquee; one fading headline at a time
  under reduced motion) and a muted live channel in a tile (YouTube, Dailymotion
  or Twitch, small to large, right column or centre). Pick a country and tick
  its sources (France, Switzerland, Belgium, Germany, UK, US, Canada, Spain,
  Italy, Portugal, Brazil, Russia, China, international; each feed checked), or
  add any RSS/Atom feed with a **Test** that shows its first headline. The server
  reads the feeds (RSS 2.0, RDF, Atom) behind the internal-address guard with
  time and size limits and a five-minute cache. New routes `GET /api/wall/news`,
  `GET|PUT /api/wall/news/settings`, `GET /api/wall/news/presets`,
  `POST /api/wall/news/test`. See [Wall mode](docs/using/wall.md).
- **Product icons on the wall's problems**, as in the device lists.
- **Website changes show which words changed.** A replaced line underlines the
  new words and strikes through the old ones, so one edit in a long paragraph is
  found at a glance. `GET …/webchange/changes` takes `kind` and `before` (the id
  of the last change received) to filter and page through the list.

- **Missing server prerequisites are shown where they matter.** Adding or editing
  a website-change device (headless browser for screenshots) or a ping device
  (ICMP sysctl) shows a notice when the server cannot do it, with the Compose
  lines to copy and a link to the matching section of
  [Install with Docker](docs/install/docker.md#website-screenshots);
  `GET /api/collectors` gains an optional `requirements` field.
- **Website health & SEO: more control over what alerts.** Each metadata check
  (final address, title, description, canonical URL, robots directives, Open
  Graph, language, h1 count, hreflang, Google verification tag) can be switched
  off on its own, and a change is reported after a configurable number of
  consecutive checks (2 by default, 1 to 10). The `hreflang` languages and the
  `google-site-verification` tag are now tracked, with new alerts "Website
  Google verification changed" and "Website SEO markup changed". See
  [Website health & SEO](docs/devices/website.md).
- **Nearly empty pages.** Besides the blank-page threshold, the visible text of
  a site is compared with a slowly moving reference: a sharp drop (80% by
  default, confirmed like any change) raises "Website page nearly empty".
- **Website categories.** Sites get a category (E-commerce, Blog, Web app,
  Showcase) guessed from clear signs, or set with the new **Category** option.
  The CSV import takes a `category` column, files unclassified sites under
  `Websites/<category>` and keeps `Websites/<platform>` when the page does not
  say. The category is the `category` label of `dumbmonit_website_info`.
- **Agent: choose its modules.** A `modules` list in `agent.yaml`
  (`--modules=docker,plakar` for `install.sh`, `-Modules` for `install.ps1`,
  `DUMBMONIT_AGENT_MODULES`) runs only the listed optional modules: the others
  send nothing and raise no alert. The enrolment screen and Settings → Agents
  have a **Choose the modules** switch that writes it into the install
  commands. Without a list, every module still detects itself.
  `install.sh --plakar-klosets=@name` picks the Plakar klosets to read.
- **Rule "Plakar kloset unreadable"** (warning, 30 min): a kloset the agent
  cannot open no longer raises the critical "Plakar backup failed", which now
  only means a snapshot with errors.

### Fixed

- **Wall: the problems on the left no longer overlap the devices or run past
  the screen.** With many problems or on a 720p display, the big tiles used to
  spill over the device tiles below; they now shrink to one line each, then fold
  into "And N more". A long device name no longer pushes the cause out of a row.
- **Watching a site that redirects to another host reads the whole site.** A
  start page redirecting from `example.com` to `www.example.com` used to leave
  every link outside the watch, which then only covered the start page. Only
  a host of the same site is adopted (`www.`, the bare domain, a subdomain,
  `http` to `https`): a start page that redirects to a sign-in page or a
  third-party domain only has its start page read.
  Tracking parameters (`utm_…`, `fbclid`, `gclid`…) no longer make one page
  several; **Ignore query strings** drops the whole `?…` part. When **Max
  pages** stops a crawl, the device page now says how many pages were found.
  New **User agent** option for sites that turn robots away.
- **Every website check that finds changes is notified, and says which
  pages.** The "Website changed" alert stayed open while consecutive checks
  kept finding changes, so only the first of them was ever notified, and the
  message only gave a count. While the alert fires, every further check that
  finds changes is now notified again, naming the pages ("/pricing, /contact
  and 2 more"); it stays one alert per device, so an acknowledgement or a
  snooze taken on it covers the following checks, and it resolves once, at the
  first check that finds nothing new. See
  [Alerts](docs/devices/webchange.md#alerts).
- **Dragging a monitor into a folder works again.** Pressing a row on its name
  or address grabbed the link instead: the browser started its own drag of it,
  cancelled the gesture, and the monitor never moved. Rows can be dragged from
  anywhere again, onto another row or onto a folder header.
- **Instant metric reads failed with "latency_offset=0ms is out of allowed
  range".** VictoriaMetrics refuses a zero offset, so every panel reading the
  latest value (device pages, Docker, Plakar…) showed an error. The offset is
  now 1 ms.
- **Plakar was never found by an agent installed as a systemd service.** The
  unit hid `/home` (`ProtectHome=yes`), so `~/.config/plakar/stores.yml` was
  invisible, and Plakar refused to start without `HOME`. The unit now uses
  `ProtectHome=read-only` and a `CacheDirectory`; the agent runs `plakar` with
  the owner's `HOME` and plugins (`-datadir`) and its own `-cachedir`, and
  warns when `/home` looks empty. Re-run the install command to get the new
  unit.
- **Docker alerts for containers nobody expects to run.** A one-shot
  `docker run` that finished, a container never started, or one stopped by
  hand without a restart policy no longer raises "Container stopped"
  (new series `dumbmonit_container_stop_expected`). A stopped container no
  longer reports its stale health verdict, so "Container unhealthy" stops
  firing next to "Container stopped".
- **Plakar log noise.** An unreadable kloset is logged once, again when the
  reason changes, and when it recovers — not every ten minutes.
- With `system_health` switched off, the agent still reports the OS identity
  (`dumbmonit_agent_os_info`) the device page shows.

### Changed

- **Website changes ignore the site's menus, header and footer.** They are the
  same on every page, so one new menu entry used to mark every page of a
  watched site as changed. The navigation, the site header and footer and the
  sidebars are now left out of the comparison (their links are still
  followed); **Compare menus, header and footer** brings them back. New
  options: **Watched area** compares only some elements (`main`, `#content`),
  **Excluded areas** leaves more out (`.cookie-banner`). The same edit found on
  several pages by one check is shown once, "+N pages"; changing **Max pages**
  now takes a new reference instead of reporting every added page as new; and
  500 changes are kept per device instead of 100. Existing watches take a new
  reference at their next check after the update. See
  [What part of the page](docs/devices/webchange.md#what-part-of-the-page).
- **Minimal `docker-compose.yml`.** About 15 lines, no `.env` and no `${VAR}`: port, secret, retention, external VictoriaMetrics, password reset and the screenshots browser are now `environment:` examples in the [installation guide](docs/install/docker.md#advanced-options). To build from a checkout, add the `docker-compose.dev.yml` overlay.

## 0.1.0-alpha.7 — 2026-10-13

### Security

- **Custom webhook headers are now a secret.** *Extra headers* moved from the
  channel settings, which every viewer and `read` token could read, to the
  encrypted secrets: they are never returned, are kept when you edit the
  channel without retyping them, and their values are masked in error
  messages. Existing channels are migrated at startup (and after a restore).
  See [Custom channel settings](docs/notifications.md#custom-channel-settings).
- **Status page subscriber emails are admin only.** Viewers, operators and
  `read` tokens can no longer list them.
- **API and agent token lists are admin only**, like creating and revoking.
- **Redirects never leave the configured host.** Built-in integrations
  (Proxmox, Synology, GitLab, Kubernetes…) and notification channels follow a
  redirect only to the same host (or from `http` to `https`): a hostile device
  can no longer bounce a probe to the loopback or a metadata service, and
  custom credential headers (`X-Api-Key`, `PRIVATE-TOKEN`…) are never sent to
  another host.
- **Notification channels cannot target DumbMonit itself.** A webhook, a
  self-hosted service URL or an SMTP relay that resolves to the embedded
  VictoriaMetrics or to the server's own loopback listener is refused with a
  configuration error, checked after DNS resolution, at connection time and on
  every redirect. Other loopback and LAN addresses stay allowed.
- **Web Push limits.** An account holds at most 10 push devices, and a test
  notification can be sent once every 30 seconds.
- **Secret-looking device options are masked for non-admins.** Options whose
  name contains `key`, `token`, `password`…, HTTP check headers and the
  password of a `user:pass@host` address read `***` for viewers, operators,
  `read` tokens and the assistant tools.
- **Status page links ignore forged forwarded headers.** The RSS feed and the
  link stored for subscription emails use `X-Forwarded-Host` and
  `X-Forwarded-Proto` only from `DUMBMONIT_TRUSTED_PROXIES`.
- **The session cookie is `Secure` over HTTPS** even without
  `DUMBMONIT_COOKIE_SECURE`, when the browser's `Origin` or a trusted proxy
  shows the page is on `https://`.
- **`/api/health` no longer shows failure details to anonymous callers**, which
  could include the address and credentials of an external VictoriaMetrics.
- **The first-run setup code is rate limited per address only**, so someone
  else on the network can no longer keep the owner locked out of `/setup`.
- The WordPress status banner escapes the fetch error it shows in the settings
  screen.

### Added

- **Website health & SEO, in bulk.** A new `website` device type reads a
  site once per check, following its redirects (http → https and example.com
  → www.example.com are normal; a loop or too many hops is an error), and
  reports a site that is down or answers an error, a page gone blank, missing
  expected text, `noindex` (meta tag or `X-Robots-Tag`), a `robots.txt` that
  blocks everything, a slow answer, and changes of the title, description,
  canonical URL or final address — only once two checks in a row have seen
  them, so A/B tests stay quiet. Each site's platform (WordPress, Shopify,
  Wix, Next.js…) is detected and exposed as `dumbmonit_website_info{platform}`.
  Nine built-in alert rules. See [Website health & SEO](docs/devices/website.md).

- **Import websites from a CSV.** *Import websites (CSV)*, on the add-device
  page and on the device list, creates one website device per row from pasted
  or uploaded text (`url` required; `name`, `folder`, `tags`, `interval` and
  `expect_text` optional; up to 1000 rows). A preview shows each row's folder
  and errors before anything is created, sites already monitored are skipped,
  and rows without a folder land in `Websites/<platform>` (WordPress, Shopify,
  Wix…) after one page fetch each. A template is one click away. Also
  available as `POST /api/targets/import`.

- **A custom domain for a status page.** The page editor's **Public domain**
  field (for example `status.example.com`) makes DumbMonit show that page at
  the root of the name when your reverse proxy (Traefik, Nginx Proxy Manager,
  Caddy, Nginx, Cloudflare Tunnel) sends it there with the original `Host`
  header. Nothing else of DumbMonit answers on that name — no sign-in, no API,
  no other page — and subscriber emails, the RSS feed and the share snippets
  link to it. Kept in backups. See
  [Custom domain](docs/using/status-pages.md#custom-domain).

- **An operator role, between viewer and admin.** Operators read everything
  and handle alerts — acknowledge, snooze, ignore a rule for one device,
  schedule or delete maintenance windows, clear resolved history — but change
  no configuration: devices, rules, channels, notification policy, status
  pages, settings, backups, tokens and accounts stay admin only, and an
  operator can never change a role. Pick it under Settings → Users, or map it
  from your identity provider with the new *Operator groups* SSO setting
  (`DUMBMONIT_OIDC_OPERATOR_GROUPS`). The API marks the operations open to
  operators with `x-roles: [admin, operator]`. See
  [Users](docs/using/settings.md#users).
- **Push notifications on phones and computers, without an app.** A new
  **Web Push** channel sends alerts as native notifications through the
  browser's push service, even with DumbMonit closed. Each account enables
  its own devices in **Settings → Push notifications** (*Enable on this
  device*, *Send a test*, list of subscribed devices); on iPhone and iPad,
  add DumbMonit to the Home Screen first. HTTPS is required. The server's
  VAPID key pair is generated at first start and stored encrypted, messages
  are end-to-end encrypted (RFC 8291), subscription addresses must be HTTPS
  on public IPs, and subscriptions the push service reports as gone (404/410)
  are removed. The interface gains a service worker that only shows
  notifications and caches nothing. See
  [Push notifications (PWA)](docs/using/push-notifications.md).
- **Periodic email reports.** **Settings > Reports** sends the team a weekly
  (or daily, or monthly) email: availability per group and device, the
  period's incidents, the least stable devices and the comparison with the
  previous period. Plain HTML tables with a text version, no remote image,
  sent once per slot, with a **Send a preview** button. Kept in backups. See
  [Email reports](docs/using/reports.md).
- **Guided updates.** The server looks at the latest GitHub release at most
  once a day (one anonymous request, nothing about your instance is sent) and
  shows **Update available: vX** in the corner and in **Settings → About**,
  with a **How to update** panel (back up the database and `/data/secret.key`,
  then `docker compose pull && docker compose up -d`), a link to the release
  notes, **Check now** and **See what's new**. Pre-releases compare correctly
  (`alpha.10` is newer than `alpha.9`). Nothing is installed automatically.
  Turn it off in **Settings → About** or with `DUMBMONIT_UPDATE_CHECK=off` for
  air-gapped instances. New `GET /api/update`. See
  [Update](docs/install/update.md).
- **A status banner for your own website.** A published status page now serves
  `banner.js`, a dependency-free script that shows "2 services are down" on any
  site, and its public JSON can be read from other origins. Copy-paste
  integrations for plain HTML, WordPress and PHP live in `integrations/`. See
  [Website banner](docs/integrations/website-banner.md).
- **A city scene behind the status page banner.** The page editor's **Scene**
  setting takes up to six of Venice, Paris, Tokyo, New York, London and Rome
  (none by default), with a rotation of every visit, minute, ten minutes or
  hour when there are several. Both settings are kept in backups. See
  [Status pages](docs/using/status-pages.md#scene).
- **A "What's new" window after an update.** The first screen after an
  upgrade lists the headline changes of the new version once, with a link to
  the full release notes; **Settings → About** reopens it.

### Changed

- **The tip bubbles are gone.** Pip's tips and the "Did you know" card on the
  overview are removed.
- **The documentation is available in 9 languages** (English, German, Spanish,
  French, Italian, Portuguese, Brazilian Portuguese, Russian, Simplified
  Chinese).

### Fixed

- **Every channel type can be saved.** The database still only accepted the
  first seven channel types (Discord, ntfy, Gotify, Telegram, Slack, webhook,
  email): saving a Matrix, Teams, Pushover or other channel failed with an
  internal error. The table is rebuilt without that list at upgrade; existing
  channels and queued notifications are kept.

- **Nginx, Apache and NTP devices are available on the server.** The three
  kinds existed in the shared collectors but were missing from the server's
  list, so they could not be added. See [Nginx](docs/devices/nginx.md) and
  [Apache](docs/devices/apache.md).

- **Per-host series no longer collapse into one.** Nginx Proxy Manager now
  labels its hosts `proxy_host`, vSphere `esxi` and Ceph `ceph_host`, instead
  of a `host` label that the device's own `host` label overwrote. Existing
  series and silences keyed on the old `host` label start new series: update
  your rules and silences to the new label. A label that still collides is
  kept as `exported_<name>`. See [Nginx Proxy Manager](docs/devices/npm.md).

- **Options and secret-looking tags are no longer copied as `tag_*` labels.**
  A device's settings (port, `insecure_tls`, CA certificate…) stay settings,
  and free tags named like a secret (`password`, `token`, `key`…) or longer
  than 256 characters are not written to the metrics store.

- **Saving a device without `group_name` keeps its folder.** `PUT
  /api/targets/{id}` used to clear the folder when the field was left out;
  now only `""` or `null` clears it. The OpenAPI description is updated.

- **Self-signed certificates for self-hosted notification channels.** Matrix,
  Mattermost, Rocket.Chat, Zulip, ntfy, Gotify, Bark, Apprise, Home Assistant,
  Signal and custom webhook channels have an **Accept an unverifiable
  certificate** setting, off by default. See
  [Self-signed certificates](docs/notifications.md#self-signed-certificates).

- **Connection errors say what went wrong.** A failed notification or probe
  now names the cause, such as an untrusted certificate (`UnknownIssuer`),
  instead of a generic transport error.

- **Rocket.Chat messages no longer repeat the title.**

- **Forgejo and Gitea.** Disabled cron tasks are no longer reported as
  overdue, runners are read on Forgejo 16 (bare list), and `idle` and
  `active` runners count as online. See [Forgejo](docs/devices/forgejo.md).

- **GitLab no longer counts a 404 as a scrape error.** `/-/readiness`
  answers 404 outside the monitoring allowlist, and the licence endpoint on
  Community Edition. See [GitLab](docs/devices/gitlab.md#readiness-check).

- **VictoriaLogs on a fresh instance** no longer shows errors before the
  first logs arrive. See [VictoriaMetrics and VictoriaLogs](docs/devices/victoriametrics.md).

- **CrowdSec bouncer ages** are read correctly. See
  [CrowdSec](docs/devices/crowdsec.md).

- **Active Directory built-in groups** (binary SIDs) are no longer read as
  garbled text. See [Active Directory](docs/devices/activedirectory.md).

- **UniFi controller not set up yet** is reported as a configuration error
  with a clear message. See [UniFi](docs/devices/unifi.md).

- **Kubernetes crash loops are detected on k3s**, where the kubelet shows
  `terminated` instead of `CrashLoopBackOff`. See
  [Kubernetes](docs/devices/kubernetes.md).

- **Nextcloud no longer fails the first probe** with a 401 right after the
  token is set: it retries once. See [Nextcloud](docs/devices/nextcloud.md).

- **Alerts hidden because their own device is unreachable say so.** They are
  no longer shown as suppressed by the device itself as parent.

- **WordPress banner and PHP include.** The WordPress plugin accepts a
  DumbMonit server on a LAN address, shows admin notices when the address is
  invalid or the page cannot be fetched, and has translatable texts. The PHP
  include gets a `lang` option and logs failures. See
  [Website banner](docs/integrations/website-banner.md).

## 0.1.0-alpha.6 — 2026-10-06

### Security

- **Hosts that never bound to their agent are refused.** The transition
  window for hosts enrolled before agent binding is closed: their batches,
  container commands and relayed probes are now refused with a message that
  says what to do, and an agent too old to be bound is no longer enrolled. The
  device page shows **Not bound — re-enrol this host** with the *Allow
  re-enrolment* button; the next batch inside that window binds the host. See
  [Hosts that predate binding](docs/devices/agent.md#hosts-that-predate-binding).
- **The first admin account needs a setup code.** While no administrator
  exists, the server prints a one-time code in its logs at startup
  (`docker compose logs dumbmonit`), kept in memory and changed at every
  restart; `/setup` asks for it, and wrong codes are rate-limited. Someone else
  on the network can no longer claim a fresh instance first.
  `DUMBMONIT_SETUP_CODE` presets it for automated deployments.
- **Agent installs always verify the binary.** The install commands now carry
  the expected SHA-256 of each binary the server ships (`--sha256=…`,
  `-Sha256 …`), and `install.sh` / `install.ps1` refuse a download that does
  not match, that has no checksum, or that cannot be hashed — no more
  warning-only. `--insecure-skip-checksum` (`-InsecureSkipChecksum`) is the
  explicit, loud escape hatch for air-gapped setups. After a server upgrade,
  copy a fresh install command.
- **Single sign-on no longer creates accounts unless asked.** *Create accounts
  on first sign-in* is off by default, for new installs and wherever it was
  never set (`DUMBMONIT_OIDC_AUTO_CREATE` unset, or settings saved before the
  option existed); Settings → Single sign-on says so. A value saved explicitly
  is kept.
- **Hardening from the autumn audit.** The login lockout is keyed on account
  and address; agent tokens are checked before request bodies are read and
  heartbeat tokens are hidden from viewers; the OIDC login is bound to the
  browser that started it and spoofed forwarded headers are distrusted; the
  Windows agent's configuration directory is locked down; the private-address
  guard also covers website-change screenshots; a secret must be re-entered
  when its destination changes; the Prometheus read proxy and the
  VictoriaMetrics import stream are tightened; backup key derivation is capped
  and runs off the async runtime.

### Added

- **Integration packs.** A device type can now be added without a release: one
  YAML file declares HTTP or Prometheus sources on the device, the metrics to
  extract (JSONPath, regular expressions, Prometheus families, rows discovered
  in a list), its options and its alert rules, or SNMP profiles in the usual
  format. Installed through `POST /api/packs`, the type is available at once
  as `pack.<id>`, its metrics under `dumbmonit_<id>_`, its rules as
  `pack:<id>:<name>` (never rewritten by an update). Every request stays on
  the device's own address: other hosts, absolute URLs and redirects elsewhere
  are refused, loopback needs the device's explicit option, responses are
  capped at 4 MiB, credentials only go into headers. `dumbmonit pack lint` and
  `dumbmonit pack test` check a pack and replay its fixtures offline. Three
  reference packs: `node-exporter`, `shelly-plug`, `speedtest-tracker`. Packs
  are part of backups.

- **Status pages worth showing to users.** Each service's history bar now
  tells a day's minutes of downtime on hover and on keyboard focus (one tab
  stop, arrow keys between days), days without any measurement stay grey, and
  uptime is given over 30 days and, on a 90-day page, 90 days. **Badges** for
  a README: uptime (`?days=1|7|30|90`, never beyond the page's history) and
  response time next to the status badge, for the page and for each service
  (`/components/<key>/…`), flat colours readable on light and dark READMEs,
  cached a minute. **Look**: an uploaded logo (PNG, JPEG or WebP, 256 KiB,
  checked by content, never SVG), an accent from a closed set that stays
  readable by day and night, a footer text and a link to the organisation's
  site — no custom CSS or HTML. **Embed**: `/s/<slug>/embed`, a compact view
  for an iframe (`?theme=`, `?history=0`); it and the page are now the only
  framable addresses. **Email subscribers** through an SMTP channel you
  already have: double opt-in, an email per announcement, update and status
  change, a one-click unsubscribe link and `List-Unsubscribe-Post` in every
  message, a rate-limited public endpoint, and the list in the editor. The
  editor's new **Share** panel builds the badge and iframe snippets. A test
  walks every public route and checks none reveals a device name, address or
  type.

- **Server hardware over Redfish** (`redfish`): the management controller of a
  server (Supermicro, Dell iDRAC, HPE iLO, Lenovo XClarity, ASRock Rack,
  OpenBMC) read through the DMTF standard API with a read-only account. Fans,
  temperatures with the critical threshold each sensor declares, voltages, power
  supplies and their redundancy, consumption, drives with predicted failure and
  remaining SSD life, memory and processor summaries, system and controller
  health, and the event logs counted by severity (never read). Both generations
  of the schema — `Thermal`/`Power` and `ThermalSubsystem`/`PowerSubsystem`/
  `Sensors` — give the same `dumbmonit_redfish_…` series. `Status.State =
  Absent` or `Disabled` produces no health series, so an empty power supply bay
  or DIMM slot is never an alert. Six built-in rules: fan failed, temperature
  above critical, power supply redundancy lost, power supply failed, drive
  failure predicted, system health critical. Basic authentication by default;
  `auth = session` reuses one Redfish session and closes the one it replaces.

- **Redfish device page**: a server read over Redfish gets its own panel. It
  says first whether the server is healthy, in one word, with every component
  that drives it; then each fan and temperature against the thresholds the
  controller declares for that sensor (caution and critical temperatures, a
  fan's minimum, now collected as `temperature_upper_caution_celsius` and
  `fan_lower_critical_rpm`); whether power redundancy holds and the state of
  each supply; the drives with predicted failure and life left; memory,
  processor and controller summaries; and the event-log counts by severity.
  Redfish, TrueNAS, OPNsense, Mail Gateway and Datacenter Manager now sit with
  the other devices in the "Add a device" picker instead of under "Other".

- **Zyxel switch** SNMP profile, checked against a real XS1930-10: CPU (one and
  five minutes), memory and firmware from Zyxel's private branch, plus a new
  **Ethernet switch** profile (included by it, selectable by hand for any other
  switch) with per-port frame errors by cause from EtherLike-MIB (FCS,
  alignment, symbol, late and excessive collisions, MAC errors), the negotiated
  duplex, broadcast and multicast in 64-bit counters, and LLDP neighbours. The
  host resources profile gains the Net-SNMP load averages, which are all a
  Supermicro BMC's own agent offers about its CPU. Two built-in rules for every
  SNMP device: **Port accumulating errors** and **Port flapping**.

- **OPNsense** device (`opnsense`): the open source firewall that sits in front
  of most homelabs, read through its API with a key that can only look at
  status pages. Every gateway with its round-trip time and packet loss — the
  multi-WAN failover that happened three weeks ago and that nobody noticed is
  exactly the failure this is for — the pf state table against its configured
  limit, interface counters and the public address of the moment, WireGuard,
  OpenVPN and IPsec tunnels up or down, DHCP leases counted per server, the
  Unbound resolver, services, CARP (including the persistent maintenance mode
  everyone forgets to switch off), FreeBSD network buffers, temperatures and
  pending updates. `dumbmonit_opnsense_…` metrics, twelve built-in rules,
  `GET /api/targets/{id}/opnsense/…` and a documentation page. OPNsense writes
  `"none"` for a gateway that is fine and `"~"` for one it does not monitor:
  the first is read as *online*, the second produces one series fewer rather
  than a zero pretending to be a measurement. The firmware check is never
  triggered — only its last result is read, so a DumbMonit probe never sends
  the firewall to the update mirror.
- **TrueNAS** device (`truenas`): the ZFS pool whose mirror or RAIDZ vdev lost a
  disk and still serves its data — the failure nobody sees until the second
  disk goes — with the disk named on the device page; scrubs and resilvers in
  progress and the result of the last scrub, pool and dataset usage against
  their quotas, snapshot counts, replication and periodic snapshot tasks with
  their last run and error, disk temperatures read from TrueNAS's cache (a
  sleeping disk is never woken), SMART self-test results, the alerts TrueNAS
  raises itself, and services. `dumbmonit_truenas_…` metrics, thirteen built-in
  rules, `GET /api/targets/{id}/truenas/…` and a documentation page, which says
  plainly that TrueNAS's REST API accepts only a full-administrator key.

- **Five more kinds of service monitor**, each one beginning a real session in
  the service's own protocol rather than checking that a port opens.
  **SMTP** (`smtp`) runs the greeting, `EHLO`, `STARTTLS` and, if the target
  carries a credential, `AUTH` — and never sends a message; a refused password
  is reported as `reason="auth"`, not as an outage. **PostgreSQL**
  (`postgres`) and **MySQL/MariaDB** (`mysql`) connect, authenticate and run
  one query (`SELECT 1` by default, which reads no table), timing the
  connection and the query separately: the "the database is up but no longer
  answering" signal. Replace the query with one that returns a number and it
  becomes a chart. **MQTT** (`mqtt`) connects, subscribes to a topic and can
  wait for a retained message — when the broker dies, the whole house goes
  quiet without anything reporting an error. **WebSocket** (`websocket`) runs
  the upgrade handshake and verifies the server's `Sec-WebSocket-Accept`, a
  failure a plain HTTP check cannot see, and can send a frame and expect one
  back. All of them go through the same address guard, the same per-probe
  timeout and the same certificate reading as the existing probes: expiry,
  issuer and TLS version are recorded for a mail relay or a broker exactly as
  for a website, and no credential is ever sent over a chain that could not be
  verified.
- **The DNS monitor asserts what a zone answers.** Beyond the expected values
  it already checked, `expect_mode = exact` refuses any record the expectation
  does not cover — a hijacked zone usually keeps the legitimate address and
  adds one beside it — and `forbid` names values that must never come back: a
  former host's address, an expired validation `TXT`, a decommissioned mail
  server.

- **The agent runs on macOS and FreeBSD.** One installer, four init systems:
  systemd and OpenRC on Linux, **launchd** on macOS (a daemon in
  `/Library/LaunchDaemons`, bootstrapped into the system domain) and **rc.d**
  on FreeBSD (`/usr/local/etc/rc.d/dumbmonit_agent`, supervised by `daemon(8)`
  and enabled with `service dumbmonit_agent enable`). The script detects the
  system, picks the right paths and the right binary, and refuses what it
  cannot install with a sentence rather than a 404. A machine is recognised
  across renames everywhere: `/etc/machine-id` on Linux, the `IOPlatformUUID`
  on macOS, `kern.hostuuid` or `/etc/hostid` on FreeBSD — and an identifier
  made only of zeroes, which FreeBSD writes when the firmware gave it nothing,
  is rejected rather than shared by a whole fleet. Watched services are named
  as each system names them: systemd units, launchd labels, rc.d services.
  The FreeBSD binary (x86_64) is built with the image and served from
  `/download/…` like the others; the macOS binaries are built on a macOS runner
  and attached to each release, Apple's SDK not being redistributable — asking
  the server for one answers with the address to fetch it from.
- **Temperatures and fans.** The agent reads the probes the machine exposes:
  `/sys/class/hwmon` on Linux, the SMC on macOS, `dev.cpu.N.temperature` on
  FreeBSD. Each sensor also publishes the critical threshold **its own
  hardware declares**, which is what makes the new **Temperature above
  critical** rule work without anyone choosing a number — 85 °C is an emergency
  on a disk and an ordinary afternoon on a laptop CPU. Fans (Linux only, rpm)
  come with them. A reading outside 1–150 °C, or a fan at 0 rpm, is dropped:
  an empty fan header and a dead fan read the same, and a wrong number costs
  more than a missing one.
- **Disk health, through SMART.** When `smartctl` is installed, the agent
  reports each disk's own verdict (`agent_disk_smart_ok`), its temperature,
  power-on hours, wear, reallocated and pending sectors, and NVMe media
  errors — with a **Disk SMART failing** rule, the single most useful warning a
  homelab can get, since it usually arrives weeks before the failure. Sleeping
  disks are never woken to be measured (`-n standby`), serial numbers never
  leave the machine, and without `smartctl` nothing at all is published.
- **ZFS pools.** State, capacity, fragmentation, per-device error counters,
  permanently corrupted files, and the last scrub — its age, and whether it
  found anything. Two rules come with it: **ZFS pool degraded** and **ZFS scrub
  found errors**, the second being the one that breaks the silence of a pool
  that is still `ONLINE` while reading back damaged data. Covers FreeBSD,
  TrueNAS and the many Linux machines on OpenZFS at once, and stays completely
  silent where there is no `zpool`.

- **Many more integrations.** Nextcloud, Immich, Paperless-ngx, Jellyfin and
  Plex; Pi-hole, AdGuard Home, UPS through NUT and MikroTik RouterOS; UniFi
  Network, Home Assistant and VMware vSphere; Redis/Valkey, MongoDB, RabbitMQ
  and CrowdSec; Traefik, Caddy, Nginx Proxy Manager, domain expiry over RDAP
  and DNS answer changes; Kubernetes, WireGuard and restic/Borg; pfSense,
  Unraid, Veeam, Tailscale, FortiGate, Sophos Firewall and Hyper-V; Nginx,
  Apache and NTP; GitLab and Forgejo/Gitea; Synology Drive and Photos.
- **Website changes**: crawl a page, diff its text, keep snapshots and
  before/after screenshots.
- **Client devices.** Last-seen clients for Immich, Tailscale, UniFi and Home
  Assistant, protected devices for PBS and Veeam.
- **Active Directory over LDAP**: domain controllers, privileged groups and
  security findings.
- **A security score per device**, from the vendor's own best practices.
- **Device folders and manual order**, moved by dragging the whole row and
  carried through backup and restore.
- **Alerts you can get rid of**: acknowledge, snooze, ignore or clear, and a
  one-click dismiss with an undo on every alert card.
- **A new wall**: a calm ambient backdrop, an OLED theme, a living Paris
  rooftop with the pigeons, and a music player whose Spotify Connect speaker
  appears on your phone with visible diagnostics and a test sound.
- **An open, documented API** and an MCP server on protocol 2026-07-28, native
  agent binaries for every OS attached to GitHub releases, and a read-only
  public demo mode.
- **Pip the pigeon** gives tips across the interface, including setup tips on
  device pages, and the status page has a pigeon of its own.
- **Ready for community translation.** The web UI is wired for community translation,
  starting with Settings, with a language picker that appears once a second
  language ships.
- **A themed message on browsers too old to run the UI**, such as a TV,
  instead of a blank page.

### Changed

- The version and build commit sit discreetly at the bottom right and in
  Settings → About; the command-palette hint and the Admin badge are gone from
  the top bar.
- Server images are published for amd64 only for now.
- Optimisation pass: a non-blocking write buffer, lazily loaded device panels
  and steadier charts.
- Dependencies: jsonwebtoken 11, base64 0.23, Vite 8; Dependabot runs monthly,
  grouped, with majors ignored by default.

## 0.1.0-alpha.5 — 2026-09-25

### Added

- **Synology DSM 7.4, checked against a real NAS.** A read-only capture of a
  production DiskStation is now a test fixture, and it exposed a silent break:
  on DSM 7.2 and later a disk's remaining life arrives as an object rather than
  a number, which made the whole storage inventory unreadable — no volumes, no
  disks, no temperatures, no SMART, on every up-to-date NAS. Two more renamed
  fields were just as quiet: the overheating flag (`temperature_warning`) and
  DSM's bad-sector verdict (`sb_days_left_critical`), without which two of the
  built-in disk rules could never fire. All of them are now read in both their
  old and new forms. New series from the same call, which was already paying
  for them: `pool_status`, `pool_failed_disks`, `pool_total/used_bytes` and the
  same four for SSD caches, with a **Synology storage pool degraded** rule — a
  RAID 5 that loses a disk shows there first, because DSM keeps the volume on
  top of it at "normal" while it rebuilds.

- **Proxmox VE 9, checked against a real cluster.** The collector was replayed
  against a capture of a three-node production cluster, and that capture is now
  a test fixture: every response is parsed by the real code, so a field Proxmox
  renames shows up as a failing test rather than an empty panel. New series,
  all from calls already made: `node_cpu_iowait_percent` (time a node spends
  waiting on its disks), `node_ksm_shared_bytes` (memory reclaimed by page
  deduplication), `node_pve_packages_upgradable` and `node_reboot_required` (a
  newer kernel installed but not yet booted — both readable without the
  `Sys.Modify` privilege that `apt/update` demands), `ha_fencing_armed`
  (Proxmox VE 9 says whether the watchdog will actually fence),
  `guest_memory_host_bytes` and `guest_running_qemu_info` (the QEMU a VM was
  started with, which a live migration does not refresh).
- **Everything collected from a NAS and a hypervisor now reaches their page.**
  A Synology device gains a **Storage pools** section — RAID type, state in
  DSM's own word, failed disks, and how much of the pool is already handed to
  volumes — plus an **SSD cache** section that only appears when there is a
  cache. Redundancy lives in the pool, not in the volume: a RAID 5 that loses a
  disk keeps its volume at "normal" while it rebuilds, and until now that loss
  was invisible on the page. A Proxmox VE node card now states **Reboot
  required** in words, with the running and the installed kernel, along with
  the number of upgradable Proxmox packages, the time the node spends waiting
  on its disks and the memory KSM has reclaimed; the HA watchdog's state
  (*armed* or *standby*) is stated beside the panel title. A guest's memory
  cell shows how much more the VM occupies on the host than inside itself, when
  the difference is worth reading — that surplus is the emulation overhead the
  hypervisor pays.
- **Maintenance windows that survive the clock change.** A window can now
  recur **monthly** — on days of the month ("the 1st and the 15th") or on a
  weekday of the month ("the first Sunday", "the last Friday") — with a start
  time and a duration, so a monthly window may run longer than a day. A month
  without the occurrence is skipped rather than approximated: no 31st in
  February, no fifth Sunday in April 2026. Recurring windows now carry a
  **time zone** (an IANA name such as `Europe/Paris`) instead of only a fixed
  offset: "every Sunday from 02:00 to 04:00" stays at 02:00 local all year, and
  still lasts exactly two hours on the night the clock moves — it no longer
  drifts by an hour twice a year. Windows saved before this keep their fixed
  offset and behave exactly as they did. The list shows the next occurrence,
  computed by the server, and a device covered by a window now reads
  **Maintenance** on public status pages instead of red.
- **Routing by tag.** A notification channel can say which alerts it wants —
  by device tag (`site=cellar`), by device kind (`proxmox`), by rule, or a
  combination — under *Delivery options → Only some alerts*. Conditions on the
  same field read as *or*, different fields as *and*, and an *Except* row
  always wins. A preview under the editor lists which of your devices the
  filter selects right now, using the very filter the alerting engine runs, and
  one sentence says the whole rule back to you ("this channel receives
  advisories and above from devices tagged site=cellar, except devices tagged
  role=lab"). A channel with no filter keeps receiving everything, so nothing
  changes for existing instances. New route
  `POST /api/notify/match-preview`.
- **Escalation to a second channel.** *Alerts → Notifications → Notification
  policy* gained one row: if nobody acknowledges within a delay, the alert is
  also sent — once — to another channel. The delay counts from the moment the
  first message actually went out, not from the alert firing, so quiet hours or
  a failed delivery postpone the hop instead of firing it into the void;
  acknowledging the alert, or its clearing, stops it. The escalation message
  goes to that channel only, and crosses its severity floor, routing filter and
  quiet hours — it was named to wake someone up. One hop, no on-call rotation:
  point it at PagerDuty or Opsgenie if you need that. New policy fields
  `escalate_after_secs` and `escalate_channel`.

- **Backup and restore**, so that upgrading is no longer a bet. *Settings →
  Backup* exports the whole configuration — every device with its credentials,
  alert rules and per-device overrides, notification channels with their
  secrets, the notification policy, status pages and incidents, maintenance
  windows, accounts, agent and API tokens (as hashes, so the agents and scripts
  already out there keep working) and heartbeat tokens (as themselves, so the
  URLs your cron jobs call stay the same) — into one file. Its secrets are
  re-encrypted with a passphrase you choose, by Argon2id and AES-256-GCM, never
  with the instance key: the point is to restore onto a fresh instance that has
  its own. The header stays readable so you can tell what a file is before
  typing anything; a bundle written by a newer DumbMonit is refused with a
  sentence saying so, and a modified one or a wrong passphrase is refused the
  same way. Restoring is a **dry run by default** that reports, section by
  section, what it would create, update and leave alone — and it is not a
  simulation but the restore itself, run in a transaction that is rolled back,
  so the report cannot be wrong. Restoring never deletes anything, never
  duplicates a device (it matches on kind and address), and never overwrites an
  account that already exists. Account password hashes and 2FA secrets are left
  out unless you ask for them. New routes `GET`/`POST /api/backup` and
  `POST /api/backup/restore`, admin sessions only — a bundle holds every
  credential of the instance, and an API token does not get one.
- **Scheduled local backups of the database.** Once a day by default, keeping
  seven, into `/data/backups/`: an online, consistent copy written with SQLite's
  `VACUUM INTO` while the server keeps running — not a file copy — plus a copy
  of `secret.key` beside it, because that file is what decrypts device
  credentials and a database restored without it gives an instance that cannot
  talk to anything. Tunable with `DUMBMONIT_BACKUP_ENABLED`, `_DIR`,
  `_INTERVAL_HOURS` and `_KEEP`; nothing else in the directory is ever touched
  by the rotation. Three new series —
  `dumbmonit_instance_backup_last_success_seconds`, `…_last_status` and
  `…_size_bytes` — and a built-in rule, **DumbMonit backup did not run**, that
  fires when the last success is more than two days old. The Settings section
  shows the last run, the files kept and their size, and a **Back up now**
  button. New page: [Backup and restore](docs/install/backup.md).
- **The first five minutes are now a path, not an empty screen.** A new
  instance opens on a three-step guide under the bulletin — add your first
  device, connect a way to be told, check a message arrives — each step one
  click, each dismissible on its own, with **Skip the guide** for all three.
  The steps are ticked by the server's own facts, not by the interface: the
  last one turns green only once a message has really left a channel, so a
  channel created and never tested does not count as done. The state lives on
  the instance (`GET`/`PUT /api/onboarding`), so skipping it in one browser
  skips it in every browser and for every user. It disappears for good once the
  three steps are settled, and never appears at all on an instance that already
  had devices and a channel — upgrading does not hand an established homelab a
  tutorial. "Scan my network" is offered as the fast path and is now a
  shareable link (`/targets/new?scan=1`).
- **The interface installs on a phone home screen**: a web manifest, icons
  derived from the existing pigeon mark, an Apple touch icon and the day/night
  theme colours already in use. Deliberately **no service worker and no offline
  cache** — a monitoring screen that serves yesterday's state, or yesterday's
  build after an upgrade, is worse than no icon.

- **An existing Prometheus or Grafana can now read DumbMonit**, so nobody has
  to run a second monitoring stack. Three routes outside `/api`, all behind an
  API token with the `read` scope and none of them needing the anti-CSRF header
  a scraper cannot send: `GET /metrics` exposes the instance's own health in the
  Prometheus text exposition format (scheduler cycle and backlog, probes and
  failures per device kind, samples written and write failures, alerting cycle,
  alerts per phase, notifications sent and failed per channel kind, agents and
  stale agents, database size, VictoriaMetrics reachability, version and
  uptime — a few dozen series, never one per device); `GET /federate` federates
  the measurements themselves by `match[]` selector, the way Prometheus reads
  another Prometheus; and `/prometheus/api/v1/…` relays the read half of the
  Prometheus API so Grafana can use `http://…/prometheus` as a plain data
  source, with the routes that delete series answering `404`.
  `DUMBMONIT_METRICS_PUBLIC=true` serves the three without a token, for
  instances whose port is already firewalled. Documented in the new
  [Scraping DumbMonit](docs/reference/metrics.md) section, with a copy-pastable
  `scrape_config`.
- **Proxmox Backup Server** now watches the server itself and its tape tier.
  Two new panels on the device page: **Server** (the systemd units with the
  ones PBS cannot do without called out, the Proxmox package versions —
  installed, available, running — with a "Restart pending" plate when the
  daemon is behind its package, the certificate and its expiry, and the
  traffic-control rules with what they are carrying) and **Tape**, shown only
  when there is a tape tier (backup jobs with their pool, drive and next tape,
  media pools with their retention and expired count, drives, changers and the
  tapes themselves). New `dumbmonit_pbs_…` families for services, package
  versions, certificate expiry and traffic limits; per datastore, the
  maintenance and removable-mount state, what is reading or writing it right
  now, how many machines of each type it holds and the growth measured per day
  from the history PBS itself keeps; and for garbage collection, the run
  duration, the chunk counters and the unreadable chunks it left in place.
  Seven built-in rules (service down, certificate expiring, restart pending
  after upgrade, tape backup failed, job never ran, corrupt chunks found,
  datastore not mounted) and five options (`services`, `datastore_details`,
  `traffic_control`, `certificates` off by default since PBS guards that call
  behind a write privilege, `tape` off by default). Garbage collection is now
  read with a single `/admin/gc` call for every datastore at once instead of
  one call per datastore, with the old call kept as a fallback.
- **PBS setup instructions corrected**: a token's effective privileges are the
  intersection of its own ACL and its **user's**, so every role has to be
  granted twice — to `dumbmonit@pbs` and to `dumbmonit@pbs!monitor`. Granted
  to the token alone, as the documentation previously said, nothing is refused
  and nothing is reported either: the calls succeed and return empty lists.
  The device page now explains how to check with
  `proxmox-backup-manager user permissions`, and records that `Remote.Audit`
  and `Tape.Audit` come from the separate `RemoteAudit` and `TapeAudit` roles,
  not from `Audit`.
- **Proxmox Mail Gateway** device (`pmg`): the postfix queues with the age of
  the oldest waiting message — the difference between slow mail and stuck
  mail —, today's mail counted and filtered (spam, viruses, bounces,
  greylisting, blocklist and SPF rejects, average processing time) with a
  recent traffic curve, the size of the spam and virus quarantines, the age of
  the ClamAV and SpamAssassin databases, services, certificates, pending
  updates and cluster sync state. `dumbmonit_pmg_…` metrics, nine built-in
  rules, `GET /api/targets/{id}/pmg/…`, and a documentation page. Quarantines
  are counted, never read: no subject, sender or message body leaves the
  gateway.
- **Proxmox Datacenter Manager** device (`pdm`): one device for the whole
  estate a console federates. Which Proxmox VE clusters and backup servers it
  reaches and the message it got back when it cannot, each instance's version
  with the ones left behind their peers, estate-wide guest, node, CPU, memory
  and storage totals, tasks that failed anywhere, and the console's own host
  (CPU, memory, root filesystem, certificates, pending updates, subscription).
  `dumbmonit_pdm_…` metrics, six built-in rules, `GET /api/targets/{id}/pdm/…`,
  and a page in the documentation explaining when to prefer it over a device
  per cluster.
- **Proxmox VE** now reads the cluster inventory in one call
  (`/cluster/resources`): the guests of a node that stopped answering stay
  listed instead of vanishing from the panel at the moment you want to look at
  them, resource pools and guest locks appear, and each probe makes two calls
  fewer per node. The guests table gains a pool filter, a lock badge, and the
  operating system and IP addresses of each guest — read from inside once an
  hour, not at every probe.
- Proxmox VE device page: a **Nodes** section above the guests, one card per
  node with its CPU, memory and root filesystem, the version installed on it,
  the Proxmox daemons that are not running, the interfaces set to start at boot
  that are down, and the fill of each LVM thin pool (data *and* metadata) and
  volume group. `GET /api/targets/{id}/proxmox/nodes`.
- Proxmox VE device page: a **Ceph** section, shown only when the cluster has
  Ceph — health, capacity, a table of OSDs with usage and latency, a table of
  pools, the CephFS filesystems, the OSD flags left set after a maintenance and
  the health checks currently muted. Also the HA manager's own view (which node
  holds the master, the state of each local resource manager and whether it is
  still reporting) and, per scheduled backup job, which disks of the guests it
  covers it actually writes. `GET /api/targets/{id}/proxmox/ceph`.
- Eleven built-in Proxmox VE rules: thin pool and thin pool metadata almost
  full, core service down, node interface down, Ceph OSD down, Ceph OSD and
  Ceph pool nearly full, `noout` flag left on, HA manager not reporting, backup
  job excluding a disk, and guest locked for six hours.
- Proxmox VE, optional and off by default: ingest the full RRD stream
  (`/cluster/metrics/export`), everything Proxmox's own metric servers receive,
  including per-node pressure stall information and every point since the last
  probe rather than just the current value.
- **Agents are bound to their machine.** An enrolment token said that a machine
  was allowed to talk; it never said *which* one, and the identity key that did
  (`/etc/machine-id`, or the host name) is no secret. One compromised host in a
  fleet could therefore push fake healthy measurements under a neighbour's
  device, rewrite its host name and OS, and — worse — fetch and consume the
  Docker restart/update commands queued for it. Now the server hands each
  machine a binding secret at its first batch, keeps only a fingerprint, and
  requires it on every request afterwards (`X-DumbMonit-Agent-Secret`); the
  agent stores it next to its configuration with mode `0600`. Nothing breaks on
  upgrade: agents installed before this keep reporting and keep running
  commands, shown as **not bound** on their device page until the install
  command is re-run on them — the support window for unbound agents ends with
  1.0. After a reinstall, *Agent → Allow re-enrolment* on the device page opens
  a one-hour window for the machine to bind itself again.
- **Enrolment tokens have a scope.** A token is now **single use** by default —
  it enrols one machine and no more, which is what an install command copied
  into a ticket or a shell history should be worth. *Settings → Agents* can
  make one reusable for a fleet, with an optional count of machines and an
  optional deadline; a deadline stops *enrolments* only, so machines already
  enrolled keep reporting. Tokens created before this stay unlimited.
  `POST /api/agent/tokens` gained `reusable`, `max_uses` and `expires_in_days`;
  the listing gained `max_uses`, `uses` and `expires_at`.
- **A real Content-Security-Policy.** Responses carried only
  `frame-ancestors 'none'`; they now carry a policy that starts from
  `default-src 'none'` and names only what the interface uses — all of it
  served by DumbMonit itself, with no third-party origin anywhere. The page's
  inline scripts are allowed by a nonce regenerated for every response, so
  `'unsafe-inline'` never applies to scripts.

### Fixed

- **Proxmox: a cluster without Ceph, or a VM whose guest agent is stopped, is no
  longer reported as a failure.** Proxmox answers HTTP 500 for "this feature is
  not installed", the same code it uses for a broken node; the collector now
  tells the two apart from the message and stays silent on absence.
- **Proxmox: VM memory no longer reads above 100%.** Proxmox VE 9 reports, in
  the cluster inventory, the memory a VM takes on the host — overhead included,
  so above its allocation. The "memory almost full" alert would have fired on
  almost every VM of a PVE 9 cluster. The percentage now comes from the guest's
  own view; the host figure is published separately as `guest_memory_host_bytes`.
- **Proxmox: SAS and SATA disks now report their temperature.** Their SMART
  report words it differently from an NVMe, and the line was being skipped.
- **Proxmox: a Proxmox Backup Server datastore no longer shows as an empty
  storage.** PVE reports no capacity for it; three zero-byte gauges read as a
  backup target with nothing in it.
- The server accepted a command report of any size from an agent. It is now cut
  to the last 4 KB like the agent's own cap, with
  `[truncated by the server, beginning dropped]` at the front so the report says
  it was shortened.
- "You cannot restart or update DumbMonit's own container" was true only of its
  name: naming the container by its id got past the check, and the server's
  reserved list held `dumbmonit` twice where the second entry should have been
  `ezymonit`. The agent now re-checks the name Docker reports after inspecting
  the container, whichever way it was designated, and refuses to act on its own
  container whatever that one is called.

## 0.1.0-alpha.4 — 2026-09-22

### Added

- Acknowledge an alert ("I know, stop reminding me"): **Ack** on every
  alert card — Overview, Alerts, device page — for 1 h, 4 h, 24 h or until
  resolved, with an optional note. Reminders and escalations pause, the
  resolution is still notified, and the ack clears when the alert resolves.
  Acked alerts move to a quieter "Acknowledged" group and leave the
  "Needs you" count. `POST`/`DELETE /api/alerts/{fingerprint}/ack`, the
  `acknowledge_alert` assistant tool, and an audit-log entry.
- Heartbeat (push) monitors: a cron job, backup script or automation calls
  `GET|POST /api/push/<token>` each time it runs; a missed call (expected
  interval + grace, set per device) or a `?status=down` report raises the new
  "Heartbeat missed" rule. Token shown and regenerable on the device page.
- API tokens (`dmt_…`) now authenticate the whole REST API as
  `Authorization: Bearer`, not only the MCP endpoint: `read` acts as a
  viewer, `write` as an administrator, without cookie or CSRF header. Tokens
  never manage accounts, sign-in settings or other tokens. The settings
  section becomes "API & assistants"; the API reference documents every
  route.
- Device page: a relayed device says *via <agent>* next to *Behind <parent>*.

### Changed

- **Documentation caught up with the product.** The built-in rules table is
  regenerated from the code and now lists all 101 rules grouped by device kind
  (Docker containers, Plakar backups and the Synology Active Backup task rules
  were missing; the hysteresis of *High CPU* and *Disk almost full* was not
  stated). The Settings page description matches the panels that ship. The FAQ
  and the HTTP API reference no longer describe "one instance password": they
  describe accounts, roles, the TOTP login exchange as it really works, and
  `DUMBMONIT_RESET_PASSWORD` as what it does — remove every account and
  session, leaving devices, rules and channels alone. The API reference gains
  the MCP endpoint, which was undocumented, and several corrected payloads.
  The README feature list gains alert acknowledgement, the second factor, the audit
  log and the scoped REST API tokens, and its "known gaps" list no longer names
  three things that have since shipped.

### Removed

- The `/dev/sky` preview board, a development-only page for the weather window,
  no longer ships in the production build.

### Fixed

- Two-factor sign-in: a code is accepted once (RFC 6238) — the code that just
  signed you in, or enabled the second factor, no longer works again within
  its clock window with a fresh password step; and the sign-in rate limit is
  no longer reset by the password step of a two-step sign-in, so wrong codes
  keep counting across attempts.
- Dependency suppression: a parent or relay agent whose probe already says
  *unreachable* suppresses its descendants at once, instead of only once its
  own "Device unreachable" alert fires — a stopped relay agent used to make
  each of its devices notify a cycle or two before the relay itself.
- `POST /api/targets` answered `via_agent: null` for a device created with a
  relay (the relay was stored; `GET` showed it).

## 0.1.0-alpha.3 — 2026-09-21

### Fixed

- Telegram rejected every notification whose device name contained an
  underscore (legacy Markdown mode); messages are now sent as escaped HTML.
- Upgrading a volume created by alpha.1: the server now stops with the
  exact `chown` command to run instead of a bare "Permission denied".
- Alert summaries no longer list the device's own tags and internal ids.
- Screenshots regenerated; new ones for the Proxmox VE, PBS and Synology
  panels.

## 0.1.0-alpha.2 — 2026-09-18

### Added

- Proxmox VE: guests panel (status, CPU, RAM, disk size and usage, network,
  uptime, last backup, HA), node disk health and wear, ZFS pools, security
  updates and changed packages, matching rules.
- Proxmox Backup Server: 30-day backup calendar per group, failures list with
  task logs, sync/verify/prune/GC jobs with their last result, disk health.
- Synology: volumes and disks (SMART, temperature, SSD remaining life), and a
  rhythm-aware Active Backup for Business monitor that learns each device's
  usual backup cadence before calling it overdue.
- Remote agent: `ghcr.io/noekan/dumbmonit-agent` Docker image and relay mode
  (`via_agent`) to monitor another network through an agent.
- Two-factor authentication (TOTP with recovery codes), CSRF protection,
  per-IP and per-user login limits, SSRF guard on HTTP monitors, checksum
  verification of agent downloads, hardened container.
- Status pages get their own top-level page; notification channels and
  policy move under Alerts; Settings keeps administration only.

### Changed

- Proxmox VE and PBS credentials are entered as Token ID + Secret; setup
  guides create a dedicated read-only user instead of root/admin.
- Plakar is detected automatically on the agent host and stays invisible
  when absent.
- Copy buttons work on plain-http (LAN) deployments.
- The container runs as user 65532 with no capability and a read-only root
  file system. **Upgrading from alpha.1**: the data volume must be handed over
  once — `docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data`
  (the server says so at startup).

### Added

- Remote sites: an agent with `relay: true` runs, on the server's behalf, the
  probes of the devices assigned to it (*Reached through* on the device form
  — SNMP, Proxmox VE, PBS, Synology, HTTP, TCP, DNS, ping, TLS), from its own
  network, over its existing outbound connection. A relay that goes silent
  suppresses the alerts of its devices like a parent would.
- Agent image `ghcr.io/noekan/dumbmonit-agent` (same tags as the server) and
  `docker-compose.agent.yml`, for Docker hosts and remote sites.

### Changed

- The network collectors moved to the `dumbmonit-collectors` crate, shared by
  the server and the agent; the server re-exports them under their old paths.

## 0.1.0-alpha.1 — 2026-09-16

First tagged build, for early testers: `ghcr.io/noekan/dumbmonit:0.1.0-alpha.1`
(also `:latest`). Everything is still moving; see the README status section.

### Added

- Sources: SNMP (auto-profiled), Proxmox VE and PBS, Synology DSM and Active
  Backup for Business, Linux/Windows agent with Docker (restart, update with
  rollback, prune) and Plakar backups, HTTP/TCP/DNS/ping/TLS monitors,
  network discovery.
- Alerting: 41 built-in rules, dependency suppression, seasonal baseline,
  forecasts, maintenance windows, notification policy (hysteresis, flap hold,
  cooldown, quiet hours, batching, hourly cap), 22 channels.
- Accounts (admin/viewer), OIDC single sign-on, API tokens, built-in MCP
  server for assistants, public status pages, wall mode, command palette.
- Single container: the server runs the VictoriaMetrics binary shipped in
  the image as a child process.

### Fixed

- Alerts are keyed on the device id: deleting, pausing or renaming a device
  no longer leaves phantom "Device unreachable" alerts.
- Queued agent commands expire after 10 minutes and can be cancelled; the UI
  says when an agent must be reinstalled to run commands.
- OIDC only links an existing local account on a verified e-mail, never a
  password-holding admin; open redirect on login closed; security headers.
- Phone layouts in Settings and Alerts; status page banner; notification
  lines name the VM/container/service concerned.

### Changed

- **EzyMonit is now DumbMonit.** Every identifier follows: the crates and
  binaries (`dumbmonit-server`, `dumbmonit`, `dumbmonit-agent`), the image
  (`ghcr.io/noekan/dumbmonit`), the environment variables (`DUMBMONIT_*`), the
  database file (`/data/dumbmonit.db`), the session cookie
  (`dumbmonit_session`), the agent token prefix (`dmon_`), the agent paths
  (`/etc/dumbmonit/agent.yaml`, `/usr/local/bin/dumbmonit-agent`, unit
  `dumbmonit-agent.service`; on Windows the `DumbMonitAgent` service under
  `C:\ProgramData\DumbMonit`) and the metric prefix (`dumbmonit_`).
  Compatibility:
    - `EZYMONIT_*` variables are still read as a fallback, with one startup
      warning per variable (`EZYMONIT_X is deprecated, use DUMBMONIT_X`).
    - `/data/ezymonit.db` is renamed to `dumbmonit.db` at startup.
    - Agent tokens `ezym_…` keep working; newly created ones are `dmon_…`.
    - The session cookie changed name: every user logs in again once. The
      browser's theme preference is migrated.
    - The metric prefix changed from `ezymonit_` to `dumbmonit_` with no
      compatibility (pre-1.0). Existing series stay in VictoriaMetrics under
      the old name, orphaned: graphs restart from the upgrade and the old data
      ages out with the retention. Custom alert rules written with the old
      prefix must be edited.
- **One container.** The image ships the VictoriaMetrics binary
  (`/victoria-metrics-prod`, from `victoriametrics/victoria-metrics:v1.152.0`).
  When `DUMBMONIT_VM_URL` is unset, the server starts it as a child process on
  `127.0.0.1:8428`, stores its series under `/data/vm` (the same volume as the
  database and `secret.key`), forwards its log lines into the server log,
  restarts it with a backoff if it dies and stops it at shutdown (SIGTERM, 20 s
  of grace, then kill). Setting `DUMBMONIT_VM_URL` keeps using an external
  instance, as before.
- New environment variables for the embedded VictoriaMetrics:
  `DUMBMONIT_VM_BINARY` (default `/victoria-metrics-prod`),
  `DUMBMONIT_VM_LISTEN` (default `127.0.0.1:8428`), `DUMBMONIT_VM_RETENTION`
  (default `12` months; `30d`, `2y`), `DUMBMONIT_VM_MEMORY` (default `256MB`).
  The flags `-search.maxUniqueTimeseries=100000`,
  `-search.maxConcurrentRequests=4` and `-dedup.minScrapeInterval=10s` are set
  by the server.
- `docker-compose.yml` declares the project name `dumbmonit` and a single
  volume with a fixed name, `dumbmonit-data`. The development overlay sets
  `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` and publishes port 8428.
- `GET /api/health` reports `victoria: {ok, embedded}`; `embedded` is `true`
  when the server runs its own VictoriaMetrics.
- The agent installers (`install.sh`, `install.ps1`) migrate an existing
  `ezymonit-agent` / `EzyMonitAgent` installation in place: old service stopped
  and removed, configuration moved to the new path, new service installed.

### Removed

- The `victoriametrics` service of `docker-compose.yml`, and with it the
  `vm-data` volume and the `depends_on` between the two containers.

### Migration

From the two-container EzyMonit deployment, whose Compose project was named
`ezymonit`: stop it with `docker compose down` in the old directory, copy its
two volumes into the new one, then start the new Compose file.

```bash
docker volume create dumbmonit-data
docker run --rm -v ezymonit_ezymonit-data:/from -v dumbmonit-data:/to alpine cp -a /from/. /to/
docker run --rm -v ezymonit_vm-data:/from -v dumbmonit-data:/to alpine sh -c 'mkdir -p /to/vm && cp -a /from/. /to/vm/'
docker compose up -d
```

The old VictoriaMetrics data directory has the layout the embedded one uses.
Keeping the external VictoriaMetrics is the alternative: set `DUMBMONIT_VM_URL`
and skip the second copy.

On each monitored machine, re-running the agent install one-liner migrates the
old `ezymonit-agent` in place; the existing `ezym_…` token can be reused.
Details in the documentation: [Install with Docker](docs/install/docker.md#upgrading)
and [Linux and Windows agent](docs/devices/agent.md#upgrade).
