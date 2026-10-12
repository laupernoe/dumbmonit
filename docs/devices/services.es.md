# Servicios: HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket {#services-http-tcp-dns-ping-tls-ntp-smtp-sql-mqtt-websocket}

Once monitores vigilan **servicios** en lugar de equipos, al estilo
Uptime Kuma: una página web, un puerto, un nombre DNS, un host, un
certificado, un servidor de hora, un relay de correo, una base de
datos, un broker MQTT, un endpoint WebSocket. Cada servicio obtiene una
barra de historial, un tiempo de respuesta y un porcentaje de
disponibilidad en su página de dispositivo.

Los últimos cinco van más allá de abrir una conexión: inician una
sesión real en el propio protocolo del servicio. Esa es la diferencia
entre «el puerto 5432 está abierto» y «la base de datos todavía acepta
mi cuenta y responde a una consulta». NTP está entre los dos: una
única petición y respuesta, en el propio protocolo NTP, suficiente
para leer el reloj del servidor sin una sesión.

## Cómo funciona el estado de servicio {#how-service-state-works}

Un servicio puede responder y seguir caído: un `500`, una palabra
clave faltante, un certificado caducado. Así que estos monitores
escriben un punto en cada comprobación, con `probe_success` siendo 1 o
0, en lugar de quedarse en silencio como un dispositivo de hardware.
Eso es lo que hace posible un porcentaje de disponibilidad:
`avg_over_time(dumbmonit_probe_success[30d])`.

| Situación | Qué ocurre |
|---|---|
| Opción o dirección inválida, sockets ICMP no permitidos (ver Ping) | Error de configuración: no se escribe nada, la página del dispositivo muestra el error, sin alerta «down». |
| Conexión rechazada, timeout, TLS rechazado, `500`, palabra clave ausente, pérdida total de paquetes, contraseña rechazada, consulta fallida | `probe_success = 0`: la caída se registra, se fecha y se cuenta. |
| Todo bien | `probe_success = 1`. |

Cada monitor tiene su propio timeout (`timeout_seconds`, 5 s por
defecto, 60 s como máximo), más corto que el `DUMBMONIT_PROBE_TIMEOUT_SECS`
del servidor: interrumpido por el scheduler, no podría escribir su
cero.

Las reglas integradas que se aplican a los once: Service down (3
minutos), Service flapping (más de seis cambios de estado en treinta
minutos), Slow service (más de 3 s durante diez minutos). Para cada
comprobación que lee un certificado — `http`, `tls`, `smtp`, `mqtt`,
`websocket`: Certificate expiring soon (14 días) y Certificate
expired.

Una contraseña rechazada nunca es una caída del servicio: se registra
como `reason="auth"`, así que la alerta dice qué corregir.

### Métricas {#metrics}

Todas gauges, todas con el prefijo `dumbmonit_`, todas etiquetadas
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket"`
además de `target`, `host` y `tag_*`.

