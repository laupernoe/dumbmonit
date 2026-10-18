# IP range blocklist

Every address of a public IPv4 range, up to a /24 (256 addresses), checked
against the DNS blocklists of [Mail server reputation](mail_blacklist.md#the-blocklists):
Spamhaus ZEN, Barracuda, SpamCop, Mailspike, UCEPROTECT Level 1, PSBL and
SpamRATS. The monitor page says how many addresses are listed, on which
lists, and gives a direct link to the lookup and removal page of each
listing.

Useful for the public range of an office, the block a hosting provider gave
you, or the addresses your servers send from: a neighbour's compromised
machine gets a listing that a stricter receiver may extend to the whole
range.

## Limits

- **At most a /24 per monitor.** A larger range is refused with a message
  saying how many addresses it holds: split it into several /24 monitors.
  Each address costs one query to every list, and a sweep of thousands of
  addresses looks like abuse to them.
- **IPv4 only.** Of these lists, only Spamhaus ZEN answers for IPv6, and an
  IPv6 range cannot be walked address by address. A single IPv6 address can
  be checked with [Mail server reputation](mail_blacklist.md).
- **Public addresses only.** A range that includes a private, shared (CGNAT)
  or documentation address is refused: blocklists only know the public
  Internet.
- **The domain lists** (Spamhaus DBL, SURBL, URIBL) do not apply to a range.

## Resolvers and pace

The same rules as for [Mail server reputation](mail_blacklist.md#resolvers-refusals-and-timeouts)
apply. Spamhaus refuses public resolvers: its refusal (`127.255.255.254`) is
shown as a configuration error, never as a listing. A list that does not
answer is unknown, never clean. Use a recursive resolver of your own, or a
free Spamhaus DQS key as the credential.

A /24 takes close to two thousand queries. They are sent at **Queries per
second** (10 by default), eight at most in flight, and the whole range is
checked once per **Refresh interval** (6 hours by default); the last result is
shown again at every check in between. The first check of a range takes a few
minutes and runs in the background.

## Check a public IP range against the blocklists

1. Enter an IPv4 address or a CIDR range up to /24, for example "198.51.100.0/24" (256 addresses). A larger range is refused: split it into several /24 monitors. IPv6 ranges are refused, since only Spamhaus answers for IPv6: check a single IPv6 address with Mail server reputation.

2. Use a recursive resolver of your own, as for Mail server reputation: Spamhaus refuses public resolvers, or paste a free Spamhaus DQS key as the credential. A /24 takes close to two thousand queries, sent at 10 per second and repeated every 6 hours by default: keep that pace, blocklists ban resolvers that sweep them.

3. The first check of a range takes a few minutes: the monitor page says it is running, then shows how many addresses are listed, on which lists, and a delisting link for each.

!!! warning
    Spamhaus PBL lists end-user ranges that should not send mail directly (home connections, dynamic pools): a listing there is a policy, not an abuse report, and only matters for an address that runs a mail server.

## What it watches

The same metrics as [Mail server reputation](mail_blacklist.md#what-it-watches),
prefixed `dumbmonit_blacklist_`, without the reverse DNS, SPF and DMARC ones.
To keep the number of series small, `check` is only written for an address
that is listed or that a list did not answer; `list_checked`, `list_listed`
and `list_unknown` count every address per list.

The [built-in rules](../alerting/rules.md#dns-blocklists) that apply: Listed
on a major blocklist and Listed on a blocklist.

## Credentials

None, or a Spamhaus DQS key.

Address: an IPv4 address or a CIDR range up to /24. Options: Resolver,
Lists (all the address lists by default), Refresh interval (6 hours),
Queries per second (10) and Timeout per query (5 s).
