# Regole

Le regole vengono inserite al primo avvio, poi ti appartengono:
DumbMonit non riscrive mai una regola che hai modificato, ricrea solo
quelle mancanti. Le regole integrate possono essere modificate e
disattivate, non eliminate.

## Regole integrate

Le severità sono mostrate con la parola dell'interfaccia; il valore
API è tra parentesi. Una regola la cui metrica nessun dispositivo
produce non ha alcuna serie e non si attiva mai, quindi ogni regola
sotto viene fornita abilitata su ogni istanza.

### Ogni dispositivo

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Device unreachable | Nessuna misurazione ricevuta da più di tre minuti. `time() - tlast_over_time(dumbmonit_up[7d])` | > 180 s | 1 min | Warning (`critical`) | 6 h |
| High CPU | Carico CPU sostenuto sopra il 90%, mediato per dispositivo su fonti SNMP, Proxmox e agent; si azzera sotto l'85%. | > 90 % | 10 min | Advisory (`warning`), alza dopo 1 h | 6 h |
| Unusual CPU | Carico CPU notevolmente diverso dal solito a quest'ora e giorno della settimana (baseline stagionale). | score > 3.5 | 15 min | Info (`info`) | 6 h |
| Disk almost full | File system pieno al 90% o più, per punto di mount (SNMP, storage Proxmox e file system root); si azzera sotto l'88%. | ≥ 90 % | 15 min | Advisory (`warning`), alza dopo 24 h | 6 h |
| Filesystem almost full | A questo ritmo, il file system sarà pieno entro quattro giorni (`predict_linear` su 6 h; solo file system in crescita). | ≥ 100 % | 30 min | Advisory (`warning`) | 6 h |

### UPS

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| UPS on battery | L'UPS alimenta il carico dalla batteria (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low | La batteria dell'UPS è scarica o esaurita (`dumbmonit_ups_battery_status`, 3 = scarica, 4 = esaurita). | ≥ 3 | 1 min | Warning (`critical`) | 30 min |

### Porte di rete (SNMP)

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Port accumulating errors | Una porta di rete ha registrato più di cinquanta errori nell'ultima ora: di solito un cavo che si sta guastando, una fibra sporca o un disallineamento del duplex (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; nulla si attiva durante la prima ora di un nuovo dispositivo, così gli errori contati prima che fosse aggiunto non vengono riportati come nuovi). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| Port flapping | Una porta di rete si è spenta e riaccesa più di due volte in trenta minuti (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Una porta disattivata di proposito non ha alcuna serie e non si attiva mai. | > 4 | 5 min | Advisory (`warning`) | 1 h |

### Hardware server (Redfish)

