# Windows without an agent (WinRM)

A Windows server or PC monitored without installing anything, through WinRM
(WS-Management) and read-only WMI queries: CPU, memory, disks, services (the
chosen ones and every automatic service that is stopped), uptime and recent
event log errors.

DumbMonit only sends fixed `SELECT` queries (WQL `Enumerate`). It never runs a
command or a script on the server, and it can go through a
[relay agent](agent.md) to reach a network the server cannot.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 5986 | TCP | WinRM over HTTPS (default) |
| 5985 | TCP | Only with the Allow unencrypted traffic option |

## Prepare the Windows server

1. On the server, in an elevated PowerShell, enable WinRM.

    ```powershell
    Enable-PSRemoting -SkipNetworkProfileCheck -Force
    ```

2. Create the HTTPS listener with a certificate named after the server (use
   your own certificate instead of this self-signed one if you have one), then
   open its port.

    ```powershell
    $cert = New-SelfSignedCertificate -DnsName "srv01.corp.example.com" -CertStoreLocation Cert:\LocalMachine\My
    New-Item -Path WSMan:\LocalHost\Listener -Transport HTTPS -Address * -CertificateThumbPrint $cert.Thumbprint -Force
    New-NetFirewallRule -DisplayName "WinRM HTTPS" -Direction Inbound -Protocol TCP -LocalPort 5986 -Action Allow
    ```

3. Create the account with a long random password, then add it to the groups
   listed below.

    ```powershell
    net user dumbmonit * /add
    net localgroup "Remote Management Users" dumbmonit /add
    net localgroup "Performance Monitor Users" dumbmonit /add
    net localgroup "Event Log Readers" dumbmonit /add
    ```

4. Give the account access to the WMI namespace: wmimgmt.msc → right-click WMI
   Control → Properties → Security → Root → CIMV2 → Security, add it and tick
   Enable Account and Remote Enable.

5. In DumbMonit, enter the server name as its certificate names it
   (srv01.corp.example.com) and log in as dumbmonit, or DOMAIN\dumbmonit for a
   domain account.

### Permissions

| Right | Used for |
|---|---|
| Remote Management Users | Allows the WinRM login |
| Performance Monitor Users | Reads the CPU load per core |
| Event Log Readers | Only for the event log counts |
| WMI root/cimv2: Enable Account, Remote Enable | Reads the WMI classes |

The account is not an administrator and runs nothing on the server.

!!! tip "Group Policy"

    To roll this out to many servers, use *Computer Configuration → Policies →
    Administrative Templates → Windows Components → Windows Remote Management
    (WinRM) → WinRM Service* (Allow remote server management through WinRM,
    with the IP filters of your management network, and Allow Basic
    authentication left off), the *Windows Defender Firewall* inbound rule for
    TCP 5986, and the certificate through certificate auto-enrollment. Once the
    HTTPS listener works you can remove the HTTP one:
    `winrm delete winrm/config/Listener?Address=*+Transport=HTTP`.

## Authentication and encryption

- **NTLM** (default): local and domain accounts. The Negotiate header carries
  NTLMv2 tokens. Over HTTPS the channel is encrypted by TLS.
- **Basic**: only over HTTPS, and only if the server allows it
  (`winrm set winrm/config/service/auth @{Basic="true"}`).
- **Kerberos** is not supported: there is no pure-Rust, open-source client
  that fits the static binary. Use NTLM.
- **Unencrypted HTTP (5985)** is refused unless you tick *Allow unencrypted
  traffic*. The server must then allow it
  (`winrm set winrm/config/service @{AllowUnencrypted="true"}`), which exposes the
  data and the login to anyone on the network path. Prefer the HTTPS listener.
- A self-signed listener certificate needs *Accept an unverifiable
  certificate*; otherwise the certificate must be trusted by the DumbMonit
  host and name the server as you typed it.

An unreachable server (refused connection, timeout) is a *down* device; a 401 or
an *access denied* is a configuration error shown on the device, not an outage.

## What it watches

These metrics have the names and labels of the Windows agent, so dashboards and
rules work the same:

| Metric | From |
|---|---|
| `cpu_usage_percent`, `cpu_core_usage_percent`, `cpu_count` | `Win32_PerfFormattedData_PerfOS_Processor` |
| `memory_total_bytes`, `memory_used_bytes`, `memory_available_bytes`, `memory_used_percent` | `Win32_OperatingSystem` |
| `swap_total_bytes`, `swap_used_bytes`, `swap_used_percent` | `Win32_PageFileUsage` |
| `filesystem_*{mountpoint,device,fstype}` | `Win32_LogicalDisk` (fixed disks) |
| `uptime_seconds`, `boot_time_seconds`, `process_count`, `windows_version_info` | `Win32_OperatingSystem` |
| `service_up{service}` | `Win32_Service` |
| `windows_services_auto_stopped`, `windows_service_auto_stopped{service}` | automatic services that are stopped |
| `windows_eventlog_errors{log}`, `windows_eventlog_events{log,event_id,source}` | `Win32_NTLogEvent` |
| `winrm_section_readable{section}` | 0 when a read is refused |

- **Services**: list names in *Services to follow* to get an up/down series each
  (a name missing from the server counts as stopped). Every service set to start
  automatically that is stopped is reported too, except updaters and
  delayed-start services (`gupdate`, `edgeupdate`, `sppsvc`…) and the names in
  *Stopped services to ignore* (`*` at the start or end is a wildcard). The
  built-in rule *Windows automatic services stopped* alerts on it.
- **Event log**: the errors of the last N minutes (60 by default) of the logs
  you choose, as a count per log and the five most frequent Event IDs. The
  Security log counts audit failures. This reads the classic logs through
  `Win32_NTLogEvent`; for per-event rules (log, source, Event ID, text), use the
  Windows agent's [event log alerts](agent-windows-events.md).
- **Pending updates** cannot be read through WQL and are not collected.

## Options

| Option | Default | Role |
|---|---|---|
| Allow unencrypted traffic | off | HTTP on 5985 instead of HTTPS on 5986 |
| Port | 5986 | Used if the address does not give one |
| Authentication | `ntlm` | `ntlm` or `basic` (HTTPS only) |
| Accept an unverifiable certificate | off | For a self-signed listener |
| Timeout per request | 15 s | 1 to 120 |
| Services to follow | none | Comma-separated service names |
| Stopped services to ignore | none | Names, `*` wildcard allowed |
| Event logs to count | `System,Application` | `none` turns it off |
| Event window | 60 min | 1 to 1440 |
