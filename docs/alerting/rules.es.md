# Reglas {#rules}

Las reglas se insertan en el primer arranque, y luego te pertenecen:
DumbMonit nunca reescribe una regla que cambiaste, solo recrea las que
faltan. Las reglas integradas se pueden editar y desactivar, no
eliminar.

## Reglas integradas {#built-in-rules}

Las severidades se muestran con la palabra de la UI; el valor de la
API está entre paréntesis. Una regla cuya métrica ningún dispositivo
produce no tiene serie y nunca se activa, así que cada regla de abajo
se distribuye activada en cada instancia.

### Todo dispositivo {#every-device}

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Device unreachable | Ninguna medición recibida en más de tres minutos. `bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))` | > 180 s | 1 min | Warning (`critical`) | 6 h |
| High CPU | Carga de CPU sostenida por encima del 90%, promediada por dispositivo entre fuentes SNMP, Proxmox y agente; se limpia por debajo del 85 %. | > 90 % | 10 min | Advisory (`warning`), escala tras 1 h | 6 h |
| Unusual CPU | Carga de CPU notablemente distinta de lo habitual a esta hora y día de la semana (referencia estacional). | score > 3.5 | 15 min | Info (`info`) | 6 h |
| Disk almost full | Sistema de archivos al 90% de llenado o más, por punto de montaje (almacenamientos SNMP, Proxmox y sistemas de archivos raíz); se limpia por debajo del 88 %. | ≥ 90 % | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |
| Filesystem almost full | A este ritmo, el sistema de archivos se llenará dentro de cuatro días (`predict_linear` sobre 6 h; solo sistemas de archivos en crecimiento). | ≥ 100 % | 30 min | Advisory (`warning`) | 6 h |

