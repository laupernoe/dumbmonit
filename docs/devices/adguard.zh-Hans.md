# AdGuard Home {#adguard-home}

DNS 过滤器本身:保护是否开启、其 DNS 服务器是否运行、它应答和
拦截了什么内容、它的过滤列表是否仍在更新、它的上游服务器是否
应答,以及是否有新版本发布。

一个损坏的 DNS 过滤器看起来并不像是坏了。页面照常打开,广告也
照常出现。常见的故障往往悄无声息:从手机应用暂停保护"一小时"
后再也没有恢复、过滤列表几周前就停止下载了、某个上游的
DNS-over-HTTPS 服务器不再应答,导致每次查询都要等到下一个服务器。

DumbMonit 使用 HTTP 基本认证读取 AdGuard Home 的 REST API
(`/control`)。已在 AdGuard Home v0.107.79 上测试(以及
v0.107.78 的版本检查)。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_adguard_` 为前缀。查询和拦截数据是
**AdGuard Home 统计窗口内的总和**(默认 24 小时,可在设置 →
常规设置中更改),不是计数器:它们会随着窗口滑动而上下波动,
按原样存储。

| 指标 | 含义 | 标签 |
|---|---|---|
| `running` | DNS 服务器运行时为 1 | |
| `protection_enabled` | 保护开启时为 1 | |
| `protection_paused_seconds` | 被暂停的保护恢复前剩余的时间,无暂停时为 0 | |
| `version_info` | 固定值 1 | `version` |
| `dns_queries` | 统计窗口内的查询数 | |
| `blocked_filtering` | 被过滤列表和自定义规则拦截的查询数 | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | 被安全浏览、家长控制、安全搜索替换的应答数 | |
| `blocked_percent` | 被拦截查询的比例(过滤器、安全浏览、家长控制);没有任何查询时不存在 | |
| `avg_processing_seconds` | 平均应答时间 | |
| `stats_window_seconds`, `stats_enabled` | 统计窗口以及统计功能是否保留(0.107.30 及更高版本) | |
| `upstream_responses`, `upstream_avg_seconds` | 窗口内每个上游的应答数和平均应答时间,与控制台"Top upstreams"显示的一致(最多 8 个) | `upstream` |
| `upstream_up` | 当 AdGuard Home 对该上游的自测通过时为 1;失败或 4 秒内没有应答时为 0(最多 8 个)。标签是 AdGuard Home 在其统计中写入该上游的方式(`https://dns10.quad9.net:443/dns-query`、`192.0.2.1:53`);对于保持沉默的上游,则按配置中的写法。 | `upstream` |
| `filtering_enabled` | 当设置 → 常规设置中开启了过滤功能时为 1 | |
| `filters`, `filters_enabled` | 配置的黑名单数,以及已启用的数量 | |
| `filter_rules` | 从已启用列表加载的规则数 | |
| `filter_rules_count`, `filter_update_age_seconds` | 按每个已启用列表(最多 32 个) | `filter` |
| `filter_oldest_update_age_seconds` | 已启用列表中最旧更新的年龄 | |
| `filters_never_updated` | 从未下载过的已启用列表数 | |
| `filter_update_interval_hours` | 列表更新的频率;0 表示关闭更新 | |
| `update_check_enabled` | 当 AdGuard Home 关闭了版本检查时为 0 | |
| `update_available` | 当上次版本检查宣布有新版本时为 1;检查关闭时不存在 | `latest` |
| `scrape_errors` | 上次检查中失败的次要调用 | |

官方 Docker 镜像和大多数发行版软件包中版本检查是关闭的,因为它们
自己负责更新 AdGuard Home:在这些情况下,`update_available` 永远
不会被写入,对应的规则会一直保持静默。

上游统计只统计应答:一个停止应答的上游在统计中不会留下任何痕迹。
这正是 DumbMonit 在每次检查时都要求 AdGuard Home 测试每个上游的
原因,这与设置 → DNS 设置中"Test upstreams"按钮的测试相同:
由 AdGuard Home 向该上游发出一次 DNS 查询。一个失效的上游会让
AdGuard Home 等待其上游超时时间的两倍(默认 20 秒);DumbMonit
会在 4 秒后停止等待,并报告该上游已失效。取消勾选"测试上游
服务器"选项可以关闭此行为。

