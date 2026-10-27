# Product website

Sources in `site-src/` (templates, strings, `static/` files); `website/` is the build output.

The static page published at **https://dumbmonit.app/**. Plain
HTML and CSS, a few lines of JavaScript for the copy buttons, no build step,
no analytics, no third-party requests. Fonts (Geist and Geist Mono, SIL OFL)
are self-hosted in `assets/fonts/`. Colours and type are those of the app's Enterprise theme
(`web/src/app.css`, `html.mode-enterprise`), always dark; the layout is a plain product site (top
navigation, full-width sections, footer in columns). The hero plays a short scripted demo in a
browser frame (`site.js`, still under `prefers-reduced-motion`), and the diagrams are drawn in HTML
and CSS (`DIAGRAMS` in `build.py`) so their text wraps and translates.

## One-time setup

In the GitHub repository: **Settings → Pages → Build and deployment → Source:
GitHub Actions**. That is the only switch.

After that, `.github/workflows/site.yml` builds the site and deploys it on every push to
`main` that touches `site-src/**`, and can be run by hand from the **Actions** tab
(*Site → Run workflow*). The first run creates the `github-pages` environment;
the URL appears in the run summary.

## Preview locally

Pages serves the site under `/dumbmonit/`, and `404.html` uses absolute
`/dumbmonit/…` paths, so preview it under the same prefix:

```sh
mkdir -p /tmp/preview && ln -sfn "$PWD/site" /tmp/preview/dumbmonit
python3 -m http.server 4321 --directory /tmp/preview
# open http://localhost:4321/dumbmonit/
```

## Contents

| Path | What |
|---|---|
| `index.html`, `features/`, `devices/`, `devices/<family>/`, `install/`, `roadmap/`, `community/` (and the same under `<lang>/`) | The pages in each language, **generated** by `site-src/build.py` (do not edit by hand). English is the root, the others live in `/fr/`, `/de/`, `/es/`, `/it/`, `/pt/`, `/pt-BR/`, `/ru/`, `/zh-Hans/`. Colours and type follow the Enterprise tokens of `web/src/app.css` (dark only). |
| `static/assets/site.css` | Styles and design tokens. |
| `static/assets/shots/` | Screenshots of the live demo (not shown at the moment; recapture them in Enterprise mode before using `@@SHOT@@` again) (`DUMBMONIT_DEMO=1`): desktop 1440 x 900 at 2x (plus a 1x copy), phone 390 x 844 at 2x, light and dark, AVIF with a WebP fallback. |
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

The device pages are generated from the project itself: `python3 site-src/extract.py` reads the
collector catalogue (`crates/server/tests/fixtures/collectors_catalog.json`), `docs/devices/` and
`docs/alerting/rules.md` and writes `site-src/data/devices.json`. Run it after adding a collector
(it fails if a type is missing from a family in `extract.py`), then `build.py`. Device and rule
texts are English; only the labels around them are translated. The locales are those of `web/messages/`.

- The browser language picks the page on the first visit to the root (English if none of the
  nine matches); the selector in the header remembers an explicit choice.
- Every page carries `lang`, `hreflang` alternates (and the sitemap too).
- Links to the documentation go through the single constant `DOCS_HOST` in `build.py`
  (`@@DOCS@@` in the strings); non-English pages point to the matching docs language.

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
