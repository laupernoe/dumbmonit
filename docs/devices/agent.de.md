# Linux-, macOS-, FreeBSD- und Windows-Agent {#linux-macos-freebsd-and-windows-agent}

Ein kleines Binary, das auf dem zu überwachenden Rechner installiert wird. Es misst CPU,
Speicher, Platten, Netzwerk, Dienste und Container — und, wo der Rechner sie bereitstellt,
Temperaturen, Plattenzustand und ZFS-Pools — und **sendet** seine Messwerte dann per HTTP an den
DumbMonit-Server.

Die Richtung ist Absicht: Der Agent ruft den Server auf, nie umgekehrt. Er überwindet NAT und
Heim-Firewalls und verlangt, auf dem überwachten Rechner **keinen Port** zu öffnen.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_`. Identitätslabels (`target`, `host`, `tag_*`) setzt
der Server beim Empfang, nie der Agent: Ein Rechner kann nicht in die Serien eines anderen
Rechners schreiben, auch nicht durch das Fälschen seiner Labels.

| Familie | Metriken | Labels |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` nur mit `cpu_per_core: true` | `core` |
| Speicher | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Dateisysteme | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Netzwerk (Zähler) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Platten (Zähler) | `disk_read_bytes`, `disk_written_bytes`, eine Serie pro Blockgerät | `device` |
| Host | `uptime_seconds`, `process_count` | |
| Dienste | `service_up` (1 = läuft) | `service` |
| Container | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (Container über `docker_max_containers` hinaus) | `container`, `image` |
| Sensoren | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (die Schwelle, die die Hardware selbst deklariert), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Plattenzustand (SMART) | `agent_disk_smart_ok` (1 = die Platte besteht ihre eigene Selbstbewertung), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degradiert, 2 schlimmer), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; pro Peer `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (Zähler) — siehe [WireGuard-Tunnel](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — siehe [restic- und Borg-Backups](#restic-and-borg-backups) | `tool`, `repo` |
| Windows-Leistungsindikatoren | `agent_perf_counter`, eine Serie pro in `perf_counters` aufgeführtem Indikator | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — siehe [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; pro VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; pro virtueller Platte `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — siehe [Hyper-V-Hosts](#hyper-v-hosts) | `vm`, `disk` |
| Agent | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Eine Serie, die nicht gemessen werden kann, ist **abwesend**, nie null: kein `smartctl`, kein
`agent_disk_*`; keine lesbare Sonde, kein `agent_sensor_*`; kein ZFS, kein `agent_zfs_*`. Eine
Temperatur außerhalb von 1–150 °C wird verworfen statt veröffentlicht, und ein Lüfter mit 0 rpm
ebenfalls — ein Mainboard hat mehr Lüfteranschlüsse als Lüfter, und ein leerer liest null wie ein
toter Lüfter.

### Was wo erfasst wird {#what-is-collected-where}

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, Speicher, Swap, Dateisysteme, Netzwerk, Laufzeit, Prozesse | ja | ja | ja | ja |
| Load Average | ja | ja | ja | nein (Windows kennt den Begriff nicht) |
| Dienste | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | Windows-Dienste |
| Docker-Container | ja | ja, wenn `docker_socket` auf den Socket von Docker Desktop zeigt | nein (kein Docker-Daemon) | nein |
| Temperaturen | `/sys/class/hwmon` | SMC-Sensoren | `dev.cpu.N.temperature` (nur CPU) | nein |
| Lüfter | `/sys/class/hwmon` | nein | nein | nein |
| Plattenzustand (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| ZFS-Pools | `zpool` (OpenZFS) | nein | `zpool` | nein |
| OS-Zustand (Updates, Neustart, fehlgeschlagene Units, SELinux) | ja | nein | nein | nein |
| Plakar-Backups | ja | ja | ja | ja |
| WireGuard-Tunnel | `wg show` | wenn `wg` installiert ist (nicht getestet) | wenn `wg` installiert ist (nicht getestet) | nein |
| restic- und Borg-Backups | ja | ja | ja | nur restic (nicht getestet; Borg hat keinen Windows-Build) |
| Leistungsindikatoren, MDaemon-Queues, Hyper-V | nein | nein | nein | ja |
| Maschinenidentität | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | Hostname |
| Relay-Modus | ja | ja | ja | ja |

Windows hat absichtlich keine Temperaturzeile: Dort Sensoren zu lesen bedeutet den Umweg über WMI,
mit Administratorrechten, für Werte, die die meisten Rechner gar nicht bereitstellen. Nichts zu
veröffentlichen ist die ehrliche Antwort.

`smartctl` (smartmontools) und `zpool` werden in jeder Leseperiode gesucht und verwendet, wenn
vorhanden; keines wird vom Agenten installiert, und keines ist erforderlich. Beide brauchen root —
der vom Installer registrierte Dienst läuft als root, ein von Hand unter Ihrem eigenen Konto
gestarteter Agent findet nichts.

Zähler werden roh gesendet, damit ein Neustart des Agenten nie eine falsche Rate erzeugt; wenden
Sie in Abfragen `rate()` an.

Zutreffende integrierte Regeln: Gerät nicht erreichbar, CPU hoch, Ungewöhnliche CPU. „Gerät nicht
erreichbar“ funktioniert auch bei Agenten: Der Server prüft das Alter des letzten empfangenen
Batches und erklärt den Rechner nach drei verpassten Perioden (mindestens 90 Sekunden) für still.

Vier weitere lösen nur auf Rechnern aus, die die passenden Serien erzeugen, brauchen also keine
Einrichtung und können auf einem Rechner ohne die Hardware nicht auslösen:

| Regel | Löst aus, wenn |
|---|---|
| **Platten-SMART fehlerhaft** (kritisch) | die eigene Selbstbewertung einer Platte sie als fehlerhaft meldet |
| **ZFS-Pool degradiert** (kritisch) | ein Pool fünf Minuten lang nicht mehr `ONLINE` ist |
| **ZFS-Scrub hat Fehler gefunden** (kritisch) | der letzte Scrub Fehler fand oder der Pool dauerhaft beschädigte Dateien meldet |
| **Temperatur über kritisch** (kritisch) | ein Sensor die kritische Schwelle überschreitet, **die seine eigene Hardware deklariert** — kein Wert ist fest codiert, denn 85 °C sind bei einer Platte ein Notfall und bei einer Laptop-CPU ein normaler Nachmittag |

Drei weitere überwachen, was Sie selbst deklarieren oder ausführen, und bleiben sonst still:
**WireGuard-Peer still** (ein Peer mit persistentem Keepalive und 15 Minuten ohne Handshake),
**restic/Borg-Backup zu alt** (48 Stunden kein Snapshot) und **restic/Borg-Repository nicht
erreichbar** — siehe [WireGuard-Tunnel](#wireguard-tunnels) und
[restic- und Borg-Backups](#restic-and-borg-backups).

Auf einem Windows-Server mit MDaemon überwachen drei weitere dessen Queues: **MDaemon-Mail-Queue
wächst**, **MDaemon-Bad-Queue nicht leer** und **MDaemon-Retry-Queue hoch** (siehe
[Regeln](../alerting/rules.md#mdaemon-and-securitygateway)). Auf einem Hyper-V-Host überwachen drei
die virtuellen Maschinen: **Hyper-V-VM-Zustand kritisch**, **Hyper-V-Host-CPU hoch** und
**Hyper-V-virtuelle-Platte-Fehler** (siehe [Hyper-V-Hosts](#hyper-v-hosts)).

## Installieren {#install}

Die folgenden Schritte sind die, die der Hinweis neben dem Formular zeigt. Auf dem Rechner ist
kein Konto zu erstellen: Das Enrollment-Token ist das einzige Geheimnis.

1. Speichern Sie das Gerät in DumbMonit: Ein Enrollment-Token (dmon_…) wird einmalig angezeigt,
   mit dem kopierfertigen Installationsbefehl für Linux und Windows. (Ein Token lässt sich auch
   unter **Einstellungen → Agenten** erstellen.)
2. Dieses Token ist der Schlüssel des Agenten, um seine Messungen an DumbMonit zu senden. Es ist
   kein Konto auf dem Rechner: Dort ist nichts zu erstellen. Ein aus dem Gerätformular erstelltes
   Token ist einmalig verwendbar: Es meldet genau diesen Rechner an und sonst nichts. Für eine
   Flotte erstellen Sie unter Einstellungen → Agenten ein wiederverwendbares Token.
3. Führen Sie den Installationsbefehl auf dem zu überwachenden Rechner mit erhöhten Rechten aus
   (sudo unter Linux und macOS, eine PowerShell als Administrator unter Windows). Er lädt den
   Agenten herunter, schreibt das Token in agent.yaml und startet den Dienst.
4. Der Rechner erscheint innerhalb weniger Sekunden von selbst, nach seinem Hostnamen benannt. Bei
   diesem ersten Batch gibt der Server dem Agenten ein eigenes Geheimnis und speichert es in
   `/etc/dumbmonit/agent-secret`, für niemand sonst lesbar: Von da an ist dieser Rechner der
   einzige, der unter diesem Gerät melden kann. Token verloren? Unter Einstellungen → Agenten
   können Sie es widerrufen und ein anderes erstellen.
5. Docker: Um die Container zu sehen und DumbMonit sie neu starten oder aktualisieren zu lassen,
   muss der Agent den Docker-Socket erreichen. Der vom Installer registrierte Dienst kann das
   bereits; betreiben Sie den Agenten stattdessen unter einem dedizierten Benutzer, fügen Sie
   diesen Benutzer der Gruppe „docker“ hinzu und starten Sie ihn neu. Neustart- und
   Auto-Update-Richtlinien werden dann pro Container auf der Geräteseite festgelegt.

Zwei Dinge sollten Sie lesen, bevor Sie dies auf mehr als einem Rechner ausrollen:
[Bindung](#binding-one-machine-one-agent), die dafür sorgt, dass ein Gerät wirklich zu einer
Agent-Installation gehört, und [Einmalig oder Flotte](#single-use-or-fleet), die Wahl, die Sie
beim Erstellen eines Tokens treffen.

    ```
    usermod -aG docker dumbmonit
    ```

6. Plakar-Backups: automatisch erkannt — das Panel Backups erscheint, wenn das Binary plakar oder
   ein Kloset (~/.config/plakar/stores.yml jedes Benutzers, ~/.plakar, /var/lib/plakar) gefunden
   wird, sonst wird nichts angezeigt. Setzen Sie „plakar_klosets“ in agent.yaml, um eine feste
   Liste zu überwachen, oder „plakar: false“, um sich abzumelden.

!!! note
    Der Agent kontaktiert den Server, nie umgekehrt: Auf dem überwachten Rechner muss kein Port
    geöffnet werden.

=== "Linux (systemd oder OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Das Skript ist POSIX-`sh` (BusyBox und dash funktionieren), lädt das Binary für die
    Architektur des Rechners (x86_64 oder aarch64) vom Server herunter, schreibt
    `/etc/dumbmonit/agent.yaml` (Modus 0600) und registriert `dumbmonit-agent` bei systemd oder
    unter Alpine bei OpenRC. Die systemd-Unit ist gehärtet (`ProtectSystem=strict`,
    `NoNewPrivileges`, `MemoryMax=128M`, `CPUQuota=20%`).

=== "macOS (launchd)"

    Derselbe Befehl wie unter Linux, mit `sudo` ausgeführt:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Die macOS-Binaries sind **nicht** im DumbMonit-Image: Sie zu bauen erfordert das SDK von
    Apple, dessen Lizenz die Weiterverbreitung verbietet. Sie hängen stattdessen an jedem
    [GitHub-Release](https://github.com/laupernoe/dumbmonit/releases/latest) —
    `dumbmonit-agent-macos-aarch64` (Apple Silicon) und `dumbmonit-agent-macos-x86_64` (Intel),
    jeweils mit seiner `.sha256` — und der Server leitet `/download/dumbmonit-agent-macos-…` dorthin
    um, sodass das Skript das für die Architektur des Macs (`uname -m`) wählt. Der Server liefert
    diese Binaries nicht mit, daher trägt der Installationsbefehl dafür keine Prüfsumme: Ergänzen
    Sie `--sha256=macos-aarch64:<hex>` (oder `macos-x86_64`) mit dem Wert der `.sha256` des
    Releases, sonst lehnt der Installer den Download ab. Der Mac muss außerdem github.com
    erreichen.

    Ohne diesen Zugriff oder um ein bereits vorhandenes Binary zu installieren, laden Sie es
    herunter und übergeben es mit `--bin` (vergleichen Sie es zuerst mit der `.sha256` des
    Releases: `--bin` installiert, was Sie ihm geben):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    Das Skript installiert das Binary in `/usr/local/bin` (und entfernt das
    Quarantäne-Attribut, das ein Browser-Download trägt und das Gatekeeper das Starten
    verweigern ließe), schreibt `/usr/local/etc/dumbmonit/agent.yaml` (Modus 0600) und
    registriert den System-Daemon `com.dumbmonit.agent` in `/Library/LaunchDaemons`, gestartet
    mit `launchctl bootstrap system`. Er läuft beim Booten, ohne dass jemand angemeldet ist.

    !!! note "Lieber selbst bauen?"
        Auf dem Mac selbst, mit installiertem Rust:
        `cargo build --release -p dumbmonit-agent`, dann
        `--bin=target/release/dumbmonit-agent`. Das ist dasselbe Binary, das der Release-Job
        erzeugt.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Nur x86_64. Das Skript lädt `dumbmonit-agent-freebsd-x86_64` vom Server herunter, schreibt
    `/usr/local/etc/dumbmonit/agent.yaml` (Modus 0600) und installiert
    `/usr/local/etc/rc.d/dumbmonit_agent`, aktiviert mit `service dumbmonit_agent enable`.
    `daemon(8)` überwacht den Agenten, startet ihn neu, wenn er stoppt, und schreibt
    `/var/log/dumbmonit-agent.log`.

    Funktioniert genauso auf TrueNAS CORE und anderen FreeBSD-basierten Appliances, wo die
    ZFS-Serien der eigentliche Zweck sind.

=== "Windows (Dienst)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    Das Skript lädt `dumbmonit-agent-windows-x86_64.exe` vom Server herunter, prüft es gegen
    dessen SHA-256, installiert es als `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, schreibt
    `C:\ProgramData\DumbMonit\agent.yaml` (nur für SYSTEM und Administratoren lesbar) und
    registriert den Windows-Dienst `DumbMonitAgent` („DumbMonit system agent“): beim Booten
    automatisch gestartet, als LocalSystem, und nach einem Fehler neu gestartet (10 s, 30 s, dann
    60 s). Der Agent spricht selbst mit dem Dienstmanager (`--service`), sodass das Stoppen des
    Dienstes ihn senden lässt, was er noch hält, bevor er beendet.

    Ein Dienst hat keine Konsole: Der Agent schreibt sein Log nach
    `C:\ProgramData\DumbMonit\agent.log`, ab 10 MB als `agent.log.old` beiseitegelegt.

    Es gibt keinen nativen ARM64-Build: Unter Windows 11 für ARM installiert das Skript das
    x86_64-Binary, das unter x64-Emulation läuft.

Ein Token kann mehrere Rechner anmelden: Benennen Sie es nach einer Gruppe oder einem Rechner.

### Native Binaries {#native-binaries}

Der Agent ist eine einzelne statische ausführbare Datei pro System, ohne dass daneben etwas zu
installieren wäre:

| System | Datei | Bezugsquelle |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | der Server, `/download/…` |
| Linux aarch64 (64-Bit-ARM: Raspberry Pi OS 64-Bit, ARM-Server) | `dumbmonit-agent-linux-aarch64` | der Server, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | der Server, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | der Server, `/download/…` |
| macOS Apple Silicon | `dumbmonit-agent-macos-aarch64` | das GitHub-Release (der Server leitet dorthin um) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | das GitHub-Release (der Server leitet dorthin um) |

Die Linux-Binaries sind statisch gegen musl gelinkt: Sie laufen auf jeder Distribution, glibc oder
nicht (Alpine, NAS-Firmware), ohne passende Bibliothek. Jede Datei hat ihre Prüfsumme daneben,
`<file>.sha256`, im Format von `sha256sum`. Jedes
[GitHub-Release](https://github.com/laupernoe/dumbmonit/releases/latest) enthält alle sechs
Binaries und ihre Prüfsummen, für Rechner, die `/download/` des Servers nicht erreichen können,
oder zum Installieren mit `--bin`.

Seit 0.1.0-alpha.9 ist jedes Binary eines Releases zusätzlich schlüssellos mit
[Sigstore](https://www.sigstore.dev/) signiert (`<file>.sigstore.json`) und wird von einer
SLSA-Provenance-Attestierung begleitet. So prüfen Sie, dass ein Binary vom Release-Workflow
dieses Repositorys gebaut wurde:

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'
# or, with the GitHub CLI:
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Um die Linux-, Windows- und FreeBSD-Binaries selbst aus einem Klon des Repositorys zu bauen
(Docker ist die einzige Voraussetzung):

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Den Dienst verwalten {#managing-the-service}

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Status | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Logs | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Neustart | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Stopp | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Konfiguration | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| Dienstdatei | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | Dienst `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` unter Windows) entfernt auf jedem System alles, den Dienst
eingeschlossen.

!!! note "Der Download wird verifiziert — immer"
    Der von DumbMonit angezeigte Installationsbefehl trägt die erwartete SHA-256 jedes vom Server
    ausgelieferten Agent-Binarys (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` unter Windows). Beide Installer prüfen die heruntergeladene Datei
    dagegen, bevor sie etwas installieren, und **verweigern** die Installation, wenn sie nicht
    passt, wenn für die Plattform keine Prüfsumme angegeben ist oder wenn kein SHA-256-Werkzeug
    (`sha256sum`, `shasum`, `sha256`) verfügbar ist.

    Die Prüfsummen ändern sich mit jedem Server-Release: Kopieren Sie nach dem Upgrade von
    DumbMonit einen frischen Installationsbefehl aus **Einstellungen → Agenten** (oder von einem
    neuen Token), statt einen alten erneut auszuführen, der mit „checksum mismatch“ anhalten
    würde. Dieselben Prüfsummen stehen unter dem Installationsbefehl und sind unter
    `/download/<file>.sha256` veröffentlicht.

    Für Air-Gap- oder manuelle Setups installiert `--insecure-skip-checksum`
    (`-InsecureSkipChecksum`) ein heruntergeladenes Binary unverifiziert und sagt das laut;
    `--bin=PATH` installiert eine lokale Datei unverändert, deren Verifizierung Ihre Sache ist.

### Installer-Flags {#installer-flags}

| `install.sh` | `install.ps1` | Wirkung |
|---|---|---|
| `--token=TOKEN` | `-Token` | Enrollment-Token (erforderlich). |
| `--url=URL` | `-Url` | Server-URL, zum Beispiel `http://server:8080`. Unter Linux wird `DUMBMONIT_URL` in der Umgebung verwendet, wenn das Flag fehlt. |
| `--interval=N` | `-Interval` | Abtastperiode in Sekunden (Standard 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Dienste, deren Zustand gemeldet wird, benannt wie das Hostsystem sie nennt: systemd-Units unter Linux, launchd-Labels (`com.apple.sshd`) unter macOS, rc.d-Namen unter FreeBSD, Windows-Dienste. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Tags, auf jeder Serie als `tag_<key>` kopiert. |
| `--hostname=NAME` | `-HostName` | Dem Server angekündigter Name (Standard: der des Rechners). |
| `--bin=PATH` | `-BinPath` | Lokales Binary, das statt eines Downloads installiert wird. |
| `--sha256=LIST` | `-Sha256` | Erwartete SHA-256 des Downloads, als durch Kommas getrennte Paare `platform-arch:hex` (ein nackter Hex-Wert funktioniert auch). Im von DumbMonit gezeigten Befehl enthalten; erforderlich, sofern nicht das nächste Flag gegeben ist. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Installiert ein heruntergeladenes Binary ohne Verifizierung, mit einer Warnung. Nur für Air-Gap- oder manuelle Setups. |
| `--no-start` | | Installiert alles, kontaktiert aber den Server nicht und startet den Dienst nicht (Maschinen-Image, Tests). |
| `--uninstall` | `-Uninstall` | Deinstalliert den Agenten und löscht seine Konfiguration. |
| `--help` | | Zeigt die Hilfe. |

### Upgrade {#upgrade}

Beide Skripte sind idempotent: Den Installationsbefehl mit demselben Token und derselben URL
erneut auszuführen ersetzt das Binary, schreibt die Konfiguration neu und startet den Dienst neu.
Das ist die Upgrade-Prozedur. Nehmen Sie den Befehl vom aktuellen Server (Einstellungen →
Agenten): Sein `--sha256` muss zu den Binaries der Serverversion passen, auf die Sie aktualisiert
haben.

Derselbe Befehl migriert einen Rechner, auf dem noch der EzyMonit-Agent läuft, unter Linux und
Windows. Der Installer stoppt und entfernt den alten Dienst (`ezymonit-agent`, oder
`EzyMonitAgent` unter Windows), verschiebt die Konfiguration an den neuen Pfad
(`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`) und installiert an seiner
Stelle `dumbmonit-agent`. Das alte Token `ezym_…` bleibt gültig, der beim Erstellen des Tokens
gezeigte Befehl funktioniert also weiterhin; es muss nichts zuvor widerrufen werden.

## Konfigurationsdatei {#configuration-file}

`/etc/dumbmonit/agent.yaml` unter Linux, `/usr/local/etc/dumbmonit/agent.yaml` unter macOS und
FreeBSD, `C:\ProgramData\DumbMonit\agent.yaml` unter Windows:

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

`sensors`, `smart` und `zfs` werden erkannt, nie vorausgesetzt: Auf `true` belassen, kosten sie auf
einem Rechner ohne Sonde, ohne `smartctl` oder ohne Pool einen Blick pro Periode und geben gar
nichts aus. Setzen Sie eines auf `false`, um gar nicht erst hinzusehen — sinnvoll auf einem NAS,
dessen Platten Sie lieber nie von `smartctl` berühren lassen, obwohl der Agent bereits
`-n standby` übergibt, sodass eine schlafende Platte nur gemessen wird, wenn sie aus anderen
Gründen wach ist.

Daneben enthält `/etc/dumbmonit/agent-secret` (`C:\ProgramData\DumbMonit\agent-secret` unter
Windows) das Bindungsgeheimnis, das der Server diesem Rechner gegeben hat. Der Agent schreibt es
mit Modus `0600` und liest es bei jedem Start neu; es ist nicht Teil von `agent.yaml`, weil Sie es
nicht festlegen — siehe [Bindung](#binding-one-machine-one-agent). `DUMBMONIT_AGENT_SECRET_FILE`
verschiebt es, was Sie für einen Agenten in einem Container wollen.

Jeder Schlüssel kann durch die Umgebung überschrieben werden, wodurch der Agent in einem
Container ohne Einbinden einer Datei nutzbar ist:

| Variable | Wirkung |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Pfad der Konfigurationsdatei |
| `DUMBMONIT_AGENT_URL` | Server-URL |
| `DUMBMONIT_AGENT_TOKEN` | Enrollment-Token |
| `DUMBMONIT_AGENT_SECRET_FILE` | Wo das Bindungsgeheimnis aufbewahrt wird (Standard: `agent-secret`, neben der Konfigurationsdatei) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Abtastperiode |
| `DUMBMONIT_AGENT_HOSTNAME` | Dem Server angekündigter Name |
| `DUMBMONIT_AGENT_SERVICES` | Zu überwachende Dienste, kommagetrennt |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, kommagetrennt |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Pfad des Docker-Sockets |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Pro Host detaillierte Container, Standard `200` (`0`: nur zählen) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Ausgelassene Schnittstellen, kommagetrennte Namen oder Regexe; Standard `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Zu behaltende Schnittstellen; wenn gesetzt, ersetzt es die Ignore-Liste |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Aus Dateisystemen und Platten-I/O ausgelassene Mountpunkte; der Standard überspringt `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true`, um zusätzlich eine CPU-Serie pro Kern zu senden (Standard `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: Temperaturen und Lüfter (Standard `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: Plattenzustand über `smartctl` (Standard `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Pfad von `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Periode zwischen zwei SMART-Messungen, Standard `300`, Minimum `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: ZFS-Pools über `zpool` (Standard `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Pfad von `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Periode zwischen zwei ZFS-Messungen, Standard `120`, Minimum `30` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: WireGuard-Tunnel über `wg` (Standard `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Pfad von `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Pfad von `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Pfad von `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Periode zwischen zwei Messungen der restic- und Borg-Repositories, Standard `600`, Minimum `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` oder `auto`: Leistungsindikatoren von MDaemon (Standard `auto`, gelesen, wenn der Windows-Dienst `MDaemon` existiert) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` oder `auto`: Leistungsindikatoren von Hyper-V (Standard `auto`, gelesen, wenn der Windows-Dienst `vmms` existiert) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Pfade von Windows-Leistungsindikatoren, durch **Semikolons** getrennt (ein Komma kann Teil eines Instanznamens sein); ersetzt `perf_counters` |
| `DUMBMONIT_AGENT_SYSTEM_HEALTH` | `true` / `false`: Zustand des Betriebssystems, der Hauptschalter der nächsten vier (Standard `true`) |
| `DUMBMONIT_AGENT_CHECK_UPDATES` | `true` / `false`: ausstehende Updates (Standard `true`) |
| `DUMBMONIT_AGENT_UPDATES_INTERVAL_SECS` | Periode zwischen zwei Prüfungen auf ausstehende Updates, Standard `3600`, Minimum `300` |
| `DUMBMONIT_AGENT_CHECK_REBOOT` | `true` / `false`: Neustart erforderlich (Standard `true`) |
| `DUMBMONIT_AGENT_CHECK_FAILED_UNITS` | `true` / `false`: fehlgeschlagene systemd-Units (Standard `true`) |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Größe des Aufhol-Puffers |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true`, um für den Server die Prüfungen der diesem Agenten zugewiesenen Geräte auszuführen (siehe [Den Agenten in Docker / in einem anderen Netzwerk betreiben](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Standortlabel, das neben dem Agenten angezeigt wird, wenn er als Relay angeboten wird |

## Windows-Leistungsindikatoren {#windows-performance-counters}

Unter Windows liest der Agent Leistungsindikatoren — die Werte, die der Leistungsmonitor
(`perfmon`) zeigt — über die PDH-Bibliothek und veröffentlicht sie in jeder Abtastperiode.
Manche Software veröffentlicht ihren Zustand sonst nirgends.

**MDaemon.** Existiert der Windows-Dienst namens `MDaemon`, liest der Agent das
MDaemon-Leistungsobjekt: Queue-Größen, Sitzungen, Nachrichten- und Spam-Summen, den Zustand der
internen Server von MDaemon und seine Laufzeit. Die Serien, die Regeln und die Seite, die sie
zeigt, sind auf der Seite [MDaemon](mdaemon.md#mail-queues-through-the-agent) beschrieben.
`mdaemon: true` liest sie auch ohne den Dienst (ein umbenannter Dienst), `mdaemon: false` nie.
Das Vorhandensein des Dienstes wird alle zehn Perioden neu geprüft, sodass ein nach dem Agenten
installiertes MDaemon ohne Neustart erfasst wird.

**Hyper-V.** Existiert der Dienst Hyper-V Virtual Machine Management (`vmms`), liest der Agent die
Leistungsobjekte von Hyper-V; siehe [Hyper-V-Hosts](#hyper-v-hosts). `hyperv: true` und
`hyperv: false` erzwingen die Wahl, wie bei MDaemon.

**Jeder andere Indikator.** Führen Sie seinen vollständigen Pfad unter `perf_counters` auf, als
`\Object(Instance)\Counter` oder `\Object\Counter` für ein Objekt mit einer einzigen Instanz, und er
wird als `dumbmonit_agent_perf_counter{counter="…"}` veröffentlicht, mit dem von Ihnen
angegebenen Namen oder dem Pfad selbst:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Schreiben Sie die Indikatornamen **auf Englisch**, wie sie im Leistungsmonitor eines englischen
  Windows erscheinen: Der Agent fügt sie mit `PdhAddEnglishCounterW` hinzu, sodass dieselbe
  `agent.yaml` auch auf einem französischen oder deutschen Windows funktioniert.
- Setzen Sie jeden Pfad in YAML in einfache Anführungszeichen, damit die Backslashes und das `%`
  unverändert bleiben.
- Ein fehlerhaft geformter Pfad stoppt den Agenten beim Start mit Schlüssel und Grund. Ein Pfad, den
  Windows nicht kennt (die Software ist nicht installiert, der Name ist falsch geschrieben), wird
  einmal im Log gemeldet, `performance counter not available`, und die übrigen Indikatoren werden
  weiter gelesen; er wird alle zehn Perioden erneut versucht.
- Raten (`…/sec`) brauchen zwei Messungen: Sie erscheinen ab der zweiten Periode.
- Es werden höchstens 200 Indikatoren gelesen. Unter Linux, macOS und FreeBSD wird die Einstellung
  akzeptiert und nichts gelesen.

Um die genauen Namen auf dem Rechner zu sehen, führen Sie `typeperf -q MDaemon` (oder einen
anderen Objektnamen) in einer Eingabeaufforderung aus.

## Hyper-V-Hosts {#hyper-v-hosts}

Auf einem Hyper-V-Host installiert — Windows Server mit der Hyper-V-Rolle oder Windows mit
aktiviertem Hyper-V — findet der Agent den Dienst `vmms` und liest die eigenen
Leistungsindikatoren von Hyper-V, ohne dass etwas zu konfigurieren wäre. Die Geräteseite des
Agenten zeigt dann ein Hyper-V-Panel: wie viele virtuelle Maschinen gesund melden, die echte
CPU-Last des Hosts, den Speicher, den jede laufende VM hält, und Durchsatz, Latenz und Fehler
jeder virtuellen Platte.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht auf einem echten System getestet"

    Die Indikatornamen stammen aus der Hyper-V-Leistungsdokumentation von Microsoft und wurden mit
    einer simulierten Indikatorquelle gegen die Logik geprüft, nicht auf einem laufenden
    Hyper-V-Host. Ein Indikator, den Windows nicht kennt, wird einmal im Log des Agenten gemeldet,
    und die übrigen werden weiter gelesen. Sagen Sie uns, was kaputtgeht.

| Indikator | Serien | Labels |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: virtuelle Maschinen in jedem Zustand | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: die Last des Hosts, Gäste eingeschlossen | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: der Anteil, der für den Betrieb von Gästen aufgewendet wird | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: laufende VMs plus der Host selbst | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: Speicherbedarf gegenüber zugewiesenem Speicher; über 100 will die VM mehr, als sie hat | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (Durchschnitt), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, ein Zähler | `disk` |

Auf einem Hyper-V-Host misst `\Processor(_Total)\% Processor Time` nur die eigene Partition des
Hosts, nicht was die virtuellen Maschinen verbrauchen: Der Indikator für logische Prozessoren des
Hypervisors ist der, der sagt, ob die Maschine gesättigt ist. `vm` ist der Name der virtuellen
Maschine, wie ihn der Hyper-V-Manager zeigt; `disk` ist der Instanzname, den Windows jeder
virtuellen Platte gibt, abgeleitet von ihrem Dateipfad. Die Instanzen `_Total` werden
ausgelassen, und pro Indikator werden höchstens 200 VMs oder Platten gelesen.

Drei [integrierte Regeln](../alerting/rules.md#hyper-v) gelten:

- **Hyper-V-VM-Zustand kritisch**: Hyper-V meldet eine VM fünf Minuten lang in kritischem Zustand
  (Warnung) — meist eine Platte, der der Platz ausgegangen ist.
- **Hyper-V-Host-CPU hoch**: die logischen Prozessoren fünfzehn Minuten lang zu mehr als 90 %
  ausgelastet (Hinweis).
- **Hyper-V-virtuelle-Platte-Fehler**: Eine virtuelle Platte meldete in der letzten Stunde
  I/O-Fehler (Hinweis).

## Den Agenten in Docker / in einem anderen Netzwerk betreiben {#run-the-agent-in-docker--on-another-network}

Der Agent wird auch als Container-Image veröffentlicht, mit denselben Tags wie der Server:
`ghcr.io/laupernoe/dumbmonit-agent:latest` (letztes Release), `:edge` (letzter Commit auf
`main`). Es ist ein `FROM scratch`-Image mit dem Agent-Binary und dem Mozilla-CA-Bundle,
vollständig über Umgebungsvariablen konfiguriert. Das Repository liefert eine einsatzbereite
`docker-compose.agent.yml` mit:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

Was der Container sieht und was einzubinden ist:

| Bedarf | Was zu tun ist | Warum |
|---|---|---|
| CPU, Speicher, Last, Platten-I/O des Hosts | Nichts | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` und `/proc/diskstats` sind in jedem Container hostweit. |
| Hostname | `DUMBMONIT_AGENT_HOSTNAME` | Der Hostname eines Containers ist eine zufällige ID; das Gerät würde danach benannt. |
| Container des Hosts | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Optional. Schreibgeschützt listet und inspiziert es; die Aktionen Neustart/Update brauchen es beschreibbar und `DUMBMONIT_AGENT_COMMANDS=true`. Ohne den Socket meldet der Agent schlicht keine Container. |
| Dateisystemnutzung | `- /:/host:ro` (oder die Mounts, die Sie interessieren) | Der Agent meldet die *innerhalb* des Containers sichtbaren Mounts, unter ihrem Container-Pfad. |
| Netzwerkzähler des Hosts | `network_mode: host` | Schnittstellen sind per Namespace getrennt: Ohne das sieht der Agent ein `veth`. |
| Über diesen Agenten weitergeleitete Ping-Monitore | `sysctls: net.ipv4.ping_group_range: "0 2147483647"` | ICMP nutzt einen Echo-Socket, den dieser Sysctl erlaubt: keine Capability nötig. Siehe [ICMP-Ping ohne Capability](../install/docker.md#icmp-ping-without-a-capability). |
| Private CA | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Server hinter einem internen Reverse-Proxy, Geräte mit selbstsignierten Zertifikaten. |
| Bindung über Neuerstellungen hinweg behalten | `- agent-state:/var/lib/dumbmonit-agent` und `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | Das Bindungsgeheimnis liegt im Dateisystem des Containers, das ein `docker compose up --force-recreate` wegwirft. Ohne das Volume kommt der Agent ungebunden zurück, und jemand muss jedes Mal auf der Geräteseite die Neuanmeldung erlauben. |

`pid: host` und `privileged: true` sind nie nötig: Der Agent erfasst keine Metriken pro Prozess.

### Relay-Modus: einen anderen Standort überwachen {#relay-mode-monitor-another-site}

Ein Agent kann mehr tun, als seinen eigenen Rechner zu beobachten. Mit `relay: true` (oder
`DUMBMONIT_AGENT_RELAY=true`) **delegiert der Server Prüfungen** an ihn: Geräte, deren
Einstellung *Erreicht über* diesen Agenten nennt, werden nicht vom Server abgefragt, sondern vom
Agenten, aus seinem eigenen Netzwerk, mit genau denselben Collectors — SNMP, Proxmox VE, Proxmox
Backup Server, Synology, HTTP, TCP, DNS, Ping, TLS. Die Ergebnisse landen unter dem Gerät, als
hätte der Server es abgefragt: dieselben Metriken, dieselbe *letzte Prüfung*, dieselben Alarme. Ein
entferntes Büro, ein zweites Homelab oder das Netzwerk eines Kunden erscheinen so in dem einen
DumbMonit, das Sie bereits haben, ohne VPN und ohne dass auf der entfernten Seite etwas zu öffnen
wäre.

Wie man es Schritt für Schritt einrichtet, steht unter
[Einen entfernten Standort überwachen](../install/remote-site.md).

## Bindung: ein Rechner, ein Agent {#binding-one-machine-one-agent}

Ein Enrollment-Token sagt *dieser Rechner darf sprechen*. Es sagt nicht, *welcher* Rechner
spricht — dafür ist der Identitätsschlüssel (`/etc/machine-id` oder der Hostname) da, und dieser
Schlüssel ist kein Geheimnis: Er lässt sich auf jedem Rechner in Sekunden lesen.

Beim ersten Batch gibt der Server daher jedem Rechner ein eigenes **Bindungsgeheimnis**. Er
behält nur einen Fingerprint; der Agent schreibt das Geheimnis mit Modus `0600` nach
`/etc/dumbmonit/agent-secret` und legt es danach bei jeder Anfrage im Header
`X-DumbMonit-Agent-Secret` vor — Messungen, Container-Befehle und weitergeleitete Prüfungen
gleichermaßen. Von da an gilt:

- Kein anderer Rechner kann Messungen unter diesem Gerät senden oder dessen Hostnamen und
  Betriebssystem überschreiben;
- kein anderer Rechner kann die für ihn eingereihten Befehle Neustart und Update abrufen — und
  damit verbrauchen.

Die Geräteseite zeigt unter **Agent → Bindung**, wo ein Rechner steht:

| Zustand | Was es bedeutet | Was zu tun ist |
|---|---|---|
| **Gebunden** | Der Agent hält ein eigenes Geheimnis. | Nichts. |
| **Nicht gebunden — diesen Host neu anmelden** | Ein Host, der angemeldet wurde, bevor es die Bindung gab, und seither nie gebunden wurde. Der Server lehnt seine Messungen, Container-Befehle und weitergeleiteten Prüfungen ab. | Klicken Sie auf der Geräteseite auf **Neuanmeldung erlauben** und starten Sie dann den Agenten neu. Ist der Agent älter als die Bindung, führen Sie zuerst den Installationsbefehl auf dem Host erneut aus. |

### Hosts, die vor der Bindung angemeldet wurden {#hosts-that-predate-binding}

!!! warning "Geändert: ungebundene Hosts werden abgelehnt"
    Frühere Releases akzeptierten weiter Hosts, die vor der Bindung angemeldet wurden, solange sie
    das Token verwendeten, mit dem sie angemeldet wurden, und banden sie still, sobald ihr Agent
    aktualisiert war. Dieses Übergangsfenster ist geschlossen: Ein Host, der nie gebunden wurde,
    wird jetzt auf **jeder Agent-Route abgelehnt** — Messungen, Container-Befehle und
    weitergeleitete Prüfungen —, bis ein Administrator ihn neu anmeldet. Ein Flotten-Token wird von
    vielen Rechnern geteilt, und der Identitätsschlüssel lässt sich erraten, daher beweist nichts
    anderes, dass der anfragende Agent der eigene des Hosts ist.

    Ein Agent-Binary, das zu alt ist, um ein Bindungsgeheimnis zu erhalten, wird ebenfalls
    abgelehnt, auch bei seiner ersten Anmeldung.

Um einen solchen Host zurückzuholen:

1. Ist sein Agent älter als die Bindung, führen Sie den Installationsbefehl erneut auf ihm aus (das
   Agent-Binary muss ein Bindungsgeheimnis speichern können).
2. Öffnen Sie seine Geräteseite und klicken Sie auf **Agent → Neuanmeldung erlauben**. Das öffnet
   ein einstündiges Fenster.
3. Starten Sie den Agenten neu oder warten Sie auf seinen nächsten Versuch: Dieser Batch bindet den
   Host mit einem frischen Geheimnis und schließt das Fenster. Ein anderes Token als das, mit dem
   der Host angemeldet wurde, muss weiterhin anmelden können (nicht aufgebraucht, nicht abgelaufen).

Bis dahin trägt das Log des Agenten die Erklärung des Servers:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Neuanmeldung nach einer Neuinstallation {#re-enrolment-after-a-reinstall}

Wird der Rechner neu aufgebaut, seine Platte ersetzt oder sein Agent-Container ohne das Volume mit
dem Geheimnis neu erstellt, kommt der Agent ohne es zurück, und der Server lehnt seine Batches ab.
Diese Ablehnung ist Absicht: Ein gebundener Rechner bleibt gebunden, bis ein Mensch etwas anderes
sagt, sonst würde die Bindung nichts schützen.

Der Weg zurück liegt auf der Geräteseite: **Agent → Neuanmeldung erlauben**. Das öffnet ein
einstündiges Fenster, in dem ein Agent, der ein gültiges Enrollment-Token vorlegt, diesen Rechner
mit einem frischen Geheimnis erneut bindet; das alte Geheimnis funktioniert nicht mehr, und das
Fenster schließt sich, sobald es genutzt wurde. Der Agent versucht es in der Zwischenzeit selbst
erneut, auf dem Rechner ist also nichts neu zu starten — und nichts geht verloren, da seine
Messwerte gepuffert bleiben, bis die Verbindung wieder akzeptiert wird.

Solange niemand dieses Fenster öffnet, sagt das Log des Agenten, was zu tun ist:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Enrollment-Tokens {#enrolment-tokens}

- Tokens werden unter **Einstellungen → Agenten** oder beim Hinzufügen eines Geräts vom Typ Agent
  erstellt. Der Server behält nur einen Fingerprint: Das Klartext-Token wird einmalig in der
  Antwort auf die Erstellung angezeigt, mit den beiden Installationsbefehlen.
- Der Agent sendet es als `Authorization: Bearer dmon_…` auf `POST /api/ingest`. Es erscheint nie
  in den Logs des Agenten, auch nicht gekürzt.
- **Einstellungen → Agenten** listet Tokens mit Präfix, Umfang, Erstellungsdatum und letzter
  Verwendung. **Widerrufen** stoppt jeden Agenten, der dieses Token verwendet, bei seinem nächsten
  Push; die Rechner werden dann nach drei verpassten Perioden *nicht erreichbar*. Führen Sie den
  Installer mit einem neuen Token erneut aus, um sie neu anzumelden.
- Rechner sind getrennte Geräte: Das Widerrufen eines Tokens löscht sie nicht.

### Einmalig oder Flotte {#single-use-or-fleet}

Der Umfang eines Tokens wird beim Erstellen gewählt, und es ist eine Wahl, kein Standard, der
Ihnen widerfährt:

| Umfang | Was es anmeldet | Wann |
|---|---|---|
| **Einmalig** (der Standard) | Genau einen Rechner. | Eine Installation. Ein Installationsbefehl lebt in der Shell-Historie, in einem Ticket, in einem Chatlog weiter; ein einmaliger ist nach der Verwendung wertlos. |
| **Wiederverwendbar für eine Flotte** | So viele Rechner, wie Sie erlauben — eine Zahl oder unbegrenzt. | Ein Playbook, ein Maschinen-Image, eine Reihe von Installationen. |

Beide können eine Frist in Tagen tragen. Eine Frist stoppt nur *Anmeldungen*: Bereits angemeldete
Rechner melden auch nach ihrem Ablauf weiter, was für ein Installationsfenster das ist, was Sie
wollen. Das Widerrufen ist es, was eine Flotte abschneidet.

Ein Token, das nicht mehr anmelden kann — aufgebraucht oder über seine Frist —, wird unter
Einstellungen → Agenten als **Meldet nicht mehr an** angezeigt. Seine Rechner melden weiter; es
kann nur keinen neuen mehr hereinlassen.

## Ausfälle und Diagnose {#outages-and-diagnostics}

Ist der Server nicht erreichbar, misst der Agent weiter und puffert seine Messwerte mit ihren
ursprünglichen Zeitstempeln im Speicher. Sie werden gesendet, sobald die Verbindung wieder steht:
Ein Serverneustart hinterlässt kein Loch in den Diagrammen. Der Puffer ist begrenzt
(`max_buffered_samples`, standardmäßig etwa eine Stunde) und verwirft bei Überlauf die ältesten
Messwerte. Wiederholungen weichen von 5 Sekunden bis 5 Minuten mit Jitter zurück.

```sh
dumbmonit-agent --dry-run          # print the readings, send nothing
dumbmonit-agent --once             # send one batch then exit
journalctl -u dumbmonit-agent -f   # service log (systemd)
tail -f /var/log/dumbmonit-agent.log   # service log (OpenRC)
```

## Häufige Fehler {#common-errors}

| Symptom | Wahrscheinliche Ursache |
|---|---|
| Installer meldet „could not reach the server“ | Falsche URL oder falsches Token. Die Konfiguration ist geschrieben: Korrigieren Sie `agent.yaml` und starten Sie den Dienst neu. |
| Download scheitert mit 404 | Der Server hat keine Agent-Binaries (`DUMBMONIT_AGENT_DIR` leer). Verwenden Sie `--bin=PATH` mit einem selbst gebauten Binary. |
| Installer stoppt mit „checksum mismatch“ | Der Befehl wurde vor einem Server-Upgrade kopiert (kopieren Sie einen frischen aus Einstellungen → Agenten), oder das erhaltene Binary ist nicht das, das der Server ausliefert: ein Proxy oder Cache im Weg, oder ein manipulierter Download. |
| Installer stoppt mit „no expected checksum“ | Der Befehl hat kein `--sha256` für diese Plattform: ein alter Befehl oder ein macOS-Binary (nicht vom Server ausgeliefert). Kopieren Sie einen frischen Befehl oder übergeben Sie `--sha256=<platform>-<arch>:<hex>` aus der `.sha256` des Releases. |
| Rechner erscheint nie | Der Push geht an `/api/ingest` auf der URL im Installationsbefehl; stellen Sie hinter einem Reverse-Proxy sicher, dass sie weitergeleitet wird und Bodies bis 16 MB erlaubt sind. |
| *Nicht erreichbar*, obwohl der Agent läuft | Das Token wurde widerrufen, oder die Pushes werden abgelehnt. Prüfen Sie `journalctl -u dumbmonit-agent`: Der Fehler wird dort protokolliert. |
| Log meldet „this agent is not recognised for this machine“ | Der Rechner ist an eine Agent-Installation gebunden, deren Geheimnis dieser nicht hat — eine Neuinstallation oder ein ohne sein Zustands-Volume neu erstellter Container. Öffnen Sie die Geräteseite und klicken Sie auf **Neuanmeldung erlauben**; siehe [Neuanmeldung nach einer Neuinstallation](#re-enrolment-after-a-reinstall). |
| Log meldet „this machine was enrolled before agent binding“ | Ein Host, der nie an seinen Agenten gebunden wurde. Klicken Sie auf seiner Geräteseite auf **Neuanmeldung erlauben** und starten Sie dann den Agenten neu; siehe [Hosts, die vor der Bindung angemeldet wurden](#hosts-that-predate-binding). |
| Log meldet „this agent is too old to be bound to its machine“ | Das Agent-Binary kann kein Bindungsgeheimnis speichern. Führen Sie den Installationsbefehl auf dem Host erneut aus; existiert das Gerät bereits, klicken Sie zuerst auf **Neuanmeldung erlauben**. |
| Log meldet „this enrollment token has already enrolled all the machines it was allowed to“ | Ein einmaliges Token wird wiederverwendet. Erstellen Sie ein neues oder ein wiederverwendbares Token unter Einstellungen → Agenten. |
| Windows | Die Windows-Dienstteile waren zum Zeitpunkt des Schreibens im Build-Image des Projekts nicht erprobt; melden Sie Probleme auf GitHub. |

## Docker-Container {#docker-containers}

Kann der Agent den Docker-Socket lesen (`docker: true`, der Standard, und ein Benutzer in der
Gruppe `docker` oder root — `usermod -aG docker dumbmonit`, dann den Agenten neu starten), wird
jeder Container zu einer Handvoll Serien, mit den Labels `container` und `image` (das `name:tag`,
das `docker ps` zeigt). Sagt die Geräteseite „Kein Docker auf diesem Rechner“, obwohl Docker
installiert ist, ist das das Erste, was zu prüfen ist.

| Metrik | Bedeutung |
|---|---|
| `container_up` | 1 läuft, 0 sonst |
| `container_health` | 0 kein Healthcheck, 1 gesund, 2 ungesund, 3 startet |
| `container_restart_count` | Neustartzähler von Docker für den Container |
| `container_started_seconds` | Sekunden seit dem letzten Start des Containers (0, wenn gestoppt) |
| `container_image_age_seconds` | Sekunden seit dem Bau des Images |
| `container_update_available` | 1, wenn die Registry ein neueres Image für dasselbe Tag hat, 0, wenn aktuell, -1 unbekannt |

Die Update-Prüfung vergleicht den Digest des lokalen Images mit dem, den die Registry für das Tag
ankündigt (`HEAD /v2/<name>/manifests/<tag>`, anonymer Token-Ablauf bei Docker Hub und GHCR). Sie
läuft im Hintergrund höchstens einmal pro Stunde und Image und verzögert nie den 30-Sekunden-Zyklus;
eine private Registry, die anonymen Zugriff verweigert, liest sich schlicht als -1. Deaktivieren Sie
sie mit `docker_update_check: false` (`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Integrierte Regeln: **Container gestoppt** (`container_up == 0` für 2 Minuten), **Container
ungesund** (`container_health == 2` für 3 Minuten), **Container startet immer wieder neu** (drei
oder mehr Neustarts in 15 Minuten), **Container-Update verfügbar** (Info, nach einer Stunde,
täglich wiederholt). Der Alarm nennt den Container.

### Aktionen {#actions}

Auf der Geräteseite hat jeder Container zwei Aktionen und zwei Schalter. Aktionen laufen über einen
**Befehlskanal**: Der Server reiht den Befehl ein, der Agent ruft ihn nach seinem nächsten Batch ab
(`GET /api/agent/commands`, dasselbe Enrollment-Token), führt ihn aus und meldet das Ergebnis
(`queued → running → done | failed`), mit einem kurzen Log, das Sie in der Oberfläche aufklappen
können. Befehle, die älter als zehn Minuten sind, werden nie ausgeführt; der Agent führt jeweils
einen Befehl aus.

Ein eingereihter Befehl kann in der Oberfläche **abgebrochen** werden, solange der Agent ihn nicht
aufgenommen hat. Ein Befehl, den niemand abgeholt hat, **verfällt** nach zehn Minuten auf dem
Server, ob der Agent je abfragt oder nicht — sodass ein Rechner mit gestopptem Agenten nie ein
veraltetes „In Warteschlange“ behält und nie die nächste Aktion auf diesem Container blockiert.

Der Agent teilt dem Server mit jedem Batch mit, ob er Befehle akzeptiert. Tut er es nicht — ein
Agent, der älter als der Befehlskanal ist, oder mit `commands: false` installiert —, sagt die
Geräteseite das, statt Neustart und Update zu zeigen, die API lehnt die Befehle ab (`409`), und
Richtlinien reihen nichts ein. Installieren Sie den Agenten mit dem aktuellen Installer neu, um
Aktionen zu aktivieren.

- **Neustart** — `docker restart` für einen laufenden Container, `docker start` für einen
  gestoppten, dann eine 20-Sekunden-Prüfung, dass er läuft.
- **Jetzt aktualisieren** — zieht das Tag, das der Container bereits verwendet, erstellt einen
  neuen Container mit derselben Konfiguration (Umgebung, Mounts, Ports, Labels, Netzwerke,
  Neustartrichtlinie), stoppt den alten, tauscht die Namen, startet den neuen und wartet auf
  seinen Healthcheck (oder 20 Sekunden plus eine Prüfung auf Laufen, wenn das Image keinen hat).
  Schlägt irgendetwas fehl, führt der Agent ein **Rollback** durch: Der neue Container wird entfernt
  und der alte neu gestartet, und der Befehl meldet `failed` mit dem Grund. Bei Erfolg wird der
  alte Container entfernt und, wenn „Prune“ an ist, auch das alte Image — es sei denn, ein anderer
  Container verwendet es noch.

Von Compose verwaltete Container (Labels `com.docker.compose.*`) werden trotzdem aktualisiert, aber
das Ergebnis warnt, dass das nächste `docker compose up` den Container aus der Compose-Datei neu
erstellen wird. Container, deren Name mit `dumbmonit` oder `dumbmonit` beginnt, werden abgelehnt: Der
Monitor aktualisiert sich nie selbst.

### Richtlinien {#policies}

Eine **Richtlinie** pro Container wird vom Server jede Minute ausgewertet:

- **Neustart, wenn ausgefallen** — ein Container, der `container_up == 0` meldet, wird automatisch
  neu gestartet, höchstens einmal alle zehn Minuten.
- **Auto-Update in Wartungsfenstern** — ein Container mit verfügbarem Update wird automatisch
  aktualisiert, aber nur, solange ein Wartungsfenster (Stummschaltung) das Gerät abdeckt,
  höchstens einmal alle sechs Stunden. Planen Sie eines auf der Seite Alarme oder schalten Sie das
  Gerät auf seiner Seite stumm. Richtlinien bereinigen standardmäßig das alte Image und gelten nur
  in Wartungsfenstern.

Zwei Stellen legen sie fest:

- der **Docker-Streifen** unter dem Geräte-Header: „28 Container · 27 laufen · 3 Updates verfügbar ·
  Richtlinien: 2 Auto-Neustart, 0 Auto-Update“, die letzte Aktion und ihr Status, **Container
  verwalten** (öffnet die untenstehende Liste) und **Richtlinien …**, eine Tabelle aller Container
  mit den beiden Schaltern und einer Zeile „Auf alle anwenden“ — der schnellste Weg, den
  Auto-Neustart für einen ganzen Rechner einzuschalten;
- die Schalter in jeder Zeile des Abschnitts **Container**, neben den Schaltflächen Neustart und
  Jetzt aktualisieren.

Jeder Schalter speichert für sich (`PUT /api/targets/<id>/containers/<name>/policy`); ein Fehler
betrifft nur diese Zeile und wird neben ihr angezeigt.

Automatische Aktionen erscheinen unter „Letzte Aktionen“ als von `policy` angefordert.

### Sicherheitsregeln {#safety-rules}

- Der Agent handelt nur bei `commands: true` (dem Standard); setzen Sie es auf `false` für einen
  Rechner, der nur gemessen werden soll.
- Der Agent ruft Befehle selbst mit seinem Enrollment-Token ab — nichts verbindet sich mit dem
  Rechner. Sie verfallen nach zehn Minuten und laufen einzeln; der Agent zieht nie ein anderes
  Tag als das, das der Container bereits verwendet.
- Ein Update, das nicht gesund zurückkommt, wird auf den vorherigen Container zurückgerollt, und
  das alte Image wird erst entfernt, wenn der neue Container läuft.
- Container mit Namen `dumbmonit*` oder `ezymonit*` werden abgelehnt, von Hand oder per Richtlinie:
  Der Monitor handelt nie an seinen eigenen Containern. Die Prüfung erfolgt zweimal — der Server
  weigert sich, den Befehl einzureihen, und der Agent weigert sich, ihn auszuführen — und der Agent
  wiederholt sie am Namen, den Docker nach dem Inspizieren des Containers meldet, sodass das
  Benennen über seine ID nichts ändert. Der Agent weigert sich auch, an seinem eigenen Container zu
  handeln, wie auch immer er heißt: Er wäre nicht mehr da, um die Arbeit abzuschließen oder
  darüber zu berichten.
- Von Compose verwaltete Container werden aktualisiert, aber das nächste `docker compose up`
  erstellt sie aus der Compose-Datei neu: Erhöhen Sie das Tag auch dort.

## Plakar-Backups {#plakar-backups}

Der Agent kann [Plakar](https://plakar.io)-Klosets (Repositories) überwachen: Alle zehn Minuten
führt er im Hintergrund `plakar at <kloset> ls` und `info` auf jedem aus und gibt die letzte
Messung in jedem Zyklus erneut aus.

Plakar wird **erkannt, nie vorausgesetzt**. Beim Start und dann in jedem Zyklus, bis er etwas
findet, sucht der Agent das Binary `plakar` (im `PATH` oder `plakar_bin`) und Klosets an den
Standardorten. Gibt es kein Binary, kein Kloset und keine Spur von Plakar (`~/.cache/plakar`),
gibt der Agent gar keine Serien `backup_*` aus, sagt das einmal auf Stufe `debug`, und die
Geräteseite zeigt kein Panel Backups — auf Rechnern ohne Plakar ist nichts zu konfigurieren. Um die
Erkennung ganz zu überspringen:

```yaml
plakar: false                        # or DUMBMONIT_AGENT_PLAKAR=false
```

### Erkennung {#discovery}

Nichts zu konfigurieren: Ist `plakar_klosets` leer, findet der Agent die Klosets selbst, in dieser
Reihenfolge, und liest jeden Ort nur einmal:

1. das eigene Konfigurationsverzeichnis des Agenten — `$XDG_CONFIG_HOME/plakar` oder
   `~/.config/plakar` des Kontos, unter dem er läuft (`plakar_home`, wenn gesetzt);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, alphabetisch;
4. `/var/lib/plakar`, wenn dieses Verzeichnis existiert.

In jedem Verzeichnis liest er `stores.yml` (oder `stores.yaml`; das veraltete `plakar.yml` wird
ebenfalls akzeptiert) und nimmt jeden Eintrag der Map `stores` (auch `repositories` und `klosets`),
der eine `location` hat. Nur die Location wird gelesen: Daneben stehende Passphrasen und
rclone-Geheimnisse werden nie behalten oder protokolliert. Jeder Eintrag wird zum Kloset
`@<name>`, gelesen mit `plakar -configdir <dir> at @<name>`, sodass ein von einem Benutzer erstelltes
Kloset auch gelesen wird, wenn der Agent als root läuft. `~/.plakar` jedes Kontos wird ergänzt,
wenn es existiert (das Standard-Kloset von `plakar create`). Zwei Konten, die verschiedene Klosets
gleich benennen, werden als `@name` und `<user>:@name` unterschieden.

Das Ergebnis wird einmal auf Stufe `info` protokolliert (und erneut nur, wenn es sich ändert). Zwei
Gauges, erst ausgegeben, wenn Plakar erkannt ist, lassen die Oberfläche die übrigen Fälle
unterscheiden: `backup_plakar_present` (1, wenn das Binary läuft) und `backup_klosets_found`
(Anzahl gelesener Klosets). Die Geräteseite sagt dann „Plakar ist installiert, aber es wurde kein
Kloset gefunden“ oder listet die Klosets auf. Existieren Klosets, aber das Binary läuft nicht (ein
Dienst-`PATH` ohne `plakar`, typischerweise), werden sie als nicht lesbar aufgeführt, mit einem
Hinweis, `plakar_bin` zu setzen, und der Agent warnt einmal.

Um stattdessen eine feste Liste zu überwachen, nennen Sie sie — die Erkennung wird dann übersprungen:

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
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — Plakar-Backups überwachen, wenn erkannt (Standard `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Pfad des Binarys `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets, kommagetrennt (überschreibt die Erkennung) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Beim Ausführen von `plakar` verwendetes Home-Verzeichnis |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Sekunden zwischen zwei Messungen |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — Registry-Prüfung auf Container-Updates |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — Aktionen vom Server akzeptieren |

Ein verschlüsseltes Kloset braucht seine Passphrase: Tragen Sie sie im Store-Eintrag ein
(`plakar store set <name> passphrase=…`, was `plakar store add` tut) oder exportieren Sie
`PLAKAR_PASSPHRASE` in der Dienstumgebung des Agenten. Ohne sie liest sich das Kloset als „Kann
nicht geöffnet werden“ (`backup_last_status = 0`).

### Metriken und Regeln {#metrics-and-rules}

Metriken mit den Labels `kloset` und `source` (das gesicherte Verzeichnis):
`backup_last_success_seconds` (Alter des neuesten Snapshots), `backup_snapshot_count`,
`backup_last_status` (1, wenn der neueste Snapshot keinen Fehler hat, 0, wenn er Fehler hat oder das
Kloset nicht geöffnet werden kann), `backup_size_bytes{kloset}` (Speichergröße des Repositorys),
dazu `backup_plakar_present` und `backup_klosets_found`, wie oben beschrieben. Keine davon existiert,
solange Plakar nicht erkannt ist, sodass die folgenden integrierten Regeln auf einem Rechner ohne
Plakar nicht auslösen können.

Integrierte Regeln: **Plakar-Backup zu alt** (Hinweis, seit mehr als zwei Tagen kein Snapshot) und
**Plakar-Backup fehlgeschlagen** (Warnung). Die Geräteseite zeigt einen Block pro Kloset — Alter des
letzten Backups, Snapshot-Anzahl, Größe, letzter Status — und eine Zeile pro Quelle, mit dem
Hinweis „Backup zu alt“ nach mehr als zwei Tagen.

### Ein erstes Backup ausführen {#run-a-first-backup}

Mit auf dem Rechner installiertem Plakar 1.1 (`plakar version`), als der Benutzer, dem die Backups
gehören sollen:

```sh
# 1. a kloset store, named so the agent finds it in ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # or any location Plakar accepts
plakar at @diskext create                                   # add -plaintext to skip encryption

# 2. a first backup, then check it
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` und `plakar at <path> backup <dir>` funktionieren genauso mit einem
schlichten Verzeichnis statt `@name`; `plakar create` ohne `at` verwendet `~/.plakar`, das der Agent
ebenfalls findet. Planen Sie `plakar at @diskext backup /srv/photos` mit Cron oder einem
systemd-Timer: Der Agent meldet das Alter des neuesten Snapshots und alarmiert, wenn ein Tag ohne
einen vergeht — zwei Tage bei der integrierten Regel.

Um es lokal mit einem unverschlüsselten Kloset auszuprobieren:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

Ergänzen Sie dann `plakar_klosets: ["/tmp/plakar-demo"]` in `agent.yaml` (ein bloßer Pfad außerhalb
eines Home wird nicht erkannt) und prüfen Sie mit `dumbmonit-agent --dry-run`, dass Samples
`backup_*` erscheinen.

## restic- und Borg-Backups {#restic-and-borg-backups}

Der Agent kann [restic](https://restic.net)- und [Borg](https://www.borgbackup.org)-Repositories
überwachen: Alle zehn Minuten öffnet er im Hintergrund jedes von Ihnen deklarierte Repository und
meldet, ob es gelesen werden konnte und wie alt sein neuester Snapshot ist. Nichts wird erkannt:
Ein Repository hat keinen Standardort und lässt sich ohne sein Passwort nicht öffnen, daher wird
jedes in `agent.yaml` aufgeführt, mit dem Weg, sein Passwort zu erhalten.

**Das Passwort verlässt den Rechner nie.** Der Agent übergibt es `restic` oder `borg` nur über die
Umgebung des Kindprozesses; es wird nie protokolliert, nie an den Server gesendet, und die Serien
tragen nur den Namen des Repositorys.

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

Halten Sie `agent.yaml` und die Passwortdateien nur für root lesbar (`chmod 600`), wie es der
Installer für `agent.yaml` tut. `password_command` entspricht `RESTIC_PASSWORD_COMMAND` oder
`BORG_PASSCOMMAND`. Alles, was die Werkzeuge aus ihrer Umgebung lesen, kann unter `env` stehen
(`RESTIC_PASSWORD`, `BORG_PASSPHRASE`, Cloud-Schlüssel, `BORG_RSH` …), und die eigene
Dienstumgebung des Agenten wird ebenfalls durchgereicht.

Was der Agent ausführt, im strengsten Sinn nur lesend:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: Es wird keine Sperre geschrieben,
  sodass ein schreibgeschütztes oder Append-only-Repository überwacht werden kann und ein `prune`,
  das die exklusive Sperre hält, das Repository nicht ausgefallen aussehen lässt; auch kein Cache,
  da der installierte Dienst nicht nach `/root` schreiben kann.
- `borg --bypass-lock list --json --last 1`: Ohne `--bypass-lock` würde die Messung für die ganze
  Dauer jedes `borg create` scheitern, das die exklusive Sperre hält. Borg schreibt einen kleinen
  Sicherheitsdatensatz unter `~/.config/borg`; ist dieses Home nicht beschreibbar (der installierte
  Dienst läuft mit `ProtectHome=yes`), verweist der Agent `BORG_BASE_DIR` stattdessen auf sein
  privates `/tmp`, sofern Sie `BORG_BASE_DIR` nicht selbst setzen. Ein Repository im Modus
  `keyfile` braucht dann `BORG_KEY_FILE` in seinem `env`.

Da der Dienst mit `ProtectHome=yes` läuft, halten Sie, was die Werkzeuge brauchen, außerhalb der
Home-Verzeichnisse: Passwortdateien, SSH-Schlüssel und `known_hosts` unter `/etc/dumbmonit/`, wie im
Beispiel, und lokale Repositories unter `/srv`, `/mnt` oder `/var` statt `/home`. Die Messungen
laufen nacheinander, innerhalb des Speicherlimits des Dienstes (128 MiB mit dem Agenten selbst).

Metriken mit den Labels `tool` (`restic` oder `borg`) und `repo`: `agent_backup_repo_reachable` (1,
wenn die letzte Messung erfolgreich war, sonst 0: Repository nicht erreichbar, Passwort abgelehnt,
Binary fehlt — der Agent protokolliert den Grund einmal und erneut, wenn er sich ändert),
`agent_backup_repo_last_snapshot_age_seconds` (Alter des neuesten Snapshots, fehlt, solange das
Repository leer ist) und `agent_backup_repo_last_snapshot_bytes` (vom neuesten restic-Snapshot
verarbeitete Bytes, restic 0.17 und neuer). Mit `--latest 1` liefert restic den neuesten Snapshot
jedes Hosts und Pfadsatzes; der neueste von allen wird behalten, setzen Sie daher `host` bei einem
von mehreren Rechnern geteilten Repository.

Integrierte Regeln: **restic/Borg-Backup zu alt** (seit mehr als 48 Stunden kein Snapshot) und
**restic/Borg-Repository nicht erreichbar** (30 Minuten lang). Die Geräteseite zeigt ein Panel
Backup-Repositories, eine Zeile pro Repository.

Um eine Deklaration zu prüfen, führen Sie den Agenten einmal im Vordergrund aus und achten Sie auf
die Samples `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## WireGuard-Tunnel {#wireguard-tunnels}

Auf einem Rechner, auf dem WireGuard läuft, liest der Agent in jedem Zyklus jede Schnittstelle und
jeden Peer mit `wg show all dump`. Nichts zu konfigurieren: Ohne den Befehl `wg` (wireguard-tools)
oder ohne Schnittstelle wird keine Serie ausgegeben und kein Panel gezeigt. `wg` braucht root
(`CAP_NET_ADMIN`); der installierte Dienst hat es, ein von Hand unter Ihrem eigenen Konto
gestarteter Agent protokolliert die Ablehnung einmal und meldet nichts.

Der Dump enthält auch den privaten Schlüssel der Schnittstelle und den Preshared Key jedes Peers:
Der Agent überspringt beide Felder, ohne sie je zu kopieren.

Pro Peer, mit den Labels `interface`, `peer` (sein öffentlicher Schlüssel) und `allowed_ips`:
`agent_wireguard_peer_handshake_age_seconds` (Sekunden seit dem letzten Handshake),
`agent_wireguard_peer_has_handshake` (0, solange der Peer nie einen abgeschlossen hat),
`agent_wireguard_peer_keepalive_seconds` (sein persistenter Keepalive, 0 wenn aus) und die Zähler
`agent_wireguard_peer_rx_bytes` und `agent_wireguard_peer_tx_bytes`; pro Schnittstelle
`agent_wireguard_interface_peers`. Bei einem Peer, der nie einen Handshake abgeschlossen hat, zählt
das Alter ab dem Moment, in dem der Agent ihn zum ersten Mal sah: Ein Tunnel, der nie hochkam, ist
ebenfalls still und wird als solcher gemeldet.

WireGuard kennt keine Peer-Namen. Geben Sie Ihre in `agent.yaml` an, nach öffentlichem Schlüssel,
und sie werden zum Label `name`:

```yaml
wireguard: true                     # default; false stops even looking
wireguard_bin: wg                   # default: on PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

Die integrierte Regel **WireGuard-Peer still** löst aus, wenn ein Peer **mit persistentem
Keepalive** seit mehr als 15 Minuten keinen Handshake abgeschlossen hat. Solange ein Tunnel steht,
erneuert WireGuard seinen Handshake alle zwei Minuten, und ein Keepalive hält ihn auch ohne Verkehr
oben; ein Peer ohne Keepalive (ein Smartphone, ein Laptop) kann tagelang ruhen, ohne dass etwas
kaputt ist, und bleibt daher außen vor. Setzen Sie `PersistentKeepalive = 25` auf den Tunneln, die
oben bleiben müssen (Site-to-Site-Verbindungen, entfernte Gateways). N ist der Schwellenwert der
Regel: Ändern Sie ihn unter **Alarme → Regeln** oder entfernen Sie den Teil `and on (…)` ihrer
Abfrage, um jeden Peer zu überwachen. Die Geräteseite zeigt ein WireGuard-Panel mit jedem Peer, der
Zeit seit seinem letzten Handshake, seinem Verkehr und seinem Keepalive.

## TLS-Zertifikate (Certbot / Let's Encrypt) {#tls-certificates-certbot-lets-encrypt}

Der Agent liest `/etc/letsencrypt` nicht direkt. Jedes von Certbot erneuerte Zertifikat wird
irgendwo ausgeliefert — von einem Webserver, einem Mailserver, einem Reverse-Proxy — und dort liest
DumbMonit es: Fügen Sie einen [TLS-Zertifikat](services.md#tls)-Monitor hinzu (oder einen
[Website oder Web-API](services.md#http)-Monitor für eine bereits so überwachte Website), der auf
den Port zeigt, auf dem das Zertifikat tatsächlich präsentiert wird. Er liest das Live-Zertifikat,
nicht eine Datei auf der Platte, und fängt daher auch eine Erneuerung ab, die in `/etc/letsencrypt`
gelang, aber vom Dienst nie übernommen wurde (ein Reverse-Proxy, der einen Reload braucht, um die
neue Datei zu sehen, ist der übliche Fall und genau die Lücke, auf die es ankommt).

Das Lesen von `/etc/letsencrypt/live/*/cert.pem`, der Erneuerungskonfiguration und des
Certbot-Logs durch den Agenten wurde erwogen und verworfen: Es würde das Ablaufdatum duplizieren,
das der TLS-Monitor bereits meldet, und einen zweiten, hostlokalen Weg zur selben Information
hinzufügen — eine Sache mehr, die nach dem Verschieben eines Zertifikats oder dem Umbenennen einer
Domain synchron zu halten ist. Ein Host, auf dem Certbot läuft, aber nichts ausliefert, was
DumbMonit direkt erreichen kann (ein interner CA-Schritt, ein DNS-Validierungs-Hook), ist der eine
Fall, den das unabgedeckt lässt; es ist auch der Fall, in dem ein TLS-Monitor ebenfalls nicht
helfen könnte, da auf diesem Host nichts TLS spricht, das man prüfen könnte.
