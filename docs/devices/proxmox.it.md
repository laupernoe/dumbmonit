# Proxmox VE

Hypervisor: nodi con i loro demoni e networking, macchine virtuali,
container, storage, pool LVM thin, backup, alta disponibilità,
snapshot, replicazione, Ceph fino all'OSD, dischi fisici e pool ZFS,
aggiornamenti in attesa, cambi di pacchetto, abbonamento e certificati.
Un dispositivo può coprire un intero cluster.

## Il pannello dei nodi

La pagina del dispositivo si apre su una scheda per nodo: CPU, memoria e
file system root come tre contatori, l'uptime e la versione
`pve-manager` installata su quel nodo, poi solo cosa non va — i demoni
non in esecuzione, le interfacce dichiarate all'avvio che non sono
attive — e il riempimento di ogni pool thin e gruppo di volumi LVM. Un
nodo che ha smesso di rispondere mantiene la sua scheda e legge
*offline*.

Due righe sotto i contatori, quando il nodo le riporta: la quota di
tempo CPU spesa in attesa dello storage (**I/O wait** — un nodo al 10%
di CPU e 30% di I/O wait non ha margine rimasto, e nessun altro numero
lo dice) e la memoria che **KSM** ha recuperato condividendo pagine
identiche tra guest, che è come un nodo ospita più della sua RAM.

Due voci in più si uniscono all'elenco "cosa non va", entrambe a
parole:

* **Reboot required** — un kernel più recente è installato di quello in
  esecuzione. La scheda nomina entrambe le versioni, poiché il nodo
  continua a eseguire quello vecchio finché non viene riavviato. Questo
  proviene da `/nodes/{node}/apt/versions`, che richiede solo
  `Sys.Audit`;
* **Updates pending** — quanti pacchetti Proxmox hanno una versione
  candidata più recente, dalla stessa chiamata. Non è la stessa cosa di
  `apt/update`, che conta ogni pacchetto in attesa del sistema ma
  richiede `Sys.Modify` ed è di solito rifiutato.

Quando il cluster riporta un watchdog HA (Proxmox VE 9 e successivi),
il suo stato viene indicato accanto al titolo del pannello: *HA fencing
armed* una volta che le risorse sono consegnate ad HA, *HA fencing
standby* mentre nessuna lo è — nel qual caso nessun nodo verrà fenced,
che è normale ma da sapere prima di contarci.

Un cluster aggiornato nodo per nodo lo mostra qui: `/version` risponde
per il nodo che è stato interrogato, quindi ogni scheda porta la propria
versione e un ritardatario è visibile senza aprire una shell.

I dati provengono da `GET /api/targets/{id}/proxmox/nodes`, letti
dall'ultima sonda nello store delle metriche.

## Il pannello dei guest

![Il pannello Guests: una riga per VM e container, raggruppati per nodo](../assets/screenshots/device-proxmox-light.png){ loading=lazy }

Sotto i nodi, il pannello **Guests**: una riga per macchina virtuale e
container, raggruppati per nodo, che risponde alle domande che un
amministratore pone prima — è in esecuzione, è occupato, il suo disco è
pieno, quando è stato fatto l'ultimo backup. Ogni riga mostra:

| Colonna | Fonte | Note |
|---|---|---|
| Status | parola + targhetta: running, stopped, paused, suspended, template | Un template viene elencato ma non conta mai come "stopped". |
| Name, VMID, node | l'inventario | |
| CPU | `guest_cpu_percent`, dei core allocati al guest | Una barra più il numero di core. |
| Memory | `guest_memory_used_bytes` / `guest_memory_total_bytes` | La dimensione del balloon quando la VM ne ha uno. Una seconda riga, *+N on host*, appare quando `guest_memory_host_bytes` è significativamente sopra quello che vede il guest: un processo QEMU costa all'hypervisor più della memoria che consegna, e quel surplus è l'overhead di emulazione. Sotto 64 MiB o 5% la riga resta fuori. |
| Disk | `guest_disk_used_bytes` / `guest_disk_total_bytes` | Container: utilizzo reale. VM: dimensione sempre; utilizzo solo quando il guest agent QEMU risponde (vedi sotto), altrimenti "size only". |
| Network | `rate(guest_network_in/out_bytes)` sulle ultime sonde | |
| Uptime | `guest_uptime_seconds` | |
| Last backup | `backup_last_age_seconds` | Vuoto quando nessun backup è conosciuto. |
| HA | `ha_resource_state_info` | Solo per i guest sotto alta disponibilità. |
| Lock | `guest_locked` | Un badge, solo mentre un lock è trattenuto (`backup`, `migrate`, `snapshot`…). |

L'elenco ordina per qualsiasi colonna, filtra per nodo, stato o pool, e
ogni gruppo di nodo si piega. Cliccare una riga apre la sua CPU e
memoria sull'intervallo temporale della pagina, insieme al sistema
operativo e agli indirizzi IP visti dall'interno del guest. I dati
provengono da `GET /api/targets/{id}/proxmox/guests`, che legge
l'ultima sonda dallo store delle metriche — nulla viene chiesto
all'hypervisor.

Un lock è normale per i pochi minuti che un backup, una migrazione o
uno snapshot richiedono. Lasciato da un job interrotto, blocca ogni
operazione su quel guest — incluso il backup della notte successiva —
finché `qm unlock` (o `pct unlock`) non lo libera, motivo per cui
"Guest locked" si attiva dopo sei ore.

L'inventario proviene da `/cluster/resources`, la cache propria del
cluster, così i guest di un nodo che ha smesso di rispondere sono
comunque elencati — con lo stato `unknown` e le loro dimensioni, senza
misurazioni live — invece di svanire dal pannello nel momento in cui
vuoi guardarlo. Costa anche due chiamate API in meno per nodo rispetto
a percorrere `/nodes/{node}/qemu` e `/nodes/{node}/lxc`, che è ciò a cui
ricade l'opzione `cluster_resources` quando disattivata.

