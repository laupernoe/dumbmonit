#!/usr/bin/env python3
"""Builds the product website's pages from one template and one string file per language.

    python3 site-src/build.py            # writes website/index.html, website/<lang>/index.html, sitemap.xml
    python3 site-src/build.py --check    # builds in memory and fails if website/ is out of date

Source of truth:
  site-src/index.template.html   the layout; every text is a {{kNNN}} placeholder
  site-src/strings/<lang>.json   the texts (en is the base; a missing key falls back to English)
  site-src/svgs.json             inline icons, referenced as [[svgN]] inside texts

English lives at the site root (the historical URL), the other languages in /<code>/.
The locale list is the one of web/messages/ and of mkdocs.yml (docs).
"""
import html
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "website"

SITE = "https://dumbmonit.app/"
# Where the documentation lives. One constant: change it here if the layout moves.
# English is the default language of the docs (no prefix); the others are served under
# <DOCS_BASE><code>/ (mkdocs-static-i18n, suffix layout).
DOCS_BASE = "https://dumbmonit.readthedocs.io/en/latest/"
LASTMOD = "2026-10-13"

# code, name in its own language, short label, Open Graph locale, docs folder ("" = default)
LANGS = [
    ("en", "English", "EN", "en_US", ""),
    ("fr", "Français", "FR", "fr_FR", "fr/"),
    ("de", "Deutsch", "DE", "de_DE", "de/"),
    ("es", "Español", "ES", "es_ES", "es/"),
    ("it", "Italiano", "IT", "it_IT", "it/"),
    ("pt", "Português", "PT", "pt_PT", "pt/"),
    ("pt-BR", "Português (Brasil)", "PT-BR", "pt_BR", "pt-BR/"),
    ("ru", "Русский", "RU", "ru_RU", "ru/"),
    ("zh-Hans", "简体中文", "中文", "zh_CN", "zh-Hans/"),
]

GLOBE = (
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" '
    'stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"/>'
    '<path d="M3 12h18M12 3c2.6 2.7 3.9 5.7 3.9 9s-1.3 6.3-3.9 9c-2.6-2.7-3.9-5.7-3.9-9S9.4 5.7 12 3Z"/></svg>'
)


def detect_script():
    codes = {c.lower(): c for c, *_ in LANGS}
    return (
        "<script>\n(function () {\n"
        "\ttry {\n"
        "\t\tif (localStorage.getItem('dm-lang')) return;\n"
        "\t\tvar known = " + json.dumps(codes) + ";\n"
        "\t\tvar prefs = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language];\n"
        "\t\tfor (var i = 0; i < prefs.length; i++) {\n"
        "\t\t\tvar l = String(prefs[i] || '').toLowerCase();\n"
        "\t\t\tvar code = known[l] || (l.indexOf('zh') === 0 ? 'zh-Hans' : known[l.split('-')[0]]);\n"
        "\t\t\tif (!code) continue;\n"
        "\t\t\tif (code !== 'en') location.replace(code + '/' + location.hash);\n"
        "\t\t\treturn;\n"
        "\t\t}\n"
        "\t} catch (e) {}\n"
        "})();\n</script>"
    )


def load_strings():
    base = json.loads((HERE / "strings" / "en.json").read_text())
    out = {}
    for code, *_ in LANGS:
        f = HERE / "strings" / f"{code}.json"
        tr = json.loads(f.read_text()) if f.exists() else {}
        unknown = set(tr) - set(base)
        if unknown:
            raise SystemExit(f"{code}: unknown keys {sorted(unknown)}")
        out[code] = {**base, **tr}
    return base, out


def shape(text):
    """What a translation must keep from the source: tags, icon tokens and doc links."""
    return sorted(re.findall(r"</?[a-z]+|\[\[svg\d+\]\]|@@DOCS@@[^\"]*", text))


def check_shapes(base, strings):
    errors = []
    for code, table in strings.items():
        for k, v in table.items():
            if shape(v) != shape(base[k]):
                errors.append(f"{code} {k}: markup differs from English")
    if errors:
        raise SystemExit("\n".join(errors))


