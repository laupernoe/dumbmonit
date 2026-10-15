# Installation mit Docker {#install-with-docker}

DumbMonit läuft als ein einziger Container: Der Server `dumbmonit` (Erfassung, API,
Alarmierung, Web-Oberfläche) startet sein eigenes VictoriaMetrics zur Speicherung der Zeitreihen; die
Binärdatei ist im Image enthalten. Konfiguration und Zustand liegen in einer eingebetteten SQLite-
Datenbank. Ein Volume, `/data`, enthält die Datenbank, das Instanz-Geheimnis und die
Zeitreihen.

!!! note "Image-Tags"
    `ghcr.io/laupernoe/dumbmonit:latest` ist der letzte getaggte Release (amd64;
    arm64-Images sind vorerst pausiert); `:edge` folgt dem letzten Commit auf `dev`. Um aus dem Quellcode zu starten,
    baut `docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
    --build` dasselbe Image lokal (beim ersten Mal etwa zehn Minuten; nur Docker
    wird benötigt).

## Voraussetzungen {#prerequisites}

- Docker Engine mit dem Compose-Plugin (`docker compose version` funktioniert).
- Ein Rechner, der die zu überwachenden Geräte erreichen kann. Er muss von ihnen nicht
  erreichbar sein, außer bei [Agenten](../devices/agent.md), die ihre
  Messwerte per HTTP an den Server senden.
