# DumbMonit agent for Synology DSM 7

A minimal `.spk` that installs the existing DumbMonit agent on a Synology NAS.
It collects what the agent already collects (CPU, memory, disks, network); no
Synology-specific collection yet.

## Layout

- `INFO.in`: package metadata template (`@VERSION@`, `@SPK_ARCH@` are injected).
- `conf/privilege`: the service runs as the dedicated package user, not root.
- `WIZARD_UIFILES/install_uifile`: asks for the server URL and the enrollment
  token (password field).
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
unreachable or the token refused, the installation fails and the file is
removed. The token is never logged (no tracing, output discarded). The agent
reads its enrollment token from that file at every start, so it stays there,
readable by the package user only, and is deleted with the package. The agent
only makes outbound connections; the package opens no port.

## Signature

Agent self-updates stay verified by the existing Ed25519 key (see
`docs/development/agent-signing.md`): the package binary is the agent, and the
agent refuses unsigned updates. The `.spk` itself is not signed yet (DSM will
show it as coming from an unknown publisher); package signing comes later.
No key is created here.

## To validate on a real NAS

Install on DSM 7 (x86_64 and ARM), wizard validation, run-as user permissions
(reading `/proc`, `/sys`, volumes), start/stop from the Package Center,
upgrade keeping the configuration, uninstall cleanup, behaviour when the
server uses the encrypted agent channel (`tls_url`/`ca_pin` are not asked by
the wizard yet).
