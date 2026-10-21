# MDaemon Email Server

MDaemon is watched with **the DumbMonit agent installed on the Windows server
that runs it**. Mail queues, sessions, Windows services and the disk space of
the mail store can only be read on the server itself: MDaemon publishes them
nowhere else, and no remote interface returns them. The agent reads them
locally and sends them to DumbMonit; the MDaemon monitor then ties the agent
to the mail ports DumbMonit checks from the outside (SMTP, IMAP, POP3,
webmail, Remote Administration).

!!! info "Agent required on the mail server"

    Without the [agent](agent.md), DumbMonit only sees the mail ports and, if
    you give an account, the version. The page of the monitor says so in a
    banner: *Agent not installed or not linked*, or *Agent unreachable*. That
    banner is a warning on the page, not an outage: no notification is sent
    for it.

| | With the agent on the server | Without it |
|---|---|---|
| Mail ports (SMTP, IMAP, POP3, webmail, Remote Administration) | Checked | Checked |
| Windows services of MDaemon (state, start mode) | Read | Not watched |
| Queues (inbound, local, remote, retry, bad, holding…) and their oldest message | Read | Not watched |
| Active sessions, message totals, spam and virus verdicts | Read | Not watched |
| Version | Read | Only with an XML API account, or from the SMTP greeting |
| Disk space of the mail store | Read | Not watched |

## Network ports to open

From the agent to the DumbMonit server:

| Port | Protocol | Purpose |
|---|---|---|
| 8080 | TCP | Agent readings (default port, 443 behind a TLS proxy) |

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 25 | TCP | SMTP check (default) |
| 143 | TCP | IMAP check (default) |
| 3000 | TCP | Webmail check (default) |
| 444 | TCP | Only with an XML API account (HTTPS, default) |
| 1000 | TCP | Only with the XML API over HTTP |
| 587 | TCP | Only with msa in the Services option |
| 465 | TCP | Only with smtps in the Services option |
| 110 | TCP | Only with pop3 in the Services option |
| 995 | TCP | Only with pop3s in the Services option |
| 993 | TCP | Only with imaps in the Services option |
| 5222 | TCP | Only with xmpp in the Services option |

The agent calls the server, never the other way round: no port needs to be
opened on the mail server for it.

!!! warning "Validated against the documentation only"

    This integration was built from MDaemon Technologies' published
    documentation, not against a real MDaemon server. Every call is a read,
    and the mail-port checks follow the protocol standards, but nobody has yet
    run it against a live MDaemon. This applies in particular to the names of
    MDaemon's Windows services, the folders of its queues (`Queues\Inbound`,
    `Retry`, `Bad`…) and the names of its performance counters, which come
    from published documentation and not from a running server. Tell us what
    breaks.

## Install

