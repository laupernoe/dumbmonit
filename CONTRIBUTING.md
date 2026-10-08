# Contributing to DumbMonit

Thanks for stopping by. Bug reports, device profiles, new integrations and
notification channels are all welcome. This page explains how the project is
built and what a good pull request looks like.

If you are not sure whether something is wanted, open an issue first — a
paragraph is enough — and we will talk it through before you write code.

## Development setup

Everything runs in Docker. You need Docker with Compose v2 and, for the web UI
only, Node 22. No Rust toolchain is required on the host: the `builder` stage of
the `Dockerfile` doubles as a development environment.

### Full stack

```bash
docker compose up -d --build                     # one container (server + embedded VictoriaMetrics), UI on http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build
                                                 # developer overlay: the embedded VictoriaMetrics is published on :8428
```

The first build takes about ten minutes; later ones reuse the dependency layers.

### Rust, without installing Rust

```bash
docker build -t dumbmonit-devenv --target builder .
alias devenv='docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv'

devenv cargo test                                            # whole workspace
devenv cargo test -p dumbmonit-server --test alerts_api       # one integration test file
devenv cargo test -p dumbmonit-server name_of_the_test        # one test by name
devenv cargo clippy --all-targets --all-features -- -D warnings
devenv cargo fmt --all --check
devenv cargo deny check licenses bans                        # needs cargo-deny in the image, see below
```

To keep cargo's registry and target directory between runs, mount named volumes:
`-v dumbmonit-cargo:/usr/local/cargo/registry -v dumbmonit-target:/build/target`.

`cargo deny` is not part of the builder image. Install it once in a derived
container (`cargo install cargo-deny`) or run it locally if you do have Rust.
CI runs it on every push: a dependency under a licence that is not in
`deny.toml`'s allow list fails the build — that is how the "100 % open source"
promise is enforced.

If you do have Rust installed (MSRV 1.95, edition 2024), `cargo test`,
`cargo clippy` and `cargo fmt` work directly. `rustfmt.toml` sets
`max_width = 100` and `use_small_heuristics = "Max"`.

### Web UI

```bash
cd web
npm ci
npm run dev        # Vite on http://localhost:5173, proxies /api to the Docker server on :8080
npm run check      # svelte-check (CI runs this)
npm run build      # static build into web/build, embedded by rust_embed at cargo build
```

The dev server needs the Rust server running (`docker compose up -d`) to answer
API calls. Never run two `vite build`s at once: they share `.svelte-kit/output`.

To see a page as CI and users see it, use the headless browser skill in
`web/tools/README.md` (Chromium + puppeteer in Docker,
screenshots of every route in both themes and both viewports).

## What CI checks

`.github/workflows/ci.yml` runs on every push and pull request:

1. `cargo fmt --all --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all-features`
4. `cargo deny check licenses bans`
5. `npm run check` and `npm run build` in `web/`
6. A multi-arch Docker build (amd64 + arm64), not pushed

Run the first five locally before opening a PR; the sixth only matters if you
touched the `Dockerfile`.

## Project layout

```
crates/proto     shared types: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    the binary — api/, auth/, collectors/, scheduler.rs, tsdb/, db/, alerting/, notify/
crates/agent     Linux/Windows agent; install/ holds install.sh and install.ps1
web/             SvelteKit UI (Svelte 5 runes, Tailwind 4, uPlot)
profiles/        SNMP collection profiles (YAML), auto-applied by sysObjectID
docs/            user documentation (MkDocs, published on Read the Docs)
```

A few mechanics span several files and are worth knowing before you dig in:
secrets never round-trip through the API (omitting `credential` / `secrets` on a
`PUT` keeps the stored value), alerts expose both the engine's `phase` and the
`effective_phase` the user reads, and the Docker build layers dependencies on
dummy sources before the real ones.

## Adding a collector (a new device kind)

