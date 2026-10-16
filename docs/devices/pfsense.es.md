# pfSense

Un firewall pfSense a través del paquete comunitario de API REST
(`pfSense-pkg-RESTAPI`, versión 2): gateways caídos, que pierden paquetes
o responden lento, interfaces activadas sin enlace, servicios activados
que se detuvieron, y CPU, memoria, disco y temperatura.

DumbMonit envía una clave de API en la cabecera `X-API-Key`. Una clave
lleva exactamente los privilegios del usuario pfSense al que pertenece:
el usuario `dumbmonit` de abajo solo tiene los privilegios `GET` de las
cinco páginas que lee DumbMonit, así que la clave puede leer esas páginas
y no cambiar nada. Cada minuto DumbMonit lee, en paralelo,
`/api/v2/status/system`, `/api/v2/status/gateways`,
`/api/v2/status/interfaces`, `/api/v2/status/services` y
`/api/v2/system/version`, y, si se dio el privilegio,
`/api/v2/system/restapi/version`.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de la documentación publicada
    del paquete de API REST y el código fuente de sus modelos (nombres
    de campo, unidades, el sobre de respuesta y sus identificadores de
    error), no contra un pfSense en ejecución. Cada llamada es de
    lectura. Dinos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_pfsense_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | Como las muestra el panel, en porcentaje. La cifra de CPU es la propia estimación de pfSense a partir del load average. |
| `temperature_celsius` | | Cuando pfSense puede leer un sensor. |
| `load_average` | `period`: `1m`, `5m`, `15m` | Load average. |
| `cpu_count` | | Núcleos del procesador. |
| `version_info` | `version` | Valor 1: la versión de pfSense, `2.7.2-RELEASE`. |
| `gateway_up` | `gateway` | 0 cuando pfSense marca el gateway como caído, 1 en caso contrario. |
| `gateway_status_info` | `gateway`, `status` | Valor 1: la propia palabra de pfSense (`none` cuando todo va bien, `down`, `loss`, `delay`). |
| `gateway_loss_percent` | `gateway` | Pérdida de paquetes hacia la dirección de monitor. |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | Ida y vuelta hacia la dirección de monitor y su desviación; ausente mientras el gateway está caído. |
| `gateways`, `gateways_down` | | Recuentos. |
| `interface_enabled`, `interface_up` | `interface`, `descr` | Activada en la configuración; enlace levantado. |
| `interface_down` | `interface`, `descr` | 1 cuando la interfaz está activada y no tiene enlace. Una interfaz desactivada nunca está caída. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | Contadores. |
| `interfaces_down` | | Recuento. |
| `service_running`, `service_enabled` | `service`, `description` | Como lo muestra Status > Services. |
| `service_stopped` | `service`, `description` | 1 cuando el servicio está activado y no en ejecución. |
| `services_stopped` | | Recuento. |
| `restapi_update_available` | | Con el privilegio `/api/v2/system/restapi/version`: 1 cuando el paquete de API REST tiene una versión más reciente. |
| `restapi_version_info` | `version`, `latest` | Valor 1: versión instalada y última versión del paquete. |

pfSense no indica a través de esta API si hay una actualización de
pfSense disponible: la única ruta que toca las actualizaciones del
sistema (`/api/v2/system/update`) inicia una. DumbMonit solo reporta las
propias actualizaciones del paquete.

Las [reglas integradas](../alerting/rules.md#pfsense) que se aplican:

- **pfSense gateway down**: un gateway está marcado caído durante tres
  minutos (Warning).
- **pfSense gateway losing packets**: más del 10 % de pérdida durante
  diez minutos, en un gateway que no está ya caído (Advisory).
- **pfSense gateway slow**: ida y vuelta por encima de 500 ms durante
  quince minutos (Advisory). Elévalo para un enlace satelital.
- **pfSense interface without link**: una interfaz activada sin enlace
  durante cinco minutos (Advisory).
- **pfSense service stopped**: un servicio activado sin ejecutarse
  durante cinco minutos (Advisory).
- **pfSense disk nearly full**: más del 90 % durante treinta minutos
  (Advisory).

## La página del monitor {#the-device-page}

El panel Firewall indica en una frase qué va mal, muestra CPU, memoria,
disco y temperatura, luego lista cada gateway con su latencia y pérdida,
las interfaces sin enlace, los servicios detenidos, y una actualización
disponible del paquete de API REST.

## Crear una clave de API REST de solo lectura para DumbMonit {#create-a-read-only-rest-api-key-for-dumbmonit}

1. Instala el paquete comunitario de API REST (pfSense-pkg-RESTAPI,
   versión 2) desde Diagnostics > Command Prompt o una shell SSH, con el
   archivo de la página de versiones del proyecto que coincida con tu
   versión de pfSense (2.7.2 abajo). Luego, en System > REST API >
   Settings, marca tanto BasicAuth como Key entre los métodos de
   autenticación.

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. En System > User Manager, añade un usuario llamado dumbmonit con una
   contraseña larga y aleatoria. En Effective Privileges, añade solo
   estos privilegios de API REST: /api/v2/status/system GET,
   /api/v2/status/gateways GET, /api/v2/status/interfaces GET,
   /api/v2/status/services GET, /api/v2/system/version GET, más
   /api/v2/system/restapi/version GET para enterarte de las propias
   actualizaciones del paquete. Solo para el siguiente paso, añade
   también /api/v2/auth/key POST.

3. Crea la clave como dumbmonit, desde cualquier máquina que alcance
   pfSense: la clave lleva exactamente los privilegios de ese usuario.
   Copia el valor de clave de la respuesta, y luego retira de nuevo el
   privilegio /api/v2/auth/key POST.

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

4. En DumbMonit, introduce la dirección de pfSense, por ejemplo
   «pfsense.lan», y pega la clave como API key. pfSense viene con un
   certificado autofirmado: marca Accept an unverifiable certificate a
   menos que lo hayas reemplazado.

!!! warning
    La API REST no indica si hay una actualización de pfSense
    disponible, solo si el propio paquete tiene una. El paquete es un
    proyecto comunitario, no parte de pfSense: reinstálalo después de
    cada actualización de pfSense.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Clave de API REST | La clave creada para `dumbmonit` en el paso 3, enviada como `X-API-Key`. |

Dirección: el nombre de host o IP de pfSense, `host:puerto`, o una URL.
Opciones:

- **Protocol** (`https`) y **Port** (443): los del webConfigurator,
  definidos en System > Advanced > Admin Access.
- **Accept an unverifiable certificate**: el propio certificado de
  pfSense es autofirmado.
- **Timeout per request** (15 s).

## Solución de problemas {#troubleshooting}

«pfSense refused the API key (401)»: la clave es incorrecta, o Key no
está marcado entre los métodos de autenticación en System > REST API >
Settings.

«pfSense accepted the API key but refused … (403)»: al usuario
`dumbmonit` le falta el privilegio de esa página; el mensaje lo nombra.

«… not found: is the REST API package installed?»: falta el paquete,
normalmente tras una actualización de pfSense, que lo elimina.
Reinstálalo como en el paso 1.
