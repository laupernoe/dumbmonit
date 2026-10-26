# Dashboards and metrics explorer

**Dashboards** (`/dashboards`) are boards you assemble yourself from widgets. The
**metrics explorer** (`/explore`) lets you browse what DumbMonit stores, try a query,
and put the result on a board.

## Dashboards

Pick a dashboard from the list, or create one with **New dashboard**. A dashboard is
either:

- **personal**: only its owner sees and changes it, whatever their role (viewers
  and operators included);
- **shared**: everyone can read it, administrators change it. Only administrators
  can create a shared dashboard or turn sharing on or off.

Each dashboard saves its own **time range** (15 minutes to 30 days) and **refresh**
interval (off, 10 s, 30 s, 1 min, 5 min). While viewing, you can change both on the
fly without saving; they return to the saved values when you reopen the dashboard.

### Widgets

| Widget | Shows |
|---|---|
| Chart | A MetricsQL query over the dashboard's time range, drawn with the same chart as the device pages. |
| Value | One number from a query, with an optional **warning** and **critical** level. Say whether the problem is a value that is *too high* or *too low*; the widget then also says it in words ("Above critical level"), never by colour alone. |
| Folder status | The state of every monitor in one folder, worst first, each linking to its page. |
| Active alerts | The alerts firing now, most severe first. |
| Note | Simple Markdown: `#` headings, `**bold**`, `*italic*`, lists, `` `code` `` and links. HTML is never interpreted. |

### Editing the layout

Choose **Edit dashboard**, then add widgets and arrange them on the 12-column grid.

- **Mouse and touch**: drag a widget by its grip, resize it from the corner handle.
  The dashed outline shows where it will land; the others make room.
- **Keyboard**: focus the grip, then use the arrow keys to move the widget, or
  Shift and the arrow keys to resize it (or the arrow keys on the corner handle).
  Escape cancels a drag. Changes are announced to screen readers.
- **Phones**: the grid is a single column. Use the *move up* and *move down* buttons
  to reorder widgets; sizes are kept for wide screens.

Nothing is saved until you choose **Save dashboard**.

## Metrics explorer

The explorer lists the live `dumbmonit_*` metrics (written during the last hour) with
their number of series and, for the metric you pick, its labels and a few values.
Selecting a metric fills the query box; selecting a label value adds it as a matcher.

The query box completes metric names as you type, label names inside `{…}` and label
values after `label="`. `Ctrl+Space` lists suggestions, `Ctrl+Enter` runs the query.
The result is a chart and a table of the latest value of each series. Queries go
through DumbMonit's own query proxy, never straight to VictoriaMetrics.

**Add to a dashboard** puts the current query on an existing dashboard you may edit,
or on a new personal one, as a chart or a value widget.

## Permissions at a glance

| Action | Viewer | Operator | Administrator |
|---|---|---|---|
| Read shared dashboards and the metrics explorer | yes | yes | yes |
| Create, edit, delete **personal** dashboards | own only | own only | own only |
| Create, edit, delete **shared** dashboards | no | no | yes |

API tokens see shared dashboards only; a `write` token can manage shared ones.
Dashboards are part of backups: shared ones with the instance,
personal ones with their owner's account name.

## API

`GET/POST /api/dashboards`, `GET/PUT/DELETE /api/dashboards/{id}` and
`GET /api/metrics/catalog` are described in the OpenAPI specification.
