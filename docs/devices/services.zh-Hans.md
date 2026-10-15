# 服务：HTTP、TCP、DNS、ping、TLS、NTP、SMTP、SQL、MQTT、WebSocket {#services-http-tcp-dns-ping-tls-ntp-smtp-sql-mqtt-websocket}

十一个监视器观看**服务**而不是设备，Uptime Kuma 风格：一个网页、一个端口、一个 DNS 名称、一个主机、一个证书、一个时间服务器、一个邮件中继、一个数据库、一个 MQTT 代理、一个 WebSocket 端点。每个服务获得一个历史条、一个响应时间和在其设备页面上的可用百分比。

最后五个进一步比打开连接：它们在服务自己的协议中开始一个真实会话。这是"端口 5432 打开"和"数据库仍然接受我的账户并答复查询"之间的区别。NTP 介于两者之间：一个单一请求和回复，在 NTP 自己的协议中，足以读取服务器的时钟而不需要会话。

## 服务状态如何工作 {#how-service-state-works}

服务可以答复并仍然关闭：一个 `500`、一个缺失关键词、一个过期证书。所以这些监视器在每个检查写一个点，`probe_success` 是 1 或 0，而不是像硬件设备那样沉默。那是使可用百分比成为可能的东西：`avg_over_time(dumbmonit_probe_success[30d])`。

| 情况 | 发生什么 |
|---|---|
| 无效选项或地址，ICMP 套接字不允许（见 Ping） | 配置错误：没有写入任何东西，设备页面显示错误，没有"关闭"警报。 |
| 连接拒绝、超时、TLS 拒绝、`500`、关键词缺失、总包丢失、密码拒绝、查询失败 | `probe_success = 0`：中断被记录、日期和计数。 |
| 一切正常 | `probe_success = 1`。 |

每个监视器有其自己的超时（`timeout_seconds`，默认 5 s，最多 60 s），比服务器的 `DUMBMONIT_PROBE_TIMEOUT_SECS` 更短：由调度器中断，它无法写其零。

应用于所有十一个的内置规则：服务关闭（3 分钟）、服务摇晃（三十分钟内六个以上状态变更）、缓慢服务（十分钟内超过 3 s）。对于每个读取证书的检查 — `http`、`tls`、`smtp`、`mqtt`、`websocket`：证书即将过期（14 天）和证书已过期。

拒绝的密码从不是服务的中断：它被记录为 `reason="auth"`，所以警报说要修复什么。

### 指标 {#metrics}

所有仪表，所有以 `dumbmonit_` 为前缀，所有标记 `probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket|ntp"` 除了 `target`、`host` 和 `tag_*`。

| 指标 | 监视器 | 意义 |
|---|---|---|
| `probe_success` | 全部 | 1 如果服务正确答复，0 否则。 |
| `probe_duration_seconds` | 全部 | 检查的总持续时间。 |
| `probe_failure_info` | 全部 | 存在（1）带 `reason` 标签：`dns`、`connect`、`timeout`、`tls`、`cert_expired`、`status`、`keyword`、`json`、`body`、`packet_loss`、`record`、`auth`、`protocol`、`query`、`payload`。 |
| `probe_http_status_code` | http、websocket | 获得的状态代码。 |
| `probe_http_first_byte_seconds` | http | 到响应标头的时间。 |
| `probe_http_content_bytes` | http | 正文大小。 |
| `probe_http_security_header{header}` | http | 每个 `header` 标签为0/1（仅 https）：HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors。 |
| `probe_connect_seconds` | 全部除外 dns 和 ping | 连接建立（对于数据库包括登录）。 |
| `probe_tls_handshake_seconds` | tls、http、smtp、mqtt、websocket | 仅 TLS 协商。 |
| `probe_tls_version_info` | tls、http、smtp、mqtt、websocket | 存在；`version` 标签。 |
| `probe_ssl_cert_expiry_days` | tls、http、smtp、mqtt、websocket | 到期前的天数，如果过期为负。 |
| `probe_ssl_cert_valid` | tls、http、smtp、mqtt、websocket | 1 如果链导向已知当局。 |
| `probe_ssl_cert_issuer_info` | tls、http、smtp、mqtt、websocket | 存在；`issuer` 标签。 |
| `probe_dns_lookup_seconds` | dns | 解析时间。 |
| `probe_dns_answer_records` | dns | 请求类型的记录。 |
| `probe_dns_answer_fingerprint` | dns | 带"答案变更时警报"：答案的 32 位指纹。其值意味着什么都没有；仅它的变更。 |
| `probe_icmp_rtt_seconds`、`_min_seconds`、`_max_seconds` | ping | 平均、最短和最长的往返。 |
| `probe_icmp_packet_loss_ratio` | ping | 0 到 1 之间的损失。 |
| `probe_icmp_packets_sent`、`probe_icmp_packets_received` | ping | 发送和接收的回应。 |
| `probe_smtp_greeting_seconds` | smtp | 到 `220` 横幅的时间。 |
| `probe_smtp_ehlo_seconds` | smtp | `EHLO` 命令的往返时间。 |
| `probe_smtp_capabilities` | smtp | 广告的扩展。 |
| `probe_smtp_authenticated` | smtp | 1 当 `AUTH` 成功时。 |
| `probe_sql_query_seconds` | postgres、mysql | 仅查询执行。 |
| `probe_sql_rows` | postgres、mysql | 返回的行。 |
| `probe_sql_value` | postgres、mysql | 第一行的第一列，当它是一个数字时。 |
| `probe_mqtt_connack_seconds` | mqtt | 到 `CONNACK` 的时间。 |
| `probe_mqtt_suback_seconds` | mqtt | 到 `SUBACK` 的时间。 |
| `probe_mqtt_message_bytes` | mqtt | 保留消息的大小。 |
| `probe_mqtt_message_value` | mqtt | 其内容，当它是一个数字时。 |
| `probe_ws_handshake_seconds` | websocket | 打开握手。 |
| `probe_ntp_offset_seconds` | ntp | 本地时钟和服务器之间的偏移。 |
| `probe_ntp_delay_seconds` | ntp | 网络往返延迟。 |
| `probe_ntp_stratum` | ntp | 到参考时钟的距离（16：未同步）。 |
| `probe_ntp_leap_indicator` | ntp | 0 到 3（闰秒警告，3 = 时钟未同步）。 |
| `probe_ntp_root_delay_seconds` | ntp | 到层级根的累积延迟。 |
| `probe_ntp_root_dispersion_seconds` | ntp | 到根的累积分散。 |
| `probe_ntp_reference_info` | ntp | 存在；`reference_id` 标签。 |
| `probe_ws_message_bytes` | websocket | 接收的第一帧的大小。仅当检查发送消息或期望回复（`send` / `expect` 选项）时存在。 |

