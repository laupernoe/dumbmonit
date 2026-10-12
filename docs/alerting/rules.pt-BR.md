# Regras

Regras são inseridas na primeira inicialização, depois pertencem a você: o
DumbMonit nunca reescreve uma regra que você alterou, ele só recria as que
faltam. Regras integradas podem ser editadas e desativadas, não excluídas.

## Regras integradas

Severidades são mostradas com a palavra da interface; o valor da API está
entre parênteses. Uma regra cuja métrica nenhum dispositivo produz não tem
série e nunca dispara, então toda regra abaixo é entregue ativada em toda
instância.

### Todo dispositivo

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Device unreachable | Nenhuma medição recebida por mais de três minutos. `time() - tlast_over_time(dumbmonit_up[7d])` | > 180 s | 1 min | Alerta (`critical`) | 6 h |
| High CPU | Carga de CPU sustentada acima de 90%, média por dispositivo entre fontes SNMP, Proxmox e agente; limpa abaixo de 85%. | > 90 % | 10 min | Aviso (`warning`), escala depois de 1 h | 6 h |
| Unusual CPU | Carga de CPU perceptivelmente diferente do usual nesse horário e dia da semana (linha de base sazonal). | score > 3.5 | 15 min | Info (`info`) | 6 h |
| Disk almost full | Sistema de arquivos 90% cheio ou mais, por ponto de montagem (SNMP, storages Proxmox e sistemas de arquivos raiz); limpa abaixo de 88%. | ≥ 90 % | 15 min | Aviso (`warning`), escala depois de 24 h | 6 h |
| Filesystem almost full | Nesse ritmo, o sistema de arquivos ficará cheio dentro de quatro dias (`predict_linear` sobre 6 h; apenas sistemas de arquivos crescendo). | ≥ 100 % | 30 min | Aviso (`warning`) | 6 h |

