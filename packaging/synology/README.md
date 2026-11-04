# DumbMonit agent for Synology DSM 7

A minimal `.spk` that installs the existing DumbMonit agent on a Synology NAS.
Besides what the agent collects on any Linux machine, it detects DSM and reads,
as the package user (no root): RAID arrays (`/proc/mdstat`), volumes, disks
(`/sys/block`, `/run/synostorage`), packages (`/var/packages`, `synopkg status`
when allowed), SMB/NFS/SSH and the UPS (`upsc`). See
`crates/agent/src/collect/dsm.rs` and `docs/devices/synology-agent.md`.

## Layout

- `INFO.in`: package metadata template (`@VERSION@`, `@SPK_ARCH@` are injected).
- `conf/privilege`: the service runs as the dedicated package user, not root.
- `WIZARD_UIFILES/install_uifile`: asks for the server HTTPS URL, the enrollment
  token (password field) and the server certificate pin (`sha256//...`, from
  Settings > System > HTTPS; optional only with a CA-trusted certificate).
- `scripts/`: `start-stop-status`, `postinst`, `preuninst`, `postuninst` (POSIX sh).
- `build-spk.sh`: assembles the package.

## Build

One package per architecture, from an agent binary built for that target
(`x86_64-unknown-linux-gnu`/musl or `aarch64-...`):

```sh
packaging/synology/build-spk.sh x86_64  path/to/dumbmonit-agent 0.15.0
packaging/synology/build-spk.sh aarch64 path/to/dumbmonit-agent 0.15.0
```

Output: `dist/dumbmonit-agent-<version>-<arch>.spk`. Icons are resized from
`web/static/icon-192.png` (72 px) and `icon-512.png` (256 px) when ImageMagick
is installed; otherwise the originals are copied unresized.

## Enrollment and secrets

`postinst` writes `agent.yaml` (mode 0600, owned by the package user) in the
package `var` directory (`/var/packages/dumbmonit-agent/var`, mode 0700), then
runs a single test send (`--once`) with its output discarded. If the server is
unreachable, the pin does not match or the token is refused, the installation fails and the file is
removed. The pin is written to `agent.yaml` as `server_pin`. The token and the pin are never logged (no tracing, output discarded). The agent
reads its enrollment token from that file at every start, so it stays there,
readable by the package user only, and is deleted with the package. The agent
only makes outbound connections; the package opens no port.

## Signature

Agent self-updates stay verified by the existing Ed25519 key (see
`docs/development/agent-signing.md`): the package binary is the agent, and the
agent refuses unsigned updates. The `.spk` itself is not signed yet (DSM will
show it as coming from an unknown publisher); package signing comes later.
No key is created here.

## Self-update

`target/bin` belongs to DSM and is read-only for the package user, so the agent
could not replace itself there (it reported `read_only` and never updated).
`start-stop-status` now copies the packaged binary to `var/bin/` (writable) and
starts that copy. The agent self-updates it in place (same SHA-256 and Ed25519
checks, `.previous` roll-back and ledger next to it). On each start the packaged
binary replaces the `var/bin` one only when its version is strictly newer (a
package upgrade), so a self-updated agent is never downgraded. Packages older
than 0.17.1 need one manual `.spk` install to get this.

## To validate on a real NAS

Install on DSM 7 (x86_64 and ARM), wizard validation, run-as user permissions
(reading `/proc`, `/sys`, volumes), start/stop from the Package Center,
upgrade keeping the configuration, uninstall cleanup, behaviour when the
server uses the encrypted agent channel (`tls_url`/`ca_pin` are not asked by
the wizard yet), self-signed server with and without the pin.
