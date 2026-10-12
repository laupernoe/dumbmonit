# Regeln

Regeln werden beim ersten Start eingefügt und gehören danach Ihnen: DumbMonit
überschreibt nie eine von Ihnen geänderte Regel, es erstellt nur fehlende neu.
Eingebaute Regeln können bearbeitet und deaktiviert, aber nicht gelöscht
werden.

## Eingebaute Regeln {#built-in-rules}

Schweregrade werden mit dem UI-Wort angezeigt; der API-Wert steht in
Klammern. Eine Regel, für deren Metrik kein Gerät Daten liefert, hat keine
Zeitreihe und feuert nie, daher wird jede der unten stehenden Regeln auf
jeder Instanz aktiviert ausgeliefert.

### Jedes Gerät

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Device unreachable | Seit mehr als drei Minuten keine Messung empfangen. `time() - tlast_over_time(dumbmonit_up[7d])` | > 180 s | 1 Min | Warning (`critical`) | 6 h |
| High CPU | CPU-Last anhaltend über 90 %, gemittelt pro Gerät über SNMP-, Proxmox- und Agent-Quellen; löst sich unter 85 %. | > 90 % | 10 Min | Advisory (`warning`), eskaliert nach 1 h | 6 h |
| Unusual CPU | CPU-Last deutlich anders als üblich zu dieser Zeit und diesem Wochentag (saisonale Baseline). | Score > 3,5 | 15 Min | Info (`info`) | 6 h |
| Disk almost full | Dateisystem zu 90 % oder mehr voll, pro Einhängepunkt (SNMP, Proxmox-Storages und Root-Dateisysteme); löst sich unter 88 %. | ≥ 90 % | 15 Min | Advisory (`warning`), eskaliert nach 24 h | 6 h |
| Filesystem almost full | Mit dieser Rate wird das Dateisystem innerhalb von vier Tagen voll sein (`predict_linear` über 6 h; nur bei steigenden Dateisystemen). | ≥ 100 % | 30 Min | Advisory (`warning`) | 6 h |

