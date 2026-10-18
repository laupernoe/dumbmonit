# 监视 DumbMonit 自身 {#watching-dumbmonit-itself}

谁来监视监视者?DumbMonit 的回答分为两部分:

- 一个**外发心跳**,用于它从内部无法看到的情况:主机宕机、容器停止、
  引擎卡住;
- 针对自身健康状况的**内置规则**,用于它能看到的情况:数据卷快满、
  指标数据库不断循环重启、轮询停滞、写入失败、数据库异常增长。

两者都位于**设置 → 系统 → DumbMonit 自身**,该页面还会显示引擎当前的
状态。

## 外发心跳 {#the-outgoing-heartbeat}

DumbMonit 会按固定间隔(默认 60 秒)调用你提供的 URL,而且**只在引擎
健康时**才调用:

- 调度器正在运行(最近两分钟内完成了一轮);
- SQLite 数据库有应答;
- VictoriaMetrics 有应答。

只要其中之一失败,就什么也不发送。"死人开关"(dead man's switch)服务
会等待这些调用,并在它们停止时提醒你,无论原因是整台主机、容器还是
某个组件。

### 设置方法 {#setting-it-up}

1. 在你选择的服务上创建一个检查,并复制它的 ping URL:
    - **Healthchecks.io**(或自托管的 Healthchecks):周期等于该间隔、
      宽限时间略长一些的检查(`https://hc-ping.com/<uuid>`);
    - **Uptime Kuma**:*Push* 类型的监控项
      (`https://kuma.example.com/api/push/<token>?status=up&msg=OK&ping=`);
    - **Better Stack**:一个心跳(`https://uptime.betterstack.com/api/v1/heartbeat/<token>`);
    - 任何接受普通 `GET` 并在调用不再到来时发出告警的服务
      (Cronitor、Dead Man's Snitch……)。
2. 在**设置 → 系统 → DumbMonit 自身**中,粘贴该 URL,选择间隔
   (30 到 3600 秒),开启**发送心跳**并保存。
3. 点击**测试**:一次调用会立即发出,并显示结果。该服务应将此检查标记
   为正常。

该调用是一个超时为 10 秒的 `GET`。它遵循与通知渠道相同的防护:绝不会
发往 DumbMonit 自己的网页界面或它的 VictoriaMetrics。

!!! note "ntfy"
    ntfy 会投递它收到的每条消息,因此把心跳指向 ntfy 主题会让你每分钟
    收到一条通知,而不是在调用停止时才通知。请使用死人开关服务,并把
    ntfy 留作告警本身的渠道。

### 故障信号 {#failure-signal}

默认情况下,出问题的引擎只是停止调用,服务会在宽限时间过后发出告警。
开启**报告故障**后,会改为调用故障 URL,服务会立即报告:

| 服务 | 故障 URL |
|---|---|
| Healthchecks.io、Better Stack | 在路径后添加 `/fail`:`https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | 查询参数中的 `status=up` 替换为 `status=down` |

对任何其他服务请保持关闭:末尾带 `/fail` 的未知 URL 很可能应答 404,
这看起来与什么也没发送无异。

### URL 是一个机密 {#the-url-is-a-secret}

ping URL 通常包含它自己的令牌:任何知道它的人都能让你的检查看起来
正常。DumbMonit 用实例密钥加密保存它,绝不通过 API 返回它(只显示其
主机),并且不会写入日志和错误消息。要更改它,请输入新的;要保留它,
请让该字段留空。**移除 URL** 会遗忘它并关闭心跳。

该设置不属于加密的配置包:恢复到新实例后请重新设置。

## 内置规则 {#built-in-rules}

DumbMonit 每分钟会向 VictoriaMetrics 写入几项关于自身的测量值,*DumbMonit
自身*组的内置规则读取它们。它们的行为与其他规则一样:相同的渠道、相同的
静默、**告警 → 规则**中相同的按规则阈值。

| 规则 | 触发条件 | 默认值 |
|---|---|---|
| DumbMonit 数据卷快满 | 存放 `/data` 的卷剩余空间不足 10% | 15 分钟,warning |
| DumbMonit 指标数据库循环重启 | 内嵌的 VictoriaMetrics 在 30 分钟内重启了 3 次或更多 | 1 分钟,critical |
| DumbMonit 调度器停滞 | 超过两分钟没有调度轮次 | 5 分钟,critical |
| DumbMonit 轮询延迟 | 在 15 分钟内,每一轮之后监控项仍然逾期 | 15 分钟,warning |
| DumbMonit 指标写入失败 | 最近 15 分钟内有批次被 VictoriaMetrics 拒绝,或有样本被丢弃 | 5 分钟,warning |
| DumbMonit 数据库快速增长 | `dumbmonit.db` 一天内增长超过一倍,且超过 100 MB | 1 小时,warning |
| DumbMonit 备份未运行 | 两天内没有成功的本地备份([详情](backup.md#scheduled-local-backups)) | 1 小时,warning |

这些规则需要 VictoriaMetrics 才能评估。当 VictoriaMetrics 本身长时间宕机,
或整台主机宕机时,没有任何规则能触发:这就是心跳的工作。

### 测量值 {#measurements}

| 指标 | 含义 |
|---|---|
| `dumbmonit_instance_data_free_bytes` | 存放 `/data` 的卷上,非特权用户可用的剩余空间。 |
| `dumbmonit_instance_data_size_bytes` | 该卷的大小。 |
| `dumbmonit_instance_database_bytes` | `dumbmonit.db` 及其预写日志的大小。 |
| `dumbmonit_instance_scheduler_lag_seconds` | 距调度器上次完成一轮的秒数(通常小于 2)。 |
| `dumbmonit_instance_scheduler_backlog` | 上一轮之后仍待探测的监控项。 |
| `dumbmonit_instance_sample_writes_failed_total` | 自服务器启动以来,VictoriaMetrics 拒绝或未应答的批次。 |
| `dumbmonit_instance_samples_dropped_total` | 自服务器启动以来,在到达写入缓冲区之前被丢弃的样本。 |
| `dumbmonit_instance_vm_restarts_total` | 自服务器启动以来内嵌 VictoriaMetrics 的重启次数(仅限内嵌)。 |

它们不带 `target` 标签:它们引发的告警不附属于某个监控项。对于你自己的
Prometheus,`GET /metrics` 会暴露同类数据,参见
[抓取 DumbMonit](../reference/metrics.md#scraping-dumbmonit)。

## API {#api}

| 方法 | 路由 | |
|---|---|---|
| `GET` | `/api/self-monitoring` | 引擎状态、数据卷、数据库大小、心跳设置和上一次心跳。 |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`。省略 `url` 表示保留,`""` 表示清除。 |
| `POST` | `/api/self-monitoring/heartbeat/test` | 立即调用一次,发往 `{ "url" }` 或已保存的 URL。 |

这两个写入操作面向管理员(或具有 `write` 范围的令牌)。
