# 大屏模式与音乐 {#wall-mode-and-music}

`/wall` 会以全屏方式单独显示简报,适用于房间中一直保持开启的
屏幕:那句话、天气窗口、三个大数值读数以及*需要处理*列表,每
20 秒刷新一次,并让屏幕保持常亮。++esc++ 或点击**退出**会返回
概览页面。可从命令面板("大屏模式")打开,也可直接输入该 URL。
`?theme=dark` 或 `?theme=light` 可以为该显示器强制指定一种主题,
而不影响你已保存的偏好设置。

![大屏模式](../assets/screenshots/wall-light.png){ loading=lazy }

左侧的每个问题都带有其监控项的产品图标(与监控项列表中的标记相同)。
列表永远不会超出自己的空间:在小屏幕上,或问题很多时,大图块会
缩成每项一行,然后折叠为*另有 N 项*,而不是与下方的监控项重叠。

## 大屏上的新闻 {#news-on-the-wall}

管理员可以在**设置 → 墙面新闻**中为大屏添加**头条新闻**和一个
**直播频道**。两者默认都是关闭的。

- **头条滚动条。** 大屏底部的一条横带会滚动显示最新的头条,每条
  都带有来源和时间,每五分钟刷新一次。如果显示监控项的系统启用了
  *减少动态效果*,则不会滑动:一次只淡入淡出一条头条。
- **直播视频。** 一个静音的直播频道显示在图块中,位于右栏的时钟
  下方(或场景中央),可选小、中、大。该图块会缩小到剩余的空间;
  它绝不会盖住状态。
- **图片轮播。** 同样这些订阅源中带有图片的最新文章,一次一篇显示在
  大屏中央,每篇八秒,交叉淡入淡出,图片上叠加来源、时间和标题。启用
  *减少动态效果*时,图片直接切换,不做淡入淡出。

读取订阅源的是服务器,而不是浏览器(浏览器无法读取其他站点的
订阅源)。服务器最多跟随三次重定向,拒绝内部地址,每个订阅源在
八秒或 2 MB 后放弃,并将应答缓存五分钟,供所有大屏共用。它支持
RSS 2.0、RSS 1.0(RDF)和 Atom。

文章的图片取自订阅源:最宽的 `media:content` 图片、`media:thumbnail`、
图片类型的 `enclosure`(BFM TV)、Atom 的 `link rel="enclosure"`,或者
文章内容中的第一个 `<img>`(Korben)。当订阅源没有图片且轮播已开启
时,服务器会读取文章页面的开头一次(最多 512 KB,同样有地址防护)并
取其 `og:image`;结果会被记住,因此每个页面只读取一次。大屏从不直接
从新闻站点加载图片:由服务器在 `/api/wall/news/image/<id>` 中转,
只限它发布过的文章,最大 6 MB,并且仅当文件确实是 PNG、JPEG、GIF、
WebP 或 AVIF 图片时(绝不包括 SVG)。显示器的地址和 Cookie 不会外泄,
大屏的内容安全策略仍然只允许来自 DumbMonit 的图片。

### 选择来源 {#choosing-sources}

选择一个**国家/地区**(根据你浏览器的地区和语言预先选定),然后
勾选它的来源:总共最多八个订阅源。若要使用其他任何订阅源,请在
**添加自定义订阅源**下粘贴其 RSS 或 Atom 地址并点击**测试**:
DumbMonit 会真正读取它,并在你添加之前显示最新的一条头条。

对于视频,可以选择该国家/地区的某个频道,或选择**其他链接…**并
粘贴一个普通的 YouTube、Dailymotion 或 Twitch 地址(视频、直播,
或 YouTube 频道的 `/live` 页面);DumbMonit 会把它转换为可嵌入的
播放器。只能显示这三种播放器。只有当频道有稳定的官方直播时,才
会作为预设提供。

