# Product website

The static page published at **https://dumbmonit.app/**. Plain
HTML and CSS, a few lines of JavaScript for the copy buttons, no build step,
no analytics, no third-party requests. Fonts (Bricolage Grotesque, SIL OFL)
are self-hosted in `assets/fonts/`.

## One-time setup

In the GitHub repository: **Settings → Pages → Build and deployment → Source:
GitHub Actions**. That is the only switch.

After that, `.github/workflows/site.yml` deploys this folder on every push to
`main` that touches `website/**`, and can be run by hand from the **Actions** tab
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
| `index.html`, `features/`, `devices/`, `devices/<family>/`, `install/`, `community/` (and the same under `<lang>/`) | The pages in each language, **generated** by `site-src/build.py` (do not edit by hand). English is the root, the others live in `/fr/`, `/de/`, `/es/`, `/it/`, `/pt/`, `/pt-BR/`, `/ru/`, `/zh-Hans/`. Colours and type follow `web/src/app.css` (day and night via `prefers-color-scheme`). |
| `assets/site.css` | Styles and design tokens. |
| `assets/shots/` | Screenshots of the live demo (`DUMBMONIT_DEMO=1`): desktop 1440 x 900 at 2x (plus a 1x copy), phone 390 x 844 at 2x, light and dark, AVIF with a WebP fallback. |
| `assets/og.png` | Social preview image, 1200 × 630. |
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
English), then run `python3 site-src/build.py` and commit the generated pages. CI runs
`build.py --check` and fails when they are stale.

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
