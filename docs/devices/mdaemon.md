# MDaemon Email Server

The mail services of an MDaemon server, checked from the outside: SMTP, IMAP,
POP3, webmail and Remote Administration accept connections and greet the way
they should. With an account, DumbMonit also reads the exact version through
the XML API. With the [agent](agent.md) installed on the MDaemon server, the
same page also shows the mail queues, sessions and message totals, which
MDaemon publishes only as Windows performance counters.

!!! warning "Validated against the documentation only"

    This integration was built from MDaemon Technologies' published
    documentation and the XML API response format shown there, not against a
    real MDaemon server. Every call is a read, and the mail-port checks follow
    the protocol standards, but nobody has yet run it against a live MDaemon.
    The same goes for the queue counters read by the agent: their names come
    from a published list of MDaemon's performance counters, not from a
    running server. Tell us what breaks.

## What it watches

All metrics are prefixed `dumbmonit_mdaemon_`.

| Metric | Labels | Meaning |
|---|---|---|
| `service_up` | `service`, `port` | 1 if the port accepted the connection and, for SMTP, POP3 and IMAP, greeted with `220`, `+OK` or `* OK`. 0 if it did not answer, or answered to refuse service (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Time to the greeting, or to the connection for a port without one. |
| `api_up` | | With an account only: 1 if the XML API answered, 0 if it did not. |
| `api_response_seconds` | | Time the XML API took to answer. |
| `api_operation_ok` | | 1 if the account was allowed to run the call, 0 if MDaemon answered with an error status. |
| `info` (value 1) | `version`, `build`, `source` | The version: from the XML API (`source="xml_api"`, `build` is the service build), or else from the SMTP greeting when the server announces it there (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Time the whole check took. |

**A partial outage is still a successful check.** One stopped service gives
`service_up 0` for that service and the others are still measured. Only when
no service and no API answers at all is the device *unreachable*.

**A refused account is a configuration error, not an outage.** A wrong
password, an address the XML API does not allow, or a wrong port is shown on
the device page with the reason and is not notified.

**Queues come from the agent, not from this device.** MDaemon publishes its
queue sizes (inbound, local, remote, retry, bad, holding, quarantine), its
session counts, its message statistics and its uptime as Windows performance
counters, in the object named MDaemon, and in its own Queue and Statistics
Manager. None of MDaemon's published documentation describes an XML API call
that returns them, so this device does not guess one: the
[agent](agent.md), installed on the MDaemon server, reads the performance
counters instead. See [Mail queues through the agent](#mail-queues-through-the-agent).
The license and the mail store disk space are not exposed through a
documented interface or a performance counter, and are not read. The agent
also watches the disk that holds the mail store; list the MDaemon service in
its `services` setting to watch the Windows service itself.

The [built-in rules](../alerting/rules.md#mdaemon-and-securitygateway) that
apply: MDaemon mail service down, MDaemon XML API not answering, plus Device
unreachable; with the agent, MDaemon mail queue growing, MDaemon Bad queue not
empty and MDaemon Retry queue high.

## The device page

An MDaemon device shows a **Mail services** panel above the charts, read from
the last stored measurement: opening the page never connects to the server.
It lists every watched service with its port, a word and a colour (Answering,
Down), and its response time; then the version and where it was read. Stopped
services come first.

When the agent on the same server reports MDaemon's counters, a **Mail
queues** panel follows: the number of messages in each queue (a frozen queue,
a non-empty Bad queue and more than fifty messages in Retry are flagged in
words), the active sessions, the messages and spam, virus and DNSBL verdicts
of the last 24 hours, MDaemon's uptime, and the internal servers MDaemon
reports as inactive. The same panel appears on the agent's own page.

## Mail queues through the agent

1. Install the [agent](agent.md#install) on the Windows server that runs
   MDaemon. Nothing else is needed: the agent notices the Windows service
   named `MDaemon` and reads the counters of the MDaemon performance object
   every sampling period. To force it on or off, set `mdaemon: true` or
   `mdaemon: false` in `agent.yaml`.
2. The queues appear on the agent's page. To also show them on this MDaemon
   device, DumbMonit looks for the agent on the same machine: the agent set as
   this device's parent, or the relay agent that probes it, or the agent whose
   host name matches this device's address (`mail` for `mail.example.com`),
   or, when there is one MDaemon device and one agent reporting MDaemon
   counters, that agent. If none matches, edit this device and choose the
   agent as its **Parent device**: the agent and MDaemon share the machine, so
   when the agent goes silent this device's alerts are held back as well.

The series belong to the agent, with its `target` label. All are prefixed
`dumbmonit_mdaemon_`:

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

## Watch the mail services, and optionally the XML API

1. Nothing to install on the server. With no credential, DumbMonit connects to the mail ports from the outside and reads each greeting: SMTP (25), IMAP (143) and Webmail (3000) by default. Add POP3, MSA, the TLS ports or Remote Administration in the Services option, as a list such as this one.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. To also read the version through the XML API, create a dedicated account in MDaemon, named for example as follows, with a long password used nowhere else. Give it the lowest level your MDaemon accepts for the XML API, and nothing more: DumbMonit only calls GetVersionInfo, which reads no mailbox and changes nothing.

    ```
    dumbmonit@example.com
    ```

3. Since MDaemon 24, the XML API only answers the addresses it allows. Open Setup → XML API Service → Address Restrictions (Setup → XML API Management on MDaemon 26) and allow the address DumbMonit connects from.

4. Check from the DumbMonit host that the account reaches the API. The command asks for the password and prints an answer that starts with `<MDaemon><API productversion=…>`.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. In DumbMonit, enter the server address, for example "mail.example.com", then the full email address of the account as user name, and its password. The XML API is reached on the Remote Administration HTTPS port, 444 by default.

!!! warning
    MDaemon has no read-only role for the XML API: never reuse your own account, and keep this password out of any other tool. Dynamic Screening can block an address after repeated failed logins, so a wrong password here can get the DumbMonit host blocked: test with the command above first. Remote Administration often uses a self-signed certificate: if the connection is refused for that reason, tick "Accept an unverifiable certificate" in the options. Mail queue sizes are not read by this device: MDaemon publishes them only as Windows performance counters, which the DumbMonit agent reads when it is installed on the mail server.

The XML API is served by the Remote Administration web server: port 1000 over
HTTP and 444 over HTTPS by default, at `/MdMgmtWS/`, or by IIS when Remote
Administration runs there. Its full reference ships with the server: open
`https://mail.example.com:444/MdMgmtWS` in a browser, or look in
`\MDaemon\Docs\API\XML API\`.

## Credentials

| Credential | Fields |
|---|---|
| Services only | Nothing: the mail ports are checked without logging in. |
| XML API account | The full email address of the account, and its password, sent as HTTP basic authentication. |

Address: the server's host name or IP, for example `mail.example.com`. A port
written in the address (`mail.example.com:8444`) is taken as the XML API port.

## Options

| Option | Default | Effect |
|---|---|---|
| `services` | `smtp,imap,webmail` | What to check, comma-separated: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `name:port` changes a port; any other name with its port adds a service of your own, checked by connection only. `none` checks no port. |
| `request_timeout_seconds` | `10` | Time allowed for each connection, greeting and API call, from 1 to 60. |
| `api_port` | `444` | The Remote Administration port that serves `/MdMgmtWS/`. Used only with an account. |
| `api_tls` | `true` | HTTPS to the XML API. Untick only for Remote Administration over plain HTTP (port 1000 by default): the password then crosses the network in clear. |
| `insecure_tls` | `false` | Accept a self-signed certificate on the XML API. |

The TLS ports (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) are checked by
connection only. To watch their certificates' expiry, add a
[TLS certificate](services.md#tls) device for each.
