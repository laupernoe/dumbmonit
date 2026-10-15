# Home Assistant

El hogar inteligente detrás del panel: si Home Assistant está en
ejecución o arrancó en modo recuperación, entidades que están
no disponibles o desconocidas por dominio, baterías bajas, actualizaciones
pendientes (core, sistema operativo, add-ons, firmware de dispositivo) y
las reparaciones abiertas.

DumbMonit lee la API REST de Home Assistant con el token de acceso de
larga duración de un usuario dedicado **sin derechos de administrador**,
y la lista de reparaciones a través de la API WebSocket. Nunca llama a un
servicio, nunca dispara un evento y nunca cambia un estado.

El fallo que vale la pena vigilar no es «Home Assistant no responde» (una
[comprobación de sitio web](services.md#http) ya ve eso) sino **Home
Assistant responde y algo detrás dejó de funcionar**: una integración
cuyas entidades todas se volvieron no disponibles tras una actualización,
un stick Zigbee que se cayó del bus USB, un sensor de puerta cuya batería
llegó al 5 %, una reparación esperando desde hace semanas.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_homeassistant_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `info` | valor 1 | `version` |
| `running` | 1 cuando el estado del core es `RUNNING`, 0 mientras arranca o se detiene | |
| `recovery_mode` | 1 en modo recuperación (o modo seguro en versiones más antiguas): la configuración no se pudo cargar | |
| `entities` | entidades por dominio | `domain` |
| `entities_unavailable`, `entities_unknown` | entidades en ese estado, por dominio | `domain` |
| `entity_unavailable` | valor 1 por entidad no disponible, como máximo 50 | `entity`, `name`, `domain` |
| `batteries` | sensores de batería y sensores binarios de batería | |
| `batteries_low` | baterías por debajo del umbral, y sensores binarios de batería que están activados | |
| `battery_low` | una serie por batería baja, valor = su nivel en porcentaje (0 para un sensor binario), como máximo 50 | `entity`, `name` |
| `updates_available` | entidades `update.*` que están activadas | |
| `update_available` | valor 1 por actualización pendiente, como máximo 50 | `entity`, `name`, `installed`, `latest` |
| `repairs` | reparaciones abiertas (ni ignoradas ni descartadas) por severidad: `critical`, `error`, `warning` | `severity` |
| `repair` | valor 1 por reparación abierta, como máximo 50 | `issue`, `domain`, `severity` |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

Algunos dominios descansan en el estado `unknown` por naturaleza: un
botón nunca pulsado, un evento nunca recibido, un servicio de
notificación o voz. No se cuentan como desconocidos: `button`,
`input_button`, `event`, `scene`, `notify`, `tts`, `stt`,
`conversation`, `wake_word`, `ai_task`, `person`. La opción **Domains
left out** elimina dominios adicionales (por ejemplo `device_tracker` o
`media_player` para dispositivos que a menudo están apagados) de los
recuentos de no disponibles y desconocidos.

Las reparaciones no están en la API REST: DumbMonit abre la API
WebSocket, envía `repairs/list_issues` y la cierra. Un usuario sin
derechos de administrador puede listarlas. La opción **Read the
repairs** desactiva esto.

Las [reglas integradas](../alerting/rules.md#home-assistant) que se
aplican: Home Assistant in recovery mode, Home Assistant not running,
Home Assistant entities went unavailable, Home Assistant battery low,
Home Assistant update available, Home Assistant repair to address,
además de Monitor unreachable.

## Dispositivos {#devices}

Cada teléfono y tableta con sesión iniciada en la app complementaria (la
integración `mobile_app`) se publica bajo la familia genérica compartida
por toda integración de DumbMonit que rastrea dispositivos cliente —
`dumbmonit_client_device_*`, con prefijo sin `homeassistant`, documentada
por completo en [Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | Qué | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última actualización de la entidad `device_tracker` del dispositivo | `device`, `type` (`Mobile`), `os` (vacío), `user` (la persona a la que está vinculado, si la hay), `kind` |
| `stale_seconds` | Positivo una vez que un dispositivo ha superado `device_stale_days` (3 por defecto) sin una actualización; impulsa la regla [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (siempre `connection`: Home Assistant no tiene copia de seguridad que reportar) |

**Cómo se reconoce un dispositivo**: `/api/states` no indica qué
integración creó una entidad — eso necesita el registro de entidades, que
la API REST no sirve. El `device_tracker` de la app complementaria es el
único que define un atributo `battery_level`; las integraciones de
ubicación de terceros (Life360, OwnTracks, la integración de demostración
usada en pruebas) definen como máximo `battery`. Una integración de
terceros que por casualidad copiara ese nombre de atributo exacto se
confundiría con la app complementaria; a la inversa, un dispositivo de la
app complementaria con el reporte de batería desactivado falta en la
tabla en lugar de mostrarse incorrectamente. La cuenta del dispositivo,
cuando se conoce, viene de una entidad `person.*` cuyo atributo
`device_trackers` lo nombra.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Home Assistant. Indica primero qué va mal (modo
recuperación, reparaciones de severidad error, entidades no
disponibles), luego las baterías bajas por nivel, las actualizaciones
pendientes con sus versiones, las entidades por dominio, y la tabla de
dispositivos de la app complementaria con su última conexión.

## Cómo se validó {#how-it-was-validated}

Contra Home Assistant 2026.9.4, con el token de larga duración de un
usuario sin derechos de administrador: configuración, estados de entidad
de la integración de demostración y de sensores de plantilla (niveles de
batería, un sensor binario de batería, un sensor no disponible),
entidades de actualización y la lista de reparaciones a través de la API
WebSocket. Los atributos `device_tracker` de la app complementaria
(`battery_level`, `source_type`) se comprobaron contra el código fuente
de la integración `mobile_app`, no contra un teléfono real.

## Crear un usuario dedicado y un token de larga duración en Home Assistant {#create-a-dedicated-user-and-a-long-lived-token-in-home-assistant}

1. En Home Assistant: Settings → People → Users → Add user (activa
   Advanced mode en tu perfil si falta la pestaña Users). Nombra el
   usuario como sigue, dale una contraseña larga y aleatoria y deja
   desactivado el interruptor Administrator. Un usuario normal puede
   leer cada entidad y la lista de reparaciones, y no puede cambiar la
   configuración.

    ```
    dumbmonit
    ```

2. Inicia sesión en Home Assistant una vez como ese usuario, abre su
   perfil (el nombre al final de la barra lateral) → Security →
   Long-lived access tokens → Create token. Nómbralo según DumbMonit y
   cópialo ahora: se muestra solo una vez.

3. En DumbMonit, introduce la dirección de Home Assistant, por ejemplo
   «homeassistant.local» (puerto 8123) o «https://ha.example.net»
   detrás de un proxy inverso, y pega el token. Las baterías por debajo
   del 20 % cuentan como bajas; la opción Low battery threshold cambia
   eso.

4. DumbMonit solo lee /api/config, /api/states y, a través de la API
   WebSocket, la lista de reparaciones. Nunca llama a un servicio, nunca
   dispara un evento y nunca cambia un estado.

!!! warning
    Un token de larga duración lleva todos los derechos de su usuario y
    permanece válido durante diez años: créalo para el usuario dedicado,
    nunca para el tuyo propio. Home Assistant sirve HTTP plano a menos
    que se configure TLS, y el token viaja con cada petición: en una red
    no confiable, usa HTTPS. Cada teléfono y tableta con sesión iniciada
    en la app complementaria (la integración mobile_app) aparece en la
    tabla Devices de abajo con su última actividad; es opcional y
    permanece en silencio sin esa app.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de acceso de larga duración | Un token del usuario dedicado, enviado como `Authorization: Bearer`. |

Dirección: un nombre de host o IP (`homeassistant.local`), `host:puerto`,
o una URL completa (`https://ha.example.net`, con un prefijo de ruta
detrás de un proxy inverso); se acepta un `/api` final. Opciones:
protocolo (HTTP por defecto), puerto (8123), comprobación de
certificado, tiempo de espera de la petición (10 s), umbral de batería
baja (20 %), dominios excluidos (ninguno), reparaciones (lectura) y
antigüedad de dispositivo (`device_stale_days`, 3 días por defecto).
