# Regras {#rules}

As regras são inseridas no primeiro arranque, depois pertencem a si:
o DumbMonit nunca reescreve uma regra que alterou, só recria as que
faltam. As regras incorporadas podem ser editadas e desativadas, não
eliminadas.

## Regras incorporadas {#built-in-rules}

As severidades são mostradas com a palavra da interface; o valor da
API está entre parênteses. Uma regra cuja métrica nenhum dispositivo
produz não tem série e nunca dispara, por isso toda regra abaixo é
distribuída ativada em toda instância.

### Todo dispositivo {#every-device}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Device unreachable | Nenhuma medição recebida há mais de três minutos. `bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))` | > 180 s | 1 min | Warning (`critical`) | 6 h |
| High CPU | Carga de CPU sustida acima de 90%, em média por dispositivo entre fontes SNMP, Proxmox e agente; limpa-se abaixo de 85%. | > 90 % | 10 min | Advisory (`warning`), agrava depois de 1 h | 6 h |
| Unusual CPU | Carga de CPU notavelmente diferente do habitual a esta hora e dia da semana (linha de base sazonal). | pontuação > 3,5 | 15 min | Info (`info`) | 6 h |
| Disk almost full | Sistema de ficheiros 90% cheio ou mais, por ponto de montagem (SNMP, armazenamentos Proxmox e sistemas de ficheiros raiz); limpa-se abaixo de 88%. | ≥ 90 % | 15 min | Advisory (`warning`), agrava depois de 24 h | 6 h |
| Filesystem almost full | A este ritmo, o sistema de ficheiros ficará cheio dentro de quatro dias (`predict_linear` sobre 6 h; só sistemas de ficheiros a subir). | ≥ 100 % | 30 min | Advisory (`warning`) | 6 h |

### UPS {#ups}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| UPS on battery | O UPS está a alimentar a carga a partir da bateria (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low | A bateria do UPS está baixa ou esgotada (`dumbmonit_ups_battery_status`, 3 = baixa, 4 = esgotada). | ≥ 3 | 1 min | Warning (`critical`) | 30 min |

### Portas de rede (SNMP) {#network-ports-snmp}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Port accumulating errors | Uma porta de rede registou mais de cinquenta erros na última hora: geralmente um cabo a falhar, uma fibra suja ou uma incompatibilidade de duplex (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; nada dispara durante a primeira hora de um novo dispositivo, por isso erros contados antes de ser adicionado não são reportados como novos). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| Port flapping | Uma porta de rede caiu e voltou mais de duas vezes em trinta minutos (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Uma porta desativada de propósito não tem série e nunca dispara. | > 4 | 5 min | Advisory (`warning`) | 1 h |

### Hardware de servidor (Redfish) {#server-hardware-redfish}

