# Active Directory {#active-directory}

通过 LDAP 的 Windows 域,仅读:域控制器以及谁持有
每个 FSMO 角色、特权组的成员、密码策略、
账户计数,以及原始安全事实审计查看首先 —
账户开放给 Kerberoasting 或 AS-REP roasting、无约束委派、
一个旧的 krbtgt 密码、没有 LAPS 的计算机、Windows 发布超出
支持。

DumbMonit 使用一个专用服务账户作为**纯域
用户,不属于任何组**的身份。在 Active Directory 中每个认证用户可以
读 DumbMonit 读取;不需要管理权限,不应给予。DumbMonit 从不
写到目录、从不读密码属性(LAPS 从读过期日期,从不
密码)并从不询问会暴露秘密的复制权限。

一个监控项覆盖整个域:指向它一个域控制器以及
DumbMonit 在配置分区中找到其他。

## LDAP 能和不能做什么 {#what-ldap-can-and-cannot-tell}

LDAP 显示目录如它是,不显示发生在其中的事情的日志。特别是:

- **复制。** 查询的域控制器发布其入站复制伙伴的状态
  (最后成功、连续失败、最后
  错误)仅到持有*监视活跃目录
  复制*右的账户。不用它,面板说*复制状态不
  可读*而不是猜测。仅查询的域控制器的视图
  显示:看另一个的伙伴,添加它作为第二个监控项。
- **不在 LDAP 上可见**:事件日志(登录、锁定时发生、
  组更改作为事件)、Group Policy 内容、SYSVOL、DNS 区域以及
  密码哈希本身。
- **最后登录日期**来自`lastLogonTimestamp`,域
  控制器仅每 9 到 14 天复制:一个账户计为
  不活跃在两周内不活跃。
- **组成员身份**用嵌套读取(` LDAP_MATCHING_RULE_IN_CHAIN`
  规则),通过`member`/`memberOf`。由`primaryGroupID`设置的主要组
  不是`member`链接并未看到。

## 它如何读 {#how-it-reads}

两个步骤:

- **在每个检查**时(默认每分钟):连接和绑定,
  RootDSE(功能级别、时钟、同步)、域对象
  (密码策略、机器账户配额)、5 个 FSMO 角色、域
  控制器从配置分区、特权组及其
  有效成员、回收站特性以及 TCP 连接到
  每个其他域控制器的 LDAP 端口。
- **在后台**每`inventory_minutes`(默认 15):每个用户
  和计算机账户,仅带发现需要的属性,在
  500 页面。每分钟枚举整个目录会加载域
  控制器毫无目的。

第一个检查等待几秒以清点;在大域上
账户发现出现在下一个检查。

## 它观察什么 {#what-it-watches}

所有度量前缀为`dumbmonit_ad_`。

| 度量 | 什么 | 标签 |
|---|---|---|
| `bind_ok` | 1 当服务账户被接受时,0 当域控制器拒绝它时 | |
| `connect_seconds`、`bind_seconds` | 打开连接(TLS 包括)的时间以及绑定的时间 | |
| `tls_verified` | 1 当证书被检查时 | |
| `domain_info` | 值 1 | `domain`、`forest`、`dc` |
| `domain_functional_level`、`forest_functional_level` | 0(2000)到 7(2016)、10(2025) | |
| `dc_synchronized`、`dc_global_catalog_ready` | 查询的域控制器已完成其初始复制,其全局目录就绪 | |
| `clock_skew_seconds` | 域控制器时钟减去 DumbMonit 主机时钟 | |
| `recycle_bin_enabled` | 1 当 AD 回收站已启用时 | |
| `dc_count` | 域的域控制器 | |
| `dc_info` | 值 1 | `dc`、`site`、`gc`、`rodc`、`os` |
| `dc_reachable` | 1 或 0 每个域控制器;其名称从 DumbMonit 解决时无系列 | `dc` |
| `fsmo_role` | 值 1 | `role`(`schema`、`domain_naming`、`pdc`、`rid`、`infrastructure`)、`holder` |
| `replication_readable` | 1 当入站复制状态可被读取时 | |
| `replication_consecutive_failures`、`replication_last_success_age_seconds` | 每入站伙伴和分区 | `source`、`naming_context` |
| `privileged_group_members`、`privileged_group_enabled_members` | 有效成员、嵌套包括、组遗漏 | `group` |
| `privileged_group_fingerprint` | 成员的哈希:更改当有人加入或离开时 | `group` |
| `users_total`、`users_enabled`、`users_disabled`、`users_locked_out`、`users_password_never_expires`、`users_stale` | 用户账户 | |
| `computers_total`、`computers_enabled`、`computers_stale`、`groups_total` | 计算机和组 | |
| `admincount_accounts` | 由 AdminSDHolder 保护的用户(`adminCount=1`) | |
| `krbtgt_password_age_seconds` | krbtgt 密码的年龄 | |
| `laps_schema`、`laps_computers`、`laps_eligible_computers` | 架构中的 LAPS;启用的非 DC 计算机带 LAPS 过期,超出所有它们 | |
| `password_min_length`、`password_history_length`、`password_max_age_seconds`(0 = 从不)、`password_complexity`、`lockout_threshold`、`machine_account_quota` | 默认域策略 | |
| `finding_count` | 关注的对象发现,当检查和干净时为 0 | `finding`、`severity`、`title` |
| `inventory_age_seconds`、`scrape_errors` | 最后完整清点的年龄;无法读取的目录部分 | |

