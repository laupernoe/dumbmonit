# HTTPS and encryption

DumbMonit encrypts the web interface and its API itself, on the port it already
uses (`8080` by default). Nothing else needs publishing: the same port answers
plain HTTP and HTTPS, and tells them apart by the first byte of the connection.

| | |
|---|---|
| Protocols | TLS 1.3, and TLS 1.2 with ECDHE + AEAD suites only (AES-GCM, ChaCha20-Poly1305). No CBC, no RSA key exchange, no 0-RTT. |
| Key exchange | Hybrid post-quantum **X25519MLKEM768** first, then X25519 and P-256. |
| Certificate | Your own (`DUMBMONIT_TLS_CERT` / `DUMBMONIT_TLS_KEY`), or a self-signed one created at first start. |
| HSTS | `Strict-Transport-Security: max-age=31536000` on every HTTPS response. |
| Session cookie | `__Host-dumbmonit_session`, `Secure`, `HttpOnly`, `SameSite=Strict` over HTTPS. |

The agents have their own encrypted channel on port 8443 (TLS 1.3 only, client
certificates): see [Agent channel security](agent-security.md).

## Out of the box

At first start, DumbMonit creates a self-signed certificate (ECDSA P-256, valid
397 days, renewed 30 days before it expires with the same key). Its private key
is stored encrypted with the instance key, like every other secret. The log
prints its fingerprint:

```
INFO HTTPS available on the web port (TLS 1.3/1.2, X25519MLKEM768) source=SelfSigned fingerprint=3A:9F:…
```

Open `https://<server>:8080`. The browser warns once about a certificate it does
not know: compare the fingerprint it shows with the one in the log (or in
**Settings › System › HTTPS and encryption**) before accepting it.

The certificate covers `localhost`, the loopback addresses, the host name of the
container (when you set one with `hostname:`) and the host of
`DUMBMONIT_PUBLIC_URL`. Add the names and addresses your browsers use with
`DUMBMONIT_TLS_NAMES` (for example `nas.lan,192.168.1.10`): the certificate is
then re-issued with the same key.

To get rid of the warning for good, either trust this certificate on your
workstations (**Download the certificate (PEM)** in the settings), or provide
one signed by an authority your browsers already trust.

## Your own certificate

Mount the certificate chain and its key, and point DumbMonit at them:

```yaml
services:
  dumbmonit:
    environment:
      DUMBMONIT_TLS_CERT: /certs/fullchain.pem
      DUMBMONIT_TLS_KEY: /certs/privkey.pem
    volumes:
      - /etc/letsencrypt/live/monit.example.com:/certs:ro
```

- The chain is PEM, server certificate first; the key is PEM (PKCS#8, PKCS#1 or
  SEC1).
- Both files are re-read within a minute when they change (a Let's Encrypt
  renewal needs no restart), or right away on `docker kill -s HUP dumbmonit`.
- An unreadable certificate at startup stops the server with a clear message; an
  unreadable renewal keeps the previous certificate and says so in the log and
  in the settings.

## Plain HTTP and the redirect

On a **new installation**, a browser that opens the interface over plain HTTP is
sent to HTTPS on the same address and port (`307`, then HSTS takes over).

On an **upgraded installation**, nothing changes by itself: the interface shows a
"This session travels in clear" banner, and an administrator turns the redirect
on in **Settings › System › HTTPS and encryption** once HTTPS works for everyone.
`DUMBMONIT_HTTPS_REDIRECT=on|off` forces the choice from the environment.

The redirect only applies to page navigations from browsers that reach
DumbMonit directly. These are never redirected, so scripts and machines keep
working over HTTP:

- the API (`/api/…`), including push heartbeats and OTLP;
- the install scripts and agent binaries (`/install.sh`, `/install.ps1`,
  `/download/…`);
- `/metrics`, `/federate` and the Prometheus read API;
- public status pages (`/s/…`, their custom domains, badges and embeds);
- connections from the loopback, and any request that comes through a proxy (see
  below).

`DUMBMONIT_TLS=off` turns HTTPS off entirely: the port serves plain HTTP, as
before.

## Behind a reverse proxy

A reverse proxy that terminates TLS and talks plain HTTP to the container keeps
working unchanged. Two things make it right:

1. **Forward the scheme.** A request carrying `X-Forwarded-Proto`,
   `X-Forwarded-For`, `X-Forwarded-Host`, `X-Real-IP` or `Forwarded` is never
   redirected: the proxy decides. Caddy, Traefik and Nginx Proxy Manager send
   these headers by default; with plain nginx, add
   `proxy_set_header X-Forwarded-Proto $scheme;` (see
   [Docker › Reverse proxy](docker.md#reverse-proxy)).
2. **Declare the proxy as trusted** with `DUMBMONIT_TRUSTED_PROXIES` (its
   address or network). Its `X-Forwarded-Proto: https` then counts: the session
   cookie becomes `Secure` with the `__Host-` prefix, and HSTS is sent. A
   forwarded header from any other address is ignored for these decisions.

The proxy can also talk HTTPS to the container (`https://dumbmonit:8080`); with
the self-signed certificate, pin it or disable verification on that one hop.

## Install commands

The commands shown when you add an agent use the address your browser is on.
When that is the self-signed HTTPS of this server, the Linux/macOS command pins
the server's public key (`curl -k --pinnedpubkey sha256//…` and
`--pinned-key=` for the script's own downloads): the script and the binary come
over HTTPS, verified by the key rather than by an authority. PowerShell cannot
verify a certificate by its key, so the Windows command downloads over HTTP on
the same port; the binary is still checked against the SHA-256 the command
carries. Behind a reverse proxy with a public certificate, both commands simply
use HTTPS.

In every case the agent itself then talks to the server over the pinned,
mutually authenticated agent channel.

## Security headers

Every response carries:

- `Content-Security-Policy` with `default-src 'none'`, scripts only from the
  server itself plus a per-response nonce, `frame-ancestors 'none'` (status pages
  and their embed view excepted), `base-uri 'none'`, `form-action 'self'`;
- `Cross-Origin-Opener-Policy: same-origin`, and
  `Cross-Origin-Resource-Policy: same-origin` (`cross-origin` for status pages
  and their badges, which are made to be embedded);
- `Permissions-Policy` denying camera, microphone, geolocation, payment, USB,
  serial and the rest; only full screen, screen wake lock (wall mode) and
  passkeys stay allowed, for the interface itself;
- `Referrer-Policy: same-origin`, `X-Content-Type-Options: nosniff`,
  `X-Frame-Options: DENY` where framing is refused,
  `X-Permitted-Cross-Domain-Policies: none`, `Origin-Agent-Cluster: ?1`;
- `Cache-Control: no-store` on API answers that do not set their own.

`style-src` keeps `'unsafe-inline'`: Svelte sets `style=` attributes, which no
nonce or hash can cover. Injected styles would still need an HTML injection
first, which the rest of the policy and the escaping prevent.

## What still travels in clear

**Settings › System › HTTPS and encryption** lists every connection the server
makes or accepts in clear or without certificate checks, with a link to where it
is changed. These settings were chosen and keep working; the list says where, and
why it matters:

| Area | Reported when |
|---|---|
| Web | HTTPS unavailable, the redirect is off, or the certificate is self-signed. |
| Storage | `DUMBMONIT_VM_URL` or `DUMBMONIT_VM_LONG_URL` is `http://` to another host. |
| Sign-in | The OIDC issuer or the SAML sign-in page is `http://`; LDAP is `ldap://` without StartTLS, or its certificate is not verified. |
| Monitors | TLS verification is off (`insecure_tls`); SNMP v1/v2c, or SNMP v3 without privacy; credentials sent to an `http://` API. |
| Notifications | SMTP with `security: none`; a webhook or service URL in `http://`. |
| Logs | Syslog over UDP/TCP, GELF without TLS, or an `http://` Loki, Elasticsearch or VictoriaLogs; TLS verification off. |
| Agents | The plain agent channel is still accepted; unsigned updates are accepted. |
| Backups | Local backups in a custom folder hold the database and `secret.key` side by side. |

Loopback destinations (`http://127.0.0.1`, `localhost`) never leave the host and
are not reported.

Defaults are already strict where it costs nothing: SMTP requires STARTTLS
unless `none` is chosen, LDAP refuses `ldap://` without StartTLS unless
`DUMBMONIT_LDAP_ALLOW_PLAIN` is set, OIDC refuses `http://` unless
`DUMBMONIT_OIDC_ALLOW_HTTP` is set, the SAML sign-in URL must be `https://`
(loopback excepted), and every collector verifies TLS certificates unless told
otherwise.

## Settings

| Variable | Default | Effect |
|---|---|---|
| `DUMBMONIT_TLS` | `on` | `off`: the web port serves plain HTTP only. |
| `DUMBMONIT_TLS_CERT` | *(unset)* | PEM certificate chain to serve. |
| `DUMBMONIT_TLS_KEY` | *(unset)* | PEM private key of that certificate. Set both, or neither. |
| `DUMBMONIT_TLS_NAMES` | *(empty)* | Extra names and addresses of the self-signed certificate, comma-separated. |
| `DUMBMONIT_HTTPS_REDIRECT` | *(setting)* | `on`/`off` forces the redirect of plain-HTTP navigations; unset, the setting decides (on for new installations). |
| `DUMBMONIT_TRUSTED_PROXIES` | *(empty)* | Reverse proxies whose forwarded headers count (cookies, HSTS, client address). |