As séries de saúde são 0 (OK), 1 (Warning) ou 2 (Critical). Uma ranhura
vazia (`Absent`) ou desativada não tem série, por isso um servidor
entregue com uma fonte de alimentação de duas nunca dispara.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Server fan failed | O controlador de gestão reporta uma ventoinha como falhada (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Server temperature above critical | Um sensor de temperatura está acima do limiar crítico que o próprio controlador declara (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Power supply redundancy lost | As fontes de alimentação já não são redundantes: mais uma falha e o servidor cai (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 min | Advisory (`warning`), agrava depois de 1 h | 6 h |
| Power supply failed | O controlador reporta uma fonte de alimentação como falhada (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Drive failure predicted | Um disco reporta uma falha prevista: substitua-o enquanto ainda funciona (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Server health critical | O controlador reporta a própria saúde do sistema como Critical (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |

### Monitores de disponibilidade e heartbeats {#uptime-monitors-and-heartbeats}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Service down | O serviço não respondeu corretamente durante três minutos (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 min | Warning (`critical`) | 30 min |
| Heartbeat missed | A tarefa não chamou dentro do seu intervalo esperado mais o período de margem, ou reportou uma falha ela própria (`dumbmonit_probe_success{probe="push"} == bool 0`; o prazo é definido por dispositivo, ver [Heartbeat](../devices/push.md)). | > 0 | 2 min | Advisory (`warning`) | 6 h |
| Service flapping | O serviço mudou de estado mais de seis vezes em trinta minutos (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 min | Advisory (`warning`) | 1 h |
| Slow service | O serviço demora mais de três segundos a responder; as verificações falhadas ficam de fora (`dumbmonit_probe_duration_seconds and dumbmonit_probe_success == 1`). | > 3 s | 10 min | Advisory (`warning`) | 6 h |
| Certificate expiring soon | O certificado expira em menos de catorze dias. | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Certificate expired | O certificado expirou. | < 0 d | 5 min | Warning (`critical`) | 24 h |

### Contentores Docker {#docker-containers}

Reportados pelo agente; ver [Agente](../devices/agent.md). O alerta
nomeia o contentor.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Container stopped | O contentor está parado há dois minutos (`dumbmonit_container_up == bool 0`). Um contentor que o Docker não reiniciaria fica de fora (`dumbmonit_container_stop_expected = 1`): um `docker run` pontual que terminou, um contentor nunca iniciado, ou um parado à mão sem política de reinício. Uma falha (qualquer outro código de saída), um encerramento por falta de memória, ou um contentor `always` / `unless-stopped` continua a contar. | > 0 | 2 min | Advisory (`warning`) | 1 h |
| Container unhealthy | A verificação de saúde do contentor está a falhar há três minutos (`dumbmonit_container_health == 2`; 0 nenhuma, 1 saudável, 3 a iniciar). Um contentor parado fica de fora: o Docker guarda o seu último veredicto, que já não significa nada. | > 0 | 3 min | Advisory (`warning`) | 1 h |
| Container restarting | O contentor reiniciou três vezes ou mais em quinze minutos (`increase(dumbmonit_container_restart_count[15m])`): raramente fica parado o tempo suficiente para a regra anterior. | ≥ 3 | 1 min | Advisory (`warning`) | 1 h |
| Container update available | Uma imagem mais recente está disponível no registo para este contentor (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Cópias de segurança Plakar {#plakar-backups}

Reportadas pelo agente, por kloset e por fonte. Nenhuma destas séries
existe enquanto o Plakar não é detetado, por isso nenhuma regra pode
disparar numa máquina sem ele.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Plakar backup too old | Nenhum snapshot Plakar novo para esta fonte há mais de dois dias (`dumbmonit_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Plakar backup failed | O snapshot mais recente tem erros (`dumbmonit_backup_last_status{source!="*"} < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |
| Plakar kloset unreadable | O agente não consegue abrir o kloset: armazenamento inacessível, senha ou plugin em falta; o registo do agente diz qual, uma vez (`dumbmonit_backup_last_status{source="*"} < 1`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Cópias de segurança restic e Borg {#restic-and-borg-backups}

Reportadas pelo agente para cada repositório declarado sob
`restic_repos` ou `borg_repos` em agent.yaml; ver
[Agente](../devices/agent.md#restic-and-borg-backups). Sem um
repositório declarado não há nenhuma série, por isso nenhuma regra
pode disparar.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| restic/Borg backup too old | Nenhum snapshot restic ou Borg novo neste repositório há mais de 48 horas (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Advisory (`warning`) | 24 h |
| restic/Borg repository unreachable | O agente não consegue ler o repositório: inacessível, senha recusada ou binário em falta; o registo do agente diz qual (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Túneis WireGuard {#wireguard-tunnels}

Reportados pelo agente; ver [Agente](../devices/agent.md#wireguard-tunnels).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| WireGuard peer silent | Um par com um keepalive persistente não completou um handshake há mais de 15 minutos: o túnel está em baixo (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Um par que nunca se ligou conta o seu silêncio a partir do momento em que o agente o viu por primeira vez. Pares sem keepalive (um telefone, um portátil) ficam inativos durante dias e são deixados de fora; altere o limiar para mudar N, ou retire a parte `and on …` para monitorizar todo par. | > 15 min | 5 min | Advisory (`warning`) | 6 h |

### Discos do agente, ZFS e sensores {#agent-disks-zfs-and-sensors}

Reportados pelo agente; ver [Agente](../devices/agent.md).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Disk SMART failing | A autoavaliação SMART do próprio disco reporta-o como a falhar (`dumbmonit_agent_disk_smart_ok == bool 0`). Substitua-o e verifique as cópias de segurança. | > 0 | 10 min | Warning (`critical`) | 24 h |
| ZFS pool degraded | Uma pool ZFS já não está saudável: um dispositivo está em falta, com falha ou removido (`dumbmonit_agent_zfs_pool_health`: 0 online, 1 degraded, 2 qualquer outro estado). | > 0 | 5 min | Warning (`critical`) | 6 h |
| ZFS scrub found errors | O último scrub ZFS encontrou erros nesta pool, ou há ficheiros permanentemente perdidos (`dumbmonit_agent_zfs_pool_scrub_errors or dumbmonit_agent_zfs_pool_data_errors`). A pool pode continuar `ONLINE`. | > 0 | 10 min | Warning (`critical`) | 24 h |
| Temperature above critical | Um sensor está acima da temperatura crítica que o seu próprio hardware declara (`dumbmonit_agent_sensor_temperature_celsius >= dumbmonit_agent_sensor_temperature_critical_celsius`); sem valor fixo. | > 0 | 5 min | Warning (`critical`) | 6 h |

### Proxmox VE {#proxmox-ve}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Backup too old | Nenhuma cópia de segurança Proxmox VE bem-sucedida há mais de sete dias (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 d | 1 h | Advisory (`warning`) | 24 h |
| VM or container stopped | O convidado estava em execução nas últimas duas horas e está parado há cinco minutos (`1 - dumbmonit_proxmox_guest_running`, e'd com `max_over_time(…[2h]) == 1`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| VM or container CPU high | Um convidado usou mais de 90% dos seus núcleos atribuídos durante quinze minutos (`dumbmonit_proxmox_guest_cpu_percent`). A notificação nomeia o convidado, `nextcloud (202)`. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| VM or container memory high | Um convidado usou mais de 95% da sua memória durante dez minutos (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 min | Advisory (`warning`) | 6 h |
| VM or container disk almost full | O disco raiz de um convidado está mais de 90% cheio (`dumbmonit_proxmox_guest_disk_used_percent`): contentores sempre, VMs só quando o agente convidado QEMU reporta o uso. | > 90 % | 15 min | Advisory (`warning`) | 24 h |
| Guest locked | Uma VM ou contentor está bloqueado há seis horas (`dumbmonit_proxmox_guest_locked`, etiqueta `lock`): uma cópia de segurança ou migração interrompida deixou o bloqueio e nenhuma operação pode correr no convidado, nem mesmo a próxima cópia de segurança. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| Old snapshot | O snapshot mais antigo de um convidado tem mais de trinta dias (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 d | 1 h | Info (`info`) | 7 d |
| HA resource in error | Um recurso HA do Proxmox está em erro ou fenced (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| HA manager not reporting | O gestor local de recursos HA (LRM) de um nó deixou de escrever o seu estado (`dumbmonit_proxmox_ha_lrm_stale`): os convidados HA desse nó não serão nem reiniciados nem realocados. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Cluster lost quorum | O cluster Proxmox não tem quórum (`1 - dumbmonit_proxmox_cluster_quorate`): os convidados já não podem ser iniciados. | > 0 | 1 min | Warning (`critical`) | 30 min |
| Proxmox node offline | Um nó do cluster é reportado offline (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 min | Warning (`critical`) | 30 min |
| Proxmox service down | Um daemon central do Proxmox não está em execução num nó (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, cada um contado só onde o nó o ativa). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Node network interface down | Uma interface definida para iniciar no arranque não está ativa num nó (`dumbmonit_proxmox_node_interface_offline`). Uma interface ativada a pedido nunca é reportada. | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Proxmox storage almost full | Um armazenamento Proxmox está mais de 85% cheio (`dumbmonit_proxmox_storage_used_percent`); «Disk almost full» assume aos 90%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Thin pool almost full | Um pool thin LVM está mais de 90% cheio: todo convidado armazenado nele fica só de leitura quando se enche (`dumbmonit_proxmox_node_thinpool_used_percent`); limpa-se abaixo de 88%. | > 90 % | 15 min | Warning (`critical`) | 6 h |
| Thin pool metadata almost full | O volume de metadados de um pool thin LVM está mais de 80% cheio (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). É muito mais pequeno do que o volume de dados e frequentemente satura primeiro, com a mesma consequência. | > 80 % | 15 min | Warning (`critical`) | 6 h |
| Proxmox disk wearing out | Um SSD ou NVMe de um nó usou mais de 90% da sua vida nominal (`dumbmonit_proxmox_node_disk_wearout_percent`). A notificação lê `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| Proxmox disk SMART failure | Um disco de um nó reporta saúde SMART `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| Proxmox ZFS pool degraded | Um pool ZFS de um nó não está `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). A notificação lê `pve2 · tank`. | > 0 | 5 min | Warning (`critical`) | 6 h |
| Backup job failed | A última tarefa vzdump num nó, ou uma tarefa de cópia de segurança agendada, falhou (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 min | Warning (`critical`) | 24 h |
| Backup job excludes a disk | Uma tarefa de cópia de segurança agendada ignora um disco de um dos convidados que cobre (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): a tarefa tem sucesso todas as noites com esses dados em falta. Unidades de CD-ROM e imagens cloudinit são excluídas por desenho e nunca contadas. | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Replication failed | Uma tarefa de replicação reporta um erro ou uma contagem de falhas não-zero (`dumbmonit_proxmox_replication_job_error`): a cópia em espera está obsoleta. | > 0 | 10 min | Warning (`critical`) | 6 h |
| Ceph health error | O Ceph reporta `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 min | Warning (`critical`) | 30 min |
| Ceph health warning | O Ceph reporta `HEALTH_WARN` há mais de quinze minutos (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Ceph OSD down | Um OSD Ceph está em baixo há cinco minutos (`1 - dumbmonit_proxmox_ceph_osd_up`). A notificação nomeia o OSD e o seu anfitrião. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Ceph OSD nearly full | Um único OSD Ceph está mais de 85% cheio (`dumbmonit_proxmox_ceph_osd_used_percent`): as escritas param quando atinge o seu limite, seja o que for que a média do cluster diga; limpa-se abaixo de 82%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph pool almost full | Uma pool Ceph está mais de 85% cheia (`dumbmonit_proxmox_ceph_pool_used_percent`); limpa-se abaixo de 82%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph noout flag left on | A flag `noout` do Ceph está definida há duas horas (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): o rebalanceamento está desativado, e o cluster lê-se saudável enquanto a sua redundância diminui. | > 0 | 2 h | Info (`info`) | 24 h |
| Proxmox updates pending | Mais de vinte atualizações de pacote estão pendentes num nó (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |
| Proxmox security updates pending | Pelo menos uma atualização pendente vem de um arquivo de segurança (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Proxmox packages changed | A versão instalada de um pacote Proxmox mudou entre duas verificações — alguém executou `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, publicado durante uma hora com o detalhe na sua etiqueta `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Resolve-se por si só uma hora depois. | > 0 | 1 min | Info (`info`) | 24 h |
| Node certificate expiring | Um certificado de nó Proxmox expira em menos de catorze dias (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |

### Proxmox Backup Server {#proxmox-backup-server}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| PBS datastore almost full | Datastore PBS mais de 90% cheio. | > 90 % | 15 min | Advisory (`warning`), agrava depois de 24 h | 6 h |
| PBS datastore filling up | Ao ritmo atual, o PBS estima o datastore cheio dentro de sete dias. | < 7 d | 1 h | Advisory (`warning`) | 24 h |
| PBS datastore not mounted | Um datastore removível não está montado (`dumbmonit_pbs_datastore_removable_unmounted`): nada pode ser escrito nele. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| PBS backup too old | Nenhum snapshot novo para este grupo de cópia de segurança (um por máquina) há mais de dois dias (`dumbmonit_pbs_backup_last_age_seconds`). Edite o limiar para máquinas com cópia de segurança semanal; a notificação nomeia o grupo e datastore. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| PBS backup verification failed | A verificação do snapshot mais recente para esta máquina falhou (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 min | Warning (`critical`) | 24 h |
| PBS task failed | Pelo menos uma tarefa PBS falhou na janela de revisão (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| PBS garbage collection too old | Nenhum garbage collection bem-sucedido neste datastore há mais de oito dias (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| PBS garbage collection failed | O último garbage collection num datastore falhou (`dumbmonit_pbs_gc_last_run_ok < 1`; de `/gc` no PBS 3.3+, da última tarefa de GC antes). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS corrupt chunks found | O garbage collection encontrou chunks ilegíveis num datastore (`dumbmonit_pbs_gc_bad_chunks`): algumas cópias de segurança já não podem ser restauradas. | > 0 | 10 min | Warning (`critical`) | 24 h |
| PBS prune job failed | A última execução de uma tarefa de prune do PBS falhou: nada é removido e o datastore continua a enchê-se (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). A notificação nomeia a tarefa e datastore. | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS verification job failed | A última execução de uma tarefa de verificação do PBS falhou (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS sync job failed | A última execução de uma tarefa de sync do PBS falhou (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS tape backup failed | A última execução de cópia de segurança em fita falhou (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): a cópia offline não está a ser feita. | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS job never ran | Uma tarefa está ativada e agendada mas nunca correu (`dumbmonit_pbs_job_never_run`). | > 0 | 2 d | Advisory (`warning`) | 7 d |
| PBS service down | Um serviço de que o Proxmox Backup Server precisa não está em execução (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); nenhuma cópia de segurança pode ser feita enquanto estiver em baixo. | < 1 | 5 min | Warning (`critical`) | 6 h |
| PBS disk SMART failure | Um disco do servidor de cópias de segurança reporta saúde SMART `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). A notificação lê `/dev/sdb`. | > 0 | 5 min | Warning (`critical`) | 24 h |
| PBS SSD worn out | Um SSD do servidor de cópias de segurança usou mais de 90% da sua resistência nominal (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| PBS ZFS pool degraded | Um pool ZFS do servidor de cópias de segurança não está `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| PBS certificate expiring | O certificado servido pelo servidor de cópias de segurança expira em menos de três semanas (`dumbmonit_pbs_node_certificate_expires_seconds`). Precisa da opção `certificates`. | < 21 d | 1 h | Advisory (`warning`) | 48 h |
| PBS restart pending after upgrade | O pacote foi atualizado mas o daemon em execução ainda é o antigo (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 d |
| PBS updates pending | Mais de vinte atualizações de pacote estão pendentes no servidor de cópias de segurança (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Datacenter Manager {#proxmox-datacenter-manager}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Federated instance unreachable | A consola do datacenter não consegue alcançar um dos clusters Proxmox VE ou servidores de cópias de segurança que federa (`dumbmonit_pdm_remote_reachable < 1`). A notificação nomeia a instância e o seu produto. | < 1 | 10 min | Warning (`critical`) | 6 h |
| Federated instance behind | Uma instância corre uma versão mais antiga do que outra instância do mesmo produto no mesmo património (`dumbmonit_pdm_remote_version_behind`). Comparado entre instâncias, nunca contra o que o Proxmox publica. | > 0 | 1 h | Info (`info`) | 7 d |
| Task failed on a federated instance | Pelo menos uma tarefa falhou numa instância federada na janela de revisão (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Datacenter console disk almost full | O sistema de ficheiros raiz da consola do datacenter está mais de 90% cheio (`dumbmonit_pdm_node_rootfs_percent`): cheia, a consola deixa de registar o que coleta. | > 90 % | 15 min | Advisory (`warning`), agrava depois de 24 h | 6 h |
| Datacenter console certificate expiring | Um certificado da consola do datacenter expira em menos de catorze dias (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Datacenter console updates pending | Mais de vinte atualizações de pacote estão pendentes na consola do datacenter (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Mail Gateway {#proxmox-mail-gateway}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Mail queue growing | A fila de email deferred cresceu mais de cem mensagens em duas horas (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): o salto seguinte está provavelmente a recusar email. Crescimento em vez de tamanho, porque um gateway ocupado mantém sempre algumas dezenas de deferred. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| Mail stuck in the queue | Uma mensagem está à espera numa fila há mais de quatro horas (`dumbmonit_pmg_queue_oldest_age_seconds`). A idade é um limite inferior — o qshape reporta intervalos — por isso a regra dispara tarde em vez de cedo. A notificação nomeia a fila. | > 4 h | 15 min | Warning (`critical`) | 6 h |
| Mail gateway service stopped | Um serviço de que o gateway precisa para aceitar e filtrar email está parado (`dumbmonit_pmg_service_running` para postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Uma unidade não instalada não tem série e não pode disparar. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Virus signatures out of date | As assinaturas de vírus diárias do ClamAV têm mais de dois dias (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): o freshclam deixou de as atualizar e o gateway continua a filtrar com a lista da semana passada. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Spam rules out of date | As regras do SpamAssassin têm mais de oito dias (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Um canal nunca atualizado não tem data nem série. | > 8 d | 1 h | Advisory (`warning`) | 7 d |
| Quarantine filling up | A quarentena de spam recebeu mais de mil mensagens em seis horas (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): uma campanha, ou uma regra que acabou de colocar um domínio legítimo no lixo. | > 1000 | 30 min | Advisory (`warning`) | 12 h |
| Mail gateway cluster degraded | Um gateway do cluster já não está sincronizado com os outros (`dumbmonit_pmg_cluster_node_insync`): filtra com uma base de dados de regras desatualizada. Um gateway autónomo não tem série. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Mail gateway certificate expiring | Um certificado do gateway de email expira em menos de catorze dias (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Mail gateway updates pending | Mais de vinte atualizações de pacote estão pendentes no gateway de email (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### MDaemon e SecurityGateway {#mdaemon-and-securitygateway}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| MDaemon mail service down | Um serviço de email monitorizado do servidor MDaemon não responde, ou sauda para recusar serviço (`dumbmonit_mdaemon_service_up`). Só os serviços listados no dispositivo têm série. A notificação nomeia o serviço e a sua porta. | < 1 | 5 min | Warning (`critical`) | 6 h |
| MDaemon XML API not answering | A API XML deixou de responder enquanto os serviços de email ainda respondem (`dumbmonit_mdaemon_api_up`): o Remote Administration está provavelmente parado. Só um dispositivo com uma conta tem a série. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| MDaemon mail queue growing | A fila inbound, local ou remote cresceu mais de cem mensagens em duas horas (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): o MDaemon deixou de entregar dela. Lido pelo [agente](../devices/agent.md#windows-performance-counters) no servidor MDaemon, por isso o alerta está no dispositivo do agente. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| MDaemon Bad queue not empty | Mensagens estão à espera na fila Bad (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): o MDaemon não as pôde nem entregar nem devolver, e ficam lá até um administrador as libertar ou eliminar. | > 0 | 30 min | Advisory (`warning`) | 24 h |
| MDaemon Retry queue high | Mais de cinquenta mensagens esperam na fila Retry (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): servidores remotos continuam a recusar email temporariamente. | > 50 | 1 h | Advisory (`warning`) | 6 h |
| SecurityGateway service down | Um serviço monitorizado do gateway não responde, ou sauda para recusar serviço (`dumbmonit_securitygateway_service_up`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| SecurityGateway API not answering | A API REST deixou de responder enquanto os serviços ainda respondem (`dumbmonit_securitygateway_api_up`). Só um dispositivo com uma chave de API tem a série. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| SecurityGateway delivery queue growing | A fila de entrega cresceu mais de cem mensagens em duas horas (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): o servidor de email por detrás do gateway está provavelmente a recusar email. Os nomes de contador vêm da API como estão; se a sua versão nomeia a fila de forma diferente, ajuste o correspondente. | > 100 | 15 min | Advisory (`warning`) | 6 h |

### OPNsense {#opnsense}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Internet gateway down | O dpinger já não obtém resposta de um gateway (`dumbmonit_opnsense_gateway_up`). Numa firewall multi-WAN o tráfego já mudou para outra ligação, silenciosamente. `loss` e `delay` são avisos, não falhas, e não disparam esta regra. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Gateway losing packets | Um gateway está a perder mais de um quinto dos pacotes enviados para ele (`dumbmonit_opnsense_gateway_loss_percent`): a ligação responde, mal. Limpa-se abaixo de 10%. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| Gateway slow | O tempo de ida e volta de um gateway manteve-se acima de 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Trezentos milissegundos deixa uma ligação de reserva móvel em paz. | > 300 ms | 15 min | Info (`info`) | 12 h |
| Firewall state table filling up | O pf está a acompanhar mais de 80% das ligações que lhe são permitidas (`dumbmonit_opnsense_pf_states_used_percent`). Além do limite a firewall descarta novas ligações sem nenhum outro sintoma. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| Firewall network buffers exhausted | Mais de 90% dos buffers de rede do FreeBSD estão em uso (`dumbmonit_opnsense_mbuf_used_percent`). Uma firewall que esgota para de encaminhar com a CPU ainda inativa. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| VPN tunnel down | Um túnel VPN configurado não tem nenhuma sessão há dez minutos (`dumbmonit_opnsense_vpn_tunnel_up`). Um plugin não instalado, ou uma ligação desativada, não tem série e não pode disparar. | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Firewall resolver stopped | O Unbound deixou de responder (`dumbmonit_opnsense_unbound_running`). A firewall continua a encaminhar; as máquinas por detrás dela já não resolvem. Desligue esta regra se a sua firewall resolve com outra coisa. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Firewall core service stopped | O dpinger, que vigia os gateways, ou o configd, que executa tudo o resto, está parado (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Só esses dois, porque um serviço desligado de propósito não é uma falha. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Firewall too hot | Um sensor lê acima de 85 °C (`dumbmonit_opnsense_temperature_celsius`): a caixa sem ventoinha no armário está a ponto de começar a limitar-se. Limpa-se a 78 °C. | > 85 °C | 10 min | Advisory (`warning`) | 6 h |
| Firewall left in CARP maintenance mode | O CARP foi deixado em modo de manutenção persistente (`dumbmonit_opnsense_carp_maintenance_mode`): esta firewall entregou os seus endereços virtuais ao seu parceiro e não os recuperará por si só. | > 0 | 1 h | Info (`info`) | 24 h |
| Firewall reboot pending | Uma atualização está instalada mas precisa de um reinício para ter efeito (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 d |
| Firewall updates pending | A firewall tem pacotes à espera de atualização (`dumbmonit_opnsense_firmware_updates_pending`). É a máquina mais exposta na rede. | > 0 | 1 h | Info (`info`) | 7 d |

### TrueNAS {#truenas}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| NAS pool degraded | O ZFS já não considera uma pool saudável (`dumbmonit_truenas_pool_healthy`). Uma pool degradada continua a servir os seus dados, que é exatamente por isso que ninguém repara até o próximo disco falhar. A página do dispositivo nomeia o disco. | < 1 | 5 min | Warning (`critical`) | 6 h |
| NAS disk errors | Discos de uma pool reportaram erros de leitura, escrita ou checksum desde o último `zpool clear` (`dumbmonit_truenas_pool_device_errors`, uma série por tipo). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| NAS pool almost full | Uma pool está mais de 85% cheia (`dumbmonit_truenas_pool_used_percent`). O ZFS abranda bem antes de ficar cheio. Limpa-se aos 80%. | > 85 % | 30 min | Advisory (`warning`), agrava depois de 7 d | 24 h |
| NAS scrub found errors | O último scrub de uma pool encontrou dados danificados (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS scrub overdue | Uma pool não completou um scrub há mais de 45 dias (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Ausente depois de um resilver, que apaga o registo do último scrub. | > 45 d | 1 h | Info (`info`) | 7 d |
| NAS disk failed its SMART test | Um autoteste SMART no registo de um disco falhou (`dumbmonit_truenas_disk_smart_failed`). Um disco nunca testado não tem série. | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS disk too hot | Um disco está acima de 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, lido da cache do TrueNAS). Limpa-se aos 50 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| NAS dataset near its quota | Um dataset usou mais de 90% da sua quota (`dumbmonit_truenas_dataset_quota_used_percent`): a 100% as suas escritas falham enquanto a pool ainda tem espaço. Sem quota, sem série. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| NAS replication failed | Uma tarefa de replicação ativada está em erro (`dumbmonit_truenas_replication_error`): a cópia do outro lado está a envelhecer. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshot task failed | Uma tarefa de snapshot periódico ativada está em erro (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshots stale | Uma tarefa de snapshot periódico ativada não corre há mais de oito dias (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): uma tarefa que deixa de correr não falha, fica silenciosa. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| TrueNAS alert raised | O próprio TrueNAS levantou um alerta de nível ERROR ou acima (`dumbmonit_truenas_alerts`). A sua própria frase está na página do dispositivo. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS service stopped | Um serviço definido para iniciar com o NAS está parado (`dumbmonit_truenas_service_running`). Um serviço desligado de propósito não tem série. | < 1 | 10 min | Advisory (`warning`) | 6 h |

### Synology DSM {#synology-dsm}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Synology disk SMART warning | O DSM já não reporta o estado SMART deste disco como `normal` (`dumbmonit_synology_disk_smart_status`: 1 atenção, 2 crítico, palavras desconhecidas contam como 1). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology disk failed | O DSM reporta o disco como avariado ou falhado (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology disk bad sectors | A contagem de setores defeituosos do disco excede o limiar definido no DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 min | Warning (`critical`) | 24 h |
| Synology disk bad sectors growing | Novos setores ilegíveis apareceram nas últimas 24 horas (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology SSD wearing out | Um SSD tem menos de 10% da sua vida nominal restante (`dumbmonit_synology_disk_remaining_life_percent`); limpa-se acima de 12%. | < 10 % | 1 h | Advisory (`warning`) | 7 d |
| Synology volume degraded | Um volume está degradado ou avariado: a sua redundância desapareceu (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology storage pool degraded | Uma storage pool ou cache SSD está degradada ou avariada: um disco falhou e a redundância desapareceu ou está a ser reconstruída (`dumbmonit_synology_pool_status or dumbmonit_synology_ssd_cache_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology volume almost full | Um volume está 90% cheio ou mais (`dumbmonit_synology_volume_used_percent`); limpa-se abaixo de 88%. | ≥ 90 % | 15 min | Advisory (`warning`), agrava depois de 24 h | 6 h |
| Synology temperature high | Um disco está acima de 55 °C, ou o DSM levantou o seu próprio aviso de temperatura (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); limpa-se abaixo de 52 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| Synology memory high | Uso de memória do NAS acima de 95% (`dumbmonit_synology_memory_usage_percent`); limpa-se abaixo de 90%. | > 95 % | 15 min | Advisory (`warning`) | 6 h |
| Active Backup task failed | A última execução de uma tarefa Active Backup for Business falhou, ou fez cópia de segurança só de parte dos seus dispositivos (`dumbmonit_abb_task_last_status == bool 0`). A notificação nomeia a tarefa. | > 0 | 10 min | Warning (`critical`) | 24 h |
| Active Backup too old | Nenhuma execução Active Backup for Business bem-sucedida para esta tarefa há mais de dois dias (`dumbmonit_abb_task_last_success_seconds`). A série não existe para uma tarefa que nunca teve sucesso; «Active Backup task failed» fala então. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Active Backup task disabled | A tarefa não tem calendário, ou a sua cópia de segurança contínua está pausada (`dumbmonit_abb_task_enabled == bool 0`): não fará cópia de segurança de nada até alguém a executar. | > 0 | 1 h | Info (`info`) | 7 d |
| Active Backup device overdue | Um dispositivo passou mais tempo sem uma execução Active Backup for Business bem-sucedida do que o seu próprio ritmo permite, dias de descanso excluídos (`dumbmonit_abb_device_overdue`, ver [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Active Backup device failing | As duas ou mais últimas tentativas de um dispositivo falharam (`dumbmonit_abb_device_consecutive_failures`); uma execução cancelada não conta como nenhuma das duas. | ≥ 2 | 10 min | Warning (`critical`) | 24 h |

### Servidores de logs e métricas {#log-and-metrics-servers}

VictoriaMetrics, VictoriaLogs, Loki e Graylog
([dispositivos](../devices/victoriametrics.md)). Os contadores passam
por `increase_prometheus`, por isso um servidor adicionado com um
histórico de dados recusados não dispara na sua primeira medição.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Metrics or log storage read-only | O espaço livre em disco caiu abaixo de `-storage.minFreeDiskSpaceBytes` e o armazenamento recusa toda escrita (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 min | Warning (`critical`) | 1 h |
| Metrics or log storage almost read-only | Menos de 10% do disco restante antes de só leitura (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); limpa-se acima de 12%. | < 10 % | 30 min | Advisory (`warning`) | 24 h |
| Metrics or log server unhealthy | `/health` não responde OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 min | Warning (`critical`) | 6 h |
| Metrics or log data refused | Amostras ou linhas de log recusadas na última hora: data-horas fora da retenção, etiquetas demasiado longas, limite de série (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| VictoriaMetrics slow inserts | Mais de 5% das novas amostras seguiram o caminho lento ao longo de 15 minutos: memória insuficiente para as séries ativas (`(100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])) and on (target) dumbmonit_victoriametrics_uptime_seconds > 900`); limpa-se abaixo de 3%. Só é avaliada depois de o servidor estar em execução há 15 minutos, para que um reinício (cache fria, contadores a zero dentro da janela) não a deixe pendente. | > 5 % | 30 min | Advisory (`warning`) | 24 h |
| Loki not ready | `/ready` não responde ready (`dumbmonit_loki_ready`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Loki refusing log lines | Linhas recusadas na última hora: demasiado antigas, limite de taxa ou stream, linha demasiado longa (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Loki cannot flush to storage | Chunks falharam ao chegar ao armazenamento nos últimos 30 minutos; perdem-se se o Loki reiniciar (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Loki write-ahead log disk full | Escritas WAL perdidas por um disco cheio (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Loki request errors | Mais de cinco pedidos respondidos com um erro de servidor em 15 minutos (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 min | Advisory (`warning`) | 6 h |
| Graylog not processing | Processamento de mensagens pausado no nó (`dumbmonit_graylog_processing`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Graylog search cluster down | OpenSearch ou Elasticsearch vermelho, ou inacessível do Graylog (`dumbmonit_graylog_indexer_status`: 2 vermelho, 3 inacessível). | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Graylog search cluster yellow | Shards de réplica não atribuídos durante meia hora (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Graylog journal filling | O journal está mais de metade cheio: a saída não acompanha (`dumbmonit_graylog_journal_used_percent`); limpa-se abaixo de 40%. | > 50 % | 10 min | Advisory (`warning`), agrava depois de 1 h | 6 h |
| Graylog input failed | Uma entrada falhou ao iniciar (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Graylog indexing failures | Escritas para o cluster de pesquisa ou processamento de mensagens falharam na última hora (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### Aplicações autoalojadas {#self-hosted-applications}

Nextcloud, Immich, Paperless-ngx, Jellyfin e Plex
([dispositivos](../devices/nextcloud.md)). Raramente caem
completamente: a página abre enquanto a sincronização, miniaturas,
importações ou análises de biblioteca deixaram de correr. Estas
regras vigiam esses estados; uma aplicação que não responde de todo é
Device unreachable.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Nextcloud stuck in maintenance | O modo de manutenção está ativo (`dumbmonit_nextcloud_maintenance`): ninguém pode iniciar sessão ou sincronizar. | > 0 | 30 min | Advisory (`warning`), agrava depois de 3 h | 6 h |
| Nextcloud database upgrade pending | O código foi atualizado e o Nextcloud espera por `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Nextcloud update available | A verificação de atualização do Nextcloud encontrou uma versão mais recente (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Nextcloud OPcache full | O OPcache do PHP está sem espaço e as páginas ficam lentas (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Nextcloud free space low | Menos de 5 GiB livres para o diretório de dados (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); limpa-se acima de 6 GiB. | < 5 GiB | 30 min | Advisory (`warning`) | 24 h |
| Immich jobs stalled | Tarefas esperam numa fila que não está pausada e não corre nada (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). A notificação nomeia a fila. | > 0 | 1 h | Advisory (`warning`) | 6 h |
| Immich queue paused | Uma fila de tarefas está pausada há um dia (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 d |
| Immich storage almost full | O disco que contém a biblioteca está mais de 90% cheio (`dumbmonit_immich_storage_used_percent`); limpa-se abaixo de 88%. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Immich update available | A verificação de versão do Immich conhece uma versão mais recente (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Paperless task queue down | Redis ou Celery em erro (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 é um erro): nada novo é importado. | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Paperless index or sanity check error | O índice de pesquisa, o classificador ou a verificação de integridade (ficheiros em falta ou alterados) reporta um erro (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 min | Advisory (`warning`) | 24 h |
| Paperless migrations pending | Migrações de base de dados a aplicar (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Paperless task failed | Uma tarefa que este utilizador pode ver falhou dentro da janela de tarefas falhadas e não foi dispensada (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Paperless storage almost full | O disco que contém os documentos está mais de 90% cheio (`dumbmonit_paperless_storage_used_percent`); limpa-se abaixo de 88%. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Paperless update available | A verificação de versão do Paperless-ngx conhece uma versão mais recente (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Jellyfin scheduled task failed | Uma tarefa agendada (uma análise de biblioteca, uma limpeza) falhou ou foi interrompida na sua última execução (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Jellyfin plugin broken | Um plugin falhou ao carregar ou não suporta esta versão (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Jellyfin restart pending | O Jellyfin espera há um dia por um reinício depois de uma instalação ou atualização de plugin (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 d |
| Plex update available | A verificação de atualização do Plex encontrou uma versão mais recente (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Forjas autoalojadas {#self-hosted-forges}

GitLab (autoalojado) e Forgejo/Gitea
([dispositivos](../devices/gitlab.md)). Uma instância que não
responde de todo é Device unreachable; estas regras vigiam as partes
que continuam a servir páginas enquanto silenciosamente ficam atrás —
Sidekiq, executores CI/CD, uma tarefa agendada que deixou de correr.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| GitLab runners all offline | Todo executor CI/CD que o GitLab conhece está offline, e pelo menos um está registado (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| GitLab Sidekiq backlog high | Uma fila do Sidekiq tem mais de 500 tarefas à espera (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): notificações, webhooks e importações ficam atrás. | > 500 | 15 min | Advisory (`warning`) | 6 h |
| GitLab migrations pending | Migrações de base de dados de uma atualização ficam por aplicar (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Info (`info`) | 24 h |
| GitLab license expiring | A licença Enterprise Edition expira em menos de 30 dias (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 24 h |
| Forgejo/Gitea runners all offline | Todo executor de Actions está offline, e pelo menos um está registado (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Forgejo/Gitea scheduled task overdue | Uma tarefa agendada (verificação de repositório, limpeza, verificador de atualizações) não correu no seu próprio intervalo (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 min | Advisory (`warning`) | 24 h |

### Dispositivos cliente {#client-devices}

Uma regra, partilhada por toda integração que acompanha dispositivos
a ligar e a fazer cópia de segurança — os telefones, tablets e
desktops do Immich ([dispositivos](../devices/immich.md)), as
máquinas com cópia de segurança do Proxmox Backup Server, uma por
grupo de cópia de segurança ([dispositivos](../devices/pbs.md)), e os
computadores e VMs protegidos do Veeam, um por objeto com um ponto de
restauro ([dispositivos](../devices/veeam.md)), hoje; mais a seguir.
Cada integração calcula o intervalo até à sua própria janela de
obsolescência de destino (`device_stale_days`, 3 dias por
predefinição) e publica-o como `dumbmonit_client_device_stale_seconds`,
etiquetado `device`, `type`, `os`, `user`, `kind` e `signal`
(`connection` ou `backup`); esta única regra então dispara para
qualquer um deles, nomeando o dispositivo na notificação. O Synology
Active Backup for Business acompanha os seus próprios dispositivos
separadamente, através de um mecanismo consciente de ritmo
(`synology_abb_device_*`) em vez desta família genérica — ver
[Synology](../devices/synology.md).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Client device stale | Um dispositivo não se ligou ou não fez cópia de segurança dentro da janela de obsolescência configurada para este destino (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Advisory (`warning`) | 24 h |

### Pi-hole {#pi-hole}

[Pi-hole](../devices/pihole.md) v6, lido através da sua API REST.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Pi-hole blocking disabled | O bloqueio está desligado, pausado ou falhou (`dumbmonit_pihole_blocking_enabled`). A retenção deixa passar uma pausa de cinco minutos da interface web. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Pi-hole blocklists stale | As listas de bloqueio (gravity) não foram reconstruídas há mais de oito dias (`dumbmonit_pihole_gravity_age_seconds`): a atualização semanal falha. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| Pi-hole update available | Uma versão mais recente está disponível (`dumbmonit_pihole_updates_available`): a imagem no Docker, core, interface web ou FTL caso contrário. O Pi-hole verifica uma vez por dia. | > 0 | 1 h | Info (`info`) | 7 d |
| Pi-hole diagnosis messages | O Pi-hole lista mensagens na sua página de diagnóstico (`dumbmonit_pihole_messages`): uma lista de bloqueio que falhou ao descarregar, um cliente limitado por taxa, um anfitrião sobrecarregado. | > 0 | 15 min | Info (`info`) | 24 h |

### AdGuard Home {#adguard-home}

O filtro DNS ([dispositivo](../devices/adguard.md)). Os valores de
consultas e bloqueios são somas sobre a própria janela de estatísticas
do AdGuard Home, não contadores: nenhuma regra é construída sobre
eles.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| AdGuard Home protection off | Proteção desativada, ou pausada e ainda não de volta (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 min | Advisory (`warning`) | 6 h |
| AdGuard Home DNS not running | A interface web responde mas o servidor DNS está parado (`dumbmonit_adguard_running`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| AdGuard Home upstream failing | Um servidor DNS a montante falha o próprio teste do AdGuard Home, ou fica silencioso 4 segundos (`dumbmonit_adguard_upstream_up`, uma série por servidor a montante). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| AdGuard Home filter lists stale | A atualização mais antiga de uma lista de filtros ativada tem mais de três dias (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 dias | 1 h | Advisory (`warning`) | 24 h |
| AdGuard Home update available | Uma versão mais recente é anunciada pela própria verificação de versão do AdGuard Home (`dumbmonit_adguard_update_available`); nunca dispara quando essa verificação está desativada, como na imagem Docker oficial. | > 0 | 1 h | Info (`info`) | 7 dias |

### UPS por detrás do NUT {#ups-behind-nut}

UPS lido de um [servidor NUT](../devices/nut.md). Toda série transporta
a etiqueta `ups`, por isso um alerta nomeia o UPS quando um servidor
publica vários. As flags de estado são 0 ou 1 em cada leitura, por
isso uma regra limpa-se por si só assim que a corrente elétrica volta.
Os UPS lidos via SNMP são cobertos pelas regras [UPS](#ups) acima.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| UPS on battery (NUT) | O UPS está a alimentar a carga a partir da bateria (`dumbmonit_nut_ups_on_battery`, a flag `OB` de `ups.status`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low (NUT) | O UPS reporta uma bateria baixa, a flag `LB` (`dumbmonit_nut_ups_low_battery`): as máquinas que alimenta estão a ponto de se encerrar. | > 0 | 1 min | Warning (`critical`) | 30 min |
| UPS battery needs replacing (NUT) | O UPS pede que a sua bateria seja substituída, a flag `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| UPS load high (NUT) | A carga de trabalho manteve-se acima de 80% (`dumbmonit_nut_ups_load_percent`); limpa-se abaixo de 75%. | > 80 % | 10 min | Advisory (`warning`) | 24 h |
| UPS runtime short (NUT) | O UPS estima menos de cinco minutos de autonomia na bateria (`dumbmonit_nut_battery_runtime_seconds`); limpa-se acima de seis. Com corrente elétrica, uma bateria gasta ou um UPS demasiado pequeno para a sua carga. | < 300 s | 5 min | Advisory (`warning`) | 24 h |
| UPS data stale (NUT) | O servidor NUT responde mas não tem dados frescos do UPS: cabo, driver ou UPS silencioso (`dumbmonit_nut_ups_data_stale`). Uma indisponibilidade passaria despercebida entretanto. | > 0 | 5 min | Advisory (`warning`), agrava depois de 1 h | 6 h |

### MikroTik RouterOS {#mikrotik-routeros}

As séries transportam a etiqueta `interface` ou `sensor`, por isso
uma notificação nomeia a porta ou o sensor
([dispositivo](../devices/mikrotik.md)). Os contadores passam por
`increase_prometheus`: um router adicionado com anos de erros atrás
dele não dispara na sua primeira medição.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| MikroTik CPU high | A CPU do router manteve-se acima de 90% (`dumbmonit_mikrotik_cpu_load_percent`): o encaminhamento, a firewall ou um script está a saturá-la. Limpa-se abaixo de 80%. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik memory high | Mais de 90% da memória está em uso (`dumbmonit_mikrotik_memory_used_percent`). Limpa-se abaixo de 85%. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik storage almost full | O armazenamento flash está mais de 90% cheio (`dumbmonit_mikrotik_storage_used_percent`): alterações de configuração e atualizações podem falhar. Limpa-se abaixo de 85%. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| MikroTik temperature high | Um sensor de temperatura lê acima de 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Limpa-se a 75 °C. | > 80 °C | 10 min | Advisory (`warning`) | 6 h |
| MikroTik power supply or fan failed | Um sensor `psu*-state` ou `fan*-state` não está `ok` (`dumbmonit_mikrotik_health_ok`). Uma fonte de alimentação redundante que morreu não corta nada, até a segunda também o fazer. | < 1 | 5 min | Warning (`critical`) | 1 h |
| MikroTik interface accumulating errors | Mais de cinquenta erros de receção e transmissão numa interface na última hora (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| MikroTik link flapping | Uma interface perdeu a sua ligação mais de duas vezes em trinta minutos (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Uma porta desligada em definitivo conta uma vez e não dispara. | > 2 | 5 min | Advisory (`warning`) | 1 h |
| MikroTik RouterOS update available | Um RouterOS mais recente está disponível no canal de atualização do router (`dumbmonit_mikrotik_update_available`). Só conhecido quando o próprio router verifica atualizações: ver o passo opcional da [configuração](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 d |
| MikroTik firmware upgrade pending | O RouterBOOT é mais antigo do que o firmware incluído com o RouterOS instalado (`dumbmonit_mikrotik_firmware_upgrade_pending`); só é atualizado a pedido, depois no próximo reinício. | > 0 | 1 h | Info (`info`) | 7 d |

### UniFi Network {#unifi-network}

[Página do dispositivo](../devices/unifi.md). As séries de dispositivo
transportam o nome, modelo e tipo do dispositivo, por isso a
notificação diz «Living room AP». Um dispositivo à espera de adoção
não tem série `device_up` e nunca conta como offline.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| UniFi device offline | Um dispositivo UniFi adotado pelo controlador está offline, isolado ou falhou ao adotar (`dumbmonit_unifi_device_up`). | < 1 | 5 min | Advisory (`warning`), agrava depois de 1 h | 6 h |
| UniFi Internet down | O gateway UniFi reporta nenhuma ligação à Internet (`dumbmonit_unifi_internet_up`). Sem um gateway UniFi a série não existe e a regra fica silenciosa. | < 1 | 3 min | Warning (`critical`) | 1 h |
| UniFi WAN link down | Uma ligação WAN do gateway UniFi está em baixo (`dumbmonit_unifi_wan_link_up`): com duas ligações, a redundância perde-se enquanto nada parece avariado. Uma porta desativada não tem série. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| UniFi gateway CPU high | O processador do gateway UniFi esteve acima de 90% durante quinze minutos (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); limpa-se abaixo de 80%. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| UniFi device waiting for adoption | Um dispositivo apareceu na rede e espera para ser adotado no UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 min | Info (`info`) | 24 h |
| UniFi firmware update available | Uma atualização de firmware está disponível para dispositivos UniFi (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 d |
| UniFi alarms | O UniFi registou eventos críticos ou levantou alarmes nas últimas 24 horas (`dumbmonit_unifi_alarms`). Só lido com uma conta View Only. | > 0 | 5 min | Advisory (`warning`) | 24 h |

### Home Assistant {#home-assistant}

[Página do dispositivo](../devices/homeassistant.md).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Home Assistant in recovery mode | O Home Assistant arrancou em modo de recuperação: a sua configuração não pôde ser carregada e as automações não correm (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| Home Assistant not running | O Home Assistant responde mas não terminou de arrancar há dez minutos (`dumbmonit_homeassistant_running`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant entities went unavailable | Pelo menos cinco entidades mais estão indisponíveis do que há uma hora: uma integração, um hub ou uma rádio parou (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). Conta a subida, não o total, que uma instalação real nunca tem a zero. | ≥ 5 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant battery low | Uma bateria reportada pelo Home Assistant está baixa (`dumbmonit_homeassistant_battery_low`, uma série por bateria abaixo do limiar do dispositivo, 20% por predefinição; o seu valor é o nível). | ≥ 0 % | 30 min | Info (`info`) | 3 d |
| Home Assistant update available | O Home Assistant tem atualizações à espera: core, sistema operativo, add-ons ou firmware de dispositivo (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Home Assistant repair to address | O Home Assistant tem uma reparação aberta de severidade error ou critical (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### VMware vSphere {#vmware-vsphere}

[Página do dispositivo](../devices/vsphere.md). As séries de anfitriões trazem `esxi`, as de VM `vm`, as de datastores `datastore`, pelo que a notificação nomeia o objeto.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| vSphere host not responding | Um anfitrião vSphere não está a responder ao vCenter (`dumbmonit_vsphere_host_connection_state`: 2 sem resposta). Um anfitrião desligado de propósito (1) não dispara. | ≥ 2 | 3 min | Warning (`critical`) | 1 h |
| vSphere host health red | O vSphere reporta a saúde geral de um anfitrião como vermelha (`dumbmonit_vsphere_host_status == 2`). Cinza significa desconhecido e não dispara. | > 0 | 5 min | Warning (`critical`) | 1 h |
| vSphere host left in maintenance | Um anfitrião vSphere está em modo de manutenção há um dia: as suas VMs foram movidas e pode ter sido esquecido lá (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| vSphere datastore almost full | Um datastore vSphere está 90% cheio ou mais (`dumbmonit_vsphere_datastore_used_percent`); limpa-se abaixo de 85%. | ≥ 90 % | 15 min | Advisory (`warning`), agrava depois de 24 h | 6 h |
| vSphere datastore inaccessible | Um datastore vSphere está inacessível: as VMs guardadas nele não podem correr (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| vSphere VM tools not running | As VMware Tools estão instaladas mas não em execução numa VM ligada: o convidado pode estar pendurado (`dumbmonit_vsphere_vm_tools_status == 2`). VMs sem Tools não disparam. | > 0 | 30 min | Info (`info`) | 24 h |
| vSphere red alarm | O vSphere levantou um alarme vermelho que ninguém confirmou (`dumbmonit_vsphere_alarm{status="red"}`, uma série por alarme, nomeada pelo alarme e pelo objeto). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Bases de dados, broker de mensagens e motor de segurança {#databases-message-broker-and-security-engine}

Redis / Valkey, MongoDB, RabbitMQ e CrowdSec
([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md),
[RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)).
Os contadores passam por `increase_prometheus` ou
`changes_prometheus`, por isso nada dispara na primeira medição de um
dispositivo.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Redis memory near maxmemory | Memória usada acima de 90% do `maxmemory` (`dumbmonit_redis_memory_used_percent`); limpa-se abaixo de 85%. Nenhuma série sem `maxmemory`. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| Redis refusing connections | Ligações de cliente recusadas nos últimos 15 minutos: `maxclients` atingido (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Redis replication link down | Uma réplica perdeu a sua ligação ao primário (`dumbmonit_redis_master_link_up`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| Redis replica lagging | Visto do primário, uma réplica não confirmou há mais de 30 segundos (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 min | Advisory (`warning`) | 6 h |
| Redis snapshot failed | O último `BGSAVE` falhou; com `stop-writes-on-bgsave-error`, as escritas são recusadas (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| Redis append-only file write failed | A última escrita AOF falhou (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replica set without primary | Nenhum membro é primário: nenhuma escrita é aceite (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| MongoDB member unreachable | Um membro do replica set está inacessível dos outros (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replication lag | Um secundário está mais de um minuto atrás do primário (`dumbmonit_mongodb_replset_member_lag_seconds`); limpa-se abaixo de 30 s. | > 60 s | 10 min | Advisory (`warning`) | 6 h |
| MongoDB connections near the limit | Mais de 80% das ligações disponíveis em uso (`dumbmonit_mongodb_connections_used_percent`); limpa-se abaixo de 75%. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB cache under pressure | Mais de 20% da cache do WiredTiger está sujos (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); limpa-se abaixo de 15%. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB internal errors | Assertions regulares na última hora (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| RabbitMQ memory or disk alarm | Um nó levantou um alarme de recurso: todo publicador está bloqueado (`dumbmonit_rabbitmq_alarm`, uma série por nó e recurso). | > 0 | 1 min | Warning (`critical`) | 1 h |
| RabbitMQ node down | Um nó do cluster não está em execução (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ network partition | Um nó vê uma partição de rede (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ queue without consumer | Uma fila detém mensagens e não tem nenhum consumidor (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); o valor é o número de mensagens à espera. | > 0 | 15 min | Advisory (`warning`) | 6 h |
| RabbitMQ queue unavailable | Uma fila não está em execução: fila quorum sem maioria, ou o seu nó está em baixo (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec Local API down | A Local API não responde a `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| CrowdSec bouncer stopped pulling | Os pedidos de um bouncer não mudaram em 30 minutos (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec reads no logs | Nenhuma linha de log lida em seis horas (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Proxies inversos e domínios {#reverse-proxies-and-domains}

Traefik, Caddy, Nginx Proxy Manager e registos de domínio ([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md), [Nginx Proxy Manager](../devices/npm.md), [Expiração de domínio](../devices/domain.md)), e a opção *Alertar quando a resposta muda* da verificação DNS ([Serviços](../devices/services.md#alerting-when-the-answer-changes)). As séries trazem `router`, `service`, `server`, `upstream`, `proxy_host` ou `cert`, pelo que a notificação nomeia o que falhou.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Traefik router disabled | Um router é desativado por um erro de configuração (serviço ou middleware em falta, regra inválida): os seus sites não são servidos (`dumbmonit_traefik_router_status`: 0 ativado, 1 aviso, 2 desativado). | ≥ 2 | 5 min | Advisory (`warning`) | 6 h |
| Traefik backend server down | Um servidor de um serviço com balanceamento de carga falha a sua verificação de saúde (`dumbmonit_traefik_server_up`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Traefik service without a server | Nenhum servidor de um serviço responde à sua verificação de saúde: os seus sites recebem 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| Traefik certificate not obtained | Routers pedem a um resolvedor de certificado um certificado que nenhum certificado detido pelo Traefik ainda cobre: o Traefik serve o seu certificado autoassinado predefinido (`dumbmonit_traefik_resolver_routers_uncovered`, precisa de `/api/certificates` do Traefik). | > 0 | 1 h | Advisory (`warning`) | 24 h |
| Caddy upstream unhealthy | Um backend falha a verificação de saúde ativa ou passiva do Caddy (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Caddy configuration reload failed | O Caddy recusou a sua última recarga de configuração e continua a executar a anterior (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 min | Advisory (`warning`) | 24 h |
| Nginx Proxy Manager host offline | O nginx recusou a configuração de um anfitrião ativado: o anfitrião não é servido (`dumbmonit_npm_host_online`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Proxy certificate not renewed | Um certificado detido pelo Traefik ou apresentado por um anfitrião Nginx Proxy Manager expira em menos de 14 dias: a renovação automática começa aos 30, por isso está a falhar (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Reverse proxy answering 5xx | Mais de 5% das respostas em dez minutos foram 5xx, com pelo menos cinquenta pedidos (`increase_prometheus` de `dumbmonit_{traefik,caddy}_requests_5xx_total` sobre `…_requests_total`). Precisa das métricas do proxy. | > 5 % | 10 min | Advisory (`warning`) | 6 h |
| Domain expiring | O domínio expira em menos de 30 dias (`dumbmonit_domain_expiry_days >= 7`). | < 30 d | 1 h | Advisory (`warning`) | 24 h |
| Domain about to expire | O domínio expira em menos de 7 dias, ou já expirou (`dumbmonit_domain_expiry_days`). | < 7 d | 1 h | Warning (`critical`) | 12 h |
| Domain on hold | O registo colocou o domínio em `clientHold` ou `serverHold`: está fora do DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 min | Warning (`critical`) | 6 h |
| Domain in redemption | O domínio está em `redemptionPeriod`, `pendingDelete` ou `pendingRestore`: expirou (`dumbmonit_domain_redemption`). | > 0 | 30 min | Warning (`critical`) | 12 h |
| DNS answer changed | A resposta de uma verificação DNS com *Alert when the answer changes* difere da anterior (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Resolve-se por si só uma hora depois. | > 0 | 1 min | Advisory (`warning`) | 24 h |

### Alterações de site {#website-changes}

Ver [Alterações de site](../devices/webchange.md). A página do
dispositivo mostra cada alteração antes e depois.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Website changed | Uma verificação de uma página ou site monitorizado encontrou páginas que mudaram, apareceram ou desapareceram (`dumbmonit_webchange_last_check_changes`). Cada verificação assim é notificada e nomeia as páginas, sob o mesmo alerta enquanto dispara; resolve-se na primeira verificação que não encontra nada de novo. | > 0 | 30 s | Info (`info`) | nenhum |

### Kubernetes {#kubernetes}

Ver [Kubernetes / k3s](../devices/kubernetes.md). Toda série é
etiquetada com o nó, ou o namespace e o pod, workload ou
reivindicação em causa.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Kubernetes node not ready | Um nó não está Ready: o seu kubelet já não reporta, e os seus pods serão despejados (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Kubernetes node under pressure | Um nó reporta pressão de memória, disco ou PID (`dumbmonit_k8s_node_pressure`, etiqueta `condition`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes pod crash looping | Um contentor está em `CrashLoopBackOff` e ainda a reiniciar: pelo menos dois reinícios em 15 minutos (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) and on (target, namespace, pod) increase_prometheus(dumbmonit_k8s_pod_restarts[15m]) >= 2`; o estado desaparece por um momento a cada tentativa de reinício). Contentores mortos por um reinício do nó ou do runtime (saída 137, 143 ou 255, não `OOMKilled`) não contam, para que um reboot não pareça um ciclo de falhas. | > 0 | 10 min | Warning (`critical`) | 6 h |
| Kubernetes pod stuck pending | Um pod está Pending há quinze minutos: nenhum nó o pode levar, ou um volume ou imagem que precisa não chega (`dumbmonit_k8s_pod_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Kubernetes pod restarting | Um pod reiniciou mais de cinco vezes na última hora sem estar em `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes workload missing replicas | Um Deployment, StatefulSet ou DaemonSet tem menos réplicas disponíveis do que pede (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 min | Advisory (`warning`), agrava depois de 1 h | 6 h |
| Kubernetes volume claim pending | Um PersistentVolumeClaim que um pod precisa, ou que um erro de provisionamento afeta, continua não bound (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |

### Active Directory {#active-directory}

Ver [Active Directory](../devices/activedirectory.md). O controlador
de domínio que o DumbMonit consulta é coberto por *Device
unreachable*; os outros controladores de domínio do domínio por
*Domain controller unreachable* (etiqueta `dc`).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Domain controller unreachable | Um controlador de domínio do domínio já não responde na sua porta LDAP (`dumbmonit_ad_dc_reachable == bool 0`). Um nome que não resolve do DumbMonit não produz série e nunca dispara. | > 0 | 5 min | Warning (`critical`) | 6 h |
| Active Directory bind failed | O controlador de domínio responde mas recusa a conta de serviço: senha expirada, conta desativada ou bloqueada (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Privileged group membership changed | Um membro juntou-se ou saiu de Domain Admins, Enterprise Admins, Schema Admins, Administrators ou um grupo de operadores, aninhamento incluído (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, etiqueta `group`). Resolve-se por si só uma hora depois. | > 0 | 1 min | Advisory (`warning`) | 24 h |
| Active Directory replication failing | O controlador de domínio consultado não consegue replicar de um parceiro (`dumbmonit_ad_replication_consecutive_failures`). Só quando a conta pode ler o estado de replicação. | > 0 | 30 min | Advisory (`warning`) | 6 h |
| krbtgt password older than 180 days | A senha krbtgt foi alterada por último há mais de 180 dias (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 dias | 10 min | Info (`info`) | nenhum |

### pfSense {#pfsense}

Ver [pfSense](../devices/pfsense.md). Os gateways são etiquetados
`gateway`, as interfaces `interface` e `descr`, os serviços
`service`.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| pfSense gateway down | Um gateway pfSense está em baixo: o seu endereço de monitorização já não responde, e o tráfego que transporta para ou falha para outro (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 min | Warning (`critical`) | 6 h |
| pfSense gateway losing packets | Um gateway pfSense perde mais de 10% das sondas ao seu endereço de monitorização: a linha está degradada (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 min | Advisory (`warning`) | 6 h |
| pfSense gateway slow | O tempo de ida e volta até ao endereço de monitorização de um gateway pfSense manteve-se acima de 500 ms durante quinze minutos (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 min | Advisory (`warning`) | 6 h |
| pfSense interface without link | Uma interface pfSense ativada não tem ligação: cabo, porta de comutador ou placa de rede (`dumbmonit_pfsense_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| pfSense service stopped | Um serviço pfSense ativado não está em execução: resolvedor DNS, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| pfSense disk nearly full | O disco do pfSense está mais de 90% cheio: logs, relatórios e atualizações de pacote deixam de ser escritos (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |

### Unraid {#unraid}

Ver [Unraid](../devices/unraid.md). Os discos são etiquetados `disk`
e `role` (`parity`, `data`, `cache`), os contentores `container`.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Unraid array stopped | O array Unraid não está iniciado: partilhas, contentores e VMs que vivem nele estão indisponíveis (`dumbmonit_unraid_array_started`). | < 1 | 10 min | Warning (`critical`) | 6 h |
| Unraid disk disabled or missing | Um disco Unraid está desativado, em falta ou inválido: o array corre degradado, e mais uma falha perde dados (`dumbmonit_unraid_disk_ok`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Unraid disk read errors | Um disco Unraid reportou erros de leitura desde que o array arrancou: pode ser desativado na próxima (`dumbmonit_unraid_disk_errors`). | > 0 | 5 min | Advisory (`warning`) | 1 d |
| Unraid disk too hot | Um disco de array ou paridade Unraid manteve-se acima de 55 °C durante quinze minutos (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| Unraid parity check found errors | A última verificação de paridade Unraid falhou ou encontrou erros de sincronização: a paridade já não corresponde aos dados (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 min | Advisory (`warning`) | 1 d |
| Unraid parity check overdue | A última verificação de paridade Unraid tem mais de 40 dias: uma verificação mensal foi perdida (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 d | 1 h | Advisory (`warning`) | 1 d |
| Unraid cache nearly full | Uma pool de cache Unraid está mais de 90% cheia: as escritas recuam para o array lento, ou falham (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |
| Unraid container stopped | Um contentor Docker definido para iniciar com o array não está em execução (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 min | Advisory (`warning`) | 6 h |

### Veeam Backup & Replication {#veeam-backup--replication}

Ver [Veeam Backup & Replication](../devices/veeam.md). As tarefas são
etiquetadas `job` e `type`, os repositórios `repository`.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Veeam job failed | A última execução de uma tarefa Veeam ativada falhou: o que protege não tem nenhum novo ponto de restauro (`dumbmonit_veeam_job_failed`). | > 0 | 5 min | Warning (`critical`) | 1 d |
| Veeam job ended with a warning | A última execução de uma tarefa Veeam ativada terminou com um aviso: alguns objetos podem não estar protegidos (`dumbmonit_veeam_job_warning`). | > 0 | 5 min | Advisory (`warning`) | 1 d |
| Veeam repository nearly full | Um repositório de cópia de segurança Veeam está mais de 90% cheio: as tarefas seguintes falharão por falta de espaço (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 min | Advisory (`warning`) | 1 d |
| Veeam license expiring | A licença Veeam expira em menos de 30 dias: as tarefas param no fim do período de graça (`dumbmonit_veeam_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 1 d |

### Tailscale {#tailscale}

Ver [Tailscale](../devices/tailscale.md). Os dispositivos são
etiquetados `device`, a primeira etiqueta do seu nome MagicDNS.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Tailscale device offline | Um dispositivo Tailscale que deve ficar online (um dispositivo etiquetado por predefinição) está desligado há mais de 15 minutos (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 min | 5 min | Advisory (`warning`) | 6 h |
| Tailscale node key expiring | A chave de nó de um dispositivo Tailscale expira em menos de 14 dias: sairá da tailnet até alguém iniciar sessão nele de novo (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 d | 1 h | Advisory (`warning`) | 1 d |
| Tailscale device awaiting approval | Um dispositivo juntou-se à tailnet e espera aprovação: aprove-o se for esperado, remova-o caso contrário (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 min | Advisory (`warning`) | 1 d |
| Tailscale client update | Um dispositivo Tailscale que deve ficar online corre um cliente com uma atualização disponível (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Info (`info`) | 7 d |

### FortiGate {#fortigate}

Ver [FortiGate](../devices/fortigate.md). Os túneis são etiquetados
`tunnel`, as interfaces `interface` e `alias`, as licenças `license`.

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| FortiGate IPsec tunnel down | Nenhuma fase 2 de um túnel IPsec site-to-site do FortiGate está ativa: o site por detrás dele está cortado (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| FortiGate interface without link | Uma interface FortiGate ativada com um endereço não tem ligação: cabo, transceptor ou o dispositivo em frente (`dumbmonit_fortigate_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| FortiGate HA out of sync | Os membros de um cluster HA FortiGate já não têm o mesmo checksum de configuração: um failover executaria outra configuração (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 min | Advisory (`warning`) | 6 h |
| FortiGate HA member lost | Um cluster HA FortiGate tem menos membros do que nas últimas 24 horas: já não há nenhum em espera (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| FortiGate memory near conserve mode | A memória do FortiGate manteve-se acima de 85% durante dez minutos: a 88% (por predefinição) entra em modo de conservação e para de inspecionar novas sessões (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 min | Advisory (`warning`) | 6 h |
| FortiGate CPU high | A CPU do FortiGate manteve-se acima de 90% durante quinze minutos (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| FortiGate licence expiring | Uma licença FortiGuard ou FortiCare do FortiGate expira em menos de 30 dias, ou já expirou (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 d | 1 h | Advisory (`warning`) | 1 d |
| FortiGate firmware update | O FortiGuard oferece uma versão de correção mais recente do ramo FortiOS que o FortiGate executa (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Sophos Firewall {#sophos-firewall}

Ver [Sophos Firewall](../devices/sophos.md).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Sophos interface without link | Uma interface Sophos Firewall ligada e associada a uma zona não tem ligação (`dumbmonit_sophos_interface_down`). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Hyper-V {#hyper-v}

Reportado pelo agente Windows num anfitrião Hyper-V; ver
[Agente](../devices/agent.md#hyper-v-hosts).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Hyper-V VM health critical | O Hyper-V reporta uma máquina virtual em saúde crítica: um recurso de que precisa, mais frequentemente espaço em disco, está esgotado (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Hyper-V host CPU high | Os processadores lógicos do anfitrião Hyper-V estiveram mais de 90% ocupados durante quinze minutos, convidados incluídos (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| Hyper-V virtual disk errors | Um disco virtual Hyper-V reportou erros de I/O na última hora (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Pontuação de segurança {#security-score}

Calculada pelo servidor a cada 15 minutos para todo dispositivo cujo
tipo tem verificações de segurança, e guardada como
`dumbmonit_security_score` (0–100). Ver
[Pontuação de segurança](../using/security-score.md).

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Security score dropped | A pontuação caiu 10 pontos ou mais em relação ao seu melhor valor das últimas 24 horas (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): uma verificação começou a falhar. | ≥ 10 pts | 30 min | Advisory (`warning`) | 24 h |
| Security score low | A pontuação está abaixo de 40, nota F (`dumbmonit_security_score`). Suba ou desça o limiar para corresponder à sua própria barra. | < 40 | 1 h | Info (`info`) | 7 d |

### O próprio DumbMonit {#dumbmonit-itself}

| Regra | O quê | Limiar predefinido | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| DumbMonit backup did not run | A cópia de segurança local agendada da base de dados do DumbMonit não corre há mais de dois dias (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |

Toda regra incorporada aplica-se a todo dispositivo e a todo canal
ativado.

## Editar uma regra na interface {#editing-a-rule-in-the-ui}

Na página **Alerts**, a secção *Rules* lista toda regra com a sua
severidade, uma marca *Built-in* e, quando relevante, quantos alertas
está a disparar agora. Cada regra tem:

- um interruptor para a **ativar ou desativar**;
- **Edit**, que desdobra um editor em linha para nome, descrição, operador,
  limiar, retenção (`for`), severidade, lembrete (`repeat every`) e escalonamento
  (`escalate after`). A consulta de uma regra incorporada é mostrada mas não
  editável; a das suas próprias regras é. As regras de linha de base mostram
  os seus parâmetros de deteção só de leitura;
- **Delete**, apenas para as suas próprias regras.

## Substituições por dispositivo {#per-device-overrides}

Uma regra pode ser ajustada para um dispositivo sem tocar na regra em
si: um limiar diferente, um limiar de limpeza diferente, ou desligada
só para esse dispositivo. O editor de cada regra tem um painel
**Per-device overrides** para adicionar ou remover uma diretamente.

A ação de um clique **Ignore** num alerta (na lista *Needs you* ou na
página de dispositivo do alerta) é o mesmo mecanismo, escrito com
`enabled: false`: o dispositivo deixa de transportar essa regra, de
imediato, reversivelmente. A página do dispositivo lista o que
ignora, com **Stop ignoring**; a própria regra mostra *Ignored on N
devices* em Alerts → Rules, onde o mesmo painel de substituições por
dispositivo a remove. Ver
[Substituições por dispositivo](../reference/api.md#per-device-overrides)
para a API.

## Criar uma regra de limiar {#creating-a-threshold-rule}

Clique em **New rule**. O formulário precisa de:

| Campo | Significado |
|---|---|
| Name | Mostrado em alertas e notificações. |
| Query | Uma expressão MetricsQL, avaliada por dispositivo. Exemplo: `dumbmonit_cpu_usage_percent`. |
| Operator | `>`, `>=`, `<` ou `<=`. |
| Threshold | O número com que o resultado da consulta é comparado. |
| Severity | Info, Advisory ou Warning. |
| Hold for | Segundos que a condição tem de durar antes de disparar. Uma retenção não-zero absorve uma amostra ruim isolada. |

Toda série devolvida pela consulta torna-se um alerta potencial,
associado ao dispositivo nomeado pela sua etiqueta `target`. Agregue
com `avg by (target, host) (…)` quando quiser um alerta por
dispositivo em vez de um por núcleo ou por interface. A identidade do
alerta ignora as etiquetas `host` e `tag_*` sempre que `target` está
presente, por isso renomear um dispositivo mantém os seus alertas;
uma série cujo `target` não corresponde a um dispositivo existente e
ativado é ignorada.

## A linguagem de consulta {#the-query-language}

As consultas são [MetricsQL](https://docs.victoriametrics.com/metricsql/),
o superconjunto PromQL da VictoriaMetrics. As métricas são as séries
`dumbmonit_*` descritas na [referência de métricas](../reference/metrics.md),
e toda série transporta uma etiqueta `target` (o id do dispositivo) e
uma etiqueta `host` (o seu nome).

Alguns padrões usados pelas regras incorporadas:

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

!!! tip "Experimente uma consulta primeiro"
    `GET /api/metrics/query?query=…` devolve o que a regra avaliaria
    (ver a [referência da API](../reference/api.md#metrics)). Em
    desenvolvimento, a camada publica a VictoriaMetrics incorporada
    em `:8428` do anfitrião, com a sua interface de consulta.

## Tipos de regra na API {#rule-kinds-in-the-api}

`kind` é `threshold` (comparar com um limiar), `predict` (a mesma
comparação, sobre uma consulta `predict_linear`; mostrada como uma
previsão) ou `anomaly` (linha de base sazonal; `params` contém `k`,
`alpha`, `mad_floor_abs`, `mad_floor_rel`, `min_samples`). A interface
cria regras `threshold`; os outros dois tipos podem ser criados
através da API.
