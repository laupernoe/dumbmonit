# Proxmox Mail Gateway

Pasarela de correo: colas de Postfix y la antigüedad del mensaje más
antiguo en espera, el correo contado y filtrado hoy, tamaños de
cuarentena, la antigüedad de las bases de datos de firmas de antivirus y
antispam, servicios, certificados y estado de clúster.

!!! warning "Probado solo contra una instancia de prueba"

    Esta integración se construyó y verificó contra un Proxmox Mail
    Gateway 9 instalado para este propósito, no contra uno en
    producción. Cada llamada es de lectura, y las formas vienen de una
    pasarela de correo real, pero todavía nadie la ha ejecutado durante
    semanas en una pasarela en producción. Dinos qué falla.

## La página del monitor {#the-device-page}

Más allá de las gráficas genéricas, un monitor Proxmox Mail Gateway
muestra tres paneles, leídos de lo que guardó la sonda — abrir la
página nunca consulta a la propia pasarela:

* **Mail queues** — las cuatro colas de Postfix en el orden en que un
  administrador de correo las lee: *incoming* (recién aceptado),
  *active* (en entrega), *deferred* (entrega fallida, se reintentará) y
  *hold* (retenido por una regla). Cada una muestra cuántos mensajes
  contiene, cuántos dominios de destino, el más ocupado de esos
  dominios, y cuánto tiempo lleva esperando el mensaje más antiguo. Una
  cola que simplemente tiene correo dentro es tráfico normal; una cola
  que retiene un mensaje durante más de cuatro horas se marca *Stuck*.
* **Mail filtered today** — mensajes entrantes y salientes, bytes,
  spam, virus, rebotes, tiempo de procesamiento promedio, y lo que se
  rechazó antes de la entrega (greylisting, listas de bloqueo, SPF,
  pregreeting). Una barra por franja de la ventana reciente dibuja la
  curva de tráfico sin consultar la base de datos de series temporales.
  Luego las tres cuarentenas con su recuento de mensajes y uso de disco,
  y los virus detectados hoy por nombre.
* **Gateway** — primero los servicios detenidos, luego los miembros del
  clúster y su estado de sincronización, luego cada nodo: tiempo de
  actividad, CPU, memoria, disco, desfase de reloj, las bases de datos
  de firmas con su antigüedad y veredicto, certificados que caducan en
  menos de dos semanas, actualizaciones pendientes y el estado de la
  suscripción.

Solo recuentos, nunca contenido. DumbMonit lee cuántos mensajes hay en
cada cuarentena, nunca un asunto, un remitente, un destinatario o un
cuerpo. Los endpoints de estadísticas por dirección (`/statistics/sender`,
`/statistics/receiver`, `/statistics/contact`) no se llaman en absoluto.

## Hoy, no las últimas veinticuatro horas {#today-not-the-last-twenty-four-hours}

