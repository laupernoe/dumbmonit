# Página de alertas {#alerts-page}

![La página de alertas: qué ocurre ahora, el mantenimiento, el historial y las reglas](../assets/screenshots/alerts-light.png){ loading=lazy }

Todo sobre las alertas en una página, detrás de cuatro pestañas. La
pestaña está en la URL (`/alerts#rules`), así que una vista se puede
enlazar. **Schedule maintenance**, arriba a la derecha, abre el
formulario para una ventana desde cualquier pestaña.

| Pestaña | Qué contiene |
|---|---|
| **Now** | La lista en vivo de problemas, una línea cada uno. Consulta [La lista Now](#the-now-list). El distintivo en la pestaña, y en Alerts en la barra superior, es el recuento de lo que aún requiere tu atención. |
| **Maintenance** | Ventanas de mantenimiento, *Active now* o *Scheduled*, puntuales, semanales o mensuales, cada una con su próxima ocurrencia (`#scheduled`). Consulta [Ventanas de mantenimiento](../alerting/maintenance.md). |
| **History** | Las últimas 200 transiciones, cada una nombrando el monitor, la regla y lo que ocurrió; las resueltas se pueden limpiar. |
| **Rules** | Cada regla con su severidad, una marca *Built-in* y, si algún monitor la ignora, *Ignored on N devices*; activar, editar en línea, eliminar las propias, crear una regla de umbral. Consulta [Reglas](../alerting/rules.md). |

Los canales de notificación y la política de notificación están en
**Settings → Notifications & reports** (consulta
[Notificaciones](#notifications)).

El mismo modelo de verdad alimenta el boletín de resumen y esta página,
así que ambos siempre coinciden en qué requiere tu atención.

## La lista Now {#the-now-list}

Una línea por problema: icono de estado, qué falla, qué monitor, la causa
en una línea, hace cuánto, y **Ack** y **Open** en la propia línea. Las
líneas se agrupan por severidad (Warning, Advisory, Info, Building up,
Suppressed by parent) o, con el conmutador **Severity | Monitor**, por
monitor. Los grupos se pliegan; *Suppressed*, *Snoozed* y *Dismissed*
(confirmadas) empiezan plegados. Una regla que salta en varias series del
mismo monitor es una sola línea («… and 7 more»).

Haz clic en una línea para desplegar sus detalles: cada serie afectada,
desde cuándo, quién la confirmó, por qué está suprimida, con **Open
monitor**, **Snooze** y **Never for this monitor**.

Seleccionar varias líneas es un modo. El botón **Select** (operadores y
administradores) muestra una casilla en cada línea y una barra flotante
abajo con **Select all**, **Acknowledge**, **Unacknowledge**, **Ignore
for this device** y **Done**. Escape sale del modo.

## Deshacerse de una alerta {#getting-rid-of-an-alert}

Una alerta activa no es binaria — «arreglarla» o «vivir con el ruido para
siempre». Cuatro acciones, para cuatro situaciones distintas, abiertas a
administradores y operadores (un visor ve el estado pero no puede
cambiarlo):

- **Acknowledge** («me encargo») es para una alerta en la que estás
  trabajando activamente: «lo sé, deja de recordármelo durante 4 h». El
  menú ofrece 1 h, 4 h, 24 h o *until resolved*, más una nota opcional
  para quien lea la tarjeta después de ti. La alerta sigue activa y
  evaluándose; solo se pausan sus recordatorios y escaladas — incluida la
  escalada a un segundo canal. Igual se te avisa cuando se resuelve, y la
  confirmación se borra en ese momento — una alerta que vuelve más tarde
  notifica de nuevo. La tarjeta se mueve de la lista principal a un grupo
  más tranquilo *Acknowledged*, dice *Acked by someone until a time*, y
  ofrece **Un-ack**.
- **Snooze** es para «ahora no, revísalo más tarde» sin afirmar que estás
  haciendo algo al respecto: 1 h, 8 h, 1 día o *until resolved*. A
  diferencia de Ack, funciona abriendo una ventana de mantenimiento
  limitada a las etiquetas exactas de esa alerta en concreto — una regla
  hermana en el mismo dispositivo sigue hablando. La tarjeta se mueve a
  un grupo tranquilo *Snoozed* mostrando el tiempo restante, y ofrece
  **Unsnooze**. Los recordatorios se pausan de la misma forma que con una
  confirmación.
- **Ignore** («no me avises de esto otra vez») es para una regla que
  nunca va a tener sentido en este dispositivo: desactiva esa regla solo
  para ese dispositivo (una anulación por dispositivo — consulta
  [Anulaciones por dispositivo](../alerting/rules.md#per-device-overrides))
  y la alerta desaparece para siempre, de inmediato, no solo se silencia.
  Es reversible: **Stop ignoring** en la página del dispositivo, o desde
  la propia regla en Alerts → Rules, donde dice *Ignored on N devices*.
- **Clear** es para el historial, no para una alerta activa: una vez que
  una transición se ha resuelto, *Clear* (o *Clear all resolved*) la
  quita de la vista de History por defecto. La fila permanece en la base
  de datos — nada se elimina — y *Show cleared* la vuelve a traer.

Las páginas de estado públicas ignoran las confirmaciones y los
aplazamientos: una alerta confirmada o aplazada sigue siendo una alerta
para el mundo exterior. Una regla ignorada, en cambio, nunca produjo
ninguna alerta en primer lugar.

## Notificaciones {#notifications}

Los canales y la política viven en **Settings → Notifications & reports**
(`/settings/notifications`), seguidos de los [Informes](reports.md). Dos
paneles:

**Notification channels.** La lista de canales, cada uno con su tipo, *Enabled* o
*Disabled*, cuándo envió algo por última vez, y su último error si lo
hay. **Add channel** abre un formulario construido a partir de la
descripción del tipo elegido que da el servidor: ajustes (visibles) y
secretos (solo escritura). **Send test** envía un mensaje de prueba y
muestra el resultado en línea. Las **Delivery options** de cada canal
contienen su severidad mínima, si escucha las resoluciones, un intervalo
mínimo por alerta, y sus **horas de silencio** (semanales, en tu zona
horaria: solo las alertas de nivel Warning pasan, el resto espera a un
resumen).

**Solo algunas alertas.** Las mismas Delivery options contienen el
**filtro de enrutado** del canal: qué alertas quiere, por etiqueta de
dispositivo (`site=cellar`), por tipo de dispositivo (`proxmox`), o por
regla. Las condiciones en el mismo campo se leen como *o*, campos
distintos como *y*, y una fila *Except* siempre gana. Bajo el editor, una
vista previa lista qué dispositivos tuyos selecciona el filtro ahora
mismo, y una frase te lo resume todo — *este canal recibe avisos y
superiores de dispositivos etiquetados site=cellar, excepto dispositivos
etiquetados role=lab*. Un canal sin filtro sigue recibiendo todo, que es
lo que hace cada canal existente.

Editar un canal y dejar el secreto vacío conserva el guardado. Los tipos
de canal y sus campos están documentados en
[Canales de notificación](../notifications.md).

Las reglas de alerta integradas notifican a todo canal activado; una
regla también se puede limitar a canales específicos en su editor.

**Notification policy.** Los controles globales que mantienen pocas las
notificaciones: la ventana de agrupamiento, el límite por canal por hora,
la detección de parpadeo bajo *More options*, la URL pública usada por
los enlaces «Open in DumbMonit», y la fila de **escalada** — *si nadie
confirma* dentro de un plazo, *también avisa* a otro canal, una vez. El
plazo cuenta desde el momento en que el primer mensaje salió realmente, y
confirmar la alerta (o su resolución) detiene el salto. Consulta
[Política de notificación](../alerting/notifications.md).

Los antiguos enlaces (`/alerts#notifications`,
`/alerts#notifications-policy`, `/settings#notifications`) redirigen
aquí.

## Operadores y visores {#operators-and-viewers}

Un **operador** gestiona las alertas sin tocar la configuración:
confirma, aplaza, ignora una regla para un dispositivo, programa o
elimina ventanas de mantenimiento y limpia el historial resuelto. Las
reglas, canales y la política de notificación se mantienen de solo
lectura para ellos, marcadas *Operator — config is admin only*. Ignorar es
la única anulación por dispositivo que un operador puede hacer; los
umbrales siguen siendo decisión de un administrador, y el *Ignore* o
*Stop ignoring* de un operador conserva cualquier umbral que un
administrador haya definido para ese dispositivo.

Un visor ve todas las pestañas pero ningún control: la página muestra
*Viewer — read only* donde un administrador encontraría los botones.
