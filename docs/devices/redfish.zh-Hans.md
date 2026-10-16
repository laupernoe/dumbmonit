# 服务器硬件(Redfish) {#server-hardware-redfish}

服务器自身的硬件,通过其管理控制器(BMC)读取:风扇、相对于各自
阈值的温度、电源及其冗余状态、驱动器、内存、处理器和事件日志。

Redfish 是 DMTF 为管理控制器标准化的 HTTPS 接口,目前所有主流
控制器都支持它:Supermicro、Dell iDRAC、HPE iLO、Lenovo
XClarity Controller、ASRock Rack、OpenBMC。这是监控服务器硬件
正确的方式。相比之下,BMC 的 SNMP 代理通常描述的是 BMC 本身——
它的小型 ARM Linux 系统、它的内存、它的网络链路——而不是它所
管理的服务器(参见
[SNMP](snmp.md#server-management-controllers-bmc))。

!!! warning "仅依据 DMTF 参考模拟环境测试"

    这项集成是依据官方 DMTF Redfish 模拟环境(机架式服务器、
    带本地存储的塔式服务器、使用较新热量和电源模式的液冷服务器)
    构建并验证的,由 DMTF 的模拟服务器提供。每次调用都是读取
    操作,资源结构都是标准的,但尚未在真实的 Supermicro、iDRAC
    或 iLO 上长期运行过。如果有任何问题,请告诉我们。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_redfish_` 为前缀。健康状态序列的值直接
来自 `Status.Health`:**0(正常)、1(警告)或 2(严重)**。

| 指标族 | 指标 | 标签 |
|---|---|---|
| 服务 | `info`(固定值 1) | `vendor`, `product`, `redfish_version` |
| 系统 | `system_health`(系统自身的)、`system_health_rollup`(其包含的一切)、`system_power_on`、`memory_health`、`processor_health`(控制器的汇总值,所有内存条和所有 CPU 共用一个值)、`memory_total_bytes` | `system` |
| 机箱 | `chassis_health`, `chassis_health_rollup` | `chassis` |
| 温度 | `temperature_celsius`、`temperature_upper_critical_celsius` 和 `temperature_upper_caution_celsius`(该传感器自身声明的严重和警戒阈值,如果有)、`temperature_health` | `chassis`, `sensor` |
| 风扇 | `fan_speed_rpm`(或控制器以百分比报告时的 `fan_speed_percent`)、`fan_lower_critical_rpm`(或 `_percent`:风扇自身声明的最低转速,仅旧版模式)、`fan_health`、`fan_redundancy_health` | `chassis`, `fan` / `group` |
| 电源 | `psu_health`, `psu_output_watts`, `psu_capacity_watts`, `power_redundancy_health`, `power_consumed_watts` | `chassis`, `psu` / `group` |
| 电压 | `voltage_volts`, `voltage_health` | `chassis`, `sensor` |
| 存储 | `storage_health`, `drive_health`, `drive_failure_predicted`, `drive_life_left_percent`(SSD)、`drive_capacity_bytes` | `system`, `storage`, `drive`, `media` |
| 控制器 | `manager_health`, `manager_info`(固定值 1) | `manager`, `firmware`, `model` |
| 日志 | `log_entries`, `log_critical_entries`, `log_warning_entries` | `owner`(系统或控制器)、`log` |
| 采集 | `scrape_errors`, `scrape_duration_seconds` | |

**空插槽不是故障。** 一个 `Status.State` 为 `Absent`(空的电源
仓位、空闲的内存插槽、缺失的第二个 CPU)或 `Disabled`(未配置的
冗余组)的组件完全不会产生任何序列。一台只装了两个电源中的一个
就出厂的服务器是健康的,也会被这样显示。

**两代模式,同一套序列。** 目前仍在服役的大多数控制器仍在公布
较旧的 `Chassis/{id}/Thermal` 和 `Chassis/{id}/Power` 资源,每个
传感器各需一次请求;最新的控制器只公布 `ThermalSubsystem`、
`PowerSubsystem`、`EnvironmentMetrics` 和 `Sensors`,每个风扇、
电源和传感器各对应一个资源。DumbMonit 在较旧形式存在时读取它,
否则读取较新的形式;两者产生完全相同的序列和标签,因此一条规则
写一次就能同时适用于两者。

**日志只会被计数,从不会被读取。** 日志条目可能包含主机名、地址
和用户名:DumbMonit 只保留条目总数,以及控制器提供的第一页中有
多少条是严重或警告级别。

适用的[内置规则](../alerting/rules.md#server-hardware-redfish):
服务器风扇故障、服务器温度超过严重阈值、电源冗余丢失、电源故障、
预测驱动器将发生故障、服务器健康状态严重,以及监控项不可达。

## 监控项页面 {#the-device-page}

一个 Redfish 监控项会在其页面上、图表上方得到自己的专属面板,读取
自上次探测存储的数据:打开该页面从不会查询控制器。它按以下顺序
作答:

1. **服务器是否健康。** 一个词——健康、降级或严重——来自控制器
   自身的健康状态和汇总值,下方用一句话列出每个拉低它的组件:
   *CPU1 Temp is at 47 °C, at or above its critical threshold of
   45 °C*、*Power supply redundancy is lost*、*Drive 3 predicts
   its own failure*。当控制器把服务器汇总为降级、却没有指明任何
   DumbMonit 能读到的原因时,面板会说明这一点并指向其事件日志。
   下方:电源状态、功耗、已安装内存和控制器的固件版本。
2. **风扇和温度是否在限制范围内。** 每个传感器都会与控制器**为
   该传感器**声明的阈值进行比较——它的警戒和严重温度、风扇的
   最低转速——绝不会与 DumbMonit 自己的数字比较。一个没有声明
   阈值的传感器只会显示其读数,不给出判定,除非控制器自身报告
   它处于降级状态。问题会被优先列出。
3. **电源是否冗余。** 每个冗余组(冗余、冗余降级、冗余丢失),
   然后是每个存在的电源及其输出相对于容量的比例和状态。如果
   没有冗余组,面板会说明控制器没有声明任何冗余组:单电源,或
   未配置冗余。
4. **驱动器是否健康。** 每个存储控制器,然后是每个驱动器及其
   容量、SSD 的剩余寿命和其状态;*预测将发生故障*会排在最前面。
5. **内存、处理器和控制器**:控制器为所有内存条和所有 CPU 给出
   的单词汇总、管理控制器自身的健康状态、型号和固件,以及电压。
6. **事件日志**:按严重级别统计的条目数。条目会一直保留直到有人
   清除日志,因此一个严重级别的计数可能是旧消息:它会被显示
   出来,但不会改变服务器健康状态的那个词。

空仓位和空闲插槽永远不会出现:它们不会产生任何序列。每个状态都
既是一个词也是一种颜色,未知值显示为"—",绝不显示为 0。

## 在管理控制器上创建一个只读账户 {#create-a-read-only-account-on-the-management-controller}

1. 打开管理控制器(BMC)的网页界面,而不是服务器的操作系统。创建一个按如下方式命名的新本地用户,使用一个不在任何其他地方使用的较长密码。

    ```
    dumbmonit
    ```

2. 赋予它能读取的最低角色,不多不少。Supermicro:Configuration → Users → Add User,权限 User,并在账户类型中勾选 Redfish。Dell iDRAC:iDRAC Settings → Users → Local Users → Add,角色 Read Only。HPE iLO:Administration → User Administration → New,取消勾选除 Login 之外的所有权限。Lenovo XClarity Controller:BMC Configuration → User/LDAP → Create,角色 Read-only。ASRock Rack:Settings → User Management,权限 User。

3. 从管理网络上的任意一台机器确认该账户能读取 Redfish。该命令会要求输入密码,并打印服务器的名称和健康状态。

    ```
    curl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems
    ```

4. 在 DumbMonit 中,输入控制器地址,例如 "bmc.lan" 或 "10.0.0.50",然后输入该账户的用户名和密码。

5. DumbMonit 只会读取:风扇、温度、电压、电源、驱动器、内存和处理器汇总信息,以及按严重级别统计的日志条目数。它从不开关服务器电源,从不更改任何设置,也从不读取日志的文本内容。

!!! warning "警告"
    不要重复使用控制器的出厂账户:它能关闭服务器电源、挂载介质
    和刷写固件。管理控制器通常也自带自签名证书:如果因此连接
    被拒绝,请在选项中勾选"接受无法验证的证书"。控制器应答较慢,
    完整读取需要多次请求:如果监控项报告超时,请调高服务器上的
    DUMBMONIT_PROBE_TIMEOUT_SECS。

以上菜单名称是当前固件的名称,较旧的代际会把它们挪到别处,但
角色保持一致。在 Supermicro 上,创建用户时选择的账户类型决定了
它可以使用哪些接口:没有 Redfish 访问权限的账户即使密码正确也会
被以 401 拒绝。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 管理控制器账户 | 该只读账户的用户名和密码。 |

地址:控制器的主机名或 IP,例如 `bmc.lan` 或 `10.0.0.50`。默认
端口是 443;写成 `host:port` 可使用其他端口,或使用完整 URL
(`https://bmc.lan:8443`)。

## 选项 {#options}

| 选项 | 默认值 | 作用 |
|---|---|---|
| `port` | `443` | HTTPS 端口,当地址中未给出时使用。 |
| `insecure_tls` | `false` | 接受自签名证书。 |
| `request_timeout_seconds` | `8` | 每次 Redfish 调用允许的时间,1 到 120。 |
| `auth` | `basic` | `basic` 会在每个请求中发送用户名和密码,不在控制器上保留任何打开的会话。`session` 只登录一次并复用会话令牌(`X-Auth-Token`),只有在控制器拒绝它时才重新打开,并关闭被替换的那个会话。只有当控制器拒绝基本认证时才使用它:控制器的会话槛位数量非常有限。 |
| `storage` | `true` | 读取存储控制器及其背后的每个驱动器。每个驱动器一次请求。 |
| `logs` | `true` | 按严重级别统计系统事件日志和控制器自身日志的条目数。 |

一次完整读取需要几十次请求:服务根、然后每个系统、机箱和控制器、
它们的热量和电源资源、存储控制器和驱动器、日志服务。DumbMonit
最多同时发送四个请求。较慢的控制器可能需要更长的探测超时
(`DUMBMONIT_PROBE_TIMEOUT_SECS`,参见
[配置](../reference/configuration.md))以及两分钟或更长的探测
间隔。

## 常见错误 {#common-errors}

| 现象 | 可能原因 |
|---|---|
| *认证被拒绝* | 密码错误,或该账户不允许使用 Redfish(Supermicro 的账户类型、iLO 缺少"Login"权限)。 |
| *权限不足* | 该角色对某个资源权限过低;其余部分仍会正常读取。有些控制器会对最低角色隐藏事件日志。 |
| *TLS 证书被拒绝* | 控制器的自签名证书:勾选"接受无法验证的证书",或在控制器上安装一个受信任的证书。 |
| *超时* | 控制器应答缓慢:调高 `DUMBMONIT_PROBE_TIMEOUT_SECS`,关闭 `storage` 或 `logs`,或延长检查间隔。 |
| 没有温度阈值 | 控制器没有为该传感器公布 `UpperThresholdCritical`:温度仍会被记录,但内置规则无法将其与任何值比较。 |
