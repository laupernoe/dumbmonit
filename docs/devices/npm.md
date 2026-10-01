# Nginx Proxy Manager

The proxy manager: hosts whose configuration nginx refused, disabled hosts,
and the certificates the hosts present, before a Let's Encrypt renewal failure
turns into an expired site.

DumbMonit logs in to Nginx Proxy Manager's REST API (`/api`, on the port of
its web interface, 81) with a user of its own, then reads the proxy hosts,
redirection hosts, 404 hosts (called dead hosts in the API) and streams. Each
host says whether it is enabled and whether nginx accepted its configuration.
A host nginx refused is not served, and the web interface only shows it as a
red dot.

Certificates are not read from the API. `/api/nginx/certificates` hands the
private key of imported certificates, and the credentials of the DNS
challenge, to any user allowed to view certificates. DumbMonit reads each
certificate where it is served instead: a TLS handshake on port 443 of
Nginx Proxy Manager, with the name of a host that uses it. That also measures
the certificate visitors actually get.

## What it watches

All metrics are prefixed `dumbmonit_npm_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | hosts of each type | `type`: `proxy`, `redirection`, `dead` (404 hosts), `stream` |
| `hosts_offline` | enabled hosts whose configuration nginx refused | `type` |
| `host_online` | per enabled host: 0 when nginx refused its configuration | `type`, `host` |
| `host_error_info` | value 1, for a host nginx refused: the first line of nginx's error | `type`, `host`, `error` |
| `certificates_checked`, `certificates_read` | certificates attached to an enabled host, and those read on the HTTPS port | |
| `cert_expiry_days` | days left before the certificate a host presents expires | `host` |

`host` is the first domain name of the host, or `port 9000` for a stream.
Disabled hosts are counted, not described: disabling a host is deliberate, and
its alert, if any, resolves. Hosts are described one by one up to 500, and at
most 100 certificates are read per check.

The [built-in rules](../alerting/rules.md#reverse-proxies-and-domains) that
apply: Nginx Proxy Manager host offline, Proxy certificate not renewed, plus
Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries Nginx Proxy Manager. It says what is wrong in a sentence (hosts nginx
refused, with its reason, certificates close to expiry, certificates that could
not be read), then the proxy hosts, redirections, 404 hosts, streams and
disabled hosts, then the certificates soonest to expire first.

## Create a view-only Nginx Proxy Manager user

1. In Nginx Proxy Manager, open Users and add a user for DumbMonit, with an email address of its own such as dumbmonit@example.com. Leave every role unticked.

2. In that user's menu, open Permissions: Item Visibility All Items; Proxy Hosts, Redirection Hosts, 404 Hosts and Streams View Only; Access Lists and SSL Certificates Hidden. Then set its password with Change Password in the same menu.

3. SSL Certificates stay Hidden on purpose: Nginx Proxy Manager hands the private key of imported certificates, and the credentials of the DNS challenge, to any user allowed to view them. DumbMonit reads each certificate where it is served instead, with a TLS handshake on port 443 for every host that has one.

4. In DumbMonit, enter the Nginx Proxy Manager host with the port of its web interface, for example "npm.lan:81", and the email and password of that user. If the hosts are served over HTTPS on another port than 443, set HTTPS port below.

!!! warning
    A host whose configuration nginx refuses (a typo in Advanced, a missing certificate file) is not served, and Nginx Proxy Manager only shows it as a red dot in the list. Let's Encrypt certificates are renewed thirty days ahead: one that expires in less than fourteen days is one whose renewal failed, and DumbMonit warns then.

Item Visibility All Items matters: with Created Items, the user only sees the
hosts it created itself, which is none. A user without Proxy Hosts View Only is
refused; one without Streams, Redirection Hosts or 404 Hosts simply does not
report them.

DumbMonit keeps the token Nginx Proxy Manager gives at login (valid a day) and
logs in again when it expires, rather than at every check.

## Credentials

| Credential | Fields |
|---|---|
| User email and password | The email address and password of the user created above. |

Address: a host name or IP with the port of the web interface (`npm.lan:81`),
or a full URL. Options: protocol (HTTP by default), web interface port (81),
Check the certificates (on), HTTPS port (443), certificate check of the API and
request timeout (10 s).

The certificate check connects to the host part of the address: when the web
interface is reached through another proxy, point the address at Nginx Proxy
Manager itself, or untick Check the certificates and watch each site with a
[TLS certificate check](services.md) instead. Wildcard certificates are read
through the first host that uses them.
