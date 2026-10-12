# 设备类型 {#device-types}

DumbMonit 把它监控的一切都称为*设备*(API 中称为 *target*),即使
它其实是一个网页或一个 DNS 名称。每种类型在服务器上对应一个
采集器;服务器会向界面描述每种类型(`GET /api/collectors`),因此
表单始终与当前版本能够监控的内容保持一致。

| 类型 | `kind` | 凭据 | 典型用途 |
|---|---|---|---|
| [SNMP 设备](snmp.md) | `snmp` | 社区名或 SNMP v3 | 交换机、路由器、NAS、UPS、打印机 |
| [Proxmox VE](proxmox.md) | `proxmox` | API 令牌或用户名/密码 | 虚拟化平台、集群 |
| [Proxmox Backup Server](pbs.md) | `pbs` | API 令牌或用户名/密码 | 备份服务器 |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | API 令牌 | 联合管理多个 PVE 集群和 PBS 实例的控制台 |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | 用户名/密码 | 过滤垃圾邮件和病毒的邮件网关 |
| [VMware vSphere](vsphere.md) | `vsphere` | 用户名/密码 | vCenter 或独立 ESXi:主机、虚拟机、数据存储、告警 |
| [Synology DSM](synology.md) | `synology` | 用户名/密码 | DiskStation、RackStation |
| [OPNsense](opnsense.md) | `opnsense` | API 密钥和密钥口令 | 防火墙、路由器、多 WAN 边缘设备 |
| [pfSense](pfsense.md) | `pfsense` | REST API 密钥(只读用户) | 防火墙:网关宕机或丢包、接口无链路、服务停止 |
| [FortiGate](fortigate.md) | `fortigate` | REST API 令牌(只读配置档案、受信主机) | 防火墙:IPsec 隧道、链路、HA 同步、节能模式、FortiGuard 许可证 |
| [Sophos Firewall](sophos.md) | `sophos` | 用户名/密码(只读配置档案) | 防火墙:仅接口链路(该 XML API 不暴露隧道、HA 或许可证状态) |
| [UniFi Network](unifi.md) | `unifi` | API 密钥或用户名/密码 | UniFi 控制台或自托管服务器:设备、WAN、客户端 |
| [Home Assistant](homeassistant.md) | `homeassistant` | 长期有效的访问令牌 | 智能家居:不可用的实体、电池电量低、更新、修复项 |
| [TrueNAS](truenas.md) | `truenas` | API 密钥 | ZFS 存储服务器(SCALE,社区版) |
| [Unraid](unraid.md) | `unraid` | API 密钥(Viewer 角色) | NAS:阵列、磁盘、校验检查、缓存池、容器、虚拟机 |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | 用户名/密码(只读分组) | 路由器或交换机:版本和固件落后情况、CPU、内存、传感器、接口错误 |
| [UPS with NUT](nut.md) | `nut` | 无,或用户名/密码 | 接在 NAS、Raspberry Pi 或服务器上、运行 Network UPS Tools 的 UPS |
| [服务器硬件(Redfish)](redfish.md) | `redfish` | 用户名/密码 | 服务器风扇、温度、电源和驱动器,从其 BMC 读取 |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | 无、用户名/密码或令牌 | 时间序列数据库:写入、拒绝的样本、磁盘剩余空间 |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | 无、用户名/密码或令牌 | 日志数据库:写入、拒绝的行、磁盘剩余空间 |
| [Grafana Loki](loki.md) | `loki` | 无、用户名/密码或令牌 | 日志服务器:就绪状态、拒绝的行、写入失败 |
| [Graylog](graylog.md) | `graylog` | 访问令牌或用户名/密码 | 日志服务器及其 OpenSearch 集群:日志队列、缓冲区、吞吐量、输入 |
| [Pi-hole](pihole.md) | `pihole` | 应用密码,或无 | DNS 广告拦截器:拦截状态、拦截比例、黑名单年龄、更新 |
| [AdGuard Home](adguard.md) | `adguard` | 用户名/密码 | DNS 过滤器:保护状态、拦截、过滤列表、上游服务器、更新 |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | 无,或邮箱地址/密码 | 邮件服务器:SMTP、IMAP、POP3、网页邮件、版本 |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | 无,或 API 密钥 | 邮件网关:服务、投递队列及其他计数器 |
| [Redis / Valkey](redis.md) | `redis` | ACL 用户、仅密码,或无 | 内存存储:内存相对于 maxmemory、复制、持久化、键 |
| [MongoDB](mongodb.md) | `mongodb` | 用户名/密码(clusterMonitor)或无 | 文档数据库:副本集、延迟、连接数、WiredTiger 缓存 |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | 用户名/密码(monitoring 标记) | 消息中间件:资源告警、节点、分区、无消费者的队列 |
| [CrowdSec](crowdsec.md) | `crowdsec` | 无,或 bouncer API 密钥 | 安全引擎:决策、告警、停止拉取的 bouncer、日志读取 |
| [Traefik](traefik.md) | `traefik` | 无,或 basicAuth 用户 | 反向代理:已禁用的路由器、后端服务器宕机、未获取或未续签的证书、5xx 比例 |
| [Caddy](caddy.md) | `caddy` | 只读查看用户,或无 | Web 服务器和反向代理:上游健康状况、重载失败、处理器错误、5xx 比例 |
| [Nginx](nginx.md) | `nginx` | 无,或基本认证 | Web 服务器:活动/等待连接、接受/处理/请求速率、NGINX Plus 上游健康状况 |
| [Apache httpd](apache.md) | `apache` | 无,或基本认证 | Web 服务器:忙碌/空闲工作进程、按状态的记分板、请求和字节速率 |
| [Nginx Proxy Manager](npm.md) | `npm` | 用户邮箱和密码(仅查看) | 代理管理器:被 nginx 拒绝的主机、已禁用的主机、出示的证书 |
| [域名到期](domain.md) | `domain` | 无 | 通过 RDAP 监控已注册域名:到期前剩余天数、保留状态、赎回期、注册商 |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | 服务账户令牌(只读 ClusterRole) | 集群:未就绪的节点、崩溃循环或挂起的 Pod、副本数不足的工作负载、卡住的卷声明 |
| [Active Directory](activedirectory.md) | `activedirectory` | 用户名/密码(普通域用户) | 通过 LDAPS 监控 Windows 域:域控制器和 FSMO 角色、特权组成员、密码策略、安全发现项 |
| [Veeam Backup & Replication](veeam.md) | `veeam` | 用户名/密码(Veeam Backup Viewer) | 备份服务器:失败的任务、失败的会话、存储库空间不足、许可证 |
| [Tailscale](tailscale.md) | `tailscale` | OAuth 客户端(devices:core:read)或 API 访问令牌 | Tailnet:离线的服务器、即将过期的节点密钥、等待批准的设备 |
| [Nextcloud](nextcloud.md) | `nextcloud` | 监控令牌(NC-Token) | 文件同步:维护模式、待处理的升级、更新、活跃用户、OPcache |
| [Immich](immich.md) | `immich` | API 密钥 | 照片库:任务队列、更新、磁盘、照片数量 |
| [Paperless-ngx](paperless.md) | `paperless` | API 令牌或用户名/密码 | 文档管理:Redis、Celery、索引、失败的导入、更新 |
| [Jellyfin](jellyfin.md) | `jellyfin` | API 密钥 | 媒体服务器:失败的计划任务、插件、流和转码 |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token,或来自允许网络时无需凭据 | 媒体服务器:流、转码、媒体库、更新 |
| [GitLab(自托管)](gitlab.md) | `gitlab` | 个人访问令牌 | 代码托管平台:Sidekiq 积压、CI runner、待处理的迁移、注册/两步验证设置、许可证 |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | 访问令牌 | 代码托管平台:健康检查、计划任务、Actions runner、仓库/账户计数 |
| [带代理的服务器](agent.md) | `agent` | 无(注册令牌) | Linux、Windows、Raspberry Pi、[Hyper-V 主机](agent.md#hyper-v-hosts) |
| [网站或 Web API](services.md#http) | `http` | 无、用户名/密码或令牌 | 健康检查页面、REST API |
| [网络端口](services.md#tcp) | `tcp` | 无 | SSH、SMB、数据库 |
| [域名](services.md#dns) | `dns` | 无 | 你的域名、内部名称 |
| [可达主机](services.md#ping) | `ping` | 无 | 网关、接入点、打印机 |
| [TLS 证书](services.md#tls) | `tls` | 无 | IMAPS、LDAPS、反向代理 |
| [NTP 时间服务器](services.md#ntp) | `ntp` | 无 | 路由器、NAS、内部时间服务器、公共时间池服务器 |
| [心跳](push.md) | `push` | 无(密钥 URL) | cron 任务、备份脚本、必须定时上报的自动化任务 |
| [网站变化](webchange.md) | `webchange` | 无 | 单个页面或整个网站:文本变化、新增和移除的页面、带截图的前后对比 |
| [演示设备](demo.md) | `dummy` | 无 | 在没有硬件的情况下体验界面 |

## 设备如何被读取 {#how-a-device-is-read}

调度器会按照各自的间隔(默认 60 秒,最小 10 秒)运行每个已启用的
设备,通过对应类型的采集器,同时最多 `DUMBMONIT_MAX_CONCURRENT_PROBES`
个探测并发执行,每次探测有 `DUMBMONIT_PROBE_TIMEOUT_SECS` 的硬性
超时。每次成功的探测都会写入其样本数据以及 `dumbmonit_up = 1`;
失败的探测什么都不会写入,"设备不可达"规则正是通过该序列的
沉默来检测的。

故障会被分类。没有应答的设备属于*不可达*,会触发告警。错误的
社区名、被拒绝的证书或缺失的能力属于*配置错误*:会在设备页面上
以红色显示原因,且不会发送通知。这个区分很重要:一个配置错误的
设备不应该在半夜把任何人吵醒。

## 选项 {#options}

有些类型会从设备的标签中读取选项:API 端口、是否接受自签名证书、
要监控哪些节点。表单会在**更多选项**下为每个选项显示一个带类型
的字段,附上其默认值和帮助文字。各类型的专属页面会逐一列出它们。
选项会作为 `tag_<key>` 标签复制到每个序列上,因此绝不要把密钥
放在选项里:请使用凭据字段,它是加密存储的。

## 父设备 {#parent-devices}

任何设备都可以把另一个设备声明为其父设备。当父设备不可达时,
其所有后代产生的告警会被标记为*被父设备抑制*,而不会被发送:
一台交换机宕机只产生一条通知,而不是三十条。在设备页面上,子设备
会堆叠在其父设备下方,并在父设备不可达时变暗。
