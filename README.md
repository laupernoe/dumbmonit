<div align="center">

<img src=".github/assets/logo.svg" alt="DumbMonit logo" width="96" height="96">

# DumbMonit

**Dumb-simple monitoring, from homelabs to mid-sized companies.**

One container, one IP address to type in, useful graphs and alerts in under a minute.

[![CI](https://github.com/laupernoe/dumbmonit/actions/workflows/ci.yml/badge.svg)](https://github.com/laupernoe/dumbmonit/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/readthedocs/dumbmonit/latest)](https://dumbmonit.readthedocs.io/en/latest/)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/15486/badge)](https://www.bestpractices.dev/projects/15486)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/laupernoe/dumbmonit/badge)](https://scorecard.dev/viewer/?uri=github.com/laupernoe/dumbmonit)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[Live demo](https://demo.dumbmonit.app)** · **[Website](https://dumbmonit.app)** · **[Documentation](https://dumbmonit.readthedocs.io/en/latest/)**

<img src=".github/assets/screenshots/overview-light.png" alt="DumbMonit overview: the bulletin sentence, counters, the weather window and the list of things that need you" width="900">

</div>

For people who watch ten machines and three switches and do not want to run Zabbix or assemble Prometheus + Grafana + Alertmanager.

- **The one-second answer.** The home screen reads like a weather bulletin: "Clear skies." or "1 unreachable, 4 building up."
- **One way to add anything.** Pick a type, read what to prepare on the device, fill the few fields that matter. SNMP profiles are detected on their own.
- **Quiet by default.** Useful rules on from the start, one notification instead of thirty when a switch goes down. 100 % open source (MIT), no paid edition. **DumbMonit is 100 % free and will stay free**: no paid version, no feature held back.

## Quick start

```bash
mkdir dumbmonit && cd dumbmonit
curl -fsSLO https://raw.githubusercontent.com/laupernoe/dumbmonit/main/docker-compose.yml
docker compose up -d
```

1. Open http://localhost:8080.
2. Create the first admin account with the one-time code printed by `docker compose logs dumbmonit`.
3. Add a monitor, connect a notification channel, send a test message.

Back up *Settings → Backup* **and** `/data/secret.key`. Updating, ports, lost password: see the
[installation guide](https://dumbmonit.readthedocs.io/en/latest/install/docker/).

## What it watches

67 integrations (monitor types), each with its own page in [the documentation](https://dumbmonit.readthedocs.io/en/latest/devices/):

- **Network and servers**: SNMP, firewalls (OPNsense, pfSense, FortiGate, MikroTik, UniFi…), Redfish, Active Directory, and an **agent** for Linux, Windows, macOS and FreeBSD.
- **Virtualisation and storage**: Proxmox (VE, Backup Server, Datacenter Manager, Mail Gateway), vSphere, Synology, TrueNAS, Unraid, Veeam.
- **Apps and services**: Nextcloud, Immich, Jellyfin, GitLab, Kubernetes, databases, HTTP/TCP/DNS/ping/TLS checks, heartbeats, **mail server and IP range blocklists** (Spamhaus, Barracuda, SpamCop…, with delisting links), and **website health and SEO** (blank page, `noindex`, title or canonical changes, redirects, slowness) with bulk CSV import sorted by platform.
- **Anything else**: network discovery and YAML **integration packs**, no release needed.

## What it does

- **Alerting that does not cry wolf**: a parent that is down hides its children, maintenance windows and silences (recoveries stay quiet too), forecasts, a seasonal baseline, and a bell on each monitor counting its active alerts.
- **23 notification channels**: Discord, Slack, Telegram, Matrix, ntfy, email, PagerDuty, webhook and more.
- **Public status pages** with history, incidents, email subscribers, badges and city scenes.
- **Wall mode** for a room monitor, with optional widgets (weather, crypto prices, stock quotes, exchange rates, critical CVEs, RSS news with pictures, Claude and OpenAI API usage), and an optional **security score** next to the main products (hosts, Proxmox, Synology, OPNsense, websites, Active Directory).
- **API and MCP server** (27 tools): ask Claude or ChatGPT "is everything fine?", or read it from Prometheus and Grafana.
- **Encrypted backup**, accounts with TOTP, OIDC single sign-on and **LDAP / Active Directory** sign-in, an installable phone app (PWA), and remote sites through an outbound-only relay agent.

One Rust binary with the UI embedded, VictoriaMetrics started by it, SQLite for configuration.

## Contributing

Bug reports, device profiles and integrations are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md) (Docker only, no Rust toolchain needed).
The interface and the [documentation](https://dumbmonit.readthedocs.io/) come in English, French, German, Spanish, Italian, Portuguese (Portugal and Brazil), Russian and Simplified Chinese. To fix or add a translation, open a pull request: see [Translating DumbMonit](CONTRIBUTING.md#translating-dumbmonit).
Security issues: [SECURITY.md](SECURITY.md). Roadmap: [public project](https://github.com/users/laupernoe/projects/2).

## Status

**Version 0.14.** It runs daily on the author's own network. Before 1.0 the HTTP API and the database schema can still change between minor versions (the [changelog](CHANGELOG.md) lists every change and how to upgrade), and some integrations have only met test instances, not years-old production hardware.
Formerly called EzyMonit; the old `EZYMONIT_*` variables are still accepted.

## Support DumbMonit

DumbMonit is free and has no company behind it. If it saves you time, you can support its development:

- **Liberapay** (recurring or one-off): [liberapay.com/laupernoe](https://liberapay.com/laupernoe)
- **Bitcoin**: `bc1phcqfc46ev9ztpuxyh3fmz6aaktshnz8z4ve9jk7pxdcyu9ks37wqq2kjgd`
- **Ethereum**: `0x67Cd93028943308B8c12782C9ad078D253278f40`
- **Solana**: `63u8bhkf63HBcNYbRRyaHPtfRtSUmbugtwaauzAxHtb3`

Bug reports, translations and device profiles help just as much.

## License

MIT, see [LICENSE](LICENSE). DumbMonit is free and will stay free.

DumbMonit is provided "as is", without warranty of any kind. The author and contributors accept no liability for any damage, undetected failure or unsent alert; validating it in your own environment is up to you.