特权组通过其众所周知的标识符找到,不通过其
名称,所以在另一种语言安装的域工作相同:
域管理员(RID 512)、企业管理员(519)和架构管理员(518) — 
最后两个仅在森林根域 — 以及内置管理员、
账户运营商、服务器运营商、打印运营商和备份运营商。

## 发现 {#findings}

发现是原始事实 — 多少对象以及哪个 — 不是分数;
监控项的[安全分数](../using/security-score.md)权衡它们。每个有一个
稳定的`id`、分数尺度上的严重级别(低、中、高、临界、
在页面显示为信息、通知和警告)、一个类别、一个计数以及
最多 20 个样本账户名。一个被检查和发现什么都没有的发现
保持计数为 0,所以一个固定的问题变绿而不是
消失;账户发现出现一旦第一个背景
清点完成。它们由`GET /api/targets/{id}/ad/findings`提供如
并发布为
`dumbmonit_ad_finding_count{finding, severity, title}`。

| `id` | 严重级别 | 什么 |
|---|---|---|
| `unconstrained_delegation` | critical | 启用的账户信任无约束委派,域控制器除外 |
| `kerberoastable_privileged` | critical | 特权组的启用成员有服务主体名称 |
| `asrep_roastable_users` | high | 启用的账户禁用了 Kerberos 预认证(`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | high | 启用的用户有服务主体名称 |
| `krbtgt_password_age` | high | krbtgt 密码比 180 天旧 |
| `laps_coverage` | high 当 LAPS 缺失时,medium 当部分 | 启用的计算机(域控制器除外)没有 LAPS 密码 |
| `reversible_encryption` | high | 用户密码存储且可逆加密 |
| `guest_enabled` | high | 内置 Guest 账户(RID 501)已启用 |
| `obsolete_os` | high | 启用的计算机在 Windows XP、Vista、7、8、2000、Server 2003、2008 或 2012 上 |
| `dc_unreachable` | high | 其 LDAP 端口不应答的域控制器 |
| `replication_failing` | high | 入站复制伙伴有连续失败 |
| `dc_not_synchronized` | high | 查询的域控制器未完成其初始复制 |
| `fsmo_role_orphaned` | high | FSMO 角色由已删除的域控制器持有 |
| `password_policy` | 8 个字符下 high 或无锁定、medium 否则 | 默认域策略中的间隙对基线:14 个字符,锁定阈值,复杂性 |
| `privileged_group_size` | medium | 超过 5 个启用的账户在域管理员、企业管理员和管理员中 |
| `stale_accounts` | medium | 启用的用户和计算机不活跃超过阈值 |
| `protocol_transition_delegation` | medium | 约束委派带协议转换 |
| `des_only` | medium | 账户限制于 DES Kerberos 密钥 |
| `password_not_required` | medium | 用户允许空密码(`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | medium | 特权用户密码永不过期 |
| `privileged_stale` | medium | 特权用户不活跃超过`stale_days_users` |
| `schema_admins_not_empty` | medium | 架构管理员有成员 |
| `functional_level_old` | medium | 域功能级别下 2012 R2 |
| `clock_skew` | medium | 域控制器和 DumbMonit 之间超过 5 分钟 |
| `windows10_end_of_support` | medium | 启用的计算机在 Windows 10 上 |
| `admincount_orphans` | low | `adminCount=1`用户不再在特权组中 |
| `password_never_expires` | low | 启用的用户密码永不过期 |
| `machine_account_quota` | low | 任何用户可加入计算机到域 |
| `recycle_bin_disabled` | low | AD 回收站未启用 |

[内置规则](../alerting/rules.md#active-directory)适用:

- **域控制器不可达**:域的另一个域控制器
  停止在 5 分钟内在其 LDAP 端口上应答(警告)。查询的被
  *监控项不可达*覆盖。
- **Active Directory 绑定失败**:域控制器拒绝服务
  账户 — 过期密码、禁用或锁定(通知)。
- **特权组成员身份更改**:有人加入或离开
  特权组、嵌套包括(通知、一小时后解决)。
- **Active Directory 复制失败**:入站伙伴半小时失败(通知),
  当复制状态可读时。
- **krbtgt 密码比 180 天旧**(信息)。

## 监控项页面 {#the-device-page}

Active Directory 面板显示域(功能级别、查询的
域控制器、时钟偏差)、带其站点、角色及是否应答的域控制器、
特权组及其成员、密码策略和账户计数以及
发现表带严重级别词。

## 为 DumbMonit 创建读唯一服务账户 {#create-a-read-only-service-account-for-dumbmonit}

1. 为 DumbMonit 创建专用用户,例如 svc-dumbmonit,并添加到没有组。纯域用户足够:每个认证用户可以读 DumbMonit 读取。在 PowerShell 中,带 Active Directory 模块:

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. 给它一个长随机密码。如果密码在你的域中过期,DumbMonit 提升 Active Directory 绑定失败这一天。

3. 可选:查看入站复制状态,在域对象本身授予账户监视活跃目录复制权。它暴露无密码和无秘密。

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. LDAPS 需要每个域控制器上的证书;企业认证机构(AD CS)自动发布。在基础 64 编码的 X.509 格式导出该权威证书并粘贴为 CA 证书下方。

5. 在 DumbMonit 中,输入一个域控制器的名称如其证书命名它,例如"dc1.corp.example.com"和登录作为 svc-dumbmonit@corp.example.com。DumbMonit 自己找到其他域控制器并检查每个应答。

6. DumbMonit 仅读:它从不写到目录,LAPS 从读过期日期,从不密码。

!!! warning
    纯 LDAP 在端口 389 以纯文本发送服务账户密码:DumbMonit 拒绝它除非你勾选允许纯 LDAP,并且要求 LDAP 签名的域控制器也拒绝它。使用 LDAPS 或 StartTLS 在端口 389。

保持账户打不开任何地方的会话,否认它*本地登录*以及*通过远程桌面服务登录*由 Group Policy:LDAP 绑定不受影响。不限制它带*登录到*或*拒绝访问此计算机从网络*:域控制器然后拒绝绑定。

### 最少特权,详细 {#least-privilege-in-detail}

- 账户不需要超出域用户的组,每个账户有。
- 不使用托管服务账户(gMSA):它无法做简单 LDAP 绑定。
- 不授予*复制目录更改*(或*所有*):那是 DCSync
  权,暴露每个密码哈希。DumbMonit 从不需要它。
- *监视活跃目录复制*仅允许读复制
  元数据(伙伴、最后成功、错误码)。它是可选的。

## LDAPS 和证书 {#ldaps-and-certificates}

- **LDAPS(端口 636)**是默认。域的每个域控制器
  有企业认证机构自动接收证书;
  不用,域控制器不在 636 上监听。
  证书必须携带 DumbMonit 使用的名称:输入域
  控制器按其 DNS 名称,不按其 IP 地址,除非证书列出
  地址。
- **CA 证书**:粘贴根(或发行)权威的 PEM。仅那
  权威然后被信任。不用,DumbMonit 信任公有
  根和其容器的系统存储,不知道一个私有
  企业权威。
- **StartTLS(端口 389)**以相同方式加密,在纯文本问候后。
  当端口 636 被过滤时使用它。
- **接受不可验证的证书**保持连接加密但
  接受任何证书:路径上的任何人可能冒充域控制器
  和接收服务账户密码。
- **LDAP 通道绑定**(`LdapEnforceChannelBinding`)涉及 NTLM 和
  Kerberos 在 TLS 上绑定;简单绑定 DumbMonit 使用不受影响。

## 凭证 {#credentials}

| 字段 | 值 |
|---|---|
| 用户名 | `svc-dumbmonit@corp.example.com`、`CORP\svc-dumbmonit`,或账户的完整 DN |
| 密码 | 账户的密码,存储加密并从不再显示 |

空密码被拒绝:Active Directory 会接受它作为
无认证的绑定不检查任何东西。

## 选项 {#options}

| 选项 | 默认 | 什么 |
|---|---|---|
| 连接安全(`security`) | `ldaps` | `ldaps`、`starttls`或`plain` |
| 端口(`port`) | 636,或 389 | 使用如果地址不给 |
| CA 证书(`ca_cert`) | | 发布域控制器证书权威的 PEM |
| 接受不可验证的证书(`insecure_tls`) | 关 | |
| 允许纯 LDAP(`allow_plaintext`) | 关 | 需要`plain` |
| 检查每个域控制器(`check_all_dcs`) | 开 | TCP 连接到每个域控制器的 LDAP 端口 |
| 不活跃用户在(`stale_days_users`) | 90 | 天 |
| 不活跃计算机在(`stale_days_computers`) | 90 | 天 |
| 完整清点每(`inventory_minutes`) | 15 | 分钟,1 到 1440 |
| 超时(`request_timeout_seconds`) | 8 | 每 LDAP 操作,1 到 60 |

## 故障排除 {#troubleshooting}

- **TLS 证书被拒绝**:粘贴权威证书作为 CA
  证书,并输入域控制器按其证书
  列出的名称。
- **绑定被拒绝:错误用户名或密码(LDAP 结果 49)**:
  从域控制器的诊断码被翻译 — 密码
  过期、账户禁用、锁定或必须在下次登录时更改其密码。
- **域控制器需要保护的连接**:LDAP 签名被
  强制;切换到 LDAPS 或 StartTLS。
- **一个域控制器显示为未解决**:其名称不解决
  从 DumbMonit 主机。指向 DumbMonit DNS 在域的 DNS 服务器,
  或取消勾选*检查每个域控制器*。未解决的名称永不提升告警。
- **复制状态不可读**:授予*监视活跃目录
  复制*如步骤 3,或接受复制不被显示。
