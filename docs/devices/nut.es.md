# SAI con NUT {#ups-with-nut}

Un SAI conectado a un NAS, una Raspberry Pi o un servidor por USB
normalmente se vigila con Network UPS Tools (NUT): su demonio, `upsd`,
publica lo que dice el SAI en el puerto TCP 3493. DumbMonit lee ese
servidor, como cualquier cliente NUT: si el SAI funciona con corriente o
con batería, si la batería está baja o desgastada, la carga, el tiempo de
funcionamiento restante, la carga de trabajo, la tensión de entrada y la
antigüedad de la batería. Un servidor a menudo publica varios SAI; todos
se leen en una conexión, y cada serie lleva el nombre de su SAI.

Para un SAI con su propia tarjeta de red, el tipo [dispositivo
SNMP](snmp.md) lee en su lugar el UPS-MIB estándar.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_nut_` y llevan la etiqueta
`ups`, el nombre del SAI en `ups.conf` (`ups` en un NAS Synology, `qnapups`
en un QNAP). Una variable que el SAI no publica no produce ninguna serie.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | Las banderas de `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 cuando está activa, 0 en caso contrario, todas en cada lectura | `ups` |
| `ups_status_info` | valor 1, el `ups.status` en bruto | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | carga, y la carga bajo la cual el SAI reporta batería baja | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | tiempo restante con batería a la carga actual, y el tiempo de batería baja | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | tiempo desde `battery.date` (instalada o reemplazada), o si no `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | lado de corriente; 0 V durante un corte | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | lado de carga | `ups` |
| `ups_load_percent` | carga de trabajo, en porcentaje de la capacidad del SAI | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | potencia consumida y la nominal, cuando el SAI las reporta | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 cuando la última autoprueba (`ups.test.result`) reporta un error o aviso, 0 cuando pasó; ausente cuando no se ejecutó ninguna prueba | `ups`, `result` |
| `ups_info` | valor 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 cuando `upsd` responde pero no tiene datos frescos del SAI (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 cuando el driver NUT de ese SAI no está en ejecución | `ups` |
| `server_ups` | número de SAI que publica el servidor | |
| `server_info` | valor 1 | `version` |

Los números de serie nunca se ponen en una etiqueta. Como máximo se leen
32 SAI por servidor.

Las [reglas integradas](../alerting/rules.md#ups-behind-nut) que se
aplican: UPS on battery, UPS battery low, UPS battery needs replacing,
UPS load high, UPS runtime short, UPS data stale, además de Device
unreachable cuando el propio servidor NUT deja de responder.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca se conecta al servidor NUT. Una tarjeta por SAI: su estado en
palabras (con corriente, con batería, batería baja, reemplazar batería,
bypass, sobrecarga…), la carga, el tiempo restante, la carga de trabajo,
la tensión de entrada y la antigüedad de la batería.

## Dejar que DumbMonit lea el servidor NUT {#let-dumbmonit-read-the-nut-server}

1. Encuentra el servidor NUT: la máquina a la que está conectado el SAI
   (un NAS, una Raspberry Pi, un servidor) ejecuta upsd en el puerto TCP
   3493. Desde el host de DumbMonit, comprueba que responde y anota los
   nombres de sus SAI.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Con las herramientas cliente de NUT instaladas, `upsc -l nas.lan` da
    la misma lista.

2. En una instalación NUT normal, haz que upsd escuche en una dirección
   que el host de DumbMonit pueda alcanzar: añade esta línea a
   upsd.conf, abre el puerto TCP 3493 en el cortafuegos para el host de
   DumbMonit, y reinicia upsd. Leer no necesita ninguna cuenta: upsd deja
   que cualquier host que acepte lea las variables, y no cambia nada sin
   un usuario que tenga el derecho a hacerlo.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. En un NAS, el mismo interruptor tiene otro nombre. Synology DSM:
   Control Panel → Hardware & Power → UPS, marca Enable network UPS
   server, y añade el host de DumbMonit bajo Permitted DiskStation
   Devices; el SAI se llama ups. QNAP: la página UPS del Control Panel,
   Enable network UPS master, con el host de DumbMonit entre las
   direcciones permitidas; el SAI se llama qnapups. TrueNAS: el servicio
   UPS con Remote Monitor marcado.

4. Opcional: DumbMonit envía un nombre de usuario y contraseña solo si
   introduces uno, y upsd no pide ninguno para leer. Si tu política
   requiere que cada cliente esté identificado, añade un usuario sin
   acciones ni instcmds a upsd.users, con una contraseña larga y
   aleatoria, y reinicia upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. En DumbMonit, introduce la dirección del servidor NUT, por ejemplo
   «nas.lan». Se vigila cada SAI que publica; para vigilar solo algunos,
   lista sus nombres en la opción UPS.

6. DumbMonit solo lee: envía LIST UPS y LIST VAR, nunca LOGIN, SET,
   INSTCMD o FSD, así que el servidor nunca lo cuenta como un secundario
   al que esperar antes de apagarse.

!!! warning
    DumbMonit vigila el SAI; no apaga nada. Las máquinas que alimenta el
    SAI siguen necesitando su propio cliente NUT (upsmon) para apagarse
    limpiamente cuando la batería se agota.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Sin autenticación | Lo por defecto: `upsd` deja que cualquier host que acepte lea las variables del SAI. |
| Nombre de usuario / contraseña de upsd | Enviados con `USERNAME` y `PASSWORD` antes de leer. `upsd` solo los comprueba para comandos, que DumbMonit nunca envía. |

Dirección: un nombre de host o IP (`nas.lan`), `host:puerto`, o la forma
`ups@host` de `upsc`, de la cual solo se conserva el host. Opciones:
puerto (3493), el SAI a vigilar (todos por defecto) y el tiempo de espera
por intercambio (10 s).

Una conexión rechazada o sin respuesta cuenta como el dispositivo
inaccesible. Cuando la máquina que ejecuta el servidor NUT se apaga en el
corte que reporta (a menudo es su propio primario de upsmon), las alertas
del SAI se ocultan como suprimidas mientras Device unreachable se activa para
ese mismo dispositivo: lee la alerta Device unreachable como la señal de que
el corte de corriente ganó. Ejecuta el servidor NUT en una máquina que
sobreviva a las demás, o vigila el SAI desde un segundo servidor NUT.
`ACCESS-DENIED` se muestra como un error de credencial, y un
nombre de SAI que el servidor no publica como un error de configuración;
ninguno de los dos despierta a nadie por la noche. La conexión es TCP
plano: `STARTTLS` no se usa, y nada de lo leído es secreto.
