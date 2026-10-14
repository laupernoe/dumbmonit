# Nutanix AHV {#nutanix-ahv}

通过 Prism Central 的 v4 REST API 读取的 Nutanix 集群。一个 Prism Central 覆盖在其上注册的所有集群。

DumbMonit 每分钟在 9440 端口上使用 HTTP 基本认证读取：

| 内容 | 路径 |
|---|---|
| 集群 | `/api/clustermgmt/v4.0/config/clusters` |
| 主机 | `/api/clustermgmt/v4.0/config/hosts` |
| AHV 虚拟机 | `/api/vmm/v4.0/ahv/config/vms` |
| 存储容器 | `/api/clustermgmt/v4.0/config/storage-containers` |
| 每个容器的填充水平 | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| 未解决的告警 | `/api/monitoring/v4.0/serviceability/alerts` |

列表按每页 100 项读取，每个列表最多 1,000 项。填充水平和告警是可选的：如果账户的角色拒绝读取它们，其余内容仍会被收集。Nutanix 已宣布弃用 v1、v2.0 和 v3 API；只有 v4 仍在持续演进，并且需要 Prism Central pc.2024.3 或更高版本。

!!! warning "仅依据厂商文档验证——尚未在真实系统上测试"

    这项集成是根据 Nutanix 的 v4 API 规范（路径和字段以官方生成的客户端所定义的为准）构建的，并针对录制的响应进行了测试，而不是针对一个正在运行的 Prism Central。如果有任何问题，请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_nutanix_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `clusters` | | 集群数量，不含 Prism Central 本身。 |
| `cluster_info` | `cluster`、`version` | 值为 1：每个集群的 AOS 版本。 |
| `cluster_available` | `cluster` | Prism Central 能连到该集群时为 1。 |
| `hosts` | | 所有集群的主机数量。 |
| `host_info` | `host`、`cluster`、`hypervisor` | 值为 1：每台主机运行的虚拟机监控程序。 |
| `host_cpu_cores`、`host_memory_bytes` | `host`、`cluster` | 容量。 |
| `host_degraded` | `host`、`cluster` | Nutanix 将该主机标记为降级时为 1。 |
| `host_maintenance` | `host`、`cluster` | 主机正在进入或处于维护模式时为 1。 |
| `vms`、`vms_running` | | AHV 虚拟机数量，以及其中已开机的数量。 |
| `vm_power_state_info` | `vm`、`cluster`、`state` | 值为 1：`on`、`off`、`paused` 或 `undetermined`。 |
| `vm_running` | `vm`、`cluster` | 虚拟机已开机时为 1。 |
| `vm_vcpus`、`vm_memory_bytes` | `vm`、`cluster` | 插槽 × 核心 × 线程，以及配置的内存。 |
| `storage_containers` | | 存储容器数量，不含 Nutanix 内部的容器。 |
| `storage_container_capacity_bytes`、`storage_container_used_bytes`、`storage_container_used_percent` | `container`、`cluster` | 填充水平，取自最近三十分钟内的最新统计数据。 |
| `alerts_unresolved`、`alerts_critical`、`alerts_warning` | | 未解决的告警数量，总数及按严重程度划分。 |

适用的[内置规则](../alerting/rules.md#nutanix)：

- **Nutanix 主机降级**：某台主机被标记为降级已五分钟（警告）。
- **Nutanix 存储容器即将满**：某个容器连续十五分钟已满 90%（通知）。
- **Nutanix 严重告警**：Prism Central 报告至少一条未解决的严重告警已五分钟（警告）。

## 在 Prism Central 中创建只读用户 {#create-a-read-only-user-in-prism-central}

1. 在 Prism Central 中，打开 Admin Center > IAM > Identities，添加一个带有长随机密码的本地用户，名称如下。

    ```
    dumbmonit
    ```

2. 在 Admin Center > IAM > Authorization Policies 下，创建一条使用内置 Prism Viewer 角色、适用于所有实体的策略，并将其分配给该用户。Prism Viewer 可读取集群、主机、虚拟机、存储容器和告警，不会更改任何内容。

3. 在 DumbMonit 中，输入 Prism Central 的地址，例如 "prism-central.lan"，以及该用户名和密码。一个 Prism Central 会报告在其上注册的所有集群。

4. DumbMonit 只读取 v4 API（clustermgmt、vmm 和 monitoring，版本 4.0）：它从不开机或关机任何虚拟机，从不解决告警，也从不更改存储容器。

!!! warning "警告"
    v4 API 由 Prism Central pc.2024.3 或更高版本提供；仅由 Prism Element 管理或由较旧 Prism Central 管理的集群不受支持。Prism Central 自带自签名证书：除非 DumbMonit 主机信任其证书颁发机构，否则请勾选"接受无法验证的证书"。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名和密码 | 具有 Prism Viewer 角色的本地用户 `dumbmonit`。 |

地址：Prism Central 的主机名或 IP、`host:port` 或 URL。选项：

- **端口**（9440）。
- **接受无法验证的证书**：Prism Central 自己的证书是自签名的。
- **每个请求的超时**（15 秒）。

## 故障排查 {#troubleshooting}

"Nutanix Prism Central refused the credentials … (401)"：用户名或密码错误。

"Nutanix Prism Central has no /api/clustermgmt/v4.0/…"：该地址是 Prism Element，或是早于 pc.2024.3 的 Prism Central。

没有 `storage_container_used_percent`：该角色不允许读取统计数据；容器仍会被计入，使用其配置的容量。
