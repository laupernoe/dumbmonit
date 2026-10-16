# SecurityGateway for Email Servers {#securitygateway-for-email-servers}

从外部检查 SecurityGateway 邮件网关的各项服务,以及在
SecurityGateway 12.5 或更高版本上,其 REST API 公布的版本和性能
计数器:投递队列、隔离区、SMTP 会话。

!!! warning "仅依据文档验证"

    这项集成是根据 MDaemon Technologies 发布的 SecurityGateway
    及其 REST API 文档构建的,而不是针对一个真实的网关。每一次
    调用都是读取操作。该文档没有公布性能计数器的路径或名称,
    因此 DumbMonit 会在 API 自身的描述中查找该路径,并保留 API
    给出的每个计数器名称。目前还没有人在真实网关上运行过它。
    如果有任何问题,请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_securitygateway_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `service_up` | `service`, `port` | 当该端口接受连接,且对于 SMTP 以 `220` 问候时为 1。未应答,或应答表示拒绝服务(`421`、`554`)时为 0。 |
| `service_response_seconds` | `service`, `port` | 到收到问候的时间,或对于没有问候的端口,到建立连接的时间。 |
| `api_up` | | 仅在配置了 API 密钥时:REST API 应答时为 1,未应答时为 0。 |
| `api_response_seconds` | | REST API 应答所用的时间。 |
| `counters_available` | | 性能计数器被成功读取时为 1;API 未描述任何计数器,或该密钥所属账户无权读取时为 0。 |
| `counter` | `counter` | 性能计数器每个数值对应一条序列,按 API 给出的名称命名,采用 `snake_case` 格式:`deliveryQueue` 变为 `delivery_queue`。最多 64 个。 |
| `info`(值为 1) | `version`, `source` | API 描述中给出的版本(`source="openapi"`),或在网关于 SMTP 问候中公布版本时取自那里(`source="smtp_banner"`)。 |
| `scrape_duration_seconds` | | 整次检查所用的时间。 |

**计数器的查找方式。** SecurityGateway 的 REST API 在
`/api/v1/openapi` 处自我描述。DumbMonit 读取该描述,选取名称中
提到计数器、不带参数的最短只读路径,读取它,并保留应答中的每一个
数值。不会重命名任何内容,也不做任何假设:如果未来版本重命名了
某个计数器,该序列会跟随新名称。SecurityGateway 帮助文档中列出的
计数器包括当前的入站和出站 SMTP 会话、投递队列、管理员隔离区、
运行时间、域和用户计数,以及数据库连接池;其中一些每分钟才变化
一次。

**部分故障仍算作一次成功的检查。** 某个服务停止会让该服务的
`service_up` 为 0,其他服务仍会被正常测量。只有当没有任何服务、
API 也完全没有应答时,该监控项才算*不可达*。密钥被拒绝属于配置
错误,会显示在监控项页面上,不会发送通知。

**不会读取的内容。** 许可证及其到期时间、磁盘空间和控制台的
消息统计,都未在 REST API 的已发布文档中描述。要监控
SecurityGateway Windows 服务本身以及服务器的磁盘,请在服务器上
安装[代理](agent.md),并在其 `services` 设置中列出该服务。

适用的[内置规则](../alerting/rules.md#mdaemon-and-securitygateway):
SecurityGateway 服务宕机、SecurityGateway API 未应答、
SecurityGateway 投递队列增长,以及监控项不可达。

## 监控项页面 {#the-device-page}

SecurityGateway 监控项在图表上方会显示一个**邮件服务**面板,读取
自上次存储的测量数据:打开该页面从不会连接到网关。它列出每个被
监控的服务及其端口、一个状态词和一种颜色(应答中、已宕机)以及
其响应时间、版本,以及在配置了 API 密钥时,每个性能计数器的
最新值。

## 监控网关服务,并可选地监控 REST API {#watch-the-gateway-services-and-optionally-the-rest-api}

1. 服务器上无需安装任何东西。不配置凭据时,DumbMonit 会从外部连接 SMTP(25)和网页界面(4000),并读取 SMTP 问候。可在"服务"选项中修改这个列表,例如改为检查 HTTPS 而不是 HTTP。

    ```
    smtp,web_https
    ```

2. 若要同时读取性能计数器,需要 SecurityGateway 12.5 或更高版本。创建一个按如下方式命名的专用账户,使用一个不在任何其他地方使用的较长密码,并赋予它 Domain Administrator 角色。如果监控项页面上的计数器仍为空,说明你的版本把它们保留给 Global Administrator 角色,该账户需要改用那个角色。

    ```
    dumbmonit
    ```

3. 以该账户登录,在 Setup/Users → Accounts → API Keys 下创建一个密钥,并设置过期日期。密钥携带创建它的那个账户的权限,这就是为什么必须从这个账户而不是你自己的账户创建它。

4. 在该账户的 IP 限制中,将其限制为 DumbMonit 连接所用的地址:API 密钥同样遵循这些限制。

5. 从 DumbMonit 所在主机确认该密钥可用。该命令会打印出 API 描述,其中也列出了 DumbMonit 读取的计数器。

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. 在 DumbMonit 中,输入网关地址,例如 "sg.example.com",并粘贴该密钥。API 通过网页界面的 HTTPS 端口访问,默认为 4443,路径在 /api/v1 下。

!!! warning "警告"
    SecurityGateway 没有只读角色,也没有更受限的密钥:该密钥能做
    其所属账户能做的一切。绝不要从你自己的账户创建它,只把它
    保存在 DumbMonit 中,并给它设置一个过期日期。12.5 之前的
    版本没有 REST API:将凭据留空,DumbMonit 只会检查这些服务。
    网页界面在 4443 上通常使用自签名证书:如果因此连接被拒绝,
    请在选项中勾选"接受无法验证的证书"。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 仅服务 | 不需要:这些端口会在不登录的情况下被检查。 |
| API 密钥(12.5 及更高版本) | 该密钥,以 `Authorization: Bearer …` 发送。 |

地址:网关的主机名或 IP,例如 `sg.example.com`。写在地址中的端口
(`sg.example.com:8443`)会被当作 API 端口。

## 选项 {#options}

| 选项 | 默认值 | 作用 |
|---|---|---|
| `services` | `smtp,web` | 要检查的内容,用逗号分隔:`smtp`(25)、`smtps`(465)、`web`(4000)、`web_https`(4443)。`name:port` 可以更改某个端口;其他任意名称配合端口会添加你自己的服务,只检查连接是否成功。`none` 不检查任何端口。 |
| `request_timeout_seconds` | `10` | 每次连接、问候和 API 调用允许的时间,1 到 60。 |
| `api_port` | `4443` | 提供 `/api/v1` 的网页界面端口。仅在配置了 API 密钥时使用。 |
| `api_tls` | `true` | 使用 HTTPS 访问 REST API。只有当网页界面通过纯 HTTP(默认端口 4000)提供服务时才取消勾选:此时密钥会以明文方式经过网络传输。 |
| `insecure_tls` | `false` | 在 REST API 上接受自签名证书。 |
| `counters` | `true` | 读取性能计数器。 |