## HTTP {#http}

**网站或网络 API（HTTP）** — 检查一个页面或 API 答复、正确的状态代码、正确的内容和有效的证书。例子：一个网站、一个应用程序健康页面、一个 REST API、一个自托管服务接口。

### 设置 {#setup}

1. 在地址中，粘贴要监视的页面的完整 URL。没有 "http://" 或 "https://"，HTTPS 被假设。
2. 更喜欢一个轻型页面，不需要登录，例如应用程序的健康页面（"/health"、"/status"），而不是主页。
3. 如果页面需要身份验证，填写凭据：用户名和密码给出基本身份验证，令牌作为 "Bearer" 发送。
4. 默认情况下，200 到 299 之间的任何状态代码都很好。为了进一步，在页面中需要关键词或 JSON 响应中的特定值。
5. 在 HTTPS 上，证书自动读取：您将在过期前被警告。

!!! warning
    绝不在选项中写密码或令牌：它们在每次测量中以明文复制。使用凭据字段，它被加密。

凭据：无、用户名 / 密码（基本身份验证）或 API 令牌（`Authorization: Bearer`）。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `method` | HTTP 方法 | `GET` | GET 适合几乎每个情况。HEAD 避免仅当状态代码重要时下载页面。选择：GET、HEAD、POST、PUT、PATCH、DELETE、OPTIONS。 |
| `accepted_status` | 接受的状态代码 | `200-299` | 被认为正常的代码或范围，由逗号分隔。任何其他代码计为关闭。例子：`200-299,301,404`。 |
| `keyword` | 期望的关键词 | *（空）* | 必须在页面中出现的文本。如果缺失，即使页面答复，服务也被报告关闭。 |
| `keyword_absent` | 关键词必须缺失 | `false` | 反转检查：关键词的存在表示失败。对于答复 200 的错误页面有用。 |
| `keyword_case_sensitive` | 匹配关键词大小写 | `false` | 默认情况下，大写和小写被视为相同。 |
| `json_path` | 要检查的 JSON 路径 | *（空）* | 对于 JSON 响应：要检查的值的路径。与期望值一起填写，从不一个不带另一个。例子：`$.status`。 |
| `json_expect` | 期望的 JSON 值 | *（空）* | 上面的 JSON 路径必须有的值，例如 "ok" 或 "true"。 |
| `headers` | 额外标头 | *（空）* | 一个或多个 "Name: value" 标头，由 "\|" 分隔。不要把秘密放在这里：选项在图表中可见。 |
| `body` | 请求正文 | *（空）* | 与请求一起发送的内容，对于期望一个的方法（POST、PUT…）。 |
| `follow_redirects` | 跟踪重定向 | `true` | 禁用以监视重定向本身，例如到 HTTPS 的 301。 |
| `max_redirects` | 跟踪的最大重定向 | `10` | 在 1 到 20 之间。如果不跟踪重定向无效。 |
| `insecure_tls` | 接受不可验证的证书 | `false` | 检查不再在自签名证书或由私有当局颁发的证书上失败。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 默认检查拒绝仅 DumbMonit 主机本身可以到达的地址（127.0.0.1、::1、169.254.x.x）。启用此以监视运行在 DumbMonit 主机上的服务。私有 LAN 地址（10.x、192.168.x）总是允许的。 |
| `check_certificate` | 读取证书 | `true` | 在 HTTPS 上，也记录证书过期日期，以便您可以在过期前被警告。 |
| `max_body_bytes` | 最大字节读取 | `524288` | 超过此，页面的其余部分不被下载。关键词和 JSON 路径仅在此部分搜索。 |
| `user_agent` | 宣布的身份（User-Agent） | `DumbMonit/<version>` | 检查给服务器的名称。如果服务器过滤机器人，改变它。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

