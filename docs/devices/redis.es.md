# Redis y Valkey

El propio almacén en memoria: memoria frente a `maxmemory`, conexiones
rechazadas, el enlace de replicación y el retraso de cada réplica,
snapshots fallidos y escrituras del append-only file, y el tamaño del
keyspace.

DumbMonit abre una conexión, envía `AUTH`, luego `INFO`, y la cierra. La
cuenta que usa puede ejecutar `INFO` y `PING` y nada más: no puede leer,
escribir ni siquiera listar una clave. Se cubren Redis 6 y posteriores,
Valkey y los forks que responden a `INFO` de la misma forma (KeyDB,
Dragonfly).

Los fallos que vale la pena vigilar son los silenciosos. En `maxmemory`,
Redis o bien desaloja claves sin avisar a nadie o rechaza toda escritura
con `OOM command not allowed`, según `maxmemory-policy`; sin `maxmemory`,
crece hasta que el kernel lo mata. Un snapshot fallido hace que Redis
rechace las escrituras mientras `stop-writes-on-bgsave-error` esté
activado, que lo está por defecto. Una réplica cuyo enlace con su primario
está caído sigue respondiendo lecturas, con datos obsoletos.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_redis_`. Los totales son
contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | valor 1 | `role`: `master` o `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | memoria según la cuenta Redis, y según el sistema | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` y la proporción usada; ausente cuando `maxmemory` es 0 (sin límite) | |
| `maxmemory_policy_info` | valor 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | conexiones rechazadas porque se alcanzó `maxclients` | |
| `evicted_keys_total`, `expired_keys_total` | claves desalojadas en `maxmemory`, claves caducadas | |
| `keyspace_hits_total`, `keyspace_misses_total` | búsquedas que encontraron su clave, y las que no | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | respuestas de error enviadas, e inicios de sesión rechazados (una contraseña siendo adivinada aparece aquí) | |
| `keys`, `keys_expiring` | claves en todas las bases de datos, y las que tienen caducidad; siempre presente, incluso en cero | |
| `db_keys` | claves por base de datos | `db` |
| `rdb_last_save_ok` | 0 cuando el último snapshot (`BGSAVE`) falló | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | append-only file; los dos estados solo cuando está activado | |
| `loading` | 1 mientras Redis carga su dataset al arrancar | |
| `connected_replicas` | réplicas conectadas a este primario | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | visto desde el primario, por réplica: en línea o no, segundos desde su última confirmación, bytes de retraso | `replica` (`ip:puerto`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | en una réplica: su enlace con el primario | |
| `uptime_seconds`, `cluster_enabled` | | |

Las [reglas integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplican: Redis memory near maxmemory, Redis refusing connections,
Redis replication link down, Redis replica lagging, Redis snapshot
failed, Redis append-only file write failed, además de Device
unreachable.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta Redis. Indica en una frase qué va mal (replicación, memoria
frente a `maxmemory` y la política que se aplica en el límite,
persistencia, conexiones rechazadas), luego las claves, memoria,
clientes, comandos por segundo, ratio de aciertos, claves desalojadas en
las últimas 24 horas y tiempo de actividad, y después las claves por base
de datos y el retraso de cada réplica.

## Crear un usuario de Redis que solo pueda ejecutar INFO {#create-a-redis-user-that-may-only-run-info}

1. En Redis 6 o posterior, o Valkey, crea un usuario que pueda ejecutar
   INFO y PING y nada más, sin acceso a ninguna clave. En redis-cli, con
   sesión iniciada con un usuario autorizado a gestionar ACLs:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Hazlo permanente: ACL SAVE si el servidor carga sus usuarios desde un
   aclfile, CONFIG REWRITE en caso contrario.

    ```
    ACL SAVE
    ```

3. En Redis 5 o anterior, o solo con requirepass definido, elige
   Password only abajo. Esa contraseña lleva todos los derechos: prefiere
   un usuario ACL siempre que el servidor lo soporte.

4. En DumbMonit, introduce la dirección del servidor, por ejemplo
   «cache.lan» o «cache.lan:6380». Si el servidor solo acepta TLS en ese
   puerto (tls-port), marca TLS abajo.

5. DumbMonit envía AUTH, luego INFO. Nunca lee, escribe ni lista una
   clave.

!!! warning
    Redis no da ningún aviso antes de maxmemory: en el límite, o bien
    desaloja claves en silencio o rechaza toda escritura, según
    maxmemory-policy. Sin maxmemory crece hasta que el kernel lo mata.
    DumbMonit avisa por encima del 90 % de maxmemory.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Usuario ACL (recomendado) | El usuario creado arriba y su contraseña, enviados como `AUTH usuario contraseña`. |
| Solo contraseña | El valor de `requirepass`, enviado como `AUTH contraseña`. |
| Sin autenticación | Para un servidor sin ninguna contraseña. |

Dirección: un nombre de host o IP (`cache.lan`), `host:puerto`, o una
cadena de conexión (`redis://cache.lan:6380`; cualquier cosa después del
puerto se ignora, y las credenciales escritas en ella nunca se usan).
Opciones: puerto (6379), TLS (desactivado), comprobación de certificado, y
tiempo de espera para toda la comprobación (10 s). Con TLS activado, el
certificado se comprueba contra las autoridades públicas incluidas en
DumbMonit; para una autoridad privada, marca Accept an unverifiable
certificate.

Una contraseña rechazada (`WRONGPASS`, `NOAUTH`, `NOPERM`) se muestra como un
error de credencial y nadie recibe notificación del error en sí, pero entonces
DumbMonit no recopila nada: la regla Monitor unreachable sigue activándose, como
con cualquier monitor que no puede leer, hasta que la credencial funcione
de nuevo. Redis reiniciándose sin su usuario ACL (usuarios definidos con
`ACL SETUSER` pero nunca guardados) provoca exactamente esto.

En un despliegue Redis Sentinel o Cluster, añade cada servidor que quieras
vigilar: cada uno reporta su propia memoria, clientes y persistencia, y un
primario reporta sus réplicas. El propio Sentinel no está cubierto.
