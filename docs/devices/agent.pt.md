# Agente Linux, macOS, FreeBSD e Windows {#linux-macos-freebsd-and-windows-agent}

Um pequeno binário instalado na máquina a monitorizar. Mede CPU, memória, discos,
rede, serviços e contentores — e, quando a máquina os expõe, temperaturas, saúde
dos discos e pools ZFS — e depois **envia** as suas leituras para o servidor
DumbMonit por HTTP.

A direção é deliberada: o agente chama o servidor, nunca o contrário. Atravessa
NAT e firewalls domésticas, e não exige abrir **nenhuma porta** na máquina
monitorizada.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_`. As etiquetas de identidade
(`target`, `host`, `tag_*`) são definidas pelo servidor na receção, nunca pelo
agente: uma máquina não pode escrever nas séries de outra, nem sequer forjando as
suas etiquetas.

| Família | Métricas | Etiquetas |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` apenas com `cpu_per_core: true` | `core` |
| Memória | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Sistemas de ficheiros | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Rede (contadores) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Discos (contadores) | `disk_read_bytes`, `disk_written_bytes`, uma série por dispositivo de blocos | `device` |
| Anfitrião | `uptime_seconds`, `process_count` | |
| Serviços | `service_up` (1 = em execução) | `service` |
| Contentores | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (contentores além de `docker_max_containers`) | `container`, `image` |
| Sensores | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (o limiar que o próprio hardware declara), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Saúde dos discos (SMART) | `agent_disk_smart_ok` (1 = o disco passa a sua própria autoavaliação), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degraded, 2 pior), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; por peer `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (contadores) — veja [Túneis WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — veja [Cópias de segurança restic e Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Contadores de desempenho do Windows | `agent_perf_counter`, uma série por contador listado em `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — veja [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; por VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; por disco virtual `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — veja [Anfitriões Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Agente | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Uma série que não pode ser medida está **ausente**, nunca a zero: sem `smartctl`,
sem `agent_disk_*`; sem sonda legível, sem `agent_sensor_*`; sem ZFS, sem
`agent_zfs_*`. Uma temperatura fora de 1–150 °C é descartada em vez de publicada,
e uma ventoinha a ler 0 rpm também é descartada — uma placa-mãe tem mais ligações
de ventoinha do que ventoinhas, e uma vazia lê zero como uma ventoinha morta.

### O que é recolhido onde {#what-is-collected-where}

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, memória, swap, sistemas de ficheiros, rede, tempo de atividade, processos | sim | sim | sim | sim |
| Carga média | sim | sim | sim | não (o Windows não tem essa noção) |
| Serviços | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | serviços do Windows |
| Contentores Docker | sim | sim, se `docker_socket` apontar para o socket do Docker Desktop | não (sem daemon Docker) | não |
| Temperaturas | `/sys/class/hwmon` | sensores SMC | `dev.cpu.N.temperature` (só CPU) | não |
| Ventoinhas | `/sys/class/hwmon` | não | não | não |
| Saúde dos discos (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Pools ZFS | `zpool` (OpenZFS) | não | `zpool` | não |
| Saúde do SO (atualizações, reinício, units falhadas, SELinux) | sim | não | não | não |
| Cópias de segurança Plakar | sim | sim | sim | sim |
| Túneis WireGuard | `wg show` | quando o `wg` está instalado (não testado) | quando o `wg` está instalado (não testado) | não |
| Cópias de segurança restic e Borg | sim | sim | sim | apenas restic (não testado; o Borg não tem build para Windows) |
| Contadores de desempenho, filas MDaemon, Hyper-V | não | não | não | sim |
| Identidade da máquina | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | nome do anfitrião |
| Modo relay | sim | sim | sim | sim |

O Windows não tem linha de temperaturas de propósito: ler sensores aí significa
passar pelo WMI, com direitos administrativos, para valores que a maioria das
máquinas nem sequer expõe. Não publicar nada é a resposta honesta.

O `smartctl` (smartmontools) e o `zpool` são procurados em cada período de leitura
e usados quando presentes; nenhum é instalado pelo agente, e nenhum é obrigatório.
Ambos precisam de root — o serviço que o instalador regista corre como root, um
agente iniciado à mão na sua própria conta não encontrará nada.

Os contadores são enviados em bruto, para que um reinício do agente nunca produza
uma taxa falsa; aplique `rate()` nas consultas.

Regras incorporadas que se aplicam: Device unreachable, High CPU, Unusual CPU.
«Device unreachable» também funciona com agentes: o servidor
verifica a idade do último lote recebido e declara a máquina silenciosa após três
períodos falhados (pelo menos 90 segundos).

Mais quatro disparam apenas em máquinas que produzem as séries correspondentes,
pelo que não precisam de configuração e não podem disparar numa máquina sem o
hardware:

| Regra | Dispara quando |
|---|---|
| **Disk SMART failing** (critical) | a autoavaliação de um disco o reporta como a falhar |
| **ZFS pool degraded** (critical) | uma pool deixa de estar `ONLINE` durante cinco minutos |
| **ZFS scrub found errors** (critical) | o último scrub encontrou erros, ou a pool reporta ficheiros permanentemente corrompidos |
| **Temperature above critical** (critical) | um sensor excede o limiar crítico que **o seu próprio hardware declara** — nenhum valor é codificado, sendo 85 °C uma emergência num disco e uma tarde normal num CPU de portátil |

Mais três vigiam o que você declara ou executa, e ficam em silêncio caso
contrário: **WireGuard peer silent** (um peer com keepalive persistente e sem
handshake há 15 minutos), **restic/Borg backup too old** (sem snapshot há 48
horas) e **restic/Borg repository unreachable** — veja
[Túneis WireGuard](#wireguard-tunnels) e
[Cópias de segurança restic e Borg](#restic-and-borg-backups).

Num servidor Windows que corre MDaemon, mais três vigiam as suas filas:
**MDaemon mail queue growing**, **MDaemon Bad queue not empty** e **MDaemon Retry
queue high** (veja [Regras](../alerting/rules.md#mdaemon-and-securitygateway)).
Num anfitrião Hyper-V, três vigiam as máquinas virtuais: **Hyper-V VM health
critical**, **Hyper-V host CPU high** e **Hyper-V virtual disk errors** (veja
[Anfitriões Hyper-V](#hyper-v-hosts)).

## Instalar {#install}

Os passos abaixo são os que o aviso junto ao formulário mostra. Não há conta a
criar na máquina: o token de registo é o único segredo.

1. Guarde o dispositivo no DumbMonit: é mostrado uma vez um token de registo (dmon_…), com o comando de instalação pronto a copiar para Linux e para Windows. (Um token pode também ser criado em **Definições → Agentes**.)
2. Esse token é a chave do agente para enviar as suas medições ao DumbMonit. Não é uma conta na máquina: nada a criar lá. Um token criado a partir do formulário do dispositivo é de uso único: regista esta máquina e mais nenhuma. Para uma frota, crie um token reutilizável em Definições → Agentes.
3. Execute o comando de instalação na máquina a monitorizar, com direitos elevados (sudo no Linux e macOS, um PowerShell elevado no Windows). Descarrega o agente, escreve o token em agent.yaml e inicia o serviço.
4. A máquina aparece sozinha em poucos segundos, com o nome do seu anfitrião. Nesse primeiro lote o servidor dá ao agente um segredo próprio e guarda-o em `/etc/dumbmonit/agent-secret`, legível por mais ninguém: a partir daí, essa máquina é a única que pode reportar sob este dispositivo. Perdeu o token? Definições → Agentes permite revogá-lo e criar outro.
5. Docker: para ver os contentores e deixar o DumbMonit reiniciá-los ou atualizá-los, o agente tem de alcançar o socket do Docker. O serviço que o instalador regista já o consegue; se executar o agente sob um utilizador dedicado, adicione esse utilizador ao grupo «docker» e reinicie-o. As políticas de reinício e de atualização automática são então definidas por contentor na página do dispositivo.

Duas coisas valem a pena ser lidas antes de implementar isto em mais do que uma
máquina: [Associação](#binding-one-machine-one-agent), que é o que faz um
dispositivo pertencer realmente a uma instalação de agente, e
[Uso único ou frota](#single-use-or-fleet), que é a escolha que faz quando cria
um token.

    ```
    usermod -aG docker dumbmonit
    ```

6. Cópias de segurança Plakar: detetadas automaticamente — o painel Backups aparece quando o binário plakar ou um kloset (~/.config/plakar/stores.yml de cada utilizador, ~/.plakar, /var/lib/plakar) é encontrado, e nada é mostrado caso contrário. Defina «plakar_klosets» em agent.yaml para vigiar uma lista fixa, ou «plakar: false» para excluir.

!!! note
    O agente contacta o servidor, nunca o contrário: não é preciso abrir nenhuma
    porta na máquina monitorizada.

=== "Linux (systemd ou OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    O script é `sh` POSIX (BusyBox e dash funcionam), descarrega do servidor o
    binário para a arquitetura da máquina (x86_64 ou aarch64), escreve
    `/etc/dumbmonit/agent.yaml` (modo 0600) e regista `dumbmonit-agent` no
    systemd, ou no OpenRC no Alpine. A unit do systemd é endurecida
    (`ProtectSystem=strict`, `NoNewPrivileges`, `MemoryMax=128M`,
    `CPUQuota=20%`).

=== "macOS (launchd)"

    O mesmo comando que no Linux, executado com `sudo`:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Os binários macOS **não** estão na imagem do DumbMonit: compilá-los exige o
    SDK da Apple, cuja licença proíbe redistribuir. Estão anexados a cada
    [versão no GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
    — `dumbmonit-agent-macos-aarch64` (Apple silicon) e
    `dumbmonit-agent-macos-x86_64` (Intel), cada um com o seu `.sha256` — e o
    servidor redireciona `/download/dumbmonit-agent-macos-…` para lá, pelo que o
    script escolhe o da arquitetura do Mac (`uname -m`). O servidor não fornece
    esses binários, por isso o comando de instalação não traz soma de
    verificação para eles: acrescente `--sha256=macos-aarch64:<hex>` (ou
    `macos-x86_64`) com o valor do `.sha256` da versão, ou o instalador recusa o
    download. O Mac precisa também de alcançar github.com.

    Sem esse acesso, ou para instalar um binário que já tem, descarregue-o e
    passe-o com `--bin` (compare-o primeiro com o `.sha256` da versão: `--bin`
    instala o que lhe der):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    O script instala o binário em `/usr/local/bin` (e remove o atributo de
    quarentena que um download pelo navegador traz, o que faria o Gatekeeper
    recusar iniciá-lo), escreve `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600)
    e regista o daemon de sistema `com.dumbmonit.agent` em
    `/Library/LaunchDaemons`, iniciado com `launchctl bootstrap system`. Corre no
    arranque, sem ninguém com sessão iniciada.

    !!! note "Prefere compilá-lo você?"
        No próprio Mac, com o Rust instalado:
        `cargo build --release -p dumbmonit-agent`, depois
        `--bin=target/release/dumbmonit-agent`. É o mesmo binário que a tarefa
        de lançamento produz.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Apenas x86_64. O script descarrega `dumbmonit-agent-freebsd-x86_64` do
    servidor, escreve `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e instala
    `/usr/local/etc/rc.d/dumbmonit_agent`, ativado com
    `service dumbmonit_agent enable`. O `daemon(8)` supervisiona o agente,
    reinicia-o se parar e escreve `/var/log/dumbmonit-agent.log`.

    Funciona da mesma forma no TrueNAS CORE e noutros appliances baseados em
    FreeBSD, onde as séries ZFS são o objetivo do exercício.

=== "Windows (serviço)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    O script descarrega `dumbmonit-agent-windows-x86_64.exe` do servidor,
    verifica-o contra o seu SHA-256, instala-o como
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, escreve
    `C:\ProgramData\DumbMonit\agent.yaml` (legível apenas por SYSTEM e
    Administrators) e regista o serviço Windows `DumbMonitAgent` («DumbMonit
    system agent»): iniciado automaticamente no arranque, como LocalSystem, e
    reiniciado após uma falha (10 s, 30 s, depois 60 s). O agente fala ele próprio
    com o gestor de serviços (`--service`), pelo que parar o serviço lhe permite
    enviar o que ainda guarda antes de sair.

    Um serviço não tem consola: o agente escreve o seu registo em
    `C:\ProgramData\DumbMonit\agent.log`, posto de parte como `agent.log.old`
    acima de 10 MB.

    Não há build nativa ARM64: no Windows 11 para ARM, o script instala o
    binário x86_64, que corre sob emulação x64.

Um token pode registar várias máquinas: dê-lhe o nome de um grupo ou de uma
máquina.

### Binários nativos {#native-binaries}

O agente é um único executável estático por sistema, sem nada para instalar ao
lado:

| Sistema | Ficheiro | Onde obtê-lo |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | o servidor, `/download/…` |
| Linux aarch64 (ARM de 64 bits: Raspberry Pi OS 64 bits, servidores ARM) | `dumbmonit-agent-linux-aarch64` | o servidor, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | o servidor, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | o servidor, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | a versão no GitHub (o servidor redireciona para lá) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | a versão no GitHub (o servidor redireciona para lá) |

Os binários Linux são ligados estaticamente contra musl: correm em qualquer
distribuição, glibc ou não (Alpine, firmwares de NAS), sem biblioteca a
corresponder. Cada ficheiro tem a sua soma de verificação ao lado,
`<file>.sha256`, em formato `sha256sum`. Cada
[versão no GitHub](https://github.com/laupernoe/dumbmonit/releases/latest) traz
os seis binários e as suas somas de verificação, para máquinas que não conseguem
alcançar o `/download/` do servidor ou para instalar com `--bin`.

Para compilar você próprio os binários Linux, Windows e FreeBSD, a partir de um
clone do repositório (o Docker é o único requisito):

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

`--uninstall` (`-Uninstall` no Windows) remove tudo isto, serviço incluído, em
todos os sistemas.

!!! note "O download é verificado — sempre"
    O comando de instalação mostrado pelo DumbMonit traz o SHA-256 esperado de
    cada binário de agente que o servidor fornece
    (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` no Windows). Ambos os instaladores verificam o
    ficheiro descarregado contra ele antes de instalar o que quer que seja, e
    **recusam** instalar quando não corresponde, quando não é dada soma de
    verificação para a plataforma, ou quando nenhuma ferramenta SHA-256
    (`sha256sum`, `shasum`, `sha256`) está disponível.

    As somas de verificação mudam a cada versão do servidor: após atualizar o
    DumbMonit, copie um comando de instalação novo de **Definições → Agentes**
    (ou de um token novo) em vez de voltar a executar um antigo, que pararia com
    «checksum mismatch». As mesmas somas de verificação são mostradas sob o
    comando de instalação, e publicadas em `/download/<file>.sha256`.

    Para configurações isoladas da rede ou manuais, `--insecure-skip-checksum`
    (`-InsecureSkipChecksum`) instala um binário descarregado sem verificação e
    di-lo em voz alta; `--bin=PATH` instala um ficheiro local tal como está,
    cabendo a si verificá-lo.

