# CrowdSec {#crowdsec}

安全引擎:当前有效的决策及其来源、告警、停止拉取的 bouncer、
已读取的日志行以及没有任何解析器识别的日志行,以及本地 API 是否
应答。

DumbMonit 读取 CrowdSec 的 Prometheus 指标(端口 6060),并检查
本地 API(端口 8080)的 `/health`。配合一个可选的 bouncer 密钥,
它还会向本地 API 查询 192.0.2.1(一个文档专用地址)上的决策:
应答始终为空,这只是为了证明认证和数据库能够正常响应。它从不
创建或删除任何决策,也从不下载封禁列表。

值得关注的故障是某个 bouncer 停止拉取。防火墙或反向代理会继续
使用它已有的列表运行,新的攻击者得以通过,而没有任何提示。
CrowdSec 不会公布某个 bouncer 上次拉取的时间;DumbMonit 从该
bouncer 的请求计数器读取这一信息——当它不再变化时就说明问题。
第二种故障是某个采集源出了问题(日志文件被重命名、容器被重建):
CrowdSec 什么都读不到,看不到任何攻击,却仍显示为绿色正常。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_crowdsec_` 为前缀。总量是计数器:图表和
规则会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `lapi_up` | 当本地 API 对 `/health` 应答时为 1 | |
| `lapi_decisions_ok` | 配合 bouncer 密钥:当本地 API 成功应答决策查询时为 1 | |
| `decisions` | 当前有效的决策数,包含社区黑名单 | |
| `decisions_by_origin` | | `origin`:`crowdsec`、`cscli`、`CAPI`、`lists`…… |
| `alerts` | 本地 API 保留的告警数,不含社区告警 | |
| `bouncers_seen` | 自 CrowdSec 启动以来调用过本地 API 的 bouncer 数 | |
| `bouncer_requests_total` | 每个 bouncer 的请求数;当它不再变化时,说明该 bouncer 停止拉取了 | `bouncer` |
| `machine_requests_total` | 每个代理(机器)的请求数 | `machine` |
| `lapi_requests_total` | 对本地 API 的全部请求数 | |
| `scenario_overflows_total` | 本地代理触发的场景数:每一个都是一条告警 | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | 代理读取的日志行数,以及已解析和未解析的行数 | |
| `source_lines_total`, `source_lines_unparsed_total` | 按日志来源统计的同样内容 | `source` |
| `buckets` | 当前打开的场景桶数 | |

本地 API 相关的指标族出现在本地 API 运行的地方,日志相关的指标族
出现在代理读取日志的地方;官方镜像和标准安装会同时运行两者。

适用的[内置规则](../alerting/rules.md#databases-message-broker-and-security-engine):
CrowdSec 本地 API 宕机、CrowdSec bouncer 停止拉取、CrowdSec 未读取
任何日志,以及监控项不可达。

## 监控项页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询
CrowdSec。它会用一句话说明哪里出了问题(本地 API、bouncer、日志
读取,以及没有任何解析器识别的行的比例),然后是当前有效的决策数、
保留的告警数、过去 24 小时内触发的场景数和每秒读取的行数,再然后
是每个 bouncer 上次拉取至今的时间、按来源划分的决策,以及每个
日志来源未解析的比例。

## 让 DumbMonit 读取 CrowdSec 的指标 {#let-dumbmonit-read-crowdsecs-metrics}

1. CrowdSec 默认在 127.0.0.1:6060 上公布其指标。将 /etc/crowdsec/config.yaml 中 prometheus 部分的 listen_addr 设置为 DumbMonit 能访问的地址,然后重启 CrowdSec。官方 Docker 镜像已经监听了所有网络接口。

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. 这些指标不需要任何凭据:它们只包含计数、bouncer 和机器名称以及日志文件路径,绝不包含 IP 地址。请过滤 6060 端口,使只有 DumbMonit 所在主机能访问它,并确认它能应答。

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. DumbMonit 还会检查同一主机 8080 端口上本地 API 的 /health:这就要求 api.server 部分中的 listen_uri 也必须可达。对于只运行代理的主机,请取消勾选"检查本地 API"。

4. 可选:为 DumbMonit 创建一个 bouncer 密钥,并将其粘贴到下方的"Bouncer API key"中。bouncer 密钥只能读取决策。DumbMonit 查询的是 192.0.2.1(一个文档专用地址)上的决策,因此应答始终为空,封禁列表也永远不会被下载。

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. 在 DumbMonit 中,输入 CrowdSec 主机地址,例如 "crowdsec.lan"。如果本地 API 和代理运行在不同的主机上,请添加每个主机:决策、告警和 bouncer 会在本地 API 运行的地方被统计,日志行会在每个代理读取它们的地方被统计。

!!! warning "警告"
    停止拉取的 bouncer 会继续执行一份已经冻结的列表:新的攻击者
    得以通过,而没有任何提示。当某个 bouncer 的请求停止 30 分钟
    时,DumbMonit 会发出警告。它只认识自 CrowdSec 上次启动以来
    至少拉取过一次的 bouncer。

用 `cscli bouncers delete` 删除的 bouncer,其计数器会一直保留到
CrowdSec 重启为止,期间会一直被报告为已停止。只有在处理请求时
才调用本地 API 的 bouncer(实时模式下的反向代理 bouncer),在
平静的夜晚可能半小时都不发声;可以把它切换到流模式,或者调大
该规则的窗口时间。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 仅指标 | 不需要:读取 `/metrics`,以及本地 API 的 `/health`。 |
| Bouncer API 密钥 | 由 `cscli bouncers add dumbmonit` 打印出的密钥,以 `X-Api-Key` 发送给本地 API。 |

地址:主机名或 IP(`crowdsec.lan`)、用于指标的 `host:port`,或
完整 URL。选项:协议(默认 HTTP)、指标端口(6060)、检查本地 API
(开启)、本地 API 端口(8080)、证书校验和请求超时(10 秒)。
被污染或过期的 Hub 项目(解析器、场景、集合)无法通过指标看到,
只能在主机上用 `cscli hub list` 查看。
