# Topology map

The **Map** page (**⋯ → Map** on the Monitors page, or *Map* in the command
palette, `/map`) draws your whole infrastructure in one picture. Nothing is
typed in: every link comes from what DumbMonit already knows, and an
administrator can [correct the map](#correct-the-map) where it is wrong.

## Where the links come from

| Link | Source | Drawn as |
|---|---|---|
| Depends on | The **parent** set on a monitor, the same one alert suppression follows. | solid line |
| Reached through a relay | The **relay agent** that polls a monitor on a remote site. | long dashes |
| Hosted by a hypervisor | The guests Proxmox VE, XCP-ng, Nutanix, vSphere or a Linux agent's libvirt module reported in the last 30 minutes. | short dashes |
| Runs on a guest | A monitor whose IP address is one the hypervisor reports for a VM or container (Proxmox guest agent, container interfaces or static configuration; VMware Tools on vSphere). | short dashes, tighter |
| Cabled to (LLDP) | An **LLDP neighbour** an SNMP switch sees on one of its ports (the `ethernet-switch` profile). | dots |
| Cabled to (switch ports) | A **layer-2 link** read from the switch tables: switch to switch (LLDP, or CDP on Cisco gear) and host to access port (bridge forwarding table matched to ARP). | dash-dot |

A guest or an LLDP neighbour is linked to a monitor only when its name
designates exactly one monitor (same name, or same host name in the address,
domain ignored). An ambiguous name creates no link rather than a wrong one. A
guest that matches no monitor appears as its own box (*Virtual machine* or
*Container*) next to its hypervisor; an unknown LLDP neighbour is left out. A
monitor without any known link simply stands in its folder.

Guest addresses also tie monitors to the guest they run on: an HTTP check, a
TCP port or a database monitor aimed at one of a VM's IP addresses hangs under
that VM. When a guest's name matches no monitor, the only agent, ping or SNMP
monitor at its address stands for the guest instead of a second box. An
address several guests report (a shared virtual IP) links nothing, a declared
parent always wins, and host names are not resolved. The guest's addresses are
shown in its details panel.

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

## Correct the map

The deduction is sometimes wrong: a device "goes through the firewall" when it
sits behind a switch, an HTTP check does not know which VM it runs on.
Administrators click **Correct the map** above the map:

- **Attach**: drag a device onto the one it really depends on. Without a mouse,
  pick the device and use **Attach** in its panel (search by name or type).
  **Also use for alerts** (ticked by default, devices only) makes it the
  device's declared parent too, the one alert suppression follows: when the
  parent goes down, the device's alerts are suppressed. Unticked, only the map
  changes. A loop of parents is refused.
- **This link is wrong**: click a link, then mark it wrong. It disappears and
  is never deduced again. Marking a declared parent wrong also clears it for
  alerts. A service found by an agent always hangs from its machine.
- **Link**: draw a link of a chosen type between two nodes.
- **Site** and **Group**: move a node to another site frame, or group it in the
  folder view of the map (its folder on Monitors does not change).

**Undo** and **Redo** (Ctrl+Z, Ctrl+Shift+Z) take each step back or forward.
Under the map, the list of corrections says who made each one and when; remove
one and the map (and a declared parent it changed) goes back to what it was.
Corrections are applied before the deduction, in this order: wrong links,
links drawn by hand and attachments, then learned rules. Each link says who
drew it: deduced by DumbMonit, corrected by hand (a hollow diamond on the
link) or a learned rule (a filled diamond). Corrections and rules are part of
the backup.

### What DumbMonit learns

From the corrections, DumbMonit suggests rules, with simple and predictable
heuristics (no statistics):

- a link marked wrong towards a device of a subnet suggests never drawing that
  type of link in the whole subnet (for example "Never draw *Through the
  firewall* links in 10.0.5.0/24");
- two devices at one address attached under the same parent suggest "what is
  at this address depends on it" (limited to their type when they share one:
  "HTTP monitors at 10.0.0.40 depend on vm-web");
- two devices of one subnet attached under the same parent suggest "Devices in
  10.0.5.0/24 depend on switch-core".

A suggestion is never applied on its own: **Apply the rule** or **Not this
one** (a declined rule is not suggested again). An applied rule also places
the devices added later, unless something surer places them (a declared
parent, a hypervisor, an agent service, a correction). Remove a rule and its
links go.

## Export and print

**Export** offers:

- **SVG image** and **PNG image**: the whole map with a title, the scope, the
  date and the legend, on a white ground, whatever the theme on screen.
- **Print / PDF (A4 landscape)** and **(A3 landscape)**: the browser prints
  only that sheet, scaled to the page. Choose *Save as PDF* in the print dialog
  for a PDF.

## API

`GET /api/topology` returns the nodes, the links and the source of each link
(any role, read-only; no device addresses, only the guest addresses a
hypervisor reports). `/api/topology/corrections`, `/api/topology/rules` and
`/api/topology/suggestions` hold the corrections, the learned rules and the
suggestions (writes for administrators). See the
[API reference](../reference/api.md).

## Layer-2 links (LLDP, CDP, bridge table)

The `ethernet-switch` SNMP profile (included by the Zyxel and HPE / Aruba
profiles) also reads LLDP and CDP neighbours, the bridge forwarding table
(FDB, `dot1d` and `dot1q`), the ARP table and 802.3ad link aggregations. The map
turns them into `l2` links:

- **Switch to switch**: a neighbour is matched to a monitor by its system name,
  otherwise by its chassis MAC address (seen in an ARP table). The link shows the
  port on each side when you hover it or open it. Parallel cables or an
  aggregate between the same two switches draw a single link.
- **Host to switch**: a MAC address learned on a port is matched, through ARP,
  to the monitor at that IP address. Only access ports count: a port with an
  LLDP/CDP neighbour, a member of an aggregate, or a port that learns more than
  24 addresses is treated as an uplink and carries no host. An address seen on
  several switches is attached where it is alone on its port.

Links are re-read at every collection and disappear once their measurements are
older than 30 minutes. They work through a relay agent like any SNMP monitor.
Links you correct by hand always win, and an `l2` link does not carry an outage
or suppress alerts by itself: attach the device to its switch from the map
("also use for alerts") when you want that.
