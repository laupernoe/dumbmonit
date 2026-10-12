# Métricas {#metrics}

DumbMonit guarda cada medición en VictoriaMetrics y la vuelve a leer
con MetricsQL, a través del [proxy de métricas](api.md#metrics) del
servidor. Esta página describe el nombrado, las etiquetas y la
retención. Las listas de métricas por tipo están en las [páginas de
dispositivo](../devices/index.md).

## Nombrado {#naming}

Cada métrica tiene el prefijo `dumbmonit_`, así que DumbMonit puede
compartir una instancia de VictoriaMetrics con otras herramientas sin
colisiones. Los nombres siguen `dumbmonit_<qué>_<unidad>`:

| Patrón | Ejemplos |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | lecturas de SAI y Synology |
| `…_info` | Series de presencia (valor 1) cuya información está en etiquetas: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Enumeraciones o booleanos: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Las familias específicas de fuente llevan su fuente como un segundo
prefijo: `dumbmonit_proxmox_*`, `dumbmonit_pbs_*`,
`dumbmonit_synology_*`, `dumbmonit_probe_*` (servicios). Los perfiles
SNMP y el agente usan nombres planos (`dumbmonit_if_octets_in`,
`dumbmonit_cpu_usage_percent`).

Los contadores (octetos y paquetes de interfaz, bytes de disco,
páginas impresas) se guardan **en bruto**, tal como los reporta el
dispositivo: aplica `rate()` en tus propias consultas para obtener un
caudal. La página del dispositivo grafica cada serie `dumbmonit_*` del
dispositivo tal como se guarda, así que un contador aparece como una
línea ascendente. Un reinicio se muestra como un reseteo del contador,
nunca como un pico falso.

## `dumbmonit_up`

Tras cada sonda exitosa, el servidor escribe `dumbmonit_up = 1` para el
dispositivo. No se escribe nada cuando una sonda falla: la serie
simplemente se detiene, y la regla «Device unreachable» mide la
antigüedad de la última muestra
(`time() - tlast_over_time(dumbmonit_up[7d])`).

Para los servicios (`http`, `tcp`, `dns`, `ping`, `tls`),
`dumbmonit_up = 1` significa «el monitor se ejecutó», y
`dumbmonit_probe_success` indica si el servicio está bien. Las dos
señales se mantienen separadas a propósito: un `probe_success` de 0
indica que el servicio está caído, una interrupción de `up` indica que
la monitorización está caída.

## `dumbmonit_security_score`

Cada 15 minutos el servidor calcula la [puntuación de
seguridad](../using/security-score.md) de cada dispositivo cuyo tipo
tiene comprobaciones de seguridad, y escribe `dumbmonit_security_score`
(0–100, ausente mientras nada se pueda puntuar) y
`dumbmonit_security_checks_failed`, ambas etiquetadas `target`.

## Etiquetas {#labels}

| Etiqueta | Definida por | Significado |
|---|---|---|
| `target` | servidor | El id del dispositivo, como en `/api/targets/{id}`. Cada regla y cada gráfica selecciona sobre ella. |
| `host` | servidor | El nombre del dispositivo. |
| `tag_<key>` | servidor | Cada etiqueta del dispositivo, incluidas las opciones de tipo (`tag_insecure_tls`, `tag_port`…). Nunca pongas un secreto en una etiqueta. |
| `probe` | servicios | `http`, `tcp`, `dns`, `ping` o `tls`. |
| etiquetas por serie | recolector | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… como se lista en cada página de dispositivo. |

Las etiquetas de identidad siempre ganan sobre las etiquetas de
recolector, y para los agentes se definen al recibirse: una máquina
no puede escribir en la serie de otra máquina.

!!! note "Prefijo antes de septiembre de 2026"
    Las series escritas por EzyMonit se nombran `ezymonit_…`. El
    prefijo cambió con el nombre del producto y nada lee las series
    antiguas: permanecen en VictoriaMetrics hasta que la retención las
    elimina, las gráficas y reglas vuelven a empezar desde la
    actualización, y las reglas personalizadas que todavía nombran
    `ezymonit_…` deben editarse.

## Retención y almacenamiento {#retention-and-storage}

La VictoriaMetrics integrada conserva 12 meses de datos
(`DUMBMONIT_VM_RETENTION`) bajo `/data/vm`, en el volumen
`dumbmonit-data`. El servidor agrupa sus escrituras y las vuelca cada 5
segundos (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Eliminar un dispositivo no
elimina sus series; caducan por su cuenta.

El historial de alertas se conserva 90 días y las referencias de
series que desaparecieron 60 días, ambos en SQLite (consulta la
[referencia de configuración](configuration.md)).

## Consultar {#querying}

- A través del servidor: `GET /api/metrics/query` y `GET
  /api/metrics/query_range`, con la cookie de sesión. Los resultados de
  rango están limitados a 2000 puntos por serie.
- Desde un Prometheus o Grafana existente: `GET /federate`, `GET
  /metrics` y la fuente de datos `/prometheus`, con un token de API —
  [abajo](#scraping-dumbmonit).
- Directamente, en desarrollo: la capa `docker-compose.dev.yml` publica
  la VictoriaMetrics integrada en `http://localhost:8428`, con su
  propia UI en `http://localhost:8428/vmui/`. En producción escucha
  solo en el loopback del contenedor; define
  `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` y publica el puerto tú mismo si
  quieres Grafana u otra herramienta sobre los mismos datos.

```
# Caudal de interfaz en bits por segundo
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Disponibilidad de cada servicio en 30 días
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Cada dispositivo que no ha reportado en 5 minutos
time() - tlast_over_time(dumbmonit_up[7d]) > 300
```

## Sondear DumbMonit {#scraping-dumbmonit}

Si ya ejecutas un Prometheus o un Grafana, no necesitas un segundo
stack. Tres rutas están fuera de `/api`, donde un scraper las busca:

| Ruta | Qué devuelve |
|---|---|
| `GET /metrics` | La salud de la propia instancia DumbMonit: scheduler, sondas, escrituras, alertas, notificaciones, agentes, base de datos. Unas pocas decenas de series, siempre el mismo número — nunca una por dispositivo. |
| `GET /federate` | Las mediciones, seleccionadas por `match[]`, exactamente como Prometheus federa otro Prometheus: el punto más reciente de cada serie seleccionada. |
| `GET /prometheus/api/v1/…` | La mitad de lectura de la API de Prometheus, retransmitida al almacén: lo que llama una fuente de datos de Grafana. |

Las tres toman `Authorization: Bearer dmt_…` con el alcance
**`read`** — un token creado en Settings → *API & assistants*, el
mismo tipo que usa el [servidor MCP](../using/assistant.md). No hay
ninguna cookie implicada, así que no se necesita ninguna cabecera
`X-Requested-With`; una cookie de sesión también se acepta, así que
puedes abrir cualquiera de ellas desde un navegador con sesión
iniciada. Un token se limita a 120 llamadas por minuto: suficiente
para un sondeo cada 15 segundos, y bastante para un dashboard de
Grafana de unas pocas decenas de paneles — dale a un dashboard ocupado
su propio token en lugar de compartir uno con un scraper.

### Prometheus

```yaml
# prometheus.yml — la propia salud de DumbMonit, luego las mediciones que recolecta.
scrape_configs:
  - job_name: dumbmonit
    metrics_path: /metrics
    scheme: http
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    static_configs:
      - targets: ['dumbmonit.lan:8080']

  - job_name: dumbmonit-federate
    metrics_path: /federate
    scheme: http
    honor_labels: true
    scrape_interval: 60s
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    params:
      'match[]':
        - '{__name__=~"dumbmonit_.*"}'
    static_configs:
      - targets: ['dumbmonit.lan:8080']
```

`honor_labels: true` conserva las etiquetas `target` y `host` que
define DumbMonit en lugar de sobrescribirlas con las propias del job
de sondeo. Pon el token en un archivo y usa
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`
si prefieres no tenerlo en la configuración.

Sin `match[]`, `/federate` selecciona `{__name__=~"dumbmonit_.*"}` —
cada serie que escribe DumbMonit, y nada que pertenezca a otra
herramienta que comparta la misma VictoriaMetrics. Limítalo para
mantener pequeña una federación: `{__name__=~"dumbmonit_.*",target="4"}`
para un dispositivo, `{__name__="dumbmonit_up"}` solo para
disponibilidad. Como máximo 10 selectores por petición, y una
respuesta de más de 8 MiB se rechaza con un mensaje que te dice que la
limites — divide una flota grande en varios jobs con selectores más
estrechos en lugar de uno que pida todo.
`max_lookback=15m` amplía la ventana en la que se busca el último
punto, para dispositivos sondeados con menos frecuencia que cada cinco
minutos.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana

Apunta Grafana a **DumbMonit**, no a la VictoriaMetrics que ejecuta: el
proceso hijo escucha en el loopback del contenedor, no tiene ninguna
autenticación propia, y expone las rutas que *eliminan* series con la
misma facilidad que las que las leen. DumbMonit retransmite la mitad
de lectura de la API de Prometheus bajo `/prometheus`, detrás del
mismo token, así que Grafana habla con una fuente de datos Prometheus
normal:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (Grafana añade
   `/api/v1/query` él mismo).
3. **HTTP headers**: añade uno, `Authorization` = `Bearer dmt_…`.
   Grafana lo guarda cifrado y nunca lo vuelve a mostrar.
4. **Save & test** — Grafana reporta que la fuente de datos funciona,
   y cada serie `dumbmonit_*` está entonces disponible en PromQL, con
   autocompletado de nombres de métrica y etiqueta.

Solo se retransmiten las rutas de lectura — `query`, `query_range`,
`series`, `labels`, valores de etiqueta, `metadata` y la versión que
pide Grafana al probar la conexión. Cualquier otra cosa,
`admin/tsdb/delete_series` entre las primeras, responde `404`: este
punto de entrada no puede cambiar ni borrar nada.

Si ya ejecutas un Prometheus, la otra vía funciona igual de bien:
Prometheus federa DumbMonit con el job de arriba, Grafana lee
Prometheus, y las consultas son idénticas.

### Métricas de instancia {#instance-metrics}

Todo lo que expone `GET /metrics`. Los valores se leen cuando sondeas;
los contadores `_total` son monótonos y se reinician solo cuando el
servidor se reinicia (`dumbmonit_uptime_seconds` te dice cuándo pasó
eso).

| Métrica | Tipo | Significado |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Siempre 1. La etiqueta `version` es la versión del servidor en ejecución, la misma que reporta `/api/health`. |
| `dumbmonit_uptime_seconds` | gauge | Segundos desde que arrancó el proceso del servidor. |
| `dumbmonit_scheduler_cycles_total` | counter | Ciclos de scheduler ejecutados. El bucle marca una vez por segundo, así que esto también sirve de señal de vida. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Duración del último ciclo. Normalmente por debajo de un milisegundo: el ciclo solo despacha, no espera a las sondas. |
| `dumbmonit_scheduler_backlog` | gauge | Dispositivos que todavía esperaban una sonda cuando terminó el último ciclo. Cero en una instancia sana; un valor distinto de cero persistente significa que `DUMBMONIT_MAX_CONCURRENT_PROBES` es demasiado bajo para el número de dispositivos y sus intervalos. |
| `dumbmonit_probes_total{kind}` | counter | Sondas ejecutadas, por tipo de dispositivo (`snmp`, `proxmox`, `http`…). Incluye las que un agente relay ejecutó en nombre del servidor. |
| `dumbmonit_probes_failed_total{kind}` | counter | Sondas que fallaron, por tipo — un dispositivo inaccesible o un error de configuración. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` es la proporción de fallo. |
| `dumbmonit_samples_written_total` | counter | Muestras aceptadas por VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | counter | Lotes que VictoriaMetrics rechazó o no respondió. El lote permanece en búfer y se reintenta; un valor en aumento con un `dumbmonit_samples_pending` en aumento significa que el almacén está caído o lleno. |
| `dumbmonit_samples_pending` | gauge | Muestras todavía en el búfer de escritura tras el último flush. Normalmente 0. |
| `dumbmonit_samples_dropped_total` | counter | Muestras descartadas antes de llegar siquiera al búfer de escritura, porque estaba lleno o cerrado. Nunca se reintentan — un valor en aumento durante una caída significa pérdida de datos, no solo retraso. |
| `dumbmonit_alerting_cycles_total` | counter | Ciclos de alertas ejecutados (uno cada `DUMBMONIT_ALERT_INTERVAL_SECS`, 30 s por defecto). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Duración del último ciclo de alertas, tiempo de consulta incluido. Acercarse al intervalo significa que las reglas consultan más de lo que puede servir el almacén. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Reglas evaluadas en el último ciclo. |
| `dumbmonit_alerting_rules_failed` | gauge | Reglas cuya consulta falló en el último ciclo. Sus alertas se congelan, no se resuelven. Cualquier cosa distinta de 0 merece revisar los registros. |
| `dumbmonit_alerts{phase}` | gauge | Alertas que rastrea el motor, por fase: `ok`, `pending` (condición verdadera, `for` no transcurrido), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Alertas ocultas porque un dispositivo padre está caído (la fase efectiva `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Alertas cubiertas por una ventana de mantenimiento. |
| `dumbmonit_alerts_learning` | gauge | Alertas de referencia todavía aprendiendo, y por tanto en silencio. |
| `dumbmonit_notifications_total{kind}` | counter | Notificaciones enviadas, por tipo de canal (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | counter | Notificaciones que el canal rechazó, por tipo. Se vuelven a encolar y reintentan; un valor en aumento constante es un webhook o cuenta SMTP rota. |
| `dumbmonit_agents` | gauge | Máquinas que han registrado un agente. |
| `dumbmonit_agents_stale` | gauge | Agentes que no han enviado nada en cinco minutos. |
| `dumbmonit_database_up` | gauge | 1 cuando SQLite responde — la misma comprobación que `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Tamaño de `dumbmonit.db` y su write-ahead log. No incluye `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1 cuando VictoriaMetrics responde — la misma comprobación que `/api/health`. 0 mientras el hijo integrado se reinicia. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1 cuando este servidor ejecuta VictoriaMetrics él mismo, 0 cuando `DUMBMONIT_VM_URL` apunta a una externa. |

Ninguna métrica de esta página lleva ninguna etiqueta por dispositivo:
el documento tiene el mismo tamaño para un dispositivo y para mil, y
ningún nombre de dispositivo se filtra a quien pueda sondearlo.

Dos alertas que vale la pena añadir de tu lado, una vez que se sondea
DumbMonit:

```
# La monitorización dejó de monitorizar
up{job="dumbmonit"} == 0

# Las mediciones se acumulan en lugar de guardarse
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Abrir las rutas sin un token {#opening-the-routes-without-a-token}

`DUMBMONIT_METRICS_PUBLIC=true` sirve `/metrics`, `/federate` y
`/prometheus/api/v1/…` a cualquiera que pueda alcanzar el puerto, para
quienes filtran el puerto ellos mismos — **cualquiera en esa red puede
entonces leer cada medición de cada dispositivo y el estado de la
instancia, así que solo defínelo cuando algo más ya restrinja quién
alcanza el puerto.**
