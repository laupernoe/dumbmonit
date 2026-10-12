# VictoriaMetrics y VictoriaLogs

La propia base de datos de series temporales y la base de datos de logs:
si aceptan lo que se les envía, cuánto disco queda antes de que dejen de
aceptar nada, y qué rechazan.

Dos tipos de dispositivo comparten esta página, porque los dos productos
comparten sus fundamentos: `victoriametrics` para una VictoriaMetrics de
un solo nodo, y `victorialogs` para una VictoriaLogs de un solo nodo.
Ambas publican su propia salud en `/health` y `/metrics`, en el puerto de
su API; DumbMonit lee esas dos y nada más. Nunca ejecuta una consulta
contra tus datos y nunca escribe.

Estos servidores rara vez caen por completo. Caen *a medias*: siguen
respondiendo consultas y los dashboards siguen abiertos, mientras cada
nueva muestra o línea de log se rechaza, porque el disco cruzó el límite
de solo lectura o los datos vinieron con un timestamp fuera de la
retención. Lo que les envía datos (vmagent, Prometheus remote write,
Vector, Fluent Bit, Promtail) reintenta, y luego descarta. Esta página
trata de ver eso venir.

## Qué vigila {#what-it-watches}

Las métricas tienen el prefijo `dumbmonit_victoriametrics_` o
`dumbmonit_victorialogs_`. Los totales son contadores: las gráficas y
las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `healthy` | 1 cuando `/health` responde OK, 0 cuando responde cualquier otra cosa | |
| `version_info` | valor 1 | `version` |
| `rows_ingested_total` | muestras (VictoriaMetrics) o líneas de log (VictoriaLogs) aceptadas | |
| `bytes_ingested_total` | volumen de log aceptado (solo VictoriaLogs) | |
| `rows_rejected_total` | todo lo rechazado, siempre presente, incluso en cero | |
| `rows_rejected_reason_total` | lo mismo, por motivo: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | espacio libre y total del volumen de almacenamiento | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: por debajo de él, el almacenamiento se vuelve de solo lectura | `path` |
| `disk_headroom_percent` | espacio libre restante por encima de ese límite, como proporción del disco | `path` |
| `read_only` | 1 una vez que el almacenamiento ha pasado a solo lectura | |
| `data_size_bytes` | datos en disco | |
| `slow_inserts_total`, `rows_added_total` | muestras nuevas que tomaron la vía lenta, y todas las muestras añadidas (solo VictoriaMetrics) | |
| `active_series` | series que recibieron una muestra en la última hora (solo VictoriaMetrics) | |
| `http_errors_total` | peticiones respondidas con un error | |
| `log_errors_total` | mensajes de error que registró el servidor | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Una muestra rechazada se pierde.** VictoriaMetrics rechaza una muestra
más antigua que su retención o más de dos días en el futuro, un valor de
etiqueta más largo que `-maxLabelValueLen`, una serie por encima de
`-maxLabelsPerTimeseries`, y toda muestra de una serie nueva una vez
alcanzado `-storage.maxHourlySeries` o `-storage.maxDailySeries`.
VictoriaLogs rechaza líneas fuera de su retención y líneas con demasiados
campos. El cliente normalmente no recibe ningún error por esto: solo
este contador lo sabe.

**Las inserciones lentas** son muestras cuya serie VictoriaMetrics tuvo
que buscar en disco en lugar de en su caché. Cada serie completamente
nueva es una, así que un despliegue causa una ráfaga; una proporción que
se mantiene por encima del 5 % significa que al servidor le falta
memoria para sus series activas. Ese es el umbral que da la
documentación de VictoriaMetrics. Tras un reinicio las cachés del servidor
están frías, así que la proporción puede mantenerse alta un tiempo: la regla
solo juzga un servidor en marcha desde hace más de 15 minutos, y espera media
hora antes de activarse. Los contadores vuelven a cero con el servidor; las
reglas usan incrementos que tienen en cuenta los reinicios, así que un
reinicio no se cuenta como datos rechazados.

