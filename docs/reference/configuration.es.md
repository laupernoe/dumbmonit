# Configuración {#configuration}

Todo pasa por variables de entorno; ninguna es obligatoria. No hay ningún
archivo de configuración que montar.

## Servidor {#server}

| Variable | Por defecto | Rol |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Dirección de escucha de la API y la interfaz web, dentro del contenedor. |
| `DUMBMONIT_DATA_DIR` | `/data` | Directorio persistente: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (VictoriaMetrics integrada) y `webchange/` (capturas de cambios de sitio web). |
| `DUMBMONIT_VM_URL` | *(sin definir)* | URL base de una VictoriaMetrics externa (`http://host:8428`). Si se define, la integrada no se inicia; consulta [abajo](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(generado)* | Secreto de la instancia. Si está vacío, se lee de `<directorio de datos>/secret.key`, generado en el primer arranque. Cifra las credenciales de dispositivo, los secretos de canal y los tokens de agente con AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Máximo de sondas simultáneas, todos los recolectores combinados. |
| `DUMBMONIT_WEBSITE_MAX_CONCURRENT` | `10` | Máximo de comprobaciones simultáneas de [salud de sitios web](../devices/website.md#pacing-hundreds-of-sites), de 1 a 256, dentro del límite anterior. |
| `DUMBMONIT_WEBSITE_MAX_PER_HOST` | `1` | Máximo de comprobaciones de sitios web simultáneas hacia un mismo dominio (`www.example.com` y `shop.example.com` cuentan como uno), de 1 a 16. |
| `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` | `2` | Segundos mínimos entre dos comprobaciones de sitios web hacia un mismo dominio, de 0 a 3600. La opción `host_spacing_seconds` de un sitio la anula tras la comprobación de ese sitio. |
| `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` | `360` | Período por defecto, en minutos, de la comparación de metadatos del sitio web (título, descripción, canonical, dirección final, `robots.txt`), de 0 a 10080; 0 compara en cada comprobación. El estado up/down se sigue comprobando en cada intervalo de sondeo. La opción `change_check_minutes` de un sitio la anula. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Tiempo de espera máximo de una sonda. Los monitores de servicio tienen su propia opción `timeout_seconds`, más corta. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Período de las escrituras agrupadas hacia VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Número de muestras en búfer que dispara una escritura antes de que transcurra el período. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Hilos del runtime asíncrono. El servidor pasa su tiempo esperando a la red: cuatro cubren un homelab, elévalo solo para cientos de dispositivos. |
| `DUMBMONIT_DB_POOL` | `4` | Máximo de conexiones SQLite abiertas. Cada una cuesta un hilo y una caché de páginas de 2 MB; las conexiones inactivas se cierran tras cinco minutos. |
| `DUMBMONIT_LOG` | `info` | Filtro de registro, sintaxis `tracing`: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Directorio de los binarios de agente servidos bajo `/download/`. Vacío o ausente: el comando de instalación falla con `404`, el servidor sigue funcionando. |
| `DUMBMONIT_RESET_PASSWORD` | *(desactivado)* | `1`, `true`, `yes` u `on`: borra la contraseña y cada sesión al arrancar. La interfaz entonces vuelve a mostrar `/setup`. Retírala después. |
| `DUMBMONIT_COOKIE_SECURE` | *(desactivado)* | `1` para definir siempre el atributo `Secure` en la cookie de sesión. Solo detrás de HTTPS: por HTTP plano el navegador nunca devolvería la cookie. Sin esto, el atributo se añade igualmente siempre que el navegador esté en una página `https://` (su `Origin` lo indica, o un proxy de confianza envía `X-Forwarded-Proto: https`), así que el HTTP plano en la LAN sigue funcionando. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(vacío)* | Proxies inversos cuyos `X-Forwarded-For`, `X-Forwarded-Proto` y `X-Forwarded-Host` se creen, como direcciones o rangos CIDR separados por comas (`10.0.0.5, 172.16.0.0/12`). El limitador de tasa de inicio de sesión y el registro de seguridad entonces ven la dirección real del cliente en lugar de la del proxy; sin esto, cada visitante detrás del proxy comparte un mismo cubo. Los enlaces de las páginas de estado (feed RSS, correos de suscripción) y la dirección de retorno del inicio de sesión único también usan el host reenviado solo desde estos proxies. Las cabeceras se ignoran desde cualquier otra dirección, así que un cliente no puede elegir su propio cubo ni la dirección de un enlace. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(desactivado)* | `true` para crear una cuenta para una identidad desconocida en su primer inicio de sesión único. Desactivado, solo las cuentas existentes (ya vinculadas, o emparejadas por un correo verificado por el proveedor) pueden iniciar sesión. Como las demás variables `DUMBMONIT_OIDC_*`, solo se lee mientras no haya ajustes de inicio de sesión único guardados en la interfaz. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(desactivado)* | `1` para aceptar un emisor OpenID Connect en `http://` plano. Desactivado, la pantalla de ajustes rechaza cualquier cosa que no sea `https://`: un emisor en claro entrega el código de autorización y el secreto de cliente a quien escuche en la red. Solo para un proveedor de prueba en el loopback. |
| `DUMBMONIT_SETUP_CODE` | *(generado)* | Código que pide `/setup` mientras no exista ningún administrador. Sin definir, se imprime uno aleatorio en el log del servidor en cada arranque; defínelo para un despliegue automatizado. Consulta [Instalar con Docker](../install/docker.md). |
| `DUMBMONIT_PUBLIC_URL` | *(sin definir)* | Dirección con la que los navegadores llegan a DumbMonit (`https://monit.example.org`). Se usa para los enlaces de las notificaciones y de los correos de páginas de estado, el URI de redirección del inicio de sesión único y el contacto dado a los servicios push (solo HTTPS). Un valor guardado en la interfaz (política de notificación, ajustes SSO) prevalece. |
| `DUMBMONIT_API_CORS_ORIGINS` | *(cualquier origen)* | Orígenes autorizados a llamar a la API entre orígenes con un token bearer, y a acceder al endpoint MCP: una lista separada por comas, u `off` para ninguno. Consulta [API HTTP](api.md#cross-origin-requests-cors). |
| `DUMBMONIT_OIDC_ISSUER` | *(sin definir)* | URL del emisor del proveedor OpenID Connect; el descubrimiento se lee de `{issuer}/.well-known/openid-configuration`. Debe ser `https://` salvo que se defina `DUMBMONIT_OIDC_ALLOW_HTTP`. |
| `DUMBMONIT_OIDC_CLIENT_ID` | *(sin definir)* | Id de cliente registrado en el proveedor. |
| `DUMBMONIT_OIDC_CLIENT_SECRET` | *(sin definir)* | Secreto de cliente. |
| `DUMBMONIT_OIDC_PROVIDER_NAME` | `SSO` | Etiqueta del botón «Continue with …». |
| `DUMBMONIT_OIDC_SCOPES` | `openid profile email` | Ámbitos solicitados, separados por espacios. |
| `DUMBMONIT_OIDC_GROUPS_CLAIM` | `groups` | Claim del token que lista los grupos del usuario. |
| `DUMBMONIT_OIDC_ADMIN_GROUPS` | *(vacío)* | Grupos separados por comas cuyos miembros pasan a ser administradores. |
| `DUMBMONIT_OIDC_OPERATOR_GROUPS` | *(vacío)* | Grupos separados por comas cuyos otros miembros pasan a ser operadores. Consulta [Inicio de sesión único](../using/settings.md#single-sign-on). |
| `DUMBMONIT_METRICS_PUBLIC` | *(desactivado)* | `1` para servir `/metrics`, `/federate` y `/prometheus/api/v1/…` sin un token de API. Cualquiera que pueda alcanzar el puerto lee entonces cada medición de cada dispositivo y el estado de la instancia; defínelo solo donde algo más ya restrinja el acceso al puerto. Consulta [Métricas](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Período de evaluación de alertas. Los valores por debajo de 10 se elevan a 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Retención del historial de alertas, en días. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Copias de seguridad locales programadas de la base de datos. `0`, `false`, `no` u `off` para dejar de escribirlas. Consulta [Copia de seguridad y restauración](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` detiene la comprobación diaria de la última versión en GitHub (instancias aisladas). Consulta [Actualización](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<directorio de datos>/backups` | Dónde se escriben las copias de seguridad programadas. Apúntalo a un segundo volumen para sobrevivir a la pérdida del primero. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Horas entre dos copias de seguridad, de 1 a 8760. La primera ocurre un intervalo después del arranque, no al arrancar. |
| `DUMBMONIT_DEMO` | *(desactivado)* | `1` para una demo pública de solo lectura con un patrimonio ficticio fijo, inicio de sesión `demo` / `demo`. **Recrea la base de datos en cada inicio**: nunca sobre un volumen de datos real. Consulta [Ejecutar una instancia de demostración](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Cuántas copias de seguridad se conservan, de 1 a 365. Las más antiguas se eliminan con su `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(sin definir)* | Chromium sin interfaz usado para las capturas de [cambios de sitio web](../devices/webchange.md#screenshots), como `http://host:9222` (su puerto DevTools). El archivo Compose incluye uno bajo el perfil `screenshots`: `DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d`. Sin definir: solo comparación de texto. |

La retención de referencia (60 días) y el período de aprendizaje de 14
días no son configurables.

### Antiguos nombres `EZYMONIT_*` {#former-ezymonit-names}

DumbMonit se llamó EzyMonit hasta septiembre de 2026. Cada variable
`DUMBMONIT_*` también se lee bajo su antiguo nombre `EZYMONIT_*`, como
respaldo, con un aviso al arrancar por cada variable que todavía se
defina así (`EZYMONIT_X is deprecated, use DUMBMONIT_X`). El antiguo
archivo de base de datos `/data/ezymonit.db` se renombra a
`dumbmonit.db` al arrancar, y los tokens de agente `ezym_…` siguen
siendo válidos (los nuevos son `dmon_…`). La cookie de sesión es ahora
`dumbmonit_session`: el primer arranque tras el renombrado cierra la
sesión de todos una vez.

### VictoriaMetrics integrada {#embedded-victoriametrics}

La imagen incluye el binario de VictoriaMetrics. A menos que se defina
`DUMBMONIT_VM_URL`, el servidor lo inicia como proceso hijo al arrancar,
reenvía sus líneas de registro al registro del servidor, lo reinicia con
un backoff si muere, y lo detiene al apagarse (`SIGTERM`, 20 s de
gracia, y luego kill; el `stop_grace_period` del archivo Compose deja
espacio para eso).

| Variable | Por defecto | Rol |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Ruta del binario de VictoriaMetrics a iniciar. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Dirección de escucha, dentro del contenedor. Loopback por defecto: solo el servidor habla con ella. La capa de desarrollo define `0.0.0.0:8428` y publica el puerto. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retención, en la sintaxis de `-retentionPeriod`: un número de meses, o una duración como `30d` o `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Presupuesto para las cachés de VictoriaMetrics (`-memory.allowedBytes`). Sin un presupuesto, VM las dimensiona para el 60 % de la RAM del host y su memoria residente crece durante días. Unas pocas decenas de dispositivos entran en 256 MB; elévalo para cientos de hosts. |

Sus datos viven en `<directorio de datos>/vm`, en el mismo volumen que la
base de datos. Tres flags están fijados por el servidor y no son
configurables: `-search.maxUniqueTimeseries=100000` (una consulta
desbocada no puede agotar el presupuesto), `-search.maxConcurrentRequests=4`
(una UI más el motor de alertas) y `-dedup.minScrapeInterval=10s` (los
puntos duplicados a menos de diez segundos se colapsan; nada se pierde a
la cadencia de 30 s de los recolectores).

Con `DUMBMONIT_VM_URL` definida, nada de esto aplica: el servidor usa la
instancia que nombras y la integrada nunca se inicia. `GET /api/health`
indica en qué caso estás (`victoria.embedded`).

## Variables a nivel de Compose {#compose-level-variables}

Estas las lee `docker-compose.yml`, no el servidor:

| Variable | Por defecto | Rol |
|---|---|---|
| `DUMBMONIT_PORT` | `8080` | Puerto del host publicado para la interfaz. |
| `DUMBMONIT_VM_RETENTION`, `DUMBMONIT_VM_MEMORY` | `12`, `256MB` | Reenviadas al servidor; consulta arriba. |
| `DUMBMONIT_VM_PORT` | `8428` | Puerto del host para la VictoriaMetrics integrada, solo capa de desarrollo. |

## Puertos {#ports}

| Puerto | Contenedor | Propósito |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Interfaz web, API, instaladores y descargas de agente, ingesta de agente. El único puerto a publicar. |
| `8428/tcp` | `dumbmonit` | API HTTP de la VictoriaMetrics integrada, en el loopback del contenedor. Publicado solo por la capa de desarrollo. |

Saliente: SNMP (UDP 161 por defecto) hacia los dispositivos, HTTPS hacia
Proxmox, PBS, Synology y servicios de notificación, además de lo que
apunten tus monitores de servicio. El ping ICMP necesita el sysctl
`net.ipv4.ping_group_range` definido en el archivo Compose (sin
capacidad).

## Volúmenes y archivos {#volumes-and-files}

Un volumen, `dumbmonit-data`, montado en `/data`. Su nombre está fijo en
el archivo Compose, sea como se llame el proyecto.

| Ruta (en el contenedor) | Volumen | Contenido |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Dispositivos, credenciales (cifradas), reglas, estado e historial de alertas, referencias, silencios, canales, tokens de agente, hash de contraseña, sesiones. |
| `/data/secret.key` | `dumbmonit-data` | El secreto de la instancia. **Haz una copia de seguridad de él.** Sin él, las credenciales cifradas son irrecuperables y el servidor se niega a arrancar. |
| `/data/backups/` | `dumbmonit-data` | Copias de seguridad locales programadas: `dumbmonit-<timestamp>.db` (una copia SQLite en caliente) y el `.key` correspondiente. Diarias por defecto, siete conservadas. |
| `/data/vm/` | `dumbmonit-data` | Series temporales de la VictoriaMetrics integrada, 12 meses por defecto. Vacío cuando se define `DUMBMONIT_VM_URL`. |
| `/agents/` | imagen | Binarios de agente. Monta otro directorio y define `DUMBMONIT_AGENT_DIR` para distribuir los tuyos propios. |
| `/etc/ssl/certs/ca-certificates.crt` | imagen | Raíces TLS para el HTTPS saliente. Monta tu propio paquete aquí para confiar en una autoridad privada. |

## Restablecer la contraseña {#resetting-the-password}

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
# abre la interfaz: /setup pide una nueva contraseña
docker compose up -d dumbmonit   # arranca de nuevo sin la variable
```

El restablecimiento borra la contraseña y cada sesión; los
dispositivos, reglas y canales no se tocan.

## Agente {#agent}

El agente tiene sus propias variables y archivo de configuración:
consulta [Agente para Linux, macOS, FreeBSD y
Windows](../devices/agent.md#configuration-file).

### Alcance de recolección {#collection-scope}

Qué mide el agente, y por lo tanto cuántas series produce una máquina.
Los valores por defecto mantienen a un host con decenas de contenedores
en unos pocos cientos de series; cada clave es una entrada de
`agent.yaml`, anulable por su variable de entorno.

| Clave | Variable | Por defecto | Rol |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Interfaces de red excluidas: interfaces de contenedor y VM, el loopback. Una lista de nombres exactos o expresiones regulares (sin anclar; cualquier cosa con un metacarácter es una regex). El valor sustituye al por defecto; una lista vacía conserva cada interfaz. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(vacío)* | Lista de permitidos, misma sintaxis. Si se define, solo se reportan estas interfaces y no se consulta `interfaces_ignore`. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Puntos de montaje excluidos del uso de sistema de archivos y E/S de disco, misma sintaxis. Las unidades extraíbles bajo `/run/media/` se mantienen. Los sistemas de archivos pseudo (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) siempre se omiten, sea cual sea la ruta. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Una serie `cpu_core_usage_percent` por núcleo, además del uso global. `cpu_count` se reporta de cualquier forma. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Contenedores detallados por ciclo, los en ejecución primero. Más allá, los contenedores solo se cuentan (`container_count`, `container_running_count` se mantienen exactos) y `container_series_skipped` indica cuántos. `0` cuenta sin detallar. |

En el entorno, las listas van separadas por comas (`^veth, ^br-,
docker0`); en YAML, o bien una lista o una cadena separada por comas. Los
contadores de E/S de disco (`disk_read_bytes`, `disk_written_bytes`)
llevan solo la etiqueta `device`: una serie por dispositivo de bloque,
sin importar cuántas veces esté montado.
