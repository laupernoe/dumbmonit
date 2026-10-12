# VictoriaMetrics 和 VictoriaLogs

时间序列数据库和日志数据库本身:它们是否接受发送给它们的数据、
在停止接受任何内容之前还剩多少磁盘空间,以及它们拒绝了什么。

两种设备类型共享这个页面,因为这两款产品共享同样的基础:
`victoriametrics` 对应单节点 VictoriaMetrics,`victorialogs`
对应单节点 VictoriaLogs。两者都在各自 API 的端口上通过 `/health`
和 `/metrics` 公布自身的健康状况;DumbMonit 只读取这两个接口,
不做其他任何事。它从不对你的数据运行任何查询,也从不写入任何
数据。

这些服务器很少彻底宕机。它们是*半宕机*:它们仍会应答查询,
仪表盘仍能打开,而每一个新的样本或日志行都被拒绝,因为磁盘超过
了只读限制,或数据带有超出保留期的时间戳。任何向它们推送数据
的东西(vmagent、Prometheus remote write、Vector、Fluent Bit、
Promtail)会重试,然后丢弃。这个页面的意义就是提前发现这一点。

## 监控内容 {#what-it-watches}

指标以 `dumbmonit_victoriametrics_` 或 `dumbmonit_victorialogs_`
为前缀。总量是计数器:图表和规则会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `healthy` | 当 `/health` 应答 OK 时为 1,其他任何应答为 0 | |
| `version_info` | 固定值 1 | `version` |
| `rows_ingested_total` | 已接受的样本数(VictoriaMetrics)或日志行数(VictoriaLogs) | |
| `bytes_ingested_total` | 已接受的日志量(仅 VictoriaLogs) | |
| `rows_rejected_total` | 所有被拒绝的内容,始终存在,即使为零也会出现 | |
| `rows_rejected_reason_total` | 同上,按原因划分:`big_timestamp`、`small_timestamp`、`too_long_label_value`、`too_many_labels`、`invalid`、`hourly_series_limit`……(VictoriaMetrics);`too_big_timestamp`、`too_small_timestamp`、`too_many_fields`、`too_long_line`(VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | 存储卷的剩余和总空间 | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`:低于此值,存储会变为只读 | `path` |
| `disk_headroom_percent` | 高于该限制的剩余空间,占磁盘的比例 | `path` |
| `read_only` | 存储一旦切换为只读时为 1 | |
| `data_size_bytes` | 磁盘上的数据量 | |
| `slow_inserts_total`, `rows_added_total` | 走慢速路径的新样本数,以及添加的所有样本数(仅 VictoriaMetrics) | |
| `active_series` | 过去一小时内收到样本的序列数(仅 VictoriaMetrics) | |
| `http_errors_total` | 应答为错误的请求数 | |
| `log_errors_total` | 服务器记录的错误消息数 | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**被拒绝的样本会丢失。** VictoriaMetrics 会拒绝超过其保留期的
样本、未来超过两天的样本、超过 `-maxLabelValueLen` 的标签值、
超过 `-maxLabelsPerTimeseries` 的序列,以及一旦达到
`-storage.maxHourlySeries` 或 `-storage.maxDailySeries` 后新
序列的每一个样本。VictoriaLogs 会拒绝超出其保留期的行以及字段
过多的行。客户端通常对此不会收到任何错误:只有这个计数器知道。

**慢速插入**指的是 VictoriaMetrics 必须在磁盘上查找、而不是在
缓存中查找其序列的样本。每一个全新的序列都算一次,因此一次部署
会产生一波突增;如果比例持续高于 5%,说明服务器的内存不足以
支撑其活跃序列。这是 VictoriaMetrics 文档给出的阈值。

适用的[内置规则](../alerting/rules.md#log-and-metrics-servers):
指标或日志存储为只读、指标或日志存储接近只读、指标或日志服务器
不健康、指标或日志数据被拒绝、VictoriaMetrics 慢速插入,以及
设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询服务器。
它按以下顺序说明:

1. **一句话说明哪里出了问题**:健康状况、存储(*只读*,或高于
   限制的剩余磁盘空间比例)、过去 24 小时内被拒绝的数据、慢速
   插入、记录的错误。每一行都配有一个词和一种颜色。
2. **服务器正在做什么**:每秒写入的样本数或行数、活跃序列数或
   写入量、磁盘上的数据量、运行时间。
3. **过去 24 小时内按原因划分的被拒绝内容**,以及每个存储路径的
   剩余空间相对于其只读限制。

## 让 DumbMonit 读取 VictoriaMetrics 自身的健康状况 {#let-dumbmonit-read-victoriametrics-own-health}

1. 无需安装任何东西:VictoriaMetrics 在其 API 端口(默认 8428)上通过 /health 和 /metrics 公布自身的健康状况。从 DumbMonit 所在主机确认两者都能应答。

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. 如果 VictoriaMetrics 以 -httpAuth.username 和 -httpAuth.password 运行,请在下方选择用户名/密码并填入。位于 vmauth 或期望 bearer token 的反向代理之后时,请选择 Bearer token。如果设置了 -metricsAuthKey,/metrics 只会在 URL 中带有该密钥时才应答,而 DumbMonit 不会发送它:请改为用上述基本认证来保护 /metrics。

3. 在 DumbMonit 中,输入服务器地址,例如 "victoriametrics.lan" 或 "http://10.0.0.5:8428"。位于反向代理之后时,请输入带路径前缀的完整 URL。这里只覆盖单节点服务器;VictoriaMetrics 集群的各个组件公布的是其他指标,不在本页覆盖范围内。

4. DumbMonit 只会读取 /health 和 /metrics。它从不对你的数据运行任何查询,也从不写入任何内容。

!!! warning "警告"
    剩余磁盘空间是需要关注的重点:低于
    -storage.minFreeDiskSpaceBytes(默认 100 MB)时,
    VictoriaMetrics 会切换为只读并拒绝每一个新样本,而它仍会
    像什么都没发生一样继续应答查询。DumbMonit 会显示高于该限制
    的剩余空间,并在低于 10% 时发出警告。

## 让 DumbMonit 读取 VictoriaLogs 自身的健康状况 {#let-dumbmonit-read-victorialogs-own-health}

1. 无需安装任何东西:VictoriaLogs 在其 API 端口(默认 9428)上通过 /health 和 /metrics 公布自身的健康状况。从 DumbMonit 所在主机确认两者都能应答。

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_rows_ingested_total
    ```

2. 如果 VictoriaLogs 以 -httpAuth.username 和 -httpAuth.password 运行,请在下方选择用户名/密码并填入。位于 vmauth 或期望 bearer token 的反向代理之后时,请选择 Bearer token。如果设置了 -metricsAuthKey,/metrics 只会在 URL 中带有该密钥时才应答,而 DumbMonit 不会发送它:请改为用上述基本认证来保护 /metrics。

3. 在 DumbMonit 中,输入服务器地址,例如 "victorialogs.lan" 或 "http://10.0.0.6:9428"。位于反向代理之后时,请输入带路径前缀的完整 URL。

4. DumbMonit 只会读取 /health 和 /metrics。它从不运行任何 LogsQL 查询,也从不读取任何日志行。

!!! warning "警告"
    剩余磁盘空间是需要关注的重点:低于
    -storage.minFreeDiskSpaceBytes(默认 10 MB)时,
    VictoriaLogs 会切换为只读并拒绝每一个新日志行。DumbMonit
    会显示高于该限制的剩余空间,并在低于 10% 时发出警告。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无认证 | 默认情况:除非你保护了它们,否则 `/health` 和 `/metrics` 是开放的。 |
| 用户名/密码 | 服务器的 `-httpAuth.username` 和 `-httpAuth.password`,或其前面反向代理的凭据。 |
| Bearer 令牌 | vmauth 或你的反向代理期望在 `Authorization: Bearer` 中收到的令牌。 |

地址:主机名或 IP(`victoriametrics.lan`、`10.0.0.5`)、
`host:port`,或反向代理之后带路径前缀的完整 URL
(`https://metrics.lan/vm`)。选项:协议(默认 HTTP)、端口
(8428 或 9428)、证书校验和请求超时(10 秒)。