| Métrica | Monitores | Significado |
|---|---|---|
| `probe_success` | todos | 1 si el servicio responde correctamente, 0 en caso contrario. |
| `probe_duration_seconds` | todos | Duración total de la comprobación. |
| `probe_failure_info` | todos | Presencia (1) con una etiqueta `reason`: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Código de estado obtenido. |
| `probe_http_first_byte_seconds` | http | Tiempo hasta las cabeceras de respuesta. |
| `probe_http_content_bytes` | http | Tamaño del cuerpo. |
| `probe_connect_seconds` | todos excepto dns y ping | Establecimiento de conexión (inicio de sesión incluido para bases de datos). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Negociación TLS por sí sola. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Presencia; etiqueta `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Días antes de la caducidad, negativo si ya caducó. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 si la cadena lleva a una autoridad conocida. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Presencia; etiqueta `issuer`. |
| `probe_dns_lookup_seconds` | dns | Tiempo de resolución. |
| `probe_dns_answer_records` | dns | Registros del tipo solicitado. |
| `probe_dns_answer_fingerprint` | dns | Con «Alert when the answer changes»: una huella de 32 bits de la respuesta ordenada. Su valor no significa nada; solo sus cambios. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Promedio, más corto y más largo del viaje de ida y vuelta. |
| `probe_icmp_packet_loss_ratio` | ping | Pérdida entre 0 y 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Ecos enviados y recibidos. |
| `probe_smtp_greeting_seconds` | smtp | Tiempo hasta el saludo `220`. |
| `probe_smtp_ehlo_seconds` | smtp | Ida y vuelta del comando `EHLO`. |
| `probe_smtp_capabilities` | smtp | Extensiones anunciadas. |
| `probe_smtp_authenticated` | smtp | 1 cuando `AUTH` tuvo éxito. |
| `probe_sql_query_seconds` | postgres, mysql | Ejecución de la consulta por sí sola. |
| `probe_sql_rows` | postgres, mysql | Filas devueltas. |
| `probe_sql_value` | postgres, mysql | Primera columna de la primera fila, cuando es un número. |
| `probe_mqtt_connack_seconds` | mqtt | Tiempo hasta el `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Tiempo hasta el `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Tamaño del mensaje retenido. |
| `probe_mqtt_message_value` | mqtt | Su contenido, cuando es un número. |
| `probe_ws_handshake_seconds` | websocket | Handshake de apertura. |
| `probe_ws_message_bytes` | websocket | Tamaño del frame recibido. |

## HTTP

**Sitio web o API web (HTTP)** — comprueba que una página o una API
responde, con el código de estado correcto, el contenido correcto, y
un certificado válido. Ejemplos: un sitio web, una página de salud de
aplicación, una API REST, una interfaz de servicio autoalojado.

### Configuración {#setup}

1. En la dirección, pega la URL completa de la página a monitorizar.
   Sin «http://» ni «https://», se asume HTTPS.
2. Prefiere una página ligera que no necesite inicio de sesión, por
   ejemplo la página de salud de la aplicación («/health», «/status»),
   en lugar de la página de inicio.
3. Si la página requiere autenticación, rellena la credencial: un
   nombre de usuario y contraseña dan autenticación básica, un token
   se envía como «Bearer».
4. Por defecto, cualquier código de estado entre 200 y 299 está bien.
   Para ir más allá, requiere una palabra clave en la página o un
   valor específico en una respuesta JSON.
5. Por HTTPS, el certificado se lee automáticamente: se te avisará
   antes de que caduque.

!!! warning
    Nunca escribas una contraseña o un token en las opciones: se
    copian en texto plano en cada medición. Usa el campo de
    credencial, que está cifrado.

Credenciales: ninguna, nombre de usuario / contraseña (autenticación
básica) o token de API (`Authorization: Bearer`).

### Opciones {#options}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `method` | HTTP method | `GET` | GET encaja en casi todos los casos. HEAD evita descargar la página cuando solo importa el código de estado. Opciones: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Accepted status codes | `200-299` | Códigos o rangos considerados normales, separados por comas. Cualquier otro código cuenta como caído. Ejemplo: `200-299,301,404`. |
| `keyword` | Expected keyword | *(vacío)* | Texto que debe aparecer en la página. Si falta, el servicio se reporta caído aunque la página responda. |
| `keyword_absent` | Keyword must be absent | `false` | Invierte la comprobación: la presencia de la palabra clave indica un fallo. Útil para una página de error que responde 200. |
| `keyword_case_sensitive` | Match keyword case | `false` | Por defecto, mayúsculas y minúsculas se tratan igual. |
| `json_path` | JSON path to check | *(vacío)* | Para una respuesta JSON: ruta del valor a comprobar. Rellénala junto con el valor esperado, nunca uno sin el otro. Ejemplo: `$.status`. |
| `json_expect` | Expected JSON value | *(vacío)* | Valor que debe tener la ruta JSON de arriba, por ejemplo «ok» o «true». |
| `headers` | Extra headers | *(vacío)* | Una o más cabeceras «Name: value», separadas por «\|». No pongas secretos aquí: las opciones son visibles en las gráficas. |
| `body` | Request body | *(vacío)* | Contenido enviado con la petición, para métodos que esperan uno (POST, PUT…). |
| `follow_redirects` | Follow redirects | `true` | Desactívalo para monitorizar la propia redirección, por ejemplo un 301 a HTTPS. |
| `max_redirects` | Maximum redirects followed | `10` | Entre 1 y 20. Sin efecto si las redirecciones no se siguen. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | La comprobación ya no falla por un certificado autofirmado o emitido por una autoridad privada. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Por defecto la comprobación rechaza direcciones que solo el propio host de DumbMonit puede alcanzar (127.0.0.1, ::1, 169.254.x.x). Actívalo para monitorizar un servicio que se ejecuta en el host de DumbMonit. Las direcciones de LAN privada (10.x, 192.168.x) siempre se permiten. |
| `check_certificate` | Read the certificate | `true` | Por HTTPS, también registra la fecha de caducidad del certificado para que puedas ser avisado antes de que caduque. |
| `max_body_bytes` | Maximum bytes read | `524288` | Más allá de esto, el resto de la página no se descarga. La palabra clave y la ruta JSON solo se buscan en esta parte. |
| `user_agent` | Announced identity (User-Agent) | `DumbMonit/<version>` | Nombre que da la comprobación al servidor. Cámbialo si el servidor filtra robots. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

## TCP

**Puerto de red (TCP)** — comprueba que un puerto acepta conexiones:
SSH, base de datos, compartición de archivos, servidor de juego,
impresora de red.

### Configuración {#setup-1}

1. En la dirección, escribe el host seguido del puerto, separados por
   dos puntos: «nas.home.lan:22» o «192.168.1.5:445».
2. Para una dirección IPv6, pon la entre corchetes: «[fd00::1]:445».
3. También puedes dejar la dirección sin puerto e introducirlo en la
   opción «Port»: se requiere uno de los dos.
4. Solo se comprueba la apertura de la conexión: no se envía ningún
   dato al servicio, así que no tiene ningún efecto sobre él.

!!! warning
    Un puerto abierto no prueba que la aplicación detrás de él
    funcione. Para un servicio web, prefiere el tipo «Website or web
    API», que lee la respuesta.

### Opciones {#options-1}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `port` | Port | *(vacío)* | Puerto a abrir, si la dirección no lo da ya como «host:port». Se requiere uno de los dos. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Por defecto la comprobación rechaza direcciones que solo el propio host de DumbMonit puede alcanzar (127.0.0.1, ::1, 169.254.x.x). Actívalo para monitorizar un servicio que se ejecuta en el host de DumbMonit. Las direcciones de LAN privada (10.x, 192.168.x) siempre se permiten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

## DNS

**Nombre de dominio (DNS)** — comprueba que un nombre resuelve, y que
apunta a la dirección correcta. Ejemplos: tu nombre de dominio, un
nombre interno servido por tu Pi-hole o AdGuard, un registro MX.

### Configuración {#setup-2}

1. En la dirección, escribe el nombre a resolver, sin «http://»:
   «www.example.com».
2. Por defecto, la comprobación pide al resolutor del sistema una
   dirección IPv4 (registro A). Elige otro tipo de registro en las
   opciones si es necesario.
3. Para monitorizar tu propio servidor DNS, introduce su dirección IP
   en «DNS server to query»: la comprobación fallará si deja de
   responder.
4. Para detectar un secuestro o una configuración errónea, lista en
   «Expected values» las direcciones que debe contener la respuesta.

!!! warning
    El servidor DNS se da por su dirección IP, no por un nombre: haría
    falta un resolutor para resolver al resolutor.

### Opciones {#options-2}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `record_type` | Record type | `A` | A para una dirección IPv4, AAAA para IPv6, MX para correo, CNAME para un alias… Opciones: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | DNS server to query | *(vacío)* | Dirección IP de un resolutor, puerto opcional. Vacío: el resolutor del sistema. Útil para comprobar tu propio servidor DNS. Ejemplo: `1.1.1.1` o `10.0.0.1:5353`. |
| `expect` | Expected values | *(vacío)* | Valores que todos deben aparecer en la respuesta, separados por comas. Vacío: solo se comprueba la resolución. |
| `expect_mode` | Comparison | `contains` | `contains`: cada valor esperado debe aparecer en algún sitio de la respuesta. `exact`: la respuesta debe contener esos valores y nada más. |
| `forbid` | Forbidden values | *(vacío)* | Valores que nunca deben aparecer en la respuesta, separados por comas. Se comprueba antes que los esperados. |
| `alert_on_change` | Alert when the answer changes | `false` | Registra una huella de la respuesta ordenada y alerta cada vez que cambia, sin tener que escribir los valores esperados. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

### Afirmar qué responde una zona {#asserting-what-a-zone-answers}

`expect` por sí solo detecta el registro que desapareció. No detecta
el registro que se *añadió*: una zona secuestrada a menudo conserva el
`A` legítimo y pone un segundo al lado, y un cliente entonces alcanza
cualquiera de los dos al azar.

- `expect_mode = exact` rechaza cualquier valor que la expectativa no
  cubra. El orden no importa — DNS no conserva ninguno.
- `forbid` nombra valores que nunca deben volver: la dirección de un
  host anterior, un `TXT` de validación caducado, un servidor de
  correo que desmantelaste. Se comprueba primero, así que gana sobre
  `expect`.

Ambos comparan sin distinguir mayúsculas/minúsculas, ignoran el punto
final de un nombre y las comillas alrededor de un `TXT`, y tratan un
valor esperado como encontrado cuando aparece dentro de un registro —
así que un `MX` coincide con `mail.example.com` sin que tengas que
copiar su prioridad.

### Alertar cuando cambia la respuesta {#alerting-when-the-answer-changes}

Escribir los valores esperados no siempre es práctico: un nombre
servido por un CDN, registros que gestiona otra persona, o
simplemente una zona sobre la que quieres saber cada vez que se mueve.
Marca **Alert when the answer changes**: cada comprobación entonces
registra una huella de la respuesta, y la regla integrada [DNS answer
changed](../alerting/rules.md#reverse-proxies-and-domains) se activa
cuando difiere de la anterior. La alerta se resuelve por sí misma una
hora después; la nueva respuesta es la referencia desde entonces.

La huella cubre los valores de la respuesta, ordenados y sin
duplicados, con la cadena CNAME incluida: un round-robin que devuelve
las mismas direcciones en otro orden no la cambia, ni lo hace un TTL.
Un nombre cuya respuesta cambia en cada consulta (un pool grande como
`pool.ntp.org`, o DNS geográfico que responde distinto de un resolutor
a otro) alertaría todo el tiempo: deja la opción desactivada para
esos, o consulta un resolutor fijo.

## Ping

**Host accesible (ping)** — envía unos pocos ecos ICMP y mide el
tiempo de respuesta y la pérdida de paquetes. Ejemplos: router o
gateway doméstico, punto de acceso Wi-Fi, impresora, máquina sin
agente ni SNMP, host remoto.

### Configuración {#setup-3}

1. En la dirección, escribe el nombre de host o dirección IP:
   «192.168.1.1» o «router.home.lan».
2. En cada sondeo, la comprobación envía cuatro ecos y mide el tiempo
   de respuesta, junto con la proporción de paquetes perdidos.
3. Por defecto, solo una pérdida total cuenta como caído; la pérdida
   parcial sigue siendo visible en las gráficas. Baja «Tolerated
   loss» para ser avisado antes.
4. Si DumbMonit se ejecuta en Docker, conserva las líneas `sysctls:`
   del docker-compose.yml incluido bajo el servicio dumbmonit:
   permiten al usuario sin privilegios del contenedor abrir sockets de
   eco ICMP. No se necesita ninguna capacidad.

!!! warning
    Sin ese sysctl, la comprobación no puede abrir un socket ICMP: lo
    reporta como un error de configuración, no como un fallo de host.
    Algunos dispositivos también ignoran los pings a propósito:
    compruébalo antes de sacar conclusiones.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Fuera de Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"`
