# Network ports

Which port does what, who must reach it, and can it face the Internet?

The short answer: **keep everything on your local network, except TCP 8443 if
you monitor remote sites.** DumbMonit only serves HTTPS.

## At a glance

| Port | Protocol | Role | Who must reach it | Exposure |
|------|----------|------|-------------------|----------|
| **8080** | TCP (HTTPS) | Web interface, API, install scripts, public status pages | Your browsers; agents on the same network | LAN only. WAN only for public status pages, behind a reverse proxy. |
| **8443** | TCP (TLS 1.3, client certificates) | Agent channel | Agents, including those on other sites | LAN, or WAN if restricted to the agents' IPs. |
| **5514** | UDP and TCP | Syslog input (off by default) | Devices that send logs | LAN only. Never WAN. |
| **1162** | UDP | SNMP trap input (off by default) | Devices that send traps | LAN only. Never WAN. |
| 8428, 8429, 9428 | TCP | Bundled storage (metrics, long-term metrics, logs) | Nobody but DumbMonit itself | Never published. They listen on `127.0.0.1`. |

Only 8080 and 8443 are published by the default `docker-compose.yml`. The
syslog and trap ports are commented out until you need them.

## Port by port

### 8080: web interface

This is where you open DumbMonit. It also serves the agent install scripts and
the public status pages (`/s/<slug>`). Change it with `DUMBMONIT_BIND`.

- **Closed:** you cannot reach the interface. Agents still report on 8443.
- **Open on the Internet:** the login page becomes visible to scanners and
  password guessers. Keep two-factor sign-in on.
- **Safer:** put a reverse proxy in front (see the third scenario), or reach
  the interface through a VPN such as Tailscale or WireGuard. Allow only your
  own IPs on the proxy.

### 8443: agent channel

Agents connect here to push their measurements. The channel is TLS 1.3 only,
and each agent proves who it is with its own certificate. It is built to face
the Internet, but it still deserves a source filter. Change it with
`DUMBMONIT_AGENT_LISTEN` (`off` closes it) and
`DUMBMONIT_AGENT_PUBLIC_PORT` if a port forward publishes another number.

- **Closed:** agents on other networks cannot report. Agents on the same
  network as the server can still use the web port.
- **Open on the Internet:** anyone can try a TLS handshake, but without an
  agent certificate they get nothing. Restrict it to your agents' IPs anyway.
- **Relay gateway:** a relay agent opens its own gateway, also on 8443 by
  default. Open it on the relay's firewall to the site's network only.

See [Agent channel security](agent-security.md).

### 5514: syslog

Off by default. Turn it on with `DUMBMONIT_SYSLOG_ENABLED`. The port is
`DUMBMONIT_SYSLOG_PORT`, the address `DUMBMONIT_SYSLOG_BIND`. The standard port
514 is privileged, so DumbMonit uses 5514.

- **Closed:** devices cannot send logs. Nothing else is affected.
- **Open on the Internet:** syslog has no login and no encryption. Anyone can
  fill your log store with fake lines.
- **Safer:** LAN only. For a remote site, forward the logs through an agent or
  a VPN, not through a public port.

See [Server logs](logs.md).

### 1162: SNMP traps

Off by default. Turn it on with `DUMBMONIT_TRAPS_ENABLED`. The port is
`DUMBMONIT_TRAPS_PORT`, the address `DUMBMONIT_TRAPS_BIND`. The standard port
162 is privileged, so DumbMonit uses 1162. Point the device at 1162, or add a
redirect from 162.

- **Closed:** traps are lost. Polling still works.
- **Open on the Internet:** v1 and v2c traps only carry a community string, in
  clear text. Anyone can forge a trap. Set `DUMBMONIT_TRAPS_COMMUNITIES` to
  accept only your communities, and keep the port on the LAN.

### 8428, 8429, 9428: bundled storage

VictoriaMetrics and the log store run inside the container and listen on
`127.0.0.1`. Do not publish them. If you point DumbMonit to an external
VictoriaMetrics (`DUMBMONIT_VM_URL`), those ports are yours to protect.

