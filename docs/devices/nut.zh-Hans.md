# 通过 NUT 监控 UPS {#ups-with-nut}

通过 USB 连接到 NAS、Raspberry Pi 或服务器的 UPS,通常由 Network
UPS Tools(NUT)监控:其守护进程 `upsd` 会在 TCP 3493 端口上公布
UPS 报告的内容。DumbMonit 像任何 NUT 客户端一样读取该服务器:
UPS 是否在使用市电还是电池、电池是否电量低或已老化、电量、剩余
运行时间、负载、输入电压以及电池的使用年限。一台服务器通常会
公布多个 UPS;它们都会在一次连接中被读取,每个序列都带有其 UPS
的名称。

对于带有自己网卡的 UPS,请改用 [SNMP 设备](snmp.md)类型读取标准
的 UPS-MIB。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_nut_` 为前缀,并带有 `ups` 标签,即
`ups.conf` 中的 UPS 名称(在 Synology NAS 上是 `ups`,在 QNAP 上是
`qnapups`)。UPS 未公布的变量不会产生任何序列。

| 指标 | 含义 | 标签 |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | `ups.status` 的各个标志位(`OL`、`OB`、`LB`、`HB`、`RB`、`CHRG`、`DISCHRG`、`BYPASS`、`CAL`、`OFF`、`OVER`、`TRIM`、`BOOST`、`FSD`、`ALARM`):设置时为 1,否则为 0,每次读取都会给出全部这些值 | `ups` |
| `ups_status_info` | 固定值 1,即原始的 `ups.status` | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | 电量,以及 UPS 报告低电量时的电量阈值 | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | 当前负载下电池剩余运行时间,以及低电量时的运行时间 | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | 自 `battery.date`(安装或更换)起经过的时间,否则取 `battery.mfr.date` | `ups`, `source`(`installed`、`manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | 市电侧;断电期间为 0 V | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | 负载侧 | `ups` |
| `ups_load_percent` | 负载,占 UPS 容量的百分比 | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | 实际功率和额定功率(当 UPS 报告这些数据时) | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 当上次自检(`ups.test.result`)报告错误或警告时为 1,通过时为 0;未进行过自检时不存在 | `ups`, `result` |
| `ups_info` | 固定值 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 当 `upsd` 应答但没有来自该 UPS 的新数据时为 1(`DATA-STALE`、`DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 当该 UPS 的 NUT 驱动未运行时为 0 | `ups` |
| `server_ups` | 该服务器公布的 UPS 数量 | |
| `server_info` | 固定值 1 | `version` |

序列号绝不会出现在任何标签中。每个服务器最多读取 32 个 UPS。

适用的[内置规则](../alerting/rules.md#ups-behind-nut):UPS 正在
使用电池、UPS 电池电量低、UPS 电池需要更换、UPS 负载过高、UPS
运行时间不足、UPS 数据陈旧,以及当 NUT 服务器本身停止应答时的
设备不可达。

## 设备页面 {#the-device-page}

图表上方的面板读取探测存储的数据;打开该页面从不会连接到 NUT
服务器。每个 UPS 一张卡片:其文字状态(市电供电、使用电池、
电量低、需要更换电池、旁路、过载……)、电量、剩余运行时间、负载、
输入电压以及电池的使用年限。

## 让 DumbMonit 读取 NUT 服务器 {#let-dumbmonit-read-the-nut-server}

1. 找到 NUT 服务器:UPS 所连接的那台机器(NAS、Raspberry Pi、服务器)运行着监听 TCP 3493 端口的 upsd。从 DumbMonit 所在主机确认它能应答,并记下它的 UPS 名称。

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    安装了 NUT 客户端工具的话,`upsc -l nas.lan` 能给出同样的列表。

2. 在普通的 NUT 安装中,让 upsd 监听一个 DumbMonit 所在主机能访问的地址:把这一行添加到 upsd.conf 中,在防火墙中为 DumbMonit 所在主机开放 TCP 3493 端口,然后重启 upsd。读取不需要任何账户:upsd 允许它接受的任何主机读取变量,在没有具备相应权限用户的情况下不会更改任何内容。

    ```
    LISTEN 0.0.0.0 3493
    ```

3. 在 NAS 上,同样的开关有不同的名称。Synology DSM:控制面板 → 硬件与电源 → UPS,勾选"启用网络 UPS 服务器",并在"允许的 DiskStation 设备"中添加 DumbMonit 所在主机;该 UPS 名为 ups。QNAP:控制面板的 UPS 页面,启用网络 UPS 主控,把 DumbMonit 所在主机加入允许的地址;该 UPS 名为 qnapups。TrueNAS:勾选了 Remote Monitor 的 UPS 服务。

4. 可选:DumbMonit 只有在你输入了用户名和密码时才会发送它们,upsd 读取时不要求任何凭据。如果你的策略要求每个客户端都具名,请在 upsd.users 中添加一个没有 actions 或 instcmds 权限的用户,配上一个较长的随机密码,然后重启 upsd。

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. 在 DumbMonit 中,输入 NUT 服务器的地址,例如 "nas.lan"。它公布的每个 UPS 都会被监控;若只想监控其中一部分,请在 UPS 选项中列出它们的名称。

6. DumbMonit 只会读取:它发送 LIST UPS 和 LIST VAR,从不发送 LOGIN、SET、INSTCMD 或 FSD,因此服务器不会把它当作一个关机前要等待的从属客户端。

!!! warning "警告"
    DumbMonit 只监控 UPS,不会关闭任何设备。UPS 供电的机器仍然
    需要自己的 NUT 客户端(upsmon),以便在电池耗尽时正常关机。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 无认证 | 默认情况:`upsd` 允许它接受的任何主机读取 UPS 变量。 |
| upsd 用户名/密码 | 在读取前以 `USERNAME` 和 `PASSWORD` 发送。`upsd` 只在执行命令时才会校验它们,而 DumbMonit 从不发送命令。 |

地址:主机名或 IP(`nas.lan`)、`host:port`,或 `upsc` 的
`ups@host` 形式,其中只保留主机部分。选项:端口(3493)、要
监控的 UPS(默认全部)和每次交互的超时时间(10 秒)。

被拒绝的连接或没有应答都会被计为设备不可达。`ACCESS-DENIED`
会被显示为凭据错误,服务器未公布的 UPS 名称会被显示为配置错误;
二者都不会在半夜吵醒任何人。该连接是纯 TCP 的:不使用
`STARTTLS`,读取的内容也没有任何机密性。
