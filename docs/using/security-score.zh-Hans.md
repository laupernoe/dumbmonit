# 安全评分

安全评分是一个**可选**的 **A** 到 **F** 等级（以及 0 到 100 的分数），显示在那些有几项检查能提供有用信息的监控项旁边：装有代理的 Linux 和 Windows 主机、Proxmox VE 和 Proxmox Backup Server、Synology、OPNsense、网站和 Active Directory。其他监控项不显示任何内容。

!!! warning "仅供参考，不是审计"
    评分是**近似的，并不详尽**。它只读取 DumbMonit 已经从监控项收集到的数据，并与厂商的少数最佳实践进行比较。A 级并不意味着监控项是安全的，缺少某项检查也不意味着该设置没有问题。请把它当作提醒您去查看的线索，在重要的时候再使用真正的评估工具（PingCastle、Mozilla HTTP Observatory、Lynis、厂商自己的安全顾问……）。

## 启用

评分**默认关闭**。管理员在**设置 → 账户和安全 → 安全评分**中启用它。启用后：

- 每个被评分的监控项在监控项列表中都会获得一个带有其字母的小**盾牌徽章**，并在其页面上获得一个带有分数的 **Security** 徽章；
- 点击徽章会打开监控项的**报告**：哪些失败（附证据和修复方法）、哪些通过、在这类监控项上提高分数的步骤，以及指向下方相应章节的链接。

关闭后，徽章会隐藏，周期性计算停止，`dumbmonit_security_score` 序列也不再写入。

!!! note "从 0.1.0-alpha.8 或更早版本升级"
    评分过去始终开启，并有自己的 **Security** 页面。已经在监测可评分监控项的安装在升级后会保持评分开启；全新安装则默认关闭。旧地址 `/security` 现在会打开该设置。

## 评分如何计算

DumbMonit 在计算评分时从不向监控项额外询问任何内容。检查读取收集器已经记录的最新值（回溯 6 小时），因此评分不会给监控项带来任何开销。

每项检查都有类别（暴露面、补丁、身份验证、加密、备份、配置）、作为其权重的**严重程度**（critical 10、high 6、medium 3、low 1）、结果（**通过**、**失败**，或数据未被收集时的**未知**）、证据、修复方法，以及它所依据的厂商页面的链接。

**分数** = 100 × （通过检查的权重）÷ （通过 + 失败检查的权重）。**未知的检查会显示，但从不计入**：分数不应因为监控账户缺少某项权限而下降，也不应因为什么都不知道而上升。所有检查都未知的监控项*不评分*。

| 等级 | 分数 | 评语 |
|---|---|---|
| A | 90–100 | 强 |
| B | 75–89 | 良好 |
| C | 60–74 | 一般 |
| D | 40–59 | 弱 |
| F | 0–39 | 差 |

**一项失败的关键检查会把等级限制在 C**，无论分数多少。

## 历史和告警

