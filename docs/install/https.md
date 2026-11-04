# HTTPS and encryption

DumbMonit encrypts the web interface and its API itself, on the port it already
uses (`8080` by default). There is nothing to configure: the web port only
speaks HTTPS. A plain HTTP request to it receives a `308` redirect to
`https://` on the same host and port, and nothing else.

| | |
|---|---|
| Protocols | TLS 1.3, and TLS 1.2 with ECDHE + AEAD suites only (AES-GCM, ChaCha20-Poly1305). No CBC, no RSA key exchange, no 0-RTT. |
| Key exchange | Hybrid post-quantum **X25519MLKEM768** first, then X25519 and P-256. |
| Certificate | A self-signed one created at first start, or your own (`DUMBMONIT_TLS_CERT` / `DUMBMONIT_TLS_KEY`). |
| HSTS | `Strict-Transport-Security: max-age=31536000` on every HTTPS response. |
| Session cookie | `__Host-dumbmonit_session`, `Secure`, `HttpOnly`, `SameSite=Strict`. |

The agents have their own encrypted channel on port 8443 (TLS 1.3 only, client
certificates): see [Agent channel security](agent-security.md).

## Out of the box

Nothing to do: at first start, DumbMonit creates a self-signed certificate (ECDSA
P-256, valid 397 days, renewed 30 days before it expires with the same key). Its
private key is stored encrypted with the instance key, like every other secret.
The log prints its fingerprint:

```
INFO HTTPS available on the web port (TLS 1.3/1.2, X25519MLKEM768) source=SelfSigned fingerprint=3A:9F:…
```

Open `https://<server>:8080`. Everything is encrypted, including the first
sign-in.

### The browser warning

At the first visit, the browser warns that it does not know the certificate: no
authority signed it, since it was made on your server. To lift the warning
safely:

1. Compare the fingerprint the browser shows (certificate details) with the one
   in the log or in **Settings › System › HTTPS and encryption**.
2. If they match, accept the exception, or download the certificate (**Download
   the certificate (PEM)** in the settings) and add it to your workstation's
   trusted certificates: the warning is gone for good.

The certificate covers `localhost`, the loopback addresses, the host name of the
container (when you set one with `hostname:`) and the host of
`DUMBMONIT_PUBLIC_URL`. Add the names and addresses your browsers use with
`DUMBMONIT_TLS_NAMES` (for example `nas.lan,192.168.1.10`): the certificate is
then re-issued with the same key.

## Behind a reverse proxy

To get a certificate that browsers trust without any warning (Let's Encrypt, for
example), put a reverse proxy in front of DumbMonit. DumbMonit does not obtain
certificates itself. The proxy holds the public certificate and must **connect
to DumbMonit over HTTPS**: a proxy that speaks plain HTTP upstream only receives
the redirect. On that hop the certificate is DumbMonit's own, so either disable
verification for this one upstream, or trust the certificate downloaded from the
settings.

=== "Nginx Proxy Manager"

    On the proxy host, set **Scheme** to `https`, **Forward Hostname** to
    `dumbmonit`, **Forward Port** to `8080`. NPM does not verify the upstream
    certificate, so the self-signed one is accepted.

=== "Traefik"

    ```yaml
    labels:
      - traefik.http.routers.monit.rule=Host(`monit.example.com`)
      - traefik.http.routers.monit.tls.certresolver=letsencrypt
      - traefik.http.services.monit.loadbalancer.server.port=8080
      - traefik.http.services.monit.loadbalancer.server.scheme=https
      - traefik.http.services.monit.loadbalancer.serverstransport=insecure@file
    ```

    `insecure` is a `serversTransport` with `insecureSkipVerify: true`, declared
    in the dynamic configuration.

=== "Caddy"

    ```
    monit.example.com {
        reverse_proxy https://dumbmonit:8080 {
            transport http {
                tls_insecure_skip_verify
            }
        }
    }
    ```

Two more things make it right:

1. **Forward the scheme and the client.** A request carrying `X-Forwarded-Proto`,
   `X-Forwarded-For`, `X-Forwarded-Host`, `X-Real-IP` or `Forwarded` comes from a
   proxy that decided already. Caddy, Traefik and Nginx Proxy Manager send these
   headers by default; with plain nginx, add
   `proxy_set_header X-Forwarded-Proto $scheme;` and `proxy_ssl_verify off;` (see
   [Docker › Reverse proxy](docker.md#reverse-proxy)).
2. **Declare the proxy as trusted** with `DUMBMONIT_TRUSTED_PROXIES` (its
   address or network): its `X-Forwarded-*` headers then count for the client
   address and HSTS. A forwarded header from any other address is ignored.

## Install commands

The commands shown when you add an agent are `https://`. The command displayed
in the interface contains the fingerprint of the server's certificate: the
script and the binary are downloaded over HTTPS and checked against that
fingerprint, so the self-signed certificate is verified without being trusted by
any authority. Behind a reverse proxy with a public certificate, the command
uses the public address and the usual verification.

In every case the agent itself then talks to the server over the pinned,
mutually authenticated agent channel.

## Your own certificate

If you do not use a reverse proxy, mount the certificate chain and its key, and
point DumbMonit at them:

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
- Both files are re-read within a minute when they change, or right away on
  `docker kill -s HUP dumbmonit`.
- An unreadable certificate at startup stops the server with a clear message; an
  unreadable renewal keeps the previous certificate and says so in the log and
  in the settings.

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
| Web | The certificate is self-signed. |
| Storage | `DUMBMONIT_VM_URL` or `DUMBMONIT_VM_LONG_URL` is `http://` to another host. |
| Sign-in | The OIDC issuer or the SAML sign-in page is `http://`; LDAP is `ldap://` without StartTLS, or its certificate is not verified. |
| Monitors | TLS verification is off (`insecure_tls`); SNMP v1/v2c, or SNMP v3 without privacy; credentials sent to an `http://` API. |
| Notifications | SMTP with `security: none`; a webhook or service URL in `http://`. |
| Logs | Syslog over UDP/TCP, GELF without TLS, or an `http://` Loki, Elasticsearch or VictoriaLogs; TLS verification off. |
| Agents | Unsigned updates are accepted. |
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
| `DUMBMONIT_TLS_CERT` | *(unset)* | PEM certificate chain to serve. |
| `DUMBMONIT_TLS_KEY` | *(unset)* | PEM private key of that certificate. Set both, or neither. |
| `DUMBMONIT_TLS_NAMES` | *(empty)* | Extra names and addresses of the self-signed certificate, comma-separated. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(empty)* | Reverse proxies whose forwarded headers count (client address, HSTS). |

Which ports to open, and which never to expose: see [Network ports](ports.md).
