# Traefik

反向代理:因配置错误而被禁用的路由器、健康检查失败的后端服务器、
解析器从未获取或未续签的证书,以及 5xx 应答的比例。

DumbMonit 读取 Traefik 的 API,这些接口只会读取数据:
`/api/overview`、`/api/http/routers`、`/api/http/services`、
`/api/certificates`(由 Traefik 3.7 提供)和 `/api/version`。当
Traefik 的 Prometheus 指标在同一端口上提供服务时,`/metrics` 会
补充请求速率和 5xx 应答比例。该 API 没有任何方法会更改任何内容,
DumbMonit 也不会发送任何这类方法。

值得关注的故障往往很隐蔽。一个服务或中间件不存在、或规则无法
解析的路由器会被禁用:其站点会应答 404,Traefik 只会在日志中
说明这一点。一个所有服务器都未通过健康检查的服务会应答 503
"no available server"。一个向 ACME 解析器请求证书、而解析器
无法获取该证书的路由器,会得到 Traefik 默认的自签名证书,
浏览器会拒绝它。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_traefik_` 为前缀。总量是计数器:图表和
规则会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | 各协议的路由器数,以及出错或警告的数量 | |
| `services`, `services_errors`, `middlewares_errors` | 服务和中间件的同类数据 | |
| `router_status` | 按 HTTP 路由器:0 已启用,1 警告,2 已禁用 | `router`, `provider` |
| `router_error_info` | 固定值 1,针对未启用的路由器:Traefik 给出的第一个错误 | `router`, `error` |
| `service_status` | 按服务:0 已启用,1 警告,2 已禁用 | `service` |
| `service_servers`, `service_servers_up` | 某个负载均衡服务的服务器数,以及通过健康检查的数量 | `service` |
| `server_up` | 当该服务器通过健康检查(或没有健康检查)时为 1 | `service`, `server` |
| `resolver_routers` | 向某个证书解析器请求证书的路由器数 | `resolver` |
| `resolver_routers_uncovered` | 其主机名尚未被 Traefik 持有的任何证书覆盖的路由器数(需要 `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | Traefik 持有的证书数,以及每个证书距离过期的剩余天数 | `cert` |
| `requests_total`, `requests_5xx_total` | 配合指标功能:每个入口点的请求数,以及应答为 5xx 的请求数 | |
| `router_requests_total`, `router_requests_5xx_total` | 配合指标功能及路由器标签:同样的数据,按路由器划分 | `router` |
| `config_reloads_total` | 配合指标功能:配置重新加载次数 | |

Traefik 自身的路由器和服务(`@internal`)不包含在内。路由器、
服务和服务器会逐一描述,各自最多 500 个;总数始终涵盖所有内容。

适用的[内置规则](../alerting/rules.md#reverse-proxies-and-domains):
Traefik 路由器已禁用、Traefik 后端服务器宕机、Traefik 服务没有
可用服务器、Traefik 证书未获取、代理证书未续签、反向代理应答
5xx,以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询
Traefik。它会用一句话说明哪里出了问题(被禁用的路由器及 Traefik
给出的原因、没有可用服务器的服务、什么都没获取到的解析器、即将
过期的证书、过高的 5xx 比例),然后是路由器数、服务数、请求速率
和 5xx 比例,再然后是每个服务可用的服务器数、按过期时间从早到晚
排序的证书,以及过去 24 小时内每个路由器的 5xx 应答情况。

## 让 DumbMonit 读取 Traefik 的 API {#let-dumbmonit-read-traefiks-api}

1. Traefik 的 API 只会读取数据:路由器、服务和证书,从不做任何更改。请连同 Prometheus 指标一起启用它,这会额外提供请求速率和 5xx 应答比例:把下面这两个参数添加到 Traefik 的命令中(或在其静态配置中添加同样的键),然后重启它。两者都会在 traefik 入口点(端口 8080)上提供服务。

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. 8080 端口无需密码即可应答:只在 DumbMonit 所在主机能访问的地址上发布它,过滤掉其他一切访问,并确认它能应答。

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. 若要改为要求密码,请像文档所示,通过一个你自己的入口点并配合 basicAuth 中间件来路由 /api 和 /metrics,并在下方选择 basicAuth 用户。用 htpasswd 创建用户 dumbmonit。

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. 在 DumbMonit 中,输入 Traefik 主机地址,例如 "traefik.lan",如果该入口点端口不是 8080,请一并填入。如果 /metrics 在别处提供服务,请取消勾选"读取指标"。

!!! warning "警告"
    一个因配置错误而被禁用的路由器会使其站点下线,而 Traefik
    只会在日志中说明这一点。检查证书解析器是否获取了证书需要
    `/api/certificates`,这是 Traefik 3.7 提供的接口;没有它,
    证书过期信息只能来自指标数据。

### 带密码的 API 入口点 {#an-api-entry-point-with-a-password}

与 `--api.insecure=true` 不同,可以为 API 和指标单独设立一个
入口点,并在两者前面都加上 basicAuth 中间件。在静态配置中:

```yaml
entryPoints:
  monitor:
    address: ":8082"
api: {}
metrics:
  prometheus:
    manualRouting: true
    addRoutersLabels: true
```

在动态配置中(以文件提供方式为例;Docker 标签方式同理),使用
`htpasswd` 打印出的那一行:

```yaml
http:
  middlewares:
    dumbmonit-auth:
      basicAuth:
        users:
          - "dumbmonit:$2y$05$…"
  routers:
    monitor-api:
      entryPoints: [monitor]
      rule: "PathPrefix(`/api`)"
      service: api@internal
      middlewares: [dumbmonit-auth]
    monitor-metrics:
      entryPoints: [monitor]
      rule: "Path(`/metrics`)"
      service: prometheus@internal
      middlewares: [dumbmonit-auth]
```

然后在 DumbMonit 中输入 `traefik.lan:8082`,用户为 `dumbmonit`。
`addRoutersLabels` 正是提供按路由器划分的 5xx 应答数据的关键;
没有它,只会统计每个入口点的总数。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无密码 | 不需要:该 API 端口已被过滤到只有 DumbMonit 能访问。 |
| basicAuth 用户 | `/api` 和 `/metrics` 前面 basicAuth 中间件的用户名和密码。 |

地址:主机名或 IP(`traefik.lan`)、`host:port`,或完整 URL。
选项:协议(默认 HTTP)、API 端口(8080)、读取指标(开启)、
证书校验和请求超时(10 秒)。

没有健康检查的服务会把每个服务器都报告为正常:Traefik 只有在
`healthCheck` 失败时才会把某个服务器标记为宕机。加权和镜像服务
没有自己的服务器,不会被计数。