### Opções do instalador {#installer-flags}

| `install.sh` | `install.ps1` | Efeito |
|---|---|---|
| `--token=TOKEN` | `-Token` | Token de registo (obrigatório). |
| `--url=URL` | `-Url` | URL do servidor, por exemplo `http://server:8080`. No Linux, `DUMBMONIT_URL` no ambiente é usado se a flag estiver ausente. |
| `--interval=N` | `-Interval` | Período de amostragem em segundos (predefinição 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Serviços cujo estado é reportado, nomeados como o sistema anfitrião os nomeia: units systemd no Linux, labels launchd (`com.apple.sshd`) no macOS, nomes rc.d no FreeBSD, serviços do Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Tags, copiadas como `tag_<key>` em cada série. |
| `--hostname=NAME` | `-HostName` | Nome anunciado ao servidor (predefinição: o da máquina). |
| `--bin=PATH` | `-BinPath` | Binário local a instalar em vez de o descarregar. |
| `--sha256=LIST` | `-Sha256` | SHA-256 esperado do download, como pares `platform-arch:hex` separados por vírgulas (um valor hex simples também serve). Incluído no comando que o DumbMonit mostra; obrigatório a menos que a flag seguinte seja dada. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Instalar um binário descarregado sem o verificar, com um aviso. Apenas para configurações isoladas da rede ou manuais. |
| `--no-start` | | Instalar tudo, mas não contactar o servidor nem iniciar o serviço (imagem de máquina, testes). |
| `--uninstall` | `-Uninstall` | Desinstalar o agente e apagar a sua configuração. |
| `--help` | | Mostrar a ajuda. |

### Atualizar {#upgrade}

Ambos os scripts são idempotentes: voltar a executar o comando de instalação, com
o mesmo token e URL, substitui o binário, reescreve a configuração e reinicia o
serviço. É esse o procedimento de atualização. Tome o comando do servidor atual
(Definições → Agentes): o seu `--sha256` tem de corresponder aos binários da
versão do servidor para que atualizou.

O mesmo comando migra uma máquina que ainda corre o agente EzyMonit, no Linux e no
Windows. O instalador pára e remove o serviço antigo (`ezymonit-agent`, ou
`EzyMonitAgent` no Windows), move a configuração para o novo caminho
(`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`) e instala
`dumbmonit-agent` no seu lugar. O antigo token `ezym_…` continua válido, pelo que
o comando mostrado quando o token foi criado ainda funciona; nada tem de ser
revogado primeiro.

## Ficheiro de configuração {#configuration-file}

`/etc/dumbmonit/agent.yaml` no Linux, `/usr/local/etc/dumbmonit/agent.yaml` no
macOS e FreeBSD, `C:\ProgramData\DumbMonit\agent.yaml` no Windows:

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # sampling period
hostname: nas-basement     # optional: name announced to the server
services:                  # systemd units or Windows services
  - sshd
  - docker
tags:                      # free labels, prefixed "tag_" on the server
  role: nas
  room: basement
docker: true               # container inventory (default: true)
docker_socket: /var/run/docker.sock
sensors: true              # temperatures and fans (default: true)
smart: true                # disk health through smartctl (default: true)
smart_bin: smartctl        # path, when it is not in the service's PATH
smart_interval_secs: 300   # minimum 60: a sleeping disk is never woken to be read
zfs: true                  # ZFS pools through zpool (default: true)
zfs_bin: zpool
zfs_interval_secs: 120     # minimum 30
wireguard: true            # WireGuard tunnels through wg (default: true), see below
restic_repos: []           # restic repositories to watch, see "restic and Borg backups"
borg_repos: []             # Borg repositories to watch
mdaemon: auto              # Windows: MDaemon queues when its service exists (true / false / auto)
hyperv: auto               # Windows: Hyper-V counters when the vmms service exists (true / false / auto)
perf_counters:             # Windows: performance counters to publish
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # value of the "counter" label; the path when omitted
max_buffered_samples: 20000
log_level: info
```

`sensors`, `smart` e `zfs` são detetados, nunca assumidos: deixados a `true`
numa máquina sem sonda, sem `smartctl` ou sem pool, custam uma olhadela por
período e não emitem nada. Defina um a `false` para deixar de sequer olhar — vale
a pena num NAS cujos discos preferia que o `smartctl` nunca tocasse, embora o
agente já passe `-n standby` para que um disco a dormir só seja medido quando
está acordado por outras razões.

Ao lado, `/etc/dumbmonit/agent-secret` (`C:\ProgramData\DumbMonit\agent-secret`
no Windows) guarda o segredo de associação que o servidor deu a esta máquina. O
agente escreve-o com modo `0600` e relê-o em cada arranque; não faz parte de
`agent.yaml` porque não é algo que você defina — veja
[Associação](#binding-one-machine-one-agent). `DUMBMONIT_AGENT_SECRET_FILE`
move-o, que é o que quer para um agente num contentor.

Cada chave pode ser substituída pelo ambiente, o que torna o agente utilizável
num contentor sem montar um ficheiro:

| Variável | Efeito |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Caminho do ficheiro de configuração |
| `DUMBMONIT_AGENT_URL` | URL do servidor |
| `DUMBMONIT_AGENT_TOKEN` | Token de registo |
| `DUMBMONIT_AGENT_SECRET_FILE` | Onde guardar o segredo de associação (predefinição: `agent-secret`, ao lado do ficheiro de configuração) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Período de amostragem |
| `DUMBMONIT_AGENT_HOSTNAME` | Nome anunciado ao servidor |
| `DUMBMONIT_AGENT_SERVICES` | Serviços a vigiar, separados por vírgulas |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, separados por vírgulas |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Caminho do socket do Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Contentores detalhados por anfitrião, predefinição `200` (`0`: apenas contagens) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfaces deixadas de fora, nomes ou regex separados por vírgulas; predefinição `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfaces a manter; quando definida, substitui a lista de ignoradas |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Pontos de montagem deixados fora dos sistemas de ficheiros e da E/S de disco; a predefinição ignora `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` para enviar também uma série de CPU por núcleo (predefinição `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: temperaturas e ventoinhas (predefinição `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: saúde dos discos através do `smartctl` (predefinição `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Caminho do `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Período entre duas leituras SMART, predefinição `300`, mínimo `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: pools ZFS através do `zpool` (predefinição `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Caminho do `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Período entre duas leituras ZFS, predefinição `120`, mínimo `30` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: túneis WireGuard através do `wg` (predefinição `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Caminho do `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Caminho do `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Caminho do `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Período entre duas leituras dos repositórios restic e Borg, predefinição `600`, mínimo `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` ou `auto`: contadores de desempenho do MDaemon (predefinição `auto`, lidos quando o serviço Windows `MDaemon` existe) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` ou `auto`: contadores de desempenho do Hyper-V (predefinição `auto`, lidos quando o serviço Windows `vmms` existe) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Caminhos de contadores de desempenho do Windows, separados por **ponto e vírgula** (uma vírgula pode fazer parte do nome de uma instância); substitui `perf_counters` |
| `DUMBMONIT_AGENT_SYSTEM_HEALTH` | `true` / `false`: saúde do sistema operativo, o interruptor geral dos quatro seguintes (predefinição `true`) |
| `DUMBMONIT_AGENT_CHECK_UPDATES` | `true` / `false`: atualizações pendentes (predefinição `true`) |
| `DUMBMONIT_AGENT_UPDATES_INTERVAL_SECS` | Período entre duas verificações de atualizações pendentes, predefinição `3600`, mínimo `300` |
| `DUMBMONIT_AGENT_CHECK_REBOOT` | `true` / `false`: reinício necessário (predefinição `true`) |
| `DUMBMONIT_AGENT_CHECK_FAILED_UNITS` | `true` / `false`: units systemd falhadas (predefinição `true`) |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Tamanho do buffer de recuperação |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` para executar, para o servidor, as sondas dos dispositivos atribuídos a este agente (veja [Executar o agente no Docker / noutra rede](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Etiqueta do site mostrada ao lado do agente quando é oferecido como relay |

## Contadores de desempenho do Windows {#windows-performance-counters}

No Windows, o agente lê contadores de desempenho — os valores que o Monitor de
Desempenho (`perfmon`) mostra — através da biblioteca PDH, e publica-os a cada
período de amostragem. Algum software não publica o seu estado em mais lado
nenhum.

**MDaemon.** Quando existe o serviço Windows chamado `MDaemon`, o agente lê o
objeto de desempenho do MDaemon: tamanhos de filas, sessões, totais de mensagens
e de spam, o estado dos servidores internos do MDaemon e o seu tempo de
atividade. As séries, as regras e a página que as mostra estão descritas na
página do [MDaemon](mdaemon.md#mail-queues-through-the-agent). `mdaemon: true`
lê-as mesmo sem o serviço (um serviço renomeado), `mdaemon: false` nunca o faz. A
presença do serviço é verificada de novo a cada dez períodos, pelo que um MDaemon
instalado depois do agente é apanhado sem reinício.

**Hyper-V.** Quando existe o serviço Hyper-V Virtual Machine Management (`vmms`),
o agente lê os objetos de desempenho do Hyper-V; veja
[Anfitriões Hyper-V](#hyper-v-hosts). `hyperv: true` e `hyperv: false` forçam a
escolha, como para o MDaemon.

**Qualquer outro contador.** Liste o seu caminho completo em `perf_counters`, como
`\Object(Instance)\Counter` ou `\Object\Counter` para um objeto com uma única
instância, e é publicado como `dumbmonit_agent_perf_counter{counter="…"}`, com o
nome que der ou o próprio caminho:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Escreva os nomes dos contadores **em inglês**, tal como aparecem no Monitor de
  Desempenho num Windows em inglês: o agente adiciona-os com
  `PdhAddEnglishCounterW`, pelo que o mesmo `agent.yaml` funciona num Windows em
  francês ou alemão.
- Ponha cada caminho entre aspas simples em YAML, para que as barras invertidas e
  o `%` sejam mantidos tal como estão.
- Um caminho malformado pára o agente no arranque com a chave e o motivo. Um
  caminho que o Windows não conhece (o software não está instalado, o nome está
  mal escrito) é reportado uma vez no registo, `performance counter not
  available`, e os outros contadores continuam a ser lidos; é tentado de novo a
  cada dez períodos.
- As taxas (`…/sec`) precisam de duas leituras: aparecem a partir do segundo
  período.
- São lidos no máximo 200 contadores. No Linux, macOS e FreeBSD a definição é
  aceite e nada é lido.

Para ver os nomes exatos na máquina, execute `typeperf -q MDaemon` (ou qualquer
outro nome de objeto) numa linha de comandos.

## Anfitriões Hyper-V {#hyper-v-hosts}

Instalado num anfitrião Hyper-V — Windows Server com a função Hyper-V, ou Windows
com o Hyper-V ativado — o agente encontra o serviço `vmms` e lê os contadores de
desempenho do próprio Hyper-V, sem nada a configurar. A página do dispositivo do
agente mostra então um painel Hyper-V: quantas máquinas virtuais reportam saúde,
a carga real de CPU do anfitrião, a memória que cada VM em execução detém, e o
débito, a latência e os erros de cada disco virtual.

!!! warning "Validado apenas contra a documentação do fabricante — ainda não testado num sistema real"

    Os nomes dos contadores vêm da documentação de desempenho do Hyper-V da
    Microsoft e foram verificados contra a lógica com uma fonte de contadores
    simulada, não num anfitrião Hyper-V em funcionamento. Um contador que o
    Windows não conhece é reportado uma vez no registo do agente e os outros
    continuam a ser lidos. Diga-nos o que falha.

| Contador | Séries | Etiquetas |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: máquinas virtuais em cada estado | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: a carga do anfitrião, convidados incluídos | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: a parte passada a executar convidados | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: VM em execução mais o próprio anfitrião | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: procura de memória sobre a memória atribuída; acima de 100 a VM quer mais do que tem | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (média), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, um contador | `disk` |

Num anfitrião Hyper-V, `\Processor(_Total)\% Processor Time` mede apenas a
partição do próprio anfitrião, não o que as máquinas virtuais consomem: o
contador de processadores lógicos do hipervisor é o que diz se a máquina está
saturada. `vm` é o nome da máquina virtual como o Gestor de Hyper-V o mostra;
`disk` é o nome de instância que o Windows dá a cada disco virtual, derivado do
caminho do seu ficheiro. As instâncias `_Total` ficam de fora, e são lidas no
máximo 200 VM ou discos por contador.

Aplicam-se três [regras incorporadas](../alerting/rules.md#hyper-v):

- **Hyper-V VM health critical**: o Hyper-V reporta uma VM com saúde crítica
  durante cinco minutos (Warning) — muitas vezes um disco sem espaço.
- **Hyper-V host CPU high**: os processadores lógicos mais de 90 % ocupados
  durante quinze minutos (Advisory).
- **Hyper-V virtual disk errors**: um disco virtual reportou erros de E/S na
  última hora (Advisory).

## Executar o agente no Docker / noutra rede {#run-the-agent-in-docker--on-another-network}

O agente é também publicado como imagem de contentor, com as mesmas tags que o
servidor: `ghcr.io/laupernoe/dumbmonit-agent:latest` (última versão), `:edge`
(último commit em `main`). É uma imagem `FROM scratch` com o binário do agente e o
pacote de CA da Mozilla, configurada inteiramente por variáveis de ambiente. O
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
| CPU, memória, carga, E/S de disco do anfitrião | Nada | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` e `/proc/diskstats` são do anfitrião inteiro dentro de qualquer contentor. |
| Nome do anfitrião | `DUMBMONIT_AGENT_HOSTNAME` | O hostname de um contentor é um id aleatório; o dispositivo ficaria com esse nome. |
| Contentores do anfitrião | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Opcional. Só de leitura lista e inspeciona; as ações Restart/Update precisam dele gravável e de `DUMBMONIT_AGENT_COMMANDS=true`. Sem o socket, o agente simplesmente não reporta contentores. |
| Uso do sistema de ficheiros | `- /:/host:ro` (ou as montagens que lhe interessam) | O agente reporta as montagens visíveis *dentro* do contentor, sob o caminho do contentor. |
| Contadores de rede do anfitrião | `network_mode: host` | As interfaces têm namespace: sem isto o agente vê um `veth`. |
| Monitores de ping retransmitidos por este agente | `cap_add: [NET_RAW]` | O ICMP precisa de um socket raw. |
| CA privada | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Servidor atrás de um proxy inverso interno, dispositivos com certificados autoassinados. |
| Manter a associação entre recriações | `- agent-state:/var/lib/dumbmonit-agent` e `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | O segredo de associação vive no sistema de ficheiros do contentor, que um `docker compose up --force-recreate` deita fora. Sem o volume, o agente volta sem associação e alguém tem de permitir o novo registo a partir da página do dispositivo de cada vez. |

`pid: host` e `privileged: true` nunca são necessários: o agente não recolhe
métricas por processo.

### Modo relay: monitorizar outro site {#relay-mode-monitor-another-site}

Um agente pode fazer mais do que vigiar a sua própria máquina. Com `relay: true`
(ou `DUMBMONIT_AGENT_RELAY=true`), o servidor **delega sondas** nele: os
dispositivos cuja definição *Reached through* nomeia este agente não são
sondados pelo servidor mas pelo agente, a partir da sua própria rede, com exatamente
os mesmos coletores — SNMP, Proxmox VE, Proxmox Backup Server, Synology, HTTP,
TCP, DNS, ping, TLS. Os resultados aterram sob o dispositivo como se o servidor o
tivesse sondado: as mesmas métricas, a mesma *última sonda*, os mesmos alertas.
Um escritório remoto, um segundo homelab ou a rede de um cliente aparecem assim
no único DumbMonit que já tem, sem VPN e sem nada a abrir do lado remoto.

Como o configurar, passo a passo, está em
[Monitorizar um site remoto](../install/remote-site.md).

## Associação: uma máquina, um agente {#binding-one-machine-one-agent}

Um token de registo diz *esta máquina tem permissão para falar*. Não diz *qual*
máquina está a falar — para isso serve a chave de identidade (`/etc/machine-id`,
ou o nome do anfitrião), e essa chave não é segredo: pode ser lida em qualquer
máquina em segundos.

Assim, no primeiro lote o servidor entrega a cada máquina um **segredo de
associação** próprio. Guarda apenas uma impressão digital; o agente escreve o
segredo em `/etc/dumbmonit/agent-secret` com modo `0600` e apresenta-o no
cabeçalho `X-DumbMonit-Agent-Secret` em cada pedido a seguir — medições, comandos
de contentores e sondas retransmitidas por igual. A partir daí:

- nenhuma outra máquina pode enviar medições sob este dispositivo, nem reescrever
  o seu nome de anfitrião e sistema operativo;
- nenhuma outra máquina pode obter — e portanto consumir — os comandos Restart e
  Update em fila para ela.

A página do dispositivo mostra em que ponto está uma máquina, em **Agente →
Associação**:

| Estado | O que significa | O que fazer |
|---|---|---|
| **Bound** | O agente tem um segredo próprio. | Nada. |
| **Not bound — re-enrol this host** | Um anfitrião registado antes de a associação existir, e nunca associado desde então. O servidor recusa as suas medições, comandos de contentores e sondas retransmitidas. | Clique em **Allow re-enrolment** na página do dispositivo, depois reinicie o agente. Se o agente é anterior à associação, volte primeiro a executar o comando de instalação no anfitrião. |

### Anfitriões anteriores à associação {#hosts-that-predate-binding}

!!! warning "Alterado: os anfitriões não associados são recusados"
    As versões anteriores continuavam a aceitar anfitriões registados antes da
    associação, desde que usassem o token que os tinha registado, e associavam-nos
    silenciosamente quando o seu agente era atualizado. Essa janela de transição
    está fechada: um anfitrião que nunca se associou é agora **recusado em todas
    as rotas do agente** — medições, comandos de contentores e sondas
    retransmitidas — até um administrador o registar de novo. Um token de frota é
    partilhado por muitas máquinas e a chave de identidade pode ser adivinhada,
    pelo que nada mais prova que o agente que pede é o do próprio anfitrião.

    Um binário de agente demasiado antigo para receber um segredo de associação
    também é recusado, incluindo no seu primeiro registo.

Para trazer esse anfitrião de volta:

1. Se o seu agente é anterior à associação, volte a executar nele o comando de instalação (o binário do agente tem de conseguir guardar um segredo de associação).
2. Abra a página do seu dispositivo e clique em **Agente → Allow re-enrolment**. Isto abre uma janela de uma hora.
3. Reinicie o agente, ou espere pela sua próxima tentativa: esse lote associa o anfitrião com um segredo novo e fecha a janela. Um token diferente do que registou o anfitrião tem ainda de poder registar (não esgotado, não expirado).

Até lá, o registo do agente traz a explicação do servidor:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Novo registo após uma reinstalação {#re-enrolment-after-a-reinstall}

Se a máquina é reconstruída, o seu disco substituído, ou o seu contentor de agente
recriado sem o volume que guarda o segredo, o agente volta sem ele e o servidor
recusa os seus lotes. Essa recusa é deliberada: uma máquina que está associada
continua associada até um humano dizer o contrário, senão a associação não
protegeria nada.

O caminho de volta está na página do dispositivo: **Agente → Allow re-enrolment**.
Abre uma janela de uma hora durante a qual um agente que apresente um token de
registo válido associa essa máquina de novo, com um segredo novo; o segredo antigo
deixa de funcionar, e a janela fecha assim que é usada. O agente tenta de novo por
si entretanto, pelo que não há nada a reiniciar na máquina — e nada se perde, já
que as suas leituras ficam em buffer até a ligação ser aceite de novo.

Até alguém abrir essa janela, o registo do agente diz o que fazer:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Tokens de registo {#enrolment-tokens}

- Os tokens são criados em **Definições → Agentes** ou ao adicionar um dispositivo
  do tipo agent. O servidor guarda apenas uma impressão digital: o token em claro
  é mostrado uma vez, na resposta de criação, com os dois comandos de instalação.
- O agente envia-o como `Authorization: Bearer dmon_…` em `POST /api/ingest`.
  Nunca aparece nos registos do agente, nem sequer truncado.
- **Definições → Agentes** lista os tokens com o seu prefixo, âmbito, data de
  criação e última utilização. **Revogar** pára todos os agentes que usam esse
  token no seu próximo envio; as máquinas ficam então *inacessíveis* após três
  períodos falhados. Volte a executar o instalador com um token novo para as
  registar de novo.
- As máquinas são dispositivos separados: revogar um token não as apaga.

### Uso único ou frota {#single-use-or-fleet}

O âmbito de um token é escolhido quando é criado, e é uma escolha, não uma
predefinição que lhe acontece:

| Âmbito | O que regista | Quando |
|---|---|---|
| **Single use** (a predefinição) | Exatamente uma máquina. | Uma instalação. Um comando de instalação sobrevive no histórico da shell, num ticket, num registo de chat; um de uso único não vale nada depois de usado. |
| **Reusable for a fleet** | Tantas máquinas quantas permitir — um número, ou sem limite. | Um playbook, uma imagem de máquina, um lote de instalações. |

Ambos podem ter um prazo em dias. Um prazo só pára os *registos*: as máquinas já
registadas continuam a reportar depois de passar, que é o que quer para uma janela
de instalação. Revogar é o que corta uma frota.

Um token que já não pode registar — esgotado, ou passado o prazo — é mostrado como
**Enrols no more** em Definições → Agentes. As suas máquinas continuam a reportar;
apenas não pode deixar entrar uma nova.

## Falhas e diagnóstico {#outages-and-diagnostics}

Quando o servidor é inacessível, o agente continua a medir e guarda as suas
leituras em buffer na memória com os carimbos temporais originais. São enviadas
assim que a ligação volta: um reinício do servidor não deixa buraco nos gráficos.
O buffer é limitado (`max_buffered_samples`, cerca de uma hora por predefinição) e
descarta as leituras mais antigas quando enche. As tentativas recuam de 5 segundos
a 5 minutos com jitter.

```sh
dumbmonit-agent --dry-run          # print the readings, send nothing
dumbmonit-agent --once             # send one batch then exit
journalctl -u dumbmonit-agent -f   # service log (systemd)
tail -f /var/log/dumbmonit-agent.log   # service log (OpenRC)
```

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| O instalador diz «could not reach the server» | URL ou token errado. A configuração está escrita: corrija o `agent.yaml`, depois reinicie o serviço. |
| O download falha com 404 | O servidor não tem binários de agente (`DUMBMONIT_AGENT_DIR` vazio). Use `--bin=PATH` com um binário que compilou. |
| O instalador pára com «checksum mismatch» | O comando foi copiado antes de uma atualização do servidor (copie um novo de Definições → Agentes), ou o binário recebido não é o que o servidor serve: um proxy ou uma cache pelo meio, ou um download adulterado. |
| O instalador pára com «no expected checksum» | O comando não tem `--sha256` para esta plataforma: um comando antigo, ou um binário macOS (não fornecido pelo servidor). Copie um comando novo, ou passe `--sha256=<platform>-<arch>:<hex>` a partir do `.sha256` da versão. |
| A máquina nunca aparece | O envio vai para `/api/ingest` no URL do comando de instalação; atrás de um proxy inverso, garanta que é reencaminhado e que corpos até 16 MB são permitidos. |
| *Inacessível* embora o agente corra | O token foi revogado, ou os envios são rejeitados. Verifique `journalctl -u dumbmonit-agent`: o erro é registado aí. |
| O registo diz «this agent is not recognised for this machine» | A máquina está associada a uma instalação de agente cujo segredo esta não tem — uma reinstalação, ou um contentor recriado sem o seu volume de estado. Abra a página do dispositivo e clique em **Allow re-enrolment**; veja [Novo registo após uma reinstalação](#re-enrolment-after-a-reinstall). |
| O registo diz «this machine was enrolled before agent binding» | Um anfitrião que nunca se associou ao seu agente. Clique em **Allow re-enrolment** na sua página do dispositivo, depois reinicie o agente; veja [Anfitriões anteriores à associação](#hosts-that-predate-binding). |
| O registo diz «this agent is too old to be bound to its machine» | O binário do agente não consegue guardar um segredo de associação. Volte a executar o comando de instalação no anfitrião; se o dispositivo já existe, clique primeiro em **Allow re-enrolment**. |
| O registo diz «this enrollment token has already enrolled all the machines it was allowed to» | Um token de uso único a ser reutilizado. Crie um novo, ou um token reutilizável em Definições → Agentes. |
| Windows | As partes do serviço Windows não tinham sido exercitadas na imagem de build do projeto à data da escrita; reporte problemas no GitHub. |

## Contentores Docker {#docker-containers}

Quando o agente consegue ler o socket do Docker (`docker: true`, a predefinição, e
um utilizador no grupo `docker` ou root — `usermod -aG docker dumbmonit`, depois
reinicie o agente), cada contentor torna-se um punhado de séries, com as
etiquetas `container` e `image` (o `name:tag` mostrado por `docker ps`). Se a
página do dispositivo diz «No Docker on this machine» enquanto o Docker está
instalado, é a primeira coisa a verificar.

| Métrica | Significado |
|---|---|
| `container_up` | 1 em execução, 0 caso contrário |
| `container_health` | 0 sem healthcheck, 1 healthy, 2 unhealthy, 3 starting |
| `container_restart_count` | Contador de reinícios do Docker para o contentor |
| `container_started_seconds` | Segundos desde que o contentor arrancou pela última vez (0 quando parado) |
| `container_image_age_seconds` | Segundos desde que a imagem foi construída |
| `container_update_available` | 1 quando o registry tem uma imagem mais recente para a mesma tag, 0 quando está atualizado, -1 desconhecido |

A verificação de atualização compara o digest da imagem local com o que o registry
anuncia para a tag (`HEAD /v2/<name>/manifests/<tag>`, fluxo de token anónimo no
Docker Hub e GHCR). Corre em segundo plano no máximo uma vez por hora por imagem e
nunca atrasa o ciclo de 30 segundos; um registry privado que recusa acesso
anónimo lê-se simplesmente como -1. Desative-a com `docker_update_check: false`
(`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Regras incorporadas: **Container stopped** (`container_up == 0` durante 2
minutos), **Container unhealthy** (`container_health == 2` durante 3 minutos),
**Container restarting** (três ou mais reinícios em 15 minutos), **Container
update available** (informação, após uma hora, repetida diariamente). O alerta
nomeia o contentor.

### Ações {#actions}

Na página do dispositivo, cada contentor tem duas ações e dois interruptores. As
ações viajam por um **canal de comandos**: o servidor põe o comando em fila, o
agente obtém-no após o seu próximo lote (`GET /api/agent/commands`, o mesmo token
de registo), executa-o, e reporta o resultado (`queued → running → done |
failed`), com um breve registo que pode desdobrar na interface. Os comandos com
mais de dez minutos nunca são executados; o agente executa um comando de cada vez.

Um comando em fila pode ser **cancelado** a partir da interface enquanto o agente
não o tiver apanhado. Um comando que ninguém foi buscar **expira** ao fim de dez
minutos, no servidor, quer o agente alguma vez consulte ou não — pelo que uma
máquina cujo agente está parado nunca mantém um «Queued» obsoleto para sempre, e
nunca bloqueia a ação seguinte nesse contentor.

O agente diz ao servidor, com cada lote, se aceita comandos. Quando não aceita —
um agente anterior ao canal de comandos, ou instalado com `commands: false` — a
página do dispositivo di-lo em vez de mostrar Restart e Update, a API recusa os
comandos (`409`), e as políticas não põem nada em fila. Reinstale o agente com o
instalador atual para ativar as ações.

- **Restart** — `docker restart` para um contentor em execução, `docker start`
  para um parado, depois uma verificação de 20 segundos de que corre.
- **Update now** — puxa a tag que o contentor já usa, cria um novo contentor com a
  mesma configuração (ambiente, montagens, portas, labels, redes, política de
  reinício), pára o antigo, troca os nomes, inicia o novo e espera pelo seu
  healthcheck (ou 20 segundos mais uma verificação de execução quando a imagem
  não tem um). Se algo falhar, o agente faz **rollback**: o novo contentor é
  removido e o antigo reiniciado, e o comando reporta `failed` com o motivo. Em
  caso de êxito o contentor antigo é removido e, com o «prune» ativado, a imagem
  antiga também — a menos que outro contentor ainda a use.

Os contentores geridos pelo Compose (labels `com.docker.compose.*`) são
atualizados na mesma, mas o resultado avisa que o próximo `docker compose up`
recriará o contentor a partir do ficheiro compose. Os contentores cujo nome
começa por `dumbmonit` ou `dumbmonit` são recusados: o monitor nunca se atualiza
a si próprio.

### Políticas {#policies}

Uma **política** por contentor é avaliada pelo servidor a cada minuto:

- **Restart if down** — um contentor que reporta `container_up == 0` é reiniciado
  automaticamente, no máximo uma vez a cada dez minutos.
- **Auto-update in maintenance windows** — um contentor com atualização disponível
  é atualizado automaticamente, mas apenas enquanto uma janela de manutenção
  (silêncio) cobre o dispositivo, no máximo uma vez a cada seis horas. Agende uma
  na página de Alertas, ou silencie o dispositivo a partir da sua página. As
  políticas por predefinição removem a imagem antiga e só atuam em janelas de
  manutenção.

Dois locais as definem:

- a **faixa Docker** sob o cabeçalho do dispositivo: «28 containers · 27 running ·
  3 updates available · policies: 2 auto-restart, 0 auto-update», a última ação e
  o seu estado, **Manage containers** (abre a lista abaixo) e **Policies…**, uma
  tabela de todos os contentores com os dois interruptores e uma linha «Apply to
  all» — a forma mais rápida de ligar o reinício automático para uma máquina
  inteira;
- os interruptores dentro de cada linha da secção **Containers**, junto aos botões
  Restart e Update now.

Cada interruptor guarda-se por si (`PUT /api/targets/<id>/containers/<name>/policy`);
uma falha diz respeito apenas a essa linha e é mostrada ao lado dela.

As ações automáticas aparecem em «Recent actions» como pedidas por `policy`.

### Regras de segurança {#safety-rules}

- O agente só atua quando `commands: true` (a predefinição); defina-o a `false`
  para uma máquina que deve apenas ser medida.
- O agente obtém os comandos ele próprio, com o seu token de registo — nada se liga
  à máquina. Expiram ao fim de dez minutos e correm um de cada vez; o agente nunca
  puxa uma tag diferente da que o contentor já usa.
- Uma atualização que não volta saudável sofre rollback para o contentor anterior,
  e a imagem antiga só é removida quando o novo contentor corre.
- Os contentores chamados `dumbmonit*` ou `ezymonit*` são recusados, à mão ou por
  política: o monitor nunca atua nos seus próprios contentores. A verificação é
  feita duas vezes — o servidor recusa pôr o comando em fila, e o agente recusa
  executá-lo — e o agente volta a fazê-la sobre o nome que o Docker reporta depois
  de inspecionar o contentor, pelo que nomeá-lo pelo seu id não muda nada. O
  agente recusa também atuar no seu próprio contentor, seja qual for o seu nome:
  não estaria lá para terminar o trabalho nem para reportar sobre ele.
- Os contentores geridos pelo Compose são atualizados, mas o próximo `docker
  compose up` recria-os a partir do ficheiro compose: suba a tag aí também.

## Cópias de segurança Plakar {#plakar-backups}

O agente pode vigiar klosets (repositórios) do [Plakar](https://plakar.io): a cada
dez minutos, em segundo plano, executa `plakar at <kloset> ls` e `info` em cada um
e reemite a última leitura a cada ciclo.

O Plakar é **detetado, nunca assumido**. No arranque e depois a cada ciclo até
encontrar algo, o agente procura o binário `plakar` (no `PATH`, ou `plakar_bin`) e
klosets nas localizações padrão. Quando não há binário, nem kloset, nem rasto do
Plakar (`~/.cache/plakar`), o agente não emite nenhuma série `backup_*`, di-lo uma
vez ao nível `debug`, e a página do dispositivo não mostra painel Backups — nada a
configurar em máquinas sem Plakar. Para saltar totalmente a deteção:

```yaml
plakar: false                        # or DUMBMONIT_AGENT_PLAKAR=false
```

### Descoberta {#discovery}

Nada a configurar: quando `plakar_klosets` está vazio, o agente encontra ele
próprio os klosets, por esta ordem, e lê cada localização apenas uma vez:

1. a própria diretoria de configuração do agente — `$XDG_CONFIG_HOME/plakar`, ou
   `~/.config/plakar` da conta sob a qual corre (`plakar_home` quando definido);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, por ordem alfabética;
4. `/var/lib/plakar`, quando essa diretoria existe.

Em cada diretoria lê `stores.yml` (ou `stores.yaml`; o antigo `plakar.yml` também
é aceite) e toma cada entrada do mapa `stores` (também `repositories` e `klosets`)
que tenha uma `location`. Apenas a localização é lida: as frases-passe e os
segredos rclone que estão ao lado nunca são guardados nem registados. Cada
entrada torna-se o kloset `@<name>`, lido com `plakar -configdir <dir> at
@<name>`, pelo que um kloset criado por um utilizador é lido mesmo quando o agente
corre como root. O `~/.plakar` de cada conta é acrescentado quando existe (o
kloset predefinido de `plakar create`). Duas contas que nomeiam klosets diferentes
da mesma forma distinguem-se como `@name` e `<user>:@name`.

O resultado é registado uma vez ao nível `info` (e de novo apenas quando muda). Dois
gauges, emitidos só depois de o Plakar ser detetado, deixam a interface distinguir
os casos restantes: `backup_plakar_present` (1 quando o binário corre) e
`backup_klosets_found` (número de klosets lidos). A página do dispositivo diz então
«Plakar is installed but no kloset was found», ou lista os klosets. Quando existem
klosets mas o binário não corre (um `PATH` de serviço sem `plakar`, tipicamente),
são listados como ilegíveis com uma dica para definir `plakar_bin`, e o agente
avisa uma vez.

Para vigiar antes uma lista fixa, nomeie-a — a descoberta é então ignorada:

```yaml
plakar: true                         # default: detect Plakar; false hides it entirely
plakar_bin: /usr/bin/plakar          # default: "plakar" on PATH
plakar_klosets:                      # explicit list: overrides discovery
  - /srv/backups/main
  - "@diskext"                       # a named store of the agent's account
plakar_home: /home/noe               # optional: HOME (and config dir) used when running plakar
plakar_interval_secs: 600            # minimum 60
```

| Variável | Efeito |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — vigiar cópias de segurança Plakar quando detetado (predefinição `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Caminho do binário `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets, separados por vírgulas (substitui a descoberta) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Diretoria home usada ao executar `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Segundos entre duas leituras |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — verificação no registry de atualizações de contentores |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — aceitar ações do servidor |

Um kloset cifrado precisa da sua frase-passe: ponha-a na entrada do store
(`plakar store set <name> passphrase=…`, que é o que `plakar store add` faz) ou
exporte `PLAKAR_PASSPHRASE` no ambiente do serviço do agente. Sem ela o kloset lê-se
como «Cannot be opened» (`backup_last_status = 0`).

### Métricas e regras {#metrics-and-rules}

Métricas, com as etiquetas `kloset` e `source` (a diretoria que foi copiada):
`backup_last_success_seconds` (idade do snapshot mais recente),
`backup_snapshot_count`, `backup_last_status` (1 quando o snapshot mais recente não
tem erro, 0 quando tem erros ou o kloset não pode ser aberto),
`backup_size_bytes{kloset}` (tamanho de armazenamento do repositório), mais
`backup_plakar_present` e `backup_klosets_found` descritos acima. Nenhuma delas
existe enquanto o Plakar não é detetado, pelo que as regras incorporadas abaixo
não podem disparar numa máquina sem Plakar.

Regras incorporadas: **Plakar backup too old** (consultiva, sem snapshot há mais de
dois dias) e **Plakar backup failed** (aviso). A página do dispositivo mostra um
bloco por kloset — idade da última cópia, contagem de snapshots, tamanho, último
estado — e uma linha por origem, com uma dica «Backup too old» passados dois dias.

### Executar uma primeira cópia de segurança {#run-a-first-backup}

Com o Plakar 1.1 instalado na máquina (`plakar version`), como o utilizador que será
dono das cópias:

```sh
# 1. a kloset store, named so the agent finds it in ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # or any location Plakar accepts
plakar at @diskext create                                   # add -plaintext to skip encryption

# 2. a first backup, then check it
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` e `plakar at <path> backup <dir>` funcionam da mesma
forma com uma diretoria simples em vez de `@name`; `plakar create` sem `at` usa
`~/.plakar`, que o agente também encontra. Agende `plakar at @diskext backup
/srv/photos` com cron ou um timer systemd: o agente reporta a idade do snapshot
mais recente, e alerta quando passa um dia sem um — dois dias para a regra
incorporada.

Para experimentar localmente com um kloset não cifrado:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

depois acrescente `plakar_klosets: ["/tmp/plakar-demo"]` ao `agent.yaml` (um
caminho simples fora de um home não é descoberto) e verifique com
`dumbmonit-agent --dry-run` que aparecem amostras `backup_*`.

## Cópias de segurança restic e Borg {#restic-and-borg-backups}

O agente pode vigiar repositórios [restic](https://restic.net) e
[Borg](https://www.borgbackup.org): a cada dez minutos, em segundo plano, abre cada
repositório que declarar e reporta se pôde ser lido e quão antigo é o seu snapshot
mais recente. Nada é descoberto: um repositório não tem localização padrão e não
pode ser aberto sem a sua palavra-passe, pelo que cada um é listado em
`agent.yaml`, com a forma de obter a sua palavra-passe.

**A palavra-passe nunca sai da máquina.** O agente entrega-a ao `restic` ou ao
`borg` apenas através do ambiente do processo filho; nunca é registada, nunca é
enviada ao servidor, e as séries só trazem o nome do repositório.

```yaml
restic_repos:
  - name: nas                                   # the "repo" label; default: the repository, without credentials
    repository: sftp:backup@nas.lan:/srv/restic  # passed as RESTIC_REPOSITORY, never on the command line
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # optional: only this host's snapshots (--host), for a shared repository
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # or any command that prints it (RESTIC_PASSWORD_COMMAND)
    env:                                        # extra environment for this repository only
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # passed as BORG_REPO
    password_file: /etc/dumbmonit/borg-offsite.pass        # read by the agent, passed as BORG_PASSPHRASE
    glob_archives: "web-01-*"                   # optional: only these archives (--glob-archives)
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # default: on PATH
borg_bin: borg
backup_repos_interval_secs: 600                 # minimum 60
```

Mantenha `agent.yaml` e os ficheiros de palavra-passe legíveis apenas por root
(`chmod 600`), como o instalador faz para `agent.yaml`. `password_command`
corresponde a `RESTIC_PASSWORD_COMMAND` ou `BORG_PASSCOMMAND`. Tudo o que as
ferramentas leem do seu ambiente pode ir em `env` (`RESTIC_PASSWORD`,
`BORG_PASSPHRASE`, chaves de nuvem, `BORG_RSH`…), e o ambiente do próprio serviço
do agente também é passado.

O que o agente executa, só de leitura no sentido mais estrito:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: nenhum bloqueio é
  escrito, pelo que um repositório só de leitura ou só de acréscimo pode ser
  vigiado, e um `prune` que detém o bloqueio exclusivo não faz o repositório
  parecer em baixo; também sem cache, já que o serviço instalado não pode escrever
  em `/root`.
- `borg --bypass-lock list --json --last 1`: sem `--bypass-lock`, a leitura
  falharia durante toda a duração de cada `borg create`, que detém o bloqueio
  exclusivo. O Borg escreve um pequeno registo de segurança em `~/.config/borg`;
  quando esse home não é gravável (o serviço instalado corre com
  `ProtectHome=yes`), o agente aponta `BORG_BASE_DIR` para o seu `/tmp` privado,
  a menos que defina você próprio `BORG_BASE_DIR`. Um repositório em modo
  `keyfile` precisa então de `BORG_KEY_FILE` no seu `env`.

Como o serviço corre com `ProtectHome=yes`, mantenha o que as ferramentas precisam
fora das diretorias home: ficheiros de palavra-passe, chaves SSH e `known_hosts`
em `/etc/dumbmonit/`, como no exemplo, e repositórios locais em `/srv`, `/mnt` ou
`/var` em vez de `/home`. As leituras correm uma após outra, dentro do limite de
memória do serviço (128 MiB com o próprio agente).

Métricas, com as etiquetas `tool` (`restic` ou `borg`) e `repo`:
`agent_backup_repo_reachable` (1 quando a última leitura teve êxito, 0 caso
contrário: repositório inacessível, palavra-passe recusada, binário em falta — o
agente regista o motivo uma vez, e de novo quando muda),
`agent_backup_repo_last_snapshot_age_seconds` (idade do snapshot mais recente,
ausente enquanto o repositório está vazio), e
`agent_backup_repo_last_snapshot_bytes` (bytes que o snapshot restic mais recente
processou, restic 0.17 e posteriores). Com `--latest 1`, o restic devolve o
snapshot mais recente de cada anfitrião e conjunto de caminhos; o mais recente de
todos é mantido, pelo que deve definir `host` num repositório partilhado por várias
máquinas.

Regras incorporadas: **restic/Borg backup too old** (sem snapshot há mais de 48
horas) e **restic/Borg repository unreachable** (durante 30 minutos). A página do
dispositivo mostra um painel Backup repositories, uma linha por repositório.

Para verificar uma declaração, execute o agente uma vez em primeiro plano e procure
as amostras `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Túneis WireGuard {#wireguard-tunnels}

Numa máquina que corre WireGuard, o agente lê cada interface e cada peer com `wg
show all dump`, em cada ciclo. Nada a configurar: sem o comando `wg`
(wireguard-tools) ou sem interface, nenhuma série é emitida e nenhum painel é
mostrado. O `wg` precisa de root (`CAP_NET_ADMIN`); o serviço instalado tem-no, um
agente iniciado à mão na sua própria conta regista a recusa uma vez e não reporta
nada.

O dump também contém a chave privada da interface e a chave pré-partilhada de cada
peer: o agente ignora ambos os campos sem nunca os copiar.

Por peer, com as etiquetas `interface`, `peer` (a sua chave pública) e
`allowed_ips`: `agent_wireguard_peer_handshake_age_seconds` (segundos desde o
último handshake), `agent_wireguard_peer_has_handshake` (0 enquanto o peer nunca
completou um), `agent_wireguard_peer_keepalive_seconds` (o seu keepalive
persistente, 0 quando desativado), e os contadores `agent_wireguard_peer_rx_bytes`
e `agent_wireguard_peer_tx_bytes`; por interface,
`agent_wireguard_interface_peers`. Para um peer que nunca completou um handshake, a
idade conta a partir do momento em que o agente o viu pela primeira vez: um túnel
que nunca subiu é também silencioso, e é reportado como tal.

O WireGuard não tem nomes de peers. Dê os seus em `agent.yaml`, por chave pública,
e tornam-se a etiqueta `name`:

```yaml
wireguard: true                     # default; false stops even looking
wireguard_bin: wg                   # default: on PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

A regra incorporada **WireGuard peer silent** dispara quando um peer **com
keepalive persistente** não completou um handshake há mais de 15 minutos.
Enquanto um túnel está de pé, o WireGuard renova o seu handshake a cada dois
minutos, e um keepalive mantém-no de pé mesmo sem tráfego; um peer sem keepalive
(um telemóvel, um portátil) pode ficar inativo dias sem nada avariado, pelo que
fica de fora. Defina `PersistentKeepalive = 25` nos túneis que têm de ficar de pé
(ligações site-to-site, gateways remotos). N é o limiar da regra: altere-o em
**Alertas → Regras**, ou remova a parte `and on (…)` da sua consulta para vigiar
todos os peers. A página do dispositivo mostra um painel WireGuard com cada peer, o
tempo desde o seu último handshake, o seu tráfego e o seu keepalive.

## Certificados TLS (Certbot / Let's Encrypt) {#tls-certificates-certbot-lets-encrypt}

O agente não lê `/etc/letsencrypt` diretamente. Cada certificado que o Certbot
renova é servido em algum lado — um servidor web, um servidor de correio, um proxy
inverso — e é aí que o DumbMonit o lê: adicione um monitor
[Certificado TLS](services.md#tls) (ou um de [Site ou API web](services.md#http),
para um site já vigiado dessa forma) apontado para a porta em que o certificado é
realmente apresentado. Lê o certificado em direto, não um ficheiro em disco, pelo
que apanha também uma renovação que teve êxito em `/etc/letsencrypt` mas nunca foi
apanhada pelo serviço (um proxy inverso que precisa de um reload para ver o novo
ficheiro é o caso habitual, e exatamente a lacuna que importa).

Ler `/etc/letsencrypt/live/*/cert.pem`, a configuração de renovação e o registo do
Certbot a partir do agente foi considerado e posto de parte: duplicaria a data de
expiração que o monitor TLS já reporta, acrescentando um segundo caminho local ao
anfitrião para a mesma informação — mais uma coisa a manter sincronizada depois de
um certificado ser movido ou um domínio renomeado. Um anfitrião onde o Certbot corre
mas nada serve que o DumbMonit possa alcançar diretamente (um passo de CA interna,
um hook de validação DNS) é o único caso que isto deixa por cobrir; é também o caso
em que um monitor TLS não poderia ajudar, já que nada nesse anfitrião fala TLS para
sondar.
