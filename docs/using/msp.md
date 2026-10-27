# Multi-client mode (MSP)

Multi-client mode lets one DumbMonit serve several customers while keeping
their data apart. Each **client** has its own monitors, status pages, agent
tokens and reports, and its members never see another client's data.

Clients sit above [teams and folders](teams.md): the client decides what an
account can see at all, then its teams narrow that down folder by folder.

Open **Settings → Team & access → Clients**, admins only.

## Turning it on

Until you switch **Multi-client mode** on, nothing changes: a single-customer
instance behaves exactly as before.

When you upgrade, everything you already have goes to a client named
**Default**: monitors, status pages and agent tokens. Accounts that are not
admins become members of Default, so turning the mode on later surprises no
one. Notification channels, alert rules, maintenance windows and report
schedules stay **shared** by the whole instance.

With the mode on:

- **Admins are MSP administrators.** They see and manage every client, and pick
  one in the header to see only that client's pages. Picking a client only
  filters what is shown; it takes no right away.
- **Every other account only sees the clients it belongs to**, within the limits
  of its role and teams. An account that belongs to no client sees nothing. The
  Clients settings list such accounts so you can place them.
- A new account created while the mode is on belongs to no client until you add
  it to one, or until an identity-provider group adds it.

## What belongs to a client

| Object | Belongs to |
|---|---|
| Monitors (agents included) | Always a client |
| Status pages | A client, or the instance |
| Agent enrolment tokens | Always a client |
| Notification channels | A client, or the instance |
| Alert rules | A client, or the instance |
| Maintenance windows | A client, or the instance (a window on one monitor follows that monitor) |
| Report schedules | A client, or the instance |
| Shared dashboards | A client, or the instance (a personal dashboard stays with its owner) |
| On-call schedules and escalation policies | A client, or the instance |
| Runbooks | A client, or the instance |
| Remediation actions | A client, or the instance (an action bound to a monitor follows that monitor) |

What the instance shares serves every client: an instance channel hears about
every client, an instance rule watches every monitor. What a client owns only
concerns that client:

- a client's **channel** only receives alerts of that client's monitors — even
  when a rule names it — and never alerts that belong to no monitor;
- a client's **rule** only evaluates that client's monitors;
- a client's **maintenance window** only silences that client's monitors;
- a client's **status page** can only list that client's monitors;
- a client's **report schedule** only reports on that client's monitors;
- a client's **runbooks and actions** only show under that client's alerts and
  monitors, and a client's action never runs on another client's monitor, by
  hand or through an automatic policy;
- a client's **shared dashboard** is only shown to that client's members.
  Instance dashboards are not shown to members at all: their widgets could name
  other clients.

Objects you create while a client is picked in the header belong to that
client. To move an existing object, use the client menu next to it (status
pages, channels, report schedules, shared dashboards, on-call schedules and
policies, runbooks, actions) or the **Client** field of a monitor.
Alert rules and maintenance windows are moved through the API
(`PUT /api/clients/assign`).

### Moving a monitor

Moving a monitor to another client takes what depends on it along: its
children, the monitors an agent relays, and the monitors created from an
agent's Docker labels. Any link left between two clients is cut — a parent,
a relay, a probe location of a multi-site monitor, a line of another client's
status page. A relay never probes, and so never receives the credentials of,
another client's monitor.

## Agents

Each client gets its own enrolment tokens: create the token while the client is
picked in the header. A machine enrolled with a client's token enters that
client, and only that client:

- a new machine becomes a monitor of the token's client;
- a token never adopts a monitor declared in another client, and never
  re-enrols (re-binds) a machine of another client;
- a machine already bound keeps reporting into its own monitor, recognised by
  its binding secret.

## Quotas

A client can have a **device quota**: the most monitors it may hold, agents and
monitors created from Docker labels included. Past it, new monitors and agent
enrolments for that client are refused. Leave the field empty for no limit.

Bulk imports (CSV, Uptime Kuma) put the monitors in the Default client; move
them afterwards.

## Members and single sign-on

Add members by hand, or list identity-provider groups (OIDC, LDAP, SAML): an
account in one of those groups joins the client when it signs in, and leaves it
when the group goes. Members added by hand are never removed by a sign-in.
With SCIM provisioning, a SCIM group whose name is listed on a client brings
its members into that client, and takes them out when they leave the group.

## Reports and branding

Give a client a **logo** (PNG, JPEG or WebP, 256 KiB at most). It appears:

- on the client's status pages that have no logo of their own, with the
  client's name;
- in the header of the client's availability report (PNG and JPEG; for WebP the
  name alone).

Turn on **Email it to the contacts** to send the availability (SLA) report of
the past month to the client's contacts: on the 1st at 8:00 in the client's
timezone, a PDF and two CSV files, with the client's name and logo. It goes out
through the client's own email channel if it has one, otherwise through the
instance's. A month is sent once; a missed month is not caught up.

Members of a client download their report from the **Clients** page. The MSP
administrator finds every client there, with what is alerting and how full its
quota is.

## What members can and cannot reach

A member of a client is a restricted account, like a member of a team: the
server only opens to it a closed list of pages — monitors, metrics, alerts,
maintenance on its own monitors, the map, status pages, vulnerabilities, its
clients' logos and reports — and refuses everything else (settings, logs,
backups, tokens, the assistant endpoint, raw metrics, log search, runbooks and
remediation, on-call, configuration files, status page subscribers). Their own
shared dashboards and the metrics explorer stay open, filtered to their
monitors. Every list is filtered
on the server; a monitor of another client answers *not found*, as if it did not
exist. API tokens of a member inherit the same limits.

When the MSP administrator picks a client in the header, lists follow that
client: dashboards, on-call schedules and policies, runbooks, actions and their
run log, and the log search, which then only shows lines of that client's
monitors (DumbMonit's own logs belong to no client). Configuration files
(GitOps export, plan and apply) always describe the whole instance and stay
with the MSP administrator.

!!! note "Known limit"
    Two monitors of the same kind and address cannot coexist, even in two
    clients. Adding a monitor whose address another client already watches is
    refused, which tells that the address is watched somewhere.

## Backups

Backups carry the clients, their members, settings, logos, and which client
every monitor, status page, agent token, channel, rule, maintenance window and
report schedule belongs to.
