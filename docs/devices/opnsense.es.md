# OPNsense

Cortafuegos y router de código abierto: el estado de cada gateway con su
tiempo de ida y vuelta y pérdida de paquetes, la tabla de estados pf,
contadores y direcciones de interfaz, túneles VPN, concesiones DHCP, el
resolutor, servicios, CARP y actualizaciones pendientes.

!!! warning "Probado solo contra una instancia de prueba"

    Esta integración se construyó y verificó contra OPNsense 26.1, la
    imagen oficial iniciada en una máquina virtual para este propósito,
    con la clave de API restringida descrita abajo — no contra un
    cortafuegos que lleva años enrutando el tráfico de una familia.
    Cada llamada es de lectura y cada forma viene de ese OPNsense real,
    pero sus gateways no tenían dirección de monitor, no formaba parte
    de un par CARP y no ejecutaba ningún túnel OpenVPN o IPsec: esas
    partes siguen el código fuente de OPNsense en lugar de una respuesta
    observada. Dinos qué falla.

## La página del dispositivo {#the-device-page}

Más allá de las gráficas genéricas, un dispositivo OPNsense muestra tres
paneles, leídos de lo que guardó la sonda — abrir la página nunca
consulta al propio cortafuegos:

* **WAN and gateways** — la dirección pública actual de cada interfaz
  que lleva un gateway, luego cada gateway con su estado en una
  palabra, su retraso de ida y vuelta y su pérdida de paquetes. Los
  gateways caídos van primero. Este es el panel por el que existe la
  integración: en un cortafuegos multi-WAN, el enlace de respaldo toma
  el control silenciosamente y nada más en la red cambia.
* **Traffic and state table** — cuántas conexiones está rastreando pf
  frente a su límite configurado, luego cada interfaz con su estado de
  enlace, sus direcciones, sus contadores de bytes y paquetes, y sus
  errores y descartes cuando los hay. Luego las concesiones DHCP,
  contadas por servidor.
* **Firewall health** — primero los servicios detenidos, luego los
  túneles VPN con el número de pares conectados, luego CARP cuando el
  cortafuegos es la mitad de un par, luego el firmware (versión en
  ejecución, lo que ofrece el espejo, paquetes esperando, y si hay un
  reinicio pendiente), luego el resolutor, luego la máquina: tiempo de
  actividad, carga, memoria, swap, buffers de red, particiones y
  sensores de temperatura.

Solo recuentos, nunca tráfico. DumbMonit no lee ninguna regla de
cortafuegos, ningún estado de conexión, ninguna dirección de cliente ni
nombre de host. Las concesiones DHCP se cuentan, nunca se listan; las
sesiones VPN se cuentan, nunca se nombran.

## «none» significa «en línea» {#none-means-online}

`GET /api/routes/gateway/status` escribe `"none"` en el campo `status`
de un gateway cuando **todo va bien** — una palabra que por sí sola se
lee como «sin información». DumbMonit la traduce a `online`, así que la
página, las series y las reglas dicen lo mismo que la propia interfaz de
OPNsense. `down` y `force_down` son los fallos; `loss` y `delay` son
avisos de dpinger, y el gateway sigue respondiendo.

La misma llamada devuelve el retraso, pérdida y desviación estándar
como **cadenas con su unidad** (`"8.4 ms"`, `"0.0 %"`), y como `"~"`
cuando el gateway no tiene dirección de monitor. `"~"` produce una serie
menos, nunca un cero: un gateway al que nadie hace ping no tiene cero
milisegundos de latencia, no tiene ninguna en absoluto.

## La comprobación de actualización nunca se activa {#the-update-check-is-never-triggered}

`GET /api/core/firmware/status` devuelve el resultado de la *última*
comprobación del cortafuegos — la que muestra el propio panel de
OPNsense. DumbMonit nunca envía la forma `POST` de esa llamada, ni
`/api/core/firmware/check`, que enviaría al cortafuegos al espejo en
cada medición, desde cada instalación de DumbMonit.

