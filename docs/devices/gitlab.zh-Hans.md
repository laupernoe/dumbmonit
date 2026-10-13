# GitLab（自托管） {#gitlab-self-managed}

一个自托管的 GitLab 实例是否在提供服务，以及静静地承载一切的部分是否跟得上：Sidekiq 的队列、CI/CD 运行器、从升级遗留的数据库迁移，以及 — 对于 GitLab 企业版 — 许可证。

DumbMonit 使用 **个人访问令牌** 读取 `/api/v4/version`，这是唯一必须成功的调用：它证明 GitLab 响应且令牌有效。其他一切 — Sidekiq、运行器、迁移、实例统计、注册和双因素设置、许可证和详细的就绪检查 — 是从 GitLab 为管理员账户保留的端点读取的，当令牌属于普通账户时跳过而不会失败探测。

这里值得关注的故障：**Sidekiq 落后**。GitLab 在其后台工作队列增长时继续提供页面，页面上什么都不会说，直到通知、webhook 或 CI 管道更新晚了几分钟或几小时。

## 监视什么 {#what-it-watches}

所有指标都带有前缀 `dumbmonit_gitlab_`。

| 指标 | 说明 | 标签 |
|---|---|---|
| `version_info` | 值为 1 | `version`, `revision` |
| `readiness_check` | 当一个就绪组件（数据库、缓存、队列、共享状态、Gitaly）不是 `ok` 时为 1；当 GitLab 不向 DumbMonit 公开它时不存在（见下文） | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | 每个 Sidekiq 队列等待的任务及其年龄 | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | 实例已知的 CI/CD 运行器 | |
| `runner_offline` | 每个离线运行器的值为 1（最多 32 个） | `runner` |
| `migrations_pending` | 要应用的数据库迁移 | |
| `migration_pending` | 每个待处理迁移的值为 1（最多 32 个） | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | 实例范围的计数 | |
| `signup_enabled` | 公开注册打开时为 1 | |
| `two_factor_required` | 每个用户必须设置双因素认证时为 1 | |
| `license_info` | 值为 1（仅企业版） | `plan` |
| `license_expired`, `license_expiry_seconds` | 许可证是否已过期，以及剩余时间 | |
| `pipelines_failed_recent`, `pipelines_pending` | 回看窗口中的失败管道，以及现在待处理或运行的管道 — 仅对"监视 CI 健康的项目"选项中列出的项目 | `project` |
| `scrape_errors` | 最后一次探测中失败的仅管理员调用（使用普通令牌时为 0，因为从未尝试） | |

适用的[内置规则](../alerting/rules.md#self-hosted-forges)：GitLab 运行器全部离线、GitLab Sidekiq 积压高、GitLab 迁移待处理、GitLab 许可证即将过期，加上设备不可达。安全评分检查双因素和注册设置，以及待处理迁移：参见[供应商更新和其他检查](../using/security-score.md)。

## 它不监视什么 {#what-it-does-not-watch}

GitLab 自身的 REST API 没有端点说"有更新的 GitLab 版本可用"：不同于 Nextcloud 或 Paperless-ngx，没有自报"可用更新"标志来读取，所以 DumbMonit 不声称有。手动对比上面的版本与 GitLab 的[发布页面](https://about.gitlab.com/releases/)，或查看 GitLab 自身管理区域的更新检查通知。

CI 管道健康仅为您列出的项目读取：GitLab 的 API 没有实例范围的"所有失败管道"端点，只有按项目的。

## 设备页面 {#the-device-page}

面板上面的图表读取探测存储的内容；打开页面永远不会调用 GitLab。它首先说出什么是错的（就绪组件故障、运行器全部离线、较大的 Sidekiq 积压），然后是版本，然后是实例计数和从管理 API 读取的设置。

## 在管理员账户上创建个人访问令牌 {#create-a-personal-access-token-on-an-administrator-account}

1. 在管理员账户上创建个人访问令牌：头像 → 编辑个人资料 → 访问令牌 → 添加新令牌。按如下方式命名，设置您舒适更新的过期日期，并仅勾选 read_api 作用域：它读取整个 API 且不写入任何内容。

    ```
    dumbmonit
    ```

2. Sidekiq 队列、CI 运行器、待处理迁移、实例统计、注册和双因素设置以及许可证每个都是从仅管理员端点读取的：GitLab 对它们没有更细粒度的委托。来自普通账户的令牌仍然有效；只读取版本，其余的跳过而不会失败探测。

3. 在 DumbMonit 中，输入您打开 GitLab 的地址，例如"gitlab.example.com"（HTTPS 在 443）或"https://git.example.com"（在反向代理后面），并粘贴令牌。

4. 要监视 CI 管道，在"监视 CI 健康的项目"选项中列出数字项目 ID（显示在每个项目的概览页面上，其名称下方），以逗号分隔。

!!! warning
    管理员账户上的个人访问令牌可以读取该账户可以读取的一切，即使使用 read_api 作用域：GitLab 没有只读管理员角色。将其专用于一个机器人账户，而不是重复使用您自己的，如果它曾泄露请撤销该令牌。

### 就绪检查 {#readiness-check}

GitLab 以 404 回复 `/-/readiness` 给任何不在其监视允许列表中的客户端地址（默认仅 127.0.0.1；在 Docker 或反向代理后面 DumbMonit 来自另一个地址）。DumbMonit 将该 404 视为"未公开"：就绪面板保持空白，它不计入 `scrape_errors`。要启用它，在 `gitlab.rb` 中添加 DumbMonit 的地址并运行 `gitlab-ctl reconfigure`：

```
gitlab_rails['monitoring_whitelist'] = ['127.0.0.0/8', '::1/128', '192.168.1.10/32']
```

许可证端点同样在社区版本上以 404 回复，这也不是错误。

从 DumbMonit 主机手动检查令牌：

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| 个人访问令牌 | 以 `PRIVATE-TOKEN` 标头形式发送。 |

地址：主机名（`gitlab.example.com`）或 `host:port`。选项：协议（默认 HTTPS）、端口（443）、证书检查、请求超时（15 秒）、监视 CI 健康的项目（默认无）和失败管道回看窗口（24 小时）。

针对 GitLab 企业版 17.5 验证（官方 API 文档）。
