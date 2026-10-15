# Regeln {#rules}

Regeln werden beim ersten Start eingefügt und gehören dann Ihnen: DumbMonit überschreibt nie eine
von Ihnen geänderte Regel, es erstellt nur fehlende neu. Integrierte Regeln lassen sich bearbeiten
und deaktivieren, nicht löschen.

## Integrierte Regeln {#built-in-rules}

Schweregrade werden mit dem Wort der Oberfläche angezeigt; der API-Wert steht in Klammern. Eine
Regel, deren Metrik kein Gerät erzeugt, hat keine Serie und löst nie aus, daher wird jede
untenstehende Regel auf jeder Instanz aktiviert ausgeliefert.

### Jedes Gerät {#every-device}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Gerät nicht erreichbar | Seit mehr als drei Minuten keine Messung empfangen. `bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))` | > 180 s | 1 Min. | Warnung (`critical`) | 6 h |
| CPU hoch | CPU-Last dauerhaft über 90 %, pro Gerät über SNMP-, Proxmox- und Agent-Quellen gemittelt; löst sich unter 85 %. | > 90 % | 10 Min. | Hinweis (`warning`), eskaliert nach 1 h | 6 h |
| Ungewöhnliche CPU | CPU-Last spürbar anders als üblich zu dieser Uhrzeit und diesem Wochentag (saisonale Baseline). | Score > 3,5 | 15 Min. | Info (`info`) | 6 h |
| Platte fast voll | Dateisystem zu 90 % oder mehr voll, pro Mountpunkt (SNMP, Proxmox-Storages und Root-Dateisysteme); löst sich unter 88 %. | ≥ 90 % | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |
| Dateisystem fast voll | Bei dieser Rate ist das Dateisystem innerhalb von vier Tagen voll (`predict_linear` über 6 h; nur wachsende Dateisysteme). | ≥ 100 % | 30 Min. | Hinweis (`warning`) | 6 h |

### USV {#ups}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| USV an der Batterie | Die USV versorgt die Last aus der Batterie (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Warnung (`critical`) | 15 Min. |
| USV-Batterie schwach | Die USV-Batterie ist schwach oder erschöpft (`dumbmonit_ups_battery_status`, 3 = schwach, 4 = erschöpft). | ≥ 3 | 1 Min. | Warnung (`critical`) | 30 Min. |