## Ready-made scenarios

### 1. Everything on the local network

Publish what you need and block the Internet at your router. Do not forward
any port.

```yaml
    ports:
      - "8080:8080"
      - "8443:8443"
```

Make sure your router has no port forward to the server. That is all.

### 2. Agents on remote sites

Only 8443 must be reachable. Keep 8080 private.

On the router or the cloud firewall, forward TCP 8443 to the server. Then
allow it **only from the IPs of your sites**. Replace `203.0.113.10` and
`198.51.100.20` with the public addresses of your sites (see the firewall
examples below). If a site has a dynamic IP, use a VPN instead, or a
DNS-based rule on your firewall.

If the router publishes another number (for example 18443), set
`DUMBMONIT_AGENT_PUBLIC_PORT=18443`.

### 3. Public status pages behind a reverse proxy

Never publish 8080 directly. Let a reverse proxy own port 443 and forward only
the status pages.

```caddyfile
status.example.com {
    @public path /s/* /assets/* /_app/*
    reverse_proxy @public https://127.0.0.1:8080 {
        transport http {
            tls_insecure_skip_verify
        }
    }
    respond "Not found" 404
}
```

The proxy talks HTTPS to DumbMonit and trusts its self-signed certificate.
Add the proxy to `DUMBMONIT_TRUSTED_PROXIES` so the client address and cookies
are right (see [HTTPS and encryption](https.md)). The login page and the API
stay unreachable from the Internet. Check this from outside:
`https://status.example.com/login` must answer 404.

## Firewall examples

Allow the web port from the LAN only, and the agent port from two sites.
Replace `192.168.1.0/24` and the site addresses with yours.

**ufw**

```bash
sudo ufw allow from 192.168.1.0/24 to any port 8080 proto tcp
sudo ufw allow from 203.0.113.10 to any port 8443 proto tcp
sudo ufw allow from 198.51.100.20 to any port 8443 proto tcp
sudo ufw allow from 192.168.1.0/24 to any port 8443 proto tcp
```

**firewalld**

```bash
sudo firewall-cmd --permanent --add-rich-rule='rule family="ipv4" source address="192.168.1.0/24" port port="8080" protocol="tcp" accept'
sudo firewall-cmd --permanent --add-rich-rule='rule family="ipv4" source address="203.0.113.10" port port="8443" protocol="tcp" accept'
sudo firewall-cmd --permanent --add-rich-rule='rule family="ipv4" source address="198.51.100.20" port port="8443" protocol="tcp" accept'
sudo firewall-cmd --reload
```

**nftables**

```nft
table inet filter {
  set agent_sites { type ipv4_addr; elements = { 203.0.113.10, 198.51.100.20 } }
  chain input {
    type filter hook input priority 0; policy drop;
    ct state established,related accept
    iif lo accept
    ip saddr 192.168.1.0/24 tcp dport { 8080, 8443 } accept
    ip saddr @agent_sites tcp dport 8443 accept
  }
}
```

Docker publishes ports through its own firewall rules, which ufw and firewalld
do not always see. Test from another machine after every change. To be sure,
publish on one address only: `"192.168.1.5:8080:8080"`. For rules that apply to
published container ports, put them in the `DOCKER-USER` chain (iptables) or a
forward chain (nftables).

## Before you open anything to the Internet

- [ ] Do you really need it? A VPN (Tailscale, WireGuard) is often simpler.
- [ ] Open only 8443, unless you publish status pages through a reverse proxy.
- [ ] Restrict 8443 to the IPs of your agents.
- [ ] Never open 5514, 1162, 8428, 8429 or 9428.
- [ ] Put the web interface behind a reverse proxy, or a VPN, never raw.
- [ ] Use a real certificate on the public name ([HTTPS and encryption](https.md)).
- [ ] Every account has a strong password and two-factor sign-in.
- [ ] You changed the default admin password and removed unused accounts.
- [ ] Test from outside (a phone on mobile data): only the ports you chose answer.
- [ ] Keep DumbMonit and its agents up to date ([Update](update.md)).