评分开启期间，服务器每 15 分钟重新计算一次，并将其存储为 `dumbmonit_security_score{target="<id>"}`（失败检查的数量存储为 `dumbmonit_security_checks_failed`）。两条内置规则使用它：**Security score dropped**（比过去 24 小时的最佳值低 10 分或更多）和 **Security score low**（低于 40）。参见[告警规则](../alerting/rules.md#security-score)。

## 检查项及改进方法

### Linux 主机 { #linux-hosts }

装有代理的机器。SSH 和防火墙检查需要与服务器同一版本的代理；使用较旧的代理时，这些检查为未知。

| 检查 | 严重程度 | 数据 |
|---|---|---|
| Security updates installed | high | 待安装的安全更新（在包管理器能区分的地方） |
| No pending updates | low | 待安装的更新 |
| No reboot pending | medium | 需要重启 |
| SSH root login with a password refused | high | 来自 `sshd -T` 的 `PermitRootLogin`（`prohibit-password` 通过） |
| SSH accepts keys only | medium | `PasswordAuthentication`，以及启用 `UsePAM` 时的 `KbdInteractiveAuthentication` |
| Host firewall filters inbound traffic | high | 丢弃或拒绝的 nftables 或 iptables input 链 |
| SELinux enforcing | medium | SELinux 模式（没有 SELinux 的地方为未知） |
| Container images up to date | medium | 镜像仓库中有更新的镜像（仅限有容器时） |
| No container image older than 180 days | low | 镜像存在时间（仅限有容器时） |

改进方法：

1. 安装待安装的安全更新，然后启用自动安全更新：Debian 和 Ubuntu 上用 `unattended-upgrades`，Fedora 和 RHEL 上用 `dnf-automatic`。
2. 在内核或库更新后重启。
3. 确认您的 SSH 密钥可用，然后在 `/etc/ssh/sshd_config` 中设置 `PermitRootLogin no`、`PasswordAuthentication no` 和 `KbdInteractiveAuthentication no`，并重新加载 sshd。
4. 启用会丢弃未经请求的入站流量的主机防火墙：Ubuntu 上用 `ufw enable`，Fedora 和 RHEL 上用 firewalld，其他系统用 nftables 规则集。
5. 在发行版提供 SELinux 的地方保持其为 enforcing；定期拉取更新的容器镜像。

来源：[CIS Controls](https://www.cisecurity.org/controls)、
[sshd_config(5)](https://man.openbsd.org/sshd_config)、
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index)。

### Windows 主机 { #windows-hosts }

装有 Windows 代理且与服务器同一版本的机器（从注册表读取；较旧的代理不会报告其中任何一项）。

| 检查 | 严重程度 | 数据 |
|---|---|---|
| Windows Firewall on for every profile | high | 域、专用和公用配置文件（组策略优先） |
| No reboot pending | medium | Windows 更新或组件维护正在等待重启 |
| Remote Desktop requires Network Level Authentication | high | 远程桌面关闭时通过 |
| SMBv1 disabled | high | SMBv1 服务器设置 |

改进方法：

1. 安装 Windows 更新（设置 → Windows 更新），并在提示时重启。
2. 为域、专用和公用配置文件保持 Windows Defender 防火墙开启。
3. 如果启用了远程桌面，请要求网络级别身份验证（NLA），并通过 VPN 访问，绝不要直接从互联网访问。
4. 移除 SMBv1：`Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`。

来源：[Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/)、
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3)。

### Proxmox VE { #proxmox-ve }

| 检查 | 严重程度 | 数据 |
|---|---|---|
| Security updates installed | high | 每个节点待安装的安全更新（令牌需要 `Sys.Modify`，否则为未知） |
| Proxmox VE packages up to date | medium | 每个节点可升级的 Proxmox 软件包 |
| Running the installed kernel | medium | 已安装的内核比正在运行的更新 |
| No test repository in use | medium | 已启用的标准软件源 |
| Package repositories correctly configured | medium | Proxmox VE 对软件源报告的错误和警告 |
| Enterprise repository subscription | low | 每个节点的订阅状态 |
| Web interface certificate valid for 14+ days | medium | 节点证书到期时间 |
| Every guest is in a backup job | high | 未被任何备份任务选中的客户机 |
| Backup jobs succeed | high | 每个备份任务的最近一次结果 |

改进方法：

1. 使用您有权使用的软件源：有订阅用 enterprise，否则用 `pve-no-subscription`；在重要节点上绝不使用 `pvetest`。
2. 在节点的 **Updates** 面板中安装更新，然后重启进入新内核。
3. 把每台虚拟机和容器都放入备份任务，最好备份到 Proxmox Backup Server，并修复任何失败的任务。
4. 用 ACME 证书替换自签名的网页证书。

来源：[System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates)、
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories)、
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore)。

### Proxmox Backup Server { #proxmox-backup-server }

| 检查 | 严重程度 | 数据 |
|---|---|---|
| System updates installed | high | 待安装的更新（需要 `/system` 上的 `Sys.Modify`） |
| Running the installed Backup Server version | medium | 仍在运行升级前版本的守护进程 |
| Verify job scheduled | high | 验证任务的数量 |
| Prune job scheduled | medium | 清理（prune）任务的数量 |
| Backups copied elsewhere (sync or tape) | medium | 同步和磁带备份任务（3-2-1 规则） |
| Garbage collection clean, no bad chunks | high | 每个数据存储最近一次 GC 结果和损坏的数据块 |
| Certificate valid for 14+ days | medium | 证书到期时间 |

改进方法：

1. 安装更新，并在内核或 `proxmox-backup` 变化时重启。
2. 在每个数据存储上安排一个验证任务和一个清理任务。
3. 在别处保留一份副本：同步到第二台 Backup Server 的任务，或磁带。
4. 为网页界面使用 ACME 证书。

来源：[Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html)、
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html)。

### Synology { #synology }

| 检查 | 严重程度 | 数据 |
|---|---|---|
| DSM 7 or later | high | DSM 版本 |
| DSM up to date | high | 有可用的 DSM 更新（仅限管理员账户） |
| Built-in admin account disabled | medium | `admin` 账户的状态（仅限管理员账户） |
| Auto Block enabled | medium | 自动封锁设置（仅限管理员账户） |
| DSM firewall enabled | medium | 防火墙设置（仅限管理员账户） |
| Hyper Backup task configured | medium | Hyper Backup 任务数量 |
| Hyper Backup tasks succeed | high | 每个任务的最近一次结果 |

