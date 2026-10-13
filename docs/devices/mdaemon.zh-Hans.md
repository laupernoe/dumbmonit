# MDaemon Email Server {#mdaemon-email-server}

从外部检查 MDaemon 服务器的邮件服务:SMTP、IMAP、POP3、网页邮件
和远程管理是否接受连接,并给出正确的问候。配合一个账户,
DumbMonit 还会通过 XML API 读取确切的版本。配合安装在 MDaemon
服务器上的[代理](agent.md),同一页面还会显示邮件队列、会话和
消息总数,而这些只能作为 Windows 性能计数器被 MDaemon 公布。

!!! warning "仅依据文档验证"

    这项集成是根据 MDaemon Technologies 发布的文档和其中展示的
    XML API 应答格式构建的,而不是针对一台真实的 MDaemon 服务器。
    每次调用都是读取操作,邮件端口检查遵循协议标准,但尚无人在
    真实运行的 MDaemon 上运行过它。代理读取的队列计数器同样
    如此:它们的名称来自一份已发布的 MDaemon 性能计数器列表,
    而不是来自一台正在运行的服务器。如果有任何问题,请告诉
    我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_mdaemon_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `service_up` | `service`, `port` | 当该端口接受连接,且对于 SMTP、POP3 和 IMAP 以 `220`、`+OK` 或 `* OK` 问候时为 1。未应答,或应答表示拒绝服务(`421`、`554`、`-ERR`、`* BYE`)时为 0。 |
| `service_response_seconds` | `service`, `port` | 到收到问候的时间,或对于没有问候的端口,到建立连接的时间。 |
| `api_up` | | 仅在配置了账户时:XML API 应答时为 1,未应答时为 0。 |
| `api_response_seconds` | | XML API 应答所用的时间。 |
| `api_operation_ok` | | 当该账户被允许执行该调用时为 1,MDaemon 以错误状态应答时为 0。 |
| `info`(值为 1) | `version`, `build`, `source` | 版本:来自 XML API(`source="xml_api"`,`build` 为服务构建号),或在服务器于 SMTP 问候中公布版本时取自那里(`source="smtp_banner"`)。 |
| `scrape_duration_seconds` | | 整次检查所用的时间。 |

**部分故障仍算作一次成功的检查。** 某个服务停止会让该服务的
`service_up` 为 0,其他服务仍会被正常测量。只有当没有任何服务、
API 也完全没有应答时,该设备才算*不可达*。

**账户被拒绝属于配置错误,不是服务中断。** 密码错误、XML API
不允许的地址,或端口错误,会显示在设备页面上并附带原因,不会
发送通知。

