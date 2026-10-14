# 保证论证

本页论证 DumbMonit 满足其安全需求:它陈述威胁、信任发生变化的边界、设计为何能限制失误造成的损害、
每一类常见弱点如何被应对,以及每项主张背后的证据。它是对[安全模型](security-model.md)的扩展,
后者更简要地列出了相同的措施。如需报告漏洞,请遵循
[安全策略](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md)。

下文中的文件路径均相对于仓库根目录。

## 安全需求

1. 只有已通过身份验证且角色允许的用户,才能读取或修改配置、告警或测量数据。
2. DumbMonit 为您的设备存储的凭据,无法从数据库文件、没有口令的备份包或 API 中读取。
3. 只有持有有效令牌的机器才能写入测量数据,并且泄露的令牌可以单独撤销。
4. DumbMonit 不能被变成访问不应访问的服务的途径,尤其是其自身不带身份验证的 VictoriaMetrics 子进程。
5. 提供给浏览器的页面不能运行并非随其发布的脚本,第三方站点也无法让已登录的浏览器更改任何内容。

## 威胁模型

| 攻击者 | 能力 | 是否在范围内 |
|---|---|---|
| 匿名网络客户端 | 能访问 HTTP 端口 | 是 |
| 低角色的已登录用户 | 有效会话或只读 API 令牌 | 是(权限提升) |
| 恶意网页 | 在管理员的浏览器中运行 | 是(CSRF、XSS、点击劫持) |
| 被攻破或恶意的被监控设备 | 以恶意内容响应探测 | 是(解析器、重定向) |
| 恶意或被盗的代理 | 持有一个代理令牌 | 是(单个令牌的影响范围) |
| 输入恶意 URL 的管理员 | 配置 webhook 或设备地址 | 是,针对服务器自身的地址 |
| 掌控主机、`/data` 卷或容器运行时的人 | 完全控制 | 否:不在范围内,与安全策略一致 |
| VictoriaMetrics 或被监控设备内部的漏洞 | 上游代码 | 否:请向上游报告 |

## 信任边界

| 边界 | 跨越边界的内容 | 如何防护 | 证据 |
|---|---|---|---|
| 浏览器到 `/api/*` | 会话 cookie 或 API 令牌、JSON | 一个身份验证中间件、三种角色、对基于 cookie 验证的写操作要求 CSRF 标头 | `crates/server/src/auth/middleware.rs`;测试 `auth_api.rs`、`operator_role_api.rs`、`api_open_security_api.rs`、`cors_policy_api.rs` |
| 浏览器到所提供的页面 | HTML、脚本、样式 | 带有每个响应独立 nonce 的内容安全策略、`nosniff`、`Referrer-Policy: same-origin`、`X-Frame-Options: DENY` | `crates/server/src/api/mod.rs`(安全标头层) |
| 身份提供商到服务器 | OpenID Connect 重定向和 ID 令牌 | State、nonce 和 PKCE(S256);本地账户关联是显式的 | `crates/server/src/auth/oidc/`;测试 `oidc_api.rs` |
| 代理到 `/api/ingest` | 测量数据、命令结果 | 每个代理一个令牌,以 SHA-256 哈希存储(`dmon_` 前缀) | `crates/server/src/api/ingest.rs`、`crates/server/src/collectors/agent/token.rs`;测试 `agent_binding_api.rs` |
| 中继代理到服务器 | 来自另一个网络的探测结果 | 相同的代理令牌检查(`agent_commands::authenticate`) | `crates/server/src/api/relay.rs`;测试 `relay_api.rs` |
| 服务器到被监控设备 | 探测请求、恶意响应 | 仅跟随同一主机内的重定向、重定向次数有限、响应解析器带有单元测试、默认启用证书验证 | `crates/collectors/src/http.rs`(`same_host_redirects`) |
| 服务器到通知渠道 | Webhook 和 SMTP 请求 | 相同的重定向策略,外加一道拒绝服务器自身地址的防护,在发送前、每次重定向时以及 DNS 解析时都会检查 | `crates/server/src/notify/guard.rs`;测试 `notify_guard_api.rs` |
| 服务器到 VictoriaMetrics | 写入和读取查询 | 内置子进程默认监听 `127.0.0.1:8428`;兼容 Prometheus 的代理只转发读取路由(query、query_range、series、labels) | `crates/server/src/config.rs`(`DUMBMONIT_VM_LISTEN`)、`crates/server/src/api/prometheus.rs`(`readable_segments`) |
| 服务器到 SQLite | 配置和状态 | 通过 `sqlx` 使用绑定参数 | `crates/server/src/db/` |
| 服务器到 `/data` | 数据库、密钥 | 机密使用 AES-256-GCM 加密;容器用户仅拥有 `/data` | `crates/server/src/crypto.rs`、`Dockerfile` |

