# 状态页面 {#status-pages}

![The Status page: your pages, and the announcements under them](../assets/screenshots/status-light.png){ loading=lazy }

状态页面是你监控系统的公开门面：任何人都能打开的页面——无需登录，没有 cookie——它说明你的服务是否正常，为每个服务显示最多 90 天的每日历史，承载你的事故和维护公告，并让访客通过 RSS 或电子邮件关注它们。徽章可以把同样的答案放进 README；紧凑视图则可嵌入内网页面。

它只显示你放上去的内容：你给每个服务设定的标签、其状态、可用率和响应时间。监控项名称、地址、类型和标识符绝不会离开服务器——不会出现在页面中、徽章中或电子邮件中。

## 1. 创建页面 {#1-create-a-page}

**设置 → 状态页面** → **新建页面**。该页面列出你的状态页面（打开、编辑、删除），其下是公告；编辑器在独立路由中打开（`/settings/status/new`、`/settings/status/<id>`）。公开渲染位于 `/s/<address>`。

| 字段 | 作用 |
| --- | --- |
| **标题** | 页面的标题（“Home lab”）。 |
| **地址** | URL 的最后一部分，`/s/<address>`。根据标题自动建议；小写字母、数字和连字符，2 到 40 个字符。 |
| **描述** | 标题下的一句话。可选。 |
| **主题** | 日间、夜间，或跟随访客的系统设置。 |
| **历史** | 历史条覆盖的最长时间窗口（30、60 或 90 天）。该条会适应页面数据的年龄：显示的天数等于最早的服务被监控的天数，最少 7 天，最多到此设置值。页面绝不会说出超过此范围的内容——参见[徽章](#badges)。 |
| **简单模式** | 朴素页面：没有场景、没有吉祥物、没有动画，也没有彩蛋。明暗跟随访客的系统设置，页眉有切换开关；页面的主题设置会被忽略。 |
| **已发布** | 关闭 = 草稿：在你打开之前，页面、其徽章和徽标对访客都返回“未找到”。 |

然后勾选要显示的监控项。对每一个，设置访客将读到的**标签**（默认为监控项名称——你可能更喜欢“Website”而不是“nginx-front-01”），并可选地设置**分组**（“Network”、“Storage”）。分组会成为页面上的区块，顺序与你排列服务的顺序一致。

保存后，链接即可复制。页面每分钟自动刷新。

### 外观 {#look}

编辑器的**外观**部分让页面换上你的配色，而不会向任意代码开放：

| 设置 | 作用 |
| --- | --- |
| **徽标** | PNG、JPEG 或 WebP，最大 256 KiB，显示在标题旁。方形效果最佳。SVG 会被拒绝：它可能携带脚本，而页面与管理界面使用同一地址提供。文件按其内容进行检查，存放在 `/data/status-pages/` 下，并且仅作为图片提供。 |
| **强调色** | 墨色（默认）、蓝色、青色、紫罗兰色、玫瑰色或琥珀色——顶部线条、链接和订阅按钮。每一种都经过挑选，确保日间和夜间都清晰可读。 |
| **组织网站** | 显示在标题旁的 `http://` 或 `https://` 链接，以站点主机名呈现。 |
| **页脚文字** | 页面下方的纯文本，最多 280 个字符：由谁运营、如何联系他们。换行会被保留；不会被当作 HTML 或 Markdown 解释。 |

### 场景 {#scene}

**场景**设置会在横幅后绘制一幅城市天际线。新页面默认使用巴黎，原来没有场景的现有页面现在也会显示它；取消勾选可得到朴素横幅，或开启**简单模式**以完全去掉场景。可以随意多选，并按你想要的顺序排列。城市：威尼斯、巴黎、东京、纽约、伦敦、罗马、悉尼、迪拜、旧金山、巴塞罗那、阿姆斯特丹、伊斯坦布尔和里约热内卢。新七大奇迹：奇琴伊察、马丘比丘、长城、佩特拉和泰姬陵（里约的基督像在里约场景中）。古代奇迹：吉萨、巴比伦、以弗所、奥林匹亚、哈利卡纳苏斯、罗得岛和亚历山大，外加雅典卫城。选择多个时，**轮换**决定页面何时切换到下一个：每次访问（默认）、每分钟、每十分钟或每小时。这些场景是 DumbMonit 随附的一组封闭图画——不会上传或解释任何内容。

刻意不提供自定义 CSS 或 HTML：状态页面是公开的，并且与你的管理界面同源，受同一[内容安全策略](../reference/api.md)约束。

## 2. 访客看到什么 {#2-what-visitors-see}

- 一个横幅，给出一秒钟即可读懂的答案，读取自各服务：**All systems operational**（所有系统正常）、**Partial outage**（部分中断，即部分服务宕机或降级）、**Major outage**（重大中断，即全部宕机）或 **Scheduled maintenance**（计划维护）。当存在未结束的事故而所有服务仍有响应时，显示 **Incident in progress**，并按其影响程度着色。
- 未结束的公告，最新更新在前，更新时间线折叠在下方。
- 每个服务及其状态标牌（Operational、Degraded、Down、Maintenance、No data）、其**历史条**——每天一个方块——其 24 小时、7 天、30 天以及（90 天页面上的）90 天可用率，以及可用性探测的响应时间。
- **过往事故**，按日期列出最近 30 天。
- **获取更新**：RSS 订阅源，以及在你设置后出现的电子邮件订阅表单（参见[电子邮件订阅者](#email-subscribers)）。

### 历史条 {#the-history-bar}

每个方块代表一天（UTC），按当天的可用率着色：99.5 % 及以上为全色，从 95 % 起为琥珀色，低于则为红色。**没有任何测量的日子是灰色**，而不是绿色——上周才添加的服务有 83 个灰色日子，而不是 83 个完美日子。悬停在方块上，或聚焦历史条并用方向键移动（Home 和 End 跳到两端），即可读取日期、可用率、**停机分钟数**以及涉及当天的事故。这些文字同样提供给屏幕阅读器：状态绝不只靠颜色传达。

状态的判定方式：

| 状态 | 含义 |
| --- | --- |
| Operational | 最近一次探测成功（可用性探测），或监控项在三个轮询周期内有上报。 |
| Degraded | 当前为正常，但在最近一小时内至少有一次探测失败。 |
| Down | 最近一次探测失败、监控项停止上报，或其配置有误。 |
| Maintenance | 维护窗口进行中且服务已宕机：属于预期，不算中断。可以是此页面上的公告，也可以是覆盖该监控项的告警[维护窗口](../alerting/maintenance.md)。 |
| No data | 尚未探测过，或已禁用。 |

探测类（`http`、`tcp`、`dns`、`ping`、`tls`、心跳）的可用率是成功检查所占的比例，来自 `dumbmonit_probe_success`；一天的停机时间是失败检查次数乘以探测间隔。对其他监控项（SNMP、代理、Proxmox……），它是监控项至少上报过一次（`dumbmonit_up`）的五分钟时段所占的比例，从第一次测量起算——昨天才添加的监控项不会因此在之前的 89 天里算作“宕机”——一天的停机时间是其静默的时段。

## 3. 发布事故和维护公告 {#3-announce-incidents-and-maintenance}

在**设置 → 状态页面**的页面列表下方，点击**事故与维护** → **新建公告**（也可用 ++ctrl+k++ → *Announce an incident*）。

- **事故**有标题、影响程度（**minor** 使页面显示为降级，**major** 显示为中断），并依次经过 *Investigating → Identified → Monitoring → Resolved*。随着进展发布更新：每条更新都带有状态和消息，并成为访客读到的最新一行。**解决**会以最终消息关闭它；之后它会在“过往事故”中保留 30 天。
- **维护**窗口有开始和结束时间，并依次经过 *Scheduled → In progress → Completed*。进行中时，横幅显示“Scheduled maintenance”，已宕机的服务显示为 *Maintenance*。

[在告警中安排的维护窗口](../alerting/maintenance.md)对其覆盖的那一个监控项起同样作用，且无需公告：窗口打开期间，该服务显示 *Maintenance* 而不是红色；当没有其他宕机或降级时，页面的整体状态显示 *Maintenance*。只有指明了监控项的窗口才会以这种方式呈现；窗口的名称和备注保持私密。

公告可显示在某一个页面上，或**所有页面**上。

### 电子邮件订阅者 {#email-subscribers}

在编辑器的**电子邮件订阅者**下选择一个**电子邮件（SMTP）渠道**后，访客即可通过电子邮件订阅你的公告。该渠道是你已在[通知](../alerting/notifications.md)下配置好的；会复用它的服务器和发件人，但不会复用它自己的收件人。没有渠道时，页面只提供 RSS 订阅源。

- **双重确认订阅。** 访客输入地址；DumbMonit 发送一封确认链接邮件，在链接被点击之前不会发送其他任何内容。从未确认的请求在两天后被丢弃。无论地址是新的、待确认的还是已订阅的，表单都回答同一句话，因此不会向任何人泄露谁在关注你的页面。
- **发送什么。** 创建公告、发布更新以及其状态变化时发送邮件——事故和维护一视同仁，针对该页面（对于显示在所有页面上的公告，则针对每个页面）。
- **退订。** 每封邮件末尾都有退订链接，并带有 `List-Unsubscribe` 和 `List-Unsubscribe-Post` 头，因此 Gmail、Apple Mail 等会提供各自的一键按钮。
- **限制。** 公开的订阅端点每个客户端地址每 15 分钟接受五次请求，总共六十次；待确认的地址在十分钟内不会被重复发信：页面不可能被变成邮件炮。
- 编辑器会列出订阅者（已确认或待确认），并可移除其中任何一个。

这些邮件中的链接，在页面有[自定义域名](#custom-domain)时指向该域名（`https://status.example.com/`）。否则，链接由[通知策略](../alerting/notifications.md)的**公开 URL**（或 `DUMBMONIT_PUBLIC_URL`）构建；两者都没有时，则取你上次保存页面时使用的地址——绝不取自访客的请求。

## 4. 分享链接 {#4-share-the-link}

页面位于 `https://your-server/s/<address>`。`/s/` 和 `/api/public/` 下的一切都无需认证即可访问；DumbMonit 的其余部分仍需登录。

编辑器的**分享**面板会为你生成下面的代码片段，并附预览。

### 徽章 {#badges}

每个已发布的页面都提供 SVG 徽章——扁平风格，纯色底白字，在 GitHub README 的明暗两种主题下都清晰可读：

| 徽章 | URL |
| --- | --- |
| 页面状态 | `/api/public/status/<address>/badge.svg` |
| 页面可用率 | `/api/public/status/<address>/uptime.svg?days=30` |
| 页面响应时间 | `/api/public/status/<address>/response.svg` |
| 服务状态 | `/api/public/status/<address>/components/<service>/badge.svg` |
| 服务可用率 | `/api/public/status/<address>/components/<service>/uptime.svg?days=30` |
| 服务响应时间 | `/api/public/status/<address>/components/<service>/response.svg` |

`<service>` 是服务标签的 URL 形式：“Public API”变为 `public-api`（第二个“Public API”则是 `public-api-2`）。它也是 JSON 文档中每个服务的 `key`。`days` 为 `1`、`7`、`30`（默认）或 `90`，且不会超过页面的历史范围：30 天的页面会拒绝 90 天的徽章。页面的可用率和响应时间是其各服务的平均值。没有任何测量的服务显示灰色的 **no data**。

在 README 中：

```markdown
[![Status](https://your-server/api/public/status/home-lab/badge.svg)](https://your-server/s/home-lab)
[![Uptime](https://your-server/api/public/status/home-lab/uptime.svg?days=30)](https://your-server/s/home-lab)
[![API](https://your-server/api/public/status/home-lab/components/public-api/uptime.svg)](https://your-server/s/home-lab)
```

徽章缓存一分钟（`Cache-Control: public, max-age=60`），因此 GitHub 的图片代理和你的访客都不会给时序数据库带来任何开销。

### 嵌入 {#embed}

`/s/<address>/embed` 是适用于 iframe 的紧凑视图：整体状态，以及每个服务一行，包含其状态和 30 天历史，另有一个在新标签页中打开完整页面的链接。

```html
<iframe src="https://your-server/s/home-lab/embed"
        title="Home lab status" width="100%" height="320" style="border:0"></iframe>
```

添加 `?theme=light` 或 `?theme=dark` 以匹配宿主页面，添加 `?history=0` 以隐藏历史条。状态页面及其嵌入视图是 DumbMonit 中仅有的允许第三方页面嵌入框架的地址；其他一切（包括确认和退订页面）都以 `frame-ancestors 'none'` 和 `X-Frame-Options: DENY` 响应。

### 自定义域名 {#custom-domain}

页面可以拥有自己的地址，如 `status.example.com`，做法与 Uptime Kuma 相同。在编辑器的**公开域名**字段中输入名称（旁边的 **i** 概括了步骤）并保存。当请求以该名称作为 `Host` 头到达 DumbMonit 时，DumbMonit 在根路径 `/` 显示该页面，除此之外什么也不显示：在该名称下，登录界面、设置、API、代理端点、指标以及其他状态页面全部返回 `404 Not found`，即使是已登录的人也一样，并且不会读取或设置任何会话 cookie。只提供该页面、其嵌入视图、其确认和退订页面、其徽章、订阅源和 JSON 文档，以及界面自身的文件（`/_app/`、图标）。

名称以小写存储，不含 `https://` 或尾部斜杠；带端口、路径或 IP 地址的会被拒绝。两个页面不能共用一个名称，而你访问 DumbMonit 本身所用的地址（或其公开 URL）不能作为页面的域名——保存它会把你锁在外面。清空该字段即可移除域名；无论有无域名，页面都保留在 `/s/<address>`。

设置方法：

1. **DNS。** 为该名称创建记录（或 Cloudflare Tunnel 的公开主机名），指向你的反向代理。
2. **代理。** 将该名称转发到 DumbMonit 的地址，例如 `http://dumbmonit:8080`，**不要重写路径**：代理转发的是 `/`，而不是 `/s/<address>`。
3. **Host 头。** 原样传递原始的 `Host` 头：DumbMonit 靠它区分页面的域名与自己的地址。DumbMonit 只读取 `Host`；`X-Forwarded-Host` 仅在来自列于 `DUMBMONIT_TRUSTED_PROXIES` 的代理时才被采信。
4. **HTTPS** 由代理处理。DumbMonit 在订阅者邮件、RSS 订阅源和编辑器的**分享**面板中链接到 `https://<domain>/`。

Traefik（Docker 标签）——默认会传递 `Host` 头：

```yaml
labels:
  - traefik.enable=true
  - traefik.http.routers.status.rule=Host(`status.example.com`)
  - traefik.http.routers.status.entrypoints=websecure
  - traefik.http.routers.status.tls.certresolver=letsencrypt
  - traefik.http.services.status.loadbalancer.server.port=8080
```

Nginx Proxy Manager——添加一个**代理主机**，域名 `status.example.com`，协议 `http`，转发主机名 `dumbmonit`，端口 `8080`，并配置 SSL 证书。无需其他设置：它默认会传递 `Host`。**自定义位置**保持为空。

Cloudflare Tunnel——添加一个**公开主机名** `status.example.com`，服务为 `http://dumbmonit:8080`，不带路径。隧道会保留访客的 `Host`；高级设置中的 **HTTP Host Header** 请留空。

Caddy——`reverse_proxy` 会传递 `Host`，且 Caddy 会自动获取证书：

```caddyfile
status.example.com {
    reverse_proxy dumbmonit:8080
}
```

Nginx——`proxy_pass` 默认发送上游的名称，因此需要设置该头：

```nginx
server {
    listen 443 ssl;
    server_name status.example.com;
    # ssl_certificate … ;

    location / {
        proxy_pass http://dumbmonit:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**故障排查。**

- *该域名显示的是 DumbMonit 的登录界面或仪表板*：`Host` 头没有到达 DumbMonit——代理把它替换成了上游的名称（`dumbmonit:8080`）。添加 `proxy_set_header Host $host;`（Nginx），移除自定义的 **HTTP Host Header**（Cloudflare Tunnel），或移除重写 `Host` 的中间件（Traefik）。不会暴露任何私密内容：仪表板仍然需要密码。
- *该域名上只有单独一句 `Not found`*：代理重写了路径（发送了 `/s/<address>` 或添加了前缀），或编辑器中的域名与浏览器中的名称不完全一致。请原样转发路径。
- *页面提示不存在*：页面仍是草稿——请发布它。
- *你无法再访问 DumbMonit 的界面*：你是通过页面的域名在浏览。请使用 DumbMonit 自己的地址，或清空页面的**公开域名**。

### 位于反向代理之后 {#behind-a-reverse-proxy}

[自定义域名](#custom-domain)是仅暴露状态页面的最简单方式。如果你更愿意在公开名称上保留 `/s/<address>` 路径，则改为在代理中过滤路径。以 Caddy 为例：

```caddyfile
status.example.com {
    @public path /s/* /api/public/* /_app/* /favicon.svg
    reverse_proxy @public dumbmonit:8080
    respond 404
}
```

使用 nginx：

```nginx
location ~ ^/(s/|api/public/|_app/|favicon\.svg) {
    proxy_pass http://dumbmonit:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
location / { return 404; }
```

`/_app/` 承载界面的脚本和样式：页面需要它们才能渲染。请转发 `X-Forwarded-Proto` 和 `X-Forwarded-Host`，以便 RSS 订阅源链接到正确的地址，并设置公开 URL（见上文），使订阅者邮件同样如此。

### 订阅源与文档 {#feed-and-document}

- `GET /api/public/status/<address>/rss` — 事故和维护窗口的 RSS 订阅源，每条公告一项，附其最新更新。
- `GET /api/public/status/<address>` — 构建页面所用的 JSON 文档，如果你想构建自己的页面：页面的标题、描述和外观、整体状态、每个服务的 `key`、`label`、`state`、可用率（`uptime_24h`、`uptime_7d`、`uptime_30d`、`uptime_90d`）、`latency_ms` 和 `history`（每天的 `date`、`uptime_pct`、`down_minutes`、`incidents`），以及公告。

任何网站都可以在浏览器中读取该文档（响应带有 `Access-Control-Allow-Origin: *`，且不含 cookie）。`GET /api/public/status/<address>/banner.js` 是一个现成的脚本，可将其变成横幅：参见[网站横幅](../integrations/website-banner.md)。

该文档在服务器上缓存 30 秒；徽章、订阅源和嵌入视图读取的都是同一份缓存文档，因此它们说出的内容都不会比页面更多。
