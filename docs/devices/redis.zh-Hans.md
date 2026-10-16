# Redis 和 Valkey {#redis-and-valkey}

内存存储本身：内存对抗 `maxmemory`、拒绝的连接、复制链接和每个副本的延迟、失败的快照和仅追加文件写入以及键空间的大小。

DumbMonit 打开一个连接，发送 `AUTH`，然后 `INFO`，并关闭它。它使用的账户只能运行 `INFO` 和 `PING` 没别的：它不能读取、写入或甚至列出一个键。Redis 6 及更高版本、Valkey 和以相同方式答复 `INFO` 的分支（KeyDB、Dragonfly）被覆盖。

值得观看的失败是无声的。在 `maxmemory`，Redis 要么无声地驱逐键，要么以 `OOM command not allowed` 拒绝每个写入，取决于 `maxmemory-policy`；没有 `maxmemory`，它增长直到内核杀死它。失败的快照使 Redis 在 `stop-writes-on-bgsave-error` 打开时拒绝写入，它默认打开。副本其链接到其主副本关闭的保持答复读取，具有过时数据。

## 它监视什么 {#what-it-watches}

所有指标都以 `dumbmonit_redis_` 为前缀。总计是计数器：图表和规则将它们变成速率。

| 指标 | 什么 | 标签 |
|---|---|---|
| `version_info` | 值 1 | `product`（`redis`、`valkey`）、`version`、`mode` |
| `role_info` | 值 1 | `role`：`master` 或 `replica` |
| `memory_used_bytes`、`memory_rss_bytes`、`memory_peak_bytes`、`memory_fragmentation_ratio` | 内存如 Redis 计数它，以及系统如何做 | |
| `memory_max_bytes`、`memory_used_percent` | `maxmemory` 和已用份额；当 `maxmemory` 是 0（无限制）时缺失 | |
| `maxmemory_policy_info` | 值 1 | `policy` |
| `connected_clients`、`blocked_clients`、`max_clients` | | |
| `rejected_connections_total` | 因 `maxclients` 被达到而拒绝的连接 | |
| `evicted_keys_total`、`expired_keys_total` | 在 `maxmemory` 驱逐的键，过期的键 | |
| `keyspace_hits_total`、`keyspace_misses_total` | 找到其键的查找，那些没有的 | |
| `commands_processed_total`、`connections_received_total`、`ops_per_second` | | |
| `error_replies_total`、`acl_denied_auth_total` | 发送的错误回复，拒绝的登录（被猜测的密码显示这里） | |
| `keys`、`keys_expiring` | 所有数据库中的键，那些带过期的；总是存在，甚至在零 | |
| `db_keys` | 每个数据库的键 | `db` |
| `rdb_last_save_ok` | 0 当最后的快照（`BGSAVE`）失败时 | |
| `rdb_changes_since_last_save`、`rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`、`aof_last_write_ok`、`aof_last_rewrite_ok` | 仅追加文件；两个状态仅当它打开时 | |
| `loading` | 1 当 Redis 在启动加载其数据集时 | |
| `connected_replicas` | 附加到此主副本的副本 | |
| `replica_online`、`replica_lag_seconds`、`replica_lag_bytes` | 从主副本看，每个副本：在线或不，它上次确认后的秒数，字节落后 | `replica`（`ip:port`） |
| `master_link_up`、`master_last_io_seconds`、`master_link_down_seconds`、`master_sync_in_progress` | 在副本上：它的链接到主副本 | |
| `uptime_seconds`、`cluster_enabled` | | |

[内置规则](../alerting/rules.md#databases-message-broker-and-security-engine)应用：Redis 内存近 maxmemory、Redis 拒绝连接、Redis 复制链接关闭、Redis 副本延迟、Redis 快照失败、Redis 仅追加文件写入失败，加监控项无法访问。

## 监控项页面 {#the-device-page}

面板在图表上方读取探针存储的内容；打开页面从不查询 Redis。它在一个句子中说什么是错误（复制、内存对抗 `maxmemory` 和在限制应用的策略、持久性、拒绝的连接），然后是键、内存、客户、每秒命令、命中比、最后 24 小时驱逐的键和正常运行时间，然后是每个数据库的键和每个副本的延迟。

## 创建一个仅可运行 INFO 的 Redis 用户 {#create-a-redis-user-that-may-only-run-info}

1. 在 Redis 6 或更高版本或 Valkey 上，创建一个仅可运行 INFO 和 PING 且没别的的用户，无权访问任何键。在 redis-cli 中，登录具有允许管理 ACL 的用户：

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. 使它永久：ACL SAVE 如果服务器从 aclfile 加载其用户，CONFIG REWRITE 否则。

    ```
    ACL SAVE
    ```

3. 在 Redis 5 或更旧的，或仅带 requirepass 设置，选择下面的"仅密码"。那个密码携带每个权限：尽可能推荐 ACL 用户。

4. 在 DumbMonit 中，输入服务器地址，例如"cache.lan"或"cache.lan:6380"。如果服务器仅在那个端口（tls-port）上接受 TLS，勾选下面的 TLS。

5. DumbMonit 发送 AUTH，然后 INFO。它从不读取、写入或列出键。

!!! warning
    Redis 在 maxmemory 前给出无警告：在限制它要么无声地驱逐键，要么拒绝每个写入，取决于 maxmemory-policy。没有 maxmemory 它增长直到内核杀死它。DumbMonit 在 maxmemory 的 90% 以上警告。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| ACL 用户（推荐） | 上面创建的用户及其密码，作为 `AUTH user password` 发送。 |
| 仅密码 | `requirepass` 值，作为 `AUTH password` 发送。 |
| 无认证 | 对于根本没有密码的服务器。 |

地址：主机名或 IP（`cache.lan`）、`host:port`，或连接字符串（`redis://cache.lan:6380`；端口后的任何东西被忽略，凭据写在其中从不使用）。选项：端口（6379）、TLS（关闭）、证书检查以及整个检查的超时（10 s）。启用 TLS，证书被检查对抗 DumbMonit 内置的公共当局；对于私有当局，勾选接受不可验证的证书。

拒绝的密码（`WRONGPASS`、`NOAUTH`、`NOPERM`）被显示为凭据错误，没有人被通知关于错误本身，但 DumbMonit 然后收集零：监控项无法访问规则保持触发，如对任何监控项它不能读取，直到凭据再次工作。Redis 重启时没有其 ACL 用户（用 `ACL SETUSER` 定义但从不保存的用户）导致正好这个。

在 Redis Sentinel 或 Cluster 部署中，添加您想要看到的每个服务器：每个报告其自己的内存、客户和持久性，主副本报告它的副本。Sentinel 本身未被覆盖。
