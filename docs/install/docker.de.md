# Installation mit Docker {#install-with-docker}

DumbMonit läuft als ein einziger Container: Der Server `dumbmonit` (Erfassung, API,
Alarmierung, Web-Oberfläche) startet sein eigenes VictoriaMetrics zur Speicherung der Zeitreihen; die
Binärdatei ist im Image enthalten. Konfiguration und Zustand liegen in einer eingebetteten SQLite-
Datenbank. Ein Volume, `/data`, enthält die Datenbank, das Instanz-Geheimnis und die
Zeitreihen.

!!! note "Alpha-Image"
    `ghcr.io/laupernoe/dumbmonit:latest` ist der letzte getaggte Alpha-Build (amd64;
    arm64-Images sind vorerst pausiert); `:edge` folgt dem letzten Commit auf `main`. Um aus dem Quellcode zu starten,
    baut `docker compose up -d --build` dasselbe Image lokal (beim ersten Mal etwa zehn
    Minuten; nur Docker wird benötigt).

## Voraussetzungen {#prerequisites}

- Docker Engine mit dem Compose-Plugin (`docker compose version` funktioniert).
- Ein Rechner, der die zu überwachenden Geräte erreichen kann. Er muss von ihnen nicht
  erreichbar sein, außer bei [Agenten](../devices/agent.md), die ihre
  Messwerte per HTTP an den Server senden.
- Port `8080` ist auf dem Host frei, oder ein anderer Port Ihrer Wahl (siehe unten).

## Die Compose-Datei {#the-compose-file}

Dies ist die `docker-compose.yml` aus dem Repository; speichern Sie sie in einem
eigenen Verzeichnis:

```yaml
# Ein Container: Der DumbMonit-Server startet sein eigenes VictoriaMetrics (im
# Image eingebettet) und hält alles in einem einzigen Volume. Um stattdessen ein externes
# VictoriaMetrics zu verwenden, setzen Sie DUMBMONIT_VM_URL; das eingebettete wird dann nicht gestartet.
name: dumbmonit

services:
  dumbmonit:
    # `docker compose up -d` lädt das veröffentlichte Image (letztes Release; mit
    # `:edge` den letzten Commit auf main). Um stattdessen aus diesem Checkout zu bauen,
    # führen Sie `docker compose up -d --build` aus: Das Ergebnis erhält denselben Namen
    # und wird ab dann verwendet.
    image: ghcr.io/laupernoe/dumbmonit:latest
    build: .
    ports:
      # Der Host-Port ist konfigurierbar: 8080 ist auf einem Homelab-Rechner ein
      # stark belegter Port. `DUMBMONIT_PORT=8099 docker compose up -d` verschiebt ihn.
      - "${DUMBMONIT_PORT:-8080}:8080"
    volumes:
      # SQLite-Datenbank, Instanz-Geheimnis und die Zeitreihen (/data/vm).
      #
      # Der Server läuft als Benutzer 65532 (nicht root). Ein von Docker angelegtes
      # benanntes Volume erbt diesen Besitzer aus dem Image: nichts zu tun. Ein Bind-Mount
      # (`./data:/data`) oder ein vor dieser Änderung angelegtes Volume muss einmalig
      # übergeben werden:
      #   docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
      # oder, um die Dateien unverändert zu lassen, den Container als deren Besitzer
      # ausführen mit `user: "1000:1000"` (jede uid funktioniert: dem Image fehlt /etc/passwd).
      - dumbmonit-data:/data
    environment:
      # Auskommentieren, um das Geheimnis selbst festzulegen, statt es von DumbMonit
      # in /data/secret.key erzeugen zu lassen. Es verschlüsselt die Zugangsdaten der Geräte: Geht es
      # verloren, müssen alle neu eingegeben werden.
      # DUMBMONIT_SECRET: replace-me-with-32-random-characters
      # Passwort vergessen: `DUMBMONIT_RESET_PASSWORD=1 docker compose up -d` löscht
      # das Passwort beim Start, und die Oberfläche fragt nach einem neuen; danach erneut
      # ohne die Variable starten.
      DUMBMONIT_RESET_PASSWORD: ${DUMBMONIT_RESET_PASSWORD:-}
      # Eingebettetes VictoriaMetrics: Aufbewahrung (Monate, oder z. B. 30d / 2y) und das
      # Speicherbudget seiner Caches. Ein Homelab mit einigen Dutzend Geräten passt in
      # 256 MB; erhöhen Sie es, wenn Sie Hunderte von Hosts überwachen.
      DUMBMONIT_VM_RETENTION: ${DUMBMONIT_VM_RETENTION:-12}
      DUMBMONIT_VM_MEMORY: ${DUMBMONIT_VM_MEMORY:-256MB}
      # Externes VictoriaMetrics statt des eingebetteten:
      # DUMBMONIT_VM_URL: http://victoriametrics:8428
    # Minimale Rechte: keinerlei Capability, keine Rechteausweitung, und das
    # Dateisystem des Images schreibgeschützt — /data (Volume) und /tmp (tmpfs) sind die
    # einzigen beschreibbaren Orte. ICMP-„Ping"-Monitore brauchen ebenfalls keine Capability: Der
    # sysctl unten erlaubt dem (unprivilegierten) Server, ICMP-Echo-Sockets im
    # eigenen Netzwerk-Namespace des Containers zu öffnen. Entfernen Sie ihn, wenn Sie nie Ping verwenden.
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    read_only: true
    tmpfs:
      - /tmp
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
    # Der Server beendet VictoriaMetrics nach sich selbst: Lassen Sie ihm die Zeit dafür.
    stop_grace_period: 30s
    restart: unless-stopped

volumes:
  dumbmonit-data:
    # Fester Name, unabhängig vom Compose-Projektnamen: Das Volume wird gesichert
    # und muss bei einem Upgrade wiedergefunden werden.
    name: dumbmonit-data
```

