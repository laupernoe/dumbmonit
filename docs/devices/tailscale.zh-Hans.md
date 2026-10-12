# Tailscale

通过 Tailscale API 监控某个 tailnet 的设备:离线的服务器、即将
过期的节点密钥、等待批准的设备,以及有可用更新的客户端。

DumbMonit 每分钟读取一次 `api.tailscale.com` 上的
`GET /api/v2/tailnet/{tailnet}/devices`。推荐使用的凭据是一个
限定了 `devices:core:read` 范围的 OAuth 客户端:DumbMonit 用其
ID 和密钥换取一个一小时有效的令牌(`/api/v2/oauth/token`),并
在它过期前保留使用。它能列出设备,不能更改任何内容。API 访问
令牌(`tskey-api-…`)也可用,但它携带创建它的用户的全部权限,
且最多 90 天后过期。

这是一个互联网 API:设备地址指的是该 API 的地址,而不是某台
机器的地址。DumbMonit 需要能够向外访问 `api.tailscale.com` 的
HTTPS;tailnet 内部不会被联系。

!!! warning "仅依据厂商文档验证——尚未在真实系统上测试"

    这项集成是根据 Tailscale 的 API 文档和其官方 Go 客户端的
    设备模型构建的,而不是针对一个正在运行的 tailnet。它只会
    读取数据。如果有任何问题,请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_tailscale_` 为前缀。设备以其 MagicDNS
名称的第一个标签命名(`nas.tail1234.ts.net` 对应 `nas`)。从
另一个 tailnet 共享进来的设备会被计数,但不会被详细描述。

| 指标 | 标签 | 含义 |
|---|---|---|
| `device_online` | `device` | 该设备连接到协调服务器期间为 1。 |
| `device_offline_seconds` | `device` | 在线时为 0,否则为自上次被发现以来经过的时间。 |
| `device_watched` | `device` | 当该设备必须保持在线时为 1:默认是每个带标签的设备,或下方选项中列出的设备。 |
| `device_key_expiry_seconds` | `device` | 距节点密钥过期的时间;密钥过期功能被禁用时不存在。 |
| `device_authorized` | `device` | 该设备等待批准期间为 0。 |
| `device_update_available` | `device` | 当 Tailscale 提供更新的客户端时为 1。 |
| `device_info` | `device`, `os`, `client_version`, `tags` | 固定值 1。 |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | 计数。 |

只有被监控的设备离线时才会触发告警:一台进入休眠的手机或笔记本
电脑不算是服务中断。带标签的设备在实践中通常是服务器、子网路由
器和出口节点;请在**要保持在线的设备**选项中列出其他设备,或
输入 `all`。

适用的[内置规则](../alerting/rules.md#tailscale):

- **Tailscale 设备离线**:某个被监控的设备断开连接超过 15 分钟
  (提示级别)。
- **Tailscale 节点密钥即将过期**:剩余不足 14 天(提示级别,
  每天提醒)。过期后,该设备会离开 tailnet,直到有人重新登录它。
- **Tailscale 设备等待批准**(提示级别):如果是预期的设备就
  批准它,否则就移除它。
- **Tailscale 客户端更新**:某个被监控的设备有客户端更新已存在
  六小时(信息级别,每周提醒)。

## 设备 {#devices}

该 tailnet 中的每一台设备,无论是否被监控(共享进来的设备除外),
都会以每个追踪客户端设备的 DumbMonit 集成共享的通用指标族发布——
`dumbmonit_client_device_*`,前缀中不含 `tailscale`,完整文档见
[客户端设备](../alerting/rules.md#client-devices):

| 指标 | 含义 | 标签 |
|---|---|---|
| `last_seen_timestamp_seconds` | 上次连接:当 `connectedToControl` 为真时为"现在",否则为 `lastSeen` | `device`, `type`(`Tagged` 或 `Personal`)、`os`、`user`、`kind` |
| `stale_seconds` | 当某设备超过 `device_stale_days`(默认 3 天)未连接时为正值;驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`, `type`, `os`, `user`, `kind`, `signal`(始终为 `connection`:Tailscale 没有备份可报告) |

一台经常出行的手机或笔记本电脑可能好几天都不重新连接:这与上面
的**要保持在线的设备**设置无关,那个设置只会在被监控的设备离线
时触发告警。对于设备会漫游的 tailnet,请调高**设备陈旧判定**。

## 设备页面 {#the-device-page}

Tailnet 面板会用一句话说明哪里出了问题,显示在线的设备数、离线
的被监控设备数、可用的更新数和共享进来的设备数,然后列出被监控
的设备,以及任何等待批准或密钥即将过期的设备,再然后是每台设备
及其上次连接时间的表格。

## 为 DumbMonit 创建一个只读 OAuth 客户端 {#create-a-read-only-oauth-client-for-dumbmonit}

1. 在 Tailscale 控制台(login.tailscale.com)中,打开 Settings > Trust credentials(原 OAuth clients),生成一个只具备 devices:core:read 范围的 OAuth 客户端。它能列出设备,不能更改任何内容。

2. 复制客户端 ID 和客户端密钥(tskey-client-…),它们只会显示一次。在 DumbMonit 中,输入 api.tailscale.com 作为地址,并把它们粘贴为 Client ID 和 Client secret。把 Tailnet 选项留空:会读取该客户端所属的 tailnet。

!!! warning "警告"
    只有带标签的设备(服务器、子网路由器、出口节点)才应该保持
    在线:一台进入休眠的手机或笔记本电脑不算是服务中断。请在
    "要保持在线的设备"选项中指定要监控的设备,或输入 all。
    tailnet 中的每一台设备都会出现在下方的设备表格中,附带其
    上次连接时间,无论是否被监控;经常出行的手机可能好几天都
    没有连接记录,因此请为这些设备调高"设备陈旧判定"。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| OAuth 客户端 | 具有 `devices:core:read` 范围的客户端的 Client ID 和 Client secret(`tskey-client-…`)。 |
| API 访问令牌 | 一个 `tskey-api-…` 令牌,以 bearer token 方式发送。 |

地址:`api.tailscale.com`。选项:

- **Tailnet**:留空表示使用凭据所属的 tailnet,或填入 Tailscale
  控制台中显示的 tailnet 名称。
- **要保持在线的设备**:以逗号分隔的设备名称或标签
  (`nas, tag:router`),或 `all`。留空:每个带标签的设备。
- **每次请求的超时时间**(15 秒)。
- **设备陈旧判定**(`device_stale_days`,默认 3 天)。

## 故障排查 {#troubleshooting}

"Tailscale refused the OAuth client":ID 或密钥错误,或该客户端
已被删除。

"Tailscale refused to list the devices (403)":该客户端缺少
`devices:core:read` 范围。

"Tailscale knows no tailnet …":请将 Tailnet 选项留空。
