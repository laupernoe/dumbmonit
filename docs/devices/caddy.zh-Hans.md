# Caddy

Web 服务器与反向代理:健康检查失败的上游、失败的配置重新加载、
处理器错误以及 5xx 应答的比例。

DumbMonit 从 Caddy 的管理 API 读取三项内容,全部使用 `GET`:
`/config/apps/http/servers`(Caddy 正在运行的 HTTP 配置,绝不读取
存放 DNS 提供商令牌的 `tls` 部分)、`/reverse_proxy/upstreams`
(后端以及被动健康检查记住的故障)和 `/metrics`(上游健康状况、
重新加载、请求和错误)。

Caddy 的管理 API 不是只读的:任何能访问它的人都可以加载一份新
配置或停止 Caddy。正因如此它默认监听在 `localhost:2019`。下面的
设置会让它保持在那里,并让 Caddy 自己发布一个受密码保护的只读
视图,该视图只应答上面三个路径的 `GET` 请求,拒绝其他一切。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_caddy_` 为前缀。总量是计数器:图表和规则
会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `config_loaded` | 当 Caddy 至少运行一个 HTTP 服务器时为 1 | |
| `servers`, `routes` | 配置中的 HTTP 服务器数和路由数 | |
| `config_last_reload_ok` | 当上次配置重新加载失败时为 0 | |
| `config_last_reload_timestamp_seconds` | 上次成功重新加载的时间 | |
| `upstreams`, `upstreams_unhealthy` | 每个 `reverse_proxy` 的后端数,以及失败的后端数 | |
| `upstream_healthy` | 当主动和被动健康检查都未将该后端排除时为 1 | `upstream` |
| `upstream_fails` | 被动健康检查当前记住的失败请求数 | `upstream` |
| `upstream_requests` | 当前发往该后端的请求数 | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | 配合 `metrics` 选项:请求数、5xx 应答数和处理器错误数 | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | 同上,按服务器划分(`srv0`、`srv1`……或你指定的名称) | `server` |

`upstream_healthy` 需要 `/metrics`:主动健康检查只能在那里看到。
不放行 `/metrics` 的只读视图仍会报告后端及其记住的故障。Caddy
不会通过管理 API 公布其版本。

适用的[内置规则](../alerting/rules.md#reverse-proxies-and-domains):
Caddy 上游不健康、Caddy 配置重新加载失败、反向代理应答 5xx,
以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Caddy。
它会用一句话说明哪里出了问题(没有配置、重新加载失败、上游不
健康、5xx 比例过高),然后是过去 24 小时的服务器数、路由数、
上游数、请求速率、5xx 比例和处理器错误,再然后是每个上游及其
记住的故障。

## 发布 Caddy 管理 API 的只读视图 {#publish-a-read-only-view-of-caddys-admin-api}

1. Caddy 的管理 API 可以加载新配置并停止 Caddy:它必须保持只有 DumbMonit 能访问。让它保留在 Caddy 默认设置的 localhost:2019,然后让 Caddy 发布它的一个只读视图:只允许 GET,限定在 DumbMonit 读取的三个路径上,并要求密码。

2. 为用户 dumbmonit 创建密码哈希。

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

3. 把下面这段添加到 Caddyfile 中,用该哈希值替换 HASH,然后重新加载 Caddy。它会在 2020 端口上应答,除上述三种读取请求外拒绝其他一切。

    ```
    :2020 {
    basic_auth {
    dumbmonit HASH
    }
    @readonly {
    method GET
    path /config/apps/http/servers /reverse_proxy/upstreams /metrics
    }
    handle @readonly {
    reverse_proxy localhost:2019 {
    header_up Host localhost:2019
    }
    }
    respond 403
    }
    ```

4. 可选:在 Caddyfile 顶部的全局选项块中添加 metrics,以统计每个服务器的请求、错误和 5xx 应答数。没有它,上游健康状况和重新加载仍会被报告。

    ```
    metrics
    ```

5. 过滤 2020 端口,使只有 DumbMonit 所在主机能访问它。在 DumbMonit 中,输入带该端口的 Caddy 主机地址,例如 "caddy.lan:2020",以及用户 dumbmonit 和其密码。

!!! warning "警告"
    绝不要在没有前置防火墙的情况下把管理 API 本身发布到网络上:
    任何能访问 2019 端口的人都可以替换配置或停止 Caddy,而
    origins 选项无法阻止不发送 Origin 请求头的客户端。如果你确实
    要在私有网络上公开它,请过滤该端口使只有 DumbMonit 所在主机
    能访问,并选择"无密码"。

Caddyfile 不依赖缩进来表达含义;`caddy fmt` 会对它进行缩进。
`header_up Host localhost:2019` 是必需的,因为管理 API 只应答
寻址到其监听地址的请求。使用 Caddy 容器镜像时,请用 `docker
exec` 运行哈希命令。

以 JSON 而不是 Caddyfile 加载的配置,会得到同样的站点,作为一个
带有 `basic_auth` 处理器、`method` 和 `path` 匹配器以及指向
`localhost:2019` 的 `reverse_proxy` 的额外服务器。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 只读视图 | 上面 `basic_auth` 块中的用户名和密码(推荐)。 |
| 无密码 | 不需要:管理 API 本身,已过滤到只有 DumbMonit 能访问。 |

地址:主机名或 IP,带只读视图的端口(`caddy.lan:2020`),或完整
URL。选项:协议(默认 HTTP)、端口(2019,即管理 API 本身)、
证书校验和请求超时(10 秒)。

被动健康检查(`fail_duration`)只会在该时长内记住故障:一个失败
一次又恢复的后端,会在此之前一直被报告为不健康。该规则会等待
五分钟才触发。
