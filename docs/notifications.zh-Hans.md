# DumbMonit通知 {#dumbmonit-notifications}

该指南解释如何在你选择的服务上接收DumbMonit警报。它不需要编程技能：每部分说哪里找地址或令牌来复制，以及在表中输入什么。

---

## 目录 {#contents}

- [它如何工作](#how-it-works)
- [创建和测试通道](#creating-and-testing-a-channel)
- [设置或密钥：区别](#settings-or-secrets-the-difference)
- [自签证书](#self-signed-certificates)
- [自定义通道](#the-custom-channel)
  - [可用变量](#available-variables)
  - [示例1——内部JSON API](#example-1--an-internal-json-api)
  - [示例2——期望表单的服务](#example-2--a-service-that-expects-a-form)
  - [示例3——通过运营商的纯文本SMS](#example-3--a-plain-text-sms-through-a-carrier)
  - [示例4——没有主体的GET请求](#example-4--a-get-request-without-a-body)
  - [通用模板错误](#common-template-mistakes)
- [支持的通道](#supported-channels)
- [故障排除](#troubleshooting)

---

## 它如何工作 {#how-it-works}

一个**通道**是一个目的地：一个Discord通道、一个电话号码、一个邮箱。你创建尽可能多如你想要，你的警报规则选择哪些要通知。

三个原则应用到每个通道：

1. **警报被分组。** 每个监控项每个周期一条消息，即使五个磁盘同时填满。没有它，你会在一周内使通知沉默。
2. **一个破损的通道从不阻止另一个。** 失败记录在通道的"最后错误"列，并继续到其他的。
3. **一个不回答的服务在15秒后被放弃。** 一个迟到一分钟到达的通知不再有用，监控循环必须不等在不可到达的服务后。

---

## 创建和测试通道 {#creating-and-testing-a-channel}

1. 打开**设置→通知与报告→添加通道** (`/settings/notifications`)。
2. 给它一个有意义的名称（"Discord——家人"、"SMS——待命"）。这是你稍后在你的警报规则中选择的名称。
3. 选择通道类型，然后填写下面描述的字段。
4. 保存，然后点击**发送测试**。

测试消息取**完全相同的路径**如真实警报：相同的请求、相同的令牌、相同的格式。如果它到达，通道工作。如果它失败，错误消息命名要修复的设置。

> **令牌从不再显示。** 一旦保存，它在数据库中加密，界面仅说"已配置"。为了编辑通道不重输其令牌，仅留下字段空。

---

## 设置或密钥：区别 {#settings-or-secrets-the-difference}

每个通道有两组字段。

| | **设置** | **密钥** |
|---|---|---|
| 内容 | 服务器地址、通道名称、主题 | 令牌、密码、webhook URL |
| 存储 | 纯文本 | 加密 |
| 由界面读回 | 是 | 从不 |

界面**拒绝**保存设置间的敏感字段，并告诉你。这不是形式：设置由API如同返回，所以那里存储的令牌会在页面每次打开时发布。

[支持的通道](#supported-channels)部分中的表，对于每个通道，说什么去哪里。

---

## 自签证书 {#self-signed-certificates}

默认DumbMonit拒绝任何它无法验证其证书的服务器。对于一个自托管服务用自签或私有机构的证书提供，Matrix、Mattermost、Rocket.Chat、Zulip、ntfy、Gotify、Bark、Apprise、Home Assistant、Signal和自定义webhook的通道有一个**接受不可验证的证书** 设置 (`insecure_tls`，一个布尔值，默认关闭）。

打开它保留连接加密但停止检查服务器的身份，所以仅对你控制的服务器，在你信任的网络上使用它。当证书是失败的原因时，为通道显示的错误命名它（例如"无效对等证书：UnknownIssuer"）。

---

## 自定义通道 {#the-custom-channel}

**`webhook`** 通道是DumbMonit的逃生舱。你描述整个HTTP请求——方法、地址、标头、内容类型和主体——，这让你插入几乎任何服务，包括一个DumbMonit永不知道的内部API。

**不有任何特定配置**，它已经发送一个结构化JSON有效载荷一个脚本可以使用：

```json
{
  "source": "dumbmonit",
  "target": "nas",
  "rule": "Disk full",
  "severity": "warning",
  "status": "firing",
  "title": "nas — Disk full",
  "text": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "value": 95.0,
  "threshold": 90.0,
  "unit": "%",
  "fingerprint": "nas/disk-full",
  "count": 1,
  "at": "2026-09-01T14:12:05+00:00",
  "link": ""
}
```

### 自定义通道设置 {#custom-channel-settings}

| 设置 | 必需 | 默认 | 角色 |
|---|---|---|---|
| `url` | 是（此处或作为密钥） | — | 要调用的地址。可能包含变量。 |
| `method` | 否 | `POST` | `POST`、`PUT`或`GET`。 |
| `content_type` | 否 | `json` | `json`、`form`或`text`。也决定变量如何被逃脱。 |
| `body_template` | 否 | — | 主体模板。当缺少时，上面的有效载荷被发送。 |
| `base_url` | 否 | — | 你的DumbMonit的公共地址，所以 `{{link}}` 给出可点击的链接。 |
| `username` | 否 | — | HTTP"Basic"认证的用户名。需要 `password` 密钥。 |

| 密钥 | 角色 |
|---|---|
| `url` | 如果地址已包含令牌，使用它而非 `url` 设置。 |
| `token` | 作为 `Authorization: Bearer …` 标头发送，除非你的模板使用 `{{token}}`。 |
| `password` | "Basic"认证的密码。 |
| `headers` | 额外标头，作为JSON对象 `{"X-Api-Key": "value"}`。存储加密如令牌且从不再显示，所以API密钥和 `Authorization` 标头属于这里。 |

`headers` 是密钥。0.1.0-alpha.6或更早保存的通道，将它们保留在设置中，在首次启动时将它们移到加密密钥，API不再返回它们。如每个密钥，它们在你编辑通道不输入它们时被保留；发送 `secrets` 替换整个集合。

DumbMonit仅追踪重定向到同一主机（或在该主机上从 `http` 到 `https`）：通道的标头和令牌永不去任何地方但你输入的地址。

### 可用变量 {#available-variables}

一个变量写在**双花括号**之间：`{{title}}`。围绕名称的空格被容忍（`{{ title }}`）。JSON主体中的单花括号保持文字，所以你可以正常写你的JSON。

| 变量 | 它包含什么 | 示例 |
|---|---|---|
| `{{title}}` | 完整消息标题 | `nas — Disk full` |
| `{{message}}` | 纯文本主体，每个警报一行 | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | 相同主体在Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | 警报规则的名称 | `Disk full` |
| `{{target}}` | 监控监控项的名称 | `nas` |
| `{{target_id}}` | 监控项的数字标识符 | `42` |
| `{{severity}}` | 技术严重程度 | `info`、`warning`或`critical` |
| `{{severity_fr}}` | 严重程度作为可读单词 | `information`、`avertissement`、`critique` |
| `{{status}}` | 技术状态 | `firing`或`resolved` |
| `{{status_fr}}` | 状态作为可读单词 | `en cours`或`résolu` |
| `{{value}}` | 测量值，单位包括 | `95 %` |
| `{{value_raw}}` | 赤裸值，作为JSON**数字**可用 | `95` |
| `{{threshold}}` | 交叉阈值，单位包括 | `90 %` |
| `{{threshold_raw}}` | 赤裸阈值，作为JSON**数字**可用 | `90` |
| `{{unit}}` | 值的单位 | `%` |
| `{{operator}}` | 阈值比较 | `>` |
| `{{count}}` | 该消息中分组的警报数 | `3` |
| `{{fingerprint}}` | 警报的稳定标识符 | `nas/disk-full` |
| `{{timestamp}}` | ISO 8601时间戳 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | 时间戳以秒自1970 | `1788358325` |
| `{{date}}` | 可读时间戳，在UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | 优先级从1（低）到5（高） | `4` |
| `{{color}}` | 强调颜色，不有散列 | `D98A00` |
| `{{color_hex}}` | 强调颜色，有散列 | `#D98A00` |
| `{{emoji}}` | 状态象形文字 | `🔴`、`⚠️`、`ℹ️`或`✅` |
| `{{link}}` | DumbMonit中的监控项链接 | `https://dumbmonit.home/targets/42` |
| `{{source}}` | 总是 `dumbmonit` | `dumbmonit` |
| `{{token}}` | 通道的 `token` 密钥 | *（你的令牌）* |

两个有用的评注：

- **一些变量可以是空的。** 一个可用性警报（"主机无法到达"）有值和阈值都没有：`{{value}}` 和 `{{value_raw}}` 然后渲染空字符串，永不 `null` 或 `NaN`。如果你作为JSON数字放它们，处理那情况——或使用 `"{{value}}"` 在引号间。
- **`{{link}}` 保持空**直到你填入 `base_url`。错误的链接会浪费比没有一个的警报更多的时间。

**你永不需要逃脱任何东西自己。** 根据选择的 `content_type`，DumbMonit自动保护替代值：在JSON中逃脱引号和换行符、在表中百分比编码、纯文本无变换。一个命名 `"System" disk full` 的规则不会打破你的JSON模板。

### 示例1——内部JSON API {#example-1--an-internal-json-api}

一个自创应用暴露 `POST /incidents` 并期望一个令牌在自定义标头。

**设置**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**密钥**

```json
{
  "token": "your-application-token",
  "headers": "{\"X-Application\": \"dumbmonit\"}"
}
```

令牌自动作为 `Authorization: Bearer …` 标头发送。这是应用接收的：

```json
{
  "title": "nas — Disk full",
  "device": "nas",
  "severity": "warning",
  "state": "firing",
  "value": 95,
  "threshold": 90,
  "detail": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "link": "https://dumbmonit.home/targets/42",
  "seen_at": "2026-09-01T14:12:05+00:00"
}
```

### 示例2——期望表单的服务 {#example-2--a-service-that-expects-a-form}

许多SMS网关和更旧的API期望 `application/x-www-form-urlencoded` 而非JSON，身体中有令牌。

**设置**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**密钥**

```json
{ "token": "gateway-key" }
```

由于模板已使用 `{{token}}`，DumbMonit**不添加** `Authorization` 标头：一些服务拒绝携带两个认证的请求。值自动被编码，空格变为 `+` 和重音字符其百分比编码等价物。

### 示例3——通过运营商的纯文本SMS {#example-3--a-plain-text-sms-through-a-carrier}

一些服务，如运营商的SMS API或自创自动化，想一行文本。

**设置**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

消息发送，不有任何逃脱：

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### 示例4——没有主体的GET请求 {#example-4--a-get-request-without-a-body}

当服务仅接受URL参数时，将变量放在 `url` 并选择 `GET` 方法。一个 `GET` 请求没有主体：`body_template` 然后被拒绝，所以没有人相信它做某事。

**设置**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**密钥**

```json
{ "token": "api-key" }
```

仅**替代值**被编码；地址的其余被如同复制。请求实际上看起来发送：

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### 通用模板错误 {#common-template-mistakes}

一个有故障的模板当你在表前看的时候被**保存时拒绝**，不在事件发生的夜晚。

| 你写什么 | 发生什么 |
|---|---|
| `{{severite}}` | 拒绝：未知变量"severite"，跟随有效名称的列表。正确名称是 `severity`。 |
| `{"t": "{{title"}` | 拒绝："未关闭的花括号"。 `}}` 缺失。 |
| `{{}}` 或 `{{  }}` | 拒绝：不有名称的变量意味着无。 |
| `{"a": {"b": 1}}` | **接受。** 单花括号是文字：正常写你的JSON。 |
| `{{token}}` 不有 `token` 密钥 | 拒绝：模板要求一个通道无的令牌。 |
| `method: GET` 有 `body_template` | 拒绝，有替代：将变量放在 `url`。 |
| `method: DELETE` | 拒绝：仅 `POST`、`PUT`和`GET`被接受。 |

---

## 支持的通道 {#supported-channels}

### Discord {#discord}

在Discord：**通道设置→集成→Webhooks→新webhook**，然后**复制webhook URL**。该URL包含令牌：它去密钥。

- **密钥** — `webhook_url`

### Slack {#slack}

在 <https://api.slack.com/apps>，创建一个应用，启用**传入Webhooks**，然后**添加新Webhook到工作空间**并选择通道。复制 `https://hooks.slack.com/services/…` URL。

- **密钥** — `webhook_url`

### Telegram {#telegram}

你需要两件东西：**机器人令牌**（密钥）和**聊天ID** (`chat_id`) 其中它必须写。第一个需要一分钟得到；第二取决于你想接收警报的位置。

#### 1. 创建机器人并得到其令牌 {#1-create-the-bot-and-get-its-token}

1. 在Telegram，搜索**@BotFather**并点击*开始*。
2. 发送 `/newbot`，然后回答问题（一个名称，然后一个用户名必须以 `bot` 结尾）。
3. BotFather以这样的消息结束：

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   行 `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` 是**机器人令牌**。不分享它：它给完整控制的机器人。

在下面的示例中，用该令牌替换 `<TOKEN>`。注意 `bot` 前缀粘在URLs中它前面：`https://api.telegram.org/bot<TOKEN>/…`。

#### 2. 找 `chat_id` {#2-find-the-chat_id}

**私有聊天**（机器人直接写给你）

1. 打开你的机器人 (`t.me/my_homelab_bot`) 并点击*开始*，或发送它任何消息。不有这个，它永不能写给你。
2. 在浏览器中打开：`https://api.telegram.org/bot<TOKEN>/getUpdates`
3. 你得到一个JSON文件；寻找 `"chat"`：

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   `chat_id` 是 `chat.id` 的值：这里 `21xxxxx38` （一个正数）。

   如果 `result` 是空 (`"result":[]`)，发送机器人另一条消息和重载页面。

**通道**

1. 作为通道管理员添加机器人（管理通道→管理员→添加）。
2. 在通道中发布消息。
3. 打开 `https://api.telegram.org/bot<TOKEN>/getUpdates`：响应包含一个 `channel_post` 块有 `"chat":{"id":-1001xxxxxx062,"type":"channel"}`。

   `chat_id` 是那个负数，`-100` 包括：`-1001xxxxxx062`。

**组**

最简单的方式通过桌面应用：

1. 添加机器人到组。
2. 在组中发送消息，右击它→*复制消息链接*。你得到 `https://t.me/c/194xxxx987/13` （或 `https://t.me/c/194xxxx987/11/13` 如果组有主题）。
3. 数字在 `/c/` 右侧是组的内部标识符：`194xxxx987`。 `chat_id` 要输入的是那个数字**前缀有 `-100`**：`-100194xxxx987`。

不有桌面应用，上面的 `getUpdates` 方法也工作：`message.chat.id` 直接是正确数字（已负）。如果机器人看不到组的消息，向BotFather要 `/setprivacy` →*禁用*，然后从组中删除机器人并再次添加。

**组主题**（"主题"模式中的组）

`chat_id` 是组的。此外，填入**组主题** (`message_thread_id`) 用复制的链接的中间数字：在 `https://t.me/c/194xxxx987/11/13`，主题是 `11`。不有该设置，消息在*一般*线程中降落。

#### 3. 保存前检查 {#3-check-before-saving}

在浏览器中打开：

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

（为主题添加 `&message_thread_id=11`）。如果 `test123` 到达，令牌和 `chat_id` 是对的：在DumbMonit中输入它们并点击*测试*。

- **设置** — `chat_id`（必需）、`message_thread_id`（主题，可选）、`api_base`（默认 `https://api.telegram.org`，仅改变通过中继）
- **密钥** — `bot_token`

> **"Telegram无法找到该聊天"** ——对私有聊天，机器人从你永不收到消息：发送它 `/start`。对组或通道，`-100` 前缀可能缺失，或机器人没被添加到。
>
> **"线程未找到"** ——`message_thread_id` 不匹配组的任何主题，或组不在"主题"模式。

### Microsoft Teams {#microsoft-teams}

**警告：该方法已改变。** 旧的"Office 365连接器"在2026年5月被退休；一个 `outlook.office.com/webhook/…` URL不再工作。

当前过程：

1. 在Teams通道，**⋯→工作流**。
2. 选择**"当webhook请求被接收时发布到通道"**模板。
3. 确认步骤，然后**复制显示在末尾的HTTPS POST URL**。

- **密钥** — `webhook_url`

DumbMonit发送一个被包装的*自适应卡*，唯一格式这些流正确显示。如果你得到提及Power Automate的错误，URL可能来自旧连接。

### Matrix {#matrix}

1. 在你的homeserver上创建一个专门用于DumbMonit的账户，并**邀请它进入房间**。不是成员的账户无法写。
2. 得到其访问令牌。在Element：**设置→帮助和关于→高级→访问令牌**。
3. 得到房间的**内部**标识符——不是其显示名称。在Element：**房间设置→高级→内部房间ID**。它开始有 `!` （例如 `!aBcDeF:matrix.org`）。

- **设置** — `server_url` （例如 `https://matrix.org`）、`room_id`
- **密钥** — `token`

### Mattermost {#mattermost}

**集成→传入Webhooks→添加**。选择默认通道，然后复制URL。

- **设置** — `channel`（可选，在默认通道以外的地方写）、`username`（可选，默认 `DumbMonit`）
- **密钥** — `webhook_url`

### Rocket.Chat {#rocketchat}

**管理→集成→新集成→传入webhook**。选择房间，启用集成，然后复制*Webhook URL*。

- **设置** — `channel`（可选）、`alias`（可选，默认 `DumbMonit`）
- **密钥** — `webhook_url`

### Google Chat {#google-chat}

在Google Chat空间：**空间名→应用和集成→Webhooks→添加webhook**。复制URL，其已包含密钥和令牌。

- **密钥** — `webhook_url`

### Zulip {#zulip}

1. 创建一个**机器人**：**个人设置→机器人→添加新机器人**，类型"Generic"。注意其电子邮件地址和API密钥。
2. 订阅机器人到目标流，否则发送会被拒绝。

- **设置** — `server_url`（例如 `https://your-org.zulipchat.com`）、`email`（机器人的地址）、`stream`（流名称）、`topic`（可选，默认 `DumbMonit`）
- **密钥** — `api_key`

### ntfy {#ntfy}

最简单的通道的所有：选择一个难猜测的主题名，然后从ntfy应用订阅它。

- **设置** — `topic`、`server_url`（可选，默认 `https://ntfy.sh`）
- **密钥** — `token` （仅如果你的服务器需要认证）

> 在公共实例上，**任何知道主题名的人可以读它**。选择一个长的、随机的名，或托管你自己的服务器。

### Gotify {#gotify}

在Gotify界面：**应用→创建应用**。复制显示的令牌。

- **设置** — `server_url` （例如 `https://gotify.home`）
- **密钥** — `token`

### Pushover {#pushover}

1. 在 <https://pushover.net>，你的**用户密钥**在主页上正确显示。
2. 然后创建应用：**创建应用/API令牌**。你得到一个**应用令牌**。

- **设置** — `priority`（可选，从-2到2）、`sound`（可选）、`retry` 和 `expire` （仅对 `priority: 2`）
- **密钥** — `token` （应用令牌）、`user_key` （用户密钥）

不有 `priority` 设置，DumbMonit根据严重程度选择：`-1` 用于顾问或解决，`0` 用于警告，`1` 用于严重警报。顾问因此不使电话响。

> 优先级 `2` （"紧急"）重复通知直到它被确认。DumbMonit然后自动添加 `retry` 和 `expire`，不有它Pushover会拒绝消息。

### Pushbullet {#pushbullet}

在 <https://www.pushbullet.com/#settings/account>，点击**创建访问令牌**。

- **设置** — `device_iden`（可选；不有它，你的所有监控项被通知）
- **密钥** — `token`

### Bark (iOS) {#bark-ios}

在iPhone上打开Bark应用：它显示一个URL包含你的**监控项密钥**，长字符串后面的域名。

- **设置** — `server_url`（可选，默认 `https://api.day.app`）、`group` （可选）、`level`（可选）、`sound`（可选）
- **密钥** — `token` （监控项密钥）

不有 `level` 设置，DumbMonit选择：`passive` 用于顾问、`active` 用于警告、`timeSensitive` 用于严重警报。 `critical` 级别，其打破通过"请勿打扰"，永不自动选择——如果你想它明确设置。

### Web推送 {#web-push}

本地通知在你的电话和计算机上，直接从你自己的DumbMonit，不有应用要安装且没第三方账户：浏览器（安装作应用或否则）接收警报即使有所有标签关闭。

1. 在每部电话或计算机上，打开DumbMonit并去**设置→推送通知→在该设备启用**，然后**发送测试**。在iPhone和iPad上，首先添加DumbMonit到主屏幕。逐步指南在[推送通知（PWA）](using/push-notifications.md)。
2. 创建一个**Web推送**通道这里。它没有密钥：其接收者是在步骤1中订阅的设备。

- **设置** — `users`（可选；每行一个账户，空：每个账户的每个订阅设备）、`urgency`（默认 `high`、`normal` 或 `low`）、`ttl`（秒推送服务为关闭的设备保留消息，默认 `86400`）、`contact`（可选 `mailto:` 或 `https:` 地址给推送服务；空：`DUMBMONIT_PUBLIC_URL` 当它是HTTPS，否则项目页面）
- **密钥** — 无。服务器的VAPID密钥对在首次启动时生成并存储加密；私钥永不离开服务器。

通知标题是严重程度和设备（`Critical · nas01`），主体一个短摘要；点击打开**警报**页面。DumbMonit需要出站HTTPS给推送服务（`fcm.googleapis.com`、`*.notify.windows.com`、`updates.push.services.mozilla.com`、`*.push.apple.com`）；消息是端到端加密，那些服务无法读它们。订阅推送服务报告为消失（HTTP 404或410，例如在应用被删除后）自动删除。

### Apprise {#apprise}

Apprise是一个**网关**：用单个配置它给访问DumbMonit不实现自己的数十服务。部署 `caronc/apprise` 容器，然后选择两个模式之一。

**推荐模式——存储在Apprise中的配置。** 在Apprise界面中创建配置，注意其密钥，此处什么不放。

- **设置** — `server_url`、`config_key`

**直接模式——DumbMonit提供目的地。** Apprise URL通常包含密码（`mailto://user:password@…`）：它们因此去密钥。

- **设置** — `server_url`
- **密钥** — `urls` （一个或更多Apprise URL，用逗号分开）

可选通用设置：`tag` （瞄准仅部分配置的目的地）和 `format` （默认 `text`，或 `markdown`）。

> 如果测试回答"Apprise在该config_key下无配置"，密钥匹配无。DumbMonit报告它而非假装消息被发送。

### Home Assistant {#home-assistant}

1. 在Home Assistant：**你的个人资料→安全→长期访问令牌→创建令牌**。复制它，它不会再显示。
2. 选择要调用的服务。默认，DumbMonit在仪表板上创建持久通知。

- **设置** — `server_url` （例如 `http://homeassistant.local:8123`）、`service`（可选，默认 `persistent_notification.create`）、`target` （可选）、`data`（可选，对象合并入调用）
- **密钥** — `token`

几个有用的服务：

| `service` | 效果 |
|---|---|
| `persistent_notification.create` | Home Assistant界面中的横幅 |
| `notify.notify` | 所有配置的通知目标 |
| `notify.mobile_app_<device>` | 一个特定监控项的移动应用 |
| `light.turn_on` | 打开灯——用 `data` 用于实体和颜色 |

`data` 设置被合并**最后**：它因此可覆盖DumbMonit准备的标题或消息。

### 电子邮件（SMTP） {#email-smtp}

唯一不取决于第三方服务的通道。

- **设置** — `host`、`from`、`to`（地址列表）、`security`（默认 `starttls`、或 `tls`、或 `none`）、`port` （从 `security` 派生：587、465或25）、`username`（可选）
- **密钥** — `password`

> 用Gmail，账户密码不工作：在你Google账户的安全设置创建一个**应用密码**。

### Signal {#signal}

通过一个[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api)实例你托管且你首先注册你的号码。

- **设置** — `server_url`（例如 `http://signal.home:8080`）、`number` （注册的发件人号码）、`recipients` （号码或组标识符列表）、`username`（可选）
- **密钥** — `password`（可选，如果你的实例被保护）

所有号码用**国际格式**写，`+33612345678`。

### Twilio通过SMS {#sms-via-twilio}

在Twilio控制台，主页显示你的**账户SID**和你的**认证令牌**。然后买一个能发送SMS的号码。

- **设置** — `account_sid`、`from`（你的Twilio号码）**或** `messaging_service_sid`、`to`（接收者列表）
- **密钥** — `token` (*认证令牌*)

SMS主体在300字符处被截断：超出那个，Twilio分割它进分段单独计费。

> 在试验账户上，Twilio仅发送给你验证的号码。

### PagerDuty {#pagerduty}

在PagerDuty：**服务→你的服务→集成→添加集成**，并选择**事件API v2**。复制*集成密钥*。

- **设置** — `region`（`eu` 如果你的账户托管在欧洲）、`source` （可选，默认 `dumbmonit`）
- **密钥** — `token` (*集成密钥*)

一个警报打开事件；其解决**自动关闭**。匹配在警报的指纹上完成，前缀有 `dumbmonit-`，所以提醒永不打开秒事件。

> 另一类型的密钥（"事件API v1"、"REST API"）得到简洁拒绝。检查集成确实是*事件API v2*类型。

### Opsgenie {#opsgenie}

**团队→你的团队→集成→添加集成→API**。复制API密钥。

- **设置** — `region`（`eu` 用于欧洲账户）、`responders`（团队名称列表，可选）、`tags`（列表，可选）
- **密钥** — `api_key`

如同PagerDuty，解决关闭警报，由其别名识别。

> Atlassian设置了**Opsgenie在2027年4月5日的服务终止**。该通道停留用于现有安装；对新的，优先PagerDuty或自定义通道指向Jira服务管理。

### 自定义webhook {#custom-webhook}

见上面[自定义通道](#the-custom-channel)。

---

## 故障排除 {#troubleshooting}

**测试立即失败，"缺失设置…"。**
一个必需字段是空的。消息命名准确设置。

**测试失败有"15秒内无答案"。**
目标服务器从DumbMonit无法到达。如果它是本地网络上的服务，检查DumbMonit容器是否能到达它——这频繁是Docker网络问题，不是通道配置问题。

**测试失败有证书错误。**
你的服务器使用自签或私有CA证书。见[自签证书](#self-signed-certificates)。

**测试成功，但我接收无。**
服务接受消息并将其发送别处：错误通道、错误ntfy主题、错误Pushbullet监控项。检查目的地设置。

**错误消息包含 `***`。**
这是打算的：你的令牌在记录前被删除消息。没有密钥永显现在日志、在"最后错误"列或在API响应中。

**我在哪里可以看到通道的最后错误？**
在通道列表：每行显示最后成功递送的日期，如果任何，最后失败的错误。
