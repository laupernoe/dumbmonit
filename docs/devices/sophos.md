# Sophos Firewall

A Sophos Firewall (SFOS) through its XML API: the link of every interface
bound to a zone, and which IPsec connections are activated.

That is deliberately all. The XML API is a configuration API: besides the
read-only connectivity status of each interface, it exposes no live state.
`<Get><IPSecConnection>` returns whether an administrator activated a
connection, not whether the tunnel is up; high availability, licences and
subscriptions and the firmware version are not in it at all. DumbMonit
publishes the activation as such and never presents it as a tunnel state.

Every minute DumbMonit posts two requests to
`https://<firewall>:4444/webconsole/APIController` (form field `reqxml`, never
in the URL, where the password would end up in logs): `<Get><Interface>` and
`<Get><IPSecConnection>`. Each carries the user name and password of an
account whose device access profile is read-only everywhere: the firewall
enforces that profile on the API too.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from Sophos' XML API documentation (the
    `Interface` and `IPSecConnection` entities, the login and status
    messages), not against a running firewall. The exact wording of the
    interface status (`Connected, 1000 Mbps…` or `Disconnected`) comes from
    that documentation and community reports. Tell us what breaks.

## What it watches

All metrics are prefixed `dumbmonit_sophos_`.

| Metric | Labels | Meaning |
|---|---|---|
| `api_version_info` | `api_version` | Value 1: the API version the firewall announces (`2000.2`), which follows the SFOS release. |
| `interface_link_up` | `interface`, `zone` | 1 when the status reads Connected, 0 when Disconnected; absent when the status is something else. |
| `interface_enabled` | `interface`, `zone` | 0 when the interface is switched off. |
| `interface_down` | `interface`, `zone` | 1 when the interface is switched on, bound to a zone, and has no link. Free ports (zone None) are left alone. |
| `interfaces`, `interfaces_down` | | Counts. |
| `ipsec_connection_activated` | `connection` | 1 when the connection is activated in the configuration. Not the tunnel state. |
| `ipsec_connections` | | Count. |

The [built-in rule](../alerting/rules.md#sophos-firewall) that applies:

- **Sophos interface without link**: an interface switched on and bound to a
  zone has had no link for five minutes (Advisory).

For traffic counters, add the firewall as an [SNMP device](snmp.md) as well:
SFOS answers the standard interface MIB.

## The device page

The Firewall panel says whether every interface in a zone has its link, then
lists those interfaces and the IPsec connections with their configured
activation, labelled as such.

## Allow DumbMonit to read the Sophos Firewall XML API

1. Under Profiles > Device access, add a profile named DumbMonit read-only and set every permission to Read-only.

2. Under Authentication > Users, add a user named dumbmonit, of type Administrator, with that profile and a long random password.

3. Under Backup & firmware > API (Administration > API access from SFOS 22), turn the API configuration on and add the address of the DumbMonit server to the allowed IP addresses.

4. In DumbMonit, enter the address of the firewall, for example "sophos.lan", with the dumbmonit user name and password. The API listens on the administration port, 4444 by default, with a self-signed certificate: tick Accept an unverifiable certificate unless you installed your own.

!!! warning
    The XML API is a configuration API: it reports the link of each interface and whether each IPsec connection is activated, not whether the tunnel is up. High availability, licences and the firmware version are not in it.

## Credentials

| Credential | Fields |
|---|---|
| User name and password | The `dumbmonit` user with the read-only device access profile. |

Address: the host name or IP of the firewall, `host:port`, or a URL. Options:

- **Port** (4444): the administration port.
- **Accept an unverifiable certificate**: the firewall's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"Sophos Firewall refused the request (534 …)": the DumbMonit address is not
among the allowed IP addresses of the API configuration.

"Sophos Firewall refused the login": wrong user name or password, or a
password with characters the firewall rejects in API calls; a long password
of letters and digits avoids the question.

"Sophos Firewall did not answer in XML": wrong port, or the API is turned off.