## 安全设计论证

该论证遵循 Saltzer 和 Schroeder 的设计原则。

- **机制精简。** 一个二进制文件、一个 SQLite 文件、一个回环子进程,所有客户端只有一个入口
  (`/api/*`)。
- **故障安全的默认值。** 在全新的实例上,身份验证守卫下的每个路由都返回 `401`;只有健康检查、
  认证状态、设置和登录会响应,因此没有人能抢在所有者之前对实例做准备
  (`auth/middleware.rs`,测试 `api_open_security_api.rs`)。除非运维人员更改,VictoriaMetrics
  只监听回环地址。除非运维人员设置 `insecure_tls`,被监控服务的 TLS 证书都会被验证
  (`crates/collectors/src/http.rs`)。
- **完全仲裁。** 身份验证中间件对每个 API 路由都会运行,而不是由各个处理函数自行检查。
- **开放设计。** 代码、API 描述(`crates/server/src/api/openapi.yaml`)和本页都是公开的。
  安全性不依赖于设计的保密。
- **权限分离。** 会话、API 令牌和代理令牌是不同的凭据。API 令牌带有 `read` 或 `write` 范围
  (`auth/token.rs`),用户则具有查看者、操作员或管理员角色。
- **最小权限。** 镜像以 UID 65532 运行(`Dockerfile`);Compose 文件丢弃所有 capability,
  设置只读根文件系统和 `no-new-privileges`(`docker-compose.yml`)。
- **最小公共机制。** 代理使用各自的令牌进行身份验证,因此撤销一台机器不会影响任何用户或其他代理。
- **心理可接受性。** 安全行为是默认的,无需额外步骤:cookie 为 `HttpOnly` 和 `SameSite=Lax`,
  CSRF 标头由 Web 客户端添加,机密在界面中只写不读。
- **纵深防御。** 在访问控制之上有静态加密;在 `SameSite` 之上有 CSRF 标头;在 Argon2 哈希之上有
  登录速率限制;在自身地址防护之上有重定向策略。

### 密码学

- 密码:Argon2(`auth/password.rs`)。会话和 API 令牌的机密以随机值的 SHA-256 哈希存储
  (`auth/session.rs`、`auth/token.rs`)。
- 静态机密:AES-256-GCM,每次加密使用新的 nonce(`crypto.rs`)。密钥通过 Argon2id 从
  `DUMBMONIT_SECRET` 或生成的密钥文件派生,并使用持久的盐;短于 16 个字符的机密会被拒绝,
  并且启动时会用一个金丝雀值检测密钥是否错误。
- 备份:密钥通过 Argon2id 从运维人员的口令派生,参数存储在备份包中,因此算法可以更改而不会破坏旧的
  备份包(`backup/bundle.rs`)。
- 第二因素:TOTP,配合经过哈希的一次性恢复码(`auth/totp.rs`)。
- OpenID Connect:state、nonce 和 PKCE `S256`(`auth/oidc/pkce.rs`)。
- 所有原语都来自维护中的库(`aes-gcm`、`argon2`、`sha2`);没有任何自行实现的算法。内置监听器使用
  明文 HTTP;文档要求使用 TLS 反向代理,并且当请求通过 HTTPS 到达时,cookie 会加上 `Secure`
  (`auth/cookie.rs`)。

## 常见弱点的对策

