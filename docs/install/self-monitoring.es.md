# Vigilar DumbMonit mismo {#watching-dumbmonit-itself}

¿Quién vigila al vigilante? DumbMonit responde en dos partes:

- un **latido saliente**, para lo que no puede ver desde dentro: el host caído,
  el contenedor detenido, el motor atascado;
- **reglas integradas** sobre su propia salud, para lo que sí puede ver: el
  volumen de datos llenándose, la base de datos de métricas reiniciándose en
  bucle, sondeos que se estancan, escrituras que fallan, la base de datos
  creciendo de forma anormal.

Ambos están en **Settings → System → DumbMonit itself**, que también muestra el
estado del motor ahora mismo.

## El latido saliente {#the-outgoing-heartbeat}

DumbMonit llama a una URL tuya a intervalos regulares (60 segundos por defecto),
**solo mientras el motor está sano**:

- el planificador funciona (completó una ronda en los dos últimos minutos);
- la base de datos SQLite responde;
- VictoriaMetrics responde.

Cuando uno de ellos falla, no se envía nada. Un servicio de «dead man's switch»
espera estas llamadas y te avisa cuando cesan, ya sea por todo el host, el
contenedor o un solo componente.

### Configurarlo {#setting-it-up}

1. Crea una comprobación en el servicio que prefieras y copia su URL de ping:
   - **Healthchecks.io** (o un Healthchecks autoalojado): una comprobación con un
     periodo igual al intervalo, y un tiempo de gracia algo mayor
     (`https://hc-ping.com/<uuid>`);
   - **Uptime Kuma**: un monitor de tipo *Push*
     (`https://kuma.example.com/api/push/<token>?status=up&msg=OK&ping=`);
   - **Better Stack**: un heartbeat (`https://uptime.betterstack.com/api/v1/heartbeat/<token>`);
   - cualquier servicio que acepte un `GET` simple y avise cuando deja de
     llegar (Cronitor, Dead Man's Snitch…).
2. En **Settings → System → DumbMonit itself**, pega la URL, elige el intervalo
   (de 30 a 3600 segundos), activa **Send a heartbeat** y guarda.
3. Pulsa **Test**: sale una llamada de inmediato y se muestra el resultado. El
   servicio debería marcar la comprobación como activa.

La llamada es un `GET` con un tiempo de espera de 10 segundos. Sigue la misma
protección que los canales de notificación: nunca se envía a la propia interfaz
web de DumbMonit ni a su VictoriaMetrics.

!!! note "ntfy"
    ntfy entrega cada mensaje que recibe, así que apuntar el latido a un tema de
    ntfy te envía una notificación cada minuto en lugar de cuando las llamadas
    cesan. Usa un servicio de dead man's switch y deja ntfy como canal para las
    propias alertas.

### Señal de fallo {#failure-signal}

Por defecto, un motor enfermo simplemente deja de llamar, y el servicio avisa
cuando pasa su tiempo de gracia. Activa **Signal failures** para llamar en su
lugar a la URL de fallo, que el servicio notifica al instante:

| Servicio | URL de fallo |
|---|---|
| Healthchecks.io, Better Stack | `/fail` añadido a la ruta: `https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | `status=up` sustituido por `status=down` en la consulta |

Déjalo desactivado para cualquier otro servicio: una URL desconocida con `/fail`
al final respondería muy probablemente 404, lo que se parece a que no se envía
nada.

### La URL es un secreto {#the-url-is-a-secret}

Una URL de ping suele contener su propio token: quien la conozca puede hacer que
tu comprobación parezca sana. DumbMonit la guarda cifrada con el secreto de la
instancia, nunca la devuelve por la API (solo se muestra su host) y la deja fuera
de los registros y de los mensajes de error. Para cambiarla, escribe la nueva;
para conservarla, deja el campo vacío. **Remove the URL** la olvida y desactiva
el latido.

El ajuste no forma parte del paquete de configuración cifrado: vuelve a
configurarlo tras restaurar en una instancia nueva.

## Reglas integradas {#built-in-rules}

Cada minuto DumbMonit escribe en VictoriaMetrics unas pocas mediciones sobre sí
mismo, y las reglas integradas del grupo *DumbMonit itself* las leen. Se
comportan como cualquier otra regla: mismos canales, mismos silencios, mismos
umbrales por regla en **Alerts → Rules**.

| Regla | Se activa cuando | Por defecto |
|---|---|---|
| DumbMonit data volume almost full | Menos del 10 % libre en el volumen que contiene `/data` | 15 min, warning |
| DumbMonit metrics database restarting in a loop | El VictoriaMetrics integrado se reinició 3 veces o más en 30 minutos | 1 min, critical |
| DumbMonit scheduler stalled | Ninguna ronda de planificación en más de dos minutos | 5 min, critical |
| DumbMonit polls running late | Los monitores siguen atrasados tras cada ronda durante 15 minutos | 15 min, warning |
| DumbMonit metric writes failing | Lotes rechazados por VictoriaMetrics, o muestras descartadas, en los últimos 15 minutos | 5 min, warning |
| DumbMonit database growing fast | `dumbmonit.db` más que duplicó su tamaño en un día y supera los 100 MB | 1 h, warning |
| DumbMonit backup did not run | Ninguna copia de seguridad local correcta en dos días ([detalles](backup.md#scheduled-local-backups)) | 1 h, warning |

Estas reglas necesitan VictoriaMetrics para evaluarse. Cuando el propio
VictoriaMetrics está caído de forma definitiva, o lo está todo el host, ninguna
regla puede activarse: ese es el trabajo del latido.

### Mediciones {#measurements}

| Métrica | Significado |
|---|---|
| `dumbmonit_instance_data_free_bytes` | Espacio libre en el volumen que contiene `/data`, para un usuario sin privilegios. |
| `dumbmonit_instance_data_size_bytes` | Tamaño de ese volumen. |
| `dumbmonit_instance_database_bytes` | Tamaño de `dumbmonit.db` y de su registro de escritura anticipada. |
| `dumbmonit_instance_scheduler_lag_seconds` | Segundos desde que el planificador completó una ronda por última vez (normalmente menos de 2). |
| `dumbmonit_instance_scheduler_backlog` | Monitores que aún tocaba sondear tras la última ronda. |
| `dumbmonit_instance_sample_writes_failed_total` | Lotes que VictoriaMetrics rechazó o a los que no respondió, desde que arrancó el servidor. |
| `dumbmonit_instance_samples_dropped_total` | Muestras descartadas antes de llegar al búfer de escritura, desde que arrancó el servidor. |
| `dumbmonit_instance_vm_restarts_total` | Reinicios del VictoriaMetrics integrado desde que arrancó el servidor (solo integrado). |

No llevan etiqueta `target`: las alertas que generan no se asocian a un monitor.
Para un Prometheus propio, `GET /metrics` expone el mismo tipo de cifras, consulta
[Scraping DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## API {#api}

| Método | Ruta | |
|---|---|---|
| `GET` | `/api/self-monitoring` | Estado del motor, volumen de datos, tamaño de la base de datos, ajuste del latido y último latido. |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`. Omitir `url` la conserva, `""` la borra. |
| `POST` | `/api/self-monitoring/heartbeat/test` | Una llamada ahora, a `{ "url" }` o a la URL guardada. |

Las dos escrituras son para administradores (o un token con el ámbito `write`).
