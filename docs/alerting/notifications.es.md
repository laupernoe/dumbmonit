# Política de notificación {#notification-policy}

La promesa de DumbMonit es que una notificación vale la pena leerla. El
motor de alertas mantiene bajo el ruido antes de que una alerta pueda
hablar (supresión por dependencia, agrupación, ventanas de
mantenimiento); la política de notificación lo mantiene bajo después,
por canal. Nada necesita configurarse: los valores por defecto de abajo
se aplican de fábrica.

## El flujo, en una lista {#the-pipeline-in-one-list}

```
evaluar (histéresis, anulaciones por dispositivo)
  → deduplicar → suprimir por dependencia → silenciar → agrupar por dispositivo
    → retención de parpadeo → filtro de severidad del canal → filtro de enrutado del canal
      → resuelto on/off → enfriamiento → horas de silencio → ventana de lote
        → límite por hora → enviar (+ enlace profundo)
                                   ↘ nadie confirmó → canal de escalada
```

| Etapa | Dónde se define | Por defecto |
|---|---|---|
| Umbral de limpieza (histéresis) | Editor de regla → *Clear below / above* | ninguno (CPU: 85 %, disco: 88 % en las reglas distribuidas) |
| Anulaciones por dispositivo | Editor de regla → *Per-device overrides* | ninguna |
| Detección de parpadeo | Settings → Notifications & reports → Notification policy → More options | 4 cambios en 30 min, retención 30 min |
| Severidad mínima | Canal → Delivery options → *Send* | todo |
| Avisarme cuando se limpia | Canal → Delivery options | activado |
| Intervalo mínimo por alerta | Canal → Delivery options | ninguno |
| Horas de silencio | Canal → Delivery options | desactivado |
| Filtro de enrutado (etiquetas, tipo, regla) | Canal → Delivery options → *Only some alerts* | desactivado (el canal escucha todo) |
| Escalada si nadie confirma | Settings → Notifications & reports → Notification policy | desactivado |
| Ventana de lote | Settings → Notifications & reports → Notification policy | 60 s |
| Mensajes por canal por hora | Settings → Notifications & reports → Notification policy | 20 |
| URL pública (enlaces profundos) | Settings → Notifications & reports → Notification policy, o `DUMBMONIT_PUBLIC_URL` | vacía |

## Histéresis y anulaciones por dispositivo {#hysteresis-and-per-device-overrides}

Una regla que se activa por encima del 90 % y se limpia en el momento
en que el valor cae por debajo del 90 % se activará y limpiará toda la
tarde en una CPU oscilando entre 89–91 %. Dale un **umbral de
limpieza**: una vez activa, la alerta solo se resuelve cuando el valor
va por debajo de él (para reglas «se activa por encima») o por encima
(para reglas «se activa por debajo»). El campo de API es
`clear_threshold` en la regla; el servidor rechaza un umbral de
limpieza en el lado equivocado del umbral.

Una **anulación por dispositivo** cambia el umbral o umbral de
limpieza de un dispositivo, o desactiva la regla para ese dispositivo,
sin duplicar la regla:

```
PUT /api/alerts/rules/{id}/overrides/{target_id}
{ "threshold": 97, "clear_threshold": 95 }      # o { "enabled": false }
GET /api/alerts/rules/{id}/overrides
GET /api/alerts/overrides?target_id=7           # cada anulación de un dispositivo
DELETE /api/alerts/rules/{id}/overrides/{target_id}
```

Una anulación con todos los campos vacíos se elimina. Los mensajes
citan el umbral realmente aplicado.

## Detección de parpadeo {#flap-detection}

Sin importar la regla, una alerta cuya huella se activó o resolvió 4
veces en 30 minutos está *flapping*: sale un aviso («flapping: 4
changes in 30 min, notifications held for 30 min») y nada más sobre esa
alerta se envía hasta que termina la retención. Una resolución tras la
retención igual se anuncia. Ajusta o desactívalo en Alerts →
Notifications → Notification policy → More options (`flap_events`,
`flap_window_secs`, `flap_hold_secs`; `flap_events: 0` lo desactiva).

## Opciones de entrega por canal {#per-channel-delivery-options}

Cada canal decide qué escucha (`policy` en la carga del canal):

- `min_severity` — `info`, `warning` (Advisory y superior) o
  `critical` (solo Warning). Un canal de teléfono normalmente quiere
  `critical`, una sala de chat todo.
- `notify_resolved` — `false` para escuchar solo problemas.
- `min_interval_secs` — al menos este tiempo entre dos mensajes sobre
  la misma alerta (reactivaciones y recordatorios). Las resoluciones
  nunca se retrasan por esto, y una resolución solo se envía a un
  canal que escuchó la activación.