## TCP {#tcp}

**网络端口（TCP）** — 检查端口接受连接：SSH、数据库、文件共享、游戏服务器、网络打印机。

### 设置 {#setup}

1. 在地址中，写主机后跟端口，由冒号分隔："nas.home.lan:22" 或 "192.168.1.5:445"。
2. 对于 IPv6 地址，将其放在括号中："[fd00::1]:445"。
3. 您也可以不带端口留下地址，在 "Port" 选项中输入：两个之一是必需的。
4. 仅测试连接打开：没有数据被发送到服务，所以它对它没有影响。

!!! warning
    开放端口不证明它后面的应用程序工作。对于网络服务，更喜欢读取响应的 "Website or web API" 类型。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `port` | 端口 | *（空）* | 要打开的端口，如果地址已经没有给出为 "host:port"。两个之一是必需的。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 默认检查拒绝仅 DumbMonit 主机本身可以到达的地址（127.0.0.1、::1、169.254.x.x）。启用此以监视运行在 DumbMonit 主机上的服务。私有 LAN 地址（10.x、192.168.x）总是允许的。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

## DNS {#dns}

**域名（DNS）** — 检查一个名称解析，它指向正确的地址。例子：您的域名、由您的 Pi-hole 或 AdGuard 提供的内部名称、MX 记录。

### 设置 {#setup}

1. 在地址中，写要解析的名称，不带 "http://"："www.example.com"。
2. 默认情况下，检查询问系统解析器关于 IPv4 地址（A 记录）。如果需要，在选项中选择另一个记录类型。
3. 要监视您自己的 DNS 服务器，在 "DNS server to query" 中输入其 IP 地址：如果它停止答复，检查会失败。
4. 为了检测劫持或配置错误，在 "Expected values" 中列出答案必须包含的地址。

!!! warning
    DNS 服务器由其 IP 地址给出，不是名称：需要一个解析器来解析解析器。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `record_type` | 记录类型 | `A` | A 对于 IPv4 地址，AAAA 对于 IPv6，MX 对于邮件，CNAME 对于别名…。选择：A、AAAA、CNAME、MX、TXT、NS、SOA、SRV、PTR、CAA。 |
| `resolver` | DNS 服务器查询 | *（空）* | 解析器的 IP 地址，端口可选。空：系统解析器。方便检查您自己的 DNS 服务器。例子：`1.1.1.1` 或 `10.0.0.1:5353`。 |
| `expect` | 期望的值 | *（空）* | 必须全部出现在答案中的值，由逗号分隔。空：仅检查解析。 |
| `expect_mode` | 比较 | `contains` | `contains`：每个期望值必须在答案中的某处出现。`exact`：答案必须持有那些值而没别的。 |
| `forbid` | 禁止的值 | *（空）* | 从不必须出现在答案中的值，由逗号分隔。在期望值之前检查。 |
| `alert_on_change` | 答案变更时警报 | `false` | 记录排序答案的指纹，每当它变更时警报，而不必写期望值。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

### 断言一个区答复什么 {#asserting-what-a-zone-answers}

`expect` 单独捕捉消失的记录。它不捕捉被*添加*的记录：被劫持的区经常保持合法的 `A` 并在旁边放置第二个，一个客户端然后随机到达其中一个。

- `expect_mode = exact` 拒绝期望不覆盖的任何值。顺序不重要 — DNS 不保留一个。
- `forbid` 命名从不必须回来的值：前主机的地址、过期的验证 `TXT`、您已停用的邮件服务器。它首先被检查，所以它赢过 `expect`。

两个比较不区分大小写，忽略名称尾随点和 `TXT` 周围的引号，并将期望值视为在记录内找到 — 所以 `MX` 在 `mail.example.com` 上匹配而不必复制其优先级。

### 答案变更时警报 {#alerting-when-the-answer-changes}

写期望值并不总是实用的：由 CDN 提供的名称、别人管理的记录或简单一个您想在任何时候移动听到的区。勾选**答案变更时警报**：每个检查然后记录答案的指纹，内置规则[DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains)当它与之前不同时触发。警报自己在一小时后解决；新答案从那时是参考。

