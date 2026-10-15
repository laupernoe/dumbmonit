# Synology DSM {#synology-dsm}

Synology NAS 通过 DSM Web API：卷、存储池、磁盘健康、温度、Hyper Backup 任务、Active Backup for Business 任务以及 Synology Drive 和 Photos。监视 NAS 的首要原因是其磁盘：升温或 SMART 状态降级的磁盘宣布长期失败。

## 它监视什么 {#what-it-watches}

所有指标都以 `dumbmonit_synology_` 为前缀。

| 家族 | 指标 | 标签 |
|---|---|---|
| 系统 | `system_info`（型号、DSM 版本、固件…）、`uptime_seconds`、`temperature_celsius`、`temperature_warning`、`system_crashed`、`system_need_repair`、`storage_health` | `model`、`dsm_version`、`firmware` |
| CPU 和内存 | `cpu_usage_percent`、`cpu_user/system/other_percent`、`cpu_cores`、`memory_total/used/available/installed_bytes`、`memory_usage_percent`、`swap_usage_percent` | |
| 网络 | `network_rx_bytes_per_second`、`network_tx_bytes_per_second` | `interface` |
| 卷 | `volume_status`、`volume_total/used/available_bytes`、`volume_used_percent`、`volume_used_warning_percent`、`volume_used_critical_percent` | `volume`、`fs_type`、`raid_type` |
| 存储池和 SSD 缓存 | `pool_status`、`pool_failed_disks`、`pool_total/used_bytes`，以及相同四个在 `ssd_cache_` 下 | `pool` 或 `ssd_cache`、`name`、`raid_type` |
| 磁盘 | `disk_status`、`disk_smart_status`、`disk_temperature_celsius`、`disk_size_bytes`、`disk_bad_sector_exceeded`、`disk_life_below_threshold`、`disk_remaining_life_percent`（SSD 仅：驱动说它有剩余的生命）、`disk_unc_count`（DSM 的不可读部门计数器，它检查反对其坏部门阈值）、`disk_info` | `disk`、`name`；在 `disk_info`：`model`、`serial`、`vendor`、`firmware`、`type`、`ssd` |
| Hyper Backup | `backup_tasks`、`backup_running`、`backup_last_result`、`backup_last_run_age_seconds`、`backup_last_success_age_seconds`、`backup_next_run_in_seconds` | 每个任务 |
| Synology Drive | `drive_up`、`drive_team_folders`、`drive_team_folder_info`、`drive_connections`（见下面） | `drive_team_folder_info`：`name` |
| Synology Photos | `photos_installed`（见下面） | |
| 安全设置 | `update_available`、`admin_account_enabled`、`autoblock_enabled`、`firewall_enabled`（`0`/`1`；每小时读取一次，仅限管理员账户，可选：DSM 拒绝时缺失） | |
| 收集 | `up`、`scrape_errors`、`scrape_duration_seconds` | |

Active Backup for Business 指标是例外：它们以 `dumbmonit_abb_` 为前缀（见下面）。

`volume_status`、`disk_status` 和 `disk_smart_status` 在标签（`status`、`smart_status`）和一个值在一个尺度中携带 DSM 自己的词：`0` 正常、`1` 注意（如 `repairing` 或 `checking` 的暂时状态，或 DumbMonit 尚不知道的 DSM 引入的词）、`2` 关键（`degrade`、`crashed`、`failed`…）。未知故意计为 `1`，从不 `0`。

部分失败保持成功探针：如果存储库存失败，系统状态和使用仍被发布，失败被计入 `scrape_errors`。仅在 API 发现调用或连接的失败谴责整个探针。

!!! note "磁盘行无法显示什么"
    重新分配部门计数活在每磁盘 SMART 属性表中，DSM 仅通过一个 DumbMonit 不能确认其形状的未文档化调用提供。行因此显示 `load_info` 给出的：DSM 的坏部门结论、其不可读部门计数器（及其增长），以及 SSD 的剩余生命。

