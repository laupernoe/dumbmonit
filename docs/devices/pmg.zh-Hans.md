# Proxmox 邮件网关 {#proxmox-mail-gateway}

邮件网关：Postfix 队列和最早等待消息的年龄、今天统计和过滤的邮件、隔离区大小、防病毒和反垃圾邮件签名数据库的年龄、服务、证书和集群状态。

!!! warning "仅在测试实例上验证"

    该集成是针对一个为此目的安装的 Proxmox 邮件网关 9 进行构建和验证的，而不是针对生产环境的。每个调用都是读操作，形状来自真实邮件网关，但尚未有人在活跃邮件网关上运行它数周。请告诉我们什么地方出了问题。

## 监控项页面 {#the-device-page}

除了通用图表外，Proxmox 邮件网关监控项显示三个面板，从探针存储的内容读取 — 打开页面从不查询网关本身：

* **邮件队列** — 四个 Postfix 队列，按邮件管理员阅读的顺序：*incoming*（刚接受）、*active*（正在传递）、*deferred*（传递失败，将重试）和 *hold*（被规则保留）。每个显示它保留的消息数、目标域数、其中最繁忙的域以及最早消息等待多长时间。只是包含邮件的队列是正常流量；持有超过四小时消息的队列被标记为 *Stuck*。
* **今天过滤的邮件** — 进出消息、字节、垃圾邮件、病毒、退回、平均处理时间以及在传递前被拒绝的内容（灰名单、黑名单、SPF、预问候）。最近窗口中每个切片的条形图绘制流量曲线，无需查询时间序列数据库。然后是三个隔离区，包含它们的消息计数和磁盘使用情况，以及今天捕获的病毒按名称分类。
* **网关** — 首先是已停止的服务，然后是集群成员及其同步状态，然后是每个节点：正常运行时间、CPU、内存、磁盘、时钟偏移、签名数据库及其年龄和结论、在两周内到期的证书、待处理更新和订阅状态。

仅计数，绝不计内容。DumbMonit 读取每个隔离区中有多少消息，绝不是主题、发件人、收件人或正文。每地址统计端点（`/statistics/sender`、`/statistics/receiver`、`/statistics/contact`）根本不被调用。

## 今天，不是最后二十四小时 {#today-not-the-last-twenty-four-hours}

`GET /statistics/mail` 按**网关上的本地日期**聚合：PMG 忽略您要求的窗口结束，仅 `starttime` 选择日期。DumbMonit 因此要求"现在"，它给出自午夜以来的总计 — 正是 PMG 自己的仪表板显示的内容。这些系列是仪表，在午夜时回到零，而不是计数器：不要将它们包裹在 `rate()` 中。最近的曲线（`/statistics/recent`）改为读取原始表，真的会滚动。

## 它监视什么 {#what-it-watches}

所有指标都以 `dumbmonit_pmg_` 为前缀。

| 家族 | 指标 | 标签 |
|---|---|---|
| 节点 | `node_cpu_percent`、`node_iowait_percent`、`node_cpu_count`、`node_load1/5/15`、`node_memory_used/total_bytes`、`node_memory_used_percent`、`node_swap_used/total_bytes`、`node_rootfs_used/total/avail_bytes`、`node_rootfs_percent`、`node_uptime_seconds`、`node_clock_offset_seconds`（当网关领先 DumbMonit 时为正）、`node_insync`、`node_kernel_info`、`node_version_info`（此节点的 API 版本 — 在集群中，被上次升级遗留的节点显示在这里）、`version_info` | `node` |
| 队列 | `queue_messages`、`queue_domains`、`queue_oldest_age_seconds` | `queue`（`incoming`、`active`、`deferred`、`hold`） |
| 今天的邮件 | `mail_count_in/out`、`mail_bytes_in/out`、`mail_spam_in/out`、`mail_virus_in/out`、`mail_bounces_in/out`、`mail_junk_in`、`mail_junk_out`（您自己的用户发送垃圾邮件：被入侵账户的第一个迹象）、`mail_junk_percent`（在没有传入邮件的日期中缺失）、`mail_greylisted`、`mail_spf_rejects`、`mail_rbl_rejects`、`mail_pregreet_rejects`、`mail_avg_processing_seconds` | |
| 吞吐量 | `mail_rate_in_per_minute`、`mail_rate_out_per_minute`（来自最近曲线的最后完整切片） | |
| 垃圾邮件分数 | `spam_score_messages` | `level`（`0` 到 `10`；`10` 聚合所有以上内容） |
| 病毒 | `virus_detections`（今天前十） | `virus` |
| 隔离区 | `quarantine_messages`、`quarantine_bytes`、`quarantine_avg_spam_level` | `kind`（`spam`、`virus`、`attachment`） |
| 服务 | `service_running`（1 或 0；对于未安装的单元根本没有系列） | `node`、`service` |
| 签名 | `signature_age_seconds`、`signature_count`、`signature_update_available` | `node`、`family`（`virus`、`spam`）、`database`（`main`、`daily`、`bytecode` 或 SpamAssassin 频道） |
| 证书 | `certificate_expires_in_seconds` | `node`、`certificate` |
| 集群 | `cluster_nodes`、`cluster_node_insync`、`cluster_node_healthy`（在独立网关上全部缺失） | `node`、`role` |
| 订阅 | `subscription_active`（标签携带确切状态） | `node`、`status`、`level` |
| 更新 | `node_updates_pending`、`node_updates_security_pending` | `node` |
| 收集 | `up`、`scrape_errors`、`scrape_duration_seconds` | |

