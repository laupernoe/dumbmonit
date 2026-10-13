# UniFi网络 {#unifi-network}

UniFi网络及其管理的内容：离线、隔离或等待采纳的网关、交换机和接入点、等待的固件更新、WAN链接和网关看到的互联网连接、客户端数量以及UniFi引发的告警。它适用于UniFi OS控制台（Dream Machine、Cloud Gateway、Cloud Key）的UniFi Network应用程序和自托管的UniFi Network Server。

DumbMonit使用API密钥或持有**仅查看**角色的本地账户读取UniFi。它从不重启、采纳、升级或配置设备。

值得关注的故障：**接入点或交换机悄无声息地掉线**。客户端漫游到下一个接入点，网络保持工作，覆盖盲点或丢失的上行链路直到几天后才被注意到。同样的情况也适用于第二条WAN链路宕机：在第一条也失败之前一切正常。

## 两种方式 {#two-ways-in}

| 凭据 | 使用的API | 读取内容 |
|---|---|---|
| API密钥（UniFi OS控制台） | 官方集成API，在控制台上位于`/proxy/network/integration/v1`，在自托管服务器上位于`/integration/v1` | 版本、设备及其状态、固件更新、每个在线设备的负载、内存和正常运行时间、客户端数量 |
| 仅查看账户 | 经典API（`/api/s/<site>/stat/*`），在控制台上位于`/proxy/network/api/…` | 以上所有内容，加上每个设备的客户端和按类型分类、WAN链接、WAN和互联网状态以及告警 |

WAN、互联网和告警状态仅存在于经典API中。使用密钥时，DumbMonit也向经典API询问WAN和互联网状态，使用相同的密钥；如果控制器拒绝它，这些序列仅仅缺失，不会出现错误。

告警是UniFi"关键"系统日志在过去24小时内的条目（UniFi Network 8及更高版本），或旧版本上未归档的告警。这是DumbMonit发送的唯一`POST`，它仅读取。

使用仅查看账户打开的会话从一个探针保存到下一个探针，并在过期时重新打开：UniFi不会每分钟记录一次新登录。

## 监控内容 {#what-it-watches}

所有指标都以`dumbmonit_unifi_`为前缀。设备序列携带`device`（其名称，或其MAC地址如果没有名称）、`mac`、`model`和`type`（`gateway`、`switch`、`access_point`、`other`）。

| 指标 | 说明 | 标签 |
|---|---|---|
| `info` | 值为1 | `version`、`api`（`integration`或`classic`） |
| `devices` | 按状态分类的设备：`online`、`offline`、`pending`、`updating`、`adopting`、`adoption_failed`、`isolated`、`other` | `state` |
| `devices_upgradable` | 有固件更新可用的已采纳设备 | |
| `device_state` | 0在线，1离线，2待采纳，3更新或配置中，4采纳中，5采纳失败，6隔离，7其他 | 设备标签 |
| `device_up` | 在线或更新时为1，离线、隔离或采纳失败时为0；未采纳的设备无此序列 | 设备标签 |
| `device_upgradable` | 有固件更新可用时为1 | 设备标签、`firmware` |
| `device_uptime_seconds`、`device_cpu_percent`、`device_memory_percent` | 仅在线设备 | 设备标签 |
| `device_clients` | 连接到设备的客户端（仅查看账户） | 设备标签 |
| `wan_link_up` | 网关的WAN端口正常时为1；禁用的端口没有序列 | 设备标签、`wan`（`wan1`、`wan2`） |
| `wan_up`、`internet_up` | 网关的WAN和互联网状态；没有UniFi网关时不存在 | |
| `internet_latency_seconds` | 网关测量的延迟 | |
| `wan_availability_percent` | 过去24小时内每条WAN的可用性 | `wan` |
| `subsystem_status` | 0正常，1警告，2错误，3未知，用于`wlan`、`wan`、`www`、`lan`、`vpn` | `subsystem` |
| `clients` | 站点上的客户端 | |
| `clients_by_type` | `wireless`、`wired`、`guest`、`vpn`（仅查看账户） | `type` |
| `alarms` | 过去24小时内的关键系统日志条目，或未归档的告警 | |
| `scrape_errors` | 上次探针失败的调用 | |

