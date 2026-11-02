# Sentinel mode

When the DumbMonit server is down, nothing inside it can tell you. The
[outgoing heartbeat](self-monitoring.md) covers that with an outside service.
Sentinel mode covers it from the inside of your own network: an agent you
designate checks that the server (and, optionally, a second exposed address)
still answers, and **sends the alert itself** when it does not, without going
through the server.

The agent never accepts a connection. It probes outward and notifies outward.

## What the sentinel does

- Every *interval* (15 to 3600 seconds, 60 by default) it requests each address
  you listed, 5 at most. Any HTTP answer below 500 counts as alive, including
  a login page or a `404`; a timeout, a connection or TLS failure, or a `5xx`
  counts as a failure.
- When **one address** fails *threshold* times in a row (1 to 20, 3 by default),
  it sends one notification. While the failure lasts it sends a reminder every
  hour, then a message when everything answers again, with the length of the
  outage.
- It never sends more than 6 notifications per hour. A link that flaps cannot
  flood your channel; a held-back message goes out when the window frees.
- A notification that could not be delivered is retried at the next round.

## Setting it up

1. Open the page of an enrolled agent machine, section **Sentinel**
   (administrators only).
2. Turn **Watch DumbMonit from this machine** on.
3. List the addresses, one `https://` address per line. Plain `http://`,
   addresses with a login in them and anything else are refused.
4. Choose the **notification channel**, a single one:
    - a **generic webhook**: an `https://` address that receives a JSON `POST`
      with `source`, `event` (`down`, `reminder` or `recovered`), `title`,
      `message`, `host` and `urls`;
    - an **ntfy topic**: a plain-text `POST` on `https://<server>/<topic>`,
      with a title, a priority and a tag.
5. Save. The agent applies the setting at its next report.

### Which addresses

The probe uses the same TLS client as the agent's own channel: the agent's
client certificate, and the server's certificate authority pinned. So the
addresses to watch are the ones served by that same authority, in practice the
**agent port** of your server (for example
`https://dumbmonit.example.org:8443/`) and, if you expose one, the relay
gateway of another site, which carries the same end-to-end channel. Any answer
below 500 from that port proves the server is up, whatever the path.

## The channel is a secret

A webhook address or an ntfy topic usually lets anyone who knows it post to
you. DumbMonit stores it encrypted with the instance secret
(`/data/secret.key`), never returns it through the API (only `has_secret` and
`channel_kind`), and includes it in the encrypted configuration backup.
Leaving the field empty when you save keeps the stored channel; use **Remove
the saved channel** to delete it.

The channel is sent to **the designated agent only**, and **only over the
encrypted agent channel**. An agent still on the older plain channel receives
the addresses but not the channel, and does not start. It receives nothing
else: none of the server's other secrets.

## Limits of this first version

- One channel, no escalation, no schedule.
- The agent learns its setting from the server. If the agent restarts while the
  server is down, it cannot start the sentinel until the server is back. An
  agent that was already running keeps watching through the outage.
- If the sentinel machine itself loses its network, every address fails, but
  the notification cannot leave either. Place the sentinel on a machine that
  does not depend on the same link as the server.
