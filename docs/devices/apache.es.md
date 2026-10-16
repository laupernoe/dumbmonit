# Apache httpd

El servidor web: workers ocupados y libres, el scoreboard de workers por
estado, y las tasas de peticiones/bytes cuando ExtendedStatus está
activado.

DumbMonit lee el `mod_status` integrado de Apache, consultado con `?auto`
para que responda en líneas `Clave: Valor` planas en lugar de la página
HTML pensada para un navegador.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_apache_`. Las métricas
`_total` son contadores: las gráficas y las reglas las convierten en
tasas. Todo lo que está por debajo de `workers_busy`/`workers_idle` y
`scoreboard` necesita `ExtendedStatus On`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `workers_busy`, `workers_idle` | procesos o hilos worker, por estado | |
| `scoreboard` | slots de worker por estado del scoreboard | `state` |
| `requests_total` | peticiones servidas desde que Apache arrancó | |
| `sent_bytes_total` | bytes enviados desde que Apache arrancó | |
| `uptime_seconds` | tiempo desde que Apache arrancó | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | promedios móviles propios de Apache | |
| `cpu_load_ratio` | el propio valor `CPULoad` de Apache | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | recuentos de conexiones async (MPMs event y worker) | |
| `version_info` | presencia; la cabecera de respuesta `Server` | `version` |

`scoreboard` siempre suma el número configurado de slots de worker: los
estados son waiting, open_slot, starting, reading, sending, keepalive,
dns_lookup, closing, logging, finishing e idle_cleanup.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Apache. Muestra workers ocupados y libres, el desglose del
scoreboard, y, con ExtendedStatus, las tasas de peticiones y bytes a lo
largo de 24 horas.

## Activar mod_status de Apache {#enable-apaches-mod_status}

1. Asegúrate de que mod_status esté cargado (viene con Apache y suele
   estar activado por defecto).

    ```
    a2enmod status
    ```

2. Añade una ubicación de estado, restringida al host de DumbMonit; ajusta
   la dirección según corresponda. En Debian y Ubuntu esto va en un
   archivo propio, en otros sitios en la configuración principal.

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Activa los contadores de peticiones y bytes (opcional, pero vale la
   pena): sin ello, solo se reportan los workers ocupados/libres y el
   scoreboard.

    ```
    ExtendedStatus On
    ```

4. Recarga Apache y comprueba que responde antes de configurar DumbMonit.

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. En DumbMonit, introduce el host de Apache, por ejemplo «apache.lan», y
   la ruta si no es «/server-status» (DumbMonit añade «?auto» por su
   cuenta).

!!! warning
    Sin ExtendedStatus, las peticiones y los bytes no se reportan en
    absoluto, no como cero: la gráfica se queda vacía en lugar de mentir
    en plano. Apache no publica su propia versión en mod_status; DumbMonit
    la lee en su lugar de la cabecera de respuesta Server, que algunas
    distribuciones recortan a propósito.

En Debian y Ubuntu, la ubicación de estado ya suele existir en
`/etc/apache2/mods-available/status.conf`; editar ahí la línea `Require`
basta, sin necesidad de un archivo nuevo. Donde la configuración es un
único archivo (la mayoría de las demás distribuciones), añade el bloque al
virtual host principal en su lugar.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Sin contraseña | Nada: la página de estado se filtra de modo que solo DumbMonit la alcance. |
| Basic auth | El usuario y contraseña del bloque que protege la página de estado, si lo hay. |

Dirección: un nombre de host o IP, con el puerto si no es 80. Opciones:
protocolo (HTTP por defecto), puerto, ruta de estado (`/server-status` por
defecto, `?auto` añadido automáticamente), comprobación de certificado y
tiempo de espera de la petición (10 s).
