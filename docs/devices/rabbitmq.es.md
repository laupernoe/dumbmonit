# RabbitMQ

El broker de mensajes: alarmas de memoria y disco, nodos detenidos y
particiones de red, colas que se llenan sin que nadie las lea, colas que
ya no están en ejecución, y el flujo de mensajes.

DumbMonit lee la API de administración con un usuario etiquetado
`monitoring` que tiene permisos vacíos: ve el estado de los nodos, las
alarmas y las colas, y no puede publicar, consumir, leer un mensaje ni
cambiar nada. Un nodo reporta el clúster entero.

El fallo que vale la pena vigilar es el que los clientes no reportan.
Cuando un nodo cruza su marca alta de memoria o su límite de disco libre,
RabbitMQ lanza una alarma y bloquea a todos los publicadores del clúster:
las aplicaciones simplemente se quedan colgadas en `publish`, sin ningún
error. El segundo es una cola que sigue llenándose porque el servicio que
la consumía se detuvo.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_rabbitmq_`. Los totales son
contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | totales del clúster | |
| `messages_ready`, `messages_unacked` | mensajes esperando en todas las colas, y entregados pero aún sin confirmar | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | flujo de mensajes; unroutable son mensajes para los que ninguna cola estaba vinculada a recibir | |
| `alarm` | 1 mientras la alarma está activa; memoria y disco siempre están presentes, a 0 sin alarma | `node`, `resource` |
| `alarms` | alarmas activas en el clúster | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | por nodo | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | memoria frente a la marca alta | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | disco libre frente al límite | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | por cola, las más llenas primero, hasta Queues watched one by one | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | entre esas colas | |

Las alarmas se leen desde la comprobación de salud
`/api/health/checks/alarms`, que las ve de inmediato; las estadísticas de
nodo tienen un retraso de unos segundos.

Las [reglas integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplican: RabbitMQ memory or disk alarm, RabbitMQ node down,
RabbitMQ network partition, RabbitMQ queue without consumer, RabbitMQ
queue unavailable, además de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta RabbitMQ. Indica en una frase qué va mal (alarmas, nodos y
particiones, colas con mensajes sin consumidor, colas no en ejecución),
luego los mensajes listos y sin confirmar, mensajes publicados y
entregados por segundo, conexiones, consumidores y colas, y después las
diez colas más llenas con sus consumidores y la memoria y disco libre de
cada nodo.

## Crear un usuario de RabbitMQ etiquetado monitoring {#create-a-rabbitmq-user-tagged-monitoring}

1. Activa el plugin de administración si no lo está ya (las imágenes
   Docker de administración lo incluyen). Su API escucha en el puerto
   15672.

    ```
    rabbitmq-plugins enable rabbitmq_management
    ```

2. Crea un usuario solo con la etiqueta monitoring: puede leer el estado
   de nodos, alarmas y colas, y no puede cambiar nada.

    ```
    rabbitmqctl add_user dumbmonit 'a-long-password'
    rabbitmqctl set_user_tags dumbmonit monitoring
    ```

3. Dale permisos vacíos en cada virtual host cuyas colas quieras ver.
   Entonces puede listar esas colas, y sigue sin poder publicar, consumir
   ni leer un mensaje. Repite con -p para cada otro virtual host.

    ```
    rabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'
    ```

4. En DumbMonit, introduce la dirección de un nodo, por ejemplo
   «rabbit.lan», o «https://rabbit.lan:15671» cuando la API se sirve por
   TLS. Un nodo reporta el clúster entero: las alarmas y particiones de
   cada nodo, y todas las colas.

!!! warning
    Una alarma de memoria o disco bloquea a todos los publicadores del
    clúster, y los clientes no reciben ningún error: las aplicaciones
    simplemente se quedan colgadas en publish. Las colas se listan las
    más llenas primero, hasta Queues watched one by one (100 por
    defecto); los totales siempre cubren todas las colas.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario / contraseña | El usuario `monitoring` creado arriba, enviado como autenticación básica HTTP. |

Sin permisos en un virtual host, sus colas no se listan una por una; los
totales del clúster igualmente las cuentan.

Dirección: un nombre de host o IP (`rabbit.lan`), `host:puerto`, o una URL
completa con un prefijo de ruta detrás de un proxy inverso. Opciones:
protocolo (HTTP por defecto), puerto (15672), comprobación de certificado,
tiempo de espera de la petición (10 s), y Queues watched one by one (100,
de 0 a 500).
