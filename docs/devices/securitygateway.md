# SecurityGateway for Email Servers

SecurityGateway is watched with **the DumbMonit agent installed on the Windows
server that runs it**. Its Windows services, its queue folders and the disk
space of its data can only be read on the server itself. The agent reads them
locally and sends them to DumbMonit; the SecurityGateway monitor then ties the
agent to what DumbMonit checks from the outside: the gateway ports and, on
SecurityGateway 12.5 or later, the version and the performance counters its
REST API publishes (delivery queue, quarantine, SMTP sessions).

!!! info "Agent required on the mail server"

    Without the [agent](agent.md), DumbMonit only sees the gateway ports and,
    with an API key, the REST API counters. The page of the monitor says so in
    a banner: *Agent not installed or not linked*, or *Agent unreachable*.
    That banner is a warning on the page, not an outage: no notification is
    sent for it. The API key stays recommended: it is what brings the
    performance counters; the agent brings the rest.

| | With the agent on the server | Without it |
|---|---|---|
| Gateway ports (SMTP, web interface) | Checked | Checked |
| Windows services of SecurityGateway (state, start mode) | Read | Not watched |
| Mail queue folders (messages and oldest message) | Read | Not watched |
| Disk space of the data | Read | Not watched |
| Performance counters (delivery queue, quarantine, sessions) and version | With an API key | With an API key |

## Network ports to open

From the agent to the DumbMonit server:

| Port | Protocol | Purpose |
|---|---|---|
| 8080 | TCP | Agent readings (default port, 443 behind a TLS proxy) |

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 25 | TCP | SMTP check (default) |
| 4000 | TCP | Web interface check (default) |
| 4443 | TCP | Only with an API key (HTTPS, default) or web_https in the Services option |
| 465 | TCP | Only with smtps in the Services option |

The agent calls the server, never the other way round: no port needs to be
opened on the gateway server for it.

!!! warning "Validated against the documentation only"

    This integration was built from MDaemon Technologies' published
    documentation of SecurityGateway and of its REST API, not against a real
    gateway. Every call is a read. The documentation does not publish the
    path or the names of the performance counters, so DumbMonit finds the
    path in the API's own description and keeps every counter under the name
    the API gives it. The names of SecurityGateway's Windows services and the
    layout of its queue folders are not documented either: the agent looks
    for services whose name contains `SecurityGateway` and counts the
    messages in the sub-folders of `Queues` under the install folder. Nobody
    has yet run it against a live gateway. Tell us what breaks.

## Install