1. Implement the `Collector` trait from `crates/proto/src/collector.rs` in a new
   module under `crates/server/src/collectors/`. Look at `uptime/` (small) or
   `synology/` (an HTTP API with options) for a template.
2. Return `ProbeError` carefully: `ProbeError::means_down()` decides whether a
   failure counts as "device unreachable" (an alert) or a configuration error
   (shown in the UI, never notified).
3. Describe the kind for the UI in `crates/server/src/api/collectors.rs`
   (`CollectorView`): label, examples, accepted credential types, the setup
   notice, and typed `options` (stored as `Target.tags[key]`). This is what
   `GET /api/collectors` returns; the "Add a device" form is generated from it,
   so no UI change is needed.
4. Register it in `crates/server/src/main.rs` (`registry.register(...)`). The
   scheduler and the API never know concrete types.
5. Emit metrics named `dumbmonit_<kind>_*` with a `target` label; add default
   alert rules in `alerting/` if the kind has obvious failure modes.
6. Add tests: unit tests next to the parser, and an integration test under
   `crates/server/tests/` if there is an API surface.

For an SNMP device that only needs a new profile, add a YAML file under
`profiles/` with its `sysObjectID` prefix: no Rust needed.

## Adding a notification channel

1. Add the channel's sender in `crates/server/src/notify/` (channels are
   grouped by family: `chat.rs`, `push.rs`, `oncall.rs`, `services.rs`, `smtp.rs`,
   `custom.rs`; the HTTP-based ones are a few dozen lines) and wire it in the
   `match kind` of `notify/mod.rs`.
2. Describe it in `crates/server/src/notify/catalog.rs`: label, docs anchor,
   `settings` (plain fields) and `secrets` (encrypted at rest, never returned by
   the API). The UI renders the form from this description.
3. Add its kind to `CHANNEL_KINDS` in `notify/channel.rs` (a test there checks
   every kind has a catalog entry).
4. Document it in `docs/notifications.md` (published on Read the Docs; the UI
   links to its anchors).
5. Test it with the "Send test message" button in *Settings → Notifications*.

## Translating DumbMonit

