# Metrics explorer

The **metrics explorer** (`/explore`) lets you browse what DumbMonit stores and try a
query. Open it from the command palette (`Ctrl+K`).

The explorer lists the live `dumbmonit_*` metrics (written during the last hour) with
their number of series and, for the metric you pick, its labels and a few values.
Selecting a metric fills the query box; selecting a label value adds it as a matcher.

The query box completes metric names as you type, label names inside `{…}` and label
values after `label="`. `Ctrl+Space` lists suggestions, `Ctrl+Enter` runs the query.
The result is a chart and a table of the latest value of each series. Queries go
through DumbMonit's own query proxy, never straight to VictoriaMetrics.

## Permissions at a glance

Everyone who is signed in can use the explorer; for an account limited by teams or
clients, the series are filtered to the monitors it can see.

## API

`GET /api/metrics/catalog` is described in the OpenAPI specification.

!!! note
    Custom dashboards were removed. Their data stays in the database but is no
    longer read, shown or exported; old backups that contain dashboards still restore
    everything else.
