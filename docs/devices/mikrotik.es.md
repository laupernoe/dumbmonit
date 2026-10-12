# MikroTik RouterOS

Routers y conmutadores que ejecutan RouterOS 7: si RouterOS y el firmware
RouterBOOT se han quedado atrás, CPU, memoria y almacenamiento,
temperaturas, ventiladores y fuentes de alimentación, y el estado,
tráfico, errores y pérdidas de enlace de cada interfaz.

DumbMonit lee la API REST que RouterOS 7 sirve bajo `/rest`, con un
usuario en un grupo que solo puede leer. Envía peticiones `GET` y nada
más.

Un MikroTik rara vez se rompe; envejece. Una versión de RouterOS que ya
no es la última de su canal, un firmware RouterBOOT olvidado tras una
actualización (solo se aplica cuando se pide, en el siguiente reinicio),
una fuente de alimentación redundante que murió sin que nadie se diera
cuenta, un puerto que sigue registrando errores o perdiendo su enlace.
Eso es lo que vigila esta página, junto con la carga.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_mikrotik_`. Los totales
son contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `info` | valor 1 | `version`, `release` (`long-term`, `stable`…), `board`, `architecture`, `identity` |
| `uptime_seconds` | tiempo desde el último arranque | |
| `cpu_load_percent`, `cpu_count` | carga de CPU, todos los núcleos | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | memoria | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | la flash o disco en el que vive RouterOS | |
| `update_checked` | 1 cuando el router ha comprobado si hay un RouterOS más reciente y obtuvo respuesta | |
| `update_available` | 1 cuando la última versión del canal difiere de la instalada | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 1 en hardware RouterBOARD, 0 en un Cloud Hosted Router o una máquina x86 | |
| `firmware_upgrade_pending` | 1 cuando RouterBOOT es más antiguo que el firmware incluido con RouterOS | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | cada sensor que tiene la placa | `sensor` (`cpu-temperature`, `fan1-speed`…) |
| `health_ok` | 1 cuando un sensor `psu*-state` o `fan*-state` lee `ok`, 0 en caso contrario | `sensor` |
| `interface_running` | 1 cuando la interfaz tiene enlace | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | tráfico | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | errores y descartes | `interface`, `type` |
| `interface_link_downs_total` | veces que se perdió el enlace | `interface`, `type` |
| `interfaces_skipped` | interfaces excluidas más allá del límite fijado en las opciones | |
| `scrape_errors` | lecturas secundarias que fallaron durante la última sonda | |

Solo se leen las interfaces activadas. Las dinámicas (un cliente PPPoE o
L2TP, cuyo nombre cambia en cada conexión) y el loopback se excluyen.

Los sensores dependen de la placa: un hAP tiene una tensión y una
temperatura, un CCR añade ventiladores y fuentes de alimentación, un
Cloud Hosted Router no tiene ninguno. Un sensor que la placa no tiene no
produce ninguna serie.

La versión más reciente de RouterOS solo se conoce cuando el propio
router comprueba las actualizaciones: a un usuario de solo lectura no se
le permite iniciar esa comprobación. El paso opcional de abajo la
programa en el router. Sin él, `update_checked` se queda en 0 y la regla
de actualización nunca se activa.

Las [reglas integradas](../alerting/rules.md#mikrotik-routeros) que se
aplican: MikroTik CPU high, memory high, storage almost full,
temperature high, power supply or fan failed, interface accumulating
errors, link flapping, RouterOS update available y firmware upgrade
pending, además de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca se conecta al router. Muestra la versión de RouterOS y si hay una
más reciente disponible, el firmware RouterBOOT, el tiempo de actividad,
CPU, memoria y almacenamiento, cada sensor con su estado, y una tabla de
las interfaces con su enlace, tráfico, errores y pérdidas de enlace.

## Crear un usuario de solo lectura en el router {#create-a-read-only-user-on-the-router}

1. Abre una terminal en el router (WinBox → New Terminal, o SSH con la
   cuenta por defecto) y crea un grupo que solo pueda leer, a través de
   la API REST. RouterOS necesita las tres políticas para una lectura
   REST: rest-api abre la API, api y read permiten la lectura en sí.

    ```
    /user group add name=dumbmonit-read policy=read,api,rest-api comment=DumbMonit
    ```

2. Crea el usuario en ese grupo con una contraseña larga y aleatoria
   propia. Para aceptar la cuenta solo desde DumbMonit, añade address=
   seguido de la dirección del host de DumbMonit.

    ```
    /user add name=dumbmonit group=dumbmonit-read password=REPLACE-WITH-A-LONG-RANDOM-PASSWORD
    ```

3. Activa el servicio web HTTPS, que RouterOS trae desactivado y sin
   certificado. Omite esto si www-ssl ya tiene uno. En caso contrario,
   estos comandos crean un certificado en el propio router; luego marca
   «Accept an unverifiable certificate» en las opciones de abajo.

    ```
    /certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
    /certificate sign local-ca
    /certificate add name=https-cert common-name=router.lan days-valid=825
    /certificate sign https-cert ca=local-ca
    /ip service set www-ssl certificate=https-cert disabled=no
    ```

4. Opcional: para que te avisen de nuevas versiones de RouterOS, haz que
   el router las compruebe al arrancar y una vez al día. Un usuario de
   solo lectura no puede iniciar esa comprobación, así que DumbMonit solo
   lee su resultado.

    ```
    /system scheduler add name=check-for-updates interval=1d start-time=startup on-event="/system package update check-for-updates once"
    ```

5. En DumbMonit, introduce la dirección del router, por ejemplo
   «192.168.88.1», con el usuario dumbmonit y su contraseña. DumbMonit
   solo envía peticiones GET a /rest: nunca cambia un ajuste, nunca
   inicia una actualización y nunca reinicia el router.

!!! warning
    Se requiere RouterOS 7.1 o posterior: RouterOS 6 no tiene API REST.
    No uses la cuenta por defecto: su grupo completo puede cambiar todo,
    mientras que dumbmonit-read no tiene ninguna política de escritura y
    cada cambio se rechaza. El servicio www (HTTP plano) también
    responde a la API REST, pero envía la contraseña en claro con cada
    petición.

Las tres políticas se comprobaron una por una en RouterOS 7.23.7, cada
una con un usuario nuevo: sin `rest-api` el router responde 401, sin
`api` responde «not allowed», sin `read` «not enough permissions».
DumbMonit reporta cada caso como un problema de credencial que nombra la
política faltante, no como un router caído. Con las tres, un cambio, un
reinicio o una comprobación de actualización se rechaza con «not enough
permissions».

RouterOS puede seguir aplicando las políticas anteriores de un grupo a un
usuario durante un tiempo tras editar el grupo. Si cambias las políticas
de un grupo existente y el error persiste, elimina y vuelve a crear el
usuario.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Usuario de solo lectura | El usuario `dumbmonit` y su contraseña, enviados como autenticación básica HTTP con cada petición. La API REST no tiene sesión. |

Dirección: un nombre de host o IP (`192.168.88.1`), `host:puerto`, o una
URL completa (`https://router.lan:8443`). Opciones: protocolo (HTTPS por
defecto; HTTP usa el servicio `www`), puerto (443 para HTTPS, 80 para
HTTP), comprobación de certificado, tiempo de espera de la petición (10
s), si leer las interfaces (activado), y cuántas como máximo (64).
