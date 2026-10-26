# mailcow (beta)

Whether the mail server is delivering: stopped containers, the size and age of Postfix's queue, the volume holding the mailboxes and mailboxes close to their quota.

!!! note "Beta"
    Written from the vendor's API documentation and tested against realistic recorded-style responses, not yet against a live instance. Tell us what you see.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | API (HTTPS, default) |
| 80 | TCP | Only with the http scheme option |

The failure worth watching: **one stopped container in a stack of twenty**. rspamd down means no spam filtering, Dovecot down means no reading, Postfix down means no delivery, and the web interface still opens. A mailbox at its quota gets its incoming mail refused with nothing shown to the sender's admin.

## What it watches

All metrics are prefixed `dumbmonit_mailcow_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `containers_total`, `containers_running` | mailcow containers, and those running |  |
| `container_up` | 1 per container while running (at most 32) | `container` |
| `vmail_used_percent` | the volume holding the mailboxes |  |
| `queue_messages`, `queue_oldest_seconds` | messages in Postfix's queue and the age of the oldest |  |
| `mailboxes_total`, `mailboxes_inactive`, `mailboxes_near_quota` | mailboxes; deactivated ones; those at 90% of their quota or more |  |
| `mailbox_quota_percent` | mailboxes at 80% or more of their quota, at most 32 | `mailbox` |
| `scrape_errors` | calls that failed on the last probe |  |

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply: mailcow container down, mailcow mail queue stuck, mailcow mail volume almost full, mailcow mailbox near quota, plus Monitor unreachable.

## Create a read-only API key in mailcow

1. In mailcow, open System → Configuration → Access → API and find the Read-only access section.

2. Tick Activate API, add DumbMonit's IP address to the allowed addresses and save.

3. Copy the read-only key, then enter mailcow's address in DumbMonit and paste it.

!!! warning
    mailcow only answers the addresses allowed on its API page: if DumbMonit reaches it through NAT or a relay agent, allow that address instead.

## Credentials

| Credential | Fields |
|---|---|
| API key | Sent in the `X-API-Key` header. |

Address: a host name (`mail.example.com`), `host:port`, or a full URL. Mailbox names are published only for mailboxes close to their quota. Options: protocol (HTTPS by default), port (443), certificate check and request timeout (15 s).
