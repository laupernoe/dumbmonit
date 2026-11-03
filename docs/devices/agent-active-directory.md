# Deploy the agent with Active Directory

Install the agent on every Windows server of a domain with one group policy
(GPO), instead of running the install command server by server.

## 1. What it does

**Deploy with Active Directory** (when you add a monitor of type **Server with
the agent**) creates a fleet token limited to your domain and gives you one
file, `Deploy-DumbMonit.ps1`. You run it once, as a Domain Admin. It:

- creates (or updates) a group policy called **DumbMonit Agent**;
- puts in it one immediate **scheduled task**, run as SYSTEM at every policy
  refresh, that installs the agent when it is missing or out of date and does
  nothing otherwise;
- links the policy where you chose (the whole domain, one OU, or a group).

Each server then installs the agent and **enrols under its fully qualified name**
(`srv01.corp.example.com`). It lands in the folder **Active Directory
&lt;domain&gt;**, with the tags `ad=<domain>` and `ad_ou=<OU>`.

## 2. Prerequisites

- You are a member of **Domain Admins** (or Enterprise Admins).
- The machine that runs the script is a domain controller, or has the
  GroupPolicy and ActiveDirectory PowerShell modules (RSAT).
- The servers can reach the DumbMonit server over **HTTPS**, on the web port
  (to download the agent) and on the **agent port, TCP 8443** (to report). Open
  nothing inbound on the servers. See [Ports](../install/ports.md).
- The DumbMonit address you give is resolvable from the servers (otherwise
  enter a DNS server in the wizard).

## 3. Steps with the wizard

