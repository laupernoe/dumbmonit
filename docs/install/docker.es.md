# Instalar con Docker {#install-with-docker}

DumbMonit se ejecuta como un único contenedor: el servidor `dumbmonit` (recolección, API,
alertas, interfaz web) arranca su propia VictoriaMetrics para almacenar las series temporales; el
binario viene en la imagen. La configuración y el estado viven en una base de datos SQLite
integrada. Un volumen, `/data`, contiene la base de datos, el secreto de la instancia y las
series temporales.

!!! note "Imagen alfa"
    `ghcr.io/laupernoe/dumbmonit:latest` es la última compilación alfa etiquetada (amd64;
    las imágenes arm64 están en pausa por ahora); `:edge` sigue el último commit de `main`. Para ejecutar desde el código fuente,
    `docker compose up -d --build` compila la misma imagen en local (unos diez
    minutos la primera vez; solo hace falta Docker).

## Requisitos previos {#prerequisites}

- Docker Engine con el plugin Compose (`docker compose version` funciona).
- Una máquina que pueda alcanzar los dispositivos que quieres monitorizar. No necesita
  ser accesible desde ellos, salvo para los [agentes](../devices/agent.md), que envían
  sus mediciones al servidor por HTTP.
- El puerto `8080` libre en el host, u otro puerto de tu elección (ver más abajo).

## El archivo Compose {#the-compose-file}

Este es el `docker-compose.yml` del repositorio; guárdalo en un directorio
propio:

```yaml
# Un contenedor: el servidor DumbMonit ejecuta su propia VictoriaMetrics (integrada en
# la imagen) y guarda todo en un único volumen. Para usar una VictoriaMetrics
# externa, define DUMBMONIT_VM_URL y la integrada no se inicia.
name: dumbmonit

services:
  dumbmonit:
    # `docker compose up -d` descarga la imagen publicada (última versión; usa
    # `:edge` para el último commit de main). Para compilar desde este checkout,
    # ejecuta `docker compose up -d --build`: el resultado se etiqueta con el mismo nombre
    # y se usa a partir de entonces.
    image: ghcr.io/laupernoe/dumbmonit:latest
    build: .
    ports:
      # El puerto del host es configurable: 8080 es un puerto muy disputado en una
      # máquina de homelab. `DUMBMONIT_PORT=8099 docker compose up -d` lo cambia.
      - "${DUMBMONIT_PORT:-8080}:8080"
    volumes:
      # Base de datos SQLite, secreto de la instancia y las series temporales (/data/vm).
      #
      # El servidor se ejecuta como usuario 65532 (no root). Un volumen con nombre creado por
      # Docker hereda ese propietario de la imagen: no hay nada que hacer. Un bind mount
      # (`./data:/data`) o un volumen creado antes de este cambio debe cederse
      # una vez:
      #   docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
      # o, para conservar los archivos tal como están, ejecuta el contenedor como su propietario
      # con `user: "1000:1000"` (cualquier uid sirve: la imagen no tiene /etc/passwd).
      - dumbmonit-data:/data
    environment:
      # Descomenta para definir tú mismo el secreto en lugar de dejar que DumbMonit
      # lo genere en /data/secret.key. Cifra las credenciales de los dispositivos: perderlo
      # significa volver a introducirlas todas.
      # DUMBMONIT_SECRET: replace-me-with-32-random-characters
      # Contraseña perdida: `DUMBMONIT_RESET_PASSWORD=1 docker compose up -d` borra
      # la contraseña al arrancar y la interfaz pide una nueva; después vuelve a arrancar
      # sin la variable.
      DUMBMONIT_RESET_PASSWORD: ${DUMBMONIT_RESET_PASSWORD:-}
      # VictoriaMetrics integrada: retención (meses, o p. ej. 30d / 2y) y el
      # presupuesto de memoria de sus cachés. Un homelab de unas pocas docenas de dispositivos cabe en
      # 256 MB; auméntalo cuando monitorices cientos de hosts.
      DUMBMONIT_VM_RETENTION: ${DUMBMONIT_VM_RETENTION:-12}
      DUMBMONIT_VM_MEMORY: ${DUMBMONIT_VM_MEMORY:-256MB}
      # VictoriaMetrics externa en lugar de la integrada:
      # DUMBMONIT_VM_URL: http://victoriametrics:8428
    # Mínimo privilegio: ninguna capability, sin escalada de privilegios, y el
    # sistema de archivos de la imagen en solo lectura — /data (volumen) y /tmp (tmpfs) son los
    # únicos lugares con escritura. Los monitores ICMP «ping» tampoco necesitan ninguna capability: el
    # sysctl de abajo permite al servidor (sin privilegios) abrir sockets ICMP echo dentro
    # del espacio de nombres de red del propio contenedor. Quítalo si nunca usas ping.
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    read_only: true
    tmpfs:
      - /tmp
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
    # El servidor detiene VictoriaMetrics tras de sí: déjale tiempo para hacerlo.
    stop_grace_period: 30s
    restart: unless-stopped

volumes:
  dumbmonit-data:
    # Nombre fijo, independiente del nombre del proyecto compose: el volumen es lo que
    # respaldas y lo que una actualización debe volver a encontrar.
    name: dumbmonit-data
```

