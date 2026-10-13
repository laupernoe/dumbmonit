# OPNsense {#opnsense}

开源防火墙和路由器:每个网关的状态及其往返时间和丢包率、pf
状态表、接口计数器和地址、VPN 隧道、DHCP 租约、解析器、服务、
CARP 以及待处理的更新。

!!! warning "仅依据测试实例验证"

    这项集成是依据 OPNsense 26.1 构建并验证的,为此专门在虚拟机
    中启动了官方镜像,并使用下面描述的受限 API 密钥——而不是
    针对一台已经为某个家庭路由多年流量的防火墙。每次调用都是
    读取操作,每种结构都来自那台真实的 OPNsense,但它的网关没有
    监控地址,它不属于某个 CARP 对,也没有运行 OpenVPN 或 IPsec
    隧道:这些部分依据的是 OPNsense 的源码,而不是观测到的实际
    应答。如果有任何问题,请告诉我们。

## 设备页面 {#the-device-page}

除了通用图表之外,一台 OPNsense 设备还会显示三个面板,读取自
探测存储的数据——打开该页面从不会查询防火墙本身:

* **WAN 与网关**——每个带有网关的接口当前的公网地址,然后是每个
  网关及其文字状态、往返延迟和丢包率。宕机的网关排在最前。这正
  是这项集成存在的意义所在:在一台多 WAN 防火墙上,备用链路会
  悄悄接管,网络上的其他一切都不会改变。
* **流量与状态表**——pf 正在跟踪的连接数相对于其配置上限,然后是
  每个接口的链路状态、地址、字节和包计数器,以及错误和丢包情况
  (如果有)。然后是 DHCP 租约,按服务器统计。
* **防火墙健康状况**——已停止的服务排在最前,然后是 VPN 隧道及
  其已连接的对端数量,然后是当该防火墙是某个 CARP 对的一半时的
  CARP 情况,然后是固件(当前运行的版本、镜像站提供的版本、待
  安装的软件包,以及是否需要重启),然后是解析器,然后是机器
  本身:运行时间、负载、内存、交换分区、网络缓冲区、分区和温度
  传感器。

只统计数量,绝不涉及流量内容。DumbMonit 不读取任何防火墙规则、
任何连接状态、任何客户端地址和任何主机名。DHCP 租约只会被计数,
从不会被列出;VPN 会话只会被计数,从不会被具名。

## "none" 意味着"在线" {#none-means-online}

当**一切正常**时,`GET /api/routes/gateway/status` 会在网关的
`status` 字段中写入 `"none"`——这个词单独看起来像是"没有信息"。
DumbMonit 会把它翻译为 `online`,这样页面、序列和规则就能与
OPNsense 自己的界面保持一致的说法。`down` 和 `force_down` 才是
故障;`loss` 和 `delay` 是 dpinger 的警告,此时网关仍会应答。

同一次调用返回的延迟、丢包率和标准差是**带单位的字符串**
(`"8.4 ms"`、`"0.0 %"`),当网关没有监控地址时则是 `"~"`。
`"~"` 会使对应序列少一条,而不是显示为零:一个没有人 ping 的
网关不是延迟为零毫秒,而是完全没有延迟数据。

## 更新检查从不会被触发 {#the-update-check-is-never-triggered}

`GET /api/core/firmware/status` 返回的是防火墙*上一次*检查的
结果——与 OPNsense 自己仪表盘显示的内容相同。DumbMonit 从不发送
该调用的 `POST` 形式,也不调用 `/api/core/firmware/check`,
后者会导致防火墙在每次测量时都去访问镜像站,而且是每一个
DumbMonit 安装实例都会这样做。

