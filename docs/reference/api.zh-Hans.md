# HTTP API {#http-api}

Web UI 所做的一切都经过 `/api/*`，同一套 API 也向你自己的脚本、仪表板和助手开放：不存在私有的另一半。它是基于 HTTP 的纯 JSON，通过会话 cookie（Web UI）或 API 令牌（其他一切）认证。

!!! warning "Alpha"
    DumbMonit 处于 alpha 阶段，其 API 也是如此：路由和字段仍可能变化。每当删除或重命名某个路由或字段时，API 版本（见下文）就会变化，因此客户端可以据此判断。

## OpenAPI 描述 {#openapi-description}

每个实例都以 [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0) 文档描述自己的 API：

```
GET /api/openapi.json
```

它是公开的——不含机密、地址，也不含任何关于你设备的信息——并且是详尽的参考：客户端可调用的每个路由、其参数、请求和响应格式、错误响应，以及所需的范围（每个操作上的 `x-token-scope` 扩展：`public`、`read`、`write`，或对令牌永远无法调用的路由为 `none`）。本页下文的表格是同一内容的易读摘要。

有意省略的内容：代理协议（`/api/ingest`、`/api/agent/commands*`、`/api/agent/relay*`），只有[代理](../devices/agent.md)使用；SSO 登录的浏览器重定向（`/api/auth/oidc/start`、`/api/auth/oidc/callback`）和 Spotify 的重定向（`/api/music/spotify/callback`）；以及代理下载（`/install.sh`、`/install.ps1`、`/download/*`）。

可在任何 OpenAPI 工具中加载：

```bash
# Browse it, and try calls with your token, in Swagger UI
docker run --rm -p 8081:8080 \
  -e SWAGGER_JSON_URL=http://monit.lan:8080/api/openapi.json swaggerapi/swagger-ui
# then open http://localhost:8081 and paste "dmt_…" under Authorize → bearerAuth

# Generate a client library (here Python)
docker run --rm -v "$PWD:/local" openapitools/openapi-generator-cli generate \
  -i http://monit.lan:8080/api/openapi.json -g python -o /local/dumbmonit-client

# Or just list what is there
curl -s http://monit.lan:8080/api/openapi.json | jq -r '.paths | keys[]'
```

Postman 和 Insomnia 可从同一 URL 导入。

## 认证 {#authentication}

有两种方式，在每个受保护的路由上都被接受：

- **会话 cookie**——Web UI 使用的方式。使用密码（或 SSO）登录，并发送 HttpOnly 的 `dumbmonit_session` cookie（SameSite Lax；当 `DUMBMONIT_COOKIE_SECURE=1` 时为 `Secure`；30 天）。每个改变状态的请求（`POST`、`PUT`、`DELETE`）还必须携带 `X-Requested-With: DumbMonit`——这是 UI 自动添加的防 CSRF 证明；没有它，请求会以 `403` 被拒绝。
- **API 令牌**——`Authorization: Bearer dmt_…`，供脚本、仪表板和助手使用。在设置 → **API 与助手**中创建（与 [MCP 服务器](../using/assistant.md)使用的是同一种令牌），只显示一次。不涉及 cookie，因此永远不需要 `X-Requested-With` 头。

```bash
# API token: one header, nothing else
curl -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/targets
curl -H 'Authorization: Bearer dmt_…' -X POST http://monit.lan:8080/api/targets/4/probe

# Session: sign in (204 and a Set-Cookie header), then send the cookie
curl -c cookies.txt -X POST http://monit.lan:8080/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"…"}'
curl -b cookies.txt http://monit.lan:8080/api/targets
curl -b cookies.txt -X POST http://monit.lan:8080/api/targets \
  -H 'X-Requested-With: DumbMonit' -H 'content-type: application/json' \
  -d '{"name":"NAS","address":"nas.lan","kind":"snmp","credential":{"type":"snmp_community","community":"public"}}'
```

当请求携带 bearer 令牌时，cookie 会被忽略：已吊销的令牌即使来自已登录的浏览器也会被拒绝，而不是悄悄回退到会话。