### USV

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| UPS on battery | Die USV versorgt die Last aus dem Akku (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Warning (`critical`) | 15 Min |
| UPS battery low | Der USV-Akku ist schwach oder erschöpft (`dumbmonit_ups_battery_status`, 3 = schwach, 4 = erschöpft). | ≥ 3 | 1 Min | Warning (`critical`) | 30 Min |

### Netzwerkports (SNMP)

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Port accumulating errors | Ein Netzwerkport hat in der letzten Stunde mehr als fünfzig Fehler verzeichnet: meist ein defektes Kabel, eine verschmutzte Glasfaser oder ein Duplex-Mismatch (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; in der ersten Stunde eines neuen Geräts feuert nichts, damit vor der Aufnahme gezählte Fehler nicht als neu gemeldet werden). | > 50 | 15 Min | Advisory (`warning`) | 24 h |
| Port flapping | Ein Netzwerkport ist in dreißig Minuten mehr als zweimal ausgefallen und wieder hochgekommen (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Ein absichtlich deaktivierter Port hat keine Zeitreihe und feuert nie. | > 4 | 5 Min | Advisory (`warning`) | 1 h |

### Server-Hardware (Redfish)

Health-Zeitreihen sind 0 (OK), 1 (Warning) oder 2 (Critical). Ein leerer Steckplatz (`Absent`) oder ein deaktivierter hat keine Zeitreihe, daher feuert ein mit nur einem von zwei Netzteilen ausgelieferter Server nie.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Server fan failed | Der Management-Controller meldet einen Lüfter als ausgefallen (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 Min | Warning (`critical`) | 6 h |
| Server temperature above critical | Ein Temperatursensor liegt über dem vom Controller selbst angegebenen kritischen Schwellenwert (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 Min | Warning (`critical`) | 6 h |
| Power supply redundancy lost | Die Netzteile sind nicht mehr redundant: ein weiterer Ausfall legt den Server lahm (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 Min | Advisory (`warning`), eskaliert nach 1 h | 6 h |
| Power supply failed | Der Controller meldet ein Netzteil als ausgefallen (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 Min | Warning (`critical`) | 6 h |
| Drive failure predicted | Ein Laufwerk meldet einen vorhergesagten Ausfall: ersetzen Sie es, während es noch funktioniert (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 Min | Advisory (`warning`) | 24 h |
| Server health critical | Der Controller meldet die eigene Systemgesundheit als Critical (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 Min | Warning (`critical`) | 6 h |

### Uptime-Monitore und Heartbeats

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Service down | Der Dienst hat drei Minuten lang nicht korrekt geantwortet (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 Min | Warning (`critical`) | 30 Min |
| Heartbeat missed | Der Job hat sich nicht innerhalb seines erwarteten Intervalls plus Kulanzzeit gemeldet, oder selbst einen Fehler gemeldet (`dumbmonit_probe_success{probe="push"} == bool 0`; die Verzögerung wird pro Gerät eingestellt, siehe [Heartbeat](../devices/push.md)). | > 0 | 2 Min | Advisory (`warning`) | 6 h |
| Service flapping | Der Dienst hat in dreißig Minuten mehr als sechsmal den Zustand gewechselt (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 Min | Advisory (`warning`) | 1 h |
| Slow service | Der Dienst braucht mehr als drei Sekunden zum Antworten. | > 3 s | 10 Min | Advisory (`warning`) | 6 h |
| Certificate expiring soon | Das Zertifikat läuft in weniger als vierzehn Tagen ab. | < 14 T | 1 h | Advisory (`warning`) | 24 h |
| Certificate expired | Das Zertifikat ist abgelaufen. | < 0 T | 5 Min | Warning (`critical`) | 24 h |

### Docker-Container

Gemeldet vom Agent; siehe [Agent](../devices/agent.md). Der Alarm benennt den
Container.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Container stopped | Der Container ist seit zwei Minuten gestoppt (`dumbmonit_container_up == bool 0`). | > 0 | 2 Min | Advisory (`warning`) | 1 h |
| Container unhealthy | Der Healthcheck des Containers schlägt seit drei Minuten fehl (`dumbmonit_container_health == 2`; 0 keiner, 1 healthy, 3 starting). | > 0 | 3 Min | Advisory (`warning`) | 1 h |
| Container restarting | Der Container ist in fünfzehn Minuten dreimal oder öfter neu gestartet (`increase(dumbmonit_container_restart_count[15m])`): er ist selten lange genug gestoppt für die vorherige Regel. | ≥ 3 | 1 Min | Advisory (`warning`) | 1 h |
| Container update available | Ein neueres Image ist für diesen Container in der Registry verfügbar (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Plakar-Backups

Gemeldet vom Agent, pro Kloset und pro Quelle. Keine dieser Zeitreihen
existiert, solange Plakar nicht erkannt wird, daher kann keine der beiden
Regeln auf einer Maschine ohne Plakar feuern.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Plakar backup too old | Seit mehr als zwei Tagen kein neuer Plakar-Snapshot für diese Quelle (`dumbmonit_backup_last_success_seconds`). | > 2 T | 1 h | Advisory (`warning`) | 24 h |
| Plakar backup failed | Der letzte Snapshot hat Fehler, oder das Kloset kann nicht gelesen werden (`dumbmonit_backup_last_status < 1`). | < 1 | 10 Min | Warning (`critical`) | 24 h |

### restic- und Borg-Backups

Gemeldet vom Agent für jedes unter `restic_repos` oder `borg_repos` in
agent.yaml deklarierte Repository; siehe
[Agent](../devices/agent.md#restic-and-borg-backups). Ohne deklariertes
Repository gibt es keine Zeitreihe, daher kann keine der beiden Regeln
feuern.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| restic/Borg backup too old | Seit mehr als 48 Stunden kein neuer restic- oder Borg-Snapshot in diesem Repository (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Advisory (`warning`) | 24 h |
| restic/Borg repository unreachable | Der Agent kann das Repository nicht lesen: nicht erreichbar, Passwort abgelehnt oder Binary fehlt; das Log des Agents sagt welches (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 Min | Advisory (`warning`) | 24 h |

### WireGuard-Tunnel

Gemeldet vom Agent; siehe [Agent](../devices/agent.md#wireguard-tunnels).

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| WireGuard peer silent | Ein Peer mit dauerhaftem Keepalive hat seit mehr als 15 Minuten kein Handshake abgeschlossen: der Tunnel ist down (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Bei einem Peer, der nie verbunden war, zählt die Stille ab dem Moment, in dem der Agent ihn zuerst sah. Peers ohne Keepalive (ein Telefon, ein Laptop) sind tagelang untätig und werden ausgeschlossen; ändern Sie den Schwellenwert, um N zu ändern, oder entfernen Sie den `and on …`-Teil, um jeden Peer zu beobachten. | > 15 Min | 5 Min | Advisory (`warning`) | 6 h |

### Proxmox VE

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Backup too old | Seit mehr als sieben Tagen kein erfolgreiches Proxmox-VE-Backup (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 T | 1 h | Advisory (`warning`) | 24 h |
| VM or container stopped | Der Gast lief innerhalb der letzten zwei Stunden und ist seit fünf Minuten gestoppt (`1 - dumbmonit_proxmox_guest_running`, UND-verknüpft mit `max_over_time(…[2h]) == 1`). | > 0 | 5 Min | Advisory (`warning`) | 6 h |
| VM or container CPU high | Ein Gast hat fünfzehn Minuten lang mehr als 90 % seiner zugewiesenen Kerne genutzt (`dumbmonit_proxmox_guest_cpu_percent`). Die Benachrichtigung benennt den Gast, `nextcloud (202)`. | > 90 % | 15 Min | Advisory (`warning`) | 6 h |
| VM or container memory high | Ein Gast hat zehn Minuten lang mehr als 95 % seines Speichers genutzt (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 Min | Advisory (`warning`) | 6 h |
| VM or container disk almost full | Die Root-Platte eines Gasts ist zu mehr als 90 % voll (`dumbmonit_proxmox_guest_disk_used_percent`): Container immer, VMs nur wenn der QEMU-Guest-Agent die Nutzung meldet. | > 90 % | 15 Min | Advisory (`warning`) | 24 h |
| Guest locked | Eine VM oder ein Container ist seit sechs Stunden gesperrt (`dumbmonit_proxmox_guest_locked`, Label `lock`): ein unterbrochenes Backup oder eine Migration hat die Sperre hinterlassen, und keine Operation kann auf dem Gast laufen, nicht einmal das nächste Backup. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| Old snapshot | Der älteste Snapshot eines Gasts ist mehr als dreißig Tage alt (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 T | 1 h | Info (`info`) | 7 T |
| HA resource in error | Eine Proxmox-HA-Ressource ist im Fehlerzustand oder fenced (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 Min | Warning (`critical`) | 1 h |
| HA manager not reporting | Der lokale HA-Ressourcenmanager eines Knotens hat aufgehört, seinen Zustand zu schreiben (`dumbmonit_proxmox_ha_lrm_stale`): Die HA-Gäste dieses Knotens werden weder neu gestartet noch verschoben. | > 0 | 5 Min | Warning (`critical`) | 1 h |
| Cluster lost quorum | Der Proxmox-Cluster hat kein Quorum mehr (`1 - dumbmonit_proxmox_cluster_quorate`): Gäste können nicht mehr gestartet werden. | > 0 | 1 Min | Warning (`critical`) | 30 Min |
| Proxmox node offline | Ein Cluster-Knoten wird als offline gemeldet (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 Min | Warning (`critical`) | 30 Min |
| Proxmox service down | Ein Proxmox-Kerndienst läuft nicht auf einem Knoten (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, jeweils nur gezählt, wo der Knoten ihn aktiviert). | > 0 | 5 Min | Warning (`critical`) | 1 h |
| Node network interface down | Eine zum Booten vorgesehene Schnittstelle ist auf einem Knoten nicht up (`dumbmonit_proxmox_node_interface_offline`). Eine bei Bedarf aktivierte Schnittstelle wird nie gemeldet. | > 0 | 5 Min | Advisory (`warning`) | 6 h |
| Proxmox storage almost full | Ein Proxmox-Storage ist zu mehr als 85 % voll (`dumbmonit_proxmox_storage_used_percent`); "Disk almost full" übernimmt bei 90 %. | > 85 % | 15 Min | Advisory (`warning`) | 24 h |
| Thin pool almost full | Ein LVM-Thin-Pool ist zu mehr als 90 % voll: jeder darauf gespeicherte Gast wird read-only, wenn er sich füllt (`dumbmonit_proxmox_node_thinpool_used_percent`); löst sich unter 88 %. | > 90 % | 15 Min | Warning (`critical`) | 6 h |
| Thin pool metadata almost full | Das Metadaten-Volume eines LVM-Thin-Pools ist zu mehr als 80 % voll (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). Es ist deutlich kleiner als das Daten-Volume und sättigt oft zuerst, mit derselben Konsequenz. | > 80 % | 15 Min | Warning (`critical`) | 6 h |
| Proxmox disk wearing out | Eine SSD oder NVMe eines Knotens hat mehr als 90 % ihrer bewerteten Lebensdauer verbraucht (`dumbmonit_proxmox_node_disk_wearout_percent`). Die Benachrichtigung liest sich `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Advisory (`warning`) | 7 T |
| Proxmox disk SMART failure | Eine Platte eines Knotens meldet den SMART-Status `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 Min | Warning (`critical`) | 24 h |
| Proxmox ZFS pool degraded | Ein ZFS-Pool eines Knotens ist nicht `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). Die Benachrichtigung liest sich `pve2 · tank`. | > 0 | 5 Min | Warning (`critical`) | 6 h |
| Backup job failed | Der letzte vzdump-Job auf einem Knoten, oder ein geplanter Backup-Job, ist fehlgeschlagen (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 Min | Warning (`critical`) | 24 h |
| Backup job excludes a disk | Ein geplanter Backup-Job überspringt eine Platte eines der von ihm abgedeckten Gäste (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): Der Job gelingt jede Nacht, wobei diese Daten fehlen. CD-ROM-Laufwerke und Cloudinit-Images sind absichtlich ausgeschlossen und werden nie gezählt. | > 0 | 1 h | Advisory (`warning`) | 7 T |
| Replication failed | Ein Replikationsjob meldet einen Fehler oder eine Fehleranzahl größer null (`dumbmonit_proxmox_replication_job_error`): Die Standby-Kopie ist veraltet. | > 0 | 10 Min | Warning (`critical`) | 6 h |
| Ceph health error | Ceph meldet `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 Min | Warning (`critical`) | 30 Min |
| Ceph health warning | Ceph meldet seit mehr als fünfzehn Minuten `HEALTH_WARN` (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 Min | Advisory (`warning`) | 6 h |
| Ceph OSD down | Ein Ceph-OSD ist seit fünf Minuten down (`1 - dumbmonit_proxmox_ceph_osd_up`). Die Benachrichtigung benennt das OSD und seinen Host. | > 0 | 5 Min | Warning (`critical`) | 1 h |
| Ceph OSD nearly full | Ein einzelnes Ceph-OSD ist zu mehr als 85 % voll (`dumbmonit_proxmox_ceph_osd_used_percent`): Schreibvorgänge stoppen, wenn es sein Limit erreicht, egal was der Cluster-Durchschnitt sagt; löst sich unter 82 %. | > 85 % | 15 Min | Advisory (`warning`) | 24 h |
| Ceph pool almost full | Ein Ceph-Pool ist zu mehr als 85 % voll (`dumbmonit_proxmox_ceph_pool_used_percent`); löst sich unter 82 %. | > 85 % | 15 Min | Advisory (`warning`) | 24 h |
| Ceph noout flag left on | Das Ceph-Flag `noout` ist seit zwei Stunden gesetzt (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): Rebalancing ist deaktiviert, und der Cluster liest sich gesund, während seine Redundanz schrumpft. | > 0 | 2 h | Info (`info`) | 24 h |
| Proxmox updates pending | Mehr als zwanzig Paket-Updates stehen auf einem Knoten aus (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T |
| Proxmox security updates pending | Mindestens ein ausstehendes Update stammt aus einem Security-Archiv (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Advisory (`warning`) | 7 T |
| Proxmox packages changed | Die installierte Version eines Proxmox-Pakets hat sich zwischen zwei Abfragen geändert — jemand hat `apt upgrade` ausgeführt (`dumbmonit_proxmox_node_packages_changed`, eine Stunde lang veröffentlicht, mit dem Detail im Label `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Löst sich eine Stunde später von selbst. | > 0 | 1 Min | Info (`info`) | 24 h |
| Node certificate expiring | Ein Zertifikat eines Proxmox-Knotens läuft in weniger als vierzehn Tagen ab (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 T | 1 h | Advisory (`warning`) | 24 h |

### Proxmox Backup Server

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| PBS datastore almost full | PBS-Datastore zu mehr als 90 % voll. | > 90 % | 15 Min | Advisory (`warning`), eskaliert nach 24 h | 6 h |
| PBS datastore filling up | Bei der aktuellen Rate schätzt PBS, dass der Datastore innerhalb von sieben Tagen voll ist. | < 7 T | 1 h | Advisory (`warning`) | 24 h |
| PBS datastore not mounted | Ein Wechsel-Datastore ist nicht eingehängt (`dumbmonit_pbs_datastore_removable_unmounted`): Es kann nichts darauf geschrieben werden. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| PBS backup too old | Seit mehr als zwei Tagen kein neuer Snapshot für diese Backup-Gruppe (eine pro Maschine) (`dumbmonit_pbs_backup_last_age_seconds`). Schwellenwert anpassen für wöchentlich gesicherte Maschinen; die Benachrichtigung benennt die Gruppe und den Datastore. | > 2 T | 1 h | Advisory (`warning`) | 24 h |
| PBS backup verification failed | Die Verifizierung des letzten Snapshots dieser Maschine ist fehlgeschlagen (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 Min | Warning (`critical`) | 24 h |
| PBS task failed | Mindestens ein PBS-Task ist im Prüffenster fehlgeschlagen (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 Min | Advisory (`warning`) | 24 h |
| PBS garbage collection too old | Seit mehr als acht Tagen keine erfolgreiche Garbage Collection auf diesem Datastore (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 T | 1 h | Advisory (`warning`) | 24 h |
| PBS garbage collection failed | Die letzte Garbage Collection auf einem Datastore ist fehlgeschlagen (`dumbmonit_pbs_gc_last_run_ok < 1`; ab `/gc` auf PBS 3.3+, davor vom letzten GC-Task). | < 1 | 10 Min | Advisory (`warning`) | 24 h |
| PBS corrupt chunks found | Die Garbage Collection hat auf einem Datastore unlesbare Chunks gefunden (`dumbmonit_pbs_gc_bad_chunks`): manche Backups können nicht mehr wiederherstellt werden. | > 0 | 10 Min | Warning (`critical`) | 24 h |
| PBS prune job failed | Der letzte Lauf eines PBS-Prune-Jobs ist fehlgeschlagen: nichts wird bereinigt, und der Datastore füllt sich weiter (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). Die Benachrichtigung benennt den Job und den Datastore. | < 1 | 10 Min | Advisory (`warning`) | 24 h |
| PBS verification job failed | Der letzte Lauf eines PBS-Verifizierungsjobs ist fehlgeschlagen (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 Min | Advisory (`warning`) | 24 h |
| PBS sync job failed | Der letzte Lauf eines PBS-Sync-Jobs ist fehlgeschlagen (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 Min | Warning (`critical`) | 24 h |
| PBS tape backup failed | Der letzte Tape-Backup-Lauf ist fehlgeschlagen (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): die Offline-Kopie wird nicht erstellt. | < 1 | 10 Min | Warning (`critical`) | 24 h |
| PBS job never ran | Ein Job ist aktiviert und geplant, aber noch nie gelaufen (`dumbmonit_pbs_job_never_run`). | > 0 | 2 T | Advisory (`warning`) | 7 T |
| PBS service down | Ein von Proxmox Backup Server benötigter Dienst läuft nicht (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); es kann kein Backup erstellt werden, solange er down ist. | < 1 | 5 Min | Warning (`critical`) | 6 h |
| PBS disk SMART failure | Eine Platte des Backup-Servers meldet den SMART-Status `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). Die Benachrichtigung liest sich `/dev/sdb`. | > 0 | 5 Min | Warning (`critical`) | 24 h |
| PBS SSD worn out | Eine SSD des Backup-Servers hat mehr als 90 % ihrer bewerteten Ausdauer verbraucht (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Advisory (`warning`) | 7 T |
| PBS ZFS pool degraded | Ein ZFS-Pool des Backup-Servers ist nicht `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 Min | Warning (`critical`) | 6 h |
| PBS certificate expiring | Das vom Backup-Server bereitgestellte Zertifikat läuft in weniger als drei Wochen ab (`dumbmonit_pbs_node_certificate_expires_seconds`). Benötigt die Option `certificates`. | < 21 T | 1 h | Advisory (`warning`) | 48 h |
| PBS restart pending after upgrade | Das Paket wurde aktualisiert, aber der laufende Dienst ist noch der alte (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 T |
| PBS updates pending | Mehr als zwanzig Paket-Updates stehen auf dem Backup-Server aus (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T |

### Proxmox Datacenter Manager

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Federated instance unreachable | Die Datacenter-Konsole kann eines der von ihr föderierten Proxmox-VE-Cluster oder Backup-Server nicht erreichen (`dumbmonit_pdm_remote_reachable < 1`). Die Benachrichtigung benennt die Instanz und ihr Produkt. | < 1 | 10 Min | Warning (`critical`) | 6 h |
| Federated instance behind | Eine Instanz läuft mit einer älteren Version als eine andere Instanz desselben Produkts im selben Verbund (`dumbmonit_pdm_remote_version_behind`). Verglichen zwischen Instanzen, nie gegen das, was Proxmox veröffentlicht. | > 0 | 1 h | Info (`info`) | 7 T |
| Task failed on a federated instance | Mindestens ein Task ist auf einer föderierten Instanz im Prüffenster fehlgeschlagen (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 Min | Advisory (`warning`) | 24 h |
| Datacenter console disk almost full | Das Root-Dateisystem der Datacenter-Konsole ist zu mehr als 90 % voll (`dumbmonit_pdm_node_rootfs_percent`): voll, hört die Konsole auf aufzuzeichnen, was sie erfasst. | > 90 % | 15 Min | Advisory (`warning`), eskaliert nach 24 h | 6 h |
| Datacenter console certificate expiring | Ein Zertifikat der Datacenter-Konsole läuft in weniger als vierzehn Tagen ab (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 T | 1 h | Advisory (`warning`) | 24 h |
| Datacenter console updates pending | Mehr als zwanzig Paket-Updates stehen auf der Datacenter-Konsole aus (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T |

### Proxmox Mail Gateway

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Mail queue growing | Die zurückgestellte Mail-Warteschlange ist in zwei Stunden um mehr als hundert Nachrichten gewachsen (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): Der nächste Hop verweigert wahrscheinlich die Mail. Wachstum statt Größe, weil ein ausgelastetes Gateway immer ein paar Dutzend zurückgestellte Nachrichten behält. | > 100 | 15 Min | Advisory (`warning`) | 6 h |
| Mail stuck in the queue | Eine Nachricht wartet seit mehr als vier Stunden in einer Warteschlange (`dumbmonit_pmg_queue_oldest_age_seconds`). Das Alter ist eine untere Grenze — qshape meldet Bereiche — daher feuert die Regel spät statt früh. Die Benachrichtigung benennt die Warteschlange. | > 4 h | 15 Min | Warning (`critical`) | 6 h |
| Mail gateway service stopped | Ein vom Gateway zum Annehmen und Filtern von Mail benötigter Dienst ist gestoppt (`dumbmonit_pmg_service_running` für postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Eine nicht installierte Unit hat keine Zeitreihe und kann nicht feuern. | < 1 | 5 Min | Warning (`critical`) | 6 h |
| Virus signatures out of date | Die täglichen ClamAV-Virensignaturen sind mehr als zwei Tage alt (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): freshclam hat aufgehört zu aktualisieren, und das Gateway filtert weiter mit der Liste von letzter Woche. | > 2 T | 1 h | Advisory (`warning`) | 24 h |
| Spam rules out of date | Die SpamAssassin-Regeln sind mehr als acht Tage alt (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Ein nie aktualisierter Kanal hat kein Datum und keine Zeitreihe. | > 8 T | 1 h | Advisory (`warning`) | 7 T |
| Quarantine filling up | Die Spam-Quarantäne hat in sechs Stunden mehr als tausend Nachrichten aufgenommen (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): eine Kampagne, oder eine Regel, die gerade eine legitime Domain in den Papierkorb gesteckt hat. | > 1000 | 30 Min | Advisory (`warning`) | 12 h |
| Mail gateway cluster degraded | Ein Gateway des Clusters ist nicht mehr mit den anderen synchron (`dumbmonit_pmg_cluster_node_insync`): Es filtert mit einer veralteten Regel-Datenbank. Ein Einzel-Gateway hat keine Zeitreihe. | < 1 | 15 Min | Advisory (`warning`) | 6 h |
| Mail gateway certificate expiring | Ein Zertifikat des Mail-Gateways läuft in weniger als vierzehn Tagen ab (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 T | 1 h | Advisory (`warning`) | 24 h |
| Mail gateway updates pending | Mehr als zwanzig Paket-Updates stehen auf dem Mail-Gateway aus (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T |

### MDaemon und SecurityGateway

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| MDaemon mail service down | Ein beobachteter Mail-Dienst des MDaemon-Servers antwortet nicht, oder begrüßt nur, um den Dienst zu verweigern (`dumbmonit_mdaemon_service_up`). Nur die am Gerät aufgeführten Dienste haben eine Zeitreihe. Die Benachrichtigung benennt den Dienst und seinen Port. | < 1 | 5 Min | Warning (`critical`) | 6 h |
| MDaemon XML API not answering | Die XML-API hat aufgehört zu antworten, während die Mail-Dienste es noch tun (`dumbmonit_mdaemon_api_up`): Remote Administration ist wahrscheinlich gestoppt. Nur ein Gerät mit einem Konto hat die Zeitreihe. | < 1 | 15 Min | Advisory (`warning`) | 12 h |
| MDaemon mail queue growing | Die Inbound-, Local- oder Remote-Warteschlange ist in zwei Stunden um mehr als hundert Nachrichten gewachsen (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): MDaemon hat aufgehört, daraus zuzustellen. Vom [Agent](../devices/agent.md#windows-performance-counters) auf dem MDaemon-Server gelesen, daher liegt der Alarm auf dem Gerät des Agents. | > 100 | 15 Min | Advisory (`warning`) | 6 h |
| MDaemon Bad queue not empty | Nachrichten warten in der Bad-Warteschlange (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): MDaemon konnte sie weder zustellen noch zurückweisen, und sie bleiben dort, bis ein Administrator sie freigibt oder löscht. | > 0 | 30 Min | Advisory (`warning`) | 24 h |
| MDaemon Retry queue high | Mehr als fünfzig Nachrichten warten in der Retry-Warteschlange (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): Entfernte Server verweigern weiterhin vorübergehend die Mail. | > 50 | 1 h | Advisory (`warning`) | 6 h |
| SecurityGateway service down | Ein beobachteter Dienst des Gateways antwortet nicht, oder begrüßt nur, um den Dienst zu verweigern (`dumbmonit_securitygateway_service_up`). | < 1 | 5 Min | Warning (`critical`) | 6 h |
| SecurityGateway API not answering | Die REST-API hat aufgehört zu antworten, während die Dienste es noch tun (`dumbmonit_securitygateway_api_up`). Nur ein Gerät mit einem API-Schlüssel hat die Zeitreihe. | < 1 | 15 Min | Advisory (`warning`) | 12 h |
| SecurityGateway delivery queue growing | Die Zustellungswarteschlange ist in zwei Stunden um mehr als hundert Nachrichten gewachsen (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): Der Mailserver hinter dem Gateway verweigert wahrscheinlich die Mail. Die Zählernamen stammen unverändert aus der API; wenn Ihre Version die Warteschlange anders benennt, passen Sie den Matcher an. | > 100 | 15 Min | Advisory (`warning`) | 6 h |

### OPNsense

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Internet gateway down | dpinger erhält keine Antwort mehr von einem Gateway (`dumbmonit_opnsense_gateway_up`). Auf einer Multi-WAN-Firewall ist der Verkehr bereits leise auf eine andere Leitung umgezogen. `loss` und `delay` sind Warnungen, keine Ausfälle, und lösen diese Regel nicht aus. | < 1 | 5 Min | Warning (`critical`) | 6 h |
| Gateway losing packets | Ein Gateway verliert mehr als ein Fünftel der an es gesendeten Pakete (`dumbmonit_opnsense_gateway_loss_percent`): Die Leitung antwortet, aber schlecht. Löst sich unter 10 %. | > 20 % | 10 Min | Advisory (`warning`) | 6 h |
| Gateway slow | Die Round-Trip-Zeit eines Gateways ist über 300 ms geblieben (`dumbmonit_opnsense_gateway_delay_seconds`). Dreihundert Millisekunden lassen eine mobile Backup-Leitung in Ruhe. | > 300 ms | 15 Min | Info (`info`) | 12 h |
| Firewall state table filling up | pf verfolgt mehr als 80 % der erlaubten Verbindungen (`dumbmonit_opnsense_pf_states_used_percent`). Jenseits des Limits verwirft die Firewall neue Verbindungen ohne weiteres Symptom. | > 80 % | 10 Min | Advisory (`warning`) | 6 h |
| Firewall network buffers exhausted | Mehr als 90 % der FreeBSD-Netzwerkpuffer sind belegt (`dumbmonit_opnsense_mbuf_used_percent`). Eine Firewall, die ausgeht, hört auf weiterzuleiten, während die CPU noch untätig ist. | > 90 % | 10 Min | Advisory (`warning`) | 6 h |
| VPN tunnel down | Ein konfigurierter VPN-Tunnel hat seit zehn Minuten keine Sitzung (`dumbmonit_opnsense_vpn_tunnel_up`). Ein nicht installiertes Plugin, oder eine deaktivierte Verbindung, hat keine Zeitreihe und kann nicht feuern. | < 1 | 10 Min | Advisory (`warning`) | 6 h |
| Firewall resolver stopped | Unbound hat aufgehört zu antworten (`dumbmonit_opnsense_unbound_running`). Die Firewall routet noch; die dahinterliegenden Maschinen lösen nicht mehr auf. Schalten Sie diese Regel aus, wenn Ihre Firewall mit etwas anderem auflöst. | < 1 | 5 Min | Advisory (`warning`) | 6 h |
| Firewall core service stopped | dpinger, das die Gateways beobachtet, oder configd, das alles andere ausführt, ist gestoppt (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Nur diese beiden, weil ein absichtlich abgeschalteter Dienst kein Ausfall ist. | < 1 | 5 Min | Warning (`critical`) | 6 h |
| Firewall too hot | Ein Sensor liest über 85 °C (`dumbmonit_opnsense_temperature_celsius`): Die lüfterlose Box im Schrank steht kurz vor dem Throttling. Löst sich bei 78 °C. | > 85 °C | 10 Min | Advisory (`warning`) | 6 h |
| Firewall left in CARP maintenance mode | CARP wurde dauerhaft im Wartungsmodus belassen (`dumbmonit_opnsense_carp_maintenance_mode`): Diese Firewall hat ihre virtuellen Adressen an ihren Partner übergeben und wird sie von selbst nicht zurücknehmen. | > 0 | 1 h | Info (`info`) | 24 h |
| Firewall reboot pending | Ein Update ist installiert, braucht aber einen Reboot, um wirksam zu werden (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 T |
| Firewall updates pending | Die Firewall hat Pakete, die auf ein Update warten (`dumbmonit_opnsense_firmware_updates_pending`). Sie ist die exponierteste Maschine im Netzwerk. | > 0 | 1 h | Info (`info`) | 7 T |

### TrueNAS

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| NAS pool degraded | ZFS nennt einen Pool nicht mehr gesund (`dumbmonit_truenas_pool_healthy`). Ein degradierter Pool liefert seine Daten noch, was genau der Grund ist, warum es niemand bemerkt, bis die nächste Platte ausfällt. Die Geräteseite benennt die Platte. | < 1 | 5 Min | Warning (`critical`) | 6 h |
| NAS disk errors | Platten eines Pools haben seit dem letzten `zpool clear` Lese-, Schreib- oder Prüfsummenfehler gemeldet (`dumbmonit_truenas_pool_device_errors`, eine Zeitreihe pro Art). | > 0 | 15 Min | Advisory (`warning`) | 24 h |
| NAS pool almost full | Ein Pool ist zu mehr als 85 % voll (`dumbmonit_truenas_pool_used_percent`). ZFS wird deutlich langsamer, bevor es voll ist. Löst sich bei 80 %. | > 85 % | 30 Min | Advisory (`warning`), eskaliert nach 7 T | 24 h |
| NAS scrub found errors | Der letzte Scrub eines Pools hat beschädigte Daten gefunden (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 Min | Warning (`critical`) | 24 h |
| NAS scrub overdue | Ein Pool hat seit mehr als 45 Tagen keinen Scrub abgeschlossen (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Fehlt nach einem Resilver, das den Eintrag des letzten Scrubs löscht. | > 45 T | 1 h | Info (`info`) | 7 T |
| NAS disk failed its SMART test | Ein SMART-Selbsttest im Log einer Platte ist fehlgeschlagen (`dumbmonit_truenas_disk_smart_failed`). Eine nie getestete Platte hat keine Zeitreihe. | > 0 | 5 Min | Warning (`critical`) | 24 h |
| NAS disk too hot | Eine Platte liegt über 55 °C (gelesen aus TrueNAS' Cache, `dumbmonit_truenas_disk_temperature_celsius`). Löst sich bei 50 °C. | > 55 °C | 15 Min | Advisory (`warning`) | 6 h |
| NAS dataset near its quota | Ein Dataset hat mehr als 90 % seines Kontingents genutzt (`dumbmonit_truenas_dataset_quota_used_percent`): Bei 100 % schlagen seine Schreibvorgänge fehl, während der Pool noch Platz hat. Kein Kontingent, keine Zeitreihe. | > 90 % | 30 Min | Advisory (`warning`) | 24 h |
| NAS replication failed | Ein aktivierter Replikationstask ist im Fehlerzustand (`dumbmonit_truenas_replication_error`): Die Kopie auf der anderen Seite wird immer älter. | > 0 | 10 Min | Advisory (`warning`) | 12 h |
| NAS snapshot task failed | Ein aktivierter periodischer Snapshot-Task ist im Fehlerzustand (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 Min | Advisory (`warning`) | 12 h |
| NAS snapshots stale | Ein aktivierter periodischer Snapshot-Task ist seit mehr als acht Tagen nicht gelaufen (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): Ein Task, der aufhört zu laufen, schlägt nicht fehl, er wird nur still. | > 8 T | 1 h | Advisory (`warning`) | 24 h |
| TrueNAS alert raised | TrueNAS selbst hat einen Alarm der Stufe ERROR oder höher ausgelöst (`dumbmonit_truenas_alerts`). Sein eigener Satz steht auf der Geräteseite. | > 0 | 10 Min | Advisory (`warning`) | 12 h |
| NAS service stopped | Ein zum Start mit dem NAS vorgesehener Dienst ist gestoppt (`dumbmonit_truenas_service_running`). Ein absichtlich abgeschalteter Dienst hat keine Zeitreihe. | < 1 | 10 Min | Advisory (`warning`) | 6 h |

### Synology DSM

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Synology disk SMART warning | DSM meldet den SMART-Status dieser Platte nicht mehr als `normal` (`dumbmonit_synology_disk_smart_status`: 1 Achtung, 2 kritisch, unbekannte Wörter zählen als 1). | > 0 | 15 Min | Advisory (`warning`) | 24 h |
| Synology disk failed | DSM meldet die Platte als abgestürzt oder ausgefallen (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 Min | Warning (`critical`) | 6 h |
| Synology disk bad sectors | Die Anzahl der fehlerhaften Sektoren der Platte überschreitet den in DSM gesetzten Schwellenwert (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 Min | Warning (`critical`) | 24 h |
| Synology disk bad sectors growing | In den letzten 24 Stunden sind neue unlesbare Sektoren aufgetreten (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 Min | Advisory (`warning`) | 24 h |
| Synology SSD wearing out | Eine SSD hat weniger als 10 % ihrer bewerteten Lebensdauer übrig (`dumbmonit_synology_disk_remaining_life_percent`); löst sich über 12 %. | < 10 % | 1 h | Advisory (`warning`) | 7 T |
| Synology volume degraded | Ein Volume ist degradiert oder abgestürzt: seine Redundanz ist weg (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 Min | Warning (`critical`) | 6 h |
| Synology volume almost full | Ein Volume ist zu 90 % oder mehr voll (`dumbmonit_synology_volume_used_percent`); löst sich unter 88 %. | ≥ 90 % | 15 Min | Advisory (`warning`), eskaliert nach 24 h | 6 h |
| Synology temperature high | Eine Platte liegt über 55 °C, oder DSM hat seine eigene Temperaturwarnung ausgelöst (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); löst sich unter 52 °C. | > 55 °C | 15 Min | Advisory (`warning`) | 6 h |
| Synology memory high | Speichernutzung des NAS über 95 % (`dumbmonit_synology_memory_usage_percent`); löst sich unter 90 %. | > 95 % | 15 Min | Advisory (`warning`) | 6 h |
| Active Backup task failed | Der letzte Lauf eines Active-Backup-for-Business-Tasks ist fehlgeschlagen, oder hat nur einen Teil seiner Geräte gesichert (`dumbmonit_abb_task_last_status == bool 0`). Die Benachrichtigung benennt den Task. | > 0 | 10 Min | Warning (`critical`) | 24 h |
| Active Backup too old | Seit mehr als zwei Tagen kein erfolgreicher Active-Backup-for-Business-Lauf für diesen Task (`dumbmonit_abb_task_last_success_seconds`). Die Zeitreihe existiert nicht für einen nie erfolgreichen Task; dann spricht "Active Backup task failed". | > 2 T | 1 h | Advisory (`warning`) | 24 h |
| Active Backup task disabled | Der Task hat keinen Zeitplan, oder sein kontinuierliches Backup ist pausiert (`dumbmonit_abb_task_enabled == bool 0`): Er sichert nichts, bis jemand ihn ausführt. | > 0 | 1 h | Info (`info`) | 7 T |
| Active Backup device overdue | Ein Gerät ist länger ohne erfolgreichen Active-Backup-for-Business-Lauf als sein eigener Rhythmus erlaubt, freie Tage ausgeschlossen (`dumbmonit_abb_device_overdue`, siehe [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 Min | Advisory (`warning`) | 24 h |
| Active Backup device failing | Die letzten zwei oder mehr Versuche eines Geräts sind fehlgeschlagen (`dumbmonit_abb_device_consecutive_failures`); ein abgebrochener Lauf zählt für keines von beiden. | ≥ 2 | 10 Min | Warning (`critical`) | 24 h |

### Log- und Metrik-Server

VictoriaMetrics, VictoriaLogs, Loki und Graylog ([Geräte](../devices/victoriametrics.md)). Zähler laufen über `increase_prometheus`, daher feuert ein mit einer Historie abgelehnter Daten hinzugefügter Server nicht bei seiner ersten Messung.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Metrics or log storage read-only | Der freie Plattenplatz ist unter `-storage.minFreeDiskSpaceBytes` gefallen, und der Storage verweigert jedes Schreiben (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 Min | Warning (`critical`) | 1 h |
| Metrics or log storage almost read-only | Weniger als 10 % der Platte übrig, bevor read-only (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); löst sich über 12 %. | < 10 % | 30 Min | Advisory (`warning`) | 24 h |
| Metrics or log server unhealthy | `/health` antwortet nicht OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 Min | Warning (`critical`) | 6 h |
| Metrics or log data refused | In der letzten Stunde abgelehnte Samples oder Log-Zeilen: Zeitstempel außerhalb der Aufbewahrung, zu lange Labels, Zeitreihen-Limit (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 Min | Advisory (`warning`) | 24 h |
| VictoriaMetrics slow inserts | Mehr als 5 % der neuen Samples haben in 15 Minuten den langsamen Pfad genommen: nicht genug Speicher für die aktiven Zeitreihen (`100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])`); löst sich unter 3 %. | > 5 % | 30 Min | Advisory (`warning`) | 24 h |
| Loki not ready | `/ready` antwortet nicht ready (`dumbmonit_loki_ready`). | < 1 | 5 Min | Warning (`critical`) | 6 h |
| Loki refusing log lines | In der letzten Stunde abgelehnte Zeilen: zu alt, Rate- oder Stream-Limit, Zeile zu lang (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 Min | Advisory (`warning`) | 24 h |
| Loki cannot flush to storage | Chunks haben den Storage in den letzten 30 Minuten nicht erreicht; sie gehen verloren, wenn Loki neu startet (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 Min | Warning (`critical`) | 1 h |
| Loki write-ahead log disk full | WAL-Schreibvorgänge an einer vollen Platte verloren (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 Min | Warning (`critical`) | 1 h |
| Loki request errors | Mehr als fünf Anfragen in 15 Minuten mit einem Serverfehler beantwortet (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 Min | Advisory (`warning`) | 6 h |
| Graylog not processing | Nachrichtenverarbeitung auf dem Knoten pausiert (`dumbmonit_graylog_processing`). | < 1 | 5 Min | Warning (`critical`) | 6 h |
| Graylog search cluster down | OpenSearch oder Elasticsearch rot, oder von Graylog aus nicht erreichbar (`dumbmonit_graylog_indexer_status`: 2 rot, 3 nicht erreichbar). | ≥ 2 | 5 Min | Warning (`critical`) | 1 h |
| Graylog search cluster yellow | Replica-Shards seit einer halben Stunde nicht zugewiesen (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 Min | Advisory (`warning`) | 24 h |
| Graylog journal filling | Das Journal ist zu mehr als der Hälfte voll: Der Output hält nicht mit (`dumbmonit_graylog_journal_used_percent`); löst sich unter 40 %. | > 50 % | 10 Min | Advisory (`warning`), eskaliert nach 1 h | 6 h |
| Graylog input failed | Ein Input konnte nicht starten (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 Min | Advisory (`warning`) | 24 h |
| Graylog indexing failures | Schreibvorgänge zum Suchcluster oder zur Nachrichtenverarbeitung sind in der letzten Stunde fehlgeschlagen (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 Min | Advisory (`warning`) | 24 h |

### Selbst gehostete Anwendungen

Nextcloud, Immich, Paperless-ngx, Jellyfin und Plex ([Geräte](../devices/nextcloud.md)). Sie fallen selten ganz aus: Die Seite öffnet sich, während Synchronisierung, Thumbnails, Importe oder Bibliotheksscans gestoppt sind. Diese Regeln beobachten diese Zustände; eine überhaupt nicht antwortende Anwendung ist Device unreachable.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Nextcloud stuck in maintenance | Der Wartungsmodus ist an (`dumbmonit_nextcloud_maintenance`): Niemand kann sich einloggen oder synchronisieren. | > 0 | 30 Min | Advisory (`warning`), eskaliert nach 3 h | 6 h |
| Nextcloud database upgrade pending | Der Code wurde aktualisiert, und Nextcloud wartet auf `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 Min | Warning (`critical`) | 1 h |
| Nextcloud update available | Nextclouds Update-Check hat eine neuere Version gefunden (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 T |
| Nextcloud OPcache full | PHPs OPcache ist voll, und Seiten werden langsam (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 Min | Advisory (`warning`) | 24 h |
| Nextcloud free space low | Weniger als 5 GiB frei für das Datenverzeichnis (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); löst sich über 6 GiB. | < 5 GiB | 30 Min | Advisory (`warning`) | 24 h |
| Immich jobs stalled | Jobs warten in einer Warteschlange, die nicht pausiert ist und nichts ausführt (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). Die Benachrichtigung benennt die Warteschlange. | > 0 | 1 h | Advisory (`warning`) | 6 h |
| Immich queue paused | Eine Job-Warteschlange ist seit einem Tag pausiert (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 T |
| Immich storage almost full | Die Platte mit der Bibliothek ist zu mehr als 90 % voll (`dumbmonit_immich_storage_used_percent`); löst sich unter 88 %. | > 90 % | 30 Min | Advisory (`warning`) | 24 h |
| Immich update available | Immichs Versionscheck kennt eine neuere Version (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 T |
| Paperless task queue down | Redis oder Celery im Fehlerzustand (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 ist ein Fehler): nichts Neues wird importiert. | ≥ 2 | 5 Min | Warning (`critical`) | 1 h |
| Paperless index or sanity check error | Der Suchindex, der Klassifikator oder die Sanity-Prüfung (fehlende oder veränderte Dateien) meldet einen Fehler (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 Min | Advisory (`warning`) | 24 h |
| Paperless migrations pending | Noch anzuwendende Datenbankmigrationen (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 Min | Advisory (`warning`) | 24 h |
| Paperless task failed | Ein für diesen Benutzer sichtbarer Task ist im Zeitfenster für fehlgeschlagene Tasks fehlgeschlagen und wurde nicht verworfen (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 Min | Advisory (`warning`) | 24 h |
| Paperless storage almost full | Die Platte mit den Dokumenten ist zu mehr als 90 % voll (`dumbmonit_paperless_storage_used_percent`); löst sich unter 88 %. | > 90 % | 30 Min | Advisory (`warning`) | 24 h |
| Paperless update available | Paperless-ngx' Versionscheck kennt eine neuere Version (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 T |
| Jellyfin scheduled task failed | Ein geplanter Task (ein Bibliotheksscan, eine Bereinigung) ist beim letzten Lauf fehlgeschlagen oder abgebrochen worden (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 Min | Advisory (`warning`) | 24 h |
| Jellyfin plugin broken | Ein Plugin konnte nicht geladen werden oder unterstützt diese Version nicht (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 Min | Advisory (`warning`) | 24 h |
| Jellyfin restart pending | Jellyfin wartet seit einem Tag auf einen Neustart nach einer Plugin-Installation oder einem Update (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 T |
| Plex update available | Plex' Update-Check hat eine neuere Version gefunden (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 T |

### Selbst gehostete Forges

GitLab (self-managed) und Forgejo/Gitea ([Geräte](../devices/gitlab.md)). Eine
überhaupt nicht antwortende Instanz ist Device unreachable; diese Regeln
beobachten die Teile, die weiter Seiten ausliefern, während sie leise
zurückfallen — Sidekiq, CI/CD-Runner, ein nicht mehr laufender geplanter
Task.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| GitLab runners all offline | Jeder GitLab bekannte CI/CD-Runner ist offline, und mindestens einer ist registriert (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 Min | Advisory (`warning`) | 6 h |
| GitLab Sidekiq backlog high | Eine Sidekiq-Warteschlange hat mehr als 500 wartende Jobs (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): Benachrichtigungen, Webhooks und Importe fallen zurück. | > 500 | 15 Min | Advisory (`warning`) | 6 h |
| GitLab migrations pending | Datenbankmigrationen aus einem Upgrade sind noch anzuwenden (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Info (`info`) | 24 h |
| GitLab license expiring | Die Enterprise-Edition-Lizenz läuft in weniger als 30 Tagen ab (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 T | 1 h | Advisory (`warning`) | 24 h |
| Forgejo/Gitea runners all offline | Jeder Actions-Runner ist offline, und mindestens einer ist registriert (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 Min | Advisory (`warning`) | 6 h |
| Forgejo/Gitea scheduled task overdue | Ein geplanter Task (Repository-Check, Bereinigung, Update-Check) lief nicht in seinem eigenen Intervall (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 Min | Advisory (`warning`) | 24 h |

### Client-Geräte

Eine Regel, geteilt von jeder Integration, die verbundene und sichernde
Geräte verfolgt — Immichs Telefone, Tablets und Desktops
([Geräte](../devices/immich.md)), Proxmox Backup Servers gesicherte
Maschinen, eine pro Backup-Gruppe ([Geräte](../devices/pbs.md)), und Veeams
geschützte Rechner und VMs, eine pro Objekt mit einem Wiederherstellungspunkt
([Geräte](../devices/veeam.md)), heute; weitere folgen. Jede Integration
berechnet den Abstand zum eigenen Veralterungsfenster ihres Ziels
(`device_stale_days`, standardmäßig 3 Tage) und veröffentlicht ihn als
`dumbmonit_client_device_stale_seconds`, beschriftet mit `device`, `type`,
`os`, `user`, `kind` und `signal` (`connection` oder `backup`); diese eine
Regel feuert dann für jede davon und benennt das Gerät in der
Benachrichtigung. Synology Active Backup for Business verfolgt seine eigenen
Geräte separat, über einen rhythmusbewussten Mechanismus
(`synology_abb_device_*`) statt über diese generische Familie — siehe
[Synology](../devices/synology.md).

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Client device stale | Ein Gerät hat sich nicht innerhalb des für dieses Ziel konfigurierten Veralterungsfensters verbunden oder gesichert (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Advisory (`warning`) | 24 h |

### Pi-hole

[Pi-hole](../devices/pihole.md) v6, gelesen über seine REST-API.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Pi-hole blocking disabled | Blockierung ist aus, pausiert oder fehlgeschlagen (`dumbmonit_pihole_blocking_enabled`). Die Haltezeit lässt eine fünfminütige Pause über die Web-Oberfläche vergehen. | < 1 | 15 Min | Advisory (`warning`) | 6 h |
| Pi-hole blocklists stale | Die Blocklisten (Gravity) wurden seit mehr als acht Tagen nicht neu erstellt (`dumbmonit_pihole_gravity_age_seconds`): Das wöchentliche Update schlägt fehl. | > 8 T | 1 h | Advisory (`warning`) | 24 h |
| Pi-hole update available | Eine neuere Version ist verfügbar (`dumbmonit_pihole_updates_available`): das Docker-Image, Core, Web-Oberfläche oder FTL im Übrigen. Pi-hole prüft einmal täglich. | > 0 | 1 h | Info (`info`) | 7 T |
| Pi-hole diagnosis messages | Pi-hole listet Meldungen auf seiner Diagnoseseite (`dumbmonit_pihole_messages`): eine fehlgeschlagene Blockliste, ein ratenbegrenzter Client, ein überlasteter Host. | > 0 | 15 Min | Info (`info`) | 24 h |

### AdGuard Home

Der DNS-Filter ([Gerät](../devices/adguard.md)). Abfrage- und Blockierzahlen sind Summen über AdGuard Homes eigenes Statistikfenster, keine Zähler: Darauf ist keine Regel gebaut.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| AdGuard Home protection off | Schutz deaktiviert, oder pausiert und noch nicht zurück (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 Min | Advisory (`warning`) | 6 h |
| AdGuard Home DNS not running | Die Web-Oberfläche antwortet, aber der DNS-Server ist gestoppt (`dumbmonit_adguard_running`). | < 1 | 5 Min | Warning (`critical`) | 1 h |
| AdGuard Home upstream failing | Ein Upstream-DNS-Server schlägt bei AdGuard Homes eigenem Test fehl, oder bleibt 4 Sekunden still (`dumbmonit_adguard_upstream_up`, eine Zeitreihe pro Upstream). | < 1 | 10 Min | Advisory (`warning`) | 6 h |
| AdGuard Home filter lists stale | Das älteste Update einer aktivierten Filterliste ist mehr als drei Tage alt (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 Tage | 1 h | Advisory (`warning`) | 24 h |
| AdGuard Home update available | Eine neuere Version wird von AdGuard Homes eigenem Versionscheck angekündigt (`dumbmonit_adguard_update_available`); feuert nie, wenn diese Prüfung deaktiviert ist, wie im offiziellen Docker-Image. | > 0 | 1 h | Info (`info`) | 7 Tage |

### USV hinter NUT

USV, gelesen von einem [NUT-Server](../devices/nut.md). Jede Zeitreihe trägt
das Label `ups`, sodass ein Alarm die USV benennt, wenn ein Server mehrere
veröffentlicht. Die Statusflags sind bei jeder Lesung 0 oder 1, daher löst
sich eine Regel von selbst, sobald der Netzstrom zurück ist. Über SNMP
gelesene USVs werden von den [USV](#usv)-Regeln oben abgedeckt.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| UPS on battery (NUT) | Die USV versorgt die Last aus dem Akku (`dumbmonit_nut_ups_on_battery`, das `OB`-Flag von `ups.status`). | > 0 | 30 s | Warning (`critical`) | 15 Min |
| UPS battery low (NUT) | Die USV meldet einen schwachen Akku, das `LB`-Flag (`dumbmonit_nut_ups_low_battery`): Die von ihr versorgten Maschinen werden gleich herunterfahren. | > 0 | 1 Min | Warning (`critical`) | 30 Min |
| UPS battery needs replacing (NUT) | Die USV bittet um einen Akkutausch, das `RB`-Flag (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Advisory (`warning`) | 7 T |
| UPS load high (NUT) | Die Last ist über 80 % geblieben (`dumbmonit_nut_ups_load_percent`); löst sich unter 75 %. | > 80 % | 10 Min | Advisory (`warning`) | 24 h |
| UPS runtime short (NUT) | Die USV schätzt weniger als fünf Minuten Laufzeit im Akkubetrieb (`dumbmonit_nut_battery_runtime_seconds`); löst sich über sechs. Bei Netzstrom ein abgenutzter Akku oder eine für ihre Last zu kleine USV. | < 300 s | 5 Min | Advisory (`warning`) | 24 h |
| UPS data stale (NUT) | Der NUT-Server antwortet, hat aber keine frischen Daten von der USV: Kabel, Treiber oder USV still (`dumbmonit_nut_ups_data_stale`). Ein Ausfall würde währenddessen unbemerkt bleiben. | > 0 | 5 Min | Advisory (`warning`), eskaliert nach 1 h | 6 h |

### MikroTik RouterOS

Zeitreihen tragen das Label `interface` oder `sensor`, sodass eine Benachrichtigung den Port oder den Sensor benennt ([Gerät](../devices/mikrotik.md)). Zähler laufen über `increase_prometheus`: Ein mit jahrelangen Fehlern hinzugefügter Router feuert nicht bei seiner ersten Messung.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| MikroTik CPU high | Die CPU des Routers ist über 90 % geblieben (`dumbmonit_mikrotik_cpu_load_percent`): Routing, die Firewall oder ein Skript sättigt sie. Löst sich unter 80 %. | > 90 % | 15 Min | Advisory (`warning`) | 6 h |
| MikroTik memory high | Mehr als 90 % des Speichers sind belegt (`dumbmonit_mikrotik_memory_used_percent`). Löst sich unter 85 %. | > 90 % | 15 Min | Advisory (`warning`) | 6 h |
| MikroTik storage almost full | Der Flash-Speicher ist zu mehr als 90 % voll (`dumbmonit_mikrotik_storage_used_percent`): Konfigurationsänderungen und Upgrades können fehlschlagen. Löst sich unter 85 %. | > 90 % | 30 Min | Advisory (`warning`) | 24 h |
| MikroTik temperature high | Ein Temperatursensor liest über 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Löst sich bei 75 °C. | > 80 °C | 10 Min | Advisory (`warning`) | 6 h |
| MikroTik power supply or fan failed | Ein `psu*-state`- oder `fan*-state`-Sensor ist nicht `ok` (`dumbmonit_mikrotik_health_ok`). Ein ausgefallenes redundantes Netzteil unterbricht nichts, bis auch das zweite ausfällt. | < 1 | 5 Min | Warning (`critical`) | 1 h |
| MikroTik interface accumulating errors | Mehr als fünfzig Empfangs- und Sendefehler auf einer Schnittstelle in der letzten Stunde (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 Min | Advisory (`warning`) | 24 h |
| MikroTik link flapping | Eine Schnittstelle hat in dreißig Minuten mehr als zweimal den Link verloren (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Ein endgültig gezogener Port zählt einmal und feuert nicht. | > 2 | 5 Min | Advisory (`warning`) | 1 h |
| MikroTik RouterOS update available | Ein neueres RouterOS ist auf dem Update-Kanal des Routers verfügbar (`dumbmonit_mikrotik_update_available`). Nur bekannt, wenn der Router selbst nach Updates sucht: siehe den optionalen Schritt der [Einrichtung](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 T |
| MikroTik firmware upgrade pending | RouterBOOT ist älter als die mit dem installierten RouterOS gelieferte Firmware (`dumbmonit_mikrotik_firmware_upgrade_pending`); es wird nur auf Anfrage und dann beim nächsten Neustart aktualisiert. | > 0 | 1 h | Info (`info`) | 7 T |

### UniFi Network

[Geräteseite](../devices/unifi.md). Geräte-Zeitreihen tragen Name, Modell und Typ des Geräts, daher sagt die Benachrichtigung "Living room AP". Ein auf Adoption wartendes Gerät hat keine `device_up`-Zeitreihe und zählt nie als offline.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| UniFi device offline | Ein vom Controller adoptiertes UniFi-Gerät ist offline, isoliert oder bei der Adoption fehlgeschlagen (`dumbmonit_unifi_device_up`). | < 1 | 5 Min | Advisory (`warning`), eskaliert nach 1 h | 6 h |
| UniFi Internet down | Das UniFi-Gateway meldet keine Internetverbindung (`dumbmonit_unifi_internet_up`). Ohne UniFi-Gateway existiert die Zeitreihe nicht, und die Regel bleibt still. | < 1 | 3 Min | Warning (`critical`) | 1 h |
| UniFi WAN link down | Ein WAN-Link des UniFi-Gateways ist down (`dumbmonit_unifi_wan_link_up`): Mit zwei Links geht die Redundanz verloren, während nichts kaputt zu sein scheint. Ein deaktivierter Port hat keine Zeitreihe. | < 1 | 5 Min | Advisory (`warning`) | 6 h |
| UniFi gateway CPU high | Der Prozessor des UniFi-Gateways war fünfzehn Minuten lang über 90 % (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); löst sich unter 80 %. | > 90 % | 15 Min | Advisory (`warning`) | 6 h |
| UniFi device waiting for adoption | Ein Gerät ist im Netzwerk aufgetaucht und wartet darauf, in UniFi adoptiert zu werden (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 Min | Info (`info`) | 24 h |
| UniFi firmware update available | Ein Firmware-Update ist für UniFi-Geräte verfügbar (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 T |
| UniFi alarms | UniFi hat in den letzten 24 Stunden kritische Ereignisse protokolliert oder Alarme ausgelöst (`dumbmonit_unifi_alarms`). Nur mit einem View-Only-Konto gelesen. | > 0 | 5 Min | Advisory (`warning`) | 24 h |

### Home Assistant

[Geräteseite](../devices/homeassistant.md).

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Home Assistant in recovery mode | Home Assistant im Recovery-Modus gestartet: seine Konfiguration konnte nicht geladen werden, und Automatisierungen laufen nicht (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 Min | Warning (`critical`) | 1 h |
| Home Assistant not running | Home Assistant antwortet, hat aber seit zehn Minuten nicht mit dem Starten fertig (`dumbmonit_homeassistant_running`). | < 1 | 10 Min | Advisory (`warning`) | 6 h |
| Home Assistant entities went unavailable | Mindestens fünf Entitäten mehr sind nicht verfügbar als vor einer Stunde: Eine Integration, ein Hub oder ein Funkmodul ist ausgefallen (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). Der Anstieg zählt, nicht die Gesamtzahl, die eine echte Installation nie bei null hat. | ≥ 5 | 10 Min | Advisory (`warning`) | 6 h |
| Home Assistant battery low | Ein von Home Assistant gemeldeter Akku ist schwach (`dumbmonit_homeassistant_battery_low`, eine Zeitreihe pro Akku unter dem Schwellenwert des Geräts, standardmäßig 20 %; ihr Wert ist der Stand). | ≥ 0 % | 30 Min | Info (`info`) | 3 T |
| Home Assistant update available | Home Assistant hat ausstehende Updates: Core, Betriebssystem, Add-ons oder Gerätefirmware (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 T |
| Home Assistant repair to address | Home Assistant hat eine offene Reparatur der Stufe Error oder Critical (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 Min | Advisory (`warning`) | 24 h |

### VMware vSphere

[Geräteseite](../devices/vsphere.md). Host-Zeitreihen tragen `host`, VM-Zeitreihen `vm`, Datastore-Zeitreihen `datastore`, daher benennt die Benachrichtigung das Objekt.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| vSphere host not responding | Ein vSphere-Host antwortet vCenter nicht (`dumbmonit_vsphere_host_connection_state`: 2 not responding). Ein absichtlich getrennter Host (1) feuert nicht. | ≥ 2 | 3 Min | Warning (`critical`) | 1 h |
| vSphere host health red | vSphere meldet die Gesamtgesundheit eines Hosts als rot (`dumbmonit_vsphere_host_status == 2`). Grau bedeutet unbekannt und feuert nicht. | > 0 | 5 Min | Warning (`critical`) | 1 h |
| vSphere host left in maintenance | Ein vSphere-Host ist seit einem Tag im Wartungsmodus: seine VMs wurden verschoben, und er könnte dort vergessen worden sein (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| vSphere datastore almost full | Ein vSphere-Datastore ist zu 90 % oder mehr voll (`dumbmonit_vsphere_datastore_used_percent`); löst sich unter 85 %. | ≥ 90 % | 15 Min | Advisory (`warning`), eskaliert nach 24 h | 6 h |
| vSphere datastore inaccessible | Ein vSphere-Datastore ist nicht zugänglich: Die darauf gespeicherten VMs können nicht laufen (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 Min | Warning (`critical`) | 1 h |
| vSphere VM tools not running | VMware Tools sind installiert, aber laufen nicht in einer eingeschalteten VM: der Gast könnte eingefroren sein (`dumbmonit_vsphere_vm_tools_status == 2`). VMs ohne Tools feuern nicht. | > 0 | 30 Min | Info (`info`) | 24 h |
| vSphere red alarm | vSphere hat einen roten Alarm ausgelöst, den niemand bestätigt hat (`dumbmonit_vsphere_alarm{status="red"}`, eine Zeitreihe pro Alarm, benannt nach dem Alarm und dem Objekt). | > 0 | 5 Min | Advisory (`warning`) | 6 h |

### Datenbanken, Message Broker und Security Engine

Redis / Valkey, MongoDB, RabbitMQ und CrowdSec ([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md), [RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)). Zähler laufen über `increase_prometheus` oder `changes_prometheus`, daher feuert nichts bei der ersten Messung eines Geräts.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Redis memory near maxmemory | Speichernutzung über 90 % von `maxmemory` (`dumbmonit_redis_memory_used_percent`); löst sich unter 85 %. Keine Zeitreihe ohne `maxmemory`. | > 90 % | 10 Min | Advisory (`warning`) | 6 h |
| Redis refusing connections | Client-Verbindungen in den letzten 15 Minuten abgelehnt: `maxclients` erreicht (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 Min | Warning (`critical`) | 1 h |
| Redis replication link down | Eine Replik hat die Verbindung zum Primary verloren (`dumbmonit_redis_master_link_up`). | < 1 | 2 Min | Warning (`critical`) | 1 h |
| Redis replica lagging | Vom Primary aus gesehen, hat eine Replik seit mehr als 30 Sekunden nicht bestätigt (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 Min | Advisory (`warning`) | 6 h |
| Redis snapshot failed | Der letzte `BGSAVE` ist fehlgeschlagen; mit `stop-writes-on-bgsave-error` werden Schreibvorgänge abgelehnt (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 Min | Warning (`critical`) | 1 h |
| Redis append-only file write failed | Der letzte AOF-Schreibvorgang ist fehlgeschlagen (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 Min | Warning (`critical`) | 1 h |
| MongoDB replica set without primary | Kein Mitglied ist Primary: kein Schreibvorgang wird angenommen (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 Min | Warning (`critical`) | 1 h |
| MongoDB member unreachable | Ein Replica-Set-Mitglied ist für die anderen nicht erreichbar (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 Min | Warning (`critical`) | 1 h |
| MongoDB replication lag | Ein Secondary ist mehr als eine Minute hinter dem Primary (`dumbmonit_mongodb_replset_member_lag_seconds`); löst sich unter 30 s. | > 60 s | 10 Min | Advisory (`warning`) | 6 h |
| MongoDB connections near the limit | Mehr als 80 % der verfügbaren Verbindungen genutzt (`dumbmonit_mongodb_connections_used_percent`); löst sich unter 75 %. | > 80 % | 10 Min | Advisory (`warning`) | 6 h |
| MongoDB cache under pressure | Mehr als 20 % des WiredTiger-Caches sind dirty (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); löst sich unter 15 %. | > 20 % | 10 Min | Advisory (`warning`) | 6 h |
| MongoDB internal errors | Reguläre Assertions in der letzten Stunde (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 Min | Advisory (`warning`) | 24 h |
| RabbitMQ memory or disk alarm | Ein Knoten hat einen Ressourcenalarm ausgelöst: jeder Publisher ist blockiert (`dumbmonit_rabbitmq_alarm`, eine Zeitreihe pro Knoten und Ressource). | > 0 | 1 Min | Warning (`critical`) | 1 h |
| RabbitMQ node down | Ein Cluster-Knoten läuft nicht (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 Min | Warning (`critical`) | 1 h |
| RabbitMQ network partition | Ein Knoten sieht eine Netzwerkpartition (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 Min | Warning (`critical`) | 1 h |
| RabbitMQ queue without consumer | Eine Warteschlange enthält Nachrichten und hat keinen Consumer (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); der Wert ist die Anzahl wartender Nachrichten. | > 0 | 15 Min | Advisory (`warning`) | 6 h |
| RabbitMQ queue unavailable | Eine Warteschlange läuft nicht: Quorum-Warteschlange ohne Mehrheit, oder ihr Knoten ist down (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 Min | Advisory (`warning`) | 6 h |
| CrowdSec Local API down | Die Local API antwortet nicht auf `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 Min | Warning (`critical`) | 1 h |
| CrowdSec bouncer stopped pulling | Die Anfragen eines Bouncers haben sich in 30 Minuten nicht geändert (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 Min | Advisory (`warning`) | 6 h |
| CrowdSec reads no logs | In sechs Stunden keine einzige Log-Zeile gelesen (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 Min | Advisory (`warning`) | 24 h |

### Reverse-Proxys und Domains

Traefik, Caddy, Nginx Proxy Manager und Domain-Registrierungen ([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md), [Nginx Proxy Manager](../devices/npm.md), [Domain-Ablauf](../devices/domain.md)), sowie die Option *Alert when the answer changes* der DNS-Prüfung ([Services](../devices/services.md#alerting-when-the-answer-changes)). Zeitreihen tragen `router`, `service`, `server`, `upstream`, `host` oder `cert`, daher benennt die Benachrichtigung, was fehlgeschlagen ist.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Traefik router disabled | Ein Router ist durch einen Konfigurationsfehler deaktiviert (fehlender Service oder Middleware, ungültige Regel): seine Sites werden nicht bedient (`dumbmonit_traefik_router_status`: 0 enabled, 1 warning, 2 disabled). | ≥ 2 | 5 Min | Advisory (`warning`) | 6 h |
| Traefik backend server down | Ein Server eines lastverteilten Services schlägt bei seinem Healthcheck fehl (`dumbmonit_traefik_server_up`). | < 1 | 5 Min | Advisory (`warning`) | 6 h |
| Traefik service without a server | Kein Server eines Services antwortet auf seinen Healthcheck: seine Sites erhalten 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 Min | Warning (`critical`) | 1 h |
| Traefik certificate not obtained | Router fragen einen Certificate-Resolver nach einem Zertifikat, das noch kein von Traefik gehaltenes Zertifikat abdeckt: Traefik liefert sein selbstsigniertes Standardzertifikat (`dumbmonit_traefik_resolver_routers_uncovered`, benötigt Traefiks `/api/certificates`). | > 0 | 1 h | Advisory (`warning`) | 24 h |
| Caddy upstream unhealthy | Ein Backend schlägt bei Caddys aktivem oder passivem Healthcheck fehl (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 Min | Advisory (`warning`) | 6 h |
| Caddy configuration reload failed | Caddy hat sein letztes Konfigurations-Reload abgelehnt und läuft noch mit dem vorherigen (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 Min | Advisory (`warning`) | 24 h |
| Nginx Proxy Manager host offline | nginx hat die Konfiguration eines aktivierten Hosts abgelehnt: der Host wird nicht bedient (`dumbmonit_npm_host_online`). | < 1 | 10 Min | Advisory (`warning`) | 6 h |
| Proxy certificate not renewed | Ein von Traefik gehaltenes Zertifikat, oder eines, das ein Nginx-Proxy-Manager-Host präsentiert, läuft in weniger als 14 Tagen ab: Die automatische Erneuerung startet bei 30, also schlägt sie fehl (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 T | 1 h | Advisory (`warning`) | 24 h |
| Reverse proxy answering 5xx | Mehr als 5 % der Antworten in zehn Minuten waren 5xx, bei mindestens fünfzig Anfragen (`increase_prometheus` von `dumbmonit_{traefik,caddy}_requests_5xx_total` gegenüber `…_requests_total`). Benötigt die Metriken des Proxys. | > 5 % | 10 Min | Advisory (`warning`) | 6 h |
| Domain expiring | Die Domain läuft in weniger als 30 Tagen ab (`dumbmonit_domain_expiry_days >= 7`). | < 30 T | 1 h | Advisory (`warning`) | 24 h |
| Domain about to expire | Die Domain läuft in weniger als 7 Tagen ab, oder ist abgelaufen (`dumbmonit_domain_expiry_days`). | < 7 T | 1 h | Warning (`critical`) | 12 h |
| Domain on hold | Die Registry hat die Domain auf `clientHold` oder `serverHold` gesetzt: sie ist aus dem DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 Min | Warning (`critical`) | 6 h |
| Domain in redemption | Die Domain ist in `redemptionPeriod`, `pendingDelete` oder `pendingRestore`: sie ist abgelaufen (`dumbmonit_domain_redemption`). | > 0 | 30 Min | Warning (`critical`) | 12 h |
| DNS answer changed | Die Antwort einer DNS-Prüfung mit *Alert when the answer changes* unterscheidet sich von der vorherigen (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Löst sich eine Stunde später von selbst. | > 0 | 1 Min | Advisory (`warning`) | 24 h |

### Website-Änderungen

Siehe [Website-Änderungen](../devices/webchange.md). Die Geräteseite zeigt jede Änderung vorher und nachher.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Website changed | Die letzte Prüfung einer beobachteten Seite oder Site hat geänderte, erschienene oder verschwundene Seiten gefunden (`dumbmonit_webchange_last_check_changes`). Löst sich bei der nächsten Prüfung, die keine findet. | > 0 | 30 s | Info (`info`) | keine |

### Kubernetes

Siehe [Kubernetes / k3s](../devices/kubernetes.md). Jede Zeitreihe ist mit
dem Knoten, oder dem Namespace und dem betroffenen Pod, Workload oder Claim
beschriftet.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Kubernetes node not ready | Ein Knoten ist nicht Ready: sein Kubelet meldet sich nicht mehr, und seine Pods werden vertrieben (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 Min | Warning (`critical`) | 6 h |
| Kubernetes node under pressure | Ein Knoten meldet Memory-, Disk- oder PID-Druck (`dumbmonit_k8s_node_pressure`, Label `condition`). | > 0 | 5 Min | Advisory (`warning`) | 6 h |
| Kubernetes pod crash looping | Ein Container ist in `CrashLoopBackOff` (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m])`: der Zustand verschwindet kurz bei jedem Neustartversuch). | > 0 | 10 Min | Warning (`critical`) | 6 h |
| Kubernetes pod stuck pending | Ein Pod ist seit fünfzehn Minuten Pending: Kein Knoten kann ihn aufnehmen, oder ein benötigtes Volume oder Image kommt nicht (`dumbmonit_k8s_pod_pending`). | > 0 | 15 Min | Advisory (`warning`) | 6 h |
| Kubernetes pod restarting | Ein Pod ist in der letzten Stunde mehr als fünfmal neu gestartet, ohne in `CrashLoopBackOff` zu sein (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 Min | Advisory (`warning`) | 6 h |
| Kubernetes workload missing replicas | Ein Deployment, StatefulSet oder DaemonSet hat weniger verfügbare Replikas als gefordert (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 Min | Advisory (`warning`), eskaliert nach 1 h | 6 h |
| Kubernetes volume claim pending | Ein von einem Pod benötigter PersistentVolumeClaim, oder einer, den ein Provisionierungsfehler betrifft, ist noch nicht gebunden (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 Min | Advisory (`warning`) | 6 h |

### Active Directory

Siehe [Active Directory](../devices/activedirectory.md). Der von DumbMonit
abgefragte Domänencontroller wird von *Device unreachable* abgedeckt; die
anderen Domänencontroller der Domäne von *Domain controller unreachable*
(Label `dc`).

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Domain controller unreachable | Ein Domänencontroller der Domäne antwortet nicht mehr auf seinem LDAP-Port (`dumbmonit_ad_dc_reachable == bool 0`). Ein von DumbMonit nicht auflösbarer Name erzeugt keine Zeitreihe und feuert nie. | > 0 | 5 Min | Warning (`critical`) | 6 h |
| Active Directory bind failed | Der Domänencontroller antwortet, lehnt aber das Dienstkonto ab: abgelaufenes Passwort, deaktiviertes oder gesperrtes Konto (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 Min | Advisory (`warning`) | 24 h |
| Privileged group membership changed | Ein Mitglied ist Domain Admins, Enterprise Admins, Schema Admins, Administrators oder einer Operators-Gruppe beigetreten oder hat sie verlassen, Verschachtelung eingeschlossen (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, Label `group`). Löst sich eine Stunde später von selbst. | > 0 | 1 Min | Advisory (`warning`) | 24 h |
| Active Directory replication failing | Der abgefragte Domänencontroller kann nicht von einem Partner replizieren (`dumbmonit_ad_replication_consecutive_failures`). Nur wenn das Konto den Replikationsstatus lesen darf. | > 0 | 30 Min | Advisory (`warning`) | 6 h |
| krbtgt password older than 180 days | Das krbtgt-Passwort wurde zuletzt vor mehr als 180 Tagen geändert (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 Tage | 10 Min | Info (`info`) | keine |

### pfSense

Siehe [pfSense](../devices/pfsense.md). Gateways sind mit `gateway` beschriftet, Schnittstellen mit `interface` und `descr`, Dienste mit `service`.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| pfSense gateway down | Ein pfSense-Gateway ist down: Seine Monitor-Adresse antwortet nicht mehr, und der darüber laufende Verkehr stoppt oder failovert (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 Min | Warning (`critical`) | 6 h |
| pfSense gateway losing packets | Ein pfSense-Gateway verliert mehr als 10 % der Proben an seine Monitor-Adresse: die Leitung ist beeinträchtigt (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 Min | Advisory (`warning`) | 6 h |
| pfSense gateway slow | Der Round-Trip zur Monitor-Adresse eines pfSense-Gateways ist seit fünfzehn Minuten über 500 ms geblieben (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 Min | Advisory (`warning`) | 6 h |
| pfSense interface without link | Eine aktivierte pfSense-Schnittstelle hat keinen Link: Kabel, Switch-Port oder Netzwerkkarte (`dumbmonit_pfsense_interface_down`). | > 0 | 5 Min | Advisory (`warning`) | 6 h |
| pfSense service stopped | Ein aktivierter pfSense-Dienst läuft nicht: DNS-Resolver, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 Min | Advisory (`warning`) | 6 h |
| pfSense disk nearly full | Die pfSense-Platte ist zu mehr als 90 % voll: Logs, Berichte und Paket-Updates werden nicht mehr geschrieben (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 Min | Advisory (`warning`) | 1 T |

### Unraid

Siehe [Unraid](../devices/unraid.md). Platten sind mit `disk` und `role` (`parity`, `data`, `cache`) beschriftet, Container mit `container`.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Unraid array stopped | Das Unraid-Array ist nicht gestartet: Freigaben, Container und VMs, die darauf leben, sind nicht verfügbar (`dumbmonit_unraid_array_started`). | < 1 | 10 Min | Warning (`critical`) | 6 h |
| Unraid disk disabled or missing | Eine Unraid-Platte ist deaktiviert, fehlt oder ist ungültig: das Array läuft degradiert, und ein weiterer Ausfall verliert Daten (`dumbmonit_unraid_disk_ok`). | < 1 | 5 Min | Warning (`critical`) | 6 h |
| Unraid disk read errors | Eine Unraid-Platte hat seit dem Start des Arrays Lesefehler gemeldet: sie könnte beim nächsten deaktiviert werden (`dumbmonit_unraid_disk_errors`). | > 0 | 5 Min | Advisory (`warning`) | 1 T |
| Unraid disk too hot | Eine Unraid-Array- oder Paritätsplatte ist seit fünfzehn Minuten über 55 °C geblieben (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 Min | Advisory (`warning`) | 6 h |
| Unraid parity check found errors | Der letzte Unraid-Paritätscheck ist fehlgeschlagen oder hat Sync-Fehler gefunden: die Parität stimmt nicht mehr mit den Daten überein (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 Min | Advisory (`warning`) | 1 T |
| Unraid parity check overdue | Der letzte Unraid-Paritätscheck ist mehr als 40 Tage alt: ein monatlicher Check wurde verpasst (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 T | 1 h | Advisory (`warning`) | 1 T |
| Unraid cache nearly full | Ein Unraid-Cache-Pool ist zu mehr als 90 % voll: Schreibvorgänge fallen auf das langsame Array zurück, oder schlagen fehl (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 Min | Advisory (`warning`) | 1 T |
| Unraid container stopped | Ein zum Start mit dem Array vorgesehener Docker-Container läuft nicht (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 Min | Advisory (`warning`) | 6 h |

### Veeam Backup & Replication

Siehe [Veeam Backup & Replication](../devices/veeam.md). Jobs sind mit `job` und `type` beschriftet, Repositories mit `repository`.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Veeam job failed | Der letzte Lauf eines aktivierten Veeam-Jobs ist fehlgeschlagen: das Geschützte hat keinen neuen Wiederherstellungspunkt (`dumbmonit_veeam_job_failed`). | > 0 | 5 Min | Warning (`critical`) | 1 T |
| Veeam job ended with a warning | Der letzte Lauf eines aktivierten Veeam-Jobs endete mit einer Warnung: manche Objekte sind möglicherweise nicht geschützt (`dumbmonit_veeam_job_warning`). | > 0 | 5 Min | Advisory (`warning`) | 1 T |
| Veeam repository nearly full | Ein Veeam-Backup-Repository ist zu mehr als 90 % voll: die nächsten Jobs scheitern mangels Platz (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 Min | Advisory (`warning`) | 1 T |
| Veeam license expiring | Die Veeam-Lizenz läuft in weniger als 30 Tagen ab: Jobs stoppen am Ende der Kulanzzeit (`dumbmonit_veeam_license_expiry_seconds`). | < 30 T | 1 h | Advisory (`warning`) | 1 T |

### Tailscale

Siehe [Tailscale](../devices/tailscale.md). Geräte sind mit `device` beschriftet, dem ersten Label ihres MagicDNS-Namens.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Tailscale device offline | Ein Tailscale-Gerät, das online bleiben muss (standardmäßig ein getaggtes Gerät), ist seit mehr als 15 Minuten getrennt (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 Min | 5 Min | Advisory (`warning`) | 6 h |
| Tailscale node key expiring | Der Node-Key eines Tailscale-Geräts läuft in weniger als 14 Tagen ab: Es verlässt das Tailnet, bis jemand es erneut anmeldet (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 T | 1 h | Advisory (`warning`) | 1 T |
| Tailscale device awaiting approval | Ein Gerät ist dem Tailnet beigetreten und wartet auf Genehmigung: genehmigen Sie es, wenn erwartet, entfernen Sie es sonst (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 Min | Advisory (`warning`) | 1 T |
| Tailscale client update | Ein Tailscale-Gerät, das online bleiben muss, läuft mit einem Client, für den ein Update verfügbar ist (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Info (`info`) | 7 T |

### FortiGate

Siehe [FortiGate](../devices/fortigate.md). Tunnel sind mit `tunnel` beschriftet, Schnittstellen mit `interface` und `alias`, Lizenzen mit `license`.

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| FortiGate IPsec tunnel down | Keine Phase-2 eines FortiGate-Site-to-Site-IPsec-Tunnels ist up: die dahinterliegende Site ist abgeschnitten (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 Min | Warning (`critical`) | 6 h |
| FortiGate interface without link | Eine aktivierte FortiGate-Schnittstelle mit einer Adresse hat keinen Link: Kabel, Transceiver oder das vorgeschaltete Gerät (`dumbmonit_fortigate_interface_down`). | > 0 | 5 Min | Advisory (`warning`) | 6 h |
| FortiGate HA out of sync | Die Mitglieder eines FortiGate-HA-Clusters haben nicht mehr dieselbe Konfigurationsprüfsumme: ein Failover würde eine andere Konfiguration ausführen (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 Min | Advisory (`warning`) | 6 h |
| FortiGate HA member lost | Ein FortiGate-HA-Cluster hat weniger Mitglieder als in den letzten 24 Stunden: es gibt keinen Standby mehr (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 Min | Warning (`critical`) | 6 h |
| FortiGate memory near conserve mode | Der FortiGate-Speicher ist seit zehn Minuten über 85 % geblieben: bei 88 % (standardmäßig) geht er in den Conserve-Modus und hört auf, neue Sitzungen zu prüfen (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 Min | Advisory (`warning`) | 6 h |
| FortiGate CPU high | Die FortiGate-CPU ist seit fünfzehn Minuten über 90 % geblieben (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 Min | Advisory (`warning`) | 6 h |
| FortiGate licence expiring | Eine FortiGuard- oder FortiCare-Lizenz der FortiGate läuft in weniger als 30 Tagen ab, oder ist abgelaufen (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 T | 1 h | Advisory (`warning`) | 1 T |
| FortiGate firmware update | FortiGuard bietet eine neuere Patch-Version des von der FortiGate ausgeführten FortiOS-Zweigs an (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Info (`info`) | 7 T |

### Sophos Firewall

Siehe [Sophos Firewall](../devices/sophos.md).

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Sophos interface without link | Eine eingeschaltete und einer Zone zugewiesene Sophos-Firewall-Schnittstelle hat keinen Link (`dumbmonit_sophos_interface_down`). | > 0 | 5 Min | Advisory (`warning`) | 6 h |

### Hyper-V

Gemeldet vom Windows-Agent auf einem Hyper-V-Host; siehe [Agent](../devices/agent.md#hyper-v-hosts).

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Hyper-V VM health critical | Hyper-V meldet eine virtuelle Maschine in kritischer Gesundheit: eine benötigte Ressource, meistens Plattenplatz, ist erschöpft (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 Min | Warning (`critical`) | 6 h |
| Hyper-V host CPU high | Die logischen Prozessoren des Hyper-V-Hosts waren fünfzehn Minuten lang mehr als 90 % ausgelastet, Gäste eingeschlossen (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 Min | Advisory (`warning`) | 6 h |
| Hyper-V virtual disk errors | Eine Hyper-V-virtuelle Platte hat in der letzten Stunde I/O-Fehler gemeldet (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 Min | Advisory (`warning`) | 6 h |

### Security Score

Vom Server alle 15 Minuten für jedes Gerät berechnet, dessen Art Sicherheitsprüfungen hat, und gespeichert als `dumbmonit_security_score` (0–100). Siehe [Security Score](../using/security-score.md).

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Security score dropped | Der Score ist um 10 Punkte oder mehr gegenüber seinem besten Wert der letzten 24 Stunden gefallen (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): eine Prüfung hat angefangen zu scheitern. | ≥ 10 Pkt | 30 Min | Advisory (`warning`) | 24 h |
| Security score low | Der Score liegt unter 40, Note F (`dumbmonit_security_score`). Heben oder senken Sie den Schwellenwert, um ihn an Ihre eigene Erwartung anzupassen. | < 40 | 1 h | Info (`info`) | 7 T |

### DumbMonit selbst

| Regel | Was | Standard-Schwellenwert | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| DumbMonit backup did not run | Das geplante lokale Backup der DumbMonit-Datenbank ist seit mehr als zwei Tagen nicht gelaufen (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 T | 1 h | Advisory (`warning`) | 24 h |

Jede eingebaute Regel gilt für alle Geräte und für jeden aktivierten Kanal.

## Eine Regel in der UI bearbeiten {#editing-a-rule-in-the-ui}

Auf der Seite **Alerts** listet der Abschnitt *Rules* jede Regel mit ihrem
Schweregrad, einer *Built-in*-Markierung und, wenn relevant, wie viele Alarme
sie gerade feuert. Jede Regel hat:

- einen Schalter zum **Aktivieren oder Deaktivieren**;
- **Edit**, der einen Inline-Editor für Name, Beschreibung, Operator,
  Schwellenwert, Haltezeit (`for`), Schweregrad, Erinnerung (`repeat every`)
  und Eskalation (`escalate after`) aufklappt. Die Abfrage einer eingebauten
  Regel wird angezeigt, aber nicht bearbeitbar; die Abfrage eigener Regeln
  schon. Baseline-Regeln zeigen ihre Erkennungsparameter nur lesbar;
- **Delete**, nur für eigene Regeln.

## Gerätespezifische Überschreibungen {#per-device-overrides}

Eine Regel kann für ein Gerät angepasst werden, ohne die Regel selbst
anzurühren: ein anderer Schwellenwert, eine andere Rückstellschwelle, oder
für genau dieses Gerät abgeschaltet. Der Editor jeder Regel hat ein Panel
**Per-device overrides**, um eine solche direkt hinzuzufügen oder zu
entfernen.

Die Ein-Klick-Aktion **Ignore** auf einem Alarm (in der Liste *Needs you*
oder der Geräteseite des Alarms) ist derselbe Mechanismus, geschrieben mit
`enabled: false`: Das Gerät hört sofort und reversibel auf, diese Regel zu
tragen. Die Geräteseite listet, was sie ignoriert, mit **Stop ignoring**;
die Regel selbst zeigt *Ignored on N devices* unter Alerts → Rules, wo
dasselbe Panel für gerätespezifische Überschreibungen sie entfernt. Siehe
[Gerätespezifische Überschreibungen](../reference/api.md#per-device-overrides)
für die API.

## Eine Schwellenwertregel erstellen {#creating-a-threshold-rule}

Klicken Sie auf **New rule**. Das Formular braucht:

| Feld | Bedeutung |
|---|---|
| Name | Wird in Alarmen und Benachrichtigungen angezeigt. |
| Query | Ein MetricsQL-Ausdruck, pro Gerät ausgewertet. Beispiel: `dumbmonit_cpu_usage_percent`. |
| Operator | `>`, `>=`, `<` oder `<=`. |
| Threshold | Die Zahl, mit der das Abfrageergebnis verglichen wird. |
| Severity | Info, Advisory oder Warning. |
| Hold for | Sekunden, die die Bedingung anhalten muss, bevor sie feuert. Eine Haltezeit größer null fängt einen isolierten schlechten Messwert ab. |

Jede von der Abfrage zurückgegebene Zeitreihe wird zu einem potenziellen
Alarm, dem durch ihr `target`-Label benannten Gerät zugeordnet. Aggregieren
Sie mit `avg by (target, host) (…)`, wenn Sie einen Alarm pro Gerät statt
einen pro Kern oder Schnittstelle wollen. Die Identität des Alarms
ignoriert die Labels `host` und `tag_*`, solange `target` vorhanden ist,
daher behält ein umbenanntes Gerät seine Alarme; eine Zeitreihe, deren
`target` zu keinem vorhandenen, aktivierten Gerät passt, wird ignoriert.

## Die Abfragesprache {#the-query-language}

Abfragen sind [MetricsQL](https://docs.victoriametrics.com/metricsql/), das
PromQL-Superset von VictoriaMetrics. Die Metriken sind die im
[Metrik-Referenz](../reference/metrics.md) beschriebenen `dumbmonit_*`-Zeitreihen,
und jede Zeitreihe trägt ein `target`-Label (die Geräte-ID) und ein
`host`-Label (ihren Namen).

Einige von den eingebauten Regeln genutzte Muster:

```
# Age of the last sample: what "unreachable" means
time() - tlast_over_time(dumbmonit_up[7d])

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

!!! tip "Erst eine Abfrage testen"
    `GET /api/metrics/query?query=…` gibt zurück, was die Regel auswerten
    würde (siehe die [API-Referenz](../reference/api.md#metrics)). In der
    Entwicklung veröffentlicht das Overlay das eingebettete VictoriaMetrics
    auf `:8428` des Hosts, mit seiner Abfrage-UI.

## Regelarten in der API {#rule-kinds-in-the-api}

`kind` ist `threshold` (Vergleich mit einem Schwellenwert), `predict`
(derselbe Vergleich, auf einer `predict_linear`-Abfrage; als Vorhersage
angezeigt) oder `anomaly` (saisonale Baseline; `params` hält `k`, `alpha`,
`mad_floor_abs`, `mad_floor_rel`, `min_samples`). Die UI erstellt
`threshold`-Regeln; die anderen beiden Arten können über die API erstellt
werden.