Las [reglas integradas](../alerting/rules.md#log-and-metrics-servers) que
se aplican: Metrics or log storage read-only, Metrics or log storage
almost read-only, Metrics or log server unhealthy, Metrics or log data
refused, VictoriaMetrics slow inserts, además de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta el servidor. Indica, en este orden:

1. **Qué va mal, en una frase**: salud, almacenamiento (*Read-only*, o
   cuánto disco queda por encima del límite), datos rechazados en las
   últimas 24 horas, inserciones lentas, errores registrados. Cada línea
   lleva una palabra además de un color.
2. **Qué hace el servidor**: muestras o líneas ingeridas por segundo,
   series activas o volumen ingerido, datos en disco, tiempo de
   actividad.
3. **Qué se rechazó en las últimas 24 horas, por motivo**, y el espacio
   libre de cada ruta de almacenamiento frente a su límite de solo
   lectura.

## Dejar que DumbMonit lea el propio estado de VictoriaMetrics {#let-dumbmonit-read-victoriametrics-own-health}

1. Nada que instalar: VictoriaMetrics publica su propia salud en /health
   y /metrics, en el puerto de su API (8428 por defecto). Comprueba
   desde el host de DumbMonit que ambos respondan.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. Si VictoriaMetrics se ejecuta con -httpAuth.username y
   -httpAuth.password, elige Username / password abajo e introduce
   esos. Detrás de vmauth o un proxy inverso que espera un token bearer,
   elige Bearer token. Si -metricsAuthKey está definido, /metrics solo
   responde con esa clave en la URL, que DumbMonit no envía: protege
   /metrics con la autenticación básica de arriba en su lugar.

3. En DumbMonit, introduce la dirección del servidor, por ejemplo
   «victoriametrics.lan» o «http://10.0.0.5:8428». Detrás de un proxy
   inverso, introduce la URL completa con su prefijo de ruta. Esto cubre
   el servidor de un solo nodo; los componentes de un clúster de
   VictoriaMetrics exponen otras métricas y no están cubiertos.

4. DumbMonit solo lee /health y /metrics. Nunca ejecuta una consulta
   contra tus datos y nunca escribe nada.

!!! warning
    El espacio libre en disco es lo que hay que vigilar: por debajo de
    -storage.minFreeDiskSpaceBytes (100 MB por defecto) VictoriaMetrics
    pasa a solo lectura y rechaza cada muestra nueva, mientras sigue
    respondiendo consultas como si nada hubiera pasado. DumbMonit
    muestra el margen restante por encima de ese límite y avisa por
    debajo del 10 %.

## Dejar que DumbMonit lea el propio estado de VictoriaLogs {#let-dumbmonit-read-victorialogs-own-health}

1. Nada que instalar: VictoriaLogs publica su propia salud en /health y
   /metrics, en el puerto de su API (9428 por defecto). Comprueba desde
   el host de DumbMonit que ambos respondan.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_free_disk_space_bytes
    ```

2. Si VictoriaLogs se ejecuta con -httpAuth.username y
   -httpAuth.password, elige Username / password abajo e introduce
   esos. Detrás de vmauth o un proxy inverso que espera un token bearer,
   elige Bearer token. Si -metricsAuthKey está definido, /metrics solo
   responde con esa clave en la URL, que DumbMonit no envía: protege
   /metrics con la autenticación básica de arriba en su lugar.

3. En DumbMonit, introduce la dirección del servidor, por ejemplo
   «victorialogs.lan» o «http://10.0.0.6:9428». Detrás de un proxy
   inverso, introduce la URL completa con su prefijo de ruta.

4. DumbMonit solo lee /health y /metrics. Nunca ejecuta una consulta
   LogsQL y nunca lee una línea de log.

!!! warning
    El espacio libre en disco es lo que hay que vigilar: por debajo de
    -storage.minFreeDiskSpaceBytes (10 MB por defecto) VictoriaLogs pasa
    a solo lectura y rechaza cada línea de log nueva. DumbMonit muestra
    el margen restante por encima de ese límite y avisa por debajo del
    10 %.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Sin autenticación | Lo por defecto: `/health` y `/metrics` están abiertos a menos que los hayas protegido. |
| Nombre de usuario / contraseña | El `-httpAuth.username` y `-httpAuth.password` del servidor, o del proxy inverso delante de él. |
| Token bearer | Un token que vmauth o tu proxy inverso espera en `Authorization: Bearer`. |

Dirección: un nombre de host o IP (`victoriametrics.lan`, `10.0.0.5`),
`host:puerto`, o una URL completa con un prefijo de ruta detrás de un
proxy inverso (`https://metrics.lan/vm`). Opciones: protocolo (HTTP por
defecto), puerto (8428 o 9428), comprobación de certificado y tiempo de
espera de la petición (10 s).
