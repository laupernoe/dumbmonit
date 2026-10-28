# 运行一个演示实例 {#run-a-demo-instance}

**[demo.dumbmonit.app](https://demo.dumbmonit.app)** 上的公开演示,是
一个以 `DUMBMONIT_DEMO=1` 启动的普通 DumbMonit 镜像。你也可以自己运行
同样的东西——在真正安装之前向团队展示 DumbMonit,或者在自己的网站上
放一个展示页。

!!! warning "切勿用于真实数据卷"
    演示模式会**在每次启动时重新创建数据库**。请给它一个独立的卷,
    或者完全不用卷:如果把它指向某个真实实例的 `/data`,它会把你的
    监控项、规则和历史记录替换成虚构的资产。

## 演示模式的行为 {#what-demo-mode-does}

- **使用 `demo` / `demo` 登录。** 登录页面会说明这一点,并一键填好
  这两个字段。该账户能看到管理员能看到的每一个界面。
- **只读。** 每一次更改——监控项、规则、静默、确认、渠道、令牌、
  用户、设置、备份与恢复、发现扫描、代理注册、助手接口的写入
  工具——都会被服务器以 `403` 拒绝,并返回
  `{"error": "This is a read-only demo…", "demo": true}`。登录和登出是
  唯一被允许的写操作。界面会在点击发生的位置显示该提示。
- **两个虚构客户。** 启动时会重新创建数据库,并填入一个位于 `*.home.arpa`
  下的家庭实验室——Proxmox VE、带一个 iSCSI LUN 的 Synology NAS、Proxmox
  Backup Server、TrueNAS、OPNsense、UniFi、Kopia、一台有部件故障的 Redfish
  服务器、一台 SNMP 交换机、HTTP、TLS、DNS 和 ping 检查、一个运行 Docker 的
  Linux 代理——并在 MSP 模式下加入日内瓦的一家小型建筑事务所:一台带代理的
  Windows 域控制器、Veeam Backup & Replication、Omada 网络、Microsoft 365 和
  Entra ID,均通过放在办公室里的中继代理轮询。监控项由服务器内部的模拟设备
  应答,它们绑定在 `127.0.0.1` 上并回放录制的 API 响应,因此真实的采集器和
  面板都会运行。重启后资产保持不变。
- **三十天的历史数据,以及产品的其余部分。** 启动时会分块向
  VictoriaMetrics 写入合成指标(最近一周为五分钟精度):图表、*最近 7 天*、
  30 天可用率、预测、备份矩阵和告警历史都有数据,并有几条告警正在触发。
  种子数据还会填好通常需要手动设置的内容:团队和文件夹、账户和值班排班及
  升级策略、静默、操作手册、两个仪表板、定时报告、通知渠道(从不使用)、
  世界地图上的站点、已知漏洞、四个状态页(每个夜景一个)及一个进行中的事件,
  以及一次发现了新设备的网络扫描。一切都由固定种子计算得出:不会从网络获取
  任何内容,连漏洞来源也不会。日志搜索保持关闭:演示没有日志存储。
- **不会离开服务器。** 不会发送任何通知,也不会对访客输入的
  地址发起任何探测。
- **引导式导览**会在首次登录后打开;横幅可重新打开它。

登录的速率限制照常生效。请把演示实例放在你平常使用的、带有
HTTPS 的反向代理之后,并像对待任何公开实例一样设置
`DUMBMONIT_COOKIE_SECURE=1` 和 `DUMBMONIT_TRUSTED_PROXIES`。

## 启动它 {#start-it}

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

或者使用 Compose,不挂载任何卷,这样每次重启都是干净的:

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/laupernoe/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

定期拉取镜像(或者让 Watchtower 之类的工具来做这件事):这样
演示实例就会始终展示最新发行版和相同的数据。
