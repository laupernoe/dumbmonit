# UniFi Network

La red UniFi y lo que gestiona: gateways, switches y puntos de acceso
fuera de línea, aislados o esperando adopción, actualizaciones de
firmware pendientes, los enlaces WAN y la conexión a Internet tal como
las ve el gateway, el número de clientes, y las alarmas que lanzó
UniFi. Funciona con la aplicación UniFi Network de una consola UniFi OS
(Dream Machine, Cloud Gateway, Cloud Key) y con un UniFi Network Server
autoalojado.

DumbMonit lee UniFi con una clave de API o con una cuenta local que tiene
el rol **View Only**. Nunca reinicia, adopta, actualiza ni aprovisiona un
dispositivo.

El fallo que vale la pena vigilar: **un punto de acceso o un switch se
cae silenciosamente**. Los clientes migran al siguiente punto de acceso,
la red sigue funcionando, y el agujero de cobertura o el enlace perdido
solo se nota días después. Lo mismo pasa con un segundo enlace WAN que se
cayó: nada se rompe hasta que el primero también falla.

## Dos formas de acceso {#two-ways-in}

| Credencial | API usada | Qué lee |
|---|---|---|
| Clave de API (consola UniFi OS) | La Integration API oficial, en `/proxy/network/integration/v1` en una consola y `/integration/v1` en un servidor autoalojado | Versión, dispositivos y su estado, actualizaciones de firmware, carga, memoria y tiempo de actividad de cada dispositivo en línea, número de clientes |
| Cuenta View Only | La API clásica (`/api/s/<site>/stat/*`), en `/proxy/network/api/…` en una consola | Todo lo anterior, más clientes por dispositivo y por tipo, los enlaces WAN, el estado WAN e Internet, y alarmas |

El estado WAN, Internet y de alarmas solo existe en la API clásica. Con
una clave, DumbMonit también pide a la API clásica el estado WAN e
Internet con la misma clave; si el controlador lo rechaza, esas series
simplemente están ausentes, sin ningún error.

Las alarmas son las entradas del registro «critical» del sistema de
UniFi de las últimas 24 horas (UniFi Network 8 y posteriores), o las
alarmas no archivadas en versiones más antiguas. Ese es el único `POST`
que envía DumbMonit, y solo lee.

Una sesión abierta con la cuenta View Only se conserva de una sonda a la
siguiente y se vuelve a abrir cuando caduca: UniFi no registra un nuevo
inicio de sesión cada minuto.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_unifi_`. Las series de
dispositivo llevan `device` (su nombre, o su dirección MAC cuando no
tiene uno), `mac`, `model` y `type` (`gateway`, `switch`,
`access_point`, `other`).

| Métrica | Qué | Etiquetas |
|---|---|---|
| `info` | valor 1 | `version`, `api` (`integration` o `classic`) |
| `devices` | dispositivos por estado: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | dispositivos adoptados con una actualización de firmware disponible | |
| `device_state` | 0 en línea, 1 fuera de línea, 2 pendiente de adopción, 3 actualizando o aprovisionando, 4 adoptando, 5 adopción fallida, 6 aislado, 7 otro | etiquetas de dispositivo |
| `device_up` | 1 en línea o actualizando, 0 fuera de línea, aislado o adopción fallida; ausente para un dispositivo aún no adoptado | etiquetas de dispositivo |
| `device_upgradable` | 1 cuando hay una actualización de firmware disponible | etiquetas de dispositivo, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | solo dispositivos en línea | etiquetas de dispositivo |
| `device_clients` | clientes conectados al dispositivo (cuenta View Only) | etiquetas de dispositivo |
| `wan_link_up` | 1 cuando un puerto WAN del gateway está activo; un puerto desactivado no tiene serie | etiquetas de dispositivo, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | el estado WAN e Internet del gateway; ausente sin un gateway UniFi | |
| `internet_latency_seconds` | latencia medida por el gateway | |
| `wan_availability_percent` | disponibilidad de cada WAN en las últimas 24 horas | `wan` |
| `subsystem_status` | 0 ok, 1 aviso, 2 error, 3 desconocido, para `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | clientes en el sitio | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (cuenta View Only) | `type` |
| `alarms` | entradas del registro crítico del sistema de las últimas 24 horas, o alarmas no archivadas | |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

