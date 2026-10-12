# Sophos Firewall

通过 XML API 监控 Sophos Firewall(SFOS):每个绑定到某个区域的接口的
链路状态,以及哪些 IPsec 连接处于激活状态。

这就是全部内容,是有意为之的。该 XML API 是一个配置 API:除了
每个接口只读的连接状态外,它不会暴露任何实时状态。`<Get><IPSecConnection>`
返回的是管理员是否激活了某个连接,而不是隧道是否已建立;高可用、
许可证和订阅以及固件版本完全不在其中。DumbMonit 会按原样发布
这个激活状态,绝不会把它呈现为隧道状态。

DumbMonit 每分钟会向
`https://<firewall>:4444/webconsole/APIController` 发出两次请求
(表单字段 `reqxml`,绝不放在 URL 中,否则密码会出现在日志里):
`<Get><Interface>` 和 `<Get><IPSecConnection>`。每次请求都带有一个
在所有设备访问配置文件中都是只读权限的账户的用户名和密码:
防火墙在 API 层面同样会强制执行该配置文件的限制。

!!! warning "仅依据厂商文档验证——尚未在真实系统上测试"

    这项集成是根据 Sophos 的 XML API 文档(`Interface` 和
    `IPSecConnection` 实体、登录和状态消息)构建的,而不是针对一台
    正在运行的防火墙。接口状态的具体文字(`Connected, 1000 Mbps…`
    或 `Disconnected`)来自该文档和社区报告。如果有任何问题,
    请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_sophos_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `api_version_info` | `api_version` | 固定值 1:防火墙公布的 API 版本(`2000.2`),与 SFOS 发行版本对应。 |
| `interface_link_up` | `interface`, `zone` | 当状态为 Connected 时为 1,为 Disconnected 时为 0;状态为其他值时不存在。 |
| `interface_enabled` | `interface`, `zone` | 当接口被关闭时为 0。 |
| `interface_down` | `interface`, `zone` | 当接口已启用、已绑定到某个区域且没有链路时为 1。空闲端口(区域为 None)不受影响。 |
| `interfaces`, `interfaces_down` | | 计数。 |
| `ipsec_connection_activated` | `connection` | 当该连接在配置中被激活时为 1。不代表隧道状态。 |
| `ipsec_connections` | | 计数。 |

适用的[内置规则](../alerting/rules.md#sophos-firewall):

- **Sophos 接口无链路**:一个已启用并绑定到某个区域的接口,
  五分钟内没有链路(提示级别)。

如需流量计数器,请同时把该防火墙作为 [SNMP 设备](snmp.md)添加:
SFOS 会应答标准的接口 MIB。

## 设备页面 {#the-device-page}

Firewall 面板会说明某个区域内的所有接口是否都有链路,然后列出
这些接口,以及各 IPsec 连接的配置激活状态,并以此方式标注。

## 允许 DumbMonit 读取 Sophos Firewall XML API {#allow-dumbmonit-to-read-the-sophos-firewall-xml-api}

1. 在 Profiles > Device access 下,添加一个名为 DumbMonit read-only 的配置文件,把每一项权限都设为只读。

2. 在 Authentication > Users 下,添加一个名为 dumbmonit 的用户,类型为 Administrator,使用该配置文件,并设置一个较长的随机密码。

3. 在 Backup & firmware > API(SFOS 22 起为 Administration > API access)下,开启 API 配置,并把 DumbMonit 服务器的地址添加到允许的 IP 地址列表中。

4. 在 DumbMonit 中,输入防火墙的地址,例如 "sophos.lan",以及 dumbmonit 用户名和密码。API 监听在管理端口上,默认为 4444,使用自签名证书:除非你安装了自己的证书,否则请勾选"接受无法验证的证书"。

!!! warning "警告"
    该 XML API 是一个配置 API:它报告的是每个接口的链路状态以及每个 IPsec 连接是否被激活,而不是隧道是否已建立。高可用、许可证和固件版本都不在其中。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名和密码 | 拥有只读设备访问配置文件的 `dumbmonit` 用户。 |

地址:防火墙的主机名或 IP、`host:port`,或 URL。选项:

- **端口**(4444):管理端口。
- **接受无法验证的证书**:防火墙自身的证书是自签名的。
- **每次请求的超时时间**(15 秒)。

## 故障排查 {#troubleshooting}

"Sophos Firewall refused the request (534 …)":DumbMonit 的地址
不在 API 配置允许的 IP 地址列表中。

"Sophos Firewall refused the login":用户名或密码错误,或者
密码中含有防火墙在 API 调用中拒绝的字符;使用一个由字母和数字
组成的较长密码可以避免这个问题。

"Sophos Firewall did not answer in XML":端口错误,或 API 未开启。
