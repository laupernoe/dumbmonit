# Reputación del servidor de correo {#mail-server-reputation}

Un dominio de correo, o la dirección pública de un servidor de correo,
comprobado frente a las listas de bloqueo DNS que aún deciden si el correo se
entrega: Spamhaus, Barracuda, SpamCop y algunas menores. Por cada lista que
incluye una dirección, la página del monitor da el código de respuesta de la
lista, su motivo publicado y un enlace directo a su página de consulta y
retirada.

Introduce un dominio (`example.com`) y DumbMonit comprueba las direcciones de sus
hosts MX (registros A y AAAA), las direcciones del propio dominio y las
**Extra addresses** que añadas (un relé de salida que no es un MX). El dominio
también se comprueba frente a las listas de bloqueo de dominios. Introduce una
dirección IP y solo se comprueba esa dirección. Para vigilar todo un rango de
direcciones, usa [Lista de bloqueo de un rango de IP](ip_blacklist.md).

## Gmail, Outlook.com y Yahoo {#gmail-outlook-com-and-yahoo}

Los grandes proveedores de buzones **no publican ninguna lista de bloqueo que se
pueda consultar**: lo que piensan de un remitente es interno. DumbMonit no
finge preguntarles. Comprueba lo que pesa en su decisión y enlaza con sus
herramientas:

- **DNS inverso**: cada dirección de envío debería tener un registro PTR cuyo
  nombre resuelva de vuelta a la misma dirección (DNS inverso confirmado hacia
  delante, FCrDNS). Gmail, Outlook.com y Yahoo rechazan o mandan a spam el
  correo de direcciones que no lo tienen.
- **SPF y DMARC**: exactamente un registro SPF en el dominio, y un registro DMARC
  en `_dmarc.<dominio>`. Gmail y Yahoo exigen ambos a los remitentes masivos.
- **Las listas de bloqueo de abajo**, que sus filtros también leen.

DKIM no se comprueba: su selector lo elige cada remitente y no se puede
adivinar. La página del monitor enlaza con:

