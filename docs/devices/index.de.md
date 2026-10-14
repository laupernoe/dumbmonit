# Gerätetypen {#device-types}

DumbMonit nennt alles, was es überwacht, ein *Gerät* (die API sagt *target*), auch wenn es eine
Webseite oder ein DNS-Name ist. Jeder Typ ist ein Collector auf dem Server; der Server beschreibt
der Oberfläche jeden Typ (`GET /api/collectors`), sodass das Formular immer dem entspricht, was
diese Version überwachen kann.

| Typ | `kind` | Zugangsdaten | Typische Verwendung |
|---|---|---|---|
| [SNMP-Gerät](snmp.md) | `snmp` | Community oder SNMP v3 | Switch, Router, NAS, USV, Drucker |
| [Proxmox VE](proxmox.md) | `proxmox` | API-Token oder Benutzername/Passwort | Hypervisor, Cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | API-Token oder Benutzername/Passwort | Backup-Server |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | API-Token | Konsole, die mehrere PVE-Cluster und PBS-Instanzen bündelt |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | Benutzername/Passwort | Mail-Gateway, das Spam und Viren filtert |
| [VMware vSphere](vsphere.md) | `vsphere` | Benutzername/Passwort | vCenter oder eigenständiger ESXi: Hosts, VMs, Datastores, Alarme |
| [XCP-ng / XenServer](xcpng.md) | `xcpng` | Benutzername/Passwort (schreibgeschützte XAPI-Rolle) | Pool über XAPI: Hosts, VMs, Storage Repositories, Hochverfügbarkeit |
| [Nutanix AHV](nutanix.md) | `nutanix` | Benutzername/Passwort (Prism Viewer) | Prism-Central-v4-API: Cluster, Hosts, AHV-VMs, Storage-Container, Alarme |
| [Synology DSM](synology.md) | `synology` | Benutzername/Passwort | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | API-Schlüssel und Secret | Firewall, Router, Multi-WAN-Edge |
| [pfSense](pfsense.md) | `pfsense` | REST-API-Schlüssel (Nur-Lese-Benutzer) | Firewall: ausgefallene oder verlustbehaftete Gateways, Schnittstellen ohne Link, gestoppte Dienste |
| [FortiGate](fortigate.md) | `fortigate` | REST-API-Token (Nur-Lese-Profil, Trusted Host) | Firewall: IPsec-Tunnel, Links, HA-Sync, Conserve Mode, FortiGuard-Lizenzen |
| [Sophos Firewall](sophos.md) | `sophos` | Benutzername/Passwort (Nur-Lese-Profil) | Firewall: nur Schnittstellen-Links (die XML-API zeigt keinen Tunnel-, HA- oder Lizenzstatus) |
| [UniFi Network](unifi.md) | `unifi` | API-Schlüssel oder Benutzername/Passwort | UniFi-Konsole oder selbst gehosteter Server: Geräte, WAN, Clients |
| [Home Assistant](homeassistant.md) | `homeassistant` | Langlebiges Zugriffstoken | Smart Home: nicht verfügbare Entitäten, schwache Batterien, Updates, Reparaturen |
| [TrueNAS](truenas.md) | `truenas` | API-Schlüssel | ZFS-Speicherserver (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | API-Schlüssel (Rolle Viewer) | NAS: Array, Platten, Paritätsprüfungen, Cache-Pools, Container, VMs |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | Benutzername/Passwort (Nur-Lese-Gruppe) | Router oder Switch: veraltete Versionen und Firmware, CPU, Speicher, Sensoren, Schnittstellenfehler |
| [USV mit NUT](nut.md) | `nut` | keine, oder Benutzername/Passwort | USV an einem NAS, Raspberry Pi oder Server mit Network UPS Tools |
| [Serverhardware (Redfish)](redfish.md) | `redfish` | Benutzername/Passwort | Lüfter, Temperaturen, Netzteile und Laufwerke eines Servers, über dessen BMC gelesen |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | keine, Benutzername/Passwort oder Token | Zeitreihendatenbank: Ingestion, abgelehnte Messwerte, Plattenspielraum |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | keine, Benutzername/Passwort oder Token | Logdatenbank: Ingestion, abgelehnte Zeilen, Plattenspielraum |
| [Grafana Loki](loki.md) | `loki` | keine, Benutzername/Passwort oder Token | Logserver: Bereitschaft, abgelehnte Zeilen, Flush-Fehler |
| [Graylog](graylog.md) | `graylog` | Zugriffstoken oder Benutzername/Passwort | Logserver und sein OpenSearch-Cluster: Journal, Puffer, Durchsatz, Inputs |
| [Pi-hole](pihole.md) | `pihole` | App-Passwort, oder keines | DNS-Werbeblocker: Blockierungsstatus, blockierter Anteil, Alter der Blocklisten, Updates |
| [AdGuard Home](adguard.md) | `adguard` | Benutzername/Passwort | DNS-Filter: Schutz, Blockierungen, Filterlisten, Upstream-Server, Updates |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | keine, oder E-Mail-Adresse/Passwort | Mailserver: SMTP, IMAP, POP3, Webmail, Version |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | keine, oder API-Schlüssel | Mail-Gateway: Dienste, Zustellwarteschlange und weitere Zähler |
| [Redis / Valkey](redis.md) | `redis` | ACL-Benutzer, nur Passwort, oder keine | In-Memory-Speicher: Speicher gegenüber maxmemory, Replikation, Persistenz, Schlüssel |
| [MongoDB](mongodb.md) | `mongodb` | Benutzername/Passwort (clusterMonitor) oder keine | Dokumentendatenbank: Replica Set, Verzug, Verbindungen, WiredTiger-Cache |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | Benutzername/Passwort (Tag monitoring) | Message Broker: Ressourcenalarme, Knoten, Partitionen, Queues ohne Consumer |
| [CrowdSec](crowdsec.md) | `crowdsec` | keine, oder Bouncer-API-Schlüssel | Sicherheits-Engine: Entscheidungen, Alarme, Bouncer, die nicht mehr abrufen, Log-Lesen |
| [Traefik](traefik.md) | `traefik` | keine, oder basicAuth-Benutzer | Reverse-Proxy: deaktivierte Router, ausgefallene Backend-Server, nicht erhaltene oder nicht erneuerte Zertifikate, 5xx-Anteil |
| [Caddy](caddy.md) | `caddy` | Nur-Lese-Ansichtsbenutzer, oder keiner | Webserver und Reverse-Proxy: Upstream-Zustand, fehlgeschlagene Reloads, Handler-Fehler, 5xx-Anteil |
| [Nginx](nginx.md) | `nginx` | keine, oder Basic Auth | Webserver: aktive/wartende Verbindungen, Accept-/Handled-/Anfrageraten, Upstream-Zustand von NGINX Plus |
| [Apache httpd](apache.md) | `apache` | keine, oder Basic Auth | Webserver: beschäftigte/freie Worker, Scoreboard nach Zustand, Anfrage- und Byte-Raten |
| [Nginx Proxy Manager](npm.md) | `npm` | E-Mail und Passwort des Benutzers (nur Ansicht) | Proxy-Manager: von nginx abgelehnte Hosts, deaktivierte Hosts, ausgelieferte Zertifikate |
| [Domain-Ablauf](domain.md) | `domain` | keine | Registrierte Domain über RDAP: Tage bis zum Ablauf, Hold, Redemption-Phase, Registrar |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | Service-Account-Token (Nur-Lese-ClusterRole) | Cluster: nicht bereite Knoten, Pods in Crash-Schleife oder Pending, Workloads mit fehlenden Replikas, festhängende Volume Claims |
| [Active Directory](activedirectory.md) | `activedirectory` | Benutzername/Passwort (einfacher Domänenbenutzer) | Windows-Domäne über LDAPS: Domänencontroller und FSMO-Rollen, Mitglieder privilegierter Gruppen, Kennwortrichtlinie, Sicherheitsbefunde |
| [Veeam Backup & Replication](veeam.md) | `veeam` | Benutzername/Passwort (Veeam Backup Viewer) | Backup-Server: fehlgeschlagene Jobs, fehlgeschlagene Sitzungen, sich füllende Repositories, Lizenz |
| [Tailscale](tailscale.md) | `tailscale` | OAuth-Client (devices:core:read) oder API-Zugriffstoken | Tailnet: Server offline, ablaufende Node-Schlüssel, Geräte, die auf Freigabe warten |
| [Nextcloud](nextcloud.md) | `nextcloud` | Monitoring-Token (NC-Token) | Dateisynchronisation: Wartungsmodus, ausstehendes Upgrade, Updates, aktive Benutzer, OPcache |
| [Immich](immich.md) | `immich` | API-Schlüssel | Fotobibliothek: Job-Warteschlangen, Updates, Platte, Fotozahlen |
| [Paperless-ngx](paperless.md) | `paperless` | API-Token oder Benutzername/Passwort | Dokumente: Redis, Celery, Index, fehlgeschlagene Importe, Updates |
| [Jellyfin](jellyfin.md) | `jellyfin` | API-Schlüssel | Medienserver: fehlgeschlagene geplante Aufgaben, Plugins, Streams und Transcodierungen |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, oder keines aus einem erlaubten Netzwerk | Medienserver: Streams, Transcodierungen, Bibliotheken, Updates |
| [GitLab (selbst gehostet)](gitlab.md) | `gitlab` | Persönliches Zugriffstoken | Forge: Sidekiq-Rückstand, CI-Runner, ausstehende Migrationen, Registrierungs-/2FA-Einstellungen, Lizenz |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | Zugriffstoken | Forge: Health-Check, geplante Aufgaben, Actions-Runner, Repository-/Kontozahlen |
| [Server mit Agent](agent.md) | `agent` | keine (Enrollment-Token) | Linux, Windows, Raspberry Pi, [Hyper-V-Host](agent.md#hyper-v-hosts) |
| [Website oder Web-API](services.md#http) | `http` | keine, Benutzername/Passwort oder Token | Health-Seite, REST-API |
| [Netzwerkport](services.md#tcp) | `tcp` | keine | SSH, SMB, Datenbank |
| [Domainname](services.md#dns) | `dns` | keine | Ihre Domain, ein interner Name |
| [Erreichbarer Host](services.md#ping) | `ping` | keine | Gateway, Access Point, Drucker |
| [TLS-Zertifikat](services.md#tls) | `tls` | keine | IMAPS, LDAPS, Reverse-Proxy |
| [SMTP-Server](services.md#smtp) | `smtp` | keine, oder Benutzername/Passwort | Mailserver: Begrüßung, STARTTLS, Zertifikat, optionale Anmeldung |
| [PostgreSQL](services.md#postgresql) | `postgres` | Benutzername/Passwort | Datenbank: Verbindung und eine von Ihnen gewählte Abfrage |
| [MySQL oder MariaDB](services.md#mysql-and-mariadb) | `mysql` | Benutzername/Passwort | Datenbank: Verbindung und eine von Ihnen gewählte Abfrage |
| [MQTT-Broker](services.md#mqtt) | `mqtt` | keine, oder Benutzername/Passwort | Broker: Verbindung, Abonnement, Retained Message |
| [WebSocket](services.md#websocket) | `websocket` | keine, Benutzername/Passwort oder Token | WebSocket-Endpunkt: Handshake, optionaler Nachrichtenaustausch |
| [NTP-Zeitserver](services.md#ntp) | `ntp` | keine | Router, NAS, interner Zeitserver, öffentlicher Pool-Server |
| [Heartbeat](push.md) | `push` | keine (geheime URL) | Cronjob, Backup-Skript, Automatisierung, die sich melden muss |
| [Website-Änderungen](webchange.md) | `webchange` | keine | Eine Seite oder eine ganze Website: Textänderungen, neue und entfernte Seiten, Vorher/Nachher mit Screenshots |
| [Website-Zustand & SEO](website.md) | `website` | keine | Websites in großer Zahl: Ausfall, leere Seite, Weiterleitungen, noindex, robots.txt, Änderungen von Titel/Beschreibung/Canonical, Plattform (CSV-Import) |
| [Demo-Gerät](demo.md) | `dummy` | keine | Die Oberfläche ohne Hardware erkunden |

## Wie ein Gerät gelesen wird {#how-a-device-is-read}

Der Scheduler führt jedes aktivierte Gerät in seinem eigenen Intervall (standardmäßig 60 s,
mindestens 10 s) über den Collector seiner Art aus, mit höchstens
`DUMBMONIT_MAX_CONCURRENT_PROBES` gleichzeitigen Prüfungen und einem harten Timeout von
`DUMBMONIT_PROBE_TIMEOUT_SECS` pro Prüfung. Jede erfolgreiche Prüfung schreibt ihre Messwerte
plus `dumbmonit_up = 1`; eine fehlgeschlagene Prüfung schreibt nichts, und es ist das Schweigen
der Serie, das die Regel „Gerät nicht erreichbar“ erkennt. Die Geräteseite zeigt den Fehler bei
der nächsten Prüfung, aber der Alarm wartet absichtlich: Er löst etwa vier Minuten nach der
letzten erfolgreichen Prüfung aus (drei Minuten Schweigen, dann eine Minute Haltezeit), sodass
eine einzelne verlorene Prüfung nie jemanden alarmiert.

Fehler werden klassifiziert. Ein Gerät, das nicht antwortet, ist *nicht erreichbar* und löst einen
Alarm aus. Eine falsche Community, ein abgelehntes Zertifikat oder eine fehlende Fähigkeit ist ein
*Konfigurationsfehler*: Er wird auf der Geräteseite in Rot mit dem Grund angezeigt und nicht
benachrichtigt. Der Unterschied ist wichtig: Ein falsch konfiguriertes Gerät sollte nachts
niemanden wecken.

## Optionen {#options}

Manche Typen lesen Optionen aus den Tags des Geräts: den API-Port, ob ein selbstsigniertes
Zertifikat akzeptiert wird, welche Knoten überwacht werden. Das Formular zeigt unter **Weitere
Optionen** pro Option ein typisiertes Feld mit Standardwert und Hilfetext. Die Seiten der
einzelnen Arten listen sie wörtlich auf. Optionen sind Einstellungen, keine Labels: Sie werden
nicht auf die Serien kopiert. Freie Tags (jeder andere Schlüssel) werden auf jede Serie als
Labels `tag_<key>` kopiert, außer Schlüsseln, die ein Geheimnis benennen (pro Wort geprüft:
`password`, `token`, `key`, `cert`, `secret`, `pass`, `private`, `pem` …, sodass `primary_key`
übersprungen wird, `keyboard` aber nicht) und Werten, die länger als 256 Zeichen sind. Legen Sie
trotzdem nie ein Geheimnis in einem Tag ab: Verwenden Sie das Zugangsdatenfeld, das verschlüsselt
ist.

## Übergeordnete Geräte {#parent-devices}

Jedes Gerät kann ein anderes als sein Elternteil angeben. Ist das Elternteil nicht erreichbar,
werden Alarme seiner Nachkommen als *Vom Elternteil unterdrückt* markiert, statt gesendet zu
werden: Ein ausgefallener Switch erzeugt eine Benachrichtigung, nicht dreißig. Auf der Seite
Geräte stapeln sich Kinder unter ihrem Elternteil und werden gedimmt, wenn es nicht erreichbar
ist.
