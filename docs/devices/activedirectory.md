# Active Directory

A Windows domain over LDAP, read-only: the domain controllers and who holds
each FSMO role, the members of the privileged groups, the password policy,
account counts, and the raw security facts an audit looks for first —
accounts open to Kerberoasting or AS-REP roasting, unconstrained delegation,
an old krbtgt password, computers without LAPS, Windows releases out of
support.

DumbMonit binds with a dedicated service account that is a **plain domain
user, member of no group**. In Active Directory every authenticated user can
read what DumbMonit reads; no administrative right is needed, and none should
be given. DumbMonit never writes to the directory, never reads a password
attribute (of LAPS it reads the expiry date, never the password) and never
asks for the replication rights that would expose secrets.

One device covers the whole domain: point it at one domain controller and
DumbMonit finds the others in the configuration partition.

## What LDAP can and cannot tell

LDAP shows the directory as it is, not the logs of what happened in it. In
particular:

- **Replication.** The queried domain controller publishes the state of its
  inbound replication partners (last success, consecutive failures, last
  error) only to an account holding the *Monitor active directory
  replication* right. Without it, the panel says *replication status not
  readable* rather than guessing. Only the queried domain controller's view is
  shown: to see another one's partners, add it as a second device.
- **Not visible over LDAP**: event logs (logons, lockouts as they happen,
  group changes as events), Group Policy contents, SYSVOL, DNS zones, and the
  password hashes themselves.
- **Last logon dates** come from `lastLogonTimestamp`, which domain
  controllers replicate only every 9 to 14 days: an account counted as
  inactive is inactive to within two weeks.
- **Group membership** is read with nesting (the `LDAP_MATCHING_RULE_IN_CHAIN`
  rule), through `member`/`memberOf`. A primary group set by `primaryGroupID`
  is not a `member` link and is not seen.

## How it reads

Two paces:

- **At every check** (each minute by default): connection and bind, the
  RootDSE (functional levels, clock, synchronisation), the domain object
  (password policy, machine account quota), the five FSMO roles, the domain
  controllers from the configuration partition, the privileged groups and
  their effective members, the Recycle Bin feature, and a TCP connection to
  the LDAP port of each other domain controller.
- **In the background** every `inventory_minutes` (15 by default): every user
  and computer account, with only the attributes the findings need, in pages
  of 500. Enumerating the whole directory every minute would load the domain
  controllers for nothing.

The first check waits a few seconds for the inventory; on a large domain the
account findings appear at the next check.

## What it watches

All metrics are prefixed `dumbmonit_ad_`.

| Metric | What | Labels |
|---|---|---|
| `bind_ok` | 1 when the service account is accepted, 0 when the domain controller refuses it | |
| `connect_seconds`, `bind_seconds` | time to open the connection (TLS included) and to bind | |
| `tls_verified` | 1 when the certificate is checked | |
| `domain_info` | value 1 | `domain`, `forest`, `dc` |
| `domain_functional_level`, `forest_functional_level` | 0 (2000) to 7 (2016), 10 (2025) | |
| `dc_synchronized`, `dc_global_catalog_ready` | the queried domain controller has finished its initial replication, its global catalog is ready | |
| `clock_skew_seconds` | domain controller clock minus DumbMonit host clock | |
| `recycle_bin_enabled` | 1 when the AD Recycle Bin is enabled | |
| `dc_count` | domain controllers of the domain | |
| `dc_info` | value 1 | `dc`, `site`, `gc`, `rodc`, `os` |
| `dc_reachable` | 1 or 0 per domain controller; no series when its name does not resolve from DumbMonit | `dc` |
| `fsmo_role` | value 1 | `role` (`schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`), `holder` |
| `replication_readable` | 1 when inbound replication status can be read | |
| `replication_consecutive_failures`, `replication_last_success_age_seconds` | per inbound partner and partition | `source`, `naming_context` |
| `privileged_group_members`, `privileged_group_enabled_members` | effective members, nesting included, groups left out | `group` |
| `privileged_group_fingerprint` | a hash of the members: changes when someone joins or leaves | `group` |
| `users_total`, `users_enabled`, `users_disabled`, `users_locked_out`, `users_password_never_expires`, `users_stale` | user accounts | |
| `computers_total`, `computers_enabled`, `computers_stale`, `groups_total` | computers and groups | |
| `admincount_accounts` | users protected by AdminSDHolder (`adminCount=1`) | |
| `krbtgt_password_age_seconds` | age of the krbtgt password | |
| `laps_schema`, `laps_computers`, `laps_eligible_computers` | LAPS in the schema; enabled non-DC computers with a LAPS expiry, out of all of them | |
| `password_min_length`, `password_history_length`, `password_max_age_seconds` (0 = never), `password_complexity`, `lockout_threshold`, `machine_account_quota` | default domain policy | |
| `finding_count` | objects concerned by a finding, 0 when examined and clean | `finding`, `severity`, `title` |
| `inventory_age_seconds`, `scrape_errors` | age of the last full inventory; parts of the directory that could not be read | |

