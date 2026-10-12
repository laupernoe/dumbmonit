# MongoDB

La propia base de datos de documentos: el replica set (miembros, su
estado, un primario ausente, cuánto se retrasa cada secundario), las
conexiones frente al límite, la caché de WiredTiger, operaciones y
aserciones.

DumbMonit inicia sesión con una cuenta que solo tiene el rol integrado
`clusterMonitor`, ejecuta `serverStatus`, y `replSetGetStatus` cuando el
servidor pertenece a un replica set. Ese rol lee el estado del servidor y
de la replicación, y ningún documento de ninguna base de datos. Se cubre
MongoDB 3.6 y posteriores; DumbMonit habla el protocolo de red (wire
protocol) por sí mismo, sin driver.

Los fallos que vale la pena vigilar: un replica set que perdió su primario
no acepta ninguna escritura hasta que una elección tenga éxito; un
secundario que se retrasa es el que será elegido el día que el primario
muera, sin las últimas escrituras; en el límite de conexiones se rechaza a
los clientes nuevos; con más del 20 % de la caché de WiredTiger sucia, las
consultas tienen que desalojar páginas ellas mismas y todo se ralentiza.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_mongodb_`. Los totales son
contadores: las gráficas y las reglas los convierten en tasas.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `process`, `storage_engine` |
| `role_info` | valor 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | actuales frente a actuales más disponibles | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | operaciones desde el arranque | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | aserciones desde el arranque; `type` es `regular` (errores internos del servidor), `warning`, `msg`, `user` (errores devueltos a los clientes), `tripwire` o `rollovers`. `regular` y `user` son las que merecen una gráfica | `type` |
| `queued_operations`, `active_clients` | operaciones esperando un bloqueo, clientes ejecutando una | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | la caché de WiredTiger y cuán llena está | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | páginas modificadas aún no escritas | |
| `wiredtiger_cache_read_bytes_total` | bytes leídos del disco hacia la caché | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | valor 1, en un replica set | `set`, `members` |
| `replset_primary_present` | 1 cuando un miembro es primario, 0 cuando ninguno lo es | |
| `replset_member_state` | el número de estado del miembro | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1 cuando el miembro es accesible, 0 cuando no lo es | `member` |
| `replset_member_lag_seconds` | cuánto se retrasa un secundario respecto al primario | `member` |

Las [reglas integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplican: MongoDB replica set without primary, MongoDB member
unreachable, MongoDB replication lag, MongoDB connections near the limit,
MongoDB cache under pressure, MongoDB internal errors, además de Device
unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta MongoDB. Indica en una frase qué va mal (replica set,
conexiones, caché, errores internos), luego las conexiones, operaciones
por segundo, caché usada, memoria residente, operaciones en cola y tiempo
de actividad, y después el estado y el retraso de cada miembro del replica
set y las operaciones por segundo por tipo.

## Crear una cuenta de MongoDB con el rol clusterMonitor {#create-a-mongodb-account-with-the-clustermonitor-role}

1. En mongosh, con sesión iniciada con un usuario autorizado a crear
   usuarios, crea una cuenta solo con el rol integrado clusterMonitor. Lee
   el estado del servidor y de la replicación, y ningún documento de
   ninguna base de datos.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit inicia sesión con SCRAM-SHA-256, lo por defecto para cuentas
   creadas en MongoDB 4.0 y posteriores. Si creaste la cuenta en otra base
   de datos distinta de admin, define Authentication database abajo.

3. En DumbMonit, introduce la dirección de un mongod, por ejemplo
   «db1.lan» o «db1.lan:27018». En un replica set, añade cada miembro:
   cada uno reporta sus propias conexiones y caché, y cualquiera de ellos
   reporta el estado y el retraso de todos los miembros. Si el servidor
   requiere TLS, marca TLS abajo.

4. DumbMonit solo ejecuta serverStatus y replSetGetStatus. Nunca lista una
   colección y nunca lee un documento.

!!! warning
    Una dirección mongodb+srv:// no se resuelve: introduce en su lugar el
    host de cada miembro. MongoDB anterior a 3.6 y los routers mongos no
    están soportados.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario / contraseña | La cuenta `clusterMonitor` creada arriba, con sesión iniciada con SCRAM-SHA-256. |
| Sin autenticación | Para un servidor iniciado sin control de acceso. |

Una cuenta que solo tiene credenciales SCRAM-SHA-1 (creada antes de
MongoDB 4.0 y nunca actualizada) es rechazada: actualiza su contraseña una
vez con
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Dirección: un nombre de host o IP (`db1.lan`), `host:puerto`, o una cadena
de conexión `mongodb://` con un único host. Opciones: puerto (27017), base
de datos de autenticación (`admin`), TLS (desactivado), comprobación de
certificado, y tiempo de espera para toda la comprobación (10 s).
