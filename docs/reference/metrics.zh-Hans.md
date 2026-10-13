# 指标 {#metrics}

DumbMonit 将每一次测量存储在 VictoriaMetrics 中，并通过服务器的[指标代理](api.md#metrics)用 MetricsQL 读回。本页介绍命名、标签和保留期。各类型的指标列表见[设备页面](../devices/index.md)。

## 命名 {#naming}

每个指标都以 `dumbmonit_` 为前缀，因此 DumbMonit 可以与其他工具共用一个 VictoriaMetrics 实例而不会冲突。名称遵循 `dumbmonit_<what>_<unit>`：

| 模式 | 示例 |
|---|---|
| `…_bytes`、`…_bytes_total`、`…_bytes_used` | `dumbmonit_memory_bytes_total`、`dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`、`dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`、`…_age_seconds` | `dumbmonit_probe_duration_seconds`、`dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`、`…_volts`、`…_hertz`、`…_watts` | UPS 和 Synology 读数 |
| `…_info` | 存在性序列（值为 1），信息放在标签中：`dumbmonit_system_info`、`dumbmonit_pbs_version_info` |
| `…_status`、`…_up`、`…_running` | 枚举或布尔值：`dumbmonit_if_oper_status`、`dumbmonit_proxmox_guest_running` |

特定来源的指标族以来源作为第二个前缀：`dumbmonit_proxmox_*`、`dumbmonit_pbs_*`、`dumbmonit_synology_*`、`dumbmonit_probe_*`（服务）。SNMP 配置文件和代理使用普通名称（`dumbmonit_if_octets_in`、`dumbmonit_cpu_usage_percent`）。

计数器（接口的字节数和包数、磁盘字节数、已打印页数）按设备上报的值**原样**存储：在你自己的查询中用 `rate()` 得到吞吐量。设备页面按存储的原样绘制该设备的每条 `dumbmonit_*` 序列，因此计数器显示为一条上升的线。重启表现为计数器重置，绝不会是虚假的尖峰。

## `dumbmonit_up` {#dumbmonit_up}

每次探测成功后，服务器为该设备写入 `dumbmonit_up = 1`。探测失败时不写入任何内容：序列直接停止，而“设备不可达”规则度量最后一个样本的年龄（`time() - tlast_over_time(dumbmonit_up[7d])`），并且只保留每个设备最新的序列（`bottomk by (target) (1, …)`）：在标签或标记变更后不再写入的序列会留在 7 天窗口内，不应被计入。

对于服务（`http`、`tcp`、`dns`、`ping`、`tls`），`dumbmonit_up = 1` 表示“监控已运行”，而 `dumbmonit_probe_success` 表示服务是否正常。这两个信号被刻意分开：`probe_success` 为 0 表示服务宕机，`up` 中断则表示监控宕机。

## `dumbmonit_security_score` {#dumbmonit_security_score}

服务器每 15 分钟为每个具有安全检查的设备类型计算[安全评分](../using/security-score.md)，并写入 `dumbmonit_security_score`（0–100，无法评级时不存在）和 `dumbmonit_security_checks_failed`，两者都带 `target` 标签。

## 标签 {#labels}

| 标签 | 由谁设置 | 含义 |
|---|---|---|
| `target` | 服务器 | 设备 id，与 `/api/targets/{id}` 中一致。每条规则和每个图表都按它选择。 |
| `host` | 服务器 | 设备名称。 |
| `tag_<key>` | 服务器 | 设备的自由标签。类型选项（`insecure_tls`、`port`……）、键名看起来像机密的标签（按单词匹配：`key`、`pass`、`cert`、`private`、`pem`……）以及超过 256 个字符的值不会被复制。切勿把机密放进标签。 |
| `probe` | 服务 | `http`、`tcp`、`dns`、`ping` 或 `tls`。 |
| 各序列自己的标签 | 采集器 | `ifname`、`mountpoint`、`device`、`fstype`、`core`、`service`、`container`、`node`、`vmid`、`name`、`storage`、`datastore`、`group`、`volume`、`disk`、`pool`、`repo`、`changes`、`reason`、`version`、`issuer`……详见各设备页面。 |

身份标签始终优先于采集器标签，对代理而言，它们在接收时设置：一台机器无法写入另一台机器的序列。

!!! note "2026 年 9 月之前的前缀"
    EzyMonit 写入的序列名为 `ezymonit_…`。前缀随产品名称一起变更，没有任何东西会读取旧序列：它们会留在 VictoriaMetrics 中直到保留期将其清除，图表和规则从升级时重新开始，仍然引用 `ezymonit_…` 的自定义规则必须修改。

## 保留期与存储 {#retention-and-storage}

内置的 VictoriaMetrics 保留 12 个月的数据（`DUMBMONIT_VM_RETENTION`），位于 `/data/vm`，即 `dumbmonit-data` 卷中。服务器批量写入，并每 5 秒刷新一次（`DUMBMONIT_FLUSH_INTERVAL_SECS`）。删除设备不会删除其序列；它们会自然过期。

告警历史保留 90 天，消失超过 60 天的序列的基线也同样保留，二者都存放在 SQLite 中（参见[配置参考](configuration.md)）。

## 查询 {#querying}

- 通过服务器：`GET /api/metrics/query` 和 `GET /api/metrics/query_range`，使用会话 cookie。范围查询结果每条序列最多 2,000 个点。
- 从已有的 Prometheus 或 Grafana：`GET /federate`、`GET /metrics` 和 `/prometheus` 数据源，使用 API 令牌——见[下文](#scraping-dumbmonit)。
- 开发时直接访问：覆盖文件 `docker-compose.dev.yml` 会将内置的 VictoriaMetrics 发布在 `http://localhost:8428`，其自带界面位于 `http://localhost:8428/vmui/`。在生产环境中，它只监听容器的回环地址；如果希望 Grafana 或其他工具使用同一份数据，请设置 `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` 并自行发布该端口。

```
# Interface throughput in bits per second
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Availability of every service over 30 days
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Every device that has not reported for 5 minutes
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d])) > 300
```

## 抓取 DumbMonit {#scraping-dumbmonit}

如果你已经在运行 Prometheus 或 Grafana，就不需要第二套技术栈。有三个路由位于 `/api` 之外，也就是抓取器通常寻找它们的地方：

| 路由 | 返回内容 |
|---|---|
| `GET /metrics` | DumbMonit 实例自身的健康状况：调度器、探测、写入、告警、通知、代理、数据库。几十条序列，数量始终相同——绝不会每个设备一条。 |
| `GET /federate` | 由 `match[]` 选择的测量数据，与 Prometheus 联邦另一个 Prometheus 的方式完全相同：每条被选序列的最新点。 |
| `GET /prometheus/api/v1/…` | Prometheus API 的读取部分，中继到存储：即 Grafana 数据源所调用的。 |

这三者都接受带有 **`read`** 范围的 `Authorization: Bearer dmt_…`——在设置 → *API 与助手*中创建的令牌，与 [MCP 服务器](../using/assistant.md)使用的是同一种。不涉及 cookie，因此不需要 `X-Requested-With` 头；会话 cookie 也被接受，所以你可以在已登录的浏览器中打开其中任何一个。令牌限制为每分钟 120 次调用：对于每 15 秒一次的抓取绰绰有余，对于几十个面板的 Grafana 仪表板也足够——请给繁忙的仪表板单独分配一个令牌，而不要与抓取器共用。

### Prometheus {#prometheus}

```yaml
# prometheus.yml — DumbMonit's own health, then the measurements it collects.
scrape_configs:
  - job_name: dumbmonit
    metrics_path: /metrics
    scheme: http
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    static_configs:
      - targets: ['dumbmonit.lan:8080']

  - job_name: dumbmonit-federate
    metrics_path: /federate
    scheme: http
    honor_labels: true
    scrape_interval: 60s
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    params:
      'match[]':
        - '{__name__=~"dumbmonit_.*"}'
    static_configs:
      - targets: ['dumbmonit.lan:8080']
```

`honor_labels: true` 会保留 DumbMonit 设置的 `target` 和 `host` 标签，而不是用抓取任务自己的标签覆盖它们。如果不想把令牌放在配置里，可把令牌放进文件并使用 `authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`。

没有 `match[]` 时，`/federate` 选择 `{__name__=~"dumbmonit_.*"}`——即 DumbMonit 写入的每一条序列，并且不包含共用同一个 VictoriaMetrics 的其他工具的任何数据。可以缩小范围以保持联邦较小：对单个设备用 `{__name__=~"dumbmonit_.*",target="4"}`，只看可用性用 `{__name__="dumbmonit_up"}`。每个请求最多 10 个选择器，超过 8 MiB 的响应会被拒绝，并附带一条要求缩小范围的消息——请用范围更窄的选择器把大型设备群拆分成多个任务，而不是用一个任务请求全部。`max_lookback=15m` 会扩大查找最后一个点的时间窗口，适用于探测间隔超过五分钟的设备。

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana {#grafana}

让 Grafana 指向 **DumbMonit**，而不是它所运行的 VictoriaMetrics：该子进程监听容器的回环地址，自身没有认证，并且暴露*删除*序列的路由，就像暴露读取路由一样随意。DumbMonit 在 `/prometheus` 下中继 Prometheus API 的读取部分，并受同一令牌保护，因此 Grafana 与一个普通的 Prometheus 数据源通信：

1. **Connections → Data sources → Add → Prometheus**。
2. **URL**：`http://dumbmonit.lan:8080/prometheus`（Grafana 会自行追加 `/api/v1/query`）。
3. **HTTP headers**：添加一项，`Authorization` = `Bearer dmt_…`。Grafana 会加密存储它，并且不再显示。
4. **Save & test**——Grafana 报告数据源工作正常，此后每条 `dumbmonit_*` 序列都可在 PromQL 中使用，并对指标和标签名称提供自动补全。

只中继读取路由——`query`、`query_range`、`series`、`labels`、标签值、`metadata`，以及 Grafana 测试连接时请求的版本。其他任何路由，首先是 `admin/tsdb/delete_series`，都返回 `404`：这个入口无法更改或擦除任何内容。

如果你已经运行 Prometheus，另一条路径同样可行：Prometheus 用上面的任务联邦 DumbMonit，Grafana 读取 Prometheus，查询完全相同。

### 实例指标 {#instance-metrics}

`GET /metrics` 暴露的全部内容。数值在你抓取时读取；`_total` 计数器单调递增，仅在服务器重启时重置（`dumbmonit_uptime_seconds` 会告诉你何时发生）。

| 指标 | 类型 | 含义 |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | 始终为 1。`version` 标签是正在运行的服务器版本，与 `/api/health` 报告的相同。 |
| `dumbmonit_uptime_seconds` | gauge | 自服务器进程启动以来的秒数。 |
| `dumbmonit_scheduler_cycles_total` | counter | 已运行的调度周期数。循环每秒滴答一次，因此这也可作为存活信号。 |
| `dumbmonit_scheduler_cycle_seconds` | gauge | 上一个周期的持续时间。通常不到一毫秒：周期只做分派，不等待探测。 |
| `dumbmonit_scheduler_backlog` | gauge | 上一个周期结束时仍待探测的设备数。健康实例上为零；持续非零意味着 `DUMBMONIT_MAX_CONCURRENT_PROBES` 对设备数量及其间隔而言过低。 |
| `dumbmonit_probes_total{kind}` | counter | 已运行的探测，按设备类型（`snmp`、`proxmox`、`http`……）。包括中继代理代表服务器运行的那些。 |
| `dumbmonit_probes_failed_total{kind}` | counter | 失败的探测，按类型——设备不可达或配置错误。`rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` 即失败率。 |
| `dumbmonit_samples_written_total` | counter | VictoriaMetrics 接受的样本数。 |
| `dumbmonit_sample_writes_failed_total` | counter | VictoriaMetrics 拒绝或未响应的批次数。该批次仍保留在缓冲区中并重试；该值上升且 `dumbmonit_samples_pending` 同时上升，意味着存储已宕机或已满。 |
| `dumbmonit_samples_pending` | gauge | 上次刷新后仍留在写缓冲区中的样本数。通常为 0。 |
| `dumbmonit_samples_dropped_total` | counter | 在到达写缓冲区之前就被丢弃的样本数，原因是缓冲区已满或已关闭。不会重试——故障期间该值上升意味着数据丢失，而不仅是延迟。 |
| `dumbmonit_alerting_cycles_total` | counter | 已运行的告警周期数（每 `DUMBMONIT_ALERT_INTERVAL_SECS` 一次，默认 30 秒）。 |
| `dumbmonit_alerting_cycle_seconds` | gauge | 上一个告警周期的持续时间，含查询时间。接近间隔意味着规则的查询量超过了存储的承受能力。 |
| `dumbmonit_alerting_rules_evaluated` | gauge | 上一个周期评估的规则数。 |
| `dumbmonit_alerting_rules_failed` | gauge | 上一个周期中查询失败的规则数。它们的告警被冻结，而不是被解除。任何非 0 的值都值得查看日志。 |
| `dumbmonit_alerts{phase}` | gauge | 引擎跟踪的告警，按阶段：`ok`、`pending`（条件为真，`for` 尚未满足）、`firing`、`resolved`。 |
| `dumbmonit_alerts_suppressed` | gauge | 因父设备宕机而被隐藏的告警（有效阶段 `suppressed`）。 |
| `dumbmonit_alerts_silenced` | gauge | 被维护窗口覆盖的告警。 |
| `dumbmonit_alerts_learning` | gauge | 仍在学习、因而保持静默的基线告警。 |
| `dumbmonit_notifications_total{kind}` | counter | 已发送的通知，按渠道类型（`email`、`discord`、`ntfy`……）。 |
| `dumbmonit_notifications_failed_total{kind}` | counter | 被渠道拒绝的通知，按类型。它们会重新排队并重试；持续上升的值意味着 webhook 或 SMTP 账户出了问题。 |
| `dumbmonit_agents` | gauge | 已注册代理的机器数。 |
| `dumbmonit_agents_stale` | gauge | 五分钟内未推送任何内容的代理数。 |
| `dumbmonit_database_up` | gauge | SQLite 有响应时为 1——与 `/api/health` 的检查相同。 |
| `dumbmonit_database_bytes` | gauge | `dumbmonit.db` 及其预写日志的大小。不包含 `/data/vm`。 |
| `dumbmonit_victoriametrics_up` | gauge | VictoriaMetrics 有响应时为 1——与 `/api/health` 的检查相同。内置子进程重启期间为 0。 |
| `dumbmonit_victoriametrics_embedded` | gauge | 此服务器自行运行 VictoriaMetrics 时为 1，`DUMBMONIT_VM_URL` 指向外部实例时为 0。 |

本页上的指标都不带每设备标签：一个设备和一千个设备的文档大小相同，也不会向任何能抓取它的人泄露设备名称。

在 DumbMonit 被抓取之后，值得在你这边添加的两个告警：

```
# The monitoring stopped monitoring
up{job="dumbmonit"} == 0

# Measurements are piling up instead of being stored
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### 不使用令牌开放这些路由 {#opening-the-routes-without-a-token}

`DUMBMONIT_METRICS_PUBLIC=true` 会向任何能访问该端口的人提供 `/metrics`、`/federate` 和 `/prometheus/api/v1/…`，适用于自行过滤端口的用户——**该网络上的任何人随后都可以读取每个设备的每条测量数据以及实例的状态，因此只有在已有其他手段限制谁能访问该端口时才设置它。**
