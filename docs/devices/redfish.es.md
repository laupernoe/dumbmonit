# Hardware de servidor (Redfish) {#server-hardware-redfish}

El propio hardware de un servidor, leído desde su controlador de gestión
(BMC): ventiladores, temperaturas frente a sus propios umbrales, fuentes
de alimentación y su redundancia, discos, memoria, procesadores y el
registro de eventos.

Redfish es la interfaz HTTPS que el DMTF estandarizó para los
controladores de gestión, y cada controlador actual la habla: Supermicro,
Dell iDRAC, HPE iLO, Lenovo XClarity Controller, ASRock Rack, OpenBMC. Es
la forma correcta de vigilar el hardware de un servidor. El agente SNMP
de un BMC, en cambio, normalmente describe el propio BMC — su pequeño
Linux ARM, su memoria, sus enlaces de red — y no el servidor que
gestiona (consulta [SNMP](snmp.md#server-management-controllers-bmc)).

!!! warning "Probado solo contra las maquetas de referencia del DMTF"

    Esta integración se construyó y verificó contra las maquetas Redfish
    oficiales del DMTF (servidor en rack, servidor torre con
    almacenamiento local, servidor refrigerado por líquido con los
    esquemas térmicos y de potencia más nuevos), servidas por el
    servidor de maquetas del DMTF. Cada llamada es de lectura y las
    formas de recurso son las estándar, pero aún no se ha ejecutado
    contra un Supermicro, iDRAC o iLO real durante semanas. Dinos qué
    falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_redfish_`. Las series de
salud son **0 (OK), 1 (Warning) o 2 (Critical)**, directamente de
`Status.Health`.

| Familia | Métricas | Etiquetas |
|---|---|---|
| Servicio | `info` (valor 1) | `vendor`, `product`, `redfish_version` |
| Sistema | `system_health` (la propia), `system_health_rollup` (todo lo que contiene), `system_power_on`, `memory_health`, `processor_health` (los resúmenes del controlador, un valor para todos los DIMM y todas las CPU), `memory_total_bytes` | `system` |
| Chasis | `chassis_health`, `chassis_health_rollup` | `chassis` |
| Temperaturas | `temperature_celsius`, `temperature_upper_critical_celsius` y `temperature_upper_caution_celsius` (los propios umbrales crítico y de precaución del sensor, cuando los declara), `temperature_health` | `chassis`, `sensor` |
| Ventiladores | `fan_speed_rpm` (o `fan_speed_percent` cuando el controlador reporta un porcentaje), `fan_lower_critical_rpm` (o `_percent`: el mínimo propio declarado del ventilador, solo esquema antiguo), `fan_health`, `fan_redundancy_health` | `chassis`, `fan` / `group` |
| Alimentación | `psu_health`, `psu_output_watts`, `psu_capacity_watts`, `power_redundancy_health`, `power_consumed_watts` | `chassis`, `psu` / `group` |
| Tensiones | `voltage_volts`, `voltage_health` | `chassis`, `sensor` |
| Almacenamiento | `storage_health`, `drive_health`, `drive_failure_predicted`, `drive_life_left_percent` (SSD), `drive_capacity_bytes` | `system`, `storage`, `drive`, `media` |
| Controlador | `manager_health`, `manager_info` (valor 1) | `manager`, `firmware`, `model` |
| Registros | `log_entries`, `log_critical_entries`, `log_warning_entries` | `owner` (sistema o controlador), `log` |
| Recolección | `scrape_errors`, `scrape_duration_seconds` | |

**Una ranura vacía no es un fallo.** Un componente cuyo `Status.State`
es `Absent` (una bahía de alimentación vacía, una ranura DIMM libre, una
segunda CPU ausente) o `Disabled` (un grupo de redundancia no
configurado) no produce ninguna serie en absoluto. Un servidor entregado
con una fuente de alimentación de dos está sano, y se lee como tal.

**Dos generaciones del esquema, un conjunto de series.** La mayoría de
los controladores en servicio todavía publican los recursos más antiguos
`Chassis/{id}/Thermal` y `Chassis/{id}/Power`, una petición cada uno por
cada sensor; los más nuevos solo publican `ThermalSubsystem`,
`PowerSubsystem`, `EnvironmentMetrics` y `Sensors`, un recurso por
ventilador, fuente de alimentación y sensor. DumbMonit lee la forma
antigua cuando existe y la nueva en caso contrario; ambas producen
exactamente las mismas series y etiquetas, así que una regla escrita una
vez funciona en ambas.

**Los registros se cuentan, nunca se leen.** Las entradas de registro
pueden nombrar hosts, direcciones y usuarios: DumbMonit solo conserva
cuántas entradas hay, y cuántas de la primera página que sirve el
controlador son Critical o Warning.

Las [reglas integradas](../alerting/rules.md#server-hardware-redfish)
que se aplican: Server fan failed, Server temperature above critical,
Power supply redundancy lost, Power supply failed, Drive failure
predicted, Server health critical, además de Monitor unreachable.

## La página del monitor {#the-device-page}

Un monitor Redfish obtiene su propio panel en su página, sobre las
gráficas, leído de lo que guardó la última sonda: abrir la página nunca
consulta al controlador. Responde, en este orden:

1. **Si el servidor está sano.** Una palabra — Healthy, Degraded o
   Critical — de la propia salud y resúmenes del controlador, y debajo
   cada componente que lo arrastra abajo, en una frase: *CPU1 Temp is at
   47 °C, at or above its critical threshold of 45 °C*, *Power supply
   redundancy is lost*, *Drive 3 predicts its own failure*. Cuando el
   controlador resume el servidor como degradado sin nombrar nada que
   DumbMonit lea, el panel lo indica y señala hacia su registro de
   eventos. Debajo: estado de energía, consumo, memoria instalada y el
   firmware del controlador.
2. **Si los ventiladores y temperaturas están dentro de sus límites.**
   Cada sensor se compara con los umbrales que declara el controlador
   **para ese sensor** — sus temperaturas de precaución y crítica, la
   velocidad mínima de un ventilador — nunca con un número propio de
   DumbMonit. Un sensor que no declara ningún umbral muestra su lectura
   y ningún veredicto, a menos que el propio controlador lo reporte
   degradado. Los problemas se listan primero.
3. **Si la alimentación es redundante.** Cada grupo de redundancia
   (Redundant, Redundancy degraded, Redundancy lost), luego cada fuente
   de alimentación presente con su salida frente a su capacidad y su
   estado. Sin ningún grupo de redundancia, el panel indica que el
   controlador no declara ninguno: una sola fuente, o redundancia no
   configurada.
4. **Si los discos están sanos.** Cada controlador de almacenamiento,
   luego cada disco con su capacidad, la vida restante de un SSD y su
   estado; *Failure predicted* va antes que cualquier otra cosa.
5. **Memoria, procesadores y controlador**: los resúmenes de una
   palabra del controlador para todos los DIMM y todas las CPU, la
   propia salud, modelo y firmware del controlador de gestión, y las
   tensiones.
6. **Registros de eventos**: entradas contadas por severidad. Las
   entradas permanecen hasta que alguien limpia el registro, así que un
   recuento crítico puede ser noticia vieja: se muestra, pero no cambia
   la palabra de salud del servidor.

Las bahías vacías y ranuras libres nunca aparecen: no producen ninguna
serie. Cada estado es una palabra además de un color, y un valor
desconocido se lee «—», nunca 0.

## Crear una cuenta de solo lectura en el controlador de gestión {#create-a-read-only-account-on-the-management-controller}

1. Abre la interfaz web del controlador de gestión (BMC), no el sistema
   operativo del servidor. Crea un nuevo usuario local con el nombre
   siguiente, con una contraseña larga usada en ningún otro sitio.

    ```
    dumbmonit
    ```

2. Dale el rol más bajo que pueda leer, y nada más. Supermicro:
   Configuration → Users → Add User, privilegio User, y marca Redfish en
   el tipo de cuenta. Dell iDRAC: iDRAC Settings → Users → Local Users →
   Add, rol Read Only. HPE iLO: Administration → User Administration →
   New, desmarca todos los privilegios salvo Login. Lenovo XClarity
   Controller: BMC Configuration → User/LDAP → Create, rol Read-only.
   ASRock Rack: Settings → User Management, privilegio User.

3. Comprueba desde cualquier máquina en la red de gestión que la cuenta
   puede leer Redfish. El comando pide la contraseña e imprime el nombre
   y salud del servidor.

    ```
    curl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems
    ```

4. En DumbMonit, introduce la dirección del controlador, por ejemplo
   «bmc.lan» o «10.0.0.50», y luego el nombre de usuario y contraseña de
   esa cuenta.

5. DumbMonit solo lee: ventiladores, temperaturas, tensiones, fuentes de
   alimentación, discos, resúmenes de memoria y procesador, y cuántas
   entradas de registro hay por severidad. Nunca enciende ni apaga el
   servidor, nunca cambia un ajuste y nunca lee el texto de los
   registros.

!!! warning
    No reutilices la cuenta de fábrica del controlador: puede apagar el
    servidor, montar medios y reflashear el firmware. Los controladores
    de gestión también vienen con un certificado autofirmado: si la
    conexión se rechaza por ese motivo, marca «Accept an unverifiable
    certificate» en las opciones. Un controlador responde lento y una
    lectura completa toma varias peticiones: si el monitor reporta
    tiempos de espera agotados, eleva DUMBMONIT_PROBE_TIMEOUT_SECS en el
    servidor.

Los nombres de menú de arriba son los del firmware actual; las
generaciones más antiguas los mueven pero conservan los mismos roles. En
Supermicro, el tipo de cuenta mostrado al crear un usuario decide qué
interfaces puede usar: una cuenta sin acceso Redfish es rechazada con un
401 incluso con la contraseña correcta.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Cuenta de controlador de gestión | Nombre de usuario y contraseña de la cuenta de solo lectura. |

Dirección: el nombre de host o IP del controlador, por ejemplo `bmc.lan`
o `10.0.0.50`. El puerto por defecto es 443; escribe `host:puerto` para
otro, o una URL completa (`https://bmc.lan:8443`).

## Opciones {#options}

| Opción | Por defecto | Efecto |
|---|---|---|
| `port` | `443` | Puerto HTTPS, si la dirección no da uno. |
| `insecure_tls` | `false` | Aceptar un certificado autofirmado. |
| `request_timeout_seconds` | `8` | Tiempo permitido para cada llamada Redfish, de 1 a 120. |
| `auth` | `basic` | `basic` envía el nombre de usuario y contraseña con cada petición y no deja nada abierto en el controlador. `session` inicia sesión una vez y reutiliza el token de sesión (`X-Auth-Token`), volviendo a abrirlo solo cuando el controlador lo rechaza y cerrando el que reemplaza. Úsalo solo si el controlador rechaza la autenticación básica: los controladores solo tienen un puñado de ranuras de sesión. |
| `storage` | `true` | Leer los controladores de almacenamiento y cada disco detrás de ellos. Una petición por disco. |
| `logs` | `true` | Contar las entradas del registro de eventos del sistema y del propio registro del controlador, por severidad. |

Una lectura completa son unas pocas docenas de peticiones: la raíz del
servicio, luego cada sistema, chasis y controlador, sus recursos
térmicos y de potencia, los controladores de almacenamiento y discos,
los servicios de registro. DumbMonit envía como máximo cuatro a la vez.
Un controlador lento puede necesitar un tiempo de espera de sonda más
largo (`DUMBMONIT_PROBE_TIMEOUT_SECS`, consulta
[Configuración](../reference/configuration.md)) y un intervalo de sonda
de dos minutos o más.

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| *Authentication refused* | Contraseña incorrecta, o la cuenta no tiene permitido usar Redfish (tipo de cuenta en Supermicro, falta el privilegio «Login» en iLO). |
| *Insufficient privileges* | El rol es demasiado bajo para un recurso; el resto de la lectura igualmente pasa. Algunos controladores ocultan el registro de eventos a su rol más bajo. |
| *TLS certificate rejected* | El certificado autofirmado del controlador: marca «Accept an unverifiable certificate», o instala un certificado de confianza en el controlador. |
| *Timed out* | El controlador es lento en responder: eleva `DUMBMONIT_PROBE_TIMEOUT_SECS`, desactiva `storage` o `logs`, o alarga el intervalo. |
| Sin umbrales de temperatura | El controlador no publica `UpperThresholdCritical` para ese sensor: la temperatura se sigue registrando, pero la regla integrada no puede compararla con nada. |