### Netzwerkports (SNMP) {#network-ports-snmp}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Port sammelt Fehler | Ein Netzwerkport hat in der letzten Stunde mehr als fünfzig Fehler protokolliert: meist ein defektes Kabel, eine verschmutzte Faser oder ein Duplex-Mismatch (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; in der ersten Stunde eines neuen Geräts löst nichts aus, sodass vor dem Hinzufügen gezählte Fehler nicht als neu gemeldet werden). | > 50 | 15 Min. | Hinweis (`warning`) | 24 h |
| Port flattert | Ein Netzwerkport ist in dreißig Minuten mehr als zweimal ausgefallen und wieder hochgekommen (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Ein absichtlich deaktivierter Port hat keine Serie und löst nie aus. | > 4 | 5 Min. | Hinweis (`warning`) | 1 h |

### Serverhardware (Redfish) {#server-hardware-redfish}

Zustandsserien sind 0 (OK), 1 (Warnung) oder 2 (Kritisch). Ein leerer Steckplatz (`Absent`) oder ein deaktivierter hat keine Serie, sodass ein mit einem von zwei Netzteilen ausgelieferter Server nie auslöst.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Server-Lüfter ausgefallen | Der Management-Controller meldet einen Lüfter als ausgefallen (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 Min. | Warnung (`critical`) | 6 h |
| Server-Temperatur über kritisch | Ein Temperatursensor liegt über der kritischen Schwelle, die der Controller selbst deklariert (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| Netzteil-Redundanz verloren | Die Netzteile sind nicht mehr redundant: ein weiterer Ausfall, und der Server geht herunter (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 Min. | Hinweis (`warning`), eskaliert nach 1 h | 6 h |
| Netzteil ausgefallen | Der Controller meldet ein Netzteil als ausgefallen (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 Min. | Warnung (`critical`) | 6 h |
| Laufwerksausfall vorhergesagt | Ein Laufwerk meldet einen vorhergesagten Ausfall: Ersetzen Sie es, solange es noch funktioniert (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 Min. | Hinweis (`warning`) | 24 h |
| Serverzustand kritisch | Der Controller meldet den eigenen Zustand des Systems als Kritisch (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 Min. | Warnung (`critical`) | 6 h |

### Uptime-Monitore und Heartbeats {#uptime-monitors-and-heartbeats}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Dienst ausgefallen | Der Dienst hat drei Minuten lang nicht korrekt geantwortet (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 Min. | Warnung (`critical`) | 30 Min. |
| Heartbeat verpasst | Der Job hat sich nicht innerhalb seines erwarteten Intervalls plus Karenzzeit gemeldet oder selbst einen Fehler gemeldet (`dumbmonit_probe_success{probe="push"} == bool 0`; die Verzögerung wird pro Gerät festgelegt, siehe [Heartbeat](../devices/push.md)). | > 0 | 2 Min. | Hinweis (`warning`) | 6 h |
| Dienst flattert | Der Dienst hat in dreißig Minuten mehr als sechsmal den Zustand gewechselt (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 Min. | Hinweis (`warning`) | 1 h |
| Langsamer Dienst | Der Dienst braucht mehr als drei Sekunden zum Antworten; fehlgeschlagene Prüfungen bleiben außen vor (`dumbmonit_probe_duration_seconds and dumbmonit_probe_success == 1`). | > 3 s | 10 Min. | Hinweis (`warning`) | 6 h |
| Zertifikat läuft bald ab | Das Zertifikat läuft in weniger als vierzehn Tagen ab. | < 14 T. | 1 h | Hinweis (`warning`) | 24 h |
| Zertifikat abgelaufen | Das Zertifikat ist abgelaufen. | < 0 T. | 5 Min. | Warnung (`critical`) | 24 h |

### Docker-Container {#docker-containers}

Vom Agenten gemeldet; siehe [Agent](../devices/agent.md). Der Alarm nennt den Container.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Container gestoppt | Der Container ist seit zwei Minuten gestoppt (`dumbmonit_container_up == bool 0`). Ein Container, den Docker nicht neu starten würde, bleibt außen vor (`dumbmonit_container_stop_expected = 1`): ein abgeschlossenes einmaliges `docker run`, ein nie gestarteter Container oder einer, der von Hand ohne Restart-Policy gestoppt wurde. Ein Absturz (jeder andere Exit-Code), ein Out-of-Memory-Kill oder ein Container mit `always` / `unless-stopped` zählt weiterhin. | > 0 | 2 Min. | Hinweis (`warning`) | 1 h |
| Container ungesund | Der Health-Check des Containers schlägt seit drei Minuten fehl (`dumbmonit_container_health == 2`; 0 keiner, 1 gesund, 3 startet). Ein gestoppter Container bleibt außen vor: Docker behält sein letztes Urteil, das nichts mehr bedeutet. | > 0 | 3 Min. | Hinweis (`warning`) | 1 h |
| Container startet immer wieder neu | Der Container ist in fünfzehn Minuten dreimal oder öfter neu gestartet (`increase(dumbmonit_container_restart_count[15m])`): Er ist selten lange genug gestoppt für die vorherige Regel. | ≥ 3 | 1 Min. | Hinweis (`warning`) | 1 h |
| Container-Update verfügbar | In der Registry ist ein neueres Image für diesen Container verfügbar (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Plakar-Backups {#plakar-backups}

Vom Agenten gemeldet, pro Kloset und pro Quelle. Keine dieser Serien existiert, solange Plakar
nicht erkannt ist, sodass keine der beiden Regeln auf einem Rechner ohne Plakar auslösen kann.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Plakar-Backup zu alt | Seit mehr als zwei Tagen kein neuer Plakar-Snapshot für diese Quelle (`dumbmonit_backup_last_success_seconds`). | > 2 T. | 1 h | Hinweis (`warning`) | 24 h |
| Plakar-Backup fehlgeschlagen | Der neueste Snapshot hat Fehler (`dumbmonit_backup_last_status{source!="*"} < 1`). | < 1 | 10 Min. | Warnung (`critical`) | 24 h |
| Plakar-Kloset nicht lesbar | Der Agent kann das Kloset nicht öffnen: Speicher nicht erreichbar, Passphrase oder Plugin fehlt; das Log des Agenten sagt einmalig, was davon (`dumbmonit_backup_last_status{source="*"} < 1`). | < 1 | 30 Min. | Hinweis (`warning`) | 24 h |

### restic- und Borg-Backups {#restic-and-borg-backups}

Vom Agenten für jedes unter `restic_repos` oder `borg_repos` in agent.yaml deklarierte Repository
gemeldet; siehe [Agent](../devices/agent.md#restic-and-borg-backups). Ohne deklariertes Repository
gibt es keine Serie, sodass keine der beiden Regeln auslösen kann.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| restic/Borg-Backup zu alt | Seit mehr als 48 Stunden kein neuer restic- oder Borg-Snapshot in diesem Repository (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Hinweis (`warning`) | 24 h |
| restic/Borg-Repository nicht erreichbar | Der Agent kann das Repository nicht lesen: nicht erreichbar, Passwort abgelehnt oder Binary fehlt; das Log des Agenten sagt, was zutrifft (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 Min. | Hinweis (`warning`) | 24 h |

### WireGuard-Tunnel {#wireguard-tunnels}

Vom Agenten gemeldet; siehe [Agent](../devices/agent.md#wireguard-tunnels).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| WireGuard-Peer still | Ein Peer mit persistentem Keepalive hat seit mehr als 15 Minuten keinen Handshake abgeschlossen: Der Tunnel ist unten (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Bei einem Peer, der nie verbunden war, zählt die Stille ab dem Moment, in dem der Agent ihn zum ersten Mal sah. Peers ohne Keepalive (ein Smartphone, ein Laptop) ruhen tagelang und bleiben außen vor; ändern Sie die Schwelle, um N zu ändern, oder streichen Sie den Teil `and on …`, um jeden Peer zu überwachen. | > 15 Min. | 5 Min. | Hinweis (`warning`) | 6 h |

### IPsec-Tunnel {#ipsec-tunnels}

Vom Agenten gemeldet; siehe [Agent](../devices/agent.md#ipsec-tunnels). Die
Serie existiert nur für eine in der strongSwan-Konfiguration deklarierte
CHILD_SA, sodass die Regel auch bei einem Tunnel auslöst, der nie oben war,
nicht nur bei einem, der abgebrochen ist.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| IPsec-Tunnel ausgefallen | Ein konfigurierter IPsec-Tunnel hat keine installierte Sitzung (`dumbmonit_agent_ipsec_tunnel_up < 1`). | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |

### Internet-Speedtest {#internet-speed-test}

Vom Agenten gemeldet; siehe [Agent](../devices/agent.md#internet-speed-test).
Der Test ist standardmäßig aus, und jede Regel bleibt still, bis die
passende Schwelle in `agent.yaml` gesetzt ist: Niemand kann erraten, was
"langsam" für Ihre Leitung bedeutet.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Internet-Downloadrate niedrig | Der letzte Test maß eine Downloadrate unter `speedtest_min_download_mbps` (`dumbmonit_agent_speedtest_download_min_bytes_per_second - dumbmonit_agent_speedtest_download_bytes_per_second > 0`). | > 0 | 10 Min. | Hinweis (`warning`) | 24 h |
| Internet-Uploadrate niedrig | Dasselbe für `speedtest_min_upload_mbps`. | > 0 | 10 Min. | Hinweis (`warning`) | 24 h |
| Internet-Latenz hoch | Der letzte Test maß eine Latenz über `speedtest_max_latency_ms`. | > 0 | 10 Min. | Hinweis (`warning`) | 24 h |

### Agent-Platten, ZFS und Sensoren {#agent-disks-zfs-and-sensors}

Vom Agenten gemeldet; siehe [Agent](../devices/agent.md).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Platten-SMART fehlerhaft | Die eigene SMART-Selbstbewertung der Platte meldet sie als fehlerhaft (`dumbmonit_agent_disk_smart_ok == bool 0`). Ersetzen Sie sie und prüfen Sie die Backups. | > 0 | 10 Min. | Warnung (`critical`) | 24 h |
| ZFS-Pool degradiert | Ein ZFS-Pool ist nicht mehr gesund: Ein Gerät fehlt, ist fehlerhaft oder entfernt (`dumbmonit_agent_zfs_pool_health`: 0 online, 1 degradiert, 2 alles andere). | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| ZFS-Scrub hat Fehler gefunden | Der letzte ZFS-Scrub hat Fehler auf diesem Pool gefunden, oder Dateien sind dauerhaft verloren (`dumbmonit_agent_zfs_pool_scrub_errors or dumbmonit_agent_zfs_pool_data_errors`). Der Pool kann `ONLINE` bleiben. | > 0 | 10 Min. | Warnung (`critical`) | 24 h |
| Temperatur über kritisch | Ein Sensor liegt über der kritischen Temperatur, die seine eigene Hardware deklariert (`dumbmonit_agent_sensor_temperature_celsius >= dumbmonit_agent_sensor_temperature_critical_celsius`); kein fester Wert. | > 0 | 5 Min. | Warnung (`critical`) | 6 h |

### Proxmox VE {#proxmox-ve}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Backup zu alt | Seit mehr als sieben Tagen kein erfolgreiches Proxmox-VE-Backup (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 T. | 1 h | Hinweis (`warning`) | 24 h |
| VM oder Container gestoppt | Der Gast lief innerhalb der letzten zwei Stunden und ist seit fünf Minuten gestoppt (`1 - dumbmonit_proxmox_guest_running`, verundet mit `max_over_time(…[2h]) == 1`). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |
| VM- oder Container-CPU hoch | Ein Gast hat fünfzehn Minuten lang mehr als 90 % seiner zugewiesenen Kerne genutzt (`dumbmonit_proxmox_guest_cpu_percent`). Die Benachrichtigung nennt den Gast, `nextcloud (202)`. | > 90 % | 15 Min. | Hinweis (`warning`) | 6 h |
| VM- oder Container-Speicher hoch | Ein Gast hat zehn Minuten lang mehr als 95 % seines Speichers genutzt (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 Min. | Hinweis (`warning`) | 6 h |
| VM- oder Container-Platte fast voll | Die Root-Platte eines Gasts ist zu mehr als 90 % voll (`dumbmonit_proxmox_guest_disk_used_percent`): bei Containern immer, bei VMs nur, wenn der QEMU-Gast-Agent die Nutzung meldet. | > 90 % | 15 Min. | Hinweis (`warning`) | 24 h |
| Gast gesperrt | Eine VM oder ein Container ist seit sechs Stunden gesperrt (`dumbmonit_proxmox_guest_locked`, Label `lock`): Ein unterbrochenes Backup oder eine Migration hat die Sperre zurückgelassen, und auf dem Gast kann keine Operation laufen, nicht einmal das nächste Backup. | > 0 | 6 h | Hinweis (`warning`) | 24 h |
| Alter Snapshot | Der älteste Snapshot eines Gasts ist mehr als dreißig Tage alt (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 T. | 1 h | Info (`info`) | 7 T. |
| HA-Ressource im Fehler | Eine Proxmox-HA-Ressource ist im Fehler oder gefenct (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 Min. | Warnung (`critical`) | 1 h |
| HA-Manager meldet nicht | Der lokale HA-Ressourcenmanager eines Knotens hat aufgehört, seinen Zustand zu schreiben (`dumbmonit_proxmox_ha_lrm_stale`): Die HA-Gäste dieses Knotens werden weder neu gestartet noch verlagert. | > 0 | 5 Min. | Warnung (`critical`) | 1 h |
| Cluster hat Quorum verloren | Der Proxmox-Cluster hat kein Quorum (`1 - dumbmonit_proxmox_cluster_quorate`): Gäste können nicht mehr gestartet werden. | > 0 | 1 Min. | Warnung (`critical`) | 30 Min. |
| Proxmox-Knoten offline | Ein Clusterknoten wird als offline gemeldet (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 Min. | Warnung (`critical`) | 30 Min. |
| Proxmox-Dienst ausgefallen | Ein zentraler Proxmox-Daemon läuft auf einem Knoten nicht (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, jeweils nur gezählt, wo der Knoten ihn aktiviert). | > 0 | 5 Min. | Warnung (`critical`) | 1 h |
| Netzwerkschnittstelle des Knotens ausgefallen | Eine beim Booten zu startende Schnittstelle ist auf einem Knoten nicht aktiv (`dumbmonit_proxmox_node_interface_offline`). Eine bei Bedarf hochgefahrene Schnittstelle wird nie gemeldet. | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |
| Proxmox-Storage fast voll | Ein Proxmox-Storage ist zu mehr als 85 % voll (`dumbmonit_proxmox_storage_used_percent`); „Platte fast voll“ übernimmt bei 90 %. | > 85 % | 15 Min. | Hinweis (`warning`) | 24 h |
| Thin-Pool fast voll | Ein LVM-Thin-Pool ist zu mehr als 90 % voll: Jeder darauf gespeicherte Gast geht auf Read-only, wenn er sich füllt (`dumbmonit_proxmox_node_thinpool_used_percent`); löst sich unter 88 %. | > 90 % | 15 Min. | Warnung (`critical`) | 6 h |
| Thin-Pool-Metadaten fast voll | Das Metadaten-Volume eines LVM-Thin-Pools ist zu mehr als 80 % voll (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). Es ist weit kleiner als das Datenvolume und oft zuerst gesättigt, mit derselben Folge. | > 80 % | 15 Min. | Warnung (`critical`) | 6 h |
| Proxmox-Platte verschleißt | Eine SSD oder NVMe eines Knotens hat mehr als 90 % ihrer Nennlebensdauer verbraucht (`dumbmonit_proxmox_node_disk_wearout_percent`). Die Benachrichtigung lautet `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Hinweis (`warning`) | 7 T. |
| Proxmox-Platte SMART-Fehler | Eine Platte eines Knotens meldet den SMART-Zustand `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 Min. | Warnung (`critical`) | 24 h |
| Proxmox-ZFS-Pool degradiert | Ein ZFS-Pool eines Knotens ist nicht `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). Die Benachrichtigung lautet `pve2 · tank`. | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| Backup-Job fehlgeschlagen | Der letzte vzdump-Job auf einem Knoten oder ein geplanter Backup-Job ist fehlgeschlagen (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 Min. | Warnung (`critical`) | 24 h |
| Backup-Job schließt eine Platte aus | Ein geplanter Backup-Job überspringt eine Platte eines der von ihm abgedeckten Gäste (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): Der Job ist jede Nacht erfolgreich, obwohl diese Daten fehlen. CD-ROM-Laufwerke und Cloudinit-Images sind konstruktionsbedingt ausgeschlossen und werden nie gezählt. | > 0 | 1 h | Hinweis (`warning`) | 7 T. |
| Replikation fehlgeschlagen | Ein Replikationsjob meldet einen Fehler oder einen Fehlerzähler ungleich null (`dumbmonit_proxmox_replication_job_error`): Die Standby-Kopie ist veraltet. | > 0 | 10 Min. | Warnung (`critical`) | 6 h |
| Ceph-Zustand Fehler | Ceph meldet `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 Min. | Warnung (`critical`) | 30 Min. |
| Ceph-Zustand Warnung | Ceph meldet mehr als fünfzehn Minuten lang `HEALTH_WARN` (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 Min. | Hinweis (`warning`) | 6 h |
| Ceph-OSD ausgefallen | Ein Ceph-OSD ist seit fünf Minuten down (`1 - dumbmonit_proxmox_ceph_osd_up`). Die Benachrichtigung nennt den OSD und seinen Host. | > 0 | 5 Min. | Warnung (`critical`) | 1 h |
| Ceph-OSD fast voll | Ein einzelner Ceph-OSD ist zu mehr als 85 % voll (`dumbmonit_proxmox_ceph_osd_used_percent`): Schreibvorgänge stoppen, wenn er sein Limit erreicht, was auch immer der Clusterdurchschnitt sagt; löst sich unter 82 %. | > 85 % | 15 Min. | Hinweis (`warning`) | 24 h |
| Ceph-Pool fast voll | Ein Ceph-Pool ist zu mehr als 85 % voll (`dumbmonit_proxmox_ceph_pool_used_percent`); löst sich unter 82 %. | > 85 % | 15 Min. | Hinweis (`warning`) | 24 h |
| Ceph-Flag noout gesetzt belassen | Das Ceph-Flag `noout` ist seit zwei Stunden gesetzt (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): Das Rebalancing ist deaktiviert, und der Cluster wirkt gesund, während seine Redundanz schrumpft. | > 0 | 2 h | Info (`info`) | 24 h |
| Proxmox-Updates ausstehend | Auf einem Knoten stehen mehr als zwanzig Paket-Updates aus (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T. |
| Proxmox-Sicherheitsupdates ausstehend | Mindestens ein ausstehendes Update stammt aus einem Sicherheitsarchiv (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Hinweis (`warning`) | 7 T. |
| Proxmox-Pakete geändert | Die installierte Version eines Proxmox-Pakets hat sich zwischen zwei Prüfungen geändert — jemand hat `apt upgrade` ausgeführt (`dumbmonit_proxmox_node_packages_changed`, eine Stunde lang veröffentlicht, mit dem Detail im Label `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Löst sich eine Stunde später von selbst auf. | > 0 | 1 Min. | Info (`info`) | 24 h |
| Knotenzertifikat läuft ab | Ein Proxmox-Knotenzertifikat läuft in weniger als vierzehn Tagen ab (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 T. | 1 h | Hinweis (`warning`) | 24 h |

### Proxmox Backup Server {#proxmox-backup-server}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| PBS-Datastore fast voll | PBS-Datastore zu mehr als 90 % voll. | > 90 % | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |
| PBS-Datastore füllt sich | Bei der aktuellen Rate schätzt PBS den Datastore innerhalb von sieben Tagen voll. | < 7 T. | 1 h | Hinweis (`warning`) | 24 h |
| PBS-Datastore nicht eingehängt | Ein Wechsel-Datastore ist nicht eingehängt (`dumbmonit_pbs_datastore_removable_unmounted`): Es kann nichts darauf geschrieben werden. | > 0 | 6 h | Hinweis (`warning`) | 24 h |
| PBS-Backup zu alt | Seit mehr als zwei Tagen kein neuer Snapshot für diese Backup-Gruppe (eine pro Maschine) (`dumbmonit_pbs_backup_last_age_seconds`). Passen Sie die Schwelle für wöchentlich gesicherte Maschinen an; die Benachrichtigung nennt Gruppe und Datastore. | > 2 T. | 1 h | Hinweis (`warning`) | 24 h |
| PBS-Backup-Verifizierung fehlgeschlagen | Die Verifizierung des neuesten Snapshots dieser Maschine ist fehlgeschlagen (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 Min. | Warnung (`critical`) | 24 h |
| PBS-Task fehlgeschlagen | Mindestens ein PBS-Task ist im Prüffenster fehlgeschlagen (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 Min. | Hinweis (`warning`) | 24 h |
| PBS-Garbage-Collection zu alt | Seit mehr als acht Tagen keine erfolgreiche Garbage Collection auf diesem Datastore (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 T. | 1 h | Hinweis (`warning`) | 24 h |
| PBS-Garbage-Collection fehlgeschlagen | Die letzte Garbage Collection auf einem Datastore ist fehlgeschlagen (`dumbmonit_pbs_gc_last_run_ok < 1`; aus `/gc` bei PBS 3.3+, davor aus dem letzten GC-Task). | < 1 | 10 Min. | Hinweis (`warning`) | 24 h |
| PBS-Chunks beschädigt gefunden | Die Garbage Collection hat auf einem Datastore unlesbare Chunks gefunden (`dumbmonit_pbs_gc_bad_chunks`): Manche Backups lassen sich nicht mehr wiederherstellen. | > 0 | 10 Min. | Warnung (`critical`) | 24 h |
| PBS-Prune-Job fehlgeschlagen | Der letzte Lauf eines PBS-Prune-Jobs ist fehlgeschlagen: Es wird nichts bereinigt, und der Datastore füllt sich weiter (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). Die Benachrichtigung nennt Job und Datastore. | < 1 | 10 Min. | Hinweis (`warning`) | 24 h |
| PBS-Verifizierungsjob fehlgeschlagen | Der letzte Lauf eines PBS-Verifizierungsjobs ist fehlgeschlagen (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 Min. | Hinweis (`warning`) | 24 h |
| PBS-Sync-Job fehlgeschlagen | Der letzte Lauf eines PBS-Sync-Jobs ist fehlgeschlagen (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 Min. | Warnung (`critical`) | 24 h |
| PBS-Tape-Backup fehlgeschlagen | Der letzte Tape-Backup-Lauf ist fehlgeschlagen (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): Die Offline-Kopie wird nicht angefertigt. | < 1 | 10 Min. | Warnung (`critical`) | 24 h |
| PBS-Job nie gelaufen | Ein Job ist aktiviert und geplant, aber nie gelaufen (`dumbmonit_pbs_job_never_run`). | > 0 | 2 T. | Hinweis (`warning`) | 7 T. |
| PBS-Dienst ausgefallen | Ein vom Proxmox Backup Server benötigter Dienst läuft nicht (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); solange er ausgefallen ist, kann kein Backup aufgenommen werden. | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| PBS-Platte SMART-Fehler | Eine Platte des Backup-Servers meldet den SMART-Zustand `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). Die Benachrichtigung lautet `/dev/sdb`. | > 0 | 5 Min. | Warnung (`critical`) | 24 h |
| PBS-SSD verschlissen | Eine SSD des Backup-Servers hat mehr als 90 % ihrer Nennlebensdauer verbraucht (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Hinweis (`warning`) | 7 T. |
| PBS-ZFS-Pool degradiert | Ein ZFS-Pool des Backup-Servers ist nicht `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| PBS-Zertifikat läuft ab | Das vom Backup-Server ausgelieferte Zertifikat läuft in weniger als drei Wochen ab (`dumbmonit_pbs_node_certificate_expires_seconds`). Braucht die Option `certificates`. | < 21 T. | 1 h | Hinweis (`warning`) | 48 h |
| PBS-Neustart nach Upgrade ausstehend | Das Paket wurde aktualisiert, aber der laufende Daemon ist noch der alte (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 T. |
| PBS-Updates ausstehend | Auf dem Backup-Server stehen mehr als zwanzig Paket-Updates aus (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T. |

### Proxmox Datacenter Manager {#proxmox-datacenter-manager}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Föderierte Instanz nicht erreichbar | Die Datacenter-Konsole erreicht einen der von ihr föderierten Proxmox-VE-Cluster oder Backup-Server nicht (`dumbmonit_pdm_remote_reachable < 1`). Die Benachrichtigung nennt die Instanz und ihr Produkt. | < 1 | 10 Min. | Warnung (`critical`) | 6 h |
| Föderierte Instanz im Rückstand | Eine Instanz läuft mit einer älteren Version als eine andere Instanz desselben Produkts im selben Bestand (`dumbmonit_pdm_remote_version_behind`). Zwischen Instanzen verglichen, nie gegen das, was Proxmox veröffentlicht. | > 0 | 1 h | Info (`info`) | 7 T. |
| Task auf föderierter Instanz fehlgeschlagen | Mindestens ein Task ist auf einer föderierten Instanz im Prüffenster fehlgeschlagen (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 Min. | Hinweis (`warning`) | 24 h |
| Datacenter-Konsole: Platte fast voll | Das Root-Dateisystem der Datacenter-Konsole ist zu mehr als 90 % voll (`dumbmonit_pdm_node_rootfs_percent`): Ist es voll, hört die Konsole auf, aufzuzeichnen, was sie sammelt. | > 90 % | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |
| Datacenter-Konsole: Zertifikat läuft ab | Ein Zertifikat der Datacenter-Konsole läuft in weniger als vierzehn Tagen ab (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 T. | 1 h | Hinweis (`warning`) | 24 h |
| Datacenter-Konsole: Updates ausstehend | Auf der Datacenter-Konsole stehen mehr als zwanzig Paket-Updates aus (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T. |

### Proxmox Mail Gateway {#proxmox-mail-gateway}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Mail-Queue wächst | Die Deferred-Mail-Queue ist in zwei Stunden um mehr als hundert Nachrichten gewachsen (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): Der nächste Hop lehnt wahrscheinlich Mail ab. Wachstum statt Größe, weil ein ausgelastetes Gateway immer einige Dutzend zurückgestellt hält. | > 100 | 15 Min. | Hinweis (`warning`) | 6 h |
| Mail in der Queue festgehangen | Eine Nachricht wartet seit mehr als vier Stunden in einer Queue (`dumbmonit_pmg_queue_oldest_age_seconds`). Das Alter ist eine Untergrenze — qshape meldet Klassen —, die Regel löst also eher spät als früh aus. Die Benachrichtigung nennt die Queue. | > 4 h | 15 Min. | Warnung (`critical`) | 6 h |
| Mail-Gateway-Dienst gestoppt | Ein Dienst, den das Gateway zum Annehmen und Filtern von Mail braucht, ist gestoppt (`dumbmonit_pmg_service_running` für postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Eine nicht installierte Unit hat keine Serie und kann nicht auslösen. | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| Virensignaturen veraltet | Die täglichen ClamAV-Virensignaturen sind mehr als zwei Tage alt (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): freshclam hat aufgehört, sie zu aktualisieren, und das Gateway filtert mit der Liste der letzten Woche weiter. | > 2 T. | 1 h | Hinweis (`warning`) | 24 h |
| Spam-Regeln veraltet | Die SpamAssassin-Regeln sind mehr als acht Tage alt (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Ein nie aktualisierter Kanal hat kein Datum und keine Serie. | > 8 T. | 1 h | Hinweis (`warning`) | 7 T. |
| Quarantäne füllt sich | Die Spam-Quarantäne hat in sechs Stunden mehr als tausend Nachrichten aufgenommen (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): eine Kampagne oder eine Regel, die gerade eine legitime Domain in den Papierkorb gelegt hat. | > 1000 | 30 Min. | Hinweis (`warning`) | 12 h |
| Mail-Gateway-Cluster beeinträchtigt | Ein Gateway des Clusters ist nicht mehr mit den anderen synchron (`dumbmonit_pmg_cluster_node_insync`): Es filtert mit einer veralteten Regeldatenbank. Ein eigenständiges Gateway hat keine Serie. | < 1 | 15 Min. | Hinweis (`warning`) | 6 h |
| Mail-Gateway-Zertifikat läuft ab | Ein Zertifikat des Mail-Gateways läuft in weniger als vierzehn Tagen ab (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 T. | 1 h | Hinweis (`warning`) | 24 h |
| Mail-Gateway-Updates ausstehend | Auf dem Mail-Gateway stehen mehr als zwanzig Paket-Updates aus (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 T. |

### MDaemon und SecurityGateway {#mdaemon-and-securitygateway}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| MDaemon-Mail-Dienst ausgefallen | Ein überwachter Mail-Dienst des MDaemon-Servers antwortet nicht oder begrüßt mit Dienstverweigerung (`dumbmonit_mdaemon_service_up`). Nur die auf dem Gerät aufgeführten Dienste haben eine Serie. Die Benachrichtigung nennt den Dienst und seinen Port. | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| MDaemon-XML-API antwortet nicht | Die XML-API antwortet nicht mehr, während die Mail-Dienste es noch tun (`dumbmonit_mdaemon_api_up`): Remote Administration ist wahrscheinlich gestoppt. Nur ein Gerät mit einem Konto hat die Serie. | < 1 | 15 Min. | Hinweis (`warning`) | 12 h |
| MDaemon-Mail-Queue wächst | Die Inbound-, Local- oder Remote-Queue ist in zwei Stunden um mehr als hundert Nachrichten gewachsen (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): MDaemon hat aufgehört, daraus zuzustellen. Vom [Agenten](../devices/agent.md#windows-performance-counters) auf dem MDaemon-Server gelesen, der Alarm liegt also auf dem Gerät des Agenten. | > 100 | 15 Min. | Hinweis (`warning`) | 6 h |
| MDaemon-Bad-Queue nicht leer | In der Bad-Queue warten Nachrichten (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): MDaemon konnte sie weder zustellen noch zurückweisen, und sie bleiben dort, bis ein Administrator sie freigibt oder löscht. | > 0 | 30 Min. | Hinweis (`warning`) | 24 h |
| MDaemon-Retry-Queue hoch | In der Retry-Queue warten mehr als fünfzig Nachrichten (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): Entfernte Server lehnen Mail weiter vorübergehend ab. | > 50 | 1 h | Hinweis (`warning`) | 6 h |
| SecurityGateway-Dienst ausgefallen | Ein überwachter Dienst des Gateways antwortet nicht oder begrüßt mit Dienstverweigerung (`dumbmonit_securitygateway_service_up`). | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| SecurityGateway-API antwortet nicht | Die REST-API antwortet nicht mehr, während die Dienste es noch tun (`dumbmonit_securitygateway_api_up`). Nur ein Gerät mit einem API-Schlüssel hat die Serie. | < 1 | 15 Min. | Hinweis (`warning`) | 12 h |
| SecurityGateway-Zustellwarteschlange wächst | Die Zustellwarteschlange ist in zwei Stunden um mehr als hundert Nachrichten gewachsen (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): Der Mailserver hinter dem Gateway lehnt wahrscheinlich Mail ab. Die Zählernamen kommen unverändert aus der API; benennt Ihre Version die Queue anders, passen Sie den Matcher an. | > 100 | 15 Min. | Hinweis (`warning`) | 6 h |

### OPNsense {#opnsense}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Internet-Gateway ausgefallen | dpinger erhält von einem Gateway keine Antwort mehr (`dumbmonit_opnsense_gateway_up`). Bei einer Multi-WAN-Firewall ist der Verkehr bereits still auf eine andere Verbindung gewandert. `loss` und `delay` sind Warnungen, keine Ausfälle, und lösen diese Regel nicht aus. | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| Gateway verliert Pakete | Ein Gateway verliert mehr als ein Fünftel der an es gesendeten Pakete (`dumbmonit_opnsense_gateway_loss_percent`): Die Verbindung antwortet, schlecht. Löst sich unter 10 %. | > 20 % | 10 Min. | Hinweis (`warning`) | 6 h |
| Gateway langsam | Die Round-Trip-Zeit eines Gateways liegt dauerhaft über 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Dreihundert Millisekunden lassen eine mobile Backup-Verbindung in Ruhe. | > 300 ms | 15 Min. | Info (`info`) | 12 h |
| Zustandstabelle der Firewall füllt sich | pf verfolgt mehr als 80 % der erlaubten Verbindungen (`dumbmonit_opnsense_pf_states_used_percent`). Über dem Limit verwirft die Firewall neue Verbindungen ohne anderes Symptom. | > 80 % | 10 Min. | Hinweis (`warning`) | 6 h |
| Netzwerkpuffer der Firewall erschöpft | Mehr als 90 % der FreeBSD-Netzwerkpuffer sind in Gebrauch (`dumbmonit_opnsense_mbuf_used_percent`). Eine Firewall, der sie ausgehen, hört auf zu forwarden, während die CPU noch im Leerlauf ist. | > 90 % | 10 Min. | Hinweis (`warning`) | 6 h |
| VPN-Tunnel ausgefallen | Ein konfigurierter VPN-Tunnel hat seit zehn Minuten keine Sitzung (`dumbmonit_opnsense_vpn_tunnel_up`). Ein nicht installiertes Plugin oder eine deaktivierte Verbindung hat keine Serie und kann nicht auslösen. | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |
| Resolver der Firewall gestoppt | Unbound antwortet nicht mehr (`dumbmonit_opnsense_unbound_running`). Die Firewall routet weiter; die Maschinen dahinter lösen nicht mehr auf. Schalten Sie diese Regel aus, wenn Ihre Firewall mit etwas anderem auflöst. | < 1 | 5 Min. | Hinweis (`warning`) | 6 h |
| Kerndienst der Firewall gestoppt | dpinger, der die Gateways überwacht, oder configd, der alles andere ausführt, ist gestoppt (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Nur diese beiden, weil ein absichtlich abgeschalteter Dienst kein Ausfall ist. | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| Firewall zu heiß | Ein Sensor liest über 85 °C (`dumbmonit_opnsense_temperature_celsius`): Die lüfterlose Box im Schrank beginnt gleich zu drosseln. Löst sich bei 78 °C. | > 85 °C | 10 Min. | Hinweis (`warning`) | 6 h |
| Firewall im CARP-Wartungsmodus belassen | CARP wurde im dauerhaften Wartungsmodus belassen (`dumbmonit_opnsense_carp_maintenance_mode`): Diese Firewall hat ihre virtuellen Adressen an ihren Partner übergeben und nimmt sie nicht von selbst zurück. | > 0 | 1 h | Info (`info`) | 24 h |
| Firewall-Neustart ausstehend | Ein Update ist installiert, braucht aber einen Neustart, um zu wirken (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 T. |
| Firewall-Updates ausstehend | Die Firewall hat Pakete, die auf ein Update warten (`dumbmonit_opnsense_firmware_updates_pending`). Sie ist die am stärksten exponierte Maschine im Netzwerk. | > 0 | 1 h | Info (`info`) | 7 T. |

### TrueNAS {#truenas}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| NAS-Pool degradiert | ZFS nennt einen Pool nicht mehr gesund (`dumbmonit_truenas_pool_healthy`). Ein degradierter Pool liefert seine Daten weiter aus, und genau deshalb merkt es niemand, bis die nächste Platte ausfällt. Die Geräteseite nennt die Platte. | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| NAS-Plattenfehler | Platten eines Pools haben seit dem letzten `zpool clear` Lese-, Schreib- oder Prüfsummenfehler gemeldet (`dumbmonit_truenas_pool_device_errors`, eine Serie pro Art). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |
| NAS-Pool fast voll | Ein Pool ist zu mehr als 85 % voll (`dumbmonit_truenas_pool_used_percent`). ZFS wird lange vor dem Vollwerden langsam. Löst sich bei 80 %. | > 85 % | 30 Min. | Hinweis (`warning`), eskaliert nach 7 T. | 24 h |
| NAS-Scrub hat Fehler gefunden | Der letzte Scrub eines Pools hat beschädigte Daten gefunden (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 Min. | Warnung (`critical`) | 24 h |
| NAS-Scrub überfällig | Ein Pool hat seit mehr als 45 Tagen keinen Scrub abgeschlossen (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Fehlt nach einem Resilver, der die Aufzeichnung des letzten Scrubs löscht. | > 45 T. | 1 h | Info (`info`) | 7 T. |
| NAS-Platte hat ihren SMART-Test nicht bestanden | Ein SMART-Selbsttest im Log einer Platte ist fehlgeschlagen (`dumbmonit_truenas_disk_smart_failed`). Eine nie getestete Platte hat keine Serie. | > 0 | 5 Min. | Warnung (`critical`) | 24 h |
| NAS-Platte zu heiß | Eine Platte liegt über 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, aus dem Cache von TrueNAS gelesen). Löst sich bei 50 °C. | > 55 °C | 15 Min. | Hinweis (`warning`) | 6 h |
| NAS-Dataset nahe seiner Quota | Ein Dataset hat mehr als 90 % seiner Quota verbraucht (`dumbmonit_truenas_dataset_quota_used_percent`): Bei 100 % scheitern seine Schreibvorgänge, obwohl der Pool noch Platz hat. Keine Quota, keine Serie. | > 90 % | 30 Min. | Hinweis (`warning`) | 24 h |
| NAS-Replikation fehlgeschlagen | Ein aktivierter Replikations-Task ist im Fehler (`dumbmonit_truenas_replication_error`): Die Kopie auf der anderen Seite wird älter. | > 0 | 10 Min. | Hinweis (`warning`) | 12 h |
| NAS-Snapshot-Task fehlgeschlagen | Ein aktivierter periodischer Snapshot-Task ist im Fehler (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 Min. | Hinweis (`warning`) | 12 h |
| NAS-Snapshots veraltet | Ein aktivierter periodischer Snapshot-Task ist seit mehr als acht Tagen nicht gelaufen (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): Ein Task, der aufhört zu laufen, schlägt nicht fehl, er wird still. | > 8 T. | 1 h | Hinweis (`warning`) | 24 h |
| TrueNAS-Alarm ausgelöst | TrueNAS selbst hat einen Alarm der Stufe ERROR oder höher ausgelöst (`dumbmonit_truenas_alerts`). Sein eigener Satz steht auf der Geräteseite. | > 0 | 10 Min. | Hinweis (`warning`) | 12 h |
| NAS-Dienst gestoppt | Ein Dienst, der mit dem NAS starten soll, ist gestoppt (`dumbmonit_truenas_service_running`). Ein absichtlich abgeschalteter Dienst hat keine Serie. | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |

### Synology DSM {#synology-dsm}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Synology-Platte SMART-Warnung | DSM meldet den SMART-Status dieser Platte nicht mehr als `normal` (`dumbmonit_synology_disk_smart_status`: 1 Achtung, 2 kritisch, unbekannte Wörter zählen als 1). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |
| Synology-Platte ausgefallen | DSM meldet die Platte als abgestürzt oder ausgefallen (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 Min. | Warnung (`critical`) | 6 h |
| Synology-Platte defekte Sektoren | Die Zahl defekter Sektoren der Platte überschreitet die in DSM gesetzte Schwelle (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 Min. | Warnung (`critical`) | 24 h |
| Synology-Platte: defekte Sektoren nehmen zu | In den letzten 24 Stunden sind neue unlesbare Sektoren aufgetreten (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |
| Synology-SSD verschleißt | Eine SSD hat weniger als 10 % ihrer Nennlebensdauer übrig (`dumbmonit_synology_disk_remaining_life_percent`); löst sich über 12 %. | < 10 % | 1 h | Hinweis (`warning`) | 7 T. |
| Synology-Volume degradiert | Ein Volume ist degradiert oder abgestürzt: Seine Redundanz ist weg (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 Min. | Warnung (`critical`) | 6 h |
| Synology-Speicherpool degradiert | Ein Speicherpool oder SSD-Cache ist degradiert oder abgestürzt: Eine Platte ist ausgefallen, und die Redundanz ist weg oder wird neu aufgebaut (`dumbmonit_synology_pool_status or dumbmonit_synology_ssd_cache_status`). | ≥ 2 | 5 Min. | Warnung (`critical`) | 6 h |
| Synology-Volume fast voll | Ein Volume ist zu 90 % oder mehr voll (`dumbmonit_synology_volume_used_percent`); löst sich unter 88 %. | ≥ 90 % | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |
| Synology-Temperatur hoch | Eine Platte liegt über 55 °C, oder DSM hat seine eigene Temperaturwarnung ausgelöst (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); löst sich unter 52 °C. | > 55 °C | 15 Min. | Hinweis (`warning`) | 6 h |
| Synology-Speicher hoch | Speicherauslastung des NAS über 95 % (`dumbmonit_synology_memory_usage_percent`); löst sich unter 90 %. | > 95 % | 15 Min. | Hinweis (`warning`) | 6 h |
| Active-Backup-Task fehlgeschlagen | Der letzte Lauf eines Active-Backup-for-Business-Tasks ist fehlgeschlagen oder hat nur einen Teil seiner Geräte gesichert (`dumbmonit_abb_task_last_status == bool 0`). Die Benachrichtigung nennt den Task. | > 0 | 10 Min. | Warnung (`critical`) | 24 h |
| Active Backup zu alt | Seit mehr als zwei Tagen kein erfolgreicher Active-Backup-for-Business-Lauf für diesen Task (`dumbmonit_abb_task_last_success_seconds`). Die Serie existiert nicht für einen Task, der nie erfolgreich war; dann spricht „Active-Backup-Task fehlgeschlagen“. | > 2 T. | 1 h | Hinweis (`warning`) | 24 h |
| Active-Backup-Task deaktiviert | Der Task hat keinen Zeitplan, oder sein kontinuierliches Backup ist pausiert (`dumbmonit_abb_task_enabled == bool 0`): Er sichert nichts, bis ihn jemand ausführt. | > 0 | 1 h | Info (`info`) | 7 T. |
| Active-Backup-Gerät überfällig | Ein Gerät ist länger ohne erfolgreichen Active-Backup-for-Business-Lauf, als sein eigener Rhythmus erlaubt, freie Tage ausgenommen (`dumbmonit_abb_device_overdue`, siehe [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 Min. | Hinweis (`warning`) | 24 h |
| Active-Backup-Gerät schlägt fehl | Die letzten zwei oder mehr Versuche eines Geräts sind fehlgeschlagen (`dumbmonit_abb_device_consecutive_failures`); ein abgebrochener Lauf zählt als keines von beiden. | ≥ 2 | 10 Min. | Warnung (`critical`) | 24 h |

### Log- und Metrikserver {#log-and-metrics-servers}

VictoriaMetrics, VictoriaLogs, Loki und Graylog ([Geräte](../devices/victoriametrics.md)). Zähler laufen über `increase_prometheus`, sodass ein mit einer Historie abgelehnter Daten hinzugefügter Server bei seiner ersten Messung nicht auslöst.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Metrik- oder Logspeicher schreibgeschützt | Der freie Plattenplatz ist unter `-storage.minFreeDiskSpaceBytes` gefallen, und der Speicher lehnt jeden Schreibvorgang ab (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 Min. | Warnung (`critical`) | 1 h |
| Metrik- oder Logspeicher fast schreibgeschützt | Weniger als 10 % der Platte verbleiben bis Read-only (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); löst sich über 12 %. | < 10 % | 30 Min. | Hinweis (`warning`) | 24 h |
| Metrik- oder Logserver fehlerhaft | `/health` antwortet nicht mit OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 Min. | Warnung (`critical`) | 6 h |
| Metrik- oder Logdaten abgelehnt | In der letzten Stunde abgelehnte Messwerte oder Logzeilen: Zeitstempel außerhalb der Aufbewahrung, zu lange Labels, Serienlimit (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |
| VictoriaMetrics Slow Inserts | Mehr als 5 % der neuen Messwerte nahmen über 15 Minuten den langsamen Pfad: nicht genug Speicher für die aktiven Serien (`(100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])) and on (target) dumbmonit_victoriametrics_uptime_seconds > 900`); löst sich unter 3 %. Wird erst beurteilt, wenn der Server 15 Minuten läuft, sodass ein Neustart (kalter Cache, Zähler im Fenster zurück auf null) sie nicht im Wartezustand hängen lässt. | > 5 % | 30 Min. | Hinweis (`warning`) | 24 h |
| Loki nicht bereit | `/ready` antwortet nicht mit ready (`dumbmonit_loki_ready`). | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| Loki lehnt Logzeilen ab | In der letzten Stunde abgelehnte Zeilen: zu alt, Raten- oder Stream-Limit, Zeile zu lang (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |
| Loki kann nicht in den Speicher flushen | Chunks haben es in den letzten 30 Minuten nicht in den Speicher geschafft; sie gehen verloren, wenn Loki neu startet (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 Min. | Warnung (`critical`) | 1 h |
| Loki-Write-Ahead-Log-Platte voll | Durch eine volle Platte verlorene WAL-Schreibvorgänge (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 Min. | Warnung (`critical`) | 1 h |
| Loki-Anfragefehler | Mehr als fünf Anfragen wurden in 15 Minuten mit einem Serverfehler beantwortet (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 Min. | Hinweis (`warning`) | 6 h |
| Graylog verarbeitet nicht | Die Nachrichtenverarbeitung auf dem Knoten ist pausiert (`dumbmonit_graylog_processing`). | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| Graylog-Suchcluster ausgefallen | OpenSearch oder Elasticsearch rot oder von Graylog aus nicht erreichbar (`dumbmonit_graylog_indexer_status`: 2 rot, 3 nicht erreichbar). | ≥ 2 | 5 Min. | Warnung (`critical`) | 1 h |
| Graylog-Suchcluster gelb | Replica-Shards seit einer halben Stunde nicht zugewiesen (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 Min. | Hinweis (`warning`) | 24 h |
| Graylog-Journal füllt sich | Das Journal ist mehr als halb voll: Die Ausgabe kommt nicht mit (`dumbmonit_graylog_journal_used_percent`); löst sich unter 40 %. | > 50 % | 10 Min. | Hinweis (`warning`), eskaliert nach 1 h | 6 h |
| Graylog-Input fehlgeschlagen | Ein Input konnte nicht gestartet werden (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 Min. | Hinweis (`warning`) | 24 h |
| Graylog-Indexierungsfehler | Schreibvorgänge in den Suchcluster oder die Nachrichtenverarbeitung sind in der letzten Stunde fehlgeschlagen (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |

### Selbst gehostete Anwendungen {#self-hosted-applications}

Nextcloud, Immich, Paperless-ngx, Jellyfin und Plex ([Geräte](../devices/nextcloud.md)). Sie fallen selten ganz aus: Die Seite öffnet sich, während Synchronisierung, Vorschaubilder, Importe oder Bibliotheksscans stehen geblieben sind. Diese Regeln überwachen diese Zustände; eine Anwendung, die gar nicht antwortet, ist Gerät nicht erreichbar.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Nextcloud im Wartungsmodus festgehangen | Der Wartungsmodus ist aktiv (`dumbmonit_nextcloud_maintenance`): Niemand kann sich anmelden oder synchronisieren. | > 0 | 30 Min. | Hinweis (`warning`), eskaliert nach 3 h | 6 h |
| Nextcloud-Datenbank-Upgrade ausstehend | Der Code wurde aktualisiert, und Nextcloud wartet auf `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 Min. | Warnung (`critical`) | 1 h |
| Nextcloud-Update verfügbar | Der Update-Check von Nextcloud hat eine neuere Version gefunden (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 T. |
| Nextcloud-OPcache voll | Dem OPcache von PHP geht der Platz aus, und Seiten werden langsam (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 Min. | Hinweis (`warning`) | 24 h |
| Nextcloud-freier Platz knapp | Weniger als 5 GiB frei für das Datenverzeichnis (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); löst sich über 6 GiB. | < 5 GiB | 30 Min. | Hinweis (`warning`) | 24 h |
| Immich-Jobs blockiert | Jobs warten in einer Queue, die nicht pausiert ist und nichts ausführt (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). Die Benachrichtigung nennt die Queue. | > 0 | 1 h | Hinweis (`warning`) | 6 h |
| Immich-Queue pausiert | Eine Job-Queue ist seit einem Tag pausiert (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 T. |
| Immich-Speicher fast voll | Die Platte mit der Bibliothek ist zu mehr als 90 % voll (`dumbmonit_immich_storage_used_percent`); löst sich unter 88 %. | > 90 % | 30 Min. | Hinweis (`warning`) | 24 h |
| Immich-Update verfügbar | Die Versionsprüfung von Immich kennt eine neuere Version (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 T. |
| Paperless-Task-Queue ausgefallen | Redis oder Celery im Fehler (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 ist ein Fehler): Nichts Neues wird importiert. | ≥ 2 | 5 Min. | Warnung (`critical`) | 1 h |
| Paperless-Index- oder Sanity-Check-Fehler | Der Suchindex, der Klassifikator oder der Sanity-Check (fehlende oder veränderte Dateien) meldet einen Fehler (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 Min. | Hinweis (`warning`) | 24 h |
| Paperless-Migrationen ausstehend | Noch anzuwendende Datenbankmigrationen (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 Min. | Hinweis (`warning`) | 24 h |
| Paperless-Task fehlgeschlagen | Ein für diesen Benutzer sichtbarer Task ist innerhalb des Fenster für fehlgeschlagene Tasks fehlgeschlagen und wurde nicht verworfen (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 Min. | Hinweis (`warning`) | 24 h |
| Paperless-Speicher fast voll | Die Platte mit den Dokumenten ist zu mehr als 90 % voll (`dumbmonit_paperless_storage_used_percent`); löst sich unter 88 %. | > 90 % | 30 Min. | Hinweis (`warning`) | 24 h |
| Paperless-Update verfügbar | Die Versionsprüfung von Paperless-ngx kennt eine neuere Version (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 T. |
| Jellyfin-geplanter-Task fehlgeschlagen | Ein geplanter Task (ein Bibliotheksscan, eine Bereinigung) ist bei seinem letzten Lauf fehlgeschlagen oder wurde abgebrochen (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 Min. | Hinweis (`warning`) | 24 h |
| Jellyfin-Plugin defekt | Ein Plugin konnte nicht geladen werden oder unterstützt diese Version nicht (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |
| Jellyfin-Neustart ausstehend | Jellyfin wartet seit einem Tag nach einer Plugin-Installation oder einem Plugin-Update auf einen Neustart (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 T. |
| Plex-Update verfügbar | Der Update-Check von Plex hat eine neuere Version gefunden (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 T. |

### Selbst gehostete Forges {#self-hosted-forges}

GitLab (selbst gehostet) und Forgejo/Gitea ([Geräte](../devices/gitlab.md)). Eine Instanz, die gar
nichts antwortet, ist Gerät nicht erreichbar; diese Regeln überwachen die Teile, die weiter Seiten
ausliefern, während sie still zurückfallen — Sidekiq, CI/CD-Runner, ein geplanter Task, der
aufgehört hat zu laufen.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| GitLab-Runner alle offline | Jeder GitLab bekannte CI/CD-Runner ist offline, und mindestens einer ist registriert (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |
| GitLab-Sidekiq-Rückstand hoch | In einer Sidekiq-Queue warten mehr als 500 Jobs (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): Benachrichtigungen, Webhooks und Importe fallen zurück. | > 500 | 15 Min. | Hinweis (`warning`) | 6 h |
| GitLab-Migrationen ausstehend | Datenbankmigrationen eines Upgrades sind noch anzuwenden (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Info (`info`) | 24 h |
| GitLab-Lizenz läuft ab | Die Lizenz der Enterprise Edition läuft in weniger als 30 Tagen ab (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 T. | 1 h | Hinweis (`warning`) | 24 h |
| Forgejo/Gitea-Runner alle offline | Jeder Actions-Runner ist offline, und mindestens einer ist registriert (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |
| Forgejo/Gitea geplante Aufgabe überfällig | Eine geplante Aufgabe (Repository-Prüfung, Bereinigung, Update-Check) ist nicht in ihrem eigenen Intervall gelaufen (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 Min. | Hinweis (`warning`) | 24 h |

### Client-Geräte {#client-devices}

Eine Regel, geteilt von jeder Integration, die Geräte verfolgt, die sich verbinden und sichern — heute die Smartphones, Tablets und Desktops von Immich ([Geräte](../devices/immich.md)), die gesicherten Maschinen des Proxmox Backup Servers, eine pro Backup-Gruppe ([Geräte](../devices/pbs.md)), und die geschützten Computer und VMs von Veeam, eines pro Objekt mit einem Wiederherstellungspunkt ([Geräte](../devices/veeam.md)); weitere folgen. Jede Integration berechnet den Abstand zum Veralterungsfenster ihres eigenen Ziels (`device_stale_days`, standardmäßig 3 Tage) und veröffentlicht ihn als `dumbmonit_client_device_stale_seconds`, mit den Labels `device`, `type`, `os`, `user`, `kind` und `signal` (`connection` oder `backup`); diese eine Regel löst dann für jedes davon aus und nennt das Gerät in der Benachrichtigung. Synology Active Backup for Business verfolgt seine eigenen Geräte getrennt, über einen rhythmusbewussten Mechanismus (`synology_abb_device_*`) statt dieser generischen Familie — siehe [Synology](../devices/synology.md).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Client-Gerät veraltet | Ein Gerät hat sich innerhalb des für dieses Ziel konfigurierten Veralterungsfensters weder verbunden noch gesichert (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Hinweis (`warning`) | 24 h |

### Pi-hole {#pi-hole}

[Pi-hole](../devices/pihole.md) v6, über seine REST-API gelesen.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Pi-hole-Blockierung deaktiviert | Die Blockierung ist aus, pausiert oder fehlgeschlagen (`dumbmonit_pihole_blocking_enabled`). Die Haltezeit lässt eine fünfminütige Pause aus der Weboberfläche verstreichen. | < 1 | 15 Min. | Hinweis (`warning`) | 6 h |
| Pi-hole-Blocklisten veraltet | Die Blocklisten (Gravity) wurden seit mehr als acht Tagen nicht neu aufgebaut (`dumbmonit_pihole_gravity_age_seconds`): Das wöchentliche Update schlägt fehl. | > 8 T. | 1 h | Hinweis (`warning`) | 24 h |
| Pi-hole-Update verfügbar | Eine neuere Version ist verfügbar (`dumbmonit_pihole_updates_available`): in Docker das Image, sonst Core, Weboberfläche oder FTL. Pi-hole prüft einmal täglich. | > 0 | 1 h | Info (`info`) | 7 T. |
| Pi-hole-Diagnosemeldungen | Pi-hole listet Meldungen auf seiner Diagnoseseite (`dumbmonit_pihole_messages`): eine Blockliste, die nicht heruntergeladen werden konnte, ein ratenbegrenzter Client, ein überlasteter Host. | > 0 | 15 Min. | Info (`info`) | 24 h |

### AdGuard Home {#adguard-home}

Der DNS-Filter ([Gerät](../devices/adguard.md)). Abfrage- und Blockierungszahlen sind Summen über das eigene Statistikfenster von AdGuard Home, keine Zähler: Auf ihnen ist keine Regel aufgebaut.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| AdGuard-Home-Schutz aus | Schutz deaktiviert oder pausiert und noch nicht zurück (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 Min. | Hinweis (`warning`) | 6 h |
| AdGuard-Home-DNS läuft nicht | Die Weboberfläche antwortet, aber der DNS-Server ist gestoppt (`dumbmonit_adguard_running`). | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| AdGuard-Home-Upstream fehlerhaft | Ein Upstream-DNS-Server besteht den eigenen Test von AdGuard Home nicht oder bleibt 4 Sekunden still (`dumbmonit_adguard_upstream_up`, eine Serie pro Upstream). | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |
| AdGuard-Home-Filterlisten veraltet | Das älteste Update einer aktivierten Filterliste ist mehr als drei Tage alt (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 Tage | 1 h | Hinweis (`warning`) | 24 h |
| AdGuard-Home-Update verfügbar | Eine neuere Version wird von der eigenen Versionsprüfung von AdGuard Home angekündigt (`dumbmonit_adguard_update_available`); löst nie aus, wenn diese Prüfung deaktiviert ist, wie im offiziellen Docker-Image. | > 0 | 1 h | Info (`info`) | 7 Tage |

### USV hinter NUT {#ups-behind-nut}

Von einem [NUT-Server](../devices/nut.md) gelesene USV. Jede Serie trägt das Label `ups`, sodass ein
Alarm die USV nennt, wenn ein Server mehrere veröffentlicht. Die Statusflags sind bei jedem Lesen 0
oder 1, sodass eine Regel sich von selbst auflöst, sobald das Netz zurück ist. Über SNMP gelesene USV
werden von den [USV](#ups)-Regeln oben abgedeckt.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| USV an der Batterie (NUT) | Die USV versorgt die Last aus der Batterie (`dumbmonit_nut_ups_on_battery`, das Flag `OB` von `ups.status`). | > 0 | 30 s | Warnung (`critical`) | 15 Min. |
| USV-Batterie schwach (NUT) | Die USV meldet eine schwache Batterie, das Flag `LB` (`dumbmonit_nut_ups_low_battery`): Die von ihr versorgten Rechner fahren gleich herunter. | > 0 | 1 Min. | Warnung (`critical`) | 30 Min. |
| USV-Batterie muss ersetzt werden (NUT) | Die USV verlangt den Austausch ihrer Batterie, das Flag `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Hinweis (`warning`) | 7 T. |
| USV-Last hoch (NUT) | Die Last ist dauerhaft über 80 % (`dumbmonit_nut_ups_load_percent`); löst sich unter 75 %. | > 80 % | 10 Min. | Hinweis (`warning`) | 24 h |
| USV-Laufzeit kurz (NUT) | Die USV schätzt weniger als fünf Minuten Laufzeit an der Batterie (`dumbmonit_nut_battery_runtime_seconds`); löst sich über sechs. Bei Netzbetrieb eine verschlissene Batterie oder eine für ihre Last zu kleine USV. | < 300 s | 5 Min. | Hinweis (`warning`) | 24 h |
| USV-Daten veraltet (NUT) | Der NUT-Server antwortet, hat aber keine frischen Daten von der USV: Kabel, Treiber oder USV still (`dumbmonit_nut_ups_data_stale`). Ein Ausfall bliebe währenddessen unbemerkt. | > 0 | 5 Min. | Hinweis (`warning`), eskaliert nach 1 h | 6 h |

### MikroTik RouterOS {#mikrotik-routeros}

Serien tragen das Label `interface` oder `sensor`, sodass eine Benachrichtigung den Port oder den Sensor nennt ([Gerät](../devices/mikrotik.md)). Zähler laufen über `increase_prometheus`: Ein Router, der mit Jahren von Fehlern im Rücken hinzugefügt wird, löst bei seiner ersten Messung nicht aus.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| MikroTik-CPU hoch | Die CPU des Routers liegt dauerhaft über 90 % (`dumbmonit_mikrotik_cpu_load_percent`): Routing, die Firewall oder ein Skript sättigt sie. Löst sich unter 80 %. | > 90 % | 15 Min. | Hinweis (`warning`) | 6 h |
| MikroTik-Speicher hoch | Mehr als 90 % des Speichers sind belegt (`dumbmonit_mikrotik_memory_used_percent`). Löst sich unter 85 %. | > 90 % | 15 Min. | Hinweis (`warning`) | 6 h |
| MikroTik-Storage fast voll | Der Flash-Speicher ist zu mehr als 90 % voll (`dumbmonit_mikrotik_storage_used_percent`): Konfigurationsänderungen und Upgrades können scheitern. Löst sich unter 85 %. | > 90 % | 30 Min. | Hinweis (`warning`) | 24 h |
| MikroTik-Temperatur hoch | Ein Temperatursensor liest über 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Löst sich bei 75 °C. | > 80 °C | 10 Min. | Hinweis (`warning`) | 6 h |
| MikroTik-Netzteil oder Lüfter ausgefallen | Ein Sensor `psu*-state` oder `fan*-state` ist nicht `ok` (`dumbmonit_mikrotik_health_ok`). Ein redundantes Netzteil, das gestorben ist, kappt nichts — bis das zweite es tut. | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| MikroTik-Schnittstelle sammelt Fehler | Mehr als fünfzig Empfangs- und Sendefehler auf einer Schnittstelle in der letzten Stunde (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 Min. | Hinweis (`warning`) | 24 h |
| MikroTik-Link flattert | Eine Schnittstelle hat in dreißig Minuten mehr als zweimal ihren Link verloren (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Ein für immer abgezogener Port zählt einmal und löst nicht aus. | > 2 | 5 Min. | Hinweis (`warning`) | 1 h |
| MikroTik-RouterOS-Update verfügbar | Auf dem Update-Kanal des Routers ist ein neueres RouterOS verfügbar (`dumbmonit_mikrotik_update_available`). Nur bekannt, wenn der Router selbst nach Updates sucht: siehe den optionalen Schritt der [Einrichtung](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 T. |
| MikroTik-Firmware-Upgrade ausstehend | RouterBOOT ist älter als die mit dem installierten RouterOS gebündelte Firmware (`dumbmonit_mikrotik_firmware_upgrade_pending`); sie wird nur auf Anforderung aktualisiert, dann beim nächsten Neustart. | > 0 | 1 h | Info (`info`) | 7 T. |

### UniFi Network {#unifi-network}

[Geräteseite](../devices/unifi.md). Geräteserien tragen Name, Modell und Typ des Geräts, sodass die Benachrichtigung „Wohnzimmer-AP“ sagt. Ein auf Adoption wartendes Gerät hat keine Serie `device_up` und zählt nie als offline.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| UniFi-Gerät offline | Ein vom Controller adoptiertes UniFi-Gerät ist offline, isoliert oder die Adoption ist fehlgeschlagen (`dumbmonit_unifi_device_up`). | < 1 | 5 Min. | Hinweis (`warning`), eskaliert nach 1 h | 6 h |
| UniFi-Internet ausgefallen | Das UniFi-Gateway meldet keine Internetverbindung (`dumbmonit_unifi_internet_up`). Ohne UniFi-Gateway existiert die Serie nicht, und die Regel bleibt still. | < 1 | 3 Min. | Warnung (`critical`) | 1 h |
| UniFi-WAN-Verbindung ausgefallen | Eine WAN-Verbindung des UniFi-Gateways ist ausgefallen (`dumbmonit_unifi_wan_link_up`): Mit zwei Verbindungen geht die Redundanz verloren, ohne dass etwas kaputt scheint. Ein deaktivierter Port hat keine Serie. | < 1 | 5 Min. | Hinweis (`warning`) | 6 h |
| UniFi-Gateway-CPU hoch | Der Prozessor des UniFi-Gateways liegt seit fünfzehn Minuten über 90 % (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); löst sich unter 80 %. | > 90 % | 15 Min. | Hinweis (`warning`) | 6 h |
| UniFi-Gerät wartet auf Adoption | Ein Gerät ist im Netzwerk aufgetaucht und wartet in UniFi auf die Adoption (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 Min. | Info (`info`) | 24 h |
| UniFi-Firmware-Update verfügbar | Für UniFi-Geräte ist ein Firmware-Update verfügbar (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 T. |
| UniFi-Alarme | UniFi hat in den letzten 24 Stunden kritische Ereignisse protokolliert oder Alarme ausgelöst (`dumbmonit_unifi_alarms`). Nur mit einem Konto Nur Ansicht gelesen. | > 0 | 5 Min. | Hinweis (`warning`) | 24 h |

### Home Assistant {#home-assistant}

[Geräteseite](../devices/homeassistant.md).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Home Assistant im Wiederherstellungsmodus | Home Assistant ist im Wiederherstellungsmodus gestartet: Seine Konfiguration konnte nicht geladen werden, und Automatisierungen laufen nicht (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 Min. | Warnung (`critical`) | 1 h |
| Home Assistant läuft nicht | Home Assistant antwortet, hat aber seit zehn Minuten den Start nicht abgeschlossen (`dumbmonit_homeassistant_running`). | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |
| Home-Assistant-Entitäten nicht verfügbar | Mindestens fünf Entitäten mehr als vor einer Stunde sind nicht verfügbar: Eine Integration, ein Hub oder ein Funkmodul ist ausgefallen (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). Der Anstieg zählt, nicht die Summe, die in einer echten Installation nie bei null liegt. | ≥ 5 | 10 Min. | Hinweis (`warning`) | 6 h |
| Home-Assistant-Batterie schwach | Eine von Home Assistant gemeldete Batterie ist schwach (`dumbmonit_homeassistant_battery_low`, eine Serie pro Batterie unter der Schwelle des Geräts, standardmäßig 20 %; ihr Wert ist der Ladestand). | ≥ 0 % | 30 Min. | Info (`info`) | 3 T. |
| Home-Assistant-Update verfügbar | Bei Home Assistant warten Updates: Core, Betriebssystem, Add-ons oder Gerätefirmware (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 T. |
| Home-Assistant-Reparatur erforderlich | Home Assistant hat eine offene Reparatur mit Schweregrad Fehler oder Kritisch (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |

### VMware vSphere {#vmware-vsphere}

[Geräteseite](../devices/vsphere.md). Host-Serien tragen `esxi`, VM-Serien `vm`, Datastore-Serien `datastore`, sodass die Benachrichtigung das Objekt nennt.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| vSphere-Host antwortet nicht | Ein vSphere-Host antwortet vCenter nicht (`dumbmonit_vsphere_host_connection_state`: 2 antwortet nicht). Ein absichtlich getrennter Host (1) löst nicht aus. | ≥ 2 | 3 Min. | Warnung (`critical`) | 1 h |
| vSphere-Host-Zustand rot | vSphere meldet den Gesamtzustand eines Hosts als rot (`dumbmonit_vsphere_host_status == 2`). Grau bedeutet unbekannt und löst nicht aus. | > 0 | 5 Min. | Warnung (`critical`) | 1 h |
| vSphere-Host im Wartungsmodus belassen | Ein vSphere-Host ist seit einem Tag im Wartungsmodus: Seine VMs wurden verlagert, und er wurde möglicherweise dort vergessen (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| vSphere-Datastore fast voll | Ein vSphere-Datastore ist zu 90 % oder mehr voll (`dumbmonit_vsphere_datastore_used_percent`); löst sich unter 85 %. | ≥ 90 % | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |
| vSphere-Datastore unzugänglich | Ein vSphere-Datastore ist unzugänglich: Die darauf gespeicherten VMs können nicht laufen (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| vSphere-VM-Tools laufen nicht | VMware Tools sind installiert, laufen aber in einer eingeschalteten VM nicht: Der Gast hängt möglicherweise (`dumbmonit_vsphere_vm_tools_status == 2`). VMs ohne Tools lösen nicht aus. | > 0 | 30 Min. | Info (`info`) | 24 h |
| vSphere-Alarm rot | vSphere hat einen roten Alarm ausgelöst, den niemand bestätigt hat (`dumbmonit_vsphere_alarm{status="red"}`, eine Serie pro Alarm, nach dem Alarm und dem Objekt benannt). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |

### Datenbanken, Message Broker und Sicherheits-Engine {#databases-message-broker-and-security-engine}

Redis / Valkey, MongoDB, RabbitMQ und CrowdSec ([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md), [RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)). Zähler laufen über `increase_prometheus` oder `changes_prometheus`, sodass bei der ersten Messung eines Geräts nichts auslöst.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Redis-Speicher nahe maxmemory | Genutzter Speicher über 90 % von `maxmemory` (`dumbmonit_redis_memory_used_percent`); löst sich unter 85 %. Keine Serie ohne `maxmemory`. | > 90 % | 10 Min. | Hinweis (`warning`) | 6 h |
| Redis weist Verbindungen ab | In den letzten 15 Minuten abgewiesene Client-Verbindungen: `maxclients` erreicht (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 Min. | Warnung (`critical`) | 1 h |
| Redis-Replikationsverbindung unterbrochen | Ein Replikat hat die Verbindung zum Primary verloren (`dumbmonit_redis_master_link_up`). | < 1 | 2 Min. | Warnung (`critical`) | 1 h |
| Redis-Replikat im Rückstand | Vom Primary aus gesehen hat ein Replikat seit mehr als 30 Sekunden nicht bestätigt (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 Min. | Hinweis (`warning`) | 6 h |
| Redis-Snapshot fehlgeschlagen | Das letzte `BGSAVE` ist fehlgeschlagen; mit `stop-writes-on-bgsave-error` werden Schreibvorgänge abgewiesen (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| Redis-Append-only-Datei-Schreibvorgang fehlgeschlagen | Der letzte AOF-Schreibvorgang ist fehlgeschlagen (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| MongoDB-Replica-Set ohne Primary | Kein Mitglied ist Primary: Kein Schreibvorgang wird angenommen (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 Min. | Warnung (`critical`) | 1 h |
| MongoDB-Mitglied nicht erreichbar | Ein Replica-Set-Mitglied ist von den anderen aus nicht erreichbar (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| MongoDB-Replikationsverzug | Ein Secondary liegt mehr als eine Minute hinter dem Primary (`dumbmonit_mongodb_replset_member_lag_seconds`); löst sich unter 30 s. | > 60 s | 10 Min. | Hinweis (`warning`) | 6 h |
| MongoDB-Verbindungen nahe am Limit | Mehr als 80 % der verfügbaren Verbindungen in Gebrauch (`dumbmonit_mongodb_connections_used_percent`); löst sich unter 75 %. | > 80 % | 10 Min. | Hinweis (`warning`) | 6 h |
| MongoDB-Cache unter Druck | Mehr als 20 % des WiredTiger-Caches sind dirty (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); löst sich unter 15 %. | > 20 % | 10 Min. | Hinweis (`warning`) | 6 h |
| MongoDB-interne Fehler | Reguläre Assertions in der letzten Stunde (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 Min. | Hinweis (`warning`) | 24 h |
| RabbitMQ-Speicher- oder Plattenalarm | Ein Knoten hat einen Ressourcenalarm ausgelöst: Jeder Publisher ist blockiert (`dumbmonit_rabbitmq_alarm`, eine Serie pro Knoten und Ressource). | > 0 | 1 Min. | Warnung (`critical`) | 1 h |
| RabbitMQ-Knoten ausgefallen | Ein Clusterknoten läuft nicht (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 Min. | Warnung (`critical`) | 1 h |
| RabbitMQ-Netzwerkpartition | Ein Knoten sieht eine Netzwerkpartition (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 Min. | Warnung (`critical`) | 1 h |
| RabbitMQ-Queue ohne Consumer | Eine Queue hält Nachrichten und hat keinen Consumer (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); der Wert ist die Zahl wartender Nachrichten. | > 0 | 15 Min. | Hinweis (`warning`) | 6 h |
| RabbitMQ-Queue nicht verfügbar | Eine Queue läuft nicht: Quorum-Queue ohne Mehrheit oder ihr Knoten ist ausgefallen (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 Min. | Hinweis (`warning`) | 6 h |
| CrowdSec-Local-API ausgefallen | Die Local API antwortet nicht auf `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| CrowdSec-Bouncer hat aufgehört abzurufen | Die Anfragen eines Bouncers haben sich in 30 Minuten nicht verändert (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 Min. | Hinweis (`warning`) | 6 h |
| CrowdSec liest keine Logs | Keine einzige Logzeile in sechs Stunden gelesen (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 Min. | Hinweis (`warning`) | 24 h |

### Reverse-Proxys und Domains {#reverse-proxies-and-domains}

Traefik, Caddy, Nginx Proxy Manager und Domainregistrierungen ([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md), [Nginx Proxy Manager](../devices/npm.md), [Domain-Ablauf](../devices/domain.md)) sowie die Option *Alarm bei Änderung der Antwort* der DNS-Prüfung ([Dienste](../devices/services.md#alerting-when-the-answer-changes)). Serien tragen `router`, `service`, `server`, `upstream`, `proxy_host` oder `cert`, sodass die Benachrichtigung nennt, was ausgefallen ist.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Traefik-Router deaktiviert | Ein Router ist durch einen Konfigurationsfehler deaktiviert (fehlender Service oder fehlende Middleware, ungültige Regel): Seine Websites werden nicht ausgeliefert (`dumbmonit_traefik_router_status`: 0 aktiviert, 1 Warnung, 2 deaktiviert). | ≥ 2 | 5 Min. | Hinweis (`warning`) | 6 h |
| Traefik-Backend-Server ausgefallen | Ein Server eines lastverteilten Services besteht seinen Health-Check nicht (`dumbmonit_traefik_server_up`). | < 1 | 5 Min. | Hinweis (`warning`) | 6 h |
| Traefik-Service ohne Server | Kein Server eines Services besteht seinen Health-Check: Seine Websites erhalten 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 Min. | Warnung (`critical`) | 1 h |
| Traefik-Zertifikat nicht erhalten | Router verlangen von einem Zertifikat-Resolver ein Zertifikat, das noch kein von Traefik gehaltenes Zertifikat abdeckt: Traefik liefert sein selbstsigniertes Standardzertifikat aus (`dumbmonit_traefik_resolver_routers_uncovered`, braucht `/api/certificates` von Traefik). | > 0 | 1 h | Hinweis (`warning`) | 24 h |
| Caddy-Upstream fehlerhaft | Ein Backend besteht den aktiven oder passiven Health-Check von Caddy nicht (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 Min. | Hinweis (`warning`) | 6 h |
| Caddy-Konfigurations-Reload fehlgeschlagen | Caddy hat seinen letzten Konfigurations-Reload abgelehnt und läuft weiter mit der vorherigen (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 Min. | Hinweis (`warning`) | 24 h |
| Nginx-Proxy-Manager-Host offline | nginx hat die Konfiguration eines aktivierten Hosts abgelehnt: Der Host wird nicht ausgeliefert (`dumbmonit_npm_host_online`). | < 1 | 10 Min. | Hinweis (`warning`) | 6 h |
| Proxy-Zertifikat nicht erneuert | Ein von Traefik gehaltenes oder von einem Nginx-Proxy-Manager-Host präsentiertes Zertifikat läuft in weniger als 14 Tagen ab: Die automatische Erneuerung beginnt bei 30, sie schlägt also fehl (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 T. | 1 h | Hinweis (`warning`) | 24 h |
| Reverse-Proxy antwortet mit 5xx | Mehr als 5 % der Antworten über zehn Minuten waren 5xx, bei mindestens fünfzig Anfragen (`increase_prometheus` von `dumbmonit_{traefik,caddy}_requests_5xx_total` über `…_requests_total`). Braucht die Metriken des Proxys. | > 5 % | 10 Min. | Hinweis (`warning`) | 6 h |
| Domain läuft ab | Die Domain läuft in weniger als 30 Tagen ab (`dumbmonit_domain_expiry_days >= 7`). | < 30 T. | 1 h | Hinweis (`warning`) | 24 h |
| Domain läuft gleich ab | Die Domain läuft in weniger als 7 Tagen ab oder ist abgelaufen (`dumbmonit_domain_expiry_days`). | < 7 T. | 1 h | Warnung (`critical`) | 12 h |
| Domain auf Hold | Die Registry hat die Domain auf `clientHold` oder `serverHold` gesetzt: Sie ist aus dem DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 Min. | Warnung (`critical`) | 6 h |
| Domain in der Redemption-Phase | Die Domain ist in `redemptionPeriod`, `pendingDelete` oder `pendingRestore`: Sie ist abgelaufen (`dumbmonit_domain_redemption`). | > 0 | 30 Min. | Warnung (`critical`) | 12 h |
| DNS-Antwort geändert | Die Antwort einer DNS-Prüfung mit *Alarm bei Änderung der Antwort* weicht von der davor ab (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Löst sich eine Stunde später von selbst auf. | > 0 | 1 Min. | Hinweis (`warning`) | 24 h |

### Website-Änderungen {#website-changes}

Siehe [Website-Änderungen](../devices/webchange.md). Die Geräteseite zeigt jede Änderung vorher und nachher.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Website geändert | Eine Prüfung einer überwachten Seite oder Website hat Seiten gefunden, die sich geändert haben, hinzugekommen oder verschwunden sind (`dumbmonit_webchange_last_check_changes`). Jede solche Prüfung wird gemeldet und nennt die Seiten, unter demselben Alarm, solange er feuert; er löst sich bei der ersten Prüfung, die nichts Neues findet. | > 0 | 30 s | Info (`info`) | keine |

### Kubernetes {#kubernetes}

Siehe [Kubernetes / k3s](../devices/kubernetes.md). Jede Serie trägt das Label des Knotens oder des
Namespace und des betroffenen Pods, Workloads oder Claims.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Kubernetes-Knoten nicht bereit | Ein Knoten ist nicht Ready: Sein Kubelet meldet nicht mehr, und seine Pods werden evakuiert (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| Kubernetes-Knoten unter Druck | Ein Knoten meldet Speicher-, Platten- oder PID-Druck (`dumbmonit_k8s_node_pressure`, Label `condition`). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |
| Kubernetes-Pod in Crash-Schleife | Ein Container ist in `CrashLoopBackOff` und startet weiter neu: mindestens zwei Neustarts in 15 Minuten (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) and on (target, namespace, pod) increase_prometheus(dumbmonit_k8s_pod_restarts[15m]) >= 2`; der Zustand verschwindet bei jedem Neustartversuch kurz). Container, die durch einen Neustart des Knotens oder der Runtime beendet wurden (Exit 137, 143 oder 255, nicht `OOMKilled`), zählen nicht, sodass ein Reboot nicht wie eine Crash-Schleife aussieht. | > 0 | 10 Min. | Warnung (`critical`) | 6 h |
| Kubernetes-Pod hängt in Pending | Ein Pod ist seit fünfzehn Minuten Pending: Kein Knoten kann ihn aufnehmen, oder ein Volume oder Image, das er braucht, kommt nicht (`dumbmonit_k8s_pod_pending`). | > 0 | 15 Min. | Hinweis (`warning`) | 6 h |
| Kubernetes-Pod startet immer wieder neu | Ein Pod ist in der letzten Stunde mehr als fünfmal neu gestartet, ohne in `CrashLoopBackOff` zu sein (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 Min. | Hinweis (`warning`) | 6 h |
| Kubernetes-Workload mit fehlenden Replikas | Ein Deployment, StatefulSet oder DaemonSet hat weniger verfügbare Replikas als angefordert (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 Min. | Hinweis (`warning`), eskaliert nach 1 h | 6 h |
| Kubernetes-Volume-Claim ausstehend | Ein PersistentVolumeClaim, den ein Pod braucht oder den ein Provisionierungsfehler betrifft, ist noch nicht gebunden (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 Min. | Hinweis (`warning`) | 6 h |

### Active Directory {#active-directory}

Siehe [Active Directory](../devices/activedirectory.md). Der von DumbMonit abgefragte
Domänencontroller wird von *Gerät nicht erreichbar* abgedeckt; die übrigen Domänencontroller der
Domäne von *Domänencontroller nicht erreichbar* (Label `dc`).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Domänencontroller nicht erreichbar | Ein Domänencontroller der Domäne antwortet auf seinem LDAP-Port nicht mehr (`dumbmonit_ad_dc_reachable == bool 0`). Ein Name, der von DumbMonit aus nicht aufgelöst wird, erzeugt keine Serie und löst nie aus. | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| Active-Directory-Bind fehlgeschlagen | Der Domänencontroller antwortet, lehnt aber das Dienstkonto ab: abgelaufenes Passwort, deaktiviertes oder gesperrtes Konto (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 Min. | Hinweis (`warning`) | 24 h |
| Mitgliedschaft in privilegierter Gruppe geändert | Ein Mitglied ist Domain Admins, Enterprise Admins, Schema Admins, Administrators oder einer Operatorengruppe beigetreten oder hat sie verlassen, Verschachtelung eingeschlossen (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, Label `group`). Löst sich eine Stunde später von selbst auf. | > 0 | 1 Min. | Hinweis (`warning`) | 24 h |
| Active-Directory-Replikation schlägt fehl | Der abgefragte Domänencontroller kann nicht von einem Partner replizieren (`dumbmonit_ad_replication_consecutive_failures`). Nur wenn das Konto den Replikationsstatus lesen darf. | > 0 | 30 Min. | Hinweis (`warning`) | 6 h |
| krbtgt-Passwort älter als 180 Tage | Das krbtgt-Passwort wurde zuletzt vor mehr als 180 Tagen geändert (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 Tage | 10 Min. | Info (`info`) | keine |

### pfSense {#pfsense}

Siehe [pfSense](../devices/pfsense.md). Gateways tragen das Label `gateway`, Schnittstellen `interface` und `descr`, Dienste `service`.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| pfSense-Gateway ausgefallen | Ein pfSense-Gateway ist ausgefallen: Seine Monitor-Adresse antwortet nicht mehr, und der von ihm getragene Verkehr stoppt oder wechselt (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 Min. | Warnung (`critical`) | 6 h |
| pfSense-Gateway verliert Pakete | Ein pfSense-Gateway verliert mehr als 10 % der Probes zu seiner Monitor-Adresse: Die Leitung ist beeinträchtigt (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 Min. | Hinweis (`warning`) | 6 h |
| pfSense-Gateway langsam | Der Round-Trip zur Monitor-Adresse eines pfSense-Gateways liegt seit fünfzehn Minuten über 500 ms (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 Min. | Hinweis (`warning`) | 6 h |
| pfSense-Schnittstelle ohne Link | Eine aktivierte pfSense-Schnittstelle hat keinen Link: Kabel, Switch-Port oder Netzwerkkarte (`dumbmonit_pfsense_interface_down`). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |
| pfSense-Dienst gestoppt | Ein aktivierter pfSense-Dienst läuft nicht: DNS-Resolver, VPN, NTP … (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |
| pfSense-Platte fast voll | Die pfSense-Platte ist zu mehr als 90 % voll: Logs, Berichte und Paket-Updates werden nicht mehr geschrieben (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 Min. | Hinweis (`warning`) | 1 T. |

### Unraid {#unraid}

Siehe [Unraid](../devices/unraid.md). Platten tragen die Labels `disk` und `role` (`parity`, `data`, `cache`), Container `container`.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Unraid-Array gestoppt | Das Unraid-Array ist nicht gestartet: Darauf liegende Shares, Container und VMs sind nicht verfügbar (`dumbmonit_unraid_array_started`). | < 1 | 10 Min. | Warnung (`critical`) | 6 h |
| Unraid-Platte deaktiviert oder fehlt | Eine Unraid-Platte ist deaktiviert, fehlt oder ist ungültig: Das Array läuft degradiert, und ein weiterer Ausfall verliert Daten (`dumbmonit_unraid_disk_ok`). | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| Unraid-Platte Lesefehler | Eine Unraid-Platte hat seit dem Start des Arrays Lesefehler gemeldet: Beim nächsten wird sie möglicherweise deaktiviert (`dumbmonit_unraid_disk_errors`). | > 0 | 5 Min. | Hinweis (`warning`) | 1 T. |
| Unraid-Platte zu heiß | Eine Unraid-Array- oder Paritätsplatte lag fünfzehn Minuten lang über 55 °C (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 Min. | Hinweis (`warning`) | 6 h |
| Unraid-Paritätsprüfung hat Fehler gefunden | Die letzte Unraid-Paritätsprüfung ist fehlgeschlagen oder hat Sync-Fehler gefunden: Die Parität stimmt nicht mehr mit den Daten überein (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 Min. | Hinweis (`warning`) | 1 T. |
| Unraid-Paritätsprüfung überfällig | Die letzte Unraid-Paritätsprüfung ist mehr als 40 Tage alt: Eine monatliche Prüfung wurde verpasst (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 T. | 1 h | Hinweis (`warning`) | 1 T. |
| Unraid-Cache fast voll | Ein Unraid-Cache-Pool ist zu mehr als 90 % voll: Schreibvorgänge fallen auf das langsame Array zurück oder scheitern (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 Min. | Hinweis (`warning`) | 1 T. |
| Unraid-Container gestoppt | Ein Docker-Container, der mit dem Array starten soll, läuft nicht (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 Min. | Hinweis (`warning`) | 6 h |

### Veeam Backup & Replication {#veeam-backup--replication}

Siehe [Veeam Backup & Replication](../devices/veeam.md). Jobs tragen die Labels `job` und `type`, Repositories `repository`.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Veeam-Job fehlgeschlagen | Der letzte Lauf eines aktivierten Veeam-Jobs ist fehlgeschlagen: Was er schützt, hat keinen neuen Wiederherstellungspunkt (`dumbmonit_veeam_job_failed`). | > 0 | 5 Min. | Warnung (`critical`) | 1 T. |
| Veeam-Job mit Warnung beendet | Der letzte Lauf eines aktivierten Veeam-Jobs endete mit einer Warnung: Manche Objekte sind möglicherweise nicht geschützt (`dumbmonit_veeam_job_warning`). | > 0 | 5 Min. | Hinweis (`warning`) | 1 T. |
| Veeam-Repository fast voll | Ein Veeam-Backup-Repository ist zu mehr als 90 % voll: Die nächsten Jobs scheitern mangels Platz (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 Min. | Hinweis (`warning`) | 1 T. |
| Veeam-Lizenz läuft ab | Die Veeam-Lizenz läuft in weniger als 30 Tagen ab: Jobs stoppen am Ende der Karenzzeit (`dumbmonit_veeam_license_expiry_seconds`). | < 30 T. | 1 h | Hinweis (`warning`) | 1 T. |

### Tailscale {#tailscale}

Siehe [Tailscale](../devices/tailscale.md). Geräte tragen das Label `device`, das erste Label ihres MagicDNS-Namens.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Tailscale-Gerät offline | Ein Tailscale-Gerät, das online bleiben muss (standardmäßig ein getaggtes Gerät), ist seit mehr als 15 Minuten getrennt (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 Min. | 5 Min. | Hinweis (`warning`) | 6 h |
| Tailscale-Node-Schlüssel läuft ab | Der Node-Schlüssel eines Tailscale-Geräts läuft in weniger als 14 Tagen ab: Es verlässt das Tailnet, bis es jemand erneut anmeldet (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 T. | 1 h | Hinweis (`warning`) | 1 T. |
| Tailscale-Gerät wartet auf Freigabe | Ein Gerät ist dem Tailnet beigetreten und wartet auf Freigabe: Geben Sie es frei, wenn es erwartet wird, entfernen Sie es sonst (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 Min. | Hinweis (`warning`) | 1 T. |
| Tailscale-Client-Update | Ein Tailscale-Gerät, das online bleiben muss, läuft mit einem Client, für den ein Update verfügbar ist (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Info (`info`) | 7 T. |

### FortiGate {#fortigate}

Siehe [FortiGate](../devices/fortigate.md). Tunnel tragen das Label `tunnel`, Schnittstellen `interface` und `alias`, Lizenzen `license`.

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| FortiGate-IPsec-Tunnel ausgefallen | Keine Phase 2 eines FortiGate-Site-to-Site-IPsec-Tunnels ist oben: Der Standort dahinter ist abgeschnitten (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 Min. | Warnung (`critical`) | 6 h |
| FortiGate-Schnittstelle ohne Link | Eine aktivierte FortiGate-Schnittstelle mit Adresse hat keinen Link: Kabel, Transceiver oder das Gerät davor (`dumbmonit_fortigate_interface_down`). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |
| FortiGate-HA nicht synchron | Die Mitglieder eines FortiGate-HA-Clusters haben nicht mehr dieselbe Konfigurationsprüfsumme: Ein Failover würde eine andere Konfiguration ausführen (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 Min. | Hinweis (`warning`) | 6 h |
| FortiGate-HA-Mitglied verloren | Ein FortiGate-HA-Cluster hat weniger Mitglieder als in den letzten 24 Stunden: Es gibt kein Standby mehr (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| FortiGate-Speicher nahe Conserve Mode | Der FortiGate-Speicher liegt seit zehn Minuten über 85 %: Bei 88 % (standardmäßig) geht er in den Conserve Mode und inspiziert keine neuen Sitzungen mehr (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 Min. | Hinweis (`warning`) | 6 h |
| FortiGate-CPU hoch | Die FortiGate-CPU liegt seit fünfzehn Minuten über 90 % (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 Min. | Hinweis (`warning`) | 6 h |
| FortiGate-Lizenz läuft ab | Eine FortiGuard- oder FortiCare-Lizenz der FortiGate läuft in weniger als 30 Tagen ab oder ist abgelaufen (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 T. | 1 h | Hinweis (`warning`) | 1 T. |
| FortiGate-Firmware-Update | FortiGuard bietet ein neueres Patch-Release des FortiOS-Zweigs an, den die FortiGate betreibt (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Info (`info`) | 7 T. |

### Sophos Firewall {#sophos-firewall}

Siehe [Sophos Firewall](../devices/sophos.md).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Sophos-Schnittstelle ohne Link | Eine eingeschaltete und an eine Zone gebundene Sophos-Firewall-Schnittstelle hat keinen Link (`dumbmonit_sophos_interface_down`). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |

### KVM (libvirt) {#kvm-libvirt}

Vom Linux-Agenten auf einem KVM-Host gemeldet; siehe [Agent](../devices/agent.md#kvm-virtual-machines-through-libvirt).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| KVM-Domain abgestürzt | libvirt meldet eine KVM-Domain als abgestürzt: Der Hypervisor musste sie stoppen (`dumbmonit_agent_libvirt_domain_crashed`). | > 0 | 1 Min. | Warnung (`critical`) | 1 h |
| libvirt-Pool fast voll | Die Belegung eines libvirt-Storage-Pools erreicht 90 % seiner Kapazität (`dumbmonit_agent_libvirt_pool_used_percent`). | ≥ 90 % (löst sich unter 88 %) | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |

### XCP-ng / XenServer {#xcp-ng-xenserver}

Siehe [XCP-ng / XenServer](../devices/xcpng.md).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| XCP-ng-Host offline | Ein Host des XCP-ng-Pools antwortet dem Koordinator nicht mehr (`dumbmonit_xcpng_host_live`). | < 1 | 5 Min. | Warnung (`critical`) | 1 h |
| XCP-ng-SR fast voll | Ein XCP-ng-Storage-Repository ist zu mehr als 90 % voll (`dumbmonit_xcpng_sr_used_percent`). | ≥ 90 % (löst sich unter 88 %) | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |
| HA-geschützte XCP-ng-VM ausgefallen | Eine durch die Hochverfügbarkeit des Pools geschützte VM ist angehalten (`dumbmonit_xcpng_vm_ha_protected_down`). | > 0 | 2 Min. | Warnung (`critical`) | 1 h |
| XCP-ng-Pool-HA überbucht | Der Pool hat keinen Plan mehr, um die Host-Ausfälle zu überstehen, die er tolerieren soll (`dumbmonit_xcpng_pool_ha_overcommitted`). | > 0 | 5 Min. | Hinweis (`warning`), eskaliert nach 1 h | 6 h |

### Nutanix {#nutanix}

Siehe [Nutanix AHV](../devices/nutanix.md).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Nutanix-Host beeinträchtigt | Prism meldet einen Nutanix-Host als beeinträchtigt (`dumbmonit_nutanix_host_degraded`). | > 0 | 5 Min. | Warnung (`critical`) | 1 h |
| Nutanix-Storage-Container fast voll | Ein Nutanix-Storage-Container ist zu mehr als 90 % voll (`dumbmonit_nutanix_storage_container_used_percent`). | ≥ 90 % (löst sich unter 88 %) | 15 Min. | Hinweis (`warning`), eskaliert nach 24 h | 6 h |
| Kritische Nutanix-Alarme | Prism Central meldet einen oder mehrere ungelöste kritische Alarme (`dumbmonit_nutanix_alerts_critical`). | > 0 | 5 Min. | Warnung (`critical`) | 1 h |

### Hyper-V {#hyper-v}

Vom Windows-Agenten auf einem Hyper-V-Host gemeldet; siehe [Agent](../devices/agent.md#hyper-v-hosts).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Hyper-V-VM-Zustand kritisch | Hyper-V meldet eine virtuelle Maschine in kritischem Zustand: Eine benötigte Ressource, meist Plattenplatz, ist erschöpft (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 Min. | Warnung (`critical`) | 6 h |
| Hyper-V-Host-CPU hoch | Die logischen Prozessoren des Hyper-V-Hosts sind seit fünfzehn Minuten zu mehr als 90 % ausgelastet, Gäste eingeschlossen (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 Min. | Hinweis (`warning`) | 6 h |
| Hyper-V-virtuelle-Platte-Fehler | Eine virtuelle Hyper-V-Platte hat in der letzten Stunde I/O-Fehler gemeldet (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 Min. | Hinweis (`warning`) | 6 h |

### Sicherheitswert {#security-score}

Vom Server alle 15 Minuten für jedes Gerät berechnet, dessen Art Sicherheitsprüfungen hat, und als `dumbmonit_security_score` (0–100) gespeichert, nur solange die Sicherheitsbewertung in den Einstellungen eingeschaltet ist (standardmäßig aus). Siehe [Sicherheitswert](../using/security-score.md).

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| Sicherheitswert gefallen | Der Wert ist gegenüber seinem besten Wert der letzten 24 Stunden um 10 Punkte oder mehr gefallen (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): Eine Prüfung schlägt neuerdings fehl. | ≥ 10 Pkt. | 30 Min. | Hinweis (`warning`) | 24 h |
| Sicherheitswert niedrig | Der Wert liegt unter 40, Note F (`dumbmonit_security_score`). Erhöhen oder senken Sie die Schwelle passend zu Ihrem eigenen Anspruch. | < 40 | 1 h | Info (`info`) | 7 T. |

### DumbMonit selbst {#dumbmonit-itself}

| Regel | Was | Standardschwelle | Haltezeit | Schweregrad | Erinnerung |
|---|---|---|---|---|---|
| DumbMonit-Backup lief nicht | Das geplante lokale Backup der DumbMonit-Datenbank ist seit mehr als zwei Tagen nicht gelaufen (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 T. | 1 h | Hinweis (`warning`) | 24 h |

Jede integrierte Regel gilt für alle Geräte und für jeden aktivierten Kanal.

## Eine Regel in der Oberfläche bearbeiten {#editing-a-rule-in-the-ui}

Auf der Seite **Alarme** listet der Abschnitt *Regeln* jede Regel mit ihrem Schweregrad, einer
Markierung *Integriert* und, wenn relevant, wie viele Alarme sie gerade auslöst. Jede Regel hat:

- einen Schalter zum **Aktivieren oder Deaktivieren**;
- **Bearbeiten**, das einen Inline-Editor für Name, Beschreibung, Operator, Schwelle, Haltezeit
  (`for`), Schweregrad, Erinnerung (`repeat every`) und Eskalation (`escalate after`) aufklappt. Die
  Abfrage einer integrierten Regel wird angezeigt, ist aber nicht bearbeitbar; die Abfrage Ihrer
  eigenen Regeln schon. Baseline-Regeln zeigen ihre Erkennungsparameter schreibgeschützt;
- **Löschen**, nur für Ihre eigenen Regeln.

## Überschreibungen pro Gerät {#per-device-overrides}

Eine Regel lässt sich für ein Gerät anpassen, ohne die Regel selbst anzufassen: eine andere
Schwelle, eine andere Auflösungsschwelle, oder allein für dieses Gerät abgeschaltet. Der Editor
jeder Regel hat ein Panel **Überschreibungen pro Gerät**, um direkt eine hinzuzufügen oder zu
entfernen.

Die Ein-Klick-Aktion **Ignorieren** bei einem Alarm (in der Liste *Braucht Sie* oder auf der
Geräteseite des Alarms) ist derselbe Mechanismus, geschrieben mit `enabled: false`: Das Gerät
trägt diese Regel nicht mehr, sofort und umkehrbar. Die Geräteseite listet auf, was sie ignoriert,
mit **Nicht mehr ignorieren**; die Regel selbst zeigt unter Alarme → Regeln *Auf N Geräten
ignoriert*, wo dasselbe Panel für Überschreibungen pro Gerät sie entfernt. Siehe
[Überschreibungen pro Gerät](../reference/api.md#per-device-overrides) für die API.

## Eine Schwellenregel erstellen {#creating-a-threshold-rule}

Klicken Sie auf **Neue Regel**. Das Formular verlangt:

| Feld | Bedeutung |
|---|---|
| Name | In Alarmen und Benachrichtigungen angezeigt. |
| Abfrage | Ein MetricsQL-Ausdruck, pro Gerät ausgewertet. Beispiel: `dumbmonit_cpu_usage_percent`. |
| Operator | `>`, `>=`, `<` oder `<=`. |
| Schwelle | Die Zahl, mit der das Abfrageergebnis verglichen wird. |
| Schweregrad | Info, Hinweis oder Warnung. |
| Halten für | Sekunden, die die Bedingung andauern muss, bevor sie auslöst. Eine Haltezeit ungleich null fängt einen einzelnen schlechten Messwert ab. |

Jede von der Abfrage zurückgegebene Serie wird zu einem potenziellen Alarm, angehängt an das durch
ihr Label `target` benannte Gerät. Aggregieren Sie mit `avg by (target, host) (…)`, wenn Sie einen
Alarm pro Gerät statt einen pro Kern oder pro Schnittstelle wollen. Die Identität des Alarms
ignoriert die Labels `host` und `tag_*`, sobald `target` vorhanden ist, sodass das Umbenennen eines
Geräts seine Alarme behält; eine Serie, deren `target` zu keinem existierenden, aktivierten Gerät
passt, wird ignoriert.

## Die Abfragesprache {#the-query-language}

Abfragen sind [MetricsQL](https://docs.victoriametrics.com/metricsql/), die PromQL-Obermenge von
VictoriaMetrics. Die Metriken sind die Serien `dumbmonit_*`, beschrieben in der
[Metrikreferenz](../reference/metrics.md), und jede Serie trägt ein Label `target` (die Geräte-ID)
und ein Label `host` (ihren Namen).

Einige von den integrierten Regeln verwendete Muster:

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

!!! tip "Eine Abfrage zuerst ausprobieren"
    `GET /api/metrics/query?query=…` liefert, was die Regel auswerten würde (siehe die
    [API-Referenz](../reference/api.md#metrics)). In der Entwicklung veröffentlicht das Overlay das
    eingebettete VictoriaMetrics auf dem `:8428` des Hosts, mit seiner Abfrageoberfläche.

## Regelarten in der API {#rule-kinds-in-the-api}

`kind` ist `threshold` (Vergleich mit einer Schwelle), `predict` (derselbe Vergleich auf einer
`predict_linear`-Abfrage; als Prognose angezeigt) oder `anomaly` (saisonale Baseline; `params`
enthält `k`, `alpha`, `mad_floor_abs`, `mad_floor_rel`, `min_samples`). Die Oberfläche erstellt
Regeln vom Typ `threshold`; die anderen beiden Arten lassen sich über die API erstellen.
