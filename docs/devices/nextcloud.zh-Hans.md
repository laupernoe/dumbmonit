# Nextcloud {#nextcloud}

Nextcloud 是否在提供服务或卡住：维护模式是否打开，更新后是否等待数据库升级，Nextcloud 和应用更新，活跃用户，数据目录的可用空间，数据库大小和 PHP 的 OPcache。

DumbMonit 使用**监控令牌**（`NC-Token`）读取公共 `status.php` 和**serverinfo** 应用的统计数据，该应用随 Nextcloud 一起提供。令牌仅打开这些统计数据，别的都不行：它无法登录、列出用户、打开文件或更改设置。

值得关注的故障：**未完成的更新**。更新后 Nextcloud 可能保持维护模式，或等待 `occ upgrade`；网页然后显示一个没人看到的通知，直到有人尝试同步。两者都从 `status.php` 读取，即使在这种情况下也会回答。

## 它监控什么 {#what-it-watches}

所有指标前缀为 `dumbmonit_nextcloud_`。

| 指标 | 说明 | 标签 |
|---|---|---|
| `maintenance` | 维护模式打开时为 1 | |
| `needs_db_upgrade` | Nextcloud 等待 `occ upgrade` 时为 1 | |
| `version_info` | 值为 1 | `version` |
| `update_available` | Nextcloud 自有更新检查发现更新版本时为 1 | `available_version` |
| `apps_installed`, `app_updates_available` | 安装的应用，以及在应用商店中有更新版本的数量 | |
| `app_update_available` | 每个有更新的应用值为 1（最多 32 个） | `app`, `available_version` |
| `active_users` | 在过去 5 分钟、1 小时和 1 天内看到的用户 | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | 账户、文件和实例上的共享 | |
| `free_space_bytes` | Nextcloud 保存其数据的地方的可用空间 | |
| `database_size_bytes` | 数据库的大小 | |
| `platform_info` | 值为 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | PHP `memory_limit` | |
| `opcache_enabled`, `opcache_full` | OPcache 打开；OPcache 已满 | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | OPcache 的饱满程度及其服务效果 | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | PHP-FPM 池，当 Nextcloud 运行在 PHP-FPM 后面时（`-fpm` 镜像，AIO） | |

当 Nextcloud 在维护或等待升级时，其 API 回答 503：仅 `maintenance`，`needs_db_upgrade` 和 `version_info` 被写入，监控项不被报告为不可达。

适用的[内置规则](../alerting/rules.md#self-hosted-applications)：Nextcloud 困在维护中、Nextcloud 数据库升级待处理、Nextcloud 更新可用、Nextcloud OPcache 已满、Nextcloud 可用空间低、加上监控项不可达。

## 后台作业 (cron) {#background-jobs-cron}

Nextcloud 不告知监控令牌后台作业最后何时运行：只有对实例拥有所有权限的账户才能读取，DumbMonit 不要求这样的账户。改用[心跳](push.md)观察 cron：在 DumbMonit 中创建一个，然后在运行 Nextcloud 后台作业的 cron 行末尾调用其 URL，因此仅在 `cron.php` 成功时调用：

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

预期间隔为 5 分钟，一旦 cron 停止，心跳立即警报，无论原因如何（容器重启但其 cron 没有，PHP 错误，后台作业留在 AJAX 上）。

## 客户端设备 {#client-devices}

Nextcloud 不通过任何 API 暴露管理员范围的同步客户端或移动设备列表。个人"设备和会话"页面（设置 → 安全）仅列出当前登录的账户，由浏览器会话而非 API 令牌支持，唯一管理员范围的等价物是命令行（`occ user:auth-tokens:list <user>`，一次一个账户）——监控令牌无法到达，DumbMonit 不要求管理员编写脚本到探针中。DumbMonit 读取的 `serverinfo` API 也没有设备列表：它只提供活跃用户的计数（上面的 `active_users`），从不是谁或他们是什么。

这是一个有意的限制，不是缺失的功能：给 DumbMonit 监控项列表意味着将监控令牌扩展到完整管理员账户，本页其余内容都在避免这种情况。如果 Nextcloud 曾经通过 OCS 暴露这个，DumbMonit 将遵循已用于 Immich、Proxmox Backup Server 和 Veeam 的相同[客户端设备](../alerting/rules.md)模式。

## 监控项页面 {#the-device-page}

面板上方读取探针存储的内容；打开页面从不调用 Nextcloud。它首先说明问题所在（维护、升级等待、OPcache 已满或关闭），然后是更新、活跃用户、可用空间、数据库大小、用户、文件和 OPcache 数据，然后是需要更新的应用名称和平台（数据库、PHP）。

## 为 Nextcloud 提供监控令牌 {#give-nextcloud-a-monitoring-token}

1. Nextcloud 的 serverinfo 应用与 Nextcloud 一起提供并启用，响应打开其统计数据及别的都不行的监控令牌：无文件、无用户、无共享。在 Nextcloud 服务器上，设置一个长随机令牌并打印它。以 web 服务器用户身份运行 occ；在 Docker 中，在前面放置 docker exec -u www-data 和容器名称。

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. 如果 occ 说 serverinfo 被禁用，先启用它。令牌设置后，Nextcloud 可能拒绝第一次读取（401），在其生效前：DumbMonit 在两秒后重试一次，所以全新目标不显示虚假"令牌被拒绝"错误。

    ```
    php occ app:enable serverinfo
    ```

3. 在 DumbMonit 中，输入打开 Nextcloud 的地址，例如"cloud.example.com"（端口 443 的 HTTPS）或"http://10.0.0.5:8080"，并粘贴令牌。主机必须是 Nextcloud 的受信任域之一，否则 Nextcloud 拒绝请求。

4. DumbMonit 仅读取 status.php 和 serverinfo 统计数据。它从不打开文件，从不列出用户，从不更改设置，令牌无处可登。

!!! warning
    Nextcloud 不告知令牌后台作业（cron）最后何时运行：只有对实例拥有所有权限的账户才能读取。要获知 cron 停止时，在 DumbMonit 中添加心跳并在 cron 行末尾调用其 URL，如文档所示。

使用 Nextcloud AIO，在 `nextcloud-aio-nextcloud` 容器中运行 occ：`docker exec -u www-data nextcloud-aio-nextcloud php occ …`。为什么不使用委托管理组：Nextcloud 仅向令牌或完整 `admin` 组的成员打开 serverinfo API，委托不改变这一点。

手工检查令牌，从 DumbMonit 主机：

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| 监控令牌 (NC-Token) | serverinfo 令牌。DumbMonit 在 `NC-Token` 头中随 `OCS-APIRequest: true` 发送。 |

地址：主机名（`cloud.example.com`），`host:port`，或带路径前缀的完整 URL（`https://example.com/nextcloud`）；接受尾部 `/index.php` 或 `/login`。选项：协议（默认 HTTPS），端口（443），证书检查和请求超时（15 秒）。

应用更新计数询问 Nextcloud 应用商店缓存；在无法到达应用商店的实例上，该计数保持为 0。

针对 Nextcloud 35.0.1（官方镜像）验证。
