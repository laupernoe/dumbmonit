# Linux-, macOS-, FreeBSD- und Windows-Agent

Ein kleines, auf der zu überwachenden Maschine installiertes Binary.
Es misst CPU, Speicher, Platten, Netzwerk, Dienste und Container —
und, wo die Maschine sie exponiert, Temperaturen, Plattengesundheit
und ZFS-Pools — und **pusht** dann seine Messwerte über HTTP an den
DumbMonit-Server.

Die Richtung ist Absicht: Der Agent ruft den Server auf, nie
umgekehrt. Er durchquert NAT und Heim-Firewalls und erfordert, dass
**kein Port** auf der überwachten Maschine geöffnet wird.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_`. Identitätslabels
(`target`, `host`, `tag_*`) werden vom Server beim Empfang gesetzt,
nie vom Agent: Eine Maschine kann nicht in die Zeitreihe einer
anderen Maschine schreiben, selbst durch Fälschen ihrer Labels.

| Familie | Metriken | Labels |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` nur mit `cpu_per_core: true` | `core` |
| Speicher | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Dateisysteme | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Netzwerk (Zähler) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Platten (Zähler) | `disk_read_bytes`, `disk_written_bytes`, eine Zeitreihe pro Block-Gerät | `device` |
| Host | `uptime_seconds`, `process_count` | |
| Dienste | `service_up` (1 = laufend) | `service` |
| Container | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (Container über `docker_max_containers` hinaus) | `container`, `image` |
| Sensoren | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (der von der Hardware selbst angegebene Schwellenwert), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Plattengesundheit (SMART) | `agent_disk_smart_ok` (1 = die Platte besteht ihre eigene Selbstbeurteilung), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degraded, 2 schlimmer), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; pro Peer `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (Zähler) — siehe [WireGuard-Tunnel](#wireguard-tunnel) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — siehe [restic- und Borg-Backups](#restic--und-borg-backups) | `tool`, `repo` |
| Windows-Leistungszähler | `agent_perf_counter`, eine Zeitreihe pro in `perf_counters` gelistetem Zähler | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — siehe [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; pro VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; pro virtueller Platte `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — siehe [Hyper-V-Hosts](#hyper-v-hosts) | `vm`, `disk` |
| Agent | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Eine nicht messbare Zeitreihe ist **abwesend**, nie null: kein
`smartctl`, kein `agent_disk_*`; keine lesbare Sonde, kein
`agent_sensor_*`; kein ZFS, kein `agent_zfs_*`. Eine Temperatur
außerhalb von 1–150 °C wird verworfen statt veröffentlicht, und eine
Lüfterdrehzahl von 0 U/min wird auch verworfen — ein Mainboard hat
mehr Lüfteranschlüsse als Lüfter, und ein leerer liest sich null wie
ein toter Lüfter.

### Was wo gesammelt wird

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, Speicher, Swap, Dateisysteme, Netzwerk, Uptime, Prozesse | ja | ja | ja | ja |
| Load Average | ja | ja | ja | nein (Windows hat diesen Begriff nicht) |
| Dienste | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | Windows-Dienste |
| Docker-Container | ja | ja, wenn `docker_socket` auf den Socket von Docker Desktop zeigt | nein (kein Docker-Daemon) | nein |
| Temperaturen | `/sys/class/hwmon` | SMC-Sensoren | `dev.cpu.N.temperature` (nur CPU) | nein |
| Lüfter | `/sys/class/hwmon` | nein | nein | nein |
| Plattengesundheit (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| ZFS-Pools | `zpool` (OpenZFS) | nein | `zpool` | nein |
| OS-Gesundheit (Updates, Reboot, fehlgeschlagene Units, SELinux) | ja | nein | nein | nein |
| Plakar-Backups | ja | ja | ja | ja |
| WireGuard-Tunnel | `wg show` | wenn `wg` installiert ist (ungetestet) | wenn `wg` installiert ist (ungetestet) | nein |
| restic- und Borg-Backups | ja | ja | ja | nur restic (ungetestet; Borg hat keinen Windows-Build) |
| Leistungszähler, MDaemon-Warteschlangen, Hyper-V | nein | nein | nein | ja |
| Maschinenidentität | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | Hostname |
| Relay-Modus | ja | ja | ja | ja |

Windows hat absichtlich keine Temperaturzeile: Dort Sensoren zu lesen
bedeutet, über WMI zu gehen, mit administrativen Rechten, für Werte,
die die meisten Maschinen überhaupt nicht exponieren. Nichts zu
veröffentlichen ist die ehrliche Antwort.

`smartctl` (smartmontools) und `zpool` werden bei jeder Lese-Periode
gesucht und genutzt, wenn vorhanden; keines wird vom Agent
installiert, und keines ist erforderlich. Beide brauchen root — der
vom Installer registrierte Dienst läuft als root, ein von Hand unter
Ihrem eigenen Konto gestarteter Agent findet nichts.

Zähler werden roh gesendet, damit ein Neustart des Agents nie eine
gefälschte Rate erzeugt; wenden Sie `rate()` in Abfragen an.

Geltende eingebaute Regeln: Device unreachable, High CPU, Unusual
CPU. "Device unreachable" funktioniert auch für Agents: Der Server
prüft das Alter des letzten empfangenen Batches und erklärt die
Maschine nach drei verpassten Perioden (mindestens 90 Sekunden) als
still.

Vier weitere feuern nur auf Maschinen, die die passende Zeitreihe
erzeugen, brauchen daher keine Einrichtung und können auf einer
Maschine ohne die Hardware nicht losgehen:

| Regel | Feuert, wenn |
|---|---|
| **Disk SMART failing** (critical) | die eigene Selbstbeurteilung einer Platte sie als ausfallend meldet |
| **ZFS pool degraded** (critical) | ein Pool seit fünf Minuten nicht mehr `ONLINE` ist |
| **ZFS scrub found errors** (critical) | der letzte Scrub Fehler fand, oder der Pool permanent beschädigte Dateien meldet |
| **Temperature above critical** (critical) | ein Sensor den kritischen Schwellenwert überschreitet, **den seine eigene Hardware angibt** — kein Wert ist hartcodiert, 85 °C ist ein Notfall auf einer Platte und ein normaler Nachmittag auf einer Laptop-CPU |

Drei weitere beobachten, was Sie selbst deklarieren oder ausführen,
und bleiben sonst still: **WireGuard peer silent** (ein Peer mit
dauerhaftem Keepalive und seit 15 Minuten ohne Handshake),
**restic/Borg backup too old** (kein Snapshot seit 48 Stunden) und
**restic/Borg repository unreachable** — siehe
[WireGuard-Tunnel](#wireguard-tunnel) und
[restic- und Borg-Backups](#restic--und-borg-backups).

Auf einem Windows-Server, der MDaemon ausführt, beobachten drei
weitere seine Warteschlangen: **MDaemon mail queue growing**,
**MDaemon Bad queue not empty** und **MDaemon Retry queue high**
(siehe [Regeln](../alerting/rules.md#mdaemon-and-securitygateway)).
Auf einem Hyper-V-Host beobachten drei die virtuellen Maschinen:
**Hyper-V VM health critical**, **Hyper-V host CPU high** und
**Hyper-V virtual disk errors** (siehe
[Hyper-V-Hosts](#hyper-v-hosts)).

## Installation

Die Schritte unten sind die, die der Hinweis neben dem Formular
zeigt. Es gibt kein auf der Maschine zu erstellendes Konto: Das
Enrollment-Token ist das einzige Secret.

1. Speichern Sie das Gerät in DumbMonit: Ein Enrollment-Token
   (dmon_…) wird einmal angezeigt, mit dem fertigen
   Installationsbefehl für Linux und für Windows. (Ein Token kann
   auch unter **Settings → Agents** erstellt werden.)
2. Dieses Token ist der Schlüssel des Agents, um seine Messungen an
   DumbMonit zu pushen. Es ist kein Konto auf der Maschine: Nichts
   dort zu erstellen. Ein aus dem Gerätedialog erstelltes Token ist
   einmalig nutzbar: Es registriert diese eine Maschine und nichts
   sonst. Für eine Flotte erstellen Sie ein wiederverwendbares Token
   unter Settings → Agents.
3. Führen Sie den Installationsbefehl auf der zu überwachenden
   Maschine mit erhöhten Rechten aus (sudo auf Linux und macOS, ein
   erhöhtes PowerShell auf Windows). Er lädt den Agent herunter,
   schreibt das Token in agent.yaml und startet den Dienst.
4. Die Maschine erscheint von selbst innerhalb weniger Sekunden,
   nach ihrem Hostnamen benannt. Bei diesem ersten Batch gibt der
   Server dem Agent ein eigenes Secret und speichert es in
   `/etc/dumbmonit/agent-secret`, für niemand anderen lesbar: Von da
   an ist diese Maschine die einzige, die unter diesem Gerät melden
   kann. Token verloren? Settings → Agents erlaubt, es zu widerrufen
   und ein neues zu erstellen.
5. Docker: Um die Container zu sehen und DumbMonit sie neu starten
   oder aktualisieren zu lassen, muss der Agent den Docker-Socket
   erreichen. Der vom Installer registrierte Dienst kann das schon;
   führen Sie den Agent stattdessen unter einem dedizierten Benutzer
   aus, fügen Sie diesen Benutzer der Gruppe "docker" hinzu und
   starten Sie ihn neu. Restart- und Auto-Update-Richtlinien werden
   dann pro Container auf der Geräteseite gesetzt.

Zwei Dinge lohnen sich zu lesen, bevor Sie das auf mehr als eine
Maschine ausrollen:
[Binding](#binding-eine-maschine-ein-agent), was ein Gerät wirklich
zu einer Agent-Installation gehören lässt, und
[Einmalig oder Flotte](#einmalig-oder-flotte), die Wahl, die Sie beim
Erstellen eines Tokens treffen.

    ```
    usermod -aG docker dumbmonit
    ```

6. Plakar-Backups: automatisch erkannt — das Backups-Panel erscheint,
   wenn das plakar-Binary oder ein Kloset
   (~/.config/plakar/stores.yml jedes Benutzers, ~/.plakar,
   /var/lib/plakar) gefunden wird, und wird sonst nicht angezeigt.
   Setzen Sie "plakar_klosets" in agent.yaml, um eine feste Liste zu
   beobachten, oder "plakar: false", um auszusteigen.

!!! note
    Der Agent kontaktiert den Server, nie umgekehrt: Kein Port muss
    auf der überwachten Maschine geöffnet werden.

=== "Linux (systemd oder OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Das Skript ist POSIX `sh` (BusyBox und dash funktionieren), lädt
    das Binary für die Architektur der Maschine (x86_64 oder
    aarch64) vom Server, schreibt `/etc/dumbmonit/agent.yaml` (Modus
    0600) und registriert `dumbmonit-agent` bei systemd, oder bei
    OpenRC auf Alpine. Die systemd-Unit ist gehärtet
    (`ProtectSystem=strict`, `NoNewPrivileges`, `MemoryMax=128M`,
    `CPUQuota=20%`).

=== "macOS (launchd)"

    Derselbe Befehl wie auf Linux, mit `sudo` ausgeführt:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Die macOS-Binaries sind **nicht** im DumbMonit-Image: Sie zu
    bauen braucht Apples SDK, dessen Lizenz die Weiterverbreitung
    verbietet. Sie sind stattdessen an jedem
    [GitHub-Release](https://github.com/laupernoe/dumbmonit/releases/latest)
    angehängt — `dumbmonit-agent-macos-aarch64` (Apple Silicon) und
    `dumbmonit-agent-macos-x86_64` (Intel), jeweils mit seinem
    `.sha256` — und der Server leitet `/download/dumbmonit-agent-macos-…`
    dorthin weiter, sodass das Skript die zur Architektur des Macs
    passende wählt (`uname -m`). Der Server liefert diese Binaries
    nicht mit, daher trägt der Installationsbefehl keine Prüfsumme
    dafür: Fügen Sie `--sha256=macos-aarch64:<hex>` (oder
    `macos-x86_64`) mit dem Wert der `.sha256` des Releases hinzu,
    sonst lehnt der Installer den Download ab. Der Mac braucht auch
    Zugriff auf github.com.

    Ohne diesen Zugriff, oder um ein schon vorhandenes Binary zu
    installieren, laden Sie es herunter und übergeben Sie es mit
    `--bin` (vergleichen Sie es zuerst mit der `.sha256` des
    Releases: `--bin` installiert, was Sie ihm geben):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    Das Skript installiert das Binary in `/usr/local/bin` (und
    entfernt das Quarantäne-Attribut, das ein Browser-Download
    trägt, das Gatekeeper sonst den Start verweigern ließe),
    schreibt `/usr/local/etc/dumbmonit/agent.yaml` (Modus 0600) und
    registriert den Systemdaemon `com.dumbmonit.agent` in
    `/Library/LaunchDaemons`, mit `launchctl bootstrap system`
    gestartet. Er läuft beim Boot, ohne dass jemand eingeloggt ist.

    !!! note "Lieber selbst bauen?"
        Auf dem Mac selbst, mit installiertem Rust:
        `cargo build --release -p dumbmonit-agent`, dann
        `--bin=target/release/dumbmonit-agent`. Das ist dasselbe
        Binary, das der Release-Job erzeugt.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Nur x86_64. Das Skript lädt `dumbmonit-agent-freebsd-x86_64` vom
    Server, schreibt `/usr/local/etc/dumbmonit/agent.yaml` (Modus
    0600) und installiert `/usr/local/etc/rc.d/dumbmonit_agent`, mit
    `service dumbmonit_agent enable` aktiviert. `daemon(8)`
    überwacht den Agent, startet ihn neu, falls er stoppt, und
    schreibt `/var/log/dumbmonit-agent.log`.

    Funktioniert genauso auf TrueNAS CORE und anderen
    FreeBSD-basierten Appliances, wo die ZFS-Zeitreihen der Sinn der
    Übung sind.

=== "Windows (Dienst)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    Das Skript lädt `dumbmonit-agent-windows-x86_64.exe` vom Server,
    prüft es gegen seinen SHA-256, installiert es als
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, schreibt
    `C:\ProgramData\DumbMonit\agent.yaml` (nur von SYSTEM und
    Administrators lesbar) und registriert den Windows-Dienst
    `DumbMonitAgent` ("DumbMonit system agent"): automatisch beim
    Boot als LocalSystem gestartet, und nach einem Fehlschlag neu
    gestartet (10 s, 30 s, dann 60 s). Der Agent spricht mit dem
    Dienstmanager selbst (`--service`), daher lässt ihn das Stoppen
    des Dienstes senden, was er noch hält, bevor er beendet.

    Ein Dienst hat keine Konsole: Der Agent schreibt sein Log nach
    `C:\ProgramData\DumbMonit\agent.log`, über 10 MB als
    `agent.log.old` zur Seite gelegt.

    Es gibt keinen nativen ARM64-Build: Auf Windows 11 für ARM
    installiert das Skript das x86_64-Binary, das unter
    x64-Emulation läuft.

Ein Token kann mehrere Maschinen registrieren: Benennen Sie es nach
einer Gruppe oder einer Maschine.

### Native Binaries

Der Agent ist eine einzelne statische ausführbare Datei pro System,
mit nichts daneben zu installieren:

| System | Datei | Wo zu bekommen |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | der Server, `/download/…` |
| Linux aarch64 (64-Bit-ARM: Raspberry Pi OS 64-Bit, ARM-Server) | `dumbmonit-agent-linux-aarch64` | der Server, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | der Server, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | der Server, `/download/…` |
| macOS Apple Silicon | `dumbmonit-agent-macos-aarch64` | das GitHub-Release (der Server leitet dorthin weiter) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | das GitHub-Release (der Server leitet dorthin weiter) |

Die Linux-Binaries sind statisch gegen musl gelinkt: Sie laufen auf
jeder Distribution, glibc oder nicht (Alpine, NAS-Firmwares), ohne
passende Bibliothek. Jede Datei hat ihre Prüfsumme daneben,
`<file>.sha256`, im `sha256sum`-Format. Jedes
[GitHub-Release](https://github.com/laupernoe/dumbmonit/releases/latest)
trägt alle sechs Binaries und ihre Prüfsummen, für Maschinen, die
`/download/` des Servers nicht erreichen können, oder zum
Installieren mit `--bin`.

Um die Linux-, Windows- und FreeBSD-Binaries selbst zu bauen, aus
einem Klon des Repositorys (Docker ist die einzige Anforderung):

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Den Dienst verwalten

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Status | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Logs | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Neustart | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Stopp | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Konfiguration | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| Dienstdatei | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | Dienst `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` auf Windows) entfernt alles davon,
Dienst eingeschlossen, auf jedem System.

!!! note "Der Download wird verifiziert — immer"
    Der von DumbMonit gezeigte Installationsbefehl trägt den
    erwarteten SHA-256 jedes vom Server gelieferten Agent-Binaries
    (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` auf Windows). Beide Installer prüfen
    die heruntergeladene Datei dagegen, bevor sie irgendetwas
    installieren, und **lehnen ab**, wenn sie nicht passt, wenn
    keine Prüfsumme für die Plattform gegeben ist, oder wenn kein
    SHA-256-Tool (`sha256sum`, `shasum`, `sha256`) verfügbar ist.

    Die Prüfsummen ändern sich mit jedem Server-Release: Kopieren
    Sie nach einem DumbMonit-Upgrade einen frischen
    Installationsbefehl aus **Settings → Agents** (oder von einem
    neuen Token), statt einen alten erneut auszuführen, der mit
    "checksum mismatch" stoppen würde. Dieselben Prüfsummen werden
    unter dem Installationsbefehl angezeigt, und unter
    `/download/<file>.sha256` veröffentlicht.

    Für Air-Gapped- oder manuelle Setups installiert
    `--insecure-skip-checksum` (`-InsecureSkipChecksum`) ein
    heruntergeladenes Binary unverifiziert und sagt das laut;
    `--bin=PATH` installiert eine lokale Datei unverändert, wobei
    die Datei Ihnen zu verifizieren überlassen bleibt.

### Installer-Flags

| `install.sh` | `install.ps1` | Wirkung |
|---|---|---|
| `--token=TOKEN` | `-Token` | Enrollment-Token (erforderlich). |
| `--url=URL` | `-Url` | Server-URL, zum Beispiel `http://server:8080`. Auf Linux wird `DUMBMONIT_URL` in der Umgebung genutzt, wenn das Flag fehlt. |
| `--interval=N` | `-Interval` | Sampling-Periode in Sekunden (standardmäßig 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Dienste, deren Zustand gemeldet wird, so benannt wie das Host-System sie nennt: systemd-Units auf Linux, launchd-Labels (`com.apple.sshd`) auf macOS, rc.d-Namen auf FreeBSD, Windows-Dienste. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Tags, auf jeder Zeitreihe als `tag_<key>` kopiert. |
| `--hostname=NAME` | `-HostName` | Dem Server angekündigter Name (Standard: der der Maschine). |
| `--bin=PATH` | `-BinPath` | Lokales Binary, statt es herunterzuladen, zu installieren. |
| `--sha256=LIST` | `-Sha256` | Erwarteter SHA-256 des Downloads, als `platform-arch:hex`-Paare, kommagetrennt (ein blanker Hex-Wert funktioniert auch). Im von DumbMonit gezeigten Befehl eingeschlossen; erforderlich, sofern nicht das nächste Flag gegeben wird. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Ein heruntergeladenes Binary ohne Verifizierung installieren, mit einer Warnung. Nur für Air-Gapped- oder manuelle Setups. |
| `--no-start` | | Alles installieren, aber den Server nicht kontaktieren oder den Dienst starten (Maschinen-Image, Testen). |
| `--uninstall` | `-Uninstall` | Den Agent deinstallieren und seine Konfiguration löschen. |
| `--help` | | Die Hilfe zeigen. |

### Upgrade

Beide Skripte sind idempotent: Den Installationsbefehl mit demselben
Token und derselben URL erneut auszuführen, ersetzt das Binary,
schreibt die Konfiguration neu und startet den Dienst neu. Das ist
die Upgrade-Prozedur. Nehmen Sie den Befehl vom aktuellen Server
(Settings → Agents): Sein `--sha256` muss zu den Binaries der
Server-Version passen, auf die Sie aktualisiert haben.

Derselbe Befehl migriert eine noch mit dem EzyMonit-Agent laufende
Maschine, auf Linux und auf Windows. Der Installer stoppt und
entfernt den alten Dienst (`ezymonit-agent`, oder `EzyMonitAgent`
auf Windows), verschiebt die Konfiguration zum neuen Pfad
(`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`)
und installiert `dumbmonit-agent` an seiner Stelle. Das alte Token
`ezym_…` bleibt gültig, daher funktioniert der Befehl, der bei der
Token-Erstellung gezeigt wurde, immer noch; nichts muss zuerst
widerrufen werden.

## Konfigurationsdatei {#configuration-file}

`/etc/dumbmonit/agent.yaml` auf Linux,
`/usr/local/etc/dumbmonit/agent.yaml` auf macOS und FreeBSD,
`C:\ProgramData\DumbMonit\agent.yaml` auf Windows:

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # sampling period
hostname: nas-basement     # optional: name announced to the server
services:                  # systemd units or Windows services
  - sshd
  - docker
tags:                      # free labels, prefixed "tag_" on the server
  role: nas
  room: basement
docker: true               # container inventory (default: true)
docker_socket: /var/run/docker.sock
sensors: true              # temperatures and fans (default: true)
smart: true                # disk health through smartctl (default: true)
smart_bin: smartctl        # path, when it is not in the service's PATH
smart_interval_secs: 300   # minimum 60: a sleeping disk is never woken to be read
zfs: true                  # ZFS pools through zpool (default: true)
zfs_bin: zpool
zfs_interval_secs: 120     # minimum 30
wireguard: true            # WireGuard tunnels through wg (default: true), see below
restic_repos: []           # restic repositories to watch, see "restic and Borg backups"
borg_repos: []             # Borg repositories to watch
mdaemon: auto              # Windows: MDaemon queues when its service exists (true / false / auto)
hyperv: auto               # Windows: Hyper-V counters when the vmms service exists (true / false / auto)
perf_counters:             # Windows: performance counters to publish
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # value of the "counter" label; the path when omitted
max_buffered_samples: 20000
log_level: info
```

`sensors`, `smart` und `zfs` werden erkannt, nie angenommen: Bei
`true` gelassen auf einer Maschine ohne Sonde, ohne `smartctl` oder
ohne Pool, kosten sie einen Blick pro Periode und veröffentlichen
überhaupt nichts. Setzen Sie eines auf `false`, um nicht einmal mehr
nachzusehen — lohnend auf einem NAS, dessen Platten Sie lieber nie
von `smartctl` berühren lassen wollen, obwohl der Agent schon
`-n standby` übergibt, sodass eine schlafende Platte nur gemessen
wird, wenn sie aus anderen Gründen wach ist.

Daneben hält `/etc/dumbmonit/agent-secret`
(`C:\ProgramData\DumbMonit\agent-secret` auf Windows) das
Binding-Secret, das der Server dieser Maschine gegeben hat. Der
Agent schreibt es mit Modus `0600` und liest es bei jedem Start neu;
es ist nicht Teil von `agent.yaml`, weil es nichts ist, das Sie
setzen — siehe [Binding](#binding-eine-maschine-ein-agent).
`DUMBMONIT_AGENT_SECRET_FILE` verschiebt es, was Sie für einen Agent
in einem Container wollen.

Jeder Schlüssel kann von der Umgebung überschrieben werden, was den
Agent in einem Container nutzbar macht, ohne eine Datei einzuhängen:

| Variable | Wirkung |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Pfad der Konfigurationsdatei |
| `DUMBMONIT_AGENT_URL` | Server-URL |
| `DUMBMONIT_AGENT_TOKEN` | Enrollment-Token |
| `DUMBMONIT_AGENT_SECRET_FILE` | Wo das Binding-Secret gehalten wird (Standard: `agent-secret`, neben der Konfigurationsdatei) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Sampling-Periode |
| `DUMBMONIT_AGENT_HOSTNAME` | Dem Server angekündigter Name |
| `DUMBMONIT_AGENT_SERVICES` | Zu beobachtende Dienste, kommagetrennt |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, kommagetrennt |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Pfad des Docker-Sockets |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Pro Host detailliert betrachtete Container, standardmäßig `200` (`0`: nur Zählungen) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Ausgelassene Schnittstellen, kommagetrennte Namen oder Regexe; Standard `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Zu behaltende Schnittstellen; wenn gesetzt, ersetzt die Ignore-Liste |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Aus Dateisystemen und Disk-I/O ausgelassene Einhängepunkte; Standard überspringt `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true`, um auch eine CPU-Zeitreihe pro Kern zu senden (Standard `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: Temperaturen und Lüfter (Standard `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: Plattengesundheit über `smartctl` (Standard `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Pfad von `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Periode zwischen zwei SMART-Lesungen, Standard `300`, Minimum `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: ZFS-Pools über `zpool` (Standard `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Pfad von `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Periode zwischen zwei ZFS-Lesungen, Standard `120`, Minimum `30` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: WireGuard-Tunnel über `wg` (Standard `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Pfad von `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Pfad von `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Pfad von `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Periode zwischen zwei Lesungen der restic- und Borg-Repositories, Standard `600`, Minimum `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` oder `auto`: MDaemons Leistungszähler (Standard `auto`, gelesen, wenn der Windows-Dienst `MDaemon` existiert) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` oder `auto`: Hyper-Vs Leistungszähler (Standard `auto`, gelesen, wenn der Windows-Dienst `vmms` existiert) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Pfade der Windows-Leistungszähler, durch **Semikolons** getrennt (ein Komma kann Teil eines Instanznamens sein); ersetzt `perf_counters` |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Größe des Nachholpuffers |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true`, um für den Server die Abfragen der diesem Agent zugewiesenen Geräte auszuführen (siehe [Den Agent in Docker / einem anderen Netzwerk betreiben](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Standort-Label, neben dem Agent gezeigt, wenn er als Relay angeboten wird |

## Windows-Leistungszähler {#windows-performance-counters}

Auf Windows liest der Agent Leistungszähler — die Werte, die der
Leistungsindikator (`perfmon`) zeigt — über die PDH-Bibliothek, und
veröffentlicht sie bei jeder Sampling-Periode. Manche Software
veröffentlicht ihren Zustand nirgendwo sonst.

**MDaemon.** Wenn der Windows-Dienst namens `MDaemon` existiert,
liest der Agent das MDaemon-Leistungsobjekt: Warteschlangengrößen,
Sitzungen, Nachrichten- und Spam-Totals, den Zustand der internen
Server von MDaemon und seine Uptime. Die Zeitreihen, die Regeln und
die Seite, die sie zeigt, sind auf der
[MDaemon](mdaemon.md#mail-queues-through-the-agent)-Seite
beschrieben. `mdaemon: true` liest sie auch ohne den Dienst (ein
umbenannter Dienst), `mdaemon: false` nie. Das Vorhandensein des
Dienstes wird alle zehn Perioden erneut geprüft, daher wird ein nach
dem Agent installiertes MDaemon ohne Neustart erkannt.

**Hyper-V.** Wenn der Hyper-V-Virtual-Machine-Management-Dienst
(`vmms`) existiert, liest der Agent Hyper-Vs Leistungsobjekte; siehe
[Hyper-V-Hosts](#hyper-v-hosts). `hyperv: true` und `hyperv: false`
erzwingen die Wahl, wie bei MDaemon.

**Jeder andere Zähler.** Listen Sie seinen vollen Pfad unter
`perf_counters`, als `\Object(Instance)\Counter` oder
`\Object\Counter` für ein Objekt mit einer einzelnen Instanz, und er
wird als `dumbmonit_agent_perf_counter{counter="…"}` veröffentlicht,
mit dem von Ihnen gegebenen Namen oder dem Pfad selbst:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Schreiben Sie die Zählernamen **auf Englisch**, wie sie im
  Leistungsindikator auf einem englischen Windows erscheinen: Der
  Agent fügt sie mit `PdhAddEnglishCounterW` hinzu, daher
  funktioniert dieselbe `agent.yaml` auf einem französischen oder
  deutschen Windows.
- Setzen Sie jeden Pfad in einfache Anführungszeichen in YAML,
  sodass die Backslashes und das `%` unverändert bleiben.
- Ein fehlerhafter Pfad stoppt den Agent beim Start, mit dem
  Schlüssel und dem Grund. Ein Pfad, den Windows nicht kennt (die
  Software ist nicht installiert, der Name ist falsch geschrieben),
  wird einmal im Log gemeldet, `performance counter not available`,
  und die anderen Zähler werden weiter gelesen; er wird alle zehn
  Perioden erneut versucht.
- Raten (`…/sec`) brauchen zwei Lesungen: Sie erscheinen ab der
  zweiten Periode.
- Höchstens 200 Zähler werden gelesen. Auf Linux, macOS und FreeBSD
  wird die Einstellung akzeptiert, und nichts wird gelesen.

Um die genauen Namen auf der Maschine zu sehen, führen Sie
`typeperf -q MDaemon` (oder jeden anderen Objektnamen) in einer
Eingabeaufforderung aus.

## Hyper-V-Hosts {#hyper-v-hosts}

Auf einem Hyper-V-Host installiert — Windows Server mit der
Hyper-V-Rolle, oder Windows mit aktiviertem Hyper-V — findet der
Agent den `vmms`-Dienst und liest Hyper-Vs eigene Leistungszähler,
ohne etwas zu konfigurieren. Die Geräteseite des Agents zeigt dann
ein Hyper-V-Panel: wie viele virtuelle Maschinen sich gesund melden,
die echte CPU-Last des Hosts, der Speicher, den jede laufende VM
hält, und der Durchsatz, die Latenz und Fehler jeder virtuellen
Platte.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Die Zählernamen stammen aus Microsofts Hyper-V-Leistungs-
    dokumentation und wurden gegen die Logik mit einer simulierten
    Zählerquelle geprüft, nicht auf einem laufenden Hyper-V-Host.
    Ein Windows unbekannter Zähler wird einmal im Log des Agents
    gemeldet, und die anderen werden weiter gelesen. Sagen Sie uns,
    was kaputt ist.

| Zähler | Zeitreihe | Labels |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: virtuelle Maschinen in jedem Zustand | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: die Last des Hosts, Gäste eingeschlossen | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: der für laufende Gäste verbrachte Anteil | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: laufende VMs plus der Host selbst | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: Speicherbedarf über zugewiesenem Speicher; über 100 will die VM mehr, als sie hat | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (Durchschnitt), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, ein Zähler | `disk` |

Auf einem Hyper-V-Host misst `\Processor(_Total)\% Processor Time`
nur die eigene Partition des Hosts, nicht, was die virtuellen
Maschinen verbrauchen: Der Zähler für logische Prozessoren des
Hypervisors ist der, der sagt, ob die Maschine gesättigt ist. `vm`
ist der Name der virtuellen Maschine, wie der Hyper-V-Manager ihn
zeigt; `disk` ist der Instanzname, den Windows jeder virtuellen
Platte gibt, von ihrem Dateipfad abgeleitet. Die
`_Total`-Instanzen werden ausgelassen, und höchstens 200 VMs oder
Platten werden pro Zähler gelesen.

Drei [eingebaute Regeln](../alerting/rules.md#hyper-v) gelten:

- **Hyper-V VM health critical**: Hyper-V meldet eine VM seit fünf
  Minuten in kritischer Gesundheit (Warning) — meist eine Platte,
  der der Platz ausging.
- **Hyper-V host CPU high**: die logischen Prozessoren seit
  fünfzehn Minuten mehr als 90 % beschäftigt (Advisory).
- **Hyper-V virtual disk errors**: eine virtuelle Platte meldete in
  der letzten Stunde I/O-Fehler (Advisory).

## Den Agent in Docker / einem anderen Netzwerk betreiben {#run-the-agent-in-docker--on-another-network}

Der Agent wird auch als Container-Image veröffentlicht, mit
denselben Tags wie der Server: `ghcr.io/laupernoe/dumbmonit-agent:latest`
(letztes Release), `:edge` (letzter Commit auf `main`). Es ist ein
`FROM scratch`-Image, das das Agent-Binary und das Mozilla-CA-Bundle
hält, vollständig über Umgebungsvariablen konfiguriert. Das
Repository liefert eine fertige `docker-compose.agent.yml`:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

Was der Container sieht, und was einzuhängen ist:

| Bedarf | Was zu tun ist | Warum |
|---|---|---|
| CPU, Speicher, Last, Disk-I/O des Hosts | Nichts | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` und `/proc/diskstats` sind hostweit innerhalb jedes Containers. |
| Hostname | `DUMBMONIT_AGENT_HOSTNAME` | Der Hostname eines Containers ist eine zufällige ID; das Gerät würde danach benannt. |
| Container des Hosts | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Optional. Nur-lesendes Listen und Inspizieren; Restart-/Update-Aktionen brauchen ihn schreibbar und `DUMBMONIT_AGENT_COMMANDS=true`. Ohne den Socket meldet der Agent einfach keine Container. |
| Dateisystemnutzung | `- /:/host:ro` (oder die Mounts, die Sie interessieren) | Der Agent meldet die *innerhalb* des Containers sichtbaren Mounts, unter ihrem Container-Pfad. |
| Host-Netzwerkzähler | `network_mode: host` | Schnittstellen sind Namespace-getrennt: ohne das sieht der Agent nur ein `veth`. |
| Über diesen Agent relayte Ping-Monitore | `cap_add: [NET_RAW]` | ICMP braucht einen Raw-Socket. |
| Private CA | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Server hinter einem internen Reverse Proxy, Geräte mit selbstsignierten Zertifikaten. |
| Das Binding über Neuerstellungen hinweg behalten | `- agent-state:/var/lib/dumbmonit-agent` und `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | Das Binding-Secret lebt im Dateisystem des Containers, das ein `docker compose up --force-recreate` wegwirft. Ohne das Volume kommt der Agent ungebunden zurück, und jemand muss jedes Mal die Neu-Registrierung von der Geräteseite erlauben. |

`pid: host` und `privileged: true` werden nie gebraucht: Der Agent
sammelt keine Pro-Prozess-Metriken.

### Relay-Modus: einen anderen Standort überwachen

Ein Agent kann mehr als seine eigene Maschine beobachten. Mit
`relay: true` (oder `DUMBMONIT_AGENT_RELAY=true`) **delegiert** der
Server ihm **Abfragen**: Geräte, deren Einstellung *Reached
through* diesen Agent benennt, werden nicht vom Server abgefragt,
sondern vom Agent, von seinem eigenen Netzwerk aus, mit genau
denselben Collectors — SNMP, Proxmox VE, Proxmox Backup Server,
Synology, HTTP, TCP, DNS, Ping, TLS. Die Ergebnisse landen unter dem
Gerät, als hätte der Server es abgefragt: dieselben Metriken,
dieselbe *letzte Abfrage*, dieselben Alarme. Ein entferntes Büro,
ein zweites Homelab oder das Netzwerk eines Kunden zeigen sich so in
dem einen DumbMonit, das Sie schon haben, ohne VPN und ohne etwas
auf der entfernten Seite zu öffnen.

Wie Sie das Schritt für Schritt einrichten, steht in
[Einen entfernten Standort überwachen](../install/remote-site.md).

## Binding: eine Maschine, ein Agent {#binding-eine-maschine-ein-agent}

Ein Enrollment-Token sagt *diese Maschine darf sprechen*. Es sagt
nicht, *welche* Maschine spricht — dafür ist der Identitätsschlüssel
(`/etc/machine-id`, oder der Hostname), und dieser Schlüssel ist
kein Secret: Er kann auf jeder Maschine innerhalb von Sekunden
gelesen werden.

Daher gibt der Server beim ersten Batch jeder Maschine ein eigenes
**Binding-Secret**. Er behält nur einen Fingerprint; der Agent
schreibt das Secret nach `/etc/dumbmonit/agent-secret` mit Modus
`0600` und präsentiert es danach im Header
`X-DumbMonit-Agent-Secret` bei jedem Request — Messungen,
Container-Befehle und relayte Abfragen gleichermaßen. Von da an:

- kann keine andere Maschine Messungen unter diesem Gerät pushen,
  oder seinen Hostnamen und Betriebssystem umschreiben;
- kann keine andere Maschine die für sie eingereihten Restart- und
  Update-Befehle abholen — und daher verbrauchen.

Die Geräteseite zeigt, wo eine Maschine steht, unter **Agent →
Binding**:

| Zustand | Was es bedeutet | Was zu tun ist |
|---|---|---|
| **Bound** | Der Agent hält ein eigenes Secret. | Nichts. |
| **Not bound — re-enrol this host** | Ein Host, der vor der Existenz von Binding registriert wurde, und seitdem nie gebunden wurde. Der Server lehnt seine Messungen, Container-Befehle und relayten Abfragen ab. | Klicken Sie **Allow re-enrolment** auf der Geräteseite, starten Sie dann den Agent neu. Stammt der Agent von vor Binding, führen Sie zuerst den Installationsbefehl erneut auf dem Host aus. |

### Hosts von vor der Existenz von Binding

!!! warning "Geändert: ungebundene Hosts werden abgelehnt"
    Frühere Releases akzeptierten weiterhin Hosts, die vor der
    Existenz von Binding registriert wurden, solange sie das Token
    nutzten, das sie registriert hatte, und banden sie leise, sobald
    ihr Agent aktualisiert wurde. Dieses Übergangsfenster ist
    geschlossen: Ein Host, der nie gebunden wurde, wird jetzt **auf
    jeder Agent-Route abgelehnt** — Messungen, Container-Befehle und
    relayte Abfragen — bis ein Administrator ihn neu registriert.
    Ein Flotten-Token wird von vielen Maschinen geteilt, und der
    Identitätsschlüssel kann erraten werden, daher beweist nichts
    sonst, dass der anfragende Agent der eigene des Hosts ist.

    Ein für Binding zu altes Agent-Binary wird ebenfalls abgelehnt,
    einschließlich bei seiner ersten Registrierung.

Um einen solchen Host zurückzubringen:

1. Stammt sein Agent von vor Binding, führen Sie den
   Installationsbefehl erneut darauf aus (das Agent-Binary muss ein
   Binding-Secret speichern können).
2. Öffnen Sie seine Geräteseite und klicken Sie **Agent → Allow
   re-enrolment**. Das öffnet ein einstündiges Fenster.
3. Starten Sie den Agent neu, oder warten Sie auf seinen nächsten
   Versuch: Dieser Batch bindet den Host mit einem frischen Secret
   und schließt das Fenster. Ein anderes Token als das, das den
   Host registrierte, muss weiterhin registrieren können (nicht
   aufgebraucht, nicht abgelaufen).

Bis dahin trägt das Log des Agents die Erklärung des Servers:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Neu-Registrierung nach einer Neuinstallation

Wird die Maschine neu aufgesetzt, ihre Platte ersetzt, oder ihr
Agent-Container ohne das Volume mit dem Secret neu erstellt, kommt
der Agent ohne es zurück, und der Server lehnt seine Batches ab.
Diese Ablehnung ist Absicht: Eine gebundene Maschine bleibt
gebunden, bis ein Mensch anders sagt, sonst würde das Binding nichts
schützen.

Der Weg zurück liegt auf der Geräteseite: **Agent → Allow
re-enrolment**. Es öffnet ein einstündiges Fenster, während dem ein
Agent, der ein gültiges Enrollment-Token präsentiert, diese Maschine
mit einem frischen Secret erneut bindet; das alte Secret funktioniert
nicht mehr, und das Fenster schließt sich, sobald es genutzt wird.
Der Agent versucht es in der Zwischenzeit selbst erneut, daher ist
auf der Maschine nichts neu zu starten — und nichts geht verloren,
da seine Messwerte gepuffert bleiben, bis der Link wieder akzeptiert
wird.

Bis jemand dieses Fenster öffnet, sagt das Log des Agents, was zu
tun ist:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Enrollment-Tokens

- Tokens werden unter **Settings → Agents** oder beim Hinzufügen
  eines Geräts der Art agent erstellt. Der Server behält nur einen
  Fingerprint: Das Klartext-Token wird einmal gezeigt, in der
  Erstellungsantwort, mit den zwei Installationsbefehlen.
- Der Agent sendet es als `Authorization: Bearer dmon_…` auf
  `POST /api/ingest`. Es erscheint nie in den Logs des Agents, nicht
  einmal abgeschnitten.
- **Settings → Agents** listet Tokens mit ihrem Präfix, Scope,
  Erstellungsdatum und letzter Nutzung. **Revoke** stoppt jeden
  Agent, der dieses Token nutzt, bei seinem nächsten Push; die
  Maschinen werden dann nach drei verpassten Perioden *unreachable*.
  Führen Sie den Installer mit einem neuen Token erneut aus, um sie
  neu zu registrieren.
- Maschinen sind getrennte Geräte: Ein Token zu widerrufen löscht
  sie nicht.

### Einmalig oder Flotte {#einmalig-oder-flotte}

Der Scope eines Tokens wird bei seiner Erstellung gewählt, und es
ist eine Wahl, kein Standard, der Ihnen passiert:

| Scope | Was es registriert | Wann |
|---|---|---|
| **Single use** (der Standard) | Genau eine Maschine. | Eine Installation. Ein Installationsbefehl lebt in der Shell-Historie weiter, in einem Ticket, in einem Chat-Log; ein einmaliger ist nichts mehr wert, sobald er genutzt ist. |
| **Reusable for a fleet** | So viele Maschinen, wie Sie erlauben — eine Zahl, oder kein Limit. | Ein Playbook, ein Maschinen-Image, ein Stapel Installationen. |

Beide können eine Frist in Tagen tragen. Eine Frist stoppt nur
*Registrierungen*: schon registrierte Maschinen melden nach ihrem
Ablauf weiter, was Sie für ein Installationsfenster wollen.
Widerrufen ist, was eine Flotte abschneidet.

Ein Token, das nicht mehr registrieren kann — aufgebraucht, oder
über seine Frist hinaus — wird in Settings → Agents als **Enrols no
more** gezeigt. Seine Maschinen melden weiter; es kann nur keine
neue hereinlassen.

## Ausfälle und Diagnose

Wenn der Server nicht erreichbar ist, misst der Agent weiter und
puffert seine Messwerte im Speicher mit ihren ursprünglichen
Zeitstempeln. Sie werden gesendet, sobald der Link zurück ist: Ein
Server-Neustart hinterlässt kein Loch in den Diagrammen. Der Puffer
ist begrenzt (`max_buffered_samples`, standardmäßig etwa eine
Stunde) und verwirft die ältesten Messwerte, wenn er voll ist.
Wiederholungsversuche steigen von 5 Sekunden bis 5 Minuten mit
Jitter.

```sh
dumbmonit-agent --dry-run          # print the readings, send nothing
dumbmonit-agent --once             # send one batch then exit
journalctl -u dumbmonit-agent -f   # service log (systemd)
tail -f /var/log/dumbmonit-agent.log   # service log (OpenRC)
```

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| Installer sagt "could not reach the server" | Falsche URL oder Token. Die Konfiguration wird geschrieben: Korrigieren Sie `agent.yaml`, starten Sie dann den Dienst neu. |
| Download schlägt mit 404 fehl | Der Server hat keine Agent-Binaries (`DUMBMONIT_AGENT_DIR` leer). Nutzen Sie `--bin=PATH` mit einem selbst gebauten Binary. |
| Installer stoppt mit "checksum mismatch" | Der Befehl wurde vor einem Server-Upgrade kopiert (kopieren Sie einen frischen aus Settings → Agents), oder das empfangene Binary ist nicht das, was der Server liefert: ein Proxy oder Cache im Weg, oder ein manipulierter Download. |
| Installer stoppt mit "no expected checksum" | Der Befehl hat kein `--sha256` für diese Plattform: ein alter Befehl, oder ein macOS-Binary (vom Server nicht mitgeliefert). Kopieren Sie einen frischen Befehl, oder übergeben Sie `--sha256=<platform>-<arch>:<hex>` aus der `.sha256` des Releases. |
| Maschine erscheint nie | Der Push geht an `/api/ingest` auf der URL im Installationsbefehl; stellen Sie hinter einem Reverse Proxy sicher, dass sie weitergeleitet wird und Bodies bis 16 MB erlaubt sind. |
| *Unreachable*, obwohl der Agent läuft | Das Token wurde widerrufen, oder die Pushes werden abgelehnt. Prüfen Sie `journalctl -u dumbmonit-agent`: Der Fehler wird dort protokolliert. |
| Log sagt "this agent is not recognised for this machine" | Die Maschine ist an eine Agent-Installation gebunden, deren Secret diese nicht hat — eine Neuinstallation, oder ein ohne sein State-Volume neu erstellter Container. Öffnen Sie die Geräteseite und klicken Sie **Allow re-enrolment**; siehe [Neu-Registrierung nach einer Neuinstallation](#neu-registrierung-nach-einer-neuinstallation). |
| Log sagt "this machine was enrolled before agent binding" | Ein Host, der nie an seinen Agent gebunden wurde. Klicken Sie **Allow re-enrolment** auf seiner Geräteseite, starten Sie dann den Agent neu; siehe [Hosts von vor der Existenz von Binding](#hosts-von-vor-der-existenz-von-binding). |
| Log sagt "this agent is too old to be bound to its machine" | Das Agent-Binary kann kein Binding-Secret speichern. Führen Sie den Installationsbefehl erneut auf dem Host aus; existiert das Gerät schon, klicken Sie zuerst **Allow re-enrolment**. |
| Log sagt "this enrollment token has already enrolled all the machines it was allowed to" | Ein wiederverwendetes Single-Use-Token. Erstellen Sie ein neues, oder ein wiederverwendbares Token unter Settings → Agents. |
| Windows | Die Windows-Dienst-Teile waren zum Zeitpunkt des Schreibens im Build-Image des Projekts nicht geübt; melden Sie Probleme auf GitHub. |

## Docker-Container

Wenn der Agent den Docker-Socket lesen kann (`docker: true`, der
Standard, und ein Benutzer in der Gruppe `docker` oder root —
`usermod -aG docker dumbmonit`, dann den Agent neu starten), wird
jeder Container zu einer Handvoll Zeitreihen, mit `container` und
`image` beschriftet (das `name:tag`, das `docker ps` zeigt). Sagt
die Geräteseite "No Docker on this machine", während Docker
installiert ist, ist das als Erstes zu prüfen.

| Metrik | Bedeutung |
|---|---|
| `container_up` | 1 laufend, sonst 0 |
| `container_health` | 0 kein Healthcheck, 1 healthy, 2 unhealthy, 3 starting |
| `container_restart_count` | Dockers Neustart-Zähler für den Container |
| `container_started_seconds` | Sekunden seit dem letzten Start des Containers (0, wenn gestoppt) |
| `container_image_age_seconds` | Sekunden seit dem Bau des Images |
| `container_update_available` | 1, wenn die Registry ein neueres Image für denselben Tag hat, 0, wenn aktuell, -1 unbekannt |

Der Update-Check vergleicht den Digest des lokalen Images mit dem,
den die Registry für den Tag ankündigt (`HEAD /v2/<name>/manifests/<tag>`,
anonymer Token-Flow auf Docker Hub und GHCR). Er läuft im
Hintergrund höchstens einmal pro Stunde pro Image und verzögert nie
den 30-Sekunden-Zyklus; eine private Registry, die anonymen Zugriff
abweist, liest sich einfach als -1. Deaktivieren Sie ihn mit
`docker_update_check: false` (`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Eingebaute Regeln: **Container stopped** (`container_up == 0` seit
2 Minuten), **Container unhealthy** (`container_health == 2` seit
3 Minuten), **Container restarting** (drei oder mehr Neustarts in
15 Minuten), **Container update available** (Info, nach einer
Stunde, täglich wiederholt). Der Alarm benennt den Container.

### Aktionen

Auf der Geräteseite hat jeder Container zwei Aktionen und zwei
Schalter. Aktionen reisen durch einen **Befehlskanal**: Der Server
reiht den Befehl ein, der Agent holt ihn nach seinem nächsten Batch
ab (`GET /api/agent/commands`, dasselbe Enrollment-Token), führt ihn
aus und meldet das Ergebnis (`queued → running → done | failed`),
mit einem kurzen Log, das Sie in der Oberfläche aufklappen können.
Befehle älter als zehn Minuten werden nie ausgeführt; der Agent
führt einen Befehl zur Zeit aus.

Ein eingereihter Befehl kann aus der Oberfläche **abgebrochen**
werden, solange der Agent ihn nicht abgeholt hat. Ein Befehl, den
niemand abholt, **läuft** nach zehn Minuten **ab**, auf dem Server,
egal ob der Agent jemals abfragt — daher behält eine Maschine, deren
Agent gestoppt ist, nie ein "Queued" für immer, und blockiert nie
die nächste Aktion auf diesem Container.

Der Agent sagt dem Server, mit jedem Batch, ob er Befehle akzeptiert.
Tut er das nicht — ein älterer Agent als der Befehlskanal, oder mit
`commands: false` installiert — sagt die Geräteseite das, statt
Restart und Update zu zeigen, die API lehnt die Befehle ab (`409`),
und Richtlinien reihen nichts ein. Installieren Sie den Agent mit
dem aktuellen Installer neu, um Aktionen zu aktivieren.

- **Restart** — `docker restart` für einen laufenden Container,
  `docker start` für einen gestoppten, dann eine
  20-Sekunden-Prüfung, dass er läuft.
- **Update now** — zieht den Tag, den der Container schon nutzt,
  erstellt einen neuen Container mit derselben Konfiguration
  (Umgebung, Mounts, Ports, Labels, Netzwerke, Restart-Richtlinie),
  stoppt den alten, tauscht die Namen, startet den neuen und wartet
  auf seinen Healthcheck (oder 20 Sekunden plus eine Lauf-Prüfung,
  wenn das Image keinen hat). Schlägt etwas fehl, **rollt der Agent
  zurück**: Der neue Container wird entfernt und der alte neu
  gestartet, und der Befehl meldet `failed` mit dem Grund. Bei
  Erfolg wird der alte Container entfernt und, wenn "prune" an ist,
  das alte Image auch — sofern kein anderer Container es noch nutzt.

Compose-verwaltete Container (Labels `com.docker.compose.*`) werden
genauso aktualisiert, aber das Ergebnis warnt, dass der nächste
`docker compose up` den Container aus der Compose-Datei neu
erstellen wird. Container, deren Name mit `dumbmonit` oder
`dumbmonit` beginnt, werden abgelehnt: Der Monitor aktualisiert sich
nie selbst.

### Richtlinien

Eine Pro-Container-**Richtlinie** wird vom Server jede Minute
ausgewertet:

- **Restart if down** — ein Container, der `container_up == 0`
  meldet, wird automatisch neu gestartet, höchstens einmal alle
  zehn Minuten.
- **Auto-update in maintenance windows** — ein Container mit
  verfügbarem Update wird automatisch aktualisiert, aber nur,
  während ein Wartungsfenster (Silence) das Gerät abdeckt,
  höchstens einmal alle sechs Stunden. Planen Sie eines auf der
  Alerts-Seite, oder schalten Sie das Gerät von seiner Seite
  stumm. Richtlinien bereinigen standardmäßig das alte Image und
  nur innerhalb von Wartungsfenstern.

Zwei Orte setzen sie:

- der **Docker-Streifen** unter dem Gerätekopf: "28 containers · 27
  running · 3 updates available · policies: 2 auto-restart, 0
  auto-update", die letzte Aktion und ihr Status, **Manage
  containers** (öffnet die Liste unten) und **Policies…**, eine
  Tabelle jedes Containers mit den zwei Schaltern und einer "Apply
  to all"-Zeile — der schnellste Weg, Auto-Restart für eine ganze
  Maschine einzuschalten;
- die Schalter innerhalb jeder Zeile des Abschnitts **Containers**,
  neben den Schaltflächen Restart und Update now.

Jeder Schalter speichert für sich
(`PUT /api/targets/<id>/containers/<name>/policy`); ein Fehlschlag
betrifft nur diese Zeile und wird daneben angezeigt.

Automatische Aktionen zeigen sich in "Recent actions" als von
`policy` angefragt.

### Sicherheitsregeln

- Der Agent handelt nur, wenn `commands: true` (der Standard);
  setzen Sie es auf `false` für eine Maschine, die nur gemessen
  werden darf.
- Der Agent holt Befehle selbst ab, mit seinem Enrollment-Token —
  nichts verbindet sich mit der Maschine. Sie laufen nach zehn
  Minuten ab und laufen eine Zeit; der Agent zieht nie einen
  anderen Tag als den, den der Container schon nutzt.
- Ein Update, das nicht gesund zurückkommt, wird auf den vorherigen
  Container zurückgerollt, und das alte Image wird nur entfernt,
  sobald der neue Container läuft.
- Container namens `dumbmonit*` oder `ezymonit*` werden abgelehnt,
  von Hand oder durch Richtlinie: Der Monitor handelt nie an seinen
  eigenen Containern. Die Prüfung erfolgt zweimal — der Server
  lehnt ab, den Befehl einzureihen, und der Agent lehnt ab, ihn
  auszuführen — und der Agent macht sie erneut am Namen, den Docker
  nach der Inspektion des Containers meldet, sodass einen bei
  seiner ID zu benennen nichts ändert. Der Agent weigert sich auch,
  an seinem eigenen Container zu handeln, wie er auch heißt: Er
  wäre nicht da, um die Arbeit zu beenden oder zu melden.
- Compose-verwaltete Container werden aktualisiert, aber der
  nächste `docker compose up` erstellt sie aus der Compose-Datei
  neu: Heben Sie den Tag auch dort an.

## Plakar-Backups

Der Agent kann [Plakar](https://plakar.io)-Klosets (Repositories)
beobachten: Alle zehn Minuten, im Hintergrund, führt er
`plakar at <kloset> ls` und `info` auf jedem aus und gibt die letzte
Lesung bei jedem Zyklus erneut aus.

Plakar wird **erkannt, nie angenommen**. Beim Start und dann bei
jedem Zyklus, bis er etwas findet, sucht der Agent nach dem
`plakar`-Binary (auf `PATH`, oder `plakar_bin`) und nach Klosets in
den Standardorten. Gibt es kein Binary, kein Kloset und keine Spur
von Plakar (`~/.cache/plakar`), gibt der Agent überhaupt keine
`backup_*`-Zeitreihe aus, sagt das einmal auf `debug`-Ebene, und die
Geräteseite zeigt kein Backups-Panel — nichts zu konfigurieren auf
Maschinen ohne Plakar. Um die Erkennung ganz zu überspringen:

```yaml
plakar: false                        # or DUMBMONIT_AGENT_PLAKAR=false
```

### Discovery

Nichts zu konfigurieren: Wenn `plakar_klosets` leer ist, findet der
Agent die Klosets selbst, in dieser Reihenfolge, und liest jeden Ort
nur einmal:

1. das eigene Konfigurationsverzeichnis des Agents —
   `$XDG_CONFIG_HOME/plakar`, oder `~/.config/plakar` des Kontos,
   unter dem er läuft (`plakar_home`, wenn gesetzt);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, alphabetisch;
4. `/var/lib/plakar`, wenn dieses Verzeichnis existiert.

In jedem Verzeichnis liest er `stores.yml` (oder `stores.yaml`; das
Legacy `plakar.yml` wird auch akzeptiert) und nimmt jeden Eintrag
der `stores`-Map (auch `repositories` und `klosets`), der eine
`location` hat. Nur der Ort wird gelesen: Passphrasen und
rclone-Secrets daneben werden nie behalten noch protokolliert. Jeder
Eintrag wird zum Kloset `@<name>`, gelesen mit `plakar -configdir
<dir> at @<name>`, daher wird ein von einem Benutzer erstelltes
Kloset gelesen, auch wenn der Agent als root läuft. `~/.plakar`
jedes Kontos wird hinzugefügt, wenn es existiert (das Standard-
Kloset von `plakar create`). Zwei Konten, die unterschiedliche
Klosets gleich benennen, werden als `@name` und `<user>:@name`
unterschieden.

Das Ergebnis wird einmal auf `info`-Ebene protokolliert (und erneut
nur, wenn es sich ändert). Zwei Gauges, nur veröffentlicht, sobald
Plakar erkannt ist, lassen die Oberfläche die übrigen Fälle
unterscheiden: `backup_plakar_present` (1, wenn das Binary läuft)
und `backup_klosets_found` (Anzahl gelesener Klosets). Die
Geräteseite sagt dann "Plakar is installed but no kloset was
found", oder listet die Klosets. Existieren Klosets, aber das
Binary läuft nicht (ein Dienst-`PATH` ohne `plakar`, typischerweise),
werden sie als nicht lesbar mit einem Hinweis gelistet, `plakar_bin`
zu setzen, und der Agent warnt einmal.

Um stattdessen eine feste Liste zu beobachten, benennen Sie sie —
Discovery wird dann übersprungen:

```yaml
plakar: true                         # default: detect Plakar; false hides it entirely
plakar_bin: /usr/bin/plakar          # default: "plakar" on PATH
plakar_klosets:                      # explicit list: overrides discovery
  - /srv/backups/main
  - "@diskext"                       # a named store of the agent's account
plakar_home: /home/noe               # optional: HOME (and config dir) used when running plakar
plakar_interval_secs: 600            # minimum 60
```

| Variable | Wirkung |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — Plakar-Backups beobachten, wenn erkannt (Standard `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Pfad des `plakar`-Binaries |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets, kommagetrennt (überschreibt Discovery) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Beim Ausführen von `plakar` genutztes Home-Verzeichnis |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Sekunden zwischen zwei Lesungen |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — Registry-Check für Container-Updates |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — Aktionen vom Server akzeptieren |

Ein verschlüsseltes Kloset braucht seine Passphrase: Legen Sie sie
in den Store-Eintrag (`plakar store set <name> passphrase=…`, was
`plakar store add` tut) oder exportieren Sie `PLAKAR_PASSPHRASE` in
der Dienstumgebung des Agents. Ohne sie liest sich das Kloset als
"Cannot be opened" (`backup_last_status = 0`).

### Metriken und Regeln

Metriken, beschriftet mit `kloset` und `source` (das gesicherte
Verzeichnis): `backup_last_success_seconds` (Alter des neuesten
Snapshots), `backup_snapshot_count`, `backup_last_status` (1, wenn
der neueste Snapshot keinen Fehler hat, 0, wenn er Fehler hat oder
das Kloset nicht geöffnet werden kann), `backup_size_bytes{kloset}`
(Storage-Größe des Repositorys), plus `backup_plakar_present` und
`backup_klosets_found`, oben beschrieben. Keine davon existiert,
solange Plakar nicht erkannt ist, daher können die eingebauten
Regeln unten auf einer Maschine ohne Plakar nicht feuern.

Eingebaute Regeln: **Plakar backup too old** (advisory, kein
Snapshot seit mehr als zwei Tagen) und **Plakar backup failed**
(warning). Die Geräteseite zeigt einen Block pro Kloset — Alter des
letzten Backups, Snapshot-Anzahl, Größe, letzter Status — und eine
Zeile pro Quelle, mit einem "Backup too old"-Hinweis nach zwei
Tagen.

### Ein erstes Backup durchführen

Mit installiertem Plakar 1.1 auf der Maschine (`plakar version`),
als der Benutzer, dem die Backups gehören werden:

```sh
# 1. a kloset store, named so the agent finds it in ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # or any location Plakar accepts
plakar at @diskext create                                   # add -plaintext to skip encryption

# 2. a first backup, then check it
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` und `plakar at <path> backup <dir>`
funktionieren genauso mit einem einfachen Verzeichnis statt
`@name`; `plakar create` ohne `at` nutzt `~/.plakar`, das der Agent
auch findet. Planen Sie `plakar at @diskext backup /srv/photos` mit
Cron oder einem systemd-Timer: Der Agent meldet das Alter des
neuesten Snapshots, und alarmiert, wenn ein Tag ohne einen vergeht —
zwei Tage für die eingebaute Regel.

Um es lokal mit einem unverschlüsselten Kloset auszuprobieren:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

fügen Sie dann `plakar_klosets: ["/tmp/plakar-demo"]` zu
`agent.yaml` hinzu (ein blanker Pfad außerhalb eines Homes wird
nicht entdeckt) und prüfen Sie mit `dumbmonit-agent --dry-run`, dass
`backup_*`-Samples erscheinen.

## restic- und Borg-Backups {#restic--und-borg-backups}

Der Agent kann [restic](https://restic.net)- und
[Borg](https://www.borgbackup.org)-Repositories beobachten: Alle
zehn Minuten, im Hintergrund, öffnet er jedes von Ihnen deklarierte
Repository und meldet, ob es gelesen werden konnte und wie alt sein
neuester Snapshot ist. Nichts wird entdeckt: Ein Repository hat
keinen Standardort und kann ohne sein Passwort nicht geöffnet
werden, daher wird jedes in `agent.yaml` gelistet, mit dem Weg, an
sein Passwort zu kommen.

**Das Passwort verlässt die Maschine nie.** Der Agent gibt es nur
über die Umgebung des Kindprozesses an `restic` oder `borg` weiter;
es wird nie protokolliert, nie an den Server gesendet, und die
Zeitreihen tragen nur den Namen des Repositorys.

```yaml
restic_repos:
  - name: nas                                   # the "repo" label; default: the repository, without credentials
    repository: sftp:backup@nas.lan:/srv/restic  # passed as RESTIC_REPOSITORY, never on the command line
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # optional: only this host's snapshots (--host), for a shared repository
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # or any command that prints it (RESTIC_PASSWORD_COMMAND)
    env:                                        # extra environment for this repository only
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # passed as BORG_REPO
    password_file: /etc/dumbmonit/borg-offsite.pass        # read by the agent, passed as BORG_PASSPHRASE
    glob_archives: "web-01-*"                   # optional: only these archives (--glob-archives)
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # default: on PATH
borg_bin: borg
backup_repos_interval_secs: 600                 # minimum 60
```

Halten Sie `agent.yaml` und die Passwortdateien nur für root lesbar
(`chmod 600`), wie es der Installer für `agent.yaml` tut.
`password_command` wird zu `RESTIC_PASSWORD_COMMAND` oder
`BORG_PASSCOMMAND`. Alles, was die Tools aus ihrer Umgebung lesen,
kann unter `env` liegen (`RESTIC_PASSWORD`, `BORG_PASSPHRASE`,
Cloud-Keys, `BORG_RSH`…), und die eigene Dienstumgebung des Agents
wird auch durchgereicht.

Was der Agent ausführt, im strengsten Sinne nur-lesend:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: kein
  Lock wird geschrieben, daher kann ein nur-lesendes oder
  Append-Only-Repository beobachtet werden, und ein `prune`, der den
  exklusiven Lock hält, lässt das Repository nicht down aussehen;
  auch kein Cache, da der installierte Dienst nicht nach `/root`
  schreiben kann.
- `borg --bypass-lock list --json --last 1`: ohne
  `--bypass-lock` würde das Lesen für die gesamte Dauer jedes `borg
  create` fehlschlagen, der den exklusiven Lock hält. Borg schreibt
  einen kleinen Sicherheitsdatensatz unter `~/.config/borg`; wenn
  dieses Home nicht schreibbar ist (der installierte Dienst läuft
  mit `ProtectHome=yes`), richtet der Agent `BORG_BASE_DIR`
  stattdessen auf sein privates `/tmp`, sofern Sie `BORG_BASE_DIR`
  nicht selbst setzen. Ein Repository im `keyfile`-Modus braucht
  dann `BORG_KEY_FILE` in seinem `env`.

Da der Dienst mit `ProtectHome=yes` läuft, halten Sie, was die Tools
brauchen, außerhalb der Home-Verzeichnisse: Passwortdateien,
SSH-Schlüssel und `known_hosts` unter `/etc/dumbmonit/`, wie im
Beispiel, und lokale Repositories unter `/srv`, `/mnt` oder `/var`
statt `/home`. Die Lesungen laufen eine nach der anderen, innerhalb
des Speicherlimits des Dienstes (128 MiB mit dem Agent selbst).

Metriken, beschriftet mit `tool` (`restic` oder `borg`) und `repo`:
`agent_backup_repo_reachable` (1, wenn die letzte Lesung gelang,
sonst 0: Repository nicht erreichbar, Passwort abgelehnt, Binary
fehlt — der Agent protokolliert den Grund einmal, und erneut, wenn
er sich ändert), `agent_backup_repo_last_snapshot_age_seconds`
(Alter des neuesten Snapshots, fehlt, während das Repository leer
ist), und `agent_backup_repo_last_snapshot_bytes` (vom neuesten
restic-Snapshot verarbeitete Bytes, restic 0.17 und später). Mit
`--latest 1` gibt restic den neuesten Snapshot jedes Hosts und
Pfad-Sets zurück; der neueste von allen wird behalten, setzen Sie
daher `host` für ein von mehreren Maschinen geteiltes Repository.

Eingebaute Regeln: **restic/Borg backup too old** (kein Snapshot
seit mehr als 48 Stunden) und **restic/Borg repository unreachable**
(seit 30 Minuten). Die Geräteseite zeigt ein
Backup-Repositories-Panel, eine Zeile pro Repository.

Um eine Deklaration zu prüfen, führen Sie den Agent einmal im
Vordergrund aus und suchen Sie nach den `agent_backup_repo_*`-Samples:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## WireGuard-Tunnel {#wireguard-tunnel}

Auf einer Maschine, die WireGuard ausführt, liest der Agent jede
Schnittstelle und jeden Peer mit `wg show all dump`, bei jedem
Zyklus. Nichts zu konfigurieren: Ohne den Befehl `wg`
(wireguard-tools) oder ohne Schnittstelle wird keine Zeitreihe
ausgegeben und kein Panel gezeigt. `wg` braucht root
(`CAP_NET_ADMIN`); der installierte Dienst hat das, ein von Hand
unter Ihrem eigenen Konto gestarteter Agent protokolliert die
Ablehnung einmal und meldet nichts.

Der Dump hält auch den privaten Schlüssel der Schnittstelle und den
Pre-Shared-Key jedes Peers: Der Agent überspringt beide Felder, ohne
sie je zu kopieren.

Pro Peer, beschriftet mit `interface`, `peer` (sein öffentlicher
Schlüssel) und `allowed_ips`:
`agent_wireguard_peer_handshake_age_seconds` (Sekunden seit dem
letzten Handshake), `agent_wireguard_peer_has_handshake` (0, solange
der Peer nie einen abgeschlossen hat),
`agent_wireguard_peer_keepalive_seconds` (sein dauerhaftes
Keepalive, 0, wenn aus), und die Zähler
`agent_wireguard_peer_rx_bytes` und `agent_wireguard_peer_tx_bytes`;
pro Schnittstelle, `agent_wireguard_interface_peers`. Für einen
Peer, der nie einen Handshake abgeschlossen hat, zählt das Alter ab
dem Moment, in dem der Agent ihn zuerst sah: Ein Tunnel, der nie
hochkam, ist auch still und wird als solcher gemeldet.

WireGuard hat keine Peer-Namen. Geben Sie Ihre in `agent.yaml`, nach
öffentlichem Schlüssel, und sie werden das Label `name`:

```yaml
wireguard: true                     # default; false stops even looking
wireguard_bin: wg                   # default: on PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

Die eingebaute Regel **WireGuard peer silent** feuert, wenn ein Peer
**mit dauerhaftem Keepalive** seit mehr als 15 Minuten keinen
Handshake abgeschlossen hat. Während ein Tunnel up ist, erneuert
WireGuard seinen Handshake alle zwei Minuten, und ein Keepalive
hält ihn auch ohne Traffic auf. Ein Peer ohne Keepalive (ein
Telefon, ein Laptop) kann tagelang untätig sein, ohne dass etwas
kaputt ist, daher wird er ausgelassen. Setzen Sie
`PersistentKeepalive = 25` auf den Tunneln, die up bleiben müssen
(Site-to-Site-Links, entfernte Gateways). N ist der Schwellenwert
der Regel: Ändern Sie ihn unter **Alerts → Rules**, oder entfernen
Sie den `and on (…)`-Teil ihrer Abfrage, um jeden Peer zu
beobachten. Die Geräteseite zeigt ein WireGuard-Panel mit jedem
Peer, der Zeit seit seinem letzten Handshake, seinem Traffic und
seinem Keepalive.

## TLS-Zertifikate (Certbot / Let's Encrypt)

Der Agent liest `/etc/letsencrypt` nicht direkt. Jedes von Certbot
erneuerte Zertifikat wird irgendwo ausgeliefert — ein Webserver, ein
Mailserver, ein Reverse Proxy — und dort liest DumbMonit es: Fügen
Sie einen [TLS-Zertifikat](services.md#tls)-Monitor hinzu (oder
einen [Website or web API](services.md#http), für eine schon so
beobachtete Site), der auf den Port zeigt, auf dem das Zertifikat
tatsächlich präsentiert wird. Er liest das lebende Zertifikat, keine
Datei auf der Platte, daher fängt er auch eine Erneuerung, die in
`/etc/letsencrypt` gelang, aber vom Dienst nie übernommen wurde
(ein Reverse Proxy, der einen Reload braucht, um die neue Datei zu
sehen, ist der übliche Fall, und genau die Lücke, die wichtig ist).

`/etc/letsencrypt/live/*/cert.pem`, die Erneuerungskonfiguration und
das Certbot-Log vom Agent aus zu lesen wurde in Betracht gezogen und
verworfen: Es würde das Ablaufdatum duplizieren, das der
TLS-Monitor schon meldet, während ein zweiter, host-lokaler Pfad zu
derselben Information hinzugefügt wird — eine weitere Sache, die
synchron zu halten ist, nachdem ein Zertifikat verschoben oder eine
Domain umbenannt wurde. Ein Host, auf dem Certbot läuft, aber nichts
ausliefert, das DumbMonit direkt erreichen kann (ein interner
CA-Schritt, ein DNS-Validierungs-Hook), ist der eine Fall, den das
unabgedeckt lässt; es ist auch der Fall, in dem ein TLS-Monitor
ebenfalls nicht helfen könnte, da nichts auf diesem Host TLS zum
Abfragen spricht.