!!! note "在 DSM 版本之间变更什么"
    同一调用不在每个 DSM 答复相同字段，DumbMonit 读两个形式而不是假设一个：

    * `disk_bad_sector_exceeded` 来自 `exceed_bad_sector_thr` 直到 DSM 7.1 并从 `sb_days_left_critical` — DSM 的天数估计在驱动达到其坏部门阈值之前 — 在 DSM 7.2 及更高版本上，第一个字段消失。同样对于 `disk_life_below_threshold` 和 `remain_life_danger`；
    * `disk_remaining_life_percent` 是一个纯数字在较旧 DSM 上，一个对象在 DSM 7.2 及更高版本上携带值和 DSM 是否信任它。DSM 标记为不可信的值不被发布；
    * `volume_used_warning_percent` 和 `volume_used_critical_percent` 重复在 DSM 中设置的阈值。DSM 7.4 不再返回它们，所以在最近的 NAS 那两个系列根本不存在 — 在它们的地方什么都不被发明。

## 设备页面 {#the-device-page}

![一个 Synology 设备：一目了然，卷、磁盘和 Active Backup](../assets/screenshots/device-synology-light.png){ loading=lazy }

一个 Synology 设备在图表上方获得它自己的面板：

* **一目了然** — CPU、内存（已用的总共）、温度和正常运行时间，带 DSM 自己的存储判决作为板（健康、注意、崩溃、修复需要）；
* **卷** — 每卷一行：使用条、已用的总共、RAID 类型和文件系统、卷在 DSM 的词中的状态（正常、降级、崩溃、修复…）；
* **存储池** — 每池一行：RAID 类型、池在 DSM 的词中的状态、DSM 报告为失败的多少磁盘、多少已交给卷。这是冗余活着的地方：一个失去磁盘的 RAID 5 在上方的卷在重建时保持 `normal`，所以卷行说什么和这一行说一切。一个完全分配给其卷的池是正常结束状态，不是警告，所以这个数字是一个句子而不是一个彩色仪表；
* **SSD 缓存** — 每缓存一行：其 RAID 类型、其状态和其大小。部分在没有缓存的 NAS 上缺失。DSM 的 `load_info` 携带无缓存方式或命中速率，所以两个都没显示；
* **磁盘** — 每湾一行：型号、大小、温度、SMART 结论、DSM 的坏部门结论、不可读部门计数器，以及在 SSD 上的剩余生命作为一个小条；
* **Active Backup for Business** — 任务，然后每个备份设备与其节奏和三十天条（见下面）。

一切来自探针存储的：打开页面从不查询 NAS。

### NAS 的内置规则 {#built-in-rules-for-the-nas}

| 规则 | 严重性 | 触发时 |
|---|---|---|
| Synology 磁盘 SMART 警告 | advisory | DSM 不再报告磁盘的 SMART 状态为 `normal`，持续 15 分钟。 |
| Synology 磁盘失败 | warning | DSM 报告磁盘为崩溃或失败，持续 5 分钟。 |
| Synology 磁盘坏部门 | warning | 坏部门计数超过 DSM 设置的阈值，持续 15 分钟。 |
| Synology 磁盘坏部门增长 | advisory | 新的不可读部门在最后 24 小时出现（`delta(…disk_unc_count[24h]) > 0`）。 |
| Synology SSD 磨损 | advisory | 一个 SSD 有少于 10% 其额定生命剩余，持续一小时。 |
| Synology 卷降级 | warning | 卷被降级或崩溃，持续 5 分钟。 |
| Synology 存储池降级 | warning | 存储池或 SSD 缓存被降级或崩溃，持续 5 分钟。一个失去磁盘的 RAID 5 首先显示这里：DSM 在上方的卷重建时保持在 `normal`。 |
| Synology 卷几乎满 | advisory | 卷是 90% 满或更多，持续 15 分钟。 |
| Synology 温度高 | advisory | 磁盘在 55°C 以上，或 DSM 升高其温度警告，持续 15 分钟。 |
| Synology 内存高 | advisory | 内存使用在 95% 以上，持续 15 分钟。 |

