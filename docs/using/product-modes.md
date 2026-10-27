# Product modes and setup wizard

DumbMonit has two ways of presenting itself. They show the same monitors, alerts
and settings; only the interface changes. An administrator picks one at first
launch and can switch at any time in **Settings → General**. Switching never
deletes anything.

## Homelab

The default, and the mode every instance created before this setting keeps.

- The bar holds four destinations: **Overview**, **Monitors**, **Alerts** and
  **Settings**. Security, the topology map, logs, status pages, reports, wall mode, the
  documentation and GitHub sit under **More**, and every page is in the command
  palette (`Ctrl K` / `⌘K`).
- Each Settings tab shows the sections most people need. The rest (single
  sign-on, LDAP, integration packs, security score, locations, notification
  policy, music, wall widgets, self-monitoring, storage, logs, API and
  assistants) waits behind **Advanced** at the foot of the tab. A link to one of
  those sections opens the fold by itself.
- Device forms show the required fields and the first setting; the others are
  under **More options**, with sensible defaults.
- The overview is the weather bulletin, and the light and dark themes work as
  before.

## Enterprise

A dense, very dark console for teams who keep it open all day.

- An **icon rail** on the left lists every destination, status pages and reports
  included. At its foot sit the documentation, GitHub, your account (the round
  button with your initials) and Settings. A phone gets a tab bar with the
  essentials and **More**.
- The **header** shows where you are (instance name, page, monitor), a **LIVE**
  switch that pauses the console's own refreshes, the time range on the
  overview, the command palette and **New monitor**.
- The **context panel** on the right, like an inspector, shows what needs
  attention, the monitor selected in a table (with **Check now** and **Silence
  1 h**), the nearest forecast and who gets told. Press `i` to show or hide it;
  on narrower screens it opens as a drawer.
- The **overview** has five figures (monitors up, availability over 30 days,
  p95 response time, open alerts, mean time to resolve), the **health river**
  (one strip per monitor, one cell per slot of the range, worst monitors on top)
  and a **monitor table** sorted worst first. The table filters by state, type
  and text, lets you add columns (address, folder) and remove others, and is
  driven by the keyboard: `j` / `k` move, `Enter` selects, `o` opens.
- **Wall mode** has its own Enterprise board: black ground, the health river, the five
  figures, open alerts and who is on call, with no scenery. Homelab keeps its
  city scene.
- More shortcuts: `g m` topology map, `g x` security, `g l` logs. Press `?` for
  the full list.
- Device forms open **More options** from the start and show more fields up
  front.

States are always written next to a shape (circle up, diamond degraded, square
down), never shown by colour alone.

## Setup wizard

The wizard opens after the first administrator signs in on a new instance. Run
it again any time from **Settings → General → Run the setup wizard**.

1. **Mode**: Homelab or Enterprise, with a preview of each.
2. **Instance**: name shown in the header, default language and time zone.
3. **Notifications**: add a channel (the same form as in Settings, so every
   channel type is there) and send a test.
4. **First monitors**: scan your network in place, or open the import and
   add-by-hand pages in a new tab.
5. **Agent**: the install command for a machine to watch.

In Enterprise mode, three optional steps follow: **single sign-on and teams**,
**on-call** (notification policy and roles) and **retention**.

Every step can be skipped, and **Skip setup** ends the wizard at once. The
wizard is marked done on the server, so it does not come back in another
browser.

## API and backup

- `GET /api/instance` is public and answers only `product_mode` and
  `setup_completed`, so the sign-in page already has the right look.
- `GET /api/instance/settings` returns the mode, name, language and time zone;
  `PUT /api/instance/settings` (administrators) changes them. An omitted field
  keeps its value.
- The settings are part of the configuration backup (section `instance`). A
  restored backup never reopens the wizard on an instance where it was done.
