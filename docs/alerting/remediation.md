# Runbooks and remediation

When an alert fires, DumbMonit can tell people what to do (**runbooks**) and,
if you allow it, do part of it (**remediation actions**), by hand or
automatically. Everything lives on the **Actions** page (in the top bar; on a
phone, from the Alerts page).

Running things on machines is sensitive. The design below puts safety first:
the catalogue is closed, the machine has the last word, automatic runs are
armed in two steps and every run is logged.

## Runbooks

A runbook is a Markdown text (headings, lists, `code`, **bold**, links) linked
to one or more of:

| Link | Matches |
|---|---|
| Alert rule | Alerts of that rule, by its uid (`host_down`, `disk_full`…) |
| Folder | Every device in that folder |
| Device type | Every device of that type (`agent`, `snmp`, `proxmox`…) |

A runbook shows:

- under the alert, when you unfold it on the Alerts page;
- on the device's page, under its alerts;
- in notifications, as a link (`Runbook: <title>: https://…/runbooks/12`),
  when the public URL is set in the notification policy. The text itself is
  never sent.

Any HTML in a runbook is shown as text, never rendered. Links only work for
`http(s)://` addresses and paths of this site. Administrators write runbooks;
everyone signed in reads them.

## Remediation actions

An action is one of four kinds. There is no free-form command field.

| Kind | What it does | Goes through |
|---|---|---|
| Restart a service | `systemctl restart -- <unit>` (Linux) or `net stop` / `net start` (Windows) | The agent |
| Restart a Docker container | The container restart of the Docker panel | The agent |
| Run a declared script | A script **declared on the machine itself** (see below) | The agent |
| Call a webhook | A JSON `POST` describing the action, device and alert | The server |

An action can be bound to one device, or follow the device of the alert that
triggers it. A webhook token is stored encrypted and never shown again; the
API only says that one exists.

### What the machine allows: `agent.yaml`

The agent refuses every service restart and every script unless its own
configuration allows them. Nothing on the server can change that.

```yaml
# /etc/dumbmonit/agent.yaml
actions: true                  # default: false
action_services: [nginx, php-fpm]   # "*" allows any service; empty: none
# actions_dir: /etc/dumbmonit/actions.d   # default: next to agent.yaml
```

| Key | Variable | Default |
|---|---|---|
| `actions` | `DUMBMONIT_AGENT_ACTIONS` | `false` |
| `action_services` | `DUMBMONIT_AGENT_ACTION_SERVICES` | *(empty)* |
| `actions_dir` | `DUMBMONIT_AGENT_ACTIONS_DIR` | `actions.d` next to `agent.yaml` (`C:\ProgramData\DumbMonit\actions.d` on Windows) |

`commands: false` still turns off every command, containers included.
Restart the agent after changing these settings or adding a script.

### Declaring a script

One YAML manifest per script in the actions folder. The file name, without
`.yaml`, is the script's name (lowercase letters, digits, `-` and `_`).

```yaml
# /etc/dumbmonit/actions.d/flush-cache.yaml
description: Flush the application cache
command: ["/usr/local/bin/flush-cache", "--quiet"]
timeout_secs: 60               # default 60, at most 300
params:                        # every parameter and its allowed values
  level: [low, high]
```

- `command` is a list: the program (an absolute path) and fixed arguments. It
  is never passed to a shell.
- Each parameter becomes **one** extra argument, `--level=high`. Only the
  values listed in the manifest are accepted.
- The script runs with an empty environment (a minimal `PATH`,
  `DUMBMONIT_ACTION=<name>`), no standard input, and is killed at the timeout.
  The first 16 KiB of its output are read; the last 4 KiB are kept in the log.
- The folder, the manifest and the program must not be writable by group or
  others, and must belong to `root` or to the agent's account. A manifest that
  breaks a rule is ignored (the agent logs why).

The agent reports its allowed services and declared scripts to the server, so
the server refuses up front what the machine would refuse anyway.

## Running an action by hand

Unfold an alert on the Alerts page: the actions that apply to its device are
listed under **Remediation**. Click one, then confirm. Operators and
administrators can do this. An operator always acts on the alert's device (or
on the device an administrator fixed on the action); only administrators can
name another device through the API.

## Automatic remediation

A policy runs an action by itself when a rule keeps firing:

| Setting | Meaning |
|---|---|
| After | Minutes the alert must have been firing (1 to 1440) |
| Attempts at most / per window | At most N runs per device in a sliding window |
| Only for this device | Restrict the policy to one device of the rule |

It never runs:

- during a maintenance window or a snooze (silenced alert);
- while a parent device is down (suppressed alert);
- on an alert someone dismissed;
- while a previous run of the same policy is still pending.

Two failures in a row **stop** the policy; an administrator must arm it again.

### Arming in two steps

A policy is created **disarmed**. **Arm…** asks the server for a single-use
code valid two minutes, with a plain summary of what will run; **Yes, arm it**
sends it back. The code only works for the account that asked for it, and is
consumed even when it does not match. Changing the policy, or the action it
uses, disarms it.

## The log

Every run is logged: who (account, or `policy:<id>`), what (action, kind,
parameters without secrets), where (device, alert), when, and the result with
its output (truncated to 4 KiB, end kept). The log shows under the alert, on
the device's page and on the Actions page. It is kept 180 days and is not part
of backups.

## Security

| Threat | Protection |
|---|---|
| Arbitrary command | Closed catalogue; scripts only exist on the machine, as manifests the server cannot write. |
| Argument injection | No shell anywhere: programs and arguments go to the system as `argv`. Each parameter is one `--key=value` argument; values are restricted to letters, digits and `. _ - : @ / +`, never starting with `-`, and must be listed in the manifest. Service names cannot start with `-` or `/`, and `systemctl` gets `--` before them. |
| Path traversal | Script names allow no `/`, `\` or `.`; the agent only loads a manifest it found by listing its folder. No symbolic links. |
| CSRF | Writes with the session cookie need `X-Requested-With: DumbMonit`. |
| Privilege escalation | Viewers cannot run anything; operators only run actions an administrator defined, on the alert's device; configuration and arming are administrator-only. API tokens cannot reach remediation at all. Arming needs a signed-in administrator, in person. |
| Replay | Each request to the agent carries a random nonce and an expiry (10 minutes). The agent refuses expired requests, expiries too far ahead and nonces it has already seen. It also never runs the same command twice. |
| Denial of service | At most 30 runs per minute for the whole instance, one pending run per action and device, per-policy attempt limits, script timeouts (300 s max), bounded output, bounded agent declarations. |
| SSRF (webhooks) | Same guard as notification channels: the server's own addresses are refused at resolution, connection and every redirect. No credentials in URLs. |
| Secret leaks | Webhook tokens are encrypted at rest, never returned by the API, never logged, and redacted from errors. Backups carry them inside the encrypted payload. |

Remaining risks worth knowing: whoever controls the server can run what the
machines allow (declare only what you are ready to see run); on Windows the
agent cannot check folder permissions itself, so keep the actions folder
writable by administrators only; a declared script is only as safe as the
script itself.

## Backup

Runbooks, actions (secrets included, inside the encrypted payload) and
policies are part of [backups](../install/backup.md). Policies are restored
**disarmed**. The run log is not exported.
