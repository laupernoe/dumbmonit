# On-call

On-call answers one question: when a critical alert fires at 3 a.m., who gets
paged, and who is paged next if that person does not react? It is built in,
free, and needs no extra service. You describe **who** (contacts), **when**
(schedules) and **what happens if nobody answers** (escalation policies), then
plug the result into the usual notification routing.

```
alert → routing (labels, severity, rule channels)
      → "DumbMonit on-call" channel → escalation policy
            step 1: whoever is on call on schedule X   → wait 10 min
            step 2: Alice + the #ops channel           → wait 15 min
            … repeated K times, until someone acknowledges
```

## 1. Your contacts and quiet hours

Every user declares how to reach them, in **Settings → My account → On-call
contacts**. A contact is one of two things:

- **Through a team channel.** Reuse a channel an admin already configured
  (email SMTP, SMS Twilio, Signal, Telegram) and give only your own address:
  your email, your phone number, your Telegram chat id. The server and its
  credentials stay the team's.
- **Your own service.** ntfy, Pushover, Telegram, Gotify, Bark, Pushbullet,
  Discord, Slack, Matrix… with your own settings and secrets, exactly as in a
  [notification channel](../notifications.md). Secrets are never shown again.

Each contact has a **minimum severity** (for example SMS only for critical,
ntfy for everything) and a **Test** button. A contact that fails shows its last
error.

**Quiet hours** (same screen, with your time zone) keep you from being woken for
small things: outside an on-call shift, only critical alerts get through during
those hours. While you are on call you are always paged, quiet hours or not.

**Calendar link.** *Subscribe to my shifts* generates a private iCal URL for
your calendar app (30 days back to 90 days ahead). Regenerating it revokes the
previous link, and *Remove* kills it.

## 2. Schedules

**Alerts → On-call → Schedules → New schedule.**

- **Time zone**: an IANA name such as `Europe/Paris`. Handoffs happen at the
  same local wall-clock time all year, across daylight-saving changes (a
  rotation counted in hours is the exception: it counts real elapsed time).
- **Layers**: a schedule is a stack of layers. Where several layers are active,
  **the one lower in the list wins**. Put the general coverage first and the
  exceptions below it.
- **Rotation** per layer: *daily*, *weekly*, or *custom* (a length in hours,
  days or weeks). The **start** is the first handoff: its time of day is the
  handoff time, and for a weekly rotation its weekday is the handoff day.
- **Participants**: an ordered list of users; the rotation walks through it.
- **Restrictions**: time windows per day of the week during which the layer
  applies. A window whose end is at or before its start crosses midnight
  (`18:00` to `09:00`). Without restrictions the layer is always active.
- **Notify on handoff**: optionally tells people when their shift starts and
  ends.

The page shows a week or month calendar and a **Who is on call now** panel.

### Example: a team of three

Alice, Bob and Chloé, time zone `Europe/Paris`.

| Layer | Rotation | Restrictions | Participants |
|---|---|---|---|
| Business days | weekly, starting Monday 09:00 | Monday to Friday, 09:00 to 18:00 | Alice, Bob, Chloé |
| Nights and weekends | weekly, starting Monday 18:00 | Monday to Friday 18:00 to 09:00, Saturday and Sunday all day | Bob, Chloé, Alice |

The second layer is lower, so it wins whenever it is active. Weekday working
hours go to the first layer, evenings and weekends to the second, and nobody
carries both in the same week.

### Replacements

On a schedule, **Add a replacement** reads "X replaces Y from … to …". If you
leave Y empty, X takes over whoever would be on call during that period.
Admins and operators can create and delete replacements, so a swap does not
need an admin to edit the rotation. A replacement shows on the calendar and in
the iCal links.

## 3. Escalation policies

**Alerts → On-call → Escalation policies.**

A policy is an ordered list of **steps**. Each step has one or more targets:

- **On-call of a schedule**: whoever is on call at that moment;
- **a person**: paged on all their contacts that match the alert's severity;
- **a channel**: any notification channel (a chat room, for instance).

Each step has a **delay** in minutes before the next one. **Repeat** replays the
whole chain K more times when it ends without an answer.

### Example: two steps, repeated twice

| Step | Targets | Then wait |
|---|---|---|
| 1 | On-call of *Team schedule* | 10 min |
| 2 | Alice, and the *#ops* channel | 15 min |

Repeat: 2. The alert pages the person on call, 10 minutes later Alice and the
channel, 15 minutes later the on-call person again, and so on: three passes in
total, then the escalation is marked exhausted.

### What stops an escalation

- **Acknowledging**, by anyone: in the UI, with the signed link in the
  notification, or by snoozing, silencing or ignoring the alert. Escalation
  stops and nobody else is paged.
- **Resolution** of the alert. Escalation stops and the people who were already
  paged are told it cleared.

Each alert keeps an **escalation log**: who was paged, when, through which
contact, and who acknowledged. It is on the alert's details.

## 4. Plugging it in

Create a notification channel of type **DumbMonit on-call** (Settings →
Notifications → Channels) and pick its escalation policy. It then
behaves like any channel: routing filters by label, a minimum severity, and the
channels you choose per rule all apply. A typical setup is a rule for critical
production alerts that sends to this channel, while the rest still goes to chat.

The existing global escalation ("if nobody acknowledges, notify another
channel") is untouched and still sends to a single channel.

Use **Test** on the channel to page step 1 with a test message.

!!! note "Acknowledge links need the public URL"
    The acknowledge and snooze links in notifications only work when a
    **public URL** is set (Settings → Notifications → Notification
    policy, or `DUMBMONIT_PUBLIC_URL`). Without it, people acknowledge from the
    UI, and the iCal links are relative paths.

## Roles

Everyone signed in can read schedules, policies and who is on call. Only admins
create or edit schedules and policies; admins and operators manage
replacements. Each user manages their own contacts, quiet hours and calendar
link.

## Backup and restore

[Backups](../install/backup.md) include schedules, replacements, policies,
contacts (with their secrets, since the archive is encrypted) and preferences.
Personal iCal links are **not** restored: generate them again.
