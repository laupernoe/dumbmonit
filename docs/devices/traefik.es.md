# Traefik

El proxy inverso: routers desactivados por un error de configuración,
servidores backend que fallan su comprobación de salud, certificados que
un resolver nunca obtuvo o que no se renuevan, y la proporción de
respuestas 5xx.

DumbMonit lee la API de Traefik, que solo lee: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (servido
por Traefik 3.7) y `/api/version`. Cuando las métricas Prometheus de
Traefik se sirven en el mismo puerto, `/metrics` añade la tasa de
peticiones y la proporción de respuestas 5xx. Ningún método de la API
cambia nada, y DumbMonit no envía ninguno.

Los fallos que vale la pena vigilar son los silenciosos. Un router cuyo
servicio o middleware no existe, o cuya regla no se puede analizar, se
desactiva: sus sitios responden 404, y Traefik solo lo indica en su
registro. Un servicio cuyos servidores fallan todos su comprobación de
salud responde 503 «no available server». Un router que pide a un
resolver ACME un certificado que el resolver no puede obtener recibe el
certificado autofirmado por defecto de Traefik, que los navegadores
rechazan.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_traefik_`. Los totales
son contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | routers de todos los protocolos, y los en error o aviso | |
| `services`, `services_errors`, `middlewares_errors` | lo mismo para servicios y middlewares | |
| `router_status` | por router HTTP: 0 activado, 1 aviso, 2 desactivado | `router`, `provider` |
| `router_error_info` | valor 1, para un router no activado: el primer error de Traefik | `router`, `error` |
| `service_status` | por servicio: 0 activado, 1 aviso, 2 desactivado | `service` |
| `service_servers`, `service_servers_up` | servidores de un servicio con balanceo de carga, y los que pasan su comprobación de salud | `service` |
| `server_up` | 1 cuando el servidor pasa su comprobación de salud (o no tiene ninguna) | `service`, `server` |
| `resolver_routers` | routers que piden un certificado a un resolver de certificados | `resolver` |
| `resolver_routers_uncovered` | aquellos cuyos nombres de host ningún certificado que tiene Traefik cubre todavía (necesita `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificados que tiene Traefik, y los días restantes antes de que cada uno caduque | `cert` |
| `requests_total`, `requests_5xx_total` | con métricas: peticiones de cada entry point, y las respondidas con 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | con métricas y etiquetas de router: lo mismo por router | `router` |
| `config_reloads_total` | con métricas: recargas de configuración | |

Los propios routers y servicios de Traefik (`@internal`) se excluyen. Los
routers, servicios y servidores se describen uno por uno hasta 500 cada
uno; los totales siempre cubren todo.

Las [reglas integradas](../alerting/rules.md#reverse-proxies-and-domains)
que se aplican: Traefik router disabled, Traefik backend server down,
Traefik service without a server, Traefik certificate not obtained,
Proxy certificate not renewed, Reverse proxy answering 5xx, además de
Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Traefik. Indica en una frase qué va mal (routers
desactivados y el motivo de Traefik, servicios sin servidor, resolvers
que no obtuvieron nada, certificados próximos a caducar, una proporción
alta de 5xx), luego los routers, servicios, tasa de peticiones y
proporción de 5xx, y después los servidores activos por servicio, los
certificados que caducan antes primero, y las respuestas 5xx por router
a lo largo de 24 horas.

## Dejar que DumbMonit lea la API de Traefik {#let-dumbmonit-read-traefiks-api}

1. La API de Traefik solo lee: routers, servicios y certificados, nunca
   un cambio. Actívala junto con las métricas Prometheus, que añaden la
   tasa de peticiones y la proporción de respuestas 5xx: añade estos dos
   argumentos al comando de Traefik (o las mismas claves a su
   configuración estática) y reinícialo. Ambas se sirven entonces en el
   entry point traefik, puerto 8080.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. El puerto 8080 responde sin contraseña: publícalo solo en una
   dirección que alcance el host de DumbMonit, fíltralo para que nada
   más lo haga, y comprueba que responde.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Para exigir una contraseña en su lugar, dirige /api y /metrics a
   través de un entry point propio con un middleware basicAuth, como
   muestra la documentación, y elige basicAuth user abajo. Crea el
   usuario dumbmonit con htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. En DumbMonit, introduce el host de Traefik, por ejemplo «traefik.lan»,
   con el puerto de ese entry point si no es 8080. Desmarca Read the
   metrics si /metrics se sirve en otro sitio.

!!! warning
    Un router desactivado por un error de configuración tira abajo sus
    sitios, y Traefik solo lo indica en su registro. La comprobación de
    que un resolver de certificados obtuvo sus certificados necesita
    /api/certificates, que Traefik 3.7 sirve; sin ella, la caducidad de
    certificados viene solo de las métricas.

### Un entry point de API con contraseña {#an-api-entry-point-with-a-password}

En lugar de `--api.insecure=true`, dale a la API y a las métricas un
entry point propio y pon un middleware basicAuth delante de ambos. En la
configuración estática:

```yaml
entryPoints:
  monitor:
    address: ":8082"
api: {}
metrics:
  prometheus:
    manualRouting: true
    addRoutersLabels: true
```

y en la configuración dinámica (se muestra el proveedor de archivo; las
etiquetas Docker funcionan igual), con la línea impresa por `htpasswd`:

```yaml
http:
  middlewares:
    dumbmonit-auth:
      basicAuth:
        users:
          - "dumbmonit:$2y$05$…"
  routers:
    monitor-api:
      entryPoints: [monitor]
      rule: "PathPrefix(`/api`)"
      service: api@internal
      middlewares: [dumbmonit-auth]
    monitor-metrics:
      entryPoints: [monitor]
      rule: "Path(`/metrics`)"
      service: prometheus@internal
      middlewares: [dumbmonit-auth]
```

Luego introduce `traefik.lan:8082` en DumbMonit con el usuario
`dumbmonit`. `addRoutersLabels` es lo que da las respuestas 5xx por
router; sin él solo se cuentan los totales por entry point.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Sin contraseña | Nada: el puerto de la API se filtra para que solo DumbMonit lo alcance. |
| Usuario basicAuth | El usuario y contraseña del middleware basicAuth delante de `/api` y `/metrics`. |

Dirección: un nombre de host o IP (`traefik.lan`), `host:puerto`, o una
URL completa. Opciones: protocolo (HTTP por defecto), puerto de la API
(8080), Read the metrics (activado), comprobación de certificado y
tiempo de espera de la petición (10 s).

Un servicio sin comprobación de salud reporta cada servidor como
activo: Traefik solo marca un servidor caído cuando falla un
`healthCheck`. Los servicios weighted y mirroring no tienen servidores
propios y no se cuentan.
