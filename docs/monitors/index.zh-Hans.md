# 监控项

DumbMonit 监控的一切,以四种方式呈现。在应用中是左侧菜单的**监控项**(`g d`),有四个标签页。**添加监控项**是“列表”标签页的主按钮(`g n`)。

| 标签页 | 您看到什么 | 阅读 |
|---|---|---|
| **列表** | 每个监控项及其状态、筛选、文件夹和批量操作;点击即可打开该监控项 | [监控项页面](../using/devices.md) |
| **地图** | 谁依赖谁(`g m`),以及您的站点在世界地图上的分布 | [拓扑图](../using/topology.md)、[世界地图](../using/world-map.md) |
| **日志** | 一次搜索覆盖服务器、代理、syslog 和 SNMP trap 的日志(`g l`,管理员) | [日志](../using/logs.md) |
| **安全** | DumbMonit 拦截的攻击来自何处(威胁地图),以及您监控项上软件的已知漏洞(CVE)(`g x`);安全评分在设置中开启 | [CVE 与漏洞](../using/cves.md)、[安全评分](../using/security-score.md) |

指标浏览器(`Ctrl+K`,然后选择 *Explore*)不是标签页:它浏览 `dumbmonit_*` 指标并可试运行查询([指标浏览器](../using/dashboards.md))。

## 添加监控项

- 逐个添加,使用**添加监控项**:[添加第一个监控项](../install/first-device.md)。
- 扫描网络:[网络发现](../using/discovery.md)。
- 根据容器标签:[通过 Docker 标签添加监控项](../using/docker-labels.md)。
- 从其他工具(Uptime Kuma、Healthchecks、CSV):[从其他工具导入](../using/imports.md)。
- 从保存在 Git 中的文件:[配置文件(GitOps)](../using/config-files.md)。

## 集成

每种监控项类型都有自己的页面,并在左侧菜单中按类别分组:服务器与代理、虚拟化、存储与 NAS、网络与防火墙、硬件、数据库、Web 服务器、身份、云与 SaaS、家庭与媒体、自托管应用、备份、电子邮件、可观测性和互联网检查。[所有集成](../devices/index.md)的表格列出了它们以及每种所需的条件。