en el host hace lo mismo, y un socket raw (`NET_RAW`, root) también
funciona.

### Opciones {#options-3}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `count` | Number of echoes | `4` | Paquetes enviados en cada sondeo, de 1 a 20. |
| `packet_timeout_ms` | Wait per packet (ms) | `1000` | Tiempo permitido para cada respuesta, de 50 a 10 000 ms. |
| `interval_ms` | Gap between two echoes (ms) | `100` | Pausa entre dos ecos, de 0 a 5000 ms. |
| `payload_bytes` | Payload size (bytes) | `56` | Datos llevados por cada eco, de 0 a 1400 bytes. |
| `ip_version` | IP version | `auto` | «auto» toma la primera dirección resuelta; fuerza 4 o 6 si el host tiene ambas y una no responde. Opciones: auto, 4, 6. |
| `loss_threshold_percent` | Tolerated loss (%) | `100` | Por encima de este porcentaje de paquetes perdidos, el host se reporta caído. 100: solo una pérdida total cuenta, la pérdida parcial sigue siendo visible en las gráficas. |
| `timeout_seconds` | Timeout (seconds) | `5` | Presupuesto total para el sondeo, de 1 a 60. Rechazado si no cubre «número de ecos × espera por paquete». |

## TLS

**Certificado TLS** — comprueba que un certificado es válido y avisa
antes de que caduque, en cualquier puerto cifrado. Ejemplos: servidor
de correo (IMAPS, SMTPS), proxy inverso, directorio LDAPS, broker
MQTT, interfaz de administración.