Un cortafuegos que nunca ha comprobado — o que acaba de instalar una
actualización, lo cual limpia la caché — responde `"status": "none"`,
exactamente como lo hace uno al día. DumbMonit distingue los dos, y
hasta que se ha ejecutado una comprobación publica `firmware_checked =
0` y nada más: sin «zero updates pending», sin «no reboot needed».
Ejecuta una comprobación una vez desde System → Firmware, o deja que la
propia programación del cortafuegos lo haga.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_opnsense_`.

| Familia | Métricas | Etiquetas |
|---|---|---|
| Identidad | `version_info` (las etiquetas llevan la versión de OPNsense, la versión de FreeBSD y la línea de producto) | |
| Máquina | `uptime_seconds`, `load1/5/15`, `cpu_count`, `memory_used/total_bytes`, `memory_used_percent` (la memoria incluye la caché ZFS, que FreeBSD cuenta como usada), `swap_used/total_bytes`, `swap_used_percent` (ausente en una máquina sin swap), `mbuf_used` y `mbuf_total` (clusters de buffer de red en uso frente a `kern.ipc.nmbclusters`), `mbuf_used_percent`, `mbuf_failures` (asignaciones rechazadas desde el arranque, un contador) | |
| Almacenamiento | `disk_used_bytes`, `disk_total_bytes`, `disk_used_percent` | `device`, `mountpoint` |
| Sensores | `temperature_celsius` | `sensor` |
| Gateways | `gateway_up` (0 solo en `down` y `force_down`), `gateway_monitored`, `gateway_delay_seconds`, `gateway_stddev_seconds`, `gateway_loss_percent`, `gateways_total`, `gateways_down` | `gateway`, `address` |
| Interfaces | `interface_up`, `interface_bytes_in/out`, `interface_packets_in/out`, `interface_errors_in/out`, `interface_drops`, `interface_collisions` (contadores, guardados en bruto), `interface_address_info` (la dirección del momento, en una etiqueta) | `interface`, `device`, y `address` para la última |
| Firewall | `pf_states`, `pf_state_limit`, `pf_states_used_percent` | |
| DHCP | `dhcp_leases`, `dhcp_leases_active` | `backend` (`kea`, `isc`, `dnsmasq`) |
| VPN | `vpn_tunnel_up`, `vpn_peers_total`, `vpn_peers_connected` (WireGuard: handshake dentro de tres minutos), `vpn_handshake_age_seconds` (WireGuard, el par más reciente; ausente mientras ningún par se haya conectado nunca), `vpn_bytes_in/out`, `vpn_tunnels_down` | `kind` (`wireguard`, `openvpn`, `ipsec`), `tunnel` (el dispositivo WireGuard, `wg0`, o la descripción de IPsec y OpenVPN) |
| Resolutor | `unbound_running`, `unbound_queries` (un contador), `unbound_cache_hit_percent` | |
| Servicios | `service_running` (1 o 0, una serie por servicio que lista el cortafuegos) | `service` |
| CARP | `carp_enabled`, `carp_maintenance_mode`, `carp_vip_master` (todos ausentes en un cortafuegos que no forma parte de un par) | `interface`, `vhid`, `address` |
| Firmware | `firmware_checked`, luego — solo una vez ejecutada una comprobación — `firmware_updates_pending`, `firmware_upgrade_available`, `firmware_reboot_required`, `firmware_connection_ok` | |
| Recolección | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Los contadores de interfaz se guardan en bruto, como contadores:
envuélvelos en `rate()` para obtener caudal. Hacerlo así significa que
un reinicio del cortafuegos se muestra como un hueco, no como un pico de
varios gigabytes por segundo.

`gateway_up` es 1 para un gateway que no se vigila en absoluto, porque
no hay nada que diga lo contrario; `gateway_monitored` es lo que
distingue a los dos. Un gateway sin dirección de monitor no produce ni
una serie de retraso ni de pérdida.

Un cortafuegos que no forma parte de un par de alta disponibilidad no
tiene dirección virtual CARP, y ninguna serie de CARP en absoluto. Eso
es un cortafuegos sin clúster, no un clúster degradado: ninguna regla
puede activarse.

WireGuard, OpenVPN, IPsec, Unbound y Kea son plugins. Un cortafuegos que
no tiene uno responde `404`, que no es ni un error de recolección ni un
error de autenticación: una métrica menos, y nada en la interfaz.

Las reglas integradas que se aplican: Device unreachable, Internet
gateway down, Gateway losing packets, Gateway slow, Firewall state
table filling up, Firewall network buffers exhausted, VPN tunnel down,
Firewall resolver stopped, Firewall core service stopped, Firewall too
hot, Firewall left in CARP maintenance mode, Firewall reboot pending,
Firewall updates pending. Las notificaciones nombran el gateway, túnel,
servicio o sensor afectado.

## Qué preparar en OPNsense {#what-to-prepare-in-opnsense}

Los pasos de abajo son los que muestra el aviso junto al formulario. El
principio: un grupo solo con los privilegios de estado que necesita
DumbMonit, un usuario que nunca inicia sesión, y una clave de API que no
pertenece a ninguna de tus propias cuentas. Este grupo exacto se
comprobó contra cada llamada que hace DumbMonit, en OPNsense 26.1.

1. En la interfaz web: System → Access → Groups → Add. Nombra el grupo
   como sigue.

    ```
    dumbmonit
    ```

2. Marca estos privilegios en el grupo, exactamente como OPNsense los
   nombra: Lobby: Dashboard (sistema, memoria, discos, temperaturas,
   tabla de estados), System: Gateways, Status: Interfaces, Status:
   Services, System: Firmware, Interfaces: Virtual IPs: Status (CARP) y
   Services: Unbound (MVC). Luego uno por cada función que uses:
   Services: DHCP: Kea(v4) o Services: Dnsmasq DNS/DHCP: Settings para
   las concesiones, VPN: WireGuard: Status, Status: OpenVPN, Status:
   IPsec. Un privilegio que omitas cuesta exactamente las métricas que
   lleva, nada más.

3. System → Access → Users → Add. Nombra la cuenta como sigue, deja que
   OPNsense genere una contraseña aleatoria (esta cuenta nunca inicia
   sesión: responde con su clave), y hazla miembro del grupo dumbmonit.

    ```
    dumbmonit
    ```

4. Edita de nuevo ese usuario y, bajo API keys, haz clic en +. OPNsense
   descarga un pequeño archivo de texto con dos líneas, key= y secret=,
   y muestra el secreto solo esta vez. Copia la key en el campo API key
   de DumbMonit y el secret en su campo API secret.

5. En DumbMonit, introduce la dirección del cortafuegos, por ejemplo
   «192.168.1.1» o «fw.lan:8443». Nunca se pide al cortafuegos que
   compruebe actualizaciones: DumbMonit lee el resultado de la
   comprobación que OPNsense ejecuta en su propia programación, o que
   inicias desde System → Firmware.

!!! warning

    No reutilices la cuenta con la que inicias sesión: una clave de API
    es una contraseña que nunca caduca. OPNsense no tiene ninguna
    variante de solo lectura de algunos privilegios de arriba: Status:
    Services también permite iniciar y detener un servicio, System:
    Firmware también permite iniciar una actualización, y los
    privilegios de Unbound, Kea y Dnsmasq también cubren sus ajustes.
    DumbMonit solo lee, pero para limitar lo que una clave filtrada
    podría hacer, restringe las redes de origen del grupo a la
    dirección del host de DumbMonit. Nunca concedas All pages. OPNsense
    también viene con un certificado autofirmado: si la conexión se
    rechaza por ese motivo, marca «Accept an unverifiable certificate»
    en las opciones.

Dirección: `192.168.1.1`, `fw.lan`, `fw.lan:8443`, `[fd00::1]` o una URL
completa. El esquema `https` y el puerto 443 se añaden si faltan.

Algunos privilegios cubren más de una llamada. `Lobby: Dashboard` lleva
los endpoints de información del sistema, hora, memoria, disco, swap,
buffer de red y temperatura y la tabla de estados; `Status: Interfaces`
lleva la vista general de interfaz con sus contadores y direcciones. Un
privilegio que omitas cuesta exactamente las métricas que lleva: la
sonda sigue teniendo éxito, y `scrape_errors` se mantiene en cero,
porque un `403` en una llamada opcional se lee como «esta cuenta no
puede ver eso», no como un fallo. Solo `Lobby: Dashboard` es
obligatorio: sin él la sonda ni siquiera puede leer el nombre y versión
del cortafuegos, y reporta un error de autenticación.

### camelCase o snake_case {#camelcase-or-snake_case}

OPNsense 25.7 renombró sus rutas de API de `systemResources` a
`system_resources`. Ambas grafías todavía responden, pero los
privilegios comparan la ruta letra por letra: en 25.7 y posteriores una
clave restringida recibe `403` en la grafía antigua, y en 25.1 en
algunas de las nuevas. DumbMonit pide primero la grafía actual y recurre
a la otra ante un `403`, así que el mismo grupo funciona en ambos lados
del renombrado.

## Opciones {#options}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `scheme` | Protocol | `https` | HTTPS encaja con un cortafuegos de fábrica. HTTP solo si la interfaz web se sirve en texto plano. |
| `port` | Web interface port | `443` | Se usa si la dirección no da un puerto. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | OPNsense viene con un certificado autofirmado por defecto. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tiempo permitido para cada llamada a la API, de 1 a 120. |
| `gateways` | Watch the gateways | `true` | Lee lo que dice dpinger de cada gateway: activo o caído, retraso de ida y vuelta y pérdida de paquetes. |
| `interfaces` | Watch the interfaces | `true` | Lee el estado de enlace, direcciones y contadores de bytes, paquetes, errores y descartes de cada interfaz. |
| `firewall` | Watch the state table | `true` | Lee cuántas conexiones está rastreando pf y el límite configurado. |
| `dhcp` | Count the DHCP leases | `true` | Cuenta las concesiones que entrega el cortafuegos, sea cual sea el servidor que ejecuta. Solo recuentos. |
| `vpn` | Watch the VPN tunnels | `true` | Lee WireGuard, OpenVPN e IPsec: qué túneles existen y cuáles están activos. |
| `unbound` | Watch the resolver | `true` | Lee si Unbound está respondiendo. |
| `services` | Watch the services | `true` | Lee la lista de servicios que gestiona el cortafuegos y cuáles están en ejecución. |
| `carp` | Watch CARP | `true` | Lee las direcciones virtuales de un par de alta disponibilidad, y el modo de mantenimiento persistente. |
| `firmware` | Watch for updates | `true` | Lee el resultado de la última comprobación de actualización del cortafuegos y si hay un reinicio pendiente. |
| `temperature` | Read the temperature sensors | `true` | Lee los sensores de CPU y placa que expone el cortafuegos. |

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| Error de autenticación | La key y el secret están intercambiados, o la key se eliminó del usuario. OPNsense muestra el secret solo una vez: si se perdió, crea otra key y elimina la antigua. |
| Una página web en lugar de JSON | La clave de API se rechaza, o la cuenta no puede alcanzar ese endpoint. OPNsense entonces sirve su página de inicio de sesión con un `200`; DumbMonit lo reporta como un problema de autenticación en lugar de una respuesta ilegible. |
| Error de certificado | Certificado autofirmado: instala uno de confianza (ACME es un plugin) o activa `insecure_tls`. |
| Ninguna métrica de gateway en absoluto | La opción `gateways` está desactivada, o al grupo le falta `System: Gateways`. Un cortafuegos sin ningún gateway definido genuinamente no tiene ninguno. |
| Un gateway no muestra retraso ni pérdida | No tiene dirección de monitor, así que dpinger no le hace ping. OPNsense escribe `~` y DumbMonit no publica ninguna serie en lugar de un cero. |
| «Gateway down» se activa para un enlace que funciona | La dirección de monitor es inaccesible aunque el enlace esté activo — un caso común con un ISP que descarta ICMP. Define otra dirección de monitor en System → Gateways. |
| Sin métricas de tabla de estados | Al grupo le falta `Lobby: Dashboard`. |
| Error de autenticación en una clave que funciona en una herramienta de navegador | La herramienta usó la otra grafía de la ruta. Con una clave restringida, OPNsense 25.7 y posteriores responden `403` a `systemInformation` y `200` a `system_information`. |
| Sin métricas de actualización salvo `firmware_checked` | El cortafuegos nunca ha comprobado actualizaciones, o acaba de instalar una. Ejecuta una comprobación desde System → Firmware. |
| Sin métricas VPN | El plugin no está instalado, o no hay ningún túnel configurado. Una conexión IPsec desactivada deliberadamente no se reporta como caída. |
| «Firewall resolver stopped» en un cortafuegos que resuelve bien | Unbound está instalado pero apagado porque el cortafuegos resuelve con otra cosa. Desactiva esa regla. |
| La versión de firmware parece desactualizada | El cortafuegos no ha comprobado el espejo recientemente. DumbMonit lee la última comprobación, nunca activa una. |
| `scrape_errors` por encima de cero | Una llamada opcional falló por un motivo distinto a un privilegio o plugin faltante — un tiempo de espera, normalmente. Las demás métricas no se afectan. |
