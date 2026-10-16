# Proxmox Datacenter Manager {#proxmox-datacenter-manager}

联合管理多个 Proxmox VE 集群和 Proxmox Backup Server 实例的
控制台:它能访问哪些实例、整个资产一览、在任何地方失败的任务,
以及该控制台自身的健康状况。

!!! warning "仅依据测试实例验证"

    这项集成是依据专门为此安装的 Proxmox Datacenter Manager
    1.1 构建并验证的,而不是针对一个生产环境的实例。每次调用都
    是读取操作,各种结构来自一个真实的数据中心管理器,但尚无人
    在一个正在运行的数据中心管理器上长期运行过它。如果有任何
    问题,请告诉我们。

## 为整个资产设一个监控项,还是每个集群各设一个? {#one-device-for-the-estate-or-one-device-per-cluster}

一个 Datacenter Manager 监控项不能替代
[Proxmox VE](proxmox.md) 和 [Proxmox Backup Server](pbs.md)
监控项。它回答的是一个不同的问题。

| | Datacenter Manager 监控项 | 每个集群和每个备份服务器各一个监控项 |
|---|---|---|
| 需要创建的监控项 | 一个 | 每个集群一个,每个备份服务器一个 |
| 需要管理的凭据 | 一个令牌,在控制台上 | 每个实例一个令牌 |
| 对每个集群的负载 | 没有:我们读取的是控制台自己的缓存 | 每个周期、每个集群一次探测 |
| 能获得的内容 | 可达性、版本、虚拟机和节点计数、CPU、内存和存储总量、失败的任务 | 以上全部,再加上逐个的虚拟机、快照、备份任务、HA、Ceph、复制、磁盘和 SMART、ZFS 存储池、30 天备份日历 |
| 无法获得的内容 | 按虚拟机的序列、按数据存储的详情、磁盘、快照、任务日志 | 整个资产范围的单一总量 |
| 代表性故障 | 控制台无法连接到某个实例 | 该集群不应答我们 |

**我们的建议。** 当你已经在运行一个控制台、并希望有一个页面能
说明整个资产是否完整时,添加 Datacenter Manager 监控项——这是
发现某个远程站点掉线的最便宜的方式,且不会给各集群带来任何
负担。为你实际运营的每个集群和备份服务器添加一个专属监控项:
深度信息都在这里,它能告诉你*是哪台*虚拟机停止了,或*是哪次*
备份失败了。两者会有重叠但不会冲突;一次失败的备份会同时从
PBS 监控项(带任务日志)和控制台(指明实例)触发一条告警。如果
觉得这重复了,可以静默控制台的任务规则,或把它限定在你没有
专属监控项的那些实例上。

如果你完全不运行控制台,不要只为了监控而专门安装一个:专属监控项
看到的内容严格更多。

## 监控项页面 {#the-device-page}

除了通用图表之外,一个 Datacenter Manager 监控项还会显示三个
面板,读取自探测存储的数据——打开该页面既不会查询控制台,也不会
查询它管理的集群:

* **联合实例**——一条资产总量条带(实例数、在线节点数、运行和
  停止的虚拟机数、核心数、内存、存储),然后是每个联合实例一行。
  控制台无法访问的实例排在最前,然后是有失败任务的实例,再然后
  是版本落后的实例。每一行给出产品(Proxmox VE 或 Backup
  Server)、版本、节点和虚拟机计数、内存和存储使用率、订阅状态、
  配置的地址、控制台上次从它那里采集数据的时间——以及,当采集
  失败时,控制台自己收到的消息。这条消息是唯一能解释这次中断的
  内容,因此会原样显示。
* **失败记录**——整个资产范围内最近 14 天内失败的每一个任务,
  最新的排在最前,附带其运行所在的实例、任务类型、开始时间、
  耗时和错误消息。任务日志本身仍保留在该实例上:在那里打开它,
  或把该集群单独添加为监控项,在那里可以用*显示日志*获取它。
* **控制台主机**——运行该控制台的机器:CPU、内存、根文件系统、
  运行时间和内核,其证书及距过期的剩余天数,等待更新的软件包,
  以及受管节点的订阅状态。整个面板是可选的:一个没有
  `Sys.Audit` 权限的令牌读取不到其中任何内容,面板会说明这一点,
  而不是显示一堆零。

