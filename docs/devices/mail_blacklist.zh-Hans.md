# 邮件服务器信誉 {#mail-server-reputation}

对一个邮件域名或邮件服务器的公网地址,对照仍然决定邮件能否送达的
DNS 黑名单进行检查:Spamhaus、Barracuda、SpamCop 以及几个规模较小的
名单。对于每一个收录了某地址的名单,监控项页面会给出该名单的应答
代码、其公布的原因,以及指向其查询和移除页面的直接链接。

**无需任何 API 密钥。** 将凭据保持为"无密钥(推荐)":这些名单通过服务器的 DNS
解析器查询,或通过你在**解析器**中设置的解析器查询。只有 Spamhaus 可能拒绝,而且
仅当该解析器是公共解析器时;见[解析器、拒绝与超时](#resolvers-refusals-and-timeouts)。

输入一个域名(`example.com`),DumbMonit 会检查其 MX 主机的地址(A 和
AAAA 记录)、域名本身的地址,以及你添加的任何**额外地址**(不属于 MX
的外发中继)。该域名还会对照域名黑名单进行检查。输入一个 IP 地址,则
只检查该地址。若要监视整段地址,请使用
[IP 段黑名单](ip_blacklist.md)。

## Gmail、Outlook.com 和 Yahoo {#gmail-outlook-com-and-yahoo}

大型邮箱服务商**没有公布任何可以查询的黑名单**:它们如何看待一个发件人
属于内部信息。DumbMonit 不会假装去询问它们,而是检查影响它们判断的
因素,并链接到它们的工具:

- **反向 DNS**:每个发信地址都应有一条 PTR 记录,其名称能解析回同一个
  地址(正向确认的反向 DNS,FCrDNS)。Gmail、Outlook.com 和 Yahoo 会
  拒收没有它的地址发来的邮件,或将其归入垃圾邮件。
- **SPF 和 DMARC**:域名上恰好有一条 SPF 记录,并且在
  `_dmarc.<domain>` 有一条 DMARC 记录。Gmail 和 Yahoo 要求批量发件人
  两者都具备。
- **下文的黑名单**,它们的过滤器同样会读取。

不检查 DKIM:其选择器由每个发件人自行选择,无法猜测。监控项页面链接到:

| 服务商 | 工具 |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/)(已验证域名的信誉)、[发件人指南](https://support.google.com/a/answer/81126)、[投递问题表单](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/)(每个地址的信誉数据)、[发件人政策](https://sendersupport.olc.protection.outlook.com/pm/)、[发件人支持表单](https://olcsupport.office.com/)(从 Outlook.com 移除)、[Microsoft 365 移除门户](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/)、[最佳实践](https://senders.yahooinc.com/best-practices/)、[发件人支持表单](https://senders.yahooinc.com/contact/) |

## 黑名单 {#the-blocklists}

检查于 2026 年 10 月 16 日。SORBS 已于 2024 年 6 月关闭,不再查询。
每个名单都在其区域下查询一条 `A` 记录。表单将它们显示为名为**黑名单**的
复选清单(名称、检查的是地址还是域名、是否主要,以及简短说明);**主要**
名单在新建监控项时默认勾选。第一列中的 id 就是 `lists` 选项所保存的内容,
也是 API 和指标对该名单的称呼。

| Id | 名单 | 区域 | 检查对象 | IPv6 | 主要 | 使用与移除 |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | 地址 | 是 | 是 | 面向低流量、非商业用途的免费公共镜像;拒绝公共和开放解析器。可在 [Spamhaus checker](https://check.spamhaus.org/) 移除。PBL 按策略列出终端用户的地址段:只有当某地址确实运行着邮件服务器时,才应将其从中移除。 |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | 地址 | 否 | 是 | 免费;Barracuda 要求查询它的 DNS 服务器在 barracudacentral.org 用免费账户登记。[移除申请](https://www.barracudacentral.org/rbl/removal-request)。 |
| `spamcop` | SpamCop | `bl.spamcop.net` | 地址 | 否 | 是 | 免费。举报停止后约一天,收录记录会自行过期;[查询页面](https://www.spamcop.net/bl.shtml)会显示原因并提供移除。 |
| `mailspike` | Mailspike | `bl.mailspike.net` | 地址 | 否 | 否 | 低流量免费。[查询与移除](https://mailspike.io/ip_verify)。 |
| `uceprotect_1` | UCEPROTECT Level 1 | `dnsbl-1.uceprotect.net` | 地址 | 否 | 否 | 查询免费。七天内没有新的垃圾邮件,收录记录就会过期;[检查页面](https://www.uceprotect.net/en/rblcheck.php)也出售加急移除,但这从来不是必需的。很少有收件方使用它。 |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | 地址 | 否 | 否 | 免费。可在[收录页面](https://psbl.org/)自助移除。 |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | 地址 | 否 | 否 | 低流量免费。RATS-NoPtr 列出没有反向 DNS 的地址。[查询](https://www.spamrats.com/lookup.php)和[移除](https://www.spamrats.com/removal.php)。 |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | 域名 | — | 是 | 条款与 ZEN 相同。可在 [Spamhaus checker](https://check.spamhaus.org/) 移除。 |
| `surbl` | SURBL (multi) | `multi.surbl.org` | 域名 | — | 否 | 低流量免费;拒绝公共和繁忙的解析器。[查询与移除](https://www.surbl.org/surbl-analysis)。 |
| `uribl` | URIBL (multi) | `multi.uribl.com` | 域名 | — | 否 | 低流量免费;拒绝公共和繁忙的解析器。[查询](https://lookup.uribl.com/),[移除](https://admin.uribl.com/)。 |

被**主要**名单收录会触发严重告警;被其他名单收录则触发通知级告警。
MX 的 IPv6 地址只对照 Spamhaus ZEN 检查,它是这里唯一对 IPv6 作出应答
的名单;其他名单会将其显示为未覆盖。

### 你自己的名单 {#your-own-lists}

在**自定义名单**中(位于复选清单的末尾),添加你使用或自己运行的任何其他名单的
DNSBL 区域(例如 `dnsbl.example.org`)。每一项都有**类型**:`ip`(针对每个地址
查询该区域,如 `1.2.0.192.dnsbl.example.org`)或 `domain`(针对邮件域名),
以及一个**主要**开关,用于标记被其收录时应触发严重告警的名单。区域必须是有效的
域名,不能是 IP 地址,也不能与另一个区域或上文的名单重复;最多 20 个自定义名单。
任何 `127.x.x.x` 应答都算作已列入;`NXDOMAIN` 算作干净。

监控项恰好查询已勾选的名单加上你自己的名单。在这些选项出现之前创建的监控项
没有该设置,会继续查询上文的所有名单。`lists` 选项保存以逗号分隔的已勾选 id
(一个都没有勾选时为 `-`),`custom_lists` 则是 JSON 数组,例如
`[{"zone": "dnsbl.example.org", "subject": "ip", "major": false}]`。

## 解析器、拒绝与超时 {#resolvers-refusals-and-timeouts}

这些名单通过 DNS 查询,其应答取决于**由哪个解析器发出查询**:

- **Spamhaus 会拒绝经由公共解析器发来的查询**(Google `8.8.8.8`、
  Cloudflare `1.1.1.1`、Quad9)、转发给它们的解析器,或没有可归属的
  反向 DNS 的解析器。此时它会应答 `127.255.255.254` 而不是
  `NXDOMAIN`(解析器发送的查询过多时为 `127.255.255.255`,查询格式
  错误或 DQS 密钥错误时为 `127.255.255.252`)。这些看起来像收录,但
  并不是:DumbMonit 会将 Spamhaus 名单标记为**已拒绝**(监控项页面显示
  "Spamhaus 拒绝此解析器"),绝不显示为"已列入",其他名单则保留各自的结果。
  只有在所选的其他名单都没有应答时,监控项才会显示**配置错误**(不会发送
  故障告警),而不是显示结果。
- **URIBL 和 SURBL 对被它们拒绝的解析器应答 `127.0.0.1`。** 该名单此时
  显示为"已拒绝"(未知),而不是收录。
- **未能及时应答的名单是未知,绝不是干净。** 只有 `NXDOMAIN` 才表示
  "未被列入"。落在 `127.0.0.0/8` 之外的应答(把 `NXDOMAIN` 改写成搜索
  页面地址的解析器)同样视为未知。

因此请使用**你自己的递归解析器**:从根服务器开始解析(而不是转发给
公共解析器)的 Unbound、Knot Resolver、BIND 或 PowerDNS Recursor,
并且如果它不是系统解析器,请在**解析器**中设置其地址。Pi-hole 或
AdGuard Home 放在 Unbound 前面可行;放在 `1.1.1.1` 前面则不行。

或者,仅当你也想要 Spamhaus 而又无法更换解析器时,创建一个免费的
[Spamhaus Data Query Service](https://www.spamhaus.com/data-access/free-data-query-service/)
账户(非商业用途,低流量),并将其密钥作为凭据输入("Spamhaus DQS 密钥(可选)"):
此时会通过 `<key>.zen.dq.spamhaus.net` 和 `<key>.dbl.dq.spamhaus.net` 向 Spamhaus
查询,它们通过任何解析器都能应答。密钥会加密保存,并且不会再次显示。

## 节奏 {#pace}

黑名单会限制解析器的查询频率,并封禁大规模扫描它们的解析器。DumbMonit
每个**刷新间隔**(默认 6 小时)对每个名单只查询一次,每秒最多
**查询数**(默认 10)个,同时最多八个查询在途,并在其间的每次检查中
再次显示最近一次的结果,使告警规则看到的是连续的序列。检查在后台
运行:添加监控项几秒后出现第一个结果,更改地址、某个选项或密钥会
立即开始新的检查。结果为错误的检查会在十分钟后重试。

## 对照黑名单检查邮件服务器 {#check-a-mail-server-against-the-blocklists}

1. 输入你发送邮件所用的域名,例如"example.com":DumbMonit 会检查其 MX 和域名本身的地址。或者输入服务器发信所用的公网 IP 地址。如果邮件经由不属于 MX 的中继发出,请在额外地址中添加其地址。

2. 无需任何 API 密钥:将凭据保持为"无密钥"。没有密钥时,黑名单通过本服务器的 DNS 解析器查询,或通过你在解析器中设置的解析器查询。

3. 在黑名单中勾选要查询的黑名单(主要名单默认已勾选),并在自定义名单中添加你使用的任何其他名单的 DNSBL 区域。

4. 只有 Spamhaus 可能拒绝查询,而且仅当解析器是公共解析器(Google、Cloudflare、Quad9)或转发给它们时。此时监控项显示"Spamhaus 拒绝此解析器",其他所有名单继续正常工作。若也想使用 Spamhaus,请将解析器设为你自己的递归解析器(一个从根服务器开始解析的 Unbound 就够了),或选择可选的 Spamhaus DQS 密钥作为凭据:这是一个免费账户(非商业用途,低流量),其密钥能让 Spamhaus 通过任何解析器应答。

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

5. 要检查设置是否正确,请为 127.0.0.2 添加一个监控项,这是每个黑名单都会收录的测试地址:每一个都应将其显示为已列入。

6. Gmail、Outlook.com 和 Yahoo 没有公布可以查询的黑名单。DumbMonit 转而检查它们所看重的因素(指回该地址的反向 DNS、SPF 和 DMARC 记录),并在监控项页面链接到它们的发件人工具。

!!! warning "警告"
    被 Spamhaus 拒绝的应答(127.255.255.254,公共解析器)绝不显示为收录:该名单会被标记为已拒绝,其他名单保留各自的结果。只有在没有其他名单应答时,监控项才会报告配置错误。未应答的黑名单显示为未知,绝不显示为干净。这些名单默认每 6 小时查询一次:更频繁地查询毫无收益,还会导致解析器被封禁。

## 监控内容 {#what-it-watches}

所有指标都带前缀 `dumbmonit_blacklist_`。

| 指标 | 含义 | 标签 |
|---|---|---|
| `listed_major_lists` | 收录了某个地址或该域名的主要名单(Spamhaus ZEN 和 DBL、Barracuda、SpamCop) | |
| `listed_lists` | 收录了某个地址或该域名的所有名单 | |
| `addresses_checked` | 已检查的地址 | |
| `addresses_listed` | 至少被一个名单收录的地址 | |
| `domain_listed` | 域名位于域名黑名单时为 1 | |
| `unknown_checks` | 没有应答的检查(超时、拒绝) | |
| `check` | 每个地址(或域名)和每个名单一条序列:1 已列入,0 干净,−1 无应答,−2 已拒绝,−3 未覆盖 | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | 每个名单:被查询的对象、已列入的对象、无应答的对象 | `list` |
| `list_info` | 值为 1,每个实际查询的名单(包括自定义名单)一个:监控项页面所显示的内容 | `list`, `name`, `zone`, `subject`, `major` |
| `address_info` | 值为 1,表示每个地址的来源 | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 反向 DNS 能解析回该地址时为 1 | `address`, `ptr` |
| `mx_hosts` | 域名的 MX 主机 | |
| `spf_records` | 域名上的 `v=spf1` 记录(1 为正确) | |
| `dmarc_records` | `_dmarc.<domain>` 处的 `v=DMARC1` 记录 | |
| `dmarc_policy_info` | 值为 1 | `policy` |
| `mail_auth_missing` | 0 到 2:SPF 缺失或重复,DMARC 缺失 | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | 所显示的检查:其时间、耗时,以及是否正在运行新的检查 | |

适用的[内置规则](../alerting/rules.md#dns-blocklists):被列入主要黑名单、被列入黑名单、邮件服务器反向 DNS 不一致、缺少 SPF 或 DMARC,以及在解析器对所有查询都没有应答时的监控项不可达。

## 监控项页面 {#the-monitor-page}

该面板读取探测所保存的内容;打开页面绝不会查询黑名单。它先显示计数,
然后是实际查询的名单(已勾选的和你自己的,每个都显示其收录的对象数,以及适用时的
"Spamhaus 拒绝此解析器"),再是每条结果(含名单的代码、其公布的原因,以及指向其
查询和移除页面的链接),接着是每个地址的反向 DNS、SPF 和 DMARC,最后是 Gmail、
Outlook.com 和 Yahoo 的发件人工具。

页面上还有一个**也在 MXToolbox 上检查**链接。它会在新标签页中打开 mxtoolbox.com 针对
被监控地址或域名的公开黑名单查询;DumbMonit 不会调用其 API。

## 凭据 {#credentials}

无需("无密钥(推荐)",默认值),或可选的 Spamhaus DQS 密钥(见上文),
它只对 Spamhaus 有意义。

地址:一个域名(`example.com`)或一个公网 IP 地址。选项:解析器、
黑名单(新建监控项时勾选主要名单;此前创建的监控项为全部)、自定义名单、
额外地址、刷新间隔(6 小时,范围 1 到 168)、每秒查询数(10,范围 1 到 50)
和每次查询的超时(5 秒,范围 1 到 30)。私有地址绝不会被检查:黑名单只了解
公共互联网。`127.0.0.2` 可作为测试地址。