`queue_oldest_age_seconds` 是**下界**。`qshape` 报告年龄括号（少于 5 分钟、5-10 分钟、10-20 分钟等，加倍），所以 DumbMonit 保持最高占用括号的下限：报告年龄为"至少 20 分钟"的消息可能已 39 分钟。低估意味着警报晚于早期触发，从不是在消息实际上不迟到时。

`queue_domains` 在单个网关上是精确的。在集群上，按节点队列合并，计数取自每个节点最繁忙域，限制为每个节点二十个：超过那个，它不足计数。在这里，去重胜于求和，因为集群的两个节点为相同域接收邮件。

独立网关用空列表答复 `/config/cluster/status`。这是没有集群的网关，不是降级集群：没有发布系列，没有规则可以触发。

仅 ClamAV 的**日常**数据库按其年龄进行判断，由面板和规则进行判断。`main` 一年重建一两次，`bytecode` 几乎不更频繁：按两天对其进行判断会在完全当前的网关上打印"过期"整年。它们仍然显示它们的年龄、版本和签名计数 — 没有结论。同样，从未获取的 SpamAssassin 频道根本没有日期，"从未更新"不是"过期"：没有系列、没有结论、没有警报。

ClamAV 在 `.cvd` 标头内以其自己的格式写入其构建日期（`16 Dec 2025 23-18 +0000`，其他所有地方使用冒号的地方用破折号），PMG 通过。如果该格式曾经更改，年龄根本不存在，而不是错误的五十年。同样的规则适用于任何地方：API 不返回的值产生一个较少的指标，绝不是假装是测量的零。

应用的内置规则：监控项无法访问、邮件队列增长、邮件卡在队列中、邮件网关服务停止、病毒签名过期、垃圾邮件规则过期、隔离区填满、邮件网关集群降级、邮件网关证书过期、邮件网关更新待处理。通知命名相关的队列、服务、数据库或节点。

## 在 Proxmox 邮件网关中准备什么 {#what-to-prepare-in-proxmox-mail-gateway}

以下步骤是表单旁边的通知显示的步骤。原则：为监视保留的用户，具有只读角色 — 从不是您登录的账户。

1. 在 Web 界面中：Configuration → User Management → Users → Add。按如下方式命名账户，给它一个长密码，在其他地方不使用，并勾选 Enabled。

    ```
    dumbmonit@pmg
    ```

2. 将其角色设置为 `Audit`。那是 DumbMonit 读取的一切的精确只读最小值：节点状态、服务、postfix 队列、邮件统计、隔离计数、ClamAV 和 SpamAssassin 数据库年龄、集群状态、证书、订阅和待处理更新。`Audit` 不能更改任何内容、从隔离区释放任何内容或读取任何消息。

3. 更喜欢 shell？同一账户在一个命令中，然后设置密码。

    ```
    pmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment "DumbMonit monitoring"
    ```

4. 在 DumbMonit 中，输入网关地址，例如"mail.lan"或"mail.lan:8006"，然后输入此账户的用户名（包括域）和密码。

5. Proxmox 邮件网关不像 Proxmox VE 和 Proxmox 备份服务器那样发出 API 令牌：用户名和密码是方式。DumbMonit 打开一个两小时的会话并更新它，而不是在每次测量时登录。

