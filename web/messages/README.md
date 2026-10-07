# UI messages

`{locale}.json` (en is the base) are the only files Weblate reads (`messages/*.json`, not recursive).
Compiled by Paraglide into `m.<key>()`.

## Workflow

- Developers add keys to `en.json`, or write `fragments/<zone>.<locale>.json` files
  (several people/agents in parallel), then run `npm run i18n:merge`
  (`--consume` deletes the fragments afterwards, `--force` overwrites existing keys).
- A duplicate key with different texts across fragments fails the merge.
- `npm run i18n:check` (also run by `npm test`): every locale has exactly the keys
  and `{variables}` of `en`.

## Key naming

`<zone>_<component>_<meaning>` in snake_case, e.g. `alerts_row_acknowledge`.
(Older Settings keys are dotted; leave them.)

- One key = one whole sentence with variables (`{count}`); never concatenate fragments.
- Plurals, dates, numbers: plugin plural syntax or `Intl`, never hand-built.
- No text in classes; generic UI components (`src/lib/ui`) get text via props.
- Texts coming from the server (collector catalog, notify kinds, API errors) stay as
  served by the server and are out of scope.
- Never put HTML markup in messages.
