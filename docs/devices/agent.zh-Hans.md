# Linux、macOS、FreeBSD 和 Windows 代理 {#linux-macos-freebsd-and-windows-agent}

一个安装在要监视的机器上的小二进制文件。它测量 CPU、内存、磁盘、网络、服务和容器 — 以及机器公开的地方，还有温度、磁盘健康、ZFS 池和 KVM 虚拟机 — 随后通过 HTTP **推送** 其读数到 DumbMonit 服务器。

方向是故意的：代理调用服务器，永远不是相反的方式。它穿过 NAT 和家庭防火墙，并要求在监视的机器上打开 **没有端口**。

## 它监视什么 {#what-it-watches}

所有指标都以 `dumbmonit_` 为前缀。身份标签（`target`、`host`、`tag_*`）在接收时由服务器设置，从不由代理设置：一台机器无法写入另一台机器的系列，即使伪造其标签。

| 族 | 指标 | 标签 |
|---|---|---|
| CPU | `cpu_usage_percent`、`cpu_count`、`load_average_1/5/15`；`cpu_core_usage_percent` 仅使用 `cpu_per_core: true` | `core` |
| 内存 | `memory_total/used/available_bytes`、`memory_used_percent`、`swap_total/used_bytes`、`swap_used_percent` | |
| 文件系统 | `filesystem_total/used/free_bytes`、`filesystem_used_percent` | `mountpoint`、`device`、`fstype` |
| 网络（计数器） | `if_octets_in/out`、`if_packets_in/out`、`if_errors_in/out` | `ifname` |
| 磁盘（计数器） | `disk_read_bytes`、`disk_written_bytes`、每个块设备一个系列 | `device` |
| 主机 | `uptime_seconds`、`process_count` | |
| 服务 | `service_up`（1 = 运行） | `service` |
| 容器 | `container_up`、`container_count`、`container_running_count`、`container_series_skipped`（超过 `docker_max_containers` 的容器） | `container`、`image` |
| 传感器 | `agent_sensor_temperature_celsius`、`agent_sensor_temperature_critical_celsius`（硬件本身声明的阈值）、`agent_sensor_fan_rpm` | `sensor`、`fan` |
| 磁盘健康（SMART） | `agent_disk_smart_ok`（1 = 磁盘通过其自身自我评估）、`agent_disk_temperature_celsius`、`agent_disk_power_on_hours`、`agent_disk_wearout_percent`、`agent_disk_reallocated_sectors`、`agent_disk_pending_sectors`、`agent_disk_media_errors` | `device`、`model` |
| KVM / libvirt | `agent_libvirt_domains`, `agent_libvirt_domains_running`, `agent_libvirt_pools`；每个域 `agent_libvirt_domain_state_info`, `agent_libvirt_domain_running`, `agent_libvirt_domain_crashed`, `agent_libvirt_domain_vcpus`, `agent_libvirt_domain_memory_max_bytes`, `agent_libvirt_domain_memory_used_bytes`；每个存储池 `agent_libvirt_pool_state_info`, `agent_libvirt_pool_active`, `agent_libvirt_pool_capacity_bytes`, `agent_libvirt_pool_allocation_bytes`, `agent_libvirt_pool_used_percent` — 参见 [通过 libvirt 监视 KVM 虚拟机](#kvm-virtual-machines-through-libvirt) | `domain`、`pool`、`state` |
| ZFS | `agent_zfs_pool_health`（0 在线，1 降级，2 更差）、`agent_zfs_pool_size/used/free_bytes`、`agent_zfs_pool_used_percent`、`agent_zfs_pool_fragmentation_percent`、`agent_zfs_pool_device_errors`、`agent_zfs_pool_data_errors`、`agent_zfs_pool_scrub_running`、`agent_zfs_pool_scrub_errors`、`agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers`；每个对等体 `agent_wireguard_peer_handshake_age_seconds`、`agent_wireguard_peer_has_handshake`、`agent_wireguard_peer_keepalive_seconds`、`agent_wireguard_peer_rx_bytes`、`agent_wireguard_peer_tx_bytes`（计数器） — 参见 [WireGuard 隧道](#wireguard-tunnels) | `interface`、`peer`、`allowed_ips`、`name` |
| IPsec (strongSwan) | `agent_ipsec_tunnel_up`；建立时，`agent_ipsec_tunnel_bytes_in/out`（计数器）和 `agent_ipsec_tunnel_rekey_seconds` — 参见 [IPsec 隧道](#ipsec-tunnels) | `connection`、`tunnel` |
| 互联网速度测试 | `agent_speedtest_ok`、`agent_speedtest_age_seconds`、`agent_speedtest_download_bytes_per_second`、`agent_speedtest_upload_bytes_per_second`、`agent_speedtest_latency_ms`、`agent_speedtest_jitter_ms`；配置的阈值回显为 `agent_speedtest_download_min_bytes_per_second`、`agent_speedtest_upload_min_bytes_per_second`、`agent_speedtest_max_latency_ms` — 参见 [互联网速度测试](#internet-speed-test) | |
| restic / Borg | `agent_backup_repo_reachable`、`agent_backup_repo_last_snapshot_age_seconds`、`agent_backup_repo_last_snapshot_bytes`（restic） — 参见 [restic 和 Borg 备份](#restic-and-borg-backups) | `tool`、`repo` |
| Windows 性能计数器 | `agent_perf_counter`，每个在 `perf_counters` 中列出的计数器一个系列 | `counter` |
| MDaemon（Windows） | `mdaemon_queue_messages`、`mdaemon_queue_frozen`、`mdaemon_sessions_active`、`mdaemon_sessions_total`、`mdaemon_messages_total`、`mdaemon_filtered_messages_total`、`mdaemon_server_active`、`mdaemon_running`、`mdaemon_uptime_seconds` — 参见 [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`、`protocol`、`filter`、`verdict`、`server` |
| Hyper-V（Windows） | `hyperv_vms_health_ok`、`hyperv_vms_health_critical`、`hyperv_host_cpu_percent`、`hyperv_host_cpu_guest_percent`、`hyperv_partitions`；每个 VM `hyperv_vm_physical_memory_megabytes`、`hyperv_vm_guest_visible_memory_megabytes`、`hyperv_vm_memory_pressure_percent`；每个虚拟磁盘 `hyperv_vhd_read_bytes_per_second`、`hyperv_vhd_write_bytes_per_second`、`hyperv_vhd_latency_seconds`、`hyperv_vhd_queue_length`、`hyperv_vhd_errors_total` — 参见 [Hyper-V 主机](#hyper-v-hosts) | `vm`、`disk` |
| 代理 | `agent_collect_seconds`、`agent_buffered_samples`、`agent_dropped_samples` | |

无法测量的系列是 **不存在的**，永不为零：没有 `smartctl`，没有 `agent_disk_*`；没有可读的探测，没有 `agent_sensor_*`；没有 ZFS，没有 `agent_zfs_*`；没有 `virsh`，没有 `agent_libvirt_*`。1–150 °C 之外的温度被丢弃而不是发布，读取 0 rpm 的风扇也被丢弃 — 主板的风扇头多于风扇，空的读取零，就像死亡风扇一样。

### 在哪里收集什么 {#what-is-collected-where}

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU、内存、交换、文件系统、网络、正常运行时间、进程 | 是 | 是 | 是 | 是 |
| 平均负载 | 是 | 是 | 是 | 否（Windows 没有这样的概念） |
| 服务 | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | Windows 服务 |
| Docker 容器 | 是 | 是，如果 `docker_socket` 指向 Docker Desktop 的套接字 | 否（没有 Docker 守护进程） | 否 |
| 温度 | `/sys/class/hwmon` | SMC 传感器 | `dev.cpu.N.temperature`（仅 CPU） | 否 |
| 风扇 | `/sys/class/hwmon` | 否 | 否 | 否 |
| 磁盘健康（SMART） | `smartctl` | `smartctl`（Homebrew） | `smartctl`（`sysutils/smartmontools`） | `smartctl` |
| ZFS 池 | `zpool`（OpenZFS） | 否 | `zpool` | 否 |
| KVM 虚拟机（libvirt） | `virsh` | 否 | 否 | 否 |
| OS 健康（更新、重启、失败的单元、SELinux） | 是 | 否 | 否 | 否 |
| Plakar 备份 | 是 | 是 | 是 | 是 |
| WireGuard 隧道 | `wg show` | 当 `wg` 被安装时（未测试） | 当 `wg` 被安装时（未测试） | 否 |
| IPsec 隧道 | `swanctl` | 当 `swanctl` 被安装时（未测试） | 当 `swanctl` 被安装时（未测试） | 否 |
| 互联网速度测试 | 是，需主动启用 | 是，需主动启用 | 是，需主动启用 | 是，需主动启用 |
| restic 和 Borg 备份 | 是 | 是 | 是 | 仅 restic（未测试；Borg 没有 Windows 构建） |
| 性能计数器、MDaemon 队列、Hyper-V | 否 | 否 | 否 | 是 |
| 机器身份 | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`、`/etc/hostid` | 主机名 |
| 中继模式 | 是 | 是 | 是 | 是 |

Windows 故意没有温度行：读取那里的传感器意味着通过 WMI 进行，具有管理权限，对于大多数机器根本不公开的值。发布什么都不是诚实的回答。

`smartctl`（smartmontools）、`zpool` 和 `virsh` 在每个读取期间被查找并在存在时使用；代理既不安装，也不是必需的。它们都需要 root（`virsh` 对 `libvirt` 组成员同样有效） — 安装程序注册的服务以 root 运行，手启动的代理在您自己的账户下将找到什么都没有。

计数器被原始发送，所以代理的重启永不产生虚假速率；在查询中应用 `rate()`。

适用的内置规则：监控项不可达、高 CPU、不寻常的 CPU。"监控项不可达"也适用于代理：服务器检查最后接收到的批次的年龄，并在错过三个周期后声明机器沉默（至少 90 秒）。

四个更多仅在产生匹配系列的机器上触发，所以它们不需要设置，也无法在没有硬件的机器上触发：

| 规则 | 何时触发 |
|---|---|
| **磁盘 SMART 故障**（关键） | 磁盘的自我评估将其报告为故障 |
| **ZFS 池降级**（关键） | 池不再是五分钟的 `ONLINE` |
| **ZFS 清洁发现错误**（关键） | 最后清洁发现错误，或池报告永久损坏的文件 |
| **KVM 域崩溃**（关键） | libvirt 将某个域报告为 `crashed` |
| **libvirt 池将满**（通知） | 某个存储池的分配量连续十五分钟达到其容量的 90% |
| **温度超过关键**（关键） | 传感器超过关键阈值 **其自己的硬件声明** — 没有硬编码的值，85 °C 是磁盘上的紧急，是笔记本电脑 CPU 上的正常下午 |

五个更多监视您声明或运行的东西，否则保持沉默：**WireGuard 对等体沉默**（一个具有持久保活且 15 分钟内没有握手的对等体）、**IPsec 隧道宕机**（一个已配置的隧道十分钟内没有已安装的会话）、**restic/Borg 备份太旧**（48 小时内没有快照）、**restic/Borg 存储库不可达**，以及 **互联网下载速率低** / **互联网上传速率低** / **互联网延迟高**（仅在设置了阈值选项之后） — 参见 [WireGuard 隧道](#wireguard-tunnels)、[IPsec 隧道](#ipsec-tunnels)、[restic 和 Borg 备份](#restic-and-borg-backups) 和 [互联网速度测试](#internet-speed-test)。

在运行 MDaemon 的 Windows 服务器上，三个更多监视其队列：**MDaemon 邮件队列增长**、**MDaemon 坏队列不为空** 和 **MDaemon 重试队列高**（参见[规则](../alerting/rules.md#mdaemon-and-securitygateway)）。
在 Hyper-V 主机上，三个监视虚拟机：**Hyper-V VM 健康关键**、**Hyper-V 主机 CPU 高** 和 **Hyper-V 虚拟磁盘错误**（参见 [Hyper-V 主机](#hyper-v-hosts)）。

## 安装 {#install}

下面的步骤是通知旁显示的步骤。没有在机器上创建账户：注册令牌是唯一的密钥。

1. 在 DumbMonit 中保存监控项：显示注册令牌（dmon_…），其中一次带有为 Linux 和 Windows 准备好复制的安装命令。（令牌也可以在 **设置 → 代理** 中创建。）
2. 该令牌是代理向 DumbMonit 推送其测量的密钥。它不是机器上的账户：那里什么也不需要创建。从监控项表单创建的令牌是单次使用的：它注册这一台机器，别的什么也没有。对于一个队列，在"设置 → 代理"中创建一个可重复使用的令牌。
3. 在机器上运行安装命令，以提升的权限（Linux 和 macOS 上的 sudo，Windows 上的提升 PowerShell）。它下载代理，将令牌写入 agent.yaml 并启动服务。
4. 机器在几秒钟内自己显示，以其主机名命名。在那个第一批中，服务器给代理一个自己的密钥，并将其存储在 `/etc/dumbmonit/agent-secret` 中，任何人都无法读取：从那以后，那台机器是唯一一台可以在这个监控项下报告的。丢失令牌？"设置 → 代理"让您撤销它并创建另一个。
5. Docker：要查看容器并让 DumbMonit 重启或更新它们，代理必须到达 Docker 套接字。安装程序注册的服务已经可以；如果您在专用用户下运行代理，将该用户添加到"docker"组并重启它。重启和自动更新策略随后在监控项页面按容器设置。

两件事值得在您推出到多台机器之前阅读：[绑定](#binding-one-machine-one-agent)，这使监控项真正属于一个代理安装，以及 [单次使用或队列](#single-use-or-fleet)，这是您创建令牌时做出的选择。

```
    usermod -aG docker dumbmonit
```

6. Plakar 备份：自动检测 — 发现 plakar 二进制或 kloset（每个用户的 ~/.config/plakar/stores.yml、~/.plakar、/var/lib/plakar）时，机器下会出现一个 Plakar 监控项，否则什么也不显示。在 agent.yaml 中设置"plakar_klosets"以监视固定列表，或"plakar: false"以选择退出。
7. 代理发现的服务（Docker、Plakar、restic/Borg、ZFS、libvirt、WireGuard、IPsec、速度测试）各自成为机器下的独立监控项，拥有自己的状态和警报。机器的"代理发现的服务"面板列出已发现和未发现的内容：取消勾选某个模块以删除其监控项，勾选则在其出现之前就监视它。参见[代理发现的服务](#services-found-by-the-agent)。

!!! note
    代理接触服务器，从不是相反的方式：机器上不需要打开任何端口。

=== "Linux（systemd 或 OpenRC）"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    脚本是 POSIX `sh`（BusyBox 和 dash 工作），下载机器架构（x86_64 或 aarch64）的二进制文件从服务器，写 `/etc/dumbmonit/agent.yaml`（模式 0600）并使用 systemd 注册 `dumbmonit-agent`，或在 Alpine 上使用 OpenRC。systemd 单元被强化（`ProtectSystem=strict`、`NoNewPrivileges`、`MemoryMax=128M`、`CPUQuota=20%`）。

=== "macOS（launchd）"

    与 Linux 上相同的命令，用 `sudo` 运行：

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    macOS 二进制文件 **不是** 在 DumbMonit 映像中：构建它们需要 Apple 的 SDK，其许可证禁止重新分发。它们附加到每个 [GitHub 发行版](https://github.com/laupernoe/dumbmonit/releases/latest) — `dumbmonit-agent-macos-aarch64`（Apple silicon）和 `dumbmonit-agent-macos-x86_64`（Intel），每个都有其 `.sha256` — 并且服务器重定向 `/download/dumbmonit-agent-macos-…` 那里，所以脚本为 Mac 的架构（`uname -m`）选择正确的。服务器不附带那些二进制文件，所以安装命令对它们不进行校验和：添加 `--sha256=macos-aarch64:<hex>`（或 `macos-x86_64`）带有发行版的 `.sha256` 的值，或安装程序拒绝下载。Mac 也需要到达 github.com。

    没有那个访问，或安装您已经有的二进制文件，下载它并用 `--bin` 传递它（首先用发行版的 `.sha256` 对比它：`--bin` 安装您给的）：

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    脚本在 `/usr/local/bin` 中安装二进制文件（并移除浏览器下载附带的隔离属性，这会使 Gatekeeper 拒绝启动它），写 `/usr/local/etc/dumbmonit/agent.yaml`（模式 0600）并在 `/Library/LaunchDaemons` 中注册系统守护进程 `com.dumbmonit.agent`，用 `launchctl bootstrap system` 启动。它在启动时运行，没有人登录。

    !!! note "更喜欢自己构建？"
        在 Mac 自己上，安装了 Rust：`cargo build --release -p dumbmonit-agent`，随后 `--bin=target/release/dumbmonit-agent`。那是发行版任务生成的相同二进制文件。

=== "FreeBSD（rc.d）"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    仅 x86_64。脚本下载 `dumbmonit-agent-freebsd-x86_64` 从服务器，写 `/usr/local/etc/dumbmonit/agent.yaml`（模式 0600）并安装 `/usr/local/etc/rc.d/dumbmonit_agent`，用 `service dumbmonit_agent enable` 启用。`daemon(8)` 监管代理，如果它停止会重启它，并写 `/var/log/dumbmonit-agent.log`。

    在 TrueNAS CORE 和其他基于 FreeBSD 的监控项上也相同工作，那里 ZFS 系列是练习的要点。

=== "Windows（服务）"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    脚本下载 `dumbmonit-agent-windows-x86_64.exe` 从服务器，对其 SHA-256 检查，在 `C:\Program Files\DumbMonit\dumbmonit-agent.exe` 中安装它，写 `C:\ProgramData\DumbMonit\agent.yaml`（仅 SYSTEM 和 Administrators 可读），并注册 Windows 服务 `DumbMonitAgent`（"DumbMonit 系统代理"）：在启动时自动启动，如 LocalSystem，并在失败后重启（10 秒、30 秒、随后 60 秒）。代理与服务管理器自己对话（`--service`），所以停止服务让它在退出前发送它仍然持有的东西。

    服务没有控制台：代理将其日志写到 `C:\ProgramData\DumbMonit\agent.log`，设置为 `agent.log.old` 超过 10 MB。

    没有本机 ARM64 构建：在 Windows 11 for ARM 上，脚本在 x64 仿真下安装 x86_64 二进制文件。

一个令牌可以注册几台机器：在一个组或一台机器之后命名它。

### 本机二进制文件 {#native-binaries}

代理是每个系统的单个静态可执行文件，旁边什么都不需要安装：

| 系统 | 文件 | 从哪里获取 |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | 服务器，`/download/…` |
| Linux aarch64（64 位 ARM：Raspberry Pi OS 64 位，ARM 服务器） | `dumbmonit-agent-linux-aarch64` | 服务器，`/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | 服务器，`/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | 服务器，`/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | GitHub 发行版（服务器重定向那里） |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | GitHub 发行版（服务器重定向那里） |

Linux 二进制文件静态链接反对 musl：它们在任何分发上运行，glibc 或不（Alpine、NAS 固件），没有要匹配的库。每个文件在旁边有其校验和，`<file>.sha256`，在 `sha256sum` 格式。每个 [GitHub 发行版](https://github.com/laupernoe/dumbmonit/releases/latest) 带有全六个二进制文件和它们的校验和，对于无法到达服务器的 `/download/` 的机器或用 `--bin` 安装。

自 0.1.0-alpha.9 起,发行版的每个二进制文件还使用 [Sigstore](https://www.sigstore.dev/) 进行了无密钥签名(`<file>.sigstore.json`),并附带 SLSA 来源证明。要验证某个二进制文件确实由本仓库的发行工作流构建:

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'
# or, with the GitHub CLI:
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

要自己构建 Linux、Windows 和 FreeBSD 二进制文件，从存储库的克隆（Docker 是唯一的要求）：

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### 管理服务 {#managing-the-service}

| | Linux（systemd） | Linux（OpenRC） | macOS（launchd） | FreeBSD（rc.d） | Windows（PowerShell） |
|---|---|---|---|---|---|
| 状态 | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| 日志 | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| 重启 | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| 停止 | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| 配置 | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| 服务文件 | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | 服务 `DumbMonitAgent`（`services.msc`） |

`--uninstall`（Windows 上的 `-Uninstall`）在每个系统上移除全部，包括服务。

!!! note "下载始终被验证 — 始终"
    DumbMonit 显示的安装命令进行服务器附带的每个代理二进制的预期 SHA-256（`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`，Windows 上的 `-Sha256 windows-x86_64:…`）。两个安装程序对它们下载的文件检查反对它在安装任何东西之前，并 **拒绝** 当它不匹配，当没有校验和为平台给出，或当没有 SHA-256 工具（`sha256sum`、`shasum`、`sha256`）可用。

    校验和随每个服务器发行版而改变：在升级 DumbMonit 后，从 **设置 → 代理** 复制一个新鲜安装命令（或从新令牌）而不是重新运行一个旧的，这会以"校验和不匹配"停止。相同校验和显示在安装命令下，并在 `/download/<file>.sha256` 发布。

    对于气隙或手动设置，`--insecure-skip-checksum`（`-InsecureSkipChecksum`）未验证安装一个下载的二进制文件并大声说所以；`--bin=PATH` 按原样安装一个本地文件，文件是您的验证。


### 安装程序标志 {#installer-flags}

| `install.sh` | `install.ps1` | 效果 |
|---|---|---|
| `--token=TOKEN` | `-Token` | 注册令牌（必需）。 |
| `--url=URL` | `-Url` | 服务器 URL，例如 `http://server:8080`。在 Linux 上，环境中的 `DUMBMONIT_URL` 在标志缺失时使用。 |
| `--interval=N` | `-Interval` | 采样期间秒数（默认 30）。 |
| `--services=a,b,c` | `-Services @('a','b')` | 报告其状态的服务，按主机系统命名方式命名：Linux 上的 systemd 单元，macOS 上的 launchd 标签（`com.apple.sshd`），FreeBSD 上的 rc.d 名称，Windows 服务。 |
| `--tags=key=value,…` | `-Tags @{key='value'}` | 标签，复制为每个系列上的 `tag_<key>`。 |
| `--hostname=NAME` | `-HostName` | 向服务器宣布的名称（默认：机器的）。 |
| `--bin=PATH` | `-BinPath` | 本地二进制文件而不是下载它。 |
| `--sha256=LIST` | `-Sha256` | 下载的预期 SHA-256，如 `platform-arch:hex` 对，以逗号分隔（裸十六进制值也有效）。包括在 DumbMonit 显示的命令中；除非给出下一个标志，否则必需。 |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | 未验证安装一个下载的二进制文件，带警告。仅对气隙或手动设置。 |
| `--no-start` | | 安装所有东西，但不接触服务器或启动服务（机器映像，测试）。 |
| `--uninstall` | `-Uninstall` | 卸载代理并删除其配置。 |
| `--help` | | 显示帮助。 |

### 升级 {#upgrade}

两个脚本都是幂等的：用相同令牌和 URL 再次运行安装命令，用新的二进制文件、重写配置并重启服务。那是升级程序。从当前服务器取命令（"设置 → 代理"）：其 `--sha256` 必须匹配您升级到的服务器版本的二进制文件。

相同命令迁移仍在运行 EzyMonit 代理的机器，在 Linux 和 Windows 上。安装程序停止并移除旧服务（`ezymonit-agent`，或 Windows 上的 `EzyMonitAgent`），将配置移动到新路径（`/etc/dumbmonit/agent.yaml`、`C:\ProgramData\DumbMonit\agent.yaml`）并在其位置安装 `dumbmonit-agent`。旧的 `ezym_…` 令牌保持有效，所以令牌创建时显示的命令仍然有效；什么都不需要首先被撤销。

## 代理发现的服务 {#services-found-by-the-agent}

机器自身的监控项显示机器本身：CPU、内存、文件系统、网络、磁盘健康（SMART）、传感器、操作系统健康以及你在 `services` 中列出的服务。代理检测到的其他一切都是**服务**，它发现的每个服务都会成为一个独立的监控项，类型为 `agent_service`，归档在机器之下：

| 服务 | 其指标 | 监控项名称 |
|---|---|---|
| Docker | `container_*` | `<machine> · Docker` |
| Plakar | `backup_*` | `<machine> · Plakar` |
| restic / Borg | `agent_backup_repo_*` | `<machine> · restic/Borg` |
| ZFS | `agent_zfs_*` | `<machine> · ZFS` |
| KVM / libvirt | `agent_libvirt_*` | `<machine> · libvirt` |
| WireGuard | `agent_wireguard_*` | `<machine> · WireGuard` |
| IPsec | `agent_ipsec_*` | `<machine> · IPsec` |
| 互联网速度测试 | `agent_speedtest_*` | `<machine> · Speed test` |

拆分在服务器上按指标名称进行，因此适用于所有代理版本。每个序列只写入一次，`target` 设置为它所属的监控项：服务的序列带有服务监控项的 id，机器的序列带有机器的 id。不会重复计数。

服务监控项有自己的状态、警报、规则、规则覆盖和静默。当代理停止报告它而机器仍在报告时（Docker 守护进程已停止、`wg` 已移除……），它会在与机器本身相同的三个周期缺失之后变为**不可达**。当机器沉默时，它的服务显示为隐藏在其后（`suppressed`），而不是发出自己的警报。新的服务监控项以机器的标签起步，因此基于标签的静默和规则继续适用。

安装之前无需选择。首次报告之后，机器页面会在**代理发现的服务**中列出每个模块：

- 已发现的服务处于勾选状态，并链接到其监控项；
- 未发现的服务处于未勾选状态；勾选它可仍然创建其监控项（在代理报告它之前它保持不可达——适合在你依赖的服务缺失时收到通知）；
- 取消勾选已发现的服务可删除其监控项并丢弃其读数。服务器在下一次确认中告知代理，代理停止收集它（旧版代理会继续收集，服务器继续丢弃）。删除服务监控项具有相同的效果。再次勾选即可恢复。

删除机器会删除其服务监控项。容器操作和策略从 Docker 监控项页面进行；命令仍发送到机器的代理。

与监控项提示相同：

1. 无需手动添加任何内容：在机器上安装代理，它发现的每个服务在首次报告时都会成为该机器下的监控项。
2. 要停止监视某个服务，请在机器的"代理发现的服务"面板中取消勾选，或删除其监控项：代理针对它的读数随后会被丢弃。

!!! note
    如果机器停止报告，它的服务显示为隐藏在其后，而不是发出自己的警报。

### 升级现有安装 {#upgrading-an-existing-installation}

无需任何操作。升级后代理首次报告某个服务时，服务器会创建其监控项，并把机器上与该服务有关的内容移到它上面：

- 读取该服务指标的规则的规则覆盖会复制到服务监控项；
- 限定于该机器的规则（`ids` 选择器）若读取该服务的指标，现在也会选中服务监控项；
- 机器的静默（维护窗口）会复制到服务监控项；
- 该服务序列上正在进行的警报会以新的指纹移到服务监控项，因此它们不会先解除再重新触发。

历史保留在写入之处：VictoriaMetrics 无法对现有序列重新打标签，因此升级前的读数仍留在机器之下（随保留期过期），服务监控项的图表从升级时开始。移动后的序列的异常基线重新开始学习。警报历史（已解除的警报）也留在机器处。

## 配置文件 {#configuration-file}

Linux 上的 `/etc/dumbmonit/agent.yaml`，macOS 和 FreeBSD 上的 `/usr/local/etc/dumbmonit/agent.yaml`，Windows 上的 `C:\ProgramData\DumbMonit\agent.yaml`：

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # 采样期间
hostname: nas-basement     # 可选：向服务器宣布的名称
services:                  # systemd 单元或 Windows 服务
  - sshd
  - docker
tags:                      # 自由标签，在服务器上带前缀"tag_"
  role: nas
  room: basement
docker: true               # 容器清单（默认：true）
docker_socket: /var/run/docker.sock
sensors: true              # 温度和风扇（默认：true）
smart: true                # 通过 smartctl 的磁盘健康（默认：true）
smart_bin: smartctl        # 路径，当它不在服务的 PATH 中
smart_interval_secs: 300   # 最小 60：睡着的磁盘永不被唤醒来被读取
zfs: true                  # 通过 zpool 的 ZFS 池（默认：true）
zfs_bin: zpool
zfs_interval_secs: 120     # 最小 30
libvirt: true              # 通过 virsh 的 KVM 域和存储池（默认：true）
libvirt_bin: virsh
libvirt_uri: qemu:///system
libvirt_interval_secs: 60  # 最小 15
wireguard: true            # WireGuard 隧道通过 wg（默认：true），见下文
ipsec: true                # 通过 swanctl 的 IPsec 隧道（默认：true），见下文
speedtest: false           # 互联网速度测试（默认：false，需主动启用），见下文
speedtest_download_url: https://speed.example.lan/backend/garbage.php?ckSize=50
speedtest_upload_url: https://speed.example.lan/backend/empty.php
restic_repos: []           # restic 存储库来监视，见"restic 和 Borg 备份"
borg_repos: []             # Borg 存储库来监视
mdaemon: auto              # Windows：MDaemon 队列当其服务存在时（true / false / auto）
hyperv: auto               # Windows：Hyper-V 计数器当 vmms 服务存在时（true / false / auto）
perf_counters:             # Windows：性能计数器来发布
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # "counter"标签的值；路径当省略时
max_buffered_samples: 20000
log_level: info
```

`sensors`、`smart`、`zfs` 和 `libvirt` 被检测，从不假定：在没有探测、没有 `smartctl`、没有池或没有 `virsh` 的机器上，留在 `true` 在每个期间花费一次查找并发出什么都没有。设置一个为 `false` 停止甚至查找 — 值得在 NAS 上做，其磁盘您宁愿 `smartctl` 永不接触，尽管代理已经传递 `-n standby` 以便睡着的磁盘仅在为其他原因清醒时被测量。

### 通过 libvirt 监视 KVM 虚拟机 {#kvm-virtual-machines-through-libvirt}

在 KVM 虚拟化主机上（装有 libvirt 的普通 Linux 主机，不是 Proxmox VE，后者有
[自己的集成](proxmox.md)），请把代理安装在主机本身，而不是客户机里。代理运行
`virsh -c qemu:///system`，使用 `list --all --name`、`dominfo --domain <名称>`、
`pool-list --all --name` 和 `pool-info --pool <名称>`：它从不定义、启动、停止或
迁移域，也从不更改存储池。对于由 `libvirt` 组成员而不是 root 运行的代理，显式的
`qemu:///system` URI 很重要：没有它，`virsh` 会读取该用户私有的、空的会话。
设置 `libvirt_uri` 可读取其他连接。

对于已关机的域，`agent_libvirt_domain_memory_used_bytes` 不存在：libvirt 只知道它
配置的最大值。存储池的大小来自 `virsh` 的人类可读输出，四舍五入到两位小数，因此
`agent_libvirt_pool_used_percent` 是一个近似值，足够接近，可以在存储池写满之前发出
警告。最多详细列出 200 个域和 64 个存储池。

旁边它，`/etc/dumbmonit/agent-secret`（Windows 上的 `C:\ProgramData\DumbMonit\agent-secret`）持有服务器交给这台机器的绑定密钥。代理用模式 `0600` 写它并在每次启动时重新读取它；它不是 `agent.yaml` 的一部分，因为它不是您设置的东西 — 参见 [绑定](#binding-one-machine-one-agent)。`DUMBMONIT_AGENT_SECRET_FILE` 移动它，这是您对一个容器中的代理想要的。

每个密钥可以通过环境被覆盖，这使代理在没有挂载文件的容器中可用：

| 变量 | 效果 |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | 配置文件的路径 |
| `DUMBMONIT_AGENT_URL` | 服务器 URL |
| `DUMBMONIT_AGENT_TOKEN` | 注册令牌 |
| `DUMBMONIT_AGENT_SECRET_FILE` | 在哪里保持绑定密钥（默认：`agent-secret`，下一个配置文件） |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | 采样期间 |
| `DUMBMONIT_AGENT_HOSTNAME` | 向服务器宣布的名称 |
| `DUMBMONIT_AGENT_SERVICES` | 要监视的服务，逗号分隔 |
| `DUMBMONIT_AGENT_TAGS` | `key=value`，逗号分隔 |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Docker 套接字路径 |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | 每个主机详细的容器，默认 `200`（`0`：仅计数） |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | 忽略的接口，逗号分隔名称或正则表达式；默认 `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | 保持的接口；当设置时，替换忽略列表 |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | 从文件系统和磁盘 I/O 中忽略的挂载点；默认跳过 `/var/lib/docker/`、`/run/…`、`/sys/`、`/proc/`、`/dev/`、`/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` 也发送每个核心一个 CPU 系列（默认 `false`） |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false`：温度和风扇（默认 `true`） |
| `DUMBMONIT_AGENT_SMART` | `true` / `false`：通过 `smartctl` 的磁盘健康（默认 `true`） |
| `DUMBMONIT_AGENT_SMART_BIN` | `smartctl` 的路径 |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | 两次 SMART 读取间的期间，默认 `300`，最小 `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false`：通过 `zpool` 的 ZFS 池（默认 `true`） |
| `DUMBMONIT_AGENT_ZFS_BIN` | `zpool` 的路径 |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | 两次 ZFS 读取间的期间，默认 `120`，最小 `30` |
| `DUMBMONIT_AGENT_LIBVIRT` | `true` / `false`：通过 `virsh` 的 KVM 域和存储池（默认 `true`） |
| `DUMBMONIT_AGENT_LIBVIRT_BIN` | `virsh` 的路径 |
| `DUMBMONIT_AGENT_LIBVIRT_URI` | libvirt 连接 URI，默认 `qemu:///system` |
| `DUMBMONIT_AGENT_LIBVIRT_INTERVAL_SECS` | 两次 libvirt 读取间的期间，默认 `60`，最小 `15` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false`：通过 `wg` 的 WireGuard 隧道（默认 `true`） |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | `wg` 的路径 |
| `DUMBMONIT_AGENT_IPSEC` | `true` / `false`：通过 `swanctl` 的 IPsec 隧道（默认 `true`） |
| `DUMBMONIT_AGENT_IPSEC_BIN` | `swanctl` 的路径 |
| `DUMBMONIT_AGENT_SPEEDTEST` | `true` / `false`：运行互联网速度测试（默认 `false`） |
| `DUMBMONIT_AGENT_SPEEDTEST_DOWNLOAD_URL` | 为测量下载速率而下载的地址 |
| `DUMBMONIT_AGENT_SPEEDTEST_UPLOAD_URL` | 为测量上传速率而向其 POST 的地址 |
| `DUMBMONIT_AGENT_SPEEDTEST_DOWNLOAD_BYTES_CAP` | 每次测试最多读取的字节数，默认 `50000000` |
| `DUMBMONIT_AGENT_SPEEDTEST_UPLOAD_BYTES` | 每次测试发送的字节数，默认 `10000000` |
| `DUMBMONIT_AGENT_SPEEDTEST_INTERVAL_SECS` | 两次测试之间的期间，默认 `21600`（6 小时），最小 `300` |
| `DUMBMONIT_AGENT_SPEEDTEST_MIN_DOWNLOAD_MBPS` | 警报阈值，下载速率（Mbps）（未设置：无警报） |
| `DUMBMONIT_AGENT_SPEEDTEST_MIN_UPLOAD_MBPS` | 警报阈值，上传速率（Mbps）（未设置：无警报） |
| `DUMBMONIT_AGENT_SPEEDTEST_MAX_LATENCY_MS` | 警报阈值，延迟（毫秒）（未设置：无警报） |
| `DUMBMONIT_AGENT_RESTIC_BIN` | `restic` 的路径 |
| `DUMBMONIT_AGENT_BORG_BIN` | `borg` 的路径 |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | 两次 restic 和 Borg 存储库读取间的期间，默认 `600`，最小 `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`、`false` 或 `auto`：MDaemon 的性能计数器（默认 `auto`，当 `MDaemon` Windows 服务存在时读取） |
| `DUMBMONIT_AGENT_HYPERV` | `true`、`false` 或 `auto`：Hyper-V 的性能计数器（默认 `auto`，当 `vmms` 服务存在时读取） |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Windows 性能计数器路径，以 **分号** 分隔（逗号可以是实例名称的一部分）；替换 `perf_counters` |
| `DUMBMONIT_AGENT_SYSTEM_HEALTH` | `true` / `false`：操作系统健康，接下来四个的主开关（默认 `true`） |
| `DUMBMONIT_AGENT_CHECK_UPDATES` | `true` / `false`：待处理更新（默认 `true`） |
| `DUMBMONIT_AGENT_UPDATES_INTERVAL_SECS` | 两次待处理更新检查间的期间，默认 `3600`，最小 `300` |
| `DUMBMONIT_AGENT_CHECK_REBOOT` | `true` / `false`：需要重启（默认 `true`） |
| `DUMBMONIT_AGENT_CHECK_FAILED_UNITS` | `true` / `false`：失败的 systemd 单元（默认 `true`） |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | 赶上缓冲的大小 |
| `DUMBMONIT_AGENT_LOG` | `trace`、`debug`、`info`、`warn`、`error` |
| `DUMBMONIT_AGENT_RELAY` | `true` 运行，对于服务器，分配给这个代理的监控项的探测（参见 [在 Docker / 另一网络中运行代理](#run-the-agent-in-docker--on-another-network)） |
| `DUMBMONIT_AGENT_SITE` | 当它被提供为中继时显示在代理旁的网站标签 |

## Windows 性能计数器 {#windows-performance-counters}

在 Windows 上，代理通过 PDH 库读取性能计数器 — 性能监视器（`perfmon`）显示的值 — 并每个采样期间发布它们。某些软件在其他地方不发布其状态。

**MDaemon。** 当名为 `MDaemon` 的 Windows 服务存在时，代理读取 MDaemon 性能对象：队列大小、会话、消息和垃圾总数、MDaemon 内部服务器的状态和其正常运行时间。系列、规则和显示它们的页面在 [MDaemon](mdaemon.md#mail-queues-through-the-agent) 页面上描述。`mdaemon: true` 即使没有服务（重命名的服务）也读取它们，`mdaemon: false` 永不。服务存在被再次检查每十个期间，所以代理在安装后被代理捕获而不需要重启。

**Hyper-V。** 当 Hyper-V 虚拟机管理服务（`vmms`）存在时，代理读取 Hyper-V 的性能对象；参见 [Hyper-V 主机](#hyper-v-hosts)。`hyperv: true` 和 `hyperv: false` 强制选择，如对 MDaemon。

**任何其他计数器。** 在 `perf_counters` 下列出其完整路径，如 `\Object(Instance)\Counter` 或 `\Object\Counter` 对于单个实例对象，它被发布为 `dumbmonit_agent_perf_counter{counter="…"}`，您给的名称或路径本身：

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- 计数器名称 **用英语写**，如它们在英语 Windows 的性能监视器中出现：代理用 `PdhAddEnglishCounterW` 添加它们，所以相同的 `agent.yaml` 在法语或德语 Windows 上也工作。
- 在 YAML 中将每个路径放在单引号之间，所以反斜杠和 `%` 保持为是。
- 一个畸形的路径在启动时用密钥和原因停止代理。Windows 不知道的路径（软件未安装，名称拼错）一次在日志中报告，`performance counter not available`，其他计数器仍然被读；每十个期间再试一次。
- 速率（`…/sec`）需要两次读取：它们从第二个期间开始出现。
- 最多 200 个计数器被读。在 Linux、macOS 和 FreeBSD 上，设置被接受什么都不被读。

要查看机器上的准确名称，在命令提示符中运行 `typeperf -q MDaemon`（或任何其他对象名称）。

## Hyper-V 主机 {#hyper-v-hosts}

安装在 Hyper-V 主机 — 带 Hyper-V 角色的 Windows Server 或启用 Hyper-V 的 Windows — 代理找到 `vmms` 服务并读取 Hyper-V 自己的性能计数器，什么都不需要配置。代理的监控项页面随后显示一个 Hyper-V 面板：多少虚拟机报告健康，主机的真实 CPU 负载，每个运行的 VM 持有的内存，以及每个虚拟磁盘的吞吐量、延迟和错误。

!!! warning "仅针对供应商文档验证 — 还未在真实系统上测试"

    计数器名称来自 Microsoft 的 Hyper-V 性能文档，并针对逻辑用模拟计数器源检查，而不是在运行的 Hyper-V 主机上。Windows 不知道的计数器一次在代理的日志中报告，其他的仍然被读。告诉我们什么破裂。

| 计数器 | 系列 | 标签 |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`、`…\Health Critical` | `hyperv_vms_health_ok`、`hyperv_vms_health_critical`：每个状态中的虚拟机 | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent`：主机的负载，客人包括 | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent`：运行客人花费的份额 | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions`：运行的 VM 加主机本身 | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`、`…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`、`hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent`：内存需求超过分配的内存；超过 100 VM 想要超过它有的 | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`、`…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`、`hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`、`…\Queue Length` | `hyperv_vhd_latency_seconds`（平均）、`hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`，一个计数器 | `disk` |

在 Hyper-V 主机上，`\Processor(_Total)\% Processor Time` 仅测量主机自己的分区，不是虚拟机消耗的：虚拟化程序的逻辑处理器计数器是告诉机器是否饱和的。`vm` 是虚拟机的名称如 Hyper-V Manager 显示它；`disk` 是 Windows 给每个虚拟磁盘的实例名称，源自其文件路径。`_Total` 实例被忽略，最多 200 VM 或磁盘每个计数器被读。

三个[内置规则](../alerting/rules.md#hyper-v)适用：

- **Hyper-V VM 健康关键**：Hyper-V 报告 VM 在关键健康五分钟（警告） — 最常见磁盘用尽空间。
- **Hyper-V 主机 CPU 高**：逻辑处理器超过 90% 忙十五分钟（通知）。
- **Hyper-V 虚拟磁盘错误**：虚拟磁盘在最后一小时报告 I/O 错误（通知）。

## 在 Docker / 另一网络中运行代理 {#run-the-agent-in-docker--on-another-network}

代理也作为容器映像发布，与服务器相同的标签：`ghcr.io/laupernoe/dumbmonit-agent:latest`（最后发行版）、`:edge`（`main` 上最后提交）。它是一个 `FROM scratch` 映像持有代理二进制文件和 Mozilla CA 包，完全由环境变量配置。存储库提供一个现成的 `docker-compose.agent.yml`：

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

容器看到什么，以及要挂载什么：

| 需要 | 做什么 | 为什么 |
|---|---|---|
| 主机的 CPU、内存、负载、磁盘 I/O | 什么都不做 | `/proc/stat`、`/proc/meminfo`、`/proc/loadavg` 和 `/proc/diskstats` 是任何容器内主机范围。 |
| 主机名 | `DUMBMONIT_AGENT_HOSTNAME` | 容器的主机名是一个随机 id；监控项会以它命名。 |
| 主机的容器 | `- /var/run/docker.sock:/var/run/docker.sock:ro` | 可选。只读列表和检查；重启/更新操作需要它可写和 `DUMBMONIT_AGENT_COMMANDS=true`。没有套接字，代理简单地报告没有容器。 |
| 文件系统使用 | `- /:/host:ro`（或您关心的挂载） | 代理报告容器内可见的挂载点，在它们的容器路径下。 |
| 主机网络计数器 | `network_mode: host` | 接口被隔离：没有它代理看到一个 `veth`。 |
| 通过这个代理中继的 Ping 监视器 | `sysctls: net.ipv4.ping_group_range: "0 2147483647"` | ICMP 使用此 sysctl 允许的 echo 套接字，无需 capability。参见 [无需 capability 的 ICMP ping](../install/docker.md#icmp-ping-without-a-capability)。 |
| 私有 CA | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | 内部反向代理后的服务器，自签名证书的监控项。 |
| 跨重新创建保留绑定 | `- agent-state:/var/lib/dumbmonit-agent` 和 `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | 绑定密钥在容器的文件系统中，`docker compose up --force-recreate` 丢弃。没有卷，代理回来未绑定，某人必须每次允许从监控项页面重新注册。 |

`pid: host` 和 `privileged: true` 永不需要：代理收集没有按进程的指标。

### 中继模式：监视另一个网站 {#relay-mode-monitor-another-site}

代理可以做超过监视自己的机器。使用 `relay: true`（或 `DUMBMONIT_AGENT_RELAY=true`），服务器 **委托探测** 到它：监控项其 *Reached through* 设置命名这个代理不是由服务器轮询而是由代理，从它自己的网络，带完全相同的收集器 — SNMP、Proxmox VE、Proxmox 备份服务器、Synology、HTTP、TCP、DNS、ping、TLS。结果在监控项下降临，如同服务器轮询了它：相同指标、相同 *最后探测*、相同警报。一个远程办公室、第二个家实验室或客户网络因此显示在一个您已经有的 DumbMonit 中，没有 VPN 和没有什么东西在远程端打开。

如何分步设置在 [监视远程网站](../install/remote-site.md) 中。

## 绑定：一台机器，一个代理 {#binding-one-machine-one-agent}

注册令牌说 *这台机器被允许对话*。它不说 *哪台* 机器正在对话 — 那是身份密钥（`/etc/machine-id`，或主机名）的作用，那个密钥不是秘密：它可以在任何机器上几秒钟内被读。

所以在第一批中服务器交给每台机器一个 **绑定密钥** 它自己的。它仅保持一个指纹；代理将密钥写到 `/etc/dumbmonit/agent-secret` 且模式 `0600` 并在之后每个请求的 `X-DumbMonit-Agent-Secret` 标头中呈现它 — 测量、容器命令和中继探测一样。从那以后：

- 没有其他机器可以在这个监控项下推送测量，或重写其主机名和操作系统；
- 没有其他机器可以获取 — 因此消费 — 对它队列的重启和更新命令。

监控项页面显示一台机器站立在哪里，在 **代理 → 绑定** 下：

| 状态 | 它意味着什么 | 要做什么 |
|---|---|---|
| **绑定** | 代理持有自己的密钥。 | 什么都不做。 |
| **未绑定 — 重新注册这个主机** | 主机在绑定存在前注册，并从未绑定自。服务器拒绝其测量、容器命令和中继探测。 | 在监控项页面点击 **允许重新注册**，随后重启代理。如果代理早于绑定，首先在主机上重新运行安装命令。 |

### 早于绑定的主机 {#hosts-that-predate-binding}

!!! warning "改变：未绑定主机被拒绝"
    早期发行版继续接受在绑定前注册的主机，只要它们使用已注册它们的令牌，和无声地绑定了它们一旦它们的代理被更新。那个转移窗口已关闭：永不绑定的主机现在 **在每个代理路由上被拒绝** — 测量、容器命令和中继探测 — 直到管理员重新注册它。一个队列令牌由许多机器共享，身份密钥可以被猜测，所以什么也不证明要求的代理是主机自己的。

    一个太旧无法接收绑定密钥的代理二进制文件也被拒绝，包括在其第一个注册上。

要带这样一个主机回来：

1. 如果其代理早于绑定，在它上面重新运行安装命令（代理二进制文件必须能够存储绑定密钥）。
2. 打开其监控项页面并点击 **代理 → 允许重新注册**。这打开一个一小时窗口。
3. 重启代理，或等待其下一次重试：那个批次用新鲜密钥绑定主机并关闭窗口。一个令牌而不是已注册主机的那个必须仍然能够注册（未用尽，未过期）。

直到那时，代理的日志进行服务器的解释：

```
WARN 这个代理对这台机器不被识别
     reason: 这台机器在代理绑定前被注册，并永不绑定
     到其代理安装，所以服务器不再接受它。重新注册
     这个主机：在 DumbMonit 中打开其监控项页面，点击
     '允许重新注册'，随后重启代理（如果代理比绑定更旧则首先重新运行安装命令）。
```

### 重新安装后重新注册 {#re-enrolment-after-a-reinstall}

如果机器被重建，其磁盘被替换，或其代理容器被重新创建没有容量持有密钥，代理回来没有它，服务器拒绝其批次。那个拒绝是故意的：一个被绑定的机器保持绑定，直到一个人说别的，否则绑定会保护什么也没有。

回来的方式在监控项页面：**代理 → 允许重新注册**。它打开一个一小时窗口，在这期间一个代理呈现一个有效注册令牌用新鲜密钥绑定那台机器；旧密钥停止工作，窗口一旦使用就关闭。代理在其间自己重试，所以在机器上什么都不需要重启 — 什么也不丢失，因为其读数保留缓冲，直到链接再次被接受。

直到某人打开那个窗口，代理的日志说什么要做：

```
WARN 这个代理对这台机器不被识别
     reason: 这台机器已被注册并绑定到另一个代理
     安装。如果您重新安装了它，在 DumbMonit 中打开其监控项页面并
     点击 '允许重新注册'，随后重启代理。
```

## 注册令牌 {#enrolment-tokens}

- 令牌在 **设置 → 代理** 中或在添加类型代理的监控项时创建。服务器仅保持一个指纹：清晰令牌在创建响应中显示一次，用两个安装命令。
- 代理在 `POST /api/ingest` 上以 `Authorization: Bearer dmon_…` 发送它。它永不在代理的日志中出现，即使被截断。
- **设置 → 代理** 列出令牌其前缀、作用域、创建日期和最后使用。**撤销** 在其下一次推送时停止每个使用那个令牌的代理；机器随后在三个错过的期间后变得 *不可达*。重新运行安装程序用新令牌来重新注册它们。
- 机器是分离的监控项：撤销令牌不删除它们。

### 单次使用或队列 {#single-use-or-fleet}

令牌的作用域在创建时被选择，它是一个选择，不是碰巧给您的默认值：

| 作用域 | 它注册什么 | 何时 |
|---|---|---|
| **单次使用**（默认） | 准确一台机器。 | 一个安装。安装命令活在 shell 历史中，在票中，在聊天日志中；一个单次使用的一旦使用值什么都没有。 |
| **队列可重复使用** | 您允许多少台机器 — 一个数字，或没有限制。 | 一个剧本，一个机器映像，一批安装。 |

两者都可以进行天的截止。截止仅停止 *注册*：已注册的机器在它通过后继续报告，这是您想要的安装窗口。撤销是什么切掉队列。

一个无法更多注册的令牌 — 用尽，或超过其截止 — 在"设置 → 代理"中显示为 **Enrols no more**。其机器继续报告；它仅无法让新的进来。

## 中断和诊断 {#outages-and-diagnostics}

当服务器不可达时，代理继续测量并用其原始时间戳在内存中缓冲其读数。它们一旦链接回来被发送：服务器重启在图表中留下没有洞。缓冲被限制（`max_buffered_samples`，默认约一小时），当满时丢弃最旧的读数。重试从 5 秒回退到 5 分钟带抖动。

```sh
dumbmonit-agent --dry-run          # 打印读数，发送什么都不
dumbmonit-agent --once             # 发送一批随后退出
journalctl -u dumbmonit-agent -f   # 服务日志（systemd）
tail -f /var/log/dumbmonit-agent.log   # 服务日志（OpenRC）
```

## 常见错误 {#common-errors}

| 症状 | 可能原因 |
|---|---|
| 安装程序说"无法到达服务器" | 错误的 URL 或令牌。配置被写：修复 `agent.yaml`，随后重启服务。 |
| 下载失败 404 | 服务器有没有代理二进制文件（`DUMBMONIT_AGENT_DIR` 空）。用 `--bin=PATH` 用您构建的二进制文件。 |
| 安装程序停止"校验和不匹配" | 命令在服务器升级前被复制（从"设置 → 代理"复制一个新鲜的），或接收的二进制文件不是服务器提供的一个：一个代理或缓存在途中，或被篡改的下载。 |
| 安装程序停止"没有预期的校验和" | 命令对这个平台没有 `--sha256`：一个旧命令，或 macOS 二进制文件（不由服务器附带）。复制一个新鲜命令，或传递 `--sha256=<platform>-<arch>:<hex>` 从发行版的 `.sha256`。 |
| 机器永不出现 | 推送到 `/api/ingest` 在安装命令中的 URL；在反向代理后面，确保它被转发以及身体达 16 MB 被允许。 |
| *不可达* 尽管代理运行 | 令牌被撤销，或推送被拒绝。检查 `journalctl -u dumbmonit-agent`：错误在那里被记录。 |
| 日志说"这个代理对这台机器不被识别" | 机器被绑定到代理安装，这个无法有其密钥 — 一个重新安装，或容器被重新创建没有其状态卷。打开监控项页面并点击 **允许重新注册**；参见 [重新安装后重新注册](#re-enrolment-after-a-reinstall)。 |
| 日志说"这台机器在代理绑定前被注册" | 一个主机永不绑定到其代理。在其监控项页面点击 **允许重新注册**，随后重启代理；参见 [早于绑定的主机](#hosts-that-predate-binding)。 |
| 日志说"这个代理太旧无法被绑定到其机器" | 代理二进制文件无法存储绑定密钥。在主机上重新运行安装命令；如果监控项已存在，首先点击 **允许重新注册**。 |
| 日志说"这个注册令牌已注册所有被允许的机器" | 一个单次使用令牌被重复使用。创建一个新的，或一个队列令牌在"设置 → 代理"中。 |
| Windows | Windows 服务部分在撰写本文时尚未在项目的构建映像中进行过练习；在 GitHub 上报告问题。 |

## Docker 容器 {#docker-containers}

当代理可以读取 Docker 套接字（`docker: true`，默认，及用户在 `docker` 组或 root — `usermod -aG docker dumbmonit`，随后重启代理），每个容器变成少数系列，标记为 `container` 和 `image`（`docker ps` 显示的 `name:tag`）。如果监控项页面说"此机器上没有 Docker"而 Docker 已安装，那是首先要检查的。

| 指标 | 含义 |
|---|---|
| `container_up` | 1 运行，0 别的 |
| `container_health` | 0 没有健康检查，1 健康，2 不健康，3 启动 |
| `container_restart_count` | 容器的 Docker 重启计数 |
| `container_started_seconds` | 自容器最后启动秒数（0 停止时） |
| `container_image_age_seconds` | 自图像被构建秒数 |
| `container_update_available` | 1 当注册表为相同标签有一个更新图像，0 当最新，-1 未知 |

更新检查对比本地图像的摘要与注册表为标签宣布的（`HEAD /v2/<name>/manifests/<tag>`，Docker 中枢和 GHCR 上的匿名令牌流）。它在背景中最多每小时每个图像运行一次，从不延迟 30 秒周期；一个拒绝匿名访问的私有注册表简单地读为 -1。用 `docker_update_check: false`（`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`）禁用它。

内置规则：**容器已停止**（`container_up == 0` 2 分钟），**容器不健康**（`container_health == 2` 3 分钟），**容器重启**（15 分钟内三个或更多重启），**容器更新可用**（信息，一小时后，每天重复）。警报命名容器。

### 操作 {#actions}

在监控项页面上，每个容器有两个操作和两个开关。操作通过一个 **命令通道** 旅行：服务器队列命令，代理在其下一批后获取它（`GET /api/agent/commands`，相同注册令牌），运行它，并报告结果（`queued → running → done | failed`），带一个短日志您可以在接口中展开。十分钟旧的命令永不执行；代理一次运行一个命令。

一个队列命令可以从接口 **取消** 只要代理尚未领取它。没有人来获取的命令 **过期** 十分钟后，在服务器上，无论代理是否曾轮询 — 所以一台代理被停止的机器永不保留一个陈旧"Queued"永远地，也不永不在那个容器上阻止下一个操作。

代理告诉服务器，带每批，无论它接受命令。当它没有 — 代理比命令通道更旧，或用 `commands: false` 安装 — 监控项页面而是说所以而不是显示重启和更新，API 拒绝命令（`409`），政策不队列什么都不。重新安装代理用当前安装程序来启用操作。

- **重启** — `docker restart` 对一个运行的容器，`docker start` 对一个停止的，随后一个 20 秒检查它运行。
- **现在更新** — 拉标签容器已在使用，用相同配置（环境、挂载、端口、标签、网络、重启政策）创建新容器，停止旧的，交换名称，启动新的并等待其健康检查（或 20 秒加当图像没有时运行检查）。如果什么都失败，代理 **回退**：新容器被移除旧的被重启，命令报告 `failed` 带原因。成功旧容器被移除，当"prune"打开时，旧图像太 — 除非另一个容器仍然使用它。

Compose 管理的容器（标签 `com.docker.compose.*`）被相同更新，但结果警告下一个 `docker compose up` 将从 compose 文件重新创建容器。名称以 `dumbmonit` 或 `ezymonit` 开始的容器被拒绝：监视器永不更新自己。

### 政策 {#policies}

按容器的 **政策** 由服务器每分钟被评估：

- **如果掉线重启** — 一个报告 `container_up == 0` 的容器被自动重启，最多每十分钟一次。
- **在维护窗口中自动更新** — 一个有可用更新的容器被自动更新，但仅在维护窗口（沉默）覆盖监控项时，最多每六小时。在警报页面上或从其页面沉默监控项来安排一个。政策默认为修剪旧图像和仅维护窗口。

两个地方设置它们：

- **监控项标头下的 Docker 条纹**："28 容器 · 27 运行 · 3 更新可用 · 政策：2 自动重启，0 自动更新"，最后操作和其状态，**管理容器**（打开下面的列表）和 **政策…**，每个容器的一个表带两个开关和一个"应用到全部"行 — 最快的方式为整个机器打开自动重启；
- **容器部分内的每一行中的开关**，在重启和现在更新按钮旁。

每个开关自己保存（`PUT /api/targets/<id>/containers/<name>/policy`）；失败仅关心那一行并显示在它旁边。

自动操作在"最近操作"中显示如 `policy` 要求。

### 安全规则 {#safety-rules}

- 代理仅当 `commands: true` 时行动（默认）；为必须仅被测量的机器将其设置为 `false`。
- 代理自己获取命令，用其注册令牌 — 什么都不连接到机器。它们在十分钟后过期并一次运行一个；代理永不拉不同的标签比容器已在使用。
- 不回来健康的更新被回退到先前容器，旧图像仅一旦新容器运行被移除。
- 名称为 `dumbmonit*` 或 `ezymonit*` 的容器被拒绝，手或通过政策：监视器永不在其自己的容器上行动。检查被做两次 — 服务器拒绝队列命令，代理拒绝运行它 — 和代理在通过检查容器 Docker 报告后在名称再次做它，所以用其 id 命名一个改变什么都没有。代理也拒绝在其自己的容器上行动，无论什么它被称为：它不会在那里完成工作或在其上报告。
- Compose 管理的容器被更新，但下一个 `docker compose up` 从 compose 文件重新创建它们：那里也凹陷标签。

## Plakar 备份 {#plakar-backups}

代理可以监视 [Plakar](https://plakar.io) klosets（存储库）：每十分钟，在背景，它运行 `plakar at <kloset> ls` 和 `info` 在每个上并每个周期重新发出最后读数。

Plakar 是 **检测，永不假定**。在启动时及随后在每个周期直到它发现什么，代理寻找 `plakar` 二进制文件（在 `PATH` 上，或 `plakar_bin`）及 klosets 在标准位置。当没有二进制文件、kloset 和痕迹的 Plakar（`~/.cache/plakar`），代理发出没有 `backup_*` 系列，在 `debug` 级别说所以一次，监控项页面显示没有 Backups 面板 — 什么都不需在没 Plakar 的机器上配置。要跳过检测完全：

```yaml
plakar: false                        # 或 DUMBMONIT_AGENT_PLAKAR=false
```

### 发现 {#discovery}

什么都不需要配置：当 `plakar_klosets` 为空，代理发现 klosets 自己，按这个顺序，并读每个位置仅一次：

1. 代理自己的配置目录 — `$XDG_CONFIG_HOME/plakar`，或 `~/.config/plakar` 的账户它运行下（`plakar_home` 当设置时）；
2. `/root/.config/plakar`；
3. `/home/*/.config/plakar`，字母顺序；
4. `/var/lib/plakar`，当那个目录存在时。

在每个目录它读 `stores.yml`（或 `stores.yaml`；遗产 `plakar.yml` 被接受太）并取 `stores` 地图的每个条目（也 `repositories` 和 `klosets`）那有一个 `location`。仅位置被读：密语和 rclone 秘密坐在它旁边永不被保留也不被记录。每个条目变成 kloset `@<name>`，用 `plakar -configdir <dir> at @<name>` 读，所以用户创建的 kloset 即使当代理以 root 运行时也被读。`~/.plakar` 的每个账户被添加当它存在时（`plakar create` 的默认 kloset）。两个账户不同 klosets 命名相同方式被区分为 `@name` 和 `<user>:@name`。

结果在 `info`（及再次仅当它改变时）在一次被记录。两个仪表，发出仅一旦 Plakar 被检测，让接口告诉剩余情况分开：`backup_plakar_present`（1 当二进制文件运行）和 `backup_klosets_found`（klosets 数被读）。监控项页面随后说"Plakar 被安装但没有 kloset 被发现"，或列出 klosets。当 klosets 存在但二进制文件不运行时（一个服务 `PATH` 没有 `plakar`，典型地），它们列为不可读，带提示来设置 `plakar_bin`，代理警告一次。

要监视一个固定列表而不，命名它 — 发现随后被跳过：

```yaml
plakar: true                         # 默认：检测 Plakar；false 完全隐藏它
plakar_bin: /usr/bin/plakar          # 默认："plakar" 在 PATH 上
plakar_klosets:                      # 明确列表：覆盖发现
  - /srv/backups/main
  - "@diskext"                       # 代理账户的命名存储
plakar_home: /home/noe               # 可选：HOME（及配置目录）当运行 plakar 时使用
plakar_interval_secs: 600            # 最小 60
```

| 变量 | 效果 |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — 当被检测时监视 Plakar 备份（默认 `true`） |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | `plakar` 二进制文件的路径 |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets，逗号分隔（覆盖发现） |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | 当运行 `plakar` 时使用的主目录 |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | 两次读取间的秒数 |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — 注册表检查容器更新 |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — 接受来自服务器的操作 |

一个加密的 kloset 需要其密语：放在存储条目（`plakar store set <name> passphrase=…`，这是什么 `plakar store add` 做）或导出 `PLAKAR_PASSPHRASE` 在代理的服务环境。没有它 kloset 读为"Cannot be opened"（`backup_last_status = 0`）。

### 指标和规则 {#metrics-and-rules}

指标，标记为 `kloset` 和 `source`（被备份的目录）：`backup_last_success_seconds`（最新快照的年龄），`backup_snapshot_count`、`backup_last_status`（1 当最新快照没有错误，0 当它有错误或 kloset 无法打开），`backup_size_bytes{kloset}`（存储库的存储大小），加 `backup_plakar_present` 和 `backup_klosets_found` 上面描述。没有什么都存在当 Plakar 未被检测，所以内置规则下面无法在没 Plakar 的机器上触发。

内置规则：**Plakar 备份太旧**（通知，超过两天没快照）和 **Plakar 备份失败**（警告）。监控项页面显示一个块每个 kloset — 最后备份的年龄、快照计数、大小、最后状态 — 及一行每个源，带一个"备份太旧"提示超过两天。

### 运行一个第一备份 {#run-a-first-backup}

使用 Plakar 1.1 在机器上安装（`plakar version`），如拥有备份的用户：

```sh
# 1. 一个 kloset 存储，命名所以代理在 ~/.config/plakar/stores.yml 中发现它
plakar store add diskext /mnt/diskext/plakar passphrase=…   # 或任何 Plakar 接受的位置
plakar at @diskext create                                   # 添加 -plaintext 来跳过加密

# 2. 一个第一备份，随后检查它
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` 和 `plakar at <path> backup <dir>` 用一个普通目录而不是 `@name` 相同工作；`plakar create` 不带 `at` 使用 `~/.plakar`，代理也发现。安排 `plakar at @diskext backup /srv/photos` 用 cron 或 systemd timer：代理报告最新快照的年龄，及当一天过去没快照时警报 — 两天对内置规则。

在本地尝试它用一个未加密的 kloset：

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

随后添加 `plakar_klosets: ["/tmp/plakar-demo"]` 到 `agent.yaml`（一个裸路径在家外不被发现）并检查用 `dumbmonit-agent --dry-run` 那 `backup_*` 样本出现。

## restic 和 Borg 备份 {#restic-and-borg-backups}

代理可以监视 [restic](https://restic.net) 和 [Borg](https://www.borgbackup.org) 存储库：每十分钟，在背景，它打开每个存储库您声明并报告无论它可以被读及其最新快照多旧。什么都不被发现：一个存储库没有标准位置，无法被打开不带其密码，所以每个一个被列在 `agent.yaml` 中，用获取其密码的方式。

**密码永不离开机器。** 代理交给它给 `restic` 或 `borg` 仅通过子进程的环境；它永不被记录，永不被发送给服务器，系列仅进行存储库的名称。

```yaml
restic_repos:
  - name: nas                                   # "repo"标签；默认：存储库，没凭证
    repository: sftp:backup@nas.lan:/srv/restic  # 作为 RESTIC_REPOSITORY 传递，永不在命令行
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # 可选：仅这个主机的快照（--host），对共享存储库
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # 或任何命令它打印它（RESTIC_PASSWORD_COMMAND）
    env:                                        # 此存储库仅的额外环境
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # 作为 BORG_REPO 传递
    password_file: /etc/dumbmonit/borg-offsite.pass        # 被代理读，作为 BORG_PASSPHRASE 传递
    glob_archives: "web-01-*"                   # 可选：仅这些存档（--glob-archives）
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # 默认：在 PATH 上
borg_bin: borg
backup_repos_interval_secs: 600                 # 最小 60
```

保留 `agent.yaml` 和密码文件可读仅由 root（`chmod 600`），如安装程序对 `agent.yaml` 做。`password_command` 地图到 `RESTIC_PASSWORD_COMMAND` 或 `BORG_PASSCOMMAND`。什么都工具从它们的环境可以读进入 `env`（`RESTIC_PASSWORD`、`BORG_PASSPHRASE`、云密钥、`BORG_RSH` 等），及代理自己的服务环境被传递通过太。

什么代理运行，最严格意义上的只读：

- `restic snapshots --json --latest 1 --no-lock --no-cache`：没有锁被写，所以一个只读或追加仅存储库可以被监视，及一个 `prune` 持有独占锁不让存储库看来掉线；也没缓存，因为安装的服务无法写到 `/root`。
- `borg --bypass-lock list --json --last 1`：不带 `--bypass-lock`，读会对 `borg create` 的整个时间失败，持有独占锁。Borg 写一个小安全记录在 `~/.config/borg` 下；当那个家不可写（安装的服务运行 `ProtectHome=yes`），代理指向 `BORG_BASE_DIR` 到其私有 `/tmp` 而不，除非您自己设置 `BORG_BASE_DIR`。一个 `keyfile` 模式中的存储库随后需要 `BORG_KEY_FILE` 在其 `env` 中。

因为服务运行 `ProtectHome=yes`，保留工具需要的在家目录外：密码文件、SSH 密钥和 `known_hosts` 在 `/etc/dumbmonit/` 下，如在例中，及本地存储库在 `/srv`、`/mnt` 或 `/var` 而不是 `/home`。读一个在另一个之后，在服务的内存限制内（128 MiB 与代理自己）。

指标，标记为 `tool`（`restic` 或 `borg`）和 `repo`：`agent_backup_repo_reachable`（1 当最后读成功，0 别的：存储库不可达，密码拒绝，二进制缺失 — 代理记录原因一次，再当它改变时），`agent_backup_repo_last_snapshot_age_seconds`（最新快照的年龄，缺失当存储库为空），及 `agent_backup_repo_last_snapshot_bytes`（字节最新 restic 快照处理，restic 0.17 及更晚）。用 `--latest 1`，restic 返回每个主机和路径集的最新快照；所有中最新被保留，所以在共享的存储库上设置 `host`。

内置规则：**restic/Borg 备份太旧**（超过 48 小时没快照）和 **restic/Borg 存储库不可达**（30 分钟）。监控项页面显示一个备份存储库面板，每个存储库一行。

要检查一个声明，运行代理一次在前景并寻找 `agent_backup_repo_*` 样本：

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## WireGuard 隧道 {#wireguard-tunnels}

在运行 WireGuard 的机器上，代理用 `wg show all dump` 在每个周期读每个接口和每个对等体。什么都不需要配置：不带 `wg` 命令（wireguard-tools）或不带接口，没系列被发出及没面板被显示。`wg` 需要 root（`CAP_NET_ADMIN`）；安装的服务有它，代理在您自己账户下启动手一次记录拒绝并报告什么都没有。

倾倒也持有接口的私钥和每个对等体的预共享密钥：代理跳过两个字段不带曾复制它们。

每个对等体，标记为 `interface`、`peer`（其公钥）和 `allowed_ips`：`agent_wireguard_peer_handshake_age_seconds`（自最新握手秒），`agent_wireguard_peer_has_handshake`（0 当对等体永不完成一个），`agent_wireguard_peer_keepalive_seconds`（其持久保活，0 当关闭），及计数器 `agent_wireguard_peer_rx_bytes` 和 `agent_wireguard_peer_tx_bytes`；每个接口，`agent_wireguard_interface_peers`。对于永不完成握手的对等体，年龄从代理首次看到它的瞬间计数：永不来上的隧道也沉默，被如此报告。

WireGuard 没对等体名称。在 `agent.yaml` 中给您的，通过公钥，它们变成 `name` 标签：

```yaml
wireguard: true                     # 默认；false 停止甚至查找
wireguard_bin: wg                   # 默认：在 PATH 上
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

内置规则 **WireGuard 对等体沉默** 在一个对等体 **有持久保活** 没完成握手超过 15 分钟时触发。当隧道上，WireGuard 每两分钟更新其握手，保活保持它上即使不带流量；一个对等体不带保活（一个电话，一个笔记本电脑）可能闲达为没什么破的天，所以它被忽略。在隧道上设置 `PersistentKeepalive = 25` 它们必须保持上（网站到网站链接，远程网关）。N 是规则的阈值：在 **警报 → 规则** 中改变它，或移除其查询的 `and on (…)` 部分来监视每个对等体。监控项页面显示一个 WireGuard 面板带每个对等体，自其最后握手的时间，其流量及其保活。

## IPsec 隧道 {#ipsec-tunnels}

在运行 strongSwan 的机器上，代理用两个只读命令读取每个已配置的隧道：`swanctl --list-conns`，用于在 `swanctl.conf` 中声明的 CHILD_SA（或通过 `swanctl --load-all` 读取 `ipsec.conf`，这是 strongSwan 自己的兼容层 — 代理不关心是哪一个写了配置），以及 `swanctl --list-sas`，用于实际已安装的 CHILD_SA。无需配置：没有 `swanctl` 命令或没有已配置的连接时，不会发出任何系列，也不会显示面板。代理从不发送 `--initiate`、`--terminate` 或任何会改变隧道状态的命令；它只读取 `swanctl` 已在命令行上显示的内容。

同时读取两个命令，而不只读活动的那个，才能让**从未**建立过的隧道被报告为宕机，而不是不可见：在配置中声明但不在 `--list-sas` 中的 CHILD_SA，从第一个周期起就算作宕机。

每个隧道，标记为 `connection`（IKE_SA 的名称）和 `tunnel`（CHILD_SA 的名称）：`agent_ipsec_tunnel_up`（1 已安装，否则 0），并且仅在隧道建立时提供计数器 `agent_ipsec_tunnel_bytes_in` / `agent_ipsec_tunnel_bytes_out` 和 `agent_ipsec_tunnel_rekey_seconds`（距下次重新协商密钥的剩余时间）。

```yaml
ipsec: true                # 默认；false 停止甚至查找
ipsec_bin: swanctl          # 默认：在 PATH 上
```

内置规则 **IPsec 隧道宕机** 在已配置的隧道十分钟内没有已安装的 CHILD_SA 时触发。监控项页面显示一个 IPsec 面板，列出每个隧道（建立或宕机）、其流量以及距下次重新协商密钥的时间。

## 互联网速度测试 {#internet-speed-test}

默认关闭：与代理测量的其他所有内容不同，此测试每次运行都会**消耗带宽** — 在按流量计费或有上限的连接上，还会消耗金钱。启用它需要同时设置 `speedtest: true` 和一个下载地址；没有该地址，即使 `speedtest` 为 `true` 也不会运行任何内容。

**不会为您选择服务器。** DumbMonit 不附带默认端点，除非您指定，否则从不联系第三方：由您决定您的线路对照谁的服务器测量，以及测试运行时谁能看到代理的地址。

测试是纯 HTTP，对远端只读：对 `speedtest_download_url` 的 `GET`，逐块读取，一读到 `speedtest_download_bytes_cap` 字节就停止（默认 50 MB，以先结束者为准），得出下载速率；如果设置了 `speedtest_upload_url`，向其 `POST` `speedtest_upload_bytes` 字节（默认 10 MB），得出上传速率；对下载地址的少量 `HEAD` 请求得出延迟（最小往返时间）和抖动（最小与最大之间的差距）。拒绝 `HEAD` 的服务器只会让延迟和抖动无法测量 — 测试的其余部分不依赖于它。

它适用于自托管的 [LibreSpeed](https://github.com/librespeed/speedtest) 后端（下载用 `backend/garbage.php?ckSize=N`，上传用 `backend/empty.php`）、[Cloudflare 的速度测试端点](https://speed.cloudflare.com/)（`https://speed.cloudflare.com/__down?bytes=N` 和 `https://speed.cloudflare.com/__up`，任何人无需账户即可调用），或您自己的任何普通 HTTP 文件和上传端点 — 无需实现特定供应商的协议，也没有超出代理已使用的 HTTP 客户端的新依赖。

```yaml
speedtest: false                    # 默认：需主动启用，此测试会使用带宽
speedtest_download_url: https://speed.cloudflare.com/__down?bytes=50000000
speedtest_upload_url: https://speed.cloudflare.com/__up
speedtest_download_bytes_cap: 50000000   # 默认；最多读取这么多字节
speedtest_upload_bytes: 10000000         # 默认；一次性发送
speedtest_interval_secs: 21600           # 默认：6 小时，最小 300
speedtest_min_download_mbps: 50          # 可选：启用"互联网下载速率低"
speedtest_min_upload_mbps: 10            # 可选：启用"互联网上传速率低"
speedtest_max_latency_ms: 80             # 可选：启用"互联网延迟高"
```

测试在后台运行，每 `speedtest_interval_secs` 最多一次（默认 6 小时，不少于 300 秒：更小的值会使代理在启动时停止）；每次代理启动后几秒也会运行一次测试；采集周期只读取其最后的结果，与 restic/Borg 和 Plakar 的读取完全一样。当下载本身失败（地址不可达、超时、HTTP 错误）时 `agent_speedtest_ok` 为 0 — 这是唯一会使测试完全无法测量任何内容的失败。

三个阈值选项有意设为可选：没有 `speedtest_min_download_mbps`，DumbMonit 无从知道"慢"对您的线路意味着什么，所以 **互联网下载速率低**、**互联网上传速率低** 和 **互联网延迟高** 保持沉默，直到您设置适用的那一个。每个阈值都作为自己的系列发布（`agent_speedtest_download_min_bytes_per_second`，等等），内置规则就是将测量值与它们比较；请在 `agent.yaml` 中改变阈值，而不是在 **警报 → 规则** 中。监控项页面显示一个速度测试面板，带有最近的下载和上传速率、延迟、抖动以及测试在多久之前运行。

## TLS 证书（Certbot / Let's Encrypt） {#tls-certificates-certbot--lets-encrypt}

代理不直接读 `/etc/letsencrypt`。每个 Certbot 更新的证书在某处被提供 — 一个 web 服务器，一个邮件服务器，一个反向代理 — 那就是 DumbMonit 读它的地方：添加一个 [TLS 证书](services.md#tls) 监视器（或一个 [网站或 Web API](services.md#http) 一个，对已那样被监视的网站）指向证书被实际呈现的端口。它读活证书，不是磁盘上的文件，所以它也抓到成功在 `/etc/letsencrypt` 中但永不被服务捡起的更新（一个反向代理需要重载来查看新文件是常见情况，准确是关键间隙）。

读 `/etc/letsencrypt/live/*/cert.pem`、更新配置及从代理 Certbot 日志被考虑及搁置：它会复制 TLS 监视器已报告的过期日期，当添加第二个、主机本地路径到相同信息 — 一件更多的东西在证书被移动或域被重命名后保持同步。一个主机 Certbot 运行但提供什么都不 DumbMonit 可以直接到达（一个内部 CA 步骤，一个 DNS 验证钩）是这一情况留下未覆盖；它也是情况其中一个 TLS 监视器无法帮助要么，因为什么都没在那个主机上对 DumbMonit 能探测对话 TLS。
