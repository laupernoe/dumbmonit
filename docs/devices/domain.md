# Domain expiry

A registered domain, read from its registry over RDAP: the days left before it
expires, a hold that takes it out of the DNS, a redemption period, its
registrar and whether its delegation is signed with DNSSEC.

RDAP is the successor of WHOIS: the same registration data, as JSON, with a
standard format across registries. DumbMonit finds the registry of the
extension in the IANA RDAP bootstrap file
(`https://data.iana.org/rdap/dns.json`, read at most once a day), then asks it
`/domain/example.com`. No credential is needed, and nothing but the
registration of the domain is asked.

Two failures are worth watching besides the expiry date itself. A domain in
`clientHold` or `serverHold` has been taken out of the DNS by the registrar or
the registry, usually over an unpaid invoice or a contact address that was
never verified: every site and mailbox under it stops at once. A domain in
`redemptionPeriod` or `pendingDelete` has already expired and will be
released unless it is restored, at a price.

## What it watches

All metrics are prefixed `dumbmonit_domain_`.

| Metric | What | Labels |
|---|---|---|
| `expiry_days` | days left before expiry, fractional; negative once expired | |
| `expiry_timestamp_seconds` | the expiry date, in Unix seconds | |
| `expiry_published` | 0 when the registry does not publish the expiry date | |
| `on_hold` | 1 in `clientHold` or `serverHold` | |
| `redemption` | 1 in `redemptionPeriod`, `pendingDelete` or `pendingRestore` | |
| `status` | value 1, one series per status code, in EPP form (`clientTransferProhibited`) | `status` |
| `registrar_info` | value 1 | `registrar` |
| `dnssec` | 1 when the delegation is signed | |
| `rdap_ok` | 0 when the last query to the registry failed and an earlier answer is shown | |
| `rdap_age_seconds` | age of the answer shown | |

The registry is asked once per Refresh interval (12 hours by default); in
between, the days left are recomputed at every check. When the registry does
not answer, the last answer is still used for up to seven days, with `rdap_ok`
at 0, so a registry outage does not look like a domain problem.

The [built-in rules](../alerting/rules.md#reverse-proxies-and-domains) that
apply: Domain expiring (under 30 days), Domain about to expire (under 7 days or
expired), Domain on hold, Domain in redemption, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries the registry. It says the expiry date and the days left, the status
codes (or the hold and redemption in a sentence), the registrar and DNSSEC,
and whether the last query to the registry failed.

## Watch a domain's registration

1. Enter the registered domain, for example "example.com", not a name under it such as www.example.com, which the registry does not know. Nothing to set up and no credential: RDAP, the successor of WHOIS, is public.

2. DumbMonit finds the registry of the extension in the IANA RDAP bootstrap file, then asks it for the expiry date, the status codes and the registrar, twice a day by default. Registries limit RDAP queries, and the expiry date changes once a year: a shorter Refresh interval gains nothing.

3. A few registries run RDAP without being listed by the IANA, those of .ch and .li for instance: set RDAP server to their address. An extension whose registry runs no public RDAP service cannot be watched.

    ```
    https://rdap.nic.ch/
    ```

4. Some registries do not publish the expiry date (.ch is one): DumbMonit then says so, and still watches the status codes.

!!! warning
    A domain in clientHold or serverHold is taken out of the DNS: every site and mailbox under it stops, usually over an unpaid invoice or an unverified contact address. DumbMonit warns 30 days before expiry, raises it 7 days before, and alerts at once on a hold or a redemption period.

A domain the registry does not know (a subdomain, a typo, or a domain that was
deleted) is reported as a configuration error, not as an outage. Names with
accents are accepted and asked in their ASCII form (`xn--…`).

## Credentials

None: RDAP is public.

Address: the registered domain (`example.com`); a pasted URL is reduced to its
host name. Options: RDAP server (empty: found through IANA), Refresh interval
(12 hours, from 1 to 168) and request timeout (10 s). The check interval of
the device itself can stay at its default: it only recomputes the days left
between two queries to the registry.
