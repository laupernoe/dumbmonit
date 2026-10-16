# Veeam备份与复制 {#veeam-backup--replication}

通过其REST API的Veeam Backup & Replication服务器：上次运行失败或以警告结束的作业、过去24小时内失败的会话、填满的仓库、许可证以及——对于每个受保护的计算机或VM——其最后一次成功备份。

DumbMonit使用仅持有Veeam Backup Viewer角色的Windows账户登录（`/api/oauth2/token`上的OAuth2密码授予，端口9419），并在其十五分钟生命周期内保持令牌。每个调用都携带强制的`x-api-version`标头，默认为`1.1-rev0`：版本12.0附带的修订版本。

作业状态（`/api/v1/jobs/states`）是所需的唯一读取。其他的——`/api/v1/serverInfo`、`/api/v1/sessions`（在过去24小时内创建）、`/api/v1/backupInfrastructure/repositories/states`、`/api/v1/license`和`/api/v1/restorePoints`（每个受保护对象的还原点，服务器范围内，最新优先）——在角色允许时读取。Veeam为Backup Administrator角色保留某些路由，许可证在当前版本中属于其中：那里的拒绝显示为"不可读"，而不是错误。一个太旧而不知道`/api/v1/restorePoints`的服务器——该路由出现在12.0之后——受到相同对待：无监控项，无错误。

!!! warning "仅根据供应商文档验证——尚未在实际系统上测试"

    这个集成是从Veeam的REST API参考和帮助中心（认证、版本控制、作业、会话、仓库、许可证和还原点模型）构建的，而不是针对运行中的备份服务器。Viewer角色可能读取的路由因版本而异：告诉我们您的服务器拒绝了什么。

## 监控内容 {#what-it-watches}

所有指标都以`dumbmonit_veeam_`为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `version_info` | `version` | 值为1：构建版本，`12.3.1.1139`。 |
| `job_enabled`、`job_running` | `job`、`type` | 每个作业。 |
| `job_failed`、`job_warning` | `job`、`type` | 启用的作业的上次运行失败或以警告结束时为1。禁用的作业保持其最后结果，这不再说明任何内容：既不计数为成功也不失败。 |
| `job_result_info` | `job`、`type`、`result` | 值为1：`success`、`warning`、`failed`、`none`。 |
| `job_last_run_age_seconds` | `job`、`type` | 自上次运行以来；从不运行过的作业无此指标。 |
| `job_progress_percent` | `job`、`type` | 作业运行时。 |
| `jobs`、`jobs_failed`、`jobs_warning`、`jobs_running`、`jobs_disabled` | | 计数；最多200个作业逐一描述。 |
| `sessions_24h`、`sessions_failed_24h`、`sessions_warning_24h` | | 过去24小时内创建的会话，按结果。 |
| `repository_capacity_bytes`、`repository_free_bytes`、`repository_used_bytes`、`repository_used_percent` | `repository`、`type` | 每个仓库。对象存储仓库没有容量：仅其已用空间被发布。 |
| `license_valid` | | 许可证状态为`Valid`时为1。 |
| `license_info` | `status`、`type`、`edition` | 值为1。 |
| `license_expiry_seconds`、`support_expiry_seconds` | | 直到许可证和支持合同过期；永久许可证无此指标。 |
| `license_instances_licensed`、`license_instances_used` | | 实例许可证使用情况。 |
| `section_readable` | `section`：`sessions`、`repositories`、`license`、`restore_points` | 账户可能读取时为1，Veeam拒绝时为0。 |

