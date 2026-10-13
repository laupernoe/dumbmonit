# MongoDB {#mongodb}

文档数据库本身：副本集（成员、状态、主节点丢失、每个副本节点的延迟），连接限制，WiredTiger 缓存，操作和断言。

DumbMonit 使用只具有内置 `clusterMonitor` 角色的账户登录，运行 `serverStatus` 和 `replSetGetStatus`（当服务器属于副本集时）。该角色读取服务器和复制的状态，但不读取任何数据库的文档。支持 MongoDB 3.6 及更高版本；DumbMonit 直接使用消息协议，无需驱动程序。

值得关注的故障：失去主节点的副本集在选举成功前不接受任何写操作；落后的副本节点是当主节点故障时会被选举的节点，但缺少最新写操作；在连接限制处新客户端被拒绝；当 WiredTiger 缓存脏数据超过 20% 时，查询必须自己驱逐页面，导致一切变慢。

## 它监控什么 {#what-it-watches}

所有指标前缀为 `dumbmonit_mongodb_`。总数是计数器：图表和规则将其转换为速率。

| 指标 | 说明 | 标签 |
|---|---|---|
| `version_info` | 值为 1 | `version`, `process`, `storage_engine` |
| `role_info` | 值为 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | 当前连接相对于当前加可用连接的百分比 | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | 自启动以来的操作 | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | 自启动以来的断言；`type` 为 `regular`（内部服务器错误），`warning`，`msg`，`user`（返回给客户端的错误），`tripwire` 或 `rollovers`。`regular` 和 `user` 值得图表显示 | `type` |
| `queued_operations`, `active_clients` | 等待锁的操作，正在运行操作的客户端 | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | WiredTiger 缓存及其饱和度 | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | 未写入的修改页面 | |
| `wiredtiger_cache_read_bytes_total` | 从磁盘读入缓存的字节数 | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | 值为 1，在副本集中 | `set`, `members` |
| `replset_primary_present` | 当有成员是主节点时为 1，无主节点时为 0 | |
| `replset_member_state` | 成员的状态号 | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 成员可达时为 1，不可达时为 0 | `member` |
| `replset_member_lag_seconds` | 副本节点落后于主节点的时间 | `member` |

适用的[内置规则](../alerting/rules.md#databases-message-broker-and-security-engine)：MongoDB 副本集无主节点、MongoDB 成员不可达、MongoDB 复制延迟、MongoDB 连接接近限制、MongoDB 缓存压力大、MongoDB 内部错误，加上设备不可达。

## 设备页面 {#the-device-page}

面板上方读取探针存储的内容；打开页面从不查询 MongoDB。它用一句话说明问题所在（副本集、连接、缓存、内部错误），然后是连接数、每秒操作数、缓存使用情况、驻留内存、排队操作和运行时间，然后是每个副本集成员的状态和延迟以及按类型的每秒操作数。

## 为 clusterMonitor 角色创建 MongoDB 账户 {#create-a-mongodb-account-with-the-clustermonitor-role}

1. 在 mongosh 中，使用被允许创建用户的用户登录，创建仅具有内置 clusterMonitor 角色的账户。它读取服务器和复制状态，不读取任何数据库的文档。

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit 使用 SCRAM-SHA-256 登录，这是在 MongoDB 4.0 及更高版本上创建的账户的默认值。如果在 admin 之外的数据库中创建了账户，请在下方设置身份验证数据库。

3. 在 DumbMonit 中，输入一个 mongod 的地址，例如 "db1.lan" 或 "db1.lan:27018"。在副本集中，添加每个成员：每个报告自己的连接和缓存，任何一个都报告所有成员的状态和延迟。如果服务器需要 TLS，请勾选下方的 TLS。

4. DumbMonit 仅运行 serverStatus 和 replSetGetStatus。它从不列出集合，从不读取文档。

!!! warning
    mongodb+srv:// 地址不被解析：改为输入每个成员的主机。不支持比 3.6 更旧的 MongoDB 和 mongos 路由器。

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| 用户名 / 密码 | 上面创建的 `clusterMonitor` 账户，使用 SCRAM-SHA-256 登录。 |
| 无身份验证 | 对于启动时不使用访问控制的服务器。 |

仅具有 SCRAM-SHA-1 凭证的账户（在 MongoDB 4.0 之前创建且从未更新）被拒绝：使用 `db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})` 更新其密码一次。

地址：主机名或 IP（`db1.lan`），`host:port`，或具有单个主机的 `mongodb://` 连接字符串。选项：端口 (27017)、身份验证数据库（`admin`）、TLS（关闭）、证书检查和整个检查的超时（10 秒）。
