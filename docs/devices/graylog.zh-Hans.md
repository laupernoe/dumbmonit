# Graylog {#graylog}

日志服务器以及其背后的搜索集群:Graylog 是否在处理消息、有多少
消息在其日志队列中等待、缓冲区有多满、流入和流出的情况、
Graylog 视角下 OpenSearch 或 Elasticsearch 的健康状况、启动失败
的输入,以及未能被索引的消息。

DumbMonit 使用一个持有内置**Reader**角色的专用用户读取 Graylog
的 REST API。它从不搜索消息,从不启动或停止任何输入,也从不
确认任何通知。

值得关注的故障是:**搜索集群跟不上处理速度**。Graylog 会继续
接收消息并写入其磁盘日志队列;网页界面能打开,搜索也能正常
工作,只是什么都搜不到最近的内容。日志队列会不断增长直至写满,
然后消息就会被丢弃。日志队列积压、搜索集群健康状况和索引失败
都会在此之前提前发出信号。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_graylog_` 为前缀。日志队列、缓冲区和
吞吐量指的都是**DumbMonit 所连接的那个节点**的数据:在 Graylog
集群中,请把每个节点都作为独立设备添加。

| 指标 | 含义 | 标签 |
|---|---|---|
| `processing` | 节点正在处理消息时为 1,处理被暂停时为 0 | |
| `running`, `lb_alive` | 节点的生命周期状态为 `running`;它向负载均衡器报告自己存活 | |
| `version_info` | 固定值 1 | `version` |
| `journal_uncommitted_entries` | 已写入日志队列但尚未处理的消息数 | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | 输入、处理和输出缓冲区的饱和程度 | `buffer` |
| `messages_in_total`, `messages_out_total` | 收到的消息数,以及写入搜索集群的消息数 | |
| `indexer_status` | 搜索集群健康状况:0 绿色,1 黄色,2 红色,3 Graylog 无法连接 | |
| `indexer_shards` | 按状态划分的分片数:`active`、`initializing`、`relocating`、`unassigned` | `state` |
| `inputs_running`, `inputs_failed` | 该节点上的输入数 | |
| `input_failed` | 每个启动失败的输入对应的值为 1 | `input`(其标题) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | 写入搜索集群失败的次数、处理或索引失败的消息数、时间戳无效的消息数 | |
| `notifications`, `notifications_urgent`, `notification` | Graylog 自身的通知(可选权限,见下文) | `type`, `severity` |
| `cluster_nodes` | Graylog 集群中的节点数 | |
| `scrape_errors` | 上次探测中失败的调用 | |

搜索集群是通过 Graylog 读取的,因此不需要单独添加 OpenSearch 或
Elasticsearch 设备来了解它是否为绿色状态;当 Graylog 完全无法
连接它时,`indexer_status` 为 3。

适用的[内置规则](../alerting/rules.md#log-and-metrics-servers):
Graylog 未处理消息、Graylog 搜索集群宕机、Graylog 搜索集群黄色
状态、Graylog 日志队列积压、Graylog 输入失败、Graylog 索引失败,
以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Graylog。
它会用一句话说明哪里出了问题(处理状态、搜索集群、日志队列积压、
按名称列出的失败输入、索引失败、紧急通知),然后是每秒收发的
消息数、日志队列积压和节点数,再然后是缓冲区、按状态划分的分片
以及 Graylog 自身的通知。

## 在 Graylog 中创建一个只读用户和令牌 {#create-a-read-only-user-and-token-in-graylog}

1. 在 Graylog 中:System → Users and Teams → Create user。按如下方式命名该用户,配置一个较长的随机密码,只分配 Reader 角色。Reader 能够读取节点状态、日志队列、缓冲区、吞吐量、输入和搜索集群健康状况,只要没有任何流(stream)与它共享,就完全不能读取任何消息。

    ```
    dumbmonit
    ```

2. 打开该用户的 Edit tokens 页面,创建一个以 DumbMonit 命名的令牌,并立即复制它:Graylog 只会显示一次。

3. 可选:Graylog 自身的通知(启动失败的输入、磁盘水位线、几乎写满的日志队列)需要额外一项权限 notifications:read,没有任何内置角色单独授予这项权限。使用一个有权限管理角色的账户,创建一个只持有该权限的角色,并把它分配给 dumbmonit 用户。没有它,通知会被跳过,其他内容不受影响。

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. 在 DumbMonit 中,输入 Graylog 节点的地址,例如 "graylog.lan"(端口 9000),或反向代理之后的 "https://graylog.lan",并粘贴该令牌。在 Graylog 集群中,请添加每一个节点:日志队列、缓冲区和吞吐量都是按节点统计的。

5. DumbMonit 只会读取。它从不搜索消息,从不启动或停止任何输入,也从不确认任何通知。

!!! warning "警告"
    不要重复使用你自己登录用的账户:泄露的令牌会携带其用户的
    全部权限,而上面的 Reader 角色什么都不能更改。Graylog 默认
    提供纯 HTTP 服务,除非你配置了 TLS,而令牌会随每个请求传输:
    在不受信任的网络中,请把 Graylog 放在 HTTPS 之后。

Reader 的各项权限分别覆盖:`system:read` 对应节点状态和集群
节点,`journal:read` 对应日志队列,`metrics:read` 和
`throughput:read` 对应吞吐量和缓冲区,`inputs:read` 对应输入,
`indexercluster:read` 对应搜索集群健康状况。Reader 角色还持有
`messages:read`,但它只允许读取明确与其共享的流中的消息:不要
共享任何流。索引失败的详细信息(`indices:failures`)不需要:
DumbMonit 从 Graylog 自身的指标中统计失败数。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 访问令牌(推荐) | 专用用户的令牌。Graylog 期望以 HTTP 基本认证方式发送,用户名为该令牌,密码为单词 `token`;DumbMonit 会自动完成这一步。 |
| 用户名/密码 | 该专用用户本身。 |

地址:主机名或 IP(`graylog.lan`)、`host:port`,或完整 URL
(`https://graylog.lan`);结尾的 `/api` 也会被接受。选项:协议
(默认 HTTP)、端口(9000)、证书校验和请求超时(10 秒)。
