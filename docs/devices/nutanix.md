# Nutanix AHV

Nutanix clusters, read through the v4 REST API of Prism Central. One Prism
Central covers every cluster registered to it.

Every minute DumbMonit reads, with HTTP basic authentication on port 9440:

| What | Path |
|---|---|
| Clusters | `/api/clustermgmt/v4.0/config/clusters` |
| Hosts | `/api/clustermgmt/v4.0/config/hosts` |
| AHV VMs | `/api/vmm/v4.0/ahv/config/vms` |
| Storage containers | `/api/clustermgmt/v4.0/config/storage-containers` |
| Fill level of each container | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Unresolved alerts | `/api/monitoring/v4.0/serviceability/alerts` |

Lists are read 100 items per page, up to 1,000 items each. The fill level and
the alerts are optional: if the account's role refuses them, the rest is still
collected. Nutanix declared the v1, v2.0 and v3 APIs deprecated; only v4 keeps
evolving, and it needs Prism Central pc.2024.3 or later.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from Nutanix's v4 API specifications (paths and
    fields as the official generated clients define them) and tested against
    recorded answers, not against a running Prism Central. Tell us what
    breaks.

## What it watches

All metrics are prefixed `dumbmonit_nutanix_`.

| Metric | Labels | Meaning |
|---|---|---|
| `clusters` | | Clusters, without Prism Central itself. |
| `cluster_info` | `cluster`, `version` | Value 1: the AOS version of each cluster. |
| `cluster_available` | `cluster` | 1 when Prism Central reaches the cluster. |
| `hosts` | | Hosts of every cluster. |
| `host_info` | `host`, `cluster`, `hypervisor` | Value 1: the hypervisor each host runs. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Capacity. |
| `host_degraded` | `host`, `cluster` | 1 when Nutanix marks the host as degraded. |
| `host_maintenance` | `host`, `cluster` | 1 when the host is entering or in maintenance mode. |
| `vms`, `vms_running` | | AHV VMs, and how many are powered on. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Value 1: `on`, `off`, `paused` or `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1 when the VM is powered on. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Sockets × cores × threads, and configured memory. |
| `storage_containers` | | Storage containers, without Nutanix's internal ones. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Fill level, from the latest statistics of the last thirty minutes. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Unresolved alerts, all and by severity. |

The [built-in rules](../alerting/rules.md#nutanix) that apply:

- **Nutanix host degraded**: a host is marked degraded for five minutes
  (Warning).
- **Nutanix storage container almost full**: a container is 90% full for
  fifteen minutes (Advisory).
- **Nutanix critical alerts**: Prism Central reports at least one unresolved
  critical alert for five minutes (Warning).

## Create a read-only user in Prism Central

1. In Prism Central, open Admin Center > IAM > Identities and add a local user with a long random password, named as follows.

    ```
    dumbmonit
    ```

2. Under Admin Center > IAM > Authorization Policies, create a policy with the built-in Prism Viewer role, for all entities, and assign it to that user. Prism Viewer reads clusters, hosts, VMs, storage containers and alerts, and changes nothing.

3. In DumbMonit, enter the address of Prism Central, for example "prism-central.lan", with that user name and password. One Prism Central reports every cluster registered to it.

4. DumbMonit only reads the v4 API (clustermgmt, vmm and monitoring, version 4.0): it never powers a VM on or off, never resolves an alert and never changes a storage container.

!!! warning
    The v4 API is served by Prism Central pc.2024.3 or later; a cluster managed by Prism Element alone, or by an older Prism Central, is not supported. Prism Central ships a self-signed certificate: enable "Accept an unverifiable certificate" unless the DumbMonit host trusts its certificate authority.

## Credentials

| Credential | Fields |
|---|---|
| User name and password | The `dumbmonit` local user with the Prism Viewer role. |

Address: the host name or IP of Prism Central, `host:port`, or a URL. Options:

- **Port** (9440).
- **Accept an unverifiable certificate**: Prism Central's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"Nutanix Prism Central refused the credentials … (401)": wrong user name or
password.

"Nutanix Prism Central has no /api/clustermgmt/v4.0/…": the address is a Prism
Element, or a Prism Central older than pc.2024.3.

No `storage_container_used_percent`: the role does not allow reading
statistics; the containers are still counted, with their configured capacity.
