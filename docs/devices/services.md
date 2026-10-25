# Services: HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket

Eleven monitors watch **services** rather than equipment, Uptime Kuma style: a
web page, a port, a DNS name, a host, a certificate, a time server, a mail
relay, a database, an MQTT broker, a WebSocket endpoint. Each service gets a
history bar, a response time and an availability percentage on its monitor
page.

The last five go further than opening a connection: they begin a real session in
the service's own protocol. That is the difference between "port 5432 is open"
and "the database still accepts my account and answers a query". NTP sits
between the two: a single request and reply, in NTP's own protocol, enough to
read the server's clock without a session.

## Coming from Uptime Kuma

**Monitors > ⋯ > Import from Uptime Kuma**, also on **Monitors > Add a
monitor > Import** (administrators), reads the database of Uptime Kuma 1.x or
2.x, or the JSON backup of 1.x. Nothing is created before you have seen the
preview: what will be created, and what is skipped with the reason. Tick the
monitors you want, then import them in one click.

Uptime Kuma 2.x has no backup export any more, so export its database instead:
stop the container (so the file is complete), then copy `kuma.db` out of the
data folder and choose it in the dialog.

```bash
docker stop uptime-kuma
docker cp uptime-kuma:/app/data/kuma.db .
docker start uptime-kuma
```

- The file is uploaded, read once in read-only mode from a temporary copy that
  is never attached to DumbMonit's own database, and deleted straight away. It
  can be up to 512 MB (it holds the whole heartbeat history); only monitors,
  groups, tags, intervals and the number of notifications are read from it.
- Uptime Kuma 2 installed on **MariaDB** is not supported: only the SQLite file
  `kuma.db` can be imported.
- The JSON backup of 1.x (**Settings > Backup > Export**, at most 4 MB) still
  works, with the same preview.