The privileged groups are found by their well-known identifier, not their
name, so a domain installed in another language works the same:
Domain Admins (RID 512), Enterprise Admins (519) and Schema Admins (518) — the
last two only in the forest root domain — and the built-in Administrators,
Account Operators, Server Operators, Print Operators and Backup Operators.

## Findings

Findings are raw facts — how many objects, and which — not a score; the
device's [security score](../using/security-score.md) weighs them. Each has a
stable `id`, a severity on the score's scale (low, medium, high, critical,
shown on the page as Info, Advisory and Warning), a category, a count and up
to twenty sample account names. A finding that was examined and found nothing
is kept with a count of 0, so a fixed problem turns green instead of
disappearing; findings about accounts appear once the first background
inventory has completed. They are served as is by
`GET /api/targets/{id}/ad/findings` and published as
`dumbmonit_ad_finding_count{finding, severity, title}`.

| `id` | Severity | What |
|---|---|---|
| `unconstrained_delegation` | critical | enabled accounts trusted for unconstrained delegation, domain controllers aside |
| `kerberoastable_privileged` | critical | enabled members of privileged groups with a service principal name |
| `asrep_roastable_users` | high | enabled accounts with Kerberos pre-authentication disabled (`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | high | enabled users with a service principal name |
| `krbtgt_password_age` | high | krbtgt password older than 180 days |
| `laps_coverage` | high when LAPS is absent, medium when partial | enabled computers (domain controllers aside) without a LAPS password |
| `reversible_encryption` | high | users whose password is stored with reversible encryption |
| `guest_enabled` | high | the built-in Guest account (RID 501) is enabled |
| `obsolete_os` | high | enabled computers on Windows XP, Vista, 7, 8, 2000, Server 2003, 2008 or 2012 |
| `dc_unreachable` | high | domain controllers whose LDAP port does not answer |
| `replication_failing` | high | inbound replication partners with consecutive failures |
| `dc_not_synchronized` | high | the queried domain controller has not finished its initial replication |
| `fsmo_role_orphaned` | high | a FSMO role held by a deleted domain controller |
| `password_policy` | high below 8 characters or without lockout, medium otherwise | gaps in the default domain policy against the baseline: 14 characters, a lockout threshold, complexity |
| `privileged_group_size` | medium | more than 5 enabled accounts in Domain Admins, Enterprise Admins and Administrators together |
| `stale_accounts` | medium | enabled users and computers inactive for more than the thresholds |
| `protocol_transition_delegation` | medium | constrained delegation with protocol transition |
| `des_only` | medium | accounts restricted to DES Kerberos keys |
| `password_not_required` | medium | users allowed an empty password (`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | medium | privileged users whose password never expires |
| `privileged_stale` | medium | privileged users inactive for more than `stale_days_users` |
| `schema_admins_not_empty` | medium | Schema Admins has members |
| `functional_level_old` | medium | domain functional level below 2012 R2 |
| `clock_skew` | medium | more than five minutes between the domain controller and DumbMonit |
| `windows10_end_of_support` | medium | enabled computers on Windows 10 |
| `admincount_orphans` | low | `adminCount=1` users no longer in a privileged group |
| `password_never_expires` | low | enabled users whose password never expires |
| `machine_account_quota` | low | any user can join computers to the domain |
| `recycle_bin_disabled` | low | the AD Recycle Bin is not enabled |

The [built-in rules](../alerting/rules.md#active-directory) that apply:

- **Domain controller unreachable**: another domain controller of the domain
  stops answering on its LDAP port for five minutes (Warning). The queried one
  is covered by *Device unreachable*.
- **Active Directory bind failed**: the domain controller refuses the service
  account — expired password, disabled or locked out (Advisory).
- **Privileged group membership changed**: someone joined or left a
  privileged group, nesting included (Advisory, resolves an hour later).
- **Active Directory replication failing**: an inbound partner fails for half
  an hour (Advisory), when replication status is readable.
- **krbtgt password older than 180 days** (Info).

## The device page

The Active Directory panel shows the domain (functional levels, the queried
domain controller, clock skew), the domain controllers with their site, roles
and whether they answer, the privileged groups with their members, the
password policy and account counts, and the findings table with severity
words.

## Create a read-only service account for DumbMonit

1. Create a dedicated user for DumbMonit, for example svc-dumbmonit, and add it to no group. A plain domain user is enough: every authenticated user can read what DumbMonit reads. In PowerShell, with the Active Directory module:

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. Give it a long random password. If passwords expire in your domain, DumbMonit raises Active Directory bind failed the day this one does.

3. Optional: to see inbound replication status, grant the account the Monitor active directory replication right on the domain object itself. It reveals no password and no secret.

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. LDAPS needs a certificate on each domain controller; an enterprise certification authority (AD CS) issues one automatically. Export the certificate of that authority in Base-64 encoded X.509 format and paste it as CA certificate below.

5. In DumbMonit, enter the name of one domain controller as its certificate names it, for example "dc1.corp.example.com", and log in as svc-dumbmonit@corp.example.com. DumbMonit finds the other domain controllers by itself and checks that each one answers.

6. DumbMonit only reads: it never writes to the directory, and of LAPS it reads the expiry date, never the password.

!!! warning
    Plain LDAP on port 389 sends the service account's password in clear text: DumbMonit refuses it unless you tick Allow plain LDAP, and a domain controller that requires LDAP signing refuses it too. Use LDAPS, or StartTLS on port 389.

To keep the account from opening a session anywhere, deny it *Log on
locally* and *Log on through Remote Desktop Services* by Group Policy: an LDAP
bind is not affected. Do not restrict it with *Log On To* or *Deny access to
this computer from the network*: the domain controllers would then refuse the
bind.

### Least privilege, in detail

- The account needs no group beyond Domain Users, which every account has.
- Do not use a managed service account (gMSA): it cannot do a simple LDAP
  bind.
- Do not grant *Replicating Directory Changes* (or *All*): that is the DCSync
  right, which exposes every password hash. DumbMonit never needs it.
- *Monitor active directory replication* only allows reading replication
  metadata (partners, last success, error codes). It is optional.

## LDAPS and certificates

- **LDAPS (port 636)** is the default. Every domain controller of a domain
  with an enterprise certification authority receives a certificate
  automatically; without one, the domain controller does not listen on 636.
  The certificate must carry the name DumbMonit uses: enter the domain
  controller by its DNS name, not its IP address, unless the certificate lists
  the address.
- **CA certificate**: paste the PEM of the root (or issuing) authority. Only
  that authority is then trusted. Without it, DumbMonit trusts the public
  roots and the system store of its container, which do not know a private
  enterprise authority.
- **StartTLS (port 389)** encrypts the same way, after a plain-text greeting.
  Use it when port 636 is filtered.
- **Accept an unverifiable certificate** keeps the connection encrypted but
  accepts any certificate: anyone on the path could pose as the domain
  controller and receive the service account's password.
- **LDAP channel binding** (`LdapEnforceChannelBinding`) concerns NTLM and
  Kerberos binds over TLS; the simple bind DumbMonit uses is unaffected.

## Credentials

| Field | Value |
|---|---|
| User name | `svc-dumbmonit@corp.example.com`, `CORP\svc-dumbmonit`, or the full DN of the account |
| Password | the account's password, stored encrypted and never shown again |

An empty password is refused: Active Directory would accept it as an
unauthenticated bind without checking anything.

## Options

| Option | Default | What |
|---|---|---|
| Connection security (`security`) | `ldaps` | `ldaps`, `starttls` or `plain` |
| Port (`port`) | 636, or 389 | used if the address does not give one |
| CA certificate (`ca_cert`) | | PEM of the authority that issued the domain controllers' certificates |
| Accept an unverifiable certificate (`insecure_tls`) | off | |
| Allow plain LDAP (`allow_plaintext`) | off | required for `plain` |
| Check every domain controller (`check_all_dcs`) | on | TCP connection to the LDAP port of each domain controller |
| Inactive user after (`stale_days_users`) | 90 | days |
| Inactive computer after (`stale_days_computers`) | 90 | days |
| Full inventory every (`inventory_minutes`) | 15 | minutes, 1 to 1440 |
| Timeout (`request_timeout_seconds`) | 8 | per LDAP operation, 1 to 60 |

## Troubleshooting

- **TLS certificate rejected**: paste the authority's certificate as CA
  certificate, and enter the domain controller by a name its certificate
  lists.
- **Bind refused: wrong user name or password (LDAP result 49)**: the
  diagnostic code from the domain controller is translated — password
  expired, account disabled, locked out, or must change its password at next
  logon.
- **The domain controller requires a protected connection**: LDAP signing is
  enforced; switch to LDAPS or StartTLS.
- **A domain controller shows as unresolved**: its name does not resolve
  from the DumbMonit host. Point DumbMonit's DNS at the domain's DNS servers,
  or untick *Check every domain controller*. Unresolved names never raise an
  alert.
- **Replication status not readable**: grant *Monitor active directory
  replication* as in step 3, or accept that replication is not shown.