1. Install the [agent](agent.md#install) on the Windows server that runs
   MDaemon: in DumbMonit, add a **Server with agent** monitor and run its
   PowerShell command in an elevated PowerShell. From **Add a device** →
   **MDaemon Email Server**, the *Install the Windows agent* button opens that
   flow with the PowerShell command only, and the *Add the MDaemon monitor*
   button brings you back.
2. Add the MDaemon monitor and choose that agent in **Agent installed on this
   server**. When the address you type has the host name of an agent,
   DumbMonit pre-selects it. Without this choice, the page falls back on
   finding the agent by itself (the agent set as the monitor's parent, the
   relay agent that probes it, the agent whose host name matches the address),
   but only the explicit choice tells you, in a banner, when the agent is
   missing or silent.
3. Leave the credential on *Services only*. An XML API account is optional and
   only adds the exact version.

The agent detects MDaemon without any setting: it looks for a Windows service
whose name or display name contains `MDaemon`, takes the install folder from
the path of its executable (`C:\MDaemon\App\MDaemon.exe` gives `C:\MDaemon`),
and reads the product's performance counters. To force it on or off, set
`mdaemon: true` or `mdaemon: false` in `agent.yaml`; `false` also switches off
the Windows service and queue folder reading described below.

## What it watches

### Read by the agent {#mail-queues-through-the-agent}

The Windows services, the queue folders, the version and the disk space come
from the agent's own reading. They are prefixed `dumbmonit_agent_mail_`, belong
to the agent's `target`, and carry `product="mdaemon"`.

| Metric | Labels | Meaning |
|---|---|---|
| `info` (value 1) | `product`, `version` | MDaemon was found on the server, with the product version of its executable (absent when unreadable). |
| `service_up` | `product`, `service`, `display`, `start` | 1 if the Windows service is running. `service` is its name, `display` its display name, `start` how Windows starts it: `auto`, `manual`, `disabled` or `other`. Every service whose name or display name contains `MDaemon` is listed (the engine, Remote Administration, XMPP, anti-virus…, as installed). |
| `queue_files` | `product`, `queue`: `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | Messages (`.msg` files) in the queue folder under `Queues\` in the install folder. Only the folders that exist; at most 20,000 files are counted per folder. |
| `queue_oldest_seconds` | `product`, `queue` | Age of the oldest message of a non-empty queue folder. |
| `data_free_bytes`, `data_total_bytes` | `product` | Free and total space of the volume that holds the install folder. |

Sizes are counted from the folders so that they exist even where MDaemon's
counters are missing. When the counters below exist, the page shows their
values; the folder count fills in the other queues.

MDaemon also publishes its queue sizes (inbound, local, remote, retry, bad,
holding, quarantine), its session counts, its message statistics and its
uptime as Windows performance counters, in the object named MDaemon. The
agent reads them; none of MDaemon's published documentation describes an XML
API call that returns them. The series are prefixed `dumbmonit_mdaemon_`:

| Metric | Labels | Performance counters read |
|---|---|---|
| `queue_messages` | `queue`: `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | `Inbound queue messages`, `Local queue messages`, `Remote queue messages`, `Retry queue messages`, `Bad queue messages`, `Holding queue messages`, `LAN queue messages`, `Quarantine queue messages`, `RAW queue messages` |
| `queue_frozen` (1 frozen) | `queue`: `inbound`, `local`, `remote` | `Inbound queue frozen`, `Local queue frozen`, `Remote queue frozen` |
| `sessions_active` | `protocol`: `smtp_in`, `smtp_out`, `pop3_in`, `pop3_out`, `imap`, `webmail` | `Active SMTP (in) sessions`, `Active SMTP (out) sessions`, `Active POP3 (in) sessions`, `Active POP3 (out) sessions`, `Active IMAP sessions`, `Active Webmail sessions` |
| `sessions_total` (counter) | `protocol`: `smtp_in`, `smtp_out`, `pop3`, `imap` | `SMTP sessions (in) total`, `SMTP sessions (out) total`, `POP3 sessions total`, `IMAP sessions total` |
| `messages_total` (counter) | `protocol`: `smtp_in`, `smtp_out`, `domainpop_in` | `SMTP messages (in) total`, `SMTP messages (out) total`, `DomainPOP messages (in) total` |
| `filtered_messages_total` (counter) | `filter`: `spam`, `virus`, `dnsbl`; `verdict`: `accepted`, `refused` | `spam accepted total`, `spam refused total`, `Viruses accepted total`, `Viruses refused total`, `DNSBL accepted total`, `DNSBL refused total` |
| `server_active` (1 active) | `server`: `smtp`, `pop3`, `imap`, `webmail`, `webadmin`, `activesync`, `antispam`, `antivirus`, `minger`, `multipop`, `domainpop` | `SMTP server state`, `POP3 server state`, `IMAP server state`, `Web Mail server state`, `Web Admin server state`, `ActiveSync server state`, `AntiSpam server state`, `AntiVirus server state`, `Minger server state`, `MultiPOP server state`, `DomainPOP server state` |
| `running` (1 running) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

Totals count since MDaemon last started: apply `increase()` or `rate()` in
queries, as the page does for its 24-hour figures. The per-second counters
(`SMTP messages (in)/sec`…) are not read, since the totals give the same
rates. A counter that your MDaemon version does not have, or names
differently, is reported once in the agent's log (`performance counter not
available`) and the others are still read; to read one the preset lacks,
list it under `perf_counters` in `agent.yaml` (see
[Windows performance counters](agent.md#windows-performance-counters)).

### Network services

Checked by DumbMonit from the outside, with no account. All metrics are
prefixed `dumbmonit_mdaemon_`.

| Metric | Labels | Meaning |
|---|---|---|
| `service_up` | `service`, `port` | 1 if the port accepted the connection and, for SMTP, POP3 and IMAP, greeted with `220`, `+OK` or `* OK`. 0 if it did not answer, or answered to refuse service (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Time to the greeting, or to the connection for a port without one. |
| `scrape_duration_seconds` | | Time the whole check took. |

### API (optional)

With an XML API account only.

| Metric | Labels | Meaning |
|---|---|---|
| `api_up` | | 1 if the XML API answered, 0 if it did not. |
| `api_response_seconds` | | Time the XML API took to answer. |
| `api_operation_ok` | | 1 if the account was allowed to run the call, 0 if MDaemon answered with an error status. |
| `info` (value 1) | `version`, `build`, `source` | The version: from the XML API (`source="xml_api"`, `build` is the service build), or else from the SMTP greeting when the server announces it there (`source="smtp_banner"`). |

**A partial outage is still a successful check.** One stopped service gives
`service_up 0` for that service and the others are still measured. Only when
no service and no API answers at all is the monitor *unreachable*.

**A refused account is a configuration error, not an outage.** A wrong
password, an address the XML API does not allow, or a wrong port is shown on
the monitor page with the reason and is not notified.

The license is not exposed through a documented interface or a performance
counter, and is not read.

The [built-in rules](../alerting/rules.md#mdaemon-and-securitygateway) that
apply: MDaemon mail service down and MDaemon XML API not answering (from the
monitor), plus Monitor unreachable; and, on the agent's monitor, Mail server
Windows service stopped, MDaemon mail queue growing, Mail queue stuck,
MDaemon queue frozen, MDaemon Bad queue not empty and MDaemon Retry queue
high.

## The monitor page

The page of an MDaemon monitor shows, from top to bottom:

1. A **banner** when the agent is missing or silent: *Agent not installed or
   not linked* (with a button to install it and one to choose it), *Agent
   unreachable* (with the time of its last report), or *The agent does not see
   MDaemon on its machine*. It warns that the queues and Windows services are
   not watched; it is not an alert.
2. **Read by the agent**: the *Windows services* (running or stopped, and how
   they start), the *Queues* (messages per queue, the age of the oldest one,
   and a word when a queue is frozen, stuck, or when Bad is not empty or Retry
   is over fifty), the *Sessions*, the messages and spam, virus and DNSBL
   verdicts of the last 24 hours, MDaemon's uptime and version, the disk space
   of the mail store, and the internal servers MDaemon reports as inactive.
   The same panel appears on the agent's own page.
3. **Network services**: every watched port, a word and a colour (Answering,
   Down), the response time; then, under **API**, the XML API state and the
   version it reads.

Everything is read from the last stored measurement: opening the page never
connects to the server.

## Install the agent, then add the monitor

1. On the Windows server that runs MDaemon, add a Server with agent monitor in DumbMonit and run its PowerShell command in an elevated PowerShell. The agent finds MDaemon by itself.

2. Back here, enter the server address and choose that agent in Agent installed on this server. DumbMonit pre-selects the one with the same host name.

3. Leave the credential on Services only: the agent already brings the queues, sessions, Windows services and disk space, and the mail ports are checked from DumbMonit.

4. Optional, only to read the exact version: open Accounts → Account Manager → New account and create the account below, with a long unique password.

    ```
    dumbmonit@example.com
    ```

5. Allow the address DumbMonit connects from in Setup → XML API Service → Address Restrictions (Setup → XML API Management on MDaemon 26), then enter the account's full email address and its password.

6. To check more ports than SMTP, IMAP and webmail, set the Services option to a list like this one.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

### Required permissions

- **Local administrator on the server**: Only to install the agent, which then runs as a Windows service
- **XML API access for the account**: Optional: only to read the version
- **XML API address restriction**: Optional: allow the address DumbMonit connects from

Vendor documentation: <https://help.mdaemon.com/mdaemon/en/xml-api-service.html>

### Good to know

The queues, sessions and Windows services are read by the agent, with the rights of the Windows service that runs it (Local System by default), which can read the install folder and the performance counters. Nothing is written on the server.

The XML API account only serves to read the version: DumbMonit only calls `GetVersionInfo`, which reads no mailbox and changes nothing. Since MDaemon 24, the XML API only answers the addresses it allows. It is reached on the Remote Administration HTTPS port, 444 by default. To check from the DumbMonit host that the account reaches the API, run the command below: it asks for the password and prints an answer that starts with `<MDaemon><API productversion=…>`.

```
curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
```

Dynamic Screening can block an address after repeated failed logins, so a wrong password here can get the DumbMonit host blocked: test with the command above first. Remote Administration often uses a self-signed certificate: if the connection is refused for that reason, tick "Accept an unverifiable certificate" in the options.

!!! warning
    The queues, sessions and Windows services of MDaemon can only be read on the server itself: without the agent, DumbMonit sees the mail ports and nothing else. The XML API account is optional and only brings the version; MDaemon has no read-only role for it, so never reuse your own account, and use a password found nowhere else, as repeated failures can get DumbMonit blocked.

The XML API is served by the Remote Administration web server: port 1000 over
HTTP and 444 over HTTPS by default, at `/MdMgmtWS/`, or by IIS when Remote
Administration runs there. Its full reference ships with the server: open
`https://mail.example.com:444/MdMgmtWS` in a browser, or look in
`\MDaemon\Docs\API\XML API\`.

## Credentials

| Credential | Fields |
|---|---|
| Services only | Nothing: the mail ports are checked without logging in. This is the usual choice: the agent brings the rest. |
| XML API account (optional) | The full email address of the account, and its password, sent as HTTP basic authentication. It only adds the exact version. |

Address: the server's host name or IP, for example `mail.example.com`. A port
written in the address (`mail.example.com:8444`) is taken as the XML API port.

## Options

| Option | Default | Effect |
|---|---|---|
| `services` | `smtp,imap,webmail` | What to check, comma-separated: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `name:port` changes a port; any other name with its port adds a service of your own, checked by connection only. `none` checks no port. |
| `agent_id` | none | The monitor of the agent installed on the MDaemon server (chosen in the form; the id of the agent's monitor). |
| `request_timeout_seconds` | `10` | Time allowed for each connection, greeting and API call, from 1 to 60. |
| `api_port` | `444` | The Remote Administration port that serves `/MdMgmtWS/`. Used only with an account. |
| `api_tls` | `true` | HTTPS to the XML API. Untick only for Remote Administration over plain HTTP (port 1000 by default): the password then crosses the network in clear. |
| `insecure_tls` | `false` | Accept a self-signed certificate on the XML API. |

The TLS ports (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) are checked by
connection only. To watch their certificates' expiry, add a
[TLS certificate](services.md#tls) monitor for each.
