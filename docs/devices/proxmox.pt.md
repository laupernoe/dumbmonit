# Proxmox VE {#proxmox-ve}

Hipervisor: nós com os seus daemons e rede, máquinas virtuais,
contentores, armazenamentos, pools LVM thin, cópias de segurança, alta
disponibilidade, snapshots, replicação, Ceph até ao OSD, discos
físicos e pools ZFS, atualizações pendentes, alterações de pacote,
subscrição e certificados. Um dispositivo pode cobrir todo um cluster.

## O painel de nós {#the-nodes-panel}

A página do dispositivo abre num cartão por nó: CPU, memória e sistema
de ficheiros raiz como três medidores, o tempo de atividade e a
versão `pve-manager` instalada nesse nó, depois só o que está mal — os
daemons que não estão em execução, as interfaces declaradas no
arranque que não estão ativas — e a ocupação de cada pool thin LVM e
grupo de volumes. Um nó que deixou de responder mantém o seu cartão e
lê-se *offline*.

Duas linhas sob os medidores, quando o nó as reporta: a fração do
tempo de CPU passado à espera de armazenamento (**I/O wait** — um nó
a 10% de CPU e 30% de I/O wait não tem margem restante, e nenhum
outro número o diz) e a memória que o **KSM** recuperou ao partilhar
páginas idênticas entre convidados, que é como um nó alberga mais do
que a sua RAM.

Duas entradas mais juntam-se à lista «o que está mal», ambas em
palavras:

* **Reboot required** — um kernel mais recente está instalado do que
  o que está em execução. O cartão nomeia ambas as versões, já que o
  nó continua a executar o antigo até ser reiniciado. Isto vem de
  `/nodes/{node}/apt/versions`, que só precisa de `Sys.Audit`;
* **Updates pending** — quantos pacotes Proxmox têm uma versão
  candidata mais recente, da mesma chamada. Não é a mesma coisa que
  `apt/update`, que conta todo pacote pendente do sistema mas precisa
  de `Sys.Modify` e geralmente é recusado.

Quando o cluster reporta um watchdog HA (Proxmox VE 9 e posterior), o
seu estado é indicado junto ao título do painel: *HA fencing armed*
uma vez recursos entregues a HA, *HA fencing standby* enquanto nenhum
está — caso em que nenhum nó será fenced, o que é normal mas vale a
pena saber antes de contar com isso.

Um cluster atualizado nó a nó mostra isso aqui: `/version` responde
para o nó que foi perguntado, por isso cada cartão transporta a sua
própria versão e um atrasado é visível sem abrir uma shell.

Os dados vêm de `GET /api/targets/{id}/proxmox/nodes`, lidos da
última verificação no armazenamento de métricas.

## O painel de convidados {#the-guests-panel}

![O painel Guests: uma linha por VM e contentor, agrupados por nó](../assets/screenshots/device-proxmox-light.png){ loading=lazy }

Sob os nós, o painel **Guests**: uma linha por máquina virtual e
contentor, agrupados por nó, que responde às perguntas que um
administrador faz primeiro — está em execução, está ocupado, o seu
disco está cheio, quando foi feita a última cópia de segurança. Cada
linha mostra:

| Coluna | Fonte | Notas |
|---|---|---|
| Status | palavra + placa: running, stopped, paused, suspended, template | Um modelo é listado mas nunca conta como «stopped». |
| Name, VMID, node | o inventário | |
| CPU | `guest_cpu_percent`, dos núcleos atribuídos ao convidado | Uma barra mais o número de núcleos. |
| Memory | `guest_memory_used_bytes` / `guest_memory_total_bytes` | O tamanho do balão quando a VM tem um. Uma segunda linha, *+N on host*, aparece quando `guest_memory_host_bytes` está significativamente acima do que o convidado vê: um processo QEMU custa ao hipervisor mais do que a memória que entrega, e esse excedente é a sobrecarga de emulação. Abaixo de 64 MiB ou 5% a linha fica fora. |
| Disk | `guest_disk_used_bytes` / `guest_disk_total_bytes` | Contentores: uso real. VMs: tamanho sempre; uso só quando o agente convidado QEMU responde (ver abaixo), caso contrário «size only». |
| Network | `rate(guest_network_in/out_bytes)` ao longo das últimas verificações | |
| Uptime | `guest_uptime_seconds` | |
| Last backup | `backup_last_age_seconds` | Vazio quando nenhuma cópia de segurança é conhecida. |
| HA | `ha_resource_state_info` | Só para convidados sob alta disponibilidade. |
| Lock | `guest_locked` | Um crachá, só enquanto um bloqueio é detido (`backup`, `migrate`, `snapshot`…). |

A lista ordena-se por qualquer coluna, filtra por nó, estado ou pool,
e cada grupo de nó colapsa. Clicar numa linha abre a sua CPU e memória
ao longo do intervalo de tempo da página, junto com o sistema
operativo e os endereços IP vistos de dentro do convidado. Os dados
vêm de `GET /api/targets/{id}/proxmox/guests`, que lê a última
verificação do armazenamento de métricas — nada é pedido ao
hipervisor.

Um bloqueio é normal durante os poucos minutos que uma cópia de
segurança, uma migração ou um snapshot demoram. Deixado por uma
tarefa interrompida, bloqueia toda operação nesse convidado —
incluindo a cópia de segurança da noite seguinte — até `qm unlock`
(ou `pct unlock`) o limpar, por isso «Guest locked» dispara depois de
seis horas.

O inventário vem de `/cluster/resources`, a própria cache do cluster,
por isso os convidados de um nó que deixou de responder continuam
listados — com o estado `unknown` e os seus tamanhos, sem medições em
tempo real — em vez de desaparecer do painel no momento em que quer
olhar. Também custa duas chamadas de API menos por nó do que percorrer
`/nodes/{node}/qemu` e `/nodes/{node}/lxc`, que é para onde a opção
`cluster_resources` recua quando desligada.