- `quiet_hours` — una ventana semanal en tu hora local, misma forma
  que una ventana de mantenimiento semanal. Durante las horas de
  silencio solo pasan las alertas `critical`; el resto espera y llega
  como un resumen cuando terminan, titulado «Quiet hours over — …».
  Una alerta que se activa y limpia durante las horas de silencio solo
  se menciona en ese resumen como «resolved during quiet hours».
- `matcher` — qué alertas quiere el canal, por etiqueta, tipo de
  dispositivo o regla. Vacío (lo por defecto, y lo que conserva cada
  canal existente) significa todo.

```
PUT /api/notify/channels/{id}
{ "name": "Phone", "kind": "ntfy", "settings": {…},
  "policy": { "min_severity": "critical", "min_interval_secs": 900,
              "quiet_hours": { "kind": "weekly", "days": [0,1,2,3,4],
                               "start_minute": 1320, "end_minute": 420,
                               "timezone": "Europe/Paris" } } }
```

Omitir `policy` en una actualización conserva el guardado;
`"quiet_hours": null` lo borra, y `"matcher": null` borra el filtro de
enrutado.

## Enrutado por etiqueta {#routing-by-tag}

Los dispositivos llevan etiquetas (`site=cellar`, `role=storage`) y un
tipo (`snmp`, `proxmox`, `synology`, `agent`…). El **filtro de
enrutado** de un canal indica qué alertas quiere:

```json
{ "policy": { "min_severity": "warning",
              "matcher": {
                "include": [ { "field": "tag", "key": "site", "value": "cellar" } ],
                "exclude": [ { "field": "tag", "key": "role", "value": "lab" } ] } } }
```

Tres campos, y solo igualdad exacta — sin expresiones regulares, sin
corchetes:

| `field` | Coincide con | Ejemplo |
|---|---|---|
| `tag` | Una etiqueta de la propia página del dispositivo | `{"field":"tag","key":"site","value":"cellar"}` |
| `kind` | El recolector que sondea el dispositivo | `{"field":"kind","value":"proxmox"}` |
| `rule` | El `uid` de la regla | `{"field":"rule","value":"disk_full"}` |

Cómo se leen las listas:

- condiciones en el **mismo** campo (la misma clave de etiqueta, o
  `kind`, o `rule`) son un **O** — `site=cellar` más `site=attic`
  significa cualquiera de los dos;
- condiciones en campos **distintos** son un **Y** — `site=cellar` más
  `role=storage` significa ambos;
- **`exclude` siempre gana**, incluso sobre un `include` que coincidió.

Así que el ejemplo de arriba se lee, y la UI lo repite exactamente con
estas palabras: *este canal recibe avisos y superiores de dispositivos
etiquetados site=cellar, excepto dispositivos etiquetados role=lab*.

Bajo el editor, una **vista previa** lista qué dispositivos tuyos
actuales selecciona el filtro. No es una aproximación: el servidor
ejecuta el mismo filtro que ejecuta el motor de alertas, reducido a la
parte que depende solo del dispositivo. Las condiciones de regla se
aplican a las alertas en lugar de a los dispositivos, así que la vista
previa las nombra por separado en lugar de pretender contarlas.

```
POST /api/notify/match-preview
{ "matcher": { "include": [ { "field": "tag", "key": "site", "value": "cellar" } ] } }

→ { "devices": [ { "id": 3, "name": "nas", "kind": "synology",
                   "tags": { "site": "cellar" }, "matched": true } ],
    "matched": 1, "total": 12, "rules": [], "excluded_rules": [] }
```

Un filtro se limita a doce condiciones. Más allá de eso, un segundo
canal es más claro que una condición más.

## Escalada: si nadie confirma {#escalation-if-nobody-acknowledges}

Un salto extra, y solo uno. Cuando la política define
`escalate_after_secs` y `escalate_channel`, una alerta que nadie ha
[confirmado](../using/alerts.md) dentro de ese plazo también se envía
— una vez — a ese canal:

```
PUT /api/notify/policy
{ "escalate_after_secs": 900, "escalate_channel": 4 }
```

- El plazo se cuenta **desde el momento en que el primer mensaje salió
  realmente**, no desde que se activó la alerta. Si las horas de
  silencio o un envío fallido se tragaron la primera notificación,
  todavía no hay nada que escalar y el salto espera.
- **Confirmar la alerta la detiene**, igual que detiene los
  recordatorios; también lo hace la limpieza de la alerta.
- El mensaje de escalada va **solo** al canal de escalada — los
  canales que ya escucharon la activación no se avisan dos veces.