Le serie di stato di salute sono 0 (OK), 1 (Warning) o 2 (Critical). Uno slot vuoto (`Absent`) o disattivato non ha alcuna serie, così un server consegnato con un alimentatore su due non si attiva mai.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Server fan failed | Il controller di gestione riporta una ventola come guasta (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Server temperature above critical | Un sensore di temperatura è sopra la soglia critica che il controller stesso dichiara (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Power supply redundancy lost | Gli alimentatori non sono più ridondanti: un guasto in più e il server si spegne (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 min | Advisory (`warning`), alza dopo 1 h | 6 h |
| Power supply failed | Il controller riporta un alimentatore come guasto (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Drive failure predicted | Un drive riporta un guasto previsto: sostituiscilo mentre funziona ancora (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Server health critical | Il controller riporta lo stato di salute proprio del sistema come Critical (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |

### Monitor di disponibilità e heartbeat

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Service down | Il servizio non ha risposto correttamente per tre minuti (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 min | Warning (`critical`) | 30 min |
| Heartbeat missed | Il job non ha chiamato entro il suo intervallo atteso più il periodo di grazia, oppure ha segnalato da solo un fallimento (`dumbmonit_probe_success{probe="push"} == bool 0`; il ritardo è impostato per dispositivo, vedi [Heartbeat](../devices/push.md)). | > 0 | 2 min | Advisory (`warning`) | 6 h |
| Service flapping | Il servizio ha cambiato stato più di sei volte in trenta minuti (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 min | Advisory (`warning`) | 1 h |
| Slow service | Il servizio richiede più di tre secondi per rispondere. | > 3 s | 10 min | Advisory (`warning`) | 6 h |
| Certificate expiring soon | Il certificato scade in meno di quattordici giorni. | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Certificate expired | Il certificato è scaduto. | < 0 d | 5 min | Warning (`critical`) | 24 h |

### Container Docker

Riportato dall'agent; vedi [Agent](../devices/agent.md). L'avviso nomina
il container.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Container stopped | Il container è fermo da due minuti (`dumbmonit_container_up == bool 0`). | > 0 | 2 min | Advisory (`warning`) | 1 h |
| Container unhealthy | Il controllo di salute del container fallisce da tre minuti (`dumbmonit_container_health == 2`; 0 nessuno, 1 sano, 3 in avvio). | > 0 | 3 min | Advisory (`warning`) | 1 h |
| Container restarting | Il container si è riavviato tre volte o più in quindici minuti (`increase(dumbmonit_container_restart_count[15m])`): raramente resta fermo abbastanza a lungo per la regola precedente. | ≥ 3 | 1 min | Advisory (`warning`) | 1 h |
| Container update available | Un'immagine più recente è disponibile nel registry per questo container (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Backup Plakar

Riportato dall'agent, per kloset e per fonte. Nessuna di queste serie
esiste mentre Plakar non è rilevato, così nessuna delle due regole può
attivarsi su una macchina senza di esso.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Plakar backup too old | Nessun nuovo snapshot Plakar per questa fonte da più di due giorni (`dumbmonit_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Plakar backup failed | L'ultimo snapshot ha errori, oppure il kloset non può essere letto (`dumbmonit_backup_last_status < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |

### Backup restic e Borg

Riportato dall'agent per ogni repository dichiarato sotto
`restic_repos` o `borg_repos` in agent.yaml; vedi
[Agent](../devices/agent.md#restic-and-borg-backups). Senza un
repository dichiarato non c'è alcuna serie, così nessuna delle due
regole può attivarsi.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| restic/Borg backup too old | Nessun nuovo snapshot restic o Borg in questo repository da più di 48 ore (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Advisory (`warning`) | 24 h |
| restic/Borg repository unreachable | L'agent non può leggere il repository: irraggiungibile, password rifiutata o binario mancante; il log dell'agent dice quale (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Tunnel WireGuard

Riportato dall'agent; vedi
[Agent](../devices/agent.md#wireguard-tunnels).

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| WireGuard peer silent | Un peer con un keepalive persistente non ha completato un handshake da più di 15 minuti: il tunnel è inattivo (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Un peer che non si è mai connesso conta il suo silenzio dal momento in cui l'agent lo ha visto per la prima volta. I peer senza keepalive (un telefono, un laptop) restano inattivi per giorni e vengono esclusi; cambia la soglia per cambiare N, oppure togli la parte `and on …` per osservare ogni peer. | > 15 min | 5 min | Advisory (`warning`) | 6 h |

### Proxmox VE

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Backup too old | Nessun backup Proxmox VE riuscito da più di sette giorni (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 d | 1 h | Advisory (`warning`) | 24 h |
| VM or container stopped | Il guest era in esecuzione nelle ultime due ore ed è fermo da cinque minuti (`1 - dumbmonit_proxmox_guest_running`, con AND su `max_over_time(…[2h]) == 1`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| VM or container CPU high | Un guest ha usato più del 90% dei suoi core allocati per quindici minuti (`dumbmonit_proxmox_guest_cpu_percent`). La notifica nomina il guest, `nextcloud (202)`. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| VM or container memory high | Un guest ha usato più del 95% della sua memoria per dieci minuti (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 min | Advisory (`warning`) | 6 h |
| VM or container disk almost full | Il disco root di un guest è pieno per più del 90% (`dumbmonit_proxmox_guest_disk_used_percent`): container sempre, VM solo quando il guest agent QEMU riporta l'utilizzo. | > 90 % | 15 min | Advisory (`warning`) | 24 h |
| Guest locked | Una VM o container è bloccato da sei ore (`dumbmonit_proxmox_guest_locked`, etichetta `lock`): un backup o migrazione interrotto ha lasciato il lock e nessuna operazione può funzionare sul guest, nemmeno il backup successivo. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| Old snapshot | Lo snapshot più vecchio di un guest ha più di trenta giorni (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 d | 1 h | Info (`info`) | 7 d |
| HA resource in error | Una risorsa HA di Proxmox è in errore o fenced (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| HA manager not reporting | Il resource manager locale HA di un nodo ha smesso di scrivere il suo stato (`dumbmonit_proxmox_ha_lrm_stale`): i guest HA di quel nodo non verranno né riavviati né trasferiti. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Cluster lost quorum | Il cluster Proxmox non è quorate (`1 - dumbmonit_proxmox_cluster_quorate`): i guest non possono più essere avviati. | > 0 | 1 min | Warning (`critical`) | 30 min |
| Proxmox node offline | Un nodo del cluster è riportato offline (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 min | Warning (`critical`) | 30 min |
| Proxmox service down | Un demone Proxmox core non è in esecuzione su un nodo (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, ognuno contato solo dove il nodo lo abilita). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Node network interface down | Un'interfaccia impostata per avviarsi al boot non è attiva su un nodo (`dumbmonit_proxmox_node_interface_offline`). Un'interfaccia attivata su richiesta non viene mai segnalata. | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Proxmox storage almost full | Uno storage Proxmox è pieno per più dell'85% (`dumbmonit_proxmox_storage_used_percent`); "Disk almost full" prende il sopravvento al 90%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Thin pool almost full | Un pool thin LVM è pieno per più del 90%: ogni guest memorizzato su di esso diventa read-only quando si riempie (`dumbmonit_proxmox_node_thinpool_used_percent`); si azzera sotto l'88%. | > 90 % | 15 min | Warning (`critical`) | 6 h |
| Thin pool metadata almost full | Il volume dei metadati di un pool thin LVM è pieno per più dell'80% (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). È molto più piccolo del volume dati e spesso satura prima, con la stessa conseguenza. | > 80 % | 15 min | Warning (`critical`) | 6 h |
| Proxmox disk wearing out | Un SSD o NVMe di un nodo ha usato più del 90% della sua vita nominale (`dumbmonit_proxmox_node_disk_wearout_percent`). La notifica legge `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| Proxmox disk SMART failure | Un disco di un nodo riporta lo stato SMART `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| Proxmox ZFS pool degraded | Un pool ZFS di un nodo non è `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). La notifica legge `pve2 · tank`. | > 0 | 5 min | Warning (`critical`) | 6 h |
| Backup job failed | L'ultimo job vzdump su un nodo, oppure un job di backup pianificato, è fallito (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 min | Warning (`critical`) | 24 h |
| Backup job excludes a disk | Un job di backup pianificato salta un disco di uno dei guest che copre (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): il job riesce ogni notte con quel dato mancante. I drive CD-ROM e le immagini cloudinit sono esclusi di progetto e non contati mai. | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Replication failed | Un job di replicazione riporta un errore o un conteggio di fallimenti diverso da zero (`dumbmonit_proxmox_replication_job_error`): la copia standby è obsoleta. | > 0 | 10 min | Warning (`critical`) | 6 h |
| Ceph health error | Ceph riporta `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 min | Warning (`critical`) | 30 min |
| Ceph health warning | Ceph riporta `HEALTH_WARN` per più di quindici minuti (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Ceph OSD down | Un OSD Ceph è inattivo da cinque minuti (`1 - dumbmonit_proxmox_ceph_osd_up`). La notifica nomina l'OSD e il suo host. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Ceph OSD nearly full | Un singolo OSD Ceph è pieno per più dell'85% (`dumbmonit_proxmox_ceph_osd_used_percent`): le scritture si fermano quando raggiunge il suo limite, qualunque sia la media del cluster; si azzera sotto l'82%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph pool almost full | Un pool Ceph è pieno per più dell'85% (`dumbmonit_proxmox_ceph_pool_used_percent`); si azzera sotto l'82%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph noout flag left on | Il flag Ceph `noout` è impostato da due ore (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): il rebalancing è disattivato, e il cluster legge sano mentre la sua ridondanza si riduce. | > 0 | 2 h | Info (`info`) | 24 h |
| Proxmox updates pending | Più di venti aggiornamenti di pacchetto sono in attesa su un nodo (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |
| Proxmox security updates pending | Almeno un aggiornamento in attesa proviene da un archivio di sicurezza (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Proxmox packages changed | La versione installata di un pacchetto Proxmox è cambiata tra due sonde — qualcuno ha eseguito `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, pubblicato per un'ora con il dettaglio nella sua etichetta `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Si risolve da solo un'ora dopo. | > 0 | 1 min | Info (`info`) | 24 h |
| Node certificate expiring | Un certificato di nodo Proxmox scade in meno di quattordici giorni (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |

### Proxmox Backup Server

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| PBS datastore almost full | Datastore PBS pieno per più del 90%. | > 90 % | 15 min | Advisory (`warning`), alza dopo 24 h | 6 h |
| PBS datastore filling up | Al ritmo attuale, PBS stima che il datastore sarà pieno entro sette giorni. | < 7 d | 1 h | Advisory (`warning`) | 24 h |
| PBS datastore not mounted | Un datastore removibile non è montato (`dumbmonit_pbs_datastore_removable_unmounted`): nulla può essere scritto su di esso. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| PBS backup too old | Nessun nuovo snapshot per questo gruppo di backup (uno per macchina) da più di due giorni (`dumbmonit_pbs_backup_last_age_seconds`). Modifica la soglia per macchine sottoposte a backup settimanale; la notifica nomina il gruppo e il datastore. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| PBS backup verification failed | La verifica dell'ultimo snapshot per questa macchina è fallita (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 min | Warning (`critical`) | 24 h |
| PBS task failed | Almeno un task PBS è fallito nella finestra di osservazione (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| PBS garbage collection too old | Nessuna garbage collection riuscita su questo datastore da più di otto giorni (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| PBS garbage collection failed | L'ultima garbage collection su un datastore è fallita (`dumbmonit_pbs_gc_last_run_ok < 1`; da `/gc` su PBS 3.3+, dall'ultimo task GC prima). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS corrupt chunks found | La garbage collection ha trovato chunk illeggibili su un datastore (`dumbmonit_pbs_gc_bad_chunks`): alcuni backup non possono più essere ripristinati. | > 0 | 10 min | Warning (`critical`) | 24 h |
| PBS prune job failed | L'ultima esecuzione di un job di prune PBS è fallita: nulla viene prunato e il datastore continua a riempirsi (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). La notifica nomina il job e il datastore. | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS verification job failed | L'ultima esecuzione di un job di verifica PBS è fallita (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS sync job failed | L'ultima esecuzione di un job di sync PBS è fallita (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS tape backup failed | L'ultima esecuzione di backup tape è fallita (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): la copia offline non viene fatta. | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS job never ran | Un job è abilitato e pianificato ma non è mai stato eseguito (`dumbmonit_pbs_job_never_run`). | > 0 | 2 d | Advisory (`warning`) | 7 d |
| PBS service down | Un servizio di cui Proxmox Backup Server ha bisogno non è in esecuzione (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); nessun backup può essere eseguito mentre è inattivo. | < 1 | 5 min | Warning (`critical`) | 6 h |
| PBS disk SMART failure | Un disco del server di backup riporta lo stato SMART `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). La notifica legge `/dev/sdb`. | > 0 | 5 min | Warning (`critical`) | 24 h |
| PBS SSD worn out | Un SSD del server di backup ha usato più del 90% della sua resistenza nominale (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| PBS ZFS pool degraded | Un pool ZFS del server di backup non è `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| PBS certificate expiring | Il certificato servito dal server di backup scade in meno di tre settimane (`dumbmonit_pbs_node_certificate_expires_seconds`). Richiede l'opzione `certificates`. | < 21 d | 1 h | Advisory (`warning`) | 48 h |
| PBS restart pending after upgrade | Il pacchetto è stato aggiornato ma il demone in esecuzione è ancora quello vecchio (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 d |
| PBS updates pending | Più di venti aggiornamenti di pacchetto sono in attesa sul server di backup (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Datacenter Manager

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Federated instance unreachable | La console datacenter non può raggiungere uno dei cluster Proxmox VE o server di backup che federa (`dumbmonit_pdm_remote_reachable < 1`). La notifica nomina l'istanza e il suo prodotto. | < 1 | 10 min | Warning (`critical`) | 6 h |
| Federated instance behind | Un'istanza esegue una versione più vecchia di un'altra istanza dello stesso prodotto nel medesimo patrimonio (`dumbmonit_pdm_remote_version_behind`). Confrontato tra istanze, mai rispetto a ciò che Proxmox pubblica. | > 0 | 1 h | Info (`info`) | 7 d |
| Task failed on a federated instance | Almeno un task è fallito su un'istanza federata nella finestra di osservazione (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Datacenter console disk almost full | Il file system root della console datacenter è pieno per più del 90% (`dumbmonit_pdm_node_rootfs_percent`): piena, la console smette di registrare ciò che raccoglie. | > 90 % | 15 min | Advisory (`warning`), alza dopo 24 h | 6 h |
| Datacenter console certificate expiring | Un certificato della console datacenter scade in meno di quattordici giorni (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Datacenter console updates pending | Più di venti aggiornamenti di pacchetto sono in attesa sulla console datacenter (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Mail Gateway

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Mail queue growing | La coda di posta deferred è crescita di più di cento messaggi in due ore (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): il prossimo salto probabilmente rifiuta la posta. Crescita invece di dimensione, perché un gateway occupato mantiene sempre qualche decina di messaggi deferred. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| Mail stuck in the queue | Un messaggio attende in una coda da più di quattro ore (`dumbmonit_pmg_queue_oldest_age_seconds`). L'età è un limite inferiore — qshape riporta fasce — quindi la regola si attiva tardi invece che presto. La notifica nomina la coda. | > 4 h | 15 min | Warning (`critical`) | 6 h |
| Mail gateway service stopped | Un servizio di cui il gateway ha bisogno per accettare e filtrare la posta è fermo (`dumbmonit_pmg_service_running` per postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Un'unità non installata non ha alcuna serie e non può attivarsi. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Virus signatures out of date | Le firme virus giornaliere di ClamAV hanno più di due giorni (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): freshclam ha smesso di aggiornarle e il gateway continua a filtrare con la lista della settimana scorsa. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Spam rules out of date | Le regole SpamAssassin hanno più di otto giorni (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Un canale mai aggiornato non ha data né serie. | > 8 d | 1 h | Advisory (`warning`) | 7 d |
| Quarantine filling up | La quarantena spam ha ricevuto più di mille messaggi in sei ore (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): una campagna, oppure una regola che ha appena messo nel cestino un dominio legittimo. | > 1000 | 30 min | Advisory (`warning`) | 12 h |
| Mail gateway cluster degraded | Un gateway del cluster non è più sincronizzato con gli altri (`dumbmonit_pmg_cluster_node_insync`): filtra con un database di regole obsoleto. Un gateway standalone non ha alcuna serie. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Mail gateway certificate expiring | Un certificato del gateway di posta scade in meno di quattordici giorni (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Mail gateway updates pending | Più di venti aggiornamenti di pacchetto sono in attesa sul gateway di posta (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### MDaemon e SecurityGateway

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| MDaemon mail service down | Un servizio di posta osservato del server MDaemon non risponde, oppure saluta per rifiutare il servizio (`dumbmonit_mdaemon_service_up`). Solo i servizi elencati sul dispositivo hanno una serie. La notifica nomina il servizio e la sua porta. | < 1 | 5 min | Warning (`critical`) | 6 h |
| MDaemon XML API not answering | L'API XML ha smesso di rispondere mentre i servizi di posta continuano (`dumbmonit_mdaemon_api_up`): Remote Administration è probabilmente ferma. Solo un dispositivo con un account ha la serie. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| MDaemon mail queue growing | La coda inbound, local o remote è crescita di più di cento messaggi in due ore (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): MDaemon ha smesso di consegnarla. Letto dall'[agent](../devices/agent.md#windows-performance-counters) sul server MDaemon, quindi l'avviso è sul dispositivo dell'agent. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| MDaemon Bad queue not empty | Messaggi attendono nella coda Bad (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): MDaemon non ha potuto né consegnarli né rimandarli, e restano lì finché un amministratore non li rilascia o elimina. | > 0 | 30 min | Advisory (`warning`) | 24 h |
| MDaemon Retry queue high | Più di cinquanta messaggi attendono nella coda Retry (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): i server remoti continuano a rifiutare la posta temporaneamente. | > 50 | 1 h | Advisory (`warning`) | 6 h |
| SecurityGateway service down | Un servizio osservato del gateway non risponde, oppure saluta per rifiutare il servizio (`dumbmonit_securitygateway_service_up`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| SecurityGateway API not answering | L'API REST ha smesso di rispondere mentre i servizi continuano (`dumbmonit_securitygateway_api_up`). Solo un dispositivo con una chiave API ha la serie. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| SecurityGateway delivery queue growing | La coda di consegna è crescita di più di cento messaggi in due ore (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): il server di posta dietro il gateway probabilmente rifiuta la posta. I nomi dei contatori provengono dall'API così come sono; se la tua versione nomina la coda diversamente, adatta il matcher. | > 100 | 15 min | Advisory (`warning`) | 6 h |

### OPNsense

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Internet gateway down | dpinger non riceve più risposta da un gateway (`dumbmonit_opnsense_gateway_up`). Su un firewall multi-WAN il traffico si è già spostato su un altro collegamento, silenziosamente. `loss` e `delay` sono avvisi, non fallimenti, e non attivano questa regola. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Gateway losing packets | Un gateway perde più di un quinto dei pacchetti inviati ad esso (`dumbmonit_opnsense_gateway_loss_percent`): il collegamento risponde, male. Si azzera sotto il 10%. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| Gateway slow | Il tempo di andata-ritorno di un gateway è rimasto sopra i 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Trecento millisecondi lascia in pace un collegamento di backup mobile. | > 300 ms | 15 min | Info (`info`) | 12 h |
| Firewall state table filling up | pf sta tracciando più dell'80% delle connessioni che gli sono consentite (`dumbmonit_opnsense_pf_states_used_percent`). Oltre il limite il firewall scarta le nuove connessioni senza altro sintomo. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| Firewall network buffers exhausted | Più del 90% dei buffer di rete FreeBSD sono in uso (`dumbmonit_opnsense_mbuf_used_percent`). Un firewall che li esaurisce smette di instradare con la CPU ancora inattiva. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| VPN tunnel down | Un tunnel VPN configurato non ha avuto alcuna sessione per dieci minuti (`dumbmonit_opnsense_vpn_tunnel_up`). Un plugin non installato, oppure una connessione disattivata, non ha alcuna serie e non può attivarsi. | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Firewall resolver stopped | Unbound ha smesso di rispondere (`dumbmonit_opnsense_unbound_running`). Il firewall continua a instradare; le macchine dietro di esso non risolvono più. Disattiva questa regola se il tuo firewall risolve con qualcos'altro. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Firewall core service stopped | dpinger, che osserva i gateway, oppure configd, che esegue tutto il resto, è fermo (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Solo questi due, perché un servizio disattivato di proposito non è un fallimento. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Firewall too hot | Un sensore legge sopra gli 85 °C (`dumbmonit_opnsense_temperature_celsius`): la scatola senza ventole nell'armadio sta per iniziare a limitare le prestazioni. Si azzera a 78 °C. | > 85 °C | 10 min | Advisory (`warning`) | 6 h |
| Firewall left in CARP maintenance mode | CARP è stato lasciato in modalità manutenzione persistente (`dumbmonit_opnsense_carp_maintenance_mode`): questo firewall ha consegnato i suoi indirizzi virtuali al suo partner e non li riprenderà da solo. | > 0 | 1 h | Info (`info`) | 24 h |
| Firewall reboot pending | Un aggiornamento è installato ma richiede un riavvio per avere effetto (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 d |
| Firewall updates pending | Il firewall ha pacchetti in attesa di aggiornamento (`dumbmonit_opnsense_firmware_updates_pending`). È la macchina più esposta sulla rete. | > 0 | 1 h | Info (`info`) | 7 d |

### TrueNAS

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| NAS pool degraded | ZFS non chiama più sano un pool (`dumbmonit_truenas_pool_healthy`). Un pool degradato serve ancora i suoi dati, che è esattamente perché nessuno se ne accorge finché il disco successivo non fallisce. La pagina del dispositivo nomina il disco. | < 1 | 5 min | Warning (`critical`) | 6 h |
| NAS disk errors | I dischi di un pool hanno riportato errori di lettura, scrittura o checksum da quando l'ultimo `zpool clear` (`dumbmonit_truenas_pool_device_errors`, una serie per tipo). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| NAS pool almost full | Un pool è pieno per più dell'85% (`dumbmonit_truenas_pool_used_percent`). ZFS rallenta molto prima di essere pieno. Si azzera all'80%. | > 85 % | 30 min | Advisory (`warning`), alza dopo 7 d | 24 h |
| NAS scrub found errors | L'ultimo scrub di un pool ha trovato dati danneggiati (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS scrub overdue | Un pool non ha completato uno scrub da più di 45 giorni (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Assente dopo un resilver, che cancella il record dell'ultimo scrub. | > 45 d | 1 h | Info (`info`) | 7 d |
| NAS disk failed its SMART test | Un autotest SMART nel log di un disco è fallito (`dumbmonit_truenas_disk_smart_failed`). Un disco mai testato non ha alcuna serie. | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS disk too hot | Un disco è sopra i 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, letto dalla cache di TrueNAS). Si azzera a 50 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| NAS dataset near its quota | Un dataset ha usato più del 90% della sua quota (`dumbmonit_truenas_dataset_quota_used_percent`): al 100% le sue scritture falliscono mentre il pool ha ancora spazio. Nessuna quota, nessuna serie. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| NAS replication failed | Un task di replicazione abilitato è in errore (`dumbmonit_truenas_replication_error`): la copia sull'altro lato invecchia. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshot task failed | Un task di snapshot periodico abilitato è in errore (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshots stale | Un task di snapshot periodico abilitato non è stato eseguito da più di otto giorni (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): un task che smette di funzionare non fallisce, diventa silenzioso. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| TrueNAS alert raised | TrueNAS stesso ha generato un avviso di livello ERROR o superiore (`dumbmonit_truenas_alerts`). La sua propria frase è sulla pagina del dispositivo. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS service stopped | Un servizio impostato per avviarsi con il NAS è fermo (`dumbmonit_truenas_service_running`). Un servizio disattivato di proposito non ha alcuna serie. | < 1 | 10 min | Advisory (`warning`) | 6 h |

### Synology DSM

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Synology disk SMART warning | DSM non riporta più lo stato SMART di questo disco come `normal` (`dumbmonit_synology_disk_smart_status`: 1 attenzione, 2 critico, parole sconosciute contano come 1). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology disk failed | DSM riporta il disco come bloccato o guasto (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology disk bad sectors | Il conteggio di settori danneggiati del disco supera la soglia impostata in DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 min | Warning (`critical`) | 24 h |
| Synology disk bad sectors growing | Sono apparsi nuovi settori illeggibili nelle ultime 24 ore (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology SSD wearing out | Un SSD ha meno del 10% della sua vita nominale rimasta (`dumbmonit_synology_disk_remaining_life_percent`); si azzera sopra il 12%. | < 10 % | 1 h | Advisory (`warning`) | 7 d |
| Synology volume degraded | Un volume è degradato o bloccato: la sua ridondanza è sparita (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology volume almost full | Un volume è pieno al 90% o più (`dumbmonit_synology_volume_used_percent`); si azzera sotto l'88%. | ≥ 90 % | 15 min | Advisory (`warning`), alza dopo 24 h | 6 h |
| Synology temperature high | Un disco è sopra i 55 °C, oppure DSM ha generato il suo proprio avviso di temperatura (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); si azzera sotto i 52 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| Synology memory high | Utilizzo di memoria del NAS sopra il 95% (`dumbmonit_synology_memory_usage_percent`); si azzera sotto il 90%. | > 95 % | 15 min | Advisory (`warning`) | 6 h |
| Active Backup task failed | L'ultima esecuzione di un task Active Backup for Business è fallita, oppure ha sottoposto a backup solo parte dei suoi dispositivi (`dumbmonit_abb_task_last_status == bool 0`). La notifica nomina il task. | > 0 | 10 min | Warning (`critical`) | 24 h |
| Active Backup too old | Nessuna esecuzione riuscita di Active Backup for Business per questo task da più di due giorni (`dumbmonit_abb_task_last_success_seconds`). La serie non esiste per un task mai riuscito; "Active Backup task failed" parla allora. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Active Backup task disabled | Il task non ha pianificazione, oppure il suo backup continuo è in pausa (`dumbmonit_abb_task_enabled == bool 0`): non sottoporrà nulla a backup finché qualcuno non lo esegue. | > 0 | 1 h | Info (`info`) | 7 d |
| Active Backup device overdue | Un dispositivo è andato più a lungo senza un'esecuzione Active Backup for Business riuscita di quanto il suo proprio ritmo consenta, giorni liberi esclusi (`dumbmonit_abb_device_overdue`, vedi [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Active Backup device failing | Gli ultimi due o più tentativi di un dispositivo sono falliti (`dumbmonit_abb_device_consecutive_failures`); un'esecuzione annullata non conta come nessuno dei due. | ≥ 2 | 10 min | Warning (`critical`) | 24 h |

### Server di log e metriche

VictoriaMetrics, VictoriaLogs, Loki e Graylog ([dispositivi](../devices/victoriametrics.md)). I contatori passano attraverso `increase_prometheus`, così un server aggiunto con una cronologia di dati rifiutati non si attiva alla sua prima misurazione.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Metrics or log storage read-only | Lo spazio libero su disco è sceso sotto `-storage.minFreeDiskSpaceBytes` e lo storage rifiuta ogni scrittura (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 min | Warning (`critical`) | 1 h |
| Metrics or log storage almost read-only | Meno del 10% del disco rimasto prima della sola lettura (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); si azzera sopra il 12%. | < 10 % | 30 min | Advisory (`warning`) | 24 h |
| Metrics or log server unhealthy | `/health` non risponde OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 min | Warning (`critical`) | 6 h |
| Metrics or log data refused | Campioni o righe di log rifiutati nell'ultima ora: timestamp fuori retention, etichette troppo lunghe, limite di serie (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| VictoriaMetrics slow inserts | Più del 5% dei nuovi campioni ha preso il percorso lento su 15 minuti: non abbastanza memoria per le serie attive (`100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])`); si azzera sotto il 3%. | > 5 % | 30 min | Advisory (`warning`) | 24 h |
| Loki not ready | `/ready` non risponde ready (`dumbmonit_loki_ready`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Loki refusing log lines | Righe rifiutate nell'ultima ora: troppo vecchie, limite di tasso o stream, riga troppo lunga (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Loki cannot flush to storage | Chunk non riusciti a raggiungere lo storage negli ultimi 30 minuti; vengono persi se Loki si riavvia (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Loki write-ahead log disk full | Scritture WAL perse per un disco pieno (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Loki request errors | Più di cinque richieste risposte con un errore server in 15 minuti (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 min | Advisory (`warning`) | 6 h |
| Graylog not processing | Elaborazione dei messaggi in pausa sul nodo (`dumbmonit_graylog_processing`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Graylog search cluster down | OpenSearch o Elasticsearch red, oppure irraggiungibile da Graylog (`dumbmonit_graylog_indexer_status`: 2 red, 3 irraggiungibile). | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Graylog search cluster yellow | Shard di replica non assegnati per mezz'ora (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Graylog journal filling | Il journal è pieno per più della metà: l'output non sta al passo (`dumbmonit_graylog_journal_used_percent`); si azzera sotto il 40%. | > 50 % | 10 min | Advisory (`warning`), alza dopo 1 h | 6 h |
| Graylog input failed | Un input non è riuscito ad avviarsi (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Graylog indexing failures | Scritture verso il cluster di ricerca o elaborazione dei messaggi fallite nell'ultima ora (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### Applicazioni self-hosted

Nextcloud, Immich, Paperless-ngx, Jellyfin e Plex ([dispositivi](../devices/nextcloud.md)). Raramente si fermano del tutto: la pagina si apre mentre la sincronizzazione, le miniature, le importazioni o le scansioni della libreria si sono fermate. Queste regole osservano quegli stati; un'applicazione che non risponde affatto è Device unreachable.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Nextcloud stuck in maintenance | La modalità manutenzione è attiva (`dumbmonit_nextcloud_maintenance`): nessuno può accedere o sincronizzare. | > 0 | 30 min | Advisory (`warning`), alza dopo 3 h | 6 h |
| Nextcloud database upgrade pending | Il codice è stato aggiornato e Nextcloud attende `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Nextcloud update available | Il controllo aggiornamenti di Nextcloud ha trovato una release più recente (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Nextcloud OPcache full | L'OPcache di PHP è senza spazio e le pagine diventano lente (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Nextcloud free space low | Meno di 5 GiB liberi per la directory dati (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); si azzera sopra 6 GiB. | < 5 GiB | 30 min | Advisory (`warning`) | 24 h |
| Immich jobs stalled | Job attendono in una coda non in pausa e che non esegue nulla (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). La notifica nomina la coda. | > 0 | 1 h | Advisory (`warning`) | 6 h |
| Immich queue paused | Una coda di job è in pausa da un giorno (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 d |
| Immich storage almost full | Il disco che contiene la libreria è pieno per più del 90% (`dumbmonit_immich_storage_used_percent`); si azzera sotto l'88%. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Immich update available | Il controllo versione di Immich conosce una release più recente (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Paperless task queue down | Redis o Celery in errore (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 è un errore): nulla di nuovo viene importato. | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Paperless index or sanity check error | L'indice di ricerca, il classificatore o il controllo di integrità (file mancanti o alterati) riporta un errore (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 min | Advisory (`warning`) | 24 h |
| Paperless migrations pending | Migrazioni di database rimaste da applicare (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Paperless task failed | Un task visibile a questo utente è fallito nella finestra di task falliti e non è stato respinto (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Paperless storage almost full | Il disco che contiene i documenti è pieno per più del 90% (`dumbmonit_paperless_storage_used_percent`); si azzera sotto l'88%. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Paperless update available | Il controllo versione di Paperless-ngx conosce una release più recente (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Jellyfin scheduled task failed | Un task pianificato (una scansione libreria, una pulizia) è fallito o è stato interrotto alla sua ultima esecuzione (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Jellyfin plugin broken | Un plugin non è riuscito a caricarsi o non supporta questa versione (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Jellyfin restart pending | Jellyfin attende un riavvio da un giorno dopo l'installazione o l'aggiornamento di un plugin (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 d |
| Plex update available | Il controllo aggiornamenti di Plex ha trovato una release più recente (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Forge self-hosted

GitLab (self-managed) e Forgejo/Gitea ([dispositivi](../devices/gitlab.md)). Un'istanza che non risponde affatto è Device unreachable; queste regole osservano le parti che continuano a servire pagine mentre silenziosamente rimangono indietro — Sidekiq, runner CI/CD, un task pianificato che ha smesso di funzionare.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| GitLab runners all offline | Ogni runner CI/CD che GitLab conosce è offline, e almeno uno è registrato (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| GitLab Sidekiq backlog high | Una coda Sidekiq ha più di 500 job in attesa (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): notifiche, webhook e importazioni rimangono indietro. | > 500 | 15 min | Advisory (`warning`) | 6 h |
| GitLab migrations pending | Migrazioni di database da un aggiornamento restano da applicare (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Info (`info`) | 24 h |
| GitLab license expiring | La licenza Enterprise Edition scade in meno di 30 giorni (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 24 h |
| Forgejo/Gitea runners all offline | Ogni runner Actions è offline, e almeno uno è registrato (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Forgejo/Gitea scheduled task overdue | Un task pianificato (controllo repository, pulizia, controllo aggiornamenti) non è stato eseguito al suo proprio intervallo (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 min | Advisory (`warning`) | 24 h |

### Dispositivi client

Una regola, condivisa da ogni integrazione che traccia dispositivi che si connettono e fanno backup — i telefoni, tablet e desktop di Immich ([dispositivi](../devices/immich.md)), le macchine sottoposte a backup di Proxmox Backup Server, una per gruppo di backup ([dispositivi](../devices/pbs.md)), e i computer e VM protetti di Veeam, uno per oggetto con un punto di ripristino ([dispositivi](../devices/veeam.md)), oggi; altri seguiranno. Ogni integrazione calcola il gap rispetto alla finestra di obsolescenza del proprio target (`device_stale_days`, 3 giorni per impostazione predefinita) e lo pubblica come `dumbmonit_client_device_stale_seconds`, etichettato `device`, `type`, `os`, `user`, `kind` e `signal` (`connection` o `backup`); questa unica regola si attiva allora per qualsiasi di essi, nominando il dispositivo nella notifica. Synology Active Backup for Business traccia i suoi propri dispositivi separatamente, tramite un meccanismo consapevole del ritmo (`synology_abb_device_*`) invece di questa famiglia generica — vedi [Synology](../devices/synology.md).

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Client device stale | Un dispositivo non si è connesso o non ha fatto backup entro la finestra di obsolescenza configurata per questo target (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Advisory (`warning`) | 24 h |

### Pi-hole

[Pi-hole](../devices/pihole.md) v6, letto tramite la sua API REST.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Pi-hole blocking disabled | Il blocco è disattivato, in pausa o fallito (`dumbmonit_pihole_blocking_enabled`). L'attesa lascia passare una pausa di cinque minuti dall'interfaccia web. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Pi-hole blocklists stale | Le blocklist (gravity) non sono state ricostruite da più di otto giorni (`dumbmonit_pihole_gravity_age_seconds`): l'aggiornamento settimanale fallisce. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| Pi-hole update available | Una versione più recente è disponibile (`dumbmonit_pihole_updates_available`): l'immagine in Docker, core, interfaccia web o FTL altrimenti. Pi-hole controlla una volta al giorno. | > 0 | 1 h | Info (`info`) | 7 d |
| Pi-hole diagnosis messages | Pi-hole elenca messaggi sulla sua pagina di diagnosi (`dumbmonit_pihole_messages`): una blocklist non scaricata, un client limitato per tasso, un host sovraccarico. | > 0 | 15 min | Info (`info`) | 24 h |

### AdGuard Home

Il filtro DNS ([dispositivo](../devices/adguard.md)). I valori di query e blocco sono somme sulla finestra di statistiche propria di AdGuard Home, non contatori: nessuna regola è costruita su di essi.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| AdGuard Home protection off | Protezione disattivata, oppure in pausa e non ancora tornata (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 min | Advisory (`warning`) | 6 h |
| AdGuard Home DNS not running | L'interfaccia web risponde ma il server DNS è fermo (`dumbmonit_adguard_running`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| AdGuard Home upstream failing | Un server DNS upstream fallisce il test proprio di AdGuard Home, oppure resta silenzioso 4 secondi (`dumbmonit_adguard_upstream_up`, una serie per upstream). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| AdGuard Home filter lists stale | L'aggiornamento più vecchio di una lista di filtri abilitata ha più di tre giorni (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 giorni | 1 h | Advisory (`warning`) | 24 h |
| AdGuard Home update available | Una versione più recente è annunciata dal controllo versione proprio di AdGuard Home (`dumbmonit_adguard_update_available`); non si attiva mai quando quel controllo è disattivato, come nell'immagine Docker ufficiale. | > 0 | 1 h | Info (`info`) | 7 giorni |

### UPS dietro NUT

UPS letti da un [server NUT](../devices/nut.md). Ogni serie porta l'etichetta `ups`, così un avviso nomina l'UPS quando un server ne pubblica diversi. I flag di stato sono 0 o 1 a ogni lettura, così una regola si azzera da sola una volta tornata la corrente. Gli UPS letti tramite SNMP sono coperti dalle regole [UPS](#ups) sopra.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| UPS on battery (NUT) | L'UPS alimenta il carico dalla batteria (`dumbmonit_nut_ups_on_battery`, il flag `OB` di `ups.status`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low (NUT) | L'UPS riporta una batteria scarica, il flag `LB` (`dumbmonit_nut_ups_low_battery`): le macchine che alimenta stanno per spegnersi. | > 0 | 1 min | Warning (`critical`) | 30 min |
| UPS battery needs replacing (NUT) | L'UPS chiede che la sua batteria sia sostituita, il flag `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| UPS load high (NUT) | Il carico è rimasto sopra l'80% (`dumbmonit_nut_ups_load_percent`); si azzera sotto il 75%. | > 80 % | 10 min | Advisory (`warning`) | 24 h |
| UPS runtime short (NUT) | L'UPS stima meno di cinque minuti di autonomia a batteria (`dumbmonit_nut_battery_runtime_seconds`); si azzera sopra sei. A corrente, una batteria esaurita o un UPS troppo piccolo per il suo carico. | < 300 s | 5 min | Advisory (`warning`) | 24 h |
| UPS data stale (NUT) | Il server NUT risponde ma non ha dati recenti dall'UPS: cavo, driver o UPS silenzioso (`dumbmonit_nut_ups_data_stale`). Un'interruzione passerebbe inosservata nel frattempo. | > 0 | 5 min | Advisory (`warning`), alza dopo 1 h | 6 h |

### MikroTik RouterOS

Le serie portano l'etichetta `interface` o `sensor`, così una notifica nomina la porta o il sensore ([dispositivo](../devices/mikrotik.md)). I contatori passano attraverso `increase_prometheus`: un router aggiunto con anni di errori dietro non si attiva alla sua prima misurazione.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| MikroTik CPU high | La CPU del router è rimasta sopra il 90% (`dumbmonit_mikrotik_cpu_load_percent`): instradamento, firewall o uno script la sta saturando. Si azzera sotto l'80%. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik memory high | Più del 90% della memoria è in uso (`dumbmonit_mikrotik_memory_used_percent`). Si azzera sotto l'85%. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik storage almost full | Lo storage flash è pieno per più del 90%: le modifiche di configurazione e gli aggiornamenti potrebbero fallire. Si azzera sotto l'85%. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| MikroTik temperature high | Un sensore di temperatura legge sopra gli 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Si azzera a 75 °C. | > 80 °C | 10 min | Advisory (`warning`) | 6 h |
| MikroTik power supply or fan failed | Un sensore `psu*-state` o `fan*-state` non è `ok` (`dumbmonit_mikrotik_health_ok`). Un alimentatore ridondante morto non tronca nulla, finché il secondo non lo fa. | < 1 | 5 min | Warning (`critical`) | 1 h |
| MikroTik interface accumulating errors | Più di cinquanta errori di ricezione e trasmissione su un'interfaccia nell'ultima ora (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| MikroTik link flapping | Un'interfaccia ha perso il suo link più di due volte in trenta minuti (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Una porta scollegata per sempre conta una volta e non si attiva. | > 2 | 5 min | Advisory (`warning`) | 1 h |
| MikroTik RouterOS update available | Un RouterOS più recente è disponibile sul canale di aggiornamento del router (`dumbmonit_mikrotik_update_available`). Conosciuto solo quando il router stesso controlla gli aggiornamenti: vedi il passaggio opzionale della [configurazione](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 d |
| MikroTik firmware upgrade pending | RouterBOOT è più vecchio del firmware incluso nel RouterOS installato (`dumbmonit_mikrotik_firmware_upgrade_pending`); viene aggiornato solo su richiesta, poi al riavvio successivo. | > 0 | 1 h | Info (`info`) | 7 d |

### UniFi Network

[Pagina del dispositivo](../devices/unifi.md). Le serie dei dispositivi portano il nome, il modello e il tipo del dispositivo, così la notifica dice "Living room AP". Un dispositivo in attesa di adozione non ha alcuna serie `device_up` e non conta mai come offline.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| UniFi device offline | Un dispositivo UniFi adottato dal controller è offline, isolato o non riuscito ad adottare (`dumbmonit_unifi_device_up`). | < 1 | 5 min | Advisory (`warning`), alza dopo 1 h | 6 h |
| UniFi Internet down | Il gateway UniFi riporta nessuna connessione internet (`dumbmonit_unifi_internet_up`). Senza un gateway UniFi la serie non esiste e la regola resta silenziosa. | < 1 | 3 min | Warning (`critical`) | 1 h |
| UniFi WAN link down | Un collegamento WAN del gateway UniFi è inattivo (`dumbmonit_unifi_wan_link_up`): con due collegamenti, la ridondanza è persa mentre nulla appare rotto. Una porta disattivata non ha alcuna serie. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| UniFi gateway CPU high | Il processore del gateway UniFi è sopra il 90% da quindici minuti (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); si azzera sotto l'80%. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| UniFi device waiting for adoption | Un dispositivo è apparso sulla rete e attende di essere adottato in UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 min | Info (`info`) | 24 h |
| UniFi firmware update available | Un aggiornamento firmware è disponibile per i dispositivi UniFi (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 d |
| UniFi alarms | UniFi ha registrato eventi critici o generato allarmi nelle ultime 24 ore (`dumbmonit_unifi_alarms`). Letto solo con un account View Only. | > 0 | 5 min | Advisory (`warning`) | 24 h |

### Home Assistant

[Pagina del dispositivo](../devices/homeassistant.md).

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Home Assistant in recovery mode | Home Assistant si è avviato in modalità recupero: la sua configurazione non ha potuto essere caricata e le automazioni non funzionano (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| Home Assistant not running | Home Assistant risponde ma non ha finito di avviarsi da dieci minuti (`dumbmonit_homeassistant_running`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant entities went unavailable | Almeno cinque entità in più non sono disponibili rispetto a un'ora fa: un'integrazione, un hub o una radio si sono fermati (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). Conta l'aumento, non il totale, che un'installazione reale non ha mai a zero. | ≥ 5 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant battery low | Una batteria riportata da Home Assistant è scarica (`dumbmonit_homeassistant_battery_low`, una serie per batteria sotto la soglia del dispositivo, 20% per impostazione predefinita; il suo valore è il livello). | ≥ 0 % | 30 min | Info (`info`) | 3 d |
| Home Assistant update available | Home Assistant ha aggiornamenti in attesa: core, sistema operativo, add-on o firmware dei dispositivi (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Home Assistant repair to address | Home Assistant ha una riparazione aperta di severità error o critical (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### VMware vSphere

[Pagina del dispositivo](../devices/vsphere.md). Le serie di host portano `host`, le serie di VM `vm`, le serie di datastore `datastore`, così la notifica nomina l'oggetto.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| vSphere host not responding | Un host vSphere non risponde a vCenter (`dumbmonit_vsphere_host_connection_state`: 2 non risponde). Un host disconnesso di proposito (1) non si attiva. | ≥ 2 | 3 min | Warning (`critical`) | 1 h |
| vSphere host health red | vSphere riporta lo stato di salute generale di un host come rosso (`dumbmonit_vsphere_host_status == 2`). Grigio significa sconosciuto e non si attiva. | > 0 | 5 min | Warning (`critical`) | 1 h |
| vSphere host left in maintenance | Un host vSphere è in modalità manutenzione da un giorno: le sue VM sono state spostate via e potrebbe essere stato scordato lì (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| vSphere datastore almost full | Un datastore vSphere è pieno al 90% o più (`dumbmonit_vsphere_datastore_used_percent`); si azzera sotto l'85%. | ≥ 90 % | 15 min | Advisory (`warning`), alza dopo 24 h | 6 h |
| vSphere datastore inaccessible | Un datastore vSphere è inaccessibile: le VM memorizzate su di esso non possono funzionare (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| vSphere VM tools not running | VMware Tools è installato ma non in esecuzione in una VM accesa: il guest potrebbe essere bloccato (`dumbmonit_vsphere_vm_tools_status == 2`). Le VM senza Tools non si attivano. | > 0 | 30 min | Info (`info`) | 24 h |
| vSphere red alarm | vSphere ha generato un allarme rosso che nessuno ha confermato (`dumbmonit_vsphere_alarm{status="red"}`, una serie per allarme, nominato dopo l'allarme e l'oggetto). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Database, message broker e motore di sicurezza

Redis / Valkey, MongoDB, RabbitMQ e CrowdSec ([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md), [RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)). I contatori passano attraverso `increase_prometheus` o `changes_prometheus`, così nulla si attiva alla prima misurazione di un dispositivo.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Redis memory near maxmemory | Memoria usata sopra il 90% di `maxmemory` (`dumbmonit_redis_memory_used_percent`); si azzera sotto l'85%. Nessuna serie senza `maxmemory`. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| Redis refusing connections | Connessioni client rifiutate negli ultimi 15 minuti: `maxclients` raggiunto (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Redis replication link down | Una replica ha perso il suo collegamento con il primario (`dumbmonit_redis_master_link_up`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| Redis replica lagging | Vista dal primario, una replica non ha confermato per più di 30 secondi (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 min | Advisory (`warning`) | 6 h |
| Redis snapshot failed | L'ultimo `BGSAVE` è fallito; con `stop-writes-on-bgsave-error`, le scritture vengono rifiutate (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| Redis append-only file write failed | L'ultima scrittura AOF è fallita (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replica set without primary | Nessun membro è primario: nessuna scrittura viene accettata (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| MongoDB member unreachable | Un membro del replica set è irraggiungibile dagli altri (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replication lag | Un secondario è più di un minuto indietro rispetto al primario (`dumbmonit_mongodb_replset_member_lag_seconds`); si azzera sotto 30 s. | > 60 s | 10 min | Advisory (`warning`) | 6 h |
| MongoDB connections near the limit | Più dell'80% delle connessioni disponibili in uso (`dumbmonit_mongodb_connections_used_percent`); si azzera sotto il 75%. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB cache under pressure | Più del 20% della cache WiredTiger è sporca (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); si azzera sotto il 15%. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB internal errors | Assert regolari nell'ultima ora (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| RabbitMQ memory or disk alarm | Un nodo ha generato un allarme di risorsa: ogni publisher è bloccato (`dumbmonit_rabbitmq_alarm`, una serie per nodo e risorsa). | > 0 | 1 min | Warning (`critical`) | 1 h |
| RabbitMQ node down | Un nodo del cluster non è in esecuzione (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ network partition | Un nodo vede una partizione di rete (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ queue without consumer | Una coda contiene messaggi e non ha alcun consumer (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); il valore è il numero di messaggi in attesa. | > 0 | 15 min | Advisory (`warning`) | 6 h |
| RabbitMQ queue unavailable | Una coda non è in esecuzione: quorum queue senza maggioranza, oppure il suo nodo è inattivo (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec Local API down | La Local API non risponde a `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| CrowdSec bouncer stopped pulling | Le richieste di un bouncer non sono cambiate in 30 minuti (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec reads no logs | Nemmeno una riga di log letta in sei ore (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Reverse proxy e domini

Traefik, Caddy, Nginx Proxy Manager e registrazioni di dominio ([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md), [Nginx Proxy Manager](../devices/npm.md), [Scadenza del dominio](../devices/domain.md)), e l'opzione *Alert when the answer changes* del controllo DNS ([Servizi](../devices/services.md#alerting-when-the-answer-changes)). Le serie portano `router`, `service`, `server`, `upstream`, `host` o `cert`, così la notifica nomina ciò che è fallito.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Traefik router disabled | Un router è disattivato da un errore di configurazione (servizio o middleware mancante, regola non valida): i suoi siti non sono serviti (`dumbmonit_traefik_router_status`: 0 abilitato, 1 avviso, 2 disattivato). | ≥ 2 | 5 min | Advisory (`warning`) | 6 h |
| Traefik backend server down | Un server di un servizio con load balancing fallisce il suo controllo di salute (`dumbmonit_traefik_server_up`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Traefik service without a server | Nessun server di un servizio risponde al suo controllo di salute: i suoi siti ottengono 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| Traefik certificate not obtained | I router chiedono a un resolver di certificati un certificato che nessun certificato in possesso di Traefik copre ancora: Traefik serve il suo certificato autofirmato predefinito (`dumbmonit_traefik_resolver_routers_uncovered`, richiede `/api/certificates` di Traefik). | > 0 | 1 h | Advisory (`warning`) | 24 h |
| Caddy upstream unhealthy | Un backend fallisce il controllo di salute attivo o passivo di Caddy (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Caddy configuration reload failed | Caddy ha rifiutato la sua ultima ricarica di configurazione ed esegue ancora quella precedente (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 min | Advisory (`warning`) | 24 h |
| Nginx Proxy Manager host offline | nginx ha rifiutato la configurazione di un host abilitato: l'host non è servito (`dumbmonit_npm_host_online`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Proxy certificate not renewed | Un certificato in possesso di Traefik o presentato da un host Nginx Proxy Manager scade in meno di 14 giorni: il rinnovo automatico inizia a 30, quindi sta fallendo (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Reverse proxy answering 5xx | Più del 5% delle risposte su dieci minuti erano 5xx, con almeno cinquanta richieste (`increase_prometheus` di `dumbmonit_{traefik,caddy}_requests_5xx_total` su `…_requests_total`). Richiede le metriche del proxy. | > 5 % | 10 min | Advisory (`warning`) | 6 h |
| Domain expiring | Il dominio scade in meno di 30 giorni (`dumbmonit_domain_expiry_days >= 7`). | < 30 d | 1 h | Advisory (`warning`) | 24 h |
| Domain about to expire | Il dominio scade in meno di 7 giorni, oppure è scaduto (`dumbmonit_domain_expiry_days`). | < 7 d | 1 h | Warning (`critical`) | 12 h |
| Domain on hold | Il registry ha messo il dominio in `clientHold` o `serverHold`: è fuori dal DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 min | Warning (`critical`) | 6 h |
| Domain in redemption | Il dominio è in `redemptionPeriod`, `pendingDelete` o `pendingRestore`: è scaduto (`dumbmonit_domain_redemption`). | > 0 | 30 min | Warning (`critical`) | 12 h |
| DNS answer changed | La risposta di un controllo DNS con *Alert when the answer changes* differisce da quella precedente (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Si risolve da sola un'ora dopo. | > 0 | 1 min | Advisory (`warning`) | 24 h |

### Modifiche a un sito web

Vedi [Modifiche a un sito web](../devices/webchange.md). La pagina del dispositivo mostra ogni modifica prima e dopo.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Website changed | L'ultimo controllo di una pagina o sito osservato ha trovato pagine cambiate, apparse o scomparse (`dumbmonit_webchange_last_check_changes`). Si risolve al controllo successivo che non ne trova nessuna. | > 0 | 30 s | Info (`info`) | nessuno |

### Kubernetes

Vedi [Kubernetes / k3s](../devices/kubernetes.md). Ogni serie è etichettata con il nodo, oppure il namespace e il pod, workload o claim interessato.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Kubernetes node not ready | Un nodo non è Ready: il suo kubelet non riporta più, e i suoi pod verranno evacuati (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Kubernetes node under pressure | Un nodo riporta pressione di memoria, disco o PID (`dumbmonit_k8s_node_pressure`, etichetta `condition`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes pod crash looping | Un container è in `CrashLoopBackOff` (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m])`: lo stato scompare per un momento a ogni tentativo di riavvio). | > 0 | 10 min | Warning (`critical`) | 6 h |
| Kubernetes pod stuck pending | Un pod è Pending da quindici minuti: nessun nodo può accoglierlo, oppure un volume o un'immagine di cui ha bisogno non arriva (`dumbmonit_k8s_pod_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Kubernetes pod restarting | Un pod si è riavviato più di cinque volte nell'ultima ora senza essere in `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes workload missing replicas | Un Deployment, StatefulSet o DaemonSet ha meno repliche disponibili di quante richiede (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 min | Advisory (`warning`), alza dopo 1 h | 6 h |
| Kubernetes volume claim pending | Un PersistentVolumeClaim di cui un pod ha bisogno, oppure che un errore di provisioning riguarda, non è ancora bound (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |

### Active Directory

Vedi [Active Directory](../devices/activedirectory.md). Il controller di dominio che DumbMonit interroga è coperto da *Device unreachable*; gli altri controller di dominio del dominio da *Domain controller unreachable* (etichetta `dc`).

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Domain controller unreachable | Un controller di dominio del dominio non risponde più sulla sua porta LDAP (`dumbmonit_ad_dc_reachable == bool 0`). Un nome che non risolve da DumbMonit non produce alcuna serie e non si attiva mai. | > 0 | 5 min | Warning (`critical`) | 6 h |
| Active Directory bind failed | Il controller di dominio risponde ma rifiuta l'account di servizio: password scaduta, account disattivato o bloccato (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Privileged group membership changed | Un membro si è unito o ha lasciato Domain Admins, Enterprise Admins, Schema Admins, Administrators o un gruppo di operatori, nidificazione inclusa (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, etichetta `group`). Si risolve da sola un'ora dopo. | > 0 | 1 min | Advisory (`warning`) | 24 h |
| Active Directory replication failing | Il controller di dominio interrogato non può replicare da un partner (`dumbmonit_ad_replication_consecutive_failures`). Solo quando l'account può leggere lo stato di replicazione. | > 0 | 30 min | Advisory (`warning`) | 6 h |
| krbtgt password older than 180 days | La password krbtgt è stata cambiata l'ultima volta più di 180 giorni fa (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 giorni | 10 min | Info (`info`) | nessuno |

### pfSense

Vedi [pfSense](../devices/pfsense.md). I gateway sono etichettati `gateway`, le interfacce `interface` e `descr`, i servizi `service`.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| pfSense gateway down | Un gateway pfSense è inattivo: il suo indirizzo di monitoraggio non risponde più, e il traffico che porta si ferma o fa failover (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 min | Warning (`critical`) | 6 h |
| pfSense gateway losing packets | Un gateway pfSense perde più del 10% delle sonde verso il suo indirizzo di monitoraggio: la linea è degradata (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 min | Advisory (`warning`) | 6 h |
| pfSense gateway slow | L'andata-ritorno verso l'indirizzo di monitoraggio di un gateway pfSense è rimasto sopra i 500 ms per quindici minuti (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 min | Advisory (`warning`) | 6 h |
| pfSense interface without link | Un'interfaccia pfSense abilitata non ha link: cavo, porta switch o scheda di rete (`dumbmonit_pfsense_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| pfSense service stopped | Un servizio pfSense abilitato non è in esecuzione: resolver DNS, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| pfSense disk nearly full | Il disco pfSense è pieno per più del 90%: log, report e aggiornamenti di pacchetto smettono di essere scritti (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |

### Unraid

Vedi [Unraid](../devices/unraid.md). I dischi sono etichettati `disk` e `role` (`parity`, `data`, `cache`), i container `container`.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Unraid array stopped | L'array Unraid non è avviato: condivisioni, container e VM che vivono su di esso sono indisponibili (`dumbmonit_unraid_array_started`). | < 1 | 10 min | Warning (`critical`) | 6 h |
| Unraid disk disabled or missing | Un disco Unraid è disattivato, mancante o non valido: l'array funziona in modo degradato, e un guasto in più perde dati (`dumbmonit_unraid_disk_ok`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Unraid disk read errors | Un disco Unraid ha riportato errori di lettura da quando l'array è stato avviato: potrebbe essere disattivato al prossimo (`dumbmonit_unraid_disk_errors`). | > 0 | 5 min | Advisory (`warning`) | 1 d |
| Unraid disk too hot | Un disco array o parità Unraid è rimasto sopra i 55 °C per quindici minuti (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| Unraid parity check found errors | L'ultimo controllo di parità Unraid è fallito o ha trovato errori di sincronizzazione: la parità non corrisponde più ai dati (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 min | Advisory (`warning`) | 1 d |
| Unraid parity check overdue | L'ultimo controllo di parità Unraid ha più di 40 giorni: un controllo mensile è stato saltato (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 d | 1 h | Advisory (`warning`) | 1 d |
| Unraid cache nearly full | Un pool cache Unraid è pieno per più del 90%: le scritture ricadono sull'array lento, oppure falliscono (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |
| Unraid container stopped | Un container Docker impostato per avviarsi con l'array non è in esecuzione (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 min | Advisory (`warning`) | 6 h |

### Veeam Backup & Replication

Vedi [Veeam Backup & Replication](../devices/veeam.md). I job sono etichettati `job` e `type`, i repository `repository`.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Veeam job failed | L'ultima esecuzione di un job Veeam abilitato è fallita: ciò che protegge non ha alcun nuovo punto di ripristino (`dumbmonit_veeam_job_failed`). | > 0 | 5 min | Warning (`critical`) | 1 d |
| Veeam job ended with a warning | L'ultima esecuzione di un job Veeam abilitato è terminata con un avviso: alcuni oggetti potrebbero non essere protetti (`dumbmonit_veeam_job_warning`). | > 0 | 5 min | Advisory (`warning`) | 1 d |
| Veeam repository nearly full | Un repository di backup Veeam è pieno per più del 90%: i prossimi job falliranno per mancanza di spazio (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |
| Veeam license expiring | La licenza Veeam scade in meno di 30 giorni: i job si fermano alla fine del periodo di grazia (`dumbmonit_veeam_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 1 d |

### Tailscale

Vedi [Tailscale](../devices/tailscale.md). I dispositivi sono etichettati `device`, la prima etichetta del loro nome MagicDNS.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Tailscale device offline | Un dispositivo Tailscale che deve restare online (un dispositivo taggato per impostazione predefinita) è disconnesso da più di 15 minuti (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 min | 5 min | Advisory (`warning`) | 6 h |
| Tailscale node key expiring | La chiave di nodo di un dispositivo Tailscale scade in meno di 14 giorni: lascerà il tailnet finché qualcuno non lo riconnette (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 d | 1 h | Advisory (`warning`) | 1 d |
| Tailscale device awaiting approval | Un dispositivo si è unito al tailnet e attende l'approvazione: approvalo se è previsto, rimuovilo altrimenti (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 min | Advisory (`warning`) | 1 d |
| Tailscale client update | Un dispositivo Tailscale che deve restare online esegue un client con un aggiornamento disponibile (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Info (`info`) | 7 d |

### FortiGate

Vedi [FortiGate](../devices/fortigate.md). I tunnel sono etichettati `tunnel`, le interfacce `interface` e `alias`, le licenze `license`.

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| FortiGate IPsec tunnel down | Nessuna fase 2 di un tunnel IPsec site-to-site FortiGate è attiva: il sito dietro di esso è tagliato fuori (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| FortiGate interface without link | Un'interfaccia FortiGate abilitata con un indirizzo non ha link: cavo, transceiver o il dispositivo davanti (`dumbmonit_fortigate_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| FortiGate HA out of sync | I membri di un cluster HA FortiGate non hanno più lo stesso checksum di configurazione: un failover eseguirebbe un'altra configurazione (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 min | Advisory (`warning`) | 6 h |
| FortiGate HA member lost | Un cluster HA FortiGate ha meno membri che nelle ultime 24 ore: non c'è più alcuno standby (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| FortiGate memory near conserve mode | La memoria FortiGate è rimasta sopra l'85% per dieci minuti: all'88% (per impostazione predefinita) entra in modalità conserve e smette di ispezionare nuove sessioni (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 min | Advisory (`warning`) | 6 h |
| FortiGate CPU high | La CPU FortiGate è rimasta sopra il 90% per quindici minuti (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| FortiGate licence expiring | Una licenza FortiGuard o FortiCare del FortiGate scade in meno di 30 giorni, oppure è scaduta (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 1 d |
| FortiGate firmware update | FortiGuard offre una release patch più recente del branch FortiOS che il FortiGate esegue (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Sophos Firewall

Vedi [Sophos Firewall](../devices/sophos.md).

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Sophos interface without link | Un'interfaccia Sophos Firewall accesa e associata a una zona non ha link (`dumbmonit_sophos_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Hyper-V

Riportato dall'agent Windows su un host Hyper-V; vedi [Agent](../devices/agent.md#hyper-v-hosts).

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Hyper-V VM health critical | Hyper-V riporta una macchina virtuale in stato di salute critico: una risorsa di cui ha bisogno, più spesso lo spazio su disco, è esaurita (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Hyper-V host CPU high | I processori logici dell'host Hyper-V sono stati occupati per più del 90% per quindici minuti, guest inclusi (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| Hyper-V virtual disk errors | Un disco virtuale Hyper-V ha riportato errori di I/O nell'ultima ora (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Punteggio di sicurezza

Calcolato dal server ogni 15 minuti per ogni dispositivo il cui tipo ha controlli di sicurezza, e memorizzato come `dumbmonit_security_score` (0-100). Vedi [Punteggio di sicurezza](../using/security-score.md).

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| Security score dropped | Il punteggio è sceso di 10 punti o più rispetto al suo valore migliore delle ultime 24 ore (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): un controllo ha iniziato a fallire. | ≥ 10 pt | 30 min | Advisory (`warning`) | 24 h |
| Security score low | Il punteggio è sotto 40, voto F (`dumbmonit_security_score`). Alza o abbassa la soglia per corrispondere alla tua propria barra. | < 40 | 1 h | Info (`info`) | 7 d |

### DumbMonit stesso

| Regola | Cosa | Soglia predefinita | Attesa | Severità | Ricordo |
|---|---|---|---|---|---|
| DumbMonit backup did not run | Il backup locale pianificato del database DumbMonit non è stato eseguito da più di due giorni (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |

Ogni regola integrata si applica a tutti i dispositivi e a ogni canale
abilitato.

## Modificare una regola nell'interfaccia

Sulla pagina **Alerts**, la sezione *Rules* elenca ogni regola con la
sua severità, un marchio *Built-in* e, quando pertinente, quanti avvisi
sta attivando ora. Ogni regola ha:

- un interruttore per **abilitarla o disattivarla**;
- **Edit**, che apre un editor inline per nome, descrizione, operatore,
  soglia, attesa (`for`), severità, ricordo (`repeat every`) ed
  escalation (`escalate after`). La query di una regola integrata viene
  mostrata ma non è modificabile; la query delle tue proprie regole lo
  è. Le regole di baseline mostrano i loro parametri di rilevamento in
  sola lettura;
- **Delete**, solo per le tue proprie regole.

## Override per dispositivo

Una regola può essere messa a punto per un dispositivo senza toccare la
regola stessa: una soglia diversa, una soglia di azzeramento diversa,
oppure disattivata solo per quel dispositivo. L'editor di ogni regola
ha un pannello **Per-device overrides** per aggiungerne o rimuoverne
uno direttamente.

L'azione a un clic **Ignore** su un avviso (nell'elenco *Needs you* o
sulla pagina del dispositivo dell'avviso) è lo stesso meccanismo,
scritto con `enabled: false`: il dispositivo smette di portare quella
regola, immediatamente, reversibilmente. La pagina del dispositivo
elenca cosa ignora, con **Stop ignoring**; la regola stessa mostra
*Ignored on N devices* in Alerts → Rules, dove lo stesso pannello di
override per dispositivo la rimuove. Vedi
[Override per dispositivo](../reference/api.md#per-device-overrides)
per l'API.

## Creare una regola di soglia

Clicca **New rule**. Il modulo richiede:

| Campo | Significato |
|---|---|
| Name | Mostrato negli avvisi e nelle notifiche. |
| Query | Un'espressione MetricsQL, valutata per dispositivo. Esempio: `dumbmonit_cpu_usage_percent`. |
| Operator | `>`, `>=`, `<` o `<=`. |
| Threshold | Il numero con cui il risultato della query viene confrontato. |
| Severity | Info, Advisory o Warning. |
| Hold for | Secondi che la condizione deve durare prima di attivarsi. Un'attesa diversa da zero assorbe un campione isolato errato. |

Ogni serie restituita dalla query diventa un potenziale avviso,
collegato al dispositivo nominato dalla sua etichetta `target`.
Aggrega con `avg by (target, host) (…)` quando vuoi un avviso per
dispositivo invece di uno per core o per interfaccia. L'identità
dell'avviso ignora le etichette `host` e `tag_*` ogni volta che
`target` è presente, così rinominare un dispositivo mantiene i suoi
avvisi; una serie il cui `target` non corrisponde a un dispositivo
esistente e abilitato viene ignorata.

## Il linguaggio di query

Le query sono [MetricsQL](https://docs.victoriametrics.com/metricsql/),
il superset PromQL di VictoriaMetrics. Le metriche sono le serie
`dumbmonit_*` descritte nel
[riferimento delle metriche](../reference/metrics.md), e ogni serie
porta un'etichetta `target` (l'id del dispositivo) e un'etichetta
`host` (il suo nome).

Alcuni pattern usati dalle regole integrate:

```
# Età dell'ultimo campione: cosa significa "unreachable"
time() - tlast_over_time(dumbmonit_up[7d])

# Un valore per dispositivo, qualunque sia la fonte
avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)

# Una percentuale calcolata da due gauge
100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total

# Una condizione booleana: la serie viene restituita solo quando vera
dumbmonit_ups_output_source == 5

# Estrapolazione fatta da VictoriaMetrics
predict_linear(dumbmonit_pbs_datastore_used_percent[6h], 345600) and deriv(dumbmonit_pbs_datastore_used_percent[6h]) > 0

# Disponibilità di un servizio su trenta giorni
avg_over_time(dumbmonit_probe_success[30d])
```

!!! tip "Prova prima una query"
    `GET /api/metrics/query?query=…` restituisce ciò che la regola
    valuterebbe (vedi il [riferimento API](../reference/api.md#metrics)).
    In sviluppo, l'overlay pubblica il VictoriaMetrics integrato sulla
    `:8428` dell'host, con la sua interfaccia di query.

## Tipi di regola nell'API

`kind` è `threshold` (confronta con una soglia), `predict` (lo stesso
confronto, su una query `predict_linear`; mostrato come una previsione)
oppure `anomaly` (baseline stagionale; `params` contiene `k`, `alpha`,
`mad_floor_abs`, `mad_floor_rel`, `min_samples`). L'interfaccia crea
regole `threshold`; gli altri due tipi possono essere creati tramite
l'API.
