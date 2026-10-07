# Connect an assistant

DumbMonit has a built-in [MCP](https://modelcontextprotocol.io) server: the
Model Context Protocol, the way assistants such as Claude, ChatGPT, Cursor or
VS Code's agent call outside tools. Connect one, then ask it things like:

- "Is everything fine?"
- "Silence the NAS for two hours, I'm swapping a disk."
- "What happened last night?"
- "How full is the backup server, and is it getting worse?"
- "Post an update on the status page: the mail server is back."

The assistant sees what the interface sees — devices, alerts, history,
metrics, agents, status pages — and uses the same words: *reporting*,
*unreachable*, *waiting*, *advisory*, *warning*, *suppressed by parent*. It
runs inside the server: nothing to install next to DumbMonit, and the tools
call the same code as the web UI.

## 1. Create a token

Settings → **API & assistants** (`/settings#assistant`). Name the token after
the assistant or the machine it runs on ("Claude on my laptop"), then choose:

| Setting | Choices |
| --- | --- |
| **Scope** | **Read**: look — status, devices, alerts, history, metrics, rules, agents, status pages. It can never change anything. **Read and write**: also act — silence, acknowledge, probe, add a device, restart a container, post an incident… (the [write tools](#what-the-assistant-can-call)). |
| **Expires** | 30 days, 90 days (the default), 1 year or never. An expired token stops working; create a new one. |
| **Allowed networks** | Optional. Addresses or CIDR ranges the token may be used from (`192.168.1.0/24, 10.8.0.5`). Empty: anywhere. |

The token looks like `dmt_` followed by 32 hexadecimal characters. It is shown
**once**, right after creation; DumbMonit only keeps a hash. Lose it, revoke it
and create another one.

Prefer a read token unless you actually want the assistant to act.

The same token also opens the [REST API](../reference/api.md#authentication)
to scripts and dashboards — `curl -H "Authorization: Bearer dmt_…"
https://monit.example.lan/api/targets` — where `read` sees what a viewer
sees and `write` does what an administrator does, accounts, tokens and backups
excepted.

## 2. Paste the snippet

The settings page fills these in with your server's address and the new
token. Below, `https://monit.example.lan` stands for your server.

### Claude Code

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

Add `--scope user` to have it in every project, not only the current one.

### Claude Desktop

Claude Desktop starts local servers from its configuration file;
[`mcp-remote`](https://www.npmjs.com/package/mcp-remote) bridges it to
DumbMonit's HTTP endpoint (it needs [Node.js](https://nodejs.org)). DumbMonit
authenticates with a bearer header rather than OAuth, so add it through this
file rather than the Connectors screen. Settings → Developer → **Edit
Config**, then add to `claude_desktop_config.json` and restart Claude Desktop:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote", "https://monit.example.lan/api/mcp",
        "--header", "Authorization:${DUMBMONIT_AUTH}"
      ],
      "env": { "DUMBMONIT_AUTH": "Bearer dmt_…" }
    }
  }
}
```

The header goes through an environment variable, with no space after the
colon, because some platforms split arguments on spaces. For a server reached
over plain HTTP on your network, add `"--allow-http"` to `args` — the settings
page does it for you when it is itself served over HTTP.

### VS Code

`.vscode/mcp.json` in a workspace (or the command **MCP: Add Server** → HTTP):

```json
{
  "servers": {
    "dumbmonit": {
      "type": "http",
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

### Cursor and other clients

`.cursor/mcp.json`, and the same shape for most clients that support
Streamable HTTP servers with custom headers:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

A client that only starts local (stdio) servers can use the `mcp-remote`
bridge shown for Claude Desktop.

### ChatGPT

Settings → Connectors → **Create** (developer mode). MCP server URL:
`https://monit.example.lan/api/mcp`, authorization header:
`Bearer dmt_…`.

ChatGPT connects from OpenAI's servers, not from your browser: DumbMonit must be
**reachable from the internet over HTTPS**. See the [security notes](#security-notes)
before doing that.

### Any other client

What a client needs to know: the endpoint is `https://<server>/api/mcp`, the
transport is Streamable HTTP, and every request carries
`Authorization: Bearer dmt_…`. A raw call, to check the token and the
network path from a terminal:

```bash
curl -s https://monit.example.lan/api/mcp \
  -H 'Authorization: Bearer dmt_…' \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

## What the assistant can call

Twenty-seven tools. Every one has a title, a description, an input schema, an
output schema and annotations (`readOnlyHint`, `destructiveHint`,
`idempotentHint`), so a client can tell which ones only look and ask you before
running the others.

| Tool | Scope | Does |
| --- | --- | --- |
| `get_status` | read | The bulletin, the answer to "is everything fine?": device counts, firing and building-up alerts, one sentence ("2 advisories, 1 unreachable."). |
| `list_devices` | read | Devices and services with state, parent and tags; filter by substring or state. |
| `get_device` | read | One device: configuration, state, active alerts, 24-hour CPU / memory / fullest-disk summary (availability and latency for a service check). |
| `list_alerts` | read | Active alerts with severity words, since when, value, suppression, acknowledgement. |
| `alert_history` | read | Alert transitions, most recent first (`since` or `hours`, optional device, limit). |
| `query_metrics` | read | A MetricsQL range query, at most 60 points per series. |
| `list_silences` | read | Maintenance windows and whether they are active now. |
| `list_rules` | read | Alert rules with kind, severity, threshold, enabled. |
| `list_device_types` | read | The device types this server can monitor, with the options and credentials each one expects — what `add_device` needs. |
| `list_agents` | read | Machines running the agent: OS, agent version, relay mode, site, devices relayed, last contact, binding. |
| `list_containers` | read | The Docker containers of a machine running the agent: state, health, restarts, update available, last command. |
| `list_heartbeats` | read | Heartbeat monitors: expected interval, last call, verdict (waiting, on time, missed, reported down). The secret URL is never shown. |
| `list_status_pages` | read | Status pages, published or not, with their public path (`/s/<slug>`), the devices they show and their open incidents. |
| `list_channels` | read | Notification channels: kind, enabled, last error, last message sent. Never their secrets. |
| `list_packs` | read | Installed integration packs: version, device type, enabled, devices using it, errors. |
| `silence_device` | write | A one-off maintenance window on a device, starting now (default 1 hour, at most a week). |
| `remove_silence` | write | Removes a maintenance window. |
| `schedule_maintenance` | write | A one-off maintenance window at a given date, on one device or the whole instance, a week at most. |
| `acknowledge_alert` | write | Acknowledges one alert by fingerprint for a number of hours (default 4): reminders pause, the resolution is still notified. `hours: 0` lifts it. |
| `probe_device` | write | Probes a device immediately and reports what was measured. |
| `set_device_enabled` | write | Enables or disables monitoring of a device. |
| `set_rule_enabled` | write | Enables or disables an alert rule. |
| `add_device` | write | Adds a device: name, type, address, interval, parent, options, credentials, relay agent. |
| `discover_network` | write | Scans a network (CIDR, 4096 addresses at most) for SNMP devices and suggests a profile for each. |
| `restart_container` | write | Asks a machine's agent to restart one of its containers. |
| `test_channel` | write | Sends a test message through a notification channel. |
| `post_incident` | write | Opens an incident on a status page, or posts an update to an existing one. |

A read token calling a write tool gets a clear refusal as the tool result
(`isError: true`); the assistant explains it and nothing changes. The same
goes for anything the tool cannot do — an unknown device, an ambiguous name, a
window longer than a week: the message says what to fix.

## Protocol details

`POST /api/mcp` implements MCP over the **Streamable HTTP** transport:
JSON-RPC 2.0, one request per `POST`, answered with plain JSON (no server-sent
events). It is stateless: no `Mcp-Session-Id` is issued, one a client sends is
ignored, and `GET` or `DELETE` on the endpoint answer `405`. Only the `tools`
capability is offered — no resources, prompts, sampling or OAuth.

Two generations of the protocol are served on the same endpoint:

| Versions | How a client speaks it |
| --- | --- |
| `2026-07-28` | Stateless. Every request carries `io.modelcontextprotocol/protocolVersion` and `io.modelcontextprotocol/clientCapabilities` in `params._meta`, and the headers `MCP-Protocol-Version`, `Mcp-Method` and, for `tools/call`, `Mcp-Name`, which must match the body. Methods: `server/discover`, `tools/list`, `tools/call`. Results carry `resultType: "complete"` and the server's identity in `_meta` (`io.modelcontextprotocol/serverInfo`); `tools/list` also carries `ttlMs` and `cacheScope`. |
| `2025-11-25`, `2025-06-18`, `2025-03-26` | The `initialize` handshake picks the version (an unknown one gets the newest of these three); `notifications/initialized` is acknowledged with `202`; then `ping`, `tools/list`, `tools/call`. |

Errors, besides the `401`, `403` and `429` of the [token checks](#security-notes),
which come before any JSON-RPC:

| HTTP | JSON-RPC code | When |
| --- | --- | --- |
| `400` | `-32020` (header mismatch) | A `2026-07-28` request without `MCP-Protocol-Version`, `Mcp-Method` or `Mcp-Name`, or with values that differ from the body. |
| `400` | `-32022` (unsupported protocol version) | A version this server does not speak; `error.data` lists the `supported` ones and the `requested` one. |
| `400` | `-32602` (invalid params) | A `2026-07-28` request missing the required `_meta` fields. |
| `404` | `-32601` (method not found) | A `2026-07-28` request for a method this server does not implement. |
| `200` | `-32602` | An unknown tool, or `arguments` that is not an object. |
| | `-32700`, `-32600` | Unreadable JSON; not a JSON-RPC 2.0 request (batches are not supported). |
| `202` | — | A notification: accepted, no body. |

A tool that runs but fails — missing scope, unknown device — is not a protocol
error: it answers normally with `isError: true` and a sentence the assistant
can relay.

## Security notes

- **A token is a password.** Anyone holding it reads everything the interface
  shows — and, with a write token, changes anything an administrator can
  through the REST API (devices, rules, channels), accounts, tokens and backups
  excepted. Keep it out of shared chats, screenshots and repositories. Revoke a
  token you are not sure about; create another one in a minute.
- **Prefer read.** Most questions ("is everything fine?") need no write scope.
  Create a write token only for an assistant you actually want to act, and
  name it so you recognise it in the list. Most clients ask before running a
  tool that is not marked read-only; leave that confirmation on.
- **Give it a lifetime and a network.** A token that expires in 90 days and
  only works from your LAN or VPN (`192.168.1.0/24`) is worth little to anyone
  who finds it. Behind a reverse proxy, declare the proxy in
  `DUMBMONIT_TRUSTED_PROXIES`, otherwise every call seems to come from the
  proxy and a network restriction refuses them all.
- **Bound to its creator.** A token stops working when the account that created
  it is disabled, loses its write power when that account is no longer an admin, and
  is revoked when that account is deleted.
- **Nothing secret comes back.** The tools never return device credentials
  (only their kind), channel secrets, heartbeat URLs or tokens. `add_device`
  does take credentials in: they travel through your assistant's conversation,
  and so through its provider. For a device with sensitive credentials, prefer
  the web interface.
- **Browsers and `Origin`.** A request that carries an `Origin` header (a
  browser-based client) is refused with `403` unless that origin is the
  server's own or is allowed by `DUMBMONIT_API_CORS_ORIGINS` (by default any
  origin, since the token is required anyway). List your origins there to
  narrow it; see [cross-origin requests](../reference/api.md#cross-origin-requests-cors).
- **Rate limit.** Each token is limited to 120 calls per minute, REST and MCP
  together: an assistant stuck in a loop gets `429` with `Retry-After`, not
  your server's full attention.
- **HTTPS before the internet.** ChatGPT's connectors need a public URL. Put
  DumbMonit behind a reverse proxy with TLS (Caddy, Traefik, nginx…) and set
  `DUMBMONIT_COOKIE_SECURE=1`. Never expose the plain HTTP port. Claude Desktop,
  Claude Code, VS Code and Cursor run on your machine and can reach a LAN
  address directly, so they need no exposure at all.
- **Watch "last used".** The token list shows when each token was last used
  and from which address. A token used at a time you were not talking to your
  assistant, or from an address you do not know, deserves a revocation.
- **Logs.** Every tool call is logged at `info` level with the token name and
  the tool, never the arguments; every write through the REST API with the
  token name, the method, the path and the status, never the body. Creating
  and revoking tokens is in the audit log (Settings → Security).

## Troubleshooting

| Symptom | Cause and fix |
| --- | --- |
| `401` / "A valid API token is required." | The header is missing or mistyped (`Authorization: Bearer dmt_…`, one space after `Bearer`), or the token was revoked. Create a new one. |
| `401` / "The API token "…" expired on …" | Create a new token; pick a longer lifetime if it was too short. |
| `403` / "cannot be used from …" | The token is restricted to other networks. The address in the message is the one the server sees: behind a reverse proxy, declare the proxy in `DUMBMONIT_TRUSTED_PROXIES`. |
| `403` on every call from a browser-based client | Its `Origin` is not allowed: add it to `DUMBMONIT_API_CORS_ORIGINS`. |
| A tool answers "needs a token with the "write" scope" | The token is read-only, or its creator is no longer an administrator. Create a write token if you want the assistant to act. |
| `429` | 120 calls in a minute: the assistant is probably looping. Wait for `Retry-After`, then ask a narrower question. |
| "Unsupported protocol version" | The client speaks a version DumbMonit does not; update the client. |
| Claude Desktop shows the server as failed | Node.js must be installed for `npx`; over plain HTTP, `--allow-http` must be in `args`. |
| ChatGPT cannot connect | It needs a public HTTPS URL; a LAN address or plain HTTP does not work from OpenAI's servers. |
