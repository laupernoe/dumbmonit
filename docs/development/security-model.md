# Security model

This page describes what DumbMonit protects, who it trusts, how the design
limits the damage of a mistake, and where to look in the code and tests. To
report a vulnerability, follow the
[security policy](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## What is at stake

DumbMonit runs inside your network. It stores the credentials it needs to read
every monitor (SNMP communities, API tokens, passwords), it receives
measurements from agents on your machines, and it sends requests to addresses
you configure. A compromise would expose those secrets and give an attacker a
foothold on the monitored network.

## Trust boundaries

| Boundary | Who is on the other side | Trust |
|---|---|---|
| Browser or script to `/api/*` | Admins, operators and viewers | Authenticated by a session cookie or an API token; three roles limit what each may do. |
| Agent to `/api/ingest` | Machines you installed an agent on | A per-machine token; a leaked token can be revoked alone. |
| Server to monitors and services | Third-party systems | Not trusted: responses are parsed defensively and redirects stay on the same host. |
| Server to notification channels | External services | Credentials are encrypted at rest and never returned by the API. |
| Server to VictoriaMetrics | A child process on loopback, or an external instance | The embedded one has no authentication and listens on loopback; an external one is the operator's responsibility. |
| `/data` volume and host | The operator | Trusted. An attacker who controls the host or the volume is out of scope. |

## Secure design principles applied

- **Least privilege.** The container runs as an unprivileged user with no
  capability, a read-only root file system and `no-new-privileges`. Three roles
  (viewer, operator, admin) restrict what a user may change
  (`crates/server/src/auth/users.rs`; test `operator_role_api.rs`).
- **Secure by default.** On a fresh instance only the health, setup, login and
  auth status routes answer until the first admin exists (test
  `api_open_security_api.rs`).
- **Economy of mechanism.** One binary, one SQLite file, one loopback child
  process, and one way in for every client: `/api/*`.
- **Fail-safe defaults.** A security check whose data is missing is `unknown`,
  never a pass; an unreachable monitor and a configuration error are different
  states.
- **Complete mediation.** One authentication middleware
  (`crates/server/src/auth/middleware.rs`) guards the API routes.
- **Separation of privilege.** Sessions, API tokens and agent tokens are
  different credentials with different reach.
- **Defence in depth.** Encryption at rest on top of access control, a CSRF
  header on top of cookie flags, a rate limit on top of password hashing.
- **No secret round trip.** The API returns `has_secret` and `credential_kind`,
  never the secret; omitting a secret on `PUT` keeps it.
- **Vetted cryptography only.** Passwords are hashed with Argon2, secrets are
  encrypted with AES-256-GCM (`crypto.rs`), two-factor authentication uses TOTP
  with hashed single-use recovery codes, OpenID Connect uses state, nonce and
  PKCE. Nothing is home-made.

## Countermeasures by class of weakness

| Class (OWASP Top 10 theme) | Measures |
|---|---|
| Broken access control | One authentication middleware; roles; a fresh instance refuses everything except setup, login, health and auth status. |
| Cryptographic failures | AES-256-GCM at rest, key kept outside the database; Argon2 password hashes; the built-in server speaks plain HTTP, so a TLS reverse proxy is documented. |
| Injection | SQL goes through `sqlx` with bound parameters; the VictoriaMetrics proxy only forwards read queries (`tsdb/mod.rs`, `proxy_read`); parsers are unit tested. |
| Insecure design | The threat model above; secrets never round-trip; a single-purpose instance. |
| Security misconfiguration | Hardened Compose files, loopback-only VictoriaMetrics, operator hardening notes in the security policy. |
| Vulnerable components | `cargo deny` checks licences, bans and RustSec advisories; a weekly Trivy scan of the published images; Dependabot; GitHub Actions pinned by commit. |
| Identification and authentication failures | Login rate limit per client address and per account (`auth/rate_limit.rs`), optional two-factor authentication, OpenID Connect, an admin can reset a user's second factor and sign them out. |
| Software and data integrity | Agent installers verify the SHA-256 of the downloaded binary; release images are built by CI. |
| Logging and monitoring | A security log of sign-ins and sensitive changes (`auth/audit.rs`). |
| Server-side request forgery | Integrations and notification channels follow redirects to the same host only, so a monitor cannot bounce a request and its API key elsewhere. |
| Cross-site request forgery | State-changing requests that rely on the cookie must carry `X-Requested-With: DumbMonit` or same-origin fetch metadata, otherwise `403`; token-authenticated requests carry no cookie. |

## Verification

Every push and pull request runs `clippy -D warnings`, the Rust test suite
(including API, authentication and role tests under `crates/server/tests/`),
the web tests under `web/tests/`, dependency checks and CodeQL analysis. The
OpenSSF Scorecard runs on a schedule. Security fixes come with a regression
test and are listed, with their CVE or GHSA identifier, in the release notes.

The [assurance case](assurance-case.md) gives the full threat model, the secure
design argument and the evidence behind each claim.

## Known limits

- The built-in server speaks plain HTTP; put TLS in front of it.
- Monitor options are stored unencrypted. Put secrets in credential fields.
- The embedded VictoriaMetrics has no authentication.
- DumbMonit is before 1.0; routes and fields can still change.
