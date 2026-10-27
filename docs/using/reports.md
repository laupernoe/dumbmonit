# Email reports

A report is a short email that tells your team how the infrastructure did over
the last day, week or month, without anyone opening DumbMonit. It is weekly by
default; daily and monthly are one click away. Reports reuse an **email (SMTP)
notification channel**, so set one up first (see
[Notification channels](../notifications.md)).

Open **Reports** (in the icon rail) to set them up. The page holds the availability (SLA) export of a period, the scheduled reports and the history of their sends; it used to be in Settings, and `/settings/reports` forwards to it. Everything here is admin-only,
reading included, because the page lists the recipients' addresses.

## What a report contains

| Section | Content |
| --- | --- |
| **Headline** | Overall availability for the period, the number of incidents, and the change against the previous period. |
| **Availability by group** | One line per group, then per monitor (the 40 least available at most), computed the same way as on [status pages](status-pages.md): probes by their success rate, hosts and monitors by how often they reported in. Monitors without history are shown as "no data" rather than as 100%. |
| **Incidents** | When each one started, how long it lasted (or that it is still open) and on which monitor. Up to 25. Informational alerts are left out. |
| **Least stable monitors** | The five monitors with the lowest availability or the most incidents. |
| **Comparison** | Availability and incident count against the period of the same length just before. |

The email is plain HTML tables with inline styles and no remote image, so it
renders the same everywhere and nothing is fetched when it is opened. A text
version is sent along. Every monitor name and message is escaped.

## Settings

| Setting | What it does |
| --- | --- |
| **Name** | Shown in the list only. |
| **Enabled** | Off keeps the settings without sending anything. |
| **Frequency** | Daily, weekly (default) or monthly. |
| **Day** | The weekday for weekly reports, the day of the month (1 to 28) for monthly ones. |
| **Hour** and **Timezone** | When the report goes out, in an IANA timezone such as `Europe/Paris`. Daylight saving time is followed. |
| **Recipients** | Up to 20 addresses, free-form: they do not have to be DumbMonit users. Each recipient gets their own email, so nobody sees the others. |
| **Email channel** | Which SMTP channel sends it. By default, the first enabled one. Only the mail server of the channel is used: its own recipients are ignored. |

The period is a rolling window that ends when the report is sent: the last 24
hours, 7 days or 30 days. The previous period is the window of the same length
just before.

Click the **i** next to the section title for a reminder of the content.

## SLA reports {#sla}

A schedule of type **SLA report** is the monthly availability report you hand to
a customer or an auditor. It covers the previous **calendar month** (the 1st at
midnight to the 1st at midnight, in the schedule's timezone), so it is always
monthly: the email goes out on the chosen day of the month, with:

- the **PDF**: the name of the organisation (the title of a status page you
  choose, otherwise DumbMonit), the period, availability, incidents and total
  downtime, availability by group and by monitor with the **mean and 95th
  percentile response time** of each probe, and every incident;
- two **CSV files**: the monitors (`availability-2026-10-monitors.csv`) and the
  incidents (`availability-2026-10-incidents.csv`), UTF-8 with a byte order mark
  for Excel. Cells that start with `=`, `+`, `-` or `@` get a leading apostrophe,
  so a monitor name cannot become a spreadsheet formula.

Both attachments can be switched off separately. With an **availability
objective** (for example 99.9), each monitor is marked **Met** or **Missed**
and the report says how many met it; without one, the usual words (Healthy,
Degraded, Poor) are used. The report is in English; the figures are the ones
of the status pages.

The PDF is written by DumbMonit itself, with the standard Helvetica fonts and no
library: A4, text and tables, a footer with the generation date and the page
number. The logo of a status page is not embedded; its name heads the report.

### Download a period

At the top of the page, **Availability report (SLA)** builds the same report on
demand, for any period of one year at most (both days included, in the timezone
you choose, ending now if it has not ended). Choose the PDF, the monitors CSV or
the incidents CSV. The default is the previous calendar month. In MSP mode,
choose the client the report is about. Administrators only.

### Sending history

At the foot of the page, **Sending history** lists each scheduled report with
its last send, the next one due and whether the last attempt succeeded (a failed
send shows its error until the next success).

## Send a preview

**Send a preview** emails the current report to the saved recipients right now,
with `[Preview]` in the subject. It changes nothing about the schedule: the
next report still goes out on time. A preview can be sent once every 30
seconds per report. Save your changes first, since the preview uses the saved
recipients.

## Delivery guarantees

- **At most once.** The slot is claimed in the database before the email is
  built, so a restart never sends the same report twice. When the send fails,
  the error is kept on the report and shown in the list, and the slot is not
  retried; the next one goes out as usual.
- **No catch-up.** If DumbMonit was down at the scheduled time, the report is
  sent when it comes back only if that is within 3 hours of the slot. Later than
  that, it waits for the next slot. A report you create or re-enable never sends
  a slot that is already past.
- An SLA report is built at the time of sending, from the previous calendar
  month: a restart or a late send (within the 3 hours) gives the same report.
- At most 10 reports can exist.
- In demo mode nothing is sent.

Reports are included in [backups](../install/backup.md): the email channel is
matched by name on restore, and a restored report starts fresh (its last-sent
date is not carried over).

## API

See [HTTP API](../reference/api.md#email-reports).
