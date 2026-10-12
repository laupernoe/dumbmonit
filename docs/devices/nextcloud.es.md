# Nextcloud

Si Nextcloud está sirviendo o atascado: modo mantenimiento dejado
activado, una actualización de base de datos pendiente tras una
actualización, actualizaciones de Nextcloud y de apps, usuarios activos,
espacio libre para el directorio de datos, el tamaño de la base de
datos, y el OPcache de PHP.

DumbMonit lee el `status.php` público y las estadísticas de la app
**serverinfo**, que viene con Nextcloud, con un **token de
monitorización** (`NC-Token`). El token abre esas estadísticas y nada
más: no puede iniciar sesión, listar usuarios, abrir un archivo ni
cambiar un ajuste.

El fallo que vale la pena vigilar: **una actualización que no
terminó**. Tras una actualización, Nextcloud puede quedarse en modo
mantenimiento, o esperar a `occ upgrade`; la página web entonces muestra
un aviso que nadie ve hasta que alguien intenta sincronizar. Ambos se
leen desde `status.php`, que responde incluso entonces.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_nextcloud_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `maintenance` | 1 mientras el modo mantenimiento está activado | |
| `needs_db_upgrade` | 1 mientras Nextcloud espera a `occ upgrade` | |
| `version_info` | valor 1 | `version` |
| `update_available` | 1 cuando la propia comprobación de actualización de Nextcloud encontró una versión más reciente | `available_version` |
| `apps_installed`, `app_updates_available` | apps instaladas, y cuántas tienen una versión más reciente en la tienda de apps | |
| `app_update_available` | valor 1 por app con una actualización (como máximo 32) | `app`, `available_version` |
| `active_users` | usuarios vistos en los últimos 5 minutos, hora y día | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | cuentas, archivos y comparticiones en la instancia | |
| `free_space_bytes` | espacio libre donde Nextcloud guarda sus datos | |
| `database_size_bytes` | tamaño de la base de datos | |
| `platform_info` | valor 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | el `memory_limit` de PHP | |
| `opcache_enabled`, `opcache_full` | OPcache activado; OPcache sin espacio | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | cuán lleno está OPcache y cuán bien sirve | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | el pool de PHP-FPM, cuando Nextcloud se ejecuta detrás de PHP-FPM (las imágenes `-fpm`, AIO) | |

Mientras Nextcloud está en mantenimiento o espera una actualización, su
API responde 503: solo se escriben entonces `maintenance`,
`needs_db_upgrade` y `version_info`, y el dispositivo no se reporta
inaccesible.

