# Network discovery

Network discovery finds what is on a network for you, so you add monitors by
picking from a list instead of typing addresses one by one. You give it a
network in CIDR notation (`192.168.1.0/24`); it opens a short list of common
ports on every address, then makes **one anonymous request** to each open port
to recognise the product behind it. It never tries to log in.

One host can carry several services: a Proxmox node that also answers SSH and
SNMP shows all three, each with the DumbMonit monitor type that fits.

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

The SNMP community you give is a secret: it is sent to the devices being
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

## A word of caution

A port sweep looks like an attack to an intrusion detection system (IDS/IPS).
Only scan networks you own or are allowed to scan, and keep the prefix small.
The /22 limit is there for the same reason.
