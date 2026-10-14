# Règles

Les règles sont insérées au premier démarrage, puis vous appartiennent :
DumbMonit ne réécrit jamais une règle que vous avez changée, il ne
recrée que celles qui manquent. Les règles intégrées peuvent être
modifiées et désactivées, pas supprimées.

## Règles intégrées

Les gravités sont montrées avec le mot de l'UI ; la valeur d'API est
entre parenthèses. Une règle dont la métrique n'est produite par aucun
appareil n'a aucune série et ne se déclenche jamais, chaque règle
ci-dessous est donc livrée activée sur chaque instance.

### Chaque appareil

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Appareil inaccessible | Aucune mesure reçue depuis plus de trois minutes. `time() - tlast_over_time(dumbmonit_up[7d])` | > 180 s | 1 min | Avertissement (`critical`) | 6 h |
| CPU élevé | Charge CPU soutenue au-dessus de 90 %, moyennée par appareil à travers les sources SNMP, Proxmox et agent ; se résout sous 85 %. | > 90 % | 10 min | Avis (`warning`), escalade après 1 h | 6 h |
| CPU inhabituel | Charge CPU sensiblement différente de l'habituel à cette heure et ce jour de la semaine (référence saisonnière). | score > 3,5 | 15 min | Info (`info`) | 6 h |
| Disque presque plein | Système de fichiers plein à 90 % ou plus, par point de montage (SNMP, stockages Proxmox et systèmes de fichiers racine) ; se résout sous 88 %. | ≥ 90 % | 15 min | Avis (`warning`), escalade après 24 h | 6 h |
| Système de fichiers presque plein | À ce rythme, le système de fichiers sera plein dans quatre jours (`predict_linear` sur 6 h ; systèmes de fichiers en croissance seulement). | ≥ 100 % | 30 min | Avis (`warning`) | 6 h |

### Onduleur {#ups}

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Onduleur sur batterie | L'onduleur alimente la charge depuis la batterie (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Avertissement (`critical`) | 15 min |
| Batterie d'onduleur faible | La batterie de l'onduleur est faible ou épuisée (`dumbmonit_ups_battery_status`, 3 = faible, 4 = épuisée). | ≥ 3 | 1 min | Avertissement (`critical`) | 30 min |