1. Add a monitor, pick **Server with the agent**, click **Deploy with Active
   Directory**. Enter the domain (`corp.example.com`) and, if needed, the name
   (FQDN) of the DumbMonit server the agents contact and a DNS server that
   resolves it. For a domain at a site behind a firewall, pick a relay under
   **Route** (see
   [Monitor a remote site](../install/remote-site.md#5-install-agents-behind-the-relay)).
2. Choose **which servers** get the agent (see [Choosing the scope](#4-choosing-the-scope))
   and the **token validity** (see [The token](#6-the-token)).
3. The wizard shows the exact **command to run**, which changes as you change
   the options. Create the token and download `Deploy-DumbMonit.ps1`. The token
   is shown only now.
4. Copy the script to a domain controller, open PowerShell **as a domain
   administrator** in its folder, and paste the single line the wizard shows:

    ```powershell
    Unblock-File .\Deploy-DumbMonit.ps1; powershell -NoProfile -ExecutionPolicy Bypass -File .\Deploy-DumbMonit.ps1
    ```

    `Unblock-File` removes the "security warning" Windows puts on a downloaded
    file, and `-ExecutionPolicy Bypass` ignores the execution policy for this run
    only. To see what would be done without changing anything, add `-WhatIf`.
5. Without any targeting parameter, the script asks what to cover (all servers,
   one OU, or one group), prints the plan, and asks one confirmation:
   `Continue? (Y/n)` and `Refresh all servers now? (Y/n)` (Enter means yes).
   `-Yes` skips the questions.
6. Wait for the servers (next section), then delete the script.

The script is idempotent: running it again updates the GPO in place.

!!! info "How the agents talk to the server"
    Once enrolled, each agent talks to DumbMonit over the encrypted channel,
    with its own agent certificate (mutual TLS, TLS 1.3). The servers only need
    to reach the DumbMonit server on **TCP 8443** (outbound from the servers
    to DumbMonit, nothing inbound), also from another site. The name you entered
    must be the one in the server's certificate and resolvable by the servers;
    if their resolver cannot find it, enter a DNS server and the kit resolves
    the name through it and records it in the server's `hosts` file (lines
    marked `# DumbMonit`). See [Agent channel security](../install/agent-security.md).

## 4. Choosing the scope

| Scope | Parameter | Example | Use it when |
|---|---|---|---|
| All Windows servers of the domain, including future ones (recommended) | `-AllServers` | `.\Deploy-DumbMonit.ps1 -AllServers` | You want every server, now and later, with nothing to maintain. |
| Servers of one OU | `-TargetOU` | `-TargetOU "OU=Servers,DC=corp,DC=example,DC=com"` | Your servers live in one OU (a parent OU covers its children). |
| Members of a security group | `-SecurityGroup` | `-SecurityGroup "DumbMonit Servers"` | You want to pick the servers by hand, by adding them to a group. |

`-AllServers` links the policy to the domain root and filters it with a WMI
filter on Windows Server (domain controllers included); workstations are left
alone. It cannot be combined with `-SecurityGroup`. `-IncludeDomainControllers`
links the policy to the Domain Controllers OU too (not needed with
`-AllServers`). `-UseSysvol` also copies the agent binary to SYSVOL for servers
without internet access.

Pitfalls:

- **`CN=Computers` is not an OU.** Servers left in the default *Computers*
  container are not covered by an OU link. Only `-AllServers` (link at the
  domain root) covers them, or move them to an OU.
- **A security group needs a restart.** A computer's group membership is read
  when it signs in to the domain. After adding a server to the group, restart
  it, or run `klist -li 0x3e7 purge` on it and then `gpupdate /force`.
- **Domain controllers are not "Domain Computers".** They are in their own group,
  and live in the Domain Controllers OU. Use `-AllServers` or
  `-IncludeDomainControllers` to include them.

## 5. Install quickly, and check

By default the script does not make you wait for the 90-minute policy refresh.
Once the GPO is deployed it refreshes every targeted server right away (all
Windows Servers of the domain, those of the OU and its children, or the members
of the security group; disabled computers are skipped):

1. on each server, `klist -li 0x3e7 purge` then `gpupdate /target:computer /force`,
   through WinRM (16 servers at a time, short timeouts); where WinRM fails, it
   falls back to `Invoke-GPUpdate -Force` (remote scheduled task);
2. a table lists each machine, the method (`WinRM`, `GPUpdate` or `unreachable`)
   and the result. An unreachable server picks the policy up within 90 minutes,
   or run `gpupdate /force` on it;
3. the script then waits up to 3 minutes (polling every 15 seconds, Ctrl+C to
   stop) and shows live how many reachable servers already run the agent
   (`X/N`). Servers that are not there yet never make the script fail.

In the interactive menu the recap asks `Refresh all servers now? (Y/n)` (Enter
means yes); `-Yes` answers yes. Use `-NoRefresh` to skip all of this (automation:
servers then install within 90 minutes) and `-NoVerify` to refresh without
waiting for the agents. Nothing is refreshed with `-WhatIf` or `-Remove`.

To check:

- the servers appear in the wizard and in the folder **Active Directory
  &lt;domain&gt;** as they enrol;
- on a server, open Event Viewer, Windows Logs, Application, and filter on the
  source `DumbMonit` (see the event table below); `gpresult /r` shows whether
  the GPO applies.

## Script parameters

The wizard options are baked in as defaults; any parameter can be overridden on
the command line.

| Parameter | Effect |
|---|---|
| `-AllServers` | Link the GPO to the domain root, restricted to Windows Server by a WMI filter. |
| `-TargetOU` | Distinguished name of the OU the GPO is linked to; a parent OU covers its children. Omitted and non-interactive: if exactly one OU (domain controllers excluded) holds Windows Server computers, it is used; if there are several, the script lists them and stops. |
| `-IncludeDomainControllers` | Also link the GPO to the Domain Controllers OU. |
| `-SecurityGroup` | Security filtering: only computers in this group apply the GPO. Also restricts who can read the SYSVOL kit. |
| `-UseSysvol` | Also copy the agent binary to SYSVOL, for servers that cannot download it from DumbMonit. They still need to reach the server to send their measurements. |
| `-GpoName` | Name of the GPO. Default `DumbMonit Agent`. |
| `-Yes` | Do not ask anything (servers are refreshed now). |
| `-NoRefresh` | Do not refresh the servers after deploying (they install within 90 minutes); also skips the verification. For automation. |
| `-NoVerify` | Refresh the servers but do not wait up to 3 minutes to see the agents appear. |
| `-UninstallAgents` | Switch the GPO to uninstalling the agent from the servers it applies to. |
| `-Remove` | Delete the GPO, its links and the SYSVOL kit. Installed agents keep running. |
| `-WhatIf` | Show what would be done, change nothing. |

## What happens on each server

The task runs `Install-DumbMonitAgent.ps1` from the kit in SYSVOL
(`\\<domain>\SYSVOL\<domain>\DumbMonit`).

- **Nothing to do when up to date.** If the agent is installed and the SHA-256
  of its binary matches the expected one, the script exits. A stopped service
  is started again.
- **The binary is verified.** Downloaded from the DumbMonit server (checked by
  the installer) or copied from SYSVOL (copied locally, then checked against
  the expected SHA-256 before it is installed).
- **An existing installation keeps its name**, so a renamed server does not
  appear twice.
- **Results go to the Application event log**, source `DumbMonit`:

| Event ID | Level | Meaning |
|---|---|---|
| 1000 | Information | Agent already up to date; its service was stopped and has been restarted. |
| 1001 | Information | Agent installed or updated. |
| 1002 | Error | Failure (installer error, or SYSVOL binary with an unexpected SHA-256). |
| 1003 | Warning | The computer is not joined to a domain; nothing installed. |
| 1004 | Information | Agent uninstalled. |

The enrolment token never appears in these logs.

## Security

- The token only enrols machines **of the domain**: a host name outside it
  (`srv01`, `srv01.other.example.com`) is refused. Each server then receives
  its own binding secret, so no server can report as another.
- The token is stored hashed and never shown again. It lives in the script and
  in the SYSVOL kit: **treat the script like a password and delete it after
  use.**
- The SYSVOL folder `\\<domain>\SYSVOL\<domain>\DumbMonit` is readable only by
  Domain Computers (or the `-SecurityGroup`), Domain Controllers and
  administrators.
- **Stop new enrolments** (button in the wizard and on the token) closes the
  token: servers already enrolled keep reporting. **Revoke** (Settings →
  Agents) cuts off every agent that uses the token. Stop enrolments once the
  fleet is deployed; revoke only to cut agents off.
- Use HTTPS for the DumbMonit URL, since agents send their measurements over
  it.

### Signed scripts

If your execution policy requires `AllSigned` (set by GPO, it overrides the
`-ExecutionPolicy Bypass` the task passes), sign `Deploy-DumbMonit.ps1` before
running it. Then sign the kit scripts in SYSVOL (`install.ps1` and
`Install-DumbMonitAgent.ps1`) after each run, since the script rewrites them.

```powershell
$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Select-Object -First 1
Set-AuthenticodeSignature .\Deploy-DumbMonit.ps1 $cert
Get-ChildItem "\\corp.example.com\SYSVOL\corp.example.com\DumbMonit\*.ps1" |
    ForEach-Object { Set-AuthenticodeSignature $_.FullName $cert }
```

The machines must trust the certificate's publisher.

## Upgrading DumbMonit

With [automatic updates](agent.md#automatic-updates), agents update themselves
after you upgrade DumbMonit: there is no need to run the kit
(`Deploy-DumbMonit.ps1`) again. The GPO script no longer reinstalls an agent
whose binary differs from the kit when the installed agent is the same version
or newer (it runs `dumbmonit-agent.exe --version`), so it never downgrades an
agent that updated itself.

Run the kit again only to change the token or URL, or for machines whose agent
predates automatic updates. Agents older than the kit are still upgraded by the
GPO; download a new kit first, as it embeds the SHA-256 of the agent binary
shipped by your server (otherwise servers refuse the binary, Event ID 1002).

## 6. The token

The wizard offers a validity of **30 days**, **1 year** or **Never expires**.
With *all servers* the default is *Never expires*, because servers added to the
domain later must still be able to enrol; otherwise the default is 30 days.
Machines already enrolled keep reporting after the token expires.

Two more safeguards apply to a fleet token (see
[Protecting enrolment](../install/agent-security.md#protecting-enrolment)):

- **Approval.** By default a new machine enrolling with a fleet token waits for
  an administrator in **Settings → Agents**. For a domain rollout, choose
  *Accept automatically if the host belongs to the AD domain of the token,
  otherwise ask*: your servers enrol at once, anything else waits.
- **Sleep.** A fleet token that enrolled no machine for 30 days goes to sleep and
  refuses new enrolments. Press **Reactivate** in **Settings → Agents** when you
  add servers later. Machines already enrolled are not affected.

A token that never expires keeps its safeguards: it only enrols machines **of
your domain**, it is reusable up to the optional maximum number of servers, and
you can revoke it at any time in **Settings, Agents**. Keep the script like a
password, and revoke the token there (or use **Stop new enrolments**) once all
your servers are in.

## 7. Uninstalling

1. Run `.\Deploy-DumbMonit.ps1 -UninstallAgents`.
2. After the servers have refreshed their policy (Event ID 1004), run
   `.\Deploy-DumbMonit.ps1 -Remove`.

Without the first step, `-Remove` leaves the installed agents running. Also
stop the token's enrolments in DumbMonit.

## 8. Troubleshooting

| Message or symptom | Cause and fix |
|---|---|
| "Servers are spread over several organisational units" | The script was run without a target and several OUs hold servers. Run it in an interactive console to pick, or pass `-AllServers`, or `-TargetOU` with a common parent OU. |
| `sddlForm` error when setting the SYSVOL permissions | Fixed in current kits (it came from a one-item list reduced to a string). Download a new kit from DumbMonit. |
| "Security warning: Do you want to run Deploy-DumbMonit.ps1?" | The file is marked as downloaded. Run `Unblock-File .\Deploy-DumbMonit.ps1` first, or use the single line from the wizard. |
| "running scripts is disabled on this system" | Use `powershell -NoProfile -ExecutionPolicy Bypass -File .\Deploy-DumbMonit.ps1`. |
| The module GroupPolicy or ActiveDirectory is missing | On a server: `Install-WindowsFeature GPMC, RSAT-AD-PowerShell`. On Windows 10/11, add the RSAT capabilities `Rsat.GroupPolicy.Management.Tools` and `Rsat.ActiveDirectory.DS-LDS.Tools`. |
| Servers do not appear, event 1002, or the download fails | The server cannot reach DumbMonit. From the server: `Test-NetConnection <dumbmonit-host> -Port 8443` (and the web port). Check the name resolves, the firewall, and the certificate. |
| "This enrollment token is limited to an Active Directory domain" | The server enrols under a name that does not end with the domain of the token: check that it is joined to the domain and that its primary DNS suffix matches. The kit is tied to one domain. |
| A server in a security group does nothing | Restart it, or `klist -li 0x3e7 purge` then `gpupdate /force`. |
| A server sits in `CN=Computers` and is not covered | Use `-AllServers`, or move it to an OU. |
| Nothing happens on a server | `gpresult /r` to check that the GPO applies, then Event Viewer, Application, source `DumbMonit`. |

## No MSI

DumbMonit does not ship an MSI: the Windows agent is cross-compiled on Linux.
The scheduled-task GPO takes the place of MSI software deployment, with the
added benefit of updating the agent and checking its SHA-256.

## Linux: Ansible or SSH

There is no GPO equivalent on Linux, but a **reusable** token (tick *Several
machines with this command* when adding a server with the agent) makes the install command shown by the
server work as is on any number of machines.

Store the full command in a vault variable `dumbmonit_install_command`
(`curl -sSL https://dumbmonit.example.com/install.sh | sh -s -- --token=dmon_xxx --url=https://dumbmonit.example.com`):

```yaml
- name: Install the DumbMonit agent
  ansible.builtin.shell: "{{ dumbmonit_install_command }}"
  args:
    creates: /etc/dumbmonit/agent.yaml
  become: true
  no_log: true
```

`creates` skips machines that already have an agent configuration, and `no_log`
keeps the token out of Ansible's output. The same idea over plain SSH:

```sh
for h in srv01 srv02 srv03; do
  ssh "$h" "sudo sh -c 'test -f /etc/dumbmonit/agent.yaml || { $INSTALL_COMMAND; }'"
done
```

Stop the token's enrolments once the machines are in, as for Active Directory.