Después, arráncalo:

```bash
docker compose up -d
```

!!! tip "Compilar la imagen tú mismo"
    `docker compose up -d` descarga la imagen publicada e ignora `build: .`.
    Desde un clon del repositorio, `docker compose up -d --build` compila la
    misma imagen en local (unos diez minutos en frío; no hace falta ninguna
    toolchain de Rust ni de Node en el host). Úsalo para ejecutar desde el código fuente o desde una
    rama.

## VictoriaMetrics integrada {#embedded-victoriametrics}

La imagen contiene el binario de VictoriaMetrics (`/victoria-metrics-prod`, de
`victoriametrics/victoria-metrics:v1.152.0`). Cuando `DUMBMONIT_VM_URL` no está
definida, el servidor lo inicia como proceso hijo escuchando en `127.0.0.1:8428`,
guarda sus series en `/data/vm`, redirige sus líneas de log al log propio,
lo reinicia con espera progresiva si se cae y lo detiene al apagarse. No se publica
nada: el puerto se queda dentro del contenedor.

Conviene conocer dos variables: `DUMBMONIT_VM_RETENTION` (`12` meses por
defecto; `30d` o `2y` también valen) y `DUMBMONIT_VM_MEMORY` (`256MB`, el
presupuesto de sus cachés; auméntalo para cientos de hosts). El resto está en la
[referencia de configuración](../reference/configuration.md#embedded-victoriametrics).

Para usar una VictoriaMetrics que ya tengas en marcha, define `DUMBMONIT_VM_URL` con su
dirección (`http://host:8428`): la integrada entonces no se inicia y
`/data/vm` queda vacío.

## Primer inicio {#first-start}

Abre `http://<tu-host>:8080`. Una instancia nueva muestra la pantalla `/setup`,
donde creas la primera cuenta de administrador (contraseña de al menos 12
caracteres).

La pantalla pide primero el **código de configuración**. Mientras no exista ningún administrador,
el servidor imprime un código de un solo uso en sus logs al arrancar:

```sh
docker compose logs dumbmonit | grep -A1 "setup code"
```

```
  First-run setup code: K7XQ4-M9PRT
```

Demuestra que quien crea la cuenta de administrador controla el servidor, de modo que nadie
más en la red pueda reclamar antes una instancia nueva. El código vive solo
en memoria, cambia en cada reinicio hasta que exista un administrador, y los
intentos fallidos tienen límite de tasa. Para un despliegue automatizado, define
`DUMBMONIT_SETUP_CODE` y elígelo tú mismo.

![La pantalla de inicio de sesión](../assets/screenshots/login-light.png){ loading=lazy }

Después añade tu primer dispositivo: consulta [Añade tu primer dispositivo](first-device.md).

## Variables de entorno {#environment-variables}

Todo se configura mediante variables de entorno; ninguna es obligatoria.

| Variable | Valor por defecto | Función |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Dirección de escucha dentro del contenedor. |
| `DUMBMONIT_DATA_DIR` | `/data` | Base de datos SQLite (`dumbmonit.db`), secreto de la instancia (`secret.key`) y los datos de la VictoriaMetrics integrada (`vm/`). |
| `DUMBMONIT_VM_URL` | *(sin definir)* | URL de una VictoriaMetrics externa. Si se define, la integrada no se inicia. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retención de la VictoriaMetrics integrada: meses, o `30d`, `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Presupuesto de memoria de las cachés de la VictoriaMetrics integrada. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Dirección de escucha de la VictoriaMetrics integrada, dentro del contenedor. |
| `DUMBMONIT_SECRET` | *(generado)* | Secreto de la instancia que cifra las credenciales de los dispositivos y los tokens. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Sondeos simultáneos, todos los colectores combinados. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Duración máxima de un sondeo. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Periodo de escritura hacia VictoriaMetrics. |
| `DUMBMONIT_LOG` | `info` | Filtro de logs (sintaxis de `tracing`, p. ej. `debug`, `dumbmonit=trace`). |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Binarios de los agentes servidos bajo `/download/…`. |
| `DUMBMONIT_RESET_PASSWORD` | *(vacío)* | Defínela como `1` para borrar la contraseña y todas las sesiones al arrancar. |
| `DUMBMONIT_SETUP_CODE` | *(generado)* | Código de configuración que pide `/setup` mientras no exista un administrador. Sin definir, se imprime uno aleatorio en los logs en cada arranque. |
| `DUMBMONIT_COOKIE_SECURE` | *(desactivado)* | Defínela como `1` tras un proxy inverso TLS para marcar la cookie de sesión como `Secure`. |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Periodo de evaluación de alertas (nunca por debajo de 10). |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | 90 días | Retención del historial de alertas. Consulta la [referencia de configuración](../reference/configuration.md) para una salvedad sobre su unidad. |
| `DUMBMONIT_BACKUP_ENABLED` | activado | Copias de seguridad locales programadas de la base de datos en `/data/backups/`. `DUMBMONIT_BACKUP_DIR`, `_INTERVAL_HOURS` (24) y `_KEEP` (7) las ajustan; consulta [Copia de seguridad y restauración](backup.md). |

La lista completa, con detalles, está en la [referencia de configuración](../reference/configuration.md).
Los nombres `EZYMONIT_*` anteriores al cambio de nombre se siguen leyendo como alternativa; consulta
[Actualización](#upgrading).

## Copia de seguridad de la clave secreta {#back-up-the-secret-key}

Las comunidades SNMP, las contraseñas y los tokens de API se cifran con AES-256-GCM usando una
clave derivada del secreto de la instancia. En el primer inicio, DumbMonit genera este
secreto en `/data/secret.key` (dentro del volumen `dumbmonit-data`).

!!! danger "Respalda `secret.key` junto con la base de datos"
    Sin ella, las credenciales de los dispositivos son irrecuperables: una base de datos restaurada
    sola da una instancia incapaz de comunicarse con nada. El servidor detecta al arrancar una
    clave ausente o modificada y se niega a continuar con un mensaje explícito, en lugar de
    fallar en silencio en cada sondeo. Las copias programadas
    la incluyen por ti — consulta [Copia de seguridad y restauración](backup.md).

Si prefieres controlar el secreto, define `DUMBMONIT_SECRET` en el archivo Compose
(32 caracteres aleatorios o más). Guárdalo en tu gestor de contraseñas.

## Actualización {#upgrading}

```bash
docker compose pull
docker compose up -d
```

Las migraciones de la base de datos se ejecutan al arrancar. Los datos de VictoriaMetrics no se tocan.

!!! tip "Haz antes una copia de seguridad"
    **Settings → Backup → Back up now** escribe en unos segundos una copia consistente de la base de datos
    y de `secret.key` en `/data/backups/`, y es desde donde
    restauras si la actualización sale mal. Consulta
    [Copia de seguridad y restauración](backup.md).

### Desde 0.1.0-alpha.1 {#from-010-alpha1}

Desde 0.1.0-alpha.2 el contenedor se ejecuta como usuario 65532 en lugar de root. Un
volumen creado por alpha.1 sigue perteneciendo a root, y el servidor se niega a
arrancar (`/data is not writable by the server`). Cede el volumen una vez:

```bash
docker compose down
docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
docker compose up -d
```

O conserva los archivos tal como están y ejecuta el contenedor como su propietario, con
`user: "0:0"` (o el uid de un bind mount) dentro del servicio en
`docker-compose.yml`.

### Desde EzyMonit y desde la configuración de dos contenedores {#from-ezymonit-and-from-the-two-container-setup}

DumbMonit se llamó EzyMonit hasta septiembre de 2026, y se ejecutaba como dos contenedores,
el servidor y un servicio `victoriametrics` aparte. Actualizar exige mover una vez los datos
del proyecto antiguo al nuevo volumen. El proyecto antiguo se llamaba
`ezymonit` (sus volúmenes, `ezymonit_ezymonit-data` y `ezymonit_vm-data`;
`docker volume ls` lo confirma). Desde el directorio antiguo:

```bash
docker compose down
```

Después, con el nuevo `docker-compose.yml`:

```bash
docker volume create dumbmonit-data
docker run --rm -v ezymonit_ezymonit-data:/from -v dumbmonit-data:/to alpine cp -a /from/. /to/
docker run --rm -v ezymonit_vm-data:/from -v dumbmonit-data:/to alpine sh -c 'mkdir -p /to/vm && cp -a /from/. /to/vm/'
docker compose up -d
```

El antiguo directorio de datos de VictoriaMetrics tiene la estructura que usa la integrada:
nada que convertir. La alternativa es conservar la VictoriaMetrics externa y
apuntar `DUMBMONIT_VM_URL` hacia ella.

Lo que el servidor gestiona por sí solo en el primer inicio:

- Las variables `EZYMONIT_*` se siguen leyendo como alternativa, con un aviso al arrancar
  por variable (`EZYMONIT_X is deprecated, use DUMBMONIT_X`).
- `/data/ezymonit.db` se renombra a `dumbmonit.db`.
- Los tokens de agente `ezym_…` siguen funcionando; los nuevos son `dmon_…`. Los agentes siguen
  enviando datos; vuelve a ejecutar el comando de instalación en cada máquina cuando te venga bien, así
  [migra el servicio antiguo en el sitio](../devices/agent.md#upgrade).
- La cookie de sesión cambió de nombre: todo el mundo vuelve a iniciar sesión una vez. Se conserva la
  preferencia de tema del navegador.

Lo que no gestiona: el prefijo de las métricas cambió de `ezymonit_` a
`dumbmonit_` sin compatibilidad. Las series antiguas permanecen en VictoriaMetrics con
el nombre antiguo y caducan según la retención; las gráficas y las reglas integradas reinician
desde la actualización. Las reglas de alerta personalizadas que nombren métricas `ezymonit_…` deben
editarse.

## Copia de seguridad y restauración {#backup-and-restore}

Un único volumen con nombre, `dumbmonit-data`, lo contiene todo:

| Ruta | Contenido |
|---|---|
| `dumbmonit.db` | Dispositivos, reglas, canales, estado de las alertas, sesiones. |
| `secret.key` | El secreto de la instancia. |
| `backups/` | Las copias locales programadas: una copia de la base de datos y del secreto, diaria por defecto, siete conservadas. |
| `vm/` | Series temporales de VictoriaMetrics (12 meses de retención por defecto). |

DumbMonit respalda su propia base de datos de forma programada, y exporta toda la
configuración — dispositivos, credenciales, reglas, canales — como un único archivo
cifrado con una frase de contraseña que tú eliges, que se restaura en una instancia nueva.
Ambas opciones están en **[Copia de seguridad y restauración](backup.md)**, junto con lo que hay que hacer antes de
una actualización y la única regla sobre `secret.key` que conviene leer dos veces.

Para archivar el propio volumen, gráficas incluidas, detén la pila y comprímelo con tar:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Para restaurar, crea el volumen, extrae el archivo en él del mismo modo y después
`docker compose up -d`.

Si el espacio importa, `--exclude=./vm` mantiene el archivo pequeño: la base de datos y
el secreto son la configuración, `vm/` son solo las gráficas.

## Cambiar el puerto {#changing-the-port}

Define `DUMBMONIT_PORT` al arrancar:

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

O pon `DUMBMONIT_PORT=8099` en un archivo `.env` junto al archivo Compose.

## Proxy inverso {#reverse-proxy}

La interfaz y la API se sirven en un solo puerto sobre HTTP plano. No se usa WebSocket:
la interfaz consulta la API periódicamente, así que cualquier proxy inverso funciona sin configuración
especial.

=== "Caddy"

    ```
    monit.example.com {
        reverse_proxy 127.0.0.1:8080
    }
    ```

=== "nginx"

    ```nginx
    server {
        listen 443 ssl;
        server_name monit.example.com;
        # ssl_certificate / ssl_certificate_key …

        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            # Los agentes envían lotes de recuperación tras una interrupción:
            client_max_body_size 16m;
        }
    }
    ```

Cuando el proxy termina TLS, define `DUMBMONIT_COOKIE_SECURE: "1"` para que la cookie de
sesión solo se envíe por HTTPS. No la definas en un despliegue HTTP plano: el
navegador nunca devolvería la cookie y sería imposible iniciar sesión.

!!! note "Agentes detrás de un proxy"
    El comando de instalación que se muestra al crear un token de agente usa la URL con la que
    tu navegador accedió a la interfaz. Si es la URL del proxy, los agentes también la usarán:
    asegúrate de que `/install.sh`, `/install.ps1`, `/download/…` y `/api/ingest`
    pasan por el proxy.

## Se ejecuta como usuario no root {#runs-as-a-non-root-user}

El contenedor ejecuta el servidor como usuario `65532:65532` (numérico: la imagen `scratch`
no tiene `/etc/passwd`), con todas las capabilities eliminadas, `no-new-privileges`
y un sistema de archivos raíz de solo lectura; `/data` (el volumen) y `/tmp` (un tmpfs)
son las únicas rutas con escritura. Todo esto está en el archivo Compose de arriba.

Un volumen con nombre creado por Docker en el primer inicio hereda de la imagen el propietario de `/data`:
no hay nada que hacer. Dos casos necesitan un comando:

- **Un volumen creado por una versión anterior** (el servidor se ejecutaba como root,
  así que los archivos pertenecen a root). Cédelos una vez, con la pila detenida:

  ```bash
  docker compose stop
  docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
  docker compose up -d
  ```

  De lo contrario, el síntoma es un contenedor que termina de inmediato con
  `creating directory /data … Permission denied` (o `unable to open database
  file`).

- **Un bind mount** (`./data:/data`) conserva el propietario del directorio del host.
  O bien `chown -R 65532:65532 ./data`, o ejecuta el contenedor como el propietario
  del directorio: `user: "1000:1000"` dentro del servicio, cualquier uid sirve.

## Ping ICMP sin capability {#icmp-ping-without-a-capability}

El [monitor de ping](../devices/services.md#ping) envía ecos ICMP. En lugar de
un socket raw — que necesita `NET_RAW`, y una capability concedida a un usuario no root
del contenedor no es utilizable de todos modos —, usa un socket ICMP echo, que Linux
permite a los grupos listados en `net.ipv4.ping_group_range`. El archivo Compose
define ese sysctl dentro del espacio de nombres de red del propio contenedor:

```yaml
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Nada cambia en el host. Docker 20.10 y posteriores definen este rango por sí mismos en cada
contenedor; las líneas explícitas hacen que se cumpla en motores más antiguos y
en Podman. Sin él, la comprobación informa de un error de configuración (se muestra en el
dispositivo, no se notifica), nunca de un falso «host caído». Con `docker run`, pasa
`--sysctl net.ipv4.ping_group_range="0 2147483647"`.

## Probar sin hardware {#testing-without-hardware}

Añade un [dispositivo de demostración](../devices/demo.md): produce mediciones falsas sin
nada que preparar, así que se pueden probar las gráficas, las reglas y las notificaciones
enseguida. Para datos SNMP reales, la máquina que ejecuta Docker suele bastar — instala
`snmpd` en ella (`apt install snmpd`, permite la comunidad `public` en el bridge de Docker
en `/etc/snmp/snmpd.conf`) y añade un dispositivo SNMP con la dirección del host
en ese bridge (`172.17.0.1` por defecto): el perfil se detecta
automáticamente y aparecen interfaces, memoria y procesos.

Los desarrolladores pueden añadir el overlay `docker-compose.dev.yml` para publicar la
VictoriaMetrics integrada en el `:8428` del host y hacer consultas MetricsQL directas:

```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```