def page_url(code):
    return SITE if code == "en" else f"{SITE}{code}/"


def build_page(template, svgs, table, code):
    root = "" if code == "en" else "../"
    _, name, short, og, docs_dir = next(l for l in LANGS if l[0] == code)

    # Texts inside attributes: quotes escaped.
    t = re.sub(r'(="\s*)\{\{(k\d+)\}\}"', lambda m: '="' + table[m.group(2)].replace('"', "&quot;") + '"', template)
    t = re.sub(r"\{\{(k\d+)\}\}", lambda m: table[m.group(1)], t)
    t = re.sub(r"\[\[(svg\d+)\]\]", lambda m: svgs[m.group(1)], t)
    t = t.replace("@@DOCS@@", DOCS_BASE + docs_dir)

    # Assets are shared: relative paths from a language folder go one level up.
    t = re.sub(r'\b(href|src|srcset)="(assets/|favicon\.svg|apple-touch-icon\.png)', lambda m: f'{m.group(1)}="{root}{m.group(2)}', t)

    alternates = "\n".join(
        f'<link rel="alternate" hreflang="{c}" href="{page_url(c)}">' for c, *_ in LANGS
    ) + f'\n<link rel="alternate" hreflang="x-default" href="{SITE}">'
    items = []
    for c, n, _s, _o, _d in LANGS:
        href = (root + ("" if c == "en" else f"{c}/")) or "./"
        if code != "en" and c == "en":
            href = "../"
        cur = ' aria-current="true"' if c == code else ""
        items.append(f'\t\t\t\t<li><a href="{href}" hreflang="{c}" lang="{c}"{cur}>{n}</a></li>')
    switcher = (
        '\t\t\t<details class="lang">\n'
        f'\t\t\t\t<summary aria-label="{html.escape(table["k187"], quote=True)}">{GLOBE}<span>{short}</span></summary>\n'
        '\t\t\t\t<ul>\n' + "\n".join(items) + "\n\t\t\t\t</ul>\n\t\t\t</details>"
    )
    t = (
        t.replace("@@LANG@@", code)
        .replace("@@URL@@", page_url(code))
        .replace("@@OGLOCALE@@", og)
        .replace("@@ALTERNATES@@", alternates)
        .replace("@@SWITCHER@@", switcher)
        .replace("@@DETECT@@", detect_script() if code == "en" else "")
    )
    assert not re.search(r"@@[A-Z]+@@", t), "unfilled placeholder"
    return t


def build_sitemap():
    rows = []
    for c, *_ in LANGS:
        links = "".join(
            f'\n    <xhtml:link rel="alternate" hreflang="{o}" href="{page_url(o)}"/>' for o, *_ in LANGS
        ) + f'\n    <xhtml:link rel="alternate" hreflang="x-default" href="{SITE}"/>'
        rows.append(f"  <url>\n    <loc>{page_url(c)}</loc>\n    <lastmod>{LASTMOD}</lastmod>{links}\n  </url>")
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n'
        + "\n".join(rows) + "\n</urlset>\n"
    )


def main():
    template = (HERE / "index.template.html").read_text()
    svgs = json.loads((HERE / "svgs.json").read_text())
    base, strings = load_strings()
    check_shapes(base, strings)
    files = {}
    for code, *_ in LANGS:
        path = OUT / "index.html" if code == "en" else OUT / code / "index.html"
        files[path] = build_page(template, svgs, strings[code], code)
    files[OUT / "sitemap.xml"] = build_sitemap()
    if "--check" in sys.argv:
        stale = [str(p) for p, c in files.items() if not p.exists() or p.read_text() != c]
        if stale:
            raise SystemExit("out of date, run site-src/build.py: " + ", ".join(stale))
        return
    for p, c in files.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(c)
    missing = {c: sorted(set(base) - set(json.loads((HERE / 'strings' / f'{c}.json').read_text())))
               for c, *_ in LANGS if c != "en" and (HERE / "strings" / f"{c}.json").exists()}
    for c, k in missing.items():
        if k:
            print(f"{c}: {len(k)} keys fall back to English")
    print("built", len(LANGS), "pages")


if __name__ == "__main__":
    main()
