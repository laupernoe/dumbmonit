# MDaemon Email Server

Los servicios de correo de un servidor MDaemon, comprobados desde fuera:
SMTP, IMAP, POP3, webmail y Remote Administration aceptan conexiones y
saludan como deben. Con una cuenta, DumbMonit también lee la versión
exacta a través de la API XML. Con el [agente](agent.md) instalado en el
servidor MDaemon, la misma página también muestra las colas de correo,
sesiones y totales de mensajes, que MDaemon solo publica como contadores
de rendimiento de Windows.

!!! warning "Validado solo contra la documentación"

    Esta integración se construyó a partir de la documentación publicada
    por MDaemon Technologies y el formato de respuesta de la API XML
    mostrado ahí, no contra un servidor MDaemon real. Cada llamada es de
    lectura, y las comprobaciones de puerto de correo siguen los
    estándares del protocolo, pero todavía nadie la ha ejecutado contra
    un MDaemon en producción. Lo mismo vale para los contadores de cola
    leídos por el agente: sus nombres vienen de una lista publicada de
    los contadores de rendimiento de MDaemon, no de un servidor en
    ejecución. Dinos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_mdaemon_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `service_up` | `service`, `port` | 1 si el puerto aceptó la conexión y, para SMTP, POP3 e IMAP, saludó con `220`, `+OK` o `* OK`. 0 si no respondió, o respondió para rechazar el servicio (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Tiempo hasta el saludo, o hasta la conexión para un puerto sin uno. |
| `api_up` | | Solo con una cuenta: 1 si la API XML respondió, 0 si no. |
| `api_response_seconds` | | Tiempo que tardó la API XML en responder. |
| `api_operation_ok` | | 1 si la cuenta tenía permiso para ejecutar la llamada, 0 si MDaemon respondió con un estado de error. |
| `info` (valor 1) | `version`, `build`, `source` | La versión: de la API XML (`source="xml_api"`, `build` es el build del servicio), o si no del saludo SMTP cuando el servidor lo anuncia ahí (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tiempo que tardó toda la comprobación. |

**Un fallo parcial sigue siendo una comprobación exitosa.** Un servicio
detenido da `service_up 0` para ese servicio y los demás se siguen
midiendo. Solo cuando ningún servicio ni ninguna API responde en
absoluto, el dispositivo es *inaccesible*.

**Una cuenta rechazada es un error de configuración, no una caída.** Una
contraseña incorrecta, una dirección que la API XML no permite, o un
puerto incorrecto se muestra en la página del dispositivo con el motivo
y no se notifica.

**Las colas vienen del agente, no de este dispositivo.** MDaemon publica
los tamaños de sus colas (entrante, local, remota, reintento, mala,
retención, cuarentena), sus recuentos de sesión, sus estadísticas de
mensajes y su tiempo de actividad como contadores de rendimiento de
Windows, en el objeto llamado MDaemon, y en su propio Queue and
Statistics Manager. Ninguna de la documentación publicada de MDaemon
describe una llamada de API XML que los devuelva, así que este
dispositivo no inventa una: el [agente](agent.md), instalado en el
servidor MDaemon, lee los contadores de rendimiento en su lugar.
Consulta [Colas de correo a través del agente](#mail-queues-through-the-agent).
La licencia y el espacio en disco del almacén de correo no se exponen a
través de ninguna interfaz documentada ni contador de rendimiento, y no
se leen. El agente también vigila el disco que contiene el almacén de
correo; lista el servicio MDaemon en su ajuste `services` para vigilar
el propio servicio de Windows.

Las [reglas integradas](../alerting/rules.md#mdaemon-and-securitygateway)
que se aplican: MDaemon mail service down, MDaemon XML API not
answering, además de Device unreachable; con el agente, MDaemon mail
queue growing, MDaemon Bad queue not empty y MDaemon Retry queue high.

## La página del dispositivo {#the-device-page}

Un dispositivo MDaemon muestra un panel **Mail services** sobre las
gráficas, leído de la última medición guardada: abrir la página nunca se
conecta al servidor. Lista cada servicio vigilado con su puerto, una
palabra y un color (Answering, Down), y su tiempo de respuesta; luego la
versión y de dónde se leyó. Los servicios detenidos van primero.

Cuando el agente en el mismo servidor reporta los contadores de
MDaemon, sigue un panel **Mail queues**: el número de mensajes en cada
cola (una cola congelada, una cola Bad no vacía y más de cincuenta
mensajes en Retry se marcan con palabras), las sesiones activas, los
mensajes y veredictos de spam, virus y DNSBL de las últimas 24 horas, el
tiempo de actividad de MDaemon, y los servidores internos que MDaemon
reporta como inactivos. El mismo panel aparece en la propia página del
agente.

## Colas de correo a través del agente {#mail-queues-through-the-agent}

1. Instala el [agente](agent.md#install) en el servidor Windows que
   ejecuta MDaemon. No se necesita nada más: el agente detecta el
   servicio de Windows llamado `MDaemon` y lee los contadores del objeto
   de rendimiento MDaemon en cada período de muestreo. Para forzarlo
   activado o desactivado, define `mdaemon: true` o `mdaemon: false` en
   `agent.yaml`.
2. Las colas aparecen en la página del agente. Para mostrarlas también
   en este dispositivo MDaemon, DumbMonit busca el agente en la misma
   máquina: el agente definido como padre de este dispositivo, o el
   agente relay que lo sondea, o el agente cuyo nombre de host coincide
   con la dirección de este dispositivo (`mail` para
   `mail.example.com`), o, cuando hay un dispositivo MDaemon y un agente
   reportando los contadores de MDaemon, ese agente. Si ninguno
   coincide, edita este dispositivo y elige el agente como su
   **Parent device**: el agente y MDaemon comparten la máquina, así que
   cuando el agente se queda en silencio las alertas de este dispositivo
   también se retienen.

Las series pertenecen al agente, con su etiqueta `target`. Todas tienen
el prefijo `dumbmonit_mdaemon_`:

| Métrica | Etiquetas | Contadores de rendimiento leídos |
|---|---|---|
| `queue_messages` | `queue`: `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | `Inbound queue messages`, `Local queue messages`, `Remote queue messages`, `Retry queue messages`, `Bad queue messages`, `Holding queue messages`, `LAN queue messages`, `Quarantine queue messages`, `RAW queue messages` |
| `queue_frozen` (1 congelada) | `queue`: `inbound`, `local`, `remote` | `Inbound queue frozen`, `Local queue frozen`, `Remote queue frozen` |
| `sessions_active` | `protocol`: `smtp_in`, `smtp_out`, `pop3_in`, `pop3_out`, `imap`, `webmail` | `Active SMTP (in) sessions`, `Active SMTP (out) sessions`, `Active POP3 (in) sessions`, `Active POP3 (out) sessions`, `Active IMAP sessions`, `Active Webmail sessions` |
| `sessions_total` (contador) | `protocol`: `smtp_in`, `smtp_out`, `pop3`, `imap` | `SMTP sessions (in) total`, `SMTP sessions (out) total`, `POP3 sessions total`, `IMAP sessions total` |
| `messages_total` (contador) | `protocol`: `smtp_in`, `smtp_out`, `domainpop_in` | `SMTP messages (in) total`, `SMTP messages (out) total`, `DomainPOP messages (in) total` |
| `filtered_messages_total` (contador) | `filter`: `spam`, `virus`, `dnsbl`; `verdict`: `accepted`, `refused` | `spam accepted total`, `spam refused total`, `Viruses accepted total`, `Viruses refused total`, `DNSBL accepted total`, `DNSBL refused total` |
| `server_active` (1 activo) | `server`: `smtp`, `pop3`, `imap`, `webmail`, `webadmin`, `activesync`, `antispam`, `antivirus`, `minger`, `multipop`, `domainpop` | `SMTP server state`, `POP3 server state`, `IMAP server state`, `Web Mail server state`, `Web Admin server state`, `ActiveSync server state`, `AntiSpam server state`, `AntiVirus server state`, `Minger server state`, `MultiPOP server state`, `DomainPOP server state` |
| `running` (1 en ejecución) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

Los totales cuentan desde el último arranque de MDaemon: aplica
`increase()` o `rate()` en las consultas, como hace la página para sus
cifras de 24 horas. Los contadores por segundo (`SMTP messages
(in)/sec`…) no se leen, ya que los totales dan las mismas tasas. Un
contador que tu versión de MDaemon no tiene, o nombra diferente, se
reporta una vez en el registro del agente (`performance counter not
available`) y los demás se siguen leyendo; para leer uno que el preset
no incluye, lístalo bajo `perf_counters` en `agent.yaml` (consulta
[Contadores de rendimiento de Windows](agent.md#windows-performance-counters)).

## Vigilar los servicios de correo, y opcionalmente la API XML {#watch-the-mail-services-and-optionally-the-xml-api}

1. Nada que instalar en el servidor. Sin ninguna credencial, DumbMonit
   se conecta a los puertos de correo desde fuera y lee cada saludo:
   SMTP (25), IMAP (143) y Webmail (3000) por defecto. Añade POP3, MSA,
   los puertos TLS o Remote Administration en la opción Services, como
   una lista así.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. Para leer también la versión a través de la API XML, crea una cuenta
   dedicada en MDaemon, nombrada por ejemplo como sigue, con una
   contraseña larga usada en ningún otro sitio. Dale el nivel más bajo
   que tu MDaemon acepte para la API XML, y nada más: DumbMonit solo
   llama a GetVersionInfo, que no lee ninguna bandeja y no cambia nada.

    ```
    dumbmonit@example.com
    ```

3. Desde MDaemon 24, la API XML solo responde a las direcciones que
   permite. Abre Setup → XML API Service → Address Restrictions (Setup →
   XML API Management en MDaemon 26) y permite la dirección desde la
   que se conecta DumbMonit.

4. Comprueba desde el host de DumbMonit que la cuenta alcanza la API. El
   comando pide la contraseña e imprime una respuesta que empieza con
   `<MDaemon><API productversion=…>`.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. En DumbMonit, introduce la dirección del servidor, por ejemplo
   «mail.example.com», luego la dirección de correo completa de la
   cuenta como nombre de usuario, y su contraseña. La API XML se
   alcanza en el puerto HTTPS de Remote Administration, 444 por defecto.

!!! warning
    MDaemon no tiene ningún rol de solo lectura para la API XML: nunca
    reutilices tu propia cuenta, y mantén esta contraseña fuera de
    cualquier otra herramienta. Dynamic Screening puede bloquear una
    dirección tras varios inicios de sesión fallidos, así que una
    contraseña incorrecta aquí puede hacer que se bloquee el host de
    DumbMonit: prueba primero con el comando de arriba. Remote
    Administration a menudo usa un certificado autofirmado: si la
    conexión se rechaza por ese motivo, marca «Accept an unverifiable
    certificate» en las opciones. Los tamaños de cola de correo no los
    lee este dispositivo: MDaemon solo los publica como contadores de
    rendimiento de Windows, que el agente de DumbMonit lee cuando está
    instalado en el servidor de correo.

La API XML la sirve el servidor web de Remote Administration: puerto
1000 por HTTP y 444 por HTTPS por defecto, en `/MdMgmtWS/`, o por IIS
cuando Remote Administration se ejecuta ahí. Su referencia completa
viene con el servidor: abre `https://mail.example.com:444/MdMgmtWS` en
un navegador, o busca en `\MDaemon\Docs\API\XML API\`.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Solo servicios | Nada: los puertos de correo se comprueban sin iniciar sesión. |
| Cuenta de API XML | La dirección de correo completa de la cuenta, y su contraseña, enviadas como autenticación básica HTTP. |

Dirección: el nombre de host o IP del servidor, por ejemplo
`mail.example.com`. Un puerto escrito en la dirección
(`mail.example.com:8444`) se toma como el puerto de la API XML.

## Opciones {#options}

| Opción | Por defecto | Efecto |
|---|---|---|
| `services` | `smtp,imap,webmail` | Qué comprobar, separado por comas: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `nombre:puerto` cambia un puerto; cualquier otro nombre con su puerto añade un servicio propio, comprobado solo por conexión. `none` no comprueba ningún puerto. |
| `request_timeout_seconds` | `10` | Tiempo permitido para cada conexión, saludo y llamada a la API, de 1 a 60. |
| `api_port` | `444` | El puerto de Remote Administration que sirve `/MdMgmtWS/`. Solo se usa con una cuenta. |
| `api_tls` | `true` | HTTPS hacia la API XML. Desmárcalo solo para Remote Administration por HTTP plano (puerto 1000 por defecto): la contraseña entonces cruza la red en claro. |
| `insecure_tls` | `false` | Aceptar un certificado autofirmado en la API XML. |

Los puertos TLS (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) solo se
comprueban por conexión. Para vigilar la caducidad de sus certificados,
añade un dispositivo [certificado TLS](services.md#tls) para cada uno.
