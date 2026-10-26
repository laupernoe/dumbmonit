# Agent channel security

Agents (and relay agents) connect to the server; the server never connects to
them. They do so over a **dedicated, encrypted agent port**
(`8443` by default) built to be exposed to the Internet: nobody on the path,
including someone on the agent's or the server's local network, can read or
alter what agents send or receive.

There is nothing to set up: the server creates its own certificate authority at
first start, and the install command shown in DumbMonit carries everything an
agent needs.

## Ports to open

| Where | Port | Why |
|-------|------|-----|
| Server, inbound | **TCP 8443** | Encrypted agent channel. The only port agents on other networks need. |
| Server, inbound | TCP 8080 | Web interface, install scripts and agent downloads. Can stay private (LAN, VPN). |
| Agents | none | Agents only make outbound connections. |

With Docker, publish the agent port next to the web port:

```yaml
    ports:
      - "8080:8080"
      - "8443:8443"
```

If a router or a TCP proxy publishes it under another number, set
`DUMBMONIT_AGENT_PUBLIC_PORT` so the install commands use that number.

## What protects the channel

- **TLS 1.3 only**, with AEAD cipher suites (AES-GCM, ChaCha20-Poly1305). No
  older version can be negotiated.
- **Post-quantum hybrid key exchange X25519MLKEM768.** The agent offers nothing
  else, so recorded traffic stays protected against a future quantum computer
  ("harvest now, decrypt later").
- **A client certificate per machine (mTLS).** The agent generates its key pair
  on the machine (ECDSA P-256); the private key never leaves it. At enrolment
  it sends the public key with its enrolment token, once, and receives a
  certificate valid 30 days. It renews it by itself 10 days before expiry,
  with a new key each time, authenticated by the current certificate.
- **Immediate revocation.** A certificate is only accepted while its machine
  exists: deleting the monitor in DumbMonit refuses it at the next request.
- **Pinned server.** The agent recognises the server only by the SHA-256
  fingerprint of DumbMonit's own certificate authority (`--ca-pin` in the
  install command). A certificate from a public authority, even valid, is
  refused: an attacker cannot impersonate the server, and a TLS-intercepting
  proxy cannot read the traffic.
- **No downgrade.** An agent installed with the fingerprint, or enrolled once,
  never talks in clear again; if the agent port cannot be reached it keeps its
  measurements and retries.
- **Two secrets per machine.** On top of the certificate, every request still
  carries the enrolment token and the machine's binding secret: a certificate
  cannot speak for another machine.
- **Throttled enrolment.** Wrong tokens are counted per address and blocked
  after a few attempts. Tokens are single use unless made reusable.
- **Nothing leaks before authentication.** The agent port serves the agent
  protocol only: no web interface, no version, no API description.
- **Keys at rest.** The authority's private key is stored encrypted with the
  instance key (`/data/secret.key`). On the agent, `agent-tls.json` (key and
  certificate) sits next to `agent.yaml`, readable by its owner only (mode 0600
  on Linux, macOS and FreeBSD; SYSTEM and Administrators only on Windows).

Commands sent to agents (container restart and update) and the probes
delegated to relay agents travel over the same channel.

## Threat model

Protected against:

- anyone able to listen to or modify the network between agent and server
  (public Wi-Fi, ISP, compromised router, someone on the same LAN);
- an attacker holding a valid public certificate for your domain, or a
  TLS-intercepting proxy;
- a stolen or leaked install command once the token is used (single-use
  tokens), and a leaked certificate once the machine is deleted;
- a compromised machine of the fleet trying to report for, or take the
  commands of, another machine.

Not protected against:

- someone with administrator rights on the agent machine (they can read its
  key, as they can read anything else there);
- someone with access to both the server's `/data` and its instance key;
- the very first install over plain HTTP: the install script and the binary
  come from the web port. Serve the web interface over HTTPS (or check the
  SHA-256 shown next to the command) if the network between you and the server
  is not trusted. The agent channel itself is pinned from the first request.

## Existing agents and the transition

On an instance that already had agents when it was updated, the plain channel
of the web port stays open, and every answer to an agent announces the
encrypted channel (its port and fingerprint). An up-to-date agent then enrols
by itself on the agent port with its token and binding secret, and switches.
This first fingerprint is trusted on first use: re-run the install command
from DumbMonit (it carries the fingerprint) on machines whose network you do
not trust.

**Settings → Agents** shows, for each agent, whether it reports encrypted
(with the key exchange used) or through the plain channel, and the switch
**Require the encrypted channel**. Turned on, the plain channel refuses agents;
new installations start with it on. `DUMBMONIT_AGENT_LEGACY_DAYS=N` closes the
plain channel automatically N days after the update.

Agents released before the encrypted channel do not know the encrypted channel: update them by
re-running the install command.

## Agent updates

Agents update themselves from the server (see
[Automatic updates](../devices/agent.md#automatic-updates)). Integrity comes
from the SHA-256 in the offer; authenticity from an Ed25519 signature, checked
against a release public key compiled into the agent, so a compromised or
impersonated server cannot push arbitrary code. Builds without a signature are
refused unless an administrator turns on *Accept unsigned builds (development)*
**and** the offer comes over the encrypted channel. The download goes through
the agent's own pinned channel, with its token, binding secret and machine key.

## Reverse proxies

Do not put a TLS-terminating reverse proxy in front of the agent port: the
client certificate would stop at the proxy. A TCP pass-through (HAProxy `mode
tcp`, nginx `stream`, Traefik TCP router with TLS passthrough, port forwarding)
works. The web port can stay behind your usual HTTPS reverse proxy.

## Rotating the certificate authority

In **Settings → Agents → Agent channel → Certificate authority rotation**:

1. **Start rotation** creates a new authority. Agents learn its fingerprint
   over the encrypted channel at their next report; it signs nothing yet.
2. Wait until all agents know it (the counter shows how many).
3. **Complete rotation**: the server now presents a certificate from the new
   authority. Agents that did not report in between are refused and must be
   reinstalled with the new install command.

## Settings

| Variable | Default | Meaning |
|----------|---------|---------|
| `DUMBMONIT_AGENT_LISTEN` | `0.0.0.0:8443` | Listen address of the agent channel; `off` closes it. |
| `DUMBMONIT_AGENT_PUBLIC_PORT` | listen port | Port written in the install commands. |
| `DUMBMONIT_AGENT_LEGACY_DAYS` | unset | Close the plain channel N days after the update. |

On the agent (`agent.yaml` or environment): `tls_url` /
`DUMBMONIT_AGENT_TLS_URL` (default: host of `server_url`, port 8443) and
`ca_pin` / `DUMBMONIT_AGENT_CA_PIN`.

## Install parameters for deployment tools

`GET /api/agent/install?base_url=…` returns the web URL, the agent channel URL,
the CA fingerprint, the binary checksums, and the Unix and PowerShell install
commands with `{TOKEN}` to replace. GPO or Ansible generators should use it
rather than rebuilding the command.
