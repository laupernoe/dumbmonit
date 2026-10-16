# FortiGate

Un Fortinet FortiGate a través de la API REST de FortiOS: túneles IPsec
sitio a sitio caídos, interfaces activadas que perdieron su enlace, un
clúster HA desincronizado o falto de un miembro, memoria aproximándose
al modo conserve, licencias de FortiGuard y FortiCare caducando, y
versiones de parche ofrecidas para la rama de FortiOS en ejecución.

DumbMonit usa un usuario de API REST cuyo perfil de acceso solo lee, y
que solo acepta llamadas desde la dirección del servidor DumbMonit
(trusted host). Su token se envía como `Authorization: Bearer`, nunca en
la URL: FortiOS 7.4.5 y 7.6.1 lo rechazan ahí de todas formas.

Cada minuto DumbMonit lee tres páginas obligatorias —
`/api/v2/monitor/system/status`, `/api/v2/monitor/system/resource/usage`
y `/api/v2/monitor/system/interface/select` — y seis opcionales que un
perfil más limitado, o un FortiGate sin clúster ni túneles, puede
rechazar o dejar vacías sin hacer fallar el resto: el estado
administrativo de las interfaces (`/api/v2/cmdb/system/interface`),
estadísticas y checksums de HA, túneles IPsec, estado de licencia y
firmware disponible.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de la documentación de
    FortiOS de Fortinet y las formas de respuesta registradas por el
    exportador Prometheus comunitario para FortiGate, no contra un
    FortiGate en ejecución. Cada llamada es de lectura. Las páginas de
    licencia y firmware son las menos documentadas: dinos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_fortigate_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Valor 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Uso actual. |
| `sessions`, `session_setup_rate` | | Sesiones activas, sesiones nuevas por segundo. |
| `log_disk_ok` | | 0 cuando el disco de registro necesita formatearse. |
| `interface_link_up` | `interface`, `alias` | Enlace físico. |
| `interface_enabled` | `interface`, `alias` | Estado administrativo; toda interfaz cuenta como activada cuando el perfil no puede leer la configuración. |
| `interface_down` | `interface`, `alias` | 1 cuando la interfaz está activada, tiene una dirección y no tiene enlace. Los puertos libres sin dirección se dejan en paz. |
| `interface_speed_mbps` | `interface`, `alias` | Mientras el enlace está activo. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Contadores. |
| `interfaces_down` | | Recuento. |
| `ha_members` | | Miembros en el clúster; 0 para un FortiGate independiente. |
| `ha_in_sync` | | Con dos miembros o más: 1 cuando cada miembro tiene el mismo checksum de configuración. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | Por miembro. |
| `ipsec_tunnel_up` | `tunnel` | 1 cuando al menos una fase 2 del túnel está activa. Los túneles dial-up (acceso remoto) se excluyen. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | Por selector de fase 2. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Contadores. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Valor 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Recuentos. |
| `license_status_info` | `license`, `status` | Valor 1, para cada licencia que tiene el FortiGate (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…); las entradas `no_license` se excluyen. |
| `license_expired` | `license` | 1 cuando el estado es `expired`. |
| `license_expiry_seconds` | `license` | Hasta la caducidad; negativo una vez caducada. |
| `firmware_update_available` | | 1 cuando FortiGuard ofrece una versión de parche más reciente de la rama en ejecución (7.4.4 → 7.4.5). Una rama nueva (7.6) es un proyecto, no una alerta. |
| `firmware_latest_info` | `version` | Valor 1: esa versión de parche. |

Las [reglas integradas](../alerting/rules.md#fortigate) que se aplican:

- **FortiGate IPsec tunnel down**: ninguna fase 2 activa durante cinco
  minutos (Warning).
- **FortiGate interface without link**: durante cinco minutos
  (Advisory).
- **FortiGate HA out of sync**: durante quince minutos (Advisory).
- **FortiGate HA member lost**: menos miembros que en cualquier momento
  de las últimas 24 horas (Warning). El recuento esperado no se publica
  en ningún sitio, así que un miembro eliminado para siempre deja de
  alertar al día siguiente.
- **FortiGate memory near conserve mode**: por encima del 85 % durante
  diez minutos (Advisory). Al 88 % por defecto, FortiOS entra en modo
  conserve y deja de inspeccionar sesiones nuevas.
- **FortiGate CPU high**: por encima del 90 % durante quince minutos
  (Advisory).
- **FortiGate licence expiring**: menos de 30 días restantes, o
  caducada (Advisory, recordado diariamente).
- **FortiGate firmware update**: se ofrece una versión de parche (Info,
  recordado semanalmente).

## La página del monitor {#the-device-page}

El panel Firewall indica en una frase qué va mal, muestra CPU, memoria,
sesiones y el estado HA, luego lista los túneles IPsec (los caídos
primero), las interfaces sin enlace, las licencias que caducan en menos
de 60 días o ya caducadas, y una versión de parche disponible.

## Crear un usuario de API REST de solo lectura para DumbMonit {#create-a-read-only-rest-api-user-for-dumbmonit}

1. Desde la CLI del FortiGate, crea un perfil de acceso que solo pueda
   leer el estado del sistema, la red y la VPN, y nada más.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Crea el usuario de API REST dumbmonit con ese perfil, confiando solo
   en la dirección del servidor DumbMonit (sustituye 192.0.2.10), y
   luego genera su token: se imprime una vez.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. En DumbMonit, introduce la dirección del FortiGate, por ejemplo
   «fw.lan» (añade el puerto si la interfaz de administración no
   escucha en 443), y pega el token como API token. FortiGate viene con
   un certificado autofirmado: marca Accept an unverifiable certificate
   a menos que hayas instalado el tuyo.

!!! warning
    Una interfaz solo cuenta como caída cuando está activada, tiene una
    dirección y no tiene enlace: los puertos libres se dejan en paz.
    Las actualizaciones de firmware son las que FortiGuard ofrece al
    propio FortiGate: sin acceso a FortiGuard, no se reporta ninguna.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de API | El token impreso por `execute api-user generate-key dumbmonit`, enviado como `Authorization: Bearer`. |

Dirección: el nombre de host o IP del FortiGate, `host:puerto`, o una
URL. Opciones:

- **Port** (443): el puerto HTTPS de la interfaz de administración
  (`set admin-sport` bajo `config system global`).
- **VDOM**: el dominio virtual a leer; vacío para el que pertenece el
  usuario de API REST.
- **Accept an unverifiable certificate**: el propio certificado del
  FortiGate es autofirmado.
- **Timeout per request** (15 s).

## Solución de problemas {#troubleshooting}

«FortiGate refused the API token (401)»: el token es incorrecto o se
regeneró. Genera uno nuevo con `execute api-user generate-key
dumbmonit`.

«FortiGate refused … (403)»: la dirección de DumbMonit no está entre
los hosts de confianza del usuario, o su perfil no puede leer esa
página. Una página opcional rechazada así simplemente se excluye.

Sin cifra de IPsec, licencia o firmware: el perfil no puede leerlas, o
el FortiGate no tiene ningún túnel; el resto se sigue leyendo.
