# Агент для Linux, macOS, FreeBSD и Windows

Небольшой бинарный файл, который устанавливается на отслеживаемую машину. Он измеряет CPU, память,
диски, сеть, службы и контейнеры — а там, где машина их предоставляет, —
температуры, состояние дисков, пулы ZFS и виртуальные машины KVM — после чего **отправляет** (push) свои показания на
сервер DumbMonit по HTTP.

Направление выбрано намеренно: агент обращается к серверу, а не наоборот.
Он проходит через NAT и домашние межсетевые экраны и не требует открывать **ни одного порта**
на отслеживаемой машине.

## Что отслеживается

Все метрики имеют префикс `dumbmonit_`. Идентифицирующие метки (`target`, `host`,
`tag_*`) задаются сервером при получении данных, но не агентом: машина
не может писать в ряды другой машины, даже подделав свои метки.

| Семейство | Метрики | Метки |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` только при `cpu_per_core: true` | `core` |
| Память | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Файловые системы | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Сеть (счётчики) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Диски (счётчики) | `disk_read_bytes`, `disk_written_bytes`, один ряд на каждое блочное устройство | `device` |
| Хост | `uptime_seconds`, `process_count` | |
| Службы | `service_up` (1 = запущена) | `service` |
| Контейнеры | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (контейнеры сверх `docker_max_containers`) | `container`, `image` |
| Датчики | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (порог, который объявляет само оборудование), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Состояние дисков (SMART) | `agent_disk_smart_ok` (1 = диск проходит собственную самооценку), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| KVM / libvirt | `agent_libvirt_domains`, `agent_libvirt_domains_running`, `agent_libvirt_pools`; для каждого домена `agent_libvirt_domain_state_info`, `agent_libvirt_domain_running`, `agent_libvirt_domain_crashed`, `agent_libvirt_domain_vcpus`, `agent_libvirt_domain_memory_max_bytes`, `agent_libvirt_domain_memory_used_bytes`; для каждого пула хранилища `agent_libvirt_pool_state_info`, `agent_libvirt_pool_active`, `agent_libvirt_pool_capacity_bytes`, `agent_libvirt_pool_allocation_bytes`, `agent_libvirt_pool_used_percent` — см. [Виртуальные машины KVM через libvirt](#kvm-virtual-machines-through-libvirt) | `domain`, `pool`, `state` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degraded, 2 хуже), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; для каждого пира `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (счётчики) — см. [Туннели WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| IPsec (strongSwan) | `agent_ipsec_tunnel_up`; пока туннель поднят, `agent_ipsec_tunnel_bytes_in/out` (счётчики) и `agent_ipsec_tunnel_rekey_seconds` — см. [Туннели IPsec](#ipsec-tunnels) | `connection`, `tunnel` |
| Тест скорости Интернета | `agent_speedtest_ok`, `agent_speedtest_age_seconds`, `agent_speedtest_download_bytes_per_second`, `agent_speedtest_upload_bytes_per_second`, `agent_speedtest_latency_ms`, `agent_speedtest_jitter_ms`; заданные пороги публикуются как `agent_speedtest_download_min_bytes_per_second`, `agent_speedtest_upload_min_bytes_per_second`, `agent_speedtest_max_latency_ms` — см. [Тест скорости Интернета](#internet-speed-test) | |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — см. [Резервные копии restic и Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Счётчики производительности Windows | `agent_perf_counter`, один ряд на каждый счётчик из списка `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — см. [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; для каждой ВМ `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; для каждого виртуального диска `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — см. [Хосты Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Агент | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Ряд, который невозможно измерить, **отсутствует**, а не равен нулю: нет `smartctl` — нет
`agent_disk_*`; нет читаемого датчика — нет `agent_sensor_*`; нет ZFS — нет
`agent_zfs_*`; нет `virsh` — нет `agent_libvirt_*`. Температура вне диапазона 1–150 °C отбрасывается, а не публикуется,
и показание вентилятора 0 об/мин тоже отбрасывается: на материнской плате
разъёмов для вентиляторов больше, чем самих вентиляторов, а пустой разъём
показывает ноль так же, как и неисправный вентилятор.

### Что собирается и где

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, память, swap, файловые системы, сеть, uptime, процессы | да | да | да | да |
| Средняя нагрузка (load average) | да | да | да | нет (в Windows такого понятия нет) |
| Службы | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | службы Windows |
| Контейнеры Docker | да | да, если `docker_socket` указывает на сокет Docker Desktop | нет (нет демона Docker) | нет |
| Температуры | `/sys/class/hwmon` | датчики SMC | `dev.cpu.N.temperature` (только CPU) | нет |
| Вентиляторы | `/sys/class/hwmon` | нет | нет | нет |
| Состояние дисков (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Пулы ZFS | `zpool` (OpenZFS) | нет | `zpool` | нет |
| Виртуальные машины KVM (libvirt) | `virsh` | нет | нет | нет |
| Состояние ОС (обновления, перезагрузка, упавшие юниты, SELinux) | да | нет | нет | нет |
| Резервные копии Plakar | да | да | да | да |
| Туннели WireGuard | `wg show` | если установлен `wg` (не проверено) | если установлен `wg` (не проверено) | нет |
| Туннели IPsec | `swanctl` | если установлен `swanctl` (не проверено) | если установлен `swanctl` (не проверено) | нет |
| Тест скорости Интернета | да, по включению | да, по включению | да, по включению | да, по включению |
| Резервные копии restic и Borg | да | да | да | только restic (не проверено; сборки Borg для Windows нет) |
| Счётчики производительности, очереди MDaemon, Hyper-V | нет | нет | нет | да |
| Идентификатор машины | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | имя хоста |
| Режим ретранслятора (relay) | да | да | да | да |

Строки с температурами для Windows нет намеренно: чтение датчиков там означает
обращение к WMI с правами администратора ради значений, которые большинство машин
вообще не предоставляют. Не публиковать ничего — самый честный ответ.

`smartctl` (smartmontools), `zpool` и `virsh` ищутся при каждом периоде считывания
и используются, если присутствуют; агент не устанавливает ни один из них, и ни один
не обязателен. Для всех нужен root (`virsh` работает и для члена группы `libvirt`) — служба, которую регистрирует установщик, работает от root, а
агент, запущенный вручную под вашей учётной записью, ничего не найдёт.

Счётчики отправляются в сыром виде, чтобы перезапуск агента никогда не порождал
ложную скорость; применяйте `rate()` в запросах.

Применяются встроенные правила: Monitor unreachable, High CPU, Unusual CPU. «Device
unreachable» работает и для агентов: сервер проверяет возраст последнего
полученного пакета и объявляет машину молчащей после трёх пропущенных периодов (не менее
90 секунд).

Ещё четыре правила срабатывают только на машинах, которые создают соответствующие ряды, поэтому их
не нужно настраивать, и они не могут сработать на машине без соответствующего оборудования:

| Правило | Срабатывает, когда |
|---|---|
| **Disk SMART failing** (критическое) | собственная самооценка диска сообщает о его неисправности |
| **ZFS pool degraded** (критическое) | пул не находится в состоянии `ONLINE` в течение пяти минут |
| **ZFS scrub found errors** (критическое) | последний scrub обнаружил ошибки, или пул сообщает о необратимо повреждённых файлах |
| **KVM domain crashed** (критическое) | libvirt сообщает о домене как о `crashed` |
| **libvirt pool almost full** (предупреждение) | выделенное место пула хранилища достигает 90 % его ёмкости в течение пятнадцати минут |
| **Temperature above critical** (критическое) | датчик превышает критический порог, **который объявляет его собственное оборудование** — никакие значения не заданы жёстко, ведь 85 °C — это авария для диска и обычный день для процессора ноутбука |

Ещё несколько отслеживают то, что вы объявляете или запускаете сами, и в остальных случаях молчат:
**WireGuard peer silent** (пир с постоянным keepalive и без handshake
в течение 15 минут), **IPsec tunnel down** (настроенный туннель без
установленной сессии в течение десяти минут), **restic/Borg backup too old** (нет снимка
в течение 48 часов), **restic/Borg repository unreachable** и
**Internet download rate low** / **Internet upload rate low** /
**Internet latency high** (только после задания порога) — см.
[Туннели WireGuard](#wireguard-tunnels), [Туннели IPsec](#ipsec-tunnels),
[Резервные копии restic и Borg](#restic-and-borg-backups) и
[Тест скорости Интернета](#internet-speed-test).

На сервере Windows с MDaemon ещё три правила отслеживают его очереди:
**MDaemon mail queue growing**, **MDaemon Bad queue not empty** и **MDaemon
Retry queue high** (см. [Правила](../alerting/rules.md#mdaemon-and-securitygateway)).
На хосте Hyper-V три правила отслеживают виртуальные машины: **Hyper-V VM health
critical**, **Hyper-V host CPU high** и **Hyper-V virtual disk errors** (см.
[Хосты Hyper-V](#hyper-v-hosts)).

## Установка {#install}

Ниже приведены шаги, которые показывает уведомление рядом с формой. Создавать
учётную запись на машине не нужно: токен регистрации — единственный секрет.

1. Сохраните монитор в DumbMonit: токен регистрации (dmon_…) показывается один раз
   вместе с готовой к копированию командой установки для Linux и для Windows. (Токен
   также можно создать в **Settings → Agents**.)
2. Этот токен — ключ агента для отправки своих измерений в DumbMonit. Это
   не учётная запись на машине: создавать там ничего не нужно. Токен, созданный в форме
   монитора, одноразовый: он регистрирует только эту машину и больше ничего.
   Для парка машин создайте многоразовый токен в Settings → Agents.
3. Выполните команду установки на отслеживаемой машине с повышенными правами
   (sudo в Linux и macOS, PowerShell с правами администратора в Windows). Она скачивает
   агент, записывает токен в agent.yaml и запускает службу.
4. Машина появляется сама в течение нескольких секунд под именем своего
   хоста. При этом первом пакете сервер выдаёт агенту собственный секрет и
   сохраняет его в `/etc/dumbmonit/agent-secret`, доступном для чтения только владельцу: с этого
   момента только эта машина может отправлять данные от имени этого монитора.
   Потеряли токен? В Settings → Agents его можно отозвать и создать другой.
5. Docker: чтобы видеть контейнеры и позволить DumbMonit перезапускать или обновлять их,
   агент должен иметь доступ к сокету Docker. Служба, которую регистрирует установщик,
   уже его имеет; если же вы запускаете агента под отдельным пользователем, добавьте этого
   пользователя в группу «docker» и перезапустите агент. Политики перезапуска и автообновления
   затем задаются для каждого контейнера на странице монитора.

Прежде чем разворачивать агент более чем на одной машине, стоит прочитать два раздела:
[Привязка](#binding-one-machine-one-agent), которая обеспечивает
реальную принадлежность монитора одной установке агента, и
[Одноразовый токен или парк машин](#single-use-or-fleet) — выбор, который вы делаете при
создании токена.

    ```
    usermod -aG docker dumbmonit
    ```

6. Резервные копии Plakar: обнаруживаются автоматически — под машиной появляется монитор Plakar, когда
   найден бинарный файл plakar или kloset (~/.config/plakar/stores.yml каждого пользователя,
   ~/.plakar, /var/lib/plakar), а в остальных случаях ничего не показывается. Задайте
   «plakar_klosets» в agent.yaml, чтобы отслеживать фиксированный список, или «plakar: false», чтобы
   отказаться.
7. Службы, которые находит агент (Docker, Plakar, restic/Borg, ZFS, libvirt,
   WireGuard, IPsec, тест скорости), становятся каждая отдельным монитором под
   машиной, со своим статусом и своими оповещениями. Панель «Службы, найденные
   агентом» на странице машины перечисляет, что найдено и что нет: снимите отметку
   с модуля, чтобы убрать его монитор, поставьте — чтобы отслеживать его до
   появления. См. [Службы, найденные агентом](#services-found-by-the-agent).

!!! note
    Агент обращается к серверу, а не наоборот: на отслеживаемой машине
    не нужно открывать ни одного порта.

=== "Linux (systemd или OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Скрипт написан на POSIX `sh` (работает с BusyBox и dash), скачивает с сервера бинарный файл
    для архитектуры машины (x86_64 или aarch64), записывает
    `/etc/dumbmonit/agent.yaml` (режим 0600) и регистрирует `dumbmonit-agent` в
    systemd или в OpenRC на Alpine. Юнит systemd усилен
    (`ProtectSystem=strict`, `NoNewPrivileges`, `MemoryMax=128M`,
    `CPUQuota=20%`).

=== "macOS (launchd)"

    Та же команда, что и в Linux, выполняемая с `sudo`:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Бинарных файлов для macOS **нет** в образе DumbMonit: для их сборки
    требуется SDK Apple, лицензия которого запрещает его распространение. Вместо этого они
    прикреплены к каждому [релизу на GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
    — `dumbmonit-agent-macos-aarch64` (Apple silicon) и
    `dumbmonit-agent-macos-x86_64` (Intel), каждый со своим `.sha256` — а
    сервер перенаправляет `/download/dumbmonit-agent-macos-…` туда, так что скрипт
    выбирает файл для архитектуры Mac (`uname -m`). Сервер не
    поставляет эти бинарные файлы, поэтому команда установки не содержит для них контрольной суммы:
    добавьте `--sha256=macos-aarch64:<hex>` (или `macos-x86_64`) со значением из `.sha256`
    релиза, иначе установщик откажется от загрузки. Mac также
    должен иметь доступ к github.com.

    Без такого доступа или чтобы установить уже имеющийся у вас бинарный файл, скачайте его
    и передайте через `--bin` (сначала сверьте его с `.sha256` релиза:
    `--bin` устанавливает то, что вы ему даёте):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    Скрипт устанавливает бинарный файл в `/usr/local/bin` (и снимает
    атрибут карантина, который несёт файл, скачанный через браузер, и из-за которого
    Gatekeeper отказался бы его запускать), записывает
    `/usr/local/etc/dumbmonit/agent.yaml` (режим 0600) и регистрирует системный
    демон `com.dumbmonit.agent` в `/Library/LaunchDaemons`, запускаемый командой
    `launchctl bootstrap system`. Он работает с момента загрузки, без входа пользователя.

    !!! note "Предпочитаете собрать сами?"
        На самом Mac с установленным Rust:
        `cargo build --release -p dumbmonit-agent`, затем
        `--bin=target/release/dumbmonit-agent`. Это тот же бинарный файл, который
        создаёт задача релиза.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Только x86_64. Скрипт скачивает с сервера `dumbmonit-agent-freebsd-x86_64`,
    записывает `/usr/local/etc/dumbmonit/agent.yaml` (режим 0600) и
    устанавливает `/usr/local/etc/rc.d/dumbmonit_agent`, включаемый командой
    `service dumbmonit_agent enable`. `daemon(8)` присматривает за агентом,
    перезапускает его при остановке и пишет в `/var/log/dumbmonit-agent.log`.

    Работает так же на TrueNAS CORE и других устройствах на базе FreeBSD, где
    ряды ZFS — это и есть главная цель.

=== "Windows (служба)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    Скрипт скачивает с сервера `dumbmonit-agent-windows-x86_64.exe`,
    проверяет его по SHA-256, устанавливает как
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, записывает
    `C:\ProgramData\DumbMonit\agent.yaml` (доступен для чтения только SYSTEM и
    Администраторам) и регистрирует службу Windows `DumbMonitAgent`
    («DumbMonit system agent»): запускается автоматически при загрузке, от LocalSystem,
    и перезапускается после сбоя (через 10 с, 30 с, затем 60 с). Агент сам взаимодействует
    с диспетчером служб (`--service`), поэтому при остановке службы он успевает
    отправить то, что ещё не отправлено, перед завершением.

    У службы нет консоли: агент пишет журнал в
    `C:\ProgramData\DumbMonit\agent.log`, который откладывается как `agent.log.old` при превышении
    10 МБ.

    Нативной сборки для ARM64 нет: в Windows 11 для ARM скрипт устанавливает
    бинарный файл x86_64, который работает в режиме эмуляции x64.

Один токен может регистрировать несколько машин: назовите его по имени группы или машины.

### Нативные бинарные файлы

Агент — это один статический исполняемый файл на каждую систему, рядом с которым
ничего устанавливать не нужно:

| Система | Файл | Где взять |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | сервер, `/download/…` |
| Linux aarch64 (64-битный ARM: Raspberry Pi OS 64-bit, серверы ARM) | `dumbmonit-agent-linux-aarch64` | сервер, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | сервер, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | сервер, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | релиз на GitHub (сервер перенаправляет туда) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | релиз на GitHub (сервер перенаправляет туда) |

Бинарные файлы для Linux статически скомпонованы с musl: они работают в любом
дистрибутиве, с glibc или без (Alpine, прошивки NAS), без подбора библиотек.
Рядом с каждым файлом лежит его контрольная сумма `<file>.sha256` в формате
`sha256sum`. Каждый [релиз на GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
содержит все шесть бинарных файлов и их контрольные суммы — для машин, которые не могут достучаться до
`/download/` сервера, или для установки через `--bin`.

Начиная с 0.1.0-alpha.9 каждый бинарный файл релиза также подписывается без
ключей с помощью [Sigstore](https://www.sigstore.dev/) (`<file>.sigstore.json`) и
сопровождается аттестацией происхождения SLSA. Чтобы проверить, что бинарный файл
собран рабочим процессом релиза этого репозитория:

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'
# or, with the GitHub CLI:
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Чтобы самостоятельно собрать бинарные файлы для Linux, Windows и FreeBSD из клона
репозитория (требуется только Docker):

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Управление службой

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Статус | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Журналы | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Перезапуск | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Остановка | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Конфигурация | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| Файл службы | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | служба `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` в Windows) удаляет всё, включая службу,
в любой системе.

!!! note "Загрузка проверяется — всегда"
    Команда установки, которую показывает DumbMonit, содержит ожидаемый SHA-256
    каждого бинарного файла агента, поставляемого сервером (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` в Windows). Оба установщика сверяют скачанный
    файл с ним, прежде чем что-либо устанавливать, и **отказываются**
    устанавливать, если он не совпадает, если для платформы не указана контрольная сумма
    или если недоступна ни одна утилита SHA-256 (`sha256sum`, `shasum`, `sha256`).

    Контрольные суммы меняются с каждым релизом сервера: после обновления DumbMonit
    скопируйте свежую команду установки из **Settings → Agents** (или из нового
    токена), а не запускайте старую, которая остановится с ошибкой
    «checksum mismatch». Те же контрольные суммы показаны под командой
    установки и опубликованы по адресу `/download/<file>.sha256`.

    Для изолированных (air-gapped) или ручных установок `--insecure-skip-checksum`
    (`-InsecureSkipChecksum`) устанавливает скачанный бинарный файл без проверки и
    громко об этом предупреждает; `--bin=PATH` устанавливает локальный файл как есть, а
    проверка файла остаётся за вами.

### Флаги установщика

| `install.sh` | `install.ps1` | Действие |
|---|---|---|
| `--token=TOKEN` | `-Token` | Токен регистрации (обязателен). |
| `--url=URL` | `-Url` | URL сервера, например `http://server:8080`. В Linux, если флаг отсутствует, используется `DUMBMONIT_URL` из окружения. |
| `--interval=N` | `-Interval` | Период опроса в секундах (по умолчанию 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Службы, состояние которых сообщается, с именами в том виде, как их называет система хоста: юниты systemd в Linux, метки launchd (`com.apple.sshd`) в macOS, имена rc.d во FreeBSD, службы Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Теги, копируемые как `tag_<key>` в каждый ряд. |
| `--hostname=NAME` | `-HostName` | Имя, объявляемое серверу (по умолчанию — имя машины). |
| `--bin=PATH` | `-BinPath` | Локальный бинарный файл для установки вместо скачивания. |
| `--sha256=LIST` | `-Sha256` | Ожидаемый SHA-256 загрузки в виде пар `platform-arch:hex`, разделённых запятыми (одно значение hex тоже подходит). Включён в команду, которую показывает DumbMonit; обязателен, если не указан следующий флаг. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Установить скачанный бинарный файл без проверки, с предупреждением. Только для изолированных или ручных установок. |
| `--no-start` | | Установить всё, но не обращаться к серверу и не запускать службу (образ машины, тестирование). |
| `--uninstall` | `-Uninstall` | Удалить агент и его конфигурацию. |
| `--help` | | Показать справку. |

### Обновление {#upgrade}

Оба скрипта идемпотентны: повторный запуск команды установки с тем же
токеном и URL заменяет бинарный файл, перезаписывает конфигурацию и перезапускает
службу. Это и есть процедура обновления. Берите команду с текущего
сервера (Settings → Agents): её `--sha256` должен соответствовать бинарным файлам той
версии сервера, до которой вы обновились.

Та же команда переносит на новую версию машину, на которой всё ещё работает агент EzyMonit, в Linux
и в Windows. Установщик останавливает и удаляет старую службу
(`ezymonit-agent` или `EzyMonitAgent` в Windows), переносит конфигурацию по
новому пути (`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`)
и устанавливает `dumbmonit-agent` на её место. Старый токен `ezym_…` остаётся
действительным, поэтому команда, показанная при создании токена, по-прежнему работает; заранее
ничего отзывать не нужно.

## Службы, найденные агентом {#services-found-by-the-agent}

Собственный монитор машины показывает саму машину: процессор, память,
файловые системы, сеть, состояние дисков (SMART), датчики, состояние ОС и службы,
которые вы перечисляете в `services`. Всё остальное, что обнаруживает агент, —
это **служба**, и каждая найденная служба становится отдельным монитором типа
`agent_service`, размещённым под машиной:

| Служба | Её метрики | Имя монитора |
|---|---|---|
| Docker | `container_*` | `<machine> · Docker` |
| Plakar | `backup_*` | `<machine> · Plakar` |
| restic / Borg | `agent_backup_repo_*` | `<machine> · restic/Borg` |
| ZFS | `agent_zfs_*` | `<machine> · ZFS` |
| KVM / libvirt | `agent_libvirt_*` | `<machine> · libvirt` |
| WireGuard | `agent_wireguard_*` | `<machine> · WireGuard` |
| IPsec | `agent_ipsec_*` | `<machine> · IPsec` |
| Тест скорости интернета | `agent_speedtest_*` | `<machine> · Speed test` |

Разделение происходит на сервере по имени метрики, поэтому работает с любой
версией агента. Каждый ряд записывается один раз, с `target`, равным монитору,
которому он принадлежит: ряды службы несут id монитора службы, ряды машины — id
машины. Ничего не учитывается дважды.

У монитора службы свои статус, оповещения, правила, переопределения правил и
отключения оповещений. Он **недоступен**, когда агент перестаёт сообщать о нём,
пока машина продолжает отчитываться (демон Docker остановлен, `wg` удалён…), по
истечении тех же трёх пропущенных периодов, что и у самой машины. Когда машина
замолкает, её службы отображаются как скрытые за ней (`suppressed`) вместо того,
чтобы поднимать собственные оповещения. Новый монитор службы начинает с тегов
машины, поэтому отключения и правила на основе тегов продолжают действовать.

Перед установкой выбирать нечего. После первого отчёта страница машины
перечисляет каждый модуль в разделе **Службы, найденные агентом**:

- найденная служба отмечена и ведёт на свой монитор;
- ненайденная служба не отмечена; отметьте её, чтобы всё равно создать её
  монитор (он остаётся недоступным, пока агент не сообщит о ней, — удобно,
  чтобы вас уведомили, если нужная вам служба пропала);
- снимите отметку с найденной службы, чтобы удалить её монитор и отбросить её
  показания. Сервер сообщает об этом агенту в следующем подтверждении, и агент
  перестаёт её собирать (старый агент продолжает собирать, а сервер продолжает
  отбрасывать). Удаление монитора службы даёт тот же эффект. Отметьте её снова,
  чтобы вернуть.

Удаление машины удаляет мониторы её служб. Действия и политики для контейнеров
работают со страницы монитора Docker; команды по-прежнему уходят агенту
машины.

То же, что в уведомлении монитора:

1. Ничего не нужно добавлять вручную: установите агент на машину, и каждая
   найденная им служба становится монитором под этой машиной при первом отчёте.
2. Чтобы перестать отслеживать службу, снимите отметку в панели «Службы,
   найденные агентом» на странице машины или удалите её монитор: показания
   агента для неё после этого отбрасываются.

!!! note
    Если машина перестаёт отчитываться, её службы отображаются как скрытые за ней
    вместо того, чтобы поднимать собственные оповещения.

### Обновление существующей установки {#upgrading-an-existing-installation}

Ничего делать не нужно. Когда агент впервые сообщает о службе после обновления,
сервер создаёт её монитор и переносит на него то, что относилось к этой службе
на машине:

- переопределения правил, читающих метрики службы, копируются на монитор
  службы;
- правила, ограниченные машиной (селектор `ids`) и читающие метрики службы,
  теперь выбирают также монитор службы;
- отключения оповещений (окна обслуживания) машины копируются на монитор
  службы;
- текущие оповещения по рядам службы переносятся на монитор службы под новым
  отпечатком, чтобы они не закрывались и не срабатывали заново.

История остаётся там, где была записана: VictoriaMetrics не умеет менять метки
существующих рядов, поэтому показания до обновления остаются под машиной (они
устаревают по сроку хранения), а графики монитора службы начинаются с момента
обновления. Базовые уровни аномалий перенесённых рядов начинают обучение заново.
История оповещений (закрытые оповещения) тоже остаётся у машины.

## Файл конфигурации {#configuration-file}

`/etc/dumbmonit/agent.yaml` в Linux, `/usr/local/etc/dumbmonit/agent.yaml` в
macOS и FreeBSD, `C:\ProgramData\DumbMonit\agent.yaml` в Windows:

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
libvirt: true              # KVM domains and storage pools through virsh (default: true)
libvirt_bin: virsh
libvirt_uri: qemu:///system
libvirt_interval_secs: 60  # minimum 15
wireguard: true            # WireGuard tunnels through wg (default: true), see below
ipsec: true                # IPsec tunnels through swanctl (default: true), see below
speedtest: false           # Internet speed test (default: false, opt-in), see below
speedtest_download_url: https://speed.example.lan/backend/garbage.php?ckSize=50
speedtest_upload_url: https://speed.example.lan/backend/empty.php
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

`sensors`, `smart`, `zfs` и `libvirt` определяются автоматически, а не предполагаются: если оставить `true` на
машине без датчиков, без `smartctl`, без пулов или без `virsh`, они стоят одного осмотра за период
и вообще ничего не выдают. Установите `false`, чтобы перестать даже смотреть — это стоит сделать
на NAS, к дискам которого `smartctl` лучше не прикасаться, хотя
агент и так передаёт `-n standby`, чтобы спящий диск измерялся, только если он
проснулся по другим причинам.

### Виртуальные машины KVM через libvirt {#kvm-virtual-machines-through-libvirt}

На гипервизоре KVM (обычный хост Linux с libvirt, а не Proxmox VE, у которого
есть [собственная интеграция](proxmox.md)) устанавливайте агент на сам хост, а
не в гостевую систему. Агент запускает `virsh -c qemu:///system` с командами
`list --all --name`, `dominfo --domain <имя>`, `pool-list --all --name` и
`pool-info --pool <имя>`: он никогда не определяет, не запускает, не
останавливает и не переносит домен и никогда не изменяет пул хранилища.
Явный URI `qemu:///system` важен для агента, запущенного участником группы
`libvirt`, а не root: без него `virsh` прочитал бы приватный пустой сеанс этого
пользователя. Задайте `libvirt_uri`, чтобы читать другое подключение.

`agent_libvirt_domain_memory_used_bytes` отсутствует для выключенного домена:
libvirt знает только его настроенный максимум. Размеры пулов хранилища берутся
из удобочитаемого вывода `virsh`, округлённого до двух знаков, поэтому
`agent_libvirt_pool_used_percent` — приближённое значение, достаточно точное,
чтобы предупредить до того, как пул заполнится. Подробно описываются не более
200 доменов и 64 пулов.

Рядом лежит `/etc/dumbmonit/agent-secret` (`C:\ProgramData\DumbMonit\agent-secret`
в Windows), в котором хранится секрет привязки, выданный серверу этой машине. Агент
записывает его с режимом `0600` и перечитывает при каждом запуске; он не входит в
`agent.yaml`, потому что вы его не задаёте — см.
[Привязка](#binding-one-machine-one-agent). `DUMBMONIT_AGENT_SECRET_FILE` переносит
его в другое место, что и нужно для агента в контейнере.

Каждый ключ можно переопределить через окружение, что позволяет использовать агент в
контейнере без монтирования файла:

| Переменная | Действие |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Путь к файлу конфигурации |
| `DUMBMONIT_AGENT_URL` | URL сервера |
| `DUMBMONIT_AGENT_TOKEN` | Токен регистрации |
| `DUMBMONIT_AGENT_SECRET_FILE` | Где хранить секрет привязки (по умолчанию: `agent-secret`, рядом с файлом конфигурации) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Период опроса |
| `DUMBMONIT_AGENT_HOSTNAME` | Имя, объявляемое серверу |
| `DUMBMONIT_AGENT_SERVICES` | Отслеживаемые службы, через запятую |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, через запятую |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Путь к сокету Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Число контейнеров, детализируемых на хост, по умолчанию `200` (`0`: только счётчики) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Исключаемые интерфейсы, имена или регулярные выражения через запятую; по умолчанию `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Интерфейсы, которые нужно оставить; если задано, заменяет список исключений |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Точки монтирования, исключаемые из файловых систем и дискового ввода-вывода; по умолчанию пропускаются `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` — дополнительно отправлять по одному ряду CPU на каждое ядро (по умолчанию `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: температуры и вентиляторы (по умолчанию `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: состояние дисков через `smartctl` (по умолчанию `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Путь к `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Период между двумя считываниями SMART, по умолчанию `300`, минимум `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: пулы ZFS через `zpool` (по умолчанию `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Путь к `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Период между двумя считываниями ZFS, по умолчанию `120`, минимум `30` |
| `DUMBMONIT_AGENT_LIBVIRT` | `true` / `false`: домены KVM и пулы хранилища через `virsh` (по умолчанию `true`) |
| `DUMBMONIT_AGENT_LIBVIRT_BIN` | Путь к `virsh` |
| `DUMBMONIT_AGENT_LIBVIRT_URI` | URI подключения libvirt, по умолчанию `qemu:///system` |
| `DUMBMONIT_AGENT_LIBVIRT_INTERVAL_SECS` | Период между двумя считываниями libvirt, по умолчанию `60`, минимум `15` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: туннели WireGuard через `wg` (по умолчанию `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Путь к `wg` |
| `DUMBMONIT_AGENT_IPSEC` | `true` / `false`: туннели IPsec через `swanctl` (по умолчанию `true`) |
| `DUMBMONIT_AGENT_IPSEC_BIN` | Путь к `swanctl` |
| `DUMBMONIT_AGENT_SPEEDTEST` | `true` / `false`: запускать тест скорости Интернета (по умолчанию `false`) |
| `DUMBMONIT_AGENT_SPEEDTEST_DOWNLOAD_URL` | Адрес, с которого скачивается файл для измерения скорости загрузки |
| `DUMBMONIT_AGENT_SPEEDTEST_UPLOAD_URL` | Адрес, на который отправляется POST для измерения скорости отдачи |
| `DUMBMONIT_AGENT_SPEEDTEST_DOWNLOAD_BYTES_CAP` | Максимум байт, считываемых за тест, по умолчанию `50000000` |
| `DUMBMONIT_AGENT_SPEEDTEST_UPLOAD_BYTES` | Байт, отправляемых за тест, по умолчанию `10000000` |
| `DUMBMONIT_AGENT_SPEEDTEST_INTERVAL_SECS` | Период между двумя тестами, по умолчанию `21600` (6 ч), минимум `300` |
| `DUMBMONIT_AGENT_SPEEDTEST_MIN_DOWNLOAD_MBPS` | Порог оповещения, скорость загрузки в Мбит/с (не задан: оповещения нет) |
| `DUMBMONIT_AGENT_SPEEDTEST_MIN_UPLOAD_MBPS` | Порог оповещения, скорость отдачи в Мбит/с (не задан: оповещения нет) |
| `DUMBMONIT_AGENT_SPEEDTEST_MAX_LATENCY_MS` | Порог оповещения, задержка в миллисекундах (не задан: оповещения нет) |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Путь к `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Путь к `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Период между двумя считываниями репозиториев restic и Borg, по умолчанию `600`, минимум `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` или `auto`: счётчики производительности MDaemon (по умолчанию `auto`, считываются, если существует служба Windows `MDaemon`) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` или `auto`: счётчики производительности Hyper-V (по умолчанию `auto`, считываются, если существует служба Windows `vmms`) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Пути счётчиков производительности Windows, разделённые **точками с запятой** (запятая может входить в имя экземпляра); заменяет `perf_counters` |
| `DUMBMONIT_AGENT_SYSTEM_HEALTH` | `true` / `false`: состояние операционной системы, главный переключатель следующих четырёх (по умолчанию `true`) |
| `DUMBMONIT_AGENT_CHECK_UPDATES` | `true` / `false`: ожидающие обновления (по умолчанию `true`) |
| `DUMBMONIT_AGENT_UPDATES_INTERVAL_SECS` | Период между двумя проверками ожидающих обновлений, по умолчанию `3600`, минимум `300` |
| `DUMBMONIT_AGENT_CHECK_REBOOT` | `true` / `false`: требуется перезагрузка (по умолчанию `true`) |
| `DUMBMONIT_AGENT_CHECK_FAILED_UNITS` | `true` / `false`: упавшие юниты systemd (по умолчанию `true`) |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Размер буфера для догоняющей отправки |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` — выполнять для сервера проверки мониторов, назначенных этому агенту (см. [Запуск агента в Docker / в другой сети](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Метка площадки, показываемая рядом с агентом, когда он предлагается как ретранслятор |

## Счётчики производительности Windows {#windows-performance-counters}

В Windows агент считывает счётчики производительности — значения, которые показывает Performance
Monitor (`perfmon`) — через библиотеку PDH и публикует их в каждый
период опроса. Некоторое программное обеспечение публикует своё состояние только таким образом.

**MDaemon.** Когда существует служба Windows с именем `MDaemon`, агент считывает
объект производительности MDaemon: размеры очередей, сессии, итоги по сообщениям и
спаму, состояние внутренних серверов MDaemon и время его работы. Ряды,
правила и страница, которая их показывает, описаны на странице
[MDaemon](mdaemon.md#mail-queues-through-the-agent). `mdaemon: true`
считывает их даже без службы (переименованная служба), `mdaemon: false`
не делает этого никогда. Наличие службы перепроверяется каждые десять периодов,
поэтому MDaemon, установленный после агента, подхватывается без перезапуска.

**Hyper-V.** Когда существует служба управления виртуальными машинами Hyper-V (`vmms`),
агент считывает объекты производительности Hyper-V; см.
[Хосты Hyper-V](#hyper-v-hosts). `hyperv: true` и `hyperv: false` принудительно задают
выбор, как и для MDaemon.

**Любой другой счётчик.** Укажите его полный путь в `perf_counters` в виде
`\Object(Instance)\Counter` или `\Object\Counter` для объекта с единственным
экземпляром, и он будет опубликован как `dumbmonit_agent_perf_counter{counter="…"}`
с заданным вами именем или самим путём:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Записывайте имена счётчиков **по-английски**, как они отображаются в Performance
  Monitor в английской версии Windows: агент добавляет их через
  `PdhAddEnglishCounterW`, поэтому один и тот же `agent.yaml` работает в французской или
  немецкой версии Windows.
- Заключайте каждый путь в YAML в одинарные кавычки, чтобы обратные косые черты и
  `%` сохранялись как есть.
- Некорректный путь останавливает агент при запуске с указанием ключа и причины.
  Путь, которого Windows не знает (программа не установлена, имя написано
  с ошибкой), однократно фиксируется в журнале сообщением `performance counter not
  available`, а остальные счётчики продолжают считываться; повторная попытка делается каждые
  десять периодов.
- Для скоростей (`…/sec`) нужно два считывания: они появляются со второго периода.
- Считывается не более 200 счётчиков. В Linux, macOS и FreeBSD параметр
  принимается, но ничего не считывается.

Чтобы увидеть точные имена на машине, выполните `typeperf -q MDaemon` (или любое
другое имя объекта) в командной строке.

## Хосты Hyper-V {#hyper-v-hosts}

При установке на хост Hyper-V — Windows Server с ролью Hyper-V или
Windows с включённым Hyper-V — агент находит службу `vmms` и считывает
собственные счётчики производительности Hyper-V, ничего не требуя настраивать. На странице монитора
агента затем появляется панель Hyper-V: сколько виртуальных машин сообщают о
нормальном состоянии, реальная загрузка CPU хоста, память, удерживаемая каждой работающей ВМ, а также
пропускная способность, задержка и ошибки каждого виртуального диска.

!!! warning "Проверено только по документации производителя — на реальной системе ещё не тестировалось"

    Имена счётчиков взяты из документации Microsoft по производительности Hyper-V и
    проверены на соответствие логике на имитируемом
    источнике счётчиков, а не на работающем хосте Hyper-V. Счётчик, которого Windows
    не знает, однократно фиксируется в журнале агента, а остальные продолжают
    считываться. Сообщите нам, что сломалось.

| Счётчик | Ряды | Метки |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: виртуальные машины в каждом состоянии | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: нагрузка хоста, включая гостей | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: доля времени, затраченная на выполнение гостей | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: работающие ВМ плюс сам хост | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: потребность в памяти относительно выделенной памяти; выше 100 ВМ хочет больше, чем имеет | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (среднее), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, счётчик | `disk` |

На хосте Hyper-V `\Processor(_Total)\% Processor Time` измеряет только
собственный раздел хоста, а не то, что потребляют виртуальные машины: счётчик
логических процессоров гипервизора — именно тот, что показывает, насыщена ли
машина. `vm` — это имя виртуальной машины в том виде, как его показывает диспетчер Hyper-V;
`disk` — это имя экземпляра, которое Windows присваивает каждому виртуальному диску, образованное из
пути к его файлу. Экземпляры `_Total` исключаются, и для каждого счётчика считывается не более 200 ВМ или дисков.

Применяются три [встроенных правила](../alerting/rules.md#hyper-v):

- **Hyper-V VM health critical**: Hyper-V сообщает о ВМ в критическом состоянии
  в течение пяти минут (Warning) — чаще всего из-за диска, на котором закончилось место.
- **Hyper-V host CPU high**: логические процессоры заняты более чем на 90 %
  в течение пятнадцати минут (Advisory).
- **Hyper-V virtual disk errors**: виртуальный диск сообщил об ошибках ввода-вывода за
  последний час (Advisory).

## Запуск агента в Docker / в другой сети {#run-the-agent-in-docker--on-another-network}

Агент также публикуется как образ контейнера с теми же тегами, что и
сервер: `ghcr.io/laupernoe/dumbmonit-agent:latest` (последний релиз), `:edge` (последний
коммит в `main`). Это образ `FROM scratch`, содержащий бинарный файл агента и
набор CA Mozilla, настраиваемый целиком через переменные окружения. В
репозитории поставляется готовый к использованию `docker-compose.agent.yml`:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

Что видит контейнер и что нужно смонтировать:

| Потребность | Что сделать | Почему |
|---|---|---|
| CPU, память, нагрузка, дисковый ввод-вывод хоста | Ничего | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` и `/proc/diskstats` относятся ко всему хосту внутри любого контейнера. |
| Имя хоста | `DUMBMONIT_AGENT_HOSTNAME` | Имя хоста контейнера — случайный идентификатор; монитор был бы назван по нему. |
| Контейнеры хоста | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Необязательно. В режиме только для чтения — перечисление и просмотр; для действий Restart/Update нужен доступ на запись и `DUMBMONIT_AGENT_COMMANDS=true`. Без сокета агент просто сообщает об отсутствии контейнеров. |
| Использование файловых систем | `- /:/host:ro` (или нужные вам точки монтирования) | Агент сообщает о точках монтирования, видимых *внутри* контейнера, по их путям в контейнере. |
| Сетевые счётчики хоста | `network_mode: host` | Интерфейсы изолированы в пространствах имён: без этого агент видит один `veth`. |
| Мониторы ping, ретранслируемые через этого агента | `sysctls: net.ipv4.ping_group_range: "0 2147483647"` | ICMP использует сокет echo, который разрешает этот sysctl: capability не нужна. См. [Ping ICMP без capability](../install/docker.md#icmp-ping-without-a-capability). |
| Частный CA | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Сервер за внутренним обратным прокси, устройства с самоподписанными сертификатами. |
| Сохранение привязки при пересоздании | `- agent-state:/var/lib/dumbmonit-agent` и `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | Секрет привязки находится в файловой системе контейнера, которую `docker compose up --force-recreate` отбрасывает. Без тома агент возвращается непривязанным, и каждый раз кому-то приходится разрешать повторную регистрацию на странице монитора. |

`pid: host` и `privileged: true` никогда не нужны: агент не собирает
метрики по отдельным процессам.

### Режим ретранслятора (relay): мониторинг другой площадки

Агент может не только следить за собственной машиной. При `relay: true` (или
`DUMBMONIT_AGENT_RELAY=true`) сервер **делегирует ему проверки**: мониторы,
в настройке *Reached through* которых указан этот агент, опрашиваются не сервером,
а агентом, из его собственной сети, теми же самыми коллекторами —
SNMP, Proxmox VE, Proxmox Backup Server, Synology, HTTP, TCP, DNS, ping, TLS.
Результаты попадают в монитор так, как если бы его опрашивал сервер: те же
метрики, тот же *last probe*, те же оповещения. Удалённый офис, вторая домашняя лаборатория или
сеть клиента, таким образом, отображаются в уже имеющемся у вас единственном DumbMonit
без VPN и без необходимости что-либо открывать на удалённой стороне.

Пошаговая настройка описана в разделе [Мониторинг удалённой площадки](../install/remote-site.md).

## Привязка: одна машина — один агент {#binding-one-machine-one-agent}

Токен регистрации говорит, что *этой машине разрешено обращаться к серверу*. Он не говорит,
*какая* именно машина обращается — для этого служит ключ идентичности
(`/etc/machine-id` или имя хоста), а этот ключ не является секретом: его можно
прочитать на любой машине за секунды.

Поэтому при первом пакете сервер выдаёт каждой машине собственный **секрет привязки**.
Он хранит только отпечаток; агент записывает секрет в
`/etc/dumbmonit/agent-secret` с режимом `0600` и предъявляет его в заголовке
`X-DumbMonit-Agent-Secret` в каждом последующем запросе — измерения,
команды контейнерам и ретранслируемые проверки. С этого момента:

- никакая другая машина не может отправлять измерения от имени этого монитора или переписывать его
  имя хоста и операционную систему;
- никакая другая машина не может получить — а значит, и «съесть» — команды Restart и Update,
  поставленные для него в очередь.

Страница монитора показывает состояние машины в разделе **Agent → Binding**:

| Состояние | Что оно означает | Что делать |
|---|---|---|
| **Bound** | У агента есть собственный секрет. | Ничего. |
| **Not bound — re-enrol this host** | Хост был зарегистрирован до появления привязки и с тех пор не привязывался. Сервер отклоняет его измерения, команды контейнерам и ретранслируемые проверки. | Нажмите **Allow re-enrolment** на странице монитора, затем перезапустите агент. Если агент старше привязки, сначала заново выполните на хосте команду установки. |

### Хосты, появившиеся до привязки {#hosts-that-predate-binding}

!!! warning "Изменено: непривязанные хосты отклоняются"
    Прежние релизы продолжали принимать хосты, зарегистрированные до привязки, пока они
    использовали токен, которым были зарегистрированы, и привязывали их молча, как только
    их агент обновлялся. Это переходное окно закрыто: хост, который никогда не
    привязывался, теперь **отклоняется на каждом маршруте агента** — измерения,
    команды контейнерам и ретранслируемые проверки — пока администратор не зарегистрирует его заново. Токен парка машин
    используется многими машинами, а ключ идентичности можно угадать, поэтому
    ничто другое не доказывает, что обращающийся агент — собственный агент хоста.

    Бинарный файл агента, слишком старый для получения секрета привязки, также отклоняется,
    в том числе при первой регистрации.

Чтобы вернуть такой хост:

1. Если его агент старше привязки, заново выполните на нём команду установки (бинарный файл
   агента должен уметь хранить секрет привязки).
2. Откройте страницу его монитора и нажмите **Agent → Allow re-enrolment**. Это открывает
   окно на один час.
3. Перезапустите агент или дождитесь его следующей повторной попытки: этот пакет привязывает хост
   с новым секретом и закрывает окно. Токен, отличный от того, которым был
   зарегистрирован хост, всё равно должен иметь возможность регистрации (не исчерпан, не просрочен).

До этого момента журнал агента содержит объяснение сервера:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Повторная регистрация после переустановки {#re-enrolment-after-a-reinstall}

Если машина пересобрана, её диск заменён или контейнер её агента пересоздан
без тома с секретом, агент возвращается без него, и
сервер отклоняет его пакеты. Этот отказ намеренный: привязанная машина
остаётся привязанной, пока человек не решит иначе, иначе привязка
ничего бы не защищала.

Путь обратно находится на странице монитора: **Agent → Allow re-enrolment**. Он открывает
окно на один час, в течение которого агент, предъявивший действительный токен регистрации,
снова привязывает эту машину с новым секретом; старый секрет перестаёт работать,
а окно закрывается, как только оно использовано. Тем временем агент сам повторяет попытки,
так что на машине ничего перезапускать не нужно — и ничего не теряется,
поскольку его показания остаются в буфере, пока связь снова не будет принята.

Пока никто не откроет это окно, журнал агента подсказывает, что делать:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Токены регистрации

- Токены создаются в **Settings → Agents** или при добавлении монитора типа
  agent. Сервер хранит только отпечаток: токен в открытом виде показывается один раз, в
  ответе на создание, вместе с двумя командами установки.
- Агент отправляет его как `Authorization: Bearer dmon_…` в `POST /api/ingest`.
  Он никогда не появляется в журналах агента, даже в усечённом виде.
- **Settings → Agents** перечисляет токены с их префиксом, областью действия, датой создания и
  последним использованием. **Revoke** останавливает каждого агента, использующего этот токен, при его
  следующей отправке; затем машины становятся *unreachable* после трёх пропущенных периодов. Заново выполните
  установщик с новым токеном, чтобы зарегистрировать их снова.
- Машины — это отдельные мониторы: отзыв токена их не удаляет.

### Одноразовый токен или парк машин {#single-use-or-fleet}

Область действия токена выбирается при его создании, и это именно выбор, а не
значение по умолчанию, которое просто с вами происходит:

| Область | Что регистрирует | Когда |
|---|---|---|
| **Single use** (по умолчанию) | Ровно одну машину. | Одна установка. Команда установки живёт дальше в истории оболочки, в тикете, в журнале чата; одноразовая после использования ничего не стоит. |
| **Reusable for a fleet** | Столько машин, сколько вы разрешите — число или без ограничений. | Плейбук, образ машины, партия установок. |

Оба могут иметь срок действия в днях. Срок останавливает только *регистрации*: машины,
уже зарегистрированные, продолжают отправлять данные и после его истечения, что и нужно для
окна установки. Отозвать токен — значит отрезать парк машин.

Токен, который больше не может регистрировать — исчерпан или просрочен — показывается как
**Enrols no more** в Settings → Agents. Его машины продолжают отправлять данные; он просто
не может впустить новую.

## Простои и диагностика

Когда сервер недоступен, агент продолжает измерения и буферизует свои
показания в памяти с их исходными метками времени. Они отправляются, как только
связь восстанавливается: перезапуск сервера не оставляет дыр на графиках. Буфер
ограничен (`max_buffered_samples`, по умолчанию около часа) и при заполнении отбрасывает самые старые
показания. Повторные попытки замедляются от 5 секунд до 5 минут с джиттером.

```sh
dumbmonit-agent --dry-run          # print the readings, send nothing
dumbmonit-agent --once             # send one batch then exit
journalctl -u dumbmonit-agent -f   # service log (systemd)
tail -f /var/log/dumbmonit-agent.log   # service log (OpenRC)
```

## Частые ошибки

| Симптом | Вероятная причина |
|---|---|
| Установщик сообщает «could not reach the server» | Неверный URL или токен. Конфигурация записана: исправьте `agent.yaml`, затем перезапустите службу. |
| Загрузка завершается ошибкой 404 | У сервера нет бинарных файлов агента (`DUMBMONIT_AGENT_DIR` пуст). Используйте `--bin=PATH` с собранным вами бинарным файлом. |
| Установщик останавливается с «checksum mismatch» | Команда была скопирована до обновления сервера (скопируйте свежую из Settings → Agents), либо полученный бинарный файл не тот, который отдаёт сервер: на пути прокси или кэш, либо загрузка подменена. |
| Установщик останавливается с «no expected checksum» | В команде нет `--sha256` для этой платформы: старая команда или бинарный файл macOS (не поставляется сервером). Скопируйте свежую команду или передайте `--sha256=<platform>-<arch>:<hex>` из `.sha256` релиза. |
| Машина так и не появляется | Отправка идёт на `/api/ingest` по URL из команды установки; за обратным прокси убедитесь, что он пересылается и что тела запросов до 16 МБ разрешены. |
| *Unreachable*, хотя агент работает | Токен был отозван или отправки отклоняются. Проверьте `journalctl -u dumbmonit-agent`: ошибка записана там. |
| В журнале «this agent is not recognised for this machine» | Машина привязана к установке агента, секрета которой у этого агента нет — переустановка или контейнер, пересозданный без тома состояния. Откройте страницу монитора и нажмите **Allow re-enrolment**; см. [Повторная регистрация после переустановки](#re-enrolment-after-a-reinstall). |
| В журнале «this machine was enrolled before agent binding» | Хост, который никогда не привязывался к своему агенту. Нажмите **Allow re-enrolment** на странице его монитора, затем перезапустите агент; см. [Хосты, появившиеся до привязки](#hosts-that-predate-binding). |
| В журнале «this agent is too old to be bound to its machine» | Бинарный файл агента не умеет хранить секрет привязки. Заново выполните на хосте команду установки; если монитор уже существует, сначала нажмите **Allow re-enrolment**. |
| В журнале «this enrollment token has already enrolled all the machines it was allowed to» | Повторно используется одноразовый токен. Создайте новый или многоразовый токен в Settings → Agents. |
| Windows | На момент написания части, относящиеся к службе Windows, не были проверены в сборочном образе проекта; сообщайте о проблемах на GitHub. |

## Контейнеры Docker {#docker-containers}

Когда агент может читать сокет Docker (`docker: true`, по умолчанию, и пользователь
в группе `docker` или root — `usermod -aG docker dumbmonit`, затем
перезапустите агент), каждый контейнер превращается в несколько рядов с метками
`container` и `image` (`name:tag`, как его показывает `docker ps`). Если страница
монитора сообщает «No Docker on this machine», хотя Docker установлен, это
первое, что нужно проверить.

| Метрика | Значение |
|---|---|
| `container_up` | 1 — работает, 0 — иначе |
| `container_health` | 0 — нет healthcheck, 1 — healthy, 2 — unhealthy, 3 — starting |
| `container_restart_count` | Счётчик перезапусков контейнера в Docker |
| `container_started_seconds` | Секунды с последнего запуска контейнера (0, если остановлен) |
| `container_image_age_seconds` | Секунды с момента сборки образа |
| `container_update_available` | 1, если в реестре есть более новый образ с тем же тегом, 0, если актуален, -1 — неизвестно |

Проверка обновлений сравнивает дайджест локального образа с тем, который
реестр объявляет для тега (`HEAD /v2/<name>/manifests/<tag>`, анонимный
поток токенов в Docker Hub и GHCR). Она выполняется в фоне не чаще одного раза в
час на образ и никогда не задерживает 30-секундный цикл; частный реестр, который
отказывает в анонимном доступе, просто читается как -1. Отключите её через
`docker_update_check: false` (`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Встроенные правила: **Container stopped** (`container_up == 0` в течение 2 минут),
**Container unhealthy** (`container_health == 2` в течение 3 минут), **Container
restarting** (три и более перезапусков за 15 минут), **Container update
available** (info, через один час, повторяется ежедневно). Оповещение называет
контейнер.

### Действия

На странице монитора у каждого контейнера есть два действия и два переключателя. Действия
проходят через **канал команд**: сервер ставит команду в очередь, агент
забирает её после своего следующего пакета (`GET /api/agent/commands`, тот же токен
регистрации), выполняет и сообщает результат (`queued → running → done | failed`)
с коротким журналом, который можно развернуть в интерфейсе. Команды старше десяти
минут никогда не выполняются; агент выполняет по одной команде за раз.

Команду в очереди можно **отменить** из интерфейса, пока агент
её не забрал. Команда, за которой никто не пришёл, **истекает** через десять
минут на сервере, независимо от того, опрашивает ли её агент вообще, — так что машина,
чей агент остановлен, никогда не застревает навсегда с устаревшим «Queued» и никогда не блокирует
следующее действие для этого контейнера.

Агент сообщает серверу с каждым пакетом, принимает ли он команды.
Если нет — агент старше канала команд или установлен
с `commands: false` — страница монитора сообщает об этом вместо кнопок Restart и
Update, API отклоняет команды (`409`), а политики ничего не ставят
в очередь. Переустановите агент текущим установщиком, чтобы включить действия.

- **Restart** — `docker restart` для работающего контейнера, `docker start` для
  остановленного, затем 20-секундная проверка, что он работает.
- **Update now** — загружает тег, который контейнер уже использует, создаёт новый
  контейнер с той же конфигурацией (окружение, монтирования, порты, метки,
  сети, политика перезапуска), останавливает старый, меняет имена местами, запускает новый
  и ждёт его healthcheck (или 20 секунд плюс проверку работы, если
  у образа его нет). Если что-либо терпит неудачу, агент **откатывается**: новый
  контейнер удаляется, а старый перезапускается, и команда сообщает
  `failed` с указанием причины. При успехе старый контейнер удаляется и, если
  включена опция «prune», старый образ тоже — если только его не использует другой контейнер.

Контейнеры под управлением Compose (метки `com.docker.compose.*`) обновляются так же,
но в результате выдаётся предупреждение, что следующий `docker compose up` пересоздаст
контейнер из файла compose. Контейнеры, имя которых начинается с `dumbmonit`
или `dumbmonit`, отклоняются: монитор никогда не обновляет сам себя.

### Политики

**Политика** каждого контейнера оценивается сервером каждую минуту:

- **Restart if down** — контейнер, сообщающий `container_up == 0`, перезапускается
  автоматически, не чаще одного раза в десять минут.
- **Auto-update in maintenance windows** — контейнер с доступным обновлением
  обновляется автоматически, но только пока действует окно обслуживания (silence)
  охватывает монитор, не чаще одного раза в шесть часов. Запланируйте его на странице Alerts
  или переведите монитор в silence на его странице. По умолчанию политики удаляют
  старый образ и действуют только в окнах обслуживания.

Задать их можно в двух местах:

- **полоса Docker** под заголовком монитора: «28 containers · 27 running ·
  3 updates available · policies: 2 auto-restart, 0 auto-update», последнее
  действие и его статус, **Manage containers** (открывает список ниже) и
  **Policies…** — таблица всех контейнеров с двумя переключателями и строкой
  «Apply to all» — самый быстрый способ включить автоперезапуск для всей
  машины;
- переключатели внутри каждой строки раздела **Containers**, рядом с кнопками
  Restart и Update now.

Каждый переключатель сохраняется сам по себе (`PUT /api/targets/<id>/containers/<name>/policy`);
сбой касается только этой строки и показывается рядом с ней.

Автоматические действия отображаются в «Recent actions» как запрошенные `policy`.

### Правила безопасности

- Агент действует, только если `commands: true` (по умолчанию); установите `false`
  для машины, которую нужно только измерять.
- Агент сам забирает команды, используя свой токен регистрации — ничто
  не подключается к машине. Они истекают через десять минут и выполняются по одной;
  агент никогда не загружает тег, отличный от того, который контейнер уже
  использует.
- Обновление, которое не вернулось в исправном состоянии, откатывается к прежнему
  контейнеру, а старый образ удаляется только после того, как новый контейнер заработал.
- Контейнеры с именами `dumbmonit*` или `ezymonit*` отклоняются, вручную или по
  политике: монитор никогда не действует на собственные контейнеры. Проверка выполняется дважды
  — сервер отказывается ставить команду в очередь, а агент отказывается её выполнять —
  и агент повторяет её по имени, которое Docker сообщает после инспекции
  контейнера, так что указание контейнера по его id ничего не меняет. Агент также отказывается
  действовать на собственный контейнер, как бы тот ни назывался: его не оказалось бы на месте,
  чтобы завершить работу или отчитаться о ней.
- Контейнеры под управлением Compose обновляются, но следующий `docker compose up`
  пересоздаёт их из файла compose: поднимите тег и там.

## Резервные копии Plakar

Агент может отслеживать kloset'ы (репозитории) [Plakar](https://plakar.io): каждые
десять минут, в фоне, он выполняет `plakar at <kloset> ls` и `info` для
каждого из них и заново выдаёт последнее показание в каждом цикле.

Plakar **определяется автоматически, а не предполагается**. При запуске, а затем в каждом цикле,
пока что-либо не найдено, агент ищет бинарный файл `plakar` (в `PATH`
или `plakar_bin`) и kloset'ы в стандартных местах. Если нет
ни бинарного файла, ни kloset'а, ни следов Plakar (`~/.cache/plakar`), агент не выдаёт
ни одного ряда `backup_*`, один раз сообщает об этом на уровне `debug`, а страница
монитора не показывает панель Backups — на машинах без Plakar ничего настраивать не нужно.
Чтобы полностью пропустить обнаружение:

```yaml
plakar: false                        # or DUMBMONIT_AGENT_PLAKAR=false
```

### Обнаружение

Настраивать ничего не нужно: когда `plakar_klosets` пуст, агент находит
kloset'ы сам, в следующем порядке, и читает каждое место только один раз:

1. собственный каталог конфигурации агента — `$XDG_CONFIG_HOME/plakar` или
   `~/.config/plakar` учётной записи, под которой он работает (`plakar_home`, если задан);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar` в алфавитном порядке;
4. `/var/lib/plakar`, если этот каталог существует.

В каждом каталоге он читает `stores.yml` (или `stores.yaml`; устаревший
`plakar.yml` тоже принимается) и берёт каждую запись из карты `stores` (а также
`repositories` и `klosets`), у которой есть `location`. Читается только расположение:
парольные фразы и секреты rclone, находящиеся рядом, никогда не сохраняются и не логируются.
Каждая запись становится kloset'ом `@<name>`, читаемым командой `plakar -configdir <dir> at
@<name>`, так что kloset, созданный пользователем, читается, даже когда агент работает от
root. `~/.plakar` каждой учётной записи добавляется, если существует (kloset
по умолчанию для `plakar create`). Два аккаунта, одинаково называющие разные kloset'ы,
различаются как `@name` и `<user>:@name`.

Результат один раз записывается в журнал на уровне `info` (и снова только при изменении). Два
показателя (gauge), выдаваемые только после обнаружения Plakar, позволяют интерфейсу различать
оставшиеся случаи: `backup_plakar_present` (1, когда бинарный файл запускается) и
`backup_klosets_found` (число прочитанных kloset'ов). Страница монитора тогда сообщает
«Plakar is installed but no kloset was found» или перечисляет kloset'ы. Когда
kloset'ы существуют, но бинарный файл не запускается (как правило, `PATH` службы без `plakar`),
они перечисляются как нечитаемые с подсказкой задать `plakar_bin`, и
агент один раз выдаёт предупреждение.

Чтобы вместо этого отслеживать фиксированный список, укажите его — обнаружение тогда пропускается:

```yaml
plakar: true                         # default: detect Plakar; false hides it entirely
plakar_bin: /usr/bin/plakar          # default: "plakar" on PATH
plakar_klosets:                      # explicit list: overrides discovery
  - /srv/backups/main
  - "@diskext"                       # a named store of the agent's account
plakar_home: /home/noe               # optional: HOME (and config dir) used when running plakar
plakar_interval_secs: 600            # minimum 60
```

| Переменная | Действие |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — отслеживать резервные копии Plakar при обнаружении (по умолчанию `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Путь к бинарному файлу `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Kloset'ы через запятую (переопределяет обнаружение) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Домашний каталог, используемый при запуске `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Секунды между двумя считываниями |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — проверка реестра на обновления контейнеров |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — принимать действия от сервера |

Зашифрованному kloset'у нужна его парольная фраза: укажите её в записи хранилища
(`plakar store set <name> passphrase=…`, что и делает `plakar store add`)
или экспортируйте `PLAKAR_PASSPHRASE` в окружении службы агента. Без неё
kloset читается как «Cannot be opened» (`backup_last_status = 0`).

### Метрики и правила

Метрики с метками `kloset` и `source` (каталог, для которого создана резервная копия):
`backup_last_success_seconds` (возраст самого нового снимка),
`backup_snapshot_count`, `backup_last_status` (1, когда в самом новом снимке
нет ошибок, 0, когда есть ошибки или kloset не удаётся открыть),
`backup_size_bytes{kloset}` (размер хранилища репозитория), а также
`backup_plakar_present` и `backup_klosets_found`, описанные выше. Ни одной из
них не существует, пока Plakar не обнаружен, поэтому встроенные правила ниже не могут
сработать на машине без Plakar.

Встроенные правила: **Plakar backup too old** (advisory, нет снимка более
двух дней) и **Plakar backup failed** (warning). Страница монитора показывает один
блок на каждый kloset — возраст последней резервной копии, число снимков, размер, последний статус —
и одну строку на каждый источник, с подсказкой «Backup too old» после двух дней.

### Первая резервная копия

С установленным на машине Plakar 1.1 (`plakar version`), от имени пользователя, который
будет владельцем резервных копий:

```sh
# 1. a kloset store, named so the agent finds it in ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # or any location Plakar accepts
plakar at @diskext create                                   # add -plaintext to skip encryption

# 2. a first backup, then check it
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` и `plakar at <path> backup <dir>` работают так же с
обычным каталогом вместо `@name`; `plakar create` без `at` использует
`~/.plakar`, который агент тоже находит. Запланируйте `plakar at @diskext backup
/srv/photos` через cron или таймер systemd: агент сообщает возраст самого нового
снимка и оповещает, если проходит день без него — два дня для
встроенного правила.

Чтобы попробовать локально с незашифрованным kloset'ом:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

затем добавьте `plakar_klosets: ["/tmp/plakar-demo"]` в `agent.yaml` (голый путь
вне домашнего каталога не обнаруживается) и убедитесь через `dumbmonit-agent --dry-run`,
что появляются образцы `backup_*`.

## Резервные копии restic и Borg {#restic-and-borg-backups}

Агент может отслеживать репозитории [restic](https://restic.net) и
[Borg](https://www.borgbackup.org): каждые десять минут, в фоне,
он открывает каждый объявленный вами репозиторий и сообщает, удалось ли его
прочитать и насколько стар его самый новый снимок. Ничего не обнаруживается автоматически: у
репозитория нет стандартного расположения, и его нельзя открыть без пароля,
поэтому каждый из них перечисляется в `agent.yaml` вместе со способом получения пароля.

**Пароль никогда не покидает машину.** Агент передаёт его `restic` или
`borg` только через окружение дочернего процесса; он никогда не логируется,
никогда не отправляется на сервер, а ряды несут только имя репозитория.

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

Делайте `agent.yaml` и файлы с паролями доступными для чтения только root
(`chmod 600`), как это делает установщик для `agent.yaml`. `password_command`
соответствует `RESTIC_PASSWORD_COMMAND` или `BORG_PASSCOMMAND`. Всё, что
инструменты читают из своего окружения, можно поместить в `env` (`RESTIC_PASSWORD`,
`BORG_PASSPHRASE`, облачные ключи, `BORG_RSH`…), а собственное окружение службы
агента тоже передаётся дальше.

Что запускает агент — только чтение в самом строгом смысле:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: блокировка не
  создаётся, поэтому можно отслеживать репозиторий только для чтения или только для добавления, а
  `prune`, удерживающий исключительную блокировку, не заставляет репозиторий выглядеть недоступным;
  кэша тоже нет, поскольку установленная служба не может писать в `/root`.
- `borg --bypass-lock list --json --last 1`: без `--bypass-lock` считывание
  завершалось бы ошибкой на всё время каждого `borg create`, который удерживает
  исключительную блокировку. Borg записывает небольшую запись безопасности в
  `~/.config/borg`; когда этот домашний каталог недоступен для записи (установленная служба работает
  с `ProtectHome=yes`), агент вместо этого указывает `BORG_BASE_DIR` на свой приватный
  `/tmp`, если только вы сами не задали `BORG_BASE_DIR`. Репозиторию в режиме
  `keyfile` тогда нужен `BORG_KEY_FILE` в его `env`.

Поскольку служба работает с `ProtectHome=yes`, храните всё необходимое инструментам
вне домашних каталогов: файлы с паролями, ключи SSH и `known_hosts` — в
`/etc/dumbmonit/`, как в примере, а локальные репозитории — в `/srv`,
`/mnt` или `/var`, а не в `/home`. Считывания выполняются одно за другим,
в пределах ограничения памяти службы (128 МиБ вместе с самим агентом).

Метрики с метками `tool` (`restic` или `borg`) и `repo`:
`agent_backup_repo_reachable` (1, когда последнее считывание удалось, иначе 0:
репозиторий недоступен, пароль отклонён, бинарный файл отсутствует — агент записывает в журнал
причину один раз и снова при её изменении), `agent_backup_repo_last_snapshot_age_seconds`
(возраст самого нового снимка, отсутствует, пока репозиторий пуст) и
`agent_backup_repo_last_snapshot_bytes` (байты, обработанные самым новым снимком restic,
restic 0.17 и новее). При `--latest 1` restic возвращает самый новый
снимок для каждого хоста и набора путей; сохраняется самый новый из всех, поэтому
задавайте `host` для репозитория, общего для нескольких машин.

Встроенные правила: **restic/Borg backup too old** (нет снимка более 48
часов) и **restic/Borg repository unreachable** (в течение 30 минут). Страница
монитора показывает панель Backup repositories, по одной строке на репозиторий.

Чтобы проверить объявление, запустите агент один раз на переднем плане и найдите
образцы `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Туннели WireGuard {#wireguard-tunnels}

На машине, где работает WireGuard, агент считывает каждый интерфейс и каждого
пира командой `wg show all dump` в каждом цикле. Настраивать ничего не нужно: без
команды `wg` (wireguard-tools) или без интерфейса ряды не выдаются
и панель не показывается. `wg` требует root (`CAP_NET_ADMIN`); установленная служба
его имеет, а агент, запущенный вручную под вашей учётной записью, один раз записывает отказ в журнал
и ничего не сообщает.

Дамп также содержит закрытый ключ интерфейса и предварительно разделённый ключ каждого пира:
агент пропускает оба поля, никогда не копируя их.

Для каждого пира, с метками `interface`, `peer` (его открытый ключ) и `allowed_ips`:
`agent_wireguard_peer_handshake_age_seconds` (секунды с последнего
handshake), `agent_wireguard_peer_has_handshake` (0, пока пир ни разу
его не завершил), `agent_wireguard_peer_keepalive_seconds` (его постоянный
keepalive, 0 при отключённом) и счётчики `agent_wireguard_peer_rx_bytes` и
`agent_wireguard_peer_tx_bytes`; для каждого интерфейса —
`agent_wireguard_interface_peers`. Для пира, который ни разу не завершил
handshake, возраст отсчитывается с момента, когда агент впервые его увидел: туннель,
который так и не поднялся, тоже считается молчащим и сообщается как таковой.

У WireGuard нет имён пиров. Задайте свои в `agent.yaml` по открытому ключу, и
они станут меткой `name`:

```yaml
wireguard: true                     # default; false stops even looking
wireguard_bin: wg                   # default: on PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

Встроенное правило **WireGuard peer silent** срабатывает, когда пир **с постоянным
keepalive** не завершал handshake более 15
минут. Пока туннель поднят, WireGuard обновляет handshake каждые две
минуты, а keepalive поддерживает его даже без трафика; пир без
keepalive (телефон, ноутбук) может простаивать днями, и ничего не сломано, поэтому он
исключён. Задайте `PersistentKeepalive = 25` на туннелях, которые должны оставаться поднятыми
(связи между площадками, удалённые шлюзы). N — это порог правила: измените его в
**Alerts → Rules** или удалите часть запроса `and on (…)`, чтобы следить за
каждым пиром. Страница монитора показывает панель WireGuard с каждым пиром, временем
с его последнего handshake, его трафиком и его keepalive.

## Туннели IPsec {#ipsec-tunnels}

На машине с strongSwan агент считывает каждый настроенный туннель двумя
командами только для чтения: `swanctl --list-conns` — для CHILD_SA, объявленных в
`swanctl.conf` (или в `ipsec.conf` через `swanctl --load-all`, собственный слой
совместимости strongSwan — агенту всё равно, чем написана конфигурация),
и `swanctl --list-sas` — для реально установленных. Настраивать ничего не нужно:
без команды `swanctl` или без настроенного соединения ряды не выдаются и панель
не показывается. Агент никогда не отправляет `--initiate`, `--terminate` и любые
другие команды, меняющие состояние туннеля; он лишь читает то, что `swanctl` и так
показывает в командной строке.

Чтение обеих команд, а не только активной, позволяет сообщить о туннеле,
который **ни разу** не поднимался, как о неработающем, а не оставить его
невидимым: CHILD_SA, объявленный в конфигурации, но отсутствующий в
`--list-sas`, считается неработающим с первого цикла.

Для каждого туннеля, с метками `connection` (имя IKE_SA) и `tunnel` (имя
CHILD_SA): `agent_ipsec_tunnel_up` (1 — установлен, 0 — иначе) и, только
пока туннель поднят, счётчики `agent_ipsec_tunnel_bytes_in` / `agent_ipsec_tunnel_bytes_out`
и `agent_ipsec_tunnel_rekey_seconds` (время до следующего rekey).

```yaml
ipsec: true                # default; false stops even looking
ipsec_bin: swanctl          # default: on PATH
```

Встроенное правило **IPsec tunnel down** срабатывает, когда у настроенного туннеля
нет установленного CHILD_SA в течение десяти минут. Страница монитора показывает
панель IPsec с каждым туннелем, поднятым или нет, его трафиком и временем до
следующего rekey.

## Тест скорости Интернета {#internet-speed-test}

По умолчанию выключен: в отличие от всего остального, что измеряет агент, этот тест
**расходует полосу** — а на лимитированном или тарифицируемом канале и деньги —
при каждом запуске. Для включения нужны и `speedtest: true`, и адрес для
загрузки; без адреса ничего не запускается, даже если `speedtest` равен `true`.

**Сервер за вас не выбирается.** DumbMonit не поставляется с адресом по умолчанию и
никогда не обращается к третьей стороне, пока вы её не назвали: вы сами решаете,
относительно чьего сервера измеряется ваш канал и кто видит адрес агента во время теста.

Тест — обычный HTTP, на дальней стороне только чтение: `GET` на
`speedtest_download_url`, читаемый по частям и прерываемый, как только прочитано
`speedtest_download_bytes_cap` байт (по умолчанию 50 МБ, что наступит раньше),
даёт скорость загрузки; `POST` из `speedtest_upload_bytes` байт (по умолчанию 10 МБ)
на `speedtest_upload_url`, если он задан, даёт скорость отдачи; несколько запросов
`HEAD` на адрес загрузки дают задержку (наименьшее время кругового пути) и джиттер
(разброс между наименьшим и наибольшим). Сервер, отклоняющий `HEAD`, просто оставляет
задержку и джиттер неизмеренными — остальной тест от них не зависит.

Это работает с собственным [LibreSpeed](https://github.com/librespeed/speedtest)
(`backend/garbage.php?ckSize=N` для загрузки, `backend/empty.php` для
отдачи), с [конечными точками теста скорости Cloudflare](https://speed.cloudflare.com/)
(`https://speed.cloudflare.com/__down?bytes=N` и
`https://speed.cloudflare.com/__up`, которые может вызывать любой без
учётной записи) или с любым вашим обычным HTTP-файлом и конечной точкой отправки —
реализовывать протокол конкретного вендора не нужно, и новых зависимостей
сверх HTTP-клиента, который агент уже использует, нет.

```yaml
speedtest: false                    # default: opt-in, this test uses bandwidth
speedtest_download_url: https://speed.cloudflare.com/__down?bytes=50000000
speedtest_upload_url: https://speed.cloudflare.com/__up
speedtest_download_bytes_cap: 50000000   # default; read at most this many bytes
speedtest_upload_bytes: 10000000         # default; sent in one block
speedtest_interval_secs: 21600           # default: 6 hours, minimum 300
speedtest_min_download_mbps: 50          # optional: arms "Internet download rate low"
speedtest_min_upload_mbps: 10            # optional: arms "Internet upload rate low"
speedtest_max_latency_ms: 80             # optional: arms "Internet latency high"
```

Тест выполняется в фоне не чаще одного раза за `speedtest_interval_secs`
(по умолчанию 6 часов, не менее 300 секунд: меньшее значение останавливает агент
при запуске); один тест также запускается через несколько секунд после каждого старта агента;
цикл сбора лишь читает его последний результат, точно так же, как для
restic/Borg и Plakar. `agent_speedtest_ok` равен 0, когда не удалась сама загрузка
(недоступный адрес, тайм-аут, ошибка HTTP) — единственный сбой, при котором тест
вообще ничего не может измерить.

Три порога намеренно необязательны: без `speedtest_min_download_mbps` никто в
DumbMonit не знает, что значит «медленно» для вашего канала, поэтому
**Internet download rate low**, **Internet upload rate low** и
**Internet latency high** молчат, пока вы не зададите нужный порог.
Каждый порог публикуется отдельным рядом
(`agent_speedtest_download_min_bytes_per_second` и так далее), и именно с ним
встроенные правила сравнивают измерение; меняйте порог в
`agent.yaml`, а не в **Alerts → Rules**. Страница монитора показывает панель
теста скорости с последними скоростями загрузки и отдачи, задержкой, джиттером и временем,
прошедшим с теста.

## Сертификаты TLS (Certbot / Let's Encrypt)

Агент не читает `/etc/letsencrypt` напрямую. Каждый сертификат,
обновляемый Certbot, где-то отдаётся — веб-сервером, почтовым сервером, обратным
прокси — и именно там DumbMonit его и читает: добавьте монитор
[TLS certificate](services.md#tls) (или [Website or web API](services.md#http), для сайта, уже
отслеживаемого таким образом), указывающий на порт, на котором сертификат действительно предъявляется.
Он читает действующий сертификат, а не файл на диске, поэтому также ловит обновление,
которое прошло успешно в `/etc/letsencrypt`, но так и не было подхвачено
службой (обратный прокси, которому нужна перезагрузка, чтобы увидеть новый файл, — обычный случай
и как раз тот пробел, который важен).

Чтение `/etc/letsencrypt/live/*/cert.pem`, конфигурации обновления и журнала
Certbot из агента рассматривалось и было отвергнуто: оно дублировало бы
дату истечения, которую уже сообщает монитор TLS, добавляя второй
локальный для хоста путь к той же информации — ещё одну вещь, которую нужно поддерживать в согласованном состоянии
после переноса сертификата или переименования домена. Хост, где Certbot работает,
но не отдаёт ничего, до чего DumbMonit может достучаться напрямую (внутренний шаг CA, хук
проверки DNS), — единственный случай, который остаётся не охваченным; это также случай,
где монитор TLS тоже не помог бы, поскольку на этом хосте нет ничего, что говорит
по TLS и что можно было бы проверить.
