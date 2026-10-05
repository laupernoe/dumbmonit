# Security score

Every supported device gets a **security score** from 0 to 100 and a grade
from **A** to **F**, in the spirit of PingCastle or Microsoft Secure Score:
a list of checks drawn from the vendor's own best-practice guidance, each one
passing, failing or not evaluated.

The score appears as a **Security** card on the device page, and on the
**Security** page, which lists every rated device, worst first.

## How the score is computed

DumbMonit never asks the device anything extra to compute the score. The
checks read the last values the collectors already recorded (looking back
6 hours), so a score costs nothing to the device and works offline from the
last probe.

Each check has:

| Field | Meaning |
|---|---|
| Category | exposure, patching, authentication, encryption, backup or configuration |
| Severity | critical, high, medium or low — its weight in the score: 10, 6, 3, 1 |
| Result | **pass**, **fail**, or **unknown** when the data is not collected |
| Evidence | what was observed: `pve1: 3 security updates`, `Expires in 9 days`… |
| Remediation | what to do, in the vendor's terms |
| Vendor guidance | the vendor (or standard) page the check is based on |

**Score** = 100 × (weight of passing checks) ÷ (weight of passing + failing
checks). **Unknown checks are shown but never counted**, neither for nor
against: a score must not drop because a monitoring token lacks a privilege,
nor rise because nothing is known. A device whose checks are all unknown is
*not rated*.

| Grade | Score | Word |
|---|---|---|
| A | 90–100 | Strong |
| B | 75–89 | Good |
| C | 60–74 | Fair |
| D | 40–59 | Weak |
| F | 0–39 | Poor |

**A failing critical check caps the grade at C**, whatever the score: a
firewall that no longer filters, or an actively exploited firmware left
unpatched, is not compensated by ten minor good settings. The card says so
when it happens.

## History and alerts

The server recomputes every score every 15 minutes and stores it as
`dumbmonit_security_score{target="<id>"}` (and the number of failing checks as
`dumbmonit_security_checks_failed`), so you can chart it and alert on it like
any other metric. Two built-in rules use it:

- **Security score dropped** — the score fell by 10 points or more compared
  with its best value of the last 24 hours: something new is failing.
- **Security score low** — the score is below 40 (grade F). Edit the rule to
  move the threshold to your own bar.

See [Alert rules](../alerting/rules.md#security-score).

## Checks per kind

### Proxmox VE

| Check | Severity | Data |
|---|---|---|
| Security updates installed | high | pending security updates per node (needs `Sys.Modify` on the token, otherwise unknown) |
| Proxmox VE packages up to date | medium | upgradable Proxmox packages per node |
| Running the installed kernel | medium | newer kernel installed than running |
| No test repository in use | medium | enabled standard repositories |
| Package repositories correctly configured | medium | errors and warnings Proxmox VE raises on the repositories (e.g. enterprise repository without subscription) |
| Enterprise repository subscription | low | subscription status per node |
| Web interface certificate valid for 14+ days | medium | node certificate expiry |
| Every guest is in a backup job | high | guests not selected by any backup job |
| Backup jobs succeed | high | last result of each backup job |

Sources: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management),
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

Sources: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html),
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html),
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html).

### Synology DSM

| Check | Severity | Data |
|---|---|---|
| DSM 7 or later | high | DSM version |
| Hyper Backup task configured | medium | number of Hyper Backup tasks |
| Hyper Backup tasks succeed | high | last result of each task |

Sources: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense and pfSense

| Check | Severity | Data |
|---|---|---|
| OPNsense: firmware updates installed | high | pending package updates (unknown until OPNsense has checked for updates) |
| OPNsense: on the current major release series | medium | new major series available |
| OPNsense: no reboot pending after an update | medium | reboot required |
| OPNsense: packet filter enabled | **critical** | `pf` status |
| pfSense: REST API package up to date | low | REST API package update available |

