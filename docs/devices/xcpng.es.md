# XCP-ng / XenServer {#xcp-ng-xenserver}

Un pool XCP-ng o XenServer (Citrix Hypervisor), leído a través de XAPI, la API
de gestión que cada host ya ejecuta. No hace falta Xen Orchestra.

Cada minuto, DumbMonit envía llamadas JSON-RPC a
`https://<coordinador>/jsonrpc`: un `session.login_with_password` (la sesión se
conserva de una consulta a la siguiente y se reabre una vez si el pool la
invalida) y un `get_all_records` por clase: `host`, `host_metrics`, `VM`,
`VM_metrics`, `SR` y `pool`. Nunca se modifica nada: la cuenta tiene el rol
integrado `read-only`, que XAPI aplica en cada llamada.

Añade el pool una sola vez, con la dirección de su coordinador: este reporta
todos los hosts, VMs y repositorios de almacenamiento. Si la dirección que
introdujiste es la de un miembro del pool, este responde `HOST_IS_SLAVE` con la
dirección del coordinador; DumbMonit la sigue una vez, con el mismo protocolo y
puerto, de modo que un cambio de coordinador no te obliga a editar el
dispositivo.

!!! warning "Validado solo contra la documentación del fabricante — aún no probado en un sistema real"

    Esta integración se construyó a partir de la documentación de XAPI (el
    protocolo JSON-RPC, las clases `host`, `VM`, `SR` y `pool`) y se probó
    contra respuestas grabadas, no contra un pool en ejecución. Cuéntanos qué
    falla.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_xcpng_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `hosts`, `hosts_enabled` | | Hosts del pool, y cuántos aceptan nuevas VMs. |
| `host_info` | `host`, `version` | Valor 1: la versión del producto de cada host. |
| `host_enabled` | `host` | 0 cuando el host está deshabilitado (mantenimiento o evacuado). |
| `host_live` | `host` | 1 cuando el latido del host llega al coordinador, 0 cuando está offline. |
| `host_cpu_cores`, `host_memory_total_bytes`, `host_memory_used_bytes` | `host` | Capacidad y memoria en uso. |
| `vms`, `vms_running` | | VMs, sin plantillas, instantáneas ni dominios de control. |
| `vm_power_state_info` | `vm`, `state` | Valor 1: `running`, `halted`, `paused` o `suspended`. |
| `vm_running` | `vm` | 1 cuando la VM está en ejecución. |
| `vm_vcpus`, `vm_memory_bytes` | `vm` | vCPUs al arrancar; memoria en uso para una VM en ejecución, su máximo configurado en caso contrario. |
| `vm_ha_protected_down` | `vm` | Solo para VMs con prioridad de reinicio, en un pool con alta disponibilidad activada: 1 cuando la VM está detenida. |
| `srs` | | Repositorios de almacenamiento. |
| `sr_size_bytes`, `sr_used_bytes`, `sr_used_percent` | `sr` | Nivel de llenado (las bibliotecas ISO se cuentan, no se detallan). |
| `sr_shared` | `sr` | 1 para un repositorio compartido (NFS, iSCSI…). |
| `pool_info` | `pool`, `master` | Valor 1: el pool y su coordinador actual. |
| `pool_ha_enabled` | `pool` | 1 cuando la alta disponibilidad está activada. |
| `pool_ha_overcommitted` | `pool` | Con HA activada: 1 cuando el pool ya no puede reiniciar sus VMs protegidas tras los fallos de host que está configurado para tolerar. |
| `pool_ha_host_failures_to_tolerate` | `pool` | El número de fallos de host que la HA está configurada para soportar. |

XenAPI no tiene un estado de energía «caído». Una VM protegida que aparece
detenida es lo más cercano que da la API a una VM que se paró contra tu
voluntad; si apagas una VM protegida a propósito, quítale antes su prioridad
de reinicio.

Las [reglas integradas](../alerting/rules.md#xcp-ng-xenserver) que se aplican:

- **XCP-ng host offline**: el latido de un host no ha llegado al coordinador
  durante cinco minutos (Warning).
- **XCP-ng SR almost full**: un repositorio de almacenamiento está lleno al 90 %
  durante quince minutos (Advisory).
- **XCP-ng HA-protected VM down**: una VM protegida por la alta disponibilidad
  está detenida durante dos minutos (Warning).
- **XCP-ng pool HA overcommitted**: el pool está sobrecomprometido durante
  cinco minutos (Advisory).

## Darle a DumbMonit una cuenta XAPI de solo lectura {#give-dumbmonit-a-read-only-xapi-account}

1. XAPI solo acepta cuentas distintas de su superusuario integrado cuando el pool usa autenticación externa. Si el pool ya está unido a Active Directory, crea ahí una cuenta dumbmonit y pasa al paso 3.

2. Si no, crea una cuenta local con una contraseña larga y aleatoria en cada host del pool y luego activa una vez la autenticación PAM, desde el coordinador del pool. Mientras no se le dé un rol, la cuenta no puede llamar a nada.

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. En el coordinador del pool, registra la cuenta como sujeto y luego dale a ese sujeto el rol integrado read-only, con el UUID que imprime el primer comando.

    ```
    xe subject-add subject-name=dumbmonit
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

4. En DumbMonit, introduce la dirección del coordinador del pool, por ejemplo «xcp01.lan», con ese nombre de usuario y contraseña. Añade el pool una sola vez: el coordinador reporta todos los hosts, VMs y repositorios de almacenamiento. Un miembro del pool responde con la dirección del coordinador, que DumbMonit sigue después.

5. DumbMonit solo lee, a través de la interfaz JSON-RPC de XAPI (/jsonrpc): nunca arranca, detiene ni migra una VM y nunca modifica un repositorio de almacenamiento ni la alta disponibilidad del pool.

!!! warning
    No le des nunca a DumbMonit la contraseña de superusuario de los hosts: con el rol read-only, una contraseña filtrada no puede cambiar nada. XCP-ng y XenServer incluyen un certificado autofirmado: marca Accept an unverifiable certificate a menos que el host de DumbMonit confíe en el certificado que presenta el pool.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario y contraseña | La cuenta `dumbmonit`, registrada como sujeto con el rol `read-only`. |

Dirección: el nombre de host o la IP del coordinador del pool, `host:port` o
una URL. Opciones:

- **Port** (443).
- **Accept an unverifiable certificate**: el propio certificado de los hosts es
  autofirmado.
- **Timeout por solicitud** (15 s).

## Solución de problemas {#troubleshooting}

«XCP-ng refused the user name or password»: contraseña incorrecta, o la cuenta
no está registrada como sujeto (`xe subject-list`).

«XCP-ng accepted the account but refused the call»: el sujeto no tiene rol, o
tiene un rol sin acceso de lectura; dale `read-only`.

«This host is a pool member, not the coordinator»: DumbMonit no pudo alcanzar
el coordinador que indicó el miembro; introduce tú mismo la dirección del
coordinador.
