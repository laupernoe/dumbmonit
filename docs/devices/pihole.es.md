# Pi-hole

El bloqueador de anuncios DNS: si está bloqueando, cuántas consultas
responde y bloquea, cuán antiguas son sus listas de bloqueo, si sus
resolutores upstream fallan, si espera una actualización, y los mensajes
de su propia página de diagnóstico.

DumbMonit lee la API REST de Pi-hole v6 (`/api`). Pi-hole 5 y anteriores
no tienen tal API y no están soportados.

Lo que va mal en un Pi-hole rara vez se nota: el DNS sigue respondiendo. El
bloqueo se pausa «durante cinco minutos» y nunca vuelve, la actualización
semanal de listas de bloqueo (gravity) lleva un mes fallando, un resolutor
upstream responde SERVFAIL, y la página de diagnóstico que nadie abre ha
estado indicando el motivo todo este tiempo.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_pihole_`. Pi-hole cuenta
las consultas de las últimas 24 horas, una ventana móvil que sube y baja:
esas cifras son gauges, con sufijo `_24h`, no contadores.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `blocking_enabled` | 1 cuando el bloqueo está activado; 0 cuando está desactivado, en pausa o falló | |
| `blocking_timer_seconds` | tiempo restante antes de que un bloqueo en pausa se reactive solo | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | consultas respondidas y bloqueadas en las últimas 24 horas | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | consultas enviadas upstream, respondidas desde la caché, dominios distintos | |
| `queries_per_second` | promedio del último minuto | |
| `replies_servfail_24h` | respuestas SERVFAIL en las últimas 24 horas: un resolutor upstream que falla aparece aquí primero | |
| `clients_active`, `clients_total` | clientes vistos | |
| `gravity_domains` | dominios en las listas de bloqueo | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | última reconstrucción de la lista de bloqueo, y hace cuánto | |
| `version_info` | valor 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 1 cuando se publica una versión más reciente de un componente, 0 en caso contrario; ausente hasta que la comprobación diaria de Pi-hole se haya ejecutado | `component` (`core`, `web`, `ftl`, `docker`) |
| `updates_available` | actualizaciones a instalar: la imagen en Docker (core, web y FTL vienen con ella), en caso contrario cada componente desactualizado | |
| `messages`, `messages_by_type` | mensajes en la página de diagnóstico | `type` (`LOAD`, `RATE_LIMIT`, `LIST`…) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | el demonio FTL | |
| `privacy_level` | el nivel de privacidad de Pi-hole, de 0 a 3 | |
| `scrape_errors` | llamadas de API secundarias que fallaron durante la última comprobación | |

En una rama de desarrollo, Pi-hole compara commits en lugar de números de
versión, y DumbMonit también.

Las [reglas integradas](../alerting/rules.md#pi-hole) que se aplican:
Pi-hole blocking disabled, Pi-hole blocklists stale, Pi-hole update
available, Pi-hole diagnosis messages, además de Monitor unreachable.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la comprobación; abrir la
página nunca se conecta a Pi-hole. Muestra si el bloqueo está activado (y
cuándo termina una pausa), las consultas y la proporción bloqueada en las
últimas 24 horas, el tamaño y antigüedad de la lista de bloqueo, los
clientes, las versiones con cualquier actualización pendiente, y el número
de mensajes de diagnóstico.

## Crear una contraseña de aplicación en Pi-hole {#create-an-app-password-in-pi-hole}

1. Se requiere Pi-hole v6 o posterior: la versión 5 no tenía API REST.
   Pi-hole no tiene cuenta de solo lectura; lo más cercano es una
   contraseña de aplicación, que no puede cambiar ningún ajuste.

2. En la interfaz web de Pi-hole, abre Settings → Web interface / API y
   cambia la página de Basic a Expert. Haz clic en Configure app
   password, copia la contraseña mostrada, y luego haz clic en Enable new
   app password. Pi-hole la muestra solo una vez, y una nueva reemplaza a
   la anterior.

3. Una contraseña de aplicación se mantiene de solo lectura para los
   ajustes mientras webserver.api.app_sudo sea false, el valor por
   defecto. Compruébalo en la máquina Pi-hole (en Docker, antepón el
   comando con docker exec y el nombre del contenedor):

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. En DumbMonit, introduce la dirección de Pi-hole, por ejemplo «pi.hole»
   o «192.168.1.53», y pega la contraseña de aplicación. Para un Pi-hole
   cuya interfaz web no tiene contraseña, elige No password en su lugar.

5. DumbMonit solo lee. Nunca pausa el bloqueo y nunca cambia una lista, y
   mantiene una sesión de API abierta entre comprobaciones en lugar de
   abrir una por comprobación: Pi-hole permite 16 sesiones a la vez.

!!! warning
    La contraseña de aplicación sigue permitiendo pausar el bloqueo y
    editar listas a través de la API: resérvala solo para DumbMonit.
    Pi-hole sirve HTTP sin cifrar en el puerto 80, y la contraseña cruza
    la red cada vez que se abre una sesión: en una red no confiable, usa
    HTTPS en el puerto 443.

Una contraseña de aplicación también inicia sesión sin el segundo factor
cuando la autenticación de dos factores está activada: para eso existe.
Para revocarla, vacía `webserver.api.app_pwhash` en Settings → All
settings, o activa una nueva: la anterior deja de funcionar de inmediato.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Contraseña de aplicación | La contraseña generada arriba. DumbMonit la intercambia por una sesión, enviada en la cabecera `X-FTL-SID`, y reutiliza esa sesión mientras Pi-hole la conserve (30 minutos tras su último uso). |
| Sin contraseña | Solo para un Pi-hole cuya interfaz web no tiene contraseña. |

Dirección: un nombre de host o IP (`pi.hole`), `host:puerto`, o una URL
completa (`https://pi.hole`); la ruta de la interfaz web o de la API,
copiada del navegador, se ignora. Opciones: protocolo (HTTP por defecto),
puerto (80), comprobación de certificado (para HTTPS en 443, el propio
certificado de Pi-hole es autofirmado) y tiempo de espera de la petición
(10 s).

Si las comprobaciones fallan con «no free API session», las sesiones de
Pi-hole están todas ocupadas, normalmente por un script que inicia sesión
en cada ejecución sin cerrarla. Caducan tras 30 minutos;
`webserver.api.max_sessions` eleva el límite.
