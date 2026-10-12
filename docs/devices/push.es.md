# Heartbeat (push) {#heartbeat-push}

El fallo silencioso que nadie nota: la copia de seguridad nocturna que
dejó de ejecutarse tras una actualización, el cron de renovación de
certificado que perdió sus permisos, la automatización de Home Assistant
que se rompió con un renombrado. Nada se queja, y te enteras el día que
necesitas la copia de seguridad.

Un monitor de **heartbeat** (también llamado *dead man's switch*, o un
monitor *push* en Uptime Kuma) invierte la lógica: DumbMonit no contacta
nada. El trabajo llama a una URL secreta cada vez que se ejecuta, y es
la **ausencia** de una llamada la que genera la alerta.

## Cómo funciona {#how-it-works}

Cada dispositivo heartbeat obtiene una URL de la forma
`https://<tu DumbMonit>/api/push/<token>`. El token es un valor
aleatorio de 128 bits, guardado con hash para la búsqueda y cifrado para
mostrarlo: se muestra en la página del dispositivo, con un botón de
copiar, durante todo el tiempo que exista el dispositivo. Es
deliberadamente abierto — sin sesión, sin token de API, sin cabecera
`X-Requested-With` — porque una línea `curl` en un crontab no tiene nada
de eso. Tener el token solo te permite decir «el trabajo se ejecutó».

En cada pasada del scheduler (el intervalo del dispositivo, 60 s por
defecto) el recolector lee la hora de la última llamada y escribe:

| Situación | `probe_success` | Efecto |
|---|---|---|
| Ninguna llamada recibida todavía | *(nada escrito)* | El dispositivo muestra **Waiting**. Sin alerta: el trabajo aún no está en su sitio. |
| Última llamada dentro del intervalo esperado + gracia | `1` | **Reporting**. |
| Última llamada más antigua que el intervalo esperado + gracia | `0`, `reason="missed"` | **Down**; la regla «Heartbeat missed» se activa tras dos minutos. |
| La última llamada dijo `status=down` | `0`, `reason="reported_down"` | **Down** de inmediato, sea lo que sea que diga el reloj. |

Una llamada también reevalúa el dispositivo de inmediato, así que un
trabajo que se reanuda limpia su alerta sin esperar a la siguiente
pasada del scheduler.

El endpoint responde `204` con un cuerpo vacío cuando la llamada se
registra, `404` para un token desconocido o regenerado, `400` para un
`status` que no es ni `up` ni `down`, y `429` (con `Retry-After`) más
allá de sesenta llamadas por minuto para un token. Se aceptan tanto
`GET` como `POST`; `HEAD` también funciona.

### Parámetros de consulta {#query-parameters}

| Parámetro | Significado |
|---|---|
| `status` | `up` (por defecto) o `down`: el trabajo reporta un fallo él mismo. |
| `msg` | Una palabra libre de explicación, conservada (truncada a 500 caracteres) y mostrada en la página del dispositivo. |

Estos son los parámetros del monitor push de Uptime Kuma: un script
escrito para él funciona sin cambios.

## Configuración {#setup}

Los pasos de abajo son los que muestra el aviso junto al formulario.

1. En la dirección, escribe una etiqueta corta para el trabajo
   («nightly-backup», «certbot-renew»): nada se contacta, la etiqueta
   solo tiene que ser única entre tus heartbeats.
2. Define el intervalo esperado según la programación del trabajo
   («24h» para un trabajo nocturno, «1h» para uno por hora) y, si la
   programación se desvía, un período de gracia más amplio. Guarda el
   dispositivo.
3. La página del dispositivo muestra la URL a llamar. Añádela al final
   del trabajo, para que solo se llame cuando el trabajo tuvo éxito:

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. Un trabajo que puede saber cuándo falló puede decirlo en lugar de
   quedarse en silencio: añade ?status=down&msg=… a la URL, y la alerta
   se activa de inmediato.
5. Hasta que llega la primera llamada el dispositivo muestra «Waiting»
   y nada se alerta. ¿URL perdida o filtrada? «Regenerate» en la página
   del dispositivo emite una nueva; la antigua deja de responder de
   inmediato.

!!! warning
    Llama a la URL al final del trabajo, después de la parte que
    importa. Una llamada colocada al principio reportaría un éxito
    incluso cuando la propia copia de seguridad falló.

Credenciales: ninguna.

### Ejemplos {#examples}

Una línea de crontab, cada noche a las 03:00, que solo avisa cuando la
copia de seguridad tuvo éxito:

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Un script que reporta su propio fallo, con el código de salida como
mensaje:

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
if /usr/local/bin/backup.sh; then
    curl -fsS -m 10 --retry 3 "$URL" > /dev/null
else
    curl -fsS -m 10 --retry 3 "$URL?status=down&msg=backup+exit+$?" > /dev/null
fi
```

Home Assistant, un `rest_command` llamado al final de una automatización:

```yaml
rest_command:
  dumbmonit_heartbeat:
    url: "https://monit.example.com/api/push/<token>"
    method: post
```

Windows Task Scheduler (PowerShell):

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "https://monit.example.com/api/push/<token>" | Out-Null
```

### Opciones {#options}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `expected_interval` | Expected interval | `24h` | Con qué frecuencia se supone que el trabajo llama: 30m, 1h, 6h, 24h, 7d (o un número de segundos). Una llamada perdida se declara una vez transcurridos ese intervalo más el período de gracia. |
| `grace` | Grace period | `10%` | Tiempo extra tolerado tras el intervalo esperado antes de que el heartbeat cuente como perdido: un porcentaje del intervalo (10%) o una duración fija (15m). Nunca menos de un minuto. |

Las duraciones aceptan sufijos `s`, `m`, `h`, `d`, `w` y se pueden
combinar (`1h30m`); un número solo es un número de segundos. El
intervalo esperado debe estar entre 10 segundos y un año.

## Métricas {#metrics}

Todas etiquetadas `probe="push"` además de `target`, `host` y `tag_*`,
todas con el prefijo `dumbmonit_`.

| Métrica | Significado |
|---|---|
| `probe_success` | 1 si la última llamada llegó a tiempo y no reportó un fallo, 0 en caso contrario. Ausente hasta la primera llamada. |
| `probe_failure_info` | Presencia (1) con una etiqueta `reason`: `missed` o `reported_down`. |
| `push_last_seen_seconds` | Antigüedad de la última llamada. |
| `push_received_total` | Llamadas recibidas desde que se creó el token (contador). |

Disponibilidad a lo largo de treinta días, como para cualquier monitor
de servicio: `avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Regla integrada que se aplica: **Heartbeat missed** (advisory, 2
minutos). Las reglas «Service down» y «Service flapping» omiten los
heartbeats, y «Device unreachable» nunca se activa para ellos: el
recolector siempre se ejecuta, solo cambia su veredicto.

## API {#api}

| Ruta | Sesión | Propósito |
|---|---|---|
| `GET` o `POST /api/push/<token>` | ninguna | La propia llamada. `204`, `404`, `400` o `429`. |
| `GET /api/targets/<id>/push` | requerida | El monitor: `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`. Se crea en la primera lectura si falta. |
| `POST /api/targets/<id>/push/regenerate` | admin | Token nuevo; la URL anterior responde `404` a partir de entonces. El contador y la última llamada se conservan. |