- Cruza el propio umbral de severidad, filtro de enrutado y horas de
  silencio de ese canal: nombraste este canal para ser despertado, así
  que no se retiene en el momento en que importa.
- La línea lo indica: `🔴 Critical · Disk full — /data — 95 % … —
  still unacknowledged`.

`"escalate_channel": null` desactiva la escalada; un plazo de `0` hace
lo mismo. Ambos se necesitan para que el salto exista, y el servidor
rechaza un canal que no existe en lugar de escalar hacia la nada.

DumbMonit deliberadamente se detiene aquí: un salto, sin rotación, sin
calendario de quién está de guardia esta semana. Si necesitas eso,
apunta el canal de escalada a PagerDuty u Opsgenie, que lo hacen bien.

## Agrupamiento por lotes y el límite por hora {#batching-and-the-hourly-cap}

`GET/PUT /api/notify/policy` contiene los ajustes globales:

```json
{ "batch_window_secs": 60, "max_per_hour": 20,
  "flap_events": 4, "flap_window_secs": 1800, "flap_hold_secs": 1800,
  "public_url": "https://monit.example.lan",
  "escalate_after_secs": 0, "escalate_channel": null }
```

- **Batch window** — las alertas que se activan dentro de la ventana
  salen como *un* mensaje por canal, agrupadas por dispositivo: «3
  alerts on 2 devices» con una sección por dispositivo; las
  resoluciones viajan con ellas («2 resolved»). `0` envía en el
  siguiente ciclo. Un resumen lista como máximo 12 líneas, y luego
  «…and N more alerts».
- **Hourly cap** — una vez que un canal ha recibido `max_per_hour`
  mensajes en la última hora, las alertas adicionales esperan y llegan
  como un resumen cuando se libera una ranura. `0` elimina el límite.
- Los **canales on-call** (PagerDuty, Opsgenie) nunca se agrupan entre
  dispositivos: abren y cierran un incidente por dispositivo, así que
  reciben un mensaje por dispositivo sin esperar.

Los fallos de envío mantienen las líneas en cola; el siguiente ciclo
reintenta.

## Qué dice una línea {#what-a-line-says}

Cada alerta es una línea, la misma en correo, ntfy, chat y la variable
`{{message}}` de un webhook personalizado:

```
🔴 Critical · Disk full — /data — 95 % (threshold > 90 %), for 12 min
⚠️ Warning · VM or container stopped — win11-desktop (101), for 59 s
✅ Resolved · Service down — https://example.lan/
```

- El glifo va seguido de la palabra de estado (Critical / Warning /
  Info, Resolved, Flapping), así que la severidad es legible sin color
  ni emoji.
- Una regla que vigila varias series nombra la que se activó: la VM y
  su id, el punto de montaje, la interfaz, el grupo de copia, la URL
  de la sonda. Las reglas de todo el dispositivo (unreachable) no
  nombran más que el dispositivo.
- El valor y el umbral se muestran para reglas medidas; una regla
  todo-o-nada sobre una métrica 0/1 (con batería, invitado detenido)
  no repite «1 (threshold > 0)».

## Enlaces profundos {#deep-links}

Cada mensaje termina con un enlace al dispositivo
(`{public_url}/targets/{id}`) cuando se conoce una URL pública — el
`public_url` de la política, o `DUMBMONIT_PUBLIC_URL`. Los webhooks
personalizados también lo obtienen como la variable `{{link}}` a menos
que tengan su propio `base_url` definido.

Deliberadamente no hay ningún enlace «silence for 1 h» en los mensajes:
los servicios de chat obtienen cada enlace que muestran para construir
una vista previa, lo cual activaría el silencio en el momento en que se
publica el mensaje. Silencia desde la página del dispositivo o la
página Alerts en su lugar.

## Qué se tomó prestado de Pulse, y qué no {#what-was-borrowed-from-pulse-and-what-was-not}

El diseño sigue a [Pulse](https://github.com/rcourtman/Pulse) donde
encaja con un monitor de homelab: umbrales de activación/limpieza, una
capa de anulación por recurso, un enfriamiento de notificación, una
ventana de agrupación, un límite de tasa por hora, horas de silencio,
detección de parpadeo con enfriamiento, «notify on resolve», y
herramientas on-call mantenidas fuera de los resúmenes. No adoptado: el
período de observación de 24 horas de Pulse antes de notificar (la
regla de referencia de DumbMonit ya aprende en silencio; las reglas de
umbral deberían hablar desde el primer día) y snooze/confirmar desde el
mensaje (ver arriba). El enrutado basado en etiquetas por destino *sí*
se adoptó, como el filtro de enrutado de arriba — elegir canales en la
regla resultó ser el lugar equivocado para «todo lo del sótano va a
esta sala».
