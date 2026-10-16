# Veeam Backup & Replication

Un servidor Veeam Backup & Replication a través de su API REST: jobs
cuya última ejecución falló o terminó con un aviso, sesiones fallidas de
las últimas 24 horas, repositorios que se llenan, la licencia, y — por
cada equipo o VM protegidos — su última copia de seguridad exitosa.

DumbMonit inicia sesión con una cuenta de Windows que solo tiene el rol
Veeam Backup Viewer (OAuth2 password grant en `/api/oauth2/token`,
puerto 9419) y conserva el token durante su vida de quince minutos. Cada
llamada lleva la cabecera obligatoria `x-api-version`, `1.1-rev0` por
defecto: la revisión que llegó con la versión 12.0.

Los estados de job (`/api/v1/jobs/states`) son la única lectura
obligatoria. Los demás — `/api/v1/serverInfo`, `/api/v1/sessions`
(creadas en las últimas 24 horas), `/api/v1/backupInfrastructure/repositories/states`,
`/api/v1/license` y `/api/v1/restorePoints` (los puntos de restauración
de cada objeto protegido, en todo el servidor, los más recientes
primero) — se leen cuando el rol lo permite. Veeam reserva algunas rutas
para el rol Backup Administrator, la licencia entre ellas en las
versiones actuales: un rechazo ahí se muestra como «no readable», nunca
como un error. Un servidor demasiado antiguo para conocer
`/api/v1/restorePoints` — la ruta apareció después de la 12.0 — se trata
de la misma forma: sin dispositivo, sin error.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de la referencia de la API
    REST de Veeam y su centro de ayuda (autenticación, versionado, los
    modelos de job, sesión, repositorio, licencia y punto de
    restauración), no contra un servidor de copias de seguridad en
    ejecución. Qué rutas puede leer el rol Viewer varía según la
    versión: dinos qué rechaza tu servidor.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_veeam_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `version_info` | `version` | Valor 1: el build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Por job. |
| `job_failed`, `job_warning` | `job`, `type` | 1 cuando la última ejecución de un job activado falló, o terminó con un aviso. Un job desactivado conserva su último resultado, que ya no dice nada: no cuenta como ninguno de los dos. |
| `job_result_info` | `job`, `type`, `result` | Valor 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Desde la última ejecución; ausente para un job que nunca se ejecutó. |
| `job_progress_percent` | `job`, `type` | Mientras el job se ejecuta. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Recuentos; como máximo se describen 200 jobs uno por uno. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sesiones creadas en las últimas 24 horas, por resultado. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Por repositorio. Un repositorio de almacenamiento de objetos no tiene capacidad: solo se publica su espacio usado. |
| `license_valid` | | 1 cuando el estado de la licencia es `Valid`. |
| `license_info` | `status`, `type`, `edition` | Valor 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Hasta que caducan la licencia y el contrato de soporte; ausente para una licencia perpetua. |
| `license_instances_licensed`, `license_instances_used` | | Uso de licencia por instancia. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1 cuando la cuenta puede leerlo, 0 cuando Veeam lo rechazó. |

