# VMware vSphere

A vCenter Server and the hosts it manages, or a standalone ESXi host: hosts
that stop responding, are left in maintenance mode or report a red health,
virtual machines and their VMware Tools, datastores filling up or becoming
inaccessible, and the alarms vSphere itself raised.

DumbMonit reads vSphere with a dedicated user holding the built-in
**Read-only** role. It never powers a VM on or off, never enters maintenance
mode and never acknowledges an alarm.

The failure worth watching: **a datastore that fills up**. When a thin
provisioned datastore reaches 100%, every VM that needs to write to it is
paused at once. The alarms vCenter raises say it too, but only to whoever has
the client open.

## Why the Web Services API

DumbMonit talks to the vSphere Web Services API (SOAP, `/sdk`), not to the
REST API (`/api/vcenter/*`): a standalone ESXi host has no REST API, and on a
vCenter the REST API does not tell maintenance mode, the overall health
(`overallStatus`), the state of the VMware Tools or the triggered alarms. One
view of the whole inventory is read in a single pass, page by page, and
released at the end.

The session is kept from one probe to the next and reopened only when it
expires: vCenter does not log a new login every minute.

## What it watches

All metrics are prefixed `dumbmonit_vsphere_`. Health values follow vSphere's
`overallStatus`: 0 green, 1 yellow, 2 red, 3 gray (unknown).

| Metric | What | Labels |
|---|---|---|
| `info` | value 1 | `product` (`vcenter` or `esxi`), `version`, `build` |
| `hosts` | hosts by connection state: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 connected, 1 disconnected by an administrator, 2 not responding | `host` |
| `host_power_state` | 0 powered on, 1 standby, 2 powered off, 3 unknown | `host` |
| `host_maintenance` | 1 in maintenance mode | `host` |
| `host_status` | overall health | `host` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | connected hosts only | `host` |
| `vms` | VMs by power state: `poweredOn`, `poweredOff`, `suspended` (templates excluded) | `state` |
| `templates` | VM templates | |
| `vm_power_state` | 0 powered on, 1 powered off, 2 suspended; at most 1000 VMs | `vm` |
| `vm_status` | overall health | `vm` |
| `vm_tools_status` | powered-on VMs only: 0 up to date, 1 old version, 2 installed but not running, 3 not installed | `vm` |
| `datastores` | datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 accessible, 0 inaccessible | `datastore` |
| `datastore_status` | overall health | `datastore` |
| `alarms` | triggered alarms not acknowledged, by color: `red`, `yellow` | `status` |
| `alarms_acknowledged` | triggered alarms already acknowledged | |
| `alarm` | one series per unacknowledged alarm, value = its health code, at most 64 | `alarm` (its name), `entity`, `entity_type`, `status` |
| `scrape_errors` | calls that failed on the last probe | |

The [built-in rules](../alerting/rules.md#vmware-vsphere) that apply: vSphere
host not responding, vSphere host health red, vSphere host left in
maintenance, vSphere datastore almost full, vSphere datastore inaccessible,
vSphere VM tools not running, vSphere red alarm, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries vSphere. It says what is wrong first (hosts not responding, red
alarms, datastores almost full or inaccessible), then the hosts with their
load, the datastores with their fill level, and the VMs by power state.

## How it was validated

Against the govmomi simulator `vcsim` 0.56.0, in vCenter and in ESXi mode:
login and refused password, session reuse and expiry, paging through the
inventory, a host not responding, a host in maintenance mode, a powered-off VM
and a template. The simulator raises no alarm: reading the triggered alarms is
validated against the documented `AlarmState` structure of the Web Services
API.

## Create a read-only user in vSphere

1. On a vCenter: Administration → Single Sign On → Users and Groups → Users, domain vsphere.local → Add. Name the user as follows and give it a long random password.

    ```
    dumbmonit
    ```

2. Still on the vCenter: Administration → Access Control → Global Permissions → Add. Pick that user, the Read-only role, and tick Propagate to children. Read-only sees every host, VM, datastore and alarm, and can change nothing.

3. On a standalone ESXi host instead: in the host client, Manage → Security & users → Users → Add user, with the same name. Then Host → Actions → Permissions → Add user: pick it, the Read-only role, and tick Propagate to all children.

4. In DumbMonit, enter the address of the vCenter or of the host, for example "vcenter.lan" or "esxi1.lan", with the user name as created: dumbmonit@vsphere.local on a vCenter, dumbmonit on a host. When a vCenter manages the hosts, add only the vCenter: it reports every host, VM and datastore.

5. DumbMonit only reads, through the vSphere Web Services API (/sdk). It never powers a VM on or off, never enters maintenance mode and never acknowledges an alarm.

!!! warning
    Do not reuse the single sign-on domain account or the host's built-in account: the Read-only role above is all DumbMonit needs. vCenter and ESXi use a self-signed certificate by default: enable "Accept an unverifiable certificate" unless the DumbMonit host trusts their certificate authority.

## Credentials

| Credential | Fields |
|---|---|
| User name / password | The dedicated user with the Read-only role: `dumbmonit@vsphere.local` on a vCenter, `dumbmonit` on a host. |

Address: a host name or IP (`vcenter.lan`), `host:port`, or a full URL; a
trailing `/sdk` or `/ui` is accepted. Options: port (443), certificate check,
request timeout (20 s) and triggered alarms (read).
