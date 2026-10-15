# Topology map

The **Map** page (button **Map** on the Monitors page, or *Map* in the command
palette, `/map`) draws your whole infrastructure in one picture. Nothing is
typed in by hand: every link comes from what DumbMonit already knows.

## Where the links come from

| Link | Source | Drawn as |
|---|---|---|
| Depends on | The **parent** set on a monitor, the same one alert suppression follows. | solid line |
| Reached through a relay | The **relay agent** that polls a monitor on a remote site. | long dashes |
| Hosted by a hypervisor | The guests Proxmox VE, XCP-ng, Nutanix, vSphere or a Linux agent's libvirt module reported in the last 30 minutes. | short dashes |
| Cabled to (LLDP) | An **LLDP neighbour** an SNMP switch sees on one of its ports (the `ethernet-switch` profile). | dots |

A guest or an LLDP neighbour is linked to a monitor only when its name
designates exactly one monitor (same name, or same host name in the address,
domain ignored). An ambiguous name creates no link rather than a wrong one. A
guest that matches no monitor appears as its own box (*Virtual machine* or
*Container*) next to its hypervisor; an unknown LLDP neighbour is left out. A
monitor without any known link simply stands in its folder.

When two sources link the same pair, the map keeps the surest one, declared
before deduced. Each box hangs from one link; the others are drawn lighter.

## Reading the map

- Upstream on the left, what depends on it to the right; one section per folder.
- Every box shows the type icon, the name and its state in words. A monitor
  that is **down** gets a thick outline and a `!` badge; what sits behind it
  (through a parent, a relay or a hypervisor) is marked **Impacted**, with a
  dashed outline: its alerts are suppressed. LLDP links do not carry an outage.
- Hovering or focusing a box highlights its links. Click (or Enter) opens the
  monitor; a guest box opens its hypervisor.
- Drag to move, scroll or pinch to zoom, arrow keys to pan, `+`, `-` and `0`
  (fit). The buttons in the corner do the same.
- **Show** narrows the map to a folder or to a relay site; the checkbox hides
  unmonitored virtual machines and containers.
- On a phone the page opens on the **List** view: the same tree, indented.

## Export and print

**Export** offers:

- **SVG image** and **PNG image**: the whole map with a title, the scope, the
  date and the legend, on a white ground, whatever the theme on screen.
- **Print / PDF (A4 landscape)** and **(A3 landscape)**: the browser prints
  only that sheet, scaled to the page. Choose *Save as PDF* in the print dialog
  for a PDF.

## API

`GET /api/topology` returns the nodes, the links and the source of each link
(any role, read-only, no addresses). See the
[API reference](../reference/api.md).
