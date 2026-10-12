# Jellyfin

Jellyfin 是否让媒体库保持最新、能持续为观众提供服务:上次运行失败的
计划任务(首先是媒体库扫描)、不再加载的插件、等待中的重启,以及
当前正在播放的内容,是否经过转码。

DumbMonit 通过在控制台中创建的**API 密钥**读取 Jellyfin 的 API。
它从不启动任务,从不停止播放,也从不更改任何设置。

值得关注的故障是:**媒体库扫描每晚都失败**。网络共享被卸载、
磁盘已满,新的电影和剧集就不再出现;Jellyfin 本身仍会照常播放
它已经知道的内容,唯一的痕迹只是计划任务里的一行记录。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_jellyfin_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `pending_restart` | 当 Jellyfin 等待重启时(安装或更新插件之后)为 1 | |
| `shutting_down` | Jellyfin 关闭过程中为 1 | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | 上次运行失败或被中止的任务;当前正在运行的任务 | |
| `scheduled_task_failed` | 每个失败任务对应的值为 1(最多 32 个) | `task`, `error`(消息的第一行) |
| `library_scan_last_ok`, `library_scan_age_seconds` | "Scan Media Library" 的上次运行:是否成功,以及结束至今经过的时间 | |
| `plugins`, `plugins_broken` | 已安装的插件;加载失败或不支持当前版本的插件 | |
| `plugin_broken` | 每个损坏插件对应的值为 1 | `plugin`, `status` |
| `sessions`, `streams`, `streams_paused`, `transcodes` | 客户端会话;正在播放的、已暂停的,以及正在转码的 | |
| `scrape_errors` | 上次探测中失败的调用 | |

Jellyfin 不会检查自身的更新(这是软件包或镜像的职责),因此没有
更新相关的指标。

适用的[内置规则](../alerting/rules.md#self-hosted-applications):
Jellyfin 计划任务失败、Jellyfin 插件损坏、Jellyfin 重启等待中,
以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会调用 Jellyfin。
它会先说明出了什么问题(失败的任务、损坏的插件、等待中的重启),
然后是流、转码、会话以及上次媒体库扫描的时间,接着按名称列出
失败的任务和损坏的插件,并附上 Jellyfin 自身给出的错误信息。

## 在 Jellyfin 中创建 API 密钥 {#create-an-api-key-in-jellyfin}

1. 在 Jellyfin 中:Dashboard → API Keys(位于 Advanced 下)→ +,按如下方式命名该密钥,然后复制它。

    ```
    DumbMonit
    ```

2. Jellyfin 无法限制一个密钥能做什么:每个密钥在服务器上都拥有完整权限。但这仍是读取计划任务和所有人播放状态的唯一方式;普通用户看不到这些。

3. 在 DumbMonit 中,输入 Jellyfin 的地址,例如 "jellyfin.lan"(端口 8096),或反向代理之后的 "https://media.example.com",并粘贴该密钥。

4. DumbMonit 只读取 /System/Info、/ScheduledTasks、/Sessions 和 /Plugins。它从不启动任务,从不停止播放,也从不更改任何设置。

!!! warning "警告"
    由于该密钥拥有全部权限,请只在你信任的网络中使用它,并让 Jellyfin 在公网上通过 HTTPS 提供服务。如果它泄露了,请在同一页面上撤销它:没有其他任何东西在使用它。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| API 密钥 | 在 `Authorization: MediaBrowser … Token="…"` 请求头中发送,这是 Jellyfin 10.11 及更高版本默认接受的格式。 |

地址:主机名(`jellyfin.lan`)、`host:port`,或带路径前缀的完整 URL
(`https://example.com/jellyfin`);结尾的 `/web` 也会被接受。
选项:协议(默认 HTTP)、端口(8096)、证书校验和请求超时(15 秒)。

已在 Jellyfin 12.1.0(官方镜像)上验证。
