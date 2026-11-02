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

## Las cuatro cifras {#the-figures}

Bajo el banner hay cuatro cifras: **Monitores activos** (sobre el total, con los recuentos de caídos y en pausa), **Disponibilidad 30 d**, **Respuesta p95** y **Alertas abiertas** (críticas y de aviso). El intervalo de tiempo de la cabecera gobierna el p95 y el río.

## Lo que requiere atención {#what-needs-attention}

La lista junto al río solo contiene lo que está roto ahora, lo peor primero, cada línea con su acción rápida: reconocer o posponer una alerta, o silenciar un monitor (la pequeña cruz). Sin nada roto, la paloma duerme y la tarjeta lo dice. Al pie, la **próxima semana** en una línea: lo que caduca en los próximos siete días. **Ver todo** abre las [Alertas](alerts.md).

## El río de salud {#the-health-river}

Una banda por monitor, una celda por tramo del intervalo, los peores monitores arriba. El resumen muestra los ocho primeros; **Ver todos los monitores** abre la pestaña [Lista](devices.md) de Monitores. Un estado siempre se escribe junto a una forma (círculo activo, rombo degradado, cuadrado caído), nunca solo con color. En un teléfono el río se abre con un toque.

## Dos gráficos {#charts}

Más abajo: el **tiempo de respuesta** de toda la flota en el intervalo (p95 y mediana, con los monitores más lentos) y las **alertas de los últimos siete días** (disparadas y resueltas).

## Modo pared {#wall-mode}

`/wall` muestra un tablero para un monitor de sala: el veredicto, el reloj, las
cinco cifras, el río de salud, las alertas abiertas y quién está de guardia.
Ábrelo con el icono **Wall mode** al pie del menú lateral o con la paleta de
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
| Pages | Resumen, Monitores, Mapa, Alertas, Automatización, Notificaciones, Explorador de métricas, Seguridad, Páginas de estado, Informes, Registros, Ajustes, Modo pared, Documentación |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Cada monitor, con su LED de estado; ++enter++ lo abre |

## Navegación {#navigation}

Un **menú** estrecho a la izquierda lista cuatro destinos: **Resumen**, **Monitores**, **Alertas** (con el recuento de lo que requiere tu atención) y **Estado**. Al pie están **Ajustes**, el **modo pared**, la documentación, GitHub y tu cuenta. Lo que antes era una página propia es ahora una pestaña de estas entradas. La cabecera indica dónde estás, con el interruptor **EN VIVO** que pausa las actualizaciones de la propia consola, el intervalo de tiempo, la paleta y **Añadir un monitor**. En un teléfono, el menú pasa a ser una barra de pestañas inferior con lo esencial y **Más**.

- **Resumen**: si todo va bien y qué requiere tu atención.
- **Monitores**: cada fuente que monitorizas, en cuatro pestañas: **Lista**, **Mapa**, **Registros** y **Seguridad**. Consulta [Monitores](../monitors/index.md).
- **Alertas**: cinco pestañas: **Ahora**, **Historial**, **Reglas**, **Automatización** y **Guardias**. Consulta la [página de alertas](alerts.md).
- **Estado**: **Páginas** (páginas de estado públicas) e **Informes**. Consulta [Estado](../status/index.md).
- **Ajustes**: tu cuenta, equipo y acceso, supervisión, notificaciones, la pared y el sistema. Consulta [Ajustes](../settings/index.md).

La interfaz es oscura y nada en ella transmite el estado solo por color: un
indicador siempre viene con una palabra (Reporting, Unreachable, Advisory,
Warning, Suppressed by parent, Building up).