### Configuración {#setup-4}

1. En la dirección, escribe el host y, si no es 443, el puerto del
   servicio cifrado: «mail.example.com:993», «ldap.home.lan:636».
2. La comprobación abre una conexión cifrada, lee el certificado
   presentado y cierra de inmediato: no se intercambia ningún dato
   con la aplicación.
3. Registra el número de días antes de la caducidad, el emisor y la
   versión de TLS. Las reglas de alerta incluidas avisan a los
   catorce días, y luego a la caducidad.
4. Si el servicio se alcanza por su dirección IP pero el certificado
   lleva un nombre, introduce ese nombre en «Server name (SNI)».
5. Para un certificado emitido por tu propia autoridad, marca «Accept
   an unverifiable certificate»: la fecha de caducidad sigue
   vigilándose.

!!! warning
    Para un sitio web, el tipo «Website or web API» ya lee el
    certificado: este tipo está pensado para servicios que no hablan
    HTTP, o cuya aplicación no quieres tocar.

El handshake se completa incluso cuando el certificado se rechaza: eso
es lo que permite leer la fecha de caducidad de un certificado que ya
ha caducado. El veredicto de la verificación está en
`probe_ssl_cert_valid`.

### Opciones {#options-4}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `server_name` | Server name (SNI) | *(vacío)* | Nombre de dominio anunciado al servidor y comprobado en el certificado. Defínelo cuando la dirección es una IP detrás de un proxy inverso. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una cadena no verificable ya no cuenta como fallo: la fecha de caducidad sigue registrándose. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Por defecto la comprobación rechaza direcciones que solo el propio host de DumbMonit puede alcanzar (127.0.0.1, ::1, 169.254.x.x). Actívalo para monitorizar un servicio que se ejecuta en el host de DumbMonit. Las direcciones de LAN privada (10.x, 192.168.x) siempre se permiten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

## SMTP

**Relay de correo (SMTP)** — abre una sesión real en un servidor de
correo: saludo, EHLO, STARTTLS, y el inicio de sesión si proporcionas
uno. Ejemplos: el relay SMTP de tu proveedor, un Postfix o msmtp
local, Proxmox Mail Gateway, el puerto de envío de un servidor de
correo.

### Configuración {#setup-5}

1. En la dirección, escribe el servidor de correo, y el puerto si no
   es el habitual: «smtp.example.com» o «smtp.example.com:2525».
2. Elige el cifrado que usa tu relay: STARTTLS para el puerto de envío
   587, TLS para el puerto 465, None para un relay en tu propia red
   escuchando en el puerto 25.
3. Para comprobar que el relay todavía acepta tu cuenta, rellena la
   credencial: la comprobación entonces ejecuta AUTH y reporta una
   contraseña rechazada como tal, no como una caída.
4. Nunca se envía nada: la comprobación se detiene tras el saludo, las
   extensiones y el inicio de sesión opcional, y luego cuelga. Ningún
   mensaje entra en la cola.
5. Para ser avisado cuando desaparece una extensión, nómbrala en
   «Expected extension»: un relay que deja de anunciar STARTTLS es un
   relay que enviaría tu correo en texto plano.

!!! warning
    Una sesión aceptada no prueba que el correo salga. Un relay que
    acepta todo y lo encola para siempre responde perfectamente aquí:
    vigila las propias colas para eso.

Credenciales: ninguna, o un nombre de usuario y contraseña enviados
con `AUTH PLAIN` (o `AUTH LOGIN` cuando el servidor solo ofrece eso).
Se envían **después** de que la conexión se cifra y después de que se
juzga el certificado: un relay cuya cadena no se puede verificar nunca
las recibe.

Por TLS o STARTTLS, el certificado se lee igual que lo lee la
comprobación TLS — caducidad, emisor, versión de protocolo — así que
un relay de correo también es un certificado del que se te avisa.

### Qué no puede detectar {#what-it-cannot-detect}

