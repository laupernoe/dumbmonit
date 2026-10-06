# Integration packs

An integration pack adds a device type to DumbMonit without a new release. It
is a single YAML file that says where to read (an HTTP API or a Prometheus
`/metrics` page on the device), what to extract, and which alert rules come
with it. Once installed, the pack's type shows up in **Add device** next to the
built-in ones, and its devices are polled, charted and alerted on like any
other.

A pack is declarative: no code, no scripts. DumbMonit runs every request
itself, under rules the pack cannot change.

## What a pack can and cannot do

| A pack can | A pack cannot |
|---|---|
| Call `GET` or `POST` paths on the device's own address | Call any other host, follow a redirect to another host, or use an absolute URL |
| Read JSON (JSONPath), plain text (regular expressions) or the Prometheus text format | Run code, shell commands or templates beyond `{{option.x}}` and `{{credential.x}}` |
| Send the device's credential in a header or as basic/bearer authentication | Put a credential in a URL or a request body |
| Declare options shown in the device form (a switch channel, a threshold) | Write metrics outside its own `dumbmonit_<pack>_` prefix |
| Ship alert rules on its own metrics | Change a built-in device type or built-in rule |
| Bring SNMP profiles, in the format of the built-in ones | Read more than 4 MiB per response, make more than 16 requests per poll, or produce more than 5,000 series per poll |

Loopback and link-local addresses (`127.0.0.1`, `169.254.x.x`) are refused
unless the device's **Allow loopback and link-local targets** option is on,
exactly like the HTTP checks. Private LAN addresses are always allowed.

## Reference packs

The repository ships three packs under
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs), each with its
fixtures and expected output:

| Pack | Reads | Rules |
|---|---|---|
| `node-exporter` | a curated subset of Prometheus node_exporter (CPU, memory, filesystems, disks, network) | filesystem almost full, memory almost exhausted |
| `shelly-plug` | Shelly Gen2+ plugs and relays, `/rpc/Switch.GetStatus` | overheating |
| `speedtest-tracker` | the latest result of Speedtest Tracker's v1 API | internet below expectations, speed test failing |

## Install a pack

In the interface, as an administrator: **Settings → Integration packs**. Paste
the pack's YAML or upload its `pack.yaml`, then **Install the pack**. A pack
that does not pass the checks is refused with the list of what to fix; one
that passes is saved with its notes, if any. The same page turns packs on and
off and uninstalls them. Installed types appear under **Community packs** when
you add a device, marked *Pack*.

Through the HTTP API, you need an administrator session or an API token with
the `write` scope (Settings → **API & assistants**).

```bash
# Install, or update to a newer version of the same pack
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @packs/shelly-plug/pack.yaml http://localhost:8080/api/packs

# List what is installed
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/api/packs
```

The answer tells what happened (`created`, `updated` or `unchanged`), the
metrics the pack produces and how many alert rules were added. A pack that does
not pass validation is refused with the full list of problems, one per line.

| Call | Does |
|---|---|
| `GET /api/packs` | installed packs: version, type, metrics, rules, devices using it |
| `GET /api/packs/{id}` | one pack |
| `POST /api/packs` | install or update; body: the YAML (`application/yaml`), or `{"yaml": "…"}` in JSON |
| `PUT /api/packs/{id}/disable` | stop polling the pack's devices, keep everything else |
| `PUT /api/packs/{id}/enable` | poll them again |
| `DELETE /api/packs/{id}` | uninstall, with its rules; refused (`409`) while devices still use it |

With a session cookie instead of a token, add `X-Requested-With: DumbMonit` to
every `POST`, `PUT` and `DELETE`, as for the rest of the [API](../reference/api.md).

Installing takes effect immediately: the new type is available in the device
form without a restart. The only exception is a pack that brings SNMP
profiles; the answer then says `"restart_required": true`, because the SNMP
catalogue is read at startup.

### Updates and your changes

Updating a pack (same `id`, new content) replaces its extraction and its
description. Its alert rules are only ever *added*: a rule you changed,
disabled or deleted is never rewritten by an update.

Disabling a pack withdraws its device type and keeps everything else: devices,
history, rules. Its devices are no longer polled (each shows a configuration
error) and, like any device that stops reporting, raise *Device unreachable*
after three minutes: pause them too if you disable the pack for a while.

### Backups

Installed packs travel in [backups](../install/backup.md) with their YAML, and
are restored before the devices that use them.

## Check a pack before installing it

The server binary validates and tests packs offline:

```bash
# Schema and consistency checks, without a server or a database
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# Replay fixtures/ through the real extraction and compare with expected.prom
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` rewrites `expected.prom` from the fixtures (mount the
directory read-write and run with `--user "$(id -u)"` so the file stays yours).

Next: [write your first pack](first-pack.md) in about ten minutes, or read the
[format reference](format.md).
