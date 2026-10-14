# 规则 {#rules}

规则在首次启动时插入,然后属于你:DumbMonit 永远不会改写
你改变的规则,它只是重新创建缺少的。内置规则可以
被编辑和禁用,不删除。

## 内置规则 {#built-in-rules}

严重级别用 UI 词显示;API 值在括号中。一个规则
其度量没有设备产生没有系列并永不触发,所以每个规则
下面在每个实例上默认启用。

### 每个设备 {#every-device}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 设备不可达 | 三分钟内没有收到测量。`time() - tlast_over_time(dumbmonit_up[7d])` | > 180 s | 1 分钟 | 警告 (`critical`) | 6 小时 |
| CPU 高 | CPU 负载维持在 90% 以上,跨 SNMP、Proxmox 和代理源平均每个设备;在 85% 下清除。 | > 90 % | 10 分钟 | 通知 (`warning`),1 小时后升级 | 6 小时 |
| 异常 CPU | CPU 负载明显不同于通常在这个时间和星期几(季节基线)。 | 分数 > 3.5 | 15 分钟 | 信息 (`info`) | 6 小时 |
| 磁盘即将满 | 文件系统 90% 或更多,每个挂载点(SNMP、Proxmox 存储和根文件系统);在 88% 下清除。 | ≥ 90 % | 15 分钟 | 通知 (`warning`),24 小时后升级 | 6 小时 |
| 文件系统即将满 | 按此速率,文件系统将在四天内满(`predict_linear`超过 6 小时;上升的文件系统仅)。 | ≥ 100 % | 30 分钟 | 通知 (`warning`) | 6 小时 |

### UPS {#ups}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| UPS 在电池上 | UPS 从电池为负载供电(`dumbmonit_ups_output_source == 5`)。 | > 0 | 30 秒 | 警告 (`critical`) | 15 分钟 |
| UPS 电池低 | UPS 电池低或耗尽(`dumbmonit_ups_battery_status`,3 = 低,4 = 耗尽)。 | ≥ 3 | 1 分钟 | 警告 (`critical`) | 30 分钟 |

### 网络端口(SNMP) {#network-ports-snmp}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 端口累积错误 | 网络端口在最后一小时内记录了超过 50 个错误:通常是故障电缆、脏纤维或双工不匹配(`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`;在新设备的第一小时内没有触发,所以在添加前计数的错误不被报告为新)。 | > 50 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| 端口飘动 | 网络端口在 30 分钟内关闭和打开超过两次(`changes_prometheus(dumbmonit_if_oper_status[30m])`)。故意禁用的端口没有系列并永不触发。 | > 4 | 5 分钟 | 通知 (`warning`) | 1 小时 |

### 服务器硬件(Redfish) {#server-hardware-redfish}

