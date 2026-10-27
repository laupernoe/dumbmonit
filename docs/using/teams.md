# Teams and folder permissions

A team gives a group of accounts a level of access on **folders** of monitors.
Use it when several groups share one DumbMonit — a NOC and a helpdesk, or one
team per site — and each should only see its own equipment.

Open **Settings → Team & access**, admins only.

## Permission levels

For each folder, a team gets one level. Each level includes the previous ones.

| Level | What members can do |
|---|---|
| **none** | Nothing: the folder is hidden. |
| **read** | See the monitors, their metrics and their alerts. |
| **operate** | Also acknowledge and dismiss alerts and set a maintenance on a monitor. |
| **manage** | Also edit, add and delete the monitors of the folder, and run a probe. |

Folder names:

- `*` means every folder.
- **Unfiled** means monitors that have no folder.
- A permission on `Paris` also covers `Paris/Rack A`. Sub-folders are written
  with `/` in the folder name.

## How rights combine

- An account in **several teams** gets the **strongest** level of the teams for
  each folder.
- The **global role caps the result**: a *viewer* gets *read* at most, an
  *operator* gets *manage* at most. An *admin* sees and does everything and is
  never restricted.
- An account in **no team** sees everything, as before. Choose **Accounts
  without a team see nothing** to reverse this.

## What a restricted account sees

An account that belongs to at least one team sees **only its folders**. The
server enforces this on every request, not the interface:

- the monitor list and detail pages, and their metrics (every metric query is
  filtered per monitor);
- alerts and their history, silences, and alert rules (without the monitors of
  other folders);
- the topology map, locations, and status pages whose components are all
  visible to the account;
- CVEs and Web Push notifications.

The following are **refused to restricted accounts**, and to the API tokens of
those accounts: logs, relays, backup, notification channels, scheduled reports,
incidents, the wall, self-monitoring, the Prometheus and federate API, and the
MCP endpoint. An API token inherits the teams of its owner.

A restricted operator can only set a maintenance on one specific monitor, not
on everything.

## Members

A team gets its members in three ways, which can be mixed:

1. **By hand** — add accounts to the team.
2. **By identity provider groups** — fill the **IdP groups** field, one group
   per line: a group name, an Entra object ID, or an LDAP DN or CN (`NOC`
   matches `cn=NOC,ou=Groups,…`). The membership is recomputed at **every
   OIDC, LDAP or SAML sign-in**. A member added by hand is never removed by
   this synchronisation.
3. **By [SCIM](scim.md)** — the identity provider pushes the membership.

Permissions on folders are always set by an admin in DumbMonit; the identity
provider only decides who is in the team.

## Backup

Teams, members and folder permissions are part of the
[backup](../install/backup.md). No secret is stored in clear text there.

See also [SAML](saml.md) and [SCIM](scim.md).
