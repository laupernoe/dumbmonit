# SNMP 设备 {#snmp-device}

最通用的一种类型:几乎所有网络硬件都支持 SNMP。交换机、路由器、
NAS、UPS 和打印机都包含在内。支持 SNMP v1、v2c 和 v3。

## 监控内容 {#what-it-watches}

采集配置档案会根据设备的 `sysObjectID`(对于 UPS,还会参考其
`sysDescr`)自动检测。产品自带七个配置档案;一台设备可以匹配
多个,各配置档案之间也会相互包含。

| 配置档案 | 适用对象 | 指标(前缀 `dumbmonit_`) |
|---|---|---|
| System(SNMPv2-MIB) | 所有设备 | `system_uptime_seconds`、`system_info`(名称、描述、位置、联系人作为标签) |
| 网络接口(IF-MIB) | 所有设备,作为后备 | `if_octets_in/out`、`if_packets_in/out`、`if_errors_in/out`、`if_discards_in/out`、`if_admin_status`、`if_oper_status`、`if_speed_bps`,按每个接口 |
| 主机资源(HOST-RESOURCES-MIB) | Net-SNMP(Linux、BSD、macOS)、Windows SNMP 代理、Synology、QNAP、Solaris | `host_uptime_seconds`、`host_processes`、`host_users`、`cpu_load_percent`、按文件系统的 `storage_bytes_total/used`、`memory_bytes_total/used`、`load_average`(1、5 和 15 分钟,仅 Net-SNMP) |
| UPS(UPS-MIB) | APC、Eaton、MGE、CyberPower、Riello、Socomec、Tripp Lite、Vertiv、Salicru,以及任何公布 UPS-MIB 的代理 | `ups_battery_status`、`ups_battery_seconds_on`、`ups_battery_minutes_remaining`、`ups_battery_charge_percent`、电池电压/电流/温度、输入电压/频率/异常次数、`ups_output_source`、输出电压/频率/功率、`ups_output_load_percent`、`ups_alarms_present` |
| 打印机(PRINTER-MIB) | HP、Kyocera、Epson、Canon、Brother、Lexmark、Ricoh、Xerox、Samsung、OKI、Konica Minolta | 按每个耗材的 `printer_supply_level/max`、按每个纸盒的 `printer_input_level/max`、`printer_pages_printed`、`printer_status` |
| 以太网交换机(EtherLike-MIB、LLDP) | 可手动为任意交换机选择;各厂商交换机配置档案已包含它 | IF-MIB 的全部内容,加上每个端口的 `ether_fcs_errors`、`ether_alignment_errors`、`ether_symbol_errors`、`ether_late_collisions`、`ether_excessive_collisions`、`ether_mac_transmit_errors`、`ether_mac_receive_errors`、`ether_duplex_status`(1 未知,2 半双工,3 全双工)、`if_broadcast_packets_in`、`if_multicast_packets_in`(64 位),以及 `lldp_neighbor_info`(谁连接在哪里:`remote_sysname`、`remote_port`、`remote_port_desc`;本地端口号是 `index` 标签的第二部分) |
| Zyxel 交换机 | 所有 Zyxel 的 `sysObjectID`(`1.3.6.1.4.1.890`) | 以太网交换机的全部内容,加上 `cpu_load_percent`(一分钟平均值)、`zyxel_cpu_5min_percent`、`memory_usage_percent`、`zyxel_firmware_info`(固件和型号作为标签),以及在发布了硬件监控信息的交换机上,每个传感器的 `zyxel_temperature_celsius` 和 `zyxel_temperature_high_threshold_celsius`,以及每个风扇的 `zyxel_fan_rpm` 和 `zyxel_fan_low_threshold_rpm` |

计数器(接口字节数、打印页数)以原始值存储:在查询中使用
`rate()` 来得到吞吐量。设备重启会表现为计数器归零,绝不会显示为
一次虚假的尖峰。

