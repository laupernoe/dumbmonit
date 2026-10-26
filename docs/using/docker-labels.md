# Monitors from Docker labels

If the [agent](../devices/agent.md) runs on a Docker host, a container can
declare its own monitors with `dumbmonit.*` labels. DumbMonit creates them, keeps
them up to date and archives them when the container goes away. Nothing to click:
the monitor lives and dies with the `docker-compose.yml` that describes the
service.

!!! note
    The agent must read the Docker socket (the `docker` module of the agent, on
    by default) and the **Docker** module must not be switched off for the machine
    in DumbMonit.

## Example

```yaml
services:
  gitea:
    image: gitea/gitea:latest
    ports: ["3000:3000", "2222:22"]
    labels:
      dumbmonit.enable: "true"
      dumbmonit.name: "Gitea"
      dumbmonit.folder: "Apps/Dev"
      dumbmonit.tags: "env=prod,team=infra"
      dumbmonit.interval: "30s"
      dumbmonit.http.url: "http://{host}:3000/api/healthz"
      dumbmonit.tcp.port: "2222"
```

This creates two monitors in the folder `Apps/Dev`: an HTTP check named
`Gitea (HTTP)` and a TCP check named `Gitea (TCP)`. `{host}` stands for the host name
of the machine the agent runs on, as the agent reports it. DumbMonit must be able to resolve
it; otherwise write the full address in the label.

## Labels

| Label | Meaning |
|---|---|
| `dumbmonit.enable` | `true` to opt the container in. Without it, every other label is ignored. |
| `dumbmonit.name` | Name of the monitor. Default: the container name. With several checks, the type is appended (`Gitea (HTTP)`). |
| `dumbmonit.folder` | Folder, `/` for nesting (`Apps/Dev`). |
| `dumbmonit.tags` | Comma-separated: `env=prod,critical` (a tag without a value is `true`). |
| `dumbmonit.interval` | Check period: `30`, `30s`, `5m`, `1h`. At least 10 s. Default 60 s. |
| `dumbmonit.http.url` | Creates an HTTP monitor. `{host}` is replaced by the machine's host name. |
| `dumbmonit.http.method`, `.keyword`, `.keyword_absent`, `.accepted_status`, `.insecure_tls`, `.timeout` | Options of the HTTP monitor, same meaning as in the monitor form. |
| `dumbmonit.tcp.port` | Creates a TCP monitor on this port. |
| `dumbmonit.tcp.host` | Host for the TCP check. Default `{host}`. |
| `dumbmonit.ping.host` | Creates a ping monitor on this host. |
| `dumbmonit.adopt` | `true` to take over an existing monitor with the same type and address (see below). |

At most one monitor of each type per container. Each monitor has a stable key
(`project/service/http` for a Compose service, otherwise `container/http`), so
recreating the container, or scaling it, does not create duplicates.

## What DumbMonit does with them

- **Create.** A new label makes a new monitor, placed under the machine's agent
  as its *parent*: if the host is down, its monitors do not each raise their own
  alert.
- **Update.** Change a label (URL, interval, folder, tags…) and the monitor follows
  within a minute.
- **Archive.** When the container is removed or the label disappears, the monitor is
  **paused and dated**, not deleted: its history stays. It wakes up by itself if
  the labels come back. Nothing is archived while Docker cannot be read.
- **Read-only.** A monitor managed by labels shows a *Managed by Docker labels*
  badge. Its fields cannot be edited, nor can it be deleted, until you **Release** it
  (button on the monitor page; or `POST /api/targets/{id}/release`). You can still
  pause it. An archived monitor can be deleted directly.

## Adoption

If a monitor with the same type and address already exists (created by hand, or
imported), DumbMonit does **not** create a duplicate and does not touch yours: the
label is reported in the server log. Add `dumbmonit.adopt: "true"` to let the
labels take it over; from then on it is managed like the others.

## Security

Anyone who can start a container on a host with the agent can create monitors for
it. That is the same trust you already give them with the Docker socket. Each
declaration is validated like a monitor created by hand, and at most 200 are
accepted per machine.
