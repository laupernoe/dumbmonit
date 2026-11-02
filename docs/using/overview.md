# The overview page

The home page answers "is everything fine?" in about one second, then lets you
drill down on demand.

## The status banner {#the-bulletin}

A band at the top shows a fixed night sky: twinkling stars, a crescent moon and a
pigeon that crosses it without ever landing. The sky does not change with the
weather; the sentence at its foot says how things are, in words. "All 146
monitors up." when nothing needs you, "pve-02 is down." or "pve-02 needs a look."
otherwise, followed in a quieter voice by what the system did about it ("3 guests
held back, alert sent."). Tap the pigeon and it rolls. Under
`prefers-reduced-motion`, the pigeon stays out of the sky and the stars hold
still.

## The first-run guide {#the-first-run-guide}

On a new instance, the page under the banner is not empty: it is a three-step
guide — add a monitor, connect a way to be told, check a message arrives. Each
step is one click, each can be waved off on its own, and **Skip the guide** puts
all three away. The choice is stored on the instance, so a second browser does
not start the guide over. See
[Add your first monitor](../install/first-device.md#the-three-first-steps).

## The four figures {#the-figures}

Under the banner, four figures: **Monitors up** (out of the total, with the down
and paused counts), **Availability 30 d**, **p95 response** and **Open alerts**
(critical and warning). The time range in the header drives the p95 and the
river.

## What needs attention {#what-needs-attention}

The list beside the river holds only what is broken now, worst first, each row
with its quick action: acknowledge or snooze an alert, or mute a monitor (the
small cross). With nothing broken, the pigeon sleeps and the card says so. At
its foot, the **week ahead** in one line: what expires in the next seven days.
**See all** opens [Alerts](alerts.md).

## The health river {#the-health-river}

One strip per monitor, one cell per slot of the range, the worst monitors on
top. The Overview shows the first eight; **See all monitors** opens the
[List](devices.md) tab of Monitors. A state is always written next to a shape
(circle up, diamond degraded, square down), never shown by colour alone. On a
phone the river opens behind a tap.

## Two charts {#charts}

Further down: the **response time** of the whole fleet over the range (p95 and
median, with the slowest monitors) and the **alerts of the last seven days**
(fired and resolved).

## Wall mode {#wall-mode}

`/wall` shows a board for a monitor in the room: the verdict, the clock, the five
figures, the health river, the open alerts and who is on call. Open it with the
**Wall mode** icon at the bottom of the left menu or the command palette; ++esc++ or **Exit**
returns to the console. See [Wall mode](wall.md).

## Install it on a phone {#install-it-on-a-phone}

The interface ships a web manifest, so a phone can add it to the home screen:
**Share → Add to Home Screen** on iOS, **Install app** in the Chrome menu on
Android. It then opens without the browser chrome, in the theme the phone is
set to.

There is deliberately **no offline mode and no service worker**. A monitoring
screen that shows a cached page is worse than one that says it cannot reach the
server: the whole point is that what you read is current. With no worker, an
upgraded server is also served straight away, with no stale build to clear.

## Command palette {#command-palette}

++cmd+k++ on macOS, ++ctrl+k++ elsewhere, opens the command palette: one
input, results in three groups.

| Group | Entries |
|---|---|
| Pages | Overview, Monitors, Map, Alerts, Automation, Notifications, Metrics explorer, Security, Status pages, Reports, Logs, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Every monitor, with its state LED; ++enter++ opens it |

## Navigation {#navigation}

A slim **menu** on the left lists four destinations: **Overview**, **Monitors**,
**Alerts** (with the count of what needs you) and **Status**. At its foot sit
**Settings**, **Wall mode**, the documentation, GitHub and your account. What
used to be pages of their own are now tabs of these entries. The header shows
where you are, the **LIVE** switch that pauses the console's own refreshes, the
time range, the palette and **New monitor**. On a phone, the menu becomes a
bottom tab bar with the essentials and **More**.

- **Overview**: is everything fine, and what needs you.
- **Monitors**: every source you monitor, in four tabs: **List**, **Map**,
  **Logs** and **Security**. See [Monitors](../monitors/index.md).
- **Alerts**: five tabs: **Now**, **History**, **Rules**, **Automation** and
  **On-call**. See [Alerts page](alerts.md).
- **Status**: **Pages** (public status pages) and **Reports**. See
  [Status](../status/index.md).
- **Settings**: your account, team and access, monitoring, notifications, the
  wall and the system. See [Settings](../settings/index.md).

The interface is dark, and nothing on it conveys status by colour alone: a
marker always comes with a word (Reporting, Unreachable, Advisory, Warning,
Suppressed by parent, Building up).
