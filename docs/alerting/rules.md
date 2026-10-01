# Rules

Rules are inserted at first start, then belong to you: DumbMonit never
rewrites a rule you changed, it only recreates missing ones. Built-in rules can
be edited and disabled, not deleted.

## Built-in rules

Severities are shown with the UI word; the API value is in parentheses. A rule
whose metric no device produces has no series and never fires, so every rule
below is shipped enabled on every instance.

### Every device

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Device unreachable | No measurement received for more than three minutes. `time() - tlast_over_time(dumbmonit_up[7d])` | > 180 s | 1 min | Warning (`critical`) | 6 h |
| High CPU | CPU load sustained above 90%, averaged per device across SNMP, Proxmox and agent sources; clears below 85 %. | > 90 % | 10 min | Advisory (`warning`), escalates after 1 h | 6 h |
| Unusual CPU | CPU load noticeably different from the usual at this time and day of the week (seasonal baseline). | score > 3.5 | 15 min | Info (`info`) | 6 h |
| Disk almost full | Filesystem 90% full or more, per mount point (SNMP, Proxmox storages and root filesystems); clears below 88 %. | ≥ 90 % | 15 min | Advisory (`warning`), escalates after 24 h | 6 h |
| Filesystem almost full | At this rate, the filesystem will be full within four days (`predict_linear` over 6 h; rising filesystems only). | ≥ 100 % | 30 min | Advisory (`warning`) | 6 h |

