# Nginx Proxy Manager

代理管理器:nginx 拒绝接受其配置的主机、已禁用的主机,以及各主机
出示的证书——在 Let's Encrypt 续签失败演变成网站过期之前就能发现。

DumbMonit 使用自己的用户登录 Nginx Proxy Manager 的 REST API
(`/api`,位于其网页界面的端口上,81),然后读取代理主机、重定向
主机、404 主机(API 中称为 dead host)以及流。每个主机都会说明
是否已启用,以及 nginx 是否接受了它的配置。被 nginx 拒绝的主机
不会被提供服务,网页界面只会把它显示为一个红点。

证书不是从 API 读取的。`/api/nginx/certificates` 会把已导入证书的
私钥、以及 DNS 验证方式的凭据,交给任何被允许查看证书的用户。
DumbMonit 改为在证书被实际提供服务的地方读取它:对 Nginx Proxy
Manager 443 端口进行 TLS 握手,并使用某个使用该证书的主机名称。
这样也能测量访客实际获得的证书。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_npm_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | 各类型的主机数量 | `type`:`proxy`、`redirection`、`dead`(404 主机)、`stream` |
| `hosts_offline` | 已启用但 nginx 拒绝其配置的主机 | `type` |
| `host_online` | 每个已启用主机:当 nginx 拒绝其配置时为 0 | `type`, `host` |
| `host_error_info` | 固定值 1,对于被 nginx 拒绝的主机:nginx 错误信息的第一行 | `type`, `host`, `error` |
| `certificates_checked`, `certificates_read` | 挂载在已启用主机上的证书数量,以及通过 HTTPS 端口实际读取的数量 | |
| `cert_expiry_days` | 某主机出示的证书距离过期的剩余天数 | `host` |

`host` 是该主机的第一个域名,对于流则是 `port 9000`。已禁用的主机
只会被计数,不会被详细描述:禁用一个主机是有意为之的操作,
它对应的告警(如果有)会自动解决。主机会被逐一描述,最多 500 个,
每次检查最多读取 100 个证书。

适用的[内置规则](../alerting/rules.md#reverse-proxies-and-domains):
Nginx Proxy Manager 主机离线、代理证书未续签,以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询 Nginx
Proxy Manager。它会用一句话说明哪里出了问题(被 nginx 拒绝的主机
及其原因、即将过期的证书、无法读取的证书),然后是代理主机、
重定向、404 主机、流和已禁用的主机,再然后是按过期时间从早到晚
排序的证书列表。

## 创建一个仅查看权限的 Nginx Proxy Manager 用户 {#create-a-view-only-nginx-proxy-manager-user}

1. 在 Nginx Proxy Manager 中打开 Users,为 DumbMonit 添加一个用户,使用它自己的电子邮箱地址,例如 dumbmonit@example.com。不要勾选任何角色。

2. 在该用户的菜单中打开 Permissions:Item Visibility 设为 All Items;Proxy Hosts、Redirection Hosts、404 Hosts 和 Streams 设为 View Only;Access Lists 和 SSL Certificates 设为 Hidden。然后在同一菜单中用 Change Password 设置密码。

3. SSL Certificates 之所以保持 Hidden 是有意为之:Nginx Proxy Manager 会把已导入证书的私钥、以及 DNS 验证方式的凭据,交给任何被允许查看它们的用户。DumbMonit 改为在证书被实际提供服务的地方读取它,对每个拥有证书的主机的 443 端口进行 TLS 握手。

4. 在 DumbMonit 中,输入 Nginx Proxy Manager 主机地址及其网页界面的端口,例如 "npm.lan:81",以及该用户的电子邮箱和密码。如果各主机通过非 443 端口提供 HTTPS 服务,请在下方设置 HTTPS 端口。

!!! warning "警告"
    一个配置被 nginx 拒绝的主机(Advanced 中的拼写错误、缺失的证书文件)不会被提供服务,Nginx Proxy Manager 只会在列表中把它显示为一个红点。Let's Encrypt 证书会提前三十天续签:若某证书的有效期不足十四天,说明其续签失败了,DumbMonit 会在此时发出警告。

Item Visibility 设为 All Items 很重要:如果设为 Created Items,
该用户只能看到自己创建的主机,而它实际上没有创建任何主机。没有
Proxy Hosts View Only 权限的用户会被拒绝;没有 Streams、Redirection
Hosts 或 404 Hosts 权限的用户只是不会报告这些内容。

DumbMonit 会保留 Nginx Proxy Manager 登录时发出的令牌(有效期一天),
并在其过期时重新登录,而不是每次检查都重新登录。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 用户邮箱和密码 | 上面创建的用户的电子邮箱地址和密码。 |

地址:主机名或 IP,带网页界面的端口(`npm.lan:81`),或完整 URL。
选项:协议(默认 HTTP)、网页界面端口(81)、检查证书(开启)、
HTTPS 端口(443)、API 的证书校验,以及请求超时(10 秒)。

证书检查会连接到地址中的主机部分:当网页界面是通过另一个代理
访问的,请把地址指向 Nginx Proxy Manager 本身,或者取消勾选
"检查证书",改用[TLS 证书检查](services.md)逐个监控各个站点。
通配符证书会通过使用它们的第一个主机来读取。
