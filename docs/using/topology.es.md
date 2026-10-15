# Mapa de topología

La página **Mapa** (botón **Mapa** en la página de monitores, *Mapa* en la paleta
de comandos, `/map`) dibuja toda tu infraestructura en una imagen. Nada se
introduce a mano: cada enlace procede de lo que DumbMonit ya sabe.

## De dónde vienen los enlaces

| Enlace | Origen | Trazo |
|---|---|---|
| Depende de | El **padre** definido en un monitor, el mismo que sigue la supresión de alertas. | línea continua |
| Alcanzado mediante un relé | El **agente relé** que consulta un equipo en un sitio remoto. | guiones largos |
| Alojado por un hipervisor | Los invitados que Proxmox VE, XCP-ng, Nutanix, vSphere o el módulo libvirt de un agente Linux informaron en los últimos 30 minutos. | guiones cortos |
| Cableado a (LLDP) | Un **vecino LLDP** que un switch SNMP ve en uno de sus puertos (perfil `ethernet-switch`). | puntos |

Un invitado o vecino LLDP solo se enlaza a un monitor si su nombre designa un
único monitor (mismo nombre o mismo nombre de host en la dirección, sin dominio).
Un nombre ambiguo no crea ningún enlace en lugar de uno falso. Un invitado sin
monitor aparece en su propia caja junto a su hipervisor; un vecino LLDP
desconocido se omite. Un monitor sin enlaces conocidos queda en su carpeta.

## Leer el mapa

- Lo de arriba a la izquierda, lo que depende de ello a la derecha; una sección por carpeta.
- Cada caja muestra el icono del tipo, el nombre y el estado en palabras. Un
  equipo **caído** tiene un borde grueso y una insignia `!`; lo que hay detrás
  (por padre, relé o hipervisor) se marca como **Afectado**, con borde
  discontinuo: sus alertas se suprimen.
- Un clic abre el monitor; la caja de un invitado abre su hipervisor.
- Arrastra para mover, rueda o pellizco para el zoom, flechas, `+`, `-` y `0` (ajustar).
- **Mostrar** limita el mapa a una carpeta o al sitio de un relé.
- En el móvil, la página se abre en la vista **Lista**: el mismo árbol, sangrado.

## Exportar e imprimir

**Exportar** ofrece **Imagen SVG**, **Imagen PNG** (título, alcance, fecha y
leyenda sobre fondo blanco) e **Imprimir / PDF (A4 horizontal)** o **(A3
horizontal)**: el navegador imprime solo esa hoja, ajustada a la página.

## API

`GET /api/topology` devuelve los nodos, los enlaces y su origen (cualquier rol,
solo lectura, sin direcciones). Consulta la [referencia de la API](../reference/api.md).