Que el correo realmente salga. Un relay que acepta cada sesión y luego
retrasa todo responde perfectamente aquí; vigila sus colas en su
lugar (consulta la página de Proxmox Mail Gateway). Tampoco envía
nunca un mensaje, así que no dice nada sobre filtrado de
destinatarios, SPF o DKIM.

### Opciones {#options-5}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `security` | Encryption | `starttls` | STARTTLS empieza en texto plano y mejora (puerto 587), TLS cifra desde el primer byte (puerto 465), None nunca cifra (puerto 25) y solo conviene a un relay en tu propia red. |
| `port` | Port | *(vacío)* | Se usa si la dirección no da uno. Vacío: 587 con STARTTLS, 465 con TLS, 25 sin cifrado. |
| `helo_name` | Name announced (EHLO) | `dumbmonit` | Nombre que da la comprobación al presentarse. Un relay estricto rechaza un nombre que no puede resolver. |
| `expect_capability` | Expected extension | *(vacío)* | Extensión que el servidor debe anunciar en su respuesta EHLO, por ejemplo STARTTLS o AUTH. Vacío: ninguna expectativa. |
| `server_name` | Server name (SNI) | *(el host)* | Nombre de dominio anunciado al servidor y comprobado en el certificado. Defínelo cuando la dirección es una IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una cadena no verificable ya no cuenta como fallo: la fecha de caducidad sigue registrándose. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Consulta «Direcciones que las comprobaciones rechazan» abajo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

### Motivos de fallo {#failure-reasons}

| `reason` | Qué ocurrió |
|---|---|
| `connect` | El puerto está cerrado o el relay es inaccesible. |
| `protocol` | Algo respondió, pero no SMTP — o rechazó la conexión, `EHLO`, o el STARTTLS que había anunciado. |
| `auth` | La cuenta fue rechazada (`535`, `530`, `534`). El relay está bien; la credencial no. |
| `payload` | La extensión nombrada en «Expected extension» no se anuncia. |
| `tls`, `cert_expired` | Mismo significado que para la comprobación TLS. |

## PostgreSQL

**Base de datos PostgreSQL** — conecta, autentica y ejecuta una
consulta: la señal de una base de datos que está activa pero ya no
responde. Ejemplos: la base de datos detrás de Nextcloud o Immich, un
recorder de Home Assistant, una base de datos de aplicación, una
instancia TimescaleDB.

### Configuración {#setup-6}

1. En el servidor, crea una cuenta para monitorización y no le des
   nada más que el derecho a conectar: CREATE ROLE dumbmonit LOGIN
   PASSWORD 'a-long-password';
2. Asegúrate de que esa cuenta puede alcanzar el servidor desde la
   máquina de DumbMonit, lo cual normalmente significa una línea más
   en pg_hba.conf seguida de una recarga de configuración.
3. En la dirección, escribe el servidor, y el puerto si no es 5432:
   «db.home.lan» o «db.home.lan:5433».
4. La consulta por defecto es SELECT 1, que no lee ninguna tabla: no
   hay que conceder nada más. El tiempo de conexión y el tiempo de
   consulta se registran por separado.
5. Para vigilar algo propio, sustituye la consulta por una que
   devuelva un único número, y se convierte en una gráfica: una
   longitud de cola, un recuento de filas, un retraso de replicación.

!!! warning
    La comprobación abre una conexión real en cada sondeo. En una
    instancia ya cerca de su límite de conexiones, dale un intervalo
    más largo.

Credenciales: un nombre de usuario y contraseña, obligatorio. Una base
de datos nunca se monitoriza de forma anónima, así que el formulario
no ofrece nada más.

### Qué no puede detectar {#what-it-cannot-detect-1}

Retraso de replicación, un bloqueo que se prolonga, una tabla que
sigue creciendo. Todo eso vive en las propias vistas de administración
del motor — escribe tú mismo la consulta, y el número que devuelve se
convierte en `probe_sql_value`, una gráfica sobre la que puedes
alertar.

### Opciones {#options-6}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `port` | Port | `5432` | Se usa si la dirección no da uno. |
| `database` | Database | `postgres` | Base de datos abierta al conectar. La cuenta de monitorización debe poder conectarse a ella. |
| `query` | Query | `SELECT 1` | Ejecutada en cada comprobación. La por defecto no lee ninguna tabla, así que la cuenta no necesita ningún privilegio más allá de conectar. Una consulta que devuelve un número la convierte en una gráfica. |
| `expect` | Expected value | *(vacío)* | Valor que debe tener la primera columna de la primera fila. Vacío: solo se comprueba que la consulta tenga éxito. |
| `sslmode` | Encryption | `prefer` | `prefer` cifra cuando el servidor lo ofrece, `require` se niega a conectar sin ello, `verify-full` también comprueba el certificado y el nombre, `disable` nunca cifra. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Consulta «Direcciones que las comprobaciones rechazan» abajo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

## MySQL y MariaDB

**Base de datos MySQL o MariaDB** — conecta, autentica y ejecuta una
consulta: la señal de una base de datos que está activa pero ya no
responde. Ejemplos: la base de datos detrás de un WordPress, un
Nextcloud o un Kimai, una base de datos de aplicación, un MariaDB en
Docker.

### Configuración {#setup-7}

1. En el servidor, crea una cuenta para monitorización sin ningún
   privilegio en absoluto: CREATE USER 'dumbmonit'@'%' IDENTIFIED BY
   'a-long-password';
2. Comprueba que la cuenta puede conectar desde la máquina de
   DumbMonit: un patrón de host de localhost solo aceptaría conexiones
   hechas en el propio servidor.
