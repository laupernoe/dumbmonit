# Plex Media Server

What Plex is doing and whether it is current: streams playing now, how many
are transcoded and whether in hardware, the bandwidth they reserve, streams
from outside the house, libraries being scanned, and updates.

DumbMonit reads the server's own API on port 32400, never plex.tv. It never
stops a stream, never scans a library and never changes a setting.

Plex does not publish the outcome of its maintenance tasks (the "butler" only
gives their schedule) nor the state of its database backups, so there is
nothing to say about those.

## What it watches

All metrics are prefixed `dumbmonit_plex_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `update_available` | 1 when Plex's own update check found a newer release; absent until Plex has checked once | `latest_version` |
| `claimed` | 1 when the server is linked to a Plex account | |
| `streams`, `streams_paused`, `streams_remote` | playbacks now; paused; from outside the local network | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | playbacks re-encoded (video or audio); with the video re-encoded; with hardware acceleration | |
| `transcoder_active_video_sessions` | video transcodes the server itself counts | |
| `stream_bandwidth_bits_per_second` | bandwidth reserved by all playbacks | |
| `libraries`, `libraries_scanning` | libraries; those being scanned now | |
| `activities` | background activities running (scans, analysis, updates) | |
| `scrape_errors` | calls that failed on the last probe | |

The [built-in rule](../alerting/rules.md#self-hosted-applications) that applies:
Plex update available, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
calls Plex. It gives the update state and whether the server is claimed, then
streams, transcodes, hardware transcodes, remote streams, bandwidth and
libraries.

## Give DumbMonit access to the Plex server

1. DumbMonit reads the server's own API on port 32400, never plex.tv. A server linked to a Plex account asks for a token (X-Plex-Token), and Plex cannot issue a limited one: it is the owner's token.

2. Either avoid the token: in Plex, Settings → Network → List of IP addresses and networks that are allowed without auth, add the DumbMonit server's address, for example as follows, and pick No token below.

    ```
    10.0.0.20/32
    ```

3. Or use the token: in Plex Web, open any movie → ⋯ → Get Info → View XML. The address of the page that opens ends with X-Plex-Token= followed by the token; copy that value.

4. In DumbMonit, enter the server's address, for example "plex.lan" (port 32400). DumbMonit only reads sessions, libraries, the updater and the activity list; it never stops a stream and never changes a setting.

!!! warning
    The token is your Plex account's: whoever holds it can manage the server and every library. Prefer the allowed network above when DumbMonit sits on your own network.

The allowed network is also what makes Plex answer without a token from Docker:
DumbMonit's container reaches Plex from its Docker network address, so list
that network, or the host's address when Plex publishes its port on the host.

## Credentials

| Credential | Fields |
|---|---|
| X-Plex-Token | Sent in the `X-Plex-Token` header. |
| No token (allowed network) | Nothing is sent; Plex must list DumbMonit's address among the networks allowed without authentication. |

Address: a host name (`plex.lan`), `host:port`, or a full URL; a trailing `/web`
is accepted. Options: protocol (HTTP by default; Plex's own certificates are
issued for `*.plex.direct` names, so HTTPS by IP address needs the certificate
check turned off), port (32400), certificate check and request timeout (15 s).

Validated against Plex Media Server 1.43.4 (official image), an unclaimed server
answering without a token, with a real transcoded playback. The token path
follows Plex's documented `X-Plex-Token` header and was not exercised against a
claimed server.
