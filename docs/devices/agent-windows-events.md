# Windows event log alerts

The Windows agent can watch the Windows event log and raise an alert when a
matching event is logged: failed logons, an unexpected reboot, a service that
crashed, disk errors. Rules are set per machine on its device page, in the
**Windows event log** panel, and reach the agent with the acknowledgement of its
next report: no change on the machine, no restart.

**No rule exists by default**: until you add one, the agent reads nothing from
the event log and nothing is raised. The panel offers examples
(see [Suggested rules](#suggested-rules)) that you add in one click and can
edit before saving.

## What a rule matches

| Field | Meaning |
|---|---|
| Event log | `System`, `Application`, `Security`, or any channel such as `Microsoft-Windows-TaskScheduler/Operational` or `Microsoft-Windows-Windows Defender/Operational`. |
| Source | The provider name as Event Viewer shows it in the *Source* column's details (`Microsoft-Windows-Kernel-Power`, `Service Control Manager`). Empty: any source. |
| Event IDs | Comma-separated IDs and ranges: `4625, 4740` or `7000-7099`, 20 at most. Empty: any ID. |
| Levels | Critical, Error, Warning, Information. None ticked: every level. Information also covers level 0, the level of every Security audit event (4625 included). |
| Message contains | Text searched in the formatted message, case-insensitive. Tick *regular expression* to use a (case-sensitive) regular expression instead. Empty: every event the other fields match. |
| Alert severity | Info, Advisory or Warning: the alert raised on the machine. |

A rule needs at least an event ID, a source or a level: a rule on a whole log
would report every event. The fields become an XPath query
(`*[System[Provider[@Name='…'] and (EventID=4625) and …]]`) run by the agent
through the Windows Event Log API.

## How the agent reads the log

- Every cycle (30 s by default), for each rule, the agent asks the log for the
  events newer than the last one it read.
- A **new rule only reports events logged after it reached the agent**: the
  history already in the log is never sent.
- The last record read for each rule is kept next to the agent's binding
  secret (`eventlog-state.json`): after a restart, the agent resumes where it
  stopped, so a `41 Kernel-Power` logged during the boot that follows a crash is
  still reported. A cleared log is detected and read again from its start.
- Messages are formatted in the system's language (`EvtFormatMessage`); when a
  provider has no message, the event's data fields are sent instead. Each
  message is cut at 2,000 characters.
- Reading the Security log needs the rights of the account the agent runs as:
  the Windows service runs as LocalSystem, which can read it. If a log cannot be
  read, the agent says so once in its log and keeps the other rules going.

## Rate limit

At most **10 events per rule and per minute** are sent. The rest is counted
and shown on the rule as *skipped by the rate limit*: a password-guessing burst
of thousands of 4625 does not flood the server or your notification channels.
Events waiting for an unreachable server are capped at 500; the oldest give way
and are counted as skipped too.

## Alerts

Every matching event raises the built-in alert of the rule's severity on the
machine:

| Rule | Severity |
|---|---|
| Windows event (critical) | Warning (`critical`) |
| Windows event (warning) | Advisory (`warning`) |
| Windows event (info) | Info (`info`) |

- One alert per machine and per severity, not one per event: the notification
  quotes the latest event (rule name, event ID, source and first line of the
  message).
- While events keep coming, the alert speaks again at most every 10 minutes,
  quoting the newest event.
- The alert resolves 15 minutes after the last matching event. There is no
  reminder: an event is a fact, not an ongoing outage.

The series behind it is `dumbmonit_agent_eventlog_matches_<severity>`, the
number of events received in the last 15 minutes. The device page lists the
last events received (the server keeps 500 per machine), with their full
message.

## Suggested rules

None is active until you add it.

| Example | Log | Source | Event IDs | Severity |
|---|---|---|---|---|
| Failed logons | Security | Microsoft-Windows-Security-Auditing | 4625 | Advisory |
| Account locked out | Security | Microsoft-Windows-Security-Auditing | 4740 | Advisory |
| Unexpected reboot | System | Microsoft-Windows-Kernel-Power | 41 | Warning |
| Service terminated unexpectedly | System | Service Control Manager | 7031, 7034 | Advisory |
| Disk errors | System | any (levels Error and Warning) | 7, 11, 51, 153 | Warning |
| Security log cleared | Security | any | 1102 | Warning |
| Application errors | Application | any (levels Critical and Error) | any | Info |

Other useful IDs:

- `6008` (System, EventLog): the previous shutdown was unexpected.
- `1074` (System, User32): who restarted or shut down the machine, and why.
- `4720`, `4726` (Security): a user account was created, deleted.
- `4728`, `4732`, `4756` (Security): a member was added to a security group.
- `7045` (System, Service Control Manager): a new service was installed.
- `1001` (System, BugCheck): the machine restarted after a blue screen.
- `1116` (`Microsoft-Windows-Windows Defender/Operational`): malware detected.

Audit events (4625, 4740, 4720…) are only logged when the matching audit
policy is enabled on the machine or by Group Policy.

## Requirements

An agent of this version or later, on Windows. An older agent ignores the
rules; rules set on a machine that does not run Windows are kept but never sent.
