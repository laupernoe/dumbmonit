# Interface and setup wizard

DumbMonit has one interface: a dense, dark console made to stay open all day.
Earlier versions offered a choice between a "Homelab" and an "Enterprise"
interface; there is only one now, and nothing was lost in the merge.

## The console

- A **menu** on the left lists four destinations: Overview, Monitors, Alerts and
  Status. Each has its own tabs (Monitors: List, Map, Logs, Security; Alerts: Now,
  History, Rules, Automation, On-call; Status: Pages, Reports). At its foot sit
  Settings, wall mode, the documentation, GitHub and your account (the round
  button with your initials). A phone gets a tab bar with the essentials and
  **More**.
- The **header** shows where you are (instance name, page, monitor), a **LIVE**
  switch that pauses the console's own refreshes, the time range on the
  overview, the command palette and **New monitor**.
- The **overview** opens on a thin night-sky banner whose one line says how
  things stand right now ("pve-02 is down. 3 guests held back, alert sent." or
  "All 146 monitors up."). Below it: four figures (monitors up, availability
  over 30 days, p95 response, open alerts), what needs attention beside the
  **health river** (one strip per monitor, worst on top), and two charts. The
  full list of monitors is the **List** tab of Monitors.
- **Wall mode** is a board for a screen in the room: the verdict and the clock,
  the problems, every monitor, the fleet figures, the health river, who is on
  call and, once set up, the music.
- Shortcuts: `g o` overview, `g d` monitors, `g a` alerts, `g s` status, `g m`
  map, `g x` security, `g l` logs, `g n` new monitor. Press `?` for the full list.

States are always written next to a shape (circle up, diamond degraded, square
down), never shown by colour alone. Public status pages keep their own light and
dark themes.

## Setup wizard

The wizard opens after the first administrator signs in on a new instance. Run
it again any time from **Settings → General → Run the setup wizard**.

1. **Instance**: name shown in the header, default language and time zone.
2. **Notifications**: add a channel (the same form as in Settings, so every
   channel type is there) and send a test.
3. **First monitors**: scan your network in place, or open the import and
   add-by-hand pages in a new tab.
4. **Agent**: the install command for a machine to watch.
5. Three optional steps: **single sign-on and teams**, **on-call**
   (notification policy and roles) and **retention**.

Every step can be skipped, and **Skip setup** ends the wizard at once. The
wizard is marked done on the server, so it does not come back in another
browser.

## API and backup

- `GET /api/instance` is public and answers only `setup_completed`.
- `GET /api/instance/settings` returns the name, language and time zone;
  `PUT /api/instance/settings` (administrators) changes them. An omitted field
  keeps its value. The former `product_mode` field is ignored if a client still
  sends it.
- The settings are part of the configuration backup (section `instance`). A
  restored backup never reopens the wizard on an instance where it was done, and
  a backup written by an older version (with `product_mode`) restores as is.
