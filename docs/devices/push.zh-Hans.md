# 心跳(push) {#heartbeat-push}

这是没人会注意到的那种悄无声息的故障:某次更新后不再运行的夜间
备份、丢失了权限的证书续签 cron、因重命名而损坏的 Home Assistant
自动化。没有任何东西会抱怨,直到你真正需要那份备份的那一天才会
发现。

**心跳**监控(也叫*死人开关*,或 Uptime Kuma 中的*push*监控)把
这个逻辑反过来:DumbMonit 不会主动联系任何东西。相反,任务每次
运行时会调用一个秘密 URL,正是调用的**缺失**才会触发告警。

## 工作原理 {#how-it-works}

每个心跳设备都会获得一个形如
`https://<你的 DumbMonit>/api/push/<token>` 的 URL。该令牌是一个
128 位的随机值,以哈希形式存储用于查找,以加密形式用于显示:
只要该设备存在,就会一直显示在设备页面上,并带有一个复制按钮。
它被故意设计为开放的——没有会话、没有 API 令牌、没有
`X-Requested-With` 请求头——因为 crontab 中的一行 `curl` 命令
不会带有这些东西。持有该令牌只能让你表示"任务运行了"。

每次调度器执行时(该设备的间隔,默认 60 秒),采集器会读取上次
调用的时间并写入:

| 情况 | `probe_success` | 效果 |
|---|---|---|
| 尚未收到任何调用 | *(不写入任何内容)* | 该设备显示为**等待中**。不会告警:该任务尚未就位。 |
| 上次调用在预期间隔加宽限期以内 | `1` | **上报中**。 |
| 上次调用超出预期间隔加宽限期 | `0`, `reason="missed"` | **宕机**;"心跳丢失"规则会在两分钟后触发。 |
| 上次调用携带 `status=down` | `0`, `reason="reported_down"` | 无论时钟怎么说,立即**宕机**。 |

一次调用也会立即重新评估该设备,因此一个恢复运行的任务会立即
清除其告警,而不需要等待下一次调度器执行。

该接口在记录调用时应答 `204` 和空响应体,对未知或已重新生成的
令牌应答 `404`,对既不是 `up` 也不是 `down` 的 `status` 应答
`400`,对单个令牌每分钟超过六十次调用应答 `429`(带有
`Retry-After`)。`GET` 和 `POST` 都被接受;`HEAD` 也可用。

### 查询参数 {#query-parameters}

| 参数 | 含义 |
|---|---|
| `status` | `up`(默认)或 `down`:任务自行报告一次失败。 |
| `msg` | 一段自由的说明文字,会被保留(截断到 500 字符)并显示在设备页面上。 |

这些正是 Uptime Kuma 的 push 监控所使用的参数:为它编写的脚本
无需修改即可使用。

## 设置 {#setup}

以下步骤正是表单旁边提示所展示的内容。

1. 在地址中,为这个任务写一个简短的标签("nightly-backup"、
   "certbot-renew"):不会联系任何东西,该标签只需要在你的心跳
   设备中保持唯一即可。
2. 把预期间隔设置为该任务的计划(夜间任务为"24h",每小时任务
   为"1h"),如果计划有波动,再设置一个更宽的宽限期。保存该
   设备。
3. 设备页面会显示要调用的 URL。把它添加到任务的末尾,这样它只会
   在任务成功时才被调用:

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. 一个能判断自身是否失败的任务,可以主动报告失败,而不是保持
   沉默:在 URL 后追加 ?status=down&msg=…,告警会立即触发。
5. 在收到第一次调用之前,该设备会显示"等待中",不会触发任何
   告警。URL 丢失或泄露了?设备页面上的"重新生成"会发出一个
   新的 URL;旧的 URL 会立即停止应答。

!!! warning "警告"
    请在任务末尾、真正重要的部分完成之后调用该 URL。如果把调用
    放在开头,即使备份本身失败了,也会报告成功。

凭据:无。

### 示例 {#examples}

一行 crontab,每晚 03:00 执行,只在备份成功时才调用:

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

一个能报告自身失败、把退出码作为消息的脚本:

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
if /usr/local/bin/backup.sh; then
    curl -fsS -m 10 --retry 3 "$URL" > /dev/null
else
    curl -fsS -m 10 --retry 3 "$URL?status=down&msg=backup+exit+$?" > /dev/null
fi
```

Home Assistant,在某个自动化末尾调用的 `rest_command`:

```yaml
rest_command:
  dumbmonit_heartbeat:
    url: "https://monit.example.com/api/push/<token>"
    method: post
```

Windows 任务计划程序(PowerShell):

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "https://monit.example.com/api/push/<token>" | Out-Null
```

### 选项 {#options}

| 键 | 标签 | 默认值 | 说明 |
|---|---|---|---|
| `expected_interval` | 预期间隔 | `24h` | 该任务预计多久调用一次:30m、1h、6h、24h、7d(或以秒为单位的数字)。一旦超过该间隔加宽限期,就会被判定为一次丢失的调用。 |
| `grace` | 宽限期 | `10%` | 在预期间隔之后、心跳被判定为丢失之前额外容忍的时间:可以是间隔的百分比(10%)或固定时长(15m)。永远不会少于一分钟。 |

时长可以使用 `s`、`m`、`h`、`d`、`w` 后缀,并可以组合使用
(`1h30m`);一个裸数字表示秒数。预期间隔必须在 10 秒到一年
之间。

## 指标 {#metrics}

所有指标除了 `target`、`host` 和 `tag_*` 之外,都带有
`probe="push"` 标签,并以 `dumbmonit_` 为前缀。

| 指标 | 含义 |
|---|---|
| `probe_success` | 如果上次调用按时到达且未报告失败则为 1,否则为 0。首次调用之前不存在。 |
| `probe_failure_info` | 存在时为 1,带有 `reason` 标签:`missed` 或 `reported_down`。 |
| `push_last_seen_seconds` | 上次调用的年龄。 |
| `push_received_total` | 自该令牌创建以来收到的调用次数(计数器)。 |

与任何服务监控一样,三十天内的可用性:
`avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`。

适用的内置规则:**心跳丢失**(提示级别,2 分钟)。"服务宕机"和
"服务抖动"规则会跳过心跳设备,"设备不可达"也永远不会为它们
触发:采集器始终会运行,只是它的判定结果会变化。

## API {#api}

| 路由 | 会话 | 用途 |
|---|---|---|
| `GET` 或 `POST /api/push/<token>` | 不需要 | 调用本身。`204`、`404`、`400` 或 `429`。 |
| `GET /api/targets/<id>/push` | 需要 | 该监控:`token`、`path`、`last_seen_at`、`last_seen_age_secs`、`last_status`、`last_message`、`received_total`、`expected_interval_secs`、`grace_secs`、`verdict`(`waiting`、`on_time`、`missed`、`reported_down`)、`settings_error`。缺失时会在首次读取时创建。 |
| `POST /api/targets/<id>/push/regenerate` | 管理员 | 新令牌;之前的 URL 从此开始应答 `404`。计数器和上次调用记录会被保留。 |
