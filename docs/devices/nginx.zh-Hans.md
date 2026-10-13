# Nginx {#nginx}

Web 服务器与反向代理:活动和等待中的连接数、接受/处理/请求速率,
以及在商业版 NGINX Plus API 可用时的上游健康状况。

DumbMonit 读取 Nginx 内置的 `stub_status` 模块,这是一组纯文本
计数器,在网络过滤正确配置的情况下不需要任何 API 或账户。当安装
了商业版 NGINX Plus 时,DumbMonit 还能读取其 JSON API,获取上游
健康状况和按区域划分的计数器。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_nginx_` 为前缀。`_total` 指标是计数器:
图表和规则会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `connections_active` | 当前打开的连接数 | |
| `connections_reading`, `connections_writing`, `connections_waiting` | 按当前状态区分的连接数 | |
| `accepts_total`, `handled_total` | 已接受的连接数,以及实际处理的连接数 | |
| `requests_total` | Nginx 启动以来处理的请求数 | |
| `version_info` | 是否存在;即 `Server` 响应头的内容(当 Nginx 发送该头时) | `server` |
| `plus_upstream_server_up` | 当 NGINX Plus 将该服务器标记为正常时为 1 | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | NGINX Plus 按服务器划分的计数器 | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | NGINX Plus 按区域划分的计数器 | `zone` |

`accepts_total` 和 `handled_total` 之间的差值表示 Nginx 在能够处理
之前就丢弃的连接(通常是达到了 `worker_connections` 上限)。
`plus_*` 指标只有在勾选了"读取 NGINX Plus API"且 NGINX Plus 应答时
才会出现;开源版本没有"upstream"的概念,单靠 `stub_status` 无法
报告它。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Nginx。
它显示活动、读取中、写入中和等待中的连接数,过去 24 小时的接受/
处理/请求速率,以及在有 NGINX Plus 时的上游健康状况和按区域划分
的流量。

## 启用 Nginx 的 stub_status 模块 {#enable-nginxs-stub_status-module}

1. Nginx 内置了 stub_status:只需要一个位置块来开启它。把下面这段添加到 Nginx 已有的 server 块中(或为它单独起一个端口的 server 块),然后重新加载 Nginx。

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. 把该位置限制为只允许 DumbMonit 所在主机访问,这样其他人就无法读取它;请按实际情况调整地址。

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. 在配置 DumbMonit 之前,先确认它能应答。

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. 在 DumbMonit 中,输入 Nginx 主机地址,例如 "nginx.lan",如果路径不是 "/basic_status" 则填入实际路径。如果安装了 NGINX Plus,请勾选"读取 NGINX Plus API",以添加上游健康状况和按区域划分的计数器。

!!! warning "警告"
    stub_status 只统计连接和请求数:某个 server 块中的配置错误,
    或 nginx 无法连接的后端,都不会在这里显示出来。开源版本没有
    "upstream" 的概念:那只存在于 NGINX Plus 中,启用后通过它自己
    的 API 读取。

上面的示例按 IP 限制了该位置,这是家庭实验室中最简单的配置;
如果状态页面无法放在自己的防火墙规则之后,`basic_auth` 块同样
可行。`stub_status` 本身无法直接被 Nginx 设置密码保护——如果有
密码,它属于周围的 `location` 块。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无密码 | 不需要:状态页面经过过滤,只有 DumbMonit 能访问它。 |
| 基本认证 | 保护状态页面的那段配置(如果有)的用户名和密码。 |

地址:主机名或 IP,若端口不是 80 则需带上端口。选项:协议
(默认 HTTP)、端口、状态路径(默认 `/basic_status`)、证书校验、
请求超时(10 秒),以及 NGINX Plus API 开关和版本。
