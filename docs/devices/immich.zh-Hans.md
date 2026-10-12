# Immich

Immich 是否能跟上上传的速度:停滞或处于暂停状态的任务队列、更新、
存放媒体库的磁盘、它持有的照片和视频数量,以及——对于登录该账户
的每一台手机、平板和桌面设备——它们的上次连接和上次备份时间。

DumbMonit 使用一个**仅限八项读取权限的 API 密钥**读取 Immich 的
API。该密钥无法看到任何照片、相册或人物,也无法更改任何内容。

值得关注的故障:**任务不再运行**。在一次大型导入过程中队列被
暂停后被遗忘,或某个任务工作进程停止了,新照片就得不到缩略图、
元数据、人脸识别和智能搜索,而上传本身仍会继续成功。Immich 只会
在其 Jobs 页面上显示这一点。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_immich_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `update_available` | 当 Immich 自身的版本检查发现新版本时为 1(检查关闭时不存在) | `latest_version` |
| `maintenance_mode` | Immich 处于维护模式期间为 1 | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | 存放媒体库的磁盘 | |
| `photos`, `videos`, `usage_bytes`, `users` | 媒体库大小,所有用户合计 | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | 所有队列合计 | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | 按队列:等待中(包括延迟和被暂停阻塞的)、运行中、失败、已暂停 | `queue` |
| `statistics_readable` | 当该密钥属于普通账户、计数和队列被跳过时为 0 | |
| `scrape_errors` | 上次探测中失败的调用 | |

较新版本的 Immich 会在失败任务被记录后把它从队列中移除,因此
`queue_failed` 通常为 0;失败的任务会显示在 Immich 自身的日志中。

适用的[内置规则](../alerting/rules.md#self-hosted-applications):
Immich 任务停滞、Immich 队列已暂停、Immich 存储空间即将用尽、
Immich 有可用更新,以及设备不可达。

## 设备 {#devices}

登录到拥有该密钥的账户的每一台设备,连同其上次连接和上次备份
时间,都会以每个追踪客户端设备的 DumbMonit 集成共享的通用指标族
发布——`dumbmonit_client_device_*`,前缀中不含 `immich`,完整文档见
[客户端设备](../alerting/rules.md#client-devices):

| 指标 | 含义 | 标签 |
|---|---|---|
| `last_seen_timestamp_seconds` | 上次连接(该会话的最后活动) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | 上次备份;见下方的限制说明 | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | 当某设备超过 `device_stale_days`(默认 3 天)未连接或未备份时为正值;驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`, `type`, `os`, `user`, `kind`, `signal`(`connection` 或 `backup`) |

**值得了解的限制**:Immich 的 API 不会记录某张照片是由哪台设备
上传的,因此 `last_backup_timestamp_seconds` 实际上是整个媒体库
最近一次上传的时间——对每台设备显示的都是同一个值,并以一个名为
`library` 而非真实设备名的设备发布。它能捕捉到所有设备同时停止
的情况(存储已满、账户在所有地方都被登出);但不能捕捉到单台
设备停止上传、而其他设备仍在继续上传的情况。只有
`last_seen_timestamp_seconds`——每台设备会话上次活动的时间——才
是真正按设备区分的。

设备页面的表格会为每台设备显示这些数值,状态为"正常"或"陈旧"。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会调用 Immich。
它会先说明哪里出了问题(队列卡住、队列已暂停、维护模式),然后
是更新情况,再然后是照片数、视频数、媒体库大小、磁盘使用情况、
等待和运行中的任务数,接着是按名称列出的有积压工作的队列,最后
是设备表格(名称、类型、操作系统、上次连接、上次备份、状态)。

## 在 Immich 中创建一个只读 API 密钥 {#create-a-read-only-api-key-in-immich}

1. 使用一个能打开 Administration 的账户登录 Immich:Immich 只允许这些账户读取照片计数和任务队列。打开 Account Settings → API Keys → New API Key,按如下方式命名,并勾选以下八项权限:前五项用于任务队列、计数和版本,后三项用于列出该账户的设备及其上次连接和备份。

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read session.read asset.read user.read
    ```

2. 现在就复制该密钥:Immich 只会显示一次。它只读取这八个接口,不会读取任何其他内容:没有照片、没有相册、没有设置。如果你的 Immich 没有提供 queue.read,请改为勾选 job.read。

3. 由普通账户创建的密钥同样可用:版本、更新、磁盘空间和设备会被读取,照片计数和任务队列会被跳过。

4. 在 DumbMonit 中,输入 Immich 的地址,例如 "immich.lan"(端口 2283),或反向代理之后的 "https://photos.example.com",并粘贴该密钥。

!!! warning "警告"
    一个被暂停的队列很容易被遗忘:新照片得不到缩略图、元数据和
    人脸识别,而 Immich 只会在其 Jobs 页面上说明这一点。当某个
    未暂停的队列中有任务在等待却没有任何任务运行时,DumbMonit
    会发出警告。session.read、asset.read 和 user.read 会列出拥有
    该密钥账户的每一台设备——它的手机、平板和浏览器——以及每台
    设备的上次连接时间。Immich 的 API 不会记录某张照片是由哪台
    设备上传的,因此"上次备份"实际上是整个媒体库最近一次上传
    的时间,对每台设备显示的都是同一个值:如果只有一台设备停止
    备份而其他设备仍在继续,它的连接会显示为陈旧,但共享的备份
    数值仍会看起来是新的。

每项权限分别打开什么:`server.about` 对应版本
(`/api/server/about`),`server.versionCheck` 对应 Immich 已知的
最新发行版,`server.storage` 对应磁盘,`server.statistics` 对应
照片和视频计数,`queue.read` 对应任务队列(`/api/queues`;较旧
版本在 `/api/jobs` 上应答,需要 `job.read`),`session.read` 对应
该账户的设备(`/api/sessions`),`asset.read` 对应媒体库最近一次
上传(`/api/search/metadata`),`user.read` 对应该账户的显示名称
(`/api/users/me`)。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| API 密钥 | 在 `x-api-key` 请求头中发送。 |

地址:主机名(`immich.lan`)、`host:port`,或带路径前缀的完整
URL;结尾的 `/api` 也会被接受。选项:协议(默认 HTTP)、端口
(2283)、证书校验、请求超时(15 秒),以及设备陈旧判定
(`device_stale_days`,默认 3 天)。

已在 Immich v3.2.4(官方镜像,关闭机器学习)上验证;设备相关的
接口是依据 Immich 自己 OpenAPI 规范(`immich-app/immich`)的
`v3.2.4` 标签验证的,该规范确认 `/api/search/metadata` 不再接受
`deviceId` 过滤器——这正是"上次备份"是整个媒体库范围而不是按
设备区分的原因。
