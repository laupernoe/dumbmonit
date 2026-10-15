# OpenVPN server

OpenVPN's server process exposes its own control channel, the **management
interface**: a local TCP listener, separate from the VPN itself, made for a
monitoring tool rather than for a VPN client. DumbMonit reads it like any
management client: the server's version, whether it reports itself connected,
and every client currently in session, with its real and virtual address and
its traffic. Nothing is ever sent that would change the server's state or
disconnect a client.

Clients are transient by nature — a laptop that goes to sleep simply
disappears from the next reading. DumbMonit does not try to remember who was
connected five minutes ago; it reports only who is connected now, plus the
client count as a metric worth graphing over time.

## What it watches

All metrics are prefixed `dumbmonit_openvpn_`.

| Metric | What | Labels |
|---|---|---|
| `server_info` | value 1 | `version` |
| `server_up` | 1 when the management interface reports the server `CONNECTED`, 0 otherwise | `state` |
| `server_state_age_seconds` | time since the server entered its current state | |
| `clients_connected` | number of clients currently in session | |
| `client_bytes_received`, `client_bytes_sent` | traffic of one client (counters) | `common_name`, `real_address` |
| `client_connected_seconds` | time since that client's session started | `common_name`, `real_address` |

`real_address` never carries the client's source port: it changes at every
reconnection behind NAT and would multiply the series for nothing. At most 256
clients get a detailed series each; `clients_connected` stays exact beyond
that.

Built-in rules that apply: **Monitor unreachable**, when the management
connection itself cannot be made or the password is refused.

## The monitor page

The panel above the charts reads what the probe stored; opening the page
never connects to the management interface. It shows the server's version and
whether it reports itself connected, the number of clients, and a list of the
clients currently in session with their address and traffic.

## Turn on the management interface

1. The management interface is a separate listener, off by default: add this to the server's configuration (server.conf, or the OpenVPN Access Server's as.conf under a different name) and restart it. Bind it to an address the DumbMonit host can reach, or to 127.0.0.1 and reach it through an SSH tunnel or a relay agent on the same machine.

    ```
    management 0.0.0.0 7505
    ```

2. Optional but recommended: the management interface has no account of its own, only a shared password. Put one in a file readable by the OpenVPN process alone and add it as a third argument.

    ```
    management 0.0.0.0 7505 /etc/openvpn/management-password
    ```

3. In DumbMonit, enter the server's address, for example "vpn.lan" (port 7505), and the management password if one is set.

4. DumbMonit only reads: it sends version, state and status 3, never signal, kill or client-kill. It cannot disconnect a client, change a setting or stop the server.

!!! warning
    Anyone who reaches the management port and knows the password can disconnect clients, read all of their real IP addresses, and reconfigure the running server: keep it off the public network, behind a firewall rule that allows only the DumbMonit host, with a password set.

## Credentials

| Credential | Fields |
|---|---|
| No password | Only for a management interface started without a password-file argument. |
| Management password | The shared password set by the third argument of the server's "management" directive. |

Address: a host name or IP, optionally with `:port`; the default port is
7505, a convention from OpenVPN's own documentation, not a standard the
server enforces. Options: port (7505) and the timeout per exchange (10 s).

A refused connection or no answer counts as the server being unreachable. A
wrong password is shown as a credential error; neither wakes anyone at
night. The connection is plain TCP: nothing read is secret, but the password,
if one is set, crosses the network in clear at every check — reach the
interface only from a trusted network or through a relay agent on the same
machine as the server.
