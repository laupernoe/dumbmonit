# Apache httpd {#apache-httpd}

Web 服务器:忙碌和空闲的工作进程数、按状态细分的工作进程记分板,
以及开启 ExtendedStatus 时的请求/字节速率。

DumbMonit 读取 Apache 内置的 `mod_status`,查询时带上 `?auto`
参数,使其以简单的 `Key: Value` 行格式应答,而不是面向浏览器的
HTML 页面。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_apache_` 为前缀。`_total` 指标是计数器:
图表和规则会把它们转换为速率。`workers_busy`/`workers_idle` 和
`scoreboard` 以下的全部指标都需要开启 `ExtendedStatus On`。

| 指标 | 含义 | 标签 |
|---|---|---|
| `workers_busy`, `workers_idle` | 按状态区分的工作进程或线程 | |
| `scoreboard` | 按记分板状态区分的工作进程槽位 | `state` |
| `requests_total` | Apache 启动以来处理的请求数 | |
| `sent_bytes_total` | Apache 启动以来发送的字节数 | |
| `uptime_seconds` | Apache 启动以来经过的时间 | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | Apache 自身计算的运行平均值 | |
| `cpu_load_ratio` | Apache 自身的 `CPULoad` 值 | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | 异步连接计数(event 和 worker MPM) | |
| `version_info` | 是否存在;即 `Server` 响应头的内容 | `version` |

`scoreboard` 的总和始终等于配置的工作进程槽位数:各状态包括
waiting、open_slot、starting、reading、sending、keepalive、
dns_lookup、closing、logging、finishing 和 idle_cleanup。

## 监控项页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Apache。
它显示忙碌和空闲的工作进程数、记分板明细,以及在开启
ExtendedStatus 时,过去 24 小时的请求和字节速率。

## 启用 Apache 的 mod_status {#enable-apaches-mod_status}

1. 确保 mod_status 已加载(它随 Apache 发布,通常默认启用)。

    ```
    a2enmod status
    ```

2. 添加一个状态位置,限制只允许 DumbMonit 所在主机访问;请按实际情况调整地址。在 Debian 和 Ubuntu 上这通常放在一个独立文件中,在其他系统上则放在主配置文件里。

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. 开启请求和字节计数器(可选,但值得开启):不开启的话,只会报告忙碌/空闲工作进程和记分板。

    ```
    ExtendedStatus On
    ```

4. 重新加载 Apache,并在配置 DumbMonit 之前确认它能应答。

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. 在 DumbMonit 中,输入 Apache 主机地址,例如 "apache.lan",如果路径不是 "/server-status" 则填入实际路径(DumbMonit 会自行添加 "?auto")。

!!! warning "警告"
    不开启 ExtendedStatus 时,请求数和字节数根本不会被报告,
    而不是报告为零:图表会保持空白,而不是显示一条虚假的平线。
    Apache 不会在 mod_status 中发布自己的版本号;DumbMonit 改为
    从 Server 响应头读取它,而有些发行版会故意削减这个信息。

在 Debian 和 Ubuntu 上,状态位置通常已经存在于
`/etc/apache2/mods-available/status.conf` 中;只需修改其中的
`Require` 行即可,无需新建文件。在配置是单一文件的系统上
(大多数其他发行版),请把这段配置添加到主虚拟主机中。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无密码 | 不需要:状态页面经过过滤,只有 DumbMonit 能访问它。 |
| 基本认证 | 保护状态页面的那段配置(如果有)的用户名和密码。 |

地址:主机名或 IP,若端口不是 80 则需带上端口。选项:协议
(默认 HTTP)、端口、状态路径(默认 `/server-status`,自动添加
`?auto`)、证书校验和请求超时(10 秒)。