这四项设置每小时读取一次，并且只在 DumbMonit 使用管理员账户登录时读取；使用普通账户时它们保持未知，不计入。两步验证和 HTTPS 重定向不会被读取。

改进方法：

1. 运行适用于您型号的最新 DSM 7 版本，并让 DSM 自动安装重要更新（控制面板 → 更新和还原）。
2. 禁用内置的 `admin` 账户；使用受两步验证保护的个人管理员账户登录。
3. 启用自动封锁和防火墙（控制面板 → 安全性），并且不要把 DSM 端口转发到互联网：请使用 VPN。
4. 用 Hyper Backup 备份到另一个监控项或云端目标，并修复任何失败的任务：RAID 和快照不是备份。

来源：[Product support status](https://www.synology.com/en-global/products/status)、
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS)。

### OPNsense { #opnsense }

| 检查 | 严重程度 | 数据 |
|---|---|---|
| Firmware updates installed | high | 待安装的软件包更新（在 OPNsense 检查更新之前为未知） |
| On the current major release series | medium | 有新的主版本系列可用 |
| No reboot pending after an update | medium | 需要重启 |
| Packet filter enabled | **critical** | `pf` 状态 |

改进方法：

1. 在 System → Firmware → Status 中检查更新，安装后在提示时重启。
2. 新的主版本发布后及时升级。
3. 保持数据包过滤器开启。
4. 只允许从 LAN 或 VPN 访问网页界面和 SSH，绝不要从 WAN 访问。

来源：[OPNsense updates](https://docs.opnsense.org/manual/updates.html)、
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html)。

### 网站 { #websites }

适用于协商 TLS 的 **HTTP(S)** 和 **TLS** 监测，以及 **Website health & SEO** 监控项。纯 HTTP 监测没有评分。

| 检查 | 严重程度 | 数据 |
|---|---|---|
| Certificate valid for 14+ days | high | 距到期的天数（HTTP(S) 和 TLS 监测） |
| Certificate chain trusted | high | 证书链验证（HTTP(S) 和 TLS 监测） |
| TLS 1.3 negotiated | low | 协商的协议版本（HTTP(S) 和 TLS 监测） |
| Served over HTTPS | high | 重定向后的最终地址（Website health） |
| Strict-Transport-Security header | medium | `max-age` 大于 0 的响应头 |
| X-Content-Type-Options: nosniff | low | 响应头的值 |
| Protected against clickjacking | low | `X-Frame-Options` 或 CSP 的 `frame-ancestors` |
| Content-Security-Policy header | low | 已强制执行的策略（仅报告模式不算） |

只在 HTTPS 响应上读取响应头。

改进方法：

1. 仅通过 HTTPS 提供网站，并把纯 HTTP 重定向到 HTTPS。
2. 自动续期证书（ACME，例如 Let's Encrypt），并提供完整的证书链。
3. 在 Web 服务器或反向代理中添加安全响应头：`Strict-Transport-Security`、`X-Content-Type-Options: nosniff`，以及带有 `frame-ancestors` 的 `Content-Security-Policy`。
4. 与 [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory) 交叉核对，它测试的内容远多于本评分。

来源：[Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS)、
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers)。

### Active Directory { #active-directory }

根据 Active Directory 收集器中 PingCastle 风格的发现项进行评分：可被 Kerberoast 的账户、没有 Kerberos 预身份验证的账户、非约束委派、`krbtgt` 密码的存在时间、特权组的规模、LAPS 覆盖率、密码策略、陈旧账户。每个发现项就是一项检查，当它涉及的对象为零时通过。

改进方法：

1. 先修复严重和高危的发现项：它们是通往 Domain Admins 的常见路径。
2. 如果 `krbtgt` 密码很旧，请间隔几小时重置两次，并从非域控制器的计算机上移除非约束委派。
3. 把以带 SPN 的用户账户运行的服务迁移到组托管服务账户（gMSA）。
4. 运行 [PingCastle](https://www.pingcastle.com/) 进行完整评估。

来源：[PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html)、
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview)。

## 限制

- 评分只反映 DumbMonit 能看到的内容；许多最佳实践未被检查，因为没有收集器读取相应数据：Proxmox 防火墙和双因素身份验证、暴露在 WAN 上的 OPNsense 网页界面、以特权模式或挂载 Docker 套接字运行的 Docker 容器、仍提供 TLS 1.0/1.1（探测只协商 TLS 1.2 和 1.3），等等。
- 检查的新鲜度取决于其数据：有些收集器每隔几小时才重新读取一次更新。
- 权重是产品层面的选择，对每个安装都相同。
- 只有上述类型会被评分。其他类型（交换机、打印机、UniFi、pfSense……）有意不显示徽章：寥寥几项检查给出的等级没有意义。
