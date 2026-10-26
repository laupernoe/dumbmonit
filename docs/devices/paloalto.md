# Palo Alto PAN-OS

A Palo Alto Networks firewall (physical or VM-Series) through the PAN-OS XML
API: interfaces bound to a zone that lost their link, an HA pair that lost its
peer or is out of sync, a session table filling up, management CPU and memory,
licences and subscriptions expiring, and a failed configuration commit.

!!! warning "Beta"

    This integration was built from Palo Alto Networks' XML API documentation,
    not against a running firewall. Every call is a read, but the shape of some
    answers (HA state, `show system resources`, `show interface all`) is the
    least documented: tell us what breaks.

DumbMonit uses a role-based account whose role only allows XML API
operational requests. The API key goes in the `X-PAN-KEY` header, never in the
URL (the older `?key=` form would leave it in access logs). Requests are sent
as POST forms. With a user name and password, DumbMonit asks the firewall for a
key (`type=keygen`, credentials in the request body), keeps it for an hour and
asks again when it is refused; with an API key you generated yourself, it is
used as is.

Every minute DumbMonit runs one required operational command, `show system
info`, and six optional ones that a narrower role, or a firewall without a
cluster, may refuse without failing the rest: `show system resources`, `show
session info`, `show high-availability state`, `request license info`, `show
interface all` and `show jobs all`.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Management interface (HTTPS, default) |

## What it watches

All metrics are prefixed `dumbmonit_paloalto_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version`, `model`, `hostname` | Value 1. |
| `uptime_seconds` | | Time since boot. |
| `cpu_usage_percent`, `memory_used_percent` | | Management plane, read from the text output of `show system resources` (CPU: 100 minus idle; memory: without buffers and cache). Left out when the output cannot be read. |
| `sessions`, `sessions_max`, `session_usage_percent` | | Active sessions, capacity of the session table, and the share used. |
| `session_setup_rate`, `packets_per_second`, `throughput_kbps` | | Rates from `show session info`. |
| `ha_enabled` | | 1 when HA is configured. |
| `ha_state_info` | `state`, `peer_state` | Value 1, with the local and peer HA states. |
| `ha_local_functional` | | 0 when the local state is `suspended` or `non-functional`. |
| `ha_peer_up` | | 1 when the peer is active or passive and the HA link to it is up. |
| `ha_in_sync` | | 1 when the running configuration is synchronized. |
| `interface_link_up` | `interface`, `zone` | Physical link of `ethernet` and `ae` interfaces. |
| `interface_down` | `interface`, `zone` | 1 when the interface is bound to a zone and has no link. Ports without a zone are left alone. |
| `interface_speed_mbps` | `interface`, `zone` | While the link is up. |
| `interface_address_info` | `interface`, `description`, `address`, `vlan` | Value 1, the IPv4 network of the interface (used by the topology map). |
| `interfaces_down` | | Count. |
| `license_status_info` | `license`, `status` | Value 1, for every licence listed. |
| `license_expired` | `license` | 1 when expired. |
| `license_expiry_seconds` | `license` | Until expiry; negative once expired. Licences that never expire have none. |
| `last_commit_ok` | | 1 when the latest finished commit succeeded (a commit with warnings counts as succeeded). |

The [built-in rules](../alerting/rules.md#palo-alto-pan-os) that apply:

- **Palo Alto interface without link**: for five minutes (Advisory).
- **Palo Alto HA peer lost**: for two minutes (Warning).
- **Palo Alto HA out of sync**: for fifteen minutes (Advisory).
- **Palo Alto HA member not functional**: for five minutes (Warning).
- **Palo Alto session table filling up**: above 90 % for ten minutes (Advisory).
- **Palo Alto management CPU high** and **memory high**: above 90 % for fifteen
  minutes (Advisory).
- **Palo Alto licence expiring**: less than 30 days left, or expired
  (Advisory, reminded daily).
- **Palo Alto commit failed**: the latest commit failed (Info, reminded daily).

## The monitor page

The Firewall panel says what is wrong in a sentence, shows CPU, memory,
sessions and the HA state, then lists the interfaces without link and the
licences expiring within 60 days or expired.

## Create a read-only XML API account for DumbMonit

1. Device → Admin Roles → Add: name the role as follows. On the XML API tab enable Operational Requests only; leave every other privilege, and the Web UI and Command Line tabs, disabled.

    ```
    dumbmonit-ro
    ```

2. Device → Administrators → Add: name the account as follows, set Authentication to a password, the type to Role Based and the role to the one above.

    ```
    dumbmonit
    ```

3. Click Commit: the role and the account exist only once committed.

4. In DumbMonit, enter the firewall address (pa.lan) with that user name and password, or with an API key you generated for that account.

### Required permissions

- **XML API → Operational Requests: enabled** — show system info, resources, sessions, interfaces, HA state, jobs and licence info
- **Every other XML API privilege: disabled** — Configuration, Commit, Export, Import, Report, Log and User-ID Agent are not needed
- **Web UI and Command Line: disabled** — The account never signs in interactively

!!! warning
    The management interface ships a self-signed certificate: tick "Accept an unverifiable certificate" unless you installed your own.

Vendor documentation: https://docs.paloaltonetworks.com/pan-os/11-1/pan-os-panorama-api/get-started-with-the-pan-os-xml-api

### Notes

- To generate the key yourself instead of letting DumbMonit do it:

    ```
    curl -k -X POST "https://pa.lan/api/?type=keygen" -d "user=dumbmonit" --data-urlencode "password=..."
    ```

  Paste the `<key>` of the answer as API key. A key generated for a role-based
  account carries that role's limits.
- The management interface must allow HTTPS; it does by default. Behind a
  data-plane interface, allow HTTPS in its management profile.
- Only the management plane is measured for CPU and memory. Data-plane load
  is not read.
- Global Protect, IPsec tunnel and URL-filtering figures are not read: they are
  not covered by the commands above.

## Credentials

| Credential | Fields |
|---|---|
| User name and password | The account created above; DumbMonit asks the firewall for an API key with it. |
| API key | A key already generated for that account, sent as `X-PAN-KEY`. |

Address: the host name or IP of the firewall, `host:port`, or a URL. Options:

- **Port** (443): the HTTPS port of the management interface.
- **Accept an unverifiable certificate**: the firewall's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"PAN-OS refused the credentials (403)": wrong or revoked key, wrong password,
or the role does not allow Operational Requests. Commit the role and the
account, then try again.

"PAN-OS refused the request (16: Unauthorized request)": the role lacks the
Operational Requests privilege.

No HA, licence, session or interface figure: the role cannot run that command,
or the firewall has no cluster; the rest is still read.
