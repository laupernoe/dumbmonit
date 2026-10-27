# Deploy the agent with Active Directory

Install the agent on every Windows server of a domain with one group policy
(GPO), instead of running the install command server by server.

When you add a monitor of type **Server with the agent** (or in the setup
assistant), **Deploy with Active Directory** creates a fleet
token limited to your domain (reusable, valid 30 days by default, with an
optional maximum number of servers) and downloads `Deploy-DumbMonit.ps1`. You
run that script once, as a Domain Admin, on a domain controller or on a machine
that has the GroupPolicy and ActiveDirectory PowerShell modules (RSAT). It
creates or updates the **DumbMonit Agent** GPO and links it to an OU. The GPO
runs an immediate scheduled task, as SYSTEM, at every policy refresh. Each
server installs the agent and enrols under its fully qualified name
(`srv01.corp.example.com`). It lands in the folder **Active Directory
&lt;domain&gt;**, with the tags `ad=<domain>` and `ad_ou=<OU>`.

## Steps

1. Add a monitor, pick **Server with the agent**, and click **Deploy with
   Active Directory**. Enter the domain, the name (FQDN) or address of the
   DumbMonit server the agents must contact (by default the address you are
   using) and, optionally, a DNS server that resolves that name — or, for a
   domain at a site behind a firewall, pick a relay under **Route**: the
   servers then download the agent from the relay's gateway and report through
   it, end-to-end encrypted to the server (see
   [Monitor a remote site](../install/remote-site.md#5-install-agents-behind-the-relay)) — adjust the options if needed, create the
   token and download `Deploy-DumbMonit.ps1`. The token is shown only now. The
   server name and the DNS server are checked for format, written into the
   script, and repeated in the summary of the next step.
2. Copy the script to a domain controller and open PowerShell as a domain
   administrator. Preview first, which changes nothing:

    ```powershell
    .\Deploy-DumbMonit.ps1 -WhatIf
    ```

3. Apply:

    ```powershell
    .\Deploy-DumbMonit.ps1 -TargetOU "OU=Servers,DC=corp,DC=example,DC=com"
    ```

4. Servers install the agent at their next policy refresh, within 90 minutes, or
   right away with `gpupdate /force` on a server. They appear in the wizard and
   in the folder **Active Directory &lt;domain&gt;** as they enrol.
5. When the fleet is deployed, delete the script and click **Stop new
   enrolments** (see [Security](#security)).

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

## Script parameters

The wizard options are baked in as defaults; any parameter can be overridden on
the command line.

| Parameter | Effect |
|---|---|
| `-TargetOU` | Distinguished name of the OU the GPO is linked to; a parent OU covers its children. Omitted: if exactly one OU (domain controllers excluded) holds Windows Server computers, it is used; if there are several, the script lists them and stops so that you pick one. |
| `-IncludeDomainControllers` | Also link the GPO to the Domain Controllers OU. |
| `-SecurityGroup` | Security filtering: only computers in this group apply the GPO. Also restricts who can read the SYSVOL kit. |
| `-UseSysvol` | Also copy the agent binary to SYSVOL, for servers that cannot download it from DumbMonit. They still need to reach the server to send their measurements. |
| `-GpoName` | Name of the GPO. Default `DumbMonit Agent`. |
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

## Removing the deployment

1. Run `.\Deploy-DumbMonit.ps1 -UninstallAgents`.
2. After the servers have refreshed their policy (Event ID 1004), run
   `.\Deploy-DumbMonit.ps1 -Remove`.

Without the first step, `-Remove` leaves the installed agents running. Also
stop the token's enrolments in DumbMonit.

## Troubleshooting

- **A module is missing.** On a server:
  `Install-WindowsFeature GPMC, RSAT-AD-PowerShell`. On Windows 10/11, add the
  RSAT capabilities `Rsat.GroupPolicy.Management.Tools` and
  `Rsat.ActiveDirectory.DS-LDS.Tools`.
- **"This enrollment token is limited to an Active Directory domain".** The server enrols under a name that does
  not end with the domain of the token: check that the machine is joined to the
  domain and that its primary DNS suffix matches. The kit is also tied to one
  domain: run the script in the domain it was generated for.
- **Several OUs hold servers.** Pass `-TargetOU` with a common parent OU, or run
  the script once per OU.
- **Nothing happens on a server.** Run `gpresult /r` to check that the GPO
  applies, then open Event Viewer → Windows Logs → Application and filter on
  the source `DumbMonit`.

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
