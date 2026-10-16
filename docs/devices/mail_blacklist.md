# Mail server reputation

A mail domain, or the public address of a mail server, checked against the
DNS blocklists that still decide whether mail is delivered: Spamhaus,
Barracuda, SpamCop and a few smaller ones. For every list that lists an
address, the monitor page gives the list's answer code, its published reason
and a direct link to its lookup and removal page.

Enter a domain (`example.com`) and DumbMonit checks the addresses of its MX
hosts (A and AAAA records), the addresses of the domain itself, and any
**Extra addresses** you add (an outgoing relay that is not an MX). The domain
is also checked against the domain blocklists. Enter an IP address and only
that address is checked. To watch a whole range of addresses, use
[IP range blocklist](ip_blacklist.md).

## Gmail, Outlook.com and Yahoo

The large mailbox providers publish **no blocklist that can be queried**: what
they think of a sender is internal to them. DumbMonit does not pretend to ask
them. It checks what weighs on their decision, and links to their tools:

- **Reverse DNS**: every sending address should have a PTR record whose name
  resolves back to the same address (forward-confirmed reverse DNS, FCrDNS).
  Gmail, Outlook.com and Yahoo refuse or junk mail from addresses without it.
- **SPF and DMARC**: exactly one SPF record on the domain, and a DMARC record
  at `_dmarc.<domain>`. Gmail and Yahoo require both from bulk senders.
- **The blocklists below**, which their filters also read.

DKIM is not checked: its selector is chosen by each sender and cannot be
guessed. The monitor page links to:

