# Redis 和 Valkey {#redis-and-valkey}

内存存储本身:内存相对于 `maxmemory` 的使用情况、被拒绝的连接、
复制链路以及每个副本的延迟、失败的快照和 append-only file 写入,
以及键空间的大小。

DumbMonit 会打开一个连接,发送 `AUTH`,然后发送 `INFO`,随后关闭
连接。它使用的账户只能运行 `INFO` 和 `PING`,不能做其他任何事:
它不能读取、写入,甚至不能列出任何一个键。支持 Redis 6 及更高
版本、Valkey,以及以相同方式应答 `INFO` 的分支版本(KeyDB、
Dragonfly)。

值得关注的故障往往是悄无声息的。到达 `maxmemory` 时,根据
`maxmemory-policy` 的设置,Redis 要么在不通知任何人的情况下驱逐
键,要么对每次写入都返回 `OOM command not allowed`;没有设置
`maxmemory` 时,它会一直增长直到被内核杀死。只要
`stop-writes-on-bgsave-error` 处于开启状态(默认如此),一次失败
的快照就会让 Redis 拒绝所有写入。与主节点的链路断开的副本仍会
继续应答读请求,只是数据是陈旧的。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_redis_` 为前缀。总量是计数器:图表和规则
会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `product`(`redis`、`valkey`)、`version`、`mode` |
| `role_info` | 固定值 1 | `role`:`master` 或 `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | Redis 自身统计的内存,以及系统统计的内存 | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` 及其使用比例;当 `maxmemory` 为 0(无限制)时不存在 | |
| `maxmemory_policy_info` | 固定值 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | 因达到 `maxclients` 而被拒绝的连接数 | |
| `evicted_keys_total`, `expired_keys_total` | 因达到 `maxmemory` 而被驱逐的键数,过期的键数 | |
| `keyspace_hits_total`, `keyspace_misses_total` | 命中的查找次数,以及未命中的查找次数 | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | 返回的错误应答数,以及被拒绝的登录数(密码被暴力尝试时会在这里显示) | |
| `keys`, `keys_expiring` | 所有数据库中的键数,以及设置了过期时间的键数;始终存在,即使为零也会出现 | |
| `db_keys` | 每个数据库的键数 | `db` |
| `rdb_last_save_ok` | 当上次快照(`BGSAVE`)失败时为 0 | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | append-only file;后两个状态只在它开启时存在 | |
| `loading` | Redis 启动时加载数据集期间为 1 | |
| `connected_replicas` | 连接到该主节点的副本数 | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | 从主节点视角看每个副本:是否在线、距上次确认经过的秒数、落后的字节数 | `replica`(`ip:port`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | 在副本上:其与主节点的链路状态 | |
| `uptime_seconds`, `cluster_enabled` | | |

适用的[内置规则](../alerting/rules.md#databases-message-broker-and-security-engine):
Redis 内存接近 maxmemory、Redis 拒绝连接、Redis 复制链路断开、
Redis 副本延迟、Redis 快照失败、Redis append-only file 写入失败,
以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Redis。
它会用一句话说明哪里出了问题(复制、内存相对于 `maxmemory` 以及
达到上限时适用的策略、持久化、被拒绝的连接),然后是键数、内存、
客户端数、每秒命令数、命中率、过去 24 小时内驱逐的键数和运行
时间,再然后是每个数据库的键数和每个副本的延迟。

## 创建一个只能运行 INFO 的 Redis 用户 {#create-a-redis-user-that-may-only-run-info}

1. 在 Redis 6 或更高版本,或 Valkey 上,创建一个只能运行 INFO 和 PING、不能访问任何键的用户。在 redis-cli 中,以一个有权限管理 ACL 的用户登录:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. 使其永久生效:如果服务器从 aclfile 加载用户,执行 ACL SAVE,否则执行 CONFIG REWRITE。

    ```
    ACL SAVE
    ```

3. 在 Redis 5 或更早版本上,或者只设置了 requirepass 的情况下,请在下方选择"仅密码"。该密码拥有全部权限:只要服务器支持,就优先使用 ACL 用户。

4. 在 DumbMonit 中,输入服务器地址,例如 "cache.lan" 或 "cache.lan:6380"。如果服务器在该端口上只接受 TLS(tls-port),请在下方勾选 TLS。

5. DumbMonit 会发送 AUTH,然后发送 INFO。它从不读取、写入或列出任何键。

!!! warning "警告"
    Redis 在达到 maxmemory 之前不会发出任何警告:到达上限时,
    根据 maxmemory-policy 的设置,它要么悄悄驱逐键,要么拒绝
    所有写入。没有设置 maxmemory 时,它会一直增长直到被内核
    杀死。DumbMonit 会在超过 maxmemory 的 90% 时发出警告。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| ACL 用户(推荐) | 上面创建的用户及其密码,以 `AUTH user password` 发送。 |
| 仅密码 | `requirepass` 的值,以 `AUTH password` 发送。 |
| 无认证 | 适用于完全没有密码的服务器。 |

地址:主机名或 IP(`cache.lan`)、`host:port`,或连接字符串
(`redis://cache.lan:6380`;端口之后的任何内容都会被忽略,写在
其中的凭据绝不会被使用)。选项:端口(6379)、TLS(默认关闭)、
证书校验,以及整次检查的超时时间(10 秒)。开启 TLS 时,证书会
对照 DumbMonit 内置的公共证书颁发机构进行校验;对于私有证书颁发
机构,请勾选"接受无法验证的证书"。

在 Redis Sentinel 或 Cluster 部署中,请添加你想监控的每一台
服务器:每台都会报告自己的内存、客户端和持久化情况,主节点还会
报告其副本。Sentinel 本身不受支持。
