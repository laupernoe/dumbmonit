# VMware vSphere {#vmware-vsphere}

一个vCenter Server及其管理的主机，或独立ESXi主机：停止响应、留在维护模式或报告红色健康状态的主机、虚拟机及其VMware Tools、数据存储填满或变得不可访问，以及vSphere本身引发的告警。

DumbMonit使用持有内置**只读**角色的专用用户读取vSphere。它从不关闭VM、从不进入维护模式，也从不确认告警。

值得关注的故障：**数据存储填满**。当精简配置的数据存储达到100%时，每个需要写入的VM立即暂停。vCenter引发的告警也说明了这一点，但仅对有客户端打开的人。

## 为什么使用Web Services API {#why-the-web-services-api}

DumbMonit与vSphere Web Services API（SOAP，`/sdk`）通信，而不是REST API（`/api/vcenter/*`）：独立ESXi主机没有REST API，在vCenter上REST API不告诉维护模式、总体健康（`overallStatus`）、VMware Tools的状态或触发的告警。整个库存的一个视图在单一次通过中逐页读取，然后在末尾释放。

会话从一个探针保存到下一个探针，仅在过期时重新打开：vCenter不会每分钟记录一次新登录。

## 监控内容 {#what-it-watches}

所有指标都以`dumbmonit_vsphere_`为前缀。健康值遵循vSphere的`overallStatus`：0绿色，1黄色，2红色，3灰色（未知）。

| 指标 | 说明 | 标签 |
|---|---|---|
| `info` | 值为1 | `product`（`vcenter`或`esxi`）、`version`、`build` |
| `hosts` | 按连接状态的主机：`connected`、`disconnected`、`notResponding` | `state` |
| `host_connection_state` | 0已连接，1被管理员断开，2无响应 | `esxi` |
| `host_power_state` | 0电源开启，1待机，2电源关闭，3未知 | `esxi` |
| `host_maintenance` | 维护模式时为1 | `esxi` |
| `host_status` | 总体健康 | `esxi` |
| `host_cpu_percent`、`host_memory_percent`、`host_memory_bytes`、`host_uptime_seconds` | 仅已连接的主机 | `esxi` |
| `vms` | 按电源状态的VM：`poweredOn`、`poweredOff`、`suspended`（排除模板） | `state` |
| `templates` | VM模板 | |
| `vm_power_state` | 0电源开启，1电源关闭，2暂停；最多1000个VM | `vm` |
| `vm_status` | 总体健康 | `vm` |
| `vm_tools_status` | 仅电源开启的VM：0最新，1旧版本，2已安装但未运行，3未安装 | `vm` |
| `datastores` | 数据存储 | |
| `datastore_capacity_bytes`、`datastore_free_bytes`、`datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 可访问时为1，不可访问时为0 | `datastore` |
| `datastore_status` | 总体健康 | `datastore` |
| `alarms` | 未确认的触发告警，按颜色：`red`、`yellow` | `status` |
| `alarms_acknowledged` | 已确认的触发告警 | |
| `alarm` | 每个未确认告警一个序列，值=其健康代码，最多64个 | `alarm`（其名称）、`entity`、`entity_type`、`status` |
| `scrape_errors` | 上次探针失败的调用 | |

适用的[内置规则](../alerting/rules.md#vmware-vsphere)：vSphere主机无响应、vSphere主机健康红色、vSphere主机留在维护、vSphere数据存储几乎满、vSphere数据存储不可访问、vSphere VM Tools未运行、vSphere红色告警，加上监控项不可达。

## 监控项页面 {#the-device-page}

图表上方的面板读取探针存储的内容；打开页面从不查询vSphere。它首先说明有什么问题（主机无响应、红色告警、数据存储几乎满或不可访问），然后是具有其负载的主机、具有其填充级别的数据存储，以及按电源状态的VM。

## 验证方式 {#how-it-was-validated}

针对govmomi模拟器`vcsim` 0.56.0，在vCenter和ESXi模式中：登录和拒绝的密码、会话重用和过期、通过库存分页、主机无响应、主机维护模式、关闭的VM和模板。模拟器不引发告警：读取触发的告警针对Web Services API的文档化`AlarmState`结构进行验证。

## 在vSphere中创建只读用户 {#create-a-read-only-user-in-vsphere}

1. 在vCenter上：Administration → Single Sign On → Users and Groups → Users，域vsphere.local → Add。按如下方式命名用户并给它一个长随机密码。

    ```
    dumbmonit
    ```

2. 仍在vCenter上：Administration → Access Control → Global Permissions → Add。选择该用户、只读角色，并勾选传播给子项。只读看到每个主机、VM、数据存储和告警，无法改变任何内容。

3. 在独立ESXi主机上：在主机客户端中，Manage → Security & users → Users → Add user，带相同名称。然后Host → Actions → Permissions → Add user：选择它、只读角色，并勾选传播给所有子项。

4. 在DumbMonit中，输入vCenter或主机的地址，例如"vcenter.lan"或"esxi1.lan"，带如创建的用户名：vCenter上的dumbmonit@vsphere.local，主机上的dumbmonit。当vCenter管理主机时，仅添加vCenter：它报告每个主机、VM和数据存储。

5. DumbMonit仅读取，通过vSphere Web Services API（/sdk）。它从不关闭VM、从不进入维护模式，也从不确认告警。

!!! warning
    不要重用单点登录域账户或主机的内置账户：上面的只读角色是DumbMonit需要的全部。vCenter和ESXi默认使用自签名证书：除非DumbMonit主机信任它们的证书颁发机构，否则启用"接受无法验证的证书"。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名/密码 | 持有只读角色的专用用户：vCenter上的`dumbmonit@vsphere.local`，主机上的`dumbmonit`。 |

地址：主机名或IP（`vcenter.lan`）、`host:port`或完整URL；接受尾部的`/sdk`或`/ui`。选项：端口（443）、证书检查、请求超时（20秒）和触发告警（读取）。