指纹覆盖答案的值，排序且无重复，CNAME 链包括：返回相同地址以另一个顺序的轮询不改变它，TTL 也不。其答案在每个查询上变更的名称（一个大池如 `pool.ntp.org`，或地理 DNS 从一个解析器到下一个答复不同的）会一直警报：对那些留下选项关闭，或查询一个固定解析器。

## Ping {#ping}

**可达主机（ping）** — 发送几个 ICMP 回应并测量响应时间和包丢失。例子：主路由器或网关、Wi-Fi 接入点、打印机、没有代理或 SNMP 的机器、远程主机。

### 设置 {#setup}

1. 在地址中，写主机名或 IP 地址："192.168.1.1" 或 "router.home.lan"。
2. 在每个投票上，检查发送四个回应并测量响应时间，以及丢失包的份额。
3. 默认情况下，仅总损失计为关闭；部分损失保持在图表中可见。降低 "Tolerated loss" 以更早被警告。
4. 如果 DumbMonit 在 Docker 中运行，保持 DumbMonit 服务的 `sysctls:` 行的运送 docker-compose.yml：它们允许容器的非特权用户打开 ICMP 回应套接字。不需要能力。

!!! warning
    没有那个 sysctl，检查无法打开 ICMP 套接字：它将这作为配置错误报告，而不是主机失败。一些设备也故意忽略 ping：在得出结论之前检查那个。

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Docker 外，`sysctl -w net.ipv4.ping_group_range="0 2147483647"` 在主机上做同一件事，原始套接字（`NET_RAW`、root）也工作。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `count` | 回应数 | `4` | 每个投票上发送的包，从 1 到 20。 |
| `packet_timeout_ms` | 每个包的等待（ms） | `1000` | 每个回复允许的时间，从 50 到 10,000 ms。 |
| `interval_ms` | 两个回应间间隙（ms） | `100` | 两个回应间的停顿，从 0 到 5,000 ms。 |
| `payload_bytes` | 有效负载大小（字节） | `56` | 每个回应携带的数据，从 0 到 1,400 字节。 |
| `ip_version` | IP 版本 | `auto` | "auto" 取第一个解析地址；如果主机有两个且其中一个不答复，强制 4 或 6。选择：auto、4、6。 |
| `loss_threshold_percent` | 容忍损失（%） | `100` | 在这个百分比的丢失包上方，主机被报告关闭。100：仅总损失计为，部分损失在图表中保持可见。 |
| `timeout_seconds` | 超时（秒） | `5` | 投票的总预算，从 1 到 60。如果它不覆盖 "packets × wait per packet" 它被拒绝。 |

## TLS {#tls}

**TLS 证书** — 检查证书有效和在过期前警告，在任何加密端口上。例子：邮件服务器（IMAPS、SMTPS）、反向代理、LDAPS 目录、MQTT 代理、管理接口。

### 设置 {#setup}

1. 在地址中，写主机，如果它不是 443，加密服务的端口："mail.example.com:993"、"ldap.home.lan:636"。
2. 检查打开加密连接，读取出现的证书并立即关闭：没有数据与应用程序交换。
3. 它记录到期前的天数、颁发者和 TLS 版本。捆绑警报规则在十四天警告，然后在过期。
4. 如果服务通过其 IP 地址被到达但证书携带名称，在 "Server name (SNI)" 中输入那个名称。
5. 对于由您自己的当局颁发的证书，勾选 "Accept an unverifiable certificate"：过期日期仍被监视。

!!! warning
    对于网站，"Website or web API" 类型已经读取证书：该类型用于不说 HTTP 的服务，或您不想击中的应用程序。

即使证书被拒绝握手也完成：那是允许读取已过期的证书过期日期的东西。验证的结论在 `probe_ssl_cert_valid`。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `server_name` | 服务器名称（SNI） | *（空）* | 宣布到服务器并在证书中检查的域名。当地址是反向代理后面的 IP 时设置。 |
| `insecure_tls` | 接受不可验证的证书 | `false` | 不可验证的链不再计为失败：过期日期仍被记录。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 默认检查拒绝仅 DumbMonit 主机本身可以到达的地址（127.0.0.1、::1、169.254.x.x）。启用此以监视运行在 DumbMonit 主机上的服务。私有 LAN 地址（10.x、192.168.x）总是允许的。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

## SMTP {#smtp}

**邮件中继（SMTP）** — 在邮件服务器上打开真实会话：问候、EHLO、STARTTLS 和登录如果您给出一个。例子：您的提供商的 SMTP 中继、本地 Postfix 或 msmtp、Proxmox 邮件网关、邮件服务器的提交端口。

### 设置 {#setup}