Sources: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html),
[pfSense REST API](https://pfrest.org/).

### UniFi

| Check | Severity | Data |
|---|---|---|
| Device firmware up to date | high | upgradable flag of each adopted device |

Source: [UniFi releases](https://community.ui.com/releases).

### TLS and HTTPS monitors

For `tls`, `http`, `smtp`, `mqtt` and `websocket` monitors that negotiate TLS.
A plain-text monitor has no Security card.

| Check | Severity | Data |
|---|---|---|
| Certificate valid for 14+ days | high | days to expiry |
| Certificate chain trusted | high | chain verification |
| TLS 1.3 negotiated | low | negotiated protocol version |

Sources: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final),
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/).

### Machines with the agent (and their Docker containers)

| Check | Severity | Data |
|---|---|---|
| Security updates installed | high | pending security updates (where the package manager tells them apart) |
| No pending updates | low | pending updates |
| No reboot pending | medium | reboot required |
| SELinux enforcing | medium | SELinux mode (unknown where SELinux is absent) |
| Container images up to date | medium | newer image in the registry (only on machines with containers) |
| No container image older than 180 days | low | image age (only on machines with containers) |

Sources: [CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index),
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often).

### Vendor updates for other integrations

One patching check per product, from the update information each collector
already reads:

| Kind | Check | Severity |
|---|---|---|
| FortiGate | FortiOS patch installed | **critical** |
| FortiGate | FortiGuard licences valid | high |
| MikroTik | RouterOS up to date | high |
| MikroTik | RouterBOOT firmware matches RouterOS | low |
| Home Assistant | Home Assistant components up to date | medium |
| Nextcloud | Nextcloud server up to date | high |
| Nextcloud | Nextcloud apps up to date | medium |
| AdGuard Home | AdGuard Home up to date | medium |
| Pi-hole | Pi-hole up to date | medium |
| Plex | Plex Media Server up to date | medium |
| Paperless-ngx | Paperless-ngx up to date | medium |
| Tailscale | Tailscale clients up to date | medium |

### Active Directory

An Active Directory device is scored from the PingCastle-style findings of its
collector: Kerberoastable accounts, accounts without Kerberos
pre-authentication, unconstrained delegation, `krbtgt` password age,
privileged group size, LAPS coverage, password policy, stale accounts. Each
finding is a check that passes when it concerns zero objects; the severity the
collector reports is the one used. Findings not reported yet are listed as
unknown.

Sources: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview),
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password).

### GitLab

Read from the administration API, with an administrator's personal access
token; unknown with an ordinary one.

| Check | Severity | Data |
|---|---|---|
| Two-factor authentication required | high | `require_two_factor_authentication` setting |
| Public sign-up disabled | medium | `signup_enabled` setting |
| Database migrations applied | medium | pending migrations after an upgrade |

Source: [GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html).

### Forgejo and Gitea

| Check | Severity | Data |
|---|---|---|
| Scheduled tasks run on time | medium | tasks whose next run is already overdue |

Unknown without a site administrator's access token: Forgejo and Gitea
reserve the scheduled-task list to administration.

Source: [Forgejo API](https://forgejo.org/docs/latest/user/api/).

## Limits

The score only reflects what DumbMonit can see. It is a prompt to look, not
an audit, and a grade A does not mean a device is secure.

Best practices that are **not** checked yet because the collectors do not
read the data today:

| Kind | Not checked (needs new data) |
|---|---|
| Proxmox VE | datacenter/node firewall enabled; two-factor authentication on `root@pam` and other admin accounts |
| Proxmox Backup Server | client-side encryption of backups (needs reading backup manifests) |
| Synology DSM | 2-step verification for administrators; default `admin` account disabled; auto block; DSM firewall; HTTP-to-HTTPS redirection; QuickConnect exposure; whether a newer DSM update is available |
| OPNsense / pfSense | web GUI or SSH reachable from the WAN; default credentials; pfSense system (not REST API) updates |
| UniFi | device SSH with default credentials; site settings |
| TLS monitors | TLS 1.0/1.1 or weak cipher suites still offered — the probe only negotiates TLS 1.2 and 1.3, so a server limited to older versions shows as a failed probe instead |
| Agent hosts | host firewall status; SSH configuration; Windows Defender status |
| Docker | containers running privileged, as root, or with the Docker socket mounted |

Other limits:

- A check is only as fresh as its data: some collectors re-read updates or
  subscriptions every few hours.
- The weights are a product choice, the same for every installation.
- Kinds without any check have no Security card and do not appear on the
  Security page.