健康系列是 0(正常)、1(警告)或 2(临界)。一个空插槽(`Absent`)或禁用的有无系列,所以一个交付的服务器只有两个电源中的一个永不触发。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 服务器风扇失败 | 管理控制器报告一个风扇已失败(`dumbmonit_redfish_fan_health`)。 | ≥ 2 | 2 分钟 | 警告 (`critical`) | 6 小时 |
| 服务器温度超过临界 | 温度传感器超过控制器本身声明的临界阈值(`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| 电源冗余丧失 | 电源不再冗余:再一次失败服务器关闭(`dumbmonit_redfish_power_redundancy_health`)。 | ≥ 1 | 5 分钟 | 通知 (`warning`),1 小时后升级 | 6 小时 |
| 电源失败 | 控制器报告电源已失败(`dumbmonit_redfish_psu_health`)。 | ≥ 2 | 2 分钟 | 警告 (`critical`) | 6 小时 |
| 驱动器故障预测 | 驱动器报告预测的故障:在它仍然工作时更换它(`dumbmonit_redfish_drive_failure_predicted`)。 | > 0 | 10 分钟 | 通知 (`warning`) | 24 小时 |
| 服务器健康临界 | 控制器报告系统自己的健康为临界(`dumbmonit_redfish_system_health`)。 | ≥ 2 | 2 分钟 | 警告 (`critical`) | 6 小时 |

### 正常运行时间监控和心跳 {#uptime-monitors-and-heartbeats}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 服务关闭 | 服务在三分钟内没有正确响应(`dumbmonit_probe_success{probe!="push"} == bool 0`)。 | > 0 | 3 分钟 | 警告 (`critical`) | 30 分钟 |
| 心跳错过 | 工作在其预期间隔加宽限期内没有调用,或报告了故障本身(`dumbmonit_probe_success{probe="push"} == bool 0`;延迟按设备设置,见[心跳](../devices/push.md))。 | > 0 | 2 分钟 | 通知 (`warning`) | 6 小时 |
| 服务飘动 | 服务在 30 分钟内改变状态超过 6 次(`changes(dumbmonit_probe_success{probe!="push"}[30m])`)。 | > 6 | 5 分钟 | 通知 (`warning`) | 1 小时 |
| 缓慢服务 | 服务需要超过三秒来响应;失败的检查不计入(`dumbmonit_probe_duration_seconds and dumbmonit_probe_success == 1`)。 | > 3 秒 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| 证书即将过期 | 证书在少于 14 天内过期。 | < 14 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| 证书已过期 | 证书已过期。 | < 0 天 | 5 分钟 | 警告 (`critical`) | 24 小时 |

### Docker 容器 {#docker-containers}

由代理报告;见[代理](../devices/agent.md)。告警命名
容器。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 容器停止 | 容器已停止两分钟(`dumbmonit_container_up == bool 0`)。Docker 不会重启的容器不计入(`dumbmonit_container_stop_expected = 1`):已结束的一次性 `docker run`、从未启动过的容器,或没有重启策略而被手动停止的容器。崩溃(任何其他退出码)、内存不足被终止,或 `always` / `unless-stopped` 容器仍会计入。 | > 0 | 2 分钟 | 通知 (`warning`) | 1 小时 |
| 容器不健康 | 容器的健康检查已失败三分钟(`dumbmonit_container_health == 2`;0 无,1 健康,3 启动中)。已停止的容器不计入:Docker 保留其最后的结论,而它已不再有意义。 | > 0 | 3 分钟 | 通知 (`warning`) | 1 小时 |
| 容器重启中 | 容器在 15 分钟内重启 3 次或更多(`increase(dumbmonit_container_restart_count[15m])`):`它很少停止足够长的时间让前一个规则通过。 | ≥ 3 | 1 分钟 | 通知 (`warning`) | 1 小时 |
| 容器更新可用 | 此容器的注册表中有更新的镜像(`dumbmonit_container_update_available == 1`)。 | > 0 | 1 小时 | 信息 (`info`) | 24 小时 |

### Plakar 备份 {#plakar-backups}

由代理报告,按 kloset 和源。没有这些系列存在
在 Plakar 不被检测时,所以没有规则可以在一个没有它的机器上触发。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Plakar 备份太旧 | 此源没有新的 Plakar 快照超过两天(`dumbmonit_backup_last_success_seconds`)。 | > 2 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| Plakar 备份失败 | 最新的快照有错误(`dumbmonit_backup_last_status{source!="*"} < 1`)。 | < 1 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| Plakar kloset 无法读取 | 代理无法打开 kloset:存储不可达、缺少口令或插件;代理的日志会说明是哪一种,仅一次(`dumbmonit_backup_last_status{source="*"} < 1`)。 | < 1 | 30 分钟 | 通知 (`warning`) | 24 小时 |

### restic 和 Borg 备份 {#restic-and-borg-backups}

由代理为在 agent.yaml 中`restic_repos`或
`borg_repos`下声明的每个存储库报告;见[代理](../devices/agent.md#restic-and-borg-backups)。
没有声明存储库就没有系列,所以没有规则可以触发。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| restic/Borg 备份太旧 | 此存储库中没有新的 restic 或 Borg 快照超过 48 小时(`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`)。 | > 48 小时 | 1 小时 | 通知 (`warning`) | 24 小时 |
| restic/Borg 存储库不可达 | 代理无法读取存储库:不可达、密码拒绝或二进制缺失;代理的日志说哪个(`dumbmonit_agent_backup_repo_reachable < 1`)。 | < 1 | 30 分钟 | 通知 (`warning`) | 24 小时 |

### WireGuard 隧道 {#wireguard-tunnels}

由代理报告;见[代理](../devices/agent.md#wireguard-tunnels)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| WireGuard 对等点沉默 | 持久保活的对等点在 15 分钟内未完成握手:隧道关闭(`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`)。从代理首次看到它的时刻计算从未连接过的对等点的沉默。没有保活的对等点(电话、笔记本电脑)闲置数天并被遗漏;改变阈值改变 N,或丢弃`and on …`部分监视每个对等点。 | > 15 分钟 | 5 分钟 | 通知 (`warning`) | 6 小时 |

### 代理磁盘、ZFS 和传感器 {#agent-disks-zfs-and-sensors}

由代理报告;见[代理](../devices/agent.md)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 磁盘 SMART 失败 | 磁盘自己的 SMART 自我评估报告其已失败(`dumbmonit_agent_disk_smart_ok == bool 0`)。更换它并检查备份。 | > 0 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| ZFS 池降级 | ZFS 池不再健康:设备缺失、故障或移除(`dumbmonit_agent_zfs_pool_health`:0 在线,1 降级,2 其他)。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| ZFS 清理发现错误 | 最后的 ZFS 清理在此池上发现了错误,或文件永久丢失(`dumbmonit_agent_zfs_pool_scrub_errors or dumbmonit_agent_zfs_pool_data_errors`)。池可以保持`ONLINE`。 | > 0 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| 温度超过临界 | 传感器超过其自己的硬件声明的临界温度(`dumbmonit_agent_sensor_temperature_celsius >= dumbmonit_agent_sensor_temperature_critical_celsius`);无固定值。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |

### Proxmox VE {#proxmox-ve}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 备份太旧 | 没有成功的 Proxmox VE 备份超过 7 天(`dumbmonit_proxmox_backup_last_age_seconds`)。 | > 7 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| VM 或容器停止 | 客人在最后两小时内运行并停止了 5 分钟(`1 - dumbmonit_proxmox_guest_running`,并'和`max_over_time(…[2h]) == 1`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| VM 或容器 CPU 高 | 客人已使用超过 90% 其分配的内核 15 分钟(`dumbmonit_proxmox_guest_cpu_percent`)。通知命名客人,`nextcloud (202)`。 | > 90 % | 15 分钟 | 通知 (`warning`) | 6 小时 |
| VM 或容器内存高 | 客人已使用超过 95% 其内存 10 分钟(`dumbmonit_proxmox_guest_memory_percent`)。 | > 95 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| VM 或容器磁盘即将满 | 客人的根磁盘超过 90% 满(`dumbmonit_proxmox_guest_disk_used_percent`):容器总是,VM 仅当 QEMU 客人代理报告使用时。 | > 90 % | 15 分钟 | 通知 (`warning`) | 24 小时 |
| 客人锁定 | VM 或容器已锁定 6 小时(`dumbmonit_proxmox_guest_locked`,标签`lock`):中断的备份或迁移留下了锁,没有操作可以在客人上运行,甚至不是下一个备份。 | > 0 | 6 小时 | 通知 (`warning`) | 24 小时 |
| 旧快照 | 客人最旧的快照超过 30 天(`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`)。 | > 30 天 | 1 小时 | 信息 (`info`) | 7 天 |
| HA 资源出错 | Proxmox HA 资源处于错误或隔离(`dumbmonit_proxmox_ha_resource_error`)。 | > 0 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| HA 管理器不报告 | 一个节点的 HA 本地资源管理器已停止写入其状态(`dumbmonit_proxmox_ha_lrm_stale`):该节点的 HA 客人将既不重启也不重新定位。 | > 0 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| 集群失去仲裁 | Proxmox 集群不是仲裁者(`1 - dumbmonit_proxmox_cluster_quorate`):客人无法再启动。 | > 0 | 1 分钟 | 警告 (`critical`) | 30 分钟 |
| Proxmox 节点离线 | 集群节点被报告离线(`1 - dumbmonit_proxmox_node_up`)。 | > 0 | 2 分钟 | 警告 (`critical`) | 30 分钟 |
| Proxmox 服务关闭 | 一个核心 Proxmox 守护进程在节点上未运行(`dumbmonit_proxmox_node_core_services_down`:`pve-cluster`、`pvedaemon`、`pveproxy`、`pvestatd`、`pve-firewall`、`corosync`、`watchdog-mux`,每个仅在节点启用时计数)。 | > 0 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| 节点网络接口关闭 | 设置为在启动时启动的接口在节点上未启动(`dumbmonit_proxmox_node_interface_offline`)。按需启动的接口永远不被报告。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Proxmox 存储即将满 | Proxmox 存储超过 85% 满(`dumbmonit_proxmox_storage_used_percent`);在 90% 时"磁盘即将满"接管。 | > 85 % | 15 分钟 | 通知 (`warning`) | 24 小时 |
| 精简池即将满 | LVM 精简池超过 90% 满:每个存储在其上的客人当其填满时变为只读(`dumbmonit_proxmox_node_thinpool_used_percent`);在 88% 下清除。 | > 90 % | 15 分钟 | 警告 (`critical`) | 6 小时 |
| 精简池元数据即将满 | LVM 精简池的元数据体积超过 80% 满(`dumbmonit_proxmox_node_thinpool_metadata_used_percent`)。它远小于数据体积并且经常首先饱和,具有相同的后果。 | > 80 % | 15 分钟 | 警告 (`critical`) | 6 小时 |
| Proxmox 磁盘磨损 | 节点的 SSD 或 NVMe 已使用超过 90% 其额定寿命(`dumbmonit_proxmox_node_disk_wearout_percent`)。通知读`pve1 · /dev/nvme0n1`。 | > 90 % | 1 小时 | 通知 (`warning`) | 7 天 |
| Proxmox 磁盘 SMART 故障 | 节点的磁盘报告 SMART 健康`FAILED`(`dumbmonit_proxmox_node_disk_smart_failed`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 24 小时 |
| Proxmox ZFS 池降级 | 节点的 ZFS 池不是`ONLINE`(`dumbmonit_proxmox_node_zfs_pool_degraded`)。通知读`pve2 · tank`。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| 备份工作失败 | 节点上最后的 vzdump 工作,或计划备份工作,失败(`1 - dumbmonit_proxmox_backup_job_last_ok`)。 | > 0 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| 备份工作排除磁盘 | 计划备份工作跳过它覆盖的客人之一的磁盘(`dumbmonit_proxmox_backup_job_guest_excluded_volumes`):工作每晚成功,该数据丢失。CD-ROM 驱动器和 cloudinit 镜像按设计被排除并永不计数。 | > 0 | 1 小时 | 通知 (`warning`) | 7 天 |
| 复制失败 | 复制工作报告错误或非零失败计数(`dumbmonit_proxmox_replication_job_error`):备用副本已过期。 | > 0 | 10 分钟 | 警告 (`critical`) | 6 小时 |
| Ceph 健康错误 | Ceph 报告`HEALTH_ERR`(`dumbmonit_proxmox_ceph_health`,0 OK,1 WARN,2 ERR)。 | ≥ 2 | 2 分钟 | 警告 (`critical`) | 30 分钟 |
| Ceph 健康警告 | Ceph 报告`HEALTH_WARN`超过 15 分钟(`dumbmonit_proxmox_ceph_health == 1`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Ceph OSD 关闭 | Ceph OSD 已关闭 5 分钟(`1 - dumbmonit_proxmox_ceph_osd_up`)。通知命名 OSD 和其主机。 | > 0 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| Ceph OSD 几乎满 | 单个 Ceph OSD 超过 85% 满(`dumbmonit_proxmox_ceph_osd_used_percent`):当它达到其限制时写入停止,无论集群平均说什么;在 82% 下清除。 | > 85 % | 15 分钟 | 通知 (`warning`) | 24 小时 |
| Ceph 池几乎满 | Ceph 池超过 85% 满(`dumbmonit_proxmox_ceph_pool_used_percent`);在 82% 下清除。 | > 85 % | 15 分钟 | 通知 (`warning`) | 24 小时 |
| Ceph noout 标志左开 | Ceph`noout`标志已设置 2 小时(`dumbmonit_proxmox_ceph_flag{flag="noout"}`):重新平衡被禁用,集群读起来健康而其冗余收缩。 | > 0 | 2 小时 | 信息 (`info`) | 24 小时 |
| Proxmox 更新待处理 | 超过 20 个包更新在节点上待处理(`dumbmonit_proxmox_node_updates_pending`)。 | > 20 | 1 小时 | 信息 (`info`) | 7 天 |
| Proxmox 安全更新待处理 | 至少一个待处理更新来自安全档案(`dumbmonit_proxmox_node_updates_security_pending`)。 | > 0 | 1 小时 | 通知 (`warning`) | 7 天 |
| Proxmox 包已更改 | Proxmox 包的已安装版本在两个探针之间更改 — 有人运行了`apt upgrade`(`dumbmonit_proxmox_node_packages_changed`,发布一小时以其变化细节在其`changes`标签:`pve1 · pve-manager 8.2.4→8.2.7`)。一小时后自行解决。 | > 0 | 1 分钟 | 信息 (`info`) | 24 小时 |
| 节点证书即将过期 | Proxmox 节点证书在少于 14 天内过期(`dumbmonit_proxmox_node_certificate_expiry_days`)。 | < 14 天 | 1 小时 | 通知 (`warning`) | 24 小时 |

### Proxmox 备份服务器 {#proxmox-backup-server}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| PBS 数据存储几乎满 | PBS 数据存储超过 90% 满。 | > 90 % | 15 分钟 | 通知 (`warning`),24 小时后升级 | 6 小时 |
| PBS 数据存储填满中 | 按当前速率,PBS 估计数据存储在 7 天内满。 | < 7 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| PBS 数据存储未挂载 | 可移动数据存储未挂载(`dumbmonit_pbs_datastore_removable_unmounted`):没什么可写入它。 | > 0 | 6 小时 | 通知 (`warning`) | 24 小时 |
| PBS 备份太旧 | 此备份组(每台机器一个)没有新快照超过 2 天(`dumbmonit_pbs_backup_last_age_seconds`)。编辑每周备份机器的阈值;通知命名组和数据存储。 | > 2 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| PBS 备份验证失败 | 此机器最新快照的验证失败(`dumbmonit_pbs_backup_last_verified < 1`)。 | < 1 | 30 分钟 | 警告 (`critical`) | 24 小时 |
| PBS 任务失败 | 至少一个 PBS 任务在审查窗口内失败(`dumbmonit_pbs_tasks_failed`)。 | > 0 | 10 分钟 | 通知 (`warning`) | 24 小时 |
| PBS 垃圾回收太旧 | 此数据存储上没有成功的垃圾回收超过 8 天(`dumbmonit_pbs_gc_last_success_age_seconds`)。 | > 8 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| PBS 垃圾回收失败 | 数据存储上最后的垃圾回收失败(`dumbmonit_pbs_gc_last_run_ok < 1`;从 PBS 3.3+ 上的`/gc`,从之前最后的 GC 任务)。 | < 1 | 10 分钟 | 通知 (`warning`) | 24 小时 |
| PBS 发现损坏块 | 垃圾回收在数据存储上发现了不可读的块(`dumbmonit_pbs_gc_bad_chunks`):某些备份无法再恢复。 | > 0 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| PBS 修剪工作失败 | PBS 修剪工作最后一次运行失败:什么都不修剪数据存储继续填满(`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`)。通知命名工作和数据存储。 | < 1 | 10 分钟 | 通知 (`warning`) | 24 小时 |
| PBS 验证工作失败 | PBS 验证工作最后一次运行失败(`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`)。 | < 1 | 10 分钟 | 通知 (`warning`) | 24 小时 |
| PBS 同步工作失败 | PBS 同步工作最后一次运行失败(`dumbmonit_pbs_sync_job_last_ok < 1`)。 | < 1 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| PBS 磁带备份失败 | 最后一次磁带备份运行失败(`dumbmonit_pbs_tape_backup_job_last_ok < 1`):不正在制作离线副本。 | < 1 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| PBS 工作永不运行 | 工作被启用并计划但永不运行(`dumbmonit_pbs_job_never_run`)。 | > 0 | 2 天 | 通知 (`warning`) | 7 天 |
| PBS 服务关闭 | Proxmox 备份服务器需要的服务未运行(`dumbmonit_pbs_node_service_active{expected="1"} < 1`);当它关闭时无法备份。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| PBS 磁盘 SMART 故障 | 备份服务器的磁盘报告 SMART 健康`FAILED`(`dumbmonit_pbs_node_disk_smart_failed`)。通知读`/dev/sdb`。 | > 0 | 5 分钟 | 警告 (`critical`) | 24 小时 |
| PBS SSD 磨损 | 备份服务器的 SSD 已使用超过 90% 其额定耐久性(`dumbmonit_pbs_node_disk_wearout_percent`)。 | > 90 % | 1 小时 | 通知 (`warning`) | 7 天 |
| PBS ZFS 池降级 | 备份服务器的 ZFS 池不是`ONLINE`(`dumbmonit_pbs_node_zfs_pool_degraded`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| PBS 证书即将过期 | 备份服务器提供的证书在少于 3 周内过期(`dumbmonit_pbs_node_certificate_expires_seconds`)。需要`certificates`选项。 | < 21 天 | 1 小时 | 通知 (`warning`) | 48 小时 |
| PBS 升级后重启待处理 | 包已升级但运行守护进程仍是旧的(`dumbmonit_pbs_node_running_version_stale`)。 | > 0 | 6 小时 | 信息 (`info`) | 7 天 |
| PBS 更新待处理 | 超过 20 个包更新在备份服务器上待处理(`dumbmonit_pbs_node_updates_pending`)。 | > 20 | 1 小时 | 信息 (`info`) | 7 天 |

### Proxmox 数据中心管理器 {#proxmox-datacenter-manager}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 联合实例不可达 | 数据中心控制台无法到达它联合的 Proxmox VE 集群或备份服务器之一(`dumbmonit_pdm_remote_reachable < 1`)。通知命名实例和其产品。 | < 1 | 10 分钟 | 警告 (`critical`) | 6 小时 |
| 联合实例落后 | 实例运行比同一产品在同一地产中另一个实例更旧的版本(`dumbmonit_pdm_remote_version_behind`)。实例之间比较,从不对 Proxmox 发布的内容。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| 联合实例上的任务失败 | 至少一个任务在审查窗口内在联合实例上失败(`dumbmonit_pdm_remote_tasks_failed`)。 | > 0 | 10 分钟 | 通知 (`warning`) | 24 小时 |
| 数据中心控制台磁盘即将满 | 数据中心控制台的根文件系统超过 90% 满(`dumbmonit_pdm_node_rootfs_percent`):满,控制台停止记录它收集的。 | > 90 % | 15 分钟 | 通知 (`warning`),24 小时后升级 | 6 小时 |
| 数据中心控制台证书即将过期 | 数据中心控制台的证书在少于 14 天内过期(`dumbmonit_pdm_node_certificate_expiry_days`)。 | < 14 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| 数据中心控制台更新待处理 | 超过 20 个包更新在数据中心控制台上待处理(`dumbmonit_pdm_node_updates_pending`)。 | > 20 | 1 小时 | 信息 (`info`) | 7 天 |

### Proxmox 邮件网关 {#proxmox-mail-gateway}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 邮件队列增长 | 延迟邮件队列在两小时内增长超过一百条消息(`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`):下一跳可能拒绝邮件。增长而不是大小,因为忙网关总是保持几十个延迟。 | > 100 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| 邮件在队列中卡住 | 消息在队列中等待超过 4 小时(`dumbmonit_pmg_queue_oldest_age_seconds`)。年龄是下界 — qshape 报告括号 — 所以规则迟到而不是早。通知命名队列。 | > 4 小时 | 15 分钟 | 警告 (`critical`) | 6 小时 |
| 邮件网关服务停止 | 网关接受和过滤邮件所需的服务已停止(`dumbmonit_pmg_service_running`用于 postfix、pmg-smtp-filter、pmgpolicy、pmgproxy、pmgdaemon)。未安装的单位没有系列无法触发。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| 病毒签名过期 | ClamAV 每日病毒签名超过 2 天旧(`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`):freshclam 已停止更新它们网关继续用上周的列表过滤。 | > 2 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| 垃圾邮件规则过期 | SpamAssassin 规则超过 8 天旧(`dumbmonit_pmg_signature_age_seconds{family="spam"}`)。从未更新的通道没有日期无系列。 | > 8 天 | 1 小时 | 通知 (`warning`) | 7 天 |
| 隔离区填满中 | 垃圾隔离区在 6 小时内接收了超过一千条消息(`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`):一个活动,或一个规则刚刚将合法域放在垃圾箱。 | > 1000 | 30 分钟 | 通知 (`warning`) | 12 小时 |
| 邮件网关集群降级 | 集群的网关不再与其他网关同步(`dumbmonit_pmg_cluster_node_insync`):它用过期规则数据库过滤。独立网关没有系列。 | < 1 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| 邮件网关证书即将过期 | 邮件网关的证书在少于 14 天内过期(`dumbmonit_pmg_certificate_expires_in_seconds`)。 | < 14 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| 邮件网关更新待处理 | 超过 20 个包更新在邮件网关上待处理(`dumbmonit_pmg_node_updates_pending`)。 | > 20 | 1 小时 | 信息 (`info`) | 7 天 |

### MDaemon 和 SecurityGateway {#mdaemon-and-securitygateway}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| MDaemon 邮件服务关闭 | MDaemon 服务器的观察邮件服务不应答,或问候以拒绝服务(`dumbmonit_mdaemon_service_up`)。仅设备上列出的服务有系列。通知命名服务和其端口。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| MDaemon XML API 未应答 | XML API 已停止应答,而邮件服务仍做(`dumbmonit_mdaemon_api_up`):远程管理可能已停止。仅有账户的设备有系列。 | < 1 | 15 分钟 | 通知 (`warning`) | 12 小时 |
| MDaemon 邮件队列增长 | 入站、本地或远程队列在两小时内增长超过一百条消息(`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`):MDaemon 已停止从其交付。由[代理](../devices/agent.md#windows-performance-counters)在 MDaemon 服务器上读取,所以告警在代理的设备上。 | > 100 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| MDaemon Bad 队列不为空 | 消息在 Bad 队列中等待(`dumbmonit_mdaemon_queue_messages{queue="bad"}`):MDaemon 既不能交付也不能退回它们,它们留在那里直到管理员释放或删除它们。 | > 0 | 30 分钟 | 通知 (`warning`) | 24 小时 |
| MDaemon Retry 队列高 | 超过 50 条消息在 Retry 队列中等待(`dumbmonit_mdaemon_queue_messages{queue="retry"}`):远程服务器继续暂时拒绝邮件。 | > 50 | 1 小时 | 通知 (`warning`) | 6 小时 |
| SecurityGateway 服务关闭 | 网关的观察服务不应答,或问候以拒绝服务(`dumbmonit_securitygateway_service_up`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| SecurityGateway API 未应答 | REST API 已停止应答,而服务仍做(`dumbmonit_securitygateway_api_up`)。仅有 API 密钥的设备有系列。 | < 1 | 15 分钟 | 通知 (`warning`) | 12 小时 |
| SecurityGateway 交付队列增长 | 交付队列在两小时内增长超过一百条消息(`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`):网关后的邮件服务器可能拒绝邮件。计数器名称来自 API 如它们是;如果你的版本命名队列不同,调整匹配器。 | > 100 | 15 分钟 | 通知 (`warning`) | 6 小时 |

### OPNsense {#opnsense}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 互联网网关关闭 | dpinger 不再从网关获得应答(`dumbmonit_opnsense_gateway_up`)。在多 WAN 防火墙上流量已经静默移动到另一条链接。`loss`和`delay`是警告,不是故障,不触发此规则。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| 网关丢失数据包 | 网关丢失超过五分之一发送给它的数据包(`dumbmonit_opnsense_gateway_loss_percent`):链接应答,坏。在 10% 下清除。 | > 20 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| 网关缓慢 | 网关的往返时间保持在 300 ms 上方(`dumbmonit_opnsense_gateway_delay_seconds`)。3 百毫秒留下移动备用链接单独。 | > 300 毫秒 | 15 分钟 | 信息 (`info`) | 12 小时 |
| 防火墙状态表填满 | pf 跟踪超过 80% 它被允许的连接(`dumbmonit_opnsense_pf_states_used_percent`)。超过限制防火墙丢弃新连接没有其他症状。 | > 80 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| 防火墙网络缓冲区耗尽 | 超过 90% FreeBSD 网络缓冲区在使用中(`dumbmonit_opnsense_mbuf_used_percent`)。耗尽缓冲区的防火墙停止转发 CPU 仍空闲。 | > 90 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| VPN 隧道关闭 | 配置 VPN 隧道已无会话 10 分钟(`dumbmonit_opnsense_vpn_tunnel_up`)。未安装的插件或禁用的连接没有系列无法触发。 | < 1 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| 防火墙解析器停止 | Unbound 已停止应答(`dumbmonit_opnsense_unbound_running`)。防火墙仍路由;其后的机器不再解决。如果你的防火墙用别的解决关闭此规则。 | < 1 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| 防火墙核心服务停止 | dpinger,它观察网关,或 configd,它运行一切,已停止(`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`)。仅那两个,因为故意关闭的服务不是故障。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| 防火墙太热 | 传感器读取 85°C 以上(`dumbmonit_opnsense_temperature_celsius`):壁橱中的无风扇盒子要开始节流。在 78°C 下清除。 | > 85°C | 10 分钟 | 通知 (`warning`) | 6 小时 |
| 防火墙留在 CARP 维护模式 | CARP 已留在持久维护模式(`dumbmonit_opnsense_carp_maintenance_mode`):此防火墙将其虚拟地址移交给其合作伙伴不会独自取回。 | > 0 | 1 小时 | 信息 (`info`) | 24 小时 |
| 防火墙重启待处理 | 更新已安装但需要重启才能生效(`dumbmonit_opnsense_firmware_reboot_required`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| 防火墙更新待处理 | 防火墙有包等待更新(`dumbmonit_opnsense_firmware_updates_pending`)。它是网络上最暴露的机器。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |

### TrueNAS {#truenas}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| NAS 池降级 | ZFS 不再称池为健康(`dumbmonit_truenas_pool_healthy`)。降级池仍然服务其数据,这正是为什么没有人注意直到下一个磁盘失败。设备页面命名磁盘。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| NAS 磁盘错误 | 池的磁盘已报告读、写或校验和错误因最后的`zpool clear`(`dumbmonit_truenas_pool_device_errors`,每个类型一个系列)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| NAS 池即将满 | 池超过 85% 满(`dumbmonit_truenas_pool_used_percent`)。ZFS 在满前减慢。在 80% 下清除。 | > 85 % | 30 分钟 | 通知 (`warning`),7 天后升级 | 24 小时 |
| NAS 清理发现错误 | 池的最后清理发现了损坏数据(`dumbmonit_truenas_pool_last_scrub_errors`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 24 小时 |
| NAS 清理逾期 | 池已 45 天未完成清理(`dumbmonit_truenas_pool_last_scrub_age_seconds`)。恢复后缺席,它删除最后清理的记录。 | > 45 天 | 1 小时 | 信息 (`info`) | 7 天 |
| NAS 磁盘 SMART 测试失败 | SMART 自我测试在磁盘日志中失败(`dumbmonit_truenas_disk_smart_failed`)。从未测试的磁盘没有系列。 | > 0 | 5 分钟 | 警告 (`critical`) | 24 小时 |
| NAS 磁盘太热 | 磁盘在 55°C 以上(`dumbmonit_truenas_disk_temperature_celsius`,从 TrueNAS 缓存读取)。在 50°C 下清除。 | > 55°C | 15 分钟 | 通知 (`warning`) | 6 小时 |
| NAS 数据集接近其配额 | 数据集已使用超过 90% 其配额(`dumbmonit_truenas_dataset_quota_used_percent`):在 100% 其写入失败而池仍有空间。无配额,无系列。 | > 90 % | 30 分钟 | 通知 (`warning`) | 24 小时 |
| NAS 复制失败 | 启用的复制任务处于错误(`dumbmonit_truenas_replication_error`):另一侧的副本在变旧。 | > 0 | 10 分钟 | 通知 (`warning`) | 12 小时 |
| NAS 快照任务失败 | 启用的定期快照任务处于错误(`dumbmonit_truenas_snapshot_task_error`)。 | > 0 | 10 分钟 | 通知 (`warning`) | 12 小时 |
| NAS 快照陈旧 | 启用的定期快照任务已 8 天未运行(`dumbmonit_truenas_snapshot_task_last_run_age_seconds`):停止运行的任务不失败,它变安静。 | > 8 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| TrueNAS 告警提升 | TrueNAS 本身已提升错误或以上级别的告警(`dumbmonit_truenas_alerts`)。其自己的句子在设备页面。 | > 0 | 10 分钟 | 通知 (`warning`) | 12 小时 |
| NAS 服务停止 | 设置为与 NAS 一起启动的服务已停止(`dumbmonit_truenas_service_running`)。故意关闭的服务没有系列。 | < 1 | 10 分钟 | 通知 (`warning`) | 6 小时 |

### Synology DSM {#synology-dsm}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Synology 磁盘 SMART 警告 | DSM 不再报告此磁盘的 SMART 状态为`normal`(`dumbmonit_synology_disk_smart_status`:1 注意,2 临界,未知词计为 1)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| Synology 磁盘失败 | DSM 报告磁盘已崩溃或失败(`dumbmonit_synology_disk_status`)。 | ≥ 2 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Synology 磁盘坏扇区 | 磁盘的坏扇区计数超过 DSM 设置的阈值(`dumbmonit_synology_disk_bad_sector_exceeded`)。 | > 0 | 15 分钟 | 警告 (`critical`) | 24 小时 |
| Synology 磁盘坏扇区增长 | 最后 24 小时出现新的不可读扇区(`delta(dumbmonit_synology_disk_unc_count[24h])`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| Synology SSD 磨损 | SSD 少于 10% 其额定寿命(`dumbmonit_synology_disk_remaining_life_percent`);在 12% 上清除。 | < 10 % | 1 小时 | 通知 (`warning`) | 7 天 |
| Synology 卷降级 | 卷已降级或崩溃:其冗余已丧失(`dumbmonit_synology_volume_status`)。 | ≥ 2 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Synology 存储池降级 | 存储池或 SSD 缓存已降级或崩溃:磁盘失败冗余已丧失或重建中(`dumbmonit_synology_pool_status or dumbmonit_synology_ssd_cache_status`)。 | ≥ 2 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Synology 卷即将满 | 卷已 90% 或更多(`dumbmonit_synology_volume_used_percent`);在 88% 下清除。 | ≥ 90 % | 15 分钟 | 通知 (`warning`),24 小时后升级 | 6 小时 |
| Synology 温度高 | 磁盘在 55°C 以上,或 DSM 提升其自己的温度警告(`dumbmonit_synology_disk_temperature_celsius`、`dumbmonit_synology_temperature_warning`);在 52°C 下清除。 | > 55°C | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Synology 内存高 | NAS 的内存使用超过 95%(`dumbmonit_synology_memory_usage_percent`);在 90% 下清除。 | > 95 % | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Active Backup 任务失败 | Active Backup for Business 任务最后一次运行失败,或仅备份其设备的一部分(`dumbmonit_abb_task_last_status == bool 0`)。通知命名任务。 | > 0 | 10 分钟 | 警告 (`critical`) | 24 小时 |
| Active Backup 太旧 | 此任务没有成功的 Active Backup for Business 运行超过 2 天(`dumbmonit_abb_task_last_success_seconds`)。系列对从未成功的任务不存在;"Active Backup 任务失败"然后讲话。 | > 2 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| Active Backup 任务禁用 | 任务没有计划,或其连续备份已暂停(`dumbmonit_abb_task_enabled == bool 0`):它不会备份任何东西直到有人运行它。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| Active Backup 设备逾期 | 设备已超过其自己节奏允许的更长时间没有成功的 Active Backup for Business 运行,不计闲日(`dumbmonit_abb_device_overdue`,见[Synology](../devices/synology.md#how-overdue-is-judged))。 | > 0 | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Active Backup 设备失败 | 设备的最后两次或更多尝试失败(`dumbmonit_abb_device_consecutive_failures`);取消的运行既不计数。 | ≥ 2 | 10 分钟 | 警告 (`critical`) | 24 小时 |

### 日志和度量服务器 {#log-and-metrics-servers}

VictoriaMetrics、VictoriaLogs、Loki 和 Graylog([设备](../devices/victoriametrics.md))。计数器通过`increase_prometheus`去,所以添加带拒绝数据历史的服务器不在其首次测量时触发。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 度量或日志存储只读 | 可用磁盘空间跌出`-storage.minFreeDiskSpaceBytes`下方存储拒绝每个写(`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`)。 | ≥ 1 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| 度量或日志存储几乎只读 | 少于 10% 磁盘左在只读前(`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`);在 12% 上清除。 | < 10 % | 30 分钟 | 通知 (`warning`) | 24 小时 |
| 度量或日志服务器不健康 | `/health`不应答 OK(`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`)。 | < 1 | 3 分钟 | 警告 (`critical`) | 6 小时 |
| 度量或日志数据被拒绝 | 样本或日志行在最后一小时被拒绝:时间戳在保留之外,标签太长,系列限制(`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| VictoriaMetrics 缓慢插入 | 超过 5% 新样本在 15 分钟内走了缓慢路径:没有足够内存对活跃系列(`(100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])) and on (target) dumbmonit_victoriametrics_uptime_seconds > 900`);在 3% 下清除。只有在服务器运行满 15 分钟后才会判断,因此重启(缓存变冷、计数器在窗口内归零)不会让它一直处于待定状态。 | > 5 % | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Loki 未就绪 | `/ready`不应答就绪(`dumbmonit_loki_ready`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Loki 拒绝日志行 | 行在最后一小时被拒绝:太旧,速率或流限制,行太长(`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| Loki 无法刷新到存储 | 块未能在最后 30 分钟到达存储;如果 Loki 重启它们丢失(`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`)。 | > 0 | 10 分钟 | 警告 (`critical`) | 1 小时 |
| Loki 预写日志磁盘满 | WAL 写因满磁盘丢失(`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| Loki 请求错误 | 超过 5 个请求在 15 分钟内应答服务器错误(`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`)。 | > 5 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Graylog 未处理 | 节点上的消息处理已暂停(`dumbmonit_graylog_processing`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Graylog 搜索集群关闭 | OpenSearch 或 Elasticsearch 红色,或从 Graylog 不可达(`dumbmonit_graylog_indexer_status`:2 红色,3 不可达)。 | ≥ 2 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| Graylog 搜索集群黄色 | 副本分片未分配半小时(`dumbmonit_graylog_indexer_status == 1`)。 | > 0 | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Graylog 日志填满中 | 日志超过半满:输出不赶上(`dumbmonit_graylog_journal_used_percent`);在 40% 下清除。 | > 50 % | 10 分钟 | 通知 (`warning`),1 小时后升级 | 6 小时 |
| Graylog 输入失败 | 输入未能启动(`dumbmonit_graylog_inputs_failed`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 24 小时 |
| Graylog 索引失败 | 写入搜索集群或消息处理在最后一小时失败(`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |

### 自托管应用 {#self-hosted-applications}

Nextcloud、Immich、Paperless-ngx、Jellyfin 和 Plex([设备](../devices/nextcloud.md))。他们很少完全跌出:页面打开而同步、缩略图、导入或库扫描已停止。这些规则观察那些状态;完全不应答的应用是设备不可达。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Nextcloud 卡在维护 | 维护模式打开(`dumbmonit_nextcloud_maintenance`):没有人可以登录或同步。 | > 0 | 30 分钟 | 通知 (`warning`),3 小时后升级 | 6 小时 |
| Nextcloud 数据库升级待处理 | 代码已更新 Nextcloud 等待`occ upgrade`(`dumbmonit_nextcloud_needs_db_upgrade`)。 | > 0 | 10 分钟 | 警告 (`critical`) | 1 小时 |
| Nextcloud 更新可用 | Nextcloud 的更新检查发现了更新的版本(`dumbmonit_nextcloud_update_available`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| Nextcloud OPcache 满 | PHP 的 OPcache 没有空间页面变慢(`dumbmonit_nextcloud_opcache_full`)。 | > 0 | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Nextcloud 可用空间低 | 数据目录少于 5 GiB 可用(`dumbmonit_nextcloud_free_space_bytes / 1073741824`);在 6 GiB 上清除。 | < 5 GiB | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Immich 工作停止 | 工作在不暂停并运行任何东西的队列中等待(`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`)。通知命名队列。 | > 0 | 1 小时 | 通知 (`warning`) | 6 小时 |
| Immich 队列暂停 | 工作队列已暂停一天(`dumbmonit_immich_queue_paused`)。 | > 0 | 24 小时 | 信息 (`info`) | 7 天 |
| Immich 存储即将满 | 持有库的磁盘超过 90% 满(`dumbmonit_immich_storage_used_percent`);在 88% 下清除。 | > 90 % | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Immich 更新可用 | Immich 的版本检查知道更新的版本(`dumbmonit_immich_update_available`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| Paperless 任务队列关闭 | Redis 或 Celery 出错(`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`:2 是错误):没什么新导入。 | ≥ 2 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| Paperless 索引或完整性检查错误 | 搜索索引、分类器或完整性检查(缺失或改变文件)报告错误(`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`)。 | ≥ 2 | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Paperless 迁移待处理 | 数据库迁移留待应用(`dumbmonit_paperless_unapplied_migrations`)。 | > 0 | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Paperless 任务失败 | 此用户可见的任务在失败任务窗口内失败并未被驳回(`dumbmonit_paperless_tasks_failed_recent`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 24 小时 |
| Paperless 存储即将满 | 持有文件的磁盘超过 90% 满(`dumbmonit_paperless_storage_used_percent`);在 88% 下清除。 | > 90 % | 30 分钟 | 通知 (`warning`) | 24 小时 |
| Paperless 更新可用 | Paperless-ngx 的版本检查知道更新的版本(`dumbmonit_paperless_update_available`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| Jellyfin 计划任务失败 | 计划任务(库扫描、清理)在最后一次运行时失败或被中止(`dumbmonit_jellyfin_scheduled_tasks_failed`)。 | > 0 | 10 分钟 | 通知 (`warning`) | 24 小时 |
| Jellyfin 插件破损 | 插件未能加载或不支持此版本(`dumbmonit_jellyfin_plugins_broken`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| Jellyfin 重启待处理 | Jellyfin 已等待一天在插件安装或更新后重启(`dumbmonit_jellyfin_pending_restart`)。 | > 0 | 24 小时 | 信息 (`info`) | 7 天 |
| Plex 更新可用 | Plex 的更新检查发现了更新的版本(`dumbmonit_plex_update_available`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |

### 自托管锻造 {#self-hosted-forges}

GitLab(自管理)和 Forgejo/Gitea([设备](../devices/gitlab.md))。一个
完全不应答任何东西的实例是设备不可达;这些规则观察
保持提供页面同时静默落后的部分 — Sidekiq、
CI/CD 运行者、一个停止运行的计划任务。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| GitLab 运行者全部离线 | 每个 CI/CD 运行者 GitLab 知道离线,并且至少一个已注册(`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`)。 | < 1 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| GitLab Sidekiq 积压高 | Sidekiq 队列有超过 500 个工作等待(`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`):通知、webhook 和导入落后。 | > 500 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| GitLab 迁移待处理 | 从升级的数据库迁移留待应用(`dumbmonit_gitlab_migrations_pending`)。 | > 0 | 1 小时 | 信息 (`info`) | 24 小时 |
| GitLab 许可证即将过期 | Enterprise Edition 许可证在少于 30 天内过期(`dumbmonit_gitlab_license_expiry_seconds`)。 | < 30 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| Forgejo/Gitea 运行者全部离线 | 每个 Actions 运行者离线,并且至少一个已注册(`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`)。 | < 1 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| Forgejo/Gitea 计划任务逾期 | 计划任务(存储库检查、清理、更新检查)未在其自己的间隔运行(`dumbmonit_forgejo_cron_task_overdue`)。 | > 0 | 30 分钟 | 通知 (`warning`) | 24 小时 |

### 客户端设备 {#client-devices}

一个规则,由每个跟踪设备连接和备份的集成共享 — Immich 的电话、平板电脑和桌面([设备](../devices/immich.md))、Proxmox 备份服务器的备份机器、每个备份组一个([设备](../devices/pbs.md))以及 Veeam 的受保护计算机和 VM,每个带有还原点([设备](../devices/veeam.md))、现在;更多要来。每个集成计算其自己的目标过期窗口的间隙(`device_stale_days`,默认 3 天)并作为`dumbmonit_client_device_stale_seconds`发布它,标签`device`、`type`、`os`、`user`、`kind`和`signal`(`connection`或`backup`);这一个规则然后对任何它们触发,在通知中命名设备。Synology Active Backup for Business 通过节奏感知机制(`synology_abb_device_*`)而不是这个一般系列分别跟踪其自己的设备 — 见[Synology](../devices/synology.md)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 客户端设备陈旧 | 设备在为此目标配置的过期窗口内未连接或未备份(`dumbmonit_client_device_stale_seconds`)。 | > 0 | 1 小时 | 通知 (`warning`) | 24 小时 |

### Pi-hole {#pi-hole}

[Pi-hole](../devices/pihole.md) v6,通过其 REST API 读取。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Pi-hole 阻止禁用 | 阻止关闭、暂停或失败(`dumbmonit_pihole_blocking_enabled`)。保持让 5 分钟暂停从网络接口通过。 | < 1 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Pi-hole 阻止列表陈旧 | 阻止列表(重力)已 8 天未重建(`dumbmonit_pihole_gravity_age_seconds`):每周更新失败。 | > 8 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| Pi-hole 更新可用 | 更新的版本可用(`dumbmonit_pihole_updates_available`):Docker 中的镜像、核心、网络接口或 FTL 别处。Pi-hole 每天检查一次。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| Pi-hole 诊断消息 | Pi-hole 在其诊断页面上列出消息(`dumbmonit_pihole_messages`):一个失败下载的阻止列表、一个速率限制客户端、一个超载主机。 | > 0 | 15 分钟 | 信息 (`info`) | 24 小时 |

### AdGuard Home {#adguard-home}

DNS 过滤器([设备](../devices/adguard.md))。查询和阻止数字是 AdGuard Home 自己统计窗口上的总和,不是计数器:没有规则建立在它们上。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| AdGuard Home 保护关闭 | 保护禁用,或暂停并还未返回(`dumbmonit_adguard_protection_enabled`)。 | < 1 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| AdGuard Home DNS 未运行 | 网络接口应答但 DNS 服务器已停止(`dumbmonit_adguard_running`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| AdGuard Home 上游失败 | 上游 DNS 服务器未通过 AdGuard Home 自己测试,或保持沉默 4 秒(`dumbmonit_adguard_upstream_up`,每个上游一个系列)。 | < 1 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| AdGuard Home 过滤列表陈旧 | 启用过滤列表的最旧更新超过 3 天旧(`dumbmonit_adguard_filter_oldest_update_age_seconds`)。 | > 3 天 | 1 小时 | 通知 (`warning`) | 24 小时 |
| AdGuard Home 更新可用 | 更新的版本由 AdGuard Home 自己版本检查宣布(`dumbmonit_adguard_update_available`);当那个检查被禁用时永不触发,如在官方 Docker 镜像。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |

### NUT 背后的 UPS {#ups-behind-nut}

UPS 从[NUT 服务器](../devices/nut.md)读取。每个系列携带`ups`
标签,所以告警命名 UPS 当一个服务器发布多个。状态
标志在每个读取时为 0 或 1,所以规则在主电源回时自行清除。UPS 通过 SNMP 读取由上面[UPS](#ups)规则涵盖。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| UPS 在电池上(NUT) | UPS 从电池为负载供电(`dumbmonit_nut_ups_on_battery`,`ups.status`的`OB`标志)。 | > 0 | 30 秒 | 警告 (`critical`) | 15 分钟 |
| UPS 电池低(NUT) | UPS 报告低电池,`LB`标志(`dumbmonit_nut_ups_low_battery`):它为其供电的机器关于关闭。 | > 0 | 1 分钟 | 警告 (`critical`) | 30 分钟 |
| UPS 电池需要更换(NUT) | UPS 请求其电池被更换,`RB`标志(`dumbmonit_nut_ups_replace_battery`)。 | > 0 | 1 小时 | 通知 (`warning`) | 7 天 |
| UPS 负载高(NUT) | 负载已保持在 80% 上方(`dumbmonit_nut_ups_load_percent`);在 75% 下清除。 | > 80 % | 10 分钟 | 通知 (`warning`) | 24 小时 |
| UPS 运行时短(NUT) | UPS 估计少于 5 分钟在电池上的运行时(`dumbmonit_nut_battery_runtime_seconds`);在 6 分钟上清除。在主电源上,磨损电池或为其负载太小的 UPS。 | < 300 秒 | 5 分钟 | 通知 (`warning`) | 24 小时 |
| UPS 数据陈旧(NUT) | NUT 服务器应答但从 UPS 没有新鲜数据:电缆、驱动程序或 UPS 沉默(`dumbmonit_nut_ups_data_stale`)。停电同时会看不见。 | > 0 | 5 分钟 | 通知 (`warning`),1 小时后升级 | 6 小时 |

### MikroTik RouterOS {#mikrotik-routeros}

系列携带`interface`或`sensor`标签,所以通知命名端口或传感器([设备](../devices/mikrotik.md))。计数器通过`increase_prometheus`:一个添加带多年错误的路由器不在其首次测量时触发。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| MikroTik CPU 高 | 路由器的 CPU 已保持在 90% 上方(`dumbmonit_mikrotik_cpu_load_percent`):路由、防火墙或脚本饱和它。在 80% 下清除。 | > 90 % | 15 分钟 | 通知 (`warning`) | 6 小时 |
| MikroTik 内存高 | 超过 90% 内存在使用中(`dumbmonit_mikrotik_memory_used_percent`)。在 85% 下清除。 | > 90 % | 15 分钟 | 通知 (`warning`) | 6 小时 |
| MikroTik 存储即将满 | 闪存存储超过 90% 满(`dumbmonit_mikrotik_storage_used_percent`):配置更改和升级可能失败。在 85% 下清除。 | > 90 % | 30 分钟 | 通知 (`warning`) | 24 小时 |
| MikroTik 温度高 | 温度传感器读取 80°C 以上(`dumbmonit_mikrotik_temperature_celsius`)。在 75°C 下清除。 | > 80°C | 10 分钟 | 通知 (`warning`) | 6 小时 |
| MikroTik 电源或风扇失败 | `psu*-state`或`fan*-state`传感器不是`ok`(`dumbmonit_mikrotik_health_ok`)。冗余电源已死亡切割什么,直到第二个也死亡。 | < 1 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| MikroTik 接口累积错误 | 超过 50 个在最后一小时接收和发送错误在接口上(`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`)。 | > 50 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| MikroTik 链接飘动 | 接口在 30 分钟内失去其链接超过两次(`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`)。好拔插的端口计数一次不触发。 | > 2 | 5 分钟 | 通知 (`warning`) | 1 小时 |
| MikroTik RouterOS 更新可用 | 更新的 RouterOS 在路由器的更新通道上可用(`dumbmonit_mikrotik_update_available`)。仅当路由器自己检查更新时已知:见[设置](../devices/mikrotik.md)的可选步骤。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| MikroTik 固件升级待处理 | RouterBOOT 比安装的 RouterOS 捆绑的固件更旧(`dumbmonit_mikrotik_firmware_upgrade_pending`);它仅在请求升级,然后在下一次重启。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |

### UniFi 网络 {#unifi-network}

[设备页面](../devices/unifi.md)。设备系列携带设备的名称、型号和类型,所以通知说"客厅 AP"。等待采纳的设备没有`device_up`系列并永不算作离线。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| UniFi 设备离线 | UniFi 设备由控制器采纳离线、隔离或未能采纳(`dumbmonit_unifi_device_up`)。 | < 1 | 5 分钟 | 通知 (`warning`),1 小时后升级 | 6 小时 |
| UniFi 互联网关闭 | UniFi 网关报告没有互联网连接(`dumbmonit_unifi_internet_up`)。没有 UniFi 网关系列不存在规则保持沉默。 | < 1 | 3 分钟 | 警告 (`critical`) | 1 小时 |
| UniFi WAN 链接关闭 | WAN UniFi 网关的链接关闭(`dumbmonit_unifi_wan_link_up`):带两个链接,冗余丧失而没什么看起来破损。禁用的端口没有系列。 | < 1 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| UniFi 网关 CPU 高 | UniFi 网关的处理器已在 90% 以上 15 分钟(`dumbmonit_unifi_device_cpu_percent{type="gateway"}`);在 80% 下清除。 | > 90 % | 15 分钟 | 通知 (`warning`) | 6 小时 |
| UniFi 设备等待采纳 | 设备出现在网络并等待在 UniFi 中采纳(`dumbmonit_unifi_devices{state="pending"}`)。 | > 0 | 30 分钟 | 信息 (`info`) | 24 小时 |
| UniFi 固件更新可用 | 固件更新对 UniFi 设备可用(`dumbmonit_unifi_devices_upgradable`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| UniFi 告警 | UniFi 在最后 24 小时内记录了关键事件或提升告警(`dumbmonit_unifi_alarms`)。仅用视图唯一账户读取。 | > 0 | 5 分钟 | 通知 (`warning`) | 24 小时 |

### Home Assistant {#home-assistant}


[设备页面](../devices/homeassistant.md)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Home Assistant 处于恢复模式 | Home Assistant 以恢复模式启动:无法加载其配置,自动化不运行(`dumbmonit_homeassistant_recovery_mode`)。 | > 0 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| Home Assistant 未运行 | Home Assistant 有响应,但十分钟内未完成启动(`dumbmonit_homeassistant_running`)。 | < 1 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| Home Assistant 实体变为不可用 | 不可用的实体比一小时前至少多五个:某个集成、网关或无线电停止了(`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`)。计算的是增量而非总数,因为真实安装从不为零。 | ≥ 5 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| Home Assistant 电池电量低 | Home Assistant 报告的某个电池电量低(`dumbmonit_homeassistant_battery_low`,每个低于设备阈值(默认 20%)的电池一个系列;其值为电量)。 | ≥ 0 % | 30 分钟 | 信息 (`info`) | 3 天 |
| Home Assistant 有可用更新 | Home Assistant 有待处理的更新:核心、操作系统、附加组件或设备固件(`dumbmonit_homeassistant_updates_available`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |
| Home Assistant 有待处理的修复 | Home Assistant 有一个错误或严重级别的未处理修复(`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |

### VMware vSphere {#vmware-vsphere}

[设备页面](../devices/vsphere.md)。主机系列携带 `esxi`,虚拟机系列携带 `vm`,数据存储系列携带 `datastore`,所以通知会指明对象。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| vSphere 主机无响应 | vSphere 主机不响应 vCenter(`dumbmonit_vsphere_host_connection_state`:2 表示无响应)。有意断开的主机(1)不触发。 | ≥ 2 | 3 分钟 | 警告 (`critical`) | 1 小时 |
| vSphere 主机健康状态为红色 | vSphere 将主机的整体健康状态报告为红色(`dumbmonit_vsphere_host_status == 2`)。灰色表示未知,不触发。 | > 0 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| vSphere 主机停留在维护模式 | vSphere 主机处于维护模式已有一天:其虚拟机已迁走,可能被遗忘在那里(`dumbmonit_vsphere_host_maintenance`)。 | > 0 | 24 小时 | 信息 (`info`) | 24 小时 |
| vSphere 数据存储几乎已满 | vSphere 数据存储已满 90% 或以上(`dumbmonit_vsphere_datastore_used_percent`);低于 85 % 时清除。 | ≥ 90 % | 15 分钟 | 通知 (`warning`),24 小时后升级 | 6 小时 |
| vSphere 数据存储不可访问 | vSphere 数据存储不可访问:存放其上的虚拟机无法运行(`dumbmonit_vsphere_datastore_accessible`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| vSphere 虚拟机工具未运行 | 已开机的虚拟机中安装了 VMware Tools 但未运行:客户机可能已挂起(`dumbmonit_vsphere_vm_tools_status == 2`)。没有 Tools 的虚拟机不触发。 | > 0 | 30 分钟 | 信息 (`info`) | 24 小时 |
| vSphere 红色告警 | vSphere 发出了无人确认的红色告警(`dumbmonit_vsphere_alarm{status="red"}`,每个告警一个系列,以告警和对象命名)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |

### 数据库、消息代理和安全引擎 {#databases-message-broker-and-security-engine}

Redis / Valkey、MongoDB、RabbitMQ 和 CrowdSec([Redis](../devices/redis.md)、[MongoDB](../devices/mongodb.md)、[RabbitMQ](../devices/rabbitmq.md)、[CrowdSec](../devices/crowdsec.md))。计数器经过 `increase_prometheus` 或 `changes_prometheus`,所以设备的首次测量不会触发任何规则。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Redis 内存接近 maxmemory | 已用内存超过 `maxmemory` 的 90 %(`dumbmonit_redis_memory_used_percent`);低于 85 % 时清除。没有 `maxmemory` 则无系列。 | > 90 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| Redis 拒绝连接 | 最近 15 分钟内有客户端连接被拒绝:达到 `maxclients`(`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| Redis 复制链路中断 | 副本失去与主节点的链路(`dumbmonit_redis_master_link_up`)。 | < 1 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| Redis 副本滞后 | 从主节点看,副本超过 30 秒未确认(`dumbmonit_redis_replica_lag_seconds`)。 | > 30 s | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Redis 快照失败 | 上一次 `BGSAVE` 失败;启用 `stop-writes-on-bgsave-error` 时,写入被拒绝(`dumbmonit_redis_rdb_last_save_ok`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| Redis 仅追加文件写入失败 | 上一次 AOF 写入失败(`dumbmonit_redis_aof_last_write_ok`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| MongoDB 副本集没有主节点 | 没有成员是主节点:不接受任何写入(`dumbmonit_mongodb_replset_primary_present`)。 | < 1 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| MongoDB 成员不可达 | 副本集成员无法被其他成员访问(`dumbmonit_mongodb_replset_member_health`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| MongoDB 复制滞后 | 从节点落后主节点超过一分钟(`dumbmonit_mongodb_replset_member_lag_seconds`);低于 30 s 时清除。 | > 60 s | 10 分钟 | 通知 (`warning`) | 6 小时 |
| MongoDB 连接接近上限 | 已使用超过 80 % 的可用连接(`dumbmonit_mongodb_connections_used_percent`);低于 75 % 时清除。 | > 80 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| MongoDB 缓存压力 | 超过 20 % 的 WiredTiger 缓存为脏数据(`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`);低于 15 % 时清除。 | > 20 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| MongoDB 内部错误 | 最近一小时内出现常规断言(`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 24 小时 |
| RabbitMQ 内存或磁盘告警 | 节点发出资源告警:所有发布者被阻塞(`dumbmonit_rabbitmq_alarm`,每个节点和资源一个系列)。 | > 0 | 1 分钟 | 警告 (`critical`) | 1 小时 |
| RabbitMQ 节点宕机 | 集群节点未运行(`dumbmonit_rabbitmq_node_running`)。 | < 1 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| RabbitMQ 网络分区 | 节点检测到网络分区(`dumbmonit_rabbitmq_node_partitions`)。 | > 0 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| RabbitMQ 队列没有消费者 | 队列中有消息但没有消费者(`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`);值为等待的消息数。 | > 0 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| RabbitMQ 队列不可用 | 队列未运行:仲裁队列没有多数,或其节点已宕机(`dumbmonit_rabbitmq_queue_running`)。 | < 1 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| CrowdSec 本地 API 宕机 | 本地 API 不响应 `/health`(`dumbmonit_crowdsec_lapi_up`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 1 小时 |
| CrowdSec bouncer 停止拉取 | bouncer 的请求数在 30 分钟内没有变化(`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`)。 | < 1 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| CrowdSec 未读取日志 | 六小时内没有读取任何一行日志(`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`)。 | < 1 | 30 分钟 | 通知 (`warning`) | 24 小时 |

### 反向代理和域名 {#reverse-proxies-and-domains}

Traefik、Caddy、Nginx Proxy Manager 和域名注册([Traefik](../devices/traefik.md)、[Caddy](../devices/caddy.md)、[Nginx Proxy Manager](../devices/npm.md)、[域名到期](../devices/domain.md)),以及 DNS 检查的*应答变化时告警*选项([服务](../devices/services.md#alerting-when-the-answer-changes))。系列携带 `router`、`service`、`server`、`upstream`、`proxy_host` 或 `cert`,所以通知会指明失败的对象。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Traefik 路由器被禁用 | 路由器因配置错误(缺少服务或中间件、规则无效)被禁用:其站点不被服务(`dumbmonit_traefik_router_status`:0 启用,1 警告,2 禁用)。 | ≥ 2 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Traefik 后端服务器宕机 | 负载均衡服务的某个服务器未通过健康检查(`dumbmonit_traefik_server_up`)。 | < 1 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Traefik 服务没有服务器 | 服务的所有服务器都不响应健康检查:其站点返回 503(`dumbmonit_traefik_service_servers_up`)。 | < 1 | 2 分钟 | 警告 (`critical`) | 1 小时 |
| Traefik 未获取证书 | 路由器向证书解析器请求证书,但 Traefik 持有的证书尚未覆盖:Traefik 提供其默认的自签名证书(`dumbmonit_traefik_resolver_routers_uncovered`,需要 Traefik 的 `/api/certificates`)。 | > 0 | 1 小时 | 通知 (`warning`) | 24 小时 |
| Caddy 上游不健康 | 后端未通过 Caddy 的主动或被动健康检查(`dumbmonit_caddy_upstream_healthy`)。 | < 1 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Caddy 配置重载失败 | Caddy 拒绝了上一次配置重载,仍运行之前的配置(`dumbmonit_caddy_config_last_reload_ok`)。 | < 1 | 5 分钟 | 通知 (`warning`) | 24 小时 |
| Nginx Proxy Manager 主机离线 | nginx 拒绝了已启用主机的配置:该主机不被服务(`dumbmonit_npm_host_online`)。 | < 1 | 10 分钟 | 通知 (`warning`) | 6 小时 |
| 代理证书未续期 | Traefik 持有的证书或 Nginx Proxy Manager 主机提供的证书将在 14 天内到期:自动续期从 30 天开始,所以续期正在失败(`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`)。 | < 14 d | 1 小时 | 通知 (`warning`) | 24 小时 |
| 反向代理返回 5xx | 十分钟内超过 5 % 的应答为 5xx,且至少有五十个请求(`dumbmonit_{traefik,caddy}_requests_5xx_total` 的 `increase_prometheus` 除以 `…_requests_total`)。需要代理的指标。 | > 5 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| 域名即将到期 | 域名将在 30 天内到期(`dumbmonit_domain_expiry_days >= 7`)。 | < 30 d | 1 小时 | 通知 (`warning`) | 24 小时 |
| 域名马上到期 | 域名将在 7 天内到期,或已到期(`dumbmonit_domain_expiry_days`)。 | < 7 d | 1 小时 | 警告 (`critical`) | 12 小时 |
| 域名被冻结 | 注册局将域名置于 `clientHold` 或 `serverHold`:它已从 DNS 中移除(`dumbmonit_domain_on_hold`)。 | > 0 | 30 分钟 | 警告 (`critical`) | 6 小时 |
| 域名处于赎回期 | 域名处于 `redemptionPeriod`、`pendingDelete` 或 `pendingRestore`:它已过期(`dumbmonit_domain_redemption`)。 | > 0 | 30 分钟 | 警告 (`critical`) | 12 小时 |
| DNS 应答已变化 | 启用*应答变化时告警*的 DNS 检查,其应答与之前不同(`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`)。一小时后自行解决。 | > 0 | 1 分钟 | 通知 (`warning`) | 24 小时 |

### 网站变化 {#website-changes}

参见[网站变化](../devices/webchange.md)。设备页面显示每次变化前后的对比。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 网站已变化 | 对被监视页面或站点的一次检查发现有页面变化、出现或消失(`dumbmonit_webchange_last_check_changes`)。每次这样的检查都会发出通知并列出这些页面,触发期间属于同一条告警;在第一次未发现新内容的检查时解决。 | > 0 | 30 秒 | 信息 (`info`) | 无 |

### Kubernetes {#kubernetes}

参见 [Kubernetes / k3s](../devices/kubernetes.md)。每个系列都标有节点,或相关的命名空间和 Pod、工作负载或卷声明。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Kubernetes 节点未就绪 | 节点不是 Ready:其 kubelet 不再汇报,其 Pod 将被驱逐(`dumbmonit_k8s_node_ready == bool 0`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Kubernetes 节点承压 | 节点报告内存、磁盘或 PID 压力(`dumbmonit_k8s_node_pressure`,标签 `condition`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Kubernetes Pod 崩溃循环 | 容器处于 `CrashLoopBackOff` 且仍在重启:15 分钟内至少重启两次(`max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) and on (target, namespace, pod) increase_prometheus(dumbmonit_k8s_pod_restarts[15m]) >= 2`;每次重启尝试时该状态会短暂消失)。因节点或运行时重启而被终止的容器(退出码 137、143 或 255,非 `OOMKilled`)不计入,因此重启机器不会看起来像崩溃循环。 | > 0 | 10 分钟 | 警告 (`critical`) | 6 小时 |
| Kubernetes Pod 卡在 Pending | Pod 已 Pending 十五分钟:没有节点能接纳它,或它需要的卷或镜像迟迟未到(`dumbmonit_k8s_pod_pending`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Kubernetes Pod 反复重启 | Pod 在最近一小时内重启超过五次,但不处于 `CrashLoopBackOff`(`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`)。 | > 5 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Kubernetes 工作负载缺少副本 | Deployment、StatefulSet 或 DaemonSet 的可用副本少于所要求的(`dumbmonit_k8s_workload_unavailable`)。 | > 0 | 10 分钟 | 通知 (`warning`),1 小时后升级 | 6 小时 |
| Kubernetes 卷声明 Pending | Pod 需要的、或与配置错误相关的 PersistentVolumeClaim 仍未绑定(`dumbmonit_k8s_pvc_pending`)。 | > 0 | 15 分钟 | 通知 (`warning`) | 6 小时 |

### Active Directory {#active-directory}

参见 [Active Directory](../devices/activedirectory.md)。DumbMonit 查询的域控制器由*设备不可达*覆盖;域中的其他域控制器由*域控制器不可达*覆盖(标签 `dc`)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 域控制器不可达 | 域中的某个域控制器不再响应其 LDAP 端口(`dumbmonit_ad_dc_reachable == bool 0`)。DumbMonit 无法解析的名称不产生系列,也永不触发。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Active Directory 绑定失败 | 域控制器有响应但拒绝服务账户:密码过期、账户被禁用或锁定(`dumbmonit_ad_bind_ok == bool 0`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 24 小时 |
| 特权组成员变化 | 有成员加入或离开 Domain Admins、Enterprise Admins、Schema Admins、Administrators 或某个操作员组,包括嵌套(`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`,标签 `group`)。一小时后自行解决。 | > 0 | 1 分钟 | 通知 (`warning`) | 24 小时 |
| Active Directory 复制失败 | 被查询的域控制器无法从伙伴复制(`dumbmonit_ad_replication_consecutive_failures`)。仅当账户可读取复制状态时。 | > 0 | 30 分钟 | 通知 (`warning`) | 6 小时 |
| krbtgt 密码超过 180 天 | krbtgt 密码上次更改已超过 180 天(`dumbmonit_ad_krbtgt_password_age_seconds`)。 | > 180 天 | 10 分钟 | 信息 (`info`) | 无 |

### pfSense {#pfsense}

参见 [pfSense](../devices/pfsense.md)。网关标有 `gateway`,接口标有 `interface` 和 `descr`,服务标有 `service`。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| pfSense 网关宕机 | pfSense 网关宕机:其监控地址不再响应,所承载的流量中断或切换(`dumbmonit_pfsense_gateway_up`)。 | < 1 | 3 分钟 | 警告 (`critical`) | 6 小时 |
| pfSense 网关丢包 | pfSense 网关对其监控地址的探测丢失超过 10 %:线路质量下降(`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`)。 | > 10 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| pfSense 网关缓慢 | 到 pfSense 网关监控地址的往返时间已连续十五分钟超过 500 ms(`dumbmonit_pfsense_gateway_delay_milliseconds`)。 | > 500 ms | 15 分钟 | 通知 (`warning`) | 6 小时 |
| pfSense 接口无链路 | 已启用的 pfSense 接口没有链路:网线、交换机端口或网卡(`dumbmonit_pfsense_interface_down`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| pfSense 服务已停止 | 已启用的 pfSense 服务未运行:DNS 解析器、VPN、NTP…(`dumbmonit_pfsense_service_stopped`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| pfSense 磁盘几乎已满 | pfSense 磁盘已满超过 90 %:日志、报告和软件包更新将无法写入(`dumbmonit_pfsense_disk_used_percent`)。 | > 90 % | 30 分钟 | 通知 (`warning`) | 1 天 |

### Unraid {#unraid}

参见 [Unraid](../devices/unraid.md)。磁盘标有 `disk` 和 `role`(`parity`、`data`、`cache`),容器标有 `container`。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Unraid 阵列已停止 | Unraid 阵列未启动:位于其上的共享、容器和虚拟机不可用(`dumbmonit_unraid_array_started`)。 | < 1 | 10 分钟 | 警告 (`critical`) | 6 小时 |
| Unraid 磁盘被禁用或丢失 | Unraid 磁盘被禁用、丢失或无效:阵列降级运行,再有一次故障就会丢失数据(`dumbmonit_unraid_disk_ok`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Unraid 磁盘读取错误 | Unraid 磁盘自阵列启动以来报告了读取错误:下一次可能被禁用(`dumbmonit_unraid_disk_errors`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 1 天 |
| Unraid 磁盘过热 | Unraid 阵列盘或校验盘已连续十五分钟超过 55 °C(`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`)。 | > 55 °C | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Unraid 校验检查发现错误 | 上一次 Unraid 校验检查失败或发现同步错误:校验不再与数据一致(`dumbmonit_unraid_parity_check_ok`)。 | < 1 | 5 分钟 | 通知 (`warning`) | 1 天 |
| Unraid 校验检查逾期 | 上一次 Unraid 校验检查已超过 40 天:错过了每月检查(`dumbmonit_unraid_parity_check_age_seconds`)。 | > 40 d | 1 小时 | 通知 (`warning`) | 1 天 |
| Unraid 缓存几乎已满 | Unraid 缓存池已满超过 90 %:写入回落到较慢的阵列,或失败(`dumbmonit_unraid_disk_used_percent{role="cache"}`)。 | > 90 % | 30 分钟 | 通知 (`warning`) | 1 天 |
| Unraid 容器已停止 | 设置为随阵列启动的 Docker 容器未运行(`dumbmonit_unraid_container_autostart_stopped`)。 | > 0 | 10 分钟 | 通知 (`warning`) | 6 小时 |

### Veeam Backup & Replication {#veeam-backup--replication}

参见 [Veeam Backup & Replication](../devices/veeam.md)。作业标有 `job` 和 `type`,存储库标有 `repository`。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Veeam 作业失败 | 已启用的 Veeam 作业上一次运行失败:它保护的对象没有新的还原点(`dumbmonit_veeam_job_failed`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 1 天 |
| Veeam 作业以警告结束 | 已启用的 Veeam 作业上一次运行以警告结束:部分对象可能未受保护(`dumbmonit_veeam_job_warning`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 1 天 |
| Veeam 存储库几乎已满 | Veeam 备份存储库已满超过 90 %:接下来的作业将因空间不足而失败(`dumbmonit_veeam_repository_used_percent`)。 | > 90 % | 30 分钟 | 通知 (`warning`) | 1 天 |
| Veeam 许可证即将到期 | Veeam 许可证将在 30 天内到期:宽限期结束后作业将停止(`dumbmonit_veeam_license_expiry_seconds`)。 | < 30 d | 1 小时 | 通知 (`warning`) | 1 天 |

### Tailscale {#tailscale}

参见 [Tailscale](../devices/tailscale.md)。设备标有 `device`,即其 MagicDNS 名称的第一段。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Tailscale 设备离线 | 必须保持在线的 Tailscale 设备(默认为带标签的设备)已断开超过 15 分钟(`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`)。 | > 15 分钟 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| Tailscale 节点密钥即将到期 | Tailscale 设备的节点密钥将在 14 天内到期:在有人重新登录之前它将离开 tailnet(`dumbmonit_tailscale_device_key_expiry_seconds`)。 | < 14 d | 1 小时 | 通知 (`warning`) | 1 天 |
| Tailscale 设备等待批准 | 有设备加入 tailnet 并等待批准:如果在预期之内就批准,否则移除(`dumbmonit_tailscale_device_authorized`)。 | < 1 | 5 分钟 | 通知 (`warning`) | 1 天 |
| Tailscale 客户端更新 | 必须保持在线的 Tailscale 设备运行的客户端有可用更新(`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`)。 | > 0 | 6 小时 | 信息 (`info`) | 7 天 |

### FortiGate {#fortigate}

参见 [FortiGate](../devices/fortigate.md)。隧道标有 `tunnel`,接口标有 `interface` 和 `alias`,许可证标有 `license`。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| FortiGate IPsec 隧道宕机 | FortiGate 站点到站点 IPsec 隧道的第 2 阶段全部未建立:其后的站点被切断(`dumbmonit_fortigate_ipsec_tunnel_up`)。 | < 1 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| FortiGate 接口无链路 | 已启用且有地址的 FortiGate 接口没有链路:网线、收发器或对端设备(`dumbmonit_fortigate_interface_down`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |
| FortiGate HA 不同步 | FortiGate HA 集群的成员配置校验和不再相同:故障切换后将运行另一份配置(`dumbmonit_fortigate_ha_in_sync`)。 | < 1 | 15 分钟 | 通知 (`warning`) | 6 小时 |
| FortiGate HA 成员丢失 | FortiGate HA 集群的成员比最近 24 小时少:不再有备用节点(`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| FortiGate 内存接近保守模式 | FortiGate 内存已连续十分钟超过 85 %:达到 88 %(默认)时进入保守模式,停止检查新会话(`dumbmonit_fortigate_memory_used_percent`)。 | > 85 % | 10 分钟 | 通知 (`warning`) | 6 小时 |
| FortiGate CPU 高 | FortiGate CPU 已连续十五分钟超过 90 %(`dumbmonit_fortigate_cpu_usage_percent`)。 | > 90 % | 15 分钟 | 通知 (`warning`) | 6 小时 |
| FortiGate 许可证即将到期 | FortiGate 的某个 FortiGuard 或 FortiCare 许可证将在 30 天内到期,或已到期(`dumbmonit_fortigate_license_expiry_seconds`)。 | < 30 d | 1 小时 | 通知 (`warning`) | 1 天 |
| FortiGate 固件更新 | FortiGuard 为 FortiGate 所运行的 FortiOS 分支提供了更新的补丁版本(`dumbmonit_fortigate_firmware_update_available`)。 | > 0 | 1 小时 | 信息 (`info`) | 7 天 |

### Sophos Firewall {#sophos-firewall}

参见 [Sophos Firewall](../devices/sophos.md)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Sophos 接口无链路 | 已开启并绑定到区域的 Sophos Firewall 接口没有链路(`dumbmonit_sophos_interface_down`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |

### Hyper-V {#hyper-v}

由 Hyper-V 主机上的 Windows 代理报告;参见[代理](../devices/agent.md#hyper-v-hosts)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| Hyper-V 虚拟机健康状态严重 | Hyper-V 报告虚拟机处于严重健康状态:它所需的资源(多数是磁盘空间)已耗尽(`dumbmonit_hyperv_vms_health_critical`)。 | > 0 | 5 分钟 | 警告 (`critical`) | 6 小时 |
| Hyper-V 主机 CPU 高 | Hyper-V 主机的逻辑处理器已连续十五分钟占用超过 90 %,包括客户机(`dumbmonit_hyperv_host_cpu_percent`)。 | > 90 % | 15 分钟 | 通知 (`warning`) | 6 小时 |
| Hyper-V 虚拟磁盘错误 | Hyper-V 虚拟磁盘在最近一小时内报告了 I/O 错误(`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`)。 | > 0 | 5 分钟 | 通知 (`warning`) | 6 小时 |

### 安全评分 {#security-score}

服务器每 15 分钟为每个其类型带有安全检查的设备计算一次,并存储为 `dumbmonit_security_score`(0–100)。参见[安全评分](../using/security-score.md)。

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| 安全评分下降 | 评分相比最近 24 小时的最佳值下降了 10 分或以上(`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`):某项检查开始失败。 | ≥ 10 分 | 30 分钟 | 通知 (`warning`) | 24 小时 |
| 安全评分低 | 评分低于 40,等级 F(`dumbmonit_security_score`)。可调高或调低阈值以符合您自己的标准。 | < 40 | 1 小时 | 信息 (`info`) | 7 天 |

### DumbMonit 自身 {#dumbmonit-itself}

| 规则 | 什么 | 默认阈值 | 保持 | 严重级别 | 提醒 |
|---|---|---|---|---|---|
| DumbMonit 备份未运行 | DumbMonit 数据库的计划本地备份已超过两天未运行(`time() - dumbmonit_instance_backup_last_success_seconds`)。 | > 2 天 | 1 小时 | 通知 (`warning`) | 24 小时 |

每条内置规则都适用于所有设备和每个已启用的通道。

## 在界面中编辑规则 {#editing-a-rule-in-the-ui}

在**告警**页面,*规则*部分列出每条规则及其严重级别、*内置*标记,以及(相关时)当前触发的告警数。每条规则有:

- 一个用于**启用或禁用**的开关;
- **编辑**,展开内联编辑器,可修改名称、描述、运算符、阈值、保持(`for`)、严重级别、提醒(`repeat every`)和升级(`escalate after`)。内置规则的查询可见但不可编辑;您自己规则的查询可以编辑。基线规则的检测参数为只读;
- **删除**,仅适用于您自己的规则。

## 按设备覆盖 {#per-device-overrides}

可以为单个设备调整规则而无需修改规则本身:不同的阈值、不同的清除阈值,或仅对该设备关闭。每条规则的编辑器都有**按设备覆盖**面板,可直接添加或移除。

告警上的一键**忽略**操作(在*需要您处理*列表或告警所在的设备页面中)是同一机制,以 `enabled: false` 写入:设备立即、可逆地不再携带该规则。设备页面列出它所忽略的内容,并带有**停止忽略**;规则本身在 告警 → 规则 中显示*已在 N 台设备上忽略*,同一个按设备覆盖面板可将其移除。API 参见[按设备覆盖](../reference/api.md#per-device-overrides)。

## 创建阈值规则 {#creating-a-threshold-rule}

点击**新建规则**。表单需要:

| 字段 | 含义 |
|---|---|
| 名称 | 显示在告警和通知中。 |
| 查询 | 按设备求值的 MetricsQL 表达式。示例:`dumbmonit_cpu_usage_percent`。 |
| 运算符 | `>`、`>=`、`<` 或 `<=`。 |
| 阈值 | 查询结果所比较的数字。 |
| 严重级别 | 信息、通知或警告。 |
| 保持时间 | 条件必须持续多少秒才触发。非零的保持时间可吸收孤立的坏样本。 |

查询返回的每个系列都成为一个潜在告警,附加到其 `target` 标签所指的设备。若希望每台设备一个告警而不是每个核心或每个接口一个,请使用 `avg by (target, host) (…)` 聚合。只要存在 `target`,告警的标识就忽略 `host` 和 `tag_*` 标签,所以重命名设备会保留其告警;`target` 与现有已启用设备不匹配的系列会被忽略。

## 查询语言 {#the-query-language}

查询使用 [MetricsQL](https://docs.victoriametrics.com/metricsql/),即 VictoriaMetrics 的 PromQL 超集。指标是[指标参考](../reference/metrics.md)中描述的 `dumbmonit_*` 系列,每个系列都带有 `target` 标签(设备 id)和 `host` 标签(其名称)。

内置规则使用的一些模式:

```
# Age of the last sample, freshest series per device: what "unreachable" means
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))

# One value per device, whatever the source
avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)

# A percentage computed from two gauges
100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total

# A boolean condition: the series is returned only when true
dumbmonit_ups_output_source == 5

# Extrapolation done by VictoriaMetrics
predict_linear(dumbmonit_pbs_datastore_used_percent[6h], 345600) and deriv(dumbmonit_pbs_datastore_used_percent[6h]) > 0

# Availability of a service over thirty days
avg_over_time(dumbmonit_probe_success[30d])
```

!!! tip "先试一试查询"
    `GET /api/metrics/query?query=…` 返回规则将要求值的内容(参见 [API 参考](../reference/api.md#metrics))。在开发环境中,覆盖文件会把内嵌的 VictoriaMetrics 发布到主机的 `:8428`,并带有其查询界面。

## API 中的规则类型 {#rule-kinds-in-the-api}

`kind` 为 `threshold`(与阈值比较)、`predict`(相同的比较,基于 `predict_linear` 查询;显示为预测)或 `anomaly`(季节性基线;`params` 包含 `k`、`alpha`、`mad_floor_abs`、`mad_floor_rel`、`min_samples`)。界面创建 `threshold` 规则;另外两种类型可通过 API 创建。