1. 在地址中，写邮件服务器，端口如果它不是通常的一个："smtp.example.com" 或 "smtp.example.com:2525"。
2. 选择您的中继使用的加密：对于提交端口 587 的 STARTTLS，对于端口 465 的 TLS，对于在您自己的网络上监听端口 25 的中继没有。
3. 为了检查中继仍接受您的账户，填写凭据：检查然后运行 AUTH 并报告拒绝的密码如此，不是作为中断。
4. 什么都不被发送：检查停止在问候、扩展和可选登录后，然后挂起。没有消息进入队列。
5. 为了在扩展消失时被警告，在 "Expected extension" 中命名它：停止广告 STARTTLS 的中继是一个会以明文发送您的邮件的中继。

!!! warning
    接受的会话不证明邮件离开。接受一切并永远排队它的中继在这里完美答复：监视队列本身。

凭据：无，或用户名和密码与 `AUTH PLAIN` 发送（或当服务器仅提供那个时 `AUTH LOGIN`）。它们在连接被加密和证书被判断之后发送：其链不能验证的中继从不接收它们。

在 TLS 或 STARTTLS 上，证书被读就像 TLS 检查读它 — 过期、颁发者、协议版本 — 所以邮件中继也是一个您被警告的证书。

### 它无法检测什么 {#what-it-cannot-detect}

那邮件实际离开。接受每个会话然后延迟一切的中继在这里完美答复；改为监视它的队列（见 Proxmox 邮件网关页面）。它也从不发送消息，所以它说关于收件人过滤、SPF 或 DKIM 的什么。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `security` | 加密 | `starttls` | STARTTLS 以明文开始并升级（端口 587），TLS 从第一个字节加密（端口 465），None 从不加密（端口 25）并仅适合您自己网络上的中继。 |
| `port` | 端口 | *（空）* | 当地址没有给出一个时使用。空：与 STARTTLS 587，与 TLS 465，不加密 25。 |
| `helo_name` | 宣布的名称（EHLO） | `dumbmonit` | 检查当它自我介绍时给出的名称。严格的中继拒绝它无法解析的名称。 |
| `expect_capability` | 期望的扩展 | *（空）* | 服务器必须在其 EHLO 答案中广告的扩展，例如 STARTTLS 或 AUTH。空：无期望。 |
| `server_name` | 服务器名称（SNI） | *（主机）* | 宣布到服务器并在证书中检查的域名。当地址是 IP 时设置。 |
| `insecure_tls` | 接受不可验证的证书 | `false` | 不可验证的链不再计为失败：过期日期仍被记录。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 见下面的 "Addresses the checks refuse"。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

### 失败原因 {#failure-reasons}

| `reason` | 发生什么 |
|---|---|
| `connect` | 端口关闭或中继无法到达。 |
| `protocol` | 什么答复，但不是 SMTP — 或它拒绝连接、`EHLO` 或它广告的 STARTTLS。 |
| `auth` | 账户被拒绝（`535`、`530`、`534`）。中继很好；凭据不。 |
| `payload` | "Expected extension" 中命名的扩展未被广告。 |
| `tls`、`cert_expired` | 与 TLS 检查相同的意义。 |

## PostgreSQL {#postgresql}

**PostgreSQL 数据库** — 连接、认证和运行一个查询：信号表示数据库启动但不再答复。例子：Nextcloud 或 Immich 后面的数据库、Home Assistant 记录器、应用数据库、TimescaleDB 实例。

### 设置 {#setup}

1. 在服务器上，为监视创建账户，给它不超过连接权限的东西：CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password'；
2. 确保那个账户可以从 DumbMonit 机器到达服务器，这通常意味着 pg_hba.conf 中的一行更多后跟配置重载。
3. 在地址中，写服务器，端口如果它不是 5432："db.home.lan" 或 "db.home.lan:5433"。
4. 默认查询是 SELECT 1，它读无表：什么别的有被授予。连接时间和查询时间被分开记录。
5. 为了监视您自己的某某，用一个返回单一数字的那个替换查询，它变成一个图表：一个队列长度、行计数、复制延迟。

!!! warning
    检查在每个投票打开一个真实连接。在一个实例已接近其连接限制，给它一个更长的间隔。

凭据：用户名和密码，必需的。数据库从不匿名地监视，所以形式提供什么别的。

### 它无法检测什么 {#what-it-cannot-detect}

复制延迟、一个拖延的锁、一个继续增长的表。那些全部活在引擎自己的管理视图中 — 写查询您自己，数字它返回变成 `probe_sql_value`，一个图表您可以警报。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `port` | 端口 | `5432` | 当地址没有给出一个时使用。 |
| `database` | 数据库 | `postgres` | 在连接时打开的数据库。监视账户必须被允许连接到它。 |
| `query` | 查询 | `SELECT 1` | 在每个检查上运行。默认读无表，所以账户需不超过连接权限。一个返回一个数字的查询变成图表。 |
| `expect` | 期望的值 | *（空）* | 第一行的第一列必须持有的值。空：仅检查查询成功。 |
| `sslmode` | 加密 | `prefer` | `prefer` 当服务器提供时加密，`require` 拒绝无它连接，`verify-full` 也检查证书和名称，`disable` 从不加密。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 见下面的 "Addresses the checks refuse"。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

