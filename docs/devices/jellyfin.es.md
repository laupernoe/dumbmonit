# Jellyfin

Si Jellyfin mantiene sus bibliotecas al día y atiende a sus espectadores:
tareas programadas que fallaron en su última ejecución (el escaneo de
biblioteca primero), plugins que ya no cargan, un reinicio pendiente y qué
se está reproduciendo ahora, transcodificado o no.

DumbMonit lee la API de Jellyfin con una **clave de API** creada en el
panel. Nunca inicia una tarea, nunca detiene una reproducción y nunca
cambia un ajuste.

El fallo que vale la pena vigilar: **el escaneo de biblioteca que falla
cada noche**. Un recurso compartido de red desmontado, un disco lleno, y
las nuevas películas y episodios dejan de aparecer; el propio Jellyfin
sigue reproduciendo lo que ya conoce, y la única traza es una línea en
Scheduled Tasks.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_jellyfin_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `pending_restart` | 1 cuando Jellyfin espera un reinicio (tras instalar o actualizar un plugin) | |
| `shutting_down` | 1 mientras Jellyfin se apaga | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | tareas cuya última ejecución falló o se abortó; tareas en ejecución ahora | |
| `scheduled_task_failed` | valor 1 por tarea fallida (como máximo 32) | `task`, `error` (primera línea del mensaje) |
| `library_scan_last_ok`, `library_scan_age_seconds` | última ejecución de «Scan Media Library»: si tuvo éxito o no, y hace cuánto terminó | |
| `plugins`, `plugins_broken` | plugins instalados; los que no cargaron o no admiten esta versión | |
| `plugin_broken` | valor 1 por plugin roto | `plugin`, `status` |
| `sessions`, `streams`, `streams_paused`, `transcodes` | sesiones de cliente; las que reproducen algo, en pausa, y transcodificando | |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

Jellyfin no comprueba sus propias actualizaciones (lo hace el paquete o la
imagen), así que no hay métrica de actualización.

Las [reglas integradas](../alerting/rules.md#self-hosted-applications) que
se aplican: Jellyfin scheduled task failed, Jellyfin plugin broken,
Jellyfin restart pending, además de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca llama a Jellyfin. Indica primero qué va mal (tareas fallidas,
plugins rotos, un reinicio pendiente), luego streams, transcodificaciones,
sesiones y la antigüedad del último escaneo de biblioteca, y después las
tareas fallidas y los plugins rotos por nombre con el error propio de
Jellyfin.

## Crear una clave de API en Jellyfin {#create-an-api-key-in-jellyfin}

1. En Jellyfin: Dashboard → API Keys (bajo Advanced) → +, y nombra la clave
   de la siguiente forma. Cópiala.

    ```
    DumbMonit
    ```

2. Jellyfin no puede limitar lo que una clave puede hacer: cada clave tiene
   todos los derechos en el servidor. Sigue siendo la única forma de leer
   las tareas programadas y la reproducción de todos; un usuario corriente
   no ve ninguna de las dos cosas.

3. En DumbMonit, introduce la dirección de Jellyfin, por ejemplo
   «jellyfin.lan» (puerto 8096) o «https://media.example.com» detrás de un
   proxy inverso, y pega la clave.

4. DumbMonit solo lee /System/Info, /ScheduledTasks, /Sessions y /Plugins.
   Nunca inicia una tarea, nunca detiene una reproducción y nunca cambia un
   ajuste.

!!! warning
    Como la clave tiene todos los derechos, mantenla en una red de
    confianza y pon Jellyfin detrás de HTTPS si es accesible por internet.
    Si alguna vez se filtra, revócala en la misma página: nada más la usa.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Clave de API | Enviada en la cabecera `Authorization: MediaBrowser … Token="…"`, la forma que Jellyfin 10.11 y posteriores aceptan por defecto. |

Dirección: un nombre de host (`jellyfin.lan`), `host:puerto`, o una URL
completa con un prefijo de ruta (`https://example.com/jellyfin`); se acepta
un `/web` final. Opciones: protocolo (HTTP por defecto), puerto (8096),
comprobación de certificado y tiempo de espera de la petición (15 s).

Validado contra Jellyfin 12.1.0 (imagen oficial).
