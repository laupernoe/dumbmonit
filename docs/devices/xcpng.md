# XCP-ng / XenServer

An XCP-ng or XenServer (Citrix Hypervisor) pool, read through XAPI, the
management API every host already runs. No Xen Orchestra is needed.

Every minute DumbMonit posts JSON-RPC calls to
`https://<coordinator>/jsonrpc`: one `session.login_with_password` (the
session is then kept from one poll to the next, and reopened once if the pool
invalidates it), and one `get_all_records` per class: `host`, `host_metrics`,
`VM`, `VM_metrics`, `SR` and `pool`. Nothing is ever changed: the account has
the built-in `read-only` role, which XAPI enforces on every call.

Add the pool once, by the address of its coordinator: it reports every host,
VM and storage repository. If the address you entered is a pool member, it
answers `HOST_IS_SLAVE` with the coordinator's address; DumbMonit follows it
once, same protocol and port, so a coordinator change does not need you to
edit the device.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the XAPI documentation (the JSON-RPC wire
    protocol, the `host`, `VM`, `SR` and `pool` classes) and tested against
    recorded answers, not against a running pool. Tell us what breaks.

## What it watches

All metrics are prefixed `dumbmonit_xcpng_`.

| Metric | Labels | Meaning |
|---|---|---|
| `hosts`, `hosts_enabled` | | Hosts in the pool, and how many accept new VMs. |
| `host_info` | `host`, `version` | Value 1: the product version of each host. |
| `host_enabled` | `host` | 0 when the host is disabled (maintenance, or evacuated). |
| `host_live` | `host` | 1 when the host's heartbeat reaches the coordinator, 0 when it is offline. |
| `host_cpu_cores`, `host_memory_total_bytes`, `host_memory_used_bytes` | `host` | Capacity and memory in use. |
| `vms`, `vms_running` | | VMs, without templates, snapshots and control domains. |
| `vm_power_state_info` | `vm`, `state` | Value 1: `running`, `halted`, `paused` or `suspended`. |
| `vm_running` | `vm` | 1 when the VM runs. |
| `vm_vcpus`, `vm_memory_bytes` | `vm` | vCPUs at start; memory in use for a running VM, its configured maximum otherwise. |
| `vm_ha_protected_down` | `vm` | Only for VMs with a restart priority, in a pool with high availability on: 1 when the VM is halted. |
| `srs` | | Storage repositories. |
| `sr_size_bytes`, `sr_used_bytes`, `sr_used_percent` | `sr` | Fill level (ISO libraries are counted, not detailed). |
| `sr_shared` | `sr` | 1 for a shared repository (NFS, iSCSI…). |
| `pool_info` | `pool`, `master` | Value 1: the pool and its current coordinator. |
| `pool_ha_enabled` | `pool` | 1 when high availability is on. |
| `pool_ha_overcommitted` | `pool` | With HA on: 1 when the pool can no longer restart its protected VMs after the host failures it is set to tolerate. |
| `pool_ha_host_failures_to_tolerate` | `pool` | The number of host failures HA is set to survive. |

XenAPI has no "crashed" power state. A protected VM found halted is the
closest signal the API gives of a VM that stopped against your will; if you
shut down a protected VM on purpose, clear its restart priority first.

The [built-in rules](../alerting/rules.md#xcp-ng-xenserver) that apply:

- **XCP-ng host offline**: a host's heartbeat has not reached the coordinator
  for five minutes (Warning).
- **XCP-ng SR almost full**: a storage repository is 90% full for fifteen
  minutes (Advisory).
- **XCP-ng HA-protected VM down**: a VM protected by high availability is
  halted for two minutes (Warning).
- **XCP-ng pool HA overcommitted**: the pool is overcommitted for five minutes
  (Advisory).

## Give DumbMonit a read-only XAPI account

1. XAPI only accepts accounts other than its built-in superuser once the pool uses external authentication. If the pool is already joined to Active Directory, create a dumbmonit account there and go to step 3.

2. Otherwise, create a local account with a long random password on every host of the pool, then turn on PAM authentication once, from the pool coordinator. Until a role is given, the account can call nothing.

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. On the pool coordinator, register the account as a subject, then give that subject the built-in read-only role, with the UUID the first command prints.

    ```
    xe subject-add subject-name=dumbmonit
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

4. In DumbMonit, enter the address of the pool coordinator, for example "xcp01.lan", with that user name and password. Add the pool once: the coordinator reports every host, VM and storage repository. A pool member answers with the coordinator's address, which DumbMonit then follows.

5. DumbMonit only reads, through XAPI's JSON-RPC interface (/jsonrpc): it never starts, stops or migrates a VM and never changes a storage repository or the pool's high availability.

!!! warning
    Never give DumbMonit the superuser password of the hosts: with the read-only role, a leaked password cannot change anything. XCP-ng and XenServer ship a self-signed certificate: enable "Accept an unverifiable certificate" unless the DumbMonit host trusts the certificate the pool presents.

## Credentials

| Credential | Fields |
|---|---|
| User name and password | The `dumbmonit` account, registered as a subject with the `read-only` role. |

Address: the host name or IP of the pool coordinator, `host:port`, or a URL.
Options:

- **Port** (443).
- **Accept an unverifiable certificate**: the hosts' own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"XCP-ng refused the user name or password": wrong password, or the account is
not registered as a subject (`xe subject-list`).

"XCP-ng accepted the account but refused the call": the subject has no role,
or a role without read access; give it `read-only`.

"This host is a pool member, not the coordinator": DumbMonit could not reach
the coordinator the member named; enter the coordinator's address yourself.
