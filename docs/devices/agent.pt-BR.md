# Agente Linux, macOS, FreeBSD e Windows {#linux-macos-freebsd-and-windows-agent}

Um pequeno binário instalado na máquina a monitorar. Ele mede CPU, memória,
discos, rede, serviços e contêineres — e, onde a máquina os expõe,
temperaturas, saúde de disco e pools ZFS — depois **envia** suas leituras
para o servidor DumbMonit via HTTP.

A direção é deliberada: o agente chama o servidor, nunca o contrário. Ele
cruza NAT e firewalls domésticos, e não exige abrir **nenhuma porta** na
máquina monitorada.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_`. Rótulos de identidade
(`target`, `host`, `tag_*`) são definidos pelo servidor na recepção, nunca
pelo agente: uma máquina não pode escrever na série de outra máquina, nem
mesmo falsificando seus rótulos.

| Família | Métricas | Rótulos |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` só com `cpu_per_core: true` | `core` |
| Memória | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Sistemas de arquivos | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Rede (contadores) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Discos (contadores) | `disk_read_bytes`, `disk_written_bytes`, uma série por dispositivo de bloco | `device` |
| Host | `uptime_seconds`, `process_count` | |
| Serviços | `service_up` (1 = em execução) | `service` |
| Contêineres | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (contêineres além de `docker_max_containers`) | `container`, `image` |
| Sensores | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (o limite que o próprio hardware declara), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Saúde de disco (SMART) | `agent_disk_smart_ok` (1 = o disco passa sua própria autoavaliação), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degradado, 2 pior), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; por peer `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (contadores) — veja [Túneis WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — veja [Backups restic e Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Contadores de desempenho do Windows | `agent_perf_counter`, uma série por contador listado em `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — veja [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; por VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; por disco virtual `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — veja [Hosts Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Agente | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Uma série que não pode ser medida está **ausente**, nunca zero: sem
`smartctl`, sem `agent_disk_*`; sem sondagem legível, sem `agent_sensor_*`;
sem ZFS, sem `agent_zfs_*`. Uma temperatura fora de 1–150 °C é descartada em
vez de publicada, e uma leitura de ventoinha de 0 rpm também é descartada —
uma placa-mãe tem mais conectores de ventoinha que ventoinhas, e um vazio
lê zero como uma ventoinha morta.

### O que é coletado onde {#what-is-collected-where}

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, memória, swap, sistemas de arquivos, rede, tempo de atividade, processos | sim | sim | sim | sim |
| Média de carga | sim | sim | sim | não (o Windows não tem essa noção) |
| Serviços | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | Serviços Windows |
| Contêineres Docker | sim | sim, se `docker_socket` aponta para o socket do Docker Desktop | não (sem daemon Docker) | não |
| Temperaturas | `/sys/class/hwmon` | sensores SMC | `dev.cpu.N.temperature` (apenas CPU) | não |
| Ventoinhas | `/sys/class/hwmon` | não | não | não |
| Saúde de disco (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Pools ZFS | `zpool` (OpenZFS) | não | `zpool` | não |
| Saúde do SO (atualizações, reinício, unidades falhadas, SELinux) | sim | não | não | não |
| Backups Plakar | sim | sim | sim | sim |
| Túneis WireGuard | `wg show` | quando `wg` está instalado (não testado) | quando `wg` está instalado (não testado) | não |
| Backups restic e Borg | sim | sim | sim | apenas restic (não testado; Borg não tem build Windows) |
| Contadores de desempenho, filas MDaemon, Hyper-V | não | não | não | sim |
| Identidade da máquina | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | nome de host |
| Modo relay | sim | sim | sim | sim |

O Windows deliberadamente não tem linha de temperatura: ler sensores lá
significa passar por WMI, com direitos administrativos, para valores que a
maioria das máquinas não expõe de forma alguma. Não publicar nada é a
resposta honesta.

`smartctl` (smartmontools) e `zpool` são procurados a cada período de
leitura e usados quando presentes; nenhum dos dois é instalado pelo agente,
e nenhum é obrigatório. Ambos precisam de root — o serviço que o instalador
registra roda como root, um agente iniciado manualmente sob sua própria
conta não encontrará nada.

Contadores são enviados em bruto, para que um reinício do agente nunca
produza uma taxa falsa; aplique `rate()` em consultas.

Regras integradas que se aplicam: Device unreachable, High CPU, Unusual
CPU. "Device unreachable" também funciona em agentes: o servidor verifica
a idade do último lote recebido e declara a máquina silenciosa depois de
três períodos perdidos (ao menos 90 segundos).

Mais quatro disparam só em máquinas que produzem a série correspondente,
então não precisam de configuração e não podem disparar em uma máquina
sem o hardware:

| Regra | Dispara quando |
|---|---|
| **Disk SMART failing** (crítica) | a própria autoavaliação de um disco o reporta como falhando |
| **ZFS pool degraded** (crítica) | um pool não está mais `ONLINE` por cinco minutos |
| **ZFS scrub found errors** (crítica) | o último scrub encontrou erros, ou o pool reporta arquivos permanentemente corrompidos |
| **Temperature above critical** (crítica) | um sensor excede o limite crítico que **seu próprio hardware declara** — nenhum valor é fixo no código, 85 °C sendo uma emergência em um disco e uma tarde normal na CPU de um laptop |

Mais três observam o que você mesmo declara ou executa, e permanecem
silenciosas caso contrário: **WireGuard peer silent** (um peer com
keepalive persistente e sem handshake por 15 minutos), **restic/Borg
backup too old** (nenhum snapshot por 48 horas) e **restic/Borg repository
unreachable** — veja [Túneis WireGuard](#wireguard-tunnels) e
[Backups restic e Borg](#restic-and-borg-backups).

Em um servidor Windows que roda o MDaemon, mais três observam suas filas:
**MDaemon mail queue growing**, **MDaemon Bad queue not empty** e
**MDaemon Retry queue high** (veja
[Regras](../alerting/rules.md#mdaemon-and-securitygateway)). Em um host
Hyper-V, três observam as máquinas virtuais: **Hyper-V VM health
critical**, **Hyper-V host CPU high** e **Hyper-V virtual disk errors**
(veja [Hosts Hyper-V](#hyper-v-hosts)).

## Instalação {#install}

Os passos abaixo são os que o aviso ao lado do formulário mostra. Não há
conta para criar na máquina: o token de cadastro é o único segredo.

1. Salve o dispositivo no DumbMonit: um token de cadastro (dmon_…) é
   mostrado uma vez, com o comando de instalação pronto para copiar para
   Linux e para Windows. (Um token também pode ser criado em
   **Configurações → Agentes**.)
2. Esse token é a chave do agente para enviar suas medições ao DumbMonit.
   Não é uma conta na máquina: nada para criar lá. Um token criado a
   partir do formulário de dispositivo é de uso único: cadastra essa
   única máquina e nada mais. Para uma frota, crie um token reutilizável
   em Configurações → Agentes.
3. Execute o comando de instalação na máquina a monitorar, com direitos
   elevados (sudo no Linux e macOS, um PowerShell elevado no Windows).
   Ele baixa o agente, escreve o token em agent.yaml e inicia o serviço.
4. A máquina aparece por conta própria em alguns segundos, nomeada após
   seu nome de host. Nesse primeiro lote o servidor dá ao agente um
   segredo próprio e o armazena em `/etc/dumbmonit/agent-secret`, legível
   por ninguém mais: a partir de então, essa máquina é a única que pode
   reportar sob este dispositivo. Perdeu o token? Configurações →
   Agentes deixa você revogá-lo e criar outro.
5. Docker: para ver os contêineres e deixar o DumbMonit reiniciá-los ou
   atualizá-los, o agente precisa alcançar o socket do Docker. O serviço
   que o instalador registra já pode; se você roda o agente sob um
   usuário dedicado em vez disso, adicione esse usuário ao grupo "docker"
   e o reinicie. Políticas de reinício e atualização automática são então
   definidas por contêiner na página do dispositivo.

Duas coisas valem a pena ler antes de implantar isso em mais de uma
máquina: [Vínculo](#binding-one-machine-one-agent), que é o que faz um
dispositivo realmente pertencer a uma instalação de agente, e
[Uso único ou frota](#single-use-or-fleet), que é a escolha que você faz
quando cria um token.

    ```
    usermod -aG docker dumbmonit
    ```

6. Backups Plakar: detectados automaticamente — o painel Backups aparece
   quando o binário plakar ou um kloset (~/.config/plakar/stores.yml de
   todo usuário, ~/.plakar, /var/lib/plakar) é encontrado, e nada é
   mostrado caso contrário. Defina "plakar_klosets" em agent.yaml para
   observar uma lista fixa, ou "plakar: false" para optar por não
   participar.

!!! note
    O agente contata o servidor, nunca o contrário: nenhuma porta precisa
    ser aberta na máquina monitorada.

=== "Linux (systemd ou OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    O script é `sh` POSIX (BusyBox e dash funcionam), baixa o binário
    para a arquitetura da máquina (x86_64 ou aarch64) do servidor, escreve
    `/etc/dumbmonit/agent.yaml` (modo 0600) e registra `dumbmonit-agent`
    com o systemd, ou com o OpenRC no Alpine. A unidade systemd é
    reforçada (`ProtectSystem=strict`, `NoNewPrivileges`,
    `MemoryMax=128M`, `CPUQuota=20%`).

=== "macOS (launchd)"

    O mesmo comando do Linux, executado com `sudo`:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Os binários macOS **não** estão na imagem DumbMonit: construí-los
    exige o SDK da Apple, cuja licença proíbe redistribuição. Em vez
    disso são anexados a cada
    [lançamento do GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
    — `dumbmonit-agent-macos-aarch64` (Apple silicon) e
    `dumbmonit-agent-macos-x86_64` (Intel), cada um com seu `.sha256` — e
    o servidor redireciona `/download/dumbmonit-agent-macos-…` para lá,
    então o script escolhe o certo para a arquitetura do Mac (`uname
    -m`). O servidor não entrega esses binários, então o comando de
    instalação não carrega checksum para eles: adicione
    `--sha256=macos-aarch64:<hex>` (ou `macos-x86_64`) com o valor do
    `.sha256` do lançamento, ou o instalador recusa o download. O Mac
    também precisa alcançar github.com.

    Sem esse acesso, ou para instalar um binário que você já tem,
    baixe-o e passe-o com `--bin` (compare-o com o `.sha256` do
    lançamento primeiro: `--bin` instala o que você der a ele):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    O script instala o binário em `/usr/local/bin` (e remove o atributo
    de quarentena que um download de navegador carrega, que faria o
    Gatekeeper recusar iniciá-lo), escreve
    `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e registra o daemon
    do sistema `com.dumbmonit.agent` em `/Library/LaunchDaemons`,
    iniciado com `launchctl bootstrap system`. Ele roda na inicialização,
    sem ninguém conectado.

    !!! note "Prefere construí-lo você mesmo?"
        No próprio Mac, com o Rust instalado:
        `cargo build --release -p dumbmonit-agent`, depois
        `--bin=target/release/dumbmonit-agent`. Esse é o mesmo binário que
        o job de lançamento produz.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Apenas x86_64. O script baixa `dumbmonit-agent-freebsd-x86_64` do
    servidor, escreve `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e
    instala `/usr/local/etc/rc.d/dumbmonit_agent`, ativado com
    `service dumbmonit_agent enable`. O `daemon(8)` supervisiona o
    agente, o reinicia se parar e escreve `/var/log/dumbmonit-agent.log`.

    Funciona da mesma forma no TrueNAS CORE e outros appliances baseados
    em FreeBSD, onde as séries ZFS são o ponto do exercício.

=== "Windows (serviço)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    O script baixa `dumbmonit-agent-windows-x86_64.exe` do servidor,
    verifica contra seu SHA-256, o instala como
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, escreve
    `C:\ProgramData\DumbMonit\agent.yaml` (legível apenas por SYSTEM e
    Administrators) e registra o serviço Windows `DumbMonitAgent`
    ("DumbMonit system agent"): iniciado automaticamente na
    inicialização, como LocalSystem, e reiniciado depois de uma falha
    (10 s, 30 s, depois 60 s). O agente fala com o próprio gerenciador de
    serviço (`--service`), então parar o serviço o deixa enviar o que
    ainda mantém antes de sair.

    Um serviço não tem console: o agente escreve seu log em
    `C:\ProgramData\DumbMonit\agent.log`, guardado como `agent.log.old`
    além de 10 MB.

    Não há build ARM64 nativo: no Windows 11 para ARM, o script instala
    o binário x86_64, que roda sob emulação x64.

Um token pode cadastrar várias máquinas: nomeie-o após um grupo ou uma
máquina.

### Binários nativos {#native-binaries}

O agente é um único executável estático por sistema, sem nada para
instalar ao lado dele:

| Sistema | Arquivo | Onde obtê-lo |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | o servidor, `/download/…` |
| Linux aarch64 (ARM 64 bits: Raspberry Pi OS 64 bits, servidores ARM) | `dumbmonit-agent-linux-aarch64` | o servidor, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | o servidor, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | o servidor, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | o lançamento do GitHub (o servidor redireciona para lá) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | o lançamento do GitHub (o servidor redireciona para lá) |

Os binários Linux são linkados estaticamente contra o musl: eles rodam em
qualquer distribuição, glibc ou não (Alpine, firmwares de NAS), sem
nenhuma biblioteca para corresponder. Cada arquivo tem seu checksum ao
lado, `<file>.sha256`, no formato `sha256sum`. Todo
[lançamento do GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
carrega os seis binários e seus checksums, para máquinas que não podem
alcançar o `/download/` do servidor ou para instalar com `--bin`.

Para construir os binários Linux, Windows e FreeBSD você mesmo, a partir
de um clone do repositório (o Docker é o único requisito):

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Gerenciando o serviço {#managing-the-service}

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Status | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Logs | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Reiniciar | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Parar | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Configuração | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| Arquivo de serviço | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | serviço `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` no Windows) remove tudo isso, serviço
incluído, em todo sistema.

!!! note "O download é verificado — sempre"
    O comando de instalação mostrado pelo DumbMonit carrega o SHA-256
    esperado de cada binário de agente que o servidor entrega
    (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` no Windows). Ambos os instaladores
    verificam o arquivo que baixaram contra isso antes de instalar
    qualquer coisa, e **recusam** instalar quando não corresponde, quando
    nenhum checksum é dado para a plataforma, ou quando nenhuma
    ferramenta SHA-256 (`sha256sum`, `shasum`, `sha256`) está disponível.

    Os checksums mudam com toda versão do servidor: depois de atualizar
    o DumbMonit, copie um comando de instalação novo de
    **Configurações → Agentes** (ou de um novo token) em vez de executar
    um antigo de novo, o que pararia com "checksum mismatch". Os mesmos
    checksums são mostrados sob o comando de instalação, e publicados em
    `/download/<file>.sha256`.

    Para configurações isoladas (air-gapped) ou manuais,
    `--insecure-skip-checksum` (`-InsecureSkipChecksum`) instala um
    binário baixado sem verificação e avisa isso em alto e bom som;
    `--bin=PATH` instala um arquivo local como está, o arquivo sendo seu
    para verificar.

