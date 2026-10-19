# Lista de bloqueo de un rango de IP {#ip-range-blocklist}

Todas las direcciones de un rango IPv4 público, hasta un /24 (256 direcciones),
comprobadas frente a las listas de bloqueo DNS de [Reputación del servidor de correo](mail_blacklist.md#the-blocklists):
Spamhaus ZEN, Barracuda, SpamCop, Mailspike, UCEPROTECT Nivel 1, PSBL y
SpamRATS. La página del monitor indica cuántas direcciones están incluidas, en
qué listas, y da un enlace directo a la página de consulta y retirada de cada
inclusión.

Resulta útil para el rango público de una oficina, el bloque que te dio un
proveedor de alojamiento o las direcciones desde las que envían tus servidores:
la máquina comprometida de un vecino consigue una inclusión que un receptor más
estricto puede extender a todo el rango.

## Límites {#limits}

- **Como máximo un /24 por monitor.** Un rango mayor se rechaza con un mensaje
  que indica cuántas direcciones contiene: divídelo en varios monitores /24.
  Cada dirección cuesta una consulta a cada lista, y un barrido de miles de
  direcciones les parece un abuso.
- **Solo IPv4.** De estas listas, solo Spamhaus ZEN responde para IPv6, y un
  rango IPv6 no se puede recorrer dirección por dirección. Una única dirección
  IPv6 se puede comprobar con [Reputación del servidor de correo](mail_blacklist.md).
- **Solo direcciones públicas.** Se rechaza un rango que incluya una dirección
  privada, compartida (CGNAT) o de documentación: las listas de bloqueo solo
  conocen la Internet pública.
- **Las listas de dominio** (Spamhaus DBL, SURBL, URIBL) no se aplican a un
  rango.

## Resolvedores y ritmo {#resolvers-and-pace}

Se aplican las mismas reglas que en [Reputación del servidor de correo](mail_blacklist.md#resolvers-refusals-and-timeouts).
**No hace falta ninguna clave de API.** Solo Spamhaus rechaza los resolvedores
públicos: su rechazo (`127.255.255.254`) nunca se muestra como una inclusión, la
página del monitor indica «Spamhaus refuses this resolver» y las demás listas
conservan sus resultados. Una lista que no responde es desconocida, nunca limpia.
Para Spamhaus también, usa un resolvedor recursivo propio, o la clave DQS gratuita
opcional de Spamhaus como credencial.

Las listas son las marcadas en **Blocklists** (por defecto las principales listas de
direcciones) más tus propias zonas en **Custom lists**; ver
[Tus propias listas](mail_blacklist.md#your-own-lists). Las listas de dominios no se
aplican a un rango.

Un /24 requiere cerca de dos mil consultas. Se envían a **Queries per second**
(10 por defecto), con ocho como máximo en vuelo, y todo el rango se comprueba una
vez por **Refresh interval** (6 horas por defecto); el último resultado se vuelve
a mostrar en cada comprobación intermedia. La primera comprobación de un rango
tarda unos minutos y se ejecuta en segundo plano.

## Comprobar un rango de IP público frente a las listas de bloqueo {#check-a-public-ip-range-against-the-blocklists}

1. Introduce una dirección IPv4 o un rango CIDR de hasta /24, por ejemplo "198.51.100.0/24" (256 direcciones). Un rango mayor se rechaza: divídelo en varios monitores /24. Los rangos IPv6 se rechazan, ya que solo Spamhaus responde para IPv6: comprueba una única dirección IPv6 con Reputación del servidor de correo.

2. No hace falta ninguna clave de API: deja la credencial en No key. Las listas de bloqueo se consultan a través del resolvedor DNS de este servidor, o del que fijes en Resolver. Solo Spamhaus puede rechazar, y solo cuando ese resolvedor es público (Google, Cloudflare, Quad9): el monitor muestra entonces «Spamhaus refuses this resolver» y las demás listas siguen funcionando. Para Spamhaus también, usa un resolvedor recursivo propio o la clave DQS gratuita opcional de Spamhaus.

3. Marca en Blocklists las listas de bloqueo de direcciones que consultar (las principales vienen marcadas por defecto) y añade en Custom lists las zonas DNSBL de cualquier otra lista. Un /24 requiere cerca de dos mil consultas con todas las listas de direcciones marcadas, enviadas a 10 por segundo y repetidas cada 6 horas por defecto: mantén ese ritmo, las listas de bloqueo vetan a los resolvedores que las barren.

4. La primera comprobación de un rango tarda unos minutos: la página del monitor indica que está en curso y luego muestra cuántas direcciones están incluidas, en qué listas, y un enlace de retirada para cada una.

!!! warning
    Spamhaus PBL incluye rangos de usuarios finales que no deberían enviar correo directamente (conexiones domésticas, pools dinámicos): una inclusión ahí es una política, no un informe de abuso, y solo importa para una dirección que ejecuta un servidor de correo.

## Qué vigila {#what-it-watches}

Las mismas métricas que [Reputación del servidor de correo](mail_blacklist.md#what-it-watches),
con el prefijo `dumbmonit_blacklist_`, sin las de DNS inverso, SPF y DMARC. Para
mantener pequeño el número de series, `check` solo se escribe para una dirección
incluida o a la que una lista no respondió; `list_checked`, `list_listed` y
`list_unknown` cuentan todas las direcciones por lista.

Las [reglas integradas](../alerting/rules.md#dns-blocklists) que se aplican:
Listed on a major blocklist y Listed on a blocklist.

La página del monitor tiene un enlace **Also check on MXToolbox**: para un rango,
junto a cada dirección incluida, abre en una pestaña nueva la consulta pública de
listas de bloqueo de mxtoolbox.com. DumbMonit no llama a su API.

## Credenciales {#credentials}

Ninguna hace falta (el valor por defecto), u opcionalmente una clave DQS de Spamhaus.

Dirección: una dirección IPv4 o un rango CIDR de hasta /24. Opciones: Resolver,
Blocklists (las principales marcadas en un monitor nuevo), Custom lists, Refresh
interval (6 horas), Queries per second (10) y Timeout per query (5 s).
