# Proxmox Backup Server {#proxmox-backup-server}

备份服务器：每台机器的三十天备份日历、失败的任务及其日志、计划的作业（同步、验证、修剪、垃圾回收），数据存储、磁盘和 ZFS 池，待处理的更新、服务器本身的服务和包版本，以及磁带层（如果存在）。

## 监控项页面 {#the-device-page}

![PBS 监控项：故障、30 天备份日历、作业和健康](../assets/screenshots/device-pbs-light.png){ loading=lazy }

超越通用图表，PBS 监控项显示六个面板，从探针存储的内容读取——打开页面从不查询 PBS 本身：

* **故障** — 过去 30 天内失败的每个任务（备份、同步、验证、修剪、GC），最新在前，及其类型、其所针对的内容、何时启动以及 PBS 自己的错误消息。*显示日志*从 PBS 按需获取任务日志的最后行（`/nodes/localhost/tasks/{upid}/log`）。
* **备份日历** — 每个备份的机器一行（数据存储、命名空间、`type/id` 和当 PVE 在快照注释中写入时的来宾名称），每天一个点：当该天备份成功（任务结束 `OK` 或快照存在——同步的组没有本地任务）时为青绿色，当备份任务失败且无成功时为红色，备份存在但其验证失败时为琥珀色，一个正在运行时为蓝色，无任何事发生时为灰色。每个点的工具提示给出时间、大小、持续时间和错误摘录；该行说明最后成功和最后失败何时、快照计数、最新快照的大小和应用的修剪作业的保留。备份全部失败的机器没有快照但仍然出现——从其任务。
* **作业** — 每个同步、验证和修剪作业以及每个数据存储的 GC，及其时间表、最后运行、结果、下次运行；失败的最后运行获得红色平板和错误文本，故障首先排序。
* **磁带** — 仅当服务器有磁带层时。磁带备份作业及其池、驱动器、下一个预期磁带的标签、最后运行及其错误；媒体池及其分配和保留政策、它们容纳的磁带数、多少已过期以及它们是否加密；驱动器和更换器；以及磁带本身及其状态、媒体集、位置和写入的字节。
* **数据存储和磁盘** — 使用条，PBS 自有的填满预报，重复数据消除因子，最后 GC；ZFS 池及其健康；物理磁盘及 SMART 判定和 SSD 磨损，完整 SMART 表按需（`/nodes/localhost/disks/smart`）。每个数据存储它还显示维护或"未挂载"平板、"忙碌：n 写，n 读"平板、每天测量的增长以及多少天的历史、它容纳的每种类型的机器数、最后 GC 花费了多长时间，以及当该 GC 发现无法读取的块时的警告行。
* **服务器** — systemd 单元，其中服务器离不开的被指出；Proxmox 包版本（已安装、可用、运行），当守护进程落后其包时带"重启待处理"平板；证书及其过期；以及流量控制规则及其限制和它们正在执行的操作。

日历在浏览器的本地午夜拆分天。其历史来自任务列表：启动后的第一个探针读取最后 30 天（`/nodes/localhost/tasks?since=…&limit=5000`），后来的探针仅任务窗口，服务器在 SQLite（`pbs_task_history`，35 天，每个监控项最多 10,000 个任务）中保留联合，所以日历在重启后存活。

## 它监控什么 {#what-it-watches}

所有指标前缀为 `dumbmonit_pbs_`。

