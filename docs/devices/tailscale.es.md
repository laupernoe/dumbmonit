# Tailscale

Los dispositivos de una tailnet a través de la API de Tailscale:
servidores que se desconectaron, claves de nodo a punto de caducar,
dispositivos esperando aprobación, y clientes con una actualización
disponible.

DumbMonit lee `GET /api/v2/tailnet/{tailnet}/devices` en
`api.tailscale.com`, una vez por minuto. La credencial recomendada es un
cliente OAuth limitado al alcance `devices:core:read`: DumbMonit
intercambia su ID y secreto por un token de una hora
(`/api/v2/oauth/token`) y lo conserva hasta que caduca. Puede listar los
dispositivos y no puede cambiar nada. Un token de acceso de API
(`tskey-api-…`) también funciona, pero lleva todos los derechos del
usuario que lo creó y caduca a los 90 días como máximo.

Esta es una API de Internet: la dirección del dispositivo es la de la
API, no la de una máquina. DumbMonit necesita HTTPS saliente hacia
`api.tailscale.com`; nada en la tailnet se contacta.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de la documentación de la API
    de Tailscale y el modelo de dispositivo de su cliente Go oficial, no
    contra una tailnet en producción. Solo lee. Dinos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_tailscale_`. Un
dispositivo se nombra por la primera etiqueta de su nombre MagicDNS
(`nas` para `nas.tail1234.ts.net`). Los dispositivos compartidos hacia
la tailnet desde otra se cuentan, no se describen.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `device_online` | `device` | 1 mientras el dispositivo está conectado al servidor de coordinación. |
| `device_offline_seconds` | `device` | 0 mientras está en línea, si no el tiempo desde que se vio por última vez. |
| `device_watched` | `device` | 1 cuando el dispositivo debe permanecer en línea: cada dispositivo etiquetado por defecto, o los nombrados en la opción de abajo. |
| `device_key_expiry_seconds` | `device` | Hasta que caduca la clave de nodo; ausente cuando la caducidad de clave está desactivada. |
| `device_authorized` | `device` | 0 mientras el dispositivo espera aprobación. |
| `device_update_available` | `device` | 1 cuando Tailscale ofrece un cliente más reciente. |
| `device_info` | `device`, `os`, `client_version`, `tags` | Valor 1. |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | Recuentos. |

Solo los dispositivos vigilados generan una alerta cuando se desconectan:
un teléfono o un portátil que duerme no es una caída. Los dispositivos
etiquetados son, en la práctica, los servidores, routers de subred y
exit nodes; lista otros en la opción **Devices to keep online**, o
introduce `all`.

Las [reglas integradas](../alerting/rules.md#tailscale) que se aplican:

- **Tailscale device offline**: un dispositivo vigilado se desconectó
  durante más de 15 minutos (Advisory).
- **Tailscale node key expiring**: menos de 14 días restantes (Advisory,
  recordado diariamente). Cuando caduca, el dispositivo abandona la
  tailnet hasta que alguien vuelve a iniciar sesión.
- **Tailscale device awaiting approval** (Advisory): apruébalo si es
  esperado, elimínalo en caso contrario.
- **Tailscale client update**: un dispositivo vigilado tiene una
  actualización de cliente durante seis horas (Info, recordado
  semanalmente).

## Dispositivos {#devices}

Cada dispositivo de la tailnet, vigilado o no (dispositivos compartidos
excepto), se publica bajo la familia genérica compartida por toda
integración de DumbMonit que rastrea dispositivos cliente —
`dumbmonit_client_device_*`, con prefijo sin `tailscale`, documentada por
completo en [Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | Qué | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última conexión: «ahora» mientras `connectedToControl` es verdadero, si no `lastSeen` | `device`, `type` (`Tagged` o `Personal`), `os`, `user`, `kind` |
| `stale_seconds` | Positivo una vez que un dispositivo ha superado `device_stale_days` (3 por defecto) sin conectarse; impulsa la regla [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (siempre `connection`: Tailscale no tiene copia de seguridad que reportar) |

Un teléfono o un portátil que viaja puede pasar varios días sin
reconectarse: esto no tiene relación con el ajuste **Devices to keep
online** de arriba, que solo genera una alerta para dispositivos
vigilados que se desconectan. Eleva **Device staleness** para una
tailnet con dispositivos que se desplazan.

## La página del dispositivo {#the-device-page}

El panel Tailnet indica en una frase qué va mal, muestra los
dispositivos en línea, los dispositivos vigilados desconectados, las
actualizaciones disponibles y los dispositivos compartidos, luego lista
los dispositivos vigilados y cualquier dispositivo esperando aprobación o
cuya clave caduca pronto, y después la tabla de todos los dispositivos
con su última conexión.

## Crear un cliente OAuth de solo lectura para DumbMonit {#create-a-read-only-oauth-client-for-dumbmonit}

1. En la consola de Tailscale (login.tailscale.com), abre Settings >
   Trust credentials (antes OAuth clients) y genera un cliente OAuth
   solo con el alcance devices:core:read. Lista los dispositivos y no
   puede cambiar nada.

2. Copia el ID de cliente y el secreto de cliente (tskey-client-…),
   mostrados una vez. En DumbMonit, introduce api.tailscale.com como
   dirección y pégalos como Client ID y Client secret. Deja la opción
   Tailnet vacía: se lee la tailnet del cliente.

!!! warning
    Solo se espera que los dispositivos etiquetados (servidores, routers
    de subred, exit nodes) permanezcan en línea: un teléfono o un
    portátil que duerme no es una caída. Nombra los dispositivos a
    vigilar en la opción Devices to keep online, o introduce all. Cada
    dispositivo de la tailnet aparece en la tabla Devices de abajo con su
    última conexión, vigilado o no; un teléfono que viaja puede pasar
    días sin una, así que eleva Device staleness para esos.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Cliente OAuth | ID de cliente y secreto de cliente (`tskey-client-…`) de un cliente con el alcance `devices:core:read`. |
| Token de acceso de API | Un token `tskey-api-…`, enviado como token bearer. |

Dirección: `api.tailscale.com`. Opciones:

- **Tailnet**: vacío para la tailnet de las credenciales, o el nombre de
  tailnet mostrado en la consola de Tailscale.
- **Devices to keep online**: nombres de dispositivo o etiquetas
  separados por comas (`nas, tag:router`), o `all`. Vacío: cada
  dispositivo etiquetado.
- **Timeout per request** (15 s).
- **Device staleness** (`device_stale_days`, 3 días por defecto).

## Solución de problemas {#troubleshooting}

«Tailscale refused the OAuth client»: el ID o el secreto son
incorrectos, o el cliente fue eliminado.

«Tailscale refused to list the devices (403)»: al cliente le falta el
alcance `devices:core:read`.

«Tailscale knows no tailnet …»: deja la opción Tailnet vacía.
