# Dispositivo SNMP

El tipo más universal: casi todo el hardware de red habla SNMP.
Conmutadores, routers, NAS, SAI e impresoras califican todos. Se admiten
SNMP v1, v2c y v3.

## Qué vigila {#what-it-watches}

El perfil de recolección se detecta automáticamente a partir del
`sysObjectID` del dispositivo (y, para los SAI, su `sysDescr`). Siete
perfiles se incluyen con el producto; un dispositivo puede coincidir con
varios, y los perfiles se incluyen unos a otros.

| Perfil | Se aplica a | Métricas (prefijo `dumbmonit_`) |
|---|---|---|
| System (SNMPv2-MIB) | Todo dispositivo | `system_uptime_seconds`, `system_info` (nombre, descripción, ubicación, contacto como etiquetas) |
| Interfaces de red (IF-MIB) | Todo dispositivo, como respaldo | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out`, `if_discards_in/out`, `if_admin_status`, `if_oper_status`, `if_speed_bps`, por interfaz |
| Host resources (HOST-RESOURCES-MIB) | Net-SNMP (Linux, BSD, macOS), el agente SNMP de Windows, Synology, QNAP, Solaris | `host_uptime_seconds`, `host_processes`, `host_users`, `cpu_load_percent`, `storage_bytes_total/used` por sistema de archivos, `memory_bytes_total/used`, `load_average` (1, 5 y 15 minutos, solo Net-SNMP) |
| SAI (UPS-MIB) | APC, Eaton, MGE, CyberPower, Riello, Socomec, Tripp Lite, Vertiv, Salicru, y cualquier agente que anuncie UPS-MIB | `ups_battery_status`, `ups_battery_seconds_on`, `ups_battery_minutes_remaining`, `ups_battery_charge_percent`, voltios/amperios/temperatura de batería, voltios/hercios/líneas defectuosas de entrada, `ups_output_source`, voltios/hercios/vatios de salida, `ups_output_load_percent`, `ups_alarms_present` |
| Impresora (PRINTER-MIB) | HP, Kyocera, Epson, Canon, Brother, Lexmark, Ricoh, Xerox, Samsung, OKI, Konica Minolta | `printer_supply_level/max` por consumible, `printer_input_level/max` por bandeja, `printer_pages_printed`, `printer_status` |
| Conmutador Ethernet (EtherLike-MIB, LLDP) | Elegido a mano para cualquier conmutador; incluido por los perfiles de conmutadores de fabricante | Todo de IF-MIB, más por puerto `ether_fcs_errors`, `ether_alignment_errors`, `ether_symbol_errors`, `ether_late_collisions`, `ether_excessive_collisions`, `ether_mac_transmit_errors`, `ether_mac_receive_errors`, `ether_duplex_status` (1 desconocido, 2 half, 3 full), `if_broadcast_packets_in`, `if_multicast_packets_in` (64 bits), y `lldp_neighbor_info` (quién está conectado dónde: `remote_sysname`, `remote_port`, `remote_port_desc`; el número de puerto local es la segunda parte de la etiqueta `index`) |
| Conmutador Zyxel | Todo `sysObjectID` Zyxel (`1.3.6.1.4.1.890`) | Todo de Ethernet switch, más `cpu_load_percent` (promedio de un minuto), `zyxel_cpu_5min_percent`, `memory_usage_percent`, `zyxel_firmware_info` (firmware y modelo como etiquetas), y en conmutadores que publican el monitor de hardware, `zyxel_temperature_celsius` y `zyxel_temperature_high_threshold_celsius` por sensor, `zyxel_fan_rpm` y `zyxel_fan_low_threshold_rpm` por ventilador |

Los contadores (octetos de interfaz, páginas impresas) se guardan en bruto:
usa `rate()` en una consulta para obtener un caudal. Un reinicio del
dispositivo se muestra como un reseteo del contador, nunca como un pico
falso.

Las [reglas integradas](../alerting/rules.md) que se aplican a los
dispositivos SNMP: Device unreachable, High CPU, Disk almost full,
Filesystem almost full (forecast), UPS on battery, UPS battery low, Port
accumulating errors, Port flapping, Unusual CPU (baseline).

### Conmutadores Zyxel {#zyxel-switches}

La parte estándar del perfil Zyxel (interfaces, errores Ethernet, LLDP) es
común a todo conmutador Zyxel. Las cifras de CPU, memoria y firmware
provienen de la rama privada que comparten los conmutadores gestionados
actuales bajo `1.3.6.1.4.1.890.1.15` (entre ellos el XS1930 y el GS1900);
los modelos más antiguos y los cortafuegos Zyxel organizan las suyas de
otra forma, y esas series simplemente quedan vacías ahí. El perfil se
comprobó contra un XS1930-10 real; sus series de temperatura y ventilador
siguen el MIB de monitor de hardware publicado por Zyxel pero aún no se
han visto en hardware real.

`dot3StatsFrameTooLongs` deliberadamente no se lee: en el XS1930 cuenta
miles de millones de tramas jumbo perfectamente sanas, y una serie que
siempre sube en una red sana solo enseña a la gente a ignorarla.

### Controladores de gestión de servidor (BMC) {#server-management-controllers-bmc}

El propio agente SNMP de un BMC describe el BMC, no el servidor: su
pequeño Linux ARM, su memoria, sus enlaces de red. Un BMC Supermicro, por
ejemplo, responde como Net-SNMP y recibe el perfil Host resources, que lee
todo lo que ese agente publica sobre sí mismo. Los ventiladores,
temperaturas, fuentes de alimentación y discos del servidor se leen en su
lugar vía [Redfish](redfish.md).

## Qué preparar en el dispositivo {#what-to-prepare-on-the-device}

1. Abre la interfaz de administración del dispositivo.
2. Busca la sección SNMP, a menudo bajo «Network», «Services» o
   «Administration».
3. Activa SNMP v2c y anota la comunidad de solo lectura («public» por
   defecto en muchos dispositivos).
4. Si el dispositivo filtra por dirección, permite la dirección del
   servidor DumbMonit.
5. Vuelve a DumbMonit, introduce la dirección y la comunidad: el resto se
   detecta automáticamente.

!!! warning
    Una comunidad no está cifrada en la red. En una red compartida,
    prefiere SNMP v3, que autentica y cifra.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| SNMP v1 / v2c (comunidad) | La comunidad de solo lectura. |
| SNMP v3 | Nombre de usuario; autenticación opcional (MD5, SHA-1, SHA-224, SHA-256, SHA-384 o SHA-512 y una frase de paso); privacidad opcional (DES, AES-128, AES-192 o AES-256 y una frase de paso); nombre de contexto opcional. |

Dirección: una IP o nombre de host, por ejemplo `192.168.1.10`. El puerto
por defecto es 161; escribe `host:puerto` para usar otro.

## Opciones {#options}

Este tipo no lee ninguna opción. Las etiquetas libres siguen disponibles y
se copian en cada serie como `tag_<key>`.

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| *Unreachable* justo después de añadirlo | Comunidad incorrecta, SNMP no activado, o el dispositivo solo acepta peticiones de direcciones permitidas. El dispositivo nunca responde a una comunidad incorrecta: parece exactamente un dispositivo apagado. |
| Solo series `system_*` e `if_*` | El `sysObjectID` no coincidió con ningún perfil específico. IF-MIB y System se aplican a todo; HOST-RESOURCES, UPS, PRINTER y Zyxel necesitan un OID de fabricante conocido. En un conmutador de otra marca, elige el perfil Ethernet switch a mano. |
| Sin tráfico de interfaz | El dispositivo no expone los contadores de 64 bits de `ifXTable`. |
| Escaneo vacío | El escaneo prueba una comunidad en cada dirección; los dispositivos con otra comunidad o solo SNMP v3 no responden. |

Usa **Probe now** en la página del dispositivo: reporta el número de
muestras y los nombres de serie producidos por una sonda.
