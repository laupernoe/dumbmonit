# Tipos de dispositivos

DumbMonit llama *dispositivo* (la API dice *objetivo*) a todo lo que vigila, incluso
cuando es una página web o un nombre de DNS. Cada tipo es un recolector en el servidor; el
servidor describe cada tipo a la interfaz (`GET /api/collectors`), por lo que el
formulario siempre coincide con lo que esta versión puede monitorear.

| Tipo | `kind` | Credencial | Uso típico |
|---|---|---|---|
| [Dispositivo SNMP](snmp.md) | `snmp` | comunidad o SNMP v3 | Conmutador, enrutador, NAS, SAI, impresora |
| [Proxmox VE](proxmox.md) | `proxmox` | token de API o nombre de usuario/contraseña | Hipervisor, clúster |
| [Servidor de copia de seguridad Proxmox](pbs.md) | `pbs` | token de API o nombre de usuario/contraseña | Servidor de copia de seguridad |
| [Gestor de centro de datos Proxmox](pdm.md) | `pdm` | token de API | Consola que federa varios clústeres PVE e instancias PBS |
| [Puerta de enlace de correo Proxmox](pmg.md) | `pmg` | nombre de usuario/contraseña | Puerta de enlace de correo filtrando spam y virus |
| [VMware vSphere](vsphere.md) | `vsphere` | nombre de usuario/contraseña | vCenter o ESXi independiente: hosts, VMs, almacenes de datos, alarmas |
| [XCP-ng / XenServer](xcpng.md) | `xcpng` | nombre de usuario/contraseña (rol XAPI de solo lectura) | Pool a través de XAPI: hosts, VMs, repositorios de almacenamiento, alta disponibilidad |
| [Nutanix AHV](nutanix.md) | `nutanix` | nombre de usuario/contraseña (Prism Viewer) | API v4 de Prism Central: clústeres, hosts, VMs AHV, contenedores de almacenamiento, alertas |
| [Synology DSM](synology.md) | `synology` | nombre de usuario/contraseña | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | clave de API y secreto | Firewall, enrutador, borde multi-WAN |
| [pfSense](pfsense.md) | `pfsense` | clave de API REST (usuario de solo lectura) | Firewall: puertas de enlace abajo o con pérdida, interfaces sin enlace, servicios detenidos |
| [FortiGate](fortigate.md) | `fortigate` | token de API REST (perfil de solo lectura, host de confianza) | Firewall: túneles IPsec, enlaces, sincronización HA, modo conservador, licencias FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | nombre de usuario/contraseña (perfil de solo lectura) | Firewall: solo enlaces de interfaz (la API XML no expone estado de túnel, HA o licencia) |
| [Servidor OpenVPN](openvpn.md) | `openvpn` | ninguna, o contraseña de gestión | Servidor OpenVPN a través de su interfaz de gestión: versión, estado, clientes conectados |
| [UniFi Network](unifi.md) | `unifi` | clave de API o nombre de usuario/contraseña | Consola UniFi o servidor autohospedado: dispositivos, WAN, clientes |
| [Home Assistant](homeassistant.md) | `homeassistant` | token de acceso de larga duración | Casa inteligente: entidades no disponibles, baterías bajas, actualizaciones, reparaciones |
| [TrueNAS](truenas.md) | `truenas` | clave de API | Servidor de almacenamiento ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | clave de API (rol Visor) | NAS: matriz, discos, comprobaciones de paridad, grupos de caché, contenedores, VMs |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | nombre de usuario/contraseña (grupo de solo lectura) | Enrutador o conmutador: versiones y firmware atrasados, CPU, memoria, sensores, errores de interfaz |
| [SAI con NUT](nut.md) | `nut` | ninguno, o nombre de usuario/contraseña | SAI en NAS, Raspberry Pi o servidor que ejecuta Network UPS Tools |
| [Hardware del servidor (Redfish)](redfish.md) | `redfish` | nombre de usuario/contraseña | Ventiladores del servidor, temperaturas, fuentes de alimentación y unidades, leído desde su BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | ninguno, nombre de usuario/contraseña o token | Base de datos de series temporales: ingesta, muestras rechazadas, espacio en disco |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | ninguno, nombre de usuario/contraseña o token | Base de datos de registros: ingesta, líneas rechazadas, fallos de descarga |
| [Grafana Loki](loki.md) | `loki` | ninguno, nombre de usuario/contraseña o token | Servidor de registros: disponibilidad, líneas rechazadas, fallos de descarga |
| [Graylog](graylog.md) | `graylog` | token de acceso o nombre de usuario/contraseña | Servidor de registros y su clúster OpenSearch: diario, búferes, rendimiento, entradas |
| [Pi-hole](pihole.md) | `pihole` | contraseña de aplicación, o ninguno | Bloqueador DNS: estado de bloqueo, participación bloqueada, antigüedad de lista de bloqueo, actualizaciones |
| [AdGuard Home](adguard.md) | `adguard` | nombre de usuario/contraseña | Filtro DNS: protección, bloqueos, listas de filtros, servidores ascendentes, actualizaciones |
| [Servidor de correo MDaemon](mdaemon.md) | `mdaemon` | ninguno, o dirección de correo/contraseña | Servidor de correo: SMTP, IMAP, POP3, correo web, versión |
| [SecurityGateway para servidores de correo](securitygateway.md) | `securitygateway` | ninguno, o clave de API | Puerta de enlace de correo: servicios, cola de entrega y otros contadores |
| [Redis / Valkey](redis.md) | `redis` | usuario ACL, solo contraseña, o ninguno | Almacén en memoria: memoria contra maxmemory, replicación, persistencia, claves |
| [MongoDB](mongodb.md) | `mongodb` | nombre de usuario/contraseña (clusterMonitor) o ninguno | Base de datos de documentos: conjunto de réplicas, retraso, conexiones, caché WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | nombre de usuario/contraseña (etiqueta de monitoreo) | Corredor de mensajes: alarmas de recursos, nodos, particiones, colas sin consumidor |
| [CrowdSec](crowdsec.md) | `crowdsec` | ninguno, o clave de API de bouncer | Motor de seguridad: decisiones, alertas, bouncers que dejaron de extraer, lectura de registro |
| [Traefik](traefik.md) | `traefik` | ninguno, o usuario basicAuth | Proxy inverso: enrutadores deshabilitados, servidores backend abajo, certificados no obtenidos o no renovados, participación 5xx |
| [Caddy](caddy.md) | `caddy` | usuario de vista de solo lectura, o ninguno | Servidor web y proxy inverso: salud ascendente, recargas fallidas, errores de manejador, participación 5xx |
| [Nginx](nginx.md) | `nginx` | ninguno, o autenticación básica | Servidor web: conexiones activas/en espera, tasas aceptadas/manejadas/solicitud, salud ascendente NGINX Plus |
| [Apache httpd](apache.md) | `apache` | ninguno, o autenticación básica | Servidor web: trabajadores ocupados/inactivos, tablero por estado, tasas de solicitud y byte |
| [Gestor de proxy Nginx](npm.md) | `npm` | correo de usuario y contraseña (solo vista) | Gestor de proxy: hosts rechazados por nginx, hosts deshabilitados, certificados servidos |
| [Expiración de dominio](domain.md) | `domain` | ninguno | Dominio registrado sobre RDAP: días antes de la expiración, retención, período de rescate, registrador |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | token de cuenta de servicio (ClusterRole de solo lectura) | Clúster: nodos no listos, pods en bucle de bloqueo o pendientes, cargas de trabajo faltando réplicas, reclamaciones de volumen atascadas |
| [Active Directory](activedirectory.md) | `activedirectory` | nombre de usuario/contraseña (usuario de dominio plano) | Dominio de Windows sobre LDAPS: controladores de dominio y roles FSMO, miembros de grupos privilegiados, política de contraseña, hallazgos de seguridad |
| [Veeam Backup & Replication](veeam.md) | `veeam` | nombre de usuario/contraseña (Veeam Backup Viewer) | Servidor de copia de seguridad: trabajos fallidos, sesiones fallidas, repositorios llenándose, licencia |
| [Tailscale](tailscale.md) | `tailscale` | cliente OAuth (devices:core:read) o token de acceso de API | Tailnet: servidores sin conexión, claves de nodo expirando, dispositivos en espera de aprobación |
| [Nextcloud](nextcloud.md) | `nextcloud` | token de monitoreo (NC-Token) | Sincronización de archivos: modo mantenimiento, actualización pendiente, actualizaciones, usuarios activos, OPcache |
| [Immich](immich.md) | `immich` | clave de API | Biblioteca de fotos: colas de trabajos, actualizaciones, disco, conteos de fotos |
| [Paperless-ngx](paperless.md) | `paperless` | token de API o nombre de usuario/contraseña | Documentos: Redis, Celery, índice, importaciones fallidas, actualizaciones |
| [Jellyfin](jellyfin.md) | `jellyfin` | clave de API | Servidor multimedia: tareas programadas fallidas, complementos, transmisiones y transcodificaciones |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, o ninguno de una red permitida | Servidor multimedia: transmisiones, transcodificaciones, bibliotecas, actualizaciones |
| [GitLab (autogestionado)](gitlab.md) | `gitlab` | token de acceso personal | Forja: atraso de cola Sidekiq, ejecutores de CI, migraciones pendientes, configuración de registro/2FA, licencia |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | token de acceso | Forja: verificación de salud, tareas programadas, ejecutores de Actions, conteos de repositorio/cuenta |
| [Servidor con agente](agent.md) | `agent` | ninguno (token de inscripción) | Linux, Windows, Raspberry Pi, [anfitrión Hyper-V](agent.md#hyper-v-hosts) |
| [Sitio web o API web](services.md#http) | `http` | ninguno, nombre de usuario/contraseña o token | Página de salud, API REST |
| [Puerto de red](services.md#tcp) | `tcp` | ninguno | SSH, SMB, base de datos |
| [Nombre de dominio](services.md#dns) | `dns` | ninguno | Tu dominio, un nombre interno |
| [Host alcanzable](services.md#ping) | `ping` | ninguno | Puerta de enlace, punto de acceso, impresora |
| [Certificado TLS](services.md#tls) | `tls` | ninguno | IMAPS, LDAPS, proxy inverso |
| [Servidor SMTP](services.md#smtp) | `smtp` | ninguno, o nombre de usuario/contraseña | Servidor de correo: saludo, STARTTLS, certificado, inicio de sesión opcional |
| [PostgreSQL](services.md#postgresql) | `postgres` | nombre de usuario/contraseña | Base de datos: conexión y una consulta que elijas |
| [MySQL o MariaDB](services.md#mysql-and-mariadb) | `mysql` | nombre de usuario/contraseña | Base de datos: conexión y una consulta que elijas |
| [Corredor MQTT](services.md#mqtt) | `mqtt` | ninguno, o nombre de usuario/contraseña | Corredor: conexión, suscripción, mensaje retenido |
| [WebSocket](services.md#websocket) | `websocket` | ninguno, nombre de usuario/contraseña o token | Punto final WebSocket: apretón de manos, intercambio de mensajes opcional |
| [Servidor de tiempo NTP](services.md#ntp) | `ntp` | ninguno | Enrutador, NAS, servidor de tiempo interno, servidor de grupo público |
| [Latido del corazón](push.md) | `push` | ninguno (URL secreta) | Trabajo cron, script de copia de seguridad, automatización que debe llamar |
| [Cambios del sitio web](webchange.md) | `webchange` | ninguno | Una página o un sitio completo: cambios de texto, páginas nuevas y eliminadas, antes/después con capturas de pantalla |
| [Salud y SEO del sitio web](website.md) | `website` | ninguno | Sitios en masa: abajo, página en blanco, redireccionamientos, noindex, robots.txt, cambios de título/descripción/canónica, plataforma (importación CSV) |
| [Dispositivo de demostración](demo.md) | `dummy` | ninguno | Explora la interfaz de usuario sin hardware |

## Cómo se lee un dispositivo

El planificador ejecuta cada dispositivo habilitado en su propio intervalo (predeterminado 60 s,
mínimo 10 s) a través del recolector para su tipo, con como máximo
`DUMBMONIT_MAX_CONCURRENT_PROBES` sondeos en vuelo y un tiempo de espera duro de
`DUMBMONIT_PROBE_TIMEOUT_SECS` por sondeo. Cada sondeo exitoso escribe sus
muestras más `dumbmonit_up = 1`; un sondeo fallido no escribe nada, y es el
silencio de la serie lo que la regla "Dispositivo inalcanzable" detecta. La página del dispositivo
muestra el fallo en el siguiente sondeo, pero la alerta espera a propósito: se
dispara aproximadamente cuatro minutos después del último sondeo exitoso (tres minutos de
silencio, luego un minuto de retención), por lo que un único sondeo perdido nunca notifica a nadie.

Los fallos se clasifican. Un dispositivo que no responde es *inalcanzable* y
genera una alerta. Una comunidad incorrecta, un certificado rechazado o una
capacidad faltante es un *error de configuración*: se muestra en la página del dispositivo, en rojo,
con la razón, y no se notifica. La distinción es importante: un dispositivo mal configurado
no debe despertar a nadie por la noche.

## Opciones

Algunos tipos leen opciones de las etiquetas del dispositivo: el puerto de API, si aceptar
un certificado autofirmado, qué nodos monitorear. El formulario muestra un campo escrito
por opción bajo **Más opciones**, con su predeterminado y texto de ayuda. Las páginas por tipo
las enumeran textualmente. Las opciones son configuración, no etiquetas: no se copian
en la serie. Las etiquetas libres (cualquier otra clave) se copian en cada serie como
etiquetas `tag_<key>`, excepto claves que nombran un secreto (coincididas por palabra: `password`, `token`,
`key`, `cert`, `secret`, `pass`, `private`, `pem`…, entonces `primary_key` se omite
pero `keyboard` no) y valores más largos que 256 caracteres. Nunca pongas un secreto en
una etiqueta de todos modos: usa el campo de credencial, que está encriptado.

## Dispositivos padres

Cualquier dispositivo puede declarar otro como su padre. Cuando el padre es
inalcanzable, las alertas de sus descendientes se marcan *Suprimido por padre*
en lugar de ser enviadas: un conmutador que cae produce una notificación, no
treinta. En la página de Dispositivos, los hijos se apilan bajo su padre y se oscurecen cuando
es inalcanzable.
