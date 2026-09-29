# SecurityGateway for Email Servers

The services of a SecurityGateway mail gateway, checked from the outside, and
on SecurityGateway 12.5 or later, the version and the performance counters its
REST API publishes: delivery queue, quarantine, SMTP sessions.

!!! warning "Validated against the documentation only"

    This integration was built from MDaemon Technologies' published
    documentation of SecurityGateway and of its REST API, not against a real
    gateway. Every call is a read. The documentation does not publish the
    path or the names of the performance counters, so DumbMonit finds the
    path in the API's own description and keeps every counter under the name
    the API gives it. Nobody has yet run it against a live gateway. Tell us
    what breaks.

## What it watches

All metrics are prefixed `dumbmonit_securitygateway_`.

| Metric | Labels | Meaning |
|---|---|---|
| `service_up` | `service`, `port` | 1 if the port accepted the connection and, for SMTP, greeted with `220`. 0 if it did not answer, or answered to refuse service (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Time to the greeting, or to the connection for a port without one. |
| `api_up` | | With an API key only: 1 if the REST API answered, 0 if it did not. |
| `api_response_seconds` | | Time the REST API took to answer. |
| `counters_available` | | 1 if the performance counters were read; 0 if the API describes none, or the key's account may not read them. |
| `counter` | `counter` | One series per numeric value of the performance counters, named as the API names it, in `snake_case`: `deliveryQueue` becomes `delivery_queue`. At most 64. |
| `info` (value 1) | `version`, `source` | The version given by the API's description (`source="openapi"`), or else by the SMTP greeting when the gateway announces it there (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Time the whole check took. |

**How the counters are found.** SecurityGateway's REST API describes itself
at `/api/v1/openapi`. DumbMonit reads that description, takes the shortest
read-only path without parameters whose name mentions counters, reads it, and
keeps every number in its answer. Nothing is renamed and nothing is assumed:
if a future version renames a counter, the series follows the new name. The
counters listed in SecurityGateway's help are the active inbound and outbound
SMTP sessions, the delivery queue, the administrative quarantine, the uptime,
the domain and user counts and the database connection pool; some of them
only change once a minute.

**A partial outage is still a successful check.** One stopped service gives
`service_up 0` for that service and the others are still measured. Only when
no service and no API answers at all is the device *unreachable*. A refused
key is a configuration error, shown on the device page and not notified.

**What is not read.** The license and its expiry, the disk space and the
message statistics of the dashboard are not described by the published
documentation of the REST API. To watch the SecurityGateway Windows service
itself and the server's disks, install the [agent](agent.md) on the server
and list the service in its `services` setting.

The [built-in rules](../alerting/rules.md#mdaemon-and-securitygateway) that
apply: SecurityGateway service down, SecurityGateway API not answering,
SecurityGateway delivery queue growing, plus Device unreachable.

## The device page

A SecurityGateway device shows a **Mail services** panel above the charts,
read from the last stored measurement: opening the page never connects to the
gateway. It lists every watched service with its port, a word and a colour
(Answering, Down) and its response time, the version, and with an API key,
every performance counter with its latest value.

## Watch the gateway services, and optionally the REST API

1. Nothing to install on the server. With no credential, DumbMonit connects to SMTP (25) and the web interface (4000) from the outside and reads the SMTP greeting. Change the list in the Services option, for example to check HTTPS instead of HTTP.

    ```
    smtp,web_https
    ```

2. To also read the performance counters, SecurityGateway 12.5 or later is needed. Create a dedicated account named as follows, with a long password used nowhere else, and give it the Domain Administrator role. If the counters stay empty on the device page, your version keeps them for the Global Administrator role, and the account needs that role instead.

    ```
    dumbmonit
    ```

3. Sign in as that account and create a key under Setup/Users → Accounts → API Keys, with an expiry date. A key carries the rights of the account that creates it, which is why it must be created from this account and not from yours.

4. Restrict the account to the address DumbMonit connects from in its IP restrictions: API keys obey them too.

5. Check from the DumbMonit host that the key works. The command prints the API description, which also lists the counters DumbMonit reads.

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. In DumbMonit, enter the gateway address, for example "sg.example.com", and paste the key. The API is reached on the web interface HTTPS port, 4443 by default, under /api/v1.

!!! warning
    SecurityGateway has no read-only role and no narrower key: the key can do everything its account can. Never create it from your own account, keep it only in DumbMonit and give it an expiry date. Versions before 12.5 have no REST API: leave the credential empty and DumbMonit checks the services only. The web interface often uses a self-signed certificate on 4443: if the connection is refused for that reason, tick "Accept an unverifiable certificate" in the options.

## Credentials

| Credential | Fields |
|---|---|
| Services only | Nothing: the ports are checked without logging in. |
| API key (12.5 and later) | The key, sent as `Authorization: Bearer …`. |

Address: the gateway's host name or IP, for example `sg.example.com`. A port
written in the address (`sg.example.com:8443`) is taken as the API port.

## Options

| Option | Default | Effect |
|---|---|---|
| `services` | `smtp,web` | What to check, comma-separated: `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `name:port` changes a port; any other name with its port adds a service of your own, checked by connection only. `none` checks no port. |
| `request_timeout_seconds` | `10` | Time allowed for each connection, greeting and API call, from 1 to 60. |
| `api_port` | `4443` | The web interface port that serves `/api/v1`. Used only with an API key. |
| `api_tls` | `true` | HTTPS to the REST API. Untick only for the web interface over plain HTTP (port 4000 by default): the key then crosses the network in clear. |
| `insecure_tls` | `false` | Accept a self-signed certificate on the REST API. |
| `counters` | `true` | Read the performance counters. |
