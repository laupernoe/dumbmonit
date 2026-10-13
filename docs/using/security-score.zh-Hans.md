# 安全分数 {#security-score}

每个支持的设备获得一个**安全分数**从 0 到 100 和一个等级从 **A** 到 **F**，本着 PingCastle 或 Microsoft Secure Score 的精神：从供应商自己的最佳实践指导中抽取的检查列表，每一个通过、失败或未评估。

分数作为设备页面上的 **Security** 卡片出现，以及 **Security** 页面，列出每个被评级的设备，最差优先。

## 分数如何计算 {#how-the-score-is-computed}

DumbMonit 从不问设备任何额外的东西来计算分数。检查读取收集器已记录的最后值（回溯 6 小时），所以分数对设备没有成本，在最后一个探针离线时有效。

每个检查有：

| Field | Meaning |
|---|---|
| Category | exposure、patching、authentication、encryption、backup 或 configuration |
| Severity | critical、high、medium 或 low — 其在分数中的权重：10、6、3、1 |
| Result | **pass**、**fail** 或 **unknown** 当数据未被收集时 |
| Evidence | 观察到的内容：`pve1: 3 security updates`、`Expires in 9 days`… |
| Remediation | 做什么，用供应商的术语 |
| Vendor guidance | 检查所基于的供应商（或标准）页面 |

**Score** = 100 × (passing checks 的权重) ÷ (passing + failing checks 的权重)。**未知检查显示但从不计数**，既不赞成也不反对：分数不能因为监控令牌缺少权限而下降，也不能因为一无所知而上升。所有检查都是 unknown 的设备是*未评级*。

| Grade | Score | Word |
|---|---|---|
| A | 90–100 | Strong |
| B | 75–89 | Good |
| C | 60–74 | Fair |
| D | 40–59 | Weak |
| F | 0–39 | Poor |

**失败的关键检查上限等级为 C**，无论分数如何：不再过滤的防火墙，或主动开发的未打补丁的固件，无法由十个次要好设置补偿。卡片在发生时说。

## 历史和告警 {#history-and-alerts}

服务器每 15 分钟重新计算每个分数并将其存储为 `dumbmonit_security_score{target="<id>"}` 和（failing 检查数为 `dumbmonit_security_checks_failed`），所以您可以像任何其他度量一样绘制并对其报警。两个内置规则使用它：

- **Security score dropped** — 分数与最后 24 小时的最佳值相比下降了 10 个点或更多：有新东西在失败。
- **Security score low** — 分数低于 40（等级 F）。编辑规则将阈值移动到您自己的条。

请参阅 [Alert rules](../alerting/rules.md#security-score)。

## 按类别检查 {#checks-per-kind}

### Proxmox VE {#proxmox-ve}

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

来源：[System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates)、
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories)、
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management)、
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore)。

### Proxmox Backup Server {#proxmox-backup-server}

| Check | Severity | Data |
|---|---|---|
| System updates installed | high | pending updates (needs `Sys.Modify` on `/system`) |
| Running the installed Backup Server version | medium | daemons still running the pre-upgrade version |
| Verify job scheduled | high | number of verify jobs |
| Prune job scheduled | medium | number of prune jobs |
| Backups copied elsewhere (sync or tape) | medium | sync and tape backup jobs (3-2-1 rule) |
| Garbage collection clean, no bad chunks | high | last GC result and bad chunks per datastore |
| Certificate valid for 14+ days | medium | certificate expiry |

来源：[Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html)、
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html)、
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html)、
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html)。

### Synology DSM {#synology-dsm}

| Check | Severity | Data |
|---|---|---|
| DSM 7 or later | high | DSM version |
| Hyper Backup task configured | medium | number of Hyper Backup tasks |
| Hyper Backup tasks succeed | high | last result of each task |

来源：[Product support status](https://www.synology.com/en-global/products/status)、
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS)。

### OPNsense 和 pfSense {#opnsense-and-pfsense}

| Check | Severity | Data |
|---|---|---|
| OPNsense: firmware updates installed | high | pending package updates (unknown until OPNsense has checked for updates) |
| OPNsense: on the current major release series | medium | new major series available |
| OPNsense: no reboot pending after an update | medium | reboot required |
| OPNsense: packet filter enabled | **critical** | `pf` status |
| pfSense: REST API package up to date | low | REST API package update available |

