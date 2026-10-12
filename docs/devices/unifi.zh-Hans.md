# UniFi Network

UniFi 网络及其管理的设备:离线、被隔离或等待接入的网关、交换机
和接入点,等待安装的固件更新,网关视角下的 WAN 链路和互联网
连接,客户端数量,以及 UniFi 触发的告警。它适用于 UniFi OS
控制台(Dream Machine、Cloud Gateway、Cloud Key)上的 UniFi
Network 应用,以及自托管的 UniFi Network Server。

DumbMonit 使用一个 API 密钥,或一个持有**仅查看**角色的本地账户
读取 UniFi。它从不重启、接入、升级或配置任何设备。

值得关注的故障:**一个接入点或交换机悄悄掉线**。客户端会漫游到
下一个接入点,网络仍能继续工作,而这个覆盖盲区或丢失的上行链路
往往要几天后才被注意到。第二条 WAN 链路宕机也是同样的道理:
直到第一条也出问题之前,一切看起来都正常。

## 两种接入方式 {#two-ways-in}

| 凭据 | 使用的 API | 读取内容 |
|---|---|---|
| API 密钥(UniFi OS 控制台) | 官方 Integration API,在控制台上为 `/proxy/network/integration/v1`,在自托管服务器上为 `/integration/v1` | 版本、设备及其状态、固件更新、每台在线设备的负载、内存和运行时间、客户端数量 |
| 仅查看账户 | 经典 API(`/api/s/<site>/stat/*`),在控制台上为 `/proxy/network/api/…` | 以上全部,加上按设备和类型划分的客户端、WAN 链路、WAN 和互联网状态,以及告警 |

WAN、互联网和告警状态只存在于经典 API 中。使用密钥时,DumbMonit
也会用同一个密钥尝试向经典 API 查询 WAN 和互联网状态;如果
控制器拒绝,这些序列就会直接缺失,不会报错。

告警是 UniFi 最近 24 小时内"critical"系统日志中的条目
(UniFi Network 8 及更高版本),或在较旧版本上是未归档的告警。
这是 DumbMonit 发送的唯一一次 `POST`,它也只是用来读取数据。

使用仅查看账户打开的会话会在两次探测之间保留,只在过期时才重新
打开:UniFi 不会因此每分钟记录一次新的登录。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_unifi_` 为前缀。设备序列带有 `device`
(其名称,没有名称时为其 MAC 地址)、`mac`、`model` 和 `type`
(`gateway`、`switch`、`access_point`、`other`)。

| 指标 | 含义 | 标签 |
|---|---|---|
| `info` | 固定值 1 | `version`, `api`(`integration` 或 `classic`) |
| `devices` | 按状态划分的设备数:`online`、`offline`、`pending`、`updating`、`adopting`、`adoption_failed`、`isolated`、`other` | `state` |
| `devices_upgradable` | 已接入且有固件更新可用的设备数 | |
| `device_state` | 0 在线,1 离线,2 等待接入,3 更新或配置中,4 正在接入,5 接入失败,6 已隔离,7 其他 | 设备标签 |
| `device_up` | 在线或更新中为 1,离线、隔离或接入失败为 0;尚未接入的设备不存在该序列 | 设备标签 |
| `device_upgradable` | 当有固件更新可用时为 1 | 设备标签, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | 仅限在线设备 | 设备标签 |
| `device_clients` | 连接到该设备的客户端数(仅查看账户) | 设备标签 |
| `wan_link_up` | 当网关的某个 WAN 端口处于已连接状态时为 1;已禁用的端口没有该序列 | 设备标签, `wan`(`wan1`、`wan2`) |
| `wan_up`, `internet_up` | 网关的 WAN 和互联网状态;没有 UniFi 网关时不存在 | |
| `internet_latency_seconds` | 网关测量的延迟 | |
| `wan_availability_percent` | 过去 24 小时内每条 WAN 的可用性 | `wan` |
| `subsystem_status` | 0 正常,1 警告,2 错误,3 未知,针对 `wlan`、`wan`、`www`、`lan`、`vpn` | `subsystem` |
| `clients` | 该站点上的客户端数 | |
| `clients_by_type` | `wireless`、`wired`、`guest`、`vpn`(仅查看账户) | `type` |
| `alarms` | 过去 24 小时内的关键系统日志条目,或未归档的告警 | |
| `scrape_errors` | 上次探测中失败的调用 | |

适用的[内置规则](../alerting/rules.md#unifi-network):UniFi 设备
离线、UniFi 互联网中断、UniFi WAN 链路中断、UniFi 网关 CPU 过高、
UniFi 设备等待接入、UniFi 固件有可用更新、UniFi 告警,以及设备
不可达。

## 设备 {#devices}

`/rest/user`(经典 API,任一凭据均可读取)会列出控制器曾经见过的
每一个客户端——实际上几乎是整个家庭网络:路过的手机、访客 Wi-Fi、
再也不会出现的智能设备。如果为所有这些设备都追踪陈旧状态,告警
就会被那些本来就不会再出现的设备淹没,而不是真正掉线的设备。
默认情况下,只有被用户以某种方式标记过的客户端才会被计数:在
UniFi 中命名过、在固定 IP 上保留过,或被标注过的客户端。
**除已标记客户端外还要监控的客户端**选项可以添加特定的 MAC
地址或客户端名称,或填 `all` 以追踪每一个客户端(除非是一个
较小且基本静态的网络,否则不建议这样做)。

被追踪的客户端会以每个追踪客户端设备的 DumbMonit 集成共享的
通用指标族发布——`dumbmonit_client_device_*`,前缀中不含
`unifi`,完整文档见
[客户端设备](../alerting/rules.md#client-devices):

| 指标 | 含义 | 标签 |
|---|---|---|
| `last_seen_timestamp_seconds` | 上次连接,与 `/rest/user` 报告的一致 | `device`, `type`(`Wired`、`Wireless` 或 `Unknown`)、`os`(空)、`user`(空)、`kind` |
| `stale_seconds` | 当某个被追踪的客户端超过 `device_stale_days`(默认 3 天)未连接时为正值;驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`, `type`, `os`, `user`, `kind`, `signal`(始终为 `connection`:UniFi 没有备份可报告) |

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 UniFi。
它会先说明哪里出了问题(设备离线、互联网或某条 WAN 链路中断、
告警),然后是按类型划分的设备及其状态、固件和负载,以及客户端
情况,再然后是被追踪客户端及其上次连接时间的表格。