Las [reglas integradas](../alerting/rules.md#unifi-network) que se
aplican: UniFi device offline, UniFi Internet down, UniFi WAN link
down, UniFi gateway CPU high, UniFi device waiting for adoption, UniFi
firmware update available, UniFi alarms, además de Device unreachable.

## Dispositivos {#devices}

`/rest/user` (la API clásica, leída con cualquiera de las dos
credenciales) lista cada cliente que el controlador ha visto alguna
vez — la mayor parte de una red doméstica, en la práctica: teléfonos de
paso, Wi-Fi de invitados, dispositivos inteligentes que nunca vuelven.
Rastrear antigüedad en todos ellos enterraría la alerta bajo
dispositivos que simplemente nunca regresan, no dispositivos que se
quedaron en silencio. Por defecto solo cuentan los clientes que el
usuario marcó de alguna forma: nombrados en UniFi, reservados en una IP
fija, o anotados. La opción **Clients to watch besides the marked ones**
añade direcciones MAC o nombres de cliente específicos, o `all` para
rastrear cada cliente (no recomendado salvo en una red pequeña y
mayormente estática).

Los clientes rastreados se publican bajo la familia genérica compartida
por toda integración de DumbMonit que rastrea dispositivos cliente —
`dumbmonit_client_device_*`, con prefijo sin `unifi`, documentada por
completo en [Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | Qué | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última conexión, tal como la reporta `/rest/user` | `device`, `type` (`Wired`, `Wireless` o `Unknown`), `os` (vacío), `user` (vacío), `kind` |
| `stale_seconds` | Positivo una vez que un cliente rastreado ha superado `device_stale_days` (3 por defecto) sin conectarse; impulsa la regla [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (siempre `connection`: UniFi no tiene copia de seguridad que reportar) |

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta UniFi. Indica primero qué va mal (dispositivos fuera de
línea, Internet o un enlace WAN caído, alarmas), luego los dispositivos
por tipo con su estado, firmware y carga, y los clientes, y después la
tabla de clientes rastreados con su última conexión.

## Cómo se validó {#how-it-was-validated}

El inicio de sesión, el rol View Only, las respuestas de error
(contraseña rechazada, sitio desconocido, clave rechazada, sesión
caducada) y las respuestas de un controlador sin dispositivos se
comprobaron contra UniFi Network 10.6.106, autoalojado. No se pudo
adoptar ningún dispositivo ahí: la lista de dispositivos, la salud de un
sitio equipado, los clientes conocidos (`/rest/user`) y las cargas de la
Integration API se validan contra la documentación oficial.

## Crear un acceso de solo lectura en UniFi Network {#create-a-read-only-access-in-unifi-network}

1. En una consola UniFi OS (Dream Machine, Cloud Gateway, Cloud Key),
   puedes crear una clave de API: en UniFi Network, Settings → Control
   Plane → Integrations → Create API Key. Nómbrala según DumbMonit y
   cópiala ahora: se muestra solo una vez. La clave lee dispositivos, su
   carga y el número de clientes a través de la Integration API oficial.

2. Para el estado WAN, Internet y de alarmas, y en un UniFi Network
   Server autoalojado, usa en su lugar una cuenta local con el rol View
   Only: Settings → Admins & Users → Create New. Nómbrala como sigue,
   dale una contraseña larga y aleatoria, restríngela al acceso local, y
   define su rol de UniFi Network como View Only. View Only lee todo y
   no puede cambiar nada.

    ```
    dumbmonit
    ```

3. En DumbMonit, introduce la dirección de la consola o servidor, por
   ejemplo «unifi.lan» (puerto 443 en una consola) o «unifi.lan:8443»
   para un servidor autoalojado, y pega la clave o la cuenta. Si la red
   no es el sitio por defecto, define la opción Site con el nombre corto
   mostrado en la barra de direcciones después de /manage/ o /network/.

4. DumbMonit solo lee. Nunca reinicia, adopta, actualiza ni aprovisiona
   un dispositivo, y nunca confirma una alarma.

!!! warning
    No uses la cuenta propietaria ni una cuenta cloud de UI.com: una
    contraseña filtrada abriría cada consola vinculada a ella. Mantén la
    autenticación de doble factor desactivada solo para esta cuenta
    local View Only, ya que un servidor de monitorización no puede
    escribir un código. Las consolas usan un certificado autofirmado por
    defecto: activa «Accept an unverifiable certificate» a menos que
    hayas instalado el tuyo. UniFi conoce cada cliente que se conectó
    alguna vez, la mayoría teléfonos de paso: solo los nombrados,
    reservados en una IP fija, o anotados en UniFi se vigilan por
    antigüedad por defecto, para evitar una alerta por el teléfono de
    cada visitante. Lista más en la opción Clients to watch, o
    introduce all.

## Controlador aún sin configurar {#controller-not-set-up-yet}

Un controlador autoalojado cuyo asistente de primer arranque no ha terminado responde a toda petición con una redirección a `/setup`. DumbMonit informa entonces «The UniFi controller is not set up yet» como un error de configuración (no una contraseña rechazada): termina el asistente en el navegador y vuelve a sondear.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Clave de API (consola UniFi OS) | Una clave creada en Settings → Control Plane → Integrations, enviada como `X-API-KEY`. |
| Cuenta View Only | El nombre de usuario y contraseña de la cuenta local View Only. |

Dirección: un nombre de host o IP (`unifi.lan`), `host:puerto`
(`unifi.lan:8443` para un servidor autoalojado), o una URL completa; se
acepta una dirección copiada del navegador, con `/manage/…` o
`/network/…`. Opciones: sitio (`default`), puerto (443), comprobación de
certificado, tiempo de espera de la petición (15 s), clientes a vigilar
además de los marcados (vacío, o `all`), y antigüedad de dispositivo
(`device_stale_days`, 3 días por defecto).
