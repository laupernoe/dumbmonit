# La página de dispositivos {#the-devices-page}

![El rack: una placa frontal por dispositivo](../assets/screenshots/devices-light.png){ loading=lazy }

## El rack {#the-rack}

Cada dispositivo es una placa frontal de rack de 1U: un LED de estado, el
nombre, el sello del tipo, la dirección, cuándo se vio por última vez, y un
sparkline de una métrica representativa. Los hijos se apilan bajo su padre
y se atenúan cuando el padre es inaccesible.

| Estado | Significado |
|---|---|
| Reporting | La última sonda tuvo éxito, dentro de tres períodos. |
| Unreachable | El dispositivo no respondió (tiempo de espera agotado, conexión rechazada), o ninguna sonda en más de tres períodos (al menos 90 s). |
| Misconfigured | La última sonda falló de nuestro lado: credenciales erróneas, dirección u opción inválida, respuesta inesperada. Se muestra en el color de aviso con el error; no genera ninguna alerta. |
| Waiting | Añadido, aún no sondeado. |
| Disabled | En pausa: no se comprueba, no genera ninguna alerta. |

Filtros: una caja de búsqueda, un segmento de estado (All, Needs attention,
Reporting, Disabled) y un filtro por tipo. **Add a device** es la acción
principal.

## Un dispositivo {#a-device}

![Página de un dispositivo: estado, acciones, alertas, el panel específico del tipo, métricas](../assets/screenshots/device-light.png){ loading=lazy }

La página del dispositivo muestra su placa de estado, dirección, tipo,
perfil detectado, etiquetas, última sonda, y para los servicios el estado
del certificado. Acciones:

| Acción | Efecto |
|---|---|
| Probe now | Ejecuta una sonda de inmediato e informa de cuántas muestras y series produjo. Lo primero que probar cuando un dispositivo no muestra datos. |
| Edit | Abre el mismo formulario que al añadir, precargado. Dejar la credencial vacía conserva la guardada. |
| Silence 1 h / Silence until… | Una ventana de mantenimiento puntual en este dispositivo. |
| Delete | Pregunta «Delete for good?» en línea durante cinco segundos, luego elimina el dispositivo. Sus series permanecen en VictoriaMetrics hasta que expira la retención. |

Debajo: **Alerts on this device** — cada alerta activa ofrece *Ack*,
*Snooze* e *Ignore*, y una regla ignorada en este dispositivo se lista por
separado con **Stop ignoring** — luego **Metrics** sobre un rango de
tiempo (1 h, 6 h, 24 h, 7 días), una gráfica por nombre de métrica con una
línea por serie. Consulta [Deshacerse de una alerta](alerts.md#getting-rid-of-an-alert).

Los servicios (HTTP, TCP, DNS, ping, TLS) obtienen instrumentos en su
lugar: **Availability** sobre las últimas 24 horas o 7 días, **Response
time**, el número de **Checks**, y una barra de **History** de franjas (la
más antigua a la izquierda) que muestra cuándo estuvo caído el servicio.

## Políticas de contenedor {#container-policies}

En una máquina con agente y Docker, una franja **Docker** se sitúa bajo la
cabecera: número de contenedores, cuántos se ejecutan, actualizaciones
disponibles, y cuántos contenedores tienen una política. **Manage
containers** abre la lista; **Policies…** abre una tabla con dos
interruptores por contenedor y una fila «Apply to all»:

- **Restart if down** — el servidor reinicia el contenedor dentro del
  minuto siguiente a que se detenga (como máximo una vez cada diez
  minutos);
- **Auto-update in maintenance windows** — cuando el registro tiene una
  imagen más reciente para la misma etiqueta, el contenedor se recrea
  durante una ventana de mantenimiento programada (página Alerts →
  Scheduled) y se revierte si falla su healthcheck.

Cada interruptor se guarda por separado. Detalles y reglas de seguridad:
[Contenedores Docker](../devices/agent.md#docker-containers).

## Añadir y editar {#adding-and-editing}

Consulta [Añade tu primer dispositivo](../install/first-device.md) para el
formulario y los [tipos de dispositivo](../devices/index.md) para lo que
necesita cada tipo. Al editar, el formulario abre **More options** por sí
mismo si el dispositivo ya usa una.

## Eliminar {#deleting}

Eliminar un dispositivo lo quita de la configuración, limpia sus alertas en
el acto — no se notifica nada, el historial conserva una línea `device
removed or disabled` — y elimina sus series temporales de la
VictoriaMetrics integrada unos segundos después (best effort: un fallo solo
se registra, y las series entonces caducan con la retención de 12 meses).
Pausar un dispositivo limpia sus alertas de la misma forma; sus series
permanecen.
