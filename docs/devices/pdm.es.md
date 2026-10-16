# Proxmox Datacenter Manager

La consola que federa varios clústeres Proxmox VE e instancias de
Proxmox Backup Server: qué instancias alcanza, todo el patrimonio de un
vistazo, las tareas que fallaron en cualquier sitio, y la propia salud
de la consola.

!!! warning "Probado solo contra una instancia de prueba"

    Esta integración se construyó y verificó contra un Proxmox
    Datacenter Manager 1.1 instalado para este propósito, no contra uno
    en producción. Cada llamada es de lectura, y las formas vienen de un
    datacenter manager real, pero todavía nadie la ha ejecutado durante
    semanas en un datacenter manager en producción. Dinos qué falla.

## ¿Un monitor para todo el patrimonio, o uno por clúster? {#one-device-for-the-estate-or-one-device-per-cluster}

Un monitor Datacenter Manager no sustituye a los monitores
[Proxmox VE](proxmox.md) y [Proxmox Backup Server](pbs.md). Responde a
una pregunta distinta.

| | Monitor Datacenter Manager | Un monitor por clúster y por servidor de copias |
|---|---|---|
| Monitores a crear | uno | uno por clúster, uno por servidor de copias |
| Credenciales a gestionar | un token, en la consola | un token por instancia |
| Carga en cada clúster | ninguna nuestra: leemos la propia caché de la consola | una sonda por intervalo, por clúster |
| Qué obtienes | accesibilidad, versión, recuentos de invitados y nodos, totales de CPU, memoria y almacenamiento, tareas fallidas | todo eso más invitados uno por uno, snapshots, trabajos de copia, HA, Ceph, replicación, discos y SMART, pools ZFS, el calendario de copias de 30 días |
| Qué no puedes obtener | series por invitado, detalle por datastore, discos, snapshots, logs de tarea | un total único de todo el patrimonio |
| Fallo principal | la consola no puede alcanzar una instancia | el clúster no nos responde |

**Nuestra recomendación.** Añade el monitor Datacenter Manager
cuando ya ejecutas una consola y quieres una página que diga si el
patrimonio está completo — es la forma más económica de notar que un
sitio remoto se cayó, y no le cuesta nada a los clústeres. Añade un
monitor dedicado para cada clúster y servidor de copias que
realmente operas: ahí está la profundidad, y es lo que te dice *qué* VM
se detuvo o *qué* copia falló. Los dos se superponen sin conflicto; una
copia fallida entonces lanza una alerta desde el monitor PBS (con el
log de tarea) y una desde la consola (nombrando la instancia). Silencia
la regla de tareas de la consola si lo encuentras redundante, o limítala
a las instancias para las que no tienes monitor dedicado.

Si no ejecutas ninguna consola en absoluto, no instales una solo para
monitorizar: los monitores dedicados ven estrictamente más.

## La página del monitor {#the-device-page}

Más allá de las gráficas genéricas, un monitor Datacenter Manager
muestra tres paneles, leídos de lo que guardó la sonda — abrir la
página no consulta ni a la consola ni a los clústeres que gestiona:

* **Federated instances** — una franja de totales del patrimonio
  (instancias, nodos en línea, invitados en ejecución y detenidos,
  núcleos, memoria, almacenamiento), luego una fila por instancia
  federada. Las instancias que la consola no puede alcanzar se ordenan
  primero, luego las que tienen tareas fallidas, luego las que van
  atrasadas en versión. Cada fila da el producto (Proxmox VE o Backup
  Server), la versión, los recuentos de nodo y invitado, el uso de
  memoria y almacenamiento, el estado de suscripción, la dirección
  configurada, cuándo recolectó la consola de ella por última vez — y,
  cuando falló, el mensaje que recibió la propia consola. Ese mensaje
  es lo único que explica la caída, así que se muestra textualmente.
* **Failures** — cada tarea que falló en todo el patrimonio en los
  últimos 14 días, la más reciente primero, con la instancia en la que
  se ejecutó, el tipo de tarea, cuándo empezó, cuánto tardó y el mensaje
  de error. El propio log de la tarea permanece en la instancia: ábrelo
  ahí, o añade ese clúster como su propio monitor, donde *Show log*
  lo obtiene.