3. En la dirección, escribe el servidor, y el puerto si no es 3306:
   «db.home.lan» o «db.home.lan:3307».
4. La consulta por defecto es SELECT 1, que no lee ninguna tabla: no
   hay que conceder nada más. El tiempo de conexión y el tiempo de
   consulta se registran por separado.
5. Para vigilar algo propio, sustituye la consulta por una que
   devuelva un único número, y se convierte en una gráfica: una
   longitud de cola, un recuento de filas, un retraso de replicación.

!!! warning
    La comprobación abre una conexión real en cada sondeo. En una
    instancia ya cerca de su límite de conexiones, dale un intervalo
    más largo.

### Opciones {#options-7}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `port` | Port | `3306` | Se usa si la dirección no da uno. |
| `database` | Database | *(vacío)* | Base de datos abierta al conectar. Vacío: ninguna, que basta para la consulta por defecto. |
| `query` | Query | `SELECT 1` | Ejecutada en cada comprobación. La por defecto no lee ninguna tabla, así que la cuenta no necesita ningún privilegio más allá de conectar. Una consulta que devuelve un número la convierte en una gráfica. |
| `expect` | Expected value | *(vacío)* | Valor que debe tener la primera columna de la primera fila. Vacío: solo se comprueba que la consulta tenga éxito. |
| `sslmode` | Encryption | `prefer` | `prefer` cifra cuando el servidor lo ofrece, `require` se niega a conectar sin ello, `verify-full` también comprueba el certificado y el nombre, `disable` nunca cifra. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Consulta «Direcciones que las comprobaciones rechazan» abajo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

### Motivos de fallo, ambos motores {#failure-reasons-both-engines}

| `reason` | Qué ocurrió |
|---|---|
| `connect` | El puerto está cerrado, el servidor es inaccesible, o rechazó la conexión por un motivo distinto de la cuenta (arrancando, demasiadas conexiones). |
| `auth` | La cuenta fue rechazada: contraseña incorrecta, ninguna línea en `pg_hba.conf`, patrón de host que no cubre la máquina de DumbMonit. |
| `query` | Conectó y autenticó, pero la consulta falló: tabla faltante, base de datos en solo lectura, error de sintaxis. |
| `payload` | La consulta funcionó pero devolvió algo distinto al valor esperado. |
| `timeout` | La conexión o la consulta no terminaron a tiempo — el caso «activo pero lento». |

## MQTT

**Broker MQTT** — conecta al broker, se suscribe a un topic, y puede
esperar un mensaje retenido. Ejemplos: Mosquitto, el broker detrás de
Home Assistant, Zigbee2MQTT, sensores ESPHome.

### Configuración {#setup-8}

1. En la dirección, escribe el broker, y el puerto si no es el
   habitual: «broker.home.lan» o «broker.home.lan:1884».
2. Marca «Encrypted connection» para un broker que escucha en 8883, y
   rellena la credencial si requiere una cuenta.
3. Conectar ya vale la pena vigilarlo: un broker que rechaza tu
   cuenta, o que ha dejado de responder, es el motivo por el que toda
   la casa se quedó en silencio.
4. Para ir más allá, nombra un topic: la comprobación entonces se
   suscribe a él, y un broker que rechaza la suscripción te dice que
   la cuenta perdió sus derechos de acceso.
5. Marca «Expect a retained message» solo para un topic que lleva uno,
   típicamente un topic de disponibilidad. Un topic al que se publica
   de vez en cuando parece silencioso a un cliente que acaba de
   conectar.

!!! warning
    La comprobación nunca publica nada. Tampoco puede saber cuán
    antiguo es un mensaje retenido: MQTT no los fecha, así que un
    valor congelado hace tres semanas sigue contando como presente.

### Qué no puede detectar {#what-it-cannot-detect-2}

Que un sensor concreto deje de emitir. El mensaje retenido que lee la
comprobación es lo que sea que guardó el broker, posiblemente de hace
tres semanas — MQTT no lo marca con fecha. Para la frescura, vigila la
automatización que lo consume, o usa un heartbeat.

### Opciones {#options-8}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `tls` | Encrypted connection | `false` | Cifra la sesión desde el primer byte, como hacen los brokers en el puerto 8883. |
| `port` | Port | *(vacío)* | Se usa si la dirección no da uno. Vacío: 1883 en texto plano, 8883 cifrado. |
| `topic` | Topic | *(vacío)* | Topic al que se suscribe la comprobación. Vacío: solo conecta, lo cual ya te dice que el broker está vivo y acepta tu cuenta. |
| `expect_message` | Expect a retained message | `false` | Debe llegar un mensaje en ese topic antes del timeout. Solo los mensajes retenidos llegan de inmediato: un topic al que simplemente se publica de vez en cuando parecerá silencioso. |
| `expect` | Expected content | *(vacío)* | Texto que ese mensaje debe contener. Rellenarlo implica esperar un mensaje. |
| `client_id` | Client identifier | *(derivado)* | Nombre anunciado al broker. El por defecto se deriva del dispositivo, así que dos comprobaciones nunca se desconectan entre sí. |
| `server_name` | Server name (SNI) | *(el host)* | Nombre de dominio anunciado al broker y comprobado en el certificado. Defínelo cuando la dirección es una IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una cadena no verificable ya no cuenta como fallo: la fecha de caducidad sigue registrándose. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Consulta «Direcciones que las comprobaciones rechazan» abajo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

