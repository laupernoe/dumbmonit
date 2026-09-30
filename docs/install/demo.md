# Run a demo instance

The public demo at **[demo.dumbmonit.app](https://demo.dumbmonit.app)** is an
ordinary DumbMonit image started with `DUMBMONIT_DEMO=1`. You can run the same
thing yourself — to show DumbMonit to a team before installing it for real, or
to put a showcase on your own site.

!!! warning "Never on a real data volume"
    Demo mode **recreates the database at every start**. Give it a volume of its
    own, or none at all: pointed at the `/data` of a real instance, it would
    replace your devices, rules and history with the fictional estate.

## What demo mode does

- **Sign in with `demo` / `demo`.** The login page says so and fills the fields
  in one click. The account sees every screen an administrator sees.
- **Read-only.** Every change — devices, rules, silences, acknowledgements,
  channels, tokens, users, settings, backups and restores, discovery scans,
  agent enrollment, write tools of the assistant endpoint — is refused by the
  server with `403` and `{"error": "This is a read-only demo…", "demo": true}`.
  Signing in and out are the only writes allowed. The interface shows the
  message where the click happened.
- **A fixed, fictional estate.** At startup the database is created fresh and
  seeded with a small homelab under `*.home.arpa`: a Proxmox VE cluster, a
  Synology NAS, a Proxmox Backup Server, a TrueNAS, an OPNsense firewall, a
  Redfish server with a failing part, a few HTTP, TLS and ping checks, an agent
  host and a status page. The devices are answered by fake responders inside
  the server, bound to `127.0.0.1` and replaying recorded API responses, so the
  real collectors and device panels run. Restarting gives the same estate.
- **Seven days of history.** Synthetic metrics are written into
  VictoriaMetrics at startup, so charts, *Last 7 days*, forecasts and alert
  history are populated, and a few alerts are firing so *Needs you* has
  something in it.
- **Nothing leaves the server.** No notification is ever sent, and no probe
  goes to an address a visitor typed.
- **A guided tour** opens after the first sign-in; the banner reopens it.

Rate limiting of the sign-in works as usual. Put the demo behind your usual
reverse proxy with HTTPS and set `DUMBMONIT_COOKIE_SECURE=1` and
`DUMBMONIT_TRUSTED_PROXIES` as for any public instance.

## Start it

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/noekan/dumbmonit:latest
```

Or with Compose, without a volume so every restart starts clean:

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/noekan/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

Pull the image regularly (or let a tool such as Watchtower do it): the demo
then always shows the latest release with the same data.