## Active Backup for Business {#active-backup-for-business}

Active Backup for Business（ABB）备份 PC、物理服务器、虚拟机和文件服务器到 NAS。如果包已安装并运行，DumbMonit 在每个探针读取其任务；如果不是，NAS 根本不宣布 API，什么都不被要求。选项 `abb` 关闭整个家族。

同样持有 Hyper Backup：DSM 仅在包运行时列出包的 API，所以一个 NAS 其 Hyper Backup 停止 — 或仅通过 Hyper Backup Vault 接收备份 — 发布无 `backup_*` 指标并报告无错误。DumbMonit 在调用任何之前问 NAS 它有哪些 API，所以缺失包是沉默，从不失败探针。

| 指标 | 值 | 标签 |
|---|---|---|
| `dumbmonit_abb_tasks` | 任务数。 | |
| `dumbmonit_abb_task_last_status` | 最后运行：`1` 成功、`0` 失败、`2` 运行、`-1` 未知（从不运行、取消、无备份）。*部分*成功计为失败：至少一个设备未被备份。 | `task`、`task_id`、`source_type`、`result` |
| `dumbmonit_abb_task_last_success_seconds` | 最后成功备份运行的年龄，秒。对于从不成功的任务缺失。 | `task`、`task_id`、`source_type` |
| `dumbmonit_abb_task_enabled` | `1` 如果任务有计划并未暂停，`0` 如果仅手工运行。 | `task`、`task_id`、`source_type` |
| `dumbmonit_abb_task_device_count` | 附加到任务的设备。 | `task`、`task_id`、`source_type` |

`task` 是如 ABB 显示的任务名；`source_type` 是 `pc`、`vm`、`physical_server`、`file_server` 或 `nas`；`result` 是最后运行的原始结果（`success`、`partial_success`、`fail`、`cancel`、`no_backup`、`running`、`none`）。

年龄取自任务历史过滤关于成功备份运行，不从任务的 "last result"：在一个每晚版本清除后，最后结果是清除，不备份。这花费每任务一个请求，限制在二十个任务。

在任务上的内置规则：

| 规则 | 严重性 | 触发时 |
|---|---|---|
| Active Backup 任务失败 | warning | 最后运行失败，或是部分成功，持续 10 分钟。通知命名任务。 |
| Active Backup 太旧 | advisory | 没有成功运行超过两天。 |
| Active Backup 任务禁用 | info | 任务无计划，或其连续备份暂停，持续一小时。 |

### 设备及其节奏 {#devices-and-their-rhythm}

任务级规则对服务器是对的，对笔记本是错的：人不留笔记本打开，"没有备份 24 小时"是每个办公室 PC 的周一早晨。DumbMonit 因此也监视 ABB **逐设备**，对抗设备自己的习惯。

在每个探针，收集器读 ABB 保留的每设备运行（`SYNO.ActiveBackup.Overview` `list_device_transfer_size`，一个调用对所有设备，仅从前一个探针改变的什么）并复制到它自己的数据库 — 每 NAS 限制到九十天 — 所以模型在不重读一个月历史的重启中存活。然后，对每设备，它看最后三十天。

| 指标 | 值 | 标签 |
|---|---|---|
| `dumbmonit_abb_device_state` | `0` ok、`1` idle、`2` learning、`3` running、`4` overdue、`5` failing、`6` never。 | `device`、`device_id`、`task`、`task_id`、`state` |
| `dumbmonit_abb_device_overdue` | `1` 当设备逾期时，`0` 否则。 | `device`、`device_id`、`task`、`task_id` |
| `dumbmonit_abb_device_consecutive_failures` | 失败尝试行排，因最后成功（一个取消运行计两个都不是）。 | 同 |
| `dumbmonit_abb_device_last_success_seconds` | 这个设备最后成功运行的年龄。如果从不成功缺失。 | 同 |
| `dumbmonit_abb_device_typical_interval_seconds` | 两个成功间的中间间隙，放假除外。学习时缺失。 | 同 |