## 验证方式 {#how-it-was-validated}

登录、仅查看角色、错误应答(密码被拒绝、未知站点、密钥被拒绝、
会话过期)以及一个没有设备的控制器的应答,都已在自托管的
UniFi Network 10.6.106 上验证。那里没有设备可以被接入:设备
列表、一个已配置站点的健康状况、已知客户端(`/rest/user`)和
Integration API 的负载内容是依据官方文档验证的。

## 在 UniFi Network 中创建一个只读访问权限 {#create-a-read-only-access-in-unifi-network}

1. 在 UniFi OS 控制台(Dream Machine、Cloud Gateway、Cloud Key)上,你可以创建一个 API 密钥:在 UniFi Network 中,Settings → Control Plane → Integrations → Create API Key。以 DumbMonit 命名它,并立即复制它:它只会显示一次。该密钥通过官方 Integration API 读取设备、它们的负载和客户端数量。

2. 若要获取 WAN、互联网和告警状态,以及在自托管 UniFi Network Server 上,请改用一个持有仅查看角色的本地账户:Settings → Admins & Users → Create New。按如下方式命名,设置一个较长的随机密码,将其限制为本地访问,并把其 UniFi Network 角色设为仅查看。仅查看能读取一切,不能更改任何内容。

    ```
    dumbmonit
    ```

3. 在 DumbMonit 中,输入控制台或服务器的地址,例如 "unifi.lan"(控制台上端口为 443),或自托管服务器的 "unifi.lan:8443",并粘贴该密钥或填入该账户。如果网络不是默认站点,请将"站点"选项设为地址栏中 /manage/ 或 /network/ 之后显示的短名称。

4. DumbMonit 只会读取。它从不重启、接入、升级或配置任何设备,也从不确认任何告警。

!!! warning "警告"
    不要使用所有者账户或 UI.com 云账户:泄露的密码会打开与它
    关联的每一个控制台。请只为这个本地的仅查看账户关闭两步
    验证,因为监控服务器无法输入验证码。控制台默认使用自签名
    证书:除非你安装了自己的证书,否则请启用"接受无法验证的
    证书"。UniFi 知道每一个曾经连接过的客户端,其中大多数只是
    路过的手机:默认情况下,只有被命名、在固定 IP 上保留,或在
    UniFi 中被标注过的客户端才会被追踪陈旧状态,以避免为每个
    访客的手机触发告警。可在"要监控的客户端"选项中列出更多,
    或输入 all。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| API 密钥(UniFi OS 控制台) | 在 Settings → Control Plane → Integrations 中创建的密钥,以 `X-API-KEY` 发送。 |
| 仅查看账户 | 该本地仅查看账户的用户名和密码。 |

地址:主机名或 IP(`unifi.lan`)、`host:port`(自托管服务器为
`unifi.lan:8443`),或完整 URL;从浏览器复制的带 `/manage/…` 或
`/network/…` 的地址也会被接受。选项:站点(`default`)、端口
(443)、证书校验、请求超时(15 秒)、除已标记客户端外还要监控
的客户端(默认为空,或 `all`),以及设备陈旧判定
(`device_stale_days`,默认 3 天)。
