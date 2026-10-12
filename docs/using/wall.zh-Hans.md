# 大屏模式与音乐 {#wall-mode-and-music}

`/wall` 会以全屏方式单独显示简报,适用于房间中一直保持开启的
屏幕:那句话、天气窗口、三个大数值读数以及*需要处理*列表,每
20 秒刷新一次,并让屏幕保持常亮。++esc++ 或点击**退出**会返回
概览页面。可从命令面板("大屏模式")打开,也可直接输入该 URL。
`?theme=dark` 或 `?theme=light` 可以为该显示器强制指定一种主题,
而不影响你已保存的偏好设置。

![大屏模式](../assets/screenshots/wall-light.png){ loading=lazy }

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

### API

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
