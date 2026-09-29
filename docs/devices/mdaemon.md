# MDaemon Email Server

The mail services of an MDaemon server, checked from the outside: SMTP, IMAP,
POP3, webmail and Remote Administration accept connections and greet the way
they should. With an account, DumbMonit also reads the exact version through
the XML API.

!!! warning "Validated against the documentation only"

    This integration was built from MDaemon Technologies' published
    documentation and the XML API response format shown there, not against a
    real MDaemon server. Every call is a read, and the mail-port checks follow
    the protocol standards, but nobody has yet run it against a live MDaemon.
    Tell us what breaks.

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

**What is not read, and why.** MDaemon publishes its queue sizes (local,
remote, retry, bad, holding, quarantine), its session counts and its uptime
as Windows performance counters, in the object named MDaemon, and in its own
Queue and Statistics Manager. None of MDaemon's published documentation
describes an XML API call that returns them, so DumbMonit does not guess one.
The license, mail store disk space and message statistics are not exposed
through a documented interface either. To watch the MDaemon Windows service
itself and the disk that holds the mail store, install the
[agent](agent.md) on the server and list the MDaemon service in its
`services` setting.

The [built-in rules](../alerting/rules.md#mdaemon-and-securitygateway) that
apply: MDaemon mail service down, MDaemon XML API not answering, plus Device
unreachable.

## The device page

An MDaemon device shows a **Mail services** panel above the charts, read from
the last stored measurement: opening the page never connects to the server.
It lists every watched service with its port, a word and a colour (Answering,
Down), and its response time; then the version and where it was read. Stopped
services come first.

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
    MDaemon has no read-only role for the XML API: never reuse your own account, and keep this password out of any other tool. Dynamic Screening can block an address after repeated failed logins, so a wrong password here can get the DumbMonit host blocked: test with the command above first. Remote Administration often uses a self-signed certificate: if the connection is refused for that reason, tick "Accept an unverifiable certificate" in the options. Mail queue sizes are not read: MDaemon publishes them only as Windows performance counters.

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
