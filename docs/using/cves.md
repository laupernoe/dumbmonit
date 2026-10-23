# CVE & vulnerabilities

**Security → CVE & vulnerabilities** lists the known vulnerabilities (CVE) of
the software installed on your monitors, and says for each one whether the
monitor is already **patched**, still **vulnerable**, or **to check** by hand.
The device page shows the same for one monitor.

!!! warning "A prompt to look, not a vulnerability scanner"
    DumbMonit compares **versions** it already collects with public
    advisories. It does not probe the monitors, does not see configuration
    (a vulnerable module that is disabled still shows), and only knows the
    software listed below. When it cannot decide, it says **to check**, never
    **vulnerable**.

## Turn it on

The scan is **off by default**: while off, nothing leaves your instance. An
administrator turns it on from the page itself. Once on, the software
inventory (product names and versions, package names and versions) is sent to
the public sources below; no host name, address or credential is ever sent.

Optional: an **NVD API key** (free, [request one](https://nvd.nist.gov/developers/request-an-api-key))
raises the NVD rate limit from 5 to 50 requests per 30 seconds, so the first
scan and the CVSS scores arrive much faster. It is stored encrypted and never
shown again.

## What is inventoried

Nothing extra is asked to the monitors: the inventory is built from what the
collectors already record.

| Source | Software |
| --- | --- |
| Proxmox VE | Proxmox VE version per node, and the packages listed by `apt/versions` (QEMU, LXC, Corosync, Ceph, swtpm, noVNC, and the Debian packages of the list) |
| Proxmox Backup Server, OPNsense, Unraid, VMware ESXi / vCenter, FortiGate (FortiOS), MikroTik (RouterOS), UniFi Network, Pi-hole, Nginx Proxy Manager, nginx, Apache, Traefik, MongoDB, Redis / Valkey, RabbitMQ, AdGuard Home, OpenVPN, Home Assistant, Kubernetes, MDaemon, SecurityGateway, Synology DSM, Veeam B&R, Immich, Forgejo, Jellyfin, Graylog | The product version their collector reports |
| Linux agent | Every installed package, once a day (`dpkg-query`, `rpm -qa` or `apk list -I`) |
| Windows agent | Windows edition, release and build (`10.0.22631.4317`), and installed programs (Firefox, Chrome, Edge, 7-Zip, Notepad++, VLC, PuTTY, WinSCP, WinRAR, KeePass, Wireshark, FileZilla, TeamViewer, AnyDesk, OpenVPN, MDaemon…) once a day |

TrueNAS, Proxmox Mail Gateway, Proxmox Datacenter Manager, XCP-ng, CrowdSec
and VictoriaMetrics are inventoried but no source covers them yet. The
**What is covered** panel lists every component with its source.

The agent's daily inventory can be turned off with `inventory: false` in
`agent.yaml` (or `DUMBMONIT_AGENT_INVENTORY=false`).

## Sources

| Source | Used for |
| --- | --- |
| [NVD](https://nvd.nist.gov/) (API 2.0) | Products, by CPE and version ranges; CVSS scores of every CVE |
| [Debian Security Tracker](https://security-tracker.debian.org/tracker/) | Debian (and Raspberry Pi OS) packages, and the Debian packages of Proxmox nodes: the version that fixes each CVE **in your Debian release**, backports included |
| [OSV.dev](https://osv.dev/) | Ubuntu, Alpine, AlmaLinux and Rocky Linux packages |
| [CISA KEV](https://www.cisa.gov/known-exploited-vulnerabilities-catalog) | Marks the CVEs that are **actively exploited** |

The sources are read **once a day** (or with **Refresh now**), in the
background, never during the collection cycle. Everything is cached in the
database: offline, the last known state stays on screen and the source shows
its error. NVD is read per product and then only for what changed
(`lastModStartDate`); the Debian tracker (about 14 MB compressed) is streamed
to disk and only the packages you run are kept. Outgoing requests honour the
standard `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` environment variables of
the container.

This product uses the NVD API but is not endorsed or certified by the NVD.

## Verdicts

| Verdict | Meaning |
| --- | --- |
| **Vulnerable** | The installed version is below the version that fixes it, inside the affected range, or the distribution lists it as open (no fix yet) |
| **To check** | The data cannot decide: a vendor-rebuilt package (Proxmox `-pve`, distribution revision) that may carry a backported fix, an advisory without version bounds, an ESXi update level, or an unreadable version |
| **Patched** | The installed version already includes the fix |

Versions are compared the way their ecosystem does: `dpkg` order for Debian
packages (`1:9.2p1-2+deb12u3`), RPM-style `epoch:version-release`, and
segment order for products and Windows builds.

To stay readable, the list keeps everything that is **exploited** or has a
**fix available**; a CVE without fix, or to check, from a distribution package
only shows when its CVSS is 7 or more (and never when Debian marks it as a
minor issue); **patched** CVEs only show for the last year. CVSS scores of
distribution CVEs are fetched from NVD progressively, a few hundred per hour
without a key.

## Alerts

Two built-in rules, adjustable like any other:

- **Exploited vulnerability not patched** (critical): a component is
  vulnerable to a CVE of the CISA KEV catalog.
- **Critical vulnerability not patched** (warning): a component is vulnerable
  to a CVE with a CVSS of 9 or more.

Everything else is only displayed; **to check** never alerts. The series are
`dumbmonit_cve_vulnerable{target,cve,component}` (value: CVSS, 0 if unknown)
and `dumbmonit_cve_exploited_vulnerable{target,cve,component}`.

## Ignore (accept the risk)

An administrator can **ignore** a CVE on one monitor, with an optional note
(the module is not used, the fix is applied another way…). It then stops
being counted and alerting, and stays listed, faded, with who ignored it and
when. **Restore** undoes it.
