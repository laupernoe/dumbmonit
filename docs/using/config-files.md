# Configuration files (GitOps)

Describe your monitors, alert rules, notification channels, maintenance windows
and status pages in YAML, keep the files in Git, and let DumbMonit converge to
them. Three ways to use the same format:

- **the web interface**: *Settings → System → Configuration files* exports the
  instance, shows the plan of a file and applies it;
- **`dumbmonit-cli`**: `plan` and `apply` from a terminal or a CI pipeline;
- **a watched folder**: `DUMBMONIT_CONFIG_DIR` is applied at startup and on
  every change, which suits a Git checkout mounted into the container.

A [Terraform provider](https://github.com/laupernoe/dumbmonit/tree/main/integrations/terraform-provider-dumbmonit)
manages the same objects one by one through the REST API.

## The file

```yaml
version: 1

channels:
  - name: Phone
    kind: ntfy
    enabled: true
    settings: { server: "https://ntfy.sh", topic: homelab-alerts }
    secrets: { token: "${NTFY_TOKEN}" }

targets:
  - kind: http
    address: https://example.org
    name: Website
    interval_secs: 60
    enabled: true
    group_name: Web            # the folder shown on the Monitors page
    tags: { env: prod }
  - kind: snmp
    address: 10.0.0.2
    name: Core switch
    interval_secs: 60
    enabled: true
    parent: "http|https://example.org"   # a reference: kind|address
    credential: { type: snmp_community, community: "${file:/run/secrets/snmp}" }

rules:
  - uid: website-slow
    name: Website is slow
    kind: threshold
    query: dumbmonit_probe_duration_seconds
    operator: ">"
    threshold: 2
    for_secs: 300
    severity: warning
    selector: { kind: refs, refs: ["http|https://example.org"] }
    channels: [Phone]                    # by channel name

rule_overrides:
  - rule_uid: cpu_high                   # a built-in rule
    target: "snmp|10.0.0.2"
    threshold: 95

silences:
  - name: Night backups
    schedule: { kind: weekly, days: [6], start_minute: 120, end_minute: 240, timezone: Europe/Paris }
    enabled: true

status_pages:
  - slug: home
    title: Home lab
    published: true
    theme: default
    show_uptime_days: 90
    items:
      - { target: "http|https://example.org", label: Website }
```

The sections are those of the [backup bundle](../install/backup.md), with the
same keys and the same references, so `GET /api/config/export` shows every
field. Objects are found by their natural key and never by an internal id: a
monitor by `kind` + `address`, a rule by `uid`, a channel by `name`, a status
page by `slug`, a maintenance window by `name` (plus its monitor, if it has
one). The file is therefore portable from one instance to another.

Not covered by files: users and tokens, incidents, agent enrolment, the world
map, packs. Monitors created by [Docker labels](docker-labels.md) or found by an
agent are not exported, and a file cannot declare them. Built-in rules are not
declared either: change them with `rule_overrides`. The password of a private
status page is set in the interface and kept when a file is applied.

## Secrets

A file never holds a secret. Wherever one is needed (`credential` of a monitor,
`secrets` of a channel) write a reference:

| Reference | Meaning |
|---|---|
| `${NAME}` | the environment variable `NAME` |
| `${file:/path}` | the content of a file, trailing newline removed (Docker or Kubernetes secrets); a relative path starts from the folder of the YAML file |

References are resolved where the secret lives: by `dumbmonit-cli` on your
machine, or by the server for the watched folder. The API itself **refuses** a
reference that is still there, so an administrator cannot make the server read
its own environment. The export replaces every secret with a reference such as
`${DUMBMONIT_CHANNEL_PHONE_TOKEN}` and lists the variables to define at the top
of the file.

## Managed by file

Whatever a file declares is marked **Managed by file**: a badge in the lists, and
the API (and so the interface) refuses to edit or delete it by hand, because the
next apply would silently overwrite the change. A monitor can be paused. To take
control back:

- change the file and apply it again (the normal way);
- or *Settings → System → Configuration files → Release*
  (`POST /api/config/release`): every object becomes editable again, nothing is
  deleted, and the next apply adopts the declared ones again.

An existing object that a file declares is adopted: it appears in the plan as an
update with `managed_by_file`.

## Plan and apply

`POST /api/config/plan` returns what would be created, updated or deleted,
naming the fields that differ (never their values) and changing nothing.
`POST /api/config/apply` does it **in one transaction**: a reference to a
missing monitor, a value the database refuses, anything, and nothing is
written.

**Prune.** By default nothing is ever deleted. With `prune`, objects that a file
manages and that the file no longer declares are deleted. Objects created by
hand are never touched, whatever the option.

```bash
dumbmonit-cli login --url https://monit.example.org      # asks for an admin API token
dumbmonit-cli plan  config/ --check                      # exit status 2 if anything would change
dumbmonit-cli apply config/ --prune
dumbmonit-cli export -o current.yaml
```

A directory argument applies every `.yaml` / `.yml` file in it as one document.
Both commands accept `--json`.

## The command-line tool

`dumbmonit-cli` is a single binary with no runtime dependency. Build it from the
repository (`cargo build --release -p dumbmonit-cli`).

| Command | What it does |
|---|---|
| `login --url URL [--token T]` | checks the connection and stores the address and the token in `~/.config/dumbmonit/cli.json` (mode 600). `DUMBMONIT_URL` and `DUMBMONIT_TOKEN` work too |
| `export [-o FILE]` | writes the instance as a file |
| `plan FILE\|DIR... [--prune] [--check]` | shows the plan |
| `apply FILE\|DIR... [--prune]` | applies the files |
| `targets list` | lists the monitors |
| `silence create --name N --duration 2h [--target-id ID] [--match k=v] [--comment C]` | opens a maintenance window starting now |

The token must be an administrator's, with the `write` scope (Settings → API
tokens). Every command takes `--json` for scripts.

## Watched folder

```yaml
services:
  dumbmonit:
    environment:
      DUMBMONIT_CONFIG_DIR: /config
      NTFY_TOKEN_FILE: /run/secrets/ntfy
    volumes:
      - ./dumbmonit-config:/config:ro
```

Every `.yaml` / `.yml` file of the folder forms one document. It is applied with
`prune` at startup and every time a file changes (the folder is re-read every
five seconds, so a network volume or a Kubernetes mount works). A folder without
any YAML file applies nothing: a missing mount must not delete what the files
managed. The last result, or the last error, is shown in the settings section.

References in a watched folder use the server's environment and files.

## Reference

| Route | Purpose |
|---|---|
| `POST /api/config/plan` | the plan of `{ "yaml": "...", "prune": false }` (or `"config": {...}` as JSON) |
| `POST /api/config/apply` | applies it |
| `GET /api/config/export` | the instance as YAML (`?format=json` for the document and the variables) |
| `GET /api/config/status` | watched folder and managed objects |
| `POST /api/config/release` | hands everything back to manual editing |

Administrators only (session or `write` token). See the
[API reference](../reference/api.md).
