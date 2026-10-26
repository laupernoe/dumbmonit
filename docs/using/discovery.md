# Network discovery

Network discovery finds what is on a network for you, so you add monitors by
picking from a list instead of typing addresses one by one. You give it a
network in CIDR notation (`192.168.1.0/24`); it first looks for live hosts with
a ping (and the neighbour table when readable), opens a short list of common
ports on every address, then makes **one anonymous request** to each open port
to recognise the product behind it. It never tries to log in.

One host can carry several services: a Proxmox node that also answers SSH and
SNMP shows all three, each with the DumbMonit monitor type that fits. A host
that answers the ping but exposes no recognised service is still listed, with
"No service identified". The ping needs ICMP to be allowed to the container (the
`ping_group_range` setting of the bundled compose files, or `NET_RAW` in host
mode); without it the scan carries on and relies on ports and the neighbour
table alone.

## Running a scan

On the **Monitors** page, start a discovery scan, choose a suggested network (or
type one), and pick the depth:

- **Standard** sweeps about thirty common ports plus SNMP.
- **Extended** adds the less usual ports and everything the product fingerprints
  ask for. It is slower and noisier.

A scan is limited to **1024 addresses (a /22)**. Wider prefixes are refused: a
sweep of a whole /16 would be tens of thousands of connections. Results fill in
as the scan runs; you can cancel it at any time.

For each recognised service you get the product name, a confidence level, the
version when it is readable without logging in, and a button that opens the
**Add monitor** form already filled with the right type, address and port — you
only enter the credentials. If a monitor already points at that address, it is
shown as "already monitored" instead.

The SNMP community you give is a secret (empty means `public`): it is sent to the devices being
probed, never written to a log and never returned by the API.

## Remote sites

To scan a network the server cannot reach, choose a **relay agent** on that
site. The agent runs the exact same scan on its own network and streams the
results back; the server never has to reach the agent. The agent's own IP does
not need to be reachable from the server.

Relay discovery needs a recent agent. An agent too old to understand the command
fails the scan with a clear message asking you to update it.

## MAC addresses and the host network

The MAC address of a host — and the vendor it points to — can only be read on
the **same layer-2 segment**. By default the container runs on its own Docker
network, one hop away, so the MAC column stays empty. To get it, run the
container in **host network mode**, where it shares the host's network stack and
can read the neighbour (ARP) table:

```bash
docker compose -f docker-compose.host.yml up -d
```

Use that file on its own, not merged with `docker-compose.yml`.

Host network mode:

- is **Linux only**. On Docker Desktop (macOS, Windows) the container still runs
  inside a VM, so there is no shared segment and no MAC.
- exposes the web UI directly on the host's port 8080 (there is no port
  mapping in host mode). The embedded metrics database stays bound to the
  loopback and is not reachable from the LAN.

## Scheduled scans and inventory

A one-off scan tells you what is on the network today. To be told when something
**new** shows up, schedule it: on the **Monitors** page, in the network scan view,
the *Scheduled scans and inventory* panel keeps a list of the devices and open
ports each scheduled network has shown, and flags what is new.

For each schedule you choose:

- the **network** (CIDR, 1024 addresses at most, like a manual scan);
- **who scans**: the server, or a [relay agent](../devices/agent.md) on another site;
- the **interval**, from 5 minutes to 30 days;
- the **depth** (the same *Standard* or *Extended* port list as a manual scan);
- whether it is **enabled**. *Scan now* runs it at the next cycle (a few seconds)
  without waiting for the interval.

A network is never scanned twice at once, and a server or a relay agent runs one
discovery scan at a time: a scheduled scan that finds its scanner busy (a manual
scan, another schedule) simply starts a little later.

### The inventory

Each scan that completes updates the inventory of its network:

- a **device** is identified by its **MAC address** when it can be read (see
  [MAC addresses](#mac-addresses-and-the-host-network)), otherwise by its IP
  address. A device whose IP changes but keeps its MAC is the same device, not a
  new one;
- every **open port** found on a device is listed with its first and last
  sighting. Only what the scan recognises is listed (the same services as a
  manual scan);
- nothing is ever removed: a device that is switched off stays in the list.

The **first completed scan** of a network builds the inventory without reporting
anything: everything it finds is known from the start. A first scan that finds no
host at all (a relay cut off from its network, say) does not count, so that the
next one does not flag a whole network as new. Changing the network or the
scanner of a schedule starts again from an empty inventory.

From the second scan on, a device or a port that is not in the inventory enters it
as **new**: highlighted with a *New device* or *New port* label (not only a colour),
and kept at the top of the list.

### Alerts

New devices and new ports raise alerts through two built-in
[alert rules](../alerting/rules.md#network-inventory):

| Rule | Raised when | Default |
|---|---|---|
| New device on the network | a scan finds a device that is not in the inventory | **disabled**, severity `info` |
| New open port on the network | a scan finds a new open port on a known device | **disabled**, severity `info` |

Both rules are **off by default**: on a network where guests come and go, they would
be noise until you want them. Enable them in the alert rules, and choose which
notification channels receive them like any other rule.

There is **one alert per new item**, and it stays open as long as the item is not
marked as known; it does not repeat and does not fire again at every scan. The ports
of a new device are covered by the alert of the device itself.

### Marking as known

Use *Mark as known* on a device (its ports are marked with it), *Mark known* on a
single port, or *Mark all as known* to accept everything at once. Marked items
stop being flagged, and their alert resolves within a few minutes. This is also
the way to accept a new port or device you expected.

## A word of caution

A port sweep looks like an attack to an intrusion detection system (IDS/IPS).
Only scan networks you own or are allowed to scan, and keep the prefix small.
The /22 limit is there for the same reason.
