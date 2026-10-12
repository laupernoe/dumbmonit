# Graylog

El servidor de logs y el clúster de búsqueda detrás de él: si Graylog
procesa mensajes, cuántos esperan en su journal, cuán llenos están sus
buffers, qué entra y qué sale, el estado de OpenSearch o Elasticsearch
como lo ve Graylog, inputs que no arrancaron, y mensajes que no se
indexaron.

DumbMonit lee la API REST de Graylog con un usuario dedicado que tiene el
rol integrado **Reader**. Nunca busca mensajes, nunca inicia o detiene un
input y nunca confirma una notificación.

El fallo que vale la pena vigilar: **el clúster de búsqueda deja de ir al
día**. Graylog sigue aceptando mensajes y los escribe en su journal en
disco; la interfaz web se abre, las búsquedas funcionan, y simplemente no
muestran nada reciente. El journal crece hasta llenarse, y entonces se
descartan mensajes. El backlog del journal, el estado del clúster de
búsqueda y los fallos de indexación lo indican mucho antes.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_graylog_`. El journal, los
buffers y el caudal son los **del nodo con el que habla DumbMonit**: en un
clúster Graylog, añade cada nodo como su propio dispositivo.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `processing` | 1 mientras el nodo procesa mensajes, 0 cuando el procesamiento está en pausa | |
| `running`, `lb_alive` | el ciclo de vida del nodo es `running`; se reporta vivo a los balanceadores de carga | |
| `version_info` | valor 1 | `version` |
| `journal_uncommitted_entries` | mensajes escritos en el journal y aún no procesados | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | cuán llenos están los buffers de entrada, proceso y salida | `buffer` |
| `messages_in_total`, `messages_out_total` | mensajes recibidos, y escritos hacia el clúster de búsqueda | |
| `indexer_status` | estado del clúster de búsqueda: 0 verde, 1 amarillo, 2 rojo, 3 inaccesible desde Graylog | |
| `indexer_shards` | shards por estado: `active`, `initializing`, `relocating`, `unassigned` | `state` |
| `inputs_running`, `inputs_failed` | inputs en este nodo | |
| `input_failed` | valor 1 por input que no arrancó | `input` (su título) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | escrituras al clúster de búsqueda que fallaron, mensajes que fallaron en el procesamiento o indexación, mensajes con un timestamp inválido | |
| `notifications`, `notifications_urgent`, `notification` | las propias notificaciones de Graylog (permiso opcional, ver abajo) | `type`, `severity` |
| `cluster_nodes` | nodos en el clúster Graylog | |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

El clúster de búsqueda se lee a través de Graylog, así que no se necesita
un dispositivo OpenSearch o Elasticsearch separado para saber si está
verde; cuando Graylog no puede alcanzarlo en absoluto, `indexer_status`
marca 3.

Las [reglas integradas](../alerting/rules.md#log-and-metrics-servers) que
se aplican: Graylog not processing, Graylog search cluster down, Graylog
search cluster yellow, Graylog journal filling, Graylog input failed,
Graylog indexing failures, además de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Graylog. Indica en una frase qué va mal (procesamiento,
clúster de búsqueda, backlog del journal, inputs fallidos por nombre,
fallos de indexación, notificaciones urgentes), luego los mensajes
entrantes y salientes por segundo, el backlog del journal y el número de
nodos, y después los buffers, los shards por estado y las propias
notificaciones de Graylog.

## Crear un usuario y token de solo lectura en Graylog {#create-a-read-only-user-and-token-in-graylog}

1. En Graylog: System → Users and Teams → Create user. Nombra el usuario
   como sigue, dale una contraseña larga y aleatoria, y asígnale solo el
   rol Reader. Reader puede leer el estado del nodo, el journal, los
   buffers, el caudal, los inputs y el estado del clúster de búsqueda, y
   ningún mensaje en absoluto mientras no se le comparta ningún stream.

    ```
    dumbmonit
    ```

2. Abre la página Edit tokens de ese usuario, crea un token con el nombre
   de DumbMonit y cópialo ahora: Graylog lo muestra solo una vez.

3. Opcional: las propias notificaciones de Graylog (un input que no
   arrancó, marcas de agua de disco, un journal casi lleno) necesitan un
   permiso más, notifications:read, que ningún rol integrado concede por
   sí solo. Con una cuenta autorizada a gestionar roles, crea un rol que
   solo tenga ese permiso y dáselo al usuario dumbmonit. Sin él, las
   notificaciones se omiten y nada más cambia.

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. En DumbMonit, introduce la dirección del nodo Graylog, por ejemplo
   «graylog.lan» (puerto 9000) o «https://graylog.lan» detrás de un proxy
   inverso, y pega el token. En un clúster Graylog, añade cada nodo: el
   journal, los buffers y el caudal son por nodo.

5. DumbMonit solo lee. Nunca busca mensajes, nunca inicia o detiene un
   input y nunca confirma una notificación.

!!! warning
    No reutilices la cuenta con la que inicias sesión: un token filtrado
    lleva todos los derechos de su usuario, mientras que el rol Reader de
    arriba no puede cambiar nada. Graylog sirve HTTP plano a menos que
    configures TLS, y el token viaja con cada petición: en una red no
    confiable, pon Graylog detrás de HTTPS.

Lo que cubre cada permiso de Reader: `system:read` el estado del nodo y
los nodos del clúster, `journal:read` el journal, `metrics:read` y
`throughput:read` el caudal y los buffers, `inputs:read` los inputs,
`indexercluster:read` el estado del clúster de búsqueda. El rol Reader
también tiene `messages:read`, que le permite leer mensajes solo de los
streams compartidos explícitamente con él: no compartas ninguno. Los
detalles de fallos de indexación (`indices:failures`) no son necesarios:
DumbMonit cuenta los fallos desde las propias métricas de Graylog.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de acceso (recomendado) | Un token del usuario dedicado. Graylog lo espera como autenticación básica HTTP con el token como nombre de usuario y la palabra `token` como contraseña; DumbMonit lo hace por ti. |
| Nombre de usuario / contraseña | El propio usuario dedicado. |

Dirección: un nombre de host o IP (`graylog.lan`), `host:puerto`, o una URL
completa (`https://graylog.lan`); se acepta un `/api` final. Opciones:
protocolo (HTTP por defecto), puerto (9000), comprobación de certificado y
tiempo de espera de la petición (10 s).
