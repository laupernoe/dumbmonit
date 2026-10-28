# Terraform provider for DumbMonit

Manage a DumbMonit instance as code: monitors, notification channels, alert
rules, maintenance windows and status pages. Built on the
[Terraform Plugin Framework](https://developer.hashicorp.com/terraform/plugin/framework).
It talks to the REST API with an administrator API token (scope `write`).

This provider is not part of the server image; its own workflow
(`.github/workflows/terraform-provider.yml`) builds and vets it.
Terraform and the declarative YAML files of DumbMonit
([Configuration files](../../docs/using/config-files.md)) are two ways to the
same goal; pick one per object, not both.

## Build and install

```bash
go build -o terraform-provider-dumbmonit .
```

Put the binary where a development override finds it (`~/.terraformrc`):

```hcl
provider_installation {
  dev_overrides {
    "registry.terraform.io/laupernoe/dumbmonit" = "/path/to/this/directory"
  }
  direct {}
}
```

## Configuration

```hcl
terraform {
  required_providers {
    dumbmonit = { source = "registry.terraform.io/laupernoe/dumbmonit" }
  }
}

provider "dumbmonit" {
  url   = "https://monit.example.org"   # or DUMBMONIT_URL
  token = var.dumbmonit_token           # or DUMBMONIT_TOKEN, admin token with write scope
}
```

`insecure = true` skips certificate verification (self-signed lab instances only).

## Resources

| Resource | Manages | Notes |
|---|---|---|
| `dumbmonit_target` | a monitor | `kind`, `address`, `name`, `interval_secs`, `enabled`, `group_name` (folder), `tags`, `credential_json` (sensitive) |
| `dumbmonit_notification_channel` | a channel | `kind`, `settings_json`, `secrets_json` (sensitive), `policy_json` |
| `dumbmonit_alert_rule` | a rule you wrote | `query`, `operator`, `threshold`, `for_secs`, `severity`, `selector_json`, `channel_ids`; built-in rules are not managed here |
| `dumbmonit_silence` | a one-off maintenance window | `starts_at`, `ends_at` (RFC 3339, UTC), `target_id`, `matchers`; immutable, any change replaces it |
| `dumbmonit_status_page` | a status page | `title`, `slug`, `published`, `theme`, `show_uptime_days`, `items` |

Data sources: `dumbmonit_target` (by `name`, or `kind` + `address`) and
`dumbmonit_notification_channel` (by `name`).

Free-form parts are JSON strings, so every kind of monitor and channel works
without the provider knowing it: `GET /api/collectors` and `GET /api/notify/kinds`
list the fields. Secrets are write-only: the server never returns them, so a
change made outside Terraform to a credential or a channel secret is not
detected.

## Example

```hcl
resource "dumbmonit_notification_channel" "phone" {
  name          = "Phone"
  kind          = "ntfy"
  settings_json = jsonencode({ server = "https://ntfy.sh", topic = "homelab-alerts" })
  secrets_json  = jsonencode({ token = var.ntfy_token })
}

resource "dumbmonit_target" "website" {
  name          = "Website"
  kind          = "http"
  address       = "https://example.org"
  interval_secs = 60
  group_name    = "Web"
  tags          = { env = "prod" }
}

resource "dumbmonit_target" "switch" {
  name            = "Core switch"
  kind            = "snmp"
  address         = "10.0.0.2"
  credential_json = jsonencode({ type = "snmp_community", community = var.snmp_community })
}

resource "dumbmonit_alert_rule" "slow" {
  name          = "Website is slow"
  query         = "dumbmonit_probe_duration_seconds"
  operator      = ">"
  threshold     = 2
  for_secs      = 300
  severity      = "warning"
  selector_json = jsonencode({ kind = "ids", ids = [tonumber(dumbmonit_target.website.id)] })
  channel_ids   = [tonumber(dumbmonit_notification_channel.phone.id)]
}

resource "dumbmonit_status_page" "home" {
  title     = "Home lab"
  slug      = "home"
  published = true
  items = [
    { target_id = tonumber(dumbmonit_target.website.id), label = "Website" },
  ]
}

resource "dumbmonit_silence" "maintenance" {
  name      = "Router upgrade"
  starts_at = "2026-11-01T22:00:00Z"
  ends_at   = "2026-11-01T23:00:00Z"
  target_id = tonumber(dumbmonit_target.switch.id)
}
```

Import an existing object with its numeric id: `terraform import dumbmonit_target.website 12`.

## Development

```bash
go vet ./...
go build ./...
```
