# Tipos de dispositivo {#device-types}

DumbMonit llama *dispositivo* a todo lo que vigila (la API dice
*target*), incluso cuando es una página web o un nombre DNS. Cada tipo
es un recolector en el servidor; el servidor describe cada tipo a la
interfaz (`GET /api/collectors`), así que el formulario siempre
coincide con lo que esta versión puede monitorizar.

| Tipo | `kind` | Credencial | Uso típico |
|---|---|---|---|
| [Dispositivo SNMP](snmp.md) | `snmp` | comunidad o SNMP v3 | Switch, router, NAS, SAI, impresora |
| [Proxmox VE](proxmox.md) | `proxmox` | token de API o usuario/contraseña | Hipervisor, clúster |
| [Proxmox Backup Server](pbs.md) | `pbs` | token de API o usuario/contraseña | Servidor de copias de seguridad |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | token de API | Consola que federa varios clústeres PVE e instancias PBS |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | usuario/contraseña | Pasarela de correo que filtra spam y virus |
| [VMware vSphere](vsphere.md) | `vsphere` | usuario/contraseña | vCenter o ESXi independiente: hosts, VMs, datastores, alarmas |
| [Synology DSM](synology.md) | `synology` | usuario/contraseña | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | clave de API y secreto | Cortafuegos, router, borde multi-WAN |
| [pfSense](pfsense.md) | `pfsense` | clave de API REST (usuario de solo lectura) | Cortafuegos: gateways caídos o con pérdidas, interfaces sin enlace, servicios detenidos |
| [FortiGate](fortigate.md) | `fortigate` | token de API REST (perfil de solo lectura, host de confianza) | Cortafuegos: túneles IPsec, enlaces, sincronización HA, modo conserve, licencias FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | usuario/contraseña (perfil de solo lectura) | Cortafuegos: solo enlaces de interfaz (la API XML no expone túnel, HA ni estado de licencia) |
| [UniFi Network](unifi.md) | `unifi` | clave de API o usuario/contraseña | Consola UniFi o servidor autoalojado: dispositivos, WAN, clientes |
| [Home Assistant](homeassistant.md) | `homeassistant` | token de acceso de larga duración | Hogar inteligente: entidades no disponibles, baterías bajas, actualizaciones, reparaciones |
| [TrueNAS](truenas.md) | `truenas` | clave de API | Servidor de almacenamiento ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | clave de API (rol Viewer) | NAS: array, discos, comprobaciones de paridad, pools de caché, contenedores, VMs |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | usuario/contraseña (grupo de solo lectura) | Router o switch: versiones y firmware atrasados, CPU, memoria, sensores, errores de interfaz |
| [SAI con NUT](nut.md) | `nut` | ninguna, o usuario/contraseña | SAI en un NAS, Raspberry Pi o servidor que ejecuta Network UPS Tools |
| [Hardware de servidor (Redfish)](redfish.md) | `redfish` | usuario/contraseña | Ventiladores, temperaturas, fuentes de alimentación y discos del servidor, leídos desde su BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | ninguna, usuario/contraseña o token | Base de datos de series temporales: ingestión, muestras rechazadas, margen de disco |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | ninguna, usuario/contraseña o token | Base de datos de logs: ingestión, líneas rechazadas, margen de disco |
| [Grafana Loki](loki.md) | `loki` | ninguna, usuario/contraseña o token | Servidor de logs: disponibilidad, líneas rechazadas, fallos de volcado |
| [Graylog](graylog.md) | `graylog` | token de acceso o usuario/contraseña | Servidor de logs y su clúster OpenSearch: journal, buffers, caudal, inputs |
| [Pi-hole](pihole.md) | `pihole` | contraseña de app, o ninguna | Bloqueador de anuncios DNS: estado de bloqueo, proporción bloqueada, antigüedad de lista, actualizaciones |
| [AdGuard Home](adguard.md) | `adguard` | usuario/contraseña | Filtro DNS: protección, bloqueos, listas de filtro, servidores upstream, actualizaciones |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | ninguna, o correo/contraseña | Servidor de correo: SMTP, IMAP, POP3, webmail, versión |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | ninguna, o clave de API | Pasarela de correo: servicios, cola de entrega y otros contadores |
| [Redis / Valkey](redis.md) | `redis` | usuario ACL, solo contraseña, o ninguna | Almacén en memoria: memoria frente a maxmemory, replicación, persistencia, claves |
| [MongoDB](mongodb.md) | `mongodb` | usuario/contraseña (clusterMonitor) o ninguna | Base de datos de documentos: replica set, retraso, conexiones, caché WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | usuario/contraseña (etiqueta monitoring) | Broker de mensajes: alarmas de recursos, nodos, particiones, colas sin consumidor |
| [CrowdSec](crowdsec.md) | `crowdsec` | ninguna, o clave de API de bouncer | Motor de seguridad: decisiones, alertas, bouncers que dejaron de actualizar, lectura de logs |
| [Traefik](traefik.md) | `traefik` | ninguna, o usuario basicAuth | Proxy inverso: routers desactivados, servidores backend caídos, certificados no obtenidos o no renovados, proporción de 5xx |
| [Caddy](caddy.md) | `caddy` | usuario de vista de solo lectura, o ninguna | Servidor web y proxy inverso: salud de upstream, recargas fallidas, errores de handler, proporción de 5xx |
| [Nginx](nginx.md) | `nginx` | ninguna, o basic auth | Servidor web: conexiones activas/en espera, tasas de accept/handled/request, salud de upstream de NGINX Plus |
| [Apache httpd](apache.md) | `apache` | ninguna, o basic auth | Servidor web: workers ocupados/libres, scoreboard por estado, tasas de peticiones y bytes |
| [Nginx Proxy Manager](npm.md) | `npm` | correo y contraseña de usuario (solo vista) | Gestor de proxy: hosts que nginx rechazó, hosts desactivados, certificados servidos |
| [Caducidad de dominio](domain.md) | `domain` | ninguna | Dominio registrado vía RDAP: días antes de caducar, hold, período de redención, registrador |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | token de cuenta de servicio (ClusterRole de solo lectura) | Clúster: nodos no listos, pods en crash looping o pending, workloads con réplicas faltantes, reclamaciones de volumen atascadas |
| [Active Directory](activedirectory.md) | `activedirectory` | usuario/contraseña (usuario de dominio corriente) | Dominio Windows vía LDAPS: controladores de dominio y roles FSMO, miembros de grupo privilegiado, política de contraseñas, hallazgos de seguridad |
| [Veeam Backup & Replication](veeam.md) | `veeam` | usuario/contraseña (Veeam Backup Viewer) | Servidor de copias de seguridad: jobs fallidos, sesiones fallidas, repositorios llenándose, licencia |
| [Tailscale](tailscale.md) | `tailscale` | cliente OAuth (devices:core:read) o token de acceso de API | Tailnet: servidores fuera de línea, claves de nodo caducando, dispositivos esperando aprobación |
| [Nextcloud](nextcloud.md) | `nextcloud` | token de monitorización (NC-Token) | Sincronización de archivos: modo mantenimiento, actualización pendiente, actualizaciones, usuarios activos, OPcache |
| [Immich](immich.md) | `immich` | clave de API | Biblioteca de fotos: colas de trabajos, actualizaciones, disco, recuentos de fotos |
| [Paperless-ngx](paperless.md) | `paperless` | token de API o usuario/contraseña | Documentos: Redis, Celery, índice, importaciones fallidas, actualizaciones |
| [Jellyfin](jellyfin.md) | `jellyfin` | clave de API | Servidor multimedia: tareas programadas fallidas, plugins, streams y transcodificaciones |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, o ninguna desde una red permitida | Servidor multimedia: streams, transcodificaciones, bibliotecas, actualizaciones |
| [GitLab (autoalojado)](gitlab.md) | `gitlab` | token de acceso personal | Forge: backlog de Sidekiq, runners de CI, migraciones pendientes, ajustes de registro/2FA, licencia |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | token de acceso | Forge: comprobación de salud, tareas programadas, runners de Actions, recuentos de repositorio/cuenta |
| [Servidor con agente](agent.md) | `agent` | ninguna (token de inscripción) | Linux, Windows, Raspberry Pi, [host Hyper-V](agent.md#hyper-v-hosts) |
| [Sitio web o API web](services.md#http) | `http` | ninguna, usuario/contraseña o token | Página de salud, API REST |
| [Puerto de red](services.md#tcp) | `tcp` | ninguna | SSH, SMB, base de datos |
| [Nombre de dominio](services.md#dns) | `dns` | ninguna | Tu dominio, un nombre interno |
| [Host accesible](services.md#ping) | `ping` | ninguna | Gateway, punto de acceso, impresora |
| [Certificado TLS](services.md#tls) | `tls` | ninguna | IMAPS, LDAPS, proxy inverso |
| [Servidor de hora NTP](services.md#ntp) | `ntp` | ninguna | Router, NAS, servidor de hora interno, servidor de pool público |
| [Heartbeat](push.md) | `push` | ninguna (URL secreta) | Tarea cron, script de copia de seguridad, automatización que debe avisar |
| [Cambios en sitios web](webchange.md) | `webchange` | ninguna | Una página o un sitio entero: cambios de texto, páginas nuevas y eliminadas, antes/después con capturas |
| [Dispositivo de demostración](demo.md) | `dummy` | ninguna | Explora la UI sin hardware |

## Cómo se lee un dispositivo {#how-a-device-is-read}

El scheduler ejecuta cada dispositivo activado en su propio intervalo
(60 s por defecto, mínimo 10 s) a través del recolector de su tipo, con
como máximo `DUMBMONIT_MAX_CONCURRENT_PROBES` sondas en vuelo y un
tiempo de espera máximo de `DUMBMONIT_PROBE_TIMEOUT_SECS` por sonda.
Cada sonda exitosa escribe sus muestras más `dumbmonit_up = 1`; una sonda
fallida no escribe nada, y es el silencio de la serie lo que detecta la
regla «Device unreachable».

Los fallos se clasifican. Un dispositivo que no responde es
*inaccesible* y genera una alerta. Una comunidad incorrecta, un
certificado rechazado o una capacidad faltante es un *error de
configuración*: se muestra en la página del dispositivo, en rojo, con
el motivo, y no se notifica. La distinción importa: un dispositivo mal
configurado no debería despertar a nadie por la noche.

## Opciones {#options}

Algunos tipos leen opciones de las etiquetas del dispositivo: el puerto
de API, si aceptar un certificado autofirmado, qué nodos monitorizar.
El formulario muestra un campo tipado por opción bajo **More options**,
con su valor por defecto y texto de ayuda. Las páginas por tipo las
listan textualmente. Las opciones se copian en cada serie como
etiquetas `tag_<key>`, así que nunca pongas un secreto en una: usa el
campo de credencial, que está cifrado.

## Dispositivos padre {#parent-devices}

Cualquier dispositivo puede declarar otro como su padre. Cuando el
padre es inaccesible, las alertas de sus descendientes se marcan
*Suppressed by parent* en lugar de enviarse: un switch que se cae
produce una notificación, no treinta. En la página Devices, los hijos
se apilan bajo su padre y se atenúan cuando es inaccesible.