Las [reglas integradas](../alerting/rules.md#veeam-backup--replication)
que se aplican:

- **Veeam job failed**: la última ejecución de un job activado falló
  (Warning, recordado diariamente).
- **Veeam job ended with a warning** (Advisory, recordado diariamente).
- **Veeam repository nearly full**: más del 90 % durante treinta minutos
  (Advisory).
- **Veeam license expiring**: menos de 30 días restantes (Advisory,
  recordado diariamente). Solo cuando la cuenta puede leer la licencia.
- **Client device stale**: un equipo o VM protegidos ha superado
  `device_stale_days` sin una copia de seguridad exitosa (Advisory,
  recordado diariamente); consulta [Dispositivos](#devices) abajo.

## Dispositivos {#devices}

Cada objeto con al menos un punto de restauración — una VM, un equipo
físico o virtual protegido por agente, una compartición de archivos — se
publica bajo la familia genérica compartida por toda integración de
DumbMonit que rastrea dispositivos protegidos —
`dumbmonit_client_device_*`, con prefijo sin `veeam`, documentada por
completo en [Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | Qué | Etiquetas |
|---|---|---|
| `last_backup_timestamp_seconds` | El punto de restauración más reciente del objeto | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo una vez que un objeto ha superado `device_stale_days` (3 por defecto) sin un nuevo punto de restauración; impulsa la regla [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (siempre `backup`) |

**Límites que conviene conocer**: un job protege sus objetos como grupo
(`jobs/states` solo da un recuento, `objectsCount`); la fecha por objeto
viene en su lugar de `/api/v1/restorePoints`, leída para todo el
servidor en una llamada. Veeam crea un punto de restauración solo una
vez que una copia de seguridad ha tenido al menos un éxito parcial, así
que el más reciente por objeto ya es «la última copia de seguridad
exitosa» — nada se filtra por resultado aquí. Veeam no expone una
«última conexión» separada de la propia copia de seguridad, así que
`last_seen_timestamp_seconds` y la señal `connection` nunca se publican;
un dispositivo muestra `last_connection` como «—» en la tabla de la
página del monitor. El `type` del dispositivo es el propio nombre de
plataforma de Veeam (`VSphere`, `WindowsPhysical`, `HyperV`…), mostrado
tal cual.

## La página del monitor {#the-device-page}

El panel Backups indica en una frase qué va mal, muestra los jobs
fallidos, las sesiones fallidas de las últimas 24 horas, los jobs en
ejecución y la licencia, y luego lista los jobs activados — los fallos
primero — con su tipo y última ejecución, y los repositorios con su
llenado y espacio libre. Sigue la tabla de dispositivos protegidos
(nombre, plataforma, última copia de seguridad, estado), de la familia
genérica de arriba.

## Dar a DumbMonit una cuenta Veeam Backup Viewer {#give-dumbmonit-a-veeam-backup-viewer-account}

1. En el servidor de copias de seguridad, crea una cuenta local de
   Windows para DumbMonit con una contraseña larga y aleatoria y ningún
   otro derecho en la máquina (una cuenta de dominio también funciona).

    ```
    net user dumbmonit * /add
    ```

2. En la consola de Veeam, abre el menú principal > Users and Roles >
   Security, haz clic en Add, introduce la cuenta (por ejemplo
   VBR01\dumbmonit) y dale solo el rol Veeam Backup Viewer.

3. En DumbMonit, introduce la dirección del servidor de copias de
   seguridad, por ejemplo «vbr.lan», y la cuenta como HOST\dumbmonit o
   DOMAIN\dumbmonit con su contraseña. La API REST escucha en el puerto
   9419 con un certificado autofirmado: marca Accept an unverifiable
   certificate a menos que hayas instalado el tuyo.

!!! warning
    Veeam reserva algunas rutas de la API REST para el rol Backup
    Administrator, la licencia entre ellas en las versiones actuales.
    Con el rol Viewer, DumbMonit lee los jobs, sesiones, repositorios y
    puntos de restauración, y muestra la licencia como no legible: nunca
    pide más.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario y contraseña | La cuenta con el rol Veeam Backup Viewer, como `HOST\dumbmonit` o `DOMAIN\dumbmonit`. |

Dirección: el nombre de host o IP del servidor de copias de seguridad,
`host:puerto`, o una URL. Opciones:

- **Protocol** (`https`) y **Port** (9419): los del servicio de API
  REST.
- **API version** (`1.1-rev0`): enviada como `x-api-version`. Cámbiala
  solo si el servidor la rechaza; un servidor v13 documenta `1.3-rev1`.
- **Accept an unverifiable certificate**: el propio certificado de la
  API REST es autofirmado.
- **Timeout per request** (15 s).
- **Device staleness** (`device_stale_days`, 3 días por defecto):
  consulta [Dispositivos](#devices) arriba.

## Solución de problemas {#troubleshooting}

«Veeam refused the login»: el nombre de usuario necesita su host o
dominio (`VBR01\dumbmonit`), o la cuenta no tiene ningún rol Veeam.

«Veeam refused to list the jobs (403)»: la cuenta tiene un rol que no
puede leer jobs; dale Veeam Backup Viewer.

«Veeam refused the API version»: define la opción API version con una
que el servidor liste en `https://<server>:9419/swagger`.

La licencia muestra «Not readable»: esperado con el rol Viewer en las
versiones actuales; DumbMonit no pide más.

Sin dispositivo, o el panel Devices está vacío: `restore_points` lee
«Not readable», probablemente porque el servidor es anterior a la ruta
(cualquier cosa antes de las revisiones posteriores de la 12.0) en lugar
de un problema de rol — los puntos de restauración son inventario
corriente, no una página solo para administradores.
