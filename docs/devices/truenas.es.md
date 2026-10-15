# TrueNAS

Servidor de almacenamiento ZFS (TrueNAS SCALE, ahora TrueNAS Community
Edition): la salud de cada pool y el disco que falló, scrubs y
resilvers, uso de pool y dataset frente a sus cuotas, snapshots y
replicación, temperatura de disco y autopruebas SMART, las alertas que
TrueNAS lanza por sí mismo, servicios y la máquina.

!!! warning "Probado solo contra una instancia de prueba"

    Esta integración se construyó y verificó contra TrueNAS 25.04.2.6,
    instalado desde la ISO oficial en una máquina virtual para este
    propósito — un pool RAIDZ1, datasets, tareas de snapshot y
    replicación, y luego el mismo pool con un disco desconectado — no
    contra un NAS que lleva años guardando las fotos de una familia. Una
    máquina virtual no tiene temperaturas de disco ni resultados SMART,
    así que esos siguen el código fuente de TrueNAS en lugar de una
    respuesta observada. Cada llamada es de lectura. Dinos qué falla.

## El fallo para el que existe esto {#the-failure-this-is-for}

Un vdev en espejo o RAIDZ que pierde un disco sigue sirviendo los datos.
La compartición permanece montada, las copias de seguridad siguen
ejecutándose, la interfaz web de cualquier otra herramienta sigue en
verde — y el pool ahora no tiene ninguna redundancia. El segundo disco se
lleva el pool. `truenas_pool_healthy` cae a 0 en el primero, la regla
**NAS pool degraded** se activa, y la página del monitor nombra el
disco.

## La página del monitor {#the-device-page}

Más allá de las gráficas genéricas, un monitor TrueNAS muestra tres
paneles, leídos de lo que guardó la sonda — abrir la página nunca
consulta al propio NAS:

* **Pools and disks** — cada pool con su estado en una palabra, la
  propia frase de ZFS cuando algo va mal, cuán lleno está, su
  distribución (espejo, RAIDZ, caché, log), sus recuentos de error de
  lectura, escritura y checksum, y sobre todo el nombre de cualquier
  disco que no esté `ONLINE`. Un scrub o resilver en progreso muestra su
  avance. Luego cada disco: modelo, serie, tipo, tamaño, pool,
  temperatura y el resultado de su última autoprueba SMART.
* **Datasets and protection** — cada dataset con su uso frente a su
  cuota, su recuento de snapshots y cuándo una tarea de snapshot lo
  cubrió por última vez; el último scrub de cada pool y si está
  pendiente; cada tarea de replicación y snapshot periódico con su
  estado, su última ejecución, el último snapshot que gestionó y,
  cuando falló, la propia frase de error de TrueNAS.