| Uptime Kuma | DumbMonit |
|---|---|
| HTTP(s), HTTP(s) - Keyword, HTTP(s) - Json Query | [HTTP](#http): accepted status codes, method, keyword (and its inverse), JSON path and expected value, redirects, timeout, ignore TLS errors |
| Ping | [Ping](#ping) |
| TCP Port | [TCP](#tcp), as `host:port` |
| DNS | [DNS](#dns): record type and resolver |
| Push | [Heartbeat](push.md), expected interval = the monitor's interval. It gets a **new** push URL: update the job that calls it |
| Group | A folder (nested groups become `Group/Subgroup`) |
| Tags | Monitor tags: `name=value`, or `name=true` for a tag without a value |
| Paused | Created disabled |

The name and interval are kept (10 s minimum). Everything else is skipped and
listed: Docker and the other types without an equivalent, and monitors in
upside-down mode. An entry already monitored (same type and address) is skipped
too, so the same file can be imported twice safely.

**Not imported:** notifications (set up your channels in
[Notifications](../notifications.md); the preview tells you how many the file
held), and credentials, custom headers and request bodies, which are never
read into DumbMonit. If a monitored site needs a login or a header, add it on the
monitor afterwards.

## How service state works

A service can answer and still be down: a `500`, a missing keyword, an expired
certificate. So these monitors write a point on every check, `probe_success`
being 1 or 0, instead of going silent like a hardware device. That is what
makes an availability percentage possible: `avg_over_time(dumbmonit_probe_success[30d])`.

| Situation | What happens |
|---|---|
| Invalid option or address, ICMP sockets not allowed (see Ping) | Configuration error: nothing is written, the monitor page shows the error, no "down" alert. |
| Connection refused, timeout, TLS refused, `500`, keyword absent, total packet loss, password refused, query failed | `probe_success = 0`: the outage is recorded, dated and counted. |
| All good | `probe_success = 1`. |

Every monitor has its own timeout (`timeout_seconds`, 5 s by default, 60 s at
most), shorter than the server's `DUMBMONIT_PROBE_TIMEOUT_SECS`: interrupted by
the scheduler, it could not write its zero.

Built-in rules that apply to all eleven: Service down (3 minutes), Service
flapping (more than six state changes in thirty minutes), Slow service (more
than 3 s for ten minutes, successful checks only: a failing check lasts its
timeout and is Service down, not slowness). For every check that reads a certificate — `http`,
`tls`, `smtp`, `mqtt`, `websocket`: Certificate expiring soon (14 days) and
Certificate expired.

A refused password is never an outage of the service: it is recorded as
`reason="auth"`, so the alert says what to fix.

### Metrics

All gauges, all prefixed `dumbmonit_`, all labelled
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket|ntp|snmp_oid|radius|grpc"` in addition
to `target`, `host` and `tag_*`.

| Metric | Monitors | Meaning |
|---|---|---|
| `probe_success` | all | 1 if the service answers correctly, 0 otherwise. |
| `probe_duration_seconds` | all | Total duration of the check. |
| `probe_failure_info` | all | Presence (1) with a `reason` label: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`, `threshold`, `not_found`, `bad_secret`, `unhealthy`. |
| `probe_http_status_code` | http, websocket | Status code obtained. |
| `probe_http_first_byte_seconds` | http | Time to the response headers. |
| `probe_http_content_bytes` | http | Body size. |
| `probe_http_security_header{header}` | http | 0/1 per `header` label (https only): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `probe_connect_seconds` | all but dns and ping | Connection establishment (login included for databases). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | TLS negotiation alone. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Presence; `version` label. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Days before expiry, negative if expired. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 if the chain leads to a known authority. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Presence; `issuer` label. |
| `probe_dns_lookup_seconds` | dns | Resolution time. |
| `probe_dns_answer_records` | dns | Records of the requested type. |
| `probe_dns_answer_fingerprint` | dns | With "Alert when the answer changes": a 32-bit fingerprint of the sorted answer. Its value means nothing; only its changes do. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Average, shortest and longest round trip. |
| `probe_icmp_packet_loss_ratio` | ping | Loss between 0 and 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Echoes sent and received. |
| `probe_smtp_greeting_seconds` | smtp | Time to the `220` banner. |
| `probe_smtp_ehlo_seconds` | smtp | Round trip of the `EHLO` command. |
| `probe_smtp_capabilities` | smtp | Extensions advertised. |
| `probe_smtp_authenticated` | smtp | 1 when `AUTH` succeeded. |
| `probe_sql_query_seconds` | postgres, mysql | Query execution alone. |
| `probe_sql_rows` | postgres, mysql | Rows returned. |
| `probe_sql_value` | postgres, mysql | First column of the first row, when it is a number. |
| `probe_mqtt_connack_seconds` | mqtt | Time to the `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Time to the `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Size of the retained message. |
| `probe_mqtt_message_value` | mqtt | Its content, when it is a number. |
| `probe_ws_handshake_seconds` | websocket | Opening handshake. |
| `probe_ntp_offset_seconds` | ntp | Offset between the local clock and the server. |
| `probe_ntp_delay_seconds` | ntp | Network round-trip delay. |
| `probe_ntp_stratum` | ntp | Distance to a reference clock (16: not synchronized). |
| `probe_ntp_leap_indicator` | ntp | 0 to 3 (leap second warning, 3 = clock unsynchronized). |
| `probe_ntp_root_delay_seconds` | ntp | Cumulated delay up to the root of the hierarchy. |
| `probe_ntp_root_dispersion_seconds` | ntp | Cumulated dispersion up to the root. |
| `probe_ntp_reference_info` | ntp | Presence; `reference_id` label. |
| `probe_ws_message_bytes` | websocket | Size of the first frame received. Only present when the check sends a message or expects a reply (`send` / `expect` options). |
| `probe_snmp_query_seconds` | snmp_oid | Round trip of the read. |
| `probe_snmp_value` | snmp_oid | Value of the object, when it is a number. |
| `probe_radius_response_seconds` | radius | Round trip of the Access-Request. |
| `probe_radius_response_code` | radius | Code of the reply: 2 accepted, 3 rejected, 11 challenge. |
| `probe_grpc_check_seconds` | grpc | Duration of the `Health/Check` call. |
| `probe_grpc_serving_status` | grpc | `ServingStatus` returned: 1 SERVING, 2 NOT_SERVING, 3 SERVICE_UNKNOWN. |
| `probe_grpc_status_code` | grpc | `grpc-status` code of the reply (0: OK). |

## HTTP

**Website or web API (HTTP)** — checks that a page or an API answers, with the
right status code, the right content, and a valid certificate. Examples: a
website, an application health page, a REST API, a self-hosted service
interface.

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Web page or API over HTTPS (default port) |
| 80 | TCP | Only with an http:// address (default port) |

### Setup

1. In the address, paste the full URL of the page to monitor.

2. Prefer a light page such as "/health" or "/status" over the home page.

3. If the page needs a login, choose a credential: username and password (basic) or a token (Bearer).

4. Optionally require a keyword in the page or a value in a JSON response, in the options.

**Required permissions**

- **Read access to the monitored URL**: Only if the page requires a login

!!! warning
    Never write a password or a token in the options: use the credential field, which is encrypted.

Without "http://" or "https://", HTTPS is assumed. By default, any status code between 200 and 299 is fine. Over HTTPS, the certificate is read automatically: you will be warned before it expires. A password or token written in the options is copied in clear text on every measurement.

Credentials: none, username / password (basic authentication) or API token
(`Authorization: Bearer`).

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `method` | HTTP method | `GET` | GET fits almost every case. HEAD avoids downloading the page when only the status code matters. Choices: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Accepted status codes | `200-299` | Codes or ranges considered normal, separated by commas. Any other code counts as down. Example: `200-299,301,404`. |
| `keyword` | Expected keyword | *(empty)* | Text that must appear in the page. If it is missing, the service is reported down even though the page answers. |
| `keyword_absent` | Keyword must be absent | `false` | Inverts the check: the presence of the keyword signals a failure. Useful for an error page that answers 200. |
| `keyword_case_sensitive` | Match keyword case | `false` | By default, upper and lower case are treated the same. |
| `json_path` | JSON path to check | *(empty)* | For a JSON response: path of the value to check. Fill it in together with the expected value, never one without the other. Example: `$.status`. |
| `json_expect` | Expected JSON value | *(empty)* | Value the JSON path above must have, for example "ok" or "true". |
| `headers` | Extra headers | *(empty)* | One or more "Name: value" headers, separated by "\|". Do not put secrets here: options are visible in the charts. |
| `body` | Request body | *(empty)* | Content sent with the request, for methods that expect one (POST, PUT…). |
| `follow_redirects` | Follow redirects | `true` | Disable to monitor the redirect itself, for example a 301 to HTTPS. |
| `max_redirects` | Maximum redirects followed | `10` | Between 1 and 20. No effect if redirects are not followed. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | The check no longer fails on a self-signed certificate or one issued by a private authority. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | By default the check refuses addresses only the DumbMonit host itself can reach (127.0.0.1, ::1, 169.254.x.x). Enable this to monitor a service running on the DumbMonit host. Private LAN addresses (10.x, 192.168.x) are always allowed. |
| `check_certificate` | Read the certificate | `true` | Over HTTPS, also records the certificate expiry date so you can be warned before it expires. |
| `max_body_bytes` | Maximum bytes read | `524288` | Beyond this, the rest of the page is not downloaded. The keyword and JSON path are only searched in this part. |
| `user_agent` | Announced identity (User-Agent) | `DumbMonit/<version>` | Name the check gives to the server. Change it if the server filters robots. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

## TCP

**Network port (TCP)** — checks that a port accepts connections: SSH, database,
file share, game server, network printer.

### Network ports to open

The port is the one you write in the address or in the "Port" option; there is no default. From the DumbMonit server (or its relay agent) to the device, open that TCP port.

### Setup

1. In the address, write the host and the port separated by a colon: "nas.home.lan:22", or "[fd00::1]:445" for IPv6.

2. Or leave the port out of the address and fill in the "Port" option.

!!! warning
    An open port does not prove the application works: for a web service, use the "Website or web API" type.

Only the opening of the connection is tested: no data is sent to the service, so it has no effect on it.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `port` | Port | *(empty)* | Port to open, if the address does not already give it as "host:port". One of the two is required. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | By default the check refuses addresses only the DumbMonit host itself can reach (127.0.0.1, ::1, 169.254.x.x). Enable this to monitor a service running on the DumbMonit host. Private LAN addresses (10.x, 192.168.x) are always allowed. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

## DNS

**Domain name (DNS)** — checks that a name resolves, and that it points to the
right address. Examples: your domain name, an internal name served by your
Pi-hole or AdGuard, an MX record.

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 53 | UDP | DNS query (default port) |
| 53 | TCP | DNS query too large for UDP (default port) |

### Setup

1. In the address, write the name to resolve, without "http://": "www.example.com".

2. Pick another record type in the options if you do not want an IPv4 address (A record).

3. To monitor your own DNS server, enter its IP address in "DNS server to query".

4. To detect hijacking, list the addresses the answer must contain in "Expected values".

!!! warning
    Give the DNS server by its IP address, not by a name.

By default, the check asks the system resolver. If the server you query stops answering, the check fails.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `record_type` | Record type | `A` | A for an IPv4 address, AAAA for IPv6, MX for mail, CNAME for an alias… Choices: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | DNS server to query | *(empty)* | IP address of a resolver, port optional. Empty: the system resolver. Handy to check your own DNS server. Example: `1.1.1.1` or `10.0.0.1:5353`. |
| `expect` | Expected values | *(empty)* | Values that must all appear in the answer, separated by commas. Empty: only the resolution is checked. |
| `expect_mode` | Comparison | `contains` | `contains`: each expected value must appear somewhere in the answer. `exact`: the answer must hold those values and nothing else. |
| `forbid` | Forbidden values | *(empty)* | Values that must never appear in the answer, separated by commas. Checked before the expected ones. |
| `alert_on_change` | Alert when the answer changes | `false` | Records a fingerprint of the sorted answer and alerts whenever it changes, without having to write the expected values. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Asserting what a zone answers

`expect` alone catches the record that disappeared. It does not catch the record
that was *added*: a hijacked zone often keeps the legitimate `A` and puts a
second one beside it, and a client then reaches either one at random.

- `expect_mode = exact` refuses any value the expectation does not cover. Order
  does not matter — DNS does not keep one.
- `forbid` names values that must never come back: the address of a former host,
  an expired validation `TXT`, a mail server you decommissioned. It is checked
  first, so it wins over `expect`.

Both compare case-insensitively, ignore the trailing dot of a name and the
quotes around a `TXT`, and treat an expected value as found when it appears
inside a record — so an `MX` matches on `mail.example.com` without you having to
copy its priority.

### Alerting when the answer changes

Writing the expected values is not always practical: a name served by a CDN,
records someone else manages, or simply a zone you want to hear about whenever
it moves. Tick **Alert when the answer changes**: every check then records a
fingerprint of the answer, and the built-in rule
[DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains) fires
when it differs from the one before. The alert resolves by itself an hour
later; the new answer is the reference from then on.

The fingerprint covers the values of the answer, sorted and without duplicates,
with the CNAME chain included: a round-robin that returns the same addresses
in another order does not change it, nor does a TTL. A name whose answer
changes on every query (a large pool such as `pool.ntp.org`, or geographic DNS
that answers differently from one resolver to the next) would alert all the
time: leave the option off for those, or query one fixed resolver.

## Ping

**Reachable host (ping)** — sends a few ICMP echoes and measures response time
and packet loss. Examples: home router or gateway, Wi-Fi access point, printer,
machine without agent or SNMP, remote host.

### Network ports to open

Ping uses ICMP echo, which has no port: allow ICMP echo request and reply from the DumbMonit server (or its relay agent) to the device.

### Setup

1. In the address, write the host name or IP address: "192.168.1.1" or "router.home.lan".

2. Lower "Tolerated loss" in the options to be warned before the loss is total.

!!! warning
    In Docker, keep the sysctls lines of the shipped docker-compose.yml, or the check cannot send pings.

On each poll, the check sends four echoes and measures the response time and the share of lost packets. By default, only a total loss counts as down; partial loss stays visible in the charts. Some devices ignore pings on purpose.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Outside Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"` on the
host does the same, and a raw socket (`NET_RAW`, root) works too.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `count` | Number of echoes | `4` | Packets sent on each poll, from 1 to 20. |
| `packet_timeout_ms` | Wait per packet (ms) | `1000` | Time allowed for each reply, from 50 to 10,000 ms. |
| `interval_ms` | Gap between two echoes (ms) | `100` | Pause between two echoes, from 0 to 5,000 ms. |
| `payload_bytes` | Payload size (bytes) | `56` | Data carried by each echo, from 0 to 1,400 bytes. |
| `ip_version` | IP version | `auto` | "auto" takes the first resolved address; force 4 or 6 if the host has both and one does not answer. Choices: auto, 4, 6. |
| `loss_threshold_percent` | Tolerated loss (%) | `100` | Above this percentage of lost packets, the host is reported down. 100: only a total loss counts, partial loss stays visible in the charts. |
| `timeout_seconds` | Timeout (seconds) | `5` | Total budget for the poll, from 1 to 60. Refused if it does not cover "number of echoes × wait per packet". |

## TLS

**TLS certificate** — checks that a certificate is valid and warns before it
expires, on any encrypted port. Examples: mail server (IMAPS, SMTPS), reverse
proxy, LDAPS directory, MQTT broker, administration interface.

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | TLS handshake (default port, the one in the address) |

### Setup

1. In the address, write the host and the port of the encrypted service: "mail.example.com:993" (443 if omitted).

2. If you reach the service by its IP address, enter the certificate's name in "Server name (SNI)".

3. For a certificate from your own authority, tick "Accept an unverifiable certificate".

!!! warning
    For a website, the "Website or web API" type already reads the certificate: use this type for services that do not speak HTTP.

The check opens an encrypted connection, reads the presented certificate and closes right away: no data is exchanged with the application. It records the number of days before expiry, the issuer and the TLS version. The bundled alert rules warn at fourteen days, then at expiry.

The handshake completes even when the certificate is refused: that is what
allows reading the expiry date of a certificate that has already expired. The
verdict of the verification is in `probe_ssl_cert_valid`.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `server_name` | Server name (SNI) | *(empty)* | Domain name announced to the server and checked in the certificate. Set it when the address is an IP behind a reverse proxy. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | An unverifiable chain no longer counts as a failure: the expiry date is still recorded. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | By default the check refuses addresses only the DumbMonit host itself can reach (127.0.0.1, ::1, 169.254.x.x). Enable this to monitor a service running on the DumbMonit host. Private LAN addresses (10.x, 192.168.x) are always allowed. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

## SMTP

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 587 | TCP | Submission with STARTTLS (default) |
| 465 | TCP | Only with TLS encryption (SMTPS) |
| 25 | TCP | Only with no encryption (relay on port 25) |

**Mail relay (SMTP)** — opens a real session on a mail server: greeting, EHLO,
STARTTLS, and the login if you give one. Examples: your provider's SMTP relay,
a local Postfix or msmtp, Proxmox Mail Gateway, the submission port of a mail
server.

### Setup

1. In DumbMonit, enter the mail server, for example smtp.example.com:2525 (the usual port for the encryption by default).

2. Pick the encryption: STARTTLS for port 587, TLS for port 465, None for a relay on your own network on port 25.

3. Optional: fill in an account, to check that the relay still accepts it (a refused password is not reported as an outage).

4. Optional: name an Expected extension, such as STARTTLS, to be warned when the relay stops advertising it.

#### Required permissions

- **A mail account that may log in (AUTH)**: Only if you fill in a credential

!!! warning
    An accepted session does not prove mail leaves: watch the relay's queues for that.

#### Why these permissions

Nothing is ever sent: the check stops after the greeting, the extensions and the optional login, then hangs up. No message enters the queue. A relay that stops advertising STARTTLS is a relay that would send your mail in clear text. A relay that accepts everything and queues it forever answers perfectly here.

Credentials: none, or a username and password sent with `AUTH PLAIN` (or
`AUTH LOGIN` when the server offers only that). They are sent **after** the
connection is encrypted and after the certificate has been judged: a relay whose
chain cannot be verified never receives them.

Over TLS or STARTTLS, the certificate is read like the TLS check reads it —
expiry, issuer, protocol version — so a mail relay is also a certificate you are
warned about.

### What it cannot detect

That mail actually leaves. A relay that accepts every session and then defers
everything answers perfectly here; watch its queues instead (see the Proxmox
Mail Gateway page). It also never sends a message, so it says nothing about
recipient filtering, SPF or DKIM.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `security` | Encryption | `starttls` | STARTTLS starts in clear text and upgrades (port 587), TLS encrypts from the first byte (port 465), None never encrypts (port 25) and only suits a relay on your own network. |
| `port` | Port | *(empty)* | Used if the address does not give one. Empty: 587 with STARTTLS, 465 with TLS, 25 without encryption. |
| `helo_name` | Name announced (EHLO) | `dumbmonit` | Name the check gives when it introduces itself. A strict relay refuses a name it cannot resolve. |
| `expect_capability` | Expected extension | *(empty)* | Extension the server must advertise in its EHLO answer, for example STARTTLS or AUTH. Empty: no expectation. |
| `server_name` | Server name (SNI) | *(the host)* | Domain name announced to the server and checked in the certificate. Set it when the address is an IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | An unverifiable chain no longer counts as a failure: the expiry date is still recorded. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons

| `reason` | What happened |
|---|---|
| `connect` | The port is closed or the relay is unreachable. |
| `protocol` | Something answered, but not SMTP — or it refused the connection, `EHLO`, or STARTTLS it had advertised. |
| `auth` | The account was refused (`535`, `530`, `534`). The relay is fine; the credential is not. |
| `payload` | The extension named in "Expected extension" is not advertised. |
| `tls`, `cert_expired` | Same meaning as for the TLS check. |

## PostgreSQL

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 5432 | TCP | PostgreSQL (default port) |

**PostgreSQL database** — connects, authenticates and runs one query: the signal
for a database that is up but no longer answering. Examples: the database behind
Nextcloud or Immich, a Home Assistant recorder, an application database, a
TimescaleDB instance.

### Setup

1. In psql, create a role that may only log in.

    ```
    CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
    ```

2. Allow it to connect from the DumbMonit host with this line in pg_hba.conf (adjust the address), then reload PostgreSQL.

    ```
    host all dumbmonit 10.0.0.5/32 scram-sha-256
    ```

3. In DumbMonit, enter the server, for example db.home.lan:5433 (port 5432 by default), then the role's user and password.

4. Optional: replace the default query SELECT 1 with one that returns a single number, to chart it.

#### Required permissions

- **LOGIN (role attribute)**: Connect only, no other privilege

!!! warning
    The check opens a real connection on every poll: on an instance close to its connection limit, give it a longer interval.

#### Why these permissions

The default query is SELECT 1, which reads no table: nothing else has to be granted. Connection time and query time are recorded separately. A query of your own that reads a table needs SELECT on that table.

Credentials: a username and password, required. A database is never monitored
anonymously, so the form offers nothing else.

### What it cannot detect

Replication lag, a lock that drags on, a table that keeps growing. All of those
live in the engine's own administration views — write the query yourself, and
the number it returns becomes `probe_sql_value`, a chart you can alert on.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `port` | Port | `5432` | Used if the address does not give one. |
| `database` | Database | `postgres` | Database opened on connection. The monitoring account must be allowed to connect to it. |
| `query` | Query | `SELECT 1` | Run on every check. The default reads no table, so the account needs no privilege beyond connecting. A query returning one number turns it into a chart. |
| `expect` | Expected value | *(empty)* | Value the first column of the first row must hold. Empty: only the query succeeding is checked. |
| `sslmode` | Encryption | `prefer` | `prefer` encrypts when the server offers it, `require` refuses to connect without it, `verify-full` also checks the certificate and the name, `disable` never encrypts. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

## MySQL and MariaDB

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 3306 | TCP | MySQL or MariaDB (default port) |

**MySQL or MariaDB database** — connects, authenticates and runs one query: the
signal for a database that is up but no longer answering. Examples: the database
behind a WordPress, a Nextcloud or a Kimai, an application database, a MariaDB
in Docker.

### Setup

1. In the mysql client, create an account with no privilege (adjust the host part to the DumbMonit machine).

    ```
    CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
    ```

2. In DumbMonit, enter the server, for example db.home.lan:3307 (port 3306 by default), then the account's user and password.

3. Optional: replace the default query SELECT 1 with one that returns a single number, to chart it.

#### Required permissions

- **USAGE (no privilege)**: Connect and run SELECT 1, nothing more

!!! warning
    The check opens a real connection on every poll: on an instance close to its connection limit, give it a longer interval.

#### Why these permissions

The default query is SELECT 1, which reads no table: nothing else has to be granted. Connection time and query time are recorded separately. A host pattern of localhost would only accept connections made on the server itself. A query of your own that reads a table needs SELECT on that table.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `port` | Port | `3306` | Used if the address does not give one. |
| `database` | Database | *(empty)* | Database opened on connection. Empty: none, which is enough for the default query. |
| `query` | Query | `SELECT 1` | Run on every check. The default reads no table, so the account needs no privilege beyond connecting. A query returning one number turns it into a chart. |
| `expect` | Expected value | *(empty)* | Value the first column of the first row must hold. Empty: only the query succeeding is checked. |
| `sslmode` | Encryption | `prefer` | `prefer` encrypts when the server offers it, `require` refuses to connect without it, `verify-full` also checks the certificate and the name, `disable` never encrypts. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons, both engines

| `reason` | What happened |
|---|---|
| `connect` | The port is closed, the server is unreachable, or it refused the connection for a reason other than the account (starting up, too many connections). |
| `auth` | The account was refused: wrong password, no `pg_hba.conf` line, host pattern that does not cover the DumbMonit machine. |
| `query` | Connected and authenticated, but the query failed: missing table, database in read-only, syntax error. |
| `payload` | The query worked but returned something other than the expected value. |
| `timeout` | The connection or the query did not finish in time — the "up but slow" case. |

## MQTT

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 1883 | TCP | MQTT (default port) |
| 8883 | TCP | Only with the Encrypted connection option (MQTT over TLS, default) |

**MQTT broker** — connects to the broker, subscribes to a topic, and can wait
for a retained message. Examples: Mosquitto, the broker behind Home Assistant,
Zigbee2MQTT, ESPHome sensors.

### Setup

1. In DumbMonit, enter the broker, for example broker.home.lan:1884 (port 1883 by default).

2. Tick Encrypted connection for a broker listening on 8883.

3. If the broker requires an account, fill in its user and password.

4. Optional: name a topic, to check that the account may still subscribe to it.

5. Optional: tick Expect a retained message only for a topic that carries one, typically an availability topic.

#### Required permissions

- **Subscribe (read) on the topic you name**: Only if you name a topic

!!! warning
    The check never publishes, and it cannot tell how old a retained message is: MQTT does not date them.

#### Why these permissions

Connecting is already worth monitoring: a broker that refuses your account, or that has stopped answering, is the reason the whole house went quiet. When you name a topic, a broker that refuses the subscription tells you the account lost its access rights. A topic published to now and then looks silent to a client that has just connected, so expect a retained message only for a topic that carries one. A value frozen three weeks ago still counts as present.

### What it cannot detect

That a given sensor stopped emitting. The retained message the check reads is
whatever the broker kept, possibly three weeks old — MQTT does not timestamp it.
For freshness, watch the automation that consumes it, or use a heartbeat.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `tls` | Encrypted connection | `false` | Encrypts the session from the first byte, as brokers do on port 8883. |
| `port` | Port | *(empty)* | Used if the address does not give one. Empty: 1883 in clear text, 8883 encrypted. |
| `topic` | Topic | *(empty)* | Topic the check subscribes to. Empty: it only connects, which already tells you the broker is alive and accepts your account. |
| `expect_message` | Expect a retained message | `false` | A message must arrive on that topic before the timeout. Only retained messages arrive right away: a topic that is merely published to from time to time will look silent. |
| `expect` | Expected content | *(empty)* | Text that message must contain. Filling it in implies expecting a message. |
| `client_id` | Client identifier | *(derived)* | Name announced to the broker. The default derives from the monitor, so two checks never disconnect each other. |
| `server_name` | Server name (SNI) | *(the host)* | Domain name announced to the broker and checked in the certificate. Set it when the address is an IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | An unverifiable chain no longer counts as a failure: the expiry date is still recorded. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons

| `reason` | What happened |
|---|---|
| `connect` | The port is closed or the broker is unreachable. |
| `auth` | The `CONNACK` refused the account, or the broker refused the subscription — an access-control list no longer covers this topic. |
| `protocol` | Something answered but does not speak MQTT 3.1.1, or the broker rejected the client identifier. |
| `payload` | No retained message arrived on the topic, or its content does not contain the expected text. |

## WebSocket

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | WSS (default) |
| 80 | TCP | Only with a ws:// address |

**WebSocket endpoint** — runs the upgrade handshake, and can send a frame and
wait for one: a different failure from a plain HTTP page. Examples: the Home
Assistant API, a live dashboard, a log stream, an endpoint behind a reverse
proxy.

### Setup

1. In DumbMonit, paste the full endpoint, for example wss://home.example.com/api/websocket (wss is assumed without a scheme).

2. Optional: fill in Expected content with a fragment of the first frame the server sends by itself.

3. Optional: if the endpoint says nothing until spoken to, put a message in Frame to send, and what the answer must contain in Expected content.

!!! warning
    The check hangs up within a second: a connection that a firewall or proxy cuts after thirty seconds looks healthy here.

#### Good to know

The check runs the whole opening handshake and verifies the answer the server computes from the key it was sent: a reverse proxy that has lost the Upgrade header is caught here, where an HTTP check would still see a healthy 200. A status other than 101 is reported with its code, so a 401 from an expired token is not mistaken for an outage. For the first frame, think of the greeting of the Home Assistant API.

Credentials: none, a username and password (sent as HTTP basic authentication in
the opening request) or a token (sent as `Authorization: Bearer`).

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `path` | Path | *(from the address)* | Path of the opening request. Taken from the address when it carries one. |
| `port` | Port | *(empty)* | Used if the address does not give one. Empty: 443 for wss, 80 for ws. |
| `send` | Frame to send | *(empty)* | Text frame sent once the connection is open. Empty: nothing is sent. |
| `expect` | Expected content | *(empty)* | Text a received frame must contain. Empty and with nothing to send, only the opening handshake is checked. |
| `subprotocol` | Subprotocol | *(empty)* | Value of Sec-WebSocket-Protocol. The check fails if the server picks a different one. |
| `origin` | Origin | *(empty)* | Value of the Origin header, which some servers require before upgrading. |
| `server_name` | Server name (SNI) | *(the host)* | Domain name announced to the server and checked in the certificate. Set it when the address is an IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | An unverifiable chain no longer counts as a failure: the expiry date is still recorded. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons

| `reason` | What happened |
|---|---|
| `connect` | The port is closed or the host is unreachable. |
| `status` | The endpoint answered something other than `101`; `probe_http_status_code` carries the code. |
| `protocol` | It answered `101` but its `Sec-WebSocket-Accept` does not match the key sent, it picked another subprotocol, or it closed the connection at once. |
| `payload` | No frame arrived, or it does not contain the expected text. |

## NTP

**NTP time server** — sends a single SNTP request and reads the reply:
offset, round-trip delay, stratum and leap indicator. Examples: a router or
NAS acting as a local time server, an internal NTP server (chrony, ntpd), a
public pool server used as a reference.

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 123 | UDP | NTP request (default port) |

### Setup

1. In the address, write the server's name or IP address: "pool.ntp.org" or "192.168.1.1".

2. Lower "Tolerated offset" in the options to be warned earlier than 100 ms.

!!! warning
    Keep the DumbMonit host itself in sync: its own drift is added to the measured offset.

UDP port 123 is used unless the "Port" option says otherwise. The check sends a single SNTP request and records the offset, the round-trip delay, the stratum and the leap indicator. A server at stratum 16, or whose leap indicator reads "unsynchronized", is reported down whatever the offset.

Credentials: none.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `port` | Port | `123` | UDP port of the NTP server. |
| `offset_threshold_ms` | Tolerated offset (ms) | `100` | Above this clock offset, the server is reported down. An unsynchronized server (stratum 16) always fails, whatever this value. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons

| `reason` | What happened |
|---|---|
| `connect` | The UDP socket could not be opened, or the reply came from an unexpected address. |
| `timeout` | No reply arrived within the timeout. |
| `protocol` | The reply was not a valid NTP packet, or did not come from a server (wrong mode). |
| `unsynchronized` | The server answered stratum 16 or 0 (a "kiss-o'-death", which also names the reason in the log), or its leap indicator reads "unsynchronized". |
| `clock_offset` | The measured offset exceeds "Tolerated offset". |

## SNMP OID {#snmp-oid}

**SNMP value (single OID)** — reads one SNMP object and checks its value
against a range or a text. Examples: a temperature or humidity sensor, a UPS
status or battery level, a version string or a firmware flag, a counter
exposed by a Net-SNMP extend script.

Where the SNMP device type describes a whole machine with a profile, this one
asks a narrow question: does this object hold what I expect?

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 161 | UDP | SNMP request (default port) |

### Setup

1. On the device, enable SNMP and check that the community or the v3 user may read the object (the SNMP view must grant its branch).

2. In DumbMonit, enter the device address, then the community or the v3 user.

3. Fill in the object (OID) with its instance suffix, for example 1.3.6.1.2.1.1.3.0.

4. Optional: set a minimum and a maximum, or an expected text, to be warned when the value drifts.

#### Required permissions

- **Read access to the object**: A read-only community or v3 user is enough

!!! warning
    An object the agent does not know, or that its SNMP view hides, is reported down with the reason not_found.

#### Good to know

The check reuses the SNMP session of the SNMP device type: same communities (v1/v2c) and v3 users, same retries. Without any bound or expected text, it succeeds as soon as the object exists. A numeric value is charted in `probe_snmp_value` (a text that holds a number, such as `42,5`, counts too). The bounds are inclusive; a value that is not a number fails when a bound is set. The expected text is searched case-insensitively in the value read as text.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `oid` | Object (OID) | *(empty)* | The SNMP object to read, in numeric form. Add the instance suffix, usually .0 for a single value. |
| `port` | Port | `161` | Used if the address does not give one. |
| `min` | Minimum value | *(empty)* | A numeric value below this fails. Empty: no lower bound. |
| `max` | Maximum value | *(empty)* | A numeric value above this fails. Empty: no upper bound. |
| `expect` | Expected text | *(empty)* | Text the value must contain, whatever its case. Empty: any value is accepted. |
| `snmp_version` | SNMP version | `2c` | Version used with a community. Pick 1 only for an old agent that ignores version 2c; SNMP v3 users ignore it. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons

| `reason` | What happened |
|---|---|
| `connect` | The device is unreachable. |
| `timeout` | The agent did not answer in time: wrong address, wrong community (agents stay silent), or a firewall. |
| `auth` | The agent rejected the community or the v3 credentials. |
| `protocol` | The agent answered something that is not a valid SNMP reply, or refused the request. |
| `not_found` | The agent does not know the object (`noSuchObject`, `noSuchInstance`): check the OID, its instance suffix and the SNMP view. |
| `threshold` | The numeric value is outside `min` / `max`. |
| `payload` | The value is not a number although a bound is set, or does not contain the expected text. |

## RADIUS

**RADIUS server** — authenticates a test account (Access-Request) and expects
an Access-Accept: it catches a server that is up but no longer accepts logins.
Examples: FreeRADIUS, Windows NPS, the RADIUS behind a WPA-Enterprise Wi-Fi,
behind a VPN or behind a 802.1X switch.

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 1812 | UDP | Authentication requests (default port) |

### Setup

1. On the RADIUS server, declare the DumbMonit host as a client, with a shared secret (clients.conf on FreeRADIUS, RADIUS Clients on Windows NPS).

2. Create a test account without a second factor, and note its name and password.

3. In DumbMonit, enter the server (UDP port 1812 by default), the shared secret and the test account.

4. Optional: change the NAS identifier if the server filters requests by it.

#### Required permissions

- **A test account that may authenticate**: No network access or admin right is needed

!!! warning
    A wrong shared secret, or a host not declared as a client, usually makes the server stay silent: the check then reports a timeout.

#### Good to know

The check sends one Access-Request per run, with the account's name, its password hidden with the shared secret (RFC 2865) and a signature (`Message-Authenticator`, RFC 3579, placed first as the BlastRADIUS mitigation asks). It never retries: a lost datagram shows as an outage.

The shared secret and the password are stored encrypted and never come back through the API. Leaving them blank when you edit the monitor keeps the saved ones; changing the account name asks for its password again.

The reply is verified with the shared secret. A server answering with a signature that does not match is reported as `bad_secret` rather than as an outage, so that the alert says what to fix. A server that does not know the secret usually does not answer at all: see the failure reasons.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `port` | Port | `1812` | UDP port of the authentication service. |
| `nas_identifier` | NAS identifier | `dumbmonit` | Value of the NAS-Identifier attribute sent with the request. Change it if the server filters requests by NAS. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons

| `reason` | What happened |
|---|---|
| `connect` | The UDP socket could not be opened. |
| `timeout` | No valid reply in time: the server is down, the port is filtered, the shared secret is wrong or this host is not declared as a client (servers drop such requests silently). |
| `auth` | Access-Reject: the server works and refuses the test account (locked, expired, wrong password). Its Reply-Message, if any, is in the log. |
| `bad_secret` | The reply's signature does not match: the shared secret is probably wrong. |
| `protocol` | Access-Challenge (the account asks for a second step), another unexpected code, or a reply that is not valid RADIUS. |

## gRPC

**gRPC service** — calls the standard gRPC health check
(`grpc.health.v1.Health/Check`): it catches a service that is up but reports
itself unhealthy. Examples: a gRPC microservice, etcd, a service behind Envoy
or a service mesh, any server that registers `grpc.health.v1`.

An HTTP check cannot do this: gRPC needs HTTP/2, often without TLS (h2c), and
the answer is a protobuf message plus a `grpc-status` in the trailers.

### Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 50051 | TCP | gRPC over HTTP/2 (default port) |
| 443 | TCP | Only with the Encrypted connection option (default port) |

### Setup

1. Check that the server registers the standard health service (grpc.health.v1.Health): most gRPC frameworks have a one-line helper for it.

2. In DumbMonit, enter the server with its port, for example grpc.home.lan:50051.

3. Tick Encrypted connection (TLS) for a server that requires it; the port then defaults to 443.

4. Optional: name a Service, to check its own health instead of the whole server's.

!!! warning
    The check only sees what the health service reports: a server that always answers SERVING proves nothing about the methods behind it.

#### Good to know

Without TLS the check speaks HTTP/2 in clear text from the first byte ("prior knowledge"), which is what gRPC servers inside a network expect. With TLS it offers ALPN `h2` and fails with `protocol` if the server does not accept it, and it reads the certificate like the other encrypted checks (expiry, issuer, "Accept an unverifiable certificate").

The reply is judged in two steps. The `grpc-status` first, read from the trailers or, for a "Trailers-Only" reply, from the headers: anything but `OK` fails. Then the `status` field of the health message: only `SERVING` succeeds.

Credentials: none.

### Options

| Key | Label | Default | Help |
|---|---|---|---|
| `service` | Service | *(empty)* | Name of the service whose health is asked, for example my.package.MyService. Empty: the health of the server as a whole. |
| `tls` | Encrypted connection (TLS) | `false` | Connect with TLS and HTTP/2 (ALPN h2). Unticked, the check speaks HTTP/2 in clear text (h2c), as most services inside a network do. |
| `port` | Port | *(empty)* | Used if the address does not give one. Empty: 50051, or 443 with TLS. |
| `server_name` | Server name (SNI) | *(the host)* | Domain name announced to the server and checked in the certificate. Set it when the address is an IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | An unverifiable chain no longer counts as a failure: the expiry date is still recorded. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | See "Addresses the checks refuse" below. |
| `timeout_seconds` | Timeout (seconds) | `5` | Time after which the service is reported down if it has not answered. Between 1 and 60. |

### Failure reasons

| `reason` | What happened |
|---|---|
| `dns`, `connect`, `timeout`, `tls`, `cert_expired` | As for the other checks: name, port, delay, TLS handshake, certificate. |
| `unhealthy` | The server answers but reports `NOT_SERVING`, `UNKNOWN` or `SERVICE_UNKNOWN`, or replies `UNAVAILABLE`. `probe_grpc_serving_status` carries the value. |
| `protocol` | Not HTTP/2, not gRPC, or a `grpc-status` other than the ones below. `UNIMPLEMENTED` means the server does not expose the standard health service; `NOT_FOUND`, that it does not know the name given in `service`. |
| `status` | The endpoint answered an HTTP status other than 200 (a proxy, another service); `probe_http_status_code` carries it. |
| `auth` | `UNAUTHENTICATED` or `PERMISSION_DENIED`: the server, or a proxy in front of it, wants credentials the health protocol does not send. |

## Addresses the checks refuse

The checks that connect to an address you give them — HTTP, TCP, TLS, NTP, SMTP,
PostgreSQL, MySQL, MQTT, WebSocket — report what they see — which is also, word for word, what a server-side request
forgery does. To keep a monitoring admin from reading services that only the
DumbMonit host can reach (the embedded metrics database on `127.0.0.1:8428`,
a cloud provider's metadata service on `169.254.169.254`), the checks refuse
by default:

- loopback addresses (`127.0.0.0/8`, `::1`);
- link-local addresses (`169.254.0.0/16`, `fe80::/10`), including the cloud
  metadata address, and the unspecified address (`0.0.0.0`, `::`).

Private LAN ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`)
are always allowed: monitoring them is the point of a homelab.

The rule applies to the address you typed, to every address a host name
resolves to (checked again at connection time, so a name that changes its
answer cannot slip through), and to every redirect the HTTP check follows. A
refused address shows as a configuration error on the target — no down alert is
sent — naming the option to enable: `allow_private_targets`. Turn it on for a
target that really monitors a service on the DumbMonit host itself.

A redirect to a refused address is reported as a failed measurement
(`reason="connect"`), because it is the monitored service that changed, not the
configuration.

Values found at a `json_path` are never echoed beyond 32 characters in a
failure message.

## Common errors

| Symptom | Likely cause |
|---|---|
| Ping shows a configuration error | The `net.ipv4.ping_group_range` sysctl is missing from the Compose file. See above. |
| HTTP down with `reason="keyword"` | The keyword is searched in the first `max_body_bytes` only, case-insensitively unless `keyword_case_sensitive` is set. |
| HTTP down with `reason="tls"` | Self-signed or private authority: set `insecure_tls`. |
| A check shows a configuration error naming `allow_private_targets` | The address is loopback or link-local. See "Addresses the checks refuse" above. |
| TCP down although `telnet host port` works from your laptop | The DumbMonit container cannot reach the host (Docker network, firewall). Test from inside the container's network. |
| DNS down with `reason="record"` | The answer does not contain every value listed in `expect`. |
| Ping down with `reason="packet_loss"` | Loss above `loss_threshold_percent`; some devices rate-limit ICMP. |
| DNS down with `reason="record"` although `expect` matches | `expect_mode` is `exact` and the answer holds an extra value, or `forbid` matched. |
| SMTP down with `reason="protocol"` on `security = starttls` | The relay does not advertise STARTTLS on this port. Use `tls` for 465, `none` for a plain local relay. |
| A database check down with `reason="auth"` | The account exists but may not connect from the DumbMonit machine: check `pg_hba.conf`, or the host part of the MySQL user. |
| A database check down with `reason="query"` | Connection and login worked. The database is up and refusing to work: read-only, missing table, recovery. |
| MQTT down with `reason="payload"` on a topic that clearly has traffic | Only *retained* messages reach a client that has just subscribed. Publish that topic with the retain flag, or untick "Expect a retained message". |
| WebSocket down with `reason="protocol"` after a `101` | Something on the path answers the upgrade without speaking WebSocket — usually a reverse proxy rule that swallows the `Upgrade` header. |
