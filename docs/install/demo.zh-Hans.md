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
- **固定的虚构资产。** 启动时会重新创建数据库,并填入一个位于
  `*.home.arpa` 下的小型家庭实验室:一个 Proxmox VE 集群、一台
  Synology NAS、一个 Proxmox Backup Server、一台 TrueNAS、一个
  OPNsense 防火墙、一台存在故障部件的 Redfish 服务器、几个 HTTP、
  TLS 和 ping 检查、一台代理主机以及一个状态页面。这些监控项由
  服务器内部的伪响应器应答,绑定在 `127.0.0.1` 上并回放录制好的
  API 响应,因此真正的采集器和监控项面板都会照常运行。重启后得到
  同样的资产。
- **七天的历史数据。** 启动时会向 VictoriaMetrics 写入合成指标,
  因此图表、*最近 7 天*、预测和告警历史都有数据,并且有几条告警
  正在触发,使*需要处理*里有内容可看。
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
