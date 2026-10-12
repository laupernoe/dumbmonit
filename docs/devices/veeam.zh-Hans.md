# Veeam Backup & Replication

通过其 REST API 监控 Veeam Backup & Replication 服务器:上次运行
失败或以警告结束的任务、过去 24 小时内失败的会话、空间不足的
存储库、许可证,以及——按每台受保护的计算机或虚拟机——其上次
成功备份的时间。

DumbMonit 使用一个只持有 Veeam Backup Viewer 角色的 Windows
账户登录(在 `/api/oauth2/token`、端口 9419 上使用 OAuth2 密码
授权),并在其十五分钟的有效期内保留该令牌。每次调用都带有
必需的 `x-api-version` 请求头,默认为 `1.1-rev0`:这是随版本
12.0 推出的修订版。

任务状态(`/api/v1/jobs/states`)是唯一必需的读取操作。其他的——
`/api/v1/serverInfo`、`/api/v1/sessions`(过去 24 小时内创建的)、
`/api/v1/backupInfrastructure/repositories/states`、
`/api/v1/license` 和 `/api/v1/restorePoints`(每个受保护对象的
还原点,整个服务器范围,最新的在前)——只有在角色允许时才会读取。
Veeam 在当前版本上把一些路由保留给 Backup Administrator 角色,
许可证也在其中:在那里被拒绝会显示为"不可读",而不是错误。
一台太旧、不认识 `/api/v1/restorePoints` 的服务器——这个路由
是在 12.0 之后才出现的——会被同样处理:没有设备,也没有错误。

!!! warning "仅依据厂商文档验证——尚未在真实系统上测试"

    这项集成是根据 Veeam 的 REST API 参考和帮助中心(认证、
    版本控制、任务、会话、存储库、许可证和还原点模型)构建的,
    而不是针对一台正在运行的备份服务器。Viewer 角色可以读取
    哪些路由会因版本而异:请告诉我们你的服务器拒绝了什么。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_veeam_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `version_info` | `version` | 固定值 1:构建版本,`12.3.1.1139`。 |
| `job_enabled`, `job_running` | `job`, `type` | 按任务。 |
| `job_failed`, `job_warning` | `job`, `type` | 当某个已启用任务的上次运行失败,或以警告结束时为 1。已禁用的任务会保留它最后的结果,但这不再有任何意义:它既不计为失败也不计为警告。 |
| `job_result_info` | `job`, `type`, `result` | 固定值 1:`success`、`warning`、`failed`、`none`。 |
| `job_last_run_age_seconds` | `job`, `type` | 距上次运行的时间;从未运行过的任务不存在。 |
| `job_progress_percent` | `job`, `type` | 任务运行期间。 |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | 计数;最多逐一描述 200 个任务。 |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | 过去 24 小时内创建的会话,按结果划分。 |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | 按存储库。对象存储存储库没有容量概念:只会公布其已用空间。 |
| `license_valid` | | 当许可证状态为 `Valid` 时为 1。 |
| `license_info` | `status`, `type`, `edition` | 固定值 1。 |
| `license_expiry_seconds`, `support_expiry_seconds` | | 距许可证和支持合同过期的时间;永久许可证不存在。 |
| `license_instances_licensed`, `license_instances_used` | | 实例许可证的使用情况。 |
| `section_readable` | `section`:`sessions`、`repositories`、`license`、`restore_points` | 当该账户有权读取时为 1,被 Veeam 拒绝时为 0。 |

