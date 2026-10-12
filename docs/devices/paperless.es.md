# Paperless-ngx

Si Paperless-ngx sigue importando lo que se le envía: el estado de su base
de datos, Redis, los workers de Celery, el índice de búsqueda, el
clasificador y la comprobación de integridad, tareas fallidas,
actualizaciones, espacio en disco y el número de documentos.

DumbMonit lee la API REST de Paperless-ngx con un **usuario dedicado que no
puede leer ningún documento**: tres permisos de vista le dan la lista de
tareas, el estado del sistema y los recuentos globales, nada más.

El fallo que vale la pena vigilar: **Redis o Celery detenidos**. La
interfaz web se abre y los documentos se pueden leer, pero nada de lo que
se deja en la carpeta de consumo o se obtiene por correo se importa hasta
que alguien se da cuenta del montón. El estado del sistema lo indica;
DumbMonit alerta sobre ello.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_paperless_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 cuando la propia comprobación de versión de Paperless-ngx conoce una versión más reciente (ausente cuando la comprobación está desactivada) | `latest_version` |
| `component_status` | 0 OK, 1 aviso, 2 error, tal como lo reporta la página de estado del sistema | `component`: `database`, `redis`, `celery`, `index`, `classifier`, `sanity_check`, `llm_index` (solo cuando está activado) |
| `unapplied_migrations` | migraciones de base de datos aún no aplicadas | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | el disco que contiene los medios | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | todas las tareas de los últimos 30 días: fallidas, total, y aún pendientes | |
| `tasks_failed_recent` | tareas fallidas que este usuario puede ver, más recientes que la ventana de tareas fallidas y no descartadas | |
| `task_failed` | valor 1 por cada tarea así (como máximo 32) | `task`, `file` |
| `documents`, `documents_inbox` | documentos en Paperless, y en la bandeja de entrada | |
| `status_readable` | 0 cuando la cuenta no tiene el permiso de monitorización del sistema | |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

Paperless mantiene privadas las tareas de cada propietario: el usuario
dedicado ve las tareas programadas (comprobación de integridad, índice,
clasificador, correo) por nombre, mientras que las importaciones de otros
usuarios se cuentan en `tasks_failed_window`, que abarca todas las tareas.

Las [reglas integradas](../alerting/rules.md#self-hosted-applications) que
se aplican: Paperless task queue down, Paperless index or sanity check
error, Paperless migrations pending, Paperless task failed, Paperless
storage almost full, Paperless update available, además de Device
unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca llama a Paperless. Indica primero qué va mal (un componente en
error, tareas fallidas, migraciones pendientes), luego actualizaciones, y
después documentos, bandeja de entrada, tareas pendientes y fallidas y
espacio en disco, y por último cada componente y las tareas fallidas por
nombre.

## Crear un usuario de solo lectura en Paperless-ngx {#create-a-read-only-user-in-paperless-ngx}

1. Crea un usuario con el nombre siguiente, con una contraseña larga y
   aleatoria, sin estado de superusuario ni de staff, y solo tres permisos
   de vista: tareas, monitorización del sistema y estadísticas globales.
   Con la API y tu propia cuenta, los dos comandos de abajo lo crean e
   imprimen su token.

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. En la interfaz web en su lugar: Users & Groups → Add user, y luego en
   sus permisos marca View solo en las filas PaperlessTask,
   SystemMonitoring y GlobalStatistics. Inicia sesión como ese usuario una
   vez, abre My Profile y genera su token de API.

3. Este usuario no lee ningún documento en absoluto: las estadísticas
   globales dan los recuentos, la monitorización del sistema da el estado
   de la base de datos, Redis, Celery y el índice, y las tareas muestran
   las fallidas. En Paperless-ngx 2.x, donde los dos últimos permisos no
   existen, el estado y los recuentos se omiten y se lee el resto.

4. En DumbMonit, introduce la dirección, por ejemplo «paperless.lan»
   (puerto 8000) o «https://paperless.example.com» detrás de un proxy
   inverso, y pega el token.

!!! warning
    Cuando Redis o Celery se detienen, Paperless sigue sirviendo sus
    páginas y documentos, pero nada nuevo se importa, ni desde la carpeta
    de consumo ni desde el correo, hasta que alguien lo revisa. DumbMonit
    alerta sobre ambos.

Sustituye `CHANGE-ME` por una contraseña larga y aleatoria antes de
ejecutar los comandos, y `YOUR_USER` por una cuenta con permiso para
gestionar usuarios (curl pide su contraseña). Pedir el estado del sistema
puede tardar varios segundos mientras Redis está caído: Paperless lo
espera antes de responder.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de API (recomendado) | El token del usuario dedicado, enviado como `Authorization: Token …`. |
| Nombre de usuario / contraseña | El propio usuario dedicado, enviado como autenticación básica HTTP. |

Dirección: un nombre de host (`paperless.lan`), `host:puerto`, o una URL
completa con un prefijo de ruta; se acepta un `/api` final. Opciones:
protocolo (HTTP por defecto), puerto (8000), comprobación de certificado,
tiempo de espera de la petición (15 s) y la ventana de tareas fallidas (24
horas, de 1 a 720).

Validado contra Paperless-ngx 3.2.1 (imagen oficial, SQLite, Valkey).