#### 如何 "overdue" 被判断 {#how-overdue-is-judged}

用普通词，对每设备：

1. **哪些天它打开？** 在最后三十天中至少尝试一个备份的工作日 — 成功或不成功；一个失败证明机器启动尽可能成功 — 是一个*活跃日*。一旦有至少两周历史，一个根本无尝试的工作日是一个*放假*。一个周一到周五使用的笔记本获得周六和周日作为放假；服务器有无。
2. **它多经常备份？** 连续成功间的间隙被收集在*活跃时间*：一个间隙包含的完整放假不被计（周五晚到周一晚是一天活跃时间，不三个）。它们的中间是设备的*通常间隙*；它们的 90 百分位是 "缓慢周" 看起来是什么。
3. **它被允许多少？** 津贴是 `max(1.5 × p90 gap, 2 × typical interval, 36 h)`。每晚桌面获得 48 h；每三天备份的机器获得大约六个；没有人获得少于一天半。当较少于两个可用间隙存在时（这周添加的设备），津贴是 72 h — 足以跨越一个周末没有假警报。
4. **多少活跃时间已经过去？** 自最后成功以来的时间，减去它包含的完整放假。一个周五在 20:00 最后备份的笔记本有，在周一早晨 08:00，十二小时活跃时间后面它，不六十。
5. **判决**，按这个顺序：*running*（备份进行中）；*failing*（最后两个或更多尝试失败 — 部分成功是失败，取消运行两个都不是）；*never*（无成功记录）；*overdue*（活跃时间超过津贴）；*idle*（今天是其放假之一：放假，如通常）；*learning*（节奏尚不知道）；*ok*。

仅 *overdue* 升高 `dumbmonit_abb_device_overdue`，仅 *failing* 供给失败规则。一个周日闲置的笔记本升高什么；同一笔记本在周二晚仍沉默做。

天和小时以服务器的本地时间表示（容器的 `TZ`），这是读页面的人活着的地方。设备页面显示，每设备：状态在词中、最后成功和最后结果、词中的节奏（"工作日，大约 20:00"、"每 ~3 天，周一、周三、周五，大约 03:00"、"仍学习（30 天 2 次运行）"）以及三十天条 — 成功日为青绿色、失败为红色、取消运行为琥珀色、无运行为灰色。

在设备上的内置规则：

| 规则 | 严重性 | 触发时 |
|---|---|---|
| Active Backup 设备逾期 | advisory | `dumbmonit_abb_device_overdue > 0` 持续 30 分钟。 |
| Active Backup 设备失败 | warning | 两个或更多失败尝试行排，持续 10 分钟。 |

!!! warning "权限"
    ABB 仅回答 `administrators` 组中的账户，包在其应用权限中允许，或账户包被委派给（DSM 7：Active Backup for Business → Settings → Privileges）。账户被拒绝应用无法读 ABB 任务：调用被拒绝（代码 105）并计入 `scrape_errors`。允许或委派包到监视账户，或取消勾选 "Watch Active Backup for Business"（`abb = false`）停止要求。

!!! note "一个未文档化的 API"
    Synology 不文档 ABB Web API。DumbMonit 跟踪社区项目记录的形状（`SYNO.ActiveBackup.Task` `list`、`SYNO.ActiveBackup.Log` `list_result` 和 `SYNO.ActiveBackup.Overview` `list_device_transfer_size`，所有版本 1）并通过 `SYNO.API.Info` 发现它们的路径。一个答复不同的 DSM 产生缺失指标，从不失败探针；一个太旧提供每设备概览的 ABB 保持其任务指标并根本无设备节奏。

    版本 1 是故意的。DSM 7.4 宣布这些 API 作为版本 1 到 2，但仅版本 1 携带读方法：在版本 2 要求它们全部答复 "请求的方法不存在"，看起来像固件差异并不是。同样的答复也来当 CSRF 令牌缺失时，所以 DumbMonit 在登录时从 DSM 要求一个并在每个调用上发送它。

