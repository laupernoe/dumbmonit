# Agente Linux, macOS, FreeBSD e Windows {#linux-macos-freebsd-and-windows-agent}

Um pequeno binário instalado na máquina a monitorizar. Mede CPU,
memória, discos, rede, serviços e contentores — e, onde a máquina os
expõe, temperaturas, saúde de disco e pools ZFS — depois **envia** as
suas leituras para o servidor DumbMonit via HTTP.

A direção é deliberada: o agente chama o servidor, nunca o contrário.
Atravessa NAT e firewalls domésticas, e não exige abrir **nenhuma
porta** na máquina monitorizada.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_`. As etiquetas de
identidade (`target`, `host`, `tag_*`) são definidas pelo servidor na
receção, nunca pelo agente: uma máquina não pode escrever na série de
outra máquina, mesmo forjando as suas etiquetas.

| Família | Métricas | Etiquetas |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` só com `cpu_per_core: true` | `core` |
| Memória | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Sistemas de ficheiros | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Rede (contadores) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Discos (contadores) | `disk_read_bytes`, `disk_written_bytes`, uma série por dispositivo de bloco | `device` |
| Anfitrião | `uptime_seconds`, `process_count` | |
| Serviços | `service_up` (1 = em execução) | `service` |
| Contentores | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (contentores além de `docker_max_containers`) | `container`, `image` |
| Sensores | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (o limiar que o próprio hardware declara), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Saúde de disco (SMART) | `agent_disk_smart_ok` (1 = o disco passa a sua própria autoavaliação), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degradado, 2 pior), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; por par `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (contadores) — ver [Túneis WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — ver [Cópias de segurança restic e Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Contadores de desempenho do Windows | `agent_perf_counter`, uma série por contador listado em `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — ver [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; por VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; por disco virtual `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — ver [Anfitriões Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Agente | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Uma série que não pode ser medida está **ausente**, nunca a zero:
sem `smartctl`, sem `agent_disk_*`; sem sonda legível, sem
`agent_sensor_*`; sem ZFS, sem `agent_zfs_*`. Uma temperatura fora de
1–150 °C é descartada em vez de publicada, e uma leitura de
ventoinha de 0 rpm também é descartada — uma placa-mãe tem mais
conetores de ventoinha do que ventoinhas, e um vazio lê-se zero como
uma ventoinha morta.

### O que é coletado onde {#what-is-collected-where}

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, memória, swap, sistemas de ficheiros, rede, tempo de atividade, processos | sim | sim | sim | sim |
| Média de carga | sim | sim | sim | não (o Windows não tem essa noção) |
| Serviços | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | Serviços Windows |
| Contentores Docker | sim | sim, se `docker_socket` apontar para o socket do Docker Desktop | não (sem daemon Docker) | não |
| Temperaturas | `/sys/class/hwmon` | sensores SMC | `dev.cpu.N.temperature` (só CPU) | não |
| Ventoinhas | `/sys/class/hwmon` | não | não | não |
| Saúde de disco (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Pools ZFS | `zpool` (OpenZFS) | não | `zpool` | não |
| Saúde do SO (atualizações, reinício, unidades falhadas, SELinux) | sim | não | não | não |
| Cópias de segurança Plakar | sim | sim | sim | sim |
| Túneis WireGuard | `wg show` | quando `wg` está instalado (não testado) | quando `wg` está instalado (não testado) | não |
| Cópias de segurança restic e Borg | sim | sim | sim | só restic (não testado; o Borg não tem build para Windows) |
| Contadores de desempenho, filas MDaemon, Hyper-V | não | não | não | sim |
| Identidade da máquina | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | nome de anfitrião |
| Modo relay | sim | sim | sim | sim |

O Windows não tem nenhuma linha de temperatura de propósito: ler
sensores aí significa passar pelo WMI, com direitos administrativos,
para valores que a maioria das máquinas não expõe de todo. Publicar
nada é a resposta honesta.

O `smartctl` (smartmontools) e o `zpool` são procurados em cada
período de leitura e usados quando presentes; nenhum é instalado pelo
agente, e nenhum é obrigatório. Ambos precisam de root — o serviço
que o instalador regista corre como root, um agente iniciado
manualmente sob a sua própria conta não encontrará nada.

Os contadores são enviados em bruto, para que um reinício do agente
nunca produza uma taxa falsa; aplique `rate()` nas consultas.

Regras incorporadas que se aplicam: Device unreachable, High CPU,
Unusual CPU. «Device unreachable» também funciona em agentes: o
servidor verifica a idade do último lote recebido e declara a máquina
silenciosa depois de três períodos perdidos (pelo menos 90 segundos).

Mais quatro só disparam em máquinas que produzem a série
correspondente, por isso não precisam de nenhuma configuração e não
podem disparar numa máquina sem o hardware:

| Regra | Dispara quando |
|---|---|
| **Disk SMART failing** (critical) | a própria autoavaliação de um disco o reporta como a falhar |
| **ZFS pool degraded** (critical) | uma pool já não está `ONLINE` durante cinco minutos |
| **ZFS scrub found errors** (critical) | o último scrub encontrou erros, ou a pool reporta ficheiros permanentemente corrompidos |
| **Temperature above critical** (critical) | um sensor excede o limiar crítico que **o seu próprio hardware declara** — nenhum valor é fixo no código, sendo 85 °C uma emergência num disco e uma tarde normal numa CPU de portátil |

Mais três vigiam o que declara ou executa você mesmo, e ficam
silenciosas caso contrário: **WireGuard peer silent** (um par com um
keepalive persistente e nenhum handshake há 15 minutos), **restic/Borg
backup too old** (nenhum snapshot há 48 horas) e **restic/Borg
repository unreachable** — ver [Túneis WireGuard](#wireguard-tunnels) e
[Cópias de segurança restic e Borg](#restic-and-borg-backups).

Num servidor Windows que executa o MDaemon, mais três vigiam as suas
filas: **MDaemon mail queue growing**, **MDaemon Bad queue not empty**
e **MDaemon Retry queue high** (ver
[Regras](../alerting/rules.md#mdaemon-and-securitygateway)). Num
anfitrião Hyper-V, três vigiam as máquinas virtuais: **Hyper-V VM
health critical**, **Hyper-V host CPU high** e **Hyper-V virtual disk
errors** (ver [Anfitriões Hyper-V](#hyper-v-hosts)).

## Instalar {#install}

Os passos abaixo são os que o aviso junto ao formulário mostra. Não
há nenhuma conta para criar na máquina: o token de registo é o único
segredo.

1. Guarde o dispositivo no DumbMonit: um token de registo (dmon_…) é mostrado uma vez, com o comando de instalação pronto para copiar para Linux e para Windows. (Um token também pode ser criado em **Settings → Agents**.)
2. Esse token é a chave do agente para enviar as suas medições ao DumbMonit. Não é uma conta na máquina: nada para criar aí. Um token criado a partir do formulário de dispositivo é de uso único: regista essa única máquina e nada mais. Para uma frota, crie um token reutilizável em Settings → Agents.
3. Execute o comando de instalação na máquina a monitorizar, com direitos elevados (sudo no Linux e macOS, um PowerShell elevado no Windows). Descarrega o agente, escreve o token em agent.yaml e inicia o serviço.
4. A máquina aparece por si só dentro de alguns segundos, nomeada pelo seu nome de anfitrião. Nesse primeiro lote, o servidor dá ao agente um segredo próprio e guarda-o em `/etc/dumbmonit/agent-secret`, legível por mais ninguém: a partir daí, essa máquina é a única que pode reportar sob este dispositivo. Perdeu o token? Settings → Agents deixa-o revogar e criar outro.
5. Docker: para ver os contentores e deixar o DumbMonit reiniciá-los ou atualizá-los, o agente tem de alcançar o socket Docker. O serviço que o instalador regista já pode; se executar o agente sob um utilizador dedicado em vez disso, adicione esse utilizador ao grupo «docker» e reinicie-o. As políticas de reinício e auto-atualização são então definidas por contentor na página do dispositivo.

Duas coisas vale a pena ler antes de implementar isto em mais do que
uma máquina: [Associação](#binding-one-machine-one-agent), que é o que
faz um dispositivo realmente pertencer a uma instalação de agente, e
[Uso único ou frota](#single-use-or-fleet), que é a escolha que faz
quando cria um token.

    ```
    usermod -aG docker dumbmonit
    ```

6. Cópias de segurança Plakar: detetadas automaticamente — o painel Backups aparece quando o binário plakar ou um kloset (~/.config/plakar/stores.yml de todo utilizador, ~/.plakar, /var/lib/plakar) é encontrado, e não é mostrado nada caso contrário. Defina «plakar_klosets» em agent.yaml para monitorizar uma lista fixa, ou «plakar: false» para desativar.

!!! note
    O agente contacta o servidor, nunca o contrário: nenhuma porta precisa de ser aberta na máquina monitorizada.

=== "Linux (systemd ou OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    O script é `sh` POSIX (BusyBox e dash funcionam), descarrega o
    binário para a arquitetura da máquina (x86_64 ou aarch64) a
    partir do servidor, escreve `/etc/dumbmonit/agent.yaml` (modo
    0600) e regista `dumbmonit-agent` com o systemd, ou com o OpenRC
    no Alpine. A unidade systemd é reforçada (`ProtectSystem=strict`,
    `NoNewPrivileges`, `MemoryMax=128M`, `CPUQuota=20%`).

=== "macOS (launchd)"

    O mesmo comando que no Linux, executado com `sudo`:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Os binários macOS **não** estão na imagem DumbMonit: construí-los
    exige o SDK da Apple, cuja licença proíbe a redistribuição. São
    anexados a cada
    [versão GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
    em vez disso — `dumbmonit-agent-macos-aarch64` (Apple silicon) e
    `dumbmonit-agent-macos-x86_64` (Intel), cada um com o seu
    `.sha256` — e o servidor redireciona
    `/download/dumbmonit-agent-macos-…` para lá, por isso o script
    escolhe o adequado à arquitetura do Mac (`uname -m`). O servidor
    não distribui esses binários, por isso o comando de instalação
    não transporta nenhum checksum para eles: adicione
    `--sha256=macos-aarch64:<hex>` (ou `macos-x86_64`) com o valor do
    `.sha256` da versão, ou o instalador recusa a descarga. O Mac
    também precisa de alcançar o github.com.

    Sem esse acesso, ou para instalar um binário que já tem,
    descarregue-o e passe-o com `--bin` (compare-o primeiro com o
    `.sha256` da versão: `--bin` instala o que lhe der):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    O script instala o binário em `/usr/local/bin` (e remove o
    atributo de quarentena que uma descarga do navegador transporta,
    que faria o Gatekeeper recusar iniciá-lo), escreve
    `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e regista o
    daemon do sistema `com.dumbmonit.agent` em
    `/Library/LaunchDaemons`, iniciado com `launchctl bootstrap
    system`. Corre no arranque, sem ninguém com sessão iniciada.

    !!! note "Prefere construí-lo você mesmo?"
        No próprio Mac, com o Rust instalado:
        `cargo build --release -p dumbmonit-agent`, depois
        `--bin=target/release/dumbmonit-agent`. É o mesmo binário que
        a tarefa de versão produz.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Só x86_64. O script descarrega `dumbmonit-agent-freebsd-x86_64`
    do servidor, escreve `/usr/local/etc/dumbmonit/agent.yaml` (modo
    0600) e instala `/usr/local/etc/rc.d/dumbmonit_agent`, ativado com
    `service dumbmonit_agent enable`. O `daemon(8)` supervisiona o
    agente, reinicia-o se parar e escreve
    `/var/log/dumbmonit-agent.log`.

    Funciona da mesma forma no TrueNAS CORE e noutros aparelhos
    baseados em FreeBSD, onde as séries ZFS são o ponto do exercício.

=== "Windows (serviço)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    O script descarrega `dumbmonit-agent-windows-x86_64.exe` do
    servidor, verifica-o contra o seu SHA-256, instala-o como
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, escreve
    `C:\ProgramData\DumbMonit\agent.yaml` (legível só por SYSTEM e
    Administrators) e regista o serviço Windows `DumbMonitAgent`
    («DumbMonit system agent»): iniciado automaticamente no arranque,
    como LocalSystem, e reiniciado depois de uma falha (10 s, 30 s,
    depois 60 s). O agente fala com o próprio gestor de serviços
    (`--service`), por isso parar o serviço deixa-o enviar o que ainda
    detém antes de sair.

    Um serviço não tem consola: o agente escreve o seu registo em
    `C:\ProgramData\DumbMonit\agent.log`, arquivado como
    `agent.log.old` além de 10 MB.

    Não há nenhum build ARM64 nativo: no Windows 11 para ARM, o script
    instala o binário x86_64, que corre sob emulação x64.

Um token pode registar várias máquinas: nomeie-o por um grupo ou uma
máquina.

### Binários nativos {#native-binaries}

O agente é um único executável estático por sistema, sem nada para
instalar junto a ele:

| Sistema | Ficheiro | Onde obtê-lo |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | o servidor, `/download/…` |
| Linux aarch64 (ARM 64 bits: Raspberry Pi OS 64 bits, servidores ARM) | `dumbmonit-agent-linux-aarch64` | o servidor, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | o servidor, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | o servidor, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | a versão GitHub (o servidor redireciona para lá) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | a versão GitHub (o servidor redireciona para lá) |

Os binários Linux são ligados estaticamente contra o musl: correm em
qualquer distribuição, glibc ou não (Alpine, firmwares de NAS), sem
nenhuma biblioteca para corresponder. Cada ficheiro tem o seu
checksum junto a ele, `<ficheiro>.sha256`, no formato `sha256sum`.
Toda [versão GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
transporta os seis binários e os seus checksums, para máquinas que
não conseguem alcançar o `/download/` do servidor ou para instalar
com `--bin`.

Para construir os binários Linux, Windows e FreeBSD você mesmo, a
partir de um clone do repositório (o Docker é o único requisito):

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Gerir o serviço {#managing-the-service}

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Estado | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Registos | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Reiniciar | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Parar | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Configuração | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| Ficheiro de serviço | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | serviço `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` no Windows) remove tudo, serviço incluído,
em todo sistema.

!!! note "A descarga é verificada — sempre"
    O comando de instalação mostrado pelo DumbMonit transporta o
    SHA-256 esperado de cada binário de agente que o servidor
    distribui (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` no Windows). Ambos os instaladores
    verificam o ficheiro que descarregaram contra ele antes de
    instalar algo, e **recusam** instalar quando não corresponde,
    quando nenhum checksum é dado para a plataforma, ou quando
    nenhuma ferramenta SHA-256 (`sha256sum`, `shasum`, `sha256`) está
    disponível.

    Os checksums mudam com cada versão do servidor: depois de
    atualizar o DumbMonit, copie um comando de instalação fresco de
    **Settings → Agents** (ou de um token novo) em vez de executar
    um antigo de novo, o que pararia com «checksum mismatch». Os
    mesmos checksums são mostrados sob o comando de instalação, e
    publicados em `/download/<ficheiro>.sha256`.

    Para configurações air-gapped ou manuais,
    `--insecure-skip-checksum` (`-InsecureSkipChecksum`) instala um
    binário descarregado sem verificação e diz isso em alto e bom
    som; `--bin=CAMINHO` instala um ficheiro local como está, sendo
    o ficheiro seu para verificar.

### Flags do instalador {#installer-flags}

| `install.sh` | `install.ps1` | Efeito |
|---|---|---|
| `--token=TOKEN` | `-Token` | Token de registo (obrigatório). |
| `--url=URL` | `-Url` | URL do servidor, por exemplo `http://server:8080`. No Linux, `DUMBMONIT_URL` no ambiente é usado se a flag estiver ausente. |
| `--interval=N` | `-Interval` | Período de amostragem em segundos (predefinição 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Serviços cujo estado é reportado, nomeados como o sistema anfitrião os nomeia: unidades systemd no Linux, etiquetas launchd (`com.apple.sshd`) no macOS, nomes rc.d no FreeBSD, serviços Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Etiquetas, copiadas como `tag_<key>` em cada série. |
| `--hostname=NAME` | `-HostName` | Nome anunciado ao servidor (predefinição: o da máquina). |
| `--bin=PATH` | `-BinPath` | Binário local a instalar em vez de o descarregar. |
| `--sha256=LIST` | `-Sha256` | SHA-256 esperado da descarga, como pares `plataforma-arquitetura:hex` separados por vírgulas (um valor hex simples também funciona). Incluído no comando que o DumbMonit mostra; obrigatório a menos que a flag seguinte seja dada. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Instala um binário descarregado sem o verificar, com um aviso. Só para configurações air-gapped ou manuais. |
| `--no-start` | | Instala tudo, mas não contacta o servidor nem inicia o serviço (imagem de máquina, teste). |
| `--uninstall` | `-Uninstall` | Desinstala o agente e elimina a sua configuração. |
| `--help` | | Mostra a ajuda. |

### Atualizar {#upgrade}

Ambos os scripts são idempotentes: executar o comando de instalação de
novo, com o mesmo token e URL, substitui o binário, reescreve a
configuração e reinicia o serviço. Esse é o procedimento de
atualização. Tome o comando do servidor atual (Settings → Agents): o
seu `--sha256` tem de corresponder aos binários da versão do servidor
para a qual atualizou.

O mesmo comando migra uma máquina ainda a executar o agente EzyMonit,
no Linux e no Windows. O instalador para e remove o serviço antigo
(`ezymonit-agent`, ou `EzyMonitAgent` no Windows), move a configuração
para o novo caminho (`/etc/dumbmonit/agent.yaml`,
`C:\ProgramData\DumbMonit\agent.yaml`) e instala `dumbmonit-agent` no
seu lugar. O antigo token `ezym_…` continua válido, por isso o comando
mostrado quando o token foi criado continua a funcionar; nada tem de
ser revogado primeiro.

## Ficheiro de configuração {#configuration-file}

`/etc/dumbmonit/agent.yaml` no Linux, `/usr/local/etc/dumbmonit/agent.yaml`
no macOS e FreeBSD, `C:\ProgramData\DumbMonit\agent.yaml` no Windows:

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # período de amostragem
hostname: nas-basement     # opcional: nome anunciado ao servidor
services:                  # unidades systemd ou serviços Windows
  - sshd
  - docker
tags:                      # etiquetas livres, prefixadas "tag_" no servidor
  role: nas
  room: basement
docker: true               # inventário de contentores (predefinição: true)
docker_socket: /var/run/docker.sock
sensors: true              # temperaturas e ventoinhas (predefinição: true)
smart: true                # saúde de disco via smartctl (predefinição: true)
smart_bin: smartctl        # caminho, quando não está no PATH do serviço
smart_interval_secs: 300   # mínimo 60: um disco em suspensão nunca é despertado para ser lido
zfs: true                  # pools ZFS via zpool (predefinição: true)
zfs_bin: zpool
zfs_interval_secs: 120     # mínimo 30
wireguard: true            # túneis WireGuard via wg (predefinição: true), ver abaixo
restic_repos: []           # repositórios restic a monitorizar, ver "cópias de segurança restic e Borg"
borg_repos: []             # repositórios Borg a monitorizar
mdaemon: auto              # Windows: filas MDaemon quando o seu serviço existe (true / false / auto)
hyperv: auto               # Windows: contadores Hyper-V quando o serviço vmms existe (true / false / auto)
perf_counters:             # Windows: contadores de desempenho a publicar
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # valor da etiqueta "counter"; o caminho quando omitido
max_buffered_samples: 20000
log_level: info
```

`sensors`, `smart` e `zfs` são detetados, nunca assumidos: deixados a
`true` numa máquina sem nenhuma sonda, nenhum `smartctl` ou nenhuma
pool, custam um olhar por período e não emitem nada de todo. Defina
um para `false` para parar mesmo de procurar — vale a pena fazer num
NAS cujos discos preferiria que o `smartctl` nunca tocasse, embora o
agente já passe `-n standby` para que um disco em suspensão só seja
medido quando estiver desperto por outras razões.

Junto a ele, `/etc/dumbmonit/agent-secret`
(`C:\ProgramData\DumbMonit\agent-secret` no Windows) contém o segredo
de associação que o servidor deu a esta máquina. O agente escreve-o
com modo `0600` e relê-o em cada arranque; não faz parte de
`agent.yaml` porque não é algo que define — ver
[Associação](#binding-one-machine-one-agent). `DUMBMONIT_AGENT_SECRET_FILE`
move-o, o que é o que quer para um agente num contentor.

Cada chave pode ser substituída pelo ambiente, o que torna o agente
utilizável num contentor sem montar um ficheiro:

| Variável | Efeito |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Caminho do ficheiro de configuração |
| `DUMBMONIT_AGENT_URL` | URL do servidor |
| `DUMBMONIT_AGENT_TOKEN` | Token de registo |
| `DUMBMONIT_AGENT_SECRET_FILE` | Onde manter o segredo de associação (predefinição: `agent-secret`, junto ao ficheiro de configuração) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Período de amostragem |
| `DUMBMONIT_AGENT_HOSTNAME` | Nome anunciado ao servidor |
| `DUMBMONIT_AGENT_SERVICES` | Serviços a monitorizar, separados por vírgulas |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, separados por vírgulas |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Caminho do socket Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Contentores detalhados por anfitrião, predefinição `200` (`0`: só contagens) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfaces deixadas de fora, nomes ou regexes separados por vírgulas; predefinição `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfaces a manter; quando definida, substitui a lista de exclusão |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Pontos de montagem deixados de fora de sistemas de ficheiros e I/O de disco; predefinição ignora `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` para também enviar uma série de CPU por núcleo (predefinição `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: temperaturas e ventoinhas (predefinição `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: saúde de disco via `smartctl` (predefinição `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Caminho do `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Período entre duas leituras SMART, predefinição `300`, mínimo `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: pools ZFS via `zpool` (predefinição `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Caminho do `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Período entre duas leituras ZFS, predefinição `120`, mínimo `30` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: túneis WireGuard via `wg` (predefinição `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Caminho do `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Caminho do `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Caminho do `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Período entre duas leituras dos repositórios restic e Borg, predefinição `600`, mínimo `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` ou `auto`: contadores de desempenho do MDaemon (predefinição `auto`, lidos quando o serviço Windows `MDaemon` existe) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` ou `auto`: contadores de desempenho do Hyper-V (predefinição `auto`, lidos quando o serviço Windows `vmms` existe) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Caminhos de contador de desempenho Windows, separados por **ponto e vírgula** (uma vírgula pode fazer parte de um nome de instância); substitui `perf_counters` |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Tamanho do buffer de recuperação |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` para executar, em nome do servidor, as verificações dos dispositivos atribuídos a este agente (ver [Executar o agente em Docker / noutra rede](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Etiqueta de site mostrada junto ao agente quando é oferecido como relay |

## Contadores de desempenho do Windows {#windows-performance-counters}

No Windows, o agente lê contadores de desempenho — os valores que o
Performance Monitor (`perfmon`) mostra — através da biblioteca PDH, e
publica-os em cada período de amostragem. Alguns softwares não
publicam o seu estado em nenhum outro lugar.

**MDaemon.** Quando o serviço Windows chamado `MDaemon` existe, o
agente lê o objeto de desempenho MDaemon: tamanhos de fila, sessões,
totais de mensagens e spam, o estado dos servidores internos do
MDaemon e o seu tempo de atividade. As séries, as regras e a página
que as mostra estão descritas na página [MDaemon](mdaemon.md#mail-queues-through-the-agent).
`mdaemon: true` lê-as mesmo sem o serviço (um serviço renomeado),
`mdaemon: false` nunca o faz. A presença do serviço é verificada de
novo a cada dez períodos, por isso um MDaemon instalado depois do
agente é captado sem reinício.

**Hyper-V.** Quando o serviço Hyper-V Virtual Machine Management
(`vmms`) existe, o agente lê os objetos de desempenho do Hyper-V; ver
[Anfitriões Hyper-V](#hyper-v-hosts). `hyperv: true` e `hyperv: false`
forçam a escolha, como para o MDaemon.

**Qualquer outro contador.** Liste o seu caminho completo sob
`perf_counters`, como `\Objeto(Instância)\Contador` ou `\Objeto\Contador`
para um objeto com uma única instância, e é publicado como
`dumbmonit_agent_perf_counter{counter="…"}`, com o nome que der ou o
próprio caminho:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Escreva os nomes de contador **em inglês**, como aparecem no
  Performance Monitor num Windows inglês: o agente adiciona-os com
  `PdhAddEnglishCounterW`, por isso o mesmo `agent.yaml` funciona
  num Windows francês ou alemão.
- Coloque cada caminho entre aspas simples no YAML, para que as
  barras invertidas e o `%` sejam mantidos como estão.
- Um caminho malformado para o agente no arranque, com a chave e o
  motivo. Um caminho que o Windows não conhece (o software não está
  instalado, o nome tem um erro de escrita) é reportado uma vez no
  registo, `performance counter not available`, e os outros
  contadores continuam a ser lidos; é tentado de novo a cada dez
  períodos.
- As taxas (`…/sec`) precisam de duas leituras: aparecem a partir do
  segundo período.
- No máximo 200 contadores são lidos. No Linux, macOS e FreeBSD a
  definição é aceite e nada é lido.

Para ver os nomes exatos na máquina, execute `typeperf -q MDaemon`
(ou qualquer outro nome de objeto) numa linha de comandos.

## Anfitriões Hyper-V {#hyper-v-hosts}

Instalado num anfitrião Hyper-V — Windows Server com a função
Hyper-V, ou Windows com o Hyper-V ativado — o agente encontra o
serviço `vmms` e lê os próprios contadores de desempenho do Hyper-V,
sem nada para configurar. A página de dispositivo do agente mostra
então um painel Hyper-V: quantas máquinas virtuais reportam
saudáveis, a carga de CPU real do anfitrião, a memória que cada VM em
execução detém, e o caudal, latência e erros de cada disco virtual.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Os nomes dos contadores vêm da documentação de desempenho Hyper-V
    da Microsoft e foram verificados contra a lógica com uma fonte de
    contador simulada, não num anfitrião Hyper-V em funcionamento. Um
    contador que o Windows não conhece é reportado uma vez no registo
    do agente e os outros continuam a ser lidos. Diga-nos o que falha.

| Contador | Série | Etiquetas |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: máquinas virtuais em cada estado | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: a carga do anfitrião, convidados incluídos | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: a fração passada a executar convidados | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: VMs em execução mais o próprio anfitrião | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: procura de memória sobre memória atribuída; acima de 100 a VM quer mais do que tem | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (média), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, um contador | `disk` |

Num anfitrião Hyper-V, `\Processor(_Total)\% Processor Time` só mede a
própria partição do anfitrião, não o que as máquinas virtuais
consomem: o contador de processador lógico do hipervisor é o que diz
se a máquina está saturada. `vm` é o nome da máquina virtual como o
Hyper-V Manager o mostra; `disk` é o nome de instância que o Windows
dá a cada disco virtual, derivado do seu caminho de ficheiro. As
instâncias `_Total` são deixadas de fora, e no máximo 200 VMs ou
discos são lidos por contador.

Três [regras incorporadas](../alerting/rules.md#hyper-v) aplicam-se:

- **Hyper-V VM health critical**: o Hyper-V reporta uma VM em saúde
  crítica durante cinco minutos (Warning) — mais frequentemente um
  disco que ficou sem espaço.
- **Hyper-V host CPU high**: os processadores lógicos mais de 90%
  ocupados durante quinze minutos (Advisory).
- **Hyper-V virtual disk errors**: um disco virtual reportou erros de
  I/O na última hora (Advisory).

## Executar o agente em Docker / noutra rede {#run-the-agent-in-docker--on-another-network}

O agente também é publicado como uma imagem de contentor, com as
mesmas etiquetas que o servidor: `ghcr.io/laupernoe/dumbmonit-agent:latest`
(última versão), `:edge` (último commit em `main`). É uma imagem
`FROM scratch` que contém o binário do agente e o conjunto de CAs da
Mozilla, configurada inteiramente por variáveis de ambiente. O
repositório traz um `docker-compose.agent.yml` pronto a usar:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

O que o contentor vê, e o que montar:

| Necessidade | O que fazer | Porquê |
|---|---|---|
| CPU, memória, carga, I/O de disco do anfitrião | Nada | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` e `/proc/diskstats` são à escala do anfitrião dentro de qualquer contentor. |
| Nome de anfitrião | `DUMBMONIT_AGENT_HOSTNAME` | O hostname de um contentor é um id aleatório; o dispositivo seria nomeado por ele. |
| Contentores do anfitrião | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Opcional. Listagens e inspeções só de leitura; as ações Restart/Update precisam dele escrevível e `DUMBMONIT_AGENT_COMMANDS=true`. Sem o socket, o agente simplesmente não reporta nenhum contentor. |
| Uso de sistema de ficheiros | `- /:/host:ro` (ou as montagens que lhe importam) | O agente reporta as montagens visíveis *dentro* do contentor, sob o seu caminho de contentor. |
| Contadores de rede do anfitrião | `network_mode: host` | As interfaces têm namespace: sem isto o agente vê um `veth`. |
| Monitores de ping relayed através deste agente | `cap_add: [NET_RAW]` | O ICMP precisa de um socket raw. |
| CA privada | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Servidor atrás de um proxy inverso interno, dispositivos com certificados autoassinados. |
| Manter a associação entre recriações | `- agent-state:/var/lib/dumbmonit-agent` e `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | O segredo de associação vive no sistema de ficheiros do contentor, que um `docker compose up --force-recreate` descarta. Sem o volume, o agente volta desassociado e alguém tem de permitir o reregisto a partir da página do dispositivo cada vez. |

`pid: host` e `privileged: true` nunca são necessários: o agente não
coleta nenhuma métrica por processo.

### Modo relay: monitorizar outro site {#relay-mode-monitor-another-site}

Um agente pode fazer mais do que vigiar a sua própria máquina. Com
`relay: true` (ou `DUMBMONIT_AGENT_RELAY=true`), o servidor **delega
verificações** a ele: dispositivos cuja definição *Reached through*
nomeia este agente não são sondados pelo servidor mas pelo agente, a
partir da sua própria rede, com exatamente os mesmos coletores — SNMP,
Proxmox VE, Proxmox Backup Server, Synology, HTTP, TCP, DNS, ping,
TLS. Os resultados aterram sob o dispositivo como se o servidor o
tivesse sondado: as mesmas métricas, a mesma *última verificação*, os
mesmos alertas. Um escritório remoto, um segundo homelab ou a rede de
um cliente assim aparecem no único DumbMonit que já tem, sem uma VPN
e sem nada para abrir do lado remoto.

Como configurá-lo, passo a passo, está em
[Monitorizar um site remoto](../install/remote-site.md).

## Associação: uma máquina, um agente {#binding-one-machine-one-agent}

Um token de registo diz *esta máquina pode falar*. Não diz *qual*
máquina está a falar — para isso serve a chave de identidade
(`/etc/machine-id`, ou o nome de anfitrião), e essa chave não é
nenhum segredo: pode ser lida em qualquer máquina em segundos.

Por isso, no primeiro lote o servidor dá a cada máquina um **segredo
de associação** próprio. Só guarda uma impressão digital; o agente
escreve o segredo em `/etc/dumbmonit/agent-secret` com modo `0600` e
apresenta-o no cabeçalho `X-DumbMonit-Agent-Secret` em todo pedido
seguinte — medições, comandos de contentor e verificações relayed
igualmente. A partir daí:

- nenhuma outra máquina pode enviar medições sob este dispositivo, ou
  reescrever o seu nome de anfitrião e sistema operativo;
- nenhuma outra máquina pode obter — e portanto consumir — os
  comandos Restart e Update enfileirados para ela.

A página do dispositivo mostra onde uma máquina está, sob **Agent →
Binding**:

| Estado | O que significa | O que fazer |
|---|---|---|
| **Bound** | O agente detém um segredo próprio. | Nada. |
| **Not bound — re-enrol this host** | Um anfitrião registado antes de a associação existir, e nunca associado desde então. O servidor recusa as suas medições, comandos de contentor e verificações relayed. | Clique em **Allow re-enrolment** na página do dispositivo, depois reinicie o agente. Se o agente for anterior à associação, execute primeiro o comando de instalação no anfitrião. |

### Anfitriões anteriores à associação {#hosts-that-predate-binding}

!!! warning "Alterado: anfitriões não associados são recusados"
    Versões anteriores continuavam a aceitar anfitriões registados
    antes de a associação existir, enquanto usassem o token que os
    registou, e associavam-nos silenciosamente uma vez o seu agente
    atualizado. Essa janela de transição está fechada: um anfitrião
    que nunca se associou agora é **recusado em toda rota de agente**
    — medições, comandos de contentor e verificações relayed — até um
    administrador o reregistar. Um token de frota é partilhado por
    muitas máquinas e a chave de identidade pode ser adivinhada, por
    isso nada mais prova que o agente a perguntar é o da própria
    máquina.

    Um binário de agente demasiado antigo para receber um segredo de
    associação também é recusado, incluindo no seu primeiro registo.

Para trazer tal anfitrião de volta:

1. Se o seu agente for anterior à associação, execute de novo o
   comando de instalação nele (o binário do agente tem de conseguir
   guardar um segredo de associação).
2. Abra a sua página de dispositivo e clique em **Agent → Allow
   re-enrolment**. Isto abre uma janela de uma hora.
3. Reinicie o agente, ou espere pela sua próxima tentativa: esse
   lote associa o anfitrião com um segredo fresco e fecha a janela.
   Um token diferente do que registou o anfitrião continua a ter de
   poder registar (não esgotado, não expirado).

Até então, o registo do agente transporta a explicação do servidor:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Reregisto depois de uma reinstalação {#re-enrolment-after-a-reinstall}

Se a máquina for reconstruída, o seu disco substituído, ou o seu
contentor de agente recriado sem o volume que detém o segredo, o
agente volta sem ele e o servidor recusa os seus lotes. Essa recusa é
deliberada: uma máquina associada permanece associada até um humano
dizer o contrário, caso contrário a associação não protegeria nada.

O caminho de volta está na página do dispositivo: **Agent → Allow
re-enrolment**. Abre uma janela de uma hora durante a qual um agente
que apresenta um token de registo válido associa essa máquina de
novo, com um segredo fresco; o antigo segredo deixa de funcionar, e a
janela fecha assim que é usada. O agente tenta de novo por si só
entretanto, por isso não há nada para reiniciar na máquina — e nada
se perde, já que as suas leituras ficam em buffer até a ligação ser
aceite de novo.

Até alguém abrir essa janela, o registo do agente diz o que fazer:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Tokens de registo {#enrolment-tokens}

- Os tokens são criados em **Settings → Agents** ou ao adicionar um
  dispositivo do tipo agent. O servidor só guarda uma impressão
  digital: o token em claro é mostrado uma vez, na resposta de
  criação, com os dois comandos de instalação.
- O agente envia-o como `Authorization: Bearer dmon_…` em
  `POST /api/ingest`. Nunca aparece nos registos do agente, nem
  mesmo truncado.
- **Settings → Agents** lista os tokens com o seu prefixo, âmbito,
  data de criação e última utilização. **Revoke** para todo agente
  que usa esse token na sua próxima publicação; as máquinas então
  tornam-se *inacessíveis* depois de três períodos perdidos. Execute
  o instalador de novo com um token novo para as reregistar.
- As máquinas são dispositivos separados: revogar um token não as
  elimina.

### Uso único ou frota {#single-use-or-fleet}

O âmbito de um token é escolhido quando é criado, e é uma escolha,
não uma predefinição que lhe acontece:

| Âmbito | O que regista | Quando |
|---|---|---|
| **Single use** (predefinição) | Exatamente uma máquina. | Uma instalação. Um comando de instalação sobrevive num histórico de shell, num ticket, num registo de chat; um de uso único não vale nada uma vez usado. |
| **Reusable for a fleet** | Tantas máquinas quantas permitir — um número, ou sem limite. | Um playbook, uma imagem de máquina, um lote de instalações. |

Ambos podem transportar um prazo em dias. Um prazo só para
*registos*: máquinas já registadas continuam a reportar depois de
passar, que é o que quer para uma janela de instalação. Revogar é o
que corta uma frota.

Um token que já não pode registar — esgotado, ou passado o seu prazo
— é mostrado como **Enrols no more** em Settings → Agents. As suas
máquinas continuam a reportar; só não pode deixar entrar uma nova.

## Indisponibilidades e diagnóstico {#outages-and-diagnostics}

Quando o servidor está inacessível, o agente continua a medir e põe
as suas leituras em buffer na memória com as suas data-horas
originais. São enviadas assim que a ligação volta: um reinício do
servidor não deixa nenhum buraco nos gráficos. O buffer é limitado
(`max_buffered_samples`, cerca de uma hora por predefinição) e
descarta as leituras mais antigas quando está cheio. As repetições
recuam de 5 segundos a 5 minutos com jitter.

```sh
dumbmonit-agent --dry-run          # imprime as leituras, não envia nada
dumbmonit-agent --once             # envia um lote depois sai
journalctl -u dumbmonit-agent -f   # registo do serviço (systemd)
tail -f /var/log/dumbmonit-agent.log   # registo do serviço (OpenRC)
```

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| O instalador diz «could not reach the server» | URL ou token errado. A configuração é escrita: corrija `agent.yaml`, depois reinicie o serviço. |
| A descarga falha com 404 | O servidor não tem binários de agente (`DUMBMONIT_AGENT_DIR` vazio). Use `--bin=PATH` com um binário que construiu. |
| O instalador para com «checksum mismatch» | O comando foi copiado antes de uma atualização do servidor (copie um fresco de Settings → Agents), ou o binário recebido não é o que o servidor serve: um proxy ou uma cache no caminho, ou uma descarga alterada. |
| O instalador para com «no expected checksum» | O comando não tem nenhum `--sha256` para esta plataforma: um comando antigo, ou um binário macOS (não distribuído pelo servidor). Copie um comando fresco, ou passe `--sha256=<plataforma>-<arquitetura>:<hex>` do `.sha256` da versão. |
| A máquina nunca aparece | O envio vai para `/api/ingest` no URL do comando de instalação; atrás de um proxy inverso, certifique-se de que é encaminhado e que corpos até 16 MB são permitidos. |
| *Unreachable* embora o agente corra | O token foi revogado, ou os envios são rejeitados. Verifique `journalctl -u dumbmonit-agent`: o erro é registado lá. |
| O registo diz «this agent is not recognised for this machine» | A máquina está associada a uma instalação de agente cujo segredo esta não tem — uma reinstalação, ou um contentor recriado sem o seu volume de estado. Abra a página do dispositivo e clique em **Allow re-enrolment**; ver [Reregisto depois de uma reinstalação](#re-enrolment-after-a-reinstall). |
| O registo diz «this machine was enrolled before agent binding» | Um anfitrião que nunca se associou ao seu agente. Clique em **Allow re-enrolment** na sua página de dispositivo, depois reinicie o agente; ver [Anfitriões anteriores à associação](#hosts-that-predate-binding). |
| O registo diz «this agent is too old to be bound to its machine» | O binário do agente não consegue guardar um segredo de associação. Execute de novo o comando de instalação no anfitrião; se o dispositivo já existe, clique primeiro em **Allow re-enrolment**. |
| O registo diz «this enrollment token has already enrolled all the machines it was allowed to» | Um token de uso único a ser reutilizado. Crie um novo, ou um token reutilizável em Settings → Agents. |
| Windows | As partes de serviço Windows não tinham sido testadas na imagem de build do projeto no momento da escrita; reporte problemas no GitHub. |

## Contentores Docker {#docker-containers}

Quando o agente consegue ler o socket Docker (`docker: true`, a
predefinição, e um utilizador no grupo `docker` ou root —
`usermod -aG docker dumbmonit`, depois reinicie o agente), cada
contentor torna-se um punhado de séries, etiquetadas `container` e
`image` (o `name:tag` mostrado por `docker ps`). Se a página do
dispositivo disser «No Docker on this machine» enquanto o Docker está
instalado, isso é a primeira coisa a verificar.

| Métrica | Significado |
|---|---|
| `container_up` | 1 em execução, 0 caso contrário |
| `container_health` | 0 sem verificação de saúde, 1 saudável, 2 não saudável, 3 a iniciar |
| `container_restart_count` | Contador de reinícios do Docker para o contentor |
| `container_started_seconds` | Segundos desde que o contentor iniciou por último (0 quando parado) |
| `container_image_age_seconds` | Segundos desde que a imagem foi construída |
| `container_update_available` | 1 quando o registo tem uma imagem mais recente para a mesma etiqueta, 0 quando atualizado, -1 desconhecido |

A verificação de atualização compara o digest da imagem local com o
que o registo anuncia para a etiqueta (`HEAD /v2/<name>/manifests/<tag>`,
fluxo de token anónimo no Docker Hub e GHCR). Corre em segundo plano
no máximo uma vez por hora por imagem e nunca atrasa o ciclo de 30
segundos; um registo privado que recusa acesso anónimo simplesmente
lê-se -1. Desative-a com `docker_update_check: false`
(`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Regras incorporadas: **Container stopped** (`container_up == 0`
durante 2 minutos), **Container unhealthy** (`container_health == 2`
durante 3 minutos), **Container restarting** (três ou mais reinícios
em 15 minutos), **Container update available** (info, depois de uma
hora, repetido diariamente). O alerta nomeia o contentor.

### Ações {#actions}

Na página do dispositivo, cada contentor tem duas ações e dois
interruptores. As ações viajam através de um **canal de comando**: o
servidor enfilera o comando, o agente obtém-o depois do seu lote
seguinte (`GET /api/agent/commands`, mesmo token de registo),
executa-o, e reporta o resultado (`queued → running → done | failed`),
com um registo curto que pode desdobrar na interface. Comandos com
mais de dez minutos nunca são executados; o agente executa um comando
de cada vez.

Um comando enfilerado pode ser **cancelado** a partir da interface
enquanto o agente não o tiver recolhido. Um comando que ninguém vem
obter **expira** depois de dez minutos, no servidor, quer o agente
consulte ou não — por isso uma máquina cujo agente está parado nunca
mantém um «Queued» obsoleto para sempre, e nunca bloqueia a ação
seguinte nesse contentor.

O agente diz ao servidor, com cada lote, se aceita comandos. Quando
não aceita — um agente mais antigo do que o canal de comando, ou
instalado com `commands: false` — a página do dispositivo diz-o em
vez de mostrar Restart e Update, a API recusa os comandos (`409`), e
as políticas não enfileiram nada. Reinstale o agente com o instalador
atual para ativar as ações.

- **Restart** — `docker restart` para um contentor em execução,
  `docker start` para um parado, depois uma verificação de 20
  segundos de que corre.
- **Update now** — obtém a etiqueta que o contentor já usa, cria um
  novo contentor com a mesma configuração (ambiente, montagens,
  portas, etiquetas, redes, política de reinício), para o antigo,
  troca os nomes, inicia o novo e espera pela sua verificação de
  saúde (ou 20 segundos mais uma verificação de execução quando a
  imagem não tem uma). Se algo falhar, o agente **reverte**: o novo
  contentor é removido e o antigo reiniciado, e o comando reporta
  `failed` com o motivo. No sucesso o contentor antigo é removido e,
  quando «prune» está ativo, a imagem antiga também — a menos que
  outro contentor ainda a use.

Os contentores geridos por Compose (etiquetas `com.docker.compose.*`)
são atualizados da mesma forma, mas o resultado avisa que o próximo
`docker compose up` recriará o contentor a partir do ficheiro
compose. Contentores cujo nome começa por `dumbmonit` ou `ezymonit`
são recusados: o monitor nunca se atualiza a si próprio.

### Políticas {#policies}

Uma **política** por contentor é avaliada pelo servidor a cada
minuto:

- **Restart if down** — um contentor a reportar `container_up == 0` é
  reiniciado automaticamente, no máximo uma vez a cada dez minutos.
- **Auto-update in maintenance windows** — um contentor com uma
  atualização disponível é atualizado automaticamente, mas só
  enquanto uma janela de manutenção (silêncio) cobre o dispositivo,
  no máximo uma vez a cada seis horas. Agende uma na página Alerts,
  ou silencie o dispositivo a partir da sua página. As políticas têm
  por predefinição podar a imagem antiga e só janelas de manutenção.

Dois lugares as definem:

- a **faixa Docker** sob o cabeçalho do dispositivo: «28 containers ·
  27 running · 3 updates available · policies: 2 auto-restart, 0
  auto-update», a última ação e o seu estado, **Manage containers**
  (abre a lista abaixo) e **Policies…**, uma tabela de todo contentor
  com os dois interruptores e uma linha «Apply to all» — a forma mais
  rápida de ativar o auto-restart para toda uma máquina;
- os interruptores dentro de cada linha da secção **Containers**,
  junto aos botões Restart e Update now.

Todo interruptor guarda por si só
(`PUT /api/targets/<id>/containers/<name>/policy`); uma falha só
concerne essa linha e é mostrada junto a ela.

As ações automáticas aparecem em «Recent actions» como pedidas por
`policy`.

### Regras de segurança {#safety-rules}

- O agente só age quando `commands: true` (predefinição); defina-o
  para `false` para uma máquina que só deve ser medida.
- O agente obtém os comandos ele próprio, com o seu token de registo
  — nada liga à máquina. Expiram depois de dez minutos e correm um
  de cada vez; o agente nunca obtém uma etiqueta diferente da que o
  contentor já usa.
- Uma atualização que não volta saudável é revertida para o
  contentor anterior, e a imagem antiga só é removida uma vez o
  novo contentor em execução.
- Contentores chamados `dumbmonit*` ou `ezymonit*` são recusados, à
  mão ou por política: o monitor nunca age sobre os seus próprios
  contentores. A verificação é feita duas vezes — o servidor recusa
  enfilerar o comando, e o agente recusa executá-lo — e o agente
  fá-la de novo sobre o nome que o Docker reporta depois de inspecionar
  o contentor, por isso nomear um pelo seu id não altera nada. O
  agente também se recusa a agir sobre o seu próprio contentor, seja
  como for que se chame: não estaria lá para terminar a tarefa ou
  reportar sobre ela.
- Os contentores geridos por Compose são atualizados, mas o próximo
  `docker compose up` recria-os a partir do ficheiro compose: suba a
  etiqueta aí também.

## Cópias de segurança Plakar {#plakar-backups}

O agente pode monitorizar klosets (repositórios)
[Plakar](https://plakar.io): a cada dez minutos, em segundo plano,
executa `plakar at <kloset> ls` e `info` em cada um e reemite a
última leitura a cada ciclo.

O Plakar é **detetado, nunca assumido**. No arranque e depois em
cada ciclo até encontrar algo, o agente procura o binário `plakar`
(no `PATH`, ou `plakar_bin`) e klosets nas localizações padrão.
Quando não há nenhum binário, nenhum kloset e nenhum vestígio de
Plakar (`~/.cache/plakar`), o agente não emite nenhuma série
`backup_*` de todo, diz-o uma vez ao nível `debug`, e a página do
dispositivo não mostra nenhum painel Backups — nada para configurar
em máquinas sem Plakar. Para saltar completamente a deteção:

```yaml
plakar: false                        # ou DUMBMONIT_AGENT_PLAKAR=false
```

### Descoberta {#discovery}

Nada para configurar: quando `plakar_klosets` está vazio, o agente
encontra os klosets ele próprio, por esta ordem, e lê cada
localização só uma vez:

1. o próprio diretório de configuração do agente —
   `$XDG_CONFIG_HOME/plakar`, ou `~/.config/plakar` da conta sob a
   qual corre (`plakar_home` quando definido);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, alfabeticamente;
4. `/var/lib/plakar`, quando esse diretório existe.

Em cada diretório lê `stores.yml` (ou `stores.yaml`; o legado
`plakar.yml` também é aceite) e toma toda entrada do mapa `stores`
(também `repositories` e `klosets`) que tem uma `location`. Só a
localização é lida: frases-passe e segredos rclone que ficam junto a
ela nunca são guardados nem registados. Cada entrada torna-se o
kloset `@<name>`, lido com `plakar -configdir <dir> at @<name>`, por
isso um kloset criado por um utilizador é lido mesmo quando o agente
corre como root. `~/.plakar` de toda conta é adicionado quando existe
(o kloset predefinido do `plakar create`). Duas contas que nomeiam
klosets diferentes da mesma forma são distinguidas como `@name` e
`<user>:@name`.

O resultado é registado uma vez ao nível `info` (e de novo só quando
muda). Dois gauges, emitidos só uma vez o Plakar detetado, deixam a
interface distinguir os casos restantes: `backup_plakar_present` (1
quando o binário corre) e `backup_klosets_found` (número de klosets
lidos). A página do dispositivo então diz «Plakar is installed but no
kloset was found», ou lista os klosets. Quando os klosets existem mas
o binário não corre (um `PATH` de serviço sem `plakar`, tipicamente),
são listados como ilegíveis com uma sugestão de definir `plakar_bin`,
e o agente avisa uma vez.

Para monitorizar uma lista fixa em vez disso, nomeie-a — a descoberta
é então saltada:

```yaml
plakar: true                         # predefinição: detetar o Plakar; false esconde-o completamente
plakar_bin: /usr/bin/plakar          # predefinição: "plakar" no PATH
plakar_klosets:                      # lista explícita: substitui a descoberta
  - /srv/backups/main
  - "@diskext"                       # um store nomeado da conta do agente
plakar_home: /home/noe               # opcional: HOME (e diretório de config) usado ao executar o plakar
plakar_interval_secs: 600            # mínimo 60
```

| Variável | Efeito |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — monitorizar cópias de segurança Plakar quando detetadas (predefinição `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Caminho do binário `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets, separados por vírgulas (substitui a descoberta) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Diretório home usado ao executar o `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Segundos entre duas leituras |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — verificação de registo para atualizações de contentor |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — aceitar ações do servidor |

Um kloset encriptado precisa da sua frase-passe: coloque-a na entrada
do store (`plakar store set <name> passphrase=…`, o que é o que
`plakar store add` faz) ou exporte `PLAKAR_PASSPHRASE` no ambiente do
serviço do agente. Sem ela o kloset lê-se «Cannot be opened»
(`backup_last_status = 0`).

### Métricas e regras {#metrics-and-rules}

Métricas, etiquetadas `kloset` e `source` (o diretório que teve cópia
de segurança): `backup_last_success_seconds` (idade do snapshot mais
recente), `backup_snapshot_count`, `backup_last_status` (1 quando o
snapshot mais recente não tem erro, 0 quando tem erros ou o kloset
não pode ser aberto), `backup_size_bytes{kloset}` (tamanho de
armazenamento do repositório), mais `backup_plakar_present` e
`backup_klosets_found` descritos acima. Nenhuma delas existe enquanto
o Plakar não é detetado, por isso as regras incorporadas abaixo não
podem disparar numa máquina sem Plakar.

Regras incorporadas: **Plakar backup too old** (advisory, nenhum
snapshot há mais de dois dias) e **Plakar backup failed** (warning).
A página do dispositivo mostra um bloco por kloset — idade da última
cópia de segurança, contagem de snapshots, tamanho, último estado —
e uma linha por fonte, com uma sugestão «Backup too old» além de
dois dias.

### Executar uma primeira cópia de segurança {#run-a-first-backup}

Com o Plakar 1.1 instalado na máquina (`plakar version`), como o
utilizador que detém as cópias de segurança:

```sh
# 1. um store de kloset, nomeado para que o agente o encontre em ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # ou qualquer localização que o Plakar aceite
plakar at @diskext create                                   # adicione -plaintext para saltar a encriptação

# 2. uma primeira cópia de segurança, depois verifique-a
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` e `plakar at <path> backup <dir>` funcionam
da mesma forma com um diretório simples em vez de `@name`; `plakar
create` sem `at` usa `~/.plakar`, que o agente também encontra.
Agende `plakar at @diskext backup /srv/photos` com cron ou um timer
systemd: o agente reporta a idade do snapshot mais recente, e alerta
quando passa um dia sem nenhum — dois dias para a regra incorporada.

Para experimentar localmente com um kloset não encriptado:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

depois adicione `plakar_klosets: ["/tmp/plakar-demo"]` a `agent.yaml`
(um caminho simples fora de uma home não é descoberto) e verifique
com `dumbmonit-agent --dry-run` que amostras `backup_*` aparecem.

## Cópias de segurança restic e Borg {#restic-and-borg-backups}

O agente pode monitorizar repositórios [restic](https://restic.net) e
[Borg](https://www.borgbackup.org): a cada dez minutos, em segundo
plano, abre cada repositório que declarar e reporta se pôde ser lido
e quão antigo é o seu snapshot mais recente. Nada é descoberto: um
repositório não tem localização padrão e não pode ser aberto sem a
sua senha, por isso cada um é listado em `agent.yaml`, com a forma de
obter a sua senha.

**A senha nunca sai da máquina.** O agente entrega-a ao `restic` ou
`borg` só através do ambiente do processo filho; nunca é registada,
nunca enviada ao servidor, e as séries só transportam o nome do
repositório.

```yaml
restic_repos:
  - name: nas                                   # a etiqueta "repo"; predefinição: o repositório, sem credenciais
    repository: sftp:backup@nas.lan:/srv/restic  # passada como RESTIC_REPOSITORY, nunca na linha de comandos
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # opcional: só os snapshots deste anfitrião (--host), para um repositório partilhado
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # ou qualquer comando que a imprima (RESTIC_PASSWORD_COMMAND)
    env:                                        # ambiente extra só para este repositório
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # passada como BORG_REPO
    password_file: /etc/dumbmonit/borg-offsite.pass        # lida pelo agente, passada como BORG_PASSPHRASE
    glob_archives: "web-01-*"                   # opcional: só estes arquivos (--glob-archives)
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # predefinição: no PATH
borg_bin: borg
backup_repos_interval_secs: 600                 # mínimo 60
```

Mantenha `agent.yaml` e os ficheiros de senha legíveis só por root
(`chmod 600`), como o instalador faz para `agent.yaml`.
`password_command` mapeia para `RESTIC_PASSWORD_COMMAND` ou
`BORG_PASSCOMMAND`. Qualquer coisa que as ferramentas lêem do seu
ambiente pode ir sob `env` (`RESTIC_PASSWORD`, `BORG_PASSPHRASE`,
chaves cloud, `BORG_RSH`…), e o próprio ambiente de serviço do agente
também é passado.

O que o agente executa, só de leitura no sentido mais estrito:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: nenhum
  bloqueio é escrito, por isso um repositório só de leitura ou só de
  anexação pode ser monitorizado, e um `prune` que detém o bloqueio
  exclusivo não faz o repositório parecer em baixo; também nenhuma
  cache, já que o serviço instalado não pode escrever em `/root`.
- `borg --bypass-lock list --json --last 1`: sem `--bypass-lock`, a
  leitura falharia durante toda a duração de cada `borg create`, que
  detém o bloqueio exclusivo. O Borg escreve um pequeno registo de
  segurança sob `~/.config/borg`; quando essa home não é escrevível
  (o serviço instalado corre com `ProtectHome=yes`), o agente aponta
  `BORG_BASE_DIR` para o seu `/tmp` privado em vez disso, a menos que
  defina `BORG_BASE_DIR` você mesmo. Um repositório em modo
  `keyfile` precisa então de `BORG_KEY_FILE` no seu `env`.

Como o serviço corre com `ProtectHome=yes`, mantenha o que as
ferramentas precisam fora dos diretórios home: ficheiros de senha,
chaves SSH e `known_hosts` sob `/etc/dumbmonit/`, como no exemplo, e
repositórios locais sob `/srv`, `/mnt` ou `/var` em vez de `/home`. As
leituras correm uma depois da outra, dentro do limite de memória do
serviço (128 MiB com o próprio agente).

Métricas, etiquetadas `tool` (`restic` ou `borg`) e `repo`:
`agent_backup_repo_reachable` (1 quando a última leitura teve
sucesso, 0 caso contrário: repositório inacessível, senha recusada,
binário em falta — o agente regista o motivo uma vez, e de novo
quando muda), `agent_backup_repo_last_snapshot_age_seconds` (idade do
snapshot mais recente, ausente enquanto o repositório está vazio), e
`agent_backup_repo_last_snapshot_bytes` (bytes que o snapshot restic
mais recente processou, restic 0.17 e posterior). Com `--latest 1`, o
restic devolve o snapshot mais recente de cada conjunto de anfitrião e
caminho; o mais recente de todos é mantido, por isso defina `host`
num repositório partilhado por várias máquinas.

Regras incorporadas: **restic/Borg backup too old** (nenhum snapshot
há mais de 48 horas) e **restic/Borg repository unreachable** (durante
30 minutos). A página do dispositivo mostra um painel Backup
repositories, uma linha por repositório.

Para verificar uma declaração, execute o agente uma vez em primeiro
plano e procure pelas amostras `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Túneis WireGuard {#wireguard-tunnels}

Numa máquina que executa o WireGuard, o agente lê toda interface e
todo par com `wg show all dump`, a cada ciclo. Nada para configurar:
sem o comando `wg` (wireguard-tools) ou sem uma interface, nenhuma
série é emitida e nenhum painel mostrado. O `wg` precisa de root
(`CAP_NET_ADMIN`); o serviço instalado tem-no, um agente iniciado
manualmente sob a sua própria conta regista a recusa uma vez e não
reporta nada.

O dump também contém a chave privada da interface e a chave
pré-partilhada de cada par: o agente ignora ambos os campos sem
nunca os copiar.

Por par, etiquetado `interface`, `peer` (a sua chave pública) e
`allowed_ips`: `agent_wireguard_peer_handshake_age_seconds` (segundos
desde o handshake mais recente), `agent_wireguard_peer_has_handshake`
(0 enquanto o par nunca completou um), `agent_wireguard_peer_keepalive_seconds`
(o seu keepalive persistente, 0 quando desligado), e os contadores
`agent_wireguard_peer_rx_bytes` e `agent_wireguard_peer_tx_bytes`; por
interface, `agent_wireguard_interface_peers`. Para um par que nunca
completou um handshake, a idade conta a partir do momento em que o
agente o viu por primeira vez: um túnel que nunca subiu também fica
silencioso, e é reportado como tal.

O WireGuard não tem nomes de par. Dê os seus em `agent.yaml`, pela
chave pública, e tornam-se a etiqueta `name`:

```yaml
wireguard: true                     # predefinição; false para de procurar mesmo
wireguard_bin: wg                   # predefinição: no PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

A regra incorporada **WireGuard peer silent** dispara quando um par
**com um keepalive persistente** não completou um handshake há mais
de 15 minutos. Enquanto um túnel está ativo, o WireGuard renova o seu
handshake a cada dois minutos, e um keepalive mantém-no ativo mesmo
sem tráfego; um par sem keepalive (um telefone, um portátil) pode
ficar inativo durante dias sem nada avariado, por isso é deixado de
fora. Defina `PersistentKeepalive = 25` nos túneis que têm de ficar
ativos (ligações site-to-site, gateways remotos). N é o limiar da
regra: altere-o em **Alerts → Rules**, ou retire a parte
`and on (…)` da sua consulta para monitorizar todo par. A página do
dispositivo mostra um painel WireGuard com cada par, o tempo desde o
seu último handshake, o seu tráfego e o seu keepalive.

## Certificados TLS (Certbot / Let's Encrypt) {#tls-certificates-certbot-lets-encrypt}

O agente não lê `/etc/letsencrypt` diretamente. Todo certificado que
o Certbot renova é servido em algum lugar — um servidor web, um
servidor de email, um proxy inverso — e é aí que o DumbMonit o lê:
adicione um monitor [Certificado TLS](services.md#tls) (ou um
[Site ou API web](services.md#http), para um site já monitorizado
dessa forma) apontado para a porta em que o certificado é realmente
apresentado. Lê o certificado ativo, não um ficheiro em disco, por
isso também apanha uma renovação que teve sucesso em
`/etc/letsencrypt` mas nunca foi captada pelo serviço (um proxy
inverso que precisa de uma recarga para ver o ficheiro novo é o caso
habitual, e exatamente a lacuna que importa).

Ler `/etc/letsencrypt/live/*/cert.pem`, a configuração de renovação e
o registo do Certbot a partir do agente foi considerado e posto de
lado: duplicaria a data de expiração que o monitor TLS já reporta,
enquanto adicionaria um segundo caminho, local ao anfitrião, para a
mesma informação — mais uma coisa a manter sincronizada depois de um
certificado ser movido ou um domínio renomeado. Um anfitrião onde o
Certbot corre mas não serve nada que o DumbMonit possa alcançar
diretamente (um passo de CA interna, um gancho de validação DNS) é o
único caso que isto deixa descoberto; é também o caso em que um
monitor TLS também não poderia ajudar, já que nada nesse anfitrião
fala TLS para sondar.