### SAI {#ups}

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| UPS on battery | El SAI alimenta la carga desde la batería (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low | La batería del SAI está baja o agotada (`dumbmonit_ups_battery_status`, 3 = baja, 4 = agotada). | ≥ 3 | 1 min | Warning (`critical`) | 30 min |

### Puertos de red (SNMP) {#network-ports-snmp}

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Port accumulating errors | Un puerto de red registró más de cincuenta errores en la última hora: normalmente un cable que falla, una fibra sucia o un desajuste de duplex (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; nada se activa durante la primera hora de un dispositivo nuevo, así que los errores contados antes de añadirlo no se reportan como nuevos). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| Port flapping | Un puerto de red se cayó y volvió a subir más de dos veces en treinta minutos (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Un puerto desactivado a propósito no tiene serie y nunca se activa. | > 4 | 5 min | Advisory (`warning`) | 1 h |

### Hardware de servidor (Redfish) {#server-hardware-redfish}

Las series de salud son 0 (OK), 1 (Warning) o 2 (Critical). Una ranura
vacía (`Absent`) o desactivada no tiene serie, así que un servidor
entregado con una fuente de alimentación de dos nunca se activa.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Server fan failed | El controlador de gestión reporta un ventilador como fallido (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Server temperature above critical | Un sensor de temperatura está por encima del umbral crítico que declara el propio controlador (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Power supply redundancy lost | Las fuentes de alimentación ya no son redundantes: un fallo más y el servidor se cae (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 min | Advisory (`warning`), escala tras 1 h | 6 h |
| Power supply failed | El controlador reporta una fuente de alimentación como fallida (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Drive failure predicted | Un disco reporta un fallo previsto: reemplázalo mientras todavía funciona (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Server health critical | El controlador reporta la propia salud del sistema como Critical (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |

### Monitores de disponibilidad y heartbeats {#uptime-monitors-and-heartbeats}

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Service down | El servicio no ha respondido correctamente durante tres minutos (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 min | Warning (`critical`) | 30 min |
| Heartbeat missed | El trabajo no ha avisado dentro de su intervalo esperado más el período de gracia, o reportó un fallo él mismo (`dumbmonit_probe_success{probe="push"} == bool 0`; el retraso se define por dispositivo, consulta [Heartbeat](../devices/push.md)). | > 0 | 2 min | Advisory (`warning`) | 6 h |
| Service flapping | El servicio cambió de estado más de seis veces en treinta minutos (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 min | Advisory (`warning`) | 1 h |
| Slow service | El servicio tarda más de tres segundos en responder; las comprobaciones fallidas se excluyen (`dumbmonit_probe_duration_seconds and dumbmonit_probe_success == 1`). | > 3 s | 10 min | Advisory (`warning`) | 6 h |
| Certificate expiring soon | El certificado caduca en menos de catorce días. | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Certificate expired | El certificado ha caducado. | < 0 d | 5 min | Warning (`critical`) | 24 h |

### Contenedores Docker {#docker-containers}

Reportado por el agente; consulta [Agente](../devices/agent.md). La
alerta nombra el contenedor.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Container stopped | El contenedor lleva dos minutos detenido (`dumbmonit_container_up == bool 0`). Se excluye un contenedor que Docker no reiniciaría (`dumbmonit_container_stop_expected = 1`): un `docker run` puntual que terminó, un contenedor que nunca se inició, o uno detenido a mano sin política de reinicio. Un fallo (cualquier otro código de salida), una terminación por falta de memoria, o un contenedor `always` / `unless-stopped` sí cuentan. | > 0 | 2 min | Advisory (`warning`) | 1 h |
| Container unhealthy | La comprobación de salud del contenedor lleva tres minutos fallando (`dumbmonit_container_health == 2`; 0 ninguna, 1 sano, 3 iniciando). Un contenedor detenido se excluye: Docker conserva su último veredicto, que ya no significa nada. | > 0 | 3 min | Advisory (`warning`) | 1 h |
| Container restarting | El contenedor se reinició tres veces o más en quince minutos (`increase(dumbmonit_container_restart_count[15m])`): raramente está detenido lo suficiente para la regla anterior. | ≥ 3 | 1 min | Advisory (`warning`) | 1 h |
| Container update available | Hay una imagen más reciente disponible en el registro para este contenedor (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Copias de seguridad Plakar {#plakar-backups}

Reportado por el agente, por kloset y por fuente. Ninguna de estas
series existe mientras no se detecta Plakar, así que ninguna de las
dos reglas puede activarse en una máquina sin él.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Plakar backup too old | Ningún snapshot Plakar nuevo para esta fuente en más de dos días (`dumbmonit_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Plakar backup failed | El snapshot más reciente tiene errores (`dumbmonit_backup_last_status{source!="*"} < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |
| Plakar kloset unreadable | El agente no puede abrir el kloset: almacenamiento inaccesible, falta la frase de contraseña o el plugin; el log del agente dice cuál, una sola vez (`dumbmonit_backup_last_status{source="*"} < 1`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Copias de seguridad restic y Borg {#restic-and-borg-backups}

Reportado por el agente para cada repositorio declarado bajo
`restic_repos` o `borg_repos` en agent.yaml; consulta
[Agente](../devices/agent.md#restic-and-borg-backups). Sin un
repositorio declarado no hay serie, así que ninguna de las dos reglas
puede activarse.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| restic/Borg backup too old | Ningún snapshot restic o Borg nuevo en este repositorio en más de 48 horas (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Advisory (`warning`) | 24 h |
| restic/Borg repository unreachable | El agente no puede leer el repositorio: inaccesible, contraseña rechazada o binario faltante; el registro del agente dice cuál (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Túneles WireGuard {#wireguard-tunnels}

Reportado por el agente; consulta
[Agente](../devices/agent.md#wireguard-tunnels).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| WireGuard peer silent | Un peer con keepalive persistente no ha completado un handshake en más de 15 minutos: el túnel está caído (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Un peer que nunca se conectó cuenta su silencio desde el momento en que el agente lo vio por primera vez. Los peers sin keepalive (un teléfono, un portátil) están inactivos durante días y se excluyen; cambia el umbral para cambiar N, o quita la parte `and on …` para vigilar cada peer. | > 15 min | 5 min | Advisory (`warning`) | 6 h |

### Túneles IPsec {#ipsec-tunnels}

Reportado por el agente; consulta
[Agente](../devices/agent.md#ipsec-tunnels). La serie existe solo para un
CHILD_SA declarado en la configuración de strongSwan, así que la regla se
activa con un túnel que nunca subió, y no solo con uno que se cayó.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| IPsec tunnel down | Un túnel IPsec configurado no tiene ninguna sesión instalada (`dumbmonit_agent_ipsec_tunnel_up < 1`). | < 1 | 10 min | Advisory (`warning`) | 6 h |

### Prueba de velocidad de Internet {#internet-speed-test}

Reportado por el agente; consulta
[Agente](../devices/agent.md#internet-speed-test). La prueba está desactivada
por defecto y cada regla permanece en silencio hasta que se define el umbral
correspondiente en `agent.yaml`: nadie puede adivinar qué significa «lento»
para tu línea.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Internet download rate low | La última prueba midió una velocidad de descarga inferior a `speedtest_min_download_mbps` (`dumbmonit_agent_speedtest_download_min_bytes_per_second - dumbmonit_agent_speedtest_download_bytes_per_second > 0`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Internet upload rate low | Lo mismo, para `speedtest_min_upload_mbps`. | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Internet latency high | La última prueba midió una latencia superior a `speedtest_max_latency_ms`. | > 0 | 10 min | Advisory (`warning`) | 24 h |

### Discos, ZFS y sensores del agente {#agent-disks-zfs-and-sensors}

Reportado por el agente; consulta [Agente](../devices/agent.md).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Disk SMART failing | La propia autoevaluación SMART del disco lo reporta como fallando (`dumbmonit_agent_disk_smart_ok == bool 0`). Reemplázalo y revisa las copias de seguridad. | > 0 | 10 min | Warning (`critical`) | 24 h |
| ZFS pool degraded | Un pool ZFS ya no está sano: falta un dispositivo, está en fallo o retirado (`dumbmonit_agent_zfs_pool_health`: 0 online, 1 degraded, 2 cualquier otro estado). | > 0 | 5 min | Warning (`critical`) | 6 h |
| ZFS scrub found errors | El último scrub de ZFS encontró errores en este pool, o hay archivos perdidos de forma permanente (`dumbmonit_agent_zfs_pool_scrub_errors or dumbmonit_agent_zfs_pool_data_errors`). El pool puede seguir `ONLINE`. | > 0 | 10 min | Warning (`critical`) | 24 h |
| Temperature above critical | Un sensor supera la temperatura crítica que declara su propio hardware (`dumbmonit_agent_sensor_temperature_celsius >= dumbmonit_agent_sensor_temperature_critical_celsius`); sin valor fijo. | > 0 | 5 min | Warning (`critical`) | 6 h |

### Proxmox VE

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Backup too old | Ninguna copia de seguridad Proxmox VE exitosa en más de siete días (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 d | 1 h | Advisory (`warning`) | 24 h |
| VM or container stopped | El invitado estaba en ejecución en las últimas dos horas y lleva cinco minutos detenido (`1 - dumbmonit_proxmox_guest_running`, combinado con `max_over_time(…[2h]) == 1`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| VM or container CPU high | Un invitado ha usado más del 90 % de sus núcleos asignados durante quince minutos (`dumbmonit_proxmox_guest_cpu_percent`). La notificación nombra al invitado, `nextcloud (202)`. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| VM or container memory high | Un invitado ha usado más del 95 % de su memoria durante diez minutos (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 min | Advisory (`warning`) | 6 h |
| VM or container disk almost full | El disco raíz de un invitado está más del 90 % lleno (`dumbmonit_proxmox_guest_disk_used_percent`): contenedores siempre, VMs solo cuando el agente invitado QEMU reporta el uso. | > 90 % | 15 min | Advisory (`warning`) | 24 h |
| Guest locked | Una VM o contenedor lleva seis horas bloqueado (`dumbmonit_proxmox_guest_locked`, etiqueta `lock`): una copia o migración interrumpida dejó el bloqueo atrás y ninguna operación puede ejecutarse en el invitado, ni siquiera la siguiente copia de seguridad. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| Old snapshot | El snapshot más antiguo de un invitado tiene más de treinta días (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 d | 1 h | Info (`info`) | 7 d |
| HA resource in error | Un recurso HA de Proxmox está en error o fenced (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| HA manager not reporting | El gestor de recursos local de HA de un nodo dejó de escribir su estado (`dumbmonit_proxmox_ha_lrm_stale`): los invitados HA de ese nodo no se reiniciarán ni reubicarán. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Cluster lost quorum | El clúster Proxmox no tiene quórum (`1 - dumbmonit_proxmox_cluster_quorate`): ya no se pueden iniciar invitados. | > 0 | 1 min | Warning (`critical`) | 30 min |
| Proxmox node offline | Un nodo del clúster se reporta fuera de línea (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 min | Warning (`critical`) | 30 min |
| Proxmox service down | Un demonio central de Proxmox no está en ejecución en un nodo (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, cada uno contado solo donde el nodo lo activa). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Node network interface down | Una interfaz configurada para arrancar al inicio no está activa en un nodo (`dumbmonit_proxmox_node_interface_offline`). Una interfaz levantada a demanda nunca se reporta. | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Proxmox storage almost full | Un almacenamiento Proxmox está más del 85% lleno (`dumbmonit_proxmox_storage_used_percent`); «Disk almost full» toma el control al 90%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Thin pool almost full | Un pool thin LVM está más del 90 % lleno: cada invitado almacenado en él pasa a solo lectura cuando se llena (`dumbmonit_proxmox_node_thinpool_used_percent`); se limpia por debajo del 88 %. | > 90 % | 15 min | Warning (`critical`) | 6 h |
| Thin pool metadata almost full | El volumen de metadatos de un pool thin LVM está más del 80 % lleno (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). Es mucho más pequeño que el volumen de datos y a menudo se satura primero, con la misma consecuencia. | > 80 % | 15 min | Warning (`critical`) | 6 h |
| Proxmox disk wearing out | Un SSD o NVMe de un nodo ha usado más del 90 % de su vida nominal (`dumbmonit_proxmox_node_disk_wearout_percent`). La notificación se lee `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| Proxmox disk SMART failure | Un disco de un nodo reporta salud SMART `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| Proxmox ZFS pool degraded | Un pool ZFS de un nodo no está `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). La notificación se lee `pve2 · tank`. | > 0 | 5 min | Warning (`critical`) | 6 h |
| Backup job failed | El último job vzdump de un nodo, o un job de copia programado, falló (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 min | Warning (`critical`) | 24 h |
| Backup job excludes a disk | Un job de copia programado omite un disco de uno de los invitados que cubre (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): el job tiene éxito cada noche con esos datos faltando. Las unidades de CD-ROM e imágenes cloudinit se excluyen por diseño y nunca se cuentan. | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Replication failed | Un job de replicación reporta un error o un fail count distinto de cero (`dumbmonit_proxmox_replication_job_error`): la copia en espera está desactualizada. | > 0 | 10 min | Warning (`critical`) | 6 h |
| Ceph health error | Ceph reporta `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 min | Warning (`critical`) | 30 min |
| Ceph health warning | Ceph reporta `HEALTH_WARN` durante más de quince minutos (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Ceph OSD down | Un OSD de Ceph lleva cinco minutos caído (`1 - dumbmonit_proxmox_ceph_osd_up`). La notificación nombra el OSD y su host. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Ceph OSD nearly full | Un único OSD de Ceph está más del 85 % lleno (`dumbmonit_proxmox_ceph_osd_used_percent`): las escrituras se detienen cuando alcanza su límite, sea lo que sea que diga el promedio del clúster; se limpia por debajo del 82 %. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph pool almost full | Un pool de Ceph está más del 85 % lleno (`dumbmonit_proxmox_ceph_pool_used_percent`); se limpia por debajo del 82 %. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph noout flag left on | La flag `noout` de Ceph lleva dos horas activa (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): el rebalanceo está desactivado, y el clúster se lee sano mientras su redundancia se reduce. | > 0 | 2 h | Info (`info`) | 24 h |
| Proxmox updates pending | Más de veinte actualizaciones de paquete están pendientes en un nodo (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |
| Proxmox security updates pending | Al menos una actualización pendiente viene de un archivo de seguridad (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Proxmox packages changed | La versión instalada de un paquete de Proxmox cambió entre dos sondas — alguien ejecutó `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, publicado durante una hora con el detalle en su etiqueta `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Se resuelve por sí mismo una hora después. | > 0 | 1 min | Info (`info`) | 24 h |
| Node certificate expiring | Un certificado de nodo de Proxmox caduca en menos de catorce días (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |

### Proxmox Backup Server

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| PBS datastore almost full | Datastore PBS al 90% lleno o más. | > 90 % | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |
| PBS datastore filling up | Al ritmo actual, PBS estima que el datastore se llenará dentro de siete días. | < 7 d | 1 h | Advisory (`warning`) | 24 h |
| PBS datastore not mounted | Un datastore extraíble no está montado (`dumbmonit_pbs_datastore_removable_unmounted`): nada se puede escribir en él. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| PBS backup too old | Ningún snapshot nuevo para este grupo de copia (uno por máquina) en más de dos días (`dumbmonit_pbs_backup_last_age_seconds`). Edita el umbral para máquinas respaldadas semanalmente; la notificación nombra el grupo y el datastore. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| PBS backup verification failed | La verificación del snapshot más reciente de esta máquina falló (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 min | Warning (`critical`) | 24 h |
| PBS task failed | Al menos una tarea PBS falló en la ventana de revisión (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| PBS garbage collection too old | Ninguna recolección de basura exitosa en este datastore en más de ocho días (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| PBS garbage collection failed | La última recolección de basura en un datastore falló (`dumbmonit_pbs_gc_last_run_ok < 1`; de `/gc` en PBS 3.3+, de la última tarea de GC antes). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS corrupt chunks found | La recolección de basura encontró chunks ilegibles en un datastore (`dumbmonit_pbs_gc_bad_chunks`): algunas copias ya no se pueden restaurar. | > 0 | 10 min | Warning (`critical`) | 24 h |
| PBS prune job failed | La última ejecución de un job de prune PBS falló: nada se prunea y el datastore sigue llenándose (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). La notificación nombra el job y el datastore. | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS verification job failed | La última ejecución de un job de verificación PBS falló (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS sync job failed | La última ejecución de un job de sync PBS falló (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS tape backup failed | La última ejecución de copia a cinta falló (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): la copia fuera de línea no se está haciendo. | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS job never ran | Un job está activado y programado pero nunca se ha ejecutado (`dumbmonit_pbs_job_never_run`). | > 0 | 2 d | Advisory (`warning`) | 7 d |
| PBS service down | Un servicio que necesita Proxmox Backup Server no está en ejecución (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); ninguna copia se puede hacer mientras esté caído. | < 1 | 5 min | Warning (`critical`) | 6 h |
| PBS disk SMART failure | Un disco del servidor de copias reporta salud SMART `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). La notificación se lee `/dev/sdb`. | > 0 | 5 min | Warning (`critical`) | 24 h |
| PBS SSD worn out | Un SSD del servidor de copias ha usado más del 90 % de su resistencia nominal (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| PBS ZFS pool degraded | Un pool ZFS del servidor de copias no está `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| PBS certificate expiring | El certificado servido por el servidor de copias caduca en menos de tres semanas (`dumbmonit_pbs_node_certificate_expires_seconds`). Necesita la opción `certificates`. | < 21 d | 1 h | Advisory (`warning`) | 48 h |
| PBS restart pending after upgrade | El paquete se actualizó pero el demonio en ejecución todavía es el antiguo (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 d |
| PBS updates pending | Más de veinte actualizaciones de paquete están pendientes en el servidor de copias (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Datacenter Manager

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Federated instance unreachable | La consola de datacenter no puede alcanzar uno de los clústeres Proxmox VE o servidores de copias que federa (`dumbmonit_pdm_remote_reachable < 1`). La notificación nombra la instancia y su producto. | < 1 | 10 min | Warning (`critical`) | 6 h |
| Federated instance behind | Una instancia ejecuta una versión más antigua que otra instancia del mismo producto en el mismo patrimonio (`dumbmonit_pdm_remote_version_behind`). Comparado entre instancias, nunca contra lo que publica Proxmox. | > 0 | 1 h | Info (`info`) | 7 d |
| Task failed on a federated instance | Al menos una tarea falló en una instancia federada en la ventana de revisión (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Datacenter console disk almost full | El sistema de archivos raíz de la consola de datacenter está más del 90 % lleno (`dumbmonit_pdm_node_rootfs_percent`): lleno, la consola deja de registrar lo que recolecta. | > 90 % | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |
| Datacenter console certificate expiring | Un certificado de la consola de datacenter caduca en menos de catorce días (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Datacenter console updates pending | Más de veinte actualizaciones de paquete están pendientes en la consola de datacenter (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Mail Gateway

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Mail queue growing | La cola de correo diferido ha crecido en más de cien mensajes en dos horas (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): el siguiente salto probablemente está rechazando correo. Crecimiento en lugar de tamaño, porque una pasarela ocupada siempre mantiene unas docenas en diferido. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| Mail stuck in the queue | Un mensaje lleva más de cuatro horas esperando en una cola (`dumbmonit_pmg_queue_oldest_age_seconds`). La antigüedad es un límite inferior — qshape reporta rangos — así que la regla se activa tarde en lugar de temprano. La notificación nombra la cola. | > 4 h | 15 min | Warning (`critical`) | 6 h |
| Mail gateway service stopped | Un servicio que la pasarela necesita para aceptar y filtrar correo está detenido (`dumbmonit_pmg_service_running` para postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Una unidad no instalada no tiene serie y no puede activarse. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Virus signatures out of date | Las firmas diarias de virus de ClamAV tienen más de dos días (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): freshclam dejó de actualizarlas y la pasarela sigue filtrando con la lista de la semana pasada. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Spam rules out of date | Las reglas de SpamAssassin tienen más de ocho días (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Un canal que nunca se actualizó no tiene fecha ni serie. | > 8 d | 1 h | Advisory (`warning`) | 7 d |
| Quarantine filling up | La cuarentena de spam ha recibido más de mil mensajes en seis horas (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): una campaña, o una regla que acaba de meter un dominio legítimo en el bote. | > 1000 | 30 min | Advisory (`warning`) | 12 h |
| Mail gateway cluster degraded | Una pasarela del clúster ya no está sincronizada con las demás (`dumbmonit_pmg_cluster_node_insync`): filtra con una base de reglas desactualizada. Una pasarela independiente no tiene serie. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Mail gateway certificate expiring | Un certificado de la pasarela de correo caduca en menos de catorce días (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Mail gateway updates pending | Más de veinte actualizaciones de paquete están pendientes en la pasarela de correo (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### MDaemon y SecurityGateway {#mdaemon-and-securitygateway}

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| MDaemon mail service down | Un servicio de correo vigilado del servidor MDaemon no responde, o saluda para rechazar el servicio (`dumbmonit_mdaemon_service_up`). Solo los servicios listados en el dispositivo tienen serie. La notificación nombra el servicio y su puerto. | < 1 | 5 min | Warning (`critical`) | 6 h |
| MDaemon XML API not answering | La API XML dejó de responder mientras los servicios de correo siguen haciéndolo (`dumbmonit_mdaemon_api_up`): Remote Administration probablemente está detenido. Solo un dispositivo con una cuenta tiene la serie. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| MDaemon mail queue growing | La cola entrante, local o remota ha crecido en más de cien mensajes en dos horas (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): MDaemon dejó de entregar desde ella. Leído por el [agente](../devices/agent.md#windows-performance-counters) en el servidor MDaemon, así que la alerta está en el dispositivo del agente. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| MDaemon Bad queue not empty | Hay mensajes esperando en la cola Bad (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): MDaemon no pudo ni entregarlos ni rebotarlos, y se quedan ahí hasta que un administrador los libera o elimina. | > 0 | 30 min | Advisory (`warning`) | 24 h |
| MDaemon Retry queue high | Más de cincuenta mensajes esperan en la cola Retry (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): los servidores remotos siguen rechazando correo temporalmente. | > 50 | 1 h | Advisory (`warning`) | 6 h |
| SecurityGateway service down | Un servicio vigilado de la pasarela no responde, o saluda para rechazar el servicio (`dumbmonit_securitygateway_service_up`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| SecurityGateway API not answering | La API REST dejó de responder mientras los servicios siguen haciéndolo (`dumbmonit_securitygateway_api_up`). Solo un dispositivo con una clave de API tiene la serie. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| SecurityGateway delivery queue growing | La cola de entrega ha crecido en más de cien mensajes en dos horas (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): el servidor de correo detrás de la pasarela probablemente está rechazando correo. Los nombres de contador vienen de la API tal cual; si tu versión nombra la cola de forma distinta, ajusta el matcher. | > 100 | 15 min | Advisory (`warning`) | 6 h |

### OPNsense

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Internet gateway down | dpinger ya no obtiene respuesta de un gateway (`dumbmonit_opnsense_gateway_up`). En un cortafuegos multi-WAN el tráfico ya se movió a otro enlace, silenciosamente. `loss` y `delay` son avisos, no fallos, y no activan esta regla. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Gateway losing packets | Un gateway pierde más de un quinto de los paquetes enviados a él (`dumbmonit_opnsense_gateway_loss_percent`): el enlace responde, mal. Se limpia por debajo del 10 %. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| Gateway slow | El tiempo de ida y vuelta de un gateway se ha mantenido por encima de 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Trescientos milisegundos deja en paz un enlace de respaldo móvil. | > 300 ms | 15 min | Info (`info`) | 12 h |
| Firewall state table filling up | pf está rastreando más del 80 % de las conexiones que se le permiten (`dumbmonit_opnsense_pf_states_used_percent`). Más allá del límite el cortafuegos descarta conexiones nuevas sin ningún otro síntoma. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| Firewall network buffers exhausted | Más del 90 % de los buffers de red de FreeBSD están en uso (`dumbmonit_opnsense_mbuf_used_percent`). Un cortafuegos que se agota deja de reenviar con la CPU todavía inactiva. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| VPN tunnel down | Un túnel VPN configurado no ha tenido ninguna sesión durante diez minutos (`dumbmonit_opnsense_vpn_tunnel_up`). Un plugin no instalado, o una conexión desactivada, no tiene serie y no puede activarse. | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Firewall resolver stopped | Unbound dejó de responder (`dumbmonit_opnsense_unbound_running`). El cortafuegos sigue enrutando; las máquinas detrás de él ya no resuelven. Desactiva esta regla si tu cortafuegos resuelve con otra cosa. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Firewall core service stopped | dpinger, que vigila los gateways, o configd, que ejecuta todo lo demás, está detenido (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Solo esos dos, porque un servicio apagado a propósito no es un fallo. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Firewall too hot | Un sensor lee por encima de 85 °C (`dumbmonit_opnsense_temperature_celsius`): la caja sin ventilador en el armario está a punto de empezar a limitarse. Se limpia a 78 °C. | > 85 °C | 10 min | Advisory (`warning`) | 6 h |
| Firewall left in CARP maintenance mode | CARP se dejó en modo de mantenimiento persistente (`dumbmonit_opnsense_carp_maintenance_mode`): este cortafuegos entregó sus direcciones virtuales a su pareja y no las recuperará por sí mismo. | > 0 | 1 h | Info (`info`) | 24 h |
| Firewall reboot pending | Una actualización está instalada pero necesita un reinicio para aplicarse (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 d |
| Firewall updates pending | El cortafuegos tiene paquetes esperando actualización (`dumbmonit_opnsense_firmware_updates_pending`). Es la máquina más expuesta de la red. | > 0 | 1 h | Info (`info`) | 7 d |

### TrueNAS

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| NAS pool degraded | ZFS ya no considera un pool sano (`dumbmonit_truenas_pool_healthy`). Un pool degradado sigue sirviendo sus datos, que es exactamente por qué nadie se da cuenta hasta que falla el siguiente disco. La página del dispositivo nombra el disco. | < 1 | 5 min | Warning (`critical`) | 6 h |
| NAS disk errors | Discos de un pool han reportado errores de lectura, escritura o checksum desde el último `zpool clear` (`dumbmonit_truenas_pool_device_errors`, una serie por tipo). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| NAS pool almost full | Un pool está más del 85 % lleno (`dumbmonit_truenas_pool_used_percent`). ZFS se ralentiza bien antes de llenarse. Se limpia al 80 %. | > 85 % | 30 min | Advisory (`warning`), escala tras 7 d | 24 h |
| NAS scrub found errors | El último scrub de un pool encontró datos dañados (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS scrub overdue | Un pool no ha completado un scrub en más de 45 días (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Ausente tras un resilver, que borra el registro del último scrub. | > 45 d | 1 h | Info (`info`) | 7 d |
| NAS disk failed its SMART test | Una autoprueba SMART en el registro de un disco falló (`dumbmonit_truenas_disk_smart_failed`). Un disco nunca probado no tiene serie. | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS disk too hot | Un disco está por encima de 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, leído de la caché de TrueNAS). Se limpia a 50 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| NAS dataset near its quota | Un dataset ha usado más del 90 % de su cuota (`dumbmonit_truenas_dataset_quota_used_percent`): al 100 % sus escrituras fallan mientras el pool todavía tiene espacio. Sin cuota, sin serie. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| NAS replication failed | Una tarea de replicación activada está en error (`dumbmonit_truenas_replication_error`): la copia del otro lado se está volviendo más vieja. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshot task failed | Una tarea de snapshot periódico activada está en error (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshots stale | Una tarea de snapshot periódico activada no se ha ejecutado en más de ocho días (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): una tarea que deja de ejecutarse no falla, se queda en silencio. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| TrueNAS alert raised | El propio TrueNAS ha lanzado una alerta de nivel ERROR o superior (`dumbmonit_truenas_alerts`). Su propia frase está en la página del dispositivo. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS service stopped | Un servicio configurado para iniciarse con el NAS está detenido (`dumbmonit_truenas_service_running`). Un servicio apagado a propósito no tiene serie. | < 1 | 10 min | Advisory (`warning`) | 6 h |

### Synology DSM

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Synology disk SMART warning | DSM ya no reporta el estado SMART de este disco como `normal` (`dumbmonit_synology_disk_smart_status`: 1 atención, 2 crítico, palabras desconocidas cuentan como 1). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology disk failed | DSM reporta el disco como crashed o failed (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology disk bad sectors | El recuento de sectores dañados del disco supera el umbral definido en DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 min | Warning (`critical`) | 24 h |
| Synology disk bad sectors growing | Aparecieron sectores ilegibles nuevos en las últimas 24 horas (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology SSD wearing out | Un SSD tiene menos del 10 % de su vida nominal restante (`dumbmonit_synology_disk_remaining_life_percent`); se limpia por encima del 12 %. | < 10 % | 1 h | Advisory (`warning`) | 7 d |
| Synology volume degraded | Un volumen está degraded o crashed: su redundancia desapareció (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology storage pool degraded | Un pool de almacenamiento o caché SSD está degraded o crashed: falló un disco y la redundancia desapareció o se está reconstruyendo (`dumbmonit_synology_pool_status or dumbmonit_synology_ssd_cache_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology volume almost full | Un volumen está al 90 % lleno o más (`dumbmonit_synology_volume_used_percent`); se limpia por debajo del 88 %. | ≥ 90 % | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |
| Synology temperature high | Un disco está por encima de 55 °C, o DSM lanzó su propio aviso de temperatura (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); se limpia por debajo de 52 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| Synology memory high | Uso de memoria del NAS por encima del 95 % (`dumbmonit_synology_memory_usage_percent`); se limpia por debajo del 90 %. | > 95 % | 15 min | Advisory (`warning`) | 6 h |
| Active Backup task failed | La última ejecución de una tarea Active Backup for Business falló, o solo respaldó parte de sus dispositivos (`dumbmonit_abb_task_last_status == bool 0`). La notificación nombra la tarea. | > 0 | 10 min | Warning (`critical`) | 24 h |
| Active Backup too old | Ninguna ejecución exitosa de Active Backup for Business para esta tarea en más de dos días (`dumbmonit_abb_task_last_success_seconds`). La serie no existe para una tarea que nunca tuvo éxito; entonces habla «Active Backup task failed». | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Active Backup task disabled | La tarea no tiene programación, o su copia continua está en pausa (`dumbmonit_abb_task_enabled == bool 0`): no respaldará nada hasta que alguien la ejecute. | > 0 | 1 h | Info (`info`) | 7 d |
| Active Backup device overdue | Un dispositivo ha pasado más tiempo sin una ejecución exitosa de Active Backup for Business de lo que permite su propio ritmo, días libres excluidos (`dumbmonit_abb_device_overdue`, consulta [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Active Backup device failing | Los últimos dos o más intentos de un dispositivo fallaron (`dumbmonit_abb_device_consecutive_failures`); una ejecución cancelada no cuenta como ninguno. | ≥ 2 | 10 min | Warning (`critical`) | 24 h |

### Servidores de logs y métricas {#log-and-metrics-servers}

VictoriaMetrics, VictoriaLogs, Loki y Graylog
([dispositivos](../devices/victoriametrics.md)). Los contadores pasan
por `increase_prometheus`, así que un servidor añadido con un
historial de datos rechazados no se activa en su primera medición.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Metrics or log storage read-only | El espacio libre en disco cayó por debajo de `-storage.minFreeDiskSpaceBytes` y el almacenamiento rechaza cada escritura (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 min | Warning (`critical`) | 1 h |
| Metrics or log storage almost read-only | Menos del 10% de disco restante antes de solo lectura (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); se limpia por encima del 12 %. | < 10 % | 30 min | Advisory (`warning`) | 24 h |
| Metrics or log server unhealthy | `/health` no responde OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 min | Warning (`critical`) | 6 h |
| Metrics or log data refused | Muestras o líneas de log rechazadas en la última hora: timestamps fuera de retención, etiquetas demasiado largas, límite de serie (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| VictoriaMetrics slow inserts | Más del 5% de las muestras nuevas tomaron la vía lenta durante 15 minutos: no hay suficiente memoria para las series activas (`(100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])) and on (target) dumbmonit_victoriametrics_uptime_seconds > 900`); se limpia por debajo del 3 %. Solo se evalúa una vez que el servidor lleva 15 minutos en marcha, así que un reinicio (caché fría, contadores a cero dentro de la ventana) no la deja pendiente. | > 5 % | 30 min | Advisory (`warning`) | 24 h |
| Loki not ready | `/ready` no responde ready (`dumbmonit_loki_ready`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Loki refusing log lines | Líneas rechazadas en la última hora: demasiado antiguas, límite de tasa o stream, línea demasiado larga (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Loki cannot flush to storage | Chunks fallaron al llegar al almacenamiento en los últimos 30 minutos; se pierden si Loki se reinicia (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Loki write-ahead log disk full | Escrituras WAL perdidas por un disco lleno (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Loki request errors | Más de cinco peticiones respondidas con un error de servidor en 15 minutos (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 min | Advisory (`warning`) | 6 h |
| Graylog not processing | El procesamiento de mensajes está pausado en el nodo (`dumbmonit_graylog_processing`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Graylog search cluster down | OpenSearch o Elasticsearch rojo, o inaccesible desde Graylog (`dumbmonit_graylog_indexer_status`: 2 rojo, 3 inaccesible). | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Graylog search cluster yellow | Shards de réplica sin asignar durante media hora (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Graylog journal filling | El journal está más de la mitad lleno: la salida no va al día (`dumbmonit_graylog_journal_used_percent`); se limpia por debajo del 40 %. | > 50 % | 10 min | Advisory (`warning`), escala tras 1 h | 6 h |
| Graylog input failed | Un input no arrancó (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Graylog indexing failures | Escrituras al clúster de búsqueda o procesamiento de mensajes fallaron en la última hora (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### Aplicaciones autoalojadas {#self-hosted-applications}

Nextcloud, Immich, Paperless-ngx, Jellyfin y Plex
([dispositivos](../devices/nextcloud.md)). Rara vez se caen por
completo: la página se abre mientras la sincronización, miniaturas,
importaciones o escaneos de biblioteca se han detenido. Estas reglas
vigilan esos estados; una aplicación que no responde en absoluto es
Device unreachable.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Nextcloud stuck in maintenance | El modo mantenimiento está activado (`dumbmonit_nextcloud_maintenance`): nadie puede iniciar sesión ni sincronizar. | > 0 | 30 min | Advisory (`warning`), escala tras 3 h | 6 h |
| Nextcloud database upgrade pending | El código se actualizó y Nextcloud espera a `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Nextcloud update available | La comprobación de actualización de Nextcloud encontró una versión más reciente (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Nextcloud OPcache full | El OPcache de PHP está sin espacio y las páginas se ralentizan (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Nextcloud free space low | Menos de 5 GiB libres para el directorio de datos (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); se limpia por encima de 6 GiB. | < 5 GiB | 30 min | Advisory (`warning`) | 24 h |
| Immich jobs stalled | Hay trabajos esperando en una cola que no está en pausa y no ejecuta nada (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). La notificación nombra la cola. | > 0 | 1 h | Advisory (`warning`) | 6 h |
| Immich queue paused | Una cola de trabajos lleva un día en pausa (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 d |
| Immich storage almost full | El disco que contiene la biblioteca está más del 90 % lleno (`dumbmonit_immich_storage_used_percent`); se limpia por debajo del 88 %. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Immich update available | La comprobación de versión de Immich conoce una versión más reciente (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Paperless task queue down | Redis o Celery en error (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 es un error): nada nuevo se importa. | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Paperless index or sanity check error | El índice de búsqueda, el clasificador o la comprobación de integridad (archivos faltantes o alterados) reporta un error (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 min | Advisory (`warning`) | 24 h |
| Paperless migrations pending | Quedan migraciones de base de datos por aplicar (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Paperless task failed | Una tarea que este usuario puede ver falló dentro de la ventana de tareas fallidas y no se descartó (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Paperless storage almost full | El disco que contiene los documentos está más del 90 % lleno (`dumbmonit_paperless_storage_used_percent`); se limpia por debajo del 88 %. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Paperless update available | La comprobación de versión de Paperless-ngx conoce una versión más reciente (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Jellyfin scheduled task failed | Una tarea programada (un escaneo de biblioteca, una limpieza) falló o se abortó en su última ejecución (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Jellyfin plugin broken | Un plugin no cargó o no admite esta versión (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Jellyfin restart pending | Jellyfin lleva un día esperando un reinicio tras instalar o actualizar un plugin (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 d |
| Plex update available | La comprobación de actualización de Plex encontró una versión más reciente (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Forges autoalojados {#self-hosted-forges}

GitLab (autoalojado) y Forgejo/Gitea
([dispositivos](../devices/gitlab.md)). Una instancia que no responde
en absoluto es Device unreachable; estas reglas vigilan las partes que
siguen sirviendo páginas mientras se quedan atrás silenciosamente —
Sidekiq, runners de CI/CD, una tarea programada que dejó de
ejecutarse.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| GitLab runners all offline | Cada runner de CI/CD que conoce GitLab está fuera de línea, y al menos uno está registrado (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| GitLab Sidekiq backlog high | Una cola de Sidekiq tiene más de 500 trabajos esperando (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): notificaciones, webhooks e importaciones se retrasan. | > 500 | 15 min | Advisory (`warning`) | 6 h |
| GitLab migrations pending | Quedan migraciones de base de datos de una actualización por aplicar (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Info (`info`) | 24 h |
| GitLab license expiring | La licencia de Enterprise Edition caduca en menos de 30 días (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 24 h |
| Forgejo/Gitea runners all offline | Cada runner de Actions está fuera de línea, y al menos uno está registrado (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Forgejo/Gitea scheduled task overdue | Una tarea programada (comprobación de repositorio, limpieza, comprobador de actualizaciones) no se ejecutó en su propio intervalo (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 min | Advisory (`warning`) | 24 h |

### Dispositivos cliente {#client-devices}

Una regla, compartida por toda integración que rastrea dispositivos
que se conectan y hacen copia de seguridad — los teléfonos, tabletas y
escritorios de Immich ([dispositivos](../devices/immich.md)), las
máquinas respaldadas de Proxmox Backup Server, una por grupo de copia
([dispositivos](../devices/pbs.md)), y los equipos y VMs protegidos de
Veeam, uno por objeto con un punto de restauración
([dispositivos](../devices/veeam.md)), hoy; más seguirán. Cada
integración calcula el margen hasta la ventana de antigüedad de su
propio target (`device_stale_days`, 3 días por defecto) y lo publica
como `dumbmonit_client_device_stale_seconds`, etiquetado `device`,
`type`, `os`, `user`, `kind` y `signal` (`connection` o `backup`); esta
única regla entonces se activa para cualquiera de ellos, nombrando el
dispositivo en la notificación. Synology Active Backup for Business
rastrea sus propios dispositivos por separado, a través de un
mecanismo consciente del ritmo (`synology_abb_device_*`) en lugar de
esta familia genérica — consulta [Synology](../devices/synology.md).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Client device stale | Un dispositivo no se ha conectado o no ha hecho copia de seguridad dentro de la ventana de antigüedad configurada para este target (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Advisory (`warning`) | 24 h |

### Pi-hole

[Pi-hole](../devices/pihole.md) v6, leído a través de su API REST.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Pi-hole blocking disabled | El bloqueo está desactivado, en pausa o falló (`dumbmonit_pihole_blocking_enabled`). La espera deja pasar una pausa de cinco minutos desde la interfaz web. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Pi-hole blocklists stale | Las listas de bloqueo (gravity) no se han reconstruido en más de ocho días (`dumbmonit_pihole_gravity_age_seconds`): la actualización semanal falla. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| Pi-hole update available | Hay una versión más reciente disponible (`dumbmonit_pihole_updates_available`): la imagen en Docker, core, interfaz web o FTL en caso contrario. Pi-hole comprueba una vez al día. | > 0 | 1 h | Info (`info`) | 7 d |
| Pi-hole diagnosis messages | Pi-hole lista mensajes en su página de diagnóstico (`dumbmonit_pihole_messages`): una lista de bloqueo que no se descargó, un cliente limitado por tasa, un host sobrecargado. | > 0 | 15 min | Info (`info`) | 24 h |

### AdGuard Home

El filtro DNS ([dispositivo](../devices/adguard.md)). Las cifras de
consulta y bloqueo son sumas sobre la propia ventana de estadísticas
de AdGuard Home, no contadores: ninguna regla se construye sobre
ellas.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| AdGuard Home protection off | Protección desactivada, o en pausa y todavía sin volver (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 min | Advisory (`warning`) | 6 h |
| AdGuard Home DNS not running | La interfaz web responde pero el servidor DNS está detenido (`dumbmonit_adguard_running`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| AdGuard Home upstream failing | Un servidor DNS upstream falla la propia prueba de AdGuard Home, o se queda en silencio 4 segundos (`dumbmonit_adguard_upstream_up`, una serie por upstream). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| AdGuard Home filter lists stale | La actualización más antigua de una lista de filtro activada tiene más de tres días (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 days | 1 h | Advisory (`warning`) | 24 h |
| AdGuard Home update available | Una versión más reciente la anuncia la propia comprobación de versión de AdGuard Home (`dumbmonit_adguard_update_available`); nunca se activa cuando esa comprobación está desactivada, como en la imagen Docker oficial. | > 0 | 1 h | Info (`info`) | 7 days |

### SAI tras NUT {#ups-behind-nut}

SAI leído desde un [servidor NUT](../devices/nut.md). Cada serie lleva
la etiqueta `ups`, así que una alerta nombra el SAI cuando un servidor
publica varios. Las banderas de estado son 0 o 1 en cada lectura, así
que una regla se limpia por sí misma una vez que vuelve la corriente.
Los SAI leídos por SNMP están cubiertos por las reglas [SAI](#ups) de
arriba.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| UPS on battery (NUT) | El SAI alimenta la carga desde la batería (`dumbmonit_nut_ups_on_battery`, la bandera `OB` de `ups.status`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low (NUT) | El SAI reporta una batería baja, la bandera `LB` (`dumbmonit_nut_ups_low_battery`): las máquinas que alimenta están a punto de apagarse. | > 0 | 1 min | Warning (`critical`) | 30 min |
| UPS battery needs replacing (NUT) | El SAI pide que se reemplace su batería, la bandera `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| UPS load high (NUT) | La carga de trabajo se ha mantenido por encima del 80 % (`dumbmonit_nut_ups_load_percent`); se limpia por debajo del 75 %. | > 80 % | 10 min | Advisory (`warning`) | 24 h |
| UPS runtime short (NUT) | El SAI estima menos de cinco minutos de funcionamiento con batería (`dumbmonit_nut_battery_runtime_seconds`); se limpia por encima de seis. Con corriente, una batería desgastada o un SAI demasiado pequeño para su carga. | < 300 s | 5 min | Advisory (`warning`) | 24 h |
| UPS data stale (NUT) | El servidor NUT responde pero no tiene datos frescos del SAI: cable, driver o SAI en silencio (`dumbmonit_nut_ups_data_stale`). Una caída pasaría desapercibida mientras tanto. | > 0 | 5 min | Advisory (`warning`), escala tras 1 h | 6 h |

### MikroTik RouterOS

Las series llevan la etiqueta `interface` o `sensor`, así que una
notificación nombra el puerto o el sensor
([dispositivo](../devices/mikrotik.md)). Los contadores pasan por
`increase_prometheus`: un router añadido con años de errores detrás
no se activa en su primera medición.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| MikroTik CPU high | La CPU del router se ha mantenido por encima del 90 % (`dumbmonit_mikrotik_cpu_load_percent`): el enrutado, el cortafuegos o un script lo está saturando. Se limpia por debajo del 80 %. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik memory high | Más del 90 % de la memoria está en uso (`dumbmonit_mikrotik_memory_used_percent`). Se limpia por debajo del 85 %. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik storage almost full | El almacenamiento flash está más del 90 % lleno (`dumbmonit_mikrotik_storage_used_percent`): los cambios de configuración y actualizaciones pueden fallar. Se limpia por debajo del 85 %. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| MikroTik temperature high | Un sensor de temperatura lee por encima de 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Se limpia a 75 °C. | > 80 °C | 10 min | Advisory (`warning`) | 6 h |
| MikroTik power supply or fan failed | Un sensor `psu*-state` o `fan*-state` no está `ok` (`dumbmonit_mikrotik_health_ok`). Una fuente de alimentación redundante que murió no corta nada, hasta que lo hace la segunda. | < 1 | 5 min | Warning (`critical`) | 1 h |
| MikroTik interface accumulating errors | Más de cincuenta errores de recepción y transmisión en una interfaz en la última hora (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| MikroTik link flapping | Una interfaz perdió su enlace más de dos veces en treinta minutos (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Un puerto desconectado para siempre cuenta una vez y no se activa. | > 2 | 5 min | Advisory (`warning`) | 1 h |
| MikroTik RouterOS update available | Hay un RouterOS más reciente disponible en el canal de actualización del router (`dumbmonit_mikrotik_update_available`). Solo se conoce cuando el router comprueba las actualizaciones por sí mismo: consulta el paso opcional de la [configuración](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 d |
| MikroTik firmware upgrade pending | RouterBOOT es más antiguo que el firmware incluido con el RouterOS instalado (`dumbmonit_mikrotik_firmware_upgrade_pending`); solo se actualiza bajo petición, y luego en el siguiente reinicio. | > 0 | 1 h | Info (`info`) | 7 d |

### UniFi Network

[Página del dispositivo](../devices/unifi.md). Las series de
dispositivo llevan el nombre, modelo y tipo del dispositivo, así que
la notificación dice «Living room AP». Un dispositivo esperando
adopción no tiene serie `device_up` y nunca cuenta como fuera de
línea.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| UniFi device offline | Un dispositivo UniFi adoptado por el controlador está fuera de línea, aislado o falló al adoptar (`dumbmonit_unifi_device_up`). | < 1 | 5 min | Advisory (`warning`), escala tras 1 h | 6 h |
| UniFi Internet down | El gateway UniFi reporta ninguna conexión a Internet (`dumbmonit_unifi_internet_up`). Sin un gateway UniFi la serie no existe y la regla permanece en silencio. | < 1 | 3 min | Warning (`critical`) | 1 h |
| UniFi WAN link down | Un enlace WAN del gateway UniFi está caído (`dumbmonit_unifi_wan_link_up`): con dos enlaces, la redundancia se pierde mientras nada parece roto. Un puerto desactivado no tiene serie. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| UniFi gateway CPU high | El procesador del gateway UniFi ha estado por encima del 90% durante quince minutos (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); se limpia por debajo del 80 %. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| UniFi device waiting for adoption | Un dispositivo apareció en la red y espera ser adoptado en UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 min | Info (`info`) | 24 h |
| UniFi firmware update available | Hay una actualización de firmware disponible para dispositivos UniFi (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 d |
| UniFi alarms | UniFi registró eventos críticos o lanzó alarmas en las últimas 24 horas (`dumbmonit_unifi_alarms`). Solo se lee con una cuenta View Only. | > 0 | 5 min | Advisory (`warning`) | 24 h |

### Home Assistant

[Página del dispositivo](../devices/homeassistant.md).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Home Assistant in recovery mode | Home Assistant arrancó en modo recuperación: su configuración no se pudo cargar y las automatizaciones no se ejecutan (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| Home Assistant not running | Home Assistant responde pero no ha terminado de arrancar en diez minutos (`dumbmonit_homeassistant_running`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant entities went unavailable | Al menos cinco entidades más no disponibles que hace una hora: una integración, un hub o una radio se detuvo (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). Cuenta el aumento, no el total, que una instalación real nunca tiene en cero. | ≥ 5 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant battery low | Una batería reportada por Home Assistant está baja (`dumbmonit_homeassistant_battery_low`, una serie por batería por debajo del umbral del dispositivo, 20% por defecto; su valor es el nivel). | ≥ 0 % | 30 min | Info (`info`) | 3 d |
| Home Assistant update available | Home Assistant tiene actualizaciones pendientes: core, sistema operativo, add-ons o firmware de dispositivo (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Home Assistant repair to address | Home Assistant tiene una reparación abierta de severidad error o critical (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### VMware vSphere

[Página del dispositivo](../devices/vsphere.md). Las series de host
llevan `esxi`, las de VM `vm`, las de datastore `datastore`, así que
la notificación nombra el objeto.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| vSphere host not responding | Un host vSphere no responde a vCenter (`dumbmonit_vsphere_host_connection_state`: 2 sin respuesta). Un host desconectado a propósito (1) no se activa. | ≥ 2 | 3 min | Warning (`critical`) | 1 h |
| vSphere host health red | vSphere reporta la salud general de un host como roja (`dumbmonit_vsphere_host_status == 2`). Gris significa desconocido y no se activa. | > 0 | 5 min | Warning (`critical`) | 1 h |
| vSphere host left in maintenance | Un host vSphere lleva un día en modo mantenimiento: sus VMs se movieron y puede que se haya olvidado ahí (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| vSphere datastore almost full | Un datastore vSphere está al 90% lleno o más (`dumbmonit_vsphere_datastore_used_percent`); se limpia por debajo del 85 %. | ≥ 90 % | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |
| vSphere datastore inaccessible | Un datastore vSphere es inaccesible: las VMs almacenadas en él no pueden ejecutarse (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| vSphere VM tools not running | VMware Tools está instalado pero no en ejecución en una VM encendida: el invitado puede estar colgado (`dumbmonit_vsphere_vm_tools_status == 2`). Las VMs sin Tools no se activan. | > 0 | 30 min | Info (`info`) | 24 h |
| vSphere red alarm | vSphere lanzó una alarma roja que nadie confirmó (`dumbmonit_vsphere_alarm{status="red"}`, una serie por alarma, nombrada según la alarma y el objeto). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Bases de datos, broker de mensajes y motor de seguridad {#databases-message-broker-and-security-engine}

Redis / Valkey, MongoDB, RabbitMQ y CrowdSec
([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md),
[RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)).
Los contadores pasan por `increase_prometheus` o
`changes_prometheus`, así que nada se activa en la primera medición de
un dispositivo.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Redis memory near maxmemory | Memoria usada por encima del 90 % de `maxmemory` (`dumbmonit_redis_memory_used_percent`); se limpia por debajo del 85 %. Sin serie sin `maxmemory`. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| Redis refusing connections | Conexiones de cliente rechazadas en los últimos 15 minutos: se alcanzó `maxclients` (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Redis replication link down | Una réplica perdió su enlace con el primario (`dumbmonit_redis_master_link_up`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| Redis replica lagging | Visto desde el primario, una réplica no ha confirmado en más de 30 segundos (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 min | Advisory (`warning`) | 6 h |
| Redis snapshot failed | El último `BGSAVE` falló; con `stop-writes-on-bgsave-error`, las escrituras se rechazan (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| Redis append-only file write failed | La última escritura AOF falló (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replica set without primary | Ningún miembro es primario: no se acepta ninguna escritura (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| MongoDB member unreachable | Un miembro del replica set es inaccesible desde los demás (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replication lag | Un secundario se retrasa más de un minuto respecto al primario (`dumbmonit_mongodb_replset_member_lag_seconds`); se limpia por debajo de 30 s. | > 60 s | 10 min | Advisory (`warning`) | 6 h |
| MongoDB connections near the limit | Más del 80 % de las conexiones disponibles en uso (`dumbmonit_mongodb_connections_used_percent`); se limpia por debajo del 75 %. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB cache under pressure | Más del 20 % de la caché de WiredTiger está sucia (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); se limpia por debajo del 15 %. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB internal errors | Aserciones regulares en la última hora (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| RabbitMQ memory or disk alarm | Un nodo lanzó una alarma de recurso: todo publicador está bloqueado (`dumbmonit_rabbitmq_alarm`, una serie por nodo y recurso). | > 0 | 1 min | Warning (`critical`) | 1 h |
| RabbitMQ node down | Un nodo del clúster no está en ejecución (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ network partition | Un nodo ve una partición de red (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ queue without consumer | Una cola contiene mensajes y no tiene consumidor (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); el valor es el número de mensajes esperando. | > 0 | 15 min | Advisory (`warning`) | 6 h |
| RabbitMQ queue unavailable | Una cola no está en ejecución: cola de quórum sin mayoría, o su nodo está caído (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec Local API down | La Local API no responde a `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| CrowdSec bouncer stopped pulling | Las peticiones de un bouncer no cambiaron en 30 minutos (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec reads no logs | Ni una sola línea de log leída en seis horas (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Proxies inversos y dominios {#reverse-proxies-and-domains}

Traefik, Caddy, Nginx Proxy Manager y registros de dominio
([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md),
[Nginx Proxy Manager](../devices/npm.md), [Caducidad de
dominio](../devices/domain.md)), y la opción *Alert when the answer
changes* de la comprobación DNS
([Servicios](../devices/services.md#alerting-when-the-answer-changes)).
Las series llevan `router`, `service`, `server`, `upstream`, `proxy_host` o
`cert`, así que la notificación nombra lo que falló.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Traefik router disabled | Un router está desactivado por un error de configuración (servicio o middleware faltante, regla inválida): sus sitios no se sirven (`dumbmonit_traefik_router_status`: 0 activado, 1 aviso, 2 desactivado). | ≥ 2 | 5 min | Advisory (`warning`) | 6 h |
| Traefik backend server down | Un servidor de un servicio con balanceo de carga falla su comprobación de salud (`dumbmonit_traefik_server_up`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Traefik service without a server | Ningún servidor de un servicio responde a su comprobación de salud: sus sitios reciben 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| Traefik certificate not obtained | Routers piden a un resolver de certificados un certificado que ningún certificado que tiene Traefik cubre todavía: Traefik sirve su certificado autofirmado por defecto (`dumbmonit_traefik_resolver_routers_uncovered`, necesita `/api/certificates` de Traefik). | > 0 | 1 h | Advisory (`warning`) | 24 h |
| Caddy upstream unhealthy | Un backend falla la comprobación de salud activa o pasiva de Caddy (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Caddy configuration reload failed | Caddy rechazó su última recarga de configuración y sigue ejecutando la anterior (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 min | Advisory (`warning`) | 24 h |
| Nginx Proxy Manager host offline | nginx rechazó la configuración de un host activado: el host no se sirve (`dumbmonit_npm_host_online`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Proxy certificate not renewed | Un certificado que tiene Traefik o que presenta un host de Nginx Proxy Manager caduca en menos de 14 días: la renovación automática empieza a los 30, así que está fallando (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Reverse proxy answering 5xx | Más del 5 % de las respuestas en diez minutos fueron 5xx, con al menos cincuenta peticiones (`increase_prometheus` de `dumbmonit_{traefik,caddy}_requests_5xx_total` sobre `…_requests_total`). Necesita las métricas del proxy. | > 5 % | 10 min | Advisory (`warning`) | 6 h |
| Domain expiring | El dominio caduca en menos de 30 días (`dumbmonit_domain_expiry_days >= 7`). | < 30 d | 1 h | Advisory (`warning`) | 24 h |
| Domain about to expire | El dominio caduca en menos de 7 días, o ya caducó (`dumbmonit_domain_expiry_days`). | < 7 d | 1 h | Warning (`critical`) | 12 h |
| Domain on hold | El registro puso el dominio en `clientHold` o `serverHold`: está fuera del DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 min | Warning (`critical`) | 6 h |
| Domain in redemption | El dominio está en `redemptionPeriod`, `pendingDelete` o `pendingRestore`: ha caducado (`dumbmonit_domain_redemption`). | > 0 | 30 min | Warning (`critical`) | 12 h |
| DNS answer changed | La respuesta de una comprobación DNS con *Alert when the answer changes* difiere de la anterior (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Se resuelve por sí misma una hora después. | > 0 | 1 min | Advisory (`warning`) | 24 h |

### Cambios en sitios web {#website-changes}

Consulta [Cambios en sitios web](../devices/webchange.md). La página
del dispositivo muestra cada cambio antes y después.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Website changed | Una comprobación de una página o sitio vigilado encontró páginas que cambiaron, aparecieron o desaparecieron (`dumbmonit_webchange_last_check_changes`). Cada comprobación así se notifica y nombra las páginas, bajo la misma alerta mientras está activa; se resuelve en la primera comprobación que no encuentra nada nuevo. | > 0 | 30 s | Info (`info`) | ninguno |

### Kubernetes

Consulta [Kubernetes / k3s](../devices/kubernetes.md). Cada serie se
etiqueta con el nodo, o el namespace y el pod, workload o claim
afectado.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Kubernetes node not ready | Un nodo no está Ready: su kubelet ya no reporta, y sus pods serán expulsados (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Kubernetes node under pressure | Un nodo reporta presión de memoria, disco o PID (`dumbmonit_k8s_node_pressure`, etiqueta `condition`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes pod crash looping | Un contenedor está en `CrashLoopBackOff` y sigue reiniciándose: al menos dos reinicios en 15 minutos (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) and on (target, namespace, pod) increase_prometheus(dumbmonit_k8s_pod_restarts[15m]) >= 2`; el estado desaparece por un momento en cada intento de reinicio). Los contenedores terminados por un reinicio del nodo o del runtime (exit 137, 143 o 255, no `OOMKilled`) no cuentan, así que un reinicio del equipo no parece un bucle de fallos. | > 0 | 10 min | Warning (`critical`) | 6 h |
| Kubernetes pod stuck pending | Un pod lleva quince minutos en Pending: ningún nodo puede tomarlo, o un volumen o imagen que necesita no llega (`dumbmonit_k8s_pod_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Kubernetes pod restarting | Un pod se reinició más de cinco veces en la última hora sin estar en `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes workload missing replicas | Un Deployment, StatefulSet o DaemonSet tiene menos réplicas disponibles de las que pide (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 min | Advisory (`warning`), escala tras 1 h | 6 h |
| Kubernetes volume claim pending | Un PersistentVolumeClaim que un pod necesita, o que concierne a un error de provisión, todavía no está vinculado (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |

### Active Directory

Consulta [Active Directory](../devices/activedirectory.md). El
controlador de dominio que consulta DumbMonit está cubierto por
*Device unreachable*; los demás controladores de dominio del dominio
por *Domain controller unreachable* (etiqueta `dc`).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Domain controller unreachable | Un controlador de dominio del dominio ya no responde en su puerto LDAP (`dumbmonit_ad_dc_reachable == bool 0`). Un nombre que no resuelve desde DumbMonit no produce ninguna serie y nunca se activa. | > 0 | 5 min | Warning (`critical`) | 6 h |
| Active Directory bind failed | El controlador de dominio responde pero rechaza la cuenta de servicio: contraseña caducada, cuenta desactivada o bloqueada (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Privileged group membership changed | Un miembro se unió o salió de Domain Admins, Enterprise Admins, Schema Admins, Administrators o un grupo de operadores, anidamiento incluido (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, etiqueta `group`). Se resuelve por sí misma una hora después. | > 0 | 1 min | Advisory (`warning`) | 24 h |
| Active Directory replication failing | El controlador de dominio consultado no puede replicar desde un socio (`dumbmonit_ad_replication_consecutive_failures`). Solo cuando la cuenta puede leer el estado de replicación. | > 0 | 30 min | Advisory (`warning`) | 6 h |
| krbtgt password older than 180 days | La contraseña krbtgt se cambió por última vez hace más de 180 días (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 days | 10 min | Info (`info`) | ninguno |

### pfSense

Consulta [pfSense](../devices/pfsense.md). Los gateways se etiquetan
`gateway`, las interfaces `interface` y `descr`, los servicios
`service`.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| pfSense gateway down | Un gateway pfSense está caído: su dirección de monitor ya no responde, y el tráfico que lleva se detiene o se conmuta (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 min | Warning (`critical`) | 6 h |
| pfSense gateway losing packets | Un gateway pfSense pierde más del 10 % de las sondas hacia su dirección de monitor: el enlace está degradado (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 min | Advisory (`warning`) | 6 h |
| pfSense gateway slow | La ida y vuelta hacia la dirección de monitor de un gateway pfSense se ha mantenido por encima de 500 ms durante quince minutos (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 min | Advisory (`warning`) | 6 h |
| pfSense interface without link | Una interfaz pfSense activada no tiene enlace: cable, puerto de switch o tarjeta de red (`dumbmonit_pfsense_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| pfSense service stopped | Un servicio pfSense que está activado no está en ejecución: resolutor DNS, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| pfSense disk nearly full | El disco de pfSense está más del 90 % lleno: los logs, informes y actualizaciones de paquete dejan de escribirse (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |

### Unraid

Consulta [Unraid](../devices/unraid.md). Los discos se etiquetan
`disk` y `role` (`parity`, `data`, `cache`), los contenedores
`container`.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Unraid array stopped | El array de Unraid no está iniciado: las comparticiones, contenedores y VMs que viven en él no están disponibles (`dumbmonit_unraid_array_started`). | < 1 | 10 min | Warning (`critical`) | 6 h |
| Unraid disk disabled or missing | Un disco de Unraid está desactivado, ausente o inválido: el array funciona degradado, y un fallo más pierde datos (`dumbmonit_unraid_disk_ok`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Unraid disk read errors | Un disco de Unraid ha reportado errores de lectura desde que arrancó el array: puede desactivarse en el próximo (`dumbmonit_unraid_disk_errors`). | > 0 | 5 min | Advisory (`warning`) | 1 d |
| Unraid disk too hot | Un disco de array o paridad de Unraid se ha mantenido por encima de 55 °C durante quince minutos (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| Unraid parity check found errors | La última comprobación de paridad de Unraid falló o encontró errores de sincronización: la paridad ya no coincide con los datos (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 min | Advisory (`warning`) | 1 d |
| Unraid parity check overdue | La última comprobación de paridad de Unraid tiene más de 40 días: se perdió una comprobación mensual (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 d | 1 h | Advisory (`warning`) | 1 d |
| Unraid cache nearly full | Un pool de caché de Unraid está más del 90 % lleno: las escrituras caen al array lento, o fallan (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |
| Unraid container stopped | Un contenedor Docker configurado para iniciarse con el array no está en ejecución (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 min | Advisory (`warning`) | 6 h |

### Veeam Backup & Replication {#veeam-backup--replication}

Consulta [Veeam Backup & Replication](../devices/veeam.md). Los jobs
se etiquetan `job` y `type`, los repositorios `repository`.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Veeam job failed | La última ejecución de un job Veeam activado falló: lo que protege no tiene ningún punto de restauración nuevo (`dumbmonit_veeam_job_failed`). | > 0 | 5 min | Warning (`critical`) | 1 d |
| Veeam job ended with a warning | La última ejecución de un job Veeam activado terminó con un aviso: algunos objetos pueden no estar protegidos (`dumbmonit_veeam_job_warning`). | > 0 | 5 min | Advisory (`warning`) | 1 d |
| Veeam repository nearly full | Un repositorio de copia Veeam está más del 90 % lleno: los próximos jobs fallarán por falta de espacio (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |
| Veeam license expiring | La licencia Veeam caduca en menos de 30 días: los jobs se detienen al final del período de gracia (`dumbmonit_veeam_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 1 d |

### Tailscale

Consulta [Tailscale](../devices/tailscale.md). Los dispositivos se
etiquetan `device`, la primera etiqueta de su nombre MagicDNS.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Tailscale device offline | Un dispositivo Tailscale que debe permanecer en línea (un dispositivo etiquetado por defecto) lleva desconectado más de 15 minutos (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 min | 5 min | Advisory (`warning`) | 6 h |
| Tailscale node key expiring | La clave de nodo de un dispositivo Tailscale caduca en menos de 14 días: abandonará la tailnet hasta que alguien vuelva a iniciar sesión (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 d | 1 h | Advisory (`warning`) | 1 d |
| Tailscale device awaiting approval | Un dispositivo se unió a la tailnet y espera aprobación: apruébalo si es esperado, elimínalo en caso contrario (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 min | Advisory (`warning`) | 1 d |
| Tailscale client update | Un dispositivo Tailscale que debe permanecer en línea ejecuta un cliente con una actualización disponible (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Info (`info`) | 7 d |

### FortiGate

Consulta [FortiGate](../devices/fortigate.md). Los túneles se
etiquetan `tunnel`, las interfaces `interface` y `alias`, las
licencias `license`.

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| FortiGate IPsec tunnel down | Ninguna fase 2 de un túnel IPsec sitio a sitio de FortiGate está activa: el sitio detrás de él está cortado (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| FortiGate interface without link | Una interfaz FortiGate activada con dirección no tiene enlace: cable, transceptor o el dispositivo delante (`dumbmonit_fortigate_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| FortiGate HA out of sync | Los miembros de un clúster HA de FortiGate ya no tienen el mismo checksum de configuración: un failover ejecutaría otra configuración (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 min | Advisory (`warning`) | 6 h |
| FortiGate HA member lost | Un clúster HA de FortiGate tiene menos miembros que en las últimas 24 horas: ya no hay respaldo (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| FortiGate memory near conserve mode | La memoria de FortiGate se ha mantenido por encima del 85 % durante diez minutos: al 88 % (por defecto) entra en modo conserve y deja de inspeccionar sesiones nuevas (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 min | Advisory (`warning`) | 6 h |
| FortiGate CPU high | La CPU de FortiGate se ha mantenido por encima del 90 % durante quince minutos (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| FortiGate licence expiring | Una licencia FortiGuard o FortiCare del FortiGate caduca en menos de 30 días, o ya caducó (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 1 d |
| FortiGate firmware update | FortiGuard ofrece una versión de parche más reciente de la rama de FortiOS que ejecuta el FortiGate (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Sophos Firewall

Consulta [Sophos Firewall](../devices/sophos.md).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Sophos interface without link | Una interfaz Sophos Firewall encendida y ligada a una zona no tiene enlace (`dumbmonit_sophos_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### KVM (libvirt)

Reportado por el agente Linux en un host KVM; consulta [Agente](../devices/agent.md#kvm-virtual-machines-through-libvirt).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| KVM domain crashed | libvirt reporta un dominio KVM como caído: el hipervisor tuvo que detenerlo (`dumbmonit_agent_libvirt_domain_crashed`). | > 0 | 1 min | Warning (`critical`) | 1 h |
| libvirt pool almost full | La asignación de un pool de almacenamiento libvirt alcanza el 90 % de su capacidad (`dumbmonit_agent_libvirt_pool_used_percent`). | ≥ 90 % (se limpia por debajo del 88 %) | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |

### XCP-ng / XenServer

Consulta [XCP-ng / XenServer](../devices/xcpng.md).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| XCP-ng host offline | Un host del pool XCP-ng ya no responde al coordinador (`dumbmonit_xcpng_host_live`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| XCP-ng SR almost full | Un repositorio de almacenamiento (SR) de XCP-ng está lleno a más del 90 % (`dumbmonit_xcpng_sr_used_percent`). | ≥ 90 % (se limpia por debajo del 88 %) | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |
| XCP-ng HA-protected VM down | Una VM protegida por la alta disponibilidad del pool está detenida (`dumbmonit_xcpng_vm_ha_protected_down`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| XCP-ng pool HA overcommitted | El pool ya no tiene un plan para sobrevivir a los fallos de host que está configurado para tolerar (`dumbmonit_xcpng_pool_ha_overcommitted`). | > 0 | 5 min | Advisory (`warning`), escala tras 1 h | 6 h |

### Nutanix

Consulta [Nutanix AHV](../devices/nutanix.md).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Nutanix host degraded | Prism reporta un host Nutanix como degradado (`dumbmonit_nutanix_host_degraded`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Nutanix storage container almost full | Un contenedor de almacenamiento Nutanix está lleno a más del 90 % (`dumbmonit_nutanix_storage_container_used_percent`). | ≥ 90 % (se limpia por debajo del 88 %) | 15 min | Advisory (`warning`), escala tras 24 h | 6 h |
| Nutanix critical alerts | Prism Central reporta una o más alertas críticas sin resolver (`dumbmonit_nutanix_alerts_critical`). | > 0 | 5 min | Warning (`critical`) | 1 h |

### Hyper-V

Reportado por el agente Windows en un host Hyper-V; consulta
[Agente](../devices/agent.md#hyper-v-hosts).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Hyper-V VM health critical | Hyper-V reporta una máquina virtual en salud crítica: un recurso que necesita, con más frecuencia espacio en disco, está agotado (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Hyper-V host CPU high | Los procesadores lógicos del host Hyper-V han estado ocupados más del 90 % durante quince minutos, invitados incluidos (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| Hyper-V virtual disk errors | Un disco virtual Hyper-V reportó errores de E/S en la última hora (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Puntuación de seguridad {#security-score}

Calculada por el servidor cada 15 minutos para cada dispositivo cuyo
tipo tiene comprobaciones de seguridad, y guardada como
`dumbmonit_security_score` (0–100), solo mientras la puntuación de seguridad
esté activada en Ajustes (desactivada por defecto). Consulta [Puntuación de
seguridad](../using/security-score.md).

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| Security score dropped | La puntuación cayó 10 puntos o más frente a su mejor valor de las últimas 24 horas (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): una comprobación empezó a fallar. | ≥ 10 pts | 30 min | Advisory (`warning`) | 24 h |
| Security score low | La puntuación está por debajo de 40, nota F (`dumbmonit_security_score`). Sube o baja el umbral para ajustarlo a tu propio listón. | < 40 | 1 h | Info (`info`) | 7 d |

### El propio DumbMonit {#dumbmonit-itself}

| Regla | Qué | Umbral por defecto | Espera | Severidad | Recordatorio |
|---|---|---|---|---|---|
| DumbMonit backup did not run | La copia de seguridad local programada de la base de datos de DumbMonit no se ha ejecutado en más de dos días (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |

Cada regla integrada se aplica a todos los dispositivos y a cada canal
activado.

## Editar una regla en la UI {#editing-a-rule-in-the-ui}

En la página **Alerts**, la sección *Rules* lista cada regla con su
severidad, una marca *Built-in* y, cuando corresponde, cuántas
alertas está activando ahora. Cada regla tiene:

- un interruptor para **activarla o desactivarla**;
- **Edit**, que despliega un editor en línea para nombre,
  descripción, operador, umbral, espera (`for`), severidad,
  recordatorio (`repeat every`) y escalada (`escalate after`). La
  consulta de una regla integrada se muestra pero no se puede editar;
  la consulta de las tuyas sí. Las reglas de referencia muestran sus
  parámetros de detección de solo lectura;
- **Delete**, solo para tus propias reglas.

## Anulaciones por dispositivo {#per-device-overrides}

Una regla se puede ajustar para un dispositivo sin tocar la regla en
sí: un umbral distinto, un umbral de limpieza distinto, o
desactivada solo para ese dispositivo. El editor de cada regla tiene
un panel **Per-device overrides** para añadir o eliminar uno
directamente.

La acción de un clic **Ignore** en una alerta (en la lista *Needs
you* o en la página de dispositivo de la alerta) es el mismo
mecanismo, escrito con `enabled: false`: el dispositivo deja de
llevar esa regla, de inmediato, de forma reversible. La página del
dispositivo lista lo que ignora, con **Stop ignoring**; la propia
regla muestra *Ignored on N devices* en Alerts → Rules, donde el
mismo panel de anulaciones por dispositivo la elimina. Consulta
[Anulaciones por dispositivo](../reference/api.md#per-device-overrides)
para la API.

## Crear una regla de umbral {#creating-a-threshold-rule}

Haz clic en **New rule**. El formulario necesita:

| Campo | Significado |
|---|---|
| Name | Mostrado en alertas y notificaciones. |
| Query | Una expresión MetricsQL, evaluada por dispositivo. Ejemplo: `dumbmonit_cpu_usage_percent`. |
| Operator | `>`, `>=`, `<` o `<=`. |
| Threshold | El número con el que se compara el resultado de la consulta. |
| Severity | Info, Advisory o Warning. |
| Hold for | Segundos que debe durar la condición antes de activarse. Una espera distinta de cero absorbe una muestra mala aislada. |

Cada serie devuelta por la consulta se convierte en una alerta
potencial, asociada al dispositivo nombrado por su etiqueta `target`.
Agrupa con `avg by (target, host) (…)` cuando quieras una alerta por
dispositivo en lugar de una por núcleo o por interfaz. La identidad de
la alerta ignora las etiquetas `host` y `tag_*` siempre que `target`
esté presente, así que renombrar un dispositivo conserva sus alertas;
una serie cuyo `target` no coincide con un dispositivo existente y
activado se ignora.

## El lenguaje de consulta {#the-query-language}

Las consultas son [MetricsQL](https://docs.victoriametrics.com/metricsql/),
el superconjunto PromQL de VictoriaMetrics. Las métricas son las
series `dumbmonit_*` descritas en la [referencia de
métricas](../reference/metrics.md), y cada serie lleva una etiqueta
`target` (el id del dispositivo) y una etiqueta `host` (su nombre).

Algunos patrones usados por las reglas integradas:

```
# Antigüedad de la última muestra, serie más reciente por dispositivo: lo que significa "unreachable"
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))

# Un valor por dispositivo, sea cual sea la fuente
avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)

# Un porcentaje calculado a partir de dos gauges
100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total

# Una condición booleana: la serie solo se devuelve cuando es verdadera
dumbmonit_ups_output_source == 5

# Extrapolación hecha por VictoriaMetrics
predict_linear(dumbmonit_pbs_datastore_used_percent[6h], 345600) and deriv(dumbmonit_pbs_datastore_used_percent[6h]) > 0

# Disponibilidad de un servicio en treinta días
avg_over_time(dumbmonit_probe_success[30d])
```

!!! tip "Prueba una consulta primero"
    `GET /api/metrics/query?query=…` devuelve lo que evaluaría la
    regla (consulta la [referencia de la API](../reference/api.md#metrics)).
    En desarrollo, la capa publica la VictoriaMetrics integrada en el
    `:8428` del host, con su propia UI de consultas.

## Tipos de regla en la API {#rule-kinds-in-the-api}

`kind` es `threshold` (compara con un umbral), `predict` (la misma
comparación, sobre una consulta `predict_linear`; mostrado como un
pronóstico) o `anomaly` (referencia estacional; `params` contiene `k`,
`alpha`, `mad_floor_abs`, `mad_floor_rel`, `min_samples`). La UI crea
reglas `threshold`; los otros dos tipos se pueden crear a través de la
API.