### Motivos de fallo {#failure-reasons-1}

| `reason` | Qué ocurrió |
|---|---|
| `connect` | El puerto está cerrado o el broker es inaccesible. |
| `auth` | El `CONNACK` rechazó la cuenta, o el broker rechazó la suscripción — una lista de control de acceso ya no cubre este topic. |
| `protocol` | Algo respondió pero no habla MQTT 3.1.1, o el broker rechazó el identificador de cliente. |
| `payload` | No llegó ningún mensaje retenido en el topic, o su contenido no contiene el texto esperado. |

## WebSocket

**Endpoint WebSocket** — ejecuta el handshake de upgrade, y puede
enviar un frame y esperar uno: un fallo distinto al de una página
HTTP plana. Ejemplos: la API de Home Assistant, un dashboard en vivo,
un flujo de logs, un endpoint detrás de un proxy inverso.

### Configuración {#setup-9}

1. En la dirección, pega el endpoint completo:
   «wss://home.example.com/api/websocket». Sin esquema, se asume wss.
2. La comprobación ejecuta todo el handshake de apertura y verifica la
   respuesta que calcula el servidor a partir de la clave que se le
   envió: un proxy inverso que perdió la cabecera Upgrade se detecta
   aquí, donde una comprobación HTTP todavía vería un 200 sano.
3. Nada más se necesita para la mayoría de endpoints. Un estado
   distinto de 101 se reporta con su código, así que un 401 de un
   token caducado no se confunde con una caída.
4. Para ir más allá, rellena «Expected content» con un fragmento del
   primer frame que envía el servidor por sí mismo, por ejemplo el
   saludo de la API de Home Assistant.
5. Si el endpoint no dice nada hasta que se le habla, pon un mensaje
   en «Frame to send» y qué debe contener su respuesta en «Expected
   content».

!!! warning
    La comprobación abre y cuelga en menos de un segundo. Una conexión
    que un cortafuegos o un timeout de proxy corta tras treinta
    segundos se ve perfectamente sana aquí.

Credenciales: ninguna, un nombre de usuario y contraseña (enviados
como autenticación básica HTTP en la petición de apertura) o un token
(enviado como `Authorization: Bearer`).

### Opciones {#options-9}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `path` | Path | *(de la dirección)* | Ruta de la petición de apertura. Se toma de la dirección cuando la lleva. |
| `port` | Port | *(vacío)* | Se usa si la dirección no da uno. Vacío: 443 para wss, 80 para ws. |
| `send` | Frame to send | *(vacío)* | Frame de texto enviado una vez abierta la conexión. Vacío: no se envía nada. |
| `expect` | Expected content | *(vacío)* | Texto que debe contener un frame recibido. Vacío y sin nada que enviar, solo se comprueba el handshake de apertura. |
| `subprotocol` | Subprotocol | *(vacío)* | Valor de Sec-WebSocket-Protocol. La comprobación falla si el servidor elige uno distinto. |
| `origin` | Origin | *(vacío)* | Valor de la cabecera Origin, que algunos servidores exigen antes de hacer el upgrade. |
| `server_name` | Server name (SNI) | *(el host)* | Nombre de dominio anunciado al servidor y comprobado en el certificado. Defínelo cuando la dirección es una IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una cadena no verificable ya no cuenta como fallo: la fecha de caducidad sigue registrándose. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Consulta «Direcciones que las comprobaciones rechazan» abajo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

### Motivos de fallo {#failure-reasons-2}

| `reason` | Qué ocurrió |
|---|---|
| `connect` | El puerto está cerrado o el host es inaccesible. |
| `status` | El endpoint respondió algo distinto de `101`; `probe_http_status_code` lleva el código. |
| `protocol` | Respondió `101` pero su `Sec-WebSocket-Accept` no coincide con la clave enviada, eligió otro subprotocolo, o cerró la conexión de inmediato. |
| `payload` | No llegó ningún frame, o no contiene el texto esperado. |

## NTP

**Servidor de hora NTP** — envía una única petición SNTP y lee la
respuesta: desfase, retraso de ida y vuelta, stratum e indicador leap.
Ejemplos: un router o NAS actuando como servidor de hora local, un
servidor NTP interno (chrony, ntpd), un servidor de pool público usado
como referencia.

### Configuración {#setup-10}

1. En la dirección, escribe el nombre o dirección IP del servidor:
   «pool.ntp.org» o «192.168.1.1». Se usa el puerto UDP 123 a menos
   que la opción «Port» diga otra cosa.
2. La comprobación envía una única petición SNTP y lee la respuesta:
   no se conserva ningún estado entre sondeos, y nada se escribe en
   el servidor.
3. Registra el desfase entre el reloj del servidor y el host de
   DumbMonit, el retraso de ida y vuelta, el stratum y el indicador
   leap.
4. Un servidor que responde stratum 16, o cuyo indicador leap se lee
   «unsynchronized», nunca ha logrado sincronizarse: la comprobación
   lo reporta caído sea cual sea el desfase.
5. Baja «Tolerated offset» para ser avisado antes de 100 ms; elévalo
   para un servidor a varios saltos de un reloj de referencia.

!!! warning
    El desfase solo es tan bueno como el propio reloj del host de
    DumbMonit: si ese host no está él mismo sincronizado (NTP, la
    sincronización de hora de un hipervisor), el desfase medido
    refleja su deriva tanto como la del servidor.

