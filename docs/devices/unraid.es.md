# Unraid

Un servidor Unraid a través de su API GraphQL: un array que no está
iniciado, discos desactivados, ausentes o reportando errores de lectura,
comprobaciones de paridad que encontraron errores o no se han ejecutado
en semanas, pools de caché que se llenan, contenedores que deberían estar
en ejecución y no lo están, y el estado de las VMs.

DumbMonit envía una clave de API con el rol Viewer en la cabecera
`x-api-key` a `/graphql`. El rol Viewer lee todo y no puede cambiar nada.
La API viene integrada en Unraid desde la 7.2; en la 6.12 a 7.1 viene con
el plugin Unraid Connect.

Cada minuto DumbMonit envía una consulta obligatoria (versión, array,
paridad, discos de datos y caché) y cuatro opcionales, cada una por su
cuenta para que un campo que una versión más antigua de la API no tenga,
o las VMs apagadas, solo omitan esa parte: la comprobación de paridad, los
contenedores Docker, las VMs y las notificaciones no leídas.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de la documentación de la API
    de Unraid y el esquema GraphQL publicado de la API de Unraid
    (nombres de campo, enumeraciones, tamaños en kilobytes), no contra un
    servidor en ejecución. Cada consulta es de lectura. Dinos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_unraid_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `version_info` | `version` | Valor 1: la versión de Unraid. |
| `array_started` | | 1 cuando el array está iniciado. |
| `array_state_info` | `state` | Valor 1: `started`, `stopped`, `new_array`, `recon_disk`, `disable_disk`… |
| `array_size_bytes`, `array_used_bytes`, `array_free_bytes`, `array_used_percent` | | Capacidad del array. |
| `disk_ok` | `disk`, `role` (`parity`, `data`, `cache`) | 1 cuando el estado del disco es `DISK_OK`. Las ranuras vacías se omiten. |
| `disk_status_info` | `disk`, `role`, `status` | Valor 1: el estado de Unraid (`disk_ok`, `disk_dsbl`, `disk_np_missing`, `disk_invalid`…). |
| `disk_errors` | `disk`, `role` | Errores de lectura desde que arrancó el array (la columna Errors). |
| `disk_temperature_celsius` | `disk`, `role` | Solo mientras el disco gira: Unraid no reporta ninguna para un disco dormido. |
| `disk_spinning` | `disk`, `role` | 1 cuando el disco gira. |
| `disk_size_bytes`, `disk_fs_size_bytes`, `disk_used_percent` | `disk`, `role` | Tamaño, tamaño del sistema de archivos y llenado, para discos con un sistema de archivos. |
| `disks`, `disks_problem`, `disks_with_errors` | | Recuentos. |
| `parity_check_status_info` | `status` | Valor 1: `completed`, `running`, `paused`, `cancelled`, `failed`, `never_run`. |
| `parity_check_running`, `parity_check_progress_percent` | | Una comprobación en curso. |
| `parity_check_ok` | | Tras una comprobación terminada: 1 sin errores de sincronización, 0 con errores o si falló. Una comprobación cancelada no demuestra nada y no fija nada. |
| `parity_check_errors`, `parity_check_age_seconds`, `parity_check_duration_seconds` | | La última comprobación. |
| `containers`, `containers_running`, `containers_autostart_stopped` | | Contenedores Docker. |
| `container_running`, `container_autostart_stopped` | `container` | Por contenedor; el segundo es 1 cuando un contenedor configurado para iniciarse con el array no está en ejecución. |
| `vms`, `vms_running` | | VMs. |
| `vm_state_info`, `vm_crashed` | `vm` (y `state`) | Por VM. |
| `notifications_unread` | `importance`: `alert`, `warning`, `info` | Notificaciones no leídas. |

Las [reglas integradas](../alerting/rules.md#unraid) que se aplican:

- **Unraid array stopped**: el array no está iniciado durante diez
  minutos (Warning).
- **Unraid disk disabled or missing**: el estado de un disco no es
  `DISK_OK` durante cinco minutos (Warning). El array funciona
  degradado.
- **Unraid disk read errors**: un disco reporta errores de lectura
  (Advisory).
- **Unraid disk too hot**: un disco de datos o paridad por encima de
  55 °C durante quince minutos (Advisory). Los discos de caché, a menudo
  NVMe, se excluyen.
- **Unraid parity check found errors** (Advisory).
- **Unraid parity check overdue**: la última comprobación tiene más de 40
  días (Advisory).
- **Unraid cache nearly full**: un disco de caché con más del 90 % de
  llenado durante treinta minutos (Advisory).
- **Unraid container stopped**: un contenedor configurado para iniciarse
  con el array no ha estado en ejecución durante diez minutos (Advisory).

## La página del monitor {#the-device-page}

El panel Array indica en una frase qué va mal, muestra el llenado del
array, la última comprobación de paridad, los contenedores y VMs en
ejecución, y luego lista cada disco — paridad primero, luego datos, luego
caché — con su estado, temperatura y llenado, y los contenedores que
deberían estar en ejecución.

## Crear una clave de API de solo lectura para DumbMonit {#create-a-read-only-api-key-for-dumbmonit}

1. En Unraid 7.2 o posterior la API viene integrada. En Unraid 6.12 a
   7.1, instala primero el plugin Unraid Connect desde la pestaña Apps:
   trae la misma API, y no se necesita ningún inicio de sesión en Unraid
   Connect para uso local.

2. Crea una clave de API con el rol Viewer, que lee todo y no puede
   cambiar nada: en Settings > Management Access > API Keys, o desde la
   terminal de Unraid.

    ```
    unraid-api apikey --create --name dumbmonit --roles VIEWER --description "DumbMonit monitoring" --json
    ```

3. Copia la clave. En DumbMonit, introduce la dirección del servidor
   Unraid, por ejemplo «tower.lan», y pega la clave como API key. Si la
   interfaz web se sirve por HTTPS, elige https como protocolo.

!!! warning
    Unraid no reporta ninguna temperatura para un disco que está
    detenido, y DumbMonit nunca despierta uno: un disco dormido
    simplemente no tiene temperatura hasta que empieza a girar.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Clave de API | Una clave con el rol Viewer, enviada como `x-api-key`. |

Dirección: el nombre de host o IP del servidor Unraid, `host:puerto`, o
una URL. Opciones:

- **Protocol** (`http`) y **Port** (80): los de la interfaz web. Con SSL
  definido como Strict en Settings > Management Access, usa `https`, y
  la dirección `myunraid.net` si ese es el certificado.
- **Accept an unverifiable certificate**: para un certificado
  autofirmado.
- **Timeout per request** (15 s).

## Solución de problemas {#troubleshooting}

«Unraid refused the API key (401)»: la clave es incorrecta o fue
eliminada.

«Unraid accepted the API key but refused to answer (403)»: la clave no
tiene ningún rol, o un rol más limitado que Viewer.

Sin cifras de contenedor, VM o paridad: esa consulta opcional falló (VMs
apagadas, una versión más antigua de la API sin el campo); el resto se
sigue leyendo. El motivo está en el registro del servidor a nivel debug.
