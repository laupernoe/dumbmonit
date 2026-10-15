# VictoriaMetrics和VictoriaLogs {#victoriametrics-and-victorialogs}

时间序列数据库和日志数据库本身：它们是否接受发送给它们的内容、在停止接受任何内容之前剩余多少磁盘空间，以及它们拒绝什么。

两个监控项类型共享此页面，因为两个产品共享其基础：`victoriametrics`用于单节点VictoriaMetrics，`victorialogs`用于单节点VictoriaLogs。两者都在其API的端口上的`/health`和`/metrics`上发布自己的健康状态；DumbMonit读取这两个，仅此而已。它从不针对您的数据运行查询，也从不写入。

这些服务器很少完全宕机。它们半途宕机：它们继续应答查询，仪表板仍然打开，而每个新样本或日志行都被拒绝，因为磁盘已越过只读限制或数据带有超出保留期的时间戳。推送给它们的任何内容（vmagent、Prometheus远程写入、Vector、Fluent Bit、Promtail）重试，然后丢弃。此页面是关于看到这种情况即将发生。

## 监控内容 {#what-it-watches}

指标前缀为`dumbmonit_victoriametrics_`或`dumbmonit_victorialogs_`。总值是计数器：图表和规则将其转换为速率。

| 指标 | 说明 | 标签 |
|---|---|---|
| `healthy` | `/health`应答OK时为1，应答其他任何内容时为0 | |
| `version_info` | 值为1 | `version` |
| `rows_ingested_total` | 接受的样本（VictoriaMetrics）或日志行（VictoriaLogs） | |
| `bytes_ingested_total` | 接受的日志量（仅VictoriaLogs） | |
| `rows_rejected_total` | 所有拒绝，始终存在，即使为零 | |
| `rows_rejected_reason_total` | 相同，按原因：`big_timestamp`、`small_timestamp`、`too_long_label_value`、`too_many_labels`、`invalid`、`hourly_series_limit`…（VictoriaMetrics）；`too_big_timestamp`、`too_small_timestamp`、`too_many_fields`、`too_long_line`（VictoriaLogs） | `reason` |
| `disk_free_bytes`、`disk_total_bytes` | 存储卷的可用和总空间 | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`：低于此值，存储转为只读 | `path` |
| `disk_headroom_percent` | 低于该限制的可用空间，作为磁盘的份额 | `path` |
| `read_only` | 存储切换为只读后为1 | |
| `data_size_bytes` | 磁盘上的数据 | |
| `slow_inserts_total`、`rows_added_total` | 走缓慢路径的新样本及所有添加的样本（仅VictoriaMetrics） | |
| `active_series` | 在过去一小时内收到样本的序列（仅VictoriaMetrics） | |
| `http_errors_total` | 以错误回答的请求 | |
| `log_errors_total` | 服务器记录的错误消息 | |
| `uptime_seconds`、`memory_resident_bytes` | | |

**被拒绝的样本丢失。** VictoriaMetrics拒绝早于其保留期的样本或超过两天的未来样本、长于`-maxLabelValueLen`的标签值、超过`-maxLabelsPerTimeseries`的序列，以及一旦达到`-storage.maxHourlySeries`或`-storage.maxDailySeries`的新序列的每个样本。VictoriaLogs拒绝超出其保留期和字段过多的行。客户端通常不会收到这些的错误：仅此计数器知道。

**缓慢插入**是其序列VictoriaMetrics不得不在磁盘上查找而不是在其缓存中的样本。每个全新序列都是一个，所以部署会导致突发；保持在5%以上的份额意味着服务器缺乏其活跃序列的内存。这是VictoriaMetrics文档给出的阈值。重启后服务器的缓存是冷的，所以份额可能有一段时间保持高于这个阈值：规则在触发前等待半小时。计数器与服务器重启时重置为零；规则使用重置感知增量，所以重启不计为拒绝的数据。

适用的[内置规则](../alerting/rules.md#log-and-metrics-servers)：指标或日志存储只读、指标或日志存储几乎只读、指标或日志服务器不健康、指标或日志数据拒绝、VictoriaMetrics缓慢插入，加上监控项不可达。

## 监控项页面 {#the-device-page}

图表上方的面板读取探针存储的内容；打开页面从不查询服务器。它按此顺序说明：

1. **用一句话说明有什么问题**：健康、存储（*只读*或磁盘在限制上方剩余多少）、过去24小时内拒绝的数据、缓慢插入、记录的错误。每行都包含一个单词和一种颜色。
2. **服务器做什么**：每秒摄取的样本或行、活跃序列或摄取的卷、磁盘上的数据、正常运行时间。
3. **过去24小时内拒绝的内容，按原因**，以及每个存储路径的可用空间与其只读限制。

## 让DumbMonit读取VictoriaMetrics自己的健康状态 {#let-dumbmonit-read-victoriametrics-own-health}

1. 无需安装：VictoriaMetrics在其API的端口（默认8428）上的/health和/metrics上发布自己的健康状态。从DumbMonit主机检查两者都应答。

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. 如果VictoriaMetrics运行时带有-httpAuth.username和-httpAuth.password，选择下面的用户名/密码并输入那些。在vmauth或期望不记名令牌的反向代理后，选择不记名令牌。如果设置了-metricsAuthKey，/metrics仅用URL中的密钥应答，DumbMonit不发送：改用上面的基本认证保护/metrics。

3. 在DumbMonit中，输入服务器地址，例如"victoriametrics.lan"或"http://10.0.0.5:8428"。在反向代理后，输入带其路径前缀的完整URL。这覆盖单节点服务器；VictoriaMetrics集群的组件公开其他指标，未覆盖。

4. DumbMonit仅读取/health和/metrics。它从不针对您的数据运行查询，也从不写入任何内容。

!!! warning
    可用磁盘空间是要监控的：低于-storage.minFreeDiskSpaceBytes（默认100MB）VictoriaMetrics切换为只读并拒绝每个新样本，而它继续应答查询，好像没什么发生。DumbMonit显示该限制上方剩余的余量，并在10%以下警告。

## 让DumbMonit读取VictoriaLogs自己的健康状态 {#let-dumbmonit-read-victorialogs-own-health}

1. 无需安装：VictoriaLogs在其API的端口（默认9428）上的/health和/metrics上发布自己的健康状态。从DumbMonit主机检查两者都应答。

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_free_disk_space_bytes
    ```

