# MikroTik RouterOS {#mikrotik-routeros}

运行 RouterOS 7 的路由器和交换机:RouterOS 和 RouterBOOT 固件是否
已经落后、CPU、内存和存储、温度、风扇和电源,以及每个接口的状态、
流量、错误和链路丢失情况。

DumbMonit 使用一个只能读取的分组中的用户,读取 RouterOS 7 在
`/rest` 下提供的 REST API。它只发送 `GET` 请求,不做其他任何事。

MikroTik 很少直接损坏;它会老化。某个 RouterOS 发行版不再是其
渠道中的最新版本、升级后被遗留的 RouterBOOT 固件(它只在被要求
时才会跟进,即下次重启时)、一个在没人注意的情况下已经损坏的
冗余电源、一个持续记录错误或丢失链路的端口。这就是本页面所监控
的内容,连同负载情况。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_mikrotik_` 为前缀。总量是计数器:图表和
规则会把它们转换为速率。

| 指标 | 含义 | 标签 |
|---|---|---|
| `info` | 固定值 1 | `version`, `release`(`long-term`、`stable`……)、`board`、`architecture`、`identity` |
| `uptime_seconds` | 自上次启动以来的时间 | |
| `cpu_load_percent`, `cpu_count` | CPU 负载,所有核心 | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | 内存 | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | RouterOS 所在的闪存或磁盘 | |
| `update_checked` | 路由器已检查过是否有更新的 RouterOS 并得到应答时为 1 | |
| `update_available` | 当该渠道的最新版本与已安装版本不同时为 1 | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 在 RouterBOARD 硬件上为 1,在云托管路由器或 x86 机器上为 0 | |
| `firmware_upgrade_pending` | 当 RouterBOOT 比 RouterOS 自带的固件更旧时为 1 | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | 该主板具备的每个传感器 | `sensor`(`cpu-temperature`、`fan1-speed`……) |
| `health_ok` | 当某个 `psu*-state` 或 `fan*-state` 传感器读取为 `ok` 时为 1,否则为 0 | `sensor` |
| `interface_running` | 该接口有链路时为 1 | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | 流量 | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | 错误和丢弃 | `interface`, `type` |
| `interface_link_downs_total` | 链路丢失的次数 | `interface`, `type` |
| `interfaces_skipped` | 超出选项中设定上限而被跳过的接口数 | |
| `scrape_errors` | 上次探测中失败的次要读取操作 | |

只会读取已启用的接口。动态接口(PPPoE 或 L2TP 客户端,其名称
在每次连接时都会变化)和回环接口不包含在内。

传感器取决于主板型号:hAP 有电压和温度传感器,CCR 还带有风扇和
电源传感器,云托管路由器则完全没有。主板不具备的传感器不会产生
任何序列。

只有路由器自己检查更新时,才能知道最新的 RouterOS 版本:只读
用户不允许启动该检查。下面的可选步骤会在路由器上定时安排该检查。
没有它,`update_checked` 会一直保持 0,更新规则也永远不会触发。

适用的[内置规则](../alerting/rules.md#mikrotik-routeros):
MikroTik CPU 过高、内存过高、存储即将用尽、温度过高、电源或
风扇故障、接口错误累积、链路抖动、RouterOS 有可用更新以及固件
升级待处理,以及监控项不可达。

## 监控项页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会连接到路由器。
它显示 RouterOS 版本以及是否有更新版本可用、RouterBOOT 固件、
运行时间、CPU、内存和存储,以及每个传感器及其状态,还有一个
包含链路、流量、错误和链路丢失情况的接口表格。

## 在路由器上创建一个只读用户 {#create-a-read-only-user-on-the-router}

1. 在路由器上打开一个终端(WinBox → New Terminal,或用默认账户通过 SSH),通过 REST API 创建一个只能读取的分组。RouterOS 需要全部三个策略才能通过 REST 读取:rest-api 打开该 API,api 和 read 允许读取本身。

    ```
    /user group add name=dumbmonit-read policy=read,api,rest-api comment=DumbMonit
    ```

2. 在该分组中创建用户,使用你自己设定的较长随机密码。若要让该账户只接受来自 DumbMonit 的连接,请添加 address= 并跟上 DumbMonit 所在主机的地址。

    ```
    /user add name=dumbmonit group=dumbmonit-read password=REPLACE-WITH-A-LONG-RANDOM-PASSWORD
    ```

3. 启用 HTTPS 网页服务,RouterOS 默认是关闭且没有证书的。如果 www-ssl 已经有证书,可跳过这一步。否则,下面的命令会在路由器本机生成一个证书;之后请在下方选项中勾选"接受无法验证的证书"。

    ```
    /certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
    /certificate sign local-ca
    /certificate add name=https-cert common-name=router.lan days-valid=825
    /certificate sign https-cert ca=local-ca
    /ip service set www-ssl certificate=https-cert disabled=no
    ```

4. 可选:若要获知新的 RouterOS 版本,可让路由器在启动时及每天检查一次。只读用户无法启动该检查,因此 DumbMonit 只会读取其结果。

    ```
    /system scheduler add name=check-for-updates interval=1d start-time=startup on-event="/system package update check-for-updates once"
    ```

5. 在 DumbMonit 中,输入路由器的地址,例如 "192.168.88.1",使用 dumbmonit 用户和其密码。DumbMonit 只会向 /rest 发送 GET 请求:它从不更改任何设置,从不启动更新,也从不重启路由器。

!!! warning "警告"
    需要 RouterOS 7.1 或更高版本:RouterOS 6 没有 REST API。
    不要使用默认账户:它所在的分组拥有完整权限,可以更改一切,
    而 dumbmonit-read 没有任何写入策略,任何更改都会被拒绝。
    www 服务(纯 HTTP)也能应答该 REST API,但它会在每个请求中
    以明文方式发送密码。

这三个策略已在 RouterOS 7.23.7 上逐一验证,每次都使用一个全新
用户:没有 `rest-api` 时路由器应答 401,没有 `api` 时应答"not
allowed",没有 `read` 时应答"not enough permissions"。DumbMonit
会把每种情况报告为凭据问题并指明缺失的策略,而不是报告为路由器
宕机。拥有这三者时,任何更改、重启或更新检查都会被以"not
enough permissions"拒绝。

在修改了某个分组的策略之后,RouterOS 可能会在一段时间内仍对
用户应用该分组之前的策略。如果修改了现有分组的策略后错误仍然
存在,请删除并重新创建该用户。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 只读用户 | `dumbmonit` 用户及其密码,在每次请求中以 HTTP 基本认证方式发送。该 REST API 没有会话机制。 |

地址:主机名或 IP(`192.168.88.1`)、`host:port`,或完整 URL
(`https://router.lan:8443`)。选项:协议(默认 HTTPS;HTTP 使用
`www` 服务)、端口(HTTPS 为 443,HTTP 为 80)、证书校验、请求
超时(10 秒)、是否读取接口(开启),以及最多读取多少个(64)。