* **Console host** — la máquina que ejecuta la consola: CPU, memoria,
  sistema de archivos raíz, tiempo de actividad y kernel, sus
  certificados con los días restantes antes de caducar, los paquetes
  esperando una actualización, y el estado de suscripción de los nodos
  gestionados. Todo el panel es opcional: un token sin `Sys.Audit` no
  lee nada de él y el panel lo indica, en lugar de mostrar ceros.

El historial de tareas se guarda en SQLite (`pdm_task_history`, 19 días,
como máximo 5000 tareas por monitor) así que la lista de fallos
sobrevive a los reinicios aunque cada sonda solo mire la ventana de
tareas.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_pdm_`.

| Familia | Métricas | Etiquetas |
|---|---|---|
| Patrimonio | `remotes_total`, `remotes_failed`, `nodes_online`, `nodes_offline`, `cpu_used_cores`, `cpu_total_cores`, `memory_used_bytes`, `memory_total_bytes`, `memory_used_percent`, `storage_used_bytes`, `storage_total_bytes`, `storage_used_percent`, `datastores_total` | |
| Invitados del patrimonio | `guests_running`, `guests_stopped` | `guest_type` (`qemu`, `lxc`) |
| Instancias | `remote_reachable` (1 la consola obtuvo respuesta, 0 no la obtuvo), `remote_version_info` (serie de presencia, etiqueta `version`), `remote_version_behind`, `remote_nodes_online`, `remote_nodes_offline`, `remote_guests_running`, `remote_guests_stopped`, `remote_cpu_used_cores`, `remote_cpu_total_cores`, `remote_cpu_used_percent`, `remote_memory_used_bytes`, `remote_memory_total_bytes`, `remote_memory_used_percent`, `remote_storage_used_bytes`, `remote_storage_total_bytes`, `remote_storage_used_percent`, `remote_datastores`, `remote_subscription_active`, `remote_updates_pending`, `remote_last_collection_age_seconds`, `remote_tasks_failed` | `remote`, `type` (`pve`, `pbs`) |
| Tareas en la ventana | `tasks_total`, `tasks_failed`, `tasks_running` | |
| Host de la consola | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`, `node_load1`, `node_memory_used_bytes`, `node_memory_total_bytes`, `node_memory_used_percent`, `node_swap_used_bytes`, `node_swap_total_bytes`, `node_rootfs_used_bytes`, `node_rootfs_total_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `node_updates_pending`, `version_info` | |
| Certificados de la consola | `node_certificate_expiry_days` (negativo una vez caducado) | `file` (`proxy.pem`) |
| Suscripción | `subscription_active` (etiqueta `status`), `subscription_active_nodes`, `subscription_total_nodes` | |
| Recolección | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Una tarea que termina en `WARNINGS: n` cuenta como exitosa. Solo `GET
/version` fallando condena toda la sonda; cualquier otra llamada que
falle se cuenta en `scrape_errors`, y una instancia que la consola no
puede alcanzar es una medición (`remote_reachable = 0`), no un error de
sonda.

`remote_version_behind` compara las instancias federadas **entre sí**,
producto por producto: es 1 cuando otro clúster Proxmox VE — u otro
servidor de copias — del mismo patrimonio ejecuta una versión más
reciente. No es una comparación contra lo que publica Proxmox; una
sonda no tiene por qué contactar a internet. La serie solo existe para
una instancia cuya versión se leyó, así que una instancia inaccesible
nunca pasa por actualizada.

Los totales de almacenamiento cuentan cada nombre de almacenamiento una
vez por instancia: un almacenamiento compartido por cinco nodos de un
clúster se cuenta una vez, no cinco.

Las reglas integradas que se aplican: Monitor unreachable, Federated
instance unreachable, Federated instance behind, Task failed on a
federated instance, Datacenter console disk almost full, Datacenter
console certificate expiring, Datacenter console updates pending. Las
notificaciones nombran la instancia afectada.

## Qué preparar en Proxmox Datacenter Manager {#what-to-prepare-in-proxmox-datacenter-manager}

Los pasos de abajo son los que muestra el aviso junto al formulario. El
principio: un usuario reservado para monitorización, un token con un
rol de solo lectura — nunca la cuenta con la que inicias sesión.

1. En la consola, abre Configuration → Access Control → Users y haz
   clic en Add. Nombra la cuenta como sigue y deja la contraseña vacía:
   el token es lo que inicia sesión.

    ```
    dumbmonit@pdm
    ```

2. Todavía en Access Control, abre API Tokens → Add, elige ese usuario
   y nombra el token «monitor». La consola muestra el secreto una vez:
   cópialo ahora.

    ```
    dumbmonit@pdm!monitor
    ```

3. Dale tanto al usuario como al token el rol `Auditor` en `/` (la
   cima del árbol, así que todo el patrimonio). `Auditor` lleva
   `System.Audit`, `Resource.Audit` y `Access.Audit`, y no puede
   cambiar nada. Un token nunca tiene más derechos que su usuario, así
   que el permiso se concede dos veces: Permissions → Add → User
   Permission, y luego de nuevo con API Token Permission.
4. Copia el id del token en el campo Token ID de DumbMonit y el secreto
   en su campo Secret.
5. En DumbMonit, introduce la dirección de la consola, por ejemplo
   «dc.lan» o «dc.lan:8443».

!!! warning
    No reutilices la cuenta con la que inicias sesión: un token
    filtrado entonces controlaría cada clúster que federa la consola a
    la vez. El rol `Auditor` solo puede leer. La consola también usa un
    certificado autofirmado por defecto: si la conexión se rechaza por
    ese motivo, marca «Accept an unverifiable certificate» en las
    opciones. Dos elementos necesitan más que `Auditor` y simplemente se
    omiten sin él: el estado de recolección de métricas por instancia, y
    el resumen de actualizaciones de las instancias federadas.

¿Prefieres la línea de comandos?
`proxmox-datacenter-manager-admin` gestiona remotes, no usuarios; crea
la cuenta, el token y los dos permisos en la interfaz web.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de API (recomendado) | **Token ID**: usuario, reino y nombre de token tal como los muestra la consola, `dumbmonit@pdm!monitor`. **Secret**: el UUID mostrado una vez al crear el token. DumbMonit los envía como la cabecera `PDMAPIToken=user@pdm!name:secret` que espera la consola. |

No hay opción de nombre de usuario / contraseña, y es deliberado: desde
su primera versión la consola devuelve su ticket de sesión solo en una
cookie `HttpOnly` — el cuerpo JSON lleva un `ticket-info` inutilizable.
Iniciar sesión no ganaría nada frente a un token mientras abriría una
sesión registrada en cada sonda. Los tokens son de todas formas lo que
recomienda Proxmox para la automatización.

Derechos, de solo lectura. El más simple y suficiente: el rol integrado
`Auditor` en `/`, concedido al usuario *y* al token. Por endpoint:

| Endpoint | Necesita |
|---|---|
| `GET /version` | cualquier usuario autenticado |
| `GET /remotes/remote` | devuelve las instancias que el llamante puede auditar |
| `GET /remotes/remote/{id}/version` | `Resource.Audit` en `/resource/{id}` |
| `GET /resources/status`, `GET /resources/list` | `Resource.Audit` en `/resource` |
| `GET /resources/subscription` | `Resource.Audit` en `/resource` |
| `GET /remotes/tasks/list` | `Resource.Audit` en `/resource/{remote}`, por instancia |
| `GET /nodes/{node}/status` | `System.Audit` en `/system/status` |
| `GET /nodes/{node}/certificates/info`, `GET /nodes/{node}/subscription` | cualquier usuario autenticado |
| `GET /nodes/{node}/apt/update` | `System.Audit`; un 403 se tolera: sin serie `node_updates_pending`, sin error de sonda |
| `GET /remotes/metric-collection/status` | más que `Auditor`; un 403 se tolera: sin serie `remote_last_collection_age_seconds`, sin error de sonda |
| `GET /remotes/updates/summary` | `Resource.Modify` — un privilegio de escritura. Desactivado por defecto (`remote_updates`) por ese motivo |

Nota que la consola redacta los tokens de acceso que conserva para sus
instancias (`GET /remotes/remote` devuelve un `token` vacío), y
DumbMonit nunca lee ese campo de todas formas: ningún secreto de
instancia pasa nunca por el recolector, la vista guardada o la API.

Dirección: `10.0.0.40`, `dc.lan`, `dc.lan:8443`, `[fd00::2]` o una URL
completa. El esquema `https` y el puerto 8443 se añaden si faltan.

## Opciones {#options}

| Clave | Etiqueta | Por defecto | Ayuda |
|---|---|---|---|
| `port` | API port | `8443` | Se usa si la dirección no da un puerto. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Proxmox Datacenter Manager viene con un certificado autofirmado por defecto: activa esto si la conexión se rechaza por ese motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `20` | Tiempo permitido para cada llamada a la API, de 1 a 120. La consola reenvía llamadas a cada instancia federada, así que un sitio lento retiene toda la respuesta. |
| `node` | Console node name | `localhost` | Nombre del nodo que ejecuta la consola. Un Datacenter Manager tiene un único nodo y lo llama «localhost». |
| `task_lookback_hours` | Task window (hours) | `24` | Las tareas más antiguas no se cuentan entre los fallos, de 1 a 8760. |
| `max_age_seconds` | Accepted cache age (seconds) | `60` | Cuán obsoleto puede ser el propio inventario de la consola antes de que vuelva a consultar las instancias federadas, de 0 a 3600. Cero pide datos frescos en cada sonda, lo cual vuelve a poner cada clúster en línea. |
| `remotes` | Monitored instances | *(vacío)* | Nombres de las instancias federadas a monitorizar, separados por comas. Vacío: cada instancia. |
| `max_remotes` | Instance version limit | `100` | Número máximo de instancias a las que se pide su versión en cada sonda, de 1 a 1000. |
| `versions` | Read each instance version | `true` | Una llamada por instancia federada, para reportar su versión y señalar las que van rezagadas respecto a sus pares. Una instancia que no responde cuenta como inaccesible. |
| `tasks` | Watch tasks across instances | `true` | Lee la lista de tareas de cada instancia federada para reportar los fallos. |
| `node_status` | Watch the console host | `true` | Lee la CPU, memoria, sistema de archivos raíz, tiempo de actividad, certificados y suscripción de la máquina que ejecuta la consola. |
| `updates` | Count pending updates | `true` | Lista los paquetes esperando una actualización en la propia consola. |
| `remote_updates` | Count updates on each instance | `false` | Lee el resumen de actualizaciones de la consola para las instancias federadas. Desactivado por defecto: la consola protege esa ruta con `Resource.Modify`, un privilegio de escritura que una cuenta de monitorización no tiene razón para tener. |

`max_age_seconds` es el que vale la pena entender. La consola mantiene
su propio inventario, refrescado por su propio bucle de recolección.
Pedir datos frescos en cada sonda hace que la consola vuelva a llamar a
cada clúster, encima de su ritmo normal — exactamente la carga que un
único monitor de consola debería evitar. Sesenta segundos sigue
siendo más fresco que el intervalo de sonda por defecto.

## Errores comunes {#common-errors}

| Síntoma | Causa probable |
|---|---|
| Error de certificado | Certificado autofirmado: instala uno de confianza (ACME está integrado en la consola) o activa `insecure_tls`. |
| Error de autenticación | El secret se pegó en Token ID (o al revés), o el permiso se concedió al usuario pero no al token — un token nunca tiene más derechos que su usuario, y las dos ACL son entradas separadas. |
| Cada instancia se muestra inaccesible, pero la propia página de la consola está bien | El token tiene `Auditor` en el usuario pero no en el token, o solo en `/system`: `Resource.Audit` en `/resource` es lo que permite a la consola responder por sus instancias. |
| Una instancia es inaccesible y el mensaje menciona una huella | La consola verifica el certificado de cada instancia que gestiona; arréglalo en la consola (Remotes → la instancia), no aquí. |
| Sin serie `remote_last_collection_age_seconds` | `GET /remotes/metric-collection/status` necesita más que `Auditor` y se omite en silencio. Nada más se afecta. |
| Sin serie `remote_updates_pending` | `remote_updates` está desactivado por defecto porque esa ruta necesita `Resource.Modify`. Actívalo solo si estás dispuesto a conceder un privilegio de escritura. |
| Sin panel de host de consola | Al token le falta `System.Audit` en `/system/status`, o `node_status` está desactivado. |
| «Federated instance behind» nunca se activa | Compara instancias del mismo producto entre sí. Una consola que federa un único clúster Proxmox VE no tiene nada con que compararlo. |
| Los totales del patrimonio son todos rayas | La consola respondió pero no contó nada — normalmente porque cada instancia es inaccesible. Las filas de instancia dicen por qué. |
