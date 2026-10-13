# Grafana Loki {#grafana-loki}

日志服务器本身:它是否就绪、拒绝了哪些日志行以及原因、它的数据块
是否成功写入存储,以及它的预写日志(WAL)磁盘是否还有空间。

Loki 在其 HTTP 端口上发布 `/ready` 和 `/metrics`;DumbMonit 只读取
这两个接口,不做其他任何事。它从不执行 LogQL 查询,也从不读取
任何日志行。

值得关注的故障往往很隐蔽。当 Loki 拒绝某一行日志时(早于
`reject_old_samples_max_age`、超过租户的写入速率或流限制、
超过 `max_line_size`),发送方会收到 4xx 响应,而 Promtail、
Alloy 或 Fluent Bit 通常会直接丢弃该批次。Grafana 什么都不会
显示,只会出现一个空档,直到某天有人去查某个事件的日志时才会
发现。第二种情况是数据块无法写入存储:Loki 会把它们留在内存中,
并在下次重启时丢失它们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_loki_` 为前缀。总量是计数器:图表和规则
会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `ready` | 当 `/ready` 返回 `ready` 时为 1,Loki 启动过程中或某个组件无法读写时为 0 | |
| `version_info` | 固定值 1 | `version` |
| `received_lines_total`, `received_bytes_total` | distributor 收到的内容,涵盖所有租户 | |
| `discarded_lines_total` | 被拒绝的行数,始终存在,即使为零也会出现 | |
| `discarded_lines_reason_total` | 按原因细分:`greater_than_max_sample_age`、`rate_limited`、`stream_limit`、`line_too_long`、`per_stream_rate_limit`…… | `reason` |
| `flush_failures_total`, `flush_queue_length` | 未能写入存储的数据块数量,以及正在等待写入的数据块数量 | |
| `memory_streams` | ingester 在内存中保存的流数量 | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | 因磁盘满而丢失的预写日志写入次数,以及该磁盘的使用率 | |
| `requests_total`, `request_errors_total` | HTTP 和 gRPC 请求数,以及以 5xx 或 gRPC 错误应答的请求数。4xx 属于客户端的问题,不计为错误。 | |
| `ring_unhealthy_members` | 哈希环中被标记为不健康的成员数量 | |
| `panics_total`, `log_errors_total` | 已恢复的 panic 数量,记录的错误消息数量 | |
| `retention_last_run_timestamp_seconds` | compactor 上次成功运行保留策略的时间(当保留策略开启时) | |
| `memory_resident_bytes` | | |

拆分为多个组件的 Loki(simple scalable 或 microservices 模式)会在
相应组件上发布这些指标族:拒绝的日志行在 distributor 上,flush 和
WAL 在 ingester 上。添加你想要监控的组件;某个组件没有的指标族
只是不会产生任何序列。

适用的[内置规则](../alerting/rules.md#log-and-metrics-servers):
Loki 未就绪、Loki 拒绝日志行、Loki 无法写入存储、Loki 预写日志磁盘
已满、Loki 请求错误,以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Loki。
它会用一句话说明哪里出了问题(就绪状态、过去 24 小时内被拒绝的
行数、写入失败的数据块、WAL 磁盘已满、不健康的环成员、服务器
错误),然后是每秒接收的行数和数据量、内存中的流和 WAL 磁盘情况,
再然后是过去 24 小时内按原因细分的被拒绝行数。

## 让 DumbMonit 读取 Loki 自身的健康状态 {#let-dumbmonit-read-lokis-own-health}

1. 无需安装任何东西:Loki 在其 HTTP 端口(默认 3100)上发布 /ready 和 /metrics。从 DumbMonit 所在主机确认两者都能应答。

    ```
    curl http://loki.lan:3100/ready
    curl -s http://loki.lan:3100/metrics | grep loki_build_info
    ```

2. Loki 本身没有登录机制。如果它位于要求输入用户名和密码的反向代理之后,请在下方选择用户名/密码;如果是 Bearer 令牌,请选择 Bearer 令牌。无需指定租户:/ready 和 /metrics 不区分租户。

3. 在 DumbMonit 中,输入 Loki 的地址,例如 "loki.lan",或反向代理之后的 "https://logs.lan/loki"。对于拆分为多个组件的 Loki(simple scalable 或 microservices),至少要添加写入路径所在的组件,被拒绝的行数和写入失败都在这里被统计。

4. DumbMonit 只读取 /ready 和 /metrics。它从不执行 LogQL 查询,也从不读取任何日志行。

!!! warning "警告"
    被拒绝的日志行是需要关注的故障:当 Loki 拒绝一行日志时(太旧、超过写入速率或流限制、行太长),发送方会收到错误,并通常会丢弃该批次。Grafana 中什么都不会显示,只会在日志中出现一个空档。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无认证 | 默认情况:Loki 本身没有登录机制。 |
| 用户名/密码 | Loki 前面反向代理的基本认证。 |
| Bearer 令牌 | 你的反向代理在 `Authorization: Bearer` 中期望的令牌。 |

地址:主机名或 IP(`loki.lan`)、`host:port`,或带路径前缀的完整 URL
(`https://logs.lan/loki`)。选项:协议(默认 HTTP)、端口(3100)、
证书校验和请求超时(10 秒)。
