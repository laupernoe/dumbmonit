# La página de resumen {#the-overview-page}

La página de inicio responde «¿todo va bien?» en aproximadamente un
segundo, y luego te deja profundizar según necesites.

![El resumen](../assets/screenshots/overview-light.png){ loading=lazy }

## El boletín {#the-bulletin}

La franja superior es un boletín meteorológico para tu red. Una frase
indica el cielo: «Clear skies.» cuando nada requiere tu atención, de lo
contrario un recuento de lo que sí la requiere («9 advisories, 1
unreachable.»). Debajo, unas placas cuentan los monitores que
reportan y los inaccesibles, con la hora de la última comprobación, y
tres indicadores: **Reporting**, **Needs you**, **Forecasts**.

La ventana meteorológica a la derecha sigue a la red: despejado, nublado,
cubierto o tormenta, según cuántas alertas estén activas y cuán severas
sean, con la paloma volando a través. Bajo `prefers-reduced-motion`, todo
permanece quieto.

La única acción principal de la página es **Add a device**.

## La guía de primer uso {#the-first-run-guide}

En una instancia nueva, la página bajo el boletín no está vacía: es una
guía de tres pasos — añade un monitor, conecta una forma de que te
avisen, comprueba que llega un mensaje. Cada paso es un clic, cada uno se
puede descartar por su cuenta, y **Skip the guide** quita los tres de una
vez. La elección se guarda en la instancia, así que un segundo navegador
no reinicia la guía. Consulta [Añade tu primer
monitor](../install/first-device.md#the-three-first-steps).

## Needs you {#needs-you}

La lista de lo que requiere tu atención ahora, lo más severo primero:
monitores inaccesibles, luego alertas activas por severidad. Cada
fila nombra la regla, el monitor, el valor actual, cuánto tiempo
lleva así, y si agrupa varias series («×4» para cuatro sistemas de
archivos en el mismo monitor). Cuatro acciones: **Ack** (confirmar
esta alerta durante 1 h, 4 h, 24 h o hasta que se resuelva — los
recordatorios se pausan, la resolución igual se notifica), **Snooze**
(ocultar solo esta alerta durante 1 h, 8 h, 1 día o hasta que se
resuelva), **Ignore** (que esta regla deje de alertar en este monitor
para siempre, de forma reversible), y **Open device**.

Las alertas confirmadas y aplazadas se mueven a su propio grupo más
tranquilo bajo la lista y salen del recuento de *Needs you*. Una alerta
ignorada desaparece por completo — no está «conocida», está desactivada
para ese monitor. Consulta [Deshacerse de una
alerta](alerts.md#getting-rid-of-an-alert).

## Resumen de monitores {#devices-summary}

Una línea: cuántos monitores, cuántos reportando, cuántos
inaccesibles, y un enlace a la página completa de Devices. El resumen
nunca lista monitores: el rack vive en [Devices](devices.md).

## Pronósticos y últimas 24 horas {#forecasts-and-last-24-hours}

**Forecasts** muestra lo predicho en lugar de lo observado: la anomalía
de referencia estacional, las extrapolaciones «lleno en cuatro días», la
estimación de llenado de PBS, y las reglas cuyo nombre anuncia una
predicción. Mientras una referencia está aprendiendo, sus entradas lo
indican.

**Last 24 hours** lista las transiciones activas del día, con un enlace
al historial completo en la página Alerts.

## Modo pared {#wall-mode}

`/wall` muestra solo el boletín, a pantalla completa, para un monitor de
sala: la frase, los recuentos y la lista *Needs you*, actualizados cada
20 segundos, con la pantalla mantenida despierta. ++esc++ o **Exit**
vuelve al resumen. Ábrelo desde la paleta de comandos («Wall mode») o
escribiendo la URL. También puede reproducir música iniciada desde tu
teléfono: consulta [Modo pared y música](wall.md).

![Modo pared](../assets/screenshots/wall-light.png){ loading=lazy }

## Instalarlo en un teléfono {#install-it-on-a-phone}

La interfaz incluye un manifiesto web, así que un teléfono puede añadirlo
a la pantalla de inicio: **Share → Add to Home Screen** en iOS,
**Install app** en el menú de Chrome en Android. Entonces se abre sin el
marco del navegador, en el tema al que esté configurado el teléfono.

Deliberadamente **no hay modo sin conexión ni service worker**. Una
pantalla de monitorización que muestra una página en caché es peor que
una que dice que no puede alcanzar el servidor: el punto entero es que lo
que lees está al día. Sin worker, un servidor actualizado también se
sirve de inmediato, sin ninguna compilación obsoleta que limpiar.

## Paleta de comandos {#command-palette}

++cmd+k++ en macOS, ++ctrl+k++ en el resto, abre la paleta de comandos:
un campo, resultados en tres grupos.

| Grupo | Entradas |
|---|---|
| Pages | Overview, Devices, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a device, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident, Toggle theme |
| Devices | Cada monitor, con su LED de estado; ++enter++ lo abre |

## Navegación {#navigation}

La barra superior tiene Overview, Monitors, Alerts (con el recuento de lo
que requiere tu atención) y Settings, el acceso rápido a la paleta y el
interruptor de tema. En un teléfono, se convierte en una barra de
pestañas inferior con esas cuatro pestañas y **More**, que contiene el
modo sala y la documentación.

- **Overview** — el boletín: si todo va bien, y qué requiere tu atención.
- **Monitors** — el rack; cada fuente que monitorizas. Consulta [Página de
  monitores](devices.md).
- **Alerts** — qué está activo, las ventanas de mantenimiento, el
  historial y las reglas. Consulta [Página de alertas](alerts.md).
- **Settings** — tu cuenta, usuarios e inicio de sesión único, tokens de
  agente y asistente, canales de notificación e informes, páginas de
  estado públicas, apariencia, acerca de. Consulta
  [Settings](settings.md). Nada en la interfaz transmite el estado solo
  por color: una placa o un LED siempre viene con una palabra (Reporting,
  Unreachable, Advisory, Warning, Suppressed by parent, Building up).
