# Product website

Sources in `site-src/` (templates, strings, `static/` files); `website/` is the build output.

The static page published at **https://dumbmonit.app/**, with the documentation under
**https://dumbmonit.app/docs/**. Plain HTML and CSS, a little JavaScript (copy buttons, the hero
demo), no analytics, no third-party requests. Fonts (Geist and Geist Mono, SIL OFL)
are self-hosted in `assets/fonts/`. Colours and type are those of the app's Enterprise theme
(`web/src/app.css`, `html.mode-enterprise`), always dark; the layout is a plain product site (top
navigation, full-width sections, footer in columns). The hero plays a short scripted demo in a
browser frame (`site.js`); the first click, tap or keyboard focus hands it to the visitor (pick a
row, acknowledge the alert, Escape closes the detail, arrow keys move between rows, Replay), and
the script resumes after 20 s without input. Under `prefers-reduced-motion` it is a still frame
that can be used the same way. The diagrams are drawn in HTML and CSS (`DIAGRAMS` in `build.py`)
so their text wraps and translates; each one lays itself out from its own width (container
queries), horizontal when it has the room, vertical otherwise.

The home page plays a recorded tour (`#video`, muted, loops while on screen, never under reduced
motion). Its files live once in the repository, in `docs/assets/demo/` (the README uses them too);
`build.py` copies `dumbmonit-demo.webm`, `dumbmonit-demo.mp4` and `demo-poster.jpg` into
`website/assets/demo/` and fails if one is missing.

## One-time setup

In the GitHub repository: **Settings → Pages → Build and deployment → Source:
GitHub Actions**. That is the only switch.

After that, `.github/workflows/site.yml` builds the site and the docs and deploys them on every
push to `main` or `dev` that touches `site-src/**`, `docs/**`, `mkdocs.yml`, `overrides/**` or
`hooks/**`, and can be run by hand from the **Actions** tab (*Site → Run workflow*). To deploy from
`dev`, allow it in *Settings → Environments → github-pages → Deployment branches*. The site is
updated without a release, but **what it announces as available comes from the latest stable
release tag**: the integrations, their count, the channels and the version are read with
`git show vX.Y.Z:<path>` (`extract.py`), while the texts, the roadmap and the docs come from the
pushed branch.

## Preview locally

```sh
python3 site-src/build.py                                  # product data from the latest tag
DUMBMONIT_SITE_REF=worktree python3 site-src/build.py      # or from the working tree
python3 -m http.server 4321 --directory website            # open http://localhost:4321/
```

The docs are built separately (a venv with `docs/requirements.txt`), one build per language:
`READTHEDOCS_LANGUAGE=fr DUMBMONIT_DOCS_BASE=https://dumbmonit.app/docs/ mkdocs build -d website/docs/fr`
(English goes to `website/docs`). The links between the site and the docs are relative on the site
side (`DOCS_DIR` in `build.py`) and absolute on the docs side (`hooks/rtd_language.py`).

## Contents

| Path | What |
|---|---|
| `index.html`, `features/`, `devices/`, `devices/<family>/`, `install/`, `roadmap/`, `community/` (and the same under `<lang>/`) | The pages in each language, **generated** by `site-src/build.py` (do not edit by hand). English is the root, the others live in `/fr/`, `/de/`, `/es/`, `/it/`, `/pt/`, `/pt-BR/`, `/ru/`, `/zh-Hans/`. Colours and type follow the Enterprise tokens of `web/src/app.css` (dark only). |
| `static/assets/site.css` | Styles and design tokens. |
| `static/assets/shots/` | Screenshots of the live demo (none at the moment: capture them from the console before using `@@SHOT@@` again) (`DUMBMONIT_DEMO=1`): desktop 1440 x 900 at 2x (plus a 1x copy), phone 390 x 844 at 2x, light and dark, AVIF with a WebP fallback. |
| `static/assets/og.png` | Social preview image, 1200 × 630. |
| `404.html`, `robots.txt`, `sitemap.xml`, `.nojekyll` | Pages housekeeping. |

## Updating screenshots

Screenshots must only show fictional names and addresses (`*.home.arpa`,
`example.com`, `10.99.99.99`, test fixtures). Never capture a view that shows
a real hostname, domain or IP address; crop it or leave it out. Take them in
both themes at 1440 px wide (390 px for the phone view) and save them as WebP.

## Languages

One shared layout (`site-src/templates/base.html`: header, language selector, footer), one template
per kind of page, one string file per language. Edit the templates and
`site-src/strings/<lang>.json` (texts; `en.json` is the base and a missing key falls back to
English), then run `python3 site-src/build.py` to preview: it writes `website/`, which is git-ignored and
never committed. The Site workflow builds it again before publishing.

The device pages are generated from the project itself at every build: `extract.py` (called by
`build.py`) reads, at the latest release tag, the collector catalogue
(`crates/server/tests/fixtures/collectors_catalog.json`), `docs/devices/` and
`docs/alerting/rules.md`. There is no list to maintain: a new collector appears on the site with
the release that ships it. `FAMILIES` in `extract.py` only sorts types into families; a type it
does not know yet lands in "More integrations" until it is given one. The badges come from the
catalogue wording: *Beta* ("(beta)" in the label or a summary starting with "Beta.") and *Via
agent* (a summary starting with "Needs the agent"). `python3 site-src/extract.py` prints what the
site would show. Device and rule texts are English; only the labels around them are translated.
The locales are those of `web/messages/`.

- The browser language picks the page on the first visit to the root (English if none of the
  nine matches); the selector in the header remembers an explicit choice.
- Every page carries `lang`, `hreflang` alternates (and the sitemap too).
- Links to the documentation go through the single constant `DOCS_DIR` in `build.py`
  (`@@DOCS@@` in the strings): `docs/` on the same site, `docs/<lang>/` for the other languages.
  The docs header repeats the site's navigation (`overrides/partials/header.html`, labels from
  `site-src/strings/<lang>.json`). Read the Docs keeps a mirror.

## Roadmap page

The page `/roadmap/` is generated from one file, `site-src/data/roadmap.json` (sections in display
order, which is the planned delivery order). Each item has a `status` (`planned`, `progress`, `idea` or
`delivered`) and a `text` with one entry per language (`fr`, `en`, `de`, `es`, `it`, `pt`, `pt-BR`,
`ru`, `zh-Hans`; a missing language falls back to English). Delivered items also carry a `version`.
Only the labels around the list (title, status names) live in `strings/<lang>.json` (`rm_*` keys).

**At every release: move the delivered items to the "Déjà livré" section (`delivered`), add the
version there with one line in every language, and delete them from the planned sections.** Put
`"status": "progress"` on what is being worked on. Then run `python3 site-src/build.py`.
The two "Ideas" sections use `groups` (a title and items) instead of `items`: each group renders as a
collapsed `<details>`, and an item is either a `text` (translated) or a plain `name` (a product name).
Ideas are not commitments; promote one to a batch when work starts. No effort estimates.
