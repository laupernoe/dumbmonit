# 通知策略 {#notification-policy}

DumbMonit 的承诺是通知值得阅读。告警引擎在告警可能讲话之前保持信息低噪声(依赖关系压制、分组、维护窗口);通知策略在之后保持低噪声,按通道。无需配置:下面的默认值开箱即用。

## 管道,一份列表 {#the-pipeline-in-one-list}

```
evaluate (hysteresis, per-device overrides)
  → deduplicate → suppress by dependency → silence → group by device
    → flap hold → channel severity filter → channel routing filter
      → resolved on/off → cooldown → quiet hours → batch window
        → hourly cap → send (+ deep link)
                                   ↘ nobody acknowledged → escalation channel
```

| 阶段 | 设置位置 | 默认值 |
|---|---|---|
| 清除阈值(滞后) | 规则编辑器 → *清除下方/上方* | 无(CPU: 85%,磁盘:运输规则上 88%) |
| 按设备覆盖 | 规则编辑器 → *按设备覆盖* | 无 |
| 故障检测 | 设置 → 通知与报告 → 通知策略 → 更多选项 | 4 个变化在 30 分钟内,保持 30 分钟 |
| 最小严重级别 | 通道 → 交付选项 → *发送* | 全部 |
| 告诉我何时清除 | 通道 → 交付选项 | 开 |
| 按告警最小间隔 | 通道 → 交付选项 | 无 |
| 安静时间 | 通道 → 交付选项 | 关 |
| 路由过滤器(标签、类型、规则) | 通道 → 交付选项 → *仅某些告警* | 关(通道听到一切) |
| 如果没有人确认则升级 | 设置 → 通知与报告 → 通知策略 | 关 |
| 批处理窗口 | 设置 → 通知与报告 → 通知策略 | 60 秒 |
| 每个通道每小时的消息 | 设置 → 通知与报告 → 通知策略 | 20 |
| 公共 URL(深链接) | 设置 → 通知与报告 → 通知策略,或`DUMBMONIT_PUBLIC_URL` | 空 |

## 滞后和按设备覆盖 {#hysteresis-and-per-device-overrides}

一个在 90%上触发,值一下 90%就立即清除的规则
会在 CPU 在 89–91%徘徊时整晚触发和清除。给它一个
**清除阈值**:一旦触发,告警仅当值下降到其下时解决
(对于"在上方触发"规则)或在其上方(对于"在下方触发"规则)。API 字段是
规则上的`clear_threshold`;服务器拒绝错误一侧的清除
阈值。

一个**按设备覆盖**改变一个设备的阈值或清除阈值,
或关闭该设备的规则,无需复制规则:

```
PUT /api/alerts/rules/{id}/overrides/{target_id}
{ "threshold": 97, "clear_threshold": 95 }      # or { "enabled": false }
GET /api/alerts/rules/{id}/overrides
GET /api/alerts/overrides?target_id=7           # every override of one device
DELETE /api/alerts/rules/{id}/overrides/{target_id}
```

一个所有字段都为空的覆盖被移除。消息引用
实际应用的阈值。

## 故障检测 {#flap-detection}