## Synology Drive {#synology-drive}

DSM 不文档 Synology Drive Web API。DumbMonit 读三个调用，其名称和形状来自社区库 `N4S4/synology-api`，从与 Synology Drive 管理控制台相同的屏幕调用，并跟踪与 Active Backup for Business 相同的规则：一个 NAS 不宣布的 API 是一个未安装的包，从不失败。

| 指标 | 值 | 标签 |
|---|---|---|
| `dumbmonit_synology_drive_up` | `1` 当服务答复 `get_status` 时。如果包未安装缺失。 | |
| `dumbmonit_synology_drive_team_folders` | 团队文件夹数。 | |
| `dumbmonit_synology_drive_team_folder_info` | 值 `1` 每团队文件夹（最多 32）。 | `name` |
| `dumbmonit_synology_drive_connections` | 活动连接数（桌面和移动客户端当前同步）。 | |

!!! note "为什么连接仅是计数"
    管理控制台的 "Client list" 页显示每个连接的设备名称、用户、应用类型、在线状态、IP 地址和位置，但那是页面的描述，不是 API 的 JSON 字段：Synology 都没有任何社区项目文档 `SYNO.SynologyDrive.Connection` 实际命名其字段。而不是猜测设备的名称或最后同步日期从一个未确认的键，DumbMonit 仅发布连接计数。个别设备将插入同一[客户设备](../alerting/rules.md)表用由 Immich、Proxmox 备份服务器和 Veeam 一旦那个形状从一个真实 NAS 的响应确认。

`drive` 选项（默认打开）关闭整个家族。

## Synology Photos {#synology-photos}

Synology Photos 运送一个良好文档的浏览 API，但 Synology 都没有任何社区库暴露一个管理员范围的移动备份设备列表或其最后备份 — 移动应用本地显示其自己的备份状态，什么都不集中化它在 NAS 侧通过一个 API。DumbMonit 因此仅报告包已安装：`dumbmonit_synology_photos_installed`（值 `1`），从 API 目录读取而不对 NAS 的任何额外请求 — 同一目录 `SYNO.API.Info` 已对每个其他包答复。`photos` 选项（默认打开）关闭它。

## 在 NAS 上准备什么 {#what-to-prepare-on-the-nas}

下面的步骤是表单旁的通知显示的步骤。原则：一个为监视保留的账户，从 DSM 的 API 不需要的一切剥离 — 从不是您登录的账户。

1. 在 DSM 中，打开 Control Panel → User & Group → User 并单击 Create。按如下方式命名账户，给它一个长密码，在其他地方不使用。

    ```
    dumbmonit
    ```

2. 加入组：勾选 administrators。DSM 仅回答存储、卷和磁盘 SMART 调用给那个组；没有它 NAS 显示活着但说它的磁盘什么。接下来两个步骤取回一切别的。
3. 分配共享文件夹权限：每个共享文件夹上无访问。分配应用权限：拒绝一切除了 DSM，加上 Active Backup for Business 如果您想读其任务。跳过配额和速度限制页。
4. 两步验证必须对这个账户保持关闭：没有自动监视器可以输入一次性代码。如果 Control Panel → Security → Account 强制它，限制规则到这个用户不在，或豁免它。
5. 备份包仅在被安装和运行时读：DSM 不宣布停止包的 API，所以 Hyper Backup 任务然后变静而不失败，一个仅接收备份的 NAS（Hyper Backup Vault）有自己要显示的无任务。Active Backup for Business 额外回答仅一个被允许使用它的账户（Active Backup for Business → Settings → Privileges）；否则取消勾选 DumbMonit 的 "Watch Active Backup for Business" 选项停止要求。
6. 在 DumbMonit 中，输入 NAS 地址（HTTPS，端口 5001 默认），然后这个账户的用户名和密码。