### Flags do instalador {#installer-flags}

| `install.sh` | `install.ps1` | Efeito |
|---|---|---|
| `--token=TOKEN` | `-Token` | Token de cadastro (obrigatório). |
| `--url=URL` | `-Url` | URL do servidor, por exemplo `http://server:8080`. No Linux, `DUMBMONIT_URL` no ambiente é usado se a flag estiver ausente. |
| `--interval=N` | `-Interval` | Período de amostragem em segundos (padrão 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Serviços cujo estado é reportado, nomeados como o sistema hospedeiro os nomeia: unidades systemd no Linux, labels launchd (`com.apple.sshd`) no macOS, nomes rc.d no FreeBSD, serviços Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Tags, copiadas como `tag_<key>` em toda série. |
| `--hostname=NAME` | `-HostName` | Nome anunciado ao servidor (padrão: o da máquina). |
| `--bin=PATH` | `-BinPath` | Binário local a instalar em vez de baixá-lo. |
| `--sha256=LIST` | `-Sha256` | SHA-256 esperado do download, como pares `platform-arch:hex` separados por vírgulas (um valor hex simples também funciona). Incluído no comando que o DumbMonit mostra; obrigatório a menos que a próxima flag seja dada. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Instala um binário baixado sem verificá-lo, com um aviso. Apenas para configurações isoladas ou manuais. |
| `--no-start` | | Instala tudo, mas não contata o servidor nem inicia o serviço (imagem de máquina, testes). |
| `--uninstall` | `-Uninstall` | Desinstala o agente e exclui sua configuração. |
| `--help` | | Mostra a ajuda. |

### Atualização {#upgrade}

Ambos os scripts são idempotentes: executar o comando de instalação de
novo, com o mesmo token e URL, substitui o binário, reescreve a
configuração e reinicia o serviço. Esse é o procedimento de atualização.
Tome o comando do servidor atual (Configurações → Agentes): seu
`--sha256` precisa corresponder aos binários da versão do servidor para
a qual você atualizou.

O mesmo comando migra uma máquina ainda rodando o agente EzyMonit, no
Linux e no Windows. O instalador para e remove o serviço antigo
(`ezymonit-agent`, ou `EzyMonitAgent` no Windows), move a configuração
para o novo caminho (`/etc/dumbmonit/agent.yaml`,
`C:\ProgramData\DumbMonit\agent.yaml`) e instala `dumbmonit-agent` em seu
lugar. O antigo token `ezym_…` permanece válido, então o comando mostrado
quando o token foi criado ainda funciona; nada precisa ser revogado
primeiro.

## Arquivo de configuração {#configuration-file}

`/etc/dumbmonit/agent.yaml` no Linux,
`/usr/local/etc/dumbmonit/agent.yaml` no macOS e FreeBSD,
`C:\ProgramData\DumbMonit\agent.yaml` no Windows:

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # período de amostragem
hostname: nas-basement     # opcional: nome anunciado ao servidor
services:                  # unidades systemd ou serviços Windows
  - sshd
  - docker
tags:                      # rótulos livres, com prefixo "tag_" no servidor
  role: nas
  room: basement
docker: true               # inventário de contêiner (padrão: true)
docker_socket: /var/run/docker.sock
sensors: true              # temperaturas e ventoinhas (padrão: true)
smart: true                # saúde de disco via smartctl (padrão: true)
smart_bin: smartctl        # caminho, quando não está no PATH do serviço
smart_interval_secs: 300   # mínimo 60: um disco dormindo nunca é despertado para ser lido
zfs: true                  # pools ZFS via zpool (padrão: true)
zfs_bin: zpool
zfs_interval_secs: 120     # mínimo 30
wireguard: true            # túneis WireGuard via wg (padrão: true), veja abaixo
restic_repos: []           # repositórios restic a observar, veja "backups restic e Borg"
borg_repos: []             # repositórios Borg a observar
mdaemon: auto              # Windows: filas MDaemon quando seu serviço existe (true / false / auto)
hyperv: auto               # Windows: contadores Hyper-V quando o serviço vmms existe (true / false / auto)
perf_counters:             # Windows: contadores de desempenho a publicar
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # valor do rótulo "counter"; o caminho quando omitido
max_buffered_samples: 20000
log_level: info
```

`sensors`, `smart` e `zfs` são detectados, nunca assumidos: deixados em
`true` em uma máquina sem sonda, sem `smartctl` ou sem pool, custam uma
olhada por período e não emitem nada de forma alguma. Defina um como
`false` para parar até de olhar — vale a pena fazer em um NAS cujos
discos você preferiria que o `smartctl` nunca tocasse, embora o agente já
passe `-n standby` para que um disco dormindo seja medido só quando está
desperto por outros motivos.

Ao lado, `/etc/dumbmonit/agent-secret`
(`C:\ProgramData\DumbMonit\agent-secret` no Windows) mantém o segredo de
vínculo que o servidor deu a essa máquina. O agente o escreve com modo
`0600` e o relê a cada início; não faz parte de `agent.yaml` porque não é
algo que você define — veja [Vínculo](#binding-one-machine-one-agent).
`DUMBMONIT_AGENT_SECRET_FILE` o move, que é o que você quer para um
agente em um contêiner.

Cada chave pode ser sobrescrita pelo ambiente, o que torna o agente
usável em um contêiner sem montar um arquivo:

| Variável | Efeito |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Caminho do arquivo de configuração |
| `DUMBMONIT_AGENT_URL` | URL do servidor |
| `DUMBMONIT_AGENT_TOKEN` | Token de cadastro |
| `DUMBMONIT_AGENT_SECRET_FILE` | Onde manter o segredo de vínculo (padrão: `agent-secret`, ao lado do arquivo de configuração) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Período de amostragem |
| `DUMBMONIT_AGENT_HOSTNAME` | Nome anunciado ao servidor |
| `DUMBMONIT_AGENT_SERVICES` | Serviços a observar, separados por vírgulas |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, separados por vírgulas |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Caminho do socket Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Contêineres detalhados por host, padrão `200` (`0`: apenas contagens) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfaces deixadas de fora, nomes ou regexes separados por vírgulas; padrão `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfaces a manter; quando definido, substitui a lista de ignorados |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Pontos de montagem deixados de fora de sistemas de arquivos e E/S de disco; padrão pula `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` para também enviar uma série de CPU por núcleo (padrão `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: temperaturas e ventoinhas (padrão `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: saúde de disco via `smartctl` (padrão `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Caminho do `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Período entre duas leituras SMART, padrão `300`, mínimo `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: pools ZFS via `zpool` (padrão `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Caminho do `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Período entre duas leituras ZFS, padrão `120`, mínimo `30` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: túneis WireGuard via `wg` (padrão `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Caminho do `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Caminho do `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Caminho do `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Período entre duas leituras dos repositórios restic e Borg, padrão `600`, mínimo `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` ou `auto`: contadores de desempenho do MDaemon (padrão `auto`, lidos quando o serviço Windows `MDaemon` existe) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` ou `auto`: contadores de desempenho do Hyper-V (padrão `auto`, lidos quando o serviço Windows `vmms` existe) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Caminhos de contador de desempenho do Windows, separados por **ponto e vírgula** (uma vírgula pode fazer parte de um nome de instância); substitui `perf_counters` |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Tamanho do buffer de recuperação |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` para executar, para o servidor, as sondagens dos dispositivos atribuídos a este agente (veja [Executar o agente no Docker / em outra rede](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Rótulo de site mostrado ao lado do agente quando oferecido como relay |

## Contadores de desempenho do Windows {#windows-performance-counters}

No Windows, o agente lê contadores de desempenho — os valores que o
Performance Monitor (`perfmon`) mostra — através da biblioteca PDH, e os
publica a cada período de amostragem. Alguns softwares não publicam seu
estado em nenhum outro lugar.

**MDaemon.** Quando o serviço Windows chamado `MDaemon` existe, o agente
lê o objeto de desempenho MDaemon: tamanhos de fila, sessões, totais de
mensagem e spam, o estado dos servidores internos do MDaemon e seu tempo
de atividade. As séries, as regras e a página que as mostra são
descritas na página [MDaemon](mdaemon.md#mail-queues-through-the-agent).
`mdaemon: true` as lê mesmo sem o serviço (um serviço renomeado),
`mdaemon: false` nunca o faz. A presença do serviço é verificada de novo
a cada dez períodos, então um MDaemon instalado depois do agente é
detectado sem reinício.

**Hyper-V.** Quando o serviço Hyper-V Virtual Machine Management
(`vmms`) existe, o agente lê os objetos de desempenho do Hyper-V; veja
[Hosts Hyper-V](#hyper-v-hosts). `hyperv: true` e `hyperv: false` forçam
a escolha, como para o MDaemon.

**Qualquer outro contador.** Liste seu caminho completo sob
`perf_counters`, como `\Object(Instance)\Counter` ou `\Object\Counter`
para um objeto com uma única instância, e é publicado como
`dumbmonit_agent_perf_counter{counter="…"}`, com o nome que você dá ou o
próprio caminho:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Escreva os nomes de contador **em inglês**, como aparecem no
  Performance Monitor em um Windows em inglês: o agente os adiciona com
  `PdhAddEnglishCounterW`, então o mesmo `agent.yaml` funciona em um
  Windows francês ou alemão.
- Coloque cada caminho entre aspas simples em YAML, para que as barras
  invertidas e o `%` sejam mantidos como estão.
- Um caminho malformado para o agente na inicialização com a chave e o
  motivo. Um caminho que o Windows não conhece (o software não está
  instalado, o nome está com erro de digitação) é reportado uma vez no
  log, `performance counter not available`, e os outros contadores
  ainda são lidos; é tentado de novo a cada dez períodos.
- Taxas (`…/sec`) precisam de duas leituras: aparecem a partir do
  segundo período.
- No máximo 200 contadores são lidos. No Linux, macOS e FreeBSD a
  configuração é aceita e nada é lido.

Para ver os nomes exatos na máquina, execute `typeperf -q MDaemon` (ou
qualquer outro nome de objeto) em um prompt de comando.

## Hosts Hyper-V {#hyper-v-hosts}

Instalado em um host Hyper-V — Windows Server com o papel Hyper-V, ou
Windows com Hyper-V ativado — o agente encontra o serviço `vmms` e lê os
próprios contadores de desempenho do Hyper-V, sem nada para configurar.
A página de dispositivo do agente então mostra um painel Hyper-V: quantas
máquinas virtuais reportam saudáveis, a carga de CPU real do host, a
memória que cada VM em execução mantém, e a vazão, latência e erros de
cada disco virtual.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Os nomes de contador vêm da documentação de desempenho Hyper-V da
    Microsoft e foram verificados contra a lógica com uma fonte de
    contador simulada, não em um host Hyper-V em execução. Um contador
    que o Windows não conhece é reportado uma vez no log do agente e os
    outros ainda são lidos. Avise-nos o que não funcionar.

| Contador | Série | Rótulos |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: máquinas virtuais em cada estado | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: a carga do host, guests incluídos | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: a parcela gasta rodando guests | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: VMs em execução mais o próprio host | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: demanda de memória sobre memória atribuída; acima de 100 a VM quer mais do que tem | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (média), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, um contador | `disk` |

Em um host Hyper-V, `\Processor(_Total)\% Processor Time` só mede a
própria partição do host, não o que as máquinas virtuais consomem: o
contador de processador lógico do hipervisor é o que diz se a máquina
está saturada. `vm` é o nome da máquina virtual como o Hyper-V Manager a
mostra; `disk` é o nome de instância que o Windows dá a cada disco
virtual, derivado de seu caminho de arquivo. As instâncias `_Total` são
deixadas de fora, e no máximo 200 VMs ou discos são lidos por contador.

Três [regras integradas](../alerting/rules.md#hyper-v) se aplicam:

- **Hyper-V VM health critical**: o Hyper-V reporta uma VM em saúde
  crítica por cinco minutos (Alerta) — mais frequentemente um disco que
  ficou sem espaço.
- **Hyper-V host CPU high**: os processadores lógicos mais de 90%
  ocupados por quinze minutos (Aviso).
- **Hyper-V virtual disk errors**: um disco virtual reportou erros de
  E/S na última hora (Aviso).

## Executar o agente no Docker / em outra rede {#run-the-agent-in-docker--on-another-network}

O agente também é publicado como uma imagem de contêiner, com as mesmas
tags do servidor: `ghcr.io/laupernoe/dumbmonit-agent:latest` (último
lançamento), `:edge` (último commit em `main`). É uma imagem
`FROM scratch` mantendo o binário do agente e o pacote CA da Mozilla,
configurado inteiramente por variáveis de ambiente. O repositório entrega
um `docker-compose.agent.yml` pronto para uso:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

O que o contêiner vê, e o que montar:

| Necessidade | O que fazer | Por quê |
|---|---|---|
| CPU, memória, carga, E/S de disco do host | Nada | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` e `/proc/diskstats` são de todo o host dentro de qualquer contêiner. |
| Nome do host | `DUMBMONIT_AGENT_HOSTNAME` | O nome de host de um contêiner é um id aleatório; o dispositivo seria nomeado após ele. |
| Contêineres do host | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Opcional. Listas e inspeções somente leitura; ações Restart/Update precisam dele gravável e `DUMBMONIT_AGENT_COMMANDS=true`. Sem o socket, o agente simplesmente não reporta contêineres. |
| Uso de sistema de arquivos | `- /:/host:ro` (ou as montagens que importam) | O agente reporta as montagens visíveis *dentro* do contêiner, sob seu caminho de contêiner. |
| Contadores de rede do host | `network_mode: host` | Interfaces têm namespace: sem isso o agente vê um `veth`. |
| Monitores de ping retransmitidos por este agente | `cap_add: [NET_RAW]` | ICMP precisa de um socket bruto. |
| CA privada | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Servidor detrás de um proxy reverso interno, dispositivos com certificados autoassinados. |
| Manter o vínculo entre recriações | `- agent-state:/var/lib/dumbmonit-agent` e `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | O segredo de vínculo vive no sistema de arquivos do contêiner, que um `docker compose up --force-recreate` descarta. Sem o volume, o agente volta desvinculado e alguém precisa permitir o recadastramento da página do dispositivo a cada vez. |

`pid: host` e `privileged: true` nunca são necessários: o agente não
coleta métricas por processo.

### Modo relay: monitorar outro site {#relay-mode-monitor-another-site}

Um agente pode fazer mais que observar sua própria máquina. Com
`relay: true` (ou `DUMBMONIT_AGENT_RELAY=true`), o servidor **delega
sondagens** a ele: dispositivos cuja configuração *Alcançado através de*
nomeia este agente não são sondados pelo servidor mas pelo agente, a
partir de sua própria rede, com exatamente os mesmos coletores — SNMP,
Proxmox VE, Proxmox Backup Server, Synology, HTTP, TCP, DNS, ping, TLS.
Os resultados caem sob o dispositivo como se o servidor o tivesse
sondado: mesmas métricas, mesma *última sondagem*, mesmos alertas. Um
escritório remoto, um segundo homelab ou a rede de um cliente assim
aparecem no único DumbMonit que você já tem, sem uma VPN e sem nada para
abrir do lado remoto.

Como configurar isso, passo a passo, está em
[Monitorar um site remoto](../install/remote-site.md).

## Vínculo: uma máquina, um agente {#binding-one-machine-one-agent}

Um token de cadastro diz *esta máquina tem permissão para falar*. Não diz
*qual* máquina está falando — isso é para o que serve a chave de
identidade (`/etc/machine-id`, ou o nome de host), e essa chave não é
nenhum segredo: pode ser lida em qualquer máquina em segundos.

Então no primeiro lote o servidor dá a cada máquina um **segredo de
vínculo** próprio. Ele mantém só uma impressão digital; o agente escreve
o segredo em `/etc/dumbmonit/agent-secret` com modo `0600` e o apresenta
no cabeçalho `X-DumbMonit-Agent-Secret` em toda requisição a partir de
então — medições, comandos de contêiner e sondagens retransmitidas
igualmente. A partir de então:

- nenhuma outra máquina pode enviar medições sob este dispositivo, ou
  reescrever seu nome de host e sistema operacional;
- nenhuma outra máquina pode buscar — e portanto consumir — os comandos
  de Reiniciar e Atualizar enfileirados para ele.

A página do dispositivo mostra onde uma máquina está, sob **Agente →
Vínculo**:

| Estado | O que significa | O que fazer |
|---|---|---|
| **Vinculado** | O agente tem um segredo próprio. | Nada. |
| **Não vinculado — recadastre este host** | Um host cadastrado antes do vínculo existir, e nunca vinculado desde então. O servidor recusa suas medições, comandos de contêiner e sondagens retransmitidas. | Clique em **Permitir recadastramento** na página do dispositivo, depois reinicie o agente. Se o agente é anterior ao vínculo, execute o comando de instalação no host primeiro. |

### Hosts anteriores ao vínculo {#hosts-that-predate-binding}

!!! warning "Mudou: hosts não vinculados são recusados"
    Versões anteriores continuavam aceitando hosts cadastrados antes do
    vínculo existir, enquanto usassem o token que os cadastrou, e os
    vinculavam silenciosamente uma vez que seu agente fosse atualizado.
    Essa janela de transição está fechada: um host que nunca se vinculou
    agora é **recusado em toda rota de agente** — medições, comandos de
    contêiner e sondagens retransmitidas — até um administrador o
    recadastrar. Um token de frota é compartilhado por muitas máquinas e
    a chave de identidade pode ser adivinhada, então nada mais prova que
    o agente perguntando é o próprio do host.

    Um binário de agente muito antigo para receber um segredo de vínculo
    também é recusado, incluindo em seu primeiro cadastro.

Para trazer tal host de volta:

1. Se seu agente é anterior ao vínculo, execute o comando de instalação
   nele de novo (o binário do agente precisa conseguir armazenar um
   segredo de vínculo).
2. Abra sua página de dispositivo e clique em **Agente → Permitir
   recadastramento**. Isso abre uma janela de uma hora.
3. Reinicie o agente, ou espere por sua próxima tentativa: esse lote
   vincula o host com um segredo novo e fecha a janela. Um token
   diferente do que cadastrou o host ainda precisa poder cadastrar (não
   esgotado, não expirado).

Até então, o log do agente carrega a explicação do servidor:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Recadastramento depois de uma reinstalação {#re-enrolment-after-a-reinstall}

Se a máquina é reconstruída, seu disco substituído, ou seu contêiner de
agente recriado sem o volume que mantém o segredo, o agente volta sem
ele e o servidor recusa seus lotes. Essa recusa é deliberada: uma máquina
vinculada permanece vinculada até um humano dizer o contrário, senão o
vínculo não protegeria nada.

O caminho de volta é na página do dispositivo: **Agente → Permitir
recadastramento**. Abre uma janela de uma hora durante a qual um agente
apresentando um token de cadastro válido vincula essa máquina de novo,
com um segredo novo; o segredo antigo para de funcionar, e a janela
fecha assim que é usada. O agente tenta de novo por conta própria nesse
meio tempo, então não há nada para reiniciar na máquina — e nada se
perde, já que suas leituras permanecem em buffer até o link ser aceito de
novo.

Até alguém abrir essa janela, o log do agente diz o que fazer:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Tokens de cadastro {#enrolment-tokens}

- Tokens são criados em **Configurações → Agentes** ou ao adicionar um
  dispositivo do tipo agent. O servidor mantém só uma impressão digital:
  o token claro é mostrado uma vez, na resposta de criação, com os dois
  comandos de instalação.
- O agente o envia como `Authorization: Bearer dmon_…` em
  `POST /api/ingest`. Nunca aparece nos logs do agente, nem mesmo
  truncado.
- **Configurações → Agentes** lista tokens com seu prefixo, escopo, data
  de criação e último uso. **Revogar** para todo agente usando esse token
  em seu próximo envio; as máquinas então se tornam *inacessíveis* depois
  de três períodos perdidos. Execute o instalador de novo com um token
  novo para recadastrá-las.
- Máquinas são dispositivos separados: revogar um token não as exclui.

### Uso único ou frota {#single-use-or-fleet}

O escopo de um token é escolhido quando é criado, e é uma escolha, não um
padrão que acontece com você:

| Escopo | O que cadastra | Quando |
|---|---|---|
| **Uso único** (o padrão) | Exatamente uma máquina. | Uma instalação. Um comando de instalação sobrevive no histórico do shell, em um ticket, em um log de chat; um de uso único não vale nada uma vez usado. |
| **Reutilizável para uma frota** | Tantas máquinas quanto você permitir — um número, ou sem limite. | Um playbook, uma imagem de máquina, um lote de instalações. |

Ambos podem carregar um prazo em dias. Um prazo só interrompe
*cadastros*: máquinas já cadastradas continuam reportando depois que
passa, que é o que você quer para uma janela de instalação. Revogar é o
que corta uma frota.

Um token que não pode mais cadastrar — esgotado, ou passado seu prazo —
é mostrado como **Não cadastra mais** em Configurações → Agentes. Suas
máquinas continuam reportando; só não consegue deixar uma nova entrar.

## Indisponibilidades e diagnósticos {#outages-and-diagnostics}

Quando o servidor é inacessível, o agente continua medindo e armazena
suas leituras em memória com seus timestamps originais. São enviadas tão
logo o link volta: um reinício de servidor não deixa lacuna nos gráficos.
O buffer é limitado (`max_buffered_samples`, cerca de uma hora por
padrão) e descarta as leituras mais antigas quando cheio. Tentativas
recuam de 5 segundos a 5 minutos com jitter.

```sh
dumbmonit-agent --dry-run          # imprime as leituras, não envia nada
dumbmonit-agent --once             # envia um lote depois sai
journalctl -u dumbmonit-agent -f   # log do serviço (systemd)
tail -f /var/log/dumbmonit-agent.log   # log do serviço (OpenRC)
```

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| O instalador diz "could not reach the server" | URL ou token errados. A configuração é escrita: corrija `agent.yaml`, depois reinicie o serviço. |
| O download falha com 404 | O servidor não tem binários de agente (`DUMBMONIT_AGENT_DIR` vazio). Use `--bin=PATH` com um binário que você construiu. |
| O instalador para com "checksum mismatch" | O comando foi copiado antes de uma atualização do servidor (copie um novo de Configurações → Agentes), ou o binário recebido não é o que o servidor serve: um proxy ou cache no caminho, ou um download alterado. |
| O instalador para com "no expected checksum" | O comando não tem `--sha256` para esta plataforma: um comando antigo, ou um binário macOS (não entregue pelo servidor). Copie um comando novo, ou passe `--sha256=<platform>-<arch>:<hex>` do `.sha256` do lançamento. |
| A máquina nunca aparece | O envio vai para `/api/ingest` na URL do comando de instalação; detrás de um proxy reverso, certifique-se de que é encaminhado e que corpos de até 16 MB são permitidos. |
| *Inacessível* mesmo com o agente rodando | O token foi revogado, ou os envios são rejeitados. Verifique `journalctl -u dumbmonit-agent`: o erro é registrado lá. |
| O log diz "this agent is not recognised for this machine" | A máquina está vinculada a uma instalação de agente cujo segredo esta não tem — uma reinstalação, ou um contêiner recriado sem seu volume de estado. Abra a página do dispositivo e clique em **Permitir recadastramento**; veja [Recadastramento depois de uma reinstalação](#re-enrolment-after-a-reinstall). |
| O log diz "this machine was enrolled before agent binding" | Um host que nunca se vinculou a seu agente. Clique em **Permitir recadastramento** em sua página de dispositivo, depois reinicie o agente; veja [Hosts anteriores ao vínculo](#hosts-that-predate-binding). |
| O log diz "this agent is too old to be bound to its machine" | O binário do agente não consegue armazenar um segredo de vínculo. Execute o comando de instalação no host de novo; se o dispositivo já existe, clique em **Permitir recadastramento** primeiro. |
| O log diz "this enrollment token has already enrolled all the machines it was allowed to" | Um token de uso único sendo reutilizado. Crie um novo, ou um token reutilizável em Configurações → Agentes. |
| Windows | As partes do serviço Windows não tinham sido testadas na imagem de build do projeto no momento da escrita; reporte problemas no GitHub. |

## Contêineres Docker {#docker-containers}

Quando o agente pode ler o socket do Docker (`docker: true`, o padrão, e
um usuário no grupo `docker` ou root — `usermod -aG docker dumbmonit`,
depois reinicie o agente), todo contêiner se torna um punhado de séries,
rotuladas `container` e `image` (o `name:tag` mostrado por `docker ps`).
Se a página do dispositivo diz "Nenhum Docker nesta máquina" enquanto o
Docker está instalado, essa é a primeira coisa a verificar.

| Métrica | Significado |
|---|---|
| `container_up` | 1 em execução, 0 caso contrário |
| `container_health` | 0 nenhuma verificação de saúde, 1 saudável, 2 não saudável, 3 iniciando |
| `container_restart_count` | Contador de reinício do Docker para o contêiner |
| `container_started_seconds` | Segundos desde que o contêiner iniciou por último (0 quando parado) |
| `container_image_age_seconds` | Segundos desde que a imagem foi construída |
| `container_update_available` | 1 quando o registro tem uma imagem mais nova para a mesma tag, 0 quando atualizado, -1 desconhecido |

A verificação de atualização compara o digest da imagem local com o que
o registro anuncia para a tag (`HEAD /v2/<name>/manifests/<tag>`, fluxo
de token anônimo no Docker Hub e GHCR). Roda em segundo plano no máximo
uma vez por hora por imagem e nunca atrasa o ciclo de 30 segundos; um
registro privado que recusa acesso anônimo simplesmente lê como -1.
Desative com `docker_update_check: false`
(`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Regras integradas: **Container stopped** (`container_up == 0` por 2
minutos), **Container unhealthy** (`container_health == 2` por 3
minutos), **Container restarting** (três ou mais reinícios em 15
minutos), **Container update available** (info, depois de uma hora,
repetida diariamente). O alerta nomeia o contêiner.

### Ações {#actions}

Na página do dispositivo, cada contêiner tem duas ações e duas chaves.
Ações viajam através de um **canal de comando**: o servidor enfileira o
comando, o agente o busca depois de seu próximo lote
(`GET /api/agent/commands`, mesmo token de cadastro), o executa, e
reporta o resultado (`queued → running → done | failed`), com um log
curto que você pode expandir na interface. Comandos com mais de dez
minutos nunca são executados; o agente executa um comando por vez.

Um comando enfileirado pode ser **cancelado** da interface enquanto o
agente não o pegou. Um comando que ninguém buscou **expira** depois de
dez minutos, no servidor, independente de o agente alguma vez pesquisar
— então uma máquina cujo agente está parado nunca mantém um "Queued"
desatualizado para sempre, e nunca bloqueia a próxima ação naquele
contêiner.

O agente diz ao servidor, a cada lote, se aceita comandos. Quando não —
um agente mais antigo que o canal de comando, ou instalado com
`commands: false` — a página do dispositivo diz isso em vez de mostrar
Restart e Update, a API recusa os comandos (`409`), e políticas não
enfileiram nada. Reinstale o agente com o instalador atual para ativar
ações.

- **Restart** — `docker restart` para um contêiner em execução,
  `docker start` para um parado, depois uma verificação de 20 segundos
  de que roda.
- **Update now** — baixa a tag que o contêiner já usa, cria um contêiner
  novo com a mesma configuração (ambiente, montagens, portas, labels,
  redes, política de reinício), para o antigo, troca os nomes, inicia o
  novo e espera por sua verificação de saúde (ou 20 segundos mais uma
  verificação em execução quando a imagem não tem nenhuma). Se qualquer
  coisa falha, o agente **desfaz**: o contêiner novo é removido e o
  antigo reiniciado, e o comando reporta `failed` com o motivo. Em caso
  de sucesso o contêiner antigo é removido e, quando "prune" está
  ativado, a imagem antiga também — a menos que outro contêiner ainda a
  use.

Contêineres gerenciados por Compose (labels `com.docker.compose.*`) são
atualizados da mesma forma, mas o resultado avisa que o próximo
`docker compose up` recriará o contêiner a partir do arquivo compose.
Contêineres cujo nome começa com `dumbmonit` ou `dumbmonit` são
recusados: o monitor nunca se atualiza.

### Políticas {#policies}

Uma **política** por contêiner é avaliada pelo servidor todo minuto:

- **Reiniciar se parado** — um contêiner reportando `container_up == 0`
  é reiniciado automaticamente, no máximo uma vez a cada dez minutos.
- **Atualização automática em janelas de manutenção** — um contêiner com
  uma atualização disponível é atualizado automaticamente, mas só
  enquanto uma janela de manutenção (silêncio) cobre o dispositivo, no
  máximo uma vez a cada seis horas. Programe uma na página Alertas, ou
  silencie o dispositivo de sua página. Políticas assumem por padrão
  remover a imagem antiga e apenas janelas de manutenção.

Dois lugares as definem:

- a **faixa Docker** sob o cabeçalho do dispositivo: "28 contêineres ·
  27 em execução · 3 atualizações disponíveis · políticas: 2
  auto-reinício, 0 auto-atualização", a última ação e seu status,
  **Gerenciar contêineres** (abre a lista abaixo) e **Políticas…**, uma
  tabela de todo contêiner com as duas chaves e uma linha "Aplicar a
  todos" — a forma mais rápida de ativar auto-reinício para uma máquina
  inteira;
- as chaves dentro de cada linha da seção **Contêineres**, ao lado dos
  botões Restart e Update now.

Toda chave salva por conta própria
(`PUT /api/targets/<id>/containers/<name>/policy`); uma falha só diz
respeito a essa linha e é mostrada ao lado dela.

Ações automáticas aparecem em "Ações recentes" como solicitado por
`policy`.

### Regras de segurança {#safety-rules}

- O agente só age quando `commands: true` (o padrão); defina como
  `false` para uma máquina que deve só ser medida.
- O agente busca comandos ele mesmo, com seu token de cadastro — nada
  se conecta à máquina. Expiram depois de dez minutos e rodam um por
  vez; o agente nunca baixa uma tag diferente da que o contêiner já usa.
- Uma atualização que não volta saudável é desfeita para o contêiner
  anterior, e a imagem antiga só é removida uma vez que o contêiner novo
  roda.
- Contêineres chamados `dumbmonit*` ou `ezymonit*` são recusados, à mão
  ou por política: o monitor nunca age em seus próprios contêineres. A
  verificação é feita duas vezes — o servidor recusa enfileirar o
  comando, e o agente recusa executá-lo — e o agente a faz de novo no
  nome que o Docker reporta depois de inspecionar o contêiner, então
  nomear um por seu id não muda nada. O agente também recusa agir em seu
  próprio contêiner, como quer que se chame: ele não estaria lá para
  terminar o trabalho ou reportar sobre ele.
- Contêineres gerenciados por Compose são atualizados, mas o próximo
  `docker compose up` os recria a partir do arquivo compose: aumente a
  tag lá também.

## Backups Plakar {#plakar-backups}

O agente pode observar klosets [Plakar](https://plakar.io) (repositórios):
a cada dez minutos, em segundo plano, ele executa `plakar at <kloset> ls`
e `info` em cada um e reemite a última leitura a cada ciclo.

O Plakar é **detectado, nunca assumido**. Na inicialização e depois a
cada ciclo até encontrar algo, o agente procura o binário `plakar` (no
`PATH`, ou `plakar_bin`) e por klosets nos locais padrão. Quando não há
binário, nenhum kloset e nenhum traço de Plakar (`~/.cache/plakar`), o
agente não emite nenhuma série `backup_*` de forma alguma, avisa isso
uma vez em nível `debug`, e a página do dispositivo não mostra painel
Backups — nada para configurar em máquinas sem Plakar. Para pular a
detecção completamente:

```yaml
plakar: false                        # ou DUMBMONIT_AGENT_PLAKAR=false
```

### Descoberta {#discovery}

Nada para configurar: quando `plakar_klosets` está vazio, o agente
encontra os klosets ele mesmo, nesta ordem, e lê cada local apenas uma
vez:

1. o próprio diretório de configuração do agente —
   `$XDG_CONFIG_HOME/plakar`, ou `~/.config/plakar` da conta sob a qual
   roda (`plakar_home` quando definido);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, alfabeticamente;
4. `/var/lib/plakar`, quando esse diretório existe.

Em cada diretório lê `stores.yml` (ou `stores.yaml`; o legado
`plakar.yml` também é aceito) e toma toda entrada do mapa `stores`
(também `repositories` e `klosets`) que tem um `location`. Só o local é
lido: frases secretas e segredos rclone que ficam ao lado nunca são
mantidos nem registrados. Cada entrada se torna o kloset `@<name>`, lido
com `plakar -configdir <dir> at @<name>`, então um kloset criado por um
usuário é lido mesmo quando o agente roda como root. O `~/.plakar` de
toda conta é adicionado quando existe (o kloset padrão de
`plakar create`). Duas contas nomeando klosets diferentes da mesma forma
são distinguidas como `@name` e `<user>:@name`.

O resultado é registrado uma vez em `info` (e de novo só quando muda).
Dois gauges, emitidos só uma vez que o Plakar é detectado, deixam a
interface distinguir os casos restantes: `backup_plakar_present` (1
quando o binário roda) e `backup_klosets_found` (número de klosets
lidos). A página do dispositivo então diz "Plakar está instalado mas
nenhum kloset foi encontrado", ou lista os klosets. Quando klosets
existem mas o binário não roda (um `PATH` de serviço sem `plakar`,
tipicamente), são listados como ilegíveis com uma dica para definir
`plakar_bin`, e o agente avisa uma vez.

Para observar uma lista fixa em vez disso, nomeie-a — a descoberta é
então pulada:

```yaml
plakar: true                         # padrão: detecta Plakar; false o esconde completamente
plakar_bin: /usr/bin/plakar          # padrão: "plakar" no PATH
plakar_klosets:                      # lista explícita: substitui a descoberta
  - /srv/backups/main
  - "@diskext"                       # um store nomeado da conta do agente
plakar_home: /home/noe               # opcional: HOME (e diretório de config) usado ao executar plakar
plakar_interval_secs: 600            # mínimo 60
```

| Variável | Efeito |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — observa backups Plakar quando detectado (padrão `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Caminho do binário `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets, separados por vírgulas (substitui a descoberta) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Diretório home usado ao executar `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Segundos entre duas leituras |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — verificação de registro para atualizações de contêiner |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — aceitar ações do servidor |

Um kloset criptografado precisa de sua frase secreta: coloque-a na
entrada do store (`plakar store set <name> passphrase=…`, que é o que
`plakar store add` faz) ou exporte `PLAKAR_PASSPHRASE` no ambiente do
serviço do agente. Sem isso o kloset lê como "Não pode ser aberto"
(`backup_last_status = 0`).

### Métricas e regras {#metrics-and-rules}

Métricas, rotuladas `kloset` e `source` (o diretório que teve backup):
`backup_last_success_seconds` (idade do snapshot mais novo),
`backup_snapshot_count`, `backup_last_status` (1 quando o snapshot mais
novo não tem erro, 0 quando tem erros ou o kloset não pode ser aberto),
`backup_size_bytes{kloset}` (tamanho de armazenamento do repositório),
mais `backup_plakar_present` e `backup_klosets_found` descritos acima.
Nenhuma delas existe enquanto o Plakar não é detectado, então as regras
integradas abaixo não podem disparar em uma máquina sem Plakar.

Regras integradas: **Plakar backup too old** (aviso, nenhum snapshot por
mais de dois dias) e **Plakar backup failed** (alerta). A página do
dispositivo mostra um bloco por kloset — idade do último backup,
contagem de snapshot, tamanho, último status — e uma linha por fonte,
com uma dica "Backup too old" depois de dois dias.

### Executar um primeiro backup {#run-a-first-backup}

Com o Plakar 1.1 instalado na máquina (`plakar version`), como o usuário
que possuirá os backups:

```sh
# 1. um store de kloset, nomeado para que o agente o encontre em ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # ou qualquer local que o Plakar aceite
plakar at @diskext create                                   # adicione -plaintext para pular criptografia

# 2. um primeiro backup, depois verifique-o
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` e `plakar at <path> backup <dir>` funcionam da
mesma forma com um diretório simples em vez de `@name`; `plakar create`
sem `at` usa `~/.plakar`, que o agente também encontra. Agende
`plakar at @diskext backup /srv/photos` com cron ou um timer systemd: o
agente reporta a idade do snapshot mais novo, e alerta quando um dia
passa sem um — dois dias para a regra integrada.

Para experimentar localmente com um kloset não criptografado:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

depois adicione `plakar_klosets: ["/tmp/plakar-demo"]` a `agent.yaml`
(um caminho simples fora de um home não é descoberto) e verifique com
`dumbmonit-agent --dry-run` que amostras `backup_*` aparecem.

## Backups restic e Borg {#restic-and-borg-backups}

O agente pode observar repositórios [restic](https://restic.net) e
[Borg](https://www.borgbackup.org): a cada dez minutos, em segundo
plano, ele abre cada repositório que você declara e reporta se pôde ser
lido e quão antigo é seu snapshot mais novo. Nada é descoberto: um
repositório não tem local padrão e não pode ser aberto sem sua senha,
então cada um é listado em `agent.yaml`, com a forma de obter sua senha.

**A senha nunca sai da máquina.** O agente a entrega ao `restic` ou
`borg` só através do ambiente do processo filho; nunca é registrada,
nunca enviada ao servidor, e as séries só carregam o nome do
repositório.

```yaml
restic_repos:
  - name: nas                                   # o rótulo "repo"; padrão: o repositório, sem credenciais
    repository: sftp:backup@nas.lan:/srv/restic  # passado como RESTIC_REPOSITORY, nunca na linha de comando
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # opcional: apenas os snapshots deste host (--host), para um repositório compartilhado
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # ou qualquer comando que a imprima (RESTIC_PASSWORD_COMMAND)
    env:                                        # ambiente extra apenas para este repositório
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # passado como BORG_REPO
    password_file: /etc/dumbmonit/borg-offsite.pass        # lido pelo agente, passado como BORG_PASSPHRASE
    glob_archives: "web-01-*"                   # opcional: apenas estes arquivos (--glob-archives)
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # padrão: no PATH
borg_bin: borg
backup_repos_interval_secs: 600                 # mínimo 60
```

Mantenha `agent.yaml` e os arquivos de senha legíveis só por root
(`chmod 600`), como o instalador faz para `agent.yaml`.
`password_command` mapeia para `RESTIC_PASSWORD_COMMAND` ou
`BORG_PASSCOMMAND`. Qualquer coisa que as ferramentas leem de seu
ambiente pode ir sob `env` (`RESTIC_PASSWORD`, `BORG_PASSPHRASE`, chaves
de nuvem, `BORG_RSH`…), e o próprio ambiente de serviço do agente também
é passado.

O que o agente executa, somente leitura no sentido mais estrito:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: nenhum
  bloqueio é escrito, então um repositório somente leitura ou
  append-only pode ser observado, e um `prune` mantendo o bloqueio
  exclusivo não faz o repositório parecer fora do ar; nenhum cache
  também, já que o serviço instalado não pode escrever em `/root`.
- `borg --bypass-lock list --json --last 1`: sem `--bypass-lock`, a
  leitura falharia por toda a duração de todo `borg create`, que
  mantém o bloqueio exclusivo. O Borg escreve um pequeno registro de
  segurança sob `~/.config/borg`; quando esse home não é gravável (o
  serviço instalado roda com `ProtectHome=yes`), o agente aponta
  `BORG_BASE_DIR` para seu `/tmp` privado em vez disso, a menos que você
  defina `BORG_BASE_DIR` você mesmo. Um repositório em modo `keyfile`
  então precisa de `BORG_KEY_FILE` em seu `env`.

Como o serviço roda com `ProtectHome=yes`, mantenha o que as ferramentas
precisam fora dos diretórios home: arquivos de senha, chaves SSH e
`known_hosts` sob `/etc/dumbmonit/`, como no exemplo, e repositórios
locais sob `/srv`, `/mnt` ou `/var` em vez de `/home`. As leituras rodam
uma depois da outra, dentro do limite de memória do serviço (128 MiB
com o próprio agente).

Métricas, rotuladas `tool` (`restic` ou `borg`) e `repo`:
`agent_backup_repo_reachable` (1 quando a última leitura teve sucesso, 0
caso contrário: repositório inacessível, senha recusada, binário
faltando — o agente registra o motivo uma vez, e de novo quando muda),
`agent_backup_repo_last_snapshot_age_seconds` (idade do snapshot mais
novo, ausente enquanto o repositório está vazio), e
`agent_backup_repo_last_snapshot_bytes` (bytes que o snapshot mais novo
do restic processou, restic 0.17 e posteriores). Com `--latest 1`, o
restic retorna o snapshot mais novo de cada conjunto host e caminho; o
mais novo de todos é mantido, então defina `host` em um repositório
compartilhado por várias máquinas.

Regras integradas: **restic/Borg backup too old** (nenhum snapshot por
mais de 48 horas) e **restic/Borg repository unreachable** (por 30
minutos). A página do dispositivo mostra um painel Repositórios de
backup, uma linha por repositório.

Para verificar uma declaração, execute o agente uma vez em primeiro
plano e procure pelas amostras `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Túneis WireGuard {#wireguard-tunnels}

Em uma máquina que roda WireGuard, o agente lê toda interface e todo
peer com `wg show all dump`, a cada ciclo. Nada para configurar: sem o
comando `wg` (wireguard-tools) ou sem uma interface, nenhuma série é
emitida e nenhum painel mostrado. O `wg` precisa de root
(`CAP_NET_ADMIN`); o serviço instalado o tem, um agente iniciado
manualmente sob sua própria conta registra a recusa uma vez e não
reporta nada.

O dump também mantém a chave privada da interface e a chave pré-
compartilhada de cada peer: o agente pula ambos os campos sem jamais
copiá-los.

Por peer, rotulado `interface`, `peer` (sua chave pública) e
`allowed_ips`: `agent_wireguard_peer_handshake_age_seconds` (segundos
desde o handshake mais recente), `agent_wireguard_peer_has_handshake` (0
enquanto o peer nunca completou um), `agent_wireguard_peer_keepalive_seconds`
(seu keepalive persistente, 0 quando desligado), e os contadores
`agent_wireguard_peer_rx_bytes` e `agent_wireguard_peer_tx_bytes`; por
interface, `agent_wireguard_interface_peers`. Para um peer que nunca
completou um handshake, a idade conta a partir do momento em que o
agente o viu primeiro: um túnel que nunca subiu também é silencioso, e é
reportado como tal.

O WireGuard não tem nomes de peer. Dê os seus em `agent.yaml`, por chave
pública, e eles se tornam o rótulo `name`:

```yaml
wireguard: true                     # padrão; false para até de olhar
wireguard_bin: wg                   # padrão: no PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

A regra integrada **WireGuard peer silent** dispara quando um peer
**com um keepalive persistente** não completou um handshake por mais de
15 minutos. Enquanto um túnel está ativo, o WireGuard renova seu
handshake a cada dois minutos, e um keepalive o mantém ativo mesmo sem
tráfego; um peer sem keepalive (um celular, um laptop) pode ficar
inativo por dias sem nada quebrado, então é deixado de fora. Defina
`PersistentKeepalive = 25` nos túneis que precisam permanecer ativos
(links site-to-site, gateways remotos). N é o limite da regra: mude-o em
**Alertas → Regras**, ou remova a parte `and on (…)` de sua consulta
para observar todo peer. A página do dispositivo mostra um painel
WireGuard com cada peer, o tempo desde seu último handshake, seu
tráfego e seu keepalive.

## Certificados TLS (Certbot / Let's Encrypt) {#tls-certificates-certbot--lets-encrypt}

O agente não lê `/etc/letsencrypt` diretamente. Todo certificado que o
Certbot renova é servido em algum lugar — um servidor web, um servidor de
e-mail, um proxy reverso — e é ali que o DumbMonit o lê: adicione um
monitor de [certificado TLS](services.md#tls) (ou um
[Site ou API web](services.md#http) para um site já observado desse
jeito) apontado para a porta em que o certificado é realmente
apresentado. Ele lê o certificado ativo, não um arquivo em disco, então
também pega uma renovação que teve sucesso em `/etc/letsencrypt` mas
nunca foi detectada pelo serviço (um proxy reverso que precisa de um
recarregamento para ver o novo arquivo é o caso habitual, e exatamente a
lacuna que importa).

Ler `/etc/letsencrypt/live/*/cert.pem`, a configuração de renovação e o
log do Certbot a partir do agente foi considerado e deixado de lado:
duplicaria a data de expiração que o monitor TLS já reporta, enquanto
adicionaria um segundo caminho, local ao host, para a mesma informação —
mais uma coisa para manter sincronizada depois que um certificado é
movido ou um domínio renomeado. Um host onde o Certbot roda mas não
serve nada que o DumbMonit possa alcançar diretamente (um passo de CA
interna, um hook de validação DNS) é o único caso que isso deixa
descoberto; é também o caso em que um monitor TLS não poderia ajudar de
qualquer forma, já que nada nesse host fala TLS para sondar.
