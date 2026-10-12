# GitLab(自托管) {#gitlab-self-managed}

一个自托管的 GitLab 实例是否在正常提供服务,以及那些默默支撑着
其他一切的部分是否跟得上节奏:Sidekiq 的队列、CI/CD runner、
升级后遗留的数据库迁移,以及——对于 GitLab 企业版——许可证。

DumbMonit 使用**个人访问令牌**读取 `/api/v4/version`,这是唯一
必须成功的调用:它证明 GitLab 能够应答,且令牌有效。其余所有
内容——Sidekiq、runner、迁移、实例统计、注册和两步验证设置、
许可证,以及详细的就绪检查——都从 GitLab 仅限管理员账户访问的
接口读取,当令牌属于普通账户时会被跳过,而不会导致探测失败。

这里值得关注的故障是:**Sidekiq 积压**。GitLab 会继续提供页面
服务,而其后台工作队列不断增长,页面上没有任何提示,直到某个
通知、webhook 或 CI 流水线更新延迟了几分钟甚至几小时才会被发现。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_gitlab_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version`, `revision` |
| `readiness_check` | 当某个就绪组件(数据库、缓存、队列、共享状态、Gitaly)不是 `ok` 时为 1 | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | 每个 Sidekiq 队列中等待的任务数及其年龄 | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | 实例已知的 CI/CD runner 数 | |
| `runner_offline` | 每个离线 runner 对应的值为 1(最多 32 个) | `runner` |
| `migrations_pending` | 待应用的数据库迁移数 | |
| `migration_pending` | 每个待处理迁移对应的值为 1(最多 32 个) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | 实例范围的计数 | |
| `signup_enabled` | 当公开注册开放时为 1 | |
| `two_factor_required` | 当每个用户都必须设置两步验证时为 1 | |
| `license_info` | 固定值 1(仅企业版) | `plan` |
| `license_expired`, `license_expiry_seconds` | 许可证是否已过期,以及剩余时间 | |
| `pipelines_failed_recent`, `pipelines_pending` | 回溯窗口内失败的流水线,以及当前待处理或运行中的流水线——仅针对"CI 健康监控的项目"选项中列出的项目 | `project` |
| `scrape_errors` | 上次探测中失败的仅限管理员的调用(普通令牌从未尝试这些调用时为 0) | |

适用的[内置规则](../alerting/rules.md#self-hosted-forges):
GitLab runner 全部离线、GitLab Sidekiq 积压过高、GitLab 迁移待
处理、GitLab 许可证即将过期,以及设备不可达。安全评分会检查
两步验证和注册设置,以及待处理的迁移:参见
[供应商更新及其他检查](../using/security-score.md)。

## 不会监控的内容 {#what-it-does-not-watch}

GitLab 自己的 REST API 没有任何接口会说"有更新的 GitLab 版本
可用":与 Nextcloud 或 Paperless-ngx 不同,没有自我报告的更新
标记可供读取,因此 DumbMonit 不会声称有这项功能。请手动将上面的
版本号与 GitLab 的[发布页面](https://about.gitlab.com/releases/)
对比,或留意 GitLab 管理区域中它自己的更新检查提示。

CI 流水线健康状况只会针对你列出的项目读取:GitLab 的 API 没有
实例范围的"所有失败流水线"接口,只有按项目的接口。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会调用 GitLab。
它会先说明出了什么问题(某个就绪组件失败、runner 全部离线、
Sidekiq 积压严重),然后是版本号,再然后是实例计数和从管理 API
读取的设置。

## 在管理员账户上创建个人访问令牌 {#create-a-personal-access-token-on-an-administrator-account}

1. 在管理员账户上创建一个个人访问令牌:头像 → Edit profile → Access tokens → Add new token。按如下方式命名,设置一个你愿意定期续期的过期日期,只勾选 read_api 范围:它能读取整个 API,不能写入任何内容。

    ```
    dumbmonit
    ```

2. Sidekiq 队列、CI runner、待处理的迁移、实例统计、注册和两步验证设置以及许可证,每一项都是从仅限管理员的接口读取的:GitLab 对它们没有更精细的权限委托。普通账户的令牌仍然可用;此时只会读取版本号,其余内容会被跳过而不会导致探测失败。

3. 在 DumbMonit 中,输入你用来打开 GitLab 的地址,例如 "gitlab.example.com"(443 端口的 HTTPS),或反向代理之后的 "https://git.example.com",并粘贴该令牌。

4. 若要同时监控 CI 流水线,请在"CI 健康监控的项目"选项中列出数字形式的项目 ID(显示在每个项目概览页面、其名称下方),用逗号分隔。

!!! warning "警告"
    管理员账户上的个人访问令牌,即使只勾选了 read_api 范围,仍能读取该账户能读取的一切:GitLab 没有只读管理员角色。请为此专门设立一个机器人账户,而不是重复使用你自己的账户,并在令牌泄露时及时撤销。

从 DumbMonit 所在主机手动检查该令牌:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 个人访问令牌 | 以 `PRIVATE-TOKEN` 请求头发送。 |

地址:主机名(`gitlab.example.com`)或 `host:port`。选项:协议
(默认 HTTPS)、端口(443)、证书校验、请求超时(15 秒)、CI 健康
监控的项目(默认无)和失败流水线的回溯窗口(24 小时)。

已在 GitLab 企业版 17.5(官方 API 文档)上验证。
