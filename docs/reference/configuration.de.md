# Konfiguration {#configuration}

Alles läuft über Umgebungsvariablen; keine ist erforderlich. Es gibt keine Konfigurationsdatei
zum Einbinden.

## Server {#server}

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Lauschadresse der API und der Web-Oberfläche, innerhalb des Containers. |
| `PUID` / `PGID` | `65532` | Benutzer und Gruppe, als die der Server läuft, wenn der Container als root startet (`user: "0:0"`, wie in der mitgelieferten `docker-compose.yml`): Das Datenverzeichnis wird beim Start an sie übergeben, danach wird root abgegeben. `0` wird abgelehnt. `DUMBMONIT_PUID` / `DUMBMONIT_PGID` haben Vorrang. Sonst wird er ignoriert. Siehe [Berechtigungen](../install/docker.md#permissions). |
| `DUMBMONIT_DATA_DIR` | `/data` | Persistentes Verzeichnis: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (eingebettetes VictoriaMetrics) und `webchange/` (Screenshots von Website-Änderungen). |
| `DUMBMONIT_VM_URL` | *(nicht gesetzt)* | Basis-URL eines externen VictoriaMetrics (`http://host:8428`). Ist sie gesetzt, wird das eingebettete nicht gestartet; siehe [unten](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(erzeugt)* | Instanzgeheimnis. Ist es leer, wird es aus `<data dir>/secret.key` gelesen, beim ersten Start erzeugt. Verschlüsselt Monitor-Zugangsdaten, Kanalgeheimnisse und Agent-Tokens mit AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Maximale gleichzeitige Prüfungen, alle Collectors zusammen. |
| `DUMBMONIT_WEBSITE_MAX_CONCURRENT` | `10` | Maximale gleichzeitige [Website-Zustands](../devices/website.md#pacing-hundreds-of-sites)-Prüfungen, 1 bis 256, innerhalb des obigen Limits. |
| `DUMBMONIT_WEBSITE_MAX_PER_HOST` | `1` | Maximale gleichzeitige Website-Prüfungen an eine Domain (`www.example.com` und `shop.example.com` zählen als eine), 1 bis 16. |
| `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` | `2` | Mindestsekunden zwischen zwei Website-Prüfungen an eine Domain, 0 bis 3600. Die Option `host_spacing_seconds` einer Website überschreibt sie nach der Prüfung dieser Website. |
| `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` | `360` | Standardperiode in Minuten des Metadatenvergleichs einer Website (Titel, Beschreibung, Canonical, endgültige Adresse, `robots.txt`), 0 bis 10080; 0 vergleicht bei jeder Prüfung. Verfügbarkeit wird weiterhin bei jedem Abfrageintervall geprüft. Die Option `change_check_minutes` einer Website überschreibt sie. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Hartes Timeout einer Prüfung. Dienstmonitore haben eine eigene, kürzere Option `timeout_seconds`. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Periode der gebündelten Schreibvorgänge Richtung VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Anzahl gepufferter Messwerte, die einen Schreibvorgang auslöst, bevor die Periode abläuft. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Threads der asynchronen Runtime. Der Server verbringt seine Zeit mit Warten auf das Netzwerk: Vier genügen für ein Homelab, erhöhen Sie sie nur für Hunderte von Monitoren. |
| `DUMBMONIT_DB_POOL` | `4` | Maximale offene SQLite-Verbindungen. Jede kostet einen Thread und einen 2-MB-Seitencache; Leerlaufverbindungen schließen nach fünf Minuten. |
| `DUMBMONIT_LOG` | `info` | Logfilter, `tracing`-Syntax: `debug`, `warn`, `dumbmonit=debug,sqlx=warn` … |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Verzeichnis der Agent-Binaries, ausgeliefert unter `/download/`. Leer oder fehlend: Der Installationsbefehl scheitert mit `404`, der Server läuft weiter. |
| `DUMBMONIT_RESET_PASSWORD` | *(aus)* | `1`, `true`, `yes` oder `on`: Löscht beim Start das Passwort und jede Sitzung. Die Oberfläche zeigt dann wieder `/setup`. Entfernen Sie sie danach. |
| `DUMBMONIT_COOKIE_SECURE` | *(aus)* | `1`, um das Attribut `Secure` am Sitzungscookie immer zu setzen. Nur hinter HTTPS: Über einfaches HTTP würde der Browser das Cookie nie zurücksenden. Ohne sie wird das Attribut trotzdem hinzugefügt, sobald der Browser auf einer `https://`-Seite ist (sein `Origin` sagt es, oder ein vertrauenswürdiger Proxy sendet `X-Forwarded-Proto: https`), sodass einfaches HTTP im LAN weiter funktioniert. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(leer)* | Reverse-Proxys, deren `X-Forwarded-For`, `X-Forwarded-Proto` und `X-Forwarded-Host` geglaubt werden, als Adressen oder CIDR-Bereiche, durch Kommas getrennt (`10.0.0.5, 172.16.0.0/12`). Der Login-Ratenbegrenzer und das Sicherheitslog sehen dann die echte Client-Adresse statt der des Proxys; ohne sie teilt sich jeder Besucher hinter dem Proxy einen Bucket. Links von Statusseiten (RSS-Feed, Abonnement-E-Mails) und die Rücksprungadresse des Single Sign-On verwenden den weitergeleiteten Host ebenfalls nur von diesen Proxys. Die Header werden von jeder anderen Adresse ignoriert, sodass ein Client weder seinen eigenen Bucket noch die Adresse eines Links wählen kann. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(aus)* | `true`, um bei der ersten Single-Sign-On-Anmeldung einer unbekannten Identität ein Konto zu erstellen. Aus können sich nur bestehende Konten (bereits verknüpft oder über eine vom Anbieter verifizierte E-Mail zugeordnet) anmelden. Wie die übrigen Variablen `DUMBMONIT_OIDC_*` nur gelesen, solange in der Oberfläche keine Single-Sign-On-Einstellungen gespeichert sind. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(aus)* | `1`, um einen OpenID-Connect-Issuer in einfachem `http://` zu akzeptieren. Aus lehnt der Einstellungsbildschirm alles außer `https://` ab: Ein Klartext-Issuer gibt den Autorisierungscode und das Client-Secret an jeden, der im Netzwerk mithört. Nur für einen Testanbieter auf dem Loopback. |
| `DUMBMONIT_SETUP_CODE` | *(erzeugt)* | Code, den `/setup` verlangt, solange kein Administrator existiert. Nicht gesetzt, wird bei jedem Start ein zufälliger im Serverlog ausgegeben; setzen Sie ihn für ein automatisiertes Deployment. Siehe [Installation mit Docker](../install/docker.md). |
| `DUMBMONIT_PUBLIC_URL` | *(nicht gesetzt)* | Adresse, unter der Browser DumbMonit erreichen (`https://monit.example.org`). Verwendet für Links in Benachrichtigungen und Statusseiten-E-Mails, die Single-Sign-On-Redirect-URI und den an Push-Dienste gegebenen Kontakt (nur HTTPS). Ein in der Oberfläche gespeicherter Wert (Benachrichtigungsrichtlinie, SSO-Einstellungen) hat Vorrang. |
| `DUMBMONIT_API_CORS_ORIGINS` | *(jeder Origin)* | Origins, die die API Cross-Origin mit einem Bearer-Token aufrufen und den MCP-Endpunkt erreichen dürfen: eine kommagetrennte Liste, oder `off` für keine. Siehe [HTTP-API](api.md#cross-origin-requests-cors). |
| `DUMBMONIT_OIDC_ISSUER` | *(nicht gesetzt)* | Issuer-URL des OpenID-Connect-Anbieters; Discovery wird aus `{issuer}/.well-known/openid-configuration` gelesen. Muss `https://` sein, sofern `DUMBMONIT_OIDC_ALLOW_HTTP` nicht gesetzt ist. |
| `DUMBMONIT_OIDC_CLIENT_ID` | *(nicht gesetzt)* | Beim Anbieter registrierte Client-ID. |
| `DUMBMONIT_OIDC_CLIENT_SECRET` | *(nicht gesetzt)* | Client-Secret. |
| `DUMBMONIT_OIDC_PROVIDER_NAME` | `SSO` | Beschriftung der Schaltfläche „Weiter mit …“. |
| `DUMBMONIT_OIDC_SCOPES` | `openid profile email` | Angeforderte Scopes, durch Leerzeichen getrennt. |
| `DUMBMONIT_OIDC_GROUPS_CLAIM` | `groups` | Token-Claim, der die Gruppen des Benutzers auflistet. |
| `DUMBMONIT_OIDC_ADMIN_GROUPS` | *(leer)* | Kommagetrennte Gruppen, deren Mitglieder Administratoren werden. |
| `DUMBMONIT_OIDC_OPERATOR_GROUPS` | *(leer)* | Kommagetrennte Gruppen, deren übrige Mitglieder Operatoren werden. Siehe [Single Sign-On](../using/settings.md#single-sign-on). |
| `DUMBMONIT_METRICS_PUBLIC` | *(aus)* | `1`, um `/metrics`, `/federate` und `/prometheus/api/v1/…` ohne API-Token auszuliefern. Jeder, der den Port erreicht, liest dann jede Messung jedes Monitors und den Zustand der Instanz; setzen Sie es nur dort, wo bereits etwas anderes den Zugriff auf den Port beschränkt. Siehe [Metriken](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Auswertungsperiode der Alarme. Werte unter 10 werden auf 10 angehoben. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Aufbewahrung des Alarmverlaufs in Tagen. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Geplante lokale Sicherungen der Datenbank. `0`, `false`, `no` oder `off`, um sie nicht mehr zu schreiben. Siehe [Sicherung und Wiederherstellung](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` stoppt den täglichen Blick auf das neueste GitHub-Release (Air-Gap-Instanzen). Siehe [Update](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Wohin die geplanten Sicherungen geschrieben werden. Verweisen Sie damit auf ein zweites Volume, um den Verlust des ersten zu überstehen. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Stunden zwischen zwei Sicherungen, 1 bis 8760. Die erste erfolgt ein Intervall nach dem Start, nicht beim Start. |
| `DUMBMONIT_DEMO` | *(aus)* | `1` für eine öffentliche, schreibgeschützte Demo mit einem festen fiktiven Bestand, Anmeldung `demo` / `demo`. **Erstellt die Datenbank bei jedem Start neu**: nie auf einem echten Datenvolume. Siehe [Eine Demo-Instanz betreiben](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Wie viele Sicherungen aufbewahrt werden, 1 bis 365. Die ältesten werden samt ihrer `.key` entfernt. |
| `DUMBMONIT_BROWSER_URL` | *(nicht gesetzt)* | Headless Chromium für die Screenshots von [Website-Änderungen](../devices/webchange.md#screenshots), als `http://host:9222` (sein DevTools-Port). Fügen Sie Ihrer Compose-Datei einen `chromedp/headless-shell`-Dienst hinzu, wie unter [Screenshots](../devices/webchange.md#screenshots) gezeigt. Nicht gesetzt: nur Textvergleich. |

Die Aufbewahrung der Baseline (60 Tage) und die 14-tägige Lernphase sind nicht konfigurierbar.

### Frühere Namen `EZYMONIT_*` {#former-ezymonit_-names}

DumbMonit hieß bis September 2026 EzyMonit. Jede Variable `DUMBMONIT_*` wird als Fallback auch
unter ihrem alten Namen `EZYMONIT_*` gelesen, mit einer Warnung beim Start pro noch so gesetzter
Variable (`EZYMONIT_X is deprecated, use DUMBMONIT_X`). Die alte Datenbankdatei
`/data/ezymonit.db` wird beim Start in `dumbmonit.db` umbenannt, und Agent-Tokens `ezym_…` bleiben
gültig (neue sind `dmon_…`). Das Sitzungscookie heißt jetzt `dumbmonit_session`: Der erste Start
nach der Umbenennung meldet alle einmalig ab.

### Eingebettetes VictoriaMetrics {#embedded-victoriametrics}

Das Image liefert das VictoriaMetrics-Binary mit. Sofern `DUMBMONIT_VM_URL` nicht gesetzt ist,
startet der Server es beim Start als Kindprozess, leitet seine Logzeilen ins Serverlog weiter,
startet es mit Backoff neu, wenn es stirbt, und stoppt es beim Herunterfahren (`SIGTERM`, 20 s
Gnadenfrist, dann Kill; das `stop_grace_period` der Compose-Datei lässt dafür Raum).

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Pfad des zu startenden VictoriaMetrics-Binarys. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Lauschadresse, innerhalb des Containers. Standardmäßig Loopback: Nur der Server spricht damit. Das Entwicklungs-Overlay setzt `0.0.0.0:8428` und veröffentlicht den Port. |
| `DUMBMONIT_VM_RETENTION` | `12` | Aufbewahrung in der Syntax von `-retentionPeriod`: eine Zahl von Monaten oder eine Dauer wie `30d` oder `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget für die Caches von VictoriaMetrics (`-memory.allowedBytes`). Ohne Budget dimensioniert VM sie für 60 % des RAMs des Hosts, und sein residenter Speicher wächst tagelang. Einige Dutzend Monitore passen in 256 MB; erhöhen Sie es für Hunderte von Hosts. |

Seine Daten liegen in `<data dir>/vm`, auf demselben Volume wie die Datenbank. Drei Flags sind vom
Server fest vorgegeben und nicht konfigurierbar: `-search.maxUniqueTimeseries=100000` (eine
ausufernde Abfrage kann das Budget nicht erschöpfen), `-search.maxConcurrentRequests=4` (eine
Oberfläche plus die Alarm-Engine) und `-dedup.minScrapeInterval=10s` (doppelte Punkte, die
weniger als zehn Sekunden auseinanderliegen, werden zusammengefasst; beim 30-s-Takt der
Collectors geht nichts verloren).

Ist `DUMBMONIT_VM_URL` gesetzt, entfällt all das: Der Server verwendet die von Ihnen genannte
Instanz, und die eingebettete wird nie gestartet. `GET /api/health` sagt, in welchem Fall Sie sind
(`victoria.embedded`).

## Einstellungen auf Compose-Ebene {#compose-level-variables}

Die mitgelieferte `docker-compose.yml` liest keine Variable und braucht keine
`.env`-Datei. Alles wird unter `environment:` des Dienstes gesetzt oder durch
Bearbeiten der Datei; siehe [Erweiterte Optionen](../install/docker.md#advanced-options)
(Port, Geheimnis, Aufbewahrung und Speicher, externes VictoriaMetrics,
Passwort-Reset, Screenshots). Das Entwicklungs-Overlay `docker-compose.dev.yml`
liest `DUMBMONIT_VM_PORT` (Standard `8428`) für den Host-Port des eingebetteten
VictoriaMetrics.

## Ports {#ports}

| Port | Container | Zweck |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Web-Oberfläche, API, Agent-Installer und Downloads, Agent-Ingest. Der einzige zu veröffentlichende Port. |
| `8428/tcp` | `dumbmonit` | HTTP-API des eingebetteten VictoriaMetrics, auf dem Loopback des Containers. Nur vom Entwicklungs-Overlay veröffentlicht. |

Ausgehend: SNMP (standardmäßig UDP 161) zu Geräten, HTTPS zu Proxmox, PBS, Synology und
Benachrichtigungsdiensten sowie alles, worauf Ihre Dienstmonitore zielen. ICMP-Ping braucht den
Sysctl `net.ipv4.ping_group_range` in der Compose-Datei (keine Capability).

## Volumes und Dateien {#volumes-and-files}

Ein Volume, `dumbmonit-data`, eingebunden unter `/data`. Sein Name ist in der Compose-Datei fest
vorgegeben, wie auch immer das Projekt heißt.

| Pfad (im Container) | Volume | Inhalt |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Monitore, Zugangsdaten (verschlüsselt), Regeln, Alarmzustand und -verlauf, Baselines, Stummschaltungen, Kanäle, Agent-Tokens, Passwort-Hash, Sitzungen. |
| `/data/secret.key` | `dumbmonit-data` | Das Instanzgeheimnis. **Sichern Sie es.** Ohne es sind verschlüsselte Zugangsdaten nicht wiederherstellbar, und der Server weigert sich zu starten. |
| `/data/backups/` | `dumbmonit-data` | Geplante lokale Sicherungen: `dumbmonit-<timestamp>.db` (eine Online-SQLite-Kopie) und die zugehörige `.key`. Standardmäßig täglich, sieben aufbewahrt. |
| `/data/vm/` | `dumbmonit-data` | Zeitreihen des eingebetteten VictoriaMetrics, standardmäßig 12 Monate. Leer, wenn `DUMBMONIT_VM_URL` gesetzt ist. |
| `/agents/` | Image | Agent-Binaries. Binden Sie ein anderes Verzeichnis ein und setzen Sie `DUMBMONIT_AGENT_DIR`, um eigene auszuliefern. |
| `/etc/ssl/certs/ca-certificates.crt` | Image | TLS-Wurzeln für ausgehendes HTTPS. Binden Sie hier Ihr eigenes Bundle ein, um einer privaten Zertifizierungsstelle zu vertrauen. |

## Das Passwort zurücksetzen {#resetting-the-password}

```bash
# DUMBMONIT_RESET_PASSWORD: "1" unter environment: in docker-compose.yml einfügen
docker compose up -d
# open the UI: /setup asks for a new password
# Zeile wieder entfernen, dann:
docker compose up -d
```

Das Zurücksetzen löscht das Passwort und jede Sitzung; Monitore, Regeln und Kanäle bleiben
unberührt.

## Agent {#agent}

Der Agent hat eigene Variablen und eine eigene Konfigurationsdatei: siehe
[Linux-, macOS-, FreeBSD- und Windows-Agent](../devices/agent.md#configuration-file).

### Erfassungsumfang {#collection-scope}

Was der Agent misst und damit, wie viele Serien ein Rechner erzeugt. Die Standardwerte halten
einen Host mit Dutzenden von Containern bei wenigen hundert Serien; jeder Schlüssel ist ein
Eintrag in `agent.yaml`, überschreibbar durch seine Umgebungsvariable.

| Schlüssel | Variable | Standard | Rolle |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Ausgelassene Netzwerkschnittstellen: Container- und VM-Schnittstellen, der Loopback. Eine Liste exakter Namen oder regulärer Ausdrücke (nicht verankert; alles mit einem Metazeichen ist ein Regex). Der Wert ersetzt den Standard; eine leere Liste behält jede Schnittstelle. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(leer)* | Positivliste, gleiche Syntax. Ist sie gesetzt, werden nur diese Schnittstellen gemeldet und `interfaces_ignore` nicht beachtet. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Aus Dateisystemnutzung und Platten-I/O ausgelassene Mountpunkte, gleiche Syntax. Wechseldatenträger unter `/run/media/` bleiben. Pseudo-Dateisysteme (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2` …) werden unabhängig vom Pfad immer übersprungen. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Eine Serie `cpu_core_usage_percent` pro Kern, zusätzlich zur Gesamtauslastung. `cpu_count` wird in beiden Fällen gemeldet. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Pro Zyklus detaillierte Container, laufende zuerst. Darüber hinaus werden Container nur gezählt (`container_count`, `container_running_count` bleiben exakt) und `container_series_skipped` sagt, wie viele. `0` zählt ohne Details. |

In der Umgebung sind Listen kommagetrennt (`^veth, ^br-, docker0`); in YAML entweder eine Liste
oder eine kommagetrennte Zeichenkette. Platten-I/O-Zähler (`disk_read_bytes`,
`disk_written_bytes`) tragen nur das Label `device`: eine Serie pro Blockgerät, egal wie oft es
eingehängt ist.
