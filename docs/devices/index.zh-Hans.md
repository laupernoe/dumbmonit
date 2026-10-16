# 监控项类型 {#device-types}

DumbMonit 把它监视的所有东西都称为*监控项*（API 说*目标*），即使是网页或 DNS 名称也是如此。每种类型都是服务器上的一个收集器；服务器向接口描述每种类型（`GET /api/collectors`），所以表单总是与这个版本能监视的东西相匹配。

| 类型 | `kind` | 凭证 | 典型用途 |
|---|---|---|---|
| [SNMP 设备](snmp.md) | `snmp` | 社区或 SNMP v3 | 交换机、路由器、NAS、UPS、打印机 |
| [Proxmox VE](proxmox.md) | `proxmox` | API 令牌或用户名/密码 | 虚拟化程序、群集 |
| [Proxmox 备份服务器](pbs.md) | `pbs` | API 令牌或用户名/密码 | 备份服务器 |
| [Proxmox 数据中心管理器](pdm.md) | `pdm` | API 令牌 | 联合多个 PVE 群集和 PBS 实例的控制台 |
| [Proxmox 邮件网关](pmg.md) | `pmg` | 用户名/密码 | 邮件网关，过滤垃圾邮件和病毒 |
| [VMware vSphere](vsphere.md) | `vsphere` | 用户名/密码 | vCenter 或独立 ESXi：主机、虚拟机、数据存储、警报 |
| [XCP-ng / XenServer](xcpng.md) | `xcpng` | 用户名/密码（只读 XAPI 角色） | 通过 XAPI 监控池：主机、虚拟机、存储库、高可用 |
| [Nutanix AHV](nutanix.md) | `nutanix` | 用户名/密码（Prism Viewer） | Prism Central v4 API：集群、主机、AHV 虚拟机、存储容器、告警 |
| [Synology DSM](synology.md) | `synology` | 用户名/密码 | DiskStation、RackStation |
| [OPNsense](opnsense.md) | `opnsense` | API 密钥和密钥 | 防火墙、路由器、多 WAN 边缘 |
| [pfSense](pfsense.md) | `pfsense` | REST API 密钥（仅读用户） | 防火墙：网关掉线或有损、没有链接的接口、停止的服务 |
| [FortiGate](fortigate.md) | `fortigate` | REST API 令牌（仅读配置文件、受信主机） | 防火墙：IPsec 隧道、链接、HA 同步、保守模式、FortiGuard 许可证 |
| [Sophos 防火墙](sophos.md) | `sophos` | 用户名/密码（仅读配置文件） | 防火墙：仅接口链接（XML API 不公开隧道、HA 或许可证状态） |
| [OpenVPN 服务器](openvpn.md) | `openvpn` | 无，或管理密码 | 通过其管理接口的 OpenVPN 服务器：版本、状态、已连接的客户端 |
| [UniFi 网络](unifi.md) | `unifi` | API 密钥或用户名/密码 | UniFi 控制台或自托管服务器：监控项、WAN、客户端 |
| [Home Assistant](homeassistant.md) | `homeassistant` | 长期有效的访问令牌 | 智能家居：不可用的实体、低电池、更新、修复 |
| [TrueNAS](truenas.md) | `truenas` | API 密钥 | ZFS 存储服务器（SCALE、社区版） |
| [Unraid](unraid.md) | `unraid` | API 密钥（查看者角色） | NAS：阵列、磁盘、奇偶校验检查、缓存池、容器、虚拟机 |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | 用户名/密码（仅读组） | 路由器或交换机：版本和固件落后、CPU、内存、传感器、接口错误 |
| [NUT 的 UPS](nut.md) | `nut` | 无，或用户名/密码 | NAS、Raspberry Pi 或运行网络 UPS 工具的服务器上的 UPS |
| [服务器硬件（Redfish）](redfish.md) | `redfish` | 用户名/密码 | 服务器风扇、温度、电源和驱动器，从其 BMC 读取 |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | 无、用户名/密码或令牌 | 时间序列数据库：摄取、拒绝的样本、磁盘空间 |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | 无、用户名/密码或令牌 | 日志数据库：摄取、拒绝的行、磁盘空间 |
| [Grafana Loki](loki.md) | `loki` | 无、用户名/密码或令牌 | 日志服务器：就绪状态、拒绝的行、刷新失败 |
| [Graylog](graylog.md) | `graylog` | 访问令牌或用户名/密码 | 日志服务器及其 OpenSearch 群集：日志、缓冲区、吞吐量、输入 |
| [Pi-hole](pihole.md) | `pihole` | 应用密码，或无 | DNS 广告拦截器：阻止状态、阻止份额、阻止列表年龄、更新 |
| [AdGuard Home](adguard.md) | `adguard` | 用户名/密码 | DNS 过滤器：保护、阻止、过滤列表、上游服务器、更新 |
| [MDaemon 邮件服务器](mdaemon.md) | `mdaemon` | 无，或电子邮件地址/密码 | 邮件服务器：SMTP、IMAP、POP3、网络邮件、版本 |
| [电子邮件服务器安全网关](securitygateway.md) | `securitygateway` | 无，或 API 密钥 | 邮件网关：服务、投递队列和其他计数 |
| [Redis / Valkey](redis.md) | `redis` | ACL 用户、仅密码或无 | 内存存储：针对最大内存的内存、复制、持久性、密钥 |
| [MongoDB](mongodb.md) | `mongodb` | 用户名/密码（clusterMonitor）或无 | 文档数据库：副本集、滞后、连接、WiredTiger 缓存 |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | 用户名/密码（监视标签） | 消息代理：资源警报、节点、分区、无消费者的队列 |
| [CrowdSec](crowdsec.md) | `crowdsec` | 无，或 Bouncer API 密钥 | 安全引擎：决策、警报、停止拉取的 Bouncer、日志读取 |
| [Traefik](traefik.md) | `traefik` | 无，或 basicAuth 用户 | 反向代理：禁用的路由器、后端服务器掉线、证书未获得或未更新、5xx 份额 |
| [Caddy](caddy.md) | `caddy` | 仅读视图用户，或无 | Web 服务器和反向代理：上游健康、失败重载、处理程序错误、5xx 份额 |
| [Nginx](nginx.md) | `nginx` | 无，或基本身份验证 | Web 服务器：活跃/等待连接、接受/处理/请求率、NGINX Plus 上游健康 |
| [Apache httpd](apache.md) | `apache` | 无，或基本身份验证 | Web 服务器：繁忙/空闲工作程序、记分板按状态、请求和字节率 |
| [Nginx 代理管理器](npm.md) | `npm` | 用户电子邮件和密码（仅查看） | 代理管理器：nginx 拒绝的主机、禁用的主机、提供的证书 |
| [域名过期](domain.md) | `domain` | 无 | 通过 RDAP 的注册域名：过期前天数、保留、赎回期、注册商 |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | 服务账户令牌（仅读 ClusterRole） | 集群：节点不就绪、pod 崩溃循环或待处理、工作负载缺少副本、卡住的卷声明 |
| [Active Directory](activedirectory.md) | `activedirectory` | 用户名/密码（普通域用户） | Windows 域通过 LDAPS：域控制器和 FSMO 角色、特权组成员、密码策略、安全发现 |
| [Veeam Backup & Replication](veeam.md) | `veeam` | 用户名/密码（Veeam Backup Viewer） | 备份服务器：失败的任务、失败的会话、存储库填满、许可证 |
| [Tailscale](tailscale.md) | `tailscale` | OAuth 客户端（devices:core:read）或 API 访问令牌 | Tailnet：服务器离线、节点密钥即将过期、监控项等待批准 |
| [Nextcloud](nextcloud.md) | `nextcloud` | 监视令牌（NC-Token） | 文件同步：维护模式、待处理升级、更新、活跃用户、OPcache |
| [Immich](immich.md) | `immich` | API 密钥 | 照片库：任务队列、更新、磁盘、照片计数 |
| [Paperless-ngx](paperless.md) | `paperless` | API 令牌或用户名/密码 | 文档：Redis、Celery、索引、失败的导入、更新 |
| [Jellyfin](jellyfin.md) | `jellyfin` | API 密钥 | 媒体服务器：失败的计划任务、插件、流和转码 |
| [Plex 媒体服务器](plex.md) | `plex` | X-Plex-Token，或来自允许网络的无 | 媒体服务器：流、转码、库、更新 |
| [GitLab（自托管）](gitlab.md) | `gitlab` | 个人访问令牌 | 锻造：Sidekiq 积压、CI 运行器、待处理迁移、注册/2FA 设置、许可证 |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | 访问令牌 | 锻造：健康检查、计划任务、Actions 运行器、存储库/账户计数 |
| [带代理的服务器](agent.md) | `agent` | 无（注册令牌） | Linux、Windows、Raspberry Pi、[Hyper-V 主机](agent.md#hyper-v-hosts) |
| [网站或 Web API](services.md#http) | `http` | 无、用户名/密码或令牌 | 健康页面、REST API |
| [网络端口](services.md#tcp) | `tcp` | 无 | SSH、SMB、数据库 |
| [域名](services.md#dns) | `dns` | 无 | 您的域名、内部名称 |
| [可达主机](services.md#ping) | `ping` | 无 | 网关、接入点、打印机 |
| [TLS 证书](services.md#tls) | `tls` | 无 | IMAPS、LDAPS、反向代理 |
| [SMTP 服务器](services.md#smtp) | `smtp` | 无，或用户名/密码 | 邮件服务器：问候、STARTTLS、证书、可选登录 |
| [PostgreSQL](services.md#postgresql) | `postgres` | 用户名/密码 | 数据库：连接和您选择的查询 |
| [MySQL 或 MariaDB](services.md#mysql-and-mariadb) | `mysql` | 用户名/密码 | 数据库：连接和您选择的查询 |
| [MQTT 代理](services.md#mqtt) | `mqtt` | 无，或用户名/密码 | 代理：连接、订阅、保留的消息 |
| [WebSocket](services.md#websocket) | `websocket` | 无、用户名/密码或令牌 | WebSocket 端点：握手、可选消息交换 |
| [NTP 时间服务器](services.md#ntp) | `ntp` | 无 | 路由器、NAS、内部时间服务器、公开池服务器 |
| [心跳](push.md) | `push` | 无（秘密 URL） | Cron 任务、备份脚本、必须调用的自动化 |
| [网站变化](webchange.md) | `webchange` | 无 | 一个页面或整个网站：文本变化、新增和删除的页面、之前/之后带屏幕截图 |
| [网站健康和 SEO](website.md) | `website` | 无 | 批量网站：掉线、空白页、重定向、noindex、robots.txt、标题/描述/规范变化、平台（CSV 导入） |
| [演示监控项](demo.md) | `dummy` | 无 | 无硬件的情况下探索 UI |

## 监控项如何被读取 {#how-a-device-is-read}

调度程序在其自己的间隔（默认 60 秒，最小 10 秒）运行每个启用的监控项，通过其类型的收集器，最多 `DUMBMONIT_MAX_CONCURRENT_PROBES` 个在途探测，以及每个探测硬超时 `DUMBMONIT_PROBE_TIMEOUT_SECS`。每个成功的探测写其样本加 `dumbmonit_up = 1`；失败的探测什么都不写，它是"监控项不可达"规则检测的系列的沉默。监控项页面在下一次探测时显示故障，但警报故意等待：在最后一个成功探测后约四分钟触发（三分钟的沉默，然后一分钟的保留期），所以单个丢失的探测永远不会激活任何人。

故障是分类的。不回应的监控项是*不可达的*并引发警报。错误的社区、被拒绝的证书或缺少的能力是*配置错误*：它显示在监控项页面上，用红色，带有原因，它不被通知。区别很重要：配置错误的监控项不应在半夜唤醒任何人。

## 选项 {#options}

一些类型从监控项的标签中读取选项：API 端口、是否接受自签名证书、要监视哪些节点。表单在"更多选项"下为每个选项显示一个类型化字段，及其默认值和帮助文本。按类型的页面逐字列出它们。选项是设置，不是标签：它们不复制到系列上。自由标签（任何其他密钥）在每个系列上复制为 `tag_<key>` 标签，除了命名密钥的密钥（按单词匹配：`password`、`token`、`key`、`cert`、`secret`、`pass`、`private`、`pem` 等，所以 `primary_key` 被跳过但 `keyboard` 不是）和长于 256 个字符的值。永远不要在标签中放置密钥：使用凭证字段，这是加密的。

## 父监控项 {#parent-devices}

任何监控项都可以声明另一个监控项为其父监控项。当父监控项不可达时，其子监控项的警报被标记为*被父监控项抑制*而不是被发送：交换机掉线产生一个通知，而不是三十个。在监控项页面上，子监控项堆叠在其父监控项下，当它不可达时变暗。
