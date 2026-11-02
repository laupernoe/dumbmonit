# Monitors

Everything DumbMonit watches, seen four ways. In the app it is **Monitors** in the left menu (`g d`), with four tabs. **Add a monitor** is the main button of the List tab (`g n`).

| Tab | What you see | Read |
|---|---|---|
| **List** | Every monitor and its state, filters, folders and bulk actions; a click opens the monitor | [Monitors page](../using/devices.md) |
| **Map** | What depends on what (`g m`), and your sites on a world map | [Topology map](../using/topology.md), [World map](../using/world-map.md) |
| **Logs** | One search over the server's, the agents', syslog and SNMP trap logs (`g l`, administrators) | [Logs](../using/logs.md) |
| **Security** | Where the attacks DumbMonit stopped come from (threat map), and the known vulnerabilities (CVE) of the software on your monitors (`g x`); the security score is switched on in Settings | [CVE & vulnerabilities](../using/cves.md), [Security score](../using/security-score.md) |

The metrics explorer (`Ctrl+K`, then *Explore*) is not a tab: it browses the `dumbmonit_*` metrics and tries a query ([Metrics explorer](../using/dashboards.md)).

## Add monitors

- One at a time, from **Add a monitor**: [Add your first monitor](../install/first-device.md).
- By scanning a network: [Network discovery](../using/discovery.md).
- From the containers' labels: [Monitors from Docker labels](../using/docker-labels.md).
- From another tool (Uptime Kuma, Healthchecks, CSV): [Import from other tools](../using/imports.md).
- From files kept in Git: [Configuration files (GitOps)](../using/config-files.md).

## Integrations

Each type of monitor has a page, grouped by category in the left menu: servers and agents, virtualization, storage and NAS, network and firewalls, hardware, databases, web servers, identity, cloud and SaaS, home and media, self-hosted apps, backup, email, observability and Internet checks. The table of [all integrations](../devices/index.md) lists them with what each one needs.