代理注册令牌（`dmon_…`）是另一回事：它们只在代理路由（`/api/ingest`、`/api/agent/commands/*`、`/api/agent/relay*`）上有效。它们表明某台机器可以通信，而不是*哪一台*机器：在这些路由上，已绑定的机器还必须在 `X-DumbMonit-Agent-Secret`（`dmab_…`）中出示自己的绑定密钥，该密钥由服务器在注册时一次性下发。参见[绑定](../devices/agent.md#binding-one-machine-one-agent)。

登录受到速率限制：失败五次后，每次后续尝试都会被以 `429` 拒绝，并带有 `Retry-After` 延迟，该延迟从 30 秒起翻倍，最长 5 分钟。切勿在循环中猜测密码。

## 范围 {#scopes}

令牌有两种范围之一。没有第三种更强大的范围：管理员在自己账户上所做的事情，任何令牌都无法触及。

| | `read` 令牌 | `write` 令牌 |
|---|---|---|
| *查看者*账户可以发起的每个 `GET`（设备、告警、指标、规则、不含机密的渠道、状态页面……） | 是 | 是 |
| *管理员*可以发起的 `POST`、`PUT`、`DELETE`（设备、规则、静默、渠道、状态页面、包、探测、发现……） | `403` | 是 |
| `/metrics`、`/federate`、`/prometheus/*`（[抓取](metrics.md#scraping-dumbmonit)） | 是 | 是 |
| MCP 读取工具 / 写入工具（[助手](../using/assistant.md#what-the-assistant-can-call)） | 仅读取 | 两者 |
| 账户、会话、双因素、SSO、审计日志（`/api/auth/*`、`/api/users/*`） | `403` | `403` |
| API 令牌和代理注册令牌（`/api/tokens/*`、`/api/agent/tokens/*`） | `403` | `403` |
| 备份与恢复（`/api/backup/*`） | `403` | `403` |

`read` 令牌访问写入路由时会得到 `403`，消息中指明该令牌和缺少的范围。由于没有任何令牌可以列出、创建或吊销令牌，令牌永远无法铸造另一个令牌——更不用说更强大的令牌了。

## 令牌安全 {#token-security}

- **只显示一次，哈希存储。** 令牌是 `dmt_` 后跟 128 位随机数。服务器只保留其 SHA-256 摘要（以及开头的字符，`dmt_1a2b3c4d`，用于在列表中识别它）；明文令牌只存在于创建它的响应中。出示的令牌以恒定时间比较。
- **有效期。** 创建时可选择 30 天、90 天、1 年或永不过期。过期的令牌会得到 `401`，消息为 `The API token "<name>" expired on …`，并带有 `WWW-Authenticate: Bearer error="invalid_token"`；它会留在列表中并标记为已过期，直到你吊销它。
- **允许的网络。** 可选择将令牌限制在一组地址或 CIDR 范围内（`192.168.1.0/24, 10.8.0.5`，最多 32 项）。从其他任何地方使用都会得到 `403`，消息为 `The API token "<name>" cannot be used from <address>.` 所检查的地址是 TCP 客户端的地址，或者——仅当连接来自列于 `DUMBMONIT_TRUSTED_PROXIES` 的代理时——该代理在 `X-Forwarded-For` 中报告的地址。在你未声明的反向代理之后，每个请求看起来都来自代理：请声明它，否则该限制会拒绝所有人。
- **最近使用。** 令牌列表显示每个令牌最近一次使用的时间和来源地址（最多每分钟更新一次，地址变化时立即更新）：没人再使用的令牌就该吊销。
- **绑定到其创建者。** 令牌会记住创建它的账户。如果该账户被停用，其令牌会被拒绝（`401`）；如果它被降级为操作员或查看者，其 `write` 令牌会按 `read` 令牌处理；如果它被删除，其令牌会被吊销。
- **吊销**立即生效：下一次调用得到 `401`。
- **有日志。** 用令牌发起的每次写入都会记录到服务器日志，包含令牌的名称和 id、方法、路径（不含查询字符串）和状态——绝不记录令牌，绝不记录正文。创建和吊销令牌记录在审计日志中（设置 → 安全）。

## 速率限制 {#rate-limits}

每个令牌每分钟可发起 120 次调用，REST 和 MCP 合计。超过之后，调用会被以 `429` 和 `Retry-After` 头（秒）拒绝，直到这一分钟结束。对使用令牌认证的请求的每个响应都会告诉你当前状况：

| 头 | 含义 |
|---|---|
| `RateLimit-Limit` | 每个窗口允许的调用数（120）。 |
| `RateLimit-Remaining` | 当前窗口中剩余的调用数。 |
| `RateLimit-Reset` | 窗口重新开始前的秒数。 |

面板很多且刷新很快的 Grafana 仪表板可能会达到限制：请给它单独的令牌，或延长刷新间隔。

## 跨源请求（CORS） {#cross-origin-requests-cors}

来自其他源的页面——你自己的仪表板、Swagger UI、基于浏览器的 MCP 客户端——可以**使用 bearer 令牌（绝不使用会话）**调用 API：

- 仅当请求携带 `Authorization: Bearer dmt_…` 时（或针对公开的 `GET /api/openapi.json`），响应才带有 `Access-Control-Allow-Origin`。绝不发送 `Access-Control-Allow-Credentials`，因此浏览器永远不会在跨源调用中附带会话 cookie；
- 对 `/api/*` 的预检（`OPTIONS`）仅当所声明的头包含 `authorization` 时才成功：`204`，方法 `GET, POST, PUT, DELETE`，头 `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`，缓存 10 分钟。其中不含 `X-Requested-With`，因此基于 cookie 的会话永远无法从另一个源发起写入；
- `Retry-After`、`WWW-Authenticate`、`RateLimit-*` 头以及 `X-DumbMonit-Api-Version` 可被调用方脚本读取。

`DUMBMONIT_API_CORS_ORIGINS` 设置允许哪些源：

| 值 | 效果 |
|---|---|
| 未设置（默认） | 任意源（`Access-Control-Allow-Origin: *`）。是安全的，因为只有 bearer 令牌才能打开任何东西。 |
| `https://grafana.example.com,https://dash.lan` | 仅这些源，原样回显。 |
| `off` | 完全没有 CORS 头：仅同源页面和非浏览器客户端。 |

同一份列表还决定 [MCP 端点](../using/assistant.md#security-notes)接受哪些 `Origin`。

```js
// From a page on another origin. Never ship a write token in a public page.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'calls left this minute');
```

## 版本控制 {#versioning}

`/api` 下的每个响应都带有 API 版本：

```
X-DumbMonit-Api-Version: 0.1
```

同一个值是 OpenAPI 文档中的 `info.version`，旁边是 `x-server-version`，即响应请求的服务器版本。当 API 处于 `0.x` 时，它并未冻结；每当删除或重命名某个路由或字段时版本就会变化。添加路由或字段不会改变它：请忽略你不认识的字段。

## 错误 {#errors}

每个错误都是 JSON：`{"error": "message"}`，用人能读懂的语言写成。

| 状态 | 何时出现 |
|---|---|
| `400` | 请求格式错误或某个值无效；消息会指明。 |
| `401` | 没有有效的会话或令牌：令牌缺失、未知、已吊销或已过期（带 `WWW-Authenticate: Bearer`），或令牌的创建者已被停用。 |
| `403` | 已认证但不被允许：查看者、告警处理之外的操作员、在写入路由上的 `read` 令牌、访问账户/令牌/备份路由的令牌、从不被允许的网络使用的令牌、没有 `X-Requested-With` 的 cookie 写入，或在公开演示上的任何写入（带 `"demo": true`）。 |
| `404` | id 不存在。`/api` 下不匹配任何路由的路径返回 `404 {"error": "Unknown API route: …"}`，而不是 Web UI 的 HTML。 |
| `405` | 该路径上不存在此方法（例如 `GET /api/mcp`）。 |
| `409` | 与当前状态冲突（重复、命令已在运行……）。 |
| `429` | 被限速，带 `Retry-After`。 |
| `500` | 服务器故障；详情在服务器日志中，绝不在响应里。 |

```bash
$ curl -i -H 'Authorization: Bearer dmt_revoked…' http://monit.lan:8080/api/targets
HTTP/1.1 401 Unauthorized
www-authenticate: Bearer
{"error":"A valid API token is required."}

$ curl -i -X DELETE -H 'Authorization: Bearer dmt_…read…' http://monit.lan:8080/api/targets/4
HTTP/1.1 403 Forbidden
{"error":"This action needs a token with the \"write\" scope; the token \"Grafana\" is \"read\" only. Create a write token in Settings → API & assistants."}

$ curl -i -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/alerts   # the 121st call this minute
HTTP/1.1 429 Too Many Requests
retry-after: 17
ratelimit-limit: 120
ratelimit-remaining: 0
{"error":"Rate limit reached (120 calls per minute per token). Retry in 17 s."}
```

[MCP 端点](../using/assistant.md#protocol-details)则改用 JSON-RPC 响应，并使用自己的错误代码。

## 账户、登录与令牌 {#accounts-sign-in-and-tokens}

在下面的表格中，*会话*指会话 cookie **或** API 令牌；*管理员*指管理员会话或 `write` 令牌；*操作员*指操作员或管理员会话，或 `write` 令牌；*仅会话*指会话 cookie，绝不是令牌。

**角色。** `viewer` 账户可发起所有 `GET`。`operator` 可进行同样的读取，外加本页中标为*操作员*的告警处理写入（OpenAPI 文档中为 `x-roles: [admin, operator]`）：确认、暂缓（维护窗口）、对某设备忽略某条规则、清除已解决的历史。其他所有写入仅限管理员，对操作员返回 `403`。

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` | `/api/auth/status` | 公开 | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`。`configured: false` 表示全新实例：在第一个管理员存在之前，每个标为*会话*的路由都返回 `401`——只有 `status`、`setup`、`login` 和 `health` 可访问。 |
| `POST` | `/api/auth/setup` | 公开 | `{"setup_code": "…", "username": "admin", "password": "…"}`。在全新实例上创建第一个管理员（至少 12 个字符）；`username` 可省略，此时为 `admin`。`setup_code` 是启动时打印在服务器日志中的一次性代码（或 `DUMBMONIT_SETUP_CODE`）；大小写、连字符和空格会被忽略。`204`；不会开启会话。代码错误返回 `401`，过多次数后返回 `429`，账户已存在时返回 `409`。 |
| `POST` | `/api/auth/login` | 公开 | `{"username": "…", "password": "…"}`。`204` 并带 `Set-Cookie`。密码错误返回 `401`，被限速时返回 `429`。当账户启用了双因素时，改为返回 `200` 及 `{"totp_required": true, "pending": "…"}`——一张有效期五分钟、输错五次验证码后失效的凭证：用 `/api/auth/login/totp` 完成登录。 |
| `POST` | `/api/auth/login/totp` | 公开 | `{"pending": "…", "code": "123456"}`——第一步得到的凭证和一个验证码（或恢复码）。`204` 并带会话 cookie。 |
| `GET` | `/api/auth/oidc/start` | 公开 | 将浏览器重定向到身份提供方。 |
| `GET` | `/api/auth/oidc/callback` | 公开 | 从提供方返回（`code`、`state`）：开启会话并重定向到 UI。 |
| `GET` | `/api/auth/me` | 仅会话 | 当前账户：`id`、`username`、`display_name`、`role`（`admin`、`operator`、`viewer`）、`auth`（`password`、`oidc`）、`disabled`、`totp_enabled`、`created_at`、`last_login_at`。 |
| `POST` | `/api/auth/logout` | 仅会话 | 结束会话并清除 cookie。 |
| `POST` | `/api/auth/password` | 仅会话 | `{"current_password": "…", "new_password": "…"}`。使所有其他会话登出。 |
| `GET` | `/api/auth/totp` | 仅会话 | 当前账户的双因素状态：`enabled`、`pending`、`recovery_codes_left`。 |
| `POST` | `/api/auth/totp/enroll` | 仅会话 | `{"password": "…"}`。提出一个密钥：`secret`（base32）、`otpauth_uri`、`issuer`、`account`。在验证之前不会强制执行任何内容。 |
| `POST` | `/api/auth/totp/verify` | 仅会话 | `{"code": "123456"}`。确认注册并返回 `{"recovery_codes": […]}`——只显示一次。 |
| `DELETE` | `/api/auth/totp` | 仅会话 | `{"password": "…"}`。移除第二因素。`204`。 |
| `GET` | `/api/auth/audit?limit=200` | 管理员，仅会话 | 最近的安全事件：`id`、`at`、`actor`、`action`（`login`、`login.failed`、`password.changed`、`token.created`、`user.updated`、……）、`subject`、`ip`。 |
| `GET` | `/api/auth/oidc/config` | 管理员，仅会话 | SSO 设置及其来源（环境变量或数据库）；绝不返回 client secret。 |
| `PUT` | `/api/auth/oidc/config` | 管理员，仅会话 | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`。缺失或为空的 `client_secret` 保留已存储的值；缺失的 `auto_create` 表示关闭。 |
| `DELETE` | `/api/auth/oidc/config` | 管理员，仅会话 | 忽略已存储的 SSO 设置（如有环境变量，则重新生效）。`204`。 |
| `POST` | `/api/auth/oidc/test` | 管理员，仅会话 | 获取提供方的发现文档并报告所发现的内容。 |
| `GET` | `/api/users` | 管理员，仅会话 | 每个账户，格式同 `/api/auth/me`。 |
| `POST` | `/api/users` | 管理员，仅会话 | `{"username", "display_name", "role", "password"}`。仅当启用了 SSO 时 `password` 才可省略。`201`。 |
| `PUT` | `/api/users/{id}` | 管理员，仅会话 | `display_name`、`role`、`disabled`、`password` 中的任意项；省略的字段保持原值。不能降级或停用最后一个管理员。 |
| `DELETE` | `/api/users/{id}` | 管理员，仅会话 | `204`。不能是你自己，也不能是最后一个管理员。 |
| `DELETE` | `/api/users/{id}/totp` | 管理员，仅会话 | 重置另一个账户的第二因素（被锁定的同事）。`204`。 |
| `GET` | `/api/tokens` | 仅会话 | 每个 API 令牌：`id`、`name`、`prefix`、`scope`、`created_at`、`created_by`（创建它的账户）、`expires_at`（`null`：永不过期）、`expired`、`allowed_networks`（空：任何地方）、`last_used_at`、`last_used_ip`、`revoked_at`。绝不返回密钥。 |
| `POST` | `/api/tokens` | 管理员，仅会话 | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`。`name` 必填（最多 80 个字符）；`scope` 为 `read`（默认）或 `write`；`expires_in_days` 范围 1 到 3650，`null` 或缺失表示永不过期的令牌；`allowed_networks` 最多接受 32 个地址或 CIDR 范围，以网络形式存储（`192.168.1.7/24` 变为 `192.168.1.0/24`），为空或缺失表示任何地方。`201`，返回令牌字段外加 `secret`（只显示一次）。`400` 会指出无效的网络。 |
| `DELETE` | `/api/tokens/{id}` | 管理员，仅会话 | 吊销。`204`；未知或已吊销时返回 `404`。 |

列表中的令牌：

```json
{
  "id": 3, "name": "Grafana", "prefix": "dmt_1a2b3c4d", "scope": "read",
  "created_at": "2026-10-01T09:12:00Z", "created_by": "admin",
  "expires_at": "2026-12-30T09:12:00Z", "expired": false,
  "allowed_networks": ["192.168.1.0/24"],
  "last_used_at": "2026-10-01T09:30:00Z", "last_used_ip": "192.168.1.20",
  "revoked_at": null
}
```

## 响应头 {#response-headers}

除了 `X-DumbMonit-Api-Version`（[版本控制](#versioning)）、`RateLimit-*` 头（[速率限制](#rate-limits)）和 CORS 头（[跨源请求](#cross-origin-requests-cors)）之外，每个响应还带有 `X-Content-Type-Options: nosniff`、`Referrer-Policy: same-origin` 和 `Content-Security-Policy`。该策略从 `default-src 'none'` 开始，只开放界面真正用到的内容——全部由 DumbMonit 自己提供：

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

其中没有第三方源，也永远不应该有：字体、图表库和所有资源都打包在二进制文件中。页面的内联脚本（首次绘制前应用的主题、SvelteKit 的引导程序）由为每个响应重新生成的 nonce 允许，因此 `'unsafe-inline'` 绝不适用于脚本。它确实适用于样式，并且无法移除：Svelte 会在元素上设置 `style=` 属性，而 nonce 和哈希都无法覆盖它们。

公开状态页面是为嵌入而设计的：`/s/<slug>` 及其紧凑视图 `/s/<slug>/embed`——仅这两个——获得相同的策略，但不含 `frame-ancestors`。其他所有内容，包括 `/s/` 下的订阅确认和退订页面，也都带有 `X-Frame-Options: DENY`。

如果你把 DumbMonit 放在反向代理之后，不要让它再添加第二个 `Content-Security-Policy` 头：浏览器会强制执行所有策略的交集，而没有 nonce 的代理默认策略会导致空白页面。

## 健康 {#health}

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` | `/api/health` | 公开 | 服务器版本及其依赖项的状态。始终为 `200`：失败的组件在响应正文中。 |
| `GET` | `/api/update` | 会话或令牌 | 运行中的版本、最新发布版本、`update_available`；服务器每天最多询问 GitHub 一次。`POST /api/update/check` 立即检查，`PUT /api/update/settings` 开启或关闭检查。参见[更新](../install/update.md)。 |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

组件失败时 `status` 为 `degraded`；该组件随后会带有一个 `error` 字符串。当服务器运行自己的 VictoriaMetrics 时 `victoria.embedded` 为 `true`，`DUMBMONIT_VM_URL` 指向外部实例时为 `false`。

## 备份与恢复 {#backup-and-restore}

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` | `/api/backup` | 管理员，仅会话 | 备份包将包含的内容（`contents`，每个分区一行及其数量）、备份包格式版本、实例密钥的位置（`secret_source`：`file` 或 `environment`）以及定时本地备份的状态（`schedule`）。 |
| `POST` | `/api/backup` | 管理员，仅会话 | `{"passphrase": "…", "include_account_secrets": false}`。将备份包本身作为 JSON 文件返回：明文头部（`format`、`version`、`created_at`、`summary`、`kdf`）和 `payload`，后者使用 AES-256-GCM 加密，密钥由口令通过 Argon2id 派生。至少 16 个字符，否则返回 `400`。 |
| `POST` | `/api/backup/restore` | 管理员，仅会话 | `{"bundle": {…}, "passphrase": "…", "apply": false}`。除非 `apply` 为 `true`，否则为试运行；二者运行相同的代码，试运行在被回滚的事务内进行。返回报告：每个分区的 `created`、`updated`、`skipped` 和 `notes`，外加 `warnings`。口令错误、文件被修改或备份包以更新的格式版本写成时返回 `400`。 |
| `POST` | `/api/backup/local` | 管理员，仅会话 | 立即写入一份定时风格的本地备份，并返回新的 `schedule`。 |

备份包包含实例的所有凭据，并用口令重新加密，因此这些路由会拒绝 API 令牌并给出说明：只有登录 Web 界面的管理员才能使用。参见[备份与恢复](../install/backup.md)。

## 设备类型 {#device-types}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/collectors` | 此服务器可监控的每种设备类型：`kind`、`label`、`summary`、`examples`、`credential_types`、`address_hint`、`default_port`、`setup`（`title`、`steps`、`warning`、`doc_url`）和 `options`（`key`、`label`、`help`、`placeholder`、`default`、`required`、`input`、`choices`）。由[集成包](../packs/index.md)带来的类型以同样方式列出，形式为 `pack.<id>`。 |

## 集成包 {#integration-packs}

管理员会话或 API 令牌（更改需要 `write` 范围）。参见[集成包](../packs/index.md)。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/packs` | 已安装的包：`id`、`version`、`label`、`summary`、`kind`（仅含 SNMP 配置文件的包为 `null`）、`enabled`、`installed_at`、`sha256`、`metrics`、`rules`（uid）、`snmp_profiles`、`targets`（使用它的设备）、`warnings`，以及已存储的包不再通过验证时的 `error`。 |
| `GET` | `/api/packs/{id}` | 一个包。 |
| `POST` | `/api/packs` | 安装或更新。正文：YAML（`application/yaml`、`text/plain`……）或 JSON 中的 `{"yaml": "…"}`。创建时返回 `201`，更新或未变化时返回 `200`，附带 `outcome`（`created`、`updated`、`unchanged`）、`pack`、`rules_added` 和 `restart_required`（SNMP 配置文件）。`400` 会列出每个验证错误。 |
| `PUT` | `/api/packs/{id}/enable`、`/api/packs/{id}/disable` | 注册或撤销该包的设备类型；设备、历史和规则保留。 |
| `DELETE` | `/api/packs/{id}` | 卸载，连同其规则。有设备使用时返回 `409`。 |

## 目标（设备） {#targets-devices}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/targets` | 列出每个设备。 |
| `POST` | `/api/targets` | 创建一个。`201` 并返回该设备。对 SNMP 会在后台开始配置文件检测。 |
| `GET` | `/api/targets/{id}` | 一个设备。 |
| `PUT` | `/api/targets/{id}` | 替换它。省略 `credential` 会保留已存储的凭据；省略 `profile_id` 会保留检测到的配置文件（`""` 清除它）。设置 `enabled: false` 会清除该设备的告警且不发送通知。 |
| `DELETE` | `/api/targets/{id}` | `204`。清除该设备的告警且不发送通知，并从 VictoriaMetrics 中删除其时间序列（尽力而为）。 |
| `POST` | `/api/targets/{id}/probe` | 立即探测：`{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}`——序列键与采集器生成的一致，不含写入器在发往 VictoriaMetrics 途中添加的 `dumbmonit_` 前缀。 |
| `POST` | `/api/targets/{id}/discover` | 重新运行配置文件检测：`{"profile_id": "host-resources"}` 或 `null`。 |

返回的设备：

```json
{
  "id": 4,
  "name": "Core switch",
  "address": "192.168.1.2",
  "kind": "snmp",
  "profile_id": "host-resources",
  "parent_id": null,
  "via_agent": null,
  "interval_secs": 60,
  "enabled": true,
  "tags": {"role": "network"},
  "credential_kind": "SNMP community",
  "last_probe_at": "2026-09-15 13:20:05",
  "last_error": null,
  "error_kind": null
}
```

机密本身绝不返回：`credential_kind` 只是一个标签。服务器时间戳为不带后缀的 UTC。`error_kind` 对 `last_error` 分类：`down`（设备没有应答）或 `config`（我们这边的问题——凭据、地址、选项），UI 将其显示为*不可达*或*配置错误*；上一次探测成功时为 `null`。

创建一个：

```json
{
  "name": "Core switch",
  "address": "192.168.1.10",
  "kind": "snmp",
  "interval_secs": 60,
  "parent_id": null,
  "tags": {},
  "credential": {"type": "snmp_community", "community": "public"}
}
```

凭据格式（`type`）：`none`；`snmp_community`（`community`）；`snmp_v3`（`username`，可选的 `auth: {protocol, passphrase}`，协议为 `md5`、`sha1`、`sha224`、`sha256`、`sha384` 或 `sha512`，可选的 `privacy: {protocol, passphrase}`，协议为 `des`、`aes128`、`aes192` 或 `aes256`，可选的 `context`）；`api_token`（`token`，或两部分 `token_id` 和 `secret`，服务器会为 Proxmox VE 和 PBS 将它们拼接成 `user@realm!name=secret`）；`username_password`（`username`、`password`）。`interval_secs` 默认为 60，不能低于 10。`name` 最多 200 个字符，`address` 最多 253 个（超出则返回 `400`）。`parent_id` 必须指向一个已存在的设备（`400 Parent device N not found.`）。类型选项以其键放在 `tags` 中。

`via_agent` 指明从自己网络探测设备（而不是由服务器探测）的[中继代理](../install/remote-site.md)；`null`（默认）表示由服务器探测。它必须是一个 `agent` 设备（否则 `400`），设备不能中继自己，`agent` 设备不能被中继。在 `PUT` 中，*省略*的 `via_agent` 会保留已存储的值，与 `profile_id` 和 `credential` 相同；发送 `null` 则恢复直接探测。对被中继设备的 `POST /api/targets/{id}/probe` 和 `/discover` 会经由中继并等待其应答（探测已在进行中时返回 `409`，中继没有应答时返回 `400 Timed out: relay agent …`）。

## 发现 {#discovery}

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `POST` | `/api/discovery` | 管理员 | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`。扫描网络中的 SNMP 设备。`cidr` 必填；`community` 默认为 `public`；`timeout_ms` 被限制在 100–10,000 之间。超过 4096 个地址的网络会被拒绝。扫描是带 JSON 正文的 `POST`，因此 community 绝不会出现在 URL 或日志行中。 |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## 首次运行向导 {#first-run-guide}

概览在新实例上显示的三个步骤。状态保存在实例上而不是浏览器中，因此跳过某一步就是在所有地方跳过。

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` | `/api/onboarding` | 会话 | 实例当前所处的状态。 |
| `PUT` | `/api/onboarding` | 管理员 | `{"skipped": true}` 将整个向导收起；`{"dismissed": ["test"]}` 将单个步骤收起。省略的字段保留其存储值；`device`、`channel`、`test` 之外的步骤 id 会返回 `400`。返回与 `GET` 相同的文档。 |

```json
{
  "skipped": false,
  "complete": false,
  "completed_at": null,
  "dismissed": [],
  "has_target": true,
  "has_channel": true,
  "notification_confirmed": false
}
```

`has_target`、`has_channel` 和 `notification_confirmed` 在每次读取时重新计算：它们是关于实例的事实，而不是界面设置的标志。`notification_confirmed` 在确实有消息从某个渠道发出之后为真——测试消息算数，仅仅创建了渠道则不算。

`complete` 是锁存的，永远不会变回 false。服务器在三个步骤都已解决（完成或已收起）时设置它，并且在首次读取一个已经有设备和渠道的实例时也会设置——因此升级一个已有的实例永远不会向它显示首次运行向导。

## 指标 {#metrics}

两个路由都代理 VictoriaMetrics，并以 Prometheus 的格式返回序列：`[{"metric": {labels…}, "values": [[timestamp_seconds, "value"], …]}]`。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | 即时 MetricsQL 查询。 |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | 范围查询。`start` 和 `end` 是自纪元起的**毫秒**；`step` 以秒为单位，并会被放宽，使一条序列绝不超过 2,000 个点。 |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## 抓取（`/api` 之外） {#scraping-outside-api}

供已有的 Prometheus 或 Grafana 使用。二者都接受 `Authorization: Bearer dmt_…` 中的 `read` API 令牌（会话 cookie 也可以）；抓取器不发送 `X-Requested-With` 头，也不需要——这些路由只读。可复制粘贴的 `scrape_config` 和 Grafana 数据源参见[指标参考](metrics.md#scraping-dumbmonit)。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/metrics` | 以 Prometheus 文本展示格式给出实例自身的健康状况：调度器、探测、写入、告警、通知、代理、数据库、VictoriaMetrics。几十条序列，绝不会每个设备一条。 |
| `GET` | `/federate` | 同一格式的测量数据。`match[]` 选择序列（可重复，最多 10 个，默认为 `{__name__=~"dumbmonit_.*"}`），`max_lookback` 放宽查找最后一个点的时间窗口（默认 `5m`）。超过 8 MiB 时返回 `400`，并说明应缩小什么。 |
| `GET`、`POST` | `/prometheus/api/v1/{route}` | Prometheus API 的读取部分，中继到存储，因此 Grafana 可以把 `http://…/prometheus` 用作 Prometheus 数据源。仅限 `query`、`query_range`、`query_exemplars`、`series`、`labels`、`label/{name}/values`、`metadata` 和 `status/buildinfo`；其他任何路由返回 `404`。 |

`DUMBMONIT_METRICS_PUBLIC=true` 会无需令牌地提供这三者——仅限在防火墙之后使用，因为它们包含每个设备的每一条测量数据。

## 其他路由 {#further-routes}

上文未详述的路由；OpenAPI 文档给出了它们的参数和响应格式。*read* 和 *write* 是所需的令牌范围；*none* 表示不接受任何令牌（仅会话）。

| 方法 | 路由 | 范围 | 用途 |
|---|---|---|---|
| `POST` | `/api/targets/reorder` | write | 重新排序设备。 |
| `GET` | `/api/targets/{id}/security` | read | 一个设备的安全评分、等级和检查项。 |
| `GET` | `/api/security/summary` | read | 每个已评级设备的安全评分，最差的在前。 |
| `GET` | `/api/targets/{id}/webchange/pages` | read | 网站监视所跟踪的页面。 |
| `GET` | `/api/targets/{id}/webchange/changes` | read | 在被监视网站上发现的变更，最新的在前。 |
| `GET` | `/api/targets/{id}/webchange/changes/{change_id}` | read | 一次变更，附两个版本及逐行差异。 |
| `GET` | `/api/targets/{id}/webchange/snapshots/{snapshot_id}/screenshot` | read | 快照的截图。 |
| `POST` | `/api/targets/{id}/webchange/check` | write | 立即检查被监视的网站。 |
| `GET` | `/api/targets/{id}/opnsense/gateways`、`/traffic`、`/health` | read | OPNsense：网关和 WAN 地址；接口、防火墙和 DHCP；系统、服务、隧道和固件。 |
| `GET` | `/api/targets/{id}/truenas/storage`、`/protection`、`/health` | read | TrueNAS：存储池、数据集和磁盘；清理任务和数据保护任务；系统、告警和服务。 |
| `GET` | `/api/targets/{id}/ad`、`/ad/findings` | read | Active Directory：域、控制器、特权组和发现项；原始安全发现。 |
| `GET` | `/api/targets/{id}/redfish` | read | 从管理控制器读取的服务器硬件信息。 |
| `GET` | `/api/targets/{id}/observability` | read | 日志或指标服务器的健康状况。 |
| `GET` | `/api/targets/{id}/backend` | read | Redis、MongoDB、RabbitMQ 或 CrowdSec 的健康状况。 |
| `GET` | `/api/targets/{id}/proxy` | read | 反向代理或域名注册的健康状况。 |
| `GET` | `/api/public/status/{slug}/badge.svg`、`/response.svg`、`/uptime.svg` | public | 状态页面的整体状态、平均响应时间和可用率徽章。 |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`、`/uptime.svg`、`/response.svg` | public | 页面中某一个服务的同样徽章。 |
| `GET` | `/api/public/status/{slug}/banner.js` | public | 供第三方网站使用的状态横幅脚本。 |
| `GET` | `/api/public/status/{slug}/rss` | public | 以 RSS 订阅源形式提供的事故。 |
| `GET` | `/api/music/now` | read | 墙面正在播放的内容（房间显示器）。 |
| `PUT`、`DELETE` | `/api/music/link` | write | 所有墙面播放此 Spotify、Deezer 或 YouTube 链接；或停止嵌入式播放器。 |
| `GET`、`DELETE` | `/api/music/spotify` | read、write | 已连接的 Spotify 账户（绝不含令牌）；断开并忘记其令牌。 |
| `POST` | `/api/music/spotify/authorize`、`/complete` | none | 开始连接 Spotify；完成回环连接。仅会话。 |
| `GET` | `/api/music/spotify/token` | none | 供墙面播放器使用的短期 Spotify 访问令牌。仅会话。 |
| `GET`、`PUT` | `/api/music/speaker` | read、write | 设置中显示的墙面 Spotify 扬声器；重命名它（管理员）。 |
| `POST` | `/api/music/speaker/report`、`/play` | none | 墙面报告其扬声器的状态；在其上播放该账户。仅会话。 |

## 告警 {#alerts}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/alerts` | 活跃的告警（pending、firing、suppressed、最近已解决）。已删除或已暂停设备的告警绝不会被列出。 |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | 阶段转换。`since` 为 RFC 3339，默认最近七天；`limit` 必须为正数。`dismissed`（默认 `false`）排除通过下面两个路由清除的条目；清除从不会销毁任何东西，只是在默认视图中隐藏。 |
| `POST` | `/api/alerts/history/{id}/dismiss` | 从默认历史视图中清除一条 `resolved` 条目。`204`。id 未知或转换仍处于活跃状态时返回 `404`。操作员；会被审计。 |
| `POST` | `/api/alerts/history/dismiss-resolved` | 一次清除所有尚未清除的 `resolved` 条目（“Clear all resolved”）。返回 `{"dismissed": <count>}`。操作员；会被审计。 |
| `POST` | `/api/alerts/{fingerprint}/ack` | 确认：`{"duration_secs": 14400, "note": "…"}` 或 `{"until": "2026-09-22T18:00:00Z"}`（二选一；都不提供则为 4 小时，最长 30 天）。`{"until": null}` 解除确认。返回该告警。操作员；会被审计。 |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | 解除确认。返回该告警。操作员。 |

一个活跃的告警：

```json
{
  "fingerprint": "…",
  "rule_uid": "disk_almost_full",
  "rule_name": "Disk almost full",
  "severity": "warning",
  "target_id": 4,
  "series_key": "…",
  "labels": {"host": "Core switch", "mountpoint": "/", "target": "4"},
  "phase": "firing",
  "effective_phase": "suppressed",
  "suppressed": true,
  "suppressed_by": 2,
  "silenced": false,
  "learning": false,
  "acked": true,
  "acked_until": "2026-09-15T17:20:00Z",
  "acked_by": "admin",
  "ack_note": "replacing the disk",
  "value": 95.96,
  "score": null,
  "condition_since": "2026-09-15T12:00:00Z",
  "firing_since": "2026-09-15T12:15:00Z",
  "last_eval_at": "2026-09-15T13:20:00Z",
  "last_notified_at": null,
  "notify_count": 0
}
```

历史条目包含 `id`、`fingerprint`、`rule_uid`、`target_id`、`from_phase`、`to_phase`、`severity`、`value`、`notified`、`reason`（为空，或说明为何没有发送任何内容：`learning: would have fired`、`suppressed: device 2 unreachable`、`maintenance window`、`acknowledged by admin`、`device removed or disabled`）、`at` 和 `dismissed`（已从活跃视图中清除；行本身会保留）。只要 `acked_until` 在未来，`acked` 就为真：提醒和升级暂停，恢复仍会通知并清除确认。已不存在的 `target_id` 在 UI 中显示为“(deleted device)”。

### 规则 {#rules}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/alerts/rules` | 每条规则。 |
| `POST` | `/api/alerts/rules` | 创建。`201`。 |
| `PUT` | `/api/alerts/rules/{id}` | 更新。内置规则保留其查询。 |
| `DELETE` | `/api/alerts/rules/{id}` | `204`。内置规则不能删除。 |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`。返回该规则。 |

一条规则：

```json
{
  "id": 2, "uid": "cpu_high", "name": "High CPU",
  "description": "CPU load sustained above 90%.",
  "kind": "threshold",
  "query": "avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)",
  "operator": ">", "threshold": 90, "for_secs": 600,
  "severity": "warning",
  "selector": {"kind": "all"},
  "channels": [],
  "params": {"k": 3.5, "alpha": 0.05, "mad_floor_abs": 1e-6, "mad_floor_rel": 0.01, "min_samples": 3},
  "unit": "%", "repeat_secs": 21600, "escalate_after_secs": 3600,
  "enabled": true, "builtin": true
}
```

创建规则至少需要 `name` 和 `query`；`kind`（`threshold`、`anomaly`、`predict`）、`operator`、`threshold`、`for_secs`、`severity`（`info`、`warning`、`critical`）、`selector`（`{"kind":"all"}`、`{"kind":"ids","ids":[…]}` 或 `{"kind":"labels","labels":{…}}`）、`channels`（渠道 id；为空表示每个已启用的渠道）、`unit`、`repeat_secs`、`escalate_after_secs` 和 `enabled` 是可选的。

### 静默（维护窗口） {#silences-maintenance-windows}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/alerts/silences` | 每个窗口，附 `active_now`、`active_until` 和 `next_start_at`（RFC 3339，由服务器计算）。 |
| `POST` | `/api/alerts/silences` | 创建。`201`。操作员。 |
| `DELETE` | `/api/alerts/silences/{id}` | `204`。操作员。 |

```json
{
  "name": "Sunday backups",
  "comment": "NAS is busy",
  "target_id": 4,
  "matchers": {},
  "schedule": {"kind": "weekly", "days": [6], "start_minute": 120, "end_minute": 240, "timezone": "Europe/Paris"},
  "enabled": true
}
```

一次性计划为 `{"kind": "once", "starts_at": "2026-09-20T22:00:00Z", "ends_at": "2026-09-21T02:00:00Z"}`。每月计划为 `{"kind": "monthly", "days": [1], "nth_weekdays": [{"nth": 1, "weekday": 6}], "start_minute": 120, "duration_minutes": 120, "timezone": "Europe/Paris"}`——`nth` 为 1 到 5，或 `-1` 表示该月最后一次；没有该次数的月份会被跳过。星期的取值为 0 = 星期一 … 6 = 星期日（每月的 `days` 为 1–31）；分钟数为自本地午夜起（0–1439）。对于周期性计划，`timezone`（IANA 名称）优先于 `utc_offset_minutes`，并且能让窗口在夏令时变化时仍保持在其本地时刻；未知的时区名称会被拒绝。

### 每设备覆盖 {#per-device-overrides}

可以针对某一个设备调整规则，而无需改动规则本身。`{"enabled": false}` 就是 UI 的**忽略**操作（在告警、设备页面或规则上）所写入的内容：该设备被永久（但可逆）地从这一条规则中剔除，而不会对其他任何人禁用该规则。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | 每个覆盖，或某个设备的覆盖：`rule_uid`、`target_id`、`threshold`、`clear_threshold`、`enabled`。 |
| `GET` | `/api/alerts/rules/{id}/overrides` | 一条规则的覆盖。 |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}`——每个字段可选；`null` 表示“与规则相同”。创建或替换。操作员只能发送 `{"enabled": false}`（忽略）；该设备的阈值覆盖（如果有）会保留。 |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`。对操作员而言，只解除忽略：阈值覆盖保留。 |

## 通知策略 {#notification-policy}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/notify/policy` | 全局策略：`batch_window_secs`（0 = 立即发送）、每个渠道的 `max_per_hour`（0 = 不限）、`flap_events`、`flap_window_secs`、`flap_hold_secs`（0 = 不检测抖动）、`public_url`（消息中的链接；为空 = `DUMBMONIT_PUBLIC_URL`）、`escalate_after_secs` 和 `escalate_channel`（0 / `null` = 不升级）。 |
| `PUT` | `/api/notify/policy` | 相同的字段，每个都是可选的：省略的字段保留其值。`"escalate_channel": null` 关闭升级。返回该策略。 |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → 渠道路由过滤器会选中哪些设备：`devices`（`id`、`name`、`kind`、`tags`、`matched`）、`matched`、`total`、`rules`、`excluded_rules`。 |

## 通知渠道 {#notification-channels}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/notify/kinds` | 每种渠道类型及其 `settings` 和 `secrets` 字段（`key`、`label`、`required`、`input`、`help`、`placeholder`、`options`、`shape`、`default`），以及 `doc_url`，例如 `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`。 |
| `GET` | `/api/notify/channels` | 每个渠道：`id`、`name`、`kind`、`enabled`、`settings`、`has_secret`、`last_error`、`last_sent_at`、`policy`（`min_severity`、`notify_resolved`、`min_interval_secs`、`quiet_hours`、`matcher`）。绝不返回机密。 |
| `POST` | `/api/notify/channels` | 创建。`201`。 |
| `PUT` | `/api/notify/channels/{id}` | 更新。省略 `secrets` 会保留已存储的机密；`"secrets": {}` 会清除它们。在 `policy` 中，省略 `matcher` 会保留路由过滤器，`"matcher": null` 会清除它。 |
| `DELETE` | `/api/notify/channels/{id}` | `204`。 |
| `POST` | `/api/notify/channels/{id}/test` | 发送测试消息：`{"ok": true, "message": "…"}`。 |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

每种类型的确切键来自 `/api/notify/kinds`，并记录在[通知渠道](../notifications.md)中。

### Web Push 设备 {#web-push-devices}

**已登录账户**的设备（仅会话，包括查看者；API 令牌会得到 `401`）。`webpush` 渠道会向它们发送；参见[推送通知（PWA）](../using/push-notifications.md)。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`：VAPID 公钥（base64url，即订阅时使用的 `applicationServerKey`）以及该账户的设备——`id`、`device`（`Firefox on Android`）、`push_service`（主机）、`fingerprint`（端点的 SHA-256 的前 16 个十六进制字符）、`created_at`、`last_success_at`、`last_error`。端点、浏览器密钥和私钥绝不返回。 |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`：`{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`。`201` 并返回该设备。端点必须是公共地址上的 HTTPS：私有、回环和链路本地地址会得到 `400`。同一端点再次提交会替换其行。 |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`；另一个账户的设备返回 `404`。 |
| `POST` | `/api/webpush/test` | `{}` 或 `{"id": 3}`。发送测试通知：`{"ok", "delivered", "failed", "removed", "message"}`。返回 `404`/`410` 的订阅会被移除。 |

## 邮件报告 {#email-reports}

仅限管理员，包括 `GET`（响应会列出收件人）。Bearer 令牌进行更改需要 `write` 范围。参见[邮件报告](../using/reports.md)。

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | 管理员 | 每个报告：`id`、`name`、`enabled`、`frequency`、`weekday`（0 是星期一）、`day_of_month`、`hour`、`timezone`、`recipients`、`channel_id`、`last_sent_at`、`last_error`、`next_run_at`。 |
| `POST` | `/api/reports/schedules` | 管理员 | 相同的可写字段；各自默认为每周、星期一、8:00、`UTC`。`recipients` 最多 20 个地址，启用报告时必填；`channel_id` 必须是 `smtp` 渠道或 `null`。`201`；超过 10 个报告时返回 `409`。 |
| `GET` | `/api/reports/schedules/{id}` | 管理员 | 一个报告。 |
| `PUT` | `/api/reports/schedules/{id}` | 管理员 | 完整替换。 |
| `DELETE` | `/api/reports/schedules/{id}` | 管理员 | `204`。 |
| `POST` | `/api/reports/schedules/{id}/send-test` | 管理员 | 向已保存的收件人发送预览邮件：`{"sent": 2, "failed": 0}`。不影响计划。没有可用的邮件渠道时返回 `400`；距上一次预览不足 30 秒时返回 `409`。 |

## 状态页面与事故 {#status-pages-and-incidents}

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` | `/api/status-pages` | 会话 | 每个页面及其条目：`page`（`id`、`slug`、`title`、`description`、`published`、`theme`、`show_uptime_days`、`created_at`、`updated_at`）和 `items`（`id`、`page_id`、`target_id`、`label`、`group_name`、`position`）。 |
| `POST` | `/api/status-pages` | 管理员 | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`。`title` 必填（最多 120 个字符）；省略 `slug`（`^[a-z0-9-]{2,40}$`）时由标题派生；`theme` 为 `auto`（默认）、`light` 或 `dark`；`show_uptime_days` 范围 7 到 90（默认 90）。`201`；slug 已被占用时返回 `409`。 |
| `GET` | `/api/status-pages/{id}` | 会话 | 一个页面及其条目。 |
| `PUT` | `/api/status-pages/{id}` | 管理员 | 相同的字段，但是完整替换而不是补丁：`title` 必填，每个省略的字段都回到默认值（slug 重新由标题派生、描述为空、`published: false`、`theme: auto`、90 天可用率）。 |
| `DELETE` | `/api/status-pages/{id}` | 管理员 | `204`。公开 URL 不再响应。 |
| `POST`、`PUT` | `/api/status-pages`、`/api/status-pages/{id}` — 外观与订阅 | 管理员 | 还接受 `accent`（`default`、`blue`、`teal`、`violet`、`rose`、`amber`）、`scenes`（来自编辑器列表的场景 id——城市、新奇迹和古代奇迹，例如 `venice`、`paris`、`giza`、`tajmahal`，不得重复；`[]` 表示没有；创建时省略则为 `["paris"]`）、`simple`（布尔值，默认 `false`：公开页面不显示场景、吉祥物或动画，并跟随访客的明暗偏好）、`scene_rotation`（`visit`、`1m`、`10m` 或 `1h`）、`footer_text`（纯文本，280 个字符）、`homepage_url`（`http(s)://` 或空）、`subscribe_channel_id`（一个 `smtp` 渠道，或 `null` 以关闭邮件订阅）以及 `domain`（页面的公开主机名，如 `status.example.com`，或 `null`/`""` 表示没有——参见[自定义域名](../using/status-pages.md#custom-domain)）。`domain` 会转为小写并去掉 `https://` 前缀或尾部斜杠；带端口、路径或 IP 地址会返回 `400`，DumbMonit 自身的地址（其公开 URL，或请求进入时使用的主机）返回 `400`，另一个页面已使用的名称返回 `409`。与其他字段不同，这八个键在 `PUT` 中被省略时会保留其存储值。页面视图会连同 `logo_type`（`image/png`、`image/jpeg`、`image/webp` 或 `null`）一并返回它们。 |
| `GET`、`PUT`、`DELETE` | `/api/status-pages/{id}/logo` | 会话 / 管理员 | 页面的徽标。`PUT` 接受 `{"data": "<base64 或 data: URL>"}`：按内容识别 PNG、JPEG 或 WebP，最大 256 KiB，其他一切返回 `400`。`204`。`GET` 是编辑器的预览（包括草稿）。 |
| `GET` | `/api/status-pages/{id}/subscribers` | 会话 | 邮件订阅者：`id`、`page_id`、`email`、`confirmed_at`（待确认时为 `null`）、`created_at`。其令牌绝不离开服务器。 |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | 管理员 | 移除一个地址。`204`。 |
| `PUT` | `/api/status-pages/{id}/items` | 管理员 | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]`——页面上显示的设备的完整有序列表。 |
| `GET` | `/api/incidents` | 会话 | 每个事故及其更新：`incident`（`id`、`page_id`、`title`、`kind`、`status`、`severity`、`starts_at`、`ends_at`、`created_at`、`updated_at`）和 `updates`。 |
| `POST` | `/api/incidents` | 管理员 | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}`——`body` 是第一条更新。`201`。 |
| `PUT` | `/api/incidents/{id}` | 管理员 | 相同的字段；省略的字段保留其值。 |
| `DELETE` | `/api/incidents/{id}` | 管理员 | `204`。 |
| `GET` | `/api/incidents/{id}/updates` | 会话 | 时间线：`id`、`incident_id`、`status`、`body`、`created_at`。 |
| `POST` | `/api/incidents/{id}/updates` | 管理员 | `{"status": "monitoring", "body": "…"}`。追加一条更新并将事故转为 `status`。`201`。 |
| `GET` | `/api/public/status/{slug}` | 公开 | 已发布页面所基于的 JSON 文档（同一路径下还有 `badge.svg` 和 `rss`）。在 `page` 中，`show_uptime_days` 是配置的最大值，`history_days` 是实际显示的窗口（页面数据的年龄，从 7 天到该最大值），`simple` 反映简单模式。参见[状态页面](../using/status-pages.md)。在页面的自定义域名上（`Host` 等于其 `domain`），只有该页面的 `/api/public/status/{slug}/…` 路由有响应；本参考中的其他所有路由在那里都返回 `404`。 |
| `GET` | `/api/public/status/{slug}/uptime.svg`、`/response.svg` | 公开 | 页面徽章：`?days=` 为 `1`、`7`、`30`（默认）或 `90` 的平均可用率——超过页面历史范围时返回 `400`——以及平均响应时间。SVG，`Cache-Control: public, max-age=60`。 |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`、`/uptime.svg`、`/response.svg` | 公开 | 某一个服务的同样三个徽章；`key` 是公开文档中该服务的 `key`（其标签的 URL 形式）。未知的 key 返回 `404`。 |
| `GET` | `/api/public/status/{slug}/logo` | 公开 | 页面的徽标及其图像类型，可缓存一天（文档的 `logo_url` 带有版本）。没有徽标时返回 `404`。 |
| `POST` | `/api/public/status/{slug}/subscribe` | 公开 | `{"email": "…"}`。通过页面的 SMTP 渠道发送确认链接邮件。无论地址处于何种状态，始终返回 `202` 和同一条消息；地址无效返回 `400`；页面不提供邮件订阅时返回 `404`；页面无法再接受更多订阅者时返回 `409`；超过每个客户端五次、每 15 分钟总共六十次的请求时返回 `429` 并带 `Retry-After`。 |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | 公开 | 确认订阅（确认邮件中的链接打开 `/s/{slug}/confirm`，其按钮向这里 POST）。令牌未知或已过期时返回 `404`。 |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | 公开 | 移除订阅；正文会被忽略，因此它也是 `List-Unsubscribe` 的 RFC 8058 一键退订目标。始终返回 `200`。 |

## 代理令牌与数据接收 {#agent-tokens-and-ingest}

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | 仅会话 | 每个令牌：`id`、`name`、`prefix`、`created_at`、`last_used_at`、`revoked_at`、`max_uses`（设备群令牌为 `null`）、`uses`、`expires_at`。 |
| `POST` | `/api/agent/tokens` | 管理员，仅会话 | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`。`201`，返回令牌字段外加 `secret`（只显示一次）、`install_linux` 和 `install_windows`。`base_url` 是代理将使用的 URL；默认为监听地址。没有 `reusable` 时，令牌是**一次性**的：只注册一台机器，不能再多。有它时，`max_uses` 限制注册次数（`null`：无限制）。`expires_in_days` 在这么多天之后停止*注册*；已注册的机器继续上报。次数小于 1 时返回 `400`。 |
| `DELETE` | `/api/agent/tokens/{id}` | 管理员，仅会话 | 吊销。`204`；未知或已吊销时返回 `404`。 |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | 接收代理发来的一批样本（正文最大 16 MB）。不应手动调用。该头携带机器的绑定密钥；尚无密钥的代理省略它，这就是它请求密钥的方式。响应会增加 `agent_secret`（密钥，在机器被绑定时仅返回一次）和 `bound`。当机器被绑定到另一个代理安装，或令牌已无法再注册时返回 `403`——消息会说明是哪种情况以及该怎么做。 |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | 委派给中继代理（`relay: true`）的探测。没有待处理项时最多保持 `wait` 秒（最多 25 秒）。每一项都是 `probe` 类型的命令，其 `args` 携带目标、其解密后的凭据、`timeout_secs` 和 `discover`。绝不写入磁盘。 |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | 被委派探测的结果：`{"duration_ms", "error", "samples", "profile_id"}`（正文最大 16 MB）。`204`；探测已过期或属于另一个代理时返回 `404`。 |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | 为代理排队的命令（容器重启或更新），没有待处理项时最多保持 `wait` 秒。不应手动调用。`key` 本身说明不了什么——它是公开的——所以已绑定的机器还必须出示其绑定密钥；否则返回 `403`，这正是阻止一台机器拿走另一台机器命令的机制。 |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | 由代理报告的命令进度和结果。`result` 被截断为最后 4 KB，发生截断时开头会加上 `[truncated by the server, beginning dropped]`。 |
| `GET` | `/api/relays` | 会话 | 每个可能作为中继的代理设备：`id`、`name`、`site`、`relay`（声明了 `relay: true`）、`last_seen_at`、`relayed`（经由它到达的设备）。中继在前，然后按名称排序。 |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## 容器与命令（代理设备） {#containers-and-commands-agent-devices}

对运行代理的机器上的容器执行操作。当设备不是 `agent` 目标时，每个路由都返回 `404`。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | 代理最近一次描述的机器：`hostname`、`os`、`os_version`、`arch`、`agent_version`、`commands_supported`、`relay`、`site`、`relayed`（经由此代理到达的设备）、`last_seen_at`、`binding`、`bound`、`bound_at`、`rebind_until`。在代理上报之前返回 `404`。只有当代理声明它会拉取命令（`commands: true`，当前代理的默认值）时 `commands_supported` 才为 `true`；较旧的代理或 `commands: false` 的代理为 `false`。`binding` 为 `bound`、`pending`（该二进制文件可以被绑定，并将在其下一批数据时绑定）或 `unsupported`（早于绑定功能的代理：请重新安装）。 |
| `POST` | `/api/targets/{id}/agent/rebind` | 管理员。开启一个一小时的窗口，在此期间这台机器可以用有效的注册令牌再次绑定自己——这是重新安装带走了代理密钥之后重新进入的途径。`{"rebind_until", "minutes"}`。窗口一经使用即关闭，旧密钥随即失效。在代理上报之前返回 `404`。 |
| `GET` | `/api/targets/{id}/containers` | 上一批数据中的每个容器：`name`、`image`、`up`、`health`、`restart_count`、`uptime_seconds`、`image_age_seconds`、`update_available`、`policy`、`last_command`。 |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`。每个字段都是可选的：省略的字段保留其存储值。返回完整的策略。 |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | 排队一次重启。`201` 并返回该命令。`name` 不在代理的清单中时返回 `404`；相同命令已在排队或运行，或代理无法运行命令时（见 `commands_supported`）返回 `409`。 |
| `POST` | `/api/targets/{id}/containers/{name}/update` | 排队一次更新；可选的 `{"prune": bool}`（默认取自策略）。状态码与重启相同。 |
| `GET` | `/api/targets/{id}/commands` | 最近 20 条命令，最新的在前：`id`、`kind`（`container.restart`、`container.update`）、`args`、`status`、`requested_by`（用户，或 `policy`）、`created_at`、`started_at`、`finished_at`、`result`。 |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | 取消一条排队中的命令。`204`；代理已领取它或它已结束时返回 `409`；它不属于此设备时返回 `404`。 |

`status` 依次为 `queued → running → done | failed`。另有两种最终状态：`cancelled`（有人撤回了命令）和 `expired`（服务器在十分钟后放弃，因为没有代理来领取——代理已停止、太旧而不支持命令通道，或以 `commands: false` 安装）。服务器每分钟自行让过期的命令失效；排队中的命令阻塞新命令的时间绝不会超过这个时长。

## Proxmox VE 客户机 {#proxmox-ve-guests}

Proxmox VE 设备的**客户机**面板中的各行，由指标存储中的最近一次探测组装而成（四个即时查询，不向虚拟机管理程序请求任何内容）。设备不存在时返回 `404`，不是 `proxmox` 目标时返回 `400`。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | 每台 VM 或容器一个对象，按节点再按 VMID 排序：`vmid`、`name`、`node`、`kind`（`qemu`、`lxc`）、`status`（`running`、`stopped`、`paused`、`suspended`、`template`、`unknown`）、`cpu_percent`（占分配核心的比例）、`cpu_count`、`memory_used_bytes`、`memory_total_bytes`、`memory_percent`、`balloon_bytes`、`disk_used_bytes`、`disk_total_bytes`、`disk_percent`、`agent`（`true` 客户机代理已应答，`false` 已启用但沉默，`null` 无）、`network_in_bps`、`network_out_bps`、`disk_read_bps`、`disk_write_bps`、`uptime_seconds`、`last_backup_age_seconds`、`ha_state`、`pool`（其资源池）、`lock`（仅在持有锁时：`backup`、`migrate`……）、`os` 和 `ip`（从客户机内部看到的，每小时刷新）。每个未知值都是 `null`——没有客户机代理的 VM 有 `disk_total_bytes` 但 `disk_used_bytes: null`；已停止的客户机保留其大小，失去其测量值。 |

```json
[
  {
    "vmid": 202, "name": "nextcloud", "node": "pve2", "kind": "lxc", "status": "running",
    "cpu_percent": 1.0, "cpu_count": 4,
    "memory_used_bytes": 1879048192, "memory_total_bytes": 4294967296, "memory_percent": 43.75, "balloon_bytes": null,
    "disk_used_bytes": 61203283968, "disk_total_bytes": 107374182400, "disk_percent": 57.0, "agent": null,
    "network_in_bps": 1024.0, "network_out_bps": 512.0, "disk_read_bps": 0.0, "disk_write_bps": 2048.0,
    "uptime_seconds": 3196800, "last_backup_age_seconds": 25200, "ha_state": "started",
    "pool": "production", "lock": null,
    "os": "Debian GNU/Linux 12 (bookworm)", "ip": "192.168.10.60"
  }
]
```

## Proxmox VE 节点与 Ceph {#proxmox-ve-nodes-and-ceph}

同一设备页面的**节点**和 **Ceph** 部分，读取自同一次最近探测。状态码同上。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}`——HA 看门狗状态（`armed`、`standby`……）以及它是否真的会隔离丢失的节点，然后是每个节点一个对象，按名称排序：`name`、`up`、`cpu_percent`、`cpu_iowait_percent`、`memory_percent`、`ksm_shared_bytes`、`rootfs_percent`、`uptime_seconds`、`version`（该节点上安装的 `pve-manager` 版本）、`services_down`（已停止或失败的守护进程的单元名称）、`interfaces_offline`（设置为开机启动但未启用的接口）、`thin_pools`（`name`、`vg`、`used_percent`、`metadata_used_percent`、`size_bytes`）和 `volume_groups`（`name`、`used_percent`、`size_bytes`）。停止应答的节点保留其条目，`up: false`。 |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available`（集群没有 Ceph 时为 `false`——此时 UI 完全不绘制该部分）、`health`（0 OK、1 WARN、2 ERR、3 未知）、`health_status`、`bytes_used`、`bytes_total`、`used_percent`、`osds_total`、`osds_up`、`osds_in`、`osds`（`name`、`host`、`device_class`、`up`、`in`、`used_percent`、`used_bytes`、`total_bytes`、`apply_latency_ms`、`commit_latency_ms`）、`pools`（`name`、`used_percent`、`used_bytes`、`size`、`min_size`、`pg_num`、`pg_num_optimal`、`autoscale`）、`filesystems`（CephFS 名称）、`flags`（当前设置的 OSD 标志，如 `noout`）和 `muted_checks`（被静默的健康检查，`HEALTH_OK` 不再提及它们）。 |

## Proxmox Backup Server {#proxmox-backup-server}

`pbs` 设备的各面板，读取自最近一次探测（除任务日志外不向服务器请求任何内容）。设备不存在时返回 `404`，不是 `pbs` 目标时返回 `400`。

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`：每个备份组一项（`datastore`、`namespace`、`backup_type`、`backup_id`、`name`、`count`、`last_time`、`last_size`、`last_verified`、`last_success`、`last_failure`、`retention`），附一个由 `{date, state, runs, snapshot}` 组成的 `days` 数组。`offset` 是查看者相对 UTC 的偏移分钟数，使天数按本地午夜切分。 |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | 失败的任务：`upid`、`worker_type`、`kind`、`worker_id`、`datastore`、`start`、`end`、`error`。 |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`：同步、验证、修剪和垃圾回收任务，附其计划和上次结果。 |
| `GET` | `/api/targets/{id}/pbs/health` | 数据存储（使用率、预计占满日期）、磁盘（SMART、磨损度）和 ZFS 池。 |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | 一个任务的日志，从服务器获取：`{"upid", "lines": […]}`；`lines` 限制返回的行数。 |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | 一块磁盘的 SMART 属性；`disk` 必填（缺少时返回 `400`）。 |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager：整体规模总计（`estate`）以及每个联邦实例一行（`id`、`kind`、`reachable`、`error`、`version`、`version_behind`、节点和客户机数量、内存和存储、`subscription`、`last_collection`、`tasks_failed`），不可达的在前。 |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | 整个规模中失败的任务：`upid`、`remote`、`worker_type`、`kind`、`worker_id`、`node`、`start`、`end`、`error`。 |
| `GET` | `/api/targets/{id}/pdm/health` | 控制台主机：CPU、内存、根文件系统、运行时间、证书、待处理更新和订阅。 |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway：按阅读顺序排列的四个 Postfix 队列（`queue`、`messages`、`domains`、`oldest_age_seconds`、`top_domains`、`stuck`），外加 `total_messages` 和网关范围的 `stuck`。`oldest_age_seconds` 是下界：qshape 报告的是年龄区间。 |
| `GET` | `/api/targets/{id}/pmg/traffic` | 今日总计（`mail`）、近期流量曲线（`recent`）、垃圾邮件评分直方图、今日拦截的病毒，以及隔离区计数（`quarantine`）。仅有计数：不含邮件内容。 |
| `GET` | `/api/targets/{id}/pmg/health` | 每个节点一项（系统、服务、带 `family`、`age_seconds` 和 `stale` 的 `signatures`、`expiring_certificates`、更新、订阅），带同步状态的 `cluster` 成员，以及跨节点的 `stopped_services`。 |

## Synology {#synology}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | 来自最近一次探测的 `synology` 设备概览：`system`（型号、DSM 版本、运行时间、温度、CPU、内存）、`volumes`（`name`、`fs_type`、`raid_type`、`status`、`severity`、`total_bytes`、`used_bytes`）、`disks`（`model`、`serial`、`kind`、`ssd`、`status`、`smart_status`、`temperature_celsius`、`size_bytes`、`remaining_life_percent`、……）和 `sampled_at`。设备不存在时返回 `404`，不是 Synology 时返回 `400`。 |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business：`tasks`，以及 `devices`——每台受保护设备一份报告，其学习到的节奏在 `assessment` 中（`state`、`last_success_s`、`typical_interval_s`、30 天计数）。 |

## 心跳（推送监控） {#heartbeats-push-monitors}

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `GET` 或 `POST` | `/api/push/{token}` | 无（令牌即认证） | 任务每次运行时发起的调用。可选的 `?status=up\|down&msg=…`（兼容 Uptime Kuma）。成功时返回空的 `204`，令牌未知返回 `404`，其他 `status` 返回 `400`，每个令牌每分钟超过六十次调用返回 `429` 并带 `Retry-After`。参见[心跳](../devices/push.md)。 |
| `GET` | `/api/targets/{id}/push` | 会话 | `push` 设备的监控：`token`、`path`（`/api/push/<token>`）、`last_seen_at`、`last_seen_age_secs`、`last_status`、`last_message`、`received_total`、`expected_interval_secs`、`grace_secs`、`settings_error`、`verdict`（`waiting`、`on_time`、`missed`、`reported_down`）。首次读取时创建。设备不是心跳设备时返回 `400`。 |
| `POST` | `/api/targets/{id}/push/regenerate` | 管理员 | 新令牌；此后旧 URL 返回 `404`。正文与读取相同。 |

## 助手（MCP） {#assistants-mcp}

内置的 [Model Context Protocol](https://modelcontextprotocol.io) 服务器，即助手与之通信的那一个。它使用上述 API 令牌认证——相同的范围、有效期、网络和速率限制——绝不使用会话。

| 方法 | 路由 | 认证 | 用途 |
|---|---|---|---|
| `POST` | `/api/mcp` | API 令牌 | 基于 Streamable HTTP 传输的 JSON-RPC 2.0，每个 `POST` 一个请求，以 JSON 应答。协议 `2026-07-28`（无状态：每个请求在 `_meta` 中携带其版本和客户端能力，外加 `MCP-Protocol-Version`、`Mcp-Method` 和 `Mcp-Name` 头；`server/discover`），以及通过 `initialize` 握手的 `2025-11-25`、`2025-06-18` 和 `2025-03-26`。没有会话：不颁发 `Mcp-Session-Id`，发送的会被忽略。通知得到 `202` 且没有正文。 |
| `GET`、`DELETE` | `/api/mcp` | 公开 | 返回 `405` 并带 `Allow: POST`——没有服务器发送的流，也没有可结束的会话；正文只说明此端点是什么。 |

`tools` 是唯一的能力——没有资源、没有提示词、没有 OAuth。在 27 个工具中，`read` 令牌可以调用其中 15 个只读工具（`get_status`、`list_devices`、`get_device`、`list_alerts`、`alert_history`、`query_metrics`、`list_silences`、`list_rules`、`list_device_types`、`list_agents`、`list_containers`、`list_heartbeats`、`list_status_pages`、`list_channels`、`list_packs`）；会更改内容的 12 个（`silence_device`、`remove_silence`、`acknowledge_alert`、`probe_device`、`set_device_enabled`、`set_rule_enabled`、`add_device`、`discover_network`、`schedule_maintenance`、`restart_container`、`test_channel`、`post_incident`）需要 `write` 令牌。这些工具调用与 Web UI 相同的代码。版本、头、错误代码和安全说明见[连接助手](../using/assistant.md#protocol-details)。

## 代理文件（`/api` 之外） {#agent-files-outside-api}

| 方法 | 路由 | 用途 |
|---|---|---|
| `GET` | `/install.sh` | Linux、macOS 和 FreeBSD 安装程序。公开。 |
| `GET` | `/install.ps1` | Windows 安装程序。公开。 |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`、`dumbmonit-agent-linux-aarch64`、`dumbmonit-agent-freebsd-x86_64`、`dumbmonit-agent-windows-x86_64.exe`，由 `DUMBMONIT_AGENT_DIR` 提供；`{name}.sha256` 以 `sha256sum` 格式给出其校验和。文件不存在时返回 `404`。`dumbmonit-agent-macos-aarch64` 和 `dumbmonit-agent-macos-x86_64`（及其 `.sha256`）以 `307` 重定向到最新 GitHub 发布版本上的同一文件。公开。 |

其他所有路径都由 Web UI 提供，它会自行要求你登录。
