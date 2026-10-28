# Interface and setup wizard

DumbMonit has one interface: a dense, dark console made to stay open all day.
Earlier versions offered a choice between a "Homelab" and an "Enterprise"
interface; there is only one now, and nothing was lost in the merge.

## The console

- An **icon rail** on the left lists every destination, status pages and reports
  included. At its foot sit the documentation, GitHub, your account (the round
  button with your initials) and Settings. A phone gets a tab bar with the
  essentials and **More**.
- The **header** shows where you are (instance name, page, monitor), a **LIVE**
  switch that pauses the console's own refreshes, the time range on the
  overview, the command palette, **New monitor**, **wall mode** (the screen
  icon) and the context panel toggle.
- The **context panel** on the right, like an inspector, shows what needs
  attention, the monitor selected in a table (with **Check now** and **Silence
  1 h**), the nearest forecast and who gets told. Press `i` to show or hide it;
  on narrower screens it opens as a drawer.
- The **overview** opens on a thin night-sky banner whose one line says how
  things stand right now ("pve-02 is down. 3 guests held back, alert sent." or
  "All 146 monitors up."). Below it: five figures (monitors up, availability
  over 30 days, p95 response time, open alerts, mean time to resolve), the
  **health river** (one strip per monitor, one cell per slot of the range, worst
  monitors on top) and a **monitor table** sorted worst first. The table filters
  by state, type and text, lets you add columns (address, folder) and remove
  others, and is driven by the keyboard: `j` / `k` move, `Enter` selects, `o`
  opens.
- **Wall mode** is a black board for a screen in the room: the health river, the
  five figures, open alerts and who is on call.
- Shortcuts: `g o` overview, `g d` monitors, `g a` alerts, `g m` topology map,
  `g x` security, `g l` logs. Press `?` for the full list.

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
