# RabbitMQ

消息中间件:内存和磁盘告警、停止运行的节点和网络分区、无人消费
而不断堆积的队列、不再运行的队列,以及消息的流动情况。

DumbMonit 使用一个标记为 `monitoring` 且权限为空的用户读取
管理 API:它能看到节点、告警和队列的状态,不能发布、消费、读取
消息,也不能更改任何内容。一个节点就能报告整个集群的情况。

值得关注的故障是客户端不会报告的那种。当某个节点超过其内存高位
水位线或剩余磁盘限制时,RabbitMQ 会发出告警,并阻塞集群中的
每一个发布者:应用程序只会在 `publish` 上挂起,没有任何错误。
第二种故障是某个队列因为消费它的服务已停止而持续堆积。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_rabbitmq_` 为前缀。总量是计数器:图表和
规则会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | 集群总数 | |
| `messages_ready`, `messages_unacked` | 所有队列中等待的消息数,以及已投递但尚未确认的消息数 | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | 消息流动情况;unroutable 是没有任何队列绑定来接收的消息 | |
| `alarm` | 告警触发期间为 1;内存和磁盘两种告警始终存在,无告警时为 0 | `node`, `resource` |
| `alarms` | 集群中触发的告警数 | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | 按节点 | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | 内存相对于高位水位线 | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | 剩余磁盘相对于限制 | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | 按队列,最满的优先,最多到"逐一监控的队列数"上限 | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | 这些队列中的子集 | |

告警是从健康检查接口 `/api/health/checks/alarms` 读取的,能立即
看到;节点统计数据会滞后几秒钟。

适用的[内置规则](../alerting/rules.md#databases-message-broker-and-security-engine):
RabbitMQ 内存或磁盘告警、RabbitMQ 节点宕机、RabbitMQ 网络分区、
RabbitMQ 队列无消费者、RabbitMQ 队列不可用,以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询
RabbitMQ。它会用一句话说明哪里出了问题(告警、节点和分区、
持有消息却无消费者的队列、未运行的队列),然后是待处理和未确认
的消息数、每秒发布和投递的消息数、连接数、消费者数和队列数,
再然后是最满的十个队列及其消费者情况,以及每个节点的内存和
剩余磁盘。

## 创建一个标记为 monitoring 的 RabbitMQ 用户 {#create-a-rabbitmq-user-tagged-monitoring}

1. 如果管理插件尚未启用,请启用它(管理版的 Docker 镜像自带该插件)。其 API 监听在 15672 端口上。

    ```
    rabbitmq-plugins enable rabbitmq_management
    ```

2. 创建一个只带有 monitoring 标记的用户:它可以读取节点、告警和队列状态,不能更改任何内容。

    ```
    rabbitmqctl add_user dumbmonit 'a-long-password'
    rabbitmqctl set_user_tags dumbmonit monitoring
    ```

3. 在你想要查看队列的每个虚拟主机上授予它空权限。这样它就能列出这些队列,但仍然不能发布、消费或读取消息。对其他每个虚拟主机使用 -p 重复这个操作。

    ```
    rabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'
    ```

4. 在 DumbMonit 中,输入某个节点的地址,例如 "rabbit.lan",或当 API 通过 TLS 提供服务时输入 "https://rabbit.lan:15671"。一个节点就能报告整个集群的情况:每个节点的告警和分区,以及每一个队列。

!!! warning "警告"
    内存或磁盘告警会阻塞集群中的每一个发布者,客户端不会收到任何错误:应用程序只会在发布操作上挂起。队列按最满优先列出,最多到"逐一监控的队列数"(默认 100);总数始终涵盖所有队列。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名/密码 | 上面创建的 `monitoring` 用户,以 HTTP 基本认证方式发送。 |

在某个虚拟主机上没有权限时,其队列不会被逐一列出;集群总数仍会
包含它们。

地址:主机名或 IP(`rabbit.lan`)、`host:port`,或反向代理之后
带路径前缀的完整 URL。选项:协议(默认 HTTP)、端口(15672)、
证书校验、请求超时(10 秒),以及逐一监控的队列数(100,可选
0 到 500)。
