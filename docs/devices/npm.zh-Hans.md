# Nginx Proxy Manager {#nginx-proxy-manager}

代理管理器：nginx 拒绝配置的主机、禁用的主机，以及主机呈现的证书，在 Let's Encrypt 更新失败变成过期网站之前。

DumbMonit 使用自己的用户登录 Nginx Proxy Manager 的 REST API（`/api`，在其网页界面的端口 81 上），然后读取代理主机、重定向主机、404 主机（API 中称为死主机）和流。每个主机说明是否启用以及 nginx 是否接受其配置。

nginx 拒绝的主机不被提供，网页界面仅将其显示为红点。

证书不从 API 读取。`/api/nginx/certificates` 将导入证书的私钥和 DNS 挑战的凭证交给任何被允许查看证书的用户。DumbMonit 改为从其被提供的地方读取每个证书：对 Nginx Proxy Manager 端口 443 进行 TLS 握手，使用使用它的主机的名称。这也测量了访客实际获得的证书。

## 它监控什么 {#what-it-watches}

所有指标前缀为 `dumbmonit_npm_`。

| 指标 | 说明 | 标签 |
|---|---|---|
| `version_info` | 值为 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | 每种类型的主机 | `type`: `proxy`, `redirection`, `dead`（404 主机），`stream` |
| `hosts_offline` | 启用但 nginx 拒绝配置的主机 | `type` |
| `host_online` | 每个启用的主机：当 nginx 拒绝其配置时为 0 | `type`, `proxy_host` |
| `host_error_info` | 值为 1，对于 nginx 拒绝的主机：nginx 错误的第一行 | `type`, `proxy_host`, `error` |
| `certificates_checked`, `certificates_read` | 附加到启用主机的证书，以及在 HTTPS 端口上读取的证书 | |
| `cert_expiry_days` | 主机呈现的证书过期前的天数 | `proxy_host` |

`proxy_host` 是主机的第一个域名，或流的 `port 9000`。禁用的主机被计数，不被描述：禁用主机是有意的，其警报（如有）被解决。主机逐个描述到 500，最多读取 100 个证书。

适用的[内置规则](../alerting/rules.md#reverse-proxies-and-domains)：Nginx Proxy Manager 主机离线、代理证书未更新、加上监控项不可达。

## 监控项页面 {#the-device-page}

面板上方读取探针存储的内容；打开页面从不查询 Nginx Proxy Manager。它用一句话说明问题所在（nginx 拒绝的主机及其原因、即将到期的证书、无法读取的证书），然后是代理主机、重定向、404 主机、流和禁用主机，然后是最快到期的证书在前。

## 创建仅查看的 Nginx Proxy Manager 用户 {#create-a-view-only-nginx-proxy-manager-user}

1. 在 Nginx Proxy Manager 中，打开用户并为 DumbMonit 添加用户，使用 dumbmonit@example.com 等自己的电子邮件地址。不勾选任何角色。

2. 在该用户的菜单中，打开权限：项目可见性 所有项目；代理主机、重定向主机、404 主机和流 仅查看；访问列表和 SSL 证书 隐藏。然后在同一菜单中用更改密码设置其密码。

3. SSL 证书有意保持隐藏：Nginx Proxy Manager 将导入证书的私钥和 DNS 挑战的凭证交给任何被允许查看它们的用户。DumbMonit 改为从其被提供的地方读取每个证书，对端口 443 进行 TLS 握手，对每个有证书的主机。

4. 在 DumbMonit 中，输入 Nginx Proxy Manager 主机及其网页界面的端口，例如"npm.lan:81"，以及该用户的电子邮件和密码。如果主机通过不同于 443 的另一端口通过 HTTPS 提供，请在下方设置 HTTPS 端口。

!!! warning
    配置 nginx 拒绝的主机（高级中的打字错误、缺失的证书文件）不被提供，Nginx Proxy Manager 仅在列表中将其显示为红点。Let's Encrypt 证书提前三十天更新：在少于十四天内过期的证书表示其更新失败，DumbMonit 然后警告。

项目可见性 所有项目很重要：使用 创建的项目，用户仅看到自己创建的主机，即没有。不具有代理主机 仅查看的用户被拒绝；不具有流、重定向主机或 404 主机的用户根本不报告它们。

DumbMonit 保留 Nginx Proxy Manager 在登录时给予的令牌（有效期一天），并在其过期时再次登录，而不是在每次检查时。

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| 用户电子邮件和密码 | 上面创建的用户的电子邮件地址和密码。 |

地址：带网页界面端口的主机名或 IP（`npm.lan:81`），或完整 URL。选项：协议（默认 HTTP），网页界面端口（81），检查证书（打开），HTTPS 端口（443），API 的证书检查和请求超时（10 秒）。

证书检查连接到地址的主机部分：当网页界面通过另一个代理到达时，将地址指向 Nginx Proxy Manager 本身，或取消勾选检查证书，改为用[TLS 证书检查](services.md)监控每个网站。通配符证书通过第一个使用它们的主机读取。