!!! warning

    不要重用您登录的账户：上面的 Audit 角色只能读取，不能从隔离区释放消息或更改规则。Proxmox 邮件网关也默认使用自签名证书：如果因此拒绝连接，请在选项中勾选"接受不可验证的证书"。

地址：`10.0.0.40`、`mail.lan`、`mail.lan:8006`、`[fd00::4]` 或完整 URL。如果缺少，`https` 方案和端口 8006 被添加。

## 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `port` | API 端口 | `8006` | 当地址没有给出端口时使用。 |
| `insecure_tls` | 接受不可验证的证书 | `false` | Proxmox 邮件网关默认带有自签名证书：如果因此拒绝连接，请启用此选项。 |
| `request_timeout_seconds` | 每个请求的超时（秒） | `15` | 为每个 API 调用分配的时间，从 1 到 120。读取队列在网关上运行 Postfix 命令，可能需要几秒钟。 |
| `node` | 监视的节点 | *（空）* | 要监视的集群节点的名称。空：网关列出的每个节点。 |
| `recent_hours` | 流量窗口（小时） | `12` | 流量曲线回溯多远，从 1 到 24。每日总计总是覆盖当前日期。 |
| `queues` | 观察邮件队列 | `true` | 读取进入、活动、延迟和保留队列以及每个中最早消息的年龄。 |
| `quarantine` | 计算隔离区 | `true` | 读取垃圾邮件和病毒隔离区持有的消息数以及它们使用的磁盘空间。仅计数。 |
| `attachment_quarantine` | 也计算附件隔离区 | `false` | 该隔离区没有计数调用：必须列出才能计数。默认关闭；仅保留条目数。 |
| `signatures` | 观察签名数据库 | `true` | 读取 ClamAV 病毒数据库和 SpamAssassin 规则频道的年龄。 |
| `services` | 观察服务 | `true` | 读取每个节点的 postfix、pmg-smtp-filter、pmgpolicy 和其他单元的状态。 |
| `certificates` | 观察证书 | `true` | 读取 Web 界面提供的证书，并在它们过期前发出警告。 |
| `updates` | 计算待处理更新 | `true` | 列出每个节点上等待更新的包。 |
| `subscription` | 读取订阅 | `true` | 读取每个节点的订阅状态。 |

## 常见错误 {#common-errors}

| 症状 | 可能原因 |
|---|---|
| ClamAV `main` 显示巨大年龄 | 预期：它一年重建一两次。仅 `daily` 附带结论。 |
| 证书错误 | 自签名证书：安装受信任的证书（ACME 内置于 PMG 中）或启用 `insecure_tls`。 |
| 认证错误 | 用户名中的域错误 — 必须读取 `dumbmonit@pmg`，而不是 `dumbmonit` — 或账户未启用。仅 `GET /version` 失败不利于整个探针；任何其他调用失败被计入 `scrape_errors`。 |
| 权限不足 | 账户的角色低于 `Audit`。`quser` 只看到自己的邮件；`helpdesk` 不能读取节点或统计。 |
| 每个 `mail_*` 指标都是零 | 网关自午夜以来确实没有过滤任何东西。总计覆盖网关上的当前本地日期，并在其午夜重置，而不是您的午夜。 |
| 根本没有 `service_running` 系列 | `services` 选项关闭，或角色不能列出它们。列表中缺少的单个单元只是在该节点上未安装（例如，在使用 `systemd-timesyncd` 的机器上的 `chrony`），并故意不报告为已停止。 |
| 没有 `signature_age_seconds` 对于 SpamAssassin 频道 | 该频道从未更新过，因此 PMG 不返回日期。在网关上运行 `sa-update`；配置但从未获取的辅助频道故意保持未日期状态。 |
| 根本没有 ClamAV 数据库 | ClamAV 未安装或 `freshclam` 从未完成下载。端点用空列表答复，这不是错误。 |
| 没有集群指标 | 网关独立。`/config/cluster/status` 答复空列表，没有集群规则可以触发。 |
| "邮件卡在队列中"触发较晚 | `queue_oldest_age_seconds` 是 `qshape` 最高占用年龄括号的下限，所以它设计上不足报告。 |
| `quarantine_messages{kind="attachment"}` 缺失 | `attachment_quarantine` 选项关闭：该隔离区没有计数端点，所以计数意味着列出它。 |
