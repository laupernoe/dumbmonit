# Forgejo / Gitea {#forgejo--gitea}

Forgejo 和 Gitea 共享相同的 REST API，因此 DumbMonit 使用一个收集器读取两者：实例是否响应、其健康检查、它持有多少个存储库、账户和组织、计划任务（存储库检查、清理、更新检查器）是否仍在自己的时间间隔运行，以及 Actions 运行器是否在线。

两个调用必须成功：`/api/v1/version`（公开的）和 `/api/v1/user`（证明 **访问令牌** 有效）。其他一切 — 健康检查、存储库/账户/组织计数、计划任务和 Actions 运行器 — 是从 Forgejo 和 Gitea 为站点管理员账户保留的端点读取的，当令牌属于普通账户时跳过而不会失败探测。

这里值得关注的故障：**计划任务停止运行**。Forgejo 和 Gitea 在其内部调度程序卡住时继续提供页面和 git 操作；没人注意到，直到更新检查器或存储库一致性检查已经好几周没有运行。

## 监视什么 {#what-it-watches}

所有指标都带有前缀 `dumbmonit_forgejo_`。

| 指标 | 说明 | 标签 |
|---|---|---|
| `version_info` | 值为 1 | `version` |
| `token_is_admin` | 当令牌的账户是站点管理员时为 1 | |
| `healthz_ok` | 除非健康检查报告 `fail` 否则为 1 | |
| `healthz_check` | 每个故障组件（数据库、缓存）为 1 | `check` |
| `repos` | 对令牌账户可见的存储库 | |
| `users`, `orgs` | 实例上的账户和组织 | |
| `cron_tasks` | 在实例上定义的计划任务 | |
| `cron_task_overdue` | 每个下次运行已过期的任务为 1，否则为 0；禁用的任务（计划 `-`）被跳过 | `task` |
| `runners_total`, `runners_online`, `runners_offline` | 实例已知的 Actions 运行器；`online`、`idle` 和 `active` 计为在线 | |
| `runner_offline` | 每个离线运行器的值为 1（最多 32 个） | `runner` |
| `scrape_errors` | 最后一次探测中失败的仅管理员调用（使用普通令牌时为 0，因为从未尝试） | |

适用的[内置规则](../alerting/rules.md#self-hosted-forges)：Forgejo/Gitea 运行器全部离线、Forgejo/Gitea 计划任务逾期，加上监控项不可达。安全评分检查计划任务：参见[供应商更新和其他检查](../using/security-score.md)。

## 它不监视什么 {#what-it-does-not-watch}

Forgejo 和 Gitea 没有只读管理员角色：每个保留给"管理员"的调用都需要一个拥有完全权限的账户，而不是作用域受限的委托。它们的 API 也没有相当于 GitLab 注册或双因素设置的东西，也没有自报"可用更新"标志 — 手动对比上面的版本与项目的[发布说明](https://forgejo.org/releases/)。

## 监控项页面 {#the-device-page}

面板上面的图表读取探测存储的内容；打开页面永远不会调用 Forgejo 或 Gitea。它首先说出什么是错的（健康检查失败、运行器全部离线、计划任务逾期），然后是版本，然后是存储库、账户和组织计数。

## 在站点管理员账户上创建访问令牌 {#create-an-access-token-on-a-site-administrator-account}

1. 在站点管理员账户上创建访问令牌：头像 → 设置 → 应用程序 → 生成新令牌。按如下方式命名并勾选 read:admin、read:repository 和 read:user 作用域。

    ```
    dumbmonit
    ```

2. read:admin 打开管理计数、计划任务和 Actions 运行器；没有它，只读取版本和令牌自身的账户，其余的跳过而不会失败探测。Forgejo 和 Gitea 也没有只读管理员角色。

3. 在 DumbMonit 中，输入地址，例如"forgejo.lan"（端口 3000）或"https://git.example.com"（在反向代理后面），并粘贴令牌。

!!! warning
    站点管理员账户上的访问令牌可以读取和更改该账户可以进行的任何操作，无论勾选了哪些作用域：Forgejo 和 Gitea 仅限制一个作用域打开哪些 API 路由，而不是管理员账户本身可以做什么。将其专用于一个机器人账户，而不是重复使用您自己的。

从 DumbMonit 主机手动检查令牌：

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| 访问令牌 | 以 `Authorization: token …` 形式发送。 |

地址：主机名（`forgejo.lan`）或 `host:port`。选项：协议（默认 HTTP — 在 LAN 之外的任何地方放一个带 TLS 的反向代理）、端口（3000）、证书检查和请求超时（15 秒）。

针对 Forgejo 9.0 验证（官方 API 源和 Swagger 定义，与 Gitea 共享）。
