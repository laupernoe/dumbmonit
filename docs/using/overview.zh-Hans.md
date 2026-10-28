# 概览页面

首页大约一秒内回答"一切正常吗?",之后让你按需深入查看。

## 状态横幅 {#the-bulletin}

顶部的横幅是一片固定的夜空:闪烁的星星、一弯新月,以及一只飞越夜空却从不降落的鸽子。夜空不会随天气变化;底部的一句话用文字说明现状。没有需要你处理的事时是"全部 146 个监控项正常。",否则是"pve-02 已宕机。"或"pve-02 需要看一眼。",随后用更轻的语气说明系统做了什么("3 个客户机已暂缓,告警已发送。")。点一下鸽子,它会翻一个滚。在 `prefers-reduced-motion` 下,鸽子不再出现,星星也保持静止。

## 首次运行引导 {#the-first-run-guide}

在一个全新的实例上,简报下方的区域不是空的:它是一个三步引导——
添加一个监控项、连接一种被告知的方式、检查消息是否送达。每一步
只需一次点击,每一步都可以单独跳过,**跳过引导**会一次性隐藏
全部三步。该选择会存储在实例上,因此换一个浏览器不会重新开始
引导。参见[添加第一个监控项](../install/first-device.md#the-three-first-steps)。

## 五项指标与健康河 {#the-figures}

横幅下方是五项指标:**Monitors up**(占总数的比例,附宕机与暂停数量)、**Availability 30 d**、**p95 response time**、**Open alerts**(严重与警告)以及 **MTTR 30 d**(平均恢复时间)。页眉中的时间范围决定 p95 和健康河。

**健康河**为每个监控项一条带,时间范围的每个时间片一格,最差的监控项在最上方。状态总是写在形状旁边(圆形为正常,菱形为降级,方形为宕机),绝不只靠颜色表示。

## 监控项表格 {#the-monitor-table}

表格列出每个监控项,最差的在前。可按状态、类型和文本筛选,可增加列(地址、文件夹)或移除其他列,并可用键盘操作:++j++ / ++k++ 移动,++enter++ 选中,++o++ 打开。

选中的监控项会驱动右侧的**上下文面板**,类似检查器:需要处理的事项(含告警操作,见[告警页面](alerts.md))、**Check now** 与 **Silence 1 h**、最近的预测以及谁在值班。++i++ 显示或隐藏它;在较窄的屏幕上它以抽屉形式打开。完整列表见[监控项页面](devices.md)。

## 大屏模式 {#wall-mode}

`/wall` 为房间里的显示器呈现一块看板:结论、时钟、五项指标、健康河、未解决的告警以及谁在值班。通过顶部栏的 **Wall mode** 图标或命令面板打开;++esc++ 或 **Exit** 返回控制台。参见[大屏模式](wall.md)。

## 安装到手机上 {#install-it-on-a-phone}

该界面提供一个 web manifest,因此手机可以把它添加到主屏幕:
iOS 上使用**分享 → 添加到主屏幕**,Android 上在 Chrome 菜单中
使用**安装应用**。之后它会在没有浏览器外框的情况下打开,并
使用手机设置的主题。

这里刻意**没有离线模式,也没有 service worker**。一个显示缓存
页面的监控屏幕,比一个明确表示连不上服务器的屏幕更糟糕:这整件
事的意义就在于你看到的内容是最新的。没有 worker,升级后的服务器
内容也会立即被提供,不存在需要清除的过期构建。

## 命令面板 {#command-palette}

macOS 上按 ++cmd+k++,其他系统按 ++ctrl+k++,即可打开命令面板:一个输入框,结果分为三组。

| 分组 | 条目 |
|---|---|
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | 每个监控项,带其状态指示灯;++enter++ 打开它 |

## 导航 {#navigation}

左侧是一条窄的**图标栏**,列出所有入口:Overview、Monitors、Alerts(带需要处理的数量)、Dashboards、Explore、拓扑图、Security、Logs、Status pages、Reports 和 Settings。底部是文档、GitHub 和你的账户。页眉显示你所在的位置,以及暂停控制台自身刷新的 **LIVE** 开关、时间范围、命令面板、**Wall mode** 和 **New monitor**。在手机上,图标栏变成底部标签栏,包含核心入口和 **More**。

- **Overview**:一切是否正常,以及哪些需要你处理。
- **Monitors**:你监控的每个来源。参见[监控项页面](devices.md)。
- **Alerts**:正在触发的告警、维护窗口、历史和规则。参见[告警页面](alerts.md)。
- **Settings**:你的账户、用户与单点登录、令牌、通知渠道、外观、关于。参见[Settings](settings.md)。

界面为深色,且界面上没有任何内容仅靠颜色传达状态:标记旁总有文字(Reporting、Unreachable、Advisory、Warning、Suppressed by parent、Building up)。