适用的[内置规则](../alerting/rules.md#veeam-backup-replication):

- **Veeam 任务失败**:某个已启用任务的上次运行失败(警告级别,
  每天提醒)。
- **Veeam 任务以警告结束**(提示级别,每天提醒)。
- **Veeam 存储库几乎已满**:超过 90% 持续三十分钟(提示级别)。
- **Veeam 许可证即将过期**:剩余不足 30 天(提示级别,每天
  提醒)。仅在该账户能读取许可证时生效。
- **客户端设备陈旧**:某台受保护的计算机或虚拟机超过
  `device_stale_days` 没有成功备份(提示级别,每天提醒);见下方
  [设备](#devices)。

## 设备 {#devices}

每一个至少有一个还原点的对象——虚拟机、受代理保护的物理或虚拟
计算机、文件共享——都会以每个追踪受保护设备的 DumbMonit 集成
共享的通用指标族发布——`dumbmonit_client_device_*`,前缀中不含
`veeam`,完整文档见[客户端设备](../alerting/rules.md#client-devices):

| 指标 | 含义 | 标签 |
|---|---|---|
| `last_backup_timestamp_seconds` | 该对象最近的还原点 | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | 当某个对象超过 `device_stale_days`(默认 3 天)没有新还原点时为正值;驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`, `type`, `os`, `user`, `kind`, `signal`(始终为 `backup`) |

**值得了解的限制**:一个任务是把其对象作为一个整体来保护的
(`jobs/states` 只给出一个数量,`objectsCount`);按对象划分的
日期改为来自 `/api/v1/restorePoints`,该接口会为整个服务器一次性
读取。只有在备份至少部分成功之后,Veeam 才会创建还原点,因此
每个对象最近的还原点本身就已经是"上次成功备份"——这里不会再按
结果过滤。Veeam 不会公布独立于备份本身的"上次连接"信息,因此
`last_seen_timestamp_seconds` 和 `connection` 信号永远不会被
发布;某台设备在设备页面的表格中"上次连接"会显示为"—"。该设备
的 `type` 就是 Veeam 自己的平台名称(`VSphere`、
`WindowsPhysical`、`HyperV`……),按原样显示。

## 设备页面 {#the-device-page}

Backups 面板会用一句话说明哪里出了问题,显示失败的任务、过去
24 小时内失败的会话、正在运行的任务和许可证,然后列出已启用的
任务——失败的排在最前——及其类型和上次运行情况,以及存储库及其
使用率和剩余空间。接下来是上面那个通用指标族生成的受保护设备
表格(名称、平台、上次备份、状态)。

## 为 DumbMonit 配置一个 Veeam Backup Viewer 账户 {#give-dumbmonit-a-veeam-backup-viewer-account}

1. 在备份服务器上,为 DumbMonit 创建一个本地 Windows 账户,使用较长的随机密码,且在该机器上没有任何其他权限(域账户也可以)。

    ```
    net user dumbmonit * /add
    ```

2. 在 Veeam 控制台中,打开主菜单 > Users and Roles > Security,点击 Add,输入该账户(例如 VBR01\dumbmonit),并只赋予它 Veeam Backup Viewer 角色。

3. 在 DumbMonit 中,输入备份服务器的地址,例如 "vbr.lan",以及该账户,格式为 HOST\dumbmonit 或 DOMAIN\dumbmonit,连同其密码。该 REST API 监听在端口 9419 上,使用自签名证书:除非你安装了自己的证书,否则请勾选"接受无法验证的证书"。

!!! warning "警告"
    Veeam 在当前版本上把一些 REST API 路由保留给 Backup
    Administrator 角色,许可证也在其中。使用 Viewer 角色时,
    DumbMonit 能读取任务、会话、存储库和还原点,并把许可证显示
    为不可读:它从不会申请更多权限。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名和密码 | 拥有 Veeam Backup Viewer 角色的账户,格式为 `HOST\dumbmonit` 或 `DOMAIN\dumbmonit`。 |

地址:备份服务器的主机名或 IP、`host:port`,或 URL。选项:

- **协议**(`https`)和**端口**(9419):该 REST API 服务的。
- **API 版本**(`1.1-rev0`):以 `x-api-version` 发送。只有当
  服务器拒绝它时才需要更改;v13 服务器记录的是 `1.3-rev1`。
- **接受无法验证的证书**:该 REST API 自身的证书是自签名的。
- **每次请求的超时时间**(15 秒)。
- **设备陈旧判定**(`device_stale_days`,默认 3 天):见上方
  [设备](#devices)。

## 故障排查 {#troubleshooting}

"Veeam refused the login":用户名需要带上其主机或域
(`VBR01\dumbmonit`),或该账户没有任何 Veeam 角色。

"Veeam refused to list the jobs (403)":该账户的角色无法读取
任务;请赋予它 Veeam Backup Viewer。

"Veeam refused the API version":请将 API 版本选项设置为服务器
在 `https://<server>:9419/swagger` 上列出的版本。

许可证显示"Not readable":在当前版本上使用 Viewer 角色时这是
预期行为;DumbMonit 不会申请更多权限。

没有任何设备,或设备面板为空:`restore_points` 显示"Not
readable",更可能是因为服务器版本比该路由出现得更早
(12.0 后续修订版之前的任何版本),而不是角色权限问题——还原点
是普通的清单信息,不是一个仅限管理员的页面。