Las [reglas integradas](../alerting/rules.md#self-hosted-applications)
que se aplican: Nextcloud stuck in maintenance, Nextcloud database
upgrade pending, Nextcloud update available, Nextcloud OPcache full,
Nextcloud free space low, además de Device unreachable.

## Trabajos en segundo plano (cron) {#background-jobs-cron}

Nextcloud no le indica al token de monitorización cuándo se ejecutaron
por última vez los trabajos en segundo plano: solo una cuenta con todos
los derechos en la instancia puede leer eso, y DumbMonit no pide una.
Vigila el cron con un [Heartbeat](push.md) en su lugar: crea uno en
DumbMonit, y luego llama a su URL al final de la línea de cron que
ejecuta los trabajos en segundo plano de Nextcloud, para que solo llame
cuando `cron.php` tuvo éxito:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Con un intervalo esperado de 5 minutos, el heartbeat alerta en cuanto el
cron se detiene, sea cual sea el motivo (un contenedor reiniciado sin su
cron, un error de PHP, trabajos en segundo plano dejados en AJAX).

## Dispositivos cliente {#client-devices}

Nextcloud no expone una lista a nivel de administrador de clientes de
sincronización o dispositivos móviles a través de ninguna API. La página
personal «Devices & sessions» (Settings → Security) lista solo la cuenta
con sesión iniciada actualmente, respaldada por una sesión de navegador
en lugar de un token de API, y el único equivalente a nivel de
administrador es la línea de comandos (`occ user:auth-tokens:list
<user>`, una cuenta a la vez) — algo que el token de monitorización no
puede alcanzar, y algo que DumbMonit no pide a un administrador
automatizar en una sonda. La API `serverinfo` que lee DumbMonit tampoco
tiene ninguna lista de dispositivos: da recuentos de usuarios activos
(`active_users`, arriba), nunca quién o qué son.

Este es un límite deliberado, no una funcionalidad faltante: dar a
DumbMonit una lista de dispositivos significaría ampliar el token de
monitorización a una cuenta de administración completa, lo que el resto
de esta página se esfuerza por evitar. Si Nextcloud alguna vez expone
esto a través de OCS, DumbMonit seguirá el mismo patrón de [dispositivos
cliente](../alerting/rules.md) ya usado para Immich, Proxmox Backup
Server y Veeam.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca llama a Nextcloud. Indica primero qué va mal (mantenimiento,
actualización pendiente, OPcache lleno o desactivado), luego
actualizaciones, y después usuarios activos, espacio libre, tamaño de
base de datos, usuarios, archivos y cifras de OPcache, y luego las apps a
actualizar por nombre y la plataforma (base de datos, PHP).

## Dar a Nextcloud un token de monitorización {#give-nextcloud-a-monitoring-token}

1. La app serverinfo de Nextcloud, incluida y activada con Nextcloud,
   responde a un token de monitorización que abre sus estadísticas y
   nada más: ningún archivo, ningún usuario, ninguna compartición. En el
   servidor Nextcloud, define un token largo y aleatorio e imprímelo de
   vuelta. Ejecuta occ como el usuario del servidor web; en Docker, pon
   docker exec -u www-data y el nombre del contenedor delante.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Si occ dice que serverinfo está desactivada, activa primero. Justo después de definir el token, Nextcloud puede rechazar la primera lectura (401) antes de que surta efecto: DumbMonit reintenta una vez tras dos segundos, así que un objetivo recién creado no muestra un falso error de «token rechazado».

    ```
    php occ app:enable serverinfo
    ```

3. En DumbMonit, introduce la dirección con la que abres Nextcloud, por
   ejemplo «cloud.example.com» (HTTPS en 443) o «http://10.0.0.5:8080»,
   y pega el token. El host debe ser uno de los dominios de confianza de
   Nextcloud, o Nextcloud rechaza la petición.

4. DumbMonit solo lee status.php y las estadísticas de serverinfo.
   Nunca abre un archivo, nunca lista usuarios y nunca cambia un ajuste,
   y el token no puede iniciar sesión en ningún sitio.

!!! warning
    Nextcloud no le indica al token cuándo se ejecutaron por última vez
    los trabajos en segundo plano (cron): solo una cuenta con todos los
    derechos en la instancia puede leer eso. Para que te avisen cuando
    el cron se detiene, añade un Heartbeat en DumbMonit y llama a su URL
    al final de la línea de cron, como muestra la documentación.

Con Nextcloud AIO, ejecuta occ en el contenedor
`nextcloud-aio-nextcloud`: `docker exec -u www-data
nextcloud-aio-nextcloud php occ …`. Por qué no un grupo de
administración delegada: Nextcloud solo abre la API serverinfo al token
o a miembros del grupo `admin` completo, y la delegación no cambia eso.

Comprobar el token a mano, desde el host de DumbMonit:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de monitorización (NC-Token) | El token de serverinfo. DumbMonit lo envía en la cabecera `NC-Token`, con `OCS-APIRequest: true`. |

Dirección: un nombre de host (`cloud.example.com`), `host:puerto`, o una
URL completa con un prefijo de ruta (`https://example.com/nextcloud`);
se acepta un `/index.php` o `/login` final. Opciones: protocolo (HTTPS
por defecto), puerto (443), comprobación de certificado y tiempo de
espera de la petición (15 s).

El recuento de actualizaciones de apps consulta la caché de la tienda de
apps de Nextcloud; en una instancia que no puede alcanzar la tienda de
apps, ese recuento se queda en 0.

Validado contra Nextcloud 35.0.1 (imagen oficial).
