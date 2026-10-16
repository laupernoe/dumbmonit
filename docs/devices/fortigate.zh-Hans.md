# FortiGate {#fortigate}

通过 FortiOS REST API 监控 Fortinet FortiGate:宕机的站点到站点
IPsec 隧道、丢失链路的已启用接口、失去同步或缺少成员的 HA 集群、
接近节能模式的内存、即将过期的 FortiGuard 和 FortiCare 许可证,
以及当前运行的 FortiOS 分支可用的补丁版本。

DumbMonit 使用一个访问配置档案仅限读取、且只接受来自 DumbMonit
服务器地址(受信主机)调用的 REST API 用户。其令牌以
`Authorization: Bearer` 发送,绝不放在 URL 中:FortiOS 7.4.5 和
7.6.1 无论如何都会拒绝放在 URL 中的令牌。

DumbMonit 每分钟读取三个必需页面——`/api/v2/monitor/system/status`、
`/api/v2/monitor/system/resource/usage` 和
`/api/v2/monitor/system/interface/select`——以及六个可选页面,
一个权限更窄的配置档案,或一台没有集群或隧道的 FortiGate,可能
拒绝或留空这些页面,而不会影响其余部分:接口的管理状态
(`/api/v2/cmdb/system/interface`)、HA 统计信息和校验和、IPsec
隧道、许可证状态和可用固件。

!!! warning "仅依据厂商文档验证——尚未在真实系统上测试"

    这项集成是根据 Fortinet 的 FortiOS 文档,以及社区版
    FortiGate Prometheus 导出器记录的应答结构构建的,而不是针对
    一台正在运行的 FortiGate。每次调用都是读取操作。许可证和
    固件页面的文档记载最少:如果有任何问题,请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_fortigate_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | 固定值 1。 |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | 当前使用率。 |
| `sessions`, `session_setup_rate` | | 活跃会话数,每秒新建会话数。 |
| `log_disk_ok` | | 当日志磁盘需要格式化时为 0。 |
| `interface_link_up` | `interface`, `alias` | 物理链路。 |
| `interface_enabled` | `interface`, `alias` | 管理状态;当该配置档案无法读取配置时,每个接口都计为已启用。 |
| `interface_down` | `interface`, `alias` | 当接口已启用、有地址且没有链路时为 1。没有地址的空闲端口不受影响。 |
| `interface_speed_mbps` | `interface`, `alias` | 链路建立期间。 |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | 计数器。 |
| `interfaces_down` | | 计数。 |
| `ha_members` | | 集群中的成员数;独立 FortiGate 为 0。 |
| `ha_in_sync` | | 两个或更多成员时:当每个成员的配置校验和相同时为 1。 |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | 按成员。 |
| `ipsec_tunnel_up` | `tunnel` | 当该隧道至少有一个 phase 2 建立时为 1。拨入隧道(远程访问)不包含在内。 |
| `ipsec_phase2_up` | `tunnel`, `phase2` | 按 phase 2 选择器。 |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | 计数器。 |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | 固定值 1。 |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | 计数。 |
| `license_status_info` | `license`, `status` | 固定值 1,对于该 FortiGate 持有的每个许可证(`antivirus`、`ips`、`web_filtering`、`forticare_hardware`……);`no_license` 项不包含在内。 |
| `license_expired` | `license` | 当状态为 `expired` 时为 1。 |
| `license_expiry_seconds` | `license` | 距过期的时间;过期后为负数。 |
| `firmware_update_available` | | 当 FortiGuard 为当前分支提供了更新的补丁版本时为 1(7.4.4 → 7.4.5)。新的分支(7.6)属于一个项目,不是一条告警。 |
| `firmware_latest_info` | `version` | 固定值 1:该补丁版本。 |

适用的[内置规则](../alerting/rules.md#fortigate):

- **FortiGate IPsec 隧道中断**:五分钟内没有任何 phase 2 建立
  (警告级别)。
- **FortiGate 接口无链路**:持续五分钟(提示级别)。
- **FortiGate HA 失去同步**:持续十五分钟(提示级别)。
- **FortiGate HA 成员丢失**:成员数少于过去 24 小时内的任何时刻
  (警告级别)。预期的成员数并未在任何地方公布,因此一个被永久
  移除的成员会在第二天停止触发告警。
- **FortiGate 内存接近节能模式**:超过 85% 持续十分钟(提示
  级别)。默认情况下达到 88% 时 FortiOS 会进入节能模式并停止
  检查新会话。
- **FortiGate CPU 过高**:超过 90% 持续十五分钟(提示级别)。
- **FortiGate 许可证即将过期**:剩余不足 30 天,或已过期
  (提示级别,每天提醒)。
- **FortiGate 固件更新**:有补丁版本可用(信息级别,每周
  提醒)。

## 监控项页面 {#the-device-page}

Firewall 面板会用一句话说明哪里出了问题,显示 CPU、内存、会话数
和 HA 状态,然后列出 IPsec 隧道(宕机的排在最前)、无链路的
接口、60 天内即将过期或已过期的许可证,以及可用的补丁版本。

## 为 DumbMonit 创建一个只读的 REST API 用户 {#create-a-read-only-rest-api-user-for-dumbmonit}

1. 从 FortiGate CLI 创建一个只能读取系统、网络和 VPN 状态、不能做其他任何事的访问配置档案。

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. 使用该配置档案创建 REST API 用户 dumbmonit,只信任 DumbMonit 服务器的地址(替换 192.0.2.10),然后生成它的令牌:它只会打印一次。

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. 在 DumbMonit 中,输入 FortiGate 的地址,例如 "fw.lan"(如果管理界面不监听 443 端口,请加上端口),并将该令牌粘贴为 API 令牌。FortiGate 自带自签名证书:除非你安装了自己的证书,否则请勾选"接受无法验证的证书"。

!!! warning "警告"
    只有当某个接口已启用、有地址且没有链路时,才会被计为宕机:
    空闲端口不受影响。固件更新指的是 FortiGuard 提供给该
    FortiGate 本身的更新:无法访问 FortiGuard 时不会报告任何
    更新。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| API 令牌 | 由 `execute api-user generate-key dumbmonit` 打印出的令牌,以 `Authorization: Bearer` 发送。 |

地址:FortiGate 的主机名或 IP、`host:port`,或 URL。选项:

- **端口**(443):管理界面的 HTTPS 端口(`config system global`
  下的 `set admin-sport`)。
- **VDOM**:要读取的虚拟域;留空表示该 REST API 用户所属的那个。
- **接受无法验证的证书**:FortiGate 自身的证书是自签名的。
- **每次请求的超时时间**(15 秒)。

## 故障排查 {#troubleshooting}

"FortiGate refused the API token (401)":令牌错误或已被重新
生成。用 `execute api-user generate-key dumbmonit` 生成一个新的。

"FortiGate refused … (403)":DumbMonit 的地址不在该用户的受信
主机列表中,或其配置档案无法读取该页面。以这种方式被拒绝的可选
页面只会被跳过。

没有任何 IPsec、许可证或固件数据:该配置档案无法读取它们,或该
FortiGate 没有任何隧道;其余内容仍会被正常读取。