来源：[OPNsense updates](https://docs.opnsense.org/manual/updates.html)、
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html)、
[pfSense REST API](https://pfrest.org/)。

### UniFi {#unifi}

| Check | Severity | Data |
|---|---|---|
| Device firmware up to date | high | upgradable flag of each adopted device |

来源：[UniFi releases](https://community.ui.com/releases)。

### TLS 和 HTTPS 监测 {#tls-and-https-monitors}

对于 `tls`、`http`、`smtp`、`mqtt` 和 `websocket` 协商 TLS 的监测。纯文本监测没有 Security 卡片。

| Check | Severity | Data |
|---|---|---|
| Certificate valid for 14+ days | high | days to expiry |
| Certificate chain trusted | high | chain verification |
| TLS 1.3 negotiated | low | negotiated protocol version |

来源：[Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS)、
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final)、
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/)。

### NTP 时间服务器 {#ntp-time-servers}

| Check | Severity | Data |
|---|---|---|
| Server reports a synchronized clock | medium | stratum and leap indicator |
| Clock offset within 100 ms | low | measured offset |

stratum 为 16（或 0，一个"kiss-o'-death"）和一个"unsynchronized"leap 指示符都失败第一个检查，无论偏移：从不同步的服务器没有可信任的东西提供。100 毫秒阈值固定在此，不跟随每设备"Tolerated offset"选项，相同方式证书检查总是在十四天时警告，无论监测自己的设置。

来源：[RFC 5905](https://www.rfc-editor.org/rfc/rfc5905)。

### Nginx 和 Apache httpd {#nginx-and-apache-httpd}

| Check | Severity | Data |
|---|---|---|
| Server header does not disclose the version | low | the `Server` response header captured alongside the status page |

仍然命名产品（"nginx"、"Apache"）但没有版本号的标头通过：这是 `server_tokens off` 和 `ServerTokens Prod` 产生的，缺失标头也通过。一个标头如"nginx/1.25.3"或"Apache/2.4.58 (Unix)"失败：它告诉扫描器哪些 CVE 精确要尝试。

来源：[Nginx core module reference](https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens)、
[Apache core module reference](https://httpd.apache.org/docs/2.4/mod/core.html#servertokens)。

### 有代理的机器（及其 Docker 容器） {#machines-with-the-agent-and-their-docker-containers}

| Check | Severity | Data |
|---|---|---|
| Security updates installed | high | pending security updates (where the package manager tells them apart) |
| No pending updates | low | pending updates |
| No reboot pending | medium | reboot required |
| SELinux enforcing | medium | SELinux mode (unknown where SELinux is absent) |
| Container images up to date | medium | newer image in the registry (only on machines with containers) |
| No container image older than 180 days | low | image age (only on machines with containers) |

来源：[CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management)、
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index)、
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often)。

### 其他集成的供应商更新 {#vendor-updates-for-other-integrations}

每个产品一个补丁检查，来自每个收集器已读的更新信息：

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

### Active Directory {#active-directory}

一个 Active Directory 设备从其收集器的 PingCastle 风格的发现中被评分：Kerberoastable 账户、没有 Kerberos 预身份验证的账户、无约束委派、`krbtgt` 密码年龄、特权组大小、LAPS 覆盖、密码策略、陈旧账户。每个发现是一个在关心零对象时通过的检查；收集器报告的严重等级是使用的那个。尚未报告的发现列为 unknown。

来源：[PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html)、
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos)、
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview)、
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password)。

### GitLab {#gitlab}

从管理 API 读取，使用管理员的个人访问令牌；使用普通令牌时 unknown。

| Check | Severity | Data |
|---|---|---|
| Two-factor authentication required | high | `require_two_factor_authentication` setting |
| Public sign-up disabled | medium | `signup_enabled` setting |
| Database migrations applied | medium | pending migrations after an upgrade |

来源：[GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html)。

### Forgejo 和 Gitea {#forgejo-and-gitea}

| Check | Severity | Data |
|---|---|---|
| Scheduled tasks run on time | medium | tasks whose next run is already overdue |

未知未有网站管理员的访问令牌：Forgejo 和 Gitea 保留计划任务列表给管理。

来源：[Forgejo API](https://forgejo.org/docs/latest/user/api/)。

## 限制 {#limits}

分数仅反映 DumbMonit 能看到的。它是看的提示，不是审计，等级 A 不意味着设备安全。

**未检查的**最佳实践因为收集器今天不读数据：

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

其他限制：

- 一个检查仅与其数据一样新鲜：一些收集器每几小时重新读更新或订阅。
- 权重是产品选择，对每个安装相同。
- 无任何检查的类别没有 Security 卡片，也不出现在 Security 页面上。
