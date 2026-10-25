# Third-party service status

The public status page of a provider you depend on (Cloudflare, GitHub,
Atlassian, AWS…), read as a monitor. It fails when the provider reports an
incident, so you hear about it before your users do, and a
[status page](../using/status-pages.md) can show the provider next to your own
services as an external component.

Two formats are read:

- the JSON of an Atlassian Statuspage, `/api/v2/summary.json`: most public
  status pages (Cloudflare, GitHub, Atlassian, Discord, DigitalOcean…);
- an RSS or Atom incident feed: the AWS Health feed
  (`https://status.aws.amazon.com/rss/all.rss`, or one feed per service and
  region) and the `history.rss` of a Statuspage.

Microsoft 365 is not covered: its service health is not public. It is only
available through the Microsoft Graph API (`serviceAnnouncement`) with an
application registered in your tenant. Any public RSS or Atom feed of incidents
can be given as the address instead.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | Status page of the provider (HTTPS) |

## What it watches

Besides the [common availability metrics](services.md) (`probe_success`,
`probe_duration_seconds`, `probe_failure_info`), all prefixed `dumbmonit_`:

| Metric | What |
|---|---|
| `probe_provider_impact` | 0 operational, 1 minor, 2 major, 3 critical |
| `probe_provider_incidents` | open (unresolved) incidents |
| `probe_provider_components_affected` | components that are not operational (Statuspage only) |

The monitor fails (`probe_success` at 0, reason `status`) when the impact reaches
**Fail on**: any degradation by default. A planned maintenance is not an
outage. The built-in "service down" alert therefore applies to a provider like
to any other service.

With a **Component** filter, only the components whose name contains the text
count (case-insensitive): "Workers", "Git Operations", "EC2". A filter that
matches nothing is shown as a configuration error, not as an outage.

In a feed, only the newest entry within **Feed window** (24 hours by default)
counts: while it is not marked "resolved" or "operating normally", an incident
is open, and a "disruption" or "outage" in its text counts as major. Narrow a
feed that mixes services with Component.

If the status page itself cannot be read, the last answer is used for up to 15
minutes; after that the monitor fails with the reason `connect`, `timeout`,
`status` or `body`.

The answer is shared: several monitors, or several status pages, on the same
provider cost one request per **Refresh interval** (60 seconds by default). The
address cannot point at the DumbMonit host itself unless "Allow loopback and
link-local targets" is ticked.

## On a status page

A `provider` monitor added to a [status page](../using/status-pages.md) appears
as a normal service tagged "External", with its own uptime history.

## Watch a provider's status page

1. Enter a provider name from the list (cloudflare, github, atlassian, discord, dropbox, npm, digitalocean, twilio, zoom, vercel, netlify, datadog, aws), or the address of any Atlassian Statuspage page or RSS/Atom incident feed, for example https://www.cloudflarestatus.com.

2. To watch one part of the provider only, enter its name under Component, for example "Workers" or "EC2".

3. Choose what makes the monitor fail: any degradation, a partial outage or worse, or a major outage only.

!!! warning
    The monitor reads what the provider chooses to publish, often minutes after the fact: it complements your own checks and never replaces them.

No credential is needed: these pages are public.

## Credentials

None.

Address: a name from the list, the address of a Statuspage page
(`https://www.githubstatus.com`) or of a feed. Options: Component, Fail on
(minor, major, critical), Refresh interval (60 s, from 30 to 3600), Feed window
(24 h, from 1 to 168) and the usual timeout (5 s).
