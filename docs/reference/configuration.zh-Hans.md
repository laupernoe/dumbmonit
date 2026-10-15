# Configuration {#configuration}

一切都通过环境变量进行；没有一个是必需的。没有配置文件要挂载。

## Server {#server}

| Variable | Default | Role |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | API 和 web UI 的监听地址，在容器内。 |
| `PUID` / `PGID` | `65532` | 容器以 root 启动（`user: "0:0"`，如随附的 `docker-compose.yml`）时服务器运行所用的用户和组：启动时数据目录会被移交给它们，随后放弃 root。`0` 会被拒绝。`DUMBMONIT_PUID` / `DUMBMONIT_PGID` 优先。其他情况下被忽略。参见[权限](../install/docker.md#permissions)。 |
| `DUMBMONIT_DATA_DIR` | `/data` | 持久目录：`dumbmonit.db`（SQLite）、`secret.key`、`vm/`（嵌入式 VictoriaMetrics）和 `webchange/`（网站更改的屏幕截图）。 |
| `DUMBMONIT_VM_URL` | *(unset)* | 外部 VictoriaMetrics 的基 URL（`http://host:8428`）。当设置时，嵌入式未启动；见[下方](#embedded-victoriametrics)。 |
| `DUMBMONIT_SECRET` | *(generated)* | 实例秘密。如果空，从 `<data dir>/secret.key` 读，在第一次启动时生成。用 AES-256-GCM 加密监控项凭证、通道秘密和代理令牌。 |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | 最大同步探针，所有收集器结合。 |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | 一个探针的硬超时。服务监测有其自己的、更短的 `timeout_seconds` 选项。 |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | 批处理写的周期朝向 VictoriaMetrics。 |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | 缓冲示例数触发在周期之前的写。 |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | 异步运行时的线程。服务器花时间等待网络：四个覆盖家庭实验室，仅为数百监控项提高。 |
| `DUMBMONIT_DB_POOL` | `4` | 最大开 SQLite 连接。每个成本一个线程和一个 2 MB 页缓存；空闲连接五分钟后关闭。 |
| `DUMBMONIT_LOG` | `info` | 日志过滤器，`tracing` 语法：`debug`、`warn`、`dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | 代理二进制目录在 `/download/` 下提供。空或缺失：安装命令失败 `404`，服务器仍运行。 |
| `DUMBMONIT_RESET_PASSWORD` | *(off)* | `1`、`true`、`yes` 或 `on`：清除密码和每个会话在启动。UI 然后再次显示 `/setup`。之后移除它。 |
| `DUMBMONIT_COOKIE_SECURE` | *(off)* | `1` 总是在会话 cookie 上设置 `Secure` 属性。仅在 HTTPS 后面：在纯 HTTP 上浏览器永远不会将 cookie 发回。不有它，属性仍被添加当浏览器在 `https://` 页面上时（其 `Origin` 说，或受信任代理发送 `X-Forwarded-Proto: https`），所以纯 HTTP 在 LAN 保持工作。 |
| `DUMBMONIT_TRUSTED_PROXIES` | *(empty)* | 反向代理其 `X-Forwarded-For`、`X-Forwarded-Proto` 和 `X-Forwarded-Host` 被信任，如地址或 CIDR 范围用逗号分离（`10.0.0.5, 172.16.0.0/12`）。登入速率限制器和安全日志然后看到真实客户端地址而不是代理的；没有它，代理后的每个访问者共享一个桶。状态页面链接（RSS 订阅、订阅电子邮件）和单一签入返回地址也仅使用来自这些代理的转发主机。来自任何其他地址的标头被忽略，所以客户端不能选择其自己的桶或地址的链接。 |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(off)* | `true` 为首次单一签入的未知身份创建账户。关时，仅现有账户（已链接，或由供应商验证的电子邮件匹配）可以签入。像其他 `DUMBMONIT_OIDC_*` 变量一样，仅在接口中未保存单一签入设置时读。 |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(off)* | `1` 接受纯 `http://` 中的 OpenID Connect 颁布者。关时，设置屏幕拒绝除 `https://` 外的任何东西：清文本颁布者将授权码和客户端秘密交给任何监听网络的人。仅为 loopback 上的测试供应商。 |
| `DUMBMONIT_SETUP_CODE` | *(generated)* | `/setup` 在没有管理员时要求的代码。未设置，随机代码在每个启动时打印在服务器日志中；为自动化部署设置它。请参阅 [Install with Docker](../install/docker.md)。 |
| `DUMBMONIT_PUBLIC_URL` | *(unset)* | 浏览器到达 DumbMonit 的地址（`https://monit.example.org`）。用于通知中的链接和状态页面电子邮件、单一签入重定向 URI 和给予推送服务的联系（仅 HTTPS）。在接口中保存的值（通知策略、SSO 设置）获胜。 |
| `DUMBMONIT_API_CORS_ORIGINS` | *(any origin)* | 允许用 bearer 令牌跨源调用 API 和到达 MCP 端点的来源：逗号分离列表，或 `off` 对于都不。请参阅 [HTTP API](api.md#cross-origin-requests-cors)。 |
| `DUMBMONIT_OIDC_ISSUER` | *(unset)* | OpenID Connect 供应商的颁布者 URL；发现从 `{issuer}/.well-known/openid-configuration` 读。必须 `https://` 除非 `DUMBMONIT_OIDC_ALLOW_HTTP` 被设置。 |
| `DUMBMONIT_OIDC_CLIENT_ID` | *(unset)* | 与供应商注册的客户端 id。 |
| `DUMBMONIT_OIDC_CLIENT_SECRET` | *(unset)* | 客户端秘密。 |
| `DUMBMONIT_OIDC_PROVIDER_NAME` | `SSO` | "Continue with …"按钮的标签。 |
| `DUMBMONIT_OIDC_SCOPES` | `openid profile email` | 要求的范围，用空格分离。 |
| `DUMBMONIT_OIDC_GROUPS_CLAIM` | `groups` | 列出用户组的令牌声明。 |
| `DUMBMONIT_OIDC_ADMIN_GROUPS` | *(empty)* | 逗号分离的组其成员变成管理员。 |
| `DUMBMONIT_OIDC_OPERATOR_GROUPS` | *(empty)* | 逗号分离的组其他成员变成操作员。请参阅 [Single sign-on](../using/settings.md#single-sign-on)。 |
| `DUMBMONIT_METRICS_PUBLIC` | *(off)* | `1` 提供 `/metrics`、`/federate` 和 `/prometheus/api/v1/…` 不用 API 令牌。任何可以到达端口的人然后读每个测量的每个监控项和实例的状态；仅在其他东西已限制对端口的访问时设置。请参阅 [Metrics](metrics.md#scraping-dumbmonit)。 |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | 告警评估周期。低于 10 的值提高到 10。 |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | 告警历史的保留，以天计。 |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | 数据库的计划本地备份。`0`、`false`、`no` 或 `off` 停止写入它们。请参阅 [Backup and restore](../install/backup.md#scheduled-local-backups)。 |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` 停止对最新 GitHub release 的每日查看（air-gapped 实例）。请参阅 [Update](../install/update.md)。 |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | 计划备份写入的位置。指向第二卷以在失去第一个时存活。 |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | 两个备份之间的小时，1 到 8760。第一个发生一个间隔在启动之后，不是启动。 |
| `DUMBMONIT_DEMO` | *(off)* | `1` 对于公开、只读演示有一个固定虚构地产，签入 `demo` / `demo`。**在每个启动时重新创建数据库**：从不在真实数据卷上。请参阅 [Run a demo instance](../install/demo.md)。 |
| `DUMBMONIT_BACKUP_KEEP` | `7` | 多少备份被保留，1 到 365。最旧的被移除其 `.key`。 |
| `DUMBMONIT_BROWSER_URL` | *(unset)* | [website changes](../devices/webchange.md#screenshots) 的屏幕截图用的 Headless Chromium，如 `http://host:9222`（其 DevTools 端口）。在你的 Compose 文件中添加一个 `chromedp/headless-shell` 服务，如 [Screenshots](../devices/webchange.md#screenshots) 所示。未设置：仅文本比较。 |

基线保留（60 天）和 14 天学习周期不可配置。

### Former `EZYMONIT_*` names {#former-ezymonit_-names}

DumbMonit 被称为 EzyMonit 直到 2026 年 9 月。每个 `DUMBMONIT_*` 变量也在其旧的 `EZYMONIT_*` 名字下读，作为回退，在启动时有一个警告per 变量仍以那种方式设置（`EZYMONIT_X is deprecated, use DUMBMONIT_X`）。旧数据库文件 `/data/ezymonit.db` 在启动时被重命名为 `dumbmonit.db`，和代理令牌 `ezym_…` 保持有效（新的是 `dmon_…`）。会话 cookie 现在是 `dumbmonit_session`：重命名后的第一次启动登出每个人一次。

### 嵌入式 VictoriaMetrics {#embedded-victoriametrics}

图像装运 VictoriaMetrics 二进制。除非 `DUMBMONIT_VM_URL` 被设置，服务器在启动时作为子进程启动它，将其日志行转发进服务器日志，如果它死亡带 backoff 重启，并在关闭时停止它
（`SIGTERM`、20 秒宽限、然后杀；Compose 文件的 `stop_grace_period` 为此留出空间）。

| Variable | Default | Role |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | 要启动的 VictoriaMetrics 二进制的路径。 |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | 监听地址，在容器内。默认 Loopback：仅服务器与之谈话。开发覆盖设置 `0.0.0.0:8428` 和发布端口。 |
| `DUMBMONIT_VM_RETENTION` | `12` | 保留，在 `-retentionPeriod` 语法中：月数，或如 `30d` 或 `2y` 的持续时间。 |
| `DUMBMONIT_VM_MEMORY` | `256MB` | VictoriaMetrics 缓存预算（`-memory.allowedBytes`）。不用预算 VM 为主机 RAM 的 60% 和其驻留内存增长的日子大小它们。几十个监控项适合在 256 MB 中；为数百主机提高。 |

其数据住在 `<data dir>/vm`，在数据库的相同卷上。三个标志由服务器固定，不可配置：
`-search.maxUniqueTimeseries=100000`（失控查询不能耗尽预算）、`-search.maxConcurrentRequests=4`（一个 UI 加告警引擎）和
`-dedup.minScrapeInterval=10s`（十秒更接近的重复点被折叠；在收集器的 30 秒步速没有损失）。

与 `DUMBMONIT_VM_URL` 设置，无一适用：服务器使用您命名的实例，嵌入式从不启动。`GET /api/health` 说哪个情况您在（`victoria.embedded`）。

## Compose-level settings {#compose-level-variables}

随附的 `docker-compose.yml` 不读取任何变量，也不需要 `.env` 文件。
一切都在服务的 `environment:` 下设置，或直接编辑该文件；参见
[高级选项](../install/docker.md#advanced-options)（端口、密钥、保留期与内存、
外部 VictoriaMetrics、密码重置、截图）。开发覆盖文件 `docker-compose.dev.yml`
读取 `DUMBMONIT_VM_PORT`（默认 `8428`），作为内嵌 VictoriaMetrics 的主机端口。

## Ports {#ports}

| Port | Container | Purpose |
