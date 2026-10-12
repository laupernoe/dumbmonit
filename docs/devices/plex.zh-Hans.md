# Plex Media Server

Plex 正在做什么,以及它是否是最新版本:当前正在播放的流、有多少
经过转码以及是否使用硬件转码、它们占用的带宽、来自外部网络的流、
正在扫描的媒体库,以及更新情况。

DumbMonit 读取服务器自身在 32400 端口上的 API,从不访问 plex.tv。
它从不停止流,从不扫描媒体库,也从不更改任何设置。

Plex 不会发布其维护任务的结果("butler" 只给出计划安排)以及
数据库备份的状态,因此这两方面没有相关指标。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_plex_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `update_available` | 当 Plex 自身的更新检查发现新版本时为 1;在 Plex 首次检查之前不存在 | `latest_version` |
| `claimed` | 当服务器已关联到某个 Plex 账户时为 1 | |
| `streams`, `streams_paused`, `streams_remote` | 当前播放数;已暂停的;来自本地网络之外的 | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | 被重新编码的播放数(视频或音频);视频被重新编码的;使用硬件加速的 | |
| `transcoder_active_video_sessions` | 服务器自身统计的视频转码数 | |
| `stream_bandwidth_bits_per_second` | 所有播放占用的带宽总和 | |
| `libraries`, `libraries_scanning` | 媒体库数量;当前正在扫描的数量 | |
| `activities` | 正在运行的后台活动(扫描、分析、更新) | |
| `scrape_errors` | 上次探测中失败的调用 | |

适用的[内置规则](../alerting/rules.md#self-hosted-applications):
Plex 有可用更新,以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会调用 Plex。
它会先给出更新状态以及服务器是否已被关联,然后是流、转码、
硬件转码、远程流、带宽和媒体库情况。

## 让 DumbMonit 访问 Plex 服务器 {#give-dumbmonit-access-to-the-plex-server}

1. DumbMonit 读取服务器自身在 32400 端口上的 API,从不访问 plex.tv。关联到某个 Plex 账户的服务器需要一个令牌(X-Plex-Token),而 Plex 无法发出受限的令牌:它就是账户所有者的令牌。

2. 要么避免使用令牌:在 Plex 中,Settings → Network → 允许无需认证访问的 IP 地址和网络列表,添加 DumbMonit 服务器的地址,例如如下所示,然后在下方选择"无令牌"。

    ```
    10.0.0.20/32
    ```

3. 要么使用令牌:在 Plex Web 中,打开任意一部电影 → ⋯ → Get Info → View XML。打开的页面地址末尾会带有 X-Plex-Token= 后跟令牌;复制该值。

4. 在 DumbMonit 中,输入服务器地址,例如 "plex.lan"(端口 32400)。DumbMonit 只读取会话、媒体库、更新检查器和活动列表;它从不停止流,也从不更改任何设置。

!!! warning "警告"
    该令牌属于你的 Plex 账户:持有它的人可以管理服务器以及每一个媒体库。当 DumbMonit 位于你自己的网络中时,优先使用上面的允许网络方式。

允许网络同时也是让 Plex 在不使用令牌的情况下从 Docker 应答的方式:
DumbMonit 的容器是从其 Docker 网络地址访问 Plex 的,因此请把那个
网络列入白名单,或者当 Plex 在主机上发布端口时,使用主机的地址。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| X-Plex-Token | 在 `X-Plex-Token` 请求头中发送。 |
| 无令牌(允许的网络) | 不发送任何内容;Plex 必须把 DumbMonit 的地址列入无需认证即可访问的网络。 |

地址:主机名(`plex.lan`)、`host:port`,或完整 URL;结尾的 `/web`
也会被接受。选项:协议(默认 HTTP;Plex 自身的证书是为 `*.plex.direct`
名称签发的,因此通过 IP 地址使用 HTTPS 需要关闭证书校验)、
端口(32400)、证书校验和请求超时(15 秒)。

已在 Plex Media Server 1.43.4(官方镜像)上验证,针对一个未关联账户、
无需令牌即可应答的服务器,并有一次真实的转码播放。令牌路径遵循
Plex 文档中记载的 `X-Plex-Token` 请求头,但未在已关联账户的服务器上
实际测试过。
