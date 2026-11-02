# Monitores

Todo lo que DumbMonit vigila, visto de cuatro maneras. En la aplicación es **Monitores** en el menú izquierdo (`g d`), con cuatro pestañas. **Añadir un monitor** es el botón principal de la pestaña Lista (`g n`).

| Pestaña | Qué se ve | Leer |
|---|---|---|
| **Lista** | Cada monitor y su estado, filtros, carpetas y acciones en bloque; un clic abre el monitor | [La página de monitores](../using/devices.md) |
| **Mapa** | Qué depende de qué (`g m`), y sus sitios en un mapa mundial | [Mapa de topología](../using/topology.md), [Mapa mundial](../using/world-map.md) |
| **Registros** | Una sola búsqueda en los registros del servidor, de los agentes, de syslog y de los traps SNMP (`g l`, administradores) | [Registros](../using/logs.md) |
| **Seguridad** | De dónde vienen los ataques que DumbMonit detuvo (mapa de amenazas) y las vulnerabilidades conocidas (CVE) del software de sus monitores (`g x`); la puntuación de seguridad se activa en Ajustes | [CVE y vulnerabilidades](../using/cves.md), [Puntuación de seguridad](../using/security-score.md) |

El explorador de métricas (`Ctrl+K`, luego *Explore*) no es una pestaña: recorre las métricas `dumbmonit_*` y prueba una consulta ([Explorador de métricas](../using/dashboards.md)).

## Añadir monitores

- Uno a uno, desde **Añadir un monitor**: [Añadir su primer monitor](../install/first-device.md).
- Escaneando una red: [Descubrimiento de red](../using/discovery.md).
- A partir de las etiquetas de los contenedores: [Monitores desde etiquetas de Docker](../using/docker-labels.md).
- Desde otra herramienta (Uptime Kuma, Healthchecks, CSV): [Importar desde otras herramientas](../using/imports.md).
- Desde archivos guardados en Git: [Archivos de configuración (GitOps)](../using/config-files.md).

## Integraciones

Cada tipo de monitor tiene su página, agrupada por categoría en el menú izquierdo: servidores y agentes, virtualización, almacenamiento y NAS, red y cortafuegos, hardware, bases de datos, servidores web, identidad, nube y SaaS, hogar y multimedia, aplicaciones autoalojadas, copias de seguridad, correo, observabilidad y comprobaciones de Internet. La tabla de [todas las integraciones](../devices/index.md) las enumera con lo que necesita cada una.
