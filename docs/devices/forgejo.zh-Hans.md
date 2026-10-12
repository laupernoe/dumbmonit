# Forgejo / Gitea

Forgejo 和 Gitea 共享同一个 REST API,因此 DumbMonit 用同一个采集器
读取两者:实例是否应答、它的健康检查、拥有多少仓库、账户和组织、
其计划任务(仓库检查、清理、更新检查器)是否仍按自己的周期运行,
以及其 Actions runner 是否在线。

必须成功的两次调用:公开的 `/api/v1/version`,以及证明**访问
令牌**有效的 `/api/v1/user`。其余所有内容——健康检查、仓库/账户/
组织计数、计划任务和 Actions runner——都从 Forgejo 和 Gitea 仅限
站点管理员账户访问的接口读取,当令牌属于普通账户时会被跳过,
而不会导致探测失败。

这里值得关注的故障是:**某个计划任务停止运行**。Forgejo 和 Gitea
在其内部调度器卡死时仍会继续提供页面和 git 操作服务;直到更新
检查器或仓库一致性检查已经数周未运行,才会有人注意到。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_forgejo_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `token_is_admin` | 当令牌所属账户是站点管理员时为 1 | |
| `healthz_ok` | 除非健康检查报告 `fail`,否则为 1 | |
| `healthz_check` | 每个失败组件对应的值为 1(数据库、缓存) | `check` |
| `repos` | 令牌所属账户可见的仓库数 | |
| `users`, `orgs` | 实例上的账户和组织数 | |
| `cron_tasks` | 实例上定义的计划任务数 | |
| `cron_task_overdue` | 每个已到期但未运行的任务对应的值为 1,否则为 0 | `task` |
| `runners_total`, `runners_online`, `runners_offline` | 实例已知的 Actions runner 数 | |
| `runner_offline` | 每个离线 runner 对应的值为 1(最多 32 个) | `runner` |
| `scrape_errors` | 上次探测中失败的仅限管理员的调用(普通令牌从未尝试这些调用时为 0) | |

适用的[内置规则](../alerting/rules.md#self-hosted-forges):
Forgejo/Gitea runner 全部离线、Forgejo/Gitea 计划任务逾期,以及
设备不可达。安全评分会检查计划任务:参见
[供应商更新及其他检查](../using/security-score.md)。

## 不会监控的内容 {#what-it-does-not-watch}

Forgejo 和 Gitea 没有只读管理员角色:每个仅限"admin"的调用都需要
一个拥有完整权限的账户,而不是一个权限受限的代理账户。它们的 API
也没有类似 GitLab 的注册或两步验证设置,也没有自我报告的
"有可用更新"标记——需要手动将上面的版本号与该项目的
[发布说明](https://forgejo.org/releases/)进行比对。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会调用 Forgejo
或 Gitea。它会先说明出了什么问题(健康检查失败、runner 全部离线、
某个计划任务逾期),然后是版本号,再然后是仓库、账户和组织的计数。

## 在站点管理员账户上创建访问令牌 {#create-an-access-token-on-a-site-administrator-account}

1. 在站点管理员账户上创建一个访问令牌:头像 → Settings → Applications → Generate new token。按如下方式命名,并勾选 read:admin、read:repository 和 read:user 这几个范围。

    ```
    dumbmonit
    ```

2. read:admin 可以打开管理计数、计划任务和 Actions runner;没有它,只会读取版本号和令牌自身账户的信息,其余内容会被跳过而不会导致探测失败。Forgejo 和 Gitea 也同样没有只读管理员角色。

3. 在 DumbMonit 中,输入地址,例如 "forgejo.lan"(端口 3000),或反向代理之后的 "https://git.example.com",并粘贴该令牌。

!!! warning "警告"
    站点管理员账户上的访问令牌,无论勾选了哪些范围,都能读取和更改该账户所能触及的一切:Forgejo 和 Gitea 的范围只决定哪些 API 路由会被打开,而不决定管理员账户本身能做多少事。请为此专门设立一个机器人账户,而不是重复使用你自己的账户。

从 DumbMonit 所在主机手动检查该令牌:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 访问令牌 | 以 `Authorization: token …` 发送。 |

地址:主机名(`forgejo.lan`)或 `host:port`。选项:协议(默认
HTTP——除局域网外的任何场景都应在前面加一个带 TLS 的反向代理)、
端口(3000)、证书校验和请求超时(15 秒)。

已在 Forgejo 9.0(官方 API 源码和 Swagger 定义,与 Gitea 共享)上验证。
