# DumbMonit {#dumbmonit}

从家庭实验室到小型企业的简单监控。一个容器，一个 IP 地址，
不到一分钟就能看到有用的图表和告警。

!!! warning "开发中"
    DumbMonit 正在积极开发中；当前镜像
    （`ghcr.io/laupernoe/dumbmonit:latest`）是面向早期测试者的 alpha 版本。
    在首个正式版发布之前，可能存在不完善之处和不兼容的变更。

DumbMonit 像**天气预报**一样解读网络：首页用一句话说明
当前的“天空”状况（“晴空万里。”或“2 条提示，1 台无法连接。”），
并在最前面列出需要你处理的事项。严重程度遵循气象分级
（info → advisory → warning），预测就是预报，
维护窗口则是预先安排好的。

![概览页面：天气预报式的一句话、“Needs you”列表和预报](assets/screenshots/overview-light.png){ loading=lazy }

<div class="dm-links" markdown>
<a href="install/docker/">安装<small>Docker Compose、首次启动、备份</small></a>
<a href="install/first-device/">添加第一台设备<small>SNMP、网络扫描、接下来会发生什么</small></a>
<a href="alerting/">告警<small>内置规则，天生安静</small></a>
</div>

## 监控对象 {#what-it-watches}

| 来源 | 你能得到什么 |
|---|---|
| [SNMP v1 / v2c / v3](devices/snmp.md) | 交换机、路由器、NAS、UPS、打印机。产品内置五个配置档案，并根据设备的 `sysObjectID` 自动应用。网络扫描可一次性添加所有有响应的设备。 |
| [Proxmox VE](devices/proxmox.md) | 节点、虚拟机和容器、存储、集群法定人数，以及每台机器最近一次成功备份的时间。 |
| [Proxmox Backup Server](devices/pbs.md) | 数据存储用量及容量耗尽预测、去重、每台机器最近一次快照的时间和校验情况、失败的任务、垃圾回收。 |
| [Synology DSM](devices/synology.md) | 通过 NAS 的 Web API 获取存储卷、磁盘及其 SMART 健康状态、温度、负载。 |
| [Linux、macOS、FreeBSD 和 Windows 代理](devices/agent.md) | 不支持 SNMP 的机器的 CPU、内存、磁盘、网络、服务、容器和运行时间，以及机器能提供的温度、磁盘健康状态（SMART）和 ZFS 存储池。一条命令即可安装，也可使用 Docker 镜像；在[中继模式](install/remote-site.md)下，它只通过出站连接探测远程站点自己的设备。 |
| [服务](devices/services.md) | HTTP(S)、TCP 端口、DNS、ping 和 TLS 证书到期，Uptime Kuma 风格，带历史条和可用率百分比。 |
| [集成包](packs/index.md) | 用单个 YAML 文件声明一种设备类型——设备上的 HTTP API 或 Prometheus `/metrics` 页面——并附带自己的告警规则，无需服务器发布新版本即可安装。 |

## 与它对话 {#talk-to-it}

- 一个[基于 MCP 的助手](using/assistant.md)——Claude Code、Claude Desktop、
  ChatGPT、VS Code、Cursor——提供 27 个工具：`read` 令牌只能查看（状态、
  设备、告警、指标、代理、容器……），`write` 令牌还能执行操作
  （静默、确认、添加设备、重启容器、发布状态页事件）。
  密钥绝不会被返回。
- UI 所有操作背后的 [HTTP API](reference/api.md)，描述见
  `/api/openapi.json`（OpenAPI 3.1）。令牌（`dmt_…`）区分读写范围，
  可设置过期时间、限制在指定网络列表内，并有
  速率限制。

## 运行内容 {#what-runs}

| 容器 | 作用 | 资源占用 |
|---|---|---|
| `dumbmonit` | 采集、API、告警、Web UI，以及用于时间序列的内嵌 VictoriaMetrics | 约 40 MB 内存 + VictoriaMetrics 的预算（默认 256 MB） |

一个容器，一个卷：服务器从同一个
镜像中启动 VictoriaMetrics，配置和状态存放在内嵌的 SQLite 数据库中。
没有数据库容器。也可以改用外部的 VictoriaMetrics
（`DUMBMONIT_VM_URL`）。

!!! note "关于名称"
    DumbMonit 在 2026 年 9 月之前叫作 EzyMonit。命令、环境
    变量、镜像名称和路径都随之重命名；旧的
    `EZYMONIT_*` 变量和 `ezym_` 代理令牌仍然可用。参见
    [升级](install/docker.md#upgrading)。

DumbMonit 是 100% 开源软件，采用 Apache 2.0 许可证，包括所有依赖项：
没有任何功能被留给付费版本。
