# Device types

DumbMonit calls everything it watches a *device* (the API says *target*), even
when it is a web page or a DNS name. Each type is a collector on the server; the
server describes every type to the interface (`GET /api/collectors`), so the
form always matches what this version can monitor.

| Type | `kind` | Credential | Typical use |
|---|---|---|---|
| [SNMP device](snmp.md) | `snmp` | community or SNMP v3 | Switch, router, NAS, UPS, printer |
| [Proxmox VE](proxmox.md) | `proxmox` | API token or username/password | Hypervisor, cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | API token or username/password | Backup server |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | API token | Console federating several PVE clusters and PBS instances |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | username/password | Mail gateway filtering spam and viruses |
| [VMware vSphere](vsphere.md) | `vsphere` | username/password | vCenter or standalone ESXi: hosts, VMs, datastores, alarms |
| [Synology DSM](synology.md) | `synology` | username/password | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | API key and secret | Firewall, router, multi-WAN edge |
| [pfSense](pfsense.md) | `pfsense` | REST API key (read-only user) | Firewall: gateways down or lossy, interfaces without link, stopped services |
| [FortiGate](fortigate.md) | `fortigate` | REST API token (read-only profile, trusted host) | Firewall: IPsec tunnels, links, HA sync, conserve mode, FortiGuard licences |
| [Sophos Firewall](sophos.md) | `sophos` | username/password (read-only profile) | Firewall: interface links only (the XML API exposes no tunnel, HA or licence state) |
| [UniFi Network](unifi.md) | `unifi` | API key or username/password | UniFi console or self-hosted server: devices, WAN, clients |
| [Home Assistant](homeassistant.md) | `homeassistant` | long-lived access token | Smart home: unavailable entities, low batteries, updates, repairs |
| [TrueNAS](truenas.md) | `truenas` | API key | ZFS storage server (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | API key (Viewer role) | NAS: array, disks, parity checks, cache pools, containers, VMs |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | username/password (read-only group) | Router or switch: versions and firmware behind, CPU, memory, sensors, interface errors |
| [UPS with NUT](nut.md) | `nut` | none, or username/password | UPS on a NAS, Raspberry Pi or server running Network UPS Tools |
| [Server hardware (Redfish)](redfish.md) | `redfish` | username/password | Server fans, temperatures, power supplies and drives, read from its BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | none, username/password or token | Time series database: ingestion, refused samples, disk headroom |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | none, username/password or token | Log database: ingestion, refused lines, disk headroom |
| [Grafana Loki](loki.md) | `loki` | none, username/password or token | Log server: readiness, refused lines, flush failures |
| [Graylog](graylog.md) | `graylog` | access token or username/password | Log server and its OpenSearch cluster: journal, buffers, throughput, inputs |
| [Pi-hole](pihole.md) | `pihole` | app password, or none | DNS ad blocker: blocking state, blocked share, blocklist age, updates |
| [AdGuard Home](adguard.md) | `adguard` | username/password | DNS filter: protection, blocks, filter lists, upstream servers, updates |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | none, or email address/password | Mail server: SMTP, IMAP, POP3, webmail, version |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | none, or API key | Mail gateway: services, delivery queue and other counters |
| [Redis / Valkey](redis.md) | `redis` | ACL user, password only, or none | In-memory store: memory against maxmemory, replication, persistence, keys |
| [MongoDB](mongodb.md) | `mongodb` | username/password (clusterMonitor) or none | Document database: replica set, lag, connections, WiredTiger cache |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | username/password (monitoring tag) | Message broker: resource alarms, nodes, partitions, queues without consumer |
| [CrowdSec](crowdsec.md) | `crowdsec` | none, or bouncer API key | Security engine: decisions, alerts, bouncers that stopped pulling, log reading |
| [Traefik](traefik.md) | `traefik` | none, or basicAuth user | Reverse proxy: disabled routers, backend servers down, certificates not obtained or not renewed, 5xx share |
| [Caddy](caddy.md) | `caddy` | read-only view user, or none | Web server and reverse proxy: upstream health, failed reloads, handler errors, 5xx share |
| [Nginx Proxy Manager](npm.md) | `npm` | user email and password (view only) | Proxy manager: hosts nginx refused, disabled hosts, certificates served |
| [Domain expiry](domain.md) | `domain` | none | Registered domain over RDAP: days before expiry, hold, redemption period, registrar |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | service account token (read-only ClusterRole) | Cluster: nodes not ready, pods crash looping or pending, workloads missing replicas, stuck volume claims |
| [Active Directory](activedirectory.md) | `activedirectory` | username/password (plain domain user) | Windows domain over LDAPS: domain controllers and FSMO roles, privileged group members, password policy, security findings |
| [Veeam Backup & Replication](veeam.md) | `veeam` | username/password (Veeam Backup Viewer) | Backup server: failed jobs, failed sessions, repositories filling up, license |
| [Tailscale](tailscale.md) | `tailscale` | OAuth client (devices:core:read) or API access token | Tailnet: servers offline, node keys expiring, devices awaiting approval |
| [Nextcloud](nextcloud.md) | `nextcloud` | monitoring token (NC-Token) | File sync: maintenance mode, pending upgrade, updates, active users, OPcache |
| [Immich](immich.md) | `immich` | API key | Photo library: job queues, updates, disk, photo counts |
| [Paperless-ngx](paperless.md) | `paperless` | API token or username/password | Documents: Redis, Celery, index, failed imports, updates |
| [Jellyfin](jellyfin.md) | `jellyfin` | API key | Media server: failed scheduled tasks, plugins, streams and transcodes |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, or none from an allowed network | Media server: streams, transcodes, libraries, updates |
| [Server with agent](agent.md) | `agent` | none (enrollment token) | Linux, Windows, Raspberry Pi, [Hyper-V host](agent.md#hyper-v-hosts) |
| [Website or web API](services.md#http) | `http` | none, username/password or token | Health page, REST API |
| [Network port](services.md#tcp) | `tcp` | none | SSH, SMB, database |
| [Domain name](services.md#dns) | `dns` | none | Your domain, an internal name |
| [Reachable host](services.md#ping) | `ping` | none | Gateway, access point, printer |
| [TLS certificate](services.md#tls) | `tls` | none | IMAPS, LDAPS, reverse proxy |
| [Heartbeat](push.md) | `push` | none (secret URL) | Cron job, backup script, automation that must call in |
| [Website changes](webchange.md) | `webchange` | none | A page or a whole site: text changes, new and removed pages, before/after with screenshots |
| [Demo device](demo.md) | `dummy` | none | Explore the UI without hardware |

## How a device is read

The scheduler runs every enabled device on its own interval (default 60 s,
minimum 10 s) through the collector for its kind, with at most
`DUMBMONIT_MAX_CONCURRENT_PROBES` probes in flight and a hard timeout of
`DUMBMONIT_PROBE_TIMEOUT_SECS` per probe. Each successful probe writes its
samples plus `dumbmonit_up = 1`; a failed probe writes nothing, and it is the
silence of the series that the "Device unreachable" rule detects.

Failures are classified. A device that does not answer is *unreachable* and
raises an alert. A wrong community, a refused certificate or a missing
capability is a *configuration error*: it is shown on the device page, in red,
with the reason, and it is not notified. The distinction matters: a misconfigured
device should not wake anyone up at night.

## Options

Some types read options from the device's tags: the API port, whether to accept
a self-signed certificate, which nodes to monitor. The form shows a typed field
per option under **More options**, with its default and help text. The per-kind
pages list them verbatim. Options are copied on every series as `tag_<key>`
labels, so never put a secret in one: use the credential field, which is
encrypted.

## Parent devices

Any device can declare another one as its parent. When the parent is
unreachable, alerts from its descendants are marked *Suppressed by parent*
instead of being sent: a switch going down produces one notification, not
thirty. On the Devices page, children stack under their parent and dim when it
is unreachable.
