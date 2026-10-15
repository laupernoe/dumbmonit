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
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build
# one container (server + embedded VictoriaMetrics) built from the sources, UI on
# http://localhost:8080; the overlay also publishes the embedded VictoriaMetrics on :8428
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

The dev server needs the Rust server running (`docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build`) to answer
API calls. Never run two `vite build`s at once: they share `.svelte-kit/output`.

To see a page as CI and users see it, use the headless browser skill in
`web/tools/README.md` (Chromium + puppeteer in Docker,
screenshots of every route in both themes and both viewports).

## Branches

- **`dev`** is where work lands: open pull requests against `dev`. Every push
  to `dev` builds the `:edge` images.
- **`main`** is the stable line. It only receives `dev` through a pull request
  once it is checked, and releases are tagged from it.

## What CI checks

`.github/workflows/ci.yml` runs on every push to `main` or `dev` and every pull request:

1. `cargo fmt --all --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all-features`
4. `cargo deny check licenses bans` and `cargo deny check advisories`
   (licences, banned crates, RustSec advisories)
5. In `web/`: `npm run check` (svelte-check, which also compiles the Paraglide
   messages), `npm test` (unit tests, including `i18n:check`, which fails when a
   locale lacks a key or changes its `{variables}`) and `npm run build`
6. A Docker build of the server image and of the agent image, not pushed

Other automated checks, outside the pull request gate:

- **CodeQL** code scanning (GitHub default setup).
- **`security.yml`** (weekly and after each release): a Trivy scan of the
  published server and agent images, and the OpenSSF Scorecard.
- **Dependabot** opens pull requests for dependency and Actions updates.

Run checks 1 to 5 locally before opening a PR; the sixth only matters if you
touched the `Dockerfile`.

## Test policy

- New behaviour comes with automated tests in the same pull request: unit tests
  next to the code, Rust integration tests under `crates/server/tests/` for
  anything with an API surface, `web/tests/*.test.mjs` for logic in the web UI.
- Every bug fix comes with a regression test that fails without the fix. If a
  fix cannot reasonably be tested, say why in the pull request.
- Run them with `cargo test --all-features` (see above for Docker) and
  `npm test` in `web/`. A pull request is not merged while CI is red.

## Coding standards

- **Rust:** `rustfmt` with the repository's `rustfmt.toml` (`max_width = 100`,
  `use_small_heuristics = "Max"`), edition 2024, and `clippy` clean with
  `-D warnings`. Comments are in French, as in the rest of the Rust code.
- **Web UI:** Svelte 5 runes only (no `export let`, no `on:` handlers); imports
  use the `#lib/...` alias; all network access goes through `src/lib/api/`.
  Every user-visible string goes through Paraglide
  (`m.<zone>_<component>_<meaning>()`).
- **Accessibility and design:** token colours only (`signal`, `advisory`,
  `warning`, `info`, `ink`, `surface`, `line`) and the primitives in
  `src/lib/ui/`; a status is never conveyed by colour alone; no horizontal
  overflow at 390 px width and tap targets of at least 40 px;
  `prefers-reduced-motion` is respected.
- **Secrets:** never return a secret from the API; never log one.

## Developer Certificate of Origin