| Provider | Tools |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/) (reputation of a verified domain), [sender guidelines](https://support.google.com/a/answer/81126), [delivery problem form](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/) (reputation data per address), [sender policies](https://sendersupport.olc.protection.outlook.com/pm/), [sender support form](https://olcsupport.office.com/) (delisting from Outlook.com), [Microsoft 365 delisting portal](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/), [best practices](https://senders.yahooinc.com/best-practices/), [sender support form](https://senders.yahooinc.com/contact/) |

## The blocklists

Checked on 16 October 2026. SORBS closed in June 2024 and is not queried.
Each list is asked an `A` record under its zone; a list is identified by the
id in the first column, which the **Lists** option takes to restrict the
check.

| Id | List | Zone | Checks | IPv6 | Major | Use and removal |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | addresses | yes | yes | Free public mirrors for low-volume, non-commercial use; refuses public and open resolvers. Removal from the [Spamhaus checker](https://check.spamhaus.org/). PBL lists end-user ranges by policy: remove an address from it only if it really runs a mail server. |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | addresses | no | yes | Free; Barracuda asks that the DNS servers querying it be registered with a free account at barracudacentral.org. [Removal request](https://www.barracudacentral.org/rbl/removal-request). |
| `spamcop` | SpamCop | `bl.spamcop.net` | addresses | no | yes | Free. Listings expire on their own about a day after the reports stop; the [lookup page](https://www.spamcop.net/bl.shtml) shows why and offers delisting. |
| `mailspike` | Mailspike | `bl.mailspike.net` | addresses | no | no | Free at low volume. [Lookup and delisting](https://mailspike.io/ip_verify). |
| `uceprotect_1` | UCEPROTECT Level 1 | `dnsbl-1.uceprotect.net` | addresses | no | no | Free to query. Listings expire after seven days without new spam; the [check page](https://www.uceprotect.net/en/rblcheck.php) also sells express delisting, which is never required. Few receivers use it. |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | addresses | no | no | Free. Self-service removal on the [listing page](https://psbl.org/). |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | addresses | no | no | Free at low volume. RATS-NoPtr lists addresses without reverse DNS. [Lookup](https://www.spamrats.com/lookup.php) and [removal](https://www.spamrats.com/removal.php). |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | the domain | — | yes | Same terms as ZEN. Removal from the [Spamhaus checker](https://check.spamhaus.org/). |
| `surbl` | SURBL (multi) | `multi.surbl.org` | the domain | — | no | Free for low volume; refuses public and busy resolvers. [Lookup and removal](https://www.surbl.org/surbl-analysis). |
| `uribl` | URIBL (multi) | `multi.uribl.com` | the domain | — | no | Free for low volume; refuses public and busy resolvers. [Lookup](https://lookup.uribl.com/), [removal](https://admin.uribl.com/). |

A listing on a **major** list fires a critical alert; one on the other lists
an advisory one. IPv6 addresses of the MX are checked against Spamhaus ZEN
only, the one list here that answers for IPv6; the others show them as not
covered.

## Resolvers, refusals and timeouts

The lists are queried through DNS, and how they answer depends on **which
resolver asks**:

- **Spamhaus refuses queries that come through public resolvers** (Google
  `8.8.8.8`, Cloudflare `1.1.1.1`, Quad9), through resolvers that forward to
  them, or through resolvers without an attributable reverse DNS. It then
  answers `127.255.255.254` instead of `NXDOMAIN` (`127.255.255.255` when a
  resolver sends too many queries, `127.255.255.252` for a malformed query or
  a wrong DQS key). These look like listings and are not: DumbMonit shows the
  check as a **configuration error**, never as "listed", and sends no outage
  alert.
- **URIBL and SURBL answer `127.0.0.1`** to resolvers they refuse. That list
  is then shown as "Refused" (unknown), not as a listing.
- **A list that does not answer in time is unknown, never clean.** Only
  `NXDOMAIN` means "not listed". An answer outside `127.0.0.0/8` (a resolver
  that rewrites `NXDOMAIN` into the address of a search page) is unknown too.

So use **a recursive resolver of your own**: Unbound, Knot Resolver, BIND or
PowerDNS Recursor resolving from the root servers (not forwarding to a public
resolver), and set its address under **Resolver** if it is not the system
resolver. Pi-hole or AdGuard Home in front of Unbound works; in front of
`1.1.1.1` it does not.

Or create a free [Spamhaus Data Query Service](https://www.spamhaus.com/data-access/free-data-query-service/)
account (non-commercial use, low volume) and enter its key as the
credential: Spamhaus is then asked under `<key>.zen.dq.spamhaus.net` and
`<key>.dbl.dq.spamhaus.net`, which answer through any resolver. The key is
stored encrypted and never shown again.

## Pace

Blocklists limit how often a resolver may ask, and ban resolvers that sweep
them. DumbMonit asks every list once per **Refresh interval** (6 hours by
default), at most **Queries per second** (10 by default) with eight queries in
flight, and shows the last result again at every check in between, so the
alert rules see a continuous series. A check runs in the background: the
first result appears a few seconds after the monitor is added, and changing
the address, an option or the key starts a new check at once. A check whose
result is an error is retried after ten minutes.

## Check a mail server against the blocklists

1. Enter the domain you send mail for, for example "example.com": DumbMonit checks the addresses of its MX and of the domain itself. Or enter the public IP address the server sends from. If mail leaves through a relay that is not an MX, add its address under Extra addresses.

2. Use a recursive resolver of your own. Spamhaus refuses queries that arrive through public resolvers (Google, Cloudflare, Quad9) or through a resolver that forwards to them, and other lists limit them: an Unbound that resolves from the root servers is enough. Set its address under Resolver if it is not the system resolver.

3. Or create a free Spamhaus Data Query Service account (non-commercial use, low volume) and paste its key as the credential: Spamhaus then answers through any resolver.

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

4. To check the setup, add a monitor for 127.0.0.2, the test address every blocklist lists: each one should show it as listed.

5. Gmail, Outlook.com and Yahoo publish no blocklist that can be queried. DumbMonit checks what they weigh instead (a reverse DNS that points back to the address, SPF and DMARC records) and links to their sender tools on the monitor page.

!!! warning
    An answer Spamhaus refused (127.255.255.254, a public resolver) is shown as a configuration error, never as a listing, and a blocklist that does not answer is shown as unknown, never as clean. The lists are asked every 6 hours by default: asking more often gains nothing and gets resolvers banned.

## What it watches

All metrics are prefixed `dumbmonit_blacklist_`.

| Metric | What | Labels |
|---|---|---|
| `listed_major_lists` | major lists (Spamhaus ZEN and DBL, Barracuda, SpamCop) that list an address or the domain | |
| `listed_lists` | all lists that list an address or the domain | |
| `addresses_checked` | addresses checked | |
| `addresses_listed` | addresses listed on at least one list | |
| `domain_listed` | 1 when the domain is on a domain blocklist | |
| `unknown_checks` | checks without an answer (timeout, refusal) | |
| `check` | one series per address (or domain) and list: 1 listed, 0 clean, −1 no answer, −2 refused, −3 not covered | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | per list: subjects asked, listed, without answer | `list` |
| `address_info` | value 1, where each address comes from | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 1 when the reverse DNS resolves back to the address | `address`, `ptr` |
| `mx_hosts` | MX hosts of the domain | |
| `spf_records` | `v=spf1` records on the domain (1 is right) | |
| `dmarc_records` | `v=DMARC1` records at `_dmarc.<domain>` | |
| `dmarc_policy_info` | value 1 | `policy` |
| `mail_auth_missing` | 0 to 2: SPF missing or duplicated, DMARC missing | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | the check shown: its age, how long it took, whether a new one runs | |

The [built-in rules](../alerting/rules.md#dns-blocklists) that apply: Listed
on a major blocklist, Listed on a blocklist, Mail server reverse DNS
mismatch, SPF or DMARC missing, plus Monitor unreachable when the resolver
answers none of the queries.

## The monitor page

The panel reads what the probe stored; opening the page never queries a
blocklist. It shows the counts, then each list with how many subjects it
lists, then every result with the list's code, its published reason and
links to its lookup and removal pages, then the reverse DNS of each address,
SPF and DMARC, and the sender tools of Gmail, Outlook.com and Yahoo.

## Credentials

None, or a Spamhaus DQS key (see above).

Address: a domain (`example.com`) or a public IP address. Options: Resolver,
Lists (all by default), Extra addresses, Refresh interval (6 hours, from 1
to 168), Queries per second (10, from 1 to 50) and Timeout per query (5 s,
from 1 to 30). Private addresses are never checked: blocklists only know the
public Internet. `127.0.0.2` is accepted as the test address.
