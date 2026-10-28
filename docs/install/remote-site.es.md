# Monitorizar un sitio remoto {#monitor-a-remote-site}

Un DumbMonit, varias redes. Un **agente relay** instalado en el sitio
remoto ejecuta, en nombre del servidor, las sondas de los monitores
que le asignes, y reporta los resultados por la misma conexión saliente
que ya usa para enviar sus propias métricas. Nada que abrir en el sitio
remoto, sin VPN, sin una segunda instancia que consultar.

```
 oficina central                          sitio remoto (NAT, cortafuegos)
 ┌──────────────────┐   HTTPS, saliente   ┌──────────────────────────────┐
 │ servidor DumbMonit│◀────────────────────│ agente relay                 │
 │  scheduler       │  sondas a ejecutar ──▶ │  ejecuta SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alertas, UI     │◀── muestras, veredicto │  envía también sus propias métricas │    hipervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Publicar el servidor por HTTPS {#1-publish-the-server-over-https}

El relay obtiene sus trabajos del servidor: el servidor debe ser
accesible desde el sitio remoto, normalmente
`https://monitor.example.org` detrás de un proxy inverso. Dos cosas
viajan por ese enlace:

- el **token de inscripción** (`dmon_…`), en la cabecera `Authorization`
  de cada petición — como con cualquier agente;
- las **credenciales de monitor** de los monitores que sondea el
  relay (comunidad SNMP, token de API de Proxmox…), descifradas, dentro
  de cada trabajo de sonda.

Por HTTP plano a través de internet ambas serían legibles en el camino:
pon el servidor detrás de HTTPS antes de hacer relay de cualquier cosa
fuera de tu propia red. En una LAN o una VPN, HTTP es aceptable y el
token es el único secreto en tránsito hasta que asignes monitores al
relay. Las credenciales nunca se escriben en disco en ninguno de los dos
lados: un trabajo de sonda vive en la memoria del servidor hasta que el
relay lo reporta, y el agente no conserva nada entre dos sondas.

El proxy inverso debe reenviar `/api/agent/relay` igual que
`/api/ingest`, permitir cuerpos de petición de hasta 16 MB (un
hipervisor ocupado reporta miles de muestras), y mantener las conexiones
inactivas abiertas durante al menos 60 segundos: el servidor retiene una
petición de relay hasta 25 segundos mientras espera trabajo (long
polling).

## 2. Crear un token de inscripción {#2-create-an-enrollment-token}

Añade un monitor, elige **Servidor con el agente** y marca *Este agente es un relé para un sitio
remoto* (o abre la entrada *Relay* de la página de alta). Da un nombre al sitio y crea el token:
lleva el modo relé y el sitio, así que no hay nada que editar en la máquina. Copia el valor
`dmon_…`: solo se muestra una vez; la página espera luego su primer envío.

## 3. Iniciar el relay en el sitio remoto {#3-start-the-relay-at-the-remote-site}

Cualquier máquina del sitio remoto con Docker, usando
`docker-compose.agent.yml` del repositorio:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

O el agente normal instalado con el instalador de una línea, con dos
claves más en `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

La máquina aparece en DumbMonit en segundos, como un monitor de
agente normal, y su página dice *Relay · site Lyon office · relays 0
devices*. El registro del agente dice `relay mode enabled: waiting for
probes from the server` una vez que se acepta su primer lote.

!!! note "Lo que el relay necesita alcanzar"
    El relay sondea los monitores desde su propia red: necesita el
    mismo acceso que necesitaría el servidor si estuviera ahí — UDP 161
    para SNMP, 8006 para Proxmox, 5001 para DSM, y así. Para los
    monitores de **ping**, el contenedor necesita el sysctl `net.ipv4.ping_group_range`, ya definido
    en el archivo compose (ver [Ping ICMP sin capability](docker.md#icmp-ping-without-a-capability)). Para monitores en una VLAN o una
    red link-local, o para objetivos `localhost`, usa `network_mode:
    host`.

## 4. Asignar monitores al relay {#4-assign-devices-to-the-relay}

Añade los monitores remotos como siempre — switch SNMP, nodo Proxmox,
comprobación HTTP — con su dirección **tal como se ve desde el sitio
remoto** (`192.168.10.1`, `https://pve.lan:8006`). En el formulario del
monitor, bajo *More options*, define **Reached through** al agente
relay en lugar de *Direct*. Los monitores existentes se pueden cambiar
de la misma forma; volver a *Direct* hace que el servidor los sondee de
nuevo.

A partir de entonces:

- el servidor encola una sonda para el monitor en cada intervalo; el
  relay la recoge en milisegundos (espera en el servidor por trabajo),
  ejecuta el recolector y reporta;
- **Probe now** y la detección de perfil en la página del monitor
  también pasan por el relay;
- la *última sonda* y el error del monitor siguen el veredicto del
  relay. Cuando el relay no recoge una sonda a tiempo, el monitor
  muestra *Timed out: relay agent … did not pick up the probe* —
  comprueba que el agente se ejecuta con `relay: true`;
- la página del agente muestra *Relay for N devices*, y `GET
  /api/relays` lista los relays con su sitio y recuento.

## 5. Instalar agentes detrás del relé {#5-install-agents-behind-the-relay}

