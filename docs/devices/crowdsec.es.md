# CrowdSec

El motor de seguridad: decisiones activas y de dónde vienen, alertas,
bouncers que dejaron de obtener actualizaciones, líneas de log leídas y
las que ningún parser reconoció, y si la Local API responde.

DumbMonit lee las métricas Prometheus de CrowdSec (puerto 6060) y
comprueba `/health` de la Local API (puerto 8080). Con una clave de
bouncer opcional, también pide a la Local API las decisiones sobre
192.0.2.1, una dirección de documentación: la respuesta siempre está
vacía, y demuestra que la autenticación y la base de datos responden.
Nunca crea ni elimina una decisión y nunca descarga la lista de bloqueo.

El fallo que vale la pena vigilar es un bouncer que dejó de obtener
actualizaciones. El cortafuegos o proxy inverso sigue funcionando con la
lista que tenía, los nuevos atacantes pasan, y nada se queja. CrowdSec no
publica cuándo un bouncer obtuvo su última actualización; DumbMonit lo lee
del contador de peticiones del bouncer, que deja de moverse. El segundo es
una adquisición que se rompió (un archivo de log renombrado, un
contenedor recreado): CrowdSec no lee nada, no ve ningún ataque, y se
mantiene en verde.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_crowdsec_`. Los totales
son contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `lapi_up` | 1 cuando la Local API responde a `/health` | |
| `lapi_decisions_ok` | con una clave de bouncer: 1 cuando la Local API respondió a la consulta de decisión | |
| `decisions` | decisiones activas, listas de bloqueo comunitarias incluidas | |
| `decisions_by_origin` | | `origin`: `crowdsec`, `cscli`, `CAPI`, `lists`… |
| `alerts` | alertas conservadas por la Local API, excluidas las comunitarias | |
| `bouncers_seen` | bouncers que llamaron a la Local API desde que CrowdSec arrancó | |
| `bouncer_requests_total` | peticiones de cada bouncer; cuando deja de moverse, el bouncer dejó de obtener actualizaciones | `bouncer` |
| `machine_requests_total` | peticiones de cada agente (machine) | `machine` |
| `lapi_requests_total` | todas las peticiones a la Local API | |
| `scenario_overflows_total` | escenarios activados por el agente local: cada uno es una alerta | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | líneas de log leídas por el agente, y las analizadas o no | |
| `source_lines_total`, `source_lines_unparsed_total` | lo mismo por fuente de log | `source` |
| `buckets` | buckets de escenario actualmente abiertos | |

Las familias de la Local API aparecen donde se ejecuta la Local API, las
familias de log donde un agente lee logs; la imagen oficial y una
instalación estándar ejecutan ambas.

Las [reglas integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplican: CrowdSec Local API down, CrowdSec bouncer stopped
pulling, CrowdSec reads no logs, además de Monitor unreachable.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta CrowdSec. Indica en una frase qué va mal (Local API,
bouncers, lectura de logs y la proporción de líneas que ningún parser
reconoció), luego las decisiones activas, alertas conservadas, escenarios
activados en las últimas 24 horas y líneas leídas por segundo, y después
el tiempo desde la última actualización de cada bouncer, las decisiones
por origen y la proporción no analizada de cada fuente de log.

## Dejar que DumbMonit lea las métricas de CrowdSec {#let-dumbmonit-read-crowdsecs-metrics}

1. CrowdSec publica sus métricas en 127.0.0.1:6060 por defecto. Define
   listen_addr en la sección prometheus de /etc/crowdsec/config.yaml con
   una dirección que DumbMonit pueda alcanzar, y luego reinicia
   CrowdSec. La imagen Docker oficial ya escucha en todas las interfaces.

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. Las métricas no necesitan ninguna credencial: contienen recuentos,
   nombres de bouncer y machine y rutas de archivos de log, nunca una
   dirección IP. Filtra el puerto 6060 para que solo el host de DumbMonit
   lo alcance, y comprueba que responde.

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. DumbMonit también comprueba /health de la Local API, en el puerto 8080
   del mismo host: listen_uri en la sección api.server debe entonces
   también ser accesible. Desmarca Check the Local API para un host que
   solo ejecuta un agente.

4. Opcional: crea una clave de bouncer para DumbMonit y pégala como
   Bouncer API key abajo. Una clave de bouncer solo puede leer
   decisiones. DumbMonit pide las decisiones sobre 192.0.2.1, una
   dirección de documentación, así que la respuesta siempre está vacía y
   la lista de bloqueo nunca se descarga.

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. En DumbMonit, introduce el host de CrowdSec, por ejemplo
   «crowdsec.lan». Cuando la Local API y los agentes se ejecutan en hosts
   distintos, añade cada host: las decisiones, alertas y bouncers se
   cuentan donde se ejecuta la Local API, las líneas de log donde cada
   agente las lee.

!!! warning
    Un bouncer que deja de obtener actualizaciones sigue aplicando una
    lista congelada: los nuevos atacantes pasan y nada se queja.
    DumbMonit avisa cuando las peticiones de un bouncer se detienen
    durante 30 minutos. Solo conoce a los bouncers que obtuvieron al
    menos una actualización desde el último arranque de CrowdSec.

Un bouncer eliminado con `cscli bouncers delete` conserva su contador
hasta que CrowdSec se reinicia, y se reporta como detenido hasta
entonces. Un bouncer que solo llama a la Local API cuando sirve una
petición (un bouncer de proxy inverso en modo live) puede quedarse
callado durante media hora en una noche tranquila; cámbialo a modo
stream, o eleva la ventana de la regla.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Solo métricas | Nada: lee `/metrics`, y `/health` de la Local API. |
| Clave de API de bouncer | La clave impresa por `cscli bouncers add dumbmonit`, enviada como `X-Api-Key` a la Local API. |

Dirección: un nombre de host o IP (`crowdsec.lan`), `host:puerto` para las
métricas, o una URL completa. Opciones: protocolo (HTTP por defecto),
puerto de métricas (6060), Check the Local API (activado), puerto de la
Local API (8080), comprobación de certificado y tiempo de espera de la
petición (10 s). Los elementos del hub (parsers, escenarios, colecciones)
contaminados o desactualizados no son visibles a través de las métricas,
solo con `cscli hub list` en el host.