### Uso de disco dentro de uma VM: o agente convidado QEMU {#disk-usage-inside-a-vm-the-qemu-guest-agent}

O Proxmox vê o disco de uma VM como um volume opaco: `disk` é sempre 0
para uma VM, só o seu tamanho é conhecido. Para ler o uso de dentro, o
coletor pergunta ao agente convidado QEMU (`agent/get-fsinfo`) por
toda VM em execução cuja configuração ativa o agente (Options → QEMU
Guest Agent). Duas condições:

* `qemu-guest-agent` instalado e em execução dentro da VM (Debian e
  Ubuntu: `apt install qemu-guest-agent`; Windows: a ISO dos drivers
  VirtIO);
* o token detém `VM.GuestAgent.Audit` na VM — `VM.Monitor` antes do
  Proxmox VE 9 (a função `DumbMonit` abaixo concede-o em `/`).

O sistema de ficheiros raiz (`/`, ou `C:\` no Windows; falhando isso,
o maior) dá `guest_disk_used_bytes` e `guest_disk_used_percent`, a
mesma série que um contentor tem, para que a regra «VM or container
disk almost full» cubra ambos. Todo sistema de ficheiros real também
recebe `guest_fs_total/used_bytes` e `guest_fs_used_percent` com uma
etiqueta `mountpoint`; sistemas de ficheiros pseudo (tmpfs, squashfs,
montagens snap) e media sem capacidade são ignorados.
`guest_agent_enabled` diz se a configuração da VM ativa o agente,
`guest_agent_running` se respondeu. Sem o agente ou o direito, a VM
mantém o seu tamanho e simplesmente não tem uso — nunca um «0%»
enganoso.

### O sistema operativo e endereços de um convidado {#the-operating-system-and-addresses-of-a-guest}

O Proxmox só conhece o tipo de SO escrito na configuração de um
convidado (`l26`, `win11`), não o que realmente corre. Com a opção
`guest_os` ativa, o coletor pergunta a cada convidado em execução o
que é: `agent/get-osinfo` e `agent/network-get-interfaces` para uma
VM, `/nodes/{node}/lxc/{vmid}/interfaces` para um contentor, que não
precisa de nada instalado dentro. Isso dá `guest_os_info` (`Debian
GNU/Linux 12 (bookworm)`, o seu id, a sua versão, o kernel) e até
quatro `guest_ip_info` — endereços loopback e link-local IPv6 são
ignorados, e uma máquina com dezenas de interfaces virtuais não
enche a base de dados de séries.

Ambos são perguntados **no máximo uma vez por hora por convidado** e
republicados da memória entretanto: um sistema operativo não muda
entre duas verificações, e um endereço quase nunca muda. Num cluster
de cinquenta máquinas em execução isso é a diferença entre duas
chamadas extra por verificação e duas por hora.

Essas duas chamadas de agente precisam de `VM.GuestAgent.Audit`. O
Proxmox VE 7 não a tinha e usava `VM.Monitor`; o Proxmox VE 9 removeu
`VM.Monitor` e só mantém os privilégios de agente convidado. Sem o
que a sua versão conhece, as duas séries simplesmente ficam ausentes;
nada mais muda.

## O painel Ceph {#the-ceph-panel}

Uma secção **Ceph** aparece na página do dispositivo só quando o
cluster realmente tem Ceph: saúde e as palavras por detrás dela,
capacidade total e usada, quantos OSDs estão up e in, uma tabela de
OSDs (anfitrião, classe de dispositivo, uso, latência de aplicação e
confirmação) e uma tabela de pools (uso, size e min size, grupos de
posicionamento contra o número que o autoscaler escolheria), os
sistemas de ficheiros CephFS, as flags de OSD deixadas definidas e as
verificações de saúde atualmente silenciadas.

Essas últimas duas são as que um painel geralmente esconde. `noout`
definido para uma manutenção e nunca limpo significa que o Ceph não
vai rebalancear um OSD morto para fora de novo, e uma verificação
silenciada já não aparece em `HEALTH_OK` — em ambos os casos o
cluster parece saudável enquanto a sua redundância desaparece
silenciosamente.

`GET /api/targets/{id}/proxmox/ceph` responde `available: false`
quando o cluster não tem Ceph, que é como a interface sabe não
desenhar a secção de todo. O detalhe custa três chamadas num único nó
mais duas à escala do cluster, qualquer que seja o tamanho do
cluster, e precisa da opção `ceph_detail` além de `ceph`.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_proxmox_`.

| Família | Métricas | Etiquetas |
|---|---|---|
| Cluster | `cluster_quorate`, `cluster_nodes`, `cluster_nodes_online`, `cluster_member_online`; `version_info` (+ `version`, `release`, `repoid`) | `cluster`, `node` |
| Inventário do cluster | `cluster_guests_total`, `cluster_guests_running`, `cluster_templates_total` (modelos contados separadamente, nunca como «stopped»); `pool_guests`, um por pool de recursos | `pool` |
| Nós | `node_up`, `node_cpu_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_cpu_iowait_percent` (tempo passado à espera de discos — um nó a 10% de CPU e 30% de iowait não tem margem), `node_ksm_shared_bytes` (memória recuperada por deduplicação de páginas), `node_uptime_seconds`, `node_version_info` | `node` |
| Serviços de nó | `node_service_running`, `node_service_state_info` (+ `state`), `node_services_failed`, `node_core_services_down`; `node_pve_version_info` (+ `version`, `release`) para o nó que respondeu | `node`, `service` |
| Rede de nó | `node_interface_active`, `node_interface_exists`, `node_interface_autostart`, `node_interface_mtu`, `node_interface_offline`; total `node_interfaces_offline` | `node`, `iface`, `type` (`bridge`, `bond`, `vlan`, `eth`…) |
| LVM, thin pools e montagens | `node_lvm_vg_size/free_bytes`, `node_lvm_vg_used_percent`, `node_lvm_vg_physical_volumes` (etiqueta `vg`); `node_thinpool_size_bytes`, `node_thinpool_used_bytes`, `node_thinpool_used_percent`, `node_thinpool_metadata_size_bytes`, `node_thinpool_metadata_used_percent` (etiquetas `vg`, `pool`); `node_directory_mount_info` (+ `path`, `device`, `fstype`), `node_directory_mounts` | `node`, `vg`, `pool` |
| Convidados (VMs e contentores) | `guest_status_info` (+ `status`: `running`, `stopped`, `paused`, `suspended`, `template`), `guest_running` (não para modelos), `guest_cpu_percent` (dos núcleos atribuídos), `guest_cpu_count`, `guest_memory_used/total_bytes`, `guest_memory_host_bytes` (Proxmox VE 9: a memória que a VM ocupa no anfitrião, sobrecarga de emulação incluída), `guest_memory_percent` (do próprio ponto de vista do convidado; no Proxmox VE 9 vem do driver de balão, e uma VM sem um tem bytes mas nenhuma percentagem em vez de um valor acima de 100%), `guest_disk_total_bytes`, `guest_disk_used_bytes` e `guest_disk_used_percent` (contentores; VMs só através do agente convidado), `guest_disk_read/write_bytes`, `guest_network_in/out_bytes` (contadores), `guest_uptime_seconds`; `guest_pool_info` (+ `pool`), `guest_locked` (+ `lock`; a série só existe enquanto o bloqueio é detido, por isso a sua duração lê-se diretamente do gráfico) | `node`, `vmid`, `name`, `type` (`qemu` ou `lxc`) |
| Detalhe de VM (VMs em execução) | `guest_agent_enabled`, `guest_agent_running`, `guest_balloon_bytes`, `guest_memory_guest_free_bytes` (memória livre vista de dentro, driver de balão), `guest_running_qemu_info` (+ `version`: o QEMU com que a VM foi iniciada — depois de uma atualização do hipervisor fica atrás até a VM ser parada e iniciada de novo, o que uma migração ao vivo não faz); por sistema de ficheiros `guest_fs_total_bytes`, `guest_fs_used_bytes`, `guest_fs_used_percent` | `node`, `vmid`, `name`, `type`; sistemas de ficheiros: + `mountpoint`, `fstype` |
| Sistema e endereços do convidado | `guest_os_info` (+ `os`, `os_id`, `os_version`, `kernel`), `guest_ip_info` (+ `iface`, `ip`; no máximo quatro endereços roteáveis por convidado). Atualizado uma vez por hora | `node`, `vmid`, `name`, `type` |
| Cartões de rede do convidado | `guest_netdev_in_bytes`, `guest_netdev_out_bytes` (contadores), um par por interface virtual onde `guest_network_in/out_bytes` só dá o total do convidado | `node`, `vmid`, `dev` (`tap100i0`, `veth200i0`) |
| Armazenamentos | `storage_active`, `storage_enabled`, `storage_used/total/avail_bytes`, `storage_used_percent` (a capacidade só é publicada quando o Proxmox reporta uma: um armazenamento inativo, ou um datastore PBS cujo tamanho o PVE não lê, não recebe nenhum em vez de zero bytes) | `node`, `storage`, `type` (`dir`, `lvmthin`, `zfspool`, `nfs`, `pbs`…), `shared` |
| Cópias de segurança (por convidado) | `backup_present`, `backup_last_timestamp_seconds`, `backup_last_age_seconds`, `backup_last_size_bytes`, `backup_count`; totais `backup_guests_total`, `backup_guests_without_backup` | `vmid`, `name`, `node`, `type` |
| Cópias de segurança (execuções vzdump, por nó) | `backup_job_runs`, `backup_job_failures`, `backup_job_last_ok`, `backup_job_last_timestamp_seconds`, `backup_job_last_age_seconds`, `backup_job_last_duration_seconds` | `node` |
| Tarefas de cópia de segurança (agendadas) | `backup_job_enabled`, `backup_job_next_run_seconds`, `backup_job_last_ok`, `backup_job_last_run_age_seconds` (as duas últimas só quando uma tarefa `vzdump` transporta o id da tarefa, PVE 7.2+); totais `backup_jobs_total`, `backup_guests_not_covered`; `backup_covered` por convidado (0 para um convidado que nenhuma tarefa cobre) | `job`, `storage`, `schedule`; `backup_covered`: `vmid`, `name`, `node`, `type` |
| Cobertura de tarefa de cópia de segurança | `backup_job_guests`, `backup_job_volumes_included`, `backup_job_volumes_excluded`; `backup_job_guest_excluded_volumes` (+ `vmid`, `name`, `type`, `reason`), publicada só para uma exclusão *inesperada* | `job` |
| HA | `ha_quorum_ok`, `ha_fencing_armed` (+ `state`: Proxmox VE 9 reporta se o watchdog está `armed` ou em `standby` — em standby nada será fenced), `ha_master_active`, `ha_lrm_active` (`node`), `ha_resources_total`; por recurso `ha_resource_started`, `ha_resource_error` (1 no estado `error`, `fence` ou `recovery`), `ha_resource_state_info` (+ `state`) | `sid`, `node`, `vmid`, `type` |
| Gestor HA | `ha_node_online`, `ha_node_status_info` (+ `status`: `online`, `fence`, `gone`, `maintenance`), `ha_master_info` (o nó que detém o master CRM), `ha_manager_age_seconds`; por nó `ha_lrm_mode_info` (+ `mode`, `state`), `ha_lrm_age_seconds`, `ha_lrm_stale` (1 quando o LRM não escreveu há mais de dois minutos) | `node` |
| Snapshots | `guest_snapshot_count`, `guest_snapshot_oldest_age_seconds`, `guest_snapshot_newest_age_seconds` (idades só quando contagem > 0); `guest_snapshot_guests_skipped` quando `max_snapshot_guests` é excedido | `vmid`, `name`, `node`, `type` |
| Replicação | `replication_job_enabled`, `replication_job_error` (mensagem de erro ou `fail_count` > 0), `replication_job_fail_count`, `replication_job_last_sync_age_seconds`, `replication_job_next_sync_seconds`, `replication_job_duration_seconds`; `replication_jobs_total` | `job`, `vmid`, `node` (origem), `to_node` |
| Ceph | `ceph_health` (0 OK, 1 WARN, 2 ERR, 3 desconhecido), `ceph_health_info` (+ `status`), `ceph_osds_total/up/in`, `ceph_bytes_total/used`, `ceph_used_percent`, `ceph_pgs_total`, `ceph_mons_total` | |
| Ceph OSDs | `ceph_osd_up`, `ceph_osd_in`, `ceph_osd_used_percent`, `ceph_osd_used_bytes`, `ceph_osd_total_bytes`, `ceph_osd_apply_latency_ms`, `ceph_osd_commit_latency_ms`, `ceph_osd_reweight`, `ceph_osd_crush_weight`; totais `ceph_osds_down`, `ceph_osds_out` | `osd`, `host`, `device_class` |
| Pools Ceph e CephFS | `ceph_pool_used_percent`, `ceph_pool_used_bytes`, `ceph_pool_size`, `ceph_pool_min_size`, `ceph_pool_pg_num`, `ceph_pool_pg_num_optimal` (o que o autoscaler escolheria), `ceph_pool_info` (+ `type`, `crush_rule`, `autoscale`), `ceph_pools_total`; `ceph_fs_info` (+ `metadata_pool`, `data_pool`), `ceph_fs_total` | `pool`; CephFS: `name` |
| Flags e silêncios Ceph | `ceph_flag` (1 enquanto a flag está definida), `ceph_flags_set`; `ceph_health_mute_info` (+ `sticky`), `ceph_health_mutes` | `flag`; silêncios: `code` |
| Discos físicos | `node_disk_size_bytes`, `node_disk_smart_failed` (1 quando a saúde SMART é `FAILED`; ausente quando o disco reporta `UNKNOWN`), `node_disk_health_info` (+ `health`, `used`: `LVM`, `ZFS`, `partitions`…), `node_disk_wearout_percent` (SSD e NVMe: 0 nova, 100 fim da vida nominal), `node_disk_temperature_celsius` (de `disks/smart`: atributo ATA 194/190, ou a linha de temperatura de um relatório de texto — `Temperature:` para NVMe, `Current Drive Temperature:` para SAS) | `node`, `disk` (`/dev/sda`, `/dev/nvme0n1`), `model`, `type` (`ssd`, `hdd`, `nvme`) |
| Pools ZFS | `node_zfs_pool_degraded` (1 a menos que `ONLINE`), `node_zfs_pool_health_info` (+ `health`), `node_zfs_pool_size/alloc/free_bytes`, `node_zfs_pool_used_percent`, `node_zfs_pool_fragmentation_percent` | `node`, `pool` |
| Atualizações | `node_updates_pending`, `node_updates_security_pending` (pacotes cuja origem, etiqueta de repositório, suite, secção ou URL de changelog nomeia um arquivo de segurança, como o `bookworm-security` do Debian). Sem `Sys.Modify` essas duas não existem; `node_pve_packages_upgradable` continua a contar os pacotes Proxmox cuja versão candidata difere da instalada, e `node_reboot_required` (+ `running`, `installed`) diz que um kernel mais recente está instalado mas ainda não arrancado — ambas lidas de `apt/versions`, que só precisa de `Sys.Audit` | `node` |
| Alterações de pacote | `node_packages_changed`: 0 normalmente; durante uma hora depois de uma verificação ver a versão instalada de um pacote Proxmox mudar (`apt/versions`), o número de pacotes que mudaram, com uma etiqueta `changes` como `pve-manager 8.2.4→8.2.7, proxmox-kernel-6.8 6.8.8-2→6.8.12-2` | `node`, `changes` |
| Subscrição e repositórios | `node_subscription_active`, `node_subscription_info` (+ `status`: `active`, `notfound`, `expired`…, `level`, `next_due`); `node_repository_errors` (ficheiros de origem ilegíveis), `node_repository_warnings` (próprios avisos do Proxmox: repositório enterprise sem subscrição, repositório de teste), `node_repository_enabled` (+ `repo`: `enterprise`, `no-subscription`, `test`, `ceph-*`) | `node` |
| Certificados | `node_certificate_expiry_days` (negativo depois de expirado) | `node`, `filename`, `subject` |
| Fluxo RRD (desligado por predefinição) | `node_rrd_<metric>`, `guest_rrd_<metric>`, `storage_rrd_<metric>` — toda medição que os próprios servidores de métricas do Proxmox recebem, sob o seu próprio nome (ver `metrics_export` abaixo) | `node`; convidados: `vmid`, `type`; armazenamentos: `node`, `storage` |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

A idade da cópia de segurança por máquina responde à falha mais cara
de um homelab: a cópia de segurança que não corre há semanas,
descoberta no dia do restauro. É calculada a partir do registo de
tarefas `vzdump` (sobre `backup_lookback_days`) e, se
`scan_backup_storage` estiver ativo, dos arquivos encontrados nos
armazenamentos de cópia de segurança.

As tarefas agendadas vêm de Datacenter → Backup: `backup_covered` é 0
para todo convidado que o próprio Proxmox lista como não cobrido por
nenhuma tarefa, por isso uma máquina criada ontem aparece antes da
sua primeira cópia de segurança perdida em vez de depois.

Uma tarefa que cobre um convidado não necessariamente escreve todos
os seus discos. Um volume marcado `backup=0` é ignorado todas as
noites, a tarefa continua a reportar sucesso, e ninguém descobre até
um restauro. `backup_job_guest_excluded_volumes` nomeia a tarefa, o
convidado e o próprio motivo do Proxmox para a exclusão — mas só
quando a exclusão é uma surpresa: uma unidade de CD-ROM, uma imagem
cloudinit ou uma entrada que não é de todo um volume é excluída por
desenho, contada em `backup_job_volumes_excluded` e nunca reportada
por convidado. Sem essa distinção toda máquina com uma unidade
virtual geraria um alerta.

Os daemons de um nó valem a pena vigiar por uma razão: `pvestatd`
parado não quebra nada visivelmente — a interface web continua a
mostrar os números do momento em que morreu, e tudo parece normal.
`node_core_services_down` só conta `pve-cluster`, `pvedaemon`,
`pveproxy`, `pvestatd`, `pve-firewall`, `corosync` e `watchdog-mux`, e
só os que o próprio nó declara como ativados, por isso um nó autónomo
sem corosync ou HA não conta nada em falta.

Uma interface conta como offline quando está definida para iniciar no
arranque e não está ativa (ou já não existe de todo). Uma que é
ativada a pedido, sem autostart, nunca é reportada: a sua ausência é
normal. Uma bridge que falha ao voltar depois de um reinício corta
todo convidado ligado a ela enquanto cada um desses convidados corre
perfeitamente, e nada mais o diz.

Um pool thin LVM que se enche torna todo convidado armazenado nele só
de leitura de imediato, e eliminar ficheiros dentro dos convidados não
devolve o espaço. Os seus metadados vivem num volume próprio, muito
mais pequeno, e frequentemente saturam primeiro, com exatamente a
mesma consequência — por isso os dois níveis de ocupação são séries
separadas e regras separadas.

As alterações de pacote são detetadas pelo próprio coletor: lembra as
versões instaladas da verificação anterior (por dispositivo e nó, em
memória — um reinício do servidor volta a aprender sem reportar). Só
um pacote presente em ambas as verificações com uma versão diferente
conta; um pacote recém-instalado ou removido não.

Uma falha parcial (um nó em baixo, um armazenamento lento) continua a
ser uma verificação bem-sucedida: o que pôde ser lido é escrito e a
falha é contada em `scrape_errors`. As funcionalidades simplesmente
ausentes ficam silenciosas: sem Ceph, sem tarefa de replicação, sem
pool ZFS, sem agente convidado, ou sem direito a listar atualizações
não produzem nenhuma série e nenhum erro. O mesmo se aplica a tudo
acima: um privilégio em falta (403), um endpoint que um Proxmox mais
antigo não tem (404 ou 501), uma funcionalidade não instalada (Ceph
responde 500) não produzem nenhum erro e nenhuma métrica de todo —
nunca um zero a fingir ser uma medição.

As temperaturas da placa não estão disponíveis: a API do Proxmox VE
não expõe sensores, só a própria temperatura SMART dos discos.
Instale o agente DumbMonit no nó se precisar de mais.

As regras incorporadas que se aplicam: Device unreachable, High CPU,
Disk almost full, Filesystem almost full (forecast), Backup too old,
Unusual CPU, VM or container stopped, VM or container CPU high, VM or
container memory high, VM or container disk almost full, HA resource
in error, Cluster lost quorum, Node offline, Storage almost full,
Backup job failed, Old snapshot, Replication failed, Ceph health
error, Ceph health warning, Updates pending, Security updates
pending, Proxmox packages changed, Proxmox disk wearing out, Proxmox
disk SMART failure, Proxmox ZFS pool degraded, Node certificate
expiring, Thin pool almost full, Thin pool metadata almost full,
Proxmox service down, Node network interface down, Ceph OSD down,
Ceph OSD nearly full, Ceph pool almost full, Ceph noout flag left on,
HA manager not reporting, Backup job excludes a disk, Guest locked.

### O fluxo RRD completo {#the-full-rrd-stream}

`/cluster/metrics/export` é o endpoint que os próprios servidores de
métricas do Proxmox (InfluxDB, Graphite) consomem: tudo o que o
`pvestatd` mede, para todo o cluster, numa chamada. Duas coisas
justificam tê-lo.

* Transporta medições que nada mais expõe — em particular a
  informação de pressão por nó (PSI): quanto tempo as tarefas passam
  à espera da CPU, do disco ou da memória. É o número que explica
  «tudo está lento» quando a CPU e a memória ambas parecem bem.
* É lido com `history=1` a partir do último ponto já visto, por isso
  todo ponto entre duas verificações é ingerido na sua própria
  data-hora. Um dispositivo verificado a cada cinco minutos recebe os
  seus cinco pontos de um minuto em vez de um único valor atual.

Está **desligado por predefinição**, e essa é uma troca deliberada.
Essas séries repetem parcialmente as que o resto da coleta já produz
sob nomes melhores, por isso ativar `metrics_export` adiciona um
grande número de séries para guardar e vasculhar para um detalhe que
a maioria das instalações nunca olha. Os nomes são publicados
exatamente como o Proxmox os envia (`node_rrd_pressure_cpu_some_avg10`,
`guest_rrd_netin`…) em vez de traduzidos: o Proxmox adiciona novos em
cada versão, e uma tabela de tradução escrita aqui estaria errada na
seguinte. Precisa de `Sys.Audit` em `/` — que a função `DumbMonit` já
tem quando foi concedida em `/` como abaixo.

## O que preparar no Proxmox {#what-to-prepare-in-proxmox}

Os passos abaixo são os que o aviso junto ao formulário mostra. O
princípio: um utilizador reservado para monitorização, uma função que
só pode ler, um token — nunca a conta com que inicia sessão.

1. Abra uma shell em qualquer nó (na interface web: selecione o nó, depois Shell; ou SSH) e crie um utilizador reservado para monitorização. Não precisa de senha: o token é que inicia sessão.

    ```
    pveum user add dumbmonit@pve --comment "DumbMonit monitoring"
    ```

2. Crie uma função só com os privilégios que o coletor usa: Sys.Audit (nós, cluster, HA, discos e SMART, ZFS, certificados, versões de pacote, subscrição), Datastore.Audit (armazenamentos e arquivos de cópia de segurança), VM.Audit (VMs, contentores, snapshots) e VM.GuestAgent.Audit (o que o agente convidado QEMU lê dentro de uma VM: uso de disco, sistema operativo, endereços IP). Nenhum deles pode alterar nada.

    ```
    pveum role add DumbMonit --privs "Datastore.Audit Sys.Audit VM.Audit VM.GuestAgent.Audit"
    ```

3. Dê ao utilizador essa função em todo o cluster.

    ```
    pveum aclmod / -user dumbmonit@pve -role DumbMonit
    ```

4. Crie o token de API do utilizador. A separação de privilégios está desligada, por isso o token simplesmente herda os direitos do utilizador.

    ```
    pveum user token add dumbmonit@pve monitor --privsep 0
    ```

5. O comando imprime uma tabela com full-tokenid e value. Copie full-tokenid para o campo Token ID do DumbMonit e value (o UUID) para o seu campo Secret. O segredo é mostrado uma vez: se se perder, remova o token e crie um novo.

    ```
    dumbmonit@pve!monitor
    ```

6. Opcional, para contar atualizações pendentes e correções de segurança pendentes: o Proxmox protege essa lista com Sys.Modify. Conceda-o só em /nodes; sem ele o coletor ignora a lista silenciosamente.

    ```
    pveum role add DumbMonitUpdates --privs Sys.Modify
    pveum aclmod /nodes -user dumbmonit@pve -role DumbMonitUpdates
    ```

7. Num Proxmox mais antigo, VM.GuestAgent.Audit pode ainda não existir e o comando acima é recusado com invalid privilege: as chamadas de agente convidado são então cobertas por VM.Monitor, que o Proxmox VE 9 removeu por sua vez. Use o que a sua versão conhece.

    ```
    pveum role add DumbMonit --privs "Datastore.Audit Sys.Audit VM.Audit VM.Monitor"
    ```

8. Prefere a interface web? Os mesmos passos vivem sob Datacenter → Permissions: Users, Roles, Add → User Permission, depois API Tokens com «Privilege Separation» desmarcado.
9. No DumbMonit, introduza o endereço de qualquer nó (porta 8006 por predefinição).

!!! warning
    Não reutilize a conta com que inicia sessão: um token exposto controlaria então todo o cluster. A função DumbMonit acima só pode ler. O Proxmox também usa um certificado autoassinado por predefinição: se a ligação for recusada por esse motivo, marque «Accept an unverifiable certificate» nas opções.

Documentação do fornecedor: [Gestão de utilizadores do Proxmox VE](https://pve.proxmox.com/wiki/User_Management).

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de API (recomendado) | **Token ID**: utilizador, realm e nome do token como o Proxmox os mostra, `dumbmonit@pve!monitor`. **Secret**: o UUID mostrado uma vez quando o token foi criado. Sem expiração, nenhuma sessão aberta no hipervisor. |
| Nome de utilizador / senha | `user@realm` e a senha. Um bilhete é obtido, posto em cache e renovado dez minutos antes da sua expiração de duas horas. |

A API aceita o token em duas partes (`token_id` + `secret`) ou, como
versões mais antigas faziam, como uma única cadeia `token` =
`user@realm!name=secret`; ambas são guardadas da mesma forma. Um
token completo colado em Token ID é dividido pelo formulário.

A função `DumbMonit` acima transporta o que `PVEAuditor` concede para
os caminhos que o coletor lê, mais o privilégio de agente convidado: o
coletor só faz `GET`. Por endpoint: `Sys.Audit` para `/version`,
`/cluster/*`, `/nodes/{node}/status`, `services`, `version`,
`network`, `netstat`, `tasks`, `replication`, `certificates/info`,
`disks/list`, `disks/smart`, `disks/zfs`, `apt/versions`,
`apt/repositories` e `subscription`; `Datastore.Audit` para `storage`
e `storage/{id}/content`; `VM.Audit` para `qemu`, `lxc`, `snapshot`,
`status/current` e as `interfaces` de um contentor;
`VM.GuestAgent.Audit` — ou `VM.Monitor` antes do Proxmox VE 9 — para
`agent/get-fsinfo`.

Dois caminhos pedem um pouco mais do que o resto, e ambos são
cobertos pela função como concedida no passo 3 — em `/`, não em
`/nodes`:

* `Sys.Audit` **em `/`** para o inventário LVM (`disks/lvm`,
  `disks/lvmthin`, `disks/directory`) e para
  `/cluster/metrics/export`, o fluxo RRD opcional. Concedido só em
  `/nodes`, as séries de thin pool e as séries `*_rrd_*` ficam
  silenciosamente ausentes.
* `VM.GuestAgent.Audit` para as duas chamadas de agente convidado que
  lêem o sistema operativo e endereços de uma VM
  (`agent/get-osinfo`, `agent/network-get-interfaces`). O Proxmox VE
  7 não tinha esse privilégio e usava `VM.Monitor`; o Proxmox VE 9
  removeu `VM.Monitor`. Sem o que a sua versão conhece,
  `guest_os_info` e `guest_ip_info` simplesmente não são publicados
  para VMs; os contentores, que não precisam de agente, mantêm os
  seus endereços.

A única coisa que a função não cobre de todo é a lista de
atualizações pendentes (`apt/update`), que o Proxmox protege com
`Sys.Modify` em `/nodes` — opcional, como o passo 6 explica; sem ele o
coletor ignora `node_updates_pending` e `node_updates_security_pending`
silenciosamente.

(Use `-token 'dumbmonit@pve!monitor'` em vez de `-user` nos comandos
`aclmod` para um token criado com «Privilege Separation» marcado.)

Endereço: `10.0.0.10`, `pve.lan`, `pve.lan:8006`, `[fd00::1]` ou um
URL completo `https://pve.example.net`. O esquema `https` e a porta
8006 são adicionados se estiverem em falta.

## Opções {#options}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | API port | `8006` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | O Proxmox vem com um certificado autoassinado por predefinição: ative isto se a ligação for recusada por esse motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `10` | Tempo permitido para cada chamada de API, de 1 a 120. |
| `backup_lookback_days` | Backup lookback (days) | `31` | Tarefas de cópia de segurança mais antigas não são examinadas, de 1 a 3650. |
| `scan_backup_storage` | Inventory backup archives | `true` | Analisa os armazenamentos de cópia de segurança para datar a última cópia de segurança de cada máquina. Desative se o armazenamento é lento a responder. |
| `nodes` | Monitored nodes | *(vazio)* | Nomes dos nós a monitorizar, separados por vírgulas. Vazio: todo nó do cluster. |
| `cluster_resources` | Read the cluster inventory | `true` | Uma chamada a `/cluster/resources` lista todo nó, convidado, armazenamento e pool. Mantenha-a ativa: é a única fonte que ainda vê os convidados de um nó que deixou de responder, e poupa duas chamadas por nó. Desligada, o coletor percorre `/nodes/{node}/qemu` e `/nodes/{node}/lxc` em vez disso, e perde as pools e bloqueios. |
| `services` | Watch node services | `true` | `/nodes/{node}/services` e `version`: o estado dos daemons Proxmox de cada nó e a versão instalada nele. Um `pvestatd` parado deixa todo o cluster a mostrar números congelados. |
| `network` | Watch node networking | `true` | `/nodes/{node}/network` e `netstat`: bridges, bonds e VLANs com o seu estado de ligação, mais os contadores de tráfego de cada cartão de rede de convidado. |
| `lvm` | Watch LVM and thin pools | `true` | `disks/lvm`, `disks/lvmthin` e `disks/directory`: grupos de volumes, thin pools (ocupação de dados *e* metadados) e montagens de diretório geridas pelo PVE. Precisa de `Sys.Audit` em `/`; silencioso caso contrário. |
| `ha` | High availability | `true` | Lê `/cluster/ha/status/current` e `manager_status`: quórum, master CRM, a própria vista do gestor de cada nó, todo LRM com a idade do seu último relatório, e todo recurso HA. |
| `backup_jobs` | Scheduled backup jobs | `true` | Lê as tarefas de Datacenter → Backup e os convidados que nenhuma tarefa cobre. |
| `scan_snapshots` | Inventory snapshots | `true` | Uma chamada por convidado (quatro em curso por nó). Desative em clusters muito grandes. |
| `max_snapshot_guests` | Snapshot inventory limit | `200` | Convidados além deste número são ignorados a cada verificação e contados em `guest_snapshot_guests_skipped`, de 1 a 10000. |
| `replication` | Replication jobs | `true` | Estado de replicação de armazenamento por nó. Silencioso quando o nó não tem nenhuma tarefa. |
| `ceph` | Ceph health | `true` | Lê `/cluster/ceph/status`. Silencioso quando o Ceph não está instalado. |
| `ceph_detail` | Watch Ceph in detail | `true` | Estado, uso e latência por OSD, uso por pool, CephFS, flags de OSD e verificações de saúde silenciadas. Três chamadas num nó mais duas à escala do cluster, qualquer que seja o tamanho do cluster. Precisa de `ceph` ativo; silencioso sem Ceph. |
| `backup_volumes` | Check what backup jobs include | `true` | Para cada tarefa agendada, quais convidados cobre e quais dos seus discos realmente escreve. Apanha uma tarefa que tem sucesso todas as noites enquanto ignora um disco de dados. Uma chamada por tarefa. |
| `guest_os` | Read guest OS and addresses | `true` | Sistema operativo e endereços IP vistos de dentro de cada convidado em execução (agente convidado QEMU para VMs, o namespace do contentor para contentores). Atualizado uma vez por hora por convidado, não a cada verificação. Precisa de `VM.GuestAgent.Audit` no Proxmox VE 8. |
| `metrics_export` | Ingest the full RRD metric stream | `false` | Lê `/cluster/metrics/export`, o fluxo que os próprios servidores de métricas do Proxmox consomem, incluindo pressão por nó (PSI), e todo ponto desde a última verificação em vez de só o atual. Desligado por predefinição: as séries `*_rrd_*` que adiciona repetem parcialmente as já coletadas. Precisa de `Sys.Audit` em `/`. |
| `updates` | Pending updates | `true` | Conta os pacotes que `apt/update` lista. Precisa de `Sys.Modify` em `/nodes` (ver acima); silencioso caso contrário. |
| `certificates` | Certificate expiry | `true` | Dias restantes em cada certificado de nó (`pve-ssl.pem`, `pveproxy-ssl.pem`, `pve-root-ca.pem`). |
| `guest_agent` | Ask the QEMU guest agent | `true` | Para cada VM em execução: `status/current` (balão, flag de agente) e, quando o agente está ativado, `agent/get-fsinfo` (precisa de `VM.GuestAgent.Audit`). Quatro chamadas em curso por nó. Silencioso quando o agente está ausente. |
| `disks` | Watch physical disks | `true` | `disks/list` mais uma chamada `disks/smart` por disco: saúde, desgaste, temperatura. |
| `zfs` | Watch ZFS pools | `true` | `disks/zfs`: saúde, capacidade, fragmentação. Silencioso sem ZFS. |
| `packages` | Detect package changes | `true` | `apt/versions` comparado com a verificação anterior; uma alteração é reportada durante uma hora em `node_packages_changed`. |
| `subscription` | Watch subscription and repositories | `true` | `subscription` e `apt/repositories`: estado, avisos, repositórios padrão ativados. |

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de certificado mostrado no dispositivo | Certificado autoassinado. Instale um certificado de confiança (ACME está incorporado no Proxmox) ou ative `insecure_tls`. A opção nunca é ativada implicitamente. |
| Erro de autenticação | O segredo colado em Token ID (ou o inverso), «Privilege Separation» deixado marcado (o token então não tem direitos), ou a função `DumbMonit` não aplicada em `/`. |
| Idade de cópia de segurança em falta para um convidado | Nenhuma tarefa `vzdump` na janela de observação e nenhum arquivo encontrado nos armazenamentos; ou `scan_backup_storage` desativado. |
| Verificações lentas | Um armazenamento de cópia de segurança que demora a listar, ou muitos convidados a inventariar para snapshots. Desative `scan_backup_storage` ou `scan_snapshots`, baixe `max_snapshot_guests`, ou suba `request_timeout_seconds`. |
| `node_updates_pending` em falta | O utilizador ou token não tem `Sys.Modify` em `/nodes`. Conceda a função `DumbMonitUpdates` do passo 6, ou ignore: `node_pve_packages_upgradable` e `node_reboot_required` cobrem os pacotes Proxmox sem isso. |
| Nenhuma percentagem de memória para uma VM | O Proxmox VE 9 reporta, no inventário do cluster, a memória que a VM ocupa **no anfitrião** — sobrecarga incluída, por isso regularmente acima da sua atribuição. Uma percentagem calculada a partir disso ficaria acima de 100% para sempre. O valor honesto vem do driver de balão dentro do convidado; uma VM com ballooning desativado portanto mostra bytes e nenhuma percentagem. `guest_memory_host_bytes` é o valor do lado do anfitrião. |
| Nenhuma capacidade para o armazenamento de cópia de segurança | Um datastore Proxmox Backup Server montado no PVE não reporta nenhum tamanho: o PVE não o lê. A própria página do dispositivo do datastore tem-o. |
| Uso de disco vazio («size only») para uma VM | A VM não tem nenhum agente convidado QEMU (não ativado nas suas opções, ou `qemu-guest-agent` não instalado dentro), ou o token não tem `VM.GuestAgent.Audit`. `guest_agent_enabled` e `guest_agent_running` dizem qual. Os contentores nunca precisam dele. |
| Nenhuma série `node_disk_*` | O token não tem `Sys.Audit` em `/` (a função `DumbMonit` concede-o), ou o nó corre atrás de um controlador RAID que esconde o SMART: `disks/list` então reporta `UNKNOWN` e nenhum `node_disk_smart_failed` é publicado. |
| `node_packages_changed` dispara depois de um reinício do servidor | Não dispara: a primeira verificação depois de um reinício só aprende as versões. Uma alteração só é reportada entre duas verificações consecutivas do mesmo processo do servidor. |
| `backup_job_last_ok{job=…}` em falta enquanto `backup_job_last_ok{node=…}` existe | As tarefas vzdump não transportam o id da tarefa (Proxmox VE mais antigo do que 7.2, ou tarefas executadas manualmente). O estado de execução por nó continua lá. |
| Nenhuma métrica de Ceph ou replicação | Nada a reportar: o Ceph não está instalado, ou nenhuma tarefa de replicação está configurada. Não é um erro. |
| Nenhum thin pool ou grupo de volumes num cartão de nó | A opção `lvm` está desligada, o nó não tem LVM, ou o utilizador detém `Sys.Audit` só em `/nodes` — esse inventário pede-o em `/`. |
| A secção Ceph está em falta enquanto o cluster tem Ceph | `ceph` ou `ceph_detail` está desligada, ou todo nó respondeu às chamadas de detalhe com um erro. `available: false` em `/api/targets/{id}/proxmox/ceph` significa «nenhum Ceph aqui», não «ainda não coletado». |
| Nenhum sistema operativo ou endereço IP para uma VM | O agente convidado QEMU não está ativado ou instalado nessa VM, ou o token não tem `VM.GuestAgent.Audit` no Proxmox VE 8 (passo 7). Os contentores não precisam de agente. Um convidado que acabou de iniciar espera até uma hora: os factos são atualizados a cada hora, não a cada verificação. |
| Um convidado mantém um crachá de bloqueio | Uma cópia de segurança, migração ou snapshot interrompida deixou-o. `qm unlock <vmid>` para uma VM, `pct unlock <vmid>` para um contentor — depois de verificar que a tarefa realmente terminou. |
| `pve_backup_excludes_disk` dispara numa máquina com um CD-ROM | Não dispara: unidades, imagens cloudinit e entradas que não são volumes são excluídas por desenho e nunca reportadas por convidado. O que disparou é um disco real a transportar `backup=0`. |
| Nenhuma série `*_rrd_*` | `metrics_export` está desligado (é por predefinição), ou o utilizador não tem `Sys.Audit` em `/`. |
