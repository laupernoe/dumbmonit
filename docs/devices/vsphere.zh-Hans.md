# VMware vSphere

一台 vCenter Server 及其管理的主机,或一台独立的 ESXi 主机:停止
应答的主机、被遗留在维护模式中的主机或报告红色健康状态的主机、
虚拟机及其 VMware Tools、空间不足或变得不可访问的数据存储,以及
vSphere 自身触发的告警。

DumbMonit 使用一个持有内置**只读**角色的专用用户读取 vSphere。
它从不开关任何虚拟机电源,从不进入维护模式,也从不确认任何告警。

值得关注的故障是:**数据存储空间耗尽**。当一个精简置备的数据
存储达到 100% 时,所有需要向它写入数据的虚拟机会立即暂停。
vCenter 触发的告警也会说明这一点,但只有打开了客户端的人才能
看到。

## 为什么使用 Web Services API {#why-the-web-services-api}

DumbMonit 使用的是 vSphere Web Services API(SOAP,`/sdk`),而
不是 REST API(`/api/vcenter/*`):独立的 ESXi 主机没有 REST
API,而在 vCenter 上,REST API 不会告知维护模式、整体健康状况
(`overallStatus`)、VMware Tools 的状态或已触发的告警。整个
清单会在一次遍历中、逐页读取完整视图,并在结束时释放。

会话会在两次探测之间保留,只有在过期时才重新打开:vCenter 不会
因此每分钟记录一次新的登录。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_vsphere_` 为前缀。健康状态值遵循
vSphere 的 `overallStatus`:0 绿色,1 黄色,2 红色,3 灰色(未知)。

| 指标 | 含义 | 标签 |
|---|---|---|
| `info` | 固定值 1 | `product`(`vcenter` 或 `esxi`)、`version`、`build` |
| `hosts` | 按连接状态划分的主机数:`connected`、`disconnected`、`notResponding` | `state` |
| `host_connection_state` | 0 已连接,1 被管理员断开,2 未响应 | `host` |
| `host_power_state` | 0 开机,1 待机,2 关机,3 未知 | `host` |
| `host_maintenance` | 处于维护模式时为 1 | `host` |
| `host_status` | 整体健康状况 | `host` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | 仅限已连接的主机 | `host` |
| `vms` | 按电源状态划分的虚拟机数:`poweredOn`、`poweredOff`、`suspended`(不含模板) | `state` |
| `templates` | 虚拟机模板数 | |
| `vm_power_state` | 0 开机,1 关机,2 挂起;最多 1000 台虚拟机 | `vm` |
| `vm_status` | 整体健康状况 | `vm` |
| `vm_tools_status` | 仅限开机的虚拟机:0 已是最新版本,1 版本过旧,2 已安装但未运行,3 未安装 | `vm` |
| `datastores` | 数据存储数量 | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 可访问,0 不可访问 | `datastore` |
| `datastore_status` | 整体健康状况 | `datastore` |
| `alarms` | 已触发但未确认的告警,按颜色划分:`red`、`yellow` | `status` |
| `alarms_acknowledged` | 已触发且已确认的告警数 | |
| `alarm` | 每个未确认告警对应一条序列,值为其健康代码,最多 64 个 | `alarm`(其名称)、`entity`、`entity_type`、`status` |
| `scrape_errors` | 上次探测中失败的调用 | |

适用的[内置规则](../alerting/rules.md#vmware-vsphere):vSphere
主机未响应、vSphere 主机健康状态红色、vSphere 主机遗留在维护
模式中、vSphere 数据存储即将用尽、vSphere 数据存储不可访问、
vSphere 虚拟机 Tools 未运行、vSphere 红色告警,以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询
vSphere。它会先说明哪里出了问题(未响应的主机、红色告警、几乎
用尽或不可访问的数据存储),然后是各主机及其负载、各数据存储
及其填充程度,以及按电源状态划分的虚拟机。

## 验证方式 {#how-it-was-validated}

针对 govmomi 模拟器 `vcsim` 0.56.0,在 vCenter 模式和 ESXi 模式
下均进行了验证:登录和密码拒绝、会话复用和过期、分页遍历清单、
未响应的主机、处于维护模式的主机、已关机的虚拟机和模板。该模拟器
不会触发任何告警:读取已触发告警的功能是依据 Web Services API
文档中记载的 `AlarmState` 结构验证的。

## 在 vSphere 中创建一个只读用户 {#create-a-read-only-user-in-vsphere}

1. 在 vCenter 上:Administration → Single Sign On → Users and Groups → Users,域 vsphere.local → Add。按如下方式命名该用户,并设置一个较长的随机密码。

    ```
    dumbmonit
    ```

2. 仍在 vCenter 上:Administration → Access Control → Global Permissions → Add。选择该用户、Read-only 角色,并勾选 Propagate to children。Read-only 能看到每一台主机、虚拟机、数据存储和告警,且不能更改任何内容。

3. 对于独立的 ESXi 主机:在主机客户端中,Manage → Security & users → Users → Add user,使用同样的名称。然后 Host → Actions → Permissions → Add user:选择该用户、Read-only 角色,并勾选 Propagate to all children。

4. 在 DumbMonit 中,输入 vCenter 或主机的地址,例如 "vcenter.lan" 或 "esxi1.lan",使用创建时的用户名:在 vCenter 上是 dumbmonit@vsphere.local,在主机上是 dumbmonit。当 vCenter 管理这些主机时,只需添加 vCenter:它会报告所有主机、虚拟机和数据存储。

5. DumbMonit 只会通过 vSphere Web Services API(/sdk)读取数据。它从不开关任何虚拟机电源,从不进入维护模式,也从不确认任何告警。

!!! warning "警告"
    不要重复使用单点登录域账户或主机的内置账户:上面的
    Read-only 角色就是 DumbMonit 所需要的全部权限。vCenter 和
    ESXi 默认使用自签名证书:除非 DumbMonit 所在主机信任它们的
    证书颁发机构,否则请启用"接受无法验证的证书"。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名/密码 | 拥有 Read-only 角色的专用用户:在 vCenter 上是 `dumbmonit@vsphere.local`,在主机上是 `dumbmonit`。 |

地址:主机名或 IP(`vcenter.lan`)、`host:port`,或完整 URL;
结尾的 `/sdk` 或 `/ui` 也会被接受。选项:端口(443)、证书校验、
请求超时(20 秒)和已触发的告警(读取)。
