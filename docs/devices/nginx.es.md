# Nginx

El servidor web y proxy inverso: conexiones activas y en espera, las tasas
de accept/handled/request, y el estado de los upstream cuando la API
comercial de NGINX Plus está disponible.

DumbMonit lee el módulo `stub_status` integrado de Nginx, un puñado de
contadores en texto plano, sin ninguna API ni cuenta necesarias en una red
correctamente filtrada. Cuando el NGINX Plus comercial está instalado,
DumbMonit también puede leer su API JSON para el estado de los upstream y
los contadores por zona.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_nginx_`. Las métricas
`_total` son contadores: las gráficas y las reglas las convierten en
tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `connections_active` | conexiones actualmente abiertas | |
| `connections_reading`, `connections_writing`, `connections_waiting` | conexiones según lo que están haciendo | |
| `accepts_total`, `handled_total` | conexiones aceptadas, y realmente atendidas | |
| `requests_total` | peticiones servidas desde que Nginx arrancó | |
| `version_info` | presencia; la cabecera de respuesta `Server`, cuando Nginx la envía | `server` |
| `plus_upstream_server_up` | 1 cuando NGINX Plus marca el servidor como activo | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | contadores por servidor de NGINX Plus | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | contadores por zona de NGINX Plus | `zone` |

Una diferencia entre `accepts_total` y `handled_total` significa
conexiones que Nginx descartó antes de poder servirlas (normalmente se
alcanzó `worker_connections`). Las métricas `plus_*` solo aparecen cuando
«Read the NGINX Plus API» está marcado y NGINX Plus responde; la versión
de código abierto no tiene «upstream» que reportar solo con
`stub_status`.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Nginx. Muestra las conexiones activas, en lectura,
escritura y espera, las tasas de accept/handled/request a lo largo de 24
horas, y, con NGINX Plus, el estado de los upstream y el tráfico por zona.

## Activar el módulo stub_status de Nginx {#enable-nginxs-stub_status-module}

1. Nginx trae stub_status integrado: solo necesita una location que lo
   active. Añade esto al bloque server que Nginx ya sirve (o a su propio
   bloque server en un puerto propio), y luego recarga Nginx.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Restringe esa location al host de DumbMonit, para que nadie más pueda
   leerla; ajusta la dirección según corresponda.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Comprueba que responde antes de configurar DumbMonit.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. En DumbMonit, introduce el host de Nginx, por ejemplo «nginx.lan», y la
   ruta si no es «/basic_status». Si NGINX Plus está instalado, marca
   «Read the NGINX Plus API» para añadir el estado de los upstream y los
   contadores por zona.

!!! warning
    stub_status solo cuenta conexiones y peticiones: un error de
    configuración en un bloque server, o un backend que nginx no puede
    alcanzar, no aparece aquí. La versión de código abierto no tiene
    noción de «upstream»: eso solo existe en NGINX Plus, leído a través de
    su propia API cuando está activada.

El ejemplo de arriba restringe la location por IP, la configuración más
simple para un homelab; un bloque `basic_auth` funciona igual de bien si
la página de estado no puede estar detrás de su propia regla de
cortafuegos. El propio `stub_status` no se puede proteger con contraseña
directamente desde Nginx — la contraseña, si la hay, pertenece al bloque
`location` que lo rodea.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Sin contraseña | Nada: la página de estado se filtra de modo que solo DumbMonit la alcance. |
| Basic auth | El usuario y contraseña del bloque que protege la página de estado, si lo hay. |

Dirección: un nombre de host o IP, con el puerto si no es 80. Opciones:
protocolo (HTTP por defecto), puerto, ruta de estado (`/basic_status` por
defecto), comprobación de certificado, tiempo de espera de la petición (10
s), y el interruptor y versión de la API de NGINX Plus.
