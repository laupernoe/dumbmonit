# 常见问题 {#faq}

## 我忘记了密码 {#i-lost-the-password}

先问一下另一位管理员:在设置 → **用户**中,对你的账户点击
**编辑**可以设置一个新密码并显示一次,同一行的**重置两步验证**
可以清除让你被锁在外面的第二因素。这是最常见的解决办法——一个
有两个管理员账户的实例永远不需要用到本页其余的内容。

当没有任何人能够登录,又没有恢复邮箱时:用
`DUMBMONIT_RESET_PASSWORD=1` 启动一次服务器:

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
```

启动时会移除每一个账户和每一个会话,界面会再次显示 `/setup`。
创建第一个管理员,然后在不带该变量的情况下再次启动(在你的环境中
取消设置该变量或将其设为空后,执行 `docker compose up -d
dumbmonit`)。设备、规则、渠道和单点登录设置不受影响。

## 添加设备后没有数据 {#no-data-after-adding-a-device}

请等待一分钟:首次探测会按设备的间隔运行(默认 60 秒),写入
每 5 秒(`DUMBMONIT_FLUSH_INTERVAL_SECS`)到达 VictoriaMetrics。
然后:

1. 打开设备页面并点击**立即探测**。它会报告一次探测产生了多少
   样本和序列,或给出确切的错误。
2. 地址下方显示*配置错误*并附带错误信息,属于配置问题:错误的
   社区名或密码、被拒绝的证书(启用"接受无法验证的证书")、
   缺失的能力。这会被显示出来,但不会触发告警。
3. 显示*不可达*且没有错误信息,意味着设备完全没有应答。对于
   SNMP,错误的社区名看起来与设备关机完全一样,因为设备根本
   不会对此作出应答。请检查社区名、SNMP 是否已启用,以及设备
   是否允许服务器的地址访问。
4. 检查容器能否访问该设备:Docker 网络和防火墙通常是常见原因。

## Ping 显示"配置错误" {#ping-says-configuration-error}

Ping 监控需要一个 ICMP echo 套接字。容器在没有任何 capability
的情况下运行,因此 Compose 文件通过 `dumbmonit` 服务下的
`net.ipv4.ping_group_range` sysctl 允许这类套接字:请确认
`sysctls:` 这几行仍然存在,然后执行 `docker compose up -d`。
参见[无需 capability 的 ICMP ping](install/docker.md#icmp-ping-without-a-capability)。

## 代理没有出现 {#the-agent-does-not-appear}

- 安装程序会在启动服务之前先做一次测试推送,如果 URL 或令牌
  有误,会给出明确的错误并停止。请修正
  `/etc/dumbmonit/agent.yaml` 并重启 `dumbmonit-agent`。
- 安装命令中的 URL 是你浏览器用来访问界面的那个地址。从被监控的
  机器上,这个 URL 必须能够访问到服务器:在那台机器上执行
  `curl http://server:8080/api/health` 就能说明问题。
- 在反向代理之后,必须转发 `/api/ingest`,并允许最多 16 MB 的
  请求体(nginx 中的 `client_max_body_size 16m`)。
- `journalctl -u dumbmonit-agent -f` 会显示每一次被拒绝的推送。
  令牌本身永远不会出现在日志中。
- 一个被撤销的令牌会在下一次推送时使代理停止;请使用新令牌
  重新运行安装程序。

参见[Linux、macOS、FreeBSD 和 Windows 代理](devices/agent.md)。

## 更改端口 {#changing-the-port}

容器监听在 8080 上;主机端口取决于 `docker-compose.yml` 发布的
内容。设置 `DUMBMONIT_PORT`(在 Compose 文件旁边的 `.env` 文件
中,或在命令行上):

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

若要更改容器*内部*的端口,请设置 `DUMBMONIT_BIND`,例如
`0.0.0.0:9000`,并相应调整端口映射。

## 为什么规则写的是"warning",告警却显示"Advisory"? {#why-is-the-alert-advisory-when-the-rule-says-warning}

API 存储三种严重级别:`info`、`warning`、`critical`。界面用天气
简报的词汇来显示它们:Info(信息)、Advisory(提示)、
Warning(警告)。参见[严重级别](alerting/index.md#severities)。

## 某条告警显示"Suppressed by parent" {#an-alert-is-suppressed-by-parent}

该设备有一个不可达的父设备,因此它自己的告警会被暂留而不发送。
修复父设备之后,子设备的告警会自行解决或触发,不会产生第二次
通知。参见[依赖抑制](alerting/index.md#dependency-suppression)。

## "Unusual CPU" 从不触发 {#unusual-cpu-never-fires}

这是一条季节性基线规则,对每个序列的最初 14 天都保持静默。在
这期间,它只会显示本来会触发的内容。参见
[基线学习](alerting/index.md#baseline-learning)。

## 我能使用自己的 VictoriaMetrics 吗? {#can-i-use-my-own-victoriametrics}

可以:将 `DUMBMONIT_VM_URL` 设置为其地址(`http://host:8428`),
内嵌的 VictoriaMetrics 就不会启动。每个指标都带有 `dumbmonit_`
前缀,因此它可以与其他工具共享同一个实例。切换完成后,
`GET /api/health` 会报告 `victoria.embedded: false`。

## DumbMonit 需要能从互联网访问吗? {#does-dumbmonit-need-to-be-reachable-from-the-internet}

不需要。它会主动连接设备和通知服务;除了代理之外没有任何东西
会回调它,而代理只需要能访问服务器的 URL。请把它保留在你的
局域网内或 VPN 之后;如果要公开它,请使用带有 TLS 的反向代理,
并设置 `DUMBMONIT_COOKIE_SECURE=1`。

## 有 API 令牌吗? {#is-there-an-api-token}

有。设置 → **API 与助手**可以创建带有 `read` 或 `write` 范围的
`dmt_…` 令牌;在任何路由上以 `Authorization: Bearer dmt_…` 发送
即可,不需要 cookie 或 CSRF 请求头。同一个令牌也能通过 MCP 连接
一个[助手](using/assistant.md)。令牌永远不能管理账户、登录设置
或其他令牌。参见[HTTP API 参考](reference/api.md#authentication)。