| Proveedor | Herramientas |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/) (reputación de un dominio verificado), [directrices para remitentes](https://support.google.com/a/answer/81126), [formulario de problemas de entrega](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/) (datos de reputación por dirección), [políticas para remitentes](https://sendersupport.olc.protection.outlook.com/pm/), [formulario de soporte para remitentes](https://olcsupport.office.com/) (retirada de Outlook.com), [portal de retirada de Microsoft 365](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/), [buenas prácticas](https://senders.yahooinc.com/best-practices/), [formulario de soporte para remitentes](https://senders.yahooinc.com/contact/) |

## Las listas de bloqueo {#the-blocklists}

Comprobado el 16 de octubre de 2026. SORBS cerró en junio de 2024 y no se
consulta. A cada lista se le pide un registro `A` bajo su zona; una lista se
identifica por el id de la primera columna, que la opción **Lists** admite para
restringir la comprobación.

| Id | Lista | Zona | Comprueba | IPv6 | Principal | Uso y retirada |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | direcciones | sí | sí | Espejos públicos gratuitos para uso de bajo volumen y no comercial; rechaza los resolvedores públicos y abiertos. Retirada desde el [verificador de Spamhaus](https://check.spamhaus.org/). PBL incluye rangos de usuarios finales por política: retira una dirección de ella solo si de verdad ejecuta un servidor de correo. |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | direcciones | no | sí | Gratuita; Barracuda pide que los servidores DNS que la consultan estén registrados con una cuenta gratuita en barracudacentral.org. [Solicitud de retirada](https://www.barracudacentral.org/rbl/removal-request). |
| `spamcop` | SpamCop | `bl.spamcop.net` | direcciones | no | sí | Gratuita. Las inclusiones caducan solas aproximadamente un día después de que cesen los informes; la [página de consulta](https://www.spamcop.net/bl.shtml) muestra el motivo y ofrece la retirada. |
| `mailspike` | Mailspike | `bl.mailspike.net` | direcciones | no | no | Gratuita con bajo volumen. [Consulta y retirada](https://mailspike.io/ip_verify). |
| `uceprotect_1` | UCEPROTECT Nivel 1 | `dnsbl-1.uceprotect.net` | direcciones | no | no | Consulta gratuita. Las inclusiones caducan tras siete días sin nuevo spam; la [página de comprobación](https://www.uceprotect.net/en/rblcheck.php) también vende una retirada exprés, que nunca es necesaria. Pocos receptores la usan. |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | direcciones | no | no | Gratuita. Retirada por autoservicio en la [página de inclusión](https://psbl.org/). |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | direcciones | no | no | Gratuita con bajo volumen. RATS-NoPtr incluye direcciones sin DNS inverso. [Consulta](https://www.spamrats.com/lookup.php) y [retirada](https://www.spamrats.com/removal.php). |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | el dominio | — | sí | Mismas condiciones que ZEN. Retirada desde el [verificador de Spamhaus](https://check.spamhaus.org/). |
| `surbl` | SURBL (multi) | `multi.surbl.org` | el dominio | — | no | Gratuita con bajo volumen; rechaza los resolvedores públicos y muy cargados. [Consulta y retirada](https://www.surbl.org/surbl-analysis). |
| `uribl` | URIBL (multi) | `multi.uribl.com` | el dominio | — | no | Gratuita con bajo volumen; rechaza los resolvedores públicos y muy cargados. [Consulta](https://lookup.uribl.com/), [retirada](https://admin.uribl.com/). |

Una inclusión en una lista **principal** dispara una alerta crítica; una en las
demás listas, una de aviso. Las direcciones IPv6 de los MX solo se comprueban
frente a Spamhaus ZEN, la única lista de aquí que responde para IPv6; las demás
las muestran como no cubiertas.

## Resolvedores, rechazos y tiempos de espera {#resolvers-refusals-and-timeouts}

Las listas se consultan por DNS, y cómo responden depende de **qué resolvedor
pregunta**:

- **Spamhaus rechaza las consultas que llegan a través de resolvedores
  públicos** (Google `8.8.8.8`, Cloudflare `1.1.1.1`, Quad9), a través de
  resolvedores que reenvían a ellos, o a través de resolvedores sin un DNS
  inverso atribuible. Entonces responde `127.255.255.254` en lugar de
  `NXDOMAIN` (`127.255.255.255` cuando un resolvedor envía demasiadas consultas,
  `127.255.255.252` para una consulta mal formada o una clave DQS errónea). Esto
  parece una inclusión y no lo es: DumbMonit muestra la comprobación como un
  **error de configuración**, nunca como «incluida», y no envía ninguna alerta de
  caída.
- **URIBL y SURBL responden `127.0.0.1`** a los resolvedores que rechazan. Esa
  lista se muestra entonces como «Refused» (desconocida), no como una inclusión.
- **Una lista que no responde a tiempo es desconocida, nunca limpia.** Solo
  `NXDOMAIN` significa «no incluida». Una respuesta fuera de `127.0.0.0/8` (un
  resolvedor que reescribe `NXDOMAIN` con la dirección de una página de
  búsqueda) también es desconocida.

Así que usa **un resolvedor recursivo propio**: Unbound, Knot Resolver, BIND o
PowerDNS Recursor resolviendo desde los servidores raíz (sin reenviar a un
resolvedor público), y fija su dirección en **Resolver** si no es el resolvedor
del sistema. Pi-hole o AdGuard Home delante de Unbound funciona; delante de
`1.1.1.1` no.

O crea una cuenta gratuita del [Spamhaus Data Query Service](https://www.spamhaus.com/data-access/free-data-query-service/)
(uso no comercial, bajo volumen) e introduce su clave como credencial: entonces
se consulta a Spamhaus bajo `<clave>.zen.dq.spamhaus.net` y
`<clave>.dbl.dq.spamhaus.net`, que responden a través de cualquier resolvedor.
La clave se guarda cifrada y no se vuelve a mostrar.

## Ritmo {#pace}

Las listas de bloqueo limitan la frecuencia con que un resolvedor puede
preguntar, y vetan a los resolvedores que las barren. DumbMonit pregunta a cada
lista una vez por **Refresh interval** (6 horas por defecto), como máximo
**Queries per second** (10 por defecto) con ocho consultas en vuelo, y vuelve a
mostrar el último resultado en cada comprobación intermedia, de modo que las
reglas de alerta ven una serie continua. Una comprobación se ejecuta en segundo
plano: el primer resultado aparece unos segundos después de añadir el monitor, y
cambiar la dirección, una opción o la clave inicia enseguida una nueva
comprobación. Una comprobación cuyo resultado es un error se reintenta tras diez
minutos.

## Comprobar un servidor de correo frente a las listas de bloqueo {#check-a-mail-server-against-the-blocklists}

1. Introduce el dominio para el que envías correo, por ejemplo "example.com": DumbMonit comprueba las direcciones de sus MX y del propio dominio. O introduce la dirección IP pública desde la que envía el servidor. Si el correo sale por un relé que no es un MX, añade su dirección en Extra addresses.

2. Usa un resolvedor recursivo propio. Spamhaus rechaza las consultas que llegan por resolvedores públicos (Google, Cloudflare, Quad9) o por un resolvedor que reenvía a ellos, y otras listas las limitan: basta un Unbound que resuelva desde los servidores raíz. Fija su dirección en Resolver si no es el resolvedor del sistema.

3. O crea una cuenta gratuita del Spamhaus Data Query Service (uso no comercial, bajo volumen) y pega su clave como credencial: Spamhaus responde entonces a través de cualquier resolvedor.

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

4. Para comprobar la configuración, añade un monitor para 127.0.0.2, la dirección de prueba que toda lista de bloqueo incluye: cada una debería mostrarla como incluida.

5. Gmail, Outlook.com y Yahoo no publican ninguna lista de bloqueo que se pueda consultar. DumbMonit comprueba en su lugar lo que ellos pesan (un DNS inverso que apunta de vuelta a la dirección, registros SPF y DMARC) y enlaza con sus herramientas para remitentes en la página del monitor.

!!! warning
    Una respuesta que Spamhaus rechazó (127.255.255.254, un resolvedor público) se muestra como un error de configuración, nunca como una inclusión, y una lista de bloqueo que no responde se muestra como desconocida, nunca como limpia. Las listas se consultan cada 6 horas por defecto: preguntar más a menudo no aporta nada y hace que veten a los resolvedores.

## Qué vigila {#what-it-watches}

Todas las métricas llevan el prefijo `dumbmonit_blacklist_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `listed_major_lists` | listas principales (Spamhaus ZEN y DBL, Barracuda, SpamCop) que incluyen una dirección o el dominio | |
| `listed_lists` | todas las listas que incluyen una dirección o el dominio | |
| `addresses_checked` | direcciones comprobadas | |
| `addresses_listed` | direcciones incluidas en al menos una lista | |
| `domain_listed` | 1 cuando el dominio está en una lista de bloqueo de dominios | |
| `unknown_checks` | comprobaciones sin respuesta (tiempo de espera, rechazo) | |
| `check` | una serie por dirección (o dominio) y lista: 1 incluida, 0 limpia, −1 sin respuesta, −2 rechazada, −3 no cubierta | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | por lista: sujetos consultados, incluidos, sin respuesta | `list` |
| `address_info` | valor 1, de dónde viene cada dirección | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 1 cuando el DNS inverso resuelve de vuelta a la dirección | `address`, `ptr` |
| `mx_hosts` | hosts MX del dominio | |
| `spf_records` | registros `v=spf1` en el dominio (1 es lo correcto) | |
| `dmarc_records` | registros `v=DMARC1` en `_dmarc.<dominio>` | |
| `dmarc_policy_info` | valor 1 | `policy` |
| `mail_auth_missing` | 0 a 2: SPF ausente o duplicado, DMARC ausente | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | la comprobación mostrada: su antigüedad, cuánto tardó, si hay una nueva en curso | |

Las [reglas integradas](../alerting/rules.md#dns-blocklists) que se aplican:
Listed on a major blocklist, Listed on a blocklist, Mail server reverse DNS
mismatch, SPF or DMARC missing, y además Monitor unreachable cuando el
resolvedor no responde a ninguna de las consultas.

## La página del monitor {#the-monitor-page}

El panel lee lo que guardó el sondeo; abrir la página nunca consulta una lista
de bloqueo. Muestra los recuentos, luego cada lista con cuántos sujetos incluye,
luego cada resultado con el código de la lista, su motivo publicado y enlaces a
sus páginas de consulta y retirada, luego el DNS inverso de cada dirección, SPF
y DMARC, y las herramientas para remitentes de Gmail, Outlook.com y Yahoo.

## Credenciales {#credentials}

Ninguna, o una clave DQS de Spamhaus (ver arriba).

Dirección: un dominio (`example.com`) o una dirección IP pública. Opciones:
Resolver, Lists (todas por defecto), Extra addresses, Refresh interval (6 horas,
de 1 a 168), Queries per second (10, de 1 a 50) y Timeout per query (5 s, de 1 a
30). Las direcciones privadas nunca se comprueban: las listas de bloqueo solo
conocen la Internet pública. `127.0.0.2` se acepta como dirección de prueba.
