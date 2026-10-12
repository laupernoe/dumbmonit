# Konfiguration

Alles läuft über Umgebungsvariablen; keine ist erforderlich. Es gibt
keine Konfigurationsdatei zum Einhängen.

## Server

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Horchadresse der API und der Web-UI, innerhalb des Containers. |
| `DUMBMONIT_DATA_DIR` | `/data` | Persistentes Verzeichnis: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (eingebettetes VictoriaMetrics) und `webchange/` (Screenshots von Website-Änderungen). |
| `DUMBMONIT_VM_URL` | *(nicht gesetzt)* | Basis-URL eines externen VictoriaMetrics (`http://host:8428`). Wenn gesetzt, wird das eingebettete nicht gestartet; siehe [unten](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(generiert)* | Instanz-Secret. Wenn leer, aus `<data dir>/secret.key` gelesen, beim ersten Start generiert. Verschlüsselt Gerätezugangsdaten, Kanal-Secrets und Agent-Tokens mit AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Maximale gleichzeitige Abfragen, alle Collectors zusammen. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Hartes Timeout einer Abfrage. Service-Monitore haben ihre eigene, kürzere Option `timeout_seconds`. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Periode der gebündelten Schreibvorgänge zu VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Anzahl gepufferter Samples, die ein Schreiben auslöst, bevor die Periode abläuft. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Threads der asynchronen Runtime. Der Server verbringt seine Zeit mit Warten auf das Netzwerk: vier decken ein Homelab ab, heben Sie es nur für Hunderte von Geräten an. |
| `DUMBMONIT_DB_POOL` | `4` | Maximal offene SQLite-Verbindungen. Jede kostet einen Thread und einen 2-MB-Page-Cache; untätige Verbindungen schließen nach fünf Minuten. |
| `DUMBMONIT_LOG` | `info` | Log-Filter, `tracing`-Syntax: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Verzeichnis der unter `/download/` bedienten Agent-Binaries. Leer oder fehlend: Der Installationsbefehl schlägt mit `404` fehl, der Server läuft trotzdem. |
| `DUMBMONIT_RESET_PASSWORD` | *(aus)* | `1`, `true`, `yes` oder `on`: löscht beim Start das Passwort und jede Sitzung. Die UI zeigt dann wieder `/setup`. Danach entfernen. |
| `DUMBMONIT_COOKIE_SECURE` | *(aus)* | `1`, um das `Secure`-Attribut auf dem Session-Cookie zu setzen. Nur hinter HTTPS: Über einfaches HTTP würde der Browser das Cookie nie zurücksenden. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(leer)* | Reverse Proxys, deren `X-Forwarded-For` geglaubt wird, als Adressen oder CIDR-Bereiche, durch Kommas getrennt (`10.0.0.5, 172.16.0.0/12`). Der Login-Rate-Limiter und das Sicherheitslog sehen dann die echte Client-Adresse statt die des Proxys; ohne das teilt sich jeder Besucher hinter dem Proxy einen Bucket. Der Header wird von jeder anderen Adresse ignoriert, sodass ein Client sich nicht seinen eigenen Bucket aussuchen kann. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(aus)* | `true`, um bei ihrem ersten Single Sign-On ein Konto für eine unbekannte Identität zu erstellen. Aus: Nur bestehende Konten (schon verknüpft, oder über eine vom Provider verifizierte E-Mail abgeglichen) können sich anmelden. Nur gelesen, solange keine Single-Sign-On-Einstellungen in der Oberfläche gespeichert sind. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(aus)* | `1`, um einen OpenID-Connect-Issuer in einfachem `http://` zu akzeptieren. Aus: Der Einstellungsbildschirm lehnt alles außer `https://` ab: Ein Klartext-Issuer gibt den Autorisierungscode und das Client-Secret an jeden weiter, der im Netzwerk mithört. Nur für einen Test-Provider auf dem Loopback. |
| `DUMBMONIT_METRICS_PUBLIC` | *(aus)* | `1`, um `/metrics`, `/federate` und `/prometheus/api/v1/…` ohne API-Token zu bedienen. Wer den Port erreicht, liest dann jede Messung jedes Geräts und den Zustand der Instanz; setzen Sie es nur, wo etwas anderes den Zugriff auf den Port schon einschränkt. Siehe [Metriken](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Alarm-Auswertungsperiode. Werte unter 10 werden auf 10 angehoben. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Aufbewahrung der Alarmhistorie, in Tagen. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Geplante lokale Backups der Datenbank. `0`, `false`, `no` oder `off`, um sie nicht mehr zu schreiben. Siehe [Backup und Wiederherstellung](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` stoppt den täglichen Blick auf das neueste GitHub-Release (Air-Gapped-Instanzen). Siehe [Update](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Wo die geplanten Backups geschrieben werden. Richten Sie es auf ein zweites Volume, um den Verlust des ersten zu überstehen. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Stunden zwischen zwei Backups, 1 bis 8760. Das erste passiert eine Periode nach dem Start, nicht beim Start. |
| `DUMBMONIT_DEMO` | *(aus)* | `1` für eine öffentliche, nur-lesende Demo mit einem festen fiktiven Bestand, Login `demo` / `demo`. **Erstellt die Datenbank bei jedem Start neu**: niemals auf einem echten Datenvolume. Siehe [Eine Demo-Instanz betreiben](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Wie viele Backups behalten werden, 1 bis 365. Die ältesten werden mit ihrem `.key` entfernt. |
| `DUMBMONIT_BROWSER_URL` | *(nicht gesetzt)* | Headless-Chromium für die Screenshots von [Website-Änderungen](../devices/webchange.md#screenshots), als `http://host:9222` (sein DevTools-Port). Die Compose-Datei liefert eines unter dem Profil `screenshots`: `DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d`. Nicht gesetzt: nur Textvergleich. |

Die Baseline-Aufbewahrung (60 Tage) und die 14-tägige Lernphase sind
nicht konfigurierbar.

### Frühere `EZYMONIT_*`-Namen

DumbMonit hieß bis September 2026 EzyMonit. Jede `DUMBMONIT_*`-Variable
wird auch unter ihrem alten `EZYMONIT_*`-Namen gelesen, als Fallback,
mit einer Warnung beim Start pro so noch gesetzter Variable
(`EZYMONIT_X is deprecated, use DUMBMONIT_X`). Die alte Datenbankdatei
`/data/ezymonit.db` wird beim Start zu `dumbmonit.db` umbenannt, und
Agent-Tokens `ezym_…` bleiben gültig (neue sind `dmon_…`). Das
Session-Cookie heißt jetzt `dumbmonit_session`: Der erste Start nach
der Umbenennung loggt einmalig jeden aus.

### Eingebettetes VictoriaMetrics {#embedded-victoriametrics}

Das Image liefert das VictoriaMetrics-Binary. Sofern
`DUMBMONIT_VM_URL` nicht gesetzt ist, startet der Server es beim Start
als Kindprozess, leitet seine Log-Zeilen in das Server-Log weiter,
startet es mit Backoff neu, falls es stirbt, und stoppt es beim
Herunterfahren (`SIGTERM`, 20 s Gnadenfrist, dann Kill; der
`stop_grace_period` der Compose-Datei lässt dafür Raum).

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Pfad des zu startenden VictoriaMetrics-Binaries. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Horchadresse, innerhalb des Containers. Standardmäßig Loopback: nur der Server spricht damit. Das Entwickler-Overlay setzt `0.0.0.0:8428` und veröffentlicht den Port. |
| `DUMBMONIT_VM_RETENTION` | `12` | Aufbewahrung, in der Syntax von `-retentionPeriod`: eine Anzahl Monate, oder eine Dauer wie `30d` oder `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget für VictoriaMetrics' Caches (`-memory.allowedBytes`). Ohne Budget bemisst VM sie für 60 % des RAM des Hosts, und sein residenter Speicher wächst über Tage. Ein paar Dutzend Geräte passen in 256 MB; heben Sie es für Hunderte von Hosts an. |

Seine Daten leben in `<data dir>/vm`, auf demselben Volume wie die
Datenbank. Drei Flags sind vom Server fest gesetzt und nicht
konfigurierbar: `-search.maxUniqueTimeseries=100000` (eine
außer-Kontrolle-geratene Abfrage kann das Budget nicht erschöpfen),
`-search.maxConcurrentRequests=4` (eine UI plus die Alarm-Engine) und
`-dedup.minScrapeInterval=10s` (doppelte Punkte näher als zehn
Sekunden werden zusammengefasst; bei der 30-s-Kadenz der Collectors
geht nichts verloren).

Mit gesetztem `DUMBMONIT_VM_URL` gilt nichts davon: Der Server nutzt
die von Ihnen benannte Instanz, und das eingebettete wird nie
gestartet. `GET /api/health` sagt, in welchem Fall Sie sind
(`victoria.embedded`).

## Compose-Ebene-Variablen

Diese werden von `docker-compose.yml` gelesen, nicht vom Server:

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_PORT` | `8080` | Für die UI veröffentlichter Host-Port. |
| `DUMBMONIT_VM_RETENTION`, `DUMBMONIT_VM_MEMORY` | `12`, `256MB` | An den Server weitergegeben; siehe oben. |
| `DUMBMONIT_VM_PORT` | `8428` | Host-Port für das eingebettete VictoriaMetrics, nur Entwickler-Overlay. |

## Ports

| Port | Container | Zweck |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Web-UI, API, Agent-Installer und Downloads, Agent-Ingest. Der einzige zu veröffentlichende Port. |
| `8428/tcp` | `dumbmonit` | Eingebettete VictoriaMetrics-HTTP-API, auf dem Loopback des Containers. Nur vom Entwickler-Overlay veröffentlicht. |

Ausgehend: SNMP (standardmäßig UDP 161) zu Geräten, HTTPS zu Proxmox,
PBS, Synology und Benachrichtigungsdiensten, plus was auch immer Ihre
Service-Monitore anvisieren. ICMP-Ping braucht das sysctl
`net.ipv4.ping_group_range`, in der Compose-Datei gesetzt (keine
Capability).

## Volumes und Dateien

Ein Volume, `dumbmonit-data`, auf `/data` eingehängt. Sein Name ist in
der Compose-Datei fest, wie auch immer das Projekt heißt.

| Pfad (im Container) | Volume | Inhalt |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Geräte, Zugangsdaten (verschlüsselt), Regeln, Alarmzustand und -historie, Baselines, Silences, Kanäle, Agent-Tokens, Passwort-Hash, Sitzungen. |
| `/data/secret.key` | `dumbmonit-data` | Das Instanz-Secret. **Sichern Sie es.** Ohne es sind verschlüsselte Zugangsdaten nicht wiederherstellbar, und der Server verweigert den Start. |
| `/data/backups/` | `dumbmonit-data` | Geplante lokale Backups: `dumbmonit-<timestamp>.db` (eine Online-SQLite-Kopie) und das passende `.key`. Standardmäßig täglich, sieben behalten. |
| `/data/vm/` | `dumbmonit-data` | Eingebettete VictoriaMetrics-Zeitreihen, standardmäßig 12 Monate. Leer, wenn `DUMBMONIT_VM_URL` gesetzt ist. |
| `/agents/` | Image | Agent-Binaries. Hängen Sie ein anderes Verzeichnis ein und setzen Sie `DUMBMONIT_AGENT_DIR`, um Ihre eigenen zu liefern. |
| `/etc/ssl/certs/ca-certificates.crt` | Image | TLS-Roots für ausgehendes HTTPS. Hängen Sie hier Ihr eigenes Bundle ein, um einer privaten Autorität zu vertrauen. |

## Das Passwort zurücksetzen

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
# open the UI: /setup asks for a new password
docker compose up -d dumbmonit   # start again without the variable
```

Das Zurücksetzen löscht das Passwort und jede Sitzung; Geräte, Regeln
und Kanäle bleiben unberührt.

## Agent

Der Agent hat seine eigenen Variablen und Konfigurationsdatei: siehe
[Linux-, macOS-, FreeBSD- und Windows-Agent](../devices/agent.md#configuration-file).

### Erfassungsumfang

Was der Agent misst, und daher wie viele Zeitreihen eine Maschine
erzeugt. Die Standardwerte halten einen Host mit Dutzenden Containern
bei ein paar hundert Zeitreihen; jeder Schlüssel ist ein
`agent.yaml`-Eintrag, über seine Umgebungsvariable überschreibbar.

| Schlüssel | Variable | Standard | Rolle |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Ausgelassene Netzwerkschnittstellen: Container- und VM-Schnittstellen, das Loopback. Eine Liste exakter Namen oder regulärer Ausdrücke (nicht verankert; alles mit einem Metazeichen ist ein Regex). Der Wert ersetzt den Standard; eine leere Liste behält jede Schnittstelle. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(leer)* | Erlaubnisliste, gleiche Syntax. Wenn gesetzt, werden nur diese Schnittstellen gemeldet und `interfaces_ignore` wird nicht konsultiert. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Aus Dateisystemnutzung und Disk-I/O ausgelassene Einhängepunkte, gleiche Syntax. Wechselmedien unter `/run/media/` bleiben. Pseudo-Dateisysteme (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) werden immer übersprungen, egal welcher Pfad. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Eine `cpu_core_usage_percent`-Zeitreihe pro Kern, zusätzlich zur globalen Nutzung. `cpu_count` wird in jedem Fall gemeldet. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Pro Zyklus detailliert betrachtete Container, laufende zuerst. Darüber hinaus werden Container nur gezählt (`container_count`, `container_running_count` bleiben exakt), und `container_series_skipped` sagt, wie viele. `0` zählt ohne Details. |

In der Umgebung sind Listen kommagetrennt (`^veth, ^br-, docker0`); in
YAML entweder eine Liste oder ein kommagetrennter String. Disk-I/O-
Zähler (`disk_read_bytes`, `disk_written_bytes`) tragen nur das Label
`device`: eine Zeitreihe pro Block-Gerät, egal wie oft es eingehängt
ist.
