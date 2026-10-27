# Interface and setup wizard

## The console

DumbMonit has a single interface: a dense, very dark console for teams who keep
it open all day. Administrators set the instance name, language and time zone in
**Settings → General**.

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
- **Wall mode** is a board of its own: black ground, the health river, the five
  figures, open alerts and who is on call, with no scenery.
- More shortcuts: `g m` topology map, `g x` security, `g l` logs. Press `?` for
  the full list.
- Device forms open **More options** from the start and show more fields up
  front.

States are always written next to a shape (circle up, diamond degraded, square
down), never shown by colour alone.

## Setup wizard

The wizard opens after the first administrator signs in on a new instance. Run
it again any time from **Settings → General → Run the setup wizard**.

1. **Instance**: name shown in the header, default language and time zone.
2. **Notifications**: add a channel (the same form as in Settings, so every
   channel type is there) and send a test.
3. **First monitors**: scan your network in place, or open the import and
   add-by-hand pages in a new tab.
4. **Agent**: the install command for a machine to watch.

Three optional steps follow: **single sign-on and teams**,
**on-call** (notification policy and roles) and **retention**.

Every step can be skipped, and **Skip setup** ends the wizard at once. The
wizard is marked done on the server, so it does not come back in another
browser.

## API and backup

- `GET /api/instance` is public and answers only `product_mode` (always
  `enterprise`) and `setup_completed`.
- `GET /api/instance/settings` returns the name, language and time zone;
  `PUT /api/instance/settings` (administrators) changes them. An omitted field
  keeps its value.
- The settings are part of the configuration backup (section `instance`). A
  restored backup never reopens the wizard on an instance where it was done.