| 国家/地区 | 头条新闻 | 直播视频 |
|---|---|---|
| 法国 | BFM TV, franceinfo, Le Monde, France 24 | BFM TV, franceinfo, France 24, Euronews |
| 瑞士 | RTS Info, SRF News, Le Temps, NZZ | 无 |
| 比利时 | RTBF, VRT NWS, La DH | 无 |
| 德国 | Tagesschau, ZDFheute, Spiegel, Deutsche Welle | DW News |
| 英国 | BBC News, The Guardian | Sky News |
| 美国 | NPR, PBS NewsHour, CBS News, The New York Times | ABC News Live |
| 加拿大 | Radio-Canada, CBC News | CBC News |
| 西班牙 | RTVE, 20minutos, El País, El Mundo | 无 |
| 意大利 | ANSA, RaiNews | 无 |
| 葡萄牙 | RTP Notícias, Público | 无 |
| 巴西 | G1, Folha de S.Paulo | CNN Brasil |
| 俄罗斯 | Interfax, Meduza, Kommersant | 无 |
| 中国 | CGTN, South China Morning Post, Xinhua | CGTN |
| 国际 | Deutsche Welle, France 24, BBC News (world) | DW News, France 24 English, Euronews English |

BFM TV 的订阅源是 `https://www.bfmtv.com/rss/news-24-7/`。该列表在编写时
经过检查;站点随时可能更换其订阅源,届时滚动条会直接跳过它。请
**测试**替代的地址,并将其作为自定义订阅源添加。

大屏的内容安全策略只允许页面嵌入 YouTube(`youtube-nocookie.com`)、
Dailymotion、Twitch、Spotify 和 Deezer 的播放器。

### API {#news-api}

`GET /api/wall/news`(大屏轮询的接口)、`GET|PUT /api/wall/news/settings`
和 `POST /api/wall/news/test`(管理员),`GET /api/wall/news/presets`,
以及 `GET /api/wall/news/image/{id}`(中转的文章图片)。

## 大屏上的小组件 {#widgets-on-the-wall}

管理员可以在**设置 → 大屏显示 → 大屏小组件**中为大屏添加**小组件**:
天气、加密货币价格、股票行情、汇率和最新的严重漏洞。每一个默认都
是关闭的,需要单独开启。

**天气**保持简洁:只显示一个图标和当前温度,针对一个地点,就在时钟
正下方。没有预报,没有细节。屏幕阅读器会朗读地点、天空状况和温度;
悬停时会显示同样的文字以及来源(Open-Meteo)。

其他小组件共用右栏时钟下方的一张卡片:一次一个小组件,每个停留所
选的秒数(默认 20 秒),然后下一个淡入。较长的列表会拆成每页五项。
卡片下方的圆点表示轮播进行到哪里。启用*减少动态效果*时,卡片直接
切换,不做淡入淡出。

涨跌始终带有箭头和符号(▲ +1.04 %,▼ −0.57 %),而不仅仅依靠颜色。
每张卡片都会低调地注明来源及数据的新鲜程度。