Contributions are accepted under the project licence (Apache-2.0) and must be
signed off to certify the
[Developer Certificate of Origin](https://developercertificate.org/): that you
wrote the change or have the right to submit it under that licence. Add a
`Signed-off-by: Your Name <you@example.com>` line to each commit with
`git commit -s`.

## Security design

DumbMonit holds the credentials of every device it watches, so changes are
reviewed with secure design principles in mind: least privilege, secure
defaults, no secret ever returned by the API, vetted cryptography only, and
untrusted input from devices and agents. The
[security model](https://dumbmonit.readthedocs.io/en/latest/development/security-model/)
lists the trust boundaries and where each countermeasure lives. Report
vulnerabilities privately, as described in [SECURITY.md](SECURITY.md). The
release notes list every fixed vulnerability with its CVE or GHSA identifier.

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

The web UI is translated by pull request, no special tooling needed. English
(`web/messages/en.json`) is the source; each other language is one file,
`web/messages/<locale>.json` (`fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`,
`zh-Hans`). Strings are compiled by [Paraglide JS](https://paraglidejs.com)
(compile-time, tree-shaken, MIT).

To improve or add a translation:

1. Edit `web/messages/<locale>.json`, or copy `en.json` to add a new locale
   (a locale appears in the language picker, Settings → Appearance, as soon as
   its file exists).
2. Keep every key and every `{variable}` of `en.json`; translate only the
   values. Never put HTML markup in a message.
3. Plurals use the message syntax (a `match`/`plural` declaration inside the
   message), never hand-built strings: keep the structure of the English
   message and translate each branch.
4. Run `cd web && npm run i18n:check` (also part of `npm test`): it fails when a
   locale lacks a key or changes its `{variables}`.
5. Open a pull request.

For developers:

- Each UI string is a key in `web/messages/en.json`, named
  `<zone>_<component>_<meaning>` in snake_case (`alerts_row_acknowledge`),
  with `{param}` placeholders. Naming rules: `web/messages/README.md`.
- In components, a string is used as `m.alerts_row_acknowledge()` from
  `import { m } from '#lib/paraglide/messages.js'`. The functions are
  generated by Vite (`web/src/lib/paraglide/`, untracked) from
  `web/project.inlang/` + `web/messages/`; nothing to build by hand.
- When you touch UI copy, add or update the key in `web/messages/en.json`
  first. Other locales may follow in the same pull request or later; the
  check requires all locales to have the same keys, so run
  `npm run i18n:merge` (see `web/messages/README.md`) to merge fragments.
  Do not block a PR on extracting a string outside the area you are changing.

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

## Conventions

**Commits** are signed off (see above) and follow [Conventional Commits](https://www.conventionalcommits.org/):
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

Versions are plain `0.MINOR.PATCH`: `0.9.0`, `0.10.0`, … A new release
bumps the minor number, a fix-only release the patch number (`0.9.1`). The
leading `0` says the project is not stable yet; `1.0.0` will mark that it is.
(Up to 0.1.0-alpha.8, versions carried an `-alpha.N` suffix.)

Releases are tagged from `main`, after `dev` has been merged into it.

Before tagging `vX.Y.Z`:

0. Check that the **Upgrade test** workflow is green on `main`. It starts the
   last published release, creates a user, devices of several kinds, a rule, a
   silence, a channel and a status page, waits for metrics, then starts the
   candidate image on the same `/data` volume and checks that health, login,
   every object and the VictoriaMetrics history survived. Run it by hand
   against any image with `scripts/upgrade-test.sh [IMAGE]` (needs docker,
   curl and python3; default image `ghcr.io/laupernoe/dumbmonit:edge`;
   `OLD_IMAGE=…` picks another starting point). A failure means an existing
   install would lose data when updating: fix it before tagging.

1. Move the `Unreleased` section of `CHANGELOG.md` under the new version and date.
2. Add the release to `web/src/lib/whatsnew/releases.ts` (newest first): the
   version exactly as the server reports it (`0.9.0`, no `v`), the date
   and three to five short highlights in English. It feeds the "What's new in
   vX.Y.Z" window shown once after an update; a version missing from the file
   shows nothing. `WHATSNEW_REQUIRE=1 npm test` in `web/` fails if the
   `Cargo.toml` version has no entry.
3. Bump the version in `Cargo.toml`, run `python3 site-src/build.py` (the website
   reads the version from `Cargo.toml`), then tag `vX.Y.Z`; `release.yml` publishes
   the images and the GitHub release, whose notes are that version's CHANGELOG
   section; the window links to that release. The workflow refuses a tag
   that does not match the `Cargo.toml` and `Cargo.lock` version.

A pre-release tag (anything with a `-`, e.g. `v0.9.0-rc.1`) runs the same
pipeline to try it out: the images get only `:0.9.0-rc.1` (never `:latest`,
`:0.9`, `:0`), and the GitHub release is a **draft**, marked pre-release and
never latest. Its notes are its own CHANGELOG section, else the `0.9.0` one,
else `Unreleased`. Bump `Cargo.toml` (and `Cargo.lock`) to the pre-release
version first, but do not run `site-src/build.py` for it.

The draft carries every binary, checksum, signature and the provenance, but
only maintainers see it: running servers never learn about it. Once
published, every instance already on a pre-release (all `0.1.0-alpha.N`) is
offered it as an update; stable instances never are. To publish it anyway,
open the draft on the Releases page, check that **Set as a pre-release** is
ticked and **Set as the latest release** is not, then **Publish release**; or
`gh release edit v0.9.0-rc.1 --draft=false --prerelease --latest=false`. To
drop it, delete the draft and the tag (`gh release delete v0.9.0-rc.1
--cleanup-tag`); the `:0.9.0-rc.1` images stay on ghcr.io until deleted there.

## Licence

By contributing you agree that your contribution is licensed under the
[Apache License 2.0](LICENSE), like the rest of the project.