### UPS

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| UPS on battery | O UPS está alimentando a carga pela bateria (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Alerta (`critical`) | 15 min |
| UPS battery low | A bateria do UPS está baixa ou esgotada (`dumbmonit_ups_battery_status`, 3 = baixa, 4 = esgotada). | ≥ 3 | 1 min | Alerta (`critical`) | 30 min |

### Portas de rede (SNMP)

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Port accumulating errors | Uma porta de rede registrou mais de cinquenta erros na última hora: geralmente um cabo falhando, uma fibra suja ou um desencontro de duplex (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; nada dispara durante a primeira hora de um dispositivo novo, então erros contados antes dele ser adicionado não são reportados como novos). | > 50 | 15 min | Aviso (`warning`) | 24 h |
| Port flapping | Uma porta de rede caiu e subiu de novo mais de duas vezes em trinta minutos (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Uma porta desativada de propósito não tem série e nunca dispara. | > 4 | 5 min | Aviso (`warning`) | 1 h |

### Hardware de servidor (Redfish)

Séries de saúde são 0 (OK), 1 (Aviso) ou 2 (Crítico). Um slot vazio
(`Absent`) ou desativado não tem série, então um servidor entregue com uma
fonte de alimentação de duas nunca dispara.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Server fan failed | O controlador de gerenciamento reporta uma ventoinha como falhada (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 min | Alerta (`critical`) | 6 h |
| Server temperature above critical | Um sensor de temperatura está acima do limite crítico que o próprio controlador declara (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 min | Alerta (`critical`) | 6 h |
| Power supply redundancy lost | As fontes de alimentação não são mais redundantes: mais uma falha e o servidor cai (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 min | Aviso (`warning`), escala depois de 1 h | 6 h |
| Power supply failed | O controlador reporta uma fonte de alimentação como falhada (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 min | Alerta (`critical`) | 6 h |
| Drive failure predicted | Um disco reporta uma falha prevista: substitua-o enquanto ainda funciona (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 min | Aviso (`warning`) | 24 h |
| Server health critical | O controlador reporta a própria saúde do sistema como Crítica (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 min | Alerta (`critical`) | 6 h |

### Monitores de disponibilidade e heartbeats

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Service down | O serviço não respondeu corretamente por três minutos (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 min | Alerta (`critical`) | 30 min |
| Heartbeat missed | O job não chamou dentro de seu intervalo esperado mais o período de tolerância, ou reportou uma falha ele mesmo (`dumbmonit_probe_success{probe="push"} == bool 0`; o prazo é definido por dispositivo, veja [Heartbeat](../devices/push.md)). | > 0 | 2 min | Aviso (`warning`) | 6 h |
| Service flapping | O serviço mudou de estado mais de seis vezes em trinta minutos (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 min | Aviso (`warning`) | 1 h |
| Slow service | O serviço leva mais de três segundos para responder. | > 3 s | 10 min | Aviso (`warning`) | 6 h |
| Certificate expiring soon | O certificado expira em menos de quatorze dias. | < 14 d | 1 h | Aviso (`warning`) | 24 h |
| Certificate expired | O certificado expirou. | < 0 d | 5 min | Alerta (`critical`) | 24 h |

### Contêineres Docker

Reportado pelo agente; veja [Agente](../devices/agent.md). O alerta nomeia
o contêiner.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Container stopped | O contêiner está parado há dois minutos (`dumbmonit_container_up == bool 0`). | > 0 | 2 min | Aviso (`warning`) | 1 h |
| Container unhealthy | A verificação de saúde do contêiner está falhando há três minutos (`dumbmonit_container_health == 2`; 0 nenhuma, 1 saudável, 3 iniciando). | > 0 | 3 min | Aviso (`warning`) | 1 h |
| Container restarting | O contêiner reiniciou três vezes ou mais em quinze minutos (`increase(dumbmonit_container_restart_count[15m])`): raramente fica parado o suficiente para a regra anterior. | ≥ 3 | 1 min | Aviso (`warning`) | 1 h |
| Container update available | Uma imagem mais nova está disponível no registro para este contêiner (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Backups Plakar

Reportado pelo agente, por kloset e por fonte. Nenhuma dessas séries existe
enquanto o Plakar não é detectado, então nenhuma regra pode disparar em
uma máquina sem ele.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Plakar backup too old | Nenhum snapshot Plakar novo para essa fonte por mais de dois dias (`dumbmonit_backup_last_success_seconds`). | > 2 d | 1 h | Aviso (`warning`) | 24 h |
| Plakar backup failed | O snapshot mais recente tem erros, ou o kloset não pode ser lido (`dumbmonit_backup_last_status < 1`). | < 1 | 10 min | Alerta (`critical`) | 24 h |

### Backups restic e Borg

Reportado pelo agente para cada repositório declarado sob `restic_repos`
ou `borg_repos` em agent.yaml; veja
[Agente](../devices/agent.md#restic-and-borg-backups). Sem um repositório
declarado não há série, então nenhuma regra pode disparar.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| restic/Borg backup too old | Nenhum snapshot restic ou Borg novo neste repositório por mais de 48 horas (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Aviso (`warning`) | 24 h |
| restic/Borg repository unreachable | O agente não consegue ler o repositório: inacessível, senha recusada ou binário faltando; o log do agente diz qual (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 min | Aviso (`warning`) | 24 h |

### Túneis WireGuard

Reportado pelo agente; veja
[Agente](../devices/agent.md#wireguard-tunnels).

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| WireGuard peer silent | Um peer com keepalive persistente não completou um handshake por mais de 15 minutos: o túnel está fora do ar (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Um peer que nunca conectou conta seu silêncio a partir do momento em que o agente o viu primeiro. Peers sem keepalive (um celular, um laptop) ficam inativos por dias e são deixados de fora; mude o limite para mudar N, ou remova a parte `and on …` para observar todo peer. | > 15 min | 5 min | Aviso (`warning`) | 6 h |

### Proxmox VE

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Backup too old | Nenhum backup Proxmox VE bem sucedido por mais de sete dias (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 d | 1 h | Aviso (`warning`) | 24 h |
| VM or container stopped | O guest estava rodando nas últimas duas horas e está parado há cinco minutos (`1 - dumbmonit_proxmox_guest_running`, combinado com `max_over_time(…[2h]) == 1`). | > 0 | 5 min | Aviso (`warning`) | 6 h |
| VM or container CPU high | Um guest usou mais de 90% de seus núcleos alocados por quinze minutos (`dumbmonit_proxmox_guest_cpu_percent`). A notificação nomeia o guest, `nextcloud (202)`. | > 90 % | 15 min | Aviso (`warning`) | 6 h |
| VM or container memory high | Um guest usou mais de 95% de sua memória por dez minutos (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 min | Aviso (`warning`) | 6 h |
| VM or container disk almost full | O disco raiz de um guest está mais de 90% cheio (`dumbmonit_proxmox_guest_disk_used_percent`): contêineres sempre, VMs só quando o agente QEMU guest reporta uso. | > 90 % | 15 min | Aviso (`warning`) | 24 h |
| Guest locked | Uma VM ou contêiner está bloqueado há seis horas (`dumbmonit_proxmox_guest_locked`, rótulo `lock`): um backup ou migração interrompido deixou o bloqueio e nenhuma operação pode rodar no guest, nem mesmo o próximo backup. | > 0 | 6 h | Aviso (`warning`) | 24 h |
| Old snapshot | O snapshot mais antigo de um guest tem mais de trinta dias (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 d | 1 h | Info (`info`) | 7 d |
| HA resource in error | Um recurso HA do Proxmox está em erro ou fenced (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 min | Alerta (`critical`) | 1 h |
| HA manager not reporting | O gerenciador de recursos local HA de um nó parou de escrever seu estado (`dumbmonit_proxmox_ha_lrm_stale`): os guests HA daquele nó não serão reiniciados nem realocados. | > 0 | 5 min | Alerta (`critical`) | 1 h |
| Cluster lost quorum | O cluster Proxmox não tem quórum (`1 - dumbmonit_proxmox_cluster_quorate`): guests não podem mais ser iniciados. | > 0 | 1 min | Alerta (`critical`) | 30 min |
| Proxmox node offline | Um nó do cluster é reportado offline (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 min | Alerta (`critical`) | 30 min |
| Proxmox service down | Um daemon central do Proxmox não está rodando em um nó (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, cada um contado só onde o nó o ativa). | > 0 | 5 min | Alerta (`critical`) | 1 h |
| Node network interface down | Uma interface configurada para iniciar no boot não está ativa em um nó (`dumbmonit_proxmox_node_interface_offline`). Uma interface ativada sob demanda nunca é reportada. | > 0 | 5 min | Aviso (`warning`) | 6 h |
| Proxmox storage almost full | Um storage Proxmox está mais de 85% cheio (`dumbmonit_proxmox_storage_used_percent`); "Disk almost full" assume em 90%. | > 85 % | 15 min | Aviso (`warning`) | 24 h |
| Thin pool almost full | Um pool thin LVM está mais de 90% cheio: todo guest armazenado nele fica somente leitura quando se enche (`dumbmonit_proxmox_node_thinpool_used_percent`); limpa abaixo de 88%. | > 90 % | 15 min | Alerta (`critical`) | 6 h |
| Thin pool metadata almost full | O volume de metadados de um pool thin LVM está mais de 80% cheio (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). É muito menor que o volume de dados e frequentemente satura primeiro, com a mesma consequência. | > 80 % | 15 min | Alerta (`critical`) | 6 h |
| Proxmox disk wearing out | Um SSD ou NVMe de um nó usou mais de 90% de sua vida nominal (`dumbmonit_proxmox_node_disk_wearout_percent`). A notificação lê `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Aviso (`warning`) | 7 d |
| Proxmox disk SMART failure | Um disco de um nó reporta saúde SMART `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 min | Alerta (`critical`) | 24 h |
| Proxmox ZFS pool degraded | Um pool ZFS de um nó não está `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). A notificação lê `pve2 · tank`. | > 0 | 5 min | Alerta (`critical`) | 6 h |
| Backup job failed | O último job vzdump em um nó, ou um job de backup agendado, falhou (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 min | Alerta (`critical`) | 24 h |
| Backup job excludes a disk | Um job de backup agendado pula um disco de um dos guests que cobre (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): o job tem sucesso toda noite com esses dados faltando. Drives de CD-ROM e imagens cloudinit são excluídas por design e nunca contadas. | > 0 | 1 h | Aviso (`warning`) | 7 d |
| Replication failed | Um job de replicação reporta um erro ou uma contagem de falha diferente de zero (`dumbmonit_proxmox_replication_job_error`): a cópia em standby está desatualizada. | > 0 | 10 min | Alerta (`critical`) | 6 h |
| Ceph health error | O Ceph reporta `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 min | Alerta (`critical`) | 30 min |
| Ceph health warning | O Ceph reporta `HEALTH_WARN` por mais de quinze minutos (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 min | Aviso (`warning`) | 6 h |
| Ceph OSD down | Um OSD Ceph está fora do ar há cinco minutos (`1 - dumbmonit_proxmox_ceph_osd_up`). A notificação nomeia o OSD e seu host. | > 0 | 5 min | Alerta (`critical`) | 1 h |
| Ceph OSD nearly full | Um único OSD Ceph está mais de 85% cheio (`dumbmonit_proxmox_ceph_osd_used_percent`): escritas param quando alcança seu limite, qualquer que seja a média do cluster; limpa abaixo de 82%. | > 85 % | 15 min | Aviso (`warning`) | 24 h |
| Ceph pool almost full | Um pool Ceph está mais de 85% cheio (`dumbmonit_proxmox_ceph_pool_used_percent`); limpa abaixo de 82%. | > 85 % | 15 min | Aviso (`warning`) | 24 h |
| Ceph noout flag left on | A flag `noout` do Ceph está definida há duas horas (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): o rebalanceamento está desativado, e o cluster lê saudável enquanto sua redundância diminui. | > 0 | 2 h | Info (`info`) | 24 h |
| Proxmox updates pending | Mais de vinte atualizações de pacote estão pendentes em um nó (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |
| Proxmox security updates pending | Ao menos uma atualização pendente vem de um arquivo de segurança (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Aviso (`warning`) | 7 d |
| Proxmox packages changed | A versão instalada de um pacote Proxmox mudou entre duas sondagens — alguém executou `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, publicado por uma hora com o detalhe em seu rótulo `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Resolve por conta própria uma hora depois. | > 0 | 1 min | Info (`info`) | 24 h |
| Node certificate expiring | Um certificado de nó Proxmox expira em menos de quatorze dias (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 d | 1 h | Aviso (`warning`) | 24 h |

### Proxmox Backup Server

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| PBS datastore almost full | Datastore PBS mais de 90% cheio. | > 90 % | 15 min | Aviso (`warning`), escala depois de 24 h | 6 h |
| PBS datastore filling up | No ritmo atual, o PBS estima o datastore cheio dentro de sete dias. | < 7 d | 1 h | Aviso (`warning`) | 24 h |
| PBS datastore not mounted | Um datastore removível não está montado (`dumbmonit_pbs_datastore_removable_unmounted`): nada pode ser escrito nele. | > 0 | 6 h | Aviso (`warning`) | 24 h |
| PBS backup too old | Nenhum snapshot novo para esse grupo de backup (um por máquina) por mais de dois dias (`dumbmonit_pbs_backup_last_age_seconds`). Edite o limite para máquinas com backup semanal; a notificação nomeia o grupo e datastore. | > 2 d | 1 h | Aviso (`warning`) | 24 h |
| PBS backup verification failed | A verificação do snapshot mais recente desta máquina falhou (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 min | Alerta (`critical`) | 24 h |
| PBS task failed | Pelo menos uma tarefa PBS falhou na janela de revisão (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 min | Aviso (`warning`) | 24 h |
| PBS garbage collection too old | Nenhuma coleta de lixo bem sucedida neste datastore por mais de oito dias (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 d | 1 h | Aviso (`warning`) | 24 h |
| PBS garbage collection failed | A última coleta de lixo em um datastore falhou (`dumbmonit_pbs_gc_last_run_ok < 1`; de `/gc` no PBS 3.3+, da tarefa de GC mais recente antes). | < 1 | 10 min | Aviso (`warning`) | 24 h |
| PBS corrupt chunks found | A coleta de lixo encontrou chunks ilegíveis em um datastore (`dumbmonit_pbs_gc_bad_chunks`): alguns backups não podem mais ser restaurados. | > 0 | 10 min | Alerta (`critical`) | 24 h |
| PBS prune job failed | A última execução de um job de prune PBS falhou: nada é prunado e o datastore continua se enchendo (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). A notificação nomeia o job e datastore. | < 1 | 10 min | Aviso (`warning`) | 24 h |
| PBS verification job failed | A última execução de um job de verificação PBS falhou (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 min | Aviso (`warning`) | 24 h |
| PBS sync job failed | A última execução de um job de sync PBS falhou (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 min | Alerta (`critical`) | 24 h |
| PBS tape backup failed | A última execução de backup em fita falhou (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): a cópia offline não está sendo feita. | < 1 | 10 min | Alerta (`critical`) | 24 h |
| PBS job never ran | Um job está ativado e agendado mas nunca rodou (`dumbmonit_pbs_job_never_run`). | > 0 | 2 d | Aviso (`warning`) | 7 d |
| PBS service down | Um serviço que o Proxmox Backup Server precisa não está rodando (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); nenhum backup pode ser feito enquanto está fora do ar. | < 1 | 5 min | Alerta (`critical`) | 6 h |
| PBS disk SMART failure | Um disco do servidor de backup reporta saúde SMART `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). A notificação lê `/dev/sdb`. | > 0 | 5 min | Alerta (`critical`) | 24 h |
| PBS SSD worn out | Um SSD do servidor de backup usou mais de 90% de sua resistência nominal (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Aviso (`warning`) | 7 d |
| PBS ZFS pool degraded | Um pool ZFS do servidor de backup não está `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 min | Alerta (`critical`) | 6 h |
| PBS certificate expiring | O certificado servido pelo servidor de backup expira em menos de três semanas (`dumbmonit_pbs_node_certificate_expires_seconds`). Precisa da opção `certificates`. | < 21 d | 1 h | Aviso (`warning`) | 48 h |
| PBS restart pending after upgrade | O pacote foi atualizado mas o daemon em execução ainda é o antigo (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 d |
| PBS updates pending | Mais de vinte atualizações de pacote estão pendentes no servidor de backup (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Datacenter Manager

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Federated instance unreachable | A console do datacenter não consegue alcançar um dos clusters Proxmox VE ou servidores de backup que federa (`dumbmonit_pdm_remote_reachable < 1`). A notificação nomeia a instância e seu produto. | < 1 | 10 min | Alerta (`critical`) | 6 h |
| Federated instance behind | Uma instância roda uma versão mais antiga que outra instância do mesmo produto no mesmo acervo (`dumbmonit_pdm_remote_version_behind`). Comparado entre instâncias, nunca contra o que o Proxmox publica. | > 0 | 1 h | Info (`info`) | 7 d |
| Task failed on a federated instance | Pelo menos uma tarefa falhou em uma instância federada na janela de revisão (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 min | Aviso (`warning`) | 24 h |
| Datacenter console disk almost full | O sistema de arquivos raiz da console do datacenter está mais de 90% cheio (`dumbmonit_pdm_node_rootfs_percent`): cheio, a console para de registrar o que coleta. | > 90 % | 15 min | Aviso (`warning`), escala depois de 24 h | 6 h |
| Datacenter console certificate expiring | Um certificado da console do datacenter expira em menos de quatorze dias (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 d | 1 h | Aviso (`warning`) | 24 h |
| Datacenter console updates pending | Mais de vinte atualizações de pacote estão pendentes na console do datacenter (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Mail Gateway

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Mail queue growing | A fila de e-mail deferred cresceu mais de cem mensagens em duas horas (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): o próximo salto provavelmente está recusando e-mail. Crescimento em vez de tamanho, porque um gateway ocupado sempre mantém algumas dezenas de deferred. | > 100 | 15 min | Aviso (`warning`) | 6 h |
| Mail stuck in the queue | Uma mensagem está esperando em uma fila há mais de quatro horas (`dumbmonit_pmg_queue_oldest_age_seconds`). A idade é um limite inferior — o qshape reporta faixas — então a regra dispara tarde em vez de cedo. A notificação nomeia a fila. | > 4 h | 15 min | Alerta (`critical`) | 6 h |
| Mail gateway service stopped | Um serviço que o gateway precisa para aceitar e filtrar e-mail está parado (`dumbmonit_pmg_service_running` para postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Uma unidade não instalada não tem série e não pode disparar. | < 1 | 5 min | Alerta (`critical`) | 6 h |
| Virus signatures out of date | As assinaturas diárias de vírus do ClamAV têm mais de dois dias (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): o freshclam parou de atualizá-las e o gateway continua filtrando com a lista da semana passada. | > 2 d | 1 h | Aviso (`warning`) | 24 h |
| Spam rules out of date | As regras do SpamAssassin têm mais de oito dias (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). Um canal nunca atualizado não tem data nem série. | > 8 d | 1 h | Aviso (`warning`) | 7 d |
| Quarantine filling up | A quarentena de spam recebeu mais de mil mensagens em seis horas (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): uma campanha, ou uma regra que acabou de colocar um domínio legítimo na lixeira. | > 1000 | 30 min | Aviso (`warning`) | 12 h |
| Mail gateway cluster degraded | Um gateway do cluster não está mais sincronizado com os outros (`dumbmonit_pmg_cluster_node_insync`): ele filtra com um banco de dados de regras desatualizado. Um gateway autônomo não tem série. | < 1 | 15 min | Aviso (`warning`) | 6 h |
| Mail gateway certificate expiring | Um certificado do gateway de e-mail expira em menos de quatorze dias (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 d | 1 h | Aviso (`warning`) | 24 h |
| Mail gateway updates pending | Mais de vinte atualizações de pacote estão pendentes no gateway de e-mail (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### MDaemon e SecurityGateway

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| MDaemon mail service down | Um serviço de e-mail observado do servidor MDaemon não responde, ou saúda para recusar o serviço (`dumbmonit_mdaemon_service_up`). Só os serviços listados no dispositivo têm série. A notificação nomeia o serviço e sua porta. | < 1 | 5 min | Alerta (`critical`) | 6 h |
| MDaemon XML API not answering | A API XML parou de responder enquanto os serviços de e-mail ainda respondem (`dumbmonit_mdaemon_api_up`): o Remote Administration provavelmente está parado. Só um dispositivo com uma conta tem a série. | < 1 | 15 min | Aviso (`warning`) | 12 h |
| MDaemon mail queue growing | A fila inbound, local ou remote cresceu mais de cem mensagens em duas horas (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): o MDaemon parou de entregar a partir dela. Lido pelo [agente](../devices/agent.md#windows-performance-counters) no servidor MDaemon, então o alerta está no dispositivo do agente. | > 100 | 15 min | Aviso (`warning`) | 6 h |
| MDaemon Bad queue not empty | Mensagens estão esperando na fila Bad (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): o MDaemon não conseguiu nem entregar nem devolver, e permanecem lá até um administrador liberar ou excluir. | > 0 | 30 min | Aviso (`warning`) | 24 h |
| MDaemon Retry queue high | Mais de cinquenta mensagens esperam na fila Retry (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): servidores remotos continuam recusando e-mail temporariamente. | > 50 | 1 h | Aviso (`warning`) | 6 h |
| SecurityGateway service down | Um serviço observado do gateway não responde, ou saúda para recusar o serviço (`dumbmonit_securitygateway_service_up`). | < 1 | 5 min | Alerta (`critical`) | 6 h |
| SecurityGateway API not answering | A API REST parou de responder enquanto os serviços ainda respondem (`dumbmonit_securitygateway_api_up`). Só um dispositivo com uma chave de API tem a série. | < 1 | 15 min | Aviso (`warning`) | 12 h |
| SecurityGateway delivery queue growing | A fila de entrega cresceu mais de cem mensagens em duas horas (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): o servidor de e-mail detrás do gateway provavelmente está recusando e-mail. Os nomes de contador vêm da API como estão; se sua versão nomeia a fila diferente, ajuste o matcher. | > 100 | 15 min | Aviso (`warning`) | 6 h |

### OPNsense

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Internet gateway down | O dpinger não obtém mais resposta de um gateway (`dumbmonit_opnsense_gateway_up`). Em um firewall multi-WAN o tráfego já se moveu para outro link, silenciosamente. `loss` e `delay` são avisos, não falhas, e não disparam esta regra. | < 1 | 5 min | Alerta (`critical`) | 6 h |
| Gateway losing packets | Um gateway está perdendo mais de um quinto dos pacotes enviados a ele (`dumbmonit_opnsense_gateway_loss_percent`): o link responde, mal. Limpa abaixo de 10%. | > 20 % | 10 min | Aviso (`warning`) | 6 h |
| Gateway slow | O tempo de ida e volta de um gateway permaneceu acima de 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Trezentos milissegundos deixa um link de backup móvel em paz. | > 300 ms | 15 min | Info (`info`) | 12 h |
| Firewall state table filling up | O pf está rastreando mais de 80% das conexões que é permitido rastrear (`dumbmonit_opnsense_pf_states_used_percent`). Além do limite o firewall descarta novas conexões sem outro sintoma. | > 80 % | 10 min | Aviso (`warning`) | 6 h |
| Firewall network buffers exhausted | Mais de 90% dos buffers de rede do FreeBSD estão em uso (`dumbmonit_opnsense_mbuf_used_percent`). Um firewall que esgota para de encaminhar com a CPU ainda ociosa. | > 90 % | 10 min | Aviso (`warning`) | 6 h |
| VPN tunnel down | Um túnel VPN configurado não teve sessão por dez minutos (`dumbmonit_opnsense_vpn_tunnel_up`). Um plugin não instalado, ou uma conexão desativada, não tem série e não pode disparar. | < 1 | 10 min | Aviso (`warning`) | 6 h |
| Firewall resolver stopped | O Unbound parou de responder (`dumbmonit_opnsense_unbound_running`). O firewall ainda roteia; as máquinas detrás dele não resolvem mais. Desligue esta regra se seu firewall resolve com outra coisa. | < 1 | 5 min | Aviso (`warning`) | 6 h |
| Firewall core service stopped | O dpinger, que observa os gateways, ou o configd, que roda tudo mais, está parado (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Só esses dois, porque um serviço desligado de propósito não é uma falha. | < 1 | 5 min | Alerta (`critical`) | 6 h |
| Firewall too hot | Um sensor lê acima de 85 °C (`dumbmonit_opnsense_temperature_celsius`): a caixa sem ventoinha no armário está prestes a começar a reduzir desempenho. Limpa a 78 °C. | > 85 °C | 10 min | Aviso (`warning`) | 6 h |
| Firewall left in CARP maintenance mode | O CARP ficou em modo de manutenção persistente (`dumbmonit_opnsense_carp_maintenance_mode`): este firewall entregou seus endereços virtuais ao seu parceiro e não os retomará por conta própria. | > 0 | 1 h | Info (`info`) | 24 h |
| Firewall reboot pending | Uma atualização está instalada mas precisa de um reinício para ter efeito (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 d |
| Firewall updates pending | O firewall tem pacotes esperando para serem atualizados (`dumbmonit_opnsense_firmware_updates_pending`). É a máquina mais exposta na rede. | > 0 | 1 h | Info (`info`) | 7 d |

### TrueNAS

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| NAS pool degraded | O ZFS não chama mais um pool de saudável (`dumbmonit_truenas_pool_healthy`). Um pool degradado ainda serve seus dados, que é exatamente por que ninguém percebe até o próximo disco falhar. A página do dispositivo nomeia o disco. | < 1 | 5 min | Alerta (`critical`) | 6 h |
| NAS disk errors | Discos de um pool reportaram erros de leitura, escrita ou checksum desde o último `zpool clear` (`dumbmonit_truenas_pool_device_errors`, uma série por tipo). | > 0 | 15 min | Aviso (`warning`) | 24 h |
| NAS pool almost full | Um pool está mais de 85% cheio (`dumbmonit_truenas_pool_used_percent`). O ZFS desacelera bem antes de ficar cheio. Limpa a 80%. | > 85 % | 30 min | Aviso (`warning`), escala depois de 7 d | 24 h |
| NAS scrub found errors | O último scrub de um pool encontrou dados corrompidos (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 min | Alerta (`critical`) | 24 h |
| NAS scrub overdue | Um pool não completou um scrub por mais de 45 dias (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Ausente depois de um resilver, que apaga o registro do último scrub. | > 45 d | 1 h | Info (`info`) | 7 d |
| NAS disk failed its SMART test | Um autoteste SMART no log de um disco falhou (`dumbmonit_truenas_disk_smart_failed`). Um disco nunca testado não tem série. | > 0 | 5 min | Alerta (`critical`) | 24 h |
| NAS disk too hot | Um disco está acima de 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, lido do cache do TrueNAS). Limpa a 50 °C. | > 55 °C | 15 min | Aviso (`warning`) | 6 h |
| NAS dataset near its quota | Um dataset usou mais de 90% de sua cota (`dumbmonit_truenas_dataset_quota_used_percent`): em 100% suas escritas falham enquanto o pool ainda tem espaço. Sem cota, sem série. | > 90 % | 30 min | Aviso (`warning`) | 24 h |
| NAS replication failed | Uma tarefa de replicação ativada está em erro (`dumbmonit_truenas_replication_error`): a cópia do outro lado está ficando mais antiga. | > 0 | 10 min | Aviso (`warning`) | 12 h |
| NAS snapshot task failed | Uma tarefa de snapshot periódico ativada está em erro (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 min | Aviso (`warning`) | 12 h |
| NAS snapshots stale | Uma tarefa de snapshot periódico ativada não rodou por mais de oito dias (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): uma tarefa que para de rodar não falha, fica silenciosa. | > 8 d | 1 h | Aviso (`warning`) | 24 h |
| TrueNAS alert raised | O próprio TrueNAS disparou um alerta de nível ERROR ou acima (`dumbmonit_truenas_alerts`). Sua própria frase está na página do dispositivo. | > 0 | 10 min | Aviso (`warning`) | 12 h |
| NAS service stopped | Um serviço configurado para iniciar com o NAS está parado (`dumbmonit_truenas_service_running`). Um serviço desligado de propósito não tem série. | < 1 | 10 min | Aviso (`warning`) | 6 h |

### Synology DSM

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Synology disk SMART warning | O DSM não reporta mais o status SMART deste disco como `normal` (`dumbmonit_synology_disk_smart_status`: 1 atenção, 2 crítico, palavras desconhecidas contam como 1). | > 0 | 15 min | Aviso (`warning`) | 24 h |
| Synology disk failed | O DSM reporta o disco como travado ou falhado (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 min | Alerta (`critical`) | 6 h |
| Synology disk bad sectors | A contagem de setores defeituosos do disco excede o limite definido no DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 min | Alerta (`critical`) | 24 h |
| Synology disk bad sectors growing | Novos setores ilegíveis apareceram nas últimas 24 horas (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 min | Aviso (`warning`) | 24 h |
| Synology SSD wearing out | Um SSD tem menos de 10% de sua vida nominal restante (`dumbmonit_synology_disk_remaining_life_percent`); limpa acima de 12%. | < 10 % | 1 h | Aviso (`warning`) | 7 d |
| Synology volume degraded | Um volume está degradado ou travado: sua redundância se perdeu (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 min | Alerta (`critical`) | 6 h |
| Synology volume almost full | Um volume está 90% cheio ou mais (`dumbmonit_synology_volume_used_percent`); limpa abaixo de 88%. | ≥ 90 % | 15 min | Aviso (`warning`), escala depois de 24 h | 6 h |
| Synology temperature high | Um disco está acima de 55 °C, ou o DSM disparou seu próprio aviso de temperatura (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); limpa abaixo de 52 °C. | > 55 °C | 15 min | Aviso (`warning`) | 6 h |
| Synology memory high | Uso de memória do NAS acima de 95% (`dumbmonit_synology_memory_usage_percent`); limpa abaixo de 90%. | > 95 % | 15 min | Aviso (`warning`) | 6 h |
| Active Backup task failed | A última execução de uma tarefa Active Backup for Business falhou, ou teve backup só de parte de seus dispositivos (`dumbmonit_abb_task_last_status == bool 0`). A notificação nomeia a tarefa. | > 0 | 10 min | Alerta (`critical`) | 24 h |
| Active Backup too old | Nenhuma execução bem sucedida do Active Backup for Business para esta tarefa por mais de dois dias (`dumbmonit_abb_task_last_success_seconds`). A série não existe para uma tarefa que nunca teve sucesso; "Active Backup task failed" fala então. | > 2 d | 1 h | Aviso (`warning`) | 24 h |
| Active Backup task disabled | A tarefa não tem programação, ou seu backup contínuo está pausado (`dumbmonit_abb_task_enabled == bool 0`): não fará backup de nada até alguém a executar. | > 0 | 1 h | Info (`info`) | 7 d |
| Active Backup device overdue | Um dispositivo passou mais tempo sem uma execução bem sucedida do Active Backup for Business do que seu próprio ritmo permite, dias de descanso excluídos (`dumbmonit_abb_device_overdue`, veja [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 min | Aviso (`warning`) | 24 h |
| Active Backup device failing | As duas últimas ou mais tentativas de um dispositivo falharam (`dumbmonit_abb_device_consecutive_failures`); uma execução cancelada não conta como nenhum dos dois. | ≥ 2 | 10 min | Alerta (`critical`) | 24 h |

### Servidores de logs e métricas

VictoriaMetrics, VictoriaLogs, Loki e Graylog
([dispositivos](../devices/victoriametrics.md)). Contadores passam por
`increase_prometheus`, então um servidor adicionado com um histórico de
dados recusados não dispara em sua primeira medição.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Metrics or log storage read-only | O espaço livre em disco caiu abaixo de `-storage.minFreeDiskSpaceBytes` e o armazenamento recusa toda escrita (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 min | Alerta (`critical`) | 1 h |
| Metrics or log storage almost read-only | Menos de 10% do disco restante antes de somente leitura (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); limpa acima de 12%. | < 10 % | 30 min | Aviso (`warning`) | 24 h |
| Metrics or log server unhealthy | `/health` não responde OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 min | Alerta (`critical`) | 6 h |
| Metrics or log data refused | Amostras ou linhas de log recusadas na última hora: timestamps fora da retenção, rótulos muito longos, limite de série (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 min | Aviso (`warning`) | 24 h |
| VictoriaMetrics slow inserts | Mais de 5% das amostras novas tomaram o caminho lento em 15 minutos: memória insuficiente para as séries ativas (`100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])`); limpa abaixo de 3%. | > 5 % | 30 min | Aviso (`warning`) | 24 h |
| Loki not ready | `/ready` não responde ready (`dumbmonit_loki_ready`). | < 1 | 5 min | Alerta (`critical`) | 6 h |
| Loki refusing log lines | Linhas recusadas na última hora: muito antigas, limite de taxa ou stream, linha muito longa (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 min | Aviso (`warning`) | 24 h |
| Loki cannot flush to storage | Chunks falharam ao alcançar o armazenamento nos últimos 30 minutos; são perdidos se o Loki reiniciar (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 min | Alerta (`critical`) | 1 h |
| Loki write-ahead log disk full | Escritas WAL perdidas por um disco cheio (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 min | Alerta (`critical`) | 1 h |
| Loki request errors | Mais de cinco requisições respondidas com um erro de servidor em 15 minutos (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 min | Aviso (`warning`) | 6 h |
| Graylog not processing | Processamento de mensagens pausado no nó (`dumbmonit_graylog_processing`). | < 1 | 5 min | Alerta (`critical`) | 6 h |
| Graylog search cluster down | OpenSearch ou Elasticsearch vermelho, ou inacessível pelo Graylog (`dumbmonit_graylog_indexer_status`: 2 vermelho, 3 inacessível). | ≥ 2 | 5 min | Alerta (`critical`) | 1 h |
| Graylog search cluster yellow | Shards de réplica não atribuídas por meia hora (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 min | Aviso (`warning`) | 24 h |
| Graylog journal filling | O journal está mais da metade cheio: a saída não acompanha o ritmo (`dumbmonit_graylog_journal_used_percent`); limpa abaixo de 40%. | > 50 % | 10 min | Aviso (`warning`), escala depois de 1 h | 6 h |
| Graylog input failed | Uma entrada falhou ao iniciar (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 min | Aviso (`warning`) | 24 h |
| Graylog indexing failures | Escritas no cluster de busca ou processamento de mensagens falharam na última hora (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 min | Aviso (`warning`) | 24 h |

### Aplicações autoalojadas

Nextcloud, Immich, Paperless-ngx, Jellyfin e Plex
([dispositivos](../devices/nextcloud.md)). Elas raramente caem
completamente: a página abre enquanto sincronização, miniaturas,
importações ou varreduras de biblioteca pararam. Essas regras observam
esses estados; uma aplicação que não responde de forma alguma é Device
unreachable.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Nextcloud stuck in maintenance | O modo de manutenção está ativo (`dumbmonit_nextcloud_maintenance`): ninguém pode entrar ou sincronizar. | > 0 | 30 min | Aviso (`warning`), escala depois de 3 h | 6 h |
| Nextcloud database upgrade pending | O código foi atualizado e o Nextcloud espera por `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 min | Alerta (`critical`) | 1 h |
| Nextcloud update available | A verificação de atualização do Nextcloud encontrou uma versão mais nova (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Nextcloud OPcache full | O OPcache do PHP está sem espaço e as páginas ficam lentas (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 min | Aviso (`warning`) | 24 h |
| Nextcloud free space low | Menos de 5 GiB livres para o diretório de dados (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); limpa acima de 6 GiB. | < 5 GiB | 30 min | Aviso (`warning`) | 24 h |
| Immich jobs stalled | Jobs esperam em uma fila que não está pausada e não executa nada (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). A notificação nomeia a fila. | > 0 | 1 h | Aviso (`warning`) | 6 h |
| Immich queue paused | Uma fila de job está pausada há um dia (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 d |
| Immich storage almost full | O disco que contém a biblioteca está mais de 90% cheio (`dumbmonit_immich_storage_used_percent`); limpa abaixo de 88%. | > 90 % | 30 min | Aviso (`warning`) | 24 h |
| Immich update available | A verificação de versão do Immich conhece uma versão mais nova (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Paperless task queue down | Redis ou Celery em erro (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 é um erro): nada novo é importado. | ≥ 2 | 5 min | Alerta (`critical`) | 1 h |
| Paperless index or sanity check error | O índice de busca, o classificador ou a verificação de integridade (arquivos faltando ou alterados) reporta um erro (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 min | Aviso (`warning`) | 24 h |
| Paperless migrations pending | Migrações de banco de dados restantes para aplicar (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 min | Aviso (`warning`) | 24 h |
| Paperless task failed | Uma tarefa que este usuário pode ver falhou dentro da janela de tarefas com falha e não foi descartada (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 min | Aviso (`warning`) | 24 h |
| Paperless storage almost full | O disco que contém os documentos está mais de 90% cheio (`dumbmonit_paperless_storage_used_percent`); limpa abaixo de 88%. | > 90 % | 30 min | Aviso (`warning`) | 24 h |
| Paperless update available | A verificação de versão do Paperless-ngx conhece uma versão mais nova (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Jellyfin scheduled task failed | Uma tarefa agendada (uma varredura de biblioteca, uma limpeza) falhou ou foi abortada em sua última execução (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 min | Aviso (`warning`) | 24 h |
| Jellyfin plugin broken | Um plugin falhou ao carregar ou não suporta esta versão (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 min | Aviso (`warning`) | 24 h |
| Jellyfin restart pending | O Jellyfin espera um dia por um reinício depois de instalar ou atualizar um plugin (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 d |
| Plex update available | A verificação de atualização do Plex encontrou uma versão mais nova (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Forges autoalojadas

GitLab (self-managed) e Forgejo/Gitea
([dispositivos](../devices/gitlab.md)). Uma instância que não responde de
forma alguma é Device unreachable; essas regras observam as partes que
continuam servindo páginas enquanto silenciosamente ficam atrasadas —
Sidekiq, executores de CI/CD, uma tarefa agendada que parou de rodar.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| GitLab runners all offline | Todo executor CI/CD que o GitLab conhece está offline, e ao menos um está registrado (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 min | Aviso (`warning`) | 6 h |
| GitLab Sidekiq backlog high | Uma fila Sidekiq tem mais de 500 jobs esperando (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): notificações, webhooks e importações ficam atrasados. | > 500 | 15 min | Aviso (`warning`) | 6 h |
| GitLab migrations pending | Migrações de banco de dados de uma atualização restam para aplicar (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 h | Info (`info`) | 24 h |
| GitLab license expiring | A licença Enterprise Edition expira em menos de 30 dias (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 d | 1 h | Aviso (`warning`) | 24 h |
| Forgejo/Gitea runners all offline | Todo executor de Actions está offline, e ao menos um está registrado (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 min | Aviso (`warning`) | 6 h |
| Forgejo/Gitea scheduled task overdue | Uma tarefa agendada (verificação de repositório, limpeza, verificador de atualizações) não rodou em seu próprio intervalo (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 min | Aviso (`warning`) | 24 h |

### Dispositivos clientes

Uma regra, compartilhada por toda integração que rastreia dispositivos se
conectando e fazendo backup — os celulares, tablets e desktops do Immich
([dispositivos](../devices/immich.md)), as máquinas com backup do Proxmox
Backup Server, um por grupo de backup
([dispositivos](../devices/pbs.md)), e os computadores e VMs protegidos do
Veeam, um por objeto com um ponto de restauração
([dispositivos](../devices/veeam.md)), hoje; mais a seguir. Cada integração
calcula o intervalo contra a janela de desatualização do próprio alvo
(`device_stale_days`, 3 dias por padrão) e o publica como
`dumbmonit_client_device_stale_seconds`, rotulado `device`, `type`, `os`,
`user`, `kind` e `signal` (`connection` ou `backup`); esta única regra então
dispara para qualquer um deles, nomeando o dispositivo na notificação. O
Synology Active Backup for Business rastreia seus próprios dispositivos
separadamente, através de um mecanismo consciente de ritmo
(`synology_abb_device_*`) em vez desta família genérica — veja
[Synology](../devices/synology.md).

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Client device stale | Um dispositivo não se conectou nem fez backup dentro da janela de desatualização configurada para este alvo (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 h | Aviso (`warning`) | 24 h |

### Pi-hole

[Pi-hole](../devices/pihole.md) v6, lido através de sua API REST.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Pi-hole blocking disabled | O bloqueio está desligado, pausado ou falhou (`dumbmonit_pihole_blocking_enabled`). A retenção deixa passar uma pausa de cinco minutos da interface web. | < 1 | 15 min | Aviso (`warning`) | 6 h |
| Pi-hole blocklists stale | As listas de bloqueio (gravity) não foram reconstruídas por mais de oito dias (`dumbmonit_pihole_gravity_age_seconds`): a atualização semanal falha. | > 8 d | 1 h | Aviso (`warning`) | 24 h |
| Pi-hole update available | Uma versão mais nova está disponível (`dumbmonit_pihole_updates_available`): a imagem no Docker, core, interface web ou FTL senão. O Pi-hole verifica uma vez por dia. | > 0 | 1 h | Info (`info`) | 7 d |
| Pi-hole diagnosis messages | O Pi-hole lista mensagens em sua página de diagnóstico (`dumbmonit_pihole_messages`): uma lista de bloqueio que falhou ao baixar, um cliente com taxa limitada, um host sobrecarregado. | > 0 | 15 min | Info (`info`) | 24 h |

### AdGuard Home

O filtro DNS ([dispositivo](../devices/adguard.md)). Números de consulta e
bloqueio são somas sobre a própria janela de estatísticas do AdGuard Home,
não contadores: nenhuma regra é construída sobre eles.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| AdGuard Home protection off | Proteção desativada, ou pausada e ainda não voltou (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 min | Aviso (`warning`) | 6 h |
| AdGuard Home DNS not running | A interface web responde mas o servidor DNS está parado (`dumbmonit_adguard_running`). | < 1 | 5 min | Alerta (`critical`) | 1 h |
| AdGuard Home upstream failing | Um servidor DNS upstream falha no próprio teste do AdGuard Home, ou fica silencioso por 4 segundos (`dumbmonit_adguard_upstream_up`, uma série por upstream). | < 1 | 10 min | Aviso (`warning`) | 6 h |
| AdGuard Home filter lists stale | A atualização mais antiga de uma lista de filtro ativada tem mais de três dias (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 dias | 1 h | Aviso (`warning`) | 24 h |
| AdGuard Home update available | Uma versão mais nova é anunciada pela própria verificação de versão do AdGuard Home (`dumbmonit_adguard_update_available`); nunca dispara quando essa verificação está desativada, como na imagem Docker oficial. | > 0 | 1 h | Info (`info`) | 7 dias |

### UPS via NUT

UPS lido de um [servidor NUT](../devices/nut.md). Toda série carrega o
rótulo `ups`, então um alerta nomeia o UPS quando um servidor publica
vários. As flags de status são 0 ou 1 em cada leitura, então uma regra
limpa por conta própria uma vez que a energia da rede elétrica volta. UPS
lidos via SNMP são cobertos pelas regras [UPS](#ups) acima.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| UPS on battery (NUT) | O UPS está alimentando a carga pela bateria (`dumbmonit_nut_ups_on_battery`, a flag `OB` de `ups.status`). | > 0 | 30 s | Alerta (`critical`) | 15 min |
| UPS battery low (NUT) | O UPS reporta bateria baixa, a flag `LB` (`dumbmonit_nut_ups_low_battery`): as máquinas que ele alimenta estão prestes a desligar. | > 0 | 1 min | Alerta (`critical`) | 30 min |
| UPS battery needs replacing (NUT) | O UPS pede para substituir sua bateria, a flag `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Aviso (`warning`) | 7 d |
| UPS load high (NUT) | A carga de trabalho permaneceu acima de 80% (`dumbmonit_nut_ups_load_percent`); limpa abaixo de 75%. | > 80 % | 10 min | Aviso (`warning`) | 24 h |
| UPS runtime short (NUT) | O UPS estima menos de cinco minutos de execução na bateria (`dumbmonit_nut_battery_runtime_seconds`); limpa acima de seis. Com energia da rede, uma bateria desgastada ou um UPS pequeno demais para sua carga. | < 300 s | 5 min | Aviso (`warning`) | 24 h |
| UPS data stale (NUT) | O servidor NUT responde mas não tem dados recentes do UPS: cabo, driver ou UPS silencioso (`dumbmonit_nut_ups_data_stale`). Uma queda passaria sem ser vista nesse meio tempo. | > 0 | 5 min | Aviso (`warning`), escala depois de 1 h | 6 h |

### MikroTik RouterOS

Séries carregam o rótulo `interface` ou `sensor`, então uma notificação
nomeia a porta ou o sensor ([dispositivo](../devices/mikrotik.md)).
Contadores passam por `increase_prometheus`: um roteador adicionado com
anos de erros atrás dele não dispara em sua primeira medição.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| MikroTik CPU high | A CPU do roteador permaneceu acima de 90% (`dumbmonit_mikrotik_cpu_load_percent`): roteamento, o firewall ou um script está saturando-a. Limpa abaixo de 80%. | > 90 % | 15 min | Aviso (`warning`) | 6 h |
| MikroTik memory high | Mais de 90% da memória está em uso (`dumbmonit_mikrotik_memory_used_percent`). Limpa abaixo de 85%. | > 90 % | 15 min | Aviso (`warning`) | 6 h |
| MikroTik storage almost full | O armazenamento flash está mais de 90% cheio (`dumbmonit_mikrotik_storage_used_percent`): mudanças de configuração e atualizações podem falhar. Limpa abaixo de 85%. | > 90 % | 30 min | Aviso (`warning`) | 24 h |
| MikroTik temperature high | Um sensor de temperatura lê acima de 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Limpa a 75 °C. | > 80 °C | 10 min | Aviso (`warning`) | 6 h |
| MikroTik power supply or fan failed | Um sensor `psu*-state` ou `fan*-state` não está `ok` (`dumbmonit_mikrotik_health_ok`). Uma fonte de alimentação redundante que morreu não corta nada, até a segunda morrer também. | < 1 | 5 min | Alerta (`critical`) | 1 h |
| MikroTik interface accumulating errors | Mais de cinquenta erros de recepção e transmissão em uma interface na última hora (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 min | Aviso (`warning`) | 24 h |
| MikroTik link flapping | Uma interface perdeu o link mais de duas vezes em trinta minutos (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Uma porta desconectada para sempre conta uma vez e não dispara. | > 2 | 5 min | Aviso (`warning`) | 1 h |
| MikroTik RouterOS update available | Um RouterOS mais novo está disponível no canal de atualização do roteador (`dumbmonit_mikrotik_update_available`). Só conhecido quando o roteador verifica atualizações ele mesmo: veja o passo opcional da [configuração](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 d |
| MikroTik firmware upgrade pending | O RouterBOOT é mais antigo que o firmware empacotado com o RouterOS instalado (`dumbmonit_mikrotik_firmware_upgrade_pending`); é atualizado só sob pedido, depois no próximo reinício. | > 0 | 1 h | Info (`info`) | 7 d |

### UniFi Network

[Página do dispositivo](../devices/unifi.md). Séries de dispositivo
carregam o nome, modelo e tipo do dispositivo, então a notificação diz
"Living room AP". Um dispositivo esperando adoção não tem série
`device_up` e nunca conta como offline.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| UniFi device offline | Um dispositivo UniFi adotado pelo controlador está offline, isolado ou falhou ao adotar (`dumbmonit_unifi_device_up`). | < 1 | 5 min | Aviso (`warning`), escala depois de 1 h | 6 h |
| UniFi Internet down | O gateway UniFi reporta nenhuma conexão de Internet (`dumbmonit_unifi_internet_up`). Sem um gateway UniFi a série não existe e a regra permanece silenciosa. | < 1 | 3 min | Alerta (`critical`) | 1 h |
| UniFi WAN link down | Um link WAN do gateway UniFi está fora do ar (`dumbmonit_unifi_wan_link_up`): com dois links, a redundância se perde enquanto nada parece quebrado. Uma porta desativada não tem série. | < 1 | 5 min | Aviso (`warning`) | 6 h |
| UniFi gateway CPU high | O processador do gateway UniFi está acima de 90% por quinze minutos (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); limpa abaixo de 80%. | > 90 % | 15 min | Aviso (`warning`) | 6 h |
| UniFi device waiting for adoption | Um dispositivo apareceu na rede e espera ser adotado no UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 min | Info (`info`) | 24 h |
| UniFi firmware update available | Uma atualização de firmware está disponível para dispositivos UniFi (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 d |
| UniFi alarms | O UniFi registrou eventos críticos ou disparou alarmes nas últimas 24 horas (`dumbmonit_unifi_alarms`). Só lido com uma conta View Only. | > 0 | 5 min | Aviso (`warning`) | 24 h |

### Home Assistant

[Página do dispositivo](../devices/homeassistant.md).

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Home Assistant in recovery mode | O Home Assistant iniciou em modo de recuperação: sua configuração não pôde ser carregada e automações não rodam (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 min | Alerta (`critical`) | 1 h |
| Home Assistant not running | O Home Assistant responde mas não terminou de iniciar por dez minutos (`dumbmonit_homeassistant_running`). | < 1 | 10 min | Aviso (`warning`) | 6 h |
| Home Assistant entities went unavailable | Pelo menos cinco entidades mais estão indisponíveis que há uma hora: uma integração, um hub ou um rádio parou (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). O aumento conta, não o total, que uma instalação real nunca tem em zero. | ≥ 5 | 10 min | Aviso (`warning`) | 6 h |
| Home Assistant battery low | Uma bateria reportada pelo Home Assistant está baixa (`dumbmonit_homeassistant_battery_low`, uma série por bateria abaixo do limite do dispositivo, 20% por padrão; seu valor é o nível). | ≥ 0 % | 30 min | Info (`info`) | 3 d |
| Home Assistant update available | O Home Assistant tem atualizações esperando: core, sistema operacional, add-ons ou firmware de dispositivo (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Home Assistant repair to address | O Home Assistant tem um reparo aberto de severidade erro ou crítica (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 min | Aviso (`warning`) | 24 h |

### VMware vSphere

[Página do dispositivo](../devices/vsphere.md). Séries de host carregam
`host`, séries de VM `vm`, séries de datastore `datastore`, então a
notificação nomeia o objeto.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| vSphere host not responding | Um host vSphere não está respondendo ao vCenter (`dumbmonit_vsphere_host_connection_state`: 2 não respondendo). Um host desconectado de propósito (1) não dispara. | ≥ 2 | 3 min | Alerta (`critical`) | 1 h |
| vSphere host health red | O vSphere reporta a saúde geral de um host como vermelha (`dumbmonit_vsphere_host_status == 2`). Cinza significa desconhecido e não dispara. | > 0 | 5 min | Alerta (`critical`) | 1 h |
| vSphere host left in maintenance | Um host vSphere está em modo de manutenção há um dia: suas VMs foram movidas e ele pode ter sido esquecido ali (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| vSphere datastore almost full | Um datastore vSphere está 90% cheio ou mais (`dumbmonit_vsphere_datastore_used_percent`); limpa abaixo de 85%. | ≥ 90 % | 15 min | Aviso (`warning`), escala depois de 24 h | 6 h |
| vSphere datastore inaccessible | Um datastore vSphere está inacessível: as VMs armazenadas nele não podem rodar (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 min | Alerta (`critical`) | 1 h |
| vSphere VM tools not running | As VMware Tools estão instaladas mas não rodando em uma VM ligada: o guest pode estar travado (`dumbmonit_vsphere_vm_tools_status == 2`). VMs sem Tools não disparam. | > 0 | 30 min | Info (`info`) | 24 h |
| vSphere red alarm | O vSphere disparou um alarme vermelho que ninguém reconheceu (`dumbmonit_vsphere_alarm{status="red"}`, uma série por alarme, nomeada após o alarme e o objeto). | > 0 | 5 min | Aviso (`warning`) | 6 h |

### Bancos de dados, message broker e motor de segurança

Redis / Valkey, MongoDB, RabbitMQ e CrowdSec
([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md),
[RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)).
Contadores passam por `increase_prometheus` ou `changes_prometheus`, então
nada dispara na primeira medição de um dispositivo.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Redis memory near maxmemory | Memória usada acima de 90% de `maxmemory` (`dumbmonit_redis_memory_used_percent`); limpa abaixo de 85%. Nenhuma série sem `maxmemory`. | > 90 % | 10 min | Aviso (`warning`) | 6 h |
| Redis refusing connections | Conexões de cliente recusadas nos últimos 15 minutos: `maxclients` atingido (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 min | Alerta (`critical`) | 1 h |
| Redis replication link down | Uma réplica perdeu seu link com a primária (`dumbmonit_redis_master_link_up`). | < 1 | 2 min | Alerta (`critical`) | 1 h |
| Redis replica lagging | Visto da primária, uma réplica não confirma há mais de 30 segundos (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 min | Aviso (`warning`) | 6 h |
| Redis snapshot failed | O último `BGSAVE` falhou; com `stop-writes-on-bgsave-error`, escritas são recusadas (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 min | Alerta (`critical`) | 1 h |
| Redis append-only file write failed | A última escrita AOF falhou (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 min | Alerta (`critical`) | 1 h |
| MongoDB replica set without primary | Nenhum membro é primary: nenhuma escrita é aceita (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 min | Alerta (`critical`) | 1 h |
| MongoDB member unreachable | Um membro do replica set é inacessível pelos outros (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 min | Alerta (`critical`) | 1 h |
| MongoDB replication lag | Uma secondary está mais de um minuto atrás da primary (`dumbmonit_mongodb_replset_member_lag_seconds`); limpa abaixo de 30 s. | > 60 s | 10 min | Aviso (`warning`) | 6 h |
| MongoDB connections near the limit | Mais de 80% das conexões disponíveis em uso (`dumbmonit_mongodb_connections_used_percent`); limpa abaixo de 75%. | > 80 % | 10 min | Aviso (`warning`) | 6 h |
| MongoDB cache under pressure | Mais de 20% do cache WiredTiger está sujo (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); limpa abaixo de 15%. | > 20 % | 10 min | Aviso (`warning`) | 6 h |
| MongoDB internal errors | Assertions regulares na última hora (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 min | Aviso (`warning`) | 24 h |
| RabbitMQ memory or disk alarm | Um nó disparou um alarme de recurso: todo publicador está bloqueado (`dumbmonit_rabbitmq_alarm`, uma série por nó e recurso). | > 0 | 1 min | Alerta (`critical`) | 1 h |
| RabbitMQ node down | Um nó do cluster não está rodando (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 min | Alerta (`critical`) | 1 h |
| RabbitMQ network partition | Um nó vê uma partição de rede (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 min | Alerta (`critical`) | 1 h |
| RabbitMQ queue without consumer | Uma fila mantém mensagens e não tem consumidor (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); o valor é o número de mensagens esperando. | > 0 | 15 min | Aviso (`warning`) | 6 h |
| RabbitMQ queue unavailable | Uma fila não está rodando: fila quorum sem maioria, ou seu nó está fora do ar (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 min | Aviso (`warning`) | 6 h |
| CrowdSec Local API down | A Local API não responde `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 min | Alerta (`critical`) | 1 h |
| CrowdSec bouncer stopped pulling | As requisições de um bouncer não mudaram em 30 minutos (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 min | Aviso (`warning`) | 6 h |
| CrowdSec reads no logs | Nenhuma linha de log lida em seis horas (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 min | Aviso (`warning`) | 24 h |

### Proxies reversos e domínios

Traefik, Caddy, Nginx Proxy Manager e registros de domínio
([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md),
[Nginx Proxy Manager](../devices/npm.md),
[Expiração de domínio](../devices/domain.md)), e a opção *Alertar quando a
resposta mudar* da verificação DNS
([Serviços](../devices/services.md#alerting-when-the-answer-changes)).
Séries carregam `router`, `service`, `server`, `upstream`, `host` ou
`cert`, então a notificação nomeia o que falhou.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Traefik router disabled | Um roteador está desativado por um erro de configuração (serviço ou middleware faltando, regra inválida): seus sites não são servidos (`dumbmonit_traefik_router_status`: 0 ativado, 1 aviso, 2 desativado). | ≥ 2 | 5 min | Aviso (`warning`) | 6 h |
| Traefik backend server down | Um servidor de um serviço com balanceamento de carga falha sua verificação de saúde (`dumbmonit_traefik_server_up`). | < 1 | 5 min | Aviso (`warning`) | 6 h |
| Traefik service without a server | Nenhum servidor de um serviço responde sua verificação de saúde: seus sites recebem 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 min | Alerta (`critical`) | 1 h |
| Traefik certificate not obtained | Roteadores pedem a um resolver de certificado um certificado que nenhum certificado mantido pelo Traefik ainda cobre: o Traefik serve seu certificado autoassinado padrão (`dumbmonit_traefik_resolver_routers_uncovered`, precisa de `/api/certificates` do Traefik). | > 0 | 1 h | Aviso (`warning`) | 24 h |
| Caddy upstream unhealthy | Um backend falha a verificação de saúde ativa ou passiva do Caddy (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 min | Aviso (`warning`) | 6 h |
| Caddy configuration reload failed | O Caddy recusou seu último recarregamento de configuração e ainda roda o anterior (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 min | Aviso (`warning`) | 24 h |
| Nginx Proxy Manager host offline | O nginx recusou a configuração de um host ativado: o host não é servido (`dumbmonit_npm_host_online`). | < 1 | 10 min | Aviso (`warning`) | 6 h |
| Proxy certificate not renewed | Um certificado que o Traefik mantém ou um host Nginx Proxy Manager apresenta expira em menos de 14 dias: a renovação automática começa em 30, então está falhando (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 d | 1 h | Aviso (`warning`) | 24 h |
| Reverse proxy answering 5xx | Mais de 5% das respostas em dez minutos foram 5xx, com pelo menos cinquenta requisições (`increase_prometheus` de `dumbmonit_{traefik,caddy}_requests_5xx_total` sobre `…_requests_total`). Precisa das métricas do proxy. | > 5 % | 10 min | Aviso (`warning`) | 6 h |
| Domain expiring | O domínio expira em menos de 30 dias (`dumbmonit_domain_expiry_days >= 7`). | < 30 d | 1 h | Aviso (`warning`) | 24 h |
| Domain about to expire | O domínio expira em menos de 7 dias, ou já expirou (`dumbmonit_domain_expiry_days`). | < 7 d | 1 h | Alerta (`critical`) | 12 h |
| Domain on hold | O registro colocou o domínio em `clientHold` ou `serverHold`: ele está fora do DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 min | Alerta (`critical`) | 6 h |
| Domain in redemption | O domínio está em `redemptionPeriod`, `pendingDelete` ou `pendingRestore`: já expirou (`dumbmonit_domain_redemption`). | > 0 | 30 min | Alerta (`critical`) | 12 h |
| DNS answer changed | A resposta de uma verificação DNS com *Alertar quando a resposta mudar* difere da anterior (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Resolve por conta própria uma hora depois. | > 0 | 1 min | Aviso (`warning`) | 24 h |

### Mudanças no site

Veja [Mudanças no site](../devices/webchange.md). A página do dispositivo
mostra cada mudança antes e depois.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Website changed | A última verificação de uma página ou site observado encontrou páginas que mudaram, apareceram ou desapareceram (`dumbmonit_webchange_last_check_changes`). Resolve na próxima verificação que não encontra nenhuma. | > 0 | 30 s | Info (`info`) | nenhum |

### Kubernetes

Veja [Kubernetes / k3s](../devices/kubernetes.md). Toda série é rotulada
com o nó, ou o namespace e o pod, workload ou reivindicação envolvida.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Kubernetes node not ready | Um nó não está Ready: seu kubelet não reporta mais, e seus pods serão despejados (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 min | Alerta (`critical`) | 6 h |
| Kubernetes node under pressure | Um nó reporta pressão de memória, disco ou PID (`dumbmonit_k8s_node_pressure`, rótulo `condition`). | > 0 | 5 min | Aviso (`warning`) | 6 h |
| Kubernetes pod crash looping | Um contêiner está em `CrashLoopBackOff` (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m])`: o estado desaparece por um momento a cada tentativa de reinício). | > 0 | 10 min | Alerta (`critical`) | 6 h |
| Kubernetes pod stuck pending | Um pod está Pending há quinze minutos: nenhum nó pode aceitá-lo, ou um volume ou imagem que precisa não chega (`dumbmonit_k8s_pod_pending`). | > 0 | 15 min | Aviso (`warning`) | 6 h |
| Kubernetes pod restarting | Um pod reiniciou mais de cinco vezes na última hora sem estar em `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 min | Aviso (`warning`) | 6 h |
| Kubernetes workload missing replicas | Um Deployment, StatefulSet ou DaemonSet tem menos réplicas disponíveis do que pede (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 min | Aviso (`warning`), escala depois de 1 h | 6 h |
| Kubernetes volume claim pending | Uma PersistentVolumeClaim que um pod precisa, ou que um erro de provisionamento envolve, ainda não está vinculada (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 min | Aviso (`warning`) | 6 h |

### Active Directory

Veja [Active Directory](../devices/activedirectory.md). O controlador de
domínio que o DumbMonit consulta é cobertor por *Device unreachable*; os
outros controladores de domínio do domínio por *Domain controller
unreachable* (rótulo `dc`).

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Domain controller unreachable | Um controlador de domínio do domínio não responde mais em sua porta LDAP (`dumbmonit_ad_dc_reachable == bool 0`). Um nome que não resolve a partir do DumbMonit não produz série e nunca dispara. | > 0 | 5 min | Alerta (`critical`) | 6 h |
| Active Directory bind failed | O controlador de domínio responde mas recusa a conta de serviço: senha expirada, conta desativada ou bloqueada (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 min | Aviso (`warning`) | 24 h |
| Privileged group membership changed | Um membro entrou ou saiu de Domain Admins, Enterprise Admins, Schema Admins, Administrators ou um grupo de operadores, aninhamento incluído (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, rótulo `group`). Resolve por conta própria uma hora depois. | > 0 | 1 min | Aviso (`warning`) | 24 h |
| Active Directory replication failing | O controlador de domínio consultado não consegue replicar de um parceiro (`dumbmonit_ad_replication_consecutive_failures`). Só quando a conta pode ler o status de replicação. | > 0 | 30 min | Aviso (`warning`) | 6 h |
| krbtgt password older than 180 days | A senha krbtgt foi alterada por último há mais de 180 dias (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 dias | 10 min | Info (`info`) | nenhum |

### pfSense

Veja [pfSense](../devices/pfsense.md). Gateways são rotulados `gateway`,
interfaces `interface` e `descr`, serviços `service`.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| pfSense gateway down | Um gateway pfSense está fora do ar: seu endereço de monitor não responde mais, e o tráfego que carrega para ou troca de link (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 min | Alerta (`critical`) | 6 h |
| pfSense gateway losing packets | Um gateway pfSense perde mais de 10% das sondagens para seu endereço de monitor: o link está degradado (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 min | Aviso (`warning`) | 6 h |
| pfSense gateway slow | O tempo de ida e volta para o endereço de monitor de um gateway pfSense permaneceu acima de 500 ms por quinze minutos (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 ms | 15 min | Aviso (`warning`) | 6 h |
| pfSense interface without link | Uma interface pfSense ativada não tem link: cabo, porta de switch ou placa de rede (`dumbmonit_pfsense_interface_down`). | > 0 | 5 min | Aviso (`warning`) | 6 h |
| pfSense service stopped | Um serviço pfSense ativado não está rodando: resolvedor DNS, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 min | Aviso (`warning`) | 6 h |
| pfSense disk nearly full | O disco do pfSense está mais de 90% cheio: logs, relatórios e atualizações de pacote param de ser escritos (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 min | Aviso (`warning`) | 1 d |

### Unraid

Veja [Unraid](../devices/unraid.md). Discos são rotulados `disk` e `role`
(`parity`, `data`, `cache`), contêineres `container`.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Unraid array stopped | O array Unraid não está iniciado: compartilhamentos, contêineres e VMs que vivem nele estão indisponíveis (`dumbmonit_unraid_array_started`). | < 1 | 10 min | Alerta (`critical`) | 6 h |
| Unraid disk disabled or missing | Um disco Unraid está desativado, faltando ou inválido: o array roda degradado, e mais uma falha perde dados (`dumbmonit_unraid_disk_ok`). | < 1 | 5 min | Alerta (`critical`) | 6 h |
| Unraid disk read errors | Um disco Unraid reportou erros de leitura desde que o array iniciou: pode ser desativado na próxima (`dumbmonit_unraid_disk_errors`). | > 0 | 5 min | Aviso (`warning`) | 1 d |
| Unraid disk too hot | Um disco de array ou paridade Unraid permaneceu acima de 55 °C por quinze minutos (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 min | Aviso (`warning`) | 6 h |
| Unraid parity check found errors | A última verificação de paridade Unraid falhou ou encontrou erros de sincronização: a paridade não corresponde mais aos dados (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 min | Aviso (`warning`) | 1 d |
| Unraid parity check overdue | A última verificação de paridade Unraid tem mais de 40 dias: uma verificação mensal foi perdida (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 d | 1 h | Aviso (`warning`) | 1 d |
| Unraid cache nearly full | Um pool de cache Unraid está mais de 90% cheio: escritas recorrem ao array lento, ou falham (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 min | Aviso (`warning`) | 1 d |
| Unraid container stopped | Um contêiner Docker configurado para iniciar com o array não está rodando (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 min | Aviso (`warning`) | 6 h |

### Veeam Backup & Replication

Veja [Veeam Backup & Replication](../devices/veeam.md). Jobs são rotulados
`job` e `type`, repositórios `repository`.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Veeam job failed | A última execução de um job Veeam ativado falhou: o que ele protege não tem novo ponto de restauração (`dumbmonit_veeam_job_failed`). | > 0 | 5 min | Alerta (`critical`) | 1 d |
| Veeam job ended with a warning | A última execução de um job Veeam ativado terminou com um aviso: alguns objetos podem não estar protegidos (`dumbmonit_veeam_job_warning`). | > 0 | 5 min | Aviso (`warning`) | 1 d |
| Veeam repository nearly full | Um repositório de backup Veeam está mais de 90% cheio: os próximos jobs falharão por falta de espaço (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 min | Aviso (`warning`) | 1 d |
| Veeam license expiring | A licença Veeam expira em menos de 30 dias: jobs param no fim do período de tolerância (`dumbmonit_veeam_license_expiry_seconds`). | < 30 d | 1 h | Aviso (`warning`) | 1 d |

### Tailscale

Veja [Tailscale](../devices/tailscale.md). Dispositivos são rotulados
`device`, o primeiro rótulo de seu nome MagicDNS.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Tailscale device offline | Um dispositivo Tailscale que precisa permanecer online (um dispositivo marcado por padrão) está desconectado há mais de 15 minutos (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 min | 5 min | Aviso (`warning`) | 6 h |
| Tailscale node key expiring | A chave de nó de um dispositivo Tailscale expira em menos de 14 dias: ele deixará a tailnet até alguém fazer login de novo (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 d | 1 h | Aviso (`warning`) | 1 d |
| Tailscale device awaiting approval | Um dispositivo entrou na tailnet e espera aprovação: aprove-o se é esperado, remova-o caso contrário (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 min | Aviso (`warning`) | 1 d |
| Tailscale client update | Um dispositivo Tailscale que precisa permanecer online roda um cliente com atualização disponível (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 h | Info (`info`) | 7 d |

### FortiGate

Veja [FortiGate](../devices/fortigate.md). Túneis são rotulados `tunnel`,
interfaces `interface` e `alias`, licenças `license`.

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| FortiGate IPsec tunnel down | Nenhuma fase 2 de um túnel IPsec site-to-site do FortiGate está ativa: o site detrás dele está cortado (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 min | Alerta (`critical`) | 6 h |
| FortiGate interface without link | Uma interface FortiGate ativada com um endereço não tem link: cabo, transceptor ou o dispositivo na frente (`dumbmonit_fortigate_interface_down`). | > 0 | 5 min | Aviso (`warning`) | 6 h |
| FortiGate HA out of sync | Os membros de um cluster HA FortiGate não têm mais o mesmo checksum de configuração: um failover rodaria outra configuração (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 min | Aviso (`warning`) | 6 h |
| FortiGate HA member lost | Um cluster HA FortiGate tem menos membros que nas últimas 24 horas: não há mais standby (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 min | Alerta (`critical`) | 6 h |
| FortiGate memory near conserve mode | A memória do FortiGate permaneceu acima de 85% por dez minutos: a 88% (por padrão) ele entra em modo de conservação e para de inspecionar sessões novas (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 min | Aviso (`warning`) | 6 h |
| FortiGate CPU high | A CPU do FortiGate permaneceu acima de 90% por quinze minutos (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 min | Aviso (`warning`) | 6 h |
| FortiGate licence expiring | Uma licença FortiGuard ou FortiCare do FortiGate expira em menos de 30 dias, ou já expirou (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 d | 1 h | Aviso (`warning`) | 1 d |
| FortiGate firmware update | O FortiGuard oferece uma versão de patch mais nova do branch FortiOS que o FortiGate roda (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Sophos Firewall

Veja [Sophos Firewall](../devices/sophos.md).

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Sophos interface without link | Uma interface Sophos Firewall que está ligada e vinculada a uma zona não tem link (`dumbmonit_sophos_interface_down`). | > 0 | 5 min | Aviso (`warning`) | 6 h |

### Hyper-V

Reportado pelo agente Windows em um host Hyper-V; veja
[Agente](../devices/agent.md#hyper-v-hosts).

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Hyper-V VM health critical | O Hyper-V reporta uma máquina virtual em saúde crítica: um recurso que ela precisa, mais frequentemente espaço em disco, está esgotado (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 min | Alerta (`critical`) | 6 h |
| Hyper-V host CPU high | Os processadores lógicos do host Hyper-V estiveram mais de 90% ocupados por quinze minutos, guests incluídos (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 min | Aviso (`warning`) | 6 h |
| Hyper-V virtual disk errors | Um disco virtual Hyper-V reportou erros de E/S na última hora (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 min | Aviso (`warning`) | 6 h |

### Pontuação de segurança

Calculada pelo servidor a cada 15 minutos para todo dispositivo cujo tipo
tem verificações de segurança, e armazenada como `dumbmonit_security_score`
(0–100). Veja [Pontuação de segurança](../using/security-score.md).

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| Security score dropped | A pontuação caiu 10 pontos ou mais contra seu melhor valor das últimas 24 horas (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): uma verificação começou a falhar. | ≥ 10 pts | 30 min | Aviso (`warning`) | 24 h |
| Security score low | A pontuação está abaixo de 40, nota F (`dumbmonit_security_score`). Aumente ou diminua o limite para corresponder à sua própria barra. | < 40 | 1 h | Info (`info`) | 7 d |

### O próprio DumbMonit

| Regra | O que é | Limite padrão | Retenção | Severidade | Lembrete |
|---|---|---|---|---|---|
| DumbMonit backup did not run | O backup local programado do banco de dados DumbMonit não rodou por mais de dois dias (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 d | 1 h | Aviso (`warning`) | 24 h |

Toda regra integrada se aplica a todo dispositivo e a todo canal ativado.

## Editando uma regra na interface

Na página **Alertas**, a seção *Regras* lista toda regra com sua
severidade, uma marca *Integrada* e, quando relevante, quantos alertas
está disparando agora. Cada regra tem:

- um alternador para **ativar ou desativar**;
- **Editar**, que expande um editor embutido para nome, descrição,
  operador, limite, retenção (`for`), severidade, lembrete (`repeat
  every`) e escalonamento (`escalate after`). A consulta de uma regra
  integrada é mostrada mas não editável; a consulta de suas próprias
  regras é. Regras de linha de base mostram seus parâmetros de detecção
  somente leitura;
- **Excluir**, apenas para suas próprias regras.

## Substituições por dispositivo

Uma regra pode ser ajustada para um dispositivo sem tocar na própria regra:
um limite diferente, um limite de limpeza diferente, ou desligada apenas
para esse dispositivo. O editor de cada regra tem um painel
**Substituições por dispositivo** para adicionar ou remover uma
diretamente.

A ação de um clique **Ignorar** em um alerta (na lista *Precisa de você*
ou na página do dispositivo do alerta) é o mesmo mecanismo, escrito com
`enabled: false`: o dispositivo para de carregar essa regra,
imediatamente, reversivelmente. A página do dispositivo lista o que
ignora, com **Parar de ignorar**; a própria regra mostra *Ignorada em N
dispositivos* em Alertas → Regras, onde o mesmo painel de substituições
por dispositivo a remove. Veja
[Substituições por dispositivo](../reference/api.md#per-device-overrides)
para a API.

## Criando uma regra de limite

Clique em **Nova regra**. O formulário precisa de:

| Campo | Significado |
|---|---|
| Nome | Mostrado em alertas e notificações. |
| Consulta | Uma expressão MetricsQL, avaliada por dispositivo. Exemplo: `dumbmonit_cpu_usage_percent`. |
| Operador | `>`, `>=`, `<` ou `<=`. |
| Limite | O número com o qual o resultado da consulta é comparado. |
| Severidade | Info, Aviso ou Alerta. |
| Reter por | Segundos que a condição precisa durar antes de disparar. Uma retenção diferente de zero absorve uma amostra ruim isolada. |

Toda série retornada pela consulta se torna um alerta potencial, conectado
ao dispositivo nomeado por seu rótulo `target`. Agregue com
`avg by (target, host) (…)` quando você quer um alerta por dispositivo em
vez de um por núcleo ou por interface. A identidade do alerta ignora os
rótulos `host` e `tag_*` sempre que `target` está presente, então renomear
um dispositivo mantém seus alertas; uma série cujo `target` não corresponde
a um dispositivo existente e ativado é ignorada.

## A linguagem de consulta

Consultas são [MetricsQL](https://docs.victoriametrics.com/metricsql/), o
superconjunto PromQL do VictoriaMetrics. As métricas são as séries
`dumbmonit_*` descritas na [referência de métricas](../reference/metrics.md),
e toda série carrega um rótulo `target` (o id do dispositivo) e um rótulo
`host` (seu nome).

Alguns padrões usados pelas regras integradas:

```
# Idade da última amostra: o que "inacessível" significa
time() - tlast_over_time(dumbmonit_up[7d])

# Um valor por dispositivo, qualquer que seja a fonte
avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)

# Uma porcentagem calculada a partir de dois gauges
100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total

# Uma condição booleana: a série só é retornada quando verdadeira
dumbmonit_ups_output_source == 5

# Extrapolação feita pelo VictoriaMetrics
predict_linear(dumbmonit_pbs_datastore_used_percent[6h], 345600) and deriv(dumbmonit_pbs_datastore_used_percent[6h]) > 0

# Disponibilidade de um serviço em trinta dias
avg_over_time(dumbmonit_probe_success[30d])
```

!!! tip "Teste uma consulta primeiro"
    `GET /api/metrics/query?query=…` retorna o que a regra avaliaria (veja
    a [referência da API](../reference/api.md#metrics)). Em
    desenvolvimento, a camada publica o VictoriaMetrics embutido em
    `:8428` do host, com sua interface de consulta.

## Tipos de regra na API

`kind` é `threshold` (compara com um limite), `predict` (mesma comparação,
em uma consulta `predict_linear`; mostrada como uma previsão) ou `anomaly`
(linha de base sazonal; `params` mantém `k`, `alpha`, `mad_floor_abs`,
`mad_floor_rel`, `min_samples`). A interface cria regras `threshold`; os
outros dois tipos podem ser criados através da API.