### Utilizzo del disco dentro una VM: il guest agent QEMU

Proxmox vede il disco di una VM come un volume opaco: `disk` è sempre 0
per una VM, solo la sua dimensione è conosciuta. Per leggere l'utilizzo
dall'interno, il collector chiede al guest agent QEMU
(`agent/get-fsinfo`) per ogni VM in esecuzione la cui configurazione
abilita l'agent (Options → QEMU Guest Agent). Due condizioni:

* `qemu-guest-agent` installato e in esecuzione dentro la VM (Debian e
  Ubuntu: `apt install qemu-guest-agent`; Windows: l'ISO dei driver
  VirtIO);
* il token possiede `VM.GuestAgent.Audit` sulla VM — `VM.Monitor`
  prima di Proxmox VE 9 (il ruolo `DumbMonit` sotto lo concede su `/`).

Il file system root (`/`, oppure `C:\` su Windows; altrimenti il più
grande) dà `guest_disk_used_bytes` e `guest_disk_used_percent`, la
stessa serie che ha un container, così la regola "VM or container disk
almost full" copre entrambi. Ogni file system reale ottiene anche
`guest_fs_total/used_bytes` e `guest_fs_used_percent` con un'etichetta
`mountpoint`; pseudo file system (tmpfs, squashfs, mount snap) e media
senza capacità vengono saltati. `guest_agent_enabled` dice se la
configurazione della VM abilita l'agent, `guest_agent_running` se ha
risposto. Senza l'agent o il diritto, la VM mantiene la sua dimensione e
semplicemente non ha utilizzo — mai un ingannevole "0%".

### Il sistema operativo e gli indirizzi di un guest

Proxmox conosce solo il tipo di OS scritto nella configurazione di un
guest (`l26`, `win11`), non cosa funziona davvero. Con l'opzione
`guest_os` attiva, il collector chiede a ogni guest in esecuzione cos'è:
`agent/get-osinfo` e `agent/network-get-interfaces` per una VM,
`/nodes/{node}/lxc/{vmid}/interfaces` per un container, che non
richiede nulla installato dentro. Questo dà `guest_os_info`
(`Debian GNU/Linux 12 (bookworm)`, il suo id, la sua versione, il
kernel) e fino a quattro `guest_ip_info` — gli indirizzi loopback e
link-local IPv6 vengono saltati, e una macchina con dozzine di
interfacce virtuali non riempie il database delle serie.

Entrambi vengono chiesti **al massimo una volta all'ora per guest** e
ripubblicati dalla memoria nel frattempo: un sistema operativo non
cambia tra due sonde, e un indirizzo quasi mai lo fa. Su un cluster di
cinquanta macchine in esecuzione questa è la differenza tra due chiamate
extra per sonda e due all'ora.

Quelle due chiamate agent richiedono `VM.GuestAgent.Audit`. Proxmox VE
7 non lo aveva e usava `VM.Monitor`; Proxmox VE 9 ha rimosso
`VM.Monitor` e mantiene solo i privilegi del guest agent. Senza quello
che conosce la tua versione, le due serie sono semplicemente assenti;
nulla altro cambia.

## Il pannello Ceph

Una sezione **Ceph** appare sulla pagina del dispositivo solo quando il
cluster ha effettivamente Ceph: stato di salute e le parole dietro di
esso, capacità totale e usata, quanti OSD sono up e in, una tabella di
OSD (host, classe di dispositivo, utilizzo, latenza apply e commit) e
una tabella di pool (utilizzo, size e min size, placement group rispetto
al numero che l'autoscaler scegerebbe), i file system CephFS, i flag
OSD lasciati impostati e i controlli di salute attualmente mutati.

Questi ultimi due sono quelli che una dashboard di solito nasconde.
`noout` impostato per una manutenzione e mai rimosso significa che Ceph
non ribilancerà più via un OSD morto, e un controllo mutato non appare
più in `HEALTH_OK` — in entrambi i casi il cluster appare sano mentre la
sua ridondanza silenziosamente scompare.

`GET /api/targets/{id}/proxmox/ceph` risponde `available: false` quando
il cluster non ha Ceph, che è come l'interfaccia sa di non disegnare
affatto la sezione. Il detaglio costa tre chiamate su un singolo nodo
più due a livello di cluster, qualunque sia la dimensione del cluster,
e richiede l'opzione `ceph_detail` sopra `ceph`.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_proxmox_`.

| Famiglia | Metriche | Etichette |
|---|---|---|
| Cluster | `cluster_quorate`, `cluster_nodes`, `cluster_nodes_online`, `cluster_member_online`; `version_info` (+ `version`, `release`, `repoid`) | `cluster`, `node` |
| Inventario cluster | `cluster_guests_total`, `cluster_guests_running`, `cluster_templates_total` (template contati separatamente, mai come "stopped"); `pool_guests`, uno per pool di risorse | `pool` |
| Nodi | `node_up`, `node_cpu_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_cpu_iowait_percent` (tempo passato attendendo i dischi — un nodo al 10% CPU e 30% iowait non ha margine), `node_ksm_shared_bytes` (memoria recuperata dalla deduplicazione di pagine), `node_uptime_seconds`, `node_version_info` | `node` |
| Servizi di nodo | `node_service_running`, `node_service_state_info` (+ `state`), `node_services_failed`, `node_core_services_down`; `node_pve_version_info` (+ `version`, `release`) per il nodo che ha risposto | `node`, `service` |
| Rete di nodo | `node_interface_active`, `node_interface_exists`, `node_interface_autostart`, `node_interface_mtu`, `node_interface_offline`; totale `node_interfaces_offline` | `node`, `iface`, `type` (`bridge`, `bond`, `vlan`, `eth`…) |
| LVM, pool thin e mount | `node_lvm_vg_size/free_bytes`, `node_lvm_vg_used_percent`, `node_lvm_vg_physical_volumes` (etichetta `vg`); `node_thinpool_size_bytes`, `node_thinpool_used_bytes`, `node_thinpool_used_percent`, `node_thinpool_metadata_size_bytes`, `node_thinpool_metadata_used_percent` (etichette `vg`, `pool`); `node_directory_mount_info` (+ `path`, `device`, `fstype`), `node_directory_mounts` | `node`, `vg`, `pool` |
| Guest (VM e container) | `guest_status_info` (+ `status`: `running`, `stopped`, `paused`, `suspended`, `template`), `guest_running` (non per i template), `guest_cpu_percent` (dei core allocati), `guest_cpu_count`, `guest_memory_used/total_bytes`, `guest_memory_host_bytes` (Proxmox VE 9: la memoria che la VM prende sull'host, overhead di emulazione incluso), `guest_memory_percent` (dal punto di vista proprio del guest; su Proxmox VE 9 proviene dal driver balloon, e una VM senza uno ha byte ma nessuna percentuale invece di un valore sopra il 100%), `guest_disk_total_bytes`, `guest_disk_used_bytes` e `guest_disk_used_percent` (container; VM solo tramite il guest agent), `guest_disk_read/write_bytes`, `guest_network_in/out_bytes` (contatori), `guest_uptime_seconds`; `guest_pool_info` (+ `pool`), `guest_locked` (+ `lock`; la serie esiste solo mentre il lock è trattenuto, così la sua durata si legge direttamente dal grafico) | `node`, `vmid`, `name`, `type` (`qemu` o `lxc`) |
| Dettaglio VM (VM in esecuzione) | `guest_agent_enabled`, `guest_agent_running`, `guest_balloon_bytes`, `guest_memory_guest_free_bytes` (memoria libera vista dall'interno, driver balloon), `guest_running_qemu_info` (+ `version`: il QEMU con cui la VM è stata avviata — dopo un aggiornamento dell'hypervisor resta indietro finché la VM non viene fermata e riavviata, cosa che una migrazione live non fa); per file system `guest_fs_total_bytes`, `guest_fs_used_bytes`, `guest_fs_used_percent` | `node`, `vmid`, `name`, `type`; file system: + `mountpoint`, `fstype` |
| Sistema e indirizzi del guest | `guest_os_info` (+ `os`, `os_id`, `os_version`, `kernel`), `guest_ip_info` (+ `iface`, `ip`; al massimo quattro indirizzi instradabili per guest). Aggiornato una volta all'ora | `node`, `vmid`, `name`, `type` |
| Schede di rete del guest | `guest_netdev_in_bytes`, `guest_netdev_out_bytes` (contatori), una coppia per interfaccia virtuale dove `guest_network_in/out_bytes` dà solo il totale del guest | `node`, `vmid`, `dev` (`tap100i0`, `veth200i0`) |
| Storage | `storage_active`, `storage_enabled`, `storage_used/total/avail_bytes`, `storage_used_percent` (la capacità viene pubblicata solo quando Proxmox ne riporta una: uno storage inattivo, oppure un datastore PBS la cui dimensione PVE non legge, non ne ha nessuna invece di zero byte) | `node`, `storage`, `type` (`dir`, `lvmthin`, `zfspool`, `nfs`, `pbs`…), `shared` |
| Backup (per guest) | `backup_present`, `backup_last_timestamp_seconds`, `backup_last_age_seconds`, `backup_last_size_bytes`, `backup_count`; totali `backup_guests_total`, `backup_guests_without_backup` | `vmid`, `name`, `node`, `type` |
| Backup (esecuzioni vzdump, per nodo) | `backup_job_runs`, `backup_job_failures`, `backup_job_last_ok`, `backup_job_last_timestamp_seconds`, `backup_job_last_age_seconds`, `backup_job_last_duration_seconds` | `node` |
| Job di backup (pianificati) | `backup_job_enabled`, `backup_job_next_run_seconds`, `backup_job_last_ok`, `backup_job_last_run_age_seconds` (gli ultimi due solo quando un task `vzdump` porta l'id del job, PVE 7.2+); totali `backup_jobs_total`, `backup_guests_not_covered`; `backup_covered` per guest (0 per un guest che nessun job copre) | `job`, `storage`, `schedule`; `backup_covered`: `vmid`, `name`, `node`, `type` |
| Copertura dei job di backup | `backup_job_guests`, `backup_job_volumes_included`, `backup_job_volumes_excluded`; `backup_job_guest_excluded_volumes` (+ `vmid`, `name`, `type`, `reason`), pubblicato solo per un'esclusione *inattesa* | `job` |
| HA | `ha_quorum_ok`, `ha_fencing_armed` (+ `state`: Proxmox VE 9 riporta se il watchdog è `armed` oppure in `standby` — in standby nulla verrà fenced), `ha_master_active`, `ha_lrm_active` (`node`), `ha_resources_total`; per risorsa `ha_resource_started`, `ha_resource_error` (1 nello stato `error`, `fence` o `recovery`), `ha_resource_state_info` (+ `state`) | `sid`, `node`, `vmid`, `type` |
| Manager HA | `ha_node_online`, `ha_node_status_info` (+ `status`: `online`, `fence`, `gone`, `maintenance`), `ha_master_info` (il nodo che detiene il master CRM), `ha_manager_age_seconds`; per nodo `ha_lrm_mode_info` (+ `mode`, `state`), `ha_lrm_age_seconds`, `ha_lrm_stale` (1 quando l'LRM non ha scritto per più di due minuti) | `node` |
| Snapshot | `guest_snapshot_count`, `guest_snapshot_oldest_age_seconds`, `guest_snapshot_newest_age_seconds` (età solo quando conteggio > 0); `guest_snapshot_guests_skipped` quando `max_snapshot_guests` viene superato | `vmid`, `name`, `node`, `type` |
| Replicazione | `replication_job_enabled`, `replication_job_error` (messaggio di errore o `fail_count` > 0), `replication_job_fail_count`, `replication_job_last_sync_age_seconds`, `replication_job_next_sync_seconds`, `replication_job_duration_seconds`; `replication_jobs_total` | `job`, `vmid`, `node` (fonte), `to_node` |
| Ceph | `ceph_health` (0 OK, 1 WARN, 2 ERR, 3 sconosciuto), `ceph_health_info` (+ `status`), `ceph_osds_total/up/in`, `ceph_bytes_total/used`, `ceph_used_percent`, `ceph_pgs_total`, `ceph_mons_total` | |
| OSD Ceph | `ceph_osd_up`, `ceph_osd_in`, `ceph_osd_used_percent`, `ceph_osd_used_bytes`, `ceph_osd_total_bytes`, `ceph_osd_apply_latency_ms`, `ceph_osd_commit_latency_ms`, `ceph_osd_reweight`, `ceph_osd_crush_weight`; totali `ceph_osds_down`, `ceph_osds_out` | `osd`, `host`, `device_class` |
| Pool Ceph e CephFS | `ceph_pool_used_percent`, `ceph_pool_used_bytes`, `ceph_pool_size`, `ceph_pool_min_size`, `ceph_pool_pg_num`, `ceph_pool_pg_num_optimal` (ciò che l'autoscaler scegerebbe), `ceph_pool_info` (+ `type`, `crush_rule`, `autoscale`), `ceph_pools_total`; `ceph_fs_info` (+ `metadata_pool`, `data_pool`), `ceph_fs_total` | `pool`; CephFS: `name` |
| Flag e mute Ceph | `ceph_flag` (1 mentre il flag è impostato), `ceph_flags_set`; `ceph_health_mute_info` (+ `sticky`), `ceph_health_mutes` | `flag`; mute: `code` |
| Dischi fisici | `node_disk_size_bytes`, `node_disk_smart_failed` (1 quando lo stato SMART è `FAILED`; assente quando il disco riporta `UNKNOWN`), `node_disk_health_info` (+ `health`, `used`: `LVM`, `ZFS`, `partitions`…), `node_disk_wearout_percent` (SSD e NVMe: 0 nuovo, 100 fine della vita nominale), `node_disk_temperature_celsius` (da `disks/smart`: attributo ATA 194/190, oppure la riga di temperatura di un report testuale — `Temperature:` per NVMe, `Current Drive Temperature:` per SAS) | `node`, `disk` (`/dev/sda`, `/dev/nvme0n1`), `model`, `type` (`ssd`, `hdd`, `nvme`) |
| Pool ZFS | `node_zfs_pool_degraded` (1 a meno che `ONLINE`), `node_zfs_pool_health_info` (+ `health`), `node_zfs_pool_size/alloc/free_bytes`, `node_zfs_pool_used_percent`, `node_zfs_pool_fragmentation_percent` | `node`, `pool` |
| Aggiornamenti | `node_updates_pending`, `node_updates_security_pending` (pacchetti la cui origine, etichetta del repository, suite, sezione o URL del changelog nomina un archivio di sicurezza, come `bookworm-security` di Debian). Senza `Sys.Modify` quei due non esistono; `node_pve_packages_upgradable` conta comunque i pacchetti Proxmox la cui versione candidata differisce da quella installata, e `node_reboot_required` (+ `running`, `installed`) dice che un kernel più recente è installato ma non ancora avviato — entrambi letti da `apt/versions`, che richiede solo `Sys.Audit` | `node` |
| Cambi di pacchetto | `node_packages_changed`: 0 normalmente; per un'ora dopo che una sonda vede cambiare la versione installata di un pacchetto Proxmox (`apt/versions`), il numero di pacchetti cambiati, con un'etichetta `changes` come `pve-manager 8.2.4→8.2.7, proxmox-kernel-6.8 6.8.8-2→6.8.12-2` | `node`, `changes` |
| Abbonamento e repository | `node_subscription_active`, `node_subscription_info` (+ `status`: `active`, `notfound`, `expired`…, `level`, `next_due`); `node_repository_errors` (file source illeggibili), `node_repository_warnings` (avvisi propri di Proxmox: repository enterprise senza abbonamento, repository di test), `node_repository_enabled` (+ `repo`: `enterprise`, `no-subscription`, `test`, `ceph-*`) | `node` |
| Certificati | `node_certificate_expiry_days` (negativo una volta scaduto) | `node`, `filename`, `subject` |
| Stream RRD (disattivato per impostazione predefinita) | `node_rrd_<metric>`, `guest_rrd_<metric>`, `storage_rrd_<metric>` — ogni misurazione che i server di metriche propri di Proxmox ricevono, con il proprio nome (vedi `metrics_export` sotto) | `node`; guest: `vmid`, `type`; storage: `node`, `storage` |
| Raccolta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

L'età del backup per macchina risponde al guasto più costoso di un
homelab: il backup che non è stato eseguito da settimane, scoperto il
giorno del ripristino. Viene calcolata dal log del task `vzdump` (su
`backup_lookback_days`) e, se `scan_backup_storage` è attivo, dagli
archivi trovati sugli storage di backup.

I job pianificati provengono da Datacenter → Backup:
`backup_covered` è 0 per ogni guest che Proxmox stesso elenca come non
coperto da alcun job, così una macchina creata ieri appare prima del
suo primo backup mancato invece che dopo.

Un job che copre un guest non necessariamente scrive tutti i suoi
dischi. Un volume marcato `backup=0` viene saltato ogni notte, il job
riporta comunque successo, e nessuno lo scopre finché un ripristino.
`backup_job_guest_excluded_volumes` nomina il job, il guest e il motivo
proprio di Proxmox per l'esclusione — ma solo quando l'esclusione è
una sorpresa: un drive CD-ROM, un'immagine cloudinit o una voce che non
è affatto un volume viene esclusa di progetto, contata in
`backup_job_volumes_excluded` e mai riportata per guest. Senza quella
distinzione ogni macchina con un drive virtuale genererebbe un avviso.

I demoni di un nodo vanno osservati per un motivo: `pvestatd` fermo non
rompe nulla in modo visibile — l'interfaccia web continua a mostrare i
numeri dal momento in cui è morto, e tutto appare normale.
`node_core_services_down` conta solo `pve-cluster`, `pvedaemon`,
`pveproxy`, `pvestatd`, `pve-firewall`, `corosync` e `watchdog-mux`, e
solo quelli che il nodo stesso dichiara abilitati, così un nodo
standalone senza corosync o HA non conta nulla come mancante.

Un'interfaccia conta come inattiva quando è impostata per avviarsi al
boot e non è attiva (o non esiste più affatto). Una attivata su
richiesta, senza autostart, non viene mai segnalata: la sua assenza è
normale. Un bridge che non torna dopo un riavvio tronca ogni guest
collegato ad esso mentre ognuno di quei guest funziona perfettamente, e
nulla altro lo dice.

Un pool thin LVM che si riempie rende read-only tutti i guest
memorizzati su di esso in una volta, ed eliminare file dentro i guest
non restituisce lo spazio. I suoi metadati vivono per conto loro, su un
volume molto più piccolo e spesso saturano prima, con esattamente la
stessa conseguenza — motivo per cui i due livelli di riempimento sono
serie separate e regole separate.

I cambi di pacchetto vengono rilevati dal collector stesso: ricorda le
versioni installate dalla sonda precedente (per dispositivo e nodo, in
memoria — un riavvio del server impara di nuovo senza riportare).
Solo un pacchetto presente in entrambe le sonde con una versione
diversa conta; un pacchetto appena installato o rimosso no.

Un fallimento parziale (un nodo inattivo, uno storage lento) è comunque
una sonda riuscita: ciò che poteva essere letto viene scritto e il
fallimento viene contato in `scrape_errors`. Le funzionalità
semplicemente assenti restano silenziose: nessun Ceph, nessun job di
replicazione, nessun pool ZFS, nessun guest agent, o nessun diritto di
elencare gli aggiornamenti non producono alcuna serie e nessun errore.
Lo stesso vale per tutto quanto sopra: un privilegio mancante (403), un
endpoint che un Proxmox più vecchio non ha (404 o 501), una
funzionalità non installata (Ceph risponde 500) non producono alcun
errore e nessuna metrica affatto — mai uno zero che finge di essere una
misurazione.

Le temperature della scheda non sono disponibili: l'API di Proxmox VE
non espone sensori, solo la temperatura SMART propria dei dischi.
Installa l'agent DumbMonit sul nodo se ne hai bisogno di più.

Le regole integrate che si applicano: Device unreachable, High CPU,
Disk almost full, Filesystem almost full (forecast), Backup too old,
Unusual CPU, VM or container stopped, VM or container CPU high, VM or
container memory high, VM or container disk almost full, HA resource
in error, Cluster lost quorum, Node offline, Storage almost full,
Backup job failed, Old snapshot, Replication failed, Ceph health
error, Ceph health warning, Updates pending, Security updates pending,
Proxmox packages changed, Proxmox disk wearing out, Proxmox disk SMART
failure, Proxmox ZFS pool degraded, Node certificate expiring, Thin
pool almost full, Thin pool metadata almost full, Proxmox service
down, Node network interface down, Ceph OSD down, Ceph OSD nearly
full, Ceph pool almost full, Ceph noout flag left on, HA manager not
reporting, Backup job excludes a disk, Guest locked.

### Lo stream RRD completo

`/cluster/metrics/export` è l'endpoint che i server di metriche propri
di Proxmox (InfluxDB, Graphite) consumano: tutto ciò che `pvestatd`
misura, per l'intero cluster, in una chiamata. Due cose lo rendono
utile avere.

* Porta misurazioni che nulla altro espone — in particolare la
  pressure stall information (PSI) per nodo: quanto tempo i task
  passano attendendo la CPU, il disco o la memoria. Quel numero spiega
  "tutto è lento" quando CPU e memoria appaiono entrambe a posto.
* Viene letto con `history=1` dall'ultimo punto già visto, così ogni
  punto tra due sonde viene ingerito al suo proprio timestamp. Un
  dispositivo sondato ogni cinque minuti ottiene i suoi cinque punti di
  un minuto invece di un singolo valore attuale.

È **disattivato per impostazione predefinita**, e quella è una scelta
deliberata. Quelle serie in parte ripetono quelle che il resto della
raccolta già produce con nomi migliori, quindi attivare
`metrics_export` aggiunge un gran numero di serie da memorizzare e
setacciare per un dettaglio che la maggior parte delle installazioni
non guarda mai. I nomi vengono pubblicati esattamente come Proxmox li
invia (`node_rrd_pressure_cpu_some_avg10`, `guest_rrd_netin`…) invece
che tradotti: Proxmox ne aggiunge nuovi a ogni release, e una tabella di
traduzione scritta qui sarebbe sbagliata alla successiva. Richiede
`Sys.Audit` su `/` — che il ruolo `DumbMonit` ha già quando è stato
concesso su `/` come sotto.

## Cosa preparare in Proxmox

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo.
Il principio: un utente riservato al monitoraggio, un ruolo che può
solo leggere, un token — mai l'account con cui accedi.

1. Apri una shell su qualsiasi nodo (nell'interfaccia web: seleziona il nodo, poi Shell; oppure SSH) e crea un utente riservato al monitoraggio. Non ha bisogno di una password: il token è ciò che accede.

    ```
    pveum user add dumbmonit@pve --comment "DumbMonit monitoring"
    ```

2. Crea un ruolo con solo i privilegi che il collector usa: Sys.Audit (nodi, cluster, HA, dischi e SMART, ZFS, certificati, versioni dei pacchetti, abbonamento), Datastore.Audit (storage e archivi di backup), VM.Audit (VM, container, snapshot) e VM.GuestAgent.Audit (ciò che il guest agent QEMU legge dentro una VM: utilizzo disco, sistema operativo, indirizzi IP). Nessuno di essi può modificare nulla.

    ```
    pveum role add DumbMonit --privs "Datastore.Audit Sys.Audit VM.Audit VM.GuestAgent.Audit"
    ```

3. Dai all'utente quel ruolo sull'intero cluster.

    ```
    pveum aclmod / -user dumbmonit@pve -role DumbMonit
    ```

4. Crea il token API dell'utente. La separazione dei privilegi è disattivata, così il token eredita semplicemente i diritti dell'utente.

    ```
    pveum user token add dumbmonit@pve monitor --privsep 0
    ```

5. Il comando stampa una tabella con full-tokenid e value. Copia full-tokenid nel campo Token ID di DumbMonit e value (l'UUID) nel suo campo Secret. Il secret viene mostrato una volta: se viene perso, rimuovi il token e creane uno nuovo.

    ```
    dumbmonit@pve!monitor
    ```

6. Opzionale, per contare gli aggiornamenti in attesa e le correzioni di sicurezza in attesa: Proxmox protegge quell'elenco con Sys.Modify. Concedilo solo su /nodes; senza di esso il collector salta l'elenco silenziosamente.

    ```
    pveum role add DumbMonitUpdates --privs Sys.Modify
    pveum aclmod /nodes -user dumbmonit@pve -role DumbMonitUpdates
    ```

7. Su un Proxmox più vecchio, VM.GuestAgent.Audit potrebbe non esistere ancora e il comando sopra viene rifiutato come privilegio non valido: le chiamate al guest agent sono allora coperte da VM.Monitor, che Proxmox VE 9 ha a sua volta rimosso. Usa quello che la tua versione conosce.

    ```
    pveum role add DumbMonit --privs "Datastore.Audit Sys.Audit VM.Audit VM.Monitor"
    ```

8. Preferisci l'interfaccia web? Gli stessi passaggi vivono sotto Datacenter → Permissions: Users, Roles, Add → User Permission, poi API Tokens con "Privilege Separation" deselezionato.
9. In DumbMonit, inserisci l'indirizzo di qualsiasi nodo (porta 8006 per impostazione predefinita).

!!! warning
    Non riutilizzare l'account con cui accedi: un token compromesso controllerebbe allora l'intero cluster. Il ruolo DumbMonit sopra può solo leggere. Proxmox usa anche un certificato autofirmato per impostazione predefinita: se la connessione viene rifiutata per questo motivo, spunta "Accept an unverifiable certificate" nelle opzioni.

Documentazione del fornitore: [Proxmox VE user management](https://pve.proxmox.com/wiki/User_Management).

## Credenziali

| Credenziale | Campi |
|---|---|
| API token (consigliato) | **Token ID**: utente, realm e nome del token come li mostra Proxmox, `dumbmonit@pve!monitor`. **Secret**: l'UUID mostrato una volta alla creazione del token. Nessuna scadenza, nessuna sessione aperta sull'hypervisor. |
| Username / password | `user@realm` e la password. Un ticket viene ottenuto, memorizzato in cache e rinnovato dieci minuti prima della sua scadenza di due ore. |

L'API accetta il token in due pezzi (`token_id` + `secret`) oppure,
come facevano le versioni più vecchie, come una stringa singola
`token` = `user@realm!name=secret`; entrambi vengono memorizzati nello
stesso modo. Un token completo incollato in Token ID viene separato dal
modulo.

Il ruolo `DumbMonit` sopra porta ciò che `PVEAuditor` concede per i
percorsi che il collector legge, più il privilegio del guest agent: il
collector esegue solo `GET`. Per endpoint: `Sys.Audit` per `/version`,
`/cluster/*`, `/nodes/{node}/status`, `services`, `version`, `network`,
`netstat`, `tasks`, `replication`, `certificates/info`, `disks/list`,
`disks/smart`, `disks/zfs`, `apt/versions`, `apt/repositories` e
`subscription`; `Datastore.Audit` per `storage` e
`storage/{id}/content`; `VM.Audit` per `qemu`, `lxc`, `snapshot`,
`status/current` e le `interfaces` di un container; `VM.GuestAgent.Audit`
— oppure `VM.Monitor` prima di Proxmox VE 9 — per `agent/get-fsinfo`.

Due percorsi chiedono un po' più del resto, ed entrambi sono coperti
dal ruolo come concesso nel passaggio 3 — su `/`, non su `/nodes`:

* `Sys.Audit` **su `/`** per l'inventario LVM (`disks/lvm`,
  `disks/lvmthin`, `disks/directory`) e per `/cluster/metrics/export`,
  lo stream RRD opzionale. Concesso solo su `/nodes`, le serie di
  pool thin e le serie `*_rrd_*` sono silenziosamente assenti.
* `VM.GuestAgent.Audit` per le due chiamate guest agent che leggono il
  sistema operativo e gli indirizzi di una VM (`agent/get-osinfo`,
  `agent/network-get-interfaces`). Proxmox VE 7 non aveva quel
  privilegio e usava `VM.Monitor`; Proxmox VE 9 ha eliminato
  `VM.Monitor`. Senza quello che conosce la tua versione,
  `guest_os_info` e `guest_ip_info` semplicemente non vengono
  pubblicati per le VM; i container, che non richiedono alcun agent,
  mantengono i loro indirizzi.

La sola cosa che il ruolo non copre affatto è l'elenco degli
aggiornamenti in attesa (`apt/update`), che Proxmox protegge con
`Sys.Modify` su `/nodes` — opzionale, come spiega il passaggio 6; senza
di esso il collector salta silenziosamente `node_updates_pending` e
`node_updates_security_pending`.

(Usa `-token 'dumbmonit@pve!monitor'` invece di `-user` nei comandi
`aclmod` per un token creato con "Privilege Separation" spuntato.)

Indirizzo: `10.0.0.10`, `pve.lan`, `pve.lan:8006`, `[fd00::1]` oppure
un URL completo `https://pve.example.net`. Lo schema `https` e la porta
8006 vengono aggiunti se mancanti.

## Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | API port | `8006` | Usata se l'indirizzo non fornisce una porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Proxmox fornisce un certificato autofirmato per impostazione predefinita: abilita questo se la connessione viene rifiutata per questo motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `10` | Tempo consentito per ogni chiamata API, da 1 a 120. |
| `backup_lookback_days` | Backup lookback (days) | `31` | I task di backup più vecchi non vengono esaminati, da 1 a 3650. |
| `scan_backup_storage` | Inventory backup archives | `true` | Scansiona gli storage di backup per datare l'ultimo backup di ogni macchina. Disattiva se lo storage è lento a rispondere. |
| `nodes` | Monitored nodes | *(vuoto)* | Nomi dei nodi da monitorare, separati da virgole. Vuoto: ogni nodo nel cluster. |
| `cluster_resources` | Read the cluster inventory | `true` | Una chiamata a `/cluster/resources` elenca ogni nodo, guest, storage e pool. Mantienila attiva: è la sola fonte che vede ancora i guest di un nodo che ha smesso di rispondere, e salva due chiamate per nodo. Disattivata, il collector percorre invece `/nodes/{node}/qemu` e `/nodes/{node}/lxc`, e perde i pool e i lock. |
| `services` | Watch node services | `true` | `/nodes/{node}/services` e `version`: lo stato dei demoni Proxmox di ogni nodo e la versione installata su di esso. Un `pvestatd` fermo lascia l'intero cluster mostrare numeri congelati. |
| `network` | Watch node networking | `true` | `/nodes/{node}/network` e `netstat`: bridge, bond e VLAN con il loro stato di link, più i contatori di traffico di ogni scheda di rete guest. |
| `lvm` | Watch LVM and thin pools | `true` | `disks/lvm`, `disks/lvmthin` e `disks/directory`: gruppi di volumi, pool thin (riempimento di dati *e* metadati) e mount directory gestiti da PVE. Richiede `Sys.Audit` su `/`; silenzioso altrimenti. |
| `ha` | High availability | `true` | Legge `/cluster/ha/status/current` e `manager_status`: quorum, master CRM, la vista propria del manager di ogni nodo, ogni LRM con l'età del suo ultimo report, e ogni risorsa HA. |
| `backup_jobs` | Scheduled backup jobs | `true` | Legge i job da Datacenter → Backup e i guest che nessun job copre. |
| `scan_snapshots` | Inventory snapshots | `true` | Una chiamata per guest (quattro in corso per nodo). Disattiva su cluster molto grandi. |
| `max_snapshot_guests` | Snapshot inventory limit | `200` | I guest oltre questo numero vengono saltati a ogni sonda e contati in `guest_snapshot_guests_skipped`, da 1 a 10000. |
| `replication` | Replication jobs | `true` | Stato della replicazione di storage per nodo. Silenzioso quando il nodo non ha alcun job. |
| `ceph` | Ceph health | `true` | Legge `/cluster/ceph/status`. Silenzioso quando Ceph non è installato. |
| `ceph_detail` | Watch Ceph in detail | `true` | Stato, utilizzo e latenza per OSD, utilizzo per pool, CephFS, flag OSD e controlli di salute mutati. Tre chiamate su un nodo più due a livello di cluster, qualunque sia la dimensione del cluster. Richiede `ceph` attivo; silenzioso senza Ceph. |
| `backup_volumes` | Check what backup jobs include | `true` | Per ogni job pianificato, quali guest copre e quali dei loro dischi scrive effettivamente. Rileva un job che riesce ogni notte saltando un disco dati. Una chiamata per job. |
| `guest_os` | Read guest OS and addresses | `true` | Sistema operativo e indirizzi IP visti dall'interno di ogni guest in esecuzione (guest agent QEMU per le VM, il namespace del container per i container). Aggiornato una volta all'ora per guest, non a ogni sonda. Richiede `VM.GuestAgent.Audit` su Proxmox VE 8. |
| `metrics_export` | Ingest the full RRD metric stream | `false` | Legge `/cluster/metrics/export`, lo stream che i server di metriche propri di Proxmox consumano, incluso il pressure stall per nodo (PSI), e ogni punto dall'ultima sonda invece che solo quello attuale. Disattivato per impostazione predefinita: le serie `*_rrd_*` che aggiunge in parte ripetono quelle già raccolte. Richiede `Sys.Audit` su `/`. |
| `updates` | Pending updates | `true` | Conta i pacchetti che `apt/update` elenca. Richiede `Sys.Modify` su `/nodes` (vedi sopra); silenzioso altrimenti. |
| `certificates` | Certificate expiry | `true` | Giorni rimanenti su ogni certificato di nodo (`pve-ssl.pem`, `pveproxy-ssl.pem`, `pve-root-ca.pem`). |
| `guest_agent` | Ask the QEMU guest agent | `true` | Per ogni VM in esecuzione: `status/current` (balloon, flag agent) e, quando l'agent è abilitato, `agent/get-fsinfo` (richiede `VM.GuestAgent.Audit`). Quattro chiamate in corso per nodo. Silenzioso quando l'agent è assente. |
| `disks` | Watch physical disks | `true` | `disks/list` più una chiamata `disks/smart` per disco: salute, usura, temperatura. |
| `zfs` | Watch ZFS pools | `true` | `disks/zfs`: salute, capacità, fragmentazione. Silenzioso senza ZFS. |
| `packages` | Detect package changes | `true` | `apt/versions` confrontato con la sonda precedente; un cambio viene riportato per un'ora in `node_packages_changed`. |
| `subscription` | Watch subscription and repositories | `true` | `subscription` e `apt/repositories`: stato, avvisi, repository standard abilitati. |

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| Errore di certificato mostrato sul dispositivo | Certificato autofirmato. Installa un certificato fidato (ACME è integrato in Proxmox) oppure abilita `insecure_tls`. L'opzione non viene mai abilitata implicitamente. |
| Errore di autenticazione | Il secret è stato incollato in Token ID (o il contrario), "Privilege Separation" lasciato spuntato (il token allora non ha diritti), oppure il ruolo `DumbMonit` non applicato su `/`. |
| Età del backup mancante per un guest | Nessun task `vzdump` nella finestra di osservazione e nessun archivio trovato sugli storage; oppure `scan_backup_storage` disattivato. |
| Sonde lente | Uno storage di backup che richiede molto tempo per l'elenco, oppure molti guest da inventariare per gli snapshot. Disattiva `scan_backup_storage` o `scan_snapshots`, abbassa `max_snapshot_guests`, oppure aumenta `request_timeout_seconds`. |
| `node_updates_pending` manca | All'utente o al token manca `Sys.Modify` su `/nodes`. Concedi il ruolo `DumbMonitUpdates` del passaggio 6, oppure ignora: `node_pve_packages_upgradable` e `node_reboot_required` coprono i pacchetti Proxmox senza di esso. |
| Nessuna percentuale di memoria per una VM | Proxmox VE 9 riporta, nell'inventario del cluster, la memoria che la VM prende **sull'host** — overhead incluso, quindi regolarmente sopra la sua allocazione. Una percentuale calcolata da essa resterebbe sopra il 100% per sempre. Il valore onesto proviene dal driver balloon dentro il guest; una VM con il ballooning disattivato mostra quindi byte e nessuna percentuale. `guest_memory_host_bytes` è il valore lato host. |
| Nessuna capacità per lo storage di backup | Un datastore Proxmox Backup Server montato su PVE non riporta alcuna dimensione: PVE non lo legge. La pagina propria del datastore ce l'ha. |
| Utilizzo disco vuoto ("size only") per una VM | La VM non ha un guest agent QEMU (non abilitato nelle sue opzioni, oppure `qemu-guest-agent` non installato dentro), oppure al token manca `VM.GuestAgent.Audit`. `guest_agent_enabled` e `guest_agent_running` dicono quale. I container non ne hanno mai bisogno. |
| Nessuna serie `node_disk_*` | Al token manca `Sys.Audit` su `/` (il ruolo `DumbMonit` lo concede), oppure il nodo funziona dietro un controller RAID che nasconde SMART: `disks/list` allora riporta `UNKNOWN` e nessun `node_disk_smart_failed` viene pubblicato. |
| `node_packages_changed` si attiva dopo un riavvio del server | Non lo fa: la prima sonda dopo un riavvio impara solo le versioni. Un cambio viene riportato solo tra due sonde consecutive dello stesso processo server. |
| `backup_job_last_ok{job=…}` manca mentre `backup_job_last_ok{node=…}` esiste | I task vzdump non portano l'id del job (Proxmox VE più vecchio di 7.2, oppure job eseguiti a mano). Lo stato di esecuzione per nodo c'è comunque. |
| Nessuna metrica Ceph o di replicazione | Nulla da riportare: Ceph non è installato, oppure nessun job di replicazione è configurato. Non è un errore. |
| Nessun pool thin o gruppo di volumi su una scheda di nodo | L'opzione `lvm` è disattivata, il nodo non ha LVM, oppure l'utente possiede `Sys.Audit` solo su `/nodes` — quell'inventario lo richiede su `/`. |
| La sezione Ceph manca mentre il cluster ha Ceph | `ceph` o `ceph_detail` è disattivato, oppure ogni nodo ha risposto alle chiamate di dettaglio con un errore. `available: false` su `/api/targets/{id}/proxmox/ceph` significa "nessun Ceph qui", non "non ancora raccolto". |
| Nessun sistema operativo o indirizzo IP per una VM | Il guest agent QEMU non è abilitato o non è installato in quella VM, oppure al token manca `VM.GuestAgent.Audit` su Proxmox VE 8 (passaggio 7). I container non richiedono alcun agent. Un guest appena avviato attende fino a un'ora: i fatti vengono aggiornati ogni ora, non a ogni sonda. |
| Un guest mantiene un badge di lock | Un backup, una migrazione o uno snapshot interrotto lo ha lasciato. `qm unlock <vmid>` per una VM, `pct unlock <vmid>` per un container — dopo aver verificato che il job sia davvero terminato. |
| `pve_backup_excludes_disk` si attiva su una macchina con un CD-ROM | Non lo fa: drive, immagini cloudinit e voci che non sono volumi vengono esclusi di progetto e mai riportati per guest. Ciò che si è attivato è un disco reale che porta `backup=0`. |
| Nessuna serie `*_rrd_*` | `metrics_export` è disattivato (lo è per impostazione predefinita), oppure all'utente manca `Sys.Audit` su `/`. |