## MySQL 和 MariaDB {#mysql-and-mariadb}

**MySQL 或 MariaDB 数据库** — 连接、认证和运行一个查询：信号表示数据库启动但不再答复。例子：WordPress、Nextcloud 或 Kimai 后面的数据库、应用数据库、Docker 中的 MariaDB。

### 设置 {#setup}

1. 在服务器上，创建具有零权限的监视账户：CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password'；
2. 检查账户可以从 DumbMonit 机器连接：localhost 的主机模式仅接受在服务器本身上做的连接。
3. 在地址中，写服务器，端口如果它不是 3306："db.home.lan" 或 "db.home.lan:3307"。
4. 默认查询是 SELECT 1，它读无表：什么别的有被授予。连接时间和查询时间被分开记录。
5. 为了监视您自己的某某，用一个返回单一数字的那个替换查询，它变成一个图表：一个队列长度、行计数、复制延迟。

!!! warning
    检查在每个投票打开一个真实连接。在一个实例已接近其连接限制，给它一个更长的间隔。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `port` | 端口 | `3306` | 当地址没有给出一个时使用。 |
| `database` | 数据库 | *（空）* | 在连接时打开的数据库。空：无，对于默认查询足够。 |
| `query` | 查询 | `SELECT 1` | 在每个检查上运行。默认读无表，所以账户需不超过连接权限。一个返回一个数字的查询变成图表。 |
| `expect` | 期望的值 | *（空）* | 第一行的第一列必须持有的值。空：仅检查查询成功。 |
| `sslmode` | 加密 | `prefer` | `prefer` 当服务器提供时加密，`require` 拒绝无它连接，`verify-full` 也检查证书和名称，`disable` 从不加密。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 见下面的 "Addresses the checks refuse"。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

### 失败原因，两个引擎 {#failure-reasons-both-engines}

| `reason` | 发生什么 |
|---|---|
| `connect` | 端口关闭、服务器无法到达或它拒绝连接为了不同于账户的理由（启动、连接过多）。 |
| `auth` | 账户被拒绝：错误密码、无 `pg_hba.conf` 行、不覆盖 DumbMonit 机器的主机模式。 |
| `query` | 连接和认证，但查询失败：缺失表、数据库为只读、语法错误。 |
| `payload` | 查询工作但返回东西不是期望值。 |
| `timeout` | 连接或查询未在时间内完成 — "启动但缓慢"的情况。 |

## MQTT {#mqtt}

**MQTT 代理** — 连接到代理，订阅主题，可以等待保留消息。例子：Mosquitto、Home Assistant 后面的代理、Zigbee2MQTT、ESPHome 传感器。

### 设置 {#setup}

1. 在地址中，写代理，端口如果它不是通常的一个："broker.home.lan" 或 "broker.home.lan:1884"。
2. 对于监听 8883 的代理勾选 "Encrypted connection"，如果它需要账户填写凭据。
3. 连接已经值得监视：拒绝您的账户或停止答复的代理是整个房子变静的原因。
4. 为了进一步，命名主题：检查然后订阅它，拒绝订阅的代理告诉您账户失去其访问权限。
5. 仅对携带保留消息的主题勾选 "Expect a retained message"，通常是可用主题。现在与然后发布的主题看静的对刚连接的客户端。

!!! warning
    检查从不发布任何东西。它也无法告诉保留消息如何旧：MQTT 不给它们日期，所以值冻结三周前仍计为存在。

### 它无法检测什么 {#what-it-cannot-detect}

一个给定传感器停止发出。检查读的保留消息是无论代理保留的，可能三周前 — MQTT 不给它们时间戳。对于新鲜，监视消费它的自动化，或使用心跳。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `tls` | 加密连接 | `false` | 从第一个字节加密会话，如代理在端口 8883 上做的。 |
| `port` | 端口 | *（空）* | 当地址没有给出一个时使用。空：明文 1883，加密 8883。 |
| `topic` | 主题 | *（空）* | 检查订阅的主题。空：它仅连接，它已经告诉您代理活着并接受您的账户。 |
| `expect_message` | 期望保留消息 | `false` | 消息必须在超时前到达那个主题。仅保留消息立即到达：一个仅从时间与然后发布的主题对刚连接的客户端看静。 |
| `expect` | 期望的内容 | *（空）* | 那个消息必须包含的文本。填写它意味着期望消息。 |
| `client_id` | 客户标识符 | *（派生）* | 宣布到代理的名称。默认派生自设备，所以两个检查从不互相断开连接。 |
| `server_name` | 服务器名称（SNI） | *（主机）* | 宣布到代理并在证书中检查的域名。当地址是 IP 时设置。 |
| `insecure_tls` | 接受不可验证的证书 | `false` | 不可验证的链不再计为失败：过期日期仍被记录。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 见下面的 "Addresses the checks refuse"。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

