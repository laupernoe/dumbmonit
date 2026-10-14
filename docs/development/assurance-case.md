# Assurance case

This page argues that DumbMonit meets its security requirements: it states the
threats, the boundaries where trust changes, why the design limits the damage of
a mistake, how each common class of weakness is countered, and what evidence
backs each claim. It extends the [security model](security-model.md), which
lists the same measures more briefly. To report a vulnerability, follow the
[security policy](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

File paths below are relative to the repository root.

## Security requirements

1. Only an authenticated user, with a role that allows it, may read or change
   the configuration, the alerts or the measurements.
2. The credentials DumbMonit stores for your devices are not readable from the
   database file, from a backup bundle without its passphrase, or from the API.
3. Only a machine holding a valid token may feed measurements in, and a leaked
   token can be revoked on its own.
4. DumbMonit cannot be turned into a way to reach services it should not,
   in particular its own unauthenticated VictoriaMetrics child.
5. A page served to a browser cannot run script it did not ship with, and a
   third-party site cannot make a signed-in browser change anything.

## Threat model

| Attacker | Capability | In scope |
|---|---|---|
| Anonymous network client | Reaches the HTTP port | Yes |
| Signed-in user with a low role | Valid session or read-only API token | Yes (privilege escalation) |
| Malicious web page | Runs in an admin's browser | Yes (CSRF, XSS, clickjacking) |
| Compromised or malicious monitored device | Answers probes with hostile content | Yes (parsers, redirects) |
| Malicious or stolen agent | Holds one agent token | Yes (reach of a single token) |
| Admin entering a hostile URL | Configures a webhook or device address | Yes, for the server's own addresses |
| Someone with the host, the `/data` volume or the container runtime | Full control | No: out of scope, as in the security policy |
| Vulnerabilities inside VictoriaMetrics or the monitored devices | Upstream code | No: report upstream |

## Trust boundaries

| Boundary | What crosses it | How it is guarded | Evidence |
|---|---|---|---|
| Browser to `/api/*` | Session cookie or API token, JSON | One authentication middleware, three roles, CSRF header on cookie-authenticated writes | `crates/server/src/auth/middleware.rs`; tests `auth_api.rs`, `operator_role_api.rs`, `api_open_security_api.rs`, `cors_policy_api.rs` |
| Browser to served pages | HTML, script, style | Content Security Policy with a per-response nonce, `nosniff`, `Referrer-Policy: same-origin`, `X-Frame-Options: DENY` | `crates/server/src/api/mod.rs` (security headers layer) |
| Identity provider to server | OpenID Connect redirect and ID token | State, nonce and PKCE (S256); local account linking is explicit | `crates/server/src/auth/oidc/`; test `oidc_api.rs` |
| Agent to `/api/ingest` | Measurements, command results | Per-agent token, stored as a SHA-256 hash (`dmon_` prefix) | `crates/server/src/api/ingest.rs`, `crates/server/src/collectors/agent/token.rs`; test `agent_binding_api.rs` |
| Relay agent to server | Probe results from another network | Same agent token check (`agent_commands::authenticate`) | `crates/server/src/api/relay.rs`; test `relay_api.rs` |
| Server to monitored devices | Probe requests, hostile responses | Redirects followed on the same host only, bounded redirect count, response parsers with unit tests, certificate verification on by default | `crates/collectors/src/http.rs` (`same_host_redirects`) |
| Server to notification channels | Webhook and SMTP requests | Same redirect policy plus a guard that refuses the server's own addresses, checked before sending, on each redirect and at DNS resolution | `crates/server/src/notify/guard.rs`; test `notify_guard_api.rs` |
| Server to VictoriaMetrics | Writes and read queries | Embedded child listens on `127.0.0.1:8428` by default; the Prometheus-compatible proxy relays only the read routes (query, query_range, series, labels) | `crates/server/src/config.rs` (`DUMBMONIT_VM_LISTEN`), `crates/server/src/api/prometheus.rs` (`readable_segments`) |
| Server to SQLite | Configuration and state | Bound parameters through `sqlx` | `crates/server/src/db/` |
| Server to `/data` | Database, secret key | Secrets encrypted with AES-256-GCM; the container user owns `/data` only | `crates/server/src/crypto.rs`, `Dockerfile` |

## Secure design argument

The argument follows the design principles of Saltzer and Schroeder.

- **Economy of mechanism.** One binary, one SQLite file, one loopback child
  process, one entrance for every client (`/api/*`).
- **Fail-safe defaults.** On a fresh instance every route under the
  authentication guard answers `401`; only health, auth status, setup and login
  respond, so nobody can prepare the instance before its owner
  (`auth/middleware.rs`, test `api_open_security_api.rs`). VictoriaMetrics
  listens on loopback unless the operator changes it. TLS certificates of
  monitored services are verified unless the operator sets `insecure_tls`
  (`crates/collectors/src/http.rs`).
- **Complete mediation.** The authentication middleware runs on every API route
  rather than each handler checking for itself.
- **Open design.** The code, the API description (`crates/server/src/api/openapi.yaml`)
  and this page are public. Security does not rely on secrecy of the design.
- **Separation of privilege.** Sessions, API tokens and agent tokens are
  different credentials. API tokens carry a `read` or `write` scope
  (`auth/token.rs`) and users have a viewer, operator or admin role.
- **Least privilege.** The image runs as UID 65532 (`Dockerfile`); the Compose
  file drops all capabilities, sets a read-only root file system and
  `no-new-privileges` (`docker-compose.yml`).
- **Least common mechanism.** Agents authenticate with their own token, so
  revoking one machine does not touch any user or other agent.
- **Psychological acceptability.** Secure behaviour is the default and needs no
  extra step: the cookie is `HttpOnly` and `SameSite=Lax`, the CSRF header is
  added by the web client, secrets are write-only in the interface.
- **Defence in depth.** Encryption at rest on top of access control; the CSRF
  header on top of `SameSite`; the login rate limit on top of Argon2 hashing; a
  redirect policy on top of the self-address guard.

### Cryptography

- Passwords: Argon2 (`auth/password.rs`). Session and API token secrets are
  stored as SHA-256 hashes of random values (`auth/session.rs`,
  `auth/token.rs`).
- Secrets at rest: AES-256-GCM with a fresh nonce per encryption
  (`crypto.rs`). The key is derived with Argon2id from `DUMBMONIT_SECRET` or the
  generated key file, with a persistent salt; a secret shorter than 16
  characters is refused, and a canary value detects a wrong key at start.
- Backups: the key derives from the operator's passphrase with Argon2id, with
  parameters stored in the bundle, so the algorithm can change without breaking
  old bundles (`backup/bundle.rs`).
- Second factor: TOTP, with hashed single-use recovery codes (`auth/totp.rs`).
- OpenID Connect: state, nonce and PKCE `S256` (`auth/oidc/pkce.rs`).
- All primitives come from maintained libraries (`aes-gcm`, `argon2`, `sha2`);
  none is home-made. The built-in listener speaks plain HTTP; the
  documentation requires a TLS reverse proxy, and cookies gain `Secure` when the
  request arrives over HTTPS (`auth/cookie.rs`).

## Countermeasures to common weaknesses

| Weakness (OWASP Top 10, CWE) | Countermeasure | Evidence |
|---|---|---|
| Broken access control, CWE-862, CWE-285 | Authentication middleware, roles, token scopes | `operator_role_api.rs`, `api_tokens_api.rs`, `api_open_security_api.rs` |
| Cryptographic failures, CWE-327, CWE-312 | AES-256-GCM at rest, Argon2, no secret returned by the API | `crypto.rs` unit tests; `has_secret` / `credential_kind` instead of the secret |
| SQL injection, CWE-89 | Bound parameters through `sqlx` | `crates/server/src/db/` |
| Cross-site scripting, CWE-79 | Svelte escapes output; CSP with `default-src 'none'`, scripts only from `'self'` and a nonce; `base-uri 'none'` | `api/mod.rs` |
| Cross-site request forgery, CWE-352 | Writes by cookie need `X-Requested-With: DumbMonit` or same-origin fetch metadata; CORS grants nobody the header; `SameSite=Lax` | `auth/middleware.rs`, `auth/cors.rs`; test `cors_policy_api.rs` |
| Clickjacking, CWE-1021 | `X-Frame-Options: DENY` and `frame-ancestors 'none'`, except on opt-in public status pages meant to be embedded | `api/mod.rs` |
| Server-side request forgery, CWE-918 | Same-host redirects only; the notification guard refuses the server's own listen and VictoriaMetrics addresses, including after DNS resolution (rebinding). Loopback and private ranges stay reachable on purpose, because homelab services live there | `notify/guard.rs`; test `notify_guard_api.rs` |
| Authentication failures, CWE-307, CWE-287 | Login rate limit with growing delay per address and per account, optional TOTP, OIDC with PKCE, admin reset of a second factor | `auth/rate_limit.rs`, `auth/totp.rs`, `auth/oidc/` |
| Insecure deserialization, resource exhaustion, CWE-400 | Typed JSON via `serde`, request bodies capped at 16 MiB, bounded redirects | `api/mod.rs`, `collectors/src/http.rs` |
| Memory safety, CWE-787, CWE-416 | Rust; `unsafe` appears only in the Windows agent's performance counter code (`crates/agent/src/collect/perf_counters.rs`) | grep for `unsafe` |
| Vulnerable components, CWE-1104 | `cargo deny` (licences, bans, advisories); Dependabot; Trivy scan of the published images; actions pinned by commit; `--locked` builds | `.github/workflows/ci.yml`, `security.yml`, `.github/dependabot.yml`, `deny.toml` |
| Software and data integrity, CWE-494 | Agent installers verify the SHA-256 of the downloaded binary | `crates/agent/install/install.sh`, `install.ps1` |
| Logging and monitoring failures, CWE-778 | Security log of sign-ins and sensitive changes | `auth/audit.rs`; test `security_audit_api.rs` |

Known residual risks are listed in the security model (plain HTTP listener,
unencrypted device options, unauthenticated embedded VictoriaMetrics). The
CSP allows inline styles (`style-src 'unsafe-inline'`), which weakens it against
style injection but not against script execution. One accepted advisory
exists, `RUSTSEC-2023-0071`, ignored in `deny.toml` with its reason: the `rsa`
crate verifies RS256 signatures with the provider's public key and never uses a
private key.

## Evidence

- **Continuous integration** (`.github/workflows/ci.yml`): `cargo fmt --check`,
  `clippy -D warnings`, the full Rust test suite, `cargo deny check licenses bans`
  and `cargo deny check advisories`, web type check, web tests and build, and a
  build of the Docker image, on every push and pull request.
- **Scheduled scans** (`.github/workflows/security.yml`): Trivy on the published
  server and agent images, and the OpenSSF Scorecard, weekly and after each
  `:edge` publication. Results appear in the repository Security tab, where
  GitHub code scanning (CodeQL) reports as well.
- **Tests**: API, authentication, role, CORS, OIDC, guard and audit tests under
  `crates/server/tests/`, unit tests next to the code, and web tests under
  `web/tests/`. Rust line coverage measured with `cargo-llvm-cov` is 87.9 %.
- **Regression policy**: a fix comes with a test that fails without it
  (see the Test policy in `CONTRIBUTING.md`).
- **Public record**: security fixes are published as advisories and listed in
  the release notes.
