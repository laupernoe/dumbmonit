# Caddy

El servidor web y proxy inverso: upstreams que fallan sus comprobaciones
de salud, una recarga de configuración que falló, errores de handler y la
proporción de respuestas 5xx.

DumbMonit lee tres cosas de la API de administración de Caddy, todas con
`GET`: `/config/apps/http/servers` (la configuración HTTP que Caddy
ejecuta, nunca la sección `tls` donde viven los tokens de proveedor DNS),
`/reverse_proxy/upstreams` (los backends y los fallos que recuerda la
comprobación de salud pasiva) y `/metrics` (salud de upstream, recargas,
peticiones y errores).

La API de administración de Caddy no es de solo lectura: quien la
alcance puede cargar una nueva configuración o detener Caddy. Por eso
escucha en `localhost:2019`. La configuración de abajo la mantiene ahí y
hace que el propio Caddy publique una vista de solo lectura de ella,
protegida por contraseña, que responde `GET` en las tres rutas de arriba
y rechaza todo lo demás.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_caddy_`. Los totales son
contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `config_loaded` | 1 cuando Caddy ejecuta al menos un servidor HTTP | |
| `servers`, `routes` | servidores HTTP y rutas en la configuración | |
| `config_last_reload_ok` | 0 cuando la última recarga de configuración falló | |
| `config_last_reload_timestamp_seconds` | hora de la última recarga exitosa | |
| `upstreams`, `upstreams_unhealthy` | backends de cada `reverse_proxy`, y los que fallan | |
| `upstream_healthy` | 1 cuando ni la comprobación activa ni la pasiva descartan el backend | `upstream` |
| `upstream_fails` | peticiones fallidas que la comprobación de salud pasiva recuerda actualmente | `upstream` |
| `upstream_requests` | peticiones en curso hacia el backend | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | con la opción `metrics`: peticiones, respuestas 5xx y errores de handler | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | lo mismo por servidor (`srv0`, `srv1`… o los nombres que les diste) | `server` |

`upstream_healthy` necesita `/metrics`: la comprobación de salud activa
solo es visible ahí. Una vista de solo lectura que no deja pasar
`/metrics` igualmente reporta los backends y sus fallos recordados.
Caddy no publica su versión a través de la API de administración.

Las [reglas integradas](../alerting/rules.md#reverse-proxies-and-domains)
que se aplican: Caddy upstream unhealthy, Caddy configuration reload
failed, Reverse proxy answering 5xx, además de Monitor unreachable.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Caddy. Indica en una frase qué va mal (sin configuración,
una recarga fallida, upstreams no saludables, una proporción alta de
5xx), luego los servidores, rutas, upstreams, tasa de peticiones,
proporción de 5xx y errores de handler a lo largo de 24 horas, y después
cada upstream con sus fallos recordados.

## Publicar una vista de solo lectura de la API de administración de Caddy {#publish-a-read-only-view-of-caddys-admin-api}

1. La API de administración de Caddy puede cargar una nueva
   configuración y detener Caddy: debe permanecer fuera del alcance de
   cualquier cosa que no sea DumbMonit. Déjala donde Caddy la pone, en
   localhost:2019, y haz que el propio Caddy publique una vista de solo
   lectura de ella: solo GET, en las tres rutas que lee DumbMonit,
   detrás de una contraseña.

2. Crea el hash de contraseña para el usuario dumbmonit.

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

3. Añade este sitio al Caddyfile, con ese hash en lugar de HASH, y luego
   recarga Caddy. Responde en el puerto 2020 y rechaza todo salvo esas
   tres lecturas.

    ```
    :2020 {
    basic_auth {
    dumbmonit HASH
    }
    @readonly {
    method GET
    path /config/apps/http/servers /reverse_proxy/upstreams /metrics
    }
    handle @readonly {
    reverse_proxy localhost:2019 {
    header_up Host localhost:2019
    }
    }
    respond 403
    }
    ```

4. Opcional: añade metrics al bloque de opciones globales en la parte
   superior del Caddyfile para contar peticiones, errores y respuestas
   5xx por servidor. La salud de upstream y las recargas se reportan sin
   ello.

    ```
    metrics
    ```

5. Filtra el puerto 2020 para que solo el host de DumbMonit lo alcance.
   En DumbMonit, introduce el host de Caddy con ese puerto, por ejemplo
   «caddy.lan:2020», y el usuario dumbmonit con su contraseña.

!!! warning
    Nunca publiques la propia API de administración en la red sin un
    cortafuegos delante: cualquiera que alcance el puerto 2019 puede
    sustituir la configuración o detener Caddy, y la opción origins no
    detiene a un cliente que no envía ninguna cabecera Origin. Si la
    expones en una red privada, filtra el puerto para que solo el host
    de DumbMonit la alcance, y elige No password.

El Caddyfile conserva su significado sin sangría; `caddy fmt` lo
sangra. `header_up Host localhost:2019` es necesario porque la API de
administración solo responde a peticiones dirigidas a la dirección en la
que escucha. Con la imagen Docker de Caddy, ejecuta el comando de hash
con `docker exec`.

Una configuración cargada como JSON en lugar de desde un Caddyfile
obtiene el mismo sitio como un servidor extra con un handler
`basic_auth`, un matcher `method` y `path` y un `reverse_proxy` hacia
`localhost:2019`.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Vista de solo lectura | El usuario y contraseña del bloque `basic_auth` de arriba (recomendado). |
| Sin contraseña | Nada: la propia API de administración, filtrada para que solo DumbMonit la alcance. |

Dirección: un nombre de host o IP con el puerto de la vista de solo
lectura (`caddy.lan:2020`), o una URL completa. Opciones: protocolo (HTTP
por defecto), puerto (2019, la propia API de administración),
comprobación de certificado y tiempo de espera de la petición (10 s).

La comprobación de salud pasiva (`fail_duration`) solo recuerda los
fallos durante esa duración: un backend que falla una vez y se recupera
se reporta no saludable hasta entonces. La regla espera cinco minutos
antes de activarse.
