# Alerts page

![The Alerts page: what needs you, the rules, scheduled maintenance and history](../assets/screenshots/alerts-light.png){ loading=lazy }

Everything about alerting on one page, behind five tabs. The tab is in the
URL (`/alerts#rules`), so a view is linkable. **Schedule maintenance**, top
right, opens the form for a window from any tab.

| Tab | What it holds |
|---|---|
| **Now** | The live *Needs you* list, grouped by device: severity plate, reason, since when, and *Ack*, *Snooze*, *Ignore* and *Open device* on each. Acknowledged and snoozed alerts each sit in their own quiet group at the bottom. The badge on the tab, and on Alerts in the top bar, is the count of what still needs you. |
| **Scheduled** | Maintenance windows, *Active now* or *Scheduled*, one-off, weekly or monthly, each with the next occurrence. See [Maintenance windows](../alerting/maintenance.md). |
| **Rules** | Every rule with its severity, a *Built-in* mark and, if any device ignores it, *Ignored on N devices*; enable, edit inline, delete your own, create a threshold rule. See [Rules](../alerting/rules.md). |
| **Notifications** | Where alerts reach you: the channels and the notification policy. Details below. |
| **History** | The last 200 transitions, each naming the device, the rule and what happened; resolved ones can be cleared. |

The same truth model feeds the overview bulletin and this page, so the two
always agree on what needs you.

## Getting rid of an alert

A firing alert is not binary — "fix it" or "live with the noise forever".
Four actions, for four different situations, open to admins and operators
(a viewer sees the state but cannot change it):

- **Acknowledge** ("I'm on it") is for one alert you are actively working on:
  "I know, stop reminding me for 4 h". The menu offers 1 h, 4 h, 24 h or
  *until resolved*, plus an optional note for whoever reads the card after
  you. The alert keeps firing and being evaluated; only its reminders and
  escalations pause — including the escalation to a second channel. You are
  still told when it resolves, and the acknowledgement clears at that moment —
  an alert that comes back later notifies again. The card moves out of the
  main list into a quieter *Acknowledged* group, reads *Acked by someone
  until a time*, and offers **Un-ack**.
- **Snooze** is for "not now, check back later" without claiming you are
  doing anything about it: 1 h, 8 h, 1 day or *until resolved*. Unlike Ack,
  it works by opening a maintenance window scoped to that one alert's exact
  labels — a sibling rule on the same device keeps talking. The card moves to
  a quiet *Snoozed* group showing the time left, and offers **Unsnooze**.
  Reminders pause the same way an acknowledgement's do.
- **Ignore** ("don't alert me about this again") is for a rule that will
  never make sense on this device: it disables that rule for that device
  alone (a per-device override — see
  [Per-device overrides](../alerting/rules.md#per-device-overrides)) and the
  alert disappears for good, immediately, not just quietened. It is
  reversible: **Stop ignoring** on the device page, or from the rule itself in
  Alerts → Rules, where it reads *Ignored on N devices*.
- **Clear** is for history, not for a live alert: once a transition has
  resolved, *Clear* (or *Clear all resolved*) drops it from the default
  History view. The row stays in the database — nothing is deleted — and
  *Show cleared* brings it back.

Public status pages ignore acknowledgements and snoozes: an acked or snoozed
alert is still an alert for the outside world. An ignored rule, on the other
hand, never produced an alert in the first place.

## Notifications

Channels and the policy live here — under Alerts, not Settings — because they
decide who hears an alert, which is an alerting concern. Two panels:

**Channels.** The list of channels, each with its kind, *Enabled* or
*Disabled*, when it last sent something, and its last error if any. **Add
channel** opens a form built from the server's description of the chosen kind:
settings (visible) and secrets (write-only). **Send test** sends a test message
and shows the result inline. Each channel's **Delivery options** hold its
minimum severity, whether it hears resolutions, a minimum interval per alert,
and its **quiet hours** (weekly, in your time zone: only Warning-level alerts
come through, the rest waits for a digest).

**Only some alerts.** The same Delivery options hold the channel's **routing
filter**: which alerts it wants, by device tag (`site=cellar`), by device kind
(`proxmox`), or by rule. Conditions on the same field read as *or*, different
fields as *and*, and an *Except* row always wins. Under the editor, a preview
lists which of your devices the filter selects right now, and one sentence
says the whole thing back to you — *this channel receives advisories and above
from devices tagged site=cellar, except devices tagged role=lab*. A channel
with no filter keeps receiving everything, which is what every existing channel
does.

Editing a channel and leaving the secret empty keeps the stored one. Channel
types and their fields are documented in
[Notification channels](../notifications.md).

Built-in alert rules notify every enabled channel; a rule can also be limited
to specific channels in its editor.

**Notification policy.** The global knobs that keep notifications few: the
batch window, the cap per channel per hour, flap detection under *More
options*, the public URL used for the "Open in DumbMonit" links, and the
**escalation** row — *if nobody acknowledges* within a delay, *also tell*
another channel, once. The delay counts from the moment the first message
actually went out, and acknowledging the alert (or its clearing) stops the
hop. See [Notification policy](../alerting/notifications.md).

Links: `/alerts#notifications` opens the tab, `/alerts#notifications-policy`
scrolls to the policy panel. The former Settings links
(`/settings#notifications`, `/settings#notifications-policy`) forward here.

## Operators and viewers

An **operator** handles alerts without touching the configuration: they
acknowledge, snooze, ignore a rule for one device, schedule or delete
maintenance windows and clear resolved history. Rules, channels and the
notification policy stay read-only for them, marked *Operator — config is
admin only*. Ignoring is the only per-device override an operator can make;
thresholds remain an admin's call, and an operator's *Ignore* or *Stop
ignoring* keeps any threshold an admin set for that device.

A viewer sees every tab but no control: the page shows *Viewer — read only*
where an admin would find the buttons.
