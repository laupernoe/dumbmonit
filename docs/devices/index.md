# Monitor types

DumbMonit calls everything it watches a *monitor* (the API says *target*), even
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
| [XCP-ng / XenServer](xcpng.md) | `xcpng` | username/password (read-only XAPI role) | Pool through XAPI: hosts, VMs, storage repositories, high availability |
| [Nutanix AHV](nutanix.md) | `nutanix` | username/password (Prism Viewer) | Prism Central v4 API: clusters, hosts, AHV VMs, storage containers, alerts |
| [Synology DSM](synology.md) | `synology` | username/password | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | API key and secret | Firewall, router, multi-WAN edge |
| [pfSense](pfsense.md) | `pfsense` | REST API key (read-only user) | Firewall: gateways down or lossy, interfaces without link, stopped services |
| [FortiGate](fortigate.md) | `fortigate` | REST API token (read-only profile, trusted host) | Firewall: IPsec tunnels, links, HA sync, conserve mode, FortiGuard licences |
| [Sophos Firewall](sophos.md) | `sophos` | username/password (read-only profile) | Firewall: interface links only (the XML API exposes no tunnel, HA or licence state) |
| [Cisco Meraki](meraki.md) | `meraki` | API key (read-only administrator) | Beta. Meraki cloud: devices offline, failed Internet uplinks, license state and expiry, clients per network |
| [TP-Link Omada](omada.md) | `omada` | OpenAPI client ID and secret (Viewer role) | Beta. Omada controller: devices disconnected or pending, CPU and memory, firmware updates, clients per site |
| [Aruba Central](arubacentral.md) | `arubacentral` | API client (ID and secret) or access token | Beta. Aruba access points, switches and gateways through the cloud API: devices offline, critical alerts, device health, clients |
| [Palo Alto PAN-OS](paloalto.md) | `paloalto` | username/password or API key (role-based, XML API operational requests only) | Beta. Firewall: interface links, HA peer and sync, session table, licences, failed commits |
| [SonicWall SonicOS](sonicwall.md) | `sonicwall` | username/password (read-only user) | Beta. Firewall: interface links only |
| [OpenVPN server](openvpn.md) | `openvpn` | none, or management password | OpenVPN server through its management interface: version, state, connected clients |
| [UniFi Network](unifi.md) | `unifi` | API key or username/password | UniFi console or self-hosted server: devices, WAN, clients |
| [Home Assistant](homeassistant.md) | `homeassistant` | long-lived access token | Smart home: unavailable entities, low batteries, updates, repairs |
| [TrueNAS](truenas.md) | `truenas` | API key | ZFS storage server (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | API key (Viewer role) | NAS: array, disks, parity checks, cache pools, containers, VMs |
| [QNAP QTS / QuTS hero](qnap.md) | `qnap` | username/password (limited account) | NAS (beta): volumes filling up, disk SMART health and temperature, firmware updates |
| [Ceph](ceph.md) | `ceph` | dashboard user and password (read-only role) | Storage cluster (beta): health checks, OSDs, placement groups, capacity, pools, monitors |
| [MinIO](minio.md) | `minio` | Prometheus token (or none) | Object store (beta): nodes and drives offline, capacity, heal and replication failures, S3 errors |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | username/password (read-only group) | Router or switch: versions and firmware behind, CPU, memory, sensors, interface errors |
| [UPS with NUT](nut.md) | `nut` | none, or username/password | UPS on a NAS, Raspberry Pi or server running Network UPS Tools |
| [Server hardware (Redfish)](redfish.md) | `redfish` | username/password | Server fans, temperatures, power supplies and drives, read from its BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | none, username/password or token | Time series database: ingestion, refused samples, disk headroom |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | none, username/password or token | Log database: ingestion, refused lines, disk headroom |
| [Grafana Loki](loki.md) | `loki` | none, username/password or token | Log server: readiness, refused lines, flush failures |
| [Graylog](graylog.md) | `graylog` | access token or username/password | Log server and its OpenSearch cluster: journal, buffers, throughput, inputs |
| [Pi-hole](pihole.md) | `pihole` | app password, or none | DNS ad blocker: blocking state, blocked share, blocklist age, updates |
| [AdGuard Home](adguard.md) | `adguard` | username/password | DNS filter: protection, blocks, filter lists, upstream servers, updates |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | none (agent on the server required), or email address/password | Mail server: agent-read queues, sessions and Windows services, plus SMTP, IMAP, POP3, webmail, version |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | none (agent on the server required), or API key | Mail gateway: agent-read Windows services and queues, delivery queue and other counters |
| [Redis / Valkey](redis.md) | `redis` | ACL user, password only, or none | In-memory store: memory against maxmemory, replication, persistence, keys |
| [MongoDB](mongodb.md) | `mongodb` | username/password (clusterMonitor) or none | Document database: replica set, lag, connections, WiredTiger cache |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | username/password (monitoring tag) | Message broker: resource alarms, nodes, partitions, queues without consumer |
| [SQL Server](mssql.md) | `mssql` | SQL login (username/password) | Databases not online, blocking, connections, volume space, Agent jobs, AlwaysOn |
| [Oracle Database](oracle.md) | `oracle` | database account (username/password), through ORDS | Instance state, tablespaces, sessions, Data Guard lag, recovery area, scheduler jobs |
| [Elasticsearch / OpenSearch](elasticsearch.md) | `elasticsearch` | username/password, token or none | Cluster status, unassigned shards, node disk and heap |
| [InfluxDB](influxdb.md) | `influxdb` | none or API token | InfluxDB 2 and 3: health, readiness, uptime, memory, server errors |
| [Apache Kafka](kafka.md) | `kafka` | none or SASL/PLAIN over TLS | Offline and under-replicated partitions, controller, consumer group lag |
| [CrowdSec](crowdsec.md) | `crowdsec` | none, or bouncer API key | Security engine: decisions, alerts, bouncers that stopped pulling, log reading |
| [Traefik](traefik.md) | `traefik` | none, or basicAuth user | Reverse proxy: disabled routers, backend servers down, certificates not obtained or not renewed, 5xx share |
| [Caddy](caddy.md) | `caddy` | read-only view user, or none | Web server and reverse proxy: upstream health, failed reloads, handler errors, 5xx share |
| [Prometheus exporter](prometheus.md) | `prometheus` | none, basic auth or bearer token | Any exporter (node_exporter, windows_exporter, cAdvisor, blackbox…): every series stored, graphed and usable in rules |
| [OpenTelemetry source](otlp.md) | `otlp` | API token (write scope) | OTLP/HTTP receiver: metrics from an OpenTelemetry Collector or an application, logs relayed to VictoriaLogs |
| [Nginx](nginx.md) | `nginx` | none, or basic auth | Web server: active/waiting connections, accept/handled/request rates, NGINX Plus upstream health |
| [Apache httpd](apache.md) | `apache` | none, or basic auth | Web server: busy/idle workers, scoreboard by state, request and byte rates |
| [Nginx Proxy Manager](npm.md) | `npm` | user email and password (view only) | Proxy manager: hosts nginx refused, disabled hosts, certificates served |
| [Domain expiry](domain.md) | `domain` | none | Registered domain over RDAP: days before expiry, hold, redemption period, registrar |
| [Third-party service status](provider.md) | `provider` | none | A provider's public status page (Cloudflare, GitHub, Atlassian, AWS…): impact, open incidents, per-component filter; shown on status pages as an external component |
| [Mail server reputation](mail_blacklist.md) | `mail_blacklist` | none, or Spamhaus DQS key | Mail domain or server address against the DNS blocklists (Spamhaus, Barracuda, SpamCop…), reverse DNS, SPF, DMARC, delisting links |
| [IP range blocklist](ip_blacklist.md) | `ip_blacklist` | none, or Spamhaus DQS key | Every address of a public IPv4 range up to /24 against the DNS blocklists, with delisting links |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | service account token (read-only ClusterRole) | Cluster: nodes not ready, pods crash looping or pending, workloads missing replicas, stuck volume claims |
| [Active Directory](activedirectory.md) | `activedirectory` | username/password (plain domain user) | Windows domain over LDAPS: domain controllers and FSMO roles, privileged group members, password policy, security findings |
| [Veeam Backup & Replication](veeam.md) | `veeam` | username/password (Veeam Backup Viewer) | Backup server: failed jobs, failed sessions, repositories filling up, license |
| [Kopia](kopia.md) (beta) | `kopia` | server user name and password | Backup server: last snapshot of each source (age, errors, size), sources behind their schedule, repository connection |
| [Duplicati](duplicati.md) (beta) | `duplicati` | web interface password | Backup server (2.1+): last successful backup of each task, last result, warnings and errors, task running |
| [Tailscale](tailscale.md) | `tailscale` | OAuth client (devices:core:read) or API access token | Tailnet: servers offline, node keys expiring, devices awaiting approval |
| [Microsoft 365](m365.md) | `m365` | Entra application (client ID and secret) | Service health, incidents, licenses, risky users |
| [Microsoft Entra ID](entra.md) | `entra` | Entra application (client ID and secret) | Application secret expiry, directory sync, admin MFA |
| [Microsoft Azure](azure.md) | `azure` | Entra application (client ID and secret) with the Reader role on the subscription | Resource Health of every resource and active Azure Monitor alerts by severity |
| [AWS (CloudWatch)](aws.md) | `aws` | Access key ID and secret (read-only IAM user, optional role) | CloudWatch alarms in ALARM, EC2 status checks and CPU, RDS CPU and free storage |
| [Cloudflare](cloudflare.md) | `cloudflare` | API token (read-only) | Zones, tunnels, certificates, 5xx errors, load balancer pools |
| [Hetzner Cloud](hetzner.md) | `hetzner` | API token (read-only) | Servers, CPU, volumes, backups |
| [OVHcloud](ovh.md) | `ovh` | OVHcloud API token (application key, application secret and consumer key) with GET /vps* and GET /dedicated/server* | State and running tasks of the VPS and dedicated servers |
| [Infomaniak](infomaniak.md) | `infomaniak` | Infomaniak API token with the Products scope | Expiry dates, locks and maintenance of domains, hosting and other products |
| [Nextcloud](nextcloud.md) | `nextcloud` | monitoring token (NC-Token) | File sync: maintenance mode, pending upgrade, updates, active users, OPcache |
| [Immich](immich.md) | `immich` | API key | Photo library: job queues, updates, disk, photo counts |
| [Paperless-ngx](paperless.md) | `paperless` | API token or username/password | Documents: Redis, Celery, index, failed imports, updates |
| [Jellyfin](jellyfin.md) | `jellyfin` | API key | Media server: failed scheduled tasks, plugins, streams and transcodes |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, or none from an allowed network | Media server: streams, transcodes, libraries, updates |
| [GitLab (self-managed)](gitlab.md) | `gitlab` | personal access token | Forge: Sidekiq backlog, CI runners, pending migrations, sign-up/2FA settings, licence |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | access token | Forge: health check, scheduled tasks, Actions runners, repository/account counts |
| [Sonarr (beta)](sonarr.md) | `sonarr` | API key | Series: health, failing clients and indexers, queue errors, disk, updates |
| [Radarr (beta)](radarr.md) | `radarr` | API key | Movies: health, failing clients and indexers, queue errors, disk, updates |
| [Lidarr (beta)](lidarr.md) | `lidarr` | API key | Music: health, failing clients and indexers, queue errors, disk, updates |
| [Prowlarr (beta)](prowlarr.md) | `prowlarr` | API key | Indexers: health, indexers disabled after failures, disk, updates |
| [qBittorrent (beta)](qbittorrent.md) | `qbittorrent` | user name and password, or none | Torrent client: torrents in error, lost connectivity, speeds, free space |
| [UniFi Protect (beta)](unifiprotect.md) | `unifiprotect` | local user name and password | Cameras: offline or not recording, NVR disks and storage |
| [Frigate NVR (beta)](frigate.md) | `frigate` | none or bearer token | NVR: cameras without frames, detector hangs, frame rates, recording storage |
| [Keycloak (beta)](keycloak.md) | `keycloak` | none, or client ID and secret | Identity: readiness and database, realms, sessions, users |
| [authentik (beta)](authentik.md) | `authentik` | API token | Identity: readiness, workers, failed tasks, outposts, updates |
| [Vaultwarden (beta)](vaultwarden.md) | `vaultwarden` | none, or admin token | Password vault: liveness, version, open sign-ups, account counts |
| [mailcow (beta)](mailcow.md) | `mailcow` | read-only API key | Mail server: stopped containers, queue, mail volume, mailbox quotas |
| [Server with agent](agent.md) | `agent` | none (enrollment token) | Linux, Windows, Raspberry Pi, [Hyper-V host](agent.md#hyper-v-hosts) |
| [Website or web API](services.md#http) | `http` | none, username/password or token | Health page, REST API |
| [Network port](services.md#tcp) | `tcp` | none | SSH, SMB, database |
| [Domain name](services.md#dns) | `dns` | none | Your domain, an internal name |
| [Reachable host](services.md#ping) | `ping` | none | Gateway, access point, printer |
| [TLS certificate](services.md#tls) | `tls` | none | IMAPS, LDAPS, reverse proxy |
| [SMTP server](services.md#smtp) | `smtp` | none, or username/password | Mail server: greeting, STARTTLS, certificate, optional login |
| [PostgreSQL](services.md#postgresql) | `postgres` | username/password | Database: connection and a query you choose |
| [MySQL or MariaDB](services.md#mysql-and-mariadb) | `mysql` | username/password | Database: connection and a query you choose |
| [MQTT broker](services.md#mqtt) | `mqtt` | none, or username/password | Broker: connection, subscription, retained message |
| [WebSocket](services.md#websocket) | `websocket` | none, username/password or token | WebSocket endpoint: handshake, optional message exchange |
| [NTP time server](services.md#ntp) | `ntp` | none | Router, NAS, internal time server, public pool server |
| [SNMP value](services.md#snmp-oid) | `snmp_oid` | SNMP community or v3 user | One SNMP object: sensor, UPS status, version string, with bounds or expected text |
| [RADIUS server](services.md#radius) | `radius` | shared secret and test account | FreeRADIUS, Windows NPS: a real authentication |
| [gRPC service](services.md#grpc) | `grpc` | none | Microservice, etcd: the standard gRPC health check |
| [Heartbeat](push.md) | `push` | none (secret URL) | Cron job, backup script, automation that must call in |
| [Website changes](webchange.md) | `webchange` | none | A page or a whole site: text changes, new and removed pages, before/after with screenshots |
| [Website health & SEO](website.md) | `website` | none | Sites in bulk: down, blank page, redirects, noindex, robots.txt, title/description/canonical changes, platform (CSV import) |
| [Demo monitor](demo.md) | `dummy` | none | Explore the UI without hardware |

## How a monitor is read

The scheduler runs every enabled monitor on its own interval (default 60 s,
minimum 10 s) through the collector for its kind, with at most
`DUMBMONIT_MAX_CONCURRENT_PROBES` probes in flight and a hard timeout of
`DUMBMONIT_PROBE_TIMEOUT_SECS` per probe. Each successful probe writes its
samples plus `dumbmonit_up = 1`; a failed probe writes nothing, and it is the
silence of the series that the "Monitor unreachable" rule detects. The monitor
page shows the failure at the next probe, but the alert waits on purpose: it
fires about four minutes after the last successful probe (three minutes of
silence, then a one-minute hold), so a single lost probe never pages anyone.

Failures are classified. A monitor that does not answer is *unreachable* and
raises an alert. A wrong community, a refused certificate or a missing
capability is a *configuration error*: it is shown on the monitor page, in red,
with the reason, and it is not notified. The distinction matters: a misconfigured
monitor should not wake anyone up at night.

## Options

Some types read options from the monitor's tags: the API port, whether to accept
a self-signed certificate, which nodes to monitor. The form shows a typed field
per option under **More options**, with its default and help text. The per-kind
pages list them verbatim. Options are settings, not labels: they are not copied
on the series. Free tags (any other key) are copied on every series as
`tag_<key>` labels, except keys that name a secret (matched per word: `password`, `token`,
`key`, `cert`, `secret`, `pass`, `private`, `pem`…, so `primary_key` is skipped
but `keyboard` is not) and values longer than 256 characters. Never put a secret in
a tag anyway: use the credential field, which is encrypted.

## Parent monitors

Any monitor can declare another one as its parent. When the parent is
unreachable, alerts from its descendants are marked *Suppressed by parent*
instead of being sent: a switch going down produces one notification, not
thirty. On the Monitors page, children stack under their parent and dim when it
is unreachable.
