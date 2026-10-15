# Security score

The security score is an **optional** grade from **A** to **F** (and a score
from 0 to 100) shown next to the devices where a few checks say something
useful: Linux and Windows hosts with the agent, Proxmox VE and Proxmox Backup
Server, Synology, OPNsense, websites and Active Directory. Other devices show
nothing.

!!! warning "Indicative, not an audit"
    The score is **approximate and not exhaustive**. It only reads what
    DumbMonit already collects from a device and compares it with a handful of
    the vendor's best practices. A grade A does not mean a device is secure,
    and a missing check does not mean the setting is fine. Use it as a prompt
    to look, then use a real assessment tool (PingCastle, Mozilla HTTP
    Observatory, Lynis, the vendor's own security advisor…) when it matters.

## Turn it on

The score is **off by default**. An administrator turns it on in
**Settings → Account & security → Security score**. Once on:

- each rated device gets a small **shield badge** with its letter in the
  device list, and a **Security** badge with the score on its page;
- clicking the badge opens the device's **report**: what fails, with the
  evidence and the fix, what passes, the steps to improve the score on this
  kind of device, and a link to the matching section below.

Turning it off hides the badges, stops the periodic computation, and the
`dumbmonit_security_score` series stop being written.

!!! note "Upgrading from 0.1.0-alpha.8 or earlier"
    The score used to be always on, with its own **Security** page. An
    installation that already monitored a rated device keeps the score on
    after the upgrade; a new installation starts with it off. The old
    `/security` address now opens the setting.

## How the score is computed

DumbMonit never asks the device anything extra to compute the score. The
checks read the last values the collectors already recorded (looking back
6 hours), so a score costs nothing to the device.

Each check has a category (exposure, patching, authentication, encryption,
backup, configuration), a **severity** that is its weight (critical 10,
high 6, medium 3, low 1), a result (**pass**, **fail**, or **unknown** when
the data is not collected), the evidence, the fix and a link to the vendor
page it is based on.

**Score** = 100 × (weight of passing checks) ÷ (weight of passing + failing
checks). **Unknown checks are shown but never counted**: a score must not
drop because a monitoring account lacks a privilege, nor rise because nothing
is known. A device whose checks are all unknown is *not rated*.

| Grade | Score | Word |
|---|---|---|
| A | 90–100 | Strong |
| B | 75–89 | Good |
| C | 60–74 | Fair |
| D | 40–59 | Weak |
| F | 0–39 | Poor |

**A failing critical check caps the grade at C**, whatever the score.

## History and alerts

While the score is on, the server recomputes it every 15 minutes and stores it
as `dumbmonit_security_score{target="<id>"}` (and the number of failing checks
as `dumbmonit_security_checks_failed`). Two built-in rules use it: **Security
score dropped** (10 points or more below the best value of the last 24 hours)
and **Security score low** (below 40). See
[Alert rules](../alerting/rules.md#security-score).

## Checks and how to improve them

### Linux hosts

Machines with the agent. SSH and firewall checks need an agent from the same
release as the server; with an older agent they are unknown.

| Check | Severity | Data |
|---|---|---|
| Security updates installed | high | pending security updates (where the package manager tells them apart) |
| No pending updates | low | pending updates |
| No reboot pending | medium | reboot required |
| SSH root login with a password refused | high | `PermitRootLogin` from `sshd -T` (`prohibit-password` passes) |
| SSH accepts keys only | medium | `PasswordAuthentication`, and `KbdInteractiveAuthentication` with `UsePAM` |
| Host firewall filters inbound traffic | high | an nftables or iptables input chain that drops or rejects |
| SELinux enforcing | medium | SELinux mode (unknown where SELinux is absent) |
| Container images up to date | medium | newer image in the registry (only with containers) |
| No container image older than 180 days | low | image age (only with containers) |

To improve it:

1. Install pending security updates, then turn on automatic security updates:
   `unattended-upgrades` on Debian and Ubuntu, `dnf-automatic` on Fedora and
   RHEL.
2. Reboot after kernel or library updates.
3. Check that your SSH key works, then set `PermitRootLogin no`,
   `PasswordAuthentication no` and `KbdInteractiveAuthentication no` in
   `/etc/ssh/sshd_config` and reload sshd.
4. Turn on a host firewall that drops unsolicited inbound traffic: `ufw enable`
   on Ubuntu, firewalld on Fedora and RHEL, an nftables ruleset elsewhere.
5. Keep SELinux enforcing where the distribution ships it; pull newer container
   images regularly.

Sources: [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Windows hosts

Machines with the Windows agent, from the same release as the server (read
from the registry; an older agent reports none of these).

| Check | Severity | Data |
|---|---|---|
| Windows Firewall on for every profile | high | domain, private and public profiles (group policy first) |
| No reboot pending | medium | Windows Update or component servicing waiting for a restart |
| Remote Desktop requires Network Level Authentication | high | passes when Remote Desktop is off |
| SMBv1 disabled | high | SMBv1 server setting |

To improve it:

1. Install Windows updates (Settings → Windows Update) and restart when asked.
2. Keep Windows Defender Firewall on for the domain, private and public profiles.
3. If Remote Desktop is on, require Network Level Authentication and reach it
   through a VPN, never straight from the internet.
4. Remove SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Sources: [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE

| Check | Severity | Data |
|---|---|---|
| Security updates installed | high | pending security updates per node (needs `Sys.Modify` on the token, otherwise unknown) |
| Proxmox VE packages up to date | medium | upgradable Proxmox packages per node |
| Running the installed kernel | medium | newer kernel installed than running |
| No test repository in use | medium | enabled standard repositories |
| Package repositories correctly configured | medium | errors and warnings Proxmox VE raises on the repositories |
| Enterprise repository subscription | low | subscription status per node |
| Web interface certificate valid for 14+ days | medium | node certificate expiry |
| Every guest is in a backup job | high | guests not selected by any backup job |
| Backup jobs succeed | high | last result of each backup job |

To improve it:

1. Use a repository you are entitled to: enterprise with a subscription,
   otherwise `pve-no-subscription`; never `pvetest` on a node that matters.
2. Install updates from the node's **Updates** panel, then reboot into the new
   kernel.
3. Put every VM and container in a backup job, ideally to a Proxmox Backup
   Server, and fix any job that fails.
4. Replace the self-signed web certificate with an ACME one.

Sources: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server

| Check | Severity | Data |
|---|---|---|
| System updates installed | high | pending updates (needs `Sys.Modify` on `/system`) |
| Running the installed Backup Server version | medium | daemons still running the pre-upgrade version |
| Verify job scheduled | high | number of verify jobs |
| Prune job scheduled | medium | number of prune jobs |
| Backups copied elsewhere (sync or tape) | medium | sync and tape backup jobs (3-2-1 rule) |
| Garbage collection clean, no bad chunks | high | last GC result and bad chunks per datastore |
| Certificate valid for 14+ days | medium | certificate expiry |

To improve it:

1. Install updates and reboot when the kernel or `proxmox-backup` changed.
2. Schedule a verify job and a prune job on every datastore.
3. Keep a copy elsewhere: a sync job to a second Backup Server, or tape.
4. Use an ACME certificate for the web interface.

Sources: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html).

### Synology

| Check | Severity | Data |
|---|---|---|
| DSM 7 or later | high | DSM version |
| DSM up to date | high | a DSM update is available (administrator account only) |
| Built-in admin account disabled | medium | state of the `admin` account (administrator account only) |
| Auto Block enabled | medium | Auto Block setting (administrator account only) |
| DSM firewall enabled | medium | firewall setting (administrator account only) |
| Hyper Backup task configured | medium | number of Hyper Backup tasks |
| Hyper Backup tasks succeed | high | last result of each task |

The four settings are read once an hour and only when DumbMonit signs in with
an administrator account; with an ordinary account they stay unknown and do
not count. Two-step verification and the HTTPS redirection are not read.

To improve it:

1. Run the latest DSM 7 release for your model and let DSM install important
   updates automatically (Control Panel → Update & Restore).
2. Disable the built-in `admin` account; sign in with a personal administrator
   protected by 2-step verification.
3. Turn on Auto Block and the firewall (Control Panel → Security), and do not
   forward DSM ports to the internet: use a VPN.
4. Back up to another device or a cloud destination with Hyper Backup, and fix
   any task that fails: RAID and snapshots are not backups.

Sources: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense

| Check | Severity | Data |
|---|---|---|
| Firmware updates installed | high | pending package updates (unknown until OPNsense has checked for updates) |
| On the current major release series | medium | new major series available |
| No reboot pending after an update | medium | reboot required |
| Packet filter enabled | **critical** | `pf` status |

To improve it:

1. Check for updates in System → Firmware → Status, install them, reboot when
   asked.
2. Move to the new major release once it is out.
3. Keep the packet filter on.
4. Allow the web GUI and SSH only from the LAN or a VPN, never from the WAN.

Sources: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Websites

For **HTTP(S)** and **TLS** monitors that negotiate TLS, and for **Website
health & SEO** devices. A plain-HTTP monitor has no score.

| Check | Severity | Data |
|---|---|---|
| Certificate valid for 14+ days | high | days to expiry (HTTP(S) and TLS monitors) |
| Certificate chain trusted | high | chain verification (HTTP(S) and TLS monitors) |
| TLS 1.3 negotiated | low | negotiated protocol version (HTTP(S) and TLS monitors) |
| Served over HTTPS | high | final address after redirects (Website health) |
| Strict-Transport-Security header | medium | header with a `max-age` above 0 |
| X-Content-Type-Options: nosniff | low | header value |
| Protected against clickjacking | low | `X-Frame-Options` or CSP `frame-ancestors` |
| Content-Security-Policy header | low | an enforced policy (report-only does not count) |

Headers are read on HTTPS responses only.

To improve it:

1. Serve the site over HTTPS only, and redirect plain HTTP to HTTPS.
2. Automate certificate renewal (ACME, e.g. Let's Encrypt) and serve the full
   chain.
3. Add security headers in the web server or reverse proxy:
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff`, and a
   `Content-Security-Policy` with `frame-ancestors`.
4. Cross-check with [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory),
   which tests far more than this score.

Sources: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory

Scored from the PingCastle-style findings of the Active Directory collector:
Kerberoastable accounts, accounts without Kerberos pre-authentication,
unconstrained delegation, `krbtgt` password age, privileged group size, LAPS
coverage, password policy, stale accounts. Each finding is a check that passes
when it concerns zero objects.

To improve it:

1. Fix critical and high findings first: they are the usual paths to Domain
   Admins.
2. If the `krbtgt` password is old, reset it twice, a few hours apart, and
   remove unconstrained delegation from computers that are not domain
   controllers.
3. Move services that run under user accounts with SPNs to group managed
   service accounts (gMSA).
4. Run [PingCastle](https://www.pingcastle.com/) for a complete assessment.

Sources: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Limits

- The score only reflects what DumbMonit can see; many best practices are not
  checked because no collector reads the data: Proxmox firewall and two-factor
  authentication, OPNsense web GUI exposed on the WAN, Docker containers
  running privileged or with the Docker socket mounted, TLS 1.0/1.1 still
  offered (the probe only negotiates TLS 1.2 and 1.3), and many more.
- A check is only as fresh as its data: some collectors re-read updates every
  few hours.
- The weights are a product choice, the same for every installation.
- Only the kinds above are rated. Others (switches, printers, UniFi,
  pfSense…) show no badge on purpose: a couple of checks would give a grade
  that means nothing.
