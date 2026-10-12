# Agent Linux, macOS, FreeBSD e Windows

Un piccolo binario installato sulla macchina da monitorare. Misura CPU,
memoria, dischi, rete, servizi e container — e, dove la macchina li
espone, temperature, salute dei dischi e pool ZFS — poi **invia** le
sue letture al server DumbMonit via HTTP.

La direzione è deliberata: l'agent chiama il server, mai il contrario.
Attraversa NAT e firewall domestici, e non richiede l'apertura di
**nessuna porta** sulla macchina monitorata.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_`. Le etichette di
identità (`target`, `host`, `tag_*`) vengono impostate dal server alla
ricezione, mai dall'agent: una macchina non può scrivere nella serie
di un'altra macchina, nemmeno falsificando le sue etichette.

| Famiglia | Metriche | Etichette |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` solo con `cpu_per_core: true` | `core` |
| Memoria | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| File system | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Rete (contatori) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Dischi (contatori) | `disk_read_bytes`, `disk_written_bytes`, una serie per dispositivo a blocchi | `device` |
| Host | `uptime_seconds`, `process_count` | |
| Servizi | `service_up` (1 = in esecuzione) | `service` |
| Container | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (container oltre `docker_max_containers`) | `container`, `image` |
| Sensori | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (la soglia dichiarata dall'hardware stesso), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Salute dischi (SMART) | `agent_disk_smart_ok` (1 = il disco passa la sua propria autovalutazione), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degraded, 2 peggio), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; per peer `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (contatori) — vedi [Tunnel WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — vedi [Backup restic e Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Contatori di prestazioni Windows | `agent_perf_counter`, una serie per contatore elencato in `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — vedi [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; per VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; per disco virtuale `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — vedi [Host Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Agent | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Una serie che non può essere misurata è **assente**, mai zero: nessun
`smartctl`, nessun `agent_disk_*`; nessuna sonda leggibile, nessun
`agent_sensor_*`; nessun ZFS, nessun `agent_zfs_*`. Una temperatura
fuori da 1-150 °C viene scartata invece che pubblicata, e una lettura
di ventola a 0 rpm viene scartata anch'essa — una scheda madre ha più
connettori per ventole che ventole, e uno vuoto legge zero come fa una
ventola morta.

### Cosa viene raccolto dove

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, memoria, swap, file system, rete, uptime, processi | sì | sì | sì | sì |
| Media di carico | sì | sì | sì | no (Windows non ha questa nozione) |
| Servizi | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | servizi Windows |
| Container Docker | sì | sì, se `docker_socket` punta al socket di Docker Desktop | no (nessun demone Docker) | no |
| Temperature | `/sys/class/hwmon` | sensori SMC | `dev.cpu.N.temperature` (solo CPU) | no |
| Ventole | `/sys/class/hwmon` | no | no | no |
| Salute dischi (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Pool ZFS | `zpool` (OpenZFS) | no | `zpool` | no |
| Salute dell'OS (aggiornamenti, riavvio, unità fallite, SELinux) | sì | no | no | no |
| Backup Plakar | sì | sì | sì | sì |
| Tunnel WireGuard | `wg show` | quando `wg` è installato (non testato) | quando `wg` è installato (non testato) | no |
| Backup restic e Borg | sì | sì | sì | solo restic (non testato; Borg non ha una build Windows) |
| Contatori di prestazioni, code MDaemon, Hyper-V | no | no | no | sì |
| Identità della macchina | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | nome host |
| Modalità relay | sì | sì | sì | sì |

Windows non ha una riga di temperatura di proposito: leggere i sensori
lì significa passare attraverso WMI, con diritti amministrativi, per
valori che la maggior parte delle macchine non espone affatto.
Pubblicare nulla è la risposta onesta.

`smartctl` (smartmontools) e `zpool` vengono cercati a ogni periodo di
lettura e usati quando presenti; nessuno dei due viene installato
dall'agent, e nessuno dei due è richiesto. Entrambi richiedono root —
il servizio che l'installer registra funziona come root, un agent
avviato a mano sotto il tuo proprio account non troverà nulla.

I contatori vengono inviati grezzi, così che un riavvio dell'agent non
produca mai un tasso fittizio; applica `rate()` nelle query.

Le regole integrate che si applicano: Device unreachable, High CPU,
Unusual CPU. "Device unreachable" funziona anche sugli agent: il
server verifica l'età dell'ultimo batch ricevuto e dichiara la
macchina silenziosa dopo tre periodi mancati (almeno 90 secondi).

Altre quattro si attivano solo su macchine che producono la serie
corrispondente, così non richiedono alcuna configurazione e non
possono attivarsi su una macchina senza l'hardware:

| Regola | Si attiva quando |
|---|---|
| **Disk SMART failing** (critical) | l'autovalutazione propria di un disco lo riporta come in fallimento |
| **ZFS pool degraded** (critical) | un pool non è più `ONLINE` per cinque minuti |
| **ZFS scrub found errors** (critical) | l'ultimo scrub ha trovato errori, oppure il pool riporta file permanentemente corrotti |
| **Temperature above critical** (critical) | un sensore supera la soglia critica che **il suo proprio hardware dichiara** — nessun valore è fissato nel codice, 85 °C essendo un'emergenza su un disco e un normale pomeriggio su una CPU di laptop |

Altre tre osservano ciò che dichiari o esegui tu stesso, e restano
silenziose altrimenti: **WireGuard peer silent** (un peer con un
keepalive persistente e nessun handshake da 15 minuti), **restic/Borg
backup too old** (nessuno snapshot da 48 ore) e **restic/Borg
repository unreachable** — vedi
[Tunnel WireGuard](#wireguard-tunnels) e
[Backup restic e Borg](#restic-and-borg-backups).

Su un server Windows che esegue MDaemon, altre tre osservano le sue
code: **MDaemon mail queue growing**, **MDaemon Bad queue not empty**
e **MDaemon Retry queue high** (vedi
[Regole](../alerting/rules.md#mdaemon-and-securitygateway)). Su un
host Hyper-V, tre osservano le macchine virtuali: **Hyper-V VM health
critical**, **Hyper-V host CPU high** e **Hyper-V virtual disk
errors** (vedi [Host Hyper-V](#hyper-v-hosts)).

## Installazione

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo.
Non c'è alcun account da creare sulla macchina: il token di iscrizione
è il solo segreto.

1. Salva il dispositivo in DumbMonit: un token di iscrizione (dmon_…) viene mostrato una volta, con il comando di installazione pronto da copiare per Linux e per Windows. (Un token può anche essere creato in **Settings → Agents**.)
2. Quel token è la chiave dell'agent per inviare le sue misurazioni a DumbMonit. Non è un account sulla macchina: nulla da creare lì. Un token creato dal modulo del dispositivo è a uso singolo: iscrive questa unica macchina e nient'altro. Per una flotta, crea un token riutilizzabile in Settings → Agents.
3. Esegui il comando di installazione sulla macchina da monitorare, con diritti elevati (sudo su Linux e macOS, un PowerShell elevato su Windows). Scarica l'agent, scrive il token in agent.yaml e avvia il servizio.
4. La macchina appare da sola entro pochi secondi, nominata secondo il suo nome host. A quel primo batch il server dà all'agent un segreto proprio e lo memorizza in `/etc/dumbmonit/agent-secret`, leggibile da nessun altro: da quel momento, quella macchina è la sola che può riportare sotto questo dispositivo. Hai perso il token? Settings → Agents ti permette di revocarlo e crearne un altro.
5. Docker: per vedere i container e lasciare che DumbMonit li riavvii o aggiorni, l'agent deve raggiungere il socket Docker. Il servizio che l'installer registra già può; se esegui l'agent sotto un utente dedicato invece, aggiungi quell'utente al gruppo "docker" e riavvialo. Le policy di riavvio e auto-aggiornamento vengono allora impostate per container sulla pagina del dispositivo.

Due cose vale la pena leggere prima di distribuire questo a più di una
macchina: [Binding](#binding-one-machine-one-agent), che è ciò che fa
appartenere davvero un dispositivo a un'installazione agent, e
[Uso singolo o flotta](#single-use-or-fleet), che è la scelta che fai
quando crei un token.

    ```
    usermod -aG docker dumbmonit
    ```

6. Backup Plakar: rilevati automaticamente — il pannello Backups appare quando il binario plakar o un kloset (~/.config/plakar/stores.yml di ogni utente, ~/.plakar, /var/lib/plakar) viene trovato, e nulla viene mostrato altrimenti. Imposta "plakar_klosets" in agent.yaml per osservare un elenco fisso, oppure "plakar: false" per disattivare.

!!! note
    L'agent contatta il server, mai il contrario: nessuna porta deve
    essere aperta sulla macchina monitorata.

=== "Linux (systemd o OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Lo script è `sh` POSIX (BusyBox e dash funzionano), scarica il
    binario per l'architettura della macchina (x86_64 o aarch64) dal
    server, scrive `/etc/dumbmonit/agent.yaml` (modo 0600) e registra
    `dumbmonit-agent` con systemd, oppure con OpenRC su Alpine.
    L'unità systemd è irrobustita (`ProtectSystem=strict`,
    `NoNewPrivileges`, `MemoryMax=128M`, `CPUQuota=20%`).

=== "macOS (launchd)"

    Lo stesso comando di Linux, eseguito con `sudo`:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    I binari macOS **non** sono nell'immagine DumbMonit: costruirli
    richiede l'SDK di Apple, la cui licenza ne proibisce la
    redistribuzione. Sono invece allegati a ogni
    [release GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
    — `dumbmonit-agent-macos-aarch64` (Apple silicon) e
    `dumbmonit-agent-macos-x86_64` (Intel), ciascuno con la propria
    `.sha256` — e il server reindirizza
    `/download/dumbmonit-agent-macos-…` lì, così lo script scelgie
    quello per l'architettura del Mac (`uname -m`). Il server non
    fornisce quei binari, quindi il comando di installazione non porta
    alcun checksum per essi: aggiungi
    `--sha256=macos-aarch64:<hex>` (oppure `macos-x86_64`) con il
    valore della `.sha256` della release, altrimenti l'installer
    rifiuta il download. Il Mac ha anche bisogno di raggiungere
    github.com.

    Senza quell'accesso, oppure per installare un binario che hai già,
    scaricalo e passalo con `--bin` (confrontalo prima con la `.sha256`
    della release: `--bin` installa ciò che gli dai):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    Lo script installa il binario in `/usr/local/bin` (e rimuove
    l'attributo di quarantena che un download da browser porta, che
    farebbe rifiutare a Gatekeeper il suo avvio), scrive
    `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e registra il
    demone di sistema `com.dumbmonit.agent` in
    `/Library/LaunchDaemons`, avviato con `launchctl bootstrap
    system`. Funziona al boot, senza nessuno connesso.

    !!! note "Preferisci costruirlo tu stesso?"
        Sul Mac stesso, con Rust installato:
        `cargo build --release -p dumbmonit-agent`, poi
        `--bin=target/release/dumbmonit-agent`. È lo stesso binario che
        produce il job di release.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Solo x86_64. Lo script scarica `dumbmonit-agent-freebsd-x86_64`
    dal server, scrive `/usr/local/etc/dumbmonit/agent.yaml` (modo
    0600) e installa `/usr/local/etc/rc.d/dumbmonit_agent`, abilitato
    con `service dumbmonit_agent enable`. `daemon(8)` supervisiona
    l'agent, lo riavvia se si ferma e scrive
    `/var/log/dumbmonit-agent.log`.

    Funziona nello stesso modo su TrueNAS CORE e altre appliance basate
    su FreeBSD, dove le serie ZFS sono il punto dell'esercizio.

=== "Windows (servizio)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    Lo script scarica `dumbmonit-agent-windows-x86_64.exe` dal server,
    lo verifica contro il suo SHA-256, lo installa come
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, scrive
    `C:\ProgramData\DumbMonit\agent.yaml` (leggibile solo da SYSTEM e
    Administrators) e registra il servizio Windows `DumbMonitAgent`
    ("DumbMonit system agent"): avviato automaticamente al boot, come
    LocalSystem, e riavviato dopo un fallimento (10 s, 30 s, poi
    60 s). L'agent parla con il gestore di servizi stesso
    (`--service`), così fermare il servizio gli permette di inviare
    ciò che ancora mantiene prima di uscire.

    Un servizio non ha una console: l'agent scrive il suo log in
    `C:\ProgramData\DumbMonit\agent.log`, messo da parte come
    `agent.log.old` oltre 10 MB.

    Non c'è una build ARM64 nativa: su Windows 11 per ARM, lo script
    installa il binario x86_64, che funziona sotto emulazione x64.

Un token può iscrivere diverse macchine: nominalo secondo un gruppo o
una macchina.

### Binari nativi

L'agent è un singolo eseguibile statico per sistema, con nulla da
installare accanto ad esso:

| Sistema | File | Dove trovarlo |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | il server, `/download/…` |
| Linux aarch64 (ARM 64 bit: Raspberry Pi OS 64 bit, server ARM) | `dumbmonit-agent-linux-aarch64` | il server, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | il server, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | il server, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | la release GitHub (il server reindirizza lì) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | la release GitHub (il server reindirizza lì) |

I binari Linux sono collegati staticamente con musl: funzionano su
qualsiasi distribuzione, glibc o no (Alpine, firmware NAS), senza
alcuna libreria da far corrispondere. Ogni file ha il suo checksum
accanto, `<file>.sha256`, nel formato `sha256sum`. Ogni
[release GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
porta tutti e sei i binari e i loro checksum, per macchine che non
possono raggiungere il `/download/` del server o per installare con
`--bin`.

Per costruire tu stesso i binari Linux, Windows e FreeBSD, da un clone
del repository (Docker è il solo requisito):

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Gestire il servizio

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Stato | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Log | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Riavvio | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Arresto | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Configurazione | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| File del servizio | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | servizio `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` su Windows) rimuove tutto, servizio
incluso, su ogni sistema.

!!! note "Il download viene verificato — sempre"
    Il comando di installazione mostrato da DumbMonit porta lo SHA-256
    atteso di ogni binario agent che il server fornisce
    (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` su Windows). Entrambi gli installer
    verificano il file scaricato contro di esso prima di installare
    qualcosa, e **rifiutano** di installare quando non corrisponde,
    quando nessun checksum è dato per la piattaforma, oppure quando
    nessuno strumento SHA-256 (`sha256sum`, `shasum`, `sha256`) è
    disponibile.

    I checksum cambiano a ogni release del server: dopo aver
    aggiornato DumbMonit, copia un comando di installazione fresco da
    **Settings → Agents** (oppure da un nuovo token) invece di
    rieseguire uno vecchio, che si fermerebbe con "checksum mismatch".
    Gli stessi checksum sono mostrati sotto il comando di
    installazione, e pubblicati su `/download/<file>.sha256`.

    Per configurazioni air-gapped o manuali,
    `--insecure-skip-checksum` (`-InsecureSkipChecksum`) installa un
    binario scaricato senza verificarlo e lo dice ad alta voce;
    `--bin=PATH` installa un file locale così com'è, il file essendo
    tuo da verificare.

### Flag dell'installer

| `install.sh` | `install.ps1` | Effetto |
|---|---|---|
| `--token=TOKEN` | `-Token` | Token di iscrizione (richiesto). |
| `--url=URL` | `-Url` | URL del server, per esempio `http://server:8080`. Su Linux, `DUMBMONIT_URL` nell'ambiente viene usata se il flag è assente. |
| `--interval=N` | `-Interval` | Periodo di campionamento in secondi (predefinito 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Servizi il cui stato viene riportato, nominati come il sistema host li nomina: unità systemd su Linux, etichette launchd (`com.apple.sshd`) su macOS, nomi rc.d su FreeBSD, servizi Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Tag, copiati come `tag_<key>` su ogni serie. |
| `--hostname=NAME` | `-HostName` | Nome annunciato al server (predefinito: quello della macchina). |
| `--bin=PATH` | `-BinPath` | Binario locale da installare invece di scaricarlo. |
| `--sha256=LIST` | `-Sha256` | SHA-256 atteso del download, come coppie `platform-arch:hex` separate da virgole (un valore hex semplice funziona anch'esso). Incluso nel comando che mostra DumbMonit; richiesto a meno che non sia dato il flag successivo. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Installa un binario scaricato senza verificarlo, con un avviso. Solo per configurazioni air-gapped o manuali. |
| `--no-start` | | Installa tutto, ma non contatta il server e non avvia il servizio (immagine macchina, test). |
| `--uninstall` | `-Uninstall` | Disinstalla l'agent ed elimina la sua configurazione. |
| `--help` | | Mostra l'aiuto. |

### Aggiornamento

Entrambi gli script sono idempotenti: eseguire di nuovo il comando di
installazione, con lo stesso token e URL, sostituisce il binario,
riscrive la configurazione e riavvia il servizio. Quella è la
procedura di aggiornamento. Prendi il comando dal server attuale
(Settings → Agents): il suo `--sha256` deve corrispondere ai binari
della versione del server a cui hai aggiornato.

Lo stesso comando migra una macchina che esegue ancora l'agent
EzyMonit, su Linux e su Windows. L'installer ferma e rimuove il
vecchio servizio (`ezymonit-agent`, oppure `EzyMonitAgent` su
Windows), sposta la configurazione al nuovo percorso
(`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`) e
installa `dumbmonit-agent` al suo posto. Il vecchio token `ezym_…`
resta valido, così il comando mostrato quando il token è stato creato
funziona ancora; nulla deve essere revocato prima.

## File di configurazione

`/etc/dumbmonit/agent.yaml` su Linux,
`/usr/local/etc/dumbmonit/agent.yaml` su macOS e FreeBSD,
`C:\ProgramData\DumbMonit\agent.yaml` su Windows:

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # periodo di campionamento
hostname: nas-basement     # opzionale: nome annunciato al server
services:                  # unità systemd o servizi Windows
  - sshd
  - docker
tags:                      # etichette libere, con prefisso "tag_" sul server
  role: nas
  room: basement
docker: true               # inventario container (predefinito: true)
docker_socket: /var/run/docker.sock
sensors: true              # temperature e ventole (predefinito: true)
smart: true                # salute dei dischi tramite smartctl (predefinito: true)
smart_bin: smartctl        # percorso, quando non è nel PATH del servizio
smart_interval_secs: 300   # minimo 60: un disco in sospensione non viene mai risvegliato per essere letto
zfs: true                  # pool ZFS tramite zpool (predefinito: true)
zfs_bin: zpool
zfs_interval_secs: 120     # minimo 30
wireguard: true            # tunnel WireGuard tramite wg (predefinito: true), vedi sotto
restic_repos: []           # repository restic da osservare, vedi "backup restic e Borg"
borg_repos: []             # repository Borg da osservare
mdaemon: auto              # Windows: code MDaemon quando il suo servizio esiste (true / false / auto)
hyperv: auto               # Windows: contatori Hyper-V quando il servizio vmms esiste (true / false / auto)
perf_counters:             # Windows: contatori di prestazioni da pubblicare
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # valore dell'etichetta "counter"; il percorso quando omesso
max_buffered_samples: 20000
log_level: info
```

`sensors`, `smart` e `zfs` vengono rilevati, mai presunti: lasciati a
`true` su una macchina senza alcuna sonda, nessun `smartctl` o nessun
pool, costano un'occhiata per periodo e non emettono nulla affatto.
Imposta uno a `false` per fermare anche il guardare — da fare su un
NAS i cui dischi preferiresti che `smartctl` non toccasse mai, anche
se l'agent già passa `-n standby` così che un disco in sospensione
venga misurato solo quando è svegliato per altri motivi.

Accanto, `/etc/dumbmonit/agent-secret` (`C:\ProgramData\DumbMonit\agent-secret`
su Windows) contiene il segreto di binding che il server ha dato a
questa macchina. L'agent lo scrive con modo `0600` e lo rilegge a ogni
avvio; non fa parte di `agent.yaml` perché non è qualcosa che
imposti — vedi [Binding](#binding-one-machine-one-agent).
`DUMBMONIT_AGENT_SECRET_FILE` lo sposta, che è ciò che vuoi per un
agent in un container.

Ogni chiave può essere sovrascritta dall'ambiente, il che rende
l'agent usabile in un container senza montare un file:

| Variabile | Effetto |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Percorso del file di configurazione |
| `DUMBMONIT_AGENT_URL` | URL del server |
| `DUMBMONIT_AGENT_TOKEN` | Token di iscrizione |
| `DUMBMONIT_AGENT_SECRET_FILE` | Dove conservare il segreto di binding (predefinito: `agent-secret`, accanto al file di configurazione) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Periodo di campionamento |
| `DUMBMONIT_AGENT_HOSTNAME` | Nome annunciato al server |
| `DUMBMONIT_AGENT_SERVICES` | Servizi da osservare, separati da virgole |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, separati da virgole |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Percorso del socket Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Container dettagliati per host, predefinito `200` (`0`: solo conteggi) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfacce escluse, nomi o regex separati da virgole; predefinito `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfacce da mantenere; quando impostata, sostituisce l'elenco di esclusione |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Punti di mount esclusi da file system e I/O disco; predefinito salta `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` per inviare anche una serie CPU per core (predefinito `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: temperature e ventole (predefinito `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: salute dei dischi tramite `smartctl` (predefinito `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Percorso di `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Periodo tra due letture SMART, predefinito `300`, minimo `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: pool ZFS tramite `zpool` (predefinito `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Percorso di `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Periodo tra due letture ZFS, predefinito `120`, minimo `30` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: tunnel WireGuard tramite `wg` (predefinito `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Percorso di `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Percorso di `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Percorso di `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Periodo tra due letture dei repository restic e Borg, predefinito `600`, minimo `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` o `auto`: i contatori di prestazioni di MDaemon (predefinito `auto`, letti quando il servizio Windows `MDaemon` esiste) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` o `auto`: i contatori di prestazioni Hyper-V (predefinito `auto`, letti quando il servizio Windows `vmms` esiste) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Percorsi dei contatori di prestazioni Windows, separati da **punto e virgola** (una virgola può far parte di un nome di istanza); sostituisce `perf_counters` |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Dimensione del buffer di recupero |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` per eseguire, per il server, le sonde dei dispositivi assegnati a questo agent (vedi [Eseguire l'agent in Docker / su un'altra rete](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Etichetta del sito mostrata accanto all'agent quando viene offerto come relay |

## Contatori di prestazioni Windows

Su Windows, l'agent legge i contatori di prestazioni — i valori che
mostra Performance Monitor (`perfmon`) — tramite la libreria PDH, e li
pubblica a ogni periodo di campionamento. Alcuni software non
pubblicano il loro stato altrove.

**MDaemon.** Quando il servizio Windows chiamato `MDaemon` esiste,
l'agent legge l'oggetto prestazioni MDaemon: dimensioni delle code,
sessioni, totali di messaggi e spam, lo stato dei server interni di
MDaemon e il suo uptime. Le serie, le regole e la pagina che le
mostra sono descritte sulla pagina [MDaemon](mdaemon.md#mail-queues-through-the-agent).
`mdaemon: true` le legge anche senza il servizio (un servizio
rinominato), `mdaemon: false` non lo fa mai. La presenza del servizio
viene verificata di nuovo ogni dieci periodi, così un MDaemon
installato dopo l'agent viene rilevato senza un riavvio.

**Hyper-V.** Quando il servizio Hyper-V Virtual Machine Management
(`vmms`) esiste, l'agent legge gli oggetti prestazioni di Hyper-V;
vedi [Host Hyper-V](#hyper-v-hosts). `hyperv: true` e `hyperv: false`
forzano la scelta, come per MDaemon.

**Qualsiasi altro contatore.** Elenca il suo percorso completo sotto
`perf_counters`, come `\Object(Instance)\Counter` oppure
`\Object\Counter` per un oggetto con una sola istanza, e viene
pubblicato come `dumbmonit_agent_perf_counter{counter="…"}`, con il
nome che dai o il percorso stesso:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Scrivi i nomi dei contatori **in inglese**, come appaiono in
  Performance Monitor su un Windows in inglese: l'agent li aggiunge
  con `PdhAddEnglishCounterW`, così lo stesso `agent.yaml` funziona su
  un Windows francese o tedesco.
- Metti ogni percorso tra virgolette singole in YAML, così che le
  barre rovesciate e il `%` vengano mantenuti come sono.
- Un percorso malformato ferma l'agent all'avvio con la chiave e il
  motivo. Un percorso che Windows non conosce (il software non è
  installato, il nome è scritto male) viene segnalato una volta nel
  log, `performance counter not available`, e gli altri contatori
  vengono comunque letti; viene riprovato ogni dieci periodi.
- I tassi (`…/sec`) richiedono due letture: appaiono dal secondo
  periodo in poi.
- Al massimo 200 contatori vengono letti. Su Linux, macOS e FreeBSD
  l'impostazione viene accettata e nulla viene letto.

Per vedere i nomi esatti sulla macchina, esegui
`typeperf -q MDaemon` (oppure qualsiasi altro nome di oggetto) in un
prompt dei comandi.

## Host Hyper-V

Installato su un host Hyper-V — Windows Server con il ruolo Hyper-V,
oppure Windows con Hyper-V abilitato — l'agent trova il servizio
`vmms` e legge i contatori di prestazioni propri di Hyper-V, senza
nulla da configurare. La pagina del dispositivo dell'agent mostra
allora un pannello Hyper-V: quante macchine virtuali riportano sane,
il carico CPU reale dell'host, la memoria che ogni VM in esecuzione
mantiene, e il throughput, la latenza e gli errori di ogni disco
virtuale.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    I nomi dei contatori provengono dalla documentazione delle
    prestazioni Hyper-V di Microsoft e sono stati verificati contro la
    logica con una fonte di contatori simulata, non su un host
    Hyper-V in funzione. Un contatore che Windows non conosce viene
    segnalato una volta nel log dell'agent e gli altri vengono
    comunque letti. Facci sapere cosa non funziona.

| Contatore | Serie | Etichette |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: macchine virtuali in ogni stato | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: il carico dell'host, guest inclusi | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: la quota spesa eseguendo i guest | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: VM in esecuzione più l'host stesso | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: domanda di memoria sulla memoria assegnata; sopra 100 la VM vuole più di quanto ha | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (media), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, un contatore | `disk` |

Su un host Hyper-V, `\Processor(_Total)\% Processor Time` misura solo
la partizione propria dell'host, non ciò che le macchine virtuali
consumano: il contatore del processore logico dell'hypervisor è
quello che dice se la macchina è saturata. `vm` è il nome della
macchina virtuale come lo mostra Hyper-V Manager; `disk` è il nome di
istanza che Windows dà a ogni disco virtuale, derivato dal suo
percorso di file. Le istanze `_Total` vengono escluse, e al massimo
200 VM o dischi vengono letti per contatore.

Tre [regole integrate](../alerting/rules.md#hyper-v) si applicano:

- **Hyper-V VM health critical**: Hyper-V riporta una VM in stato di
  salute critico per cinque minuti (Warning) — più spesso un disco
  che ha esaurito lo spazio.
- **Hyper-V host CPU high**: i processori logici occupati per più del
  90% per quindici minuti (Advisory).
- **Hyper-V virtual disk errors**: un disco virtuale ha riportato
  errori di I/O nell'ultima ora (Advisory).

## Eseguire l'agent in Docker / su un'altra rete

L'agent è pubblicato anche come immagine container, con gli stessi tag
del server: `ghcr.io/laupernoe/dumbmonit-agent:latest` (ultima
release), `:edge` (ultimo commit su `main`). È un'immagine
`FROM scratch` che contiene il binario agent e il bundle CA di
Mozilla, configurata interamente tramite variabili d'ambiente. Il
repository fornisce un `docker-compose.agent.yml` pronto all'uso:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

Cosa vede il container, e cosa montare:

| Bisogno | Cosa fare | Perché |
|---|---|---|
| CPU, memoria, carico, I/O disco dell'host | Nulla | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` e `/proc/diskstats` sono a livello host dentro qualsiasi container. |
| Nome host | `DUMBMONIT_AGENT_HOSTNAME` | L'hostname di un container è un id casuale; il dispositivo verrebbe nominato secondo esso. |
| Container dell'host | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Opzionale. Elenchi e inspect di sola lettura; le azioni Restart/Update richiedono che sia scrivibile e `DUMBMONIT_AGENT_COMMANDS=true`. Senza il socket, l'agent semplicemente non riporta container. |
| Utilizzo del file system | `- /:/host:ro` (oppure i mount di cui ti importa) | L'agent riporta i mount visibili *dentro* il container, sotto il loro percorso container. |
| Contatori di rete dell'host | `network_mode: host` | Le interfacce sono in namespace: senza di esso l'agent vede un solo `veth`. |
| Monitor ping relayati tramite questo agent | `cap_add: [NET_RAW]` | ICMP richiede un socket raw. |
| CA privata | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Server dietro un reverse proxy interno, dispositivi con certificati autofirmati. |
| Mantenere il binding attraverso le ricreazioni | `- agent-state:/var/lib/dumbmonit-agent` e `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | Il segreto di binding vive nel file system del container, che un `docker compose up --force-recreate` scarta. Senza il volume, l'agent ritorna non bound e qualcuno deve consentire la ri-iscrizione dalla pagina del dispositivo ogni volta. |

`pid: host` e `privileged: true` non sono mai necessari: l'agent non
raccoglie alcuna metrica per processo.

### Modalità relay: monitorare un altro sito

Un agent può fare di più che osservare la propria macchina. Con
`relay: true` (oppure `DUMBMONIT_AGENT_RELAY=true`), il server
**delega le sonde** ad esso: i dispositivi la cui impostazione
*Reached through* nomina questo agent non vengono sondati dal server
ma dall'agent, dalla sua propria rete, con esattamente gli stessi
collector — SNMP, Proxmox VE, Proxmox Backup Server, Synology, HTTP,
TCP, DNS, ping, TLS. I risultati finiscono sotto il dispositivo come
se il server lo avesse sondato: stesse metriche, stessa *ultima
sonda*, stessi avvisi. Un ufficio remoto, un secondo homelab o la rete
di un cliente appaiono quindi nell'unico DumbMonit che già hai, senza
una VPN e senza nulla da aprire dal lato remoto.

Come configurarlo, passo per passo, è in
[Monitorare una sede remota](../install/remote-site.md).

## Binding: una macchina, un agent

Un token di iscrizione dice *questa macchina può parlare*. Non dice
*quale* macchina sta parlando — è a questo che serve la chiave di
identità (`/etc/machine-id`, oppure il nome host), e quella chiave non
è un segreto: può essere letta su qualsiasi macchina in pochi secondi.

Quindi al primo batch il server dà a ogni macchina un **segreto di
binding** proprio. Mantiene solo un'impronta digitale; l'agent scrive
il segreto in `/etc/dumbmonit/agent-secret` con modo `0600` e lo
presenta nell'header `X-DumbMonit-Agent-Secret` su ogni richiesta
successiva — misurazioni, comandi container e sonde relayate allo
stesso modo. Da quel momento:

- nessun'altra macchina può inviare misurazioni sotto questo
  dispositivo, o riscrivere il suo nome host e sistema operativo;
- nessun'altra macchina può recuperare — e quindi consumare — i
  comandi Restart e Update accodati per esso.

La pagina del dispositivo mostra dove si trova una macchina, sotto
**Agent → Binding**:

| Stato | Cosa significa | Cosa fare |
|---|---|---|
| **Bound** | L'agent mantiene un segreto proprio. | Nulla. |
| **Not bound — re-enrol this host** | Un host iscritto prima che il binding esistesse, e mai bound da allora. Il server rifiuta le sue misurazioni, comandi container e sonde relayate. | Clicca **Allow re-enrolment** sulla pagina del dispositivo, poi riavvia l'agent. Se l'agent è precedente al binding, esegui di nuovo il comando di installazione sull'host prima. |

### Host precedenti al binding

!!! warning "Cambiato: gli host non bound vengono rifiutati"
    Le release precedenti continuavano ad accettare host iscritti
    prima che esistesse il binding, purché usassero il token che li
    aveva iscritti, e li bindavano silenziosamente una volta che il
    loro agent veniva aggiornato. Quella finestra di transizione è
    chiusa: un host che non si è mai bound viene ora **rifiutato su
    ogni rotta agent** — misurazioni, comandi container e sonde
    relayate — finché un amministratore non lo ri-iscrive. Un token
    flotta è condiviso da molte macchine e la chiave di identità può
    essere indovinata, quindi nulla altro prova che l'agent che chiede
    sia quello proprio dell'host.

    Anche un binario agent troppo vecchio per ricevere un segreto di
    binding viene rifiutato, inclusa la sua prima iscrizione.

Per riportare un tale host:

1. Se il suo agent è precedente al binding, esegui di nuovo il comando di installazione su di esso (il binario agent deve essere in grado di memorizzare un segreto di binding).
2. Apri la sua pagina del dispositivo e clicca **Agent → Allow re-enrolment**. Questo apre una finestra di un'ora.
3. Riavvia l'agent, oppure attendi il suo prossimo tentativo: quel batch binda l'host con un segreto fresco e chiude la finestra. Un token diverso da quello che ha iscritto l'host deve poter ancora iscrivere (non esaurito, non scaduto).

Finché non lo fai, il log dell'agent porta la spiegazione del server:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Ri-iscrizione dopo una reinstallazione

Se la macchina viene ricostruita, il suo disco sostituito, oppure il
suo container agent ricreato senza il volume che contiene il segreto,
l'agent ritorna senza di esso e il server rifiuta i suoi batch. Quel
rifiuto è deliberato: una macchina che è bound resta bound finché un
umano non dice il contrario, altrimenti il binding non proteggerebbe
nulla.

La via di rientro è sulla pagina del dispositivo: **Agent → Allow
re-enrolment**. Apre una finestra di un'ora durante la quale un agent
che presenta un token di iscrizione valido binda di nuovo quella
macchina, con un segreto fresco; il vecchio segreto smette di
funzionare, e la finestra si chiude non appena viene usata. L'agent
ritenta da solo nel frattempo, così non c'è nulla da riavviare sulla
macchina — e nulla viene perso, poiché le sue letture restano
bufferizzate finché il collegamento non viene accettato di nuovo.

Finché qualcuno non apre quella finestra, il log dell'agent dice cosa
fare:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Token di iscrizione

- I token vengono creati in **Settings → Agents** oppure aggiungendo
  un dispositivo di tipo agent. Il server mantiene solo un'impronta:
  il token in chiaro viene mostrato una volta, nella risposta di
  creazione, con i due comandi di installazione.
- L'agent lo invia come `Authorization: Bearer dmon_…` su
  `POST /api/ingest`. Non appare mai nei log dell'agent, nemmeno
  troncato.
- **Settings → Agents** elenca i token con il loro prefisso, scope,
  data di creazione e ultimo uso. **Revoke** ferma ogni agent che usa
  quel token al suo prossimo push; le macchine diventano allora
  *unreachable* dopo tre periodi mancati. Esegui di nuovo l'installer
  con un nuovo token per ri-iscriverle.
- Le macchine sono dispositivi separati: revocare un token non le
  elimina.

### Uso singolo o flotta

Lo scope di un token viene scelto alla sua creazione, ed è una scelta,
non un predefinito che ti capita:

| Scope | Cosa iscrive | Quando |
|---|---|---|
| **Single use** (il predefinito) | Esattamente una macchina. | Un'installazione. Un comando di installazione vive nella cronologia della shell, in un ticket, in un log di chat; uno a uso singolo non vale nulla una volta usato. |
| **Reusable for a fleet** | Quante macchine consenti — un numero, o nessun limite. | Un playbook, un'immagine macchina, un lotto di installazioni. |

Entrambi possono portare una scadenza in giorni. Una scadenza ferma
solo le *iscrizioni*: le macchine già iscritte continuano a riportare
dopo che passa, che è ciò che vuoi per una finestra di installazione.
Revocare è ciò che tronca una flotta.

Un token che non può più iscrivere — esaurito, oppure oltre la sua
scadenza — viene mostrato come **Enrols no more** in Settings →
Agents. Le sue macchine continuano a riportare; semplicemente non può
farne entrare una nuova.

## Interruzioni e diagnostica

Quando il server è irraggiungibile, l'agent continua a misurare e
bufferizza le sue letture in memoria con i loro timestamp originali.
Vengono inviate non appena il collegamento ritorna: un riavvio del
server non lascia alcun buco nei grafici. Il buffer è limitato
(`max_buffered_samples`, circa un'ora per impostazione predefinita) e
scarta le letture più vecchie quando pieno. I tentativi fanno backoff
da 5 secondi a 5 minuti con jitter.

```sh
dumbmonit-agent --dry-run          # stampa le letture, non invia nulla
dumbmonit-agent --once             # invia un batch poi esce
journalctl -u dumbmonit-agent -f   # log del servizio (systemd)
tail -f /var/log/dumbmonit-agent.log   # log del servizio (OpenRC)
```

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| L'installer dice "could not reach the server" | URL o token errati. La configurazione viene scritta: correggi `agent.yaml`, poi riavvia il servizio. |
| Il download fallisce con 404 | Il server non ha binari agent (`DUMBMONIT_AGENT_DIR` vuota). Usa `--bin=PATH` con un binario che hai costruito. |
| L'installer si ferma con "checksum mismatch" | Il comando è stato copiato prima di un aggiornamento del server (copiane uno fresco da Settings → Agents), oppure il binario ricevuto non è quello che il server serve: un proxy o una cache nel mezzo, oppure un download manomesso. |
| L'installer si ferma con "no expected checksum" | Il comando non ha `--sha256` per questa piattaforma: un comando vecchio, oppure un binario macOS (non fornito dal server). Copia un comando fresco, oppure passa `--sha256=<platform>-<arch>:<hex>` dalla `.sha256` della release. |
| La macchina non appare mai | Il push va a `/api/ingest` sull'URL nel comando di installazione; dietro un reverse proxy, assicurati che sia inoltrato e che corpi fino a 16 MB siano consentiti. |
| *Unreachable* anche se l'agent funziona | Il token è stato revocato, oppure i push vengono rifiutati. Controlla `journalctl -u dumbmonit-agent`: l'errore viene registrato lì. |
| Il log dice "this agent is not recognised for this machine" | La macchina è bound a un'installazione agent di cui questa non ha il segreto — una reinstallazione, oppure un container ricreato senza il suo volume di stato. Apri la pagina del dispositivo e clicca **Allow re-enrolment**; vedi [Ri-iscrizione dopo una reinstallazione](#re-enrolment-after-a-reinstall). |
| Il log dice "this machine was enrolled before agent binding" | Un host che non si è mai bound al suo agent. Clicca **Allow re-enrolment** sulla sua pagina del dispositivo, poi riavvia l'agent; vedi [Host precedenti al binding](#hosts-that-predate-binding). |
| Il log dice "this agent is too old to be bound to its machine" | Il binario agent non può memorizzare un segreto di binding. Esegui di nuovo il comando di installazione sull'host; se il dispositivo esiste già, clicca prima **Allow re-enrolment**. |
| Il log dice "this enrollment token has already enrolled all the machines it was allowed to" | Un token a uso singolo riutilizzato. Creane uno nuovo, oppure un token riutilizzabile in Settings → Agents. |
| Windows | Le parti del servizio Windows non erano state esercitate nell'immagine di build del progetto al momento della scrittura; segnala problemi su GitHub. |

## Container Docker

Quando l'agent può leggere il socket Docker (`docker: true`, il
predefinito, e un utente nel gruppo `docker` oppure root —
`usermod -aG docker dumbmonit`, poi riavvia l'agent), ogni container
diventa una manciata di serie, etichettate `container` e `image` (il
`name:tag` mostrato da `docker ps`). Se la pagina del dispositivo dice
"No Docker on this machine" mentre Docker è installato, quella è la
prima cosa da controllare.

| Metrica | Significato |
|---|---|
| `container_up` | 1 in esecuzione, 0 altrimenti |
| `container_health` | 0 nessun healthcheck, 1 sano, 2 non sano, 3 in avvio |
| `container_restart_count` | Il contatore di riavvii di Docker per il container |
| `container_started_seconds` | Secondi dall'ultimo avvio del container (0 quando fermo) |
| `container_image_age_seconds` | Secondi da quando l'immagine è stata costruita |
| `container_update_available` | 1 quando il registry ha un'immagine più recente per lo stesso tag, 0 quando aggiornato, -1 sconosciuto |

Il controllo aggiornamenti confronta il digest dell'immagine locale
con quello che il registry annuncia per il tag
(`HEAD /v2/<name>/manifests/<tag>`, flusso di token anonimo su Docker
Hub e GHCR). Funziona in background al massimo una volta all'ora per
immagine e non ritarda mai il ciclo di 30 secondi; un registry privato
che rifiuta l'accesso anonimo semplicemente legge come -1. Disattivalo
con `docker_update_check: false`
(`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Regole integrate: **Container stopped** (`container_up == 0` per 2
minuti), **Container unhealthy** (`container_health == 2` per 3
minuti), **Container restarting** (tre o più riavvii in 15 minuti),
**Container update available** (info, dopo un'ora, ripetuta ogni
giorno). L'avviso nomina il container.

### Azioni

Sulla pagina del dispositivo, ogni container ha due azioni e due
interruttori. Le azioni viaggiano attraverso un **canale di comando**:
il server accoda il comando, l'agent lo recupera dopo il suo batch
successivo (`GET /api/agent/commands`, stesso token di iscrizione), lo
esegue, e riporta l'esito (`queued → running → done | failed`), con un
breve log che puoi espandere nell'interfaccia. I comandi più vecchi di
dieci minuti non vengono mai eseguiti; l'agent esegue un comando alla
volta.

Un comando in coda può essere **annullato** dall'interfaccia finché
l'agent non lo ha raccolto. Un comando che nessuno è venuto a
recuperare **scade** dopo dieci minuti, sul server, che l'agent lo
sondi o no — così una macchina il cui agent è fermo non mantiene mai
un "Queued" obsoleto per sempre, e non blocca mai l'azione successiva
su quel container.

L'agent dice al server, con ogni batch, se accetta comandi. Quando non
lo fa — un agent più vecchio del canale di comando, oppure installato
con `commands: false` — la pagina del dispositivo lo dice invece di
mostrare Restart e Update, l'API rifiuta i comandi (`409`), e le
policy non accodano nulla. Reinstalla l'agent con l'installer attuale
per abilitare le azioni.

- **Restart** — `docker restart` per un container in esecuzione,
  `docker start` per uno fermo, poi un controllo di 20 secondi che
  funzioni.
- **Update now** — recupera il tag che il container già usa, crea un
  nuovo container con la stessa configurazione (ambiente, mount,
  porte, etichette, reti, policy di riavvio), ferma quello vecchio,
  scambia i nomi, avvia quello nuovo e attende il suo healthcheck
  (oppure 20 secondi più un controllo di esecuzione quando l'immagine
  non ne ha uno). Se qualcosa fallisce, l'agent **ripristina**: il
  nuovo container viene rimosso e quello vecchio riavviato, e il
  comando riporta `failed` con il motivo. Al successo il vecchio
  container viene rimosso e, quando "prune" è attivo, anche la
  vecchia immagine — a meno che un altro container non la usi ancora.

I container gestiti da Compose (etichette `com.docker.compose.*`)
vengono aggiornati comunque, ma il risultato avvisa che il prossimo
`docker compose up` ricreerà il container dal file compose. I
container il cui nome inizia con `dumbmonit` o `dumbmonit` vengono
rifiutati: il monitor non si aggiorna mai da solo.

### Policy

Una **policy** per container viene valutata dal server ogni minuto:

- **Restart if down** — un container che riporta `container_up == 0`
  viene riavviato automaticamente, al massimo una volta ogni dieci
  minuti.
- **Auto-update in maintenance windows** — un container con un
  aggiornamento disponibile viene aggiornato automaticamente, ma solo
  mentre una finestra di manutenzione (silenzio) copre il dispositivo,
  al massimo una volta ogni sei ore. Pianificane una sulla pagina
  Alerts, oppure silenzia il dispositivo dalla sua pagina. Le policy
  sono predefinite per pulire la vecchia immagine e solo per le
  finestre di manutenzione.

Due posti le impostano:

- la **striscia Docker** sotto l'intestazione del dispositivo: "28
  containers · 27 running · 3 updates available · policies: 2
  auto-restart, 0 auto-update", l'ultima azione e il suo stato,
  **Manage containers** (apre l'elenco sotto) e **Policies…**, una
  tabella di ogni container con i due interruttori e una riga "Apply
  to all" — il modo più veloce per attivare l'auto-restart per
  un'intera macchina;
- gli interruttori dentro ogni riga della sezione **Containers**,
  accanto ai pulsanti Restart e Update now.

Ogni interruttore salva per proprio conto
(`PUT /api/targets/<id>/containers/<name>/policy`); un fallimento
riguarda solo quella riga e viene mostrato accanto ad essa.

Le azioni automatiche appaiono in "Recent actions" come richieste da
`policy`.

### Regole di sicurezza

- L'agent agisce solo quando `commands: true` (il predefinito);
  impostalo a `false` per una macchina che deve solo essere misurata.
- L'agent recupera i comandi da solo, con il suo token di iscrizione —
  nulla si connette alla macchina. Scadono dopo dieci minuti e
  funzionano uno alla volta; l'agent non recupera mai un tag diverso
  da quello che il container già usa.
- Un aggiornamento che non torna sano viene ripristinato al container
  precedente, e la vecchia immagine viene rimossa solo una volta che
  il nuovo container funziona.
- I container nominati `dumbmonit*` o `ezymonit*` vengono rifiutati,
  a mano o per policy: il monitor non agisce mai sui propri container.
  Il controllo viene fatto due volte — il server rifiuta di accodare
  il comando, e l'agent rifiuta di eseguirlo — e l'agent lo fa di
  nuovo sul nome che Docker riporta dopo aver fatto l'inspect del
  container, così nominarne uno con il suo id non cambia nulla.
  L'agent rifiuta anche di agire sul proprio container, qualunque sia
  il suo nome: non sarebbe lì per finire il lavoro o riportarlo.
- I container gestiti da Compose vengono aggiornati, ma il prossimo
  `docker compose up` li ricrea dal file compose: aggiorna il tag
  anche lì.

## Backup Plakar

L'agent può osservare i kloset (repository) [Plakar](https://plakar.io):
ogni dieci minuti, in background, esegue `plakar at <kloset> ls` e
`info` su ognuno e ri-emette l'ultima lettura a ogni ciclo.

Plakar viene **rilevato, mai presunto**. All'avvio e poi a ogni ciclo
finché non trova qualcosa, l'agent cerca il binario `plakar` (nel
`PATH`, oppure `plakar_bin`) e i kloset nelle posizioni standard.
Quando non c'è alcun binario, nessun kloset e nessuna traccia di
Plakar (`~/.cache/plakar`), l'agent non emette alcuna serie
`backup_*` affatto, lo dice una volta a livello `debug`, e la pagina
del dispositivo non mostra alcun pannello Backups — nulla da
configurare su macchine senza Plakar. Per saltare del tutto il
rilevamento:

```yaml
plakar: false                        # oppure DUMBMONIT_AGENT_PLAKAR=false
```

### Scoperta

Nulla da configurare: quando `plakar_klosets` è vuoto, l'agent trova
i kloset da solo, in quest'ordine, e legge ogni posizione una sola
volta:

1. la directory di configurazione propria dell'agent —
   `$XDG_CONFIG_HOME/plakar`, oppure `~/.config/plakar` dell'account
   sotto cui funziona (`plakar_home` quando impostato);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, in ordine alfabetico;
4. `/var/lib/plakar`, quando quella directory esiste.

In ogni directory legge `stores.yml` (oppure `stores.yaml`; il vecchio
`plakar.yml` viene accettato anch'esso) e prende ogni voce della mappa
`stores` (anche `repositories` e `klosets`) che ha una `location`.
Solo la location viene letta: le passphrase e i segreti rclone che
stanno accanto non vengono mai mantenuti né registrati nei log. Ogni
voce diventa il kloset `@<name>`, letto con
`plakar -configdir <dir> at @<name>`, così un kloset creato da un
utente viene letto anche quando l'agent funziona come root.
`~/.plakar` di ogni account viene aggiunto quando esiste (il kloset
predefinito di `plakar create`). Due account che nominano kloset
diversi nello stesso modo vengono distinti come `@name` e
`<user>:@name`.

Il risultato viene registrato una volta a `info` (e di nuovo solo
quando cambia). Due gauge, emessi solo una volta rilevato Plakar,
permettono all'interfaccia di distinguere i casi rimanenti:
`backup_plakar_present` (1 quando il binario funziona) e
`backup_klosets_found` (numero di kloset letti). La pagina del
dispositivo dice allora "Plakar is installed but no kloset was
found", oppure elenca i kloset. Quando i kloset esistono ma il
binario non funziona (un `PATH` del servizio senza `plakar`,
tipicamente), vengono elencati come illeggibili con un suggerimento di
impostare `plakar_bin`, e l'agent avvisa una volta.

Per osservare un elenco fisso invece, nominalo — la scoperta viene
allora saltata:

```yaml
plakar: true                         # predefinito: rileva Plakar; false lo nasconde del tutto
plakar_bin: /usr/bin/plakar          # predefinito: "plakar" nel PATH
plakar_klosets:                      # elenco esplicito: sovrascrive la scoperta
  - /srv/backups/main
  - "@diskext"                       # uno store nominato dell'account dell'agent
plakar_home: /home/noe               # opzionale: HOME (e directory di configurazione) usato quando si esegue plakar
plakar_interval_secs: 600            # minimo 60
```

| Variabile | Effetto |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — osserva i backup Plakar quando rilevati (predefinito `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Percorso del binario `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Kloset, separati da virgole (sovrascrive la scoperta) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Directory home usata quando si esegue `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Secondi tra due letture |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — controllo registry per gli aggiornamenti container |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — accetta azioni dal server |

Un kloset cifrato richiede la sua passphrase: metterla nella voce
store (`plakar store set <name> passphrase=…`, che è ciò che fa
`plakar store add`) oppure esportare `PLAKAR_PASSPHRASE`
nell'ambiente del servizio dell'agent. Senza di essa il kloset legge
come "Cannot be opened" (`backup_last_status = 0`).

### Metriche e regole

Metriche, etichettate `kloset` e `source` (la directory sottoposta a
backup): `backup_last_success_seconds` (età dello snapshot più
recente), `backup_snapshot_count`, `backup_last_status` (1 quando lo
snapshot più recente non ha errori, 0 quando ne ha o il kloset non
può essere aperto), `backup_size_bytes{kloset}` (dimensione di
storage del repository), più `backup_plakar_present` e
`backup_klosets_found` descritti sopra. Nessuna di esse esiste mentre
Plakar non è rilevato, così le regole integrate sotto non possono
attivarsi su una macchina senza Plakar.

Regole integrate: **Plakar backup too old** (advisory, nessuno
snapshot da più di due giorni) e **Plakar backup failed** (warning).
La pagina del dispositivo mostra un blocco per kloset — età
dell'ultimo backup, conteggio snapshot, dimensione, ultimo stato — e
una riga per fonte, con un suggerimento "Backup too old" oltre due
giorni.

### Eseguire un primo backup

Con Plakar 1.1 installato sulla macchina (`plakar version`), come
l'utente che possiederà i backup:

```sh
# 1. uno store kloset, nominato così l'agent lo trova in ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # oppure qualsiasi posizione che Plakar accetta
plakar at @diskext create                                   # aggiungi -plaintext per saltare la cifratura

# 2. un primo backup, poi verificalo
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` e `plakar at <path> backup <dir>`
funzionano nello stesso modo con una directory semplice invece di
`@name`; `plakar create` senza `at` usa `~/.plakar`, che l'agent
trova anch'esso. Pianifica `plakar at @diskext backup /srv/photos`
con cron o un timer systemd: l'agent riporta l'età dello snapshot più
recente, e avvisa quando passa un giorno senza uno — due giorni per
la regola integrata.

Per provarlo localmente con un kloset non cifrato:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

poi aggiungi `plakar_klosets: ["/tmp/plakar-demo"]` a `agent.yaml` (un
percorso semplice fuori da una home non viene scoperto) e verifica con
`dumbmonit-agent --dry-run` che appaiano i campioni `backup_*`.

## Backup restic e Borg

L'agent può osservare repository [restic](https://restic.net) e
[Borg](https://www.borgbackup.org): ogni dieci minuti, in background,
apre ogni repository che dichiari e riporta se può essere letto e
quanto vecchio è il suo snapshot più recente. Nulla viene scoperto: un
repository non ha una posizione standard e non può essere aperto senza
la sua password, quindi ognuno viene elencato in `agent.yaml`, con il
modo di ottenere la sua password.

**La password non lascia mai la macchina.** L'agent la passa a
`restic` o `borg` solo tramite l'ambiente del processo figlio; non
viene mai registrata nei log, mai inviata al server, e le serie
portano solo il nome del repository.

```yaml
restic_repos:
  - name: nas                                   # l'etichetta "repo"; predefinito: il repository, senza credenziali
    repository: sftp:backup@nas.lan:/srv/restic  # passata come RESTIC_REPOSITORY, mai sulla riga di comando
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # opzionale: solo gli snapshot di questo host (--host), per un repository condiviso
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # oppure qualsiasi comando che la stampa (RESTIC_PASSWORD_COMMAND)
    env:                                        # ambiente extra solo per questo repository
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # passata come BORG_REPO
    password_file: /etc/dumbmonit/borg-offsite.pass        # letta dall'agent, passata come BORG_PASSPHRASE
    glob_archives: "web-01-*"                   # opzionale: solo questi archivi (--glob-archives)
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # predefinito: nel PATH
borg_bin: borg
backup_repos_interval_secs: 600                 # minimo 60
```

Mantieni `agent.yaml` e i file delle password leggibili solo da root
(`chmod 600`), come fa l'installer per `agent.yaml`. `password_command`
corrisponde a `RESTIC_PASSWORD_COMMAND` oppure `BORG_PASSCOMMAND`.
Qualsiasi cosa gli strumenti leggano dal loro ambiente può andare sotto
`env` (`RESTIC_PASSWORD`, `BORG_PASSPHRASE`, chiavi cloud, `BORG_RSH`…),
e l'ambiente del servizio proprio dell'agent viene passato anch'esso.

Cosa esegue l'agent, di sola lettura nel senso più stretto:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: nessun
  lock viene scritto, così un repository di sola lettura o
  append-only può essere osservato, e un `prune` che tiene il lock
  esclusivo non fa apparire il repository inattivo; nessuna cache
  nemmeno, poiché il servizio installato non può scrivere su `/root`.
- `borg --bypass-lock list --json --last 1`: senza `--bypass-lock`, la
  lettura fallirebbe per l'intera durata di ogni `borg create`, che
  tiene il lock esclusivo. Borg scrive un piccolo record di sicurezza
  sotto `~/.config/borg`; quando quella home non è scrivibile (il
  servizio installato funziona con `ProtectHome=yes`), l'agent punta
  `BORG_BASE_DIR` alla sua `/tmp` privata invece, a meno che tu non
  imposti `BORG_BASE_DIR` tu stesso. Un repository in modalità
  `keyfile` richiede allora `BORG_KEY_FILE` nel suo `env`.

Poiché il servizio funziona con `ProtectHome=yes`, mantieni ciò di cui
gli strumenti hanno bisogno fuori dalle directory home: file di
password, chiavi SSH e `known_hosts` sotto `/etc/dumbmonit/`, come
nell'esempio, e repository locali sotto `/srv`, `/mnt` o `/var`
invece di `/home`. Le letture funzionano una dopo l'altra, dentro il
limite di memoria del servizio (128 MiB con l'agent stesso).

Metriche, etichettate `tool` (`restic` o `borg`) e `repo`:
`agent_backup_repo_reachable` (1 quando l'ultima lettura è riuscita, 0
altrimenti: repository irraggiungibile, password rifiutata, binario
mancante — l'agent registra il motivo una volta, e di nuovo quando
cambia), `agent_backup_repo_last_snapshot_age_seconds` (età dello
snapshot più recente, assente mentre il repository è vuoto), e
`agent_backup_repo_last_snapshot_bytes` (byte che lo snapshot restic
più recente ha elaborato, restic 0.17 e successivi). Con
`--latest 1`, restic restituisce lo snapshot più recente di ogni host
e insieme di percorsi; il più recente di tutti viene mantenuto, così
imposta `host` su un repository condiviso da diverse macchine.

Regole integrate: **restic/Borg backup too old** (nessuno snapshot da
più di 48 ore) e **restic/Borg repository unreachable** (per 30
minuti). La pagina del dispositivo mostra un pannello Backup
repositories, una riga per repository.

Per verificare una dichiarazione, esegui l'agent una volta in
foreground e cerca i campioni `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Tunnel WireGuard

Su una macchina che esegue WireGuard, l'agent legge ogni interfaccia e
ogni peer con `wg show all dump`, a ogni ciclo. Nulla da configurare:
senza il comando `wg` (wireguard-tools) o senza un'interfaccia,
nessuna serie viene emessa e nessun pannello mostrato. `wg` richiede
root (`CAP_NET_ADMIN`); il servizio installato lo ha, un agent avviato
a mano sotto il tuo proprio account registra il rifiuto una volta e
non riporta nulla.

Il dump contiene anche la chiave privata dell'interfaccia e la chiave
preshared di ogni peer: l'agent salta entrambi i campi senza copiarli
mai.

Per peer, etichettato `interface`, `peer` (la sua chiave pubblica) e
`allowed_ips`: `agent_wireguard_peer_handshake_age_seconds` (secondi
dall'ultimo handshake), `agent_wireguard_peer_has_handshake` (0
mentre il peer non ne ha mai completato uno),
`agent_wireguard_peer_keepalive_seconds` (il suo keepalive
persistente, 0 quando disattivato), e i contatori
`agent_wireguard_peer_rx_bytes` e `agent_wireguard_peer_tx_bytes`; per
interfaccia, `agent_wireguard_interface_peers`. Per un peer che non ha
mai completato un handshake, l'età conta dal momento in cui l'agent
lo ha visto per la prima volta: un tunnel che non si è mai alzato è
anch'esso silenzioso, e viene riportato come tale.

WireGuard non ha nomi di peer. Dai i tuoi in `agent.yaml`, per chiave
pubblica, e diventano l'etichetta `name`:

```yaml
wireguard: true                     # predefinito; false ferma anche il guardare
wireguard_bin: wg                   # predefinito: nel PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

La regola integrata **WireGuard peer silent** si attiva quando un
peer **con un keepalive persistente** non ha completato un handshake
da più di 15 minuti. Mentre un tunnel è attivo, WireGuard rinnova il
suo handshake ogni due minuti, e un keepalive lo mantiene attivo anche
senza traffico; un peer senza keepalive (un telefono, un laptop) può
restare inattivo per giorni senza nulla di rotto, quindi viene
escluso. Imposta `PersistentKeepalive = 25` sui tunnel che devono
restare attivi (collegamenti site-to-site, gateway remoti). N è la
soglia della regola: cambiala in **Alerts → Rules**, oppure rimuovi la
parte `and on (…)` della sua query per osservare ogni peer. La pagina
del dispositivo mostra un pannello WireGuard con ogni peer, il tempo
dal suo ultimo handshake, il suo traffico e il suo keepalive.

## Certificati TLS (Certbot / Let's Encrypt)

L'agent non legge `/etc/letsencrypt` direttamente. Ogni certificato
che Certbot rinnova viene servito da qualche parte — un server web, un
server di posta, un reverse proxy — ed è lì che DumbMonit lo legge:
aggiungi un monitor [Certificato TLS](services.md#tls) (oppure uno
[Website or web API](services.md#http), per un sito già osservato in
quel modo) puntato sulla porta su cui il certificato viene
effettivamente presentato. Legge il certificato live, non un file su
disco, così rileva anche un rinnovo riuscito in `/etc/letsencrypt` ma
mai raccolto dal servizio (un reverse proxy che richiede un reload per
vedere il nuovo file è il caso usuale, ed esattamente il buco che
conta).

Leggere `/etc/letsencrypt/live/*/cert.pem`, la configurazione di
rinnovo e il log di Certbot dall'agent è stato considerato e scartato:
duplicherebbe la data di scadenza che il monitor TLS già riporta,
aggiungendo un secondo percorso, locale all'host, alla stessa
informazione — una cosa in più da mantenere sincronizzata dopo che un
certificato viene spostato o un dominio rinominato. Un host dove
Certbot funziona ma non serve nulla che DumbMonit possa raggiungere
direttamente (un passaggio CA interno, un hook di validazione DNS) è
il solo caso che questo lascia scoperto; è anche il caso in cui un
monitor TLS non potrebbe aiutare comunque, poiché nulla su quell'host
parla TLS da sondare.
