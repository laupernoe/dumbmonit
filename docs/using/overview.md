# The overview page

The home page answers "is everything fine?" in about one second, then lets you
drill down on demand.

![The overview](../assets/screenshots/overview-light.png){ loading=lazy }

## The bulletin

The band at the top is a weather bulletin for your network. One sentence states
the sky: "Clear skies." when nothing needs you, otherwise a count of what does
("9 advisories, 1 unreachable."). Under it, plates count monitors reporting and
unreachable, with the time of the last check, and three readouts: **Reporting**,
**Needs you**, **Forecasts**.

The weather window on the right follows the network: clear, cloudy, overcast or
storm, depending on how many alerts are firing and how severe they are, with the
pigeon flying across. Under `prefers-reduced-motion`, everything is still.

The single primary action of the page is **Add a monitor**.

## The first-run guide

On a new instance, the page under the bulletin is not empty: it is a three-step
guide — add a monitor, connect a way to be told, check a message arrives. Each
step is one click, each can be waved off on its own, and **Skip the guide** puts
all three away. The choice is stored on the instance, so a second browser does
not start the guide over. See
[Add your first monitor](../install/first-device.md#the-three-first-steps).

## Needs you

The list of what needs you now, most severe first: unreachable monitors, then
firing alerts by severity. Each row names the rule, the monitor, the current
value, how long it has been going on, and whether it groups several series
("×4" for four filesystems on the same monitor). Four actions: **Ack**
(acknowledge this alert for 1 h, 4 h, 24 h or until resolved — reminders
pause, the resolution is still notified), **Snooze** (hide just this alert
for 1 h, 8 h, 1 day or until resolved), **Ignore** (stop this rule from ever
alerting on this monitor again, reversibly), and **Open monitor**.

Acknowledged and snoozed alerts move to their own quieter group under the
list and leave the *Needs you* count. An ignored alert disappears outright —
it is not "known", it is turned off for that monitor. See
[Getting rid of an alert](alerts.md#getting-rid-of-an-alert).

## Monitors summary

One line: how many monitors, how many reporting, how many unreachable, and a
link to the full Monitors page. The overview never lists monitors: the rack lives
on [Monitors](devices.md).

## Forecasts and last 24 hours

**Forecasts** shows what is predicted rather than observed: the seasonal
baseline anomaly, the "full within four days" extrapolations, the PBS
fill-up estimate, and rules whose name announces a prediction. While a baseline
is learning, its entries say so.

**Last 24 hours** lists the firing transitions of the day, with a link to the
full history on the Alerts page.

## Wall mode

`/wall` shows the bulletin alone, full screen, for a room monitor: the
sentence, the counts and the *Needs you* list, refreshed every 20 seconds, with
the screen kept awake. ++esc++ or **Exit** returns to the overview. Open it from
the command palette ("Wall mode") or by typing the URL. It can also play music
started from your phone: see [Wall mode and music](wall.md).

![Wall mode](../assets/screenshots/wall-light.png){ loading=lazy }

## Install it on a phone

The interface ships a web manifest, so a phone can add it to the home screen:
**Share → Add to Home Screen** on iOS, **Install app** in the Chrome menu on
Android. It then opens without the browser chrome, in the theme the phone is
set to.

There is deliberately **no offline mode and no service worker**. A monitoring
screen that shows a cached page is worse than one that says it cannot reach the
server: the whole point is that what you read is current. With no worker, an
upgraded server is also served straight away, with no stale build to clear.

## Command palette

++cmd+k++ on macOS, ++ctrl+k++ elsewhere, opens the command palette: one
input, results in three groups.

| Group | Entries |
|---|---|
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident, Toggle theme |
| Monitors | Every monitor, with its state LED; ++enter++ opens it |

## Navigation

The top bar has Overview, Monitors, Alerts (with the count of what needs you)
and Settings, the palette shortcut and the theme toggle. On a phone, it
becomes a bottom tab bar with those four tabs and **More**, which holds wall
mode and the documentation.

- **Overview** — the bulletin: is everything fine, and what needs you.
- **Monitors** — the rack; every source you monitor. See [Monitors page](devices.md).
- **Alerts** — what is firing, maintenance windows, history and the rules.
  See [Alerts page](alerts.md).
- **Settings** — your account, users and single sign-on, agent and assistant
  tokens, notification channels and reports, public status pages, appearance,
  about. See [Settings](settings.md). Nothing on the interface conveys status by colour alone: a
plate or an LED always comes with a word (Reporting, Unreachable, Advisory,
Warning, Suppressed by parent, Building up).