适用的[内置规则](../alerting/rules.md#veeam-backup--replication)：

- **Veeam作业失败**：启用的作业的上次运行失败（警告，每日提醒）。
- **Veeam作业以警告结束**（建议，每日提醒）。
- **Veeam仓库几乎满**：超过90%持续30分钟（建议）。
- **Veeam许可证过期**：剩余不足30天（建议，每日提醒）。仅当账户可以读取许可证时。
- **客户端设备陈旧**：受保护的计算机或VM已超过`device_stale_days`而不成功备份（建议，每日提醒）；见下面[监控项](#devices)。

## 设备 {#devices}

每个至少有一个还原点的对象——VM、物理或虚拟代理保护的计算机、文件共享——在跟踪受保护设备的每个DumbMonit集成共享的通用家族下发布——`dumbmonit_client_device_*`，前缀中不含`veeam`，在[客户端设备](../alerting/rules.md#client-devices)下完整记录：

| 指标 | 说明 | 标签 |
|---|---|---|
| `last_backup_timestamp_seconds` | 对象的最新还原点 | `device`、`type`、`os`、`user`、`kind` |
| `stale_seconds` | 一旦对象超过`device_stale_days`（默认3）而不新还原点就为正数；驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`、`type`、`os`、`user`、`kind`、`signal`（始终为`backup`） |

**值得了解的限制**：一个作业作为一个组保护其对象（`jobs/states`仅给出计数，`objectsCount`）；每个对象日期来自`/api/v1/restorePoints`，在一个调用中为整个服务器读取。Veeam仅在备份至少部分成功后创建还原点，所以每个对象的最新点已经是"最后一次成功备份"——此处不在结果上过滤。Veeam不公开与备份本身分开的"最后连接"，所以`last_seen_timestamp_seconds`和`connection`信号从不发布；设备在监控项页面的表中将`last_connection`显示为"——"。设备的`type`是Veeam自己的平台名称（`VSphere`、`WindowsPhysical`、`HyperV`…），按原样显示。

## 监控项页面 {#the-device-page}

备份面板说明有什么问题，显示失败的作业、过去24小时内失败的会话、运行的作业和许可证，然后列出启用的作业——失败优先——及其类型和最后运行，以及仓库及其填充和可用空间。受保护设备的表（名称、平台、最后备份、状态）随后，来自上面的通用家族。

## 为DumbMonit提供Veeam Backup Viewer账户 {#give-dumbmonit-a-veeam-backup-viewer-account}

1. 在备份服务器上，为DumbMonit创建一个本地Windows账户，带有长随机密码，机器上没有其他权限（域账户也有效）。

    ```
    net user dumbmonit * /add
    ```

2. 仍在Veeam控制台上：打开主菜单 > Users and Roles > Security，点击Add，输入账户（例如VBR01\dumbmonit）并仅给它Veeam Backup Viewer角色。

3. 在DumbMonit中，输入备份服务器的地址，例如"vbr.lan"，以及账户为HOST\dumbmonit或DOMAIN\dumbmonit及其密码。REST API在端口9419上监听，带有自签名证书：勾选接受无法验证的证书，除非您安装了自己的。

!!! warning
    Veeam为Backup Administrator角色保留某些REST API路由，许可证在当前版本中属于其中。使用Viewer角色，DumbMonit读取作业、会话、仓库和还原点，并将许可证显示为不可读：它从不请求更多。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名和密码 | 具有Veeam Backup Viewer角色的账户，作为`HOST\dumbmonit`或`DOMAIN\dumbmonit`。 |

地址：备份服务器的主机名或IP、`host:port`或URL。选项：

- **协议**（`https`）和**端口**（9419）：REST API服务的。
- **API版本**（`1.1-rev0`）：作为`x-api-version`发送。仅在服务器拒绝时更改；v13服务器记录`1.3-rev1`。
- **接受无法验证的证书**：REST API自己的证书是自签名的。
- **每个请求的超时**（15秒）。
- **设备陈旧性**（`device_stale_days`，默认3天）：见上面[监控项](#devices)。

## 故障排除 {#troubleshooting}

"Veeam拒绝登录"：用户名需要其主机或域（`VBR01\dumbmonit`），或账户没有Veeam角色。

"Veeam拒绝列出作业（403）"：账户的角色无法读取作业；给它Veeam Backup Viewer。

"Veeam拒绝API版本"：将API版本选项设置为服务器在`https://<server>:9419/swagger`上列出的版本。

许可证显示"不可读"：在当前版本中使用Viewer角色时预期；DumbMonit不请求更多。

无监控项或监控项面板为空：`restore_points`读取"不可读"，可能是因为服务器比路由更旧（12.0之前的任何内容的较早修订）而不是角色问题——还原点是普通库存，不是仅管理员页面。