`GET /statistics/mail` agrega por **día local en la pasarela**: PMG
ignora el final de la ventana que pidas, solo `starttime` elige el día.
DumbMonit por lo tanto pide «ahora», lo que da los totales desde la
medianoche — exactamente lo que muestra el propio panel de PMG. Esas
series son gauges que vuelven a cero a medianoche, no contadores: no las
envuelvas en `rate()`. La curva reciente (`/statistics/recent`) lee en
su lugar la tabla en bruto y de verdad va rodando.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_pmg_`.

| Familia | Métricas | Etiquetas |
|---|---|---|
| Nodo | `node_cpu_percent`, `node_iowait_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_used_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_clock_offset_seconds` (positivo cuando la pasarela va por delante de DumbMonit), `node_insync`, `node_kernel_info`, `node_version_info` (la versión de API de *este* nodo — en un clúster, el que quedó atrás tras la última actualización se muestra aquí), `version_info` | `node` |
| Colas | `queue_messages`, `queue_domains`, `queue_oldest_age_seconds` | `queue` (`incoming`, `active`, `deferred`, `hold`) |
| Correo de hoy | `mail_count_in/out`, `mail_bytes_in/out`, `mail_spam_in/out`, `mail_virus_in/out`, `mail_bounces_in/out`, `mail_junk_in`, `mail_junk_out` (tus propios usuarios enviando basura: la primera señal de una cuenta comprometida), `mail_junk_percent` (ausente en un día sin correo entrante), `mail_greylisted`, `mail_spf_rejects`, `mail_rbl_rejects`, `mail_pregreet_rejects`, `mail_avg_processing_seconds` | |
| Caudal | `mail_rate_in_per_minute`, `mail_rate_out_per_minute` (de la última franja completa de la curva reciente) | |
| Puntuaciones de spam | `spam_score_messages` | `level` (`0` a `10`; `10` agrega todo lo que está por encima) |
| Virus | `virus_detections` (el top diez del día) | `virus` |
| Cuarentenas | `quarantine_messages`, `quarantine_bytes`, `quarantine_avg_spam_level` | `kind` (`spam`, `virus`, `attachment`) |
| Servicios | `service_running` (1 o 0; sin ninguna serie para una unidad no instalada) | `node`, `service` |
| Firmas | `signature_age_seconds`, `signature_count`, `signature_update_available` | `node`, `family` (`virus`, `spam`), `database` (`main`, `daily`, `bytecode`, o el canal de SpamAssassin) |
| Certificados | `certificate_expires_in_seconds` | `node`, `certificate` |
| Clúster | `cluster_nodes`, `cluster_node_insync`, `cluster_node_healthy` (todos ausentes en una pasarela independiente) | `node`, `role` |
| Suscripción | `subscription_active` (las etiquetas llevan el estado exacto) | `node`, `status`, `level` |
| Actualizaciones | `node_updates_pending`, `node_updates_security_pending` | `node` |
| Recolección | `up`, `scrape_errors`, `scrape_duration_seconds` | |

`queue_oldest_age_seconds` es un **límite inferior**. `qshape` reporta
rangos de antigüedad (menos de 5 min, 5–10 min, 10–20 min, y así,
duplicando), así que DumbMonit conserva el suelo del rango más alto
ocupado: un mensaje reportado como «al menos 20 minutos» puede tener 39
minutos. Errar por lo bajo significa que la alerta se activa más tarde
en lugar de antes, nunca sobre un mensaje que en realidad no está
retrasado.

`queue_domains` es exacto en una pasarela única. En un clúster las colas
por nodo se combinan y el recuento se toma sobre los dominios más
ocupados de cada nodo, limitado a veinte por nodo: más allá de eso
subcuenta. Deduplicar gana a sumar aquí, ya que dos nodos de un mismo
clúster reciben correo para los mismos dominios.

Una pasarela independiente responde `/config/cluster/status` con una
lista vacía. Eso es una pasarela sin clúster, no un clúster degradado:
no se publica ninguna serie y ninguna regla puede activarse.

Solo la base de datos **daily** de ClamAV se juzga por su antigüedad,
tanto por el panel como por la regla. `main` se reconstruye una o dos
veces al año y `bytecode` apenas más a menudo: juzgarlas con dos días
imprimiría «desactualizado» todo el año en una pasarela perfectamente al
día. Igualmente muestran su antigüedad, versión y recuento de firmas —
sin un veredicto. Del mismo modo, un canal de SpamAssassin que nunca se
ha descargado no tiene ninguna fecha en absoluto, y «nunca actualizado»
no es «desactualizado»: sin serie, sin veredicto, sin alerta.

ClamAV escribe su fecha de compilación en su propio formato dentro de la
cabecera `.cvd` (`16 Dec 2025 23-18 +0000`, con un guion donde todos los
demás ponen dos puntos) y PMG la pasa tal cual. Si ese formato cambia
alguna vez, la antigüedad simplemente está ausente en lugar de
equivocada por cincuenta años. La misma regla se aplica en todas partes:
un valor que la API no devuelve produce una métrica menos, nunca un cero
fingiendo ser una medición.

Las reglas integradas que se aplican: Monitor unreachable, Mail queue
growing, Mail stuck in the queue, Mail gateway service stopped, Virus
signatures out of date, Spam rules out of date, Quarantine filling up,
Mail gateway cluster degraded, Mail gateway certificate expiring, Mail
gateway updates pending. Las notificaciones nombran la cola, servicio,
base de datos o nodo afectado.

## Qué preparar en Proxmox Mail Gateway {#what-to-prepare-in-proxmox-mail-gateway}

Los pasos de abajo son los que muestra el aviso junto al formulario. El
principio: un usuario reservado para monitorización, con un rol de solo
lectura — nunca la cuenta con la que inicias sesión.

1. En la interfaz web: Configuration → User Management → Users → Add.
   Nombra la cuenta como sigue, dale una contraseña larga usada en
   ningún otro sitio, y marca Enabled.

    ```
    dumbmonit@pmg
    ```

2. Define su Role como `Audit`. Ese es el mínimo exacto de solo lectura
   para todo lo que lee DumbMonit: estado de nodo, servicios, colas de
   postfix, estadísticas de correo, recuentos de cuarentena, antigüedad
   de bases de datos de ClamAV y SpamAssassin, estado de clúster,
   certificados, suscripción y actualizaciones pendientes. `Audit` no
   puede cambiar nada, liberar nada de cuarentena ni leer ningún
   mensaje.

3. ¿Prefieres una shell? La misma cuenta en un comando, y luego define
   la contraseña.

    ```
    pmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment "DumbMonit monitoring"
    ```

4. En DumbMonit, introduce la dirección de la pasarela, por ejemplo
   «mail.lan» o «mail.lan:8006», y luego el nombre de usuario (con el
   reino incluido) y contraseña de esta cuenta.

5. Proxmox Mail Gateway no emite tokens de API, a diferencia de Proxmox
   VE y Proxmox Backup Server: el nombre de usuario y contraseña son la
   vía de acceso. DumbMonit abre una sesión de dos horas y la renueva,
   en lugar de iniciar sesión en cada medición.

!!! warning

    No reutilices la cuenta con la que inicias sesión: el rol Audit de
    arriba solo puede leer, y no puede liberar un mensaje en cuarentena
    ni cambiar una regla. Proxmox Mail Gateway también usa un
    certificado autofirmado por defecto: si la conexión se rechaza por
    ese motivo, marca «Accept an unverifiable certificate» en las
    opciones.

Dirección: `10.0.0.40`, `mail.lan`, `mail.lan:8006`, `[fd00::4]` o una
URL completa. El esquema `https` y el puerto 8006 se añaden si faltan.

## Opciones {#options}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `port` | API port | `8006` | Se usa si la dirección no da un puerto. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Proxmox Mail Gateway viene con un certificado autofirmado por defecto: activa esto si la conexión se rechaza por ese motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tiempo permitido para cada llamada a la API, de 1 a 120. Leer una cola ejecuta un comando de Postfix en la pasarela y puede tardar unos segundos. |
| `node` | Monitored node | *(vacío)* | Nombre del nodo del clúster a monitorizar. Vacío: cada nodo que lista la pasarela. |
| `recent_hours` | Traffic window (hours) | `12` | Cuánto retrocede la curva de tráfico, de 1 a 24. Los totales diarios siempre cubren el día actual. |
| `queues` | Watch the mail queues | `true` | Lee las colas incoming, active, deferred y hold y la antigüedad del mensaje más antiguo en cada una. |
| `quarantine` | Count the quarantines | `true` | Lee cuántos mensajes contienen las cuarentenas de spam y virus, y cuánto disco usan. Solo recuentos. |
| `attachment_quarantine` | Also count the attachment quarantine | `false` | Esa cuarentena no tiene llamada de recuento: hay que listarla para contarla. Desactivado por defecto; solo se conserva el número de entradas. |
| `signatures` | Watch the signature databases | `true` | Lee la antigüedad de las bases de datos de virus de ClamAV y de los canales de reglas de SpamAssassin. |
| `services` | Watch the services | `true` | Lee el estado de postfix, pmg-smtp-filter, pmgpolicy y las demás unidades de cada nodo. |
| `certificates` | Watch the certificates | `true` | Lee los certificados que sirve la interfaz web y avisa antes de que caduquen. |
| `updates` | Count pending updates | `true` | Lista los paquetes esperando una actualización en cada nodo. |
| `subscription` | Read the subscription | `true` | Lee el estado de suscripción de cada nodo. |

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| ClamAV `main` muestra una antigüedad enorme | Esperado: se reconstruye una o dos veces al año. Solo `daily` lleva un veredicto. |
| Error de certificado | Certificado autofirmado: instala uno de confianza (ACME está integrado en PMG) o activa `insecure_tls`. |
| Error de autenticación | Reino incorrecto en el nombre de usuario — debe leer `dumbmonit@pmg`, no `dumbmonit` — o la cuenta no está Enabled. Solo `GET /version` fallando condena toda la sonda; cualquier otra llamada que falle se cuenta en `scrape_errors`. |
| Permisos insuficientes | El rol de la cuenta está por debajo de `Audit`. `quser` solo ve su propio correo; `helpdesk` no puede leer el nodo ni las estadísticas. |
| Toda métrica `mail_*` está en cero | La pasarela genuinamente no ha filtrado nada desde medianoche. Los totales cubren el día local actual en la pasarela y se reinician a su medianoche, no a la tuya. |
| Ninguna serie `service_running` en absoluto | La opción `services` está desactivada, o el rol no puede listarlas. Una sola unidad que falta en la lista simplemente no está instalada en ese nodo (`chrony` en una máquina que usa `systemd-timesyncd`, por ejemplo) y deliberadamente no se reporta como detenida. |
| Sin `signature_age_seconds` para un canal de SpamAssassin | Ese canal nunca se ha actualizado, así que PMG no devuelve ninguna fecha. Ejecuta `sa-update` en la pasarela; un canal secundario configurado pero nunca descargado se queda sin fecha a propósito. |
| Ninguna base de datos de ClamAV en absoluto | ClamAV no está instalado o `freshclam` nunca ha completado una descarga. El endpoint responde con una lista vacía, que no es un error. |
| Sin métricas de clúster | La pasarela es independiente. `/config/cluster/status` responde una lista vacía, y ninguna regla de clúster puede activarse. |
| «Mail stuck in the queue» se activa tarde | `queue_oldest_age_seconds` es el suelo del rango de antigüedad más alto ocupado de `qshape`, así que subreporta por diseño. |
| Falta `quarantine_messages{kind="attachment"}` | La opción `attachment_quarantine` está desactivada: esa cuarentena no tiene endpoint de recuento, así que contarla significa listarla. |
