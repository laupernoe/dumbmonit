# Ventanas de mantenimiento {#maintenance-windows}

Una ventana de mantenimiento (un *silencio* en la API) detiene las
notificaciones de un dispositivo, o de todo, durante un período. La
evaluación continúa por debajo: cuando la ventana termina, una alerta que
todavía está activa se notifica una vez, en lugar de aparecer de la nada
después de dos horas.

## Dónde {#where}

- **Alerts → Scheduled maintenance**: la lista de ventanas, con *Active
  now* o *Scheduled*, y el formulario para añadir una.
- **Página del dispositivo → Silence 1 h / Silence until…**: una ventana
  puntual rápida para todo el dispositivo.
- ++ctrl+k++ → *Schedule maintenance*.

**Snooze**, en una alerta de la lista *Needs you* o en una página de
dispositivo, es una versión más pequeña del mismo mecanismo: una ventana
puntual limitada a las etiquetas exactas de esa alerta en concreto en
lugar de todo el dispositivo, así que una regla hermana sigue hablando.
Aparece aquí también, nombrada `Snooze · …`. Consulta [Deshacerse de una
alerta](../using/alerts.md#getting-rid-of-an-alert).

## Ventana puntual {#one-off-window}

| Campo | Significado |
|---|---|
| Name | Se muestra en la lista y en el historial. |
| Device | Déjalo vacío para cubrir todos los dispositivos. |
| Start, End | Horas absolutas. La ventana incluye el inicio y excluye el fin. |
| Comment | Opcional. Por qué existe la ventana. |

Las ventanas puntuales se purgan una vez han terminado.

## Ventana semanal {#weekly-window}

| Campo | Significado |
|---|---|
| Days | Qué días de la semana (de lunes a domingo). |
| From, To | Hora local en la zona horaria de la ventana. Si *To* es igual o anterior a *From*, la ventana se extiende más allá de la medianoche («23:00 → 01:00» pertenece al día en que empieza). |
| Time zone | Un nombre IANA como `Europe/Paris`. Precargado con la zona de tu navegador. |

## Ventana mensual {#monthly-window}

| Campo | Significado |
|---|---|
| On a weekday | *First*, *Second*, …, *Fifth* o *Last*, más el día — «el primer domingo», «el último viernes». |
| On a date | Uno o más días del mes, de 1 a 31, separados por comas («1, 15»). |
| From | Hora local de inicio, en la zona horaria de la ventana. |
| For | Cuánto dura la ventana, como tiempo real transcurrido. Una ventana mensual puede durar más de un día. |
| Time zone | Como arriba. |

Un mes que no contiene la ocurrencia se omite en lugar de aproximarse: no
hay un día 31 en febrero, ni un quinto domingo en abril de 2026, así que
la ventana simplemente no se abre ese mes. La lista muestra la próxima
ocurrencia que calculó el servidor, para que puedas comprobarla de un
vistazo.

DumbMonit no tiene expresiones cron, a propósito: una expresión cron
nombra *instantes*, y una ventana de mantenimiento necesita una
*duración*. Los días del mes y «el n-ésimo día de la semana» cubren los
calendarios que la gente realmente usa, y se leen de un vistazo.

## Zonas horarias y horario de verano {#time-zones-and-daylight-saving}

Una ventana recurrente se abre a su hora local de reloj de pared y luego
se ejecuta durante su duración en tiempo real. Ambas mitades importan:

- **La hora local no se desplaza.** «Cada domingo a las 02:00 en
  `Europe/Paris`» son las 01:00 UTC en invierno y las 00:00 UTC en verano
  — la ventana sigue el cambio de hora en lugar de desplazarse una hora
  dos veces al año.
- **La duración no se estira.** La noche en que el reloj retrocede, de
  02:00 a 04:00 en la pared duraría tres horas reales; la ventana sigue
  durando dos. La noche en que el reloj avanza y las 02:00 no existen, la
  ventana se abre en cuanto el reloj se reanuda.

Una ventana guardada antes de que existieran las zonas horarias conserva
su desfase UTC fijo y se comporta exactamente como antes. Vuelve a
guardarla con una zona para que siga los cambios de hora. Un nombre de
zona que el servidor no conoce se rechaza al guardar, con un ejemplo.

## Coincidencias {#matchers}

Una ventana también puede limitarse por coincidencias exactas de etiqueta
(`matchers` en la API): todas deben coincidir con las etiquetas de la
alerta, por ejemplo `tag_role = lab` y `__name__ = dumbmonit_up`. Una
ventana sin dispositivo y sin coincidencia silencia toda la instancia. Las
ventanas superpuestas no son un problema; la primera que cubre el momento
explica el silencio.

## Cómo se ve una alerta silenciada {#what-a-silenced-alert-looks-like}

La alerta conserva su fase y muestra `silenced: true`; el historial
registra la transición con `notified: false` y el motivo. Desactivar una
ventana (`enabled: false`) la mantiene en la lista sin efecto.

## En una página de estado pública {#on-a-public-status-page}

Un dispositivo cubierto por una ventana de mantenimiento se muestra como
**Maintenance** en cualquier [página de estado](../using/status-pages.md)
que lo liste, en lugar de rojo. Si nada más en la página está caído o
degradado, el estado general de la página también indica *Maintenance*.
Solo las ventanas que nombran un dispositivo — o cuyas coincidencias son
etiquetas a nivel de dispositivo — se muestran así; una ventana limitada a
una sola métrica sigue siendo un asunto de alertas.