任务历史保存在 SQLite 中(`pdm_task_history`,19 天,每个监控项
最多 5000 个任务),因此即使每次探测只查看任务窗口,失败列表
仍能在重启后保留。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_pdm_` 为前缀。

| 指标族 | 指标 | 标签 |
|---|---|---|
| 资产 | `remotes_total`, `remotes_failed`, `nodes_online`, `nodes_offline`, `cpu_used_cores`, `cpu_total_cores`, `memory_used_bytes`, `memory_total_bytes`, `memory_used_percent`, `storage_used_bytes`, `storage_total_bytes`, `storage_used_percent`, `datastores_total` | |
| 资产虚拟机 | `guests_running`, `guests_stopped` | `guest_type`(`qemu`、`lxc`) |
| 实例 | `remote_reachable`(控制台收到应答时为 1,否则为 0)、`remote_version_info`(存在性序列,标签 `version`)、`remote_version_behind`、`remote_nodes_online`、`remote_nodes_offline`、`remote_guests_running`、`remote_guests_stopped`、`remote_cpu_used_cores`、`remote_cpu_total_cores`、`remote_cpu_used_percent`、`remote_memory_used_bytes`、`remote_memory_total_bytes`、`remote_memory_used_percent`、`remote_storage_used_bytes`、`remote_storage_total_bytes`、`remote_storage_used_percent`、`remote_datastores`、`remote_subscription_active`、`remote_updates_pending`、`remote_last_collection_age_seconds`、`remote_tasks_failed` | `remote`, `type`(`pve`、`pbs`) |
| 窗口内的任务 | `tasks_total`, `tasks_failed`, `tasks_running` | |
| 控制台主机 | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`, `node_load1`, `node_memory_used_bytes`, `node_memory_total_bytes`, `node_memory_used_percent`, `node_swap_used_bytes`, `node_swap_total_bytes`, `node_rootfs_used_bytes`, `node_rootfs_total_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `node_updates_pending`, `version_info` | |
| 控制台证书 | `node_certificate_expiry_days`(过期后为负数) | `file`(`proxy.pem`) |
| 订阅 | `subscription_active`(标签 `status`)、`subscription_active_nodes`、`subscription_total_nodes` | |
| 采集 | `up`, `scrape_errors`, `scrape_duration_seconds` | |

以 `WARNINGS: n` 结束的任务会被计为成功。只有 `GET /version`
失败才会判定整次探测失败;其他任何调用失败都会被计入
`scrape_errors`,而控制台无法访问的实例是一次测量
(`remote_reachable = 0`),不是一次采集错误。

`remote_version_behind` 是在**联合实例之间**互相比较的,按产品
分别比较:当同一资产中的另一个 Proxmox VE 集群——或另一个备份
服务器——运行着更新的版本时,它为 1。这不是与 Proxmox 官方发布
版本的比较;探测没有理由去访问互联网。该序列只存在于其版本已被
成功读取的实例上,因此一个不可达的实例永远不会被当作是最新的。

存储总量对每个实例中的每个存储名称只统计一次:一个由集群中五个
节点共享的存储只会被统计一次,不是五次。

适用的内置规则:监控项不可达、联合实例不可达、联合实例版本落后、
联合实例上的任务失败、数据中心控制台磁盘即将用尽、数据中心
控制台证书即将过期、数据中心控制台有待处理更新。通知会指明涉及
的实例。

## 需要在 Proxmox Datacenter Manager 中准备的内容 {#what-to-prepare-in-proxmox-datacenter-manager}

以下步骤正是表单旁边提示所展示的内容。原则:一个专用于监控的
用户,一个只读角色的令牌——绝不是你自己登录用的账户。

1. 在控制台中,打开 Configuration → Access Control → Users 并点击 Add。按如下方式命名该账户,密码留空:登录靠的是令牌。

    ```
    dumbmonit@pdm
    ```

2. 仍在 Access Control 下,打开 API Tokens → Add,选择该用户,把令牌命名为"monitor"。控制台只会显示一次密钥:请立即复制它。

    ```
    dumbmonit@pdm!monitor
    ```

3. 把 `Auditor` 角色同时赋予该用户和该令牌,在 `/`(树的顶层,即整个资产)上。`Auditor` 带有 `System.Audit`、`Resource.Audit` 和 `Access.Audit`,不能更改任何内容。令牌的权限永远不会超过其所属用户,因此需要授予两次该权限:Permissions → Add → User Permission,然后再用 API Token Permission 重复一次。
4. 把令牌 ID 复制到 DumbMonit 的 Token ID 字段,把密钥复制到其 Secret 字段。
5. 在 DumbMonit 中,输入控制台地址,例如 "dc.lan" 或 "dc.lan:8443"。

!!! warning "警告"
    不要重复使用你自己登录用的账户:泄露的令牌会一次性控制该
    控制台联合管理的每一个集群。`Auditor` 角色只能读取。该控制台
    默认也使用自签名证书:如果因此连接被拒绝,请在选项中勾选
    "接受无法验证的证书"。有两项内容需要超出 `Auditor` 的权限,
    没有它们会被直接跳过:按实例的指标采集状态,以及联合实例的
    更新摘要。

倾向于使用命令行?`proxmox-datacenter-manager-admin` 管理的是
远程实例,不是用户;请在网页界面中创建账户、令牌和这两项权限。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| API 令牌(推荐) | **令牌 ID**:控制台显示的用户、域和令牌名称,`dumbmonit@pdm!monitor`。**密钥**:令牌创建时显示一次的 UUID。DumbMonit 会以控制台期望的 `PDMAPIToken=user@pdm!name:secret` 请求头发送它们。 |

没有用户名/密码选项,这是有意为之的:自其首个版本起,该控制台
只会在一个 `HttpOnly` cookie 中返回其会话票据——JSON 响应体中
携带的 `ticket-info` 无法使用。登录相比令牌没有任何优势,反而会
在每次探测时打开一个已登录的会话。而且 Proxmox 本身也推荐在
自动化场景中使用令牌。

权限,只读。最简单且足够的方式:在 `/` 上的内置 `Auditor` 角色,
同时授予该用户*和*该令牌。按接口划分:

| 接口 | 需要的权限 |
|---|---|
| `GET /version` | 任何已认证的用户 |
| `GET /remotes/remote` | 返回调用者有权审计的实例 |
| `GET /remotes/remote/{id}/version` | 在 `/resource/{id}` 上的 `Resource.Audit` |
| `GET /resources/status`, `GET /resources/list` | 在 `/resource` 上的 `Resource.Audit` |
| `GET /resources/subscription` | 在 `/resource` 上的 `Resource.Audit` |
| `GET /remotes/tasks/list` | 每个实例在 `/resource/{remote}` 上的 `Resource.Audit` |
| `GET /nodes/{node}/status` | 在 `/system/status` 上的 `System.Audit` |
| `GET /nodes/{node}/certificates/info`, `GET /nodes/{node}/subscription` | 任何已认证的用户 |
| `GET /nodes/{node}/apt/update` | `System.Audit`;可以容忍 403:没有 `node_updates_pending` 序列,不算采集错误 |
| `GET /remotes/metric-collection/status` | 超出 `Auditor` 的权限;可以容忍 403:没有 `remote_last_collection_age_seconds` 序列,不算采集错误 |
| `GET /remotes/updates/summary` | `Resource.Modify`——一项写入权限。因此默认关闭(`remote_updates`) |

请注意,该控制台会对其持有的、用于各实例的访问令牌进行屏蔽
(`GET /remotes/remote` 返回的 `token` 为空),而 DumbMonit
无论如何也从不读取该字段:任何实例的密钥都不会经过该采集器、
存储的视图或 API。

地址:`10.0.0.40`、`dc.lan`、`dc.lan:8443`、`[fd00::2]`,或完整
URL。如果缺失,会自动补上 `https` 协议和 8443 端口。

## 选项 {#options}

| 键 | 标签 | 默认值 | 说明 |
|---|---|---|---|
| `port` | API 端口 | `8443` | 当地址中未给出端口时使用。 |
| `insecure_tls` | 接受无法验证的证书 | `false` | Proxmox Datacenter Manager 默认自带自签名证书:如果因此连接被拒绝,请启用此选项。 |
| `request_timeout_seconds` | 每次请求的超时时间(秒) | `20` | 每次 API 调用允许的时间,1 到 120。该控制台会把调用转发给每个联合实例,因此一个缓慢的站点会拖住整个应答。 |
| `node` | 控制台节点名称 | `localhost` | 运行该控制台的节点名称。一个 Datacenter Manager 只有一个节点,称之为"localhost"。 |
| `task_lookback_hours` | 任务窗口(小时) | `24` | 更早的任务不会被计入失败统计,1 到 8760。 |
| `max_age_seconds` | 可接受的缓存年龄(秒) | `60` | 控制台自身的清单在重新查询联合实例之前可以陈旧多久,0 到 3600。设为零会在每次探测时都要求最新数据,这会让每个集群重新上线被查询。 |
| `remotes` | 被监控的实例 | *(空)* | 要监控的联合实例名称,用逗号分隔。留空:所有实例。 |
| `max_remotes` | 实例版本上限 | `100` | 每次探测询问版本的实例数上限,1 到 1000。 |
| `versions` | 读取每个实例的版本 | `true` | 每个联合实例一次调用,用于报告其版本并标记落后于同类的实例。不应答的实例会被计为不可达。 |
| `tasks` | 跨实例监控任务 | `true` | 读取每个联合实例的任务列表以报告失败情况。 |
| `node_status` | 监控控制台主机 | `true` | 读取运行该控制台的机器的 CPU、内存、根文件系统、运行时间、证书和订阅情况。 |
| `updates` | 统计待处理的更新 | `true` | 列出控制台自身等待更新的软件包。 |
| `remote_updates` | 统计每个实例上的更新 | `false` | 读取控制台对联合实例的更新摘要。默认关闭:该控制台用 `Resource.Modify`(一项写入权限)来保护这条路径,而监控账户没有理由持有它。 |

`max_age_seconds` 是值得理解的一项。该控制台保留着自己的清单,
由其自身的采集循环刷新。每次探测都要求最新数据,会导致控制台
在其正常节奏之外再次调用每个集群——这恰恰是单一控制个监控项
本应避免的负载。六十秒仍比默认的探测间隔更新。

## 常见错误 {#common-errors}

| 现象 | 可能原因 |
|---|---|
| 证书错误 | 自签名证书:安装一个受信任的证书(控制台内置了 ACME),或启用 `insecure_tls`。 |
| 认证错误 | 密钥被粘贴到了令牌 ID 中(或反过来),或权限只授予了用户而没有授予令牌——令牌的权限永远不会超过其用户,这两个访问控制列表是分开的条目。 |
| 每个实例都显示为不可达,但控制台自己的页面正常 | 该令牌在用户上有 `Auditor`,但令牌本身没有,或只在 `/system` 上有:`/resource` 上的 `Resource.Audit` 才是让控制台能为其实例作答的关键。 |
| 某个实例不可达,且消息中提到了指纹 | 该控制台会验证它管理的每个实例的证书;请在控制台中(Remotes → 该实例)修复它,而不是在这里。 |
| 没有 `remote_last_collection_age_seconds` 序列 | `GET /remotes/metric-collection/status` 需要超出 `Auditor` 的权限,会被静默跳过。其他内容不受影响。 |
| 没有 `remote_updates_pending` 序列 | `remote_updates` 默认关闭,因为该路径需要 `Resource.Modify`。只有在你愿意授予一项写入权限时才开启它。 |
| 没有控制台主机面板 | 该令牌在 `/system/status` 上缺少 `System.Audit`,或 `node_status` 被关闭了。 |
| "联合实例版本落后" 从不触发 | 它是在比较同一产品的各个实例。一个只联合了单个 Proxmox VE 集群的控制台没有任何东西可以比较。 |
| 资产总量全部显示为破折号 | 控制台应答了但什么都没统计到——通常是因为每个实例都不可达。各实例行会说明原因。 |