适用的[内置规则](../alerting/rules.md#unifi-network)：UniFi设备离线、UniFi互联网宕机、UniFi WAN链路宕机、UniFi网关CPU高、UniFi设备待采纳、UniFi固件更新可用、UniFi告警，加上设备不可达。

## 设备 {#devices}

`/rest/user`（经典API，使用任一凭据读取）列出控制器曾见过的每个客户端——实际上是大多数家庭网络：经过的手机、访客Wi-Fi、永远不会回来的智能设备。跟踪所有这些以确定陈旧性会在未返回的设备告警中淹没，而不是无声设备。默认情况下，只有用户以某种方式标记的客户端才计数：在UniFi中命名、保留在固定IP上或注明。**监控除标记的客户端外的其他客户端**选项添加特定的MAC地址或客户端名称，或`all`跟踪每个客户端（不建议在除小型、大多数静态网络外的任何地方）。

被跟踪的客户端在跟踪客户端设备的每个DumbMonit集成共享的通用家族下发布——`dumbmonit_client_device_*`，前缀中不含`unifi`，在[客户端设备](../alerting/rules.md#client-devices)下完整记录：

| 指标 | 说明 | 标签 |
|---|---|---|
| `last_seen_timestamp_seconds` | 最后连接，如`/rest/user`报告 | `device`、`type`（`Wired`、`Wireless`或`Unknown`）、`os`（空）、`user`（空）、`kind` |
| `stale_seconds` | 一旦被跟踪的客户端超过`device_stale_days`（默认3）而不连接就为正数；驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`、`type`、`os`、`user`、`kind`、`signal`（始终为`connection`：UniFi没有备选方案） |

## 设备页面 {#the-device-page}

图表上方的面板读取探针存储的内容；打开页面从不查询UniFi。它首先说明有什么问题（离线设备、互联网或WAN链路宕机、告警），然后是按类型分类的设备及其状态、固件和负载，以及客户端，然后是被跟踪客户端的表，显示它们的最后连接。

## 验证方式 {#how-it-was-validated}

针对UniFi Network 10.6.106自托管版本检查了登录、仅查看角色、错误响应（拒绝的密码、未知站点、拒绝的密钥、过期的会话）和没有设备的控制器的响应。没有设备可以在那里采纳：设备列表、装备站点的健康状态、已知客户端（`/rest/user`）和集成API有效载荷根据官方文档进行验证。

## 在UniFi Network中创建只读访问 {#create-a-read-only-access-in-unifi-network}

1. 在UniFi OS控制台（Dream Machine、Cloud Gateway、Cloud Key）上，您可以创建API密钥：在UniFi Network中，Settings → Control Plane → Integrations → Create API Key。以DumbMonit命名并现在复制：它只显示一次。密钥通过官方集成API读取设备、其负载和客户端数量。

2. 对于WAN、互联网和告警状态，以及在自托管的UniFi Network Server上，改用具有仅查看角色的本地账户：Settings → Admins & Users → Create New。按如下名称命名，给它一个长随机密码，限制它只能本地访问，并将其UniFi Network角色设置为仅查看。仅查看可读取所有内容并无法改变任何内容。

    ```
    dumbmonit
    ```

3. 在DumbMonit中，输入控制台或服务器的地址，例如"unifi.lan"（控制台上的端口443）或"unifi.lan:8443"（自托管服务器），并粘贴密钥或账户。如果网络不是默认站点，请将Site选项设置为地址栏中/manage/或/network/后显示的短名称。

4. DumbMonit仅读取。它从不重启、采纳、升级或配置设备，也从不确认告警。

!!! warning
    不要使用所有者账户或UI.com云账户：泄露的密码会打开链接到它的每个控制台。仅对此本地仅查看账户关闭两因素认证，因为监控服务器无法输入代码。控制台默认使用自签名证书：除非您安装了自己的证书，否则启用"接受无法验证的证书"。UniFi知道曾连接的每个客户端，大多数都是经过的手机：默认情况下，只有命名的、保留在固定IP上或在UniFi中注明的客户端才被监视以确定陈旧性，以避免每个访客手机的告警。在客户端监控选项中列出更多，或输入all。

## 控制器未设置 {#controller-not-set-up-yet}

自托管控制器的第一次运行向导未完成会向每个请求回复重定向到`/setup`。DumbMonit然后报告"UniFi控制器未设置"作为配置错误（不是拒绝的密码）：在浏览器中完成向导，然后再次探针。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| API密钥（UniFi OS控制台） | 在Settings → Control Plane → Integrations中创建的密钥，作为`X-API-KEY`发送。 |
| 仅查看账户 | 本地仅查看账户的用户名和密码。 |

地址：主机名或IP（`unifi.lan`）、`host:port`（自托管服务器为`unifi.lan:8443`）或完整URL；从浏览器复制的地址，带`/manage/…`或`/network/…`被接受。选项：站点（`default`）、端口（443）、证书检查、请求超时（15秒）、除标记外要监控的客户端（空或`all`）以及设备陈旧性（`device_stale_days`，默认3天）。