* **NAS health** — las alertas que TrueNAS ha lanzado por sí mismo, las
  más severas primero, luego los servicios detenidos, luego la máquina:
  versión, tiempo de actividad, carga, memoria, CPU.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_truenas_`.

| Familia | Métricas | Etiquetas |
|---|---|---|
| Identidad | `version_info` (la versión, sin el prefijo `TrueNAS-SCALE-` que las versiones más antiguas ponen delante) | `version` |
| Máquina | `uptime_seconds`, `load1/5/15`, `memory_total_bytes`, `cpu_count` | |
| Pools | `pool_healthy` (el propio veredicto de ZFS: 0 en `DEGRADED` igual que en `FAULTED`), `pool_warning` (resilvering, funciones no activadas), `pool_status_info`, `pool_size/allocated/free_bytes`, `pool_used_percent`, `pool_fragmentation_percent` | `pool`, y `status` para la serie info |
| Dispositivos de pool | `pool_device_errors` (acumulados desde el último `zpool clear`), `pool_devices_unhealthy` (ambos ausentes cuando el pool no se puede abrir) | `pool`, `kind` (`read`, `write`, `checksum`) |
| Scrubs | `pool_scan_running`, `pool_scan_percent` (solo mientras se ejecuta), `pool_last_scrub_age_seconds`, `pool_last_scrub_errors`, `pool_scrub_threshold_days` | `pool`, `function` (`SCRUB`, `RESILVER`) |
| Datasets | `dataset_used_bytes`, `dataset_available_bytes`, `dataset_quota_bytes` y `dataset_quota_used_percent` (ausente sin cuota), `dataset_snapshots`, `dataset_snapshot_age_seconds` (antigüedad del último snapshot tomado por una tarea periódica que cubre el dataset; ausente cuando ninguna lo hace), `dataset_locked` (solo datasets cifrados) | `dataset`, `pool` |
| Snapshots | `snapshots` (total) | |
| Discos | `disk_temperature_celsius`, `disk_size_bytes`, `disk_smart_failed` (ausente para un disco nunca probado) | `disk`, `serial` |
| Alertas | `alerts` — el número de alertas activas por nivel, cero incluido | `level` (`INFO` … `EMERGENCY`) |
| Tareas | `replication_error`, `replication_last_run_age_seconds`, `snapshot_task_error`, `snapshot_task_last_run_age_seconds` (solo tareas activadas) | `task` |
| Servicios | `service_running` (solo servicios configurados para iniciarse con el NAS) | `service` |
| Recolección | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Algunas cosas que hace TrueNAS que vale la pena conocer:

* **El último scrub vive solo en el registro de escaneo del pool.** No
  hay historial de scrub en la API, y un resilver sobrescribe el
  registro de escaneo: tras un resilver, la fecha del último scrub
  simplemente se desconoce hasta que termina el siguiente, y
  `pool_last_scrub_age_seconds` está ausente en lugar de incorrecto.
* **Un pool que no se puede abrir** — exportado, o con demasiados
  discos ausentes — todavía aparece, sin distribución ni tamaños. Su
  `pool_healthy` es 0; las series de dispositivo y tamaño están
  ausentes en lugar de en cero.
* **Una cuota sin definir no es una cuota de cero.** TrueNAS responde
  `parsed: null` para «sin cuota» y DumbMonit no publica ninguna serie
  de cuota para ese dataset. `quota` tiene precedencia sobre `refquota`
  cuando ambas están definidas.
* **Los recuentos de snapshot vienen de la propia caché de ZFS**
  (`snapshots_count`), y el total del recuento rápido de snapshots de
  TrueNAS — nunca se lista ningún snapshot. El total es mayor que la
  suma de los recuentos por dataset: incluye los snapshots de los
  datasets internos de TrueNAS, que la lista de datasets oculta.
* **La antigüedad del snapshot más reciente de un dataset** es la
  última ejecución de la tarea de snapshot periódico que lo cubre,
  directa o a través de un padre recursivo. TrueNAS no puede dar la
  fecha del snapshot más reciente sin listarlos todos, así que un
  dataset que ninguna tarea cubre no tiene antigüedad — lo cual es en sí
  mismo digno de saber.
* **Las temperaturas se leen de la caché de TrueNAS** (`only_cached`),
  refrescada por TrueNAS cada cinco minutos aproximadamente. Un disco
  dormido nunca se despierta, y un disco sin lectura no tiene serie.
* **Nada se inicia nunca.** Ningún scrub, ninguna prueba SMART, ninguna
  comprobación de actualización, ninguna ejecución de replicación. La
  sonda lee.
* **Las rutas que cambiaron entre versiones son opcionales.**
  `zfs/snapshot` se convirtió en `pool/snapshot` en la 25.10 y ambas se
  prueban; `smart/test/results` desapareció en la 25.10 y entonces se
  omite en silencio.

Las reglas integradas que se aplican: Monitor unreachable, NAS pool
degraded, NAS disk errors, NAS pool almost full, NAS scrub found
errors, NAS scrub overdue, NAS disk failed its SMART test, NAS disk too
hot, NAS dataset near its quota, NAS replication failed, NAS snapshot
task failed, NAS snapshots stale, TrueNAS alert raised, NAS service
stopped. Las notificaciones nombran el pool, disco, dataset, tarea o
servicio afectado.

## Qué preparar en TrueNAS {#what-to-prepare-in-truenas}

Los pasos de abajo son los que muestra el aviso junto al formulario. Una
cosa que conviene saber primero: a través de la API REST que usa
DumbMonit, **TrueNAS no tiene ninguna clave de solo lectura**. Sus roles
de solo lectura solo se conectaron a su API WebSocket más nueva; una
clave cuyo usuario es un Read-Only Administrator es rechazada, con un
`403` vacío, en cada llamada REST. Esto se comprobó en TrueNAS
25.04.2.6: una clave de Read-Only Administrator recibió `403` en las 55
rutas REST probadas, y esa misma clave funcionó a través de la API
WebSocket. La clave debe pertenecer a un usuario con el privilegio Local
Administrator — así que obtiene un usuario y un grupo propios, y nada
más.

1. En la interfaz web: Credentials → Groups → Add. Nombra el grupo como
   sigue y dale el privilegio Local Administrator.

    ```
    dumbmonit
    ```

2. Por qué Local Administrator y no Read-Only Administrator: TrueNAS
   solo conectó sus roles de solo lectura a su API WebSocket. A través
   de la API REST que usa DumbMonit, una clave de solo lectura se
   rechaza en cada llamada. El propio DumbMonit solo lee: nunca inicia
   un scrub, una prueba SMART, una actualización o una replicación.

3. Credentials → Users → Add. Nombra la cuenta como sigue, haz que
   dumbmonit sea su grupo primario, y deja desactivados el acceso shell,
   sudo y SSH: ninguno de ellos es necesario.

    ```
    dumbmonit
    ```

4. Crea la clave: Credentials → Users → API Keys → Add, elige el
   usuario dumbmonit y dale un nombre a la clave. TrueNAS muestra la
   clave solo una vez: cópiala. En TrueNAS 24.10 la ruta es el menú de
   usuario arriba a la derecha → API Keys → Add, y una clave no
   pertenece a ningún usuario.

5. En DumbMonit, introduce la dirección del NAS, por ejemplo «nas.lan»,
   y pega la clave. Mantén HTTPS: TrueNAS revoca una clave que recibe
   alguna vez por HTTP plano desde otra máquina.

!!! warning

    Esta clave puede cambiar cualquier cosa en el NAS: TrueNAS no ofrece
    ninguna clave de solo lectura a través de su API REST. Trátala como
    la contraseña de una cuenta que puede borrar tus pools, mantenla
    solo para DumbMonit, y revócala desde la misma página si se filtra.
    TrueNAS también viene con un certificado autofirmado: si la conexión
    se rechaza por ese motivo, marca «Accept an unverifiable
    certificate» en las opciones.

Dirección: `192.168.1.20`, `nas.lan`, `nas.lan:8443`, `[fd00::20]` o una
URL completa. El esquema `https` y el puerto 443 se añaden si faltan.

TrueNAS ha anunciado que la API REST se eliminará en TrueNAS 26, en
favor de la API WebSocket. Hasta entonces, REST es lo que sirve cada
versión hasta la 25.10.

## Opciones {#options}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `scheme` | Protocol | `https` | Mantén HTTPS: TrueNAS revoca una clave de API que recibe por HTTP plano desde otra máquina. |
| `port` | Web interface port | `443` | Se usa si la dirección no da un puerto. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | TrueNAS viene con un certificado autofirmado por defecto. |
| `request_timeout_seconds` | Timeout per request (seconds) | `20` | Tiempo permitido para cada llamada a la API, de 1 a 120. |
| `datasets` | Watch the datasets | `true` | Uso frente a cuota, recuento de snapshots, último cambio de snapshot. |
| `disks` | Watch the disks | `true` | Inventario y temperaturas en caché; un disco dormido nunca se despierta. |
| `smart` | Read the SMART test results | `true` | La última autoprueba de cada disco; ninguna se inicia nunca. |
| `alerts` | Read TrueNAS's own alerts | `true` | La lista de alertas que TrueNAS conserva por sí mismo, las descartadas excluidas. |
| `tasks` | Watch replication and snapshot tasks | `true` | Estado, última ejecución y último snapshot de cada tarea. |
| `services` | Watch the services | `true` | Servicios configurados para iniciarse con el NAS, y si se ejecutan. |

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| Error de autenticación mencionando Local Administrator | El usuario de la clave no es un administrador completo — típicamente un Read-Only Administrator. TrueNAS rechaza tal clave en cada llamada REST, con un `403` vacío. |
| Error de autenticación «Invalid API key» | La clave se escribió mal, se eliminó, caducó — o TrueNAS la revocó porque se envió una vez por HTTP plano desde otra máquina. Crea una nueva y mantén HTTPS. |
| Error de certificado | Certificado autofirmado: instala uno de confianza (ACME está integrado) o activa `insecure_tls`. |
| Sin temperaturas de disco | El NAS se ejecuta en una máquina virtual, o TrueNAS aún no ha consultado los discos desde que arrancó. Las lecturas vienen de su caché. |
| Sin serie SMART | Nunca se ha ejecutado ninguna autoprueba en ese disco, o el NAS ejecuta TrueNAS 25.10, que eliminó esa llamada. Programa pruebas SMART en Data Protection. |
| Sin `pool_last_scrub_age_seconds` | El último escaneo de ese pool fue un resilver, que borra el registro del último scrub, o el pool nunca se escaneó. |
| Un dataset no tiene serie de cuota | No tiene cuota. Una cuota sin definir no es una cuota de cero. |
| «NAS pool degraded» y la página no muestra ningún nombre de disco | El pool no se puede abrir en absoluto (exportado, o demasiados discos ausentes): TrueNAS entonces no reporta ninguna distribución. |
| `scrape_errors` por encima de cero | Una llamada opcional falló por un motivo distinto a una ruta faltante — un tiempo de espera, normalmente. Las demás métricas no se afectan. |