一台从未检查过——或刚刚安装完更新、清空了缓存——的防火墙,会
应答 `"status": "none"`,与一台已是最新版本的防火墙完全一样。
DumbMonit 会区分这两种情况,在一次检查运行之前,它只会发布
`firmware_checked = 0`,不会发布其他任何内容:不会显示"零个
待处理更新",也不会显示"不需要重启"。请从 System → Firmware
手动运行一次检查,或让防火墙自己的计划任务来做这件事。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_opnsense_` 为前缀。

| 指标族 | 指标 | 标签 |
|---|---|---|
| 身份 | `version_info`(标签带有 OPNsense 版本、FreeBSD 版本和产品线) | |
| 机器 | `uptime_seconds`, `load1/5/15`, `cpu_count`, `memory_used/total_bytes`, `memory_used_percent`(内存包含 ZFS 缓存,FreeBSD 把它算作已使用)、`swap_used/total_bytes`, `swap_used_percent`(没有交换分区的机器上不存在)、`mbuf_used` 和 `mbuf_total`(相对于 `kern.ipc.nmbclusters` 使用中的网络缓冲区簇)、`mbuf_used_percent`, `mbuf_failures`(自启动以来被拒绝的分配次数,计数器) | |
| 存储 | `disk_used_bytes`, `disk_total_bytes`, `disk_used_percent` | `device`, `mountpoint` |
| 传感器 | `temperature_celsius` | `sensor` |
| 网关 | `gateway_up`(只有 `down` 和 `force_down` 时为 0)、`gateway_monitored`, `gateway_delay_seconds`, `gateway_stddev_seconds`, `gateway_loss_percent`, `gateways_total`, `gateways_down` | `gateway`, `address` |
| 接口 | `interface_up`, `interface_bytes_in/out`, `interface_packets_in/out`, `interface_errors_in/out`, `interface_drops`, `interface_collisions`(计数器,原始值存储)、`interface_address_info`(当前地址,以标签形式) | `interface`, `device`,最后一项还带 `address` |
| 防火墙 | `pf_states`, `pf_state_limit`, `pf_states_used_percent` | |
| DHCP | `dhcp_leases`, `dhcp_leases_active` | `backend`(`kea`、`isc`、`dnsmasq`) |
| VPN | `vpn_tunnel_up`, `vpn_peers_total`, `vpn_peers_connected`(WireGuard:三分钟内有握手)、`vpn_handshake_age_seconds`(WireGuard,最近一个对端;从未有任何对端连接过时不存在)、`vpn_bytes_in/out`, `vpn_tunnels_down` | `kind`(`wireguard`、`openvpn`、`ipsec`)、`tunnel`(WireGuard 的设备名 `wg0`,或 IPsec 和 OpenVPN 的描述) |
| 解析器 | `unbound_running`, `unbound_queries`(计数器)、`unbound_cache_hit_percent` | |
| 服务 | `service_running`(1 或 0,防火墙列出的每个服务对应一条序列) | `service` |
| CARP | `carp_enabled`, `carp_maintenance_mode`, `carp_vip_master`(不属于某个对的防火墙上全部不存在) | `interface`, `vhid`, `address` |
| 固件 | `firmware_checked`,然后——只有在检查运行过之后——`firmware_updates_pending`, `firmware_upgrade_available`, `firmware_reboot_required`, `firmware_connection_ok` | |
| 采集 | `up`, `scrape_errors`, `scrape_duration_seconds` | |

接口计数器以原始计数器形式存储:用 `rate()` 包裹即可得到
吞吐量。这样做意味着防火墙重启会表现为一个空档,而不是一次
每秒数 GB 的虚假尖峰。

对于完全不受监控的网关,`gateway_up` 为 1,因为没有任何依据
说明其他情况;`gateway_monitored` 才是区分这两种情况的指标。
一个没有监控地址的网关不会产生延迟或丢包序列。

一台不属于高可用对的防火墙没有 CARP 虚拟地址,也完全没有 CARP
序列。这是一台没有集群的防火墙,不是一个降级的集群:不会触发
任何规则。

WireGuard、OpenVPN、IPsec、Unbound 和 Kea 都是插件。没有安装
某个插件的防火墙会应答 `404`,这既不算采集错误也不算认证错误:
只是少一个指标,界面上也不会显示任何内容。

适用的内置规则:设备不可达、互联网网关宕机、网关丢包、网关
缓慢、防火墙状态表接近满、防火墙网络缓冲区耗尽、VPN 隧道中断、
防火墙解析器已停止、防火墙核心服务已停止、防火墙过热、防火墙
遗留在 CARP 维护模式中、防火墙重启待处理、防火墙更新待处理。
通知会指明涉及的网关、隧道、服务或传感器。

## 需要在 OPNsense 中准备的内容 {#what-to-prepare-in-opnsense}

以下步骤正是表单旁边提示所展示的内容。原则:一个只拥有
DumbMonit 所需状态权限的组,一个从不登录的用户,以及一个不属于
你自己任何账户的 API 密钥。这个确切的组已针对 DumbMonit 发出的
每一次调用、在 OPNsense 26.1 上进行了验证。

1. 在网页界面中:System → Access → Groups → Add。按如下方式命名该组。

    ```
    dumbmonit
    ```

2. 在该组中勾选以下权限,按 OPNsense 命名的方式完全一致:Lobby: Dashboard(系统、内存、磁盘、温度、状态表)、System: Gateways、Status: Interfaces、Status: Services、System: Firmware、Interfaces: Virtual IPs: Status(CARP)和 Services: Unbound(MVC)。然后,按你实际运行的每项功能添加:Services: DHCP: Kea(v4)或 Services: Dnsmasq DNS/DHCP: Settings 用于租约、VPN: WireGuard: Status、Status: OpenVPN、Status: IPsec。省略的权限只会使对应的那部分指标缺失,不影响其他内容。

3. System → Access → Users → Add。按如下方式命名该账户,让 OPNsense 生成一个混乱的密码(这个账户从不登录:它靠密钥来应答),并把它加入 dumbmonit 组。

    ```
    dumbmonit
    ```

4. 再次编辑该用户,在 API keys 下点击 +。OPNsense 会下载一个带有两行内容(key= 和 secret=)的小文本文件,密钥只会显示这一次。把密钥复制到 DumbMonit 的 API key 字段,把密钥口令复制到其 API secret 字段。

5. 在 DumbMonit 中,输入防火墙地址,例如 "192.168.1.1" 或 "fw.lan:8443"。该防火墙永远不会被要求检查更新:DumbMonit 读取的是 OPNsense 按自己的计划运行的检查结果,或是你从 System → Firmware 手动启动的检查结果。

!!! warning "警告"

    不要重复使用你自己登录用的账户:API 密钥是一个永不过期的
    密码。OPNsense 的某些权限没有只读变体:Status: Services
    同样允许启动和停止服务,System: Firmware 同样允许启动更新,
    Unbound、Kea 和 Dnsmasq 相关权限也同样覆盖其设置。
    DumbMonit 永远只会读取,但为了限制一个泄露的密钥所能做的
    事,请把该组的来源网络限制为 DumbMonit 所在主机的地址。
    绝不要授予 All pages。OPNsense 也自带自签名证书:如果因此
    连接被拒绝,请在选项中勾选"接受无法验证的证书"。

地址:`192.168.1.1`、`fw.lan`、`fw.lan:8443`、`[fd00::1]`,或
完整 URL。如果缺失,会自动补上 `https` 协议和 443 端口。

有些权限覆盖多个调用。`Lobby: Dashboard` 涵盖系统信息、时间、
内存、磁盘、交换分区、网络缓冲区和温度相关接口以及状态表;
`Status: Interfaces` 涵盖带计数器和地址的接口概览。省略的权限
只会使对应的那部分指标缺失:探测仍会成功,`scrape_errors` 仍会
保持为零,因为一个可选调用上的 `403` 会被理解为"这个账户不能
看到那个",而不是失败。只有 `Lobby: Dashboard` 是必需的:没有
它,探测甚至无法读取防火墙的名称和版本,会报告一个认证错误。

### camelCase 还是 snake_case {#camelcase-or-snake_case}

OPNsense 25.7 把其 API 路径从 `systemResources` 重命名为
`system_resources`。两种写法目前都能应答,但权限检查是逐字符
比较路径的:在 25.7 及更高版本上,一个受限密钥会对旧写法得到
`403`,而在 25.1 上则是对部分新写法得到 `403`。DumbMonit 会先
尝试当前的写法,在遇到 `403` 时回退到另一种,因此同一个组在
重命名前后都能正常工作。

## 选项 {#options}

| 键 | 标签 | 默认值 | 说明 |
|---|---|---|---|
| `scheme` | 协议 | `https` | HTTPS 适合开箱即用的防火墙。只有当网页界面以明文方式提供服务时才使用 HTTP。 |
| `port` | 网页界面端口 | `443` | 当地址中未给出端口时使用。 |
| `insecure_tls` | 接受无法验证的证书 | `false` | OPNsense 默认自带自签名证书。 |
| `request_timeout_seconds` | 每次请求的超时时间(秒) | `15` | 每次 API 调用允许的时间,1 到 120。 |
| `gateways` | 监控网关 | `true` | 读取 dpinger 对每个网关的报告:在线还是宕机、往返延迟和丢包率。 |
| `interfaces` | 监控接口 | `true` | 读取每个接口的链路状态、地址以及字节、包、错误和丢弃计数器。 |
| `firewall` | 监控状态表 | `true` | 读取 pf 正在跟踪的连接数以及配置的上限。 |
| `dhcp` | 统计 DHCP 租约 | `true` | 统计防火墙正在分配的租约,无论其运行哪种服务器。只统计数量。 |
| `vpn` | 监控 VPN 隧道 | `true` | 读取 WireGuard、OpenVPN 和 IPsec:存在哪些隧道,哪些处于建立状态。 |
| `unbound` | 监控解析器 | `true` | 读取 Unbound 是否在应答。 |
| `services` | 监控服务 | `true` | 读取防火墙管理的服务列表以及其中哪些正在运行。 |
| `carp` | 监控 CARP | `true` | 读取高可用对的虚拟地址,以及持久维护模式。 |
| `firmware` | 监控更新 | `true` | 读取防火墙上次更新检查的结果以及是否需要重启。 |
| `temperature` | 读取温度传感器 | `true` | 读取防火墙公布的 CPU 和主板传感器。 |

## 常见错误 {#common-errors}

| 现象 | 可能原因 |
|---|---|
| 认证错误 | 密钥和密钥口令被调换了,或该密钥已从该用户处被删除。OPNsense 只会显示一次密钥口令:如果丢失了,请创建另一个并删除旧的。 |
| 得到一个网页而不是 JSON | API 密钥被拒绝,或该账户无法访问那个接口。OPNsense 此时会以 `200` 提供其登录页面;DumbMonit 会把它报告为认证问题,而不是无法读取的应答。 |
| 证书错误 | 自签名证书:安装一个受信任的证书(ACME 是一个插件),或启用 `insecure_tls`。 |
| 完全没有网关相关指标 | `gateways` 选项被关闭了,或该组缺少 `System: Gateways`。一台没有定义任何网关的防火墙确实没有网关。 |
| 某个网关没有显示延迟或丢包数据 | 它没有监控地址,因此 dpinger 不会 ping 它。OPNsense 写入 `~`,DumbMonit 不发布序列,而不是发布一个零值。 |
| "网关宕机" 对一条正常工作的链路触发 | 该监控地址不可达,即使链路本身是正常的——这在拒绝 ICMP 的 ISP 上很常见。请在 System → Gateways 中设置另一个监控地址。 |
| 没有状态表相关指标 | 该组缺少 `Lobby: Dashboard`。 |
| 在浏览器工具中可用的密钥却报认证错误 | 该工具使用了另一种路径写法。对于受限密钥,OPNsense 25.7 及更高版本对 `systemInformation` 会应答 `403`,对 `system_information` 会应答 `200`。 |
| 除 `firmware_checked` 外没有任何更新相关指标 | 该防火墙从未检查过更新,或刚刚安装了一个。请从 System → Firmware 运行一次检查。 |
| 没有任何 VPN 相关指标 | 该插件未安装,或没有配置任何隧道。一个被禁用的 IPsec 连接会被有意地不报告为宕机。 |
| "防火墙解析器已停止" 出现在一台解析正常的防火墙上 | Unbound 已安装但被关闭了,因为该防火墙用别的方式解析。请关闭这条规则。 |
| 固件版本看起来过期 | 该防火墙最近没有检查过镜像站。DumbMonit 读取的是上次检查的结果,它从不会触发一次检查。 |
| `scrape_errors` 大于零 | 某个可选调用因缺失权限或缺失插件之外的原因失败——通常是超时。其他指标不受影响。 |