Credenciales: ninguna.

### Opciones {#options-10}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `port` | Port | `123` | Puerto UDP del servidor NTP. |
| `offset_threshold_ms` | Tolerated offset (ms) | `100` | Por encima de este desfase de reloj, el servidor se reporta caído. Un servidor no sincronizado (stratum 16) siempre falla, sea cual sea este valor. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Consulta «Direcciones que las comprobaciones rechazan» abajo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tiempo tras el cual el servicio se reporta caído si no ha respondido. Entre 1 y 60. |

### Motivos de fallo {#failure-reasons-3}

| `reason` | Qué ocurrió |
|---|---|
| `connect` | El socket UDP no se pudo abrir, o la respuesta vino de una dirección inesperada. |
| `timeout` | No llegó ninguna respuesta dentro del timeout. |
| `protocol` | La respuesta no era un paquete NTP válido, o no vino de un servidor (modo incorrecto). |
| `unsynchronized` | El servidor respondió stratum 16 o 0 (un «kiss-o'-death», que también nombra el motivo en el registro), o su indicador leap se lee «unsynchronized». |
| `clock_offset` | El desfase medido supera «Tolerated offset». |

## Direcciones que las comprobaciones rechazan {#addresses-the-checks-refuse}

Las comprobaciones que se conectan a una dirección que tú les das —
HTTP, TCP, TLS, NTP, SMTP, PostgreSQL, MySQL, MQTT, WebSocket —
reportan lo que ven — que es también, palabra por palabra, lo que hace
una falsificación de petición del lado del servidor. Para evitar que
un administrador de monitorización lea servicios que solo el propio
host de DumbMonit puede alcanzar (la base de datos de métricas
integrada en `127.0.0.1:8428`, el servicio de metadatos de un
proveedor cloud en `169.254.169.254`), las comprobaciones rechazan por
defecto:

- direcciones loopback (`127.0.0.0/8`, `::1`);
- direcciones link-local (`169.254.0.0/16`, `fe80::/10`), incluida la
  dirección de metadatos cloud, y la dirección sin especificar
  (`0.0.0.0`, `::`).

Los rangos de LAN privada (`10.0.0.0/8`, `172.16.0.0/12`,
`192.168.0.0/16`, `fc00::/7`) siempre se permiten: monitorizarlos es
el punto de un homelab.

La regla se aplica a la dirección que escribiste, a cada dirección a
la que resuelve un nombre de host (comprobado de nuevo en el momento
de conectar, así que un nombre que cambia su respuesta no puede
escabullirse), y a cada redirección que sigue la comprobación HTTP.
Una dirección rechazada se muestra como un error de configuración en
el target — no se envía ninguna alerta de caída — nombrando la opción
a activar: `allow_private_targets`. Actívala para un target que
realmente monitoriza un servicio en el propio host de DumbMonit.

Una redirección a una dirección rechazada se reporta como una
medición fallida (`reason="connect"`), porque es el servicio
monitorizado el que cambió, no la configuración.

Los valores encontrados en un `json_path` nunca se repiten más allá de
32 caracteres en un mensaje de fallo.

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| Ping muestra un error de configuración | Falta el sysctl `net.ipv4.ping_group_range` en el archivo Compose. Ver arriba. |
| HTTP caído con `reason="keyword"` | La palabra clave se busca solo en los primeros `max_body_bytes`, sin distinguir mayúsculas a menos que se defina `keyword_case_sensitive`. |
| HTTP caído con `reason="tls"` | Autofirmado o autoridad privada: define `insecure_tls`. |
| Una comprobación muestra un error de configuración nombrando `allow_private_targets` | La dirección es loopback o link-local. Consulta «Direcciones que las comprobaciones rechazan» arriba. |
| TCP caído aunque `telnet host port` funciona desde tu portátil | El contenedor de DumbMonit no puede alcanzar el host (red Docker, cortafuegos). Prueba desde dentro de la red del contenedor. |
| DNS caído con `reason="record"` | La respuesta no contiene todos los valores listados en `expect`. |
| Ping caído con `reason="packet_loss"` | Pérdida por encima de `loss_threshold_percent`; algunos dispositivos limitan la tasa de ICMP. |
| DNS caído con `reason="record"` aunque `expect` coincide | `expect_mode` es `exact` y la respuesta tiene un valor extra, o `forbid` coincidió. |
| SMTP caído con `reason="protocol"` en `security = starttls` | El relay no anuncia STARTTLS en este puerto. Usa `tls` para 465, `none` para un relay local plano. |
| Una comprobación de base de datos caída con `reason="auth"` | La cuenta existe pero puede no estar permitida conectar desde la máquina de DumbMonit: comprueba `pg_hba.conf`, o la parte de host del usuario MySQL. |
| Una comprobación de base de datos caída con `reason="query"` | La conexión e inicio de sesión funcionaron. La base de datos está activa y se niega a trabajar: solo lectura, tabla faltante, recuperación. |
| MQTT caído con `reason="payload"` en un topic que claramente tiene tráfico | Solo los mensajes *retenidos* llegan a un cliente que acaba de suscribirse. Publica ese topic con la bandera retain, o desmarca «Expect a retained message». |
| WebSocket caído con `reason="protocol"` tras un `101` | Algo en el camino responde al upgrade sin hablar WebSocket — normalmente una regla de proxy inverso que se traga la cabecera `Upgrade`. |
