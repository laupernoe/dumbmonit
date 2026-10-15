# Regole

Le regole vengono inserite al primo avvio, poi diventano tue: DumbMonit non
riscrive mai una regola che hai modificato, ricrea solo quelle mancanti. Le
regole integrate possono essere modificate e disabilitate, non eliminate.

## Regole integrate

Le gravità sono mostrate con la parola dell'interfaccia; il valore dell'API è tra
parentesi. Una regola la cui metrica non è prodotta da alcun monitor non ha
serie e non scatta mai, quindi ogni regola qui sotto viene distribuita abilitata
su ogni istanza.

### Ogni monitor

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Monitor irraggiungibile | Nessuna misura ricevuta da più di tre minuti. `bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))` | > 180 s | 1 min | Avvertenza (`critical`) | 6 h |
| CPU elevata | Carico CPU sostenuto sopra il 90%, mediato per monitor tra sorgenti SNMP, Proxmox e agent; rientra sotto l'85 %. | > 90 % | 10 min | Avviso (`warning`), escalation dopo 1 h | 6 h |
| CPU inusuale | Carico CPU sensibilmente diverso dal solito a quest'ora e in questo giorno della settimana (baseline stagionale). | score > 3.5 | 15 min | Informazioni (`info`) | 6 h |
| Disco quasi pieno | Filesystem pieno al 90% o più, per punto di mount (SNMP, storage Proxmox e filesystem root); rientra sotto l'88 %. | ≥ 90 % | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |
| Filesystem quasi pieno | A questo ritmo, il filesystem sarà pieno entro quattro giorni (`predict_linear` su 6 h; solo filesystem in crescita). | ≥ 100 % | 30 min | Avviso (`warning`) | 6 h |

### UPS

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| UPS a batteria | L'UPS alimenta il carico dalla batteria (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Avvertenza (`critical`) | 15 min |
| Batteria UPS scarica | La batteria dell'UPS è in esaurimento o esaurita (`dumbmonit_ups_battery_status`, 3 = in esaurimento, 4 = esaurita). | ≥ 3 | 1 min | Avvertenza (`critical`) | 30 min |

