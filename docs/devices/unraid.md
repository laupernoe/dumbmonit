# Unraid

An Unraid server through its GraphQL API: an array that is not started,
disks disabled, missing or reporting read errors, parity checks that found
errors or have not run for weeks, cache pools filling up, containers that
should be running and are not, and the state of the VMs.

DumbMonit sends an API key with the Viewer role in the `x-api-key` header to
`/graphql`. The Viewer role reads everything and can change nothing. The API
is built into Unraid from 7.2; on 6.12 to 7.1 it comes with the Unraid Connect
plugin.

Every minute DumbMonit sends one required query (version, array, parity, data
and cache disks) and four optional ones, each on its own so that a field an
older API version lacks, or VMs turned off, only leaves out that part: the
parity check, the Docker containers, the VMs and the unread notifications.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from Unraid's API documentation and the
    published GraphQL schema of the Unraid API (field names, enumerations,
    sizes in kilobytes), not against a running server. Every query is a
    read. Tell us what breaks.

## What it watches

All metrics are prefixed `dumbmonit_unraid_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version` | Value 1: the Unraid version. |
| `array_started` | | 1 when the array is started. |
| `array_state_info` | `state` | Value 1: `started`, `stopped`, `new_array`, `recon_disk`, `disable_disk`… |
| `array_size_bytes`, `array_used_bytes`, `array_free_bytes`, `array_used_percent` | | Array capacity. |
| `disk_ok` | `disk`, `role` (`parity`, `data`, `cache`) | 1 when the disk's status is `DISK_OK`. Empty slots are left out. |
| `disk_status_info` | `disk`, `role`, `status` | Value 1: Unraid's status (`disk_ok`, `disk_dsbl`, `disk_np_missing`, `disk_invalid`…). |
| `disk_errors` | `disk`, `role` | Read errors since the array started (the Errors column). |
| `disk_temperature_celsius` | `disk`, `role` | Only while the disk spins: Unraid reports none for a sleeping disk. |
| `disk_spinning` | `disk`, `role` | 1 when the disk spins. |
| `disk_size_bytes`, `disk_fs_size_bytes`, `disk_used_percent` | `disk`, `role` | Size, file system size and fill, for disks with a file system. |
| `disks`, `disks_problem`, `disks_with_errors` | | Counts. |
| `parity_check_status_info` | `status` | Value 1: `completed`, `running`, `paused`, `cancelled`, `failed`, `never_run`. |
| `parity_check_running`, `parity_check_progress_percent` | | A check under way. |
| `parity_check_ok` | | After a finished check: 1 without sync errors, 0 with errors or when it failed. A cancelled check proves nothing and sets nothing. |
| `parity_check_errors`, `parity_check_age_seconds`, `parity_check_duration_seconds` | | The last check. |
| `containers`, `containers_running`, `containers_autostart_stopped` | | Docker containers. |
| `container_running`, `container_autostart_stopped` | `container` | Per container; the second is 1 when a container set to start with the array is not running. |
| `vms`, `vms_running` | | VMs. |
| `vm_state_info`, `vm_crashed` | `vm` (and `state`) | Per VM. |
| `notifications_unread` | `importance`: `alert`, `warning`, `info` | Unread notifications. |

The [built-in rules](../alerting/rules.md#unraid) that apply:

- **Unraid array stopped**: the array is not started for ten minutes (Warning).
- **Unraid disk disabled or missing**: a disk's status is not `DISK_OK` for
  five minutes (Warning). The array runs degraded.
- **Unraid disk read errors**: a disk reports read errors (Advisory).
- **Unraid disk too hot**: a data or parity disk above 55 °C for fifteen
  minutes (Advisory). Cache disks, often NVMe, are left out.
- **Unraid parity check found errors** (Advisory).
- **Unraid parity check overdue**: the last check is more than 40 days old
  (Advisory).
- **Unraid cache nearly full**: a cache disk more than 90 % full for thirty
  minutes (Advisory).
- **Unraid container stopped**: a container set to start with the array has
  not been running for ten minutes (Advisory).

## The device page

The Array panel says what is wrong in a sentence, shows the array fill, the
last parity check, the running containers and VMs, then lists every disk —
parity first, then data, then cache — with its status, temperature and fill,
and the containers that should be running.

## Create a read-only API key for DumbMonit

1. On Unraid 7.2 or later the API is built in. On Unraid 6.12 to 7.1, install the Unraid Connect plugin from the Apps tab first: it brings the same API, and no Unraid Connect sign-in is needed for local use.

2. Create an API key with the Viewer role, which reads everything and can change nothing: under Settings > Management Access > API Keys, or from the Unraid terminal.

    ```
    unraid-api apikey --create --name dumbmonit --roles VIEWER --description "DumbMonit monitoring" --json
    ```

3. Copy the key. In DumbMonit, enter the address of the Unraid server, for example "tower.lan", and paste the key as API key. If the web interface is served over HTTPS, choose https as protocol.

!!! warning
    Unraid reports no temperature for a disk that is spun down, and DumbMonit never wakes one: a sleeping disk simply has no temperature until it spins up.

## Credentials

| Credential | Fields |
|---|---|
| API key | A key with the Viewer role, sent as `x-api-key`. |

Address: the host name or IP of the Unraid server, `host:port`, or a URL.
Options:

- **Protocol** (`http`) and **Port** (80): those of the web interface. With
  SSL set to Strict under Settings > Management Access, use `https`, and the
  `myunraid.net` address if the certificate is that one.
- **Accept an unverifiable certificate**: for a self-signed certificate.
- **Timeout per request** (15 s).

## Troubleshooting

"Unraid refused the API key (401)": the key is wrong or was deleted.

"Unraid accepted the API key but refused to answer (403)": the key has no
role, or a role narrower than Viewer.

No container, VM or parity figure: that optional query failed (VMs turned off,
an older API version without the field); the rest is still read. The reason is
in the server log at debug level.
