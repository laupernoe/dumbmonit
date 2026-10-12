# Nextcloud

Nextcloud 是在正常提供服务还是卡住了:维护模式被遗留开启、更新后
等待的数据库升级、Nextcloud 和应用的更新、活跃用户数、数据目录的
剩余空间、数据库大小,以及 PHP 的 OPcache。

DumbMonit 使用一个**监控令牌**(`NC-Token`)读取公开的
`status.php` 以及 Nextcloud 自带的 **serverinfo** 应用的统计
数据。该令牌只能打开这些统计数据,不能做其他任何事:它不能
登录、列出用户、打开文件或更改任何设置。

值得关注的故障:**一次没有完成的更新**。更新之后 Nextcloud 可能
停留在维护模式中,或等待 `occ upgrade`;网页此时会显示一条没人
会看到的提示,直到有人尝试同步时才会发现。这两种情况都是从
`status.php` 读取的,即使此时它仍会应答。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_nextcloud_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `maintenance` | 维护模式开启期间为 1 | |
| `needs_db_upgrade` | Nextcloud 等待 `occ upgrade` 期间为 1 | |
| `version_info` | 固定值 1 | `version` |
| `update_available` | 当 Nextcloud 自身的更新检查发现新版本时为 1 | `available_version` |
| `apps_installed`, `app_updates_available` | 已安装的应用数,以及应用商店中有新版本的应用数 | |
| `app_update_available` | 每个有更新的应用对应的值为 1(最多 32 个) | `app`, `available_version` |
| `active_users` | 最近 5 分钟、1 小时和 1 天内出现的用户数 | `window`(`5m`、`1h`、`24h`) |
| `users`, `files`, `shares` | 实例上的账户、文件和共享数 | |
| `free_space_bytes` | Nextcloud 存放数据位置的剩余空间 | |
| `database_size_bytes` | 数据库大小 | |
| `platform_info` | 固定值 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | PHP 的 `memory_limit` | |
| `opcache_enabled`, `opcache_full` | OPcache 是否开启;OPcache 是否空间不足 | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | OPcache 的使用率以及服务效果 | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | 当 Nextcloud 运行在 PHP-FPM 之后时(`-fpm` 镜像、AIO)的 PHP-FPM 进程池情况 | |

当 Nextcloud 处于维护模式或等待升级时,其 API 会应答 503:此时
只会写入 `maintenance`、`needs_db_upgrade` 和 `version_info`,
且该设备不会被报告为不可达。

适用的[内置规则](../alerting/rules.md#self-hosted-applications):
Nextcloud 卡在维护模式、Nextcloud 数据库升级待处理、Nextcloud
有可用更新、Nextcloud OPcache 已满、Nextcloud 剩余空间不足,以及
设备不可达。

## 后台任务(cron) {#background-jobs-cron}

Nextcloud 不会告诉监控令牌后台任务上次运行的时间:只有拥有实例
全部权限的账户才能读取这一信息,而 DumbMonit 不会去申请这样的
账户。请改用[心跳](push.md)来监控 cron:在 DumbMonit 中创建一个
心跳,然后在运行 Nextcloud 后台任务的 cron 命令行末尾调用它的
URL,这样它只会在 `cron.php` 成功时才被调用:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

以 5 分钟的预期间隔,无论原因是什么(容器重启后丢失了 cron、
PHP 错误、后台任务被留在了 AJAX 方式上),该心跳会在 cron 停止
后立即告警。

## 客户端设备 {#client-devices}

Nextcloud 没有通过任何 API 公开一份管理员可见的同步客户端或移动
设备列表。个人的"设备与会话"页面(设置 → 安全)只列出当前登录的
那个账户,依赖的是浏览器会话而不是 API 令牌,而唯一的管理员级
等价方式是命令行(`occ user:auth-tokens:list <user>`,一次只能
查一个账户)——这不是监控令牌能触及的内容,DumbMonit 也不会要求
管理员把它脚本化接入某次探测。DumbMonit 读取的 `serverinfo` API
也没有设备列表:它只给出活跃用户数(上面的 `active_users`),
从不说明具体是谁或是什么设备。

这是一个有意为之的限制,不是缺失的功能:要给 DumbMonit 一份设备
列表,就意味着要把监控令牌扩大为一个完整的管理账户,而本页其余
内容正是在努力避免这一点。如果 Nextcloud 未来通过 OCS 公开这项
信息,DumbMonit 会遵循已经用于 Immich、Proxmox Backup Server 和
Veeam 的同一套[客户端设备](../alerting/rules.md)模式。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会调用
Nextcloud。它会先说明哪里出了问题(维护模式、升级等待中、
OPcache 已满或关闭),然后是更新情况,再然后是活跃用户数、剩余
空间、数据库大小、用户数、文件数和 OPcache 数据,最后是按名称
列出需要更新的应用以及平台信息(数据库、PHP)。

## 给 Nextcloud 配置一个监控令牌 {#give-nextcloud-a-monitoring-token}

1. Nextcloud 随附并默认启用的 serverinfo 应用会响应一个只能打开其统计数据、不能做其他任何事的监控令牌:没有文件、没有用户、没有共享。在 Nextcloud 服务器上,设置一个较长的随机令牌并把它打印出来。以网页服务器用户身份运行 occ;在 Docker 中,请在前面加上 docker exec -u www-data 和容器名。

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. 如果 occ 提示 serverinfo 已禁用,请先启用它。

    ```
    php occ app:enable serverinfo
    ```

3. 在 DumbMonit 中,输入你用来打开 Nextcloud 的地址,例如 "cloud.example.com"(443 端口的 HTTPS)或 "http://10.0.0.5:8080",并粘贴该令牌。该主机必须是 Nextcloud 受信任域名之一,否则 Nextcloud 会拒绝该请求。

4. DumbMonit 只会读取 status.php 和 serverinfo 的统计数据。它从不打开任何文件,从不列出用户,也从不更改任何设置,该令牌也无法在任何地方登录。

!!! warning "警告"
    Nextcloud 不会告诉该令牌后台任务(cron)上次运行的时间:
    只有拥有实例全部权限的账户才能读取这一信息。若要在 cron
    停止时被告知,请在 DumbMonit 中添加一个心跳,并像文档所示
    那样在 cron 命令行末尾调用它的 URL。

使用 Nextcloud AIO 时,请在 `nextcloud-aio-nextcloud` 容器中运行
occ:`docker exec -u www-data nextcloud-aio-nextcloud php occ …`。
为什么不用委派管理组:Nextcloud 只向该令牌或完整 `admin` 组的
成员开放 serverinfo API,委派权限不会改变这一点。

从 DumbMonit 所在主机手动检查该令牌:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 监控令牌(NC-Token) | serverinfo 令牌。DumbMonit 会在 `NC-Token` 请求头中发送它,并带上 `OCS-APIRequest: true`。 |

地址:主机名(`cloud.example.com`)、`host:port`,或带路径前缀的
完整 URL(`https://example.com/nextcloud`);结尾的 `/index.php`
或 `/login` 也会被接受。选项:协议(默认 HTTPS)、端口(443)、
证书校验和请求超时(15 秒)。

应用更新计数会查询 Nextcloud 的应用商店缓存;在无法访问应用商店
的实例上,该计数会保持为 0。

已在 Nextcloud 35.0.1(官方镜像)上验证。
