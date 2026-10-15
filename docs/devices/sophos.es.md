# Sophos Firewall

Un Sophos Firewall (SFOS) a través de su API XML: el enlace de cada
interfaz ligada a una zona, y qué conexiones IPsec están activadas.

Eso es, deliberadamente, todo. La API XML es una API de configuración:
además del estado de conectividad de solo lectura de cada interfaz, no
expone ningún estado en vivo. `<Get><IPSecConnection>` devuelve si un
administrador activó una conexión, no si el túnel está levantado; la alta
disponibilidad, las licencias y suscripciones y la versión del firmware no
están en absoluto incluidos. DumbMonit publica la activación como tal y
nunca la presenta como un estado de túnel.

Cada minuto DumbMonit envía dos peticiones a
`https://<firewall>:4444/webconsole/APIController` (campo de formulario
`reqxml`, nunca en la URL, donde la contraseña acabaría en los registros):
`<Get><Interface>` y `<Get><IPSecConnection>`. Cada una lleva el nombre de
usuario y contraseña de una cuenta cuyo perfil de acceso al dispositivo es
de solo lectura en todas partes: el firewall aplica ese perfil también a
la API.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de la documentación de la API
    XML de Sophos (las entidades `Interface` e `IPSecConnection`, los
    mensajes de inicio de sesión y de estado), no contra un firewall en
    ejecución. La redacción exacta del estado de la interfaz
    (`Connected, 1000 Mbps…` o `Disconnected`) proviene de esa
    documentación y de informes de la comunidad. Dinos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_sophos_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `api_version_info` | `api_version` | Valor 1: la versión de API que anuncia el firewall (`2000.2`), que sigue la versión de SFOS. |
| `interface_link_up` | `interface`, `zone` | 1 cuando el estado indica Connected, 0 cuando Disconnected; ausente cuando el estado es otra cosa. |
| `interface_enabled` | `interface`, `zone` | 0 cuando la interfaz está apagada. |
| `interface_down` | `interface`, `zone` | 1 cuando la interfaz está encendida, ligada a una zona, y no tiene enlace. Los puertos libres (zona None) se dejan en paz. |
| `interfaces`, `interfaces_down` | | Recuentos. |
| `ipsec_connection_activated` | `connection` | 1 cuando la conexión está activada en la configuración. No el estado del túnel. |
| `ipsec_connections` | | Recuento. |

La [regla integrada](../alerting/rules.md#sophos-firewall) que se aplica:

- **Sophos interface without link**: una interfaz encendida y ligada a una
  zona no ha tenido enlace durante cinco minutos (Advisory).

Para contadores de tráfico, añade también el firewall como [dispositivo
SNMP](snmp.md): SFOS responde al MIB de interfaz estándar.

## La página del monitor {#the-device-page}

El panel Firewall indica si cada interfaz de una zona tiene su enlace,
luego lista esas interfaces y las conexiones IPsec con su activación
configurada, etiquetada como tal.

## Permitir que DumbMonit lea la API XML de Sophos Firewall {#allow-dumbmonit-to-read-the-sophos-firewall-xml-api}

1. En Profiles > Device access, añade un perfil llamado DumbMonit
   read-only y pon todos los permisos en Read-only.

2. En Authentication > Users, añade un usuario llamado dumbmonit, de tipo
   Administrator, con ese perfil y una contraseña larga y aleatoria.

3. En Backup & firmware > API (Administration > API access desde SFOS 22),
   activa la configuración de la API y añade la dirección del servidor
   DumbMonit a las direcciones IP permitidas.

4. En DumbMonit, introduce la dirección del firewall, por ejemplo
   «sophos.lan», con el nombre de usuario y contraseña de dumbmonit. La API
   escucha en el puerto de administración, 4444 por defecto, con un
   certificado autofirmado: marca Accept an unverifiable certificate a
   menos que hayas instalado el tuyo.

!!! warning
    La API XML es una API de configuración: reporta el enlace de cada
    interfaz y si cada conexión IPsec está activada, no si el túnel está
    levantado. La alta disponibilidad, las licencias y la versión del
    firmware no están incluidas.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario y contraseña | El usuario `dumbmonit` con el perfil de acceso al dispositivo de solo lectura. |

Dirección: el nombre de host o IP del firewall, `host:puerto`, o una URL.
Opciones:

- **Port** (4444): el puerto de administración.
- **Accept an unverifiable certificate**: el propio certificado del
  firewall es autofirmado.
- **Timeout per request** (15 s).

## Solución de problemas {#troubleshooting}

«Sophos Firewall refused the request (534 …)»: la dirección de DumbMonit
no está entre las direcciones IP permitidas de la configuración de la API.

«Sophos Firewall refused the login»: nombre de usuario o contraseña
incorrectos, o una contraseña con caracteres que el firewall rechaza en
las llamadas a la API; una contraseña larga de letras y cifras evita el
problema.

«Sophos Firewall did not answer in XML»: puerto incorrecto, o la API está
desactivada.
