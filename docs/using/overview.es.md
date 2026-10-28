# La página de resumen

La página de inicio responde a «¿está todo bien?» en cerca de un segundo y luego
te deja profundizar cuando lo necesites.

## El banner de estado {#the-bulletin}

Una banda arriba muestra un cielo nocturno fijo: estrellas que titilan, una luna
creciente y una paloma que lo cruza sin posarse nunca. El cielo no cambia con el
tiempo; la frase a su pie dice cómo van las cosas, con palabras. «Los 146
monitores están activos.» cuando nada requiere tu atención; si no, «pve-02 está
caído.» o «pve-02 merece un vistazo.», seguido en voz más baja de lo que hizo el
sistema («3 invitados retenidos, alerta enviada.»). Toca la paloma y da una
voltereta. Con `prefers-reduced-motion`, la paloma sale del cielo y las estrellas
quedan quietas.

## La guía de primer uso {#the-first-run-guide}

En una instancia nueva, la página bajo el boletín no está vacía: es una
guía de tres pasos — añade un monitor, conecta una forma de que te
avisen, comprueba que llega un mensaje. Cada paso es un clic, cada uno se
puede descartar por su cuenta, y **Skip the guide** quita los tres de una
vez. La elección se guarda en la instancia, así que un segundo navegador
no reinicia la guía. Consulta [Añade tu primer
monitor](../install/first-device.md#the-three-first-steps).

## Las cinco cifras y el río de salud {#the-figures}

Bajo el banner, cinco cifras: **Monitors up** (sobre el total, con los caídos y
los pausados), **Availability 30 d**, **p95 response time**, **Open alerts**
(críticas y avisos) y **MTTR 30 d** (tiempo medio de resolución). El intervalo de
tiempo de la cabecera gobierna el p95 y el río.

El **río de salud** es una franja por monitor, una celda por tramo del intervalo,
con los peores monitores arriba. Un estado siempre se escribe junto a una forma
(círculo activo, rombo degradado, cuadrado caído), nunca solo con color.

## La tabla de monitores {#the-monitor-table}

La tabla lista cada monitor, el peor primero. Se filtra por estado, tipo y texto,
permite añadir columnas (dirección, carpeta) o quitar otras y se maneja con el
teclado: ++j++ / ++k++ mueven, ++enter++ selecciona, ++o++ abre.

El monitor seleccionado alimenta el **panel de contexto** de la derecha, como un
inspector: lo que requiere atención (con las acciones sobre alertas, consulta
[Página de alertas](alerts.md)), **Check now** y **Silence 1 h**, las previsiones
más cercanas y quién está de guardia. ++i++ lo muestra u oculta; en una pantalla
más estrecha se abre como cajón. La lista completa está en la
[página de monitores](devices.md).

## Modo pared {#wall-mode}

`/wall` muestra un tablero para un monitor de sala: el veredicto, el reloj, las
cinco cifras, el río de salud, las alertas abiertas y quién está de guardia.
Ábrelo con el icono **Wall mode** de la barra superior o con la paleta de
comandos; ++esc++ o **Exit** vuelve a la consola. Consulta
[Modo pared](wall.md).

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
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Cada monitor, con su LED de estado; ++enter++ lo abre |

## Navegación {#navigation}

Un **raíl de iconos** estrecho a la izquierda lista cada destino: Overview,
Monitors, Alerts (con el recuento de lo que requiere tu atención), Dashboards,
Explore, el mapa de topología, Security, Logs, Status pages, Reports y Settings.
Al pie están la documentación, GitHub y tu cuenta. La cabecera indica dónde
estás, con el interruptor **LIVE** que pausa las actualizaciones de la propia
consola, el intervalo de tiempo, la paleta, **Wall mode** y **New monitor**. En un
teléfono, el raíl pasa a ser una barra de pestañas inferior con lo esencial y
**More**.

- **Overview**: si todo va bien y qué requiere tu atención.
- **Monitors**: cada fuente que monitorizas. Consulta la
  [página de monitores](devices.md).
- **Alerts**: qué está activo, ventanas de mantenimiento, historial y reglas.
  Consulta la [página de alertas](alerts.md).
- **Settings**: tu cuenta, usuarios e inicio de sesión único, tokens, canales de
  notificación, apariencia, acerca de. Consulta [Settings](settings.md).

La interfaz es oscura y nada en ella transmite el estado solo por color: un
indicador siempre viene con una palabra (Reporting, Unreachable, Advisory,
Warning, Suppressed by parent, Building up).