Starten Sie es dann:

```bash
docker compose up -d
```

!!! tip "Das Image selbst bauen"
    `docker compose up -d` lädt das veröffentlichte Image und ignoriert `build: .`.
    Aus einem Klon des Repositorys baut `docker compose up -d --build` stattdessen
    dasselbe Image lokal (etwa zehn Minuten bei kaltem Cache; auf dem Host wird weder eine Rust- noch eine Node-
    Toolchain benötigt). Verwenden Sie das, um aus dem Quellcode oder einem
    Branch zu starten.

## Eingebettetes VictoriaMetrics {#embedded-victoriametrics}

Das Image enthält die VictoriaMetrics-Binärdatei (`/victoria-metrics-prod`, aus
`victoriametrics/victoria-metrics:v1.152.0`). Ist `DUMBMONIT_VM_URL` nicht
gesetzt, startet der Server sie als Kindprozess, der auf `127.0.0.1:8428` lauscht,
legt ihre Serien unter `/data/vm` ab, leitet ihre Logzeilen in sein eigenes Log weiter,
startet sie mit Backoff neu, falls sie stirbt, und beendet sie beim Herunterfahren. Nichts wird
veröffentlicht: Der Port bleibt im Container.

Zwei Variablen sind wissenswert: `DUMBMONIT_VM_RETENTION` (standardmäßig `12` Monate;
`30d` oder `2y` funktionieren ebenfalls) und `DUMBMONIT_VM_MEMORY` (`256MB`, das
Budget seiner Caches; erhöhen Sie es für Hunderte von Hosts). Der Rest steht in der
[Konfigurationsreferenz](../reference/configuration.md#embedded-victoriametrics).

Um ein bereits betriebenes VictoriaMetrics zu verwenden, setzen Sie `DUMBMONIT_VM_URL` auf dessen
Adresse (`http://host:8428`): Das eingebettete wird dann nicht gestartet und
`/data/vm` bleibt leer.

## Erster Start {#first-start}

Öffnen Sie `http://<your-host>:8080`. Eine frische Instanz zeigt den Bildschirm `/setup`,
auf dem Sie das erste Administratorkonto anlegen (Passwort mit mindestens 12
Zeichen).

Zuerst fragt der Bildschirm nach dem **Setup-Code**. Solange kein Administrator existiert,
gibt der Server beim Start einen einmaligen Code in seinen Logs aus:

```sh
docker compose logs dumbmonit | grep -A1 "setup code"
```

```
  First-run setup code: K7XQ4-M9PRT
```

Er belegt, dass derjenige, der das Admin-Konto anlegt, den Server betreibt, sodass
niemand sonst im Netzwerk eine frische Instanz zuerst in Besitz nehmen kann. Der Code existiert nur
im Speicher, ändert sich bei jedem Neustart, bis ein Admin existiert, und falsche
Versuche werden durch ein Rate-Limit begrenzt. Für eine automatisierte Bereitstellung setzen Sie
`DUMBMONIT_SETUP_CODE`, um ihn selbst zu wählen.

![Der Anmeldebildschirm](../assets/screenshots/login-light.png){ loading=lazy }

Fügen Sie dann Ihr erstes Gerät hinzu: siehe [Erstes Gerät hinzufügen](first-device.md).

## Umgebungsvariablen {#environment-variables}

Alles läuft über Umgebungsvariablen; keine ist erforderlich.

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Lauschadresse innerhalb des Containers. |
| `DUMBMONIT_DATA_DIR` | `/data` | SQLite-Datenbank (`dumbmonit.db`), Instanz-Geheimnis (`secret.key`) und die Daten des eingebetteten VictoriaMetrics (`vm/`). |
| `DUMBMONIT_VM_URL` | *(nicht gesetzt)* | URL eines externen VictoriaMetrics. Wenn gesetzt, wird das eingebettete nicht gestartet. |
| `DUMBMONIT_VM_RETENTION` | `12` | Aufbewahrung des eingebetteten VictoriaMetrics: Monate, oder `30d`, `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Speicherbudget der Caches des eingebetteten VictoriaMetrics. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Lauschadresse des eingebetteten VictoriaMetrics, innerhalb des Containers. |
| `DUMBMONIT_SECRET` | *(erzeugt)* | Instanz-Geheimnis, das Gerätezugangsdaten und Tokens verschlüsselt. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Gleichzeitige Prüfungen, alle Collectors zusammen. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Maximale Dauer einer Prüfung. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Schreibintervall Richtung VictoriaMetrics. |
| `DUMBMONIT_LOG` | `info` | Log-Filter (`tracing`-Syntax, z. B. `debug`, `dumbmonit=trace`). |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Agent-Binärdateien, ausgeliefert unter `/download/…`. |
| `DUMBMONIT_RESET_PASSWORD` | *(leer)* | Auf `1` setzen, um beim Start das Passwort und alle Sitzungen zu löschen. |
| `DUMBMONIT_SETUP_CODE` | *(erzeugt)* | Setup-Code, den `/setup` verlangt, solange kein Admin existiert. Ist er nicht gesetzt, wird bei jedem Start ein zufälliger in den Logs ausgegeben. |
| `DUMBMONIT_COOKIE_SECURE` | *(aus)* | Hinter einem TLS-Reverse-Proxy auf `1` setzen, um das Sitzungs-Cookie als `Secure` zu markieren. |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Auswertungsintervall der Alarme (nie unter 10). |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | 90 Tage | Aufbewahrung des Alarmverlaufs. Zu einem Vorbehalt bei der Einheit siehe die [Konfigurationsreferenz](../reference/configuration.md). |
| `DUMBMONIT_BACKUP_ENABLED` | an | Geplante lokale Backups der Datenbank in `/data/backups/`. `DUMBMONIT_BACKUP_DIR`, `_INTERVAL_HOURS` (24) und `_KEEP` (7) justieren sie; siehe [Backup und Wiederherstellung](backup.md). |

Die vollständige Liste mit Details steht in der [Konfigurationsreferenz](../reference/configuration.md).
`EZYMONIT_*`-Namen aus der Zeit vor der Umbenennung werden weiterhin als Fallback gelesen; siehe
[Upgrade](#upgrading).

## Den geheimen Schlüssel sichern {#back-up-the-secret-key}

SNMP-Communities, Passwörter und API-Tokens werden mit AES-256-GCM verschlüsselt, mit einem
Schlüssel, der aus dem Instanz-Geheimnis abgeleitet wird. Beim ersten Start erzeugt DumbMonit dieses
Geheimnis in `/data/secret.key` (im Volume `dumbmonit-data`).

!!! danger "`secret.key` zusammen mit der Datenbank sichern"
    Ohne ihn sind die Gerätezugangsdaten unwiederbringlich: Eine allein wiederhergestellte
    Datenbank ergibt eine Instanz, die mit nichts sprechen kann. Der Server erkennt beim Start einen
    fehlenden oder geänderten Schlüssel und weigert sich mit einer eindeutigen
    Meldung weiterzumachen, statt bei jeder Prüfung stillschweigend zu scheitern. Die geplanten Backups
    kopieren ihn für Sie — siehe [Backup und Wiederherstellung](backup.md).

Wenn Sie das Geheimnis lieber selbst besitzen möchten, setzen Sie stattdessen `DUMBMONIT_SECRET` in der Compose-Datei
(32 zufällige Zeichen oder mehr). Bewahren Sie es in Ihrem Passwortmanager auf.

## Upgrade {#upgrading}

```bash
docker compose pull
docker compose up -d
```

Datenbankmigrationen laufen beim Start. Die VictoriaMetrics-Daten bleiben unberührt.

!!! tip "Zuerst ein Backup erstellen"
    **Settings → Backup → Back up now** schreibt in wenigen Sekunden eine konsistente Kopie der Datenbank
    und von `secret.key` nach `/data/backups/`; daraus stellen Sie
    wieder her, falls das Upgrade schiefgeht. Siehe
    [Backup und Wiederherstellung](backup.md).

### Von 0.1.0-alpha.1 {#from-010-alpha1}

Seit 0.1.0-alpha.2 läuft der Container als Benutzer 65532 statt als root. Ein
von alpha.1 angelegtes Volume gehört noch root, und der Server weigert sich zu
starten (`/data is not writable by the server`). Übergeben Sie das Volume einmalig:

```bash
docker compose down
docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
docker compose up -d
```

Oder lassen Sie die Dateien unverändert und führen den Container als deren Besitzer aus, mit
`user: "0:0"` (oder der uid eines Bind-Mounts) unter dem Service in
`docker-compose.yml`.

### Von EzyMonit und vom Setup mit zwei Containern {#from-ezymonit-and-from-the-two-container-setup}

DumbMonit hieß bis September 2026 EzyMonit und lief als zwei Container,
der Server und ein separater Service `victoriametrics`. Für das Upgrade müssen die Daten
des alten Projekts einmalig in das neue Volume verschoben werden. Das alte Projekt hieß
`ezymonit` (seine Volumes `ezymonit_ezymonit-data` und `ezymonit_vm-data`;
`docker volume ls` bestätigt das). Aus dem alten Verzeichnis:

```bash
docker compose down
```

Dann, mit der neuen `docker-compose.yml`:

```bash
docker volume create dumbmonit-data
docker run --rm -v ezymonit_ezymonit-data:/from -v dumbmonit-data:/to alpine cp -a /from/. /to/
docker run --rm -v ezymonit_vm-data:/from -v dumbmonit-data:/to alpine sh -c 'mkdir -p /to/vm && cp -a /from/. /to/vm/'
docker compose up -d
```

Das alte VictoriaMetrics-Datenverzeichnis hat das Layout, das auch das eingebettete verwendet:
nichts zu konvertieren. Die Alternative ist, das externe VictoriaMetrics beizubehalten und
`DUMBMONIT_VM_URL` darauf zeigen zu lassen.

Was der Server beim ersten Start selbst erledigt:

- `EZYMONIT_*`-Variablen werden weiterhin als Fallback gelesen, mit einer Warnung beim Start
  pro Variable (`EZYMONIT_X is deprecated, use DUMBMONIT_X`).
- `/data/ezymonit.db` wird in `dumbmonit.db` umbenannt.
- Agent-Tokens `ezym_…` funktionieren weiter; neue sind `dmon_…`. Agenten senden
  weiter; führen Sie den Installationsbefehl bei Gelegenheit auf jeder Maschine erneut aus, er
  [migriert den alten Dienst an Ort und Stelle](../devices/agent.md#upgrade).
- Das Sitzungs-Cookie hat seinen Namen geändert: Alle melden sich einmal neu an. Die Theme-Einstellung
  des Browsers wird übernommen.

Was er nicht erledigt: Das Metrik-Präfix wurde ohne Kompatibilität von `ezymonit_` auf
`dumbmonit_` geändert. Die alten Serien bleiben unter dem alten Namen in VictoriaMetrics
und laufen mit der Aufbewahrung ab; Diagramme und eingebaute Regeln beginnen
mit dem Upgrade von vorn. Eigene Alarmregeln, die `ezymonit_…`-Metriken nennen, müssen
angepasst werden.

## Backup und Wiederherstellung {#backup-and-restore}

Ein benanntes Volume, `dumbmonit-data`, enthält alles:

| Pfad | Inhalt |
|---|---|
| `dumbmonit.db` | Geräte, Regeln, Kanäle, Alarmzustand, Sitzungen. |
| `secret.key` | Das Instanz-Geheimnis. |
| `backups/` | Die geplanten lokalen Backups: eine Kopie der Datenbank und des Geheimnisses, standardmäßig täglich, sieben werden behalten. |
| `vm/` | VictoriaMetrics-Zeitreihen (standardmäßig 12 Monate Aufbewahrung). |

DumbMonit sichert seine eigene Datenbank nach Zeitplan und exportiert die gesamte
Konfiguration — Geräte, Zugangsdaten, Regeln, Kanäle — als einzelne Datei,
verschlüsselt mit einer Passphrase Ihrer Wahl, die sich auf einer frischen Instanz wiederherstellen lässt.
Beides steht in **[Backup und Wiederherstellung](backup.md)**, zusammen mit dem, was vor
einem Upgrade zu tun ist, und der einen Regel zu `secret.key`, die man zweimal lesen sollte.

Um das Volume selbst samt Diagrammen zu archivieren, stoppen Sie den Stack und packen es mit tar:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Zum Wiederherstellen legen Sie das Volume an, entpacken das Archiv auf dieselbe Weise hinein und führen dann
`docker compose up -d` aus.

Wenn Speicherplatz knapp ist, hält `--exclude=./vm` das Archiv klein: Die Datenbank und
das Geheimnis sind das Setup, `vm/` sind nur die Diagramme.

## Den Port ändern {#changing-the-port}

Setzen Sie `DUMBMONIT_PORT` beim Start:

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Oder tragen Sie `DUMBMONIT_PORT=8099` in eine `.env`-Datei neben der Compose-Datei ein.

## Reverse-Proxy {#reverse-proxy}

Die Oberfläche und die API werden auf einem Port über einfaches HTTP ausgeliefert. Es wird kein WebSocket verwendet:
Die Oberfläche fragt die API per Polling ab, sodass jeder Reverse-Proxy ohne besondere
Konfiguration funktioniert.

=== "Caddy"

    ```
    monit.example.com {
        reverse_proxy 127.0.0.1:8080
    }
    ```

=== "nginx"

    ```nginx
    server {
        listen 443 ssl;
        server_name monit.example.com;
        # ssl_certificate / ssl_certificate_key …

        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            # Agenten senden nach einem Ausfall nachgeholte Batches:
            client_max_body_size 16m;
        }
    }
    ```

Wenn der Proxy TLS terminiert, setzen Sie `DUMBMONIT_COOKIE_SECURE: "1"`, damit das Sitzungs-
Cookie nur über HTTPS gesendet wird. Setzen Sie es nicht bei einer reinen HTTP-Bereitstellung: Der
Browser würde das Cookie nie zurücksenden, und eine Anmeldung wäre unmöglich.

!!! note "Agenten hinter einem Proxy"
    Der Installationsbefehl, der beim Anlegen eines Agent-Tokens angezeigt wird, verwendet die URL, mit der Ihr
    Browser die Oberfläche erreicht hat. Ist das die URL hinter dem Proxy, verwenden die Agenten sie
    ebenfalls: Stellen Sie sicher, dass `/install.sh`, `/install.ps1`, `/download/…` und `/api/ingest`
    den Proxy passieren.

## Läuft als Nicht-root-Benutzer {#runs-as-a-non-root-user}

Der Container führt den Server als Benutzer `65532:65532` aus (numerisch: Dem `scratch`-
Image fehlt `/etc/passwd`), mit entfernten Capabilities, `no-new-privileges`
und einem schreibgeschützten Root-Dateisystem; `/data` (das Volume) und `/tmp` (ein tmpfs)
sind die einzigen beschreibbaren Pfade. All das steht in der obigen Compose-Datei.

Ein von Docker beim ersten Start angelegtes benanntes Volume erbt den Besitzer von `/data`
aus dem Image: nichts zu tun. Zwei Fälle brauchen einen Befehl:

- **Ein von einer früheren Version angelegtes Volume** (der Server lief früher als root,
  die Dateien gehören also root). Übergeben Sie sie einmalig, bei gestopptem Stack:

  ```bash
  docker compose stop
  docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
  docker compose up -d
  ```

  Das Symptom ist sonst ein Container, der sofort mit
  `creating directory /data … Permission denied` (oder `unable to open database
  file`) beendet wird.

- **Ein Bind-Mount** (`./data:/data`) behält den Besitzer des Host-Verzeichnisses.
  Entweder `chown -R 65532:65532 ./data`, oder den Container als Besitzer des Verzeichnisses ausführen:
  `user: "1000:1000"` unter dem Service, jede uid funktioniert.

## ICMP-Ping ohne Capability {#icmp-ping-without-a-capability}

Der [Ping-Monitor](../devices/services.md#ping) sendet ICMP-Echos. Statt eines
Raw-Sockets — der `NET_RAW` braucht, und eine einem Nicht-root-Containerbenutzer gewährte Capability ist ohnehin nicht nutzbar —
verwendet er einen ICMP-Echo-Socket, den Linux den in `net.ipv4.ping_group_range`
aufgeführten Gruppen erlaubt. Die Compose-Datei
setzt diesen sysctl im eigenen Netzwerk-Namespace des Containers:

```yaml
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Auf dem Host ändert sich nichts. Docker 20.10 und neuer setzen diesen Bereich in jedem
Container von selbst; die expliziten Zeilen sorgen dafür, dass er auch auf älteren Engines und
bei Podman gilt. Ohne ihn meldet die Prüfung einen Konfigurationsfehler (am
Gerät angezeigt, nicht benachrichtigt), nie ein falsches „Host down". Bei `docker run` übergeben Sie
`--sysctl net.ipv4.ping_group_range="0 2147483647"`.

## Testen ohne Hardware {#testing-without-hardware}

Fügen Sie ein [Demo-Gerät](../devices/demo.md) hinzu: Es erzeugt Fake-Messwerte, ohne
dass etwas vorzubereiten ist, sodass Diagramme, Regeln und Benachrichtigungen sofort
ausprobiert werden können. Für echte SNMP-Daten genügt oft der Rechner, auf dem Docker läuft — installieren Sie
darauf `snmpd` (`apt install snmpd`, die Community `public` auf der Docker-
Bridge in `/etc/snmp/snmpd.conf` erlauben) und fügen Sie ein SNMP-Gerät mit der
Adresse des Hosts auf dieser Bridge hinzu (standardmäßig `172.17.0.1`): Das Profil wird
automatisch erkannt, und Schnittstellen, Speicher und Prozesse erscheinen.

Entwickler können das Overlay `docker-compose.dev.yml` hinzufügen, um das eingebettete
VictoriaMetrics auf dem `:8428` des Hosts für direkte MetricsQL-Abfragen zu veröffentlichen:

```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```