不管规则如何,一个其指纹在 30 分钟内触发或解决 4 次
的告警是*飘动*:一个通知出去("飘动:4 个变化在
30 分钟,通知保持 30 分钟")并且关于该告警的其他内容
在保持结束前不发送。保持后的解决仍被公告。
调整或在设置 → 通知与报告 → 通知策略 → 更多选项中关闭
(`flap_events`、`flap_window_secs`、`flap_hold_secs`;`flap_events: 0`
禁用它)。

## 按通道交付选项 {#per-channel-delivery-options}

每个通道决定它听到什么(通道有效负载中的`policy`):

- `min_severity` — `info`、`warning`(通知及以上)或`critical`
  (仅警告)。电话通道通常想要`critical`,聊天室
  一切。
- `notify_resolved` — `false`仅听关于问题。
- `min_interval_secs` — 关于
  同一告警的两条消息之间至少这么长(重新触发和提醒)。解决永远不会被它延迟,
  解决仅发送到听到触发的通道。
- `quiet_hours` — 你当地时间的每周窗口,与每周维护窗口相同的形状。在安静时间期间仅`critical`告警通过;
  其余的等待并在它们结束时作为一个摘要到达,标题为"安静时间
  结束 — …"。在安静时间内触发和清除的告警仅
  在该摘要中被提及为"在安静时间期间解决"。
- `matcher` — 通道想要哪些告警,按标签、设备类型或规则。
  空(默认,以及每个现有通道保持)意味着一切。

```
PUT /api/notify/channels/{id}
{ "name": "Phone", "kind": "ntfy", "settings": {…},
  "policy": { "min_severity": "critical", "min_interval_secs": 900,
              "quiet_hours": { "kind": "weekly", "days": [0,1,2,3,4],
                               "start_minute": 1320, "end_minute": 420,
                               "timezone": "Europe/Paris" } } }
```

在更新中省略`policy`保持存储的;`"quiet_hours": null`
清除它们,`"matcher": null`清除路由过滤器。

## 按标签路由 {#routing-by-tag}

设备携带标签(`site=cellar`、`role=storage`)和一种类型(`snmp`、
`proxmox`、`synology`、`agent`…)。通道的**路由过滤器**说哪些
告警它想要:

```json
{ "policy": { "min_severity": "warning",
              "matcher": {
                "include": [ { "field": "tag", "key": "site", "value": "cellar" } ],
                "exclude": [ { "field": "tag", "key": "role", "value": "lab" } ] } } }
```

三个字段,精确相等仅 — 无正则表达式,无括号:

| `field` | 匹配 | 示例 |
|---|---|---|
| `tag` | 设备自己页面的标签 | `{"field":"tag","key":"site","value":"cellar"}` |
| `kind` | 轮询设备的收集器 | `{"field":"kind","value":"proxmox"}` |
| `rule` | 规则的`uid` | `{"field":"rule","value":"disk_full"}` |

列表读取方式:

- **相同**字段上的条件(相同标签键或`kind`或`rule`)
  是一个**OR** — `site=cellar`加`site=attic`意味着任一;
- **不同**字段上的条件是一个**AND** — `site=cellar`加
  `role=storage`意味着两者;
- **`exclude`总是赢**,即使超过匹配的`include`。

所以上面的示例读取,UI 以这些确切的词说回去:
*此通道从标签 site=cellar 的设备接收通知及以上,
除了标签为 role=lab 的设备*。

在编辑器下,一个**预览**列出你当前设备的哪些
过滤器选择。它不是一个近似:服务器运行告警引擎运行的确切过滤器,
仅限于依赖于设备本身的部分。
规则条件适用于告警而不是设备,所以预览命名它们
单独而不是假装计数它们。

```
POST /api/notify/match-preview
{ "matcher": { "include": [ { "field": "tag", "key": "site", "value": "cellar" } ] } }

→ { "devices": [ { "id": 3, "name": "nas", "kind": "synology",
                   "tags": { "site": "cellar" }, "matched": true } ],
    "matched": 1, "total": 12, "rules": [], "excluded_rules": [] }
```

一个过滤器限制为十二个条件。超过那个,第二个通道
比一个更多条件清晰。

## 升级:如果没有人确认 {#escalation-if-nobody-acknowledges}

一个额外的跳跃,仅一个。当策略设置`escalate_after_secs`和
`escalate_channel`,一个没有人[确认](../using/alerts.md)的告警
在该延迟内仅一次也发送 — 到该通道:

```
PUT /api/notify/policy
{ "escalate_after_secs": 900, "escalate_channel": 4 }
```

- 延迟从**第一条消息实际发出的时刻**计数,
  不是从告警触发。如果安静时间或失败的交付吞没了
  第一个通知,没有什么要升级的还,跳跃等待。
- **确认告警停止它**,就像它停止提醒一样;告警清除也是。
- 升级消息去升级通道**仅** — 已经听到触发的通道
  不被告诉两次。
- 它超过该通道自己的严重级别下限、路由过滤器和安静时间:
  你命名这个通道被唤醒,所以在它重要的那一刻不被保持。
- 这行说:🔴 Critical · Disk full — /data — 95% … — still
  unacknowledged`。

`"escalate_channel": null`关闭升级;延迟为`0`做
相同。两者都需要跳跃存在,服务器拒绝不存在的通道
而不是升级进入虚空。

DumbMonit 故意停止在这里:一个跳跃,无轮换,无
这周谁在值班的时间表。如果你需要那个,指向升级通道
在 PagerDuty 或 Opsgenie,这做它适当。

## 批处理和每小时上限 {#batching-and-the-hourly-cap}

`GET/PUT /api/notify/policy`持有全局设置:

```json
{ "batch_window_secs": 60, "max_per_hour": 20,
  "flap_events": 4, "flap_window_secs": 1800, "flap_hold_secs": 1800,
  "public_url": "https://monit.example.lan",
  "escalate_after_secs": 0, "escalate_channel": null }
```

- **批处理窗口** — 在窗口内触发的告警离开为*一条*
  每个通道的消息,按设备分组:"2 个设备上的 3 个告警"有
  每个设备的部分;解决搭车("2 个已解决")。`0`在下一个周期发送。摘要列出最多 12 行,然后"…和 N 个更多告警"。
- **每小时上限** — 一旦一个通道在最后一小时接收了`max_per_hour`条消息,
  进一步的告警等待并作为一个摘要到达当一个插槽释放。
  `0`移除上限。
- **值班通道**(PagerDuty、Opsgenie)永远不被批处理过设备:它们打开和关闭
  每个设备一个事件,所以它们接收每个设备一条消息而不等待。

交付失败保持行队列;下一个周期重试。

## 一行说什么 {#what-a-line-says}

每个告警是一行,在邮件、ntfy、聊天和
自定义 webhook 的`{{message}}`变量中相同:

```
🔴 Critical · Disk full — /data — 95 % (threshold > 90 %), for 12 min
⚠️ Warning · VM or container stopped — win11-desktop (101), for 59 s
✅ Resolved · Service down — https://example.lan/
```

- 字形之后是状态词(Critical / Warning / Info,
  Resolved、Flapping),所以严重级别可读没有颜色或表情符号。
- 一个规则看几个系列名称那个触发的:VM 和
  其 id、挂载点、接口、备份组、探针的 URL。设备范围的规则(不可达)命名不超过设备。
- 值和阈值显示用于测量规则;一个万无一失的
  规则在 0/1 度量(在电池上、客人停止)不重复"1
  (threshold > 0)"。

## 深链接 {#deep-links}

每条消息以一个设备的链接结尾(`{public_url}/targets/{id}`)
当已知公共 URL — 策略的`public_url`或
`DUMBMONIT_PUBLIC_URL`。自定义 webhook 也将其作为`{{link}}`
变量接收除非它们自己的`base_url`被设置。

故意没有消息中的"沉默 1 小时"链接:聊天服务
获取每条他们显示的链接以构建预览,这会触发
沉默时刻消息被发布。从设备页面或
告警页面改为沉默。

## 什么从 Pulse 借来,什么不是 {#what-was-borrowed-from-pulse-and-what-was-not}

设计跟随[Pulse](https://github.com/rcourtman/Pulse)它适合
homelab 监视器:触发/清除阈值、一个按资源覆盖层、一个
通知冷却、一个分组窗口、每小时速率限制、安静时间、
故障检测带冷却、"在解决时通知",以及值班工具
保在摘要之外。未采纳:Pulse 的 24 小时观察期
在通知前(DumbMonit 的基线规则已经静默学习;阈值规则
应该从第一天讲话)和从消息打盹/确认(见上文)。
按目的地的基于标签的路由*被*采纳,如上面的路由过滤器 —
在规则上选择通道变成了错误的地方,对于"地下室的一切
去这个房间"。