### Porte di rete (SNMP)

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Porta con errori in accumulo | Una porta di rete ha registrato più di cinquanta errori nell'ultima ora: di solito un cavo in avaria, una fibra sporca o un duplex non corrispondente (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; nulla scatta durante la prima ora di un nuovo monitor, quindi gli errori contati prima della sua aggiunta non vengono segnalati come nuovi). | > 50 | 15 min | Avviso (`warning`) | 24 h |
| Porta instabile (flapping) | Una porta di rete è caduta ed è risalita più di due volte in trenta minuti (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Una porta disabilitata di proposito non ha serie e non scatta mai. | > 4 | 5 min | Avviso (`warning`) | 1 h |

### Hardware dei server (Redfish) {#server-hardware-redfish}

Le serie di salute valgono 0 (OK), 1 (Avviso) o 2 (Critico). Uno slot vuoto (`Absent`) o disabilitato non ha serie, quindi un server consegnato con un alimentatore su due non scatta mai.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Ventola del server guasta | Il controller di gestione segnala una ventola come guasta (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 min | Avvertenza (`critical`) | 6 h |
| Temperatura del server sopra il livello critico | Un sensore di temperatura è sopra la soglia critica dichiarata dal controller stesso (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| Ridondanza degli alimentatori persa | Gli alimentatori non sono più ridondanti: un altro guasto e il server si spegne (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 min | Avviso (`warning`), escalation dopo 1 h | 6 h |
| Alimentatore guasto | Il controller segnala un alimentatore come guasto (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 min | Avvertenza (`critical`) | 6 h |
| Guasto previsto di un disco | Un disco segnala un guasto previsto: sostituiscilo finché funziona (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 min | Avviso (`warning`) | 24 h |
| Salute del server critica | Il controller segnala la salute del sistema stesso come Critica (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 min | Avvertenza (`critical`) | 6 h |

### Monitor di uptime e heartbeat

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Servizio non disponibile | Il servizio non risponde correttamente da tre minuti (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 min | Avvertenza (`critical`) | 30 min |
| Heartbeat mancato | Il job non si è fatto sentire entro l'intervallo previsto più il periodo di tolleranza, oppure ha segnalato esso stesso un fallimento (`dumbmonit_probe_success{probe="push"} == bool 0`; il ritardo è impostato per monitor, vedi [Heartbeat](../devices/push.md)). | > 0 | 2 min | Avviso (`warning`) | 6 h |
| Servizio instabile (flapping) | Il servizio ha cambiato stato più di sei volte in trenta minuti (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 min | Avviso (`warning`) | 1 h |
| Servizio lento | Il servizio impiega più di tre secondi a rispondere; i controlli falliti sono esclusi (`dumbmonit_probe_duration_seconds and dumbmonit_probe_success == 1`). | > 3 s | 10 min | Avviso (`warning`) | 6 h |
| Certificato in scadenza | Il certificato scade tra meno di quattordici giorni. | < 14 d | 1 h | Avviso (`warning`) | 24 h |
| Certificato scaduto | Il certificato è scaduto. | < 0 d | 5 min | Avvertenza (`critical`) | 24 h |

### Container Docker

Segnalati dall'agent; vedi [Agent](../devices/agent.md). L'avviso riporta il nome
del container.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Container fermo | Il container è fermo da due minuti (`dumbmonit_container_up == bool 0`). Un container che Docker non riavvierebbe è escluso (`dumbmonit_container_stop_expected = 1`): un `docker run` una tantum terminato, un container mai avviato, o uno fermato a mano senza restart policy. Un crash (qualsiasi altro codice di uscita), un kill per memoria esaurita (OOM), o un container `always` / `unless-stopped` conta comunque. | > 0 | 2 min | Avviso (`warning`) | 1 h |
| Container non sano | Il controllo di salute del container fallisce da tre minuti (`dumbmonit_container_health == 2`; 0 nessuno, 1 sano, 3 in avvio). Un container fermo è escluso: Docker conserva il suo ultimo verdetto, che non significa più nulla. | > 0 | 3 min | Avviso (`warning`) | 1 h |
| Container in riavvio continuo | Il container si è riavviato tre volte o più in quindici minuti (`increase(dumbmonit_container_restart_count[15m])`): raramente resta fermo abbastanza a lungo per la regola precedente. | ≥ 3 | 1 min | Avviso (`warning`) | 1 h |
| Aggiornamento del container disponibile | Nel registry è disponibile un'immagine più recente per questo container (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Informazioni (`info`) | 24 h |

### Backup Plakar

Segnalati dall'agent, per kloset e per sorgente. Nessuna di queste serie esiste
finché Plakar non viene rilevato, quindi nessuna regola può scattare su una
macchina che non lo ha.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Backup Plakar troppo vecchio | Nessun nuovo snapshot Plakar per questa sorgente da più di due giorni (`dumbmonit_backup_last_success_seconds`). | > 2 d | 1 h | Avviso (`warning`) | 24 h |
| Backup Plakar fallito | L'ultimo snapshot contiene errori (`dumbmonit_backup_last_status{source!="*"} < 1`). | < 1 | 10 min | Avvertenza (`critical`) | 24 h |
| Kloset Plakar non leggibile | L'agent non riesce ad aprire il kloset: storage irraggiungibile, passphrase o plugin mancante; il log dell'agent dice quale, una sola volta (`dumbmonit_backup_last_status{source="*"} < 1`). | < 1 | 30 min | Avviso (`warning`) | 24 h |

### Backup restic e Borg

Segnalati dall'agent per ogni repository dichiarato in `restic_repos` o
`borg_repos` in agent.yaml; vedi [Agent](../devices/agent.md#restic-and-borg-backups).
Senza un repository dichiarato non c'è alcuna serie, quindi nessuna regola può
scattare.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Backup restic/Borg troppo vecchio | Nessun nuovo snapshot restic o Borg in questo repository da più di 48 ore (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Avviso (`warning`) | 24 h |
| Repository restic/Borg irraggiungibile | L'agent non riesce a leggere il repository: irraggiungibile, password rifiutata o binario mancante; il log dell'agent dice quale (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 min | Avviso (`warning`) | 24 h |

### Tunnel WireGuard

Segnalati dall'agent; vedi [Agent](../devices/agent.md#wireguard-tunnels).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Peer WireGuard silenzioso | Un peer con keepalive persistente non completa un handshake da più di 15 minuti: il tunnel è caduto (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Un peer che non si è mai connesso conta il suo silenzio dal momento in cui l'agent lo ha visto per la prima volta. I peer senza keepalive (un telefono, un portatile) restano inattivi per giorni e vengono esclusi; cambia la soglia per cambiare N, oppure elimina la parte `and on …` per sorvegliare ogni peer. | > 15 min | 5 min | Avviso (`warning`) | 6 h |

### Tunnel IPsec

Segnalati dall'agent; vedi [Agent](../devices/agent.md#ipsec-tunnels). La serie
esiste solo per un CHILD_SA dichiarato nella configurazione di strongSwan, quindi
la regola scatta su un tunnel che non si è mai avviato, non solo su uno caduto.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Tunnel IPsec non attivo | Un tunnel IPsec configurato non ha una sessione installata (`dumbmonit_agent_ipsec_tunnel_up < 1`). | < 1 | 10 min | Avviso (`warning`) | 6 h |

### Speed test di Internet

Segnalato dall'agent; vedi [Agent](../devices/agent.md#internet-speed-test).
Il test è disattivato per impostazione predefinita e ogni regola resta silenziosa
finché la soglia corrispondente non viene impostata in `agent.yaml`: nessuno può
indovinare che cosa significhi "lento" per la tua linea.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Velocità di download Internet bassa | L'ultimo test ha misurato una velocità di download inferiore a `speedtest_min_download_mbps` (`dumbmonit_agent_speedtest_download_min_bytes_per_second - dumbmonit_agent_speedtest_download_bytes_per_second > 0`). | > 0 | 10 min | Avviso (`warning`) | 24 h |
| Velocità di upload Internet bassa | Lo stesso, per `speedtest_min_upload_mbps`. | > 0 | 10 min | Avviso (`warning`) | 24 h |
| Latenza Internet alta | L'ultimo test ha misurato una latenza superiore a `speedtest_max_latency_ms`. | > 0 | 10 min | Avviso (`warning`) | 24 h |

### Dischi, ZFS e sensori dell'agent

Segnalati dall'agent; vedi [Agent](../devices/agent.md).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Disco con SMART in errore | L'autovalutazione SMART del disco lo segnala come in avaria (`dumbmonit_agent_disk_smart_ok == bool 0`). Sostituiscilo e controlla i backup. | > 0 | 10 min | Avvertenza (`critical`) | 24 h |
| Pool ZFS degradato | Un pool ZFS non è più sano: un dispositivo manca, è in errore o è stato rimosso (`dumbmonit_agent_zfs_pool_health`: 0 online, 1 degradato, 2 qualsiasi altro stato). | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| Lo scrub ZFS ha trovato errori | L'ultimo scrub ZFS ha trovato errori su questo pool, oppure dei file sono definitivamente persi (`dumbmonit_agent_zfs_pool_scrub_errors or dumbmonit_agent_zfs_pool_data_errors`). Il pool può restare `ONLINE`. | > 0 | 10 min | Avvertenza (`critical`) | 24 h |
| Temperatura sopra il livello critico | Un sensore è sopra la temperatura critica dichiarata dal suo stesso hardware (`dumbmonit_agent_sensor_temperature_celsius >= dumbmonit_agent_sensor_temperature_critical_celsius`); nessun valore fisso. | > 0 | 5 min | Avvertenza (`critical`) | 6 h |

### Proxmox VE

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Backup troppo vecchio | Nessun backup Proxmox VE riuscito da più di sette giorni (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 d | 1 h | Avviso (`warning`) | 24 h |
| VM o container fermo | Il guest era in esecuzione nelle ultime due ore ed è fermo da cinque minuti (`1 - dumbmonit_proxmox_guest_running`, in `and` con `max_over_time(…[2h]) == 1`). | > 0 | 5 min | Avviso (`warning`) | 6 h |
| CPU di VM o container elevata | Un guest ha usato più del 90 % dei core assegnati per quindici minuti (`dumbmonit_proxmox_guest_cpu_percent`). La notifica riporta il nome del guest, `nextcloud (202)`. | > 90 % | 15 min | Avviso (`warning`) | 6 h |
| Memoria di VM o container elevata | Un guest ha usato più del 95 % della sua memoria per dieci minuti (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 min | Avviso (`warning`) | 6 h |
| Disco di VM o container quasi pieno | Il disco root di un guest è pieno per più del 90 % (`dumbmonit_proxmox_guest_disk_used_percent`): sempre per i container, per le VM solo quando il guest agent QEMU segnala l'utilizzo. | > 90 % | 15 min | Avviso (`warning`) | 24 h |
| Guest bloccato (lock) | Una VM o un container è bloccato da sei ore (`dumbmonit_proxmox_guest_locked`, etichetta `lock`): un backup o una migrazione interrotti hanno lasciato il lock e nessuna operazione può essere eseguita sul guest, nemmeno il prossimo backup. | > 0 | 6 h | Avviso (`warning`) | 24 h |
| Snapshot vecchio | Lo snapshot più vecchio di un guest ha più di trenta giorni (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 d | 1 h | Informazioni (`info`) | 7 d |
| Risorsa HA in errore | Una risorsa HA di Proxmox è in errore o isolata (fenced) (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 min | Avvertenza (`critical`) | 1 h |
| Gestore HA non risponde | Il local resource manager HA di un nodo ha smesso di scrivere il proprio stato (`dumbmonit_proxmox_ha_lrm_stale`): i guest HA di quel nodo non verranno né riavviati né spostati. | > 0 | 5 min | Avvertenza (`critical`) | 1 h |
| Cluster senza quorum | Il cluster Proxmox non ha quorum (`1 - dumbmonit_proxmox_cluster_quorate`): non è più possibile avviare i guest. | > 0 | 1 min | Avvertenza (`critical`) | 30 min |
| Nodo Proxmox offline | Un nodo del cluster risulta offline (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 min | Avvertenza (`critical`) | 30 min |
| Servizio Proxmox non attivo | Un demone Proxmox essenziale non è in esecuzione su un nodo (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, ciascuno contato solo dove il nodo lo abilita). | > 0 | 5 min | Avvertenza (`critical`) | 1 h |
| Interfaccia di rete del nodo inattiva | Un'interfaccia impostata per avviarsi al boot non è attiva su un nodo (`dumbmonit_proxmox_node_interface_offline`). Un'interfaccia attivata su richiesta non viene mai segnalata. | > 0 | 5 min | Avviso (`warning`) | 6 h |
| Storage Proxmox quasi pieno | Uno storage Proxmox è pieno per più dell'85% (`dumbmonit_proxmox_storage_used_percent`); "Disco quasi pieno" subentra al 90%. | > 85 % | 15 min | Avviso (`warning`) | 24 h |
| Thin pool quasi pieno | Un thin pool LVM è pieno per più del 90 %: ogni guest memorizzato al suo interno passa in sola lettura quando si riempie (`dumbmonit_proxmox_node_thinpool_used_percent`); rientra sotto l'88 %. | > 90 % | 15 min | Avvertenza (`critical`) | 6 h |
| Metadati del thin pool quasi pieni | Il volume dei metadati di un thin pool LVM è pieno per più dell'80 % (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). È molto più piccolo del volume dati e spesso si satura per primo, con la stessa conseguenza. | > 80 % | 15 min | Avvertenza (`critical`) | 6 h |
| Disco Proxmox in usura | Un SSD o NVMe di un nodo ha consumato più del 90 % della sua vita nominale (`dumbmonit_proxmox_node_disk_wearout_percent`). La notifica riporta `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Avviso (`warning`) | 7 d |
| Guasto SMART di un disco Proxmox | Un disco di un nodo segnala uno stato di salute SMART `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 min | Avvertenza (`critical`) | 24 h |
| Pool ZFS Proxmox degradato | Un pool ZFS di un nodo non è `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). La notifica riporta `pve2 · tank`. | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| Job di backup fallito | L'ultimo job vzdump su un nodo, o un job di backup pianificato, è fallito (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 min | Avvertenza (`critical`) | 24 h |
| Il job di backup esclude un disco | Un job di backup pianificato salta un disco di uno dei guest che copre (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): il job riesce ogni notte con quei dati mancanti. I lettori CD-ROM e le immagini cloudinit sono esclusi per progetto e non vengono mai contati. | > 0 | 1 h | Avviso (`warning`) | 7 d |
| Replica fallita | Un job di replica segnala un errore o un numero di fallimenti diverso da zero (`dumbmonit_proxmox_replication_job_error`): la copia di standby è obsoleta. | > 0 | 10 min | Avvertenza (`critical`) | 6 h |
| Ceph in errore di salute | Ceph segnala `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 min | Avvertenza (`critical`) | 30 min |
| Ceph in avviso di salute | Ceph segnala `HEALTH_WARN` da più di quindici minuti (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 min | Avviso (`warning`) | 6 h |
| OSD Ceph non attivo | Un OSD Ceph è giù da cinque minuti (`1 - dumbmonit_proxmox_ceph_osd_up`). La notifica riporta l'OSD e il suo host. | > 0 | 5 min | Avvertenza (`critical`) | 1 h |
| OSD Ceph quasi pieno | Un singolo OSD Ceph è pieno per più dell'85 % (`dumbmonit_proxmox_ceph_osd_used_percent`): le scritture si fermano quando raggiunge il limite, qualunque sia la media del cluster; rientra sotto l'82 %. | > 85 % | 15 min | Avviso (`warning`) | 24 h |
| Pool Ceph quasi pieno | Un pool Ceph è pieno per più dell'85 % (`dumbmonit_proxmox_ceph_pool_used_percent`); rientra sotto l'82 %. | > 85 % | 15 min | Avviso (`warning`) | 24 h |
| Flag Ceph noout lasciato attivo | Il flag Ceph `noout` è impostato da due ore (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): il ribilanciamento è disabilitato e il cluster appare sano mentre la sua ridondanza si riduce. | > 0 | 2 h | Informazioni (`info`) | 24 h |
| Aggiornamenti Proxmox in sospeso | Più di venti aggiornamenti di pacchetti sono in sospeso su un nodo (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Informazioni (`info`) | 7 d |
| Aggiornamenti di sicurezza Proxmox in sospeso | Almeno un aggiornamento in sospeso proviene da un archivio di sicurezza (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Avviso (`warning`) | 7 d |
| Pacchetti Proxmox modificati | La versione installata di un pacchetto Proxmox è cambiata tra due probe — qualcuno ha eseguito `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, pubblicata per un'ora con il dettaglio nell'etichetta `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Si risolve da sola un'ora dopo. | > 0 | 1 min | Informazioni (`info`) | 24 h |
| Certificato del nodo in scadenza | Un certificato di un nodo Proxmox scade tra meno di quattordici giorni (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 d | 1 h | Avviso (`warning`) | 24 h |

### Proxmox Backup Server

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Datastore PBS quasi pieno | Datastore PBS pieno per più del 90%. | > 90 % | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |
| Datastore PBS in riempimento | Al ritmo attuale, PBS stima il datastore pieno entro sette giorni. | < 7 d | 1 h | Avviso (`warning`) | 24 h |
| Datastore PBS non montato | Un datastore rimovibile non è montato (`dumbmonit_pbs_datastore_removable_unmounted`): non vi si può scrivere nulla. | > 0 | 6 h | Avviso (`warning`) | 24 h |
| Backup PBS troppo vecchio | Nessun nuovo snapshot per questo gruppo di backup (uno per macchina) da più di due giorni (`dumbmonit_pbs_backup_last_age_seconds`). Modifica la soglia per le macchine di cui si fa il backup settimanalmente; la notifica riporta il gruppo e il datastore. | > 2 d | 1 h | Avviso (`warning`) | 24 h |
| Verifica del backup PBS fallita | La verifica dell'ultimo snapshot di questa macchina è fallita (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 min | Avvertenza (`critical`) | 24 h |
| Task PBS fallito | Almeno un task PBS è fallito nella finestra di controllo (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 min | Avviso (`warning`) | 24 h |
| Garbage collection PBS troppo vecchia | Nessuna garbage collection riuscita su questo datastore da più di otto giorni (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 d | 1 h | Avviso (`warning`) | 24 h |
| Garbage collection PBS fallita | L'ultima garbage collection su un datastore è fallita (`dumbmonit_pbs_gc_last_run_ok < 1`; da `/gc` su PBS 3.3+, dall'ultimo task GC in precedenza). | < 1 | 10 min | Avviso (`warning`) | 24 h |
| Chunk PBS corrotti trovati | La garbage collection ha trovato chunk illeggibili su un datastore (`dumbmonit_pbs_gc_bad_chunks`): alcuni backup non possono più essere ripristinati. | > 0 | 10 min | Avvertenza (`critical`) | 24 h |
| Job di prune PBS fallito | L'ultima esecuzione di un job di prune PBS è fallita: nulla viene eliminato e il datastore continua a riempirsi (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). La notifica riporta il job e il datastore. | < 1 | 10 min | Avviso (`warning`) | 24 h |
| Job di verifica PBS fallito | L'ultima esecuzione di un job di verifica PBS è fallita (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 min | Avviso (`warning`) | 24 h |
| Job di sync PBS fallito | L'ultima esecuzione di un job di sync PBS è fallita (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 min | Avvertenza (`critical`) | 24 h |
| Backup su nastro PBS fallito | L'ultima esecuzione del backup su nastro è fallita (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): la copia offline non viene eseguita. | < 1 | 10 min | Avvertenza (`critical`) | 24 h |
| Job PBS mai eseguito | Un job è abilitato e pianificato ma non è mai stato eseguito (`dumbmonit_pbs_job_never_run`). | > 0 | 2 d | Avviso (`warning`) | 7 d |
| Servizio PBS non attivo | Un servizio necessario a Proxmox Backup Server non è in esecuzione (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); finché è fermo non si può eseguire alcun backup. | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Guasto SMART di un disco PBS | Un disco del server di backup segnala uno stato di salute SMART `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). La notifica riporta `/dev/sdb`. | > 0 | 5 min | Avvertenza (`critical`) | 24 h |
| SSD PBS esaurito | Un SSD del server di backup ha consumato più del 90 % della sua resistenza nominale (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Avviso (`warning`) | 7 d |
| Pool ZFS PBS degradato | Un pool ZFS del server di backup non è `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| Certificato PBS in scadenza | Il certificato servito dal server di backup scade tra meno di tre settimane (`dumbmonit_pbs_node_certificate_expires_seconds`). Richiede l'opzione `certificates`. | < 21 d | 1 h | Avviso (`warning`) | 48 h |
| Riavvio PBS in sospeso dopo l'aggiornamento | Il pacchetto è stato aggiornato ma il demone in esecuzione è ancora quello vecchio (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Informazioni (`info`) | 7 d |
| Aggiornamenti PBS in sospeso | Più di venti aggiornamenti di pacchetti sono in sospeso sul server di backup (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Informazioni (`info`) | 7 d |

### Proxmox Datacenter Manager

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Istanza federata irraggiungibile | La console del datacenter non riesce a raggiungere uno dei cluster Proxmox VE o dei server di backup che federa (`dumbmonit_pdm_remote_reachable < 1`). La notifica riporta l'istanza e il suo prodotto. | < 1 | 10 min | Avvertenza (`critical`) | 6 h |
| Istanza federata indietro con la versione | Un'istanza esegue una versione più vecchia di un'altra istanza dello stesso prodotto nello stesso parco (`dumbmonit_pdm_remote_version_behind`). Confrontata tra istanze, mai con ciò che Proxmox pubblica. | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Task fallito su un'istanza federata | Almeno un task è fallito su un'istanza federata nella finestra di controllo (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 min | Avviso (`warning`) | 24 h |
| Disco della console del datacenter quasi pieno | Il filesystem root della console del datacenter è pieno per più del 90 % (`dumbmonit_pdm_node_rootfs_percent`): quando è pieno, la console smette di registrare ciò che raccoglie. | > 90 % | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |
| Certificato della console del datacenter in scadenza | Un certificato della console del datacenter scade tra meno di quattordici giorni (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 d | 1 h | Avviso (`warning`) | 24 h |
| Aggiornamenti della console del datacenter in sospeso | Più di venti aggiornamenti di pacchetti sono in sospeso sulla console del datacenter (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Informazioni (`info`) | 7 d |

### Proxmox Mail Gateway

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Coda di posta in crescita | La coda della posta differita è cresciuta di più di cento messaggi in due ore (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): il next hop probabilmente rifiuta la posta. Si misura la crescita e non la dimensione, perché un gateway molto usato ha sempre qualche decina di messaggi differiti. | > 100 | 15 min | Avviso (`warning`) | 6 h |
| Posta bloccata in coda | Un messaggio attende in una coda da più di quattro ore (`dumbmonit_pmg_queue_oldest_age_seconds`). L'età è un limite inferiore — qshape riporta intervalli — quindi la regola scatta in ritardo piuttosto che in anticipo. La notifica riporta la coda. | > 4 h | 15 min | Avvertenza (`critical`) | 6 h |
| Servizio del mail gateway fermo | Un servizio necessario al gateway per accettare e filtrare la posta è fermo (`dumbmonit_pmg_service_running` per postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Un'unità non installata non ha serie e non può scattare. | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Firme antivirus non aggiornate | Le firme antivirus giornaliere di ClamAV hanno più di due giorni (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): freshclam ha smesso di aggiornarle e il gateway continua a filtrare con l'elenco della settimana scorsa. | > 2 d | 1 h | Avviso (`warning`) | 24 h |
| Regole antispam non aggiornate | Le regole di SpamAssassin hanno più di otto giorni (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Un canale che non è mai stato aggiornato non ha data né serie. | > 8 d | 1 h | Avviso (`warning`) | 7 d |
| Quarantena in riempimento | La quarantena dello spam ha accolto più di mille messaggi in sei ore (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): una campagna, oppure una regola che ha appena messo un dominio legittimo nel cestino. | > 1000 | 30 min | Avviso (`warning`) | 12 h |
| Cluster del mail gateway degradato | Un gateway del cluster non è più sincronizzato con gli altri (`dumbmonit_pmg_cluster_node_insync`): filtra con un database di regole non aggiornato. Un gateway standalone non ha serie. | < 1 | 15 min | Avviso (`warning`) | 6 h |
| Certificato del mail gateway in scadenza | Un certificato del mail gateway scade tra meno di quattordici giorni (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 d | 1 h | Avviso (`warning`) | 24 h |
| Aggiornamenti del mail gateway in sospeso | Più di venti aggiornamenti di pacchetti sono in sospeso sul mail gateway (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Informazioni (`info`) | 7 d |

### MDaemon e SecurityGateway {#mdaemon-and-securitygateway}

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Servizio di posta MDaemon fermo | Un servizio di posta sorvegliato del server MDaemon non risponde, oppure saluta per rifiutare il servizio (`dumbmonit_mdaemon_service_up`). Hanno una serie solo i servizi elencati sul monitor. La notifica riporta il servizio e la sua porta. | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| API XML di MDaemon non risponde | L'API XML ha smesso di rispondere mentre i servizi di posta rispondono ancora (`dumbmonit_mdaemon_api_up`): probabilmente Remote Administration è fermo. Ha la serie solo un monitor con un account. | < 1 | 15 min | Avviso (`warning`) | 12 h |
| Coda di posta MDaemon in crescita | La coda in entrata, locale o remota è cresciuta di più di cento messaggi in due ore (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): MDaemon ha smesso di consegnare da essa. Letta dall'[agent](../devices/agent.md#windows-performance-counters) sul server MDaemon, quindi l'avviso riguarda il monitor dell'agent. | > 100 | 15 min | Avviso (`warning`) | 6 h |
| Coda Bad di MDaemon non vuota | Dei messaggi attendono nella coda Bad (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): MDaemon non ha potuto né consegnarli né rimbalzarli, e restano lì finché un amministratore non li rilascia o li elimina. | > 0 | 30 min | Avviso (`warning`) | 24 h |
| Coda Retry di MDaemon alta | Più di cinquanta messaggi attendono nella coda Retry (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): i server remoti continuano a rifiutare temporaneamente la posta. | > 50 | 1 h | Avviso (`warning`) | 6 h |
| Servizio SecurityGateway fermo | Un servizio sorvegliato del gateway non risponde, oppure saluta per rifiutare il servizio (`dumbmonit_securitygateway_service_up`). | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| API di SecurityGateway non risponde | L'API REST ha smesso di rispondere mentre i servizi rispondono ancora (`dumbmonit_securitygateway_api_up`). Ha la serie solo un monitor con una chiave API. | < 1 | 15 min | Avviso (`warning`) | 12 h |
| Coda di consegna di SecurityGateway in crescita | La coda di consegna è cresciuta di più di cento messaggi in due ore (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): il server di posta dietro il gateway probabilmente rifiuta la posta. I nomi dei contatori provengono dall'API così come sono; se la tua versione chiama la coda in modo diverso, adatta il matcher. | > 100 | 15 min | Avviso (`warning`) | 6 h |

### OPNsense

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Gateway Internet non attivo | dpinger non riceve più risposta da un gateway (`dumbmonit_opnsense_gateway_up`). Su un firewall multi-WAN il traffico si è già spostato su un altro collegamento, in silenzio. `loss` e `delay` sono avvisi, non guasti, e non fanno scattare questa regola. | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Gateway con perdita di pacchetti | Un gateway perde più di un quinto dei pacchetti inviati (`dumbmonit_opnsense_gateway_loss_percent`): il collegamento risponde, male. Rientra sotto il 10 %. | > 20 % | 10 min | Avviso (`warning`) | 6 h |
| Gateway lento | Il tempo di andata e ritorno di un gateway resta sopra i 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Trecento millisecondi lasciano in pace un collegamento di backup mobile. | > 300 ms | 15 min | Informazioni (`info`) | 12 h |
| Tabella degli stati del firewall in riempimento | pf sta tracciando più dell'80 % delle connessioni consentite (`dumbmonit_opnsense_pf_states_used_percent`). Oltre il limite il firewall scarta le nuove connessioni senza altri sintomi. | > 80 % | 10 min | Avviso (`warning`) | 6 h |
| Buffer di rete del firewall esauriti | Più del 90 % dei buffer di rete FreeBSD è in uso (`dumbmonit_opnsense_mbuf_used_percent`). Un firewall che li esaurisce smette di inoltrare mentre la CPU è ancora a riposo. | > 90 % | 10 min | Avviso (`warning`) | 6 h |
| Tunnel VPN non attivo | Un tunnel VPN configurato non ha sessioni da dieci minuti (`dumbmonit_opnsense_vpn_tunnel_up`). Un plugin non installato, o una connessione disabilitata, non ha serie e non può scattare. | < 1 | 10 min | Avviso (`warning`) | 6 h |
| Resolver del firewall fermo | Unbound ha smesso di rispondere (`dumbmonit_opnsense_unbound_running`). Il firewall instrada ancora; le macchine dietro di esso non risolvono più i nomi. Disattiva questa regola se il tuo firewall risolve con qualcos'altro. | < 1 | 5 min | Avviso (`warning`) | 6 h |
| Servizio essenziale del firewall fermo | dpinger, che sorveglia i gateway, o configd, che esegue tutto il resto, è fermo (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Solo questi due, perché un servizio spento di proposito non è un guasto. | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Firewall troppo caldo | Un sensore legge più di 85 °C (`dumbmonit_opnsense_temperature_celsius`): la scatola senza ventole nell'armadio sta per iniziare a ridurre le prestazioni. Rientra a 78 °C. | > 85 °C | 10 min | Avviso (`warning`) | 6 h |
| Firewall lasciato in modalità manutenzione CARP | CARP è stato lasciato in modalità manutenzione persistente (`dumbmonit_opnsense_carp_maintenance_mode`): questo firewall ha ceduto i propri indirizzi virtuali al partner e non li riprenderà da solo. | > 0 | 1 h | Informazioni (`info`) | 24 h |
| Riavvio del firewall in sospeso | Un aggiornamento è installato ma richiede un riavvio per avere effetto (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Aggiornamenti del firewall in sospeso | Il firewall ha pacchetti in attesa di aggiornamento (`dumbmonit_opnsense_firmware_updates_pending`). È la macchina più esposta della rete. | > 0 | 1 h | Informazioni (`info`) | 7 d |

### TrueNAS

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Pool NAS degradato | ZFS non considera più sano un pool (`dumbmonit_truenas_pool_healthy`). Un pool degradato serve ancora i suoi dati, ed è esattamente per questo che nessuno se ne accorge fino al guasto del prossimo disco. La pagina del monitor indica il disco. | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Errori sui dischi del NAS | I dischi di un pool hanno segnalato errori di lettura, scrittura o checksum dall'ultimo `zpool clear` (`dumbmonit_truenas_pool_device_errors`, una serie per tipo). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| Pool NAS quasi pieno | Un pool è pieno per più dell'85 % (`dumbmonit_truenas_pool_used_percent`). ZFS rallenta ben prima di essere pieno. Rientra all'80 %. | > 85 % | 30 min | Avviso (`warning`), escalation dopo 7 d | 24 h |
| Lo scrub del NAS ha trovato errori | L'ultimo scrub di un pool ha trovato dati danneggiati (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 min | Avvertenza (`critical`) | 24 h |
| Scrub del NAS in ritardo | Un pool non completa uno scrub da più di 45 giorni (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Assente dopo un resilver, che cancella il record dell'ultimo scrub. | > 45 d | 1 h | Informazioni (`info`) | 7 d |
| Disco del NAS fallisce il test SMART | Un self-test SMART nel registro di un disco è fallito (`dumbmonit_truenas_disk_smart_failed`). Un disco mai testato non ha serie. | > 0 | 5 min | Avvertenza (`critical`) | 24 h |
| Disco del NAS troppo caldo | Un disco è sopra i 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, letta dalla cache di TrueNAS). Rientra a 50 °C. | > 55 °C | 15 min | Avviso (`warning`) | 6 h |
| Dataset del NAS vicino alla quota | Un dataset ha usato più del 90 % della sua quota (`dumbmonit_truenas_dataset_quota_used_percent`): al 100 % le sue scritture falliscono mentre il pool ha ancora spazio. Nessuna quota, nessuna serie. | > 90 % | 30 min | Avviso (`warning`) | 24 h |
| Replica del NAS fallita | Un task di replica abilitato è in errore (`dumbmonit_truenas_replication_error`): la copia dall'altra parte invecchia. | > 0 | 10 min | Avviso (`warning`) | 12 h |
| Task di snapshot del NAS fallito | Un task di snapshot periodico abilitato è in errore (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 min | Avviso (`warning`) | 12 h |
| Snapshot del NAS obsoleti | Un task di snapshot periodico abilitato non viene eseguito da più di otto giorni (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): un task che smette di girare non fallisce, tace. | > 8 d | 1 h | Avviso (`warning`) | 24 h |
| Avviso sollevato da TrueNAS | TrueNAS stesso ha sollevato un avviso di livello ERROR o superiore (`dumbmonit_truenas_alerts`). La sua frase è sulla pagina del monitor. | > 0 | 10 min | Avviso (`warning`) | 12 h |
| Servizio del NAS fermo | Un servizio impostato per avviarsi con il NAS è fermo (`dumbmonit_truenas_service_running`). Un servizio spento di proposito non ha serie. | < 1 | 10 min | Avviso (`warning`) | 6 h |

### Synology DSM

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Avviso SMART su disco Synology | DSM non riporta più lo stato SMART di questo disco come `normal` (`dumbmonit_synology_disk_smart_status`: 1 attenzione, 2 critico, le parole sconosciute contano come 1). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| Disco Synology guasto | DSM segnala il disco come crashato o guasto (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 min | Avvertenza (`critical`) | 6 h |
| Settori danneggiati su disco Synology | Il numero di settori danneggiati del disco supera la soglia impostata in DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 min | Avvertenza (`critical`) | 24 h |
| Settori danneggiati in aumento su disco Synology | Nelle ultime 24 ore sono comparsi nuovi settori illeggibili (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| SSD Synology in usura | Un SSD ha meno del 10 % della sua vita nominale residua (`dumbmonit_synology_disk_remaining_life_percent`); rientra sopra il 12 %. | < 10 % | 1 h | Avviso (`warning`) | 7 d |
| Volume Synology degradato | Un volume è degradato o crashato: la sua ridondanza è persa (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 min | Avvertenza (`critical`) | 6 h |
| Pool di storage Synology degradato | Un pool di storage o una cache SSD è degradato o crashato: un disco si è guastato e la ridondanza è persa o in ricostruzione (`dumbmonit_synology_pool_status or dumbmonit_synology_ssd_cache_status`). | ≥ 2 | 5 min | Avvertenza (`critical`) | 6 h |
| Volume Synology quasi pieno | Un volume è pieno per il 90 % o più (`dumbmonit_synology_volume_used_percent`); rientra sotto l'88 %. | ≥ 90 % | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |
| Temperatura Synology elevata | Un disco è sopra i 55 °C, oppure DSM ha sollevato un proprio avviso di temperatura (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); rientra sotto i 52 °C. | > 55 °C | 15 min | Avviso (`warning`) | 6 h |
| Memoria Synology elevata | Utilizzo della memoria del NAS sopra il 95 % (`dumbmonit_synology_memory_usage_percent`); rientra sotto il 90 %. | > 95 % | 15 min | Avviso (`warning`) | 6 h |
| Task Active Backup fallito | L'ultima esecuzione di un task Active Backup for Business è fallita, oppure ha salvato solo una parte dei suoi dispositivi (`dumbmonit_abb_task_last_status == bool 0`). La notifica riporta il task. | > 0 | 10 min | Avvertenza (`critical`) | 24 h |
| Active Backup troppo vecchio | Nessuna esecuzione riuscita di Active Backup for Business per questo task da più di due giorni (`dumbmonit_abb_task_last_success_seconds`). La serie non esiste per un task che non è mai riuscito; allora parla "Task Active Backup fallito". | > 2 d | 1 h | Avviso (`warning`) | 24 h |
| Task Active Backup disabilitato | Il task non ha pianificazione, oppure il suo backup continuo è in pausa (`dumbmonit_abb_task_enabled == bool 0`): non salverà nulla finché qualcuno non lo esegue. | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Dispositivo Active Backup in ritardo | Un dispositivo resta senza un'esecuzione riuscita di Active Backup for Business più a lungo di quanto consenta il suo ritmo, esclusi i giorni di riposo (`dumbmonit_abb_device_overdue`, vedi [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 min | Avviso (`warning`) | 24 h |
| Dispositivo Active Backup in errore | Gli ultimi due o più tentativi di un dispositivo sono falliti (`dumbmonit_abb_device_consecutive_failures`); un'esecuzione annullata non conta in nessuno dei due casi. | ≥ 2 | 10 min | Avvertenza (`critical`) | 24 h |

### Server di log e metriche {#log-and-metrics-servers}

VictoriaMetrics, VictoriaLogs, Loki e Graylog ([monitor](../devices/victoriametrics.md)). I contatori passano da `increase_prometheus`, quindi un server aggiunto con uno storico di dati rifiutati non scatta alla sua prima misura.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Storage di metriche o log in sola lettura | Lo spazio libero su disco è sceso sotto `-storage.minFreeDiskSpaceBytes` e lo storage rifiuta ogni scrittura (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 min | Avvertenza (`critical`) | 1 h |
| Storage di metriche o log quasi in sola lettura | Resta meno del 10% di disco prima della sola lettura (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); rientra sopra il 12 %. | < 10 % | 30 min | Avviso (`warning`) | 24 h |
| Server di metriche o log non sano | `/health` non risponde OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 min | Avvertenza (`critical`) | 6 h |
| Dati di metriche o log rifiutati | Campioni o righe di log rifiutati nell'ultima ora: timestamp fuori dalla retention, etichette troppo lunghe, limite di serie (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| Inserimenti lenti in VictoriaMetrics | Più del 5% dei nuovi campioni ha preso il percorso lento in 15 minuti: memoria insufficiente per le serie attive (`(100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])) and on (target) dumbmonit_victoriametrics_uptime_seconds > 900`); rientra sotto il 3 %. Viene valutata solo dopo che il server è in funzione da 15 minuti, così un riavvio (cache fredda, contatori tornati a zero nella finestra) non la lascia in attesa. | > 5 % | 30 min | Avviso (`warning`) | 24 h |
| Loki non pronto | `/ready` non risponde ready (`dumbmonit_loki_ready`). | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Loki rifiuta righe di log | Righe rifiutate nell'ultima ora: troppo vecchie, limite di frequenza o di stream, riga troppo lunga (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| Loki non riesce a scaricare sullo storage | Dei chunk non sono arrivati allo storage negli ultimi 30 minuti; vanno persi se Loki si riavvia (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 min | Avvertenza (`critical`) | 1 h |
| Disco del write-ahead log di Loki pieno | Scritture del WAL perse a causa di un disco pieno (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 min | Avvertenza (`critical`) | 1 h |
| Errori nelle richieste a Loki | Più di cinque richieste hanno ricevuto un errore del server in 15 minuti (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 min | Avviso (`warning`) | 6 h |
| Graylog non elabora | L'elaborazione dei messaggi è in pausa sul nodo (`dumbmonit_graylog_processing`). | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Cluster di ricerca di Graylog fermo | OpenSearch o Elasticsearch rosso, o irraggiungibile da Graylog (`dumbmonit_graylog_indexer_status`: 2 rosso, 3 irraggiungibile). | ≥ 2 | 5 min | Avvertenza (`critical`) | 1 h |
| Cluster di ricerca di Graylog giallo | Shard di replica non assegnati da mezz'ora (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 min | Avviso (`warning`) | 24 h |
| Journal di Graylog in riempimento | Il journal è pieno per più della metà: l'output non tiene il passo (`dumbmonit_graylog_journal_used_percent`); rientra sotto il 40 %. | > 50 % | 10 min | Avviso (`warning`), escalation dopo 1 h | 6 h |
| Input di Graylog fallito | Un input non è riuscito ad avviarsi (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 min | Avviso (`warning`) | 24 h |
| Errori di indicizzazione in Graylog | Le scritture verso il cluster di ricerca o l'elaborazione dei messaggi sono fallite nell'ultima ora (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 min | Avviso (`warning`) | 24 h |

### Applicazioni self-hosted {#self-hosted-applications}

Nextcloud, Immich, Paperless-ngx, Jellyfin e Plex ([monitor](../devices/nextcloud.md)). Raramente cadono del tutto: la pagina si apre mentre sincronizzazione, miniature, importazioni o scansioni della libreria si sono fermate. Queste regole sorvegliano quegli stati; un'applicazione che non risponde affatto è "Monitor irraggiungibile".

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Nextcloud bloccato in manutenzione | La modalità manutenzione è attiva (`dumbmonit_nextcloud_maintenance`): nessuno può accedere o sincronizzare. | > 0 | 30 min | Avviso (`warning`), escalation dopo 3 h | 6 h |
| Aggiornamento del database di Nextcloud in sospeso | Il codice è stato aggiornato e Nextcloud attende `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 min | Avvertenza (`critical`) | 1 h |
| Aggiornamento di Nextcloud disponibile | Il controllo aggiornamenti di Nextcloud ha trovato una release più recente (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Informazioni (`info`) | 7 d |
| OPcache di Nextcloud piena | L'OPcache di PHP ha esaurito lo spazio e le pagine rallentano (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 min | Avviso (`warning`) | 24 h |
| Poco spazio libero su Nextcloud | Meno di 5 GiB liberi per la directory dei dati (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); rientra sopra i 6 GiB. | < 5 GiB | 30 min | Avviso (`warning`) | 24 h |
| Job di Immich bloccati | Dei job attendono in una coda che non è in pausa e non esegue nulla (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). La notifica riporta la coda. | > 0 | 1 h | Avviso (`warning`) | 6 h |
| Coda di Immich in pausa | Una coda di job è in pausa da un giorno (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Informazioni (`info`) | 7 d |
| Storage di Immich quasi pieno | Il disco che contiene la libreria è pieno per più del 90 % (`dumbmonit_immich_storage_used_percent`); rientra sotto l'88 %. | > 90 % | 30 min | Avviso (`warning`) | 24 h |
| Aggiornamento di Immich disponibile | Il controllo versione di Immich conosce una release più recente (`dumbmonit_immich_update_available`). | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Coda dei task di Paperless ferma | Redis o Celery in errore (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 è un errore): non viene importato nulla di nuovo. | ≥ 2 | 5 min | Avvertenza (`critical`) | 1 h |
| Errore di indice o di sanity check di Paperless | L'indice di ricerca, il classificatore o il sanity check (file mancanti o alterati) segnala un errore (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 min | Avviso (`warning`) | 24 h |
| Migrazioni di Paperless in sospeso | Restano migrazioni del database da applicare (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 min | Avviso (`warning`) | 24 h |
| Task di Paperless fallito | Un task visibile a questo utente è fallito entro la finestra dei task falliti e non è stato ignorato (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 min | Avviso (`warning`) | 24 h |
| Storage di Paperless quasi pieno | Il disco che contiene i documenti è pieno per più del 90 % (`dumbmonit_paperless_storage_used_percent`); rientra sotto l'88 %. | > 90 % | 30 min | Avviso (`warning`) | 24 h |
| Aggiornamento di Paperless disponibile | Il controllo versione di Paperless-ngx conosce una release più recente (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Task pianificato di Jellyfin fallito | Un task pianificato (una scansione della libreria, una pulizia) è fallito o è stato interrotto alla sua ultima esecuzione (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 min | Avviso (`warning`) | 24 h |
| Plugin di Jellyfin guasto | Un plugin non è riuscito a caricarsi o non supporta questa versione (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| Riavvio di Jellyfin in sospeso | Jellyfin attende da un giorno un riavvio dopo l'installazione o l'aggiornamento di un plugin (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Informazioni (`info`) | 7 d |
| Aggiornamento di Plex disponibile | Il controllo aggiornamenti di Plex ha trovato una release più recente (`dumbmonit_plex_update_available`). | > 0 | 1 h | Informazioni (`info`) | 7 d |

### Forge self-hosted {#self-hosted-forges}

GitLab (self-managed) e Forgejo/Gitea ([monitor](../devices/gitlab.md)). Un'istanza
che non risponde affatto è "Monitor irraggiungibile"; queste regole sorvegliano
le parti che continuano a servire pagine restando silenziosamente indietro — Sidekiq,
i runner CI/CD, un task pianificato che ha smesso di girare.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Tutti i runner GitLab offline | Ogni runner CI/CD noto a GitLab è offline, e almeno uno è registrato (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 min | Avviso (`warning`) | 6 h |
| Backlog di Sidekiq in GitLab alto | Una coda Sidekiq ha più di 500 job in attesa (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): notifiche, webhook e importazioni restano indietro. | > 500 | 15 min | Avviso (`warning`) | 6 h |
| Migrazioni di GitLab in sospeso | Restano da applicare migrazioni del database di un aggiornamento (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Informazioni (`info`) | 24 h |
| Licenza GitLab in scadenza | La licenza Enterprise Edition scade tra meno di 30 giorni (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 d | 1 h | Avviso (`warning`) | 24 h |
| Runner Forgejo/Gitea tutti offline | Ogni runner Actions è offline, e almeno uno è registrato (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 min | Avviso (`warning`) | 6 h |
| Task pianificato di Forgejo/Gitea in ritardo | Un task pianificato (controllo dei repository, pulizia, controllo aggiornamenti) non è stato eseguito al proprio intervallo (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 min | Avviso (`warning`) | 24 h |

### Dispositivi client {#client-devices}

Una regola, condivisa da ogni integrazione che tiene traccia dei dispositivi che si connettono e fanno backup — oggi i telefoni, tablet e desktop di Immich ([dispositivi](../devices/immich.md)), le macchine di cui Proxmox Backup Server fa il backup, una per gruppo di backup ([dispositivi](../devices/pbs.md)), e i computer e le VM protetti da Veeam, uno per oggetto con un punto di ripristino ([dispositivi](../devices/veeam.md)); altri seguiranno. Ogni integrazione calcola lo scarto rispetto alla finestra di obsolescenza del proprio target (`device_stale_days`, 3 giorni per impostazione predefinita) e lo pubblica come `dumbmonit_client_device_stale_seconds`, con le etichette `device`, `type`, `os`, `user`, `kind` e `signal` (`connection` o `backup`); questa regola scatta poi per ciascuno di essi, riportando il dispositivo nella notifica. Synology Active Backup for Business traccia i propri dispositivi separatamente, tramite un meccanismo che tiene conto del ritmo (`synology_abb_device_*`) anziché questa famiglia generica — vedi [Synology](../devices/synology.md).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Dispositivo client obsoleto | Un dispositivo non si è connesso o non ha fatto backup entro la finestra di obsolescenza configurata per questo target (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Avviso (`warning`) | 24 h |

### Pi-hole

[Pi-hole](../devices/pihole.md) v6, letto tramite la sua API REST.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Blocco di Pi-hole disabilitato | Il blocco è disattivato, in pausa o fallito (`dumbmonit_pihole_blocking_enabled`). Il mantenimento lascia passare una pausa di cinque minuti dall'interfaccia web. | < 1 | 15 min | Avviso (`warning`) | 6 h |
| Blocklist di Pi-hole obsolete | Le blocklist (gravity) non vengono ricostruite da più di otto giorni (`dumbmonit_pihole_gravity_age_seconds`): l'aggiornamento settimanale fallisce. | > 8 d | 1 h | Avviso (`warning`) | 24 h |
| Aggiornamento di Pi-hole disponibile | È disponibile una versione più recente (`dumbmonit_pihole_updates_available`): l'immagine in Docker, altrimenti core, interfaccia web o FTL. Pi-hole controlla una volta al giorno. | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Messaggi di diagnosi di Pi-hole | Pi-hole elenca messaggi nella sua pagina di diagnosi (`dumbmonit_pihole_messages`): una blocklist che non è stato possibile scaricare, un client limitato in frequenza, un host sovraccarico. | > 0 | 15 min | Informazioni (`info`) | 24 h |

### AdGuard Home

Il filtro DNS ([monitor](../devices/adguard.md)). Le cifre di query e blocchi sono somme sulla finestra statistica di AdGuard Home stesso, non contatori: nessuna regola si basa su di esse.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Protezione di AdGuard Home disattivata | Protezione disabilitata, oppure in pausa e non ancora ripristinata (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 min | Avviso (`warning`) | 6 h |
| DNS di AdGuard Home non in esecuzione | L'interfaccia web risponde ma il server DNS è fermo (`dumbmonit_adguard_running`). | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| Upstream di AdGuard Home in errore | Un server DNS upstream fallisce il test di AdGuard Home stesso, oppure resta muto per 4 secondi (`dumbmonit_adguard_upstream_up`, una serie per upstream). | < 1 | 10 min | Avviso (`warning`) | 6 h |
| Liste di filtri di AdGuard Home obsolete | L'aggiornamento più vecchio di una lista di filtri abilitata risale a più di tre giorni fa (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 giorni | 1 h | Avviso (`warning`) | 24 h |
| Aggiornamento di AdGuard Home disponibile | Il controllo versione di AdGuard Home stesso annuncia una versione più recente (`dumbmonit_adguard_update_available`); non scatta mai quando quel controllo è disabilitato, come nell'immagine Docker ufficiale. | > 0 | 1 h | Informazioni (`info`) | 7 giorni |

### UPS dietro NUT {#ups-behind-nut}

UPS letti da un [server NUT](../devices/nut.md). Ogni serie porta l'etichetta
`ups`, quindi un avviso riporta l'UPS quando un server ne pubblica diversi. I
flag di stato valgono 0 o 1 a ogni lettura, quindi una regola rientra da sola
quando l'alimentazione di rete torna. Gli UPS letti via SNMP sono coperti dalle
regole [UPS](#ups) qui sopra.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| UPS a batteria (NUT) | L'UPS alimenta il carico dalla batteria (`dumbmonit_nut_ups_on_battery`, il flag `OB` di `ups.status`). | > 0 | 30 s | Avvertenza (`critical`) | 15 min |
| Batteria UPS scarica (NUT) | L'UPS segnala una batteria scarica, il flag `LB` (`dumbmonit_nut_ups_low_battery`): le macchine che alimenta stanno per spegnersi. | > 0 | 1 min | Avvertenza (`critical`) | 30 min |
| Batteria UPS da sostituire (NUT) | L'UPS chiede la sostituzione della batteria, il flag `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Avviso (`warning`) | 7 d |
| Carico UPS elevato (NUT) | Il carico è rimasto sopra l'80 % (`dumbmonit_nut_ups_load_percent`); rientra sotto il 75 %. | > 80 % | 10 min | Avviso (`warning`) | 24 h |
| Autonomia UPS ridotta (NUT) | L'UPS stima meno di cinque minuti di autonomia a batteria (`dumbmonit_nut_battery_runtime_seconds`); rientra sopra i sei. Con alimentazione di rete, indica una batteria usurata o un UPS troppo piccolo per il suo carico. | < 300 s | 5 min | Avviso (`warning`) | 24 h |
| Dati UPS obsoleti (NUT) | Il server NUT risponde ma non ha dati recenti dall'UPS: cavo, driver o UPS muto (`dumbmonit_nut_ups_data_stale`). Nel frattempo un'interruzione passerebbe inosservata. | > 0 | 5 min | Avviso (`warning`), escalation dopo 1 h | 6 h |

### MikroTik RouterOS

Le serie portano l'etichetta `interface` o `sensor`, quindi una notifica riporta la porta o il sensore ([monitor](../devices/mikrotik.md)). I contatori passano da `increase_prometheus`: un router aggiunto con anni di errori alle spalle non scatta alla sua prima misura.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| CPU MikroTik elevata | La CPU del router è rimasta sopra il 90 % (`dumbmonit_mikrotik_cpu_load_percent`): l'instradamento, il firewall o uno script la saturano. Rientra sotto l'80 %. | > 90 % | 15 min | Avviso (`warning`) | 6 h |
| Memoria MikroTik elevata | Più del 90 % della memoria è in uso (`dumbmonit_mikrotik_memory_used_percent`). Rientra sotto l'85 %. | > 90 % | 15 min | Avviso (`warning`) | 6 h |
| Storage MikroTik quasi pieno | La memoria flash è piena per più del 90 % (`dumbmonit_mikrotik_storage_used_percent`): modifiche di configurazione e aggiornamenti possono fallire. Rientra sotto l'85 %. | > 90 % | 30 min | Avviso (`warning`) | 24 h |
| Temperatura MikroTik elevata | Un sensore di temperatura legge più di 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Rientra a 75 °C. | > 80 °C | 10 min | Avviso (`warning`) | 6 h |
| Alimentatore o ventola MikroTik guasti | Un sensore `psu*-state` o `fan*-state` non è `ok` (`dumbmonit_mikrotik_health_ok`). Un alimentatore ridondante che muore non interrompe nulla, finché non muore anche il secondo. | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| Interfaccia MikroTik con errori in accumulo | Più di cinquanta errori in ricezione e trasmissione su un'interfaccia nell'ultima ora (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 min | Avviso (`warning`) | 24 h |
| Link MikroTik instabile (flapping) | Un'interfaccia ha perso il link più di due volte in trenta minuti (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Una porta scollegata definitivamente conta una volta e non scatta. | > 2 | 5 min | Avviso (`warning`) | 1 h |
| Aggiornamento di MikroTik RouterOS disponibile | Un RouterOS più recente è disponibile sul canale di aggiornamento del router (`dumbmonit_mikrotik_update_available`). Noto solo quando il router controlla da sé gli aggiornamenti: vedi il passaggio facoltativo della [configurazione](../devices/mikrotik.md). | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Aggiornamento del firmware MikroTik in sospeso | RouterBOOT è più vecchio del firmware incluso nel RouterOS installato (`dumbmonit_mikrotik_firmware_upgrade_pending`); viene aggiornato solo su richiesta, poi al successivo riavvio. | > 0 | 1 h | Informazioni (`info`) | 7 d |

### UniFi Network

[Pagina del dispositivo](../devices/unifi.md). Le serie dei dispositivi portano nome, modello e tipo del dispositivo, quindi la notifica dice "Living room AP". Un dispositivo in attesa di adozione non ha la serie `device_up` e non conta mai come offline.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Dispositivo UniFi offline | Un dispositivo UniFi adottato dal controller è offline, isolato o non è riuscito ad essere adottato (`dumbmonit_unifi_device_up`). | < 1 | 5 min | Avviso (`warning`), escalation dopo 1 h | 6 h |
| Internet UniFi non disponibile | Il gateway UniFi non segnala alcuna connessione a Internet (`dumbmonit_unifi_internet_up`). Senza un gateway UniFi la serie non esiste e la regola resta silenziosa. | < 1 | 3 min | Avvertenza (`critical`) | 1 h |
| Collegamento WAN UniFi non attivo | Un collegamento WAN del gateway UniFi è giù (`dumbmonit_unifi_wan_link_up`): con due collegamenti, la ridondanza è persa mentre nulla sembra guasto. Una porta disabilitata non ha serie. | < 1 | 5 min | Avviso (`warning`) | 6 h |
| CPU del gateway UniFi elevata | Il processore del gateway UniFi è sopra il 90% da quindici minuti (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); rientra sotto l'80 %. | > 90 % | 15 min | Avviso (`warning`) | 6 h |
| Dispositivo UniFi in attesa di adozione | Un dispositivo è comparso sulla rete e attende di essere adottato in UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 min | Informazioni (`info`) | 24 h |
| Aggiornamento firmware UniFi disponibile | È disponibile un aggiornamento firmware per dispositivi UniFi (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Allarmi UniFi | UniFi ha registrato eventi critici o sollevato allarmi nelle ultime 24 ore (`dumbmonit_unifi_alarms`). Letto solo con un account View Only. | > 0 | 5 min | Avviso (`warning`) | 24 h |

### Home Assistant

[Pagina del monitor](../devices/homeassistant.md).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Home Assistant in modalità di ripristino | Home Assistant si è avviato in modalità di ripristino: la sua configurazione non è stata caricata e le automazioni non girano (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 min | Avvertenza (`critical`) | 1 h |
| Home Assistant non in esecuzione | Home Assistant risponde ma non ha finito di avviarsi da dieci minuti (`dumbmonit_homeassistant_running`). | < 1 | 10 min | Avviso (`warning`) | 6 h |
| Entità di Home Assistant diventate non disponibili | Almeno cinque entità in più non sono disponibili rispetto a un'ora fa: un'integrazione, un hub o una radio si è fermato (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). Conta l'aumento, non il totale, che in un'installazione reale non è mai a zero. | ≥ 5 | 10 min | Avviso (`warning`) | 6 h |
| Batteria scarica in Home Assistant | Una batteria segnalata da Home Assistant è scarica (`dumbmonit_homeassistant_battery_low`, una serie per batteria sotto la soglia del monitor, 20% per impostazione predefinita; il suo valore è il livello). | ≥ 0 % | 30 min | Informazioni (`info`) | 3 d |
| Aggiornamento di Home Assistant disponibile | Home Assistant ha aggiornamenti in attesa: core, sistema operativo, add-on o firmware dei dispositivi (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Informazioni (`info`) | 7 d |
| Riparazione da affrontare in Home Assistant | Home Assistant ha una riparazione aperta di gravità error o critical (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 min | Avviso (`warning`) | 24 h |

### VMware vSphere

[Pagina del monitor](../devices/vsphere.md). Le serie degli host portano `esxi`, quelle delle VM `vm`, quelle dei datastore `datastore`, quindi la notifica riporta l'oggetto.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Host vSphere non risponde | Un host vSphere non risponde a vCenter (`dumbmonit_vsphere_host_connection_state`: 2 non risponde). Un host disconnesso di proposito (1) non scatta. | ≥ 2 | 3 min | Avvertenza (`critical`) | 1 h |
| Salute dell'host vSphere rossa | vSphere segnala la salute complessiva di un host come rossa (`dumbmonit_vsphere_host_status == 2`). Grigio significa sconosciuto e non scatta. | > 0 | 5 min | Avvertenza (`critical`) | 1 h |
| Host vSphere lasciato in manutenzione | Un host vSphere è in modalità manutenzione da un giorno: le sue VM sono state spostate ed è stato forse dimenticato lì (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Informazioni (`info`) | 24 h |
| Datastore vSphere quasi pieno | Un datastore vSphere è pieno per il 90% o più (`dumbmonit_vsphere_datastore_used_percent`); rientra sotto l'85 %. | ≥ 90 % | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |
| Datastore vSphere inaccessibile | Un datastore vSphere è inaccessibile: le VM che vi sono memorizzate non possono funzionare (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| VMware Tools non in esecuzione in una VM | VMware Tools è installato ma non in esecuzione in una VM accesa: il guest potrebbe essere bloccato (`dumbmonit_vsphere_vm_tools_status == 2`). Le VM senza Tools non scattano. | > 0 | 30 min | Informazioni (`info`) | 24 h |
| Allarme rosso vSphere | vSphere ha sollevato un allarme rosso che nessuno ha riconosciuto (`dumbmonit_vsphere_alarm{status="red"}`, una serie per allarme, con il nome dell'allarme e dell'oggetto). | > 0 | 5 min | Avviso (`warning`) | 6 h |

### Database, message broker e motore di sicurezza {#databases-message-broker-and-security-engine}

Redis / Valkey, MongoDB, RabbitMQ e CrowdSec ([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md), [RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)). I contatori passano da `increase_prometheus` o `changes_prometheus`, quindi nulla scatta alla prima misura di un monitor.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Memoria di Redis vicina a maxmemory | Memoria usata sopra il 90 % di `maxmemory` (`dumbmonit_redis_memory_used_percent`); rientra sotto l'85 %. Nessuna serie senza `maxmemory`. | > 90 % | 10 min | Avviso (`warning`) | 6 h |
| Redis rifiuta connessioni | Connessioni client rifiutate negli ultimi 15 minuti: `maxclients` raggiunto (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 min | Avvertenza (`critical`) | 1 h |
| Collegamento di replica Redis interrotto | Una replica ha perso il collegamento con il primario (`dumbmonit_redis_master_link_up`). | < 1 | 2 min | Avvertenza (`critical`) | 1 h |
| Replica Redis in ritardo | Visto dal primario, una replica non conferma da più di 30 secondi (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 min | Avviso (`warning`) | 6 h |
| Snapshot di Redis fallito | L'ultimo `BGSAVE` è fallito; con `stop-writes-on-bgsave-error`, le scritture sono rifiutate (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| Scrittura dell'append-only file di Redis fallita | L'ultima scrittura AOF è fallita (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| Replica set MongoDB senza primario | Nessun membro è primario: nessuna scrittura è accettata (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 min | Avvertenza (`critical`) | 1 h |
| Membro MongoDB irraggiungibile | Un membro del replica set è irraggiungibile dagli altri (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| Ritardo di replica MongoDB | Un secondario è in ritardo di più di un minuto rispetto al primario (`dumbmonit_mongodb_replset_member_lag_seconds`); rientra sotto i 30 s. | > 60 s | 10 min | Avviso (`warning`) | 6 h |
| Connessioni MongoDB vicine al limite | Più dell'80 % delle connessioni disponibili è in uso (`dumbmonit_mongodb_connections_used_percent`); rientra sotto il 75 %. | > 80 % | 10 min | Avviso (`warning`) | 6 h |
| Cache MongoDB sotto pressione | Più del 20 % della cache WiredTiger è sporca (dirty) (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); rientra sotto il 15 %. | > 20 % | 10 min | Avviso (`warning`) | 6 h |
| Errori interni di MongoDB | Asserzioni regolari nell'ultima ora (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| Allarme di memoria o disco di RabbitMQ | Un nodo ha sollevato un allarme di risorse: ogni publisher è bloccato (`dumbmonit_rabbitmq_alarm`, una serie per nodo e risorsa). | > 0 | 1 min | Avvertenza (`critical`) | 1 h |
| Nodo RabbitMQ fermo | Un nodo del cluster non è in esecuzione (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 min | Avvertenza (`critical`) | 1 h |
| Partizione di rete RabbitMQ | Un nodo vede una partizione di rete (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 min | Avvertenza (`critical`) | 1 h |
| Coda RabbitMQ senza consumer | Una coda contiene messaggi e non ha consumer (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); il valore è il numero di messaggi in attesa. | > 0 | 15 min | Avviso (`warning`) | 6 h |
| Coda RabbitMQ non disponibile | Una coda non è in esecuzione: quorum queue senza maggioranza, oppure il suo nodo è giù (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 min | Avviso (`warning`) | 6 h |
| Local API di CrowdSec non attiva | La Local API non risponde a `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| Bouncer CrowdSec ha smesso di interrogare | Le richieste di un bouncer non sono cambiate in 30 minuti (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 min | Avviso (`warning`) | 6 h |
| CrowdSec non legge log | Nemmeno una riga di log letta in sei ore (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 min | Avviso (`warning`) | 24 h |

### Reverse proxy e domini {#reverse-proxies-and-domains}

Traefik, Caddy, Nginx Proxy Manager e registrazioni di domini ([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md), [Nginx Proxy Manager](../devices/npm.md), [Scadenza dei domini](../devices/domain.md)), e l'opzione *Avvisa quando la risposta cambia* del controllo DNS ([Servizi](../devices/services.md#alerting-when-the-answer-changes)). Le serie portano `router`, `service`, `server`, `upstream`, `proxy_host` o `cert`, quindi la notifica riporta ciò che è fallito.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Router Traefik disabilitato | Un router è disabilitato da un errore di configurazione (servizio o middleware mancante, regola non valida): i suoi siti non sono serviti (`dumbmonit_traefik_router_status`: 0 abilitato, 1 avviso, 2 disabilitato). | ≥ 2 | 5 min | Avviso (`warning`) | 6 h |
| Server backend Traefik non attivo | Un server di un servizio con bilanciamento del carico fallisce il suo health check (`dumbmonit_traefik_server_up`). | < 1 | 5 min | Avviso (`warning`) | 6 h |
| Servizio Traefik senza server | Nessun server di un servizio risponde al proprio health check: i suoi siti ricevono 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 min | Avvertenza (`critical`) | 1 h |
| Certificato Traefik non ottenuto | I router chiedono a un certificate resolver un certificato che nessun certificato detenuto da Traefik copre ancora: Traefik serve il proprio certificato autofirmato predefinito (`dumbmonit_traefik_resolver_routers_uncovered`, richiede `/api/certificates` di Traefik). | > 0 | 1 h | Avviso (`warning`) | 24 h |
| Upstream Caddy non sano | Un backend fallisce l'health check attivo o passivo di Caddy (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 min | Avviso (`warning`) | 6 h |
| Ricaricamento della configurazione di Caddy fallito | Caddy ha rifiutato l'ultimo ricaricamento della configurazione e continua a eseguire la precedente (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 min | Avviso (`warning`) | 24 h |
| Host Nginx Proxy Manager offline | nginx ha rifiutato la configurazione di un host abilitato: l'host non è servito (`dumbmonit_npm_host_online`). | < 1 | 10 min | Avviso (`warning`) | 6 h |
| Certificato del proxy non rinnovato | Un certificato detenuto da Traefik o presentato da un host Nginx Proxy Manager scade tra meno di 14 giorni: il rinnovo automatico inizia a 30, quindi sta fallendo (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 d | 1 h | Avviso (`warning`) | 24 h |
| Reverse proxy che risponde 5xx | Più del 5 % delle risposte in dieci minuti era 5xx, con almeno cinquanta richieste (`increase_prometheus` di `dumbmonit_{traefik,caddy}_requests_5xx_total` su `…_requests_total`). Richiede le metriche del proxy. | > 5 % | 10 min | Avviso (`warning`) | 6 h |
| Dominio in scadenza | Il dominio scade tra meno di 30 giorni (`dumbmonit_domain_expiry_days >= 7`). | < 30 d | 1 h | Avviso (`warning`) | 24 h |
| Dominio in scadenza imminente | Il dominio scade tra meno di 7 giorni, o è scaduto (`dumbmonit_domain_expiry_days`). | < 7 d | 1 h | Avvertenza (`critical`) | 12 h |
| Dominio in hold | Il registro ha messo il dominio in `clientHold` o `serverHold`: è fuori dal DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 min | Avvertenza (`critical`) | 6 h |
| Dominio in redemption | Il dominio è in `redemptionPeriod`, `pendingDelete` o `pendingRestore`: è scaduto (`dumbmonit_domain_redemption`). | > 0 | 30 min | Avvertenza (`critical`) | 12 h |
| Risposta DNS cambiata | La risposta di un controllo DNS con *Avvisa quando la risposta cambia* differisce da quella precedente (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Si risolve da sola un'ora dopo. | > 0 | 1 min | Avviso (`warning`) | 24 h |

### Modifiche ai siti web {#website-changes}

Vedi [Modifiche ai siti web](../devices/webchange.md). La pagina del monitor mostra ogni modifica prima e dopo.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Sito web modificato | Un controllo di una pagina o di un sito sorvegliato ha trovato pagine modificate, comparse o scomparse (`dumbmonit_webchange_last_check_changes`). Ogni controllo di questo tipo viene notificato e nomina le pagine, sotto lo stesso avviso finché è attivo; si risolve al primo controllo che non trova nulla di nuovo. | > 0 | 30 s | Informazioni (`info`) | nessuno |

### Kubernetes

Vedi [Kubernetes / k3s](../devices/kubernetes.md). Ogni serie ha come etichetta
il nodo, oppure il namespace e il pod, il workload o il claim interessato.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Nodo Kubernetes non pronto | Un nodo non è Ready: il suo kubelet non riporta più nulla e i suoi pod verranno rimossi (evicted) (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| Nodo Kubernetes sotto pressione | Un nodo segnala pressione di memoria, disco o PID (`dumbmonit_k8s_node_pressure`, etichetta `condition`). | > 0 | 5 min | Avviso (`warning`) | 6 h |
| Pod Kubernetes in crash loop | Un container è in `CrashLoopBackOff` e continua a riavviarsi: almeno due riavvii in 15 minuti (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) and on (target, namespace, pod) increase_prometheus(dumbmonit_k8s_pod_restarts[15m]) >= 2`; lo stato scompare per un istante a ogni tentativo di riavvio). I container terminati da un riavvio del nodo o del runtime (exit 137, 143 o 255, non `OOMKilled`) non contano, così un reboot non sembra un crash loop. | > 0 | 10 min | Avvertenza (`critical`) | 6 h |
| Pod Kubernetes bloccato in pending | Un pod è Pending da quindici minuti: nessun nodo può accoglierlo, oppure un volume o un'immagine di cui ha bisogno non arriva (`dumbmonit_k8s_pod_pending`). | > 0 | 15 min | Avviso (`warning`) | 6 h |
| Pod Kubernetes in riavvio continuo | Un pod si è riavviato più di cinque volte nell'ultima ora senza essere in `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 min | Avviso (`warning`) | 6 h |
| Workload Kubernetes con repliche mancanti | Un Deployment, StatefulSet o DaemonSet ha meno repliche disponibili di quante ne richieda (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 min | Avviso (`warning`), escalation dopo 1 h | 6 h |
| Volume claim Kubernetes in pending | Un PersistentVolumeClaim di cui un pod ha bisogno, o che riguarda un errore di provisioning, non è ancora associato (bound) (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 min | Avviso (`warning`) | 6 h |

### Active Directory

Vedi [Active Directory](../devices/activedirectory.md). Il domain controller che
DumbMonit interroga è coperto da *Monitor irraggiungibile*; gli altri domain
controller del dominio da *Domain controller irraggiungibile* (etichetta `dc`).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Domain controller irraggiungibile | Un domain controller del dominio non risponde più sulla sua porta LDAP (`dumbmonit_ad_dc_reachable == bool 0`). Un nome che DumbMonit non riesce a risolvere non produce serie e non scatta mai. | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| Bind ad Active Directory fallito | Il domain controller risponde ma rifiuta l'account di servizio: password scaduta, account disabilitato o bloccato (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 min | Avviso (`warning`) | 24 h |
| Appartenenza a un gruppo privilegiato modificata | Un membro è entrato o uscito da Domain Admins, Enterprise Admins, Schema Admins, Administrators o da un gruppo di operatori, annidamenti inclusi (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, etichetta `group`). Si risolve da sola un'ora dopo. | > 0 | 1 min | Avviso (`warning`) | 24 h |
| Replica di Active Directory in errore | Il domain controller interrogato non riesce a replicare da un partner (`dumbmonit_ad_replication_consecutive_failures`). Solo quando l'account può leggere lo stato di replica. | > 0 | 30 min | Avviso (`warning`) | 6 h |
| Password krbtgt più vecchia di 180 giorni | La password krbtgt è stata cambiata l'ultima volta più di 180 giorni fa (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 giorni | 10 min | Informazioni (`info`) | nessuno |

### pfSense

Vedi [pfSense](../devices/pfsense.md). I gateway hanno l'etichetta `gateway`, le interfacce `interface` e `descr`, i servizi `service`.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Gateway pfSense non attivo | Un gateway pfSense è giù: il suo indirizzo di monitoraggio non risponde più e il traffico che trasporta si ferma o passa a un altro percorso (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 min | Avvertenza (`critical`) | 6 h |
| Gateway pfSense con perdita di pacchetti | Un gateway pfSense perde più del 10 % dei probe verso il suo indirizzo di monitoraggio: la linea è degradata (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 min | Avviso (`warning`) | 6 h |
| Gateway pfSense lento | Il tempo di andata e ritorno verso l'indirizzo di monitoraggio di un gateway pfSense è rimasto sopra i 500 ms per quindici minuti (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 min | Avviso (`warning`) | 6 h |
| Interfaccia pfSense senza link | Un'interfaccia pfSense abilitata non ha link: cavo, porta dello switch o scheda di rete (`dumbmonit_pfsense_interface_down`). | > 0 | 5 min | Avviso (`warning`) | 6 h |
| Servizio pfSense fermo | Un servizio pfSense abilitato non è in esecuzione: resolver DNS, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 min | Avviso (`warning`) | 6 h |
| Disco pfSense quasi pieno | Il disco di pfSense è pieno per più del 90 %: log, report e aggiornamenti di pacchetti smettono di essere scritti (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 min | Avviso (`warning`) | 1 d |

### Unraid

Vedi [Unraid](../devices/unraid.md). I dischi hanno le etichette `disk` e `role` (`parity`, `data`, `cache`), i container `container`.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Array Unraid fermo | L'array Unraid non è avviato: condivisioni, container e VM che vi risiedono non sono disponibili (`dumbmonit_unraid_array_started`). | < 1 | 10 min | Avvertenza (`critical`) | 6 h |
| Disco Unraid disabilitato o mancante | Un disco Unraid è disabilitato, mancante o non valido: l'array funziona degradato, e un altro guasto fa perdere dati (`dumbmonit_unraid_disk_ok`). | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Errori di lettura su disco Unraid | Un disco Unraid ha segnalato errori di lettura dall'avvio dell'array: potrebbe essere disabilitato al successivo (`dumbmonit_unraid_disk_errors`). | > 0 | 5 min | Avviso (`warning`) | 1 d |
| Disco Unraid troppo caldo | Un disco dell'array o di parità Unraid è rimasto sopra i 55 °C per quindici minuti (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 min | Avviso (`warning`) | 6 h |
| Il controllo di parità Unraid ha trovato errori | L'ultimo controllo di parità Unraid è fallito o ha trovato errori di sincronizzazione: la parità non corrisponde più ai dati (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 min | Avviso (`warning`) | 1 d |
| Controllo di parità Unraid in ritardo | L'ultimo controllo di parità Unraid risale a più di 40 giorni fa: un controllo mensile è stato saltato (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 d | 1 h | Avviso (`warning`) | 1 d |
| Cache Unraid quasi piena | Un pool di cache Unraid è pieno per più del 90 %: le scritture ripiegano sull'array lento, o falliscono (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 min | Avviso (`warning`) | 1 d |
| Container Unraid fermo | Un container Docker impostato per avviarsi con l'array non è in esecuzione (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 min | Avviso (`warning`) | 6 h |

### Veeam Backup & Replication

Vedi [Veeam Backup & Replication](../devices/veeam.md). I job hanno le etichette `job` e `type`, i repository `repository`.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Job Veeam fallito | L'ultima esecuzione di un job Veeam abilitato è fallita: ciò che protegge non ha un nuovo punto di ripristino (`dumbmonit_veeam_job_failed`). | > 0 | 5 min | Avvertenza (`critical`) | 1 d |
| Job Veeam terminato con un avviso | L'ultima esecuzione di un job Veeam abilitato è terminata con un avviso: alcuni oggetti potrebbero non essere protetti (`dumbmonit_veeam_job_warning`). | > 0 | 5 min | Avviso (`warning`) | 1 d |
| Repository Veeam quasi pieno | Un repository di backup Veeam è pieno per più del 90 %: i prossimi job falliranno per mancanza di spazio (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 min | Avviso (`warning`) | 1 d |
| Licenza Veeam in scadenza | La licenza Veeam scade tra meno di 30 giorni: i job si fermano alla fine del periodo di tolleranza (`dumbmonit_veeam_license_expiry_seconds`). | < 30 d | 1 h | Avviso (`warning`) | 1 d |

### Tailscale

Vedi [Tailscale](../devices/tailscale.md). I dispositivi hanno l'etichetta `device`, la prima etichetta del loro nome MagicDNS.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Dispositivo Tailscale offline | Un dispositivo Tailscale che deve restare online (per impostazione predefinita un dispositivo con tag) è disconnesso da più di 15 minuti (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 min | 5 min | Avviso (`warning`) | 6 h |
| Chiave del nodo Tailscale in scadenza | La chiave del nodo di un dispositivo Tailscale scade tra meno di 14 giorni: uscirà dalla tailnet finché qualcuno non lo fa accedere di nuovo (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 d | 1 h | Avviso (`warning`) | 1 d |
| Dispositivo Tailscale in attesa di approvazione | Un dispositivo è entrato nella tailnet e attende approvazione: approvalo se è previsto, rimuovilo altrimenti (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 min | Avviso (`warning`) | 1 d |
| Aggiornamento del client Tailscale | Un dispositivo Tailscale che deve restare online esegue un client per il quale è disponibile un aggiornamento (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Informazioni (`info`) | 7 d |

### FortiGate

Vedi [FortiGate](../devices/fortigate.md). I tunnel hanno l'etichetta `tunnel`, le interfacce `interface` e `alias`, le licenze `license`.

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Tunnel IPsec FortiGate non attivo | Nessuna fase 2 di un tunnel IPsec site-to-site FortiGate è attiva: il sito dietro di esso è isolato (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 min | Avvertenza (`critical`) | 6 h |
| Interfaccia FortiGate senza link | Un'interfaccia FortiGate abilitata con un indirizzo non ha link: cavo, transceiver o il dispositivo a monte (`dumbmonit_fortigate_interface_down`). | > 0 | 5 min | Avviso (`warning`) | 6 h |
| HA FortiGate non sincronizzato | I membri di un cluster HA FortiGate non hanno più lo stesso checksum di configurazione: un failover eseguirebbe un'altra configurazione (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 min | Avviso (`warning`) | 6 h |
| Membro HA FortiGate perso | Un cluster HA FortiGate ha meno membri che nelle ultime 24 ore: non c'è più uno standby (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| Memoria FortiGate vicina alla conserve mode | La memoria del FortiGate è rimasta sopra l'85 % per dieci minuti: all'88 % (per impostazione predefinita) entra in conserve mode e smette di ispezionare le nuove sessioni (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 min | Avviso (`warning`) | 6 h |
| CPU FortiGate elevata | La CPU del FortiGate è rimasta sopra il 90 % per quindici minuti (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 min | Avviso (`warning`) | 6 h |
| Licenza FortiGate in scadenza | Una licenza FortiGuard o FortiCare del FortiGate scade tra meno di 30 giorni, o è scaduta (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 d | 1 h | Avviso (`warning`) | 1 d |
| Aggiornamento firmware FortiGate | FortiGuard propone una patch release più recente del ramo FortiOS che il FortiGate esegue (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Informazioni (`info`) | 7 d |

### Sophos Firewall

Vedi [Sophos Firewall](../devices/sophos.md).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Interfaccia Sophos senza link | Un'interfaccia Sophos Firewall accesa e associata a una zona non ha link (`dumbmonit_sophos_interface_down`). | > 0 | 5 min | Avviso (`warning`) | 6 h |

### KVM (libvirt)

Segnalato dall'agent Linux su un host KVM; vedi [Agent](../devices/agent.md#kvm-virtual-machines-through-libvirt).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Dominio KVM in crash | libvirt segnala un dominio KVM come in crash: l'hypervisor ha dovuto fermarlo (`dumbmonit_agent_libvirt_domain_crashed`). | > 0 | 1 min | Avvertenza (`critical`) | 1 h |
| Pool libvirt quasi pieno | L'allocazione di un pool di storage libvirt raggiunge il 90% della sua capacità (`dumbmonit_agent_libvirt_pool_used_percent`). | ≥ 90 % (rientra sotto l'88 %) | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |

### XCP-ng / XenServer

Vedi [XCP-ng / XenServer](../devices/xcpng.md).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Host XCP-ng offline | Un host del pool XCP-ng non risponde più al coordinatore (`dumbmonit_xcpng_host_live`). | < 1 | 5 min | Avvertenza (`critical`) | 1 h |
| SR XCP-ng quasi pieno | Uno storage repository XCP-ng è pieno oltre il 90% (`dumbmonit_xcpng_sr_used_percent`). | ≥ 90 % (rientra sotto l'88 %) | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |
| VM XCP-ng protetta da HA ferma | Una VM protetta dall'alta disponibilità del pool è ferma (`dumbmonit_xcpng_vm_ha_protected_down`). | > 0 | 2 min | Avvertenza (`critical`) | 1 h |
| HA del pool XCP-ng in overcommit | Il pool non ha più un piano per sopravvivere ai guasti di host che è configurato per tollerare (`dumbmonit_xcpng_pool_ha_overcommitted`). | > 0 | 5 min | Avviso (`warning`), escalation dopo 1 h | 6 h |

### Nutanix

Vedi [Nutanix AHV](../devices/nutanix.md).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Host Nutanix degradato | Prism segnala un host Nutanix come degradato (`dumbmonit_nutanix_host_degraded`). | > 0 | 5 min | Avvertenza (`critical`) | 1 h |
| Storage container Nutanix quasi pieno | Uno storage container Nutanix è pieno oltre il 90% (`dumbmonit_nutanix_storage_container_used_percent`). | ≥ 90 % (rientra sotto l'88 %) | 15 min | Avviso (`warning`), escalation dopo 24 h | 6 h |
| Avvisi critici Nutanix | Prism Central segnala uno o più avvisi critici non risolti (`dumbmonit_nutanix_alerts_critical`). | > 0 | 5 min | Avvertenza (`critical`) | 1 h |

### Hyper-V

Segnalati dall'agent Windows su un host Hyper-V; vedi [Agent](../devices/agent.md#hyper-v-hosts).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Salute VM Hyper-V critica | Hyper-V segnala una macchina virtuale in stato di salute critico: una risorsa di cui ha bisogno, più spesso lo spazio su disco, è esaurita (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 min | Avvertenza (`critical`) | 6 h |
| CPU dell'host Hyper-V elevata | I processori logici dell'host Hyper-V sono occupati per più del 90 % da quindici minuti, guest inclusi (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 min | Avviso (`warning`) | 6 h |
| Errori sul disco virtuale Hyper-V | Un disco virtuale Hyper-V ha segnalato errori di I/O nell'ultima ora (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 min | Avviso (`warning`) | 6 h |

### Punteggio di sicurezza {#security-score}

Calcolato dal server ogni 15 minuti per ogni monitor il cui tipo prevede controlli di sicurezza, e memorizzato come `dumbmonit_security_score` (0–100), solo finché il punteggio di sicurezza è attivo nelle Impostazioni (disattivato per impostazione predefinita). Vedi [Punteggio di sicurezza](../using/security-score.md).

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Punteggio di sicurezza calato | Il punteggio è sceso di 10 punti o più rispetto al suo valore migliore delle ultime 24 ore (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): un controllo ha iniziato a fallire. | ≥ 10 pt | 30 min | Avviso (`warning`) | 24 h |
| Punteggio di sicurezza basso | Il punteggio è sotto 40, voto F (`dumbmonit_security_score`). Alza o abbassa la soglia in base al tuo livello di esigenza. | < 40 | 1 h | Informazioni (`info`) | 7 d |

### DumbMonit stesso

| Regola | Cosa | Soglia predefinita | Mantenimento | Gravità | Promemoria |
|---|---|---|---|---|---|
| Backup di DumbMonit non eseguito | Il backup locale pianificato del database di DumbMonit non viene eseguito da più di due giorni (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 d | 1 h | Avviso (`warning`) | 24 h |
| Volume dati di DumbMonit quasi pieno | Il volume che contiene `/data` ha meno del 10% di spazio libero (`100 * dumbmonit_instance_data_free_bytes / dumbmonit_instance_data_size_bytes`). | < 10 % | 15 min | Avviso (`warning`) | 24 h |
| Database delle metriche di DumbMonit in riavvio continuo | Il VictoriaMetrics integrato è stato riavviato tre o più volte in 30 minuti (`increase_prometheus(dumbmonit_instance_vm_restarts_total[30m])`). | ≥ 3 | 1 min | Avvertenza (`critical`) | 6 h |
| Pianificatore di DumbMonit bloccato | Nessun giro di pianificazione da più di due minuti: i monitor non vengono più interrogati (`dumbmonit_instance_scheduler_lag_seconds`). | > 120 s | 5 min | Avvertenza (`critical`) | 6 h |
| Interrogazioni di DumbMonit in ritardo | Alcuni monitor restano in ritardo dopo ogni giro (`min_over_time(dumbmonit_instance_scheduler_backlog[15m])`). | > 0 | 15 min | Avviso (`warning`) | 24 h |
| Scritture delle metriche di DumbMonit in errore | Lotti rifiutati da VictoriaMetrics o misure scartate negli ultimi 15 minuti (`dumbmonit_instance_sample_writes_failed_total`, `dumbmonit_instance_samples_dropped_total`). | > 0 | 5 min | Avviso (`warning`) | 6 h |
| Database di DumbMonit in crescita rapida | `dumbmonit.db` è più che raddoppiato in un giorno e supera i 100 MB (`dumbmonit_instance_database_bytes`). | > 2× | 1 h | Avviso (`warning`) | 24 h |

Queste regole e l'heartbeat in uscita sono spiegati in [Watching DumbMonit itself](../install/self-monitoring.md).

Ogni regola integrata si applica a tutti i monitor e a ogni canale abilitato.

## Modificare una regola nell'interfaccia

Nella pagina **Avvisi**, la sezione *Regole* elenca ogni regola con la sua
gravità, un indicatore *Integrata* e, quando pertinente, quanti avvisi sta
generando in questo momento. Ogni regola ha:

- un interruttore per **abilitarla o disabilitarla**;
- **Modifica**, che apre un editor in linea per nome, descrizione, operatore,
  soglia, mantenimento (`for`), gravità, promemoria (`repeat every`) ed
  escalation (`escalate after`). La query di una regola integrata è mostrata ma
  non modificabile; quella delle tue regole lo è. Le regole baseline mostrano i
  loro parametri di rilevamento in sola lettura;
- **Elimina**, solo per le tue regole.

## Override per monitor {#per-device-overrides}

Una regola può essere regolata per un solo monitor senza toccare la regola
stessa: una soglia diversa, una soglia di rientro diversa, oppure disattivata
per quel solo monitor. L'editor di ogni regola ha un pannello **Override per
monitor** per aggiungerne o rimuoverne uno direttamente.

L'azione **Ignora** con un clic su un avviso (nella scheda *Now* o nella pagina del monitor dell'avviso) è lo stesso meccanismo,
scritto con `enabled: false`: il monitor smette subito di applicare quella
regola, in modo reversibile. La pagina del monitor elenca ciò che ignora,
con **Smetti di ignorare**; la regola stessa mostra *Ignorata su N monitor*
in Avvisi → Regole, dove lo stesso pannello degli override per monitor la
rimuove. Vedi [Override per monitor](../reference/api.md#per-device-overrides)
per l'API.

## Creare una regola a soglia

Fai clic su **Nuova regola**. Il modulo richiede:

| Campo | Significato |
|---|---|
| Nome | Mostrato negli avvisi e nelle notifiche. |
| Query | Un'espressione MetricsQL, valutata per monitor. Esempio: `dumbmonit_cpu_usage_percent`. |
| Operatore | `>`, `>=`, `<` o `<=`. |
| Soglia | Il numero con cui viene confrontato il risultato della query. |
| Gravità | Informazioni, Avviso o Avvertenza. |
| Mantieni per | Secondi per cui la condizione deve durare prima di scattare. Un mantenimento diverso da zero assorbe un singolo campione anomalo. |

Ogni serie restituita dalla query diventa un potenziale avviso, associato al
monitor indicato dalla sua etichetta `target`. Aggrega con
`avg by (target, host) (…)` quando vuoi un avviso per monitor anziché uno per
core o per interfaccia. L'identità dell'avviso ignora le etichette `host` e
`tag_*` quando `target` è presente, quindi rinominare un monitor conserva i
suoi avvisi; una serie il cui `target` non corrisponde a un monitor
esistente e abilitato viene ignorata.

## Il linguaggio delle query

Le query sono in [MetricsQL](https://docs.victoriametrics.com/metricsql/), il
superset di PromQL di VictoriaMetrics. Le metriche sono le serie `dumbmonit_*`
descritte nel [riferimento delle metriche](../reference/metrics.md), e ogni serie
porta un'etichetta `target` (l'id del monitor) e un'etichetta `host` (il suo
nome).

Alcuni schemi usati dalle regole integrate:

```
# Age of the last sample, freshest series per device: what "unreachable" means
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))

# One value per device, whatever the source
avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)

# A percentage computed from two gauges
100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total

# A boolean condition: the series is returned only when true
dumbmonit_ups_output_source == 5

# Extrapolation done by VictoriaMetrics
predict_linear(dumbmonit_pbs_datastore_used_percent[6h], 345600) and deriv(dumbmonit_pbs_datastore_used_percent[6h]) > 0

# Availability of a service over thirty days
avg_over_time(dumbmonit_probe_success[30d])
```

!!! tip "Prova prima una query"
    `GET /api/metrics/query?query=…` restituisce ciò che la regola valuterebbe
    (vedi il [riferimento API](../reference/api.md#metrics)). In sviluppo,
    l'overlay pubblica la VictoriaMetrics incorporata sulla porta `:8428`
    dell'host, con la sua interfaccia di query.

## Tipi di regola nell'API

`kind` è `threshold` (confronto con una soglia), `predict` (stesso confronto,
su una query `predict_linear`; mostrata come previsione) o `anomaly` (baseline
stagionale; `params` contiene `k`, `alpha`, `mad_floor_abs`, `mad_floor_rel`,
`min_samples`). L'interfaccia crea regole `threshold`; gli altri due tipi possono
essere creati tramite l'API.