| 弱点(OWASP Top 10、CWE) | 对策 | 证据 |
|---|---|---|
| 访问控制失效,CWE-862、CWE-285 | 身份验证中间件、角色、令牌范围 | `operator_role_api.rs`、`api_tokens_api.rs`、`api_open_security_api.rs` |
| 加密失败,CWE-327、CWE-312 | 静态 AES-256-GCM、Argon2、API 不返回任何机密 | `crypto.rs` 单元测试;用 `has_secret` / `credential_kind` 代替机密 |
| SQL 注入,CWE-89 | 通过 `sqlx` 使用绑定参数 | `crates/server/src/db/` |
| 跨站脚本,CWE-79 | Svelte 会转义输出;CSP 使用 `default-src 'none'`,脚本仅来自 `'self'` 和 nonce;`base-uri 'none'` | `api/mod.rs` |
| 跨站请求伪造,CWE-352 | 通过 cookie 的写操作需要 `X-Requested-With: DumbMonit` 或同源 fetch 元数据;CORS 不向任何人授予该标头;`SameSite=Lax` | `auth/middleware.rs`、`auth/cors.rs`;测试 `cors_policy_api.rs` |
| 点击劫持,CWE-1021 | `X-Frame-Options: DENY` 和 `frame-ancestors 'none'`,但选择启用、旨在被嵌入的公开状态页面除外 | `api/mod.rs` |
| 服务器端请求伪造,CWE-918 | 仅同一主机的重定向;通知防护会拒绝服务器自身的监听地址和 VictoriaMetrics 地址,包括 DNS 解析之后的情况(重绑定)。回环地址和私有网段特意保持可访问,因为家庭实验室的服务就在那里 | `notify/guard.rs`;测试 `notify_guard_api.rs` |
| 身份验证失败,CWE-307、CWE-287 | 按地址和按账户、延迟递增的登录速率限制、可选的 TOTP、带 PKCE 的 OIDC、管理员重置第二因素 | `auth/rate_limit.rs`、`auth/totp.rs`、`auth/oidc/` |
| 不安全的反序列化、资源耗尽,CWE-400 | 通过 `serde` 使用类型化 JSON、请求体上限为 16 MiB、重定向次数有限 | `api/mod.rs`、`collectors/src/http.rs` |
| 内存安全,CWE-787、CWE-416 | Rust;`unsafe` 仅出现在 Windows 代理的性能计数器代码中(`crates/agent/src/collect/perf_counters.rs`) | 用 grep 搜索 `unsafe` |
| 易受攻击的组件,CWE-1104 | `cargo deny`(许可证、禁用项、公告);Dependabot;对已发布镜像的 Trivy 扫描;按提交固定的 actions;`--locked` 构建 | `.github/workflows/ci.yml`、`security.yml`、`.github/dependabot.yml`、`deny.toml` |
| 软件和数据完整性,CWE-494 | 代理安装程序会校验所下载二进制文件的 SHA-256 | `crates/agent/install/install.sh`、`install.ps1` |
| 日志记录和监控失败,CWE-778 | 登录和敏感变更的安全日志 | `auth/audit.rs`;测试 `security_audit_api.rs` |

已知的剩余风险列在安全模型中(明文 HTTP 监听器、未加密的设备选项、不带身份验证的内置
VictoriaMetrics)。CSP 允许内联样式(`style-src 'unsafe-inline'`),这会削弱其对样式注入的防护,
但不影响对脚本执行的防护。存在一条已接受的公告 `RUSTSEC-2023-0071`,在 `deny.toml` 中连同理由一起被
忽略:`rsa` crate 使用提供商的公钥验证 RS256 签名,从不使用私钥。

## 证据

- **持续集成**(`.github/workflows/ci.yml`):每次推送和拉取请求都会运行 `cargo fmt --check`、
  `clippy -D warnings`、完整的 Rust 测试套件、`cargo deny check licenses bans` 和
  `cargo deny check advisories`、Web 类型检查、Web 测试和构建,以及 Docker 镜像构建。
- **定期扫描**(`.github/workflows/security.yml`):对已发布的服务器和代理镜像进行 Trivy 扫描,
  以及 OpenSSF Scorecard,每周一次,并在每次 `:edge` 发布之后运行。结果显示在仓库的 Security
  选项卡中,GitHub 代码扫描(CodeQL)的报告也在那里。
- **测试**:`crates/server/tests/` 下的 API、身份验证、角色、CORS、OIDC、防护和审计测试,代码旁的
  单元测试,以及 `web/tests/` 下的 Web 测试。使用 `cargo-llvm-cov` 测得的 Rust 行覆盖率为 87.9 %。
- **回归策略**:每个修复都附带一个没有该修复就会失败的测试(参见 `CONTRIBUTING.md` 中的测试策略)。
- **公开记录**:安全修复以公告形式发布,并列在发行说明中。