| 族 | 指标 | 标签 |
|---|---|---|
| 节点 | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`（等待 I/O 的 CPU 时间——在备份服务器上这是说磁盘饱和的），`node_load1/5/15`, `node_memory_used/total/free_bytes`, `node_memory_used_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `version_info` | |
| 数据存储 | `datastore_available`, `datastore_bytes_used/total/avail`, `datastore_used_percent`, `datastore_estimated_full_seconds`（PBS 自有的估计；缺乏数据或使用减少时不存在），`datastore_dedup_factor`, `datastore_removable_unmounted`（当可移动数据存储是 `notmounted` 时为 1——不是错误，它根本没有插入；标签 `mount_status`），`datastore_backend_info`（标签 `backend`, `filesystem` 或 `s3`），`datastore_maintenance`（当数据存储被保存以供维护时为 1；标签 `mode`, `offline`, `read-only`…），`datastore_groups` 和 `datastore_snapshots`（标签 `type`, `vm`/`ct`/`host`/`other`），`datastore_active_reads`, `datastore_active_writes`（现在正在保存数据存储的内容——这是解释不会完成的垃圾回收的内容），`datastore_growth_percent_per_day`, `datastore_growth_bytes_per_day`, `datastore_history_days` | `datastore` |
| 垃圾回收 | `gc_last_removed_bytes`, `gc_last_pending_bytes`, `gc_last_run_ok`（PBS 3.3+），`gc_last_success_age_seconds`, `gc_last_duration_seconds`, `gc_disk_chunks`, `gc_last_pending_chunks`, `gc_last_removed_chunks`, `gc_bad_chunks`（GC 发现无法读取且留在原地的块——腐败，不是要回收的空间：引用它们的备份不再可恢复），`verify_last_success_age_seconds`, `sync_last_success_age_seconds` | `datastore` |
| 备份组 | `backup_count`, `backup_last_timestamp_seconds`, `backup_last_age_seconds`, `backup_last_size_bytes`, `backup_last_verified`（1 已验证，0 失败，从不验证时不存在） | `datastore`, `namespace`, `backup_type`, `group` |
| 命名空间 | `namespace_groups`, `namespace_snapshots`（根命名空间是 `namespace=""`） | `datastore`, `namespace` |
| 窗口中的任务 | `tasks_running`, `tasks_ok`, `tasks_failed` | `worktype`（`backup`, `verificationjob`, `garbage_collection`, `prune`, `syncjob`…） |
| 作业 | `job_enabled`, `job_last_ok`（对于 `OK` 或 `WARNINGS: n` 为 1，否则为 0，作业从不运行时不存在），`job_never_run`（当作业启用、有时间表且从不运行时为 1——它既没有失败也没有成功，所以它在其他地方是不可见的），`job_last_run_age_seconds`, `job_next_run_seconds`（逾期时为负），`sync_jobs_total`, `verify_jobs_total`, `prune_jobs_total`；别名 `sync_job_last_ok` 用于规则 | `job`, `datastore`, `kind`（`sync`, `verify`, `prune`）；同步作业上的 `remote`（`remote:remote-store` 或 `local`）——别名携带 `job`, `datastore`, `remote` |
| 磁带（选项 `tape`，默认关闭，五个调用） | `tape_backup_job_last_ok`, `tape_backup_job_never_run`, `tape_backup_job_last_run_age_seconds`, `tape_backup_job_next_run_seconds`, `tape_backup_jobs_total`；`tape_drive_info`（存在系列），`tape_drives_total`, `tape_changers_total`, `tape_media_pools_total`；`tape_media_total`, `tape_media_writable`, `tape_media_expired`, `tape_media_bytes_used` | 作业上的 `job`, `datastore`, `pool`, `drive`；`tape_drive_info` 上的 `drive`, `vendor`, `model`, `changer`；媒体计数上的 `pool` |
| 磁盘（与 Proxmox VE 相同的名称） | `node_disk_size_bytes`, `node_disk_smart_failed`（0 `passed`, 1 `failed`；SMART 无判定时不存在），`node_disk_health_info`（存在系列；标签 `health`, `serial`, `used`），`node_disk_wearout_percent`（耐用性已使用，仅 SSD，如 PBS 显示：`100 − wearout`） | `disk`（`/dev/sda`），`model`, `type` |
| ZFS 池（与 Proxmox VE 相同的名称） | `node_zfs_pool_degraded`（0 `ONLINE`, 1 其他），`node_zfs_pool_health_info`（标签 `health`），`node_zfs_pool_size_bytes`, `node_zfs_pool_alloc_bytes`, `node_zfs_pool_free_bytes`, `node_zfs_pool_used_percent`, `node_zfs_pool_fragmentation_percent` | `pool` |
| 更新 | `node_updates_pending`（有可用更新的包数） | |
| 版本（选项 `updates`，来自 `/nodes/localhost/apt/versions`） | `node_package_upgradable`（1/0），`node_package_info`（存在系列；标签 `installed`, `available`, `running`），`node_running_version_stale`（当已安装的 `proxmox-backup-server` 包不再是运行的守护进程时为 1——包被升级，服务未重启） | `package`，受限于 `proxmox-backup`, `proxmox-backup-server`, `proxmox-backup-client`, `proxmox-kernel-helper` |
| 服务（选项 `services`，一个调用） | `node_service_active`（1 运行，0 未运行），`node_service_enabled`（对于 `static` 单元不存在，它由另一个单元拉入，因此既不启用也不禁用），`node_services_total` | `service`, `expected`（对于 `proxmox-backup`, `proxmox-backup-proxy` 和 `proxmox-backup-banner` 为 `"1"`，服务器离不开的单元；每个其他单元仍获得其系列，带 `expected="0"`，从不引发警报） |
| 证书（选项 `certificates`，默认关闭） | `node_certificate_expires_seconds`，过期后为负 | `certificate`（文件名），`issuer` |
| 流量控制（选项 `traffic_control`） | `traffic_rate_in_bytes`, `traffic_rate_out_bytes`（在探针时刻测量），`traffic_limit_in_bytes`, `traffic_limit_out_bytes`（已配置的限制，从 PBS 的十进制字符串如 `100 MB` 解析；该方向无限制时不存在，而不是会读作"一切都被阻止"的零），`traffic_rules_total` | `rule` |
| 限制和收集 | `backup_groups_total`, `backup_groups_dropped`, `up`, `scrape_errors`, `scrape_duration_seconds` | |

