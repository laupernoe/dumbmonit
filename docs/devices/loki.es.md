# Grafana Loki

El propio servidor de logs: si está listo, qué líneas de log rechaza y por
qué, si sus chunks llegan al almacenamiento, y si su write-ahead log
todavía tiene disco.

Loki publica `/ready` y `/metrics` en su puerto HTTP; DumbMonit lee esos
dos y nada más. Nunca ejecuta una consulta LogQL y nunca lee una línea de
log.

El fallo que vale la pena vigilar es silencioso. Cuando Loki rechaza una
línea (más antigua que `reject_old_samples_max_age`, por encima del límite
de tasa de ingestión o de stream del tenant, más larga que
`max_line_size`) el emisor recibe un 4xx, y Promtail, Alloy o Fluent Bit
suelen descartar el lote. Grafana no muestra nada salvo un hueco,
descubierto el día que alguien busca el log de un incidente. El segundo es
chunks que no se pueden escribir en el almacenamiento: Loki los mantiene en
memoria, y los pierde en el siguiente reinicio.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_loki_`. Los totales son
contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `ready` | 1 cuando `/ready` responde `ready`, 0 mientras Loki arranca o cuando un componente no puede escribir ni leer | |
| `version_info` | valor 1 | `version` |
| `received_lines_total`, `received_bytes_total` | lo que recibió el distribuidor, todos los tenants | |
| `discarded_lines_total` | líneas rechazadas, siempre presente, incluso en cero | |
| `discarded_lines_reason_total` | lo mismo por motivo: `greater_than_max_sample_age`, `rate_limited`, `stream_limit`, `line_too_long`, `per_stream_rate_limit`… | `reason` |
| `flush_failures_total`, `flush_queue_length` | chunks que fallaron al llegar al almacenamiento, chunks esperando para hacerlo | |
| `memory_streams` | streams retenidos en memoria por el ingester | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | escrituras del write-ahead log perdidas por un disco lleno, y cuán lleno está ese disco | |
| `requests_total`, `request_errors_total` | peticiones HTTP y gRPC, y las respondidas con un 5xx o un error gRPC. Un 4xx es culpa del cliente y no se cuenta como error. | |
| `ring_unhealthy_members` | miembros de los hash rings marcados como no saludables | |
| `panics_total`, `log_errors_total` | panics recuperados, mensajes de error registrados | |
| `retention_last_run_timestamp_seconds` | última ejecución correcta de retención del compactor, cuando la retención está activada | |
| `memory_resident_bytes` | | |

Loki dividido en componentes (simple scalable, microservicios) expone
estas familias donde corresponde: líneas rechazadas en el distribuidor,
flush y WAL en el ingester. Añade los componentes que quieras vigilar; una
familia que un componente no tiene simplemente no produce ninguna serie.

Las [reglas integradas](../alerting/rules.md#log-and-metrics-servers) que
se aplican: Loki not ready, Loki refusing log lines, Loki cannot flush to
storage, Loki write-ahead log disk full, Loki request errors, además de
Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Loki. Indica en una frase qué va mal (disponibilidad,
líneas rechazadas en las últimas 24 horas, chunks que fallaron al
volcarse, un disco WAL lleno, miembros del ring no saludables, errores de
servidor), luego las líneas y el volumen recibidos por segundo, los
streams en memoria y el disco WAL, y después las líneas rechazadas en las
últimas 24 horas por motivo.

## Dejar que DumbMonit lea el propio estado de Loki {#let-dumbmonit-read-lokis-own-health}

1. Nada que instalar: Loki publica /ready y /metrics en su puerto HTTP
   (3100 por defecto). Comprueba desde el host de DumbMonit que ambos
   respondan.

    ```
    curl http://loki.lan:3100/ready
    curl -s http://loki.lan:3100/metrics | grep loki_build_info
    ```

2. Loki no tiene inicio de sesión propio. Si está detrás de un proxy
   inverso que pide un nombre de usuario y contraseña, elige Username /
   password abajo; para un token bearer, elige Bearer token. No se
   necesita ningún tenant: /ready y /metrics no son por tenant.

3. En DumbMonit, introduce la dirección de Loki, por ejemplo «loki.lan», o
   «https://logs.lan/loki» detrás de un proxy inverso. Con Loki dividido
   en componentes (simple scalable o microservicios), añade al menos la
   vía de escritura, donde se cuentan las líneas rechazadas y los fallos
   de volcado.

4. DumbMonit solo lee /ready y /metrics. Nunca ejecuta una consulta LogQL
   y nunca lee una línea de log.

!!! warning
    Las líneas rechazadas son el fallo a vigilar: cuando Loki rechaza una
    línea (demasiado antigua, por encima del límite de tasa de ingestión o
    de stream, línea demasiado larga), el emisor recibe un error y
    normalmente descarta el lote. Nada se ve en Grafana salvo un hueco en
    los logs.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Sin autenticación | Lo por defecto: el propio Loki no tiene inicio de sesión. |
| Nombre de usuario / contraseña | La autenticación básica del proxy inverso frente a Loki. |
| Token bearer | Un token que tu proxy inverso espera en `Authorization: Bearer`. |

Dirección: un nombre de host o IP (`loki.lan`), `host:puerto`, o una URL
completa con un prefijo de ruta (`https://logs.lan/loki`). Opciones:
protocolo (HTTP por defecto), puerto (3100), comprobación de certificado y
tiempo de espera de la petición (10 s).