Un sitio tras un cortafuegos suele tener servidores que no pueden llegar a DumbMonit en absoluto:
solo sale el relé. Esos servidores pueden ejecutar igualmente el agente completo (sistema, discos,
servicios, contenedores, copias de seguridad): se conectan a la **pasarela** del relé, y el relé
lleva su conexión hasta el servidor.

```
 remote site (only the relay goes out)                     head office
 ┌────────────┐  TLS 1.3, mTLS   ┌──────────────┐  tunnel inside the  ┌──────────────────┐
 │ agent      │═════════════════▶│ relay agent  │═ relay's own mTLS ═▶│ DumbMonit server │
 │ (server A) │  to the relay's  │  gateway     │  connection, TCP    │  agent port 8443 │
 └────────────┘  port 8443       └──────────────┘  8443 outbound      └──────────────────┘
        └──────────── one TLS session, end to end, agent ⇄ server ─────────────┘
```

**En DumbMonit**, añade un monitor, elige **Servidor con el agente** y, en *¿Cómo llega la máquina
a este servidor?*, elige **A través de &lt;relé&gt; (sitio)**. Indica la dirección por la que el
sitio llega al relé (una IP o un nombre local). Los comandos mostrados — Linux/macOS, Windows,
Docker — apuntan entonces a la pasarela del relé
(`--url=http://192.168.10.5:8443 --tls-url=https://192.168.10.5:8443`) y siguen llevando la huella
de la autoridad de certificación del **servidor**. El flujo espera la primera conexión y te deja
nombrar la máquina, archivarla en una carpeta y situarla en un sitio sin salir de la página. Desde
un sitio en *Añadir por sitio*, **Servidor con el agente** preselecciona el relé de ese sitio; la
página del propio relé tiene **Instalar un agente a través de él**.

Elegir un relé para una instalación le pide que abra su pasarela (en el puerto 8443 por defecto)
en su próximo envío. El panel *Relay* de la página del relé muestra la pasarela — *Escuchando en el
puerto 8443 — 3 agentes conectados* — y permite cambiar el puerto y la dirección, o cerrarla. Abre
ese puerto en el cortafuegos del propio relé, **solo hacia la red del sitio**. En un relé Docker,
publica el puerto (`-p 8443:8443`, o los `ports:` comentados de `docker-compose.agent.yml`).
`gateway_listen: 0.0.0.0:9443` en el `agent.yaml` del relé (`DUMBMONIT_AGENT_GATEWAY_LISTEN`)
prevalece sobre el ajuste de la interfaz.

Lo que pasa por el relé:

- **El canal cifrado del agente**, intacto. El relé reconoce un ClientHello TLS y copia los bytes,
  en ambos sentidos, en un túnel que abre hacia el puerto de agentes del servidor con su *propio*
  certificado de cliente (un upgrade HTTP en `/api/agent/relay/tunnel`). El servidor hace entonces
  el handshake TLS 1.3 con el agente del sitio como si se hubiera conectado directamente: el
  certificado de cliente del agente, la autoridad fijada, el token y el secreto de vinculación los
  comprueba el servidor, de extremo a extremo. La inscripción sigue el mismo camino.
- **El instalador y el binario**, que el relé obtiene del servidor por su propio canal y sirve en
  el mismo puerto (solo `/install.sh`, `/install.ps1`, `/download/<archivo>`). El comando de
  instalación lleva el SHA-256 de cada binario: el script rechaza cualquier otro.

El servidor registra, para cada agente, el relé por el que se conecta. La página del agente dice
*Llega a este servidor a través del relé …*; el mapa de topología dibuja el relé como su padre; y
cuando el relé cae, los agentes detrás se muestran como `suppressed` por el relé, no como muchas
caídas separadas. Un agente que después se conecta directamente (o por otro relé) se actualiza en
su siguiente envío.

El kit de Active Directory ofrece la misma elección: en **Route**, elige el relé, y los servidores
del dominio descargan el agente de él e informan a través de él.

## Alertas cuando el relay está caído {#alerting-when-the-relay-is-down}

Un relay que se queda en silencio se lleva consigo a sus monitores:
sus sondas se detienen, y todos se volverían *unreachable* de golpe. El
relay se trata como un **padre** de cada monitor que sondea (además
del padre que puedas haber definido): cuando el propio relay es
*unreachable*, las alertas de sus monitores se muestran como
`suppressed` con `suppressed_by` = el relay, y solo sale una
notificación — para el relay. El padre que definiste explícitamente
conserva su rol y gana cuando ambos están caídos.

## Límites {#limits}

- Un relay por monitor. Un monitor sondeado por el relay no es
  sondeado también por el servidor.
- Los monitores de agente nunca se sondean a través de un relé (envían sus propias métricas);
  un agente puede en cambio *conectarse* a través de la pasarela de un relé (ver arriba). Un relé
  siempre se conecta directamente: no hay relé detrás de un relé.
- La pasarela de un relé acepta por defecto 64 conexiones a la vez y 4 MiB/s por túnel y sentido
  (`gateway_max_connections`, `gateway_rate_kib`); el servidor acepta 256 túneles por relé.
- El relay ejecuta como máximo 8 sondas a la vez; una sonda está acotada
  por el tiempo de espera de sonda del servidor
  (`DUMBMONIT_PROBE_TIMEOUT_SECS`), limitado a 120 s en el agente.
- Un agente relay necesita los recolectores compilados: los agentes más
  antiguos que esta función ignoran `relay: true`, y el servidor reporta
  sus sondas como agotadas. Reinstala con el binario o imagen actual.