结束于 `WARNINGS: n` 的任务计为成功。每个数据存储的调用限制为四个平行：列出快照读取数据存储的索引，突发会减慢进行中的备份。

作业列表不限于任务窗口：三周来一直失败的同步作业仍被报告为失败。从不运行的作业没有 `job_last_ok` 系列，但它确实获得 `job_never_run`，"PBS 作业从不运行"规则对该情况精确触发。

垃圾回收是用一个覆盖每个数据存储的单一 `/admin/gc` 调用读取的，之前对每个数据存储进行一个 `/admin/datastore/{store}/gc` 调用；当 `/admin/gc` 不可用时——较旧的 PBS 或缺失的特权——该每数据存储调用仍作为后备存在，未计为错误。`/admin/gc` 也携带运行持续时间和每数据存储调用不具有的块计数器。`gc_last_run_ok` 来自 PBS 3.3 及更高版本的 `last-run-state`；在较旧的版本上它后备到窗口的最近垃圾回收任务。每类型计数来自 `/admin/datastore/{store}/status?verbose=1`：不带 `verbose`，PBS 仅返回大小，这是 `/status/datastore-usage` 已经给出的。`/admin/datastore/{store}/namespace` 返回整个命名空间树，所以子命名空间中没有损失——`pve/site-a/rack1` 被计为根。磁盘和池来自 `/nodes/localhost/disks/list` 和 `/nodes/localhost/disks/zfs`（选项 `disks`；403 被容忍，以及服务器无 ZFS 时答的 400：无池系列，无擦洗错误）。

`datastore_growth_percent_per_day` 是 PBS 本身使用 `/status/datastore-usage` 返回的使用历史上的最小二乘拟合：一个使用*分数*介于 0 和 1 之间，过去一个月每三十分钟一个点，`null` 洞被跳过而不是填补——一台关闭的服务器在关闭期间未增长。这些是 PBS 自有"完满N天"预报后面的相同数字。需要至少八个测量点，才能发布任何东西，`datastore_history_days` 说斜率覆盖多少历史。负值不是故障：修剪回收了超过备份添加的内容。

