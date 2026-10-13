# Traefik {#traefik}

反向代理：配置错误导致的路由器禁用、后端服务器健康检查失败、证书解析器未获取或未续期的证书，以及5xx响应的份额。

DumbMonit读取Traefik的API，只读取：`/api/overview`、`/api/http/routers`、`/api/http/services`、`/api/certificates`（由Traefik 3.7提供）和`/api/version`。当Traefik的Prometheus指标在同一端口上提供时，`/metrics`添加请求速率和5xx响应的份额。API没有任何方法会改变任何内容，DumbMonit也不发送任何内容。

值得关注的是无声的故障。一个其服务或中间件不存在、或其规则无法解析的路由器会被禁用：其站点返回404，而Traefik只在日志中说明这一点。一个所有服务器都未通过健康检查的服务返回503"无可用服务器"。一个请求ACME解析器获取证书但解析器无法获取的路由器会获得Traefik的默认自签名证书，浏览器会拒绝。

## 监控内容 {#what-it-watches}

所有指标都以`dumbmonit_traefik_`为前缀。总值是计数器：图表和规则将其转换为速率。

| 指标 | 说明 | 标签 |
|---|---|---|
| `version_info` | 值为1 | `version` |
| `routers`、`routers_errors`、`routers_warnings` | 每个协议的路由器及处于错误或警告状态的路由器 | |
| `services`、`services_errors`、`middlewares_errors` | 服务和中间件的相同指标 | |
| `router_status` | 每个HTTP路由器：0启用，1警告，2禁用 | `router`、`provider` |
| `router_error_info` | 值为1，对于未启用的路由器：Traefik的第一个错误 | `router`、`error` |
| `service_error_info` | 值为1，对于未启用的服务：Traefik的第一个错误 | `service`、`error` |
| `service_status` | 每个服务：0启用，1警告，2禁用 | `service` |
| `service_servers`、`service_servers_up` | 负载均衡服务的服务器及通过健康检查的服务器 | `service` |
| `server_up` | 服务器通过健康检查（或没有健康检查）时为1 | `service`、`server` |
| `resolver_routers` | 请求证书解析器获取证书的路由器 | `resolver` |
| `resolver_routers_uncovered` | 其主机名尚未被Traefik持有的证书覆盖的路由器（需要`/api/certificates`） | `resolver` |
| `certificates`、`cert_expiry_days` | Traefik持有的证书及每个证书过期前的天数 | `cert` |
| `requests_total`、`requests_5xx_total` | 包含指标：每个入口点的请求及5xx响应 | |
| `router_requests_total`、`router_requests_5xx_total` | 包含指标和路由器标签：每个路由器的相同指标 | `router` |
| `config_reloads_total` | 包含指标：配置重载 | |

Traefik自己的路由器和服务（`@internal`）被排除在外。路由器、服务和服务器逐一描述，每个最多500个；总值始终覆盖所有内容。

一个Traefik不再列出的路由器、服务或服务器（Docker提供者会立即删除已停止容器的路由器）只是不再被报告：DumbMonit不为其保留"健康"值，其序列结束。图表和规则可能在指标存储删除陈旧序列的几分钟内仍然显示其最后值。Traefik对这样的路由返回404，没有规则报告：只有被列出且禁用的路由器才会被报告。

适用的[内置规则](../alerting/rules.md#reverse-proxies-and-domains)：Traefik路由器禁用、Traefik后端服务器宕机、Traefik服务无服务器、Traefik证书未获取、代理证书未续期、反向代理返回5xx，加上设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探针存储的内容；打开页面从不查询Traefik。它用一句话说明有什么问题（禁用的路由器和Traefik的原因、无服务器的服务、未获取任何内容的解析器、即将过期的证书、5xx的高份额），然后是路由器、服务、请求速率和5xx份额，然后是每个服务的正常服务器、最即将过期的证书优先，以及24小时内每个路由器的5xx响应。

## 让DumbMonit读取Traefik的API {#let-dumbmonit-read-traefiks-api}

1. Traefik的API只读：路由器、服务和证书，从不改变。通过启用Prometheus指标来启用它，这将添加请求速率和5xx响应的份额：将这两个参数添加到Traefik的命令（或相同的密钥添加到其静态配置）并重启。两者都在traefik入口点上提供，端口8080。

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. 端口8080无需密码应答：仅在DumbMonit主机可以到达的地址上发布它，过滤它以便没有其他任何东西可以到达，并检查它应答。

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. 要改为需要密码，请通过您自己的入口点路由/api和/metrics，并使用basicAuth中间件，如文档所示，并在下方选择basicAuth用户。使用htpasswd创建用户dumbmonit。

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. 在DumbMonit中，输入Traefik主机，例如"traefik.lan"，加上该入口点的端口（如果不是8080）。如果/metrics在其他地方提供，取消选中读取指标。

!!! warning
    配置错误禁用的路由器会导致其站点宕机，而Traefik只在其日志中说明这一点。检查证书解析器是否获取了其证书需要/api/certificates，Traefik 3.7提供该接口；没有它，证书过期只来自指标。

### 带密码的API入口点 {#an-api-entry-point-with-a-password}

不使用`--api.insecure=true`，而是为API和指标提供它们自己的入口点，并在两者前面放置basicAuth中间件。在静态配置中：

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

以及在动态配置中（显示文件提供者；Docker标签工作方式相同），包含`htpasswd`打印的行：

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

然后在DumbMonit中输入`traefik.lan:8082`，用户为`dumbmonit`。`addRoutersLabels`是给予每个路由器5xx响应计数的原因；没有它，只计算每个入口点的总数。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无密码 | 无：API端口被过滤以便只有DumbMonit可以到达它。 |
| basicAuth用户 | /api和/metrics前面basicAuth中间件的用户和密码。 |

地址：主机名或IP（`traefik.lan`）、`host:port`或完整URL。选项：协议（默认HTTP）、API端口（8080）、读取指标（开启）、证书检查和请求超时（10秒）。

没有健康检查的服务将每个服务器报告为正常：Traefik只在`healthCheck`失败时将服务器标记为宕机。加权和镜像服务没有自己的服务器，不计数。