### 失败原因 {#failure-reasons}

| `reason` | 发生什么 |
|---|---|
| `connect` | 端口关闭或代理无法到达。 |
| `auth` | `CONNACK` 拒绝账户，或代理拒绝订阅 — 访问控制列表不再覆盖这个主题。 |
| `protocol` | 什么答复但不说 MQTT 3.1.1，或代理拒绝客户标识符。 |
| `payload` | 没有保留消息到达主题，或其内容不包含期望文本。 |

## WebSocket {#websocket}

**WebSocket 端点** — 运行升级握手，可以发送框和等待一个：不同失败从纯 HTTP 页面。例子：Home Assistant API、活仪表板、日志流、反向代理后面的端点。

### 设置 {#setup}

1. 在地址中，粘贴完整端点："wss://home.example.com/api/websocket"。不带方案，wss 被假设。
2. 检查运行整个打开握手并验证服务器从它被发送的密钥计算的答案：已失去 Upgrade 标头的反向代理在这里被捕获，那里 HTTP 检查仍看到健康 200。
3. 对于大多数端点什么别的是需要的。除 101 以外的状态被报告与其代码，所以一个来自过期令牌的 401 不被误认为中断。
4. 为了进一步，用服务器自己发送的第一帧的片段填写 "Expected content"，例如 Home Assistant API 的问候。
5. 如果端点在被说话到之前说什么，在 "Frame to send" 中放消息，在 "Expected content" 中放其答案必须包含什么。

!!! warning
    检查在一秒内打开并挂起。防火墙或代理超时在三十秒后切割的连接看完美健康这里。

凭据：无、用户名和密码（在打开请求中作为 HTTP 基本认证发送）或令牌（作为 `Authorization: Bearer` 发送）。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `path` | 路径 | *（来自地址）* | 打开请求的路径。当它携带一个时从地址取。 |
| `port` | 端口 | *（空）* | 当地址没有给出一个时使用。空：wss 443，ws 80。 |
| `send` | 要发送的框 | *（空）* | 连接打开时发送的文本框。空：什么都不发送。 |
| `expect` | 期望的内容 | *（空）* | 接收的框必须包含的文本。空且什么都不发送，仅打开握手被检查。 |
| `subprotocol` | Subprotocol | *（空）* | Sec-WebSocket-Protocol 的值。如果服务器选择一个不同的检查失败。 |
| `origin` | 来源 | *（空）* | Origin 标头的值，一些服务器在升级前要求。 |
| `server_name` | 服务器名称（SNI） | *（主机）* | 宣布到服务器并在证书中检查的域名。当地址是 IP 时设置。 |
| `insecure_tls` | 接受不可验证的证书 | `false` | 不可验证的链不再计为失败：过期日期仍被记录。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 见下面的 "Addresses the checks refuse"。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

### 失败原因 {#failure-reasons}

| `reason` | 发生什么 |
|---|---|
| `connect` | 端口关闭或主机无法到达。 |
| `status` | 端点答复东西不是 `101`；`probe_http_status_code` 携带代码。 |
| `protocol` | 它答复 `101` 但其 `Sec-WebSocket-Accept` 不匹配发送的密钥，它选择另一个 subprotocol，或它立即关闭连接。 |
| `payload` | 没有框到达，或它不包含期望文本。 |

## NTP {#ntp}

**NTP 时间服务器** — 发送单一 SNTP 请求并读回复：偏移、往返延迟、层级和闰指示器。例子：一个作为本地时间服务器的路由器或 NAS、内部 NTP 服务器（chrony、ntpd）、用作参考的公共池服务器。

### 设置 {#setup}

1. 在地址中，写服务器的名称或 IP 地址："pool.ntp.org" 或 "192.168.1.1"。UDP 端口 123 被使用除非 "Port" 选项说别的。
2. 检查发送单一 SNTP 请求并读回复：没有状态被保留在投票之间，什么都不被写到服务器。
3. 它记录服务器的时钟和 DumbMonit 主机之间的偏移、往返延迟、层级和闰指示器。
4. 答复层级 16 或其闰指示器读 "unsynchronized" 的服务器从未管理同步：检查报告它关闭无论偏移什么。
5. 降低 "Tolerated offset" 以比 100 ms 更早被警告；为几跳从参考时钟远的服务器升高它。

!!! warning
    偏移仅与 DumbMonit 主机自己的时钟一样好：如果那个主机不本身保持同步（NTP、虚拟机管理程序的时间同步），测量的偏移反映它的漂移尽可能多服务器的。

凭据：无。

### 选项 {#options}

