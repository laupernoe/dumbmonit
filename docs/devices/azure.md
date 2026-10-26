# Microsoft Azure

The health of an Azure subscription: resources that Azure reports as
unavailable or degraded, and the Azure Monitor alerts that are fired and not
closed.

DumbMonit reads two lists on `management.azure.com`, once a minute:
**Resource Health** (`Microsoft.ResourceHealth/availabilityStatuses`, API
2022-10-01), the availability that the Azure platform announces for each
resource, and the **Azure Monitor alerts** (`Microsoft.AlertsManagement/alerts`,
API 2019-05-05-preview) that are fired and either new or acknowledged. The
credential is a service principal: an Entra application with a client secret
and the **Reader** role on the subscription. DumbMonit exchanges the secret for
a one-hour token (client credentials flow, scope
`https://management.azure.com/.default`) and keeps it until it expires. The
Reader role can read and change nothing.

A stopped (deallocated) virtual machine is reported by Azure as unavailable
because its owner asked for it: DumbMonit counts it apart and does not alert.

This is an Internet API: the address is the API's, not a machine's. DumbMonit
needs outbound HTTPS to `management.azure.com` and `login.microsoftonline.com`;
nothing inside your resources is contacted.

!!! warning "Validated against the vendor documentation only — not yet tested on a real system"

    This integration was built from Microsoft's REST API documentation, not
    against a live subscription. It only reads. Tell us what breaks.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Azure Resource Manager API (HTTPS, management.azure.com) |
| 443 | TCP | Microsoft sign-in, access token (HTTPS, login.microsoftonline.com) |

## What it watches

All metrics are prefixed `dumbmonit_azure_`.

| Metric | Labels | Meaning |
|---|---|---|
| `resources` | | Resources with a Resource Health status. |
| `resources_available`, `resources_degraded`, `resources_unavailable`, `resources_stopped`, `resources_unknown` | | Resources by state; `stopped` is unavailable on the owner's request. |
| `resource_unhealthy` | `resource`, `type`, `resource_group`, `state` | Value 1 for each degraded or unavailable resource. |
| `alerts_readable` | | 0 when the alerts cannot be read (see Troubleshooting); Resource Health is still reported. |
| `alerts_active` | `severity` (`Sev0` to `Sev4`) | Fired alerts, new or acknowledged, by severity. |
| `alerts_high_active` | | Fired alerts of severity Sev0 or Sev1. |
| `alerts_total` | | All fired alerts. |

At most 200 named resources and 200 named alerts are listed; the counts stay
exact. Lists are read over 20 pages at most.

The [built-in rules](../alerting/rules.md#microsoft-azure) that apply:

- **Azure resource unavailable**: a resource unavailable for 10 minutes
  (Critical).
- **Azure resource degraded**: a resource degraded for 15 minutes (Warning).
- **Azure Monitor alert Sev0 or Sev1**: a Sev0 or Sev1 alert fired and not
  closed for 5 minutes (Warning).

## The monitor page

The Azure panel says what is wrong in a sentence, shows the number of
resources, the unavailable and degraded ones and the active alerts, then lists
the unhealthy resources (and the stopped ones) and the active alerts.

## Create a read-only application for DumbMonit

1. In the Azure portal, open Microsoft Entra ID > App registrations > New registration, name it DumbMonit, keep single tenant, then Register.

2. On the Overview page, copy the Application (client) ID and the Directory (tenant) ID.

3. Open Certificates & secrets > New client secret, choose a lifetime, then copy the secret Value: it is shown once.

4. Open Subscriptions > your subscription > Access control (IAM) > Add > Add role assignment, pick the Reader role, select the DumbMonit application, then Review + assign.

5. Copy the Subscription ID from the subscription Overview page.

6. In DumbMonit, enter management.azure.com as address, paste the client ID and secret, and fill the Tenant ID and Subscription ID options.

### Required permissions

- **Reader (role on the subscription)** — Reads Resource Health and the active Azure Monitor alerts; changes nothing.

!!! warning
    A stopped or deallocated virtual machine is reported by Azure as unavailable on the user's request: it is counted apart and raises no alert. Azure Monitor alerts need the Microsoft.AlertsManagement resource provider registered on the subscription; without it only Resource Health is monitored.

Vendor documentation: https://learn.microsoft.com/rest/api/resourcehealth/availability-statuses/list-by-subscription-id and https://learn.microsoft.com/rest/api/monitor/alertsmanagement/alerts

## Credentials

| Credential | Fields |
|---|---|
| Entra application (service principal) | Application (client) ID and client secret Value. |

Address: `management.azure.com`. Options:

- **Tenant ID** (required): the Directory (tenant) ID of the application.
- **Subscription ID** (required): the subscription to watch.
- **Sign-in URL**: `https://login.microsoftonline.com` by default; change it
  only for a sovereign cloud.
- **Timeout per request** (15 s).

The client secret expires (at most 24 months): renew it before, or monitor it
with the Microsoft Entra ID integration.

## Troubleshooting

"Microsoft refused the application": the client ID or secret is wrong, or the
secret has expired.

"Azure refused to read the Resource Health (403)": the application does not
have the Reader role on that subscription; role assignments can take a few
minutes to apply.

"Azure knows no subscription …": check the Subscription ID and the Tenant ID.

`alerts_readable` is 0: the application cannot read Azure Monitor alerts, or
the Microsoft.AlertsManagement resource provider is not registered on the
subscription (Subscriptions > Resource providers).
