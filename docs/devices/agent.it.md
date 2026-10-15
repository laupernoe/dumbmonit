# Agent Linux, macOS, FreeBSD e Windows

Un piccolo binario da installare sulla macchina da monitorare. Misura CPU,
memoria, dischi, rete, servizi e container — e, dove la macchina li espone,
temperature, salute dei dischi, pool ZFS e macchine virtuali KVM — poi **invia** (push) le sue letture
al server DumbMonit via HTTP.

La direzione è voluta: è l'agent a chiamare il server, mai il contrario.
Attraversa il NAT e i firewall domestici, e non richiede di aprire **nessuna
porta** sulla macchina monitorata.

## Cosa sorveglia

Tutte le metriche hanno il prefisso `dumbmonit_`. Le etichette di identità
(`target`, `host`, `tag_*`) sono impostate dal server alla ricezione, mai
dall'agent: una macchina non può scrivere nelle serie di un'altra macchina,
nemmeno falsificando le proprie etichette.

| Famiglia | Metriche | Etichette |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` solo con `cpu_per_core: true` | `core` |
| Memoria | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Filesystem | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Rete (contatori) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Dischi (contatori) | `disk_read_bytes`, `disk_written_bytes`, una serie per dispositivo a blocchi | `device` |
| Host | `uptime_seconds`, `process_count` | |
| Servizi | `service_up` (1 = in esecuzione) | `service` |
| Container | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (container oltre `docker_max_containers`) | `container`, `image` |
| Sensori | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (la soglia dichiarata dall'hardware stesso), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Salute dei dischi (SMART) | `agent_disk_smart_ok` (1 = il disco supera la propria autovalutazione), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| KVM / libvirt | `agent_libvirt_domains`, `agent_libvirt_domains_running`, `agent_libvirt_pools`; per dominio `agent_libvirt_domain_state_info`, `agent_libvirt_domain_running`, `agent_libvirt_domain_crashed`, `agent_libvirt_domain_vcpus`, `agent_libvirt_domain_memory_max_bytes`, `agent_libvirt_domain_memory_used_bytes`; per pool di storage `agent_libvirt_pool_state_info`, `agent_libvirt_pool_active`, `agent_libvirt_pool_capacity_bytes`, `agent_libvirt_pool_allocation_bytes`, `agent_libvirt_pool_used_percent` — vedi [Macchine virtuali KVM tramite libvirt](#kvm-virtual-machines-through-libvirt) | `domain`, `pool`, `state` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degradato, 2 peggio), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; per peer `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (contatori) — vedi [Tunnel WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| IPsec (strongSwan) | `agent_ipsec_tunnel_up`; quando attivo, `agent_ipsec_tunnel_bytes_in/out` (contatori) e `agent_ipsec_tunnel_rekey_seconds` — vedi [Tunnel IPsec](#ipsec-tunnels) | `connection`, `tunnel` |
| Speed test di Internet | `agent_speedtest_ok`, `agent_speedtest_age_seconds`, `agent_speedtest_download_bytes_per_second`, `agent_speedtest_upload_bytes_per_second`, `agent_speedtest_latency_ms`, `agent_speedtest_jitter_ms`; le soglie configurate riportate come `agent_speedtest_download_min_bytes_per_second`, `agent_speedtest_upload_min_bytes_per_second`, `agent_speedtest_max_latency_ms` — vedi [Speed test di Internet](#internet-speed-test) | |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — vedi [Backup restic e Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Contatori delle prestazioni di Windows | `agent_perf_counter`, una serie per contatore elencato in `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — vedi [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; per VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; per disco virtuale `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — vedi [Host Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Agent | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Una serie che non può essere misurata è **assente**, mai zero: senza `smartctl`,
niente `agent_disk_*`; senza una sonda leggibile, niente `agent_sensor_*`; senza
ZFS, niente `agent_zfs_*`; senza `virsh`, niente `agent_libvirt_*`. Una temperatura fuori dall'intervallo 1–150 °C viene
scartata anziché pubblicata, e anche una ventola che legge 0 giri/min viene
scartata — una scheda madre ha più connettori per ventole che ventole, e uno
vuoto legge zero come una ventola morta.

### Cosa viene raccolto dove

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, memoria, swap, filesystem, rete, uptime, processi | sì | sì | sì | sì |
| Load average | sì | sì | sì | no (Windows non ha questa nozione) |
| Servizi | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | servizi di Windows |
| Container Docker | sì | sì, se `docker_socket` punta al socket di Docker Desktop | no (nessun demone Docker) | no |
| Temperature | `/sys/class/hwmon` | sensori SMC | `dev.cpu.N.temperature` (solo CPU) | no |
| Ventole | `/sys/class/hwmon` | no | no | no |
| Salute dei dischi (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Pool ZFS | `zpool` (OpenZFS) | no | `zpool` | no |
| Macchine virtuali KVM (libvirt) | `virsh` | no | no | no |
| Salute del sistema operativo (aggiornamenti, riavvio, unit fallite, SELinux) | sì | no | no | no |
| Backup Plakar | sì | sì | sì | sì |
| Tunnel WireGuard | `wg show` | se `wg` è installato (non testato) | se `wg` è installato (non testato) | no |
| Tunnel IPsec | `swanctl` | se `swanctl` è installato (non testato) | se `swanctl` è installato (non testato) | no |
| Speed test di Internet | sì, opt-in | sì, opt-in | sì, opt-in | sì, opt-in |
| Backup restic e Borg | sì | sì | sì | solo restic (non testato; Borg non ha una build per Windows) |
| Contatori delle prestazioni, code MDaemon, Hyper-V | no | no | no | sì |
| Identità della macchina | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | nome host |
| Modalità relay | sì | sì | sì | sì |

Windows non ha una riga per le temperature di proposito: leggere i sensori lì
significa passare da WMI, con diritti amministrativi, per valori che la maggior
parte delle macchine non espone affatto. Non pubblicare nulla è la risposta
onesta.

`smartctl` (smartmontools), `zpool` e `virsh` vengono cercati a ogni periodo di lettura e
usati se presenti; nessuno è installato dall'agent, e nessuno è
obbligatorio. Tutti richiedono root (`virsh` funziona anche per un membro del gruppo `libvirt`) — il servizio registrato dall'installer
gira come root, un agent avviato a mano con il tuo account non troverà nulla.

I contatori vengono inviati grezzi, in modo che un riavvio dell'agent non
produca mai un falso tasso; applica `rate()` nelle query.

Regole integrate che si applicano: Monitor irraggiungibile, CPU elevata, CPU
inusuale. "Monitor irraggiungibile" funziona anche sugli agent: il server
controlla l'età dell'ultimo batch ricevuto e dichiara la macchina silenziosa dopo
tre periodi saltati (almeno 90 secondi).

Altre quattro scattano solo sulle macchine che producono le serie corrispondenti,
quindi non richiedono alcuna configurazione e non possono scattare su una
macchina senza l'hardware:

| Regola | Scatta quando |
|---|---|
| **Disco con SMART in errore** (critical) | l'autovalutazione di un disco lo segnala come in avaria |
| **Pool ZFS degradato** (critical) | un pool non è più `ONLINE` da cinque minuti |
| **Lo scrub ZFS ha trovato errori** (critical) | l'ultimo scrub ha trovato errori, oppure il pool segnala file definitivamente corrotti |
| **Dominio KVM in crash** (critical) | libvirt segnala un dominio come `crashed` |
| **Pool libvirt quasi pieno** (warning) | l'allocazione di un pool di storage raggiunge il 90% della sua capacità per quindici minuti |
| **Temperatura sopra il livello critico** (critical) | un sensore supera la soglia critica **dichiarata dal suo stesso hardware** — nessun valore è fissato nel codice, 85 °C essendo un'emergenza su un disco e un normale pomeriggio su una CPU di portatile |

Altre sorvegliano ciò che dichiari o esegui tu stesso, e restano silenziose
altrimenti: **Peer WireGuard silenzioso** (un peer con keepalive persistente e
senza handshake da 15 minuti), **Tunnel IPsec non attivo** (un tunnel
configurato senza sessione installata da dieci minuti), **Backup restic/Borg
troppo vecchio** (nessuno snapshot da 48 ore), **Repository restic/Borg
irraggiungibile** e **Velocità di download Internet bassa** / **Velocità di
upload Internet bassa** / **Latenza Internet alta** (solo una volta impostata
un'opzione di soglia) — vedi [Tunnel WireGuard](#wireguard-tunnels),
[Tunnel IPsec](#ipsec-tunnels), [Backup restic e Borg](#restic-and-borg-backups)
e [Speed test di Internet](#internet-speed-test).

Su un server Windows che esegue MDaemon, altre tre sorvegliano le sue code:
**Coda di posta MDaemon in crescita**, **Coda Bad di MDaemon non vuota** e
**Coda Retry di MDaemon alta** (vedi [Regole](../alerting/rules.md#mdaemon-and-securitygateway)).
Su un host Hyper-V, tre sorvegliano le macchine virtuali: **Salute VM Hyper-V
critica**, **CPU dell'host Hyper-V elevata** ed **Errori sul disco virtuale
Hyper-V** (vedi [Host Hyper-V](#hyper-v-hosts)).

## Installazione {#install}

I passaggi qui sotto sono quelli mostrati dall'avviso accanto al modulo. Non c'è
alcun account da creare sulla macchina: il token di registrazione è l'unico
segreto.

1. Salva il monitor in DumbMonit: un token di registrazione (dmon_…) viene
   mostrato una sola volta, con il comando di installazione pronto da copiare per
   Linux e per Windows. (Un token può essere creato anche in **Impostazioni →
   Agent**.)
2. Quel token è la chiave con cui l'agent invia le proprie misure a DumbMonit.
   Non è un account sulla macchina: non c'è nulla da creare lì. Un token creato
   dal modulo del monitor è monouso: registra questa sola macchina e nient'altro.
   Per una flotta, crea un token riutilizzabile in Impostazioni → Agent.
3. Esegui il comando di installazione sulla macchina da monitorare, con diritti
   elevati (sudo su Linux e macOS, un PowerShell elevato su Windows). Scarica
   l'agent, scrive il token in agent.yaml e avvia il servizio.
4. La macchina compare da sola entro pochi secondi, con il nome del suo host. Con
   quel primo batch il server dà all'agent un segreto proprio e lo memorizza in
   `/etc/dumbmonit/agent-secret`, leggibile da nessun altro: da quel momento, quella
   macchina è l'unica che può riferire sotto questo monitor. Token perso?
   Impostazioni → Agent permette di revocarlo e crearne un altro.
5. Docker: per vedere i container e lasciare che DumbMonit li riavvii o li
   aggiorni, l'agent deve raggiungere il socket di Docker. Il servizio registrato
   dall'installer ci riesce già; se esegui invece l'agent con un utente dedicato,
   aggiungi quell'utente al gruppo "docker" e riavvialo. Le politiche di riavvio e
   di aggiornamento automatico si impostano poi per container nella pagina del
   monitor.

Due cose vale la pena leggere prima di distribuirlo su più di una macchina:
[Associazione](#binding-one-machine-one-agent), che è ciò che fa appartenere
davvero un monitor a una sola installazione dell'agent, e
[Monouso o flotta](#single-use-or-fleet), che è la scelta che fai quando crei un
token.

    ```
    usermod -aG docker dumbmonit
    ```

6. Backup Plakar: rilevati automaticamente — sotto la macchina compare un monitor Plakar quando
   viene trovato il binario plakar o un kloset (~/.config/plakar/stores.yml di
   ogni utente, ~/.plakar, /var/lib/plakar), e altrimenti non viene mostrato
   nulla. Imposta "plakar_klosets" in agent.yaml per sorvegliare un elenco fisso,
   o "plakar: false" per escluderli.
7. I servizi che l'agent trova (Docker, Plakar, restic/Borg, ZFS, libvirt,
   WireGuard, IPsec, speed test) diventano ciascuno un monitor a sé sotto la
   macchina, con stato e avvisi propri. Il pannello "Servizi trovati dall'agente"
   della macchina elenca ciò che è stato trovato e ciò che non lo è: deseleziona
   un modulo per eliminarne il monitor, selezionalo per sorvegliarlo prima che
   compaia. Vedi [Servizi trovati dall'agente](#services-found-by-the-agent).

!!! note
    L'agent contatta il server, mai il contrario: non serve aprire alcuna porta
    sulla macchina monitorata.

=== "Linux (systemd o OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Lo script è POSIX `sh` (funzionano BusyBox e dash), scarica dal server il
    binario per l'architettura della macchina (x86_64 o aarch64), scrive
    `/etc/dumbmonit/agent.yaml` (modo 0600) e registra `dumbmonit-agent` con
    systemd, oppure con OpenRC su Alpine. L'unit systemd è rinforzata
    (`ProtectSystem=strict`, `NoNewPrivileges`, `MemoryMax=128M`,
    `CPUQuota=20%`).

=== "macOS (launchd)"

    Lo stesso comando di Linux, eseguito con `sudo`:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    I binari per macOS **non** sono nell'immagine di DumbMonit: compilarli
    richiede l'SDK di Apple, la cui licenza ne vieta la redistribuzione. Sono
    allegati a ogni [release di GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
    — `dumbmonit-agent-macos-aarch64` (Apple silicon) e
    `dumbmonit-agent-macos-x86_64` (Intel), ciascuno con il suo `.sha256` — e il
    server reindirizza `/download/dumbmonit-agent-macos-…` lì, così lo script
    sceglie quello per l'architettura del Mac (`uname -m`). Il server non
    distribuisce quei binari, quindi il comando di installazione non contiene
    alcun checksum per essi: aggiungi `--sha256=macos-aarch64:<hex>` (o
    `macos-x86_64`) con il valore del `.sha256` della release, altrimenti
    l'installer rifiuta il download. Il Mac deve inoltre raggiungere github.com.

    Senza quell'accesso, o per installare un binario che hai già, scaricalo e
    passalo con `--bin` (confrontalo prima con il `.sha256` della release:
    `--bin` installa ciò che gli dai):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    Lo script installa il binario in `/usr/local/bin` (e rimuove l'attributo di
    quarantena che porta un download da browser, che farebbe rifiutare l'avvio a
    Gatekeeper), scrive `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e
    registra il demone di sistema `com.dumbmonit.agent` in
    `/Library/LaunchDaemons`, avviato con `launchctl bootstrap system`. Gira
    all'avvio, senza nessuno connesso.

    !!! note "Preferisci compilarlo da te?"
        Sul Mac stesso, con Rust installato:
        `cargo build --release -p dumbmonit-agent`, poi
        `--bin=target/release/dumbmonit-agent`. È lo stesso binario prodotto dal
        job di release.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Solo x86_64. Lo script scarica `dumbmonit-agent-freebsd-x86_64` dal server,
    scrive `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e installa
    `/usr/local/etc/rc.d/dumbmonit_agent`, abilitato con
    `service dumbmonit_agent enable`. `daemon(8)` supervisiona l'agent, lo
    riavvia se si ferma e scrive `/var/log/dumbmonit-agent.log`.

    Funziona allo stesso modo su TrueNAS CORE e altri appliance basati su FreeBSD,
    dove le serie ZFS sono il punto dell'esercizio.

=== "Windows (servizio)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    Lo script scarica `dumbmonit-agent-windows-x86_64.exe` dal server, lo
    verifica rispetto al suo SHA-256, lo installa come
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, scrive
    `C:\ProgramData\DumbMonit\agent.yaml` (leggibile solo da SYSTEM e
    Administrators) e registra il servizio Windows `DumbMonitAgent`
    ("DumbMonit system agent"): avviato automaticamente all'avvio, come
    LocalSystem, e riavviato dopo un errore (10 s, 30 s, poi 60 s). L'agent parla
    da sé con il gestore dei servizi (`--service`), quindi fermare il servizio gli
    permette di inviare ciò che ancora trattiene prima di uscire.

    Un servizio non ha console: l'agent scrive il proprio log in
    `C:\ProgramData\DumbMonit\agent.log`, messo da parte come `agent.log.old`
    oltre i 10 MB.

    Non esiste una build ARM64 nativa: su Windows 11 per ARM, lo script installa
    il binario x86_64, che gira con l'emulazione x64.

Un token può registrare più macchine: dagli il nome di un gruppo o di una macchina.

### Binari nativi

L'agent è un singolo eseguibile statico per sistema, senza nulla da installare
accanto:

| Sistema | File | Dove ottenerlo |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | il server, `/download/…` |
| Linux aarch64 (ARM a 64 bit: Raspberry Pi OS 64 bit, server ARM) | `dumbmonit-agent-linux-aarch64` | il server, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | il server, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | il server, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | la release di GitHub (il server reindirizza lì) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | la release di GitHub (il server reindirizza lì) |

I binari Linux sono collegati staticamente a musl: girano su qualsiasi
distribuzione, glibc o no (Alpine, firmware di NAS), senza librerie da far
corrispondere. Ogni file ha accanto il proprio checksum, `<file>.sha256`, in
formato `sha256sum`. Ogni [release di GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
contiene tutti e sei i binari e i loro checksum, per le macchine che non possono
raggiungere `/download/` del server o per installare con `--bin`.

Dalla versione 0.1.0-alpha.9, ogni binario di una release è inoltre firmato
senza chiavi (keyless) con [Sigstore](https://www.sigstore.dev/)
(`<file>.sigstore.json`) e accompagnato da un'attestazione di provenienza SLSA.
Per verificare che un binario sia stato compilato dal workflow di rilascio di
questo repository:

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'
# or, with the GitHub CLI:
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Per compilare da te i binari Linux, Windows e FreeBSD, da un clone del
repository (Docker è l'unico requisito):

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

`--uninstall` (`-Uninstall` su Windows) rimuove tutto, servizio compreso, su ogni
sistema.

!!! note "Il download è verificato — sempre"
    Il comando di installazione mostrato da DumbMonit contiene lo SHA-256 atteso
    di ciascun binario dell'agent distribuito dal server
    (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` su Windows). Entrambi gli installer controllano il
    file scaricato rispetto a esso prima di installare qualsiasi cosa, e
    **rifiutano** di installare quando non corrisponde, quando non è dato alcun
    checksum per la piattaforma, o quando non è disponibile alcuno strumento
    SHA-256 (`sha256sum`, `shasum`, `sha256`).

    I checksum cambiano a ogni release del server: dopo aver aggiornato
    DumbMonit, copia un comando di installazione nuovo da **Impostazioni →
    Agent** (o da un nuovo token) invece di rieseguirne uno vecchio, che si
    fermerebbe con "checksum mismatch". Gli stessi checksum sono mostrati sotto
    il comando di installazione, e pubblicati in `/download/<file>.sha256`.

    Per configurazioni air-gapped o manuali, `--insecure-skip-checksum`
    (`-InsecureSkipChecksum`) installa un binario scaricato senza verificarlo e
    lo segnala a chiare lettere; `--bin=PATH` installa un file locale così com'è,
    essendo tuo il compito di verificarlo.

### Opzioni dell'installer

| `install.sh` | `install.ps1` | Effetto |
|---|---|---|
| `--token=TOKEN` | `-Token` | Token di registrazione (obbligatorio). |
| `--url=URL` | `-Url` | URL del server, per esempio `http://server:8080`. Su Linux, se l'opzione manca viene usata `DUMBMONIT_URL` dell'ambiente. |
| `--interval=N` | `-Interval` | Periodo di campionamento in secondi (predefinito 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Servizi di cui viene riportato lo stato, nominati come li chiama il sistema ospite: unit systemd su Linux, label launchd (`com.apple.sshd`) su macOS, nomi rc.d su FreeBSD, servizi di Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Tag, copiati come `tag_<key>` su ogni serie. |
| `--hostname=NAME` | `-HostName` | Nome comunicato al server (predefinito: quello della macchina). |
| `--bin=PATH` | `-BinPath` | Binario locale da installare invece di scaricarlo. |
| `--sha256=LIST` | `-Sha256` | SHA-256 atteso del download, come coppie `platform-arch:hex` separate da virgole (funziona anche un semplice valore esadecimale). Incluso nel comando mostrato da DumbMonit; obbligatorio a meno che sia data l'opzione successiva. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Installa un binario scaricato senza verificarlo, con un avviso. Solo per configurazioni air-gapped o manuali. |
| `--no-start` | | Installa tutto, ma non contatta il server né avvia il servizio (immagine di macchina, test). |
| `--uninstall` | `-Uninstall` | Disinstalla l'agent ed elimina la sua configurazione. |
| `--help` | | Mostra l'aiuto. |

### Aggiornamento {#upgrade}

Entrambi gli script sono idempotenti: rieseguire il comando di installazione, con
lo stesso token e lo stesso URL, sostituisce il binario, riscrive la
configurazione e riavvia il servizio. È questa la procedura di aggiornamento.
Prendi il comando dal server attuale (Impostazioni → Agent): il suo `--sha256`
deve corrispondere ai binari della versione del server a cui hai aggiornato.

Lo stesso comando migra una macchina che esegue ancora l'agent EzyMonit, su Linux
e su Windows. L'installer ferma e rimuove il vecchio servizio
(`ezymonit-agent`, o `EzyMonitAgent` su Windows), sposta la configurazione nel
nuovo percorso (`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`)
e installa al suo posto `dumbmonit-agent`. Il vecchio token `ezym_…` resta
valido, quindi il comando mostrato alla creazione del token funziona ancora;
non occorre revocare nulla prima.

## Servizi trovati dall'agente {#services-found-by-the-agent}

Il monitor della macchina mostra la macchina: CPU, memoria, filesystem, rete,
salute dei dischi (SMART), sensori, salute del SO e i servizi che elenchi in
`services`. Tutto il resto che l'agent rileva è un **servizio**, e ogni servizio
che trova diventa un monitor a sé, di tipo `agent_service`, archiviato sotto
la macchina:

| Servizio | Le sue metriche | Nome del monitor |
|---|---|---|
| Docker | `container_*` | `<machine> · Docker` |
| Plakar | `backup_*` | `<machine> · Plakar` |
| restic / Borg | `agent_backup_repo_*` | `<machine> · restic/Borg` |
| ZFS | `agent_zfs_*` | `<machine> · ZFS` |
| KVM / libvirt | `agent_libvirt_*` | `<machine> · libvirt` |
| WireGuard | `agent_wireguard_*` | `<machine> · WireGuard` |
| IPsec | `agent_ipsec_*` | `<machine> · IPsec` |
| Speed test di Internet | `agent_speedtest_*` | `<machine> · Speed test` |

La suddivisione avviene sul server, in base al nome della metrica, quindi
funziona con ogni versione dell'agent. Ogni serie viene scritta una sola volta,
con `target` impostato sul monitor a cui appartiene: le serie di un servizio
portano l'id del monitor del servizio, quelle della macchina l'id della
macchina. Nulla viene contato due volte.

Un monitor di servizio ha stato, avvisi, regole, override delle regole e
silenziamenti propri. È **irraggiungibile** quando l'agent smette di riportarlo
mentre la macchina continua a riportare (daemon Docker fermato, `wg`
rimosso…), dopo gli stessi tre periodi mancati della macchina stessa. Quando la
macchina tace, i suoi servizi vengono mostrati come nascosti dietro di essa
(`suppressed`) invece di generare avvisi propri. Un nuovo monitor di servizio
parte con i tag della macchina, quindi silenziamenti e regole basati sui tag
continuano ad applicarsi.

Prima dell'installazione non c'è nulla da scegliere. Dopo il primo report, la
pagina della macchina elenca ogni modulo in **Servizi trovati dall'agente**:

- un servizio trovato è selezionato e rimanda al suo monitor;
- un servizio non trovato è deselezionato; selezionalo per creare comunque il suo
  monitor (resta irraggiungibile finché l'agent non lo riporta — utile per
  essere avvisati quando manca un servizio su cui conti);
- deseleziona un servizio trovato per eliminarne il monitor e scartarne le
  letture. Il server lo comunica all'agent nel suo prossimo riscontro, e l'agent
  smette di raccoglierlo (un agent più vecchio continua a raccogliere, e il
  server continua a scartare). Eliminare un monitor di servizio ha lo stesso
  effetto. Selezionalo di nuovo per ripristinarlo.

Eliminare la macchina elimina i suoi monitor di servizio. Azioni e politiche
dei container funzionano dalla pagina del monitor Docker; i comandi vanno
sempre all'agent della macchina.

Come nell'avviso del monitor:

1. Non c'è nulla da aggiungere a mano: installa l'agent sulla macchina, e ogni
   servizio che trova diventa un monitor sotto quella macchina al suo primo
   report.
2. Per smettere di sorvegliare un servizio, deselezionalo nel pannello "Servizi
   trovati dall'agente" della macchina, oppure elimina il suo monitor: le
   letture dell'agent per esso vengono allora scartate.

!!! note
    Se la macchina smette di riportare, i suoi servizi vengono mostrati come
    nascosti dietro di essa invece di generare avvisi propri.

### Aggiornare un'installazione esistente {#upgrading-an-existing-installation}

Non c'è nulla da fare. La prima volta che l'agent riporta un servizio dopo
l'aggiornamento, il server crea il suo monitor e vi sposta ciò che riguardava
quel servizio sulla macchina:

- gli override delle regole che leggono le metriche del servizio vengono copiati
  sul monitor del servizio;
- le regole limitate alla macchina (selettore `ids`) che leggono le metriche del
  servizio ora selezionano anche il monitor del servizio;
- i silenziamenti (finestre di manutenzione) della macchina vengono copiati sul
  monitor del servizio;
- gli avvisi in corso sulle serie del servizio vengono spostati sul monitor
  del servizio con la loro nuova impronta, così non si risolvono per poi
  scattare di nuovo.

Lo storico resta dove è stato scritto: VictoriaMetrics non può rietichettare le
serie esistenti, quindi le letture precedenti all'aggiornamento restano sotto la
macchina (scadono con la retention), e i grafici del monitor del servizio
partono dall'aggiornamento. Le baseline di anomalia delle serie spostate
ricominciano ad apprendere. Anche lo storico degli avvisi (avvisi risolti) resta
con la macchina.

## File di configurazione

`/etc/dumbmonit/agent.yaml` su Linux, `/usr/local/etc/dumbmonit/agent.yaml` su
macOS e FreeBSD, `C:\ProgramData\DumbMonit\agent.yaml` su Windows:

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
libvirt: true              # KVM domains and storage pools through virsh (default: true)
libvirt_bin: virsh
libvirt_uri: qemu:///system
libvirt_interval_secs: 60  # minimum 15
wireguard: true            # WireGuard tunnels through wg (default: true), see below
ipsec: true                # IPsec tunnels through swanctl (default: true), see below
speedtest: false           # Internet speed test (default: false, opt-in), see below
speedtest_download_url: https://speed.example.lan/backend/garbage.php?ckSize=50
speedtest_upload_url: https://speed.example.lan/backend/empty.php
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

`sensors`, `smart`, `zfs` e `libvirt` vengono rilevati, mai presupposti: lasciati a `true`
su una macchina senza sonde, senza `smartctl`, senza pool o senza `virsh`, costano un'occhiata
per periodo e non emettono assolutamente nulla. Impostane uno a `false` per
smettere persino di guardare — vale la pena su un NAS i cui dischi preferisci che
`smartctl` non tocchi mai, anche se l'agent passa già `-n standby` in modo che un
disco in sospensione venga misurato solo quando è sveglio per altri motivi.

### Macchine virtuali KVM tramite libvirt {#kvm-virtual-machines-through-libvirt}

Su un hypervisor KVM (un normale host Linux con libvirt, non Proxmox VE, che ha
una [propria integrazione](proxmox.md)), installa l'agent sull'host stesso, non
in un guest. L'agent esegue `virsh -c qemu:///system` con `list --all --name`,
`dominfo --domain <nome>`, `pool-list --all --name` e `pool-info --pool <nome>`:
non definisce, avvia, ferma né migra mai un dominio, e non modifica mai un pool
di storage. L'URI esplicito `qemu:///system` conta per un agent eseguito da un
membro del gruppo `libvirt` anziché da root: senza, `virsh` leggerebbe la
sessione privata, vuota, di quell'utente. Imposta `libvirt_uri` per leggere
un'altra connessione.

`agent_libvirt_domain_memory_used_bytes` è assente per un dominio spento:
libvirt ne conosce solo il massimo configurato. Le dimensioni dei pool di
storage provengono dall'output leggibile di `virsh`, arrotondato a due
decimali, quindi `agent_libvirt_pool_used_percent` è un'approssimazione,
abbastanza vicina da avvisare prima che un pool si riempia. Vengono
dettagliati al massimo 200 domini e 64 pool.

Accanto, `/etc/dumbmonit/agent-secret` (`C:\ProgramData\DumbMonit\agent-secret`
su Windows) contiene il segreto di associazione che il server ha consegnato a
questa macchina. L'agent lo scrive con modo `0600` e lo rilegge a ogni avvio; non
fa parte di `agent.yaml` perché non è qualcosa che imposti tu — vedi
[Associazione](#binding-one-machine-one-agent). `DUMBMONIT_AGENT_SECRET_FILE` lo
sposta, ed è ciò che serve per un agent in un container.

Ogni chiave può essere sovrascritta dall'ambiente, il che rende l'agent
utilizzabile in un container senza montare un file:

| Variabile | Effetto |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Percorso del file di configurazione |
| `DUMBMONIT_AGENT_URL` | URL del server |
| `DUMBMONIT_AGENT_TOKEN` | Token di registrazione |
| `DUMBMONIT_AGENT_SECRET_FILE` | Dove conservare il segreto di associazione (predefinito: `agent-secret`, accanto al file di configurazione) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Periodo di campionamento |
| `DUMBMONIT_AGENT_HOSTNAME` | Nome comunicato al server |
| `DUMBMONIT_AGENT_SERVICES` | Servizi da sorvegliare, separati da virgole |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, separati da virgole |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Percorso del socket Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Container dettagliati per host, predefinito `200` (`0`: solo conteggi) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfacce escluse, nomi o regex separati da virgole; predefinito `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfacce da tenere; se impostato, sostituisce l'elenco di esclusione |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Punti di mount esclusi da filesystem e I/O dei dischi; il predefinito salta `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` per inviare anche una serie CPU per core (predefinito `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: temperature e ventole (predefinito `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: salute dei dischi tramite `smartctl` (predefinito `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Percorso di `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Periodo tra due letture SMART, predefinito `300`, minimo `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: pool ZFS tramite `zpool` (predefinito `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Percorso di `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Periodo tra due letture ZFS, predefinito `120`, minimo `30` |
| `DUMBMONIT_AGENT_LIBVIRT` | `true` / `false`: domini KVM e pool di storage tramite `virsh` (predefinito `true`) |
| `DUMBMONIT_AGENT_LIBVIRT_BIN` | Percorso di `virsh` |
| `DUMBMONIT_AGENT_LIBVIRT_URI` | URI di connessione libvirt, predefinito `qemu:///system` |
| `DUMBMONIT_AGENT_LIBVIRT_INTERVAL_SECS` | Periodo tra due letture libvirt, predefinito `60`, minimo `15` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: tunnel WireGuard tramite `wg` (predefinito `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Percorso di `wg` |
| `DUMBMONIT_AGENT_IPSEC` | `true` / `false`: tunnel IPsec tramite `swanctl` (predefinito `true`) |
| `DUMBMONIT_AGENT_IPSEC_BIN` | Percorso di `swanctl` |
| `DUMBMONIT_AGENT_SPEEDTEST` | `true` / `false`: esegue lo speed test di Internet (predefinito `false`) |
| `DUMBMONIT_AGENT_SPEEDTEST_DOWNLOAD_URL` | Indirizzo scaricato per misurare la velocità di download |
| `DUMBMONIT_AGENT_SPEEDTEST_UPLOAD_URL` | Indirizzo a cui si invia con POST per misurare la velocità di upload |
| `DUMBMONIT_AGENT_SPEEDTEST_DOWNLOAD_BYTES_CAP` | Byte letti al massimo per test, predefinito `50000000` |
| `DUMBMONIT_AGENT_SPEEDTEST_UPLOAD_BYTES` | Byte inviati per test, predefinito `10000000` |
| `DUMBMONIT_AGENT_SPEEDTEST_INTERVAL_SECS` | Periodo tra due test, predefinito `21600` (6 h), minimo `300` |
| `DUMBMONIT_AGENT_SPEEDTEST_MIN_DOWNLOAD_MBPS` | Soglia di avviso, velocità di download in Mbps (non impostata: nessun avviso) |
| `DUMBMONIT_AGENT_SPEEDTEST_MIN_UPLOAD_MBPS` | Soglia di avviso, velocità di upload in Mbps (non impostata: nessun avviso) |
| `DUMBMONIT_AGENT_SPEEDTEST_MAX_LATENCY_MS` | Soglia di avviso, latenza in millisecondi (non impostata: nessun avviso) |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Percorso di `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Percorso di `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Periodo tra due letture dei repository restic e Borg, predefinito `600`, minimo `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` o `auto`: contatori delle prestazioni di MDaemon (predefinito `auto`, letti quando esiste il servizio Windows `MDaemon`) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` o `auto`: contatori delle prestazioni di Hyper-V (predefinito `auto`, letti quando esiste il servizio Windows `vmms`) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Percorsi dei contatori delle prestazioni di Windows, separati da **punti e virgola** (una virgola può far parte del nome di un'istanza); sostituisce `perf_counters` |
| `DUMBMONIT_AGENT_SYSTEM_HEALTH` | `true` / `false`: salute del sistema operativo, interruttore generale dei quattro successivi (predefinito `true`) |
| `DUMBMONIT_AGENT_CHECK_UPDATES` | `true` / `false`: aggiornamenti in sospeso (predefinito `true`) |
| `DUMBMONIT_AGENT_UPDATES_INTERVAL_SECS` | Periodo tra due controlli degli aggiornamenti in sospeso, predefinito `3600`, minimo `300` |
| `DUMBMONIT_AGENT_CHECK_REBOOT` | `true` / `false`: riavvio necessario (predefinito `true`) |
| `DUMBMONIT_AGENT_CHECK_FAILED_UNITS` | `true` / `false`: unit systemd fallite (predefinito `true`) |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Dimensione del buffer di recupero |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` per eseguire, per il server, i probe dei monitor assegnati a questo agent (vedi [Eseguire l'agent in Docker / su un'altra rete](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Etichetta del sito mostrata accanto all'agent quando viene proposto come relay |

## Contatori delle prestazioni di Windows {#windows-performance-counters}

Su Windows, l'agent legge i contatori delle prestazioni — i valori che mostra
Performance Monitor (`perfmon`) — tramite la libreria PDH, e li pubblica a ogni
periodo di campionamento. Alcuni software non pubblicano il proprio stato in
nessun altro posto.

**MDaemon.** Quando esiste il servizio Windows chiamato `MDaemon`, l'agent legge
l'oggetto di prestazioni di MDaemon: dimensioni delle code, sessioni, totali di
messaggi e spam, stato dei server interni di MDaemon e il suo uptime. Le serie, le
regole e la pagina che le mostra sono descritte nella pagina
[MDaemon](mdaemon.md#mail-queues-through-the-agent). `mdaemon: true` le legge
anche senza il servizio (un servizio rinominato), `mdaemon: false` non le legge
mai. La presenza del servizio viene ricontrollata ogni dieci periodi, così un
MDaemon installato dopo l'agent viene rilevato senza riavvio.

**Hyper-V.** Quando esiste il servizio Hyper-V Virtual Machine Management
(`vmms`), l'agent legge gli oggetti di prestazioni di Hyper-V; vedi
[Host Hyper-V](#hyper-v-hosts). `hyperv: true` e `hyperv: false` forzano la
scelta, come per MDaemon.

**Qualsiasi altro contatore.** Elenca il suo percorso completo sotto
`perf_counters`, come `\Object(Instance)\Counter` oppure `\Object\Counter` per un
oggetto con una sola istanza, e viene pubblicato come
`dumbmonit_agent_perf_counter{counter="…"}`, con il nome che dai o con il
percorso stesso:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Scrivi i nomi dei contatori **in inglese**, come appaiono in Performance
  Monitor su un Windows in inglese: l'agent li aggiunge con
  `PdhAddEnglishCounterW`, quindi lo stesso `agent.yaml` funziona su un Windows
  francese o tedesco.
- Metti ogni percorso tra apici singoli in YAML, in modo che le barre rovesciate
  e il `%` vengano mantenuti così come sono.
- Un percorso malformato ferma l'agent all'avvio indicando la chiave e il motivo.
  Un percorso che Windows non conosce (il software non è installato, il nome è
  scritto male) viene segnalato una volta nel log, `performance counter not
  available`, e gli altri contatori vengono comunque letti; viene ritentato ogni
  dieci periodi.
- I tassi (`…/sec`) richiedono due letture: compaiono dal secondo periodo in poi.
- Vengono letti al massimo 200 contatori. Su Linux, macOS e FreeBSD
  l'impostazione è accettata e non viene letto nulla.

Per vedere i nomi esatti sulla macchina, esegui `typeperf -q MDaemon` (o qualsiasi
altro nome di oggetto) in un prompt dei comandi.

## Host Hyper-V {#hyper-v-hosts}

Installato su un host Hyper-V — Windows Server con il ruolo Hyper-V, o Windows con
Hyper-V abilitato — l'agent trova il servizio `vmms` e legge i contatori delle
prestazioni propri di Hyper-V, senza nulla da configurare. La pagina del
monitor dell'agent mostra allora un pannello Hyper-V: quante macchine virtuali
risultano sane, il vero carico CPU dell'host, la memoria detenuta da ogni VM in
esecuzione, e il throughput, la latenza e gli errori di ogni disco virtuale.

!!! warning "Validato solo rispetto alla documentazione del produttore — non ancora testato su un sistema reale"

    I nomi dei contatori provengono dalla documentazione sulle prestazioni di
    Hyper-V di Microsoft e sono stati verificati rispetto alla logica con una
    sorgente di contatori simulata, non su un host Hyper-V in funzione. Un
    contatore che Windows non conosce viene segnalato una volta nel log
    dell'agent e gli altri vengono comunque letti. Facci sapere cosa non funziona.

| Contatore | Serie | Etichette |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: macchine virtuali in ciascuno stato | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: il carico dell'host, guest inclusi | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: la quota spesa per eseguire i guest | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: VM in esecuzione più l'host stesso | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: richiesta di memoria rispetto alla memoria assegnata; sopra 100 la VM vuole più di quanto abbia | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (media), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, un contatore | `disk` |

Su un host Hyper-V, `\Processor(_Total)\% Processor Time` misura solo la
partizione propria dell'host, non ciò che consumano le macchine virtuali: il
contatore del processore logico dell'hypervisor è quello che dice se la macchina è
satura. `vm` è il nome della macchina virtuale come lo mostra Hyper-V Manager;
`disk` è il nome d'istanza che Windows assegna a ogni disco virtuale, derivato dal
percorso del suo file. Le istanze `_Total` sono escluse, e vengono letti al
massimo 200 VM o dischi per contatore.

Si applicano tre [regole integrate](../alerting/rules.md#hyper-v):

- **Salute VM Hyper-V critica**: Hyper-V segnala una VM in stato di salute critico
  per cinque minuti (Avvertenza) — più spesso un disco che ha esaurito lo spazio.
- **CPU dell'host Hyper-V elevata**: i processori logici occupati per più del 90 %
  per quindici minuti (Avviso).
- **Errori sul disco virtuale Hyper-V**: un disco virtuale ha segnalato errori di
  I/O nell'ultima ora (Avviso).

## Eseguire l'agent in Docker / su un'altra rete {#run-the-agent-in-docker--on-another-network}

L'agent è pubblicato anche come immagine di container, con gli stessi tag del
server: `ghcr.io/laupernoe/dumbmonit-agent:latest` (ultima release), `:edge`
(ultimo commit su `main`). È un'immagine `FROM scratch` che contiene il binario
dell'agent e il bundle di CA di Mozilla, configurata interamente tramite variabili
d'ambiente. Il repository include un `docker-compose.agent.yml` pronto all'uso:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

Cosa vede il container, e cosa montare:

| Esigenza | Cosa fare | Perché |
|---|---|---|
| CPU, memoria, load, I/O dei dischi dell'host | Niente | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` e `/proc/diskstats` sono a livello di host dentro qualsiasi container. |
| Nome dell'host | `DUMBMONIT_AGENT_HOSTNAME` | L'hostname di un container è un id casuale; il monitor verrebbe nominato così. |
| Container dell'host | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Facoltativo. In sola lettura elenca e ispeziona; le azioni Riavvia/Aggiorna lo richiedono scrivibile e `DUMBMONIT_AGENT_COMMANDS=true`. Senza il socket, l'agent semplicemente non riporta container. |
| Utilizzo dei filesystem | `- /:/host:ro` (o i mount che ti interessano) | L'agent riporta i mount visibili *dentro* il container, sotto il loro percorso nel container. |
| Contatori di rete dell'host | `network_mode: host` | Le interfacce sono in namespace separati: senza di esso l'agent vede un solo `veth`. |
| Monitor ping inoltrati tramite questo agent | `sysctls: net.ipv4.ping_group_range: "0 2147483647"` | ICMP usa un socket echo, consentito da questo sysctl: nessuna capability richiesta. Vedi [Ping ICMP senza capability](../install/docker.md#icmp-ping-without-a-capability). |
| CA privata | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Server dietro un reverse proxy interno, monitor con certificati autofirmati. |
| Mantenere l'associazione tra le ricreazioni | `- agent-state:/var/lib/dumbmonit-agent` e `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | Il segreto di associazione vive nel filesystem del container, che un `docker compose up --force-recreate` getta via. Senza il volume, l'agent torna non associato e qualcuno deve consentire una nuova registrazione dalla pagina del monitor ogni volta. |

`pid: host` e `privileged: true` non servono mai: l'agent non raccoglie metriche
per processo.

### Modalità relay: monitorare un altro sito

Un agent può fare più che sorvegliare la propria macchina. Con `relay: true` (o
`DUMBMONIT_AGENT_RELAY=true`), il server gli **delega i probe**: i monitor la
cui impostazione *Raggiunto tramite* indica questo agent non vengono interrogati
dal server ma dall'agent, dalla sua rete, con esattamente gli stessi collector —
SNMP, Proxmox VE, Proxmox Backup Server, Synology, HTTP, TCP, DNS, ping, TLS. I
risultati finiscono sotto il monitor come se lo avesse interrogato il server:
stesse metriche, stesso *ultimo probe*, stessi avvisi. Una sede remota, un secondo
homelab o la rete di un cliente compaiono così nell'unico DumbMonit che hai già,
senza VPN e senza nulla da aprire sul lato remoto.

Come configurarlo, passo per passo, è descritto in [Monitorare un sito remoto](../install/remote-site.md).

## Associazione: una macchina, un agent {#binding-one-machine-one-agent}

Un token di registrazione dice *questa macchina è autorizzata a parlare*. Non dice
*quale* macchina sta parlando — a questo serve la chiave di identità
(`/etc/machine-id`, o il nome host), e quella chiave non è un segreto: si legge su
qualsiasi macchina in pochi secondi.

Quindi con il primo batch il server consegna a ogni macchina un **segreto di
associazione** proprio. Ne conserva solo un'impronta; l'agent scrive il segreto in
`/etc/dumbmonit/agent-secret` con modo `0600` e lo presenta nell'header
`X-DumbMonit-Agent-Secret` a ogni richiesta successiva — misure, comandi per i
container e probe inoltrati. Da quel momento:

- nessun'altra macchina può inviare misure sotto questo monitor, né riscriverne
  il nome host e il sistema operativo;
- nessun'altra macchina può recuperare — e quindi consumare — i comandi Riavvia e
  Aggiorna messi in coda per esso.

La pagina del monitor mostra a che punto è una macchina, sotto **Agent →
Associazione**:

| Stato | Cosa significa | Cosa fare |
|---|---|---|
| **Associato** | L'agent possiede un segreto proprio. | Niente. |
| **Non associato — registra di nuovo questo host** | Un host registrato prima che esistesse l'associazione, e mai associato da allora. Il server rifiuta le sue misure, i suoi comandi per i container e i probe inoltrati. | Fai clic su **Consenti nuova registrazione** nella pagina del monitor, poi riavvia l'agent. Se l'agent è anteriore all'associazione, riesegui prima il comando di installazione sull'host. |

### Host anteriori all'associazione {#hosts-that-predate-binding}

!!! warning "Cambiato: gli host non associati vengono rifiutati"
    Le versioni precedenti continuavano ad accettare gli host registrati prima
    dell'associazione, finché usavano il token che li aveva registrati, e li
    associavano in silenzio una volta aggiornato il loro agent. Quella finestra di
    transizione è chiusa: un host che non si è mai associato viene ora **rifiutato
    su ogni rotta dell'agent** — misure, comandi per i container e probe inoltrati
    — finché un amministratore non lo registra di nuovo. Un token di flotta è
    condiviso da molte macchine e la chiave di identità può essere indovinata,
    quindi nient'altro prova che l'agent che chiede sia quello dell'host stesso.

    Viene rifiutato anche un binario dell'agent troppo vecchio per ricevere un
    segreto di associazione, anche alla sua prima registrazione.

Per riportare un host di questo tipo:

1. Se il suo agent è anteriore all'associazione, riesegui su di esso il comando di
   installazione (il binario dell'agent deve poter memorizzare un segreto di
   associazione).
2. Apri la pagina del suo monitor e fai clic su **Agent → Consenti nuova
   registrazione**. Questo apre una finestra di un'ora.
3. Riavvia l'agent, o attendi il suo prossimo tentativo: quel batch associa
   l'host con un segreto nuovo e chiude la finestra. Un token diverso da quello
   che ha registrato l'host deve comunque poter registrare (non esaurito, non
   scaduto).

Fino ad allora, il log dell'agent riporta la spiegazione del server:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Nuova registrazione dopo una reinstallazione {#re-enrolment-after-a-reinstall}

Se la macchina viene ricostruita, il suo disco sostituito, o il container del suo
agent ricreato senza il volume che contiene il segreto, l'agent torna senza di
esso e il server rifiuta i suoi batch. Quel rifiuto è voluto: una macchina
associata resta associata finché un essere umano non dice altrimenti, altrimenti
l'associazione non proteggerebbe nulla.

La via per rientrare è nella pagina del monitor: **Agent → Consenti nuova
registrazione**. Apre una finestra di un'ora durante la quale un agent che
presenta un token di registrazione valido associa di nuovo quella macchina, con
un segreto nuovo; il vecchio segreto smette di funzionare, e la finestra si chiude
non appena viene usata. Nel frattempo l'agent riprova da solo, quindi non c'è
nulla da riavviare sulla macchina — e nulla va perso, poiché le sue letture
restano nel buffer finché il collegamento non viene accettato di nuovo.

Finché qualcuno non apre quella finestra, il log dell'agent dice cosa fare:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Token di registrazione

- I token vengono creati in **Impostazioni → Agent** o quando si aggiunge un
  monitor di tipo agent. Il server ne conserva solo un'impronta: il token in
  chiaro viene mostrato una volta, nella risposta di creazione, con i due comandi
  di installazione.
- L'agent lo invia come `Authorization: Bearer dmon_…` su `POST /api/ingest`.
  Non compare mai nei log dell'agent, nemmeno troncato.
- **Impostazioni → Agent** elenca i token con il loro prefisso, ambito, data di
  creazione e ultimo utilizzo. **Revoca** ferma ogni agent che usa quel token al
  suo prossimo invio; le macchine diventano poi *irraggiungibili* dopo tre
  periodi saltati. Riesegui l'installer con un nuovo token per registrarle di
  nuovo.
- Le macchine sono monitor separati: revocare un token non le elimina.

### Monouso o flotta {#single-use-or-fleet}

L'ambito di un token si sceglie alla sua creazione, ed è una scelta, non un
valore predefinito che ti capita:

| Ambito | Cosa registra | Quando |
|---|---|---|
| **Monouso** (il predefinito) | Esattamente una macchina. | Un'installazione. Un comando di installazione sopravvive nella cronologia della shell, in un ticket, in un log di chat; uno monouso non vale nulla una volta usato. |
| **Riutilizzabile per una flotta** | Tante macchine quante ne consenti — un numero, o nessun limite. | Un playbook, un'immagine di macchina, un lotto di installazioni. |

Entrambi possono avere una scadenza in giorni. Una scadenza ferma solo le
*registrazioni*: le macchine già registrate continuano a riferire dopo il suo
passaggio, ed è ciò che serve per una finestra di installazione. È la revoca a
tagliare fuori una flotta.

Un token che non può più registrare — esaurito, o oltre la scadenza — è mostrato
come **Non registra più** in Impostazioni → Agent. Le sue macchine continuano a
riferire; semplicemente non può far entrare una nuova macchina.

## Interruzioni e diagnostica

Quando il server è irraggiungibile, l'agent continua a misurare e mette in buffer
le sue letture in memoria con i timestamp originali. Vengono inviate non appena il
collegamento torna: un riavvio del server non lascia buchi nei grafici. Il buffer
è limitato (`max_buffered_samples`, circa un'ora per impostazione predefinita) e
scarta le letture più vecchie quando è pieno. I tentativi aumentano il ritardo da
5 secondi a 5 minuti con jitter.

```sh
dumbmonit-agent --dry-run          # print the readings, send nothing
dumbmonit-agent --once             # send one batch then exit
journalctl -u dumbmonit-agent -f   # service log (systemd)
tail -f /var/log/dumbmonit-agent.log   # service log (OpenRC)
```

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| L'installer dice "could not reach the server" | URL o token errati. La configurazione è scritta: correggi `agent.yaml`, poi riavvia il servizio. |
| Il download fallisce con 404 | Il server non ha binari dell'agent (`DUMBMONIT_AGENT_DIR` vuoto). Usa `--bin=PATH` con un binario compilato da te. |
| L'installer si ferma con "checksum mismatch" | Il comando è stato copiato prima di un aggiornamento del server (copiane uno nuovo da Impostazioni → Agent), oppure il binario ricevuto non è quello servito dal server: un proxy o una cache di mezzo, o un download manomesso. |
| L'installer si ferma con "no expected checksum" | Il comando non ha `--sha256` per questa piattaforma: un vecchio comando, o un binario macOS (non distribuito dal server). Copia un comando nuovo, oppure passa `--sha256=<platform>-<arch>:<hex>` dal `.sha256` della release. |
| La macchina non compare mai | L'invio va a `/api/ingest` sull'URL del comando di installazione; dietro un reverse proxy, assicurati che sia inoltrato e che siano consentiti corpi fino a 16 MB. |
| *Irraggiungibile* sebbene l'agent giri | Il token è stato revocato, oppure gli invii vengono rifiutati. Controlla `journalctl -u dumbmonit-agent`: l'errore viene registrato lì. |
| Il log dice "this agent is not recognised for this machine" | La macchina è associata a un'installazione dell'agent il cui segreto questo non possiede — una reinstallazione, o un container ricreato senza il suo volume di stato. Apri la pagina del monitor e fai clic su **Consenti nuova registrazione**; vedi [Nuova registrazione dopo una reinstallazione](#re-enrolment-after-a-reinstall). |
| Il log dice "this machine was enrolled before agent binding" | Un host che non si è mai associato al suo agent. Fai clic su **Consenti nuova registrazione** nella pagina del suo monitor, poi riavvia l'agent; vedi [Host anteriori all'associazione](#hosts-that-predate-binding). |
| Il log dice "this agent is too old to be bound to its machine" | Il binario dell'agent non può memorizzare un segreto di associazione. Riesegui il comando di installazione sull'host; se il monitor esiste già, fai prima clic su **Consenti nuova registrazione**. |
| Il log dice "this enrollment token has already enrolled all the machines it was allowed to" | Un token monouso che viene riutilizzato. Creane uno nuovo, o un token riutilizzabile in Impostazioni → Agent. |
| Windows | Le parti del servizio Windows non erano state messe alla prova nell'immagine di build del progetto al momento della scrittura; segnala i problemi su GitHub. |

## Container Docker {#docker-containers}

Quando l'agent può leggere il socket Docker (`docker: true`, il predefinito, e un
utente nel gruppo `docker` oppure root — `usermod -aG docker dumbmonit`, poi
riavvia l'agent), ogni container diventa una manciata di serie, con le etichette
`container` e `image` (il `name:tag` mostrato da `docker ps`). Se la pagina del
monitor dice "No Docker on this machine" mentre Docker è installato, è la prima
cosa da controllare.

| Metrica | Significato |
|---|---|
| `container_up` | 1 in esecuzione, 0 altrimenti |
| `container_health` | 0 nessun healthcheck, 1 sano, 2 non sano, 3 in avvio |
| `container_restart_count` | Contatore dei riavvii di Docker per il container |
| `container_started_seconds` | Secondi dall'ultimo avvio del container (0 quando è fermo) |
| `container_image_age_seconds` | Secondi dalla creazione dell'immagine |
| `container_update_available` | 1 quando il registry ha un'immagine più recente per lo stesso tag, 0 quando è aggiornato, -1 sconosciuto |

Il controllo degli aggiornamenti confronta il digest dell'immagine locale con
quello che il registry annuncia per il tag (`HEAD /v2/<name>/manifests/<tag>`,
flusso con token anonimo su Docker Hub e GHCR). Gira in background al massimo una
volta all'ora per immagine e non ritarda mai il ciclo di 30 secondi; un registry
privato che rifiuta l'accesso anonimo risulta semplicemente -1. Disabilitalo con
`docker_update_check: false` (`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Regole integrate: **Container fermo** (`container_up == 0` per 2 minuti),
**Container non sano** (`container_health == 2` per 3 minuti), **Container in
riavvio continuo** (tre o più riavvii in 15 minuti), **Aggiornamento del container
disponibile** (info, dopo un'ora, ripetuto ogni giorno). L'avviso riporta il nome
del container.

### Azioni

Nella pagina del monitor, ogni container ha due azioni e due interruttori. Le
azioni viaggiano attraverso un **canale di comandi**: il server mette il comando in
coda, l'agent lo recupera dopo il suo prossimo batch (`GET /api/agent/commands`,
stesso token di registrazione), lo esegue e riporta l'esito
(`queued → running → done | failed`), con un breve log che puoi aprire
nell'interfaccia. I comandi più vecchi di dieci minuti non vengono mai eseguiti;
l'agent esegue un comando alla volta.

Un comando in coda può essere **annullato** dall'interfaccia finché l'agent non lo
ha preso in carico. Un comando che nessuno è venuto a recuperare **scade** dopo
dieci minuti, sul server, che l'agent faccia polling o no — così una macchina il
cui agent è fermo non tiene mai un "In coda" obsoleto per sempre, e non blocca mai
l'azione successiva su quel container.

L'agent comunica al server, a ogni batch, se accetta comandi. Quando non li
accetta — un agent anteriore al canale di comandi, o installato con
`commands: false` — la pagina del monitor lo dice invece di mostrare Riavvia e
Aggiorna, l'API rifiuta i comandi (`409`), e le politiche non mettono nulla in
coda. Reinstalla l'agent con l'installer attuale per abilitare le azioni.

- **Riavvia** — `docker restart` per un container in esecuzione, `docker start`
  per uno fermo, poi un controllo di 20 secondi che sia in esecuzione.
- **Aggiorna ora** — scarica (pull) il tag che il container usa già, crea un nuovo
  container con la stessa configurazione (ambiente, mount, porte, label, reti,
  politica di riavvio), ferma il vecchio, scambia i nomi, avvia il nuovo e attende
  il suo healthcheck (o 20 secondi più un controllo di esecuzione quando
  l'immagine non ne ha uno). Se qualcosa fallisce, l'agent esegue un **rollback**:
  il nuovo container viene rimosso e il vecchio riavviato, e il comando riporta
  `failed` con il motivo. In caso di successo il vecchio container viene rimosso e,
  quando "prune" è attivo, anche la vecchia immagine — a meno che un altro
  container la usi ancora.

I container gestiti da Compose (label `com.docker.compose.*`) vengono aggiornati
allo stesso modo, ma il risultato avverte che il successivo `docker compose up`
ricreerà il container dal file compose. I container il cui nome inizia con
`dumbmonit` o `dumbmonit` sono rifiutati: il monitor non aggiorna mai sé stesso.

### Politiche

Una **politica** per container viene valutata dal server ogni minuto:

- **Riavvia se fermo** — un container che riporta `container_up == 0` viene
  riavviato automaticamente, al massimo una volta ogni dieci minuti.
- **Aggiornamento automatico nelle finestre di manutenzione** — un container con
  un aggiornamento disponibile viene aggiornato automaticamente, ma solo mentre una
  finestra di manutenzione (silenziamento) copre il monitor, al massimo una
  volta ogni sei ore. Pianificane una nella pagina Avvisi, o silenzia il
  monitor dalla sua pagina. Le politiche per impostazione predefinita
  eliminano la vecchia immagine e agiscono solo nelle finestre di manutenzione.

Due posti le impostano:

- la **barra Docker** sotto l'intestazione del monitor: "28 container · 27 in
  esecuzione · 3 aggiornamenti disponibili · politiche: 2 riavvio automatico, 0
  aggiornamento automatico", l'ultima azione e il suo stato, **Gestisci container**
  (apre l'elenco qui sotto) e **Politiche…**, una tabella di ogni container con i
  due interruttori e una riga "Applica a tutti" — il modo più rapido per attivare il
  riavvio automatico su un'intera macchina;
- gli interruttori all'interno di ogni riga della sezione **Container**, accanto ai
  pulsanti Riavvia e Aggiorna ora.

Ogni interruttore si salva da sé (`PUT /api/targets/<id>/containers/<name>/policy`);
un errore riguarda solo quella riga ed è mostrato accanto ad essa.

Le azioni automatiche compaiono in "Azioni recenti" come richieste da `policy`.

### Regole di sicurezza

- L'agent agisce solo quando `commands: true` (il predefinito); impostalo a
  `false` per una macchina che deve essere soltanto misurata.
- L'agent recupera da sé i comandi, con il proprio token di registrazione —
  nulla si connette alla macchina. Scadono dopo dieci minuti e vengono eseguiti
  uno alla volta; l'agent non scarica mai un tag diverso da quello che il
  container usa già.
- Un aggiornamento che non torna sano subisce un rollback al container
  precedente, e la vecchia immagine viene rimossa solo dopo che il nuovo
  container è in esecuzione.
- I container chiamati `dumbmonit*` o `ezymonit*` sono rifiutati, a mano o per
  politica: il monitor non agisce mai sui propri container. Il controllo viene
  fatto due volte — il server rifiuta di mettere in coda il comando, e l'agent
  rifiuta di eseguirlo — e l'agent lo ripete sul nome che Docker riporta dopo
  aver ispezionato il container, quindi indicarlo con il suo id non cambia nulla.
  L'agent rifiuta inoltre di agire sul proprio container, comunque si chiami: non
  ci sarebbe più per finire il lavoro o riferire su di esso.
- I container gestiti da Compose vengono aggiornati, ma il successivo
  `docker compose up` li ricrea dal file compose: aggiorna il tag anche lì.

## Backup Plakar

L'agent può sorvegliare i kloset (repository) di [Plakar](https://plakar.io): ogni
dieci minuti, in background, esegue `plakar at <kloset> ls` e `info` su ciascuno e
riemette l'ultima lettura a ogni ciclo.

Plakar viene **rilevato, mai presupposto**. All'avvio e poi a ogni ciclo finché non
trova qualcosa, l'agent cerca il binario `plakar` (nel `PATH`, o `plakar_bin`) e i
kloset nelle posizioni standard. Quando non c'è alcun binario, alcun kloset e
alcuna traccia di Plakar (`~/.cache/plakar`), l'agent non emette alcuna serie
`backup_*`, lo dice una volta a livello `debug`, e la pagina del monitor non
mostra alcun pannello Backup — niente da configurare sulle macchine senza Plakar.
Per saltare del tutto il rilevamento:

```yaml
plakar: false                        # or DUMBMONIT_AGENT_PLAKAR=false
```

### Rilevamento

Niente da configurare: quando `plakar_klosets` è vuoto, l'agent trova da sé i
kloset, in quest'ordine, e legge ogni posizione una sola volta:

1. la directory di configurazione propria dell'agent — `$XDG_CONFIG_HOME/plakar`,
   o `~/.config/plakar` dell'account con cui gira (`plakar_home` se impostato);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, in ordine alfabetico;
4. `/var/lib/plakar`, quando quella directory esiste.

In ciascuna directory legge `stores.yml` (o `stores.yaml`; è accettato anche il
vecchio `plakar.yml`) e prende ogni voce della mappa `stores` (anche
`repositories` e `klosets`) che abbia una `location`. Viene letta solo la
location: le passphrase e i segreti di rclone che stanno accanto non vengono mai
conservati né registrati nei log. Ogni voce diventa il kloset `@<name>`, letto con
`plakar -configdir <dir> at @<name>`, così un kloset creato da un utente viene letto
anche quando l'agent gira come root. `~/.plakar` di ogni account viene aggiunto
quando esiste (il kloset predefinito di `plakar create`). Due account che chiamano
allo stesso modo kloset diversi vengono distinti come `@name` e `<user>:@name`.

Il risultato viene registrato una volta a livello `info` (e di nuovo solo quando
cambia). Due gauge, emessi solo una volta rilevato Plakar, permettono
all'interfaccia di distinguere i casi rimanenti: `backup_plakar_present` (1 quando
il binario funziona) e `backup_klosets_found` (numero di kloset letti). La pagina
del monitor dice allora "Plakar is installed but no kloset was found", oppure
elenca i kloset. Quando i kloset esistono ma il binario non funziona (tipicamente
un `PATH` del servizio senza `plakar`), vengono elencati come illeggibili con un
suggerimento di impostare `plakar_bin`, e l'agent avvisa una volta.

Per sorvegliare invece un elenco fisso, indicalo — il rilevamento viene allora
saltato:

```yaml
plakar: true                         # default: detect Plakar; false hides it entirely
plakar_bin: /usr/bin/plakar          # default: "plakar" on PATH
plakar_klosets:                      # explicit list: overrides discovery
  - /srv/backups/main
  - "@diskext"                       # a named store of the agent's account
plakar_home: /home/noe               # optional: HOME (and config dir) used when running plakar
plakar_interval_secs: 600            # minimum 60
```

| Variabile | Effetto |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — sorveglia i backup Plakar quando rilevato (predefinito `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Percorso del binario `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Kloset, separati da virgole (sostituisce il rilevamento) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Directory home usata quando si esegue `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Secondi tra due letture |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — controllo del registry per gli aggiornamenti dei container |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — accetta azioni dal server |

Un kloset cifrato richiede la sua passphrase: mettila nella voce dello store
(`plakar store set <name> passphrase=…`, che è ciò che fa `plakar store add`) oppure
esporta `PLAKAR_PASSPHRASE` nell'ambiente del servizio dell'agent. Senza di essa il
kloset risulta "Cannot be opened" (`backup_last_status = 0`).

### Metriche e regole

Metriche, con le etichette `kloset` e `source` (la directory di cui è stato fatto
il backup): `backup_last_success_seconds` (età dello snapshot più recente),
`backup_snapshot_count`, `backup_last_status` (1 quando lo snapshot più recente non
ha errori, 0 quando ha errori o il kloset non può essere aperto),
`backup_size_bytes{kloset}` (dimensione di archiviazione del repository), più
`backup_plakar_present` e `backup_klosets_found` descritti sopra. Nessuna di
esse esiste finché Plakar non viene rilevato, quindi le regole integrate qui sotto
non possono scattare su una macchina senza Plakar.

Regole integrate: **Backup Plakar troppo vecchio** (avviso, nessuno snapshot da più
di due giorni) e **Backup Plakar fallito** (avvertenza). La pagina del monitor
mostra un blocco per kloset — età dell'ultimo backup, numero di snapshot,
dimensione, ultimo stato — e una riga per sorgente, con un suggerimento "Backup too
old" oltre i due giorni.

### Eseguire un primo backup

Con Plakar 1.1 installato sulla macchina (`plakar version`), come utente che
possiederà i backup:

```sh
# 1. a kloset store, named so the agent finds it in ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # or any location Plakar accepts
plakar at @diskext create                                   # add -plaintext to skip encryption

# 2. a first backup, then check it
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` e `plakar at <path> backup <dir>` funzionano allo stesso
modo con una directory semplice al posto di `@name`; `plakar create` senza `at`
usa `~/.plakar`, che l'agent trova a sua volta. Pianifica `plakar at @diskext backup
/srv/photos` con cron o un timer systemd: l'agent riporta l'età dello snapshot più
recente, e avvisa quando passa un giorno senza uno — due giorni per la regola
integrata.

Per provarlo in locale con un kloset non cifrato:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

poi aggiungi `plakar_klosets: ["/tmp/plakar-demo"]` a `agent.yaml` (un percorso
semplice fuori da una home non viene rilevato) e verifica con
`dumbmonit-agent --dry-run` che compaiano campioni `backup_*`.

## Backup restic e Borg {#restic-and-borg-backups}

L'agent può sorvegliare i repository [restic](https://restic.net) e
[Borg](https://www.borgbackup.org): ogni dieci minuti, in background, apre ogni
repository che dichiari e riporta se è stato possibile leggerlo e quanto è vecchio
il suo snapshot più recente. Nulla viene rilevato: un repository non ha una
posizione standard e non può essere aperto senza la sua password, quindi ognuno
viene elencato in `agent.yaml`, con il modo di ottenere la sua password.

**La password non lascia mai la macchina.** L'agent la passa a `restic` o `borg`
solo attraverso l'ambiente del processo figlio; non viene mai registrata nei log,
mai inviata al server, e le serie portano soltanto il nome del repository.

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

Mantieni `agent.yaml` e i file delle password leggibili solo da root
(`chmod 600`), come fa l'installer per `agent.yaml`. `password_command` corrisponde
a `RESTIC_PASSWORD_COMMAND` o `BORG_PASSCOMMAND`. Tutto ciò che gli strumenti
leggono dal loro ambiente può andare sotto `env` (`RESTIC_PASSWORD`,
`BORG_PASSPHRASE`, chiavi cloud, `BORG_RSH`…), e viene passato anche l'ambiente del
servizio dell'agent stesso.

Cosa esegue l'agent, in sola lettura nel senso più rigoroso:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: nessun lock viene
  scritto, quindi un repository in sola lettura o append-only può essere
  sorvegliato, e un `prune` che detiene il lock esclusivo non fa sembrare il
  repository fermo; nessuna cache neppure, poiché il servizio installato non può
  scrivere in `/root`.
- `borg --bypass-lock list --json --last 1`: senza `--bypass-lock`, la lettura
  fallirebbe per tutta la durata di ogni `borg create`, che detiene il lock
  esclusivo. Borg scrive un piccolo record di sicurezza sotto `~/.config/borg`;
  quando quella home non è scrivibile (il servizio installato gira con
  `ProtectHome=yes`), l'agent punta invece `BORG_BASE_DIR` al proprio `/tmp`
  privato, a meno che tu non imposti `BORG_BASE_DIR` da te. Un repository in
  modalità `keyfile` richiede allora `BORG_KEY_FILE` nel suo `env`.

Poiché il servizio gira con `ProtectHome=yes`, tieni ciò di cui gli strumenti hanno
bisogno fuori dalle directory home: file delle password, chiavi SSH e
`known_hosts` sotto `/etc/dumbmonit/`, come nell'esempio, e i repository locali
sotto `/srv`, `/mnt` o `/var` anziché `/home`. Le letture vengono eseguite una dopo
l'altra, entro il limite di memoria del servizio (128 MiB con l'agent stesso).

Metriche, con le etichette `tool` (`restic` o `borg`) e `repo`:
`agent_backup_repo_reachable` (1 quando l'ultima lettura è riuscita, 0 altrimenti:
repository irraggiungibile, password rifiutata, binario mancante — l'agent registra
il motivo una volta, e di nuovo quando cambia),
`agent_backup_repo_last_snapshot_age_seconds` (età dello snapshot più recente,
assente finché il repository è vuoto), e `agent_backup_repo_last_snapshot_bytes`
(byte elaborati dallo snapshot restic più recente, restic 0.17 e successivi). Con
`--latest 1`, restic restituisce lo snapshot più recente di ogni host e insieme di
percorsi; viene mantenuto il più recente fra tutti, quindi imposta `host` su un
repository condiviso da più macchine.

Regole integrate: **Backup restic/Borg troppo vecchio** (nessuno snapshot da più di
48 ore) e **Repository restic/Borg irraggiungibile** (per 30 minuti). La pagina
del monitor mostra un pannello Repository di backup, una riga per repository.

Per verificare una dichiarazione, esegui l'agent una volta in primo piano e cerca
i campioni `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Tunnel WireGuard {#wireguard-tunnels}

Su una macchina che esegue WireGuard, l'agent legge ogni interfaccia e ogni peer
con `wg show all dump`, a ogni ciclo. Niente da configurare: senza il comando `wg`
(wireguard-tools) o senza un'interfaccia, non viene emessa alcuna serie né mostrato
alcun pannello. `wg` richiede root (`CAP_NET_ADMIN`); il servizio installato ce
l'ha, un agent avviato a mano con il tuo account registra il rifiuto una volta e
non riporta nulla.

Il dump contiene anche la chiave privata dell'interfaccia e la chiave
preshared di ogni peer: l'agent salta entrambi i campi senza mai copiarli.

Per peer, con le etichette `interface`, `peer` (la sua chiave pubblica) e
`allowed_ips`: `agent_wireguard_peer_handshake_age_seconds` (secondi dall'ultimo
handshake), `agent_wireguard_peer_has_handshake` (0 finché il peer non ne ha mai
completato uno), `agent_wireguard_peer_keepalive_seconds` (il suo keepalive
persistente, 0 se disattivato), e i contatori `agent_wireguard_peer_rx_bytes` e
`agent_wireguard_peer_tx_bytes`; per interfaccia, `agent_wireguard_interface_peers`.
Per un peer che non ha mai completato un handshake, l'età conta dal momento in cui
l'agent lo ha visto per la prima volta: anche un tunnel che non si è mai attivato è
silenzioso, e viene segnalato come tale.

WireGuard non ha nomi per i peer. Dai i tuoi in `agent.yaml`, per chiave pubblica, e
diventano l'etichetta `name`:

```yaml
wireguard: true                     # default; false stops even looking
wireguard_bin: wg                   # default: on PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

La regola integrata **Peer WireGuard silenzioso** scatta quando un peer **con
keepalive persistente** non completa un handshake da più di 15 minuti. Finché un
tunnel è attivo, WireGuard rinnova il suo handshake ogni due minuti, e un keepalive
lo mantiene attivo anche senza traffico; un peer senza keepalive (un telefono, un
portatile) può restare inattivo per giorni senza che nulla sia rotto, quindi viene
escluso. Imposta `PersistentKeepalive = 25` sui tunnel che devono restare attivi
(collegamenti site-to-site, gateway remoti). N è la soglia della regola:
modificala in **Avvisi → Regole**, oppure rimuovi la parte `and on (…)` della sua
query per sorvegliare ogni peer. La pagina del monitor mostra un pannello
WireGuard con ogni peer, il tempo dall'ultimo handshake, il suo traffico e il suo
keepalive.

## Tunnel IPsec {#ipsec-tunnels}

Su una macchina che esegue strongSwan, l'agent legge ogni tunnel configurato con
due comandi di sola lettura: `swanctl --list-conns`, per i CHILD_SA dichiarati in
`swanctl.conf` (oppure in `ipsec.conf` tramite `swanctl --load-all`, il livello di
compatibilità di strongSwan stesso — all'agent non importa quale dei due abbia
scritto la configurazione), e `swanctl --list-sas`, per quelli effettivamente
installati. Niente da configurare: senza il comando `swanctl` o senza una
connessione configurata, non viene emessa alcuna serie né mostrato alcun pannello.
L'agent non invia mai `--initiate`, `--terminate` né alcun altro comando che
cambierebbe lo stato di un tunnel; legge soltanto ciò che `swanctl` mostra già
sulla riga di comando.

Leggere entrambi i comandi, e non solo quello attivo, permette di segnalare come
non attivo un tunnel che **non si è mai** avviato, invece di renderlo invisibile:
un CHILD_SA dichiarato nella configurazione ma assente da `--list-sas` conta come
non attivo fin dal primo ciclo.

Per tunnel, con le etichette `connection` (il nome dell'IKE_SA) e `tunnel` (il
nome del CHILD_SA): `agent_ipsec_tunnel_up` (1 installato, 0 altrimenti) e, solo
mentre è attivo, i contatori `agent_ipsec_tunnel_bytes_in` / `agent_ipsec_tunnel_bytes_out`
e `agent_ipsec_tunnel_rekey_seconds` (tempo rimanente prima del prossimo rekey).

```yaml
ipsec: true                # default; false stops even looking
ipsec_bin: swanctl          # default: on PATH
```

La regola integrata **Tunnel IPsec non attivo** scatta quando un tunnel
configurato non ha un CHILD_SA installato da dieci minuti. La pagina del
monitor mostra un pannello IPsec con ogni tunnel, attivo o no, il suo
traffico e il tempo al prossimo rekey.

## Speed test di Internet {#internet-speed-test}

Disattivato per impostazione predefinita: a differenza di tutto il resto ciò che
l'agent misura, questo test **consuma banda** — e, su una connessione a consumo o
con limiti, denaro — a ogni esecuzione. Per attivarlo servono sia `speedtest: true`
sia un indirizzo di download; senza l'indirizzo non viene eseguito nulla, anche se
`speedtest` è `true`.

**Nessun server viene scelto al posto tuo.** DumbMonit non include alcun endpoint
predefinito e non contatta mai una terza parte a meno che tu non ne indichi una:
sei tu a decidere rispetto al server di chi viene misurata la tua linea e chi vede
l'indirizzo dell'agent mentre il test è in corso.

Il test è HTTP semplice, in sola lettura dall'altra parte: una `GET` su
`speedtest_download_url`, letta a blocchi e interrotta non appena sono stati letti
`speedtest_download_bytes_cap` byte (predefinito 50 MB, a seconda di quale evento
si verifica per primo), dà la velocità di download; una `POST` di
`speedtest_upload_bytes` byte (predefinito 10 MB) a `speedtest_upload_url`, se
impostato, dà la velocità di upload; una manciata di richieste `HEAD` all'indirizzo
di download dà la latenza (il round trip più breve) e il jitter (la differenza tra
il più breve e il più lungo). Un server che rifiuta `HEAD` lascia semplicemente
latenza e jitter non misurati — il resto del test non ne dipende.

Funziona con un backend [LibreSpeed](https://github.com/librespeed/speedtest)
auto-ospitato (`backend/garbage.php?ckSize=N` per il download, `backend/empty.php`
per l'upload), con gli [endpoint di speed test di Cloudflare](https://speed.cloudflare.com/)
(`https://speed.cloudflare.com/__down?bytes=N` e
`https://speed.cloudflare.com/__up`, che chiunque può chiamare senza un
account), oppure con qualsiasi file HTTP semplice ed endpoint di upload tuo —
non c'è alcun protocollo specifico di un fornitore da implementare, né una nuova
dipendenza oltre al client HTTP che l'agent già usa.

```yaml
speedtest: false                    # default: opt-in, this test uses bandwidth
speedtest_download_url: https://speed.cloudflare.com/__down?bytes=50000000
speedtest_upload_url: https://speed.cloudflare.com/__up
speedtest_download_bytes_cap: 50000000   # default; read at most this many bytes
speedtest_upload_bytes: 10000000         # default; sent in one block
speedtest_interval_secs: 21600           # default: 6 hours, minimum 300
speedtest_min_download_mbps: 50          # optional: arms "Internet download rate low"
speedtest_min_upload_mbps: 10            # optional: arms "Internet upload rate low"
speedtest_max_latency_ms: 80             # optional: arms "Internet latency high"
```

Il test viene eseguito in background, al massimo una volta ogni
`speedtest_interval_secs` (predefinito 6 ore, mai meno di 300 secondi: un valore
più piccolo ferma l'agent all'avvio); un test viene eseguito anche pochi secondi
dopo ogni avvio dell'agent; il ciclo di raccolta legge solo il suo ultimo
risultato, esattamente come le letture di restic/Borg e Plakar. `agent_speedtest_ok`
vale 0 quando il download stesso è fallito (indirizzo irraggiungibile, timeout,
errore HTTP) — l'unico errore che impedisce al test di misurare qualsiasi cosa.

Le tre opzioni di soglia sono volutamente facoltative: senza
`speedtest_min_download_mbps`, nessuno in DumbMonit sa che cosa significhi
"lento" per la tua linea, quindi **Velocità di download Internet bassa**,
**Velocità di upload Internet bassa** e **Latenza Internet alta** restano
silenziose finché non imposti quella che ti serve. Ogni soglia viene pubblicata
come serie a sé (`agent_speedtest_download_min_bytes_per_second`, e così via),
ed è con queste che le regole integrate confrontano la misura; cambia la soglia
in `agent.yaml`, non in **Avvisi → Regole**. La pagina del monitor mostra un
pannello dello speed test con l'ultima velocità di download e di upload, la
latenza, il jitter e quanto tempo fa è stato eseguito il test.

## Certificati TLS (Certbot / Let's Encrypt)

L'agent non legge direttamente `/etc/letsencrypt`. Ogni certificato che Certbot
rinnova viene servito da qualche parte — un web server, un mail server, un reverse
proxy — ed è lì che DumbMonit lo legge: aggiungi un monitor
[Certificato TLS](services.md#tls) (o uno [Sito web o API web](services.md#http),
per un sito già sorvegliato in quel modo) puntato sulla porta su cui il
certificato viene effettivamente presentato. Legge il certificato in uso, non un
file su disco, quindi intercetta anche un rinnovo riuscito in `/etc/letsencrypt` ma
mai recepito dal servizio (un reverse proxy che ha bisogno di un reload per vedere
il nuovo file è il caso tipico, ed esattamente la lacuna che conta).

Leggere dall'agent `/etc/letsencrypt/live/*/cert.pem`, la configurazione di
rinnovo e il log di Certbot è stato valutato e scartato: duplicherebbe la data di
scadenza che il monitor TLS già riporta, aggiungendo un secondo percorso locale
all'host per la stessa informazione — una cosa in più da tenere sincronizzata dopo
lo spostamento di un certificato o la ridenominazione di un dominio. Un host in cui
Certbot gira ma non serve nulla che DumbMonit possa raggiungere direttamente (un
passaggio con una CA interna, un hook di validazione DNS) è l'unico caso che questo
lascia scoperto; è anche il caso in cui nemmeno un monitor TLS potrebbe aiutare,
dato che nulla su quell'host parla TLS da sondare.
