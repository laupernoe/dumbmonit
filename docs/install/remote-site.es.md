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

**Settings → Agents → New token**, o añade un monitor de tipo
*agent*. Copia el valor `dmon_…`: se muestra una vez.

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
- Los propios monitores de agente no se pueden hacer relay: envían
  sus propias métricas.
- El relay ejecuta como máximo 8 sondas a la vez; una sonda está acotada
  por el tiempo de espera de sonda del servidor
  (`DUMBMONIT_PROBE_TIMEOUT_SECS`), limitado a 120 s en el agente.
- Un agente relay necesita los recolectores compilados: los agentes más
  antiguos que esta función ignoran `relay: true`, y el servidor reporta
  sus sondas como agotadas. Reinstala con el binario o imagen actual.
