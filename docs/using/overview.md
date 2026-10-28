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

## The five figures and the health river {#the-figures}

Under the banner, five figures: **Monitors up** (out of the total, with down and
paused counts), **Availability 30 d**, **p95 response time**, **Open alerts**
(critical and warning) and **MTTR 30 d** (mean time to resolve). The time range
in the header drives the p95 and the river.

The **health river** is one strip per monitor, one cell per slot of the range,
the worst monitors on top. A state is always written next to a shape (circle up,
diamond degraded, square down), never shown by colour alone.

## The monitor table {#the-monitor-table}

The table lists every monitor, worst first. It filters by state, type and text,
lets you add columns (address, folder) and remove others, and is driven by the
keyboard: ++j++ / ++k++ move, ++enter++ selects, ++o++ opens.

The monitor you select feeds the **context panel** on the right, like an
inspector: what needs attention (with the alert actions, see
[Alerts page](alerts.md)), **Check now** and **Silence 1 h**, the nearest
forecasts and who is on call. ++i++ shows or hides it; on a narrower screen it
opens as a drawer. For a full list of monitors, see [Monitors page](devices.md).

## Wall mode {#wall-mode}

`/wall` shows a board for a monitor in the room: the verdict, the clock, the five
figures, the health river, the open alerts and who is on call. Open it with the
**Wall mode** icon in the top bar or the command palette; ++esc++ or **Exit**
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
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Every monitor, with its state LED; ++enter++ opens it |

## Navigation {#navigation}

A slim **icon rail** on the left lists every destination: Overview, Monitors,
Alerts (with the count of what needs you), Dashboards, Explore, the topology map,
Security, Logs, Status pages, Reports and Settings. At its foot sit the
documentation, GitHub and your account. The header shows where you are, the
**LIVE** switch that pauses the console's own refreshes, the time range, the
palette, **Wall mode** and **New monitor**. On a phone, the rail becomes a bottom
tab bar with the essentials and **More**.

- **Overview**: is everything fine, and what needs you.
- **Monitors**: every source you monitor. See [Monitors page](devices.md).
- **Alerts**: what is firing, maintenance windows, history and the rules.
  See [Alerts page](alerts.md).
- **Settings**: your account, users and single sign-on, tokens, notification
  channels, appearance, about. See [Settings](settings.md).

The interface is dark, and nothing on it conveys status by colour alone: a
marker always comes with a word (Reporting, Unreachable, Advisory, Warning,
Suppressed by parent, Building up).