### UPS

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| UPS on battery | The UPS is powering the load from battery (`dumbmonit_ups_output_source == 5`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low | The UPS battery is low or depleted (`dumbmonit_ups_battery_status`, 3 = low, 4 = depleted). | ≥ 3 | 1 min | Warning (`critical`) | 30 min |

### Network ports (SNMP)

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Port accumulating errors | A network port logged more than fifty errors in the last hour: usually a failing cable, a dirty fibre or a duplex mismatch (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; nothing fires during the first hour of a new device, so errors counted before it was added are not reported as new). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| Port flapping | A network port went down and up again more than twice in thirty minutes (`changes_prometheus(dumbmonit_if_oper_status[30m])`). A port disabled on purpose has no series and never fires. | > 4 | 5 min | Advisory (`warning`) | 1 h |

### Server hardware (Redfish)

Health series are 0 (OK), 1 (Warning) or 2 (Critical). An empty slot (`Absent`) or a disabled one has no series, so a server delivered with one power supply out of two never fires.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Server fan failed | The management controller reports a fan as failed (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Server temperature above critical | A temperature sensor is above the critical threshold the controller itself declares (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Power supply redundancy lost | The power supplies are no longer redundant: one more failure and the server goes down (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 min | Advisory (`warning`), escalates after 1 h | 6 h |
| Power supply failed | The controller reports a power supply as failed (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |
| Drive failure predicted | A drive reports a predicted failure: replace it while it still works (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Server health critical | The controller reports the system's own health as Critical (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 min | Warning (`critical`) | 6 h |

### Uptime monitors and heartbeats

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Service down | The service has not responded correctly for three minutes (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 min | Warning (`critical`) | 30 min |
| Heartbeat missed | The job has not called in within its expected interval plus grace period, or reported a failure itself (`dumbmonit_probe_success{probe="push"} == bool 0`; the delay is set per device, see [Heartbeat](../devices/push.md)). | > 0 | 2 min | Advisory (`warning`) | 6 h |
| Service flapping | The service changed state more than six times in thirty minutes (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 min | Advisory (`warning`) | 1 h |
| Slow service | The service takes more than three seconds to respond. | > 3 s | 10 min | Advisory (`warning`) | 6 h |
| Certificate expiring soon | The certificate expires in less than fourteen days. | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Certificate expired | The certificate has expired. | < 0 d | 5 min | Warning (`critical`) | 24 h |

### Docker containers

Reported by the agent; see [Agent](../devices/agent.md). The alert names the
container.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Container stopped | The container has been stopped for two minutes (`dumbmonit_container_up == bool 0`). | > 0 | 2 min | Advisory (`warning`) | 1 h |
| Container unhealthy | The container's health check has been failing for three minutes (`dumbmonit_container_health == 2`; 0 none, 1 healthy, 3 starting). | > 0 | 3 min | Advisory (`warning`) | 1 h |
| Container restarting | The container restarted three times or more in fifteen minutes (`increase(dumbmonit_container_restart_count[15m])`): it is rarely stopped long enough for the previous rule. | ≥ 3 | 1 min | Advisory (`warning`) | 1 h |
| Container update available | A newer image is available in the registry for this container (`dumbmonit_container_update_available == 1`). | > 0 | 1 h | Info (`info`) | 24 h |

### Plakar backups

Reported by the agent, per kloset and per source. None of these series exists
while Plakar is not detected, so neither rule can fire on a machine without it.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Plakar backup too old | No new Plakar snapshot for this source for more than two days (`dumbmonit_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Plakar backup failed | The latest snapshot has errors, or the kloset cannot be read (`dumbmonit_backup_last_status < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |

### restic and Borg backups

Reported by the agent for each repository declared under `restic_repos` or
`borg_repos` in agent.yaml; see [Agent](../devices/agent.md#restic-and-borg-backups).
Without a declared repository there is no series, so neither rule can fire.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| restic/Borg backup too old | No new restic or Borg snapshot in this repository for more than 48 hours (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 h | 1 h | Advisory (`warning`) | 24 h |
| restic/Borg repository unreachable | The agent cannot read the repository: unreachable, password refused or binary missing; the agent's log says which (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### WireGuard tunnels

Reported by the agent; see [Agent](../devices/agent.md#wireguard-tunnels).

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| WireGuard peer silent | A peer with a persistent keepalive has not completed a handshake for more than 15 minutes: the tunnel is down (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). A peer that never connected counts its silence from the moment the agent first saw it. Peers without keepalive (a phone, a laptop) idle for days and are left out; change the threshold to change N, or drop the `and on …` part to watch every peer. | > 15 min | 5 min | Advisory (`warning`) | 6 h |

### Proxmox VE

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Backup too old | No successful Proxmox VE backup for more than seven days (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 d | 1 h | Advisory (`warning`) | 24 h |
| VM or container stopped | The guest was running within the last two hours and has been stopped for five minutes (`1 - dumbmonit_proxmox_guest_running`, and'd with `max_over_time(…[2h]) == 1`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| VM or container CPU high | A guest has used more than 90 % of its allocated cores for fifteen minutes (`dumbmonit_proxmox_guest_cpu_percent`). The notification names the guest, `nextcloud (202)`. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| VM or container memory high | A guest has used more than 95 % of its memory for ten minutes (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 min | Advisory (`warning`) | 6 h |
| VM or container disk almost full | The root disk of a guest is more than 90 % full (`dumbmonit_proxmox_guest_disk_used_percent`): containers always, VMs only when the QEMU guest agent reports usage. | > 90 % | 15 min | Advisory (`warning`) | 24 h |
| Guest locked | A VM or container has been locked for six hours (`dumbmonit_proxmox_guest_locked`, label `lock`): an interrupted backup or migration left the lock behind and no operation can run on the guest, not even the next backup. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| Old snapshot | The oldest snapshot of a guest is more than thirty days old (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 d | 1 h | Info (`info`) | 7 d |
| HA resource in error | A Proxmox HA resource is in error or fenced (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| HA manager not reporting | The HA local resource manager of a node has stopped writing its state (`dumbmonit_proxmox_ha_lrm_stale`): that node's HA guests will be neither restarted nor relocated. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Cluster lost quorum | The Proxmox cluster is not quorate (`1 - dumbmonit_proxmox_cluster_quorate`): guests can no longer be started. | > 0 | 1 min | Warning (`critical`) | 30 min |
| Proxmox node offline | A cluster node is reported offline (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 min | Warning (`critical`) | 30 min |
| Proxmox service down | A core Proxmox daemon is not running on a node (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, each counted only where the node enables it). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Node network interface down | An interface set to start at boot is not up on a node (`dumbmonit_proxmox_node_interface_offline`). An interface brought up on demand is never reported. | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Proxmox storage almost full | A Proxmox storage is more than 85% full (`dumbmonit_proxmox_storage_used_percent`); "Disk almost full" takes over at 90%. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Thin pool almost full | An LVM thin pool is more than 90 % full: every guest stored on it goes read-only when it fills (`dumbmonit_proxmox_node_thinpool_used_percent`); clears below 88 %. | > 90 % | 15 min | Warning (`critical`) | 6 h |
| Thin pool metadata almost full | The metadata volume of an LVM thin pool is more than 80 % full (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). It is far smaller than the data volume and often saturates first, with the same consequence. | > 80 % | 15 min | Warning (`critical`) | 6 h |
| Proxmox disk wearing out | An SSD or NVMe of a node has used more than 90 % of its rated life (`dumbmonit_proxmox_node_disk_wearout_percent`). The notification reads `pve1 · /dev/nvme0n1`. | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| Proxmox disk SMART failure | A disk of a node reports SMART health `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| Proxmox ZFS pool degraded | A ZFS pool of a node is not `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). The notification reads `pve2 · tank`. | > 0 | 5 min | Warning (`critical`) | 6 h |
| Backup job failed | The last vzdump job on a node, or a scheduled backup job, failed (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 min | Warning (`critical`) | 24 h |
| Backup job excludes a disk | A scheduled backup job skips a disk of one of the guests it covers (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): the job succeeds every night with that data missing. CD-ROM drives and cloudinit images are excluded by design and never counted. | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Replication failed | A replication job reports an error or a non-zero fail count (`dumbmonit_proxmox_replication_job_error`): the standby copy is stale. | > 0 | 10 min | Warning (`critical`) | 6 h |
| Ceph health error | Ceph reports `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 min | Warning (`critical`) | 30 min |
| Ceph health warning | Ceph reports `HEALTH_WARN` for more than fifteen minutes (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Ceph OSD down | A Ceph OSD has been down for five minutes (`1 - dumbmonit_proxmox_ceph_osd_up`). The notification names the OSD and its host. | > 0 | 5 min | Warning (`critical`) | 1 h |
| Ceph OSD nearly full | A single Ceph OSD is more than 85 % full (`dumbmonit_proxmox_ceph_osd_used_percent`): writes stop when it reaches its limit, whatever the cluster average says; clears below 82 %. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph pool almost full | A Ceph pool is more than 85 % full (`dumbmonit_proxmox_ceph_pool_used_percent`); clears below 82 %. | > 85 % | 15 min | Advisory (`warning`) | 24 h |
| Ceph noout flag left on | The Ceph `noout` flag has been set for two hours (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): rebalancing is disabled, and the cluster reads healthy while its redundancy shrinks. | > 0 | 2 h | Info (`info`) | 24 h |
| Proxmox updates pending | More than twenty package updates are pending on a node (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |
| Proxmox security updates pending | At least one pending update comes from a security archive (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| Proxmox packages changed | The installed version of a Proxmox package changed between two probes — someone ran `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, published for one hour with the detail in its `changes` label: `pve1 · pve-manager 8.2.4→8.2.7`). Resolves by itself an hour later. | > 0 | 1 min | Info (`info`) | 24 h |
| Node certificate expiring | A Proxmox node certificate expires in less than fourteen days (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |

### Proxmox Backup Server

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| PBS datastore almost full | PBS datastore more than 90% full. | > 90 % | 15 min | Advisory (`warning`), escalates after 24 h | 6 h |
| PBS datastore filling up | At the current rate, PBS estimates the datastore full within seven days. | < 7 d | 1 h | Advisory (`warning`) | 24 h |
| PBS datastore not mounted | A removable datastore is not mounted (`dumbmonit_pbs_datastore_removable_unmounted`): nothing can be written to it. | > 0 | 6 h | Advisory (`warning`) | 24 h |
| PBS backup too old | No new snapshot for this backup group (one per machine) for more than two days (`dumbmonit_pbs_backup_last_age_seconds`). Edit the threshold for machines backed up weekly; the notification names the group and datastore. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| PBS backup verification failed | Verification of the latest snapshot for this machine failed (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 min | Warning (`critical`) | 24 h |
| PBS task failed | At least one PBS task failed in the review window (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| PBS garbage collection too old | No successful garbage collection on this datastore for more than eight days (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| PBS garbage collection failed | The last garbage collection on a datastore failed (`dumbmonit_pbs_gc_last_run_ok < 1`; from `/gc` on PBS 3.3+, from the latest GC task before). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS corrupt chunks found | The garbage collection found unreadable chunks on a datastore (`dumbmonit_pbs_gc_bad_chunks`): some backups can no longer be restored. | > 0 | 10 min | Warning (`critical`) | 24 h |
| PBS prune job failed | The last run of a PBS prune job failed: nothing is pruned and the datastore keeps filling up (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). The notification names the job and datastore. | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS verification job failed | The last run of a PBS verification job failed (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 min | Advisory (`warning`) | 24 h |
| PBS sync job failed | The last run of a PBS sync job failed (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS tape backup failed | The last tape backup run failed (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): the offline copy is not being made. | < 1 | 10 min | Warning (`critical`) | 24 h |
| PBS job never ran | A job is enabled and scheduled but has never run (`dumbmonit_pbs_job_never_run`). | > 0 | 2 d | Advisory (`warning`) | 7 d |
| PBS service down | A service Proxmox Backup Server needs is not running (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); no backup can be taken in while it is down. | < 1 | 5 min | Warning (`critical`) | 6 h |
| PBS disk SMART failure | A disk of the backup server reports SMART health `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). The notification reads `/dev/sdb`. | > 0 | 5 min | Warning (`critical`) | 24 h |
| PBS SSD worn out | An SSD of the backup server has used more than 90 % of its rated endurance (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 h | Advisory (`warning`) | 7 d |
| PBS ZFS pool degraded | A ZFS pool of the backup server is not `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| PBS certificate expiring | The certificate served by the backup server expires in less than three weeks (`dumbmonit_pbs_node_certificate_expires_seconds`). Needs the `certificates` option. | < 21 d | 1 h | Advisory (`warning`) | 48 h |
| PBS restart pending after upgrade | The package was upgraded but the running daemon is still the old one (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 h | Info (`info`) | 7 d |
| PBS updates pending | More than twenty package updates are pending on the backup server (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Datacenter Manager

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Federated instance unreachable | The datacenter console cannot reach one of the Proxmox VE clusters or backup servers it federates (`dumbmonit_pdm_remote_reachable < 1`). The notification names the instance and its product. | < 1 | 10 min | Warning (`critical`) | 6 h |
| Federated instance behind | An instance runs an older version than another instance of the same product in the same estate (`dumbmonit_pdm_remote_version_behind`). Compared between instances, never against what Proxmox publishes. | > 0 | 1 h | Info (`info`) | 7 d |
| Task failed on a federated instance | At least one task failed on a federated instance in the review window (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Datacenter console disk almost full | The root filesystem of the datacenter console is more than 90 % full (`dumbmonit_pdm_node_rootfs_percent`): full, the console stops recording what it collects. | > 90 % | 15 min | Advisory (`warning`), escalates after 24 h | 6 h |
| Datacenter console certificate expiring | A certificate of the datacenter console expires in less than fourteen days (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Datacenter console updates pending | More than twenty package updates are pending on the datacenter console (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### Proxmox Mail Gateway

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Mail queue growing | The deferred mail queue has grown by more than a hundred messages in two hours (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): the next hop is probably refusing mail. Growth rather than size, because a busy gateway always keeps a few dozen deferred. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| Mail stuck in the queue | A message has been waiting in a queue for more than four hours (`dumbmonit_pmg_queue_oldest_age_seconds`). The age is a lower bound — qshape reports brackets — so the rule fires late rather than early. The notification names the queue. | > 4 h | 15 min | Warning (`critical`) | 6 h |
| Mail gateway service stopped | A service the gateway needs to accept and filter mail is stopped (`dumbmonit_pmg_service_running` for postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). A unit that is not installed has no series and cannot fire. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Virus signatures out of date | The ClamAV daily virus signatures are more than two days old (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): freshclam has stopped updating them and the gateway keeps filtering with last week's list. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Spam rules out of date | The SpamAssassin rules are more than eight days old (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). A channel that has never been updated has no date and no series. | > 8 d | 1 h | Advisory (`warning`) | 7 d |
| Quarantine filling up | The spam quarantine has taken in more than a thousand messages in six hours (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): a campaign, or a rule that just put a legitimate domain in the bin. | > 1000 | 30 min | Advisory (`warning`) | 12 h |
| Mail gateway cluster degraded | A gateway of the cluster is no longer in sync with the others (`dumbmonit_pmg_cluster_node_insync`): it filters with an out-of-date rule database. A standalone gateway has no series. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Mail gateway certificate expiring | A certificate of the mail gateway expires in less than fourteen days (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 d | 1 h | Advisory (`warning`) | 24 h |
| Mail gateway updates pending | More than twenty package updates are pending on the mail gateway (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 h | Info (`info`) | 7 d |

### MDaemon and SecurityGateway

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| MDaemon mail service down | A watched mail service of the MDaemon server does not answer, or greets to refuse service (`dumbmonit_mdaemon_service_up`). Only the services listed on the device have a series. The notification names the service and its port. | < 1 | 5 min | Warning (`critical`) | 6 h |
| MDaemon XML API not answering | The XML API has stopped answering while the mail services still do (`dumbmonit_mdaemon_api_up`): Remote Administration is probably stopped. Only a device with an account has the series. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| MDaemon mail queue growing | The inbound, local or remote queue has grown by more than a hundred messages in two hours (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): MDaemon has stopped delivering from it. Read by the [agent](../devices/agent.md#windows-performance-counters) on the MDaemon server, so the alert is on the agent's device. | > 100 | 15 min | Advisory (`warning`) | 6 h |
| MDaemon Bad queue not empty | Messages are waiting in the Bad queue (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): MDaemon could neither deliver nor bounce them, and they stay there until an administrator releases or deletes them. | > 0 | 30 min | Advisory (`warning`) | 24 h |
| MDaemon Retry queue high | More than fifty messages wait in the Retry queue (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): remote servers keep refusing mail temporarily. | > 50 | 1 h | Advisory (`warning`) | 6 h |
| SecurityGateway service down | A watched service of the gateway does not answer, or greets to refuse service (`dumbmonit_securitygateway_service_up`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| SecurityGateway API not answering | The REST API has stopped answering while the services still do (`dumbmonit_securitygateway_api_up`). Only a device with an API key has the series. | < 1 | 15 min | Advisory (`warning`) | 12 h |
| SecurityGateway delivery queue growing | The delivery queue has grown by more than a hundred messages in two hours (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): the mail server behind the gateway is probably refusing mail. The counter names come from the API as they are; if your version names the queue differently, adjust the matcher. | > 100 | 15 min | Advisory (`warning`) | 6 h |

### OPNsense

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Internet gateway down | dpinger no longer gets an answer from a gateway (`dumbmonit_opnsense_gateway_up`). On a multi-WAN firewall the traffic has already moved to another link, quietly. `loss` and `delay` are warnings, not failures, and do not fire this rule. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Gateway losing packets | A gateway is losing more than a fifth of the packets sent to it (`dumbmonit_opnsense_gateway_loss_percent`): the link answers, badly. Clears below 10 %. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| Gateway slow | A gateway's round-trip time has stayed above 300 ms (`dumbmonit_opnsense_gateway_delay_seconds`). Three hundred milliseconds leaves a mobile backup link alone. | > 300 ms | 15 min | Info (`info`) | 12 h |
| Firewall state table filling up | pf is tracking more than 80 % of the connections it is allowed (`dumbmonit_opnsense_pf_states_used_percent`). Past the limit the firewall drops new connections with no other symptom. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| Firewall network buffers exhausted | More than 90 % of the FreeBSD network buffers are in use (`dumbmonit_opnsense_mbuf_used_percent`). A firewall that runs out stops forwarding with the CPU still idle. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| VPN tunnel down | A configured VPN tunnel has had no session for ten minutes (`dumbmonit_opnsense_vpn_tunnel_up`). A plugin that is not installed, or a connection that is disabled, has no series and cannot fire. | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Firewall resolver stopped | Unbound has stopped answering (`dumbmonit_opnsense_unbound_running`). The firewall still routes; the machines behind it no longer resolve. Turn this rule off if your firewall resolves with something else. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| Firewall core service stopped | dpinger, which watches the gateways, or configd, which runs everything else, is stopped (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Only those two, because a service switched off on purpose is not a failure. | < 1 | 5 min | Warning (`critical`) | 6 h |
| Firewall too hot | A sensor reads above 85 °C (`dumbmonit_opnsense_temperature_celsius`): the fanless box in the cupboard is about to start throttling. Clears at 78 °C. | > 85 °C | 10 min | Advisory (`warning`) | 6 h |
| Firewall left in CARP maintenance mode | CARP has been left in persistent maintenance mode (`dumbmonit_opnsense_carp_maintenance_mode`): this firewall handed its virtual addresses to its partner and will not take them back on its own. | > 0 | 1 h | Info (`info`) | 24 h |
| Firewall reboot pending | An update is installed but needs a reboot to take effect (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 h | Info (`info`) | 7 d |
| Firewall updates pending | The firewall has packages waiting to be updated (`dumbmonit_opnsense_firmware_updates_pending`). It is the most exposed machine on the network. | > 0 | 1 h | Info (`info`) | 7 d |

### TrueNAS

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| NAS pool degraded | ZFS no longer calls a pool healthy (`dumbmonit_truenas_pool_healthy`). A degraded pool still serves its data, which is exactly why nobody notices until the next disk fails. The device page names the disk. | < 1 | 5 min | Warning (`critical`) | 6 h |
| NAS disk errors | Disks of a pool have reported read, write or checksum errors since the last `zpool clear` (`dumbmonit_truenas_pool_device_errors`, one series per kind). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| NAS pool almost full | A pool is more than 85 % full (`dumbmonit_truenas_pool_used_percent`). ZFS slows down well before it is full. Clears at 80 %. | > 85 % | 30 min | Advisory (`warning`), escalates after 7 d | 24 h |
| NAS scrub found errors | The last scrub of a pool found damaged data (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS scrub overdue | A pool has not completed a scrub in more than 45 days (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Absent after a resilver, which erases the record of the last scrub. | > 45 d | 1 h | Info (`info`) | 7 d |
| NAS disk failed its SMART test | A SMART self-test in a disk's log failed (`dumbmonit_truenas_disk_smart_failed`). A disk never tested has no series. | > 0 | 5 min | Warning (`critical`) | 24 h |
| NAS disk too hot | A disk is above 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, read from TrueNAS's cache). Clears at 50 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| NAS dataset near its quota | A dataset has used more than 90 % of its quota (`dumbmonit_truenas_dataset_quota_used_percent`): at 100 % its writes fail while the pool still has room. No quota, no series. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| NAS replication failed | An enabled replication task is in error (`dumbmonit_truenas_replication_error`): the copy on the other side is getting older. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshot task failed | An enabled periodic snapshot task is in error (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS snapshots stale | An enabled periodic snapshot task has not run for more than eight days (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): a task that stops running does not fail, it goes quiet. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| TrueNAS alert raised | TrueNAS itself has raised an alert of level ERROR or above (`dumbmonit_truenas_alerts`). Its own sentence is on the device page. | > 0 | 10 min | Advisory (`warning`) | 12 h |
| NAS service stopped | A service set to start with the NAS is stopped (`dumbmonit_truenas_service_running`). A service switched off on purpose has no series. | < 1 | 10 min | Advisory (`warning`) | 6 h |

### Synology DSM

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Synology disk SMART warning | DSM no longer reports this disk's SMART status as `normal` (`dumbmonit_synology_disk_smart_status`: 1 attention, 2 critical, unknown words count as 1). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology disk failed | DSM reports the disk as crashed or failed (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology disk bad sectors | The disk's bad-sector count exceeds the threshold set in DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 min | Warning (`critical`) | 24 h |
| Synology disk bad sectors growing | New unreadable sectors appeared in the last 24 hours (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Synology SSD wearing out | An SSD has less than 10 % of its rated life left (`dumbmonit_synology_disk_remaining_life_percent`); clears above 12 %. | < 10 % | 1 h | Advisory (`warning`) | 7 d |
| Synology volume degraded | A volume is degraded or crashed: its redundancy is gone (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 min | Warning (`critical`) | 6 h |
| Synology volume almost full | A volume is 90 % full or more (`dumbmonit_synology_volume_used_percent`); clears below 88 %. | ≥ 90 % | 15 min | Advisory (`warning`), escalates after 24 h | 6 h |
| Synology temperature high | A disk is above 55 °C, or DSM raised its own temperature warning (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); clears below 52 °C. | > 55 °C | 15 min | Advisory (`warning`) | 6 h |
| Synology memory high | Memory usage of the NAS above 95 % (`dumbmonit_synology_memory_usage_percent`); clears below 90 %. | > 95 % | 15 min | Advisory (`warning`) | 6 h |
| Active Backup task failed | The last run of an Active Backup for Business task failed, or backed up only part of its devices (`dumbmonit_abb_task_last_status == bool 0`). The notification names the task. | > 0 | 10 min | Warning (`critical`) | 24 h |
| Active Backup too old | No successful Active Backup for Business run for this task for more than two days (`dumbmonit_abb_task_last_success_seconds`). The series does not exist for a task that has never succeeded; "Active Backup task failed" speaks then. | > 2 d | 1 h | Advisory (`warning`) | 24 h |
| Active Backup task disabled | The task has no schedule, or its continuous backup is paused (`dumbmonit_abb_task_enabled == bool 0`): it will not back up anything until someone runs it. | > 0 | 1 h | Info (`info`) | 7 d |
| Active Backup device overdue | A device has gone longer without a successful Active Backup for Business run than its own rhythm allows, off-days excluded (`dumbmonit_abb_device_overdue`, see [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Active Backup device failing | The last two or more attempts of a device failed (`dumbmonit_abb_device_consecutive_failures`); a cancelled run counts as neither. | ≥ 2 | 10 min | Warning (`critical`) | 24 h |

### Log and metrics servers

VictoriaMetrics, VictoriaLogs, Loki and Graylog ([devices](../devices/victoriametrics.md)). Counters go through `increase_prometheus`, so a server added with a history of refused data does not fire on its first measurement.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Metrics or log storage read-only | Free disk space fell below `-storage.minFreeDiskSpaceBytes` and the storage refuses every write (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 min | Warning (`critical`) | 1 h |
| Metrics or log storage almost read-only | Less than 10% of the disk left before read-only (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); clears above 12 %. | < 10 % | 30 min | Advisory (`warning`) | 24 h |
| Metrics or log server unhealthy | `/health` does not answer OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 min | Warning (`critical`) | 6 h |
| Metrics or log data refused | Samples or log lines refused in the last hour: timestamps outside retention, labels too long, series limit (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| VictoriaMetrics slow inserts | More than 5% of new samples took the slow path over 15 minutes: not enough memory for the active series (`100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])`); clears below 3 %. | > 5 % | 30 min | Advisory (`warning`) | 24 h |
| Loki not ready | `/ready` does not answer ready (`dumbmonit_loki_ready`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Loki refusing log lines | Lines refused in the last hour: too old, rate or stream limit, line too long (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Loki cannot flush to storage | Chunks failed to reach storage in the last 30 minutes; they are lost if Loki restarts (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Loki write-ahead log disk full | WAL writes lost to a full disk (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Loki request errors | More than five requests answered with a server error in 15 minutes (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 min | Advisory (`warning`) | 6 h |
| Graylog not processing | Message processing paused on the node (`dumbmonit_graylog_processing`). | < 1 | 5 min | Warning (`critical`) | 6 h |
| Graylog search cluster down | OpenSearch or Elasticsearch red, or unreachable from Graylog (`dumbmonit_graylog_indexer_status`: 2 red, 3 unreachable). | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Graylog search cluster yellow | Replica shards unassigned for half an hour (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Graylog journal filling | The journal is more than half full: the output does not keep up (`dumbmonit_graylog_journal_used_percent`); clears below 40 %. | > 50 % | 10 min | Advisory (`warning`), escalates after 1 h | 6 h |
| Graylog input failed | An input failed to start (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Graylog indexing failures | Writes to the search cluster or message processing failed in the last hour (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### Self-hosted applications

Nextcloud, Immich, Paperless-ngx, Jellyfin and Plex ([devices](../devices/nextcloud.md)). They rarely fall over outright: the page opens while syncing, thumbnails, imports or library scans have stopped. These rules watch those states; an application that does not answer at all is Device unreachable.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Nextcloud stuck in maintenance | Maintenance mode is on (`dumbmonit_nextcloud_maintenance`): nobody can log in or sync. | > 0 | 30 min | Advisory (`warning`), escalates after 3 h | 6 h |
| Nextcloud database upgrade pending | The code was updated and Nextcloud waits for `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 min | Warning (`critical`) | 1 h |
| Nextcloud update available | Nextcloud's update check found a newer release (`dumbmonit_nextcloud_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Nextcloud OPcache full | PHP's OPcache is out of room and pages get slow (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Nextcloud free space low | Less than 5 GiB free for the data directory (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); clears above 6 GiB. | < 5 GiB | 30 min | Advisory (`warning`) | 24 h |
| Immich jobs stalled | Jobs wait in a queue that is not paused and runs nothing (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). The notification names the queue. | > 0 | 1 h | Advisory (`warning`) | 6 h |
| Immich queue paused | A job queue has been paused for a day (`dumbmonit_immich_queue_paused`). | > 0 | 24 h | Info (`info`) | 7 d |
| Immich storage almost full | The disk holding the library is more than 90 % full (`dumbmonit_immich_storage_used_percent`); clears below 88 %. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Immich update available | Immich's version check knows a newer release (`dumbmonit_immich_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Paperless task queue down | Redis or Celery in error (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 is an error): nothing new is imported. | ≥ 2 | 5 min | Warning (`critical`) | 1 h |
| Paperless index or sanity check error | The search index, the classifier or the sanity check (missing or altered files) reports an error (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 min | Advisory (`warning`) | 24 h |
| Paperless migrations pending | Database migrations left to apply (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 min | Advisory (`warning`) | 24 h |
| Paperless task failed | A task this user can see failed within the failed task window and was not dismissed (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 min | Advisory (`warning`) | 24 h |
| Paperless storage almost full | The disk holding the documents is more than 90 % full (`dumbmonit_paperless_storage_used_percent`); clears below 88 %. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| Paperless update available | Paperless-ngx's version check knows a newer release (`dumbmonit_paperless_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Jellyfin scheduled task failed | A scheduled task (a library scan, a cleanup) failed or was aborted on its last run (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 min | Advisory (`warning`) | 24 h |
| Jellyfin plugin broken | A plugin failed to load or does not support this version (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| Jellyfin restart pending | Jellyfin has waited a day for a restart after a plugin install or update (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 h | Info (`info`) | 7 d |
| Plex update available | Plex's update check found a newer release (`dumbmonit_plex_update_available`). | > 0 | 1 h | Info (`info`) | 7 d |

### Pi-hole

[Pi-hole](../devices/pihole.md) v6, read through its REST API.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Pi-hole blocking disabled | Blocking is off, paused or failed (`dumbmonit_pihole_blocking_enabled`). The hold lets a five-minute pause from the web interface go by. | < 1 | 15 min | Advisory (`warning`) | 6 h |
| Pi-hole blocklists stale | The blocklists (gravity) have not been rebuilt for more than eight days (`dumbmonit_pihole_gravity_age_seconds`): the weekly update fails. | > 8 d | 1 h | Advisory (`warning`) | 24 h |
| Pi-hole update available | A newer version is available (`dumbmonit_pihole_updates_available`): the image in Docker, core, web interface or FTL otherwise. Pi-hole checks once a day. | > 0 | 1 h | Info (`info`) | 7 d |
| Pi-hole diagnosis messages | Pi-hole lists messages on its diagnosis page (`dumbmonit_pihole_messages`): a blocklist that failed to download, a rate-limited client, an overloaded host. | > 0 | 15 min | Info (`info`) | 24 h |

### AdGuard Home

The DNS filter ([device](../devices/adguard.md)). Query and block figures are sums over AdGuard Home's own statistics window, not counters: no rule is built on them.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| AdGuard Home protection off | Protection disabled, or paused and not back yet (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 min | Advisory (`warning`) | 6 h |
| AdGuard Home DNS not running | The web interface answers but the DNS server is stopped (`dumbmonit_adguard_running`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| AdGuard Home upstream failing | An upstream DNS server fails AdGuard Home's own test, or stays silent 4 seconds (`dumbmonit_adguard_upstream_up`, one series per upstream). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| AdGuard Home filter lists stale | The oldest update of an enabled filter list is more than three days old (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 days | 1 h | Advisory (`warning`) | 24 h |
| AdGuard Home update available | A newer version is announced by AdGuard Home's own version check (`dumbmonit_adguard_update_available`); never fires when that check is disabled, as in the official Docker image. | > 0 | 1 h | Info (`info`) | 7 days |

### UPS behind NUT

UPS read from a [NUT server](../devices/nut.md). Every series carries the `ups`
label, so an alert names the UPS when one server publishes several. The status
flags are 0 or 1 at every read, so a rule clears by itself once mains power is
back. UPS read over SNMP are covered by the [UPS](#ups) rules above.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| UPS on battery (NUT) | The UPS is powering the load from battery (`dumbmonit_nut_ups_on_battery`, the `OB` flag of `ups.status`). | > 0 | 30 s | Warning (`critical`) | 15 min |
| UPS battery low (NUT) | The UPS reports a low battery, the `LB` flag (`dumbmonit_nut_ups_low_battery`): the machines it powers are about to shut down. | > 0 | 1 min | Warning (`critical`) | 30 min |
| UPS battery needs replacing (NUT) | The UPS asks for its battery to be replaced, the `RB` flag (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 h | Advisory (`warning`) | 7 d |
| UPS load high (NUT) | The load has stayed above 80 % (`dumbmonit_nut_ups_load_percent`); clears below 75 %. | > 80 % | 10 min | Advisory (`warning`) | 24 h |
| UPS runtime short (NUT) | The UPS estimates less than five minutes of runtime on battery (`dumbmonit_nut_battery_runtime_seconds`); clears above six. On mains power, a worn battery or a UPS too small for its load. | < 300 s | 5 min | Advisory (`warning`) | 24 h |
| UPS data stale (NUT) | The NUT server answers but has no fresh data from the UPS: cable, driver or UPS silent (`dumbmonit_nut_ups_data_stale`). An outage would go unseen meanwhile. | > 0 | 5 min | Advisory (`warning`), escalates after 1 h | 6 h |

### MikroTik RouterOS

Series carry the `interface` or `sensor` label, so a notification names the port or the sensor ([device](../devices/mikrotik.md)). Counters go through `increase_prometheus`: a router added with years of errors behind it does not fire on its first measurement.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| MikroTik CPU high | The router's CPU has stayed above 90 % (`dumbmonit_mikrotik_cpu_load_percent`): routing, the firewall or a script is saturating it. Clears below 80 %. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik memory high | More than 90 % of the memory is in use (`dumbmonit_mikrotik_memory_used_percent`). Clears below 85 %. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| MikroTik storage almost full | The flash storage is more than 90 % full (`dumbmonit_mikrotik_storage_used_percent`): configuration changes and upgrades may fail. Clears below 85 %. | > 90 % | 30 min | Advisory (`warning`) | 24 h |
| MikroTik temperature high | A temperature sensor reads above 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Clears at 75 °C. | > 80 °C | 10 min | Advisory (`warning`) | 6 h |
| MikroTik power supply or fan failed | A `psu*-state` or `fan*-state` sensor is not `ok` (`dumbmonit_mikrotik_health_ok`). A redundant power supply that died cuts nothing, until the second one does. | < 1 | 5 min | Warning (`critical`) | 1 h |
| MikroTik interface accumulating errors | More than fifty receive and transmit errors on an interface in the last hour (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 min | Advisory (`warning`) | 24 h |
| MikroTik link flapping | An interface lost its link more than twice in thirty minutes (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). A port unplugged for good counts once and does not fire. | > 2 | 5 min | Advisory (`warning`) | 1 h |
| MikroTik RouterOS update available | A newer RouterOS is available on the router's update channel (`dumbmonit_mikrotik_update_available`). Only known when the router checks for updates itself: see the optional step of the [setup](../devices/mikrotik.md). | > 0 | 1 h | Info (`info`) | 7 d |
| MikroTik firmware upgrade pending | RouterBOOT is older than the firmware bundled with the installed RouterOS (`dumbmonit_mikrotik_firmware_upgrade_pending`); it is upgraded only on request, then at the next reboot. | > 0 | 1 h | Info (`info`) | 7 d |

### UniFi Network

[Device page](../devices/unifi.md). Device series carry the device's name, model and type, so the notification says "Living room AP". A device waiting for adoption has no `device_up` series and never counts as offline.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| UniFi device offline | A UniFi device adopted by the controller is offline, isolated or failed to adopt (`dumbmonit_unifi_device_up`). | < 1 | 5 min | Advisory (`warning`), escalates after 1 h | 6 h |
| UniFi Internet down | The UniFi gateway reports no Internet connection (`dumbmonit_unifi_internet_up`). Without a UniFi gateway the series does not exist and the rule stays silent. | < 1 | 3 min | Warning (`critical`) | 1 h |
| UniFi WAN link down | A WAN link of the UniFi gateway is down (`dumbmonit_unifi_wan_link_up`): with two links, redundancy is lost while nothing seems broken. A disabled port has no series. | < 1 | 5 min | Advisory (`warning`) | 6 h |
| UniFi gateway CPU high | The UniFi gateway's processor has been above 90% for fifteen minutes (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); clears below 80 %. | > 90 % | 15 min | Advisory (`warning`) | 6 h |
| UniFi device waiting for adoption | A device appeared on the network and waits to be adopted in UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 min | Info (`info`) | 24 h |
| UniFi firmware update available | A firmware update is available for UniFi devices (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 h | Info (`info`) | 7 d |
| UniFi alarms | UniFi logged critical events or raised alarms in the last 24 hours (`dumbmonit_unifi_alarms`). Only read with a View Only account. | > 0 | 5 min | Advisory (`warning`) | 24 h |

### Home Assistant

[Device page](../devices/homeassistant.md).

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Home Assistant in recovery mode | Home Assistant started in recovery mode: its configuration could not be loaded and automations do not run (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| Home Assistant not running | Home Assistant answers but has not finished starting for ten minutes (`dumbmonit_homeassistant_running`). | < 1 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant entities went unavailable | At least five more entities are unavailable than an hour ago: an integration, a hub or a radio stopped (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). The rise counts, not the total, which a real installation never has at zero. | ≥ 5 | 10 min | Advisory (`warning`) | 6 h |
| Home Assistant battery low | A battery reported by Home Assistant is low (`dumbmonit_homeassistant_battery_low`, one series per battery below the device's threshold, 20% by default; its value is the level). | ≥ 0 % | 30 min | Info (`info`) | 3 d |
| Home Assistant update available | Home Assistant has updates waiting: core, operating system, add-ons or device firmware (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 h | Info (`info`) | 7 d |
| Home Assistant repair to address | Home Assistant has an open repair of error or critical severity (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 min | Advisory (`warning`) | 24 h |

### VMware vSphere

[Device page](../devices/vsphere.md). Host series carry `host`, VM series `vm`, datastore series `datastore`, so the notification names the object.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| vSphere host not responding | A vSphere host is not responding to vCenter (`dumbmonit_vsphere_host_connection_state`: 2 not responding). A host disconnected on purpose (1) does not fire. | ≥ 2 | 3 min | Warning (`critical`) | 1 h |
| vSphere host health red | vSphere reports a host's overall health as red (`dumbmonit_vsphere_host_status == 2`). Gray means unknown and does not fire. | > 0 | 5 min | Warning (`critical`) | 1 h |
| vSphere host left in maintenance | A vSphere host has been in maintenance mode for a day: its VMs were moved away and it may have been forgotten there (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 h | Info (`info`) | 24 h |
| vSphere datastore almost full | A vSphere datastore is 90% full or more (`dumbmonit_vsphere_datastore_used_percent`); clears below 85 %. | ≥ 90 % | 15 min | Advisory (`warning`), escalates after 24 h | 6 h |
| vSphere datastore inaccessible | A vSphere datastore is inaccessible: the VMs stored on it cannot run (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| vSphere VM tools not running | VMware Tools are installed but not running in a powered-on VM: the guest may be hung (`dumbmonit_vsphere_vm_tools_status == 2`). VMs without Tools do not fire. | > 0 | 30 min | Info (`info`) | 24 h |
| vSphere red alarm | vSphere raised a red alarm that nobody acknowledged (`dumbmonit_vsphere_alarm{status="red"}`, one series per alarm, named after the alarm and the object). | > 0 | 5 min | Advisory (`warning`) | 6 h |

### Databases, message broker and security engine

Redis / Valkey, MongoDB, RabbitMQ and CrowdSec ([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md), [RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)). Counters go through `increase_prometheus` or `changes_prometheus`, so nothing fires on a device's first measurement.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Redis memory near maxmemory | Memory used above 90 % of `maxmemory` (`dumbmonit_redis_memory_used_percent`); clears below 85 %. No series without `maxmemory`. | > 90 % | 10 min | Advisory (`warning`) | 6 h |
| Redis refusing connections | Client connections refused in the last 15 minutes: `maxclients` reached (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 min | Warning (`critical`) | 1 h |
| Redis replication link down | A replica lost its link to the primary (`dumbmonit_redis_master_link_up`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| Redis replica lagging | Seen from the primary, a replica has not acknowledged for more than 30 seconds (`dumbmonit_redis_replica_lag_seconds`). | > 30 s | 5 min | Advisory (`warning`) | 6 h |
| Redis snapshot failed | The last `BGSAVE` failed; with `stop-writes-on-bgsave-error`, writes are refused (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| Redis append-only file write failed | The last AOF write failed (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replica set without primary | No member is primary: no write is accepted (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| MongoDB member unreachable | A replica set member is unreachable from the others (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| MongoDB replication lag | A secondary is more than a minute behind the primary (`dumbmonit_mongodb_replset_member_lag_seconds`); clears below 30 s. | > 60 s | 10 min | Advisory (`warning`) | 6 h |
| MongoDB connections near the limit | More than 80 % of the available connections in use (`dumbmonit_mongodb_connections_used_percent`); clears below 75 %. | > 80 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB cache under pressure | More than 20 % of the WiredTiger cache is dirty (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); clears below 15 %. | > 20 % | 10 min | Advisory (`warning`) | 6 h |
| MongoDB internal errors | Regular assertions in the last hour (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 min | Advisory (`warning`) | 24 h |
| RabbitMQ memory or disk alarm | A node raised a resource alarm: every publisher is blocked (`dumbmonit_rabbitmq_alarm`, one series per node and resource). | > 0 | 1 min | Warning (`critical`) | 1 h |
| RabbitMQ node down | A cluster node is not running (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ network partition | A node sees a network partition (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 min | Warning (`critical`) | 1 h |
| RabbitMQ queue without consumer | A queue holds messages and has no consumer (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); the value is the number of messages waiting. | > 0 | 15 min | Advisory (`warning`) | 6 h |
| RabbitMQ queue unavailable | A queue is not running: quorum queue without majority, or its node is down (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec Local API down | The Local API does not answer `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 min | Warning (`critical`) | 1 h |
| CrowdSec bouncer stopped pulling | A bouncer's requests did not change in 30 minutes (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 min | Advisory (`warning`) | 6 h |
| CrowdSec reads no logs | Not a single log line read in six hours (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 min | Advisory (`warning`) | 24 h |

### Kubernetes

See [Kubernetes / k3s](../devices/kubernetes.md). Every series is labelled
with the node, or the namespace and the pod, workload or claim concerned.

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| Kubernetes node not ready | A node is not Ready: its kubelet no longer reports, and its pods will be evicted (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 min | Warning (`critical`) | 6 h |
| Kubernetes node under pressure | A node reports memory, disk or PID pressure (`dumbmonit_k8s_node_pressure`, label `condition`). | > 0 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes pod crash looping | A container is in `CrashLoopBackOff` (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m])`: the state disappears for a moment at each restart attempt). | > 0 | 10 min | Warning (`critical`) | 6 h |
| Kubernetes pod stuck pending | A pod has been Pending for fifteen minutes: no node can take it, or a volume or an image it needs does not come (`dumbmonit_k8s_pod_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |
| Kubernetes pod restarting | A pod restarted more than five times in the last hour without being in `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 min | Advisory (`warning`) | 6 h |
| Kubernetes workload missing replicas | A Deployment, StatefulSet or DaemonSet has fewer available replicas than it asks for (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 min | Advisory (`warning`), escalates after 1 h | 6 h |
| Kubernetes volume claim pending | A PersistentVolumeClaim that a pod needs, or that a provisioning error concerns, is still not bound (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 min | Advisory (`warning`) | 6 h |

### DumbMonit itself

| Rule | What | Default threshold | Hold | Severity | Reminder |
|---|---|---|---|---|---|
| DumbMonit backup did not run | The scheduled local backup of the DumbMonit database has not run for more than two days (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 d | 1 h | Advisory (`warning`) | 24 h |

Every built-in rule applies to all devices and to every enabled channel.

## Editing a rule in the UI

On the **Alerts** page, the *Rules* section lists every rule with its
severity, a *Built-in* mark and, when relevant, how many alerts it is firing
now. Each rule has:

- a toggle to **enable or disable** it;
- **Edit**, which unfolds an inline editor for name, description, operator,
  threshold, hold (`for`), severity, reminder (`repeat every`) and escalation
  (`escalate after`). The query of a built-in rule is shown but not editable;
  the query of your own rules is. Baseline rules show their detection
  parameters read-only;
- **Delete**, for your own rules only.

## Creating a threshold rule

Click **New rule**. The form needs:

| Field | Meaning |
|---|---|
| Name | Shown in alerts and notifications. |
| Query | A MetricsQL expression, evaluated per device. Example: `dumbmonit_cpu_usage_percent`. |
| Operator | `>`, `>=`, `<` or `<=`. |
| Threshold | The number the query result is compared with. |
| Severity | Info, Advisory or Warning. |
| Hold for | Seconds the condition must last before firing. A non-zero hold absorbs an isolated bad sample. |

Every series returned by the query becomes one potential alert, attached to the
device named by its `target` label. Aggregate with `avg by (target, host) (…)`
when you want one alert per device rather than one per core or per interface.
The alert's identity ignores the `host` and `tag_*` labels whenever `target` is
present, so renaming a device keeps its alerts; a series whose `target` does
not match an existing, enabled device is ignored.

## The query language

Queries are [MetricsQL](https://docs.victoriametrics.com/metricsql/), the
PromQL superset of VictoriaMetrics. The metrics are the `dumbmonit_*` series
described in the [metrics reference](../reference/metrics.md), and every series
carries a `target` label (the device id) and a `host` label (its name).

Some patterns used by the built-in rules:

```
# Age of the last sample: what "unreachable" means
time() - tlast_over_time(dumbmonit_up[7d])

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

!!! tip "Try a query first"
    `GET /api/metrics/query?query=…` returns what the rule would evaluate
    (see the [API reference](../reference/api.md#metrics)). In development, the
    overlay publishes the embedded VictoriaMetrics on the host's `:8428`, with
    its query UI.

## Rule kinds in the API

`kind` is `threshold` (compare with a threshold), `predict` (same comparison,
on a `predict_linear` query; shown as a forecast) or `anomaly` (seasonal
baseline; `params` holds `k`, `alpha`, `mad_floor_abs`, `mad_floor_rel`,
`min_samples`). The UI creates `threshold` rules; the other two kinds can be
created through the API.
