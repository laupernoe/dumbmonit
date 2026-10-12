# Immich

Si Immich va al día con lo que se sube: colas de trabajos que se atascan
o se quedan en pausa, actualizaciones, el disco que contiene la
biblioteca, cuántas fotos y vídeos contiene, y — para cada teléfono,
tableta y escritorio con sesión iniciada en la cuenta — su última
conexión y su última copia de seguridad.

DumbMonit lee la API de Immich con una **clave de API limitada a ocho
permisos de lectura**. La clave no puede ver una foto, un álbum ni una
persona, y no puede cambiar nada.

El fallo que vale la pena vigilar: **trabajos que ya no se ejecutan**.
Una cola pausada durante una importación grande y olvidada, o un worker
de trabajos que se detuvo, y las fotos nuevas no obtienen miniatura, ni
metadatos, ni cara ni búsqueda inteligente, mientras las subidas siguen
teniendo éxito. Immich solo lo muestra en su página Jobs.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_immich_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 cuando la propia comprobación de versión de Immich conoce una versión más reciente (ausente cuando la comprobación está desactivada) | `latest_version` |
| `maintenance_mode` | 1 mientras Immich está en modo mantenimiento | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | el disco que contiene la biblioteca | |
| `photos`, `videos`, `usage_bytes`, `users` | tamaño de la biblioteca, todos los usuarios juntos | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | todas las colas juntas | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | por cola: esperando (incluidos retrasados y retenidos por una pausa), en ejecución, fallidos, en pausa | `queue` |
| `statistics_readable` | 0 cuando la clave pertenece a una cuenta corriente y se omiten los recuentos y las colas | |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

Las versiones recientes de Immich retiran los trabajos fallidos de la
cola una vez registrados, así que `queue_failed` normalmente marca 0; un
trabajo que falló se muestra en el propio registro de Immich.

Las [reglas integradas](../alerting/rules.md#self-hosted-applications)
que se aplican: Immich jobs stalled, Immich queue paused, Immich storage
almost full, Immich update available, además de Device unreachable.

## Dispositivos {#devices}

Cada dispositivo con sesión iniciada en la cuenta propietaria de la
clave, con su última conexión y última copia de seguridad, se publica
bajo una familia genérica compartida por toda integración de DumbMonit
que rastrea dispositivos cliente — `dumbmonit_client_device_*`, con
prefijo sin `immich`, documentada por completo en [Dispositivos
cliente](../alerting/rules.md#client-devices):

| Métrica | Qué | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última conexión (la última actividad de la sesión) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | Última copia de seguridad; ver el límite abajo | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo una vez que un dispositivo ha superado `device_stale_days` (3 por defecto) sin conectarse ni hacer copia de seguridad; impulsa la regla [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (`connection` o `backup`) |

**Límite que conviene conocer**: la API de Immich no registra qué
dispositivo subió una foto concreta, así que `last_backup_timestamp_seconds`
es la subida más reciente de toda la biblioteca — el mismo valor para
cada dispositivo, publicado bajo un dispositivo llamado `library` en
lugar de uno real. Detecta todos los dispositivos deteniéndose a la vez
(almacenamiento lleno, la cuenta deslogueada en todas partes); no
detectará un solo dispositivo quedándose callado mientras otro sigue
subiendo. Solo `last_seen_timestamp_seconds` — la última vez que la
sesión de cada dispositivo estuvo activa — es genuinamente por
dispositivo.

La tabla de la página del dispositivo muestra estas cifras para cada
dispositivo, con un estado de OK o Stale.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca llama a Immich. Indica primero qué va mal (colas atascadas, colas
en pausa, mantenimiento), luego actualizaciones, y después fotos,
vídeos, tamaño de la biblioteca, disco usado y trabajos esperando y en
ejecución, luego las colas con trabajo, por nombre, y después la tabla
de dispositivos (nombre, tipo, SO, última conexión, última copia de
seguridad, estado).

## Crear una clave de API de solo lectura en Immich {#create-a-read-only-api-key-in-immich}

1. Inicia sesión en Immich con una cuenta que pueda abrir Administration:
   Immich solo deja a esas cuentas leer los recuentos de fotos y las
   colas de trabajos. Abre Account Settings → API Keys → New API Key,
   nómbrala como sigue, y marca estos ocho permisos: los primeros cinco
   para colas de trabajos, recuentos y versión, los últimos tres para
   listar los dispositivos de la cuenta y su última conexión y copia de
   seguridad.

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read session.read asset.read user.read
    ```

2. Copia la clave ahora: Immich la muestra solo una vez. Lee esas ocho
   páginas y nada más: ninguna foto, ningún álbum, ningún ajuste. Si tu
   Immich no ofrece queue.read, marca job.read en su lugar.

3. Una clave hecha por una cuenta corriente también funciona: se leen la
   versión, las actualizaciones, el espacio en disco y los dispositivos;
   se omiten los recuentos de fotos y las colas de trabajos.

4. En DumbMonit, introduce la dirección de Immich, por ejemplo
   «immich.lan» (puerto 2283) o «https://photos.example.com» detrás de
   un proxy inverso, y pega la clave.

!!! warning
    Una cola en pausa es fácil de olvidar: las fotos nuevas no obtienen
    miniatura, metadatos ni cara, e Immich solo lo indica en su página
    Jobs. DumbMonit avisa cuando hay trabajos esperando en una cola que
    no está en pausa y no ejecuta nada. session.read, asset.read y
    user.read listan todos los dispositivos de la cuenta propietaria de
    la clave — sus teléfonos, tabletas y navegadores — con la última
    conexión de cada uno. La API de Immich no registra qué dispositivo
    subió una foto concreta, así que «última copia de seguridad» es la
    subida más reciente de toda la biblioteca, mostrada igual para cada
    dispositivo: si solo un dispositivo deja de hacer copia mientras los
    demás siguen, su conexión se mostrará obsoleta pero la cifra de copia
    compartida seguirá viéndose fresca.

Lo que abre cada permiso: `server.about` la versión
(`/api/server/about`), `server.versionCheck` la última versión que
conoce Immich, `server.storage` el disco, `server.statistics` los
recuentos de fotos y vídeos, `queue.read` las colas de trabajos
(`/api/queues`; las versiones más antiguas responden en `/api/jobs` con
`job.read`), `session.read` los dispositivos de la cuenta
(`/api/sessions`), `asset.read` la subida más reciente de la biblioteca
(`/api/search/metadata`), `user.read` el nombre mostrado de la cuenta
(`/api/users/me`).

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Clave de API | Enviada en la cabecera `x-api-key`. |

Dirección: un nombre de host (`immich.lan`), `host:puerto`, o una URL
completa con un prefijo de ruta; se acepta un `/api` final. Opciones:
protocolo (HTTP por defecto), puerto (2283), comprobación de certificado,
tiempo de espera de la petición (15 s), y antigüedad de dispositivo
(`device_stale_days`, 3 días por defecto).

Validado contra Immich v3.2.4 (imágenes oficiales, aprendizaje automático
desactivado); los endpoints de dispositivo se comprobaron contra la
etiqueta `v3.2.4` de la propia especificación OpenAPI de Immich
(`immich-app/immich`), que confirmó que `/api/search/metadata` ya no
acepta un filtro `deviceId` — la razón por la que «última copia de
seguridad» es a nivel de toda la biblioteca en lugar de por dispositivo.
