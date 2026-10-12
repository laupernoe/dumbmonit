# MongoDB

文档数据库本身:副本集(成员、其状态、缺失的主节点、每个从节点
落后多少)、相对于上限的连接数、WiredTiger 缓存、操作和断言。

DumbMonit 使用一个只持有内置 `clusterMonitor` 角色的账户登录,
运行 `serverStatus`,以及当服务器属于某个副本集时运行
`replSetGetStatus`。该角色只能读取服务器和复制的状态,不能读取
任何数据库中的任何文档。支持 MongoDB 3.6 及更高版本;DumbMonit
自行实现了其网络协议,不依赖任何驱动。

值得关注的故障:失去主节点的副本集在选举成功之前不会接受任何
写入;落后的从节点正是主节点挂掉那天会被选为主节点、却没有最新
写入数据的那个节点;达到连接上限时新客户端会被拒绝;当 WiredTiger
缓存中超过 20% 的页面是脏页时,查询必须自行换出页面,导致一切
变慢。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_mongodb_` 为前缀。总量是计数器:图表和
规则会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version`, `process`, `storage_engine` |
| `role_info` | 固定值 1 | `role`:`primary`、`secondary`、`arbiter`、`standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | 当前连接数相对于当前加可用连接数的占比 | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | 启动以来的操作数 | `op`:`insert`、`query`、`update`、`delete`、`getmore`、`command` |
| `asserts_total` | 启动以来的断言数;`regular` 是服务器内部错误,`user` 是返回给客户端的错误 | `type` |
| `queued_operations`, `active_clients` | 等待锁的操作数,正在运行操作的客户端数 | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | WiredTiger 缓存及其使用率 | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | 已修改但尚未写入的页面 | |
| `wiredtiger_cache_read_bytes_total` | 从磁盘读入缓存的字节数 | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | 固定值 1,表示属于某个副本集 | `set`, `members` |
| `replset_primary_present` | 当有成员是主节点时为 1,没有时为 0 | |
| `replset_member_state` | 该成员的状态编号 | `member`, `state`:`primary`、`secondary`、`arbiter`、`recovering`、`down`…… |
| `replset_member_health` | 当该成员可达时为 1,不可达时为 0 | `member` |
| `replset_member_lag_seconds` | 从节点落后主节点的程度 | `member` |

适用的[内置规则](../alerting/rules.md#databases-message-broker-and-security-engine):
MongoDB 副本集没有主节点、MongoDB 成员不可达、MongoDB 复制延迟、
MongoDB 连接数接近上限、MongoDB 缓存压力过大、MongoDB 内部错误,
以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 MongoDB。
它会用一句话说明哪里出了问题(副本集、连接数、缓存、内部错误),
然后是连接数、每秒操作数、缓存使用率、常驻内存、排队中的操作数
和运行时间,再然后是每个副本集成员的状态和延迟,以及按类型划分
的每秒操作数。

## 创建一个拥有 clusterMonitor 角色的 MongoDB 账户 {#create-a-mongodb-account-with-the-clustermonitor-role}

1. 在 mongosh 中,以一个有权限创建用户的账户登录,创建一个只拥有内置 clusterMonitor 角色的账户。该角色读取服务器和复制状态,不读取任何数据库中的任何文档。

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit 使用 SCRAM-SHA-256 登录,这是 MongoDB 4.0 及更高版本创建账户时的默认机制。如果你在 admin 之外的其他数据库中创建了该账户,请在下方设置认证数据库。

3. 在 DumbMonit 中,输入某个 mongod 的地址,例如 "db1.lan" 或 "db1.lan:27018"。对于副本集,请添加每一个成员:每个成员都会报告自己的连接数和缓存情况,其中任何一个都能报告所有成员的状态和延迟。如果服务器要求使用 TLS,请在下方勾选 TLS。

4. DumbMonit 只运行 serverStatus 和 replSetGetStatus。它从不列出任何集合,也从不读取任何文档。

!!! warning "警告"
    `mongodb+srv://` 地址不会被解析:请改为逐一输入每个成员的主机地址。不支持低于 3.6 版本的 MongoDB 以及 mongos 路由器。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名/密码 | 上面创建的 `clusterMonitor` 账户,以 SCRAM-SHA-256 方式登录。 |
| 无认证 | 适用于未启用访问控制启动的服务器。 |

只持有 SCRAM-SHA-1 凭据的账户(在 MongoDB 4.0 之前创建且从未更新过)
会被拒绝:请用
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`
更新一次其密码。

地址:主机名或 IP(`db1.lan`)、`host:port`,或只含单个主机的
`mongodb://` 连接字符串。选项:端口(27017)、认证数据库(`admin`)、
TLS(默认关闭)、证书校验,以及整次检查的超时时间(10 秒)。