!!! warning
    administrators 组由 DSM 的存储 API 要求，不由 DumbMonit。那是为什么这个账户获得没有共享文件夹、没有应用和在其他地方不使用的密码：它可以读 NAS，不接触您的文件。

!!! note "哪些权限给哪些指标"
    DSM API 不需要相同权限对每个调用。系统状态（型号、版本、温度、正常运行时间）回答任何账户。CPU、内存、卷和尤其磁盘 SMART 状态要求 `administrators` 组。在 DSM 7 上，"System monitoring" 委派（Control Panel → User & Group → Delegation）对使用足够但不对存储库存。没有特定权限的账户因此给一个 NAS 那是 "活着"，但关于其磁盘什么。

供应商文档：[在 DSM 中创建用户](https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/file_user_create)。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| DSM 账户 | **User name**：上面创建的账户。**Password**：其密码。两步验证必须对这个账户关闭：没有自动监视器可以输入一次性代码。 |

地址：`192.168.1.30`、`nas.lan` 或 `nas.lan:5001`。没有端口，5001 在 HTTPS 和 5000 在 HTTP 上使用。

## 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `scheme` | 协议 | `https` | HTTPS 适合一个新出盒的 NAS。HTTP 仅如果 DSM 仅以明文监听。选择：`https`、`http`。 |
| `port` | DSM 端口 | *（空）* | 当地址没有给出一个时使用。空：HTTPS 5001，HTTP 5000。 |
| `insecure_tls` | 接受不可验证的证书 | `false` | 一个 NAS 默认带自签名证书：如果因此拒绝连接启用这个。 |
| `request_timeout_seconds` | 每个请求的超时（秒） | `15` | 为每个 API 调用分配的时间，从 1 到 120。存储库存可以唤醒睡眠磁盘。 |
| `abb` | 观察 Active Backup for Business | `true` | 读 Active Backup for Business 任务。需要包已安装和被允许使用它的账户；没有包的 NAS 简单被跳过。 |
| `drive` | 观察 Synology Drive | `true` | 读 Synology Drive 服务是否回答、其团队文件夹和其活动连接计数。没有包的 NAS 简单被跳过。 |
| `photos` | 观察 Synology Photos | `true` | 显示 Synology Photos 包是否已安装，无额外请求到 NAS。Synology 不通过任何 API 暴露一个移动备份设备列表。 |

## 常见错误 {#common-errors}

| 症状 | 可能原因 |
|---|---|
| 认证错误 | 错误密码，或这个账户需要两步验证。DSM 甚至在失败答复 HTTP 200；JSON 正文的错误代码是什么重要，DumbMonit 读它。 |
| 证书错误 | 自签名证书：启用 `insecure_tls`。这是配置错误，从不 "无法到达"：答复的 NAS 不显示为关闭。 |
| 无磁盘或卷指标 | 账户不在 `administrators` 组（见上面的注）。 |
| 无 `dumbmonit_abb_*` 指标，`scrape_errors` 在 1 | 账户无法使用 Active Backup for Business：委派包到它或禁用 `abb` 选项。没有包安装无错误和无指标。 |
| 任务指标但在页面上无设备行 | NAS 不宣布 `SYNO.ActiveBackup.Overview`（较旧 ABB），或设备被探测通过保留历史在其侧内存的中继代理：指标存在，日历不。 |
| 笔记本在添加后一天显示 "Overdue" | 少于三个成功：模型仍学习并允许 72 小时备份间；它一旦两周习惯被知道就解决。 |
| 磁盘在每个探针上旋转启动 | 存储库存唤醒睡眠磁盘。延长检查间隔。 |
