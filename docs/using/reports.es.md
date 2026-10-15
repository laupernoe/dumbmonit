# Informes por correo {#email-reports}

Un informe es un correo breve que cuenta a tu equipo cómo se comportó la
infraestructura durante el último día, semana o mes, sin que nadie tenga que
abrir DumbMonit. Es semanal por defecto; diario y mensual están a un clic de
distancia. Los informes reutilizan un **canal de notificación de correo
(SMTP)**, así que configura uno primero (consulta
[Canales de notificación](../notifications.md)).

Abre **Settings > Reports** para configurarlos. Todo aquí es solo para
administradores, incluida la lectura, porque la página lista las
direcciones de los destinatarios.

## Qué contiene un informe {#what-a-report-contains}

| Sección | Contenido |
| --- | --- |
| **Titular** | Disponibilidad global del período, número de incidentes y la variación respecto al período anterior. |
| **Disponibilidad por grupo** | Una línea por grupo, luego por monitor (los 40 menos disponibles como máximo), calculada igual que en las [páginas de estado](status-pages.md): las sondas por su tasa de éxito, los hosts y monitores por la frecuencia con que reportaron. Los monitores sin historial se muestran como «sin datos» en lugar de como 100 %. |
| **Incidentes** | Cuándo empezó cada uno, cuánto duró (o que todavía está abierto) y en qué monitor. Hasta 25. Las alertas informativas se excluyen. |
| **Monitores menos estables** | Los cinco monitores con menor disponibilidad o más incidentes. |
| **Comparación** | Disponibilidad y número de incidentes frente al período de la misma duración justo anterior. |

El correo son tablas HTML sencillas con estilos en línea y sin imágenes
remotas, así que se ve igual en todas partes y no se obtiene nada al
abrirlo. Se envía también una versión en texto. Cada nombre de monitor y
mensaje se escapa.

## Ajustes {#settings}

| Ajuste | Qué hace |
| --- | --- |
| **Name** | Se muestra solo en la lista. |
| **Enabled** | Desactivado mantiene los ajustes sin enviar nada. |
| **Frequency** | Diaria, semanal (por defecto) o mensual. |
| **Day** | El día de la semana para los informes semanales, el día del mes (de 1 a 28) para los mensuales. |
| **Hour** y **Timezone** | Cuándo se envía el informe, en una zona horaria IANA como `Europe/Paris`. Se respeta el horario de verano. |
| **Recipients** | Hasta 20 direcciones, en formato libre: no tienen que ser usuarios de DumbMonit. Cada destinatario recibe su propio correo, así que nadie ve a los demás. |
| **Email channel** | Qué canal SMTP lo envía. Por defecto, el primero activado. Solo se usa el servidor de correo del canal: sus propios destinatarios se ignoran. |

El período es una ventana móvil que termina cuando se envía el informe: las
últimas 24 horas, 7 días o 30 días. El período anterior es la ventana de la
misma duración justo antes.

Haz clic en la **i** junto al título de la sección para un recordatorio del
contenido.

## Enviar una vista previa {#send-a-preview}

**Send a preview** envía por correo el informe actual a los destinatarios
guardados en ese momento, con `[Preview]` en el asunto. No cambia nada en
la programación: el siguiente informe se sigue enviando a su hora. Se puede
enviar una vista previa como máximo una vez cada 30 segundos por informe.
Guarda tus cambios primero, ya que la vista previa usa los destinatarios
guardados.

## Garantías de entrega {#delivery-guarantees}

- **Como máximo una vez.** La franja se reclama en la base de datos antes
  de construir el correo, así que un reinicio nunca envía el mismo informe
  dos veces. Cuando el envío falla, el error se guarda en el informe y se
  muestra en la lista, y la franja no se reintenta; la siguiente se envía
  con normalidad.
- **Sin recuperación retroactiva.** Si DumbMonit estaba caído a la hora
  programada, el informe se envía al volver solo si está dentro de las 3
  horas siguientes a la franja. Más tarde que eso, espera a la siguiente
  franja. Un informe que creas o reactivas nunca envía una franja que ya ha
  pasado.
- Pueden existir como máximo 10 informes.
- En modo demo no se envía nada.

Los informes se incluyen en las [copias de seguridad](../install/backup.md):
el canal de correo se busca por nombre al restaurar, y un informe
restaurado empieza de cero (su fecha de último envío no se conserva).

## API {#api}

Consulta la [API HTTP](../reference/api.md#email-reports).