- Port `8080` ist auf dem Host frei, oder ein anderer Port Ihrer Wahl ([siehe unten](#advanced-options)).

## Die Compose-Datei {#the-compose-file}

Dies ist die vollständige `docker-compose.yml` (keine `.env`-Datei nötig); speichern
Sie sie in einem eigenen Verzeichnis:

```yaml
services:
  dumbmonit:
    image: ghcr.io/laupernoe/dumbmonit:latest
    ports:
      - "8080:8080"
    volumes:
      - dumbmonit-data:/data
      # Or a host directory; on a SELinux host (Fedora, RHEL) add `:Z`:
      # - ./dumbmonit:/data:Z
    # environment:
    #   PUID: "1000"   # owner of the files in /data (default 65532)
    #   PGID: "1000"
    restart: unless-stopped
    stop_grace_period: 30s
    read_only: true
    tmpfs: [/tmp]
    # Starts as root only to give /data to user 65532 (or PUID/PGID), then
    # drops root for good: a host directory works as it is. Remove `user` and
    # `cap_add` to run as 65532 from the start (the files must belong to it).
    user: "0:0"
    cap_drop: [ALL]
    cap_add: [CHOWN, DAC_READ_SEARCH, SETUID, SETGID]
    security_opt: ["no-new-privileges:true"]
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"

volumes:
  dumbmonit-data:
    name: dumbmonit-data
```

Starten Sie es dann:

```bash
docker compose up -d
```

Das veröffentlichte Image wird geladen; das schreibgeschützte Dateisystem, die
Capabilities, `PUID`/`PGID` und der sysctl `ping_group_range` sind unter
[Berechtigungen und Datenverzeichnis](#permissions) und
[ICMP-Ping ohne Capability](#icmp-ping-without-a-capability) erklärt. Optionen
(Port, Geheimnis, Aufbewahrung, externes VictoriaMetrics, Screenshots) sind
[Umgebungsvariablen, die Sie selbst ergänzen](#advanced-options).

!!! tip "Das Image selbst bauen"
    Aus einem Klon des Repositorys baut `docker compose -f docker-compose.yml -f
    docker-compose.dev.yml up -d --build` stattdessen dasselbe Image lokal (etwa
    zehn Minuten bei kaltem Cache; auf dem Host wird weder eine Rust- noch eine
    Node-Toolchain benötigt). Verwenden Sie das, um aus dem Quellcode oder einem
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

Fügen Sie dann Ihren ersten Monitor hinzu: siehe [Ersten Monitor hinzufügen](first-device.md).

## Umgebungsvariablen {#environment-variables}

Alles läuft über Umgebungsvariablen; keine ist erforderlich. Ergänzen Sie die
benötigten unter `environment:` des Dienstes, wie unter
[Erweiterte Optionen](#advanced-options) gezeigt.

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Lauschadresse innerhalb des Containers. |
| `PUID` / `PGID` | `65532` | Mit `user: "0:0"`: Benutzer und Gruppe, als die der Server läuft, und Besitzer der Dateien in `/data` ([Details](#permissions)). `DUMBMONIT_PUID` / `DUMBMONIT_PGID` funktionieren ebenfalls. |
| `DUMBMONIT_DATA_DIR` | `/data` | SQLite-Datenbank (`dumbmonit.db`), Instanz-Geheimnis (`secret.key`) und die Daten des eingebetteten VictoriaMetrics (`vm/`). |
| `DUMBMONIT_VM_URL` | *(nicht gesetzt)* | URL eines externen VictoriaMetrics. Wenn gesetzt, wird das eingebettete nicht gestartet. |
| `DUMBMONIT_VM_RETENTION` | `12` | Aufbewahrung des eingebetteten VictoriaMetrics: Monate, oder `30d`, `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Speicherbudget der Caches des eingebetteten VictoriaMetrics. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Lauschadresse des eingebetteten VictoriaMetrics, innerhalb des Containers. |
| `DUMBMONIT_SECRET` | *(erzeugt)* | Instanz-Geheimnis, das Monitor-Zugangsdaten und Tokens verschlüsselt. |
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
    Ohne ihn sind die Monitor-Zugangsdaten unwiederbringlich: Eine allein wiederhergestellte
    Datenbank ergibt eine Instanz, die mit nichts sprechen kann. Der Server erkennt beim Start einen
    fehlenden oder geänderten Schlüssel und weigert sich mit einer eindeutigen
    Meldung weiterzumachen, statt bei jeder Prüfung stillschweigend zu scheitern. Die geplanten Backups
    kopieren ihn für Sie — siehe [Backup und Wiederherstellung](backup.md).

Wenn Sie das Geheimnis lieber selbst besitzen möchten, setzen Sie stattdessen `DUMBMONIT_SECRET` in der Compose-Datei
([Beispiel](#advanced-options)) (32 zufällige Zeichen oder mehr). Bewahren Sie es in Ihrem Passwortmanager auf.

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

### Bind-Mounts {#from-cap-drop-all}

Eine bestehende `docker-compose.yml` funktioniert unverändert weiter: Das Image
läuft weiterhin als Benutzer 65532. Die neue Datei ergänzt `user: "0:0"` und eine
`cap_add`-Zeile, damit ein Host-Verzeichnis ohne Vorbereitung funktioniert
([Berechtigungen](#permissions)); übernehmen Sie diese beiden Zeilen, wenn Sie auf
ein Bind-Mount umsteigen möchten.

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
| `dumbmonit.db` | Monitore, Regeln, Kanäle, Alarmzustand, Sitzungen. |
| `secret.key` | Das Instanz-Geheimnis. |
| `backups/` | Die geplanten lokalen Backups: eine Kopie der Datenbank und des Geheimnisses, standardmäßig täglich, sieben werden behalten. |
| `vm/` | VictoriaMetrics-Zeitreihen (standardmäßig 12 Monate Aufbewahrung). |

DumbMonit sichert seine eigene Datenbank nach Zeitplan und exportiert die gesamte
Konfiguration — Monitore, Zugangsdaten, Regeln, Kanäle — als einzelne Datei,
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

## Erweiterte Optionen {#advanced-options}

Die Compose-Datei oben enthält bewusst keine Variablen. Um eine Einstellung zu
ändern, ergänzen Sie unter `dumbmonit:` in Ihrer `docker-compose.yml` einen
`environment:`-Block (oder eine Zeile darin) und führen Sie erneut
`docker compose up -d` aus. Der Container wird neu erstellt; das Datenvolume
bleibt erhalten.

**Ein anderer Port.** Ändern Sie die linke Seite des Mappings; der Container
lauscht weiterhin auf 8080:

```yaml
    ports:
      - "8099:8080"
```

**Ihr eigenes Geheimnis** statt des in `/data/secret.key` erzeugten. Es
verschlüsselt die Zugangsdaten der Monitore: Geht es verloren, müssen alle neu
eingegeben werden.

```yaml
    environment:
      DUMBMONIT_SECRET: replace-me-with-32-random-characters
```

**Aufbewahrung und Speicher** des eingebetteten VictoriaMetrics (Monate, oder
`30d` / `2y`; ein Homelab mit einigen Dutzend Monitoren passt in 256 MB):

```yaml
    environment:
      DUMBMONIT_VM_RETENTION: "24"
      DUMBMONIT_VM_MEMORY: 512MB
```

**Externes VictoriaMetrics**; das eingebettete wird dann nicht gestartet:

```yaml
    environment:
      DUMBMONIT_VM_URL: http://victoriametrics:8428
```

**Passwort vergessen.** Fügen Sie die folgende Zeile hinzu, führen Sie
`docker compose up -d` aus, legen Sie in der Oberfläche den ersten Administrator
neu an (Monitore, Regeln und Kanäle bleiben), entfernen Sie die Zeile dann wieder
und führen Sie `docker compose up -d` noch einmal aus:

```yaml
    environment:
      DUMBMONIT_RESET_PASSWORD: "1"
```

**Screenshots von Website-Änderungen**: ein Headless-Chromium neben dem Server; siehe unten [Screenshots von Website-Änderungen](#website-screenshots).

Weitere Variablen: [Konfigurationsreferenz](../reference/configuration.md).

## Screenshots von Website-Änderungen {#website-screenshots}

Der [Website-Änderungsmonitor](../devices/webchange.md) vergleicht immer den sichtbaren Text der Seiten. Für einen Screenshot jeder neuen Version braucht es einen Browser, und das DumbMonit-Image enthält keinen: Ein Headless-Chromium läuft als zweiter Dienst, und `DUMBMONIT_BROWSER_URL` sagt dem Server, wo er zu finden ist.

Fügen Sie diese Zeilen Ihrer `docker-compose.yml` hinzu und führen Sie dann `docker compose up -d` aus:

```yaml
services:
  dumbmonit:
    environment:
      DUMBMONIT_BROWSER_URL: http://browser:9222
  browser:
    image: chromedp/headless-shell:stable
    shm_size: 256m
    restart: unless-stopped
```

- `browser` ist der Dienstname; im Compose-Netzwerk führt `http://browser:9222` dorthin. Kein Port wird veröffentlicht: Nur DumbMonit spricht mit ihm.
- `shm_size: 256m`: Chromium rendert in `/dev/shm`, und Dockers Standard von 64 MB ist für eine lange Seite zu klein.
- Jedes mit `--remote-debugging-port` gestartete Chromium funktioniert; hier wird das Image `chromedp/headless-shell` verwendet.

Ohne Browser funktioniert alles andere wie gewohnt (Textvergleich, Alarme, Vorher/Nachher-Kopien); die Schnappschüsse haben nur kein Bild. Solange der Server keinen Browser erreicht, zeigt das Formular eines Website-Änderungsmonitors einen Hinweis mit diesen Zeilen.
Bleibt der Hinweis nach dem Neustart bestehen, prüfen Sie `docker compose logs browser`, ob der Hostname in `DUMBMONIT_BROWSER_URL` der Dienstname ist und ob beide Dienste im selben Compose-Projekt liegen.

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

## Berechtigungen und Datenverzeichnis {#permissions}

Der Server läuft nie als root. Standardmäßig läuft das Image als Benutzer
`65532:65532`, dem ein von Docker angelegtes benanntes Volume bereits gehört.

Ein Bind-Mount (`./dumbmonit:/data`) behält dagegen den Besitzer des
Host-Verzeichnisses, oft root, wenn Docker es angelegt hat. Deshalb startet die
Compose-Datei oben den Container als root (`user: "0:0"`). Der Server bleibt dann
nur einen Augenblick root: Bevor er die Datenbank öffnet, VictoriaMetrics startet
oder im Netzwerk lauscht, übergibt er `/data` an `65532:65532` (nur die Einträge,
die jemand anderem gehören, ohne symbolischen Links zu folgen), gibt root
endgültig ab und prüft, dass er es nicht zurückerlangen kann. Die vier
Capabilities in `cap_add` dienen nur diesem Schritt; danach bleibt keine übrig.
`no-new-privileges` und ein schreibgeschütztes Root-Dateisystem bleiben aktiv:
`/data` und `/tmp` sind die einzigen beschreibbaren Pfade.

**Besitzer wählen: `PUID` und `PGID`.** Legen Sie mit `user: "0:0"` die uid und gid
fest, als die der Server läuft; die Dateien werden beim Start an sie übergeben:

```yaml
    environment:
      PUID: "1000"
      PGID: "1000"
```

`DUMBMONIT_PUID` / `DUMBMONIT_PGID` funktionieren ebenfalls und haben Vorrang.
`0` wird abgelehnt. Unter Unraid entsprechen `PUID=99` und `PGID=100` dem
üblichen Besitzer von `appdata`. Ohne `user: "0:0"` werden `PUID`/`PGID` ignoriert.

**Ohne `user: "0:0"`** (ein einfaches `docker run -v ./data:/data` oder `--user
1000:1000`) ändert sich kein Besitzer, und `/data` muss bereits beschreibbar sein.
Ist es das nicht, bricht der Server ab und schlägt zuerst `--user 0:0` vor, dann
`sudo chown -R 65532:65532 ./data` auf dem Host für ein Bind-Mount oder, für ein
benanntes Volume:

```bash
docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
```

### SELinux und Podman {#selinux-and-podman}

Auf einem Host, auf dem SELinux erzwungen wird (Fedora, RHEL, CentOS Stream,
AlmaLinux …), wird dem Container ein Bind-Mount verweigert, auch als root,
solange er kein Container-Label trägt. Hängen Sie `:Z` an das Volume an, oder
`:z`, wenn andere Container das Verzeichnis teilen:

```yaml
    volumes:
      - ./dumbmonit:/data:Z
```

Ohne das beendet sich der Server mit `the host enforces SELinux and the volume has
no container label`. Benannte Volumes werden von Docker mit Labels versehen und
brauchen nichts.

Rootless Podman funktioniert genauso, mit `--user 0:0`. Root im Container ist Ihr eigener
Benutzer auf dem Host, und die Dateien eines Bind-Mounts gehen an die uid 65532
des Containers, die der Host als eine Ihrer Sub-uids anzeigt (`165531` bei der
üblichen `/etc/subuid`). Lesen Sie sie mit `podman unshare ls -l ./dumbmonit`.
Soll stattdessen Ihre eigene uid Besitzer bleiben, führen Sie den Container als
sich selbst aus; dann ändert sich kein Besitzer:

```bash
podman run --userns keep-id --user "$(id -u):$(id -g)" -v ./dumbmonit:/data:Z …
```

## ICMP-Ping ohne Capability {#icmp-ping-without-a-capability}

Der [Ping-Monitor](../devices/services.md#ping) sendet ICMP-Echos. Statt eines
Raw-Sockets — der `NET_RAW` braucht, und eine einem Nicht-root-Containerbenutzer gewährte Capability ist ohnehin nicht nutzbar —
verwendet er einen ICMP-Echo-Socket, den Linux den in `net.ipv4.ping_group_range`
aufgeführten Gruppen erlaubt. Die Compose-Datei
setzt diesen sysctl im eigenen Netzwerk-Namespace des Containers:

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Auf dem Host ändert sich nichts. Docker 20.10 und neuer setzen diesen Bereich in jedem
Container von selbst; die expliziten Zeilen sorgen dafür, dass er auch auf älteren Engines und
bei Podman gilt. Ohne ihn meldet die Prüfung einen Konfigurationsfehler (am
Monitor angezeigt, nicht benachrichtigt), nie ein falsches „Host down". Bei `docker run` übergeben Sie
`--sysctl net.ipv4.ping_group_range="0 2147483647"`. Solange der Server keinen ICMP-Socket öffnen kann, zeigt das Formular eines Ping-Monitors einen Hinweis mit diesen Zeilen.

## Testen ohne Hardware {#testing-without-hardware}

Fügen Sie ein [Demo-Monitor](../devices/demo.md) hinzu: Es erzeugt Fake-Messwerte, ohne
dass etwas vorzubereiten ist, sodass Diagramme, Regeln und Benachrichtigungen sofort
ausprobiert werden können. Für echte SNMP-Daten genügt oft der Rechner, auf dem Docker läuft — installieren Sie
darauf `snmpd` (`apt install snmpd`, die Community `public` auf der Docker-
Bridge in `/etc/snmp/snmpd.conf` erlauben) und fügen Sie ein SNMP-Monitor mit der
Adresse des Hosts auf dieser Bridge hinzu (standardmäßig `172.17.0.1`): Das Profil wird
automatisch erkannt, und Schnittstellen, Speicher und Prozesse erscheinen.

Entwickler können das Overlay `docker-compose.dev.yml` hinzufügen, um das eingebettete
VictoriaMetrics auf dem `:8428` des Hosts für direkte MetricsQL-Abfragen zu veröffentlichen:

```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```