1. Install the [agent](agent.md#install) on the Windows server that runs
   SecurityGateway: in DumbMonit, add a **Server with agent** monitor and run
   its PowerShell command in an elevated PowerShell. From **Add a device** →
   **SecurityGateway for Email Servers**, the *Install the Windows agent*
   button opens that flow with the PowerShell command only, and the *Add the
   SecurityGateway monitor* button brings you back.
2. Add the SecurityGateway monitor and choose that agent in **Agent installed
   on this server**. When the address you type has the host name of an agent,
   DumbMonit pre-selects it.
3. Paste an API key (recommended, SecurityGateway 12.5 or later) to read the
   performance counters. The agent covers the rest.

The agent detects SecurityGateway without any setting: it looks for a Windows
service whose name or display name contains `SecurityGateway`, and takes the
install folder from the path of its executable. To switch this off, set
`mdaemon: false` in `agent.yaml` (the same setting covers both products).

## What it watches

### Read by the agent {#read-by-the-agent}

Prefixed `dumbmonit_agent_mail_`, on the agent's `target`, with
`product="securitygateway"`.

| Metric | Labels | Meaning |
|---|---|---|
| `info` (value 1) | `product`, `version` | SecurityGateway was found on the server, with the product version of its executable (absent when unreadable). |
| `service_up` | `product`, `service`, `display`, `start` | 1 if the Windows service is running. `start` is how Windows starts it: `auto`, `manual`, `disabled` or `other`. |
| `queue_files` | `product`, `queue` | Messages (`.msg` files) in each sub-folder of `Queues` under the install folder, named in lower case. At most 20 folders, and 20,000 files per folder. |
| `queue_oldest_seconds` | `product`, `queue` | Age of the oldest message of a non-empty queue folder. |
| `data_free_bytes`, `data_total_bytes` | `product` | Free and total space of the volume that holds the install folder. |

No Windows performance counter of SecurityGateway is read by default, since
its documentation does not name an object. If your server publishes some, list
them under `perf_counters` in `agent.yaml` (see
[Windows performance counters](agent.md#windows-performance-counters)).

### Network services and API

Checked by DumbMonit from the outside. All metrics are prefixed
`dumbmonit_securitygateway_`.

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
no service and no API answers at all is the monitor *unreachable*. A refused
key is a configuration error, shown on the monitor page and not notified.

**What is not read.** The license and its expiry, and the message statistics
of the dashboard, are not described by the published documentation of the
REST API.

The [built-in rules](../alerting/rules.md#mdaemon-and-securitygateway) that
apply: SecurityGateway service down, SecurityGateway API not answering and
SecurityGateway delivery queue growing (from the monitor), plus Monitor
unreachable; and, on the agent's monitor, Mail server Windows service stopped.

## The monitor page

The page of a SecurityGateway monitor shows, from top to bottom:

1. A **banner** when the agent is missing or silent (*Agent not installed or
   not linked*, *Agent unreachable*, or *The agent does not see
   SecurityGateway on its machine*). It is a warning, not an alert.
2. **Read by the agent**: the *Windows services*, the *Queues* (messages per
   folder and the age of the oldest one), the version and the disk space.
3. **Network services**: every watched port, a word and a colour (Answering,
   Down) and its response time; then, under **API**, the REST API state, the
   version, and with an API key, every performance counter with its latest
   value.

Everything is read from the last stored measurement: opening the page never
connects to the gateway.

## Install the agent, then add the monitor

1. On the Windows server that runs SecurityGateway, add a Server with agent monitor in DumbMonit and run its PowerShell command in an elevated PowerShell. The agent finds SecurityGateway by itself.

2. Back here, enter the gateway address and choose that agent in Agent installed on this server. DumbMonit pre-selects the one with the same host name.

3. Recommended, for the performance counters: in SecurityGateway 12.5 or later, open Setup/Users → Accounts → Administrators → New and create the account below, with a long unique password.

    ```
    dumbmonit
    ```

4. Give the account the role listed below, and restrict it to the DumbMonit address in IP Restriction Options.

5. Sign in as that account, open Setup/Users → Accounts → API Keys and create a key with an expiry date, then paste the key in DumbMonit.

6. To check more ports than SMTP and the web interface, set the Services option to a list like this one.

    ```
    smtp,web_https
    ```

### Required permissions

- **Local administrator on the server**: Only to install the agent, which then runs as a Windows service
- **Domain Administrator**: Use Global Administrator if the counters stay empty

Vendor documentation: <https://help.mdaemon.com/SecurityGateway/en/api_keys.html>

### Good to know

The Windows services, queue folders and disk space are read by the agent, with the rights of the Windows service that runs it (Local System by default). Nothing is written on the server.

The role is Domain Administrator; if the counters stay empty on the monitor page, your version keeps them for the Global Administrator role, and the account needs that role instead. A key carries the rights of the account that creates it, which is why it must be created from the dedicated account and not from yours. API keys obey the account's IP restrictions.

The API is reached on the web interface HTTPS port, 4443 by default, under `/api/v1`. To check from the DumbMonit host that the key works, run the command below: it prints the API description, which also lists the counters DumbMonit reads.

```
curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
```

The web interface often uses a self-signed certificate on 4443: if the connection is refused for that reason, tick "Accept an unverifiable certificate" in the options.

!!! warning
    The Windows services, mail queue folders and disk space of SecurityGateway can only be read on the server itself: without the agent, DumbMonit sees the gateway ports and, with a key, the REST API counters. SecurityGateway has no read-only role or narrower key: create the key from the dedicated account, never your own, and give it an expiry date.

## Credentials

| Credential | Fields |
|---|---|
| Services only | Nothing: the ports are checked without logging in. The agent still brings the Windows services, queue folders and disk space. |
| API key (12.5 and later, recommended) | The key, sent as `Authorization: Bearer …`. It adds the version and the performance counters (delivery queue, quarantine, sessions). |

Address: the gateway's host name or IP, for example `sg.example.com`. A port
written in the address (`sg.example.com:8443`) is taken as the API port.

## Options

| Option | Default | Effect |
|---|---|---|
| `services` | `smtp,web` | What to check, comma-separated: `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `name:port` changes a port; any other name with its port adds a service of your own, checked by connection only. `none` checks no port. |
| `agent_id` | none | The monitor of the agent installed on the SecurityGateway server (chosen in the form; the id of the agent's monitor). |
| `request_timeout_seconds` | `10` | Time allowed for each connection, greeting and API call, from 1 to 60. |
| `api_port` | `4443` | The web interface port that serves `/api/v1`. Used only with an API key. |
| `api_tls` | `true` | HTTPS to the REST API. Untick only for the web interface over plain HTTP (port 4000 by default): the key then crosses the network in clear. |
| `insecure_tls` | `false` | Accept a self-signed certificate on the REST API. |
| `counters` | `true` | Read the performance counters. |