### Ports réseau (SNMP)

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Port accumulant des erreurs | Un port réseau a journalisé plus de cinquante erreurs dans la dernière heure : généralement un câble défaillant, une fibre sale ou un désaccord de duplex (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])` ; rien ne se déclenche pendant la première heure d'un nouvel appareil, les erreurs comptées avant son ajout ne sont donc pas rapportées comme nouvelles). | > 50 | 15 min | Avis (`warning`) | 24 h |
| Port instable | Un port réseau est tombé et remonté plus de deux fois en trente minutes (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Un port désactivé volontairement n'a aucune série et ne se déclenche jamais. | > 4 | 5 min | Avis (`warning`) | 1 h |

### Matériel serveur (Redfish) {#server-hardware-redfish}

Les séries de santé sont 0 (OK), 1 (Avertissement) ou 2 (Critique). Un
emplacement vide (`Absent`) ou désactivé n'a aucune série, un serveur
livré avec une alimentation sur deux ne se déclenche donc jamais.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Ventilateur serveur en échec | Le contrôleur de gestion rapporte un ventilateur en échec (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 min | Avertissement (`critical`) | 6 h |
| Température serveur au-dessus du critique | Un capteur de température est au-dessus du seuil critique que le contrôleur déclare lui-même (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 min | Avertissement (`critical`) | 6 h |
| Redondance d'alimentation perdue | Les alimentations ne sont plus redondantes : une panne de plus et le serveur s'arrête (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 min | Avis (`warning`), escalade après 1 h | 6 h |
| Alimentation en échec | Le contrôleur rapporte une alimentation en échec (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 min | Avertissement (`critical`) | 6 h |
| Échec de disque prédit | Un disque rapporte une panne prédite : remplacez-le tant qu'il fonctionne encore (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 min | Avis (`warning`) | 24 h |
| Santé serveur critique | Le contrôleur rapporte la propre santé du système comme Critical (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 min | Avertissement (`critical`) | 6 h |

### Moniteurs de disponibilité et battements de cœur

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Service en panne | Le service n'a pas répondu correctement depuis trois minutes (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 min | Avertissement (`critical`) | 30 min |
| Battement de cœur manqué | La tâche n'a pas appelé dans son intervalle attendu plus la période de grâce, ou a rapporté un échec elle-même (`dumbmonit_probe_success{probe="push"} == bool 0` ; le délai est réglé par appareil, voir [Battement de cœur](../devices/push.md)). | > 0 | 2 min | Avis (`warning`) | 6 h |
| Service instable | Le service a changé d'état plus de six fois en trente minutes (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 min | Avis (`warning`) | 1 h |
| Service lent | Le service prend plus de trois secondes pour répondre. | > 3 s | 10 min | Avis (`warning`) | 6 h |
| Certificat arrivant à expiration | Le certificat expire dans moins de quatorze jours. | < 14 j | 1 h | Avis (`warning`) | 24 h |
| Certificat expiré | Le certificat a expiré. | < 0 j | 5 min | Avertissement (`critical`) | 24 h |

### Conteneurs Docker

Rapporté par l'agent ; voir [Agent](../devices/agent.md). L'alerte nomme
le conteneur.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Conteneur arrêté | Le conteneur est arrêté depuis deux minutes (`dumbmonit_container_up == bool 0`). | > 0 | 2 min | Avis (`warning`) | 1 h |
| Conteneur malsain | Le contrôle de santé du conteneur échoue depuis trois minutes (`dumbmonit_container_health == 2` ; 0 aucun, 1 sain, 3 démarrage). | > 0 | 3 min | Avis (`warning`) | 1 h |
| Conteneur redémarre | Le conteneur a redémarré trois fois ou plus en quinze minutes (`increase(dumbmonit_container_restart_count[15m])`) : il est rarement arrêté assez longtemps pour la règle précédente. | ≥ 3 | 1 min | Avis (`warning`) | 1 h |
| Mise à jour de conteneur disponible | Une image plus récente est disponible dans le registre pour ce conteneur (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Sauvegardes Plakar

Rapporté par l'agent, par kloset et par source. Aucune de ces séries
n'existe tant que Plakar n'est pas détecté, aucune règle ne peut donc se
déclencher sur une machine sans lui.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Sauvegarde Plakar trop ancienne | Aucun nouveau snapshot Plakar pour cette source depuis plus de deux jours (`dumbmonit_backup_last_success_seconds`). | > 2 j | 1 h | Avis (`warning`) | 24 h |
| Sauvegarde Plakar échouée | Le dernier snapshot a des erreurs, ou le kloset ne peut pas être lu (`dumbmonit_backup_last_status < 1`). | < 1 | 10 min | Avertissement (`critical`) | 24 h |

### Sauvegardes restic et Borg

Rapporté par l'agent pour chaque dépôt déclaré sous `restic_repos` ou
`borg_repos` dans agent.yaml ; voir
[Agent](../devices/agent.md#restic-and-borg-backups). Sans dépôt
déclaré, il n'y a aucune série, aucune règle ne peut donc se déclencher.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Sauvegarde restic/Borg trop ancienne | Aucun nouveau snapshot restic ou Borg dans ce dépôt depuis plus de 48 heures (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Avis (`warning`) | 24 h |
| Dépôt restic/Borg inaccessible | L'agent ne peut pas lire le dépôt : inaccessible, mot de passe refusé ou binaire manquant ; le journal de l'agent dit lequel (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 min | Avis (`warning`) | 24 h |

### Tunnels WireGuard

Rapporté par l'agent ; voir
[Agent](../devices/agent.md#wireguard-tunnels).

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Pair WireGuard silencieux | Un pair avec un keepalive persistant n'a pas terminé de handshake depuis plus de 15 minutes : le tunnel est en panne (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Un pair qui ne s'est jamais connecté compte son silence depuis le moment où l'agent l'a vu pour la première fois. Les pairs sans keepalive (un téléphone, un portable) sont inactifs pendant des jours et sont laissés de côté ; changez le seuil pour changer N, ou retirez la partie `and on …` pour surveiller chaque pair. | > 15 min | 5 min | Avis (`warning`) | 6 h |

### Tunnels IPsec

Rapporté par l'agent ; voir [Agent](../devices/agent.md#ipsec-tunnels). La série
n'existe que pour une CHILD_SA déclarée dans la configuration strongSwan : la
règle se déclenche donc aussi sur un tunnel qui n'a jamais monté, pas
seulement sur un tunnel qui est tombé.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Tunnel IPsec hors service | Un tunnel IPsec configuré n'a aucune session installée (`dumbmonit_agent_ipsec_tunnel_up < 1`). | < 1 | 10 min | Avis (`warning`) | 6 h |

### Test de débit Internet

Rapporté par l'agent ; voir
[Agent](../devices/agent.md#internet-speed-test). Le test est désactivé par
défaut et chaque règle reste silencieuse tant que le seuil correspondant n'est
pas réglé dans `agent.yaml` : personne ne peut deviner ce que « lent » veut dire
pour votre ligne.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Débit descendant Internet bas | Le dernier test a mesuré un débit descendant sous `speedtest_min_download_mbps` (`dumbmonit_agent_speedtest_download_min_bytes_per_second - dumbmonit_agent_speedtest_download_bytes_per_second > 0`). | > 0 | 10 min | Avis (`warning`) | 24 h |
| Débit montant Internet bas | Idem, pour `speedtest_min_upload_mbps`. | > 0 | 10 min | Avis (`warning`) | 24 h |
| Latence Internet élevée | Le dernier test a mesuré une latence au-dessus de `speedtest_max_latency_ms`. | > 0 | 10 min | Avis (`warning`) | 24 h |

### Proxmox VE

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Sauvegarde trop ancienne | Aucune sauvegarde Proxmox VE réussie depuis plus de sept jours (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 j | 1 h | Avis (`warning`) | 24 h |
| VM ou conteneur arrêté | L'invité tournait dans les deux dernières heures et est arrêté depuis cinq minutes (`1 - dumbmonit_proxmox_guest_running`, et avec `max_over_time(…[2h]) == 1`). | > 0 | 5 min | Avis (`warning`) | 6 h |
| VM ou conteneur CPU élevé | Un invité a utilisé plus de 90 % de ses cœurs alloués pendant quinze minutes (`dumbmonit_proxmox_guest_cpu_percent`). La notification nomme l'invité, `nextcloud (202)`. | > 90 % | 15 min | Avis (`warning`) | 6 h |
| VM ou conteneur mémoire élevée | Un invité a utilisé plus de 95 % de sa mémoire pendant dix minutes (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 min | Avis (`warning`) | 6 h |
| VM ou conteneur disque presque plein | Le disque racine d'un invité est plein à plus de 90 % (`dumbmonit_proxmox_guest_disk_used_percent`) : conteneurs toujours, VMs seulement quand l'agent invité QEMU rapporte l'usage. | > 90 % | 15 min | Avis (`warning`) | 24 h |
| Invité verrouillé | Une VM ou un conteneur est verrouillé depuis six heures (`dumbmonit_proxmox_guest_locked`, label `lock`) : une sauvegarde ou migration interrompue a laissé le verrou et aucune opération ne peut tourner sur l'invité, pas même la prochaine sauvegarde. | > 0 | 6 h | Avis (`warning`) | 24 h |
| Vieux snapshot | Le plus vieux snapshot d'un invité a plus de trente jours (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 j | 1 h | Info (`info`) | 7 j |
| Ressource HA en erreur | Une ressource HA Proxmox est en erreur ou isolée (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 min | Avertissement (`critical`) | 1 h |
| Gestionnaire HA ne rapporte pas | Le gestionnaire de ressources local HA d'un nœud a arrêté d'écrire son état (`dumbmonit_proxmox_ha_lrm_stale`) : les invités HA de ce nœud ne seront ni redémarrés ni déplacés. | > 0 | 5 min | Avertissement (`critical`) | 1 h |
| Cluster a perdu le quorum | Le cluster Proxmox n'a pas le quorum (`1 - dumbmonit_proxmox_cluster_quorate`) : les invités ne peuvent plus être démarrés. | > 0 | 1 min | Avertissement (`critical`) | 30 min |
| Nœud Proxmox hors ligne | Un nœud de cluster est rapporté hors ligne (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 min | Avertissement (`critical`) | 30 min |
| Service Proxmox arrêté | Un démon Proxmox principal ne tourne pas sur un nœud (`dumbmonit_proxmox_node_core_services_down` : `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, chacun compté seulement là où le nœud l'active). | > 0 | 5 min | Avertissement (`critical`) | 1 h |
| Interface réseau de nœud en panne | Une interface réglée pour démarrer au boot n'est pas active sur un nœud (`dumbmonit_proxmox_node_interface_offline`). Une interface amenée à la demande n'est jamais rapportée. | > 0 | 5 min | Avis (`warning`) | 6 h |
| Stockage Proxmox presque plein | Un stockage Proxmox est plein à plus de 85 % (`dumbmonit_proxmox_storage_used_percent`) ; « Disque presque plein » prend le relais à 90 %. | > 85 % | 15 min | Avis (`warning`) | 24 h |
| Pool thin presque plein | Un pool thin LVM est plein à plus de 90 % : chaque invité stocké sur lui passe en lecture seule quand il se remplit (`dumbmonit_proxmox_node_thinpool_used_percent`) ; se résout sous 88 %. | > 90 % | 15 min | Avertissement (`critical`) | 6 h |
| Métadonnées de pool thin presque pleines | Le volume de métadonnées d'un pool thin LVM est plein à plus de 80 % (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). Il est bien plus petit que le volume de données et sature souvent d'abord, avec la même conséquence. | > 80 % | 15 min | Avertissement (`critical`) | 6 h |
| Disque Proxmox qui s'use | Un SSD ou NVMe d'un nœud a utilisé plus de 90 % de sa vie nominale (`dumbmonit_proxmox_node_disk_wearout_percent`). La notification lit `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Avis (`warning`) | 7 j |
| Échec SMART de disque Proxmox | Un disque d'un nœud rapporte une santé SMART `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 min | Avertissement (`critical`) | 24 h |
| Pool ZFS Proxmox dégradé | Un pool ZFS d'un nœud n'est pas `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). La notification lit `pve2 · tank`. | > 0 | 5 min | Avertissement (`critical`) | 6 h |
| Job de sauvegarde échoué | Le dernier job vzdump sur un nœud, ou un job de sauvegarde planifié, a échoué (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 min | Avertissement (`critical`) | 24 h |
| Job de sauvegarde exclut un disque | Un job de sauvegarde planifié saute un disque d'un des invités qu'il couvre (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`) : le job réussit chaque nuit avec cette donnée manquante. Les lecteurs CD-ROM et images cloudinit sont exclus par conception et jamais comptés. | > 0 | 1 h | Avis (`warning`) | 7 j |
| Réplication échouée | Un job de réplication rapporte une erreur ou un compteur d'échec non nul (`dumbmonit_proxmox_replication_job_error`) : la copie de secours est périmée. | > 0 | 10 min | Avertissement (`critical`) | 6 h |
| Erreur de santé Ceph | Ceph rapporte `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 min | Avertissement (`critical`) | 30 min |
| Avertissement de santé Ceph | Ceph rapporte `HEALTH_WARN` depuis plus de quinze minutes (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 min | Avis (`warning`) | 6 h |
| OSD Ceph en panne | Un OSD Ceph est en panne depuis cinq minutes (`1 - dumbmonit_proxmox_ceph_osd_up`). La notification nomme l'OSD et son hôte. | > 0 | 5 min | Avertissement (`critical`) | 1 h |
| OSD Ceph presque plein | Un seul OSD Ceph est plein à plus de 85 % (`dumbmonit_proxmox_ceph_osd_used_percent`) : les écritures s'arrêtent quand il atteint sa limite, quoi que dise la moyenne du cluster ; se résout sous 82 %. | > 85 % | 15 min | Avis (`warning`) | 24 h |
| Pool Ceph presque plein | Un pool Ceph est plein à plus de 85 % (`dumbmonit_proxmox_ceph_pool_used_percent`) ; se résout sous 82 %. | > 85 % | 15 min | Avis (`warning`) | 24 h |
| Drapeau noout Ceph laissé actif | Le drapeau `noout` de Ceph est réglé depuis deux heures (`dumbmonit_proxmox_ceph_flag{flag="noout"}`) : le rééquilibrage est désactivé, et le cluster se lit sain tandis que sa redondance se réduit. | > 0 | 2 h | Info (`info`) | 24 h |
| Mises à jour Proxmox en attente | Plus de vingt mises à jour de paquet sont en attente sur un nœud (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 j |
| Mises à jour de sécurité Proxmox en attente | Au moins une mise à jour en attente vient d'une archive de sécurité (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Avis (`warning`) | 7 j |
| Paquets Proxmox changés | La version installée d'un paquet Proxmox a changé entre deux sondes — quelqu'un a lancé `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, publié pendant une heure avec le détail dans son label `changes` : `pve1 · pve-manager 8.2.4→8.2.7`). Se résout d'elle-même une heure plus tard. | > 0 | 1 min | Info (`info`) | 24 h |
| Certificat de nœud arrivant à expiration | Un certificat de nœud Proxmox expire dans moins de quatorze jours (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 j | 1 h | Avis (`warning`) | 24 h |

### Proxmox Backup Server

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Datastore PBS presque plein | Datastore PBS plein à plus de 90 %. | > 90 % | 15 min | Avis (`warning`), escalade après 24 h | 6 h |
| Datastore PBS qui se remplit | Au rythme actuel, PBS estime le datastore plein dans sept jours. | < 7 j | 1 h | Avis (`warning`) | 24 h |
| Datastore PBS non monté | Un datastore amovible n'est pas monté (`dumbmonit_pbs_datastore_removable_unmounted`) : rien ne peut y être écrit. | > 0 | 6 h | Avis (`warning`) | 24 h |
| Sauvegarde PBS trop ancienne | Aucun nouveau snapshot pour ce groupe de sauvegarde (un par machine) depuis plus de deux jours (`dumbmonit_pbs_backup_last_age_seconds`). Modifiez le seuil pour des machines sauvegardées chaque semaine ; la notification nomme le groupe et le datastore. | > 2 j | 1 h | Avis (`warning`) | 24 h |
| Vérification de sauvegarde PBS échouée | La vérification du dernier snapshot de cette machine a échoué (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 min | Avertissement (`critical`) | 24 h |
| Tâche PBS échouée | Au moins une tâche PBS a échoué dans la fenêtre de revue (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 min | Avis (`warning`) | 24 h |
| Ramasse-miettes PBS trop ancien | Aucun ramasse-miettes réussi sur ce datastore depuis plus de huit jours (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 j | 1 h | Avis (`warning`) | 24 h |
| Ramasse-miettes PBS échoué | Le dernier ramasse-miettes sur un datastore a échoué (`dumbmonit_pbs_gc_last_run_ok < 1` ; depuis `/gc` sur PBS 3.3+, depuis la dernière tâche GC avant). | < 1 | 10 min | Avis (`warning`) | 24 h |
| Chunks corrompus PBS trouvés | Le ramasse-miettes a trouvé des chunks illisibles sur un datastore (`dumbmonit_pbs_gc_bad_chunks`) : certaines sauvegardes ne peuvent plus être restaurées. | > 0 | 10 min | Avertissement (`critical`) | 24 h |
| Job prune PBS échoué | La dernière exécution d'un job prune PBS a échoué : rien n'est purgé et le datastore continue de se remplir (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). La notification nomme le job et le datastore. | < 1 | 10 min | Avis (`warning`) | 24 h |
| Job de vérification PBS échoué | La dernière exécution d'un job de vérification PBS a échoué (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 min | Avis (`warning`) | 24 h |
| Job sync PBS échoué | La dernière exécution d'un job sync PBS a échoué (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 min | Avertissement (`critical`) | 24 h |
| Sauvegarde sur bande PBS échouée | La dernière exécution de sauvegarde sur bande a échoué (`dumbmonit_pbs_tape_backup_job_last_ok < 1`) : la copie hors ligne n'est pas faite. | < 1 | 10 min | Avertissement (`critical`) | 24 h |
| Job PBS jamais exécuté | Un job est activé et planifié mais n'a jamais tourné (`dumbmonit_pbs_job_never_run`). | > 0 | 2 j | Avis (`warning`) | 7 j |
| Service PBS arrêté | Un service dont Proxmox Backup Server a besoin ne tourne pas (`dumbmonit_pbs_node_service_active{expected="1"} < 1`) ; aucune sauvegarde ne peut être prise tant qu'il est arrêté. | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Échec SMART de disque PBS | Un disque du serveur de sauvegarde rapporte une santé SMART `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). La notification lit `/dev/sdb`. | > 0 | 5 min | Avertissement (`critical`) | 24 h |
| SSD PBS usé | Un SSD du serveur de sauvegarde a utilisé plus de 90 % de son endurance nominale (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Avis (`warning`) | 7 j |
| Pool ZFS PBS dégradé | Un pool ZFS du serveur de sauvegarde n'est pas `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 min | Avertissement (`critical`) | 6 h |
| Certificat PBS arrivant à expiration | Le certificat servi par le serveur de sauvegarde expire dans moins de trois semaines (`dumbmonit_pbs_node_certificate_expires_seconds`). A besoin de l'option `certificates`. | < 21 j | 1 h | Avis (`warning`) | 48 h |
| Redémarrage PBS en attente après mise à niveau | Le paquet a été mis à niveau mais le démon en cours est encore l'ancien (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 j |
| Mises à jour PBS en attente | Plus de vingt mises à jour de paquet sont en attente sur le serveur de sauvegarde (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 j |

### Proxmox Datacenter Manager

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Instance fédérée inaccessible | La console de datacenter ne peut pas atteindre un des clusters Proxmox VE ou serveurs de sauvegarde qu'elle fédère (`dumbmonit_pdm_remote_reachable < 1`). La notification nomme l'instance et son produit. | < 1 | 10 min | Avertissement (`critical`) | 6 h |
| Instance fédérée en retard | Une instance tourne une version plus ancienne qu'une autre instance du même produit dans le même parc (`dumbmonit_pdm_remote_version_behind`). Comparé entre instances, jamais contre ce que Proxmox publie. | > 0 | 1 h | Info (`info`) | 7 j |
| Tâche en échec sur une instance fédérée | Au moins une tâche a échoué sur une instance fédérée dans la fenêtre de revue (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 min | Avis (`warning`) | 24 h |
| Disque de console de datacenter presque plein | Le système de fichiers racine de la console de datacenter est plein à plus de 90 % (`dumbmonit_pdm_node_rootfs_percent`) : plein, la console arrête d'enregistrer ce qu'elle collecte. | > 90 % | 15 min | Avis (`warning`), escalade après 24 h | 6 h |
| Certificat de console de datacenter arrivant à expiration | Un certificat de la console de datacenter expire dans moins de quatorze jours (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 j | 1 h | Avis (`warning`) | 24 h |
| Mises à jour de console de datacenter en attente | Plus de vingt mises à jour de paquet sont en attente sur la console de datacenter (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 j |

### Proxmox Mail Gateway

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| File de mail qui grandit | La file de mail différé a grandi de plus de cent messages en deux heures (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`) : le prochain saut refuse probablement le mail. Croissance plutôt que taille, parce qu'une passerelle chargée garde toujours quelques dizaines de différés. | > 100 | 15 min | Avis (`warning`) | 6 h |
| Mail bloqué dans la file | Un message attend dans une file depuis plus de quatre heures (`dumbmonit_pmg_queue_oldest_age_seconds`). L'âge est une borne inférieure — qshape rapporte des tranches — la règle se déclenche donc tard plutôt que tôt. La notification nomme la file. | > 4 h | 15 min | Avertissement (`critical`) | 6 h |
| Service de passerelle mail arrêté | Un service dont la passerelle a besoin pour accepter et filtrer le mail est arrêté (`dumbmonit_pmg_service_running` pour postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Une unité qui n'est pas installée n'a aucune série et ne peut pas se déclencher. | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Signatures antivirus obsolètes | Les signatures virus quotidiennes de ClamAV ont plus de deux jours (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`) : freshclam a arrêté de les mettre à jour et la passerelle continue de filtrer avec la liste de la semaine dernière. | > 2 j | 1 h | Avis (`warning`) | 24 h |
| Règles antispam obsolètes | Les règles SpamAssassin ont plus de huit jours (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Un canal jamais mis à jour n'a aucune date ni série. | > 8 j | 1 h | Avis (`warning`) | 7 j |
| Quarantaine qui se remplit | La quarantaine spam a reçu plus de mille messages en six heures (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`) : une campagne, ou une règle qui vient de mettre un domaine légitime à la poubelle. | > 1000 | 30 min | Avis (`warning`) | 12 h |
| Cluster de passerelle mail dégradé | Une passerelle du cluster n'est plus synchronisée avec les autres (`dumbmonit_pmg_cluster_node_insync`) : elle filtre avec une base de règles obsolète. Une passerelle autonome n'a aucune série. | < 1 | 15 min | Avis (`warning`) | 6 h |
| Certificat de passerelle mail arrivant à expiration | Un certificat de la passerelle mail expire dans moins de quatorze jours (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 j | 1 h | Avis (`warning`) | 24 h |
| Mises à jour de passerelle mail en attente | Plus de vingt mises à jour de paquet sont en attente sur la passerelle mail (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 j |

### MDaemon et SecurityGateway {#mdaemon-and-securitygateway}

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Service mail MDaemon arrêté | Un service mail surveillé du serveur MDaemon ne répond pas, ou salue pour refuser le service (`dumbmonit_mdaemon_service_up`). Seuls les services listés sur l'appareil ont une série. La notification nomme le service et son port. | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| API XML MDaemon ne répond pas | L'API XML a arrêté de répondre tandis que les services mail répondent toujours (`dumbmonit_mdaemon_api_up`) : Remote Administration est probablement arrêté. Seul un appareil avec un compte a la série. | < 1 | 15 min | Avis (`warning`) | 12 h |
| File de mail MDaemon qui grandit | La file inbound, locale ou remote a grandi de plus de cent messages en deux heures (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`) : MDaemon a arrêté d'y livrer. Lu par l'[agent](../devices/agent.md#windows-performance-counters) sur le serveur MDaemon, l'alerte est donc sur l'appareil de l'agent. | > 100 | 15 min | Avis (`warning`) | 6 h |
| File Bad MDaemon non vide | Des messages attendent dans la file Bad (`dumbmonit_mdaemon_queue_messages{queue="bad"}`) : MDaemon n'a pu ni les livrer ni les rejeter, et ils y restent jusqu'à ce qu'un administrateur les libère ou les supprime. | > 0 | 30 min | Avis (`warning`) | 24 h |
| File Retry MDaemon élevée | Plus de cinquante messages attendent dans la file Retry (`dumbmonit_mdaemon_queue_messages{queue="retry"}`) : des serveurs distants continuent de refuser le mail temporairement. | > 50 | 1 h | Avis (`warning`) | 6 h |
| Service SecurityGateway arrêté | Un service surveillé de la passerelle ne répond pas, ou salue pour refuser le service (`dumbmonit_securitygateway_service_up`). | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| API SecurityGateway ne répond pas | L'API REST a arrêté de répondre tandis que les services répondent toujours (`dumbmonit_securitygateway_api_up`). Seul un appareil avec une clé d'API a la série. | < 1 | 15 min | Avis (`warning`) | 12 h |
| File de livraison SecurityGateway qui grandit | La file de livraison a grandi de plus de cent messages en deux heures (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`) : le serveur mail derrière la passerelle refuse probablement le mail. Les noms de compteur viennent de l'API tels quels ; si votre version nomme la file différemment, ajustez le filtre. | > 100 | 15 min | Avis (`warning`) | 6 h |

### OPNsense

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Passerelle Internet en panne | dpinger n'obtient plus de réponse d'une passerelle (`dumbmonit_opnsense_gateway_up`). Sur un pare-feu multi-WAN, le trafic a déjà bougé vers un autre lien, silencieusement. `loss` et `delay` sont des avertissements, pas des pannes, et ne déclenchent pas cette règle. | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Passerelle qui perd des paquets | Une passerelle perd plus d'un cinquième des paquets qui lui sont envoyés (`dumbmonit_opnsense_gateway_loss_percent`) : le lien répond, mal. Se résout sous 10 %. | > 20 % | 10 min | Avis (`warning`) | 6 h |
| Passerelle lente | Le temps d'aller-retour d'une passerelle est resté au-dessus de 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Trois cents millisecondes laisse tranquille un lien de secours mobile. | > 300 ms | 15 min | Info (`info`) | 12 h |
| Table d'états du pare-feu qui se remplit | pf suit plus de 80 % des connexions qu'il est autorisé à suivre (`dumbmonit_opnsense_pf_states_used_percent`). Au-delà de la limite, le pare-feu abandonne les nouvelles connexions sans autre symptôme. | > 80 % | 10 min | Avis (`warning`) | 6 h |
| Buffers réseau du pare-feu épuisés | Plus de 90 % des buffers réseau FreeBSD sont utilisés (`dumbmonit_opnsense_mbuf_used_percent`). Un pare-feu qui en manque arrête de transférer avec le CPU encore inactif. | > 90 % | 10 min | Avis (`warning`) | 6 h |
| Tunnel VPN en panne | Un tunnel VPN configuré n'a eu aucune session depuis dix minutes (`dumbmonit_opnsense_vpn_tunnel_up`). Un plugin qui n'est pas installé, ou une connexion désactivée, n'a aucune série et ne peut pas se déclencher. | < 1 | 10 min | Avis (`warning`) | 6 h |
| Résolveur du pare-feu arrêté | Unbound a arrêté de répondre (`dumbmonit_opnsense_unbound_running`). Le pare-feu route toujours ; les machines derrière ne résolvent plus. Désactivez cette règle si votre pare-feu résout avec autre chose. | < 1 | 5 min | Avis (`warning`) | 6 h |
| Service principal du pare-feu arrêté | dpinger, qui surveille les passerelles, ou configd, qui fait tourner tout le reste, est arrêté (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Seulement ces deux, parce qu'un service éteint volontairement n'est pas une panne. | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Pare-feu trop chaud | Un capteur lit au-dessus de 85 °C (`dumbmonit_opnsense_temperature_celsius`) : le boîtier sans ventilateur dans le placard va bientôt réduire ses performances. Se résout à 78 °C. | > 85 °C | 10 min | Avis (`warning`) | 6 h |
| Pare-feu laissé en mode maintenance CARP | CARP est resté en mode maintenance persistant (`dumbmonit_opnsense_carp_maintenance_mode`) : ce pare-feu a donné ses adresses virtuelles à son partenaire et ne les reprendra pas de lui-même. | > 0 | 1 h | Info (`info`) | 24 h |
| Redémarrage du pare-feu en attente | Une mise à jour est installée mais a besoin d'un redémarrage pour prendre effet (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 j |
| Mises à jour du pare-feu en attente | Le pare-feu a des paquets en attente de mise à jour (`dumbmonit_opnsense_firmware_updates_pending`). C'est la machine la plus exposée du réseau. | > 0 | 1 h | Info (`info`) | 7 j |

### TrueNAS

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Pool NAS dégradé | ZFS n'appelle plus un pool sain (`dumbmonit_truenas_pool_healthy`). Un pool dégradé sert toujours ses données, c'est exactement pour ça que personne ne le remarque jusqu'à ce que le disque suivant tombe. La page de l'appareil nomme le disque. | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Erreurs de disque NAS | Des disques d'un pool ont rapporté des erreurs de lecture, d'écriture ou de checksum depuis le dernier `zpool clear` (`dumbmonit_truenas_pool_device_errors`, une série par type). | > 0 | 15 min | Avis (`warning`) | 24 h |
| Pool NAS presque plein | Un pool est plein à plus de 85 % (`dumbmonit_truenas_pool_used_percent`). ZFS ralentit bien avant d'être plein. Se résout à 80 %. | > 85 % | 30 min | Avis (`warning`), escalade après 7 j | 24 h |
| Scrub NAS a trouvé des erreurs | Le dernier scrub d'un pool a trouvé des données endommagées (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 min | Avertissement (`critical`) | 24 h |
| Scrub NAS en retard | Un pool n'a pas terminé de scrub depuis plus de 45 jours (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Absent après un resilver, qui efface l'enregistrement du dernier scrub. | > 45 j | 1 h | Info (`info`) | 7 j |
| Disque NAS a échoué son test SMART | Un autotest SMART dans le journal d'un disque a échoué (`dumbmonit_truenas_disk_smart_failed`). Un disque jamais testé n'a aucune série. | > 0 | 5 min | Avertissement (`critical`) | 24 h |
| Disque NAS trop chaud | Un disque est au-dessus de 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, lu depuis le cache de TrueNAS). Se résout à 50 °C. | > 55 °C | 15 min | Avis (`warning`) | 6 h |
| Dataset NAS proche de son quota | Un dataset a utilisé plus de 90 % de son quota (`dumbmonit_truenas_dataset_quota_used_percent`) : à 100 % ses écritures échouent tandis que le pool a encore de la place. Pas de quota, pas de série. | > 90 % | 30 min | Avis (`warning`) | 24 h |
| Réplication NAS échouée | Une tâche de réplication activée est en erreur (`dumbmonit_truenas_replication_error`) : la copie de l'autre côté devient plus vieille. | > 0 | 10 min | Avis (`warning`) | 12 h |
| Tâche de snapshot NAS échouée | Une tâche de snapshot périodique activée est en erreur (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 min | Avis (`warning`) | 12 h |
| Snapshots NAS périmés | Une tâche de snapshot périodique activée n'a pas tourné depuis plus de huit jours (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`) : une tâche qui arrête de tourner n'échoue pas, elle devient silencieuse. | > 8 j | 1 h | Avis (`warning`) | 24 h |
| Alerte TrueNAS levée | TrueNAS lui-même a levé une alerte de niveau ERROR ou plus (`dumbmonit_truenas_alerts`). Sa propre phrase est sur la page de l'appareil. | > 0 | 10 min | Avis (`warning`) | 12 h |
| Service NAS arrêté | Un service réglé pour démarrer avec le NAS est arrêté (`dumbmonit_truenas_service_running`). Un service éteint volontairement n'a aucune série. | < 1 | 10 min | Avis (`warning`) | 6 h |

### Synology DSM

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Avertissement SMART de disque Synology | DSM ne rapporte plus le statut SMART de ce disque comme `normal` (`dumbmonit_synology_disk_smart_status` : 1 attention, 2 critique, mots inconnus comptent comme 1). | > 0 | 15 min | Avis (`warning`) | 24 h |
| Disque Synology en échec | DSM rapporte le disque comme crashé ou en échec (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 min | Avertissement (`critical`) | 6 h |
| Secteurs défaillants de disque Synology | Le compteur de secteurs défaillants du disque dépasse le seuil réglé dans DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 min | Avertissement (`critical`) | 24 h |
| Secteurs défaillants de disque Synology en croissance | De nouveaux secteurs illisibles sont apparus dans les dernières 24 heures (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 min | Avis (`warning`) | 24 h |
| SSD Synology qui s'use | Un SSD a moins de 10 % de sa vie nominale restante (`dumbmonit_synology_disk_remaining_life_percent`) ; se résout au-dessus de 12 %. | < 10 % | 1 h | Avis (`warning`) | 7 j |
| Volume Synology dégradé | Un volume est dégradé ou crashé : sa redondance a disparu (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 min | Avertissement (`critical`) | 6 h |
| Volume Synology presque plein | Un volume est plein à 90 % ou plus (`dumbmonit_synology_volume_used_percent`) ; se résout sous 88 %. | ≥ 90 % | 15 min | Avis (`warning`), escalade après 24 h | 6 h |
| Température Synology élevée | Un disque est au-dessus de 55 °C, ou DSM a levé son propre avertissement de température (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`) ; se résout sous 52 °C. | > 55 °C | 15 min | Avis (`warning`) | 6 h |
| Mémoire Synology élevée | Usage mémoire du NAS au-dessus de 95 % (`dumbmonit_synology_memory_usage_percent`) ; se résout sous 90 %. | > 95 % | 15 min | Avis (`warning`) | 6 h |
| Tâche Active Backup échouée | La dernière exécution d'une tâche Active Backup for Business a échoué, ou n'a sauvegardé qu'une partie de ses appareils (`dumbmonit_abb_task_last_status == bool 0`). La notification nomme la tâche. | > 0 | 10 min | Avertissement (`critical`) | 24 h |
| Active Backup trop ancienne | Aucune exécution Active Backup for Business réussie pour cette tâche depuis plus de deux jours (`dumbmonit_abb_task_last_success_seconds`). La série n'existe pas pour une tâche qui n'a jamais réussi ; « Tâche Active Backup échouée » parle alors. | > 2 j | 1 h | Avis (`warning`) | 24 h |
| Tâche Active Backup désactivée | La tâche n'a pas de calendrier, ou sa sauvegarde continue est en pause (`dumbmonit_abb_task_enabled == bool 0`) : elle ne sauvegardera rien jusqu'à ce que quelqu'un la lance. | > 0 | 1 h | Info (`info`) | 7 j |
| Appareil Active Backup en retard | Un appareil a dépassé, sans exécution Active Backup for Business réussie, ce que son propre rythme autorise, jours de repos exclus (`dumbmonit_abb_device_overdue`, voir [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 min | Avis (`warning`) | 24 h |
| Appareil Active Backup en échec | Les deux dernières tentatives ou plus d'un appareil ont échoué (`dumbmonit_abb_device_consecutive_failures`) ; une exécution annulée ne compte comme aucune des deux. | ≥ 2 | 10 min | Avertissement (`critical`) | 24 h |

### Serveurs de logs et de métriques {#log-and-metrics-servers}

VictoriaMetrics, VictoriaLogs, Loki et Graylog
([appareils](../devices/victoriametrics.md)). Les compteurs passent par
`increase_prometheus`, un serveur ajouté avec un historique de données
refusées ne se déclenche donc pas sur sa première mesure.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Stockage de métriques ou de logs en lecture seule | L'espace disque libre est tombé sous `-storage.minFreeDiskSpaceBytes` et le stockage refuse chaque écriture (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 min | Avertissement (`critical`) | 1 h |
| Stockage de métriques ou de logs presque en lecture seule | Moins de 10 % de disque restant avant la lecture seule (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`) ; se résout au-dessus de 12 %. | < 10 % | 30 min | Avis (`warning`) | 24 h |
| Serveur de métriques ou de logs en mauvaise santé | `/health` ne répond pas OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 min | Avertissement (`critical`) | 6 h |
| Données de métriques ou de logs refusées | Échantillons ou lignes de log refusés dans la dernière heure : horodatages hors rétention, labels trop longs, limite de série (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 min | Avis (`warning`) | 24 h |
| Insertions lentes VictoriaMetrics | Plus de 5 % des nouveaux échantillons ont pris le chemin lent sur 15 minutes : pas assez de mémoire pour les séries actives (`100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])`) ; se résout sous 3 %. | > 5 % | 30 min | Avis (`warning`) | 24 h |
| Loki non prêt | `/ready` ne répond pas ready (`dumbmonit_loki_ready`). | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Loki refuse des lignes de log | Lignes refusées dans la dernière heure : trop anciennes, limite de débit ou de flux, ligne trop longue (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 min | Avis (`warning`) | 24 h |
| Loki ne peut pas écrire vers le stockage | Des chunks n'ont pas atteint le stockage dans les 30 dernières minutes ; ils sont perdus si Loki redémarre (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 min | Avertissement (`critical`) | 1 h |
| Disque du journal d'écriture anticipée de Loki plein | Écritures WAL perdues pour un disque plein (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 min | Avertissement (`critical`) | 1 h |
| Erreurs de requête Loki | Plus de cinq requêtes répondues par une erreur serveur en 15 minutes (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 min | Avis (`warning`) | 6 h |
| Graylog ne traite pas | Traitement des messages en pause sur le nœud (`dumbmonit_graylog_processing`). | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Cluster de recherche Graylog en panne | OpenSearch ou Elasticsearch rouge, ou inaccessible depuis Graylog (`dumbmonit_graylog_indexer_status` : 2 rouge, 3 inaccessible). | ≥ 2 | 5 min | Avertissement (`critical`) | 1 h |
| Cluster de recherche Graylog jaune | Shards de réplica non assignés depuis une demi-heure (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 min | Avis (`warning`) | 24 h |
| Journal Graylog qui se remplit | Le journal est plein à plus de moitié : la sortie ne tient pas le rythme (`dumbmonit_graylog_journal_used_percent`) ; se résout sous 40 %. | > 50 % | 10 min | Avis (`warning`), escalade après 1 h | 6 h |
| Input Graylog en échec | Un input n'a pas démarré (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 min | Avis (`warning`) | 24 h |
| Échecs d'indexation Graylog | Écritures vers le cluster de recherche ou traitement de message en échec dans la dernière heure (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 min | Avis (`warning`) | 24 h |

### Applications auto-hébergées {#self-hosted-applications}

Nextcloud, Immich, Paperless-ngx, Jellyfin et Plex
([appareils](../devices/nextcloud.md)). Elles tombent rarement en
panne franche : la page s'ouvre pendant que la synchro, les vignettes,
les imports ou les scans de bibliothèque se sont arrêtés. Ces règles
surveillent ces états ; une application qui ne répond pas du tout est
Appareil inaccessible.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Nextcloud bloqué en maintenance | Le mode maintenance est actif (`dumbmonit_nextcloud_maintenance`) : personne ne peut se connecter ni synchroniser. | > 0 | 30 min | Avis (`warning`), escalade après 3 h | 6 h |
| Mise à niveau de base de données Nextcloud en attente | Le code a été mis à jour et Nextcloud attend `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 min | Avertissement (`critical`) | 1 h |
| Mise à jour Nextcloud disponible | La vérification de mise à jour de Nextcloud a trouvé une release plus récente (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 j |
| OPcache Nextcloud plein | L'OPcache de PHP n'a plus de place et les pages ralentissent (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 min | Avis (`warning`) | 24 h |
| Espace libre Nextcloud faible | Moins de 5 Gio libres pour le répertoire de données (`dumbmonit_nextcloud_free_space_bytes / 1073741824`) ; se résout au-dessus de 6 Gio. | < 5 Gio | 30 min | Avis (`warning`) | 24 h |
| Tâches Immich bloquées | Des tâches attendent dans une file qui n'est pas en pause et qui n'exécute rien (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). La notification nomme la file. | > 0 | 1 h | Avis (`warning`) | 6 h |
| File Immich en pause | Une file de tâches est en pause depuis un jour (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 j |
| Stockage Immich presque plein | Le disque qui contient la bibliothèque est plein à plus de 90 % (`dumbmonit_immich_storage_used_percent`) ; se résout sous 88 %. | > 90 % | 30 min | Avis (`warning`) | 24 h |
| Mise à jour Immich disponible | La vérification de version d'Immich connaît une release plus récente (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 j |
| File d'attente de tâches Paperless arrêtée | Redis ou Celery en erreur (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})` : 2 est une erreur) : rien de nouveau n'est importé. | ≥ 2 | 5 min | Avertissement (`critical`) | 1 h |
| Erreur d'index ou de contrôle de cohérence Paperless | L'index de recherche, le classificateur ou le contrôle de cohérence (fichiers manquants ou altérés) rapporte une erreur (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 min | Avis (`warning`) | 24 h |
| Migrations Paperless en attente | Migrations de base de données restant à appliquer (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 min | Avis (`warning`) | 24 h |
| Tâche Paperless en échec | Une tâche que cet utilisateur peut voir a échoué dans la fenêtre des tâches en échec et n'a pas été rejetée (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 min | Avis (`warning`) | 24 h |
| Stockage Paperless presque plein | Le disque qui contient les documents est plein à plus de 90 % (`dumbmonit_paperless_storage_used_percent`) ; se résout sous 88 %. | > 90 % | 30 min | Avis (`warning`) | 24 h |
| Mise à jour Paperless disponible | La vérification de version de Paperless-ngx connaît une release plus récente (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 j |
| Échec d'une tâche planifiée Jellyfin | Une tâche planifiée (un scan de bibliothèque, un nettoyage) a échoué ou a été interrompue à sa dernière exécution (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 min | Avis (`warning`) | 24 h |
| Extension Jellyfin cassée | Une extension n'a pas pu se charger ou ne supporte pas cette version (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 min | Avis (`warning`) | 24 h |
| Redémarrage Jellyfin en attente | Jellyfin attend un redémarrage depuis un jour après l'installation ou la mise à jour d'une extension (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 j |
| Mise à jour Plex disponible | La vérification de mise à jour de Plex a trouvé une release plus récente (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 j |

### Forges auto-hébergées {#self-hosted-forges}

GitLab (self-managed) et Forgejo/Gitea
([appareils](../devices/gitlab.md)). Une instance qui ne répond rien
du tout est Appareil inaccessible ; ces règles surveillent les parties
qui continuent de servir des pages tout en prenant discrètement du
retard — Sidekiq, les runners CI/CD, une tâche planifiée qui a arrêté
de tourner.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Runners GitLab tous hors ligne | Chaque runner CI/CD que GitLab connaît est hors ligne, et au moins un est enregistré (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 min | Avis (`warning`) | 6 h |
| Retard Sidekiq GitLab élevé | Une file Sidekiq a plus de 500 jobs en attente (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`) : notifications, webhooks et imports prennent du retard. | > 500 | 15 min | Avis (`warning`) | 6 h |
| Migrations GitLab en attente | Des migrations de base de données d'une mise à niveau restent à appliquer (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Info (`info`) | 24 h |
| Licence GitLab arrivant à expiration | La licence Enterprise Edition expire dans moins de 30 jours (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 j | 1 h | Avis (`warning`) | 24 h |
| Runners Forgejo/Gitea tous hors ligne | Chaque runner Actions est hors ligne, et au moins un est enregistré (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 min | Avis (`warning`) | 6 h |
| Tâche planifiée Forgejo/Gitea en retard | Une tâche planifiée (contrôle de dépôt, nettoyage, vérificateur de mise à jour) n'a pas tourné à son propre intervalle (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 min | Avis (`warning`) | 24 h |

### Appareils clients {#client-devices}

Une règle, partagée par chaque intégration qui suit des appareils qui
se connectent et sauvegardent — les téléphones, tablettes et
ordinateurs d'Immich ([appareils](../devices/immich.md)), les machines
sauvegardées de Proxmox Backup Server, une par groupe de sauvegarde
([appareils](../devices/pbs.md)), et les ordinateurs et VM protégés de
Veeam, un par objet avec un point de restauration
([appareils](../devices/veeam.md)), aujourd'hui ; plus à venir. Chaque
intégration calcule l'écart par rapport à la propre fenêtre de
péremption de sa cible (`device_stale_days`, 3 jours par défaut) et le
publie comme `dumbmonit_client_device_stale_seconds`, étiqueté
`device`, `type`, `os`, `user`, `kind` et `signal` (`connection` ou
`backup`) ; cette seule règle se déclenche alors pour n'importe lequel
d'eux, nommant l'appareil dans la notification. Synology Active Backup
for Business suit ses propres appareils séparément, via un mécanisme
sensible au rythme (`synology_abb_device_*`) plutôt que cette famille
générique — voir [Synology](../devices/synology.md).

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Appareil client périmé | Un appareil ne s'est pas connecté ou n'a pas sauvegardé dans la fenêtre de péremption configurée pour cette cible (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Avis (`warning`) | 24 h |

### Pi-hole {#pi-hole}

[Pi-hole](../devices/pihole.md) v6, lu via son API REST.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Blocage Pi-hole désactivé | Le blocage est désactivé, en pause ou en échec (`dumbmonit_pihole_blocking_enabled`). Le maintien laisse passer une pause de cinq minutes depuis l'interface web. | < 1 | 15 min | Avis (`warning`) | 6 h |
| Listes de blocage Pi-hole obsolètes | Les listes de blocage (gravity) n'ont pas été reconstruites depuis plus de huit jours (`dumbmonit_pihole_gravity_age_seconds`) : la mise à jour hebdomadaire échoue. | > 8 j | 1 h | Avis (`warning`) | 24 h |
| Mise à jour Pi-hole disponible | Une version plus récente est disponible (`dumbmonit_pihole_updates_available`) : l'image en Docker, core, interface web ou FTL sinon. Pi-hole vérifie une fois par jour. | > 0 | 1 h | Info (`info`) | 7 j |
| Messages de diagnostic Pi-hole | Pi-hole liste des messages sur sa page de diagnostic (`dumbmonit_pihole_messages`) : une liste de blocage qui n'a pas pu se télécharger, un client limité en débit, un hôte surchargé. | > 0 | 15 min | Info (`info`) | 24 h |

### AdGuard Home {#adguard-home}

Le filtre DNS ([appareil](../devices/adguard.md)). Les chiffres de
requêtes et de blocages sont des sommes sur la propre fenêtre de
statistiques d'AdGuard Home, pas des compteurs : aucune règle n'est
construite sur eux.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Protection AdGuard Home désactivée | Protection désactivée, ou en pause et pas encore revenue (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 min | Avis (`warning`) | 6 h |
| DNS AdGuard Home ne tourne pas | L'interface web répond mais le serveur DNS est arrêté (`dumbmonit_adguard_running`). | < 1 | 5 min | Avertissement (`critical`) | 1 h |
| Serveur amont AdGuard Home en échec | Un serveur DNS amont échoue le propre test d'AdGuard Home, ou reste silencieux 4 secondes (`dumbmonit_adguard_upstream_up`, une série par serveur amont). | < 1 | 10 min | Avis (`warning`) | 6 h |
| Listes de filtres AdGuard Home obsolètes | La plus ancienne mise à jour d'une liste de filtres activée a plus de trois jours (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 jours | 1 h | Avis (`warning`) | 24 h |
| Mise à jour AdGuard Home disponible | Une version plus récente est annoncée par la propre vérification de version d'AdGuard Home (`dumbmonit_adguard_update_available`) ; ne se déclenche jamais quand cette vérification est désactivée, comme dans l'image Docker officielle. | > 0 | 1 h | Info (`info`) | 7 jours |

### Onduleur via NUT {#ups-behind-nut}

Onduleur lu depuis un [serveur NUT](../devices/nut.md). Chaque série
porte le label `ups`, une alerte nomme donc l'onduleur quand un serveur
en publie plusieurs. Les drapeaux de statut sont 0 ou 1 à chaque
lecture, une règle se résout donc d'elle-même une fois le secteur de
retour. Les onduleurs lus via SNMP sont couverts par les règles
[Onduleur](#ups) ci-dessus.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Onduleur sur batterie (NUT) | L'onduleur alimente la charge depuis la batterie (`dumbmonit_nut_ups_on_battery`, le drapeau `OB` de `ups.status`). | > 0 | 30 s | Avertissement (`critical`) | 15 min |
| Batterie d'onduleur faible (NUT) | L'onduleur rapporte une batterie faible, le drapeau `LB` (`dumbmonit_nut_ups_low_battery`) : les machines qu'il alimente vont bientôt s'arrêter. | > 0 | 1 min | Avertissement (`critical`) | 30 min |
| Batterie d'onduleur à remplacer (NUT) | L'onduleur demande le remplacement de sa batterie, le drapeau `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Avis (`warning`) | 7 j |
| Charge d'onduleur élevée (NUT) | La charge est restée au-dessus de 80 % (`dumbmonit_nut_ups_load_percent`) ; se résout sous 75 %. | > 80 % | 10 min | Avis (`warning`) | 24 h |
| Temps restant d'onduleur court (NUT) | L'onduleur estime moins de cinq minutes de temps restant sur batterie (`dumbmonit_nut_battery_runtime_seconds`) ; se résout au-dessus de six. Sur secteur, une batterie usée ou un onduleur trop petit pour sa charge. | < 300 s | 5 min | Avis (`warning`) | 24 h |
| Données d'onduleur périmées (NUT) | Le serveur NUT répond mais n'a pas de données fraîches de l'onduleur : câble, pilote ou onduleur silencieux (`dumbmonit_nut_ups_data_stale`). Une panne passerait inaperçue entre-temps. | > 0 | 5 min | Avis (`warning`), escalade après 1 h | 6 h |

### MikroTik RouterOS {#mikrotik-routeros}

Les séries portent le label `interface` ou `sensor`, une notification
nomme donc le port ou le capteur ([appareil](../devices/mikrotik.md)).
Les compteurs passent par `increase_prometheus` : un routeur ajouté
avec des années d'erreurs derrière lui ne se déclenche pas sur sa
première mesure.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| CPU MikroTik élevé | Le CPU du routeur est resté au-dessus de 90 % (`dumbmonit_mikrotik_cpu_load_percent`) : le routage, le pare-feu ou un script le sature. Se résout sous 80 %. | > 90 % | 15 min | Avis (`warning`) | 6 h |
| Mémoire MikroTik élevée | Plus de 90 % de la mémoire est utilisée (`dumbmonit_mikrotik_memory_used_percent`). Se résout sous 85 %. | > 90 % | 15 min | Avis (`warning`) | 6 h |
| Stockage MikroTik presque plein | Le stockage flash est plein à plus de 90 % : les changements de configuration et mises à niveau peuvent échouer (`dumbmonit_mikrotik_storage_used_percent`). Se résout sous 85 %. | > 90 % | 30 min | Avis (`warning`) | 24 h |
| Température MikroTik élevée | Un capteur de température lit au-dessus de 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Se résout à 75 °C. | > 80 °C | 10 min | Avis (`warning`) | 6 h |
| Alimentation ou ventilateur MikroTik en échec | Un capteur `psu*-state` ou `fan*-state` n'est pas `ok` (`dumbmonit_mikrotik_health_ok`). Une alimentation redondante qui meurt ne coupe rien, jusqu'à ce que la seconde le fasse. | < 1 | 5 min | Avertissement (`critical`) | 1 h |
| Interface MikroTik accumulant des erreurs | Plus de cinquante erreurs de réception et d'émission sur une interface dans la dernière heure (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 min | Avis (`warning`) | 24 h |
| Lien MikroTik instable | Une interface a perdu son lien plus de deux fois en trente minutes (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Un port débranché pour de bon compte une fois et ne se déclenche pas. | > 2 | 5 min | Avis (`warning`) | 1 h |
| Mise à jour RouterOS MikroTik disponible | Un RouterOS plus récent est disponible sur le canal de mise à jour du routeur (`dumbmonit_mikrotik_update_available`). Connu seulement quand le routeur vérifie lui-même les mises à jour : voir l'étape optionnelle de la [configuration](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 j |
| Mise à jour de firmware MikroTik en attente | RouterBOOT est plus ancien que le firmware fourni avec le RouterOS installé (`dumbmonit_mikrotik_firmware_upgrade_pending`) ; il n'est mis à niveau qu'à la demande, puis au prochain redémarrage. | > 0 | 1 h | Info (`info`) | 7 j |

### UniFi Network {#unifi-network}

[Page d'appareil](../devices/unifi.md). Les séries d'appareil portent
le nom, le modèle et le type de l'appareil, la notification dit donc
« Living room AP ». Un appareil en attente d'adoption n'a pas de série
`device_up` et ne compte jamais comme hors ligne.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Appareil UniFi hors ligne | Un appareil UniFi adopté par le contrôleur est hors ligne, isolé ou l'adoption a échoué (`dumbmonit_unifi_device_up`). | < 1 | 5 min | Avis (`warning`), escalade après 1 h | 6 h |
| Internet UniFi en panne | La passerelle UniFi rapporte aucune connexion Internet (`dumbmonit_unifi_internet_up`). Sans passerelle UniFi, la série n'existe pas et la règle reste silencieuse. | < 1 | 3 min | Avertissement (`critical`) | 1 h |
| Lien WAN UniFi en panne | Un lien WAN de la passerelle UniFi est en panne (`dumbmonit_unifi_wan_link_up`) : avec deux liens, la redondance est perdue tandis que rien ne semble cassé. Un port désactivé n'a aucune série. | < 1 | 5 min | Avis (`warning`) | 6 h |
| CPU de passerelle UniFi élevé | Le processeur de la passerelle UniFi est au-dessus de 90 % depuis quinze minutes (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`) ; se résout sous 80 %. | > 90 % | 15 min | Avis (`warning`) | 6 h |
| Appareil UniFi en attente d'adoption | Un appareil est apparu sur le réseau et attend d'être adopté dans UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 min | Info (`info`) | 24 h |
| Mise à jour de firmware UniFi disponible | Une mise à jour de firmware est disponible pour des appareils UniFi (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 j |
| Alarmes UniFi | UniFi a journalisé des événements critiques ou levé des alarmes dans les dernières 24 heures (`dumbmonit_unifi_alarms`). Lu seulement avec un compte View Only. | > 0 | 5 min | Avis (`warning`) | 24 h |

### Home Assistant

[Page d'appareil](../devices/homeassistant.md).

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Home Assistant en mode récupération | Home Assistant a démarré en mode récupération : sa configuration n'a pas pu être chargée et les automatisations ne tournent pas (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 min | Avertissement (`critical`) | 1 h |
| Home Assistant ne tourne pas | Home Assistant répond mais n'a pas fini de démarrer depuis dix minutes (`dumbmonit_homeassistant_running`). | < 1 | 10 min | Avis (`warning`) | 6 h |
| Entités Home Assistant devenues indisponibles | Au moins cinq entités de plus sont indisponibles qu'il y a une heure : une intégration, un hub ou une radio s'est arrêtée (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). La hausse compte, pas le total, qu'une installation réelle n'a jamais à zéro. | ≥ 5 | 10 min | Avis (`warning`) | 6 h |
| Batterie Home Assistant faible | Une batterie rapportée par Home Assistant est faible (`dumbmonit_homeassistant_battery_low`, une série par batterie sous le seuil de l'appareil, 20 % par défaut ; sa valeur est le niveau). | ≥ 0 % | 30 min | Info (`info`) | 3 j |
| Mise à jour Home Assistant disponible | Home Assistant a des mises à jour en attente : core, système d'exploitation, extensions ou firmware d'appareil (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 j |
| Réparation Home Assistant à traiter | Home Assistant a une réparation ouverte de gravité erreur ou critique (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 min | Avis (`warning`) | 24 h |

### VMware vSphere {#vmware-vsphere}

[Page d'appareil](../devices/vsphere.md). Les séries d'hôte portent
`host`, les séries de VM `vm`, les séries de datastore `datastore`, la
notification nomme donc l'objet.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Hôte vSphere ne répond pas | Un hôte vSphere ne répond pas à vCenter (`dumbmonit_vsphere_host_connection_state` : 2 ne répond pas). Un hôte déconnecté volontairement (1) ne se déclenche pas. | ≥ 2 | 3 min | Avertissement (`critical`) | 1 h |
| Santé d'hôte vSphere rouge | vSphere rapporte la santé globale d'un hôte comme rouge (`dumbmonit_vsphere_host_status == 2`). Gris signifie inconnu et ne se déclenche pas. | > 0 | 5 min | Avertissement (`critical`) | 1 h |
| Hôte vSphere laissé en maintenance | Un hôte vSphere est en mode maintenance depuis un jour : ses VM ont été déplacées et il a peut-être été oublié là (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| Datastore vSphere presque plein | Un datastore vSphere est plein à 90 % ou plus (`dumbmonit_vsphere_datastore_used_percent`) ; se résout sous 85 %. | ≥ 90 % | 15 min | Avis (`warning`), escalade après 24 h | 6 h |
| Datastore vSphere inaccessible | Un datastore vSphere est inaccessible : les VM stockées sur lui ne peuvent pas tourner (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 min | Avertissement (`critical`) | 1 h |
| VMware Tools vSphere non lancés | VMware Tools est installé mais ne tourne pas dans une VM allumée : l'invité est peut-être bloqué (`dumbmonit_vsphere_vm_tools_status == 2`). Les VM sans Tools ne se déclenchent pas. | > 0 | 30 min | Info (`info`) | 24 h |
| Alarme rouge vSphere | vSphere a levé une alarme rouge que personne n'a acquittée (`dumbmonit_vsphere_alarm{status="red"}`, une série par alarme, nommée d'après l'alarme et l'objet). | > 0 | 5 min | Avis (`warning`) | 6 h |

### Bases de données, courtier de messages et moteur de sécurité {#databases-message-broker-and-security-engine}

Redis / Valkey, MongoDB, RabbitMQ et CrowdSec
([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md),
[RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)).
Les compteurs passent par `increase_prometheus` ou `changes_prometheus`,
rien ne se déclenche donc sur la première mesure d'un appareil.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Mémoire Redis proche de maxmemory | Mémoire utilisée au-dessus de 90 % de `maxmemory` (`dumbmonit_redis_memory_used_percent`) ; se résout sous 85 %. Aucune série sans `maxmemory`. | > 90 % | 10 min | Avis (`warning`) | 6 h |
| Redis refuse des connexions | Connexions clientes refusées dans les 15 dernières minutes : `maxclients` atteint (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 min | Avertissement (`critical`) | 1 h |
| Lien de réplication Redis tombé | Un réplica a perdu son lien vers le primaire (`dumbmonit_redis_master_link_up`). | < 1 | 2 min | Avertissement (`critical`) | 1 h |
| Réplica Redis en retard | Vu depuis le primaire, un réplica n'a pas acquitté depuis plus de 30 secondes (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 min | Avis (`warning`) | 6 h |
| Snapshot Redis en échec | Le dernier `BGSAVE` a échoué ; avec `stop-writes-on-bgsave-error`, les écritures sont refusées (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 min | Avertissement (`critical`) | 1 h |
| Écriture du fichier append-only Redis en échec | La dernière écriture AOF a échoué (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 min | Avertissement (`critical`) | 1 h |
| Replica set MongoDB sans primaire | Aucun membre n'est primaire : aucune écriture n'est acceptée (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 min | Avertissement (`critical`) | 1 h |
| Membre MongoDB inaccessible | Un membre du replica set est inaccessible depuis les autres (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 min | Avertissement (`critical`) | 1 h |
| Retard de réplication MongoDB | Un secondaire est en retard de plus d'une minute sur le primaire (`dumbmonit_mongodb_replset_member_lag_seconds`) ; se résout sous 30 s. | > 60 s | 10 min | Avis (`warning`) | 6 h |
| Connexions MongoDB proches de la limite | Plus de 80 % des connexions disponibles utilisées (`dumbmonit_mongodb_connections_used_percent`) ; se résout sous 75 %. | > 80 % | 10 min | Avis (`warning`) | 6 h |
| Cache MongoDB sous pression | Plus de 20 % du cache WiredTiger est sale (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`) ; se résout sous 15 %. | > 20 % | 10 min | Avis (`warning`) | 6 h |
| Erreurs internes MongoDB | Assertions régulières dans la dernière heure (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 min | Avis (`warning`) | 24 h |
| Alarme mémoire ou disque RabbitMQ | Un nœud a levé une alarme de ressource : chaque publieur est bloqué (`dumbmonit_rabbitmq_alarm`, une série par nœud et ressource). | > 0 | 1 min | Avertissement (`critical`) | 1 h |
| Nœud RabbitMQ arrêté | Un nœud du cluster ne tourne pas (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 min | Avertissement (`critical`) | 1 h |
| Partition réseau RabbitMQ | Un nœud voit une partition réseau (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 min | Avertissement (`critical`) | 1 h |
| File RabbitMQ sans consommateur | Une file contient des messages et n'a aucun consommateur (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`) ; la valeur est le nombre de messages en attente. | > 0 | 15 min | Avis (`warning`) | 6 h |
| File RabbitMQ indisponible | Une file ne tourne pas : file quorum sans majorité, ou son nœud est en panne (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 min | Avis (`warning`) | 6 h |
| Local API CrowdSec en panne | La Local API ne répond pas à `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 min | Avertissement (`critical`) | 1 h |
| Bouncer CrowdSec arrêté de récupérer | Les requêtes d'un bouncer n'ont pas changé en 30 minutes (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 min | Avis (`warning`) | 6 h |
| CrowdSec ne lit aucun log | Pas une seule ligne de log lue en six heures (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 min | Avis (`warning`) | 24 h |

### Reverse proxies et domaines {#reverse-proxies-and-domains}

Traefik, Caddy, Nginx Proxy Manager et les enregistrements de domaine
([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md),
[Nginx Proxy Manager](../devices/npm.md),
[Expiration de domaine](../devices/domain.md)), et l'option *Alerter
quand la réponse change* de la vérification DNS
([Services](../devices/services.md#alerting-when-the-answer-changes)).
Les séries portent `router`, `service`, `server`, `upstream`, `host` ou
`cert`, la notification nomme donc ce qui a échoué.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Routeur Traefik désactivé | Un routeur est désactivé par une erreur de configuration (service ou middleware manquant, règle invalide) : ses sites ne sont pas servis (`dumbmonit_traefik_router_status` : 0 activé, 1 avertissement, 2 désactivé). | ≥ 2 | 5 min | Avis (`warning`) | 6 h |
| Serveur backend Traefik en panne | Un serveur d'un service en répartition de charge échoue son contrôle de santé (`dumbmonit_traefik_server_up`). | < 1 | 5 min | Avis (`warning`) | 6 h |
| Service Traefik sans serveur | Aucun serveur d'un service ne répond à son contrôle de santé : ses sites obtiennent 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 min | Avertissement (`critical`) | 1 h |
| Certificat Traefik non obtenu | Des routeurs demandent un certificat à un resolver qu'aucun certificat détenu par Traefik ne couvre encore : Traefik sert son certificat autosigné par défaut (`dumbmonit_traefik_resolver_routers_uncovered`, a besoin du `/api/certificates` de Traefik). | > 0 | 1 h | Avis (`warning`) | 24 h |
| Upstream Caddy en échec | Un backend échoue le contrôle de santé actif ou passif de Caddy (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 min | Avis (`warning`) | 6 h |
| Rechargement de configuration Caddy échoué | Caddy a refusé son dernier rechargement de configuration et fait toujours tourner le précédent (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 min | Avis (`warning`) | 24 h |
| Hôte Nginx Proxy Manager hors ligne | nginx a refusé la configuration d'un hôte activé : l'hôte n'est pas servi (`dumbmonit_npm_host_online`). | < 1 | 10 min | Avis (`warning`) | 6 h |
| Certificat de proxy non renouvelé | Un certificat détenu par Traefik ou présenté par un hôte Nginx Proxy Manager expire dans moins de 14 jours : le renouvellement automatique démarre à 30, il échoue donc (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 j | 1 h | Avis (`warning`) | 24 h |
| Reverse proxy répondant en 5xx | Plus de 5 % des réponses sur dix minutes étaient des 5xx, avec au moins cinquante requêtes (`increase_prometheus` de `dumbmonit_{traefik,caddy}_requests_5xx_total` sur `…_requests_total`). A besoin des métriques du proxy. | > 5 % | 10 min | Avis (`warning`) | 6 h |
| Domaine arrivant à expiration | Le domaine expire dans moins de 30 jours (`dumbmonit_domain_expiry_days >= 7`). | < 30 j | 1 h | Avis (`warning`) | 24 h |
| Domaine proche de l'expiration | Le domaine expire dans moins de 7 jours, ou a expiré (`dumbmonit_domain_expiry_days`). | < 7 j | 1 h | Avertissement (`critical`) | 12 h |
| Domaine suspendu | Le registre a mis le domaine en `clientHold` ou `serverHold` : il est hors du DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 min | Avertissement (`critical`) | 6 h |
| Domaine en rachat | Le domaine est en `redemptionPeriod`, `pendingDelete` ou `pendingRestore` : il a expiré (`dumbmonit_domain_redemption`). | > 0 | 30 min | Avertissement (`critical`) | 12 h |
| Réponse DNS changée | La réponse d'une vérification DNS avec *Alerter quand la réponse change* diffère de la précédente (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Se résout d'elle-même une heure plus tard. | > 0 | 1 min | Avis (`warning`) | 24 h |

### Changements de site web {#website-changes}

Voir [Changements de site web](../devices/webchange.md). La page de
l'appareil montre chaque changement avant et après.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Site web changé | Une vérification d'une page ou d'un site surveillé a trouvé des pages qui ont changé, sont apparues ou ont disparu (`dumbmonit_webchange_last_check_changes`). Chacune de ces vérifications est notifiée et nomme les pages, sous la même alerte tant qu'elle est active ; elle se résout à la première vérification qui ne trouve rien de nouveau. | > 0 | 30 s | Info (`info`) | aucun |

### Kubernetes {#kubernetes}

Voir [Kubernetes / k3s](../devices/kubernetes.md). Chaque série est
étiquetée avec le nœud, ou le namespace et le pod, workload ou demande
concerné.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Nœud Kubernetes non prêt | Un nœud n'est pas Ready : son kubelet ne rapporte plus, et ses pods seront évincés (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 min | Avertissement (`critical`) | 6 h |
| Nœud Kubernetes sous pression | Un nœud rapporte une pression mémoire, disque ou PID (`dumbmonit_k8s_node_pressure`, label `condition`). | > 0 | 5 min | Avis (`warning`) | 6 h |
| Pod Kubernetes en boucle de crash | Un conteneur est en `CrashLoopBackOff` (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m])` : l'état disparaît un instant à chaque tentative de redémarrage). | > 0 | 10 min | Avertissement (`critical`) | 6 h |
| Pod Kubernetes bloqué en attente | Un pod est Pending depuis quinze minutes : aucun nœud ne peut le prendre, ou un volume ou une image dont il a besoin n'arrive pas (`dumbmonit_k8s_pod_pending`). | > 0 | 15 min | Avis (`warning`) | 6 h |
| Pod Kubernetes redémarre | Un pod a redémarré plus de cinq fois dans la dernière heure sans être en `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 min | Avis (`warning`) | 6 h |
| Workload Kubernetes avec des répliques manquantes | Un Deployment, StatefulSet ou DaemonSet a moins de répliques disponibles qu'il n'en demande (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 min | Avis (`warning`), escalade après 1 h | 6 h |
| Demande de volume Kubernetes en attente | Un PersistentVolumeClaim dont un pod a besoin, ou que concerne une erreur de provisionnement, n'est toujours pas lié (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 min | Avis (`warning`) | 6 h |

### Active Directory {#active-directory}

Voir [Active Directory](../devices/activedirectory.md). Le contrôleur
de domaine que DumbMonit interroge est couvert par *Appareil
inaccessible* ; les autres contrôleurs de domaine du domaine par
*Contrôleur de domaine inaccessible* (label `dc`).

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Contrôleur de domaine inaccessible | Un contrôleur de domaine du domaine ne répond plus sur son port LDAP (`dumbmonit_ad_dc_reachable == bool 0`). Un nom qui ne se résout pas depuis DumbMonit ne produit aucune série et ne se déclenche jamais. | > 0 | 5 min | Avertissement (`critical`) | 6 h |
| Connexion Active Directory échouée | Le contrôleur de domaine répond mais refuse le compte de service : mot de passe expiré, compte désactivé ou verrouillé (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 min | Avis (`warning`) | 24 h |
| Appartenance à un groupe privilégié changée | Un membre a rejoint ou quitté Domain Admins, Enterprise Admins, Schema Admins, Administrators ou un groupe d'opérateurs, imbrication incluse (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, label `group`). Se résout d'elle-même une heure plus tard. | > 0 | 1 min | Avis (`warning`) | 24 h |
| Réplication Active Directory en échec | Le contrôleur de domaine interrogé ne peut pas répliquer depuis un partenaire (`dumbmonit_ad_replication_consecutive_failures`). Seulement quand le compte peut lire le statut de réplication. | > 0 | 30 min | Avis (`warning`) | 6 h |
| Mot de passe krbtgt plus ancien que 180 jours | Le mot de passe krbtgt a été changé pour la dernière fois il y a plus de 180 jours (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 jours | 10 min | Info (`info`) | aucun |

### pfSense {#pfsense}

Voir [pfSense](../devices/pfsense.md). Les passerelles sont étiquetées
`gateway`, les interfaces `interface` et `descr`, les services
`service`.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Passerelle pfSense en panne | Une passerelle pfSense est en panne : son adresse de surveillance ne répond plus, et le trafic qu'elle porte s'arrête ou bascule (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 min | Avertissement (`critical`) | 6 h |
| Passerelle pfSense perd des paquets | Une passerelle pfSense perd plus de 10 % des sondes vers son adresse de surveillance : la ligne est dégradée (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 min | Avis (`warning`) | 6 h |
| Passerelle pfSense lente | L'aller-retour vers l'adresse de surveillance d'une passerelle pfSense est resté au-dessus de 500 ms pendant quinze minutes (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 min | Avis (`warning`) | 6 h |
| Interface pfSense sans lien | Une interface pfSense activée n'a pas de lien : câble, port de commutateur ou carte réseau (`dumbmonit_pfsense_interface_down`). | > 0 | 5 min | Avis (`warning`) | 6 h |
| Service pfSense arrêté | Un service pfSense activé ne tourne pas : résolveur DNS, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 min | Avis (`warning`) | 6 h |
| Disque pfSense presque plein | Le disque pfSense est plein à plus de 90 % : les logs, rapports et mises à jour de paquet arrêtent de s'écrire (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 min | Avis (`warning`) | 1 j |

### Unraid {#unraid}

Voir [Unraid](../devices/unraid.md). Les disques sont étiquetés `disk`
et `role` (`parity`, `data`, `cache`), les conteneurs `container`.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Array Unraid arrêté | L'array Unraid n'est pas démarré : partages, conteneurs et VM qui y vivent sont indisponibles (`dumbmonit_unraid_array_started`). | < 1 | 10 min | Avertissement (`critical`) | 6 h |
| Disque Unraid désactivé ou manquant | Un disque Unraid est désactivé, manquant ou invalide : l'array tourne en mode dégradé, et une panne de plus perd des données (`dumbmonit_unraid_disk_ok`). | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Erreurs de lecture disque Unraid | Un disque Unraid a rapporté des erreurs de lecture depuis le démarrage de l'array : il sera peut-être désactivé à la prochaine (`dumbmonit_unraid_disk_errors`). | > 0 | 5 min | Avis (`warning`) | 1 j |
| Disque Unraid trop chaud | Un disque de données ou de parité Unraid est resté au-dessus de 55 °C pendant quinze minutes (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 min | Avis (`warning`) | 6 h |
| Vérification de parité Unraid a trouvé des erreurs | La dernière vérification de parité Unraid a échoué ou a trouvé des erreurs de synchro : la parité ne correspond plus aux données (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 min | Avis (`warning`) | 1 j |
| Vérification de parité Unraid en retard | La dernière vérification de parité Unraid a plus de 40 jours : une vérification mensuelle a été manquée (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 j | 1 h | Avis (`warning`) | 1 j |
| Cache Unraid presque plein | Un pool de cache Unraid est plein à plus de 90 % : les écritures se replient sur l'array lent, ou échouent (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 min | Avis (`warning`) | 1 j |
| Conteneur Unraid arrêté | Un conteneur Docker réglé pour démarrer avec l'array ne tourne pas (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 min | Avis (`warning`) | 6 h |

### Veeam Backup & Replication {#veeam-backup-replication}

Voir [Veeam Backup & Replication](../devices/veeam.md). Les jobs sont
étiquetés `job` et `type`, les dépôts `repository`.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Job Veeam en échec | La dernière exécution d'un job Veeam activé a échoué : ce qu'il protège n'a pas de nouveau point de restauration (`dumbmonit_veeam_job_failed`). | > 0 | 5 min | Avertissement (`critical`) | 1 j |
| Job Veeam terminé avec un avertissement | La dernière exécution d'un job Veeam activé s'est terminée avec un avertissement : certains objets ne sont peut-être pas protégés (`dumbmonit_veeam_job_warning`). | > 0 | 5 min | Avis (`warning`) | 1 j |
| Dépôt Veeam presque plein | Un dépôt de sauvegarde Veeam est plein à plus de 90 % : les prochains jobs échoueront par manque d'espace (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 min | Avis (`warning`) | 1 j |
| Licence Veeam arrivant à expiration | La licence Veeam expire dans moins de 30 jours : les jobs s'arrêtent à la fin de la période de grâce (`dumbmonit_veeam_license_expiry_seconds`). | < 30 j | 1 h | Avis (`warning`) | 1 j |

### Tailscale {#tailscale}

Voir [Tailscale](../devices/tailscale.md). Les appareils sont étiquetés
`device`, le premier label de leur nom MagicDNS.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Appareil Tailscale hors ligne | Un appareil Tailscale qui doit rester en ligne (un appareil étiqueté par défaut) est déconnecté depuis plus de 15 minutes (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 min | 5 min | Avis (`warning`) | 6 h |
| Clé de nœud Tailscale arrivant à expiration | La clé de nœud d'un appareil Tailscale expire dans moins de 14 jours : il quittera le tailnet jusqu'à ce que quelqu'un le reconnecte (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 j | 1 h | Avis (`warning`) | 1 j |
| Appareil Tailscale en attente d'approbation | Un appareil a rejoint le tailnet et attend l'approbation : approuvez-le s'il est attendu, supprimez-le sinon (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 min | Avis (`warning`) | 1 j |
| Mise à jour client Tailscale | Un appareil Tailscale qui doit rester en ligne fait tourner un client avec une mise à jour disponible (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Info (`info`) | 7 j |

### FortiGate {#fortigate}

Voir [FortiGate](../devices/fortigate.md). Les tunnels sont étiquetés
`tunnel`, les interfaces `interface` et `alias`, les licences
`license`.

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Tunnel IPsec FortiGate en panne | Aucune phase 2 d'un tunnel IPsec site à site FortiGate n'est active : le site derrière est coupé (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 min | Avertissement (`critical`) | 6 h |
| Interface FortiGate sans lien | Une interface FortiGate activée avec une adresse n'a pas de lien : câble, transceiver ou l'appareil devant (`dumbmonit_fortigate_interface_down`). | > 0 | 5 min | Avis (`warning`) | 6 h |
| HA FortiGate désynchronisé | Les membres d'un cluster HA FortiGate n'ont plus la même somme de contrôle de configuration : un basculement ferait tourner une autre configuration (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 min | Avis (`warning`) | 6 h |
| Membre HA FortiGate perdu | Un cluster HA FortiGate a moins de membres qu'il y a 24 heures : il n'y a plus de secours (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 min | Avertissement (`critical`) | 6 h |
| Mémoire FortiGate proche du mode conserve | La mémoire FortiGate est restée au-dessus de 85 % pendant dix minutes : à 88 % (par défaut) elle entre en mode conserve et arrête d'inspecter les nouvelles sessions (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 min | Avis (`warning`) | 6 h |
| CPU FortiGate élevé | Le CPU FortiGate est resté au-dessus de 90 % pendant quinze minutes (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 min | Avis (`warning`) | 6 h |
| Licence FortiGate arrivant à expiration | Une licence FortiGuard ou FortiCare du FortiGate expire dans moins de 30 jours, ou a expiré (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 j | 1 h | Avis (`warning`) | 1 j |
| Mise à jour firmware FortiGate | FortiGuard propose une release de correctif plus récente de la branche FortiOS en cours sur le FortiGate (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Info (`info`) | 7 j |

### Sophos Firewall {#sophos-firewall}

Voir [Sophos Firewall](../devices/sophos.md).

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Interface Sophos sans lien | Une interface Sophos Firewall allumée et liée à une zone n'a pas de lien (`dumbmonit_sophos_interface_down`). | > 0 | 5 min | Avis (`warning`) | 6 h |

### Hyper-V {#hyper-v}

Rapporté par l'agent Windows sur un hôte Hyper-V ; voir
[Agent](../devices/agent.md#hyper-v-hosts).

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Santé de VM Hyper-V critique | Hyper-V rapporte une machine virtuelle en santé critique : une ressource dont elle a besoin, le plus souvent l'espace disque, est épuisée (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 min | Avertissement (`critical`) | 6 h |
| CPU d'hôte Hyper-V élevé | Les processeurs logiques de l'hôte Hyper-V sont occupés à plus de 90 % depuis quinze minutes, invités inclus (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 min | Avis (`warning`) | 6 h |
| Erreurs de disque virtuel Hyper-V | Un disque virtuel Hyper-V a rapporté des erreurs d'E/S dans la dernière heure (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 min | Avis (`warning`) | 6 h |

### Score de sécurité {#security-score}

Calculé par le serveur toutes les 15 minutes pour chaque appareil dont
le type a des contrôles de sécurité, et stocké comme
`dumbmonit_security_score` (0–100). Voir
[Score de sécurité](../using/security-score.md).

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| Score de sécurité en baisse | Le score a chuté de 10 points ou plus par rapport à sa meilleure valeur des dernières 24 heures (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`) : un contrôle a commencé à échouer. | ≥ 10 pts | 30 min | Avis (`warning`) | 24 h |
| Score de sécurité faible | Le score est sous 40, note F (`dumbmonit_security_score`). Montez ou descendez le seuil pour correspondre à votre propre barre. | < 40 | 1 h | Info (`info`) | 7 j |

### DumbMonit lui-même

| Règle | Quoi | Seuil par défaut | Maintien | Gravité | Rappel |
|---|---|---|---|---|---|
| La sauvegarde de DumbMonit n'a pas tourné | La sauvegarde locale planifiée de la base de données DumbMonit n'a pas tourné depuis plus de deux jours (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 j | 1 h | Avis (`warning`) | 24 h |

Chaque règle intégrée s'applique à tous les appareils et à chaque canal
activé.

## Modifier une règle dans l'UI

Sur la page **Alertes**, la section *Règles* liste chaque règle avec sa
gravité, une marque *Intégrée* et, si pertinent, combien d'alertes elle
déclenche maintenant. Chaque règle a :

- un interrupteur pour **l'activer ou la désactiver** ;
- **Modifier**, qui déplie un éditeur en place pour le nom, la
  description, l'opérateur, le seuil, le maintien (`for`), la gravité,
  le rappel (`repeat every`) et l'escalade (`escalate after`). La
  requête d'une règle intégrée est montrée mais pas modifiable ; celle
  de vos propres règles l'est. Les règles de référence montrent leurs
  paramètres de détection en lecture seule ;
- **Supprimer**, seulement pour vos propres règles.

## Surcharges par appareil {#per-device-overrides}

Une règle peut être ajustée pour un appareil sans toucher la règle
elle-même : un seuil différent, un seuil de résolution différent, ou
désactivée pour cet appareil seul. L'éditeur de chaque règle a un
panneau **Surcharges par appareil** pour en ajouter ou en retirer une
directement.

L'action en un clic **Ignorer** sur une alerte (dans la liste *À
traiter* ou la page d'appareil de l'alerte) est le même mécanisme,
écrit avec `enabled: false` : l'appareil arrête de porter cette règle,
immédiatement, réversiblement. La page de l'appareil liste ce qu'il
ignore, avec **Arrêter d'ignorer** ; la règle elle-même montre *Ignorée
sur N appareils* dans Alertes → Règles, où le même panneau de
surcharges par appareil la retire. Voir
[Surcharges par appareil](../reference/api.md#per-device-overrides)
pour l'API.

## Créer une règle de seuil

Cliquez sur **Nouvelle règle**. Le formulaire demande :

| Champ | Signification |
|---|---|
| Nom | Montré dans les alertes et notifications. |
| Requête | Une expression MetricsQL, évaluée par appareil. Exemple : `dumbmonit_cpu_usage_percent`. |
| Opérateur | `>`, `>=`, `<` ou `<=`. |
| Seuil | Le nombre contre lequel le résultat de la requête est comparé. |
| Gravité | Info, Avis ou Avertissement. |
| Maintenir pendant | Secondes que la condition doit durer avant de se déclencher. Un maintien non nul absorbe un échantillon isolé mauvais. |

Chaque série renvoyée par la requête devient une alerte potentielle,
attachée à l'appareil nommé par son label `target`. Agrégez avec
`avg by (target, host) (…)` quand vous voulez une alerte par appareil
plutôt qu'une par cœur ou par interface. L'identité de l'alerte ignore
les labels `host` et `tag_*` dès que `target` est présent, renommer un
appareil garde donc ses alertes ; une série dont le `target` ne
correspond à aucun appareil existant et activé est ignorée.

## Le langage de requête

Les requêtes sont en [MetricsQL](https://docs.victoriametrics.com/metricsql/),
le sur-ensemble PromQL de VictoriaMetrics. Les métriques sont les
séries `dumbmonit_*` décrites dans la
[référence des métriques](../reference/metrics.md), et chaque série
porte un label `target` (l'id de l'appareil) et un label `host` (son
nom).

Quelques motifs utilisés par les règles intégrées :

```
# Âge du dernier échantillon : ce que "inaccessible" signifie
time() - tlast_over_time(dumbmonit_up[7d])

# Une valeur par appareil, quelle que soit la source
avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)

# Un pourcentage calculé depuis deux jauges
100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total

# Une condition booléenne : la série n'est renvoyée que quand elle est vraie
dumbmonit_ups_output_source == 5

# Extrapolation faite par VictoriaMetrics
predict_linear(dumbmonit_pbs_datastore_used_percent[6h], 345600) and deriv(dumbmonit_pbs_datastore_used_percent[6h]) > 0

# Disponibilité d'un service sur trente jours
avg_over_time(dumbmonit_probe_success[30d])
```

!!! tip "Essayez une requête d'abord"
    `GET /api/metrics/query?query=…` renvoie ce que la règle évaluerait
    (voir la [référence de l'API](../reference/api.md#metrics)). En
    développement, la surcouche publie le VictoriaMetrics embarqué sur
    le `:8428` de l'hôte, avec son interface de requête.

## Types de règle dans l'API

`kind` est `threshold` (comparer avec un seuil), `predict` (la même
comparaison, sur une requête `predict_linear` ; montré comme une
prévision) ou `anomaly` (référence saisonnière ; `params` contient `k`,
`alpha`, `mad_floor_abs`, `mad_floor_rel`, `min_samples`). L'UI crée des
règles `threshold` ; les deux autres types peuvent être créés via
l'API.
