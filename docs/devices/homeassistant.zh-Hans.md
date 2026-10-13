# Home Assistant {#home-assistant}

仪表盘背后的智能家居:Home Assistant 是否在运行,还是以恢复模式
启动的、按域分类的不可用或未知实体、电池电量低、等待中的更新
(核心、操作系统、附加组件、设备固件),以及待处理的修复项。

DumbMonit 使用一个**没有管理员权限**的专用用户的长期访问令牌读取
Home Assistant 的 REST API,并通过 WebSocket API 读取修复项列表。
它从不调用任何服务,从不触发任何事件,也从不更改任何状态。

值得关注的故障不是"Home Assistant 不应答"(这一点
[网站检查](services.md#http)已经能发现),而是**Home Assistant
能应答,但其背后的某些东西停止工作了**:某次更新之后某个集成的
所有实体全部变为不可用、一个 Zigbee 适配器从 USB 总线上脱落、
某个门传感器的电池电量降到了 5%、一项修复待处理了好几周。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_homeassistant_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `info` | 固定值 1 | `version` |
| `running` | 核心状态为 `RUNNING` 时为 1,启动或停止期间为 0 | |
| `recovery_mode` | 处于恢复模式(较旧版本上为安全模式)时为 1:配置无法加载 | |
| `entities` | 按域划分的实体数 | `domain` |
| `entities_unavailable`, `entities_unknown` | 按域划分处于该状态的实体数 | `domain` |
| `entity_unavailable` | 每个不可用实体对应的值为 1,最多 50 个 | `entity`, `name`, `domain` |
| `batteries` | 电池传感器和电池二元传感器数量 | |
| `batteries_low` | 低于阈值的电池数,以及处于开启状态的电池二元传感器数 | |
| `battery_low` | 每个低电量电池对应一条序列,值为其电量百分比(二元传感器为 0),最多 50 个 | `entity`, `name` |
| `updates_available` | 处于开启状态的 `update.*` 实体数 | |
| `update_available` | 每个待处理更新对应的值为 1,最多 50 个 | `entity`, `name`, `installed`, `latest` |
| `repairs` | 按严重级别划分的待处理修复项(既未忽略也未关闭):`critical`、`error`、`warning` | `severity` |
| `repair` | 每个待处理修复项对应的值为 1,最多 50 个 | `issue`, `domain`, `severity` |
| `scrape_errors` | 上次探测中失败的调用 | |

有些域本质上就停留在"未知"状态:从未被按下的按钮、从未收到的
事件、通知或语音服务。这些不会被计为未知:`button`、
`input_button`、`event`、`scene`、`notify`、`tts`、`stt`、
`conversation`、`wake_word`、`ai_task`、`person`。**排除的域**
选项可以把更多域(例如经常处于关闭状态的设备所用的
`device_tracker` 或 `media_player`)从不可用和未知的计数中排除。

修复项不在 REST API 中:DumbMonit 会打开 WebSocket API,发送
`repairs/list_issues`,然后关闭连接。一个没有管理员权限的用户
也可以列出它们。**读取修复项**选项可以关闭这一功能。

适用的[内置规则](../alerting/rules.md#home-assistant):Home
Assistant 处于恢复模式、Home Assistant 未运行、Home Assistant
实体变为不可用、Home Assistant 电池电量低、Home Assistant 有可用
更新、Home Assistant 有待处理的修复项,以及设备不可达。

## 设备 {#devices}

登录到配套应用(`mobile_app` 集成)的每一台手机和平板,都会以每个
追踪客户端设备的 DumbMonit 集成共享的通用指标族发布——
`dumbmonit_client_device_*`,前缀中不含 `homeassistant`,完整
文档见[客户端设备](../alerting/rules.md#client-devices):

| 指标 | 含义 | 标签 |
|---|---|---|
| `last_seen_timestamp_seconds` | 该设备 `device_tracker` 实体的上次更新时间 | `device`, `type`(`Mobile`)、`os`(空)、`user`(与之关联的人,如果有)、`kind` |
| `stale_seconds` | 当某设备超过 `device_stale_days`(默认 3 天)未更新时为正值;驱动[客户端设备陈旧](../alerting/rules.md#client-devices)规则 | `device`, `type`, `os`, `user`, `kind`, `signal`(始终为 `connection`:Home Assistant 没有备份可报告) |

**如何识别一台设备**:`/api/states` 不会说明某个实体是由哪个
集成创建的——这需要实体注册表,而 REST API 不提供这一信息。配套
应用的 `device_tracker` 是唯一会设置 `battery_level` 属性的;
第三方位置集成(Life360、OwnTracks、测试中用到的 demo 集成)最多
只会设置 `battery`。如果某个第三方集成恰好复制了同样的属性名,
就会被误认为是配套应用;反过来,一台关闭了电池上报的配套应用
设备会从表格中缺失,而不是被错误地显示。该设备所属的账户,
如果已知,来自某个 `person.*` 实体,其 `device_trackers` 属性
会指明它。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Home
Assistant。它会先说明哪里出了问题(恢复模式、错误级别的修复项、
不可用的实体),然后是按电量排列的低电量电池、附带版本号的待
处理更新、按域划分的实体,再然后是配套应用设备及其上次连接时间
的表格。

## 验证方式 {#how-it-was-validated}

针对 Home Assistant 2026.9.4,使用一个没有管理员权限的用户的长期
令牌验证:配置、demo 集成和模板传感器的实体状态(电池电量、一个
电池二元传感器、一个不可用传感器)、更新实体,以及通过 WebSocket
API 的修复项列表。配套应用的 `device_tracker` 属性
(`battery_level`、`source_type`)是依据 `mobile_app` 集成的源码
验证的,而不是针对一台真实的手机。

## 在 Home Assistant 中创建一个专用用户和长期令牌 {#create-a-dedicated-user-and-a-long-lived-token-in-home-assistant}

1. 在 Home Assistant 中:Settings → People → Users → Add user(如果没有 Users 标签,请先在你的个人资料中打开高级模式)。按如下方式命名该用户,设置一个较长的随机密码,保持 Administrator 开关关闭。普通用户能读取每一个实体和修复项列表,不能更改配置。

    ```
    dumbmonit
    ```

2. 以该用户身份登录 Home Assistant 一次,打开其个人资料(侧边栏底部的名称)→ Security → Long-lived access tokens → Create token。以 DumbMonit 命名它,并立即复制它:它只会显示一次。

3. 在 DumbMonit 中,输入 Home Assistant 的地址,例如 "homeassistant.local"(端口 8123),或反向代理之后的 "https://ha.example.net",并粘贴该令牌。低于 20% 的电池会被计为电量低;"低电量阈值"选项可以更改这个比例。

4. DumbMonit 只会读取 /api/config、/api/states,以及通过 WebSocket API 读取修复项列表。它从不调用任何服务,从不触发任何事件,也从不更改任何状态。

!!! warning "警告"
    长期令牌携带其用户的全部权限,并有效十年:请为专用用户创建
    它,绝不要用你自己的账户。Home Assistant 默认提供纯 HTTP
    服务,除非配置了 TLS,令牌会随每个请求传输:在不受信任的
    网络中,请使用 HTTPS。每一台登录了配套应用(mobile_app 集成)
    的手机和平板都会出现在下方的设备表格中,附带其上次活动时间;
    这是可选的,没有安装该应用时则不会有任何内容。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 长期访问令牌 | 该专用用户的令牌,以 `Authorization: Bearer` 发送。 |

地址:主机名或 IP(`homeassistant.local`)、`host:port`,或完整
URL(`https://ha.example.net`,带反向代理后的路径前缀);结尾的
`/api` 也会被接受。选项:协议(默认 HTTP)、端口(8123)、证书
校验、请求超时(10 秒)、低电量阈值(20%)、排除的域(默认无)、
修复项(读取),以及设备陈旧判定(`device_stale_days`,默认
3 天)。
