# Microsoft Entra ID

Microsoft Entra ID through Microsoft Graph: application secrets and
certificates about to expire, a stalled synchronization with the on-premises
Active Directory, and administrators with no multifactor authentication.

An expired application secret is the quiet outage of an Entra tenant: the
backup job, the sign-in of an intranet or the mail relay that used it stops
working that day, with nothing to warn you. DumbMonit reads the expiry date of
every secret and certificate of every app registration (never their values)
and alerts 30 days before.

DumbMonit signs in as an Entra application (client credentials flow): it
exchanges the application's client ID and secret for a one-hour token at
`login.microsoftonline.com`, keeps it until it expires, and reads
`graph.microsoft.com` once a minute. Every call is a read.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the Microsoft Graph v1.0 documentation,
    not against a live tenant. It only reads. Tell us what breaks.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Microsoft Graph API (HTTPS, graph.microsoft.com) |
| 443 | TCP | Microsoft sign-in, token request (HTTPS, login.microsoftonline.com) |

## What it watches

All metrics are prefixed `dumbmonit_entra_`.

| Metric | Labels | Meaning |
|---|---|---|
| `credential_expiry_seconds` | `app`, `kind` (`secret` or `certificate`), `key` | Seconds until the secret or certificate expires; negative once expired. Secrets expired for more than 30 days are not published. |
| `applications`, `credentials` | | App registrations read, and secrets and certificates found. |
| `credentials_expired` | | Every expired secret or certificate, however old. |
| `credentials_expiring_30d` | | Secrets and certificates that expire within 30 days. |
| `ad_sync_enabled` | | 1 when the tenant synchronizes an on-premises directory (Entra Connect). |
| `ad_sync_age_seconds` | | Time since the last synchronization; only when sync is enabled. |
| `admins`, `admin_without_mfa` | | Administrators, and those with no MFA method registered. |
| `admin_mfa_registered` | `admin` | 0, for each administrator without MFA (user principal name). |
| `mfa_readable` | | 1 when the registration report could be read, 0 when Microsoft refused it. |

The `key` label is the name given to the secret or certificate, with the
start of its identifier (`prod (aaaa1111)`), so that two secrets with the same
name stay distinct.

Only the applications and the organization are mandatory. The MFA report
needs an Entra ID P1 or P2 license: without it, `mfa_readable` is 0 and the
probe still succeeds.

The [built-in rules](../alerting/rules.md#microsoft-entra-id) that apply:

- **Entra application secret expiring**: a secret or certificate expires in
  less than 30 days (Advisory, reminded daily).
- **Entra application secret expired**: a secret or certificate expired in
  the last 30 days (Advisory, reminded daily). Delete it, or renew it if its
  application still needs it.
- **Entra directory sync late**: the last sync with the on-premises Active
  Directory is more than 3 hours old (Advisory, reminded every 6 hours).
- **Entra administrator without MFA**: an administrator has no MFA method
  registered (Advisory, reminded daily).

## The monitor page

The Entra ID panel says what is wrong in a sentence, shows the secrets that
expire or expired, the directory sync state and the administrators without
MFA, then lists the secrets expiring within 90 days, the sync state and each
administrator without MFA.

## Create a read-only Entra application for DumbMonit

1. In the Microsoft Entra admin center (entra.microsoft.com), open Entra ID → App registrations → New registration, name it DumbMonit, keep a single tenant and register it.

2. On the Overview page, copy the Application (client) ID and the Directory (tenant) ID.

3. Open API permissions → Add a permission → Microsoft Graph → Application permissions, add the permissions listed below, then click Grant admin consent.

4. Open Certificates & secrets → New client secret and copy its Value (not the Secret ID): it is shown once and it expires, so note the date.

5. In DumbMonit, enter graph.microsoft.com as address, paste the client ID and the secret value, and fill in the Tenant ID option.

### Required permissions

Add them as application permissions (not delegated), then grant admin consent.

- **Application.Read.All** — Reads app registrations: the expiry date of each secret and certificate, never their values.
- **Organization.Read.All** — Reads the tenant and its on-premises directory sync status.
- **Reports.Read.All** — Reads admin MFA registration; needs Entra ID P1 or P2 (optional).

!!! warning
    Admin consent gives the application tenant-wide read access to the data above. Use a dedicated app registration with only these permissions, and renew its client secret before it expires: DumbMonit watches its own secret like any other.

Vendor documentation: https://learn.microsoft.com/en-us/graph/api/application-list

### Notes

The same app registration can serve the Microsoft 365 integration: add the
permissions of both pages to it and use the same credentials. DumbMonit
watches its own client secret like any other: the **Entra application secret
expiring** rule fires 30 days before it stops working.

## Credentials

| Credential | Fields |
|---|---|
| Entra application | Application (client) ID and the client secret Value. |

Address: `graph.microsoft.com`. Options:

- **Tenant ID** (required): the Directory (tenant) ID, a GUID, or a verified
  domain name.
- **Sign-in URL**: `https://login.microsoftonline.com` by default. Change it
  only for a sovereign cloud (for example `https://login.microsoftonline.us`,
  with `graph.microsoft.us` as address).
- **Timeout per request (seconds)** (15 s).

## Troubleshooting

"Microsoft refused the application": the client ID or the secret is
wrong, or the secret expired. Copy the secret Value, not its ID.

"Microsoft Graph refused … (403): grant the application the
Application.Read.All application permission" (or `Organization.Read.All`): add
it and click Grant admin consent; it can take a few minutes to apply.

`mfa_readable` stays 0: the tenant has no Entra ID P1 or P2 license, or
`Reports.Read.All` was not granted. Nothing is broken.

No `ad_sync_age_seconds`: the tenant does not synchronize an on-premises
directory (`ad_sync_enabled` is 0), or Microsoft reports no sync date yet.

A secret you deleted from the portal is still listed for a minute: wait for
the next probe.
