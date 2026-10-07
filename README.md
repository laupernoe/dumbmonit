<div align="center">

<img src=".github/assets/logo.svg" alt="DumbMonit logo" width="96" height="96">

# DumbMonit

**Dumb-simple monitoring from the homelab to the small business.**

One container, one IP address to type in, useful graphs and alerts in under a minute.

[![CI](https://github.com/laupernoe/dumbmonit/actions/workflows/ci.yml/badge.svg)](https://github.com/laupernoe/dumbmonit/actions/workflows/ci.yml)
[![Documentation](https://readthedocs.org/projects/dumbmonit/badge/?version=latest)](https://dumbmonit.readthedocs.io/en/latest/)
[![Translation status](https://hosted.weblate.org/widget/dumbmonit/web-ui/svg-badge.svg)](https://hosted.weblate.org/engage/dumbmonit/)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/laupernoe/dumbmonit/badge)](https://scorecard.dev/viewer/?uri=github.com/laupernoe/dumbmonit)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**[Live demo](https://demo.dumbmonit.app)** · **[Website](https://dumbmonit.app)** · **[Documentation](https://dumbmonit.readthedocs.io/en/latest/)**

<img src=".github/assets/screenshots/overview-dark.png" alt="DumbMonit overview: the bulletin sentence, counters, the weather window and the list of things that need you" width="900">

</div>

> **Alpha for early testers.** Expect rough edges and breaking changes. See [Status](#status).

For people who watch ten machines and three switches and do not want to run Zabbix or assemble Prometheus + Grafana + Alertmanager.

- **The one-second answer.** The home screen reads like a weather bulletin: "Clear skies." or "1 unreachable, 4 building up."
- **One way to add anything.** Pick a type, read what to prepare on the device, fill the few fields that matter. SNMP profiles are detected on their own.
- **Quiet by default.** Useful rules on from the start, one notification instead of thirty when a switch goes down. 100 % open source (Apache 2.0), no paid edition.

## Quick start

```bash
mkdir dumbmonit && cd dumbmonit
curl -fsSLO https://raw.githubusercontent.com/laupernoe/dumbmonit/main/docker-compose.yml
docker compose up -d
```

1. Open http://localhost:8080.
2. Create the first admin account with the one-time code printed by `docker compose logs dumbmonit`.
3. Add a device, connect a notification channel, send a test message.

Back up *Settings → Backup* **and** `/data/secret.key`. Updating, ports, lost password: see the
[installation guide](https://dumbmonit.readthedocs.io/en/latest/install/docker/).

## What it watches

50+ integrations, each with its own page in [the documentation](https://dumbmonit.readthedocs.io/en/latest/devices/):

- **Network and servers**: SNMP, firewalls (OPNsense, pfSense, FortiGate, MikroTik, UniFi…), Redfish, Active Directory, and an **agent** for Linux, Windows, macOS and FreeBSD.
- **Virtualisation and storage**: Proxmox (VE, Backup Server, Datacenter Manager, Mail Gateway), vSphere, Synology, TrueNAS, Unraid, Veeam.
- **Apps and services**: Nextcloud, Immich, Jellyfin, GitLab, Kubernetes, databases, HTTP/TCP/DNS/ping/TLS checks, heartbeats.
- **Anything else**: network discovery and YAML **integration packs**, no release needed.

## What it does

- **Alerting that does not cry wolf**: parent/child suppression, maintenance windows, forecasts, a seasonal baseline.
- **22 notification channels**: Discord, Slack, Telegram, Matrix, ntfy, email, PagerDuty, webhook and more.
- **Public status pages** with history, incidents, email subscribers, badges and city scenes.
- **Wall mode** for a room monitor, and a **security score** per device.
- **API and MCP server** (27 tools): ask Claude or ChatGPT "is everything fine?", or read it from Prometheus and Grafana.
- **Encrypted backup**, accounts with TOTP and OIDC, and remote sites through an outbound-only relay agent.

One Rust binary with the UI embedded, VictoriaMetrics started by it, SQLite for configuration.

## Contributing

Bug reports, device profiles and integrations are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md) (Docker only, no Rust toolchain needed).
The web UI can be **translated on [Weblate](https://hosted.weblate.org/engage/dumbmonit/)** (8 languages besides English; only part of the interface is translated so far).
Security issues: [SECURITY.md](SECURITY.md). Roadmap: [public project](https://github.com/users/laupernoe/projects/2).

## Status

**Alpha.** It runs daily on the author's own network, but the HTTP API is not frozen, the database schema still moves, and some integrations have only met test instances.
Formerly called EzyMonit; the old `EZYMONIT_*` variables are still accepted.

## License

Apache 2.0, see [LICENSE](LICENSE).
