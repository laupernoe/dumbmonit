# pfSense {#pfsense}

通过社区 REST API 插件(`pfSense-pkg-RESTAPI`,版本 2)监控 pfSense
防火墙:宕机、丢包或响应缓慢的网关,无链路的已启用接口,停止运行
的已启用服务,以及 CPU、内存、磁盘和温度。

DumbMonit 在 `X-API-Key` 请求头中发送一个 API 密钥。密钥恰好
拥有其所属 pfSense 用户的权限:下面的 `dumbmonit` 用户只拥有
DumbMonit 读取的五个页面的 `GET` 权限,因此该密钥只能读取这些
页面,不能更改任何内容。DumbMonit 每分钟并行读取
`/api/v2/status/system`、`/api/v2/status/gateways`、
`/api/v2/status/interfaces`、`/api/v2/status/services` 和
`/api/v2/system/version`,以及在授予了相应权限时的
`/api/v2/system/restapi/version`。

!!! warning "仅依据厂商文档验证——尚未在真实系统上测试"

    这项集成是根据该 REST API 插件已发布的文档及其模型源码
    (字段名称、单位、应答信封及其错误标识符)构建的,而不是
    针对一台正在运行的 pfSense。每次调用都是读取操作。如果有
    任何问题,请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_pfsense_` 为前缀。

| 指标 | 标签 | 含义 |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | 与控制台显示的一致,以百分比表示。CPU 数值是 pfSense 自己根据负载平均值做出的估算。 |
| `temperature_celsius` | | 当 pfSense 能够读取某个传感器时。 |
| `load_average` | `period`:`1m`、`5m`、`15m` | 负载平均值。 |
| `cpu_count` | | 处理器核心数。 |
| `version_info` | `version` | 固定值 1:pfSense 版本,`2.7.2-RELEASE`。 |
| `gateway_up` | `gateway` | 当 pfSense 把该网关标记为宕机时为 0,否则为 1。 |
| `gateway_status_info` | `gateway`, `status` | 固定值 1:pfSense 自身的状态词(一切正常时为 `none`,`down`、`loss`、`delay`)。 |
| `gateway_loss_percent` | `gateway` | 到监控地址的丢包率。 |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | 到监控地址的往返时间及其偏差;网关宕机期间不存在。 |
| `gateways`, `gateways_down` | | 计数。 |
| `interface_enabled`, `interface_up` | `interface`, `descr` | 在配置中已启用;链路已建立。 |
| `interface_down` | `interface`, `descr` | 当接口已启用且没有链路时为 1。已禁用的接口永不会被视为宕机。 |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | 计数器。 |
| `interfaces_down` | | 计数。 |
| `service_running`, `service_enabled` | `service`, `description` | 与 Status > Services 显示的一致。 |
| `service_stopped` | `service`, `description` | 当该服务已启用但未运行时为 1。 |
| `services_stopped` | | 计数。 |
| `restapi_update_available` | | 配合 `/api/v2/system/restapi/version` 权限:当该 REST API 插件有新版本时为 1。 |
| `restapi_version_info` | `version`, `latest` | 固定值 1:该插件已安装的版本和最新版本。 |

pfSense 不会通过这个 API 说明是否有 pfSense 本身的更新可用:唯一
涉及系统更新的接口(`/api/v2/system/update`)会直接启动更新。
DumbMonit 只报告该插件自身的更新。

适用的[内置规则](../alerting/rules.md#pfsense):

- **pfSense 网关宕机**:某个网关被标记为宕机持续三分钟
  (警告级别)。
- **pfSense 网关丢包**:丢包率超过 10% 持续十分钟,且该网关
  尚未被标记为宕机(提示级别)。
- **pfSense 网关响应缓慢**:往返时间超过 500 毫秒持续十五分钟
  (提示级别)。对于卫星链路可调高此阈值。
- **pfSense 接口无链路**:某个已启用的接口没有链路持续五分钟
  (提示级别)。
- **pfSense 服务已停止**:某个已启用的服务未运行持续五分钟
  (提示级别)。
- **pfSense 磁盘几乎已满**:超过 90% 持续三十分钟(提示级别)。

## 设备页面 {#the-device-page}

Firewall 面板会用一句话说明哪里出了问题,显示 CPU、内存、磁盘
和温度,然后列出每个网关及其延迟和丢包率、无链路的接口、已停止
的服务,以及该 REST API 插件的可用更新。

## 为 DumbMonit 创建一个只读的 REST API 密钥 {#create-a-read-only-rest-api-key-for-dumbmonit}

1. 从 Diagnostics > Command Prompt 或 SSH shell 安装社区 REST API 插件(pfSense-pkg-RESTAPI,版本 2),使用项目发布页面中与你的 pfSense 版本匹配的文件(下面以 2.7.2 为例)。然后在 System > REST API > Settings 下,在认证方式中勾选 BasicAuth 和 Key 两项。

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. 在 System > User Manager 下,添加一个名为 dumbmonit、密码为较长随机字符串的用户。在 Effective Privileges 下,只添加以下 REST API 权限:/api/v2/status/system GET、/api/v2/status/gateways GET、/api/v2/status/interfaces GET、/api/v2/status/services GET、/api/v2/system/version GET,再加上 /api/v2/system/restapi/version GET 以便获知该插件自身的更新。仅为下一步,再额外添加 /api/v2/auth/key POST。

3. 以 dumbmonit 身份,从任何能访问 pfSense 的机器上创建密钥:该密钥恰好携带该用户的权限。复制应答中的密钥值,然后再次移除 /api/v2/auth/key POST 权限。

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

4. 在 DumbMonit 中,输入 pfSense 的地址,例如 "pfsense.lan",并把该密钥粘贴为 API 密钥。pfSense 自带自签名证书:除非你替换过它,否则请勾选"接受无法验证的证书"。

!!! warning "警告"
    该 REST API 不会说明 pfSense 本身是否有更新,只会说明该插件
    本身是否有更新。该插件是一个社区项目,不属于 pfSense 本身:
    请在每次 pfSense 升级后重新安装它。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| REST API 密钥 | 在第 3 步中为 `dumbmonit` 创建的密钥,以 `X-API-Key` 发送。 |

地址:pfSense 的主机名或 IP、`host:port`,或 URL。选项:

- **协议**(`https`)和**端口**(443):webConfigurator 所使用的,
  在 System > Advanced > Admin Access 下设置。
- **接受无法验证的证书**:pfSense 自身的证书是自签名的。
- **每次请求的超时时间**(15 秒)。

## 故障排查 {#troubleshooting}

"pfSense refused the API key (401)":密钥错误,或在 System >
REST API > Settings 下的认证方式中没有勾选 Key。

"pfSense accepted the API key but refused … (403)":`dumbmonit`
用户缺少该页面所需的权限;错误消息会指明是哪一项。

"… not found: is the REST API package installed?":该插件缺失,
通常是在 pfSense 升级后被移除了。请按第 1 步重新安装它。
