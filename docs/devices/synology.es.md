# Synology DSM

NAS Synology a través de la API web de DSM: volúmenes, pools de
almacenamiento, salud de disco, temperatura, tareas Hyper Backup,
tareas Active Backup for Business, y Synology Drive y Photos. La
primera razón para monitorizar un NAS son sus discos: un disco que se
calienta o cuyo estado SMART se degrada anuncia un fallo mucho antes de
que un volumen se caiga.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_synology_`.

| Familia | Métricas | Etiquetas |
|---|---|---|
| Sistema | `system_info` (modelo, versión DSM, firmware…), `uptime_seconds`, `temperature_celsius`, `temperature_warning`, `system_crashed`, `system_need_repair`, `storage_health` | `model`, `dsm_version`, `firmware` |
| CPU y memoria | `cpu_usage_percent`, `cpu_user/system/other_percent`, `cpu_cores`, `memory_total/used/available/installed_bytes`, `memory_usage_percent`, `swap_usage_percent` | |
| Red | `network_rx_bytes_per_second`, `network_tx_bytes_per_second` | `interface` |
| Volúmenes | `volume_status`, `volume_total/used/available_bytes`, `volume_used_percent`, `volume_used_warning_percent`, `volume_used_critical_percent` | `volume`, `fs_type`, `raid_type` |
| Pools de almacenamiento y cachés SSD | `pool_status`, `pool_failed_disks`, `pool_total/used_bytes`, y los mismos cuatro bajo `ssd_cache_` | `pool` o `ssd_cache`, `name`, `raid_type` |
| Discos | `disk_status`, `disk_smart_status`, `disk_temperature_celsius`, `disk_size_bytes`, `disk_bad_sector_exceeded`, `disk_life_below_threshold`, `disk_remaining_life_percent` (solo SSD: la vida que el disco dice que le queda), `disk_unc_count` (el contador de sectores ilegibles de DSM, el que compara con su umbral de sectores dañados), `disk_info` | `disk`, `name`; en `disk_info`: `model`, `serial`, `vendor`, `firmware`, `type`, `ssd` |
| Hyper Backup | `backup_tasks`, `backup_running`, `backup_last_result`, `backup_last_run_age_seconds`, `backup_last_success_age_seconds`, `backup_next_run_in_seconds` | por tarea |
| Synology Drive | `drive_up`, `drive_team_folders`, `drive_team_folder_info`, `drive_connections` (ver abajo) | `drive_team_folder_info`: `name` |
| Synology Photos | `photos_installed` (ver abajo) | |
| Recolección | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Las métricas de Active Backup for Business son la excepción: tienen el
prefijo `dumbmonit_abb_` (ver abajo).

`volume_status`, `disk_status` y `disk_smart_status` llevan la propia
palabra de DSM en una etiqueta (`status`, `smart_status`) y un valor en
una escala: `0` normal, `1` atención (un estado transitorio como
`repairing` o `checking`, o una palabra que DSM introdujo y que
DumbMonit todavía no conoce), `2` crítico (`degrade`, `crashed`,
`failed`…). Lo desconocido deliberadamente cuenta como `1`, nunca `0`.

Un fallo parcial sigue siendo una sonda exitosa: si el inventario de
almacenamiento falla, el estado del sistema y la utilización igual se
publican y el fallo se cuenta en `scrape_errors`. Solo un fallo en la
llamada de descubrimiento de API o en la conexión condena toda la
sonda.

!!! note "Lo que las filas de disco no pueden mostrar"
    Los recuentos de sectores reasignados viven en la tabla de
    atributos SMART por disco, que DSM solo sirve a través de una
    llamada no documentada cuya forma DumbMonit no ha podido confirmar.
    Las filas por tanto muestran lo que da `load_info`: el veredicto de
    sectores dañados de DSM, su contador de sectores ilegibles (y su
    crecimiento), y la vida restante del SSD.

!!! note "Lo que cambia entre versiones de DSM"
    La misma llamada no responde los mismos campos en cada DSM, y
    DumbMonit lee ambas formas en lugar de asumir una:

    * `disk_bad_sector_exceeded` viene de `exceed_bad_sector_thr` hasta
      DSM 7.1 y de `sb_days_left_critical` — la estimación de DSM de los
      días restantes antes de que el disco alcance su umbral de
      sectores dañados — en DSM 7.2 y posteriores, donde el primer
      campo desaparece. Lo mismo para `disk_life_below_threshold` y
      `remain_life_danger`;
    * `disk_remaining_life_percent` es un número plano en DSM más
      antiguo y un objeto que lleva el valor y si DSM confía en él en
      DSM 7.2 y posteriores. Un valor que DSM marca como no confiable
      no se publica;
    * `volume_used_warning_percent` y `volume_used_critical_percent`
      repiten los umbrales definidos en DSM. DSM 7.4 ya no los
      devuelve, así que en un NAS reciente esas dos series simplemente
      no existen — nada se inventa en su lugar.

## La página del dispositivo {#the-device-page}

![Un dispositivo Synology: de un vistazo, volúmenes, discos y Active Backup](../assets/screenshots/device-synology-light.png){ loading=lazy }

Un dispositivo Synology obtiene su propio panel sobre las gráficas:

* **At a glance** — CPU, memoria (usada del total), temperatura y
  tiempo de actividad, con el propio veredicto de almacenamiento de
  DSM como una placa (sano, atención, crashed, repair needed);
* **Volumes** — una fila por volumen: barra de uso, usado del total,
  tipo RAID y sistema de archivos, el estado del volumen en la palabra
  de DSM (normal, degraded, crashed, repairing…);
* **Storage pools** — una fila por pool: tipo RAID, el estado del pool
  en la palabra de DSM, cuántos de sus discos reporta DSM como
  fallidos, y cuánto de él ya está entregado a volúmenes. Aquí es donde
  vive la redundancia: un RAID 5 que pierde un disco mantiene el
  volumen encima de él en `normal` mientras reconstruye, así que la
  fila del volumen no dice nada y esta lo dice todo. Un pool
  completamente asignado a sus volúmenes es el estado final normal, no
  un aviso, así que esta cifra es una frase en lugar de un medidor con
  color;
* **SSD cache** — una fila por caché: su tipo RAID, su estado y su
  tamaño. La sección está ausente en un NAS sin caché. El `load_info`
  de DSM no lleva ningún modo de caché ni tasa de aciertos, así que
  ninguno de los dos se muestra;
* **Disks** — una fila por bahía: modelo, tamaño, temperatura,
  veredicto SMART, veredicto de sectores dañados de DSM, el contador
  de sectores ilegibles y, en un SSD, la vida restante como una
  pequeña barra;
* **Active Backup for Business** — las tareas, luego cada dispositivo
  respaldado con su ritmo y una franja de treinta días (ver abajo).

Todo viene de lo que guardó la sonda: abrir la página nunca consulta al
NAS.

### Reglas integradas para el NAS {#built-in-rules-for-the-nas}

| Regla | Severidad | Se activa cuando |
|---|---|---|
| Synology disk SMART warning | advisory | DSM ya no reporta el estado SMART del disco como `normal`, durante 15 minutos. |
| Synology disk failed | warning | DSM reporta el disco como crashed o failed, durante 5 minutos. |
| Synology disk bad sectors | warning | El recuento de sectores dañados supera el umbral definido en DSM, durante 15 minutos. |
| Synology disk bad sectors growing | advisory | Aparecieron sectores ilegibles nuevos en las últimas 24 horas (`delta(…disk_unc_count[24h]) > 0`). |
| Synology SSD wearing out | advisory | Un SSD tiene menos del 10 % de su vida nominal restante, durante una hora. |
| Synology volume degraded | warning | Un volumen está degraded o crashed, durante 5 minutos. |
| Synology storage pool degraded | warning | Un pool de almacenamiento o caché SSD está degraded o crashed, durante 5 minutos. Un RAID 5 que pierde un disco se muestra aquí primero: DSM mantiene el volumen encima de él en `normal` mientras reconstruye. |
| Synology volume almost full | advisory | Un volumen está al 90 % lleno o más, durante 15 minutos. |
| Synology temperature high | advisory | Un disco está por encima de 55 °C, o DSM lanzó su aviso de temperatura, durante 15 minutos. |
| Synology memory high | advisory | Uso de memoria por encima del 95 % durante 15 minutos. |

## Active Backup for Business

Active Backup for Business (ABB) respalda PCs, servidores físicos,
máquinas virtuales y servidores de archivos en el NAS. Si el paquete
está instalado y en ejecución, DumbMonit lee sus tareas en cada sonda;
si no lo está, el NAS simplemente no anuncia la API y no se pide nada.
La opción `abb` desactiva toda la familia.

Lo mismo vale para Hyper Backup: DSM solo lista las APIs de un paquete
mientras ese paquete se ejecuta, así que un NAS cuyo Hyper Backup está
detenido — o que solo recibe copias a través de Hyper Backup Vault —
no publica ninguna métrica `backup_*` y tampoco reporta ningún error.
DumbMonit pregunta al NAS qué APIs tiene antes de llamar a cualquiera
de ellas, así que un paquete faltante es silencio, nunca una sonda
fallida.

| Métrica | Valor | Etiquetas |
|---|---|---|
| `dumbmonit_abb_tasks` | Número de tareas. | |
| `dumbmonit_abb_task_last_status` | Última ejecución: `1` éxito, `0` fallo, `2` en ejecución, `-1` desconocido (nunca ejecutada, cancelada, sin copia). Un éxito *parcial* cuenta como fallo: al menos un dispositivo no se respaldó. | `task`, `task_id`, `source_type`, `result` |
| `dumbmonit_abb_task_last_success_seconds` | Antigüedad de la última ejecución de copia exitosa, en segundos. Ausente para una tarea que nunca tuvo éxito. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_enabled` | `1` si la tarea tiene una programación y no está en pausa, `0` si solo se ejecuta a mano. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_device_count` | Dispositivos adjuntos a la tarea. | `task`, `task_id`, `source_type` |

`task` es el nombre de la tarea tal como se muestra en ABB;
`source_type` es `pc`, `vm`, `physical_server`, `file_server` o `nas`;
`result` es el resultado en bruto de la última ejecución (`success`,
`partial_success`, `fail`, `cancel`, `no_backup`, `running`, `none`).

La antigüedad se toma del historial de tareas filtrado en ejecuciones
de copia exitosas, no del «último resultado» de la tarea: tras una
purga nocturna de versiones, el último resultado es la purga, no la
copia. Esto cuesta una petición por tarea, limitado a veinte tareas.

Reglas integradas sobre tareas:

| Regla | Severidad | Se activa cuando |
|---|---|---|
| Active Backup task failed | warning | La última ejecución falló, o fue un éxito parcial, durante 10 minutos. La notificación nombra la tarea. |
| Active Backup too old | advisory | Ninguna ejecución exitosa en más de dos días. |
| Active Backup task disabled | info | La tarea no tiene programación, o su copia continua está en pausa, durante una hora. |

### Dispositivos y su ritmo {#devices-and-their-rhythm}

Una regla a nivel de tarea es correcta para un servidor e incorrecta
para un portátil: la gente no deja sus portátiles encendidos, y «sin
copia en 24 horas» es el lunes por la mañana para cada PC de oficina.
DumbMonit por tanto también vigila ABB **dispositivo por dispositivo**,
contra los propios hábitos del dispositivo.

En cada sonda, el recolector lee las ejecuciones por dispositivo que
guarda ABB (`SYNO.ActiveBackup.Overview` `list_device_transfer_size`,
una llamada para todos los dispositivos, solo lo que cambió desde la
sonda anterior) y las copia en su propia base de datos — limitada a
noventa días por NAS — para que el modelo sobreviva a un reinicio sin
volver a leer un mes de historial. Luego, para cada dispositivo, mira
los últimos treinta días.

| Métrica | Valor | Etiquetas |
|---|---|---|
| `dumbmonit_abb_device_state` | `0` ok, `1` idle, `2` learning, `3` running, `4` overdue, `5` failing, `6` never. | `device`, `device_id`, `task`, `task_id`, `state` |
| `dumbmonit_abb_device_overdue` | `1` mientras el dispositivo está vencido, `0` en caso contrario. | `device`, `device_id`, `task`, `task_id` |
| `dumbmonit_abb_device_consecutive_failures` | Intentos fallidos consecutivos desde el último éxito (una ejecución cancelada no cuenta como ninguno). | igual |
| `dumbmonit_abb_device_last_success_seconds` | Antigüedad de la última ejecución exitosa de este dispositivo. Ausente si nunca tuvo éxito. | igual |
| `dumbmonit_abb_device_typical_interval_seconds` | Mediana del hueco entre dos éxitos, días libres excluidos. Ausente mientras aprende. | igual |

#### Cómo se juzga «overdue» {#how-overdue-is-judged}

En palabras simples, para cada dispositivo:

1. **¿Qué días está activo?** Un día de la semana en el que el
   dispositivo intentó al menos una copia en los últimos treinta días
   — exitosa o no; un fallo prueba que la máquina estaba encendida tanto
   como un éxito — es un *día activo*. Una vez que hay al menos dos
   semanas de historial, un día de la semana sin ningún intento es un
   *día libre*. Un portátil usado de lunes a viernes obtiene el sábado
   y el domingo como días libres; un servidor no tiene ninguno.
2. **¿Con qué frecuencia hace copia?** Los huecos entre éxitos
   consecutivos se recogen en *tiempo activo*: los días libres
   completos que contiene un hueco no se cuentan (de viernes por la
   tarde a lunes por la tarde es un día de tiempo activo, no tres). Su
   mediana es el *intervalo típico* del dispositivo; su percentil 90 es
   lo que parece «una semana lenta».
3. **¿Cuánto tiempo se le permite?** El margen es `max(1.5 × hueco
   p90, 2 × intervalo típico, 36 h)`. Un escritorio nocturno obtiene
   48 h; una máquina respaldada cada tres días obtiene unas seis;
   nadie obtiene menos de un día y medio. Mientras existan menos de
   dos huecos utilizables (un dispositivo añadido esta semana), el
   margen es 72 h — suficiente para cruzar un fin de semana sin una
   falsa alarma.
4. **¿Cuánto tiempo activo ha pasado?** El tiempo desde el último
   éxito, menos los días libres completos que contiene. Un portátil
   que hizo su última copia el viernes a las 20:00 tiene, el lunes a
   las 08:00, doce horas de tiempo activo detrás, no sesenta.
5. **El veredicto**, en este orden: *running* (una copia está en
   curso); *failing* (los últimos dos o más intentos fallaron — un
   éxito parcial es un fallo, una ejecución cancelada no es ninguno de
   los dos); *never* (sin éxito registrado); *overdue* (el tiempo
   activo supera el margen); *idle* (hoy es uno de sus días libres:
   apagado, como siempre); *learning* (ritmo aún no conocido); *ok*.

Solo *overdue* genera `dumbmonit_abb_device_overdue`, y solo *failing*
alimenta la regla de fallo. Un portátil inactivo un domingo no genera
nada; el mismo portátil todavía en silencio el martes por la noche sí.

Los días y horas se expresan en la hora local del servidor (`TZ` del
contenedor), que es donde vive la persona que lee la página. La página
del dispositivo muestra, por dispositivo: el estado en una palabra, el
último éxito y último resultado, el ritmo en palabras («weekdays,
around 20:00», «every ~3 days, on Mon, Wed, Fri, around 03:00», «still
learning (2 runs in 30 days)») y una franja de treinta días — verde
azulado para un día con éxito, rojo para un fallo, ámbar para una
ejecución cancelada, gris para sin ejecución.

Reglas integradas sobre dispositivos:

| Regla | Severidad | Se activa cuando |
|---|---|---|
| Active Backup device overdue | advisory | `dumbmonit_abb_device_overdue > 0` durante 30 minutos. |
| Active Backup device failing | warning | Dos o más intentos fallidos consecutivos, durante 10 minutos. |

!!! warning "Permisos"
    ABB solo responde a una cuenta del grupo `administrators` con el
    paquete permitido en sus permisos de aplicación, o una cuenta a la
    que se le ha delegado el paquete (DSM 7: Active Backup for Business
    → Settings → Privileges). Una cuenta a la que se le niega la
    aplicación no puede leer las tareas de ABB: la llamada se rechaza
    (código 105) y se cuenta en `scrape_errors`. O bien permite o
    delega el paquete a la cuenta de monitorización, o desmarca «Watch
    Active Backup for Business» (`abb = false`) para dejar de preguntar.

!!! note "Una API no documentada"
    Synology no documenta la API web de ABB. DumbMonit sigue la forma
    registrada por proyectos comunitarios (`SYNO.ActiveBackup.Task`
    `list`, `SYNO.ActiveBackup.Log` `list_result` y
    `SYNO.ActiveBackup.Overview` `list_device_transfer_size`, todas
    versión 1) y descubre su ruta a través de `SYNO.API.Info`. Un DSM
    cuyas respuestas difieren produce métricas faltantes, nunca una
    sonda fallida; un ABB demasiado antiguo para ofrecer el resumen por
    dispositivo conserva sus métricas de tarea y simplemente no tiene
    ritmo de dispositivo.

    La versión 1 es deliberada. DSM 7.4 anuncia estas APIs como
    versiones 1 a 2, pero solo la versión 1 lleva los métodos de
    lectura: pedidos en la versión 2 todos responden «the requested
    method does not exist», lo cual parece una diferencia de firmware y
    no lo es. La misma respuesta también vuelve cuando falta el token
    CSRF, así que DumbMonit pide a DSM uno al iniciar sesión y lo envía
    en cada llamada.

## Synology Drive

DSM no documenta la API web de Synology Drive. DumbMonit lee tres
llamadas cuyo nombre y forma vienen de la biblioteca comunitaria
`N4S4/synology-api`, que las llama desde las mismas pantallas que la
consola de administración de Synology Drive, y sigue la misma regla
que Active Backup for Business: una API que un NAS no anuncia es un
paquete no instalado, nunca un fallo.

| Métrica | Valor | Etiquetas |
|---|---|---|
| `dumbmonit_synology_drive_up` | `1` mientras el servicio responde a `get_status`. Ausente si el paquete no está instalado. | |
| `dumbmonit_synology_drive_team_folders` | Número de carpetas de equipo. | |
| `dumbmonit_synology_drive_team_folder_info` | Valor `1` por carpeta de equipo (como máximo 32). | `name` |
| `dumbmonit_synology_drive_connections` | Número de conexiones activas (clientes de escritorio y móviles actualmente sincronizados). | |

!!! note "Por qué las conexiones son solo un recuento"
    La página «Client list» de la consola de administración muestra el
    nombre del dispositivo, usuario, tipo de aplicación, estado en
    línea, dirección IP y ubicación de cada conexión, pero eso es una
    descripción de la página, no de los campos JSON de la API: ni
    Synology ni ningún proyecto comunitario documenta cómo
    `SYNO.SynologyDrive.Connection` nombra realmente sus campos. En
    lugar de adivinar el nombre de un dispositivo o su fecha de última
    sincronización a partir de una clave no confirmada, DumbMonit solo
    publica el recuento de conexiones. Los dispositivos individuales se
    conectarán a la misma tabla de [dispositivos cliente](../alerting/rules.md)
    usada por Immich, Proxmox Backup Server y Veeam una vez que esa
    forma se confirme con la respuesta de un NAS real.

La opción `drive` (activada por defecto) desactiva toda la familia.

## Synology Photos

Synology Photos incluye una API de navegación bien documentada, pero
ni Synology ni las bibliotecas comunitarias exponen una lista a nivel
de administración de dispositivos de copia móvil o su última copia —
la app móvil muestra su propio estado de copia localmente, y nada lo
centraliza en el lado del NAS a través de una API. DumbMonit por tanto
solo reporta que el paquete está instalado:
`dumbmonit_synology_photos_installed` (valor `1`), leído del catálogo
de API sin ninguna petición extra al NAS — el mismo catálogo
`SYNO.API.Info` que ya responde para cualquier otro paquete. La opción
`photos` (activada por defecto) lo desactiva.

## Qué preparar en el NAS {#what-to-prepare-on-the-nas}

Los pasos de abajo son los que muestra el aviso junto al formulario. El
principio: una cuenta reservada para monitorización, despojada de todo
lo que la API de DSM no necesita — nunca la cuenta con la que inicias
sesión.

1. En DSM, abre Control Panel → User & Group → User y haz clic en
   Create. Nombra la cuenta como sigue y dale una contraseña larga
   usada en ningún otro sitio.

    ```
    dumbmonit
    ```

2. Únela a grupos: marca administrators. DSM solo responde a las
   llamadas de almacenamiento, volumen y SMART de disco a ese grupo;
   sin él el NAS se muestra vivo pero no dice nada sobre sus discos.
   Los siguientes dos pasos retiran todo lo demás.
3. Asigna permisos de carpeta compartida: No access en cada carpeta
   compartida. Asigna permisos de aplicación: Deny todo salvo DSM, más
   Active Backup for Business si quieres que se lean sus tareas.
   Omite las páginas de cuota y límite de velocidad.
4. La verificación en dos pasos debe permanecer desactivada para esta
   cuenta: ningún monitor automatizado puede escribir un código de un
   solo uso. Si Control Panel → Security → Account la fuerza,
   restringe la regla a grupos en los que este usuario no está, o
   exímela.
5. Los paquetes de copia solo se leen mientras están instalados y en
   ejecución: DSM no anuncia en absoluto la API de un paquete
   detenido, así que las tareas de Hyper Backup entonces quedan en
   silencio en lugar de fallar, y un NAS que solo recibe copias (Hyper
   Backup Vault) no tiene tareas propias que mostrar. Active Backup
   for Business adicionalmente solo responde a una cuenta autorizada a
   usarlo (Active Backup for Business → Settings → Privileges); de lo
   contrario desmarca la opción «Watch Active Backup for Business» de
   DumbMonit para dejar de preguntar.
6. En DumbMonit, introduce la dirección del NAS (HTTPS, puerto 5001
   por defecto), y luego el nombre de usuario y contraseña de esta
   cuenta.

!!! warning
    El grupo administrators lo requiere la API de almacenamiento de
    DSM, no DumbMonit. Por eso esta cuenta no obtiene ninguna carpeta
    compartida, ninguna aplicación y una contraseña usada en ningún
    otro sitio: puede leer el NAS, no tocar tus archivos.

!!! note "Qué derechos dan qué métricas"
    La API de DSM no necesita los mismos derechos para cada llamada. El
    estado del sistema (modelo, versión, temperatura, tiempo de
    actividad) responde a cualquier cuenta. CPU, memoria, volúmenes y,
    sobre todo, el estado SMART de disco requieren el grupo
    `administrators`. En DSM 7, la delegación «System monitoring»
    (Control Panel → User & Group → Delegation) basta para la
    utilización pero no para el inventario de almacenamiento. Una
    cuenta sin derechos particulares por tanto da un NAS que está
    «vivo», pero nada sobre sus discos.

Documentación del fabricante: [Create a user in DSM](https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/file_user_create).

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Cuenta DSM | **User name**: la cuenta creada arriba. **Password**: su contraseña. La verificación en dos pasos debe estar desactivada para esta cuenta: ningún monitor automatizado puede escribir un código de un solo uso. |

Dirección: `192.168.1.30`, `nas.lan` o `nas.lan:5001`. Sin puerto, se
usa 5001 por HTTPS y 5000 por HTTP.

## Opciones {#options}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `scheme` | Protocol | `https` | HTTPS encaja con un NAS recién salido de fábrica. HTTP solo si DSM escucha solo en texto plano. Opciones: `https`, `http`. |
| `port` | DSM port | *(vacío)* | Se usa si la dirección no da un puerto. Vacío: 5001 por HTTPS, 5000 por HTTP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Un NAS viene con un certificado autofirmado por defecto: activa esto si la conexión se rechaza por ese motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tiempo permitido para cada llamada a la API, de 1 a 120. El inventario de almacenamiento puede despertar discos dormidos. |
| `abb` | Watch Active Backup for Business | `true` | Lee las tareas de Active Backup for Business. Necesita el paquete instalado y una cuenta autorizada a usarlo; un NAS sin el paquete simplemente se omite. |
| `drive` | Watch Synology Drive | `true` | Lee si el servicio Synology Drive responde, sus carpetas de equipo y su recuento de conexiones activas. Un NAS sin el paquete simplemente se omite. |
| `photos` | Watch Synology Photos | `true` | Muestra si el paquete Synology Photos está instalado, sin ninguna petición extra al NAS. Synology no expone una lista de dispositivos de copia móvil a través de ninguna API. |

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| Error de autenticación | Contraseña incorrecta, o se requiere verificación en dos pasos para esta cuenta. DSM responde HTTP 200 incluso en caso de fallo; el código de error en el cuerpo JSON es lo que cuenta, y DumbMonit lo lee. |
| Error de certificado | Certificado autofirmado: activa `insecure_tls`. Esto es un error de configuración, nunca «unreachable»: un NAS que responde no se muestra como apagado. |
| Sin métricas de disco o volumen | La cuenta no está en el grupo `administrators` (ver la nota arriba). |
| Sin métricas `dumbmonit_abb_*`, `scrape_errors` en 1 | La cuenta no puede usar Active Backup for Business: delégale el paquete, o desactiva la opción `abb`. Sin el paquete instalado no hay error ni métrica. |
| Métricas de tarea pero sin filas de dispositivo en la página | El NAS no anuncia `SYNO.ActiveBackup.Overview` (ABB más antiguo), o el dispositivo se sondea a través de un agente relay, que mantiene el historial en memoria de su lado: las métricas existen, el calendario no. |
| Un portátil se muestra «Overdue» el día después de añadirlo | Menos de tres éxitos: el modelo todavía está aprendiendo y permite 72 horas entre copias; se estabiliza una vez que se conocen dos semanas de hábitos. |
| Los discos giran en cada sonda | El inventario de almacenamiento despierta discos dormidos. Alarga el intervalo de comprobación. |
