# XCP-ng / XenServer {#xcp-ng-xenserver}

通过 XAPI 读取的 XCP-ng 或 XenServer（Citrix Hypervisor）池。XAPI 是每台主机本来就在运行的管理 API，无需 Xen Orchestra。

DumbMonit 每分钟向 `https://<协调器>/jsonrpc` 发送 JSON-RPC 调用：一次 `session.login_with_password`（会话在两次轮询之间保留，若池使其失效则重新打开一次），以及每个类各一次 `get_all_records`：`host`、`host_metrics`、`VM`、`VM_metrics`、`SR` 和 `pool`。不会更改任何内容：该账户拥有内置的 `read-only` 角色，XAPI 会在每次调用时强制执行。

只需按协调器的地址添加一次池：它会报告所有主机、虚拟机和存储库。如果你输入的地址是池成员，它会以协调器的地址回应 `HOST_IS_SLAVE`；DumbMonit 会按相同的协议和端口跟随一次，因此更换协调器时无需编辑监控项。

!!! warning "仅依据厂商文档验证——尚未在真实系统上测试"

    这项集成是根据 XAPI 文档（JSON-RPC 协议、`host`、`VM`、`SR` 和 `pool` 类）构建的，并针对录制的响应进行了测试，而不是针对一个正在运行的池。如果有任何问题，请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_xcpng_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `hosts`、`hosts_enabled` | | 池中的主机数，以及其中接受新虚拟机的数量。 |
| `host_info` | `host`、`version` | 值为 1：每台主机的产品版本。 |
| `host_enabled` | `host` | 主机被禁用（维护或已疏散）时为 0。 |
| `host_live` | `host` | 主机的心跳到达协调器时为 1，离线时为 0。 |
| `host_cpu_cores`、`host_memory_total_bytes`、`host_memory_used_bytes` | `host` | 容量和已用内存。 |
| `vms`、`vms_running` | | 虚拟机数量，不含模板、快照和控制域。 |
| `vm_power_state_info` | `vm`、`state` | 值为 1：`running`、`halted`、`paused` 或 `suspended`。 |
| `vm_running` | `vm` | 虚拟机正在运行时为 1。 |
| `vm_vcpus`、`vm_memory_bytes` | `vm` | 启动时的 vCPU 数；运行中的虚拟机为已用内存，否则为其配置的最大值。 |
| `vm_ha_protected_down` | `vm` | 仅针对设置了重启优先级、且所在池启用了高可用的虚拟机：虚拟机已停止时为 1。 |
| `srs` | | 存储库数量。 |
| `sr_size_bytes`、`sr_used_bytes`、`sr_used_percent` | `sr` | 填充水平（ISO 库会计入，但不展开）。 |
| `sr_shared` | `sr` | 共享存储库（NFS、iSCSI……）为 1。 |
| `pool_info` | `pool`、`master` | 值为 1：池及其当前协调器。 |
| `pool_ha_enabled` | `pool` | 高可用启用时为 1。 |
| `pool_ha_overcommitted` | `pool` | 启用 HA 时：池在其设定可容忍的主机故障之后无法再重启受保护的虚拟机时为 1。 |
| `pool_ha_host_failures_to_tolerate` | `pool` | HA 设定可承受的主机故障数量。 |

XenAPI 没有“已崩溃”的电源状态。发现受保护的虚拟机处于停止状态，是 API 能给出的最接近“虚拟机违背你的意愿而停止”的信号；如果你有意关闭一台受保护的虚拟机，请先清除它的重启优先级。

适用的[内置规则](../alerting/rules.md#xcp-ng-xenserver)：

- **XCP-ng 主机离线**：某台主机的心跳已五分钟未到达协调器（警告）。
- **XCP-ng SR 即将满**：某个存储库连续十五分钟已满 90%（通知）。
- **XCP-ng HA 保护的虚拟机宕机**：一台受高可用保护的虚拟机已停止两分钟（警告）。
- **XCP-ng 池 HA 超额分配**：池已超额分配五分钟（通知）。

## 给 DumbMonit 一个只读的 XAPI 账户 {#give-dumbmonit-a-read-only-xapi-account}

1. 只有当池使用外部认证时，XAPI 才接受其内置超级用户之外的账户。如果池已加入 Active Directory，请在其中创建一个 dumbmonit 账户，然后跳到步骤 3。

2. 否则，在池的每台主机上创建一个带有长随机密码的本地账户，然后在池协调器上一次性启用 PAM 认证。在被授予角色之前，该账户什么都无法调用。

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. 在池协调器上，将该账户注册为 subject，然后给这个 subject 授予内置的 read-only 角色，使用第一条命令打印出的 UUID。

    ```
    xe subject-add subject-name=dumbmonit
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

4. 在 DumbMonit 中，输入池协调器的地址，例如 "xcp01.lan"，以及该用户名和密码。只需添加一次池：协调器会报告所有主机、虚拟机和存储库。池成员会以协调器的地址作答，DumbMonit 随后会跟随该地址。

5. DumbMonit 只通过 XAPI 的 JSON-RPC 接口（/jsonrpc）读取：它从不启动、停止或迁移虚拟机，也从不更改存储库或池的高可用。

!!! warning "警告"
    切勿把主机的超级用户密码交给 DumbMonit：使用 read-only 角色，泄露的密码也无法更改任何内容。XCP-ng 和 XenServer 自带自签名证书：除非 DumbMonit 主机信任池所出示的证书，否则请勾选"接受无法验证的证书"。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名和密码 | `dumbmonit` 账户，已注册为 subject 并具有 `read-only` 角色。 |

地址：池协调器的主机名或 IP、`host:port` 或 URL。选项：

- **端口**（443）。
- **接受无法验证的证书**：主机自己的证书是自签名的。
- **每个请求的超时**（15 秒）。

## 故障排查 {#troubleshooting}

"XCP-ng refused the user name or password"：密码错误，或该账户未注册为 subject（`xe subject-list`）。

"XCP-ng accepted the account but refused the call"：该 subject 没有角色，或角色没有读取权限；请给它 `read-only`。

"This host is a pool member, not the coordinator"：DumbMonit 无法连到该成员指出的协调器；请自行输入协调器的地址。
