# VMware vSphere

Un vCenter Server y los hosts que gestiona, o un host ESXi independiente:
hosts que dejan de responder, se quedan en modo mantenimiento o reportan
un estado de salud rojo, máquinas virtuales y sus VMware Tools,
datastores que se llenan o se vuelven inaccesibles, y las alarmas que el
propio vSphere lanzó.

DumbMonit lee vSphere con un usuario dedicado que tiene el rol integrado
**Read-only**. Nunca enciende ni apaga una VM, nunca entra en modo
mantenimiento y nunca confirma una alarma.

El fallo que vale la pena vigilar: **un datastore que se llena**. Cuando
un datastore thin provisioned alcanza el 100 %, toda VM que necesita
escribir en él se pausa de golpe. Las alarmas que lanza vCenter también lo
indican, pero solo a quien tenga el cliente abierto.

## Por qué la Web Services API {#why-the-web-services-api}

DumbMonit habla con la vSphere Web Services API (SOAP, `/sdk`), no con la
API REST (`/api/vcenter/*`): un host ESXi independiente no tiene API REST,
y en un vCenter la API REST no indica el modo mantenimiento, la salud
general (`overallStatus`), el estado de las VMware Tools ni las alarmas
activadas. Se lee una vista de todo el inventario en una sola pasada,
página por página, y se libera al final.

La sesión se conserva de una sonda a la siguiente y se vuelve a abrir solo
cuando caduca: vCenter no registra un nuevo inicio de sesión cada minuto.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_vsphere_`. Los valores de
salud siguen el `overallStatus` de vSphere: 0 verde, 1 amarillo, 2 rojo, 3
gris (desconocido).

| Métrica | Qué | Etiquetas |
|---|---|---|
| `info` | valor 1 | `product` (`vcenter` o `esxi`), `version`, `build` |
| `hosts` | hosts por estado de conexión: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 conectado, 1 desconectado por un administrador, 2 sin respuesta | `esxi` |
| `host_power_state` | 0 encendido, 1 en espera, 2 apagado, 3 desconocido | `esxi` |
| `host_maintenance` | 1 en modo mantenimiento | `esxi` |
| `host_status` | salud general | `esxi` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | solo hosts conectados | `esxi` |
| `vms` | VMs por estado de encendido: `poweredOn`, `poweredOff`, `suspended` (plantillas excluidas) | `state` |
| `templates` | plantillas de VM | |
| `vm_power_state` | 0 encendida, 1 apagada, 2 suspendida; como máximo 1000 VMs | `vm` |
| `vm_status` | salud general | `vm` |
| `vm_tools_status` | solo VMs encendidas: 0 actualizadas, 1 versión antigua, 2 instaladas pero no en ejecución, 3 no instaladas | `vm` |
| `datastores` | datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 accesible, 0 inaccesible | `datastore` |
| `datastore_status` | salud general | `datastore` |
| `alarms` | alarmas activadas no confirmadas, por color: `red`, `yellow` | `status` |
| `alarms_acknowledged` | alarmas activadas ya confirmadas | |
| `alarm` | una serie por alarma no confirmada, valor = su código de salud, como máximo 64 | `alarm` (su nombre), `entity`, `entity_type`, `status` |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

Las [reglas integradas](../alerting/rules.md#vmware-vsphere) que se
aplican: vSphere host not responding, vSphere host health red, vSphere
host left in maintenance, vSphere datastore almost full, vSphere
datastore inaccessible, vSphere VM tools not running, vSphere red alarm,
además de Device unreachable.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca consulta vSphere. Indica primero qué va mal (hosts sin respuesta,
alarmas rojas, datastores casi llenos o inaccesibles), luego los hosts
con su carga, los datastores con su nivel de llenado, y las VMs por
estado de encendido.

## Cómo se validó {#how-it-was-validated}

Contra el simulador govmomi `vcsim` 0.56.0, en modo vCenter y en modo
ESXi: inicio de sesión y contraseña rechazada, reutilización y caducidad
de sesión, paginación por el inventario, un host sin respuesta, un host
en modo mantenimiento, una VM apagada y una plantilla. El simulador no
lanza ninguna alarma: la lectura de las alarmas activadas se validó
contra la estructura `AlarmState` documentada de la Web Services API.

## Crear un usuario de solo lectura en vSphere {#create-a-read-only-user-in-vsphere}

1. En un vCenter: Administration → Single Sign On → Users and Groups →
   Users, dominio vsphere.local → Add. Nombra el usuario como sigue y
   dale una contraseña larga y aleatoria.

    ```
    dumbmonit
    ```

2. Todavía en el vCenter: Administration → Access Control → Global
   Permissions → Add. Elige ese usuario, el rol Read-only, y marca
   Propagate to children. Read-only ve cada host, VM, datastore y alarma,
   y no puede cambiar nada.

3. En un host ESXi independiente en su lugar: en el host client, Manage →
   Security & users → Users → Add user, con el mismo nombre. Luego Host →
   Actions → Permissions → Add user: elígelo, el rol Read-only, y marca
   Propagate to all children.

4. En DumbMonit, introduce la dirección del vCenter o del host, por
   ejemplo «vcenter.lan» o «esxi1.lan», con el nombre de usuario tal como
   se creó: dumbmonit@vsphere.local en un vCenter, dumbmonit en un host.
   Cuando un vCenter gestiona los hosts, añade solo el vCenter: reporta
   cada host, VM y datastore.

5. DumbMonit solo lee, a través de la vSphere Web Services API (/sdk).
   Nunca enciende ni apaga una VM, nunca entra en modo mantenimiento y
   nunca confirma una alarma.

!!! warning
    No reutilices la cuenta de dominio de inicio de sesión único ni la
    cuenta integrada del host: el rol Read-only de arriba es todo lo que
    DumbMonit necesita. vCenter y ESXi usan un certificado autofirmado
    por defecto: activa «Accept an unverifiable certificate» a menos que
    el host de DumbMonit confíe en su autoridad certificadora.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario / contraseña | El usuario dedicado con el rol Read-only: `dumbmonit@vsphere.local` en un vCenter, `dumbmonit` en un host. |

Dirección: un nombre de host o IP (`vcenter.lan`), `host:puerto`, o una URL
completa; se acepta un `/sdk` o `/ui` final. Opciones: puerto (443),
comprobación de certificado, tiempo de espera de la petición (20 s) y
alarmas activadas (lectura).