**队列数据来自代理,不是来自这个设备本身。** MDaemon 以 Windows
性能计数器的形式(在名为 MDaemon 的对象下,以及其自己的 Queue
and Statistics Manager 中)公布其队列大小(入站、本地、远程、
重试、坏件、持有、隔离)、会话计数、消息统计和运行时间。MDaemon
发布的任何文档都没有描述一个能返回这些数据的 XML API 调用,
因此该设备不会去猜测一个:安装在 MDaemon 服务器上的
[代理](agent.md)会改为读取这些性能计数器。参见下方
[通过代理监控邮件队列](#mail-queues-through-the-agent)。许可证
和邮件存储的磁盘空间没有通过任何已记录的接口或性能计数器暴露,
因此不会被读取。代理还会监控存放邮件存储的磁盘;在其 `services`
设置中列出 MDaemon 服务即可监控该 Windows 服务本身。

适用的[内置规则](../alerting/rules.md#mdaemon-and-securitygateway):
MDaemon 邮件服务宕机、MDaemon XML API 未应答,以及设备不可达;
配合代理时还有 MDaemon 邮件队列增长、MDaemon 坏件队列非空,以及
MDaemon 重试队列过高。

## 设备页面 {#the-device-page}

一台 MDaemon 设备会在图表上方显示一个**邮件服务**面板,读取自
上次存储的测量数据:打开该页面从不会连接到服务器。它列出每个被
监控的服务及其端口、一个状态词和一种颜色(应答中、已宕机)以及
其响应时间;然后是版本及其读取来源。已停止的服务会排在最前。

当同一台服务器上的代理报告 MDaemon 的计数器时,随后会出现一个
**邮件队列**面板:每个队列中的消息数(冻结的队列、非空的坏件
队列以及超过五十条消息的重试队列会以文字标注)、活跃会话数、
过去 24 小时的消息数以及垃圾邮件、病毒和 DNSBL 的判定结果、
MDaemon 的运行时间,以及 MDaemon 报告为不活跃的内部服务器。
同一个面板也会出现在代理自己的页面上。

## 通过代理监控邮件队列 {#mail-queues-through-the-agent}

1. 在运行 MDaemon 的 Windows 服务器上安装[代理](agent.md#install)。无需其他任何操作:代理会注意到名为 `MDaemon` 的 Windows 服务,并在每个采样周期读取 MDaemon 性能对象的计数器。要强制开启或关闭它,请在 `agent.yaml` 中设置 `mdaemon: true` 或 `mdaemon: false`。
2. 这些队列会出现在代理自己的页面上。若要同时在这台 MDaemon 设备上显示它们,DumbMonit 会在同一台机器上查找代理:被设为该设备父设备的代理、探测它的中继代理,或主机名与该设备地址匹配的代理(`mail.example.com` 对应 `mail`),或者在只有一台 MDaemon 设备和一个报告 MDaemon 计数器的代理时,就是那个代理。如果没有匹配,请编辑该设备并选择该代理作为其**父设备**:代理和 MDaemon 共享同一台机器,因此当代理静默时,该设备的告警也会被同时抑制。

这些序列属于代理,带有它的 `target` 标签。全部以
`dumbmonit_mdaemon_` 为前缀:

| 指标 | 标签 | 读取的性能计数器 |
|---|---|---|
| `queue_messages` | `queue`:`inbound`、`local`、`remote`、`retry`、`bad`、`holding`、`lan`、`quarantine`、`raw` | `Inbound queue messages`、`Local queue messages`、`Remote queue messages`、`Retry queue messages`、`Bad queue messages`、`Holding queue messages`、`LAN queue messages`、`Quarantine queue messages`、`RAW queue messages` |
| `queue_frozen`(1 表示冻结) | `queue`:`inbound`、`local`、`remote` | `Inbound queue frozen`、`Local queue frozen`、`Remote queue frozen` |
| `sessions_active` | `protocol`:`smtp_in`、`smtp_out`、`pop3_in`、`pop3_out`、`imap`、`webmail` | `Active SMTP (in) sessions`、`Active SMTP (out) sessions`、`Active POP3 (in) sessions`、`Active POP3 (out) sessions`、`Active IMAP sessions`、`Active Webmail sessions` |
| `sessions_total`(计数器) | `protocol`:`smtp_in`、`smtp_out`、`pop3`、`imap` | `SMTP sessions (in) total`、`SMTP sessions (out) total`、`POP3 sessions total`、`IMAP sessions total` |
| `messages_total`(计数器) | `protocol`:`smtp_in`、`smtp_out`、`domainpop_in` | `SMTP messages (in) total`、`SMTP messages (out) total`、`DomainPOP messages (in) total` |
| `filtered_messages_total`(计数器) | `filter`:`spam`、`virus`、`dnsbl`;`verdict`:`accepted`、`refused` | `spam accepted total`、`spam refused total`、`Viruses accepted total`、`Viruses refused total`、`DNSBL accepted total`、`DNSBL refused total` |
| `server_active`(1 表示活跃) | `server`:`smtp`、`pop3`、`imap`、`webmail`、`webadmin`、`activesync`、`antispam`、`antivirus`、`minger`、`multipop`、`domainpop` | `SMTP server state`、`POP3 server state`、`IMAP server state`、`Web Mail server state`、`Web Admin server state`、`ActiveSync server state`、`AntiSpam server state`、`AntiVirus server state`、`Minger server state`、`MultiPOP server state`、`DomainPOP server state` |
| `running`(1 表示正在运行) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

总量是自 MDaemon 上次启动以来的累计值:在查询中使用
`increase()` 或 `rate()`,就像该页面处理其 24 小时数据那样。
按秒统计的计数器(`SMTP messages (in)/sec`……)不会被读取,
因为总量能给出同样的速率。如果你的 MDaemon 版本没有某个计数器,
或名称不同,会在代理日志中报告一次(`performance counter not
available`),其余计数器仍会被正常读取;若要读取预设列表中没有
的某个计数器,请在 `agent.yaml` 的 `perf_counters` 下列出它
(参见[Windows 性能计数器](agent.md#windows-performance-counters))。

## 监控邮件服务,并可选地监控 XML API {#watch-the-mail-services-and-optionally-the-xml-api}

1. 服务器上无需安装任何东西。不配置凭据时,DumbMonit 会从外部连接邮件端口并读取每个问候:默认是 SMTP(25)、IMAP(143)和网页邮件(3000)。可在"服务"选项中添加 POP3、MSA、TLS 端口或远程管理,格式如下。

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. 若要同时通过 XML API 读取版本,请在 MDaemon 中创建一个专用账户,例如按如下方式命名,使用一个不在任何其他地方使用的较长密码。为它赋予你的 MDaemon 对 XML API 所接受的最低级别,不多不少:DumbMonit 只会调用 GetVersionInfo,它不读取任何邮箱,也不更改任何内容。

    ```
    dumbmonit@example.com
    ```

3. 从 MDaemon 24 起,XML API 只应答它允许的地址。打开 Setup → XML API Service → Address Restrictions(在 MDaemon 26 上为 Setup → XML API Management),允许 DumbMonit 连接所用的地址。

4. 从 DumbMonit 所在主机确认该账户能访问该 API。该命令会要求输入密码,并打印一个以 `<MDaemon><API productversion=…>` 开头的应答。

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. 在 DumbMonit 中,输入服务器地址,例如 "mail.example.com",然后把该账户的完整邮箱地址作为用户名,以及其密码。XML API 通过远程管理的 HTTPS 端口访问,默认为 444。

!!! warning "警告"
    MDaemon 对 XML API 没有只读角色:绝不要重复使用你自己的
    账户,并把这个密码隔离在任何其他工具之外。动态屏蔽功能
    会在多次登录失败后封锁某个地址,因此这里的密码错误可能会
    导致 DumbMonit 所在主机被封锁:请先用上面的命令测试。远程
    管理通常使用自签名证书:如果因此连接被拒绝,请在选项中
    勾选"接受无法验证的证书"。邮件队列大小不会被这个设备读取:
    MDaemon 只会以 Windows 性能计数器的形式公布它们,只有在
    DumbMonit 代理安装在邮件服务器上时才会被读取。

该 XML API 由远程管理网页服务器提供服务:默认情况下 HTTP 为
1000 端口、HTTPS 为 444 端口,路径为 `/MdMgmtWS/`,或当远程管理
运行在 IIS 上时由 IIS 提供。其完整参考文档随服务器一起提供:
在浏览器中打开 `https://mail.example.com:444/MdMgmtWS`,或查看
`\MDaemon\Docs\API\XML API\`。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 仅服务 | 不需要:这些邮件端口会在不登录的情况下被检查。 |
| XML API 账户 | 该账户的完整邮箱地址和密码,以 HTTP 基本认证方式发送。 |

地址:服务器的主机名或 IP,例如 `mail.example.com`。写在地址中
的端口(`mail.example.com:8444`)会被当作 XML API 端口。

## 选项 {#options}

| 选项 | 默认值 | 作用 |
|---|---|---|
| `services` | `smtp,imap,webmail` | 要检查的内容,用逗号分隔:`smtp`(25)、`msa`(587)、`smtps`(465)、`pop3`(110)、`pop3s`(995)、`imap`(143)、`imaps`(993)、`webmail`(3000)、`remote_admin`(1000)、`remote_admin_https`(444)、`xmpp`(5222)。`name:port` 可以更改某个端口;其他任意名称配合端口会添加你自己的服务,只检查连接是否成功。`none` 不检查任何端口。 |
| `request_timeout_seconds` | `10` | 每次连接、问候和 API 调用允许的时间,1 到 60。 |
| `api_port` | `444` | 提供 `/MdMgmtWS/` 的远程管理端口。仅在配置了账户时使用。 |
| `api_tls` | `true` | 使用 HTTPS 访问 XML API。只有当远程管理通过纯 HTTP(默认端口 1000)提供服务时才取消勾选:此时密码会以明文方式经过网络传输。 |
| `insecure_tls` | `false` | 在 XML API 上接受自签名证书。 |

TLS 端口(`smtps`、`pop3s`、`imaps`、`remote_admin_https`)只会
通过连接检查。若要监控它们证书的过期情况,请为每个端口添加一个
[TLS 证书](services.md#tls)设备。