The web UI is wired for community translation through
[Weblate](https://hosted.weblate.org/engage/dumbmonit/) (project `dumbmonit`,
component `web-ui`), no code needed: a translator picks or adds a language
there and Weblate opens a pull request adding or updating
`web/messages/<locale>.json`, which a maintainer merges. Nothing is
translated yet — English (`web/messages/en.json`) is the only shipped locale. Strings are extracted with
[Paraglide JS](https://paraglidejs.com) (compile-time, tree-shaken, MIT):

- Each UI string is a flat, dotted key in `web/messages/en.json`
  (`"settings.notifications.title": "Notifications"`), with `{param}`
  placeholders for interpolation. Weblate edits per-locale copies of this file
  (`web/messages/<locale>.json`) directly; nothing else needs to change for a
  new language to appear.
- In components, a string is used as `m["settings.notifications.title"]()`
  (Paraglide exports each message under its literal dotted key, so it is read
  with bracket notation, not dot notation), from
  `import { m } from '$lib/paraglide/messages.js'`. The functions are
  generated by Vite (`web/src/lib/paraglide/`, untracked) from
  `web/project.inlang/` + `web/messages/`; nothing to build by hand.
- When you touch UI copy, add or update the key in `web/messages/en.json`
  first — that is the only locale a pull request needs. The rest of the UI
  (everything outside `web/src/routes/settings/` and the built-in strings of
  `web/src/lib/ui/Confirm.svelte` / `ErrorNotice.svelte`) still has plain
  English text and is being extracted progressively; do not block a PR on
  extracting a string outside the area you are already changing.
- Translators (Weblate) edit `web/messages/<locale>.json` only. Developers add
  keys in `en.json` (or in transient `web/messages/fragments/<zone>.<locale>.json`
  files) and run `npm run i18n:merge`; `npm run i18n:check` (part of `npm test`)
  fails when a locale lacks a key or changes its `{variables}`. Naming rules:
  `web/messages/README.md`.
- A locale only appears in the language picker (Settings → Appearance) once
  its `messages/<locale>.json` file exists; with English alone, the picker
  stays hidden.

See `docs/development.md` for the Weblate component settings (file mask,
format, base file).

### Translating the documentation

The docs (MkDocs, published on Read the Docs) are multilingual through the
[`mkdocs-static-i18n`](https://github.com/ultrabug/mkdocs-static-i18n) plugin,
with the same languages as the UI: `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`,
`zh-Hans` (English is the source). A translation is a sibling file with the
locale before the extension:

```
docs/index.md                  English (source)
docs/index.fr.md               French
docs/install/docker.zh-Hans.md Simplified Chinese
```

- A page without a translation is served in English under the language's
  URL, with a notice at the top; the language selector in the header lists
  the languages by their native names. Nothing breaks when a page is missing.
- Translate prose, headings, table cells, image alt text, link text and the
  comments inside code blocks. Leave commands, code, environment variables,
  paths, URLs and link targets untouched; keep links to other pages as in
  English (`../devices/agent.md`), the plugin points them to the translated
  page when there is one.
- Other pages link to heading anchors, so give each translated heading an
  explicit id equal to the English slug: `## Première installation {#first-start}`.
  `mkdocs build --strict` fails on a broken anchor.
- Navigation titles of the top-level sections are translated in
  `mkdocs.yml` (`nav_translations` of each language); the title of a page is
  its translated first heading.
- `scripts/docs-i18n-status.sh` lists, per language, the pages translated and
  missing. Check the build with
  `docker run --rm -v "$PWD:/docs" -w /docs python:3-slim sh -c "pip install -q -r docs/requirements.txt && mkdocs build --strict"`.
- Weblate is not used for the docs: it is built for key/value files (the UI's
  JSON), and its Markdown support is weak for whole pages with admonitions,
  tabs and attribute lists. Translate by pull request instead.

## Conventions

**Commits** follow [Conventional Commits](https://www.conventionalcommits.org/):
`feat(snmp): add Mikrotik profile`, `fix(alerting): keep silences across
restarts`, `docs: ...`, `chore(deps): ...`. Scopes are free-form; the crate or
area name is a good one.

**Pull requests**: one topic per PR. Small and focused beats large and complete.
UI changes include before/after screenshots (light and dark). Fill in the PR
template; link the issue if there is one.

**Code language**: comments in the Rust crates are in French — that is the
existing style, keep it consistent within a file. The web UI, its copy, its code
and everything the user sees are in English. Commit messages, issues and PR
discussions are in English.

**Web UI rules**: Svelte 5 runes only, all network access through
`src/lib/api/`, token colours only, status is never colour alone, one authored
motion per page, `prefers-reduced-motion` respected.

**Keep types in sync**: `web/src/lib/api/types.ts` mirrors the Rust structs in
`crates/server/src/api/*.rs` exactly and is not validated at runtime.

## Releasing

Before tagging `vX.Y.Z`:

1. Move the `Unreleased` section of `CHANGELOG.md` under the new version and date.
2. Add the release to `web/src/lib/whatsnew/releases.ts` (newest first): the
   version exactly as the server reports it (`0.1.0-alpha.7`, no `v`), the date
   and three to five short highlights in English. It feeds the "What's new in
   vX.Y.Z" window shown once after an update; a version missing from the file
   shows nothing. `WHATSNEW_REQUIRE=1 npm test` in `web/` fails if the
   `Cargo.toml` version has no entry.
3. Bump the version in `Cargo.toml`, then tag `vX.Y.Z`; `release.yml` publishes
   the images, and the window links to the GitHub release of that tag.

## Licence

By contributing you agree that your contribution is licensed under the
[Apache License 2.0](LICENSE), like the rest of the project.
