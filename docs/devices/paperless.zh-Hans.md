# Paperless-ngx {#paperless-ngx}

Paperless-ngx 是否还在导入发送给它的内容:数据库、Redis、Celery
工作进程、搜索索引、分类器和健全性检查的健康状况,失败的任务、
更新、磁盘空间以及文档数量。

DumbMonit 使用一个**无法读取任何文档的专用用户**读取 Paperless-ngx
的 REST API:三个查看权限让它能获取任务列表、系统状态和全局计数,
仅此而已。

值得关注的故障是:**Redis 或 Celery 停止运行**。网页界面仍能打开,
文档仍能被读取,但在有人注意到堆积之前,从导入文件夹或邮件中
获取的任何内容都不会被导入。系统状态会说明这一点;DumbMonit 会
为此告警。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_paperless_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `update_available` | 当 Paperless-ngx 自身的版本检查发现新版本时为 1(检查关闭时不存在) | `latest_version` |
| `component_status` | 0 表示正常,1 表示警告,2 表示错误,与系统状态页面一致 | `component`:`database`、`redis`、`celery`、`index`、`classifier`、`sanity_check`、`llm_index`(仅启用时) |
| `unapplied_migrations` | 尚未应用的数据库迁移 | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | 存放媒体文件的磁盘 | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | 过去 30 天内的所有任务:失败的、总数,以及仍待处理的 | |
| `tasks_failed_recent` | 该用户可见、比失败任务窗口更新且未被忽略的失败任务 | |
| `task_failed` | 每个此类任务对应的值为 1(最多 32 个) | `task`, `file` |
| `documents`, `documents_inbox` | Paperless 中的文档数,以及收件箱中的文档数 | |
| `status_readable` | 当该账户缺少系统监控权限时为 0 | |
| `scrape_errors` | 上次探测中失败的调用 | |

Paperless 会将每个所有者的任务保密:专用用户能按名称看到计划任务
(健全性检查、索引、分类器、邮件),而其他用户的导入操作会被计入
涵盖所有任务的 `tasks_failed_window`。

适用的[内置规则](../alerting/rules.md#self-hosted-applications):
Paperless 任务队列宕机、Paperless 索引或健全性检查错误、Paperless
迁移待处理、Paperless 任务失败、Paperless 存储空间即将用尽、
Paperless 有可用更新,以及监控项不可达。

## 监控项页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会调用 Paperless。
它会先说明出了什么问题(某个组件出错、任务失败、迁移待处理),
然后是更新情况,接着是文档数、收件箱、待处理与失败的任务以及
磁盘空间,再然后是按名称列出的各个组件和失败任务。

## 在 Paperless-ngx 中创建一个只读用户 {#create-a-read-only-user-in-paperless-ngx}

1. 创建一个按如下方式命名的用户,配置一个较长的随机密码,不授予超级用户或职员身份,只授予三个查看权限:任务、系统监控和全局统计。使用 API 和你自己的账户,下面两条命令可以创建它并打印出它的令牌。

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. 或者通过网页界面:Users & Groups → Add user,然后在其权限中只勾选 PaperlessTask、SystemMonitoring 和 GlobalStatistics 这三行的 View。以该用户身份登录一次,打开 My Profile 并生成它的 API 令牌。

3. 该用户完全不读取任何文档:全局统计给出计数,系统监控给出数据库、Redis、Celery 和索引的健康状况,任务则显示失败的任务。在没有后两个权限的 Paperless-ngx 2.x 上,健康状况和计数会被跳过,其余内容照常读取。

4. 在 DumbMonit 中,输入地址,例如 "paperless.lan"(端口 8000),或反向代理之后的 "https://paperless.example.com",并粘贴该令牌。

!!! warning "警告"
    当 Redis 或 Celery 停止运行时,Paperless 仍会提供页面和文档服务,但无论是来自导入文件夹还是邮件,都不会有任何新内容被导入,直到有人注意到。DumbMonit 会针对这两者都发出告警。

在运行上述命令前,请把 `CHANGE-ME` 替换为一个较长的随机密码,
把 `YOUR_USER` 替换为一个有权限管理用户的账户(curl 会要求输入
其密码)。当 Redis 处于宕机状态时,查询系统状态可能需要几秒钟:
Paperless 会等待它之后才应答。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| API 令牌(推荐) | 专用用户的令牌,以 `Authorization: Token …` 发送。 |
| 用户名/密码 | 专用用户本身,以 HTTP 基本认证方式发送。 |

地址:主机名(`paperless.lan`)、`host:port`,或带路径前缀的完整 URL;
结尾的 `/api` 也会被接受。选项:协议(默认 HTTP)、端口(8000)、
证书校验、请求超时(15 秒)以及失败任务窗口(24 小时,可选 1 到 720)。

已在 Paperless-ngx 3.2.1(官方镜像,SQLite,Valkey)上验证。