适用的[内置规则](../alerting/rules.md#adguard-home):AdGuard
Home 保护已关闭、AdGuard Home DNS 未运行、AdGuard Home 上游失效、
AdGuard Home 过滤列表过期、AdGuard Home 有可用更新,以及监控项
不可达。

## 监控项页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会连接到
AdGuard Home。它说明保护和 DNS 服务器是否开启(以及暂停何时
结束),然后是统计窗口内的查询数、拦截数和应答时间,上游服务器
及其测试结果和应答时间,以及过滤列表及其规则数和上次更新时间。

## 为 DumbMonit 在 AdGuard Home 中添加一个用户 {#add-a-user-for-dumbmonit-to-adguard-home}

1. AdGuard Home 没有只读账户:AdGuardHome.yaml 中列出的每个用户都能更改每一项设置。请为 DumbMonit 设立一个独立的用户,以便可以单独移除它而不影响你自己的账户。首先用 bcrypt 为一个较长的随机密码生成哈希值(htpasswd 随 apache2-utils 或 httpd-tools 软件包提供):

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. 停止 AdGuard Home,打开 AdGuardHome.yaml(与 AdGuardHome 二进制文件同目录,或在 Docker 卷的 conf 文件夹中),在"users:"下、与已有的"- name:"行相同的缩进处添加这一行。把 HASH 替换为 htpasswd 在"dumbmonit:"之后打印的内容。再次启动 AdGuard Home。

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    该部分最终会是这样:

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. 从 DumbMonit 所在主机确认该用户被接受:

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. 在 DumbMonit 中,输入 AdGuard Home 网页界面的地址,例如 "adguard.lan"(端口 80)或 "http://adguard.lan:3000",然后输入用户 dumbmonit 和其密码。

5. DumbMonit 只会读取状态、统计数据、过滤列表和上次版本检查结果。它从不读取查询日志。在每次检查时,它都会要求 AdGuard Home 测试每个上游 DNS 服务器,就像"Test upstreams"按钮那样;取消勾选下方的"测试上游服务器"可以关闭此行为。

!!! warning "警告"
    dumbmonit 用户能更改每一项设置,与任何 AdGuard Home 用户
    一样:请只把它的密码留给 DumbMonit。它会随每个请求传输,
    因此在不受信任的网络中,请在设置 → 加密设置中开启 HTTPS。
    连续五次密码错误后,AdGuard Home 会拒绝该地址 15 分钟。

编辑该文件时必须先停止 AdGuard Home:只要设置发生更改,它就会
重写 AdGuardHome.yaml,会丢弃运行期间添加的任何一行。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户名/密码 | 添加到 AdGuardHome.yaml 中的专用用户,以 HTTP 基本认证方式发送。 |
| 无认证 | 仅适用于 AdGuardHome.yaml 中完全没有任何用户的 AdGuard Home。 |

地址:主机名或 IP(`adguard.lan`)、`host:port`,或反向代理之后
带路径前缀的完整 URL(`https://dns.lan/adguard`);结尾的
`/control` 会被忽略。选项:协议(默认 HTTP)、端口(80)、证书
校验、请求超时(5 秒)和上游测试(开启)。

## 调用 {#calls}

| 调用 | 原因 |
|---|---|
| `GET /control/status` | 运行状态、保护状态、版本:唯一一个失败会使整次检查失败的调用 |
| `GET /control/stats`, `GET /control/stats/config` | 查询数、拦截数、上游应答情况、统计窗口 |
| `GET /control/filtering/status` | 过滤列表 |
| 携带 `{"recheck_now": false}` 的 `POST /control/version.json` | 上次版本检查的结果,与网页界面在每次加载页面时读取的内容相同;它从不强制进行新的检查 |
| `GET /control/dns_info`,然后对每个上游调用一次 `POST /control/test_upstream_dns` | 已配置的上游和引导服务器,然后是上游测试;不会更改任何设置 |
