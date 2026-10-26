# OVHcloud

The VPS and dedicated servers of an OVHcloud account through the OVHcloud
API: servers that are stopped or in error, and the tasks (reboot, reinstall,
maintenance) running on them.

DumbMonit reads `GET /1.0/vps` and `GET /1.0/dedicated/server` on
`eu.api.ovh.com`, then the state and the pending tasks of each server, once a
minute. The credential is an OVHcloud API token limited to two read rights.
Every request is signed (`X-Ovh-Signature`, a SHA-1 of the secret, the
consumer key, the method, the URL and the server time read from
`/auth/time`), so the clock of the DumbMonit host does not matter.

This is an Internet API: the address of the device is the API's, not a
machine's. DumbMonit needs outbound HTTPS to `eu.api.ovh.com` (or
`ca.api.ovh.com` for OVHcloud Canada); no server is contacted.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the OVHcloud API documentation, not
    against a live account. It only reads. Tell us what breaks.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | OVHcloud API (HTTPS, eu.api.ovh.com) |

## What it watches

All metrics are prefixed `dumbmonit_ovh_`. The first 30 servers of each
family are described; the counts stay exact beyond that.

| Metric | Labels | Meaning |
|---|---|---|
| `vps_up` | `vps` | 1 while the VPS is running (or taking a backup). |
| `vps_info` | `vps`, `state`, `zone`, `model` | Value 1. |
| `vps_tasks_active` | `vps` | Tasks pending or running on the VPS. |
| `dedicated_up` | `server` | 1 while OVHcloud monitoring says the server is `ok` and it is powered on. |
| `dedicated_monitoring` | `server` | 1 when OVHcloud monitoring is enabled on the server. |
| `dedicated_info` | `server`, `state`, `power`, `datacenter` | Value 1. |
| `dedicated_tasks_active` | `server` | Tasks pending or running on the server. |
| `servers`, `servers_down`, `tasks_active` | | Counts. |
| `vps_readable`, `dedicated_readable`, `tasks_readable` | | 0 when the API token has no right to read that part: its series are left out and the probe still succeeds. |

The [built-in rules](../alerting/rules.md#ovhcloud) that apply:

- **OVHcloud VPS not running**: a VPS has not been in the running state for
  10 minutes (Advisory).
- **OVHcloud dedicated server in error**: OVHcloud monitoring reports a
  dedicated server in error, or powered off, for 5 minutes (Critical).
- **OVHcloud task in progress**: tasks have been pending or running for an
  hour (Info).

OVHcloud publishes no calendar of planned maintenance through its public API:
DumbMonit sees an intervention only once it has started, as a task. A VPS you
stopped yourself counts as not running, and a dedicated server is reported in
error only if OVHcloud monitoring is enabled on it.

## The monitor page

The panel says what is wrong in a sentence, shows the number of servers, the
servers down and the tasks in progress, then lists the VPS and the dedicated
servers with their state.

## Create a read-only API token for DumbMonit

1. Open the OVHcloud token page of your region and sign in with your OVHcloud account. The link pre-selects the two read rights.
   `https://eu.api.ovh.com/createToken/?GET=/vps*&GET=/dedicated/server*`

2. Name the script DumbMonit, set Validity to Unlimited, and keep only the two GET rights listed below, then click Create keys.

3. Copy the Application Key, the Application Secret and the Consumer Key: they are shown once.

4. In DumbMonit, enter eu.api.ovh.com as address (ca.api.ovh.com for OVHcloud Canada) and paste the Application Key as Application key.

5. In the second field, type the Application Secret, a colon, then the Consumer Key.
   `application_secret:consumer_key`

### Required permissions

- **GET /vps\*** — Lists the VPS and reads their state and tasks; changes nothing.
- **GET /dedicated/server\*** — Lists the dedicated servers and reads their state and tasks; changes nothing.

!!! warning
    The OVHcloud API publishes no calendar of planned maintenance: DumbMonit sees an intervention only once it has started, as a running task. A VPS you stopped yourself counts as down. A dedicated server is reported in error only if OVHcloud monitoring is enabled on it. Either right may be omitted to watch only the other family.

## Credentials

| Credential | Fields |
|---|---|
| OVHcloud API keys | **Application key**, and in the password field the application secret and the consumer key joined by a colon: `application_secret:consumer_key`. |

Address: `eu.api.ovh.com`. Options:

- **Timeout per request** (15 s).

## Troubleshooting

"OVHcloud refused the signature (400)": the application key, the secret or
the consumer key is wrong, or the password field is not in the
`secret:consumer` order.

"OVHcloud accepted the credentials but refused ... (403)": the consumer key was revoked or
expired. Create a new token.

"OVHcloud granted this consumer key neither GET /vps* nor
GET /dedicated/server*": create the token again with at least one of the two
rights.

A family missing from the page with its `readable` metric at 0: the token
has no right on it, or the account has none of that product.