| 小组件 | 显示内容 | 来源 | 密钥 | 服务器请求频率 |
|---|---|---|---|---|
| 天气 | 一个地点当前的图标和温度,显示在时钟下方 | [Open-Meteo](https://open-meteo.com)(当前状况和地理编码) | 无 | 每 15 分钟 |
| 加密货币 | 最多十种币的价格及 24 小时涨跌,以所选货币计价 | [CoinGecko](https://www.coingecko.com) 公共 API | 无(可选的免费 Demo 密钥) | 每分钟一次调用 |
| 股票 | 最多十个代码的最新价格和当日涨跌 | [Finnhub](https://finnhub.io) | **需要免费密钥** | 每 5 分钟,每个代码一次调用 |
| 汇率 | 一个单位(小面额货币为 100 个单位)折合基准货币多少,以及相对前一天的变化 | [Frankfurter](https://frankfurter.dev)(央行参考汇率) | 无 | 每小时 |
| 严重漏洞 | 本周评级为严重(CVSS 9 或以上)的最新 CVE,和/或 CISA 已被积极利用漏洞目录中的最新条目,附带评分和日期 | [NVD](https://nvd.nist.gov) API 2.0、[CISA KEV](https://www.cisa.gov/known-exploited-vulnerabilities-catalog) | 无 | 每小时 |

**来源及其条款**(2026 年 10 月核查):

- **Open-Meteo** 对非商业用途免费且无需密钥(每天最多 10,000 次调用),
  其数据采用 CC BY 4.0 许可。
- **CoinGecko** 的免密钥 API 在每个地址上共享大约每分钟 10 到 30 次
  调用的额度,不适合高频轮询;DumbMonit 对所有大屏每分钟只请求一次。
  免费的 **Demo 密钥**(coingecko.com)可为该实例提供独立的速率限制。
- **股票**需要一个免费的 **Finnhub** 密钥:没有任何免密钥的行情来源
  既可靠又被允许(Stooq 现在要求密钥,Yahoo 禁止使用其未公开的
  API)。免费套餐涵盖美国上市股票,允许每分钟 60 次调用;行情可能
  有延迟。
- **Frankfurter** 无需密钥也没有配额;汇率每个工作日更新一次。
- **NVD** 在没有密钥时允许每 30 秒 5 次请求;DumbMonit 每小时只发出
  一次。*本产品使用 NVD API,但未获得 NVD 的认可或认证。* **CISA
  KEV** 目录是一个公开的 JSON 文件。

**一切都经过服务器。** 没有任何显示器会直接访问这些服务:无论是它的
地址,还是这些服务的限额,都不取决于屏幕的数量。每个来源都有自己的
缓存。当某个来源失败时,卡片会保留其上一次的数字最多六小时,并标注
*已过期*,服务器会等待一分钟再重新请求;如果没有任何内容可显示,卡片
会显示*不可用*。大屏绝不会因为某个小组件而出问题。

**API 密钥绝不会回传。** 它们用实例密钥加密保存;设置中只显示是否已
保存。输入新密钥会替换它,**移除密钥**会清除它。

### 选择显示内容 {#choosing-what-to-show}

- **天气:**输入城市名并点击**搜索**(Open-Meteo 的地理编码,使用你的
  语言),然后选择地点(选择另一个会替换它)。单位:°C 或 °F。
- **加密货币:**在主要币种(BTC、ETH、SOL、XRP、BNB、ADA、DOGE、TRX、
  TON、AVAX、DOT、LINK、LTC、XLM、XMR、USDT、USDC)中勾选,或按
  CoinGecko 的 id 添加任意币种(即 coingecko.com 上其地址的最后一部分,
  例如 `avalanche-2`)。显示货币:USD、EUR、GBP、CHF、JPY 等。
- **股票:**输入股票代码,以逗号分隔(`AAPL, MSFT, NVDA`),并粘贴你的
  Finnhub 密钥。
- **汇率:**选择基准货币,以及主要货币中最多十种其他货币(USD、EUR、
  GBP、JPY、CHF、CNY、CAD、AUD……)。
- **严重漏洞:***两者*(严重且已被利用;已在 NVD 列表中的被利用 CVE 会
  标注*已被利用*)、*严重*(NVD)或*已被积极利用*(CISA KEV),以及
  显示的数量(1 到 10)。

#### 如何获取管理员密钥 {#ai-admin-keys}

两家提供商都需要**管理员密钥**，它不同于普通的 API 密钥，只用于读取组织的用量和费用。请妥善保管：把它粘贴到 **Settings → Wall display → Wall widgets**，DumbMonit 会加密存储。

##### Claude (Anthropic) {#ai-admin-key-claude}

1. 以组织的 **admin** 角色成员身份登录 [Claude Console](https://platform.claude.com/settings/admin-keys)（个人账户无法创建；请先在 Settings → Organization 中创建组织）。
2. 打开 [Admin keys 页面](https://platform.claude.com/settings/admin-keys)，点击 **Create admin key**，填写名称并选择有效期。
3. 立即复制密钥（`sk-ant-admin01-…`）：它只显示一次。

##### OpenAI {#ai-admin-key-openai}

1. 以组织**所有者**（owner）身份登录 [OpenAI 平台](https://platform.openai.com/settings/organization/admin-keys)。
2. 打开 [Admin keys 页面](https://platform.openai.com/settings/organization/admin-keys)（Organization settings → Admin keys），点击 **Create new admin key**；*Read only* 权限即可。
3. 立即复制密钥：它只显示一次。

个人订阅（Claude Pro 和 Max、ChatGPT Plus）**没有用量 API**，无法显示在大屏上。

### API {#widgets-api}

`GET /api/wall/widgets`(大屏每分钟轮询的接口:每个已开启的小组件对应
一个部分,含 `data`、`updated`、`error`、`needs_key`、`source`)、
`GET|PUT /api/wall/widgets/settings` 和 `GET /api/wall/widgets/places?q=`
(管理员),以及 `GET /api/wall/widgets/catalog`。在 `PUT` 中,
`secrets.finnhub` 和 `secrets.coingecko` 用于设置密钥,空字符串会清除
密钥,省略则保持不变。

## 大屏上的音乐 {#music-on-the-wall}

大屏还可以播放音乐并显示正在播放的内容,因此你可以从手机启动
一个播放列表,在电视上听到它,并在房间的另一端看到它。音乐始终
位于天气窗口的右下角,叠在天空图案之上:它绝不会占用简报或
*需要处理*列表的空间,没有任何内容播放时它会消失。每张卡片会
折叠成一个小药丸形(显示器会记住这个状态)。

共有两种方式,两者都可以同时使用。

| | Spotify Connect | 发送给各大屏的链接 |
|---|---|---|
| 从哪里启动 | 你手机上的 Spotify 应用(**设备 → DumbMonit Wall**) | 任意一个 DumbMonit 屏幕,包括你的手机(**在大屏上播放**) |
| 支持的服务 | Spotify | Spotify、Deezer、YouTube 和 YouTube Music |
| 需要 | Spotify Premium、你自己的 Spotify Developer 应用、与手机使用同一个 Spotify 账户、大屏通过 HTTPS(或 localhost)在一个支持 DRM 的浏览器中提供服务 | 不需要任何东西;如果没有在大屏上登录,Spotify 和 Deezer 会播放 30 秒的预览片段 |
| 是否显示正在播放的内容 | 是,无论在哪里播放:大屏、电视的 Spotify 应用、投屏音箱 | 该服务自己的播放器 |

连接 Spotify 和发送链接都是管理员操作;大屏上的查看者账户能
播放和显示一切。

### Spotify Connect:大屏变成一个音箱 {#spotify-connect-the-wall-becomes-a-speaker}

目标:在你的手机上打开 Spotify,点击设备图标,选择那台电视——
音乐会在电视上播放,而大屏会显示正在播放的内容。大屏是通过
Spotify 的**Web Playback SDK**实现这一点的:大屏的浏览器会把
自己注册为一个 Spotify Connect 设备。只有**全部**以下条件都
满足时才能工作,大屏会在自己的屏幕上说明缺少了哪一项:

| 需要 | 原因 | 缺失时的表现 |
|---|---|---|
| 已连接账户拥有**Spotify Premium** | Spotify 只允许 Premium 账户使用浏览器音箱 | 大屏显示*需要 Spotify Premium*;设置中显示*Premium:No* |
| **手机登录的是同一个 Spotify 账户** | 一个浏览器音箱只会出现在持有其令牌的那个账户下——它不像智能音箱那样在网络上被发现,家庭套餐的其他成员也看不到它 | 手机上根本不会列出该大屏 |
| **HTTPS**(或显示器自身上的 `http://localhost`) | 浏览器只在安全页面中允许受保护的媒体内容(DRM) | 大屏显示*以纯 HTTP 打开* |
| **一个 DRM 可用的浏览器**(Widevine、PlayReady 或 FairPlay) | Spotify 传输加密的音频流 | 大屏显示*该浏览器无法播放 Spotify* |
| **大屏每次加载页面后需要一次点击或按键** | 浏览器会在有人与页面交互之前保持静音 | 大屏显示*点击以启用声音* |

哪些浏览器可用:电脑上的 Chrome、Edge 和 Firefox(Firefox 需要
允许*播放受 DRM 保护的内容*)、Mac 上的 Safari、Raspberry Pi 上
带有 Widevine 的 Chromium(`libwidevinecdm0`)。大多数**智能
电视浏览器**(三星 Tizen、LG webOS)、**信息屏浏览器**以及许多
**Linux Chromium 构建版本**都没有可用的 Widevine:无论 DumbMonit
怎么做,它们都无法成为一个 Spotify 音箱。对于这类电视,参见
[当电视的浏览器无法播放 Spotify 时](#when-the-tvs-browser-cannot-play-spotify)。

#### 设置步骤 {#set-it-up}

1. **创建一个 Spotify 应用。** 打开
   [Spotify Developer Dashboard](https://developer.spotify.com/dashboard),
   创建一个应用(任意名称),勾选**Web API**和**Web Playback
   SDK**,并添加**设置 → 大屏音乐**显示的重定向 URI:
    - DumbMonit 通过 HTTPS 提供服务:
      `https://<你的地址>/api/music/spotify/callback`。你批准后
      Spotify 会直接跳转回 DumbMonit。
    - DumbMonit 通过纯 HTTP 提供服务:
      `http://127.0.0.1:8888/callback`。自 2025 年起,Spotify
      只接受回环地址(而不是 `localhost`)使用 `http://`。该
      地址上没有任何东西会应答:你批准后,浏览器会显示一个错误
      页面,其地址中带有验证码——复制它并粘贴回 DumbMonit。
2. **连接。** 在**设置 → 大屏音乐**中,粘贴该应用的**Client
   ID**(没有密钥:DumbMonit 使用带 PKCE 的 Authorization
   Code),选择**连接 Spotify**,然后在 Spotify 中**使用你手机
   所用的账户**批准。
3. **在电视上**,通过 HTTPS 打开 DumbMonit 的 `/wall`,并登录
   (一个查看者账户就够了),点击屏幕或在遥控器上按一次**OK**:
   任何手势都能解锁声音。以
   `--autoplay-policy=no-user-gesture-required` 启动的浏览器
   (信息屏模式)不需要点击。
4. **在你的手机上**,打开 Spotify 应用,点击设备图标,选择
   **DumbMonit Wall**。声音会从电视上播出,大屏会显示封面、
   标题和进度。

**设置 → 大屏音乐 → 音箱**可以在不走到电视前的情况下查看这一切
是否就位:账户、Premium、Spotify 当前是否在该账户的设备列表中
列出了该音箱(以及列出了哪些设备),以及最近十五分钟内上报过的
每个大屏——它的浏览器("Linux 上的 Chrome 130")、是否就绪、
如果不是则是为什么。**测试声音**会让该账户在该音箱上播放(会
恢复上次播放的内容);每个就绪的大屏也各自有自己的**测试声音**。
在大屏本身上,**音乐**面板有**在此播放**,可以把该账户当前播放
的内容转移到这台显示器上。

**名称。** 在**设置 → 大屏音乐 → 音箱**中重命名该音箱("客厅
电视");每个大屏会在几秒钟内获知新名称。若要单独命名某一个
大屏,请在其地址后添加 `?speaker=Kitchen`——显示器会记住它
(`?speaker=` 空值会遗忘它)。

**大屏可以连续开启数天。** 当 Spotify 的令牌过期时(每小时一次),
大屏会向 DumbMonit 请求一个新令牌;当该设备掉线(Wi-Fi、休眠的
网卡)且三十秒内没有恢复时,大屏会重新注册它;一次偶发的失败会
自行重试(15 秒、30 秒……然后每五分钟一次);如果 Spotify 不再
列出该大屏、而它自认为是就绪的,它会重新注册。只有等待无法修复
的问题——没有 DRM、没有 Premium——才会永久停止,直到点击**重试**。

DumbMonit 会用实例密钥加密保存 Spotify 的刷新令牌,绝不会把它
发送给浏览器。大屏只会收到一个用于其播放器的短期有效访问令牌,
而且只能通过一个已登录的会话获得(API 令牌无法获得它)。
**断开连接**会遗忘该账户;若要同时从你的 Spotify 账户中移除
DumbMonit,请使用
[spotify.com/account/apps](https://www.spotify.com/account/apps/)。

还值得了解:

- 一个个人 Spotify 应用(*开发模式*)属于一个 Premium 账户,最多
  允许五个账户使用:可在该控制面板的**User Management**下添加
  其他账户。
- **每六个月重新连接一次。** Spotify 会在批准六个月后结束一次
  连接;设置中会显示该日期,大屏也会在到期时说明这一点。

**正在播放功能不需要以上任何条件。** DumbMonit 的服务器每隔几秒
会向 Spotify 查询该账户正在播放的内容(所有大屏共用一次请求),
大屏会显示封面、标题、艺术家、进度以及正在播放的设备——大屏
本身、某台智能电视的 Spotify 应用、一个 Chromecast、你的
笔记本电脑——即使是在纯 HTTP 下,以及在无法播放 Spotify 的
浏览器中也是如此。若只想使用这项功能,请在大屏的音乐面板中关闭
**将此显示器用作 Spotify 音箱**。

#### 当电视的浏览器无法播放 Spotify 时 {#when-the-tvs-browser-cannot-play-spotify}

选择与你的设置相符的方式——无论哪种情况,大屏都会继续显示正在
播放的内容,因为*正在播放*读取的是账户信息,而不是音箱本身:

- **电视自带 Spotify 应用,或连接了 Chromecast / Google TV /
  Fire TV 棒。** 在 Spotify 的设备列表中选择它;大屏会显示
  "正在客厅电视上播放"。这是最简单、最可靠的选项。
- **一台小型计算机驱动着电视**(Raspberry Pi、迷你 PC、旧
  笔记本电脑)。用带有 Widevine 的 Chrome 或 Chromium 打开大屏:
  它会像上文描述的那样成为音箱。
- **你想要一个始终在那里、即使没有浏览器也能用的音箱。** 在
  **声音输出能到达电视或音箱的那台机器上**,运行一个基于
  [librespot](https://github.com/librespot-org/librespot)
  (MIT)的 Spotify Connect 接收器——例如
  [raspotify](https://github.com/dtcooper/raspotify) 或
  spotifyd。它会出现在网络上任何人手机的设备列表中。

DumbMonit 本身故意不附带这样的接收器:DumbMonit 容器通常运行在
壁橱里的某台服务器上,而不是插在电视上的那台机器,因此它的声音
会从错误的设备发出;一个接收器需要宿主的声卡和宿主网络
(多播 DNS),而一个监控容器不应该拥有这些;而且 librespot 依赖
非官方的 Spotify 协议,时常会失效。在正确的机器上、与
DumbMonit 分开安装一个接收器,效果会更好。

### 发送给每个大屏的链接 {#a-link-sent-to-every-wall}

在**设置 → 大屏音乐**(或大屏自己的**音乐**面板)中,粘贴一个
Spotify、Deezer 或 YouTube 的曲目、专辑、播放列表、单集或视频
链接,然后选择**在大屏上播放**。每个大屏都会用该服务官方的嵌入
播放器播放它;在大屏打开期间发送的链接,会在该大屏被点击一次后
在该服务允许自动播放的地方(YouTube、Deezer)自行开始播放。
**停止**会把它从每个大屏上移除。

Deezer 和 YouTube 没有网页可用的 Spotify Connect 等价物——
Deezer 已不再接受新的开发者应用,其 API 也从未公开过正在播放的
内容——因此对它们来说,链接就是唯一的方式。除非大屏的浏览器已
登录该服务,否则嵌入播放器只会播放 30 秒的预览。

### 隐私 {#privacy}

在 Spotify 连接或设置了链接之前,大屏上不会加载任何第三方内容。
该页面的内容安全策略恰好只允许这三种嵌入播放器、Spotify 的
播放器脚本(`sdk.scdn.co/spotify-player.js`)及其框架,以及来自
`i.scdn.co` 的内容——没有其他任何内容,也不会超出网页界面本身。

### API {#api}

| 路由 | 权限 | 作用 |
|---|---|---|
| `GET /api/music/now` | 任意会话或令牌 | 该 Spotify 账户正在播放的内容(缓存几秒钟)以及各大屏正在播放的链接 |
| `PUT /api/music/link` `{"link": "…"}` | 管理员 | 每个大屏播放这个链接 |
| `DELETE /api/music/link` | 管理员 | 各大屏停止嵌入播放器 |
| `GET /api/music/spotify` | 任意会话或令牌 | 已连接的账户:状态、Client ID、需要重新连接的日期——绝不包含令牌 |
| `POST /api/music/spotify/authorize` | 管理员会话 | 启动一次连接:`{"client_id", "redirect_uri"}` → Spotify 的批准 URL |
| `POST /api/music/spotify/complete` `{"url": "…"}` | 管理员会话 | 用浏览器最终到达的地址完成一次回环连接 |
| `GET /api/music/spotify/callback` | 管理员的浏览器 | Spotify 的直接重定向(HTTPS 安装) |
| `DELETE /api/music/spotify` | 管理员 | 断开连接并遗忘各令牌 |
| `GET /api/music/spotify/token` | 仅限浏览器会话 | 用于大屏播放器的短期访问令牌 |
| `GET /api/music/speaker` | 任意会话或令牌 | 该音箱:名称、Premium、该账户的 Spotify Connect 设备、该音箱是否被列出、各大屏上报的内容 |
| `PUT /api/music/speaker` `{"name": "…"}` | 管理员 | 重命名该音箱(`null` 恢复为"DumbMonit Wall") |
| `POST /api/music/speaker/report` | 仅限浏览器会话 | 一个大屏上报其音箱状态(每分钟发送一次);应答会说明 Spotify 是否列出了它 |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | 仅限浏览器会话 | 在该设备上播放该账户的内容,或在与该音箱同名的设备上播放("测试声音"、"在此播放") |
