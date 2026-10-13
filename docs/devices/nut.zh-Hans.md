# UPS with NUT {#ups-with-nut}

通过 USB 连接到 NAS、树莓派或服务器的 UPS 通常由网络 UPS 工具 (NUT) 监控：其守护进程 `upsd` 在 TCP 端口 3493 上发布 UPS 报告的内容。DumbMonit 像任何 NUT 客户端一样读取该服务器：UPS 是否在市电还是电池上运行，电池是否低电量或老化，电量、剩余运行时间、负载、输入电压和电池年龄。一个服务器通常发布多个 UPS；所有这些都在一个连接中读取，每个系列都携带其 UPS 的名称。

对于具有自己网络卡的 UPS，[SNMP 设备](snmp.md)类型改为读取标准 UPS-MIB。

## 它监控什么 {#what-it-watches}

所有指标前缀为 `dumbmonit_nut_` 并携带标签 `ups`，即 `ups.conf` 中的 UPS 名称（Synology NAS 上的 `ups`，QNAP 上的 `qnapups`）。UPS 不发布的变量不产生系列。

| 指标 | 说明 | 标签 |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | `ups.status` 的标志（`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`）：设置时为 1，否则为 0，在每次读取时全部 | `ups` |
| `ups_status_info` | 值为 1，原始 `ups.status` | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | 电量，以及 UPS 报告低电池时的电量 | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | 以现有负载在电池上的剩余运行时间，以及低电池运行时间 | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | 自 `battery.date`（安装或更换）以来的时间，或else `battery.mfr.date` | `ups`, `source`（`installed`, `manufactured`） |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | 市电侧；停电时为 0 V | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | 负载侧 | `ups` |
| `ups_load_percent` | 负载，以 UPS 容量的百分比表示 | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | 消耗的功率和额定值，当 UPS 报告时 | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 当最后一个自测（`ups.test.result`）报告错误或警告时为 1，通过时为 0；无测试运行时不存在 | `ups`, `result` |
| `ups_info` | 值为 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 当 `upsd` 回答但从 UPS（`DATA-STALE`, `DRIVER-NOT-CONNECTED`）没有新鲜数据时为 1 | `ups` |
| `ups_driver_connected` | 当该 UPS 的 NUT 驱动程序未运行时为 0 | `ups` |
| `server_ups` | 服务器发布的 UPS 数量 | |
| `server_info` | 值为 1 | `version` |

序列号从不放在标签中。每个服务器最多读取 32 个 UPS。

适用的[内置规则](../alerting/rules.md#ups-behind-nut)：UPS 在电池上、UPS 电池低、UPS 电池需要更换、UPS 负载高、UPS 运行时短、UPS 数据陈旧、加上当 NUT 服务器本身停止回答时设备不可达。

## 设备页面 {#the-device-page}

面板上方读取探针存储的内容；打开页面从不连接到 NUT 服务器。每个 UPS 一张卡：其状态（文字形式）（市电、电池、低电池、更换电池、旁路、过载…），电量、剩余运行时间、负载、输入电压和电池年龄。

## 让 DumbMonit 读取 NUT 服务器 {#let-dumbmonit-read-the-nut-server}

1. 找到 NUT 服务器：UPS 插入的机器（NAS、树莓派、服务器）在 TCP 端口 3493 上运行 upsd。从 DumbMonit 主机，检查它是否回答并注意其 UPS 的名称。

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    安装了 NUT 客户端工具，`upsc -l nas.lan` 给出相同的列表。

2. 在纯 NUT 安装上，让 upsd 侦听 DumbMonit 主机可以到达的地址：将此行添加到 upsd.conf，为 DumbMonit 主机打开防火墙中的 TCP 端口 3493，然后重启 upsd。读取不需要账户：upsd 让任何它接受的主机读取变量，没有有权利的用户无法更改任何东西。

    ```
    LISTEN 0.0.0.0 3493
    ```

3. 在 NAS 上，相同的开关有另一个名称。Synology DSM：控制面板 → 硬件和电源 → UPS，勾选启用网络 UPS 服务器，并在允许的 DiskStation 设备中添加 DumbMonit 主机；UPS 名为 ups。QNAP：控制面板的 UPS 页面，启用网络 UPS 主控，DumbMonit 主机在允许的地址中；UPS 名为 qnapups。TrueNAS：启用了远程监视的 UPS 服务。

4. 可选：DumbMonit 仅在输入用户名和密码时发送，upsd 不要求任何读取。如果您的政策需要每个客户端都被命名，向 upsd.users 添加没有操作或 instcmds 的用户，使用长随机密码，然后重启 upsd。

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. 在 DumbMonit 中，输入 NUT 服务器的地址，例如"nas.lan"。它发布的每个 UPS 都被监控；要仅监控某些，请在 UPS 选项中列出它们的名称。

6. DumbMonit 仅读取：它发送 LIST UPS 和 LIST VAR，从不发送 LOGIN、SET、INSTCMD 或 FSD，因此服务器从不将其计为关闭前要等待的副本。

!!! warning
    DumbMonit 监控 UPS；它不关闭任何东西。UPS 供电的机器仍需要自己的 NUT 客户端（upsmon）在电池电量耗尽时干净地关闭。

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| 无身份验证 | 默认：`upsd` 让任何它接受的主机读取 UPS 变量。 |
| upsd 用户名 / 密码 | 在读取前用 `USERNAME` 和 `PASSWORD` 发送。`upsd` 仅对命令检查，DumbMonit 从不发送。 |

地址：主机名或 IP（`nas.lan`），`host:port`，或 `upsc` 的 `ups@host` 形式，仅保留主机。选项：端口（3493），要监控的 UPS（默认所有）和每次交换的超时（10 秒）。

拒绝连接或无回答将计为设备不可达。当运行 NUT 服务器的机器在其报告的停电中关闭时（通常是其自己的 upsmon 主控），UPS 警报在设备不可达同样触发时被隐藏为受压制：将设备不可达警报读为电源故障赢得的标志。在一台比其他机器存活时间更长的机器上运行 NUT 服务器，或从第二个 NUT 服务器监控 UPS。`ACCESS-DENIED` 显示为凭证错误，服务器不发布的 UPS 名称显示为配置错误；两者都不会让任何人整夜不眠。连接是纯 TCP：不使用 `STARTTLS`，读取的内容都不是秘密。
