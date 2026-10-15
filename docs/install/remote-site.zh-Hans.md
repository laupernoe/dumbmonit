# 监控远程网站 {#monitor-a-remote-site}

一个DumbMonit，几个网络。一个在远程网站安装的**中继代理**代表服务器运行你分配给它的监控项探针，并通过它已经用于推送自己指标的相同出站连接报告结果。在远程网站没有要打开的东西、没有VPN、没有第二个实例要查看。

```
总部                                 远程网站（NAT、防火墙）
┌──────────────────┐   HTTPS、出站   ┌──────────────────────────────┐
│ DumbMonit服务器  │◀────────────────│ 中继代理                     │
│  调度器          │  要运行的探针──▶ │  运行SNMP/Proxmox/HTTP等   │──▶ 交换机、NAS、
│  警报、UI        │◀── 样本、结论   │  推送其自己的指标            │    管理程序...
└──────────────────┘                 └──────────────────────────────┘
```

## 1. 通过HTTPS发布服务器 {#1-publish-the-server-over-https}

中继从服务器获取其作业：服务器必须从远程网站可到达，通常 `https://monitor.example.org` 在反向代理后。两件事在该链接上传导：

- **注册令牌** (`dmon_…`)，在每个请求的 `Authorization` 标头中——对任何代理如此；
- **中继探针的监控项凭证**（SNMP社区、Proxmox API令牌…），解密，在每个探针作业内。

通过互联网的纯HTTP两者都会在途中被读取：在中继任何东西在你自己的网络外之前将服务器放在HTTPS后。在LAN或VPN上HTTP可接受，令牌是在你分配监控项给中继前唯一在途的密钥。凭证从不写到任何一方上的磁盘：探针作业生活在服务器的内存中直到中继报告它，代理在两个探针间保留无。

反向代理必须转发 `/api/agent/relay` 如 `/api/ingest`，允许请求体最多16MB（一个忙碌的管理程序报告数千样本），并保留闲置连接打开至少60秒：服务器持有中继请求最多25秒等待工作（长轮询）。

## 2. 创建注册令牌 {#2-create-an-enrollment-token}

**设置→代理→新令牌**，或添加一个代理类型的监控项。复制 `dmon_…` 值：它仅显示一次。

## 3. 在远程网站启动中继 {#3-start-the-relay-at-the-remote-site}

任何在远程网站有Docker的机器，使用来自仓库的 `docker-compose.agent.yml`：

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

或用一行安装器安装的常规代理，在 `/etc/dumbmonit/agent.yaml` 中有两个额外的密钥：

```yaml
relay: true
site: Lyon office
```

机器在几秒内出现在DumbMonit中，作为常规代理监控项，其页面说*中继·网站Lyon办公室·中继0监控项*。代理的日志说`中继模式启用：等待来自服务器的探针`一旦其第一个批被接受。

!!! note "中继需要到达什么"
    中继从其自己的网络探针监控项：它需要相同的访问服务器在那里需要的——SNMP的UDP 161、Proxmox的8006、DSM的5001，等等。对于**ping**监控，容器需要 `net.ipv4.ping_group_range` sysctl（compose文件中已设置，见[无需 capability 的 ICMP ping](docker.md#icmp-ping-without-a-capability)）。对于VLAN或链路本地网络上的监控项，或对于 `localhost` 目标，使用 `network_mode: host`。

## 4. 分配监控项给中继 {#4-assign-devices-to-the-relay}

如常添加远程监控项——SNMP交换机、Proxmox节点、HTTP检查——用其地址**如从远程网站看的** (`192.168.10.1`、`https://pve.lan:8006`)。在监控项表中，在*更多选项*下，设置**通过...到达**给中继代理而非*直接*。现有监控项可以以相同方式切换；切换回*直接*让服务器再次轮询它们。

从那时起：

- 服务器将探针排队给监控项在每个间隔；中继在几毫秒内取上它（它在服务器上等待工作），运行收集器并报告；
- **立即探针**和监控项页面上的文件检测通过中继进行；
- 监控项的*最后探针*和错误跟随中继的结论。当中继没有及时取起一个探针时，监控项显示*超时：中继代理…未取起探针*——检查代理用 `relay: true` 运行；
- 代理的页面显示*N个监控项的中继*，`GET /api/relays` 列出中继及其网站和计数。

## 警报当中继断掉 {#alerting-when-the-relay-is-down}

一个停止讲话的中继带上其监控项：它们的探针停止，它们都会立即成为*无法到达*。中继被对待作为每个监控项的**父级**它探针（除了你可能已设置的父级）：当中继本身*无法到达*时，其监控项的警报显示为 `suppressed` 有 `suppressed_by` = 中继，只有一个通知出去——对中继。你明确设置的父级保留其角色并赢当两者断掉。

## 限制 {#limits}

- 每个监控项一个中继。由中继探针的监控项也不被服务器探针。
- 代理监控项本身无法被中继：它们推送自己的指标。
- 中继最多同时运行8个探针；一个探针由服务器的探针超时绑定 (`DUMBMONIT_PROBE_TIMEOUT_SECS`)，在代理上覆盖最多120秒。
- 一个中继代理需要编译中的收集器：比该功能更旧的代理忽略 `relay: true`，服务器报告其探针超时。用当前二进制或镜像重新安装。
