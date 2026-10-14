# Agente para Linux, macOS, FreeBSD y Windows {#linux-macos-freebsd-and-windows-agent}

Un pequeño binario instalado en la máquina a monitorizar. Mide CPU,
memoria, discos, red, servicios y contenedores — y, donde la máquina
los expone, temperaturas, salud de disco, pools ZFS y máquinas virtuales KVM — y luego **envía**
sus lecturas al servidor DumbMonit por HTTP.

La dirección es deliberada: el agente llama al servidor, nunca al
revés. Cruza NAT y cortafuegos domésticos, y no requiere abrir
**ningún puerto** en la máquina monitorizada.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_`. Las etiquetas de
identidad (`target`, `host`, `tag_*`) las define el servidor al
recibir, nunca el agente: una máquina no puede escribir en la serie de
otra máquina, ni siquiera falsificando sus etiquetas.

| Familia | Métricas | Etiquetas |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15`; `cpu_core_usage_percent` solo con `cpu_per_core: true` | `core` |
| Memoria | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Sistemas de archivos | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Red (contadores) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Discos (contadores) | `disk_read_bytes`, `disk_written_bytes`, una serie por dispositivo de bloque | `device` |
| Host | `uptime_seconds`, `process_count` | |
| Servicios | `service_up` (1 = en ejecución) | `service` |
| Contenedores | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (contenedores más allá de `docker_max_containers`) | `container`, `image` |
| Sensores | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (el umbral que el propio hardware declara), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Salud de disco (SMART) | `agent_disk_smart_ok` (1 = el disco pasa su propia autoevaluación), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| KVM / libvirt | `agent_libvirt_domains`, `agent_libvirt_domains_running`, `agent_libvirt_pools`; por dominio `agent_libvirt_domain_state_info`, `agent_libvirt_domain_running`, `agent_libvirt_domain_crashed`, `agent_libvirt_domain_vcpus`, `agent_libvirt_domain_memory_max_bytes`, `agent_libvirt_domain_memory_used_bytes`; por pool de almacenamiento `agent_libvirt_pool_state_info`, `agent_libvirt_pool_active`, `agent_libvirt_pool_capacity_bytes`, `agent_libvirt_pool_allocation_bytes`, `agent_libvirt_pool_used_percent` — consulta [Máquinas virtuales KVM con libvirt](#kvm-virtual-machines-through-libvirt) | `domain`, `pool`, `state` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 degraded, 2 peor), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`; por peer `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (contadores) — consulta [Túneles WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — consulta [Copias de seguridad restic y Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Contadores de rendimiento de Windows | `agent_perf_counter`, una serie por contador listado en `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — consulta [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions`; por VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent`; por disco virtual `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — consulta [Hosts Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Agente | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Una serie que no se puede medir está **ausente**, nunca en cero: sin
`smartctl`, sin `agent_disk_*`; sin sonda legible, sin
`agent_sensor_*`; sin ZFS, sin `agent_zfs_*`; sin `virsh`, sin `agent_libvirt_*`. Una temperatura fuera de
1–150 °C se descarta en lugar de publicarse, y una lectura de
ventilador de 0 rpm también se descarta — una placa base tiene más
conectores de ventilador que ventiladores, y uno vacío lee cero igual
que lo haría un ventilador muerto.

### Qué se recolecta dónde {#what-is-collected-where}

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, memoria, swap, sistemas de archivos, red, tiempo de actividad, procesos | sí | sí | sí | sí |
| Carga media | sí | sí | sí | no (Windows no tiene esa noción) |
| Servicios | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | servicios de Windows |
| Contenedores Docker | sí | sí, si `docker_socket` apunta al socket de Docker Desktop | no (sin demonio Docker) | no |
| Temperaturas | `/sys/class/hwmon` | sensores SMC | `dev.cpu.N.temperature` (solo CPU) | no |
| Ventiladores | `/sys/class/hwmon` | no | no | no |
| Salud de disco (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Pools ZFS | `zpool` (OpenZFS) | no | `zpool` | no |
| Máquinas virtuales KVM (libvirt) | `virsh` | no | no | no |
| Salud del SO (actualizaciones, reinicio, unidades fallidas, SELinux) | sí | no | no | no |
| Copias de seguridad Plakar | sí | sí | sí | sí |
| Túneles WireGuard | `wg show` | cuando `wg` está instalado (no probado) | cuando `wg` está instalado (no probado) | no |
| Copias de seguridad restic y Borg | sí | sí | sí | solo restic (no probado; Borg no tiene build para Windows) |
| Contadores de rendimiento, colas MDaemon, Hyper-V | no | no | no | sí |
| Identidad de máquina | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | nombre de host |
| Modo relay | sí | sí | sí | sí |

Windows deliberadamente no tiene fila de temperatura: leer sensores
ahí significa pasar por WMI, con derechos administrativos, para
valores que la mayoría de máquinas no exponen en absoluto. No publicar
nada es la respuesta honesta.

`smartctl` (smartmontools), `zpool` y `virsh` se buscan en cada período de
lectura y se usan cuando están presentes; ninguno lo instala el
agente, y ninguno es obligatorio. Todos necesitan root (`virsh` también funciona para un miembro del grupo `libvirt`) — el servicio
que registra el instalador se ejecuta como root, un agente iniciado a
mano con tu propia cuenta no encontrará nada.

Los contadores se envían en bruto, para que un reinicio del agente
nunca produzca una tasa falsa; aplica `rate()` en las consultas.

Las reglas integradas que se aplican: Device unreachable, High CPU,
Unusual CPU. «Device unreachable» también funciona en agentes: el
servidor comprueba la antigüedad del último lote recibido y declara la
máquina silenciosa tras tres períodos perdidos (al menos 90 segundos).

Cuatro más se activan solo en máquinas que producen la serie
correspondiente, así que no necesitan ninguna configuración y no
pueden activarse en una máquina sin el hardware:

| Regla | Se activa cuando |
|---|---|
| **Disk SMART failing** (critical) | la propia autoevaluación de un disco lo reporta como failing |
| **ZFS pool degraded** (critical) | un pool ya no está `ONLINE` durante cinco minutos |
| **ZFS scrub found errors** (critical) | el último scrub encontró errores, o el pool reporta archivos permanentemente corruptos |
| **KVM domain crashed** (critical) | libvirt reporta un dominio como `crashed` |
| **libvirt pool almost full** (warning) | la asignación de un pool de almacenamiento alcanza el 90 % de su capacidad durante quince minutos |
| **Temperature above critical** (critical) | un sensor supera el umbral crítico que declara **su propio hardware** — ningún valor está escrito a fuego, siendo 85 °C una emergencia en un disco y una tarde normal en la CPU de un portátil |

Tres más vigilan lo que tú declaras o ejecutas tú mismo, y permanecen
en silencio en caso contrario: **WireGuard peer silent** (un peer con
keepalive persistente y sin handshake durante 15 minutos),
**restic/Borg backup too old** (sin snapshot durante 48 horas) y
**restic/Borg repository unreachable** — consulta [Túneles
WireGuard](#wireguard-tunnels) y [Copias de seguridad restic y
Borg](#restic-and-borg-backups).

En un servidor Windows que ejecuta MDaemon, tres más vigilan sus
colas: **MDaemon mail queue growing**, **MDaemon Bad queue not empty**
y **MDaemon Retry queue high** (consulta
[Reglas](../alerting/rules.md#mdaemon-and-securitygateway)). En un
host Hyper-V, tres vigilan las máquinas virtuales: **Hyper-V VM
health critical**, **Hyper-V host CPU high** y **Hyper-V virtual disk
errors** (consulta [Hosts Hyper-V](#hyper-v-hosts)).

## Instalación {#install}

Los pasos de abajo son los que muestra el aviso junto al formulario.
No hay ninguna cuenta que crear en la máquina: el token de inscripción
es el único secreto.

1. Guarda el dispositivo en DumbMonit: un token de inscripción
   (dmon_…) se muestra una vez, con el comando de instalación listo
   para copiar para Linux y para Windows. (Un token también se puede
   crear en **Settings → Agents**.)
2. Ese token es la clave del agente para enviar sus mediciones a
   DumbMonit. No es una cuenta en la máquina: nada que crear ahí. Un
   token creado desde el formulario de dispositivo es de un solo uso:
   inscribe esa única máquina y nada más. Para una flota, crea un
   token reutilizable en Settings → Agents.
3. Ejecuta el comando de instalación en la máquina a monitorizar, con
   derechos elevados (sudo en Linux y macOS, un PowerShell elevado en
   Windows). Descarga el agente, escribe el token en agent.yaml e
   inicia el servicio.
4. La máquina aparece por sí sola en unos segundos, nombrada según su
   nombre de host. En ese primer lote el servidor le da al agente un
   secreto propio y lo guarda en `/etc/dumbmonit/agent-secret`,
   legible por nadie más: desde entonces, esa máquina es la única que
   puede reportar bajo este dispositivo. ¿Perdiste el token?
   Settings → Agents te permite revocarlo y crear otro.
5. Docker: para ver los contenedores y permitir que DumbMonit los
   reinicie o actualice, el agente debe alcanzar el socket de Docker.
   El servicio que registra el instalador ya puede; si ejecutas el
   agente bajo un usuario dedicado en su lugar, añade ese usuario al
   grupo «docker» y reinícialo. Las políticas de reinicio y
   auto-actualización entonces se definen por contenedor en la página
   del dispositivo.

Dos cosas vale la pena leer antes de desplegar esto a más de una
máquina: [Vinculación](#binding-one-machine-one-agent), que es lo que
hace que un dispositivo realmente pertenezca a una instalación de
agente, y [Un solo uso o flota](#single-use-or-fleet), que es la
elección que haces al crear un token.

    ```
    usermod -aG docker dumbmonit
    ```

6. Copias de seguridad Plakar: detectadas automáticamente — el panel
   Backups aparece cuando se encuentra el binario plakar o un kloset
   (~/.config/plakar/stores.yml de cada usuario, ~/.plakar,
   /var/lib/plakar), y no se muestra nada en caso contrario. Define
   «plakar_klosets» en agent.yaml para vigilar una lista fija, o
   «plakar: false» para desactivarlo.

!!! note
    El agente contacta al servidor, nunca al revés: no se necesita
    abrir ningún puerto en la máquina monitorizada.

=== "Linux (systemd o OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    El script es `sh` POSIX (BusyBox y dash funcionan), descarga el
    binario para la arquitectura de la máquina (x86_64 o aarch64) desde
    el servidor, escribe `/etc/dumbmonit/agent.yaml` (modo 0600) y
    registra `dumbmonit-agent` con systemd, o con OpenRC en Alpine. La
    unidad systemd está reforzada (`ProtectSystem=strict`,
    `NoNewPrivileges`, `MemoryMax=128M`, `CPUQuota=20%`).

=== "macOS (launchd)"

    El mismo comando que en Linux, ejecutado con `sudo`:

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Los binarios de macOS **no** están en la imagen de DumbMonit:
    compilarlos requiere el SDK de Apple, cuya licencia prohíbe
    redistribuirlo. En su lugar se adjuntan a cada [versión de
    GitHub](https://github.com/laupernoe/dumbmonit/releases/latest) —
    `dumbmonit-agent-macos-aarch64` (Apple silicon) y
    `dumbmonit-agent-macos-x86_64` (Intel), cada uno con su `.sha256`
    — y el servidor redirige `/download/dumbmonit-agent-macos-…` ahí,
    así que el script elige el de la arquitectura del Mac (`uname
    -m`). El servidor no distribuye esos binarios, así que el comando
    de instalación no lleva ningún checksum para ellos: añade
    `--sha256=macos-aarch64:<hex>` (o `macos-x86_64`) con el valor del
    `.sha256` de la versión, o el instalador rechaza la descarga. El
    Mac también necesita alcanzar github.com.

    Sin ese acceso, o para instalar un binario que ya tienes,
    descárgalo y pásalo con `--bin` (compáralo primero con el
    `.sha256` de la versión: `--bin` instala lo que le das):

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    El script instala el binario en `/usr/local/bin` (y elimina el
    atributo de cuarentena que lleva una descarga de navegador, que
    haría que Gatekeeper se negara a iniciarlo), escribe
    `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) y registra el
    demonio del sistema `com.dumbmonit.agent` en
    `/Library/LaunchDaemons`, iniciado con `launchctl bootstrap
    system`. Se ejecuta al arrancar, sin nadie con sesión iniciada.

    !!! note "¿Prefieres compilarlo tú mismo?"
        En el propio Mac, con Rust instalado:
        `cargo build --release -p dumbmonit-agent`, y luego
        `--bin=target/release/dumbmonit-agent`. Es el mismo binario
        que produce el trabajo de versión.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Solo x86_64. El script descarga `dumbmonit-agent-freebsd-x86_64`
    desde el servidor, escribe
    `/usr/local/etc/dumbmonit/agent.yaml` (modo 0600) e instala
    `/usr/local/etc/rc.d/dumbmonit_agent`, activado con `service
    dumbmonit_agent enable`. `daemon(8)` supervisa el agente, lo
    reinicia si se detiene y escribe `/var/log/dumbmonit-agent.log`.

    Funciona igual en TrueNAS CORE y otros electrodomésticos basados
    en FreeBSD, donde las series ZFS son el punto del ejercicio.

=== "Windows (servicio)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    El script descarga `dumbmonit-agent-windows-x86_64.exe` desde el
    servidor, lo comprueba contra su SHA-256, lo instala como
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, escribe
    `C:\ProgramData\DumbMonit\agent.yaml` (legible solo por SYSTEM y
    Administrators) y registra el servicio de Windows
    `DumbMonitAgent` («DumbMonit system agent»): iniciado
    automáticamente al arrancar, como LocalSystem, y reiniciado tras
    un fallo (10 s, 30 s, luego 60 s). El agente habla con el propio
    gestor de servicios (`--service`), así que detener el servicio le
    permite enviar lo que todavía conserva antes de salir.

    Un servicio no tiene consola: el agente escribe su registro en
    `C:\ProgramData\DumbMonit\agent.log`, archivado como
    `agent.log.old` más allá de 10 MB.

    No hay compilación ARM64 nativa: en Windows 11 para ARM, el
    script instala el binario x86_64, que se ejecuta bajo emulación
    x64.

Un token puede inscribir varias máquinas: nómbralo según un grupo o
una máquina.

### Binarios nativos {#native-binaries}

El agente es un único ejecutable estático por sistema, sin nada que
instalar a su lado:

| Sistema | Archivo | Dónde obtenerlo |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | el servidor, `/download/…` |
| Linux aarch64 (ARM de 64 bits: Raspberry Pi OS 64 bits, servidores ARM) | `dumbmonit-agent-linux-aarch64` | el servidor, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | el servidor, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | el servidor, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | la versión de GitHub (el servidor redirige ahí) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | la versión de GitHub (el servidor redirige ahí) |

Los binarios de Linux están enlazados estáticamente contra musl: se
ejecutan en cualquier distribución, glibc o no (Alpine, firmwares de
NAS), sin ninguna biblioteca que hacer coincidir. Cada archivo tiene
su checksum al lado, `<archivo>.sha256`, en formato `sha256sum`. Cada
[versión de GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
lleva los seis binarios y sus checksums, para máquinas que no pueden
alcanzar el `/download/` del servidor o para instalar con `--bin`.

Desde 0.1.0-alpha.9, cada binario de una versión también está firmado sin
claves (keyless) con [Sigstore](https://www.sigstore.dev/)
(`<archivo>.sigstore.json`) y lleva una atestación de procedencia SLSA. Para
comprobar que un binario fue compilado por el flujo de trabajo de publicación de
este repositorio:

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'
# or, with the GitHub CLI:
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Para compilar tú mismo los binarios de Linux, Windows y FreeBSD, desde
un clon del repositorio (Docker es el único requisito):

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Gestionar el servicio {#managing-the-service}

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Estado | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Registros | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Reiniciar | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Detener | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Configuración | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| Archivo de servicio | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | servicio `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` en Windows) lo elimina todo, servicio
incluido, en cada sistema.

!!! note "La descarga se verifica — siempre"
    El comando de instalación que muestra DumbMonit lleva el SHA-256
    esperado de cada binario de agente que distribuye el servidor
    (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` en Windows). Ambos instaladores
    comprueban el archivo descargado contra él antes de instalar nada,
    y **se niegan** a instalar cuando no coincide, cuando no se da
    ningún checksum para la plataforma, o cuando no hay disponible
    ninguna herramienta SHA-256 (`sha256sum`, `shasum`, `sha256`).

    Los checksums cambian con cada versión del servidor: tras
    actualizar DumbMonit, copia un comando de instalación nuevo desde
    **Settings → Agents** (o desde un token nuevo) en lugar de volver
    a ejecutar uno antiguo, que se detendría con «checksum mismatch».
    Los mismos checksums se muestran bajo el comando de instalación, y
    se publican en `/download/<archivo>.sha256`.

    Para configuraciones aisladas o manuales,
    `--insecure-skip-checksum` (`-InsecureSkipChecksum`) instala un
    binario descargado sin verificarlo y lo dice en voz alta;
    `--bin=RUTA` instala un archivo local tal cual, siendo el archivo
    tuyo de verificar.

### Flags del instalador {#installer-flags}

| `install.sh` | `install.ps1` | Efecto |
|---|---|---|
| `--token=TOKEN` | `-Token` | Token de inscripción (obligatorio). |
| `--url=URL` | `-Url` | URL del servidor, por ejemplo `http://server:8080`. En Linux, se usa `DUMBMONIT_URL` del entorno si falta el flag. |
| `--interval=N` | `-Interval` | Período de muestreo en segundos (por defecto 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Servicios cuyo estado se reporta, nombrados como los nombra el sistema anfitrión: unidades systemd en Linux, etiquetas launchd (`com.apple.sshd`) en macOS, nombres rc.d en FreeBSD, servicios de Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Etiquetas, copiadas como `tag_<key>` en cada serie. |
| `--hostname=NAME` | `-HostName` | Nombre anunciado al servidor (por defecto: el de la máquina). |
| `--bin=PATH` | `-BinPath` | Binario local a instalar en lugar de descargarlo. |
| `--sha256=LIST` | `-Sha256` | SHA-256 esperado de la descarga, como pares `plataforma-arch:hex` separados por comas (un valor hex sin más también funciona). Incluido en el comando que muestra DumbMonit; obligatorio a menos que se dé el siguiente flag. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Instala un binario descargado sin verificarlo, con un aviso. Solo para configuraciones aisladas o manuales. |
| `--no-start` | | Instala todo, pero no contacta al servidor ni inicia el servicio (imagen de máquina, pruebas). |
| `--uninstall` | `-Uninstall` | Desinstala el agente y elimina su configuración. |
| `--help` | | Muestra la ayuda. |

### Actualizar {#upgrade}

Ambos scripts son idempotentes: ejecutar de nuevo el comando de
instalación, con el mismo token y URL, sustituye el binario, reescribe
la configuración y reinicia el servicio. Ese es el procedimiento de
actualización. Toma el comando del servidor actual (Settings →
Agents): su `--sha256` debe coincidir con los binarios de la versión
del servidor a la que actualizaste.

El mismo comando migra una máquina que todavía ejecuta el agente
EzyMonit, en Linux y en Windows. El instalador detiene y elimina el
servicio antiguo (`ezymonit-agent`, o `EzyMonitAgent` en Windows),
mueve la configuración a la nueva ruta
(`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`) e
instala `dumbmonit-agent` en su lugar. El antiguo token `ezym_…` sigue
siendo válido, así que el comando mostrado al crear el token sigue
funcionando; no hay que revocar nada primero.

## Archivo de configuración {#configuration-file}

`/etc/dumbmonit/agent.yaml` en Linux,
`/usr/local/etc/dumbmonit/agent.yaml` en macOS y FreeBSD,
`C:\ProgramData\DumbMonit\agent.yaml` en Windows:

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # período de muestreo
hostname: nas-basement     # opcional: nombre anunciado al servidor
services:                  # unidades systemd o servicios de Windows
  - sshd
  - docker
tags:                      # etiquetas libres, con prefijo "tag_" en el servidor
  role: nas
  room: basement
docker: true               # inventario de contenedores (por defecto: true)
docker_socket: /var/run/docker.sock
sensors: true              # temperaturas y ventiladores (por defecto: true)
smart: true                # salud de disco a través de smartctl (por defecto: true)
smart_bin: smartctl        # ruta, cuando no está en el PATH del servicio
smart_interval_secs: 300   # mínimo 60: un disco dormido nunca se despierta para leerse
zfs: true                  # pools ZFS a través de zpool (por defecto: true)
zfs_bin: zpool
zfs_interval_secs: 120     # mínimo 30
libvirt: true              # dominios KVM y pools de almacenamiento a través de virsh (por defecto: true)
libvirt_bin: virsh
libvirt_uri: qemu:///system
libvirt_interval_secs: 60  # mínimo 15
wireguard: true            # túneles WireGuard a través de wg (por defecto: true), ver abajo
restic_repos: []           # repositorios restic a vigilar, ver "copias de seguridad restic y Borg"
borg_repos: []             # repositorios Borg a vigilar
mdaemon: auto              # Windows: colas MDaemon cuando existe su servicio (true / false / auto)
hyperv: auto               # Windows: contadores Hyper-V cuando existe el servicio vmms (true / false / auto)
perf_counters:             # Windows: contadores de rendimiento a publicar
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # valor de la etiqueta "counter"; la ruta cuando se omite
max_buffered_samples: 20000
log_level: info
```

`sensors`, `smart`, `zfs` y `libvirt` se detectan, nunca se asumen: dejados en
`true` en una máquina sin sonda, sin `smartctl`, sin pool o sin `virsh`, cuestan
una mirada por período y no emiten nada en absoluto. Define uno como
`false` para dejar de mirar incluso así — vale la pena hacerlo en un
NAS cuyos discos prefieras que `smartctl` nunca toque, aunque el
agente ya pasa `-n standby` para que un disco dormido se mida solo
cuando está despierto por otras razones.

### Máquinas virtuales KVM con libvirt {#kvm-virtual-machines-through-libvirt}

En un hipervisor KVM (un host Linux normal con libvirt, no Proxmox VE, que
tiene su [propia integración](proxmox.md)), instala el agente en el propio
host, no en un invitado. El agente ejecuta `virsh -c qemu:///system` con
`list --all --name`, `dominfo --domain <nombre>`, `pool-list --all --name` y
`pool-info --pool <nombre>`: nunca define, arranca, detiene ni migra un
dominio, y nunca modifica un pool de almacenamiento. La URI explícita
`qemu:///system` importa para un agente ejecutado por un miembro del grupo
`libvirt` en lugar de root: sin ella, `virsh` leería la sesión privada y vacía
de ese usuario. Define `libvirt_uri` para leer otra conexión.

`agent_libvirt_domain_memory_used_bytes` está ausente para un dominio apagado:
libvirt solo conoce su máximo configurado. Los tamaños de los pools de
almacenamiento provienen de la salida legible de `virsh`, redondeada a dos
decimales, así que `agent_libvirt_pool_used_percent` es una aproximación,
suficientemente cercana para avisar antes de que un pool se llene. Se detallan
como máximo 200 dominios y 64 pools.

Junto a él, `/etc/dumbmonit/agent-secret`
(`C:\ProgramData\DumbMonit\agent-secret` en Windows) contiene el
secreto de vinculación que el servidor dio a esta máquina. El agente
lo escribe con modo `0600` y lo vuelve a leer en cada arranque; no es
parte de `agent.yaml` porque no es algo que tú defines — consulta
[Vinculación](#binding-one-machine-one-agent).
`DUMBMONIT_AGENT_SECRET_FILE` lo mueve, que es lo que quieres para un
agente en un contenedor.

Cada clave se puede anular con el entorno, lo cual hace al agente
usable en un contenedor sin montar un archivo:

| Variable | Efecto |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Ruta del archivo de configuración |
| `DUMBMONIT_AGENT_URL` | URL del servidor |
| `DUMBMONIT_AGENT_TOKEN` | Token de inscripción |
| `DUMBMONIT_AGENT_SECRET_FILE` | Dónde guardar el secreto de vinculación (por defecto: `agent-secret`, junto al archivo de configuración) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Período de muestreo |
| `DUMBMONIT_AGENT_HOSTNAME` | Nombre anunciado al servidor |
| `DUMBMONIT_AGENT_SERVICES` | Servicios a vigilar, separados por comas |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, separados por comas |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Ruta del socket de Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Contenedores detallados por host, por defecto `200` (`0`: solo recuentos) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfaces excluidas, nombres o regex separados por comas; por defecto `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfaces a conservar; si se define, sustituye la lista de exclusión |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Puntos de montaje excluidos de sistemas de archivos y E/S de disco; por defecto omite `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` para también enviar una serie de CPU por núcleo (por defecto `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`: temperaturas y ventiladores (por defecto `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`: salud de disco a través de `smartctl` (por defecto `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Ruta de `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Período entre dos lecturas SMART, por defecto `300`, mínimo `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`: pools ZFS a través de `zpool` (por defecto `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Ruta de `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Período entre dos lecturas ZFS, por defecto `120`, mínimo `30` |
| `DUMBMONIT_AGENT_LIBVIRT` | `true` / `false`: dominios KVM y pools de almacenamiento a través de `virsh` (por defecto `true`) |
| `DUMBMONIT_AGENT_LIBVIRT_BIN` | Ruta de `virsh` |
| `DUMBMONIT_AGENT_LIBVIRT_URI` | URI de conexión de libvirt, por defecto `qemu:///system` |
| `DUMBMONIT_AGENT_LIBVIRT_INTERVAL_SECS` | Período entre dos lecturas de libvirt, por defecto `60`, mínimo `15` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`: túneles WireGuard a través de `wg` (por defecto `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Ruta de `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Ruta de `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Ruta de `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Período entre dos lecturas de los repositorios restic y Borg, por defecto `600`, mínimo `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` o `auto`: los contadores de rendimiento de MDaemon (por defecto `auto`, leído cuando existe el servicio Windows `MDaemon`) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` o `auto`: los contadores de rendimiento de Hyper-V (por defecto `auto`, leído cuando existe el servicio Windows `vmms`) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Rutas de contador de rendimiento de Windows, separadas por **punto y coma** (una coma puede formar parte de un nombre de instancia); sustituye `perf_counters` |
| `DUMBMONIT_AGENT_SYSTEM_HEALTH` | `true` / `false`: salud del sistema operativo, el interruptor general de los cuatro siguientes (por defecto `true`) |
| `DUMBMONIT_AGENT_CHECK_UPDATES` | `true` / `false`: actualizaciones pendientes (por defecto `true`) |
| `DUMBMONIT_AGENT_UPDATES_INTERVAL_SECS` | Periodo entre dos comprobaciones de actualizaciones pendientes, por defecto `3600`, mínimo `300` |
| `DUMBMONIT_AGENT_CHECK_REBOOT` | `true` / `false`: reinicio necesario (por defecto `true`) |
| `DUMBMONIT_AGENT_CHECK_FAILED_UNITS` | `true` / `false`: unidades systemd fallidas (por defecto `true`) |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Tamaño del búfer de recuperación |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` para ejecutar, por el servidor, las sondas de los dispositivos asignados a este agente (consulta [Ejecutar el agente en Docker / en otra red](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Etiqueta de sitio mostrada junto al agente cuando se ofrece como relay |

## Contadores de rendimiento de Windows {#windows-performance-counters}

En Windows, el agente lee los contadores de rendimiento — los valores
que muestra Performance Monitor (`perfmon`) — a través de la
biblioteca PDH, y los publica en cada período de muestreo. Algún
software no publica su estado en ningún otro sitio.

**MDaemon.** Cuando existe el servicio de Windows llamado `MDaemon`,
el agente lee el objeto de rendimiento de MDaemon: tamaños de cola,
sesiones, totales de mensajes y spam, el estado de los servidores
internos de MDaemon y su tiempo de actividad. Las series, las reglas y
la página que las muestra se describen en la página de
[MDaemon](mdaemon.md#mail-queues-through-the-agent). `mdaemon: true`
las lee incluso sin el servicio (un servicio renombrado), `mdaemon:
false` nunca lo hace. La presencia del servicio se vuelve a comprobar
cada diez períodos, así que un MDaemon instalado después se recoge sin
reiniciar el agente.

**Hyper-V.** Cuando existe el servicio Hyper-V Virtual Machine
Management (`vmms`), el agente lee los objetos de rendimiento de
Hyper-V; consulta [Hosts Hyper-V](#hyper-v-hosts). `hyperv: true` y
`hyperv: false` fuerzan la elección, como para MDaemon.

**Cualquier otro contador.** Lista su ruta completa bajo
`perf_counters`, como `\Objeto(Instancia)\Contador` o
`\Objeto\Contador` para un objeto con una sola instancia, y se publica
como `dumbmonit_agent_perf_counter{counter="…"}`, con el nombre que
den o la propia ruta:

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Escribe los nombres de contador **en inglés**, tal como aparecen en
  Performance Monitor en un Windows en inglés: el agente los añade
  con `PdhAddEnglishCounterW`, así que el mismo `agent.yaml` funciona
  en un Windows en francés o alemán.
- Pon cada ruta entre comillas simples en YAML, para que las barras
  invertidas y el `%` se conserven tal cual.
- Una ruta mal formada detiene el agente al arrancar con la clave y el
  motivo. Una ruta que Windows no conoce (el software no está
  instalado, el nombre está mal escrito) se reporta una vez en el
  registro, `performance counter not available`, y los demás
  contadores se siguen leyendo; se vuelve a intentar cada diez
  períodos.
- Las tasas (`…/sec`) necesitan dos lecturas: aparecen a partir del
  segundo período.
- Como máximo se leen 200 contadores. En Linux, macOS y FreeBSD el
  ajuste se acepta y no se lee nada.

Para ver los nombres exactos en la máquina, ejecuta `typeperf -q
MDaemon` (o cualquier otro nombre de objeto) en una línea de comandos.

## Hosts Hyper-V {#hyper-v-hosts}

Instalado en un host Hyper-V — Windows Server con el rol Hyper-V, o
Windows con Hyper-V activado — el agente encuentra el servicio `vmms`
y lee los propios contadores de rendimiento de Hyper-V, sin nada que
configurar. La página de dispositivo del agente entonces muestra un
panel Hyper-V: cuántas máquinas virtuales reportan sanas, la carga
real de CPU del host, la memoria que retiene cada VM en ejecución, y
el caudal, latencia y errores de cada disco virtual.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Los nombres de contador vienen de la documentación de rendimiento
    de Hyper-V de Microsoft y se comprobaron contra la lógica con una
    fuente de contador simulada, no en un host Hyper-V en ejecución.
    Un contador que Windows no conoce se reporta una vez en el
    registro del agente y los demás se siguen leyendo. Dinos qué
    falla.

| Contador | Serie | Etiquetas |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`: máquinas virtuales en cada estado | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`: la carga del host, invitados incluidos | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`: la proporción dedicada a ejecutar invitados | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`: VMs en ejecución más el propio host | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`: demanda de memoria sobre memoria asignada; por encima de 100 la VM quiere más de lo que tiene | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (promedio), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, un contador | `disk` |

En un host Hyper-V, `\Processor(_Total)\% Processor Time` solo mide la
propia partición del host, no lo que consumen las máquinas virtuales:
el contador de procesador lógico del hipervisor es el que dice si la
máquina está saturada. `vm` es el nombre de la máquina virtual tal
como lo muestra Hyper-V Manager; `disk` es el nombre de instancia que
da Windows a cada disco virtual, derivado de su ruta de archivo. Las
instancias `_Total` se excluyen, y como máximo se leen 200 VMs o
discos por contador.

Se aplican tres [reglas integradas](../alerting/rules.md#hyper-v):

- **Hyper-V VM health critical**: Hyper-V reporta una VM en salud
  crítica durante cinco minutos (Warning) — con más frecuencia un
  disco que se quedó sin espacio.
- **Hyper-V host CPU high**: los procesadores lógicos más del 90 %
  ocupados durante quince minutos (Advisory).
- **Hyper-V virtual disk errors**: un disco virtual reportó errores de
  E/S en la última hora (Advisory).

## Ejecutar el agente en Docker / en otra red {#run-the-agent-in-docker--on-another-network}

El agente también se distribuye como una imagen de contenedor, con
las mismas etiquetas que el servidor:
`ghcr.io/laupernoe/dumbmonit-agent:latest` (última versión), `:edge`
(último commit en `main`). Es una imagen `FROM scratch` que contiene
el binario del agente y el paquete de CA de Mozilla, configurada
enteramente por variables de entorno. El repositorio incluye un
`docker-compose.agent.yml` listo para usar:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

Qué ve el contenedor, y qué montar:

| Necesidad | Qué hacer | Por qué |
|---|---|---|
| CPU, memoria, carga, E/S de disco del host | Nada | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` y `/proc/diskstats` son de todo el host dentro de cualquier contenedor. |
| Nombre de host | `DUMBMONIT_AGENT_HOSTNAME` | El hostname de un contenedor es un id aleatorio; el dispositivo se nombraría según él. |
| Contenedores del host | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Opcional. Listas e inspecciones de solo lectura; las acciones Restart/Update lo necesitan en escritura y `DUMBMONIT_AGENT_COMMANDS=true`. Sin el socket, el agente simplemente no reporta ningún contenedor. |
| Uso del sistema de archivos | `- /:/host:ro` (o los montajes que te importen) | El agente reporta los montajes visibles *dentro* del contenedor, bajo su ruta de contenedor. |
| Contadores de red del host | `network_mode: host` | Las interfaces están en namespace: sin esto el agente ve un `veth`. |
| Monitores de ping relayados a través de este agente | `sysctls: net.ipv4.ping_group_range: "0 2147483647"` | ICMP usa un socket de eco, permitido por este sysctl: sin capability. Ver [Ping ICMP sin capability](../install/docker.md#icmp-ping-without-a-capability). |
| CA privada | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Servidor detrás de un proxy inverso interno, dispositivos con certificados autofirmados. |
| Mantener la vinculación entre recreaciones | `- agent-state:/var/lib/dumbmonit-agent` y `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | El secreto de vinculación vive en el sistema de archivos del contenedor, que un `docker compose up --force-recreate` descarta. Sin el volumen, el agente vuelve sin vincular y alguien tiene que permitir la reinscripción desde la página de dispositivo cada vez. |

`pid: host` y `privileged: true` nunca son necesarios: el agente no
recolecta ninguna métrica por proceso.

### Modo relay: monitorizar otro sitio {#relay-mode-monitor-another-site}

Un agente puede hacer más que vigilar su propia máquina. Con `relay:
true` (o `DUMBMONIT_AGENT_RELAY=true`), el servidor **delega
sondas** en él: los dispositivos cuyo ajuste *Reached through* nombra
a este agente no son sondeados por el servidor sino por el agente,
desde su propia red, con exactamente los mismos recolectores — SNMP,
Proxmox VE, Proxmox Backup Server, Synology, HTTP, TCP, DNS, ping,
TLS. Los resultados caen bajo el dispositivo como si el servidor lo
hubiera sondeado: mismas métricas, mismo *last probe*, mismas
alertas. Una oficina remota, un segundo homelab o la red de un
cliente así aparecen en el único DumbMonit que ya tienes, sin VPN y
sin nada que abrir del lado remoto.

Cómo configurarlo, paso a paso, está en [Monitorizar un sitio
remoto](../install/remote-site.md).

## Vinculación: una máquina, un agente {#binding-one-machine-one-agent}

Un token de inscripción dice *esta máquina puede hablar*. No dice
*cuál* máquina está hablando — para eso está la clave de identidad
(`/etc/machine-id`, o el nombre de host), y esa clave no es ningún
secreto: se puede leer en cualquier máquina en segundos.

Así que en el primer lote el servidor entrega a cada máquina un
**secreto de vinculación** propio. Solo conserva una huella; el
agente escribe el secreto en `/etc/dumbmonit/agent-secret` con modo
`0600` y lo presenta en la cabecera `X-DumbMonit-Agent-Secret` en cada
petición a partir de entonces — mediciones, comandos de contenedor y
sondas relayadas por igual. Desde entonces:

- ninguna otra máquina puede enviar mediciones bajo este dispositivo,
  ni reescribir su nombre de host y sistema operativo;
- ninguna otra máquina puede obtener — y por tanto consumir — los
  comandos Restart y Update encolados para ella.

La página del dispositivo muestra dónde está una máquina, bajo
**Agent → Binding**:

| Estado | Qué significa | Qué hacer |
|---|---|---|
| **Bound** | El agente tiene un secreto propio. | Nada. |
| **Not bound — re-enrol this host** | Un host inscrito antes de que existiera la vinculación, y que nunca se vinculó desde entonces. El servidor rechaza sus mediciones, comandos de contenedor y sondas relayadas. | Haz clic en **Allow re-enrolment** en la página del dispositivo, y luego reinicia el agente. Si el agente es anterior a la vinculación, vuelve a ejecutar primero el comando de instalación en el host. |

### Hosts anteriores a la vinculación {#hosts-that-predate-binding}

!!! warning "Cambiado: los hosts sin vincular se rechazan"
    Las versiones anteriores seguían aceptando hosts inscritos antes
    de que existiera la vinculación, siempre que usaran el token que
    los había inscrito, y los vinculaban en silencio una vez
    actualizado su agente. Esa ventana de transición está cerrada: un
    host que nunca se vinculó ahora se **rechaza en cada ruta de
    agente** — mediciones, comandos de contenedor y sondas relayadas —
    hasta que un administrador lo vuelve a inscribir. Un token de
    flota lo comparten muchas máquinas y la clave de identidad se
    puede adivinar, así que nada más prueba que el agente que pregunta
    es el propio del host.

    Un binario de agente demasiado antiguo para recibir un secreto de
    vinculación también se rechaza, incluso en su primera
    inscripción.

Para traer de vuelta tal host:

1. Si su agente es anterior a la vinculación, vuelve a ejecutar el
   comando de instalación en él (el binario del agente debe poder
   guardar un secreto de vinculación).
2. Abre su página de dispositivo y haz clic en **Agent → Allow
   re-enrolment**. Esto abre una ventana de una hora.
3. Reinicia el agente, o espera a su siguiente reintento: ese lote
   vincula el host con un secreto nuevo y cierra la ventana. Un token
   distinto del que inscribió el host todavía debe poder inscribir
   (no agotado, no caducado).

Hasta entonces, el registro del agente lleva la explicación del
servidor:

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Reinscripción tras una reinstalación {#re-enrolment-after-a-reinstall}

Si la máquina se reconstruye, su disco se reemplaza, o su contenedor
de agente se recrea sin el volumen que guarda el secreto, el agente
vuelve sin él y el servidor rechaza sus lotes. Ese rechazo es
deliberado: una máquina que está vinculada permanece vinculada hasta
que un humano dice lo contrario, de lo contrario la vinculación no
protegería nada.

La vía de vuelta está en la página del dispositivo: **Agent → Allow
re-enrolment**. Abre una ventana de una hora durante la cual un agente
que presenta un token de inscripción válido vincula esa máquina de
nuevo, con un secreto nuevo; el secreto antiguo deja de funcionar, y
la ventana se cierra en cuanto se usa. El agente reintenta por su
cuenta mientras tanto, así que no hay nada que reiniciar en la
máquina — y nada se pierde, ya que sus lecturas se mantienen en búfer
hasta que se vuelve a aceptar el enlace.

Hasta que alguien abre esa ventana, el registro del agente dice qué
hacer:

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Tokens de inscripción {#enrolment-tokens}

- Los tokens se crean en **Settings → Agents** o al añadir un
  dispositivo de tipo agent. El servidor solo conserva una huella: el
  token en claro se muestra una vez, en la respuesta de creación, con
  los dos comandos de instalación.
- El agente lo envía como `Authorization: Bearer dmon_…` en `POST
  /api/ingest`. Nunca aparece en los registros del agente, ni siquiera
  truncado.
- **Settings → Agents** lista los tokens con su prefijo, alcance,
  fecha de creación y último uso. **Revoke** detiene cada agente que
  usa ese token en su próximo envío; las máquinas entonces se vuelven
  *unreachable* tras tres períodos perdidos. Vuelve a ejecutar el
  instalador con un token nuevo para volver a inscribirlas.
- Las máquinas son dispositivos separados: revocar un token no las
  elimina.

### Un solo uso o flota {#single-use-or-fleet}

El alcance de un token se elige al crearlo, y es una elección, no un
valor por defecto que te ocurre:

| Alcance | Qué inscribe | Cuándo |
|---|---|---|
| **Single use** (por defecto) | Exactamente una máquina. | Una instalación. Un comando de instalación persiste en el historial de shell, en un ticket, en un registro de chat; uno de un solo uso no vale nada una vez usado. |
| **Reusable for a fleet** | Tantas máquinas como permitas — un número, o sin límite. | Un playbook, una imagen de máquina, un lote de instalaciones. |

Ambos pueden llevar un plazo en días. Un plazo solo detiene las
*inscripciones*: las máquinas ya inscritas siguen reportando después
de que pase, que es lo que quieres para una ventana de instalación.
Revocar es lo que corta a una flota.

Un token que ya no puede inscribir — agotado, o pasado su plazo — se
muestra como **Enrols no more** en Settings → Agents. Sus máquinas
siguen reportando; simplemente no puede dejar entrar a una nueva.

## Caídas y diagnóstico {#outages-and-diagnostics}

Cuando el servidor es inaccesible, el agente sigue midiendo y
mantiene sus lecturas en memoria con sus timestamps originales. Se
envían en cuanto el enlace vuelve: un reinicio del servidor no deja
ningún hueco en las gráficas. El búfer está limitado
(`max_buffered_samples`, aproximadamente una hora por defecto) y
descarta las lecturas más antiguas cuando está lleno. Los reintentos
se retrasan de 5 segundos a 5 minutos con jitter.

```sh
dumbmonit-agent --dry-run          # imprime las lecturas, no envía nada
dumbmonit-agent --once             # envía un lote y sale
journalctl -u dumbmonit-agent -f   # registro del servicio (systemd)
tail -f /var/log/dumbmonit-agent.log   # registro del servicio (OpenRC)
```

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| El instalador dice «could not reach the server» | URL o token incorrectos. La configuración se escribe: corrige `agent.yaml`, y luego reinicia el servicio. |
| La descarga falla con 404 | El servidor no tiene binarios de agente (`DUMBMONIT_AGENT_DIR` vacío). Usa `--bin=RUTA` con un binario que hayas compilado. |
| El instalador se detiene con «checksum mismatch» | El comando se copió antes de una actualización del servidor (copia uno nuevo desde Settings → Agents), o el binario recibido no es el que sirve el servidor: un proxy o una caché en el camino, o una descarga manipulada. |
| El instalador se detiene con «no expected checksum» | El comando no tiene `--sha256` para esta plataforma: un comando antiguo, o un binario de macOS (no distribuido por el servidor). Copia un comando nuevo, o pasa `--sha256=<plataforma>-<arch>:<hex>` del `.sha256` de la versión. |
| La máquina nunca aparece | El envío va a `/api/ingest` en la URL del comando de instalación; detrás de un proxy inverso, asegúrate de que se reenvía y que se permiten cuerpos de hasta 16 MB. |
| *Unreachable* aunque el agente se ejecute | El token se revocó, o los envíos se rechazan. Comprueba `journalctl -u dumbmonit-agent`: el error se registra ahí. |
| El registro dice «this agent is not recognised for this machine» | La máquina está vinculada a una instalación de agente cuyo secreto este agente no tiene — una reinstalación, o un contenedor recreado sin su volumen de estado. Abre la página del dispositivo y haz clic en **Allow re-enrolment**; consulta [Reinscripción tras una reinstalación](#re-enrolment-after-a-reinstall). |
| El registro dice «this machine was enrolled before agent binding» | Un host que nunca se vinculó a su agente. Haz clic en **Allow re-enrolment** en su página de dispositivo, y luego reinicia el agente; consulta [Hosts anteriores a la vinculación](#hosts-that-predate-binding). |
| El registro dice «this agent is too old to be bound to its machine» | El binario del agente no puede guardar un secreto de vinculación. Vuelve a ejecutar el comando de instalación en el host; si el dispositivo ya existe, haz clic primero en **Allow re-enrolment**. |
| El registro dice «this enrollment token has already enrolled all the machines it was allowed to» | Se está reutilizando un token de un solo uso. Crea uno nuevo, o un token reutilizable en Settings → Agents. |
| Windows | Las partes de servicio de Windows no se habían probado en la imagen de compilación del proyecto en el momento de escribir esto; reporta problemas en GitHub. |

## Contenedores Docker {#docker-containers}

Cuando el agente puede leer el socket de Docker (`docker: true`, por
defecto, y un usuario en el grupo `docker` o root —
`usermod -aG docker dumbmonit`, y luego reinicia el agente), cada
contenedor se convierte en un puñado de series, etiquetadas
`container` e `image` (el `name:tag` que muestra `docker ps`). Si la
página del dispositivo dice «No Docker on this machine» mientras
Docker está instalado, eso es lo primero a comprobar.

| Métrica | Significado |
|---|---|
| `container_up` | 1 en ejecución, 0 en caso contrario |
| `container_health` | 0 sin healthcheck, 1 sano, 2 no sano, 3 iniciando |
| `container_restart_count` | El contador de reinicios de Docker para el contenedor |
| `container_started_seconds` | Segundos desde que el contenedor se inició por última vez (0 cuando está detenido) |
| `container_image_age_seconds` | Segundos desde que se compiló la imagen |
| `container_update_available` | 1 cuando el registro tiene una imagen más reciente para la misma etiqueta, 0 cuando está al día, -1 desconocido |

La comprobación de actualización compara el digest de la imagen local
con el que anuncia el registro para la etiqueta (`HEAD
/v2/<name>/manifests/<tag>`, flujo de token anónimo en Docker Hub y
GHCR). Se ejecuta en segundo plano como máximo una vez por hora por
imagen y nunca retrasa el ciclo de 30 segundos; un registro privado
que rechaza el acceso anónimo simplemente lee como -1. Desactívala con
`docker_update_check: false` (`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Reglas integradas: **Container stopped** (`container_up == 0` durante
2 minutos), **Container unhealthy** (`container_health == 2` durante
3 minutos), **Container restarting** (tres o más reinicios en 15
minutos), **Container update available** (info, tras una hora,
repetido diariamente). La alerta nombra el contenedor.

### Acciones {#actions}

En la página del dispositivo, cada contenedor tiene dos acciones y dos
interruptores. Las acciones viajan por un **canal de comandos**: el
servidor encola el comando, el agente lo obtiene tras su siguiente
lote (`GET /api/agent/commands`, mismo token de inscripción), lo
ejecuta, y reporta el resultado (`queued → running → done | failed`),
con un registro breve que puedes desplegar en la interfaz. Los
comandos de más de diez minutos nunca se ejecutan; el agente ejecuta
un comando a la vez.

Un comando en cola se puede **cancelar** desde la interfaz mientras el
agente no lo haya recogido. Un comando que nadie viene a buscar
**caduca** tras diez minutos, en el servidor, tanto si el agente
sondea como si no — así que una máquina cuyo agente está detenido
nunca mantiene un «Queued» obsoleto para siempre, y nunca bloquea la
siguiente acción en ese contenedor.

El agente le dice al servidor, con cada lote, si acepta comandos.
Cuando no lo hace — un agente más antiguo que el canal de comandos, o
instalado con `commands: false` — la página del dispositivo lo indica
en lugar de mostrar Restart y Update, la API rechaza los comandos
(`409`), y las políticas no encolan nada. Reinstala el agente con el
instalador actual para activar las acciones.

- **Restart** — `docker restart` para un contenedor en ejecución,
  `docker start` para uno detenido, y luego una comprobación de 20
  segundos de que se ejecuta.
- **Update now** — obtiene la etiqueta que ya usa el contenedor, crea
  un contenedor nuevo con la misma configuración (entorno, montajes,
  puertos, etiquetas, redes, política de reinicio), detiene el
  antiguo, intercambia los nombres, inicia el nuevo y espera su
  healthcheck (o 20 segundos más una comprobación de ejecución cuando
  la imagen no tiene ninguno). Si algo falla, el agente **revierte**:
  el contenedor nuevo se elimina y el antiguo se reinicia, y el
  comando reporta `failed` con el motivo. En éxito el contenedor
  antiguo se elimina y, cuando «prune» está activado, la imagen
  antigua también — a menos que otro contenedor todavía la use.

Los contenedores gestionados por Compose (etiquetas
`com.docker.compose.*`) se actualizan igual, pero el resultado avisa
de que el siguiente `docker compose up` recreará el contenedor desde
el archivo compose. Los contenedores cuyo nombre empieza con
`dumbmonit` o `ezymonit` se rechazan: el monitor nunca se actualiza a
sí mismo.

### Políticas {#policies}

Una **política** por contenedor la evalúa el servidor cada minuto:

- **Restart if down** — un contenedor que reporta `container_up == 0`
  se reinicia automáticamente, como máximo una vez cada diez minutos.
- **Auto-update in maintenance windows** — un contenedor con una
  actualización disponible se actualiza automáticamente, pero solo
  mientras una ventana de mantenimiento (silencio) cubra el
  dispositivo, como máximo una vez cada seis horas. Programa una en la
  página Alerts, o silencia el dispositivo desde su página. Las
  políticas por defecto podan la imagen antigua y solo en ventanas de
  mantenimiento.

Dos lugares las definen:

- la **franja Docker** bajo la cabecera del dispositivo: «28
  containers · 27 running · 3 updates available · policies: 2
  auto-restart, 0 auto-update», la última acción y su estado,
  **Manage containers** (abre la lista de abajo) y **Policies…**, una
  tabla de cada contenedor con los dos interruptores y una fila
  «Apply to all» — la forma más rápida de activar el auto-restart para
  toda una máquina;
- los interruptores dentro de cada fila de la sección **Containers**,
  junto a los botones Restart y Update now.

Cada interruptor se guarda por su cuenta (`PUT
/api/targets/<id>/containers/<name>/policy`); un fallo solo afecta a
esa fila y se muestra junto a ella.

Las acciones automáticas aparecen en «Recent actions» como solicitadas
por `policy`.

### Reglas de seguridad {#safety-rules}

- El agente solo actúa cuando `commands: true` (por defecto); defínelo
  como `false` para una máquina que solo deba medirse.
- El agente obtiene los comandos él mismo, con su token de
  inscripción — nada se conecta a la máquina. Caducan tras diez
  minutos y se ejecutan de uno en uno; el agente nunca obtiene una
  etiqueta distinta de la que ya usa el contenedor.
- Una actualización que no vuelve sana se revierte al contenedor
  anterior, y la imagen antigua solo se elimina una vez que el
  contenedor nuevo se ejecuta.
- Los contenedores llamados `dumbmonit*` o `ezymonit*` se rechazan, a
  mano o por política: el monitor nunca actúa sobre sus propios
  contenedores. La comprobación se hace dos veces — el servidor se
  niega a encolar el comando, y el agente se niega a ejecutarlo — y el
  agente la vuelve a hacer sobre el nombre que reporta Docker tras
  inspeccionar el contenedor, así que nombrar uno por su id no cambia
  nada. El agente también se niega a actuar sobre su propio
  contenedor, se llame como se llame: no estaría ahí para terminar el
  trabajo o reportarlo.
- Los contenedores gestionados por Compose se actualizan, pero el
  siguiente `docker compose up` los recrea desde el archivo compose:
  sube también la etiqueta ahí.

## Copias de seguridad Plakar {#plakar-backups}

El agente puede vigilar klosets (repositorios) de
[Plakar](https://plakar.io): cada diez minutos, en segundo plano,
ejecuta `plakar at <kloset> ls` e `info` en cada uno y vuelve a emitir
la última lectura en cada ciclo.

Plakar se **detecta, nunca se asume**. Al arrancar y luego en cada
ciclo hasta que encuentra algo, el agente busca el binario `plakar`
(en `PATH`, o `plakar_bin`) y klosets en las ubicaciones estándar.
Cuando no hay ningún binario, ningún kloset ni ningún rastro de
Plakar (`~/.cache/plakar`), el agente no emite ninguna serie
`backup_*` en absoluto, lo indica una vez a nivel `debug`, y la página
del dispositivo no muestra ningún panel Backups — nada que configurar
en máquinas sin Plakar. Para omitir la detección por completo:

```yaml
plakar: false                        # o DUMBMONIT_AGENT_PLAKAR=false
```

### Descubrimiento {#discovery}

Nada que configurar: cuando `plakar_klosets` está vacío, el agente
encuentra los klosets él mismo, en este orden, y lee cada ubicación
solo una vez:

1. el propio directorio de configuración del agente —
   `$XDG_CONFIG_HOME/plakar`, o `~/.config/plakar` de la cuenta bajo
   la que se ejecuta (`plakar_home` cuando se define);
2. `/root/.config/plakar`;
3. `/home/*/.config/plakar`, alfabéticamente;
4. `/var/lib/plakar`, cuando ese directorio existe.

En cada directorio lee `stores.yml` (o `stores.yaml`; el antiguo
`plakar.yml` también se acepta) y toma cada entrada del mapa `stores`
(también `repositories` y `klosets`) que tiene una `location`. Solo se
lee la ubicación: las frases de paso y secretos de rclone que están
junto a ella nunca se conservan ni se registran. Cada entrada se
convierte en el kloset `@<name>`, leído con `plakar -configdir <dir>
at @<name>`, así que un kloset creado por un usuario se lee incluso
cuando el agente se ejecuta como root. `~/.plakar` de cada cuenta se
añade cuando existe (el kloset por defecto de `plakar create`). Dos
cuentas que nombran klosets distintos de la misma forma se distinguen
como `@name` y `<user>:@name`.

El resultado se registra una vez a nivel `info` (y de nuevo solo
cuando cambia). Dos gauges, emitidos solo una vez detectado Plakar,
permiten a la interfaz distinguir los demás casos:
`backup_plakar_present` (1 cuando el binario se ejecuta) y
`backup_klosets_found` (número de klosets leídos). La página del
dispositivo entonces dice «Plakar is installed but no kloset was
found», o lista los klosets. Cuando existen klosets pero el binario
no se ejecuta (un `PATH` de servicio sin `plakar`, típicamente), se
listan como illegibles con una pista para definir `plakar_bin`, y el
agente avisa una vez.

Para vigilar una lista fija en su lugar, nómbrala — el descubrimiento
entonces se omite:

```yaml
plakar: true                         # por defecto: detectar Plakar; false lo oculta por completo
plakar_bin: /usr/bin/plakar          # por defecto: "plakar" en PATH
plakar_klosets:                      # lista explícita: anula el descubrimiento
  - /srv/backups/main
  - "@diskext"                       # un store nombrado de la cuenta del agente
plakar_home: /home/noe               # opcional: HOME (y directorio de config) usado al ejecutar plakar
plakar_interval_secs: 600            # mínimo 60
```

| Variable | Efecto |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — vigilar copias Plakar cuando se detectan (por defecto `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Ruta del binario `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets, separados por comas (anula el descubrimiento) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Directorio home usado al ejecutar `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Segundos entre dos lecturas |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — comprobación de registro para actualizaciones de contenedor |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — aceptar acciones del servidor |

Un kloset cifrado necesita su frase de paso: ponla en la entrada del
store (`plakar store set <name> passphrase=…`, que es lo que hace
`plakar store add`) o exporta `PLAKAR_PASSPHRASE` en el entorno del
servicio del agente. Sin ella el kloset se lee como «Cannot be
opened» (`backup_last_status = 0`).

### Métricas y reglas {#metrics-and-rules}

Métricas, etiquetadas `kloset` y `source` (el directorio que se
respaldó): `backup_last_success_seconds` (antigüedad del snapshot más
reciente), `backup_snapshot_count`, `backup_last_status` (1 cuando el
snapshot más reciente no tiene errores, 0 cuando los tiene o el
kloset no se puede abrir), `backup_size_bytes{kloset}` (tamaño de
almacenamiento del repositorio), más `backup_plakar_present` y
`backup_klosets_found` descritos arriba. Ninguna de ellas existe
mientras no se detecta Plakar, así que las reglas integradas de abajo
no pueden activarse en una máquina sin Plakar.

Reglas integradas: **Plakar backup too old** (advisory, sin snapshot
durante más de dos días) y **Plakar backup failed** (warning). La
página del dispositivo muestra un bloque por kloset — antigüedad de
la última copia, recuento de snapshots, tamaño, último estado — y una
línea por fuente, con una pista «Backup too old» pasados dos días.

### Ejecutar una primera copia {#run-a-first-backup}

Con Plakar 1.1 instalado en la máquina (`plakar version`), como el
usuario que será propietario de las copias:

```sh
# 1. un store de kloset, nombrado para que el agente lo encuentre en ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # o cualquier ubicación que acepte Plakar
plakar at @diskext create                                   # añade -plaintext para omitir el cifrado

# 2. una primera copia, luego compruébala
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` y `plakar at <path> backup <dir>` funcionan
igual con un directorio plano en lugar de `@name`; `plakar create`
sin `at` usa `~/.plakar`, que el agente también encuentra. Programa
`plakar at @diskext backup /srv/photos` con cron o un timer systemd:
el agente reporta la antigüedad del snapshot más reciente, y alerta
cuando pasa un día sin uno — dos días para la regla integrada.

Para probarlo localmente con un kloset sin cifrar:

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

y luego añade `plakar_klosets: ["/tmp/plakar-demo"]` a `agent.yaml`
(una ruta plana fuera de un home no se descubre) y comprueba con
`dumbmonit-agent --dry-run` que aparecen muestras `backup_*`.

## Copias de seguridad restic y Borg {#restic-and-borg-backups}

El agente puede vigilar repositorios [restic](https://restic.net) y
[Borg](https://www.borgbackup.org): cada diez minutos, en segundo
plano, abre cada repositorio que declaras y reporta si se pudo leer y
cuán antiguo es su snapshot más reciente. Nada se descubre: un
repositorio no tiene ninguna ubicación estándar y no se puede abrir
sin su contraseña, así que cada uno se lista en `agent.yaml`, con la
forma de obtener su contraseña.

**La contraseña nunca sale de la máquina.** El agente la entrega a
`restic` o `borg` solo a través del entorno del proceso hijo; nunca se
registra, nunca se envía al servidor, y las series solo llevan el
nombre del repositorio.

```yaml
restic_repos:
  - name: nas                                   # la etiqueta "repo"; por defecto: el repositorio, sin credenciales
    repository: sftp:backup@nas.lan:/srv/restic  # pasado como RESTIC_REPOSITORY, nunca en la línea de comandos
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # opcional: solo los snapshots de este host (--host), para un repositorio compartido
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # o cualquier comando que la imprima (RESTIC_PASSWORD_COMMAND)
    env:                                        # entorno extra solo para este repositorio
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # pasado como BORG_REPO
    password_file: /etc/dumbmonit/borg-offsite.pass        # leído por el agente, pasado como BORG_PASSPHRASE
    glob_archives: "web-01-*"                   # opcional: solo estos archivos (--glob-archives)
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # por defecto: en PATH
borg_bin: borg
backup_repos_interval_secs: 600                 # mínimo 60
```

Mantén `agent.yaml` y los archivos de contraseña legibles solo por
root (`chmod 600`), como hace el instalador para `agent.yaml`.
`password_command` se mapea a `RESTIC_PASSWORD_COMMAND` o
`BORG_PASSCOMMAND`. Cualquier cosa que las herramientas lean de su
entorno puede ir bajo `env` (`RESTIC_PASSWORD`, `BORG_PASSPHRASE`,
claves cloud, `BORG_RSH`…), y el propio entorno de servicio del
agente también se pasa.

Lo que ejecuta el agente, de solo lectura en el sentido más estricto:

- `restic snapshots --json --latest 1 --no-lock --no-cache`: no se
  escribe ningún bloqueo, así que un repositorio de solo lectura o
  solo-anexar se puede vigilar, y un `prune` que mantiene el bloqueo
  exclusivo no hace que el repositorio parezca caído; tampoco caché,
  ya que el servicio instalado no puede escribir en `/root`.
- `borg --bypass-lock list --json --last 1`: sin `--bypass-lock`, la
  lectura fallaría durante toda la duración de cada `borg create`,
  que mantiene el bloqueo exclusivo. Borg escribe un pequeño registro
  de seguridad bajo `~/.config/borg`; cuando ese home no se puede
  escribir (el servicio instalado se ejecuta con `ProtectHome=yes`),
  el agente apunta `BORG_BASE_DIR` a su `/tmp` privado en su lugar, a
  menos que tú mismo definas `BORG_BASE_DIR`. Un repositorio en modo
  `keyfile` entonces necesita `BORG_KEY_FILE` en su `env`.

Como el servicio se ejecuta con `ProtectHome=yes`, mantén lo que
necesitan las herramientas fuera de los directorios home: archivos de
contraseña, claves SSH y `known_hosts` bajo `/etc/dumbmonit/`, como en
el ejemplo, y repositorios locales bajo `/srv`, `/mnt` o `/var` en
lugar de `/home`. Las lecturas se ejecutan una tras otra, dentro del
límite de memoria del servicio (128 MiB con el propio agente).

Métricas, etiquetadas `tool` (`restic` o `borg`) y `repo`:
`agent_backup_repo_reachable` (1 cuando la última lectura tuvo éxito,
0 en caso contrario: repositorio inaccesible, contraseña rechazada,
binario faltante — el agente registra el motivo una vez, y de nuevo
cuando cambia), `agent_backup_repo_last_snapshot_age_seconds`
(antigüedad del snapshot más reciente, ausente mientras el
repositorio está vacío), y `agent_backup_repo_last_snapshot_bytes`
(bytes que procesó el snapshot restic más reciente, restic 0.17 y
posteriores). Con `--latest 1`, restic devuelve el snapshot más
reciente de cada host y conjunto de rutas; se conserva el más
reciente de todos, así que define `host` en un repositorio compartido
por varias máquinas.

Reglas integradas: **restic/Borg backup too old** (sin snapshot
durante más de 48 horas) y **restic/Borg repository unreachable**
(durante 30 minutos). La página del dispositivo muestra un panel
Backup repositories, una línea por repositorio.

Para comprobar una declaración, ejecuta el agente una vez en primer
plano y busca las muestras `agent_backup_repo_*`:

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Túneles WireGuard {#wireguard-tunnels}

En una máquina que ejecuta WireGuard, el agente lee cada interfaz y
cada peer con `wg show all dump`, en cada ciclo. Nada que configurar:
sin el comando `wg` (wireguard-tools) o sin ninguna interfaz, no se
emite ninguna serie y no se muestra ningún panel. `wg` necesita root
(`CAP_NET_ADMIN`); el servicio instalado lo tiene, un agente iniciado
a mano con tu propia cuenta registra el rechazo una vez y no reporta
nada.

El dump también contiene la clave privada de la interfaz y la clave
precompartida de cada peer: el agente omite ambos campos sin copiarlos
nunca.

Por peer, etiquetado `interface`, `peer` (su clave pública) y
`allowed_ips`: `agent_wireguard_peer_handshake_age_seconds` (segundos
desde el último handshake), `agent_wireguard_peer_has_handshake` (0
mientras el peer nunca ha completado uno),
`agent_wireguard_peer_keepalive_seconds` (su keepalive persistente, 0
cuando está desactivado), y los contadores
`agent_wireguard_peer_rx_bytes` y `agent_wireguard_peer_tx_bytes`; por
interfaz, `agent_wireguard_interface_peers`. Para un peer que nunca ha
completado un handshake, la antigüedad se cuenta desde el momento en
que el agente lo vio por primera vez: un túnel que nunca se levantó
también está en silencio, y se reporta como tal.

WireGuard no tiene nombres de peer. Da los tuyos en `agent.yaml`, por
clave pública, y se convierten en la etiqueta `name`:

```yaml
wireguard: true                     # por defecto; false deja de mirar incluso así
wireguard_bin: wg                   # por defecto: en PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

La regla integrada **WireGuard peer silent** se activa cuando un peer
**con keepalive persistente** no ha completado un handshake durante
más de 15 minutos. Mientras un túnel está levantado, WireGuard renueva
su handshake cada dos minutos, y un keepalive lo mantiene levantado
incluso sin tráfico; un peer sin keepalive (un teléfono, un portátil)
puede estar inactivo durante días sin nada roto, así que se excluye.
Define `PersistentKeepalive = 25` en los túneles que deben permanecer
levantados (enlaces sitio a sitio, gateways remotos). N es el umbral
de la regla: cámbialo en **Alerts → Rules**, o elimina la parte `and
on (…)` de su consulta para vigilar cada peer. La página del
dispositivo muestra un panel WireGuard con cada peer, el tiempo desde
su último handshake, su tráfico y su keepalive.

## Certificados TLS (Certbot / Let's Encrypt) {#tls-certificates-certbot--lets-encrypt}

El agente no lee `/etc/letsencrypt` directamente. Cada certificado que
renueva Certbot se sirve en algún sitio — un servidor web, un
servidor de correo, un proxy inverso — y ahí es donde lo lee
DumbMonit: añade un monitor de [certificado
TLS](services.md#tls) (o uno de [sitio web o API
web](services.md#http), para un sitio ya vigilado de esa forma)
apuntado al puerto en el que realmente se presenta el certificado.
Lee el certificado en vivo, no un archivo en disco, así que también
detecta una renovación que tuvo éxito en `/etc/letsencrypt` pero que
el servicio nunca recogió (un proxy inverso que necesita una recarga
para ver el archivo nuevo es el caso habitual, y exactamente el hueco
que importa).

Leer `/etc/letsencrypt/live/*/cert.pem`, la configuración de
renovación y el registro de Certbot desde el agente se consideró y se
descartó: duplicaría la fecha de caducidad que ya reporta el monitor
TLS, mientras añade una segunda ruta, local al host, a la misma
información — una cosa más que mantener sincronizada tras mover un
certificado o renombrar un dominio. Un host donde se ejecuta Certbot
pero que no sirve nada que DumbMonit pueda alcanzar directamente (un
paso de CA interna, un hook de validación DNS) es el único caso que
esto deja sin cubrir; también es el caso en el que un monitor TLS
tampoco podría ayudar, ya que nada en ese host habla TLS para
sondearse.