| 键 | 标签 | 默认值 | 帮助 |
|---|---|---|---|
| `port` | 端口 | `123` | NTP 服务器的 UDP 端口。 |
| `offset_threshold_ms` | 容忍偏移（ms） | `100` | 在这个时钟偏移上方，服务器被报告关闭。不同步服务器（层级 16）总是失败，无论这个值。 |
| `allow_private_targets` | 允许环回和链接本地目标 | `false` | 见下面的 "Addresses the checks refuse"。 |
| `timeout_seconds` | 超时（秒） | `5` | 在服务未答复后的时间报告关闭。在 1 到 60 之间。 |

### 失败原因 {#failure-reasons}

| `reason` | 发生什么 |
|---|---|
| `connect` | UDP 套接字无法被打开，或回复来自一个不期望的地址。 |
| `timeout` | 超时内没有回复到达。 |
| `protocol` | 回复不是有效 NTP 包，或不来自服务器（错误方式）。 |
| `unsynchronized` | 服务器答复层级 16 或 0（一个 "kiss-o'-death"，也在日志中命名原因），或其闰指示器读 "unsynchronized"。 |
| `clock_offset` | 测量的偏移超过 "Tolerated offset"。 |

## 检查拒绝的地址 {#addresses-the-checks-refuse}

连接到您给它们的地址的检查 — HTTP、TCP、TLS、NTP、SMTP、PostgreSQL、MySQL、MQTT、WebSocket — 报告它们看到的 — 这也是，逐字地，什么服务端请求伪造做的。为了保持监视管理员从读仅 DumbMonit 主机可以到达的服务（`127.0.0.1:8428` 上的嵌入指标数据库、`169.254.169.254` 上的云提供商元数据服务），检查默认拒绝：

- 环回地址（`127.0.0.0/8`、`::1`）；
- 链接本地地址（`169.254.0.0/16`、`fe80::/10`），包括云元数据地址，和未指定地址（`0.0.0.0`、`::`）。

私有 LAN 范围（`10.0.0.0/8`、`172.16.0.0/12`、`192.168.0.0/16`、`fc00::/7`）总是允许的：监视它们是家庭实验室的要点。

规则适用于您输入的地址、主机名解析为的每个地址（在连接时再次检查，所以改变其答案的名称不能溜过），以及 HTTP 检查跟踪的每个重定向。被拒绝的地址显示为目标上的配置错误 — 没有关闭警报被发送 — 命名选项启用：`allow_private_targets`。对于确实监视 DumbMonit 主机本身上的服务的目标打开它。

到被拒绝地址的重定向被报告为失败测量（`reason="connect"`），因为它是被监视服务改变，不是配置。

在 `json_path` 找到的值绝不在失败消息中的 32 个字符以上回应。

## 常见错误 {#common-errors}

| 症状 | 可能原因 |
|---|---|
| Ping 显示配置错误 | `net.ipv4.ping_group_range` sysctl 从 Compose 文件缺失。见上面。 |
| HTTP 关闭，`reason="keyword"` | 关键词仅在前 `max_body_bytes` 搜索，不区分大小写除非 `keyword_case_sensitive` 设置。 |
| HTTP 关闭，`reason="tls"` | 自签名或私有当局：设置 `insecure_tls`。 |
| 检查显示命名 `allow_private_targets` 的配置错误 | 地址是环回或链接本地。见上面的 "Addresses the checks refuse"。 |
| TCP 关闭，虽然 `telnet host port` 从您的笔记本工作 | DumbMonit 容器无法到达主机（Docker 网络、防火墙）。从容器的网络内测试。 |
| DNS 关闭，`reason="record"` | 答案不包含 `expect` 中列出的每个值。 |
| Ping 关闭，`reason="packet_loss"` | 损失在 `loss_threshold_percent` 上方；一些设备速率限制 ICMP。 |
| DNS 关闭，`reason="record"`，虽然 `expect` 匹配 | `expect_mode` 是 `exact` 并且答案保留额外值，或 `forbid` 匹配。 |
| SMTP 关闭，`reason="protocol"` 在 `security = starttls` | 中继不在这个端口广告 STARTTLS。对 465 使用 `tls`，对纯本地中继使用 `none`。 |
| 数据库检查关闭，`reason="auth"` | 账户存在但可能不从 DumbMonit 机器连接：检查 `pg_hba.conf` 或 MySQL 用户的主机部分。 |
| 数据库检查关闭，`reason="query"` | 连接和登录工作。数据库启动并拒绝工作：只读、缺失表、恢复。 |
| MQTT 关闭，`reason="payload"` 在明确有流量的主题上 | 仅*保留*消息到达刚订阅的客户端。发布那个主题与保留标志，或取消勾选 "Expect a retained message"。 |
| WebSocket 关闭，`reason="protocol"` 在 `101` 后 | 路径上什么东西答复升级而不说 WebSocket — 通常吞下 `Upgrade` 标头的反向代理规则。 |
