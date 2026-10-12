# 域名到期 {#domain-expiry}

一个已注册的域名,通过 RDAP 从其注册局读取:距离到期还有多少天、
是否被置于导致脱离 DNS 的保留状态、赎回期、其注册商,以及其委派
是否使用 DNSSEC 签名。

RDAP 是 WHOIS 的后继者:同样的注册数据,以 JSON 格式呈现,在各个
注册局之间有统一的格式。DumbMonit 会在 IANA 的 RDAP 引导文件
(`https://data.iana.org/rdap/dns.json`,最多每天读取一次)中查找
该后缀对应的注册局,然后向其查询 `/domain/example.com`。无需任何
凭据,而且除了该域名的注册信息外不会查询任何其他内容。

除到期日期本身之外,还有两种值得关注的故障。处于 `clientHold` 或
`serverHold` 状态的域名已被注册商或注册局从 DNS 中移除,通常是因为
账单未支付或联系地址从未被验证:该域名下的所有网站和邮箱会立即
全部停止工作。处于 `redemptionPeriod` 或 `pendingDelete` 状态的域名
已经过期,除非出价赎回,否则将被释放。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_domain_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `expiry_days` | 距离到期剩余的天数,为小数;过期后为负数 | |
| `expiry_timestamp_seconds` | 到期日期,以 Unix 秒表示 | |
| `expiry_published` | 当注册局未公布到期日期时为 0 | |
| `on_hold` | 处于 `clientHold` 或 `serverHold` 时为 1 | |
| `redemption` | 处于 `redemptionPeriod`、`pendingDelete` 或 `pendingRestore` 时为 1 | |
| `status` | 固定值 1,每个状态码对应一条序列,采用 EPP 格式(`clientTransferProhibited`) | `status` |
| `registrar_info` | 固定值 1 | `registrar` |
| `dnssec` | 当委派已签名时为 1 | |
| `rdap_ok` | 当上次查询注册局失败、显示的是先前的应答时为 0 | |
| `rdap_age_seconds` | 当前显示应答的年龄 | |

注册局每个刷新间隔(默认 12 小时)只会被查询一次;在此期间,
每次检查时剩余天数都会被重新计算。当注册局没有应答时,之前的
应答仍会被使用,最多可用七天,此时 `rdap_ok` 为 0,因此注册局
故障不会被误认为是域名问题。

适用的[内置规则](../alerting/rules.md#reverse-proxies-and-domains):
域名即将到期(不足 30 天)、域名快到期(不足 7 天或已过期)、
域名被保留、域名处于赎回期,以及设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会查询注册局。
它会给出到期日期和剩余天数、状态码(或用一句话说明保留和赎回
状态)、注册商和 DNSSEC 情况,以及上次查询注册局是否失败。

## 监控一个域名的注册状态 {#watch-a-domains-registration}

1. 输入已注册的域名,例如 "example.com",而不是其下的某个名称如 www.example.com,注册局并不知道后者。无需任何设置,也无需凭据:RDAP(WHOIS 的后继者)是公开的。

2. DumbMonit 会在 IANA 的 RDAP 引导文件中查找该后缀对应的注册局,然后向其查询到期日期、状态码和注册商,默认每天两次。注册局会限制 RDAP 查询频率,而到期日期一年只变化一次:更短的刷新间隔没有任何好处。

3. 少数注册局运行 RDAP 服务却未在 IANA 中登记,例如 .ch 和 .li:请将 RDAP 服务器设置为它们的地址。注册局未运行公开 RDAP 服务的后缀无法被监控。

    ```
    https://rdap.nic.ch/
    ```

4. 有些注册局不公布到期日期(.ch 就是其中之一):此时 DumbMonit 会说明这一点,但仍会监控状态码。

!!! warning "警告"
    处于 clientHold 或 serverHold 状态的域名已被从 DNS 中移除:其下的所有网站和邮箱都会停止工作,通常是因为账单未支付或联系地址未验证。DumbMonit 会在到期前 30 天发出提醒,到期前 7 天升级告警级别,并在出现保留或赎回状态时立即告警。

注册局不认识的域名(子域名、拼写错误,或已被删除的域名)会被报告
为配置错误,而不是服务中断。带重音符号的名称会被接受,并以其
ASCII 形式(`xn--…`)进行查询。

## 凭据 {#credentials}

无:RDAP 是公开的。

地址:已注册的域名(`example.com`);粘贴的 URL 会被简化为其主机名。
选项:RDAP 服务器(留空:通过 IANA 查找)、刷新间隔(12 小时,
可选 1 到 168)和请求超时(10 秒)。设备本身的检查间隔可以保持
默认值:它只会在两次向注册局查询之间重新计算剩余天数。
