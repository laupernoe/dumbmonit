# Gerätearten

DumbMonit nennt alles, was es beobachtet, ein *Gerät* (die API sagt
*Target*), selbst wenn es eine Webseite oder ein DNS-Name ist. Jede Art
ist ein Collector auf dem Server; der Server beschreibt jede Art der
Oberfläche (`GET /api/collectors`), sodass das Formular immer dem
entspricht, was diese Version überwachen kann.

| Art | `kind` | Credential | Typische Nutzung |
|---|---|---|---|
| [SNMP-Gerät](snmp.md) | `snmp` | Community oder SNMP v3 | Switch, Router, NAS, USV, Drucker |
| [Proxmox VE](proxmox.md) | `proxmox` | API-Token oder Benutzername/Passwort | Hypervisor, Cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | API-Token oder Benutzername/Passwort | Backup-Server |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | API-Token | Konsole, die mehrere PVE-Cluster und PBS-Instanzen föderiert |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | Benutzername/Passwort | Mail-Gateway, das Spam und Viren filtert |
| [VMware vSphere](vsphere.md) | `vsphere` | Benutzername/Passwort | vCenter oder eigenständiges ESXi: Hosts, VMs, Datastores, Alarme |
| [Synology DSM](synology.md) | `synology` | Benutzername/Passwort | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | API-Key und -Secret | Firewall, Router, Multi-WAN-Edge |
| [pfSense](pfsense.md) | `pfsense` | REST-API-Key (nur-lesender Benutzer) | Firewall: Gateways down oder verlustreich, Schnittstellen ohne Link, gestoppte Dienste |
| [FortiGate](fortigate.md) | `fortigate` | REST-API-Token (nur-lesendes Profil, vertrauter Host) | Firewall: IPsec-Tunnel, Links, HA-Sync, Conserve-Modus, FortiGuard-Lizenzen |
| [Sophos Firewall](sophos.md) | `sophos` | Benutzername/Passwort (nur-lesendes Profil) | Firewall: nur Schnittstellen-Links (die XML-API zeigt keinen Tunnel-, HA- oder Lizenzzustand) |
| [UniFi Network](unifi.md) | `unifi` | API-Key oder Benutzername/Passwort | UniFi-Konsole oder selbst gehosteter Server: Geräte, WAN, Clients |
| [Home Assistant](homeassistant.md) | `homeassistant` | langlebiges Access-Token | Smart Home: nicht verfügbare Entitäten, schwache Akkus, Updates, Repairs |
| [TrueNAS](truenas.md) | `truenas` | API-Key | ZFS-Storage-Server (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | API-Key (Viewer-Rolle) | NAS: Array, Platten, Paritätsprüfungen, Cache-Pools, Container, VMs |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | Benutzername/Passwort (nur-lesende Gruppe) | Router oder Switch: Versionen und Firmware-Rückstand, CPU, Speicher, Sensoren, Schnittstellenfehler |
| [USV mit NUT](nut.md) | `nut` | keins, oder Benutzername/Passwort | USV an einem NAS, Raspberry Pi oder Server mit Network UPS Tools |
| [Server-Hardware (Redfish)](redfish.md) | `redfish` | Benutzername/Passwort | Server-Lüfter, -Temperaturen, -Netzteile und -Laufwerke, von seinem BMC gelesen |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | keins, Benutzername/Passwort oder Token | Zeitreihendatenbank: Ingestion, abgelehnte Samples, Platten-Spielraum |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | keins, Benutzername/Passwort oder Token | Log-Datenbank: Ingestion, abgelehnte Zeilen, Platten-Spielraum |
| [Grafana Loki](loki.md) | `loki` | keins, Benutzername/Passwort oder Token | Log-Server: Bereitschaft, abgelehnte Zeilen, Flush-Fehler |
| [Graylog](graylog.md) | `graylog` | Access-Token oder Benutzername/Passwort | Log-Server und sein OpenSearch-Cluster: Journal, Puffer, Durchsatz, Inputs |
| [Pi-hole](pihole.md) | `pihole` | App-Passwort, oder keins | DNS-Werbeblocker: Blockierzustand, blockierter Anteil, Blocklisten-Alter, Updates |
| [AdGuard Home](adguard.md) | `adguard` | Benutzername/Passwort | DNS-Filter: Schutz, Blockierungen, Filterlisten, Upstream-Server, Updates |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | keins, oder E-Mail-Adresse/Passwort | Mail-Server: SMTP, IMAP, POP3, Webmail, Version |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | keins, oder API-Key | Mail-Gateway: Dienste, Zustellungswarteschlange und andere Zähler |
| [Redis / Valkey](redis.md) | `redis` | ACL-Benutzer, nur Passwort, oder keins | In-Memory-Store: Speicher gegen maxmemory, Replikation, Persistenz, Keys |
| [MongoDB](mongodb.md) | `mongodb` | Benutzername/Passwort (clusterMonitor) oder keins | Dokumentendatenbank: Replica Set, Lag, Verbindungen, WiredTiger-Cache |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | Benutzername/Passwort (Tag monitoring) | Message Broker: Ressourcenalarme, Knoten, Partitionen, Warteschlangen ohne Consumer |
| [CrowdSec](crowdsec.md) | `crowdsec` | keins, oder Bouncer-API-Key | Security-Engine: Entscheidungen, Alarme, Bouncer, die aufgehört haben abzufragen, Log-Lesen |
| [Traefik](traefik.md) | `traefik` | keins, oder basicAuth-Benutzer | Reverse Proxy: deaktivierte Router, ausgefallene Backend-Server, nicht erhaltene oder erneuerte Zertifikate, 5xx-Anteil |
| [Caddy](caddy.md) | `caddy` | nur-lesender Ansichtsbenutzer, oder keins | Webserver und Reverse Proxy: Upstream-Gesundheit, fehlgeschlagene Reloads, Handler-Fehler, 5xx-Anteil |
| [Nginx](nginx.md) | `nginx` | keins, oder Basic Auth | Webserver: aktive/wartende Verbindungen, Accept-/Handled-/Request-Raten, NGINX-Plus-Upstream-Gesundheit |
| [Apache httpd](apache.md) | `apache` | keins, oder Basic Auth | Webserver: beschäftigte/untätige Worker, Scoreboard nach Zustand, Request- und Byte-Raten |
| [Nginx Proxy Manager](npm.md) | `npm` | Benutzer-E-Mail und Passwort (nur Ansicht) | Proxy-Manager: von nginx abgelehnte Hosts, deaktivierte Hosts, ausgelieferte Zertifikate |
| [Domain-Ablauf](domain.md) | `domain` | keins | Registrierte Domain über RDAP: Tage bis zum Ablauf, Sperre, Erlösungsfrist, Registrar |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | Service-Account-Token (nur-lesende ClusterRole) | Cluster: Knoten nicht ready, Crash-loopende oder pending Pods, Workloads mit fehlenden Replikas, feststeckende Volume Claims |
| [Active Directory](activedirectory.md) | `activedirectory` | Benutzername/Passwort (einfacher Domänenbenutzer) | Windows-Domäne über LDAPS: Domänencontroller und FSMO-Rollen, Mitglieder privilegierter Gruppen, Passwortrichtlinie, Sicherheitsbefunde |
| [Veeam Backup & Replication](veeam.md) | `veeam` | Benutzername/Passwort (Veeam Backup Viewer) | Backup-Server: fehlgeschlagene Jobs, fehlgeschlagene Sitzungen, sich füllende Repositories, Lizenz |
| [Tailscale](tailscale.md) | `tailscale` | OAuth-Client (devices:core:read) oder API-Access-Token | Tailnet: Server offline, ablaufende Node-Keys, Geräte, die auf Genehmigung warten |
| [Nextcloud](nextcloud.md) | `nextcloud` | Monitoring-Token (NC-Token) | Datei-Sync: Wartungsmodus, ausstehendes Upgrade, Updates, aktive Benutzer, OPcache |
| [Immich](immich.md) | `immich` | API-Key | Fotobibliothek: Job-Warteschlangen, Updates, Platte, Fotoanzahl |
| [Paperless-ngx](paperless.md) | `paperless` | API-Token oder Benutzername/Passwort | Dokumente: Redis, Celery, Index, fehlgeschlagene Importe, Updates |
| [Jellyfin](jellyfin.md) | `jellyfin` | API-Key | Medienserver: fehlgeschlagene geplante Tasks, Plugins, Streams und Transkodierungen |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, oder keins aus einem erlaubten Netzwerk | Medienserver: Streams, Transkodierungen, Bibliotheken, Updates |
| [GitLab (self-managed)](gitlab.md) | `gitlab` | Personal Access Token | Forge: Sidekiq-Rückstand, CI-Runner, ausstehende Migrationen, Sign-up/2FA-Einstellungen, Lizenz |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | Access-Token | Forge: Healthcheck, geplante Tasks, Actions-Runner, Repository-/Kontenzahlen |
| [Server mit Agent](agent.md) | `agent` | keins (Enrollment-Token) | Linux, Windows, Raspberry Pi, [Hyper-V-Host](agent.md#hyper-v-hosts) |
| [Website oder Web-API](services.md#http) | `http` | keins, Benutzername/Passwort oder Token | Health-Seite, REST-API |
| [Netzwerkport](services.md#tcp) | `tcp` | keins | SSH, SMB, Datenbank |
| [Domainname](services.md#dns) | `dns` | keins | Ihre Domain, ein interner Name |
| [Erreichbarer Host](services.md#ping) | `ping` | keins | Gateway, Access Point, Drucker |
| [TLS-Zertifikat](services.md#tls) | `tls` | keins | IMAPS, LDAPS, Reverse Proxy |
| [NTP-Zeitserver](services.md#ntp) | `ntp` | keins | Router, NAS, interner Zeitserver, öffentlicher Pool-Server |
| [Heartbeat](push.md) | `push` | keins (geheime URL) | Cron-Job, Backup-Skript, Automatisierung, die sich melden muss |
| [Website-Änderungen](webchange.md) | `webchange` | keins | Eine Seite oder eine ganze Site: Textänderungen, neue und entfernte Seiten, Vorher/Nachher mit Screenshots |
| [Demo-Gerät](demo.md) | `dummy` | keins | Die UI ohne Hardware erkunden |

## Wie ein Gerät gelesen wird

Der Scheduler führt jedes aktivierte Gerät in seinem eigenen Intervall
aus (standardmäßig 60 s, mindestens 10 s) über den Collector seiner Art,
mit höchstens `DUMBMONIT_MAX_CONCURRENT_PROBES` laufenden Abfragen und
einem harten Timeout von `DUMBMONIT_PROBE_TIMEOUT_SECS` pro Abfrage.
Jede erfolgreiche Abfrage schreibt ihre Samples plus `dumbmonit_up = 1`;
eine fehlgeschlagene Abfrage schreibt nichts, und es ist das Schweigen
der Zeitreihe, das die Regel "Device unreachable" erkennt.

Fehler werden klassifiziert. Ein nicht antwortendes Gerät ist
*unreachable* und löst einen Alarm aus. Eine falsche Community, ein
abgelehntes Zertifikat oder eine fehlende Fähigkeit ist ein
*Konfigurationsfehler*: Er wird auf der Geräteseite rot mit dem Grund
angezeigt und nicht benachrichtigt. Die Unterscheidung ist wichtig: Ein
falsch konfiguriertes Gerät sollte nachts niemanden wecken.

## Optionen

Manche Arten lesen Optionen aus den Tags des Geräts: den API-Port, ob
ein selbstsigniertes Zertifikat akzeptiert wird, welche Knoten
überwacht werden. Das Formular zeigt ein typisiertes Feld pro Option
unter **More options**, mit seinem Standard und Hilfetext. Die
Seiten je Art listen sie wortgetreu. Optionen werden auf jeder
Zeitreihe als `tag_<key>`-Labels kopiert, also legen Sie nie ein Secret
in eine: Nutzen Sie das Credential-Feld, das verschlüsselt ist.

## Übergeordnete Geräte

Jedes Gerät kann ein anderes als sein übergeordnetes Gerät angeben.
Wenn das übergeordnete Gerät nicht erreichbar ist, werden Alarme seiner
Nachkommen als *Durch übergeordnetes Gerät unterdrückt* markiert, statt
gesendet zu werden: ein ausfallender Switch erzeugt eine
Benachrichtigung, nicht dreißig. Auf der Geräteseite stapeln sich
Kinder unter ihrem Elternteil und werden abgedunkelt, wenn dieser nicht
erreichbar ist.
