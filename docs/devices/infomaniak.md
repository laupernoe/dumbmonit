# Infomaniak

The products of an Infomaniak account through the Infomaniak API: domains,
hosting, mail and other services that are about to expire, locked, or under
maintenance.

DumbMonit reads `GET /1/product` on `api.infomaniak.com` once a minute, for
the account you name (or for every account the token can list). The
credential is an API token created in the Infomaniak Manager, sent as a
bearer token.

This is an Internet API: the address of the device is the API's, not a
machine's. DumbMonit needs outbound HTTPS to `api.infomaniak.com`; nothing
else is contacted.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from the Infomaniak API documentation, not
    against a live account. It only reads. Tell us what breaks.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Infomaniak API (HTTPS, api.infomaniak.com) |

## What it watches

All metrics are prefixed `dumbmonit_infomaniak_`. The first 200 products are
described; the counts stay exact beyond that.

| Metric | Labels | Meaning |
|---|---|---|
| `product_expiry_seconds` | `product`, `service`, `id` | Until the product expires; negative once it has. Absent for products without an expiry date (free ones). |
| `product_locked` | `product`, `service`, `id` | 1 when Infomaniak flags the product as locked. |
| `product_maintenance` | `product`, `service`, `id` | 1 when the product is under maintenance. |
| `product_operation_in_progress` | `product`, `service`, `id` | 1 while an operation is running on the product. |
| `products`, `products_expiring`, `products_expired`, `products_locked`, `products_maintenance` | | Counts (`products_expiring`: less than 14 days left). |

The Infomaniak API gives no finer state than the expiry date and these
flags: it does not tell whether a site or a mailbox is up. Use an HTTP monitor
for that.

The [built-in rules](../alerting/rules.md#infomaniak) that apply:

- **Infomaniak product expiring**: less than 14 days left (Advisory,
  reminded daily).
- **Infomaniak product expired**: the expiry date has passed (Critical).
- **Infomaniak product locked**: for an hour (Advisory).
- **Infomaniak product in maintenance**: for an hour (Info).

## The monitor page

The panel says what is wrong in a sentence, shows the number of products,
those expiring soon and those expired, then lists the products with their
state.

## Create a read-only API token for DumbMonit

1. Open the Infomaniak Manager token page and sign in.
   `https://manager.infomaniak.com/v3/ng/profile/user/token/list`

2. Click Create a token and name it DumbMonit.

3. Choose the scopes listed below, and set the validity you want (no expiry avoids renewing it).

4. Click Create the token, then copy it: it is shown once.

5. In DumbMonit, enter api.infomaniak.com as address and paste the token. Fill the Account ID option if the token has no Accounts scope.

### Required permissions

- **Products (scope: product)** — Lists the products of the account with their expiry date and flags; changes nothing.
- **Accounts (scope: accounts)** — Lists the account IDs of the token; optional when the Account ID option is filled.

!!! warning
    The Infomaniak API gives no finer state than the expiry date, the lock and the maintenance flags of each product: it does not tell whether a site or a mailbox is up. Use an HTTP monitor for that.

## Credentials

| Credential | Fields |
|---|---|
| API token | The token created in the Infomaniak Manager, sent as a bearer token. |

Address: `api.infomaniak.com`. Options:

- **Account ID**: the account to read. Empty: every account the token can
  list (up to 10), which needs the Accounts scope.
- **Services**: comma-separated service names to read (`domain, hosting`).
  Empty: every product of the account.
- **Timeout per request** (15 s).

## Troubleshooting

"Infomaniak refused the API token": the token is wrong, expired or revoked,
or it lacks a scope. When it lacks the Accounts scope, fill the Account ID
option.

"Infomaniak knows nothing at /1/product…": check the Account ID.

"Infomaniak answered 422 …": the API wants a service name for this account;
fill the Services option.