适用于 SNMP 设备的[内置规则](../alerting/rules.md):设备不可达、
CPU 过高、磁盘即将用尽、文件系统即将用尽(预测)、UPS 正在使用
电池、UPS 电池电量低、端口错误累积、端口抖动、CPU 异常(基线)。

### Zyxel 交换机 {#zyxel-switches}

Zyxel 配置档案中标准的部分(接口、以太网错误、LLDP)在所有 Zyxel
交换机上通用。CPU、内存和固件数据来自当前各款可管理交换机共享
的私有分支
`1.3.6.1.4.1.890.1.15`(其中包括 XS1930 和 GS1900);更老的型号
以及 Zyxel 防火墙的布局不同,这些系列在这部分就只会留空。该配置
档案已在真实的 XS1930-10 上验证过;其温度和风扇序列遵循 Zyxel
公布的硬件监控 MIB,但尚未在真实硬件上观察到。

`dot3StatsFrameTooLongs` 被有意地不读取:在 XS1930 上,它会统计
数十亿个完全健康的巨型帧(jumbo frame),而一个在健康网络上也
始终攀升的序列只会教会人们去忽略它。

### 服务器管理控制器(BMC) {#server-management-controllers-bmc}

BMC 自身的 SNMP 代理描述的是 BMC 本身,而不是服务器:它的小型
ARM Linux 系统、它的内存、它的网络链路。例如,Supermicro 的 BMC
会以 Net-SNMP 身份应答,并获得主机资源配置档案,该档案会读取该
代理公布的关于自身的一切信息。服务器的风扇、温度、电源和驱动器
则改为通过[Redfish](redfish.md)读取。

## 需要在设备上准备的内容 {#what-to-prepare-on-the-device}

1. 打开设备的管理界面。
2. 寻找 SNMP 相关设置,通常位于"Network"、"Services" 或
   "Administration" 之下。
3. 启用 SNMP v2c,并记下只读社区名("public" 是许多设备的默认值)。
4. 如果设备按地址过滤,请允许 DumbMonit 服务器的地址。
5. 回到 DumbMonit,输入地址和社区名:其余部分会被自动检测。

!!! warning "警告"
    社区名在网络上不是加密传输的。在共享网络中,优先使用具备
    认证和加密的 SNMP v3。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| SNMP v1 / v2c(社区名) | 只读社区名。 |
| SNMP v3 | 用户名;可选的认证方式(MD5、SHA-1、SHA-224、SHA-256、SHA-384 或 SHA-512 及密码);可选的加密方式(DES、AES-128、AES-192 或 AES-256 及密码);可选的上下文名称。 |

地址:IP 或主机名,例如 `192.168.1.10`。默认端口是 161;若要
使用其他端口,请写成 `host:port`。

## 选项 {#options}

该类型不读取任何选项。自由标签仍然可用,并会作为 `tag_<key>`
复制到每个序列上。

## 常见错误 {#common-errors}

| 现象 | 可能原因 |
|---|---|
| 添加后立刻显示*不可达* | 社区名错误、未启用 SNMP,或设备只接受来自允许地址的请求。设备在社区名错误时从不应答:看起来与设备关机完全一样。 |
| 只有 `system_*` 和 `if_*` 序列 | `sysObjectID` 未匹配任何特定配置档案。IF-MIB 和 System 适用于所有设备;HOST-RESOURCES、UPS、PRINTER 和 Zyxel 需要已知的厂商 OID。对于其他品牌的交换机,可手动选择以太网交换机配置档案。 |
| 没有接口流量数据 | 该设备未公布 64 位的 `ifXTable` 计数器。 |
| 扫描结果为空 | 扫描会对每个地址尝试一个社区名;使用其他社区名或只支持 SNMP v3 的设备不会应答。 |

在设备页面使用**立即探测**:它会报告一次探测产生的样本数量和
序列名称。