2. 如果VictoriaLogs运行时带有-httpAuth.username和-httpAuth.password，选择下面的用户名/密码并输入那些。在vmauth或期望不记名令牌的反向代理后，选择不记名令牌。如果设置了-metricsAuthKey，/metrics仅用URL中的密钥应答，DumbMonit不发送：改用上面的基本认证保护/metrics。

3. 在DumbMonit中，输入服务器地址，例如"victorialogs.lan"或"http://10.0.0.6:9428"。在反向代理后，输入带其路径前缀的完整URL。

4. DumbMonit仅读取/health和/metrics。它从不运行LogsQL查询，也从不读取日志行。

!!! warning
    可用磁盘空间是要监控的：低于-storage.minFreeDiskSpaceBytes（默认10MB）VictoriaLogs切换为只读并拒绝每个新日志行。DumbMonit显示该限制上方剩余的余量，并在10%以下警告。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无认证 | 默认值：除非您保护了`/health`和`/metrics`，否则它们是开放的。 |
| 用户名/密码 | 服务器或其前面反向代理的`-httpAuth.username`和`-httpAuth.password`。 |
| 不记名令牌 | vmauth或您的反向代理期望在`Authorization: Bearer`中的令牌。 |

地址：主机名或IP（`victoriametrics.lan`、`10.0.0.5`）、`host:port`或带反向代理后面路径前缀的完整URL（`https://metrics.lan/vm`）。选项：协议（默认HTTP）、端口（8428或9428）、证书检查和请求超时（10秒）。
