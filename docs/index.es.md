# DumbMonit {#dumbmonit}

Monitorización sencilla, del homelab a la pequeña empresa. Un contenedor, una dirección IP
que escribir, gráficas útiles y alertas en menos de un minuto.

!!! warning "Trabajo en curso"
    DumbMonit está en pleno desarrollo; la imagen actual
    (`ghcr.io/laupernoe/dumbmonit:latest`) es una alfa para los primeros probadores. Cuenta con
    asperezas y cambios incompatibles hasta la primera versión estable.

DumbMonit lee la red como un **parte meteorológico**: la página de inicio enuncia
el cielo en una frase («Cielo despejado.» o «2 avisos, 1 inaccesible.») y
enumera lo que requiere tu atención, antes que nada. Las severidades siguen la escala
meteorológica (info → aviso → alerta), las predicciones son pronósticos y las ventanas
de mantenimiento son programadas.

![La página de resumen: la frase del parte, la lista «Needs you» y los pronósticos](assets/screenshots/overview-light.png){ loading=lazy }

<div class="dm-links" markdown>
<a href="install/docker/">Instalar<small>Docker Compose, primer inicio, copias de seguridad</small></a>
<a href="install/first-device/">Añade tu primer dispositivo<small>SNMP, escaneo de red, qué ocurre después</small></a>
<a href="alerting/">Alertas<small>Reglas integradas, silenciosas por diseño</small></a>
</div>

## Qué vigila {#what-it-watches}

| Fuente | Qué obtienes |
|---|---|
| [SNMP v1 / v2c / v3](devices/snmp.md) | Switches, routers, NAS, SAI, impresoras. Cinco perfiles se incluyen con el producto y se aplican automáticamente a partir del `sysObjectID` del dispositivo. Un escaneo de red añade de una vez todo lo que responde. |
| [Proxmox VE](devices/proxmox.md) | Nodos, máquinas virtuales y contenedores, almacenamientos, quórum del clúster y antigüedad de la última copia de seguridad correcta de cada máquina. |
| [Proxmox Backup Server](devices/pbs.md) | Uso del datastore y previsión de llenado, deduplicación, antigüedad y verificación del último snapshot de cada máquina, tareas fallidas, recolección de basura. |
| [Synology DSM](devices/synology.md) | Volúmenes, discos y su estado SMART, temperatura, carga, a través de la API web del NAS. |
| [Agente para Linux, macOS, FreeBSD y Windows](devices/agent.md) | CPU, memoria, discos, red, servicios, contenedores y tiempo de actividad de las máquinas que no hablan SNMP, además de temperaturas, salud de los discos (SMART) y pools ZFS cuando la máquina los expone. Un solo comando para instalarlo, o una imagen Docker; en [modo relay](install/remote-site.md) sondea los dispositivos propios de un sitio remoto usando únicamente conexiones salientes. |
| [Servicios](devices/services.md) | HTTP(S), puerto TCP, DNS, ping y caducidad de certificados TLS, al estilo Uptime Kuma, con barra de historial y porcentaje de disponibilidad. |
| [Paquetes de integración](packs/index.md) | Un tipo de dispositivo declarado en un único archivo YAML — una API HTTP o una página Prometheus `/metrics` del dispositivo — con sus propias reglas de alerta, instalado sin esperar a una versión del servidor. |

## Habla con él {#talk-to-it}

- Un [asistente a través de MCP](using/assistant.md) — Claude Code, Claude Desktop,
  ChatGPT, VS Code, Cursor — con 27 herramientas: un token `read` solo consulta (estado,
  dispositivos, alertas, métricas, agentes, contenedores…), un token `write` también puede actuar
  (silenciar, reconocer, añadir un dispositivo, reiniciar un contenedor, publicar un
  incidente en la página de estado). Los secretos nunca se devuelven.
- La [API HTTP](reference/api.md) que hay detrás de todo lo que hace la interfaz, descrita en
  `/api/openapi.json` (OpenAPI 3.1). Los tokens (`dmt_…`) tienen alcance de lectura o
  escritura, pueden caducar, restringirse a una lista de redes y están
  limitados en tasa.

## Qué se ejecuta {#what-runs}

| Contenedor | Función | Consumo |
|---|---|---|
| `dumbmonit` | Recolección, API, alertas, interfaz web y la VictoriaMetrics integrada para las series temporales | ~40 MB de RAM + el presupuesto de VictoriaMetrics (256 MB por defecto) |

Un contenedor, un volumen: el servidor arranca VictoriaMetrics desde la misma
imagen, y la configuración y el estado viven en una base de datos SQLite integrada. No
hay contenedor de base de datos. Se puede usar una VictoriaMetrics externa en su lugar
(`DUMBMONIT_VM_URL`).

!!! note "Sobre el nombre"
    DumbMonit se llamó EzyMonit hasta septiembre de 2026. Los comandos, variables de
    entorno, nombres de imagen y rutas se renombraron con él; las antiguas
    variables `EZYMONIT_*` y los tokens de agente `ezym_` todavía se aceptan. Consulta
    [Actualización](install/docker.md#upgrading).

DumbMonit es 100 % código abierto bajo la licencia Apache 2.0, dependencias
incluidas: ninguna función se reserva para una edición de pago.
