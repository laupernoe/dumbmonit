# Pi-hole {#pi-hole}

DNS 广告拦截器:是否正在拦截、它应答和拦截了多少查询、黑名单的
更新时间、上游解析器是否失败、是否有更新待处理,以及它自身
诊断页面上的消息。

DumbMonit 读取 Pi-hole v6 的 REST API(`/api`)。Pi-hole 5 及更早
版本没有该 API,不受支持。

Pi-hole 出问题时很少表现出来:DNS 仍会继续应答。拦截被暂停"五
分钟"后再也没恢复,每周一次的黑名单更新(gravity)已经失败了一个
月,某个上游解析器一直应答 SERVFAIL,而那个从没人打开过的诊断
页面其实一直都列出了原因。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_pihole_` 为前缀。Pi-hole 统计过去 24 小时
内的查询数,这是一个上下波动的滑动窗口:这些数字是仪表值,带有
`_24h` 后缀,不是计数器。

| 指标 | 含义 | 标签 |
|---|---|---|
| `blocking_enabled` | 当拦截开启时为 1;关闭、暂停或失败时为 0 | |
| `blocking_timer_seconds` | 暂停的拦截自动恢复前剩余的时间 | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | 过去 24 小时内应答和拦截的查询数 | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | 转发到上游的查询数、从缓存应答的查询数、不同的域名数 | |
| `queries_per_second` | 最近一分钟的平均值 | |
| `replies_servfail_24h` | 过去 24 小时内的 SERVFAIL 应答数:上游解析器出现故障时,这里会最先显示出来 | |
| `clients_active`, `clients_total` | 已发现的客户端数量 | |
| `gravity_domains` | 黑名单中的域名数量 | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | 上次黑名单重建的时间,以及距今经过的时间 | |
| `version_info` | 固定值 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 当某个组件发布了新版本时为 1,否则为 0;Pi-hole 每日检查运行之前不存在 | `component`(`core`、`web`、`ftl`、`docker`) |
| `updates_available` | 待安装的更新:在 Docker 中是镜像本身(core、web 和 FTL 都包含在内),否则是每个过期的组件 | |
| `messages`, `messages_by_type` | 诊断页面上的消息 | `type`(`LOAD`、`RATE_LIMIT`、`LIST`……) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | FTL 守护进程 | |
| `privacy_level` | Pi-hole 的隐私级别,0 到 3 | |
| `scrape_errors` | 上次检查中失败的次要 API 调用 | |

在开发分支上,Pi-hole 比较的是提交而不是版本号,DumbMonit 也是
如此。

适用的[内置规则](../alerting/rules.md#pi-hole):Pi-hole 拦截已
禁用、Pi-hole 黑名单过期、Pi-hole 有可用更新、Pi-hole 诊断消息,
以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取检查存储的数据;打开该页面从不会连接到
Pi-hole。它显示拦截是否开启(以及暂停何时结束)、过去 24 小时内
的查询数和被拦截的比例、黑名单的大小和年龄、客户端数量、各组件
版本及待处理的更新,以及诊断消息的数量。

## 在 Pi-hole 中创建应用密码 {#create-an-app-password-in-pi-hole}

1. 需要 Pi-hole v6 或更高版本:版本 5 没有 REST API。Pi-hole 没有只读账户;最接近的是应用密码,它无法更改任何设置。

2. 在 Pi-hole 的网页界面中,打开 Settings → Web interface / API,把页面从 Basic 切换到 Expert。点击 Configure app password,复制显示出的密码,然后点击 Enable new app password。Pi-hole 只会显示一次,新密码会替换旧密码。

3. 只要 webserver.api.app_sudo 为 false(默认值),应用密码就始终对设置保持只读。在 Pi-hole 所在机器上检查这一点(在 Docker 中,请在命令前加上 docker exec 和容器名):

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. 在 DumbMonit 中,输入 Pi-hole 地址,例如 "pi.hole" 或 "192.168.1.53",并粘贴该应用密码。对于网页界面没有设置密码的 Pi-hole,请改为选择"无密码"。

5. DumbMonit 只会读取数据。它从不暂停拦截,也从不更改任何列表,并且会在两次检查之间保持一个 API 会话打开,而不是每次检查都新开一个:Pi-hole 同时最多允许 16 个会话。

!!! warning "警告"
    应用密码仍然可以通过 API 暂停拦截和编辑列表:请只把它用于 DumbMonit。Pi-hole 在 80 端口上提供纯 HTTP 服务,密码会在每次打开会话时经过网络传输:在不受信任的网络中,请使用 443 端口的 HTTPS。

当启用了两步验证时,应用密码也可以不经过第二因素直接登录:这正是
它存在的目的。要撤销它,请在 Settings → All settings 下清空
`webserver.api.app_pwhash`,或启用一个新的密码:旧密码会立即失效。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 应用密码 | 上面生成的密码。DumbMonit 会用它换取一个会话,通过 `X-FTL-SID` 请求头发送,并在 Pi-hole 保留该会话期间(自上次使用起 30 分钟)重复使用它。 |
| 无密码 | 仅适用于网页界面没有设置密码的 Pi-hole。 |

地址:主机名或 IP(`pi.hole`)、`host:port`,或完整 URL
(`https://pi.hole`);从浏览器复制的网页界面或 API 路径会被忽略。
选项:协议(默认 HTTP)、端口(80)、证书校验(对于 443 端口上的
HTTPS,Pi-hole 自身的证书是自签名的)和请求超时(10 秒)。

如果检查失败并提示"no free API session",说明 Pi-hole 的会话
已全部被占用,通常是某个脚本每次运行都登录却从不登出造成的。
它们会在 30 分钟后过期;`webserver.api.max_sessions` 可以提高该
上限。
