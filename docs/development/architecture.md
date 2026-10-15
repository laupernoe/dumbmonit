# Architecture

DumbMonit ships as one container. Inside it there is one Rust process that
serves the web UI and the API, plus a child process (VictoriaMetrics) that
the Rust process starts and supervises. Configuration and state live in
SQLite.

## Components

| Component | Where | Role |
|---|---|---|
| Server | `crates/server` | The single Rust binary: HTTP API (axum), authentication, scheduler, alerting engine, notification channels, reports. |
| Web UI | `web/` | SvelteKit single-page app (Svelte 5, Tailwind 4, uPlot). Built to static files and embedded in the server binary with `rust_embed`. |
| Time series | `crates/server/src/tsdb/` | Batched writer and query proxy for VictoriaMetrics. `embedded.rs` starts and supervises the bundled VictoriaMetrics child process (data in `/data/vm`); `DUMBMONIT_VM_URL` points to an external instance instead. |
| Config and state | `crates/server/src/db/` | SQLite with numbered migrations: monitors, credentials, users, rules, alert state, silences, channels. |
| Collectors | `crates/collectors`, `crates/server/src/collectors/` | One implementation of the `Collector` trait per kind of monitor or service, registered in a `Registry`. The scheduler only knows the trait. |
| Shared types | `crates/proto` | `Sample`, `Target`, `Credential`, the `Collector` trait and `ProbeError`, and the agent protocol. |
| Agent | `crates/agent` | Linux, macOS, FreeBSD and Windows agent that pushes host metrics; it can also act as a relay that runs collectors for a remote site. |
| Alerting | `crates/server/src/alerting/` | Rules, the alert state machine, suppression by parent, silences, learned baselines. |
| Notifications | `crates/server/src/notify/` | Channels (chat, push, on-call, e-mail, webhooks), described to the UI by `catalog.rs`. |
| Secrets | `crates/server/src/crypto.rs` | AES-256-GCM for credentials and tokens stored in SQLite; the key comes from `/data/secret.key` or `DUMBMONIT_SECRET`. |

## Data flow

```text
 devices / services ──(SNMP, HTTP APIs, probes)──► collectors ┐
                                                              │ samples
 agents ──(push, per-machine token)──► /api/ingest ───────────┤
 relay agent (collectors run at a remote site) ───────────────┤
                                                              ▼
                                           tsdb writer (batched) ──► VictoriaMetrics
                                                                           │
 browser / scripts / assistants ◄─ /api/* ◄─ query proxy ◄─────────────────┘
                                   │
                                   └─► alerting engine ──► notify channels

 SQLite (config, users, alert state, silences) is read and written by the
 API, the scheduler and the alerting engine.
```

1. The scheduler runs every enabled monitor on its interval through the
   collector registry. Agents and relays push their measurements to
   `/api/ingest` with a per-machine token.
2. Collectors return samples named `dumbmonit_<kind>_*` with a `target` label
   (the monitor id). The writer batches them into VictoriaMetrics.
3. The alerting engine evaluates rules against the stored series. `phase` is
   the engine's state; `effective_phase` is what users see (`suppressed` when
   a parent monitor is down). An unreachable monitor raises an alert; a
   configuration error is shown but not notified.
4. Notifications leave through the channels configured in the notification
   policy.
5. The UI and any script read everything through `/api/*`; there is no private
   half of the API.

## Extension points

- A new kind of monitor is a `Collector` implementation plus one
  `Registry::register` call. The UI forms are generated from
  `GET /api/collectors`.
- A new notification channel is a sender plus an entry in `notify/catalog.rs`;
  the UI renders the form from `GET /api/notify/kinds`.
- An SNMP device that only needs a profile needs no Rust: a YAML file under
  `profiles/`.

See [Development](../development.md) for the build instructions and
[Security model](security-model.md) for the trust boundaries.
