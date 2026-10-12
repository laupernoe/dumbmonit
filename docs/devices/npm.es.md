# Nginx Proxy Manager

El gestor de proxy: hosts cuya configuración nginx rechazó, hosts
desactivados, y los certificados que presentan los hosts, antes de que un
fallo de renovación de Let's Encrypt se convierta en un sitio caducado.

DumbMonit inicia sesión en la API REST de Nginx Proxy Manager (`/api`, en
el puerto de su interfaz web, 81) con un usuario propio, y luego lee los
hosts proxy, los hosts de redirección, los hosts 404 (llamados dead hosts
en la API) y los streams. Cada host indica si está activado y si nginx
aceptó su configuración. Un host que nginx rechazó no se sirve, y la
interfaz web solo lo muestra como un punto rojo.

Los certificados no se leen desde la API. `/api/nginx/certificates` entrega
la clave privada de los certificados importados, y las credenciales del
desafío DNS, a cualquier usuario autorizado a ver certificados. DumbMonit
lee en su lugar cada certificado donde se sirve: un handshake TLS en el
puerto 443 de Nginx Proxy Manager, con el nombre de un host que lo usa. Eso
también mide el certificado que los visitantes realmente reciben.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_npm_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | hosts de cada tipo | `type`: `proxy`, `redirection`, `dead` (hosts 404), `stream` |
| `hosts_offline` | hosts activados cuya configuración nginx rechazó | `type` |
| `host_online` | por host activado: 0 cuando nginx rechazó su configuración | `type`, `proxy_host` |
| `host_error_info` | valor 1, para un host que nginx rechazó: la primera línea del error de nginx | `type`, `proxy_host`, `error` |
| `certificates_checked`, `certificates_read` | certificados adjuntos a un host activado, y los leídos en el puerto HTTPS | |
| `cert_expiry_days` | días restantes antes de que caduque el certificado que presenta un host | `proxy_host` |

`proxy_host` es el primer nombre de dominio del host, o `port 9000` para un
stream. Los hosts desactivados se cuentan, no se describen: desactivar un
host es deliberado, y su alerta, si la hay, se resuelve. Los hosts se
describen uno por uno hasta 500, y como máximo se leen 100 certificados por
comprobación.

Las [reglas integradas](../alerting/rules.md#reverse-proxies-and-domains)
que se aplican: Nginx Proxy Manager host offline, Proxy certificate not
renewed, además de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Nginx Proxy Manager. Indica en una frase qué va mal (hosts
que nginx rechazó, con su motivo, certificados próximos a caducar,
certificados que no se pudieron leer), luego los hosts proxy, las
redirecciones, los hosts 404, los streams y los hosts desactivados, y
después los certificados que caducan antes primero.

## Crear un usuario de solo lectura en Nginx Proxy Manager {#create-a-view-only-nginx-proxy-manager-user}

1. En Nginx Proxy Manager, abre Users y añade un usuario para DumbMonit,
   con una dirección de correo propia como dumbmonit@example.com. Deja
   todos los roles sin marcar.

2. En el menú de ese usuario, abre Permissions: Item Visibility All Items;
   Proxy Hosts, Redirection Hosts, 404 Hosts y Streams View Only; Access
   Lists y SSL Certificates Hidden. Luego define su contraseña con Change
   Password en el mismo menú.

3. SSL Certificates se deja en Hidden a propósito: Nginx Proxy Manager
   entrega la clave privada de los certificados importados, y las
   credenciales del desafío DNS, a cualquier usuario autorizado a verlos.
   DumbMonit lee en su lugar cada certificado donde se sirve, con un
   handshake TLS en el puerto 443 para cada host que tiene uno.

4. En DumbMonit, introduce el host de Nginx Proxy Manager con el puerto de
   su interfaz web, por ejemplo «npm.lan:81», y el correo y contraseña de
   ese usuario. Si los hosts se sirven por HTTPS en un puerto distinto de
   443, define HTTPS port abajo.

!!! warning
    Un host cuya configuración nginx rechaza (una errata en Advanced, un
    archivo de certificado faltante) no se sirve, y Nginx Proxy Manager
    solo lo muestra como un punto rojo en la lista. Los certificados Let's
    Encrypt se renuevan treinta días antes: uno que caduca en menos de
    catorce días es uno cuya renovación falló, y DumbMonit avisa entonces.

Item Visibility All Items importa: con Created Items, el usuario solo ve
los hosts que él mismo creó, que son ninguno. Un usuario sin Proxy Hosts
View Only es rechazado; uno sin Streams, Redirection Hosts o 404 Hosts
simplemente no los reporta.

DumbMonit conserva el token que Nginx Proxy Manager entrega al iniciar
sesión (válido un día) y vuelve a iniciar sesión cuando caduca, en lugar de
hacerlo en cada comprobación.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Correo y contraseña del usuario | La dirección de correo y la contraseña del usuario creado arriba. |

Dirección: un nombre de host o IP con el puerto de la interfaz web
(`npm.lan:81`), o una URL completa. Opciones: protocolo (HTTP por
defecto), puerto de la interfaz web (81), Check the certificates
(activado), puerto HTTPS (443), comprobación de certificado de la API y
tiempo de espera de la petición (10 s).

La comprobación de certificado se conecta a la parte de host de la
dirección: cuando la interfaz web se alcanza a través de otro proxy, apunta
la dirección al propio Nginx Proxy Manager, o desmarca Check the
certificates y vigila cada sitio con una [comprobación de certificado
TLS](services.md) en su lugar. Los certificados wildcard se leen a través
del primer host que los usa.
