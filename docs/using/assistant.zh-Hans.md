# 连接一个助手 {#connect-an-assistant}

DumbMonit 有一个内置 [MCP](https://modelcontextprotocol.io) 服务器：模型上下文协议，助手如 Claude、ChatGPT、Cursor 或 VS Code 的代理调用外部工具的方式。连接一个，然后问它像这样的事：

- "一切都好吗？"
- "沉默 NAS 两小时，我在交换一个磁盘。"
- "昨晚发生了什么？"
- "备份服务器有多满，它在变糟吗？"
- "在状态页面发布更新：邮件服务器回来了。"

助手看到接口看到的 — 监控项、告警、历史、度量、代理、状态页面 — 并使用相同的词：*reporting*、*unreachable*、*waiting*、*advisory*、*warning*、*suppressed by parent*。它运行在服务器内部：在 DumbMonit 旁没有要安装的东西，工具调用与 web UI 相同的代码。

## 1. 创建一个令牌 {#1-create-a-token}

Settings → **API & assistants** (`/settings#assistant`)。在助手或运行它的机器之后命名令牌（"Claude on my laptop"），然后选择：

| Setting | Choices |
| --- | --- |
| **Scope** | **Read**：查看 — 状态、监控项、告警、历史、度量、规则、代理、状态页面。它永不能改变任何东西。**Read and write**：也行动 — 沉默、承认、探针、添加监控项、重启容器、发布事件…（[write tools](#what-the-assistant-can-call)）。 |
| **Expires** | 30 天、90 天（默认）、1 年或永不。过期的令牌停止工作；创建新的。 |
| **Allowed networks** | 可选。令牌可能被使用的地址或 CIDR 范围（`192.168.1.0/24, 10.8.0.5`）。空：任何地方。 |

令牌看起来像 `dmt_` 后跟 32 个十六进制字符。它显示**一次**，创建后对；DumbMonit 仅保留哈希。丢失它、撤销它并创建另一个。

除非您实际想要助手行动，否则倾向阅读令牌。

同令牌也打开[REST API](../reference/api.md#authentication)给脚本和仪表板 — `curl -H "Authorization: Bearer dmt_…" https://monit.example.lan/api/targets` — 其中 `read` 看查看者看到的和 `write` 做管理员可以做的，账户、令牌和备份除外。

## 2. 粘贴片段 {#2-paste-the-snippet}

设置页面填这些进您的服务器地址和新令牌。下方，`https://monit.example.lan` 代表您的服务器。

### Claude Code {#claude-code}

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

添加 `--scope user` 在每个项目中有它，不仅仅当前的。

### Claude Desktop {#claude-desktop}

Claude Desktop 从其配置文件启动本地服务器；[`mcp-remote`](https://www.npmjs.com/package/mcp-remote) 桥它给 DumbMonit 的 HTTP 端点（它需要 [Node.js](https://nodejs.org)）。DumbMonit 用 bearer 标头而不是 OAuth 认证，所以通过这个文件添加它而不是 Connectors 屏幕。Settings → Developer → **Edit Config**，然后添加给 `claude_desktop_config.json` 并重启 Claude Desktop：

```json
{
  "mcpServers": {
    "dumbmonit": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote", "https://monit.example.lan/api/mcp",
        "--header", "Authorization:${DUMBMONIT_AUTH}"
      ],
      "env": { "DUMBMONIT_AUTH": "Bearer dmt_…" }
    }
  }
}
```

标头通过环境变量，冒号后无空格，因为某些平台在空格上分割参数。对于通过纯 HTTP 在您网络上到达的服务器，添加 `"--allow-http"` 给 `args` — 设置页面在其本身通过 HTTP 提供时为您做它。

### VS Code {#vs-code}

`.vscode/mcp.json` 在工作区中（或命令 **MCP: Add Server** → HTTP）：

```json
{
  "servers": {
    "dumbmonit": {
      "type": "http",
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

### Cursor 和其他客户端 {#cursor-and-other-clients}

`.cursor/mcp.json`，以及相同形状对于大多数支持 Streamable HTTP 服务器带自定义标头的客户端：

```json
{
  "mcpServers": {
    "dumbmonit": {
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

仅启动本地（stdio）服务器的客户端可以使用为 Claude Desktop 显示的 `mcp-remote` 桥。

### ChatGPT {#chatgpt}

Settings → Connectors → **Create**（开发者模式）。MCP 服务器 URL：`https://monit.example.lan/api/mcp`，授权标头：`Bearer dmt_…`。

ChatGPT 从 OpenAI 的服务器连接，不从您的浏览器：DumbMonit 必须**从互联网通过 HTTPS 可达**。在做那个前见[security notes](#security-notes)。

### 任何其他客户端 {#any-other-client}

客户端需要知道什么：端点是 `https://<server>/api/mcp`，传输是 Streamable HTTP，每个请求携带 `Authorization: Bearer dmt_…`。原始调用，检查令牌和从终端网络路径：

```bash
curl -s https://monit.example.lan/api/mcp \
  -H 'Authorization: Bearer dmt_…' \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

## 助手可以调用的 {#what-the-assistant-can-call}

二十七个工具。每一个有标题、描述、输入模式、输出模式和注解（`readOnlyHint`、`destructiveHint`、`idempotentHint`），所以客户端可以告诉哪个仅看并问您在运行其他前。

| Tool | Scope | Does |
| --- | --- | --- |
| `get_status` | read | 公报，"一切都好吗？"的答案：监控项数、触发和构建告警，一句话（"2 advisories, 1 unreachable."）。 |
| `list_devices` | read | 监控项和服务与状态、父和标签；用子串或状态过滤。 |
| `get_device` | read | 一个监控项：配置、状态、活跃告警、24 小时 CPU / 内存 / 最满磁盘摘要（可用性和延迟对服务检查）。 |
| `list_alerts` | read | 活跃告警有严重词、自从、值、抑制、承认。 |
| `alert_history` | read | 告警转换，最最近优先（`since` 或 `hours`，可选监控项，限制）。 |
| `query_metrics` | read | MetricsQL 范围查询，最多 60 点每系列。 |
| `list_silences` | read | 维护窗口和是否现在活跃。 |
| `list_rules` | read | 告警规则带类别、严重等级、阈值、启用。 |
| `list_device_types` | read | 这个服务器可以监测的监控项类型，及其期望的选项和凭证 — `add_device` 需要什么。 |
| `list_agents` | read | 运行代理的机器：OS、代理版本、中继模式、网站、中继监控项、最后联系、绑定。 |
| `list_containers` | read | 运行代理的机器的 Docker 容器：状态、健康、重启、更新可用、最后命令。 |
| `list_heartbeats` | read | 心跳监测：期望间隔、最后调用、判定（等待、按时、错过、报告宕机）。秘密 URL 从不显示。 |
| `list_status_pages` | read | 状态页面，发布或不，其公共路径（`/s/<slug>`）、它们显示的监控项和其开启事件。 |
| `list_channels` | read | 通知通道：类别、启用、最后错误、最后消息发送。永不其秘密。 |
| `list_packs` | read | 安装集成 packs：版本、监控项类型、启用、使用它的监控项、错误。 |
| `silence_device` | write | 监控项上一个一次性维护窗口，现在启动（默认 1 小时，最多一周）。 |
| `remove_silence` | write | 移除维护窗口。 |
| `schedule_maintenance` | write | 在给定日期的一次性维护窗口，一个监控项或整个实例，最多一周。 |
| `acknowledge_alert` | write | 通过指纹承认一个告警多小时（默认 4）：提醒暂停，解决仍被通知。`hours: 0` 提起它。 |
| `probe_device` | write | 立即探针一个监控项并报告测量内容。 |
| `set_device_enabled` | write | 启用或禁用监控项监测。 |
| `set_rule_enabled` | write | 启用或禁用告警规则。 |
| `add_device` | write | 添加监控项：名称、类型、地址、间隔、父、选项、凭证、中继代理。 |
| `discover_network` | write | 扫描网络（CIDR，最多 4096 地址）对 SNMP 设备并建议每个的 profile。 |
| `restart_container` | write | 要求机器的代理重启其一个容器。 |
| `test_channel` | write | 通过通知通道发送测试消息。 |
| `post_incident` | write | 在状态页面上打开事件，或发布现有一个的更新。 |

读令牌调用写工具获得清除拒绝作为工具结果（`isError: true`）；助手解释它什么改变。同样对于工具无法做的任何东西 — 未知监控项、模糊名称、长于一周的窗口：消息说什么修复。

## 协议细节 {#protocol-details}

`POST /api/mcp` 实现 MCP over **Streamable HTTP** 传输：JSON-RPC 2.0，一个请求每 `POST`，用纯 JSON 答复（无服务器发送事件）。它无状态：无 `Mcp-Session-Id` 被发出，一个客户端发送的被忽略，`GET` 或 `DELETE` 在端点答 `405`。仅 `tools` 能力被提供 — 无资源、提示、采样或 OAuth。

两代协议在相同端点提供：

| Versions | How a client speaks it |
| --- | --- |
| `2026-07-28` | 无状态。每个请求携带 `io.modelcontextprotocol/protocolVersion` 和 `io.modelcontextprotocol/clientCapabilities` 在 `params._meta` 中，以及标头 `MCP-Protocol-Version`、`Mcp-Method` 和，对于 `tools/call`，`Mcp-Name`，必须匹配正文。方法：`server/discover`、`tools/list`、`tools/call`。结果携带 `resultType: "complete"` 和服务器身份在 `_meta` 中（`io.modelcontextprotocol/serverInfo`）；`tools/list` 也携带 `ttlMs` 和 `cacheScope`。 |
| `2025-11-25`、`2025-06-18`、`2025-03-26` | `initialize` 握手选版本（未知获最新这三个）；`notifications/initialized` 用 `202` 承认；然后 `ping`、`tools/list`、`tools/call`。 |

错误，除了[令牌检查](#security-notes)的 `401`、`403` 和 `429`，在任何 JSON-RPC 前来：

| HTTP | JSON-RPC code | When |
| --- | --- | --- |
| `400` | `-32020`（标头不匹配） | 无 `MCP-Protocol-Version`、`Mcp-Method` 或 `Mcp-Name` 的 `2026-07-28` 请求，或值从正文不同。 |
| `400` | `-32022`（不支持的协议版本） | 这个服务器不说的版本；`error.data` 列表 `supported` 和 `requested` 的。 |
| `400` | `-32602`（无效参数） | 缺失必需 `_meta` 字段的 `2026-07-28` 请求。 |
| `404` | `-32601`（未找到方法） | 这个服务器不实现方法的 `2026-07-28` 请求。 |
| `200` | `-32602` | 未知工具，或 `arguments` 不是对象。 |
| | `-32700`、`-32600` | 不可读 JSON；不是 JSON-RPC 2.0 请求（批次不支持）。 |
| `202` | — | 一个通知：接受，无正文。 |

运行但失败的工具 — 缺少范围、未知监控项 — 不是协议错误：它通常用 `isError: true` 和助手可以中继的句子答。

## 安全注意 {#security-notes}

- **令牌是密码。** 任何持有它的人读接口显示的一切 — 并且，用写令牌，改变任何管理员可以通过 REST API 做的（监控项、规则、通道），账户、令牌和备份除外。把它远离共享聊天、屏幕截图和仓库。撤销您不确定的令牌；在一分钟内创建另一个。
- **倾向读。** 大多数问题（"一切都好吗？"）需要无写范围。仅为您实际想要行动的助手创建写令牌，命名它所以您在列表中识别它。大多数客户端问在运行标记为非只读的工具前；留那个确认开启。
- **给它一个生命和网络。** 在 90 天过期并仅从您的 LAN 或 VPN 工作的令牌（`192.168.1.0/24`）对发现它的任何人值很少。在反向代理后，声明代理在 `DUMBMONIT_TRUSTED_PROXIES`，否则每个调用似乎来自代理，网络限制拒绝它们全。
- **绑定给其创建者。** 令牌当创建它的账户被禁用时停止工作，当该账户不再是管理员时失去其写力量，被撤销当该账户被删除时。
- **无秘密回来。** 工具从不返回监控项凭证（仅其类别）、通道秘密、心跳 URL 或令牌。`add_device` 确实接受凭证：它们通过您的助手对话，因此通过其供应商。对于有敏感凭证的监控项，倾向 web 接口。
- **浏览器和 `Origin`。** 携带 `Origin` 标头（基于浏览器的客户端）的请求用 `403` 被拒绝，除非那个来源是服务器自己或被 `DUMBMONIT_API_CORS_ORIGINS` 允许（默认任何来源，自令牌无论如何必需）。在那里列出您的来源缩小它；见[cross-origin requests](../reference/api.md#cross-origin-requests-cors)。
- **速率限制。** 每个令牌限制到 120 次调用每分钟，REST 和 MCP 一起：困在循环中的助手获 `429` 和 `Retry-After`，不您服务器的全注意。
- **HTTPS 互联网前。** ChatGPT 的连接器需要公共 URL。把 DumbMonit 放在带 TLS 的反向代理后（Caddy、Traefik、nginx…）并设置 `DUMBMONIT_COOKIE_SECURE=1`。永不暴露纯 HTTP 端口。Claude Desktop、Claude Code、VS Code 和 Cursor 运行在您机器并可以直接到达 LAN 地址，所以它们需无暴露在全。
- **看"last used"。** 令牌列表显示每个令牌何时最后被使用和从哪地址。一个在您不与您的助手谈话时使用的令牌，或从一个地址您不知道，值得撤销。
- **日志。** 每个工具调用在 `info` 等级用令牌名和工具记录，从不参数；每个通过令牌的 REST API 写用令牌名、方法、路径和状态，从不正文。创建和撤销令牌在审计日志中（Settings → Security）。

## 故障排除 {#troubleshooting}

| Symptom | Cause and fix |
| --- | --- |
| `401` / "A valid API token is required." | 标头缺失或误输（`Authorization: Bearer dmt_…`，一个空格在 `Bearer` 后），或令牌被撤销。创建新的。 |
| `401` / "The API token "…" expired on …" | 创建新令牌；选更长生命如太短。 |
| `403` / "cannot be used from …" | 令牌被限制给其他网络。消息中的地址是服务器看到的：在反向代理后，声明代理在 `DUMBMONIT_TRUSTED_PROXIES`。 |
| `403` 在每个来自基于浏览器客户端的调用上 | 其 `Origin` 不允许：添加它给 `DUMBMONIT_API_CORS_ORIGINS`。 |
| 工具答 "needs a token with the "write" scope" | 令牌是只读，或其创建者不再是管理员。如想助手行动创建写令牌。 |
| `429` | 120 次调用分钟：助手可能循环。等 `Retry-After`，然后问狭窄问题。 |
| "Unsupported protocol version" | 客户端说 DumbMonit 不的版本；更新客户端。 |
| Claude Desktop 显示服务器失败 | Node.js 必须为 `npx` 安装；通过纯 HTTP，`--allow-http` 必须在 `args`。 |
| ChatGPT 无法连接 | 它需公共 HTTPS URL；LAN 地址或纯 HTTP 从 OpenAI 服务器不工作。 |