当服务器没有驱动器、更换器、池和磁带作业时，完全没有为磁带发布——没有磁带的安装必须不增长一打零系列，它读起来像磁带层处于困境。属于无池、空白媒体或从池退休的媒体的磁带，不在池计数中。

## 设备 {#devices}

每个备份的机器——主机、VM 或容器——也发布在由每个 DumbMonit 集成共享的通用族下，该集成跟踪受保护的设备——`dumbmonit_client_device_*`，前缀不带 `pbs`，完整记录在[客户端设备](../alerting/rules.md#client-devices)下。对 PBS 没有要求任何新东西：这是与 `backup_last_timestamp_seconds` 后面相同的最后快照日期，在共享族下重新发布，以便一个规则和一个表与 PBS 一起服务每个保护设备的其他集成。

| 指标 | 说明 | 标签 |
|---|---|---|
| `last_backup_timestamp_seconds` | 与该组的 `backup_last_timestamp_seconds` 相同的时刻 | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | 一旦组超过 `device_stale_days`（默认 3）没有新快照就为正数；驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`, `type`, `os`, `user`, `kind`, `signal`（总是 `backup`） |

**值得知道的限制**：PBS 不为备份的机器记录单独的"连接"，仅备份——`last_seen_timestamp_seconds` 和 `connection` 信号从不发布在这里；设备在监控项页面的表上将 `last_connection` 显示为"——"。警报中命名的设备是当 PVE 写入一个时快照注释中的来宾名称，否则 `type/id`，与备份日历相同。

适用的内置规则：设备不可达、PBS 数据存储几乎已满、PBS 数据存储填满、PBS 备份过旧（每个备份组：两天内无新快照——编辑规则的阈值以获得更长的窗口）、PBS 备份验证失败、PBS 任务失败、PBS 垃圾回收过旧、PBS 垃圾回收失败、PBS 发现腐败块、PBS 同步作业失败、PBS 修剪作业失败、PBS 验证作业失败、PBS 作业从不运行、PBS 数据存储未挂载、PBS 服务故障、PBS 升级后重启待处理、PBS 证书过期、PBS 磁带备份失败、PBS 磁盘 SMART 故障、PBS SSD 磨损、PBS ZFS 池降级、PBS 更新待处理、客户端设备陈旧。通知命名有关的作业、数据存储或备份组。

## 在 PBS 中要准备什么 {#what-to-prepare-in-pbs}

下面的步骤是通知旁边的表单显示的内容。原则：为监控预留的用户，具有仅读角色的令牌——从不登录的账户。

1. 在备份服务器上打开shell（网页 UI 中：管理 → Shell；或 SSH）并创建为监控预留的用户。它不需要密码：令牌是登录的内容。

    ```
    proxmox-backup-manager user create dumbmonit@pbs --comment "DumbMonit monitoring"
    ```

2. 创建用户的 API 令牌。命令打印令牌 id 和其秘密：现在复制两者，PBS 从不再次显示秘密。

    ```
    proxmox-backup-manager user generate-token dumbmonit@pbs monitor
    ```

3. 给予仅读最小值，每个路径——给令牌和拥有它的用户。令牌的有效特权是其自有 ACL 和其用户的交集：仅授予令牌，它等于完全没有，PBS 在任何地方都不报告那个。因此每行被写入两次。`/datastore`（`Datastore.Audit`）上的 `DatastoreAudit` 读取数据存储、快照、验证和修剪作业和 GC；`/system`（`Sys.Audit`）上的 `Audit` 读取节点状态、任务列表和任务日志、服务、流量控制规则、磁盘和 ZFS 池。

    ```
    proxmox-backup-manager acl update /datastore DatastoreAudit --auth-id 'dumbmonit@pbs'
    proxmox-backup-manager acl update /datastore DatastoreAudit --auth-id 'dumbmonit@pbs!monitor'
    proxmox-backup-manager acl update /system Audit --auth-id 'dumbmonit@pbs'
    proxmox-backup-manager acl update /system Audit --auth-id 'dumbmonit@pbs!monitor'
    ```

    可选，和两倍。`/remote`（`Remote.Audit`）上的 `RemoteAudit` 显示从远程拉取的同步作业：内置 `Audit` 角色仅覆盖 `Sys.Audit` 和 `Datastore.Audit`，没有 `Remote.Audit` 这样的作业不被拒绝，它根本从作业列表中不存在。`/tape`（`Tape.Audit`）上的 `TapeAudit` 读取磁带层。`/`（顶级 `Sys.Audit`）上的 `Audit` 列出待处理的包更新，并在你愿意时也覆盖 `/datastore` 和 `/system` 更少行。不带它们这些项被静默跳过，别的都不改变。这里什么都无法写：收集器仅执行 `GET`。

    ```
    proxmox-backup-manager acl update /remote RemoteAudit --auth-id 'dumbmonit@pbs'
    proxmox-backup-manager acl update /remote RemoteAudit --auth-id 'dumbmonit@pbs!monitor'
    proxmox-backup-manager acl update /tape TapeAudit --auth-id 'dumbmonit@pbs'
    proxmox-backup-manager acl update /tape TapeAudit --auth-id 'dumbmonit@pbs!monitor'
    proxmox-backup-manager acl update / Audit --auth-id 'dumbmonit@pbs'
    proxmox-backup-manager acl update / Audit --auth-id 'dumbmonit@pbs!monitor'
    ```

4. 在离开 shell 前，检查令牌可以实际读取什么。命令打印其有效特权，路径至路径；空结果意味着用户缺失令牌拥有的 ACL——令牌令牌拥有的情况，监控项看起来完全活跃并报告无东西。

    ```
    proxmox-backup-manager user permissions 'dumbmonit@pbs!monitor'
    ```

5. 将令牌 id 复制到 DumbMonit 的令牌 ID 字段中，秘密复制到其秘密字段中。

    ```
    dumbmonit@pbs!monitor
    ```

6. 偏好网页 UI？配置 → 访问控制：用户 → 添加，然后 API 令牌 → 添加，然后权限 → 添加。添加每个权限两次，一次作为 `dumbmonit@pbs` 的用户权限，一次作为 `dumbmonit@pbs!monitor` 的 API 令牌权限：带路径 `/datastore` 和角色 `DatastoreAudit`，然后再用路径 `/system` 和角色 `Audit`。令牌权限本身不授予任何东西。
7. 在 DumbMonit 中，输入服务器地址，例如"pbs.lan"或"pbs.lan:8007"。

!!! warning
    不重用登录的账户：泄露的令牌然后管理每个备份。审计角色只能读。PBS 默认也使用自签名证书：如果连接因此原因被拒绝，勾选选项中的"接受无法验证的证书"。

供应商文档：[PBS API 令牌](https://pbs.proxmox.com/docs/user-management.html#api-tokens)。

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| API 令牌（推荐） | **令牌 ID**：用户、领域和令牌名称如 PBS 显示，`dumbmonit@pbs!monitor`。**秘密**：创建令牌时显示一次的 UUID。DumbMonit 将它们作为 PBS 期望的 `PBSAPIToken=user@pbs!name:secret` 头发送。 |
| 用户名 / 密码 | `user@pbs`（或 `@pam`）和密码。获取、缓存和在两小时过期前十分钟更新票证。 |

API 接受令牌在两件（`token_id` + `secret`）或如较旧的版本所做，作为一个字符串 `token` = `user@pbs!name=secret`；两者都以相同的方式存储。整个令牌粘贴到令牌 ID 被形式拆分。

权限，仅读，并授予两次：令牌的有效特权是其自有 ACL 和其用户的交集，因此下面每个角色都必须给予 `dumbmonit@pbs` 以及 `dumbmonit@pbs!monitor`。最简单：`/` 上的内置 `Audit` 角色，加 `/remote` 上的 `RemoteAudit` 用于同步作业以及 `/tape` 上的 `TapeAudit` 用于磁带层——`Audit` 仅覆盖 `Sys.Audit` 和 `Datastore.Audit`，这两个角色正是因为它不存在。每个端点：

| 端点 | 需要 |
|---|---|
| `GET /version` | 任何已认证的用户 |
| `GET /nodes/localhost/status`, `GET /nodes/localhost/tasks`, `GET /nodes/localhost/tasks/{upid}/log` | `/system` 上的 `Sys.Audit`（`/system` 上的 `Audit`） |
| `GET /nodes/localhost/disks/list`, `…/disks/smart`, `…/disks/zfs` | `/system` 上的 `Sys.Audit`（`/system` 上的 `Audit`）；403 被容忍：无磁盘系列，无擦洗错误。`…/disks/zfs` 在无 ZFS 的服务器上答 400，以相同的方式被容忍 |
| `GET /nodes/localhost/services`, `GET /admin/traffic-control` | `/system` 上的 `Sys.Audit`（`/system` 上的 `Audit`）；403 被容忍：无系列，无擦洗错误 |
| `GET /nodes/localhost/certificates/info` | `/system` 上的 `Sys.Modify`——一个**写**特权，这是为什么 `certificates` 选项默认关闭以及为什么监控令牌不期望有它 |
| `GET /status/datastore-usage`, `GET /admin/gc`, `GET /config/datastore`, `GET /admin/datastore/{store}/gc`, `…/status`, `…/active-operations`, `…/namespace`, `…/snapshots` | `/datastore/{store}` 上的 `Datastore.Audit`（`/datastore` 上的 `DatastoreAudit`） |
| `GET /admin/verify`, `GET /admin/prune` | 在令牌有 `Datastore.Audit` 的数据存储中列出；其他作业被静默省略 |
| `GET /admin/sync` | `/datastore` 上的 `Datastore.Audit` **和** `/remote/{remote}`（`/remote` 上的 `RemoteAudit`）上的 `Remote.Audit`；其他作业被静默省略 |
| `GET /nodes/localhost/apt/update`, `…/apt/versions` | `/`（`/` 上的 `Audit`）上的 `Sys.Audit`；403 被容忍：无更新或版本系列，无擦洗错误 |
| `GET /tape/backup`, `/tape/drive`, `/tape/changer`, `/config/media-pool`, `/tape/media/list` | `/tape` 上的 `Tape.Audit`（`/tape` 上的 `TapeAudit`）；每个故障被容忍，无磁带支持的 PBS 包括：无磁带系列，无擦洗错误 |

注意 `/system` 上的 `Audit` 单独不覆盖更新列表，PBS 在根路径检查。作业列表或更新列表上的 403 被视为可选特权：无任何东西产生并无任何东西计数在 `scrape_errors` 中。用户上缺失的角色比 403 更坏：调用成功并返回空列表，所以无任何东西产生并无任何东西怨言。

地址：`10.0.0.20`, `pbs.lan`, `pbs.lan:8007`, `[fd00::2]` 或完整 URL。`https` 方案和端口 8007 在缺失时被添加。

## 选项 {#options}

| 键 | 标签 | 默认 | 帮助 |
|---|---|---|---|
| `port` | API 端口 | `8007` | 当地址不给出端口时使用。 |
| `insecure_tls` | 接受无法验证的证书 | `false` | Proxmox Backup Server 默认使用自签名证书：如果连接因此原因被拒绝，启用这个。 |
| `request_timeout_seconds` | 每个请求的超时（秒） | `15` | 允许每个 API 调用的时间，从 1 到 120。列出大数据存储的快照可能花费数秒。 |
| `task_lookback_hours` | 任务窗口（小时） | `24` | 较旧的任务不被计入故障，从 1 到 8760。 |
| `datastores` | 监控的数据存储 | *（空）* | 要监控的数据存储的名称，以逗号分隔。空：每个数据存储。 |
| `max_groups` | 备份组限制 | `500` | 最大生成系列的备份机器数；超过它，最旧的被忽视。从 1 到 100000。 |
| `jobs` | 观看计划的作业 | `true` | 列出同步、验证和修剪作业及其最后运行（`/admin/sync`, `/admin/verify`, `/admin/prune`）。 |
| `updates` | 观看待处理的更新 | `true` | 计数有可用更新的包（`/nodes/localhost/apt/update`）。 |
| `disks` | 观看磁盘和 ZFS 池 | `true` | 读取物理磁盘（SMART 判定，SSD 磨损）和 ZFS 池（`/nodes/localhost/disks`）。 |
| `services` | 观看服务器的服务 | `true` | 读取 systemd 单元并报告服务器离不开的单元是否运行。需要 `/system` 上的 `Sys.Audit`；否则静默跳过。 |
| `datastore_details` | 读取数据存储详情 | `true` | 每个数据存储两个更多调用：它容纳多少机器和快照，现在什么在读或写它，以及它是否被保存以供维护。 |
| `traffic_control` | 观看流量限制 | `true` | 读取速率限制和现在它们正在执行什么。需要 `Sys.Audit`；否则静默跳过。 |
| `certificates` | 观看证书过期 | `false` | 默认关闭：PBS 在 `Sys.Modify` 后面保护这一调用，一个监控令牌不应该被给的写特权。 |
| `tape` | 观看磁带备份 | `false` | 读取磁带层。默认关闭，因为大多数安装没有磁带硬件。需要 `/tape` 上的 `Tape.Audit`。 |
| `device_stale_days` | 设备陈旧（天） | `3` | 一台没有新快照超过这个时间的备份机器被报告陈旧。从 1 到 90。 |

## 常见错误 {#common-errors}

| 症状 | 可能原因 |
|---|---|
| 证书错误 | 自签名证书：安装受信任的一个（ACME 内置到 PBS 中）或启用 `insecure_tls`。 |
| 认证错误 | 秘密粘贴到令牌 ID（或反过来），或令牌没有 `Audit` 权限——最常见原因是角色被授予令牌但不被授予拥有它的用户，这不失败：调用成功并返回空列表。仅 `GET /version` 失败谴责整个探针；任何其他调用失败被计数在 `scrape_errors` 中。 |
| 一切读为空但监控项向上 | 用户缺失令牌拥有的 ACL，PBS 交集两个：`proxmox-backup-manager user permissions 'dumbmonit@pbs!monitor'` 打印无东西。也给 `dumbmonit@pbs` 授予每个角色。 |
| 无 `node_updates_pending` 系列 | 令牌缺失 `/` 上的 `Sys.Audit`：给它 `/` 上的 `Audit` 角色，或设置 `updates` 为 `false` 停止询问。 |
| 同步作业丢失 | 令牌缺失该作业的远程上的 `Remote.Audit`：在 `/remote` 上添加 `RemoteAudit` 角色。作业不被拒绝，它根本从列表中不存在。 |
| 无证书过期 | `certificates` 选项关闭，这是默认：PBS 需要该调用的 `Sys.Modify`，写特权。 |
| 磁带面板从不出现 | `tape` 选项关闭，这是默认，或服务器没有驱动器、更换器、池或磁带作业。当无东西时无东西被发布。 |
| 数据存储上无增长数据 | PBS 的使用历史有少于八个测量点；它在第一天填满。 |
| "PBS 数据存储填满"从不触发 | `datastore_estimated_full_seconds` 由 PBS 从一个月的测量计算并在然后不存在。 |
| 缺失的组 | 超过 `max_groups` 备份的机器：`backup_groups_dropped` 说有多少被留出。 |
| 日历为空或短 | 它从启动后的第一个探针（最后 30 天）从任务列表填满；任务存档被清洁的 PBS 仅显示什么保留。从不备份到这个服务器且从不有任务的机器没有行。 |
| 一台机器显示为 `vm/104` 无其名称 | PVE 仅用默认注释模板（`{{guestname}}`）在快照注释中写来宾名称；自定义模板或较旧的 PVE 留注释空。 |
| "显示日志"失败 | 令牌缺失任务日志的 `/system` 上的 `Sys.Audit`，或 PBS 已轮转旧任务的日志。 |
