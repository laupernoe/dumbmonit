# Nutanix AHV {#nutanix-ahv}

Clústeres Nutanix, leídos a través de la API REST v4 de Prism Central. Un
Prism Central cubre todos los clústeres registrados en él.

Cada minuto, DumbMonit lee, con autenticación HTTP básica en el puerto 9440:

| Qué | Ruta |
|---|---|
| Clústeres | `/api/clustermgmt/v4.0/config/clusters` |
| Hosts | `/api/clustermgmt/v4.0/config/hosts` |
| VMs AHV | `/api/vmm/v4.0/ahv/config/vms` |
| Contenedores de almacenamiento | `/api/clustermgmt/v4.0/config/storage-containers` |
| Nivel de llenado de cada contenedor | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Alertas sin resolver | `/api/monitoring/v4.0/serviceability/alerts` |

Las listas se leen de 100 elementos por página, hasta 1.000 elementos cada
una. El nivel de llenado y las alertas son opcionales: si el rol de la cuenta
los rechaza, el resto se recopila igualmente. Nutanix declaró obsoletas las
APIs v1, v2.0 y v3; solo la v4 sigue evolucionando, y requiere Prism Central
pc.2024.3 o posterior.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de las especificaciones de la API
    v4 de Nutanix (rutas y campos tal como los definen los clientes generados
    oficiales) y se probó contra respuestas grabadas, no contra un Prism
    Central en ejecución. Cuéntanos qué falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_nutanix_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `clusters` | | Clústeres, sin el propio Prism Central. |
| `cluster_info` | `cluster`, `version` | Valor 1: la versión de AOS de cada clúster. |
| `cluster_available` | `cluster` | 1 cuando Prism Central alcanza el clúster. |
| `hosts` | | Hosts de todos los clústeres. |
| `host_info` | `host`, `cluster`, `hypervisor` | Valor 1: el hipervisor que ejecuta cada host. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Capacidad. |
| `host_degraded` | `host`, `cluster` | 1 cuando Nutanix marca el host como degradado. |
| `host_maintenance` | `host`, `cluster` | 1 cuando el host entra en modo de mantenimiento o está en él. |
| `vms`, `vms_running` | | VMs AHV, y cuántas están encendidas. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Valor 1: `on`, `off`, `paused` o `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1 cuando la VM está encendida. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Sockets × núcleos × hilos, y memoria configurada. |
| `storage_containers` | | Contenedores de almacenamiento, sin los internos de Nutanix. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Nivel de llenado, según las últimas estadísticas de los últimos treinta minutos. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Alertas sin resolver, todas y por severidad. |

Las [reglas integradas](../alerting/rules.md#nutanix) que se aplican:

- **Nutanix host degraded**: un host está marcado como degradado durante cinco
  minutos (Warning).
- **Nutanix storage container almost full**: un contenedor está lleno al 90 %
  durante quince minutos (Advisory).
- **Nutanix critical alerts**: Prism Central reporta al menos una alerta
  crítica sin resolver durante cinco minutos (Warning).

## Crear un usuario de solo lectura en Prism Central {#create-a-read-only-user-in-prism-central}

1. En Prism Central, abre Admin Center > IAM > Identities y añade un usuario local con una contraseña larga y aleatoria, con este nombre.

    ```
    dumbmonit
    ```

2. En Admin Center > IAM > Authorization Policies, crea una política con el rol integrado Prism Viewer, para todas las entidades, y asígnala a ese usuario. Prism Viewer lee clústeres, hosts, VMs, contenedores de almacenamiento y alertas, y no cambia nada.

3. En DumbMonit, introduce la dirección de Prism Central, por ejemplo «prism-central.lan», con ese nombre de usuario y contraseña. Un Prism Central reporta todos los clústeres registrados en él.

4. DumbMonit solo lee la API v4 (clustermgmt, vmm y monitoring, versión 4.0): nunca enciende ni apaga una VM, nunca resuelve una alerta y nunca modifica un contenedor de almacenamiento.

!!! warning
    La API v4 la sirve Prism Central pc.2024.3 o posterior; un clúster gestionado solo por Prism Element, o por un Prism Central más antiguo, no es compatible. Prism Central incluye un certificado autofirmado: marca Accept an unverifiable certificate a menos que el host de DumbMonit confíe en su autoridad de certificación.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario y contraseña | El usuario local `dumbmonit` con el rol Prism Viewer. |

Dirección: el nombre de host o la IP de Prism Central, `host:port` o una URL.
Opciones:

- **Port** (9440).
- **Accept an unverifiable certificate**: el propio certificado de Prism
  Central es autofirmado.
- **Timeout por solicitud** (15 s).

## Solución de problemas {#troubleshooting}

«Nutanix Prism Central refused the credentials … (401)»: nombre de usuario o
contraseña incorrectos.

«Nutanix Prism Central has no /api/clustermgmt/v4.0/…»: la dirección es la de
un Prism Element, o de un Prism Central anterior a pc.2024.3.

Sin `storage_container_used_percent`: el rol no permite leer las estadísticas;
los contenedores se cuentan igualmente, con su capacidad configurada.
